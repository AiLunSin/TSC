# readme_update · 编码任务清单（根 README 更新）

> **阶段**：第三阶段（编码任务规划）。本文件把 `.codeartsdoer/specs/readme_update/spec.md`（856 行，现状诊断 §1.5、12 章结构 §6.1、事实表 §6.2、禁止措辞 §6.3、验收 RD-AC-01~20 §7.2）翻译成可执行、可机器验收的任务；**本文件不修改 `README.md`**，只定义后续执行阶段怎么做、怎么验。
> **唯一交付物**：根 `README.md`（正文 120~200 行、12 个二级章节、单一 H1）。
> **唯一需求基线**：spec §6.1（结构）+ §6.2（事实值）+ §6.3（禁止措辞）+ §7.2（RD-AC-01~20）；与任何其他文档冲突时以 AC 为准。
> **事实来源（只读）**：`docs/progress/b4-status.md` §1~§22、`docs/progress/ci-readiness.md`、`.github/workflows/ci.yml`、`deny.toml`、`Cargo.toml`、`scripts/*.sh`、`crates/*/src/**`、`crates/*/Cargo.toml`、`.codeartsdoer/specs/{b4_gate,ci_deny_repair}/**`。
> **本次不改**：`crates/**`、`app/**`、`app/src-tauri/**`、`Cargo.toml`、`Cargo.lock`、`deny.toml`、`.github/workflows/ci.yml`、`scripts/**`、`docs/sdd/**`、`docs/compliance/**`；不新增依赖；不新增/改写任何 `AC-B4-*` 编号（使用 `RD-AC-01~20`）。

---

## 0. 执行口径

### 0.1 环境前置（每个 shell 会话执行一次）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$HOME/.cargo/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export EVID="$PWD/.tmp/readme-update/$(date +%Y%m%d)"; mkdir -p "$EVID"
```

- 所有命令在项目根 `TSC/` 下用 **Windows git-bash** 执行；PowerShell 写法只作为"可选"，不得作为主路径。
- `PATH` 必须同时含 rustup toolchain 与 `~/.cargo/bin`；缺后者会让 `cargo deny` 内部的 `cargo metadata` 失败（属环境问题，不计入 AC）。
- `$EVID` 位于 `.tmp/`（已 gitignore），证据一律入库外保存。

### 0.2 编号与书写格式

- 主任务 `W-RD-<n>`（全局 01..09）；子任务 `W-RD-<n>.<m>`。
- 每个子任务至少含：**改动（文件/章节）**、**依赖**、**DoD**（命令 + 期望退出码 + grep 计数）、**证据**（可直接粘贴）。
- 最多两级：`##` 主任务、`- [ ]` 子任务；不出现第三级复选框。
- 任务描述用祈使句、无主语；只写"做什么/做到什么程度"，不写实现代码。

### 0.3 冻结口径（用户已确认，执行时不再权衡）

| 编号 | 冻结内容 | 落点 |
|---|---|---|
| **C-RD-01** | README 按仓库实况写：AC-B4-01~94 **passed**；AC-B4-95~104 **pending**（U-06 未开工，`gate/replay.rs` 仍为 65 字节占位）；AC-B4-105~110 **本地 passed、远端 CI 取证待确认** | W-RD-07.1 |
| C-RD-02 | `.codeartsdoer/` 被 `.gitignore` 忽略，**不得**作为 README 链接目标；只做文字说明"本地工作区、未入库" | W-RD-03.4、W-RD-07.2 |
| C-RD-03 | "门禁"≠CI：`check-b3-gates.sh`/`check-b4-gates.sh`/`check-mac-scope.sh` 当前**不在** `ci.yml` | W-RD-05.1~5.3 |
| C-RD-04 | 审计 = 设计约束 180 天 + 当前内存实现；检索源 = 端口契约、真实源未接入；平台层 = 空壳 | W-RD-03.3、W-RD-06.4 |
| C-RD-05 | 不保留硬编码回归数字；一律改为"命令/脚本口径"，必要时标"截至日期 + 命令" | W-RD-07.1 |
| C-RD-06 | 不新增 `AC-B4-111+`；验收用 `RD-AC-01~20` | 全文件 |
| C-RD-07 | 正文简体中文；命令、路径、变量名、crate 名保持英文；不用营销词/emoji | 全文件 |
| 边界-1 | 38 只产卡片数据、39 只做引用对齐、51 只做视图/点击契约；设置向导不落盘明文；平台层为空壳；Windows 主平台、macOS 不适用 | W-RD-03.3、W-RD-06 |
| 边界-2 | 不伪造 CI 绿、不写真实端点/密钥/模型名/request_id、不新增依赖 | W-RD-08.2、W-RD-09.1 |
| 边界-3 | `--rule=doc-plan`、`--rule=zero-transcript`、`scan-secrets-docs.sh --all-md`、`fmt --check` 必须退 0 | W-RD-09.1 |

### 0.4 开工前置闸门（硬）

1. `git status --short` 必须为空；若非空，先单独提交既有改动，再开工（否则"零 diff / 仅 README 变更"类判据基线不清）。
2. `README.md` 必须为当前 130 行版本；`awk 'END{print NR}' README.md` = `130`。若行数不同，先按 W-RD-01.3 重做旧段落映射表。
3. `crates/application/src/gate/replay.rs` 必须仍为 65 字节占位（`wc -c` = `65`）；若已被实现，**停止并上报**——C-RD-01 的 pending 口径与 AC 三态须重新裁决。
4. 不满足以上任一条，停止执行，不得以"先写再说"绕过。

### 0.5 目标 README 结构与旧段落处置（spec §6.1 / §8.1 的落地表）

README 必须且只按下列顺序包含 12 个 `##` 章节（标题文字可微调，语义不可缺）：

| # | 章节标题（建议用字） | 旧 README 段落来源 | 动作 |
|---|---|---|---|
| 1 | 项目定位与当前能力边界 | L1~L5（标题 + 一句话 + 首版约束） | 改写：目标与现状分块；首版约束保留但加"未验收" |
| 2 | 快速开始 | L50~L59、L96~L102 | 新增/重组：前置条件、PATH/TEMP、build/test、前端、桌面壳 |
| 3 | 架构与 crate 职责 | L29~L48 | 重写：补 `gate/`、`cards/`、`citation/`、`keyword_card/`、`settings/`；平台层写空壳 |
| 4 | 构建与测试 | L61~L68、L70~L79 | 保留 + 补：`--locked` 说明、9 ignored、SQLCipher/nasm 可选 |
| 5 | 门禁与 CI | L81~L94 | 重写：本地/CI 分栏；补 `check-b4-gates.sh`；cargo-deny 同版同命令 |
| 6 | LLM 配置入口 | 无（散落于 L126） | 新增：默认关闭、变量名、设置向导仅契约、凭据不落盘 |
| 7 | 关键词卡片能力 | 无 | 新增：B4 契约、38/39/51 边界、未交付声明 |
| 8 | 隐私与凭据边界 | L120~L127 | 改写：零转录、日志白名单、审计/SPI/密钥三态 |
| 9 | 当前进度与未完成项 | L7~L27 | 重写：B0~B4、AC 三态、U-06/07/08、9 ignored；删除 L18 硬编码数字 |
| 10 | 文档索引 | L104~L119 | 重写：补 `b4-status.md`、`ci-readiness.md`、`tasks-b2-offline-closure.md`；`.codeartsdoer` 不入库 |
| 11 | 贡献 | 无 | 新增：最小步骤、门禁、依赖纪律 |
| 12 | 许可证 | L128~L130 | 保留 SPDX + 补白名单边界与"不构成法律意见" |

**必须删除的段落**：L3（"监听会议对话、自动捕捉要点、主动联网补全信息"写成已具备能力）、L5 的完成态语气、L18 的 `45 targets / 560 passed`、L20 的"未完成项只指向 b3-status 与 tasks.md"、L38~L39 的 `WASAPI / UIA / OCR` 与 macOS"能力矩阵"、L93 的 `cargo deny --offline ...` 作为唯一命令、L100 的 `npm run dev` 与桌面壳并列（会被读成启动桌面）、L122 的"检索源仅通过开放 SPI 接入"、L125 的"审计记录只追加，保留 180 天"无限定。

### 0.6 证据与提交纪律

- 每完成一个主任务，把 DoD 输出写入 `$EVID/W-RD-<n>-<step>.txt`；证据文件不入库。
- 提交顺序固定为：**A**（`README.md` 单文件）→ **B**（可选，`docs/progress/b4-status.md` 追加 `## §24 README 更新记录`）。
- 禁止 `git add -A`；禁止把 `.tmp/`、`docs/compliance/sbom/*.json`、代码/脚本/配置混入提交；禁止 `--amend`、禁止 push。

---

## 1. W-RD-01 事实基线核对与冲突裁决取证（不改 README）

> 目的：在动笔前把"要写进 README 的每个易漂移值"实测一遍，避免把 spec 的时点断言直接抄进 README。
> 依赖：§0.4 全部满足。

### 1.1 采集事实快照
- [ ] 采集 crate 清单与版本：`grep -n 'members\|rust-version\|license\|edition' Cargo.toml`，确认 7 个 member（含 `app/src-tauri`）、`rust-version = "1.89"`、`license = "MIT OR Apache-2.0"`，写入 `$EVID/facts-crates.txt`。
- [ ] 采集平台层空壳证据：`wc -l crates/platform-win/src/lib.rs crates/platform-mac/src/lib.rs`，并 `grep -c 'PLATFORM_NAME' crates/platform-*/src/lib.rs`，确认每文件仅一处常量。
- [ ] 采集 U-06 未开工证据：`wc -c crates/application/src/gate/replay.rs`（= 65）、`test -e crates/application/tests/gate_replay_driver.rs; echo "driver=$?"`（期望 1），写入 `$EVID/facts-u06.txt`。
- [ ] 采集 9 ignored 证据：`cargo test --workspace -- --list --ignored 2>/dev/null | grep -c ': test'`，期望 `9`，写入 `$EVID/facts-ignored.txt`（首次需全量编译，耗时较长，建议单独一次执行）。
- [ ] 采集 cargo-deny 版本与命令：`cargo deny --version`（期望 `cargo-deny 0.22.0`）、`grep -n 'cargo deny check licenses bans sources advisories' .github/workflows/ci.yml`（期望 1 行命中），写入 `$EVID/facts-deny.txt`。
- [ ] 采集 `deny.toml` 白名单条目数：`sed -n '/^allow = \[/,/^\]/p' deny.toml | grep -c '^  "'`，期望 `15`；并记录 `unused-allowed-license = "allow"` 存在（第 17 行）。
- **依赖**：无（本任务为其余全部任务的前置）。
- **DoD**：`$EVID` 下 `facts-crates.txt`/`facts-platform.txt`/`facts-u06.txt`/`facts-ignored.txt`/`facts-deny.txt` 共 5 个文件（W-RD-01.2 再产 `facts-targets.txt`）逐个 `test -s` 退 0；`facts-ignored.txt` 内容为 `9`；`facts-u06.txt` 含 `65` 与 `driver=1`。
- **证据**：
  ```bash
  { awk 'END{print NR}' Cargo.toml; grep -n 'rust-version\|license\|members' Cargo.toml; } > "$EVID/facts-crates.txt"
  { wc -l crates/platform-win/src/lib.rs crates/platform-mac/src/lib.rs; grep -c 'PLATFORM_NAME' crates/platform-win/src/lib.rs crates/platform-mac/src/lib.rs; } > "$EVID/facts-platform.txt"
  { wc -c crates/application/src/gate/replay.rs; test -e crates/application/tests/gate_replay_driver.rs; echo "driver=$?"; } > "$EVID/facts-u06.txt"
  cargo test --workspace -- --list --ignored 2>/dev/null | grep -c ': test' > "$EVID/facts-ignored.txt"
  { cargo deny --version; grep -n 'cargo deny check licenses bans sources advisories' .github/workflows/ci.yml; } > "$EVID/facts-deny.txt"
  ```

### 1.2 `NEW_TARGETS` 实测与 C-RD-01 冲突裁决
- [ ] 实测条目数：`awk -F"'" '/^NEW_TARGETS=/{n=split($2,a," "); print n}' scripts/test-name-inventory.sh`，记录到 `$EVID/facts-targets.txt`。
- [ ] 按实测值决定 README 写法（**不得两种写法并存**）：
      a. 实测 = 25 → 写 "`NEW_TARGETS` 当前 25 项（含历史增量）"；
      b. 实测 ≠ 25 → 写 "`NEW_TARGETS` 以 `scripts/test-name-inventory.sh` 为准；截至 2026-11-22 实测 `<实测数>` 项（`gate_replay_driver` 随 U-06 落地后为 25 项）"，并在 `$EVID/D-RD-01.txt` 登记偏差（原因：U-06 未开工、target 文件未落地）。
- [ ] 复核 7 条 `--rule=targets` 循环项均在清单内：`bash scripts/check-b4-gates.sh --rule=targets; echo "rc=$?"`，期望 `rc=0`（当前 7 项均在 `NEW_TARGETS` 中）。
- **依赖**：1.1。
- **DoD**：`facts-targets.txt` 首行为整数；`check-b4-gates.sh --rule=targets` 退出码 = 0；若实测 ≠ 25，`D-RD-01.txt` 存在且含实测数与日期。
- **证据**：
  ```bash
  awk -F"'" '/^NEW_TARGETS=/{n=split($2,a," "); print n}' scripts/test-name-inventory.sh | tee "$EVID/facts-targets.txt"
  bash scripts/check-b4-gates.sh --rule=targets; echo "targets_rc=$?" | tee -a "$EVID/facts-targets.txt"
  ```

### 1.3 旧段落映射与链接清单核对
- [ ] 复核 §0.5 表格：`awk 'END{print NR}' README.md` = `130`；`sed -n '18p;38,39p;93p;100p;122p;125p' README.md` 逐行确认待删除内容仍在原位。
- [ ] 生成"必链文档存在性"清单：对 `docs/sdd/{spec,design,tasks,tasks-b2-offline-closure}.md`、`docs/progress/{b0,b1,b2,b3,b4}-status.md`、`docs/progress/{b2-offline-closure,b2-24-merge,ci-readiness}.md`、`docs/progress/test-names-before.txt`、`docs/compliance/license-review.md`、`docs/baseline/asr.md` 逐个 `test -s`，把结果写入 `$EVID/links-baseline.txt`（期望全部为 `ok`）。
- [ ] 确认 `.codeartsdoer/` 被忽略：`grep -n '^\.codeartsdoer/' .gitignore`（期望 1 行命中）。
- **依赖**：1.1。
- **DoD**：`README.md` 行数 = 130；`links-baseline.txt` 中 `missing` 计数为 0；`.gitignore` 命中 `.codeartsdoer/`。
- **证据**：
  ```bash
  awk 'END{print NR}' README.md
  for f in docs/sdd/spec.md docs/sdd/design.md docs/sdd/tasks.md docs/sdd/tasks-b2-offline-closure.md \
           docs/progress/b0-status.md docs/progress/b1-status.md docs/progress/b2-status.md \
           docs/progress/b3-status.md docs/progress/b4-status.md docs/progress/b2-offline-closure.md \
           docs/progress/b2-24-merge.md docs/progress/ci-readiness.md docs/progress/test-names-before.txt \
           docs/compliance/license-review.md docs/baseline/asr.md; do
    test -s "$f" && echo "ok $f" || echo "missing $f"; done | tee "$EVID/links-baseline.txt"
  grep -n '^\.codeartsdoer/' .gitignore
  ```

---

## 2. W-RD-02 第 1 章：项目定位与当前能力边界

> 目标：让读者在 30 秒内分清"目标"与"当前"。首屏必须在无滚动/一次滚动内完成"标题 → 一句话目标 → 当前阶段 → 能力边界"。
> 依赖：W-RD-01。

### 2.1 重建唯一 H1 与首屏三段
- [ ] 把 `README.md` 的 L1~L5 改写为：唯一 `# Meeting Copilot（会议副驾）` + 不超过 50 字的目标句 + 独立的"当前阶段"句（例：`当前阶段：B4 门禁与关键词卡片离线契约已交付；真实采集、真实会议平台与产品级 UI 未验收。`）。
- [ ] 把原 L5"首版约束：全自动主动提示；强制对接腾讯会议/飞书/Zoom"改为"目标与约束"，句尾加"**真实适配与 G2 未验收**"限定。
- **依赖**：无（须在 W-RD-03 之前完成）。
- **DoD**：`grep -c '^# ' README.md` = 1；`grep -c '^## ' README.md` ≥ 1；首 15 行内命中 `当前阶段`；`grep -cE '已实现全自动主动提示|已对接腾讯会议|已对接飞书|已对接 Zoom' README.md` = 0。
- **证据**：
  ```bash
  grep -c '^# ' README.md
  sed -n '1,15p' README.md
  grep -nE '已实现全自动主动提示|已对接腾讯会议|已对接飞书|已对接 Zoom' README.md; echo "rc=$?"
  ```

### 2.2 能力三态与真实采集边界
- [ ] 在第 1 章内新增"能力三态"小节：用三行分别标注"已实现 / 仅有契约 / 未实现或未验证"，并把 B4 门控、38/39/51、平台层、真实端点分别归入对应态。
- [ ] 写明真实采集边界句（与 spec §5.1.1-3 一致）：`采集协议与离线状态机已实现；真实音频捕获、腾讯会议/飞书/Zoom 适配与真机稳定性未验收（G2 未通过）。`
- **依赖**：2.1。
- **DoD**：命中 `已实现`、`仅有契约`、`未实现或未验证` 三个词各 ≥1；命中 `真实音频`/`未验收`；`grep -c '产品级' README.md` 出现的每一处均须是否定句（人工复核 + `grep -n '产品级' README.md` 逐行确认）。
- **证据**：
  ```bash
  grep -nE '已实现|仅有契约|未实现或未验证' README.md
  grep -n '真实音频\|未验收' README.md
  grep -n '产品级' README.md
  ```

### 2.3 平台边界三句
- [ ] 在第 1 章写明：`Windows 为主平台（git-bash 验证）；macOS 不适用（无设备）；CI 的 macOS 条目仅作交叉编译观察，不构成支持。`
- **依赖**：2.1。
- **DoD**：命中 `Windows`、`不适用（无设备）`、`交叉编译观察`；`grep -cE '三平台支持|macOS 支持|mac 支持' README.md` = 0。
- **证据**：
  ```bash
  grep -nE '不适用（无设备）|交叉编译观察' README.md
  grep -nE '三平台支持|macOS 支持|mac 支持' README.md; echo "rc=$?"
  ```

---

## 3. W-RD-03 第 2~3 章：快速开始 + 架构与 crate 职责

> 目标：读者复制命令即可跑通；架构只给"目录 → 职责 → 边界"，不展开实现。
> 依赖：W-RD-01（事实快照）、W-RD-02（保证章节顺序从 1 开始）。

### 3.1 前置条件与 PATH/TEMP
- [ ] 在第 2 章写前置条件：Rust（`rust-version = "1.89"`，CI 固定 1.100.0）、Node 22、Windows git-bash；SQLCipher 测试另需 nasm（仅 CI `sqlcipher` job）。
- [ ] 给出可复制命令块（`bash` 围栏），**必须同时含** toolchain 与 `~/.cargo/bin`：
      `export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$HOME/.cargo/bin:$PATH"` 与 `export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"`。
- [ ] 补一句说明：缺少 `~/.cargo/bin` 时 `cargo deny` 内部的 `cargo metadata` 会失败。
- **依赖**：W-RD-02 完成（章节定位已定）。
- **DoD**：`grep -F 'export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$HOME/.cargo/bin:$PATH"' README.md` ≥ 1；`grep -F 'mkdir -p "$TEMP"' README.md` ≥ 1；命中 `1.89` 与 `Node 22`。
- **证据**：
  ```bash
  grep -nF '$HOME/.cargo/bin' README.md
  grep -nF 'mkdir -p "$TEMP"' README.md
  grep -nE '1\.89|Node 22|nasm' README.md
  ```

### 3.2 Rust / 前端 / 桌面壳命令
- [ ] 第 2 章写 Rust 四条：`cargo build --workspace`、`cargo test --workspace`、`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`。
- [ ] 第 2 章写前端五条（`cd app` 后）：`npm ci`、`npm run lint`、`npm run typecheck`、`npm run check:bindings`、`npm run build`。
- [ ] 第 2 章写桌面壳：`cargo build -p meeting-copilot-desktop` / `cargo run -p meeting-copilot-desktop`，并明确 `npm run dev` **仅启动 Vite 前端**（不启动 Tauri 窗口）。
- **依赖**：3.1。
- **DoD**：`grep -F` 逐条命中上述 4+5+2 条命令（每条 ≥1）；`grep -c 'npm run tauri dev' README.md` = 0；命中 `仅前端` 或等价句。
- **证据**：
  ```bash
  for c in 'cargo build --workspace' 'cargo test --workspace' 'cargo fmt --all --check' \
           'cargo clippy --workspace --all-targets -- -D warnings' 'npm run lint' 'npm run typecheck' \
           'npm run check:bindings' 'npm run build' 'cargo build -p meeting-copilot-desktop' \
           'cargo run -p meeting-copilot-desktop'; do
    printf '%s => %s\n' "$c" "$(grep -cF -- "$c" README.md)"; done
  grep -n 'npm run tauri dev' README.md; echo "rc=$?"
  ```

### 3.3 crate 职责与平台空壳
- [ ] 第 3 章列 7 个成员：`crates/{domain,application,infrastructure,platform-win,platform-mac,ui-bridge}` + `app/src-tauri`，并按实际目录写职责：
      `domain`（模型/值对象/端口 trait）、`application`（`capture/`、`gate/`、`cards/`、`citation/`、`understand/`、治理、镜像时钟）、`infrastructure`（`store/`、`crypto/`、`audit/`、`metrics/`、`telemetry/`、`llm/`）、`ui-bridge`（`dto.rs`、`keyword_card/`、`settings/`）、`app`（React 19 + Vite）、`app/src-tauri`（Tauri 2 桌面壳）。
- [ ] 明确平台空壳句：`platform-win`/`platform-mac` 当前仅 `PLATFORM_NAME` 常量与条件编译空壳，真实 WASAPI/UIA/OCR 与 macOS 适配未实现。
- [ ] 写明分层约束：`domain` 不依赖 `infrastructure`/`platform-*`；`application` 不直接依赖平台实现（端口 trait 注入）；本地用 `bash scripts/check-layering.sh` 验证。
- **依赖**：3.1。
- **DoD**：命中 `crates/application/src/gate` 或 `gate/`、`cards/`、`citation/`、`keyword_card/`、`settings/` 五处；`grep -c 'WASAPI / UIA / OCR' README.md` = 0；命中 `空壳` 或 `未实现`；`grep -c 'pub fn' README.md` = 0。
- **证据**：
  ```bash
  grep -nE 'gate/|cards/|citation/|keyword_card/|settings/' README.md
  grep -nE 'WASAPI / UIA / OCR' README.md; echo "rc=$?"
  grep -nE '空壳|未实现' README.md
  grep -n 'pub fn' README.md; echo "rc=$?"
  ```

### 3.4 本地 SDD 目录只作文字说明
- [ ] 在第 3 章或第 10 章写明：`.codeartsdoer/` 为本地工作区、被 `.gitignore` 忽略、克隆后不存在；**不做成链接**。
- **依赖**：3.3。
- **DoD**：`grep -c '](\.codeartsdoer/' README.md` = 0；命中 `.codeartsdoer` 且同行含"未入库"或"忽略"。
- **证据**：
  ```bash
  grep -n '\.codeartsdoer' README.md
  grep -n '](\.codeartsdoer/' README.md; echo "rc=$?"
  ```

---

## 4. W-RD-04 第 4 章：构建与测试（含 9 ignored 与可选特性）

> 目标：命令与 CI 的 `fmt`/`clippy`/`build`/`test` 对齐，并解释 `ignored` 不是失败。
> 依赖：W-RD-03（代码块风格统一后再补说明，避免重复粘贴命令）。

### 4.1 命令块补充与语言标注
- [ ] 第 4 章不重复粘贴第 2 章命令，只写增量说明：`--locked`（如需可复现构建）、`cargo test --workspace` 的耗时口径（有缓存时 10 分钟内，不承诺无缓存）。
- [ ] 确认全文所有围栏均带语言（`bash`/`text`/`toml`/`json`），无裸 ```` ``` ````。
- **依赖**：3.2。
- **DoD**：第 4 章正文 ≤ 40 行；`grep -c '^```$' README.md` = 0；`grep -cE '^```(bash|text|toml|json)' README.md` ≥ 命令块数（人工核对 ≥8）。
- **证据**：
  ```bash
  grep -c '^```$' README.md
  grep -nE '^```' README.md
  ```

### 4.2 9 ignored 与 SQLCipher
- [ ] 写明"全量测试含 9 个 `#[ignore]`，属于真实端点/真机用例；`0 failed` 不等于全部能力已验证"。
- [ ] 给出计数命令：`cargo test --workspace -- --list --ignored 2>/dev/null | grep -c ': test'`（期望 9）。
- [ ] 写 SQLCipher 为可选：`cargo test -p meeting-copilot-infrastructure --features sqlcipher`（CI `sqlcipher` job 安装 nasm）。
- **依赖**：4.1。
- **DoD**：命中 `9` 与 `ignore`（同段）；命中 `真实端点`/`真机` 与 `未验证`；`grep -F 'npm run tauri dev' README.md` 命中 0；命中 `sqlcipher`。
- **证据**：
  ```bash
  grep -nE '9 个 .*ignore|ignored.*9' README.md
  grep -n 'sqlcipher' README.md
  cargo test --workspace -- --list --ignored 2>/dev/null | grep -c ': test'
  ```

---

## 5. W-RD-05 第 5 章：门禁与 CI（本地 / CI 必须分栏）

> 目标：让维护者一眼看出"哪些检查只在本地跑、哪些在 GitHub Actions 跑"，避免把本地脚本当 CI 门禁。
> 依赖：W-RD-01.1（cargo-deny 事实）、W-RD-01.2（targets 口径）。

### 5.1 本地门禁清单
- [ ] 第 5 章设子节"本地门禁"，列 10 条（每条单独成行、可直接复制）：
      `bash scripts/check-layering.sh`、`bash scripts/scan-secrets.sh`、`bash scripts/scan-secrets-docs.sh --all-md`、`bash scripts/check-capture-offline.sh --all`、`bash scripts/check-mac-scope.sh --rule=na`、`bash scripts/check-b3-gates.sh --rule=source`、`bash scripts/check-b4-gates.sh --rule=source`、`bash scripts/check-g2-claims.sh`、`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt`、`bash scripts/sbom.sh`。
- [ ] 明确"这些脚本当前**不在** `.github/workflows/ci.yml` 中执行"。
- **依赖**：无（可与 5.2 并行，但须先于 5.3）。
- **DoD**：上述 10 条 `grep -F` 各 ≥1；命中 `不在` 与 `ci.yml`（同段）；`test -f scripts/check-b4-gates.sh` 退 0。
- **证据**：
  ```bash
  for c in 'scripts/check-layering.sh' 'scripts/scan-secrets.sh' 'scripts/scan-secrets-docs.sh --all-md' \
           'scripts/check-capture-offline.sh --all' 'scripts/check-mac-scope.sh --rule=na' \
           'scripts/check-b3-gates.sh --rule=source' 'scripts/check-b4-gates.sh --rule=source' \
           'scripts/check-g2-claims.sh' 'scripts/test-name-inventory.sh --diff' 'scripts/sbom.sh'; do
    printf '%s => %s\n' "$c" "$(grep -cF -- "$c" README.md)"; done
  ```

### 5.2 B3 / B4 规则名与退出码口径
- [ ] 写 B3 六条规则：`source`/`no-truncate`/`zero-transcript`/`fallback-untouched`/`new-targets`/`exports`。
- [ ] 写 B4 七条规则：`source`/`budget`/`record`/`outbound`/`zero-transcript`/`targets`/`doc-plan`；退出码 `0=通过 / 1=命中违规 / 2=扫描面缺失`。
- [ ] 不写 `--self-test`（若必须提，须注明"反例自检预期退 1"）。
- **依赖**：5.1。
- **DoD**：六 + 七规则名 `grep -F` 各 ≥1；命中 `0=通过` 与 `2=扫描面缺失`；`grep -c 'self-test' README.md` = 0（或每处同行含 `退 1`）。
- **证据**：
  ```bash
  docplan_mentions=$(grep -cF -- '--rule=doc-plan' README.md); echo "docplan_mentions=$docplan_mentions"
  grep -nE 'no-truncate|fallback-untouched|new-targets|exports' README.md
  grep -n 'self-test' README.md; echo "rc=$?"
  ```

### 5.3 GitHub Actions job 清单
- [ ] 设子节"GitHub Actions"，列 6 个 job 与内容：`web`（npm lint/typecheck/bindings/build + gzip 预算 819200 B）、`rust`（3 target matrix + fmt/clippy/build/test + `llm-https` 编译覆盖）、`test-name-inventory`（Windows 上 diff）、`feature-coverage`（汇总 `llm-https` 覆盖，linux 标"不适用"）、`compliance`（layering/secrets/capture-offline/g2-claims/secrets-docs/cargo deny/sbom）、`sqlcipher`。
- [ ] 写明 CI 固定 Rust `1.100.0`、Node `22`、`cargo-deny 0.22.0`。
- **依赖**：5.1。
- **DoD**：6 个 job 名 `grep -F` 各 ≥1（`web`/`rust`/`test-name-inventory`/`feature-coverage`/`compliance`/`sqlcipher`）；命中 `1.100.0`、`Node 22`、`0.22.0`；`grep -cE 'CI.*check-b[34]-gates' README.md` = 0。
- **证据**：
  ```bash
  for j in web rust test-name-inventory feature-coverage compliance sqlcipher; do
    printf '%s => %s\n' "$j" "$(grep -cF -- "$j" README.md)"; done
  grep -nE '1\.100\.0|Node 22|0\.22\.0' README.md
  ```

### 5.4 cargo-deny 同版同命令与 CI deny 状态
- [ ] 写 `cargo deny --version`（期望 `cargo-deny 0.22.0`）+ `cargo deny check licenses bans sources advisories`；说明 advisories 需网络或本地 advisory-db 缓存。
- [ ] 写 CI deny 状态：`AC-B4-105~110 本地通过；远端 CI 取证待推送后确认`。
- [ ] 全文不得出现 `cargo deny --offline`。
- **依赖**：5.3。
- **DoD**：`grep -cF 'cargo deny check licenses bans sources advisories' README.md` ≥1；`grep -c 'cargo deny --offline' README.md` = 0；命中 `本地通过` 与 `待`；`grep -cE 'CI 全绿|远端已通过' README.md` = 0。
- **证据**：
  ```bash
  grep -nF 'cargo deny check licenses bans sources advisories' README.md
  grep -n 'cargo deny --offline' README.md; echo "rc=$?"
  grep -nE 'CI 全绿|远端已通过' README.md; echo "rc=$?"
  ```

### 5.5 `NEW_TARGETS` 口径
- [ ] 按 W-RD-01.2 的裁决结论写 `NEW_TARGETS` 段（含实测值与"以脚本为准"），不得写 `45 targets`、`9 个 target`。
- **依赖**：W-RD-01.2、5.4。
- **DoD**：命中 `NEW_TARGETS`；`grep -cE '45 targets|560 passed|9 个 target' README.md` = 0；数值与 `$EVID/facts-targets.txt` 一致。
- **证据**：
  ```bash
  grep -n 'NEW_TARGETS' README.md
  grep -nE '45 targets|560 passed|9 个 target' README.md; echo "rc=$?"
  cat "$EVID/facts-targets.txt"
  ```

---

## 6. W-RD-06 第 6~8 章：LLM 配置 / 关键词卡片 / 隐私与凭据

> 目标：把"已实现 / 仅契约 / 未实现"写进能力描述，杜绝"设置向导可用""可点击卡片已交付"等夸大。
> 依赖：W-RD-05（第 5 章收口后章节号稳定）。

### 6.1 LLM 默认关闭与环境变量名
- [ ] 第 6 章写明 LLM 默认关闭、真实联调需显式开启并有预算上限。
- [ ] 只列变量名（不给值）：`LLM_BASE_URL`、`LLM_MODEL`、`LLM_API_KEY`、`LLM_LIVE_CALL_CAP`；可补 `LLM_LIVE`、`LLM_MODEL_CONFIRMED`、`LLM_ALLOWLIST`（`scripts/run-llm-live.sh` 校验项）。
- [ ] 写明凭据只经运行期环境变量注入、不写入仓库任何文件；测试记录不含凭据/完整端点/模型名/request_id。
- **依赖**：无（须先于 6.2）。
- **DoD**：4 个变量名 `grep -F` 各 ≥1；命中 `默认关闭`；`grep -cE 'sk-[A-Za-z0-9]{8,}' README.md` = 0；`grep -c '开箱即用' README.md` = 0。
- **证据**：
  ```bash
  for v in LLM_BASE_URL LLM_MODEL LLM_API_KEY LLM_LIVE_CALL_CAP; do
    printf '%s => %s\n' "$v" "$(grep -cF -- "$v" README.md)"; done
  grep -nE 'sk-[A-Za-z0-9]{8,}|开箱即用' README.md; echo "rc=$?"
  ```

### 6.2 设置向导仅契约、未接线
- [ ] 写明：51 设置向导目前只有离线视图契约（`SettingsView`/`SettingsInput`/`ProbeRequest` 等），**未接入桌面壳**；不直连网络、不读环境变量、不落盘明文；真实连通性由装配层调用既有网关。
- [ ] 写明模型/端点显示名只来自注入清单，51 只接受端点清单下标、不扩展白名单。
- **依赖**：6.1。
- **DoD**：命中 `仅`/`未接线`/`未接入` 至少两处；`grep -c '设置向导可用' README.md` = 0；`grep -cE 'https?://[^ ]+\.[a-z]+' README.md` 中不得出现真实 LLM 域名（人工复核）。
- **证据**：
  ```bash
  grep -nE '设置向导|未接线|未接入' README.md
  grep -n '设置向导可用' README.md; echo "rc=$?"
  ```

### 6.3 关键词卡片 38 / 39 / 51 边界与未交付声明
- [ ] 写 B4 契约范围：13 项卡片业务字段、只有 `Allow` 可出卡/可点击、点击本地、零转录。
- [ ] 写 38：只做确定性组装，输出 11 项可展示字段，`gate_decision_ref`/`click_state` 由装配方补齐；不调 LLM、不复制引用正文、不判定点击。
- [ ] 写 39：只做引用对齐三态 `Passed`/`Failed`/`Unknown`，落 B4 时 `Unknown→Unverified` 唯一投影；只存引用 ID 面、不联网抓正文。
- [ ] 写 51 点击：只做视图模型与点击契约（`10/4/5/6/2/2` 封闭字段），点击只本地展开（0 外发/0 LLM/0 审计）；投屏/紧急遮蔽隐藏正文；撤回不复活。
- [ ] 写明确否定句：仍不宣称产品级可点击卡片；真实 UI 渲染、焦点、投屏/遮蔽、真实引用可访问性未验收。
- **依赖**：6.1。
- **DoD**：命中 `13 项`、`Allow`、`11 项`、`唯一投影`、`10/4/5/6/2/2`（或等价列法）、`不联网`；`grep -cE '产品级可点击卡片已交付|真机可用|G4 达标' README.md` = 0；命中 `不宣称`。
- **证据**：
  ```bash
  grep -nE '13 项|11 项|唯一投影|不联网|不宣称' README.md
  grep -nE '产品级可点击卡片已交付|真机可用|G4 达标' README.md; echo "rc=$?"
  ```

### 6.4 零转录 / 日志白名单 / 审计 / SPI / 密钥三态
- [ ] 写零转录：原文、URL 全文、查询全文、凭据、模型名、request_id 不进入门控记录、审计、回放夹具、报告、日志或进度文档。
- [ ] 写日志白名单字段（平台、通道、客户端版本、会话状态、错误码、时延、提供方、信任档、卡片状态、触发类型、失败类、脱敏计数）。
- [ ] 写审计：设计约束 180 天只追加；**当前内存哈希链、持久化未实现（U-07/52）**。
- [ ] 写数据库密钥：打开存储需 OS 密钥库引用（`dpapi://`/`keychain://`），不接受明文口令；SQLCipher 为页级加密特性。
- [ ] 写检索源：`SearchProvider` 仅为领域端口契约，真实检索源未接入。
- **依赖**：6.3。
- **DoD**：命中 `零转录`、`180`、`内存`、`持久化未实现`、`dpapi://`、`keychain://`、`SearchProvider`、`未接入`；`grep -c '检索源已接入' README.md` = 0；`grep -c '审计记录只追加，保留 180 天' README.md` = 0。
- **证据**：
  ```bash
  grep -nE '零转录|180|内存|持久化未实现' README.md
  grep -nE 'dpapi://|keychain://|SearchProvider' README.md
  grep -nE '检索源已接入|审计记录只追加，保留 180 天' README.md; echo "rc=$?"
  ```

---

## 7. W-RD-07 第 9~12 章：进度 / 文档索引 / 贡献 / 许可证

> 目标：进度与 `b4-status.md` 同口径；索引只指向已入库文件；许可表述不越界为法律意见。
> 依赖：W-RD-06（第 8 章收口后章节号稳定）。

### 7.1 进度表与 AC 三态、U-06/07/08
- [ ] 第 9 章写 B0~B4 五行进度表（B0 工程基座、B1 LLM 网关、B2 离线闭环、B3 理解层、B4 门控与增量契约），每行状态与 `b4-status.md` 同口径；不得止于 B3。
- [ ] 写 AC 口径：`AC-B4-01~94 passed；AC-B4-95~104 pending（U-06 未开工，gate/replay.rs 仍为占位）；AC-B4-105~110 本地 passed、远端 CI 取证待确认`。
- [ ] 写未完成项：`U-06`（13.3 回放本体，P9 未开工）、`U-07`（41.5/42/43/52）、`U-08`（真实端点/真机/性能），三者状态均 `open`。
- [ ] 写 9 ignored 与"不宣称 G4、6s/10s p95、误触发率 ≤30%"。
- [ ] 删除旧 L18 的回归数字；如需给量级，只给命令（`cargo test --workspace`）。
- **依赖**：无（须先于 7.2）。
- **DoD**：命中 `B4` 与 `B0`（同表）；`grep -F 'AC-B4-95' README.md` ≥1 且同行含 `pending`；`U-06`/`U-07`/`U-08` 各 ≥1 且含 `open`；`grep -cE '01~104 全部闭环|B4 全部完成' README.md` = 0；`grep -cE '45 targets|560 passed' README.md` = 0。
- **证据**：
  ```bash
  grep -nE 'AC-B4-(01|94|95|104|105|110)' README.md
  grep -nE 'U-0[678]' README.md
  grep -nE '01~104 全部闭环|45 targets|560 passed' README.md; echo "rc=$?"
  ```

### 7.2 文档索引（只链已入库文件）
- [ ] 第 10 章建表链接：`docs/sdd/{spec,design,tasks,tasks-b2-offline-closure}.md`、`docs/progress/{b0,b1,b2,b3,b4}-status.md`、`docs/progress/{b2-offline-closure,b2-24-merge,ci-readiness}.md`、`docs/compliance/license-review.md`、`docs/baseline/asr.md`。
- [ ] 单独用文字说明：`.codeartsdoer/specs/**` 为本地工作区、未入库，克隆后不存在。
- [ ] 同一文档链接最多出现两次（正文一次 + 索引一次）。
- **依赖**：7.1。
- **DoD**：`grep -c '](\.codeartsdoer/' README.md` = 0；`grep -c 'b4-status.md' README.md` ≥1；`grep -c 'ci-readiness.md' README.md` ≥1；`grep -c 'tasks-b2-offline-closure.md' README.md` ≥1；链接存在性由 W-RD-08.1 的 node 检查统一判定。
- **证据**：
  ```bash
  grep -nE '\]\(docs/' README.md | wc -l
  grep -n '](\.codeartsdoer/' README.md; echo "rc=$?"
  ```

### 7.3 贡献步骤与许可证
- [ ] 第 11 章写最小贡献路径：改代码 → `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace` → 相关门禁脚本 → 提 PR；不新增依赖除非有明确理由；不引用不存在的 `CONTRIBUTING.md`。
- [ ] 第 12 章写 `MIT OR Apache-2.0`（与 `Cargo.toml` 一致）、`deny.toml` 白名单外一律拒绝、高风险许可证（AGPL/GPL/SSPL/OSL/EUPL）不内置不捆绑、`licenses ok` 不构成法律意见。
- **依赖**：7.2。
- **DoD**：命中 `MIT OR Apache-2.0`、`不构成法律意见`、`PR`；`grep -c 'CONTRIBUTING.md' README.md` = 0；命中 5 个高风险许可证名中至少 AGPL/GPL（人工复核全串）。
- **证据**：
  ```bash
  grep -n 'MIT OR Apache-2.0' README.md
  grep -nE '不构成法律意见|白名单' README.md
  grep -n 'CONTRIBUTING.md' README.md; echo "rc=$?"
  ```

---

## 8. W-RD-08 结构与措辞全量回归（RD-AC-01~20）

> 目标：把 spec §7.2 的 20 条 AC 一次性机器化，确认 README 结构、命令、链接、措辞都达标。
> 依赖：W-RD-02 ~ W-RD-07 全部完成。

### 8.1 结构与围栏、链接与锚点
- [ ] 运行结构断言：H1 唯一、`##` 恰 12 个、无裸围栏、无本机绝对路径、正文行数 120~200。
- [ ] 运行链接存在性检查（相对链接全部指向仓库内已入库文件；`path.resolve` 存在性）。
- [ ] 运行锚点检查：读 README 内所有 `](#...)`，与标题 slug 比对（GitHub 规则）；若 README 不使用内部锚点，断言命中 0 即可。
- **依赖**：W-RD-07。
- **DoD**：`grep -c '^# ' README.md` = 1；`grep -c '^## ' README.md` = 12；`grep -c '^```$' README.md` = 0；`grep -c 'C:\\Users' README.md` = 0；`awk 'END{print NR}' README.md` 在 120..200；node 链接检查退 0；锚点检查退 0。
- **证据**：
  ```bash
  grep -c '^# ' README.md; grep -c '^## ' README.md; grep -c '^```$' README.md
  awk 'END{print NR}' README.md
  node -e '
  const fs=require("fs"),path=require("path");
  const s=fs.readFileSync("README.md","utf8");const bad=[];
  for(const m of s.matchAll(/\]\(([^)#]+)(?:#[^)]*)?\)/g)){
    const p=m[1]; if(/^(https?:|mailto:)/.test(p)) continue;
    if(!fs.existsSync(path.resolve(p))) bad.push(p);}
  if(bad.length){console.log("BROKEN:",bad.join(", "));process.exit(1)}
  console.log("links ok")' | tee "$EVID/W-RD-08-links.txt"
  grep -cE '\]\(#' README.md
  ```

### 8.2 禁用措辞与事实一致性
- [ ] 运行禁止措辞扫描（spec §6.3 全表）：`产品级可点击卡片已交付`、`设置向导可用`、`真机可用`、`G4 达标`、`CI 全绿`、`远端已通过`、`已实现全自动主动提示`、`已对接腾讯会议`、`WASAPI / UIA / OCR`、`检索源已接入`、`审计记录只追加，保留 180 天`、`45 targets`、`560 passed`、`9 个 target`、`licenses ok = 合规`、`advisories ok = 无漏洞`。
- [ ] 核对 platform/mac、LLM 变量、AC 三态、U-06~08 与 `b4-status.md` 一一对应（逐条对照 §9/§13/§15/§19/§20.3/§22）。
- **依赖**：8.1。
- **DoD**：`grep -nE` 扫描 spec §6.3 的 16 条禁止措辞正字面（命令见下方"证据"）无命中（退出码 1）；`bash scripts/check-g2-claims.sh` 退 0（README 在其扫描面内）；人工复核"产品级"仅出现在否定句。
- **证据**：
  ```bash
  grep -nE '产品级可点击卡片已交付|设置向导可用|真机可用|G4 达标|CI 全绿|远端已通过|已实现全自动主动提示|已对接腾讯会议|WASAPI / UIA / OCR|检索源已接入|审计记录只追加，保留 180 天|45 targets|560 passed|9 个 target|licenses ok = 合规|advisories ok = 无漏洞' README.md; echo "banned_rc=$?" | tee "$EVID/W-RD-08-banned.txt"
  bash scripts/check-g2-claims.sh; echo "g2_rc=$?" | tee -a "$EVID/W-RD-08-banned.txt"
  ```

### 8.3 RD-AC-01~20 逐条核对表
- [ ] 按附录 A 的映射表逐条勾验；任何一条失败即回到对应主任务修复，不得只改断言。
- **依赖**：8.2。
- **DoD**：附录 A 的 20 条全部 `pass`，并在 `$EVID/W-RD-08-ac.md` 记录每条的命令与输出摘要。
- **证据**：`cp .codeartsdoer/specs/readme_update/tasks.md "$EVID/"; # 在 W-RD-08-ac.md 中逐条填 pass/fail`

---

## 9. W-RD-09 既有门禁复跑与提交

> 目标：README 更新不得破坏任何既有门禁；提交只含文档。
> 依赖：W-RD-08 全绿。

### 9.1 既有门禁复跑（全部预期退 0）
- [ ] `bash scripts/check-b4-gates.sh --rule=doc-plan`
- [ ] `bash scripts/check-b4-gates.sh --rule=zero-transcript`
- [ ] `bash scripts/scan-secrets-docs.sh --all-md`
- [ ] `cargo fmt --all --check`
- [ ] `bash scripts/check-b3-gates.sh --rule=source`
- [ ] `bash scripts/check-g2-claims.sh`
- [ ] `bash scripts/check-mac-scope.sh --rule=na`
- [ ] `bash scripts/check-b4-gates.sh --rule=source`、`--rule=record`、`--rule=budget`、`--rule=outbound`、`--rule=targets`
- [ ] 追加（只读、不改文件）：`git diff --stat HEAD -- crates app Cargo.toml Cargo.lock deny.toml .github scripts docs/sdd` 输出为空。
- **依赖**：W-RD-08。
- **DoD**：上列命令逐条 `rc=0`（`--rule=targets` 亦为 0）；`scan-secrets-docs.sh --all-md` 输出无 `README.md:` 行；`git diff --stat` 对代码/配置/脚本为空。
- **证据**：
  ```bash
  for r in source budget record outbound zero-transcript targets doc-plan; do
    bash scripts/check-b4-gates.sh --rule=$r; echo "$r=$?"; done | tee "$EVID/W-RD-09-gates.txt"
  bash scripts/scan-secrets-docs.sh --all-md; echo "secrets=$?" >> "$EVID/W-RD-09-gates.txt"
  cargo fmt --all --check; echo "fmt=$?" >> "$EVID/W-RD-09-gates.txt"
  bash scripts/check-g2-claims.sh; echo "g2=$?" >> "$EVID/W-RD-09-gates.txt"
  git diff --stat HEAD -- crates app Cargo.toml Cargo.lock deny.toml .github scripts docs/sdd | wc -l
  ```

### 9.2 提交 A：README 单文件
- [ ] 确认 `git status --short` 仅含 `M README.md`（可加 `?? .tmp/` 之外的忽略项）。
- [ ] 以 `docs(readme): 重写根 README 至仓库实况（12 章 / AC 三态 / 本地与 CI 分栏）` 提交，仅 `git add README.md`。
- **依赖**：9.1。
- **DoD**：`git status --short` 在提交后为空；`git show --stat --oneline HEAD` 只列 `README.md`；`git diff HEAD~1 HEAD --stat` 行数 = 1。
- **证据**：
  ```bash
  git status --short; git add README.md
  git commit -m 'docs(readme): 重写根 README 至仓库实况（12 章 / AC 三态 / 本地与 CI 分栏）'
  git show --stat --oneline HEAD
  ```

### 9.3 提交 B（可选）：`b4-status.md` 追加 §24 记录
- [ ] 仅当维护者要求留痕时执行：在 `docs/progress/b4-status.md` **文件末尾追加** `## §24 README 更新记录`，内容为：更新日期、README 行数、9.1 执行的命令与退出码、未裁定项（C-RD-01 / D-RD-01）。
- [ ] 不改 §1~§22（若 §23 已存在，同样不改）；不复制 README 正文；不新增/改写 AC 编号。
- **依赖**：9.2。
- **DoD**：`grep -c '^## §24' docs/progress/b4-status.md` = 1；`git diff HEAD -- docs/progress/b4-status.md` 只有新增行（`--numstat` 删除列 = 0）。
- **证据**：
  ```bash
  git diff --numstat HEAD -- docs/progress/b4-status.md
  grep -nE '^## §2[2-4]' docs/progress/b4-status.md
  git add docs/progress/b4-status.md
  git commit -m 'docs(progress): 登记 README 更新取证（§24）'
  ```

---

## 附录 A · RD-AC-01~20 → 任务映射

| AC | 判定要点 | 归属任务 |
|---|---|---|
| RD-AC-01 | 唯一 H1、UTF-8 非空 | W-RD-02.1、W-RD-08.1 |
| RD-AC-02 | 12 个二级章节且顺序正确 | W-RD-02~07、W-RD-08.1 |
| RD-AC-03 | 相对链接全部存在、不链 `.codeartsdoer/` | W-RD-03.4、W-RD-07.2、W-RD-08.1 |
| RD-AC-04 | 围栏均带语言、无本机绝对路径 | W-RD-04.1、W-RD-08.1 |
| RD-AC-05 | PATH/TEMP + Rust 4 条 + 前端 5 条 + 桌面壳 | W-RD-03.1、W-RD-03.2 |
| RD-AC-06 | 无过期硬编码数字 | W-RD-01.3、W-RD-07.1、W-RD-08.2 |
| RD-AC-07 | `npm run dev` 仅前端、桌面壳用 cargo | W-RD-03.2 |
| RD-AC-08 | Windows 主 / macOS 不适用 / CI 仅观察 | W-RD-02.3 |
| RD-AC-09 | 9 ignored 与真实端点未验证 | W-RD-04.2、W-RD-07.1 |
| RD-AC-10 | 平台层写空壳、无 WASAPI/UIA/OCR 完成态 | W-RD-03.3 |
| RD-AC-11 | 本地门禁与 CI 分栏 | W-RD-05.1、W-RD-05.3 |
| RD-AC-12 | cargo-deny 同版同命令、无 `--offline` 唯一命令 | W-RD-05.4 |
| RD-AC-13 | LLM 默认关闭 + 4 变量 + 设置向导仅契约 | W-RD-06.1、W-RD-06.2 |
| RD-AC-14 | 38/39/51 边界 + 不宣称产品级 | W-RD-06.3 |
| RD-AC-15 | 无真实凭据/端点/模型名/request_id | W-RD-06.1、W-RD-08.2、W-RD-09.1 |
| RD-AC-16 | 39 只做对齐、不联网抓正文、唯一投影 | W-RD-06.3 |
| RD-AC-17 | 审计 180 天为设计约束、当前内存实现 | W-RD-06.4 |
| RD-AC-18 | 检索源仅端口契约 | W-RD-06.4 |
| RD-AC-19 | AC 三态与 `b4-status.md` 同口径 | W-RD-07.1 |
| RD-AC-20 | `MIT OR Apache-2.0` + 不构成法律意见 | W-RD-07.3 |

## 附录 B · 依赖顺序与提交顺序（一览）

```text
W-RD-01（事实基线）
  ├─ W-RD-02（第 1 章）
  ├─ W-RD-03（第 2~3 章）
  │    └─ W-RD-04（第 4 章）
  ├─ W-RD-05（第 5 章）
  ├─ W-RD-06（第 6~8 章）
  └─ W-RD-07（第 9~12 章）
          └─ W-RD-08（结构/措辞全量回归）
                  └─ W-RD-09（门禁复跑 → 提交 A → 可选提交 B）
```

- 提交 A：仅 `README.md`。
- 提交 B（可选）：仅 `docs/progress/b4-status.md` 的 `## §24` 追加节。
- 不 push；不 amend；不 `git add -A`。