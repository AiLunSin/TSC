import { useCallback, useEffect, useState } from "react";

import { invokeCommand } from "./bridge/tauri";
import type { CardDetailDto, KeywordCardViewDto } from "./bridge/types";
import { CardDetail } from "./cards/CardDetail";
import { KeywordCard } from "./cards/KeywordCard";
import { SettingsPage } from "./settings/SettingsPage";

type ChannelState = "ready" | "degraded" | "off";

const CHANNELS: readonly { name: string; state: ChannelState; detail: string }[] = [
  { name: "字幕文本通道", state: "ready", detail: "腾讯会议 · 已连接" },
  { name: "系统音频通道", state: "ready", detail: "WASAPI 进程级环回" },
  { name: "本地 ASR", state: "degraded", detail: "模型热加载中" },
];

const STATE_TEXT: Record<ChannelState, string> = {
  ready: "就绪",
  degraded: "降级",
  off: "未启用",
};

export default function App() {
  const [active, setActive] = useState(true);
  const [showSettings, setShowSettings] = useState(false);
  const [cards, setCards] = useState<readonly KeywordCardViewDto[]>([]);
  const [cardError, setCardError] = useState<string | null>(null);
  const [detail, setDetail] = useState<CardDetailDto | null>(null);

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
  }, [loadCards]);

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
          <span className="pill">腾讯会议 · 产品评审会</span>
          <span className="timer">00:18:42</span>
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
            {CHANNELS.map((channel) => (
              <li key={channel.name} className={`channel channel-${channel.state}`}>
                <span className="dot" aria-hidden="true" />
                <div>
                  <span className="channel-name">{channel.name}</span>
                  <span className="channel-detail">{channel.detail}</span>
                </div>
                <span className="state-tag">{STATE_TEXT[channel.state]}</span>
              </li>
            ))}
          </ul>

          <h2>隐私</h2>
          <dl className="privacy">
            <div>
              <dt>自动外发</dt>
              <dd className="ok">已一次性授权</dd>
            </div>
            <div>
              <dt>查询脱敏</dt>
              <dd className="ok">已启用</dd>
            </div>
            <div>
              <dt>审计</dt>
              <dd>哈希链正常</dd>
            </div>
          </dl>
        </aside>

        {showSettings ? (
          <SettingsPage />
        ) : (
          <section className="panel cards">
            <div className="panel-head">
              <h2>关键词卡片</h2>
              <span className="hint">按会议内容实时生成</span>
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
            <li>
              <time>10:16</time>
              <span>灰度范围限定华东两站点</span>
            </li>
            <li>
              <time>10:14</time>
              <span>确认 SLA 99.9% 与赔付比例</span>
            </li>
            <li>
              <time>10:09</time>
              <span>待办：补充压测报告</span>
            </li>
            <li>
              <time>10:03</time>
              <span>会议开始，自动采集已启动</span>
            </li>
          </ol>
        </aside>
      </main>
    </div>
  );
}