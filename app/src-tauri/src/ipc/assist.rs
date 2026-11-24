//! 辅助回复：把最近一段转写（可带选中句）交给模型，产出"该注意什么"与"可以直接怎么说"。
//!
//! 边界：只有用户点按钮才会出网；发送内容是最近 12 句（≤900 字）+ 选中的一句，
//! 不发送整场转写，不落盘、不写日志正文。

use std::sync::Arc;

use tauri::State;

use crate::ipc::dto::{AssistInputDto, AssistResultDto, IpcErrorDto};
use crate::ipc::state::AppState;
use crate::ipc::transcript::TranscriptLog;

/// 送上文的句数上限：够看出话题，又不会把整场会议发出去。
const CONTEXT_LINES: usize = 12;
/// 上下文总字数上限。
const MAX_CONTEXT_CHARS: usize = 900;
/// 选中句上限。
const MAX_FOCUS_CHARS: usize = 160;
/// 输出上限：两条回复 + 三条提醒足够，再长开会看不完。
const MAX_TOKENS: u16 = 520;
const MAX_POINTS: usize = 3;
const MAX_REPLIES: usize = 2;
const MAX_POINT_CHARS: usize = 40;
const MAX_REPLY_CHARS: usize = 80;

const SYSTEM_PROMPT: &str = "你是会议实时助手。用户正在开会，给你最近一段会议转写\
（可能带语音识别错字、口语重复），以及（可选的）他刚选中的一句话。\
请只输出一个 JSON 对象，不要 Markdown 代码块、不要任何解释文字，格式严格为：\
{\"points\":[\"...\"],\"replies\":[\"...\"]}。\
points 放 2~3 条提醒：接下来要注意什么、可能被追问什么、哪里有风险或待确认项，每条不超过 30 字，\
只写具体动作或风险，不写\"保持沟通\"这类套话。\
replies 放 1~2 条可以直接说出口的回应，每条不超过 60 字，口语、自然、不堆术语，能接上对方的话。\
不要编造数字、日期、人名或承诺；信息不足时写成\"确认…\"而不是替用户下结论。";

/// 生成辅助提醒与回复。
pub fn assist_impl(
    state: &AppState,
    transcripts: &TranscriptLog,
    input: AssistInputDto,
) -> Result<AssistResultDto, IpcErrorDto> {
    let focus: String = input.focus.trim().chars().take(MAX_FOCUS_CHARS).collect();
    let recent = recent_lines(transcripts);
    if focus.is_empty() && recent.is_empty() {
        return Err(IpcErrorDto::new(
            "no_context",
            "还没有可参考的内容：先开始转写，或先选中一段文字。",
            false,
        ));
    }
    let Some(secret) = crate::ipc::llm::read_secret(state)? else {
        return Err(IpcErrorDto::new(
            "llm_key_missing",
            "请先在设置页配置 API Key，再使用辅助回复。",
            false,
        ));
    };
    let model = crate::ipc::llm::current_model(state)?;
    let user = build_user_message(&focus, &recent);
    let raw = crate::ipc::llm::call_llm(&secret, &model, SYSTEM_PROMPT, &user, MAX_TOKENS)?;
    match parse_suggestions(&raw) {
        Some((points, replies)) => Ok(AssistResultDto {
            source: "llm".to_owned(),
            points,
            replies,
            note: String::new(),
        }),
        // 模型没按格式返回时不装作成功：把原文交回去，让用户自己看。
        None => Ok(AssistResultDto {
            source: "llm".to_owned(),
            points: Vec::new(),
            replies: Vec::new(),
            note: raw.chars().take(400).collect(),
        }),
    }
}

/// 取最近若干句，从后往前累计到字数上限；返回按时间正序。
fn recent_lines(transcripts: &TranscriptLog) -> Vec<String> {
    let lines = transcripts.snapshot();
    let mut picked: Vec<String> = Vec::new();
    let mut used = 0usize;
    for line in lines.iter().rev() {
        if picked.len() >= CONTEXT_LINES {
            break;
        }
        let cost = line.text.chars().count();
        if used + cost > MAX_CONTEXT_CHARS {
            break;
        }
        used += cost;
        picked.push(line.text.clone());
    }
    picked.reverse();
    picked
}

fn build_user_message(focus: &str, recent: &[String]) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !recent.is_empty() {
        parts.push(format!("最近转写：\n{}", recent.join("\n")));
    }
    if !focus.is_empty() {
        parts.push(format!("刚选中的一句：{focus}"));
    }
    parts.push("请按要求输出 JSON。".to_owned());
    parts.join("\n\n")
}

/// 解析模型输出：容忍 ```json 围栏和前后多余文字。
fn parse_suggestions(raw: &str) -> Option<(Vec<String>, Vec<String>)> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(&raw[start..=end]).ok()?;
    let points = collect(value.get("points"), MAX_POINTS, MAX_POINT_CHARS);
    let replies = collect(value.get("replies"), MAX_REPLIES, MAX_REPLY_CHARS);
    if points.is_empty() && replies.is_empty() {
        return None;
    }
    Some((points, replies))
}

fn collect(field: Option<&serde_json::Value>, limit: usize, max_chars: usize) -> Vec<String> {
    let Some(items) = field.and_then(|value| value.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        let Some(text) = item.as_str() else {
            continue;
        };
        let text = text.trim().trim_start_matches(['-', '*', '·']).trim();
        if text.is_empty() {
            continue;
        }
        out.push(text.chars().take(max_chars).collect());
        if out.len() >= limit {
            break;
        }
    }
    out
}

#[tauri::command]
pub fn assist_reply(
    state: State<'_, AppState>,
    transcripts: State<'_, Arc<TranscriptLog>>,
    input: AssistInputDto,
) -> Result<AssistResultDto, IpcErrorDto> {
    assist_impl(state.inner(), transcripts.inner(), input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let raw = "{\"points\":[\"确认回滚窗口\",\"补充灰度范围\"],\"replies\":[\"我这边周三前给回滚预案。\"]}";
        let (points, replies) = parse_suggestions(raw).expect("parsed");
        assert_eq!(points, vec!["确认回滚窗口", "补充灰度范围"]);
        assert_eq!(replies.len(), 1);
    }

    #[test]
    fn parses_fenced_json_with_prose() {
        let raw = "好的，结果如下：\n```json\n{\"points\": [\"- 确认 SLA 口径\"], \"replies\": [\"我确认下再回你。\"]}\n```\n以上。";
        let (points, replies) = parse_suggestions(raw).expect("parsed");
        // 列表符号被剥掉。
        assert_eq!(points, vec!["确认 SLA 口径"]);
        assert_eq!(replies, vec!["我确认下再回你。"]);
    }

    #[test]
    fn caps_counts_and_length() {
        let raw = serde_json::json!({
            "points": ["一", "二", "三", "四", "五"],
            "replies": ["a".repeat(200), "b", "c"]
        })
        .to_string();
        let (points, replies) = parse_suggestions(&raw).expect("parsed");
        assert_eq!(points.len(), MAX_POINTS);
        assert_eq!(replies.len(), MAX_REPLIES);
        assert_eq!(replies[0].chars().count(), MAX_REPLY_CHARS);
    }

    #[test]
    fn rejects_non_json_or_empty_payload() {
        assert!(parse_suggestions("抱歉，我无法回答").is_none());
        assert!(parse_suggestions("{\"points\":[],\"replies\":[]}").is_none());
    }

    #[test]
    fn user_message_carries_recent_and_focus() {
        let message = build_user_message("灰度范围", &["前面一句".to_owned()]);
        assert!(message.contains("最近转写：\n前面一句"));
        assert!(message.contains("刚选中的一句：灰度范围"));
    }
}
