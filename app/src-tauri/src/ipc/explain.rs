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
/// 多句/整段的判定门槛：太短的"好的。收到。"仍按术语处理。
const PASSAGE_MIN_CHARS: usize = 24;
const PASSAGE_MIN_SENTENCES: usize = 2;
const MAX_TOKENS_TERM: u16 = 320;
const MAX_TOKENS_PASSAGE: u16 = 420;

/// 术语/短语：先给在此处的意思，再补通用含义。
const SYSTEM_PROMPT_TERM: &str = "你是会议实时助手。用户会给你一段在会议转写里选中的词或短语，\
以及它前后的上下文。用简体中文回答，总长不超过 120 字，纯文本、不要 Markdown、不要罗列项目符号：\
先用一句话说明它在这里指什么；若是专业术语、缩写或产品名，补充它通常的含义（确定的英文全称可以给出）；\
若上下文能看出它在讨论里的作用，用半句话点出来。不要复述原文，不要编造数字或结论，不确定就直说。";

/// 多句/整段：概括 + 关键信息，不逐句翻译。
const SYSTEM_PROMPT_PASSAGE: &str = "你是会议实时助手。用户选中的是多句或整段会议转写，\
可能带识别错字与口语重复。用简体中文回答，总长不超过 180 字，纯文本、不要 Markdown、不要罗列项目符号：\
先用一句话概括这段在讲什么；再点出最多两个关键信息（数字、结论、待办、争议点）；\
若出现术语或缩写，顺带用半句话解释。不要逐句翻译或逐词解释，不要复述原文，不确定就直说。";

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

    let system = system_prompt_for(text);
    let max_tokens = if is_passage(text) {
        MAX_TOKENS_PASSAGE
    } else {
        MAX_TOKENS_TERM
    };
    let user = build_user_message(text, &context);
    match call_llm(&secret, &model, system, &user, max_tokens) {
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

/// 是否是"多句/整段"：句数够且总长够，避免把"好的。收到。"当整段处理。
fn is_passage(text: &str) -> bool {
    text.chars().count() >= PASSAGE_MIN_CHARS
        && crate::ipc::transcript::split_sentences(text).len() >= PASSAGE_MIN_SENTENCES
}

fn system_prompt_for(text: &str) -> &'static str {
    if is_passage(text) {
        SYSTEM_PROMPT_PASSAGE
    } else {
        SYSTEM_PROMPT_TERM
    }
}

/// 请求体里显式标出"多句"，让模型知道该概括而不是逐词解释。
fn build_user_message(text: &str, context: &str) -> String {
    let label = if is_passage(text) {
        "选中（多句）"
    } else {
        "选中"
    };
    if context.is_empty() {
        format!("{label}：{text}")
    } else {
        format!("{label}：{text}\n上下文：{context}")
    }
}

fn local_fallback(transcripts: &TranscriptLog, text: &str, note: &str) -> ExplainResultDto {
    let hits = transcripts.hits(text);
    let context = transcripts.context_for(text);
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
fn call_llm(
    secret: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u16,
) -> Result<String, IpcErrorDto> {
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
        "max_tokens": max_tokens,
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
    _max_tokens: u16,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_exchange_stays_term_mode() {
        assert!(!is_passage("好的。收到。"));
        assert_eq!(system_prompt_for("好的。收到。"), SYSTEM_PROMPT_TERM);
    }

    #[test]
    fn long_multi_sentence_selection_uses_passage_prompt() {
        let text = "我们这次灰度范围限定在华中两个站点。先看 SLA 指标，再决定要不要扩到华东。";
        assert!(is_passage(text));
        assert_eq!(system_prompt_for(text), SYSTEM_PROMPT_PASSAGE);
        assert!(build_user_message(text, "前一句 后一句").contains("选中（多句）"));
    }

    #[test]
    fn term_message_carries_context() {
        let message = build_user_message("灰度", "前面一句 后面一句");
        assert!(message.starts_with("选中：灰度"), "{message}");
        assert!(message.contains("上下文：前面一句 后面一句"), "{message}");
    }

    #[test]
    fn empty_context_omits_context_line() {
        assert!(!build_user_message("SLA", "").contains("上下文"));
    }
}
