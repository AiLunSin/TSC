//! 真实联调载体（阶段 B）：默认关闭，零公网依赖。
//!
//! 未开启 `LLM_LIVE=1` 时不构造传输、不发起任何请求。

pub mod auth;
pub mod budget;
pub mod config;
pub mod gate;
pub mod policy;
pub mod record;
pub mod run_id;
pub mod stats;

use super::transport::HttpTransport;
use std::sync::{Arc, RwLock};

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::llm::LlmTask;
use meeting_copilot_domain::model::provider::DomainPattern;
use meeting_copilot_domain::port::llm::{LlmObserver, StructuredLlmGateway};

use crate::llm::degrade::Clock;
use crate::llm::{assemble_with_clock, CredentialRef, LlmConfig};

pub use auth::BearerTransport;
pub use budget::{BudgetExhausted, BudgetLedger, BudgetedTransport, Permit};
pub use config::{LiveRunConfig, SecretString};
pub use gate::{
    admit, Admission, AdmissionTicket, EnvView, GateReject, LiveDecision, ProcessEnv, SkipReason,
};
pub use policy::{
    host_is_locked, model_confirmed, path_confirmed, scheme_verdict, HostVerdict, Scheme,
    SchemeVerdict, DEEPSEEK_HOSTS, PATH_CONSTANT,
};
pub use record::{LiveRecorder, RunRecord, SubStatus, SubStatusState, STATS_RULE};
pub use stats::{percentile, summarize, Sample, TaskStats};

/// 只有在 [`gate::evaluate`] 返回 `Run` 时才允许构造传输实例。
pub fn transport_if_allowed<T: HttpTransport>(
    decision: LiveDecision,
    build: impl FnOnce() -> T,
) -> Option<T> {
    match decision {
        LiveDecision::Run => Some(build()),
        LiveDecision::Skip(_) => None,
    }
}

/// 联调会话：网关 + 预算台账 + 记录器 + 配置。
pub struct LiveSession {
    pub gateway: Arc<dyn StructuredLlmGateway>,
    pub ledger: Arc<BudgetLedger>,
    pub recorder: Arc<LiveRecorder>,
    pub config: Arc<LiveRunConfig>,
}

/// 真实装配：票据 → 配置（唯一凭据读点）→ Bearer → 预算 → 网关 → 记录器。
#[allow(clippy::too_many_arguments)]
pub fn assemble_live(
    ticket: &AdmissionTicket,
    env: &dyn EnvView,
    transport: Arc<dyn HttpTransport>,
    observer: Arc<dyn LlmObserver>,
    clock: Arc<dyn Clock>,
    run_id: &str,
) -> Result<LiveSession, CopilotError> {
    let config = Arc::new(LiveRunConfig::try_new(env, None, None, None)?);
    let recorder = Arc::new(LiveRecorder::new(run_id)?);
    let ledger = Arc::new(BudgetLedger::new(config.call_budget as u64));
    let bearer = Arc::new(BearerTransport::new(transport, config.secret.clone()));
    let budgeted: Arc<dyn HttpTransport> = Arc::new(BudgetedTransport::new(bearer, ledger.clone()));

    let host_pattern = DomainPattern::new(ticket.endpoint_host.clone()).map_err(|_| {
        CopilotError::ProviderConfigInvalid {
            detail: "endpoint_host: 非法 host".into(),
        }
    })?;
    let llm_config = LlmConfig {
        endpoint: config.endpoint.clone(),
        model: config.model.clone(),
        domain_allowlist: vec![host_pattern],
        request_timeout_ms: config.deadline_ms,
        max_concurrency: 1,
        credential_ref: CredentialRef::new("env:LLM_API_KEY"),
        defaults: [
            LlmTask::InsightExtract.default_budget(),
            LlmTask::Summary.default_budget(),
            LlmTask::CardConclusion.default_budget(),
            LlmTask::MinutesDraft.default_budget(),
        ],
        ..LlmConfig::default()
    };
    llm_config.validate()?;

    let gateway =
        assemble_with_clock(Arc::new(RwLock::new(llm_config)), budgeted, observer, clock)?;
    Ok(LiveSession {
        gateway,
        ledger,
        recorder,
        config,
    })
}
