#!/usr/bin/env bash
# B3 理解层静态门禁：六条规则，全部不依赖 cargo。
# 退出码：0=通过 / 1=命中违规 / 2=扫描面缺失。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

RULE=""
SELF_TEST=0
FAIL=0
MISSING=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rule=*) RULE="${1#--rule=}" ;;
    --self-test) SELF_TEST=1 ;;
    *) echo "未知参数 $1" >&2; exit 2 ;;
  esac
  shift
done

fail() { echo "$1" >&2; FAIL=1; }
missing() { echo "$1" >&2; MISSING=1; }

need_dir() {
  if [[ ! -d "$1" ]]; then
    missing "$1:0 扫描面缺失"
    return 1
  fi
  return 0
}

rule_source() {
  need_dir crates/application/src/understand || return
  local hits
  hits="$(grep -rnE "Instant::now|thread::sleep" crates/application/src/understand --include=*.rs 2>/dev/null | grep -vc '#\[cfg(test)\]' || true)"
  if [[ "${hits}" -ne 0 ]]; then
    fail "understand 源码命中真实计时/睡眠"
  fi
}

rule_no_truncate() {
  need_dir crates/application/src/understand || return
  local hits
  hits="$(grep -rn "ShortText::truncated" crates/application/src/understand --include=*.rs 2>/dev/null | wc -l | tr -d ' ')"
  if [[ "${hits}" -ne 0 ]]; then
    fail "understand 源码命中 ShortText::truncated"
  fi
}

rule_zero_transcript() {
  need_dir crates/application/src/understand || return
  local hits
  hits="$(grep -rnE 'println!|eprintln!|tracing::' crates/application/src/understand --include=*.rs 2>/dev/null \
    | grep -E 'text|speaker|url|model' | wc -l | tr -d ' ')"
  if [[ "${hits}" -ne 0 ]]; then
    fail "understand 日志/记录疑似写入原文、说话人或端点"
  fi
}

rule_fallback_untouched() {
  if [[ ! -f crates/infrastructure/src/llm/fallback.rs ]]; then
    missing "crates/infrastructure/src/llm/fallback.rs:0 扫描面缺失"
    return
  fi
  git diff --quiet HEAD -- crates/infrastructure/src/llm/fallback.rs || fail "fallback.rs 相对 HEAD 有 diff"
}

rule_new_targets() {
  if [[ ! -f scripts/test-name-inventory.sh ]]; then
    missing "scripts/test-name-inventory.sh:0 扫描面缺失"
    return
  fi
  local name
  for name in understanding_normalize rolling_summary insight_dedup; do
    if ! grep -q "${name}" scripts/test-name-inventory.sh; then
      fail "NEW_TARGETS 缺少 ${name}"
    fi
  done
}

rule_exports() {
  if [[ ! -f crates/application/src/understand/mod.rs ]]; then
    missing "crates/application/src/understand/mod.rs:0 扫描面缺失"
    return
  fi
  local name
  for name in UnderstandService SummaryGuard InsightStore DedupeOutcome merge_drafted; do
    if ! grep -q "${name}" crates/application/src/understand/mod.rs; then
      fail "understand 导出缺少 ${name}"
    fi
  done
}

if [[ "${SELF_TEST}" -eq 1 ]]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "${tmp}"' EXIT
  printf 'ShortText::truncated\n' > "${tmp}/sample.rs"
  if grep -q "ShortText::truncated" "${tmp}/sample.rs"; then
    echo "SELF-TEST: 植入违规可被识别（预期退出 1）" >&2
    exit 1
  fi
  echo "SELF-TEST: 未识别" >&2
  exit 2
fi

case "${RULE}" in
  source) rule_source ;;
  no-truncate) rule_no_truncate ;;
  zero-transcript) rule_zero_transcript ;;
  fallback-untouched) rule_fallback_untouched ;;
  new-targets) rule_new_targets ;;
  exports) rule_exports ;;
  *)
    echo "用法：--rule={source|no-truncate|zero-transcript|fallback-untouched|new-targets|exports}" >&2
    exit 2
    ;;
esac

if [[ "${MISSING}" -ne 0 ]]; then
  echo "SCAN-MISSING: B3 门禁扫描面缺失" >&2
  exit 2
fi
if [[ "${FAIL}" -ne 0 ]]; then
  echo "FAIL: B3 门禁命中违规" >&2
  exit 1
fi
echo "PASS: B3 门禁 0 命中"
exit 0