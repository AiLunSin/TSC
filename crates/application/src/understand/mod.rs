//! 理解层：把流式识别结果稳定化后再交给下游。

pub mod stable_prefix;

pub use stable_prefix::StablePrefix;
