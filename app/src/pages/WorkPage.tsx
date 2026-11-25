import type { JSX } from "react";

import { CardDetail } from "../cards/CardDetail";
import { KeywordCard } from "../cards/KeywordCard";
import type { ChannelState, Workbench } from "../state/useWorkbench";
import { CHANNEL_DETAIL, STATE_TEXT, asrHintText } from "../state/useWorkbench";

interface WorkPageProps {
  wb: Workbench;
}

interface StatusLight {
  key: string;
  name: string;
  state: ChannelState;
  /** 灯色已经说明状态，副行只写"具体在做什么"，不再重复"就绪 / 未启用"。 */
  detail: string;
}

function StatusRail({ wb }: WorkPageProps): JSX.Element {
  const meetingLight: StatusLight =
    wb.meeting === null
      ? { key: "meeting", name: "会议检测", state: "off", detail: "未检测到会议" }
      : {
          key: "meeting",
          name: "会议检测",
          state: "ready",
          detail: wb.meeting.title === "" ? wb.meeting.app : `${wb.meeting.app} · ${wb.meeting.title}`,
        };

  const asrLight: StatusLight = {
    key: "asr",
    name: "实时转写",
    state: wb.asrRunning ? "ready" : "off",
    detail: wb.asrRunning ? "转写中" : "未开始",
  };

  const channels: StatusLight[] = wb.channels.map((channel) => ({
    key: channel.name,
    name: channel.name,
    state: channel.state,
    detail: CHANNEL_DETAIL[channel.state],
  }));

  const privacy: StatusLight[] = [
    { key: "egress", name: "自动外发", state: "off", detail: "未接入 · 仅手动触发" },
    { key: "mask", name: "查询脱敏", state: "off", detail: "未接入 · 发原文" },
    { key: "audit", name: "审计", state: "off", detail: "未接入" },
  ];

  function renderLight(light: StatusLight): JSX.Element {
    return (
      <li
        key={light.key}
        className={`light light-${light.state}`}
        tabIndex={0}
        aria-label={`${light.name}：${STATE_TEXT[light.state]}（${light.detail}）`}
      >
        <span className="light-dot" aria-hidden="true" />
        <span className="light-copy">
          <span className="light-name">{light.name}</span>
          <span className="light-detail">{light.detail}</span>
        </span>
      </li>
    );
  }

  return (
    <aside className="rail rail-left" aria-label="采集与隐私状态">
      <p className="rail-caption">采集</p>
      <ul className="light-list">
        {renderLight(meetingLight)}
        {renderLight(asrLight)}
        {channels.map((channel) => renderLight(channel))}
      </ul>
      <span className="rail-divider" aria-hidden="true" />
      <p className="rail-caption">隐私</p>
      <ul className="light-list">{privacy.map((item) => renderLight(item))}</ul>
    </aside>
  );
}

function CardsRail({ wb }: WorkPageProps): JSX.Element {
  return (
    <aside className="rail rail-cards" aria-label="关键词卡片">
      <div className="cards-head">
        <span className="cards-title">卡片</span>
        <span className="cards-count">{wb.cards.length}</span>
        <button
          className="btn btn-ghost btn-xs"
          type="button"
          disabled={wb.cards.length === 0}
          onClick={() => {
            void wb.clearCards();
          }}
        >
          清空
        </button>
        <button
          className="btn btn-ghost btn-xs"
          type="button"
          onClick={() => {
            void wb.loadQuality();
          }}
        >
          质量
        </button>
      </div>
      <p className="cards-empty">{wb.qualityLabel}</p>
      <div className="cards-body">
        {wb.cards.length === 0 ? (
          <p className="cards-empty">
            暂无卡片。在转写区选中文字 → 查意思 → 生成卡片；也可以粘贴会议文本出卡。
          </p>
        ) : (
          <ul className="card-list">
            {wb.cards.map((card) => (
              <KeywordCard
                key={card.cardId}
                card={card}
                onOpen={(id) => {
                  void wb.openCard(id);
                }}
                onFeedback={(id, outcome) => {
                  void wb.sendFeedback(id, outcome);
                }}
              />
            ))}
          </ul>
        )}
      </div>
    </aside>
  );
}

export function WorkPage({ wb }: WorkPageProps): JSX.Element {
  return (
    <div className="work">
      <StatusRail wb={wb} />

      <main className="stage" aria-label="工作区">
        <section className="panel transcript-panel">
          <div className="panel-head">
            <div className="panel-heading">
              <h2 className="panel-title">实时转写</h2>
              <span className="panel-sub">
                {asrHintText(wb.asrRunning, wb.asrText)}
                {wb.asrRunning && wb.asrStats !== "" ? ` · ${wb.asrStats}` : ""}
              </span>
            </div>
            <button
              className={wb.asrRunning ? "btn btn-stop" : "btn btn-primary"}
              type="button"
              onClick={() => {
                void wb.toggleAsr();
              }}
            >
              {wb.asrRunning ? "停止转写" : "开始实时转写"}
            </button>
          </div>

          {wb.asrError === "" ? null : (
            <p className="notice notice-error" role="alert">
              {wb.asrError}
            </p>
          )}

          <div
            className="transcript-body"
            role="textbox"
            aria-readonly="true"
            aria-label="实时转写全文"
            onMouseUp={(event) => {
              wb.captureSelection(event.currentTarget);
            }}
          >
            {wb.lines.length === 0 ? (
              <p className="empty-hint">
                还没有转写内容。开始实时转写后，识别到的句子会逐条出现在这里。
              </p>
            ) : (
              wb.lines.map((line) => (
                <p key={line.index} className="transcript-line">
                  {line.text}
                </p>
              ))
            )}
          </div>

          <div className="transcript-foot">
            {wb.selection === "" ? (
              <span className="foot-hint">选中一段文字，可以查意思、联网查，或直接让模型给提醒。</span>
            ) : (
              <div className="selection-bar">
                <span className="selection-text" title={wb.selection}>
                  已选中：{wb.selection}
                </span>
                <button
                  className="btn btn-primary btn-sm"
                  type="button"
                  disabled={wb.explainBusy}
                  onClick={() => {
                    void wb.explainSelection();
                  }}
                >
                  {wb.explainBusy ? "查询中…" : "查意思"}
                </button>
                <button
                  className="btn btn-ghost btn-sm"
                  type="button"
                  disabled={wb.searchBusy}
                  onClick={() => {
                    void wb.searchSelection();
                  }}
                >
                  {wb.searchBusy ? "检索中…" : "联网查"}
                </button>
                <button
                  className="btn btn-ghost btn-sm"
                  type="button"
                  onClick={() => {
                    wb.clearSelection();
                  }}
                >
                  取消选中
                </button>
              </div>
            )}
            <button
              className="btn btn-quiet btn-sm"
              type="button"
              disabled={wb.assistBusy}
              onClick={() => {
                void wb.askAssist();
              }}
            >
              {wb.assistBusy ? "生成中…" : "AI 提醒 / 怎么回"}
            </button>
          </div>
        </section>

        {wb.detail === null ? null : (
          <section className="panel panel-detail">
            <div className="panel-head">
              <h2 className="panel-title">卡片详情</h2>
              <button
                className="btn btn-ghost btn-xs"
                type="button"
                onClick={() => {
                  wb.setDetail(null);
                }}
              >
                收起
              </button>
            </div>
            <div className="panel-body">
              <CardDetail detail={wb.detail} />
            </div>
          </section>
        )}

        {wb.assist === null ? null : (
          <section className="panel panel-result">
            <div className="panel-head">
              <h2 className="panel-title">AI 提醒 / 怎么回</h2>
              <span className="panel-sub">
                提醒看最近 5 句，回应参考最近 12 句
                {wb.selection === "" ? "" : " + 选中句"}
              </span>
            </div>
            <div className="panel-body">
              {wb.assist.points.length === 0 ? null : (
                <>
                  <p className="assist-label">注意这些</p>
                  <ul className="assist-list">
                    {wb.assist.points.map((point) => (
                      <li key={point}>{point}</li>
                    ))}
                  </ul>
                </>
              )}
              {wb.assist.replies.length === 0 ? null : (
                <>
                  <p className="assist-label">可以直接说</p>
                  <ul className="assist-list">
                    {wb.assist.replies.map((reply) => (
                      <li key={reply} className="assist-reply">
                        <span>{reply}</span>
                        <button
                          className="btn btn-ghost btn-xs"
                          type="button"
                          onClick={() => {
                            void wb.copyReply(reply);
                          }}
                        >
                          复制
                        </button>
                      </li>
                    ))}
                  </ul>
                </>
              )}
              {wb.assist.note === "" ? null : <p className="result-text">{wb.assist.note}</p>}
            </div>
          </section>
        )}

        {wb.explainResult === null ? null : (
          <section className="panel panel-result">
            <div className="panel-head">
              <h2 className="panel-title">
                {wb.explainResult.text === "" ? "释义" : `释义 · ${wb.explainResult.text}`}
              </h2>
              <span className="panel-sub">
                {wb.explainResult.source === "llm" ? "模型释义" : "本机上下文"}
                {wb.searchHits.length > 0 ? ` · 附 ${String(wb.searchHits.length)} 条网络来源` : ""}
              </span>
            </div>
            <div className="panel-body">
              <p className="result-text">{wb.explainResult.explanation}</p>
              <div className="panel-actions">
                <button
                  className="btn btn-primary btn-sm"
                  type="button"
                  disabled={wb.cardBusy}
                  onClick={() => {
                    void wb.cardFromExplanation();
                  }}
                >
                  {wb.cardBusy ? "生成中…" : "生成卡片"}
                </button>
              </div>
            </div>
          </section>
        )}

        {wb.searchHits.length === 0 ? null : (
          <section className="panel panel-result">
            <div className="panel-head">
              <h2 className="panel-title">联网结果</h2>
              <span className="panel-sub">
                {wb.searchHitProvider === "" ? "搜索结果" : `${wb.searchHitProvider} 结果`}
                · 生成卡片时这些站点会记为来源
              </span>
            </div>
            <div className="panel-body">
              <ul className="hit-list">
                {wb.searchHits.map((hit) => (
                  <li key={`${hit.url}|${hit.title}`}>
                    <span className="hit-title">{hit.title}</span>
                    <span className="hit-site">{hit.site}</span>
                    <p className="hit-snippet">{hit.snippet}</p>
                    {hit.url === "" ? null : <p className="hit-url">{hit.url}</p>}
                  </li>
                ))}
              </ul>
            </div>
          </section>
        )}

        {wb.searchNotice === null ? null : (
          <p className="notice" role="status">
            {wb.searchNotice}
          </p>
        )}
        {wb.assistNotice === null ? null : (
          <p className="notice" role="status">
            {wb.assistNotice}
          </p>
        )}
        {wb.explainNotice === null ? null : (
          <p className="notice" role="status">
            {wb.explainNotice}
          </p>
        )}
        {wb.cardError === null ? null : (
          <p className="notice notice-error" role="alert">
            {wb.cardError}
          </p>
        )}

        <details className="panel panel-collapse">
          <summary className="collapse-summary">手动粘贴会议文本生成卡片</summary>
          <div className="panel-body">
            <label className="field">
              <span>会议文本</span>
              <textarea
                rows={3}
                value={wb.ingestText}
                placeholder="粘贴会议转写文本，例如：我们确认 SLA 99.9%，灰度范围限定华东两站点。"
                onChange={(event) => {
                  wb.setIngestText(event.target.value);
                }}
              />
            </label>
            <div className="panel-actions">
              <button
                className="btn btn-primary btn-sm"
                type="button"
                disabled={wb.ingestBusy || wb.ingestText.trim() === ""}
                onClick={() => {
                  void wb.ingest();
                }}
              >
                {wb.ingestBusy ? "生成中…" : "生成卡片"}
              </button>
            </div>
            {wb.ingestNotice === null ? null : (
              <p className="notice" role="status">
                {wb.ingestNotice}
              </p>
            )}
          </div>
        </details>
      </main>

      <CardsRail wb={wb} />
    </div>
  );
}
