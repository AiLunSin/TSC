#!/usr/bin/env bash
# 分层依赖断言：domain 不得依赖 infrastructure / platform-* / 网络与存储实现。
# 追溯 DESIGN 2.1.2、2.4.1（依赖方向自上而下单向）。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "SKIP: 未检测到 cargo，分层校验留待 CI（ubuntu-latest）执行" >&2
  exit 0
fi

FORBIDDEN='^(meeting-copilot-infrastructure|meeting-copilot-platform-win|meeting-copilot-platform-mac|reqwest|hyper|sqlx|rusqlite|windows|objc2|tauri)( |$)'

echo "检查 domain 依赖闭包..."
if cargo tree -p meeting-copilot-domain --edges normal --prefix none 2>/dev/null | grep -E "${FORBIDDEN}"; then
  echo "FAIL: domain 出现禁止依赖（见上方列表）" >&2
  exit 1
fi

echo "检查 application 不得依赖 platform-* ..."
if cargo tree -p meeting-copilot-application --edges normal --prefix none 2>/dev/null | grep -E '^meeting-copilot-platform-(win|mac)( |$)'; then
  echo "FAIL: application 直接依赖平台实现，须通过端口 trait 注入" >&2
  exit 1
fi

echo "PASS: 分层依赖符合约束"