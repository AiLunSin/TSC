use serde::{Deserialize, Serialize};

use crate::model::enums::{InsightKind, InsightStatus};
use crate::model::session::TextRange;
use crate::vo::{Confidence, InsightRef, ShortText, UtteranceRef, WallClock};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Topic {
    pub label: String,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub normalized: String,
}

/// 滚动摘要：覆盖最近一段对话，供缺口识别与去重聚类使用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollingSummary {
    pub version: u32,
    pub covered: TextRange,
    pub topics: Vec<Topic>,
    pub entities: Vec<Entity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DecisionFields {
    pub owner: Option<String>,
    pub deadline: Option<WallClock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TodoFields {
    pub assignee: Option<String>,
    pub due: Option<WallClock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct QuestionFields {
    pub unanswered: bool,
}

/// 类型专属字段。非对应类型时必须为 `None`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InsightFields {
    pub decision: Option<DecisionFields>,
    pub todo: Option<TodoFields>,
    pub question: Option<QuestionFields>,
}

/// 要点。`text` 上限 60 字，且至少引用一条片段（SPEC 6.3）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Insight {
    pub reference: InsightRef,
    pub kind: InsightKind,
    pub text: ShortText,
    pub confidence: Confidence,
    pub status: InsightStatus,
    pub flagged_important: bool,
    pub mention_count: u32,
    pub fields: InsightFields,
    pub sources: Vec<UtteranceRef>,
    pub manual_locked: bool,
    pub created_at: WallClock,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InsightError {
    #[error("要点必须至少引用一条对话片段")]
    MissingSource,
    #[error("要点已删除，禁止合并")]
    Deleted,
    #[error("字段与要点类型不匹配：{0:?}")]
    FieldsMismatch(InsightKind),
}

impl Insight {
    pub fn new(
        reference: InsightRef,
        kind: InsightKind,
        text: ShortText,
        confidence: Confidence,
        sources: Vec<UtteranceRef>,
        created_at: WallClock,
    ) -> Result<Self, InsightError> {
        if sources.is_empty() {
            return Err(InsightError::MissingSource);
        }
        Ok(Self {
            reference,
            kind,
            text,
            confidence,
            status: InsightStatus::Auto,
            flagged_important: false,
            mention_count: 1,
            fields: InsightFields::default(),
            sources,
            manual_locked: false,
            created_at,
        })
    }

    /// 人工编辑优先：锁定后自动合并只增加计数，不覆盖文本（SPEC 6.3-7）。
    pub fn lock_for_manual_edit(&mut self) {
        self.manual_locked = true;
        self.status = InsightStatus::Edited;
    }

    pub fn merge_repeated(&mut self, other: &Self) -> Result<(), InsightError> {
        if self.status == InsightStatus::Deleted {
            return Err(InsightError::Deleted);
        }
        self.mention_count = self.mention_count.saturating_add(other.mention_count.max(1));
        if !self.manual_locked {
            for source in &other.sources {
                if !self.sources.contains(source) {
                    self.sources.push(source.clone());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insight() -> Insight {
        Insight::new(
            InsightRef::new("ins-1").expect("ref"),
            InsightKind::Decision,
            ShortText::new("下周三开始灰度").expect("text"),
            Confidence::new(0.9).expect("conf"),
            vec![UtteranceRef::new("utt-1").expect("source")],
            WallClock::from_unix_millis(1_000),
        )
        .expect("合法要点")
    }

    #[test]
    fn insight_requires_source() {
        let error = Insight::new(
            InsightRef::new("ins-2").expect("ref"),
            InsightKind::KeyInfo,
            ShortText::new("孤立要点").expect("text"),
            Confidence::new(0.5).expect("conf"),
            Vec::new(),
            WallClock::from_unix_millis(1_000),
        );
        assert_eq!(error, Err(InsightError::MissingSource));
    }

    #[test]
    fn manual_edit_is_not_overwritten_by_merge() {
        let mut mine = insight();
        mine.lock_for_manual_edit();
        let original = mine.text.clone();

        let mut other = insight();
        other.text = ShortText::new("自动生成的另一版表述").expect("text");
        mine.merge_repeated(&other).expect("合并");

        assert_eq!(mine.text, original);
        assert_eq!(mine.status, InsightStatus::Edited);
        assert_eq!(mine.mention_count, 2);
    }

    #[test]
    fn deleted_insight_cannot_merge() {
        let mut mine = insight();
        mine.status = InsightStatus::Deleted;
        assert_eq!(mine.merge_repeated(&insight()), Err(InsightError::Deleted));
    }
}