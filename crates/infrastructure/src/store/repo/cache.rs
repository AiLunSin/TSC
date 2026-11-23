//! `query_cache` 仓储（SPEC 7.2.1、DESIGN 2.10.2）。

use rusqlite::{params, OptionalExtension};

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::card::Reference;
use meeting_copilot_domain::model::proactive::{CachedQuery, CACHE_MAX_BYTES, CACHE_MAX_ENTRIES};
use meeting_copilot_domain::port::proactive::QueryCacheStore;
use meeting_copilot_domain::vo::{ProviderId, SessionId};

use super::codec::checked_usize;
use super::{sql_error, SqliteStore};
use crate::store::StoreError;

impl QueryCacheStore for SqliteStore {
    fn get(&self, query_hash: &str) -> Result<Option<CachedQuery>, CopilotError> {
        let row = self
            .lock()
            .query_row(
                "SELECT provider_profile_id, session_id, fetched_at_ms, expires_at_ms,
                        payload_refs_json, byte_size, last_access_ms
                 FROM query_cache WHERE query_hash = ?1",
                params![query_hash],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        let Some((
            provider,
            session,
            fetched_at_ms,
            expires_at_ms,
            payload,
            byte_size,
            last_access_ms,
        )) = row
        else {
            return Ok(None);
        };
        let references: Vec<Reference> = serde_json::from_str(&payload)
            .map_err(|error| CopilotError::from(StoreError::Serialization(error.to_string())))?;
        Ok(Some(CachedQuery {
            query_hash: query_hash.to_owned(),
            provider_profile_id: ProviderId::new(provider).map_err(|error| {
                CopilotError::StoreFailure {
                    detail: error.to_string(),
                }
            })?,
            session_id: session.map(SessionId::new).transpose().map_err(|error| {
                CopilotError::StoreFailure {
                    detail: error.to_string(),
                }
            })?,
            fetched_at_ms,
            expires_at_ms,
            references,
            byte_size: checked_usize(byte_size, "query_cache.byte_size")?,
            last_access_ms,
        }))
    }

    fn put(&self, entry: &CachedQuery) -> Result<usize, CopilotError> {
        entry
            .validate()
            .map_err(|error| CopilotError::StoreFailure {
                detail: error.to_string(),
            })?;
        let payload = serde_json::to_string(&entry.references)
            .map_err(|error| CopilotError::from(StoreError::Serialization(error.to_string())))?;
        let byte_size = i64::try_from(entry.byte_size).unwrap_or(i64::MAX);
        let mut conn = self.lock();
        let tx = conn
            .transaction()
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        tx.execute(
            "DELETE FROM query_cache WHERE expires_at_ms <= ?1",
            params![entry.last_access_ms],
        )
        .map_err(|error| CopilotError::from(sql_error(error)))?;
        tx.execute(
            "INSERT INTO query_cache (
                query_hash, provider_profile_id, session_id, fetched_at_ms, expires_at_ms,
                payload_refs_json, byte_size, last_access_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(query_hash) DO UPDATE SET
                provider_profile_id = excluded.provider_profile_id,
                session_id = excluded.session_id,
                fetched_at_ms = excluded.fetched_at_ms,
                expires_at_ms = excluded.expires_at_ms,
                payload_refs_json = excluded.payload_refs_json,
                byte_size = excluded.byte_size,
                last_access_ms = excluded.last_access_ms",
            params![
                entry.query_hash,
                entry.provider_profile_id.as_str(),
                entry.session_id.as_ref().map(SessionId::as_str),
                entry.fetched_at_ms,
                entry.expires_at_ms,
                payload,
                byte_size,
                entry.last_access_ms,
            ],
        )
        .map_err(|error| CopilotError::from(sql_error(error)))?;

        let mut evicted = 0_usize;
        loop {
            let (count, bytes): (i64, i64) = tx
                .query_row(
                    "SELECT COUNT(*), COALESCE(SUM(byte_size), 0) FROM query_cache",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|error| CopilotError::from(sql_error(error)))?;
            let count = checked_usize(count, "query_cache.count")?;
            let bytes = checked_usize(bytes, "query_cache.bytes")?;
            if count <= CACHE_MAX_ENTRIES && bytes <= CACHE_MAX_BYTES {
                break;
            }
            let removed = tx
                .execute(
                    "DELETE FROM query_cache WHERE query_hash = (
                        SELECT query_hash FROM query_cache
                        ORDER BY last_access_ms ASC, query_hash ASC LIMIT 1
                     )",
                    [],
                )
                .map_err(|error| CopilotError::from(sql_error(error)))?;
            if removed == 0 {
                break;
            }
            evicted += removed;
        }
        tx.commit()
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(evicted)
    }

    fn touch(&self, query_hash: &str, at_ms: i64) -> Result<(), CopilotError> {
        self.lock()
            .execute(
                "UPDATE query_cache SET last_access_ms = ?1 WHERE query_hash = ?2",
                params![at_ms, query_hash],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(())
    }

    fn sweep_expired(&self, now_ms: i64) -> Result<usize, CopilotError> {
        self.lock()
            .execute(
                "DELETE FROM query_cache WHERE expires_at_ms <= ?1",
                params![now_ms],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))
    }

    fn delete_session(&self, session_id: &SessionId) -> Result<usize, CopilotError> {
        self.lock()
            .execute(
                "DELETE FROM query_cache WHERE session_id = ?1",
                params![session_id.as_str()],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))
    }
}
