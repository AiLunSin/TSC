//! 提示采纳率与自动误触发率（SPEC 4.4-1/4-4、7.1.4）。
//!
//! 会话维度只在内存聚合；持久化事件仍遵守 SPEC 7.1.1-8 的字段白名单。

use std::collections::BTreeMap;

use meeting_copilot_domain::model::enums::TriggerKind;
use meeting_copilot_domain::model::feedback::FeedbackValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackObservation {
    pub session_id: String,
    pub platform: String,
    pub topic_cluster_id: String,
    pub trigger_kind: TriggerKind,
    pub value: FeedbackValue,
}

/// 自动提示卡的展示计数（分母来源，可来自内存计数或卡片仓储）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoShown {
    pub session_id: String,
    pub platform: String,
    pub topic_cluster_id: String,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateStat {
    Value(f64),
    /// 分母为 0，禁止用 0% 冒充。
    Insufficient,
}

impl RateStat {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Value(_) => "rate",
            Self::Insufficient => "样本不足",
        }
    }

    pub const fn value(self) -> Option<f64> {
        match self {
            Self::Value(value) => Some(value),
            Self::Insufficient => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedbackStat {
    pub session_id: String,
    pub platform: String,
    pub topic_cluster_id: String,
    pub adopted: u32,
    pub dismissed: u32,
    pub reported: u32,
    pub auto_shown: u32,
    pub adoption: RateStat,
    pub false_trigger: RateStat,
}

#[derive(Default)]
struct Tally {
    adopted: u32,
    dismissed: u32,
    reported: u32,
    auto_negative: u32,
    auto_shown: u32,
}

/// 按会话 / 平台 / 主题三维汇总。未反馈的展示只进入误触发率分母。
pub fn summarize(
    observations: &[FeedbackObservation],
    auto_shown: &[AutoShown],
) -> Vec<FeedbackStat> {
    let mut tallies: BTreeMap<(String, String, String), Tally> = BTreeMap::new();

    for observation in observations {
        let key = (
            observation.session_id.clone(),
            observation.platform.clone(),
            observation.topic_cluster_id.clone(),
        );
        let tally = tallies.entry(key).or_default();
        match observation.value {
            FeedbackValue::Adopted => tally.adopted = tally.adopted.saturating_add(1),
            FeedbackValue::Dismissed => {
                tally.dismissed = tally.dismissed.saturating_add(1);
                if observation.trigger_kind == TriggerKind::Auto {
                    tally.auto_negative = tally.auto_negative.saturating_add(1);
                }
            }
            FeedbackValue::Reported => {
                tally.reported = tally.reported.saturating_add(1);
                if observation.trigger_kind == TriggerKind::Auto {
                    tally.auto_negative = tally.auto_negative.saturating_add(1);
                }
            }
        }
    }

    for shown in auto_shown {
        let key = (
            shown.session_id.clone(),
            shown.platform.clone(),
            shown.topic_cluster_id.clone(),
        );
        let tally = tallies.entry(key).or_default();
        tally.auto_shown = tally.auto_shown.saturating_add(shown.count);
    }

    tallies
        .into_iter()
        .map(|((session_id, platform, topic_cluster_id), tally)| {
            let feedback_total = tally
                .adopted
                .saturating_add(tally.dismissed)
                .saturating_add(tally.reported);
            FeedbackStat {
                session_id,
                platform,
                topic_cluster_id,
                adopted: tally.adopted,
                dismissed: tally.dismissed,
                reported: tally.reported,
                auto_shown: tally.auto_shown,
                adoption: rate(tally.adopted, feedback_total),
                false_trigger: rate(tally.auto_negative, tally.auto_shown),
            }
        })
        .collect()
}

fn rate(numerator: u32, denominator: u32) -> RateStat {
    if denominator == 0 {
        RateStat::Insufficient
    } else {
        RateStat::Value(f64::from(numerator) / f64::from(denominator))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(value: FeedbackValue, trigger_kind: TriggerKind) -> FeedbackObservation {
        FeedbackObservation {
            session_id: "s-1".into(),
            platform: "zoom".into(),
            topic_cluster_id: "t-1".into(),
            trigger_kind,
            value,
        }
    }

    #[test]
    fn adoption_and_false_trigger_use_distinct_denominators() {
        let observations = vec![
            observation(FeedbackValue::Adopted, TriggerKind::Auto),
            observation(FeedbackValue::Dismissed, TriggerKind::Auto),
            observation(FeedbackValue::Reported, TriggerKind::Auto),
        ];
        let stats = summarize(
            &observations,
            &[AutoShown {
                session_id: "s-1".into(),
                platform: "zoom".into(),
                topic_cluster_id: "t-1".into(),
                count: 10,
            }],
        );
        let stat = stats.first().expect("stat");
        assert_eq!(stat.adoption, RateStat::Value(1.0 / 3.0));
        assert_eq!(stat.false_trigger, RateStat::Value(0.2));
        assert_eq!(stat.auto_shown, 10);
    }

    #[test]
    fn zero_denominator_is_insufficient_not_zero() {
        let stats = summarize(
            &[observation(FeedbackValue::Dismissed, TriggerKind::OnDemand)],
            &[],
        );
        let stat = stats.first().expect("stat");
        assert_eq!(stat.adoption, RateStat::Value(0.0));
        assert_eq!(stat.false_trigger, RateStat::Insufficient);
        assert_eq!(stat.false_trigger.label(), "样本不足");
        assert_eq!(stat.false_trigger.value(), None);
    }
}
