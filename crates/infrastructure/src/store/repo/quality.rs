//! 卡片质量事件：自动展示、前端可见、用户反馈。
//!
//! 只存卡 ID、事件枚举、来源、时间与数值；不存转写正文或关键词原文。
//! 误触发率的分母是"有反馈的自动卡"，未反馈不计入（SPEC 质量口径）。

use serde::{Deserialize, Serialize};

use super::{sql_error, SqliteStore};
use crate::store::StoreError;

pub const KIND_SHOWN: &str = "auto_shown";
pub const KIND_RENDERED: &str = "rendered";
pub const KIND_FEEDBACK: &str = "feedback";
pub const SOURCE_AUTO: &str = "auto";
pub const SOURCE_MANUAL: &str = "manual";
pub const OUTCOME_USEFUL: &str = "useful";
pub const OUTCOME_USELESS: &str = "useless";
pub const OUTCOME_WRONG: &str = "wrong";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityEvent {
    pub event_id: String,
    pub card_id: String,
    pub kind: String,
    pub source: String,
    pub at_ms: i64,
    pub latency_ms: i64,
    pub outcome: Option<String>,
}

impl QualityEvent {
    fn new(
        card_id: &str,
        kind: &str,
        source: &str,
        at_ms: u64,
        latency_ms: u64,
        outcome: Option<&str>,
    ) -> Self {
        Self {
            event_id: format!("{card_id}:{kind}"),
            card_id: card_id.to_owned(),
            kind: kind.to_owned(),
            source: source.to_owned(),
            at_ms: i64::try_from(at_ms).unwrap_or(i64::MAX),
            latency_ms: i64::try_from(latency_ms).unwrap_or(i64::MAX),
            outcome: outcome.map(str::to_owned),
        }
    }

    pub fn shown(card_id: &str, at_ms: u64, latency_ms: u64) -> Self {
        Self::new(card_id, KIND_SHOWN, SOURCE_AUTO, at_ms, latency_ms, None)
    }

    pub fn rendered(card_id: &str, at_ms: u64, latency_ms: u64, source: &str) -> Self {
        Self::new(card_id, KIND_RENDERED, source, at_ms, latency_ms, None)
    }

    pub fn feedback(card_id: &str, outcome: &str, at_ms: u64, source: &str) -> Self {
        Self::new(card_id, KIND_FEEDBACK, source, at_ms, 0, Some(outcome))
    }
}

/// 质量统计：比率为 `None` 时表示样本不足，不得按 0 展示。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub auto_shown: u64,
    pub feedback: u64,
    pub useful: u64,
    pub useless: u64,
    pub wrong: u64,
    pub mis_trigger_rate: Option<f64>,
    pub adopt_rate: Option<f64>,
    pub latency_samples: u64,
    pub latency_p50_ms: Option<u64>,
    pub latency_p95_ms: Option<u64>,
    pub latency_p99_ms: Option<u64>,
}

impl SqliteStore {
    /// 写入或覆盖一条质量事件；同一卡同一事件只保留最新值。
    pub fn record_quality_event(&self, event: &QualityEvent) -> Result<(), StoreError> {
        self.lock()
            .execute(
                "INSERT INTO quality_event
                   (event_id, event_kind, card_id, source, at_ms, latency_ms, outcome)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(event_id) DO UPDATE SET
                   source = excluded.source,
                   at_ms = excluded.at_ms,
                   latency_ms = excluded.latency_ms,
                   outcome = excluded.outcome",
                rusqlite::params![
                    event.event_id,
                    event.kind,
                    event.card_id,
                    event.source,
                    event.at_ms,
                    event.latency_ms,
                    event.outcome
                ],
            )
            .map_err(sql_error)?;
        Ok(())
    }

    /// 这张卡是否由自动路径展示过；手动卡不进误触发率分母。
    pub fn has_auto_shown(&self, card_id: &str) -> bool {
        self.auto_shown_at(card_id).is_some()
    }

    /// 自动展示时刻（毫秒）；未展示过返回 `None`。
    pub fn auto_shown_at(&self, card_id: &str) -> Option<i64> {
        self.lock()
            .query_row(
                "SELECT at_ms FROM quality_event
                 WHERE card_id = ?1 AND event_kind = ?2 AND source = ?3",
                rusqlite::params![card_id, KIND_SHOWN, SOURCE_AUTO],
                |row| row.get(0),
            )
            .ok()
    }

    /// 统计 `since_ms` 之后的自动卡质量；时间窗内的手动卡反馈不计入。
    pub fn quality_metrics(&self, since_ms: i64) -> Result<QualityMetrics, StoreError> {
        let conn = self.lock();
        let auto_shown: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM quality_event
                 WHERE event_kind = ?1 AND source = ?2 AND at_ms >= ?3",
                rusqlite::params![KIND_SHOWN, SOURCE_AUTO, since_ms],
                |row| row.get(0),
            )
            .map_err(sql_error)?;

        let mut metrics = QualityMetrics {
            auto_shown: u64::try_from(auto_shown).unwrap_or(u64::MAX),
            ..QualityMetrics::default()
        };

        {
            let mut statement = conn
                .prepare(
                    "SELECT outcome, COUNT(*) FROM quality_event
                     WHERE event_kind = ?1 AND source = ?2 AND at_ms >= ?3
                     GROUP BY outcome",
                )
                .map_err(sql_error)?;
            let rows = statement
                .query_map(
                    rusqlite::params![KIND_FEEDBACK, SOURCE_AUTO, since_ms],
                    |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?)),
                )
                .map_err(sql_error)?;
            for row in rows {
                let (outcome, count) = row.map_err(sql_error)?;
                let count = u64::try_from(count).unwrap_or(u64::MAX);
                match outcome.as_deref() {
                    Some(OUTCOME_USEFUL) => metrics.useful = count,
                    Some(OUTCOME_USELESS) => metrics.useless = count,
                    Some(OUTCOME_WRONG) => metrics.wrong = count,
                    _ => {}
                }
            }
        }

        metrics.feedback = metrics.useful + metrics.useless + metrics.wrong;
        if metrics.feedback > 0 {
            let bad = metrics.useless + metrics.wrong;
            metrics.mis_trigger_rate = Some(bad as f64 / metrics.feedback as f64);
            metrics.adopt_rate = Some(metrics.useful as f64 / metrics.feedback as f64);
        }

        let mut latencies = Vec::new();
        {
            let mut statement = conn
                .prepare(
                    "SELECT latency_ms FROM quality_event
                     WHERE event_kind = ?1 AND source = ?2 AND at_ms >= ?3
                     ORDER BY latency_ms ASC",
                )
                .map_err(sql_error)?;
            let rows = statement
                .query_map(
                    rusqlite::params![KIND_RENDERED, SOURCE_AUTO, since_ms],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(sql_error)?;
            for row in rows {
                latencies.push(u64::try_from(row.map_err(sql_error)?).unwrap_or(0));
            }
        }
        metrics.latency_samples = u64::try_from(latencies.len()).unwrap_or(u64::MAX);
        metrics.latency_p50_ms = percentile(&latencies, 0.50);
        metrics.latency_p95_ms = percentile(&latencies, 0.95);
        metrics.latency_p99_ms = percentile(&latencies, 0.99);
        Ok(metrics)
    }
}

/// 最近秩法；`sorted` 必须已升序。
fn percentile(sorted: &[u64], p: f64) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = (p * sorted.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SqliteStore {
        SqliteStore::open_in_memory().expect("memory store")
    }

    #[test]
    fn rates_need_feedback_and_ignore_unfeedbacked_cards() {
        let store = store();
        for card in ["card-1", "card-2", "card-3"] {
            store
                .record_quality_event(&QualityEvent::shown(card, 1_000, 2))
                .expect("shown");
        }
        let metrics = store.quality_metrics(0).expect("metrics");
        assert_eq!(metrics.auto_shown, 3);
        assert_eq!(metrics.feedback, 0);
        assert_eq!(metrics.mis_trigger_rate, None);
        assert_eq!(metrics.adopt_rate, None);

        store
            .record_quality_event(&QualityEvent::feedback(
                "card-1",
                OUTCOME_USEFUL,
                1_100,
                SOURCE_AUTO,
            ))
            .expect("useful");
        let metrics = store.quality_metrics(0).expect("metrics");
        assert_eq!(metrics.feedback, 1);
        assert_eq!(metrics.mis_trigger_rate, Some(0.0));
        assert_eq!(metrics.adopt_rate, Some(1.0));

        for (card, outcome) in [("card-2", OUTCOME_USELESS), ("card-3", OUTCOME_WRONG)] {
            store
                .record_quality_event(&QualityEvent::feedback(card, outcome, 1_200, SOURCE_AUTO))
                .expect("negative");
        }
        let metrics = store.quality_metrics(0).expect("metrics");
        assert_eq!(metrics.feedback, 3);
        assert_eq!(metrics.useful, 1);
        assert_eq!(metrics.useless, 1);
        assert_eq!(metrics.wrong, 1);
        assert!((metrics.mis_trigger_rate.expect("rate") - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn manual_feedback_does_not_enter_auto_rates() {
        let store = store();
        store
            .record_quality_event(&QualityEvent::feedback(
                "manual-1",
                OUTCOME_WRONG,
                10,
                SOURCE_MANUAL,
            ))
            .expect("feedback");
        let metrics = store.quality_metrics(0).expect("metrics");
        assert_eq!(metrics.feedback, 0);
        assert_eq!(metrics.mis_trigger_rate, None);
    }

    #[test]
    fn rendered_is_upserted_and_latency_percentiles_are_reported() {
        let store = store();
        store
            .record_quality_event(&QualityEvent::shown("card-1", 1_000, 2))
            .expect("shown");
        store
            .record_quality_event(&QualityEvent::rendered("card-1", 2_500, 1_500, SOURCE_AUTO))
            .expect("rendered");
        store
            .record_quality_event(&QualityEvent::rendered("card-1", 2_600, 1_600, SOURCE_AUTO))
            .expect("rendered again");
        let rendered: i64 = store
            .lock()
            .query_row(
                "SELECT COUNT(*) FROM quality_event WHERE event_kind = ?1",
                [KIND_RENDERED],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(rendered, 1, "同卡可见事件只保留一条");

        store
            .record_quality_event(&QualityEvent::rendered("card-2", 3_000, 2_000, SOURCE_AUTO))
            .expect("rendered");
        let metrics = store.quality_metrics(0).expect("metrics");
        assert_eq!(metrics.latency_samples, 2);
        assert_eq!(metrics.latency_p50_ms, Some(1_600));
        assert_eq!(metrics.latency_p95_ms, Some(2_000));
    }

    #[test]
    fn auto_shown_lookup_distinguishes_manual_cards() {
        let store = store();
        store
            .record_quality_event(&QualityEvent::shown("auto-1", 1_000, 1))
            .expect("shown");
        assert!(store.has_auto_shown("auto-1"));
        assert_eq!(store.auto_shown_at("auto-1"), Some(1_000));
        assert!(!store.has_auto_shown("manual-1"));
        assert_eq!(store.auto_shown_at("manual-1"), None);
    }
}
