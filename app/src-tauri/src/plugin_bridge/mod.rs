//! 本地插件桥：给 DSH 插件（或其它本机程序）一个只走回环、要令牌的 JSON 入口。
//!
//! 边界：
//! - 只绑定 `127.0.0.1` 的临时端口，不监听局域网地址；
//! - 每次启动生成新令牌，写入应用配置目录的 `plugin-bridge.json`，退出即删；
//! - 拒绝带 `Origin` 的请求（浏览器发起的跨站请求拿不到令牌，也不给它机会）；
//! - 请求体上限 64 KiB，响应不落盘、不写正文日志；
//! - 桥只转发现有 IPC 能力，不新增出网路径：查义、检索、提醒出不出网，
//!   仍按用户在应用里的配置走。
//!
//! 这里不引用 Tauri 命令包装，直接调用 `ipc` 层的 `*_impl`，保持与桌面端同一套
//! 门控、脱敏与降级行为。

mod http;

use std::io;
use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::ipc::asr::AsrPipeline;
use crate::ipc::assist::{assist_impl, AssistState};
use crate::ipc::card_store::CardStore;
use crate::ipc::cards::{keyword_card_open_impl, keyword_cards_snapshot_impl};
use crate::ipc::dto::{
    AssistInputDto, AssistResultDto, CardDetailDto, ExplainInputDto, ExplainResultDto, IpcErrorDto,
    KeywordCardViewDto, OpenInputDto, SearchResponseDto, TranscriptLineDto,
};
use crate::ipc::explain::explain_impl;
use crate::ipc::search::SearchState;
use crate::ipc::state::AppState;
use crate::ipc::transcript::TranscriptLog;

/// 插件桥协议版本；不兼容改动时递增，插件据此给出可读提示。
pub const PROTOCOL_VERSION: u16 = 1;

/// 应用配置目录下的描述符文件名。
pub const DESCRIPTOR_FILE: &str = "plugin-bridge.json";

/// 启动时写入、供本机客户端发现端口与令牌。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeDescriptor {
    pub protocol: u16,
    pub port: u16,
    pub token: String,
    pub pid: u32,
    pub app_version: String,
    pub started_at_ms: u64,
}

/// 桥可调用的能力集合；测试里替换成假实现。
pub trait BridgeApi: Send + Sync + 'static {
    fn status(&self) -> Result<Value, IpcErrorDto>;
    fn explain(&self, text: &str, context: &str) -> Result<ExplainResultDto, IpcErrorDto>;
    fn cards(&self) -> Result<Vec<KeywordCardViewDto>, IpcErrorDto>;
    fn card_detail(&self, card_id: &str) -> Result<CardDetailDto, IpcErrorDto>;
    fn save_card(
        &self,
        word: &str,
        explanation: &str,
        sources: &[String],
    ) -> Result<usize, IpcErrorDto>;
    fn transcript(&self) -> Result<Vec<TranscriptLineDto>, IpcErrorDto>;
    fn search(&self, query: &str) -> Result<SearchResponseDto, IpcErrorDto>;
    fn assist(&self, focus: &str) -> Result<AssistResultDto, IpcErrorDto>;
    fn asr(&self, action: AsrAction) -> Result<Value, IpcErrorDto>;
}

/// 转写控制动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsrAction {
    Start,
    Stop,
    Status,
}

/// 真实实现：把请求转给桌面的既有运行态。
struct AppBridge {
    app: AppHandle,
    started_at_ms: u64,
}

impl BridgeApi for AppBridge {
    fn status(&self) -> Result<Value, IpcErrorDto> {
        let asr = self.app.state::<Arc<AsrPipeline>>();
        let state = self.app.state::<AppState>();
        let facts = state.view_facts.view_facts();
        Ok(json!({
            "protocol": PROTOCOL_VERSION,
            "appVersion": env!("CARGO_PKG_VERSION"),
            "pid": process::id(),
            "startedAtMs": self.started_at_ms,
            "nowMs": now_ms(),
            "asr": {
                "running": asr.is_running(),
                "displayText": asr.display_text(),
                "lastError": asr.last_error(),
                "autoCards": asr.auto_cards(),
            },
            "cards": state.cards.cards().len(),
            "privacy": {
                "screenShare": facts.screen_share,
                "emergencyMasked": facts.emergency_masked,
            },
        }))
    }

    fn explain(&self, text: &str, context: &str) -> Result<ExplainResultDto, IpcErrorDto> {
        let state = self.app.state::<AppState>();
        let transcripts = self.app.state::<Arc<TranscriptLog>>();
        explain_impl(
            state.inner(),
            transcripts.inner(),
            ExplainInputDto {
                text: text.to_owned(),
                context: context.to_owned(),
            },
        )
    }

    fn cards(&self) -> Result<Vec<KeywordCardViewDto>, IpcErrorDto> {
        let state = self.app.state::<AppState>();
        keyword_cards_snapshot_impl(state.inner())
    }

    fn card_detail(&self, card_id: &str) -> Result<CardDetailDto, IpcErrorDto> {
        let state = self.app.state::<AppState>();
        keyword_card_open_impl(
            state.inner(),
            OpenInputDto {
                card_id: card_id.to_owned(),
            },
        )
    }

    fn save_card(
        &self,
        word: &str,
        explanation: &str,
        sources: &[String],
    ) -> Result<usize, IpcErrorDto> {
        let cards = self.app.state::<Arc<CardStore>>();
        cards.ingest_explanation_with_sources(word, explanation, sources)
    }

    fn transcript(&self) -> Result<Vec<TranscriptLineDto>, IpcErrorDto> {
        let transcripts = self.app.state::<Arc<TranscriptLog>>();
        Ok(transcripts.snapshot())
    }

    fn search(&self, query: &str) -> Result<SearchResponseDto, IpcErrorDto> {
        let search = self.app.state::<Arc<SearchState>>();
        search.query(query)
    }

    fn assist(&self, focus: &str) -> Result<AssistResultDto, IpcErrorDto> {
        let state = self.app.state::<AppState>();
        let transcripts = self.app.state::<Arc<TranscriptLog>>();
        let assist = self.app.state::<Arc<AssistState>>();
        assist_impl(
            state.inner(),
            transcripts.inner(),
            &assist.store,
            AssistInputDto {
                focus: focus.to_owned(),
            },
        )
    }

    fn asr(&self, action: AsrAction) -> Result<Value, IpcErrorDto> {
        let pipeline = self.app.state::<Arc<AsrPipeline>>();
        match action {
            AsrAction::Start => {
                pipeline.inner().start()?;
                Ok(json!({ "running": pipeline.is_running() }))
            }
            AsrAction::Stop => {
                pipeline.stop();
                Ok(json!({ "running": pipeline.is_running() }))
            }
            AsrAction::Status => Ok(json!({
                "running": pipeline.is_running(),
                "displayText": pipeline.display_text(),
                "lastError": pipeline.last_error(),
                "diagnostics": pipeline.diagnostics(),
                "autoCards": pipeline.auto_cards(),
            })),
        }
    }
}

/// 启动桥；失败只记日志，不影响桌面应用本身。
pub fn start(app: &AppHandle) {
    let descriptor_path = match descriptor_location(app) {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(target: "plugin_bridge", %error, "定位不到描述符路径，插件桥不启动");
            return;
        }
    };
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
        Ok(listener) => listener,
        Err(error) => {
            tracing::warn!(target: "plugin_bridge", %error, "回环端口绑定失败，插件桥不启动");
            return;
        }
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(error) => {
            tracing::warn!(target: "plugin_bridge", %error, "读不到本地端口，插件桥不启动");
            return;
        }
    };
    let descriptor = BridgeDescriptor {
        protocol: PROTOCOL_VERSION,
        port,
        token: new_token(),
        pid: process::id(),
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        started_at_ms: now_ms(),
    };
    if let Err(error) = write_descriptor(&descriptor_path, &descriptor) {
        tracing::warn!(target: "plugin_bridge", %error, path = %descriptor_path.display(), "描述符写入失败，插件桥不启动");
        return;
    }
    let started_at_ms = descriptor.started_at_ms;
    let token = descriptor.token;
    let api: Arc<dyn BridgeApi> = Arc::new(AppBridge {
        app: app.clone(),
        started_at_ms,
    });
    let spawned = thread::Builder::new()
        .name("plugin-bridge".to_owned())
        .spawn(move || http::serve(listener, token, port, api));
    match spawned {
        Ok(_) => tracing::info!(target: "plugin_bridge", port, "本地插件桥已启动"),
        Err(error) => {
            tracing::warn!(target: "plugin_bridge", %error, "插件桥线程启动失败");
            remove_descriptor_owned(&descriptor_path);
        }
    }
}

/// 退出时删掉本进程写的描述符；别的实例写的保持不动。
pub fn stop(app: &AppHandle) {
    let Ok(path) = descriptor_location(app) else {
        return;
    };
    remove_descriptor_owned(&path);
}

fn remove_descriptor_owned(path: &Path) {
    let owned = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<BridgeDescriptor>(&raw).ok())
        .is_some_and(|descriptor| descriptor.pid == process::id());
    if owned {
        let _ = std::fs::remove_file(path);
    }
}

/// 描述符位置：默认在应用配置目录；`MEETING_COPILOT_BRIDGE_FILE` 可覆盖，
/// 供无窗口测试或便携运行把描述符放进可写目录（插件读同一个环境变量）。
fn descriptor_location(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("MEETING_COPILOT_BRIDGE_FILE") {
        return Ok(PathBuf::from(path));
    }
    app.path()
        .app_config_dir()
        .map(|dir| descriptor_path(&dir))
        .map_err(|error| error.to_string())
}

fn write_descriptor(path: &Path, descriptor: &BridgeDescriptor) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp_path = path.with_extension("json.tmp");
    let body = serde_json::to_vec_pretty(descriptor)?;
    std::fs::write(&temp_path, body)?;
    // Windows 上 rename 不覆盖已存在文件：先删旧描述符，旧实例的桥即使还在，
    // 插件也会因为端口/令牌对不上而报"请重启会议副驾"。
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(&temp_path, path)
}

/// 令牌：SHA-256 吸收系统随机哈希种子、时间、进程号与栈地址。
///
/// 不需要密码学级 CSPRNG：它只证明"读到了本机用户配置目录里的文件"，
/// 攻击者真要拿到令牌，得先能读同一用户的文件。
fn new_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    hasher.update(now.to_le_bytes());
    hasher.update(process::id().to_le_bytes());
    let local = 0u8;
    hasher.update((&local as *const u8 as usize).to_le_bytes());
    let random = RandomState::new();
    for round in 0u64..4 {
        let mut state = random.build_hasher();
        state.write_u64(round);
        state.write_u128(now);
        hasher.update(state.finish().to_le_bytes());
    }
    let digest = hasher.finalize();
    let mut token = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(token, "{byte:02x}");
    }
    token
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

/// 描述符路径；插件端按同样规则找。
pub fn descriptor_path(config_dir: &Path) -> PathBuf {
    config_dir.join(DESCRIPTOR_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        // 用 target/ 下的目录而不是系统临时目录：CI 与自动化里 TEMP 可能不可写，
        // 这里只需要一个本次运行独占、能放描述符的位置。
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-tmp");
        let dir = base.join(format!(
            "plugin-bridge-{tag}-{}-{}",
            process::id(),
            now_ms()
        ));
        std::fs::create_dir_all(&dir).expect("创建临时目录");
        dir
    }

    #[test]
    fn descriptor_roundtrip() {
        let dir = temp_dir("roundtrip");
        let descriptor = BridgeDescriptor {
            protocol: PROTOCOL_VERSION,
            port: 4567,
            token: "abc".to_owned(),
            pid: process::id(),
            app_version: "0.0.0".to_owned(),
            started_at_ms: 1,
        };
        write_descriptor(&descriptor_path(&dir), &descriptor).expect("写入");
        let raw = std::fs::read_to_string(descriptor_path(&dir)).expect("读取");
        let parsed: BridgeDescriptor = serde_json::from_str(&raw).expect("解析");
        assert_eq!(parsed, descriptor);
        assert!(!dir.join("plugin-bridge.json.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn token_is_long_hex_and_changes() {
        let first = new_token();
        let second = new_token();
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn write_descriptor_replaces_stale_file() {
        let dir = temp_dir("replace");
        let mut descriptor = BridgeDescriptor {
            protocol: PROTOCOL_VERSION,
            port: 1,
            token: "one".to_owned(),
            pid: 1,
            app_version: "0.0.0".to_owned(),
            started_at_ms: 1,
        };
        write_descriptor(&descriptor_path(&dir), &descriptor).expect("第一次写入");
        descriptor.token = "two".to_owned();
        write_descriptor(&descriptor_path(&dir), &descriptor).expect("第二次写入");
        let raw = std::fs::read_to_string(descriptor_path(&dir)).expect("读取");
        assert!(raw.contains("\"two\""));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
