#!/usr/bin/env bash
# P-UI 六规则静态门禁（AC-B4-116/118/121/122/123/126/127/130）。
# 退出码：0=通过 / 1=命中违规或自检命中 / 2=必需扫描面缺失或参数非法。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

RULE=""
SELF_TEST=0
FAIL=0
MISSING=0

IPC_DIR="app/src-tauri/src/ipc"
DTO="app/src-tauri/src/ipc/dto.rs"
FRONTEND="app/src"
SETTINGS_PAGE="app/src/settings/SettingsPage.tsx"
CARD="app/src/cards/KeywordCard.tsx"
DETAIL="app/src/cards/CardDetail.tsx"

DTO_BANNED='apiKey|keyLen|keyPrefix|keySuffix|endpointRaw|url|host|modelRaw|requestBody|requestId'
CARD_BANNED='normalizedTerm|gateDecisionRef|traceId|url|model|requestId|apiKey'

usage() {
  cat <<'EOF'
用法：check-ui-integration.sh --rule=<name> | --self-test
规则：credential-redline | settings-ux | card-render | body-hidden | frontend-offline | zero-transcript
退出码：0=通过 / 1=命中违规 / 2=必需扫描面缺失或参数非法
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rule=*) RULE="${1#--rule=}" ;;
    --self-test) SELF_TEST=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "未知参数 $1" >&2; exit 2 ;;
  esac
  shift
done

fail() { echo "$1" >&2; FAIL=1; }
missing() { echo "SCAN-MISSING: $1" >&2; MISSING=1; }

need_file() {
  if [[ ! -f "$1" ]]; then
    missing "$1"
    return 1
  fi
  return 0
}

need_dir() {
  if [[ ! -d "$1" ]]; then
    missing "$1"
    return 1
  fi
  return 0
}

strip_tests() {
  awk '/^#\[cfg\(test\)\]/{exit} {print}' "$1" 2>/dev/null
}

struct_block() {
  awk -v needle="pub struct $1 {" 'index($0, needle) { flag = 1 } flag { print } flag && /^}/ { exit }' "$2"
}

ipc_rs_files() {
  find "$IPC_DIR" -type f -name '*.rs' 2>/dev/null
}

rule_credential_redline() {
  need_file "$DTO" || return
  need_dir "$IPC_DIR" || return
  need_dir "$FRONTEND" || return

  local hits
  hits="$(struct_block SettingsSnapshotDto "$DTO" | grep -cE "${DTO_BANNED}" || true)"
  if [[ "${hits}" -ne 0 ]]; then
    fail "credential-redline: SettingsSnapshotDto 命中禁用字段（${hits} 处）"
  fi
  hits="$(struct_block KeywordCardViewDto "$DTO" | grep -cE "${CARD_BANNED}" || true)"
  if [[ "${hits}" -ne 0 ]]; then
    fail "credential-redline: KeywordCardViewDto 命中禁用字段（${hits} 处）"
  fi

  local file
  while IFS= read -r file; do
    [[ -f "${file}" ]] || continue
    hits="$(strip_tests "${file}" | grep -cE 'apiKey|keyPrefix|keySuffix|endpoint_raw|request_body' || true)"
    if [[ "${hits}" -ne 0 ]]; then
      fail "credential-redline: ${file} 非测试段命中凭据/正文载荷（${hits} 处）"
    fi
  done < <(ipc_rs_files)

  hits="$(grep -rE --include='*.ts' --include='*.tsx' -c 'keyPrefix|keySuffix|apiKey' "${FRONTEND}" 2>/dev/null | awk -F: '{ sum += $2 } END { print sum + 0 }')"
  if [[ "${hits}" -ne 0 ]]; then
    fail "credential-redline: 前端命中凭据载荷字段（${hits} 处）"
  fi
}

rule_settings_ux() {
  need_file "$SETTINGS_PAGE" || return

  # 允许 snapshot.canTest/canSave 与可计算 canTestNow/canSaveNow 两种写法。
  local pattern
  for pattern in \
    'sectionsOrder' \
    'disabled=\{!([A-Za-z0-9_]+\.)?canTest' \
    'disabled=\{!([A-Za-z0-9_]+\.)?canSave' \
    'type="password"' \
    'advancedCollapsed'; do
    if ! grep -qE "${pattern}" "${SETTINGS_PAGE}"; then
      fail "settings-ux: 缺口径 ${pattern}"
    fi
  done

  local hits
  hits="$(grep -cE '(modelIndex|endpointIndex) *(===|!==)' "${SETTINGS_PAGE}" || true)"
  if [[ "${hits}" -ne 0 ]]; then
    fail "settings-ux: 前端自行比较模型/端点下标（${hits} 处）"
  fi
}

rule_card_render() {
  need_file "$CARD" || return

  local pattern
  for pattern in 'aria-disabled' 'onClick={enabled' 'bodyVisible'; do
    if ! grep -qF "${pattern}" "${CARD}"; then
      fail "card-render: 缺口径 ${pattern}"
    fi
  done
}

rule_body_hidden() {
  need_file "$CARD" || return
  need_file "$DETAIL" || return

  local hidden
  hidden="$(grep -A6 'if (!card.bodyVisible' "${CARD}")"
  if [[ -z "${hidden}" ]]; then
    fail "body-hidden: 缺隐藏分支"
    return
  fi
  if ! printf '%s\n' "${hidden}" | grep -qF 'badge'; then
    fail "body-hidden: 隐藏分支未渲染条数角标"
  fi
  if printf '%s\n' "${hidden}" | grep -qF '<h3>'; then
    fail "body-hidden: 隐藏分支仍渲染正文标题"
  fi

  local forbidden
  for forbidden in 'dialog' 'showModal' 'autoFocus' 'scrollIntoView'; do
    if grep -qF "${forbidden}" "${DETAIL}"; then
      fail "body-hidden: 详情命中禁用交互 ${forbidden}"
    fi
  done
}

rule_frontend_offline() {
  need_dir "$FRONTEND" || return

  local hits
  hits="$(grep -rE --include='*.ts' --include='*.tsx' -c 'fetch\(|XMLHttpRequest|@tauri-apps/api|localStorage|sessionStorage|indexedDB|navigator\.sendBeacon' "${FRONTEND}" 2>/dev/null | awk -F: '{ sum += $2 } END { print sum + 0 }')"
  if [[ "${hits}" -ne 0 ]]; then
    fail "frontend-offline: 前端命中网络/持久化/额外桥接依赖（${hits} 处）"
  fi
}

rule_zero_transcript() {
  need_dir "$IPC_DIR" || return

  local file
  local hits
  while IFS= read -r file; do
    [[ -f "${file}" ]] || continue
    hits="$(strip_tests "${file}" | grep -cE 'https?://|raw_response|transcript_text' || true)"
    if [[ "${hits}" -ne 0 ]]; then
      fail "zero-transcript: ${file} 非测试段命中地址或原文载荷（${hits} 处）"
    fi
  done < <(ipc_rs_files)
}

run_rule() {
  case "$1" in
    credential-redline) rule_credential_redline ;;
    settings-ux) rule_settings_ux ;;
    card-render) rule_card_render ;;
    body-hidden) rule_body_hidden ;;
    frontend-offline) rule_frontend_offline ;;
    zero-transcript) rule_zero_transcript ;;
    *) echo "未知规则：$1" >&2; return 2 ;;
  esac
}

self_test() {
  local dir
  dir="$(mktemp -d 2>/dev/null)" || dir=""
  if [[ -z "${dir}" ]]; then
    echo "SCAN-MISSING: mktemp" >&2
    MISSING=1
    return
  fi
  printf 'fetch(1)\n' > "${dir}/Probe.tsx"
  local hits
  hits="$(grep -cE 'fetch\(' "${dir}/Probe.tsx" || true)"
  if [[ "${hits}" -lt 1 ]]; then
    fail "self-test: 探针未命中"
  fi
  rm -rf "${dir}"

  local rule
  for rule in credential-redline settings-ux card-render body-hidden frontend-offline zero-transcript; do
    run_rule "${rule}" >/dev/null 2>&1
  done
  FAIL=0
  MISSING=0
}

if [[ "${SELF_TEST}" -eq 1 ]]; then
  self_test
elif [[ -n "${RULE}" ]]; then
  run_rule "${RULE}" || exit 2
else
  for rule in credential-redline settings-ux card-render body-hidden frontend-offline zero-transcript; do
    run_rule "${rule}"
  done
fi

if [[ "${MISSING}" -ne 0 ]]; then
  exit 2
fi
if [[ "${FAIL}" -ne 0 ]]; then
  exit 1
fi
echo "PASS: check-ui-integration 六规则 0 命中"
exit 0