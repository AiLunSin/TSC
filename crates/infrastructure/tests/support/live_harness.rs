//! 真实联调测试脚手架：进程级台账、序号递推、三项三态与中止标志。
//!
//! 只被 `#[ignore]` 的真实用例使用；离线用例不得依赖本模块。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use meeting_copilot_infrastructure::llm::live::budget::BudgetLedger;
use meeting_copilot_infrastructure::llm::live::record::{LiveRecorder, SubStatus, SubStatusState};

static LEDGER: OnceLock<Arc<BudgetLedger>> = OnceLock::new();
static REGISTRY: OnceLock<SubStatusRegistry> = OnceLock::new();
static ABORT: OnceLock<AbortFlag> = OnceLock::new();

/// 进程内唯一预算台账；`limit` 仅在首次初始化时生效。
pub fn shared_ledger(limit: u64) -> Arc<BudgetLedger> {
    LEDGER
        .get_or_init(|| Arc::new(BudgetLedger::new(limit)))
        .clone()
}

/// 序号递推：`samples.jsonl` 现有非空行数 + 1。
pub fn next_seq(recorder: &LiveRecorder) -> u32 {
    let count = std::fs::read_to_string(recorder.samples_path())
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
        })
        .unwrap_or(0);
    u32::try_from(count + 1).unwrap_or(u32::MAX)
}

fn default_status() -> Vec<SubStatus> {
    vec![
        SubStatus {
            item: "ac23".into(),
            status: SubStatusState::NotMeasured,
            reason_code: Some("not_executed".into()),
        },
        SubStatus {
            item: "ac24".into(),
            status: SubStatusState::NotMeasured,
            reason_code: Some("not_executed".into()),
        },
        SubStatus {
            item: "ac25".into(),
            status: SubStatusState::NotMeasured,
            reason_code: Some("construction_conflict".into()),
        },
        SubStatus {
            item: "ac35".into(),
            status: SubStatusState::Measured,
            reason_code: None,
        },
    ]
}

/// 三项三态注册表：`ac23` / `ac24` / `ac25`，并保留既有 `ac35`。
pub struct SubStatusRegistry {
    items: Mutex<Vec<SubStatus>>,
}

impl SubStatusRegistry {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(default_status()),
        }
    }

    pub fn global() -> &'static Self {
        REGISTRY.get_or_init(Self::new)
    }

    pub fn set(&self, item: &str, status: SubStatusState, reason: Option<&str>) {
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some(slot) = items.iter_mut().find(|slot| slot.item == item) {
            slot.status = status;
            slot.reason_code = reason.map(str::to_owned);
        } else {
            items.push(SubStatus {
                item: item.to_owned(),
                status,
                reason_code: reason.map(str::to_owned),
            });
        }
    }

    pub fn snapshot(&self) -> Vec<SubStatus> {
        self.items
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

impl Default for SubStatusRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 失败即停：首个不可恢复失败后置位，后续用例在构造传输前直接返回。
pub struct AbortFlag {
    flag: AtomicBool,
    reason: Mutex<Option<String>>,
}

impl AbortFlag {
    pub fn global() -> &'static Self {
        ABORT.get_or_init(|| Self {
            flag: AtomicBool::new(false),
            reason: Mutex::new(None),
        })
    }

    pub fn set(&self, reason: &str) {
        let mut slot = self
            .reason
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if slot.is_none() {
            *slot = Some(reason.to_owned());
        }
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_set(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    pub fn reason(&self) -> Option<String> {
        self.reason
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    pub fn clear(&self) {
        self.flag.store(false, Ordering::SeqCst);
        let mut slot = self
            .reason
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *slot = None;
    }
}

/// 一次性整体生成 `summary.md`：固定键值，不含请求内容与模型名。
pub fn write_summary(recorder: &LiveRecorder, ac23_line: &str, ac24_line: &str) {
    let path = recorder.root().join("summary.md");
    let body = format!(
        "风险提示：本文件为真实联调观测摘要，不构成三平台稳定性结论。\n\
         {ac23_line}\n\
         {ac24_line}\n\
         ac25_not_measured reason=construction_conflict\n"
    );
    let _ = std::fs::create_dir_all(recorder.root());
    let _ = std::fs::write(path, body);
}
