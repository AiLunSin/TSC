//! 缺口合并、门限、恢复边界与区间代数纯函数（AC-B2-OC-C01~C04、C09）。
//!
//! 全部无状态：无 I/O、无时钟、无 `unwrap()`；顺序固定为
//! `merge_for_persist` → `persist_gate`（K1）。

use meeting_copilot_domain::vo::MonoMillis;

use super::types::CaptureGapRecord;

/// 落盘门限结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    Persist,
    DropShort,
}

/// 半开区间 `[start, end)` 的时长门限：`end <= start` 一律丢弃。
pub fn persist_gate(start: MonoMillis, end: MonoMillis, gap_min_ms: u64) -> GateDecision {
    if end <= start {
        return GateDecision::DropShort;
    }
    if end.saturating_sub(start) >= gap_min_ms {
        GateDecision::Persist
    } else {
        GateDecision::DropShort
    }
}

/// 先合并：`incoming.start <= pending.end + reopen_merge_ms` 时并入同一缺口，
/// 保留最早起点、最晚终点，`backfilled` 取逻辑或；跨会话或窗口外返回 `None`。
pub fn merge_for_persist(
    pending: Option<CaptureGapRecord>,
    incoming: CaptureGapRecord,
    reopen_merge_ms: u64,
) -> Option<CaptureGapRecord> {
    let Some(mut pending) = pending else {
        return Some(incoming);
    };
    if pending.session != incoming.session {
        return None;
    }
    if incoming.start.as_millis() > pending.end.as_millis().saturating_add(reopen_merge_ms) {
        return None;
    }
    if incoming.start < pending.start {
        pending.start = incoming.start;
    }
    if incoming.end > pending.end {
        pending.end = incoming.end;
    }
    pending.backfilled = pending.backfilled || incoming.backfilled;
    pending.reason = incoming.reason;
    Some(pending)
}

/// 恢复边界：`resume = close - min(max_window, close - start)`，保证 `resume >= start`。
pub fn resume_boundary(close: MonoMillis, start: MonoMillis, max_window_ms: u64) -> MonoMillis {
    let duration = close.saturating_sub(start);
    let window = duration.min(max_window_ms);
    MonoMillis::from_millis(close.as_millis().saturating_sub(window))
}

/// 区间代数违规项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalViolation {
    GapOverlap {
        left: usize,
        right: usize,
    },
    SegmentOverlap {
        left: usize,
        right: usize,
    },
    OverlapWithGap {
        segment: usize,
        gap: usize,
        overlap_ms: u64,
    },
}

fn overlap_ms(
    a_start: MonoMillis,
    a_end: MonoMillis,
    b_start: MonoMillis,
    b_end: MonoMillis,
) -> u64 {
    let begin = a_start.max(b_start);
    let finish = a_end.min(b_end);
    finish.saturating_sub(begin)
}

/// 缺口两两不重叠；片段两两不交；片段与缺口仅允许 `<= boundary_contact_max_ms` 接触。
pub fn validate_intervals(
    gaps: &[CaptureGapRecord],
    segments: &[(MonoMillis, MonoMillis)],
    boundary_contact_max_ms: u64,
) -> Result<(), IntervalViolation> {
    for right in 0..gaps.len() {
        for left in 0..right {
            if overlap_ms(
                gaps[left].start,
                gaps[left].end,
                gaps[right].start,
                gaps[right].end,
            ) > 0
            {
                return Err(IntervalViolation::GapOverlap { left, right });
            }
        }
    }
    for right in 0..segments.len() {
        for left in 0..right {
            if overlap_ms(
                segments[left].0,
                segments[left].1,
                segments[right].0,
                segments[right].1,
            ) > 0
            {
                return Err(IntervalViolation::SegmentOverlap { left, right });
            }
        }
    }
    for (segment, (start, end)) in segments.iter().enumerate() {
        for (gap, record) in gaps.iter().enumerate() {
            let overlap = overlap_ms(*start, *end, record.start, record.end);
            if overlap > boundary_contact_max_ms {
                return Err(IntervalViolation::OverlapWithGap {
                    segment,
                    gap,
                    overlap_ms: overlap,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::GapReason;
    use meeting_copilot_domain::vo::SessionId;

    fn at(millis: u64) -> MonoMillis {
        MonoMillis::from_millis(millis)
    }

    fn gap(session: &str, start: u64, end: u64, backfilled: bool) -> CaptureGapRecord {
        CaptureGapRecord {
            session: SessionId::new(session).expect("session"),
            start: at(start),
            end: at(end),
            reason: GapReason::BothChannelsUnavailable,
            backfilled,
        }
    }

    /// AC-B2-OC-C01/C02：999 丢弃，1000/1001 落盘。
    #[test]
    fn gap_gate_persist_boundary_999_1000_1001() {
        assert_eq!(persist_gate(at(0), at(999), 1_000), GateDecision::DropShort);
        assert_eq!(persist_gate(at(0), at(1_000), 1_000), GateDecision::Persist);
        assert_eq!(persist_gate(at(0), at(1_001), 1_000), GateDecision::Persist);
    }

    /// AC-B2-OC-C02：非正时长不 panic 且判丢弃。
    #[test]
    fn gap_gate_rejects_non_positive_interval() {
        assert_eq!(
            persist_gate(at(500), at(500), 1_000),
            GateDecision::DropShort
        );
        assert_eq!(
            persist_gate(at(500), at(499), 1_000),
            GateDecision::DropShort
        );
        assert_eq!(persist_gate(at(0), at(0), 0), GateDecision::DropShort);
    }

    /// AC-B2-OC-C03：窗口内合并为 1 条。
    #[test]
    fn merge_for_persist_merges_within_reopen_window() {
        let merged = merge_for_persist(
            Some(gap("s1", 0, 600, false)),
            gap("s1", 1_000, 1_600, true),
            1_000,
        )
        .expect("merge");
        assert_eq!(merged.start, at(0));
        assert_eq!(merged.end, at(1_600));
        assert!(merged.backfilled);
    }

    /// AC-B2-OC-C03：保留最早起点与最晚终点。
    #[test]
    fn merge_for_persist_keeps_earliest_start_and_latest_end() {
        let merged = merge_for_persist(
            Some(gap("s1", 500, 1_200, false)),
            gap("s1", 100, 900, false),
            1_000,
        )
        .expect("merge");
        assert_eq!(merged.start, at(100));
        assert_eq!(merged.end, at(1_200));
    }

    /// AC-B2-OC-C03：窗口外返回 None。
    #[test]
    fn merge_for_persist_rejects_beyond_window() {
        assert!(merge_for_persist(
            Some(gap("s1", 0, 600, false)),
            gap("s1", 1_601, 2_000, false),
            1_000
        )
        .is_none());
    }

    /// AC-B2-OC-C03：跨会话不得合并。
    #[test]
    fn merge_for_persist_keeps_different_sessions_apart() {
        assert!(merge_for_persist(
            Some(gap("s1", 0, 600, false)),
            gap("s2", 600, 1_200, false),
            1_000
        )
        .is_none());
    }

    /// AC-B2-OC-C04：短于上限时用完整时长。
    #[test]
    fn resume_boundary_short_gap_uses_full_duration() {
        assert_eq!(resume_boundary(at(5_000), at(2_000), 8_000), at(2_000));
    }

    /// AC-B2-OC-C04：超过上限时截到 8000。
    #[test]
    fn resume_boundary_caps_at_max_window() {
        assert_eq!(resume_boundary(at(30_000), at(0), 8_000), at(22_000));
    }

    /// AC-B2-OC-C04：恰等于上限时保持恒等。
    #[test]
    fn resume_boundary_exact_window_is_identity() {
        assert_eq!(resume_boundary(at(8_000), at(0), 8_000), at(0));
    }

    /// AC-B2-OC-C09：缺口两两重叠被拒。
    #[test]
    fn validate_intervals_rejects_gap_overlap() {
        let gaps = [gap("s1", 0, 1_000, false), gap("s1", 900, 2_000, false)];
        assert_eq!(
            validate_intervals(&gaps, &[], 400),
            Err(IntervalViolation::GapOverlap { left: 0, right: 1 })
        );
    }

    /// AC-B2-OC-C09：400ms 边界接触放行。
    #[test]
    fn validate_intervals_allows_boundary_contact_400ms() {
        let gaps = [gap("s1", 0, 1_000, false)];
        let segments = [(at(600), at(2_000))];
        assert!(validate_intervals(&gaps, &segments, 400).is_ok());
    }

    /// AC-B2-OC-C09：超过边界接触上限被拒。
    #[test]
    fn validate_intervals_rejects_overlap_beyond_boundary() {
        let gaps = [gap("s1", 0, 1_000, false)];
        let segments = [(at(500), at(2_000))];
        assert_eq!(
            validate_intervals(&gaps, &segments, 400),
            Err(IntervalViolation::OverlapWithGap {
                segment: 0,
                gap: 0,
                overlap_ms: 500
            })
        );
    }

    /// AC-B2-OC-C09：空集合通过。
    #[test]
    fn validate_intervals_accepts_empty_sets() {
        assert!(validate_intervals(&[], &[], 400).is_ok());
    }

    /// AC-B2-OC-C09：片段两两重叠被拒。
    #[test]
    fn validate_intervals_rejects_segment_overlap() {
        let segments = [(at(0), at(1_000)), (at(500), at(1_500))];
        assert_eq!(
            validate_intervals(&[], &segments, 400),
            Err(IntervalViolation::SegmentOverlap { left: 0, right: 1 })
        );
    }
}
