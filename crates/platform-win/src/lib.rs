//! Windows 平台适配层。
//!
//! 非 Windows 目标下只保留空壳，保证 workspace 可交叉检查。

#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

pub const PLATFORM_NAME: &str = "windows";
pub mod meeting;
pub mod secure_store;
#[cfg(windows)]
pub mod wasapi;
