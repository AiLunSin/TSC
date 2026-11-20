use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

/// 文本越界错误。构造期返回 `Err`，不 panic（SPEC 6.3-4、6.4-3）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field} 长度为 {actual}，超出上限 {limit}")]
pub struct TextBoundError {
    pub field: &'static str,
    pub limit: usize,
    pub actual: usize,
}

macro_rules! bounded_text {
    ($name:ident, $field:literal, $limit:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub const LIMIT: usize = $limit;

            /// 去掉首尾空白后校验字符数上限；空串视为非法。
            pub fn new(raw: impl Into<String>) -> Result<Self, TextBoundError> {
                let value = raw.into();
                let trimmed = value.trim();
                let actual = trimmed.chars().count();
                if actual == 0 {
                    return Err(TextBoundError {
                        field: $field,
                        limit: $limit,
                        actual,
                    });
                }
                if actual > $limit {
                    return Err(TextBoundError {
                        field: $field,
                        limit: $limit,
                        actual,
                    });
                }
                Ok(Self(trimmed.to_owned()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn char_count(&self) -> usize {
                self.0.chars().count()
            }
        }

        impl TryFrom<String> for $name {
            type Error = TextBoundError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

bounded_text!(ShortText, "要点文本", 60);
bounded_text!(TitleText, "标题", 40);
bounded_text!(ConclusionText, "一句话结论", 80);
bounded_text!(UtteranceText, "对话片段", 2000);

impl ShortText {
    /// 无法判定类型时的安全降级：截断到上限并保留可读前缀。
    pub fn truncated(raw: &str) -> Self {
        let prefix: String = raw.trim().chars().take(Self::LIMIT).collect();
        Self(if prefix.is_empty() {
            "（空要点）".to_owned()
        } else {
            prefix
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_after_trim() {
        assert!(ShortText::new("   ").is_err());
    }

    #[test]
    fn trims_and_counts_chars_not_bytes() {
        let text = ShortText::new("  你好世界  ").expect("合法");
        assert_eq!(text.as_str(), "你好世界");
        assert_eq!(text.char_count(), 4);
    }

    #[test]
    fn boundary_values() {
        let at_limit = "字".repeat(ShortText::LIMIT);
        assert!(ShortText::new(at_limit).is_ok());
        let over = "字".repeat(ShortText::LIMIT + 1);
        assert!(ShortText::new(over).is_err());
    }

    #[test]
    fn serde_roundtrip_keeps_validation() {
        let ok: ShortText = serde_json::from_str("\"有效要点\"").expect("合法");
        assert_eq!(ok.as_str(), "有效要点");
        let over = format!("\"{}\"", "字".repeat(61));
        assert!(serde_json::from_str::<ShortText>(&over).is_err());
    }

    #[test]
    fn truncated_never_exceeds_limit() {
        let long = "长".repeat(200);
        let short = ShortText::truncated(&long);
        assert_eq!(short.char_count(), ShortText::LIMIT);
    }
}
