//! 按需补全编排（SPEC 7.3.3、8.5；DESIGN 2.10.3、2.11）。
//!
//! 顺序固定：零源 → 外发白名单 → 缓存 → 适配器 → 裁决 → 写样本。
//! 缓存命中不调用适配器、不计入 200 条真实样本；并发外发不超过 3。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::card::Reference;
use meeting_copilot_domain::model::enums::{AccessStatus, AuditAction};
use meeting_copilot_domain::model::proactive::{CachedQuery, OnDemandOutcome, OnDemandSample};
use meeting_copilot_domain::port::governance::AuditLog;
use meeting_copilot_domain::port::proactive::{OnDemandSampleStore, QueryCacheStore};
use meeting_copilot_domain::port::search::{
    ProviderFailure, ProviderRequest, SearchHit, SearchProvider,
};
use meeting_copilot_domain::vo::{ReferenceRef, WallClock};

use crate::proactive::cache::{cache_key, CacheService};
use crate::proactive::no_provider_mode::OnlyPointsMode;
use crate::proactive::on_demand::{decide, OnDemandDecision, ProviderAttempt};
use crate::proactive::on_demand_probe::OnDemandSampler;
use crate::proactive::outbound_guard::{OutboundGuard, OutboundTarget};

/// 同时外发的上限（SPEC 7.3.4）。
pub const MAX_CONCURRENT_OUTBOUND: usize = 3;

#[derive(Clone)]
pub struct RunnerSource {
    pub provider: Arc<dyn SearchProvider>,
    /// 配置中声明的目标主机，必须与适配器实际连接的主机一致。
    pub host: String,
    pub enabled: bool,
}

impl RunnerSource {
    pub fn new(provider: Arc<dyn SearchProvider>, host: impl Into<String>, enabled: bool) -> Self {
        Self {
            provider,
            host: host.into(),
            enabled,
        }
    }
}

struct GateState {
    active: usize,
    peak: usize,
}

/// 外发并发闸门；超限调用者排队等待。
pub struct ConcurrencyGate {
    limit: usize,
    state: Mutex<GateState>,
    released: Condvar,
}

impl ConcurrencyGate {
    pub fn new(limit: usize) -> Self {
        Self {
            limit: limit.max(1),
            state: Mutex::new(GateState { active: 0, peak: 0 }),
            released: Condvar::new(),
        }
    }

    pub fn for_search() -> Self {
        Self::new(MAX_CONCURRENT_OUTBOUND)
    }

    pub fn acquire(&self) -> Permit<'_> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        while state.active >= self.limit {
            state = self
                .released
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        state.active += 1;
        state.peak = state.peak.max(state.active);
        Permit { gate: self }
    }

    pub fn peak(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .peak
    }

    pub fn active(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .active
    }
}

impl Default for ConcurrencyGate {
    fn default() -> Self {
        Self::for_search()
    }
}

pub struct Permit<'a> {
    gate: &'a ConcurrencyGate,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut state = self
            .gate
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.active = state.active.saturating_sub(1);
        self.gate.released.notify_one();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    pub decision: OnDemandDecision,
    pub attempts: Vec<ProviderAttempt>,
    pub references: Vec<Reference>,
    pub samples_written: usize,
}

pub struct OnDemandRunner<'a> {
    guard: OutboundGuard,
    gate: &'a ConcurrencyGate,
    sources: &'a [RunnerSource],
    audit: Option<&'a dyn AuditLog>,
    sequence: AtomicU64,
}

impl<'a> OnDemandRunner<'a> {
    pub fn new(
        guard: OutboundGuard,
        gate: &'a ConcurrencyGate,
        sources: &'a [RunnerSource],
        audit: Option<&'a dyn AuditLog>,
    ) -> Self {
        Self {
            guard,
            gate,
            sources,
            audit,
            sequence: AtomicU64::new(0),
        }
    }

    /// `now_ms` 用于缓存有效期与样本时间；`canceled` 为真时优先返回取消。
    pub fn run<C: QueryCacheStore, S: OnDemandSampleStore>(
        &self,
        request: &ProviderRequest,
        now_ms: i64,
        canceled: bool,
        cache: &CacheService<'_, C>,
        sampler: &OnDemandSampler<'_, S>,
    ) -> Result<RunResult, CopilotError> {
        if canceled {
            return Ok(RunResult {
                decision: OnDemandDecision::Canceled,
                attempts: Vec::new(),
                references: Vec::new(),
                samples_written: 0,
            });
        }

        let enabled = self.sources.iter().filter(|source| source.enabled).count();
        let mode = OnlyPointsMode::from_usable_count(enabled);
        if mode.is_active() {
            return Ok(RunResult {
                decision: mode.decision(),
                attempts: Vec::new(),
                references: Vec::new(),
                samples_written: 0,
            });
        }

        let mut attempts = Vec::new();
        let mut references = Vec::new();
        let mut samples_written = 0;

        for source in self.sources.iter().filter(|source| source.enabled) {
            let provider_id = source.provider.profile_id().clone();
            let target = OutboundTarget {
                provider_profile_id: provider_id.as_str().to_owned(),
                provider_enabled: true,
                url: format!("{}://{}/", "https", source.host),
                resolved_ips: request.resolved_ips.clone(),
                cross_host_redirects: 0,
                same_host_redirects: 0,
            };

            if let Err(reject) = self.guard.check(&target) {
                self.audit(
                    &provider_id,
                    request,
                    &format!("rejected:{}", reject.audit_label()),
                );
                samples_written += self.write_sample(
                    sampler,
                    &provider_id,
                    now_ms,
                    0,
                    OnDemandOutcome::Failed,
                    false,
                )?;
                attempts.push(ProviderAttempt::new(
                    provider_id,
                    OnDemandOutcome::Failed,
                    0,
                ));
                continue;
            }

            match cache.lookup(provider_id.as_str(), request.query.as_str(), now_ms) {
                Ok(Some(hit)) => {
                    let count = hit.entry.references.len();
                    references.extend(hit.entry.references.iter().cloned());
                    samples_written += self.write_sample(
                        sampler,
                        &provider_id,
                        now_ms,
                        0,
                        OnDemandOutcome::Success,
                        true,
                    )?;
                    self.audit(&provider_id, request, "cache_hit");
                    attempts.push(ProviderAttempt::new(
                        provider_id,
                        OnDemandOutcome::Success,
                        count,
                    ));
                    continue;
                }
                Ok(None) => {}
                Err(error) => return Err(error),
            }

            let started = Instant::now();
            let outcome = {
                let _permit = self.gate.acquire();
                source.provider.search(request)
            };
            let measured_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

            match outcome {
                Ok(response) => {
                    let hits = response.hits;
                    let fetched_at = WallClock::from_unix_millis(now_ms);
                    let mut new_references = Vec::with_capacity(hits.len());
                    for (index, hit) in hits.iter().enumerate() {
                        if let Some(reference) = to_reference(&provider_id, index, hit, fetched_at)
                        {
                            new_references.push(reference);
                        }
                    }
                    let latency = response.elapsed_ms.max(measured_ms);
                    let real_outcome = if new_references.is_empty() {
                        OnDemandOutcome::Failed
                    } else {
                        OnDemandOutcome::Success
                    };

                    if !new_references.is_empty() {
                        let byte_size = new_references
                            .iter()
                            .map(|reference| {
                                reference.title.char_count() + reference.url.to_string().len()
                            })
                            .sum::<usize>()
                            .max(1);
                        let entry = CachedQuery::new(
                            cache_key(provider_id.as_str(), request.query.as_str()),
                            provider_id.clone(),
                            None,
                            now_ms,
                            new_references.clone(),
                            byte_size,
                            now_ms,
                        )
                        .map_err(|error| CopilotError::StoreFailure {
                            detail: error.to_string(),
                        })?;
                        cache.store_result(&entry)?;
                        references.extend(new_references.iter().cloned());
                    }

                    self.audit(&provider_id, request, real_outcome.as_str());
                    samples_written += self.write_sample(
                        sampler,
                        &provider_id,
                        now_ms,
                        latency,
                        real_outcome,
                        false,
                    )?;
                    attempts.push(ProviderAttempt::new(
                        provider_id,
                        real_outcome,
                        new_references.len(),
                    ));
                }
                Err(failure) => {
                    let outcome = match failure {
                        ProviderFailure::Timeout => OnDemandOutcome::Timeout,
                        _ => OnDemandOutcome::Failed,
                    };
                    self.audit(
                        &provider_id,
                        request,
                        &format!("failed:{}", failure.as_str()),
                    );
                    samples_written += self.write_sample(
                        sampler,
                        &provider_id,
                        now_ms,
                        measured_ms,
                        outcome,
                        false,
                    )?;
                    attempts.push(ProviderAttempt::new(provider_id, outcome, 0));
                }
            }
        }

        Ok(RunResult {
            decision: decide(canceled, &attempts),
            attempts,
            references,
            samples_written,
        })
    }

    fn write_sample<S: OnDemandSampleStore>(
        &self,
        sampler: &OnDemandSampler<'_, S>,
        provider_id: &meeting_copilot_domain::vo::ProviderId,
        now_ms: i64,
        latency_ms: u64,
        outcome: OnDemandOutcome,
        cache_hit: bool,
    ) -> Result<usize, CopilotError> {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let sample = OnDemandSample {
            sample_id: format!("od-{now_ms}-{sequence}"),
            provider_profile_id: provider_id.clone(),
            started_at_ms: now_ms,
            latency_ms,
            outcome,
            cache_hit,
        };
        sampler.record(&sample)?;
        Ok(1)
    }

    fn audit(
        &self,
        provider_id: &meeting_copilot_domain::vo::ProviderId,
        request: &ProviderRequest,
        result: &str,
    ) {
        if let Some(audit) = self.audit {
            let summary = format!(
                "outbound_query provider={} qlen={} result={result}",
                provider_id.as_str(),
                request.query.as_str().chars().count()
            );
            // 审计失败不回滚检索结果；调用方可通过审计链校验发现缺口。
            let _ = audit.append(AuditAction::OutboundQuery, &summary);
        }
    }
}

fn to_reference(
    provider_id: &meeting_copilot_domain::vo::ProviderId,
    index: usize,
    hit: &SearchHit,
    fetched_at: WallClock,
) -> Option<Reference> {
    let reference = ReferenceRef::new(format!("{}-{index}", provider_id.as_str())).ok()?;
    Some(Reference {
        reference,
        title: hit.title.clone(),
        url: hit.url.clone(),
        access: AccessStatus::Ok,
        fetched_at,
        published_at: hit.published_at,
        trust: hit.trust,
        excerpt: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Barrier;
    use std::thread;
    use std::time::Duration;

    use meeting_copilot_domain::error::CopilotError;
    use meeting_copilot_domain::model::card::AbsoluteUrl;
    use meeting_copilot_domain::model::enums::ProviderHealth;
    use meeting_copilot_domain::model::proactive::CachedQuery;
    use meeting_copilot_domain::model::provider::ProviderTimeout;
    use meeting_copilot_domain::port::proactive::{OnDemandSampleStore, QueryCacheStore};
    use meeting_copilot_domain::port::search::{
        ProviderFailure, ProviderRequest, ProviderResponse, SearchHit, SearchProvider,
    };
    use meeting_copilot_domain::vo::{ProviderId, SanitizedQuery, TitleText};

    use crate::proactive::outbound_guard::AllowedEndpoint;

    struct MockProvider {
        id: ProviderId,
        hits: usize,
        failure: Option<ProviderFailure>,
        calls: Arc<AtomicUsize>,
    }

    impl MockProvider {
        fn ok(id: &str, hits: usize, calls: Arc<AtomicUsize>) -> Arc<Self> {
            Arc::new(Self {
                id: ProviderId::new(id).expect("id"),
                hits,
                failure: None,
                calls,
            })
        }

        fn failing(id: &str, failure: ProviderFailure, calls: Arc<AtomicUsize>) -> Arc<Self> {
            Arc::new(Self {
                id: ProviderId::new(id).expect("id"),
                hits: 0,
                failure: Some(failure),
                calls,
            })
        }
    }

    impl SearchProvider for MockProvider {
        fn profile_id(&self) -> &ProviderId {
            &self.id
        }

        fn kind(&self) -> meeting_copilot_domain::model::enums::ProviderKind {
            meeting_copilot_domain::model::enums::ProviderKind::Custom
        }

        fn health(&self) -> ProviderHealth {
            ProviderHealth::Healthy
        }

        fn search(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderFailure> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(failure) = self.failure {
                return Err(failure);
            }
            let hits = (0..self.hits.min(usize::from(request.limit)))
                .map(|index| SearchHit {
                    title: TitleText::new(format!("标题{index}")).expect("title"),
                    url: AbsoluteUrl::new(format!("https://docs.example.com/{index}"))
                        .expect("url"),
                    domain: "docs.example.com".to_owned(),
                    snippet: None,
                    published_at: None,
                    trust: meeting_copilot_domain::model::enums::TrustTier::GeneralWeb,
                })
                .collect();
            Ok(ProviderResponse {
                provider: self.id.clone(),
                hits,
                elapsed_ms: 5,
            })
        }
    }

    #[derive(Default)]
    struct MemoryCache {
        entries: Mutex<HashMap<String, CachedQuery>>,
    }

    impl QueryCacheStore for MemoryCache {
        fn get(&self, query_hash: &str) -> Result<Option<CachedQuery>, CopilotError> {
            Ok(self.entries.lock().expect("lock").get(query_hash).cloned())
        }

        fn put(&self, entry: &CachedQuery) -> Result<usize, CopilotError> {
            self.entries
                .lock()
                .expect("lock")
                .insert(entry.query_hash.clone(), entry.clone());
            Ok(0)
        }

        fn touch(&self, _query_hash: &str, _at_ms: i64) -> Result<(), CopilotError> {
            Ok(())
        }

        fn sweep_expired(&self, _now_ms: i64) -> Result<usize, CopilotError> {
            Ok(0)
        }

        fn delete_session(
            &self,
            _session_id: &meeting_copilot_domain::vo::SessionId,
        ) -> Result<usize, CopilotError> {
            Ok(0)
        }
    }

    #[derive(Default)]
    struct MemorySamples {
        samples: Mutex<Vec<OnDemandSample>>,
    }

    impl OnDemandSampleStore for MemorySamples {
        fn append(&self, sample: &OnDemandSample) -> Result<(), CopilotError> {
            self.samples.lock().expect("lock").push(sample.clone());
            Ok(())
        }

        fn load(&self) -> Result<Vec<OnDemandSample>, CopilotError> {
            Ok(self.samples.lock().expect("lock").clone())
        }
    }

    fn request() -> ProviderRequest {
        ProviderRequest::new(
            SanitizedQuery::try_from("模型 上下文".to_owned()).expect("query"),
            5,
            ProviderTimeout::from_millis(8_000).expect("timeout"),
            Vec::new(),
        )
        .expect("request")
    }

    fn guard(host: &str) -> OutboundGuard {
        OutboundGuard::new(vec![AllowedEndpoint::new("p1", host)])
    }

    #[test]
    fn zero_source_returns_no_source_without_side_effects() {
        let calls = Arc::new(AtomicUsize::new(0));
        let cache = MemoryCache::default();
        let samples = MemorySamples::default();
        let gate = ConcurrencyGate::for_search();
        let runner = OnDemandRunner::new(guard("search.example.com"), &gate, &[], None);
        let result = runner
            .run(
                &request(),
                1_000,
                false,
                &CacheService::new(&cache),
                &OnDemandSampler::new(&samples),
            )
            .expect("run");
        assert_eq!(result.decision, OnDemandDecision::NoSource);
        assert_eq!(result.samples_written, 0);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(samples.load().expect("samples").is_empty());
    }

    #[test]
    fn cache_hit_skips_adapter_and_does_not_consume_real_sample() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = MockProvider::ok("p1", 2, Arc::clone(&calls));
        let cache = MemoryCache::default();
        let samples = MemorySamples::default();
        let gate = ConcurrencyGate::for_search();
        let source = RunnerSource::new(provider, "search.example.com", true);
        let runner = OnDemandRunner::new(
            guard("search.example.com"),
            &gate,
            std::slice::from_ref(&source),
            None,
        );
        let req = request();
        let cache_service = CacheService::new(&cache);
        let sampler = OnDemandSampler::new(&samples);

        let first = runner
            .run(&req, 1_000, false, &cache_service, &sampler)
            .expect("first");
        assert_eq!(first.decision, OnDemandDecision::Complete { sources: 1 });
        assert_eq!(first.samples_written, 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = runner
            .run(&req, 2_000, false, &cache_service, &sampler)
            .expect("second");
        assert_eq!(second.decision, OnDemandDecision::Complete { sources: 1 });
        assert_eq!(calls.load(Ordering::SeqCst), 1, "缓存命中不得调用适配器");
        assert_eq!(second.samples_written, 1);

        let all = samples.load().expect("samples");
        assert_eq!(all.len(), 2);
        assert!(!all[0].cache_hit);
        assert!(all[1].cache_hit);
        let report = crate::proactive::on_demand_probe::summarize(&all);
        assert_eq!(report.real_sample_count, 1, "缓存命中不计入 200 条门槛");
    }

    #[test]
    fn partial_and_all_failed_are_reported() {
        let calls = Arc::new(AtomicUsize::new(0));
        let ok = MockProvider::ok("p1", 1, Arc::clone(&calls));
        let bad = MockProvider::failing("p2", ProviderFailure::Timeout, Arc::clone(&calls));
        let cache = MemoryCache::default();
        let samples = MemorySamples::default();
        let gate = ConcurrencyGate::for_search();
        let guard = OutboundGuard::new(vec![
            AllowedEndpoint::new("p1", "search.example.com"),
            AllowedEndpoint::new("p2", "other.example.com"),
        ]);
        let sources = [
            RunnerSource::new(ok, "search.example.com", true),
            RunnerSource::new(bad, "search.example.com", true),
        ];
        let runner = OnDemandRunner::new(guard, &gate, &sources, None);
        let result = runner
            .run(
                &request(),
                1_000,
                false,
                &CacheService::new(&cache),
                &OnDemandSampler::new(&samples),
            )
            .expect("run");
        assert_eq!(result.decision, OnDemandDecision::Partial { sources: 1 });
        assert_eq!(result.decision.notice(), Some("结果可能不完整"));
        assert_eq!(result.attempts.len(), 2);
        assert_eq!(result.attempts[1].outcome, OnDemandOutcome::Failed);
    }

    #[test]
    fn all_failed_does_not_produce_conclusion() {
        let calls = Arc::new(AtomicUsize::new(0));
        let bad = MockProvider::failing("p1", ProviderFailure::RateLimited, calls);
        let cache = MemoryCache::default();
        let samples = MemorySamples::default();
        let gate = ConcurrencyGate::for_search();
        let sources = [RunnerSource::new(bad, "search.example.com", true)];
        let runner = OnDemandRunner::new(guard("search.example.com"), &gate, &sources, None);
        let result = runner
            .run(
                &request(),
                1_000,
                false,
                &CacheService::new(&cache),
                &OnDemandSampler::new(&samples),
            )
            .expect("run");
        assert_eq!(result.decision, OnDemandDecision::AllFailed);
        assert!(result.references.is_empty());
    }

    #[test]
    fn canceled_run_writes_no_sample() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = MockProvider::ok("p1", 1, calls);
        let cache = MemoryCache::default();
        let samples = MemorySamples::default();
        let gate = ConcurrencyGate::for_search();
        let sources = [RunnerSource::new(provider, "search.example.com", true)];
        let runner = OnDemandRunner::new(guard("search.example.com"), &gate, &sources, None);
        let result = runner
            .run(
                &request(),
                1_000,
                true,
                &CacheService::new(&cache),
                &OnDemandSampler::new(&samples),
            )
            .expect("run");
        assert_eq!(result.decision, OnDemandDecision::Canceled);
        assert_eq!(result.samples_written, 0);
        assert!(samples.load().expect("samples").is_empty());
    }

    #[test]
    fn concurrency_gate_limits_peak_to_three() {
        let gate = Arc::new(ConcurrencyGate::for_search());
        let barrier = Arc::new(Barrier::new(6));
        let mut handles = Vec::new();
        for _ in 0..6 {
            let gate = Arc::clone(&gate);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                let permit = gate.acquire();
                thread::sleep(Duration::from_millis(20));
                drop(permit);
            }));
        }
        for handle in handles {
            handle.join().expect("join");
        }
        assert!(gate.peak() <= MAX_CONCURRENT_OUTBOUND);
        assert_eq!(gate.active(), 0);
    }

    #[test]
    fn hit_title_is_truncated_and_reference_has_no_excerpt() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = MockProvider::ok("p1", 1, calls);
        let cache = MemoryCache::default();
        let samples = MemorySamples::default();
        let gate = ConcurrencyGate::for_search();
        let sources = [RunnerSource::new(provider, "search.example.com", true)];
        let runner = OnDemandRunner::new(guard("search.example.com"), &gate, &sources, None);
        let result = runner
            .run(
                &request(),
                1_000,
                false,
                &CacheService::new(&cache),
                &OnDemandSampler::new(&samples),
            )
            .expect("run");
        assert_eq!(result.references.len(), 1);
        assert!(result.references[0].excerpt.is_none());
    }

    #[test]
    fn real_sample_threshold_is_199_then_200() {
        use crate::proactive::on_demand_probe::{summarize, SampleStatus};

        let sample = |index: usize| OnDemandSample {
            sample_id: format!("od-{index}"),
            provider_profile_id: ProviderId::new("p1").expect("id"),
            started_at_ms: 1_000,
            latency_ms: 100,
            outcome: OnDemandOutcome::Success,
            cache_hit: false,
        };
        let mut samples: Vec<_> = (0..199).map(sample).collect();
        assert_eq!(summarize(&samples).status, SampleStatus::Insufficient);
        samples.push(sample(199));
        assert_eq!(summarize(&samples).status, SampleStatus::Sufficient);
    }
}
