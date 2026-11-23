# ci_gate_hardening · 编码任务清单（P0~P3）

> **执行基线（唯一需求与方案来源，不再重新权衡）**
> - 需求：[`.codeartsdoer/specs/ci_gate_hardening/spec.md`](./spec.md)（AC-CI-01~27、X1~X8、U1~U3）。
> - 方案：[`.codeartsdoer/specs/ci_gate_hardening/design.md`](./design.md)（2.0 设计裁决、2.1 增量设计、三章回滚兼容、四章 AC 映射、五章阶段划分）。
> - 已裁决前提：U1 = `rust` job 的 Windows 与 macOS 两矩阵任务均编译 `--features llm-https`，Linux 无 rust job、以 `compiled=0 … 不适用` 显式声明；U2 = C3 用独立 `windows-2022` job + 平台绑定基线，不新增跨平台元数据；U3 = C4 自检落为脚本子命令 `--self-test`（6 断言）。
> - **本文性质**：只做任务分解与验收判据。**不修改** 任何源码、脚本与 `.github/workflows/ci.yml`；`docs/sdd/**` 只读。
> - 编号约定：阶段 `P0~P3`、主任务 `W-CI-<n>`、子任务 `W-CI-<n>.<m>`；与缺口编号 `C1~C4`、验收编号 `AC-CI-*` 均不复用。

## 一、执行基线与冲突处置（只读，先读后做）

| 编号 | 事项 | 基线口径 | 本清单处置 | 影响任务 |
|---|---|---|---|---|
| R-0 | `native-tls` 声明范围 | spec 7.3 / X2 记为"仅 `cfg(windows)`"；实际 `crates/infrastructure/Cargo.toml:29-32` 同时声明 `cfg(windows)` 与 `cfg(target_os = "macos")` | 采用 design 2.0 事实校正 **F1**：win/mac 两矩阵均可编译，不补依赖声明、不新增 Linux rust job。**不改变任何 AC 口径**，仅在 L3 观测时记录 macOS 实际编译结果 | W-CI-10、W-CI-12、W-CI-18 |
| R-1 | AC-CI-16 与 AC-CI-17 字面冲突（`--ignored`） | AC-CI-16 禁 `--ignored`；AC-CI-17 要求 `-- --list --ignored` 计数 ≥8 / ≥1 | 采用 design 2.0 的 **X9** 裁决：唯一允许形态是 `-- --list --ignored`（列举）；禁止 `-- --ignored`（执行）、`--nocapture`、`--test-threads`、`run-llm-live.sh`。静态审计必须带 `grep -v -- '--list --ignored'` 过滤 | W-CI-10.2、W-CI-13.1、W-CI-13.3 |
| R-2 | AC-CI-24 的 job 计数 | spec 写"静态检查 4 个 job 均有 `timeout-minutes`"；design 新增 2 个 job | 取**更严格者**：改造后共 **6** 个 job（`web`/`rust`/`compliance`/`sqlcipher`/`feature-coverage`/`test-name-inventory`）全部必须声明 job 级超时 | W-CI-16.3 |
| R-3 | `baseline_drift=N` | spec 未要求此输出 | 视为 design 增量（不改变退出码、不改变 0/1/2 语义），登记为"超出 spec 的增量输出"，不作为 AC 判据 | W-CI-4.3 |
| R-4 | 首行诊断的文件名 | spec 6.1-1/4.5-4 要求相对路径 + 正斜杠，未固定具体文件 | 采用 design 2.1.3-A-4：`file` 固定为 `scripts/test-name-inventory.sh`；基线类诊断的 `file` 为**基线路径本身**（相对、正斜杠） | W-CI-1.1、W-CI-3.1、W-CI-15.3 |

## 二、全局执行约定（适用于全部任务）

```bash
# G-1 环境（本地验证命令的前置，必须最先执行）
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
tmp="$(mktemp -d)"           # 受 TEMP/TMP 影响，落在 .tmp/test-temp 下；.tmp/ 已被 .gitignore 忽略
```

| 编号 | 约定 | 判定方式 |
|---|---|---|
| G-1 | 本地一律 git-bash（bash），`cargo` 走工具链内真实可执行文件（`CARGO` 覆盖），**禁止**使用损坏的 rustup shim 解释成功 | `echo "$CARGO"` 指向 `.../toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe` |
| G-2 | 退出码三分：`0` 通过 / `1` 命中违规 / `2` 扫描面缺失（含不可比）。`2` 不得映射为 `0` 或"跳过" | 每个场景显式 `echo rc=$?` |
| G-3 | 首行必须是 `文件:行号 标记`（`^.+:[0-9]+(:| )`），相对路径 + 正斜杠，无 `C:\` | `head -1 … | grep -qE '^.+:[0-9]+(:| )'` |
| G-4 | 失败路径（退 2）输出**不得**出现 `removed=`；`1` 与 `2` 分支互斥 | `grep -c 'removed=' …` = 0 |
| G-5 | 脚本必须兼容 bash 3.2：禁用 `mapfile`、`declare -A`、数组切片、`&>>`；`*.sh` 行尾 LF | `bash -n <script>` 退出 0；文件无 CRLF |
| G-6 | 排序与比较固定 `LC_ALL=C`；`comm` 两侧先 `sort -u` | 构造 locale 差异样本不产生伪 removed |
| G-7 | CI 禁项：`|| true`、`continue-on-error: true`、`set +e`、`if: false`、在 `ci.yml` 出现基线生成子命令字样、`--all-features`、`LLM_*` 联调变量 | 各阶段静态 grep 命中计数 = 0 |
| G-8 | 既有 4 个 job 的既有步骤 `run` 命令**逐字不改**（唯一例外：`compliance` 的 `checkout` 增加 `fetch-depth: 0`） | `git diff .github/workflows/ci.yml` 中既有 step 的 `run:` 行无删改 |
| G-9 | 不得删除、改名、`#[ignore]` 化任何既有测试用例 | `--diff` 持续 `removed=0`；`cargo test --workspace` 计数不低于 399 passed / 8 ignored / 38 targets |

## 三、阶段 P0：C4 抓取失败语义可判定化（最高优先级）

**阶段目标**：任何"抓取不到的用例面"必须被判为扫描面缺失并退 `2`；只有确证抓取完整才允许进入差集并返回 `0/1`。
**覆盖**：AC-CI-01~09、AC-CI-23、AC-CI-25（C4 部分）。
**前置**：无。**出口判据**见 W-CI-6。
**强制事项**：必须完成 W-CI-2.2（`inventory` 子 shell 收口重构）——只改 `list_target` 会让失败面在主 shell 恒为空，AC-CI-02 静默失效（design 5.4-I-1）。

- [x] **W-CI-1 入口分发与工具链可用性前置** · `scripts/test-name-inventory.sh`
  - 追溯：AC-CI-01 · design 2.1.3-A-1、2.1.3-A-9、2.2.2-I-1
  - DoD：`--diff` / `--emit-baseline` / `--self-test` / 无参四条入口均先过探针；探针失败一律 `2`；`bash -n` 退出 0。子任务 1.1~1.3 全达成
  - 证据：见 1.1~1.3

  - [x] **W-CI-1.1 `diag()` 首行双写与相对路径约定** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-01/06/07 · design 2.1.3-A-4、3.4-4、spec 6.1
    - DoD：`file` 恒为 `scripts/test-name-inventory.sh`（相对、正斜杠，禁止 `$(pwd)` 拼接）；`line` 为脚本内真实调用行整数；首行**同时**写 stdout 与 stderr；明细仍写 stderr；`G-3` 正则对首行命中
    - 证据：`bash -n scripts/test-name-inventory.sh; echo rc=$?` → `0`；`grep -c 'test-name-inventory.sh' scripts/test-name-inventory.sh` ≥ 1（常量入口）；构造任一 `2` 场景后 `head -1 "$tmp/o" | grep -qE '^.+:[0-9]+(:| )'` → 真；`grep -c 'C:\\' "$tmp/o"` → `0`

  - [x] **W-CI-1.2 工具链可用性探针（127 即退 2）** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-01 · design 2.1.3-A-1、spec 5.1.3-1
    - DoD：`CARGO` 含 `/` 时 `[[ -x "${CARGO}" ]]` 判存在性，随后无条件执行 `"${CARGO}" --version` 取返回码；不含 `/` 时先 `command -v`；任何非 0（含 127 / `STATUS_DLL_NOT_FOUND` 映射）→ 首行 `扫描面缺失 cargo_unavailable` + 退 `2`；**不写入任何清单、不进入抓取**
    - 证据：`printf '#!/usr/bin/env bash\nexit 127\n' > "$tmp/cargo127"; chmod +x "$tmp/cargo127"; CARGO="$tmp/cargo127" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt >"$tmp/o" 2>&1; echo rc=$?` → `2`；`CARGO=nonexistent-cargo-xyz bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt >"$tmp/o2" 2>&1; echo rc=$?` → `2`；两者 `grep -c 'removed='` → `0`

  - [x] **W-CI-1.3 入口分发与未知参数退 2** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-01 · design 2.2.2-I-1（异常映射：未知参数 → 2）
    - DoD：`--diff`/`--emit-baseline`/`--self-test`/无参 均在抓取前完成探针；未知参数输出 `文件:行号` 诊断并退 `2`（不得落入"打印清单"分支）
    - 证据：`bash scripts/test-name-inventory.sh --bogus >"$tmp/o" 2>&1; echo rc=$?` → `2`；`head -1 "$tmp/o" | grep -qE '^.+:[0-9]+(:| )'` → 真

- [x] **W-CI-2 抓取面改造与 `inventory` 收口重构** · `scripts/test-name-inventory.sh`
  - 追溯：AC-CI-02/03/05 · design 2.1.3-A-2/A-3、5.4-I-1
  - DoD：失败面（`pkg::target::rc`）可从抓取层传递到主 shell 并决定退出码；`inventory` 无子 shell 收口；失败面永不出现在 `comm` 输入中。子任务 2.1~2.5 全达成
  - 证据：`grep -cE 'done[[:space:]]*\|[[:space:]]*sort' scripts/test-name-inventory.sh` → `0`；`bash scripts/test-name-inventory.sh --self-test` 的 T2 通过

  - [x] **W-CI-2.1 `list_target` 先落盘再归一化** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-02 · design 2.1.3-A-2、1.2.1-关键缺陷 1
    - DoD：stdout 写 `mktemp` 文件、stderr 写另一 `mktemp` 文件，随后取 `rc=$?`；`rc != 0` 时把 `${pkg}::${target}::${rc}` 追加到失败面集合并输出 stderr 前 5 行（缩进、不得含 `removed`）；**仅 `rc == 0` 时才**把归一化结果拼接进清单；`list_target` 内不得出现 `2>/dev/null`；函数签名与调用点不变
    - 证据：`grep -n 'list_target' -A16 scripts/test-name-inventory.sh | grep -c '2>/dev/null'` → `0`；自检 T2 输出含 `face=` 与具体 `pkg::target`；`bash -n` 退出 0

  - [x] **W-CI-2.2 【强制】重构 `inventory` 的子 shell 收口** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-02 · design 2.1.3-A-2、5.4-I-1（最高隐蔽风险）
    - DoD：删除 `done | sort -u` 形态，改为"每面结果追加写入临时文件 + `inventory` 结束后在主 shell 执行 `LC_ALL=C sort -u`"；失败面集合由全局变量（或临时文件）承载，**在调用方（`case` 分支）可见**；失败面非空时优先退 `2`，且不执行 `comm`；T2 必须在"真实仓库根 + 桩"下触发（非仅空目录）
    - 证据：`grep -cE 'done[[:space:]]*\|[[:space:]]*sort' scripts/test-name-inventory.sh` → `0`；`grep -c 'FAILED_FACES\|failed_face' scripts/test-name-inventory.sh` ≥ `2`（定义 + 调用方读取）；`bash scripts/test-name-inventory.sh --self-test` 日志含 `SELF-TEST: 6/6 ok`

  - [x] **W-CI-2.3 manifest 扫描面缺失（manifests=0 → 退 2）** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-05 · design 2.1.3-A-3、spec 5.1.3-5
    - DoD：`app/src-tauri/Cargo.toml` 与 `crates/*/Cargo.toml` 均不存在时，首行 `扫描面缺失 manifests=0`，诊断含 `app/src-tauri` 与 `crates` 两处扫描面；`--emit-baseline` 与 `--diff` 均退 `2` 且不产生 PASS
    - 证据：`mkdir -p "$tmp/empty"; INVENTORY_ROOT="$tmp/empty" bash scripts/test-name-inventory.sh --emit-baseline "$tmp/b.txt" >"$tmp/o" 2>&1; echo rc=$?` → `2`；`grep -c 'app/src-tauri' "$tmp/o"` ≥ `1`；`grep -c 'crates' "$tmp/o"` ≥ `1`；`test ! -e "$tmp/b.txt"` → 真

  - [x] **W-CI-2.4 空清单禁令（0 行 → 退 2）** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-03 · design 2.1.3-A-3、spec 4.2-4
    - DoD：所有面 `rc=0` 但归一化后 0 行时，首行 `扫描面缺失 inventory_empty`，退 `2`，**不执行 `comm`**、不输出 `removed=`
    - 证据：`printf '#!/usr/bin/env bash\nexit 0\n' > "$tmp/cargo0"; chmod +x "$tmp/cargo0"; CARGO="$tmp/cargo0" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt >"$tmp/o" 2>&1; echo rc=$?` → `2`；`grep -c '扫描面缺失' "$tmp/o"` ≥ `1`；`grep -c 'removed=' "$tmp/o"` → `0`

  - [x] **W-CI-2.5 失败路径禁令（无 `removed=`、1/2 互斥）** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-02/03/07 · design 2.1.3-A-3、spec 5.1.1-10
    - DoD：全部退 `2` 的路径输出中 `removed=` 出现 0 次；退 `1` 路径必须先确证抓取完整与基线可用；`1` 与 `2` 由互斥分支产生（同一路径不可能同时成立）；`2` 路径不输出 removed 明细行
    - 证据：对 2.3 / 2.4 / 3.1 三场景分别 `grep -c 'removed=' "$tmp/oN"` → 均为 `0`；对 15.2 场景 `grep -c 'removed='` 亦为 `0` 且存在明细行

- [x] **W-CI-3 基线校验与原子写** · `scripts/test-name-inventory.sh`
  - 追溯：AC-CI-04/06 · design 2.1.3-A-5/A-6
  - DoD：基线不可用时退 `2` 且不进入 `comm`；`--emit-baseline` 失败不破坏既有基线。子任务 3.1~3.3 全达成
  - 证据：见 3.1~3.3

  - [x] **W-CI-3.1 基线存在 / 可读 / `LC_ALL=C` 有序校验** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-06 · design 2.1.3-A-6、spec 5.1.3-4
    - DoD：`[[ -f && -r ]]` 全过；去空行后 `LC_ALL=C sort -c` 通过；否则首行退 `2`，标记串分别为 `baseline_missing` / `baseline_unreadable` / `baseline_unsorted`；首行 `file` 为基线路径本身（相对、正斜杠）；不输出 removed 明细
    - 证据：`bash scripts/test-name-inventory.sh --diff docs/progress/nope.txt >"$tmp/o1" 2>&1; echo rc=$?` → `2`；`LC_ALL=C sort -r docs/progress/test-names-before.txt > "$tmp/rev.txt"; bash scripts/test-name-inventory.sh --diff "$tmp/rev.txt" >"$tmp/o2" 2>&1; echo rc=$?` → `2`；两者 `head -1 | grep -qE '^.+:[0-9]+(:| )'` → 真；`grep -c '扫描面缺失' "$tmp/o1"` ≥ `1`

  - [x] **W-CI-3.2 `--emit-baseline` 原子写** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-04 · design 2.1.3-A-5、spec 5.1.3-6
    - DoD：`mkdir -p "$(dirname "${OUT}")"` → 同目录 `mktemp "${OUT}.tmp.XXXXXX"` → `trap 'rm -f "${TMP}"' EXIT` → 写入后 `wc -l ≥ 1` → `mv -f`；抓取失败时目标路径**不存在**，目标已存在时 `sha256` **不变**；不出现半截/空文件
    - 证据：`printf '#!/usr/bin/env bash\nexit 127\n' > "$tmp/cargo127"; chmod +x "$tmp/cargo127"`；`echo keep > "$tmp/exists.txt"; before=$(sha256sum "$tmp/exists.txt" | cut -d' ' -f1); CARGO="$tmp/cargo127" bash scripts/test-name-inventory.sh --emit-baseline "$tmp/exists.txt" >"$tmp/o" 2>&1; echo rc=$?; test "$before" = "$(sha256sum "$tmp/exists.txt" | cut -d' ' -f1)" && echo unchanged` → `2` + `unchanged`；`CARGO="$tmp/cargo127" bash scripts/test-name-inventory.sh --emit-baseline "$tmp/new.txt" >/dev/null 2>&1; test ! -e "$tmp/new.txt" && echo not-created` → `not-created`

  - [x] **W-CI-3.3 比较前双侧归一化与漂移提示** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-06/09 · design 2.1.3-A-6、2.1.3-E-4
    - DoD：基线与当前两侧均 `grep -v '^[[:space:]]*$' | LC_ALL=C sort -u` 落临时文件后才 `comm`；空行/重复不得产生伪 removed；正常态输出 `removed=0`、新增条数、`baseline_drift=N`（增量输出，不改退出码）
    - 证据：`{ cat docs/progress/test-names-before.txt; printf '\n\n'; } > "$tmp/base_blank.txt"; CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff "$tmp/base_blank.txt" >"$tmp/o" 2>&1; echo rc=$?` → `0`；`grep -c 'removed=0' "$tmp/o"` ≥ `1`；`grep -c 'baseline_drift=' "$tmp/o"` ≥ `1`

- [x] **W-CI-4 平台可比性（AC-CI-23）** · `scripts/test-name-inventory.sh`
  - 追溯：AC-CI-23 · design 2.1.3-A-7、2.2.2-I-2、2.0-U2
  - DoD：平台不可比 → 退 `2` 且首行含 `不可比` 与两个 triple；平台无法解析 → 退 `2`；一致时才可能出现 `0/1`。子任务 4.1~4.3 全达成
  - 证据：见 4.1~4.3

  - [x] **W-CI-4.1 `exec_platform` 解析优先级** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-23 · design 2.1.3-A-7、2.1.2 配置表
    - DoD：解析顺序 `INVENTORY_PLATFORM` > `"${CARGO}" -vV` 的 `host:` 行 > `uname -m` 映射；两者都失败 → 视为扫描面缺失退 `2`；解析发生在**抓取前**（不可比场景不触发全量编译）
    - 证据：`time (INVENTORY_PLATFORM=aarch64-apple-darwin CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt >"$tmp/o" 2>&1; echo rc=$?)` → `rc=2` 且实际耗时 < 60s（证明未进入全量编译）

  - [x] **W-CI-4.2 基线平台常量与"不可比"退 2** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-23 · design 2.1.3-A-7、2.0-U2、spec 6.5
    - DoD：`baseline_platform` 取 `INVENTORY_BASELINE_PLATFORM` > 脚本常量 `x86_64-pc-windows-msvc`；两个 triple 不等 → 首行 `不可比 exec=<...> baseline=<...>` 退 `2`，不输出 removed 结论；不新增基线文件、不新增元数据文件
    - 证据：`INVENTORY_PLATFORM=aarch64-apple-darwin CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt >"$tmp/o" 2>&1; echo rc=$?` → `2`；`head -1 "$tmp/o" | grep -q '不可比'` → 真；`grep -c 'aarch64-apple-darwin' "$tmp/o"` ≥ `1`；`grep -c 'x86_64-pc-windows-msvc' "$tmp/o"` ≥ `1`；`grep -c 'removed=' "$tmp/o"` → `0`

  - [x] **W-CI-4.3 正常态输出契约（removed=0 + 新增 N）** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-09 · design 2.1.3-A-6、2.1.3-E-4
    - DoD：平台一致、抓取完整、无 removed 时退 `0`；输出含 `removed=0` 与新增条数；`0` 退出不得伴随失败语义首行（spec 6.1-5）
    - 证据：`CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt >"$tmp/o" 2>&1; echo rc=$?` → `0`；`grep -c 'removed=0' "$tmp/o"` ≥ `1`；`grep -cE '扫描面缺失|出现 removed 用例' "$tmp/o"` → `0`

- [x] **W-CI-5 `--self-test` 六断言自检（U3 落地）** · `scripts/test-name-inventory.sh`
  - 追溯：AC-CI-08 · design 2.1.3-A-8（T1~T6 表）、2.0-U3
  - DoD：`--self-test` 输出 `SELF-TEST: 6/6 ok` 且退 `0`；任一不符输出 `SELF-TEST: failed case=T<n>` 且退 `1`；全程 `mktemp -d` + 桩 CARGO，不联网、不调用真实 cargo。子任务 5.1~5.3 全达成
  - 证据：`bash scripts/test-name-inventory.sh --self-test; echo rc=$?` → `0`；日志含 `SELF-TEST: 6/6 ok`

  - [x] **W-CI-5.1 断言 T1 / T2 / T3（缺面三态）** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-01/02/03 · design 2.1.3-A-8 的 T1/T2/T3
    - DoD：T1 = 桩 `exit 127` 指向真实仓库 → `rc=2`、首行匹配、无 `removed=`；T2 = 桩对 `--lib` 返 `0`、对 `--test` 返 `1` → `rc=2`、输出含具体 `pkg::target`、无 `removed=`；T3 = 桩 `exit 0` 无输出 → `rc=2`、含 `扫描面缺失`
    - 证据：`bash scripts/test-name-inventory.sh --self-test 2>&1 | grep -cE 'T1|T2|T3'` ≥ `3`；进程退出 `0`；`--self-test` 运行期无 `target/` 全量编译（耗时 < 30s）

  - [x] **W-CI-5.2 断言 T4 / T5（扫描面缺失与原子写）** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-05/04 · design 2.1.3-A-8 的 T4/T5
    - DoD：T4 = `INVENTORY_ROOT` 指向空目录 → `rc=2`，诊断含 `app/src-tauri` 与 `crates`；T5 = 失败桩对"已存在文件"与"不存在路径"各执行一次 → 前者哈希不变、后者仍不存在
    - 证据：`bash scripts/test-name-inventory.sh --self-test 2>&1 | grep -cE 'T4|T5'` ≥ `2`；退出码 `0`

  - [x] **W-CI-5.3 断言 T6（正常态）与汇总输出** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-08/09 · design 2.1.3-A-8 的 T6
    - DoD：T6 = 正常桩（固定输出 `fake_case: test`）先生成基线、再对同桩 `--diff` → 两次 `rc=0`，第二次含 `removed=0`；汇总行严格为 `SELF-TEST: 6/6 ok`
    - 证据：`out=$(bash scripts/test-name-inventory.sh --self-test 2>&1); rc=$?; echo "rc=$rc"; printf '%s\n' "$out" | grep -c 'SELF-TEST: 6/6 ok'` → `rc=0` + `1`；`printf '%s\n' "$out" | grep -c 'T6'` ≥ `1`

- [x] **W-CI-6 P0 出口：本地实跑硬判据与脚本层不回退** · `scripts/test-name-inventory.sh`
  - 追溯：AC-CI-01/08/09/23/26 · design 5.1（P0 门）、5.3
  - DoD：三条硬判据（6.1 / 6.2 / 6.3）全部达成 + 6.4 脚本层既有契约不回退 + 按 design 2.1.3-A-9 决策表 8 行逐项核对（`cargo_unavailable` / `face=` / `inventory_empty` / `manifests=0` / `baseline_missing|unreadable|unsorted` / `不可比` / `出现 removed 用例` / `removed=0`），每行记录退出码与首行标记串。任一失败即 P0 未出口，禁止进入 P1/P2/P3
  - 证据：见 6.1~6.4；三项结果需落盘记录（`$tmp/p0-evidence.txt`）供 W-CI-18 比对

  - [x] **W-CI-6.1 【硬判据-1】本地实跑"shim 不可用 → 退 2"复现** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-01 · design 4.2（AC-CI-01 行）、spec 7.3（C4 误报事实）
    - DoD：桩返回 127 时退 `2`、首行匹配 `^.+:[0-9]+(:| )` 且含 `扫描面缺失`、输出 `removed=` 计数 `0`；**连续执行 3 次结论一致**（spec 4.2-1）
    - 证据：
      ```bash
      export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
      export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
      tmp="$(mktemp -d)"
      printf '#!/usr/bin/env bash\nexit 127\n' > "$tmp/cargo"; chmod +x "$tmp/cargo"
      for i in 1 2 3; do
        CARGO="$tmp/cargo" bash scripts/test-name-inventory.sh \
          --diff docs/progress/test-names-before.txt >"$tmp/o$i" 2>&1
        echo "run$i rc=$?"
        head -1 "$tmp/o$i" | grep -qE '^.+:[0-9]+(:| )' && echo "run$i first-line=ok"
        echo "run$i removed_hits=$(grep -c 'removed=' "$tmp/o$i")"
        echo "run$i scan_miss=$(grep -c '扫描面缺失' "$tmp/o$i")"
      done
      ```
      期望：3 次均为 `rc=2` + `first-line=ok` + `removed_hits=0` + `scan_miss>=1`

  - [x] **W-CI-6.2 【硬判据-2】真实仓库 `--diff` 退 0 且 `removed=0`** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-09 · design 4.2（AC-CI-09 行）、4.4
    - DoD：真实工具链 + 真实基线，退 `0`，输出 `removed=0` 与新增条数（当前实测口径：新增 42）；连续 3 次一致
    - 证据：`CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt >"$tmp/p0diff.txt" 2>&1; echo rc=$?; grep -c 'removed=0' "$tmp/p0diff.txt"` → `rc=0` + `≥1`。**注意**：该命令会做 workspace 全量编译，请在其余任务之后执行，并预留 ≥10 分钟

  - [x] **W-CI-6.3 【硬判据-3】`--self-test` 6/6 通过** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-08 · design 2.1.3-A-8、5.1（P0 门）
    - DoD：退 `0` 且日志含 `SELF-TEST: 6/6 ok`；全程不联网、不调用真实 cargo
    - 证据：`bash scripts/test-name-inventory.sh --self-test >"$tmp/p0self.txt" 2>&1; echo rc=$?; grep -c 'SELF-TEST: 6/6 ok' "$tmp/p0self.txt"` → `rc=0` + `1`

  - [x] **W-CI-6.4 脚本层既有契约不回退** · `scripts/check-capture-offline.sh`、`scripts/check-g2-claims.sh`、`scripts/scan-secrets-docs.sh`
    - 追溯：AC-CI-11/14/25 · design 1.2.2、3.2 兼容结论
    - DoD：`check-capture-offline.sh --self-test` 仍退 `1`；`check-g2-claims.sh --self-test` 仍退 `1`；`scan-secrets-docs.sh --self-test` 退 `0`；三脚本正常态退 `0`；P0 未触碰三脚本（本阶段 0 改动）
    - 证据：`for s in check-capture-offline check-g2-claims; do bash scripts/$s.sh --self-test >/dev/null 2>&1; echo "$s self_test_rc=$?"; done` → 均 `1`；`bash scripts/scan-secrets-docs.sh --self-test; echo rc=$?` → `0`；`git diff --stat scripts/check-capture-offline.sh scripts/check-g2-claims.sh scripts/scan-secrets-docs.sh` → 空

## 四、阶段 P1：C1 三条静态门禁纳入 compliance

**阶段目标**：三条静态门禁成为 `compliance` 必过步骤；任一退 `1`/`2` 即 job 失败；脚本缺失不静默；浅克隆收窄被补齐。
**覆盖**：AC-CI-10~14、AC-CI-24/25（C1 部分）。
**前置**：P0 出口（W-CI-6 三条硬判据）达成——保证脚本头注释与 `diag` 风格统一、`ci.yml` 一次只改一段。
**强制事项**：不得改写既有 `layering` / `secrets` / `licenses` / `sbom` 步骤的命令；新增步骤插在 `secrets` 之后、`licenses` 之前。

- [x] **W-CI-7 两个脚本的"缺失 → 2"最小补丁** · `scripts/check-capture-offline.sh`、`scripts/check-g2-claims.sh`
  - 追溯：AC-CI-12/25 · design 2.1.3-B（脚本侧补丁）、1.2.2
  - DoD：扫描面缺失优先退 `2`、命中仍退 `1`；`--self-test` 路径完全不动；两脚本 `bash -n` 通过。子任务 7.1~7.3 全达成
  - 证据：见 7.1~7.3

  - [x] **W-CI-7.1 `check-capture-offline.sh` 新增 `MISSING` 分支** · `scripts/check-capture-offline.sh`
    - 追溯：AC-CI-12 · design 2.1.3-B（`scan()` 缺失分支 → `missing()`）
    - DoD：新增 `MISSING=0` 与 `missing()`（置 `MISSING=1` 并输出 `文件:行号 扫描面缺失 [label]`）；`scan()` 的"目标不存在"改调 `missing()`；末尾判定顺序 `MISSING != 0 → exit 2` **优先于** `FAIL != 0 → exit 1`；`rule_harness` 的"复用文件数 < 4"仍属内容违规，保持 `FAIL=1`；检测规则（pattern）零改动
    - 证据：`bash -n scripts/check-capture-offline.sh; echo rc=$?` → `0`；`mv crates/application/tests/support/fakes.rs "$tmp/fakes.rs"; bash scripts/check-capture-offline.sh --rule=fakes >"$tmp/o" 2>&1; echo rc=$?; mv -f "$tmp/fakes.rs" crates/application/tests/support/fakes.rs` → `2`，日志含 `扫描面缺失`；恢复后 `git status --short crates/application/tests/support/fakes.rs` → 空

  - [x] **W-CI-7.2 `check-g2-claims.sh` 新增 `MISSING` 分支** · `scripts/check-g2-claims.sh`
    - 追溯：AC-CI-12 · design 2.1.3-B
    - DoD：同上；`git log` 不可用**不作为**缺失（由 `fetch-depth` 保证），仍只影响标题扫描；`PATTERN` 与排除目录零改动
    - 证据：`mv README.md "$tmp/README.md"; bash scripts/check-g2-claims.sh >"$tmp/o" 2>&1; echo rc=$?; mv -f "$tmp/README.md" README.md` → `2`，日志含 `扫描面缺失`；恢复后 `git status --short README.md` → 空

  - [x] **W-CI-7.3 两脚本 `--self-test` 契约冻结（仍退 1）** · `scripts/check-capture-offline.sh`、`scripts/check-g2-claims.sh`
    - 追溯：AC-CI-11 · design 2.1.3-B（自检路径完全不动）、1.2.2
    - DoD：两脚本 `--self-test` 植入违规后仍退 `1`，输出含 `预期退出 1`；`--rule=*` 子命令的命中断言语义不变
    - 证据：`bash scripts/check-capture-offline.sh --self-test >/dev/null 2>&1; echo rc=$?` → `1`；`bash scripts/check-g2-claims.sh --self-test >/dev/null 2>&1; echo rc=$?` → `1`；`git diff -U0 scripts/check-capture-offline.sh | grep '^+' | grep -cE 'SELF-TEST|selftest'` → `0`；`git diff -U0 scripts/check-g2-claims.sh | grep '^+' | grep -cE 'SELF-TEST|selftest'` → `0`（自检分支零改动）

- [x] **W-CI-8 compliance 三 step 与 `fetch-depth: 0`** · `.github/workflows/ci.yml`
  - 追溯：AC-CI-10 · design 2.1.3-B（插入位置与命令原文）、2.1.3-C（X7）
  - DoD：三条调用均存在且无吞错修饰；`deploy` 顺序为 `secrets` → 三新步骤 → `licenses`；`checkout` 带 `fetch-depth: 0`；既有步骤 `run` 逐字不变。子任务 8.1~8.4 全达成
  - 证据：见 8.1~8.4

  - [x] **W-CI-8.1 compliance 的 `checkout` 增加 `fetch-depth: 0`** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-10（子条件）· design 2.1.3-C、spec X7
    - DoD：仅 `compliance` job 的 `actions/checkout@v4` 增加 `with: fetch-depth: 0`（其余 job 的 checkout 不变）；不引入 `commit-scan=<N>` 收窄标记（`fetch-depth: 0` 已补齐）
    - 证据：`grep -c 'fetch-depth: 0' .github/workflows/ci.yml` → `1`；`grep -n -B2 -A1 'fetch-depth: 0' .github/workflows/ci.yml` 落在 `compliance` 段（第 100 行之后）

  - [x] **W-CI-8.2 `capture-offline (C1)` 步骤（命令原文 + 超时）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-10/11/12 · design 2.1.3-B 表第 1 行、2.2.2-I-3 的 S-1
    - DoD：步骤原文为
      ```yaml
      - name: capture-offline (C1)
        timeout-minutes: 5
        run: bash scripts/check-capture-offline.sh --all
      ```
      无 `continue-on-error`、无 `|| true`、无 `if:`
    - 证据：`grep -c 'check-capture-offline.sh --all' .github/workflows/ci.yml` → `1`；`grep -n -A6 'name: capture-offline (C1)' .github/workflows/ci.yml` 含 `timeout-minutes: 5`；`grep -n -A6 'name: capture-offline (C1)' .github/workflows/ci.yml | grep -c 'continue-on-error'` → `0`

  - [x] **W-CI-8.3 `g2-claims (C1)` 步骤（命令原文 + 超时）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-10/11/12 · design 2.2.2-I-3 的 S-2
    - DoD：步骤原文为
      ```yaml
      - name: g2-claims (C1)
        timeout-minutes: 5
        run: bash scripts/check-g2-claims.sh
      ```
      无吞错修饰
    - 证据：`grep -c 'scripts/check-g2-claims.sh' .github/workflows/ci.yml` → `1`；`grep -n -A2 'name: g2-claims (C1)' .github/workflows/ci.yml` 含 `timeout-minutes: 5`

  - [x] **W-CI-8.4 `secrets-docs (C1)` 步骤（命令原文 + 超时）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-10/11/12/14 · design 2.2.2-I-3 的 S-3
    - DoD：步骤原文为
      ```yaml
      - name: secrets-docs (C1)
        timeout-minutes: 5
        run: bash scripts/scan-secrets-docs.sh --all-md
      ```
      无吞错修饰；位于 `licenses` 之前
    - 证据：`grep -c 'scan-secrets-docs.sh --all-md' .github/workflows/ci.yml` → `1`；`grep -n 'name: \(secrets\|secrets-docs (C1)\|licenses\)' .github/workflows/ci.yml` 的行号顺序为 `secrets` < `secrets-docs (C1)` < `licenses`

- [x] **W-CI-9 P1 静态审计与本地证据** · `.github/workflows/ci.yml`、`scripts/`
  - 追溯：AC-CI-10/12/13 · design 5.3（P1 最小可复现证据）、4.2
  - DoD：6 条静态 grep 全部符合；缺失面构造退 `2`；脚本缺失不静默。子任务 9.1~9.3 全达成
  - 证据：见 9.1~9.3

  - [x] **W-CI-9.1 静态审计 6 条** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-10 · design 4.2（AC-CI-10 行）
    - DoD：三条命令各 ≥1；吞错修饰 0 命中；`fetch-depth: 0` ≥1
    - 证据：
      ```bash
      grep -c 'check-capture-offline.sh --all' .github/workflows/ci.yml    # ≥1
      grep -c 'scripts/check-g2-claims.sh' .github/workflows/ci.yml        # ≥1
      grep -c 'scan-secrets-docs.sh --all-md' .github/workflows/ci.yml     # ≥1
      grep -c 'fetch-depth: 0' .github/workflows/ci.yml                    # ≥1
      grep -nE '\|\| true|continue-on-error: true|set \+e|if: false' .github/workflows/ci.yml | wc -l   # 0
      grep -c 'timeout-minutes: 5' .github/workflows/ci.yml                # ≥3
      ```

  - [x] **W-CI-9.2 扫描面缺失即失败（退 2 证据）** · `scripts/check-g2-claims.sh`
    - 追溯：AC-CI-12 · design 4.2（AC-CI-12 行）
    - DoD：移除 `docs/progress` 后脚本退 `2`，日志含 `扫描面缺失`；`ci.yml` 对应步骤无 `continue-on-error`
    - 证据：`mv docs/progress "$tmp/progress.bak"; bash scripts/check-g2-claims.sh >"$tmp/o" 2>&1; echo rc=$?; mv -f "$tmp/progress.bak" docs/progress` → `2`；`grep -c '扫描面缺失' "$tmp/o"` ≥ `1`；`git status --short docs/progress` → 空

  - [x] **W-CI-9.3 脚本缺失不静默（非 0 且日志含路径）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-13 · design 4.2（AC-CI-13 行）
    - DoD：步骤直接引用仓库相对路径，无 `if [ -f ... ]` 短路分支；脚本被移出时执行原命令返回非 0，日志首行含缺失路径
    - 证据：`mv scripts/check-g2-claims.sh "$tmp/"; bash scripts/check-g2-claims.sh >"$tmp/o" 2>&1; echo rc=$?; mv -f "$tmp/check-g2-claims.sh" scripts/` → 非 0（`127` 或 `1`）；`grep -c 'check-g2-claims.sh' "$tmp/o"` ≥ `1`；`grep -cE 'if \[ -f .*scripts/' .github/workflows/ci.yml` → `0`

## 五、阶段 P2：C2 `llm-https` 编译覆盖（只编译不执行）

**阶段目标**：至少在 Windows 与 macOS 两矩阵完成 `llm-https` 编译 + 用例列举，并以 artifact 证明 `baseline_30_samples_per_task` 被收集；Linux 显式"不适用"。
**覆盖**：AC-CI-15~19、AC-CI-24（C2 部分）。
**前置**：P0 出口达成（`ci.yml` 一次只改一段）。
**强制禁项**：不注入任何 `LLM_*` 联调变量；不执行 ignored 用例（唯一允许形态 `-- --list --ignored`）；不使用 `--all-features`；`compiled=0` 只允许出现在 Linux 静态声明行。

- [x] **W-CI-10 rust job 的 `llm-https-compile (C2)` 步骤** · `.github/workflows/ci.yml`
  - 追溯：AC-CI-15/16/17/19 · design 2.1.3-D（选型论证）、2.2.2-I-3 的 S-5
  - DoD：步骤位于 `test` 之后，`timeout-minutes: 15`、显式 `shell: bash`、`set -euo pipefail`；命令同时满足"含 `--features llm-https`""含 `--test llm_live_baseline`""含 `-- --list`"、**不含** `--all-features`；输出 `llm-https: compiled=1 platform=<triple>`；既有 `fmt`/`clippy`/`build`/`test` 命令逐字不变。子任务 10.1~10.3 全达成
  - 证据：见 10.1~10.3

  - [x] **W-CI-10.1 步骤骨架与禁项** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-16/19 · design 2.1.3-D（"为什么不用 `--no-run`/`check`"论证）、2.0-X9
    - DoD：`name: llm-https-compile (C2)`、`timeout-minutes: 15`、`shell: bash`、`env: LLM_HTTPS_TARGET: ${{ matrix.target }}`；`set -euo pipefail`；不含 `--ignored`（执行形态）、`--nocapture`、`--test-threads`、`run-llm-live`、`--all-features`；不定义任何 `LLM_*` 联调变量；插入位置紧接 `test` 步骤之后
    - 证据：`grep -n -A6 'name: llm-https-compile (C2)' .github/workflows/ci.yml` 含 `timeout-minutes: 15` 与 `shell: bash`；`grep -c 'all-features' .github/workflows/ci.yml` → `0`；`grep -c 'run-llm-live' .github/workflows/ci.yml` → `0`；`grep -nE 'LLM_(LIVE|API_KEY|BASE_URL|MODEL|MODEL_CONFIRMED|ALLOWLIST)' .github/workflows/ci.yml | wc -l` → `0`

  - [x] **W-CI-10.2 列举断言（编译覆盖 + 收集证据 + ignored 计数）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-15/17 · design 2.1.3-D、5.4-I-5（`grep -c` 0 匹配退出 1 的陷阱）
    - DoD：`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live_baseline --target "$LLM_HTTPS_TARGET" -- --list` 输出中 `^baseline_30_samples_per_task: test$` 命中 ≥1；`--test llm_live -- --list --ignored` 的 `: test$` 计数 ≥8；`--test llm_live_baseline -- --list --ignored` 计数 ≥1；所有计数命令统一 `|| true` 后再 `test -ge`；步骤末尾打印 `llm-https: compiled=1 platform=… collected=… ignored_live=… ignored_baseline=…`
    - 证据：`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live_baseline -- --list 2>&1 | grep -cE '^baseline_30_samples_per_task: test$'` → `1`；`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --list --ignored 2>&1 | grep -cE ': test$'` → `≥8`；`… --test llm_live_baseline -- --list --ignored 2>&1 | grep -cE ': test$'` → `≥1`；`grep -nE 'grep -cE .*: test\$' .github/workflows/ci.yml | grep -c '|| true'` ≥ `3`（I-5 防御）

  - [x] **W-CI-10.3 coverage artifact 写入（两行契约）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-15 · design 2.2.2-I-4、2.1.3-D 的 artifact 片段
    - DoD：写入 `${RUNNER_TEMP}/llm-https-coverage-${LLM_HTTPS_TARGET}.txt`，内容恰两行：① `llm-https: compiled=1 platform=<triple> feature=llm-https collected=<n> ignored_live=<n> ignored_baseline=<n>`；② `collect: meeting-copilot-infrastructure::llm_live_baseline::baseline_30_samples_per_task: test`；字段值不含空格；失败时不写入（编译失败不伪造 `compiled=0`）
    - 证据：`grep -c 'llm-https-coverage-${LLM_HTTPS_TARGET}.txt' .github/workflows/ci.yml` → `1`；`grep -c 'collect: meeting-copilot-infrastructure::llm_live_baseline::baseline_30_samples_per_task: test' .github/workflows/ci.yml` → `1`；本地模拟写入后 `wc -l` → `2`

- [x] **W-CI-11 artifact 上传步骤** · `.github/workflows/ci.yml`
  - 追溯：AC-CI-15/18 · design 2.2.2-I-3 的 S-6
  - DoD：`if: always()`、`actions/upload-artifact@v4`、`name: llm-https-coverage-${{ matrix.target }}`、`path: ${{ runner.temp }}/llm-https-coverage-*.txt`、`if-no-files-found: warn`；继承 job 级超时。子任务 11.1~11.2 全达成
  - 证据：`grep -n -A6 'llm-https-coverage-upload (C2)' .github/workflows/ci.yml` 含 `if: always()` 与 `upload-artifact@v4`

  - [x] **W-CI-11.1 上传步骤形态** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-15 · design 2.1.3-D（`llm-https-coverage-upload` 片段）
    - DoD：字段与取值逐字符合 design；`if-no-files-found: warn`（缺失由 S-7 断言捕获，不在此处 fail）
    - 证据：`grep -c 'name: llm-https-coverage-\${{ matrix.target }}' .github/workflows/ci.yml` → `1`；`grep -c 'if-no-files-found: warn' .github/workflows/ci.yml` ≥ `1`

  - [x] **W-CI-11.2 三个 triple 各有 1 组 artifact** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-18 · design 2.1.3-D、spec 6.4-1
    - DoD：`rust` 矩阵恰 3 个 target（`x86_64-pc-windows-msvc`、`aarch64-apple-darwin`、`x86_64-apple-darwin`），每个任务产出 1 个 artifact；矩阵 `fail-fast: false` 保留
    - 证据：`grep -cE '^\s+- os: (windows-2022|macos-latest)$' .github/workflows/ci.yml` → `3`；`grep -c 'target: aarch64-apple-darwin' .github/workflows/ci.yml` → `1`；`grep -c 'fail-fast: false' .github/workflows/ci.yml` → `1`

- [x] **W-CI-12 新增 `feature-coverage (C2)` 汇总 job** · `.github/workflows/ci.yml`
  - 追溯：AC-CI-18 · design 2.1.3-D（`feature-coverage` job 片段）、2.1.2 配置表
  - DoD：job 名 `feature-coverage (C2)`、`needs: rust`、`if: always()`、`runs-on: ubuntu-latest`、`timeout-minutes: 5`；断言"3 个 triple 各一行 + 至少 1 行 `compiled=1` + 至少 1 行 `collect:`"；Linux 行含 `不适用` 与 `reason=no_linux_runner_in_matrix`；与既有 job 无其它 `needs`。子任务 12.1~12.3 全达成
  - 证据：见 12.1~12.3

  - [x] **W-CI-12.1 job 骨架与 artifact 下载** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-18 · design 2.1.3-D
    - DoD：`needs: rust` + `if: always()`（rust 失败时仍执行，artifact 缺失使 `download-artifact` 显式失败而非 skip）；`download-artifact@v4` 的 `pattern: llm-https-coverage-*`、`path: coverage`、`merge-multiple: true`；job 级 `timeout-minutes: 5`
    - 证据：`grep -n -A8 'feature-coverage (C2)' .github/workflows/ci.yml` 含 `needs: rust` 与 `if: always()`；`grep -c 'merge-multiple: true' .github/workflows/ci.yml` → `1`

  - [x] **W-CI-12.2 `assert-feature-coverage (C2)` 三断言** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-15/18 · design 2.1.3-D 的 S-7
    - DoD：`timeout-minutes: 3`、`set -euo pipefail`、`shopt -s nullglob`；① 文件数 ≥1；② 三 triple 每个都能 `grep -hq "platform=${triple} "`；③ `compiled=1` 行数 ≥1；④ 存在 `^collect: .*::baseline_30_samples_per_task: test$`；任一不满足退 `1` 并输出 `llm-https: missing platform=<triple>`
    - 证据：`grep -n -A14 'assert-feature-coverage (C2)' .github/workflows/ci.yml` 含 `shopt -s nullglob` 与 `baseline_30_samples_per_task`；`grep -c 'compiled=1' .github/workflows/ci.yml` ≥ `2`（写 artifact + 断言）

  - [x] **W-CI-12.3 Linux"不适用"声明行** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-18 · design 2.0-U1、2.1.3-D 的 S-4、spec 4.4-1
    - DoD：步骤 `llm-https-linux-not-applicable (C2)`，`timeout-minutes: 1`，输出逐字为 `llm-https: compiled=0 platform=x86_64-unknown-linux-gnu 不适用 reason=no_linux_runner_in_matrix`；不新增 Linux rust job
    - 证据：`grep -c 'llm-https-linux-not-applicable (C2)' .github/workflows/ci.yml` → `1`；`grep -c 'compiled=0 platform=x86_64-unknown-linux-gnu 不适用 reason=no_linux_runner_in_matrix' .github/workflows/ci.yml` → `1`；`grep -cE 'runs-on: ubuntu-latest' .github/workflows/ci.yml` 不因 C2 增加 rust 任务（`rust` job 的 `matrix.include` 仍 3 项）

- [x] **W-CI-13 P2 本地断言与 X9 静态审计** · `.github/workflows/ci.yml`
  - 追溯：AC-CI-16/17/19 · design 5.3（P2 证据）、5.4-I-3（X9 误删断言风险）
  - DoD：Windows 本机三条 cargo 断言达成；X9 审计 0 命中；禁项 grep 0 命中。子任务 13.1~13.3 全达成
  - 证据：见 13.1~13.3

  - [x] **W-CI-13.1 Windows 本机三条 cargo 断言** · `crates/infrastructure/tests/llm_live_baseline.rs`
    - 追溯：AC-CI-15/17 · design 4.2（AC-CI-15/17 行）、4.4
    - DoD：`baseline_30_samples_per_task: test` 命中 = 1；`llm_live` ignored 计数 ≥8；`llm_live_baseline` ignored 计数 ≥1；三者连续 3 次一致
    - 证据：
      ```bash
      export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
      export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
      cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live_baseline -- --list 2>&1 | grep -cE '^baseline_30_samples_per_task: test$'
      cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --list --ignored 2>&1 | grep -cE ': test$'
      cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live_baseline -- --list --ignored 2>&1 | grep -cE ': test$'
      ```
      期望 `1` / `≥8` / `≥1`

  - [x] **W-CI-13.2 X9 裁决后的审计命令（0 命中）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-16/17 · design 2.0-X9、5.4-I-3
    - DoD：审计命令必须过滤 `--list --ignored`，剩余执行形态 0 命中；严禁使用"未过滤"版本自证违规后删掉 ignored 断言
    - 证据：`grep -nE -- '--(ignored|nocapture|test-threads)' .github/workflows/ci.yml | grep -v -- '--list --ignored' | wc -l` → `0`；`grep -c -- '-- --list --ignored' .github/workflows/ci.yml` ≥ `2`（live 与 baseline 两条）

  - [x] **W-CI-13.3 禁项与定向 feature 审计** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-16/19 · design 2.0-X6、2.1.3-D
    - DoD：`LLM_*` 联调变量 0；`--all-features` 0；`run-llm-live` 0；`sqlcipher` job 未被触碰
    - 证据：
      ```bash
      grep -nE 'LLM_(LIVE|API_KEY|BASE_URL|MODEL|MODEL_CONFIRMED|ALLOWLIST)' .github/workflows/ci.yml | wc -l   # 0
      grep -c 'all-features' .github/workflows/ci.yml            # 0
      grep -c 'run-llm-live' .github/workflows/ci.yml            # 0
      grep -n -A12 'sqlcipher (page-level encryption)' .github/workflows/ci.yml | grep -c 'llm-https'   # 0
      ```

## 六、阶段 P3：C3 差集门禁独立 job（强制依赖 P0）

**阶段目标**：差集门禁在 CI 中独立生效；退 `1`（真 removed）与退 `2`（扫描面缺失/不可比）均使 job 失败；平台绑定保证不产生 365 条误报。
**覆盖**：AC-CI-20~24、AC-CI-26、AC-CI-27。
**前置**：**P0 必须先落地**（design 5.2-1：先接 C3 会把环境故障报成全量 removed）。
**强制禁项**：`ci.yml` 全文不得出现基线生成子命令字样（含注释与 step 名）。

- [x] **W-CI-14 新增 `test-name-inventory (C3)` job** · `.github/workflows/ci.yml`
  - 追溯：AC-CI-20/22/24 · design 2.1.3-E、2.0-U2
  - DoD：job 名 `test-name-inventory (C3)`、`runs-on: windows-2022`、`timeout-minutes: 45`、**无 `needs`**；所有 `run` 显式 `shell: bash`；步骤顺序 = checkout → toolchain → cache → node → web-assets → baseline-presence → diff。子任务 14.1~14.4 全达成
  - 证据：见 14.1~14.4

  - [x] **W-CI-14.1 job 头与依赖隔离** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-20/24 · design 2.1.3-E（"与既有 4 job 无 needs"）、spec 4.2-3
    - DoD：`runs-on: windows-2022`、`timeout-minutes: 45`、job 内无 `needs:`（不被其它 job 失败掩盖，也不掩盖其它 job）
    - 证据：`grep -n -A5 'test-name-inventory (C3)' .github/workflows/ci.yml` 含 `timeout-minutes: 45` 且其后 6 行内无 `needs:`；`grep -c 'runs-on: windows-2022' .github/workflows/ci.yml` → `2`（rust 矩阵 + C3）

  - [x] **W-CI-14.2 工具链、缓存、前端产物（`shell: bash` 强制）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-24 · design 2.1.3-E、5.4-I-2（pwsh 风险）
    - DoD：`dtolnay/rust-toolchain@1.100.0` 带 `targets: x86_64-pc-windows-msvc`；`Swatinem/rust-cache@v2` 的 `key: inventory-x86_64-pc-windows-msvc`、`workspaces: ". -> target"`；node 22 + npm 缓存；`web-assets` 步骤显式 `shell: bash` + `set -euo pipefail`，执行 `npm ci --no-audit --no-fund && npm run build`
    - 证据：`grep -n -A6 'name: web-assets' .github/workflows/ci.yml | grep -c 'shell: bash'` ≥ `2`（rust 段 + C3 段各 1）；`grep -c 'key: inventory-x86_64-pc-windows-msvc' .github/workflows/ci.yml` → `1`

  - [x] **W-CI-14.3 `baseline-presence (C3)` 基线可用性（超时 2 分钟）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-22 · design 2.1.3-E-2、2.2.2-I-3 的 S-9
    - DoD：`timeout-minutes: 2`、`shell: bash`；`test -s docs/progress/test-names-before.txt` 与 `wc -l` 回显；输出 `baseline: path=docs/progress/test-names-before.txt lines=<n>`；空/缺即步骤失败
    - 证据：`grep -n -A6 'name: baseline-presence (C3)' .github/workflows/ci.yml` 含 `timeout-minutes: 2` 与 `test -s docs/progress/test-names-before.txt`；本地 `wc -l < docs/progress/test-names-before.txt` → `365`；`git ls-files --error-unmatch docs/progress/test-names-before.txt` 退出 `0`

  - [x] **W-CI-14.4 `test-name-inventory-diff (C3)` 平台戳与超时 40** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-20/21/23/24 · design 2.1.3-E（env 两个平台戳）、2.2.2-I-3 的 S-8
    - DoD：`timeout-minutes: 40`、`shell: bash`、`env: INVENTORY_PLATFORM: x86_64-pc-windows-msvc` 与 `INVENTORY_BASELINE_PLATFORM: x86_64-pc-windows-msvc`；`run` 逐字为 `bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt`；无吞错修饰
    - 证据：`grep -n -A8 'name: test-name-inventory-diff (C3)' .github/workflows/ci.yml` 含两行 env 与 `timeout-minutes: 40`；`grep -c 'test-name-inventory.sh --diff docs/progress/test-names-before.txt' .github/workflows/ci.yml` → `1`；该段 10 行内 `grep -cE '\|\| true|continue-on-error'` → `0`

- [x] **W-CI-15 基线四态失败格式验收** · `scripts/test-name-inventory.sh`
  - 追溯：AC-CI-21/23 · design 2.1.3-E-4（四态表）、2.1.3-A-9
  - DoD：0 / 1 / 2（基线缺失）/ 2（平台不可比）四态输出与退出码严格符合 design 四态表；1 与 2 互斥。子任务 15.1~15.4 全达成
  - 证据：四态各自 `echo rc=$?` + `head -1` 记录，落 `$tmp/p3-evidence.txt`

  - [x] **W-CI-15.1 态一：一致 → 退 0** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-21 · design 2.1.3-E-4 第 1 行
    - DoD：输出 `PASS: removed=0，新增 0 条`（新增为 0 时）或 `新增 N 条` + `baseline_drift=N`；退 `0`
    - 证据：`CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt; echo rc=$?` → `0` + `removed=0`

  - [x] **W-CI-15.2 态二：真 removed → 退 1 + 明细** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-21 · design 2.1.3-E-4 第 3 行、spec 5.1.3-7 场景
    - DoD：首行 `docs/progress/test-names-before.txt:0 出现 removed 用例`（或临时基线路径 `:0`），随后逐行 removed 明细 ≥1；退 `1`；输出无 `removed=`
    - 证据：
      ```bash
      { cat docs/progress/test-names-before.txt; echo 'meeting-copilot-nope::lib::ghost_case'; } | LC_ALL=C sort > "$tmp/base_extra.txt"
      CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff "$tmp/base_extra.txt" >"$tmp/o" 2>&1; echo rc=$?
      head -1 "$tmp/o" | grep -q '出现 removed 用例' && echo first-line=ok
      echo "removed_eq_hits=$(grep -c 'removed=' "$tmp/o")"
      echo "detail_lines=$(tail -n +2 "$tmp/o" | grep -c 'meeting-copilot')"
      ```
      期望 `rc=1` / `first-line=ok` / `removed_eq_hits=0` / `detail_lines>=1`

  - [x] **W-CI-15.3 态三：基线缺失/为空 → 退 2** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-22 · design 2.1.3-E-4 第 4 行、R-4
    - DoD：首行为 `<基线相对路径>:0 扫描面缺失 baseline_missing`；退 `2`；不进入抓取计算
    - 证据：`: > "$tmp/empty.txt"; CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff "$tmp/empty.txt" >"$tmp/o" 2>&1; echo rc=$?` → `2`；`grep -c 'baseline_missing' "$tmp/o"` ≥ `1`；`grep -c 'removed=' "$tmp/o"` → `0`

  - [x] **W-CI-15.4 态四：平台不可比 → 退 2** · `scripts/test-name-inventory.sh`
    - 追溯：AC-CI-23 · design 2.1.3-E-4 第 5 行
    - DoD：首行 `不可比 exec=<...> baseline=<...>`；退 `2`；不输出 removed 结论
    - 证据：`INVENTORY_PLATFORM=aarch64-apple-darwin CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt >"$tmp/o" 2>&1; echo rc=$?` → `2`；`grep -c '不可比' "$tmp/o"` ≥ `1`；`grep -c 'removed=' "$tmp/o"` → `0`

- [x] **W-CI-16 C3 禁项与编排纪律静态审计** · `.github/workflows/ci.yml`
  - 追溯：AC-CI-20/24 · design 2.1.3-E-5、2.1.3-F、5.4-I-4
  - DoD：`ci.yml` 全文无基线生成子命令字样；差集调用唯一且无吞错；6 个 job 均有 job 级超时、全文 `continue-on-error` 计数 0、C3/C2 的 `run` 均 `shell: bash`。子任务 16.1~16.3 全达成
  - 证据：见 16.1~16.3

  - [x] **W-CI-16.1 基线生成禁项（全文 0 命中）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-20 · design 2.1.3-E-5、5.4-I-4
    - DoD：`emit-baseline` 在 `ci.yml`（含注释、step 名）出现 0 次；差集调用恰好 1 处
    - 证据：`grep -cE 'emit-baseline' .github/workflows/ci.yml` → `0`；`grep -c 'test-name-inventory.sh --diff' .github/workflows/ci.yml` → `1`

  - [x] **W-CI-16.2 差集步骤无吞错修饰** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-20/21 · design 2.1.3-F
    - DoD：差集步骤块内无 `|| true`、`continue-on-error`、`set +e`、`if: false`、`if: always()`（`feature-coverage` 的 `if: always()` 属 job 级汇总，不在此处）
    - 证据：`grep -n -A10 'name: test-name-inventory-diff (C3)' .github/workflows/ci.yml | grep -cE '\|\| true|continue-on-error|set \+e|if: false'` → `0`

  - [x] **W-CI-16.3 编排纪律（6 job 超时 / 无 continue-on-error / shell 显式）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-24 · design 2.1.3-F、R-2（job 数取更严格者 6）
    - DoD：6 个 job 均声明 `timeout-minutes`；全文 `continue-on-error` 计数 0；`shell: bash` ≥5 处（既有 1 + C2 1 + C3 3）；C3 每个 `run` 步骤均有 `shell: bash`
    - 证据：
      ```bash
      grep -cE '^  (web|rust|compliance|sqlcipher|feature-coverage|test-name-inventory):$' .github/workflows/ci.yml   # 6
      grep -c 'continue-on-error' .github/workflows/ci.yml        # 0
      grep -c 'shell: bash' .github/workflows/ci.yml              # ≥5
      grep -c 'timeout-minutes' .github/workflows/ci.yml           # ≥ 6 job + 新增 step 数
      ```

- [x] **W-CI-17 既有门禁不回退（跨 P0~P3 全量回归）** · `.github/workflows/ci.yml`、`scripts/`
  - 追溯：AC-CI-24/27 · design 3.1（影响面）、3.2（兼容结论）、5.4-I-2
  - DoD：Rust 门禁全绿；静态门禁全绿；用例数与用例名不回退；既有 4 job 命令逐字不变；`docs/sdd/**` 零改动。子任务 17.1~17.4 全达成，任一失败即禁止进入 W-CI-18
  - 证据：见 17.1~17.4

  - [x] **W-CI-17.1 Rust 门禁：fmt / clippy / build / test** · `crates/`
    - 追溯：AC-CI-24 · design 3.1（`rust` job 既有步骤逐字不变）、`docs/progress/ci-readiness.md` 13.1
    - DoD：`fmt` 退出 `0`；`clippy` 退出 `0`（`-D warnings`）；`build` 退出 `0`；`test` 退出 `0` 且 **399 passed / 0 failed / 8 ignored / 38 targets**
    - 证据：
      ```bash
      export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
      export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
      cargo fmt --all --check; echo "fmt_rc=$?"
      cargo clippy --workspace --all-targets -- -D warnings; echo "clippy_rc=$?"
      cargo build --workspace; echo "build_rc=$?"
      cargo test --workspace --no-fail-fast 2>&1 | tail -40
      ```
      期望 `fmt_rc=0`、`clippy_rc=0`、`build_rc=0`；汇总行 `399 passed`、`8 ignored`

  - [x] **W-CI-17.2 静态门禁：layering / secrets / deny / capture-offline / g2-claims** · `scripts/`
    - 追溯：AC-CI-25 · design 3.2、`docs/progress/ci-readiness.md` 13.1
    - DoD：`check-layering.sh` 退 `0`；`scan-secrets.sh` 退 `0`；`cargo deny --offline check licenses bans sources advisories` 退 `0`；`check-capture-offline.sh --all` 退 `0`；`check-g2-claims.sh` 退 `0`
    - 证据：
      ```bash
      bash scripts/check-layering.sh; echo "layering_rc=$?"
      bash scripts/scan-secrets.sh; echo "secrets_rc=$?"
      cargo deny --offline check licenses bans sources advisories; echo "deny_rc=$?"
      bash scripts/check-capture-offline.sh --all; echo "capture_rc=$?"
      bash scripts/check-g2-claims.sh; echo "g2_rc=$?"
      ```
      期望全部 `rc=0`

  - [x] **W-CI-17.3 用例名与用例数不回退** · `docs/progress/test-names-before.txt`
    - 追溯：AC-CI-09/27 · design 3.2-1、G-9
    - DoD：`--diff` 退 `0` 且 `removed=0`；基线仍 365 行；无任何既有测试用例被删除/改名/`#[ignore]` 化
    - 证据：`CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt; echo rc=$?` → `0` + `removed=0`；`wc -l < docs/progress/test-names-before.txt` → `365`；`git status --short docs/progress/test-names-before.txt` → 空

  - [x] **W-CI-17.4 既有 4 job 命令逐字不变 + `docs/sdd` 零改动** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-27 · design 3.1（统一回滚原则）、spec 1.4-1
    - DoD：`web` / `rust` / `compliance` / `sqlcipher` 的既有 step `run` 行无删改（`compliance` 的 `checkout` 增加 `fetch-depth` 与三条新 step 属允许新增）；`git status --short docs/sdd` 与 `git diff --stat docs/sdd` 均为空
    - 证据：`git diff -U0 .github/workflows/ci.yml | grep -cE '^-'` 中删除行仅允许 `- uses: actions/checkout@v4`（compliance 段被 `+ with: fetch-depth: 0` 替换）与空行/上下文；`git status --short docs/sdd` → 空；`git diff --stat docs/sdd` → 空

- [x] **W-CI-18 L3 远端观测与本地/远端分界** · `docs/progress/ci-readiness.md`
  - 追溯：AC-CI-26 · design 4.3（本地与远端分界）、5.1（L3 验收行）
  - DoD：推送后按第 7 章清单逐项取远端证据；本地结论与远端结论逐项比对，差异有根因登记；**禁止**把本地模拟记为远端通过。子任务 18.1~18.2 全达成
  - 证据：见 18.1~18.2

  - [x] **W-CI-18.1 六 job 结论与标记串观测（远端）** · `.github/workflows/ci.yml`
    - 追溯：AC-CI-26/24 · design 4.3、5.3（L3）
    - DoD：`web` / `rust`（3 矩阵）/ `compliance` / `sqlcipher` / `feature-coverage` / `test-name-inventory` 全部结论可读；日志中逐条 grep 到 `扫描面缺失`、`出现 removed 用例`、`removed=0`、`llm-https: compiled=`、`不适用` 的出现位置；任一 job 为红则必须定位到缺口编号
    - 证据：在 Actions 页面导出各 job 日志后：`grep -c 'llm-https: compiled=1' rust-*.log` ≥ `1`；且对每个 triple 执行 `grep -c 'platform=<triple>' rust-*.log` → 各 `≥1`（AC-CI-18 逐平台）；`grep -c '不适用' feature-coverage.log` ≥ `1`；`grep -c 'removed=0' test-name-inventory.log` ≥ `1`；`grep -c 'baseline: path=docs/progress/test-names-before.txt lines=365' test-name-inventory.log` → `1`

  - [x] **W-CI-18.2 本地/远端差异登记** · `docs/progress/ci-readiness.md`
    - 追溯：AC-CI-26 · design 4.3、5.1（L3 交付物）
    - DoD：新增一节记录：本地退出码 + 首行 vs 远端日志；差异项给出根因（工具链/平台/缓存/扫描面）；残余风险 R4/R5/R7 状态更新；不宣称 G2 通过
    - 证据：`git diff docs/progress/ci-readiness.md` 含新增小节标题与差异表；`grep -c 'AC-CI-26' docs/progress/ci-readiness.md` ≥ `1`；`git status --short docs/sdd` → 空

## 七、只能由远端 CI 判定的项（本地能验到哪一步）

> **总则**：本地模拟**不等于**远端通过。下表每一行都必须在 W-CI-18.1 取得 Actions 证据后才可标记为达成；本地只提供"静态正确性"与"命令可执行性"证据。

| 编号 | 远端才能判定的项 | 本地最多能验到 | 远端判定方式 | 对应任务 |
|---|---|---|---|---|
| L3-1 | 6 个 job 在 Actions 上实际执行且结论为绿 | 静态读 `ci.yml`（结构、名称、超时、无吞错） | job 结论徽标 / Runs 列表 | W-CI-16.3、18.1 |
| L3-2 | `compliance` 三条 C1 步骤在 runner 上真实跑通 | 脚本本地退出码 0/1/2 + 缺失面构造 | `compliance` 日志中三个步骤的 ✓ | W-CI-8、9、18.1 |
| L3-3 | `fetch-depth: 0` 后 `git log -20` 实际可扫满 20 条 | 只能验 YAML 存在该参数 | `g2-claims` 步骤日志 / checkout 深度 | W-CI-8.1、18.1 |
| L3-4 | `llm-https` 在 **macOS** 两矩阵编译成功（R2/事实 F1 的实际验证） | 仅 Windows host 可验（`CARGO` 绝对路径） | `rust (aarch64-apple-darwin)` / `rust (x86_64-apple-darwin)` 日志中 `compiled=1` | W-CI-10.2、13.1、18.1 |
| L3-5 | artifact 上传与跨 job 下载（`coverage/` 合并 3 个文件） | 只能验路径字符串与两行内容格式 | Actions Artifacts 面板 + `assert-feature-coverage` 日志 | W-CI-11、12.1、18.1 |
| L3-6 | Linux"不适用"行在汇总 job 日志中可见 | 静态断言该 echo 字符串存在 | `feature-coverage` 日志含 `不适用 reason=no_linux_runner_in_matrix` | W-CI-12.3、18.1 |
| L3-7 | C3 在 `windows-2022` runner 上全量编译 + 差集耗时是否 ≤40 分钟 | 本地可计时，但 runner 性能/缓存命中不同，不足为证 | `test-name-inventory-diff (C3)` 步骤耗时 | W-CI-14.4、18.1 |
| L3-8 | `windows-2022` 上 `shell: bash` 的 `set -e` 语义与 `npm ci` 中断行为（I-2） | 只能静态确认 `shell: bash` 已写 | `web-assets` 步骤日志与失败传播 | W-CI-14.2、18.1 |
| L3-9 | `feature-coverage` 在 rust 失败时是否仍执行（`if: always()` 的实际效果） | 静态读 YAML | 人为/自然失败时的 job 关系图 | W-CI-12.1、18.1 |
| L3-10 | CI 分钟数成本与缓存命中率（R7） | 不可判定 | Actions 计费面板 / 步骤耗时 | W-CI-14.1、18.2 |

**本地可完整判定（无需远端）**：AC-CI-01~09、14、15、17、19、21、22、23（平台戳注入）、25、27，以及 C1 脚本自身 0/1/2 与 C3 的 `removed=0`（design 4.3 表）。

## 八、任务依赖图与关键路径

```
P0: W-CI-1.2（探针）
      └─> W-CI-2.1（list_target 落盘）
            └─> W-CI-2.2【强制：inventory 收口重构】
                  ├─> W-CI-2.3 / 2.4 / 2.5
                  ├─> W-CI-3.1 / 3.2 / 3.3
                  ├─> W-CI-4.1 / 4.2 / 4.3
                  └─> W-CI-5.1 / 5.2 / 5.3
                        └─> W-CI-6.1 / 6.2 / 6.3（P0 三条硬判据）+ 6.4
                              ├─> P1: W-CI-7.x ─> W-CI-8.x ─> W-CI-9.x
                              ├─> P2: W-CI-10.x ─> W-CI-11.x ─> W-CI-12.x ─> W-CI-13.x
                              └─> P3: W-CI-14.x ─> W-CI-15.x ─> W-CI-16.x
                                        └─> W-CI-17.x（全量回归，门）─> W-CI-18.x（L3）
```

| 关系 | 依据 | 说明 |
|---|---|---|
| W-CI-2.2 是 P0 的**必经节点** | design 5.4-I-1 | 不重构收口，T2 无法在真实仓库触发，AC-CI-02 静默失效 |
| P0 → P1 | design 5.2-2 | 脚本行为未定前不写死 CI 命令与断言 |
| P0 → P2 | design 5.2-2 | 同上（`diag` 风格统一） |
| **P0 → P3（强制）** | design 5.2-1 | 先接 C3 会把环境故障报成全量 removed |
| W-CI-10 → W-CI-11 → W-CI-12 | design 5.2-4 | 先建汇总 job 会因无 artifact 恒红 |
| P1 → P2 → P3 串行 | design 5.2-3 | 每次失败可唯一归因到缺口编号；P2/P3 无技术依赖但同文件需避免冲突 |
| W-CI-17 是 W-CI-18 的门 | 本清单 G-8/G-9 | 既有门禁未全绿时不得宣称 L3 达成 |

**关键路径（最长链）**：`W-CI-1.2 → W-CI-2.1 → W-CI-2.2 → W-CI-5.3 → W-CI-6.3 → W-CI-10.x → W-CI-12.2 → W-CI-14.4 → W-CI-17.1 → W-CI-18.1`。

## 九、AC-CI-01~27 覆盖矩阵

| AC | 验收要点 | 主任务 / 子任务 | 判定层 |
|---|---|---|---|
| AC-CI-01 | cargo 不可用 → 2 + 首行 | W-CI-1.2、5.1(T1)、6.1 | L1+L2 |
| AC-CI-02 | 单面失败 → 2 + 面标识 | W-CI-2.1、2.2、5.1(T2) | L1+L2 |
| AC-CI-03 | 空清单 → 2 | W-CI-2.4、5.1(T3) | L1+L2 |
| AC-CI-04 | 基线原子写 | W-CI-3.2、5.2(T5) | L1 |
| AC-CI-05 | 仓库扫描面缺失 → 2 | W-CI-2.3、5.2(T4) | L1+L2 |
| AC-CI-06 | 基线缺失/不可读/未排序 → 2 | W-CI-3.1、3.3 | L2 |
| AC-CI-07 | 真 removed → 1 + 明细；1 与 2 互斥 | W-CI-2.5、15.2、16.2 | L2 |
| AC-CI-08 | 自检双向 6 断言 | W-CI-5.1~5.3、6.3 | L1 |
| AC-CI-09 | 正常态 0 + `removed=0` | W-CI-4.3、6.2、15.1 | L2 |
| AC-CI-10 | 三条调用存在 + 不吞错 + fetch-depth | W-CI-8.1~8.4、9.1 | L2 静态 + L3 |
| AC-CI-11 | 命中即失败 | W-CI-7.3、9.1 | L1+L3 |
| AC-CI-12 | 扫描面缺失即失败 | W-CI-7.1、7.2、9.2 | L1+L3 |
| AC-CI-13 | 脚本缺失不静默 | W-CI-9.3 | L2 静态 |
| AC-CI-14 | 文档扫描不回显 | W-CI-6.4、17.2 | L1 |
| AC-CI-15 | feature 生效证据（`baseline_30_samples_per_task: test`） | W-CI-10.2、10.3、12.2、13.1 | L2+L3 |
| AC-CI-16 | 不运行真实用例 / 不注入凭据 | W-CI-10.1、13.2、13.3 | L2 静态 |
| AC-CI-17 | ignored 数量不减（8 / 1） | W-CI-10.2、13.1 | L2+L3 |
| AC-CI-18 | 平台能力显式化 + 至少 1 条 compiled=1 | W-CI-11.2、12.1~12.3、13.2 | L3 主判 |
| AC-CI-19 | 只编译不执行 + 不用 `--all-features` | W-CI-10.1、13.3 | L2 静态 |
| AC-CI-20 | 差集步骤存在 + 无基线生成 | W-CI-14.4、16.1、16.2 | L2 静态 + L3 |
| AC-CI-21 | 1/2 均失败、0 通过 | W-CI-15.1~15.4 | L2+L3 |
| AC-CI-22 | 基线在 CI 中可用（365 行） | W-CI-14.3、17.3 | L2 静态 + L3 |
| AC-CI-23 | 平台可比性 | W-CI-4.1、4.2、15.4 | L2+L3 |
| AC-CI-24 | 编排纪律（6 job 超时 / 无 continue-on-error） | W-CI-8.2~8.4、10.1、14.1、16.3 | L2 静态 |
| AC-CI-25 | 退出码三分一致 | W-CI-7.1、7.2、15.1~15.4、17.2 | L2 |
| AC-CI-26 | 离线/远端复跑一致 | W-CI-18.1、18.2 | L3 |
| AC-CI-27 | 文档边界（`docs/sdd` 不变） | W-CI-17.4、18.2 | L2 |

## 十、阶段门总表

| 阶段 | 主任务 | 覆盖 AC | 前置 | 出口硬判据（全部满足才可进入下一阶段） |
|---|---|---|---|---|
| **P0** | W-CI-1~6（6 主 / 21 子） | 01~09、23、25(C4) | 无 | ① `--self-test` → `0` 且 `SELF-TEST: 6/6 ok`；② 127 桩 `--diff` → `2` + 首行匹配 + `removed=` 计数 0（连跑 3 次一致）；③ 真实 `--diff` → `0` 且 `removed=0` |
| **P1** | W-CI-7~9（3 主 / 10 子） | 10~14、25(C1) | P0 | 三脚本本地全绿；缺失面构造 → `2`；9.1 的 6 条静态 grep 全部符合 |
| **P2** | W-CI-10~13（4 主 / 11 子） | 15~19 | P0 | Windows 本机三条 cargo 断言 `1 / ≥8 / ≥1`；X9 审计 0 命中；Linux 行含 `不适用` |
| **P3** | W-CI-14~17（4 主 / 15 子） | 20~24、27 | **P0（强制）** | 本地 `--diff` → `0`；`INVENTORY_PLATFORM=aarch64-apple-darwin` → `2`；16.1~16.3 静态审计全符合；17.1~17.4 全绿 |
| **L3** | W-CI-18（1 主 / 2 子） | 26 | P0~P3 合并 | 第 7 章 10 项远端证据齐备；本地/远端差异有根因登记；`docs/sdd` 哈希不变 |

**统计**：4 个阶段 + L3，共 **18 个主任务 / 59 个子任务**；AC-CI-01~27 全部映射（见第九章），无遗漏、无孤立 AC。