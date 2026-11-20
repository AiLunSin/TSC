use std::ops::Deref;

use serde::{Deserialize, Serialize};

/// 外发查询的最小上下文上限（SPEC 5.3.1-4：≤200 字）。
pub const URL_LIMIT: usize = 200;

/// 已脱敏的检索查询。
///
/// **不可伪造**：字段私有，且 [`SanitizedQuery::issue`] 仅在 crate 内可见。
/// 外部实现者必须通过 `Sanitizer` 端口的受控入口产出本类型（DESIGN 2.2.2-C）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SanitizedQuery {
    text: String,
    #[serde(default)]
    redactions: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SanitizedQueryError {
    #[error("脱敏查询为空")]
    Empty,
    #[error("脱敏查询长度为 {actual}，超出上限 {limit}")]
    TooLong { limit: usize, actual: usize },
}

impl SanitizedQuery {
    pub(crate) fn issue(text: &str, redactions: u16) -> Result<Self, SanitizedQueryError> {
        let trimmed = text.trim();
        let actual = trimmed.chars().count();
        if actual == 0 {
            return Err(SanitizedQueryError::Empty);
        }
        if actual > URL_LIMIT {
            return Err(SanitizedQueryError::TooLong {
                limit: URL_LIMIT,
                actual,
            });
        }
        Ok(Self {
            text: trimmed.to_owned(),
            redactions,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// 脱敏替换次数，用于审计与质量统计；不包含原文。
    pub fn redaction_count(&self) -> u16 {
        self.redactions
    }

    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }
}

impl TryFrom<String> for SanitizedQuery {
    type Error = SanitizedQueryError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::issue(&value, 0)
    }
}

impl From<SanitizedQuery> for String {
    fn from(value: SanitizedQuery) -> Self {
        value.text
    }
}

impl Deref for SanitizedQuery {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert_eq!(
            SanitizedQuery::issue("  ", 0),
            Err(SanitizedQueryError::Empty)
        );
    }

    #[test]
    fn enforces_limit() {
        let ok = "字".repeat(URL_LIMIT);
        assert!(SanitizedQuery::issue(&ok, 3).is_ok());
        let over = "字".repeat(URL_LIMIT + 1);
        assert!(matches!(
            SanitizedQuery::issue(&over, 0),
            Err(SanitizedQueryError::TooLong { .. })
        ));
    }

    #[test]
    fn redaction_count_is_preserved() {
        let query = SanitizedQuery::issue("模型上下文长度 是什么", 2).expect("合法");
        assert_eq!(query.redaction_count(), 2);
        assert_eq!(query.as_str(), "模型上下文长度 是什么");
    }
}
