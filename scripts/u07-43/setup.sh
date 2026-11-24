#!/usr/bin/env bash
# 生成 .tmp/u07-43/config.env：A/B/C 三条路径都从这里落地，之后直接跑主脚本。
#
#   # A. 自建 SearXNG（无需凭据）
#   bash scripts/u07-43/setup.sh --mode a --endpoint https://<域名>/search
#
#   # B. 转发 Worker（deploy-worker.sh 也可自动生成，通常不用手填）
#   bash scripts/u07-43/setup.sh --mode b --endpoint https://<worker>/search --token '<SHARED_TOKEN>'
#
#   # C. 自定义 providers.json（模板：scripts/u07-43/providers.example.json）
#   bash scripts/u07-43/setup.sh --mode c --providers <绝对路径> --token '<凭据>'
#
# 生成后自动跑一次离线预检。凭据只写进被 .gitignore 忽略的本地配置，不回显。
#
# 选项：
#   --mode a|b|c          默认 b
#   --endpoint URL        A/B 的 HTTPS 检索端点
#   --token VALUE         写入 authRef 指向的环境变量
#   --auth-ref NAME       B 模式的变量名，默认 SEARCH_API_TOKEN
#   --providers FILE      C 模式的 providers.json（会转成绝对路径）
#   --env NAME=VALUE      额外写入配置（C 模式有多个 authRef 时可重复）
#   --out FILE            配置输出路径，默认 .tmp/u07-43/config.env
#   --no-check            生成后不跑预检
#   -h | --help           显示本说明
# 退出码：0=成功 / 2=参数或预检错误。

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

MODE="b"
ENDPOINT=""
TOKEN=""
AUTH_REF="SEARCH_API_TOKEN"
PROVIDERS=""
OUT="${U07_CONFIG_FILE:-${ROOT}/.tmp/u07-43/config.env}"
NO_CHECK=0
EXTRA_ENVS=()

usage() { sed -n '2,30p' "${BASH_SOURCE[0]}"; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode) [[ $# -ge 2 ]] || { echo "--mode 需要 a|b|c" >&2; exit 2; }; MODE="$2"; shift ;;
    --endpoint) [[ $# -ge 2 ]] || { echo "--endpoint 需要 URL" >&2; exit 2; }; ENDPOINT="$2"; shift ;;
    --token) [[ $# -ge 2 ]] || { echo "--token 需要值" >&2; exit 2; }; TOKEN="$2"; shift ;;
    --auth-ref) [[ $# -ge 2 ]] || { echo "--auth-ref 需要变量名" >&2; exit 2; }; AUTH_REF="$2"; shift ;;
    --providers) [[ $# -ge 2 ]] || { echo "--providers 需要路径" >&2; exit 2; }; PROVIDERS="$2"; shift ;;
    --out) [[ $# -ge 2 ]] || { echo "--out 需要路径" >&2; exit 2; }; OUT="$2"; shift ;;
    --env) [[ $# -ge 2 ]] || { echo "--env 需要 NAME=VALUE" >&2; exit 2; }; EXTRA_ENVS+=("$2"); shift ;;
    --no-check) NO_CHECK=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "未知参数：$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

PY=""
for candidate in "${PYTHON:-}" python3 python py; do
  if [[ -n "${candidate}" ]] && command -v "${candidate}" >/dev/null 2>&1; then
    PY="${candidate}"; break
  fi
done
[[ -n "${PY}" ]] || { echo "未找到 python3/python" >&2; exit 2; }

case "${MODE}" in
  a|b|c) ;;
  *) echo "--mode 只支持 a|b|c，收到：${MODE}" >&2; exit 2 ;;
esac
if [[ ! "${AUTH_REF}" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
  echo "--auth-ref 必须是合法环境变量名" >&2; exit 2
fi
for pair in "${EXTRA_ENVS[@]}"; do
  if [[ ! "${pair}" =~ ^[A-Za-z_][A-Za-z0-9_]*= ]]; then
    echo "--env 需要 NAME=VALUE 形式：${pair%%=*}" >&2; exit 2
  fi
done

mkdir -p "$(dirname "${OUT}")"

# 凭据通过环境变量交给 Python，避免出现在命令行与回显里。
U07_MODE="${MODE}" U07_ENDPOINT="${ENDPOINT}" U07_TOKEN="${TOKEN}" \
U07_AUTH_REF="${AUTH_REF}" U07_PROVIDERS="${PROVIDERS}" U07_OUT="${OUT}" \
  "${PY}" - "${EXTRA_ENVS[@]}" <<'PYEOF'
import io, json, os, re, sys
from urllib.parse import urlsplit

mode = os.environ["U07_MODE"]
endpoint = os.environ.get("U07_ENDPOINT", "")
token = os.environ.get("U07_TOKEN", "")
auth_ref = os.environ["U07_AUTH_REF"]
providers = os.environ.get("U07_PROVIDERS", "")
out = os.environ["U07_OUT"]
extra = sys.argv[1:]


def fail(message: str) -> None:
    print(f"setup 失败：{message}", file=sys.stderr)
    raise SystemExit(2)


def quote(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


lines = [
    "# 由 setup.sh 生成；.tmp/ 已被 .gitignore 忽略，勿提交。",
    f"# 模式：{mode}",
]

if mode in ("a", "b"):
    if not endpoint:
        fail("A/B 模式需要 --endpoint")
    parts = urlsplit(endpoint)
    if parts.scheme != "https" or not parts.hostname or parts.username or parts.password:
        fail("--endpoint 必须是无凭据的 HTTPS 绝对地址")
    lines.append(f'SEARCH_ENDPOINT="{quote(endpoint.rstrip("/"))}"')
    if token:
        lines.append(f'SEARCH_AUTH_REF="{auth_ref}"')
        lines.append(f'{auth_ref}="{quote(token)}"')
    elif mode == "b":
        fail("B 模式需要 --token（与 Worker 的 SHARED_TOKEN 一致）；A 模式可省略")
else:
    if not providers:
        fail("C 模式需要 --providers")
    path = os.path.abspath(os.path.expanduser(providers))
    if not os.path.isfile(path):
        fail(f"找不到 providers.json：{path}")
    try:
        doc = json.load(io.open(path, encoding="utf-8"))
    except (OSError, ValueError) as exc:
        fail(f"providers.json 解析失败：{exc}")
    refs = []
    for item in doc.get("providers", []):
        if isinstance(item, dict) and item.get("authRef"):
            refs.append(str(item["authRef"]))
    unique_refs = sorted(set(refs))
    provided = {pair.split("=", 1)[0] for pair in extra}
    if unique_refs:
        missing = [ref for ref in unique_refs if ref not in provided and (len(unique_refs) > 1 or not token)]
        if missing:
            fail(
                "providers.json 的 authRef 需要凭据："
                + ", ".join(missing)
                + "；用 --env NAME=VALUE 逐个提供（单个 authRef 也可用 --token）"
            )
    lines.append(f'SEARCH_PROVIDERS_FILE="{quote(path)}"')
    if token and len(unique_refs) == 1:
        lines.append(f'{unique_refs[0]}="{quote(token)}"')
    for pair in extra:
        name, value = pair.split("=", 1)
        if not re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", name):
            fail(f"非法环境变量名：{name}")
        lines.append(f'{name}="{quote(value)}"')

lines.append("")
io.open(out, "w", encoding="utf-8", newline="\n").write("\n".join(lines))
print(f"已写入 {out}（凭据未回显）")
PYEOF
[[ $? -eq 0 ]] || exit 2

if [[ "${NO_CHECK}" -eq 0 ]]; then
  echo "---- 离线预检（不出网）----"
  U07_CONFIG_FILE="${OUT}" bash scripts/run-u07-43-samples.sh --check-only || exit 2
fi

echo "下一步：bash scripts/run-u07-43-samples.sh"
