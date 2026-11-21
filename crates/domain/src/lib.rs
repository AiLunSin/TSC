//! 领域层根模块。
//!
//! 依赖方向约束（DESIGN 2.1.2）：本 crate 不得依赖 `infrastructure`、
//! `platform-*` 或任何网络/存储实现。所有可变能力以端口 trait 形式暴露。

pub mod error;
pub mod merge;
pub mod model;
pub mod port;
pub mod vo;

pub use error::{CopilotError, DenyCode, ErrorCode};
