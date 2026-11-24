//! 选中文字查意思：优先走已配置的模型，无 Key 时回退到本机上下文。
//!
//! 合规边界：只有用户**主动选中并点击**才会把这段文字连同前后各一句发出去；
//! 不做自动外发，不发整段转写。未配置 Key 时完全不出网。

use std::sync::Arc;

use tauri::State;

use crate::ipc::card_store::CardStore;
use crate::ipc::dto::{ExplainInputDto, ExplainResultDto, IngestExplanationInputDto, IpcErrorDto};
use crate::ipc::state::AppState;
use crate::ipc::transcript::TranscriptLog;

/// 单次选中上限：再长就不是"查词"了，也避免把整段会议发出去。
const MAX_SELECTION: usize = 200;
/// 上下文上限：只取前后各一句，且总量封顶。
const MAX_CONTEXT: usize = 600;
/// 请求超时：短于探针，避免用户等太久。
#[cfg(feature = "llm-https")]
const TIMEOUT_MS: u64 = 20_000;

const SYSTEM_PROMPT: &str = "你是会议实时助手。用户会给你一段在会议转写中选中的文字，\
以及它前后的上下文。请用简体中文回答，总长不超过 120 字，不要用 Markdown：\
（1）先说明它在这里的意思；（2）若是专业术语或缩写，补充它通常指什么；\
（3）若与上下文相关，指出它在当前讨论中的作用。不要编造事实，不确定就直说不确定。";

/// 释义入口：校验 → 取凭据 → 出网或本地回退。
pub fn explain_impl(
    state: &AppState,
    transcripts: &TranscriptLog,
    input: ExplainInputDto,
) -> Result<ExplainResultDto, IpcErrorDto> {
    let text = input.text.trim();
    if text.is_empty() {
        return Err(IpcErrorDto::new(
            "empty_selection",
            "请先选中一段文字。",
            false,
        ));
    }
    if text.chars().count() > MAX_SELECTION {
        return Err(IpcErrorDto::new(
            "selection_too_long",
            "选中的文字过长，请控制在 200 字以内。",
            false,
        ));
    }
    let given: String = input.context.trim().chars().take(MAX_CONTEXT).collect();
    let context = if given.is_empty() {
        transcripts.context_around(text)
    } else {
        given
    };

    let secret = read_secret(state)?;
    let Some(secret) = secret else {
        return Ok(local_fallback(
            transcripts,
            text,
            "尚未配置 API Key，以下仅为本机上下文。",
        ));
    };
    let model = current_model(state)?;

    let user = if context.is_empty() {
        format!("选中：{text}")
    } else {
        format!("选中：{text}\n上下文：{context}")
    };
    match call_llm(&secret, &model, SYSTEM_PROMPT, &user) {
        Ok(explanation) => Ok(ExplainResultDto {
            text: text.to_owned(),
            explanation,
            source: "llm".to_owned(),
        }),
        // 出网失败但仍能给出本机上下文时，不让用户空手而归。
        Err(error) if error.retryable => Ok(local_fallback(
            transcripts,
            text,
            &format!("{} 以下仅为本机上下文。", error.message),
        )),
        Err(error) => Err(error),
    }
}

fn read_secret(state: &AppState) -> Result<Option<String>, IpcErrorDto> {
    let stored = state
        .secure
        .read()
        .map_err(|_| IpcErrorDto::new("secret_unavailable", "凭据存储不可用。", true))?;
    Ok(stored
        .map(|bytes| bytes.with_bytes(|raw| String::from_utf8_lossy(raw).into_owned()))
        .filter(|secret| !secret.trim().is_empty()))
}

fn current_model(state: &AppState) -> Result<String, IpcErrorDto> {
    let config = state
        .config
        .load()
        .unwrap_or_default()
        .normalized(state.catalog.model_count(), state.catalog.endpoint_count());
    state
        .catalog
        .presets
        .get(config.model_index)
        .or_else(|| state.catalog.presets.first())
        .cloned()
        .ok_or_else(|| IpcErrorDto::new("model_missing", "未配置可用的模型。", false))
}

fn local_fallback(transcripts: &TranscriptLog, text: &str, note: &str) -> ExplainResultDto {
    let lines = transcripts.snapshot();
    let hits = lines.iter().filter(|line| line.text.contains(text)).count();
    let context = transcripts.context_around(text);
    let explanation = if hits == 0 {
        format!("{note}本次转写里还没有出现这段文字。")
    } else if context.is_empty() {
        format!("{note}该内容在本次转写里出现 {hits} 次。")
    } else {
        format!("{note}该内容在本次转写里出现 {hits} 次。相关上下文：{context}")
    };
    ExplainResultDto {
        text: text.to_owned(),
        explanation,
        source: "local".to_owned(),
    }
}

#[cfg(feature = "llm-https")]
fn call_llm(secret: &str, model: &str, system: &str, user: &str) -> Result<String, IpcErrorDto> {
    use std::time::{Duration, Instant};

    use meeting_copilot_domain::port::llm::CancelToken;
    use meeting_copilot_infrastructure::llm::live::{
        BearerTransport, SecretString, DEEPSEEK_HOSTS, PATH_CONSTANT,
    };
    use meeting_copilot_infrastructure::llm::transport::{HttpRequest, HttpTransport};
    use meeting_copilot_infrastructure::llm::transport_tls::TlsTransport;

    let transport: Arc<dyn HttpTransport> = Arc::new(BearerTransport::new(
        Arc::new(TlsTransport::new()),
        SecretString::new(secret.to_owned()),
    ));
    let body = serde_json::to_vec(&serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "thinking": {"type": "disabled"},
        "max_tokens": 320,
        "temperature": 0.3,
    }))
    .map_err(|_| IpcErrorDto::new("llm_bad_request", "请求构造失败。", false))?;
    let headers = vec![
        ("Content-Type".to_owned(), "application/json".to_owned()),
        ("Accept".to_owned(), "application/json".to_owned()),
    ];
    let host = DEEPSEEK_HOSTS.first().copied().unwrap_or_default();
    let mut request = HttpRequest::json_post(host, 443, PATH_CONSTANT, headers, body);
    request.secure = true;
    let cancel = CancelToken::new();
    let deadline = Instant::now() + Duration::from_millis(TIMEOUT_MS);
    let response = transport
        .send(&request, deadline, &cancel)
        .map_err(|_| IpcErrorDto::new("llm_unreachable", "无法连接模型服务，请检查网络。", true))?;
    match response.status {
        200..=299 => {}
        401 | 403 => {
            return Err(IpcErrorDto::new(
                "llm_auth_failed",
                "API Key 无效或已失效，请在设置页重新测试。",
                false,
            ));
        }
        402 => {
            return Err(IpcErrorDto::new(
                "llm_quota_exceeded",
                "账户额度不足，请检查模型账户。",
                false,
            ));
        }
        429 => {
            return Err(IpcErrorDto::new(
                "llm_rate_limited",
                "请求过于频繁，请稍后再试。",
                true,
            ));
        }
        _ => {
            return Err(IpcErrorDto::new(
                "llm_failed",
                "模型服务返回异常，请稍后再试。",
                true,
            ));
        }
    }
    let value: serde_json::Value = serde_json::from_slice(&response.body)
        .map_err(|_| IpcErrorDto::new("llm_bad_response", "模型返回内容无法解析。", true))?;
    let content = value["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim();
    if content.is_empty() {
        return Err(IpcErrorDto::new(
            "llm_empty",
            "模型没有返回内容，请重试。",
            true,
        ));
    }
    Ok(content.to_owned())
}

#[cfg(not(feature = "llm-https"))]
fn call_llm(
    _secret: &str,
    _model: &str,
    _system: &str,
    _user: &str,
) -> Result<String, IpcErrorDto> {
    // 标记为可回退：交给 local_fallback 给出本机上下文，而不是直接报错。
    Err(IpcErrorDto::new(
        "llm_transport_off",
        "当前构建未启用模型传输。",
        true,
    ))
}

/// 选中文字查意思；未配置 Key 或未启用传输时回退到本机上下文。
#[tauri::command]
pub fn explain_selection(
    state: State<'_, AppState>,
    transcripts: State<'_, Arc<TranscriptLog>>,
    input: ExplainInputDto,
) -> Result<ExplainResultDto, IpcErrorDto> {
    explain_impl(state.inner(), transcripts.inner(), input)
}

/// 把一次释义落成卡片，进入现有卡片列表。
#[tauri::command]
pub fn ingest_explanation(
    state: State<'_, Arc<CardStore>>,
    input: IngestExplanationInputDto,
) -> Result<usize, IpcErrorDto> {
    state.ingest_explanation(&input.word, &input.explanation)
}
