import { useCallback, useEffect, useRef, useState } from "react";

import { invokeCommand } from "./bridge/tauri";
import type {
  CardDetailDto,
  ExplainResultDto,
  KeywordCardViewDto,
  TranscriptLineDto,
} from "./bridge/types";
import { CardDetail } from "./cards/CardDetail";
import { KeywordCard } from "./cards/KeywordCard";
import { SettingsPage } from "./settings/SettingsPage";

type ChannelState = "ready" | "degraded" | "off";

const DEFAULT_CHANNELS: readonly { name: string; state: ChannelState }[] = [
  { name: "字幕文本通道", state: "off" },
  { name: "系统音频通道", state: "off" },
  { name: "本地 ASR", state: "off" },
];

const STATE_TEXT: Record<ChannelState, string> = {
  ready: "就绪",
  degraded: "降级",
  off: "未启用",
};

const CHANNEL_DETAIL: Record<ChannelState, string> = {
  ready: "运行就绪",
  degraded: "运行中降级",
  off: "未启动或不可用",
};

function toChannelState(value: string): ChannelState {
  if (value === "ready" || value === "degraded") {
    return value;
  }
  return "off";
}

function asrHintText(running: boolean, text: string): string {
  if (text !== "") {
    return text;
  }
  return running
    ? "系统音频实时识别，等待说话…"
    : "点「开始实时转写」后按系统音频出卡；也可直接粘贴会议文本。";
}

function formatElapsed(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const pad = (value: number): string => String(value).padStart(2, "0");
  return hours > 0
    ? `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`
    : `${pad(minutes)}:${pad(seconds)}`;
}

export default function App() {
  const [active, setActive] = useState(true);
  const [showSettings, setShowSettings] = useState(false);
  const [cards, setCards] = useState<readonly KeywordCardViewDto[]>([]);
  const [cardError, setCardError] = useState<string | null>(null);
  const [detail, setDetail] = useState<CardDetailDto | null>(null);
  const [meeting, setMeeting] = useState<{ app: string; title: string } | null>(null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [channels, setChannels] = useState<readonly { name: string; state: ChannelState }[]>(
    DEFAULT_CHANNELS,
  );
  const meetingStartedAt = useRef<number | null>(null);
  const [ingestText, setIngestText] = useState("");
  const [ingestBusy, setIngestBusy] = useState(false);
  const [ingestNotice, setIngestNotice] = useState<string | null>(null);
  const [asrRunning, setAsrRunning] = useState(false);
  const [asrText, setAsrText] = useState("");
  const [asrError, setAsrError] = useState("");
  const [asrStats, setAsrStats] = useState("");
  const [lines, setLines] = useState<TranscriptLineDto[]>([]);
  const [selection, setSelection] = useState("");
  const [explainBusy, setExplainBusy] = useState(false);
  const [explainResult, setExplainResult] = useState<ExplainResultDto | null>(null);
  const [explainNotice, setExplainNotice] = useState<string | null>(null);

  const loadCards = useCallback(async () => {
    try {
      const next = await invokeCommand<KeywordCardViewDto[]>("keyword_cards_snapshot");
      setCards(next);
      setCardError(null);
    } catch {
      setCardError("卡片暂不可用，请稍后重试。");
    }
  }, []);

  useEffect(() => {
    void loadCards();
    const id = window.setInterval(() => {
      void loadCards();
    }, 1000);
    return () => {
      window.clearInterval(id);
    };
  }, [loadCards]);

  const loadTranscript = useCallback(async () => {
    try {
      const next = await invokeCommand<TranscriptLineDto[]>("transcript_snapshot");
      setLines(next);
    } catch {
      // 保留上一次内容
    }
  }, []);

  useEffect(() => {
    void loadTranscript();
    const id = window.setInterval(() => {
      void loadTranscript();
    }, 1200);
    return () => {
      window.clearInterval(id);
    };
  }, [loadTranscript]);

  useEffect(() => {
    let cancelled = false;
    async function refreshMeeting(): Promise<void> {
      try {
        const info = await invokeCommand<[string, string] | null>("meeting_status");
        if (!cancelled) {
          setMeeting(info === null ? null : { app: info[0], title: info[1] });
        }
      } catch {
        if (!cancelled) {
          setMeeting(null);
        }
      }
    }
    void refreshMeeting();
    const id = window.setInterval(() => {
      void refreshMeeting();
    }, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function refreshCapture(): Promise<void> {
      try {
        const rows = await invokeCommand<Array<[string, string]>>("capture_status");
        if (!cancelled) {
          setChannels(rows.map(([name, state]) => ({ name, state: toChannelState(state) })));
        }
      } catch {
        // 保留上一次状态
      }
    }
    void refreshCapture();
    const id = window.setInterval(() => {
      void refreshCapture();
    }, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  useEffect(() => {
    if (meeting === null) {
      meetingStartedAt.current = null;
      setElapsedSeconds(0);
      return;
    }
    if (meetingStartedAt.current === null) {
      meetingStartedAt.current = Date.now();
    }
    const id = window.setInterval(() => {
      if (meetingStartedAt.current !== null) {
        setElapsedSeconds(Math.floor((Date.now() - meetingStartedAt.current) / 1000));
      }
    }, 1000);
    return () => {
      window.clearInterval(id);
    };
  }, [meeting]);

  async function openCard(cardId: string): Promise<void> {
    try {
      const next = await invokeCommand<CardDetailDto>("keyword_card_open", {
        input: { cardId },
      });
      setDetail(next);
    } catch {
      setCardError("详情暂不可用。");
    }
  }

  // 只接受转写区内部的选区，避免把整页文字误当成选中。
  function captureSelection(container: HTMLElement): void {
    const active = window.getSelection();
    if (active === null || active.rangeCount === 0) {
      return;
    }
    if (!container.contains(active.getRangeAt(0).commonAncestorContainer)) {
      return;
    }
    const text = active.toString().trim().replace(/\s+/g, " ");
    if (text === "") {
      return;
    }
    setSelection(text.slice(0, 200));
    setExplainResult(null);
    setExplainNotice(null);
  }

  async function explainSelection(): Promise<void> {
    const text = selection.trim();
    if (text === "") {
      return;
    }
    setExplainBusy(true);
    setExplainNotice(null);
    try {
      const result = await invokeCommand<ExplainResultDto>("explain_selection", {
        input: { text, context: "" },
      });
      setExplainResult(result);
    } catch (error) {
      setExplainResult(null);
      const message =
        typeof error === "object" && error !== null && "message" in error
          ? String(error.message)
          : "";
      setExplainNotice(message === "" ? "查询失败，请稍后重试。" : message);
    } finally {
      setExplainBusy(false);
    }
  }

  async function cardFromExplanation(): Promise<void> {
    if (explainResult === null) {
      return;
    }
    try {
      const added = await invokeCommand<number>("ingest_explanation", {
        input: {
          word: explainResult.text,
          explanation: explainResult.explanation,
        },
      });
      setExplainNotice(added > 0 ? "已生成卡片。" : "这条内容未通过卡片生成条件。");
      await loadCards();
    } catch {
      setExplainNotice("生成卡片失败，请稍后重试。");
    }
  }

  useEffect(() => {
    let cancelled = false;
    async function refreshAsr(): Promise<void> {
      try {
        const [running, text, error, stats] = await invokeCommand<[boolean, string, string, string]>(
          "asr_status",
        );
        if (!cancelled) {
          setAsrRunning(running);
          setAsrText(text);
          setAsrError(error);
          setAsrStats(stats);
        }
      } catch {
        // 保留上一次状态
      }
    }
    void refreshAsr();
    const id = window.setInterval(() => {
      void refreshAsr();
    }, 700);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  async function toggleAsr(): Promise<void> {
    try {
      await invokeCommand(asrRunning ? "stop_asr" : "start_asr");
      if (!asrRunning) {
        setAsrError("");
      }
      setAsrRunning(!asrRunning);
    } catch (error) {
      // 后端已把可读原因放在 IpcErrorDto.message 里，直接展示，别再吞成通用文案。
      const detail =
        typeof error === "object" && error !== null && "message" in error
          ? String(error.message)
          : "";
      setAsrError(detail !== "" ? detail : "实时转写启动失败，请稍后重试。");
    }
  }

  async function ingest(): Promise<void> {
    if (ingestBusy || ingestText.trim() === "") {
      return;
    }
    setIngestBusy(true);
    try {
      const added = await invokeCommand<number>("ingest_text", { text: ingestText });
      setIngestText("");
      setIngestNotice(`已生成 ${String(added)} 张卡片。`);
      await loadCards();
    } catch {
      setIngestNotice(null);
      setCardError("生成卡片失败，请稍后重试。");
    } finally {
      setIngestBusy(false);
    }
  }

  return (
    <div className="shell">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true" />
          <div>
            <strong>会议副驾</strong>
            <span className="brand-sub">Meeting Copilot</span>
          </div>
        </div>
        <div className="topbar-meta">
          <span className="pill">
            {meeting === null
              ? "未检测到会议"
              : meeting.title === ""
                ? `检测到会议：${meeting.app}`
                : `检测到会议：${meeting.app} · ${meeting.title}`}
          </span>
          <span className="timer">
            {meeting === null ? "--:--" : formatElapsed(elapsedSeconds)}
          </span>
          <button
            className="btn"
            type="button"
            aria-pressed={showSettings}
            onClick={() => {
              setShowSettings((prev) => !prev);
            }}
          >
            {showSettings ? "返回卡片" : "设置"}
          </button>
          <button
            className={active ? "btn btn-stop" : "btn btn-start"}
            onClick={() => {
              setActive((prev) => !prev);
            }}
            type="button"
          >
            {active ? "暂停提示" : "恢复提示"}
          </button>
        </div>
      </header>

      <main className="layout">
        <aside className="panel session">
          <h2>采集状态</h2>
          <ul className="channel-list">
            {channels.map((channel) => (
              <li key={channel.name} className={`channel channel-${channel.state}`}>
                <span className="dot" aria-hidden="true" />
                <div>
                  <span className="channel-name">{channel.name}</span>
                  <span className="channel-detail">{CHANNEL_DETAIL[channel.state]}</span>
                </div>
                <span className="state-tag">{STATE_TEXT[channel.state]}</span>
              </li>
            ))}
          </ul>

          <h2>隐私</h2>
          <dl className="privacy">
            <div>
              <dt>自动外发</dt>
              <dd>未接入</dd>
            </div>
            <div>
              <dt>查询脱敏</dt>
              <dd>未接入</dd>
            </div>
            <div>
              <dt>审计</dt>
              <dd>未接入</dd>
            </div>
          </dl>
        </aside>

        {showSettings ? (
          <SettingsPage />
        ) : (
          <section className="panel cards">
            <div className="panel-head">
              <h2>关键词卡片</h2>
              <span className="hint">粘贴会议文本，自动提取关键词并生成卡片</span>
            </div>
            <div className="settings-section">
              <div className="settings-actions">
                <button
                  className="btn btn-start"
                  type="button"
                  onClick={() => {
                    void toggleAsr();
                  }}
                >
                  {asrRunning ? "停止实时转写" : "开始实时转写"}
                </button>
                {asrError === "" ? (
                  <span className="hint">
                    {asrHintText(asrRunning, asrText)}
                    {asrRunning && asrStats !== "" ? ` · ${asrStats}` : ""}
                  </span>
                ) : (
                  <span className="settings-error" role="alert">
                    {asrError}
                  </span>
                )}
              </div>
            </div>
            <div className="settings-section transcript-section">
              <div className="panel-head">
                <h2>完整转写</h2>
                <span className="hint">选中任意文字可查意思，并生成卡片</span>
              </div>
              <div
                className="transcript-body"
                role="textbox"
                aria-readonly="true"
                aria-label="实时转写全文"
                onMouseUp={(event) => {
                  captureSelection(event.currentTarget);
                }}
              >
                {lines.length === 0 ? (
                  <p className="hint">
                    还没有转写内容。开始实时转写后，识别到的句子会逐条出现在这里。
                  </p>
                ) : (
                  lines.map((line) => (
                    <p key={line.index} className="transcript-line">
                      {line.text}
                    </p>
                  ))
                )}
              </div>
              {selection === "" ? null : (
                <div className="settings-actions">
                  <span className="hint">已选中：{selection}</span>
                  <button
                    className="btn btn-start"
                    type="button"
                    disabled={explainBusy}
                    onClick={() => {
                      void explainSelection();
                    }}
                  >
                    {explainBusy ? "查询中…" : "查意思"}
                  </button>
                  <button
                    className="btn"
                    type="button"
                    onClick={() => {
                      setSelection("");
                      setExplainResult(null);
                      setExplainNotice(null);
                    }}
                  >
                    取消选中
                  </button>
                </div>
              )}
              {explainResult === null ? null : (
                <div className="explain-panel">
                  <p className="explain-source">
                    {explainResult.source === "llm" ? "模型释义" : "本机上下文"}
                  </p>
                  <p className="explain-text">{explainResult.explanation}</p>
                  <div className="settings-actions">
                    <button
                      className="btn btn-start"
                      type="button"
                      onClick={() => {
                        void cardFromExplanation();
                      }}
                    >
                      生成卡片
                    </button>
                  </div>
                </div>
              )}
              {explainNotice === null ? null : (
                <p className="settings-notice" role="status">
                  {explainNotice}
                </p>
              )}
            </div>
            <div className="settings-section">
              <label className="field">
                <span>会议文本</span>
                <textarea
                  rows={3}
                  value={ingestText}
                  placeholder="粘贴会议转写文本，例如：我们确认 SLA 99.9%，灰度范围限定华东两站点。"
                  onChange={(event) => {
                    setIngestText(event.target.value);
                  }}
                />
              </label>
              <div className="settings-actions">
                <button
                  className="btn btn-start"
                  type="button"
                  disabled={ingestBusy || ingestText.trim() === ""}
                  onClick={() => {
                    void ingest();
                  }}
                >
                  {ingestBusy ? "生成中…" : "生成卡片"}
                </button>
              </div>
              {ingestNotice === null ? null : (
                <p className="settings-notice" role="status">
                  {ingestNotice}
                </p>
              )}
            </div>
            {cardError === null ? null : (
              <p className="settings-error" role="alert">
                {cardError}
              </p>
            )}
            {cards.length === 0 ? (
              <p className="hint">暂无卡片。</p>
            ) : (
              <ul className="card-list">
                {cards.map((card) => (
                  <KeywordCard key={card.cardId} card={card} onOpen={(id) => void openCard(id)} />
                ))}
              </ul>
            )}
            {detail === null ? null : <CardDetail detail={detail} />}
          </section>
        )}

        <aside className="panel timeline">
          <h2>要点时间轴</h2>
          <ol className="timeline-list">
            <li className="hint">暂无要点。</li>
          </ol>
        </aside>
      </main>
    </div>
  );
}