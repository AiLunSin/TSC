#!/usr/bin/env bash
# 用例名清单冻结与差集（AC-B2-OC-E11）。
# 输出归一化为 pkg::target::name，升序；`--diff <before>` 差集非空退出 1。
# 退出码：0=PASS / 1=出现 removed / 2=扫描面缺失。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

CARGO="${CARGO:-cargo}"
NEW_TARGETS='capture_degraded_guard capture_status_push capture_fakes capture_offline_gate capture_contract_freeze'

resolve_pkg() {
  local manifest="$1"
  sed -n 's/^name[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' "${manifest}" | head -n 1
}

list_target() {
  local pkg="$1"
  local target="$2"
  local mode="$3"
  if [[ "${mode}" == "lib" ]]; then
    "${CARGO}" test -p "${pkg}" --lib -- --list 2>/dev/null
  else
    "${CARGO}" test -p "${pkg}" --test "${target}" -- --list 2>/dev/null
  fi | sed -n 's/: test$//p' | sed "s|^|${pkg}::${target}::|"
}

inventory() {
  local exclude_new="${1:-no}"
  local manifests=()
  [[ -f app/src-tauri/Cargo.toml ]] && manifests+=("app/src-tauri/Cargo.toml")
  while IFS= read -r manifest; do
    manifests+=("${manifest}")
  done < <(find crates -maxdepth 2 -name Cargo.toml | sort)

  local manifest pkg target
  for manifest in "${manifests[@]}"; do
    pkg="$(resolve_pkg "${manifest}")"
    [[ -z "${pkg}" ]] && continue
    list_target "${pkg}" lib lib
    local dir
    dir="$(dirname "${manifest}")"
    for file in "${dir}"/tests/*.rs; do
      [[ -e "${file}" ]] || continue
      target="$(basename "${file}" .rs)"
      if [[ "${exclude_new}" == "yes" ]] && echo " ${NEW_TARGETS} " | grep -q " ${target} "; then
        continue
      fi
      list_target "${pkg}" "${target}" test
    done
  done | sort -u
}

case "${1:-}" in
  --emit-baseline)
    OUT="${2:-docs/progress/test-names-before.txt}"
    mkdir -p "$(dirname "${OUT}")"
    inventory yes > "${OUT}"
    lines=$(wc -l < "${OUT}" | tr -d ' ')
    if [[ "${lines}" -lt 1 ]]; then
      echo "${OUT}:0 清单为空" >&2
      exit 2
    fi
    echo "PASS: 基线清单 ${OUT}（${lines} 行）"
    ;;
  --diff)
    BEFORE="${2:-docs/progress/test-names-before.txt}"
    if [[ ! -f "${BEFORE}" ]]; then
      echo "${BEFORE}:0 基线清单缺失" >&2
      exit 2
    fi
    CURRENT="$(mktemp)"
    inventory no > "${CURRENT}"
    REMOVED="$(comm -23 "${BEFORE}" "${CURRENT}")"
    ADDED="$(comm -13 "${BEFORE}" "${CURRENT}")"
    rm -f "${CURRENT}"
    if [[ -n "${REMOVED}" ]]; then
      echo "${BEFORE}:0 出现 removed 用例" >&2
      echo "${REMOVED}" >&2
      exit 1
    fi
    echo "PASS: removed=0，新增 $(printf '%s\n' "${ADDED}" | sed '/^$/d' | wc -l | tr -d ' ') 条"
    ;;
  *)
    inventory no
    ;;
esac