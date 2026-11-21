//! 理解层策略：13 字段、默认值与固定顺序边界校验。

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 理解层策略。默认值见 spec 6.4。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InsightPolicy {
    pub insight_confidence_min_milli: u16,
    pub silence_min_ms: u64,
    pub silence_overlap_ratio_min_milli: u16,
    pub split_char_limit: u16,
    pub summary_interval_ms: u64,
    pub summary_utterance_count: u16,
    pub summary_timeout_ms: u64,
    pub summary_retained_versions: u8,
    pub kind_fallback_penalty_milli: u16,
    pub similarity_min_milli: u16,
    pub dedupe_window_ms: u64,
    pub short_text_merge_min_chars: u16,
    pub prefilter_enabled: bool,
}

impl Default for InsightPolicy {
    fn default() -> Self {
        Self {
            insight_confidence_min_milli: 500,
            silence_min_ms: 5_000,
            silence_overlap_ratio_min_milli: 800,
            split_char_limit: 2_000,
            summary_interval_ms: 30_000,
            summary_utterance_count: 20,
            summary_timeout_ms: 15_000,
            summary_retained_versions: 2,
            kind_fallback_penalty_milli: 150,
            similarity_min_milli: 880,
            dedupe_window_ms: 600_000,
            short_text_merge_min_chars: 8,
            prefilter_enabled: true,
        }
    }
}

/// 策略违规项；不并入 `ErrorCode` 冻结集。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum InsightPolicyError {
    #[error("字段 {field} 越界：{value} 不在 [{min},{max}]")]
    OutOfRange {
        field: &'static str,
        value: i64,
        min: i64,
        max: i64,
    },
    #[error("可提取置信度下限 {value} 低于推测性边界 {min}")]
    ConfidenceBelowSpeculativeBound { value: u16, min: u16 },
    #[error("切分上限 {value} 超过正文上限 {max}")]
    SplitExceedsTextLimit { value: u16, max: u16 },
    #[error("摘要保留版本固定为 2，实际 {value}")]
    RetainedVersionsFixed { value: u8 },
}

fn as_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn check_range(
    field: &'static str,
    value: i64,
    min: i64,
    max: i64,
) -> Result<(), InsightPolicyError> {
    if value < min || value > max {
        Err(InsightPolicyError::OutOfRange {
            field,
            value,
            min,
            max,
        })
    } else {
        Ok(())
    }
}

impl InsightPolicy {
    /// 校验顺序固定：字段 1 → 13，返回首个违规。
    pub fn validate(&self) -> Result<(), InsightPolicyError> {
        if self.insight_confidence_min_milli < 350 {
            return Err(InsightPolicyError::ConfidenceBelowSpeculativeBound {
                value: self.insight_confidence_min_milli,
                min: 350,
            });
        }
        check_range(
            "insight_confidence_min_milli",
            i64::from(self.insight_confidence_min_milli),
            350,
            1000,
        )?;
        check_range("silence_min_ms", as_i64(self.silence_min_ms), 500, 60_000)?;
        check_range(
            "silence_overlap_ratio_min_milli",
            i64::from(self.silence_overlap_ratio_min_milli),
            0,
            1000,
        )?;
        if self.split_char_limit > 2_000 {
            return Err(InsightPolicyError::SplitExceedsTextLimit {
                value: self.split_char_limit,
                max: 2_000,
            });
        }
        check_range(
            "split_char_limit",
            i64::from(self.split_char_limit),
            100,
            2_000,
        )?;
        check_range(
            "summary_interval_ms",
            as_i64(self.summary_interval_ms),
            5_000,
            300_000,
        )?;
        check_range(
            "summary_utterance_count",
            i64::from(self.summary_utterance_count),
            1,
            200,
        )?;
        check_range(
            "summary_timeout_ms",
            as_i64(self.summary_timeout_ms),
            1_000,
            60_000,
        )?;
        if self.summary_retained_versions != 2 {
            return Err(InsightPolicyError::RetainedVersionsFixed {
                value: self.summary_retained_versions,
            });
        }
        check_range(
            "kind_fallback_penalty_milli",
            i64::from(self.kind_fallback_penalty_milli),
            0,
            1000,
        )?;
        check_range(
            "similarity_min_milli",
            i64::from(self.similarity_min_milli),
            0,
            1000,
        )?;
        check_range(
            "dedupe_window_ms",
            as_i64(self.dedupe_window_ms),
            60_000,
            3_600_000,
        )?;
        check_range(
            "short_text_merge_min_chars",
            i64::from(self.short_text_merge_min_chars),
            0,
            200,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]

    use super::*;

    #[test]
    fn insight_policy_defaults_match_spec() {
        let policy = InsightPolicy::default();
        assert_eq!(policy.insight_confidence_min_milli, 500);
        assert_eq!(policy.silence_min_ms, 5_000);
        assert_eq!(policy.silence_overlap_ratio_min_milli, 800);
        assert_eq!(policy.split_char_limit, 2_000);
        assert_eq!(policy.summary_interval_ms, 30_000);
        assert_eq!(policy.summary_utterance_count, 20);
        assert_eq!(policy.summary_timeout_ms, 15_000);
        assert_eq!(policy.summary_retained_versions, 2);
        assert_eq!(policy.kind_fallback_penalty_milli, 150);
        assert_eq!(policy.similarity_min_milli, 880);
        assert_eq!(policy.dedupe_window_ms, 600_000);
        assert_eq!(policy.short_text_merge_min_chars, 8);
        assert!(policy.prefilter_enabled);
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn insight_policy_confidence_lower_bound() {
        let mut policy = InsightPolicy::default();
        policy.insight_confidence_min_milli = 349;
        assert!(matches!(
            policy.validate(),
            Err(InsightPolicyError::ConfidenceBelowSpeculativeBound { .. })
        ));
    }

    #[test]
    fn insight_policy_retained_versions_fixed() {
        let mut policy = InsightPolicy::default();
        policy.summary_retained_versions = 3;
        assert!(matches!(
            policy.validate(),
            Err(InsightPolicyError::RetainedVersionsFixed { .. })
        ));
    }

    #[test]
    fn insight_policy_split_exceeds_text_limit() {
        let mut policy = InsightPolicy::default();
        policy.split_char_limit = 2_001;
        assert!(matches!(
            policy.validate(),
            Err(InsightPolicyError::SplitExceedsTextLimit { .. })
        ));
    }

    #[test]
    fn insight_policy_first_violation_wins() {
        let mut policy = InsightPolicy::default();
        policy.silence_min_ms = 499;
        policy.split_char_limit = 2_001;
        assert!(matches!(
            policy.validate(),
            Err(InsightPolicyError::OutOfRange {
                field: "silence_min_ms",
                ..
            })
        ));
    }

    #[test]
    fn insight_policy_rejects_unknown_field() {
        let json = r#"{"insight_confidence_min_milli":500,"unknown_field":1}"#;
        assert!(serde_json::from_str::<InsightPolicy>(json).is_err());
    }

    #[test]
    fn insight_policy_round_trips() {
        let policy = InsightPolicy::default();
        let json = serde_json::to_string(&policy).expect("serialize");
        let decoded: InsightPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(policy, decoded);
    }
}
