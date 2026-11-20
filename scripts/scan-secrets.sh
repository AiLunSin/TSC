#!/usr/bin/env bash
# 密钥与明文残留扫描。追溯 SPEC 4.3-3 · DESIGN 2.7.8-3。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

FAIL=0

scan() {
  local label="$1"
  local pattern="$2"
  local target="$3"
  local hits
  hits="$(grep -rInE "${pattern}" "${target}" \
    --exclude-dir=node_modules \
    --exclude-dir=target \
    --exclude-dir=.git \
    --exclude-dir=dist \
    --exclude='*.lock' \
    --exclude='scan-secrets.sh' \
    --exclude='*.md' 2>/dev/null || true)"
  if [[ -n "${hits}" ]]; then
    echo "FAIL [${label}]" >&2
    echo "${hits}" >&2
    FAIL=1
  fi
}

echo "扫描硬编码密钥..."
scan "私钥块" "BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY" "."
scan "常见令牌" "(sk-[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}|ghp_[A-Za-z0-9]{30,})" "."
scan "明文口令赋值" "(password|passwd|secret|api_key|apikey)[[:space:]]*[:=][[:space:]]*['\"][^'\"]{8,}['\"]" "app/src crates scripts"

echo "扫描明文数据库口令（不允许静默降级为明文库）..."
if grep -rInE "PRAGMA key[[:space:]]*=[[:space:]]*['\"]" app/src crates 2>/dev/null | grep -vE "keychain://|dpapi://|/\\* test \\*/"; then
  echo "FAIL: 发现内联数据库口令字面量" >&2
  FAIL=1
fi

if [[ "${FAIL}" -ne 0 ]]; then
  echo "密钥扫描未通过" >&2
  exit 1
fi

echo "PASS: 密钥/明文扫描 0 命中"