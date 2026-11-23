//! 仓储共用的枚举/计数编解码。

use serde::{Deserialize, Serialize};

use crate::store::StoreError;

pub(crate) fn enum_to_str<T: Serialize>(value: &T) -> Result<String, StoreError> {
    match serde_json::to_value(value)
        .map_err(|error| StoreError::Serialization(error.to_string()))?
    {
        serde_json::Value::String(raw) => Ok(raw),
        _ => Err(StoreError::Serialization("枚举编码不是字符串".to_owned())),
    }
}

pub(crate) fn enum_from_str<T: for<'de> Deserialize<'de>>(raw: &str) -> Result<T, StoreError> {
    serde_json::from_value(serde_json::Value::String(raw.to_owned()))
        .map_err(|error| StoreError::Serialization(error.to_string()))
}

pub(crate) fn checked_usize(value: i64, field: &str) -> Result<usize, StoreError> {
    usize::try_from(value).map_err(|_| StoreError::Serialization(format!("{field} 计数溢出")))
}
