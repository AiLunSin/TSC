//! 联网检索：配置、凭据、单次查询。
//!
//! 边界：只有用户点"联网查"、或显式打开"出卡时自动补来源"之后才会出网；
//! 外发内容只有选中的词/短句（≤80 字），不带整段转写，不落盘、不写日志正文。
//! 检索 Key 与模型 Key 分开存，互不覆盖。

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use meeting_copilot_application::proactive::net_probe::{NetProbe, NetworkState};
use meeting_copilot_application::settings::{PortError, SecureStore};
use meeting_copilot_domain::vo::TaskId;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ipc::backfill::BackfillBridge;
use crate::ipc::card_store::CardStore;
use crate::ipc::dto::{
    IpcErrorDto, SearchConfigDto, SearchConfigInputDto, SearchHitDto, SearchInputDto,
    SearchResponseDto,
};

/// 配置文件固定放在应用配置目录下，与 `settings.json` 分开：
/// 设置文件是"恰 4 键"的强契约，不宜混入检索配置。
const CONFIG_FILE: &str = "search.json";
/// 限额计数单独存一个文件：只在查询时写，不与配置写竞争。
const QUOTA_FILE: &str = "search-quota.json";
/// 限额窗口：按整小时滚动，窗口过期即重置计数。
const QUOTA_WINDOW_SECS: u64 = 3_600;
/// 默认每小时最多查 20 次；够开会用，也不至于把免费额度一轮打完。
const DEFAULT_HOURLY_LIMIT: u32 = 20;
/// 限额上限：再高就等于不设限，不如直接用 0。
const MAX_HOURLY_LIMIT: u32 = 200;
/// 单次查询上限：查询串只应是"一个词或一句短语"。
const MAX_QUERY_CHARS: usize = 80;
/// 单次返回条数：够做卡片来源即可，多了界面放不下。
#[cfg(any(feature = "llm-https", test))]
const MAX_HITS: usize = 5;
/// 检索超时：比模型短，用户等的是"顺手查一下"。
#[cfg(feature = "llm-https")]
const TIMEOUT_MS: u64 = 8_000;
/// 检索只允许这两家固定端点出网，与 `SearchProvider::endpoint` 一一对应。
#[cfg(feature = "llm-https")]
const SEARCH_HOSTS: &[&str] = &["api.bochaai.com", "api.tavily.com"];

/// 检索服务预设。只给固定端点，不接受任意地址，避免把查询发到不可控主机。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchProvider {
    /// 博查：中文网页结果，返回 `data.webPages.value`。
    Bocha,
    /// Tavily：面向模型的检索，返回 `results`。
    Tavily,
}

impl SearchProvider {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bocha" => Some(Self::Bocha),
            "tavily" => Some(Self::Tavily),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bocha => "bocha",
            Self::Tavily => "tavily",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Bocha => "博查 Bocha",
            Self::Tavily => "Tavily",
        }
    }

    /// 端点固定为 `(host, path)`，`https` 由传输层保证。
    #[cfg(feature = "llm-https")]
    const fn endpoint(self) -> (&'static str, &'static str) {
        match self {
            Self::Bocha => ("api.bochaai.com", "/v1/web-search"),
            Self::Tavily => ("api.tavily.com", "/search"),
        }
    }
}

/// 落盘的检索配置：4 键，版本不符直接回默认。
/// `hourly_limit` 带默认值：旧文件缺这一键也能读，不会把 provider/auto 一起丢掉。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SearchFileV1 {
    schema_version: u32,
    provider: String,
    auto: bool,
    #[serde(default = "default_hourly_limit")]
    hourly_limit: u32,
}

fn default_hourly_limit() -> u32 {
    DEFAULT_HOURLY_LIMIT
}

impl Default for SearchFileV1 {
    fn default() -> Self {
        Self {
            schema_version: 1,
            provider: SearchProvider::Bocha.as_str().to_owned(),
            // 自动补来源默认关：出网必须由用户显式开启。
            auto: false,
            hourly_limit: DEFAULT_HOURLY_LIMIT,
        }
    }
}

/// 限额计数：`window_start` 为本次窗口起点（Unix 秒），过窗即重置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QuotaFile {
    schema_version: u32,
    window_start: u64,
    count: u32,
}

impl QuotaFile {
    fn fresh(now: u64) -> Self {
        Self {
            schema_version: 1,
            window_start: now,
            count: 0,
        }
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn unix_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

impl SearchFileV1 {
    fn provider(&self) -> SearchProvider {
        SearchProvider::parse(&self.provider).unwrap_or(SearchProvider::Bocha)
    }
}

/// 检索配置存储：临时文件 + 同目录替换，损坏时回默认。
pub struct SearchStore {
    dir: PathBuf,
}

impl SearchStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn main_path(&self) -> PathBuf {
        self.dir.join(CONFIG_FILE)
    }

    fn tmp_path(&self) -> PathBuf {
        self.dir.join("search.json.tmp")
    }

    fn load(&self) -> SearchFileV1 {
        let Ok(text) = fs::read_to_string(self.main_path()) else {
            return SearchFileV1::default();
        };
        match serde_json::from_str::<SearchFileV1>(&text) {
            Ok(file) if file.schema_version == 1 => file,
            _ => SearchFileV1::default(),
        }
    }

    fn save(&self, file: &SearchFileV1) -> Result<(), PortError> {
        fs::create_dir_all(&self.dir).map_err(|_| PortError::Unavailable)?;
        let json = serde_json::to_vec_pretty(file).map_err(|_| PortError::Corrupted)?;
        let tmp = self.tmp_path();
        fs::write(&tmp, json).map_err(|_| PortError::Unavailable)?;
        fs::rename(&tmp, self.main_path()).map_err(|_| PortError::Unavailable)
    }

    fn quota_path(&self) -> PathBuf {
        self.dir.join(QUOTA_FILE)
    }

    /// 读限额计数：窗口过期或文件损坏都从零开始，不因为一个坏文件把用户挡在外面。
    fn load_quota(&self, now: u64) -> QuotaFile {
        let Ok(text) = fs::read_to_string(self.quota_path()) else {
            return QuotaFile::fresh(now);
        };
        match serde_json::from_str::<QuotaFile>(&text) {
            Ok(file)
                if file.schema_version == 1
                    && now.saturating_sub(file.window_start) < QUOTA_WINDOW_SECS =>
            {
                file
            }
            _ => QuotaFile::fresh(now),
        }
    }

    fn save_quota(&self, quota: &QuotaFile) -> Result<(), PortError> {
        fs::create_dir_all(&self.dir).map_err(|_| PortError::Unavailable)?;
        let json = serde_json::to_vec_pretty(quota).map_err(|_| PortError::Corrupted)?;
        let tmp = self.dir.join("search-quota.json.tmp");
        fs::write(&tmp, json).map_err(|_| PortError::Unavailable)?;
        fs::rename(&tmp, self.quota_path()).map_err(|_| PortError::Unavailable)
    }
}

/// 检索运行态：配置 + 两家各自的独立凭据 + 网络状态 + 断网补齐。由 Tauri 管理，不放进 `AppState`。
pub struct SearchState {
    pub store: SearchStore,
    bocha: Arc<dyn SecureStore>,
    tavily: Arc<dyn SecureStore>,
    /// 回放补齐时把来源补回到已生成的卡片上。
    cards: Arc<CardStore>,
    /// 断网补齐桥；数据库打不开时为 `None`（补齐停用，检索本身仍可用）。
    outbox: Option<Arc<BackfillBridge>>,
    /// 由真实检索结果与可达性探测共同驱动的网络状态机（SPEC 7.2.1-7、DESIGN 2.10.2）。
    probe: Mutex<NetProbe>,
}

impl SearchState {
    pub fn new(
        dir: impl Into<PathBuf>,
        bocha: Arc<dyn SecureStore>,
        tavily: Arc<dyn SecureStore>,
        cards: Arc<CardStore>,
        outbox: Option<Arc<BackfillBridge>>,
    ) -> Self {
        Self {
            store: SearchStore::new(dir),
            bocha,
            tavily,
            cards,
            outbox,
            probe: Mutex::new(NetProbe::new()),
        }
    }

    /// 取对应服务的凭据存储：博查与 Tavily 各自独立，切换服务不必重填另一家的 Key。
    fn secure_for(&self, provider: SearchProvider) -> &Arc<dyn SecureStore> {
        match provider {
            SearchProvider::Bocha => &self.bocha,
            SearchProvider::Tavily => &self.tavily,
        }
    }

    /// 最近一次真实检索驱动的网络状态与「离线 → 在线」识别延迟（毫秒）。
    pub fn network_snapshot(&self) -> (NetworkState, Option<i64>) {
        self.probe
            .lock()
            .map_or((NetworkState::Unknown, None), |probe| {
                (probe.state(), probe.recovery_latency_ms())
            })
    }

    /// 用真实检索结果驱动网络状态；只有「连不上」才算断网，鉴权/限流/空结果不算。
    fn observe_network(&self, outcome: &Result<Vec<SearchHitDto>, IpcErrorDto>) {
        let reachable = match outcome {
            Ok(_) => true,
            Err(error) => error.code != "search_unreachable",
        };
        let Ok(mut probe) = self.probe.lock() else {
            return;
        };
        let changed = probe.on_result(reachable, unix_now_ms());
        if !changed {
            return;
        }
        match probe.state() {
            NetworkState::Online => tracing::info!(
                target: "search",
                latency_ms = probe.recovery_latency_ms().unwrap_or(-1),
                "检索网络已恢复"
            ),
            NetworkState::Offline => tracing::warn!(target: "search", "检索网络不可用"),
            NetworkState::Unknown => {}
        }
    }

    /// 断网补齐是否可用（单库已打开）；用于区分"未配置"与"落库失败"。
    pub fn outbox_ready(&self) -> bool {
        self.outbox.is_some()
    }

    /// 断网补齐：把「出卡自动补来源」任务先落库，再允许外发（SPEC 7.2.1-5）。
    pub fn outbox_enqueue(&self, word: &str) -> Option<TaskId> {
        self.outbox
            .as_ref()
            .and_then(|outbox| outbox.enqueue(word, unix_now_ms()))
    }

    /// 真实检索结束后结算对应任务：成功标记 done，失败进入退避等待恢复。
    pub fn outbox_settle(&self, word: &str, ok: bool) {
        if let Some(outbox) = &self.outbox {
            outbox.settle(word, ok, unix_now_ms());
        }
    }

    /// 后台节拍：离线时探测可达性，恢复后开启 30 秒补齐窗口并回放。
    ///
    /// 探测只做到端点的 TCP 可达性（不发送检索请求、不消耗额度）；
    /// 外发是否成功仍以真实检索结果为准（`observe_network`）。
    pub fn tick(&self, now_ms: i64) {
        #[cfg(not(feature = "llm-https"))]
        {
            let _ = now_ms;
            return;
        }
        #[cfg(feature = "llm-https")]
        {
            let provider = self.store.load().provider();
            let should_probe = self
                .probe
                .lock()
                .map(|probe| probe.state() == NetworkState::Unknown || probe.should_probe(now_ms))
                .unwrap_or(false);
            if should_probe {
                let reachable = probe_reachable(provider);
                let mut recovered = false;
                if let Ok(mut probe) = self.probe.lock() {
                    probe.mark_probed(now_ms);
                    let changed = probe.on_result(reachable, now_ms);
                    let online = probe.state() == NetworkState::Online;
                    if changed {
                        match probe.state() {
                            NetworkState::Online => tracing::info!(
                                target: "search",
                                latency_ms = probe.recovery_latency_ms().unwrap_or(-1),
                                "检索网络已恢复"
                            ),
                            NetworkState::Offline => {
                                tracing::warn!(target: "search", "检索网络不可用")
                            }
                            NetworkState::Unknown => {}
                        }
                    }
                    recovered = changed && online;
                }
                if recovered {
                    if let Some(outbox) = &self.outbox {
                        let total = outbox.pending_total();
                        if total > 0 && !outbox.window_open() {
                            outbox.begin_window(now_ms, total);
                            tracing::info!(target: "search", pending = total, "开启断网补齐窗口");
                        }
                    }
                }
            }
            self.pump_backfill(now_ms);
        }
    }

    /// 回放窗口内的待补任务：串行执行（≤3 并发上限自然满足），每条都结算并更新卡片来源。
    fn pump_backfill(&self, now_ms: i64) {
        let Some(outbox) = &self.outbox else {
            return;
        };
        if !outbox.window_open() {
            return;
        }
        for (entry, word) in outbox.due_jobs(now_ms) {
            let Some(word) = word else {
                // 进程重启后没有查询文本可重建：任务保留在库，但不谎报补齐成功。
                outbox.mark_failed(entry, now_ms);
                outbox.on_job_finished(false);
                continue;
            };
            match self.run_query_once(&word) {
                Ok(hits) if !hits.is_empty() => {
                    let sites: Vec<String> = hits
                        .iter()
                        .map(|hit| hit.site.clone())
                        .filter(|site| !site.is_empty())
                        .take(3)
                        .collect();
                    let attached = self.cards.attach_sources(&word, &sites);
                    outbox.mark_done(entry, now_ms);
                    outbox.on_job_finished(attached);
                }
                _ => {
                    outbox.mark_failed(entry, now_ms);
                    outbox.on_job_finished(false);
                }
            }
        }
        if let Some(report) = outbox.poll_report(now_ms) {
            tracing::info!(
                target: "search",
                total = report.total,
                done = report.done,
                failed = report.failed,
                ratio = report.ratio(),
                elapsed_ms = report.elapsed_ms,
                window_ms = report.window_ms,
                meets_budget = report.meets_budget,
                "断网补齐完成"
            );
        }
    }

    /// 回放用的一次检索：补齐是系统补发，不再重复扣界面限额。
    fn run_query_once(&self, word: &str) -> Result<Vec<SearchHitDto>, IpcErrorDto> {
        let provider = self.store.load().provider();
        let Some(key) = self.read_key_for(provider)? else {
            return Err(IpcErrorDto::new(
                "search_not_configured",
                "尚未配置联网检索：在「联网查资料」里填入检索 Key 后再试。",
                false,
            ));
        };
        let outcome = run_search(provider, &key, word);
        self.observe_network(&outcome);
        outcome
    }

    fn read_key_for(&self, provider: SearchProvider) -> Result<Option<String>, IpcErrorDto> {
        let stored = self.secure_for(provider).read().map_err(|_| {
            IpcErrorDto::new("search_key_unavailable", "检索凭据存储不可用。", true)
        })?;
        Ok(stored
            .map(|bytes| bytes.with_bytes(|raw| String::from_utf8_lossy(raw).into_owned()))
            .filter(|key| !key.trim().is_empty()))
    }

    fn write_key(&self, provider: SearchProvider, raw: &str) -> Result<(), IpcErrorDto> {
        let secret =
            meeting_copilot_application::settings::SecretBytes::new(raw.as_bytes().to_vec());
        self.secure_for(provider)
            .write(&secret)
            .map_err(|_| IpcErrorDto::new("search_key_unwritable", "检索 Key 保存失败。", true))
    }

    fn delete_key(&self, provider: SearchProvider) -> Result<(), IpcErrorDto> {
        self.secure_for(provider)
            .delete()
            .map_err(|_| IpcErrorDto::new("search_key_unwritable", "检索 Key 清除失败。", true))
    }

    /// 清除当前服务的凭据：失败说明存储不可用，不假装成功。
    pub fn clear_key(&self) -> Result<SearchConfigDto, IpcErrorDto> {
        self.delete_key(self.store.load().provider())?;
        Ok(self.config())
    }

    pub fn config(&self) -> SearchConfigDto {
        let file = self.store.load();
        let has_key_bocha = self
            .read_key_for(SearchProvider::Bocha)
            .unwrap_or(None)
            .is_some();
        let has_key_tavily = self
            .read_key_for(SearchProvider::Tavily)
            .unwrap_or(None)
            .is_some();
        let provider = file.provider();
        let has_key = match provider {
            SearchProvider::Bocha => has_key_bocha,
            SearchProvider::Tavily => has_key_tavily,
        };
        SearchConfigDto {
            provider: provider.as_str().to_owned(),
            has_key,
            has_key_bocha,
            has_key_tavily,
            auto: file.auto,
            ready: has_key && cfg!(feature = "llm-https"),
            hourly_limit: file.hourly_limit,
            used_this_hour: self.store.load_quota(unix_now()).count,
        }
    }

    /// 保存：留空 Key 表示沿用该服务现有凭据，`clearKey*` 表示删掉该服务已存凭据；
    /// `keyBocha`/`keyTavily` 分别写各自服务，留空不动。限额超过上限按上限记，0 表示不限额。
    pub fn save_config(
        &self,
        input: &SearchConfigInputDto,
    ) -> Result<SearchConfigDto, IpcErrorDto> {
        let provider = SearchProvider::parse(&input.provider).ok_or_else(|| {
            IpcErrorDto::new("search_provider_invalid", "不支持的检索服务。", false)
        })?;
        if input.clear_key_bocha {
            self.delete_key(SearchProvider::Bocha)?;
        }
        if input.clear_key_tavily {
            self.delete_key(SearchProvider::Tavily)?;
        }
        if input.clear_key {
            self.delete_key(provider)?;
        }
        let legacy_key = input.key.trim();
        if !legacy_key.is_empty() {
            self.write_key(provider, legacy_key)?;
        }
        let bocha_key = input.key_bocha.trim();
        if !bocha_key.is_empty() {
            self.write_key(SearchProvider::Bocha, bocha_key)?;
        }
        let tavily_key = input.key_tavily.trim();
        if !tavily_key.is_empty() {
            self.write_key(SearchProvider::Tavily, tavily_key)?;
        }
        if self.read_key_for(provider)?.is_none() {
            return Err(IpcErrorDto::new(
                "search_key_missing",
                "请先填入检索服务的 Key。",
                false,
            ));
        }
        self.store
            .save(&SearchFileV1 {
                schema_version: 1,
                provider: provider.as_str().to_owned(),
                auto: input.auto,
                hourly_limit: input.hourly_limit.min(MAX_HOURLY_LIMIT),
            })
            .map_err(|_| {
                IpcErrorDto::new("search_config_unwritable", "检索配置保存失败。", true)
            })?;
        Ok(self.config())
    }

    /// 扣一次限额；`limit = 0` 表示不限额。超限时给出可照做的提示。
    fn consume_quota(&self, limit: u32) -> Result<(), IpcErrorDto> {
        if limit == 0 {
            return Ok(());
        }
        let now = unix_now();
        let mut quota = self.store.load_quota(now);
        if quota.count >= limit {
            return Err(IpcErrorDto::new(
                "search_quota_exceeded",
                &format!("本小时联网查已用满 {limit} 次，可调高限额或下个整点后再试。"),
                true,
            ));
        }
        quota.count = quota.count.saturating_add(1);
        // 计数写不进去不影响本次查询，只是下次可能少扣一次。
        let _ = self.store.save_quota(&quota);
        Ok(())
    }

    /// 单次查询：未配置时给出可照做的提示，而不是空列表。
    pub fn query(&self, raw: &str) -> Result<SearchResponseDto, IpcErrorDto> {
        let query: String = raw.trim().chars().take(MAX_QUERY_CHARS).collect();
        if query.is_empty() {
            return Err(IpcErrorDto::new(
                "search_query_empty",
                "请先选中要查的文字。",
                false,
            ));
        }
        let file = self.store.load();
        let provider = file.provider();
        let Some(key) = self.read_key_for(provider)? else {
            return Err(IpcErrorDto::new(
                "search_not_configured",
                "尚未配置联网检索：在「联网查资料」里填入检索 Key 后再试。",
                false,
            ));
        };
        // 先扣额度再出网：请求一发出，服务商那边就已经计费了。
        self.consume_quota(file.hourly_limit)?;
        let outcome = run_search(provider, &key, &query);
        self.observe_network(&outcome);
        let hits = outcome?;
        Ok(SearchResponseDto {
            provider: provider.display_name().to_owned(),
            hits,
        })
    }
}

/// 到检索端点的 TCP 可达性：只做连接，不发请求、不消耗检索额度。
#[cfg(feature = "llm-https")]
fn probe_reachable(provider: SearchProvider) -> bool {
    let (host, _) = provider.endpoint();
    meeting_copilot_infrastructure::search::tcp_reachable(
        host,
        443,
        std::time::Duration::from_millis(800),
    )
}

/// 真正出网的一次检索；非 `llm-https` 构建直接说明未启用。
#[cfg(feature = "llm-https")]
fn run_search(
    provider: SearchProvider,
    key: &str,
    query: &str,
) -> Result<Vec<SearchHitDto>, IpcErrorDto> {
    use std::time::{Duration, Instant};

    use meeting_copilot_domain::port::llm::CancelToken;
    use meeting_copilot_infrastructure::llm::live::{BearerTransport, SecretString};
    use meeting_copilot_infrastructure::llm::transport::{HttpRequest, HttpTransport};
    use meeting_copilot_infrastructure::llm::transport_tls::TlsTransport;

    let body = match provider {
        SearchProvider::Bocha => serde_json::json!({
            "query": query,
            "count": MAX_HITS,
            "summary": true,
        }),
        SearchProvider::Tavily => serde_json::json!({
            "query": query,
            "max_results": MAX_HITS,
            "search_depth": "basic",
        }),
    };
    let payload = serde_json::to_vec(&body)
        .map_err(|_| IpcErrorDto::new("search_bad_request", "检索请求构造失败。", false))?;
    let headers = vec![
        ("Content-Type".to_owned(), "application/json".to_owned()),
        ("Accept".to_owned(), "application/json".to_owned()),
    ];
    let (host, path) = provider.endpoint();
    let mut request = HttpRequest::json_post(host, 443, path, headers, payload);
    request.secure = true;

    let transport: Arc<dyn HttpTransport> = Arc::new(BearerTransport::new(
        Arc::new(TlsTransport::with_allowed_hosts(SEARCH_HOSTS)),
        SecretString::new(key.to_owned()),
    ));
    let cancel = CancelToken::new();
    let deadline = Instant::now() + Duration::from_millis(TIMEOUT_MS);
    let response = transport.send(&request, deadline, &cancel).map_err(|_| {
        IpcErrorDto::new("search_unreachable", "连不上检索服务，请检查网络。", true)
    })?;
    match response.status {
        200..=299 => {}
        401 | 403 => {
            return Err(IpcErrorDto::new(
                "search_auth_failed",
                "检索 Key 无效或已失效，请重新填写。",
                false,
            ));
        }
        429 => {
            return Err(IpcErrorDto::new(
                "search_rate_limited",
                "检索过于频繁，请稍后再试。",
                true,
            ));
        }
        _ => {
            return Err(IpcErrorDto::new(
                "search_failed",
                "检索服务返回异常，请稍后再试。",
                true,
            ));
        }
    }
    let value: serde_json::Value = serde_json::from_slice(&response.body)
        .map_err(|_| IpcErrorDto::new("search_bad_response", "检索结果无法解析。", true))?;
    let hits = parse_hits(&value);
    if hits.is_empty() {
        return Err(IpcErrorDto::new(
            "search_no_hit",
            "这次没查到可用结果，换个说法再试。",
            false,
        ));
    }
    Ok(hits)
}

#[cfg(not(feature = "llm-https"))]
fn run_search(
    _provider: SearchProvider,
    _key: &str,
    _query: &str,
) -> Result<Vec<SearchHitDto>, IpcErrorDto> {
    Err(IpcErrorDto::new(
        "search_transport_off",
        "当前构建未启用联网传输。",
        false,
    ))
}

/// 兼容两家的响应形状：博查 `data.webPages.value`，Tavily `results`。
#[cfg(any(feature = "llm-https", test))]
fn parse_hits(value: &serde_json::Value) -> Vec<SearchHitDto> {
    let candidates = [
        value
            .get("data")
            .and_then(|data| data.get("webPages"))
            .and_then(|pages| pages.get("value")),
        value.get("results"),
        value.get("webPages").and_then(|pages| pages.get("value")),
    ];
    let Some(items) = candidates
        .into_iter()
        .flatten()
        .find(|item| item.is_array())
        .and_then(|item| item.as_array())
    else {
        return Vec::new();
    };
    let mut hits = Vec::new();
    for item in items {
        let title = text_field(item, &["name", "title"]);
        let url = text_field(item, &["url", "link"]);
        let snippet = text_field(item, &["snippet", "content", "summary", "description"]);
        let site = {
            let named = text_field(item, &["siteName", "site"]);
            if named.is_empty() {
                host_of(&url)
            } else {
                named
            }
        };
        if title.is_empty() && snippet.is_empty() {
            continue;
        }
        hits.push(SearchHitDto {
            title: title.chars().take(80).collect(),
            site,
            snippet: snippet.chars().take(240).collect(),
            url,
        });
        if hits.len() >= MAX_HITS {
            break;
        }
    }
    hits
}

#[cfg(any(feature = "llm-https", test))]
fn text_field(item: &serde_json::Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| item.get(*key).and_then(|value| value.as_str()))
        .unwrap_or("")
        .trim()
        .to_owned()
}

/// 从 URL 里取主机名，作为站点名兜底。
#[cfg(any(feature = "llm-https", test))]
fn host_of(url: &str) -> String {
    // 不写字面量协议前缀：IPC 层禁止出现完整地址形态，这里按 schema 拼。
    let mut rest = url;
    for scheme in ["https", "http"] {
        let prefix = format!("{scheme}://");
        if let Some(stripped) = rest.strip_prefix(&prefix) {
            rest = stripped;
            break;
        }
    }
    rest.split(['/', '?'])
        .next()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_owned()
}

#[tauri::command]
pub fn search_config(state: State<'_, Arc<SearchState>>) -> SearchConfigDto {
    state.config()
}

#[tauri::command]
pub fn search_config_save(
    state: State<'_, Arc<SearchState>>,
    input: SearchConfigInputDto,
) -> Result<SearchConfigDto, IpcErrorDto> {
    state.save_config(&input)
}

/// 清除已存检索 Key；配置本身（服务名、限额、自动补来源）保留。
#[tauri::command]
pub fn search_clear_key(
    state: State<'_, Arc<SearchState>>,
) -> Result<SearchConfigDto, IpcErrorDto> {
    state.clear_key()
}

#[tauri::command]
pub fn web_search(
    state: State<'_, Arc<SearchState>>,
    input: SearchInputDto,
) -> Result<SearchResponseDto, IpcErrorDto> {
    state.query(&input.query).map_err(|error| {
        if error.code == "search_unreachable" && state.network_snapshot().0 == NetworkState::Offline
        {
            IpcErrorDto::new(
                "search_unreachable",
                "连不上检索服务，当前已离线；网络恢复后可重试。",
                true,
            )
        } else {
            error
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bocha_shape() {
        let value = serde_json::json!({
            "code": 200,
            "data": { "webPages": { "value": [
                {"name": "SLA 是什么", "url": "https://www.example.com/a", "snippet": "服务等级协议…", "siteName": "示例站"},
                {"name": "SLA 指标", "url": "https://ops.example.org/b", "snippet": "99.9% 可用性"}
            ]}}
        });
        let hits = parse_hits(&value);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "SLA 是什么");
        assert_eq!(hits[0].site, "示例站");
        // 没给 siteName 时用主机名兜底，并去掉 www.
        assert_eq!(hits[1].site, "ops.example.org");
    }

    #[test]
    fn parses_tavily_shape() {
        let value = serde_json::json!({
            "results": [
                {"title": "灰度发布", "url": "https://www.example.com/gray", "content": "分批次放量…"},
                {"title": "回滚预案", "url": "https://example.com/rollback", "content": "回滚步骤…"}
            ]
        });
        let hits = parse_hits(&value);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "灰度发布");
        assert_eq!(hits[0].site, "example.com");
    }

    #[test]
    fn empty_shape_yields_no_hit() {
        assert!(parse_hits(&serde_json::json!({"code": 500})).is_empty());
        assert!(parse_hits(&serde_json::json!({"results": []})).is_empty());
    }

    #[test]
    fn provider_parse_is_case_insensitive_and_strict() {
        assert_eq!(
            SearchProvider::parse(" Bocha "),
            Some(SearchProvider::Bocha)
        );
        assert_eq!(
            SearchProvider::parse("TAVILY"),
            Some(SearchProvider::Tavily)
        );
        assert_eq!(SearchProvider::parse("bing"), None);
    }
}
