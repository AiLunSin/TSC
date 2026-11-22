//! 13.3 回放本体：驱动、装载、比对与报告（0 IO / 0 网络 / 0 随机 / 0 生产计数）。

use std::collections::BTreeSet;

use meeting_copilot_domain::model::gate::{GateDecision, GatePolicy, GateRecord};
use meeting_copilot_domain::model::gate_replay::{ExpectedDecision, ReplayReport};

use crate::gate::{detect_gaps, evaluate_gate, record_from, FixedClock, GateClock, GateInput};

/// 时钟适配：首值取自固定序列时钟，tick 恒为 0 时逐次读数一致。
struct ReplayClock {
    now_ms: u64,
}

impl ReplayClock {
    fn new(anchor_ms: u64) -> Self {
        let fixed = FixedClock::new(anchor_ms, 0);
        Self {
            now_ms: fixed.now_ms(),
        }
    }
}

impl GateClock for ReplayClock {
    fn now_ms(&self) -> u64 {
        self.now_ms
    }
}

/// 执行层用例：4 项封闭，只借用用例源内部 arena 的只读快照。
#[derive(Clone)]
pub struct ReplayCase<'a> {
    pub case_id: &'a str,
    pub input: GateInput<'a>,
    pub policy: GatePolicy,
    pub expected: ExpectedDecision,
}

/// 用例源：`seed` 只进报告，不参与排序、筛选或抽样。
pub trait ReplayCaseSource {
    fn seed(&self) -> u64;
    fn cases(&self) -> Vec<ReplayCase<'_>>;
}

/// 回放驱动：无状态 unit，唯一入口为 `run`。
#[derive(Debug, Clone, Copy, Default)]
pub struct ReplayDriver;

impl ReplayDriver {
    /// 装载校验后逐例执行三函数，再按决策与记录核验产出失败项。
    pub fn run(&self, source: &dyn ReplayCaseSource) -> Result<ReplayRun, ReplayLoadError> {
        let seed = source.seed();
        let mut cases = source.cases();

        load_guard(&cases)?;
        cases.sort_by(|left, right| left.case_id.cmp(right.case_id));

        let mut failures = Vec::new();
        let mut records = Vec::with_capacity(cases.len());

        for case in &cases {
            let gaps = detect_gaps(&case.input.scan);
            debug_assert!(gaps.len() <= 64);
            let clock = ReplayClock::new(case.input.now_ms);
            let decision = evaluate_gate(&case.input, &case.policy, &clock);
            let record = record_from(&case.input, &decision, &case.input.budget);

            let mut code = compare_decision(&case.expected, &decision);
            if code.is_none() {
                code = record_check(&record);
            }
            if let Some(code) = code {
                failures.push(ReplayFailure {
                    case_id: case.case_id.to_owned(),
                    code,
                });
            }
            records.push(record);
        }

        let total = u32::try_from(cases.len()).unwrap_or(u32::MAX);
        let mismatched = u32::try_from(failures.len()).unwrap_or(u32::MAX);
        let report = ReplayReport {
            total,
            matched: total.saturating_sub(mismatched),
            mismatched,
            mismatch_cases: failures
                .iter()
                .map(|failure| failure.case_id.clone())
                .collect(),
            seed,
        };

        Ok(ReplayRun {
            report,
            failures,
            records,
        })
    }
}

/// 运行结果：3 项封闭；计数不变量由报告与两个序列共同承载。
#[derive(Clone, PartialEq)]
pub struct ReplayRun {
    pub report: ReplayReport,
    pub failures: Vec<ReplayFailure>,
    pub records: Vec<GateRecord>,
}

impl core::fmt::Debug for ReplayCase<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ReplayCase")
            .field("case_id", &self.case_id)
            .finish()
    }
}

impl core::fmt::Debug for ReplayRun {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ReplayRun")
            .field("report", &self.report)
            .field("failures", &self.failures.len())
            .field("records", &self.records.len())
            .finish()
    }
}

/// 失败项：只含用例标识与原因码，不携带期望或实际结构。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayFailure {
    pub case_id: String,
    pub code: ReplayMismatchCode,
}

/// 失败原因码：5 变体封闭，按声明顺序互斥判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMismatchCode {
    ExpectedAllowGotDeny,
    ExpectedDenyGotAllow,
    ReasonMismatch,
    StageMismatch,
    RecordViolation,
}

impl ReplayMismatchCode {
    pub const ALL: [Self; 5] = [
        Self::ExpectedAllowGotDeny,
        Self::ExpectedDenyGotAllow,
        Self::ReasonMismatch,
        Self::StageMismatch,
        Self::RecordViolation,
    ];
}

/// 装载错误：3 变体封闭，判定顺序空集、形态、唯一。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayLoadError {
    EmptyCaseSet,
    DuplicateCaseId,
    InvalidCaseId,
}

impl ReplayLoadError {
    pub const ALL: [Self; 3] = [
        Self::EmptyCaseSet,
        Self::DuplicateCaseId,
        Self::InvalidCaseId,
    ];
}

fn load_guard(cases: &[ReplayCase<'_>]) -> Result<(), ReplayLoadError> {
    if cases.is_empty() {
        return Err(ReplayLoadError::EmptyCaseSet);
    }

    for case in cases {
        if !is_valid_case_id(case.case_id) {
            return Err(ReplayLoadError::InvalidCaseId);
        }
    }

    let mut seen = BTreeSet::new();
    for case in cases {
        if !seen.insert(case.case_id) {
            return Err(ReplayLoadError::DuplicateCaseId);
        }
    }

    Ok(())
}

fn is_valid_case_id(case_id: &str) -> bool {
    !case_id.is_empty()
        && case_id.len() <= 64
        && case_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn compare_decision(
    expected: &ExpectedDecision,
    decision: &GateDecision,
) -> Option<ReplayMismatchCode> {
    match (expected, decision) {
        (ExpectedDecision::Allow, GateDecision::Allow { .. }) => None,
        (ExpectedDecision::Allow, GateDecision::Deny { .. }) => {
            Some(ReplayMismatchCode::ExpectedAllowGotDeny)
        }
        (ExpectedDecision::Deny { .. }, GateDecision::Allow { .. }) => {
            Some(ReplayMismatchCode::ExpectedDenyGotAllow)
        }
        (
            ExpectedDecision::Deny { reason, stopped_at },
            GateDecision::Deny {
                reason: actual_reason,
                stopped_at: actual_stage,
                ..
            },
        ) => {
            if reason != actual_reason {
                Some(ReplayMismatchCode::ReasonMismatch)
            } else if stopped_at != actual_stage {
                Some(ReplayMismatchCode::StageMismatch)
            } else {
                None
            }
        }
    }
}

fn record_check(record: &GateRecord) -> Option<ReplayMismatchCode> {
    let closed = record.schema_version == 1
        && record.refs_are_valid()
        && record.skipped_stages_are_valid()
        && record.decision_tag_matches_stopped_at();

    if closed {
        None
    } else {
        Some(ReplayMismatchCode::RecordViolation)
    }
}
