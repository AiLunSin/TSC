//! 真实 HTTPS 传输（阶段 D；仅 `llm-https` feature 编译）。
//!
//! 系统 TLS（Windows SChannel / macOS Security.framework），不打包 OpenSSL。
//! 协议编解码复用 `Http1Codec`；不跟随 3xx、不读系统代理、不复用连接。

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use native_tls::{HandshakeError, Protocol, TlsConnector, TlsStream};

use meeting_copilot_domain::port::llm::CancelToken;

use crate::llm::live::policy::DEEPSEEK_HOSTS;
use crate::llm::transport::{
    parse_head, read_until_headers, Http1Codec, HttpRequest, HttpResponse, HttpTransport,
    IncrementalBodyReader, StreamReader, TimedRead, TransportError,
};

/// 只读 TLS 策略，供离线断言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsPolicy {
    pub min_version: &'static str,
    pub allow_invalid_certs: bool,
    pub custom_ca: bool,
    pub sni_is_host: bool,
}

pub const TLS_POLICY: TlsPolicy = TlsPolicy {
    min_version: "TLS1.2",
    allow_invalid_certs: false,
    custom_ca: false,
    sni_is_host: true,
};

pub struct TlsTransport {
    allowed_hosts: Vec<String>,
}

impl Default for TlsTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl TlsTransport {
    pub fn new() -> Self {
        Self {
            allowed_hosts: DEEPSEEK_HOSTS
                .iter()
                .map(|host| (*host).to_owned())
                .collect(),
        }
    }

    /// 仅供离线测试连回环地址使用；生产入口仍使用 [`TlsTransport::new`]。
    pub fn with_allowed_hosts(hosts: &[&str]) -> Self {
        Self {
            allowed_hosts: hosts.iter().map(|host| (*host).to_owned()).collect(),
        }
    }

    fn host_allowed(&self, host: &str) -> bool {
        let normalized = host.to_ascii_lowercase();
        self.allowed_hosts
            .iter()
            .any(|allowed| allowed == &normalized)
    }

    fn connect_tcp(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<TcpStream, TransportError> {
        if !request.secure {
            return Err(TransportError::TlsFailure);
        }
        if !self.host_allowed(&request.host) {
            return Err(TransportError::Malformed);
        }
        if cancel.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        let addresses = (request.host.as_str(), request.port)
            .to_socket_addrs()
            .map_err(|_| TransportError::DnsFailure)?;
        let mut last = TransportError::NetworkUnreachable;
        for address in addresses {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(TransportError::ConnectTimeout);
            }
            let timeout = remaining.max(Duration::from_millis(1));
            match TcpStream::connect_timeout(&address, timeout) {
                Ok(stream) => {
                    let _ = stream.set_nodelay(true);
                    return Ok(stream);
                }
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
                    last = TransportError::ConnectRefused;
                }
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                    last = TransportError::ConnectTimeout;
                }
                Err(_) => last = TransportError::NetworkUnreachable,
            }
        }
        Err(last)
    }

    fn handshake(
        &self,
        stream: TcpStream,
        host: &str,
        deadline: Instant,
    ) -> Result<TlsStream<TcpStream>, TransportError> {
        let connector = TlsConnector::builder()
            .min_protocol_version(Some(Protocol::Tlsv12))
            .build()
            .map_err(|_| TransportError::TlsFailure)?;
        let budget = deadline.saturating_duration_since(Instant::now());
        let timeout = budget
            .min(Duration::from_millis(500))
            .max(Duration::from_millis(1));
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));
        match connector.connect(host, stream) {
            Ok(stream) => Ok(stream),
            Err(HandshakeError::WouldBlock(_)) => Err(TransportError::ConnectTimeout),
            Err(HandshakeError::Failure(_)) => {
                if Instant::now() >= deadline {
                    Err(TransportError::ConnectTimeout)
                } else {
                    Err(TransportError::TlsFailure)
                }
            }
        }
    }

    fn send_tls(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<Vec<u8>, TransportError> {
        let encoded = Http1Codec::encode(request)?;
        let tcp = self.connect_tcp(request, deadline, cancel)?;
        let mut tls = self.handshake(tcp, &request.host, deadline)?;
        write_all_tls(&mut tls, &encoded, deadline, cancel)?;
        read_all_tls(&mut tls, deadline, cancel, 1_048_576)
    }
}

impl HttpTransport for TlsTransport {
    fn send(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<HttpResponse, TransportError> {
        let bytes = self.send_tls(request, deadline, cancel)?;
        Http1Codec::parse_response(&bytes, 1_048_576)
    }

    fn open_stream(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<Box<dyn StreamReader>, TransportError> {
        let encoded = Http1Codec::encode(request)?;
        let tcp = self.connect_tcp(request, deadline, cancel)?;
        let mut tls = self.handshake(tcp, &request.host, deadline)?;
        write_all_tls(&mut tls, &encoded, deadline, cancel)?;
        let head = read_until_headers(&mut tls, deadline, cancel)?;
        let (status, headers, _) = parse_head(&head)?;
        if !(200..300).contains(&status) {
            let retry_after = headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("retry-after"))
                .map(|(_, value)| value.as_str())
                .unwrap_or("");
            return Ok(Box::new(StatusStream {
                payload: Some(
                    format!("HTTPSTATUS:{status}\nRETRY-AFTER:{retry_after}\n").into_bytes(),
                ),
            }));
        }
        Ok(Box::new(IncrementalBodyReader::new(
            tls, &headers, 1_048_576,
        )))
    }
}

impl TimedRead for TlsStream<TcpStream> {
    fn apply_timeout(&self, deadline: Instant) {
        set_timeout(self.get_ref(), deadline);
    }
}

struct StatusStream {
    payload: Option<Vec<u8>>,
}

impl StreamReader for StatusStream {
    fn next_chunk(
        &mut self,
        _deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        if cancel.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        Ok(self.payload.take())
    }
}

fn set_timeout(stream: &TcpStream, deadline: Instant) {
    let budget = deadline
        .saturating_duration_since(Instant::now())
        .min(Duration::from_millis(50))
        .max(Duration::from_millis(1));
    let _ = stream.set_read_timeout(Some(budget));
    let _ = stream.set_write_timeout(Some(budget));
}

fn write_all_tls(
    stream: &mut TlsStream<TcpStream>,
    payload: &[u8],
    deadline: Instant,
    cancel: &CancelToken,
) -> Result<(), TransportError> {
    let mut written = 0;
    while written < payload.len() {
        if cancel.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(TransportError::ReadTimeout);
        }
        set_timeout(stream.get_ref(), deadline);
        match stream.write(&payload[written..]) {
            Ok(0) => return Err(TransportError::PeerClosed),
            Ok(count) => written += count,
            Err(error) if is_timeout(&error) => return Err(TransportError::ReadTimeout),
            Err(_) => return Err(TransportError::PeerClosed),
        }
    }
    Ok(())
}

fn read_all_tls(
    stream: &mut TlsStream<TcpStream>,
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
        set_timeout(stream.get_ref(), deadline);
        match stream.read(&mut buffer) {
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

fn is_timeout(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}
