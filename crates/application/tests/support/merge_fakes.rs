//! 合并计数证据用的 recording fake。
//!
//! 与 `fakes.rs` 物理隔离：本文件允许实现 `UtteranceRepository::delete`，
//! 既有 `--rule=fakes` 不放宽。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::utterance::{Correction, Utterance};
use meeting_copilot_domain::port::UtteranceRepository;
use meeting_copilot_domain::vo::{SessionId, UtteranceRef};

use meeting_copilot_application::capture::capture_ports::{InsightInvariantPort, InsightReadError};

/// 记录要点不变式读取次数。
#[derive(Default)]
pub struct RecordingInsight {
    calls: AtomicUsize,
}

impl RecordingInsight {
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl InsightInvariantPort for RecordingInsight {
    fn read_insight_ids(&self, _session: &SessionId) -> Result<Vec<String>, InsightReadError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }
}

/// 记录片段写入次数与内容。
#[derive(Default)]
pub struct RecordingRepo {
    append_calls: AtomicUsize,
    items: Mutex<Vec<Utterance>>,
}

impl RecordingRepo {
    pub fn append_calls(&self) -> usize {
        self.append_calls.load(Ordering::SeqCst)
    }

    pub fn items(&self) -> Vec<Utterance> {
        self.items
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

impl UtteranceRepository for RecordingRepo {
    fn append(&self, _session: &SessionId, utterance: &Utterance) -> Result<(), CopilotError> {
        self.append_calls.fetch_add(1, Ordering::SeqCst);
        self.items
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(utterance.clone());
        Ok(())
    }

    fn apply_correction(
        &self,
        _session: &SessionId,
        _correction: &Correction,
    ) -> Result<(), CopilotError> {
        Ok(())
    }

    fn list(&self, _session: &SessionId) -> Result<Vec<Utterance>, CopilotError> {
        Ok(self.items())
    }

    fn delete(&self, _session: &SessionId, _reference: &UtteranceRef) -> Result<(), CopilotError> {
        Ok(())
    }
}

/// 记录滚动摘要入窗条数。
#[derive(Default)]
pub struct RecordingSummary {
    window_entries: AtomicUsize,
}

impl RecordingSummary {
    pub fn record(&self, count: usize) {
        self.window_entries.fetch_add(count, Ordering::SeqCst);
    }

    pub fn window_entries(&self) -> usize {
        self.window_entries.load(Ordering::SeqCst)
    }
}