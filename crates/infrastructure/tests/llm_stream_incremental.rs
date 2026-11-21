//! AC-36 增量读取离线用例：只连 127.0.0.1，零证书夹具。

use std::io::Write;
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use meeting_copilot_domain::model::provider::DomainPattern;
use meeting_copilot_domain::port::llm::CancelToken;
use meeting_copilot_infrastructure::llm::transport::{
    HttpRequest, HttpTransport, LoopbackTransport,
};

fn transport() -> LoopbackTransport {
    LoopbackTransport::new(
        vec![DomainPattern::new("127.0.0.1").expect("pattern")],
        1_048_576,
    )
}

fn request(port: u16) -> HttpRequest {
    HttpRequest::json_post(
        "127.0.0.1",
        port,
        "/chat/completions",
        vec![("Accept".into(), "text/event-stream".into())],
        b"{}".to_vec(),
    )
}

fn spawn_writer(chunks: Vec<Vec<u8>>, hold_ms: u64) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            for chunk in chunks {
                let _ = stream.write_all(&chunk);
                let _ = stream.flush();
            }
            thread::sleep(Duration::from_millis(hold_ms));
        }
    });
    (port, handle)
}

#[test]
fn first_delta_incremental_loopback() {
    let (port, worker) = spawn_writer(
        vec![
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec(),
            b"1a\r\ndata: {\"choices\":[{\"delta\":\r\n".to_vec(),
        ],
        500,
    );
    let mut stream = transport()
        .open_stream(
            &request(port),
            Instant::now() + Duration::from_secs(2),
            &CancelToken::new(),
        )
        .expect("open");
    let started = Instant::now();
    let chunk = stream
        .next_chunk(Instant::now() + Duration::from_secs(2), &CancelToken::new())
        .expect("chunk");
    assert!(chunk.is_some());
    assert!(started.elapsed() <= Duration::from_millis(200));
    let _ = worker.join();
}

#[test]
fn cancel_is_prompt() {
    let (port, worker) = spawn_writer(
        vec![b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n".to_vec()],
        500,
    );
    let mut stream = transport()
        .open_stream(
            &request(port),
            Instant::now() + Duration::from_secs(2),
            &CancelToken::new(),
        )
        .expect("open");
    let token = CancelToken::new();
    let clone = token.clone();
    let canceller = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        clone.cancel();
    });
    let started = Instant::now();
    let error = stream
        .next_chunk(Instant::now() + Duration::from_secs(2), &token)
        .expect_err("取消");
    assert_eq!(
        error,
        meeting_copilot_infrastructure::llm::transport::TransportError::Cancelled
    );
    assert!(started.elapsed() <= Duration::from_millis(300));
    let _ = canceller.join();
    let _ = worker.join();
}

#[test]
fn deadline_is_respected() {
    let (port, worker) = spawn_writer(
        vec![b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n".to_vec()],
        500,
    );
    let mut stream = transport()
        .open_stream(
            &request(port),
            Instant::now() + Duration::from_secs(2),
            &CancelToken::new(),
        )
        .expect("open");
    let deadline = Instant::now() + Duration::from_millis(100);
    let error = stream
        .next_chunk(deadline, &CancelToken::new())
        .expect_err("超时");
    assert_eq!(
        error,
        meeting_copilot_infrastructure::llm::transport::TransportError::ReadTimeout
    );
    assert!(Instant::now() <= deadline + Duration::from_millis(200));
    let _ = worker.join();
}

#[test]
fn chunked_boundaries() {
    let chunks = vec![
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec(),
        b"4\r\n{\"a\"\r\n3\r\n:1}".to_vec(),
        b"\r\n0\r\n\r\n".to_vec(),
    ];
    let (port, worker) = spawn_writer(chunks, 100);
    let mut stream = transport()
        .open_stream(
            &request(port),
            Instant::now() + Duration::from_secs(2),
            &CancelToken::new(),
        )
        .expect("open");
    let mut body = Vec::new();
    while let Some(bytes) = stream
        .next_chunk(Instant::now() + Duration::from_secs(2), &CancelToken::new())
        .expect("read")
    {
        body.extend(bytes);
    }
    assert_eq!(body, b"{\"a\":1}");
    let _ = worker.join();
}

#[test]
fn too_large_is_rejected() {
    let mut response = b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\n".to_vec();
    response.resize(response.len() + 200, b'x');
    let (port, worker) = spawn_writer(vec![response], 100);
    let transport =
        LoopbackTransport::new(vec![DomainPattern::new("127.0.0.1").expect("pattern")], 50);
    let mut stream = transport
        .open_stream(
            &request(port),
            Instant::now() + Duration::from_secs(2),
            &CancelToken::new(),
        )
        .expect("open");
    let error = stream
        .next_chunk(Instant::now() + Duration::from_secs(2), &CancelToken::new())
        .expect_err("超长");
    assert_eq!(
        error,
        meeting_copilot_infrastructure::llm::transport::TransportError::TooLarge
    );
    let _ = worker.join();
}
