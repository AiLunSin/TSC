//! 模型调用公共层：一处拼请求，释义、辅助回复等场景共用。
//!
//! 只在调用方已确认"用户主动触发"后使用；这里不缓存、不落盘、不写日志正文。

#[cfg(feature = "llm-https")]
use std::sync::Arc;

use crate::ipc::dto::IpcErrorDto;
use crate::ipc::state::AppState;

/// 请求超时：短于探针，避免用户等太久。
#[cfg(feature = "llm-https")]
pub(crate) const TIMEOUT_MS: u64 = 20_000;

/// 读模型 Key：优先系统安全存储，失败直接报可重试错误。
pub(crate) fn read_secret(state: &AppState) -> Result<Option<String>, IpcErrorDto> {
    let stored = state
        .secure
        .read()
        .map_err(|_| IpcErrorDto::new("secret_unavailable", "凭据存储不可用。", true))?;
    Ok(stored
        .map(|bytes| bytes.with_bytes(|raw| String::from_utf8_lossy(raw).into_owned()))
        .filter(|secret| !secret.trim().is_empty()))
}

/// 当前配置的模型名；配置越界时退回第一个预设。
pub(crate) fn current_model(state: &AppState) -> Result<String, IpcErrorDto> {
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

#[cfg(feature = "llm-https")]
pub(crate) fn call_llm(
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
pub(crate) fn call_llm(
    _secret: &str,
    _model: &str,
    _system: &str,
    _user: &str,
    _max_tokens: u16,
) -> Result<String, IpcErrorDto> {
    Err(IpcErrorDto::new(
        "llm_transport_off",
        "当前构建未启用模型传输。",
        true,
    ))
}
