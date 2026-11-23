//! `proactive_outbox` 仓储（SPEC 7.2.2、DESIGN 2.10.2）。

use rusqlite::{params, OptionalExtension, Row};

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::proactive::{OutboxEntry, OutboxState};
use meeting_copilot_domain::port::proactive::OutboxStore;
use meeting_copilot_domain::vo::{SessionId, TaskId};

use super::codec::{checked_usize, enum_from_str, enum_to_str};
use super::{sql_error, SqliteStore};
use crate::store::StoreError;

const COLUMNS: &str = "task_id, session_id, trigger_kind, dedupe_key, topic_cluster_id,
                       candidate_json, state, attempts, next_attempt_ms, created_at_ms, updated_at_ms";

fn decode_row(row: &Row<'_>) -> Result<OutboxEntry, StoreError> {
    let task_raw: String = row.get(0)?;
    let session_raw: Option<String> = row.get(1)?;
    let trigger_raw: String = row.get(2)?;
    let dedupe_raw: String = row.get(3)?;
    let state_raw: String = row.get(6)?;
    let attempts: i64 = row.get(7)?;
    Ok(OutboxEntry {
        task_id: TaskId::new(task_raw)
            .map_err(|error| StoreError::Serialization(error.to_string()))?,
        session_id: session_raw
            .map(SessionId::new)
            .transpose()
            .map_err(|error| StoreError::Serialization(error.to_string()))?,
        trigger_kind: enum_from_str(&trigger_raw)?,
        dedupe_key: DedupeKey::new(dedupe_raw)
            .map_err(|error| StoreError::Serialization(error.to_string()))?,
        topic_cluster_id: row.get(4)?,
        candidate_json: row.get(5)?,
        state: OutboxState::parse(&state_raw)
            .ok_or_else(|| StoreError::Serialization(format!("未知 outbox 状态 {state_raw}")))?,
        attempts: u8::try_from(attempts)
            .map_err(|_| StoreError::Serialization("attempts 溢出".to_owned()))?,
        next_attempt_ms: row.get(8)?,
        created_at_ms: row.get(9)?,
        updated_at_ms: row.get(10)?,
    })
}

impl OutboxStore for SqliteStore {
    fn insert(&self, entry: &OutboxEntry) -> Result<(), CopilotError> {
        entry
            .validate()
            .map_err(|error| CopilotError::StoreFailure {
                detail: error.to_string(),
            })?;
        let trigger_kind = enum_to_str(&entry.trigger_kind)?;
        self.lock()
            .execute(
                "INSERT INTO proactive_outbox (
                    task_id, session_id, trigger_kind, dedupe_key, topic_cluster_id,
                    candidate_json, state, attempts, next_attempt_ms, created_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    entry.task_id.as_str(),
                    entry.session_id.as_ref().map(SessionId::as_str),
                    trigger_kind,
                    entry.dedupe_key.as_str(),
                    entry.topic_cluster_id,
                    entry.candidate_json,
                    entry.state.as_str(),
                    i64::from(entry.attempts),
                    entry.next_attempt_ms,
                    entry.created_at_ms,
                    entry.updated_at_ms,
                ],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(())
    }

    fn find_active_by_dedupe(&self, dedupe_key: &str) -> Result<Option<OutboxEntry>, CopilotError> {
        let sql = format!(
            "SELECT {COLUMNS} FROM proactive_outbox
             WHERE dedupe_key = ?1 AND state IN ('pending', 'failed')
             ORDER BY created_at_ms DESC LIMIT 1"
        );
        let row = self
            .lock()
            .query_row(&sql, params![dedupe_key], |row| {
                decode_row(row).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })
            })
            .optional()
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(row)
    }

    fn recoverable(&self) -> Result<Vec<OutboxEntry>, CopilotError> {
        let sql = format!(
            "SELECT {COLUMNS} FROM proactive_outbox
             WHERE state IN ('pending', 'failed')
             ORDER BY COALESCE(next_attempt_ms, 0) ASC, created_at_ms ASC"
        );
        let conn = self.lock();
        let mut statement = conn
            .prepare(&sql)
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        let rows = statement
            .query_map([], |row| {
                decode_row(row).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })
            })
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| CopilotError::from(sql_error(error)))
    }

    fn active_count(&self) -> Result<usize, CopilotError> {
        let count: i64 = self
            .lock()
            .query_row(
                "SELECT COUNT(*) FROM proactive_outbox WHERE state IN ('pending', 'failed')",
                [],
                |row| row.get(0),
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        checked_usize(count, "proactive_outbox.count").map_err(CopilotError::from)
    }

    fn update(&self, entry: &OutboxEntry) -> Result<(), CopilotError> {
        entry
            .validate()
            .map_err(|error| CopilotError::StoreFailure {
                detail: error.to_string(),
            })?;
        self.lock()
            .execute(
                "UPDATE proactive_outbox SET
                    state = ?2, attempts = ?3, next_attempt_ms = ?4, updated_at_ms = ?5
                 WHERE task_id = ?1",
                params![
                    entry.task_id.as_str(),
                    entry.state.as_str(),
                    i64::from(entry.attempts),
                    entry.next_attempt_ms,
                    entry.updated_at_ms,
                ],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(())
    }

    fn delete_session(&self, session_id: &SessionId) -> Result<usize, CopilotError> {
        self.lock()
            .execute(
                "DELETE FROM proactive_outbox WHERE session_id = ?1",
                params![session_id.as_str()],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))
    }
}
