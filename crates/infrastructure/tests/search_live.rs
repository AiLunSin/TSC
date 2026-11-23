//! 真实检索源手工验收与采样（H-34-G、G-43-C）。
//!
//! 默认全部 `#[ignore]`，由用户终端注入环境变量后执行：
//! - `SEARCH_PROVIDERS_FILE`：`providers.json` 绝对路径（最少 1 个启用源）。
//! - `SEARCH_QUERIES_FILE`：每行一条查询，UTF-8；查询原文只在本进程内存使用，不写入报告。
//! - `<authRef>`：`providers.json` 里 `authRef` 指定的环境变量名，值为 Bearer 凭据。
//! - `SEARCH_REPORT_PATH`（可选）：聚合报告输出路径；只包含计数与时延。
//!
//! 样本写入内存库，不落盘；报告与终端输出均不含查询原文、URL、标题或页面正文。

use std::fs;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use meeting_copilot_application::proactive::cache::CacheService;
use meeting_copilot_application::proactive::on_demand::OnDemandDecision;
use meeting_copilot_application::proactive::on_demand_probe::{
    summarize, OnDemandSampler, SampleStatus,
};
use meeting_copilot_application::proactive::on_demand_runner::{
    ConcurrencyGate, OnDemandRunner, RunnerSource,
};
use meeting_copilot_application::proactive::outbound_guard::{AllowedEndpoint, OutboundGuard};
use meeting_copilot_domain::model::proactive::OnDemandOutcome;
use meeting_copilot_domain::model::provider::ProviderTimeout;
use meeting_copilot_domain::port::governance::AuditLog;
use meeting_copilot_domain::port::proactive::OnDemandSampleStore;
use meeting_copilot_domain::port::search::ProviderRequest;
use meeting_copilot_domain::vo::SanitizedQuery;
use meeting_copilot_infrastructure::audit::AuditStore;
use meeting_copilot_infrastructure::search::{
    parse_https_endpoint, HttpJsonProvider, ProfileStore, Registry,
};
use meeting_copilot_infrastructure::store::SqliteStore;

const QUERY_LIMIT: u8 = 5;

fn env_path(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("缺少环境变量 {name}"))
}

fn now_ms() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时钟早于 1970")
        .as_millis();
    i64::try_from(millis).unwrap_or(i64::MAX)
}

fn read_queries() -> Vec<String> {
    let raw = fs::read_to_string(env_path("SEARCH_QUERIES_FILE")).expect("读取查询文件失败");
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

struct Harness {
    guard: OutboundGuard,
    gate: ConcurrencyGate,
    sources: Vec<RunnerSource>,
}

impl Harness {
    fn build() -> Self {
        let store = ProfileStore::new(env_path("SEARCH_PROVIDERS_FILE"));
        let registry = Registry::from_entries(store.load().expect("providers.json 解析失败"));
        let usable = registry.usable();
        assert!(!usable.is_empty(), "providers.json 没有可用检索源");

        let mut sources = Vec::new();
        let mut endpoints = Vec::new();
        for entry in usable {
            let endpoint = parse_https_endpoint(&entry.endpoint).expect("端点应为 HTTPS");
            let provider = HttpJsonProvider::from_entry(entry).expect("构造适配器失败");
            endpoints.push(AllowedEndpoint::new(
                entry.profile.id.as_str(),
                endpoint.host.clone(),
            ));
            sources.push(RunnerSource::new(Arc::new(provider), endpoint.host, true));
        }
        Self {
            guard: OutboundGuard::new(endpoints),
            gate: ConcurrencyGate::for_search(),
            sources,
        }
    }

    fn runner<'a>(&'a self, audit: Option<&'a dyn AuditLog>) -> OnDemandRunner<'a> {
        OnDemandRunner::new(self.guard.clone(), &self.gate, &self.sources, audit)
    }
}

fn request_for(query: &str) -> ProviderRequest {
    let timeout = ProviderTimeout::from_millis(8_000).expect("超时上限");
    ProviderRequest::new(
        SanitizedQuery::try_from(query.to_owned()).expect("查询为空或超过长度上限"),
        QUERY_LIMIT,
        timeout,
        Vec::new(),
    )
    .expect("请求参数非法")
}

struct RunOutcome {
    samples_written: usize,
    audit_records: usize,
}

fn run_queries(harness: &Harness, queries: &[String], store: &SqliteStore) -> RunOutcome {
    let cache = CacheService::new(store);
    let sampler = OnDemandSampler::new(store);
    let audit = AuditStore::new(store);
    let runner = harness.runner(Some(&audit as &dyn AuditLog));
    let mut written = 0;
    for (index, query) in queries.iter().enumerate() {
        let request = request_for(query);
        let result = runner
            .run(&request, now_ms(), false, &cache, &sampler)
            .expect("runner 执行失败");
        assert!(
            !matches!(
                result.decision,
                OnDemandDecision::NoSource | OnDemandDecision::Canceled
            ),
            "第 {} 次检索未进入可用源",
            index + 1
        );
        written += result.samples_written;
    }
    let broken = audit.verify().expect("审计链校验失败");
    assert!(broken.is_none(), "审计链在 {broken:?} 处断开");
    RunOutcome {
        samples_written: written,
        audit_records: audit.record_count().expect("读取审计条数失败"),
    }
}

fn emit(samples: &[meeting_copilot_domain::model::proactive::OnDemandSample], providers: usize) {
    let summary = summarize(samples);
    let timeouts = samples
        .iter()
        .filter(|sample| sample.outcome == OnDemandOutcome::Timeout)
        .count();
    let failures = samples
        .iter()
        .filter(|sample| sample.outcome == OnDemandOutcome::Failed)
        .count();
    let json = format!(
        "{{\"providers\":{providers},\"samples\":{},\"real_samples\":{},\"cache_hits\":{},\
         \"timeouts\":{},\"failures\":{},\"p50_ms\":{},\"p95_ms\":{},\"status\":\"{}\",\"target\":200}}",
        summary.real_sample_count + samples.iter().filter(|s| s.cache_hit).count() + timeouts + failures,
        summary.real_sample_count,
        samples.iter().filter(|s| s.cache_hit).count(),
        timeouts,
        failures,
        summary.p50_ms.unwrap_or(0),
        summary.p95_ms.unwrap_or(0),
        summary.status.label()
    );
    println!("{json}");
    if let Ok(path) = std::env::var("SEARCH_REPORT_PATH") {
        fs::write(path, format!("{json}\n")).expect("写入报告失败");
    }
}

#[test]
#[ignore = "需真实 HTTPS 检索源与 SEARCH_* 环境变量"]
fn smoke_three_rounds() {
    let harness = Harness::build();
    let queries = read_queries();
    assert!(queries.len() >= 3, "至少提供 3 条查询");
    let store = SqliteStore::open_in_memory().expect("内存库");
    let outcome = run_queries(&harness, &queries[..3], &store);
    assert_eq!(outcome.samples_written, 3, "三次检索均应写入样本");
    assert!(
        outcome.audit_records >= 3,
        "三次检索至少产生 3 条审计，实际 {}",
        outcome.audit_records
    );

    let samples = store.load().expect("读取样本");
    let succeeded = samples
        .iter()
        .filter(|sample| !sample.cache_hit && sample.outcome == OnDemandOutcome::Success)
        .count();
    assert!(succeeded > 0, "三次检索均未成功，请检查端点与凭据");
    assert_eq!(
        summarize(&samples).status,
        SampleStatus::Insufficient,
        "3 条样本应判定为样本不足"
    );
    emit(&samples, harness.sources.len());
}

#[test]
#[ignore = "需真实 HTTPS 检索源、200 条查询与 SEARCH_* 环境变量"]
fn collect_200_real_samples() {
    let harness = Harness::build();
    let queries = read_queries();
    assert!(queries.len() >= 200, "查询文件至少 200 条");

    let store = SqliteStore::open_in_memory().expect("内存库");
    let outcome = run_queries(&harness, &queries[..200], &store);
    assert!(
        outcome.audit_records >= 200,
        "审计条数不足 200，实际 {}",
        outcome.audit_records
    );
    let samples = store.load().expect("读取样本");
    let summary = summarize(&samples);
    assert_eq!(
        summary.real_sample_count, 200,
        "真实成功样本不足 200（缓存命中、失败或超时不计），请补测"
    );
    emit(&samples, harness.sources.len());
}
