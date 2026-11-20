#!/usr/bin/env bash
# 从锁文件生成 CycloneDX SBOM。追溯 SPEC 4.5-1 · DESIGN 2.4.2。
# 零外部依赖：仅需 Node。许可证判定由 cargo-deny 与 license-review 负责。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-}"
OUT_FILE="${ROOT}/docs/compliance/sbom/sbom-${VERSION:-ci}.cdx.json"

if [[ -n "${VERSION}" ]]; then
  node "${ROOT}/scripts/gen-sbom.mjs" "${ROOT}" "${VERSION}" "${OUT_FILE}"
else
  node "${ROOT}/scripts/gen-sbom.mjs" "${ROOT}" "" "${OUT_FILE}"
fi