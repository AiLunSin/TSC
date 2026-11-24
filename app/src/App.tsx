import { useCallback, useEffect, useRef, useState } from "react";

import { invokeCommand } from "./bridge/tauri";
import type { CardDetailDto, KeywordCardViewDto } from "./bridge/types";
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

  useEffect(() => {
    let cancelled = false;
    async function refreshAsr(): Promise<void> {
      try {
        const [running, text, error] = await invokeCommand<[boolean, string, string]>("asr_status");
        if (!cancelled) {
          setAsrRunning(running);
          setAsrText(text);
          setAsrError(error);
        }
      } catch {
        // 保留上一次状态
      }
    }
    void refreshAsr();
    const id = window.setInterval(() => {
      void refreshAsr();
    }, 1500);
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
    } catch {
      setCardError("实时转写启动失败，请稍后重试。");
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
                  <span className="hint">{asrHintText(asrRunning, asrText)}</span>
                ) : (
                  <span className="settings-error" role="alert">
                    {asrError}
                  </span>
                )}
              </div>
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