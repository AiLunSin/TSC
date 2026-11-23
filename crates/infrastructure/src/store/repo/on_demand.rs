//! `on_demand_sample` 仓储（SPEC 7.3.2）。

use rusqlite::params;

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::proactive::{OnDemandOutcome, OnDemandSample};
use meeting_copilot_domain::port::proactive::OnDemandSampleStore;
use meeting_copilot_domain::vo::ProviderId;

use super::{sql_error, SqliteStore};
use crate::store::StoreError;

impl OnDemandSampleStore for SqliteStore {
    fn append(&self, sample: &OnDemandSample) -> Result<(), CopilotError> {
        sample
            .validate()
            .map_err(|error| CopilotError::StoreFailure {
                detail: error.to_string(),
            })?;
        let latency = i64::try_from(sample.latency_ms).map_err(|_| CopilotError::StoreFailure {
            detail: "样本时延溢出".to_owned(),
        })?;
        self.lock()
            .execute(
                "INSERT INTO on_demand_sample (
                    sample_id, provider_profile_id, started_at_ms, latency_ms, outcome, cache_hit
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    sample.sample_id,
                    sample.provider_profile_id.as_str(),
                    sample.started_at_ms,
                    latency,
                    sample.outcome.as_str(),
                    i64::from(sample.cache_hit),
                ],
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        Ok(())
    }

    fn load(&self) -> Result<Vec<OnDemandSample>, CopilotError> {
        let conn = self.lock();
        let mut statement = conn
            .prepare(
                "SELECT sample_id, provider_profile_id, started_at_ms, latency_ms, outcome, cache_hit
                 FROM on_demand_sample ORDER BY started_at_ms ASC, rowid ASC",
            )
            .map_err(|error| CopilotError::from(sql_error(error)))?;
        let rows = statement
            .query_map([], |row| {
                let outcome_raw: String = row.get(4)?;
                let latency: i64 = row.get(3)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    latency,
                    outcome_raw,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|error| CopilotError::from(sql_error(error)))?;

        let mut samples = Vec::new();
        for row in rows {
            let (sample_id, provider, started_at_ms, latency, outcome_raw, cache_hit) =
                row.map_err(|error| CopilotError::from(sql_error(error)))?;
            samples.push(OnDemandSample {
                sample_id,
                provider_profile_id: ProviderId::new(provider).map_err(|error| {
                    CopilotError::StoreFailure {
                        detail: error.to_string(),
                    }
                })?,
                started_at_ms,
                latency_ms: u64::try_from(latency).map_err(|_| {
                    CopilotError::from(StoreError::Serialization("时延为负".to_owned()))
                })?,
                outcome: OnDemandOutcome::parse(&outcome_raw).ok_or_else(|| {
                    CopilotError::from(StoreError::Serialization(format!(
                        "未知补全结果 {outcome_raw}"
                    )))
                })?,
                cache_hit: cache_hit != 0,
            });
        }
        Ok(samples)
    }
}
