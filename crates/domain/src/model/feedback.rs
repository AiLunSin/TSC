//! 提示卡负反馈与主题权重（SPEC 7.1、DESIGN 2.10.1）。
//!
//! 本模块只做纯计算：事件、权重与排序分数。持久化由 `port::proactive`
//! 的仓储端口承担，调用方不得在此写入任何发言原文或卡片结论。

use serde::{Deserialize, Serialize};

use crate::model::card::DedupeKey;
use crate::model::enums::{FeedbackStatus, Platform, TriggerKind};
use crate::vo::{CardRef, ReferenceRef};

/// 权重下限。低于该值不再继续惩罚。
pub const WEIGHT_MIN_MILLI: i32 = 100;
/// 权重上限，也是新主题的初始权重。
pub const WEIGHT_MAX_MILLI: i32 = 1_000;
/// 新主题初始权重。
pub const WEIGHT_INITIAL_MILLI: i32 = WEIGHT_MAX_MILLI;
/// 每次"无用"的扣减。
pub const DISMISS_PENALTY_MILLI: i32 = 250;
/// 每次"有用"的回补。
pub const ADOPT_BONUS_MILLI: i32 = 100;
/// 72 小时无新增负反馈后的每日回补。
pub const RECOVERY_BONUS_MILLI: i32 = 100;
/// 连续无负反馈的静默窗口。
pub const NEGATIVE_QUIET_MS: i64 = 72 * 60 * 60 * 1_000;
/// 回补周期（每日一次）。
pub const RECOVERY_PERIOD_MS: i64 = 24 * 60 * 60 * 1_000;
/// "举报错误"触发的同主题同来源硬抑制时长。
pub const SUPPRESSION_MS: i64 = 7 * 24 * 60 * 60 * 1_000;

/// 反馈取值。`None` 不是一次反馈，因此不在此枚举内。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackValue {
    Adopted,
    Dismissed,
    Reported,
}

impl FeedbackValue {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Adopted => "adopted",
            Self::Dismissed => "dismissed",
            Self::Reported => "reported",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "adopted" => Some(Self::Adopted),
            "dismissed" => Some(Self::Dismissed),
            "reported" => Some(Self::Reported),
            _ => None,
        }
    }

    pub const fn status(self) -> FeedbackStatus {
        match self {
            Self::Adopted => FeedbackStatus::Adopted,
            Self::Dismissed => FeedbackStatus::Dismissed,
            Self::Reported => FeedbackStatus::Reported,
        }
    }

    pub const fn from_status(status: FeedbackStatus) -> Option<Self> {
        match status {
            FeedbackStatus::None => None,
            FeedbackStatus::Adopted => Some(Self::Adopted),
            FeedbackStatus::Dismissed => Some(Self::Dismissed),
            FeedbackStatus::Reported => Some(Self::Reported),
        }
    }

    /// 是否属于负反馈（用于 72 小时回补窗口）。
    pub const fn is_negative(self) -> bool {
        !matches!(self, Self::Adopted)
    }
}

fn clamp_weight(weight_milli: i32) -> i32 {
    weight_milli.clamp(WEIGHT_MIN_MILLI, WEIGHT_MAX_MILLI)
}

/// 单次反馈后的权重。`Reported` 只触发抑制，不改变权重。
pub fn apply_weight(current_milli: i32, value: FeedbackValue) -> i32 {
    let current = clamp_weight(current_milli);
    match value {
        FeedbackValue::Adopted => (current + ADOPT_BONUS_MILLI).min(WEIGHT_MAX_MILLI),
        FeedbackValue::Dismissed => (current - DISMISS_PENALTY_MILLI).max(WEIGHT_MIN_MILLI),
        FeedbackValue::Reported => current,
    }
}

/// 72 小时无负反馈后的每日回补。返回不高于上限的权重。
pub fn recover_weight(current_milli: i32, last_negative_at_ms: Option<i64>, now_ms: i64) -> i32 {
    let current = clamp_weight(current_milli);
    let Some(last_negative) = last_negative_at_ms else {
        return current;
    };
    let elapsed = now_ms.saturating_sub(last_negative);
    if elapsed < NEGATIVE_QUIET_MS {
        return current;
    }
    let periods = (elapsed - NEGATIVE_QUIET_MS) / RECOVERY_PERIOD_MS + 1;
    let steps = i32::try_from(periods).unwrap_or(i32::MAX);
    current
        .saturating_add(steps.saturating_mul(RECOVERY_BONUS_MILLI))
        .min(WEIGHT_MAX_MILLI)
}

/// 排序分 = 基础相关度 × 权重 / 1000。只在门控 Allow 之后使用。
pub fn rank_score_milli(base_relevance_milli: i32, weight_milli: i32) -> i32 {
    let base = base_relevance_milli.max(0);
    let weight = clamp_weight(weight_milli);
    base.saturating_mul(weight) / WEIGHT_MAX_MILLI
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeedbackError {
    #[error("反馈字段非法：{0}")]
    InvalidField(&'static str),
}

/// 一次反馈事件。字段白名单见 SPEC 7.1.1-8，禁止扩展内容字段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackEvent {
    pub feedback_id: String,
    pub card_id: CardRef,
    pub dedupe_key: DedupeKey,
    pub topic_cluster_id: String,
    pub source_ref_id: Option<ReferenceRef>,
    pub trigger_kind: TriggerKind,
    pub platform: Platform,
    pub value: FeedbackValue,
    pub occurred_at_ms: i64,
    pub weight_after_milli: i32,
}

impl FeedbackEvent {
    pub fn validate(&self) -> Result<(), FeedbackError> {
        if self.feedback_id.trim().is_empty() || self.feedback_id.chars().count() > 128 {
            return Err(FeedbackError::InvalidField("feedback_id"));
        }
        if self.topic_cluster_id.trim().is_empty() {
            return Err(FeedbackError::InvalidField("topic_cluster_id"));
        }
        if self.occurred_at_ms < 0 {
            return Err(FeedbackError::InvalidField("occurred_at_ms"));
        }
        if !(WEIGHT_MIN_MILLI..=WEIGHT_MAX_MILLI).contains(&self.weight_after_milli) {
            return Err(FeedbackError::InvalidField("weight_after_milli"));
        }
        Ok(())
    }

    pub fn source_ref(&self) -> &str {
        self.source_ref_id.as_ref().map_or("", ReferenceRef::as_str)
    }
}

/// 同一主题簇 + 同一来源的派生权重。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicWeight {
    pub topic_cluster_id: String,
    pub source_ref_id: String,
    pub weight_milli: i32,
    pub blocked_until_ms: Option<i64>,
    /// 最近一次负反馈时间；`adopted` 不刷新，保证 72 小时窗口语义。
    pub last_negative_at_ms: Option<i64>,
    pub updated_at_ms: i64,
}

impl TopicWeight {
    pub fn initial(
        topic_cluster_id: impl Into<String>,
        source_ref_id: impl Into<String>,
        now_ms: i64,
    ) -> Self {
        Self {
            topic_cluster_id: topic_cluster_id.into(),
            source_ref_id: source_ref_id.into(),
            weight_milli: WEIGHT_INITIAL_MILLI,
            blocked_until_ms: None,
            last_negative_at_ms: None,
            updated_at_ms: now_ms,
        }
    }

    /// 应用一次反馈，返回新权重（不修改自身）。
    pub fn apply(&self, value: FeedbackValue, now_ms: i64) -> Self {
        let weight_milli = apply_weight(self.weight_milli, value);
        let blocked_until_ms = if value == FeedbackValue::Reported {
            let until = now_ms.saturating_add(SUPPRESSION_MS);
            Some(
                self.blocked_until_ms
                    .map_or(until, |existing| existing.max(until)),
            )
        } else {
            self.blocked_until_ms
        };
        let last_negative_at_ms = if value.is_negative() {
            Some(now_ms)
        } else {
            self.last_negative_at_ms
        };
        Self {
            weight_milli,
            blocked_until_ms,
            last_negative_at_ms,
            updated_at_ms: now_ms,
            ..self.clone()
        }
    }

    /// 按 72 小时规则回补权重。抑制状态不因回补解除。
    pub fn recovered(&self, now_ms: i64) -> Self {
        let weight_milli = recover_weight(self.weight_milli, self.last_negative_at_ms, now_ms);
        Self {
            weight_milli,
            ..self.clone()
        }
    }

    pub fn is_suppressed(&self, now_ms: i64) -> bool {
        self.blocked_until_ms.is_some_and(|until| until > now_ms)
    }

    /// 仅由使用者显式操作解除硬抑制（SPEC 7.1.1-8c）。
    pub fn clear_suppression(&mut self) {
        self.blocked_until_ms = None;
    }

    pub fn rank_score(&self, base_relevance_milli: i32) -> i32 {
        rank_score_milli(base_relevance_milli, self.weight_milli)
    }

    pub fn validate(&self) -> Result<(), FeedbackError> {
        if self.topic_cluster_id.trim().is_empty() {
            return Err(FeedbackError::InvalidField("topic_cluster_id"));
        }
        if !(WEIGHT_MIN_MILLI..=WEIGHT_MAX_MILLI).contains(&self.weight_milli) {
            return Err(FeedbackError::InvalidField("weight_milli"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_dismissals_drop_to_half() {
        let mut weight = TopicWeight::initial("topic-1", "ref-1", 0);
        weight = weight.apply(FeedbackValue::Dismissed, 10);
        assert_eq!(weight.weight_milli, 750);
        weight = weight.apply(FeedbackValue::Dismissed, 20);
        assert_eq!(weight.weight_milli, 500);
    }

    #[test]
    fn dismissals_never_below_floor() {
        let mut weight = TopicWeight::initial("topic-1", "ref-1", 0);
        for step in 0..6 {
            weight = weight.apply(FeedbackValue::Dismissed, step);
        }
        assert_eq!(weight.weight_milli, WEIGHT_MIN_MILLI);
        assert_eq!(
            apply_weight(120, FeedbackValue::Dismissed),
            WEIGHT_MIN_MILLI
        );
    }

    #[test]
    fn adoption_is_capped() {
        assert_eq!(
            apply_weight(1_000, FeedbackValue::Adopted),
            WEIGHT_MAX_MILLI
        );
        assert_eq!(apply_weight(950, FeedbackValue::Adopted), 1_000);
    }

    #[test]
    fn reported_keeps_weight_and_suppresses_for_seven_days() {
        let weight =
            TopicWeight::initial("topic-1", "ref-1", 1_000).apply(FeedbackValue::Reported, 1_000);
        assert_eq!(weight.weight_milli, 1_000);
        assert_eq!(weight.blocked_until_ms, Some(1_000 + SUPPRESSION_MS));
        assert!(weight.is_suppressed(1_000 + SUPPRESSION_MS - 1));
        assert!(!weight.is_suppressed(1_000 + SUPPRESSION_MS));
    }

    #[test]
    fn recovery_follows_quiet_window() {
        let weight = TopicWeight {
            last_negative_at_ms: Some(0),
            weight_milli: 500,
            ..TopicWeight::initial("topic-1", "ref-1", 0)
        };
        assert_eq!(weight.recovered(NEGATIVE_QUIET_MS - 1).weight_milli, 500);
        assert_eq!(weight.recovered(NEGATIVE_QUIET_MS).weight_milli, 600);
        assert_eq!(
            weight
                .recovered(NEGATIVE_QUIET_MS + RECOVERY_PERIOD_MS)
                .weight_milli,
            700
        );
        assert_eq!(weight.recovered(i64::MAX).weight_milli, WEIGHT_MAX_MILLI);
    }

    #[test]
    fn recovery_does_not_clear_suppression() {
        let weight = TopicWeight {
            last_negative_at_ms: Some(0),
            blocked_until_ms: Some(SUPPRESSION_MS),
            weight_milli: 400,
            ..TopicWeight::initial("topic-1", "ref-1", 0)
        };
        let recovered = weight.recovered(SUPPRESSION_MS - 1);
        assert!(recovered.is_suppressed(SUPPRESSION_MS - 1));
    }

    #[test]
    fn ranking_scales_by_weight() {
        assert_eq!(rank_score_milli(800, 1_000), 800);
        assert_eq!(rank_score_milli(800, 500), 400);
        assert_eq!(rank_score_milli(-1, 500), 0);
        assert_eq!(rank_score_milli(800, 0), 80);
    }

    #[test]
    fn event_validation_rejects_out_of_range_weight() {
        let event = FeedbackEvent {
            feedback_id: "fb-1".to_owned(),
            card_id: CardRef::new("card-1").expect("card"),
            dedupe_key: DedupeKey::new("a".repeat(32)).expect("key"),
            topic_cluster_id: "topic-1".to_owned(),
            source_ref_id: None,
            trigger_kind: TriggerKind::Auto,
            platform: Platform::Zoom,
            value: FeedbackValue::Dismissed,
            occurred_at_ms: 1,
            weight_after_milli: 99,
        };
        assert!(event.validate().is_err());
        assert_eq!(event.source_ref(), "");
    }
}
