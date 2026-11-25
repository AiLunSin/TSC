//! 质量事件命令：自动展示、前端可见、用户反馈与统计出口。
//!
//! 只处理卡 ID 与枚举，不接收也不返回转写正文或关键词原文。

use std::sync::Arc;

use tauri::State;

use crate::ipc::card_store::CardStore;
use crate::ipc::dto::{CardFeedbackInputDto, CardRenderedInputDto, IpcErrorDto, QualityMetricsDto};

#[tauri::command]
pub fn card_feedback(
    cards: State<'_, Arc<CardStore>>,
    input: CardFeedbackInputDto,
) -> Result<bool, IpcErrorDto> {
    cards.record_feedback(&input.card_id, &input.outcome)
}

#[tauri::command]
pub fn card_rendered(
    cards: State<'_, Arc<CardStore>>,
    input: CardRenderedInputDto,
) -> Result<(), IpcErrorDto> {
    cards.mark_rendered(&input.card_id)
}

/// `hours` 省略或为 0 表示不限时间窗；上限 30 天，避免误传超大窗口。
#[tauri::command]
pub fn quality_metrics(
    cards: State<'_, Arc<CardStore>>,
    hours: Option<u32>,
) -> Result<QualityMetricsDto, IpcErrorDto> {
    Ok(cards.quality_metrics(hours)?.into())
}
