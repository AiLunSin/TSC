//! AC-28/29/30/31 离线运行门用例：零 socket、零真实凭据。

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use meeting_copilot_domain::model::llm::{
    DeadlineMs, LlmGenerationRequest, LlmTask, OutputBudget, RedactedPayload, SummaryPurpose,
    TemperatureMilli,
};
use meeting_copilot_domain::port::llm::CancelToken;
use meeting_copilot_domain::vo::TaskId;
use meeting_copilot_infrastructure::llm::gateway::{
    request_body, thinking_verdict, THINKING_FIELD, THINKING_NOT_DISABLED,
};
use meeting_copilot_infrastructure::llm::live::budget::{BudgetLedger, BudgetedTransport};
use meeting_copilot_infrastructure::llm::live::config::{secret_read_counters, LiveRunConfig};
use meeting_copilot_infrastructure::llm::live::gate::{
    admit, Admission, EnvView, GateReject, MapEnv,
};
use meeting_copilot_infrastructure::llm::live::policy::{
    host_is_locked, path_confirmed, HostVerdict, DEEPSEEK_HOSTS, PATH_CONSTANT,
};
use meeting_copilot_infrastructure::llm::live::record::{
    LiveRecorder, RunRecord, SubStatus, SubStatusState, TERMS_DECLARATION,
};
use meeting_copilot_infrastructure::llm::transport::{
    HttpRequest, HttpResponse, HttpTransport, StreamReader, TransportError,
};

/// 全局凭据计数探针在测试进程内共享，涉及计数的用例串行执行。
static COUNTER_LOCK: Mutex<()> = Mutex::new(());

struct CountingEnv {
    values: BTreeMap<String, String>,
    gets: AtomicUsize,
}

impl CountingEnv {
    fn new(env: MapEnv) -> Self {
        let mut values = BTreeMap::new();
        for key in [
            "LLM_LIVE",
            "LLM_API_KEY",
            "LLM_BASE_URL",
            "LLM_MODEL",
            "LLM_MODEL_CONFIRMED",
            "LLM_LIVE_CALL_CAP",
            "LLM_LIVE_SAMPLES",
            "LLM_ALLOWLIST",
        ] {
            if let Some(value) = env.get(key) {
                values.insert(key.to_owned(), value);
            }
        }
        Self {
            values,
            gets: AtomicUsize::new(0),
        }
    }

    fn gets(&self) -> usize {
        self.gets.load(Ordering::SeqCst)
    }
}

impl EnvView for CountingEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.gets.fetch_add(1, Ordering::SeqCst);
        self.values.get(key).cloned()
    }
}

struct CountingTransport {
    sends: AtomicUsize,
    streams: AtomicUsize,
}

impl CountingTransport {
    fn new() -> Self {
        Self {
            sends: AtomicUsize::new(0),
            streams: AtomicUsize::new(0),
        }
    }
}

impl HttpTransport for CountingTransport {
    fn send(
        &self,
        _request: &HttpRequest,
        _deadline: Instant,
        _cancel: &CancelToken,
    ) -> Result<HttpResponse, TransportError> {
        self.sends.fetch_add(1, Ordering::SeqCst);
        Err(TransportError::PeerClosed)
    }

    fn open_stream(
        &self,
        _request: &HttpRequest,
        _deadline: Instant,
        _cancel: &CancelToken,
    ) -> Result<Box<dyn StreamReader>, TransportError> {
        self.streams.fetch_add(1, Ordering::SeqCst);
        Err(TransportError::PeerClosed)
    }
}

fn locked_env(model: &str, confirmed: &str) -> MapEnv {
    MapEnv::new()
        .with("LLM_LIVE", "1")
        .with(
            "LLM_BASE_URL",
            "https://api.deepseek.com/v1/chat/completions",
        )
        .with("LLM_API_KEY", "sk-test-0000")
        .with("LLM_MODEL", model)
        .with("LLM_MODEL_CONFIRMED", confirmed)
        .with("LLM_LIVE_CALL_CAP", "360")
        .with("LLM_ALLOWLIST", "api.deepseek.com")
}

fn assert_reject(env: &CountingEnv, expected: GateReject) {
    match admit(env, true) {
        Admission::Reject(actual) => assert_eq!(actual, expected),
        other => panic!("应拒绝 {expected:?}，实际 {other:?}"),
    }
}

#[test]
fn endpoint_host_locked() {
    let _guard = COUNTER_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let (before_constructions, before_exposures) = secret_read_counters();
    let variants = [
        "https://evil.api.deepseek.com/v1",
        "https://api.deepseek.com.evil.com/v1",
        "https://xapi.deepseek.com/v1",
        "https://api.deepseek.co/v1",
        "https://api.deepseek.com:8443/v1",
        "https://user@api.deepseek.com/v1",
        "https://api.deepseek.com./v1",
        "https://127.0.0.1/v1",
        "https://[::1]/v1",
        "https://xn--api.deepseek.com/v1",
        "ftp://api.deepseek.com/v1",
    ];
    for raw in variants {
        let env =
            CountingEnv::new(locked_env("test-model", "test-model").with("LLM_BASE_URL", raw));
        assert_reject(&env, GateReject::EndpointNotAllowed);
    }
    for raw in [
        "https://api.deepseek.com",
        "https://api.deepseek.com/v1/chat",
    ] {
        let env =
            CountingEnv::new(locked_env("test-model", "test-model").with("LLM_BASE_URL", raw));
        assert!(matches!(admit(&env, true), Admission::Admit(_)), "{raw}");
    }
    let transport = CountingTransport::new();
    assert_eq!(transport.sends.load(Ordering::SeqCst), 0);
    assert_eq!(transport.streams.load(Ordering::SeqCst), 0);
    let (after_constructions, after_exposures) = secret_read_counters();
    assert_eq!(after_constructions, before_constructions);
    assert_eq!(after_exposures, before_exposures);
}

#[test]
fn model_explicitly_confirmed() {
    let cases = [
        ("", "test-model"),
        ("   ", "test-model"),
        ("candidate-pending", "test-model"),
        ("test-modeL", "test-model"),
    ];
    for (actual, confirmed) in cases {
        let env = CountingEnv::new(locked_env(actual, confirmed));
        assert_reject(&env, GateReject::ModelNotConfirmed);
    }
    let env = CountingEnv::new(locked_env("test-model", "test-model"));
    assert!(matches!(admit(&env, true), Admission::Admit(_)));
    let env = CountingEnv::new(locked_env("test-model", ""));
    assert_reject(&env, GateReject::ModelNotConfirmed);
}

#[test]
fn insecure_transport_rejected() {
    let env = CountingEnv::new(
        locked_env("test-model", "test-model")
            .with("LLM_BASE_URL", "http://api.deepseek.com/v1/chat"),
    );
    assert_reject(&env, GateReject::InsecureTransportRejected);
    let env = CountingEnv::new(locked_env("test-model", "test-model"));
    assert!(matches!(admit(&env, true), Admission::Admit(_)));
}

#[test]
fn admit_order_is_deterministic() {
    let env = CountingEnv::new(locked_env("test-model", "test-model"));
    let first = admit(&env, true);
    let second = admit(&env, true);
    let third = admit(&env, true);
    assert_eq!(first, second);
    assert_eq!(second, third);
    assert!(matches!(first, Admission::Admit(_)));
}

#[test]
fn budget_cap_enforced() {
    let _guard = COUNTER_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let limit = |env: MapEnv, samples: Option<usize>| {
        let config = LiveRunConfig::try_new(&env, None, samples, None).expect("配置");
        config.call_budget
    };
    let base = locked_env("test-model", "test-model");
    assert_eq!(
        limit(base.clone().with("LLM_LIVE_CALL_CAP", "200"), None),
        200
    );
    assert_eq!(
        limit(base.clone().with("LLM_LIVE_CALL_CAP", "1000"), None),
        360
    );

    let ledger = BudgetLedger::new(2);
    assert!(ledger.try_acquire().is_ok());
    assert!(ledger.try_acquire().is_ok());
    assert!(ledger.try_acquire().is_err());

    let inner = Arc::new(CountingTransport::new());
    let decorated = BudgetedTransport::new(inner.clone(), Arc::new(BudgetLedger::new(1)));
    let request =
        HttpRequest::json_post("api.deepseek.com", 443, "/v1/chat", Vec::new(), Vec::new());
    let deadline = Instant::now() + std::time::Duration::from_secs(1);
    let _ = decorated.send(&request, deadline, &CancelToken::new());
    assert_eq!(
        decorated.send(&request, deadline, &CancelToken::new()),
        Err(TransportError::BudgetExhausted)
    );
    assert_eq!(inner.sends.load(Ordering::SeqCst), 1);

    let env = CountingEnv::new(base.with("LLM_LIVE_CALL_CAP", ""));
    assert_reject(&env, GateReject::BudgetNotConfirmed);
}

#[test]
fn c1c2c3_fail_closed_matrix() {
    let _guard = COUNTER_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let base = locked_env("test-model", "test-model");
    for endpoint in ["https://api.deepseek.com", "https://api.deepseek.com/"] {
        let env = base.clone().with("LLM_BASE_URL", endpoint);
        let error = LiveRunConfig::try_new(&env, None, None, None).expect_err("必须拒绝");
        assert!(format!("{error}").contains("endpoint_path_unconfirmed"));
    }
    let env = CountingEnv::new(base.clone().with("LLM_MODEL", ""));
    assert_reject(&env, GateReject::ModelNotConfirmed);
    let env = CountingEnv::new(base.with("LLM_LIVE_CALL_CAP", ""));
    assert_reject(&env, GateReject::BudgetNotConfirmed);

    let recorder = LiveRecorder::new("gate-fail-closed").expect("recorder");
    let record = RunRecord {
        schema_version: "llm.live.v2".into(),
        endpoint_host: "api.deepseek.com".into(),
        model: "test-model".into(),
        started_at: "2026-11-21T00:00:00Z".into(),
        ended_at: "2026-11-21T00:00:01Z".into(),
        tz_offset: "+08:00".into(),
        commit_short: "abcdef0".into(),
        config: "samples=30".into(),
        per_task: 30,
        sub_status: vec![SubStatus {
            item: "c1".into(),
            status: SubStatusState::NotMeasured,
            reason_code: Some("endpoint_path_unconfirmed".into()),
        }],
        samples_path: "samples.jsonl".into(),
        stats_rule: "nearest-rank".into(),
        build: "test".into(),
        path_confirmed: false,
        call_limit: 0,
        calls_used: 0,
        budget_hit: false,
        model_confirmed: false,
        host_locked: true,
        data_terms_declaration: TERMS_DECLARATION.into(),
        data_terms_accepted_at: "2026-11-21T00:00:00+08:00".into(),
        credential_rotated: false,
        tls_trust_root: None,
    };
    recorder.write_run(&record).expect("记录");
    let raw = std::fs::read_to_string(recorder.root().join("run.json")).expect("读取");
    assert!(!raw.contains("candidate-pending"));
    let _ = std::fs::remove_dir_all(recorder.root());
}

#[test]
fn gate_never_reads_credentials_on_reject() {
    let _guard = COUNTER_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let (before_construct, before_expose) = secret_read_counters();
    let env = CountingEnv::new(
        locked_env("test-model", "test-model")
            .with("LLM_BASE_URL", "https://evil.api.deepseek.com"),
    );
    let _ = admit(&env, true);
    assert!(env.gets() > 0);
    let (after_construct, after_expose) = secret_read_counters();
    assert_eq!(after_construct, before_construct);
    assert_eq!(after_expose, before_expose);
}
fn thinking_request() -> LlmGenerationRequest {
    LlmGenerationRequest::new(
        TaskId::new("req-thinking").expect("id"),
        RedactedPayload::new("上下文", LlmTask::Summary, 8_000).expect("payload"),
        Some(SummaryPurpose::RollingWindow),
        OutputBudget::new(512).expect("output"),
        TemperatureMilli::new(200).expect("temperature"),
        DeadlineMs::new(8_000).expect("deadline"),
    )
    .expect("request")
}

#[test]
fn thinking_is_fixed_disabled() {
    let request = thinking_request();
    for stream in [false, true] {
        let body = request_body("test-model", &request, stream);
        assert_eq!(body[THINKING_FIELD]["type"], "disabled");
        assert!(thinking_verdict(&body).is_none());

        let mut missing = body.clone();
        missing
            .as_object_mut()
            .expect("对象")
            .remove(THINKING_FIELD);
        assert_eq!(thinking_verdict(&missing), Some(THINKING_NOT_DISABLED));

        let mut wrong = body;
        wrong[THINKING_FIELD]["type"] = serde_json::json!("enabled");
        assert_eq!(thinking_verdict(&wrong), Some(THINKING_NOT_DISABLED));
    }
}

#[test]
fn path_confirmed_derived_from_constant() {
    assert!(path_confirmed(PATH_CONSTANT));
    for path in ["/", "/v1/chat/completions", "", "/chat/completions/"] {
        assert!(!path_confirmed(path), "{path:?}");
    }

    let env = CountingEnv::new(locked_env("test-model", "test-model").with(
        "LLM_BASE_URL",
        "https://api.deepseek.com/v1/chat/completions",
    ));
    match admit(&env, true) {
        Admission::Admit(ticket) => assert!(!ticket.path_confirmed),
        other => panic!("应放行：{other:?}"),
    }
    let env = CountingEnv::new(
        locked_env("test-model", "test-model")
            .with("LLM_BASE_URL", "https://api.deepseek.com/chat/completions"),
    );
    match admit(&env, true) {
        Admission::Admit(ticket) => assert!(ticket.path_confirmed),
        other => panic!("应放行：{other:?}"),
    }
}

#[test]
fn allowlist_cannot_widen_hosts() {
    assert_eq!(DEEPSEEK_HOSTS.len(), 1);
    assert_eq!(
        host_is_locked("https://evil.api.deepseek.com/v1", DEEPSEEK_HOSTS),
        HostVerdict::NotAllowed
    );
    let env = CountingEnv::new(
        locked_env("test-model", "test-model")
            .with("LLM_ALLOWLIST", "evil.api.deepseek.com")
            .with("LLM_BASE_URL", "https://evil.api.deepseek.com/v1"),
    );
    assert_reject(&env, GateReject::EndpointNotAllowed);
}
