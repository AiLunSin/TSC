//! 审计留存与按月归档（SPEC 7.4.1-6、DESIGN 2.10.4）。
//!
//! 线上保留 180 天；到期自然月整月加密封存后才允许删除在线行。
//! 归档包再保留 5 年的期限仍待法务确认，因此只标记 `retained`。

use std::path::{Path, PathBuf};

use rusqlite::{params, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::audit::store::SqliteAuditLog;
use crate::crypto::MASTER_KEY_LEN;
use crate::store::{SqliteStore, StoreError};

/// 线上审计保留天数。
pub const ONLINE_RETENTION_DAYS: i64 = 180;
/// 归档保留年限（待法务确认，仅标记）。
pub const ARCHIVE_RETENTION_YEARS: u16 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ArchivedRecord {
    seq: i64,
    audit_id: String,
    action: String,
    actor: String,
    occurred_at_ms: i64,
    target: String,
    sanitized_summary: String,
    prev_hash: String,
    self_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ArchivedAnchor {
    up_to_seq: i64,
    last_self_hash: String,
    anchor_hash: String,
    created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ArchiveBundle {
    month: String,
    first_seq: i64,
    last_seq: i64,
    record_count: u64,
    terminal_self_hash: String,
    records: Vec<ArchivedRecord>,
    anchors: Vec<ArchivedAnchor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveOutcome {
    pub month: String,
    pub first_seq: i64,
    pub last_seq: i64,
    pub record_count: u64,
    pub bundle_path: PathBuf,
    pub bundle_sha256: String,
}

fn sql_error(error: rusqlite::Error) -> StoreError {
    StoreError::Sqlite(error.to_string())
}

fn cutoff_ms(now_ms: i64) -> i64 {
    now_ms.saturating_sub(ONLINE_RETENTION_DAYS * 24 * 60 * 60 * 1_000)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// 归档所有"整月都早于 180 天"的分段。归档前先全链校验；任一步失败都保留在线数据。
pub fn archive_due(
    store: &SqliteStore,
    key: &[u8; MASTER_KEY_LEN],
    bundle_dir: &Path,
    now_ms: i64,
) -> Result<Vec<ArchiveOutcome>, StoreError> {
    let log = SqliteAuditLog::new(store);
    if let Some(broken) = log.verify()? {
        return Err(StoreError::MigrationFailed(format!(
            "审计链断裂于 {broken}，拒绝归档"
        )));
    }

    let due = {
        let conn = store.connection();
        let mut statement = conn
            .prepare(
                "SELECT month, first_seq, last_seq, record_count FROM (
                    SELECT strftime('%Y-%m', occurred_at_ms / 1000, 'unixepoch') AS month,
                           MIN(seq) AS first_seq,
                           MAX(seq) AS last_seq,
                           COUNT(*) AS record_count,
                           MAX(occurred_at_ms) AS max_occurred
                    FROM audit_record
                    GROUP BY month
                 ) grouped
                 WHERE grouped.max_occurred < ?1
                 ORDER BY month ASC",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map(params![cutoff_ms(now_ms)], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)?
    };
    if due.is_empty() {
        return Ok(Vec::new());
    }

    std::fs::create_dir_all(bundle_dir)
        .map_err(|error| StoreError::MigrationFailed(format!("创建归档目录失败：{error}")))?;

    let mut outcomes = Vec::new();
    for (month, first_seq, last_seq, record_count) in due {
        let (records, anchors, terminal) = collect_segment(store, first_seq, last_seq)?;
        let bundle = ArchiveBundle {
            month: month.clone(),
            first_seq,
            last_seq,
            record_count: u64::try_from(record_count).unwrap_or(0),
            terminal_self_hash: terminal.clone(),
            records,
            anchors,
        };
        let plaintext = serde_json::to_vec(&bundle)
            .map_err(|error| StoreError::Serialization(error.to_string()))?;
        let aad = format!("{month}:{first_seq}:{last_seq}");
        let envelope = crate::crypto::encrypt_field(key, &aad, &plaintext)
            .map_err(|error| StoreError::Serialization(error.to_string()))?;
        let bundle_path = bundle_dir.join(format!("audit-{month}.bin"));
        std::fs::write(&bundle_path, &envelope)
            .map_err(|error| StoreError::MigrationFailed(format!("写入归档包失败：{error}")))?;
        let bundle_sha256 = sha256_hex(&envelope);

        let mut conn = store.connection();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error)?;
        tx.execute(
            "INSERT INTO audit_archive (
                month, first_seq, last_seq, record_count, terminal_self_hash,
                bundle_path, bundle_sha256, sealed_at_ms, disposal_state
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'retained')
             ON CONFLICT(month) DO UPDATE SET
                first_seq = excluded.first_seq,
                last_seq = excluded.last_seq,
                record_count = excluded.record_count,
                terminal_self_hash = excluded.terminal_self_hash,
                bundle_path = excluded.bundle_path,
                bundle_sha256 = excluded.bundle_sha256,
                sealed_at_ms = excluded.sealed_at_ms",
            params![
                month,
                first_seq,
                last_seq,
                record_count,
                terminal,
                bundle_path.to_string_lossy(),
                bundle_sha256,
                now_ms,
            ],
        )
        .map_err(sql_error)?;
        tx.execute(
            "DELETE FROM audit_record WHERE seq BETWEEN ?1 AND ?2",
            params![first_seq, last_seq],
        )
        .map_err(sql_error)?;
        tx.commit().map_err(sql_error)?;

        outcomes.push(ArchiveOutcome {
            month,
            first_seq,
            last_seq,
            record_count: u64::try_from(record_count).unwrap_or(0),
            bundle_path,
            bundle_sha256,
        });
    }
    Ok(outcomes)
}

fn collect_segment(
    store: &SqliteStore,
    first_seq: i64,
    last_seq: i64,
) -> Result<(Vec<ArchivedRecord>, Vec<ArchivedAnchor>, String), StoreError> {
    let conn = store.connection();
    let mut statement = conn
        .prepare(
            "SELECT seq, audit_id, action, actor, occurred_at_ms, target,
                    sanitized_summary, prev_hash, self_hash
             FROM audit_record WHERE seq BETWEEN ?1 AND ?2 ORDER BY seq ASC",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map(params![first_seq, last_seq], |row| {
            Ok(ArchivedRecord {
                seq: row.get(0)?,
                audit_id: row.get(1)?,
                action: row.get(2)?,
                actor: row.get(3)?,
                occurred_at_ms: row.get(4)?,
                target: row.get(5)?,
                sanitized_summary: row.get(6)?,
                prev_hash: row.get(7)?,
                self_hash: row.get(8)?,
            })
        })
        .map_err(sql_error)?;
    let records = rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)?;
    let terminal = records
        .last()
        .map(|record| record.self_hash.clone())
        .ok_or_else(|| StoreError::MigrationFailed("归档分段为空".to_owned()))?;

    let mut anchor_statement = conn
        .prepare(
            "SELECT up_to_seq, last_self_hash, anchor_hash, created_at_ms
             FROM audit_anchor WHERE up_to_seq BETWEEN ?1 AND ?2 ORDER BY up_to_seq ASC",
        )
        .map_err(sql_error)?;
    let anchor_rows = anchor_statement
        .query_map(params![first_seq, last_seq], |row| {
            Ok(ArchivedAnchor {
                up_to_seq: row.get(0)?,
                last_self_hash: row.get(1)?,
                anchor_hash: row.get(2)?,
                created_at_ms: row.get(3)?,
            })
        })
        .map_err(sql_error)?;
    let anchors = anchor_rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)?;

    Ok((records, anchors, terminal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::store::SqliteAuditLog;
    use meeting_copilot_domain::model::enums::AuditAction;

    fn temp_dir(tag: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let dir = std::env::temp_dir().join(format!(
            "meeting-copilot-{tag}-{}-{unique}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn old_months_are_sealed_then_removed() {
        let store = SqliteStore::open_in_memory().expect("store");
        let log = SqliteAuditLog::new(&store);
        let now = 1_800_000_000_000_i64;
        let old = now - (ONLINE_RETENTION_DAYS + 20) * 24 * 60 * 60 * 1_000;
        for index in 0..3 {
            log.append_record(
                AuditAction::CardShown,
                "actor-1",
                old + index,
                "card-1",
                "展示卡片",
            )
            .expect("append");
        }
        log.append_record(
            AuditAction::SessionStart,
            "actor-1",
            now,
            "session-1",
            "会话开始",
        )
        .expect("append");

        let dir = temp_dir("audit-archive");
        let key = [7_u8; MASTER_KEY_LEN];
        let outcomes = archive_due(&store, &key, &dir, now).expect("archive");
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].record_count, 3);
        assert_eq!(outcomes[0].first_seq, 1);
        assert_eq!(outcomes[0].last_seq, 3);

        let remaining: i64 = store
            .connection()
            .query_row("SELECT COUNT(*) FROM audit_record", [], |row| row.get(0))
            .expect("count");
        assert_eq!(remaining, 1);
        assert!(SqliteAuditLog::new(&store)
            .verify()
            .expect("verify")
            .is_none());

        let envelope = std::fs::read(&outcomes[0].bundle_path).expect("bundle");
        assert_eq!(sha256_hex(&envelope), outcomes[0].bundle_sha256);
        let aad = format!(
            "{}:{}:{}",
            outcomes[0].month, outcomes[0].first_seq, outcomes[0].last_seq
        );
        let plaintext = crate::crypto::decrypt_field(&key, &aad, &envelope).expect("decrypt");
        let bundle: ArchiveBundle = serde_json::from_slice(&plaintext).expect("bundle json");
        assert_eq!(bundle.record_count, 3);
        assert_eq!(bundle.records.len(), 3);
        assert_eq!(bundle.month, outcomes[0].month);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_failure_keeps_online_rows() {
        let store = SqliteStore::open_in_memory().expect("store");
        let log = SqliteAuditLog::new(&store);
        let now = 1_800_000_000_000_i64;
        let old = now - (ONLINE_RETENTION_DAYS + 20) * 24 * 60 * 60 * 1_000;
        log.append_record(AuditAction::CardShown, "actor-1", old, "card-1", "展示")
            .expect("append");

        let dir = temp_dir("audit-blocked");
        std::fs::create_dir_all(dir.parent().expect("parent")).expect("parent dir");
        std::fs::write(&dir, b"not a directory").expect("blocker");
        let result = archive_due(&store, &[1_u8; MASTER_KEY_LEN], &dir, now);
        assert!(result.is_err());
        let remaining: i64 = store
            .connection()
            .query_row("SELECT COUNT(*) FROM audit_record", [], |row| row.get(0))
            .expect("count");
        assert_eq!(remaining, 1);
        let _ = std::fs::remove_file(&dir);
    }
}
