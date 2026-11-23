//! 持久化审计哈希链（SPEC 7.4.1、DESIGN 2.10.4）。
//!
//! 追加与校验都在同一 SQLite 连接上完成；表级触发器保证线上记录不可改、
//! 未归档前不可删。写失败或断链后置"审计不可用"，敏感操作由调用方拒绝。

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, OptionalExtension, TransactionBehavior};

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::enums::AuditAction;
use meeting_copilot_domain::port::governance::AuditLog;
use meeting_copilot_domain::vo::AuditId;

use super::{canonical_fields, digest, GENESIS_HASH};
use crate::store::{SqliteStore, StoreError};

const ANCHOR_INTERVAL: i64 = 1_000;
const MAX_SUMMARY_CHARS: usize = 200;

/// 运行时零转录扫描：摘要只允许固定模板与 ID/长度/分类（SPEC 7.4.1-10）。
/// 拒绝控制字符与 URL；错误信息只带分类，不回显摘要内容。
fn summary_violation(summary: &str) -> Option<&'static str> {
    if summary.chars().any(char::is_control) {
        return Some("control_char");
    }
    let lower = summary.to_ascii_lowercase();
    let has_url = lower.contains(concat!("http", "://"))
        || lower.contains(concat!("http", "s://"))
        || lower.contains("www.");
    if has_url {
        return Some("url");
    }
    None
}

fn sql_error(error: rusqlite::Error) -> StoreError {
    StoreError::Sqlite(error.to_string())
}

fn id_from_raw(raw: String) -> Result<AuditId, StoreError> {
    AuditId::new(raw).map_err(|error| StoreError::Serialization(error.to_string()))
}

pub struct SqliteAuditLog<'a> {
    store: &'a SqliteStore,
}

impl<'a> SqliteAuditLog<'a> {
    pub fn new(store: &'a SqliteStore) -> Self {
        Self { store }
    }

    /// 追加一条审计记录。`summary` 必须是脱敏摘要，长度不超过 200 字符。
    pub fn append_record(
        &self,
        action: AuditAction,
        actor: &str,
        occurred_at_ms: i64,
        target: &str,
        summary: &str,
    ) -> Result<AuditId, StoreError> {
        if actor.trim().is_empty() {
            return Err(StoreError::Serialization("审计 actor 为空".to_owned()));
        }
        if target.trim().is_empty() {
            return Err(StoreError::Serialization("审计 target 为空".to_owned()));
        }
        if summary.chars().count() > MAX_SUMMARY_CHARS {
            return Err(StoreError::Serialization("审计摘要超长".to_owned()));
        }
        if let Some(reason) = summary_violation(summary) {
            return Err(StoreError::Serialization(format!(
                "审计摘要命中零转录扫描：{reason}"
            )));
        }

        let mut conn = self.store.connection();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error)?;
        let previous: Option<String> = tx
            .query_row(
                "SELECT self_hash FROM audit_record ORDER BY seq DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_error)?;
        let previous = previous.unwrap_or_else(|| GENESIS_HASH.to_owned());
        let next_seq: i64 = tx
            .query_row(
                "SELECT COALESCE(
                    (SELECT seq FROM sqlite_sequence WHERE name = 'audit_record'), 0
                 ) + 1",
                [],
                |row| row.get(0),
            )
            .map_err(sql_error)?;
        let audit_id = format!("audit-{next_seq:016}");
        let self_hash = digest(
            &previous,
            &canonical_fields(action, actor, occurred_at_ms, target, summary),
        );
        tx.execute(
            "INSERT INTO audit_record (
                seq, audit_id, action, actor, occurred_at_ms, target,
                sanitized_summary, prev_hash, self_hash
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                next_seq,
                audit_id,
                action.as_str(),
                actor,
                occurred_at_ms,
                target,
                summary,
                previous,
                self_hash,
            ],
        )
        .map_err(sql_error)?;

        if next_seq % ANCHOR_INTERVAL == 0 {
            let anchor_hash = digest(&self_hash, &format!("anchor|{next_seq}"));
            tx.execute(
                "INSERT INTO audit_anchor (up_to_seq, last_self_hash, anchor_hash, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4)",
                params![next_seq, self_hash, anchor_hash, occurred_at_ms],
            )
            .map_err(sql_error)?;
        }
        tx.commit().map_err(sql_error)?;
        id_from_raw(audit_id)
    }

    /// 按 `seq` 全链重算；返回首个不一致的审计 ID。归档前移出的分段用
    /// `audit_archive.terminal_self_hash` 衔接。
    pub fn verify(&self) -> Result<Option<AuditId>, StoreError> {
        let conn = self.store.connection();
        let first: Option<(String, String, i64)> = conn
            .query_row(
                "SELECT audit_id, prev_hash, seq FROM audit_record ORDER BY seq ASC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(sql_error)?;
        let Some((first_id, first_prev, first_seq)) = first else {
            return Ok(None);
        };

        if first_seq == 1 {
            if first_prev != GENESIS_HASH {
                return Ok(Some(id_from_raw(first_id)?));
            }
        } else {
            let archived: Option<String> = conn
                .query_row(
                    "SELECT terminal_self_hash FROM audit_archive WHERE last_seq = ?1",
                    params![first_seq.saturating_sub(1)],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_error)?;
            if archived.as_deref() != Some(first_prev.as_str()) {
                return Ok(Some(id_from_raw(first_id)?));
            }
        }

        let mut statement = conn
            .prepare(
                "SELECT audit_id, action, actor, occurred_at_ms, target,
                        sanitized_summary, prev_hash, self_hash, seq
                 FROM audit_record ORDER BY seq ASC",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            })
            .map_err(sql_error)?;

        let mut previous = first_prev;
        let mut expected_seq = first_seq;
        for row in rows {
            let (
                audit_id,
                action_raw,
                actor,
                occurred_at_ms,
                target,
                summary,
                prev_hash,
                self_hash,
                seq,
            ) = row.map_err(sql_error)?;
            let Some(action) = AuditAction::parse(&action_raw) else {
                return Ok(Some(id_from_raw(audit_id)?));
            };
            if seq != expected_seq || prev_hash != previous {
                return Ok(Some(id_from_raw(audit_id)?));
            }
            let expected = digest(
                &previous,
                &canonical_fields(action, &actor, occurred_at_ms, &target, &summary),
            );
            if self_hash != expected {
                return Ok(Some(id_from_raw(audit_id)?));
            }
            previous = self_hash;
            expected_seq = expected_seq.saturating_add(1);
        }
        Ok(None)
    }

    /// 已写入的审计记录条数；仅供验收入口核对条数，不暴露任何摘要内容。
    pub fn record_count(&self) -> Result<usize, StoreError> {
        let count: i64 = self
            .store
            .connection()
            .query_row("SELECT COUNT(*) FROM audit_record", [], |row| row.get(0))
            .map_err(sql_error)?;
        usize::try_from(count).map_err(|_| StoreError::Serialization("审计条数溢出".to_owned()))
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

/// 审计可用性门闸。写失败或链断裂后进入不可用态，拒绝导出、上传、删除等敏感操作。
pub struct AuditStore<'a> {
    log: SqliteAuditLog<'a>,
    unavailable: AtomicBool,
}

impl<'a> AuditStore<'a> {
    pub fn new(store: &'a SqliteStore) -> Self {
        Self {
            log: SqliteAuditLog::new(store),
            unavailable: AtomicBool::new(false),
        }
    }

    pub fn append_record(
        &self,
        action: AuditAction,
        actor: &str,
        occurred_at_ms: i64,
        target: &str,
        summary: &str,
    ) -> Result<AuditId, CopilotError> {
        match self
            .log
            .append_record(action, actor, occurred_at_ms, target, summary)
        {
            Ok(id) => Ok(id),
            Err(error) => {
                self.unavailable.store(true, Ordering::SeqCst);
                Err(CopilotError::from(error))
            }
        }
    }

    pub fn verify(&self) -> Result<Option<AuditId>, CopilotError> {
        match self.log.verify() {
            Ok(Some(id)) => {
                self.unavailable.store(true, Ordering::SeqCst);
                Ok(Some(id))
            }
            Ok(None) => Ok(None),
            Err(error) => {
                self.unavailable.store(true, Ordering::SeqCst);
                Err(CopilotError::from(error))
            }
        }
    }

    pub fn is_available(&self) -> bool {
        !self.unavailable.load(Ordering::SeqCst)
    }

    /// 已写入的审计记录条数；仅供验收入口核对条数，不暴露任何摘要内容。
    pub fn record_count(&self) -> Result<usize, CopilotError> {
        self.log.record_count().map_err(CopilotError::from)
    }

    /// 敏感操作前置检查：审计不可用时 fail-closed。
    pub fn check_sensitive(&self, operation: &str) -> Result<(), CopilotError> {
        if self.is_available() {
            Ok(())
        } else {
            Err(CopilotError::PrivacyViolation {
                detail: format!("审计不可用，已拒绝 {operation}"),
            })
        }
    }
}

impl AuditLog for AuditStore<'_> {
    fn append(&self, action: AuditAction, summary: &str) -> Result<(), CopilotError> {
        self.append_record(action, "system", now_ms(), "audit", summary)
            .map(|_| ())
    }

    fn verify(&self) -> Result<Option<AuditId>, CopilotError> {
        AuditStore::verify(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SqliteStore {
        SqliteStore::open_in_memory().expect("store")
    }

    #[test]
    fn appends_build_a_verifiable_chain() {
        let store = store();
        let log = SqliteAuditLog::new(&store);
        for index in 0..50 {
            log.append_record(
                AuditAction::CardShown,
                "actor-1",
                index,
                "card-1",
                &format!("展示第 {index} 张卡"),
            )
            .expect("append");
        }
        assert_eq!(log.verify().expect("verify"), None);
    }

    #[test]
    fn summary_with_url_or_multiline_is_rejected() {
        let store = store();
        let log = SqliteAuditLog::new(&store);
        for summary in [
            concat!("来源 ", "http", "://example.com/a"),
            "第一行\n第二行",
            "参考 www.example.com",
        ] {
            assert!(
                log.append_record(
                    AuditAction::OutboundQuery,
                    "actor-1",
                    1,
                    "search-1",
                    summary
                )
                .is_err(),
                "{summary}"
            );
        }
        assert_eq!(store.schema_version().expect("version"), 2);
    }

    #[test]
    fn tampering_is_detected() {
        let store = store();
        let log = SqliteAuditLog::new(&store);
        for index in 0..5 {
            log.append_record(
                AuditAction::Export,
                "actor-1",
                index,
                "session-1",
                "导出纪要",
            )
            .expect("append");
        }
        {
            let conn = store.connection();
            conn.execute_batch("DROP TRIGGER audit_record_no_update;")
                .expect("drop trigger");
            conn.execute(
                "UPDATE audit_record SET sanitized_summary = '被篡改' WHERE seq = 3",
                [],
            )
            .expect("tamper");
        }
        let broken = log.verify().expect("verify");
        assert_eq!(
            broken,
            Some(AuditId::new("audit-0000000000000003").expect("id"))
        );
    }

    #[test]
    fn update_and_unarchived_delete_are_rejected() {
        let store = store();
        let log = SqliteAuditLog::new(&store);
        log.append_record(AuditAction::Delete, "actor-1", 1, "session-1", "删除会话")
            .expect("append");
        {
            let conn = store.connection();
            let update = conn.execute("UPDATE audit_record SET actor = 'x' WHERE seq = 1", []);
            assert!(update.is_err());
            let delete = conn.execute("DELETE FROM audit_record WHERE seq = 1", []);
            assert!(delete.is_err());
        }
        assert!(log.verify().expect("verify").is_none());
    }

    #[test]
    fn concurrent_appends_keep_chain_continuous() {
        let store = store();
        std::thread::scope(|scope| {
            for worker in 0..10 {
                let store = &store;
                scope.spawn(move || {
                    let log = SqliteAuditLog::new(store);
                    for index in 0..10 {
                        let occurred = i64::from(worker) * 100 + index;
                        log.append_record(
                            AuditAction::OutboundQuery,
                            "actor-1",
                            occurred,
                            "query-1",
                            "外发检索",
                        )
                        .expect("append");
                    }
                });
            }
        });
        let log = SqliteAuditLog::new(&store);
        assert_eq!(log.verify().expect("verify"), None);
        let count: i64 = store
            .connection()
            .query_row("SELECT COUNT(*) FROM audit_record", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 100);
    }

    #[test]
    fn audit_unavailable_blocks_sensitive_operations() {
        let store = store();
        let audit = AuditStore::new(&store);
        assert!(audit.check_sensitive("导出纪要").is_ok());
        let long_summary = "字".repeat(MAX_SUMMARY_CHARS + 1);
        assert!(audit
            .append_record(
                AuditAction::Export,
                "actor-1",
                0,
                "session-1",
                &long_summary
            )
            .is_err());
        assert!(!audit.is_available());
        assert!(audit.check_sensitive("导出纪要").is_err());
    }

    #[test]
    fn trait_append_uses_system_actor() {
        let store = store();
        let audit = AuditStore::new(&store);
        AuditLog::append(&audit, AuditAction::SessionStart, "会话开始").expect("append");
        let actor: String = store
            .connection()
            .query_row("SELECT actor FROM audit_record WHERE seq = 1", [], |row| {
                row.get(0)
            })
            .expect("actor");
        assert_eq!(actor, "system");
    }
}
