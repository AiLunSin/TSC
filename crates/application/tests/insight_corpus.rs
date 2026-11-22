//! 要点语料回归集（30.6）：四类信号、兜底 kind 与占位拒绝。

use meeting_copilot_application::understand::rule_fallback::produce;
use meeting_copilot_application::understand::{prefilter_hit, NormalizedFragment};
use meeting_copilot_domain::model::enums::InsightKind;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::SpeakerRef;
use meeting_copilot_domain::vo::{Confidence, SessionId, UtteranceRef, UtteranceText};

const CORPUS: &[(&str, InsightKind, bool)] = &[
    ("确认已经上线", InsightKind::Decision, true),
    ("我来跟进回滚方案", InsightKind::Todo, true),
    ("是否已经完成监控演练？", InsightKind::Question, true),
    ("嗯好的", InsightKind::KeyInfo, false),
];

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
fn corpus_prefilter_matches_signals() {
    for (text, _, hit) in CORPUS {
        assert_eq!(prefilter_hit(text), *hit, "预筛口径不符：{text}");
    }
}

#[test]
fn corpus_fallback_kind_matches_signals() {
    for (text, kind, hit) in CORPUS {
        if !*hit {
            continue;
        }
        let draft = produce(&fragment(text)).expect("draft");
        assert_eq!(draft.kind, *kind, "kind 不符：{text}");
        assert!(!draft.sources.is_empty());
    }
}

#[test]
fn corpus_placeholder_produces_none() {
    assert!(produce(&fragment("…")).is_none());
    assert!(produce(&fragment("……")).is_some());
}

#[test]
fn corpus_todo_always_has_action() {
    let draft = produce(&fragment("我来跟进回滚方案")).expect("draft");
    assert!(draft
        .fields
        .todo
        .as_ref()
        .and_then(|todo| todo.action.as_ref())
        .is_some());
}

#[test]
fn corpus_question_is_unanswered() {
    let draft = produce(&fragment("是否已经完成监控演练？")).expect("draft");
    assert_eq!(
        draft
            .fields
            .question
            .as_ref()
            .map(|question| question.unanswered),
        Some(true)
    );
}
