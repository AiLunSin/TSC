use std::fmt;

use serde::{Deserialize, Serialize};

/// 标识符上限：128 字符，禁止空白。
const ID_LIMIT: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{label} 非法：{reason}")]
pub struct IdError {
    pub label: &'static str,
    pub reason: &'static str,
}

macro_rules! id_type {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(raw: impl Into<String>) -> Result<Self, IdError> {
                let value = raw.into();
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(IdError {
                        label: $label,
                        reason: "为空",
                    });
                }
                if trimmed.chars().count() > ID_LIMIT {
                    return Err(IdError {
                        label: $label,
                        reason: "超出 128 字符上限",
                    });
                }
                if trimmed.chars().any(char::is_whitespace) {
                    return Err(IdError {
                        label: $label,
                        reason: "含空白字符",
                    });
                }
                Ok(Self(trimmed.to_owned()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

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

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(SessionId, "会话 ID");
id_type!(UtteranceRef, "片段引用");
id_type!(InsightRef, "要点引用");
id_type!(CardRef, "提示卡引用");
id_type!(TraceId, "溯源链 ID");
id_type!(ProviderId, "检索源 ID");
id_type!(TaskId, "任务 ID");
id_type!(AuditId, "审计 ID");
id_type!(ReferenceRef, "来源引用");
id_type!(ActorRef, "操作者");
id_type!(ResourceRef, "操作对象");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_blank_and_whitespace() {
        assert!(SessionId::new("").is_err());
        assert!(SessionId::new("   ").is_err());
        assert!(SessionId::new("a b").is_err());
    }

    #[test]
    fn accepts_typical_ids() {
        assert_eq!(
            SessionId::new("sess-2026-11-20-001").map(|s| s.to_string()),
            Ok("sess-2026-11-20-001".to_owned())
        );
    }
}
