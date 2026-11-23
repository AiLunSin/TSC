//! `feedback_event` / `topic_weight` 仓储（SPEC 7.1.1-8）。

use rusqlite::{params, OptionalExtension};

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::feedback::{FeedbackEvent, FeedbackValue, TopicWeight};
use meeting_copilot_domain::port::proactive::FeedbackStore;
use meeting_copilot_domain::vo::{CardRef, ReferenceRef};

use super::codec::{enum_from_str, enum_to_str};
use super::{sql_error, SqliteStore};
use crate::store::StoreError;

impl FeedbackStore for SqliteStore {
    fn append_event(&self, event: &FeedbackEvent) -> Result<(), CopilotError> {
        event
            .validate()
            .map_err(|error| CopilotError::StoreFailure {
                detail: error.to_string(),
            })?;
        let trigger_kind = enum_to_str(&event.trigger_kind)?;
        let platform = enum_to_str(&event.platform)?;
        self.lock()
            .execute(
                "INSERT INTO feedback_event (
                    feedback_id, card_id, dedupe_key, topic_cluster_id, source_ref_id,
                    trigger_kind, platform, value, occurred_at_ms, weight_after_milli
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    event.feedback_id,
                    event.card_id.as_str(),
                    event.dedupe_key.as_str(),
                    event.topic_cluster_id,
                    event.source_ref_id.as_ref().map(ReferenceRef::as_str),
                    trigger_kind,
                    platform,
                    event.value.as_str(),
                    event.occurred_at_ms,
                    event.weight_after_milli,
                ],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(())
    }

    fn weight(
        &self,
        topic_cluster_id: &str,
        source_ref_id: &str,
    ) -> Result<Option<TopicWeight>, CopilotError> {
        let row = self
            .lock()
            .query_row(
                "SELECT weight_milli, blocked_until_ms, last_negative_at_ms, updated_at_ms
                 FROM topic_weight WHERE topic_cluster_id = ?1 AND source_ref_id = ?2",
                params![topic_cluster_id, source_ref_id],
                |row| {
                    Ok(TopicWeight {
                        topic_cluster_id: topic_cluster_id.to_owned(),
                        source_ref_id: source_ref_id.to_owned(),
                        weight_milli: row.get(0)?,
                        blocked_until_ms: row.get(1)?,
                        last_negative_at_ms: row.get(2)?,
                        updated_at_ms: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(row)
    }

    fn save_weight(&self, weight: &TopicWeight) -> Result<(), CopilotError> {
        weight
            .validate()
            .map_err(|error| CopilotError::StoreFailure {
                detail: error.to_string(),
            })?;
        self.lock()
            .execute(
                "INSERT INTO topic_weight (
                    topic_cluster_id, source_ref_id, weight_milli, blocked_until_ms,
                    last_negative_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(topic_cluster_id, source_ref_id) DO UPDATE SET
                    weight_milli = excluded.weight_milli,
                    blocked_until_ms = excluded.blocked_until_ms,
                    last_negative_at_ms = excluded.last_negative_at_ms,
                    updated_at_ms = excluded.updated_at_ms",
                params![
                    weight.topic_cluster_id,
                    weight.source_ref_id,
                    weight.weight_milli,
                    weight.blocked_until_ms,
                    weight.last_negative_at_ms,
                    weight.updated_at_ms,
                ],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(())
    }

    fn events(&self) -> Result<Vec<FeedbackEvent>, CopilotError> {
        let conn = self.lock();
        let mut statement = conn
            .prepare(
                "SELECT feedback_id, card_id, dedupe_key, topic_cluster_id, source_ref_id,
                        trigger_kind, platform, value, occurred_at_ms, weight_after_milli
                 FROM feedback_event ORDER BY occurred_at_ms ASC, rowid ASC",
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        let rows = statement
            .query_map([], |row| {
                let row_ref: String = row.get(1)?;
                let dedupe: String = row.get(2)?;
                let source_ref: Option<String> = row.get(4)?;
                let trigger_raw: String = row.get(5)?;
                let platform_raw: String = row.get(6)?;
                let value_raw: String = row.get(7)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row_ref,
                    dedupe,
                    row.get::<_, String>(3)?,
                    source_ref,
                    trigger_raw,
                    platform_raw,
                    value_raw,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i32>(9)?,
                ))
            })
            .map_err(|error| CopilotError::from(sql_error(error)))?;

        let mut events = Vec::new();
        for row in rows {
            let (
                feedback_id,
                card_ref,
                dedupe_key,
                topic_cluster_id,
                source_ref,
                trigger_raw,
                platform_raw,
                value_raw,
                occurred_at_ms,
                weight_after_milli,
            ) = row.map_err(|error| CopilotError::from(sql_error(error)))?;
            let value =
                FeedbackValue::parse(&value_raw).ok_or_else(|| CopilotError::StoreFailure {
                    detail: StoreError::Serialization(format!("未知反馈值 {value_raw}"))
                        .to_string(),
                })?;
            events.push(FeedbackEvent {
                feedback_id,
                card_id: CardRef::new(card_ref).map_err(|error| CopilotError::StoreFailure {
                    detail: error.to_string(),
                })?,
                dedupe_key: DedupeKey::new(dedupe_key).map_err(|error| {
                    CopilotError::StoreFailure {
                        detail: error.to_string(),
                    }
                })?,
                topic_cluster_id,
                source_ref_id: source_ref
                    .map(ReferenceRef::new)
                    .transpose()
                    .map_err(|error| CopilotError::StoreFailure {
                        detail: error.to_string(),
                    })?,
                trigger_kind: enum_from_str(&trigger_raw).map_err(CopilotError::from)?,
                platform: enum_from_str(&platform_raw).map_err(CopilotError::from)?,
                value,
                occurred_at_ms,
                weight_after_milli,
            });
        }
        Ok(events)
    }
}
