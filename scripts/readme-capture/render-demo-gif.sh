#!/usr/bin/env bash
# 重新生成 docs/assets/demo.gif（README 首屏动图：选中 → 查意思 → 生成卡片）。
#
# 用法：bash scripts/readme-capture/render-demo-gif.sh
# 依赖：Node（构建前端）、Python 3 + Pillow、Edge 或 Chrome。
#
# 原理与 render-work-shot.sh 相同（真实前端 + 假 IPC 离屏渲染），区别是这里按不同的
# 虚拟时间点各截一帧，再拼成 GIF。脚本 mock.js / demo.js 都是外部文件，
# 不能内联——页面 CSP 是 script-src 'self'。

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${ROOT}/docs/assets/demo.gif"
WORK="${ROOT}/.tmp/readme-demo"
PORT="${PORT:-8132}"
SIZE="${SIZE:-1240,840}"
# 虚拟时间毫秒：初始 / 选中 / 释义 / 出卡 / 停住
BUDGETS="${BUDGETS:-700 1600 2600 3900 5200}"
DURATIONS="${DURATIONS:-1100,1000,1100,1300,2200}"

find_browser() {
  local candidates=(
    "/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
    "/c/Program Files/Microsoft/Edge/Application/msedge.exe"
    "/c/Program Files/Google/Chrome/Application/chrome.exe"
    "/c/Program Files (x86)/Google/Chrome/Application/chrome.exe"
  )
  local path
  for path in "${candidates[@]}"; do
    if [[ -x "${path}" ]]; then
      echo "${path}"
      return 0
    fi
  done
  return 1
}

BROWSER="$(find_browser)" || {
  echo "找不到 Edge/Chrome，无法离屏渲染" >&2
  exit 1
}

echo "== 构建前端"
npm --prefix "${ROOT}/app" run build

echo "== 准备离屏页面"
rm -rf "${WORK}"
mkdir -p "${WORK}/frames"
cp -r "${ROOT}/app/dist/." "${WORK}/"
cp "${ROOT}/scripts/readme-capture/mock.js" "${WORK}/mock.js"
cp "${ROOT}/scripts/readme-capture/demo.js" "${WORK}/demo.js"

python - "${WORK}/index.html" <<'PY'
import re
import sys

path = sys.argv[1]
html = open(path, encoding="utf-8").read()
if 'src="/mock.js"' not in html:
    html, count = re.subn(
        r'(<script[^>]*type="module"[^>]*></script>)',
        r'<script src="/mock.js"></script>\n    <script src="/demo.js"></script>\n    \1',
        html,
        count=1,
    )
    if count != 1:
        raise SystemExit("没找到入口 module 脚本，页面结构可能变了")
    open(path, "w", encoding="utf-8", newline="\n").write(html)
PY

echo "== 起静态服务 :${PORT}"
python -m http.server "${PORT}" --bind 127.0.0.1 --directory "${WORK}" >/dev/null 2>&1 &
SERVER_PID=$!
trap 'kill "${SERVER_PID}" 2>/dev/null || true' EXIT

for _ in $(seq 1 40); do
  if curl -sf -o /dev/null "http://127.0.0.1:${PORT}/index.html?demo=1"; then
    break
  fi
  sleep 0.25
done

index=0
for budget in ${BUDGETS}; do
  index=$((index + 1))
  frame="$(printf '%s/frames/%02d.png' "${WORK}" "${index}")"
  echo "== 截帧 ${index} @ ${budget}ms"
  "${BROWSER}" \
    --headless=new \
    --disable-gpu \
    --hide-scrollbars \
    --window-size="${SIZE}" \
    --virtual-time-budget="${budget}" \
    --screenshot="${frame}" \
    "http://127.0.0.1:${PORT}/index.html?demo=1" 2>/dev/null
done

echo "== 合成 GIF"
python "${ROOT}/scripts/readme-capture/make-gif.py" "${OUT}" "${WORK}"/frames/*.png "${DURATIONS}"

echo "== 完成：${OUT}"
