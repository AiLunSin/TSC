//! 理解层输入/输出类型。

use thiserror::Error;

use meeting_copilot_domain::model::enums::{Platform, SessionState};
use meeting_copilot_domain::model::insight_policy::InsightPolicyError;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::SpeakerRef;
use meeting_copilot_domain::vo::{Confidence, SessionId, UtteranceRef, UtteranceText};

use crate::capture::merge_types::MergedUnit;

use super::ports::SilenceSignal;

/// 初始化错误；不并入 `ErrorCode` 冻结集。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UnderstandInitError {
    #[error("理解层策略非法：{0}")]
    Policy(InsightPolicyError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnderstandInput {
    pub session: SessionId,
    pub unit: MergedUnit,
    pub state: SessionState,
    pub silence: SilenceSignal,
    pub session_platform: Option<Platform>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    Paused,
    SourcePlatformMismatch,
    Speculative,
    Silence,
    Empty,
    LowConfidence,
}

/// 归一化后的片段：合并单元的投影。
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedFragment {
    pub session: SessionId,
    pub reference: UtteranceRef,
    pub parent_reference: UtteranceRef,
    pub span: TextRange,
    pub text: UtteranceText,
    pub speaker: SpeakerRef,
    pub confidence: Confidence,
    pub dual_channel: bool,
    pub origin_pair: Option<(UtteranceRef, UtteranceRef)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnderstandOutcome {
    Disabled,
    Duplicate,
    Rejected(RejectReason),
    StoredOnly { reason: RejectReason },
    Admitted { fragments: Vec<NormalizedFragment> },
}
