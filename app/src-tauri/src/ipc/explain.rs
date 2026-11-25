//! 选中文字查意思：优先走已配置的模型，无 Key 时回退到本机上下文。
//!
//! 合规边界：只有用户**主动选中并点击**才会把这段文字连同前后各一句发出去；
//! 不做自动外发，不发整段转写。未配置 Key 时完全不出网。

use std::sync::Arc;

use tauri::State;

use crate::ipc::card_store::CardStore;
use crate::ipc::dto::{ExplainInputDto, ExplainResultDto, IngestExplanationInputDto, IpcErrorDto};
use crate::ipc::search::SearchState;
use crate::ipc::state::AppState;
use crate::ipc::transcript::TranscriptLog;

/// 单次选中上限：再长就不是"查词"了，也避免把整段会议发出去。
const MAX_SELECTION: usize = 200;
/// 上下文上限：只取前后各一句，且总量封顶。
const MAX_CONTEXT: usize = 600;
/// 请求超时：短于探针，避免用户等太久。
/// 多句/整段的判定门槛：太短的"好的。收到。"仍按术语处理。
const PASSAGE_MIN_CHARS: usize = 24;
const PASSAGE_MIN_SENTENCES: usize = 2;
const MAX_TOKENS_TERM: u16 = 320;
const MAX_TOKENS_PASSAGE: u16 = 420;

/// 术语/短语：先给在此处的意思，再补通用含义。
const SYSTEM_PROMPT_TERM: &str = "你是会议实时助手。用户会给你一段在会议转写里选中的词或短语，\
以及它前后的上下文。用简体中文回答，总长不超过 120 字，纯文本、不要 Markdown、不要罗列项目符号：\
先用一句话说明它在这里指什么；若是专业术语、缩写或产品名，补充它通常的含义（确定的英文全称可以给出）；\
若上下文能看出它在讨论里的作用，用半句话点出来。不要复述原文，不要编造数字或结论，不确定就直说。";

/// 多句/整段：概括 + 关键信息，不逐句翻译。
const SYSTEM_PROMPT_PASSAGE: &str = "你是会议实时助手。用户选中的是多句或整段会议转写，\
可能带识别错字与口语重复。用简体中文回答，总长不超过 180 字，纯文本、不要 Markdown、不要罗列项目符号：\
先用一句话概括这段在讲什么；再点出最多两个关键信息（数字、结论、待办、争议点）；\
若出现术语或缩写，顺带用半句话解释。不要逐句翻译或逐词解释，不要复述原文，不确定就直说。";

/// 释义入口：校验 → 取凭据 → 出网或本地回退。
pub fn explain_impl(
    state: &AppState,
    transcripts: &TranscriptLog,
    input: ExplainInputDto,
) -> Result<ExplainResultDto, IpcErrorDto> {
    let text = input.text.trim();
    if text.is_empty() {
        return Err(IpcErrorDto::new(
            "empty_selection",
            "请先选中一段文字。",
            false,
        ));
    }
    if text.chars().count() > MAX_SELECTION {
        return Err(IpcErrorDto::new(
            "selection_too_long",
            "选中的文字过长，请控制在 200 字以内。",
            false,
        ));
    }
    let given: String = input.context.trim().chars().take(MAX_CONTEXT).collect();
    let context = if given.is_empty() {
        transcripts.context_around(text)
    } else {
        given
    };

    let secret = crate::ipc::llm::read_secret(state)?;
    let Some(secret) = secret else {
        return Ok(local_fallback(
            transcripts,
            text,
            "尚未配置 API Key，以下仅为本机上下文。",
        ));
    };
    let model = crate::ipc::llm::current_model(state)?;

    let system = system_prompt_for(text);
    let max_tokens = if is_passage(text) {
        MAX_TOKENS_PASSAGE
    } else {
        MAX_TOKENS_TERM
    };
    let user = build_user_message(text, &context);
    match crate::ipc::llm::call_llm(&secret, &model, system, &user, max_tokens) {
        Ok(explanation) => Ok(ExplainResultDto {
            text: text.to_owned(),
            explanation,
            source: "llm".to_owned(),
        }),
        // 出网失败但仍能给出本机上下文时，不让用户空手而归。
        Err(error) if error.retryable => Ok(local_fallback(
            transcripts,
            text,
            &format!("{} 以下仅为本机上下文。", error.message),
        )),
        Err(error) => Err(error),
    }
}

/// 是否是"多句/整段"：句数够且总长够，避免把"好的。收到。"当整段处理。
fn is_passage(text: &str) -> bool {
    text.chars().count() >= PASSAGE_MIN_CHARS
        && crate::ipc::transcript::split_sentences(text).len() >= PASSAGE_MIN_SENTENCES
}

fn system_prompt_for(text: &str) -> &'static str {
    if is_passage(text) {
        SYSTEM_PROMPT_PASSAGE
    } else {
        SYSTEM_PROMPT_TERM
    }
}

/// 请求体里显式标出"多句"，让模型知道该概括而不是逐词解释。
fn build_user_message(text: &str, context: &str) -> String {
    let label = if is_passage(text) {
        "选中（多句）"
    } else {
        "选中"
    };
    if context.is_empty() {
        format!("{label}：{text}")
    } else {
        format!("{label}：{text}\n上下文：{context}")
    }
}

fn local_fallback(transcripts: &TranscriptLog, text: &str, note: &str) -> ExplainResultDto {
    let hits = transcripts.hits(text);
    let context = transcripts.context_for(text);
    let explanation = if hits == 0 {
        format!("{note}本次转写里还没有出现这段文字。")
    } else if context.is_empty() {
        format!("{note}该内容在本次转写里出现 {hits} 次。")
    } else {
        format!("{note}该内容在本次转写里出现 {hits} 次。相关上下文：{context}")
    };
    ExplainResultDto {
        text: text.to_owned(),
        explanation,
        source: "local".to_owned(),
    }
}

/// 选中文字查意思；未配置 Key 或未启用传输时回退到本机上下文。
#[tauri::command]
pub fn explain_selection(
    state: State<'_, AppState>,
    transcripts: State<'_, Arc<TranscriptLog>>,
    input: ExplainInputDto,
) -> Result<ExplainResultDto, IpcErrorDto> {
    explain_impl(state.inner(), transcripts.inner(), input)
}

/// 把一次释义落成卡片，进入现有卡片列表。
///
/// 来源顺序：界面上已「联网查」过的来源 > 用户开启"出卡自动补来源"时现查 > 本机片段。
/// 自动补来源只在用户已显式开启时发生，查不到就静默退回本机引用，不打断出卡。
#[tauri::command]
pub fn ingest_explanation(
    cards: State<'_, Arc<CardStore>>,
    search: State<'_, Arc<SearchState>>,
    input: IngestExplanationInputDto,
) -> Result<usize, IpcErrorDto> {
    let sources = input.sources.clone();
    let config = search.config();
    if sources.is_empty() && config.auto && config.ready {
        let (network, latency_ms) = search.network_snapshot();
        tracing::debug!(
            target: "search",
            state = network.as_str(),
            latency_ms = latency_ms.unwrap_or(-1),
            "出卡自动补来源：准备出网"
        );
        // 先出卡（无来源），断网也能拿到卡片；再落词与卡片副本，最后外发补来源。
        let added = cards.ingest_explanation_with_sources(&input.word, &input.explanation, &[])?;
        if added == 0 {
            return Ok(0);
        }
        // SPEC 7.2.1-5：先落库为 pending，再允许外发；落库失败时不外发。
        let queued = search.outbox_enqueue_card(&input.word, &input.explanation);
        if queued.is_none() && search.outbox_ready() {
            tracing::warn!(target: "search", "outbox 落库失败，跳过自动补来源（本地要点模式）");
            return Ok(added);
        }
        match search.query(&input.word) {
            Ok(response) => {
                let sites: Vec<String> = response
                    .hits
                    .iter()
                    .map(|hit| hit.site.clone())
                    .filter(|site| !site.is_empty())
                    .take(3)
                    .collect();
                if !sites.is_empty() {
                    cards.attach_sources(&input.word, &sites);
                }
                if queued.is_some() {
                    search.outbox_settle(&input.word, true);
                }
            }
            Err(_) => {
                // 失败保留在 outbox，等网络恢复后由回放窗口补齐。
                if queued.is_some() {
                    search.outbox_settle(&input.word, false);
                }
            }
        }
        return Ok(added);
    }
    cards.ingest_explanation_with_sources(&input.word, &input.explanation, &sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_exchange_stays_term_mode() {
        assert!(!is_passage("好的。收到。"));
        assert_eq!(system_prompt_for("好的。收到。"), SYSTEM_PROMPT_TERM);
    }

    #[test]
    fn long_multi_sentence_selection_uses_passage_prompt() {
        let text = "我们这次灰度范围限定在华中两个站点。先看 SLA 指标，再决定要不要扩到华东。";
        assert!(is_passage(text));
        assert_eq!(system_prompt_for(text), SYSTEM_PROMPT_PASSAGE);
        assert!(build_user_message(text, "前一句 后一句").contains("选中（多句）"));
    }

    #[test]
    fn term_message_carries_context() {
        let message = build_user_message("灰度", "前面一句 后面一句");
        assert!(message.starts_with("选中：灰度"), "{message}");
        assert!(message.contains("上下文：前面一句 后面一句"), "{message}");
    }

    #[test]
    fn empty_context_omits_context_line() {
        assert!(!build_user_message("SLA", "").contains("上下文"));
    }
}
