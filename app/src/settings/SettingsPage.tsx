import { useCallback, useEffect, useState } from "react";

import { invokeCommand } from "../bridge/tauri";
import type {
  AssistConfigDto,
  EndpointChoiceDto,
  ModelChoiceDto,
  ProbeReportDto,
  SettingsSnapshotDto,
} from "../bridge/types";
import type { Workbench } from "../state/useWorkbench";
import { SEARCH_PROVIDER_TEXT } from "../state/useWorkbench";

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

function ipcErrorText(error: unknown): string | null {
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = error.message;
    if (typeof message === "string" && message !== "") {
      return message;
    }
  }
  return null;
}

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

interface SettingsPageProps {
  wb: Workbench;
}

/**
 * 设置页：所有"配一次、长期用"的东西都在这里。
 *
 * 工作页只留当场要按的按钮；模型、检索、辅助窗口、显示方式与隐私口径全部收进本页，
 * 开会时不会被配置项打断。
 */
export function SettingsPage({ wb }: SettingsPageProps) {
  const [snapshot, setSnapshot] = useState<SettingsSnapshotDto | null>(null);
  const [assistConfig, setAssistConfig] = useState<AssistConfigDto | null>(null);
  const [tipLines, setTipLines] = useState(5);
  const [contextLines, setContextLines] = useState(12);
  const [assistNotice, setAssistNotice] = useState<string | null>(null);
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

  // 辅助回复的窗口设置独立于模型设置，单独读一份。
  useEffect(() => {
    void (async () => {
      try {
        const config = await invokeCommand<AssistConfigDto>("assist_config");
        setAssistConfig(config);
        setTipLines(config.tipLines);
        setContextLines(config.contextLines);
      } catch {
        // 读不到就按默认值展示
      }
    })();
  }, []);

  if (errorText !== null && snapshot === null) {
    return (
      <main className="settings-page" aria-label="设置">
        <div className="settings-inner">
          <section className="settings-card span-2">
            <h2 className="card-title">设置</h2>
            <p className="notice notice-error" role="alert">
              {errorText}
            </p>
          </section>
        </div>
      </main>
    );
  }

  if (snapshot === null) {
    return (
      <main className="settings-page" aria-label="设置">
        <div className="settings-inner">
          <section className="settings-card span-2">
            <h2 className="card-title">设置</h2>
            <p className="hint">正在读取设置…</p>
          </section>
        </div>
      </main>
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
    setNotice("正在测试连接…");
    setErrorText(null);
    try {
      const report = await invokeCommand<ProbeReportDto>("settings_probe", {
        input: { key: keyDraft, modelIndex, endpointIndex, timeoutMs: null },
      });
      setNotice(report.succeeded ? report.message : `连接失败：${report.message}`);
      await load();
    } catch (error) {
      setNotice(null);
      setErrorText(ipcErrorText(error) ?? "测试失败，请稍后重试。");
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
    } catch (error) {
      setErrorText(ipcErrorText(error) ?? "设置未保存，请稍后重试。");
    } finally {
      setBusy(false);
    }
  }

  async function clear(): Promise<void> {
    if (busy) {
      return;
    }
    if (!window.confirm("确认清除已保存的访问密钥与设置？")) {
      return;
    }
    setBusy(true);
    try {
      await invokeCommand("settings_clear", { input: { confirm: true } });
      setNotice("设置已清除。");
      await load();
    } catch (error) {
      setErrorText(ipcErrorText(error) ?? "清除失败，请稍后重试。");
    } finally {
      setBusy(false);
    }
  }

  async function openLog(): Promise<void> {
    try {
      const path = await invokeCommand<string>("open_log");
      setNotice(`日志已定位：${path}`);
    } catch (error) {
      setErrorText(ipcErrorText(error) ?? "无法打开日志。");
    }
  }

  async function saveAssist(): Promise<void> {
    try {
      const config = await invokeCommand<AssistConfigDto>("assist_config_save", {
        input: { tipLines, contextLines },
      });
      setAssistConfig(config);
      setTipLines(config.tipLines);
      setContextLines(config.contextLines);
      setAssistNotice(
        `已保存：提醒看最近 ${String(config.tipLines)} 句，回应参考最近 ${String(config.contextLines)} 句。`,
      );
    } catch (error) {
      setAssistNotice(ipcErrorText(error) ?? "保存失败，请稍后重试。");
    }
  }

  const advancedVisible = advancedOpen || !snapshot.advancedCollapsed;
  const canSaveNow = snapshot.canSave || keyDraft.trim() !== "";
  const canTestNow = snapshot.canTest || keyDraft.trim() !== "";

  const searchStatus =
    wb.searchConfig === null
      ? "读取配置中…"
      : wb.searchConfig.ready
        ? `${SEARCH_PROVIDER_TEXT[wb.searchConfig.provider] ?? wb.searchConfig.provider} 已就绪 · 本小时 ${
            wb.searchConfig.hourlyLimit === 0
              ? `${String(wb.searchConfig.usedThisHour)} 次（不限）`
              : `${String(wb.searchConfig.usedThisHour)}/${String(wb.searchConfig.hourlyLimit)} 次`
          }`
        : wb.searchConfig.hasKey
          ? "已存 Key，当前构建未启用传输"
          : "未配置 · 填 Key 后可用";

  return (
    <main className="settings-page" aria-label="设置">
      <div className="settings-inner">
        <header className="page-head">
          <h1 className="page-title">设置</h1>
          <p className="page-sub">配置一次，开会时就不用再翻这里</p>
        </header>

        <section className="settings-card span-2">
          <div className="card-head">
            <h2 className="card-title">模型与凭据</h2>
            <span className="tag">{FORM_STATE_TEXT[snapshot.formState] ?? "未知状态"}</span>
          </div>

          <div className="settings-grid">
            <div className="settings-section">
              <h3>{snapshot.sectionsOrder[0]}</h3>
              <input
                className="key-input key-wide"
                type="password"
                autoComplete="off"
                aria-label="访问密钥"
                value={keyDraft}
                placeholder={snapshot.hasKey ? "已保存，重新输入可覆盖" : "粘贴访问密钥"}
                onChange={(event) => {
                  setKeyDraft(event.target.value);
                }}
              />
              <p className="hint">只用于本次保存，不回显、不落盘。</p>
              {inlineIssues === "" ? null : (
                <p className="notice notice-error" role="alert">
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
          </div>

          <div className="settings-section">
            <h3>{snapshot.sectionsOrder[3]}</h3>
            <div className="row-actions">
              <button
                className="btn btn-ghost"
                type="button"
                disabled={!canTestNow || busy}
                onClick={() => {
                  void probe();
                }}
              >
                测试连接
              </button>
              <button
                className="btn btn-primary"
                type="button"
                disabled={!canSaveNow || busy}
                onClick={() => {
                  void save();
                }}
              >
                保存设置
              </button>
              <button
                className="btn btn-ghost"
                type="button"
                disabled={busy}
                onClick={() => {
                  void clear();
                }}
              >
                清除设置
              </button>
              <button
                className="btn btn-ghost"
                type="button"
                onClick={() => {
                  void openLog();
                }}
              >
                打开日志
              </button>
            </div>
          </div>

          <button
            className="link-btn"
            type="button"
            aria-expanded={advancedVisible}
            onClick={() => {
              setAdvancedOpen((prev) => !prev);
            }}
          >
            {advancedVisible ? "收起高级信息" : "展开高级信息"}
          </button>

          {advancedVisible ? (
            <dl className="kv">
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
            <p className="notice" role="status">
              {notice}
            </p>
          )}
          {errorText === null ? null : (
            <p className="notice notice-error" role="alert">
              {errorText}
            </p>
          )}
        </section>

        <section className="settings-card">
          <div className="card-head">
            <h2 className="card-title">界面显示</h2>
          </div>
          <p className="hint">
            左侧状态灯与右侧卡片平时只占一列窄条，鼠标移上去才展开文字。想一直看到文字就选「常显」。
          </p>
          <div className="seg seg-block" role="group" aria-label="文字显示方式">
            <button
              className="seg-btn"
              type="button"
              aria-pressed={wb.labelMode === "hover"}
              onClick={() => {
                void wb.saveLabelMode("hover");
              }}
            >
              悬停显示
            </button>
            <button
              className="seg-btn"
              type="button"
              aria-pressed={wb.labelMode === "always"}
              onClick={() => {
                void wb.saveLabelMode("always");
              }}
            >
              常显
            </button>
          </div>
          {wb.labelNotice === null ? null : (
            <p className="notice notice-error" role="status">
              {wb.labelNotice}
            </p>
          )}

          <div className="settings-section">
            <label className="checkline">
              <input
                type="checkbox"
                checked={wb.autoCards}
                onChange={() => {
                  void wb.toggleAutoCards();
                }}
              />
              <span>转写时自动出卡（默认关；同一词 90 秒内不重复，每分钟最多 8 张）</span>
            </label>
            <label className="checkline">
              <input type="checkbox" checked={wb.searchAuto} onChange={wb.toggleSearchAuto} />
              <span>生成卡片时自动补来源</span>
            </label>
          </div>
        </section>

        <section className="settings-card">
          <div className="card-head">
            <h2 className="card-title">联网查资料</h2>
            <span className="tag">{searchStatus}</span>
          </div>
          <div className="settings-section">
            <div className="row-actions">
              <select
                className="key-input"
                aria-label="检索服务"
                value={wb.searchProvider}
                onChange={(event) => {
                  wb.setSearchProvider(event.target.value);
                }}
              >
                <option value="bocha">博查 Bocha（中文）</option>
                <option value="tavily">Tavily（英文 / 通用）</option>
              </select>
              <label className="field field-inline">
                <span>每小时上限</span>
                <input
                  className="key-input key-limit"
                  type="number"
                  min={0}
                  max={200}
                  step={5}
                  value={wb.searchLimit}
                  onChange={(event) => {
                    const next = Number.parseInt(event.target.value, 10);
                    wb.setSearchLimit(Number.isNaN(next) ? 0 : next);
                  }}
                />
              </label>
              <button
                className="btn btn-primary btn-sm"
                type="button"
                disabled={wb.searchBusy}
                onClick={() => {
                  void wb.saveSearchConfig();
                }}
              >
                {wb.searchBusy ? "保存中…" : "保存"}
              </button>
            </div>
          </div>

          <div className="settings-section">
            <label className="field">
              <span>博查 Key</span>
              <input
                className="key-input"
                type="password"
                autoComplete="off"
                placeholder={
                  wb.searchConfig?.hasKeyBocha === true
                    ? "已保存，留空不修改"
                    : "博查 Bocha Key（sk- 开头）"
                }
                value={wb.searchKeyBocha}
                onChange={(event) => {
                  wb.setSearchKeyBocha(event.target.value);
                }}
              />
            </label>
            <div className="row-actions">
              <button
                className="btn btn-ghost btn-xs"
                type="button"
                disabled={wb.searchBusy || wb.searchConfig?.hasKeyBocha !== true}
                onClick={() => {
                  void wb.clearSearchKeyFor("bocha");
                }}
              >
                清除博查 Key
              </button>
            </div>
          </div>

          <div className="settings-section">
            <label className="field">
              <span>Tavily Key</span>
              <input
                className="key-input"
                type="password"
                autoComplete="off"
                placeholder={
                  wb.searchConfig?.hasKeyTavily === true
                    ? "已保存，留空不修改"
                    : "Tavily Key（tvly- 开头）"
                }
                value={wb.searchKeyTavily}
                onChange={(event) => {
                  wb.setSearchKeyTavily(event.target.value);
                }}
              />
            </label>
            <div className="row-actions">
              <button
                className="btn btn-ghost btn-xs"
                type="button"
                disabled={wb.searchBusy || wb.searchConfig?.hasKeyTavily !== true}
                onClick={() => {
                  void wb.clearSearchKeyFor("tavily");
                }}
              >
                清除 Tavily Key
              </button>
            </div>
          </div>

          <p className="hint">
            只有点「联网查」或勾选自动补来源时才会出网：发出去的是选中的词（≤80 字），不发整段转写；
            Key 存系统凭据，不进配置文件。
          </p>
          {wb.searchConfigNotice === null ? null : (
            <p className="notice" role="status">
              {wb.searchConfigNotice}
            </p>
          )}
        </section>

        <section className="settings-card">
          <div className="card-head">
            <h2 className="card-title">辅助回复窗口</h2>
          </div>
          <div className="row-actions">
            <label className="field field-inline">
              <span>提醒看最近（句）</span>
              <input
                className="key-input key-limit"
                type="number"
                min={3}
                max={10}
                value={tipLines}
                onChange={(event) => {
                  setTipLines(Number.parseInt(event.target.value, 10) || 0);
                }}
              />
            </label>
            <label className="field field-inline">
              <span>上下文（句）</span>
              <input
                className="key-input key-limit"
                type="number"
                min={5}
                max={20}
                value={contextLines}
                onChange={(event) => {
                  setContextLines(Number.parseInt(event.target.value, 10) || 0);
                }}
              />
            </label>
            <button
              className="btn btn-primary btn-sm"
              type="button"
              onClick={() => {
                void saveAssist();
              }}
            >
              保存
            </button>
          </div>
          <p className="hint">
            当前：提醒看最近 {assistConfig?.tipLines ?? 5} 句、回应参考最近{" "}
            {assistConfig?.contextLines ?? 12} 句。提醒只依据最后几句，上下文供&quot;怎么回&quot;参考；
            越界会夹到 3~10 / 5~20 句。
          </p>
          {assistNotice === null ? null : (
            <p className="notice" role="status">
              {assistNotice}
            </p>
          )}
        </section>

        <section className="settings-card">
          <div className="card-head">
            <h2 className="card-title">隐私与数据</h2>
          </div>
          <dl className="kv">
            <div>
              <dt>自动外发</dt>
              <dd>未接入（仅手动触发）</dd>
            </div>
            <div>
              <dt>查询脱敏</dt>
              <dd>未接入（发原文）</dd>
            </div>
            <div>
              <dt>审计</dt>
              <dd>未接入</dd>
            </div>
            <div>
              <dt>凭据</dt>
              <dd>系统凭据库</dd>
            </div>
            <div>
              <dt>转写正文</dt>
              <dd>不落盘、不写日志</dd>
            </div>
          </dl>
          <p className="hint">
            选中文本只有在点「查意思 / 联网查」时才离开本机；卡片与来源按会话保存在本机库里。
          </p>
        </section>
      </div>
    </main>
  );
}
