#!/usr/bin/env bash
# 重新生成 docs/assets/work-page.png（README 里的工作页截图）。
#
# 为什么不直接截桌面：转写和卡片都是内存态，程序启动后只有空状态；这里改为用真实前端
# 加一份假 IPC（mock.js）在浏览器里离屏渲染，布局、样式、文案都来自仓库代码。
#
# 关键约束：页面 CSP 是 script-src 'self'，mock 必须是独立文件（本目录的 mock.js），
# 内联 <script> 会被拦掉。Tauri 里也是同一份 CSP，所以这个限制对应用本体同样成立。
#
# 用法：bash scripts/readme-capture/render-work-shot.sh
# 依赖：Node（构建前端）、Python 3（起静态服务）、Edge 或 Chrome。

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${ROOT}/docs/assets/work-page.png"
WORK="${ROOT}/.tmp/readme-shot"
PORT="${PORT:-8131}"
SIZE="${SIZE:-1240,840}"

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
mkdir -p "${WORK}"
cp -r "${ROOT}/app/dist/." "${WORK}/"
cp "${ROOT}/scripts/readme-capture/mock.js" "${WORK}/mock.js"

# 在入口模块之前插入外部 mock；用 python 改 html，避免依赖 sed 的跨平台差异。
python - "${WORK}/index.html" <<'PY'
import re, sys

path = sys.argv[1]
html = open(path, encoding="utf-8").read()
if 'src="/mock.js"' in html:
    sys.exit(0)
html, count = re.subn(r'(<script[^>]*type="module"[^>]*></script>)',
                      r'<script src="/mock.js"></script>\n    \1',
                      html, count=1)
if count != 1:
    raise SystemExit("没找到入口 module 脚本，页面结构可能变了")
open(path, "w", encoding="utf-8", newline="\n").write(html)
PY

echo "== 起静态服务 :${PORT}"
python -m http.server "${PORT}" --bind 127.0.0.1 --directory "${WORK}" >/dev/null 2>&1 &
SERVER_PID=$!
trap 'kill "${SERVER_PID}" 2>/dev/null || true' EXIT

for _ in $(seq 1 40); do
  if curl -sf -o /dev/null "http://127.0.0.1:${PORT}/index.html"; then
    break
  fi
  sleep 0.25
done

echo "== 截图 ${SIZE} @2x"
"${BROWSER}" \
  --headless=new \
  --disable-gpu \
  --hide-scrollbars \
  --force-device-scale-factor=2 \
  --window-size="${SIZE}" \
  --virtual-time-budget=4000 \
  --screenshot="${OUT}" \
  "http://127.0.0.1:${PORT}/index.html" 2>/dev/null

echo "== 完成：${OUT}"
