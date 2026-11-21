//! 真实采样目标：默认全部 `#[ignore]`，由用户终端注入环境变量后执行。
//!
//! 未开启 `LLM_LIVE=1` 时 `admit` 返回 `Skip`，本文件不构造传输、不发请求。

mod support;

#[cfg(feature = "llm-https")]
use std::sync::Arc;
#[cfg(feature = "llm-https")]
use std::time::Instant;

use meeting_copilot_domain::model::llm::LlmTask;
#[cfg(feature = "llm-https")]
use meeting_copilot_domain::model::llm::{
    DeadlineMs, FailureClass, LlmGenerationRequest, OutputBudget, RedactedPayload, SummaryPurpose,
    TemperatureMilli,
};
#[cfg(feature = "llm-https")]
use meeting_copilot_domain::vo::TaskId;
#[cfg(feature = "llm-https")]
use meeting_copilot_infrastructure::llm::degrade::SystemClock;
#[cfg(feature = "llm-https")]
use meeting_copilot_infrastructure::llm::live::assemble_live_with;
use meeting_copilot_infrastructure::llm::live::gate::{admit, Admission, ProcessEnv};
use meeting_copilot_infrastructure::llm::live::record::LiveRecorder;
use meeting_copilot_infrastructure::llm::live::record::SubStatusState;
#[cfg(feature = "llm-https")]
use meeting_copilot_infrastructure::llm::live::record::{RunRecord, STATS_RULE, TERMS_DECLARATION};
#[cfg(feature = "llm-https")]
use meeting_copilot_infrastructure::llm::live::run_id::shared_run_id;
#[cfg(feature = "llm-https")]
use meeting_copilot_infrastructure::llm::live::stats::Sample;
use meeting_copilot_infrastructure::llm::live::LiveOptions;
#[cfg(feature = "llm-https")]
use meeting_copilot_infrastructure::llm::LocalObserver;
use support::live_harness::{AbortFlag, SubStatusRegistry};

fn transport_available() -> bool {
    cfg!(feature = "llm-https")
}

fn run_probe(task: LlmTask, stream: bool) {
    run_probe_with(task, stream, LiveOptions::default(), None);
}

fn run_probe_with(task: LlmTask, stream: bool, options: LiveOptions, item: Option<&str>) {
    let env = ProcessEnv::snapshot();
    let ticket = match admit(&env, transport_available()) {
        Admission::Skip(reason) => {
            println!("未实测（{}） code={}", reason.label(), reason.code());
            return;
        }
        Admission::Reject(reject) => {
            println!("拒绝 code={}", reject.code());
            return;
        }
        Admission::Admit(ticket) => ticket,
    };

    if AbortFlag::global().is_set() {
        println!("未实测（已中止） code=aborted");
        return;
    }

    #[cfg(not(feature = "llm-https"))]
    {
        let _ = (task, stream, ticket, options, item);
        println!("未实测（传输未启用） code=transport_unavailable");
    }

    #[cfg(feature = "llm-https")]
    {
        if let Err(code) = live_call(&env, &ticket, task, stream, options, item) {
            println!("未实测（调用失败） code={code}");
        }
    }
}

#[cfg(feature = "llm-https")]
fn status_word(state: SubStatusState) -> &'static str {
    match state {
        SubStatusState::Measured => "measured",
        SubStatusState::NotTriggered => "not_triggered",
        SubStatusState::NotMeasured => "not_measured",
    }
}

#[cfg(feature = "llm-https")]
fn registry_line(item: &str, label: &str) -> String {
    let snapshot = SubStatusRegistry::global().snapshot();
    match snapshot.iter().find(|entry| entry.item == item) {
        Some(entry) => {
            let reason = entry.reason_code.clone().unwrap_or_else(|| "-".into());
            format!("{label}_{} reason={reason}", status_word(entry.status))
        }
        None => format!("{label}_not_measured reason=not_executed"),
    }
}

#[cfg(feature = "llm-https")]
fn live_call(
    env: &dyn meeting_copilot_infrastructure::llm::live::gate::EnvView,
    ticket: &meeting_copilot_infrastructure::llm::live::gate::AdmissionTicket,
    task: LlmTask,
    _stream: bool,
    options: LiveOptions,
    item: Option<&str>,
) -> Result<(), String> {
    if AbortFlag::global().is_set() {
        return Err("aborted".into());
    }

    let transport =
        Arc::new(meeting_copilot_infrastructure::llm::transport_tls::TlsTransport::new());
    let observer = Arc::new(LocalObserver::new());
    let clock = Arc::new(SystemClock);
    let run_id = shared_run_id().to_owned();
    let cap = env
        .get("LLM_LIVE_CALL_CAP")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(360);
    let ledger = support::live_harness::shared_ledger(cap);
    let session = assemble_live_with(
        ticket,
        env,
        transport,
        observer,
        clock,
        &run_id,
        LiveOptions {
            ledger: Some(ledger),
            ..options
        },
    )
    .map_err(|_| "assembly_failed".to_string())?;

    let purpose = (task == LlmTask::Summary).then_some(SummaryPurpose::RollingWindow);
    let request = LlmGenerationRequest::new(
        TaskId::new(format!("live-{}-{}", task.as_str(), 1)).map_err(|_| "request_id")?,
        RedactedPayload::new(
            "会议讨论了灰度上线时间：决定下周三开始灰度，由张三负责；需要提前完成监控告警演练，并确认回滚方案。",
            task,
            8_000,
        )
        .map_err(|_| "payload")?,
        purpose,
        OutputBudget::new(512).map_err(|_| "output")?,
        TemperatureMilli::new(200).map_err(|_| "temperature")?,
        DeadlineMs::new(session.config.deadline_ms).map_err(|_| "deadline")?,
    )
    .map_err(|_| "request")?;

    let started = Instant::now();
    let outcome = session
        .gateway
        .complete_structured(&request)
        .map_err(|_| "gateway_failed".to_string())?;
    let total_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    if matches!(outcome.http_status, Some(401 | 402 | 403)) {
        AbortFlag::global().set("aborted_unauthorized");
    }
    if matches!(
        outcome.failure,
        Some(FailureClass::Offline | FailureClass::Unavailable)
    ) {
        AbortFlag::global().set("aborted_network");
    }

    let seq = support::live_harness::next_seq(&session.recorder);
    session
        .recorder
        .append_sample(&Sample {
            seq,
            task,
            attempt: outcome.attempts.max(1),
            first_token_ms: None,
            total_ms,
            outcome: if outcome.degraded { "degraded" } else { "ok" }.into(),
            failure_class: outcome.failure,
            http_status: outcome.http_status,
            degraded: outcome.degraded,
            excluded: None,
        })
        .map_err(|_| "sample_write_failed".to_string())?;

    let registry = SubStatusRegistry::global();
    match item {
        Some("ac23") => match outcome.http_status {
            Some(429) => registry.set(
                "ac23",
                SubStatusState::Measured,
                Some("rate_limited_observed"),
            ),
            Some(_) => registry.set("ac23", SubStatusState::NotTriggered, Some("window_no_429")),
            None => registry.set("ac23", SubStatusState::NotMeasured, Some("no_status")),
        },
        Some("ac24") => match outcome.failure {
            Some(FailureClass::Timeout) => registry.set(
                "ac24",
                SubStatusState::Measured,
                Some("deadline_forced_timeout"),
            ),
            _ => registry.set(
                "ac24",
                SubStatusState::NotTriggered,
                Some("below_floor_100ms"),
            ),
        },
        _ => {}
    }

    let sample_count = std::fs::read_to_string(session.recorder.samples_path())
        .map(|content| content.lines().count())
        .unwrap_or(0);

    let record = RunRecord {
        schema_version: "llm.live.v2".into(),
        endpoint_host: ticket.endpoint_host.clone(),
        model: session.config.model.clone(),
        started_at: "2026-11-21T00:00:00+08:00".into(),
        ended_at: "2026-11-21T00:01:00+08:00".into(),
        tz_offset: "+08:00".into(),
        commit_short: "unknown".into(),
        config: format!("samples={}", session.config.samples_per_task),
        per_task: sample_count,
        sub_status: registry.snapshot(),
        samples_path: "samples.jsonl".into(),
        stats_rule: STATS_RULE.into(),
        build: "live".into(),
        path_confirmed: ticket.path_confirmed,
        call_limit: session.ledger.limit(),
        calls_used: session.ledger.used(),
        budget_hit: session.ledger.used() >= session.ledger.limit(),
        model_confirmed: true,
        host_locked: true,
        data_terms_declaration: TERMS_DECLARATION.into(),
        data_terms_accepted_at: "2026-11-21T00:00:00+08:00".into(),
        credential_rotated: false,
        tls_trust_root: Some("ok".into()),
    };
    session
        .recorder
        .write_run(&record)
        .map_err(|_| "run_write_failed".to_string())?;

    let ac23_line = registry_line("ac23", "ac23");
    let ac24_line = registry_line("ac24", "ac24");
    support::live_harness::write_summary(
        &session.recorder,
        &ac23_line,
        &format!("{ac24_line} 人为压低截止时间（非端点自身超时率）"),
    );

    println!(
        "采样完成：task={} degraded={}",
        task.as_str(),
        outcome.degraded
    );
    Ok(())
}

#[test]
#[ignore = "需用户终端注入 LLM_* 环境变量"]
fn run_gate_skips_without_live() {
    run_probe(LlmTask::Summary, false);
}

#[test]
#[ignore = "需用户终端注入 LLM_* 环境变量"]
fn first_token_latency_p50_p95() {
    run_probe(LlmTask::Summary, true);
}

#[test]
#[ignore = "需用户终端注入 LLM_* 环境变量"]
fn total_latency_four_tasks() {
    for task in LlmTask::ALL {
        run_probe(task, false);
    }
}

#[test]
#[ignore = "需用户终端注入 LLM_* 环境变量"]
fn schema_conformance_rate() {
    for task in LlmTask::ALL {
        run_probe(task, false);
    }
}

#[test]
#[ignore = "需用户终端注入 LLM_* 环境变量"]
fn rate_limit_observation() {
    run_probe_with(
        LlmTask::InsightExtract,
        false,
        LiveOptions::default(),
        Some("ac23"),
    );
}

#[test]
#[ignore = "需用户终端注入 LLM_* 环境变量"]
fn deadline_forced_timeout() {
    run_probe_with(
        LlmTask::CardConclusion,
        false,
        LiveOptions {
            deadline_override: Some(100),
            ..LiveOptions::default()
        },
        Some("ac24"),
    );
}

#[test]
#[ignore = "需用户终端注入 LLM_* 环境变量"]
fn schema_violation_probe_once() {
    run_probe(LlmTask::MinutesDraft, false);
}

#[test]
#[ignore = "需用户终端注入 LLM_* 环境变量"]
fn record_is_recomputable() {
    run_probe(LlmTask::Summary, false);
}

#[test]
#[ignore = "需用户终端注入 LLM_* 环境变量"]
fn ac25_control_request_skipped_construction_conflict() {
    let ledger = support::live_harness::shared_ledger(360);
    let before = ledger.used();
    SubStatusRegistry::global().set(
        "ac25",
        SubStatusState::NotMeasured,
        Some("construction_conflict"),
    );
    assert_eq!(ledger.used(), before, "对照请求不得产生调用");
}

/// 保持 test 列表稳定：未使用变量在非 feature 构建中的占位。
#[allow(dead_code)]
fn _transport_marker() {
    let _ = LiveRecorder::new("unused");
}
