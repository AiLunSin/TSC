#!/usr/bin/env bash
# G2 宣称扫描：禁止把离线结果表述为三平台真实运行已验证。
# 追溯 AC-B2-OC-E12、spec 4.5 G2 宣称禁令。
# 退出码：0=PASS / 1=命中违规 / 2=扫描面缺失。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

FAIL=0
MISSING=0
PATTERN='三平台稳定运行已验证|真实 5 秒回退已实测|真实去重已验证|G2 门禁通过|G2 已通过|B2 已完成'

missing() {
  echo "$1" >&2
  MISSING=1
}

scan() {
  local label="$1"
  local target="$2"
  if [[ ! -e "${target}" ]]; then
    missing "${target}:0 扫描面缺失 [${label}]"
    return
  fi
  while IFS= read -r hit; do
    echo "${hit}  [${label}]" >&2
    FAIL=1
  done < <(grep -rInE "${PATTERN}" "${target}" \
      --exclude-dir=.codeartsdoer \
      --exclude-dir=target \
      --exclude='check-g2-claims.sh' 2>/dev/null || true)
}

if [[ "${1:-}" == "--self-test" ]]; then
  mkdir -p .tmp
  printf '结论：G2 门禁通过\n' > .tmp/g2-claims-selftest.md
  if grep -nE "${PATTERN}" .tmp/g2-claims-selftest.md >/dev/null 2>&1; then
    grep -nE "${PATTERN}" .tmp/g2-claims-selftest.md >&2
    echo "SELF-TEST: 植入宣称被识别（预期退出 1）" >&2
    exit 1
  fi
  echo "SELF-TEST: 检测器未识别植入宣称" >&2
  exit 2
fi

scan "progress" docs/progress
scan "capture-src" crates/application/src/capture
scan "capture-tests" crates/application/tests
scan "readme" README.md

if command -v git >/dev/null 2>&1; then
  while IFS= read -r line; do
    if [[ -n "${line}" ]] && echo "${line}" | grep -qE "${PATTERN}"; then
      echo "git-log:0 ${line}  [commit]" >&2
      FAIL=1
    fi
  done < <(git log --format=%s -20 2>/dev/null || true)
fi

if [[ "${MISSING}" -ne 0 ]]; then
  echo "SCAN-MISSING: G2 宣称扫描面缺失" >&2
  exit 2
fi

if [[ "${FAIL}" -ne 0 ]]; then
  echo "FAIL: 命中 G2 禁止宣称" >&2
  exit 1
fi

echo "PASS: G2 宣称扫描 0 命中"
exit 0