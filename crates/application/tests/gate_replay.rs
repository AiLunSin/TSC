//! 回放面收敛：4 函数 + 3 结构（AC-B4-46/48/49）。

mod common;

use common::Harness;
use meeting_copilot_application::gate::{detect_gaps, FixedClock};
use meeting_copilot_domain::model::gate_replay::ReplayReport;

#[test]
fn replay_interface_is_io_free() {
    let harness = Harness::new(10_000);
    let scan = harness.scan();
    let first = detect_gaps(&scan);
    for _ in 0..10 {
        assert_eq!(detect_gaps(&scan), first);
    }
    let decision = harness.outcome().decision;
    assert_eq!(harness.outcome().decision, decision);
}

#[test]
fn fixed_clock_is_deterministic_sequence() {
    let clock = FixedClock::new(1_000, 250);
    let first: Vec<u64> = (0..3).map(|_| clock.now_ms()).collect();
    let clock = FixedClock::new(1_000, 250);
    let second: Vec<u64> = (0..3).map(|_| clock.now_ms()).collect();
    assert_eq!(first, second);
    assert_eq!(first, vec![1_000, 1_250, 1_500]);
}

#[test]
fn replay_report_has_closed_fields() {
    let report = ReplayReport {
        total: 200,
        matched: 200,
        mismatched: 0,
        mismatch_cases: Vec::new(),
        seed: 7,
    };
    assert_eq!(report.total, 200);
    assert_eq!(report.mismatched, 0);
    assert_eq!(report.mismatch_cases.len(), 0);
    assert_eq!(report.seed, 7);
}
