//! Bearer 注入装饰器（AC-35；DESIGN 2.16.3）。
//!
//! 只在发送时构造带 `Authorization` 的请求副本；不实现 `Serialize`/`Display`。

use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use meeting_copilot_domain::port::llm::CancelToken;

use super::config::SecretString;
use crate::llm::transport::{
    HttpRequest, HttpResponse, HttpTransport, StreamReader, TransportError,
};

pub struct BearerTransport {
    inner: Arc<dyn HttpTransport>,
    secret: SecretString,
}

impl BearerTransport {
    pub fn new(inner: Arc<dyn HttpTransport>, secret: SecretString) -> Self {
        Self { inner, secret }
    }

    fn signed(&self, request: &HttpRequest) -> HttpRequest {
        let token = self.secret.expose();
        let mut signed = request.clone();
        signed
            .headers
            .retain(|(key, _)| !key.eq_ignore_ascii_case("authorization"));
        signed
            .headers
            .push(("Authorization".into(), format!("Bearer {token}")));
        signed
    }
}

impl fmt::Debug for BearerTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BearerTransport { secret: *** }")
    }
}

impl HttpTransport for BearerTransport {
    fn send(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<HttpResponse, TransportError> {
        self.inner.send(&self.signed(request), deadline, cancel)
    }

    fn open_stream(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<Box<dyn StreamReader>, TransportError> {
        self.inner
            .open_stream(&self.signed(request), deadline, cancel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct RecordingTransport {
        headers: Mutex<Vec<(String, String)>>,
    }

    impl HttpTransport for RecordingTransport {
        fn send(
            &self,
            request: &HttpRequest,
            _deadline: Instant,
            _cancel: &CancelToken,
        ) -> Result<HttpResponse, TransportError> {
            *self.headers.lock().expect("锁") = request.headers.clone();
            Ok(HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: b"{}".to_vec(),
            })
        }

        fn open_stream(
            &self,
            request: &HttpRequest,
            _deadline: Instant,
            _cancel: &CancelToken,
        ) -> Result<Box<dyn StreamReader>, TransportError> {
            *self.headers.lock().expect("锁") = request.headers.clone();
            Err(TransportError::PeerClosed)
        }
    }

    fn request() -> HttpRequest {
        HttpRequest::json_post(
            "127.0.0.1",
            443,
            "/chat/completions",
            vec![("Content-Type".into(), "application/json".into())],
            b"{}".to_vec(),
        )
    }

    #[test]
    fn bearer_header_injected_on_send() {
        let inner = Arc::new(RecordingTransport {
            headers: Mutex::new(Vec::new()),
        });
        let bearer = BearerTransport::new(inner.clone(), SecretString::new("sk-test-0000"));
        let _ = bearer.send(
            &request(),
            Instant::now() + std::time::Duration::from_secs(1),
            &CancelToken::new(),
        );
        let headers = inner.headers.lock().expect("锁").clone();
        let auth = headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("authorization"))
            .map(|(_, value)| value.as_str());
        assert_eq!(auth, Some("Bearer sk-test-0000"));
    }

    #[test]
    fn bearer_debug_is_masked() {
        let bearer = BearerTransport::new(
            Arc::new(RecordingTransport {
                headers: Mutex::new(Vec::new()),
            }),
            SecretString::new("sk-test-0000"),
        );
        let debug = format!("{bearer:?}");
        assert!(!debug.contains("sk-test"));
        assert!(!debug.contains("0000"));
    }
}
