//! 联调记录：只写 `.tmp/llm-live/<run-id>/`，原子写，三态标注。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use meeting_copilot_domain::error::CopilotError;
use serde::Serialize;

#[cfg(test)]
use super::policy::{path_confirmed, PATH_CONSTANT};
use super::stats::Sample;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubStatusState {
    Measured,
    NotTriggered,
    NotMeasured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SubStatus {
    pub item: String,
    pub status: SubStatusState,
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunRecord {
    pub schema_version: String,
    pub endpoint_host: String,
    pub model: String,
    pub started_at: String,
    pub ended_at: String,
    pub tz_offset: String,
    pub commit_short: String,
    pub config: String,
    pub per_task: usize,
    pub sub_status: Vec<SubStatus>,
    pub samples_path: String,
    pub stats_rule: String,
    pub build: String,
    pub path_confirmed: bool,
    pub call_limit: u64,
    pub calls_used: u64,
    pub budget_hit: bool,
    pub model_confirmed: bool,
    pub host_locked: bool,
    pub data_terms_declaration: String,
    pub data_terms_accepted_at: String,
    pub credential_rotated: bool,
    pub tls_trust_root: Option<String>,
}

pub const TERMS_DECLARATION: &str = "自用；已接受服务商条款";
/// C5（未轮换）固定风险文案，写入 summary.md 首行。
pub const CREDENTIAL_RISK: &str = "凭据未轮换·存在既暴露风险";

impl RunRecord {
    pub fn validate(&self) -> Result<(), CopilotError> {
        if self.schema_version != "llm.live.v2" {
            return Err(invalid("schema_version: 必须为 llm.live.v2"));
        }
        if self.data_terms_declaration != TERMS_DECLARATION {
            return Err(invalid("data_terms_declaration: 不完整"));
        }
        if !is_rfc3339(&self.data_terms_accepted_at) {
            return Err(invalid("data_terms_accepted_at: 不完整"));
        }
        if let Some(root) = self.tls_trust_root.as_deref() {
            if !matches!(root, "ok" | "tls_trust_root" | "other") {
                return Err(invalid("tls_trust_root: 非法枚举"));
            }
        }
        let all_not_measured = self
            .sub_status
            .iter()
            .all(|item| item.status == SubStatusState::NotMeasured);
        if all_not_measured && (self.calls_used != 0 || self.budget_hit) {
            return Err(invalid("calls_used: 未实测时必须为 0"));
        }
        Ok(())
    }
}

fn is_rfc3339(value: &str) -> bool {
    let Some((date, time)) = value.split_once('T') else {
        return false;
    };
    if date.len() != 10 || time.len() < 9 {
        return false;
    }
    let has_zone = time.ends_with('Z')
        || time
            .rfind(['+', '-'])
            .is_some_and(|index| time[index..].len() == 6);
    has_zone && value.chars().all(|c| c.is_ascii_graphic())
}

pub struct LiveRecorder {
    root: PathBuf,
}

fn invalid(detail: impl Into<String>) -> CopilotError {
    CopilotError::ProviderConfigInvalid {
        detail: detail.into(),
    }
}

impl LiveRecorder {
    pub fn new(run_id: impl Into<String>) -> Result<Self, CopilotError> {
        let run_id = run_id.into();
        if run_id.is_empty()
            || run_id.contains('/')
            || run_id.contains('\\')
            || run_id.contains("..")
        {
            return Err(invalid("run_id: 非法路径片段"));
        }
        Ok(Self {
            root: PathBuf::from(".tmp").join("llm-live").join(run_id),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn samples_path(&self) -> PathBuf {
        self.root.join("samples.jsonl")
    }

    fn ensure_root(&self) -> Result<(), CopilotError> {
        fs::create_dir_all(&self.root).map_err(|_| CopilotError::StoreFailure {
            detail: "创建联调记录目录失败".into(),
        })
    }

    pub fn write_run(&self, record: &RunRecord) -> Result<PathBuf, CopilotError> {
        record.validate()?;
        self.ensure_root()?;
        let path = self.root.join("run.json");
        let json = serde_json::to_vec_pretty(record).map_err(|_| CopilotError::StoreFailure {
            detail: "序列化 run.json 失败".into(),
        })?;
        atomic_write(&path, &json)?;
        Ok(path)
    }

    pub fn append_sample(&self, sample: &Sample) -> Result<(), CopilotError> {
        self.ensure_root()?;
        let line = serde_json::to_string(sample).map_err(|_| CopilotError::StoreFailure {
            detail: "序列化样本失败".into(),
        })?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.samples_path())
            .map_err(|_| CopilotError::StoreFailure {
                detail: "打开 samples.jsonl 失败".into(),
            })?;
        writeln!(file, "{line}").map_err(|_| CopilotError::StoreFailure {
            detail: "追加样本失败".into(),
        })
    }

    pub fn write_summary(&self, content: &str) -> Result<PathBuf, CopilotError> {
        self.ensure_root()?;
        let path = self.root.join("summary.md");
        let body = format!("{CREDENTIAL_RISK}\n{content}");
        atomic_write(&path, body.as_bytes())?;
        Ok(path)
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CopilotError> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(|_| CopilotError::StoreFailure {
        detail: "写入临时文件失败".into(),
    })?;
    if fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(path);
        fs::rename(&temporary, path).map_err(|_| CopilotError::StoreFailure {
            detail: "提交临时文件失败".into(),
        })?;
    }
    Ok(())
}

/// 由 `samples.jsonl` 独立重算 p50/p95 的口径版本号。
pub const STATS_RULE: &str = "nearest-rank; exclude_degraded; count_degraded_in_denominator";

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::llm::LlmTask;

    fn record() -> RunRecord {
        RunRecord {
            schema_version: "llm.live.v2".into(),
            endpoint_host: "127.0.0.1".into(),
            model: "test-model".into(),
            started_at: "2026-11-21T00:00:00Z".into(),
            ended_at: "2026-11-21T00:01:00Z".into(),
            tz_offset: "+08:00".into(),
            commit_short: "abcdef0".into(),
            config: "samples=30,deadline=6000".into(),
            per_task: 30,
            sub_status: vec![SubStatus {
                item: "ac20".into(),
                status: SubStatusState::NotMeasured,
                reason_code: Some("endpoint_missing".into()),
            }],
            samples_path: "samples.jsonl".into(),
            stats_rule: STATS_RULE.into(),
            build: "debug".into(),
            path_confirmed: path_confirmed(PATH_CONSTANT),
            call_limit: 0,
            calls_used: 0,
            budget_hit: false,
            model_confirmed: true,
            host_locked: true,
            data_terms_declaration: TERMS_DECLARATION.into(),
            data_terms_accepted_at: "2026-11-21T00:00:00+08:00".into(),
            credential_rotated: false,
            tls_trust_root: None,
        }
    }

    #[test]
    fn run_id_rejects_paths() {
        assert!(LiveRecorder::new("").is_err());
        assert!(LiveRecorder::new("../escape").is_err());
        assert!(LiveRecorder::new("a/b").is_err());
        assert!(LiveRecorder::new("20261121-abcdef0").is_ok());
    }

    #[test]
    fn partial_not_measured_allows_nonzero_calls() {
        let mut record = record();
        record.sub_status = vec![
            SubStatus {
                item: "ac23".into(),
                status: SubStatusState::Measured,
                reason_code: None,
            },
            SubStatus {
                item: "ac25".into(),
                status: SubStatusState::NotMeasured,
                reason_code: Some("construction_conflict".into()),
            },
        ];
        record.calls_used = 3;
        assert!(record.validate().is_ok());

        let mut all_not_measured = record.clone();
        all_not_measured.sub_status = vec![SubStatus {
            item: "ac23".into(),
            status: SubStatusState::NotMeasured,
            reason_code: Some("not_executed".into()),
        }];
        all_not_measured.calls_used = 1;
        assert!(all_not_measured.validate().is_err());

        let mut empty = record;
        empty.sub_status = Vec::new();
        empty.calls_used = 1;
        assert!(empty.validate().is_err());
    }

    #[test]
    fn records_are_written_under_tmp_only() {
        let recorder = LiveRecorder::new("self-test-record").expect("recorder");
        assert!(recorder
            .root()
            .starts_with(Path::new(".tmp").join("llm-live")));
        assert_eq!(
            recorder.samples_path().file_name().expect("name"),
            "samples.jsonl"
        );
        let _ = fs::remove_dir_all(recorder.root());
    }

    #[test]
    fn three_state_serialization_has_no_numbers_for_not_measured() {
        let mut record = record();
        record.sub_status = vec![SubStatus {
            item: "ac23".into(),
            status: SubStatusState::NotTriggered,
            reason_code: Some("window_120s".into()),
        }];
        let json = serde_json::to_string(&record).expect("序列化");
        assert!(json.contains("not_triggered"));
        assert!(!json.contains("\"p95\""));
    }

    #[test]
    fn atomic_write_writes_and_replaces() {
        let recorder = LiveRecorder::new("self-test-atomic").expect("recorder");
        recorder.write_run(&record()).expect("第一次写入");
        recorder.write_run(&record()).expect("第二次写入");
        let content = fs::read_to_string(recorder.root().join("run.json")).expect("读取");
        assert!(content.contains("llm.live.v2"));
        assert!(!content.contains("SENSITIVE-TOKEN"));
        let _ = fs::remove_dir_all(recorder.root());
    }

    #[test]
    fn v2_key_set_is_closed() {
        let value = serde_json::to_value(record()).expect("序列化");
        let keys: std::collections::BTreeSet<_> =
            value.as_object().expect("对象").keys().cloned().collect();
        let expected: std::collections::BTreeSet<String> = [
            "schema_version",
            "endpoint_host",
            "model",
            "started_at",
            "ended_at",
            "tz_offset",
            "commit_short",
            "config",
            "per_task",
            "sub_status",
            "samples_path",
            "stats_rule",
            "build",
            "path_confirmed",
            "call_limit",
            "calls_used",
            "budget_hit",
            "model_confirmed",
            "host_locked",
            "data_terms_declaration",
            "data_terms_accepted_at",
            "credential_rotated",
            "tls_trust_root",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(keys, expected);
    }

    #[test]
    fn missing_terms_accepted_at_is_rejected() {
        let mut missing_date = record();
        missing_date.data_terms_accepted_at = String::new();
        assert!(missing_date.validate().is_err());
        let mut missing_declaration = record();
        missing_declaration.data_terms_declaration = String::new();
        assert!(missing_declaration.validate().is_err());
    }

    #[test]
    fn sample_rows_have_no_payload_fields() {
        let sample = Sample {
            seq: 1,
            task: LlmTask::InsightExtract,
            attempt: 1,
            first_token_ms: Some(120),
            total_ms: 480,
            outcome: "ok".into(),
            failure_class: None,
            http_status: Some(200),
            degraded: false,
            excluded: None,
        };
        let line = serde_json::to_string(&sample).expect("序列化");
        assert!(!line.contains("payload"));
        assert!(!line.contains("content"));
        assert!(!line.contains("sk-"));
    }

    #[test]
    fn unrotated_flag_does_not_force_not_measured() {
        let mut record = record();
        record.sub_status = vec![SubStatus {
            item: "ac35".into(),
            status: SubStatusState::Measured,
            reason_code: None,
        }];
        record.calls_used = 1;
        assert!(!record.credential_rotated);
        assert!(record.validate().is_ok());
    }
}
