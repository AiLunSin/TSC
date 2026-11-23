//! 按需补全外发白名单与首卡时延样本用例（SPEC 7.3、DESIGN 2.10.3）。

use std::net::IpAddr;
use std::sync::Mutex;

use meeting_copilot_application::proactive::on_demand_probe::{
    measure_first_card_ms, summarize, OnDemandSampler, SampleStatus,
};
use meeting_copilot_application::proactive::outbound_guard::{
    AllowedEndpoint, OutboundGuard, OutboundReject, OutboundTarget,
};
use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::proactive::{
    OnDemandOutcome, OnDemandSample, ON_DEMAND_P95_BUDGET_MS, ON_DEMAND_REQUIRED_SAMPLES,
};
use meeting_copilot_domain::port::proactive::OnDemandSampleStore;
use meeting_copilot_domain::vo::ProviderId;

fn guard() -> OutboundGuard {
    OutboundGuard::new(vec![AllowedEndpoint::new("provider-1", "wiki.example.com")])
}

fn target(url: &str) -> OutboundTarget {
    OutboundTarget {
        provider_profile_id: "provider-1".to_owned(),
        provider_enabled: true,
        url: url.to_owned(),
        resolved_ips: Vec::new(),
        cross_host_redirects: 0,
        same_host_redirects: 0,
    }
}

#[test]
fn allows_enabled_https_endpoint_on_configured_host() {
    assert_eq!(
        guard().check(&target("https://wiki.example.com/page?id=1")),
        Ok(())
    );
}

#[test]
fn rejects_disabled_unknown_and_non_https() {
    let mut disabled = target("https://wiki.example.com/page");
    disabled.provider_enabled = false;
    assert_eq!(
        guard().check(&disabled),
        Err(OutboundReject::ProviderDisabled)
    );

    let mut unknown = target("https://wiki.example.com/page");
    unknown.provider_profile_id = "provider-2".to_owned();
    assert_eq!(
        guard().check(&unknown),
        Err(OutboundReject::ProviderUnknown)
    );

    assert_eq!(
        guard().check(&target("http://wiki.example.com/page")),
        Err(OutboundReject::NotHttps)
    );
    assert_eq!(
        guard().check(&target("https://other.example.com/page")),
        Err(OutboundReject::HostMismatch)
    );
}

#[test]
fn rejects_loopback_private_and_link_local_addresses() {
    let mut loopback = target("https://wiki.example.com/page");
    loopback.resolved_ips = vec!["127.0.0.1".parse::<IpAddr>().expect("ip")];
    assert_eq!(
        guard().check(&loopback),
        Err(OutboundReject::LoopbackAddress)
    );

    let mut private = target("https://wiki.example.com/page");
    private.resolved_ips = vec!["10.0.0.8".parse::<IpAddr>().expect("ip")];
    assert_eq!(guard().check(&private), Err(OutboundReject::PrivateAddress));

    let mut link_local = target("https://wiki.example.com/page");
    link_local.resolved_ips = vec!["169.254.10.1".parse::<IpAddr>().expect("ip")];
    assert_eq!(
        guard().check(&link_local),
        Err(OutboundReject::LinkLocalAddress)
    );

    let localhost_guard = OutboundGuard::new(vec![AllowedEndpoint::new("provider-1", "localhost")]);
    assert_eq!(
        localhost_guard.check(&target("https://localhost/page")),
        Err(OutboundReject::LoopbackAddress)
    );
}

#[test]
fn rejects_cross_host_and_excessive_redirects() {
    let mut cross = target("https://wiki.example.com/page");
    cross.cross_host_redirects = 1;
    assert_eq!(
        guard().check(&cross),
        Err(OutboundReject::RedirectCrossHost)
    );

    let mut too_many = target("https://wiki.example.com/page");
    too_many.same_host_redirects = 3;
    assert_eq!(
        guard().check(&too_many),
        Err(OutboundReject::RedirectTooMany)
    );

    let mut allowed = target("https://wiki.example.com/page");
    allowed.same_host_redirects = 2;
    assert_eq!(guard().check(&allowed), Ok(()));
}

#[test]
fn rejects_malformed_urls() {
    assert_eq!(
        guard().check(&target("wiki.example.com/page")),
        Err(OutboundReject::Malformed)
    );
    assert_eq!(
        guard().check(&target("https://user@wiki.example.com/page")),
        Err(OutboundReject::Malformed)
    );
}

fn sample(seed: usize, latency_ms: u64, outcome: OnDemandOutcome) -> OnDemandSample {
    OnDemandSample {
        sample_id: format!("sample-{seed}"),
        provider_profile_id: ProviderId::new("provider-1").expect("provider"),
        started_at_ms: i64::try_from(seed).expect("seed"),
        latency_ms,
        outcome,
        cache_hit: false,
    }
}

#[test]
fn fewer_than_two_hundred_real_samples_stays_insufficient() {
    let samples: Vec<OnDemandSample> = (0..ON_DEMAND_REQUIRED_SAMPLES - 1)
        .map(|seed| sample(seed, 1_000, OnDemandOutcome::Success))
        .collect();
    let report = summarize(&samples);
    assert_eq!(report.real_sample_count, ON_DEMAND_REQUIRED_SAMPLES - 1);
    assert_eq!(report.status, SampleStatus::Insufficient);
    assert_eq!(report.status.label(), "样本不足");
}

#[test]
fn two_hundred_real_samples_is_sufficient_and_within_budget() {
    let mut samples: Vec<OnDemandSample> = (0..ON_DEMAND_REQUIRED_SAMPLES)
        .map(|seed| sample(seed, 1_000, OnDemandOutcome::Success))
        .collect();
    let report = summarize(&samples);
    assert_eq!(report.status, SampleStatus::Sufficient);
    assert_eq!(report.real_sample_count, ON_DEMAND_REQUIRED_SAMPLES);
    assert_eq!(report.p95_ms, Some(1_000));
    assert_eq!(
        report.p95_within_budget,
        Some(1_000 <= ON_DEMAND_P95_BUDGET_MS)
    );

    samples.push(sample(999, 9_000, OnDemandOutcome::Success));
    let report = summarize(&samples);
    assert_eq!(report.real_sample_count, ON_DEMAND_REQUIRED_SAMPLES + 1);
    assert_eq!(report.p95_ms, Some(1_000));
}

#[test]
fn cache_hits_are_excluded_from_real_samples() {
    let mut samples: Vec<OnDemandSample> = (0..ON_DEMAND_REQUIRED_SAMPLES)
        .map(|seed| sample(seed, 500, OnDemandOutcome::Success))
        .collect();
    for seed in 0..50 {
        let mut hit = sample(seed, 10, OnDemandOutcome::Success);
        hit.cache_hit = true;
        samples.push(hit);
    }
    let report = summarize(&samples);
    assert_eq!(report.real_sample_count, ON_DEMAND_REQUIRED_SAMPLES);
    assert_eq!(report.cache_hit_rate, Some(0.2));
}

#[test]
fn timeout_rate_budget_is_two_percent() {
    let mut samples: Vec<OnDemandSample> = (0..98)
        .map(|seed| sample(seed, 500, OnDemandOutcome::Success))
        .collect();
    samples.push(sample(98, 9_000, OnDemandOutcome::Timeout));
    samples.push(sample(99, 9_000, OnDemandOutcome::Timeout));

    let report = summarize(&samples);
    assert_eq!(report.timeout_rate, Some(0.02));
    assert_eq!(report.timeout_within_budget, Some(true));

    samples.push(sample(100, 9_000, OnDemandOutcome::Timeout));
    let report = summarize(&samples);
    assert_eq!(report.timeout_within_budget, Some(false));
}

#[test]
fn first_card_measurement_rejects_negative_offsets() {
    assert_eq!(measure_first_card_ms(1_000, 1_500), Some(500));
    assert_eq!(measure_first_card_ms(1_000, 1_000), Some(0));
    assert_eq!(measure_first_card_ms(1_000, 999), None);
}

#[derive(Default)]
struct FakeSampleStore {
    samples: Mutex<Vec<OnDemandSample>>,
}

impl OnDemandSampleStore for FakeSampleStore {
    fn append(&self, sample: &OnDemandSample) -> Result<(), CopilotError> {
        self.samples.lock().expect("samples").push(sample.clone());
        Ok(())
    }

    fn load(&self) -> Result<Vec<OnDemandSample>, CopilotError> {
        Ok(self.samples.lock().expect("samples").clone())
    }
}

#[test]
fn sampler_records_and_reports_samples() {
    let store = FakeSampleStore::default();
    let sampler = OnDemandSampler::new(&store);
    sampler
        .record(&sample(1, 900, OnDemandOutcome::Success))
        .expect("record");

    let report = sampler.report().expect("report");
    assert_eq!(report.real_sample_count, 1);
    assert_eq!(report.status, SampleStatus::Insufficient);

    let mut invalid = sample(2, 900, OnDemandOutcome::Success);
    invalid.sample_id = "  ".to_owned();
    assert!(sampler.record(&invalid).is_err());
}
