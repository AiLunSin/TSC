//! 缺口合并（B2.23.18.x）：同会话、容差内相邻缺口合并，避免“重/漏”。
//!
//! 纯函数；不写日志、不触碰 Session 状态。

use std::collections::HashMap;

use super::types::CaptureGapRecord;

/// 按会话合并；`next.start <= prev.end + tolerance_ms` 视为同一缺口。
/// 结果按 start 升序；`backfilled` 取 OR。
pub fn merge_gaps(records: Vec<CaptureGapRecord>, tolerance_ms: u64) -> Vec<CaptureGapRecord> {
    let mut by_session: HashMap<_, Vec<CaptureGapRecord>> = HashMap::new();
    for record in records {
        by_session
            .entry(record.session.clone())
            .or_default()
            .push(record);
    }

    let mut merged = Vec::new();
    for (_, mut gaps) in by_session {
        gaps.sort_by_key(|gap| gap.start);
        let mut current: Option<CaptureGapRecord> = None;
        for gap in gaps {
            match current.as_mut() {
                Some(previous)
                    if gap.start.as_millis()
                        <= previous.end.as_millis().saturating_add(tolerance_ms) =>
                {
                    if gap.end > previous.end {
                        previous.end = gap.end;
                    }
                    previous.backfilled = previous.backfilled || gap.backfilled;
                }
                Some(previous) => {
                    merged.push(previous.clone());
                    current = Some(gap);
                }
                None => current = Some(gap),
            }
        }
        if let Some(previous) = current {
            merged.push(previous);
        }
    }
    merged.sort_by_key(|gap| gap.start);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::GapReason;
    use meeting_copilot_domain::vo::{MonoMillis, SessionId};

    fn gap(
        session: &str,
        start: u64,
        end: u64,
        reason: GapReason,
        backfilled: bool,
    ) -> CaptureGapRecord {
        CaptureGapRecord {
            session: SessionId::new(session).expect("session"),
            start: MonoMillis::from_millis(start),
            end: MonoMillis::from_millis(end),
            reason,
            backfilled,
        }
    }

    #[test]
    fn adjacent_gaps_within_tolerance_merge() {
        let merged = merge_gaps(
            vec![
                gap("s1", 0, 1_000, GapReason::BothChannelsUnavailable, false),
                gap("s1", 1_400, 2_000, GapReason::BothChannelsUnavailable, true),
            ],
            400,
        );
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].start.as_millis(), 0);
        assert_eq!(merged[0].end.as_millis(), 2_000);
        assert!(merged[0].backfilled);
    }

    #[test]
    fn gap_beyond_tolerance_is_not_merged() {
        let merged = merge_gaps(
            vec![
                gap("s1", 0, 1_000, GapReason::BothChannelsUnavailable, false),
                gap(
                    "s1",
                    1_401,
                    2_000,
                    GapReason::BothChannelsUnavailable,
                    false,
                ),
            ],
            400,
        );
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn different_sessions_are_not_merged() {
        let merged = merge_gaps(
            vec![
                gap("s1", 0, 1_000, GapReason::BothChannelsUnavailable, false),
                gap("s2", 900, 2_000, GapReason::BothChannelsUnavailable, false),
            ],
            400,
        );
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn output_is_sorted_by_start() {
        let merged = merge_gaps(
            vec![
                gap("s1", 2_000, 2_100, GapReason::PreferenceRestricted, false),
                gap("s1", 0, 1_000, GapReason::BothChannelsUnavailable, false),
            ],
            0,
        );
        assert_eq!(merged[0].start.as_millis(), 0);
        assert_eq!(merged[1].start.as_millis(), 2_000);
    }
}
