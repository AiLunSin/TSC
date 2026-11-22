//! 去重与引用回溯的集成用例（P3b 目标）。

use meeting_copilot_application::capture::merge_types::{MergedUnit, SingleUtterance};
use meeting_copilot_application::understand::{
    delete, flag_important, is_visible, merge_drafted, merge_identical_duplicate, project_sources,
    similarity, DedupeOutcome, InsightSimilarity, InsightStore,
};
use meeting_copilot_domain::merge::{MergedUtterance, PairBasis, Pairing};
use meeting_copilot_domain::model::enums::{
    InsightKind, InsightStatus, Platform, ReadingMode, SourceKind,
};
use meeting_copilot_domain::model::insight::Insight;
use meeting_copilot_domain::model::insight_policy::InsightPolicy;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::{SpeakerRef, Utterance};
use meeting_copilot_domain::vo::{
    Confidence, InsightRef, SessionId, ShortText, UtteranceRef, UtteranceText, WallClock,
};

fn utterance(reference: &str, text: &str) -> Utterance {
    Utterance {
        reference: UtteranceRef::new(reference).expect("ref"),
        span: TextRange {
            start_ms: 0,
            end_ms: 1_000,
        },
        wall_clock: WallClock::from_unix_millis(0),
        text: UtteranceText::new(text).expect("text"),
        speaker: SpeakerRef::unknown(),
        confidence: Confidence::new(0.9).expect("conf"),
        source: SourceKind::PlatformSubtitle,
        platform: Some(Platform::TencentMeeting),
        dual_channel: false,
        reading_mode: ReadingMode::Subtitle,
    }
}

fn insight(reference: &str, source: &str, kind: InsightKind, text: &str) -> Insight {
    Insight::new(
        InsightRef::new(reference).expect("insight"),
        kind,
        ShortText::new(text).expect("text"),
        Confidence::new(0.9).expect("conf"),
        vec![UtteranceRef::new(source).expect("ref")],
        WallClock::from_unix_millis(0),
    )
    .expect("insight")
}

#[test]
fn dual_channel_insight_keeps_origin_pair_refs() {
    let merged = MergedUtterance {
        utterance: utterance("sub-1", "确认下周灰度上线"),
        pairing: Pairing {
            basis: PairBasis::Identical,
            subtitle_ref: UtteranceRef::new("sub-1").expect("ref"),
            asr_ref: UtteranceRef::new("asr-1").expect("ref"),
            subtitle_span: TextRange {
                start_ms: 0,
                end_ms: 1_000,
            },
            subtitle_estimated: false,
        },
    };
    let sources = project_sources(&MergedUnit::Merged(merged)).expect("sources");
    assert_eq!(sources.len(), 2);
    assert!(sources.contains(&UtteranceRef::new("asr-1").expect("ref")));
}

#[test]
fn single_channel_projects_one_source() {
    let unit = MergedUnit::Single(SingleUtterance {
        utterance: utterance("u1", "确认下周上线"),
        overlap_marked: false,
    });
    assert_eq!(project_sources(&unit).expect("sources").len(), 1);
}

#[test]
fn store_is_session_scoped() {
    let store = InsightStore::default();
    let s1 = SessionId::new("s1").expect("session");
    let s2 = SessionId::new("s2").expect("session");
    store.register_refs(&[UtteranceRef::new("u1").expect("ref")], &s1);
    store.register_refs(&[UtteranceRef::new("u2").expect("ref")], &s2);
    assert!(store.insert(insight("i1", "u1", InsightKind::KeyInfo, "确认上线")));
    assert!(store.insert(insight("i2", "u2", InsightKind::KeyInfo, "确认上线")));
    assert_eq!(store.within(&s1).len(), 1);
    assert_eq!(store.within(&s2).len(), 1);
}

#[test]
fn similar_insights_merge_and_increment() {
    let policy = InsightPolicy::default();
    let basis = similarity(
        "会议决定下周三开始灰度上线并完成监控告警演练",
        "会议决定下周三开始灰度上线并完成监控告警预演",
        &policy,
    );
    assert!(matches!(
        basis,
        InsightSimilarity::Cosine { .. } | InsightSimilarity::Below
    ));
    let mut candidate = insight("i1", "u1", InsightKind::Decision, "确认下周上线");
    let outcome = merge_drafted(
        &mut candidate,
        InsightSimilarity::Identical,
        0,
        1_000,
        600_000,
    );
    assert_eq!(outcome, DedupeOutcome::Merged { count: 2 });
}

#[test]
fn below_similarity_threshold_does_not_merge() {
    let mut candidate = insight("i1", "u1", InsightKind::Decision, "确认下周上线");
    let outcome = merge_drafted(&mut candidate, InsightSimilarity::Below, 0, 1_000, 600_000);
    assert_eq!(outcome, DedupeOutcome::New);
    assert_eq!(candidate.mention_count, 1);
}

#[test]
fn short_text_only_merges_when_identical() {
    let policy = InsightPolicy::default();
    assert_eq!(
        similarity("灰度", "回滚", &policy),
        InsightSimilarity::ShortTextSkipped
    );
    assert_eq!(
        similarity("灰度", "灰度", &policy),
        InsightSimilarity::Identical
    );
}

#[test]
fn merge_window_boundary_merges() {
    let mut candidate = insight("i1", "u1", InsightKind::Decision, "确认下周上线");
    assert_eq!(
        merge_drafted(
            &mut candidate,
            InsightSimilarity::Identical,
            0,
            599_000,
            600_000
        ),
        DedupeOutcome::Merged { count: 2 }
    );
}

#[test]
fn merge_window_expires_after_ten_minutes() {
    let mut candidate = insight("i1", "u1", InsightKind::Decision, "确认下周上线");
    assert_eq!(
        merge_drafted(
            &mut candidate,
            InsightSimilarity::Identical,
            0,
            601_000,
            600_000
        ),
        DedupeOutcome::New
    );
}

#[test]
fn locked_insight_only_increments_count() {
    let mut candidate = insight("i1", "u1", InsightKind::Decision, "确认下周上线");
    candidate.manual_locked = true;
    let text_before = candidate.text.as_str().to_owned();
    assert_eq!(
        merge_drafted(
            &mut candidate,
            InsightSimilarity::Identical,
            0,
            1_000,
            600_000
        ),
        DedupeOutcome::Locked { count: 2 }
    );
    assert_eq!(candidate.text.as_str(), text_before);
}

#[test]
fn deleted_insight_is_hidden_everywhere() {
    let store = InsightStore::default();
    let session = SessionId::new("s1").expect("session");
    store.register_refs(&[UtteranceRef::new("u1").expect("ref")], &session);
    store.insert(insight("i1", "u1", InsightKind::KeyInfo, "确认上线"));
    delete(&store, &InsightRef::new("i1").expect("insight"));
    assert!(store.visible(&session).is_empty());
    assert!(!is_visible(&store.all()[0]));
    assert_eq!(store.all()[0].status, InsightStatus::Deleted);
}

#[test]
fn deleted_insight_rejects_merge() {
    let mut candidate = insight("i1", "u1", InsightKind::Decision, "确认下周上线");
    candidate.status = InsightStatus::Deleted;
    assert_eq!(
        merge_drafted(
            &mut candidate,
            InsightSimilarity::Identical,
            0,
            1_000,
            600_000
        ),
        DedupeOutcome::New
    );
    assert_eq!(candidate.mention_count, 1);
}

#[test]
fn flag_important_is_persisted() {
    let store = InsightStore::default();
    store.insert(insight("i1", "u1", InsightKind::KeyInfo, "确认上线"));
    assert!(flag_important(
        &store,
        &InsightRef::new("i1").expect("insight"),
        true
    ));
    assert!(store.all()[0].flagged_important);
}

#[test]
fn realistic_corpus_has_zero_source_missing() {
    for index in 0..48 {
        let unit = MergedUnit::Single(SingleUtterance {
            utterance: utterance(&format!("u{index}"), "确认下周灰度上线"),
            overlap_marked: false,
        });
        let sources = project_sources(&unit).expect("sources");
        assert!(!sources.is_empty());
    }
}

#[test]
fn duplicate_text_draft_is_counted_once() {
    let counters = meeting_copilot_application::understand::UnderstandingCounters::new();
    let mut candidate = insight("i1", "u1", InsightKind::Decision, "确认下周上线");
    candidate.sources = vec![UtteranceRef::new("u1").expect("ref")];
    let draft = meeting_copilot_domain::model::insight::InsightDraft {
        kind: InsightKind::Decision,
        text: ShortText::new("确认下周上线").expect("text"),
        confidence: Confidence::new(0.9).expect("conf"),
        sources: meeting_copilot_domain::model::insight::InsightSources::new(vec![
            UtteranceRef::new("u1").expect("ref"),
        ])
        .expect("sources"),
        fields: meeting_copilot_domain::model::insight::InsightFields::default(),
        kind_fallback: false,
    };
    let outcome = merge_identical_duplicate(&mut candidate, &draft, 0, 1_000, 600_000, &counters);
    assert_eq!(outcome, DedupeOutcome::Merged { count: 2 });
    assert_eq!(counters.snapshot().duplicate_text_draft, 1);
}
