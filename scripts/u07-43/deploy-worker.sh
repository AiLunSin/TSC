#!/usr/bin/env bash
# B 路径：部署 U-07-43 的转发 Worker，并把采样配置一次性写好。
#
# 只需一个输入——检索服务的 Key：
#   UPSTREAM_KEY='<博查/Tavily Key>' bash scripts/u07-43/deploy-worker.sh --provider bocha
#   bash scripts/run-u07-43-samples.sh
#
# SHARED_TOKEN 缺省时自动生成随机串（不打印），并连同 Worker 地址写入
# .tmp/u07-43/config.env；主脚本会自动加载该文件，无需手动 export。
# 首次使用需先 `wrangler login`（或设置 CLOUDFLARE_API_TOKEN）。
#
# 选项：
#   --provider bocha|tavily   上游检索服务，默认 bocha
#   --dry-run                 只做语法与环境检查，不部署、不写配置
#   --skip-health             部署后不请求 /health 自检
#   -h | --help               显示本说明
#
# 可覆盖：WRANGLER（wrangler 可执行文件/命令）、U07_CONFIG_FILE（配置输出路径）。
# 退出码：0=成功 / 2=配置或部署错误。

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HERE="${ROOT}/scripts/u07-43"
CONFIG="${U07_CONFIG_FILE:-${ROOT}/.tmp/u07-43/config.env}"
cd "${HERE}"

PROVIDER="bocha"
DRY_RUN=0
SKIP_HEALTH=0

usage() {
  cat <<'EOF'
用法：
  UPSTREAM_KEY='<检索服务 Key>' bash scripts/u07-43/deploy-worker.sh [--provider bocha|tavily]
  bash scripts/run-u07-43-samples.sh

选项：
  --provider bocha|tavily   默认 bocha
  --dry-run                 只做本机检查，不部署
  --skip-health             跳过部署后的 /health 自检
  -h | --help               显示本说明

SHARED_TOKEN 未设置时自动生成；三个 secret 由脚本注入，不写进仓库。
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --provider)
      [[ $# -ge 2 ]] || { echo "--provider 需要 bocha 或 tavily" >&2; exit 2; }
      PROVIDER="$2"; shift ;;
    --dry-run) DRY_RUN=1 ;;
    --skip-health) SKIP_HEALTH=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "未知参数：$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

case "${PROVIDER}" in
  bocha|tavily) ;;
  *) echo "--provider 只支持 bocha 或 tavily，收到：${PROVIDER}" >&2; exit 2 ;;
esac

PY=""
for candidate in "${PYTHON:-}" python3 python py; do
  if [[ -n "${candidate}" ]] && command -v "${candidate}" >/dev/null 2>&1 \
    && "${candidate}" -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 8) else 1)' >/dev/null 2>&1; then
    PY="${candidate}"; break
  fi
done
[[ -n "${PY}" ]] || { echo "未找到 python3/python" >&2; exit 2; }

if [[ -z "${UPSTREAM_KEY:-}" && "${DRY_RUN}" -eq 0 ]]; then
  echo "缺少 UPSTREAM_KEY（博查/Tavily 的检索 Key）。示例：" >&2
  echo "  UPSTREAM_KEY='<你的 Key>' bash scripts/u07-43/deploy-worker.sh --provider ${PROVIDER}" >&2
  echo "只想检查本机环境时用 --dry-run（不要求 Key，也不部署）。" >&2
  exit 2
fi
UPSTREAM_KEY="${UPSTREAM_KEY:-dry-run-placeholder}"

if [[ -z "${SHARED_TOKEN:-}" ]]; then
  SHARED_TOKEN="$("${PY}" -c 'import secrets; print(secrets.token_urlsafe(24))')" || {
    echo "生成 SHARED_TOKEN 失败" >&2; exit 2;
  }
  echo "已自动生成 SHARED_TOKEN（不显示；值会写入 config.env）。"
fi

command -v node >/dev/null 2>&1 || { echo "未找到 node（Worker 语法检查与 wrangler 需要）" >&2; exit 2; }
node --check search-proxy.worker.js || { echo "Worker 语法检查失败" >&2; exit 2; }

if [[ "${DRY_RUN}" -eq 1 ]]; then
  echo "dry-run：Worker 语法与本机环境检查通过；未部署、未写 config.env。"
  echo "  name=u07-43-search-proxy provider=${PROVIDER} main=search-proxy.worker.js"
  echo "  下一步需真实部署：去掉 --dry-run 并提供 UPSTREAM_KEY。"
  exit 0
fi

mkdir -p "$(dirname "${CONFIG}")"
mkdir -p "${ROOT}/.tmp/u07-43"

if [[ -n "${WRANGLER:-}" ]]; then
  read -r -a WCMD <<< "${WRANGLER}"
elif [[ -x node_modules/.bin/wrangler ]]; then
  WCMD=(node_modules/.bin/wrangler)
elif command -v wrangler >/dev/null 2>&1; then
  WCMD=("$(command -v wrangler)")
else
  WCMD=(npx --yes wrangler@4)
fi

DEPLOY_LOG="${ROOT}/.tmp/u07-43/wrangler-deploy.log"
echo "部署 Worker（${WCMD[*]} deploy）…"
if ! "${WCMD[@]}" deploy >"${DEPLOY_LOG}" 2>&1; then
  echo "wrangler deploy 失败，日志尾部：" >&2
  tail -20 "${DEPLOY_LOG}" >&2 || true
  echo "常见原因：未登录（wrangler login）、网络不通、账号未开通 Workers。" >&2
  exit 2
fi

# 先 deploy 再注入：Worker 不存在时 secret put 会失败。secret 是绑定，注入后立即生效。
put_secret() {
  local name="$1" value="$2"
  printf '%s' "${value}" | "${WCMD[@]}" secret put "${name}" >/dev/null 2>&1
}
if ! put_secret UPSTREAM_KEY "${UPSTREAM_KEY}" \
  || ! put_secret SHARED_TOKEN "${SHARED_TOKEN}" \
  || ! put_secret UPSTREAM_PROVIDER "${PROVIDER}"; then
  echo "写入 Worker secret 失败。请确认已 wrangler login，或手动执行：" >&2
  echo "  ${WCMD[*]} secret put UPSTREAM_KEY" >&2
  echo "  ${WCMD[*]} secret put SHARED_TOKEN" >&2
  echo "  ${WCMD[*]} secret put UPSTREAM_PROVIDER" >&2
  exit 2
fi

WORKER_URL="$("${PY}" - "${DEPLOY_LOG}" <<'PYEOF'
import io, re, sys
try:
    text = io.open(sys.argv[1], encoding="utf-8", errors="replace").read()
except OSError:
    text = ""
match = re.search(r"https://[A-Za-z0-9._-]+\.workers\.dev", text)
print(match.group(0) if match else "")
PYEOF
)"

if [[ -n "${WORKER_URL}" && "${SKIP_HEALTH}" -eq 0 ]] && command -v curl >/dev/null 2>&1; then
  health_code="$(curl -s -o /dev/null -w '%{http_code}' -m 12 "${WORKER_URL}/health" || true)"
  if [[ "${health_code}" != "200" ]]; then
    echo "提示：/health 自检未返回 200（${health_code:-无响应}）；Worker 可能刚部署尚未生效，稍后重试即可。" >&2
  else
    echo "Worker 自检通过（/health 200）。"
  fi
fi

# token 只写进被 .gitignore 忽略的本地配置，不打印、不进提交。
U07_ENDPOINT="${WORKER_URL}" U07_TOKEN="${SHARED_TOKEN}" \
  "${PY}" - "${CONFIG}" <<'PYEOF'
import io, os, sys

endpoint = os.environ.get("U07_ENDPOINT", "").rstrip("/")
token = os.environ.get("U07_TOKEN", "").replace("\\", "\\\\").replace('"', '\\"')
url = f"{endpoint}/search" if endpoint else "https://<your-worker>.workers.dev/search"
lines = [
    "# 由 deploy-worker.sh 生成；.tmp/ 已被 .gitignore 忽略，勿提交。",
    f'SEARCH_ENDPOINT="{url}"',
    'SEARCH_AUTH_REF="SEARCH_API_TOKEN"',
    f'SEARCH_API_TOKEN="{token}"',
    "",
]
io.open(sys.argv[1], "w", encoding="utf-8", newline="\n").write("\n".join(lines))
PYEOF
[[ $? -eq 0 ]] || { echo "写入 config.env 失败" >&2; exit 2; }

echo "已写入 ${CONFIG}"
if [[ -z "${WORKER_URL}" ]]; then
  echo "未从 wrangler 输出解析到 Worker 地址，请把上面配置里的 SEARCH_ENDPOINT 改成实际地址。" >&2
else
  echo "Worker 地址：${WORKER_URL}/search"
fi
echo "下一步："
echo "  bash scripts/run-u07-43-samples.sh --check-only   # 离线预检（不出网）"
echo "  bash scripts/run-u07-43-samples.sh                # 3 次冒烟 + 200 条真实采样"
