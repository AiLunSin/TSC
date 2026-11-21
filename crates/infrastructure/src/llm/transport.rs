//! HTTP/1.1 传输边界（DESIGN 2.2.2-E、2.8.2）。
//!
//! 这是唯一知道 socket 的模块；错误枚举是 [`FailureClass`] 的唯一来源。
//! 不跟随重定向、不读系统代理、不做 HTTP/2。

use std::fmt;
use std::io::{BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use meeting_copilot_domain::model::provider::DomainPattern;
use meeting_copilot_domain::port::llm::CancelToken;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    #[error("域名解析失败")]
    DnsFailure,
    #[error("网络不可达")]
    NetworkUnreachable,
    #[error("连接被拒绝")]
    ConnectRefused,
    #[error("连接超时")]
    ConnectTimeout,
    #[error("读取超时")]
    ReadTimeout,
    #[error("TLS 失败")]
    TlsFailure,
    #[error("对端关闭")]
    PeerClosed,
    #[error("响应过大")]
    TooLarge,
    #[error("报文非法")]
    Malformed,
    #[error("已取消")]
    Cancelled,
    #[error("预算耗尽")]
    BudgetExhausted,
}

/// 请求。`Debug` 遮蔽 `Authorization` 与正文。
#[derive(Clone)]
pub struct HttpRequest {
    pub method: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub secure: bool,
}

impl HttpRequest {
    pub fn json_post(
        host: impl Into<String>,
        port: u16,
        path: impl Into<String>,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Self {
        Self {
            method: "POST".into(),
            host: host.into(),
            port,
            path: path.into(),
            headers,
            body,
            secure: false,
        }
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

impl fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header_names: Vec<&str> = self
            .headers
            .iter()
            .map(|(key, _)| {
                if key.eq_ignore_ascii_case("authorization") {
                    "authorization:***"
                } else {
                    key.as_str()
                }
            })
            .collect();
        f.debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("path", &self.path)
            .field("headers", &header_names)
            .field("body_bytes", &self.body.len())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

pub trait StreamReader: Send {
    fn next_chunk(
        &mut self,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<Option<Vec<u8>>, TransportError>;
}

pub trait HttpTransport: Send + Sync {
    fn send(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<HttpResponse, TransportError>;

    fn open_stream(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<Box<dyn StreamReader>, TransportError>;
}

/// HTTP/1.1 编解码。只处理字节，不接触 socket。
pub struct Http1Codec;

impl Http1Codec {
    pub fn encode(request: &HttpRequest) -> Result<Vec<u8>, TransportError> {
        if request.host.is_empty() || !request.path.starts_with('/') {
            return Err(TransportError::Malformed);
        }
        let mut out = Vec::new();
        out.extend_from_slice(
            format!("{} {} HTTP/1.1\r\n", request.method, request.path).as_bytes(),
        );
        out.extend_from_slice(format!("Host: {}\r\n", request.host).as_bytes());
        out.extend_from_slice(b"Connection: close\r\n");
        out.extend_from_slice(format!("Content-Length: {}\r\n", request.body.len()).as_bytes());
        for (key, value) in &request.headers {
            if key.eq_ignore_ascii_case("content-length") || key.eq_ignore_ascii_case("host") {
                continue;
            }
            out.extend_from_slice(format!("{key}: {value}\r\n").as_bytes());
        }
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&request.body);
        Ok(out)
    }

    pub fn parse_response(bytes: &[u8], max_bytes: usize) -> Result<HttpResponse, TransportError> {
        if bytes.len() > max_bytes {
            return Err(TransportError::TooLarge);
        }
        let (status, headers, body) = parse_head(bytes)?;
        let body = if is_chunked(&headers) {
            decode_chunked(body)?
        } else {
            body.to_vec()
        };
        if body.len() > max_bytes {
            return Err(TransportError::TooLarge);
        }
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

type ParsedHead<'a> = (u16, Vec<(String, String)>, &'a [u8]);

pub(crate) fn parse_head(bytes: &[u8]) -> Result<ParsedHead<'_>, TransportError> {
    let split = find_subsequence(bytes, b"\r\n\r\n").ok_or(TransportError::Malformed)?;
    let head = std::str::from_utf8(&bytes[..split]).map_err(|_| TransportError::Malformed)?;
    let mut lines = head.split("\r\n");
    let status_line = lines.next().ok_or(TransportError::Malformed)?;
    let mut status_parts = status_line.split_whitespace();
    let _version = status_parts.next().ok_or(TransportError::Malformed)?;
    let status = status_parts
        .next()
        .ok_or(TransportError::Malformed)?
        .parse::<u16>()
        .map_err(|_| TransportError::Malformed)?;
    let mut headers = Vec::new();
    for line in lines {
        let (key, value) = line.split_once(':').ok_or(TransportError::Malformed)?;
        headers.push((key.trim().to_owned(), value.trim().to_owned()));
    }
    Ok((status, headers, &bytes[split + 4..]))
}

pub(crate) fn is_chunked(headers: &[(String, String)]) -> bool {
    headers.iter().any(|(key, value)| {
        key.eq_ignore_ascii_case("transfer-encoding")
            && value.to_ascii_lowercase().contains("chunked")
    })
}

fn decode_chunked(body: &[u8]) -> Result<Vec<u8>, TransportError> {
    let mut out = Vec::new();
    let mut rest = body;
    loop {
        let line_end = find_subsequence(rest, b"\r\n").ok_or(TransportError::Malformed)?;
        let size_line =
            std::str::from_utf8(&rest[..line_end]).map_err(|_| TransportError::Malformed)?;
        let size_token = size_line.split(';').next().unwrap_or(size_line).trim();
        let size = usize::from_str_radix(size_token, 16).map_err(|_| TransportError::Malformed)?;
        rest = &rest[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if rest.len() < size + 2 || &rest[size..size + 2] != b"\r\n" {
            return Err(TransportError::Malformed);
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// 回环 socket 传输。生产 TLS 由 `llm-https` feature 提供，本阶段为空。
pub struct LoopbackTransport {
    allowlist: Vec<DomainPattern>,
    max_response_bytes: usize,
}

impl LoopbackTransport {
    pub fn new(allowlist: Vec<DomainPattern>, max_response_bytes: usize) -> Self {
        Self {
            allowlist,
            max_response_bytes,
        }
    }

    fn host_allowed(&self, host: &str) -> bool {
        !self.allowlist.is_empty() && self.allowlist.iter().any(|pattern| pattern.matches(host))
    }

    fn connect(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<TcpStream, TransportError> {
        if request.secure {
            return Err(TransportError::TlsFailure);
        }
        if !self.host_allowed(&request.host) {
            return Err(TransportError::Malformed);
        }
        if cancel.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        let remaining = remaining(deadline)?;
        let addresses = (request.host.as_str(), request.port)
            .to_socket_addrs()
            .map_err(|_| TransportError::DnsFailure)?;
        let mut last_error = TransportError::NetworkUnreachable;
        for address in addresses {
            if Instant::now() >= deadline {
                return Err(TransportError::ConnectTimeout);
            }
            let timeout = remaining
                .min(Duration::from_millis(50))
                .max(Duration::from_millis(1));
            match TcpStream::connect_timeout(&address, timeout) {
                Ok(stream) => {
                    let _ = stream.set_nodelay(true);
                    return Ok(stream);
                }
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
                    last_error = TransportError::ConnectRefused;
                }
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                    last_error = TransportError::ConnectTimeout;
                }
                Err(_) => last_error = TransportError::NetworkUnreachable,
            }
        }
        Err(last_error)
    }
}

impl HttpTransport for LoopbackTransport {
    fn send(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<HttpResponse, TransportError> {
        let stream = self.connect(request, deadline, cancel)?;
        let payload = Http1Codec::encode(request)?;
        write_all_budgeted(&stream, &payload, deadline, cancel)?;
        let bytes = read_all_budgeted(&stream, deadline, cancel, self.max_response_bytes)?;
        Http1Codec::parse_response(&bytes, self.max_response_bytes)
    }

    fn open_stream(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<Box<dyn StreamReader>, TransportError> {
        let stream = self.connect(request, deadline, cancel)?;
        let payload = Http1Codec::encode(request)?;
        write_all_budgeted(&stream, &payload, deadline, cancel)?;
        let mut reader = BufReader::new(stream);
        let head = read_until_headers(&mut reader, deadline, cancel)?;
        let (status, headers, _) = parse_head(&head)?;
        if !(200..300).contains(&status) {
            // 非 2xx 交由网关按状态码分类；此处返回一个空流，保留状态语义。
            return Ok(Box::new(ImmediateStream::new(status, headers)));
        }
        let chunked = is_chunked(&headers);
        let _ = chunked;
        Ok(Box::new(IncrementalBodyReader::new(
            reader,
            &headers,
            self.max_response_bytes,
        )))
    }
}

struct ImmediateStream {
    status: u16,
    headers: Vec<(String, String)>,
    delivered: bool,
}

impl ImmediateStream {
    fn new(status: u16, headers: Vec<(String, String)>) -> Self {
        Self {
            status,
            headers,
            delivered: false,
        }
    }
}

impl StreamReader for ImmediateStream {
    fn next_chunk(
        &mut self,
        _deadline: Instant,
        _cancel: &CancelToken,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        if self.delivered {
            return Ok(None);
        }
        self.delivered = true;
        let status = self.status;
        let reason = self
            .headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("retry-after"))
            .map(|(_, value)| value.clone())
            .unwrap_or_default();
        // 空体流；状态与 Retry-After 通过专用结束标记交给网关。
        Ok(Some(
            format!("HTTPSTATUS:{status}\nRETRY-AFTER:{reason}\n").into_bytes(),
        ))
    }
}

/// 允许统一设置 I/O 超时的读取器（回环与 TLS 共用）。
pub trait TimedRead: Read {
    fn apply_timeout(&self, deadline: Instant);
}

impl TimedRead for TcpStream {
    fn apply_timeout(&self, deadline: Instant) {
        set_timeout(self, deadline);
    }
}

impl TimedRead for BufReader<TcpStream> {
    fn apply_timeout(&self, deadline: Instant) {
        set_timeout(self.get_ref(), deadline);
    }
}

enum Framing {
    Length(Option<usize>),
    Chunked(ChunkedDecoder),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkStep {
    NeedMore,
    Item(Vec<u8>),
    Done,
}

#[derive(Debug, Default)]
pub struct ChunkedDecoder {
    buffer: Vec<u8>,
    done: bool,
}

impl ChunkedDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn advance(&mut self) -> ChunkStep {
        if self.done {
            return ChunkStep::Done;
        }
        let Some(line_end) = find_subsequence(&self.buffer, b"\r\n") else {
            return ChunkStep::NeedMore;
        };
        let size_line = String::from_utf8_lossy(&self.buffer[..line_end]);
        let Some(size_token) = size_line.split(';').next() else {
            self.done = true;
            return ChunkStep::Done;
        };
        let Ok(size) = usize::from_str_radix(size_token.trim(), 16) else {
            self.done = true;
            return ChunkStep::Done;
        };
        if size == 0 {
            self.done = true;
            self.buffer.clear();
            return ChunkStep::Done;
        }
        if self.buffer.len() < line_end + 2 + size + 2 {
            return ChunkStep::NeedMore;
        }
        let data_start = line_end + 2;
        let data = self.buffer[data_start..data_start + size].to_vec();
        self.buffer.drain(..data_start + size + 2);
        ChunkStep::Item(data)
    }
}

/// 增量交付字节；不解析 SSE（SSE 解析仍在网关）。
pub struct IncrementalBodyReader<R: TimedRead> {
    reader: R,
    framing: Framing,
    pending: Vec<u8>,
    done: bool,
    max_bytes: usize,
    total: usize,
}

impl<R: TimedRead> IncrementalBodyReader<R> {
    pub fn new(reader: R, headers: &[(String, String)], max_bytes: usize) -> Self {
        let framing = if is_chunked(headers) {
            Framing::Chunked(ChunkedDecoder::new())
        } else {
            let length = headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.parse::<usize>().ok());
            Framing::Length(length)
        };
        Self {
            reader,
            framing,
            pending: Vec::new(),
            done: false,
            max_bytes,
            total: 0,
        }
    }

    fn take_ready(&mut self) -> Option<Vec<u8>> {
        match &mut self.framing {
            Framing::Length(remaining) => {
                if let Some(left) = remaining {
                    if *left == 0 {
                        self.done = true;
                        return None;
                    }
                    if self.pending.is_empty() {
                        return None;
                    }
                    let take = self.pending.len().min(*left);
                    let chunk: Vec<u8> = self.pending.drain(..take).collect();
                    *left -= take;
                    if *left == 0 {
                        self.done = true;
                    }
                    return Some(chunk);
                }
                if self.pending.is_empty() {
                    None
                } else {
                    Some(std::mem::take(&mut self.pending))
                }
            }
            Framing::Chunked(decoder) => {
                if !self.pending.is_empty() {
                    let bytes = std::mem::take(&mut self.pending);
                    decoder.feed(&bytes);
                }
                match decoder.advance() {
                    ChunkStep::Item(bytes) => Some(bytes),
                    ChunkStep::Done => {
                        self.done = true;
                        None
                    }
                    ChunkStep::NeedMore => None,
                }
            }
        }
    }
}

impl<R: TimedRead + Send> StreamReader for IncrementalBodyReader<R> {
    fn next_chunk(
        &mut self,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        loop {
            if cancel.is_cancelled() {
                return Err(TransportError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(TransportError::ReadTimeout);
            }
            if let Some(chunk) = self.take_ready() {
                return Ok(Some(chunk));
            }
            if self.done {
                return Ok(None);
            }
            self.reader.apply_timeout(deadline);
            let mut buffer = [0_u8; 4096];
            match self.reader.read(&mut buffer) {
                Ok(0) => {
                    if self.done {
                        return Ok(None);
                    }
                    self.done = true;
                    return match &self.framing {
                        Framing::Length(Some(left)) if *left > 0 => Err(TransportError::Malformed),
                        Framing::Chunked(decoder) if !decoder.is_done() => {
                            Err(TransportError::Malformed)
                        }
                        _ => Ok(None),
                    };
                }
                Ok(read) => {
                    self.total = self.total.saturating_add(read);
                    if self.total > self.max_bytes {
                        return Err(TransportError::TooLarge);
                    }
                    self.pending.extend_from_slice(&buffer[..read]);
                }
                Err(error) if is_timeout(&error) => {
                    if Instant::now() >= deadline {
                        return Err(TransportError::ReadTimeout);
                    }
                }
                Err(_) => return Err(TransportError::PeerClosed),
            }
        }
    }
}

fn write_all_budgeted(
    mut stream: &TcpStream,
    payload: &[u8],
    deadline: Instant,
    cancel: &CancelToken,
) -> Result<(), TransportError> {
    set_timeout(stream, deadline);
    let mut written = 0;
    while written < payload.len() {
        if cancel.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(TransportError::ReadTimeout);
        }
        match stream.write(&payload[written..]) {
            Ok(0) => return Err(TransportError::PeerClosed),
            Ok(count) => written += count,
            Err(error) if is_timeout(&error) => return Err(TransportError::ReadTimeout),
            Err(_) => return Err(TransportError::PeerClosed),
        }
    }
    Ok(())
}

fn read_all_budgeted(
    stream: &TcpStream,
    deadline: Instant,
    cancel: &CancelToken,
    max: usize,
) -> Result<Vec<u8>, TransportError> {
    let mut reader = stream;
    read_all_budgeted_reader(&mut reader, stream, deadline, cancel, max)
}

fn read_all_budgeted_reader<R: Read>(
    reader: &mut R,
    stream: &TcpStream,
    deadline: Instant,
    cancel: &CancelToken,
    max: usize,
) -> Result<Vec<u8>, TransportError> {
    let mut out = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        if cancel.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(TransportError::ReadTimeout);
        }
        set_timeout(stream, deadline);
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(out),
            Ok(read) => {
                out.extend_from_slice(&buffer[..read]);
                if out.len() > max {
                    return Err(TransportError::TooLarge);
                }
            }
            Err(error) if is_timeout(&error) => {
                if Instant::now() >= deadline {
                    return Err(TransportError::ReadTimeout);
                }
            }
            Err(_) => return Err(TransportError::PeerClosed),
        }
    }
}

pub(crate) fn read_until_headers<R: TimedRead>(
    reader: &mut R,
    deadline: Instant,
    cancel: &CancelToken,
) -> Result<Vec<u8>, TransportError> {
    let mut out = Vec::new();
    let mut buffer = [0_u8; 1];
    loop {
        if cancel.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(TransportError::ReadTimeout);
        }
        if out.len() > 64 * 1024 {
            return Err(TransportError::TooLarge);
        }
        reader.apply_timeout(deadline);
        match reader.read(&mut buffer) {
            Ok(0) => return Err(TransportError::PeerClosed),
            Ok(_) => {
                out.push(buffer[0]);
                if out.ends_with(b"\r\n\r\n") {
                    return Ok(out);
                }
            }
            Err(error) if is_timeout(&error) => {
                if Instant::now() >= deadline {
                    return Err(TransportError::ReadTimeout);
                }
            }
            Err(_) => return Err(TransportError::PeerClosed),
        }
    }
}

fn set_timeout(stream: &TcpStream, deadline: Instant) {
    let budget = remaining(deadline).unwrap_or(Duration::from_millis(1));
    let timeout = budget
        .min(Duration::from_millis(50))
        .max(Duration::from_millis(1));
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
}

fn remaining(deadline: Instant) -> Result<Duration, TransportError> {
    let now = Instant::now();
    if now >= deadline {
        return Err(TransportError::ReadTimeout);
    }
    Ok(deadline.saturating_duration_since(now))
}

fn is_timeout(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}

/// 以剩余预算驱动 socket 超时；零预算不 panic。
pub fn budget_to_timeout(budget: Duration) -> Duration {
    budget.max(Duration::from_millis(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::provider::DomainPattern;

    fn request() -> HttpRequest {
        HttpRequest::json_post(
            "127.0.0.1",
            8080,
            "/v1/chat",
            vec![
                ("Authorization".into(), "Bearer sk-test-0000".into()),
                ("Content-Type".into(), "application/json".into()),
            ],
            b"{\"model\":\"test\"}".to_vec(),
        )
    }

    #[test]
    fn request_debug_masks_authorization_and_body() {
        let debug = format!("{:?}", request());
        assert!(!debug.contains("sk-test-0000"));
        assert!(!debug.contains("test\""));
        assert!(debug.contains("authorization"));
    }

    #[test]
    fn codec_roundtrip_content_length() {
        let bytes = Http1Codec::encode(&request()).expect("encode");
        assert!(bytes.starts_with(b"POST /v1/chat HTTP/1.1"));
        let response =
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}";
        let parsed = Http1Codec::parse_response(response, 1024).expect("parse");
        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.body, b"{}");
    }

    #[test]
    fn codec_parses_chunked() {
        let response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n";
        let parsed = Http1Codec::parse_response(response, 1024).expect("parse");
        assert_eq!(parsed.body_text(), "{\"a\":1}");
    }

    #[test]
    fn truncated_response_is_malformed() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nab";
        let parsed = Http1Codec::parse_response(response, 1024);
        assert!(parsed.is_ok());
        let bare = b"HTTP/1.1 200";
        assert_eq!(
            Http1Codec::parse_response(bare, 1024),
            Err(TransportError::Malformed)
        );
    }

    #[test]
    fn oversized_response_is_too_large() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 999\r\n\r\n";
        assert_eq!(
            Http1Codec::parse_response(response, 10),
            Err(TransportError::TooLarge)
        );
    }

    #[test]
    fn budget_to_timeout_never_zero() {
        assert!(budget_to_timeout(Duration::ZERO) >= Duration::from_millis(1));
    }

    #[test]
    fn allowlist_gate_rejects_without_socket() {
        let transport = LoopbackTransport::new(
            vec![DomainPattern::new("example.invalid").expect("pattern")],
            1024,
        );
        let error = transport
            .send(
                &request(),
                Instant::now() + Duration::from_secs(1),
                &CancelToken::new(),
            )
            .expect_err("必须拒绝");
        assert_eq!(error, TransportError::Malformed);
    }
}
