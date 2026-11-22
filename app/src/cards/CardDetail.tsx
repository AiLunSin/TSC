import type { CardDetailDto } from "../bridge/types";

import { conclusionText } from "./KeywordCard";

interface CardDetailProps {
  detail: CardDetailDto;
}

export function CardDetail({ detail }: CardDetailProps) {
  return (
    <section className="card-detail" role="region" aria-label="卡片详情">
      <h3>{detail.title}</h3>
      <p>{conclusionText(detail.conclusion)}</p>
      <p className="hint">引用一致性：{detail.citationAlignment}</p>
      <ul className="card-detail-refs">
        {detail.referenceIds.map((referenceId) => (
          <li key={referenceId}>{referenceId}</li>
        ))}
      </ul>
    </section>
  );
}