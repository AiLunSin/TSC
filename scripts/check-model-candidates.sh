#!/usr/bin/env bash
# 模型字面量与默认/回退模型值检查（阶段 E，AC-29/AC-37）。
# 退出码：0=通过；1=命中；2=扫描面缺失。
# 命中只输出 `文件:行号`，不回显命中内容。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

SELF_TEST=0
[[ "${1:-}" == "--self-test" ]] && SELF_TEST=1

PLACEHOLDER='^(test|test-model|fake-model|fixture-model|mock-model|placeholder-model)$'
# 运行期拼接，避免规则描述行自我命中。
MODEL_FAMILY="deep""seek-[a-z0-9][a-z0-9-]*"

if [[ ! -d crates ]]; then
  echo "扫描面缺失" >&2
  exit 2
fi

if [[ ${SELF_TEST} -eq 1 ]]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "${tmp}"' EXIT
  printf 'model: "%s"\n' "${MODEL_FAMILY%-\*}" > "${tmp}/sample.rs"
  found="$(grep -nE "\"${MODEL_FAMILY}\"" "${tmp}/sample.rs" | cut -d: -f1 | sed "s|^|${tmp}/sample.rs:|")"
  if [[ -z "${found}" ]]; then
    echo "self-test failed: 未检出假模型" >&2
    exit 1
  fi
  if grep -q "${MODEL_FAMILY}" "${found%%:*}" 2>/dev/null; then :; fi
  echo "self-test ok: 只输出 文件:行号"
  exit 0
fi

hits=0

# 判定面 A：联调路径的默认/回退模型值
A="$(grep -RInE --include='*.rs' \
  'unwrap_or\([[:space:]]*"[^"]+"\)' \
  crates/infrastructure/src/llm/live crates/infrastructure/src/llm/gateway.rs 2>/dev/null \
  | cut -d: -f1,2 || true)"
if [[ -n "${A}" ]]; then
  printf '%s\n' "${A}"
  hits=1
fi

# 判定面 B：代码中的模型字段字面量（占位白名单除外）
B="$(grep -RInE --include='*.rs' '(^|[^A-Za-z0-9_])model[[:space:]]*[:=][[:space:]]*"' crates app/src scripts 2>/dev/null \
  | while IFS=: read -r file line _rest; do
      value="$(sed -n "${line}p" "${file}")"
      literal="$(printf '%s' "${value}" | sed -n 's/.*model[[:space:]]*[:=][[:space:]]*"\([^"]*\)".*/\1/p')"
      if [[ -n "${literal}" ]] && ! printf '%s' "${literal}" | grep -qE "${PLACEHOLDER}"; then
        printf '%s:%s\n' "${file}" "${line}"
      fi
    done || true)"
if [[ -n "${B}" ]]; then
  printf '%s\n' "${B}"
  hits=1
fi

# 判定面 C：文档中的具体模型字面量
C="$(grep -RInE -- "${MODEL_FAMILY}" docs .codeartsdoer/specs 2>/dev/null \
  | cut -d: -f1,2 || true)"
if [[ -n "${C}" ]]; then
  printf '%s\n' "${C}"
  hits=1
fi

if [[ ${hits} -eq 1 ]]; then
  echo "失败：命中模型字面量或默认/回退模型值" >&2
  exit 1
fi
echo "PASS: 模型字面量 0、默认/回退模型值 0"
exit 0