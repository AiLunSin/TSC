//! 应用层：编排领域端口，不含平台 API 与 SQL 细节。

pub mod capture;
pub mod cards;
pub mod citation;
pub mod gate;
pub mod governance;
pub mod mirror_clock;
pub mod proactive;
pub mod settings;
pub mod understand;

pub use governance::AuditGuard;
pub use understand::StablePrefix;
