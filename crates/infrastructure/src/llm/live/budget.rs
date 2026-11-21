//! 预算账本与唯一计数点（AC-31；DESIGN 2.10.8）。
//!
//! `Permit` 不回收：失败与重试同样计入预算。

use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use meeting_copilot_domain::port::llm::CancelToken;

use crate::llm::transport::{
    HttpRequest, HttpResponse, HttpTransport, StreamReader, TransportError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetExhausted;

impl fmt::Display for BudgetExhausted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("budget_exhausted")
    }
}

#[derive(Debug)]
pub struct BudgetLedger {
    used: AtomicU64,
    limit: u64,
}

#[derive(Debug)]
pub struct Permit<'a> {
    _ledger: PhantomData<&'a BudgetLedger>,
}

impl BudgetLedger {
    pub fn new(limit: u64) -> Self {
        Self {
            used: AtomicU64::new(0),
            limit,
        }
    }

    pub fn try_acquire(&self) -> Result<Permit<'_>, BudgetExhausted> {
        loop {
            let current = self.used.load(Ordering::Acquire);
            if current >= self.limit {
                return Err(BudgetExhausted);
            }
            if self
                .used
                .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(Permit {
                    _ledger: PhantomData,
                });
            }
        }
    }

    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Acquire)
    }

    pub const fn limit(&self) -> u64 {
        self.limit
    }
}

/// 装饰器：`send` 与 `open_stream` 的唯一计数点。
pub struct BudgetedTransport {
    inner: Arc<dyn HttpTransport>,
    ledger: Arc<BudgetLedger>,
}

impl BudgetedTransport {
    pub fn new(inner: Arc<dyn HttpTransport>, ledger: Arc<BudgetLedger>) -> Self {
        Self { inner, ledger }
    }

    pub fn ledger(&self) -> &Arc<BudgetLedger> {
        &self.ledger
    }
}

impl HttpTransport for BudgetedTransport {
    fn send(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<HttpResponse, TransportError> {
        let _permit = self
            .ledger
            .try_acquire()
            .map_err(|_| TransportError::BudgetExhausted)?;
        self.inner.send(request, deadline, cancel)
    }

    fn open_stream(
        &self,
        request: &HttpRequest,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<Box<dyn StreamReader>, TransportError> {
        let _permit = self
            .ledger
            .try_acquire()
            .map_err(|_| TransportError::BudgetExhausted)?;
        self.inner.open_stream(request, deadline, cancel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Mutex;

    #[test]
    fn ledger_never_exceeds_limit() {
        let ledger = Arc::new(BudgetLedger::new(50));
        let successes = Arc::new(AtomicU64::new(0));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let ledger = ledger.clone();
            let successes = successes.clone();
            handles.push(std::thread::spawn(move || {
                for _ in 0..100 {
                    if ledger.try_acquire().is_ok() {
                        successes.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        for handle in handles {
            handle.join().expect("线程");
        }
        assert_eq!(successes.load(Ordering::Relaxed), 50);
        assert_eq!(ledger.used(), 50);
        assert!(ledger.used() <= ledger.limit());
    }

    #[test]
    fn permit_drop_does_not_refund() {
        let ledger = BudgetLedger::new(1);
        {
            let _permit = ledger.try_acquire().expect("许可");
        }
        assert_eq!(ledger.used(), 1);
        assert!(ledger.try_acquire().is_err());
    }

    #[test]
    fn zero_limit_rejects() {
        let ledger = BudgetLedger::new(0);
        assert!(ledger.try_acquire().is_err());
    }

    struct CountingTransport {
        calls: AtomicUsize,
        events: Mutex<Vec<usize>>,
    }

    impl HttpTransport for CountingTransport {
        fn send(
            &self,
            _request: &HttpRequest,
            _deadline: Instant,
            _cancel: &CancelToken,
        ) -> Result<HttpResponse, TransportError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.events
                .lock()
                .expect("锁")
                .push(self.calls.load(Ordering::SeqCst));
            Err(TransportError::ReadTimeout)
        }

        fn open_stream(
            &self,
            _request: &HttpRequest,
            _deadline: Instant,
            _cancel: &CancelToken,
        ) -> Result<Box<dyn StreamReader>, TransportError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(TransportError::ReadTimeout)
        }
    }

    #[test]
    fn exhausted_short_circuits_before_inner() {
        let inner = Arc::new(CountingTransport {
            calls: AtomicUsize::new(0),
            events: Mutex::new(Vec::new()),
        });
        let ledger = Arc::new(BudgetLedger::new(1));
        let transport = BudgetedTransport::new(inner.clone(), ledger.clone());
        let request =
            HttpRequest::json_post("api.deepseek.com", 443, "/v1/x", Vec::new(), Vec::new());
        let deadline = Instant::now() + std::time::Duration::from_secs(1);
        assert!(transport
            .send(&request, deadline, &CancelToken::new())
            .is_err());
        assert_eq!(
            transport.send(&request, deadline, &CancelToken::new()),
            Err(TransportError::BudgetExhausted)
        );
        assert_eq!(inner.calls.load(Ordering::SeqCst), 1);
        assert_eq!(ledger.used(), 1);
    }

    #[test]
    fn retry_counts_through_single_point() {
        let inner = Arc::new(CountingTransport {
            calls: AtomicUsize::new(0),
            events: Mutex::new(Vec::new()),
        });
        let ledger = Arc::new(BudgetLedger::new(4));
        let transport = BudgetedTransport::new(inner, ledger.clone());
        let request =
            HttpRequest::json_post("api.deepseek.com", 443, "/v1/x", Vec::new(), Vec::new());
        let deadline = Instant::now() + std::time::Duration::from_secs(1);
        let _ = transport.send(&request, deadline, &CancelToken::new());
        let _ = transport.send(&request, deadline, &CancelToken::new());
        assert_eq!(ledger.used(), 2);
    }
}
