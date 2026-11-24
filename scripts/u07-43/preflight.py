#!/usr/bin/env python3
"""U-07-43 采样前的离线预检：只校验配置与查询集，不出网、不打印凭据。

对齐 Rust 侧的真实约束（避免预检通过但 cargo test 里静默零源）：
- profile_store.rs::parse_document：字段、枚举、取值范围、id/priority 唯一
- provider.rs::is_usable：enabled == enabled 且 health != unavailable（degraded 也算可用）
- outbound_guard.rs：HTTPS、无 userinfo、拒绝环回/私网/链路本地
- http_json.rs：authRef 指向的环境变量名，值为 Bearer 凭据
- SanitizedQuery：查询非空且不超过 200 字

输出只包含主机列表与计数，不含完整 URL、凭据、查询原文。
"""

from __future__ import annotations

import argparse
import io
import ipaddress
import json
import math
import os
import re
import sys
from urllib.parse import urlsplit

KINDS = {"searxng_http", "tavily", "brave", "bing", "custom"}
ENABLED = {"enabled", "disabled"}
HEALTH = {"healthy", "degraded", "unavailable"}
ENV_NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
ALLOWED_TOP = {"schemaVersion", "providers"}
ALLOWED_PROVIDER = {
    "id", "name", "kind", "priority", "weight", "enabled", "timeout",
    "rateLimit", "blockedDomains", "health", "adapter", "endpoint", "authRef",
}
REQUIRED_PROVIDER = ALLOWED_PROVIDER - {"authRef"}


def fail(message: str) -> None:
    print(f"预检失败：{message}", file=sys.stderr)
    raise SystemExit(2)


def is_public_host(host: str) -> bool:
    lowered = host.rstrip(".").lower()
    if lowered == "localhost" or lowered.endswith((".localhost", ".local", ".internal")):
        return False
    try:
        address = ipaddress.ip_address(lowered)
    except ValueError:
        return True  # 域名：真实地址由运行时解析，这里不额外做 DNS
    return bool(address.is_global)


def check_number(value: object, field: str, *, integral: bool = True) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        fail(f"{field} 必须是数字")
    if not math.isfinite(float(value)):
        fail(f"{field} 必须是有限数")
    if integral and not isinstance(value, int):
        fail(f"{field} 必须是整数")
    return float(value)


def check_provider(raw: object, index: int) -> dict:
    label = f"providers[{index}]"
    if not isinstance(raw, dict):
        fail(f"{label} 必须是对象")
    unknown = set(raw) - ALLOWED_PROVIDER
    if unknown:
        fail(f"{label} 含未知字段 {sorted(unknown)}（Rust 侧 deny_unknown_fields）")
    missing = REQUIRED_PROVIDER - set(raw)
    if missing:
        fail(f"{label} 缺字段 {sorted(missing)}")

    name = raw["name"]
    if not isinstance(name, str) or not (1 <= len(name.strip()) <= 50):
        fail(f"{label}.name 必须是 1~50 字")
    pid = raw["id"]
    if not isinstance(pid, str) or not pid.strip():
        fail(f"{label}.id 不能为空")
    if raw["kind"] not in KINDS:
        fail(f"{label}.kind 取值非法：{raw['kind']}")
    if raw.get("enabled", "enabled") not in ENABLED:
        fail(f"{label}.enabled 取值非法：{raw.get('enabled')}")
    if raw.get("health", "healthy") not in HEALTH:
        fail(f"{label}.health 取值非法：{raw.get('health')}")
    if raw["adapter"] != "http_json":
        fail(f"{label}.adapter 只能是 http_json")

    priority = check_number(raw["priority"], f"{label}.priority")
    if priority < 0:
        fail(f"{label}.priority 不得为负")
    weight = check_number(raw["weight"], f"{label}.weight", integral=False)
    if not (0 < weight <= 100):
        fail(f"{label}.weight 必须落在 (0, 100]")
    timeout = check_number(raw["timeout"], f"{label}.timeout")
    if timeout <= 0:
        fail(f"{label}.timeout 必须为正整数（毫秒）")

    rate = raw["rateLimit"]
    if not isinstance(rate, dict) or set(rate) != {"perMinute", "perDay"}:
        fail(f"{label}.rateLimit 需要且只需要 perMinute / perDay")
    per_minute = check_number(rate["perMinute"], f"{label}.rateLimit.perMinute")
    per_day = check_number(rate["perDay"], f"{label}.rateLimit.perDay")
    if not (0 <= per_minute <= 65535) or per_day < 0:
        fail(f"{label}.rateLimit 超出 u16/u32 范围")

    blocked = raw["blockedDomains"]
    if not isinstance(blocked, list) or any(not isinstance(item, str) for item in blocked):
        fail(f"{label}.blockedDomains 必须是字符串数组")

    endpoint = raw["endpoint"]
    if not isinstance(endpoint, str):
        fail(f"{label}.endpoint 必须是字符串")
    parts = urlsplit(endpoint)
    if parts.scheme != "https" or not parts.hostname or parts.username or parts.password:
        fail(f"{label}.endpoint 必须是无凭据的 HTTPS 绝对地址")
    if not is_public_host(parts.hostname):
        fail(f"{label}.endpoint 指向环回/私网/链路本地地址，外发白名单会拒绝")

    auth_ref = raw.get("authRef")
    if auth_ref is not None:
        if not isinstance(auth_ref, str) or not ENV_NAME.match(auth_ref):
            fail(f"{label}.authRef 必须是环境变量名（不是凭据值）")
    return raw


def main() -> None:
    parser = argparse.ArgumentParser(description="U-07-43 离线预检")
    parser.add_argument("providers", help="providers.json 路径")
    parser.add_argument("queries", help="查询文件（每行一条）")
    parser.add_argument("--target", type=int, default=200)
    parser.add_argument("--mode", choices=("generated", "custom"), default="generated")
    args = parser.parse_args()

    try:
        doc = json.load(io.open(args.providers, encoding="utf-8"))
    except (OSError, ValueError) as exc:
        fail(f"providers.json 解析失败：{exc}")
    if not isinstance(doc, dict):
        fail("providers.json 顶层必须是对象")
    unknown = set(doc) - ALLOWED_TOP
    if unknown:
        fail(f"providers.json 顶层含未知字段 {sorted(unknown)}")
    if doc.get("schemaVersion", "1.0") != "1.0":
        fail("schemaVersion 必须为 1.0")
    providers = doc.get("providers", [])
    if not isinstance(providers, list) or not (1 <= len(providers) <= 64):
        fail("providers 必须是 1~64 条的数组")

    checked = [check_provider(raw, index) for index, raw in enumerate(providers)]
    ids = [item["id"] for item in checked]
    priorities = [item["priority"] for item in checked]
    if len(set(ids)) != len(ids):
        fail("providers[].id 不得重复")
    if len(set(priorities)) != len(priorities):
        fail("providers[].priority 不得重复")

    # 与 provider.rs::is_usable 对齐：degraded 也算可用，只有 unavailable 不可用。
    usable = [
        item for item in checked
        if item.get("enabled", "enabled") == "enabled"
        and item.get("health", "healthy") != "unavailable"
    ]
    if not usable:
        fail("没有启用且非 unavailable 的检索源")

    hosts = []
    for item in usable:
        endpoint = item["endpoint"]
        hosts.append(urlsplit(endpoint).hostname or "")
        auth_ref = item.get("authRef")
        if auth_ref and not os.environ.get(auth_ref):
            fail(f"authRef 指向的环境变量 {auth_ref} 未设置或为空")

    try:
        lines = [line.strip() for line in io.open(args.queries, encoding="utf-8") if line.strip()]
    except OSError as exc:
        fail(f"查询文件读取失败：{exc}")
    unique = set(lines)
    if len(lines) < args.target or len(unique) < args.target:
        fail(f"查询不足：{len(lines)} 行 / {len(unique)} 条不重复，门槛 {args.target} 条")
    if any(len(query) > 200 for query in lines):
        fail("存在超过 200 字的查询（SanitizedQuery 上限）")

    mode_text = "自定义 providers.json（脚本不会修改）" if args.mode == "custom" else "脚本生成的 providers.json"
    print(f"预检通过：{len(usable)} 个可用源，主机 {', '.join(sorted(set(hosts)))}")
    print(f"配置模式：{mode_text}；{len(lines)} 条查询 / {len(unique)} 条不重复")
    print(f"前 {args.target} 条将作为样本；缓存命中、失败、超时不计入真实样本")


if __name__ == "__main__":
    main()
