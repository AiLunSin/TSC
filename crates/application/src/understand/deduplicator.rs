//! 去重判定与来源投影（C-03）。

use meeting_copilot_domain::merge::{ngram_cosine_milli, normalize_text};
use meeting_copilot_domain::model::enums::InsightStatus;
use meeting_copilot_domain::model::insight::{Insight, InsightDraft, InsightError, InsightSources};
use meeting_copilot_domain::model::insight_policy::InsightPolicy;
use meeting_copilot_domain::vo::WallClock;

use super::counters::{Counter, UnderstandingCounters};
use super::store::InsightStore;
use crate::capture::merge_types::MergedUnit;

/// 同主题判定的确定性常量（不进入 `InsightPolicy`）。
pub const SUPPLEMENT_TOPIC_OVERLAP_MILLI: u16 = 400;

/// 四步判定结果；`ShortTextSkipped` 与 `Below` 都表示不合并。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsightSimilarity {
    Identical,
    Cosine { milli: u16 },
    ShortTextSkipped,
    Below,
}

/// `Single` → 自身引用；`Merged` → `[正文引用, 字幕引用, ASR 引用]` 去重保序。
pub fn project_sources(unit: &MergedUnit) -> Result<InsightSources, InsightError> {
    let refs = match unit {
        MergedUnit::Single(single) => vec![single.utterance.reference.clone()],
        MergedUnit::Merged(merged) => vec![
            merged.utterance.reference.clone(),
            merged.pairing.subtitle_ref.clone(),
            merged.pairing.asr_ref.clone(),
        ],
    };
    InsightSources::new(refs)
}

/// ① 归一化相等 → `Identical`；② 短于阈值且不相等 → `ShortTextSkipped`；
/// ③ 余弦 ≥ 阈值 → `Cosine`；④ 否则 `Below`。
pub fn similarity(left: &str, right: &str, policy: &InsightPolicy) -> InsightSimilarity {
    let a = normalize_text(left);
    let b = normalize_text(right);
    if a == b {
        return InsightSimilarity::Identical;
    }
    let min_chars = a.chars().count().min(b.chars().count());
    if min_chars < usize::from(policy.short_text_merge_min_chars) {
        return InsightSimilarity::ShortTextSkipped;
    }
    let milli = ngram_cosine_milli(&a, &b);
    if milli >= policy.similarity_min_milli {
        InsightSimilarity::Cosine { milli }
    } else {
        InsightSimilarity::Below
    }
}

/// 三处消费者共用：`Deleted` 一律不可见。
pub fn is_visible(insight: &Insight) -> bool {
    insight.status != InsightStatus::Deleted
}

/// 三态合并结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupeOutcome {
    New,
    Merged { count: u32 },
    Locked { count: u32 },
}

/// 删除 / 不相似 / 超窗 → `New`；锁定只加计数；否则计为 `Merged`。
pub fn merge_drafted(
    candidate: &mut Insight,
    similarity: InsightSimilarity,
    created_ms: u64,
    now_ms: u64,
    window_ms: u64,
) -> DedupeOutcome {
    if candidate.status == InsightStatus::Deleted {
        return DedupeOutcome::New;
    }
    if matches!(
        similarity,
        InsightSimilarity::ShortTextSkipped | InsightSimilarity::Below
    ) {
        return DedupeOutcome::New;
    }
    if now_ms.saturating_sub(created_ms) > window_ms {
        return DedupeOutcome::New;
    }
    candidate.mention_count = candidate.mention_count.saturating_add(1);
    if meeting_copilot_domain::model::insight_lock::is_locked(candidate) {
        DedupeOutcome::Locked {
            count: candidate.mention_count,
        }
    } else {
        DedupeOutcome::Merged {
            count: candidate.mention_count,
        }
    }
}

/// 冲突但同主题：新建补充要点指向原要点，不改原条任何字段。
pub fn supplement_draft(
    original: &Insight,
    draft: &InsightDraft,
    reference: meeting_copilot_domain::vo::InsightRef,
    created_at: WallClock,
) -> Insight {
    Insight {
        reference,
        kind: draft.kind,
        text: draft.text.clone(),
        confidence: draft.confidence,
        status: InsightStatus::Auto,
        flagged_important: false,
        mention_count: 1,
        fields: draft.fields.clone(),
        sources: draft.sources.as_slice().to_vec(),
        manual_locked: false,
        created_at,
        supplement_of: Some(original.reference.clone()),
    }
}

/// 冲突并列落库：原锁定条不变，新增补充条指向原条。
pub fn apply_supplement(
    store: &InsightStore,
    original: &meeting_copilot_domain::vo::InsightRef,
    draft: &InsightDraft,
    reference: meeting_copilot_domain::vo::InsightRef,
    created_at: WallClock,
) -> bool {
    let Some(original_insight) = store.get(original) else {
        return false;
    };
    let supplement = supplement_draft(&original_insight, draft, reference, created_at);
    store.insert(supplement)
}

/// 正文与来源都与既有可见要点完全相同：先计重复草稿，再按 `Identical` 合并。
pub fn merge_identical_duplicate(
    candidate: &mut Insight,
    draft: &InsightDraft,
    created_ms: u64,
    now_ms: u64,
    window_ms: u64,
    counters: &UnderstandingCounters,
) -> DedupeOutcome {
    let same_text = candidate.text.as_str() == draft.text.as_str();
    let same_sources = candidate.sources.as_slice() == draft.sources.as_slice();
    if same_text && same_sources {
        counters.bump(Counter::DuplicateTextDraft);
    }
    merge_drafted(
        candidate,
        InsightSimilarity::Identical,
        created_ms,
        now_ms,
        window_ms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplement_is_persisted_against_locked_original() {
        let store = super::InsightStore::default();
        let mut original = insight_for_merge();
        original.manual_locked = true;
        original.status = InsightStatus::Edited;
        let original_ref = original.reference.clone();
        assert!(store.insert(original));

        let draft = InsightDraft {
            kind: InsightKind::KeyInfo,
            text: ShortText::new("补充信息").expect("text"),
            confidence: Confidence::new(0.9).expect("conf"),
            sources: InsightSources::new(vec![UtteranceRef::new("u2").expect("ref")])
                .expect("sources"),
            fields: meeting_copilot_domain::model::insight::InsightFields::default(),
            kind_fallback: false,
        };
        assert!(apply_supplement(
            &store,
            &original_ref,
            &draft,
            InsightRef::new("i2").expect("insight"),
            WallClock::from_unix_millis(0),
        ));
        let stored = store
            .get(&InsightRef::new("i2").expect("insight"))
            .expect("supplement");
        assert_eq!(stored.supplement_of, Some(original_ref.clone()));
        let locked = store.get(&original_ref).expect("original");
        assert_eq!(locked.status, InsightStatus::Edited);
        assert!(locked.manual_locked);
    }

    #[test]
    fn conflicting_content_is_listed_as_supplement() {
        let mut original = insight_for_merge();
        original.manual_locked = true;
        original.status = InsightStatus::Edited;
        let sources_before = original.sources.len();
        let text_before = original.text.as_str().to_owned();

        let draft = InsightDraft {
            kind: InsightKind::KeyInfo,
            text: ShortText::new("另一段补充信息").expect("text"),
            confidence: Confidence::new(0.9).expect("conf"),
            sources: InsightSources::new(vec![UtteranceRef::new("u2").expect("ref")])
                .expect("sources"),
            fields: meeting_copilot_domain::model::insight::InsightFields::default(),
            kind_fallback: false,
        };

        let supplement = supplement_draft(
            &original,
            &draft,
            InsightRef::new("i2").expect("insight"),
            WallClock::from_unix_millis(0),
        );

        assert_eq!(supplement.supplement_of, Some(original.reference.clone()));
        assert_eq!(supplement.mention_count, 1);
        assert_eq!(original.text.as_str(), text_before);
        assert_eq!(original.sources.len(), sources_before);
        assert_eq!(original.status, InsightStatus::Edited);
        assert!(original.manual_locked);
    }

    fn insight_for_merge() -> Insight {
        Insight::new(
            InsightRef::new("i1").expect("insight"),
            InsightKind::KeyInfo,
            ShortText::new("确认下周上线").expect("text"),
            Confidence::new(0.9).expect("conf"),
            vec![UtteranceRef::new("u1").expect("ref")],
            WallClock::from_unix_millis(0),
        )
        .expect("insight")
    }

    #[test]
    fn merge_window_boundary_merges() {
        let mut candidate = insight_for_merge();
        assert!(matches!(
            merge_drafted(
                &mut candidate,
                InsightSimilarity::Identical,
                0,
                599_000,
                600_000
            ),
            DedupeOutcome::Merged { count: 2 }
        ));
    }

    #[test]
    fn merge_window_expires_after_ten_minutes() {
        let mut candidate = insight_for_merge();
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
        assert_eq!(candidate.mention_count, 1);
    }

    #[test]
    fn locked_insight_only_increments_count() {
        let mut candidate = insight_for_merge();
        candidate.manual_locked = true;
        let text_before = candidate.text.as_str().to_owned();
        assert!(matches!(
            merge_drafted(
                &mut candidate,
                InsightSimilarity::Identical,
                0,
                1_000,
                600_000
            ),
            DedupeOutcome::Locked { count: 2 }
        ));
        assert_eq!(candidate.text.as_str(), text_before);
    }
    use meeting_copilot_domain::merge::{MergedUtterance, PairBasis, Pairing};
    use meeting_copilot_domain::model::enums::{InsightKind, Platform, ReadingMode, SourceKind};
    use meeting_copilot_domain::model::session::TextRange;
    use meeting_copilot_domain::model::utterance::{SpeakerRef, Utterance};
    use meeting_copilot_domain::vo::{
        Confidence, InsightRef, ShortText, UtteranceRef, UtteranceText, WallClock,
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
        assert!(sources.contains(&UtteranceRef::new("sub-1").expect("ref")));
        assert!(sources.contains(&UtteranceRef::new("asr-1").expect("ref")));
    }

    #[test]
    fn similarity_is_identical_cosine_skipped_or_below() {
        let policy = InsightPolicy::default();
        assert_eq!(
            similarity("确认下周上线。", "确认下周上线", &policy),
            InsightSimilarity::Identical
        );
        let long_a = "会议决定下周三开始灰度上线并完成监控告警演练";
        let long_b = "会议决定下周三开始灰度上线并完成监控告警预演";
        assert!(matches!(
            similarity(long_a, long_b, &policy),
            InsightSimilarity::Cosine { .. } | InsightSimilarity::Below
        ));
    }

    #[test]
    fn short_text_is_skipped_unless_identical() {
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
    fn deleted_insight_is_not_visible() {
        let mut insight = Insight::new(
            meeting_copilot_domain::vo::InsightRef::new("i1").expect("insight"),
            InsightKind::KeyInfo,
            meeting_copilot_domain::vo::ShortText::new("确认下周上线").expect("text"),
            Confidence::new(0.9).expect("conf"),
            vec![UtteranceRef::new("u1").expect("ref")],
            WallClock::from_unix_millis(0),
        )
        .expect("insight");
        assert!(is_visible(&insight));
        insight.status = InsightStatus::Deleted;
        assert!(!is_visible(&insight));
    }
}
