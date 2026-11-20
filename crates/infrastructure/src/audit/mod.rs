//! 只追加审计表与哈希链（SPEC 6.8、DESIGN 2.7.4）。

use sha2::{Digest, Sha256};

use meeting_copilot_domain::model::enums::AuditAction;
use meeting_copilot_domain::model::governance::AuditRecord;
use meeting_copilot_domain::vo::{ActorRef, AuditId, ResourceRef, WallClock};

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn digest(prev_hash: &str, canonical: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(canonical.as_bytes());
    let bytes = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in bytes {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

fn canonical(record: &AuditRecord) -> String {
    format!(
        "{:?}|{}|{}|{}|{}",
        record.action,
        record.actor,
        record.occurred_at.as_unix_millis(),
        record.target,
        record.summary
    )
}

/// 内存哈希链实现。删除/更新方法刻意不存在（编译期不可达）。
#[derive(Debug, Default)]
pub struct AuditChain {
    records: Vec<AuditRecord>,
    anchors: Vec<String>,
    anchor_interval: usize,
}

impl AuditChain {
    pub const DEFAULT_ANCHOR_INTERVAL: usize = 1_000;

    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            anchors: Vec::new(),
            anchor_interval: Self::DEFAULT_ANCHOR_INTERVAL,
        }
    }

    pub fn with_anchor_interval(interval: usize) -> Self {
        Self {
            anchor_interval: interval.max(1),
            ..Self::new()
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }

    pub fn anchors(&self) -> &[String] {
        &self.anchors
    }

    #[allow(clippy::too_many_arguments)]
    pub fn append(
        &mut self,
        id: AuditId,
        action: AuditAction,
        actor: ActorRef,
        occurred_at: WallClock,
        target: ResourceRef,
        summary: impl Into<String>,
    ) -> &AuditRecord {
        let prev_hash = self
            .records
            .last()
            .map_or_else(|| GENESIS_HASH.to_owned(), |last| last.self_hash.clone());

        let mut record = AuditRecord {
            id,
            action,
            actor,
            occurred_at,
            target,
            summary: summary.into(),
            prev_hash,
            self_hash: String::new(),
        };
        record.self_hash = digest(&record.prev_hash, &canonical(&record));
        self.records.push(record);

        if self.records.len() % self.anchor_interval == 0 {
            let anchor = digest(
                self.records.last().map_or(GENESIS_HASH, |r| r.self_hash.as_str()),
                "anchor",
            );
            self.anchors.push(anchor);
        }

        self.records.last().expect("刚追加的记录必然存在")
    }

    /// 返回首个链断裂的记录 ID，链完整时返回 `None`。
    pub fn verify(&self) -> Option<AuditId> {
        let mut previous = GENESIS_HASH.to_owned();
        for record in &self.records {
            let expected = digest(&previous, &canonical(record));
            if record.prev_hash != previous || record.self_hash != expected {
                return Some(record.id.clone());
            }
            previous = record.self_hash.clone();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn append_n(chain: &mut AuditChain, n: usize) {
        for index in 0..n {
            chain.append(
                AuditId::new(format!("audit-{index}")).expect("id"),
                AuditAction::CardShown,
                ActorRef::new("actor-1").expect("actor"),
                WallClock::from_unix_millis(i64::try_from(index).expect("index")),
                ResourceRef::new("card-1").expect("target"),
                format!("展示第 {index} 张卡"),
            );
        }
    }

    #[test]
    fn chain_verifies_after_appends() {
        let mut chain = AuditChain::new();
        append_n(&mut chain, 50);
        assert_eq!(chain.verify(), None);
        assert_eq!(chain.len(), 50);
    }

    #[test]
    fn tampering_is_localized() {
        let mut chain = AuditChain::new();
        append_n(&mut chain, 10);
        chain.records[4].summary = "被篡改".to_owned();
        assert_eq!(
            chain.verify(),
            Some(AuditId::new("audit-4").expect("id"))
        );
    }

    #[test]
    fn anchors_written_every_interval() {
        let mut chain = AuditChain::with_anchor_interval(10);
        append_n(&mut chain, 25);
        assert_eq!(chain.anchors().len(), 2);
    }
}