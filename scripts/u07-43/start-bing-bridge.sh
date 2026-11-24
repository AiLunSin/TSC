#!/usr/bin/env bash
# U-07-43 D 路径：零账号、零 Key 的真实检索采样。
#
# 组成：本机检索桥（必应 RSS → {results:[{url,title}]}）
#     + cloudflared quick tunnel（公网 HTTPS，不需要 Cloudflare 账号）
#     + run-u07-43-samples.sh（3 次冒烟 → 200 条 → 判定）
#
#   bash scripts/u07-43/start-bing-bridge.sh              # 全流程
#   bash scripts/u07-43/start-bing-bridge.sh --check-only # 只建端点并预检，不出网采样
#   bash scripts/u07-43/start-bing-bridge.sh -- --smoke-only
#
# 选项：
#   --port N         桥端口，默认 8788
#   --no-download    不自动下载 cloudflared（找不到就报错）
#   --protocol auto|http2  隧道传输协议，默认 auto；QUIC 不通时用 http2
#   -h | --help      显示本说明
#   --               其后的参数原样传给 run-u07-43-samples.sh
#
# 退出时会停掉桥与隧道；配置与日志在 .tmp/u07-43/（已 gitignore，凭据不入库）。

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

PORT=8788
# quick tunnel 用 QUIC 在实测中可建立连接且地址稳定，http2 在本机自检阶段连不通，故默认 auto（QUIC 优先）。
PROTOCOL="auto"
TUNNEL_MODE="auto"
NO_DOWNLOAD=0
SERVE_ONLY=0
OUT_DIR=".tmp/u07-43"
CONFIG="${OUT_DIR}/config.env"
BRIDGE_LOG="${OUT_DIR}/bridge.log"
TUNNEL_LOG="${OUT_DIR}/tunnel.log"
PASS_ARGS=()

usage() {
  cat <<'EOF'
用法：
  bash scripts/u07-43/start-bing-bridge.sh               # 建端点 → 3 次冒烟 → 200 条采样
  bash scripts/u07-43/start-bing-bridge.sh --check-only  # 只建端点并预检，不出网采样
  bash scripts/u07-43/start-bing-bridge.sh --serve-only  # 只建端点并保持，采样另开终端跑
  bash scripts/u07-43/start-bing-bridge.sh --smoke-only  # 只跑 3 次冒烟

选项：
  --port N                 桥端口，默认 8788
  --protocol auto|http2|quic  隧道传输协议，默认 auto
  --tunnel auto|cloudflare|lhr  隧道种类，默认 auto（cloudflared 优先，429 时回退 localhost.run）
  --serve-only             建好端点后保持 3600 秒，不跑采样
  --no-download            不自动下载 cloudflared
  -h | --help              显示本说明
  --                       其后的参数原样传给 run-u07-43-samples.sh

退出时会停掉桥与隧道；配置与日志在 .tmp/u07-43/（已 gitignore，凭据不入库）。
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port) [[ $# -ge 2 ]] || { echo "--port 需要端口号" >&2; exit 2; }; PORT="$2"; shift ;;
    --protocol) [[ $# -ge 2 ]] || { echo "--protocol 需要 auto 或 http2" >&2; exit 2; }; PROTOCOL="$2"; shift ;;
    --tunnel) [[ $# -ge 2 ]] || { echo "--tunnel 需要 auto|cloudflare|lhr" >&2; exit 2; }; TUNNEL_MODE="$2"; shift ;;
    --serve-only) SERVE_ONLY=1 ;;
    --no-download) NO_DOWNLOAD=1 ;;
    -h|--help) usage; exit 0 ;;
    --) shift; PASS_ARGS=("$@"); break ;;
    *) PASS_ARGS+=("$1") ;;
  esac
  shift
done

case "${PROTOCOL}" in auto|http2|quic) ;; *) echo "--protocol 只支持 auto|http2|quic" >&2; exit 2 ;; esac
case "${TUNNEL_MODE}" in auto|cloudflare|lhr) ;; *) echo "--tunnel 只支持 auto|cloudflare|lhr" >&2; exit 2 ;; esac

PY=""
for candidate in "${PYTHON:-}" python3 python py; do
  if [[ -n "${candidate}" ]] && command -v "${candidate}" >/dev/null 2>&1 \
    && "${candidate}" -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 8) else 1)' >/dev/null 2>&1; then
    PY="${candidate}"; break
  fi
done
[[ -n "${PY}" ]] || { echo "未找到 python3/python" >&2; exit 2; }

mkdir -p "${OUT_DIR}/bin"
EXE_EXT=""
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) EXE_EXT=".exe" ;; esac

CF="${CLOUDFLARED:-}"
if [[ -z "${CF}" ]]; then
  if [[ -x "${OUT_DIR}/bin/cloudflared${EXE_EXT}" ]]; then
    CF="${OUT_DIR}/bin/cloudflared${EXE_EXT}"
  elif command -v cloudflared >/dev/null 2>&1; then
    CF="$(command -v cloudflared)"
  elif [[ "${NO_DOWNLOAD}" -eq 0 ]]; then
    case "$(uname -s)" in
      MINGW*|MSYS*|CYGWIN*) asset="cloudflared-windows-amd64.exe" ;;
      Linux) asset="cloudflared-linux-amd64" ;;
      Darwin) asset="cloudflared-darwin-amd64.tgz" ;;
      *) echo "未识别的系统，请用 CLOUDFLARED=<路径> 指定 cloudflared" >&2; exit 2 ;;
    esac
    echo "未找到 cloudflared，下载 ${asset} …"
    CF="${OUT_DIR}/bin/cloudflared${EXE_EXT}"
    if [[ "${asset}" == *.tgz ]]; then
      curl -sSL --ssl-no-revoke -o "${OUT_DIR}/bin/cf.tgz" \
        "https://github.com/cloudflare/cloudflared/releases/latest/download/${asset}" \
        && tar -xzf "${OUT_DIR}/bin/cf.tgz" -C "${OUT_DIR}/bin" \
        || { echo "下载 cloudflared 失败" >&2; exit 2; }
      chmod +x "${OUT_DIR}/bin/cloudflared" 2>/dev/null || true
      CF="${OUT_DIR}/bin/cloudflared"
    else
      curl -sSL --ssl-no-revoke -o "${CF}" \
        "https://github.com/cloudflare/cloudflared/releases/latest/download/${asset}" \
        || { echo "下载 cloudflared 失败" >&2; exit 2; }
      chmod +x "${CF}" 2>/dev/null || true
    fi
  else
    echo "未找到 cloudflared（--no-download 已禁用下载）" >&2; exit 2
  fi
fi

if ! "${CF}" --version >/dev/null 2>&1; then
  echo "cloudflared 无法运行：${CF}（下载可能未完成，删掉重试）" >&2
  exit 2
fi

TOKEN="$("${PY}" -c 'import secrets; print(secrets.token_urlsafe(24))')" || {
  echo "生成桥令牌失败" >&2; exit 2;
}

BRIDGE_PID=""
TUNNEL_PID=""
stop_pid() {
  local pid="${1:-}"
  [[ -n "${pid}" ]] || return 0
  kill "${pid}" 2>/dev/null || true
  if command -v powershell >/dev/null 2>&1; then
    powershell -NoProfile -Command "Stop-Process -Id ${pid} -Force -ErrorAction SilentlyContinue" \
      >/dev/null 2>&1 || true
  fi
}
cleanup() {
  stop_pid "${TUNNEL_PID}"
  stop_pid "${BRIDGE_PID}"
  TUNNEL_PID=""; BRIDGE_PID=""
}
trap cleanup EXIT INT TERM

: >"${BRIDGE_LOG}"
: >"${TUNNEL_LOG}"

echo "启动本机检索桥（端口 ${PORT}，必应 RSS）…"
BRIDGE_TOKEN="${TOKEN}" "${PY}" scripts/u07-43/local-bridge.py --port "${PORT}" \
  >>"${BRIDGE_LOG}" 2>&1 &
BRIDGE_PID=$!

BRIDGE_URL="http://127.0.0.1:${PORT}"
for _ in $(seq 1 20); do
  if "${PY}" - "${BRIDGE_URL}/health" <<'PYEOF' >/dev/null 2>&1
import sys, urllib.request
urllib.request.urlopen(sys.argv[1], timeout=2).read()
PYEOF
  then break; fi
  sleep 0.5
done
if ! "${PY}" - "${BRIDGE_URL}/health" <<'PYEOF' >/dev/null 2>&1
import sys, urllib.request
urllib.request.urlopen(sys.argv[1], timeout=2).read()
PYEOF
then
  echo "桥未启动，日志尾部：" >&2
  tail -15 "${BRIDGE_LOG}" >&2 || true
  exit 2
fi

PUBLIC_URL=""

# Cloudflare 对免账号 quick tunnel 有创建频率限制（429 Tunnelstore request failed），
# 所以默认 auto：先试 cloudflared，未就绪就回退 localhost.run 的 SSH 隧道（同样免账号）。
start_tunnel() {
  local mode="$1"
  : >"${TUNNEL_LOG}"
  case "${mode}" in
    cloudflare)
      echo "启动 cloudflared quick tunnel（protocol=${PROTOCOL}）…"
      "${CF}" tunnel --no-autoupdate --protocol "${PROTOCOL}" --url "${BRIDGE_URL}" \
        >"${TUNNEL_LOG}" 2>&1 &
      TUNNEL_PID=$!
      for _ in $(seq 1 30); do
        # quick tunnel 域名形如 swing-jaguar-critical-endif.trycloudflare.com；
        # 日志里还会出现 api.trycloudflare.com（预检目标），必须排除。
        PUBLIC_URL="$(grep -oE 'https://[A-Za-z0-9]+(-[A-Za-z0-9]+)+\.trycloudflare\.com' "${TUNNEL_LOG}" | head -1 || true)"
        [[ -n "${PUBLIC_URL}" ]] && return 0
        if grep -q '429\|Tunnelstore request failed' "${TUNNEL_LOG}"; then return 1; fi
        kill -0 "${TUNNEL_PID}" 2>/dev/null || return 1
        sleep 1
      done
      return 1
      ;;
    lhr)
      command -v ssh >/dev/null 2>&1 || { echo "未找到 ssh，无法用 localhost.run" >&2; return 1; }
      echo "启动 localhost.run SSH 隧道…"
      ssh -T -n -o BatchMode=yes -o StrictHostKeyChecking=no -o ServerAliveInterval=30 \
        -o ExitOnForwardFailure=yes -R "80:127.0.0.1:${PORT}" nokey@localhost.run \
        >>"${TUNNEL_LOG}" 2>&1 &
      TUNNEL_PID=$!
      for _ in $(seq 1 30); do
        PUBLIC_URL="$(grep -oE 'https://[A-Za-z0-9.-]+\.lhr\.life' "${TUNNEL_LOG}" | head -1 || true)"
        [[ -n "${PUBLIC_URL}" ]] && return 0
        kill -0 "${TUNNEL_PID}" 2>/dev/null || return 1
        sleep 1
      done
      return 1
      ;;
    *) return 1 ;;
  esac
}

TUNNEL_OK=0
case "${TUNNEL_MODE}" in
  cloudflare) start_tunnel cloudflare && TUNNEL_OK=1 ;;
  lhr) start_tunnel lhr && TUNNEL_OK=1 ;;
  auto)
    for round in 1 2 3; do
      if start_tunnel cloudflare; then
        TUNNEL_OK=1
        break
      fi
      echo "cloudflared 第 ${round} 次未就绪（常见：quick tunnel 创建 429），30s 后重试。" >&2
      stop_pid "${TUNNEL_PID}"; TUNNEL_PID=""
      sleep 30
    done
    if [[ "${TUNNEL_OK}" -ne 1 ]]; then
      echo "cloudflared 三次均未就绪，回退 localhost.run（免费域名会轮换，仅适合短跑）。" >&2
      if start_tunnel lhr; then TUNNEL_OK=1; fi
    fi
    ;;
esac

if [[ "${TUNNEL_OK}" -ne 1 || -z "${PUBLIC_URL}" ]]; then
  echo "未拿到公网地址，隧道日志尾部：" >&2
  tail -25 "${TUNNEL_LOG}" >&2 || true
  echo "提示：可用 --tunnel lhr 直接走 localhost.run，或稍后重试 cloudflared。" >&2
  exit 2
fi

CURL_OPTS=(-sS -m 25)
if curl --help all 2>/dev/null | grep -q -- '--ssl-no-revoke'; then
  CURL_OPTS+=(--ssl-no-revoke)
fi

echo "公网端点：${PUBLIC_URL}/search"
echo "---- 公网往返自检（真实检索，最多 20 次重试）----"
smoke_body=""
smoke_ok=0
for _ in $(seq 1 20); do
  smoke_body="$(curl "${CURL_OPTS[@]}" -H "Authorization: Bearer ${TOKEN}" \
    "${PUBLIC_URL}/search?q=%E4%BC%9A%E8%AE%AE%E7%BA%AA%E8%A6%81&format=json" 2>/dev/null || true)"
  if printf '%s' "${smoke_body}" | grep -q '"results"'; then
    smoke_ok=1
    break
  fi
  sleep 2
done
if [[ "${smoke_ok}" -ne 1 ]]; then
  echo "公网自检失败，最后一次返回：" >&2
  printf '%s\n' "${smoke_body}" | head -c 400 >&2; echo >&2
  echo "检查 .tmp/u07-43/bridge.log 与 tunnel.log。" >&2
  exit 2
fi
echo "公网自检通过（端点返回 results）。"

cat >"${CONFIG}" <<EOF
# 由 start-bing-bridge.sh 生成；.tmp/ 已 gitignore，勿提交。
SEARCH_ENDPOINT="${PUBLIC_URL}/search"
SEARCH_AUTH_REF="SEARCH_API_TOKEN"
SEARCH_API_TOKEN="${TOKEN}"
EOF
echo "已写入 ${CONFIG}"

if [[ "${SERVE_ONLY}" -eq 1 ]]; then
  echo "serve-only：端点已就绪，保持 3600 秒（进程结束时自动关闭）。"
  echo "另开一个终端执行：bash scripts/run-u07-43-samples.sh"
  sleep 3600
  exit 0
fi

U07_CONFIG_FILE="$(pwd)/${CONFIG}" bash scripts/run-u07-43-samples.sh "${PASS_ARGS[@]}"
rc=$?
echo "---- 采样结束（rc=${rc}），正在关闭桥与隧道 ----"
exit "${rc}"