//! 检索结果缓存用例（SPEC 7.2.1、DESIGN 2.10.2）。
//!
//! 缓存键只由"检索源 + 归一化查询"派生并立即散列；归一化查询原文不落库。

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::proactive::CachedQuery;
use meeting_copilot_domain::port::proactive::QueryCacheStore;
use sha2::{Digest, Sha256};

/// SHA-256(provider_profile_id ‖ '\u{1f}' ‖ normalized_query)，64 位小写十六进制。
pub fn cache_key(provider_profile_id: &str, normalized_query: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(provider_profile_id.as_bytes());
    hasher.update([0x1f]);
    hasher.update(normalized_query.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

#[derive(Debug, Clone, PartialEq)]
pub struct CacheLookup {
    pub entry: CachedQuery,
    /// 命中结果的年龄（小时），用于界面展示"结果来自 X 小时前"。
    pub age_hours: u64,
}

pub struct CacheService<'a, C: QueryCacheStore> {
    store: &'a C,
}

impl<'a, C: QueryCacheStore> CacheService<'a, C> {
    pub fn new(store: &'a C) -> Self {
        Self { store }
    }

    /// 命中且未过期时刷新访问时间；过期、时钟回拨或缺失都返回 `None`。
    pub fn lookup(
        &self,
        provider_profile_id: &str,
        normalized_query: &str,
        now_ms: i64,
    ) -> Result<Option<CacheLookup>, CopilotError> {
        let key = cache_key(provider_profile_id, normalized_query);
        let Some(entry) = self.store.get(&key)? else {
            return Ok(None);
        };
        if !entry.is_usable(now_ms) {
            return Ok(None);
        }
        self.store.touch(&key, now_ms)?;
        Ok(Some(CacheLookup {
            age_hours: entry.age_hours(now_ms),
            entry,
        }))
    }

    pub fn store_result(&self, entry: &CachedQuery) -> Result<usize, CopilotError> {
        self.store.put(entry)
    }

    pub fn sweep(&self, now_ms: i64) -> Result<usize, CopilotError> {
        self.store.sweep_expired(now_ms)
    }
}
