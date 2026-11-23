//! 按需补全首卡时延与样本充分性（SPEC 7.3.2、DESIGN 2.10.3）。

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::proactive::{
    OnDemandOutcome, OnDemandSample, ON_DEMAND_P95_BUDGET_MS, ON_DEMAND_REQUIRED_SAMPLES,
    ON_DEMAND_TIMEOUT_RATE_MAX,
};
use meeting_copilot_domain::port::proactive::OnDemandSampleStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleStatus {
    Sufficient,
    Insufficient,
}

impl SampleStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sufficient => "样本充足",
            Self::Insufficient => "样本不足",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnDemandReport {
    /// 未命中缓存的成功样本数，只有它计入 200 条门槛与分位数。
    pub real_sample_count: usize,
    pub p50_ms: Option<u64>,
    pub p95_ms: Option<u64>,
    pub p99_ms: Option<u64>,
    pub timeout_rate: Option<f64>,
    pub cache_hit_rate: Option<f64>,
    pub status: SampleStatus,
    pub p95_within_budget: Option<bool>,
    pub timeout_within_budget: Option<bool>,
}

/// 触发到首卡的毫秒数；首卡早于触发或溢出时返回 `None`。
pub fn measure_first_card_ms(triggered_at_ms: i64, first_card_at_ms: i64) -> Option<u64> {
    if first_card_at_ms < triggered_at_ms {
        return None;
    }
    u64::try_from(first_card_at_ms - triggered_at_ms).ok()
}

pub fn summarize(samples: &[OnDemandSample]) -> OnDemandReport {
    let mut latencies: Vec<u64> = samples
        .iter()
        .filter(|sample| !sample.cache_hit && sample.outcome == OnDemandOutcome::Success)
        .map(|sample| sample.latency_ms)
        .collect();
    latencies.sort_unstable();

    let real_sample_count = latencies.len();
    let total = samples.len();
    let cache_hits = samples.iter().filter(|sample| sample.cache_hit).count();
    let timeouts = samples
        .iter()
        .filter(|sample| sample.outcome == OnDemandOutcome::Timeout)
        .count();

    let p95_ms = percentile(&latencies, 95);
    let timeout_rate = rate(timeouts, total);
    let status = if real_sample_count >= ON_DEMAND_REQUIRED_SAMPLES {
        SampleStatus::Sufficient
    } else {
        SampleStatus::Insufficient
    };

    OnDemandReport {
        real_sample_count,
        p50_ms: percentile(&latencies, 50),
        p95_ms,
        p99_ms: percentile(&latencies, 99),
        timeout_rate,
        cache_hit_rate: rate(cache_hits, total),
        status,
        p95_within_budget: p95_ms.map(|value| value <= ON_DEMAND_P95_BUDGET_MS),
        timeout_within_budget: timeout_rate.map(|value| value <= ON_DEMAND_TIMEOUT_RATE_MAX),
    }
}

fn percentile(sorted: &[u64], percent: usize) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let index = (sorted.len() - 1).saturating_mul(percent) / 100;
    sorted.get(index).copied()
}

fn rate(numerator: usize, denominator: usize) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

pub struct OnDemandSampler<'a, S: OnDemandSampleStore> {
    store: &'a S,
}

impl<'a, S: OnDemandSampleStore> OnDemandSampler<'a, S> {
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }

    pub fn record(&self, sample: &OnDemandSample) -> Result<(), CopilotError> {
        sample
            .validate()
            .map_err(|error| CopilotError::StoreFailure {
                detail: error.to_string(),
            })?;
        self.store.append(sample)
    }

    pub fn report(&self) -> Result<OnDemandReport, CopilotError> {
        Ok(summarize(&self.store.load()?))
    }
}
