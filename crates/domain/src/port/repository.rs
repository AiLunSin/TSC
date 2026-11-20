use crate::error::CopilotError;
use crate::model::card::SuggestionCard;
use crate::model::enums::SessionState;
use crate::model::insight::{Insight, RollingSummary};
use crate::model::session::Session;
use crate::model::utterance::{Correction, Utterance};
use crate::vo::{CardRef, InsightRef, SessionId, UtteranceRef};

/// 会话仓储。删除必须是不可恢复的（SPEC 5.5.1-8）。
pub trait SessionRepository: Send + Sync {
    fn create(&self, session: &Session) -> Result<(), CopilotError>;

    fn transition(&self, id: &SessionId, next: SessionState) -> Result<Session, CopilotError>;

    fn get(&self, id: &SessionId) -> Result<Option<Session>, CopilotError>;

    /// 1 分钟内完成：删密钥派生材料 → 清正文与索引 → 仅保留脱敏审计摘要。
    fn delete_irrecoverable(&self, id: &SessionId) -> Result<(), CopilotError>;
}

pub trait UtteranceRepository: Send + Sync {
    /// 主表 + FTS5 + 向量索引三写同事务。
    fn append(&self, session: &SessionId, utterance: &Utterance) -> Result<(), CopilotError>;

    fn apply_correction(&self, session: &SessionId, correction: &Correction)
        -> Result<(), CopilotError>;

    fn list(&self, session: &SessionId) -> Result<Vec<Utterance>, CopilotError>;

    /// 被要点引用的片段默认禁止删除。
    fn delete(&self, session: &SessionId, reference: &UtteranceRef) -> Result<(), CopilotError>;
}

pub trait InsightRepository: Send + Sync {
    fn save(&self, session: &SessionId, insight: &Insight) -> Result<(), CopilotError>;

    fn get(&self, session: &SessionId, reference: &InsightRef) -> Result<Option<Insight>, CopilotError>;

    fn list(&self, session: &SessionId) -> Result<Vec<Insight>, CopilotError>;

    fn save_summary(&self, session: &SessionId, summary: &RollingSummary) -> Result<(), CopilotError>;
}

pub trait CardRepository: Send + Sync {
    fn save(&self, session: &SessionId, card: &SuggestionCard) -> Result<(), CopilotError>;

    fn get(&self, session: &SessionId, reference: &CardRef) -> Result<Option<SuggestionCard>, CopilotError>;

    fn update(&self, session: &SessionId, card: &SuggestionCard) -> Result<(), CopilotError>;
}