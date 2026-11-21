#!/usr/bin/env bash
# 真实联调执行入口。默认关闭：未开启时只打印“未实测”并退出 0。
# 不接收凭据作为位置参数，不写临时凭据文件；凭据只经环境变量与 stdin。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

skip() {
  echo "未实测（$1） code=$2"
  exit 0
}

reject() {
  echo "拒绝 code=$1"
  exit 2
}

# 判定顺序与 admit 的 11 步一致；此处只提示，不代替业务判定。
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

echo "说明：LLM_ALLOWLIST 仅作存在性检查，不参与 host 判定"
echo "开始真实采样（串行、默认关闭失败即退出）"
cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --ignored --test-threads=1
SAMPLE_RC=$?

echo "收尾：检查四泄漏面（只输出命中计数，不回显命中内容）"
LEAKS=0
for target in ".tmp/llm-live" "${TMPDIR:-/tmp}"; do
  if [[ -e "${target}" ]] && printf '%s\n' "${LLM_API_KEY}" | grep -RIlF -f - "${target}" >/dev/null 2>&1; then
    LEAKS=$((LEAKS + 1))
  fi
done
echo "泄漏面命中计数=${LEAKS}"

CMDLINE="$(ps -o args= -p $$ 2>/dev/null || true)"
if [[ -n "${CMDLINE}" ]] && printf '%s\n' "${LLM_API_KEY}" | grep -qF -f - <<<"${CMDLINE}"; then
  echo "进程命令行命中计数=1"
  LEAKS=$((LEAKS + 1))
else
  echo "进程命令行命中计数=0"
fi

bash scripts/scan-secrets-docs.sh --all-md --artifacts
DOC_RC=$?
if [[ ${DOC_RC} -eq 2 ]]; then
  echo "扫描面缺失，记为不可判定" >&2
  exit 2
fi
if [[ ${DOC_RC} -ne 0 ]]; then
  echo "文档/产物泄漏扫描命中" >&2
  exit 1
fi

bash scripts/scan-secrets.sh || exit 1

if [[ ${SAMPLE_RC} -ne 0 ]]; then
  exit "${SAMPLE_RC}"
fi
if [[ ${LEAKS} -ne 0 ]]; then
  exit 1
fi
exit 0