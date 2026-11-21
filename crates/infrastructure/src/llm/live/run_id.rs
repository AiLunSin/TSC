//! 运行标识：同进程共享一个 run_id；跨进程每次不同。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn fresh_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}-{}-{seq:x}", std::process::id())
}

/// 同一进程内 8 个 ignored 用例共享同一目录。
pub fn shared_run_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(fresh_run_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_ids_differ_across_calls() {
        let first = fresh_run_id();
        let second = fresh_run_id();
        assert_ne!(first, second);
    }

    #[test]
    fn shared_id_is_stable() {
        assert_eq!(shared_run_id(), shared_run_id());
    }
}
