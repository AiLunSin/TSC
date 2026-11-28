//! 插件桥的 HTTP 传输层：极小的请求解析、鉴权与路由。
//!
//! 只服务本机插件：HTTP/1.1、每连接一个请求、`Connection: close`。
//! 这里刻意不引入 HTTP 框架，避免为一个回环入口扩大依赖面。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{AsrAction, BridgeApi};
use crate::ipc::dto::IpcErrorDto;

/// 请求头上限：正常请求只有几个头。
const MAX_HEADER_BYTES: usize = 16 * 1024;
/// 请求体上限：查义正文 200 字、检索 80 字，64 KiB 已经非常宽松。
const MAX_BODY_BYTES: usize = 64 * 1024;
/// 读超时：客户端连上不发数据时尽快释放线程。
const READ_TIMEOUT: Duration = Duration::from_secs(5);
/// 写超时：查义要等模型、检索要等上游，留足时间。
const WRITE_TIMEOUT: Duration = Duration::from_secs(65);
/// 同时处理的连接数：本机插件通常一次只发一个请求。
const MAX_CONNECTIONS: usize = 8;

/// 一条已解析的请求。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Request {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// 解析失败时直接回给客户端的 HTTP 状态。
type Rejected = u16;

/// 接收连接并逐个分派；连接处理放在独立线程，慢查询不堵住新请求。
pub fn serve(listener: TcpListener, token: String, port: u16, api: Arc<dyn BridgeApi>) {
    let token = Arc::new(token);
    let active = Arc::new(AtomicUsize::new(0));
    for incoming in listener.incoming() {
        let Ok(stream) = incoming else {
            continue;
        };
        if active.load(Ordering::Acquire) >= MAX_CONNECTIONS {
            let mut overloaded = stream;
            let _ = write_response(
                &mut overloaded,
                503,
                &error_body("busy", "插件桥正忙，请稍后重试。"),
            );
            continue;
        }
        active.fetch_add(1, Ordering::AcqRel);
        let token = Arc::clone(&token);
        let api = Arc::clone(&api);
        let worker_active = Arc::clone(&active);
        let spawned = thread::Builder::new()
            .name("plugin-bridge-conn".to_owned())
            .spawn(move || {
                handle_connection(stream, &token, port, api.as_ref());
                worker_active.fetch_sub(1, Ordering::AcqRel);
            });
        if spawned.is_err() {
            active.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

fn handle_connection(mut stream: TcpStream, token: &str, port: u16, api: &dyn BridgeApi) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
    let response = match read_request(&mut stream) {
        Ok(raw) => respond(&raw, token, port, api),
        Err(status) => {
            let (code, message) = match status {
                413 => ("body_too_large", "请求体超过 64 KiB。"),
                431 => ("headers_too_large", "请求头过大。"),
                _ => ("bad_request", "请求格式不正确。"),
            };
            response_bytes(status, &error_body(code, message))
        }
    };
    let _ = stream.write_all(&response);
    let _ = stream.flush();
}

/// 读到完整请求（头 + 按 Content-Length 的正文）。
fn read_request(stream: &mut TcpStream) -> Result<Vec<u8>, Rejected> {
    let mut buffer = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];
    let mut header_end: Option<usize> = None;
    loop {
        if buffer.len() > MAX_HEADER_BYTES + MAX_BODY_BYTES {
            return Err(413);
        }
        let read = stream.read(&mut chunk).map_err(|_| 400u16)?;
        if read == 0 {
            return Err(400);
        }
        buffer.extend_from_slice(&chunk[..read]);
        if header_end.is_none() {
            if let Some(position) = find_header_end(&buffer) {
                if position > MAX_HEADER_BYTES {
                    return Err(431);
                }
                if buffer[..position]
                    .windows(17)
                    .any(|window| window.eq_ignore_ascii_case(b"transfer-encoding"))
                {
                    return Err(400);
                }
                header_end = Some(position + 4);
            } else if buffer.len() > MAX_HEADER_BYTES {
                return Err(431);
            }
        }
        if let Some(end) = header_end {
            let length = content_length(&buffer[..end]).ok_or(400u16)?;
            if length > MAX_BODY_BYTES {
                return Err(413);
            }
            if buffer.len() >= end + length {
                buffer.truncate(end + length);
                return Ok(buffer);
            }
        }
    }
}

/// 解析一条完整请求；测试直接喂字节，生产由 `read_request` 收齐。
fn parse_request(raw: &[u8]) -> Result<Request, Rejected> {
    let position = find_header_end(raw).ok_or(400u16)?;
    if position > MAX_HEADER_BYTES {
        return Err(431);
    }
    let header_bytes = raw.get(..position).ok_or(400u16)?;
    let header_text = std::str::from_utf8(header_bytes).map_err(|_| 400u16)?;
    let body = raw.get(position + 4..).ok_or(400u16)?.to_vec();
    if body.len() > MAX_BODY_BYTES {
        return Err(413);
    }

    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or(400u16)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(400u16)?.to_owned();
    let target = parts.next().ok_or(400u16)?;
    let version = parts.next().ok_or(400u16)?;
    if parts.next().is_some() || !version.starts_with("HTTP/1.") {
        return Err(400);
    }

    let mut headers = Vec::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(400);
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_owned();
        if name.is_empty() {
            return Err(400);
        }
        headers.push((name, value));
    }

    let declared = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .map(|(_, value)| value.parse::<usize>())
        .transpose()
        .map_err(|_| 400u16)?;
    let declared = declared.unwrap_or(0);
    if declared > MAX_BODY_BYTES {
        return Err(413);
    }
    if declared != body.len() {
        return Err(400);
    }

    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if !path.starts_with('/') {
        return Err(400);
    }

    Ok(Request {
        method: method.to_ascii_uppercase(),
        path: path.to_owned(),
        query: query.to_owned(),
        headers,
        body,
    })
}

/// 解析 + 鉴权 + 路由；返回完整 HTTP 响应字节。
pub fn respond(raw: &[u8], token: &str, port: u16, api: &dyn BridgeApi) -> Vec<u8> {
    let request = match parse_request(raw) {
        Ok(request) => request,
        Err(status) => {
            let message = if status == 413 {
                "请求体超过 64 KiB。"
            } else {
                "请求格式不正确。"
            };
            return response_bytes(status, &error_body("bad_request", message));
        }
    };
    if let Err((status, body)) = authorize(&request, token, port) {
        return response_bytes(status, &body);
    }
    let (status, body) = dispatch(api, &request);
    response_bytes(status, &body)
}

fn authorize(request: &Request, token: &str, port: u16) -> Result<(), (u16, Value)> {
    let host = request.header("host").unwrap_or_default();
    let expected = format!("127.0.0.1:{port}");
    let expected_localhost = format!("localhost:{port}");
    if !host.eq_ignore_ascii_case(&expected) && !host.eq_ignore_ascii_case(&expected_localhost) {
        return Err((
            403,
            error_body("forbidden_host", "请求 Host 不是本机回环地址。"),
        ));
    }
    if request.header("origin").is_some() {
        // 浏览器会带 Origin；本桥只给本机进程用，收到就拒绝，顺带挡住 DNS 重绑定。
        return Err((
            403,
            error_body("forbidden_origin", "拒绝来自浏览器的跨站请求。"),
        ));
    }
    let authorization = request.header("authorization").unwrap_or_default();
    let presented = authorization
        .strip_prefix("Bearer ")
        .or_else(|| authorization.strip_prefix("bearer "))
        .unwrap_or_default();
    if !token_matches(presented, token) {
        return Err((401, error_body("unauthorized", "缺少或错误的插件桥令牌。")));
    }
    Ok(())
}

fn token_matches(presented: &str, expected: &str) -> bool {
    let presented = presented.as_bytes();
    let expected = expected.as_bytes();
    if presented.len() != expected.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in presented.iter().zip(expected.iter()) {
        difference |= left ^ right;
    }
    difference == 0
}

fn dispatch(api: &dyn BridgeApi, request: &Request) -> (u16, Value) {
    let known = matches!(
        request.path.as_str(),
        "/v1/status"
            | "/v1/explain"
            | "/v1/cards"
            | "/v1/cards/detail"
            | "/v1/cards/save"
            | "/v1/transcript"
            | "/v1/search"
            | "/v1/assist"
            | "/v1/asr/status"
            | "/v1/asr/start"
            | "/v1/asr/stop"
    );
    if !known {
        return (404, error_body("not_found", "插件桥没有这个接口。"));
    }
    let method_allowed = match request.path.as_str() {
        "/v1/status" | "/v1/cards" | "/v1/transcript" | "/v1/asr/status" => request.method == "GET",
        _ => request.method == "POST",
    };
    if !method_allowed {
        return (405, error_body("method_not_allowed", "请求方法不对。"));
    }

    match request.path.as_str() {
        "/v1/status" => ok(api.status()),
        "/v1/explain" => with_body::<ExplainBody, _>(request, |body| {
            api.explain(&body.text, &body.context)
                .map(|result| json!(result))
        }),
        "/v1/cards" => {
            let limit = limit_param(&request.query).unwrap_or(30).clamp(1, 100);
            match api.cards() {
                Ok(cards) => ok(Ok(json!({
                    "count": cards.len(),
                    "cards": cards.into_iter().take(limit).collect::<Vec<_>>(),
                }))),
                Err(error) => api_error(error),
            }
        }
        "/v1/cards/detail" => with_body::<CardDetailBody, _>(request, |body| {
            api.card_detail(&body.card_id).map(|detail| json!(detail))
        }),
        "/v1/cards/save" => with_body::<SaveCardBody, _>(request, |body| {
            api.save_card(&body.word, &body.explanation, &body.sources)
                .map(|added| json!({ "added": added }))
        }),
        "/v1/transcript" => {
            let limit = limit_param(&request.query).unwrap_or(50).clamp(1, 200);
            match api.transcript() {
                Ok(lines) => {
                    let total = lines.len();
                    let mut tail: Vec<_> = lines.into_iter().rev().take(limit).collect();
                    tail.reverse();
                    ok(Ok(json!({ "total": total, "lines": tail })))
                }
                Err(error) => api_error(error),
            }
        }
        "/v1/search" => with_body::<SearchBody, _>(request, |body| {
            api.search(&body.query).map(|response| json!(response))
        }),
        "/v1/assist" => with_body::<AssistBody, _>(request, |body| {
            api.assist(&body.focus).map(|result| json!(result))
        }),
        "/v1/asr/status" => ok(api.asr(AsrAction::Status)),
        "/v1/asr/start" => ok(api.asr(AsrAction::Start)),
        "/v1/asr/stop" => ok(api.asr(AsrAction::Stop)),
        _ => (404, error_body("not_found", "插件桥没有这个接口。")),
    }
}

/// 把 `Result` 转成响应；API 错误统一 400，错误码与桌面端一致。
fn ok(result: Result<Value, IpcErrorDto>) -> (u16, Value) {
    match result {
        Ok(value) => (200, value),
        Err(error) => api_error(error),
    }
}

fn api_error(error: IpcErrorDto) -> (u16, Value) {
    (400, json!(error))
}

fn with_body<T, F>(request: &Request, handler: F) -> (u16, Value)
where
    T: DeserializeOwned,
    F: FnOnce(T) -> Result<Value, IpcErrorDto>,
{
    let body: T = match serde_json::from_slice(&request.body) {
        Ok(body) => body,
        Err(_) => {
            return (
                400,
                error_body("invalid_json", "请求体不是合法 JSON，或字段不符合约定。"),
            )
        }
    };
    ok(handler(body))
}

fn limit_param(query: &str) -> Option<usize> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key == "limit" {
            value.parse().ok()
        } else {
            None
        }
    })
}

fn error_body(code: &str, message: &str) -> Value {
    json!(IpcErrorDto::new(code, message, false))
}

fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(header: &[u8]) -> Option<usize> {
    let text = std::str::from_utf8(header).ok()?;
    for line in text.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return value.trim().parse().ok();
        }
    }
    Some(0)
}

fn response_bytes(status: u16, value: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    let mut response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);
    response
}

fn write_response(stream: &mut TcpStream, status: u16, value: &Value) -> std::io::Result<()> {
    stream.write_all(&response_bytes(status, value))?;
    stream.flush()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExplainBody {
    text: String,
    #[serde(default)]
    context: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CardDetailBody {
    card_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SaveCardBody {
    word: String,
    explanation: String,
    #[serde(default)]
    sources: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SearchBody {
    query: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AssistBody {
    #[serde(default)]
    focus: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::dto::{
        AssistResultDto, CardDetailDto, ExplainResultDto, KeywordCardViewDto, SearchResponseDto,
        TranscriptLineDto,
    };

    const TOKEN: &str = "unit-test-token";
    const PORT: u16 = 12345;

    struct FakeApi;

    impl BridgeApi for FakeApi {
        fn status(&self) -> Result<Value, IpcErrorDto> {
            Ok(json!({ "protocol": 1, "appVersion": "test", "cards": 0 }))
        }

        fn explain(&self, text: &str, _context: &str) -> Result<ExplainResultDto, IpcErrorDto> {
            if text.is_empty() {
                return Err(IpcErrorDto::new(
                    "empty_selection",
                    "请先选中一段文字。",
                    false,
                ));
            }
            Ok(ExplainResultDto {
                text: text.to_owned(),
                explanation: "本机上下文".to_owned(),
                source: "local".to_owned(),
            })
        }

        fn cards(&self) -> Result<Vec<KeywordCardViewDto>, IpcErrorDto> {
            Ok(Vec::new())
        }

        fn card_detail(&self, card_id: &str) -> Result<CardDetailDto, IpcErrorDto> {
            Err(IpcErrorDto::new(
                "card_not_found",
                &format!("{card_id} 不存在"),
                false,
            ))
        }

        fn save_card(
            &self,
            _word: &str,
            _explanation: &str,
            _sources: &[String],
        ) -> Result<usize, IpcErrorDto> {
            Ok(1)
        }

        fn transcript(&self) -> Result<Vec<TranscriptLineDto>, IpcErrorDto> {
            Ok((0..3)
                .map(|index| TranscriptLineDto {
                    index,
                    text: format!("第 {index} 句"),
                    at_ms: index as i64,
                })
                .collect())
        }

        fn search(&self, _query: &str) -> Result<SearchResponseDto, IpcErrorDto> {
            Ok(SearchResponseDto {
                provider: "fake".to_owned(),
                hits: Vec::new(),
            })
        }

        fn assist(&self, focus: &str) -> Result<AssistResultDto, IpcErrorDto> {
            Ok(AssistResultDto {
                source: "local".to_owned(),
                points: vec![focus.to_owned()],
                replies: Vec::new(),
                note: String::new(),
            })
        }

        fn asr(&self, action: AsrAction) -> Result<Value, IpcErrorDto> {
            Ok(json!({ "action": format!("{action:?}"), "running": false }))
        }
    }

    fn build(method: &str, target: &str, body: &str, headers: &[(&str, &str)]) -> Vec<u8> {
        let mut request = format!(
            "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nAuthorization: Bearer {TOKEN}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        );
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        request.push_str(body);
        request.into_bytes()
    }

    fn status_of(response: &[u8]) -> u16 {
        let text = std::str::from_utf8(response).expect("响应是 UTF-8");
        let line = text.lines().next().expect("有状态行");
        let status = line.split_whitespace().nth(1).expect("有状态码");
        status.parse().expect("状态码是数字")
    }

    fn body_of(response: &[u8]) -> Value {
        let text = std::str::from_utf8(response).expect("响应是 UTF-8");
        let body = text.split("\r\n\r\n").nth(1).expect("有响应体");
        serde_json::from_str(body).expect("响应体是 JSON")
    }

    #[test]
    fn status_returns_json() {
        let response = respond(&build("GET", "/v1/status", "", &[]), TOKEN, PORT, &FakeApi);
        assert_eq!(status_of(&response), 200);
        assert_eq!(body_of(&response)["appVersion"], "test");
    }

    #[test]
    fn missing_token_is_rejected() {
        let request = format!("GET /v1/status HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\n\r\n");
        let response = respond(request.as_bytes(), TOKEN, PORT, &FakeApi);
        assert_eq!(status_of(&response), 401);
        assert_eq!(body_of(&response)["code"], "unauthorized");
    }

    #[test]
    fn wrong_token_is_rejected() {
        let request = format!(
            "GET /v1/status HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nAuthorization: Bearer nope\r\n\r\n"
        );
        let response = respond(request.as_bytes(), TOKEN, PORT, &FakeApi);
        assert_eq!(status_of(&response), 401);
    }

    #[test]
    fn browser_origin_is_rejected() {
        let response = respond(
            &build(
                "GET",
                "/v1/status",
                "",
                &[("Origin", "http://evil.example")],
            ),
            TOKEN,
            PORT,
            &FakeApi,
        );
        assert_eq!(status_of(&response), 403);
        assert_eq!(body_of(&response)["code"], "forbidden_origin");
    }

    #[test]
    fn foreign_host_is_rejected() {
        let request = format!(
            "GET /v1/status HTTP/1.1\r\nHost: 10.0.0.2:{PORT}\r\nAuthorization: Bearer {TOKEN}\r\n\r\n"
        );
        let response = respond(request.as_bytes(), TOKEN, PORT, &FakeApi);
        assert_eq!(status_of(&response), 403);
        assert_eq!(body_of(&response)["code"], "forbidden_host");
    }

    #[test]
    fn localhost_host_is_accepted() {
        let request = format!(
            "GET /v1/status HTTP/1.1\r\nHost: localhost:{PORT}\r\nAuthorization: Bearer {TOKEN}\r\n\r\n"
        );
        let response = respond(request.as_bytes(), TOKEN, PORT, &FakeApi);
        assert_eq!(status_of(&response), 200);
    }

    #[test]
    fn unknown_route_is_404() {
        let response = respond(&build("GET", "/v1/nope", "", &[]), TOKEN, PORT, &FakeApi);
        assert_eq!(status_of(&response), 404);
    }

    #[test]
    fn wrong_method_is_405() {
        let response = respond(&build("GET", "/v1/explain", "", &[]), TOKEN, PORT, &FakeApi);
        assert_eq!(status_of(&response), 405);
    }

    #[test]
    fn explain_returns_local_fallback() {
        let response = respond(
            &build("POST", "/v1/explain", r#"{"text":"灰度"}"#, &[]),
            TOKEN,
            PORT,
            &FakeApi,
        );
        assert_eq!(status_of(&response), 200);
        assert_eq!(body_of(&response)["source"], "local");
    }

    #[test]
    fn api_error_keeps_code() {
        let response = respond(
            &build("POST", "/v1/explain", r#"{"text":""}"#, &[]),
            TOKEN,
            PORT,
            &FakeApi,
        );
        assert_eq!(status_of(&response), 400);
        assert_eq!(body_of(&response)["code"], "empty_selection");
    }

    #[test]
    fn invalid_json_is_400() {
        let response = respond(
            &build("POST", "/v1/explain", "not-json", &[]),
            TOKEN,
            PORT,
            &FakeApi,
        );
        assert_eq!(status_of(&response), 400);
        assert_eq!(body_of(&response)["code"], "invalid_json");
    }

    #[test]
    fn unknown_field_is_400() {
        let response = respond(
            &build("POST", "/v1/explain", r#"{"text":"灰度","extra":1}"#, &[]),
            TOKEN,
            PORT,
            &FakeApi,
        );
        assert_eq!(status_of(&response), 400);
        assert_eq!(body_of(&response)["code"], "invalid_json");
    }

    #[test]
    fn oversized_body_is_413() {
        let body = "x".repeat(MAX_BODY_BYTES + 1);
        let raw = build("POST", "/v1/explain", &body, &[]);
        let response = respond(&raw, TOKEN, PORT, &FakeApi);
        assert_eq!(status_of(&response), 413);
    }

    #[test]
    fn transcript_returns_tail_in_order() {
        let response = respond(
            &build("GET", "/v1/transcript?limit=2", "", &[]),
            TOKEN,
            PORT,
            &FakeApi,
        );
        assert_eq!(status_of(&response), 200);
        let body = body_of(&response);
        assert_eq!(body["total"], 3);
        assert_eq!(body["lines"].as_array().expect("数组").len(), 2);
        assert_eq!(body["lines"][0]["index"], 1);
        assert_eq!(body["lines"][1]["index"], 2);
    }

    #[test]
    fn cards_limit_is_clamped() {
        let response = respond(
            &build("GET", "/v1/cards?limit=999", "", &[]),
            TOKEN,
            PORT,
            &FakeApi,
        );
        assert_eq!(status_of(&response), 200);
        assert!(body_of(&response)["cards"].is_array());
    }

    #[test]
    fn asr_status_returns_json() {
        let response = respond(
            &build("GET", "/v1/asr/status", "", &[]),
            TOKEN,
            PORT,
            &FakeApi,
        );
        assert_eq!(status_of(&response), 200);
        assert_eq!(body_of(&response)["running"], false);
    }

    #[test]
    fn token_compare_is_exact() {
        assert!(token_matches("abc", "abc"));
        assert!(!token_matches("abc", "abd"));
        assert!(!token_matches("abc", "ab"));
    }

    #[test]
    fn content_length_ignores_trailing_blank_lines() {
        let header = b"GET /v1/status HTTP/1.1\r\nHost: 127.0.0.1:1\r\n\r\n";
        assert_eq!(content_length(header), Some(0));
        let with_body = b"POST /v1/explain HTTP/1.1\r\nContent-Length: 12\r\n\r\n";
        assert_eq!(content_length(with_body), Some(12));
    }
}
