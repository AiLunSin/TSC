//! Repository 的 SQLite 实现。写操作统一使用事务，保证主表、FTS5 与向量表一致。

use std::sync::{Mutex, MutexGuard};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::card::SuggestionCard;
use meeting_copilot_domain::model::enums::SessionState;
use meeting_copilot_domain::model::insight::{Insight, RollingSummary};
use meeting_copilot_domain::model::session::Session;
use meeting_copilot_domain::model::utterance::{Correction, Utterance};
use meeting_copilot_domain::port::repository::{
    CardRepository, InsightRepository, SessionRepository, UtteranceRepository,
};
use meeting_copilot_domain::vo::{CardRef, InsightRef, SessionId, UtteranceRef};

use super::{StoreConfig, StoreError, StoreKey, SCHEMA_VERSION};

mod cache;
mod codec;
mod feedback;
mod on_demand;
mod outbox;

const SCHEMA_V2_CACHE_OUTBOX: &str = include_str!("../migrations/v2_cache_outbox.sql");
const SCHEMA_V2_FEEDBACK: &str = include_str!("../migrations/v2_feedback.sql");
const SCHEMA_V2_AUDIT: &str = include_str!("../migrations/v2_audit.sql");
const SCHEMA_V3_AUDIT_HARDENING: &str = include_str!("../migrations/v3_audit_hardening.sql");

/// 字段级密文前缀；无前缀行视为旧版明文，仅用于兼容读取。
const FIELD_CIPHER_PREFIX: &str = "enc:v1:";

const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
  id           TEXT PRIMARY KEY,
  state        TEXT NOT NULL,
  payload      TEXT NOT NULL,
  created_at   INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS utterances (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id   TEXT NOT NULL,
  reference    TEXT NOT NULL,
  payload      TEXT NOT NULL,
  UNIQUE(session_id, reference)
);
CREATE VIRTUAL TABLE IF NOT EXISTS utterances_fts USING fts5(
  session_id UNINDEXED,
  reference UNINDEXED,
  text,
  tokenize = 'trigram'
);
CREATE TABLE IF NOT EXISTS utterance_vectors (
  session_id   TEXT NOT NULL,
  reference    TEXT NOT NULL,
  vector       BLOB NOT NULL,
  PRIMARY KEY(session_id, reference)
);
CREATE TABLE IF NOT EXISTS insights (
  session_id   TEXT NOT NULL,
  reference    TEXT NOT NULL,
  payload      TEXT NOT NULL,
  PRIMARY KEY(session_id, reference)
);
CREATE TABLE IF NOT EXISTS cards (
  session_id   TEXT NOT NULL,
  reference    TEXT NOT NULL,
  payload      TEXT NOT NULL,
  PRIMARY KEY(session_id, reference)
);
CREATE TABLE IF NOT EXISTS summaries (
  session_id   TEXT NOT NULL,
  version      INTEGER NOT NULL,
  payload      TEXT NOT NULL,
  PRIMARY KEY(session_id, version)
);
"#;

fn sql_error(error: rusqlite::Error) -> StoreError {
    StoreError::Sqlite(error.to_string())
}

fn decode<T: for<'de> Deserialize<'de>>(payload: &str) -> Result<T, StoreError> {
    serde_json::from_str(payload).map_err(|error| StoreError::Serialization(error.to_string()))
}

fn encode<T: Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value).map_err(|error| StoreError::Serialization(error.to_string()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn hex_decode(raw: &str) -> Result<Vec<u8>, StoreError> {
    if !raw.len().is_multiple_of(2) {
        return Err(StoreError::Serialization("字段密文编码非法".to_owned()));
    }
    let mut bytes = Vec::with_capacity(raw.len() / 2);
    for chunk in raw.as_bytes().as_chunks::<2>().0 {
        let text = std::str::from_utf8(chunk)
            .map_err(|_| StoreError::Serialization("字段密文编码非法".to_owned()))?;
        bytes.push(
            u8::from_str_radix(text, 16)
                .map_err(|_| StoreError::Serialization("字段密文编码非法".to_owned()))?,
        );
    }
    Ok(bytes)
}

pub struct SqliteStore {
    conn: Mutex<Connection>,
    field_key: StoreKey,
}

impl SqliteStore {
    /// 打开库但不提供密钥。任何构建下都会失败，禁止静默生成明文业务库；
    /// 调用方必须使用 [`SqliteStore::open_with_key`]。
    pub fn open(config: &StoreConfig) -> Result<Self, StoreError> {
        Self::open_with_key(config, None)
    }

    /// 打开库并注入 L1 派生的 32B 数据库密钥（`crypto::derive_l1` 的输出）。
    /// 默认后端用该密钥加密业务载荷；SQLCipher 后端额外启用页级加密。
    pub fn open_with_key(config: &StoreConfig, key: Option<&StoreKey>) -> Result<Self, StoreError> {
        config.validate()?;
        let conn = Connection::open(&config.path).map_err(sql_error)?;
        Self::prepare(conn, config.wal_enabled, key)
    }

    /// 内存库不落盘。SQLCipher 模式下仍需密钥才能启用加密引擎，
    /// 因此此处生成一次性随机密钥；库销毁后密钥随之丢失。
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory().map_err(sql_error)?;
        let key = StoreKey::new(crate::crypto::generate_master());
        Self::prepare(conn, false, Some(&key))
    }

    fn prepare(
        conn: Connection,
        wal_enabled: bool,
        key: Option<&StoreKey>,
    ) -> Result<Self, StoreError> {
        let field_key = key.cloned().ok_or(StoreError::MissingKey)?;
        #[cfg(feature = "sqlcipher")]
        apply_key(&conn, Some(&field_key))?;
        if wal_enabled {
            let _ = conn.pragma_update(None, "journal_mode", "WAL");
        }
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(sql_error)?;
        migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            field_key,
        })
    }

    fn encode_payload<T: Serialize>(
        &self,
        session_id: &str,
        value: &T,
    ) -> Result<String, StoreError> {
        let plaintext = encode(value)?;
        let envelope = crate::crypto::encrypt_field(
            self.field_key.as_bytes(),
            session_id,
            plaintext.as_bytes(),
        )
        .map_err(|error| StoreError::Serialization(error.to_string()))?;
        Ok(format!("{FIELD_CIPHER_PREFIX}{}", hex_encode(&envelope)))
    }

    fn decode_payload<T: for<'de> Deserialize<'de>>(
        &self,
        session_id: &str,
        payload: &str,
    ) -> Result<T, StoreError> {
        let Some(encoded) = payload.strip_prefix(FIELD_CIPHER_PREFIX) else {
            // 兼容升级前已存在的明文行；所有新写入均为字段级密文。
            return decode(payload);
        };
        let envelope = hex_decode(encoded)?;
        let plaintext =
            crate::crypto::decrypt_field(self.field_key.as_bytes(), session_id, &envelope)
                .map_err(|error| StoreError::Serialization(error.to_string()))?;
        serde_json::from_slice(&plaintext)
            .map_err(|error| StoreError::Serialization(error.to_string()))
    }

    pub fn schema_version(&self) -> Result<u32, StoreError> {
        let version: i64 = self
            .lock()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(sql_error)?;
        u32::try_from(version).map_err(|_| StoreError::MigrationFailed("user_version 非法".into()))
    }

    #[cfg(test)]
    fn append_with_failure(
        &self,
        session: &SessionId,
        utterance: &Utterance,
        fail_before_vector: bool,
    ) -> Result<(), CopilotError> {
        self.append_tx(session, utterance, fail_before_vector)
            .map_err(CopilotError::from)
    }

    fn append_tx(
        &self,
        session: &SessionId,
        utterance: &Utterance,
        fail_before_vector: bool,
    ) -> Result<(), StoreError> {
        let payload = self.encode_payload(session.as_str(), utterance)?;
        let mut conn = self.lock();
        let tx = conn.transaction().map_err(sql_error)?;
        tx.execute(
            "INSERT INTO utterances (session_id, reference, payload) VALUES (?1, ?2, ?3)",
            params![session.as_str(), utterance.reference.as_str(), payload],
        )
        .map_err(sql_error)?;
        tx.execute(
            "INSERT INTO utterances_fts (session_id, reference, text) VALUES (?1, ?2, ?3)",
            params![
                session.as_str(),
                utterance.reference.as_str(),
                utterance.text.as_str()
            ],
        )
        .map_err(sql_error)?;
        if fail_before_vector {
            return Err(StoreError::InjectedFailure);
        }
        tx.execute(
            "INSERT INTO utterance_vectors (session_id, reference, vector) VALUES (?1, ?2, ?3)",
            params![
                session.as_str(),
                utterance.reference.as_str(),
                vec![0_u8; 8]
            ],
        )
        .map_err(sql_error)?;
        tx.commit().map_err(sql_error)?;
        Ok(())
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 供同 crate 的审计/统计模块复用同一连接（不对外暴露 SQL 能力）。
    pub(crate) fn connection(&self) -> MutexGuard<'_, Connection> {
        self.lock()
    }

    pub fn fts_search(&self, session: &SessionId, term: &str) -> Result<Vec<String>, StoreError> {
        let conn = self.lock();
        let mut statement = conn
            .prepare(
                "SELECT reference FROM utterances_fts WHERE session_id = ?1 AND utterances_fts MATCH ?2",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map(params![session.as_str(), term], |row| row.get(0))
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<String>, _>>().map_err(sql_error)
    }

    pub fn vector_count(&self, session: &SessionId) -> Result<usize, StoreError> {
        let count: i64 = self
            .lock()
            .query_row(
                "SELECT COUNT(*) FROM utterance_vectors WHERE session_id = ?1",
                params![session.as_str()],
                |row| row.get(0),
            )
            .map_err(sql_error)?;
        usize::try_from(count).map_err(|_| StoreError::Sqlite("count 溢出".into()))
    }
}

/// 在读取任何数据页之前下发 SQLCipher 原始密钥，并用一次元数据读取校验密钥。
#[cfg(feature = "sqlcipher")]
fn apply_key(conn: &Connection, key: Option<&StoreKey>) -> Result<(), StoreError> {
    let key = key.ok_or(StoreError::MissingKey)?;
    // SQLCipher 原始密钥字面量：`x'<64 位十六进制>'`，跳过口令 KDF。
    conn.execute_batch(&format!("PRAGMA key = \"x'{}'\";", key.to_hex()))
        .map_err(sql_error)?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|_| StoreError::WrongKey)?;
    Ok(())
}

fn migrate(conn: &Connection) -> Result<(), StoreError> {
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(sql_error)?;
    if version > i64::from(SCHEMA_VERSION) {
        return Err(StoreError::MigrationFailed(format!(
            "库版本 {version} 高于程序支持的 {SCHEMA_VERSION}"
        )));
    }
    if version < 1 {
        conn.execute_batch(SCHEMA_V1).map_err(sql_error)?;
        conn.pragma_update(None, "user_version", 1)
            .map_err(sql_error)?;
    }
    if version < 2 {
        for script in [SCHEMA_V2_CACHE_OUTBOX, SCHEMA_V2_FEEDBACK, SCHEMA_V2_AUDIT] {
            conn.execute_batch(script).map_err(sql_error)?;
        }
        conn.pragma_update(None, "user_version", 2)
            .map_err(sql_error)?;
    }
    if version < 3 {
        conn.execute_batch(SCHEMA_V3_AUDIT_HARDENING)
            .map_err(sql_error)?;
        conn.pragma_update(None, "user_version", 3)
            .map_err(sql_error)?;
    }
    debug_assert_eq!(SCHEMA_VERSION, 3);
    Ok(())
}

impl SessionRepository for SqliteStore {
    fn create(&self, session: &Session) -> Result<(), CopilotError> {
        let payload = self.encode_payload(session.id.as_str(), session)?;
        self.lock()
            .execute(
                "INSERT INTO sessions (id, state, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    session.id.as_str(),
                    format!("{:?}", session.state),
                    payload,
                    session.started_at.as_unix_millis()
                ],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(())
    }

    fn transition(&self, id: &SessionId, next: SessionState) -> Result<Session, CopilotError> {
        let current =
            SessionRepository::get(self, id)?.ok_or_else(|| CopilotError::StoreFailure {
                detail: format!("会话 {id} 不存在"),
            })?;
        let updated = current.transition(next)?;
        let payload = self.encode_payload(id.as_str(), &updated)?;
        self.lock()
            .execute(
                "UPDATE sessions SET state = ?1, payload = ?2 WHERE id = ?3",
                params![format!("{:?}", updated.state), payload, id.as_str()],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(updated)
    }

    fn get(&self, id: &SessionId) -> Result<Option<Session>, CopilotError> {
        let payload: Option<String> = self
            .lock()
            .query_row(
                "SELECT payload FROM sessions WHERE id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        payload
            .map(|value| {
                self.decode_payload::<Session>(id.as_str(), &value)
                    .map_err(CopilotError::from)
            })
            .transpose()
    }

    fn delete_irrecoverable(&self, id: &SessionId) -> Result<(), CopilotError> {
        let mut conn = self.lock();
        let tx = conn
            .transaction()
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        for statement in [
            "DELETE FROM utterances WHERE session_id = ?1",
            "DELETE FROM utterances_fts WHERE session_id = ?1",
            "DELETE FROM utterance_vectors WHERE session_id = ?1",
            "DELETE FROM insights WHERE session_id = ?1",
            "DELETE FROM cards WHERE session_id = ?1",
            "DELETE FROM summaries WHERE session_id = ?1",
            "DELETE FROM query_cache WHERE session_id = ?1",
            "DELETE FROM proactive_outbox WHERE session_id = ?1",
        ] {
            tx.execute(statement, params![id.as_str()])
                .map_err(|e| CopilotError::from(sql_error(e)))?;
        }
        tx.execute("DELETE FROM sessions WHERE id = ?1", params![id.as_str()])
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        tx.commit().map_err(|e| CopilotError::from(sql_error(e)))?;
        Ok(())
    }
}

impl UtteranceRepository for SqliteStore {
    fn append(&self, session: &SessionId, utterance: &Utterance) -> Result<(), CopilotError> {
        self.append_tx(session, utterance, false)
            .map_err(CopilotError::from)
    }

    fn apply_correction(
        &self,
        session: &SessionId,
        correction: &Correction,
    ) -> Result<(), CopilotError> {
        let mut conn = self.lock();
        let tx = conn
            .transaction()
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        let payload: Option<String> = tx
            .query_row(
                "SELECT payload FROM utterances WHERE session_id = ?1 AND reference = ?2",
                params![session.as_str(), correction.replaces.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        let raw = payload.ok_or_else(|| CopilotError::StoreFailure {
            detail: "被修订片段不存在".into(),
        })?;
        let mut utterance: Utterance = self.decode_payload(session.as_str(), &raw)?;
        utterance.text = correction.revised_text.clone();
        utterance.confidence = correction.confidence;
        let updated = self.encode_payload(session.as_str(), &utterance)?;
        tx.execute(
            "UPDATE utterances SET payload = ?1 WHERE session_id = ?2 AND reference = ?3",
            params![updated, session.as_str(), correction.replaces.as_str()],
        )
        .map_err(|e| CopilotError::from(sql_error(e)))?;
        tx.execute(
            "UPDATE utterances_fts SET text = ?1 WHERE session_id = ?2 AND reference = ?3",
            params![
                correction.revised_text.as_str(),
                session.as_str(),
                correction.replaces.as_str()
            ],
        )
        .map_err(|e| CopilotError::from(sql_error(e)))?;
        tx.commit().map_err(|e| CopilotError::from(sql_error(e)))?;

        Ok(())
    }

    fn list(&self, session: &SessionId) -> Result<Vec<Utterance>, CopilotError> {
        let conn = self.lock();
        let mut statement = conn
            .prepare("SELECT payload FROM utterances WHERE session_id = ?1 ORDER BY id")
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        let rows = statement
            .query_map(params![session.as_str()], |row| row.get::<_, String>(0))
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        let mut items = Vec::new();
        for row in rows {
            let payload = row.map_err(|e| CopilotError::from(sql_error(e)))?;
            items.push(self.decode_payload(session.as_str(), &payload)?);
        }
        Ok(items)
    }

    fn delete(&self, session: &SessionId, reference: &UtteranceRef) -> Result<(), CopilotError> {
        let mut conn = self.lock();
        let payloads: Vec<String> = {
            let mut statement = conn
                .prepare("SELECT payload FROM insights WHERE session_id = ?1")
                .map_err(|e| CopilotError::from(sql_error(e)))?;
            let rows = statement
                .query_map(params![session.as_str()], |row| row.get::<_, String>(0))
                .map_err(|e| CopilotError::from(sql_error(e)))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| CopilotError::from(sql_error(e)))?
        };
        for payload in payloads {
            let insight: Insight = self.decode_payload(session.as_str(), &payload)?;
            if insight.sources.iter().any(|item| item == reference) {
                return Err(CopilotError::StoreFailure {
                    detail: "片段被要点引用，禁止删除".into(),
                });
            }
        }

        let tx = conn
            .transaction()
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        for statement in [
            "DELETE FROM utterances WHERE session_id = ?1 AND reference = ?2",
            "DELETE FROM utterances_fts WHERE session_id = ?1 AND reference = ?2",
            "DELETE FROM utterance_vectors WHERE session_id = ?1 AND reference = ?2",
        ] {
            tx.execute(statement, params![session.as_str(), reference.as_str()])
                .map_err(|e| CopilotError::from(sql_error(e)))?;
        }
        tx.commit().map_err(|e| CopilotError::from(sql_error(e)))?;
        Ok(())
    }
}

impl InsightRepository for SqliteStore {
    fn save(&self, session: &SessionId, insight: &Insight) -> Result<(), CopilotError> {
        let payload = self.encode_payload(session.as_str(), insight)?;
        self.lock()
            .execute(
                "INSERT INTO insights (session_id, reference, payload) VALUES (?1, ?2, ?3)
                 ON CONFLICT(session_id, reference) DO UPDATE SET payload = excluded.payload",
                params![session.as_str(), insight.reference.as_str(), payload],
            )
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        Ok(())
    }

    fn get(
        &self,
        session: &SessionId,
        reference: &InsightRef,
    ) -> Result<Option<Insight>, CopilotError> {
        let payload: Option<String> = self
            .lock()
            .query_row(
                "SELECT payload FROM insights WHERE session_id = ?1 AND reference = ?2",
                params![session.as_str(), reference.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        payload
            .map(|value| self.decode_payload(session.as_str(), &value))
            .transpose()
            .map_err(CopilotError::from)
    }

    fn list(&self, session: &SessionId) -> Result<Vec<Insight>, CopilotError> {
        let conn = self.lock();
        let mut statement = conn
            .prepare("SELECT payload FROM insights WHERE session_id = ?1")
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        let rows = statement
            .query_map(params![session.as_str()], |row| row.get::<_, String>(0))
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        let mut items = Vec::new();
        for row in rows {
            let payload = row.map_err(|e| CopilotError::from(sql_error(e)))?;
            items.push(self.decode_payload(session.as_str(), &payload)?);
        }
        Ok(items)
    }

    fn save_summary(
        &self,
        session: &SessionId,
        summary: &RollingSummary,
    ) -> Result<(), CopilotError> {
        let payload = self.encode_payload(session.as_str(), summary)?;
        self.lock()
            .execute(
                "INSERT INTO summaries (session_id, version, payload) VALUES (?1, ?2, ?3)
                 ON CONFLICT(session_id, version) DO UPDATE SET payload = excluded.payload",
                params![session.as_str(), summary.version, payload],
            )
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        Ok(())
    }
}

impl CardRepository for SqliteStore {
    fn save(&self, session: &SessionId, card: &SuggestionCard) -> Result<(), CopilotError> {
        card.validate()
            .map_err(|error| CopilotError::PrivacyViolation {
                detail: error.to_string(),
            })?;
        let payload = self.encode_payload(session.as_str(), card)?;
        self.lock()
            .execute(
                "INSERT INTO cards (session_id, reference, payload) VALUES (?1, ?2, ?3)",
                params![session.as_str(), card.identity.as_str(), payload],
            )
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        Ok(())
    }

    fn get(
        &self,
        session: &SessionId,
        reference: &CardRef,
    ) -> Result<Option<SuggestionCard>, CopilotError> {
        let payload: Option<String> = self
            .lock()
            .query_row(
                "SELECT payload FROM cards WHERE session_id = ?1 AND reference = ?2",
                params![session.as_str(), reference.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        payload
            .map(|value| self.decode_payload(session.as_str(), &value))
            .transpose()
            .map_err(CopilotError::from)
    }

    fn update(&self, session: &SessionId, card: &SuggestionCard) -> Result<(), CopilotError> {
        let payload = self.encode_payload(session.as_str(), card)?;
        self.lock()
            .execute(
                "UPDATE cards SET payload = ?1 WHERE session_id = ?2 AND reference = ?3",
                params![payload, session.as_str(), card.identity.as_str()],
            )
            .map_err(|e| CopilotError::from(sql_error(e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use meeting_copilot_domain::model::enums::{
        ChannelPreference, ConsentMode, Platform, ReadingMode, SourceKind,
    };
    use meeting_copilot_domain::model::session::{
        ConsentRecord, ParticipantList, PlatformBinding, RetentionWindow, TextRange,
    };
    use meeting_copilot_domain::model::utterance::SpeakerRef;
    use meeting_copilot_domain::vo::{Confidence, UtteranceRef, UtteranceText, WallClock};

    use super::*;

    fn session() -> Session {
        Session {
            id: SessionId::new("sess-1").expect("id"),
            state: SessionState::NotStarted,
            binding: PlatformBinding {
                platform: Platform::Zoom,
                version: "6.1".into(),
                preference: ChannelPreference::SubtitleFirst,
                subtitle_enabled: true,
            },
            channel: ChannelPreference::SubtitleFirst,
            started_at: WallClock::from_unix_millis(1_000),
            ended_at: None,
            retention: RetentionWindow::new(30).expect("retention"),
            consent: ConsentRecord {
                mode: ConsentMode::Explicit,
                granted_at: WallClock::from_unix_millis(1_000),
                notice_version: 1,
            },
            participants: None::<ParticipantList>,
            screen_share_protection: true,
        }
    }

    fn utterance(reference: &str, text: &str) -> Utterance {
        Utterance {
            reference: UtteranceRef::new(reference).expect("reference"),
            span: TextRange::new(0, 1_000).expect("span"),
            wall_clock: WallClock::from_unix_millis(1_000),
            text: UtteranceText::new(text).expect("text"),
            speaker: SpeakerRef::unknown(),
            confidence: Confidence::new(0.9).expect("confidence"),
            source: SourceKind::PlatformSubtitle,
            platform: Some(Platform::Zoom),
            dual_channel: false,
            reading_mode: ReadingMode::Subtitle,
        }
    }

    fn store() -> SqliteStore {
        let store = SqliteStore::open_in_memory().expect("open");
        SessionRepository::create(&store, &session()).expect("create");
        store
    }

    fn session_id() -> SessionId {
        SessionId::new("sess-1").expect("id")
    }

    #[test]
    fn schema_is_migrated_to_v2() {
        let store = SqliteStore::open_in_memory().expect("open");
        assert_eq!(store.schema_version().expect("version"), SCHEMA_VERSION);
        let conn = store.connection();
        for table in [
            "query_cache",
            "proactive_outbox",
            "on_demand_sample",
            "feedback_event",
            "topic_weight",
            "audit_record",
            "audit_anchor",
            "audit_archive",
        ] {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get(0),
                )
                .expect("count");
            assert_eq!(exists, 1, "缺少表 {table}");
        }
    }

    #[test]
    fn session_create_get_and_transition() {
        let store = store();
        let loaded = SessionRepository::get(&store, &session_id())
            .expect("get")
            .expect("存在");
        assert_eq!(loaded.state, SessionState::NotStarted);

        let listening =
            SessionRepository::transition(&store, &session_id(), SessionState::Listening)
                .expect("transition");
        assert_eq!(listening.state, SessionState::Listening);
        assert!(listening.is_eligible_for_capture());
    }

    #[test]
    fn three_writes_are_consistent_in_one_transaction() {
        let store = store();
        let item = utterance("utt-1", "我们把灰度范围收窄到华东两个站点");
        UtteranceRepository::append(&store, &session_id(), &item).expect("append");

        let listed = UtteranceRepository::list(&store, &session_id()).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(store.vector_count(&session_id()).expect("vector"), 1);
        let hits = store.fts_search(&session_id(), "灰度范围").expect("fts");
        assert_eq!(hits, vec!["utt-1".to_owned()]);
    }

    #[test]
    fn business_payload_is_encrypted_at_rest_and_readable() {
        let store = store();
        let item = utterance("utt-secret", "客户的私有转录内容");
        UtteranceRepository::append(&store, &session_id(), &item).expect("append");

        let payload: String = store
            .lock()
            .query_row(
                "SELECT payload FROM utterances WHERE reference = 'utt-secret'",
                [],
                |row| row.get(0),
            )
            .expect("payload");
        assert!(payload.starts_with(FIELD_CIPHER_PREFIX));
        assert!(!payload.contains("客户的私有转录内容"));

        let listed = UtteranceRepository::list(&store, &session_id()).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].text.as_str(), "客户的私有转录内容");
    }

    #[test]
    fn failed_write_rolls_back_all_three_tables() {
        let store = store();
        let item = utterance("utt-rollback", "这条不该落库");
        let result = store.append_with_failure(&session_id(), &item, true);
        assert!(result.is_err());

        assert!(UtteranceRepository::list(&store, &session_id())
            .expect("list")
            .is_empty());
        assert_eq!(store.vector_count(&session_id()).expect("vector"), 0);
        assert!(store
            .fts_search(&session_id(), "不该落库")
            .expect("fts")
            .is_empty());
    }

    #[test]
    fn delete_irrecoverable_clears_everything() {
        let store = store();
        let item = utterance("utt-1", "待删除片段");
        UtteranceRepository::append(&store, &session_id(), &item).expect("append");

        SessionRepository::delete_irrecoverable(&store, &session_id()).expect("delete");
        assert!(SessionRepository::get(&store, &session_id())
            .expect("get")
            .is_none());
        assert!(UtteranceRepository::list(&store, &session_id())
            .expect("list")
            .is_empty());
        assert_eq!(store.vector_count(&session_id()).expect("vector"), 0);
    }

    #[test]
    fn utterance_delete_is_blocked_when_referenced_by_insight() {
        let store = store();
        let item = utterance("utt-1", "被要点引用的片段");
        UtteranceRepository::append(&store, &session_id(), &item).expect("append");

        let insight = Insight::new(
            InsightRef::new("ins-1").expect("insight id"),
            meeting_copilot_domain::model::enums::InsightKind::KeyInfo,
            meeting_copilot_domain::vo::ShortText::new("引用该片段").expect("text"),
            Confidence::new(0.8).expect("conf"),
            vec![UtteranceRef::new("utt-1").expect("ref")],
            WallClock::from_unix_millis(2_000),
        )
        .expect("insight");
        InsightRepository::save(&store, &session_id(), &insight).expect("save");

        let result = UtteranceRepository::delete(&store, &session_id(), &item.reference);
        assert!(result.is_err(), "被引用片段必须禁止删除");
    }

    #[test]
    fn opening_without_key_is_refused() {
        let config = StoreConfig {
            path: "unused.db".into(),
            key_ref: None,
            wal_enabled: true,
        };
        assert!(SqliteStore::open(&config).is_err());
    }

    // 样本库是明文 SQLite，仅在非 SQLCipher 构建下有意义。
    #[cfg(not(feature = "sqlcipher"))]
    #[test]
    fn v0_database_is_migrated_and_preserves_rows() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "meeting-copilot-v0-{}-{stamp}.db",
            std::process::id()
        ));

        {
            let conn = Connection::open(&path).expect("open v0");
            conn.execute_batch(
                "CREATE TABLE sessions (
                   id TEXT PRIMARY KEY,
                   state TEXT NOT NULL,
                   payload TEXT NOT NULL,
                   created_at INTEGER NOT NULL
                 );
                 PRAGMA user_version = 0;",
            )
            .expect("schema v0");
            conn.execute(
                "INSERT INTO sessions (id, state, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
                params!["legacy-1", "Ended", "{}", 1_i64],
            )
            .expect("legacy row");
        }

        let config = StoreConfig {
            path: path.to_string_lossy().into_owned(),
            key_ref: Some("keychain://test/db".into()),
            wal_enabled: false,
        };
        let key = StoreKey::new([3_u8; crate::crypto::MASTER_KEY_LEN]);
        let store = SqliteStore::open_with_key(&config, Some(&key)).expect("migrate");
        assert_eq!(store.schema_version().expect("version"), SCHEMA_VERSION);

        let count: i64 = store
            .lock()
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 1, "迁移不得丢失旧数据");

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[cfg(feature = "sqlcipher")]
    mod sqlcipher {
        use super::*;
        use crate::crypto::{derive_l1, MASTER_KEY_LEN};
        use crate::store::StoreKey;

        fn key(master: &[u8; MASTER_KEY_LEN]) -> StoreKey {
            StoreKey::new(
                derive_l1(master, "com.meetingcopilot.desktop", b"sqlcipher-test-salt")
                    .expect("派生"),
            )
        }

        fn temp_db(tag: &str) -> std::path::PathBuf {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            std::env::temp_dir().join(format!(
                "meeting-copilot-sqlcipher-{tag}-{}-{stamp}.db",
                std::process::id()
            ))
        }

        fn config_at(path: &std::path::Path) -> StoreConfig {
            StoreConfig {
                path: path.to_string_lossy().into_owned(),
                key_ref: Some("keychain://test/db".into()),
                wal_enabled: false,
            }
        }

        fn cleanup(path: &std::path::Path) {
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
            }
        }

        #[test]
        fn missing_key_is_rejected() {
            let path = temp_db("missing-key");
            assert_eq!(
                SqliteStore::open(&config_at(&path)).err(),
                Some(StoreError::MissingKey)
            );
            cleanup(&path);
        }

        #[test]
        fn wrong_key_is_rejected_and_correct_key_roundtrips() {
            let path = temp_db("roundtrip");
            let config = config_at(&path);
            let master = [7_u8; MASTER_KEY_LEN];

            {
                let store = SqliteStore::open_with_key(&config, Some(&key(&master))).expect("open");
                SessionRepository::create(&store, &session()).expect("create");
            }

            let bytes = std::fs::read(&path).expect("read");
            assert!(
                !bytes.starts_with(b"SQLite format 3\0"),
                "库文件必须是 SQLCipher 密文"
            );

            let wrong = key(&[9_u8; MASTER_KEY_LEN]);
            assert_eq!(
                SqliteStore::open_with_key(&config, Some(&wrong)).err(),
                Some(StoreError::WrongKey)
            );

            let store = SqliteStore::open_with_key(&config, Some(&key(&master))).expect("reopen");
            assert!(
                SessionRepository::get(&store, &session_id())
                    .expect("get")
                    .is_some(),
                "正确密钥必须能读回数据"
            );

            drop(store);
            cleanup(&path);
        }
    }
}
