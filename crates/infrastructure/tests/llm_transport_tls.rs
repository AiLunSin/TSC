#![cfg(feature = "llm-https")]

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use meeting_copilot_domain::port::llm::CancelToken;
use meeting_copilot_infrastructure::llm::transport::{
    Http1Codec, HttpRequest, HttpTransport, TransportError,
};
use meeting_copilot_infrastructure::llm::transport_tls::{TlsTransport, TLS_POLICY};

fn request(secure: bool, port: u16) -> HttpRequest {
    let mut request = HttpRequest::json_post(
        "127.0.0.1",
        port,
        "/v1/chat",
        vec![("Content-Type".into(), "application/json".into())],
        b"{}".to_vec(),
    );
    request.secure = secure;
    request
}

fn transport() -> TlsTransport {
    TlsTransport::with_allowed_hosts(&["127.0.0.1"])
}

fn listener() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("绑定回环");
    let port = listener.local_addr().expect("地址").port();
    (listener, port)
}

#[test]
fn tls_connector_policy_is_pinned() {
    let policy = std::hint::black_box(TLS_POLICY);
    assert_eq!(policy.min_version, "TLS1.2");
    assert!(!policy.allow_invalid_certs);
    assert!(!policy.custom_ca);
    assert!(policy.sni_is_host);
}

#[test]
fn plaintext_peer_is_rejected() {
    let (listener, port) = listener();
    let worker = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
        }
    });
    let error = transport()
        .send(
            &request(true, port),
            Instant::now() + Duration::from_secs(1),
            &CancelToken::new(),
        )
        .expect_err("明文对端必须拒绝");
    assert_eq!(error, TransportError::TlsFailure);
    let _ = worker.join();
}

#[test]
fn garbage_peer_is_rejected() {
    let (listener, port) = listener();
    let worker = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = stream.write_all(b"\x00\x01\x02\x03garbage");
        }
    });
    let error = transport()
        .send(
            &request(true, port),
            Instant::now() + Duration::from_secs(1),
            &CancelToken::new(),
        )
        .expect_err("垃圾字节必须拒绝");
    assert_eq!(error, TransportError::TlsFailure);
    let _ = worker.join();
}

#[test]
fn connect_refused_is_classified() {
    let (listener, port) = listener();
    drop(listener);
    let error = transport()
        .send(
            &request(true, port),
            Instant::now() + Duration::from_secs(1),
            &CancelToken::new(),
        )
        .expect_err("端口关闭必须拒绝");
    assert!(
        matches!(
            error,
            TransportError::ConnectRefused | TransportError::ConnectTimeout
        ),
        "Windows 防火墙可能把本地拒绝表现为超时；两者都属于连接层拒绝，实际 {error:?}"
    );
}

#[test]
fn handshake_timeout_respects_deadline() {
    let (listener, port) = listener();
    let worker = thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            thread::sleep(Duration::from_secs(2));
            drop(stream);
        }
    });
    let deadline = Duration::from_millis(300);
    let started = Instant::now();
    let error = transport()
        .send(
            &request(true, port),
            Instant::now() + deadline,
            &CancelToken::new(),
        )
        .expect_err("静默对端必须超时");
    assert_eq!(error, TransportError::ConnectTimeout);
    assert!(started.elapsed() <= deadline + Duration::from_millis(600));
    let _ = worker.join();
}

#[test]
fn insecure_request_is_rejected_by_tls_transport() {
    let (listener, port) = listener();
    listener.set_nonblocking(true).expect("非阻塞");
    let error = transport()
        .send(
            &request(false, port),
            Instant::now() + Duration::from_secs(1),
            &CancelToken::new(),
        )
        .expect_err("secure=false 必须拒绝");
    assert_eq!(error, TransportError::TlsFailure);
    let accepted = listener.accept();
    assert!(accepted.is_err(), "拒绝路径不得建立连接");
}

#[test]
fn codec_contract_is_shared() {
    let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}";
    let parsed = Http1Codec::parse_response(raw, 1024).expect("解析");
    assert_eq!(parsed.status, 200);
    assert_eq!(parsed.body, b"{}");
    let chunked = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n";
    assert_eq!(
        Http1Codec::parse_response(chunked, 1024)
            .expect("分块")
            .body_text(),
        "{\"a\":1}"
    );
}

#[allow(dead_code)]
fn _stream_marker(_: TcpStream) {}
