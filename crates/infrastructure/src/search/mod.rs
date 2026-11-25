//! 检索源接入（SPEC 第 8 章、DESIGN 2.11）。
//!
//! 本模块只处理配置、注册表、健康与单源 HTTP JSON 适配器；
//! 不内置任何默认源、示例源或服务商常量。

pub mod health;
pub mod http_json;
pub mod profile_store;
pub mod registry;

pub use health::{
    classify_transport, tcp_reachable, test_connection, HealthMonitor, FAILURE_THRESHOLD,
};
pub use http_json::HttpJsonProvider;
pub use profile_store::{
    parse_https_endpoint, ProfileError, ProfileStore, ProviderEntry, SearchAdapter,
};
pub use registry::Registry;
