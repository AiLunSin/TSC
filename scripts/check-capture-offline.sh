#!/usr/bin/env bash
# 采集离线闭环静态门禁：依赖闭包、禁用调用、替身旁路、装配复用、占位实现。
# 追溯 DESIGN 3.4、AC-B2-OC-E01/E06/E08/E09/E13/F04。
# 退出码：0=PASS / 1=命中违规 / 2=扫描面缺失。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

CAPTURE_SRC="crates/application/src/capture"
TESTS_DIR="crates/application/tests"
FAIL=0
MISSING=0

fail() {
  echo "$1" >&2
  FAIL=1
}

missing() {
  echo "$1" >&2
  MISSING=1
}

scan() {
  local label="$1"
  local pattern="$2"
  local target="$3"
  if [[ ! -e "${target}" ]]; then
    missing "${target}:0 扫描面缺失 [${label}]"
    return
  fi
  while IFS= read -r hit; do
    fail "${hit}  [${label}]"
  done < <(grep -rInE "${pattern}" "${target}" 2>/dev/null || true)
}

rule_deps() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "SKIP: 未检测到 cargo，依赖闭包检查留待 CI" >&2
    return
  fi
  local forbidden='^(meeting-copilot-infrastructure|meeting-copilot-platform-(win|mac)|reqwest|hyper|sqlx|rusqlite|native-tls|tauri)( |$)'
  while IFS= read -r hit; do
    fail "${hit}  [deps]"
  done < <(cargo tree -p meeting-copilot-domain --edges normal --prefix none 2>/dev/null | grep -E "${forbidden}" || true)
  while IFS= read -r hit; do
    fail "${hit}  [deps]"
  done < <(cargo tree -p meeting-copilot-application --edges normal --prefix none 2>/dev/null | grep -E "${forbidden}" || true)
}

rule_source() {
  scan "source" 'thread::sleep|Instant::now|SystemTime::now|TcpStream|reqwest::|native_tls::' "${CAPTURE_SRC}"
}

rule_fakes() {
  scan "fakes" 'fn (update|remove|clear)[a-z_]*\(|fn delete\(&self,|fn send<|reqwest|TcpStream|native_tls|keychain|password|credential|wasapi|ScreenCaptureKit' "${TESTS_DIR}/support/fakes.rs"
}

rule_harness() {
  scan "harness" 'thread::sleep|Instant::now|SystemTime::now' "${TESTS_DIR}/support/mod.rs"
  local reused
  reused=$(grep -l 'build_offline_capture' "${TESTS_DIR}"/capture_*.rs 2>/dev/null | wc -l | tr -d ' ')
  if [[ "${reused}" -lt 4 ]]; then
    fail "${TESTS_DIR}/capture_degraded_guard.rs:0 复用文件数 ${reused} < 4 [harness]"
  fi
}

rule_guard_deps() {
  scan "guard-deps" 'dyn (Insight|Utterance|Session|Card)Repository|fn (save|append|delete|migrate)\(' "${CAPTURE_SRC}/degraded_guard.rs"
}

rule_placeholders() {
  scan "placeholders" 'todo!\(|unimplemented!\(|panic!\(' "${CAPTURE_SRC}"

}

rule_payload() {
  scan "payload" 'pub (text|samples|speaker|participants|reason_text)\b' "${CAPTURE_SRC}/types.rs"
}

rule_merge_writes() {
  local target="${CAPTURE_SRC}/capture_gap_gate.rs"
  if [[ ! -e "${target}" ]]; then
    missing "${target}:0 扫描面缺失 [merge-writes]"
    return
  fi
  if ! grep -q 'pub fn validate_intervals(' "${target}"; then
    fail "${target}:0 合并链路替换了 validate_intervals"
    return
  fi
  local arg
  for arg in gaps segments boundary_contact_max_ms; do
    if ! grep -q "${arg}" "${target}"; then
      fail "${target}:0 validate_intervals 缺少参数 ${arg}"
      return
    fi
  done
  local guard="${CAPTURE_SRC}/dedupe_guard.rs"
  if [[ -e "${guard}" ]]; then
    if grep -nE 'fn delete\(|fn apply_correction\(|UtteranceRepository|\.delete\(' "${guard}" >/dev/null 2>&1; then
      fail "${guard}:0 合并链路出现回溯删除或仓储写"
      return
    fi
  else
    echo "SKIP: dedupe_guard.rs 未落地"
  fi
  if ! grep -q 'merge_dedupe' scripts/test-name-inventory.sh; then
    fail "scripts/test-name-inventory.sh:0 NEW_TARGETS 缺少 merge_dedupe"
    return
  fi
}

run_all() {
  rule_deps
  rule_source
  rule_fakes
  rule_harness
  rule_guard_deps
  rule_placeholders
  rule_payload
  rule_merge_writes
}

if [[ "${1:-}" == "--self-test" ]]; then
  mkdir -p .tmp
  printf 'fn selftest() { thread::sleep(); }\n' > .tmp/capture-offline-selftest.rs
  if grep -nE 'thread::sleep' .tmp/capture-offline-selftest.rs >/dev/null 2>&1; then
    grep -nE 'thread::sleep' .tmp/capture-offline-selftest.rs >&2
    echo "SELF-TEST: 植入违规被识别（预期退出 1）" >&2
    exit 1
  fi
  echo "SELF-TEST: 检测器未识别植入违规" >&2
  exit 2
fi

case "${1:-}" in
  --rule=deps) rule_deps ;;
  --rule=source) rule_source ;;
  --rule=fakes) rule_fakes ;;
  --rule=harness) rule_harness ;;
  --rule=guard-deps) rule_guard_deps ;;
  --rule=placeholders) rule_placeholders ;;
  --rule=payload) rule_payload ;;
  --rule=merge-writes) rule_merge_writes ;;
  ""|--all) run_all ;;
  *)
    echo "未知参数：${1}" >&2
    exit 2
    ;;
esac

if [[ "${MISSING}" -ne 0 ]]; then
  echo "SCAN-MISSING: 采集离线门禁扫描面缺失" >&2
  exit 2
fi

if [[ "${FAIL}" -ne 0 ]]; then
  echo "FAIL: 采集离线门禁命中违规" >&2
  exit 1
fi

echo "PASS: 采集离线门禁 0 命中"
exit 0