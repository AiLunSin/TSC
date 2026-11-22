import { useCallback, useEffect, useState } from "react";

import { invokeCommand } from "../bridge/tauri";
import type {
  EndpointChoiceDto,
  ModelChoiceDto,
  SettingsSnapshotDto,
} from "../bridge/types";

const FIELD_ISSUE_TEXT: Record<string, string> = {
  api_key_missing: "请填写访问密钥。",
  api_key_too_short: "访问密钥过短，请检查。",
  api_key_has_whitespace: "访问密钥不能包含空白字符。",
  model_missing: "请选择可用模型。",
  model_too_long: "模型名称过长。",
  endpoint_unavailable: "服务地址不可用。",
};

const FORM_STATE_TEXT: Record<string, string> = {
  not_configured: "尚未配置",
  configured_unverified: "已保存，未验证",
  testing: "测试中",
  verified: "已验证",
  failed: "验证失败",
  cooling: "冷却中",
};

export function modelLabel(choice: ModelChoiceDto): string {
  if ("preset" in choice) {
    return choice.preset.displayName;
  }
  return choice.custom.name;
}

export function endpointLabel(choice: EndpointChoiceDto): string {
  if (choice === "unavailable") {
    return "暂不可用";
  }
  return choice.option.displayName;
}

function issueText(codes: readonly string[]): string {
  return codes
    .map((code) => FIELD_ISSUE_TEXT[code] ?? "输入不符合要求。")
    .join(" ");
}

export function SettingsPage() {
  const [snapshot, setSnapshot] = useState<SettingsSnapshotDto | null>(null);
  const [keyDraft, setKeyDraft] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const [errorText, setErrorText] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [advancedOpen, setAdvancedOpen] = useState(false);

  const load = useCallback(async () => {
    try {
      const next = await invokeCommand<SettingsSnapshotDto>("settings_snapshot");
      setSnapshot(next);
      setErrorText(null);
    } catch {
      setErrorText("设置暂不可用，请稍后重试。");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (errorText !== null && snapshot === null) {
    return (
      <section className="panel settings" aria-label="设置">
        <h2>设置</h2>
        <p className="settings-error" role="alert">
          {errorText}
        </p>
      </section>
    );
  }

  if (snapshot === null) {
    return (
      <section className="panel settings" aria-label="设置">
        <h2>设置</h2>
        <p className="hint">正在读取设置…</p>
      </section>
    );
  }

  const modelIndex = 0;
  const endpointIndex = 0;
  const inlineIssues = issueText(snapshot.inlineErrors);

  async function probe(): Promise<void> {
    if (busy) {
      return;
    }
    setBusy(true);
    try {
      await invokeCommand("settings_probe", {
        input: { key: keyDraft, modelIndex, endpointIndex, timeoutMs: null },
      });
      await load();
    } catch {
      setErrorText("测试失败，请稍后重试。");
    } finally {
      setBusy(false);
    }
  }

  async function save(): Promise<void> {
    if (busy) {
      return;
    }
    setBusy(true);
    try {
      await invokeCommand("settings_save", {
        input: { key: keyDraft, modelIndex, endpointIndex, timeoutMs: null },
      });
      setKeyDraft("");
      setNotice("设置已保存。");
      await load();
    } catch {
      setErrorText("设置未保存，请稍后重试。");
    } finally {
      setBusy(false);
    }
  }

  async function clear(): Promise<void> {
    if (busy) {
      return;
    }
    setBusy(true);
    try {
      await invokeCommand("settings_clear", { input: { confirm: true } });
      setNotice("设置已清除。");
      await load();
    } catch {
      setErrorText("清除失败，请稍后重试。");
    } finally {
      setBusy(false);
    }
  }

  const advancedVisible = advancedOpen || !snapshot.advancedCollapsed;

  return (
    <section className="panel settings" aria-label="设置">
      <div className="panel-head">
        <h2>设置</h2>
        <span className="hint">{FORM_STATE_TEXT[snapshot.formState] ?? "未知状态"}</span>
      </div>

      <div className="settings-section">
        <h3>{snapshot.sectionsOrder[0]}</h3>
        <label className="field">
          <span>访问密钥</span>
          <input
            type="password"
            autoComplete="off"
            value={keyDraft}
            placeholder={snapshot.hasKey ? "已保存，重新输入可覆盖" : "粘贴访问密钥"}
            onChange={(event) => {
              setKeyDraft(event.target.value);
            }}
          />
        </label>
        <p className="hint">密钥只用于本次保存，不回显、不落盘。</p>
        {inlineIssues === "" ? null : (
          <p className="settings-error" role="alert">
            {inlineIssues}
          </p>
        )}
      </div>

      <div className="settings-section">
        <h3>{snapshot.sectionsOrder[1]}</h3>
        <p className="settings-value">{modelLabel(snapshot.modelChoice)}</p>
      </div>

      <div className="settings-section">
        <h3>{snapshot.sectionsOrder[2]}</h3>
        <p className="settings-value">{endpointLabel(snapshot.endpointChoice)}</p>
      </div>

      <div className="settings-section">
        <h3>{snapshot.sectionsOrder[3]}</h3>
        <div className="settings-actions">
          <button
            className="btn btn-start"
            type="button"
            disabled={!snapshot.canTest || busy}
            onClick={() => {
              void probe();
            }}
          >
            测试连接
          </button>
          <button
            className="btn btn-start"
            type="button"
            disabled={!snapshot.canSave || busy}
            onClick={() => {
              void save();
            }}
          >
            保存设置
          </button>
          <button
            className="btn"
            type="button"
            disabled={busy}
            onClick={() => {
              void clear();
            }}
          >
            清除设置
          </button>
        </div>
      </div>

      <button
        className="btn settings-toggle"
        type="button"
        aria-expanded={advancedVisible}
        onClick={() => {
          setAdvancedOpen((prev) => !prev);
        }}
      >
        {advancedVisible ? "收起高级设置" : "展开高级设置"}
      </button>

      {advancedVisible ? (
        <dl className="settings-advanced">
          <div>
            <dt>探针回执</dt>
            <dd>
              {snapshot.probeReport === null
                ? "暂无"
                : `${snapshot.probeReport.succeeded ? "成功" : "失败"} · ${snapshot.probeReport.message}`}
            </dd>
          </div>
          <div>
            <dt>密钥状态</dt>
            <dd>{snapshot.keyDisplay === "masked" ? "已保存（掩码）" : "未保存"}</dd>
          </div>
        </dl>
      ) : null}

      {notice === null ? null : (
        <p className="settings-notice" role="status">
          {notice}
        </p>
      )}
      {errorText === null ? null : (
        <p className="settings-error" role="alert">
          {errorText}
        </p>
      )}
    </section>
  );
}