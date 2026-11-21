//! 理解层门面：唯一入 `observe`，串行、无 Result、不阻塞。

use std::sync::Mutex;

use meeting_copilot_domain::model::insight::Insight;
use meeting_copilot_domain::model::insight_policy::InsightPolicy;
use meeting_copilot_domain::model::rolling_summary::SummaryVersionPair;
use meeting_copilot_domain::vo::SessionId;

use super::counters::{Counter, CounterSnapshot, UnderstandingCounters};
use super::normalize;
use super::ports::UnderstandFeature;
use super::types::{UnderstandInitError, UnderstandInput, UnderstandOutcome};

/// 理解层门面。默认不由 `CaptureChannelService` 持有。
pub struct UnderstandService {
    policy: InsightPolicy,
    feature: UnderstandFeature,
    counters: UnderstandingCounters,
    seen: Mutex<Vec<String>>,
}

impl UnderstandService {
    pub fn new(
        policy: InsightPolicy,
        feature: UnderstandFeature,
    ) -> Result<Self, UnderstandInitError> {
        policy.validate().map_err(UnderstandInitError::Policy)?;
        Ok(Self {
            policy,
            feature,
            counters: UnderstandingCounters::new(),
            seen: Mutex::new(Vec::new()),
        })
    }

    /// 关闭时零计数、零对象、零网关调用。
    pub fn observe(&self, input: &UnderstandInput) -> UnderstandOutcome {
        if matches!(self.feature, UnderstandFeature::Disabled) {
            return UnderstandOutcome::Disabled;
        }
        let reference = unit_reference(&input.unit);
        {
            let mut seen = self
                .seen
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if seen.contains(&reference) {
                self.counters.bump(Counter::DuplicateDeliveryIgnored);
                return UnderstandOutcome::Duplicate;
            }
            seen.push(reference);
        }
        normalize::admit(input, &self.policy, &self.counters)
    }

    pub fn counters(&self) -> CounterSnapshot {
        self.counters.snapshot()
    }

    pub fn insights(&self, _session: &SessionId) -> Vec<Insight> {
        Vec::new()
    }

    pub fn summary_view(&self, _session: &SessionId) -> Option<SummaryVersionPair> {
        None
    }
}

fn unit_reference(unit: &crate::capture::merge_types::MergedUnit) -> String {
    match unit {
        crate::capture::merge_types::MergedUnit::Single(single) => {
            single.utterance.reference.as_str().to_owned()
        }
        crate::capture::merge_types::MergedUnit::Merged(merged) => {
            merged.utterance.reference.as_str().to_owned()
        }
    }
}
