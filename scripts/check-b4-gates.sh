#!/usr/bin/env bash
# B4 门控静态门禁：七条规则 + 自检。全部不依赖 cargo。
# 退出码：0=通过 / 1=命中违规或自检命中 / 2=必需扫描面缺失或参数非法。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

RULE=""
SELF_TEST=0
FAIL=0
MISSING=0

GATE_DIR="crates/application/src/gate"
GATE_MODEL="crates/domain/src/model/gate.rs"
GATE_REPLAY="crates/domain/src/model/gate_replay.rs"
STATUS_FILE="docs/progress/b4-status.md"
SPEC_FILE=".codeartsdoer/specs/b4_gate/spec.md"
RECORD_WHITELIST="schema_version trace_id session_id candidate_ref insight_ref trigger per_stage decision stopped_at budget_remaining cooldown_until_ms authorization_status skipped_stages decided_at_ms"

usage() {
  cat <<'EOF'
用法：check-b4-gates.sh --rule=<name> | --self-test
规则：source | budget | record | outbound | zero-transcript | targets | doc-plan
退出码：0=通过 / 1=命中违规 / 2=必需扫描面缺失或参数非法
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rule=*) RULE="${1#--rule=}" ;;
    --self-test) SELF_TEST=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "未知参数 $1" >&2; exit 2 ;;
  esac
  shift
done

fail() { echo "$1" >&2; FAIL=1; }
missing() { echo "SCAN-MISSING: $1" >&2; MISSING=1; }
absent() { echo "SCAN-ABSENT: $1"; }

need_dir() {
  if [[ ! -d "$1" ]]; then
    missing "$1"
    return 1
  fi
  return 0
}

need_file() {
  if [[ ! -f "$1" ]]; then
    missing "$1"
    return 1
  fi
  return 0
}

strip_tests() {
  awk '/^#\[cfg\(test\)\]/{exit} {print}' "$1" 2>/dev/null
}

gate_files() {
  local file
  for file in "${GATE_DIR}"/*.rs; do
    [[ -f "${file}" ]] && printf '%s\n' "${file}"
  done
}

scan_files() {
  local pattern="$1"
  shift
  local total=0
  local file
  local hits
  for file in "$@"; do
    [[ -f "${file}" ]] || continue
    hits="$(strip_tests "${file}" | grep -cE "${pattern}" || true)"
    total=$((total + hits))
  done
  printf '%s' "${total}"
}

require_core_faces() {
  local ok=0
  need_dir "${GATE_DIR}" || ok=1
  need_file "${GATE_MODEL}" || ok=1
  need_file "${GATE_REPLAY}" || ok=1
  return "${ok}"
}

core_files() {
  printf '%s\n' "${GATE_MODEL}" "${GATE_REPLAY}"
  gate_files
}

rule_source() {
  require_core_faces || return
  if [[ ! -d crates/domain/src/gap ]]; then
    absent "crates/domain/src/gap"
  fi

  local files=()
  local file
  while IFS= read -r file; do
    files+=("${file}")
  done < <(core_files)

  local hits
  hits="$(scan_files 'reqwest::|hyper::|TcpStream|std::net|Instant::now|SystemTime::now|thread::sleep|tokio::time|std::env|env::var|LLM_|todo!\(|unimplemented!\(|panic!\(|static mut|OnceLock|lazy_static|thread_local' "${files[@]}")"
  if [[ "${hits}" -ne 0 ]]; then
    fail "source: 门控源码命中网络/计时/睡眠/环境变量/占位 panic/全局态（${hits} 处）"
  fi

  local gate_list=()
  while IFS= read -r file; do
    gate_list+=("${file}")
  done < <(gate_files)
  hits="$(scan_files 'Mutex|RwLock|AtomicU64' "${gate_list[@]}")"
  if [[ "${hits}" -ne 0 ]]; then
    fail "source: gate/ 命中锁或原子计数（${hits} 处）"
  fi
}

rule_budget() {
  require_core_faces || return
  local files=()
  local file
  while IFS= read -r file; do
    files+=("${file}")
  done < <(core_files)

  local hits
  hits="$(scan_files 'LLM_LIVE_CALL_CAP|LLM_|std::env' "${files[@]}")"
  if [[ "${hits}" -ne 0 ]]; then
    fail "budget: 门控源码命中 LLM 预算或环境变量（${hits} 处）"
  fi

  local field
  for field in session_outbound_used global_window_used topic_window_used entity_last_seen_ms inflight tokens_milli window_start_ms; do
    if ! grep -q "pub ${field}:" "${GATE_MODEL}"; then
      fail "budget: GateBudget 缺字段 ${field}"
    fi
  done
}

record_block() {
  awk '/^pub struct GateRecord \{/{flag=1; next} flag && /^\}/{exit} flag' "${GATE_MODEL}"
}

rule_record() {
  require_core_faces || return

  local actual
  actual="$(record_block | grep -oE '^    pub [a-z_]+:' | sed -E 's/^    pub //; s/:$//' | grep -v '^unknown$' | sort)"
  local expected
  expected="$(printf '%s\n' ${RECORD_WHITELIST} | sort)"
  if [[ "${actual}" != "${expected}" ]]; then
    fail "record: GateRecord 字段集合不等于 14 项白名单"
    diff -u <(printf '%s\n' "${expected}") <(printf '%s\n' "${actual}") >&2 || true
  fi

  local variants
  variants="$(awk '/^pub enum GateRejectReason \{/{flag=1; next} flag && /^\}/{exit} flag' "${GATE_MODEL}" | grep -cE '^    [A-Z][A-Za-z0-9]*,$' || true)"
  if [[ "${variants}" -ne 23 ]]; then
    fail "record: GateRejectReason 变体数 ${variants} != 23"
  fi

  local skip_stages
  skip_stages="$(awk '/fn skipped_stages_are_valid/{flag=1} flag{print; if (/^    \}$/) exit}' "${GATE_MODEL}" | grep -oE 'GateStage::G[0-9]' | sort -u | tr '\n' ' ')"
  if [[ "${skip_stages}" != "GateStage::G2 GateStage::G3 " ]]; then
    fail "record: skipped_stages 白名单不是仅 G2/G3（实际：${skip_stages}）"
  fi
}

rule_outbound() {
  require_core_faces || return
  if [[ ! -d crates/application/src/proactive ]]; then
    absent "crates/application/src/proactive"
  fi

  local files=()
  local file
  while IFS= read -r file; do
    files+=("${file}")
  done < <(gate_files)

  local hits
  hits="$(scan_files 'reqwest::|hyper::|TcpStream|std::net' "${files[@]}")"
  if [[ "${hits}" -ne 0 ]]; then
    fail "outbound: gate/ 命中协议客户端调用（${hits} 处）"
  fi

  local issuers
  issuers="$(grep -rlE 'issue_query|AuditLog::append' "${GATE_DIR}" 2>/dev/null | wc -l | tr -d ' ')"
  if [[ "${issuers}" -gt 1 ]]; then
    fail "outbound: issue_query/AuditLog::append 出现在 ${issuers} 个文件（上限 1）"
  fi
}

rule_zero_transcript() {
  need_dir "${GATE_DIR}" || return
  if ! compgen -G "crates/application/tests/gate_*.rs" > /dev/null; then
    absent "crates/application/tests/gate_*.rs"
  fi
  if [[ ! -f "${STATUS_FILE}" ]]; then
    absent "${STATUS_FILE}"
  fi

  local files=()
  local file
  while IFS= read -r file; do
    files+=("${file}")
  done < <(gate_files)
  if compgen -G "crates/application/tests/gate_*.rs" > /dev/null; then
    while IFS= read -r file; do
      files+=("${file}")
    done < <(printf '%s\n' crates/application/tests/gate_*.rs)
  fi
  if [[ -f "${STATUS_FILE}" ]]; then
    files+=("${STATUS_FILE}")
  fi

  local hits
  hits="$(scan_files 'https?://|sk-[A-Za-z0-9]{8,}|Authorization:[[:space:]]*Bearer|Bearer [A-Za-z0-9._-]{8,}|(deepseek|gpt-4|gpt-3|claude)-[A-Za-z0-9._-]+|request_id[[:space:]]*[:=]' "${files[@]}")"
  if [[ "${hits}" -ne 0 ]]; then
    fail "zero-transcript: 命中 URL/Key/鉴权头/模型名/请求标识（${hits} 处）"
  fi
}

rule_targets() {
  local inventory="scripts/test-name-inventory.sh"
  need_file "${inventory}" || return

  local names
  names="$(grep -m1 '^NEW_TARGETS=' "${inventory}" | sed -E "s/^NEW_TARGETS='//; s/'$//" | tr ' ' '\n')"
  local target
  for target in gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay; do
    if ! printf '%s\n' "${names}" | grep -qx "${target}"; then
      echo "TARGETS-MISSING: ${target}"
      FAIL=1
    fi
  done
}

rule_doc_plan() {
  need_file "${SPEC_FILE}" || return

  local rows
  rows="$(sed -n '/^# \*\*8\. 文档变更清单/,/^# \*\*9\./p' "${SPEC_FILE}" | grep -E '^\|' | grep -vE '^\|[[:space:]]*文件|^\|[[:space:]]*-+' || true)"
  if [[ -z "${rows}" ]]; then
    fail "doc-plan: 第 8 章表格无数据行"
    return
  fi

  local line path action
  while IFS= read -r line; do
    [[ -z "${line}" ]] && continue
    path="$(printf '%s' "${line}" | awk -F'|' '{print $2}' | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//; s/^`//; s/`$//')"
    action="$(printf '%s' "${line}" | awk -F'|' '{print $4}')"
    if [[ "${action}" == *"新增"* ]]; then
      continue
    fi
    if [[ ! -e "${path}" ]]; then
      fail "doc-plan: 路径不存在且动作非新增：${path}"
    fi
  done <<< "${rows}"
}

self_test_fail() {
  FAIL=1
}

rule_self_test() {
  local before_dirty
  before_dirty="$(git status --porcelain | wc -l | tr -d ' ')"

  local tmp
  tmp="$(mktemp -d)"
  trap "rm -rf '${tmp}'" EXIT

  printf 'fn f(){ let _c = reqwest::Client::new(); }\n' > "${tmp}/network.rs"
  printf 'fn f(){ let _c = LLM_LIVE_CALL_CAP; }\n' > "${tmp}/llm.rs"
  printf 'pub struct GateRecord {\n    pub schema_version: u16,\n}\n' > "${tmp}/record.rs"
  printf 'fn f(stage: GateStage) -> bool { matches!(stage, GateStage::G4) }\n' > "${tmp}/skip.rs"

  local detected=0
  local hits
  hits="$(grep -cE 'reqwest::|hyper::|TcpStream|std::net' "${tmp}/network.rs" || true)"
  [[ "${hits}" -gt 0 ]] && detected=$((detected + 1))
  hits="$(grep -cE 'LLM_LIVE_CALL_CAP|LLM_' "${tmp}/llm.rs" || true)"
  [[ "${hits}" -gt 0 ]] && detected=$((detected + 1))

  local actual
  actual="$(awk '/^pub struct GateRecord \{/{flag=1; next} flag && /^\}/{exit} flag' "${tmp}/record.rs" | grep -oE '^    pub [a-z_]+:' | sed -E 's/^    pub //; s/:$//' | sort)"
  local expected
  expected="$(printf '%s\n' ${RECORD_WHITELIST} | sort)"
  [[ "${actual}" != "${expected}" ]] && detected=$((detected + 1))

  hits="$(grep -oE 'GateStage::G[0-9]' "${tmp}/skip.rs" | sort -u | tr '\n' ' ')"
  [[ "${hits}" != "GateStage::G2 GateStage::G3 " ]] && detected=$((detected + 1))

  if [[ "${detected}" -lt 1 ]]; then
    self_test_fail
  fi

  local after_dirty
  after_dirty="$(git status --porcelain | wc -l | tr -d ' ')"
  if [[ "${after_dirty}" -ne "${before_dirty}" ]]; then
    echo "SELF-TEST-DIRTY: 工作区由 ${before_dirty} 行变为 ${after_dirty} 行" >&2
  fi

  echo "SELF-TEST-WORKTREE: before=${before_dirty} after=${after_dirty}"
  echo "SELF-TEST-DETECTED: ${detected}"
}

if [[ "${SELF_TEST}" -eq 1 ]]; then
  rule_self_test
  exit 1
fi

if [[ -z "${RULE}" ]]; then
  usage >&2
  exit 2
fi

case "${RULE}" in
  source) rule_source ;;
  budget) rule_budget ;;
  record) rule_record ;;
  outbound) rule_outbound ;;
  zero-transcript) rule_zero_transcript ;;
  targets) rule_targets ;;
  doc-plan) rule_doc_plan ;;
  *) echo "未知规则 ${RULE}" >&2; usage >&2; exit 2 ;;
esac

if [[ "${MISSING}" -ne 0 ]]; then
  exit 2
fi
if [[ "${FAIL}" -ne 0 ]]; then
  exit 1
fi
exit 0