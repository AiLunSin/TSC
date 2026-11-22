#!/usr/bin/env bash
# 用例名清单冻结与差集（AC-B2-OC-E11 / AC-CI-01~09、21~23）。
# 输出归一化为 pkg::target::name，升序。
# 退出码：0=通过 / 1=出现 removed / 2=扫描面缺失（含平台不可比）。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}" || {
  printf 'scripts/test-name-inventory.sh:0 扫描面缺失 repo_root\n' >&2
  exit 2
}

SELF="scripts/test-name-inventory.sh"
SELF_ABS="${ROOT}/${SELF}"
BASE_ROOT="${INVENTORY_ROOT:-${ROOT}}"
CARGO="${CARGO:-cargo}"
DEFAULT_BASELINE_PLATFORM="x86_64-pc-windows-msvc"
NEW_TARGETS='capture_degraded_guard capture_status_push capture_fakes capture_offline_gate capture_contract_freeze merge_dedupe understanding_normalize rolling_summary insight_dedup insight_extract understand_ingest insight_corpus merge_corpus gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay'

WORKDIR="$(mktemp -d 2>/dev/null)" || WORKDIR=""
TMP_OUT=""
FAILED_FACES=()
FAILED_COUNT=0

cleanup() {
  if [[ -n "${WORKDIR}" && -d "${WORKDIR}" ]]; then
    rm -rf "${WORKDIR}"
  fi
  if [[ -n "${TMP_OUT}" && -e "${TMP_OUT}" ]]; then
    rm -f "${TMP_OUT}"
  fi
}
trap cleanup EXIT

diag() {
  local line="$1"
  shift
  local msg="${SELF}:${line} $*"
  printf '%s\n' "${msg}"
  printf '%s\n' "${msg}" >&2
}

probe_cargo() {
  local at="$1"
  if [[ "${CARGO}" == */* ]]; then
    if [[ ! -x "${CARGO}" ]]; then
      diag "${at}" "扫描面缺失 cargo_unavailable path=${CARGO}"
      return 2
    fi
  else
    if ! command -v "${CARGO}" >/dev/null 2>&1; then
      diag "${at}" "扫描面缺失 cargo_unavailable command=${CARGO}"
      return 2
    fi
  fi
  if ! "${CARGO}" --version >/dev/null 2>&1; then
    diag "${at}" "扫描面缺失 cargo_unavailable exec=${CARGO}"
    return 2
  fi
  return 0
}

resolve_pkg() {
  local manifest="$1"
  sed -n 's/^name[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' "${manifest}" | head -n 1
}

list_target() {
  local pkg="$1"
  local target="$2"
  local mode="$3"
  local out="$4"
  local at="$5"
  local o e rc line i
  o="$(mktemp "${WORKDIR}/list.out.XXXXXX")" || return 0
  e="$(mktemp "${WORKDIR}/list.err.XXXXXX")" || {
    rm -f "${o}"
    return 0
  }
  if [[ "${mode}" == "lib" ]]; then
    "${CARGO}" test -p "${pkg}" --lib -- --list >"${o}" 2>"${e}"
  else
    "${CARGO}" test -p "${pkg}" --test "${target}" -- --list >"${o}" 2>"${e}"
  fi
  rc=$?
  if [[ "${rc}" -ne 0 ]]; then
    FAILED_FACES+=("${pkg}::${target}::${rc}")
    FAILED_COUNT=$((FAILED_COUNT + 1))
    printf '  face=%s::%s rc=%s\n' "${pkg}" "${target}" "${rc}" >&2
    i=0
    while IFS= read -r line; do
      i=$((i + 1))
      [[ "${i}" -gt 5 ]] && break
      printf '  | %s\n' "${line}" >&2
    done < "${e}"
    rm -f "${o}" "${e}"
    return 0
  fi
  sed -n 's/: test$//p' "${o}" | sed "s|^|${pkg}::${target}::|" >> "${out}"
  rm -f "${o}" "${e}"
  return 0
}

inventory() {
  local exclude_new="$1"
  local out="$2"
  local at="$3"
  local manifests=()
  local manifest pkg dir file target

  if [[ -f "${BASE_ROOT}/app/src-tauri/Cargo.toml" ]]; then
    manifests+=("${BASE_ROOT}/app/src-tauri/Cargo.toml")
  fi
  while IFS= read -r manifest; do
    manifests+=("${manifest}")
  done < <(find "${BASE_ROOT}/crates" -maxdepth 2 -name Cargo.toml 2>/dev/null | sort)

  if [[ "${#manifests[@]}" -eq 0 ]]; then
    diag "${at}" "扫描面缺失 manifests=0 面=app/src-tauri,crates root=${BASE_ROOT}"
    return 2
  fi

  : > "${out}"
  for manifest in "${manifests[@]}"; do
    pkg="$(resolve_pkg "${manifest}")"
    [[ -z "${pkg}" ]] && continue
    dir="$(dirname "${manifest}")"
    if [[ -f "${dir}/src/lib.rs" ]] || grep -qE '^\[lib\]' "${manifest}" 2>/dev/null; then
      list_target "${pkg}" lib lib "${out}" "${at}"
    fi
    for file in "${dir}"/tests/*.rs; do
      [[ -e "${file}" ]] || continue
      target="$(basename "${file}" .rs)"
      if [[ "${exclude_new}" == "yes" ]] && echo " ${NEW_TARGETS} " | grep -q " ${target} "; then
        continue
      fi
      list_target "${pkg}" "${target}" test "${out}" "${at}"
    done
  done

  LC_ALL=C sort -u -o "${out}" "${out}"
  return 0
}

report_failures() {
  local at="$1"
  local idx face rc
  if [[ "${FAILED_COUNT}" -gt 0 ]]; then
    face="${FAILED_FACES[0]%::*}"
    rc="${FAILED_FACES[0]##*::}"
    diag "${at}" "扫描面缺失 face=${face} rc=${rc} failed=${FAILED_COUNT}"
    idx=1
    while [[ "${idx}" -lt "${#FAILED_FACES[@]}" ]]; do
      printf '  face=%s\n' "${FAILED_FACES[${idx}]}" >&2
      idx=$((idx + 1))
    done
    return 2
  fi
  return 0
}

resolve_exec_platform() {
  if [[ -n "${INVENTORY_PLATFORM:-}" ]]; then
    printf '%s\n' "${INVENTORY_PLATFORM}"
    return 0
  fi
  local host
  host="$("${CARGO}" -vV 2>/dev/null | sed -n 's/^host: //p' | head -n 1)"
  if [[ -n "${host}" ]]; then
    printf '%s\n' "${host}"
    return 0
  fi
  local arch os
  arch="$(uname -m 2>/dev/null)"
  os="$(uname -s 2>/dev/null)"
  case "${arch}:${os}" in
    x86_64:MINGW*|x86_64:MSYS*|x86_64:CYGWIN*) printf '%s\n' "x86_64-pc-windows-msvc" ;;
    x86_64:Darwin) printf '%s\n' "x86_64-apple-darwin" ;;
    arm64:Darwin|aarch64:Darwin) printf '%s\n' "aarch64-apple-darwin" ;;
    x86_64:Linux) printf '%s\n' "x86_64-unknown-linux-gnu" ;;
    *) return 1 ;;
  esac
  return 0
}

BASELINE_NORM=""

load_baseline() {
  local before="$1"
  local at="$2"
  local norm
  if [[ ! -f "${before}" ]]; then
    diag "${at}" "扫描面缺失 baseline_missing path=${before}"
    return 2
  fi
  if [[ ! -r "${before}" ]]; then
    diag "${at}" "扫描面缺失 baseline_unreadable path=${before}"
    return 2
  fi
  if [[ -z "${WORKDIR}" || ! -d "${WORKDIR}" ]]; then
    diag "${at}" "扫描面缺失 workdir_unavailable"
    return 2
  fi
  norm="${WORKDIR}/baseline.norm"
  grep -v '^[[:space:]]*$' "${before}" > "${norm}" 2>/dev/null
  if [[ ! -s "${norm}" ]]; then
    diag "${at}" "扫描面缺失 baseline_missing path=${before} reason=empty"
    return 2
  fi
  if ! LC_ALL=C sort -c "${norm}" 2>/dev/null; then
    diag "${at}" "扫描面缺失 baseline_unsorted path=${before}"
    return 2
  fi
  BASELINE_NORM="${norm}"
  return 0
}

mk_stub() {
  local path="$1"
  shift
  printf '#!/usr/bin/env bash\n' > "${path}"
  printf '%s\n' "$@" >> "${path}"
  chmod +x "${path}" 2>/dev/null || true
}

run_self_test() {
  local work rc out before after
  work="${WORKDIR}/selftest"
  mkdir -p "${work}" || {
    printf 'SELF-TEST: failed case=T0\n'
    return 1
  }
  before=""
  after=""

  mk_stub "${work}/cargo127" 'exit 127'
  mk_stub "${work}/cargo0" \
    'if [[ "${1:-}" == "--version" ]]; then echo "cargo 0.0.0-stub"; exit 0; fi' \
    'exit 0'
  mk_stub "${work}/cargo_t2" \
    'if [[ "${1:-}" == "--version" ]]; then echo "cargo 0.0.0-stub"; exit 0; fi' \
    'for a in "$@"; do' \
    '  if [[ "${a}" == "--lib" ]]; then echo "lib_case: test"; exit 0; fi' \
    '  if [[ "${a}" == "--test" ]]; then echo "stub test target failure" >&2; exit 1; fi' \
    'done' \
    'exit 0'
  mk_stub "${work}/cargo_fake" \
    'if [[ "${1:-}" == "--version" ]]; then echo "cargo 0.0.0-stub"; exit 0; fi' \
    'for a in "$@"; do' \
    '  case "${a}" in' \
    '    --lib|--test) echo "fake_case: test"; exit 0;;' \
    '  esac' \
    'done' \
    'exit 0'

  # T1 工具链不可用
  out="${work}/t1.out"
  CARGO="${work}/cargo127" bash "${SELF_ABS}" --diff docs/progress/test-names-before.txt >"${out}" 2>&1
  rc=$?
  if [[ "${rc}" -ne 2 ]] \
    || ! head -1 "${out}" | grep -qE '^.+:[0-9]+(:| )' \
    || ! grep -q '扫描面缺失' "${out}" \
    || [[ "$(grep -c 'removed=' "${out}")" -ne 0 ]]; then
    printf 'SELF-TEST: failed case=T1\n'
    printf 'SELF-TEST: failed case=T1\n' >&2
    return 1
  fi
  printf 'T1 ok\n'

  # T2 单面失败
  out="${work}/t2.out"
  CARGO="${work}/cargo_t2" bash "${SELF_ABS}" --diff docs/progress/test-names-before.txt >"${out}" 2>&1
  rc=$?
  if [[ "${rc}" -ne 2 ]] \
    || ! grep -qE 'face=.+::' "${out}" \
    || [[ "$(grep -c 'removed=' "${out}")" -ne 0 ]]; then
    printf 'SELF-TEST: failed case=T2\n'
    printf 'SELF-TEST: failed case=T2\n' >&2
    return 1
  fi
  printf 'T2 ok\n'

  # T3 空清单
  out="${work}/t3.out"
  CARGO="${work}/cargo0" bash "${SELF_ABS}" --diff docs/progress/test-names-before.txt >"${out}" 2>&1
  rc=$?
  if [[ "${rc}" -ne 2 ]] \
    || ! grep -q '扫描面缺失' "${out}" \
    || [[ "$(grep -c 'removed=' "${out}")" -ne 0 ]]; then
    printf 'SELF-TEST: failed case=T3\n'
    printf 'SELF-TEST: failed case=T3\n' >&2
    return 1
  fi
  printf 'T3 ok\n'

  # T4 仓库扫描面缺失
  mkdir -p "${work}/empty"
  out="${work}/t4.out"
  INVENTORY_ROOT="${work}/empty" CARGO="${work}/cargo0" bash "${SELF_ABS}" --emit-baseline "${work}/t4-base.txt" >"${out}" 2>&1
  rc=$?
  if [[ "${rc}" -ne 2 ]] \
    || ! grep -q 'app/src-tauri' "${out}" \
    || ! grep -q 'crates' "${out}" \
    || [[ -e "${work}/t4-base.txt" ]]; then
    printf 'SELF-TEST: failed case=T4\n'
    printf 'SELF-TEST: failed case=T4\n' >&2
    return 1
  fi
  printf 'T4 ok\n'

  # T5 原子写
  printf 'keep\n' > "${work}/exists.txt"
  before="$(sha256sum "${work}/exists.txt" | cut -d' ' -f1)"
  CARGO="${work}/cargo127" bash "${SELF_ABS}" --emit-baseline "${work}/exists.txt" >"${work}/t5a.out" 2>&1
  rc=$?
  after="$(sha256sum "${work}/exists.txt" | cut -d' ' -f1)"
  CARGO="${work}/cargo127" bash "${SELF_ABS}" --emit-baseline "${work}/new.txt" >"${work}/t5b.out" 2>&1
  local rc2=$?
  if [[ "${rc}" -ne 2 || "${rc2}" -ne 2 || "${before}" != "${after}" || -e "${work}/new.txt" ]]; then
    printf 'SELF-TEST: failed case=T5\n'
    printf 'SELF-TEST: failed case=T5\n' >&2
    return 1
  fi
  printf 'T5 ok\n'

  # T6 正常态
  out="${work}/t6a.out"
  CARGO="${work}/cargo_fake" bash "${SELF_ABS}" --emit-baseline "${work}/base.txt" >"${out}" 2>&1
  rc=$?
  if [[ "${rc}" -ne 0 ]]; then
    printf 'SELF-TEST: failed case=T6\n'
    printf 'SELF-TEST: failed case=T6\n' >&2
    return 1
  fi
  out="${work}/t6b.out"
  CARGO="${work}/cargo_fake" bash "${SELF_ABS}" --diff "${work}/base.txt" >"${out}" 2>&1
  rc=$?
  if [[ "${rc}" -ne 0 ]] || ! grep -q 'removed=0' "${out}"; then
    printf 'SELF-TEST: failed case=T6\n'
    printf 'SELF-TEST: failed case=T6\n' >&2
    return 1
  fi
  printf 'T6 ok\n'

  printf 'SELF-TEST: 6/6 ok\n'
  return 0
}

case "${1:-}" in
  --emit-baseline)
    OUT="${2:-docs/progress/test-names-before.txt}"
    probe_cargo "${LINENO}" || exit 2
    mkdir -p "$(dirname "${OUT}")" 2>/dev/null || {
      diag "${LINENO}" "扫描面缺失 output_dir_unavailable path=${OUT}"
      exit 2
    }
    TMP_OUT="$(mktemp "${OUT}.tmp.XXXXXX" 2>/dev/null)" || {
      TMP_OUT=""
      diag "${LINENO}" "扫描面缺失 output_tmp_unavailable path=${OUT}"
      exit 2
    }
    inventory yes "${TMP_OUT}" "${LINENO}" || exit 2
    report_failures "${LINENO}" || exit 2
    lines="$(grep -c . "${TMP_OUT}" 2>/dev/null || true)"
    [[ -z "${lines}" ]] && lines=0
    if [[ "${lines}" -lt 1 ]]; then
      diag "${LINENO}" "扫描面缺失 inventory_empty path=${OUT}"
      exit 2
    fi
    mv -f "${TMP_OUT}" "${OUT}" || {
      diag "${LINENO}" "扫描面缺失 output_move_failed path=${OUT}"
      exit 2
    }
    TMP_OUT=""
    printf 'PASS: 基线清单 %s（%s 行） baseline_platform=%s\n' \
      "${OUT}" "${lines}" "${INVENTORY_BASELINE_PLATFORM:-${DEFAULT_BASELINE_PLATFORM}}"
    ;;
  --diff)
    BEFORE="${2:-docs/progress/test-names-before.txt}"
    probe_cargo "${LINENO}" || exit 2
    load_baseline "${BEFORE}" "${LINENO}" || exit 2
    exec_platform="$(resolve_exec_platform)" || {
      diag "${LINENO}" "扫描面缺失 platform_unresolved"
      exit 2
    }
    baseline_platform="${INVENTORY_BASELINE_PLATFORM:-${DEFAULT_BASELINE_PLATFORM}}"
    if [[ "${exec_platform}" != "${baseline_platform}" ]]; then
      diag "${LINENO}" "不可比 exec=${exec_platform} baseline=${baseline_platform}"
      exit 2
    fi
    CURRENT="${WORKDIR}/current.list"
    inventory no "${CURRENT}" "${LINENO}" || exit 2
    report_failures "${LINENO}" || exit 2
    if [[ ! -s "${CURRENT}" ]]; then
      diag "${LINENO}" "扫描面缺失 inventory_empty"
      exit 2
    fi
    LC_ALL=C sort -u -o "${BASELINE_NORM}" "${BASELINE_NORM}"
    LC_ALL=C sort -u -o "${CURRENT}" "${CURRENT}"
    removed="$(LC_ALL=C comm -23 "${BASELINE_NORM}" "${CURRENT}")"
    added="$(LC_ALL=C comm -13 "${BASELINE_NORM}" "${CURRENT}")"
    if [[ -n "${removed}" ]]; then
      printf '%s:0 出现 removed 用例\n' "${BEFORE}"
      printf '%s:0 出现 removed 用例\n' "${BEFORE}" >&2
      printf '%s\n' "${removed}" >&2
      exit 1
    fi
    added_lines="$(printf '%s\n' "${added}" | sed '/^$/d' | wc -l | tr -d ' ')"
    printf 'PASS: removed=0，新增 %s 条 baseline_drift=%s\n' "${added_lines}" "${added_lines}"
    ;;
  --self-test)
    run_self_test
    exit $?
    ;;
  "")
    probe_cargo "${LINENO}" || exit 2
    CURRENT="${WORKDIR}/current.list"
    inventory no "${CURRENT}" "${LINENO}" || exit 2
    report_failures "${LINENO}" || exit 2
    if [[ ! -s "${CURRENT}" ]]; then
      diag "${LINENO}" "扫描面缺失 inventory_empty"
      exit 2
    fi
    cat "${CURRENT}"
    ;;
  *)
    diag "${LINENO}" "未知参数 ${1}（扫描面缺失 usage_error）"
    exit 2
    ;;
esac