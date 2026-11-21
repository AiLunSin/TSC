//! 降级决策：分类、重试、退避、冷却与并发（DESIGN 2.1.3-（3）~（6））。
//!
//! 纯决策逻辑 + 可注入时钟，测试不依赖真实睡眠。

use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use meeting_copilot_domain::model::llm::{CircuitState, DegradeCause, FailureClass};
use meeting_copilot_domain::port::llm::CancelToken;

use crate::llm::transport::TransportError;

pub const MIN_ATTEMPT_BUDGET: Duration = Duration::from_millis(50);
pub const MAX_ATTEMPTS: u8 = 3;
pub const MAX_RETRY_AFTER: Duration = Duration::from_secs(2);
pub const COOLING_THRESHOLD: u32 = 3;
pub const COOLING_DURATION: Duration = Duration::from_secs(60);

/// 传输错误 → 六类故障的唯一映射点。取消不进入分类。
pub fn classify(error: &TransportError) -> Option<FailureClass> {
    match error {
        TransportError::DnsFailure | TransportError::NetworkUnreachable => {
            Some(FailureClass::Offline)
        }
        TransportError::ConnectRefused
        | TransportError::TlsFailure
        | TransportError::PeerClosed => Some(FailureClass::Unavailable),
        TransportError::ConnectTimeout | TransportError::ReadTimeout => Some(FailureClass::Timeout),
        TransportError::TooLarge | TransportError::Malformed => Some(FailureClass::InvalidSchema),
        TransportError::Cancelled | TransportError::BudgetExhausted => None,
    }
}

/// 状态码 → 六类故障。只看状态码与固定标记，不对正文做自由文本猜测。
pub fn classify_status(status: u16) -> Option<FailureClass> {
    match status {
        200..=299 => None,
        429 => Some(FailureClass::RateLimited),
        401..=403 => Some(FailureClass::Unavailable),
        400 | 404 | 422 => Some(FailureClass::InvalidSchema),
        300..=399 => Some(FailureClass::Unavailable),
        400..=499 => Some(FailureClass::Unavailable),
        _ => Some(FailureClass::Unavailable),
    }
}

/// 带响应正文的 4xx/5xx 分类：显式内容策略标记优先，其次状态码矩阵。
pub fn classify_http(status: u16, body: &str) -> Option<FailureClass> {
    if (200..=299).contains(&status) {
        return None;
    }
    if status == 429 {
        return Some(FailureClass::RateLimited);
    }
    let lower = body.to_ascii_lowercase();
    if lower.contains("content_filter") || lower.contains("content_policy") {
        return Some(FailureClass::ContentFiltered);
    }
    classify_status(status)
}

/// 内容被拒的显式判定（仅供网关在解析固定错误码/ `finish_reason` 后调用）。
pub fn content_filtered() -> FailureClass {
    FailureClass::ContentFiltered
}

/// 每类故障允许的额外重试次数（不含首次尝试）。
pub const fn retry_allowance(class: FailureClass) -> u8 {
    match class {
        FailureClass::Timeout | FailureClass::Unavailable => 2,
        FailureClass::RateLimited | FailureClass::InvalidSchema => 1,
        FailureClass::ContentFiltered | FailureClass::Offline => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    Retry { delay: Duration },
    Degrade { cause: DegradeCause },
}

/// 是否继续重试；截止时间优先于重试。
pub fn decide_retry(
    class: FailureClass,
    attempt: u8,
    remaining: Duration,
    retry_after: Option<Duration>,
    request_id: &str,
) -> RetryDecision {
    if attempt >= MAX_ATTEMPTS || attempt > retry_allowance(class) {
        return RetryDecision::Degrade {
            cause: DegradeCause::Failed(class),
        };
    }
    let delay = match retry_after {
        Some(value) if value > MAX_RETRY_AFTER => {
            return RetryDecision::Degrade {
                cause: DegradeCause::Failed(class),
            };
        }
        Some(value) => value.max(Duration::from_millis(1)),
        None => jittered_backoff(class, attempt, request_id),
    };
    if remaining < delay + MIN_ATTEMPT_BUDGET {
        return RetryDecision::Degrade {
            cause: DegradeCause::Failed(class),
        };
    }
    RetryDecision::Retry { delay }
}

/// 抖动由 `request_id` 与尝试序号确定性派生（±20%）。
fn jittered_backoff(class: FailureClass, attempt: u8, request_id: &str) -> Duration {
    let base_ms = match class {
        FailureClass::RateLimited => 800_u64,
        _ => 200_u64,
    };
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in request_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash ^= u64::from(attempt);
    let span = base_ms / 5;
    let offset = if span == 0 { 0 } else { hash % (span * 2 + 1) };
    let millis = base_ms.saturating_sub(span).saturating_add(offset);
    Duration::from_millis(millis.max(1))
}

/// 解析 `Retry-After`：仅接受纯数字秒；HTTP-date 视为未声明。
pub fn parse_retry_after(value: Option<&str>) -> Option<Duration> {
    let raw = value?.trim();
    if raw.is_empty() || !raw.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let seconds = raw.parse::<u64>().ok()?;
    Some(Duration::from_secs(seconds))
}

pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration);
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// 测试时钟：`sleep` 只推进偏移，不真实等待。
#[derive(Debug)]
pub struct FakeClock {
    base: Instant,
    offset: Mutex<Duration>,
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeClock {
    pub fn new() -> Self {
        Self {
            base: Instant::now(),
            offset: Mutex::new(Duration::ZERO),
        }
    }

    pub fn advance(&self, duration: Duration) {
        let mut offset = self
            .offset
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *offset = offset.saturating_add(duration);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        let offset = *self
            .offset
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        self.base + offset
    }

    fn sleep(&self, duration: Duration) {
        self.advance(duration);
    }
}

/// 端点冷却状态机：连续 3 次最终失败进入 60 秒冷却。
#[derive(Debug, Clone)]
pub struct EndpointCircuit {
    failures: u32,
    opened_at: Option<Instant>,
    threshold: u32,
    cooling: Duration,
}

impl Default for EndpointCircuit {
    fn default() -> Self {
        Self::new(COOLING_THRESHOLD, COOLING_DURATION)
    }
}

impl EndpointCircuit {
    pub fn new(threshold: u32, cooling: Duration) -> Self {
        Self {
            failures: 0,
            opened_at: None,
            threshold: threshold.max(1),
            cooling,
        }
    }

    pub fn state(&mut self, now: Instant) -> CircuitState {
        if let Some(opened_at) = self.opened_at {
            if now.saturating_duration_since(opened_at) >= self.cooling {
                self.failures = 0;
                self.opened_at = None;
                CircuitState::Closed
            } else {
                CircuitState::Open { opened_at }
            }
        } else {
            CircuitState::Closed
        }
    }

    pub fn is_closed(&mut self, now: Instant) -> bool {
        self.state(now) == CircuitState::Closed
    }

    pub fn on_final_failure(&mut self, now: Instant) {
        self.failures = self.failures.saturating_add(1);
        if self.failures >= self.threshold && self.opened_at.is_none() {
            self.opened_at = Some(now);
        }
    }

    pub fn on_success(&mut self) {
        self.failures = 0;
        self.opened_at = None;
    }

    pub const fn failures(&self) -> u32 {
        self.failures
    }
}

/// 并发闸门：默认上限 3，FIFO 等待，RAII 释放。
#[derive(Debug)]
pub struct ConcurrencyGate {
    limit: usize,
    active: Mutex<usize>,
    available: Condvar,
}

#[derive(Debug)]
pub struct GatePermit<'a> {
    gate: &'a ConcurrencyGate,
}

impl Drop for GatePermit<'_> {
    fn drop(&mut self) {
        let mut active = self
            .gate
            .active
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *active = active.saturating_sub(1);
        self.gate.available.notify_one();
    }
}

impl ConcurrencyGate {
    pub fn new(limit: usize) -> Self {
        Self {
            limit: limit.max(1),
            active: Mutex::new(0),
            available: Condvar::new(),
        }
    }

    pub const fn limit(&self) -> usize {
        self.limit
    }

    pub fn acquire(
        &self,
        deadline: Instant,
        cancel: &CancelToken,
    ) -> Result<GatePermit<'_>, DegradeCause> {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        loop {
            if cancel.is_cancelled() {
                return Err(DegradeCause::QueueTimeout);
            }
            if *active < self.limit {
                *active += 1;
                return Ok(GatePermit { gate: self });
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(DegradeCause::QueueTimeout);
            }
            let wait = deadline
                .saturating_duration_since(now)
                .min(Duration::from_millis(10));
            let (guard, _timeout) = self
                .available
                .wait_timeout(active, wait)
                .unwrap_or_else(|poison| poison.into_inner());
            active = guard;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_errors_map_to_six_classes() {
        let cases = [
            (TransportError::DnsFailure, FailureClass::Offline),
            (TransportError::NetworkUnreachable, FailureClass::Offline),
            (TransportError::ConnectRefused, FailureClass::Unavailable),
            (TransportError::TlsFailure, FailureClass::Unavailable),
            (TransportError::PeerClosed, FailureClass::Unavailable),
            (TransportError::ConnectTimeout, FailureClass::Timeout),
            (TransportError::ReadTimeout, FailureClass::Timeout),
            (TransportError::TooLarge, FailureClass::InvalidSchema),
            (TransportError::Malformed, FailureClass::InvalidSchema),
        ];
        for (error, expected) in cases {
            assert_eq!(classify(&error), Some(expected), "{error:?}");
        }
        assert_eq!(classify(&TransportError::Cancelled), None);
    }

    #[test]
    fn status_codes_are_classified() {
        assert_eq!(classify_status(429), Some(FailureClass::RateLimited));
        assert_eq!(classify_status(500), Some(FailureClass::Unavailable));
        assert_eq!(classify_status(302), Some(FailureClass::Unavailable));
        assert_eq!(classify_status(200), None);
    }

    #[test]
    fn retry_allowance_matches_table() {
        assert_eq!(retry_allowance(FailureClass::Timeout), 2);
        assert_eq!(retry_allowance(FailureClass::Unavailable), 2);
        assert_eq!(retry_allowance(FailureClass::RateLimited), 1);
        assert_eq!(retry_allowance(FailureClass::InvalidSchema), 1);
        assert_eq!(retry_allowance(FailureClass::ContentFiltered), 0);
        assert_eq!(retry_allowance(FailureClass::Offline), 0);
    }

    #[test]
    fn deadline_wins_over_retry() {
        let decision = decide_retry(
            FailureClass::Timeout,
            1,
            Duration::from_millis(20),
            None,
            "req-1",
        );
        assert!(matches!(decision, RetryDecision::Degrade { .. }));
    }

    #[test]
    fn retry_after_over_two_seconds_degrades() {
        let decision = decide_retry(
            FailureClass::RateLimited,
            1,
            Duration::from_secs(5),
            Some(Duration::from_secs(5)),
            "req-2",
        );
        assert!(matches!(decision, RetryDecision::Degrade { .. }));
    }

    #[test]
    fn retry_after_one_second_waits_within_budget() {
        let decision = decide_retry(
            FailureClass::RateLimited,
            1,
            Duration::from_secs(3),
            Some(Duration::from_secs(1)),
            "req-3",
        );
        assert_eq!(
            decision,
            RetryDecision::Retry {
                delay: Duration::from_secs(1)
            }
        );
    }

    #[test]
    fn jitter_is_deterministic() {
        let first = jittered_backoff(FailureClass::RateLimited, 1, "req-4");
        let second = jittered_backoff(FailureClass::RateLimited, 1, "req-4");
        assert_eq!(first, second);
        assert!(first > Duration::ZERO);
        assert!(first <= Duration::from_millis(960));
    }

    #[test]
    fn retry_after_parser_rules() {
        assert_eq!(parse_retry_after(Some("1")), Some(Duration::from_secs(1)));
        assert_eq!(
            parse_retry_after(Some("Wed, 21 Oct 2015 07:28:00 GMT")),
            None
        );
        assert_eq!(parse_retry_after(Some("abc")), None);
        assert_eq!(parse_retry_after(None), None);
    }

    #[test]
    fn circuit_opens_after_three_failures_and_recovers() {
        let clock = FakeClock::new();
        let mut circuit = EndpointCircuit::default();
        for _ in 0..3 {
            circuit.on_final_failure(clock.now());
        }
        assert!(matches!(
            circuit.state(clock.now()),
            CircuitState::Open { .. }
        ));
        clock.advance(Duration::from_secs(60));
        assert_eq!(circuit.state(clock.now()), CircuitState::Closed);
    }

    #[test]
    fn success_resets_failure_count() {
        let clock = FakeClock::new();
        let mut circuit = EndpointCircuit::default();
        circuit.on_final_failure(clock.now());
        circuit.on_success();
        circuit.on_final_failure(clock.now());
        assert_eq!(circuit.failures(), 1);
        assert_eq!(circuit.state(clock.now()), CircuitState::Closed);
    }

    #[test]
    fn gate_limits_concurrency_and_releases() {
        let gate = ConcurrencyGate::new(1);
        let token = CancelToken::new();
        let deadline = Instant::now() + Duration::from_secs(1);
        let first = gate.acquire(deadline, &token).expect("第一许可");
        let second = gate.acquire(Instant::now() + Duration::from_millis(30), &token);
        assert_eq!(second.expect_err("应排队超时"), DegradeCause::QueueTimeout);
        drop(first);
        assert!(gate.acquire(deadline, &token).is_ok());
    }

    #[test]
    fn cancelled_waiter_leaves_immediately() {
        let gate = ConcurrencyGate::new(1);
        let token = CancelToken::new();
        let deadline = Instant::now() + Duration::from_secs(1);
        let _permit = gate.acquire(deadline, &token).expect("第一许可");
        token.cancel();
        assert_eq!(
            gate.acquire(deadline, &token).expect_err("取消"),
            DegradeCause::QueueTimeout
        );
    }
}
