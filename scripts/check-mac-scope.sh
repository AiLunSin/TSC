#!/usr/bin/env bash
# macOS 范围裁剪一致性门禁（AC-MS-01~11）。
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

fail() {
  echo "$1" >&2
  FAIL=1
}

missing() {
  echo "$1" >&2
  MISSING=1
}

need_file() {
  if [[ ! -f "$1" ]]; then
    missing "$1:0 扫描面缺失"
    return 1
  fi
  return 0
}

rule_na() {
  local spec_targets=(5.1.3 5.8.1 5.8.3)
  local task_targets=(8.1 8.2 8.3 8.4 16.1 16.2 16.3 16.4 71.1 71.2 71.3 9.2 17.2 18.1 19.1 20.1 21.1 22.1 44.2 49.3 49.4 75.3 67.1 69.1)
  local item line_no
  need_file docs/sdd/spec.md || return
  for item in "${spec_targets[@]}"; do
    line_no="$(grep -nF -- "${item}" docs/sdd/spec.md | head -1 | cut -d: -f1)"
    if [[ -z "${line_no}" ]]; then
      fail "docs/sdd/spec.md:0 缺少条款 ${item}"
      continue
    fi
    if ! sed -n "$((line_no + 1)),$((line_no + 6))p" docs/sdd/spec.md | grep -qF "平台处置"; then
      fail "docs/sdd/spec.md:${line_no} 条款 ${item} 缺少平台处置标注"
    fi
  done
  for item in "${task_targets[@]}"; do
    line_no="$(grep -nE -- "\*\*${item} " docs/sdd/tasks.md | head -1 | cut -d: -f1)"
    if [[ -z "${line_no}" ]]; then
      fail "docs/sdd/tasks.md:0 缺少条目 ${item}"
      continue
    fi
    if ! sed -n "$((line_no + 1)),$((line_no + 6))p" docs/sdd/tasks.md | grep -qF "平台处置"; then
      fail "docs/sdd/tasks.md:${line_no} 条目 ${item} 缺少平台处置标注"
    fi
  done
}

rule_matrix() {
  # 2026-11-25: platform-mac 已删除（用户决定），仅保留 Windows 能力矩阵检查。
  local win="crates/platform-win/resources/capability_matrix.toml"
  need_file "${win}" || return
  if [[ "$(grep -c '^\[\[capability\]\]' "${win}")" -ne 6 ]]; then
    fail "${win}:0 能力条目应恰为 6"
  fi
}

rule_ci() {
  need_file .github/workflows/ci.yml || return
  if [[ "$(grep -c 'apple-darwin' .github/workflows/ci.yml)" -lt 2 ]]; then
    fail ".github/workflows/ci.yml:0 缺少 mac 两 target 编译"
  fi
  if grep -q 'unknown-linux' .github/workflows/ci.yml; then
    fail ".github/workflows/ci.yml:0 仍含 unknown-linux 字面量"
  fi
  if grep -qE 'runs-on: ubuntu-latest' .github/workflows/ci.yml \
    && grep -B3 'runs-on: ubuntu-latest' .github/workflows/ci.yml | grep -q 'cargo test --workspace'; then
    fail ".github/workflows/ci.yml:0 Linux 被用作 rust 测试门禁"
  fi
  if [[ "$(grep -cE '^  (web|rust|test-name-inventory|feature-coverage|compliance|sqlcipher):$' .github/workflows/ci.yml)" -ne 6 ]]; then
    fail ".github/workflows/ci.yml:0 job 数不为 6"
  fi
}

rule_g2() {
  local files=(docs/progress/b2-status.md docs/progress/b2-offline-closure.md docs/sdd/tasks.md)
  local file
  for file in "${files[@]}"; do
    need_file "${file}" || continue
  done
  if grep -nF "三平台均可建立会话并稳定运行 30 分钟" docs/sdd/tasks.md >/dev/null 2>&1; then
    fail "docs/sdd/tasks.md:0 G2 仍以三平台作为通过条件"
  fi
  if grep -rnF "G2 仍需真实三平台" docs/progress/ >/dev/null 2>&1; then
    fail "docs/progress:0 仍宣称三平台 G2 为待办"
  fi
}

rule_claims() {
  local pattern='macOS 已完成|macOS 已验证|mac 真机通过|三平台真机矩阵全部通过'
  if grep -rInE "${pattern}" docs/ crates/ README.md 2>/dev/null | grep -vE '禁止|不得|不宣称|不适用|N/A|红线'; then
    fail "macOS 宣称命中"
  fi
}

rule_contracts() {
  local checks=(
    "deny.toml|x86_64-apple-darwin"
    "deny.toml|aarch64-apple-darwin"
    "crates/application/src/capture/merge_gate.rs|validate_merged_set"
    "crates/domain/src/model/utterance.rs|dual_channel"
    "crates/domain/src/model/channel.rs|ChannelKind"
    "crates/domain/src/model/channel.rs|ChannelState"
    "scripts/check-layering.sh|platform-mac"
    ".github/workflows/ci.yml|aarch64-apple-darwin"
  )
  local entry file needle
  for entry in "${checks[@]}"; do
    file="${entry%%|*}"
    needle="${entry#*|}"
    if [[ ! -f "${file}" ]]; then
      fail "${file}:0 契约文件缺失"
      continue
    fi
    if ! grep -qF -- "${needle}" "${file}"; then
      fail "${file}:0 契约缺失 ${needle}"
    fi
  done
}

rule_docs() {
  local mac_docs
  mac_docs="$(ls docs/test/mac-*.md 2>/dev/null || true)"
  if [[ -n "${mac_docs}" ]]; then
    local file
    while IFS= read -r file; do
      [[ -z "${file}" ]] && continue
      if ! head -1 "${file}" | grep -qF "不适用（无设备）"; then
        fail "${file}:0 mac 专项文档未标不适用"
      fi
    done <<< "${mac_docs}"
  fi
  local removed=0
  removed=$((removed + $(grep -c 'removed-by: mac_scope_and_b2_24' docs/sdd/design.md 2>/dev/null || true)))
  removed=$((removed + $(grep -c 'removed-by: mac_scope_and_b2_24' docs/sdd/tasks.md 2>/dev/null || true)))
  if [[ "${removed}" -ne 2 ]]; then
    fail "docs/sdd:0 removed-by 计数应为 2，实际 ${removed}"
  fi
  if grep -rnE 'mac 编译绿灯.*平台已验证' docs/ 2>/dev/null; then
    fail "docs:0 存在 mac 编译绿灯=已验证的表述"
  fi
}

rule_forbidden() {
  local pattern='fingerprint|voiceprint|speaker_embedding|audio_hash|声纹|音频指纹'
  if grep -rInE "${pattern}" crates/application/src/capture/merge_types.rs \
    crates/application/src/capture/dedupe_guard.rs crates/domain/src/merge crates/application/src/capture/merge_gate.rs 2>/dev/null; then
    fail "禁止字段命中"
  fi
}

if [[ "${SELF_TEST}" -eq 1 ]]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "${tmp}"' EXIT
  printf 'macOS 已完成\n' > "${tmp}/sample.md"
  if grep -q 'macOS 已完成' "${tmp}/sample.md"; then
    echo "SELF-TEST: 植入违规被识别（预期退出 1）" >&2
    exit 1
  fi
  echo "SELF-TEST: 未识别" >&2
  exit 2
fi

case "${RULE}" in
  na) rule_na ;;
  matrix) rule_matrix ;;
  ci) rule_ci ;;
  g2) rule_g2 ;;
  claims) rule_claims ;;
  contracts) rule_contracts ;;
  docs) rule_docs ;;
  forbidden) rule_forbidden ;;
  *)
    echo "用法：--rule={na|matrix|ci|g2|claims|contracts|docs|forbidden}" >&2
    exit 2
    ;;
esac

if [[ "${MISSING}" -ne 0 ]]; then
  echo "SCAN-MISSING: mac 范围门禁扫描面缺失" >&2
  exit 2
fi
if [[ "${FAIL}" -ne 0 ]]; then
  echo "FAIL: mac 范围门禁命中违规" >&2
  exit 1
fi
echo "PASS: mac 范围门禁 0 命中"
exit 0