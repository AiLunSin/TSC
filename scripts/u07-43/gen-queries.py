#!/usr/bin/env python3
"""生成 U-07-43 的 200+ 条互不重复的中文检索查询（每行一条，UTF-8）。

用途：`search_live.rs::collect_200_real_samples` 要求 ≥200 条查询，且每条都要
真实出网、未命中本地缓存才算样本。重复查询会命中 `CacheService`，不计入 200 条，
所以这里按「术语 × 提问角度」构造 240 条互不重复的查询。

只用固定词表，不读任何转写、会议记录或个人数据；生成结果可安全提交到仓库
（`.tmp/u07-43/` 下的产物默认被忽略，不入库）。
用法：
    python3 scripts/u07-43/gen-queries.py --out .tmp/u07-43/queries.txt
    python3 scripts/u07-43/gen-queries.py --count 260 --seed 7 --out -
"""

from __future__ import annotations

import argparse
import io
import sys

# 50 个会议/工程场景里真实会被选中查证的术语；不含人名、公司名、具体数值。
TERMS: tuple[str, ...] = (
    "灰度发布", "金丝雀部署", "流量回放", "熔断降级", "限流算法",
    "幂等设计", "分布式事务", "最终一致性", "消息积压", "分库分表",
    "慢查询优化", "缓存雪崩", "缓存穿透", "冷启动预热", "容量评估",
    "SLA 定义", "SLO 与错误预算", "故障演练", "变更窗口", "回滚预案",
    "告警收敛", "日志规范", "链路追踪", "数据血缘", "指标口径",
    "埋点方案", "AB 实验", "用户分层", "增长漏斗", "留存分析",
    "客户成功", "需求评审", "排期估算", "跨团队依赖", "技术债治理",
    "代码评审", "单元测试覆盖率", "接口契约", "版本兼容", "灰度回滚",
    "数据合规", "个人信息保护", "等级保护要求", "审计留痕", "权限最小化",
    "成本优化", "云资源预算", "服务器选型", "灾备双活", "应急预案",
)

# 6 个提问角度；前 4 个角度各轮转全部 50 个术语，
# 恰好构成前 200 条（覆盖全部术语），后 2 个角度仅用于补充余量。
INTENTS: tuple[str, ...] = (
    "是什么", "最佳实践", "常见风险", "落地案例", "指标怎么定", "合规要求",
)


def build(count: int) -> list[str]:
    """生成 count 条互不重复查询；前 200 条固定覆盖 50 术语 × 4 角度。"""
    queries: list[str] = []
    for intent in INTENTS[:4]:
        for term in TERMS:
            queries.append(f"{term} {intent}")
            if len(queries) >= count:
                return queries
    # 余量：用后两个角度覆盖前 20 个术语，最多再加 40 条。
    for intent in INTENTS[4:]:
        for term in TERMS[:20]:
            queries.append(f"{term} {intent}")
            if len(queries) >= count:
                return queries
    # 词表用尽仍不足时，追加角度 + 序号后缀，保证条数但不制造重复。
    index = 1
    while len(queries) < count:
        for term in TERMS:
            for intent in INTENTS[:2]:
                queries.append(f"{term} {intent} 补充{index}")
                if len(queries) >= count:
                    return queries
        index += 1
    return queries


def main() -> int:
    parser = argparse.ArgumentParser(description="生成 U-07-43 采样查询集")
    parser.add_argument("--count", type=int, default=240, help="默认 240 条，留 40 条余量")
    parser.add_argument("--out", default="-", help="输出路径，- 表示标准输出")
    parser.add_argument(
        "--seed", type=int, default=0, help="保留参数：本生成器为确定性输出，不使用随机数"
    )
    args = parser.parse_args()

    if args.count < 200:
        print("U-07-43 至少需要 200 条查询", file=sys.stderr)
        return 2

    queries = build(args.count)
    assert len(queries) == args.count
    assert len(set(queries)) == len(queries), "出现重复查询，会导致缓存命中而不计样本"
    assert all(0 < len(q) <= 200 for q in queries), "存在空串或超过 200 字的查询"

    payload = "\n".join(queries) + "\n"
    if args.out == "-":
        sys.stdout.write(payload)
    else:
        with io.open(args.out, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(payload)
        print(
            f"已生成 {len(queries)} 条查询（互不重复） -> {args.out}",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
