#!/usr/bin/env bash
# 重建桌面程序，并确保最新的 app/dist 被重新嵌入 exe。
#
# 背景：Tauri 在编译期把前端产物压进二进制（tauri-codegen 读 `frontendDist`）。
# 只跑 `cargo build` 时，增量缓存可能复用上一次的资源和宏展开结果，表现为
# “改了前端、重新编译了，但窗口还是旧版”。先 build 前端、再清掉本 crate 的
# 产物重编，代价约 15 秒，比事后排查划算。
#
# 用法：bash scripts/build-desktop.sh [--release]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PROFILE="dev"
OUT_DIR="debug"
PROFILE_FLAG=()
if [[ "${1:-}" == "--release" ]]; then
  PROFILE="release"
  OUT_DIR="release"
  PROFILE_FLAG=(--release)
fi

echo "== 1/3 生成前端产物 =="
npm --prefix app run build

echo "== 2/3 清掉桌面 crate 旧产物（强制重新嵌入资源）=="
cargo clean -p meeting-copilot-desktop

echo "== 3/3 编译桌面程序（${PROFILE}）=="
cargo build -p meeting-copilot-desktop "${PROFILE_FLAG[@]}" --features llm-https

BIN="target/${OUT_DIR}/meeting-copilot-desktop.exe"
echo "== 完成：${BIN} =="
ls -la "${BIN}"
