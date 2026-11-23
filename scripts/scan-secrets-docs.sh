#!/usr/bin/env bash
# 六载体泄漏补充扫描：覆盖 *.md 与 .tmp/llm-live/**。
# 只输出 `文件:行号`，绝不回显命中内容。
# 退出码：0=无命中；1=有命中；2=扫描面缺失/不可读（不得当作 0）。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

SCAN_MD=0
SCAN_ARTIFACTS=0
SELF_TEST=0
ARTIFACT_DIR=".tmp/llm-live"
ARTIFACT_EXPLICIT=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --all-md) SCAN_MD=1 ;;
    --artifacts)
      SCAN_ARTIFACTS=1
      if [[ $# -ge 2 && "${2}" != --* ]]; then
        ARTIFACT_DIR="$2"
        ARTIFACT_EXPLICIT=1
        shift
      fi
      ;;
    --self-test) SELF_TEST=1 ;;
    *) echo "未知参数" >&2; exit 2 ;;
  esac
  shift
done

PATTERNS=(
  'sk-[A-Za-z0-9_-]{16,}'
  'sk-[A-Za-z0-9]{4,}\*{3,}[A-Za-z0-9]{2,}'
  '([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})'
  'https?://[^ \t"'\'']+\?[^ \t"'\'']+'
  'danger_accept_invalid|--insecure|CERT_NONE'
)

scan_file() {
  local file="$1"
  local pattern
  for pattern in "${PATTERNS[@]}"; do
    local matched
    matched="$(grep -nE -- "${pattern}" "${file}" 2>/dev/null || true)"
    # 规格文档会把禁用项原文写进“禁止/否决”条款；这类说明不是泄漏。
    if [[ "${pattern}" == 'danger_accept_invalid|--insecure|CERT_NONE' ]]; then
      matched="$(printf '%s\n' "${matched}" \
        | grep -vE '禁止|不得|不调用|否决|严禁|不允许|防止|规则 [0-9]|\*\*不\*\*|不.*调用' || true)"
    fi
    printf '%s\n' "${matched}" | grep -v '^$' \
      | cut -d: -f1 \
      | sed "s|^|${file}:|"
  done
}

scan_md() {
  local files
  files="$(git ls-files '*.md' 2>/dev/null || true)"
  if [[ -z "${files}" ]]; then
    return 2
  fi
  local file hit=0
  while IFS= read -r file; do
    [[ -z "${file}" ]] && continue
    local lines
    lines="$(scan_file "${file}")"
    if [[ -n "${lines}" ]]; then
      printf '%s\n' "${lines}"
      hit=1
    fi
  done <<< "${files}"
  [[ ${hit} -eq 1 ]] && return 1
  return 0
}

unknown_keys() {
  local file="$1"
  [[ -f "${file}" ]] || return 0
  node - "${file}" <<'JS'
const fs = require('fs');
const file = process.argv[2];
const knownRun = new Set(['schema_version','endpoint_host','model','started_at','ended_at','tz_offset','commit_short','config','per_task','sub_status','samples_path','stats_rule','build','path_confirmed','call_limit','calls_used','budget_hit','model_confirmed','host_locked','data_terms_declaration','data_terms_accepted_at','credential_rotated','tls_trust_root']);
const knownSample = new Set(['seq','task','attempt','first_token_ms','total_ms','outcome','failure_class','http_status','degraded','excluded']);
const isRun = file.endsWith('run.json');
let data;
try { data = JSON.parse(fs.readFileSync(file, 'utf8')); } catch { process.exit(0); }
const keys = isRun ? Object.keys(data) : Object.keys(data);
const known = isRun ? knownRun : knownSample;
const unknown = keys.filter((key) => !known.has(key));
if (unknown.length > 0) {
  for (const key of unknown) console.log(`${file}:${key}`);
  process.exit(1);
}
JS
}

scan_artifacts() {
  local dir="$1"
  local explicit="${2:-0}"
  if [[ ! -d "${dir}" ]]; then
    if [[ "${explicit}" -eq 1 ]]; then
      return 2
    fi
    return 0
  fi
  if [[ ! -r "${dir}" ]]; then
    return 2
  fi
  local hit=0 lines
  while IFS= read -r file; do
    lines="$(scan_file "${file}")"
    if [[ -n "${lines}" ]]; then
      printf '%s\n' "${lines}"
      hit=1
    fi
  done < <(find "${dir}" -type f 2>/dev/null)
  if [[ -f "${dir}/run.json" ]]; then
    if ! unknown_keys "${dir}/run.json" >/dev/null; then
      echo "${dir}/run.json:unknown-key"
      hit=1
    fi
  fi
  if [[ -f "${dir}/samples.jsonl" ]]; then
    local line
    while IFS= read -r line; do
      [[ -z "${line}" ]] && continue
      if ! node -e '
const known = new Set(["seq","task","attempt","first_token_ms","total_ms","outcome","failure_class","http_status","degraded","excluded"]);
const keys = Object.keys(JSON.parse(process.argv[1]));
if (keys.some((key) => !known.has(key))) process.exit(1);
' "${line}" >/dev/null 2>&1; then
        echo "${dir}/samples.jsonl:unknown-key"
        hit=1
      fi
    done < "${dir}/samples.jsonl"
  fi
  [[ ${hit} -eq 1 ]] && return 1
  return 0
}

if [[ ${SELF_TEST} -eq 1 ]]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "${tmp}"' EXIT
  fake="sk-$(printf 'abcd%.0s' {1..5})"
  printf 'fake=%s\n' "${fake}" > "${tmp}/leak.md"
  out="$(scan_file "${tmp}/leak.md")"
  if [[ -z "${out}" ]]; then
    echo "self-test failed: 未检出假凭据" >&2
    exit 1
  fi
  if printf '%s' "${out}" | grep -q 'abcdefghij'; then
    echo "self-test failed: 回显了内容" >&2
    exit 1
  fi
  echo "self-test ok: 只输出 文件:行号"
  exit 0
fi

if [[ ${SCAN_MD} -eq 0 && ${SCAN_ARTIFACTS} -eq 0 ]]; then
  echo "用法：--all-md | --artifacts [DIR] | --self-test" >&2
  exit 2
fi

final=0
if [[ ${SCAN_MD} -eq 1 ]]; then
  scan_md || final=$?
fi
if [[ ${SCAN_ARTIFACTS} -eq 1 ]]; then
  scan_artifacts "${ARTIFACT_DIR}" "${ARTIFACT_EXPLICIT}" || final=$?
fi
if [[ ${final} -eq 2 ]]; then
  echo "扫描面缺失或不可读" >&2
  exit 2
fi
exit ${final}