//! IPC 桥接层：跨进程传输的 DTO 与类型绑定。
//!
//! 约定（DESIGN 2.2.1）：配置类 DTO 保留未知字段透传；业务 DTO 一律
//! `#[serde(deny_unknown_fields)]`，避免协议漂移被静默吞掉。

pub mod dto;

/// IPC 协议版本。前端与后端不一致时拒绝建立会话（非配置类）。
pub const IPC_PROTOCOL_VERSION: u16 = 1;