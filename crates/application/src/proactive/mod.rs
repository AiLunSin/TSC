//! 主动提示的反馈、缓存与断网补齐编排（SPEC 7.1~7.3、DESIGN 2.10）。

pub mod backfill;
pub mod cache;
pub mod degrade_queue;
pub mod feedback;
pub mod net_probe;
pub mod no_provider_mode;
pub mod on_demand;
pub mod on_demand_probe;
pub mod on_demand_runner;
pub mod outbound_guard;
pub mod outbox;
pub mod recovery;
