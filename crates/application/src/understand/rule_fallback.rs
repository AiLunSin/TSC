//! 应用层规则兜底：网关不可用 / 超时 / 冷却时产出四类草稿。

use meeting_copilot_domain::model::insight::InsightSources;
use meeting_copilot_domain::model::insight::{
    InsightDraft, InsightFields, QuestionFields, TodoFields,
};
use meeting_copilot_domain::vo::ShortText;

use super::insight_extractor::classify_signals;
use super::types::NormalizedFragment;

/// 取正文前 60 个标量值（不调用 UI 截断辅助）。
fn head_60(text: &str) -> String {
    text.chars().take(ShortText::LIMIT).collect()
}

pub fn produce(fragment: &NormalizedFragment) -> Option<InsightDraft> {
    let raw = fragment.text.as_str();
    if raw.trim().is_empty() {
        return None;
    }
    let classification = classify_signals(raw);
    let text = ShortText::new(head_60(raw)).ok()?;
    let sources = InsightSources::new(vec![fragment.reference.clone()]).ok()?;

    let mut fields = InsightFields::default();
    match classification.kind {
        meeting_copilot_domain::model::enums::InsightKind::Todo => {
            fields.todo = Some(TodoFields {
                action: Some(text.clone()),
                ..TodoFields::default()
            });
        }
        meeting_copilot_domain::model::enums::InsightKind::Question => {
            fields.question = Some(QuestionFields { unanswered: true });
        }
        _ => {}
    }

    Some(InsightDraft {
        kind: classification.kind,
        text,
        confidence: fragment.confidence,
        sources,
        fields,
        kind_fallback: classification.fallback,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::understand::types::NormalizedFragment;
    use meeting_copilot_domain::model::session::TextRange;
    use meeting_copilot_domain::model::utterance::SpeakerRef;
    use meeting_copilot_domain::vo::{Confidence, SessionId, UtteranceRef, UtteranceText};

    fn fragment(text: &str) -> NormalizedFragment {
        NormalizedFragment {
            session: SessionId::new("s1").expect("session"),
            reference: UtteranceRef::new("u1").expect("ref"),
            parent_reference: UtteranceRef::new("u1").expect("ref"),
            span: TextRange {
                start_ms: 0,
                end_ms: 1_000,
            },
            text: UtteranceText::new(text).expect("text"),
            speaker: SpeakerRef::unknown(),
            confidence: Confidence::new(0.9).expect("conf"),
            dual_channel: false,
            origin_pair: None,
        }
    }

    #[test]
    fn fallback_produces_todo_with_action() {
        let draft = produce(&fragment("我来跟进回滚方案")).expect("draft");
        assert!(draft
            .fields
            .todo
            .as_ref()
            .and_then(|todo| todo.action.as_ref())
            .is_some());
        assert_eq!(draft.sources.len(), 1);
    }

    #[test]
    fn fallback_question_is_unanswered() {
        let draft = produce(&fragment("是否下周上线？")).expect("draft");
        assert_eq!(
            draft
                .fields
                .question
                .as_ref()
                .map(|question| question.unanswered),
            Some(true)
        );
    }

    #[test]
    fn fallback_plain_text_is_key_info() {
        let draft = produce(&fragment("嗯好的")).expect("draft");
        assert!(draft.kind_fallback);
    }
}
