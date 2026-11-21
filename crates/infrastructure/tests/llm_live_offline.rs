mod support;

use std::sync::atomic::{AtomicUsize, Ordering};

use meeting_copilot_domain::model::llm::LlmTask;
use meeting_copilot_infrastructure::llm::live::config::{LiveRunConfig, SecretString};
use meeting_copilot_infrastructure::llm::live::gate::{
    evaluate, EnvView, LiveDecision, MapEnv, SkipReason,
};
use meeting_copilot_infrastructure::llm::live::record::{
    LiveRecorder, RunRecord, SubStatus, SubStatusState,
};
use meeting_copilot_infrastructure::llm::live::stats::{percentile, summarize, Sample};
use meeting_copilot_infrastructure::llm::live::{transport_if_allowed, STATS_RULE};
use meeting_copilot_infrastructure::llm::transport::HttpTransport;

fn full_env() -> MapEnv {
    MapEnv::new()
        .with("LLM_LIVE", "1")
        .with("LLM_API_KEY", "sk-test-0000")
        .with("LLM_BASE_URL", "https://example.invalid/v1/chat")
        .with("LLM_MODEL", "test-model")
        .with("LLM_MODEL_CONFIRMED", "test-model")
        .with("LLM_LIVE_CALL_CAP", "360")
        .with("LLM_ALLOWLIST", "example.invalid")
}

#[test]
fn gate_default_off_skips() {
    let decision = evaluate(&MapEnv::new(), true);
    assert_eq!(decision, LiveDecision::Skip(SkipReason::LiveDisabled));
    assert!(SkipReason::LiveDisabled.label().contains("未开启真实联调"));
    assert_eq!(SkipReason::LiveDisabled.code(), "live_disabled");
}

#[test]
fn gate_switch_on_credential_missing_skips() {
    let env = MapEnv::new().with("LLM_LIVE", "1");
    assert_eq!(
        evaluate(&env, true),
        LiveDecision::Skip(SkipReason::CredentialMissing)
    );
}

#[test]
fn gate_precedence_is_deterministic() {
    let env = MapEnv::new();
    for _ in 0..3 {
        assert_eq!(
            evaluate(&env, true),
            LiveDecision::Skip(SkipReason::LiveDisabled)
        );
    }
    let env = MapEnv::new()
        .with("LLM_LIVE", "1")
        .with("LLM_BASE_URL", "https://example.invalid/v1");
    for _ in 0..3 {
        assert_eq!(
            evaluate(&env, true),
            LiveDecision::Skip(SkipReason::CredentialMissing)
        );
    }
}

#[test]
fn skip_path_constructs_zero_transports() {
    struct Probe;
    impl HttpTransport for Probe {
        fn send(
            &self,
            _request: &meeting_copilot_infrastructure::llm::transport::HttpRequest,
            _deadline: std::time::Instant,
            _cancel: &meeting_copilot_domain::port::llm::CancelToken,
        ) -> Result<
            meeting_copilot_infrastructure::llm::transport::HttpResponse,
            meeting_copilot_infrastructure::llm::transport::TransportError,
        > {
            Err(meeting_copilot_infrastructure::llm::transport::TransportError::PeerClosed)
        }

        fn open_stream(
            &self,
            _request: &meeting_copilot_infrastructure::llm::transport::HttpRequest,
            _deadline: std::time::Instant,
            _cancel: &meeting_copilot_domain::port::llm::CancelToken,
        ) -> Result<
            Box<dyn meeting_copilot_infrastructure::llm::transport::StreamReader>,
            meeting_copilot_infrastructure::llm::transport::TransportError,
        > {
            Err(meeting_copilot_infrastructure::llm::transport::TransportError::PeerClosed)
        }
    }

    let constructed = AtomicUsize::new(0);
    let decision = evaluate(&MapEnv::new(), false);
    let transport = transport_if_allowed(decision, || {
        constructed.fetch_add(1, Ordering::SeqCst);
        Probe
    });
    assert!(transport.is_none());
    assert_eq!(constructed.load(Ordering::SeqCst), 0);
}

#[test]
fn secret_masked_in_debug_display() {
    let config = LiveRunConfig::try_new(&full_env(), None, None, None).expect("配置");
    assert_eq!(format!("{:?}", config.secret), "***");
    assert_eq!(format!("{}", config.secret), "***");
    assert!(!format!("{config:?}").contains("sk-test-0000"));
    let secret = SecretString::new("sk-test-0000");
    assert!(!format!("{secret:?}").contains("sk-test-0000"));
}

#[test]
fn record_fields_complete_and_secret_free() {
    let recorder = LiveRecorder::new("offline-self-test").expect("recorder");
    let record = RunRecord {
        schema_version: "llm.live.v2".into(),
        endpoint_host: "example.invalid".into(),
        model: "test-model".into(),
        started_at: "2026-11-21T00:00:00Z".into(),
        ended_at: "2026-11-21T00:01:00Z".into(),
        tz_offset: "+08:00".into(),
        commit_short: "abcdef0".into(),
        config: "samples=30".into(),
        per_task: 30,
        sub_status: vec![SubStatus {
            item: "ac20".into(),
            status: SubStatusState::NotMeasured,
            reason_code: Some("credential_missing".into()),
        }],
        samples_path: "samples.jsonl".into(),
        stats_rule: STATS_RULE.into(),
        build: "test".into(),
        path_confirmed: true,
        call_limit: 0,
        calls_used: 0,
        budget_hit: false,
        model_confirmed: true,
        host_locked: true,
        data_terms_declaration:
            meeting_copilot_infrastructure::llm::live::record::TERMS_DECLARATION.into(),
        data_terms_accepted_at: "2026-11-21T00:00:00+08:00".into(),
        credential_rotated: false,
        tls_trust_root: None,
    };
    recorder.write_run(&record).expect("写入 run.json");
    for index in 0..3 {
        recorder
            .append_sample(&Sample {
                seq: index,
                task: LlmTask::InsightExtract,
                attempt: 1,
                first_token_ms: Some(100),
                total_ms: 400,
                outcome: "ok".into(),
                failure_class: None,
                http_status: Some(200),
                degraded: false,
                excluded: None,
            })
            .expect("追加样本");
    }
    let value = serde_json::to_value(&record).expect("序列化");
    for key in [
        "schema_version",
        "endpoint_host",
        "model",
        "started_at",
        "ended_at",
        "tz_offset",
        "commit_short",
        "config",
        "per_task",
        "sub_status",
        "samples_path",
        "stats_rule",
        "build",
        "path_confirmed",
        "call_limit",
        "calls_used",
        "budget_hit",
        "model_confirmed",
        "host_locked",
        "data_terms_declaration",
        "data_terms_accepted_at",
        "credential_rotated",
        "tls_trust_root",
    ] {
        assert!(value.get(key).is_some(), "缺少字段 {key}");
    }
    let serialized = serde_json::to_string(&record).expect("序列化");
    assert!(!serialized.contains("sk-test-0000"));
    assert_eq!(record.calls_used, 0);
    assert!(!record.budget_hit);
    assert!(!record.data_terms_accepted_at.is_empty());
    assert!(record.samples_path.starts_with("samples"));
    assert!(recorder.root().starts_with(".tmp"));
    let _ = std::fs::remove_dir_all(recorder.root());
}

#[test]
fn error_text_has_no_secret_fragment() {
    let env = MapEnv::new()
        .with("LLM_BASE_URL", "not-a-url")
        .with("LLM_MODEL", "test-model")
        .with("LLM_API_KEY", "sk-test-0000");
    let error = LiveRunConfig::try_new(&env, None, None, None).expect_err("必须拒绝");
    let text = format!("{error}");
    assert!(!text.contains("sk-test"));
    assert!(!text.contains("0000"));
    assert!(!text.contains("LLM_API_KEY"));
}

#[test]
fn stats_rules_are_shared_with_recompute() {
    let values: Vec<u64> = (1..=30).collect();
    assert_eq!(percentile(&values, 0.95), Some(29));
    let samples: Vec<Sample> = values
        .iter()
        .enumerate()
        .map(|(index, value)| Sample {
            seq: u32::try_from(index).expect("index"),
            task: LlmTask::Summary,
            attempt: 1,
            first_token_ms: None,
            total_ms: *value,
            outcome: "ok".into(),
            failure_class: None,
            http_status: Some(200),
            degraded: false,
            excluded: None,
        })
        .collect();
    let stats = summarize(&samples, LlmTask::Summary);
    assert_eq!(stats.p95_ms, Some(29));
    assert!(STATS_RULE.contains("nearest-rank"));
}

#[test]
fn env_view_is_abstracted_from_process_env() {
    let env = full_env();
    assert_eq!(env.get("LLM_MODEL").as_deref(), Some("test-model"));
    let missing = MapEnv::new();
    assert!(missing.get("LLM_MODEL").is_none());
    let _: &dyn EnvView = &env;
}
