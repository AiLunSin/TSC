//! 分类兜底惩罚（整数毫值，无浮点误差）与字段补全。

use meeting_copilot_domain::model::insight::{InsightDraft, InsightFields, QuestionFields};
use meeting_copilot_domain::model::insight_policy::InsightPolicy;
use meeting_copilot_domain::vo::Confidence;

/// `max(0, 原值 - penalty)`，全程毫值整数。
pub fn apply_kind_penalty(
    confidence: Confidence,
    policy: &InsightPolicy,
    fallback: bool,
) -> Confidence {
    if !fallback {
        return confidence;
    }
    let original = (confidence.value() * 1000.0).round() as i64;
    let reduced = (original - i64::from(policy.kind_fallback_penalty_milli)).max(0);
    let milli = u16::try_from(reduced).unwrap_or(1000);
    Confidence::new(f64::from(milli) / 1000.0).unwrap_or(Confidence::ZERO)
}

/// 疑问缺字段时初始 `unanswered=true`；非疑问类保持 `None`。
pub fn fill_fields(draft: &mut InsightDraft) {
    if matches!(
        draft.kind,
        meeting_copilot_domain::model::enums::InsightKind::Question
    ) {
        let mut fields = draft.fields.clone();
        if fields.question.is_none() {
            fields.question = Some(QuestionFields { unanswered: true });
        }
        draft.fields = fields;
    } else if draft.fields.question.is_some() {
        let mut fields: InsightFields = draft.fields.clone();
        fields.question = None;
        draft.fields = fields;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]

    use super::*;

    #[test]
    fn kind_fallback_penalty_is_recorded() {
        let policy = InsightPolicy::default();
        let base = Confidence::new(0.80).expect("conf");
        let penalized = apply_kind_penalty(base, &policy, true);
        assert!((penalized.value() - 0.65).abs() < 0.001);
        assert!((apply_kind_penalty(base, &policy, false).value() - 0.80).abs() < 0.001);
    }

    #[test]
    fn kind_fallback_penalty_is_configurable() {
        let mut policy = InsightPolicy::default();
        policy.kind_fallback_penalty_milli = 100;
        let base = Confidence::new(0.80).expect("conf");
        assert!((apply_kind_penalty(base, &policy, true).value() - 0.70).abs() < 0.001);
        let low = Confidence::new(0.10).expect("conf");
        assert_eq!(apply_kind_penalty(low, &policy, true), Confidence::ZERO);
    }
}
