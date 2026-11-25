import type { ConclusionDto, KeywordCardViewDto } from "../bridge/types";

const DISABLED_REASON_TEXT: Record<string, string> = {
  stale_reference: "引用已过期",
  authorization_revoked: "授权已撤回",
  not_allowed: "当前卡片不可点击",
  emergency_masked: "紧急遮蔽中",
  screen_share_protection: "投屏保护中",
  reference_unverified: "引用未经核实",
};

/** 关键词类型：契约里是英文枚举，界面上要给人看中文。 */
const KIND_TEXT: Record<string, string> = {
  proper_noun: "专有名词",
  acronym: "缩写",
  institution_role: "机构 / 角色",
  policy_standard: "政策 / 标准",
  metric: "指标",
};

function kindLabel(kind: string): string {
  return KIND_TEXT[kind] ?? kind;
}

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

/**
 * 卡片栏条目：平时只露关键词，鼠标移上来再展开正文与操作。
 *
 * 窄栏是为了给中间的会议内容让位；展开用行内高度，不用浮层，避免被栏的滚动区裁掉。
 */
export function KeywordCard({ card, onOpen }: KeywordCardProps) {
  if (!card.bodyVisible || card.body === null) {
    return (
      <li className="kcard kcard-muted">
        <span className="kcard-badge">有 {card.badgeCount} 条</span>
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
    <li className={enabled ? "kcard" : "kcard kcard-disabled"}>
      <span className="kcard-key" title={body.title}>
        {body.title}
      </span>
      <div className="kcard-more">
        <div className="kcard-meta">
          <span className={`kcard-kind kind-${card.keywordKind}`}>{kindLabel(card.keywordKind)}</span>
          <span className="kcard-time">{timeLabel(card.triggeredAtMs)}</span>
        </div>
        <p className="kcard-conclusion">{conclusionText(body.conclusion)}</p>
        <div className="kcard-foot">
          <span>引用 {body.referenceIds.length} 条</span>
          <span>置信度 {(body.confidence * 100).toFixed(0)}%</span>
        </div>
        {enabled ? null : <p className="kcard-reason">{reason}</p>}
        <button
          className="btn btn-ghost btn-xs"
          type="button"
          aria-disabled={!enabled}
          onClick={enabled ? () => { onOpen(card.cardId); } : undefined}
        >
          {enabled ? "查看详情" : "不可点击"}
        </button>
      </div>
    </li>
  );
}
