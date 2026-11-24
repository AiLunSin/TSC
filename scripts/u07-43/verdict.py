#!/usr/bin/env python3
"""判定 U-07-43 的 200 条样本是否达标（`search_live.rs` 报告 → 结论）。

报告只含计数与时延；本脚本同样不打印查询原文、URL、标题、端点或凭据。
退出码：0=达标 / 1=样本不足 / 2=报告读取失败 / 3=时延或超时率超预算。
"""

from __future__ import annotations

import argparse
import io
import json

P95_BUDGET_MS = 8_000
P99_BUDGET_MS = 12_000
TIMEOUT_RATE_MAX = 0.02


def main() -> int:
    parser = argparse.ArgumentParser(description="U-07-43 样本判定")
    parser.add_argument("report", help="SEARCH_REPORT_PATH 指向的 JSON 报告")
    parser.add_argument("--target", type=int, default=200)
    args = parser.parse_args()

    try:
        with io.open(args.report, encoding="utf-8") as handle:
            report = json.load(handle)
    except (OSError, ValueError) as exc:
        print(f"报告读取失败：{exc}")
        return 2

    real = int(report.get("real_samples", 0))
    samples = int(report.get("samples", 0))
    timeouts = int(report.get("timeouts", 0))
    failures = int(report.get("failures", 0))
    p50 = int(report.get("p50_ms", 0))
    p95 = int(report.get("p95_ms", 0))
    p99 = report.get("p99_ms")
    p99 = int(p99) if p99 is not None else None
    rate = timeouts / samples if samples else 0.0

    print("")
    print("========== U-07-43 判定 ==========")
    print(f"真实成功样本 : {real} / {args.target}")
    print(
        f"请求总数     : {samples}（超时 {timeouts} / 失败 {failures} "
        f"/ 缓存命中 {report.get('cache_hits', 0)}）"
    )
    print(
        f"p50 / p95 / p99: {p50} ms / {p95} ms / "
        f"{p99 if p99 is not None else '未提供'} ms"
        f"（预算 p95 ≤ {P95_BUDGET_MS} ms、p99 ≤ {P99_BUDGET_MS} ms）"
    )
    print(f"超时率       : {rate:.2%}（预算 ≤ {TIMEOUT_RATE_MAX:.0%}）")
    print(f"报告状态     : {report.get('status')}")

    if real < args.target:
        print(f"结论：未达标——真实样本 {real} 条，缺 {args.target - real} 条（缓存/失败/超时不计）")
        rc = 1
    elif p95 > P95_BUDGET_MS or (p99 is not None and p99 > P99_BUDGET_MS) or rate > TIMEOUT_RATE_MAX:
        print("结论：样本数达标，但时延/超时率超出 8s p95、12s p99、≤2% 超时预算")
        rc = 3
    else:
        print("结论：达标（样本数、p95、超时率）")
        if p99 is None:
            print("注：报告不含 p99；SPEC 的 p99 ≤ 12s 需另按原始分位数核对。")
        rc = 0

    print("取证记录（可粘贴到 b4-status.md，勿写端点与 Key）：")
    print(
        f"  U-07-43 real_samples={real} p50_ms={p50} p95_ms={p95} "
        f"p99_ms={p99 if p99 is not None else 'n/a'} "
        f"timeouts={timeouts} failures={failures} status={report.get('status')}"
    )
    print(f"报告路径：{args.report}")
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
