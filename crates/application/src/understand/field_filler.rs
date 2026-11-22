//! 分类兜底惩罚（整数毫值）与字段补全。

use meeting_copilot_domain::model::enums::InsightKind;
use meeting_copilot_domain::model::insight::{
    DecisionFields, InsightDraft, InsightFields, QuestionFields, TodoFields,
};
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

/// 按 kind 补齐本类字段并清空异类字段。
///
/// - `Decision`：缺 `decision` 时补空壳；`owner` 为 `None` 即“待补”。
/// - `Todo`：缺 `action` 时用正文回填；`assignee` 为 `None` 即“待补”。
/// - `Question`：`unanswered=true`。
/// - `KeyInfo`：不保留任何异类字段。
pub fn fill_fields(draft: &mut InsightDraft) {
    let mut fields = draft.fields.clone();
    match draft.kind {
        InsightKind::Decision => {
            if fields.decision.is_none() {
                fields.decision = Some(DecisionFields {
                    owner: None,
                    deadline: None,
                });
            }
            fields.todo = None;
            fields.question = None;
        }
        InsightKind::Todo => {
            let mut todo = fields.todo.take().unwrap_or_default();
            if todo.action.is_none() {
                todo.action = Some(draft.text.clone());
            }
            fields.todo = Some(todo);
            fields.decision = None;
            fields.question = None;
        }
        InsightKind::Question => {
            fields.question = Some(QuestionFields { unanswered: true });
            fields.decision = None;
            fields.todo = None;
        }
        InsightKind::KeyInfo => {
            fields.decision = None;
            fields.todo = None;
            fields.question = None;
        }
    }
    draft.fields = fields;
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]

    use super::*;
    use meeting_copilot_domain::model::insight::InsightSources;
    use meeting_copilot_domain::vo::{ShortText, UtteranceRef};

    fn draft(kind: InsightKind, text: &str) -> InsightDraft {
        InsightDraft {
            kind,
            text: ShortText::new(text).expect("text"),
            confidence: Confidence::new(0.9).expect("conf"),
            sources: InsightSources::new(vec![UtteranceRef::new("u1").expect("ref")])
                .expect("sources"),
            fields: InsightFields::default(),
            kind_fallback: false,
        }
    }

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

    #[test]
    fn decision_missing_owner_stays_pending() {
        let mut item = draft(InsightKind::Decision, "确认下周灰度上线");
        fill_fields(&mut item);
        let decision = item.fields.decision.expect("decision");
        assert!(decision.owner.is_none());
        assert!(item.fields.todo.is_none());
        assert!(item.fields.question.is_none());
    }

    #[test]
    fn todo_missing_assignee_stays_pending_with_action() {
        let mut item = draft(InsightKind::Todo, "我来跟进回滚方案");
        fill_fields(&mut item);
        let todo = item.fields.todo.expect("todo");
        assert!(todo.assignee.is_none());
        assert_eq!(
            todo.action.as_ref().map(|action| action.as_str()),
            Some("我来跟进回滚方案")
        );
        assert!(item.fields.decision.is_none());
    }

    #[test]
    fn question_is_unanswered_and_foreign_fields_cleared() {
        let mut item = draft(InsightKind::Question, "是否下周上线？");
        item.fields.decision = Some(DecisionFields {
            owner: Some("张三".into()),
            deadline: None,
        });
        item.fields.todo = Some(TodoFields::default());
        fill_fields(&mut item);
        assert_eq!(
            item.fields
                .question
                .as_ref()
                .map(|question| question.unanswered),
            Some(true)
        );
        assert!(item.fields.decision.is_none());
        assert!(item.fields.todo.is_none());
    }

    #[test]
    fn key_info_keeps_no_foreign_fields() {
        let mut item = draft(InsightKind::KeyInfo, "确认下周上线");
        item.fields.question = Some(QuestionFields { unanswered: true });
        fill_fields(&mut item);
        assert!(item.fields.decision.is_none());
        assert!(item.fields.todo.is_none());
        assert!(item.fields.question.is_none());
    }
}
