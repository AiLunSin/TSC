//! 应用层：编排领域端口，不含平台 API 与 SQL 细节。

pub mod governance;

pub use governance::AuditGuard;
