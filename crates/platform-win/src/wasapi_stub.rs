//! 非 Windows 目标：WASAPI 不可用的占位实现。
//!
//! 保持与 Windows 实现相同的公开接口，供桌面壳无条件引用。

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::enums::Platform;
use meeting_copilot_domain::port::capture::{
    AudioChunk, AudioSource, SourceHealth, SubtitleDescriptor,
};

/// 非 Windows 目标恒为不可用。
pub struct WasapiLoopbackSource {
    descriptor: SubtitleDescriptor,
}

impl WasapiLoopbackSource {
    pub fn new(platform: Platform) -> Self {
        Self {
            descriptor: SubtitleDescriptor {
                platform,
                version: "wasapi-unavailable".to_owned(),
                channel: "system_audio".to_owned(),
            },
        }
    }
}

impl AudioSource for WasapiLoopbackSource {
    fn descriptor(&self) -> &SubtitleDescriptor {
        &self.descriptor
    }

    fn open(&self, _device: Option<&str>) -> Result<(), CopilotError> {
        Err(CopilotError::AudioDeviceUnavailable {
            detail: "当前平台不支持 WASAPI 环回".to_owned(),
        })
    }

    fn next_chunk(&self) -> Result<Option<AudioChunk>, CopilotError> {
        Err(CopilotError::AudioDeviceUnavailable {
            detail: "当前平台不支持 WASAPI 环回".to_owned(),
        })
    }

    fn health(&self) -> SourceHealth {
        SourceHealth::Unavailable("当前平台不支持 WASAPI".to_owned())
    }
}

/// 系统音频通道状态：非 Windows 恒为未启用。
pub fn system_audio_status() -> &'static str {
    "off"
}
