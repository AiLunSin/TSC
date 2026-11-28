//! 辅助回复：把最近一段转写（可带选中句）交给模型，产出"该注意什么"与"可以直接怎么说"。
//!
//! 边界：只有用户点按钮才会出网；发送内容是最近 12 句（≤900 字）+ 选中的一句，
//! 其中**提醒只依据最后 5 句**，更早的句子只作回应参考；
//! 不发送整场转写，不落盘、不写日志正文。

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use meeting_copilot_application::settings::PortError;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ipc::dto::{
    AssistConfigDto, AssistConfigInputDto, AssistInputDto, AssistResultDto, IpcErrorDto,
};
use crate::ipc::state::AppState;
use crate::ipc::transcript::TranscriptLog;

/// 配置文件：辅助回复的窗口设置。与 `settings.json`（恰 4 键）分开。
const CONFIG_FILE: &str = "assist.json";
/// 默认上下文句数：够看出话题，又不会把整场会议发出去。
const DEFAULT_CONTEXT_LINES: u32 = 12;
/// 默认提醒句数：只拿最后几句写"该注意什么"，免得反复念叨早就翻篇的话题。
const DEFAULT_TIP_LINES: u32 = 5;
/// 提醒窗口可调范围：少于 3 句看不出上下文，多于 10 句就退回"翻旧账"。
const MIN_TIP_LINES: u32 = 3;
const MAX_TIP_LINES: u32 = 10;
/// 上下文可调范围：少于 5 句接不上话，多于 20 句发送量偏大且无助于聚焦。
const MIN_CONTEXT_LINES: u32 = 5;
const MAX_CONTEXT_LINES: u32 = 20;
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
（分成「更早的对话」和「最近几句话」两段，可能带语音识别错字、口语重复），以及（可选的）他刚选中的一句话。\
请只输出一个 JSON 对象，不要 Markdown 代码块、不要任何解释文字，格式严格为：\
{\"points\":[\"...\"],\"replies\":[\"...\"]}。\
points 放 2~3 条提醒：接下来要注意什么、可能被追问什么、哪里有风险或待确认项，每条不超过 30 字，\
只写具体动作或风险，不写\"保持沟通\"这类套话；提醒只依据「最近几句话」这一段，\
不要拿更早的对话里已经翻篇的内容反复提醒。\
replies 放 1~2 条可以直接说出口的回应，每条不超过 60 字，口语、自然、不堆术语，能接上对方的话；\
可以结合更早的对话。\
不要编造数字、日期、人名或承诺；信息不足时写成\"确认…\"而不是替用户下结论。";

/// 落盘的辅助回复设置：版本不符或损坏直接回默认。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AssistFileV1 {
    schema_version: u32,
    tip_lines: u32,
    context_lines: u32,
}

impl Default for AssistFileV1 {
    fn default() -> Self {
        Self {
            schema_version: 1,
            tip_lines: DEFAULT_TIP_LINES,
            context_lines: DEFAULT_CONTEXT_LINES,
        }
    }
}

impl AssistFileV1 {
    /// 夹到允许范围，并保证提醒窗口不长于上下文窗口。
    fn sanitized(&self) -> (usize, usize) {
        let context = self
            .context_lines
            .clamp(MIN_CONTEXT_LINES, MAX_CONTEXT_LINES);
        let tip = self
            .tip_lines
            .clamp(MIN_TIP_LINES, MAX_TIP_LINES)
            .min(context);
        (tip as usize, context as usize)
    }
}

/// 辅助回复的设置存储：临时文件 + 同目录替换。
pub struct AssistStore {
    dir: PathBuf,
}

impl AssistStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn main_path(&self) -> PathBuf {
        self.dir.join(CONFIG_FILE)
    }

    fn load(&self) -> AssistFileV1 {
        let Ok(text) = fs::read_to_string(self.main_path()) else {
            return AssistFileV1::default();
        };
        match serde_json::from_str::<AssistFileV1>(&text) {
            Ok(file) if file.schema_version == 1 => file,
            _ => AssistFileV1::default(),
        }
    }

    fn save(&self, file: &AssistFileV1) -> Result<(), PortError> {
        fs::create_dir_all(&self.dir).map_err(|_| PortError::Unavailable)?;
        let json = serde_json::to_vec_pretty(file).map_err(|_| PortError::Corrupted)?;
        let tmp = self.dir.join("assist.json.tmp");
        fs::write(&tmp, json).map_err(|_| PortError::Unavailable)?;
        fs::rename(&tmp, self.main_path()).map_err(|_| PortError::Unavailable)
    }
}

/// 辅助回复运行态：只有窗口设置，凭据仍走模型 Key。
pub struct AssistState {
    pub(crate) store: AssistStore,
}

impl AssistState {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            store: AssistStore::new(dir),
        }
    }

    pub fn config(&self) -> AssistConfigDto {
        let (tip, context) = self.store.load().sanitized();
        AssistConfigDto {
            tip_lines: tip as u32,
            context_lines: context as u32,
        }
    }

    /// 保存：越界值夹到范围内，而不是报错拒存。
    pub fn save_config(
        &self,
        input: &AssistConfigInputDto,
    ) -> Result<AssistConfigDto, IpcErrorDto> {
        let file = AssistFileV1 {
            schema_version: 1,
            tip_lines: input.tip_lines,
            context_lines: input.context_lines,
        };
        self.store.save(&file).map_err(|_| {
            IpcErrorDto::new("assist_config_unwritable", "辅助回复设置保存失败。", true)
        })?;
        Ok(self.config())
    }
}

/// 生成辅助提醒与回复。
pub fn assist_impl(
    state: &AppState,
    transcripts: &TranscriptLog,
    store: &AssistStore,
    input: AssistInputDto,
) -> Result<AssistResultDto, IpcErrorDto> {
    let (tip_lines, context_lines) = store.load().sanitized();
    let focus: String = input.focus.trim().chars().take(MAX_FOCUS_CHARS).collect();
    let recent = recent_lines(transcripts, context_lines);
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
    let user = build_user_message(&focus, &recent, tip_lines);
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
fn recent_lines(transcripts: &TranscriptLog, limit: usize) -> Vec<String> {
    let lines = transcripts.snapshot();
    let mut picked: Vec<String> = Vec::new();
    let mut used = 0usize;
    for line in lines.iter().rev() {
        if picked.len() >= limit {
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

/// 拼请求正文：把上下文切成「更早的对话」与「最近 N 句」两段，
/// 让模型知道哪几句才允许用来写提醒（一次调用同时产出提醒与回应，不额外多花一次请求）。
fn build_user_message(focus: &str, recent: &[String], tip_lines: usize) -> String {
    let mut parts: Vec<String> = Vec::new();
    let split = recent.len().saturating_sub(tip_lines);
    let (earlier, latest) = recent.split_at(split);
    if !earlier.is_empty() {
        parts.push(format!(
            "更早的对话（只作背景，不要据此写提醒）：\n{}",
            earlier.join("\n")
        ));
    }
    if !latest.is_empty() {
        parts.push(format!(
            "最近 {} 句（提醒只依据这几句）：\n{}",
            latest.len(),
            latest.join("\n")
        ));
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
pub fn assist_config(state: State<'_, Arc<AssistState>>) -> AssistConfigDto {
    state.config()
}

#[tauri::command]
pub fn assist_config_save(
    state: State<'_, Arc<AssistState>>,
    input: AssistConfigInputDto,
) -> Result<AssistConfigDto, IpcErrorDto> {
    state.save_config(&input)
}

#[tauri::command]
pub fn assist_reply(
    state: State<'_, AppState>,
    transcripts: State<'_, Arc<TranscriptLog>>,
    assist: State<'_, Arc<AssistState>>,
    input: AssistInputDto,
) -> Result<AssistResultDto, IpcErrorDto> {
    assist_impl(state.inner(), transcripts.inner(), &assist.store, input)
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
    fn user_message_splits_tip_window() {
        let recent: Vec<String> = (1..=12).map(|index| format!("第{index}句")).collect();
        let message = build_user_message("", &recent, DEFAULT_TIP_LINES as usize);
        let (earlier, latest) = message.split_once("最近 5 句").expect("两段");
        // 前 7 句只作背景，最后 5 句才是提醒依据。
        assert!(earlier.contains("更早的对话"));
        assert!(earlier.contains("第1句"));
        assert!(earlier.contains("第7句"));
        assert!(!earlier.contains("第8句"));
        assert!(latest.contains("第8句"));
        assert!(latest.contains("第12句"));
    }

    #[test]
    fn window_settings_are_clamped() {
        let wide = AssistFileV1 {
            schema_version: 1,
            tip_lines: 99,
            context_lines: 99,
        };
        assert_eq!(
            wide.sanitized(),
            (MAX_TIP_LINES as usize, MAX_CONTEXT_LINES as usize)
        );

        let narrow = AssistFileV1 {
            schema_version: 1,
            tip_lines: 1,
            context_lines: 1,
        };
        assert_eq!(
            narrow.sanitized(),
            (MIN_TIP_LINES as usize, MIN_CONTEXT_LINES as usize)
        );

        // 提醒窗口不得长于上下文窗口。
        let inverted = AssistFileV1 {
            schema_version: 1,
            tip_lines: 10,
            context_lines: 6,
        };
        assert_eq!(inverted.sanitized(), (6, 6));
    }

    #[test]
    fn default_window_is_five_and_twelve() {
        assert_eq!(
            AssistFileV1::default().sanitized(),
            (DEFAULT_TIP_LINES as usize, DEFAULT_CONTEXT_LINES as usize)
        );
    }

    #[test]
    fn short_context_keeps_single_section() {
        let recent: Vec<String> = (1..=3).map(|index| format!("第{index}句")).collect();
        let message = build_user_message("灰度范围", &recent, DEFAULT_TIP_LINES as usize);
        assert!(!message.contains("更早的对话"));
        assert!(message.contains("最近 3 句（提醒只依据这几句）：\n第1句"));
        assert!(message.contains("刚选中的一句：灰度范围"));
    }
}
