#!/usr/bin/env bash
# 生成 CycloneDX SBOM（Rust + npm 两侧）。追溯 SPEC 4.5-1 · DESIGN 2.4.2。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-$(node -p "require('${ROOT}/app/package.json').version")}"
OUT_DIR="${ROOT}/docs/compliance/sbom"
OUT_FILE="${OUT_DIR}/sbom-${VERSION}.cdx.json"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

mkdir -p "${OUT_DIR}"

if ! command -v cyclonedx-npm >/dev/null 2>&1; then
  echo "缺少 cyclonedx-npm，请先执行: npm i -g @cyclonedx/cyclonedx-npm" >&2
  exit 1
fi

echo "[1/3] 生成 npm SBOM"
(cd "${ROOT}/app" && cyclonedx-npm --output-file "${TMP_DIR}/npm.cdx.json" --output-format JSON)

echo "[2/3] 生成 Rust SBOM"
if command -v cargo-cyclonedx >/dev/null 2>&1; then
  (cd "${ROOT}" && cargo cyclonedx --format json --output-cdx)
  find "${ROOT}/crates" "${ROOT}/app/src-tauri" -maxdepth 2 -name '*.cdx.json' -exec cp {} "${TMP_DIR}/" \; 2>/dev/null || true
else
  echo "警告: 未安装 cargo-cyclonedx，Rust 侧 SBOM 留空并标记 incomplete" >&2
  printf '{"bomFormat":"CycloneDX","specVersion":"1.5","tools":[{"name":"cargo-cyclonedx","version":"missing"}]}' > "${TMP_DIR}/rust.cdx.json"
fi

echo "[3/3] 合并并写出 ${OUT_FILE}"
node "${ROOT}/scripts/merge-sbom.mjs" "${TMP_DIR}" "${OUT_FILE}"

if command -v cyclonedx-cli >/dev/null 2>&1; then
  cyclonedx-cli validate --input-file "${OUT_FILE}" --input-format json
else
  echo "警告: 未安装 cyclonedx-cli，跳过 schema 校验" >&2
fi

echo "SBOM: ${OUT_FILE}"