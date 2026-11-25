//! 断网出卡的最小卡片副本：进程重启后据此重建卡片并补来源（SPEC 7.2.1-8a）。
//!
//! 与 `proactive_outbox` 分开：outbox 不保存内容字段（7.2.1-11），
//! 本表存字段级加密的「标题 + 释义」，只服务活跃补齐任务，补齐完成即删除。

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use super::{sql_error, SqliteStore};
use crate::store::StoreError;

/// 重启后重建卡片所需的最小字段；不含转写正文。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackfillCardRecord {
    pub card_id: String,
    pub title: String,
    pub explanation: String,
}

impl SqliteStore {
    /// 落一份断网卡片的加密副本；同 `dedupe_key` 覆盖为最新。
    pub fn save_backfill_card(
        &self,
        session_id: &str,
        dedupe_key: &str,
        record: &BackfillCardRecord,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let payload = self.encode_payload(session_id, record)?;
        self.lock()
            .execute(
                "INSERT INTO backfill_card (session_id, dedupe_key, payload, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(session_id, dedupe_key) DO UPDATE SET
                   payload = excluded.payload,
                   created_at_ms = excluded.created_at_ms",
                rusqlite::params![session_id, dedupe_key, payload, now_ms],
            )
            .map_err(sql_error)?;
        Ok(())
    }

    /// 取回断网卡片的加密副本；没有记录时返回 `None`。
    pub fn load_backfill_card(
        &self,
        session_id: &str,
        dedupe_key: &str,
    ) -> Result<Option<BackfillCardRecord>, StoreError> {
        let payload: Option<String> = self
            .lock()
            .query_row(
                "SELECT payload FROM backfill_card WHERE session_id = ?1 AND dedupe_key = ?2",
                rusqlite::params![session_id, dedupe_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_error)?;
        payload
            .map(|value| self.decode_payload(session_id, &value))
            .transpose()
    }

    /// 补齐完成后删除卡片副本。
    pub fn delete_backfill_card(
        &self,
        session_id: &str,
        dedupe_key: &str,
    ) -> Result<(), StoreError> {
        self.lock()
            .execute(
                "DELETE FROM backfill_card WHERE session_id = ?1 AND dedupe_key = ?2",
                rusqlite::params![session_id, dedupe_key],
            )
            .map_err(sql_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> BackfillCardRecord {
        BackfillCardRecord {
            card_id: "card-0000000000000001".to_owned(),
            title: "灰度发布".to_owned(),
            explanation: "分批次放量，先小范围验证。".to_owned(),
        }
    }

    #[test]
    fn card_roundtrips_and_payload_is_encrypted() {
        let store = SqliteStore::open_in_memory().expect("memory store");
        store
            .save_backfill_card("s1", "dedupe-1", &record(), 1)
            .expect("save");
        assert_eq!(
            store.load_backfill_card("s1", "dedupe-1").expect("load"),
            Some(record())
        );
        let raw: String = store
            .lock()
            .query_row("SELECT payload FROM backfill_card", [], |row| row.get(0))
            .expect("payload");
        assert!(!raw.contains("灰度发布"), "载荷不得含明文");
        assert!(raw.starts_with("enc:v1:"), "载荷应为字段级密文");
    }

    #[test]
    fn delete_removes_card_copy() {
        let store = SqliteStore::open_in_memory().expect("memory store");
        store
            .save_backfill_card("s1", "dedupe-1", &record(), 1)
            .expect("save");
        store
            .delete_backfill_card("s1", "dedupe-1")
            .expect("delete");
        assert!(store
            .load_backfill_card("s1", "dedupe-1")
            .expect("load")
            .is_none());
    }
}
