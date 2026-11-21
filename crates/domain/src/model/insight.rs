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
    #[serde(default)]
    pub action: Option<ShortText>,
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
    #[serde(default)]
    pub supplement_of: Option<InsightRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InsightError {
    #[error("要点必须至少引用一条对话片段")]
    MissingSource,
    #[error("要点已删除，禁止合并")]
    Deleted,
    #[error("字段与要点类型不匹配：{0:?}")]
    FieldsMismatch(InsightKind),
    #[error("待办缺少动作")]
    TodoActionMissing,
    #[error("正文超长：{actual} 超过 {limit}")]
    OversizeText { actual: usize, limit: usize },
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
            supplement_of: None,
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
        self.mention_count = self
            .mention_count
            .saturating_add(other.mention_count.max(1));
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

/// 非空来源集合：空集合在类型上不可构造（反序列化同样拒绝）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<UtteranceRef>", into = "Vec<UtteranceRef>")]
pub struct InsightSources(Vec<UtteranceRef>);

impl InsightSources {
    pub fn new(refs: Vec<UtteranceRef>) -> Result<Self, InsightError> {
        if refs.is_empty() {
            return Err(InsightError::MissingSource);
        }
        let mut deduped: Vec<UtteranceRef> = Vec::new();
        for reference in refs {
            if !deduped.contains(&reference) {
                deduped.push(reference);
            }
        }
        Ok(Self(deduped))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_slice(&self) -> &[UtteranceRef] {
        &self.0
    }

    pub fn contains(&self, reference: &UtteranceRef) -> bool {
        self.0.contains(reference)
    }

    /// 去重保序追加（合并单元的 `origin_pair` 双引用）。
    pub fn with_parent(&self, extra: UtteranceRef) -> Self {
        let mut refs = self.0.clone();
        if !refs.contains(&extra) {
            refs.push(extra);
        }
        Self(refs)
    }
}

impl TryFrom<Vec<UtteranceRef>> for InsightSources {
    type Error = InsightError;

    fn try_from(refs: Vec<UtteranceRef>) -> Result<Self, Self::Error> {
        Self::new(refs)
    }
}

impl From<InsightSources> for Vec<UtteranceRef> {
    fn from(sources: InsightSources) -> Self {
        sources.0
    }
}

/// 抽取草稿：来源在类型上非空，运行期再做一次断言。
#[derive(Debug, Clone, PartialEq)]
pub struct InsightDraft {
    pub kind: InsightKind,
    pub text: ShortText,
    pub confidence: Confidence,
    pub sources: InsightSources,
    pub fields: InsightFields,
    pub kind_fallback: bool,
}

impl InsightDraft {
    pub fn validate(&self) -> Result<(), InsightError> {
        if matches!(self.kind, InsightKind::Todo)
            && self
                .fields
                .todo
                .as_ref()
                .and_then(|todo| todo.action.as_ref())
                .is_none()
        {
            return Err(InsightError::TodoActionMissing);
        }
        let chars = self.text.as_str().chars().count();
        if chars > ShortText::LIMIT {
            return Err(InsightError::OversizeText {
                actual: chars,
                limit: ShortText::LIMIT,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insight_sources_reject_empty() {
        assert_eq!(
            InsightSources::new(Vec::new()),
            Err(InsightError::MissingSource)
        );
    }

    #[test]
    fn insight_sources_with_parent_is_order_preserving() {
        let base =
            InsightSources::new(vec![UtteranceRef::new("a").expect("ref")]).expect("sources");
        let same = base.with_parent(UtteranceRef::new("a").expect("ref"));
        assert_eq!(same.len(), 1);
        let extended = same.with_parent(UtteranceRef::new("b").expect("ref"));
        assert_eq!(extended.len(), 2);
        assert_eq!(extended.as_slice()[1].as_str(), "b");
    }

    #[test]
    fn draft_requires_todo_action() {
        let draft = InsightDraft {
            kind: InsightKind::Todo,
            text: ShortText::new("完成回滚方案").expect("text"),
            confidence: Confidence::new(0.9).expect("conf"),
            sources: InsightSources::new(vec![UtteranceRef::new("a").expect("ref")])
                .expect("sources"),
            fields: InsightFields::default(),
            kind_fallback: false,
        };
        assert_eq!(draft.validate(), Err(InsightError::TodoActionMissing));
    }

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
