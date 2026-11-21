#!/usr/bin/env bash
# 真实联调执行入口。默认关闭：未开启时只打印“未实测”并退出 0。
# 不接收凭据作为位置参数，不写临时凭据文件；凭据只经环境变量与 stdin。
# 退出码：0=通过 / 1=泄漏命中 / 2=不可判定（扫描面缺失、超时、凭据为空等）/ 其他=采样失败码。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

SCAN_ONLY=0
SAMPLE_RC=0
ROOT_OVERRIDE=""
PRINT_ROOTS=0
UNDETERMINED=0
EXIT_REASON=""
LEAK_HITS=0
DOC_HIT=0

record_roots() {
  printf '%s\n' "crates/infrastructure/.tmp/llm-live" ".tmp/llm-live"
}

skip() {
  echo "未实测（$1） code=$2"
  exit 0
}

reject() {
  echo "拒绝 code=$1"
  exit 2
}

finish() {
  if [[ "${UNDETERMINED}" -ne 0 ]]; then
    echo "不可判定 reason=${EXIT_REASON:-scan_timeout}" >&2
    exit 2
  fi
  if [[ "${LEAK_HITS}" -gt 0 || "${DOC_HIT}" -ne 0 ]]; then
    exit 1
  fi
  if [[ "${SAMPLE_RC}" -ne 0 ]]; then
    exit "${SAMPLE_RC}"
  fi
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scan-only) SCAN_ONLY=1 ;;
    --record-root)
      if [[ $# -lt 2 ]]; then
        echo "缺少 --record-root 参数" >&2
        exit 2
      fi
      ROOT_OVERRIDE="$2"
      shift
      ;;
    --selftest-sample-rc)
      if [[ $# -lt 2 || ! "${2}" =~ ^[0-9]+$ ]]; then
        echo "缺少或非法的 --selftest-sample-rc" >&2
        exit 2
      fi
      SAMPLE_RC="$2"
      SCAN_ONLY=1
      shift
      ;;
    --print-record-roots) PRINT_ROOTS=1 ;;
    *)
      echo "未知参数 $1" >&2
      exit 2
      ;;
  esac
  shift
done

if [[ "${PRINT_ROOTS}" -eq 1 ]]; then
  record_roots
  exit 0
fi

if [[ "${SCAN_ONLY}" -eq 0 ]]; then
  if [[ "${LLM_LIVE:-}" != "1" ]]; then
    skip "未开启真实联调" "live_disabled"
  fi
  if [[ -z "${LLM_BASE_URL:-}" ]]; then
    skip "端点未配置" "endpoint_missing"
  fi
  if [[ -z "${LLM_API_KEY:-}" ]]; then
    skip "凭据缺失" "credential_missing"
  fi
  if [[ -z "${LLM_MODEL:-}" ]]; then
    reject "model_not_confirmed"
  fi
  if [[ -z "${LLM_MODEL_CONFIRMED:-}" || "${LLM_MODEL}" != "${LLM_MODEL_CONFIRMED}" ]]; then
    reject "model_not_confirmed"
  fi
  if [[ -z "${LLM_ALLOWLIST:-}" ]]; then
    skip "外发白名单缺失" "allowlist_missing"
  fi
  if [[ -z "${LLM_LIVE_CALL_CAP:-}" || ! "${LLM_LIVE_CALL_CAP}" =~ ^[0-9]+$ || "${LLM_LIVE_CALL_CAP}" -eq 0 ]]; then
    reject "budget_not_confirmed"
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    skip "传输未启用" "transport_unavailable"
  fi
  if ! cargo test -p meeting-copilot-infrastructure --features llm-https --no-run --quiet >/dev/null 2>&1; then
    skip "传输未启用" "transport_unavailable"
  fi
fi

KEY="${LLM_API_KEY:-}"
if [[ -z "${KEY//[[:space:]]/}" ]]; then
  echo "credential_missing" >&2
  UNDETERMINED=1
  EXIT_REASON=credential_missing
fi

if [[ "${SCAN_ONLY}" -eq 0 ]]; then
  echo "说明：LLM_ALLOWLIST 仅作存在性检查，不参与 host 判定"
  echo "开始真实采样（串行、失败即停）"
  cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live \
    -- --ignored --test-threads=1 --fail-fast
  SAMPLE_RC=$?
fi

SCAN_BUDGET_S="${LLM_LIVE_SCAN_TIMEOUT_S:-10}"
TIMEOUT_BIN="${LLM_LIVE_TIMEOUT_BIN:-timeout}"
SCAN_START="${SECONDS}"
TARGETS=()

if [[ "${UNDETERMINED}" -eq 0 ]]; then
  if [[ -n "${ROOT_OVERRIDE}" ]]; then
    ABS="$(realpath -m "${ROOT_OVERRIDE}" 2>/dev/null || true)"
    case "${ABS}" in
      "${ROOT}"/*) ;;
      *)
        echo "record_root_outside_repo" >&2
        UNDETERMINED=1
        EXIT_REASON=record_root_outside_repo
        ;;
    esac
    if [[ "${UNDETERMINED}" -eq 0 && -d "${ROOT_OVERRIDE}" ]]; then
      TARGETS+=("${ROOT_OVERRIDE}")
    fi
  else
    while IFS= read -r root; do
      if [[ -d "${root}" ]]; then
        TARGETS+=("${root}")
      fi
    done < <(record_roots)
  fi

  if [[ "${UNDETERMINED}" -eq 0 && "${#TARGETS[@]}" -eq 0 ]]; then
    echo "record_dir_missing" >&2
    UNDETERMINED=1
    EXIT_REASON=record_dir_missing
  fi
fi

ensure_symlink_target() {
  local dir="$1"
  local link="${dir}/target"
  if [[ -L "${link}" ]]; then
    return 0
  fi
  rm -f "${link}" 2>/dev/null || true
  MSYS=winsymlinks:nativestrict ln -s ../leak "${link}" 2>/dev/null || true
  if [[ -L "${link}" ]]; then
    return 0
  fi
  rm -f "${link}" 2>/dev/null || true
  return 1
}

if [[ "${UNDETERMINED}" -eq 0 ]]; then
  for root in "${TARGETS[@]}"; do
    if [[ "${root}" == *"/scan/symlink" ]]; then
      if ! ensure_symlink_target "${root}"; then
        echo "symlink_unsupported_on_host" >&2
        UNDETERMINED=1
        EXIT_REASON=symlink_unsupported_on_host
        break
      fi
    fi

    remaining=$((SCAN_BUDGET_S - (SECONDS - SCAN_START)))
    if [[ "${remaining}" -le 0 ]]; then
      echo "scan_timeout" >&2
      UNDETERMINED=1
      EXIT_REASON=scan_timeout
      break
    fi

    files="$(printf '%s\n' "${KEY}" | "${TIMEOUT_BIN}" "${remaining}s" grep -rIlF -f - "${root}" 2>/dev/null)"
    rc=$?
    case "${rc}" in
      0)
        n="$(printf '%s\n' "${files}" | grep -c . || true)"
        LEAK_HITS=$((LEAK_HITS + n))
        ;;
      1) : ;;
      124)
        echo "scan_timeout" >&2
        UNDETERMINED=1
        EXIT_REASON=scan_timeout
        break
        ;;
      2)
        echo "record_dir_unreadable" >&2
        UNDETERMINED=1
        EXIT_REASON=record_dir_unreadable
        break
        ;;
      *)
        echo "scan_error rc=${rc}" >&2
        UNDETERMINED=1
        EXIT_REASON=scan_error
        break
        ;;
    esac

    if [[ "${root}" == crates/* ]]; then
      echo "scan_face=crate record_root=${root}"
    else
      echo "scan_face=repo record_root=${root}"
    fi

    bash scripts/scan-secrets-docs.sh --artifacts "${root}" >/dev/null 2>&1
    doc_rc=$?
    case "${doc_rc}" in
      0) : ;;
      1) DOC_HIT=1 ;;
      *)
        UNDETERMINED=1
        EXIT_REASON=scan_timeout
        break
        ;;
    esac
  done
fi

if [[ "${UNDETERMINED}" -eq 0 && "${#TARGETS[@]}" -gt 0 ]]; then
  echo "leak_hits=${LEAK_HITS}"
fi

if [[ "${UNDETERMINED}" -eq 0 && -n "${KEY}" ]]; then
  CMDLINE="$(ps -o args= -p $$ 2>/dev/null || true)"
  if [[ -n "${CMDLINE}" ]] && printf '%s\n' "${KEY}" | grep -qF -f - <<<"${CMDLINE}"; then
    LEAK_HITS=$((LEAK_HITS + 1))
  fi
fi

if [[ "${UNDETERMINED}" -eq 0 ]]; then
  bash scripts/scan-secrets.sh >/dev/null 2>&1
  secret_rc=$?
  case "${secret_rc}" in
    0) : ;;
    1) DOC_HIT=1 ;;
    *)
      UNDETERMINED=1
      EXIT_REASON=scan_timeout
      ;;
  esac
fi

finish