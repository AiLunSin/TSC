//! 卡片列表持久化：重启后仍能看到本次会话的卡片（字段级 AES-256-GCM）。
//!
//! 只存重建卡片所需的三项：标题、释义、网页来源；不含转写正文与门控内部字段。

use serde::{Deserialize, Serialize};

use super::{sql_error, SqliteStore};
use crate::store::StoreError;

/// 重启后重建一张卡片所需的最小数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredCard {
    pub card_id: String,
    pub title: String,
    pub explanation: String,
    pub sources: Vec<String>,
    pub created_at_ms: i64,
}

impl SqliteStore {
    /// 保存/更新一张卡片快照。
    pub fn save_card_snapshot(
        &self,
        session_id: &str,
        record: &StoredCard,
    ) -> Result<(), StoreError> {
        let payload = self.encode_payload(session_id, record)?;
        self.lock()
            .execute(
                "INSERT INTO card_snapshot (session_id, card_id, payload, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(session_id, card_id) DO UPDATE SET
                   payload = excluded.payload,
                   created_at_ms = excluded.created_at_ms",
                rusqlite::params![session_id, record.card_id, payload, record.created_at_ms],
            )
            .map_err(sql_error)?;
        Ok(())
    }

    /// 取最近 `limit` 张卡片，按创建时间升序返回（便于按原顺序回灌）。
    pub fn load_recent_cards(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<StoredCard>, StoreError> {
        let conn = self.lock();
        let mut statement = conn
            .prepare(
                "SELECT payload FROM card_snapshot WHERE session_id = ?1
                 ORDER BY created_at_ms DESC LIMIT ?2",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map(rusqlite::params![session_id, limit as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(sql_error)?;
        let mut payloads = Vec::new();
        for row in rows {
            payloads.push(row.map_err(sql_error)?);
        }
        drop(statement);
        // 查询按时间倒序取最近 N 条，回灌时再翻回升序。
        payloads.reverse();
        payloads
            .into_iter()
            .map(|payload| self.decode_payload(session_id, &payload))
            .collect()
    }

    /// 清空本次会话的卡片快照。
    pub fn delete_card_snapshots(&self, session_id: &str) -> Result<usize, StoreError> {
        self.lock()
            .execute(
                "DELETE FROM card_snapshot WHERE session_id = ?1",
                rusqlite::params![session_id],
            )
            .map_err(sql_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(card_id: &str, at: i64) -> StoredCard {
        StoredCard {
            card_id: card_id.to_owned(),
            title: "灰度发布".to_owned(),
            explanation: "分批次放量。".to_owned(),
            sources: vec!["example.com".to_owned()],
            created_at_ms: at,
        }
    }

    #[test]
    fn card_snapshot_roundtrips_and_stays_encrypted() {
        let store = SqliteStore::open_in_memory().expect("memory store");
        store
            .save_card_snapshot("s1", &record("card-1", 1))
            .expect("save");
        let cards = store.load_recent_cards("s1", 10).expect("load");
        assert_eq!(cards, vec![record("card-1", 1)]);
        let raw: String = store
            .lock()
            .query_row("SELECT payload FROM card_snapshot", [], |row| row.get(0))
            .expect("payload");
        assert!(!raw.contains("灰度发布"), "载荷不得含明文");
        assert!(raw.starts_with("enc:v1:"), "载荷应为字段级密文");
    }

    #[test]
    fn recent_cards_are_ordered_and_limited() {
        let store = SqliteStore::open_in_memory().expect("memory store");
        for (index, at) in [(1, 10), (2, 20), (3, 30)] {
            store
                .save_card_snapshot("s1", &record(&format!("card-{index}"), at))
                .expect("save");
        }
        let cards = store.load_recent_cards("s1", 2).expect("load");
        assert_eq!(
            cards.iter().map(|c| c.card_id.as_str()).collect::<Vec<_>>(),
            vec!["card-2", "card-3"]
        );
        assert_eq!(store.delete_card_snapshots("s1").expect("delete"), 3);
        assert!(store.load_recent_cards("s1", 10).expect("load").is_empty());
    }

    #[test]
    fn unknown_session_returns_empty() {
        let store = SqliteStore::open_in_memory().expect("memory store");
        assert!(store
            .load_recent_cards("missing", 10)
            .expect("load")
            .is_empty());
    }
}
