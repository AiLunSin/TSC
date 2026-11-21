//! 合并集合区间校验收口（AC-DD-16、AC-DD-13）。
//!
//! 与 `capture_gap_gate::validate_intervals` **并存但不互相调用**：
//! 前者校验「原始片段 vs 缺口」且允许 400 ms 边界接触；
//! 本模块校验「合并后的输出集合」且缺口重叠为**严格 0**。

use meeting_copilot_domain::vo::{MonoMillis, UtteranceRef};

use super::types::CaptureGapRecord;

/// 合并后的一个输出单元（`Single` 与 `Merged` 的统一投影）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSpan {
    pub start: MonoMillis,
    pub end: MonoMillis,
    pub dual_channel: bool,
    pub overlap_marked: bool,
    pub origin_pair: Option<(UtteranceRef, UtteranceRef)>,
}

/// 合并集合违规项；不并入 `ErrorCode`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeSetViolation {
    InvalidSpan {
        unit: usize,
    },
    MergedOverlap {
        left: usize,
        right: usize,
        overlap_ms: u64,
    },
    MergedOverlapsGap {
        unit: usize,
        gap: usize,
        overlap_ms: u64,
    },
    CrossOriginOverlapNotMarked {
        left: usize,
        right: usize,
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

/// 校验合并输出集合：顺序固定为
/// `InvalidSpan → MergedOverlap → MergedOverlapsGap → CrossOriginOverlapNotMarked`。
///
/// 不变量：
/// - 任一条必须有 `start < end`；
/// - 两条 `dual_channel == true` 的单元不得相交；
/// - 任意单元与缺口不得相交（严格 0，比 400 ms 更严）；
/// - 两条 `dual_channel == false` 的单元相交时，必须 `overlap_marked == true`。
pub fn validate_merged_set(
    units: &[MergeSpan],
    gaps: &[CaptureGapRecord],
) -> Result<(), MergeSetViolation> {
    for (unit, span) in units.iter().enumerate() {
        if span.end <= span.start {
            return Err(MergeSetViolation::InvalidSpan { unit });
        }
    }

    for right in 0..units.len() {
        for left in 0..right {
            if !units[left].dual_channel && !units[right].dual_channel {
                continue;
            }
            let overlap = overlap_ms(
                units[left].start,
                units[left].end,
                units[right].start,
                units[right].end,
            );
            if overlap > 0 {
                return Err(MergeSetViolation::MergedOverlap {
                    left,
                    right,
                    overlap_ms: overlap,
                });
            }
        }
    }

    for (unit, span) in units.iter().enumerate() {
        for (gap, record) in gaps.iter().enumerate() {
            let overlap = overlap_ms(span.start, span.end, record.start, record.end);
            if overlap > 0 {
                return Err(MergeSetViolation::MergedOverlapsGap {
                    unit,
                    gap,
                    overlap_ms: overlap,
                });
            }
        }
    }

    for right in 0..units.len() {
        for left in 0..right {
            if units[left].dual_channel || units[right].dual_channel {
                continue;
            }
            if units[left].overlap_marked && units[right].overlap_marked {
                continue;
            }
            let overlap = overlap_ms(
                units[left].start,
                units[left].end,
                units[right].start,
                units[right].end,
            );
            if overlap > 0 {
                return Err(MergeSetViolation::CrossOriginOverlapNotMarked {
                    left,
                    right,
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

    fn span(start: u64, end: u64, dual: bool, marked: bool) -> MergeSpan {
        MergeSpan {
            start: at(start),
            end: at(end),
            dual_channel: dual,
            overlap_marked: marked,
            origin_pair: None,
        }
    }

    fn gap(start: u64, end: u64) -> CaptureGapRecord {
        CaptureGapRecord {
            session: SessionId::new("s1").expect("session"),
            start: at(start),
            end: at(end),
            reason: GapReason::BothChannelsUnavailable,
            backfilled: false,
        }
    }

    #[test]
    fn empty_set_is_valid() {
        assert!(validate_merged_set(&[], &[]).is_ok());
    }

    #[test]
    fn single_unit_is_valid() {
        assert!(validate_merged_set(&[span(0, 1_000, false, false)], &[]).is_ok());
    }

    #[test]
    fn non_positive_span_is_invalid() {
        assert_eq!(
            validate_merged_set(&[span(1_000, 1_000, false, false)], &[]),
            Err(MergeSetViolation::InvalidSpan { unit: 0 })
        );
        assert_eq!(
            validate_merged_set(&[span(1_001, 1_000, false, false)], &[]),
            Err(MergeSetViolation::InvalidSpan { unit: 0 })
        );
    }

    #[test]
    fn merged_units_must_not_overlap() {
        let units = [span(0, 1_000, true, false), span(900, 2_000, true, false)];
        assert_eq!(
            validate_merged_set(&units, &[]),
            Err(MergeSetViolation::MergedOverlap {
                left: 0,
                right: 1,
                overlap_ms: 100
            })
        );
    }

    #[test]
    fn merged_unit_must_not_overlap_gap() {
        let units = [span(0, 1_000, true, false)];
        let gaps = [gap(1_000, 2_000), gap(500, 1_500)];
        assert_eq!(
            validate_merged_set(&units, &gaps),
            Err(MergeSetViolation::MergedOverlapsGap {
                unit: 0,
                gap: 1,
                overlap_ms: 500
            })
        );
    }

    #[test]
    fn boundary_contact_is_valid() {
        let units = [
            span(0, 1_000, true, false),
            span(1_000, 2_000, false, false),
        ];
        assert!(validate_merged_set(&units, &[gap(2_000, 3_000)]).is_ok());
    }
}
