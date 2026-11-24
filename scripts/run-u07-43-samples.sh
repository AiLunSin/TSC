#!/usr/bin/env bash
# U-07-43：一键采集 200 条真实检索样本（H-34-G / G-43-C 取证入口）。
#
# 它只做编排：准备 providers.json 与 200+ 条查询 → 预检 → 3 次冒烟 → 200 条采样
# → 输出只含计数与时延的报告。真正的网络请求由被 #[ignore] 的
# `crates/infrastructure/tests/search_live.rs` 发出，脚本不伪造任何样本。
#
# 必备：一个**公网可达的 HTTPS 检索端点**，能响应
#   GET <endpoint>?q=<query>&format=json  →  {"results":[{"url":"https://...","title":"..."}]}
# 三种来源任选其一：
#   A. 自建/可用的 SearXNG（`format=json` 需启用）——直接填 SEARCH_ENDPOINT。
#   B. 已有的博查 / Tavily Key ——部署 scripts/u07-43/search-proxy.worker.js，
#      再把 Worker 的公开地址填给 SEARCH_ENDPOINT。
#   C. 自己写好 providers.json ——设 SEARCH_PROVIDERS_FILE，跳过生成。
# 端点必须是公网域名：外发白名单明确拒绝环回 / 私网 / 链路本地地址，本机代理不算真实源。
#
# 用法：
#   export SEARCH_ENDPOINT="https://your-host.example/search"
#   export SEARCH_AUTH_REF="SEARCH_API_TOKEN"   # 可选；环境变量「名」，不是值
#   export SEARCH_API_TOKEN="<Bearer 凭据>"      # 仅当设置了 authRef
#   bash scripts/run-u07-43-samples.sh
#
# 选项：
#   --check-only          只做预检与文件准备，不出网
#   --smoke-only          只跑 3 次冒烟
#   --check-only          只做预检与文件准备，不出网
#   --smoke-only          只跑 3 次冒烟
#   --skip-smoke          跳过冒烟直接跑 200 条（不推荐）
#   --out-dir DIR         中间产物目录，默认 .tmp/u07-43
#   --regenerate-queries  强制重新生成查询集（会覆盖自定义查询文件）
#
# 样本门槛固定 200：`collect_200_real_samples` 在 Rust 侧写死取前 200 条查询。
#
# 退出码：0=达标 / 1=测试失败或样本不足 / 2=配置或预检错误 / 3=样本够但时延预算不达标

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

TARGET=200
SKIP_SMOKE=0
CHECK_ONLY=0
SMOKE_ONLY=0
REGEN_QUERIES=0
OUT_DIR="${U07_OUT_DIR:-.tmp/u07-43}"

usage() {
  cat <<'EOF'
用法：
  export SEARCH_ENDPOINT="https://your-host.example/search"
  export SEARCH_AUTH_REF="SEARCH_API_TOKEN"   # 可选，环境变量「名」不是值
  export SEARCH_API_TOKEN="<Bearer 凭据>"      # 仅当设置了 authRef
  bash scripts/run-u07-43-samples.sh [选项]

选项：
  --check-only          只做预检与文件准备，不出网
  --smoke-only          只跑 3 次冒烟
  --skip-smoke          跳过冒烟直接跑 200 条（不推荐）
  --out-dir DIR         中间产物目录，默认 .tmp/u07-43
  --regenerate-queries  强制重新生成查询集（会覆盖自定义查询文件）
  -h | --help           显示本说明

也可改用 export SEARCH_PROVIDERS_FILE / SEARCH_QUERIES_FILE / SEARCH_REPORT_PATH。
端点必须是公网 HTTPS：外发白名单拒绝环回 / 私网 / 链路本地地址。
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check-only) CHECK_ONLY=1 ;;
    --smoke-only) SMOKE_ONLY=1 ;;
    --skip-smoke) SKIP_SMOKE=1 ;;
    --regenerate-queries) REGEN_QUERIES=1 ;;
    --out-dir)
      [[ $# -ge 2 ]] || { echo "--out-dir 需要路径" >&2; exit 2; }
      OUT_DIR="$2"; shift ;;
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
SEARCH_PROVIDERS_FILE="${SEARCH_PROVIDERS_FILE:-${OUT_DIR}/providers.json}"
SEARCH_QUERIES_FILE="${SEARCH_QUERIES_FILE:-${OUT_DIR}/queries.txt}"
REPORT_PATH="${SEARCH_REPORT_PATH:-${OUT_DIR}/report.json}"
SMOKE_REPORT_PATH="${OUT_DIR}/smoke.json"
PROVIDERS_ABS="$(abs_path "${SEARCH_PROVIDERS_FILE}")"
QUERIES_ABS="$(abs_path "${SEARCH_QUERIES_FILE}")"
REPORT_ABS="$(abs_path "${REPORT_PATH}")"
SMOKE_REPORT_ABS="$(abs_path "${SMOKE_REPORT_PATH}")"

# ---------- 1. 准备 providers.json ----------
# 换端点重跑时必须重建，否则会静默沿用上一次的（占位）配置。
FORCE_PROVIDERS=0
if [[ -f "${SEARCH_PROVIDERS_FILE}" && -n "${SEARCH_ENDPOINT:-}" ]]; then
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

if [[ ! -f "${SEARCH_PROVIDERS_FILE}" || "${FORCE_PROVIDERS}" -eq 1 ]]; then
  if [[ -z "${SEARCH_ENDPOINT:-}" ]]; then
    echo "缺少检索源配置：未找到 ${SEARCH_PROVIDERS_FILE}，且未设置 SEARCH_ENDPOINT。" >&2
    echo "请任选其一：" >&2
    echo "  A. export SEARCH_ENDPOINT='https://<SearXNG 公网域名>/search'" >&2
    echo "  B. 部署 scripts/u07-43/search-proxy.worker.js 后填 Worker 地址" >&2
    echo "  C. export SEARCH_PROVIDERS_FILE='<已有 providers.json 绝对路径>'" >&2
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

# ---------- 3. 预检（不打印任何凭据） ----------
"${PY}" - "${SEARCH_PROVIDERS_FILE}" "${SEARCH_QUERIES_FILE}" "${TARGET}" <<'PYEOF' || exit 2
import io, json, sys
from urllib.parse import urlsplit

providers_path, queries_path, target = sys.argv[1], sys.argv[2], int(sys.argv[3])

try:
    doc = json.load(io.open(providers_path, encoding="utf-8"))
except (OSError, ValueError) as exc:
    print(f"providers.json 解析失败：{exc}", file=sys.stderr)
    raise SystemExit(2)
if doc.get("schemaVersion") != "1.0":
    print("schemaVersion 必须为 1.0", file=sys.stderr); raise SystemExit(2)
usable = [p for p in doc.get("providers", [])
          if p.get("enabled", "enabled") == "enabled" and p.get("health", "healthy") == "healthy"]
if not usable:
    print("没有启用且健康的检索源", file=sys.stderr); raise SystemExit(2)
for p in usable:
    ep = p.get("endpoint", "")
    parts = urlsplit(ep)
    if parts.scheme != "https" or not parts.hostname or parts.username or parts.password:
        print("端点必须是无凭据的 HTTPS 绝对地址", file=sys.stderr); raise SystemExit(2)
    if p.get("adapter") != "http_json":
        print("adapter 必须为 http_json", file=sys.stderr); raise SystemExit(2)
host = urlsplit(usable[0]["endpoint"]).hostname
auth_ref = usable[0].get("authRef")
if auth_ref:
    import os
    if not os.environ.get(auth_ref):
        print(f"authRef 指向的环境变量 {auth_ref} 未设置或为空", file=sys.stderr)
        raise SystemExit(2)

lines = [line.strip() for line in io.open(queries_path, encoding="utf-8") if line.strip()]
unique = set(lines)
if len(lines) < target or len(unique) < target:
    print(f"查询不足：{len(lines)} 行 / {len(unique)} 条不重复，门槛 {target} 条", file=sys.stderr)
    raise SystemExit(2)
if any(len(q) > 200 for q in lines):
    print("存在超过 200 字的查询（SanitizedQuery 上限）", file=sys.stderr)
    raise SystemExit(2)
print(f"预检通过：{len(usable)} 个可用源，主机 {host}；{len(lines)} 条查询 / {len(unique)} 条不重复")
print(f"前 {target} 条将作为样本；结果不计入：缓存命中、失败、超时")
PYEOF

# ---------- 4. 本机编译环境（Windows / MSVC 常见坑） ----------
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    RUST_BIN="${HOME}/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin"
    [[ -d "${RUST_BIN}" ]] && export PATH="${RUST_BIN}:${PATH}"
    mkdir -p "${ROOT}/.tmp/msvc-temp"
    # 必须 Windows 风格绝对路径：MSYS 的 /c/... 会让 std::env::temp_dir()
    # 落到不可写位置，若干临时文件用例会报「拒绝访问」。
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
  echo "  providers : ${PROVIDERS_ABS}"
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
