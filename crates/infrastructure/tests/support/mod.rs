//! 集成测试脚手架：进程内 mock 端点、脚本化传输、装配与断言工具。

#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use meeting_copilot_domain::model::llm::{
    DeadlineMs, LlmGenerationRequest, LlmTask, OutputBudget, RedactedPayload, SummaryPurpose,
    TemperatureMilli,
};
use meeting_copilot_domain::model::provider::DomainPattern;
use meeting_copilot_domain::port::llm::{CancelToken, StructuredLlmGateway};
use meeting_copilot_domain::vo::TaskId;
use meeting_copilot_infrastructure::llm::degrade::{Clock, FakeClock};
use meeting_copilot_infrastructure::llm::transport::{
    HttpRequest, HttpResponse, HttpTransport, StreamReader, TransportError,
};
use meeting_copilot_infrastructure::llm::{assemble_with_clock, LlmConfig, LocalObserver};

pub const SENSITIVE: &str = "SENSITIVE-TOKEN";

#[derive(Debug, Clone)]
pub enum Step {
    Json(String),
    Status(u16, Vec<(String, String)>, String),
    Fail(TransportError),
    Delay(Duration),
    DelayThen(Duration, Box<Step>),
    Sse(Vec<String>),
    Empty,
}

pub struct FakeTransport {
    steps: Mutex<VecDeque<Step>>,
    requests: AtomicUsize,
}

impl FakeTransport {
    pub fn new(steps: Vec<Step>) -> Arc<Self> {
        Arc::new(Self {
            steps: Mutex::new(steps.into()),
            requests: AtomicUsize::new(0),
        })
    }

    pub fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    fn next(&self) -> Step {
        self.steps
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .pop_front()
            .unwrap_or(Step::Empty)
    }

    fn respond(&self, step: Step, deadline: Instant) -> Result<HttpResponse, TransportError> {
        match step {
            Step::Json(body) => Ok(json_response(&body)),
            Step::Status(status, headers, body) => Ok(HttpResponse {
                status,
                headers,
                body: body.into_bytes(),
            }),
            Step::Fail(error) => Err(error),
            Step::Delay(duration) => {
                if Instant::now() + duration >= deadline {
                    return Err(TransportError::ReadTimeout);
                }
                thread::sleep(duration);
                Ok(json_response("{}"))
            }
            Step::DelayThen(duration, inner) => {
                if Instant::now() + duration >= deadline {
                    return Err(TransportError::ReadTimeout);
                }
                thread::sleep(duration);
                self.respond(*inner, deadline)
            }
            Step::Sse(_) => Ok(json_response("{}")),
            Step::Empty => Ok(json_response("{}")),
        }
    }
}

impl HttpTransport for FakeTransport {
    fn send(
        &self,
        _request: &HttpRequest,
        deadline: Instant,
        _cancel: &CancelToken,
    ) -> Result<HttpResponse, TransportError> {
        self.requests.fetch_add(1, Ordering::SeqCst);
        let step = self.next();
        self.respond(step, deadline)
    }

    fn open_stream(
        &self,
        _request: &HttpRequest,
        deadline: Instant,
        _cancel: &CancelToken,
    ) -> Result<Box<dyn StreamReader>, TransportError> {
        self.requests.fetch_add(1, Ordering::SeqCst);
        let step = self.next();
        match step {
            Step::Fail(error) => Err(error),
            Step::Status(status, headers, body) => Ok(Box::new(ImmediateStream {
                status,
                headers,
                body: Some(body),
            })),
            Step::Sse(chunks) => Ok(Box::new(FakeStream {
                chunks: chunks.into_iter().map(String::into_bytes).collect(),
                delay: Duration::ZERO,
                deadline,
            })),
            Step::DelayThen(duration, inner) => match *inner {
                Step::Sse(chunks) => Ok(Box::new(FakeStream {
                    chunks: chunks.into_iter().map(String::into_bytes).collect(),
                    delay: duration,
                    deadline,
                })),
                other => self.respond(other, deadline).map(|response| {
                    let body = Some(response.body_text());
                    Box::new(ImmediateStream {
                        status: response.status,
                        headers: response.headers,
                        body,
                    }) as Box<dyn StreamReader>
                }),
            },
            Step::Json(body) => Ok(Box::new(FakeStream {
                chunks: vec![body.into_bytes()].into(),
                delay: Duration::ZERO,
                deadline,
            })),
            Step::Empty => Ok(Box::new(FakeStream {
                chunks: vec![b"{}".to_vec()].into(),
                delay: Duration::ZERO,
                deadline,
            })),
            Step::Delay(duration) => {
                if Instant::now() + duration >= deadline {
                    return Err(TransportError::ReadTimeout);
                }
                thread::sleep(duration);
                Ok(Box::new(FakeStream {
                    chunks: vec![b"{}".to_vec()].into(),
                    delay: Duration::ZERO,
                    deadline,
                }))
            }
        }
    }
}

struct FakeStream {
    chunks: VecDeque<Vec<u8>>,
    delay: Duration,
    deadline: Instant,
}

impl StreamReader for FakeStream {
    fn next_chunk(
        &mut self,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        if cancel.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        if Instant::now() >= deadline.min(self.deadline) {
            return Err(TransportError::ReadTimeout);
        }
        if !self.delay.is_zero() {
            if Instant::now() + self.delay >= deadline.min(self.deadline) {
                return Err(TransportError::ReadTimeout);
            }
            thread::sleep(self.delay);
            self.delay = Duration::ZERO;
        }
        Ok(self.chunks.pop_front())
    }
}

struct ImmediateStream {
    status: u16,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

impl StreamReader for ImmediateStream {
    fn next_chunk(
        &mut self,
        _deadline: Instant,
        _cancel: &CancelToken,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        if self.body.take().is_none() {
            return Ok(None);
        }
        let retry_after = self
            .headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("retry-after"))
            .map(|(_, value)| value.as_str())
            .unwrap_or("");
        Ok(Some(
            format!("HTTPSTATUS:{}\nRETRY-AFTER:{retry_after}\n", self.status).into_bytes(),
        ))
    }
}

pub fn json_response(body: &str) -> HttpResponse {
    HttpResponse {
        status: 200,
        headers: vec![("Content-Type".into(), "application/json".into())],
        body: body.as_bytes().to_vec(),
    }
}

pub fn envelope(content: &str) -> String {
    serde_json::json!({
        "model": "test-model",
        "choices": [{"message": {"content": content}}]
    })
    .to_string()
}

pub fn insight_body() -> String {
    serde_json::json!({
        "insights": [{
            "kind": "key_info",
            "text": "确认下周灰度上线",
            "sources": ["utt-1"],
            "confidence": 0.8
        }]
    })
    .to_string()
}

pub fn summary_body() -> String {
    serde_json::json!({
        "purpose": "rolling_window",
        "text": "已讨论灰度计划",
        "covered": null,
        "version": 1,
        "carried_over": false,
        "produced": true
    })
    .to_string()
}

pub fn conclusion_body() -> String {
    serde_json::json!({
        "title": "灰度策略",
        "conclusion": null,
        "references": [],
        "confidence": 0.4,
        "conflict": null,
        "notice": "generation_unavailable"
    })
    .to_string()
}

pub fn minutes_body() -> String {
    serde_json::json!({
        "summary": "本次会议讨论了灰度",
        "insights": {"key_info": [], "decision": [], "todo": [], "question": []},
        "timeline": {"start": null, "end": null, "entries": []},
        "open_questions": [{"text": "上线时间待定", "reason": "awaiting_due"}],
        "sources": ["utt-1"],
        "unrefined_count": 1,
        "completion_hint": "more_utterances"
    })
    .to_string()
}

pub fn test_config() -> LlmConfig {
    LlmConfig {
        endpoint: "http://127.0.0.1:9/v1/chat/completions".into(),
        domain_allowlist: vec![DomainPattern::new("127.0.0.1").expect("pattern")],
        model: "test-model".into(),
        request_timeout_ms: 8_000,
        payload_char_limit: 8_000,
        ..LlmConfig::default()
    }
}

pub fn request(task: LlmTask, payload: &str, deadline_ms: u64) -> LlmGenerationRequest {
    let purpose = (task == LlmTask::Summary).then_some(SummaryPurpose::RollingWindow);
    LlmGenerationRequest::new(
        TaskId::new(format!("req-{}", task.as_str())).expect("id"),
        RedactedPayload::new(payload, task, 8_000).expect("payload"),
        purpose,
        OutputBudget::new(512).expect("output"),
        TemperatureMilli::new(200).expect("temperature"),
        DeadlineMs::new(deadline_ms).expect("deadline"),
    )
    .expect("request")
}

pub fn gateway(
    config: LlmConfig,
    transport: Arc<dyn HttpTransport>,
) -> (Arc<dyn StructuredLlmGateway>, Arc<LocalObserver>) {
    gateway_with_config(
        Arc::new(RwLock::new(config)),
        transport,
        Arc::new(FakeClock::new()),
    )
}

pub fn gateway_with_config(
    config: Arc<RwLock<LlmConfig>>,
    transport: Arc<dyn HttpTransport>,
    clock: Arc<dyn Clock>,
) -> (Arc<dyn StructuredLlmGateway>, Arc<LocalObserver>) {
    let observer = Arc::new(LocalObserver::new());
    let gateway =
        assemble_with_clock(config, transport, observer.clone(), clock).expect("装配成功");
    (gateway, observer)
}

pub fn assert_timeout_within(elapsed: Duration, deadline: Duration) {
    assert!(
        elapsed <= deadline + Duration::from_millis(200),
        "耗时 {elapsed:?} 超过截止 {deadline:?} + 200ms"
    );
}

/// 进程内回环端点：每次接受一个连接并返回固定正文。
pub struct MockServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    requests: Arc<AtomicUsize>,
    handle: Option<JoinHandle<()>>,
}

impl MockServer {
    pub fn start(body: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定回环");
        let addr = listener.local_addr().expect("地址");
        listener.set_nonblocking(true).expect("非阻塞监听");
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(AtomicUsize::new(0));
        let stop_thread = stop.clone();
        let requests_thread = requests.clone();
        let handle = thread::spawn(move || {
            while !stop_thread.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        requests_thread.fetch_add(1, Ordering::SeqCst);
                        let mut buffer = [0_u8; 8192];
                        let _ = stream.read(&mut buffer);
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            addr,
            stop,
            requests,
            handle: Some(handle),
        }
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
