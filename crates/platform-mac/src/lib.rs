//! macOS 平台适配层。
//!
//! 非 macOS 目标下只保留空壳，保证 workspace 可交叉检查。

#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

pub const PLATFORM_NAME: &str = "macos";
