//! 按需补全的结果裁决（SPEC 7.3.3-3/4、DESIGN 2.10.3）。
//!
//! 只做纯判定：不发起网络、不重试、不产生文案结论。部分结果只标记
//! “结果可能不完整”，明确不自动补发第二次；取消优先于任何成功结果。

use meeting_copilot_domain::model::proactive::OnDemandOutcome;
use meeting_copilot_domain::vo::ProviderId;

/// 单个检索源的一次尝试。只含标识、结果与引用条数，不含查询或引用正文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAttempt {
    pub provider_id: ProviderId,
    pub outcome: OnDemandOutcome,
    pub reference_count: usize,
}

impl ProviderAttempt {
    pub fn new(provider_id: ProviderId, outcome: OnDemandOutcome, reference_count: usize) -> Self {
        Self {
            provider_id,
            outcome,
            reference_count,
        }
    }

    /// 只有成功且带回至少一条引用才算可用来源。
    pub fn is_usable(&self) -> bool {
        self.outcome == OnDemandOutcome::Success && self.reference_count > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnDemandDecision {
    /// 没有任何已启用的检索源。
    NoSource,
    /// 使用者取消：在途请求终止，且不计入成功样本。
    Canceled,
    /// 部分来源可用；展示已有来源并标注“结果可能不完整”。
    Partial { sources: usize },
    /// 全部来源可用。
    Complete { sources: usize },
    /// 有来源但全部失败或超时。
    AllFailed,
}

impl OnDemandDecision {
    pub const fn sources(self) -> usize {
        match self {
            Self::Partial { sources } | Self::Complete { sources } => sources,
            _ => 0,
        }
    }

    pub const fn is_partial(self) -> bool {
        matches!(self, Self::Partial { .. })
    }

    /// 需要随卡片展示的提示；仅部分结果有提示。
    pub const fn notice(self) -> Option<&'static str> {
        match self {
            Self::Partial { .. } => Some("结果可能不完整"),
            _ => None,
        }
    }
}

/// `canceled` 为真时立即终止，无论已返回多少来源。
pub fn decide(canceled: bool, attempts: &[ProviderAttempt]) -> OnDemandDecision {
    if canceled {
        return OnDemandDecision::Canceled;
    }
    if attempts.is_empty() {
        return OnDemandDecision::NoSource;
    }
    let usable = attempts
        .iter()
        .filter(|attempt| attempt.is_usable())
        .count();
    if usable == 0 {
        return OnDemandDecision::AllFailed;
    }
    if usable < attempts.len() {
        OnDemandDecision::Partial { sources: usable }
    } else {
        OnDemandDecision::Complete { sources: usable }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(outcome: OnDemandOutcome, count: usize) -> ProviderAttempt {
        ProviderAttempt::new(
            ProviderId::new("provider-1").expect("provider"),
            outcome,
            count,
        )
    }

    #[test]
    fn no_source_and_all_failed_are_distinguished() {
        assert_eq!(decide(false, &[]), OnDemandDecision::NoSource);
        assert_eq!(
            decide(false, &[attempt(OnDemandOutcome::Timeout, 0)]),
            OnDemandDecision::AllFailed
        );
    }

    #[test]
    fn partial_results_carry_notice_and_sources() {
        let decision = decide(
            false,
            &[
                attempt(OnDemandOutcome::Success, 2),
                attempt(OnDemandOutcome::Timeout, 0),
            ],
        );
        assert_eq!(decision, OnDemandDecision::Partial { sources: 1 });
        assert!(decision.is_partial());
        assert_eq!(decision.notice(), Some("结果可能不完整"));
    }

    #[test]
    fn cancel_wins_even_with_successful_sources() {
        let decision = decide(true, &[attempt(OnDemandOutcome::Success, 3)]);
        assert_eq!(decision, OnDemandDecision::Canceled);
        assert_eq!(decision.notice(), None);
        assert_eq!(decision.sources(), 0);
    }

    #[test]
    fn zero_reference_success_is_not_a_usable_source() {
        assert_eq!(
            decide(false, &[attempt(OnDemandOutcome::Success, 0)]),
            OnDemandDecision::AllFailed
        );
    }
}
