#!/usr/bin/env bash
# U-07-43：一键采集 200 条真实检索样本（H-34-G / G-43-C 取证入口）。
#
# 它只做编排：准备 providers.json 与 200+ 条查询 → 预检 → 3 次冒烟 → 200 条采样
# → 输出只含计数与时延的报告。真正的网络请求由被 #[ignore] 的
# `crates/infrastructure/tests/search_live.rs` 发出，脚本不伪造任何样本。
#
# 三条路径都已配好，任选其一，填完端点与凭据后直接跑本脚本：
#   A. 自建 SearXNG（启用 format=json）：
#        bash scripts/u07-43/setup.sh --mode a --endpoint https://<域名>/search
#   B. 已有的博查 / Tavily Key：
#        UPSTREAM_KEY='<Key>' bash scripts/u07-43/deploy-worker.sh --provider bocha
#        （部署脚本会注入 secret 并写好 config.env）
#   C. 自定义 providers.json：
#        cp scripts/u07-43/providers.example.json .tmp/u07-43/providers.json
#        bash scripts/u07-43/setup.sh --mode c --providers .tmp/u07-43/providers.json --token '<凭据>'
#   bash scripts/run-u07-43-samples.sh
#
# 也可以手动 export（不写配置文件）：
#   export SEARCH_ENDPOINT="https://<公网 HTTPS 端点>/search"
#   export SEARCH_AUTH_REF="SEARCH_API_TOKEN"   # 可选，环境变量「名」不是值
#   export SEARCH_API_TOKEN="<Bearer 凭据>"
#   export SEARCH_PROVIDERS_FILE="<已有 providers.json 绝对路径>"  # C 路径
#   bash scripts/run-u07-43-samples.sh
#
# 配置会自动从 .tmp/u07-43/config.env 加载（可用 --config 指定、--no-config 忽略）。
# 端点必须是公网域名：外发白名单明确拒绝环回 / 私网 / 链路本地地址，本机代理不算真实源。
#
# 选项：
#   --check-only          只做预检与文件准备，不出网
#   --smoke-only          只跑 3 次冒烟
#   --skip-smoke          跳过冒烟直接跑 200 条（不推荐）
#   --out-dir DIR         中间产物目录，默认 .tmp/u07-43
#   --config FILE         指定配置文件（默认 ${U07_OUT_DIR:-.tmp/u07-43}/config.env）
#   --no-config           不加载任何配置文件，只用当前环境变量
#   --regenerate-queries  强制重新生成查询集（会覆盖自定义查询文件）
#   -h | --help           显示本说明
#
# 样本门槛固定 200：`collect_200_real_samples` 在 Rust 侧写死取前 200 条查询。
# 退出码：0=达标 / 1=测试失败或样本不足 / 2=配置或预检错误 / 3=样本够但时延预算不达标

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

TARGET=200
SKIP_SMOKE=0
CHECK_ONLY=0
SMOKE_ONLY=0
REGEN_QUERIES=0
NO_CONFIG=0
CONFIG_ARG=""
OUT_DIR="${U07_OUT_DIR:-.tmp/u07-43}"

usage() {
  cat <<'EOF'
用法：
  先配好（三选一）：
    A  bash scripts/u07-43/setup.sh --mode a --endpoint https://<域名>/search
    B  UPSTREAM_KEY='<Key>' bash scripts/u07-43/deploy-worker.sh --provider bocha
    C  bash scripts/u07-43/setup.sh --mode c --providers <绝对路径> --token '<凭据>'
  再跑：
    bash scripts/run-u07-43-samples.sh

选项：
  --check-only          只做预检与文件准备，不出网
  --smoke-only          只跑 3 次冒烟
  --skip-smoke          跳过冒烟直接跑 200 条（不推荐）
  --out-dir DIR         中间产物目录，默认 .tmp/u07-43
  --config FILE         指定配置文件，默认 .tmp/u07-43/config.env
  --no-config           忽略配置文件，只用当前环境变量
  --regenerate-queries  强制重新生成查询集
  -h | --help           显示本说明

也可手动 export SEARCH_ENDPOINT / SEARCH_AUTH_REF / <凭据> / SEARCH_PROVIDERS_FILE。
端点必须是公网 HTTPS：外发白名单拒绝环回 / 私网 / 链路本地地址。
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check-only) CHECK_ONLY=1 ;;
    --smoke-only) SMOKE_ONLY=1 ;;
    --skip-smoke) SKIP_SMOKE=1 ;;
    --regenerate-queries) REGEN_QUERIES=1 ;;
    --no-config) NO_CONFIG=1 ;;
    --out-dir)
      [[ $# -ge 2 ]] || { echo "--out-dir 需要路径" >&2; exit 2; }
      OUT_DIR="$2"; shift ;;
    --config)
      [[ $# -ge 2 ]] || { echo "--config 需要路径" >&2; exit 2; }
      CONFIG_ARG="$2"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "未知参数：$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

PY=""
for candidate in "${PYTHON:-}" python3 python py; do
  if [[ -n "${candidate}" ]] && command -v "${candidate}" >/dev/null 2>&1; then
    PY="${candidate}"; break
  fi
done
if [[ -z "${PY}" ]]; then echo "未找到 python3/python，无法生成查询集与解析报告" >&2; exit 2; fi

# cargo test 的测试进程 CWD 是 crate 根目录，不是仓库根目录；相对路径会被当成
# "文件不存在" 而静默得到零源。这里统一转成 Windows/Unix 绝对路径再传下去。
abs_path() {
  "${PY}" -c 'import os, sys; print(os.path.abspath(sys.argv[1]))' "$1"
}

mkdir -p "${OUT_DIR}"

# ---------- 0. 加载本地配置（凭据不进仓库） ----------
# 顺序很重要：必须在给 SEARCH_* 赋默认值之前 source，否则配置里的值会被默认值盖掉。
CONFIG_FILE="${CONFIG_ARG:-${U07_CONFIG_FILE:-${OUT_DIR}/config.env}}"
LOADED_CONFIG=""
if [[ "${NO_CONFIG}" -eq 0 && -f "${CONFIG_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${CONFIG_FILE}"
  set +a
  LOADED_CONFIG="$(abs_path "${CONFIG_FILE}")"
  echo "已加载配置：${LOADED_CONFIG}"
elif [[ -n "${CONFIG_ARG}" && "${NO_CONFIG}" -eq 0 ]]; then
  echo "指定的配置文件不存在：${CONFIG_FILE}" >&2
  exit 2
fi

# 记录用户/配置是否显式给了 providers.json：给了且文件已存在 = C 路径，脚本绝不改动它。
EXPLICIT_PROVIDERS="${SEARCH_PROVIDERS_FILE:-}"
SEARCH_PROVIDERS_FILE="${SEARCH_PROVIDERS_FILE:-${OUT_DIR}/providers.json}"
SEARCH_QUERIES_FILE="${SEARCH_QUERIES_FILE:-${OUT_DIR}/queries.txt}"
REPORT_PATH="${SEARCH_REPORT_PATH:-${OUT_DIR}/report.json}"
SMOKE_REPORT_PATH="${OUT_DIR}/smoke.json"
PROVIDERS_ABS="$(abs_path "${SEARCH_PROVIDERS_FILE}")"
QUERIES_ABS="$(abs_path "${SEARCH_QUERIES_FILE}")"
REPORT_ABS="$(abs_path "${REPORT_PATH}")"
SMOKE_REPORT_ABS="$(abs_path "${SMOKE_REPORT_PATH}")"

CUSTOM_PROVIDERS=0
if [[ -n "${EXPLICIT_PROVIDERS}" && -f "${SEARCH_PROVIDERS_FILE}" ]]; then
  CUSTOM_PROVIDERS=1
  echo "C 路径：使用自定义 providers.json（${PROVIDERS_ABS}），脚本不会修改它。"
fi

# ---------- 1. 准备 providers.json（C 路径跳过） ----------
# 换端点重跑时必须重建，否则会静默沿用上一次的（占位）配置。
FORCE_PROVIDERS=0
if [[ "${CUSTOM_PROVIDERS}" -eq 0 && -f "${SEARCH_PROVIDERS_FILE}" && -n "${SEARCH_ENDPOINT:-}" ]]; then
  existing_endpoint="$("${PY}" - "${SEARCH_PROVIDERS_FILE}" <<'PYEOF'
import io, json, sys
try:
    doc = json.load(io.open(sys.argv[1], encoding="utf-8"))
    print((doc.get("providers") or [{}])[0].get("endpoint", ""))
except (OSError, ValueError, IndexError, TypeError):
    print("")
PYEOF
)"
  if [[ -n "${existing_endpoint}" && "${existing_endpoint}" != "${SEARCH_ENDPOINT}" ]]; then
    echo "提示：SEARCH_ENDPOINT 与现有 providers.json 的端点不同，按新端点重建配置。"
    FORCE_PROVIDERS=1
  fi
fi

if [[ "${CUSTOM_PROVIDERS}" -eq 0 && ( ! -f "${SEARCH_PROVIDERS_FILE}" || "${FORCE_PROVIDERS}" -eq 1 ) ]]; then
  if [[ -z "${SEARCH_ENDPOINT:-}" ]]; then
    echo "缺少检索源配置：未找到 ${SEARCH_PROVIDERS_FILE}，且未设置 SEARCH_ENDPOINT。" >&2
    echo "任选其一：" >&2
    echo "  A. bash scripts/u07-43/setup.sh --mode a --endpoint https://<域名>/search" >&2
    echo "  B. UPSTREAM_KEY='<Key>' bash scripts/u07-43/deploy-worker.sh --provider bocha" >&2
    echo "  C. bash scripts/u07-43/setup.sh --mode c --providers <绝对路径> --token '<凭据>'" >&2
    exit 2
  fi
  if [[ ! "${SEARCH_ENDPOINT}" =~ ^https://[^/[:space:]]+(/.*)?$ ]]; then
    echo "SEARCH_ENDPOINT 必须是 https:// 开头的绝对地址（不含账号密码与空白）" >&2
    exit 2
  fi
  "${PY}" - "${SEARCH_ENDPOINT}" "${SEARCH_AUTH_REF:-}" "${SEARCH_PROVIDERS_FILE}" <<'PYEOF'
import io, json, sys
endpoint, auth_ref, out = sys.argv[1], sys.argv[2], sys.argv[3]
provider = {
    "id": "u07-43-source",
    "name": "U-07-43 采样源",
    "kind": "custom",
    "priority": 1,
    "weight": 60,
    "enabled": "enabled",
    "timeout": 8000,
    "rateLimit": {"perMinute": 60, "perDay": 2000},
    "blockedDomains": [],
    "health": "healthy",
    "adapter": "http_json",
    "endpoint": endpoint,
}
if auth_ref:
    provider["authRef"] = auth_ref
doc = {"schemaVersion": "1.0", "providers": [provider]}
with io.open(out, "w", encoding="utf-8", newline="\n") as handle:
    handle.write(json.dumps(doc, ensure_ascii=False, indent=2) + "\n")
print(f"已生成 {out}")
PYEOF
  [[ $? -eq 0 ]] || { echo "providers.json 生成失败" >&2; exit 2; }
fi

# ---------- 2. 准备查询集 ----------
if [[ ! -f "${SEARCH_QUERIES_FILE}" || "${REGEN_QUERIES}" -eq 1 ]]; then
  "${PY}" scripts/u07-43/gen-queries.py --count 240 --out "${SEARCH_QUERIES_FILE}" \
    || { echo "查询集生成失败" >&2; exit 2; }
fi

# ---------- 3. 离线预检（不打印任何凭据、端点全文与查询原文） ----------
PREFLIGHT_MODE="generated"
[[ "${CUSTOM_PROVIDERS}" -eq 1 ]] && PREFLIGHT_MODE="custom"
"${PY}" scripts/u07-43/preflight.py "${SEARCH_PROVIDERS_FILE}" "${SEARCH_QUERIES_FILE}" \
  --target "${TARGET}" --mode "${PREFLIGHT_MODE}" || exit 2

# ---------- 4. 本机编译环境（Windows / MSVC 常见坑） ----------
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    RUST_BIN="${HOME}/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin"
    [[ -d "${RUST_BIN}" ]] && export PATH="${RUST_BIN}:${PATH}"
    mkdir -p "${ROOT}/.tmp/msvc-temp"
    # 必须 Windows 风格绝对路径：MSYS 的 /c/... 会让 std::env::temp_dir()
    # 落到不可写位置，若干临时文件用例报「拒绝访问」。
    # 注意分两行 export：写在同一行时 ${TEMP} 展开的是旧值（先展开后赋值），
    # GetTempPathW 会优先读 TMP 又落回系统临时目录。
    export TEMP="$(abs_path "${ROOT}/.tmp/msvc-temp")"
    export TMP="${TEMP}"
    export CARGO_PROFILE_DEV_DEBUG=0
    ;;
esac
# 必须带 llm-https：缺省特性下 HttpJsonProvider 没有真实 TLS 传输，
# 每次检索都会 Unreachable，200 条样本不可能达标。
FEATURES=(--features llm-https)

if [[ "${CHECK_ONLY}" -eq 1 ]]; then
  echo "check-only：未发起任何网络请求。"
  [[ -n "${LOADED_CONFIG}" ]] && echo "  config    : ${LOADED_CONFIG}"
  echo "  providers : ${PROVIDERS_ABS}（${PREFLIGHT_MODE}）"
  echo "  queries   : ${QUERIES_ABS}"
  echo "  report    : ${REPORT_ABS}"
  echo "  cargo test -p meeting-copilot-infrastructure ${FEATURES[*]} --test search_live collect_200_real_samples -- --ignored --test-threads=1 --nocapture"
  exit 0
fi

run_case() {
  local case_name="$1" report="$2"
  echo "---- ${case_name} 开始（真实出网，串行执行，请耐心等待）----"
  SEARCH_PROVIDERS_FILE="${PROVIDERS_ABS}" \
  SEARCH_QUERIES_FILE="${QUERIES_ABS}" \
  SEARCH_REPORT_PATH="$(abs_path "${report}")" \
    cargo test -p meeting-copilot-infrastructure "${FEATURES[@]}" \
      --test search_live "${case_name}" -- --ignored --test-threads=1 --nocapture
}

# ---------- 5. 先冒烟，再跑 200 条 ----------
if [[ "${SKIP_SMOKE}" -eq 0 ]]; then
  run_case smoke_three_rounds "${SMOKE_REPORT_PATH}"
  smoke_rc=$?
  if [[ ${smoke_rc} -ne 0 ]]; then
    echo "3 次冒烟未通过（rc=${smoke_rc}）。常见原因：" >&2
    echo "  1) 端点不可达 / 返回非 2xx / 不是 {results:[{url,title}]} 形状" >&2
    echo "  2) authRef 的凭据无效（401/403）或触发限流（429）" >&2
    echo "  3) 忘了 --features llm-https（本脚本已固定带上）" >&2
    echo "  B 路径可先访问 <Worker 地址>/health 确认 secret 已注入。" >&2
    echo "冒烟未通不要直接跑 200 条；修好端点后重试。" >&2
    exit 1
  fi
  echo "冒烟通过。"
fi

if [[ "${SMOKE_ONLY}" -eq 1 ]]; then
  echo "smoke-only：已跳过 200 条采样。"
  exit 0
fi

run_case collect_200_real_samples "${REPORT_PATH}"
test_rc=$?
if [[ ${test_rc} -ne 0 ]]; then
  echo "采样测试失败（rc=${test_rc}）；下方报告给出已采集到的样本数。" >&2
fi

# ---------- 6. 判定（样本数 + p95 + 超时率） ----------
# 无论 cargo 是否通过都判一次：断言失败时报告里仍留着部分样本，便于定位差多少。
"${PY}" scripts/u07-43/verdict.py "${REPORT_PATH}" --target "${TARGET}"
verdict_rc=$?
if [[ ${test_rc} -ne 0 ]]; then
  echo "采样测试失败（rc=${test_rc}），以测试失败为准。" >&2
  exit 1
fi
exit "${verdict_rc}"
