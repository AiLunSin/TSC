//! 断网补齐查询词：进程重启后重建查询所需的加密落盘（SPEC 7.2.1-8a）。
//!
//! 与 `proactive_outbox` 分开：outbox 自身不保存查询原文（7.2.1-11），
//! 本表只保留**活跃任务**用到的词，任务完成或失效后即删除，避免会议内容残留。
//! 载荷走与其他业务表相同的字段级 AES-256-GCM。

use rusqlite::OptionalExtension;

use super::{sql_error, SqliteStore};
use crate::store::StoreError;

impl SqliteStore {
    /// 落一个待补任务的查询词；同 `dedupe_key` 覆盖为最新值。
    pub fn save_backfill_word(
        &self,
        session_id: &str,
        dedupe_key: &str,
        word: &str,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let payload = self.encode_payload(session_id, &word.to_owned())?;
        self.lock()
            .execute(
                "INSERT INTO backfill_word (session_id, dedupe_key, payload, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(session_id, dedupe_key) DO UPDATE SET
                   payload = excluded.payload,
                   created_at_ms = excluded.created_at_ms",
                rusqlite::params![session_id, dedupe_key, payload, now_ms],
            )
            .map_err(sql_error)?;
        Ok(())
    }

    /// 取回一个待补任务的查询词；没有记录时返回 `None`。
    pub fn load_backfill_word(
        &self,
        session_id: &str,
        dedupe_key: &str,
    ) -> Result<Option<String>, StoreError> {
        let payload: Option<String> = self
            .lock()
            .query_row(
                "SELECT payload FROM backfill_word WHERE session_id = ?1 AND dedupe_key = ?2",
                rusqlite::params![session_id, dedupe_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_error)?;
        payload
            .map(|value| self.decode_payload(session_id, &value))
            .transpose()
    }

    /// 任务完成/失效后删除对应词。
    pub fn delete_backfill_word(
        &self,
        session_id: &str,
        dedupe_key: &str,
    ) -> Result<(), StoreError> {
        self.lock()
            .execute(
                "DELETE FROM backfill_word WHERE session_id = ?1 AND dedupe_key = ?2",
                rusqlite::params![session_id, dedupe_key],
            )
            .map_err(sql_error)?;
        Ok(())
    }

    /// 清掉除 `keep_dedupes` 之外的词（启动时清理已完成/已失效任务的残留）。
    pub fn purge_backfill_words(
        &self,
        session_id: &str,
        keep_dedupes: &[String],
    ) -> Result<usize, StoreError> {
        if keep_dedupes.is_empty() {
            return self
                .lock()
                .execute(
                    "DELETE FROM backfill_word WHERE session_id = ?1",
                    rusqlite::params![session_id],
                )
                .map_err(sql_error);
        }
        let placeholders = vec!["?"; keep_dedupes.len()].join(",");
        let sql = format!(
            "DELETE FROM backfill_word WHERE session_id = ?1 AND dedupe_key NOT IN ({placeholders})"
        );
        let mut values: Vec<rusqlite::types::Value> = Vec::with_capacity(keep_dedupes.len() + 1);
        values.push(session_id.to_owned().into());
        for dedupe in keep_dedupes {
            values.push(dedupe.clone().into());
        }
        self.lock()
            .execute(&sql, rusqlite::params_from_iter(values))
            .map_err(sql_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_roundtrips_and_payload_is_encrypted() {
        let store = SqliteStore::open_in_memory().expect("memory store");
        store
            .save_backfill_word("s1", "dedupe-1", "灰度发布", 1_000)
            .expect("save");
        assert_eq!(
            store.load_backfill_word("s1", "dedupe-1").expect("load"),
            Some("灰度发布".to_owned())
        );
        let raw: String = store
            .lock()
            .query_row("SELECT payload FROM backfill_word", [], |row| row.get(0))
            .expect("payload");
        assert!(!raw.contains("灰度发布"), "载荷不得含明文");
        assert!(raw.starts_with("enc:v1:"), "载荷应为字段级密文");
    }

    #[test]
    fn delete_and_purge_keep_only_active_words() {
        let store = SqliteStore::open_in_memory().expect("memory store");
        store
            .save_backfill_word("s1", "keep", "保留", 1)
            .expect("save keep");
        store
            .save_backfill_word("s1", "drop", "过期", 2)
            .expect("save drop");
        store.delete_backfill_word("s1", "drop").expect("delete");
        assert!(store
            .load_backfill_word("s1", "drop")
            .expect("load")
            .is_none());

        store
            .save_backfill_word("s1", "drop2", "再过期", 3)
            .expect("save drop2");
        let removed = store
            .purge_backfill_words("s1", &["keep".to_owned()])
            .expect("purge");
        assert_eq!(removed, 1);
        assert_eq!(
            store.load_backfill_word("s1", "keep").expect("load"),
            Some("保留".to_owned())
        );
    }
}
