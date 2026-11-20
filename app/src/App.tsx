import { useState } from "react";

type ChannelState = "ready" | "degraded" | "off";

type InsightKind = "fact" | "decision" | "todo" | "question";

interface InsightCard {
  readonly id: string;
  readonly kind: InsightKind;
  readonly title: string;
  readonly conclusion: string;
  readonly source: string;
  readonly trust: "high" | "medium" | "low";
  readonly at: string;
}

const KIND_LABEL: Record<InsightKind, string> = {
  fact: "关键信息",
  decision: "决策",
  todo: "待办",
  question: "疑问",
};

const SEED_CARDS: readonly InsightCard[] = [
  {
    id: "c-001",
    kind: "fact",
    title: "SLA 与赔付条款",
    conclusion: "合同约定月度可用性 99.9%，低于阈值按当月费用的 10% 赔付。",
    source: "合同正文 §7.2 · 本地文档",
    trust: "high",
    at: "10:14:02",
  },
  {
    id: "c-002",
    kind: "decision",
    title: "灰度范围收口",
    conclusion: "首批灰度限定华东两个站点，不做全国放量。",
    source: "发言人 · 张工",
    trust: "high",
    at: "10:16:41",
  },
  {
    id: "c-003",
    kind: "question",
    title: "P99 时延口径未决",
    conclusion: "当前未明确 P99 是否含网络往返，需会后确认。",
    source: "发言人 · 李经理",
    trust: "medium",
    at: "10:18:09",
  },
];

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

        <section className="panel cards">
          <div className="panel-head">
            <h2>提示卡流</h2>
            <span className="hint">门控预算：10 分钟 3 条</span>
          </div>
          <ul className="card-list">
            {SEED_CARDS.map((card) => (
              <li key={card.id} className="card">
                <div className="card-top">
                  <span className={`kind kind-${card.kind}`}>{KIND_LABEL[card.kind]}</span>
                  <span className="time">{card.at}</span>
                </div>
                <h3>{card.title}</h3>
                <p>{card.conclusion}</p>
                <footer>
                  <span className="source">{card.source}</span>
                  <span className={`trust trust-${card.trust}`}>
                    可信度 {card.trust === "high" ? "高" : card.trust === "medium" ? "中" : "低"}
                  </span>
                </footer>
              </li>
            ))}
          </ul>
        </section>

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