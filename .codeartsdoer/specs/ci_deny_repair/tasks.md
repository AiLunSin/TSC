# ci_deny_repair · 编码任务清单（CI cargo-deny 四类合规修复）

> 阶段：第三阶段（编码任务规划）。本文件把 `.codeartsdoer/specs/ci_deny_repair/spec.md`（569 行，根因 R-1/R-2/R-3、AC-B4-105~110）与 `.codeartsdoer/specs/ci_deny_repair/design.md`（1056 行，§2.0~§2.9、D-B4-P135~P143、改动预算、验收用例表）翻译成可执行、可机器验收的任务；**不修改任何实现文件**。
> 唯一需求基线：spec §2.3 根因定案 + §6 核心能力 + §7 数据约束 + §8 AC-B4-105~110；与 design 冲突时以 AC 为准，处置写在对应子任务的"追溯/禁止"行。
> 设计基线：design §2.4 改动清单（`deny.toml` `1 0`、ui-bridge `2 2`、`Cargo.lock` `4 4`、`ci.yml` `2 0`）、§2.5 验收用例、§2.6 回滚矩阵、§2.7 门禁矩阵、§2.8 决策登记。
> 代码基线（2026-11-22 实测）：HEAD `90df24a`；工作区仅 `M docs/progress/b4-status.md`（§21 未提交）；`deny.toml` 49 行且**末尾无换行**、`allow` 15 项；`crates/ui-bridge/Cargo.toml` 第 11/12 行为无 `version` 的 path 依赖；`Cargo.lock` 中 `url 2.5.8`（L4432-4435）、`form_urlencoded 1.2.2`（L1013-1016）；`ci.yml` 第 239-240 行为 `licenses`；`NEW_TARGETS` 25 项。
> 本次**不新增** Rust target、不新增脚本、不新增 artifact、不改 `crates/**/src/**`、不改 `scripts/**`、不改根 `Cargo.toml`、不新增依赖。

---

## 0. 执行口径

### 0.1 环境前置（每个 shell 会话执行一次）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$HOME/.cargo/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
export EVID="$PWD/.tmp/ci-deny-repair/$(date +%Y%m%d)"; mkdir -p "$EVID"
```

- `cargo` 必须为 `1.100.0`、`cargo-deny` 必须为 `0.22.0`；`PATH` 前置 toolchain 目录是硬要求（否则 cargo-deny 内部 `cargo metadata` 失败，属环境问题，不计入 AC）。
- 证据目录 `$EVID` 位于 `.tmp/`（已 gitignore），**不得入库**。
- `advisories` 依赖网络或本地 advisory 缓存；离线时 AC-B4-107 记 `not-applicable`，不得宣称 `passed`。

### 0.2 编号与书写格式

- 主任务：`W-CID-<n>`（本文件全局递增 01..11，不与 `W-B4-*` 复用、不冲突）。子任务：`W-CID-<n>.<m>`。
- 每个子任务至少含：**追溯**（AC + spec/design 章节 + 决策编号）、**改动内容**、**依赖**、**禁止**、**DoD**（命令/退出码/grep 计数）、**证据**（可直接粘贴的 bash）、**证据产物**。
- 最多两级：`##` 为主任务，`- [ ]` 为子任务；不出现第三级复选框。
- 所有证据命令在项目根 `TSC/` 执行；`$CARGO`、`$EVID`、`$TEMP` 来自 §0.1。

### 0.3 冻结口径（用户已确认，执行时不再权衡）

| 编号 | 冻结内容 | 落点 |
|---|---|---|
| R-1 | `[licenses]` 增 `unused-allowed-license = "allow"`；`allow` 15 项原样保留 | W-CID-02 |
| R-2 | `url` 升 `>=2.5.9`；**接受** D-B4-P138：lock 为 **4 行**目标补丁（`url` + `form_urlencoded 1.2.2→1.2.3`）；禁止 `ignore`；禁止误降 `syn` | W-CID-04 |
| R-3 | `crates/ui-bridge/Cargo.toml` 两条 path 依赖补 `version = "0.1.0"`（version 在前） | W-CID-03 |
| C-1 | CI 在 `licenses` 前打印 `cargo deny --version`（`+2/-0`） | W-CID-06 |
| 边界 | `deny.toml +1`、ui-bridge `2 2`、`crates/**/src/**` 零 diff、零新增依赖；白名单外仍拒；`NEW_TARGETS=25` 不变 | W-CID-07 |
| D-1 | `b4-status.md` 追加 §22（AC-105~110 三态与证据） | W-CID-10 |

### 0.4 开工前置闸门（硬）

1. **先提交 §21**：当前 `git status --short` 为 `M docs/progress/b4-status.md`（§21 增量登记未提交）。必须先单独提交 §21，再开始实现，否则 `git diff --numstat HEAD` 类预算判据的基线包含文档改动、`git status` 空集断言不成立。
2. `git status --short` 必须为空；`git log -1 --format=%s` 必须为 §21 提交（`docs(b4): 登记 §21 ...`）。
3. `deny.toml` 必须仍为 49 行、`allow` 15 项、末尾无换行；`Cargo.lock` 中 `url` 必须仍为 `2.5.8`（若已被改动，先 `git checkout -- Cargo.lock`）。
4. 不满足以上任一条，**停止并上报**，不得以"先改再说"绕过。

### 0.5 改动预算与 diff 形态（验收硬指标）

| 文件 | 操作 | `git diff --numstat HEAD` | 关键内容 |
|---|---|---|---|
| `deny.toml` | 插入 1 行 | `1 0` | `unused-allowed-license = "allow"`，位置在 `confidence-threshold` 之后、`allow` 之前 |
| `crates/ui-bridge/Cargo.toml` | 替换 2 行 | `2 2` | 两条 path 依赖补 `version = "0.1.0"` |
| `Cargo.lock` | 替换 4 行 | `4 4` | 仅 `url`/`form_urlencoded` 的 `version` + `checksum` |
| `.github/workflows/ci.yml` | 插入 2 行 | `2 0` | `cargo-deny-version` step 紧邻 `licenses` 之前 |
| `docs/progress/b4-status.md` | 追加一节 | 不纳入预算 | 文件末尾追加 `## §22`，不重写 §21 |

> 说明：`deny.toml` 原文件末尾无换行，**必须保持**；若插入后出现整文件 diff（numstat 远大于 `1 0`），立即按 W-CID-02.1 的"禁止"项恢复后重做。

### 0.6 与既有门禁的关系（本次均不改这些门禁）

| 门禁 | 命令 | 本次是否改动 | 预期 | 判据来源 |
|---|---|---|---|---|
| **deny（修复对象）** | `cargo deny check licenses bans sources advisories` | 是（R-1/R-2/R-3） | 退 0；四条 `ok` | AC-B4-105/107/108/109 |
| **B3 六规则** | `bash scripts/check-b3-gates.sh` | 否 | 退 0 | `crates/**` 零 diff |
| **B4 七规则** | `bash scripts/check-b4-gates.sh --rule=<r>` | 否 | 7×0；`--rule=targets` 空输出退 0 | 无新增 target、无源码改动 |
| **B4 self-test** | `bash scripts/check-b4-gates.sh --self-test` | 否 | 退 0 | 脚本零 diff |
| **layering** | `bash scripts/check-layering.sh` | 否 | `PASS` 退 0 | 只补 `version` 属性，不改依赖边 |
| **test-name-inventory** | `bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt` | 否 | 退 0，`removed=0` | lock 版本变化不产生 target |
| **secrets / secrets-docs** | `bash scripts/scan-secrets.sh`、`bash scripts/scan-secrets-docs.sh --all-md` | 否 | 退 0 | 本次不写密钥；§22 **禁止任何 URL** |
| **SBOM** | `bash scripts/sbom.sh ci` | 否 | 退 0；`url`/`form_urlencoded` 版本变化属预期 | `Cargo.lock` 下游 |
| **rust 静态 / 测试** | `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` | 否 | 0 / 0 / `failed=0`、`ignored=9` | 无 `.rs` 改动，复跑为旁证 |

> **关键约束（本次新增登记，源自实测读取脚本）**：`check-b4-gates.sh` 的 `rule_zero_transcript()` 会把 `docs/progress/b4-status.md` 纳入扫描面，并拒绝 `https?://`、`sk-` 长串、`Bearer`、模型名前缀、`request_id`。因此 **§22 内不得出现任何 URL**（含注册表地址、公告链接），只能用文字描述；否则 W-CID-08 与 W-CID-10.3 会失败。

### 0.7 AC-B4-105~110 映射总表

| AC | 陈述要点 | 主任务 | 关键判据 |
|---|---|---|---|
| **AC-B4-105** | `[licenses]` 含 `unused-allowed-license="allow"`；`licenses` 退 0、未命中 0；`allow` 15 项不变 | W-CID-02、W-CID-07 | 退 0 / `grep -c`=0 / 条目=15 |
| **AC-B4-106** | 白名单收紧后负例退 4 且含 `error[rejected]`；负例不入库；高危许可证零命中 | W-CID-05 | 退 4 / `grep -c`≥1 / `git status`=0 |
| **AC-B4-107** | `advisories` 退 0、`RUSTSEC-2026-0592` 命中 0；`url`≥2.5.9；lock 无新增包 | W-CID-04 | 退 0 / `grep -c`=0 / 包集合不变 |
| **AC-B4-108** | `wildcards="deny"` 保持；`bans` 退 0、`error[wildcard]`=0；ui-bridge 两处版本 | W-CID-03 | 退 0 / `grep -c`=0 / 计数 2 |
| **AC-B4-109** | 本地/CI 同版 0.22.0、同命令串；四合一退 0；CI `compliance` success；版本前打印 | W-CID-06、W-CID-09 | 退 0 / `grep -c`=1 / `success` |
| **AC-B4-110** | 变更最小：`1 0`/`2 2`/`4 4`；`crates/**/src/**` 零 diff；根 manifest 零 diff；无新依赖；状态已登记 | W-CID-07、W-CID-10 | numstat 精确 / 空输出 / `grep`≥1 |

---

## 1. 基线与取证设施（W-CID-01）

> 目的：把"修复前"固化为可比较的事实，后续每条 AC 都能做 before/after 对照。本主任务**零改动**。

- [ ] **W-CID-01.1 固化基线三元组（HEAD / 工作区 / 四合一退 3）** · 仓库根
  - 追溯：AC-B4-105~110 · spec §2.1 F-07、§10-1 · design §2.1.3 修复顺序第 1 步、§2.5.7
  - 改动内容：仅记录，不改文件；确认 §0.4 闸门已满足
  - 依赖：§0.4 提交闸门完成
  - 禁止：不得在 `git status` 非空时取基线；不得用历史日志替代当日实测
  - DoD：`git status --short` 为空；`echo "all=$?"` 为 **3**；`50-all.log` 内 `advisories FAILED`、`bans FAILED`、`licenses ok`、`sources ok` 各 ≥1
  - 证据：
    ```bash
    set +e
    test -z "$(git status --porcelain)" || { echo "工作区非空，先完成 §0.4"; exit 1; }
    git rev-parse HEAD > "$EVID/00-head.txt"
    cargo deny check licenses bans sources advisories > "$EVID/00-baseline.log" 2>&1; all=$?
    echo "all=$all"
    [ "$all" = "3" ] || echo "注意：当日 advisory-db 可能已变化，记录实际值并写 baseline-note.txt"
    [ "$(grep -c 'advisories FAILED' "$EVID/00-baseline.log")" -ge 1 ]
    [ "$(grep -c 'bans FAILED' "$EVID/00-baseline.log")" -ge 1 ]
    [ "$(grep -c 'licenses ok' "$EVID/00-baseline.log")" -ge 1 ]
    [ "$(grep -c 'sources ok' "$EVID/00-baseline.log")" -ge 1 ]
    ```
  - 证据产物：`$EVID/00-head.txt`、`$EVID/00-baseline.log`
  - 偏差口径：若当日 advisory-db 已不再含该公告致 `all≠3`，记录实际值并在 `$EVID/00-baseline.log` 同目录写 `baseline-note.txt`，**不得**改写 AC 期望值。

- [ ] **W-CID-01.2 备份 lock 与 deny 基线，导出 15 项白名单** · `Cargo.lock`、`deny.toml`（只读）
  - 追溯：AC-B4-105 ④、AC-B4-107 ⑤ · design §2.4.1 验证命令、§2.4.3 执行策略第 1 步、§2.5.7
  - 改动内容：`cp` 到 `$EVID`；用 `awk+sed` 导出 `allow` 15 项到 `$EVID/allow.before`；记录包集合数
  - 依赖：W-CID-01.1
  - 禁止：不得修改被备份文件；不得把备份写入仓库跟踪路径
  - DoD：`cmp` lock 备份与工作区一致；`allow.before` 行数 = **15**；`grep -c '^name = ' Cargo.lock` 记录为 `$EVID/pkgs.before`
  - 证据：
    ```bash
    set -e
    cp Cargo.lock "$EVID/lock.before"
    cp deny.toml "$EVID/deny.toml.before"
    allow_list() { awk '/^allow = \[/{f=1;next} f&&/^\]/{f=0} f&&/^  "/{print}' "$1" | sed -E 's/^  "//; s/",$//'; }
    allow_list deny.toml > "$EVID/allow.before"
    [ "$(wc -l < "$EVID/allow.before" | tr -d ' ')" = "15" ]
    cmp "$EVID/lock.before" Cargo.lock
    grep -c '^name = ' Cargo.lock > "$EVID/pkgs.before"
    cat "$EVID/pkgs.before"
    ```
  - 证据产物：`$EVID/lock.before`、`$EVID/deny.toml.before`、`$EVID/allow.before`、`$EVID/pkgs.before`

- [ ] **W-CID-01.3 记录四项单项退出码（lic/adv/bans/src）** · 仓库根
  - 追溯：AC-B4-105/107/108/109 · spec §2.2 位掩码表 · design §2.1.3 退出码判定表
  - 改动内容：仅执行与记录
  - 依赖：W-CID-01.1
  - 禁止：不得只跑四合一就推断单项结果
  - DoD：`lic=0`、`adv=1`、`bans=2`、`src=0`（`adv` 以当日 advisory-db 为准，含 `RUSTSEC-2026-0592` 时为 1）
  - 证据：
    ```bash
    set +e
    cargo deny check licenses   > "$EVID/10-licenses.log"   2>&1; lic=$?
    cargo deny check advisories > "$EVID/30-advisories.log" 2>&1; adv=$?
    cargo deny check bans       > "$EVID/20-bans.log"       2>&1; bans=$?
    cargo deny check sources    > "$EVID/40-sources.log"    2>&1; src=$?
    printf 'lic=%s adv=%s bans=%s src=%s\n' "$lic" "$adv" "$bans" "$src"
    [ "$lic" = "0" ]
    [ "$bans" = "2" ]
    [ "$src" = "0" ]
    [ "$adv" = "1" ] || [ "$(grep -c 'RUSTSEC-2026-0592' "$EVID/30-advisories.log")" = "0" ]
    ```
  - 证据产物：`$EVID/10-licenses.log`、`20-bans.log`、`30-advisories.log`、`40-sources.log`

---

## 2. 许可证未命中噪音消除（W-CID-02 · R-1 → AC-B4-105）

> 本主任务只动 `deny.toml` 一行。`allow` 15 项与 `exceptions = []` 是冻结基线，任何删改即失败。

- [ ] **W-CID-02.1 `[licenses]` 增 `unused-allowed-license = "allow"`（保持末尾无换行）** · `deny.toml`
  - 追溯：AC-B4-105 ①③④ · spec §6.1.1-1、§7.1-2 · design §2.4.1、D-B4-P131 / D-B4-P135
  - 改动内容：在 `confidence-threshold = 0.93`（L17）之后、`allow = [`（L18）之前**单行插入** `unused-allowed-license = "allow"`
  - 依赖：W-CID-01
  - 禁止：不得改 `allow` 任一 entry 或顺序；不得改 `exceptions = []`；不得新增 `deny` 键；不得拆分为第二个配置文件；不得整文件重写（会引入末尾换行，使 numstat 不再是 `1 0`）；不得把键放进 `[bans]` 或写在 `[licenses]` 之外
  - DoD：`grep -c 'unused-allowed-license = "allow"'`=**1**；`grep -c 'unused_allowed_license'`=**0**；`git diff --numstat HEAD -- deny.toml`=`1 0`；`allow` 条目数=**15**；`ls deny*.toml` 恰 **1** 个；`tail -c 1 deny.toml | od -c` 末字符为 `]`（无 `\n`）
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'unused-allowed-license = "allow"' deny.toml)" = "1" ]
    [ "$(grep -c 'unused_allowed_license' deny.toml)" = "0" ]
    [ "$(git diff --numstat HEAD -- deny.toml | awk '{print $1" "$2}')" = "1 0" ]
    allow_count() { awk '/^allow = \[/{f=1;next} f&&/^\]/{f=0} f&&/^  "/{c++} END{print c+0}' "$1"; }
    [ "$(allow_count deny.toml)" = "15" ]
    [ "$(ls deny*.toml | wc -l | tr -d ' ')" = "1" ]
    [ "$(tail -c 1 deny.toml | od -An -c | tr -d ' \n')" = "]" ]
    echo "末尾字符=$(tail -c 1 deny.toml | od -An -c | tr -d ' \n')"
    ```
  - 证据产物：`$EVID/deny.toml.after`（`cp deny.toml`）、`$EVID/numstat.txt` 首行

- [ ] **W-CID-02.2 正向验证：`licenses` 退 0、未命中归零、汇总行 `0/0`** · 仓库根
  - 追溯：AC-B4-105 ①②③ · spec §6.1.1-1a、§8.3 正向命令 · design §2.5.2、§2.5.5
  - 改动内容：仅执行与记录
  - 依赖：W-CID-02.1
  - 禁止：不得加 `--deny warnings`、`-A`、`-L off` 制造假绿/假红；不得以"CI 是 3 条、本地 4 条"判断配置是否生效
  - DoD：`lic=0`；`grep -c 'license-not-encountered'`=**0**；`grep -cE 'licenses ok: 0 errors, 0 warnings'`≥**1**
  - 证据：
    ```bash
    set -e
    cargo deny check licenses > "$EVID/11-licenses-after.log" 2>&1; echo "lic=$?"
    [ "$(grep -c 'license-not-encountered' "$EVID/11-licenses-after.log")" = "0" ]
    cargo deny check licenses -s > "$EVID/51-licenses-summary.log" 2>&1
    [ "$(grep -cE 'licenses ok: 0 errors, 0 warnings' "$EVID/51-licenses-summary.log")" -ge 1 ]
    ```
  - 证据产物：`$EVID/11-licenses-after.log`、`$EVID/51-licenses-summary.log`

- [ ] **W-CID-02.3 白名单冻结守卫：15 项逐项相等 + 高危零命中 + 禁止通配** · `deny.toml`
  - 追溯：AC-B4-105 ④、AC-B4-106 ④ · spec §6.1.1-2、§6.2.1-3/4、§7.1-4/5/6 · design §2.4.1 异常、§2.9-7/8
  - 改动内容：仅比对，不改文件
  - 依赖：W-CID-02.1
  - 禁止：不得为"让 diff 更像 1 行"而调整白名单顺序；不得把高危许可证放进 `exceptions`
  - DoD：`diff "$EVID/allow.before" "$EVID/allow.after"` 为空；`grep -cE 'AGPL|GPL-|SSPL|OSL-|EUPL'`=**0**；`grep -cE '"\*"|UNLICENSED'`=**0**；`[licenses]` 段内 `exceptions = []` 计数=**1**
  - 证据：
    ```bash
    set -e
    allow_list() { awk '/^allow = \[/{f=1;next} f&&/^\]/{f=0} f&&/^  "/{print}' "$1" | sed -E 's/^  "//; s/",$//'; }
    allow_list deny.toml > "$EVID/allow.after"
    diff -u "$EVID/allow.before" "$EVID/allow.after"
    [ "$(grep -cE 'AGPL|GPL-|SSPL|OSL-|EUPL' deny.toml)" = "0" ]
    [ "$(grep -cE '"\*"|UNLICENSED' deny.toml)" = "0" ]
    [ "$(sed -n '/^\[licenses\]/,/^\[bans\]/p' deny.toml | grep -c '^exceptions = \[\]')" = "1" ]
    ```
  - 证据产物：`$EVID/allow.after`、`$EVID/allow.diff`（把 `diff -u` 输出重定向）

---

## 3. 通配依赖清零（W-CID-03 · R-3 → AC-B4-108）

> 本主任务只动 `crates/ui-bridge/Cargo.toml` 两行；不放宽 `[bans].wildcards`。

- [ ] **W-CID-03.1 两条 path 依赖补 `version = "0.1.0"`（version 在前）** · `crates/ui-bridge/Cargo.toml` L11-12
  - 追溯：AC-B4-108 ③④ · spec §6.4.1-1/2/3、§7.3-1/2 · design §2.4.2、D-B4-P133 / D-B4-P136
  - 改动内容：`meeting-copilot-application = { path = "../application" }` → `{ version = "0.1.0", path = "../application" }`；`meeting-copilot-domain` 同形改 `../domain`
  - 依赖：W-CID-01（基线已固化）
  - 禁止：不得用 `version.workspace = true`（`[workspace.dependencies]` 无同名条目）；不得改根 `Cargo.toml` 增 `[workspace.dependencies]`；不得新增/删除依赖；不得改 `package.version`/`license`/`dev-dependencies`；不得只改一条
  - DoD：`git diff --numstat HEAD -- crates/ui-bridge/Cargo.toml`=`2 2`；`grep -c 'version = "0.1.0"'`=**2**；两处 path 字面量各命中 **1**；`grep -c 'version.workspace = true'`=**1**（仅 package 段，未新增）
  - 证据：
    ```bash
    set -e
    [ "$(git diff --numstat HEAD -- crates/ui-bridge/Cargo.toml | awk '{print $1" "$2}')" = "2 2" ]
    [ "$(grep -c 'version = "0.1.0"' crates/ui-bridge/Cargo.toml)" = "2" ]
    [ "$(grep -c 'version = "0.1.0", path = "../application"' crates/ui-bridge/Cargo.toml)" = "1" ]
    [ "$(grep -c 'version = "0.1.0", path = "../domain"' crates/ui-bridge/Cargo.toml)" = "1" ]
    [ "$(grep -c 'version.workspace = true' crates/ui-bridge/Cargo.toml)" = "1" ]
    ```
  - 证据产物：`$EVID/ui-bridge.after`

- [ ] **W-CID-03.2 `cargo metadata --locked` 退 0 + `bans` 退 0、wildcard 归零** · 仓库根
  - 追溯：AC-B4-108 ①② · spec §6.4.1-1a、§8.3 正向 · design §2.5.2、F-13
  - 改动内容：仅执行与记录
  - 依赖：W-CID-03.1
  - 禁止：不得为了过 `bans` 把 `wildcards` 改 `"warn"`；不得加 `[bans].exceptions`
  - DoD：`metadata` 退 **0**；`bans=0`；`grep -c 'error\[wildcard\]'`=**0**；`bans ok`≥**1**；`grep -c 'wildcards = "deny"' deny.toml`=**1**
  - 证据：
    ```bash
    set -e
    cargo metadata --locked --format-version 1 > "$EVID/metadata.json"
    cargo deny check bans > "$EVID/21-bans-after.log" 2>&1; echo "bans=$?"
    [ "$(grep -c 'error\[wildcard\]' "$EVID/21-bans-after.log")" = "0" ]
    [ "$(grep -c 'bans ok' "$EVID/21-bans-after.log")" -ge 1 ]
    [ "$(grep -c 'wildcards = "deny"' deny.toml)" = "1" ]
    ```
  - 证据产物：`$EVID/metadata.json`、`$EVID/21-bans-after.log`

- [ ] **W-CID-03.3 版本与 workspace 一致 + 依赖边不变** · `Cargo.toml`、`crates/ui-bridge/Cargo.toml`
  - 追溯：AC-B4-108 ④ · spec §6.4.1-3、§7.3-1 · design §2.4.2 一致性守卫、D-B4-P136
  - 改动内容：仅断言；依赖边以基线与改后 `cargo tree` 对比（基线在修复前记录一次即可，若已错过则用 `git stash`/临时 worktree 补录，禁止修改工作区）
  - 依赖：W-CID-03.1
  - 禁止：不得改 `[workspace.package].version`
  - DoD：`ws` 提取为 `0.1.0`；两处字面量各=1；`ui-bridge` 正常依赖子图与基线**逐行相同**
  - 证据：
    ```bash
    set -e
    ws="$(awk -F'"' '/^\[workspace.package\]/{f=1;next} f&&/^version *=/{print $2;exit}' Cargo.toml)"
    [ "$ws" = "0.1.0" ]
    [ "$(grep -c "version = \"${ws}\", path = \"../application\"" crates/ui-bridge/Cargo.toml)" = "1" ]
    [ "$(grep -c "version = \"${ws}\", path = \"../domain\"" crates/ui-bridge/Cargo.toml)" = "1" ]
    cargo tree -p meeting-copilot-ui-bridge --edges normal --prefix none > "$EVID/tree.ui-bridge.after"
    # 期望与 $EVID/tree.ui-bridge.before 的包名集合一致（新增 version 不改变边）
    ```
  - 证据产物：`$EVID/tree.ui-bridge.after`、`$EVID/version-check.log`
  - 备注：`tree.ui-bridge.before` 允许在 W-CID-03.1 之前用 `git stash push -- crates/ui-bridge/Cargo.toml` 采集后 `stash pop` 恢复；**不得**把该操作留到提交后。

---

## 4. url 漏洞清零（W-CID-04 · R-2 → AC-B4-107）

> 本主任务只允许 `Cargo.lock` 出现 `url` 与 `form_urlencoded` 的 **4 行**补丁；`cssparser-macros` 的 `syn` 行必须恢复原值。

- [ ] **W-CID-04.1 升级前复核：lock 未漂移、url 仍 2.5.8** · `Cargo.lock`（只读）
  - 追溯：AC-B4-107 ③⑤ · design §2.4.3 执行策略第 1/3 步
  - 改动内容：仅比对与记录
  - 依赖：W-CID-03（前两项已绿）
  - 禁止：不得跳过复核直接 `cargo update`（若 lock 已被改动，会污染 4 行判据）
  - DoD：`cmp "$EVID/lock.before" Cargo.lock` 退 0；`url` 版本行为 `version = "2.5.8"`
  - 证据：
    ```bash
    set -e
    cmp "$EVID/lock.before" Cargo.lock
    awk '/^name = "url"/{f=1} f&&/^version/{print;exit}' Cargo.lock | grep -qx 'version = "2.5.8"'
    ```
  - 证据产物：`$EVID/40-lock-precheck.log`

- [ ] **W-CID-04.2 执行精确升级 `cargo update -p url --precise 2.5.9`** · `Cargo.lock`
  - 追溯：AC-B4-107 ③ · spec §6.3.1-1、§7.4-1 · design §2.4.3、D-B4-P132 / D-B4-P137
  - 改动内容：执行命令并由 cargo 生成 checksum；输出落日志
  - 依赖：W-CID-04.1
  - 禁止：不得手工编辑 `Cargo.lock`/伪造 checksum；不得改 `[advisories].ignore`；不得改用 `<2.5.9` 的约束变通；失败时不得降回 2.5.8 后宣称通过
  - DoD：命令退 **0**；`url` 版本=**2.5.9**；`form_urlencoded`=**1.2.3**
  - 证据：
    ```bash
    set -e
    cargo update -p url --precise 2.5.9 > "$EVID/41-cargo-update.log" 2>&1
    awk '/^name = "url"/{f=1} f&&/^version/{print;exit}' Cargo.lock | grep -qx 'version = "2.5.9"'
    awk '/^name = "form_urlencoded"/{f=1} f&&/^version/{print;exit}' Cargo.lock | grep -qx 'version = "1.2.3"'
    ```
  - 证据产物：`$EVID/41-cargo-update.log`、`$EVID/lock.after-update`
  - 异常处置：若退非 0 或未解析到 2.5.9 → 停止，登记"升级受阻"，AC-B4-107 记 `pending`，**不得**用 `ignore` 绕过（design §2.6.2 D-1）。

- [ ] **W-CID-04.3 收敛 diff 到 4 行：恢复 `cssparser-macros` 的 `syn` 副作用** · `Cargo.lock`
  - 追溯：AC-B4-107 ⑤、AC-B4-110 ① · design §2.4.3 执行策略第 3~4 步、D-B4-P138 / D-B4-P139、R-14-02
  - 改动内容：先看 numstat。若为 `4 4` 直接进入本任务 DoD；若更大，`git diff --unified=0 HEAD -- Cargo.lock` 定位 `cssparser-macros` 依赖数组中的 `- "syn 3.0.10"` / `+ "syn 2.0.119"`，把该行恢复为原值（从 `$EVID/lock.before` 摘回），再复跑 `cargo metadata --locked`
  - 依赖：W-CID-04.2
  - 禁止：不得接受 `syn` 降级；不得恢复 `url`/`form_urlencoded` 之外的其他版本行；不得手工改 checksum；不得用 `--offline` 掩盖索引问题
  - DoD：`numstat`=`4 4`；`grep -cE '^[+-](version|checksum)'`=**4**；`^+name = `=**0**、`^-name = `=**0**；包集合数 = `$EVID/pkgs.before`；`cargo metadata --locked` 退 **0**
  - 证据：
    ```bash
    set -e
    git diff --numstat HEAD -- Cargo.lock > "$EVID/numstat.lock.txt"
    [ "$(awk '{print $1" "$2}' "$EVID/numstat.lock.txt")" = "4 4" ]
    [ "$(git diff --unified=0 HEAD -- Cargo.lock | grep -cE '^[+-](version|checksum)')" = "4" ]
    [ "$(git diff HEAD -- Cargo.lock | grep -c '^+name = ')" = "0" ]
    [ "$(git diff HEAD -- Cargo.lock | grep -c '^-name = ')" = "0" ]
    [ "$(grep -c '^name = ' Cargo.lock)" = "$(cat "$EVID/pkgs.before")" ]
    cargo metadata --locked --format-version 1 > /dev/null
    git diff --unified=0 HEAD -- Cargo.lock > "$EVID/lock.diff"
    ```
  - 证据产物：`$EVID/numstat.lock.txt`、`$EVID/lock.diff`、`$EVID/metadata.locked.log`
  - 异常处置：恢复 `syn` 后若 `metadata --locked` 失败 → 从 `$EVID/lock.before` 整体重做（W-CID-04.2 起），不得提交半成品。

- [ ] **W-CID-04.4 `advisories` 退 0、公告命中归零、`ignore` 仍为空** · 仓库根
  - 追溯：AC-B4-107 ①②、AC-B4-110 ⑤ · spec §6.3.1-1/3、§7.2-1 · design §2.5.2、F-14/F-15
  - 改动内容：仅执行与记录
  - 依赖：W-CID-04.3
  - 禁止：不得用 `[advisories].ignore` 掩盖；不得引用旧日期证据；离线时不得宣称 `passed`
  - DoD：`adv=0`；`grep -c 'RUSTSEC-2026-0592'`=**0**；`advisories ok`≥**1**；`grep -c 'ignore = \[\]'`（`[advisories]` 段内）=**1**
  - 证据：
    ```bash
    set -e
    cargo deny check advisories > "$EVID/31-advisories-after.log" 2>&1; echo "adv=$?"
    [ "$(grep -c 'RUSTSEC-2026-0592' "$EVID/31-advisories-after.log")" = "0" ]
    [ "$(grep -c 'advisories ok' "$EVID/31-advisories-after.log")" -ge 1 ]
    [ "$(sed -n '/^\[advisories\]/,/^\[licenses\]/p' deny.toml | grep -c 'ignore = \[\]')" = "1" ]
    ```
  - 证据产物：`$EVID/31-advisories-after.log`
  - 降级：若离线无缓存 → `$EVID/offline-note.txt` 记录，AC-B4-107 记 `not-applicable`（W-CID-10.2）。

---

## 5. 真实约束负例（W-CID-05 · AC-B4-106）

> 负例只允许存在于 `$TEMP`；仓库内 `deny.toml` 必须始终是修复后的版本。

- [ ] **W-CID-05.1 生成临时收紧副本（精确删除 `"MIT"` 一行）** · `$TEMP/deny_negative.toml`
  - 追溯：AC-B4-106 ① · spec §6.2.1-1、§6.2.3 · design §2.5.3、D-B4-P140
  - 改动内容：`grep -v '^  "MIT",$' deny.toml > "$TEMP/deny_negative.toml"`；断言 15→14、MIT 计数 0
  - 依赖：W-CID-02（`deny.toml` 已含新键）
  - 禁止：**不得使用 spec §8.3 的 `awk` 删除方式**（会删掉 `]`，导致配置非法而非 `error[rejected]`）；不得写入仓库根或任何跟踪路径；不得用 `diff` 行数判断副本（末尾无换行会导致整文件差异）
  - DoD：`allow_count` 15→**14**；副本中 `^  "MIT",$` 计数=**0**；副本位于 `$TEMP`；`git status --short` 不含该文件
  - 证据：
    ```bash
    set -e
    NEG="$TEMP/deny_negative.toml"
    grep -v '^  "MIT",$' deny.toml > "$NEG"
    allow_count() { awk '/^allow = \[/{f=1;next} f&&/^\]/{f=0} f&&/^  "/{c++} END{print c+0}' "$1"; }
    [ "$(allow_count deny.toml)" = "15" ]
    [ "$(allow_count "$NEG")" = "14" ]
    [ "$(grep -c '^  "MIT",$' "$NEG")" = "0" ]
    [ "$(git status --porcelain | grep -c 'deny_negative')" = "0" ]
    ```
  - 证据产物：`$EVID/51-negative-setup.log`

- [ ] **W-CID-05.2 负例执行：退 4 + `error[rejected]`** · 仓库根
  - 追溯：AC-B4-106 ①② · spec §6.2.1-1a · design §2.5.3、F-12
  - 改动内容：`cargo deny --config "$NEG" check licenses`
  - 依赖：W-CID-05.1
  - 禁止：不得用 `-A license-not-allowed`、`--allow`、`-W` 等方式制造/消除结果；不得因退 4 就改动仓库配置去"修"负例
  - DoD：退 **4**；`error[rejected]`≥**1**；`licenses FAILED`≥**1**
  - 证据：
    ```bash
    set +e
    NEG="$TEMP/deny_negative.toml"
    cargo deny --config "$NEG" check licenses > "$EVID/60-negative.log" 2>&1; neg=$?
    echo "neg=$neg"
    [ "$neg" = "4" ]
    [ "$(grep -c 'error\[rejected\]' "$EVID/60-negative.log")" -ge 1 ]
    [ "$(grep -c 'licenses FAILED' "$EVID/60-negative.log")" -ge 1 ]
    ```
  - 证据产物：`$EVID/60-negative.log`
  - 异常处置：退 0 或出现配置解析错误 → 检查 `--config` 路径与副本是否只删 1 行；重做副本，不得改结论。

- [ ] **W-CID-05.3 负例不入库 + 静态守卫（高危零命中、无通配放行）** · 仓库根
  - 追溯：AC-B4-106 ③④ · spec §6.2.1-2/3/4、§6.2.3-2 · design §2.5.3 末段
  - 改动内容：仅断言
  - 依赖：W-CID-05.2
  - 禁止：不得把负例配置提交；不得为了让负例"更真实"而删除仓库白名单项
  - DoD：`git status --short | grep -c 'deny_negative'`=**0**；仓库 `deny.toml` 中 `"MIT",` 计数=**1**；高危标识=**0**；`"\*"|UNLICENSED`=**0**
  - 证据：
    ```bash
    set -e
    [ "$(git status --short | grep -c 'deny_negative')" = "0" ]
    [ "$(grep -c '^  "MIT",$' deny.toml)" = "1" ]
    [ "$(grep -cE 'AGPL|GPL-|SSPL|OSL-|EUPL' deny.toml)" = "0" ]
    [ "$(grep -cE '"\*"|UNLICENSED' deny.toml)" = "0" ]
    ```
  - 证据产物：`$EVID/61-negative-guard.log`

---

## 6. CI 版本可观测（W-CID-06 · C-1 → AC-B4-109）

> 只新增一个独立 step，不改 `licenses` 的 `run`，不动 `cargo-deny@0.22.0`。

- [ ] **W-CID-06.1 在 `licenses` 前插入 `cargo-deny-version` step（+2 行）** · `.github/workflows/ci.yml`
  - 追溯：AC-B4-109 ③④ · spec §5.4-1、§6.5.1-3、§7.5-2 · design §2.4.4、D-B4-P134 / D-B4-P141
  - 改动内容：在 `- name: secrets-docs (C1)` 的 `run:` 之后、`- name: licenses` 之前插入：
    `      - name: cargo-deny-version` / `        run: cargo deny --version`
  - 依赖：W-CID-02、W-CID-03、W-CID-04（建议四文件同期修改，但本任务只改 ci.yml）
  - 禁止：不得把版本打印塞进 `licenses.run`（会破坏"命令串恰为四合一"）；不得改 `tool: cargo-deny@0.22.0`；不得插入到 `layering` 之前；不得加 `--deny warnings`、`-A`、`-L off`；不得改其他 step
  - DoD：`git diff --numstat HEAD -- .github/workflows/ci.yml`=`2 0`；`grep -c 'cargo deny --version'`=**1**；`licenses` 前 3 行内命中 `cargo deny --version`≥**1**；缩进为 6/8 空格
  - 证据：
    ```bash
    set -e
    [ "$(git diff --numstat HEAD -- .github/workflows/ci.yml | awk '{print $1" "$2}')" = "2 0" ]
    [ "$(grep -c 'cargo deny --version' .github/workflows/ci.yml)" = "1" ]
    L="$(awk '/- name: licenses/{print NR;exit}' .github/workflows/ci.yml)"
    [ "$(sed -n "$((L-3)),$((L-1))p" .github/workflows/ci.yml | grep -c 'cargo deny --version')" -ge 1 ]
    grep -nE '^      - name: cargo-deny-version$|^        run: cargo deny --version$' .github/workflows/ci.yml
    ```
  - 证据产物：`$EVID/ci.yml.after`、`$EVID/ci.diff`

- [ ] **W-CID-06.2 CI 静态一致性断言（命令串唯一、版本固定、无禁用参数）** · `.github/workflows/ci.yml`
  - 追溯：AC-B4-109 ③ · spec §6.1.1-4、§6.5.1-2/4、§7.5-1/3 · design §2.4.4 验证命令
  - 改动内容：仅断言
  - 依赖：W-CID-06.1
  - 禁止：不得为"让 grep 唯一"而删掉其他 job 的合法命令串（本仓库当前仅 compliance 一处）
  - DoD：四合一命令串计数=**1**；`cargo-deny@0.22.0` 计数=**1**；`unused-allowed-license|--deny warnings|-A license|-L off` 计数=**0**
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'cargo deny check licenses bans sources advisories' .github/workflows/ci.yml)" = "1" ]
    [ "$(grep -c 'cargo-deny@0.22.0' .github/workflows/ci.yml)" = "1" ]
    [ "$(grep -cE 'unused-allowed-license|--deny warnings|-A license|-L off' .github/workflows/ci.yml)" = "0" ]
    ```
  - 证据产物：`$EVID/ci-static.log`

- [ ] **W-CID-06.3 本地四合一退 0（AC-B4-109 ②）** · 仓库根
  - 追溯：AC-B4-109 ②④ · spec §6.5.1-5、§8.3 · design §2.5.2、F-15
  - 改动内容：仅执行与记录
  - 依赖：W-CID-02、W-CID-03、W-CID-04
  - 禁止：不得用单项替代四合一；不得加任何降噪/提级参数
  - DoD：退 **0**；四条 `ok` 各≥**1**；`license-not-encountered`=**0**；`error\[wildcard\]`=**0**；`RUSTSEC-2026-0592`=**0**；`cargo deny --version` 前缀=`cargo-deny 0.22.0`
  - 证据：
    ```bash
    set +e
    cargo deny --version > "$EVID/00-version.log" 2>&1
    [ "$(grep -c 'cargo-deny 0.22.0' "$EVID/00-version.log")" -ge 1 ]
    cargo deny check licenses bans sources advisories > "$EVID/50-all.log" 2>&1; all=$?
    echo "all=$all"
    [ "$all" = "0" ]
    for s in 'advisories ok' 'bans ok' 'licenses ok' 'sources ok'; do
      [ "$(grep -c "$s" "$EVID/50-all.log")" -ge 1 ]
    done
    [ "$(grep -c 'license-not-encountered' "$EVID/50-all.log")" = "0" ]
    [ "$(grep -c 'error\[wildcard\]' "$EVID/50-all.log")" = "0" ]
    [ "$(grep -c 'RUSTSEC-2026-0592' "$EVID/50-all.log")" = "0" ]
    ```
  - 证据产物：`$EVID/00-version.log`、`$EVID/50-all.log`

---

## 7. 变更预算与冻结面复核（W-CID-07 · AC-B4-110）

> 本主任务零改动，只做判据。任一不满足即回到对应改动修正，**不得**放宽判据。

- [ ] **W-CID-07.1 四处 numstat 精确匹配** · 仓库根
  - 追溯：AC-B4-110 ①② · spec §8.2 AC-110、§9.2-5/6 · design §2.4.1~§2.4.4、§2.9-3~6
  - 改动内容：仅断言
  - 依赖：W-CID-02/03/04/06
  - 禁止：不得用"新增行数 ≤3"替代 `1 0`（本目录取更严口径）；不得通过增删空行"修正" numstat
  - DoD：`deny.toml`=`1 0`；ui-bridge=`2 2`；`Cargo.lock`=`4 4`；`ci.yml`=`2 0`
  - 证据：
    ```bash
    set -e
    git diff --numstat HEAD -- deny.toml crates/ui-bridge/Cargo.toml Cargo.lock .github/workflows/ci.yml > "$EVID/numstat.txt"
    cat "$EVID/numstat.txt"
    [ "$(awk '$3=="deny.toml"{print $1" "$2}' "$EVID/numstat.txt")" = "1 0" ]
    [ "$(awk '$3=="crates/ui-bridge/Cargo.toml"{print $1" "$2}' "$EVID/numstat.txt")" = "2 2" ]
    [ "$(awk '$3=="Cargo.lock"{print $1" "$2}' "$EVID/numstat.txt")" = "4 4" ]
    [ "$(awk '$3==".github/workflows/ci.yml"{print $1" "$2}' "$EVID/numstat.txt")" = "2 0" ]
    ```
  - 证据产物：`$EVID/numstat.txt`

- [ ] **W-CID-07.2 冻结面零 diff（源码 / 根 manifest / 脚本 / 其他 manifest）** · 仓库根
  - 追溯：AC-B4-110 ③④ · spec §9.2-5/6 · design §1.2.6 冻结面清单、§2.9-10~12
  - 改动内容：仅断言
  - 依赖：W-CID-07.1
  - 禁止：不得为"绕过"而用 `git update-index --assume-unchanged`；不得改 `.gitignore` 隐藏改动
  - DoD：`crates/**/src/**` 空；根 `Cargo.toml` 空；`scripts/` 空；除 ui-bridge 外无 `*Cargo.toml` 变更；`NEW_TARGETS`=**25** 且含 `llm_settings_view`
  - 证据：
    ```bash
    set -e
    test -z "$(git diff --stat HEAD -- 'crates/*/src/**')"
    git diff --quiet HEAD -- Cargo.toml
    git diff --quiet HEAD -- scripts
    [ "$(git diff --name-only HEAD -- '*Cargo.toml' | grep -v '^crates/ui-bridge/Cargo.toml$' | wc -l | tr -d ' ')" = "0" ]
    [ "$(grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//" | wc -w | tr -d ' ')" = "25" ]
    grep -q 'llm_settings_view' scripts/test-name-inventory.sh
    ```
  - 证据产物：`$EVID/zero-diff.log`

- [ ] **W-CID-07.3 硬红线静态断言（ignore / wildcards / sources / 白名单 / 无新依赖）** · `deny.toml`、`Cargo.lock`
  - 追溯：AC-B4-105~108 · spec §5.3、§7.1~§7.4 · design §2.6.3 禁止降级项
  - 改动内容：仅断言
  - 依赖：W-CID-07.1
  - 禁止：不得以注释形式"保留"被禁键；不得新增 `exceptions`
  - DoD：`ignore = []`=**1**；`yanked = "deny"`=**1**；`wildcards = "deny"`=**1**；两个 `unknown-* = "deny"` 各=**1**；`allow-registry`=**1**；`allow` 15；`unused-allowed-license = "allow"`=**1**；`+name = `=**0**
  - 证据：
    ```bash
    set -e
    [ "$(sed -n '/^\[advisories\]/,/^\[licenses\]/p' deny.toml | grep -c 'ignore = \[\]')" = "1" ]
    [ "$(grep -c 'yanked = "deny"' deny.toml)" = "1" ]
    [ "$(grep -c 'wildcards = "deny"' deny.toml)" = "1" ]
    [ "$(grep -c 'unknown-registry = "deny"' deny.toml)" = "1" ]
    [ "$(grep -c 'unknown-git = "deny"' deny.toml)" = "1" ]
    [ "$(grep -c 'allow-registry' deny.toml)" = "1" ]
    [ "$(grep -c 'unused-allowed-license = "allow"' deny.toml)" = "1" ]
    allow_count() { awk '/^allow = \[/{f=1;next} f&&/^\]/{f=0} f&&/^  "/{c++} END{print c+0}' "$1"; }
    [ "$(allow_count deny.toml)" = "15" ]
    [ "$(git diff HEAD -- Cargo.lock | grep -c '^+name = ')" = "0" ]
    ```
  - 证据产物：`$EVID/redlines.log`

- [ ] **W-CID-07.4 target 清单与 inventory 无漂移** · `scripts/test-name-inventory.sh`（只读）
  - 追溯：AC-B4-110 · spec §9.2-5 · design §1.2.5、§2.9-13
  - 改动内容：仅执行
  - 依赖：W-CID-07.1
  - 禁止：不得改 `NEW_TARGETS` 或 `--diff` 语义
  - DoD：`--diff` 退 **0**；输出含 `removed=0`；`git diff --quiet HEAD -- scripts/test-name-inventory.sh` 退 **0**
  - 证据：
    ```bash
    set -e
    bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
    git diff --quiet HEAD -- scripts/test-name-inventory.sh
    ```
  - 证据产物：`$EVID/inventory.log`

---

## 8. 既有门禁复跑（W-CID-08）

> 与设计 §2.7 一一对应；本主任务零改动。任一非 0 先判断是否与本次变更相关，相关则回滚对应改动，不相关则单独登记。

- [ ] **W-CID-08.1 B3 六规则退 0** · 仓库根
  - 追溯：AC-B4-110（旁证） · design §2.7.1、§2.7.2-1
  - 改动内容：仅执行
  - 依赖：W-CID-07
  - 禁止：不得改 `scripts/check-b3-gates.sh` 或 `crates/**` 源码
  - DoD：退 **0**；日志无 `FAIL`/`MISSING`
  - 证据：
    ```bash
    set +e
    bash scripts/check-b3-gates.sh > "$EVID/70-b3.log" 2>&1; b3=$?
    echo "b3=$b3"
    [ "$b3" = "0" ]
    ```
  - 证据产物：`$EVID/70-b3.log`

- [ ] **W-CID-08.2 B4 七规则 + self-test 全 0，`--rule=targets` 空输出** · 仓库根
  - 追溯：AC-B4-110（旁证） · design §2.7.1、§2.7.2-2/3、§2.7.4
  - 改动内容：仅执行
  - 依赖：W-CID-08.1
  - 禁止：不得为过 `targets` 改 `NEW_TARGETS`；不得新增第 8 条规则
  - DoD：`source/budget/record/outbound/zero-transcript/doc-plan/targets` 7×**0**；`--self-test`=**0**；`--rule=targets` 输出为空
  - 证据：
    ```bash
    set +e
    rc=0
    for r in source budget record outbound zero-transcript doc-plan targets; do
      bash scripts/check-b4-gates.sh --rule="$r" > "$EVID/71-b4-$r.log" 2>&1; rrc=$?
      echo "$r=$rrc"
      [ "$rrc" = "0" ] || rc=1
    done
    [ "$rc" = "0" ]
    bash scripts/check-b4-gates.sh --self-test > "$EVID/71-b4-self-test.log" 2>&1; st=$?
    echo "self-test=$st"
    [ "$st" = "0" ]
    [ -z "$(bash scripts/check-b4-gates.sh --rule=targets)" ]
    ```
  - 证据产物：`$EVID/71-b4-*.log`
  - 特别提示：`zero-transcript` 会扫描 `b4-status.md`，若此前试写 §22 时留下了 URL，此步会退 1 —— 应删除 URL 而不是放宽规则。

- [ ] **W-CID-08.3 layering + SBOM 复跑** · 仓库根
  - 追溯：AC-B4-110（旁证） · design §2.7.1、§2.7.2-4/5
  - 改动内容：仅执行；SBOM 产物只落 `docs/compliance/sbom/`（被 ignore）
  - 依赖：W-CID-08.2
  - 禁止：不得因 SBOM 里 `url`/`form_urlencoded` 版本变化判失败；不得改 `scripts/sbom.sh`
  - DoD：`layering=0` 且输出含 `PASS`；`sbom=0`；`git status --short` 仍只含预期的 **4** 个改动文件（§22 尚未写）
  - 证据：
    ```bash
    set +e
    bash scripts/check-layering.sh > "$EVID/72-layering.log" 2>&1; layer=$?
    echo "layer=$layer"
    [ "$layer" = "0" ]
    [ "$(grep -c 'PASS' "$EVID/72-layering.log")" -ge 1 ]
    bash scripts/sbom.sh ci > "$EVID/73-sbom.log" 2>&1; sbom=$?
    echo "sbom=$sbom"
    [ "$sbom" = "0" ]
    git status --short
    ```
  - 证据产物：`$EVID/72-layering.log`、`$EVID/73-sbom.log`

- [ ] **W-CID-08.4 rust 静态与测试旁证（fmt / clippy / test）** · 仓库根
  - 追溯：AC-B4-110（旁证） · design §2.7.1 末两行、§2.9-16
  - 改动内容：仅执行；耗时较长，允许分进程，但最终结论以完整命令退出码为准
  - 依赖：W-CID-08.3
  - 禁止：不得为过 lint 改任何 `.rs`（本次零 `.rs` 改动，出现失败即为环境/基线问题，单独登记）
  - DoD：`fmt=0`；`clippy=0`；`test` 输出 `failed=0` 且 `ignored=9`
  - 证据：
    ```bash
    set +e
    "$CARGO" fmt --all -- --check > "$EVID/74-fmt.log" 2>&1; fmt=$?
    "$CARGO" clippy --workspace --all-targets -- -D warnings > "$EVID/75-clippy.log" 2>&1; clippy=$?
    "$CARGO" test --workspace > "$EVID/76-test.log" 2>&1; tst=$?
    printf 'fmt=%s clippy=%s test=%s\n' "$fmt" "$clippy" "$tst"
    [ "$fmt" = "0" ]
    [ "$clippy" = "0" ]
    [ "$tst" = "0" ]
    grep -E 'test result:.*failed' "$EVID/76-test.log" | tail -5
    "$CARGO" test --workspace -- --list --ignored 2>/dev/null | grep -c ': test' > "$EVID/77-ignored.txt"
    cat "$EVID/77-ignored.txt"
    [ "$(cat "$EVID/77-ignored.txt")" = "9" ] || echo "注意：ignored 计数与基线 9 不同，登记偏差"
    ```
  - 证据产物：`$EVID/74-fmt.log`、`$EVID/75-clippy.log`、`$EVID/76-test.log`

---

## 9. 提交与远端取证（W-CID-09 · AC-B4-109 ④）

> 四文件必须**同一提交**；§22 因含 run-id，允许后置一次提交。

- [ ] **W-CID-09.1 提交前复核：工作区只含预期 4 文件 + 无空白错误** · 仓库根
  - 追溯：AC-B4-110 · design §2.1.3 "事务/一致性设计"
  - 改动内容：仅断言
  - 依赖：W-CID-07、W-CID-08
  - 禁止：不得把 `.tmp/`、`deny_negative.toml`、SBOM 中间产物加入暂存区
  - DoD：`git status --short` 恰 4 个 `M`（`deny.toml`、`crates/ui-bridge/Cargo.toml`、`Cargo.lock`、`.github/workflows/ci.yml`）；`git diff --check` 退 **0**
  - 证据：
    ```bash
    set +e
    git status --short > "$EVID/80-status.txt"
    cat "$EVID/80-status.txt"
    [ "$(sed '/^$/d' "$EVID/80-status.txt" | wc -l | tr -d ' ')" = "4" ]
    [ "$(grep -cE 'deny.toml|crates/ui-bridge/Cargo.toml|Cargo.lock|.github/workflows/ci.yml' "$EVID/80-status.txt")" = "4" ]
    git diff --check
    ```
  - 证据产物：`$EVID/80-status.txt`

- [ ] **W-CID-09.2 四文件同一提交（禁止拆分、禁止跳过钩子）** · git
  - 追溯：AC-B4-110 · design §2.6.1 回滚纪律、§2.1.3 一致性边界
  - 改动内容：`git add` 四文件后一次提交，消息建议 `fix(ci): 修复 cargo-deny 四类合规检查并打印版本`
  - 依赖：W-CID-09.1
  - 禁止：不得把 `deny.toml` 与 `Cargo.lock` 分成两个提交（会产生"许可证已修、漏洞/通配仍红"的半成品历史）；不得 `--no-verify`；不得 amend 既有 §21 提交
  - DoD：`git show --stat --oneline HEAD` 恰 **4** 个文件；`git log -1 --format=%s` 含 `cargo-deny`；`git status --short` 为空
  - 证据：
    ```bash
    set -e
    git add deny.toml crates/ui-bridge/Cargo.toml Cargo.lock .github/workflows/ci.yml
    git commit -m "fix(ci): 修复 cargo-deny 四类合规检查并打印版本"
    git show --stat --oneline HEAD
    [ "$(git show --pretty=format: --name-only HEAD | sed '/^$/d' | sort -u | wc -l | tr -d ' ')" = "4" ]
    git status --short
    ```
  - 证据产物：`$EVID/81-commit.txt`（`git show --stat HEAD`）

- [ ] **W-CID-09.3 push 后取 compliance 结论与日志（版本行 + 四条 ok）** · 远端
  - 追溯：AC-B4-109 ④ · spec §6.5.1-1/5、§8.3 远端 · design §2.5.4
  - 改动内容：`git push`；用 `gh run` 取证并落 `$EVID/ci-run-id.txt`
  - 依赖：W-CID-09.2
  - 禁止：不得只看"run 已结束"就宣称通过；不得改命令串/加参数重跑；`gh` 不可用时不得伪造 run-id
  - DoD：输出含 `cargo-deny 0.22.0`≥**1**；四条 `ok` 各≥**1**；`compliance` 结论=`success`；`cargo-deny-version` step 结论=`success`
  - 证据：
    ```bash
    set -e
    git push
    # 若 gh 可用：
    gh run list --limit 3
    gh run view <run-id> --log | grep -E 'cargo-deny 0\.22\.0|licenses ok|bans ok|advisories ok|sources ok'
    gh run view <run-id> --json jobs --jq '.jobs[] | select(.name|startswith("compliance")) | .conclusion'
    gh run view <run-id> --json jobs --jq '.jobs[] | select(.name|startswith("compliance")) | .steps[] | select(.name=="cargo-deny-version") | .conclusion'
    echo "<run-id>" > "$EVID/ci-run-id.txt"
    ```
  - 证据产物：`$EVID/ci-run-id.txt`、`$EVID/ci-compliance.log`
  - 降级：`gh` 不可用时，用 GitHub UI 记录 run-id 与结论到 `$EVID/manual-ci.txt`，AC-B4-109 ④ 记 `pending`（其余子项仍可 `passed`），不得记 `not-applicable` 以外的状态。

- [ ] **W-CID-09.4 远端失败分支处置（仅诊断，不放宽约束）** · 远端
  - 追溯：AC-B4-107 复跑纪律 · spec §10-3 · design §2.6.2 D-4/D-5
  - 改动内容：若 `compliance` 非 success，按日志定位：① 新增 advisory → 同日重跑并记录；② `form_urlencoded`/`syn` 回退 → 回 W-CID-04.3；③ 版本/命令串不符 → 回 W-CID-06
  - 依赖：W-CID-09.3（仅在其失败时执行）
  - 禁止：不得加 `[advisories].ignore`；不得改 `wildcards`/`sources` 级别；不得在 CI 加 `--offline`/`-A`/`--deny warnings`
  - DoD：失败原因归类为上述三类之一并写入 `$EVID/ci-fail-diagnosis.txt`；修复后重跑至 `success`，且四文件 numstat 预算不变
  - 证据：
    ```bash
    set +e
    gh run view <run-id> --log-failed > "$EVID/ci-fail.log"
    grep -cE 'RUSTSEC-|error\[wildcard\]|error\[rejected\]|cargo metadata' "$EVID/ci-fail.log"
    git diff --numstat HEAD~1 HEAD
    ```
  - 证据产物：`$EVID/ci-fail.log`、`$EVID/ci-fail-diagnosis.txt`

---

## 10. 状态登记与收口（W-CID-10 · D-1 → AC-B4-110 ⑤）

> 只追加 §22，不重写 §21；§22 **禁止任何 URL**（见 §0.6 关键约束）。

- [ ] **W-CID-10.1 追加 `## §22 实现登记（CI cargo-deny 修复）`** · `docs/progress/b4-status.md`
  - 追溯：AC-B4-110 ⑤ · spec §5.4-3、§8.2 AC-110 · design §2.4.5、D-B4-P142
  - 改动内容：文件末尾追加一节，含 8 个子表：基线（HEAD、修前 `all=3`、F-11~F-15 摘要）、变更集（R-1/R-2/R-3/C-1 的 numstat，含 lock `4 4` 实际形态）、偏差（`form_urlencoded` 必要性、`syn` 副作用恢复、负例命令对 spec §8.3 的修正）、AC 三态（105~110）、CI 证据（run-id、compliance 结论、版本行、四条汇总行）、复跑纪律（取证日期）、决策（D-B4-P135~P143）、边界（零源码改动、无新增依赖/target、`NEW_TARGETS=25`、B3/B4/layering 复跑结果）
  - 依赖：W-CID-09.3（需要 run-id 与 CI 结论）
  - 禁止：不得重写/删除 §21；不得写任何 URL、`sk-` 长串、`Bearer`、模型名前缀、`request_id`；不得把 `.tmp` 路径写成仓库内相对链接；不得只写"通过"而缺少命令/计数
  - DoD：`grep -c '^## §22'`=**1**；§22 内 `AC-B4-10[5-9]|AC-B4-110` 覆盖 6 条；`D-B4-P135`、`D-B4-P143` 各≥**1**；§22 内 `https?://`=**0**；`grep -c 'D-B4-P131'`≥**1**（§21 既有）
  - 证据：
    ```bash
    set -e
    [ "$(grep -c '^## §22' docs/progress/b4-status.md)" = "1" ]
    S="$(sed -n '/^## §22/,$p' docs/progress/b4-status.md)"
    [ "$(printf '%s\n' "$S" | grep -cE 'AC-B4-(10[5-9]|110)')" -ge 6 ]
    [ "$(printf '%s\n' "$S" | grep -c 'D-B4-P135')" -ge 1 ]
    [ "$(printf '%s\n' "$S" | grep -c 'D-B4-P143')" -ge 1 ]
    [ "$(printf '%s\n' "$S" | grep -cE 'https?://')" = "0" ]
    [ "$(grep -c 'D-B4-P131' docs/progress/b4-status.md)" -ge 1 ]
    [ "$(grep -c 'AC-B4-105' docs/progress/b4-status.md)" -ge 1 ]
    ```
  - 证据产物：`docs/progress/b4-status.md`（入库；本节为唯一入库证据）

- [ ] **W-CID-10.2 AC-B4-105~110 三态逐条赋值（只允许命令实测支撑）** · `docs/progress/b4-status.md §22`
  - 追溯：AC-B4-105~110 · spec §8.1-3、§8.4-4 · design §1.1.3、§2.5.5、§2.5.6
  - 改动内容：为 6 条 AC 逐条写 `passed`/`pending`/`not-applicable` + 证据摘要（命令名 + 退出码/计数 + 日志文件名，不含 `.tmp` 的绝对路径）
  - 依赖：W-CID-10.1、W-CID-01~09
  - 禁止：不得因"配置已写"把 AC 置 `passed`；不得离线把 AC-107 置 `passed`；不得把 `gh` 失败置 `not-applicable`
  - DoD：6 条 AC 均有状态与证据；`pending`/`not-applicable` 项附"原因 + 复跑条件"；状态词仅使用三态之一
  - 证据：
    ```bash
    set -e
    sed -n '/^## §22/,$p' docs/progress/b4-status.md | grep -cE '\| *AC-B4-10[5-9] *\||\| *AC-B4-110 *\|'
    sed -n '/^## §22/,$p' docs/progress/b4-status.md | grep -cE 'passed|pending|not-applicable'
    ```
  - 证据产物：同 §22

- [ ] **W-CID-10.3 §22 追加后门禁复跑（doc-plan / zero-transcript / record）** · 仓库根
  - 追溯：AC-B4-110 ⑤ · design §2.7.1、§2.7.2 · `scripts/check-b4-gates.sh:239-295`
  - 改动内容：仅执行
  - 依赖：W-CID-10.2
  - 禁止：不得因 zero-transcript 命中而删除 §22 的 AC 证据行（应删 URL/敏感串）
  - DoD：`doc-plan=0`、`zero-transcript=0`、`record=0`；`scan-secrets-docs.sh --all-md` 退 **0**
  - 证据：
    ```bash
    set +e
    rc=0
    for r in doc-plan zero-transcript record; do
      bash scripts/check-b4-gates.sh --rule="$r"; rrc=$?
      echo "$r=$rrc"
      [ "$rrc" = "0" ] || rc=1
    done
    bash scripts/scan-secrets-docs.sh --all-md; sd=$?
    echo "secrets-docs=$sd"
    [ "$rc" = "0" ]
    [ "$sd" = "0" ]
    ```
  - 证据产物：`$EVID/90-status-gates.log`

- [ ] **W-CID-10.4 §22 单独提交（§21 保持原提交历史）** · git
  - 追溯：AC-B4-110 ⑤ · design §2.4.5 "不重写 §21"
  - 改动内容：仅提交 `docs/progress/b4-status.md`
  - 依赖：W-CID-10.3
  - 禁止：不得与四文件修复合并成 amend；不得改动 §21 段落
  - DoD：`git show --stat --oneline HEAD` 恰 **1** 个文件；`git log --oneline -3` 中四文件提交与 §22 提交为两个独立提交；`git status --short` 为空
  - 证据：
    ```bash
    set -e
    git add docs/progress/b4-status.md
    git commit -m "docs(b4): 登记 §22 cargo-deny 修复实现与 AC 三态"
    git show --stat --oneline HEAD
    [ "$(git show --pretty=format: --name-only HEAD | sed '/^$/d' | wc -l | tr -d ' ')" = "1" ]
    git log --oneline -3
    git status --short
    ```
  - 证据产物：`$EVID/91-status-commit.txt`

---

## 11. 出口总控（W-CID-11 · AC-B4-105~110）

- [ ] **W-CID-11.1 一条串联命令复现全部硬判据** · 仓库根
  - 追溯：AC-B4-105~110 · design §2.7.4、§2.9
  - 改动内容：仅执行；用于最终交付前一把过
  - 依赖：W-CID-10.2（§22 已写）
  - 禁止：不得跳步；任一非 0 必须定位到主任务，不得"重跑碰运气"
  - DoD：整段退 **0**；输出末尾 `all-gates=0`
  - 证据：
    ```bash
    set +e
    rc=0
    cargo deny check licenses bans sources advisories > "$EVID/99-deny.log" 2>&1; deny=$?
    echo "deny=$deny"
    [ "$deny" = "0" ] || rc=1
    [ "$(grep -c 'unused-allowed-license = "allow"' deny.toml)" = "1" ] || rc=1
    [ "$(grep -c 'version = "0.1.0"' crates/ui-bridge/Cargo.toml)" = "2" ] || rc=1
    test -z "$(git diff --stat HEAD -- 'crates/*/src/**')" || rc=1
    git diff --quiet HEAD -- Cargo.toml || rc=1
    echo "--- numstat（提交前采集口径，以 W-CID-07.1 为准）---"
    cat "$EVID/numstat.txt"
    bash scripts/check-layering.sh; layer=$?
    echo "layer=$layer"; [ "$layer" = "0" ] || rc=1
    bash scripts/check-b3-gates.sh; b3=$?
    echo "b3=$b3"; [ "$b3" = "0" ] || rc=1
    for r in source budget record outbound zero-transcript doc-plan targets; do
      bash scripts/check-b4-gates.sh --rule="$r"; rrc=$?
      echo "$r=$rrc"; [ "$rrc" = "0" ] || rc=1
    done
    bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt || rc=1
    [ "$rc" = "0" ]
    echo "all-gates=0"
    ```
  - 证据产物：`$EVID/99-final.log`
  - 备注：`deny.toml` 的 numstat 在 §22 提交后仍以 `HEAD` 为基线会变成 0；因此本任务应在 W-CID-09.2 **之后、W-CID-10.4 之前**，或把基线改为 `HEAD~1`（若 §22 已提交）。判据以 §7.1 的 `$EVID/numstat.txt` 为准，本节不重复该断言的最终效力。

- [ ] **W-CID-11.2 出口判据表逐项打勾** · 文档
  - 追溯：AC-B4-105~110 · design §2.9-1~20
  - 改动内容：把 §0.7 与 §0.5 的判据逐项对照实测值，写入 §22 的"出口判据"小节
  - 依赖：W-CID-11.1
  - 禁止：不得把"未执行"写成"通过"
  - DoD：6 条 AC 全有状态；4 处 numstat 以 `$EVID/numstat.txt`（提交前采集）为准且精确；零 diff 4 项为空；门禁 11 项 0；CI 结论已记录
  - 证据：
    ```bash
    set +e
    cat "$EVID/numstat.txt"
    awk '$3=="deny.toml"{print "deny=" $1" "$2}' "$EVID/numstat.txt"
    awk '$3=="crates/ui-bridge/Cargo.toml"{print "uib=" $1" "$2}' "$EVID/numstat.txt"
    awk '$3=="Cargo.lock"{print "lock=" $1" "$2}' "$EVID/numstat.txt"
    awk '$3==".github/workflows/ci.yml"{print "ci=" $1" "$2}' "$EVID/numstat.txt"
    test -z "$(git diff --stat HEAD -- 'crates/*/src/**')"
    git diff --quiet HEAD -- Cargo.toml
    git log --oneline -3
    ```
  - 证据产物：`docs/progress/b4-status.md §22` 出口判据小节

- [ ] **W-CID-11.3 不宣称清单与后续动作登记** · `docs/progress/b4-status.md §22`
  - 追溯：spec §10-2 · design §2.5.6、§2.8.4
  - 改动内容：登记四条不宣称：`licenses ok` ≠ 法律合规；`advisories ok` 仅对当日快照；`bans ok` 时 `multiple-versions` 仍 warn；本次不新增 Rust 用例/target
  - 依赖：W-CID-11.2
  - 禁止：不得宣称"供应链已无漏洞"或"许可证全面达标"
  - DoD：§22 内四条不宣称各≥**1**；`NEW_TARGETS=25` 与 `--rule=targets` 结果同时出现
  - 证据：
    ```bash
    set -e
    S="$(sed -n '/^## §22/,$p' docs/progress/b4-status.md)"
    [ "$(printf '%s\n' "$S" | grep -c '不宣称')" -ge 4 ]
    printf '%s\n' "$S" | grep -c 'NEW_TARGETS'
    ```
  - 证据产物：同 §22

---

## 附 A：提交顺序与依赖图

```text
S0  docs(b4): 登记 §21 cargo-deny 需求增量        ← §0.4 闸门，必须先做
     │
S1  R-1 deny.toml（W-CID-02）  ─┐
    R-3 ui-bridge（W-CID-03）   ├─ 互不依赖，可并行修改；均不依赖网络
    （建议 R-3 → R-1，按 design §2.1.3 的 fork 顺序）
     │
S2  R-2 Cargo.lock（W-CID-04）  ← 依赖 crates.io 索引；放最后，避免副作用干扰前两项判定
     │
S3  C-1 ci.yml（W-CID-06）      ← 与前四项无代码依赖，但在提交上必须同批
     │
S4  负例 + 预算 + 门禁（W-CID-05/07/08）
     │
S5  fix(ci): 四文件同一提交       ← deny.toml + ui-bridge + Cargo.lock + ci.yml
     │
S6  push → compliance success    ← AC-B4-109 ④，取 run-id
     │
S7  docs(b4): 登记 §22            ← 依赖 S6 的 run-id；单独提交
     │
S8  push（§22）→ 复查 zero-transcript / secrets-docs
```

**依赖与并行规则**

| 关系 | 说明 |
|---|---|
| W-CID-02 ∥ W-CID-03 | 无相互依赖，可并行；`bans` 只受 W-CID-03 影响，`licenses` 只受 W-CID-02 影响 |
| W-CID-04 → W-CID-02/03 | 必须在两项绿之后执行，避免 `cargo update` 的解析副作用干扰单项判定 |
| W-CID-06 → W-CID-04 | 建议在四项改动完成后插入，便于一次验证 |
| W-CID-08 → W-CID-07 | 预算先于门禁；预算不过时无需跑门禁 |
| W-CID-09 → W-CID-10 | §22 需要 run-id；无 run-id 时 AC-109 ④ 记 `pending` |
| W-CID-10 → W-CID-11 | 出口总控在 §22 落盘后执行 |

---

## 附 B：阶段出口一览

| 主任务 | 子任务 | 出口判据（摘要） |
|---|---|---|
| W-CID-01 基线 | 3 | `all=3`；allow=15；四个单项 `0/1/2/0`；备份齐 |
| W-CID-02 R-1 | 3 | numstat `1 0`；`lic=0`；未命中 0；汇总 `0/0`；15 项 diff 空 |
| W-CID-03 R-3 | 3 | numstat `2 2`；metadata 0；`bans=0`；wildcard 0；版本一致 |
| W-CID-04 R-2 | 4 | numstat `4 4`；仅 4 行；包集合不变；metadata 0；`adv=0`；公告 0 |
| W-CID-05 负例 | 3 | 退 4；`rejected`≥1；不入库；高危 0 |
| W-CID-06 C-1 | 3 | numstat `2 0`；版本行前 3 行；命令串 1；`all=0` |
| W-CID-07 预算 | 4 | 4 处 numstat 精确；4 项零 diff；硬红线全绿；inventory 0 |
| W-CID-08 门禁 | 4 | B3 0；B4 7×0 + self-test 0；layering 0；SBOM 0；fmt/clippy/test 0 |
| W-CID-09 提交 | 4 | 4 文件同提交；push；compliance success；失败分支有归类 |
| W-CID-10 登记 | 4 | §22=1 节；6 条 AC 三态；无 URL；门禁复跑 0；单独提交 |
| W-CID-11 总控 | 3 | 串联全 0；判据逐项打勾；不宣称 4 条 |
| **合计** | **38** | AC-B4-105~110 全覆盖；6 条 AC 均有命令-计数证据 |

---

## 附 C：禁止清单（硬红线，任一违反即回退）

1. 禁止在 `[advisories].ignore` 加入 `RUSTSEC-2026-0592` 或任何漏洞。
2. 禁止把 `[bans].wildcards` 改为 `"warn"`/`"allow"`，或新增 `exceptions` 豁免。
3. 禁止把 `[sources].unknown-registry`/`unknown-git` 改为非 `"deny"`。
4. 禁止删改 `[licenses].allow` 的 15 项（含顺序），禁止加 `"*"`/`UNLICENSED`，禁止用 `exceptions` 通配。
5. 禁止在 CI 命令加 `--deny warnings`、`-A`、`-W`、`-L off`；禁止把版本打印塞进 `licenses.run`。
6. 禁止浮动/降级 `cargo-deny@0.22.0`（不得改为 `latest`）。
7. 禁止手工伪造 `Cargo.lock` checksum；禁止接受 `cssparser-macros` 的 `syn` 降级。
8. 禁止把负例配置写入仓库或提交；禁止为让负例复现而删仓库白名单项。
9. 禁止改 `crates/**/src/**`、`scripts/**`、根 `Cargo.toml`；禁止新增依赖或 Rust target。
10. 禁止拆分四文件提交（`deny.toml` 与 `Cargo.lock` 必须同一提交）。
11. 禁止在 §22 写任何 URL、`sk-` 长串、`Bearer`、模型名前缀、`request_id`（`--rule=zero-transcript` 会扫 `b4-status.md`）。
12. 禁止在 AC 未实测时置 `passed`；禁止离线把 AC-107 置 `passed`。