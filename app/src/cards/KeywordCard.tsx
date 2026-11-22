import type { ConclusionDto, KeywordCardViewDto } from "../bridge/types";

const DISABLED_REASON_TEXT: Record<string, string> = {
  stale_reference: "引用已过期",
  authorization_revoked: "授权已撤回",
  not_allowed: "当前卡片不可点击",
  emergency_masked: "紧急遮蔽中",
  screen_share_protection: "投屏保护中",
  reference_unverified: "引用未经核实",
};

export function conclusionText(conclusion: ConclusionDto): string {
  if (conclusion === "noReliableSource") {
    return "未找到可靠来源";
  }
  if (conclusion === "absent") {
    return "";
  }
  return conclusion.available.text;
}

interface KeywordCardProps {
  card: KeywordCardViewDto;
  onOpen: (cardId: string) => void;
}

function timeLabel(triggeredAtMs: number): string {
  const date = new Date(triggeredAtMs);
  return `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
}

export function KeywordCard({ card, onOpen }: KeywordCardProps) {
  if (!card.bodyVisible || card.body === null) {
    return (
      <li className="card card-hidden">
        <span className="badge">有 {card.badgeCount} 条</span>
      </li>
    );
  }

  const body = card.body;
  const enabled = card.clickState === "enabled";
  const reason =
    card.disabledReason === null
      ? ""
      : (DISABLED_REASON_TEXT[card.disabledReason] ?? "暂不可点击");

  return (
    <li className={enabled ? "card" : "card card-disabled"}>
      <div className="card-top">
        <span className="kind">{card.keywordKind}</span>
        <span className="time">{timeLabel(card.triggeredAtMs)}</span>
      </div>
      <h3>{body.title}</h3>
      <p>{conclusionText(body.conclusion)}</p>
      <footer>
        <span className="source">引用 {body.referenceIds.length} 条</span>
        <span className="trust">置信度 {(body.confidence * 100).toFixed(0)}%</span>
      </footer>
      {enabled ? null : <p className="card-reason">{reason}</p>}
      <button
        className="card-open"
        type="button"
        aria-disabled={!enabled}
        onClick={enabled ? () => { onOpen(card.cardId); } : undefined}
      >
        {enabled ? "查看详情" : "不可点击"}
      </button>
    </li>
  );
}