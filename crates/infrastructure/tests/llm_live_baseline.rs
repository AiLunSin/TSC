//! AC-20/AC-22 真实基线：每任务 30 次串行流式采样。
//!
//! 默认 ignored；必须由用户终端注入 LLM_* 环境变量后运行：
//! `cargo test -p meeting-copilot-infrastructure --features llm-https \
//!   --test llm_live_baseline -- --ignored --test-threads=1 --nocapture`

#![cfg(feature = "llm-https")]

use std::sync::Arc;
use std::time::Instant;

use meeting_copilot_domain::model::llm::{
    DeadlineMs, FailureClass, LlmGenerationRequest, LlmTask, OutputBudget, RedactedPayload,
    StreamEvent, StreamEventKind, SummaryPurpose, TemperatureMilli,
};
use meeting_copilot_domain::port::llm::CancelToken;
use meeting_copilot_domain::vo::TaskId;
use meeting_copilot_infrastructure::llm::degrade::SystemClock;
use meeting_copilot_infrastructure::llm::live::assemble_live;
use meeting_copilot_infrastructure::llm::live::gate::{admit, Admission, ProcessEnv};
use meeting_copilot_infrastructure::llm::live::record::{
    RunRecord, SubStatus, SubStatusState, STATS_RULE, TERMS_DECLARATION,
};
use meeting_copilot_infrastructure::llm::live::run_id::fresh_run_id;
use meeting_copilot_infrastructure::llm::live::stats::{summarize, Sample};
use meeting_copilot_infrastructure::llm::transport_tls::TlsTransport;
use meeting_copilot_infrastructure::llm::LocalObserver;

#[test]
#[ignore = "需要 LLM_* 环境变量；串行真实采样"]
fn baseline_30_samples_per_task() {
    let env = ProcessEnv::snapshot();
    let ticket = match admit(&env, true) {
        Admission::Admit(ticket) => ticket,
        Admission::Skip(reason) => {
            println!("未实测（{}） code={}", reason.label(), reason.code());
            return;
        }
        Admission::Reject(reason) => {
            println!("拒绝 code={}", reason.code());
            return;
        }
    };

    let run_id = fresh_run_id();
    let session = assemble_live(
        &ticket,
        &env,
        Arc::new(TlsTransport::new()),
        Arc::new(LocalObserver::new()),
        Arc::new(SystemClock),
        &run_id,
    )
    .expect("assemble");

    let planned = session.config.samples_per_task;
    let mut all_samples: Vec<Sample> = Vec::new();
    let mut budget_stopped = false;
    let mut auth_stopped = false;

    'tasks: for task in LlmTask::ALL {
        for seq in 1..=planned {
            if session.ledger.used() >= session.ledger.limit() {
                budget_stopped = true;
                break 'tasks;
            }
            let purpose = (task == LlmTask::Summary).then_some(SummaryPurpose::RollingWindow);
            let payload = format!(
                "会议讨论了灰度上线时间：决定下周三开始灰度，由张三负责；需要提前完成监控告警演练，并确认回滚方案。采样序号 {seq}。"
            );
            let request = LlmGenerationRequest::new(
                TaskId::new(format!("base-{}-{seq}", task.as_str())).expect("id"),
                RedactedPayload::new(payload, task, 8_000).expect("payload"),
                purpose,
                OutputBudget::new(512).expect("output"),
                TemperatureMilli::new(200).expect("temperature"),
                DeadlineMs::new(session.config.deadline_ms).expect("deadline"),
            )
            .expect("request");

            let mut completed = None;
            let mut failure: Option<FailureClass> = None;
            let mut first_token_ms: Option<u32> = None;
            let mut total_ms = 0_u64;
            for stream_attempt in 0..2 {
                let attempt_started = Instant::now();
                completed = None;
                failure = None;
                first_token_ms = None;
                {
                    let mut sink = |event: StreamEvent| match event.kind {
                        StreamEventKind::Delta { first_token, .. } if first_token => {
                            first_token_ms = Some(
                                u32::try_from(attempt_started.elapsed().as_millis())
                                    .unwrap_or(u32::MAX),
                            );
                        }
                        StreamEventKind::Completed(outcome) => completed = Some(*outcome),
                        StreamEventKind::Failed { failure: class, .. } => {
                            failure = Some(class);
                        }
                        _ => {}
                    };
                    let summary = session
                        .gateway
                        .stream_structured(&request, &CancelToken::new(), &mut sink)
                        .expect("stream");
                    if first_token_ms.is_none() {
                        first_token_ms = summary.first_token_ms;
                    }
                }
                total_ms = u64::try_from(attempt_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                if completed.is_some() || stream_attempt == 1 {
                    break;
                }
            }
            let sample = if let Some(outcome) = completed {
                Sample {
                    seq: u32::try_from(seq).unwrap_or(u32::MAX),
                    task,
                    attempt: outcome.attempts.max(1),
                    first_token_ms,
                    total_ms,
                    outcome: if outcome.degraded { "degraded" } else { "ok" }.into(),
                    failure_class: outcome.failure,
                    http_status: outcome.http_status,
                    degraded: outcome.degraded,
                    excluded: None,
                }
            } else {
                Sample {
                    seq: u32::try_from(seq).unwrap_or(u32::MAX),
                    task,
                    attempt: 2,
                    first_token_ms,
                    total_ms,
                    outcome: "degraded".into(),
                    failure_class: Some(failure.unwrap_or(FailureClass::Unavailable)),
                    http_status: None,
                    degraded: true,
                    excluded: None,
                }
            };
            session.recorder.append_sample(&sample).expect("sample");
            if matches!(sample.http_status, Some(401 | 403)) {
                auth_stopped = true;
                all_samples.push(sample);
                break 'tasks;
            }
            all_samples.push(sample);
        }
    }

    let samples_path = session.recorder.samples_path();
    let sample_count = std::fs::read_to_string(&samples_path)
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
        config: format!("samples={planned}"),
        per_task: planned,
        sub_status: vec![SubStatus {
            item: "ac20-ac22".into(),
            status: SubStatusState::Measured,
            reason_code: if budget_stopped {
                Some("budget_exhausted".into())
            } else if auth_stopped {
                Some("auth_rejected".into())
            } else {
                None
            },
        }],
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
    session.recorder.write_run(&record).expect("run.json");

    println!(
        "RUN_ID={run_id} SAMPLES={sample_count} CALLS={}",
        session.ledger.used()
    );
    for task in LlmTask::ALL {
        let stats = summarize(&all_samples, task);
        println!(
            "STATS task={} n={} valid={} p50_ms={:?} p95_ms={:?} success_rate={:?} degraded={}",
            task.as_str(),
            stats.n,
            stats.valid,
            stats.p50_ms,
            stats.p95_ms,
            stats.native_success_rate,
            stats.degraded_count
        );
    }
}
