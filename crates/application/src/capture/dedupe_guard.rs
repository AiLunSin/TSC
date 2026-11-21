//! 双通道去重合并编排：唯一有状态组件。
//!
//! 纪律：不写仓储、不删片段、不新增线程/定时器；只在入库前做合并与计数。

use std::sync::Mutex;

use meeting_copilot_domain::merge::{
    build_merged, classify_equivalence, normalize_text, MergePolicy, MergePolicyError,
};
use meeting_copilot_domain::model::channel::ChannelKind;
use meeting_copilot_domain::model::enums::{ReadingMode, SourceKind};
use meeting_copilot_domain::model::utterance::Utterance;
use meeting_copilot_domain::vo::{MonoMillis, WallClock};

use super::merge_types::{
    AsrTextEvent, DedupeCounters, DedupeEvidence, MergeCounters, MergedUnit, SingleUtterance,
};

struct Pending {
    event: AsrTextEvent,
    side: ChannelKind,
    deadline: u64,
}

struct State {
    pending: Vec<Pending>,
    seen: Vec<(String, u8, String)>,
    counters: MergeCounters,
    evidence: DedupeEvidence,
}

/// 合并器：开关、窗口、去重与计数都在这里，但**不接触仓储**。
pub struct DedupeMergeGuard {
    policy: MergePolicy,
    state: Mutex<State>,
}

fn side_code(side: ChannelKind) -> u8 {
    match side {
        ChannelKind::Subtitle => 0,
        ChannelKind::Audio => 1,
    }
}

fn intersection_ms(left: &AsrTextEvent, right: &AsrTextEvent) -> u64 {
    let begin = left.span.start_ms.max(right.span.start_ms);
    let finish = left.span.end_ms.min(right.span.end_ms);
    finish.saturating_sub(begin)
}

fn to_utterance(event: &AsrTextEvent, side: ChannelKind) -> Utterance {
    Utterance {
        reference: event.reference.clone(),
        span: event.span.clone(),
        wall_clock: WallClock::from_unix_millis(0),
        text: event.text.clone(),
        speaker: event.speaker.clone(),
        confidence: event.confidence,
        source: match side {
            ChannelKind::Subtitle => SourceKind::PlatformSubtitle,
            ChannelKind::Audio => SourceKind::PlatformAudio,
        },
        platform: event.platform,
        dual_channel: false,
        reading_mode: match side {
            ChannelKind::Subtitle => ReadingMode::Subtitle,
            ChannelKind::Audio => ReadingMode::Audio,
        },
    }
}

fn single(event: &AsrTextEvent, side: ChannelKind, overlap_marked: bool) -> MergedUnit {
    MergedUnit::Single(SingleUtterance {
        utterance: to_utterance(event, side),
        overlap_marked,
    })
}

impl DedupeMergeGuard {
    pub fn new(policy: MergePolicy) -> Result<Self, MergePolicyError> {
        policy.validate()?;
        Ok(Self {
            policy,
            state: Mutex::new(State {
                pending: Vec::new(),
                seen: Vec::new(),
                counters: MergeCounters::default(),
                evidence: DedupeEvidence::default(),
            }),
        })
    }

    pub fn policy(&self) -> MergePolicy {
        self.policy
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// 摄入一条事件；返回本次可落盘的单元（可能为空）。
    pub fn push(&self, side: ChannelKind, event: AsrTextEvent) -> Vec<MergedUnit> {
        let mut state = self.lock();
        let current = single(&event, side, false);

        if !self.policy.merge_enabled {
            return vec![current];
        }

        let normalized = normalize_text(event.text.as_str());
        if normalized.is_empty() || event.text.as_str().trim() == "…" {
            state.counters.non_mergeable_text += 1;
            return vec![current];
        }
        if event.span.start_ms >= event.span.end_ms {
            state.counters.merge_rejected += 1;
            return vec![current];
        }

        let key = (
            event.session.as_str().to_owned(),
            side_code(side),
            event.reference.as_str().to_owned(),
        );
        if state.seen.contains(&key) {
            state.counters.duplicate_suppressed += 1;
            return Vec::new();
        }
        state.seen.push(key);

        let window = if event.estimated {
            self.policy.pair_window_estimated_ms
        } else {
            self.policy.pair_window_ms
        };
        let deadline = event.span.end_ms.saturating_add(window);

        let late = state.pending.iter().any(|pending| {
            pending.event.session == event.session
                && pending.side != side
                && pending
                    .event
                    .span
                    .start_ms
                    .abs_diff(event.span.start_ms)
                    > window
        });
        if late {
            state.counters.late_unmerged += 1;
            state.counters.overlap_marked += 1;
            let released: Vec<MergedUnit> = state
                .pending
                .drain(..)
                .map(|pending| single(&pending.event, pending.side, true))
                .collect();
            let mut units = released;
            units.push(single(&event, side, true));
            return units;
        }

        let mut candidates: Vec<usize> = state
            .pending
            .iter()
            .enumerate()
            .filter(|(_, pending)| {
                pending.event.session == event.session
                    && pending.side != side
                    && intersection_ms(&pending.event, &event) > 0
                    && pending
                        .event
                        .span
                        .start_ms
                        .abs_diff(event.span.start_ms)
                        <= window
            })
            .map(|(index, _)| index)
            .collect();

        if candidates.is_empty() {
            state.pending.push(Pending {
                event,
                side,
                deadline,
            });
            return Vec::new();
        }

        candidates.sort_by_key(|index| {
            let pending = &state.pending[*index];
            (
                pending.event.span.start_ms,
                pending.event.span.end_ms,
                normalize_text(pending.event.text.as_str()),
                pending.event.reference.as_str().to_owned(),
            )
        });

        state.counters.pair_candidates += u64::try_from(candidates.len()).unwrap_or(u64::MAX);
        let chosen = candidates[0];
        let candidate = state.pending.remove(chosen);

        let Some(basis) = classify_equivalence(
            candidate.event.text.as_str(),
            event.text.as_str(),
            &self.policy,
        ) else {
            state.counters.merge_rejected += 1;
            state.pending.push(Pending {
                event,
                side,
                deadline,
            });
            return Vec::new();
        };

        let (subtitle, asr) = if side == ChannelKind::Subtitle {
            (&event, &candidate.event)
        } else {
            (&candidate.event, &event)
        };

        let merged = build_merged(
            &to_utterance(subtitle, ChannelKind::Subtitle),
            &to_utterance(asr, ChannelKind::Audio),
            &self.policy,
            basis,
        );

        match merged {
            Some(merged) => {
                state.counters.merge_applied += 1;
                state.evidence.merged = DedupeCounters {
                    insight_trigger_count: 1,
                    gap_detect_count: 1,
                    summary_entry_count: 1,
                    persisted_utterance_count: 1,
                    mentions_delta: 0,
                };
                vec![MergedUnit::Merged(merged)]
            }
            None => {
                state.counters.merge_rejected += 1;
                vec![single(subtitle, ChannelKind::Subtitle, false), single(asr, ChannelKind::Audio, false)]
            }
        }
    }

    /// 到期释放：超窗未配对的事件转为单路输出。
    pub fn advance_to(&self, now: MonoMillis) -> Vec<MergedUnit> {
        let mut state = self.lock();
        let now_ms = now.as_millis();
        let mut released = Vec::new();
        let mut kept = Vec::with_capacity(state.pending.len());
        for pending in state.pending.drain(..) {
            if pending.deadline > 0 && pending.deadline <= now_ms {
                released.push(single(&pending.event, pending.side, false));
            } else {
                kept.push(pending);
            }
        }
        state.pending = kept;
        released
    }

    pub fn counters(&self) -> MergeCounters {
        self.lock().counters
    }

    pub fn dedupe_evidence(&self) -> DedupeEvidence {
        self.lock().evidence
    }
}