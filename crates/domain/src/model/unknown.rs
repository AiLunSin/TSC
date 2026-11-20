//! 枚举未知值降级（SPEC 4.5-8）。
//!
//! 读取旧版本数据遇到未知枚举值时，保留原始值并产生告警，绝不丢弃整条记录。

use serde::{Deserialize, Serialize};

/// 承载无法识别的枚举原始值。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnknownValue {
    pub raw: String,
}

impl UnknownValue {
    pub fn new(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }
}

/// 降级结果：已知值或未知值（带原始字符串）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Degraded<T> {
    Known(T),
    Unknown { raw: String },
}

impl<T> Degraded<T> {
    pub fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    pub fn raw_or<'a>(&'a self, fallback: &'a str) -> &'a str {
        match self {
            Self::Known(_) => fallback,
            Self::Unknown { raw } => raw,
        }
    }
}

/// 一次降级事件，供告警与诊断包使用（不含原文）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DegradationWarning {
    pub field: String,
    pub raw: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_value_roundtrips_raw_text() {
        let degraded: Degraded<u8> =
            serde_json::from_str(r#"{"raw":"FUTURE_STATE"}"#).expect("解析");
        assert!(!degraded.is_known());
        assert_eq!(degraded.raw_or("fallback"), "FUTURE_STATE");

        let json = serde_json::to_string(&degraded).expect("序列化");
        assert!(json.contains("FUTURE_STATE"));
    }

    #[test]
    fn known_value_roundtrips() {
        let degraded = Degraded::Known(42_u8);
        let json = serde_json::to_string(&degraded).expect("序列化");
        let back: Degraded<u8> = serde_json::from_str(&json).expect("反序列化");
        assert_eq!(back, degraded);
    }
}
