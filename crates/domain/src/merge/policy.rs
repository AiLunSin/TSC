//! 合并策略：10 个字段、默认值与固定顺序的边界校验。

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 双通道去重合并策略。默认值见 design 2.3.3。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MergePolicy {
    pub merge_enabled: bool,
    pub pair_window_ms: u64,
    pub pair_window_estimated_ms: u64,
    pub simhash_max_distance: u8,
    pub length_ratio_min_milli: u16,
    pub length_ratio_max_milli: u16,
    pub cosine_min_milli: u16,
    pub cosine_min_chars: u16,
    pub confidence_cap_milli: u16,
    pub time_bucket_ms: u64,
}

impl Default for MergePolicy {
    fn default() -> Self {
        Self {
            merge_enabled: true,
            pair_window_ms: 2500,
            pair_window_estimated_ms: 3500,
            simhash_max_distance: 3,
            length_ratio_min_milli: 600,
            length_ratio_max_milli: 1600,
            cosine_min_milli: 860,
            cosine_min_chars: 8,
            confidence_cap_milli: 950,
            time_bucket_ms: 2500,
        }
    }
}

/// 策略违规项；不并入 `ErrorCode`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MergePolicyError {
    #[error("字段 {field} 越界：{value} 不在 [{min},{max}]")]
    OutOfRange {
        field: &'static str,
        value: i64,
        min: i64,
        max: i64,
    },
    #[error("估算窗口 {estimated_ms} 小于基础窗口 {base_ms}")]
    WindowInverted { estimated_ms: u64, base_ms: u64 },
    #[error("长度比下限 {min_milli} 不小于上限 {max_milli}")]
    RatioInverted { min_milli: u16, max_milli: u16 },
    #[error("时间桶 {bucket_ms} 小于配对窗口 {window_ms}")]
    BucketBelowWindow { bucket_ms: u64, window_ms: u64 },
}

fn as_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn check_range(
    field: &'static str,
    value: i64,
    min: i64,
    max: i64,
) -> Result<(), MergePolicyError> {
    if value < min || value > max {
        Err(MergePolicyError::OutOfRange {
            field,
            value,
            min,
            max,
        })
    } else {
        Ok(())
    }
}

impl MergePolicy {
    /// 校验顺序固定：字段2 → 字段3（区间→倒置）→ 字段4 → 字段5 →
    /// 字段6（区间→倒置）→ 字段7 → 字段8 → 字段9 → 字段10（区间→桶下界）。
    pub fn validate(&self) -> Result<(), MergePolicyError> {
        check_range("pair_window_ms", as_i64(self.pair_window_ms), 500, 5000)?;
        check_range(
            "pair_window_estimated_ms",
            as_i64(self.pair_window_estimated_ms),
            500,
            5000,
        )?;
        if self.pair_window_estimated_ms < self.pair_window_ms {
            return Err(MergePolicyError::WindowInverted {
                estimated_ms: self.pair_window_estimated_ms,
                base_ms: self.pair_window_ms,
            });
        }
        check_range(
            "simhash_max_distance",
            i64::from(self.simhash_max_distance),
            0,
            8,
        )?;
        check_range(
            "length_ratio_min_milli",
            i64::from(self.length_ratio_min_milli),
            100,
            1000,
        )?;
        check_range(
            "length_ratio_max_milli",
            i64::from(self.length_ratio_max_milli),
            101,
            3000,
        )?;
        if self.length_ratio_max_milli <= self.length_ratio_min_milli {
            return Err(MergePolicyError::RatioInverted {
                min_milli: self.length_ratio_min_milli,
                max_milli: self.length_ratio_max_milli,
            });
        }
        check_range(
            "cosine_min_milli",
            i64::from(self.cosine_min_milli),
            0,
            1000,
        )?;
        check_range("cosine_min_chars", i64::from(self.cosine_min_chars), 0, 200)?;
        check_range(
            "confidence_cap_milli",
            i64::from(self.confidence_cap_milli),
            0,
            1000,
        )?;
        check_range("time_bucket_ms", as_i64(self.time_bucket_ms), 500, 5000)?;
        if self.time_bucket_ms < self.pair_window_ms {
            return Err(MergePolicyError::BucketBelowWindow {
                bucket_ms: self.time_bucket_ms,
                window_ms: self.pair_window_ms,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]

    use super::*;

    #[test]
    fn defaults_match_design_2_3_3() {
        let policy = MergePolicy::default();
        assert!(policy.merge_enabled);
        assert_eq!(policy.pair_window_ms, 2_500);
        assert_eq!(policy.pair_window_estimated_ms, 3_500);
        assert_eq!(policy.simhash_max_distance, 3);
        assert_eq!(policy.length_ratio_min_milli, 600);
        assert_eq!(policy.length_ratio_max_milli, 1_600);
        assert_eq!(policy.cosine_min_milli, 860);
        assert_eq!(policy.cosine_min_chars, 8);
        assert_eq!(policy.confidence_cap_milli, 950);
        assert_eq!(policy.time_bucket_ms, 2_500);
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn merge_policy_defaults_match_design() {
        assert!(MergePolicy::default().validate().is_ok());
    }

    #[test]
    fn merge_policy_rejects_out_of_range() {
        let mut policy = MergePolicy::default();
        policy.pair_window_ms = 499;
        assert!(matches!(
            policy.validate(),
            Err(MergePolicyError::OutOfRange {
                field: "pair_window_ms",
                ..
            })
        ));
        policy.pair_window_ms = 5_001;
        assert!(policy.validate().is_err());
        policy.pair_window_ms = 2_500;
        policy.pair_window_estimated_ms = 2_499;
        assert!(matches!(
            policy.validate(),
            Err(MergePolicyError::WindowInverted { .. })
        ));
        policy.pair_window_estimated_ms = 3_500;
        policy.simhash_max_distance = 9;
        assert!(policy.validate().is_err());
        policy.simhash_max_distance = 3;
        policy.length_ratio_min_milli = 99;
        assert!(policy.validate().is_err());
        policy.length_ratio_min_milli = 600;
        policy.length_ratio_max_milli = 600;
        assert!(matches!(
            policy.validate(),
            Err(MergePolicyError::RatioInverted { .. })
        ));
        policy.length_ratio_max_milli = 1_600;
        policy.cosine_min_milli = 1_001;
        assert!(policy.validate().is_err());
        policy.cosine_min_milli = 860;
        policy.cosine_min_chars = 201;
        assert!(policy.validate().is_err());
        policy.cosine_min_chars = 8;
        policy.confidence_cap_milli = 1_001;
        assert!(policy.validate().is_err());
        policy.confidence_cap_milli = 950;
        policy.time_bucket_ms = 2_499;
        assert!(matches!(
            policy.validate(),
            Err(MergePolicyError::BucketBelowWindow { .. })
        ));
    }
}
