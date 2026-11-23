# b4_gate · 编码任务清单（P0~P11）

> 阶段：第三阶段（编码任务规划）。本文件把 `.codeartsdoer/specs/b4_gate/spec.md`（866 行，AC-B4-01~50）、`.codeartsdoer/specs/b4_gate/requirements.md`（955 行，REQ-B4P0-01~24）与 `.codeartsdoer/specs/b4_gate/design.md`（2706 行，含 §3 P0 剩余落地设计）翻译成可执行、可机器验收的任务；**不修改任何代码，不修改 `docs/sdd/**`**。
> 唯一需求基线：spec 的 AC + requirements §5~§9；与 design 冲突时以 AC 为准，处置记录在各子任务"追溯"行与 §0.3 裁决表。
> 代码基线（2026-11 实测）：HEAD `ade9111`；`git status --short` 为空；`crates/domain/src/model/gate.rs` 241 行（仅 `Visibility` + `GateRejectReason` 23 项 + 7 单测）；`crates/application/src/gate/`、`gate_replay.rs`、`scripts/check-b4-gates.sh`、`docs/progress/b4-status.md` 均不存在；`cargo test --workspace -- --list` 593 用例 / **584 passed / 9 ignored**。
> **2026-11-22 实测更正**：上述"不存在"的文件与目录均已交付（P0~P3 已完成：`gate/**` 13 个文件、`gate_replay.rs` 12 项、`check-b4-gates.sh` 7 规则、`b4-status.md` §1~§10）；AC-B4-01~47、49、50 为 `passed`，AC-B4-45 为 `not-applicable`（macOS 无设备），**AC-B4-48 与 51~57 为 `pending`**（由 P4 承接）。`gate/api.rs` 现 89 行、`^pub fn ` = 4。
> **本阶段范围**：只执行 P0 剩余 7 个主任务（W-B4-2 ~ W-B4-8）；W-B4-1 已在 `ade9111` 交付，不重做。P1~P3 章节保留为后续阶段口径，其中 5 处已按 P0 交付物加注（W-B4-9.5、10.1、10.3、28.2、31.1、31.2）。
> 本轮**只做 B4**：不碰 B5；不实现 13.3 本体；不实现 38/39/41.5/42/43/51 UI/52 存储的实体能力。
>
> **2026-11-22 增量修订（P4）**：需求与设计新增"可点击关键词卡片契约"（AC-B4-51~57）与 AC-B4-48 方案 A 口径。本文件追加 **P4 · W-B4-34 ~ W-B4-42**（9 个主任务、46 个子任务），把 `design.md` §四（4.0~4.10）、`requirements.md` §10 与 `spec.md` §5.6/§6.11~6.13/§7.11 翻译成可执行任务。既有 P0~P3 的任务编号、语义与验收口径**全部保留**，仅在两处作加注式更新：`NEW_TARGETS` 长度 19→20（X-02）、`gate_replay.rs` 的 AC-48 用例名按 spec §7.7 逐字（W-B4-41）。
> **P4 不动的三条硬边界**：① `crates/application/src/gate/api.rs` 仍恰 4 个 `pub fn` 且**零 diff**；② `GateRecord` 14 字段 / `GateAuditEvent` 7 / `GateInputFixture` 23 / `ReplayReport` 5 字段全部不变；③ 不建 `crates/application/tests/replay/`，不实现 38/39/51/13.3 实体文件。
>
> **2026-11-22 增量修订（P5）**：需求与设计新增"39 引用对齐校验"（AC-B4-58~64，7 条）。本文件追加 **P5 · W-B4-43 ~ W-B4-48**（6 个主任务、27 个子任务、1 个新 target、12 个用例），把 `design.md` §五（5.0~5.11，含 §5.10.4 裁决 X-03~X-06）、`requirements.md` §11、`spec.md` §5.7 / §6.14~6.15 / §7.9-5 / §7.12 翻译成可执行任务。既有 P0~P4 的任务编号、语义与验收口径**全部保留**，仅在 §0 加注式更新：阶段范围（P0~P5）、主任务上限 42→48、`NEW_TARGETS` 20→21、`--rule=targets` 循环**仍 7 项不改**（X-05）。
> **P5 不动的五条硬边界**：① `crates/application/src/gate/api.rs` 仍恰 4 个 `pub fn` 且**零 diff**；② `GateRecord` 14 / `GateAuditEvent` 7 / `GateInputFixture` 23 / `ReplayReport` 5 / `GateCounter` 32 / `GateRejectReason` 23 全部不变；③ 不改 `gate_keyword.rs`（`CitationAlignment` 仍 3 变体，`UNVERIFIED` 以 `CONTRACT_UNVERIFIED` 语义别名实现，X-04 方案 A）；④ 不建 `crates/application/tests/replay/`，不建 38 合成（`src/cards/`）、51 UI（`ui-bridge` 卡片面）、52 存储；⑤ `scripts/check-b4-gates.sh` 规则**仍 7 条**、退出码语义不变，`--rule=targets` 循环**不追加** `citation_align`（X-05）。
>
> **2026-11-22 增量修订（P6）**：需求、设计与 39 增量之后新增"38 卡片合成"（AC-B4-65~74，10 条）。本文件追加 **P6 · W-B4-49 ~ W-B4-56**（8 个主任务、31 个子任务、1 个新 target、14 个用例），把 `design.md` 第六章（§6.0~§6.13，含 §6.11.3 裁决 X-07~X-09）、`requirements.md` §12、`spec.md` §5.8 / §6.16~6.18 / §7.13 / §9.1 J-21~J-26 翻译成可执行任务。既有 P0~P5 的任务编号、语义与验收口径**全部保留**，仅在 §0 加注式更新：阶段范围（P0~P6）、主任务上限 48→56、`NEW_TARGETS` 21→22、`--rule=targets` 循环**仍 7 项不改**（沿用 X-05）。
> **P6 不动的六条硬边界**：① `crates/application/src/gate/api.rs` 仍恰 4 个 `pub fn` 且**零 diff**；`gate/**` 零 diff（38 不 `use crate::gate`）；② `crates/domain/src/model/**` **零 diff**（38 的 4 个契约类型落 application，不新增 domain 契约，X-07）；③ `gate_keyword.rs`/`gate.rs`/`gate_replay.rs`/`enums.rs` 零 diff；`GateRecord` 14 / `GateAuditEvent` 7 / `GateInputFixture` 23 / `ReplayReport` 5 / `GateCounter` 32 / `GateRejectReason` 23 / `KeywordCardDraft` 13 / `ReferenceView` 5 / `CitationAlignment` 3 全部不变；④ 不建 51 UI（`ui-bridge` 卡片面）、52 存储、`crates/application/tests/replay/`；⑤ `scripts/check-b4-gates.sh` 规则**仍 7 条**、退出码语义（0/1/2）不变，`--rule=targets` 循环**不追加** `card_compose`（X-05）；⑥ 不新增 crate、不改 `Cargo.toml`/`Cargo.lock`，不渲染/不存储/不联网/不读原文/不判定点击/不调 LLM。
>
> **2026-11-22 增量修订（P7 + P8 同批规划）**：需求与设计新增"51 点击 UI"（AC-B4-75~84，10 条）与"51 设置向导"（AC-B4-85~94，10 条）。本文件追加 **P7 · W-B4-57 ~ W-B4-64**（8 个主任务、29 个子任务、1 个新 target `keyword_card_view`、19 个用例）与 **P8 · W-B4-65 ~ W-B4-72**（8 个主任务、30 个子任务、1 个新 target `llm_settings_view`、18 个用例），把 `design.md` §四（4.2.2/4.3/4.5.2/4.7/4.9）、§七（7.0~7.13，含 §7.11.2 D-B4-P85~P101 与 §7.11.4 X-10/X-11）、`requirements.md` §13~§14、`spec.md` §5.9~§5.10 / §6.19~6.28 / §7.14~§7.15 / §9.1 J-27~J-34、`docs/progress/b4-status.md` §16~§17 翻译成可执行任务。既有 P0~P6 的任务编号、语义与验收口径**全部保留**，仅在 §0 加注式更新：阶段范围（P0~P8）、主任务上限 56→**72**、`NEW_TARGETS` 22→**24**、`--rule=targets` 循环**仍 7 项不改**（沿用 X-05）。
> **P7/P8 已冻结的六条前提（用户已确认，不再权衡）**：① **X-11 顺序**：P7 先落（`keyword_card_view`，`NEW_TARGETS` 22→23），P8 后落（`llm_settings_view`，23→24）；② **X-10 降级口径**：安全存储降级到内存时用 `PERSIST_FALLBACK_NOTICE` 常量 + `persist_intent(Memory)`，**不新增第 19 类型、不扩 8 项视图**；③ P7 只做视图模型 + 点击交互契约，点击本地只读（0 外发/0 LLM/0 审计），只降不升，投屏/紧急遮蔽隐藏正文，`Withdrawn` 不出视图；④ P8 只做"意图 + 纯函数"，Key 固定掩码、`Debug` 零回显、不落盘明文、清除二次确认、8 类人话失败、不新增依赖、不直连网络、不出现 `LLM_`/`std::env`/`reqwest`/`TcpStream`、模型名/域名全注入、测试只进 `--rule=zero-transcript`；⑤ 落位：`crates/ui-bridge/src/keyword_card/`（`mod.rs` + `view.rs`）与 `crates/ui-bridge/src/settings/`（4 文件），`ui-bridge/src/lib.rs` **共 +2 行**，两模块互不耦合，不建 52 存储；⑥ 交付面：两个新 target，脚本扫描面一律用**拼接写法**，使脚本文件内不出现完整 target 名。
> **P7/P8 开工前置（硬）**：`git log -1 --format=%s` 必须已含 P6（`W-B4-49~56` 对应的提交）；`git status --short` 必须为空。当前实况 `git status` 显示 P6 变更（`crates/application/src/cards/` 未跟踪 + 3 个脚本/文档已改）**尚未提交**，因此**P7 的 W-B4-57.0 是一次提交闸门**：先单独提交 P6，再进 57.1；否则 P7/P8 的 `git diff --quiet HEAD` 类零 diff 判据基线不清。
> **P7/P8 不动的八条硬边界**：① `crates/application/src/gate/api.rs` 仍恰 4 个 `pub fn` 且零 diff；② `crates/application/src/gate/**` 与 `crates/application/src/cards/**`（**源码**）零 diff——P7 唯一例外是 `crates/application/tests/card_compose.rs` 的 1 行路径断言（X-12，见 W-B4-57.1）；③ `crates/domain/src/model/**` 零 diff；④ `crates/infrastructure/src/llm/**` 零 diff（白名单/路径常量/超时口径只读）；⑤ `crates/ui-bridge/src/dto.rs` 与 `crates/ui-bridge/tests/schema.rs` 零 diff；⑥ 13 项封闭面（`KeywordCardDraft` 13 / `ReferenceView` 5 / `CitationAlignment` 3 / `ComposedCard` 11 / `CardComposeInput` 7 / `CardComposeOutcome` 2 / `ComposeRejectReason` 6 / `GateRecord` 14 / 审计 7 / 夹具 23 / `ReplayReport` 5 / `GateCounter` 32 / `GateRejectReason` 23）与 `api.rs` 4 不变；⑦ 不建 52 存储、不建 `crates/application/tests/replay/`、不建平台适配/窗口句柄/焦点 API/密钥链句柄；⑧ 不新增外部 crate（P7 仅允许 `crates/ui-bridge/Cargo.toml` 新增 2 条 **path** 依赖 `meeting-copilot-application`/`meeting-copilot-domain`，`Cargo.lock` 仅因 path 依赖关系变化；P8 对 `Cargo.toml`/`Cargo.lock` **零 diff**）。

> **2026-11-22 增量修订（P9）**：需求与设计新增"13.3 回放本体"（AC-B4-95~104，10 条）。本文件追加 **P9 · W-B4-73 ~ W-B4-81**（9 个主任务、30 个子任务、1 个新 target `gate_replay_driver`、22 个测试函数 + 24 例全绿集 + 5 例探针集），把 `design.md` 第八章（§8.0~§8.11，含 §8.9.2 D-B4-P118~P130）、`requirements.md` §15、`spec.md` §5.11 / §6.29 / §7.16 / §9.1 J-35~J-44 / D-B4-10、`docs/progress/b4-status.md` §20（D-B4-P108~P117）翻译成可执行任务。既有 P0~P8 的任务编号、语义与验收口径**全部保留**，仅在 §0 加注式更新：阶段范围（P0~P9）、主任务上限 72→**81**、`NEW_TARGETS` 24→**25**、`--rule=targets` 循环**仍 7 项不改**（沿用 X-05）。
> **P9 已冻结的五条口径（用户已确认，不再权衡）**：① `tests/replay/` **不解禁**，用例落 `crates/application/tests/gate_replay_driver.rs`，target `gate_replay_driver`；② 与既有 `tests/gate_replay.rs` 的 200 例**并存**（零 diff、6 用例全绿、200/0/7 不变），13.3 **不复用/不迁移/不修改**其构造器，自检 ≥20 例（22 测试函数 + 24 回放用例双口径）；③ `NEW_TARGETS` 24→25，`--rule=targets` 循环**仍 7 项**、`scripts/check-b4-gates.sh` **零改动**（`gate/*.rs` 与 `tests/gate_*.rs` glob 已覆盖）；④ 不改 `gate/**` 既有语义：`gate/replay.rs` **唯一改写** + `gate/mod.rs` **+1 行** + `api.rs` 4 函数；封闭面 `GateRecord` 14 / 审计 7 / `GateRejectReason` 23 / `GateInputFixture` 23 / `ReplayReport` 5 / `GateCounter` 32 / `ComposedCard` 11 / `KeywordCardView` 10 / `SettingsView` 8 不变；⑤ 只调 4 个公开纯函数与 `FixedClock`；零转录；失败报告只含 `case_id`/原因码；两次/乱序/100 次逐项一致。
> **P9 不动的七条硬边界**：① `crates/application/src/gate/api.rs` 仍恰 4 个 `pub fn` 且**零 diff**；② `gate/**` 其余 12 个文件（`prefilter`/`relevance`/`dedupe`/`budget`/`lease`/`outbound`/`record`/`diagnostics`/`input`/`clock`/`keyword`）**零 diff**；③ `crates/domain/src/model/**`（含 `gate.rs`/`gate_replay.rs`）**零 diff**，`GateInputFixture` 仍 23；④ `crates/application/tests/gate_replay.rs`、`crates/application/src/cards/**`、`crates/ui-bridge/**`、`crates/infrastructure/**` **零 diff**；⑤ `scripts/check-b4-gates.sh` **零 diff**、规则仍 7 条、`--rule=targets` 循环不追加；⑥ 根 `Cargo.toml`/`Cargo.lock`/`crates/application/Cargo.toml` **零 diff**（不新增 `serde_json`/`rand`）；⑦ 不建 `crates/application/tests/replay/`、不建 JSON/CSV 用例格式、不建分组统计报表类型、不新增 `GateCounter`/`GateRejectReason` 变体、不建第 6 个失败码。
> **P9 开工前置（硬）**：`git log -1 --format=%s` 必须已含 P8（`feat(settings)`）**与 CI deny 修复**；`git status --short` 必须为空。**P9 零 diff 基线 = 当前 HEAD `f153d7f`**（`docs(b4): 登记 CI deny 修复取证（§22）`；该提交已改动 `Cargo.lock` 的 `url`/`form_urlencoded`，故基线**不是** P8 的 `4ae54a9`）。判据语义不变（仍是"相对基线 HEAD 零 diff"），仅指针前移；P9 内 `Cargo.lock` 仍须相对 `f153d7f` 零 diff。当前实况仅 `docs/progress/b4-status.md`（§20 增量登记 + §20.6 补正）未提交，因此 **W-B4-73.0 是一次提交闸门**：先单独提交该文档，再进 73.1；否则 P9 的 `git diff --quiet HEAD` 类零 diff 判据基线不清（§20 本身不在零 diff 断言内，但工作区脏会让 `--rule` 与 `git diff` 判据失去意义）。

> **2026-11-22 增量修订（P10 · 收尾批次）**：P9 已于 `1b0dbf7`（`feat(gate): 落地 13.3 回放本体与 22 例自检集`）提交并验收（实测：`gate_replay_driver` 22 passed、`cargo test --workspace` 59 个 test result / **825 passed / 0 failed / 9 ignored**、`--list` 834、AC-B4-95~104 全部 `passed`、U-06 `closed`、`NEW_TARGETS`=25、七规则 7×0）。本文件追加 **P10 · W-B4-82**（1 个主任务、5 个子任务）：只做**文档实况收口**（`README.md` 现值口径 + `docs/progress/b4-status.md` 追加 §25 时点补正），**不含任何 Rust 功能实现**；同时登记 U-07/U-08 与 AC-B4-109 远端取证的"**需用户提供条件**"清单（见 §0.10.3）。既有 P0~P9 的任务编号、语义与验收口径**全部保留**，仅在 §0 加注式更新：阶段范围（P0~P10）、主任务上限 81→**82**、`NEW_TARGETS` **仍 25**（P10 不新增 target）、AC 汇总 **仍 104**（P10 不新增 AC）。
> **P10 零 diff 基线 = `1b0dbf7`**（P9 代码提交）。P9 章节内的 `git diff --quiet HEAD -- …` 类判据在 P9 提交后已不可复跑，复跑口径见 §0.10.2（改用 `1b0dbf7^` 作为 P9 的比较基线）。
> **P10 不动的边界**：只改 `README.md` 与 `docs/progress/b4-status.md`（追加式）；`crates/**`、`scripts/**`、`Cargo.toml`/`Cargo.lock`、`deny.toml`、`.codeartsdoer/specs/b4_gate/{spec,requirements}.md` **零 diff**；不新增 target、不改 `NEW_TARGETS`、不新增 `#[ignore]`、不改任何 AC 编号或 `check-b4-gates.sh` 规则。

> **2026-11-22 增量修订（P11 · clippy 质量收口）**：CI `rust` matrix 的 `cargo clippy --workspace --all-targets -- -D warnings` 实测红，`feature-coverage (C2)` 因 `needs: rust` 无 coverage artifact 而级联失败（**非独立缺陷**，rust 转绿即自愈，`ci.yml` 不改）。本文件追加 **P11 · W-B4-83**（1 个主任务、6 个子任务，**无新 AC / 无新 target**）：只收敛 lint，不含任何功能实现。既有 P0~P10 的任务编号、语义与验收口径**全部保留**，仅在 §0 加注式更新：阶段范围（P0~P**11**）、主任务上限 82→**83**、`NEW_TARGETS` **仍 25**、AC 汇总 **仍 104**。
> **P11 告警面（cargo 1.100.0 实测）**：`gate/replay.rs:81,82` 2 处 `clippy::map_or_identity`；临时 allow 后 `tests/gate_replay_driver.rs` 再暴露 **22 处** test-target 告警（`dead_code` 1 / `field_reassign_with_default` 3 / `default_constructed_unit_structs` 17 / `unnecessary_min_or_max` 1）。合计 **24 处**。
> **P11 零 diff 基线 = `52c4bd3`**（`docs(status): 校准 README 实况并登记 P10 收口（§25）`，P10 单提交、工作区已清）。P11 提交面**恰 3 文件**（`gate/replay.rs`、`tests/gate_replay_driver.rs`、`b4-status.md`）；`design.md` 的 §8.9.6 与 X-34/X-35 登记在**规划侧**完成，不进入代码提交。
> **P11 不动的边界**：`.github/workflows/ci.yml`、`Cargo.toml`/`Cargo.lock`、`crates/*/Cargo.toml`、`deny.toml`、`scripts/**`、`crates/domain/**`、`crates/application/src/cards/**`、`crates/ui-bridge/**`、`crates/infrastructure/**`、`crates/application/tests/gate_replay.rs` 与其余 11 个 `gate/*.rs` **零 diff**；不以 `#[allow]`/`#[expect]` 规避；不新增 target/AC/`#[ignore]`；不改用例名与断言数值；不改 `check-b4-gates.sh` 规则（仍 7 条）。

## 0. 执行口径

### 0.1 环境前置（每个 shell 会话先执行一次）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
```

- `test-name-inventory.sh` 支持 `CARGO` 覆盖，直接沿用上面的 `$CARGO`。
- `cargo fmt` 需要 `rustfmt` 可见：PATH 前置已包含；若仍失败，显式 `export RUSTFMT="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/rustfmt.exe"`。
- 平台口径：Windows 单平台；macOS 一律"不适用（无设备）"，不新增任何 mac 依赖/路径/分支。
- **PATH 必须前置才能跑 `--diff`（X-31 实测）**：`bash scripts/test-name-inventory.sh --diff …` 内部会调用 `cargo test --no-run` 探面；若未把工具链目录前置到 `PATH`，`ui-bridge` 编译会以 `STATUS_DLL_NOT_FOUND (0xc0000135)` 退 101，脚本报 `face=… rc=101 failed=4`。**前置 PATH 后同一命令退 0（`PASS: removed=0`）**。故所有 `--diff` 判据必须先执行上面的三行 `export`。

### 0.2 编号与书写格式

- 阶段：`P0`、`P1`、`P2`、`P3`、`P4`、`P5`、`P6`、`P7`、`P8`、`P9`、`P10`（P4~P6 为 B4 契约增量；**P7 = 51 点击 UI**、**P8 = 51 设置向导**、**P9 = 13.3 回放本体**、**P10 = 文档实况收口**，2026-11-22 规划、**串行执行**：P7 → P8 → P9 → P10）。主任务：`W-B4-<n>`（全局递增 1..82，不复用 `AC-B4-*`）。子任务：`W-B4-<n>.<m>`。
- 每个子任务**至少三行**：**追溯**（REQ/AC + design 章节号）、**DoD**（可机器判定：退出码/用例名/计数/产物/grep）、**证据**（可直接粘贴执行的命令或断言）；必要时补 **改动内容**、**依赖/顺序**、**禁止**、**证据产物**。
- 证据命令一律在项目根 `TSC/` 下执行；`$CARGO` 已 export。
- **证据产物统一落点**：每条命令的退出码、日期与结论写入 `docs/progress/b4-status.md` 对应段（§2 三条硬判据、§3 旁证/规则矩阵、§4 偏差、§5 未判定、§6 复跑方法），**不新增**临时产物文件；临时文件只允许 `mktemp -d`（`trap` 清理）或仓库已忽略的 `.tmp/`，且阶段结束必须清空。

### 0.3 已裁决约束（直接采用，不再权衡）

| 编号 | 裁决 | 落地任务 |
|---|---|---|
| 输入 | 门控**只读** `Insight`；`InsightDraft` 禁入；`store.rs`/`deduplicator.rs` **零 diff** | W-B4-9.3、W-B4-29 |
| 顺序 | 固定 G1→G2→G3→G4→G5；强制级拒绝即短路；仅 G2 与 G3-dedupe 可关 | W-B4-13.3、W-B4-18.6 |
| 预算 | 门控独立预算（120/10min 全局 3/单主题 2/实体 5min/并发 3）；**禁止读 `LLM_LIVE_CALL_CAP`** | W-B4-19.1、W-B4-21 |
| 授权 | G5 授权 7 字段；未授权禁止；撤销不可自恢复；审计不可用或脱敏失败 fail-closed | W-B4-24、W-B4-26 |
| 回放 | 只暴露 4 个纯函数 + 3 个结构；不建 `tests/replay/` | W-B4-6、W-B4-28 |
| C-01 | `GateRejectReason` 23 项唯一对外；12 项映射 11 项 `DenyCode`；11 项内部码；UI `gate.reason.*` | W-B4-1 |
| C-02 | 全输入闭包（`GateInput` 携带 budget/authorization/now_ms）+ `FixedClock` | W-B4-9、W-B4-10、W-B4-28 |
| C-03 | `OutboundLease` 3 槽 + 单排队位 + `LeaseId`，无锁/无 await/无定时器 | W-B4-21 |
| 依赖 | 零新依赖；`domain` 已有 `serde_json` 可直接用于 `unknown` 透传 | W-B4-8.4、W-B4-30.4 |
| 范围 | 不新增 `#[ignore]`；9 ignored 名称不变；macOS 不适用 | W-B4-30.6、W-B4-30.7 |

**本阶段新增/修正裁决（design §3.10，执行时直接采用）**

| 编号 | 裁决 | 落地 |
|---|---|---|
| `D-B4-D06` | `budget_remaining` 上限由 requirements §6.6-10 的 120 修正为 **600**（与 `session_outbound_cap ∈ 0..=600` 自洽；默认 cap=120 时行为不变） | 6.4、6.6 |
| `D-B4-D07` | `--rule=targets` 在 P0 **退 1** 且 6 项可枚举，推翻 requirements §5.7.1-2 对 `targets` 的"必须退 0"；P1 起逐项转绿、P3 退 0 | 7.6、8.3 |
| `D-B4-D05` | `GateInput`/`GateSnapshot` 各增 `candidate_ref`、`insight_ref`、`trace_id` 三字段（否则 `record_from` 无法产出合法 14 字段） | 6.1、6.4 |
| `D-B4-D08` | `EntityKey::parse` 返回 `Option<Self>`；`GateRecord` 结构违规用谓词方法，不新增 domain 错误枚举（31 项不变） | 5.1、4.5、6.6 |
| `D-B4-D09` | 7 个 ≤5 行骨架文件在 6.3 一次性创建（`mod x;` 要求文件存在），P1~P3 只填内容 | 6.3 |
| `D-B4-D10` | P0 的 `record.rs` 用内部 `RecordViolation{RefsEmpty, IllegalSkipStage}`，不落 `GateError`（P3 再升格，签名不变） | 6.6 |
| `D-B4-D11` | `GateCounter` 32 变体逐项显式 `#[serde(rename=...)]`，不依赖 `rename_all` 的字母-数字边界 | 5.3 |

**P4 增量裁决（design §4.8，执行时直接采用，不再权衡）**

| 编号 | 裁决 | 落地 |
|---|---|---|
| `D-B4-P19` | 关键词契约落**新文件** `crates/domain/src/model/gate_keyword.rs`；`gate.rs`（31 项）与 `gate_replay.rs`（12 项）零改 | W-B4-34.1~34.8 |
| `D-B4-P20` | 新增 7 个 B4 专属封闭枚举（`KeywordKind`/`KeywordCardStatus`/`ReferenceTrustTier`/`ReferenceAccessStatus`/`CitationAlignment`/`ClickState`/`ClickDisabledReason`），对既有 `enums.rs` 做**单向投影**，不改 `enums.rs` | W-B4-34.2 |
| `D-B4-P21` | 复用既有 `TitleText`(40)/`ConclusionText`(80)/`Confidence`/`DedupeKey`/`TextRange`/`CardRef`/`ReferenceRef`/`GapState`；新增 `KeywordRef`（`id_type!`）与 `NormalizedTerm`（`bounded_text!` 120） | W-B4-34.1、34.4~34.6 |
| `D-B4-P22` | `click_state` 六因**固定优先级**：`StaleReference → AuthorizationRevoked → NotAllowed → EmergencyMasked → ScreenShareProtection → ReferenceUnverified`，输出恒为唯一原因 | W-B4-37.2 |
| `D-B4-P23` | 候选派生输出 `new_candidates` + `reobservations`；首见键 = `(session_id, normalize_text(term))`；G1 原串首见不改（AC-B4-02 零回归） | W-B4-35.1、36.3 |
| `D-B4-P24` | **不扩** `GateInput`/`GateRecord`/`GateAuditEvent`/`ReplayReport`/`GateInputFixture`/`GateCounters`；`record_from.skipped_stages` 的既有缺口本轮不修 | W-B4-34.8、41.1、42.4 |
| `D-B4-P25` | 38/39/51 端口只在 design §4.5.2 预留签名，**不在 `gate/**` 建 trait/文件** | W-B4-40.4（只做 0 命中断言） |
| `D-B4-P26` | 新增第 7 个 target `gate_keyword`（`NEW_TARGETS` 19→20）；`check-b4-gates.sh` 新增 `GATE_KEYWORD` 并纳入 `source`/`budget`/`zero-transcript` 扫描面，**规则仍 7 条** | W-B4-42.1~42.3 |
| `D-B4-P27`【P4 新增】 | `AllowTraceRef` 计入 `gate_keyword.rs`，成为该文件第 **4** 个结构（design §4.8.2 记的"3 结构 + 7 枚举"修正为"4 结构 + 7 枚举" = **11 项**）；理由：`gate_decision_ref` 必须"只能由 `Allow` 构造"（I-06），`Option<TraceId>` 无法阻止 `Deny` 路径伪造。`gate.rs` 31 项不变 | W-B4-34.3 |
| `D-B4-P28`【P4 新增】 | 跳过级可判定投影：`policy.relevance_enabled=false → per_stage.0[1]=Skipped`（且不参与 `LowRelevance`）；`policy.dedupe_enabled=false → per_stage.0[2]=Skipped`。只改 `diagnostics.rs` 两处赋值，不扩任何结构字段；默认 `true` 路径行为逐字节不变 | W-B4-35.3、35.4 |
| `D-B4-P29`【P4 新增】 | AC-B4-48 的 runner 写在 `crates/application/tests/gate_replay.rs` 内（`fn run(cases: &[GateReplayCase]) -> ReplayReport`）；`gate/replay.rs` 保持现有 1 行注释**零 diff**，不新增 src 符号、不扩回放面 | W-B4-41.3~41.5 |
| `D-B4-P30`【P4 新增】 | `scan_candidates_with_rule` 返回 `Vec<ScannedCandidate>`（`pub(crate) struct ScannedCandidate { rule: GapRule, term: String, candidate: GapCandidate }`），**不是** design §4.1.2 记的 `(GapRule, GapCandidate)`；理由：`KeywordCandidate` 必须携带 `keyword_text`/`normalized_term`，而 `GapCandidate` 不含词，二次调用 `detect_terms` 会与首见判定产生双解析漂移 | W-B4-35.1、36.2 |
| `D-B4-P31`【P4 新增】 | `card_drafts` **只**由 `GateDecision::Allow` 决定（Deny→0 / Allow→1）；`click_state` 与 `body_visible` 独立由 7 项合取与屏蔽态决定。否则 AC-B4-52（Allow→1 卡）与 AC-B4-55（未对齐/投屏→卡仍展示但不可点）不可同时满足 | W-B4-37.1 |
| `D-B4-P32`【P4 新增】 | `KeywordCardAdmission` 含 `body_visible: bool`（投屏保护或紧急遮蔽时 `false`），承载 spec §5.6.8"卡片正文隐藏"的**契约要求**；实际渲染仍由 51 执行，B4 不写 UI | W-B4-37.1、40.2 |
| `X-01`【已接受】 | 推翻"`core_files` 只含三处"：新增 `GATE_KEYWORD="crates/domain/src/model/gate_keyword.rs"` 并加入 `core_files()` 与 `zero-transcript` 的 `files` 数组 | W-B4-42.1 |
| `X-02`【已接受】 | 推翻"仅 6 个 target 的隐含上限"：新增 `crates/application/tests/gate_keyword.rs`，`NEW_TARGETS` 19→20；`--rule=targets` 仍只断言 6 项存在 | W-B4-42.2 |

**P5 增量裁决（design §5.10，执行时直接采用，不再权衡）**

| 编号 | 裁决 | 落地 |
|---|---|---|
| `D-B4-P38` | **实况别名**：规格 `Unverified` ≡ 代码 `CitationAlignment::Unknown`；`Authority` ≡ `ReferenceTrustTier::AuthoritativeMedia`；`Ok` ≡ `ReferenceAccessStatus::Available`。39 侧以 `CONTRACT_UNVERIFIED` / `is_authority_tier()` / `is_accessible()` 封装，**不改既有枚举** | W-B4-43.5 |
| `D-B4-P39` | 39 输入复用 `Vec<ReferenceView>`（既有 5 字段），**不新增引用类型** | W-B4-43.3 |
| `D-B4-P40` | `Failed` 三因的判定信号：`ReferenceView.alignment` 是**唯一上游信号**；单条 `Failed → Contradicted`、`Passed` 与 `Failed` 并存 → `ConflictingSources`；`TermMismatch` 只用 §5.3.3 结构规则；全无信号 → `InsufficientEvidence` | W-B4-44.3 |
| `D-B4-P41` | `CitationCheckInput` **手写 `Debug`**，`normalized_term` 永不输出、不实现 `Serialize`；`ClaimRef` 构造期拒绝非法形态 | W-B4-43.2、43.3 |
| `D-B4-P42` | 在 `citation.rs` 内补 `impl CitationAlignment { pub const ALL }`；`CITATION_CONTRACT_VERSION: u16 = 1`；**不新增** `GateCounter`，诊断用局部 `CitationDiagnostics` | W-B4-43.4/43.5、44.4 |
| `D-B4-P43` | 39 独立模块 `application/src/citation/`（`mod.rs` + `align.rs`），`lib.rs` +1 行；不进 `api.rs`、不建 trait/端口 | W-B4-44.1、45.1 |
| `X-03`【已冻结】 | 脚本 `require_core_faces()` 纳入 39 面后，文件缺失会使 7 条规则退 **2**；故脚本改动与 39 文件**同批提交**，顺序为"先建文件、后改脚本" | W-B4-47 前置、P5 公共断言 |
| `X-04`【已冻结·方案 A】 | 不改 `CitationAlignment`，`CONTRACT_UNVERIFIED = CitationAlignment::Unknown` 作语义别名；`CitationAlignment::ALL.len()` 仍 3 | W-B4-43.5、46.3 |
| `X-05`【已冻结】 | `--rule=targets` 循环**不加** `citation_align`；由 `NEW_TARGETS` token（20→21）与 target 实跑断言 | W-B4-47.3/47.4 |
| `X-06`【已接受】 | `ReferenceView.alignment` 作 `Failed` 三因的唯一上游信号；`Passed` 仅表示"上游信号无冲突且存在高档可访问引用"，不宣称真实语义一致（D-B4-05） | W-B4-44.3、46.4 |

**P6 增量裁决（design §6.11，执行时直接采用，不再权衡）**

| 编号 | 裁决 | 落地 |
|---|---|---|
| `D-B4-P53` | 38 的 4 个契约类型（`CardComposeInput`/`ComposedCard`/`CardComposeOutcome`/`ComposeRejectReason`）落 `application/src/cards/compose.rs`；**不新增 domain 契约**，`crates/domain/src/model/**` 零 diff；`kw_fields` 判据指向该文件 | W-B4-49.1、49.4 |
| `D-B4-P54` | `allow_ref: Option<AllowTraceRef>`；`None` 覆盖"缺失 / 来自 `Deny` / 未获 Allow"三种上游情形，不引入第 8 字段 | W-B4-50.2 |
| `D-B4-P55` | 撤回信号 = 装配方把只读 `current_card_status` 置 `Withdrawn`；`CardStatusSignal`（`Present`/`Withdraw`）不进 7 项输入；`compose` 在 S3 短路为 0 卡 `CardWithdrawn` | W-B4-50.4、50.5 |
| `D-B4-P56`（X-08 已接受） | `DuplicateCard` 的唯一信号 = `suppressed_notice == true`；顺序固定在 `CardWithdrawn` 之后；38 不维护卡片表 | W-B4-50.4 |
| `D-B4-P57`（X-09 已接受） | `TitleTooLong` 为类型层不可构造的防御码；AC-66 判据 = `TitleText::new(41 字)` 返回 `Err` + 40 字通过 + 私有 `title_fits_limit` 分支单测 | W-B4-50.5、53.2 |
| `D-B4-P58` | `Failed`/`Unverified` 的 `summary` 一律 `None`；固定文案 `NO_RELIABLE_SOURCE` 归 51，不进 `ComposedCard` | W-B4-51.2、54.3 |
| `D-B4-P59` | `CARDS_TEST` **同时**进 `core_files()`（source/budget）与 `zero-transcript`；与 39 的"test 只进 zero-transcript"不同 | W-B4-56.1、56.2 |
| `D-B4-P60` | `CardComposeDiagnostics`（`composes`/`succeeded`/`rejected`/`reasons[6]`）为 38 私有；**不进** `GateCounter`（32）、**不并入** `GateRejectReason`（23） | W-B4-49.3 |
| `D-B4-P61` | 38 = `cards/mod.rs` + `cards/compose.rs` 两文件 + 1 test target；`lib.rs` **+1 行**且不加顶层 `pub use`；不建 trait/端口、不实现 §4.5.2 的 `CardSynthesis` | W-B4-49.1 |
| `D-B4-P62` | 引用排序在 `cards/compose.rs` 内独立实现（`sort_by_key` 稳定、不 `dedup`），**不复用** 39 的私有 `sorted_references` | W-B4-51.1 |
| `D-B4-P63`【P6 新增】 | application **无 `serde_json` 依赖且 Cargo 零 diff**，故 38 测试的"11 键封闭 / 100 次逐字节一致"改由 `kw_fields ComposedCard`=11 + `#[derive(Serialize)]` 字段序 + `Debug`/逐字段比较承担（与 P5 的 D-B4-P44 同口径） | W-B4-51.3、55.1、55.3 |
| `D-B4-P64`【P6 新增】 | `cards/compose.rs` 顶层 `pub` 类型 = **6**（4 契约 + `CardStatusSignal` + `CardComposeDiagnostics`），设计 §6.13 自检 #1 的"4"是"契约类型"口径；`^pub fn ` 仍 = **2**；`mod.rs` 的 `pub use` 导出 **8** 个符号（设计 §6.4.3-1 的"9"把 `SUMMARY_TEMPLATE_PREFIX` 计入，该常量保留在 `compose` 模块内经 `cards::compose::SUMMARY_TEMPLATE_PREFIX` 可达） | W-B4-49.1、49.4 |
| `Y-01`【**待用户裁决**】 | `KeywordCandidate` 10 字段**无** `trigger`，而 spec §6.17-5 要求 `ComposedCard.trigger` "与候选恒等"：7 项封闭无法同时满足。推荐 **C 方案**（38 只合成 `Auto` 候选，`trigger` 输出常量 `TriggerKind::Auto`，OnDemand 不经 38）；备选 A（输入 7→8，破坏 AC-65 字面）、备选 B（输出 11→10，破坏 11+2=13） | W-B4-49.0、51.4 |
| `Y-01`【已裁决 C】 | P6 已按 C 方案落盘（`ComposedCard.trigger = TriggerKind::Auto`），见 `b4-status.md` §15.4；本表原"待裁决"原文保留为历史 | W-B4-51.4 已完成 |

**P7 增量裁决（design §4.8 / §4.5.2 / §7.11.4，执行时直接采用，不再权衡）**

| 编号 | 裁决 | 落地 |
|---|---|---|
| `D-B4-P65` | 51 点击 UI 独立交付形态 = `crates/ui-bridge/src/keyword_card/`（`mod.rs` + `view.rs`）+ `ui-bridge/src/lib.rs` +1 行 + `crates/ui-bridge/tests/keyword_card_view.rs`；**3 个纯函数 + 8 个视图类型**；不建平台适配/52 存储/`tests/replay/` | W-B4-57.2、57.3 |
| `D-B4-P66` | 51 只读 13 字段 `KeywordCardDraft`（唯一权威）与 11 项 `ComposedCard`（只读预览）；`ComposedCard` 预览**不补** `gate_decision_ref`/`click_state` | W-B4-58.2 |
| `D-B4-P67` | 视图封闭 `KeywordCardView` 10 / `CardBodyView` 4 / `CardDetailView` 5 / `ViewFacts` 6 / `CardViewOutcome` 2 / `ClickOutcome` 2；`body_visible == body.is_some()` | W-B4-57.3、58.3、58.5 |
| `D-B4-P68` | `click_state` **只降不升**；固定优先级 `Withdrawn`（短路）→ `screen_share` → `emergency_masked` → `authorization_revoked` → `!session_matches \|\| reference_stale` → 输入 `Disabled{reason}` → `alignment != Passed` → `Enabled` | W-B4-59.1、59.3 |
| `D-B4-P69` | `Withdrawn` 不生成视图（`view=None` + `CardWithdrawn`），不因重新授权/刷新/重放复活；100 次投影一致 | W-B4-59.2 |
| `D-B4-P70` | 投屏/紧急遮蔽 `body_visible=false`、`body=None`、仅 `badge_count`；点击一律拒绝；暂停/静默**不进入** `ViewFacts` | W-B4-58.3、60.1 |
| `D-B4-P71` | `KeywordCardView`/`CardBodyView`/`CardDetailView` **手写 `Debug`** 白名单，不输出标题/结论取值；展示层可持有文本，但不写记录/审计/夹具/日志/进度 | W-B4-61.1 |
| `D-B4-P72` | 新 target `keyword_card_view`（`NEW_TARGETS` 22→23）；脚本 scan 面追加 `UI_KEYWORD_APP_DIR` 与 `UI_KEYWORD_TEST`（**拼接写法**），`require_core_faces()` 增目录断言（缺目录退 2）；`--rule=targets` 循环**仍 7 项**；`tests/schema.rs` 不纳入；**X-03 同口径：脚本与文件同批提交** | W-B4-64 |
| `D-B4-P73` | `crates/ui-bridge/Cargo.toml` 新增 `meeting-copilot-application`/`meeting-copilot-domain` 两条 **path 依赖**；不新增外部 crate；`Cargo.lock` **仅因 path 依赖关系变化**（P7 零 diff 断言据此收窄，见 X-13）；不实现 §4.5.2 的 `CardSurface` trait | W-B4-57.2、57.4 |
| `D-B4-P74` | 51 **不实现** `CardSurface` trait；改 3 纯函数 + 视图类型；真实像素/焦点/投屏 API 不进入离线判定 | W-B4-57.3、64.5 |
| `D-B4-P102`【P7 新增】 | 域枚举语义名与本文件口径的映射：规格 `Unverified` ≡ 代码 `CitationAlignment::Unknown`；规格 `Ok` ≡ 代码 `ReferenceAccessStatus::Available`（沿用 D-B4-P38）。51 视图**直接透传** `CitationAlignment` 三变体，不新增第 4 态、不做 `Unknown→Passed` | W-B4-57.3、58.5、63.5 |
| `D-B4-P103`【P7 新增】 | `ComposedCard` 预览在**默认事实**（无投屏/无遮蔽/非撤回）下恒 `Disabled{NotAllowed}`（AC-75 判据）；在投屏/遮蔽事实下按 AC-76 优先落 `Disabled{ScreenShareProtection}`/`Disabled{EmergencyMasked}`（更严格，不违反"不升级"）。两条 AC 用不同用例、不同事实构造，互不冲突 | W-B4-58.2、63.1、63.2 |
| `D-B4-P104`【P7 新增】 | 契约校验顺序（唯一）：`Withdrawn` 短路 → `InputContractViolation`（`!summary_is_consistent()` / `Passed` 且引用为空 / `ComposedCard.card_status==Collapsed`）→ `ReferenceContractViolation`（`reference_id` 重复）→ 投影 | W-B4-58.1 |
| `X-12`【已核实·必须处置】 | `crates/application/tests/card_compose.rs:267` 现断言 `!Path::new("crates/ui-bridge/src/keyword_card").exists()`；P7 落盘后**必然转红**。处置：改为编译期源码文本断言 `include_str!("../src/cards/compose.rs")` 不含 `keyword_card`（更强、且与 51 面存在性解耦）；不引入 `std::fs`、不改 `crates/application/src/cards/**` | W-B4-57.1 |
| `X-13`【已核实·必须处置】 | P6 的"`Cargo.toml`/`Cargo.lock` 零 diff"在 P7 因 2 条 path 依赖而失效。处置：P7 零 diff 断言收窄为"**根** `Cargo.toml` 零 diff + `crates/ui-bridge/Cargo.toml` 仅 +2 行 + `Cargo.lock` 仅 `meeting-copilot-ui-bridge` 依赖条目变化"；P8 恢复"`Cargo.*` 零 diff"（以 P7 提交后的 HEAD 为基线） | W-B4-57.4、72.5 |
| `X-16`【已核实·必须处置】 | AC-B4-84 的字面判据"`grep -c '^| AC-B4-' spec.md` = 84"在 85~94 追加后**不可成立**（spec 当前 **94** 行）。处置：P7 收口把该判据改读为"spec 的 `^| AC-B4-` 行数 = **94**（与 P8 收口同一值）+ AC-75~84 十行齐备 + `b4-status.md` §18 十行 `passed`"；**不改 AC 编号/语义、不改 spec**（不改 `spec.md`） | W-B4-64.5、64.5 证据块 |

**P8 增量裁决（design §7.11，执行时直接采用，不再权衡）**

| 编号 | 裁决 | 落地 |
|---|---|---|
| `D-B4-P75` | 51 设置向导独立交付形态 = `crates/ui-bridge/src/settings/`（`mod.rs` + `view.rs` + `input.rs` + `probe.rs`）+ `ui-bridge/src/lib.rs` +1 行 + `crates/ui-bridge/tests/llm_settings_view.rs`；**18 顶层 `pub` 类型 + 7 顶层 `pub fn`**；不建平台句柄/安全存储适配/52 存储/`tests/replay/` | W-B4-65.1~65.5 |
| `D-B4-P76` | 视图 8 / 事实 6 / 输入 5 / 意图 3；`SECTION_ORDER` 恰 4 段且顺序固定（视图不新增 `sections` 字段） | W-B4-65.1、67.3 |
| `D-B4-P77` | `MaskedKey::Masked` **无载荷**；输出恒唯一常量，与长度/字符集/前后缀无关；`SecretInput` 不 `Serialize`/`Deserialize`/`Display`，`Debug` 恒 `***`，无长度/前后缀访问器 | W-B4-66.1、66.2 |
| `D-B4-P78` | 一步式默认 0/0/6_000/true；只填 Key 即 `can_test=true`；清单为空保留自定义入口并禁用测试/保存 | W-B4-67.1、67.3 |
| `D-B4-P79` | 预设模型名/端点显示名**只来自注入清单**；端点只允许下标；源码/测试/进度零内联模型名/域名；白名单与路径常量零 diff | W-B4-67.2、67.3、72.4 |
| `D-B4-P80` | 状态 6 态、事件 5 项、失败 8 项、字段问题 6 项、回执 2 项 XOR；单飞由 `probe_in_flight` 保证；取消回"未验证"；冷却只读注入镜像，不建熔断/计时器 | W-B4-69.1~69.3 |
| `D-B4-P81` | 8 条固定中文文案与 `ProbeFailureKind` 一一对应；不含 Key 子串/请求体/端点串/内部码；失败保留输入并恢复可测 | W-B4-70.1、70.2 |
| `D-B4-P82` | 保存/清除只产意图；清除需二次确认；安全存储拒绝降级 `Memory` + `PERSIST_FALLBACK_NOTICE`；禁止静默失败/明文落盘 | W-B4-68.1~68.3 |
| `D-B4-P83` | 新 target `llm_settings_view`；新增 `UI_SETTINGS_DIR`/`UI_SETTINGS_TEST`（**拼接写法**）+ `ui_settings_files()`；`source` 扫源码、`zero-transcript` 扫源码 + 测试；规则仍 7；`--rule=targets` 不追加；脚本与文件同批 | W-B4-72 |
| `D-B4-P84` | **零新增依赖**（`crates/ui-bridge/Cargo.toml` 与 `Cargo.lock` 零 diff）；不复用 P7 的 path 依赖；与 `keyword_card/**` 互不依赖 | W-B4-65.5、72.5 |
| `D-B4-P86` | 不引入 `SettingsContext`；7 个函数参数直接展开；`next_probe_state` 恰 7 参数（不触发 clippy 阈值） | W-B4-65.2、69.1 |
| `D-B4-P87` | 7 个顶层 `pub fn` 仅含 5 视图函数 + 2 探针函数；保存/清除意图由 `impl SettingsInput` 方法产出（4 空格缩进，不计 7） | W-B4-65.3、68.1 |
| `D-B4-P88` | `SecretInput` 内部 2 态（`Draft(Option<String>)`/`Stored`）；公开面只 `empty/stored/from_draft/is_provided/is_stored`；`ProbeRequest.credential` 保持 `SecretInput` | W-B4-66.1、69.1 |
| `D-B4-P89` | `project_settings_view` 接收 `state: SettingsFormState`，按 `has_key=false → NotConfigured`、`probe_in_flight → Testing`、冷却 → `Cooling` 归一化 | W-B4-67.3 |
| `D-B4-P90` | 只有 `ClockTick{now_ms}` 用事件自带毫值；其余用 `facts.now_ms`；`remaining_ms = cooldown_until_ms.saturating_sub(now_ms)` | W-B4-69.3 |
| `D-B4-P91` | 失败后是否 `Cooling` **只由** `facts.cooldown_until_ms > facts.now_ms` 决定；51 不维护连续失败计数 | W-B4-69.1、69.3 |
| `D-B4-P92` | 降级**不新增第 19 类型、不扩视图**：`persist_intent(Memory)` + `PERSIST_FALLBACK_NOTICE` 常量（**X-10 已接受**） | W-B4-68.3 |
| `D-B4-P93` | "字段失焦" ≡ `submitted=true`；`SettingsInput` 仍恰 5 项 | W-B4-66.3 |
| `D-B4-P94` | `SettingsView` 与选择/掩码/状态枚举 `derive(Serialize)`（仅渲染），全部**手写 `Debug`**；`SettingsInput`/`SettingsIntent`/`ProbeRequest`/`SecretInput` **不实现** `Serialize`/`Deserialize` | W-B4-65.2、65.3、66.2 |
| `D-B4-P95` | `core_files()` 追加 settings **源码**；`rule_zero_transcript()` 追加 settings **源码 + 测试**；测试**不进** `core_files()`（与 38 的 D-B4-P59 不同） | W-B4-72.1、72.2 |
| `D-B4-P96` | 8 条文案只用中文 + 标点，不含协议前缀/Key 形态/模型名/域名/`request_id`；源码注释同样不写禁词字面量；测试假 Key 统一 `test-key-` 前缀 | W-B4-70.1、71.1 |
| `D-B4-P97` | 越界下标 fail-closed：`PresetIndex(i>=len)` → `Custom{""}`（→`ModelMissing`）；`Index(i>=len)` → `Unavailable`（→`EndpointUnavailable`） | W-B4-67.2、67.3 |
| `D-B4-P98` | `FieldIssue` 派生 `Ord`；`inline_errors` 用 `BTreeSet<FieldIssue>` 去重后按声明序输出 | W-B4-66.3 |
| `D-B4-P99` | 清单以 `&[String]` 注入，**下标 0 即推荐位**；`facts.*_count` 与清单长度不一致时**以清单长度为准**并 fail-closed | W-B4-67.2、67.3 |
| `D-B4-P100` | `ProbeOutcome` 变体名 `Succeeded`/`Failed` + `to_event()` 唯一映射；settings 的 `kw_variants` 正则用 `[[:space:]]*([({]\|,)`，把 `Failed { ... }`/`Cooling { ... }`/`Preset { ... }` 结构体变体计入 | W-B4-65.5、69.2、71.1 |
| `D-B4-P101` | `submitted=false` 时 `inline_errors` **恒空**；清单为空由 `endpoint_choice=Unavailable`/`model_choice=Custom{""}`/`can_test=false` 表达 | W-B4-66.3 |
| `D-B4-P105`【P8 新增】 | `validate_settings_input` 需要 Key 的"空/长度<16/内部空白"判定，但公开面禁止长度访问器：在 `input.rs` 内给 `SecretInput` 加 **`pub(crate) fn draft_issue(&self) -> KeyDraftIssue`**（`pub(crate) enum KeyDraftIssue { Empty, TooShort, HasWhitespace, Ok }`）。`pub(crate)` 不命中 `^pub (struct\|enum) ` / `^pub fn `，**18/7 计数不变**；仍不泄露长度（只返回枚举） | W-B4-66.1、66.3 |
| `D-B4-P106`【P8 新增】 | `view.rs` 与 `probe.rs` **互相 `use`** 类型是允许的（同 crate 单编译单元）：`view.rs` 用 `ProbeFailureKind`（`Failed{failure}` 载荷），`probe.rs` 用 `SettingsFormState`（`next_probe_state` 入参/出参）。**禁止**在 `mod.rs` 加 `pub use` 环、禁止嵌套 `mod`；依赖图仍保持 `mod → {view,input,probe}`、`view → input`、`probe → {input,view}` | W-B4-65.1~65.4 |
| `D-B4-P107`【P8 新增】 | `SettingsDefaults` **手写 `Default`**（`fn default() -> Self { Self::single_step() }`），**不得** `derive(Default)`（否则 0/0/**0**/false 破坏 6_000 与 `auto_save=true`） | W-B4-67.1 |
| `X-14`【已核实】 | `--rule=source` 的禁词含 `panic!(`/`todo!(`/`unimplemented!(`/`std::env`/`LLM_`/`Instant::now`/`SystemTime::now`/`thread::sleep`/`std::net`/`tokio::time`/`static mut`/`OnceLock`/`lazy_static`/`thread_local`；settings/keyword_card **源码**（进 `core_files`）必须 0 命中，注释亦不写这些字面量 | W-B4-57.3、65.x、64.1、72.1 |
| `X-15`【已核实】 | `--rule=zero-transcript` 的模型名模式为 `(deepseek\|gpt-4\|gpt-3\|claude)-[A-Za-z0-9._-]+`，Key 形态为 `sk-[A-Za-z0-9]{8,}`；P8 测试假 Key 须为 `test-key-` 前缀（≥16 字符），模型/端点夹具只用 `model-a`/`endpoint-a` 这类中文/短串，不得出现真实模型名与域名 | W-B4-70.1、71.1、72.2 |

**P9 增量裁决（design §8.9、§20.2 D-B4-P108~P117，执行时直接采用，不再权衡）**

| 编号 | 裁决 | 落地 |
|---|---|---|
| `D-B4-P108` | 13.3 交付形态 = `crates/application/src/gate/replay.rs` 内 **7 项顶层封闭契约** + `ReplayDriver::run`；`gate/mod.rs` 仅 **+1 行** `pub use`；`api.rs` 4 个 `pub fn` 与其余 `gate/**` 零 diff；不新增 domain 契约 | W-B4-73.1、73.3、73.4 |
| `D-B4-P109` | **不从 `GateInputFixture` 反构 `GateInput`**：报告层保留 `GateReplayCase`（零文本），执行层用 `ReplayCase` 借用测试进程 arena；`from_fixture`/`InsightDraft` 命中 0 | W-B4-73.1、74.1、80.1 |
| `D-B4-P110` | 与 200 例**并存**：`tests/gate_replay.rs` 零 diff、6 用例全绿；13.3 不复刻/不迁移 `build_cases`/`Kind`/`run`，自检集独立 ≥20 例 | W-B4-79.1、80.1、80.2、81.3 |
| `D-B4-P111` | **`tests/replay/` 不解禁**：13.3 落单文件 `tests/gate_replay_driver.rs`（target `gate_replay_driver`）；不建目录 | W-B4-79.1、81.3 |
| `D-B4-P112` | 驱动只调 `detect_gaps`→`evaluate_gate`→`record_from`（`score_relevance` 在测试侧独立比对）；每例 `FixedClock::new(input.now_ms, 0)` | W-B4-75.1、75.2、75.3 |
| `D-B4-P113` | 失败报告封闭 `ReplayFailure{case_id, code}` 2 项 + 5 码；`ReplayReport` 5 字段不变；`report.mismatch_cases` 与 `failures.case_id` 逐项一致 | W-B4-76.1~76.3 |
| `D-B4-P114` | 确定性以 `ReplayReport` 5 字段 + `failures` + `records` **逐项相等**判定；不新增 `serde_json`、不引入 `rand`/`getrandom` | W-B4-78.1~78.3 |
| `D-B4-P115` | 新增第 12 个 target `gate_replay_driver`（`NEW_TARGETS` 24→25）；`check-b4-gates.sh` **零改动**；`--rule=targets` 循环不追加（X-05）；规则仍 7 条 | W-B4-81.1、81.2 |
| `D-B4-P116` | 每例 1 条 `GateRecord`，核验 `schema_version`+3 谓词；违规记 `RecordViolation` 且不丢弃案例；`Debug` 白名单与断言消息纪律 | W-B4-77.1~77.3 |
| `D-B4-P117` | AC-B4-50/64/74/84/94 的"汇总/清单"等值为**阶段时点断言**；U-06 起以 AC-B4-104 的"汇总=104、`NEW_TARGETS`=25"为准；既有编号与其余语义不重排不改写 | W-B4-81.3、81.4 |
| `D-B4-P118` | `gate/replay.rs` 是**唯一改写**的源码文件（1 行骨架 → 实现）；`gate/mod.rs` +1 行；其余 12 个 `gate/*.rs` 与 `api.rs` **零 diff**；不动 `domain/**`/`cards/**`/`ui-bridge/**` | W-B4-73.1、73.3、73.4 |
| `D-B4-P119` | 报告层 `GateReplayCase`（4，零文本）与执行层 `ReplayCase`（4，借用 `GateInput<'a>`）**双层不复用**；禁止 `from_fixture`/`InsightDraft`；禁止 `mod common` | W-B4-73.1、79.1、80.1 |
| `D-B4-P120` | 装载校验**单射顺序**：空集 → 逐项形态（空/＞64 字节/字符集）→ 唯一性 → 升序稳定排序；`InvalidCaseId` 优先于 `DuplicateCaseId` | W-B4-74.1、74.2、74.3 |
| `D-B4-P121` | **每例恰 0/1 个失败码**；判定顺序"决策 4 码 → 记录核验 1 码"；决策失配时**不再**核验记录（记录仍入 `records`） | W-B4-76.1、77.1、77.3 |
| `D-B4-P122` | 记录核验运行时口径 = `schema_version == 1` + `refs_are_valid()` + `skipped_stages_are_valid()` + `decision_tag_matches_stopped_at()`；"14 字段"由编译期结构 + shell 计数（`kw_fields`=15，剔除 `unknown` 后 14）双证；**不读** `unknown` 私有键 | W-B4-77.1、81.3 |
| `D-B4-P123` | `detect_gaps` 结果**不进报告、不产失败码**，只作链首调用 + `debug_assert!(gaps.len() <= 64)`；`score_relevance` **不在驱动内调用**，由测试侧独立比对 | W-B4-75.2、75.3、80.1 |
| `D-B4-P124` | 确定性判据用 `PartialEq` 逐项相等（`report`/`failures`/`records`），**不**序列化、**不**新增 `serde_json` | W-B4-78.2、78.3 |
| `D-B4-P125` | `gate/mod.rs` 用 `pub use replay::*;`（单行），不用逐项导出（逐项会超 100 列被 rustfmt 拆 7 行，破坏"仅 +1 行"） | W-B4-73.3 |
| `D-B4-P126` | 自检集**双层口径同时满足**：22 个 `#[test]` 函数（≥20）+ 全绿集成集 24 例（≥20）；探针集 5 例单独运行 | W-B4-79.2、79.3、80.1 |
| `D-B4-P127` | 自检集用本地 `Arena` + `build_self_check`/`build_probe`；**禁止** `fn build_cases`/`enum Kind`/`mod common`；用 `include_str!("gate_replay_driver.rs")` 自扫取证（编译期宏、零运行期 IO） | W-B4-79.1、80.2 |
| `D-B4-P128` | `NEW_TARGETS` 实况 **24** → **25**（无基准偏差）；`check-b4-gates.sh` **零改动**；`--rule=targets` 仍 7 项 | W-B4-81.1、81.2 |
| `D-B4-P129` | `ReplayCase`/`ReplayRun` **手写** `Debug`（只输出 `case_id` / `report`+两计数）；`ReplayFailure` 允许派生；`ReplayDriver` 派生（unit）；`ReplayRun` 不实现 `Serialize` | W-B4-73.1、77.2 |
| `D-B4-P130` | `ReplayMismatchCode` 互斥优先级：`ExpectedAllowGotDeny` → `ExpectedDenyGotAllow` → `ReasonMismatch`（码不同）→ `StageMismatch`（码同阶不同）→ `RecordViolation`；**码不同永远报 `ReasonMismatch`**，不叠加 | W-B4-76.1、76.3 |
| `X-17`【P9 新增·已核实】 | 用例名冲突：`spec.md` §5.11.1-7 与 §7.16 AC-101 逐字为 `replay_driver_is_order_insensitive`，而 `requirements.md` §15.5 与 design **原** §8.6.5/§8.7.1 写 `replay_driver_order_insensitive`（**design 已在 P9 收口时统一为 spec 逐字名，共 5 处**；`requirements.md` 保持原文）。处置：**以 spec 逐字为准**，测试内只定义 `replay_driver_is_order_insensitive`；另一写法登记为别名，**不另建函数**（否则 22 个计数变 23） | W-B4-78.3、80.1 |
| `X-18`【P9 新增·已核实】 | `--rule=source` 的 `strip_tests` 只截到文件内第一个 `#[cfg(test)]`，因此 `gate/replay.rs` 的**非测试区**（含注释）不得出现 `panic!(`/`todo!(`/`unimplemented!(`/`std::env`/`Instant::now`/`SystemTime::now`/`thread::sleep`/`std::net`/`tokio::time`/`static mut`/`OnceLock`/`lazy_static`/`thread_local`/`LLM_`；`tests/gate_replay_driver.rs` 由 `--rule=zero-transcript` 覆盖，同样不得出现协议前缀/`sk-` 形态/鉴权头/模型名前缀/`request_id:` | W-B4-73.2、77.2、80.2、80.3 |
| `X-19`【P9 补正·已核实】 | `GateRecord` 双口径：结构体业务字段 **14**，但 `kw_fields` 实测 **15**（多出 `unknown` 透传容器）；AC-B4-99/requirements §15.3.4-1a 原文只写 14 | 判据统一为"`kw_fields`=15 且剔除 `unknown` 后=14"双断言（73.4、77.3、81.3）；**不改 AC 语句、不改 14 字段封闭面**；驱动只读 `refs_are_valid()`，不读私有键名（D-B4-P122） | W-B4-73.4、77.1、77.3、81.3 |
| `X-20`【P9 补正·已核实】 | 基准 arena 缺 `effective_policy`；`EffectivePolicy::fail_closed()` 的 `auto_suggestion=DisabledByPolicy`，不后置则 `r22`~`r24` 的 `Allow` 不可达、`r05` 也不可构造 | 基准必须后置 `auto_suggestion=Enabled`、`consent=Explicit`，并保持 `screen_share_protection_default=true`；79.1 DoD 增 `EffectivePolicy::fail_closed` grep | W-B4-79.1 |
| `X-21`【P9 补正·已核实】 | 表述错位：`GateReplayCase` 是 **4** 字段、`ReplayReport` 才是 **5** 字段；AC-97 用例栏曾漏列 `replay_case_source_rejects_invalid_case_id`；进度登记曾只写 P118~P130 | 80.2 统一为"`GateReplayCase` 4 + `ReplayReport` 5 均可构造"；22 用例表已含 invalid_case_id；81.4 登记 **P108~P117 与 P118~P130 同时登记** | W-B4-80.2、81.4 |
| `X-22`【P9 编号冲突·已裁决】 | W-B4-81.4 原要求 `## §21 P9 增量快照`，但 `b4-status.md` 的 §21（CI deny 需求登记）与 §22（CI 修复快照）已被占用 | **P9 快照改用 `## §23`**，子节 §23.1~§23.5；DoD 锚点一律 `^## §23` 与 `sed -n '/^## §23/,$p'`；§21/§22 原样保留 | W-B4-81.4 |
| `X-23`【P9 补正·已核实】 | 基准 arena 原缺 G1 的 8 项输入基准（`auto_suggestion_enabled`/`consent_confirmed`/`session_state`/`silent_period`/`screen_share`/`session_suppressed`/`duplicate_detected`/`trigger`） | 79.1 已逐项列出；`screen_share=false` 是 `r05` 产生 `Deny{ScreenShareProtection,G1}` 的必要前提；`r01`~`r12`/`r23` 的"只改 1 项"口径不变 | W-B4-79.1 |
| `X-24`【P9 补正·已核实】 | 基准 arena 与源码实况不符：`SessionState` 无 `Active` 变体；授权 7 字段缺 `granted_at_ms`/`provider_ids` 会使 `is_complete()` 为 false；`provider` 需 `provider_ids` 且与授权同集；`lexicon_names` 与片段文本须**同串**才产候选；`scan` 11 字段与 `GateSignals` 10 字段须列全 | 已补正：`SessionState::Listening`、授权 7 字段齐备（`granted_at_ms=Some(0)`/`provider_ids=["provider-1"]`）、`provider.provider_ids` 同集、**唯一恒串 `"内部示例文本-13r"`** 复用于片段/词表/`r11` 敏感词/`query_seed`、基准 `sensitive_terms=[]`、`budget.window_start_ms=0`、`GateSignals` 全 10 字段；79.1 DoD 增 3 条 grep | W-B4-79.1 |
| `X-25`【P9 补正·已核实】 | 判据与实况不符：① `grep -c 'detect_gaps'` 会命中 import 行（必为 2）；② 脚本 `GATE_REPLAY` 指向 **domain** 的 `gate_replay.rs`（`:16`），非本阶段 `application/src/gate/replay.rs`；③ design §8.6.3 的 `api.rs` 行号已漂移（实况 `:68-70`/`:46-49`）；④ 乱序用例名以 spec 逐字为准（X-17）；⑤ `gate/mod.rs` 为 9 条 `pub use`/共 11 行 | 75.1 改用**带括号**判据 `grep -cE 'detect_gaps\('` 等；81.2 结论仍"脚本零改动"（本阶段文件由 `gate_files()` 的 `gate/*.rs` glob 覆盖）；行号改 `:68-70`/`:46-49`；用例名统一；73.3 表述为"9 条/11 行" | W-B4-73.3、75.1、78.3、81.2 |
| `X-26`【P9 本轮新增·已核实】 | 三条判据不可执行：① `kw_fields ReplayCase` 的 awk 锚 `pub struct ReplayCase {` 不匹配泛型行 `pub struct ReplayCase<'a> {`，计数恒 0；② `record_check` 为 `$RP` 私有 `fn`，集成测试 crate **不可见**，77.3 无法直接调用（升公开项会破坏 AC-95 的 7/0）；③ `LlmCapabilityView`/`GateBudget`/`GateInputFixture` **无 `Default`**，79.1/80.2 的简写会编译失败；④ 74.1 用全文件 `grep -n` 比较三错误顺序会被 73.1 的枚举定义序污染（枚举序为 `Empty`/`Duplicate`/`Invalid`） | ① 新增 `kw_fields_life`（锚 `pub struct <name><`）并在 P9 出口判据、73.1 改用它；② 77.3 改为"**pub 谓词运行时断言 + `$RP` 静态消费证据**"等价证明（`skipped_stages_are_valid()`/`decision_tag_matches_stopped_at()`），不新增公开项；③ 79.1/80.2 逐字段写全；④ 74.1 改为 `awk` 截取 `load_guard` 函数体后比对出现序列。**判据语义与阈值不变**，仅修正可执行性 | W-B4-73.1、74.1、77.3、79.1、80.2 |

### 0.4 产出文件清单（本阶段新增 15 + 扩展 3；P1~P3 另计）

**A. 本阶段（W-B4-2 ~ W-B4-8）新增 15 个文件 + 追加扩展 3 个既有文件（design §3.2.1）**

| # | 路径 | 任务 | 动作 | 预算 |
|---|---|---|---|---|
| 1 | `crates/domain/src/model/gate.rs` | W-B4-2~5 | **扩展**：既有 2 项零改，追加 29 项 | 620~900 行 / 31 类型 / 23 码 / 32 计数 / 14 字段 |
| 2 | `crates/domain/src/model/gate_replay.rs` | W-B4-2.2 | **新增**（扁平文件） | 220~280 行 / 12 类型 |
| 3 | `crates/domain/src/model/mod.rs` | W-B4-2.3 | **扩展**：仅追加 3 行 | +3 / -0 |
| 4 | `crates/application/src/gate/mod.rs` | W-B4-6.5 | 新增 | 35~50 行 / 11 `mod` + 3 导出 |
| 5 | `crates/application/src/gate/api.rs` | W-B4-6.4 | 新增（4 个 `pub fn` 唯一落点） | 90~140 行 / `^pub fn ` = 4 |
| 6 | `crates/application/src/gate/input.rs` | W-B4-6.1 | 新增 | 4 类型 |
| 7 | `crates/application/src/gate/clock.rs` | W-B4-6.2 | 新增 | 2 类型 |
| 8 | `crates/application/src/gate/record.rs` | W-B4-6.6 | 新增 | 80~120 行 / 2 违规变体 |
| 9~15 | `crates/application/src/gate/{prefilter,relevance,dedupe,budget,lease,outbound,replay}.rs` | W-B4-6.3 | 新增（各 ≤5 行 `//!` 骨架） | 7 文件 |
| 16 | `crates/application/src/lib.rs` | W-B4-6.5 | **扩展**：仅追加 1 行 | +1 / -0 |
| 17 | `scripts/check-b4-gates.sh` | W-B4-7 | 新增 | 380~460 行 / 7 规则 / 3 退出码 |
| 18 | `docs/progress/b4-status.md` | W-B4-8.1 | 新增 | 90~140 行 / 6 段 |

> 计数口径：表中 #2、#4~#15、#17、#18 为新建 = **15 个**（#9~#15 为 7 个骨架文件）；#1、#3、#16 为追加式扩展 = **3 个**。7 个 ≤5 行骨架在 P0 创建（D-B4-D09），P1~P3 只填内容、不改文件名与模块名。

**B. P1~P3 另计（本阶段不创建）**

| 层 | 路径 | 任务 |
|---|---|---|
| script | `scripts/test-name-inventory.sh`（`NEW_TARGETS` 13→19；**P4 再追加 `gate_keyword` → 20**，见 W-B4-42.2） | W-B4-14.1、W-B4-23.1、W-B4-28.5、W-B4-42.2 |
| test | `crates/application/tests/gate_prefilter.rs` / `gate_relevance.rs` / `gate_dedupe.rs` / `gate_budget.rs` / `gate_outbound.rs` / `gate_replay.rs` | W-B4-14/16/18/19/24/28 |

**C. P4 增量另计（新增 1 + 扩展 6；W-B4-34 ~ W-B4-42）**

| # | 路径 | 任务 | 动作 | 预算 |
|---|---|---|---|---|
| 1 | `crates/domain/src/model/gate_keyword.rs` | W-B4-34 | **新增**（11 项 = 4 结构 + 7 枚举） | 220~300 行 |
| 2 | `crates/domain/src/model/mod.rs` | W-B4-34.8 | **扩展**：+2 行（`pub mod gate_keyword;` + `pub use gate_keyword::*;`） | +2 / -0 |
| 3 | `crates/domain/src/vo/ids.rs` | W-B4-34.1 | **扩展**：+1 行 `id_type!(KeywordRef, …)` | +1 / -0 |
| 4 | `crates/domain/src/vo/text.rs` | W-B4-34.1 | **扩展**：+1 行 `bounded_text!(NormalizedTerm, …)` | +1 / -0 |
| 5 | `crates/domain/src/vo/mod.rs` | W-B4-34.1 | **扩展**：+2 行导出 | +2 / -0 |
| 6 | `crates/application/src/gate/keyword.rs` | W-B4-36、37 | **新增**（4 个 `pub fn` + 5 个入参/输出类型；**不在 `api.rs`**） | 180~260 行 |
| 7 | `crates/application/src/gate/mod.rs` | W-B4-37.4 | **扩展**：+1 行 `pub mod keyword;`（**不**在 `pub use` 列 4 函数面） | +1 / -0 |
| 8 | `crates/application/src/gate/prefilter.rs` | W-B4-35.1~35.2 | **扩展**：新增 `pub(crate) fn scan_candidates_with_rule`，`scan_candidates` 改为复用 | +25 / -8 |
| 9 | `crates/application/src/gate/diagnostics.rs` | W-B4-35.3~35.4 | **扩展**：G2/G3 跳过投影 2 处 | +6 / -2 |
| 10 | `crates/application/tests/gate_keyword.rs` | W-B4-38~40 | **新增 target**（13 个用例） | 380~520 行 |
| 11 | `crates/application/tests/gate_replay.rs` | W-B4-41 | **追加**（生成器 + runner + 2 个用例；`Harness` 复用） | +260 / -0 |
| 12 | `scripts/check-b4-gates.sh` | W-B4-42.1 | **扩展**：`GATE_KEYWORD` 变量 + `core_files()` + `zero-transcript` `files` | +4 / -1 |
| 13 | `scripts/test-name-inventory.sh` | W-B4-42.2 | **扩展**：`NEW_TARGETS` 尾部追加 `gate_keyword` | +1 token |
| 14 | `docs/progress/b4-status.md` | W-B4-42.4 | **扩展**：新增 §11（P4 快照 + 7 条 AC 三态 + U-01/U-02 关闭登记） | 60~90 行 |

> P4 计数口径：新增文件 **2 个**（#1、#10）；追加式扩展 **8 个**（#2~#5、#7~#9、#12~#14）；#6 为新增模块文件，计入"新增 2"之内。**零 diff 清单**：`gate/api.rs`、`gate/input.rs`、`gate/replay.rs`、`gate_replay.rs`、`gate.rs`、`enums.rs`、`Cargo.toml`、`Cargo.lock`、`understand/store.rs`、`understand/deduplicator.rs`。

**D. P5 增量另计（新增 4 + 扩展 3；W-B4-43 ~ W-B4-48）**

| # | 路径 | 任务 | 动作 | 预算 |
|---|---|---|---|---|
| 1 | `crates/domain/src/model/citation.rs` | W-B4-43 | **新增**（6 项 = 3 struct + 3 enum） | 180~260 行 |
| 2 | `crates/application/src/citation/mod.rs` | W-B4-44.1 | **新增**（4 行导出） | 4 行 |
| 3 | `crates/application/src/citation/align.rs` | W-B4-44、45 | **新增**（2 个 `pub fn` + 1 诊断类型 + 5 私有步骤） | 160~240 行 |
| 4 | `crates/application/tests/citation_align.rs` | W-B4-46 | **新增 target**（12 用例） | 320~460 行 |
| 5 | `crates/domain/src/model/mod.rs` | W-B4-43.6 | **扩展**：+2 行（`pub mod citation;` + `pub use citation::*;`） | +2 / -0 |
| 6 | `crates/application/src/lib.rs` | W-B4-44.1 | **扩展**：+1 行 `pub mod citation;` | +1 / -0 |
| 7 | `scripts/check-b4-gates.sh` | W-B4-47.1/47.2 | **扩展**：3 个变量 + `citation_files()` + `require_core_faces()` + `core_files()` + `zero-transcript` | +8 / -2 |
| 8 | `scripts/test-name-inventory.sh` | W-B4-47.3 | **扩展**：`NEW_TARGETS` 尾部追加 `citation_align` | +1 token |
| 9 | `docs/progress/b4-status.md` | W-B4-48.1/48.2 | **扩展**：新增 §13（P5 快照 + 7 条 AC 三态 + U-04 关闭） | 60~90 行 |

> P5 计数口径：新增文件 **4 个**（#1~#4，其中 #2/#3 同属新目录）；追加式扩展 **5 个**（#5~#9）。**P5 零 diff 清单（在 P4 十条之外追加）**：`gate/keyword.rs`、`gate/keyword.rs` 所属 `gate/**`、`api.rs`、`gate_keyword.rs`、`gate.rs`、`gate_replay.rs`、`enums.rs`、`Cargo.toml`、`Cargo.lock`、`understand/store.rs`、`understand/deduplicator.rs`；`tests/replay/` 保持不存在。

**E. P6 增量另计（新增 3 + 扩展 4；W-B4-49 ~ W-B4-56）**

| # | 路径 | 任务 | 动作 | 预算 |
|---|---|---|---|---|
| 1 | `crates/application/src/cards/compose.rs` | W-B4-49~51 | **新增**（6 顶层类型 = 4 契约 + 状态信号 + 诊断；2 `pub fn`；5 私有步骤） | 240~360 行 |
| 2 | `crates/application/src/cards/mod.rs` | W-B4-49.1 | **新增**（4 行：`pub mod compose;` + 1 行导出 9 符号） | 4 行 |
| 3 | `crates/application/tests/card_compose.rs` | W-B4-52~55 | **新增 target**（14 用例 + 本地构造器） | 340~480 行 |
| 4 | `crates/application/src/lib.rs` | W-B4-49.1 | **扩展**：+1 行 `pub mod cards;`（`capture` 与 `citation` 之间；**不**加 `pub use`） | +1 / -0 |
| 5 | `scripts/check-b4-gates.sh` | W-B4-56.1/56.2 | **扩展**：2 变量（`CARDS_APP_DIR`/`CARDS_TEST`）+ `cards_files()` + `require_core_faces()` + `core_files()` + `zero-transcript` | +12 / -2 |
| 6 | `scripts/test-name-inventory.sh` | W-B4-56.3 | **扩展**：`NEW_TARGETS` 尾部追加 `card_compose`（21→22） | +1 token |
| 7 | `docs/progress/b4-status.md` | W-B4-56.5 | **扩展**：新增 §15（P6 快照 + 10 条 AC 三态 + U-03 关闭） | 60~90 行 |

> P6 计数口径：新增文件 **3 个**（#1~#3，其中 #1/#2 同属新目录 `cards/`）；追加式扩展 **4 个**（#4~#7）。合计 7 个文件面。
> **P6 零 diff 清单（在 P5 十一条之外追加）**：`crates/application/src/gate/**`（整目录）、`crates/domain/src/model/**`（整目录，含 `gate_keyword.rs`/`gate.rs`/`gate_replay.rs`/`enums.rs`/`citation.rs`/`mod.rs`）、`gate/api.rs`（4 函数）、`Cargo.toml`、`Cargo.lock`、`scripts/check-capture-offline.sh`、`scripts/check-b3-gates.sh`、`scripts/check-mac-scope.sh`、`crates/application/src/understand/store.rs`、`crates/application/src/understand/deduplicator.rs`；`crates/application/tests/replay/` 与 `crates/ui-bridge/src/keyword_card/` 保持不存在。
> 【**P7 加注**】上句末的"`crates/ui-bridge/src/keyword_card/` 保持不存在"在 **P7 之后失效**：P7 落盘该目录属预期交付；P6 的该断言已由 X-12 在 W-B4-57.1 改写为"38 源码不含 `keyword_card`"（更强、不依赖 51 面存在性）。**零 diff 清单同样加注**：P6 的 `Cargo.lock` 零 diff 在 P7 因 2 条 path 依赖而失效（X-13），P8 以 P7 提交后的 HEAD 为基线恢复零 diff。

**F. P7 增量另计（新增 3 + 扩展 4；W-B4-57 ~ W-B4-64）**

| # | 路径 | 任务 | 动作 | 预算 |
|---|---|---|---|---|
| 1 | `crates/ui-bridge/src/keyword_card/view.rs` | W-B4-57.3、58、59、60、61 | **新增**（8 顶层类型 + 3 顶层 `pub fn` + 私有步骤 + `#[cfg(test)]` 单测） | 280~420 行 |
| 2 | `crates/ui-bridge/src/keyword_card/mod.rs` | W-B4-57.2 | **新增**（`pub mod view;` + 1 行 `pub use` 导出 11 符号） | 4~8 行 |
| 3 | `crates/ui-bridge/tests/keyword_card_view.rs` | W-B4-62、63 | **新增 target**（19 个 AC 用例 + 1 个自检用例 + 本地夹具） | 420~600 行 |
| 4 | `crates/ui-bridge/src/lib.rs` | W-B4-57.2 | **扩展**：+1 行 `pub mod keyword_card;`（`pub mod dto;` 之后；**不加**顶层 `pub use`） | +1 / -0 |
| 5 | `crates/ui-bridge/Cargo.toml` | W-B4-57.2 | **扩展**：+2 条 path 依赖（`meeting-copilot-application`/`meeting-copilot-domain`），**不新增外部 crate** | +2 / -0 |
| 6 | `crates/application/tests/card_compose.rs` | W-B4-57.1 | **扩展**：仅改 1 行路径断言（X-12），`1 +/1 -` | +1 / -1 |
| 7 | `scripts/check-b4-gates.sh` | W-B4-64.1/64.2 | **扩展**：2 变量（`UI_KEYWORD_APP_DIR`/`UI_KEYWORD_TEST`，**拼接写法**）+ `ui_keyword_files()` + `require_core_faces()` + `core_files()` + `zero-transcript` | +12 / -2 |
| 8 | `scripts/test-name-inventory.sh` | W-B4-64.3 | **扩展**：`NEW_TARGETS` 尾部追加 `keyword_card_view`（22→23） | +1 token |
| 9 | `docs/progress/b4-status.md` | W-B4-64.5 | **扩展**：新增 §18（P7 快照 + 10 条 AC 三态 + X-12/X-13 登记 + U-05 部分关闭） | 60~90 行 |

> P7 计数口径：新增文件 **3 个**（#1~#3，其中 #1/#2 同属新目录 `keyword_card/`）；追加式扩展 **6 个**（#4~#9）。合计 9 个文件面。
> **P7 零 diff 清单（在 P6 十四条之外，按 X-13 收窄）**：`crates/application/src/gate/**`、`crates/application/src/cards/**`（**源码**）、`crates/domain/src/model/**`、`crates/infrastructure/src/llm/**`、`crates/ui-bridge/src/dto.rs`、`crates/ui-bridge/tests/schema.rs`、`scripts/check-capture-offline.sh`、`scripts/check-b3-gates.sh`、`scripts/check-mac-scope.sh`、`crates/application/src/understand/store.rs`、`crates/application/src/understand/deduplicator.rs`、**根** `Cargo.toml`；`crates/application/tests/replay/` 与 `crates/ui-bridge/src/settings/` 仍不存在。**唯一允许**：`crates/ui-bridge/Cargo.toml` +2 行、`Cargo.lock` 的 `meeting-copilot-ui-bridge` 依赖条目变化、`crates/application/tests/card_compose.rs` 1 行（X-12）。

**G. P8 增量另计（新增 5 + 扩展 2；W-B4-65 ~ W-B4-72）**

| # | 路径 | 任务 | 动作 | 预算 |
|---|---|---|---|---|
| 1 | `crates/ui-bridge/src/settings/mod.rs` | W-B4-65.1 | **新增**（`pub mod` ×3 + 1 个 `pub use` 块 + 3 常量） | 24~36 行 |
| 2 | `crates/ui-bridge/src/settings/view.rs` | W-B4-65.2、66.2、66.3、67.1~67.3、70.2 | **新增**（8 项视图 + 6 项事实 + 4 项默认值 + 4 视图子类型 + 5 顶层 `pub fn` + 单测） | 300~420 行 |
| 3 | `crates/ui-bridge/src/settings/input.rs` | W-B4-65.3、66.1、68.1 | **新增**（`SecretInput`/`SettingsInput`/2 草稿/3 意图/2 目标/4 请求 + 2 意图方法 + `pub(crate)` 判定） | 220~320 行 |
| 4 | `crates/ui-bridge/src/settings/probe.rs` | W-B4-65.4、69.1~69.3、70.1 | **新增**（5 事件 + 8 失败 + 2 回执 + 文案表 + 2 顶层 `pub fn` + 单测） | 200~300 行 |
| 5 | `crates/ui-bridge/tests/llm_settings_view.rs` | W-B4-71 | **新增 target**（18 个 AC 用例 + 本地夹具） | 420~620 行 |
| 6 | `crates/ui-bridge/src/lib.rs` | W-B4-65.5 | **扩展**：+1 行 `pub mod settings;`（`pub mod keyword_card;` 之后） | +1 / -0 |
| 7 | `scripts/check-b4-gates.sh` | W-B4-72.1/72.2 | **扩展**：2 变量（`UI_SETTINGS_DIR`/`UI_SETTINGS_TEST`，**拼接写法**）+ `ui_settings_files()` + `require_core_faces()` + `core_files()` + `zero-transcript` | +12 / -2 |
| 8 | `scripts/test-name-inventory.sh` | W-B4-72.3 | **扩展**：`NEW_TARGETS` 尾部追加 `llm_settings_view`（23→24） | +1 token |
| 9 | `docs/progress/b4-status.md` | W-B4-72.5 | **扩展**：新增 §19（P8 快照 + 10 条 AC 三态 + X-10/X-11 落地登记 + U-05 关闭） | 60~90 行 |

> P8 计数口径：新增文件 **5 个**（#1~#5，其中 #1~#4 同属新目录 `settings/`）；追加式扩展 **4 个**（#6~#9）。合计 9 个文件面。
> **P8 零 diff 清单（在 P7 十八条之上追加）**：`crates/ui-bridge/Cargo.toml`（**P8 不得再改**）、`Cargo.lock`（**P8 不得再改**）、`crates/ui-bridge/src/keyword_card/**`（P7 已落，P8 零 diff）、`crates/ui-bridge/src/dto.rs`、`crates/ui-bridge/tests/schema.rs`、`crates/application/tests/card_compose.rs`（P7 已改，P8 零 diff）；`crates/application/tests/replay/` 保持不存在。`lib.rs` 的 `git diff` 累计应为 **+2 / -0**（P7 +1、P8 +1）。

**H. P9 增量另计（改写 1 + 新增 1 + 扩展 2；W-B4-73 ~ W-B4-81）**

| # | 路径 | 任务 | 动作 | 预算 |
|---|---|---|---|---|
| 1 | `crates/application/src/gate/replay.rs` | W-B4-73~78 | **改写**（1 行骨架 → 7 项顶层契约 + `ReplayDriver::run` + 3 私有助手） | 180~240 行 |
| 2 | `crates/application/src/gate/mod.rs` | W-B4-73.3 | **扩展**：仅 +1 行 `pub use replay::*;`（`pub(crate) mod replay;` 保留于 `:35`） | +1 / -0 |
| 3 | `crates/application/tests/gate_replay_driver.rs` | W-B4-74.3、75.3、76.3、77.3、78.3、79、80 | **新增 target**（22 个测试函数 + 本地 `Arena` + 24 例全绿集 + 5 例探针集 + `record_check` 合成单测） | 560~760 行 |
| 4 | `scripts/test-name-inventory.sh` | W-B4-81.1 | **扩展**：`NEW_TARGETS` 尾部追加 `gate_replay_driver`（24→25） | +1 token |
| 5 | `docs/progress/b4-status.md` | W-B4-81.4 | **扩展**：新增 §23（P9 快照 + 10 条 AC 三态 + D-B4-P118~P130 落地登记 + U-06 关闭；X-22 已裁决用 §23） | 60~90 行 |

> P9 计数口径：**改写 1**（#1）、**新增 1**（#3）、**追加式扩展 3**（#2、#4、#5），合计 5 个文件面；`scripts/check-b4-gates.sh` **零改动不计入**。
> **P9 零 diff 清单（在 P8 之上追加）**：`crates/application/src/gate/api.rs` + `gate/{prefilter,relevance,dedupe,budget,lease,outbound,record,diagnostics,input,clock,keyword}.rs`（12 文件）、`crates/domain/src/model/**`、`crates/application/src/cards/**`、`crates/ui-bridge/**`、`crates/infrastructure/**`、`crates/application/tests/gate_replay.rs`、`scripts/check-b4-gates.sh`、根 `Cargo.toml`/`Cargo.lock`/`crates/application/Cargo.toml`；`crates/application/tests/replay/` 保持不存在。**唯一允许**：`gate/replay.rs` 全文改写、`gate/mod.rs` +1 行、`scripts/test-name-inventory.sh` 1 token、`b4-status.md` 追加。

### 0.5 关键路径与依赖

```text
已完成：W-B4-1（C-01 码表，HEAD ade9111；本阶段不重做）
P0 剩余：W-B4-2 ─► {W-B4-3 ∥ W-B4-4 ∥ W-B4-5(5.1~5.4a)} ─► W-B4-6(含 5.4b → 6.6) ─► W-B4-7 ─► W-B4-8
P1: W-B4-9 ∥ W-B4-10 ─► W-B4-11 ─► W-B4-12 ─► W-B4-13 ─► W-B4-14
P2: W-B4-15 ─► W-B4-16 ∥ W-B4-17 ─► W-B4-18 ─► W-B4-22 ─► W-B4-19 ─► W-B4-20 ─► W-B4-21 ─► W-B4-23
P3: W-B4-24 ─► W-B4-25 ─► W-B4-26 ─► W-B4-27 ─► W-B4-28 ─► W-B4-29 ─► W-B4-30 ─► W-B4-31 ─► W-B4-32 ─► W-B4-33
P4（增量）: W-B4-34 ─► W-B4-35 ─► W-B4-36 ─► W-B4-37 ─► {W-B4-38 ─► W-B4-39 ─► W-B4-40} ∥ W-B4-41 ─► W-B4-42
P5（增量）: W-B4-43 ─► W-B4-44 ─► W-B4-45 ─► W-B4-46 ─► W-B4-47 ─► W-B4-48
P6（增量）: W-B4-49 ─► W-B4-50 ─► W-B4-51 ─► W-B4-52 ─► W-B4-53 ─► W-B4-54 ─► W-B4-55 ─► W-B4-56
P7（51 点击 UI）: W-B4-57 ─► {W-B4-58 ─► W-B4-59 ─► W-B4-60} ─► W-B4-61 ─► W-B4-62 ─► W-B4-63 ─► W-B4-64
P8（51 设置向导）: W-B4-65 ─► {W-B4-66 ─► W-B4-67} ─► {W-B4-68 ∥ W-B4-69 ─► W-B4-70} ─► W-B4-71 ─► W-B4-72
P9（13.3 回放本体）: W-B4-73.0 ─► W-B4-73 ─► W-B4-74 ─► W-B4-75 ─► W-B4-76 ─► W-B4-77 ─► W-B4-78 ─► W-B4-79 ─► W-B4-80 ─► W-B4-81
```

- **P7 不可压缩四点**：① W-B4-57.0（提交 P6，闸门）必须先于 57.1；② W-B4-57.1（X-12 断言修正）必须先于 57.2，否则 `card_compose` 14 用例在 P7 落盘后立刻转红，全量回归不绿；③ W-B4-57.2（`Cargo.toml` +2 + `lib.rs` +1 + `keyword_card/mod.rs`）必须先于 57.3 的 `view.rs`（否则 `crate::keyword_card` 不可见）；④ W-B4-58.1（契约校验顺序）必须早于 58.3/58.5（屏蔽态与结论态都建立在"输入已合法"之上）。
- **P7 依赖约束**：58.1 → 58.2/58.3/58.4/58.5；59.1 依赖 58.1 的 `CardViewOutcome`；60.1 依赖 58.5（`CardBodyView`）与 59.1（`ClickState` 归一化结果）；61.1 依赖 58/59/60 的三类型定稿；62 依赖 57~61 全部编译通过；63 依赖 62；64 必须在 57~63 全绿后、并与 57~63 **同批提交**（X-03）。
- **P7 可并行**：58.2（Composed 预览）∥ 58.3（屏蔽态）；63.1~63.6 六组用例互不依赖（同文件不同 `#[test]`）；64.3（token）∥ 64.4（targets 口径复核）。
- **P8 不可压缩三点**：① W-B4-65.1（`mod.rs` + 3 常量）必须先于 65.2~65.4（子模块声明）；② W-B4-66.1（`SecretInput` 2 态 + `pub(crate) draft_issue`）必须先于 66.3（`validate_settings_input`）与 67.1（`apply_defaults` 的 `Stored`/`empty` 分支）；③ W-B4-69.1（6×5 转移表）必须先于 69.2（`to_event` + 自动保存）与 70.2（失败后恢复可测）。
- **P8 依赖约束**：66.2（`MaskedKey`/`mask_api_key`）→ 67.3（`key_display` 投影）；67.3 依赖 67.1/67.2；68.1 依赖 66.1、68.3 依赖 68.1；69.1 依赖 65.2（`SettingsFormState`）与 65.3（`SettingsIntent`/`ProbeRequest`）；71 依赖 65~70；72 必须在 65~71 全绿后、并与 65~71 **同批提交**（X-03）。
- **P8 可并行**：68（意图）∥ 69（状态机）——两者都只依赖 65/66/67 的类型；71.2~71.6 五组用例互不依赖；72.3（token）∥ 72.4（targets/`cargo tree` 口径复核）。
- **两个串行收口（X-03 同口径）**：P7 的 W-B4-64.1/64.2 与 P8 的 W-B4-72.1/72.2 都把新目录纳入 `require_core_faces()`，**目录缺失即令 7 条 `--rule` 全部退 2**；因此脚本改动必须与对应源文件/测试**同批提交**，禁止先改脚本、禁止单独提交脚本。且 **P7 必须先提交，P8 才开工**（X-11）。
- **P7→P8 之间必须有一次提交**：P8 的 `Cargo.toml`/`Cargo.lock` 零 diff 判据以 **P7 提交后的 HEAD** 为基线；若 P7/P8 未分别提交，X-13 的收窄口径无法复核。
- **P8→P9 之间必须有一次提交**：P9 的 `gate/**`/`domain/**`/`tests/gate_replay.rs`/`scripts/check-b4-gates.sh` 零 diff 判据以**基线 HEAD `f153d7f`**（P8 + CI deny 修复；含 `b4-status.md` §20 登记）为基线；W-B4-73.0 完成该闸门。
- **P9 不可压缩四点**：① W-B4-73.0（提交 §20 登记）必须先于 73.1，否则零 diff 基线含未提交文档；② W-B4-73.1（7 类型 + `impl ReplayDriver` 可编译）必须先于 73.2/74/75（否则 `load_guard`/`compare_decision`/`record_check` 无承载类型）；③ W-B4-73.3（`pub use replay::*;`）必须先于 79/80（集成测试从 crate 外 `use meeting_copilot_application::gate::*`，否则类型不可见）；④ W-B4-75.1（常量时钟 + 三调用）必须先于 76/77（决策与记录都来自该步）。
- **P9 依赖约束**：74.1（`load_guard` 单射）→ 74.2（稳定排序）→ 74.3（4 个装载/排序用例）；75.1（`evaluate_gate` + `FixedClock`）→ 76.1（`compare_decision` 4 码）与 77.1（`record_check`）；76.1 → 76.2（汇总不变量）；77.1 → 77.2（`Debug` 白名单）；78 依赖 74~77 全部定稿；79 依赖 73.3 与 78；80 依赖 79 的两个数据集；81 必须在 73~80 全绿后、并与 73~80 **同批提交**（`test-name-inventory.sh` 的 token 若单独先改，`--diff` 会把新 target 判为"未登记"）。
- **P9 可并行**：74.2（排序）∥ 75.1（驱动主流程）——两者只改 `replay.rs` 不同函数，但同文件编辑须串行提交；76.1 ∥ 77.1（两个私有助手互不依赖）；79.1（`Arena`）∥ 80.2（自扫断言模板）在 `Arena` 骨架可编译后并行；81.1（token）∥ 81.2（`--rule=targets`/七规则复核）。
- **P9 串行收口**：W-B4-81.4（`b4-status.md` §23 + AC 三态 + U-06 关闭）必须在 73~81.3 全绿后一次性登记，避免进度表与实际状态抖动；U-06 整行标 `closed` 但**保留**"规格已定"原文（AC-104 的 `grep -c 'U-06.*回放本体'` 与 AC-103 的历史证据依赖它）。

- **P4 不可压缩的三步**：34（domain 契约可编译）→ 35（G1 同源规则标签，否则 `institution_role` 不可达）→ 36/37（派生与准入）。
- **P4 可并行**：W-B4-38~40 的用例与 W-B4-41 的 200 例互不依赖（都只依赖 36/37）；但同文件 `tests/` 目录不同文件，可并行提交。
- **P4 唯一串行收口**：W-B4-42 必须在 34~41 全绿后执行（脚本扫描面 + target 清单 + 三态登记一次收口，避免中途红绿抖动）。
- **P5 不可压缩三点**：① W-B4-43.6（`model/mod.rs` +2 行）必须先于 44.1，否则 `meeting_copilot_domain::model::citation` 不可见；② W-B4-44.4（`CitationDiagnostics`）必须先于 46.x，否则 `basis` 分布不可断言；③ W-B4-45.1（`project_alignment`）必须先于 46.3/46.4。
- **P5 可并行**：46.2（AC-58 只读）∥ 46.3（AC-59 三态）；47.3（inventory token）∥ 47.4（targets 口径复核）。
- **P5 两个串行收口（X-03）**：W-B4-47 必须在 43/44 落盘**之后**、并与 43~46 **同批提交**（否则 `require_core_faces()` 令 7 条规则全部退 2）；W-B4-48 必须在 43~47 全绿后一次性登记（避免进度表与实际状态抖动）。
- **P6 不可压缩三点**：① W-B4-49.0（Y-01 `trigger` 裁决）必须先于 49.1，否则 11 项组装口径无法定稿；② W-B4-49.1（`cards/mod.rs` + `compose.rs` 骨架 + `lib.rs` +1 行）必须先于 50/51（否则 `crate::cards` 不可见）；③ W-B4-51.1（`project_alignment` 调用）必须先于 53/54（AC-68 唯一投影入口）。
- **P6 可并行**：W-B4-53/54/55 三组用例互不依赖（同文件不同 `#[test]` 函数，可串行提交、断言独立）；W-B4-56.1（脚本扫描面）与 56.3（token）可并行。
- **P6 两个串行收口（X-03 同口径）**：W-B4-56.1/56.2（`require_core_faces()` 纳入 `CARDS_APP_DIR`）必须在 `cards/**` + `tests/card_compose.rs` 落盘**之后**、并与 49~55 **同批提交**（否则 7 条规则全部退 2）；W-B4-56.5（状态登记与出口收口）必须在 49~56 全绿后一次性执行。

- **已完成**：`gate.rs` 既有 `Visibility` + `GateRejectReason`（23 变体 / `ALL` / `as_str` / `to_deny_code` 12 `Some` / `ui_message_key` / `visibility`）与 7 个单测均在 `ade9111` 交付；本阶段只追加，不重做、不改写。
- **不可压缩**：2.1（类型可编译）→ 2.2/2.3（domain 可解析 12+29 项）必须先于 W-B4-3~5 的行为方法。
- **5.4 拆分**：5.4a（domain `counter_at`）随 W-B4-5；5.4b（application `record::bump_by`）并入 W-B4-6.6（`gate/` 目录存在后）。
- **可并行**：W-B4-3 / W-B4-4 / W-B4-5 三者只改 `gate.rs` 不同段落，可并行；但同文件编辑须串行提交，避免冲突。
- **P0 内编译顺序（每步都必须 `cargo build`/`cargo test` 可过，design §3.2.4）**

| 步 | 动作 | 完成后验证 | 对应子任务 |
|---|---|---|---|
| S1 | `gate.rs` 追加 29 项类型（无行为） | `cargo test -p meeting-copilot-domain` 编译通过 | 2.1 |
| S2 | `gate_replay.rs` 12 项 + `mod.rs` 3 行 | `cargo test -p meeting-copilot-domain` 退 0（既有 7 例不回归） | 2.2、2.3 |
| S3 | `gate.rs` 行为方法（`validate`/`milli`/`per_window`/`chars`/`session_remaining_with`/`commit`/`counter_at`/谓词） | `cargo test -p meeting-copilot-domain` 退 0 | 3.x、4.x、5.x |
| S4 | `gate/**` 11 文件骨架 + `input`/`clock` + `mod.rs` + `lib.rs` | `cargo build -p meeting-copilot-application` 退 0 | 6.1~6.5 |
| S5 | `api.rs` 4 函数 + `record.rs` 校验/映射 | `cargo build` 退 0 + 三条 grep 断言 4/4/1 | 6.4、6.6、6.7 |
| S6 | `check-b4-gates.sh` 7 规则 | 7 条 `--rule` 全部有确定退出码 | 7.1~7.6 |
| S7 | `--self-test` | 退 1 且 `git status` 为空 | 7.7 |
| S8 | `b4-status.md` + 取证 | §8.2~8.6 全部命令退 0 | 8.1~8.6 |

### 0.6 AC-B4-01~50 覆盖矩阵（50/50）

| AC | 主判据（用例名 / 命令） | 任务 |
|---|---|---|
| 01 | `prefilter_scores_six_gap_rules`、`acronym_whitelist_is_configurable` | W-B4-11.2、11.4 |
| 02 | `first_seen_suppresses_second_candidate`、`sensitive_term_is_not_a_seed` | W-B4-12.1、12.2 |
| 03 | `unanswered_question_yields_after_ten_seconds`、`answered_question_is_skipped` | W-B4-12.3 |
| 04 | `below_half_gap_confidence_yields_no_candidate`、`exactly_half_gap_confidence_passes` | W-B4-12.4 |
| 05 | `candidate_is_ready_within_two_seconds_of_stable_prefix` | W-B4-12.5 |
| 06 | `gap_kind_is_closed_enum_and_institution_maps_to_proper_noun`、`dedupe_key_is_32_hex` | W-B4-11.3、11.5 |
| 07 | `prefilter_is_pure_and_deterministic` + `--rule=source` | W-B4-11.5、W-B4-7.2 |
| 08 | `prefilter_denies_eleven_hard_cases` | W-B4-13.2 |
| 09 | `relevance_weights_match_spec`、`relevance_full_score_is_thousand` | W-B4-15.1 |
| 10 | `time_decay_at_180_seconds_is_367_milli`、`time_decay_at_zero_is_thousand` | W-B4-15.2 |
| 11 | `threshold_profiles_are_072_062_052`、`balanced_threshold_boundary` | W-B4-16.1 |
| 12 | `relevance_parts_are_all_reported` | W-B4-15.3 |
| 13 | `low_relevance_is_denied`、`empty_topic_is_deterministic` | W-B4-16.2、16.3 |
| 14 | `relevance_never_calls_llm_or_network`、`relevance_is_deterministic_for_mixed_text` | W-B4-16.4 |
| 15 | `semantic_cluster_threshold_is_088`、`cluster_boundary_08799_08800` | W-B4-17.1 |
| 16 | `duplicate_key_blocks_within_ten_minutes`、`duplicate_key_expires_at_ten_minutes` | W-B4-17.2 |
| 17 | `topic_quota_is_two_per_ten_minutes`、`global_quota_is_three_per_ten_minutes` | W-B4-18.1 |
| 18 | `entity_cooldown_is_five_minutes`、`entity_cooldown_expires_at_five_minutes` | W-B4-18.2 |
| 19 | `suppression_blocks_auto_allows_on_demand` | W-B4-18.3 |
| 20 | `fresh_cache_is_allowed`、`stale_cache_is_denied` | W-B4-18.4 |
| 21 | `candidate_convergence_keeps_top_three` | W-B4-18.5 |
| 22 | `budget_is_independent_of_llm_live_cap` + `--rule=budget` | W-B4-19.1 |
| 23 | `session_outbound_cap_is_120` | W-B4-19.2 |
| 24 | `token_bucket_capacity_and_refill`、`frequency_profiles_are_1_3_5` | W-B4-19.3 |
| 25 | `gateway_cooling_denies_without_retry` | W-B4-20.1 |
| 26 | `gateway_cooling_does_not_block_admission` | W-B4-20.2 |
| 27 | `budget_recovers_after_window`、`fourth_concurrent_request_is_queued` | W-B4-20.3、W-B4-21.2 |
| 28 | `authorization_records_seven_required_fields` | W-B4-24.1 |
| 29 | `unauthorized_outbound_is_zero` | W-B4-24.2 |
| 30 | `revoked_authorization_is_not_auto_restored`、`reauthorization_bumps_notice_version` | W-B4-24.3 |
| 31 | `auto_query_is_capped_at_80`、`on_demand_query_is_capped_at_200`、`oversize_never_reaches_sanitizer` | W-B4-25 |
| 32 | `authorization_change_invalidates_old_grant` | W-B4-24.4 |
| 33 | `--rule=outbound` 退 0 | W-B4-27.1 |
| 34 | `outbound_audit_has_required_fields_without_text` + `--rule=zero-transcript` | W-B4-27.2、27.4 |
| 35 | `audit_unavailable_blocks_outbound` | W-B4-26.1 |
| 36 | `sanitize_failure_blocks_outbound` | W-B4-26.4 |
| 37 | `gate_consumes_insights_readonly` + `git diff --quiet HEAD` | W-B4-29.1、29.2 |
| 38 | `gate_input_has_no_insight_draft` + grep 0 | W-B4-9.3 |
| 39 | `cargo test --workspace` ≥584/9 + `--diff` | W-B4-30.1、30.2 |
| 40 | 常量 grep 5 项 + B3 48 条 AC + `--diff` | W-B4-30.3 |
| 41 | `cargo deny` 四项 + `Cargo.toml`/`Cargo.lock` 差集 | W-B4-30.4 |
| 42 | `scan-secrets.sh` + `--rule=zero-transcript` + `scan-secrets-docs.sh --all-md` | W-B4-30.5 |
| 43 | `--rule=source` 退 0（扫描面缺失退 2 视为失败） | W-B4-7.2、W-B4-30.6 |
| 44 | `--list --ignored` = 9 + `check-g2-claims.sh` | W-B4-30.6 |
| 45 | `check-mac-scope.sh --rule=claims` + 规格"不适用（无设备）" | W-B4-30.7 |
| 46 | `replay_interface_is_io_free` + `pub fn` 计数 =4（单文件） | W-B4-6.4、W-B4-28.1 |
| 47 | `gate_record_roundtrips_with_closed_fields`、`skipped_stages_only_g2_g3` + `--rule=record` | W-B4-4.2、4.3、W-B4-31.1 |
| 48 | `replay_200_cases_is_byte_identical` | W-B4-28.3、28.4 |
| 49 | `test ! -d crates/application/tests/replay` + 符号 0 命中 | W-B4-28.5 |
| 50 | `--rule=doc-plan` 退 0 | W-B4-31.2 |

### 0.6.1 本阶段 REQ-B4P0-01~24 → 子任务 → AC 映射

| REQ | 本阶段子任务 | 对位 AC | 关键判据（命令 / 单测） |
|---|---|---|---|
| REQ-B4P0-01 | 2.1、2.2、2.4；8.3 | AC-B4-43/47 | `test -f` + `test ! -d`；三规则退 0 由 8.3 复跑 |
| REQ-B4P0-02 | 2.1、3.1~3.3、4.1~4.5、5.1~5.3 | AC-B4-43/47 | `cargo test -p meeting-copilot-domain` 退 0；31 项可解析 |
| REQ-B4P0-03 | 2.2 | AC-B4-48/49 | 12 项齐备；`GateInputFixture` 23 字段覆盖 8 类状态；无 runner |
| REQ-B4P0-04 | 2.3 | AC-B4-43 | `git diff --numstat` → 删除 0 / 新增 ≤3 |
| REQ-B4P0-05 | 3.1 | AC-B4-11/24 | `policy_validate_rejects_out_of_range_and_fixes_query_limits` |
| REQ-B4P0-06 | 3.2 | AC-B4-08 | `policy_rejects_skip_g1_g4_g5`；`grep -c "pub skip_g"` = 0 |
| REQ-B4P0-07 | 3.3 | AC-B4-47 | `policy_keeps_unknown_fields_roundtrip` |
| REQ-B4P0-08 | 4.1 | AC-B4-47 | `per_stage_serializes_exactly_five_entries` |
| REQ-B4P0-09 | 4.2 | AC-B4-47 | `gate_record_roundtrips_with_closed_fields`；`--rule=record` |
| REQ-B4P0-10 | 4.3、6.6 | AC-B4-47 | `skipped_stages_only_g2_g3`；`validate_record` → `IllegalSkipStage` |
| REQ-B4P0-11 | 4.5、6.6 | AC-B4-47 | `record_rejects_empty_refs`；双空 → `RefsEmpty`，不 panic |
| REQ-B4P0-12 | 4.4 | AC-B4-34/29/42 | `outbound_intent_and_audit_event_have_no_content_fields` |
| REQ-B4P0-13 | 5.1 | AC-B4-22/23/24 | `budget_rejects_corrupted_snapshot_and_saturates_remaining` |
| REQ-B4P0-14 | 5.2、5.3 | AC-B4-08/13/24/35/42 | `gate_counter_all_is_32_in_spec_order`、`gate_counter_names_match_spec_field_names` |
| REQ-B4P0-15 | 5.4a、6.6 | AC-B4-08/13/16/17/20/25/35/36 | `bump_by_reason_matches_static_table`（domain + application 同名）、`duplicate_splits_by_stage`、`no_provider_is_counted_once` |
| REQ-B4P0-16 | 6.1~6.5、6.7 | AC-B4-46 | `pub fn` 4 / 四名 4 / 单文件 1；`cargo build -p application` 退 0 |
| REQ-B4P0-17 | 6.4、6.6、6.7 | AC-B4-43/46 | 占位语义单测；三类宏 0；`mod.rs` 不导出回放面三类符号 |
| REQ-B4P0-18 | 7.1~7.6；8.3 | AC-B4-07/22/33/34/42/43/47/50 | 6 条规则退 0（`targets` 退 1，D-B4-D07） |
| REQ-B4P0-19 | 7.2、7.5 | AC-B4-43/33 | `SCAN-ABSENT` 只登记；必需面缺失 `SCAN-MISSING` + 退 2 |
| REQ-B4P0-20 | 7.7 | AC-B4-43/47 | `--self-test` 退 1 且 `git status --porcelain` 行数 0 |
| REQ-B4P0-21 | 8.2 | AC-B4-08/22/47 | 三条硬判据串联 `exit=0` |
| REQ-B4P0-22 | 8.4 | AC-B4-37/40/41 | 两文件零 diff；6 常量各 1；`Cargo.*` 零 diff |
| REQ-B4P0-23 | 8.4、8.5 | AC-B4-39/43/44/45 | fmt/clippy/layering/secrets/b3/deny 全绿；ignored = 9 |
| REQ-B4P0-24 | 8.1、8.3、8.6 | AC-B4-50 | `b4-status.md` 6 段；`--rule=doc-plan` 退 0 |

### 0.6.2 P4 增量 AC 覆盖矩阵（AC-B4-48、51~57 → 任务）

| AC | 主判据（用例名 / 命令） | 任务 | 当前状态 |
|---|---|---|---|
| AC-B4-48 | `replay_200_cases_is_byte_identical`、`replay_200_cases_cover_all_reject_reasons`；`total=200`/`mismatched=0`/`seed=7` | W-B4-41.2~41.5 | pending（U-01） |
| AC-B4-51 | `keyword_candidates_cover_five_kinds`、`keyword_candidate_keeps_first_seen_only`、`unanswered_question_is_not_a_keyword_card` | W-B4-35、36、38.2~38.4 | pending（U-02） |
| AC-B4-52 | `keyword_card_requires_gate_allow`、`deny_never_emits_clickable_card`；`gate_decision_ref`/`click_state`/`card_drafts` | W-B4-35.3/35.4、37.1、38.5/38.6 | pending（U-02） |
| AC-B4-53 | `keyword_card_has_closed_fields`、`keyword_card_reference_is_id_only`；字段集 =13、`Disabled` 原因 ⊆ 6 | W-B4-34.2~34.7、39.1/39.2 | pending（U-02） |
| AC-B4-54 | `clicking_keyword_card_is_local_only`、`continue_search_reenters_gate` | W-B4-37.1、39.3/39.4 | pending（U-02） |
| AC-B4-55 | `click_permission_is_fail_closed`、`keyword_card_hidden_under_screen_share` | W-B4-37.2、40.1/40.2 | pending（U-02） |
| AC-B4-56 | `keyword_card_zero_transcript_boundary`；`--rule=zero-transcript` 退 0；禁止字段命中 0 | W-B4-34.8、40.3、41.6 | pending（U-02） |
| AC-B4-57 | `keyword_card_ownership_is_outside_gate`；`--rule=outbound` 退 0；禁词 0 命中；`tests/replay/` 不存在 | W-B4-40.4、40.5、42.1 | pending（U-02） |

> 覆盖口径：AC-B4-51~57 各至少 1 个用例，13 个用例名与 spec §7.11 / design §4.7.1 **逐字一致**；AC-B4-48 两个用例名与 spec §7.7 **逐字一致**。
> **U-01~U-08 归属**：U-01 → W-B4-41（本轮闭合）；U-02 → W-B4-34~40（本轮闭合）；U-03（38 合成）/U-04（39 对齐）/U-05（51 UI）→ **P4 不承接**，只保留 design §4.5.2 的签名预留，实体文件由各自阶段创建；U-06（13.3 本体）→ 保持 `tests/replay/` 不存在；U-07（41.5/42/43/52）→ 不承接；U-08（真实端点/真机）→ 保持 9 个 `#[ignore]`，不宣称 G4。

### 0.6.3 P5 增量 AC 覆盖矩阵（AC-B4-58~64 → 任务，7/7）

| AC | 主判据（用例名 / 命令） | 任务 | 实现前状态 | 实现后状态 |
|---|---|---|---|---|
| AC-B4-58 | `citation_input_is_readonly_and_closed`；`grep -c '^pub fn ' api.rs` = 4；`--rule=record` 退 0；`kw_fields KeywordCardDraft`=13、`kw_fields ReferenceView`=5 | W-B4-43.3、46.2、48.3 | pending | passed |
| AC-B4-59 | `citation_alignment_is_three_state_and_fail_closed`、`unknown_projects_to_unverified_only`；`CitationAlignment::ALL.len()` = 3；`basis` 11/11 | W-B4-43.1/43.5、44.2~44.4、45.1、46.3、46.7 | pending | passed |
| AC-B4-60 | `citation_passed_enables_click_only_with_other_facts`、`citation_failed_still_displayed_but_disabled`、`citation_unknown_is_conservative`；`click_state`/`summary`/`card_drafts` | W-B4-45.1/45.2、46.4 | pending | passed |
| AC-B4-61 | `citation_result_is_id_only`；`--rule=zero-transcript` 退 0；39 契约禁词计数 = 0 | W-B4-43.2/43.3/43.4、46.5、47.2 | pending | passed |
| AC-B4-62 | `citation_check_is_offline_and_deterministic`、`citation_ignores_reference_order`；`--rule=source` 退 0（扫描面含 `citation/**` 与 `citation.rs`）；缺面退 2 | W-B4-44.1/44.2、46.6、47.1 | pending | passed |
| AC-B4-63 | `citation_ownership_is_outside_b4_and_ui`；6 禁词命中 0；`tests/replay/` 不存在；`snapshot().len()` = 32 | W-B4-46.5、48.3 | pending | passed |
| AC-B4-64 | `NEW_TARGETS` token = 21 且含 `citation_align`；`grep -c '^| AC-B4-' spec.md` = 64；`U-04.*规格已定` ≥ 1；`citation_align` 12 passed | W-B4-46.1~46.7、47.3/47.4、48.1/48.2/48.4 | pending | passed |

> 覆盖口径：AC-B4-58~64 七条**全部**由 P5 承接，用例名与 `spec.md` §7.12 / `design.md` §5.8.1 **逐字一致**，共 12 个用例（含 2 个辅助）。**不判定**真实引用可访问性、真实语义对齐质量、51 交互质量（D-B4-05），也不宣称 G3/G4/真机。
> **编号纪律**：AC-B4-01~57 不重排、不改语义；P5 只新增 58~64 的实现，既有 57 条的判据与状态一律不动。
> **U-04 归属**：U-04（39 引用对齐校验）→ 由 W-B4-43~48 **本轮闭合**；U-03（38 合成）/U-05（51 UI）→ **P5 不承接、保持 open**；U-06 → `tests/replay/` 仍不存在；U-07/U-08 → 不承接，9 个 `#[ignore]` 不变。

### 0.6.4 P7 增量 AC 覆盖矩阵（AC-B4-75~84 → 任务，10/10）

| AC | 主判据（用例名 / 命令） | 任务 | 实现前 | 实现后 |
|---|---|---|---|---|
| AC-B4-75 | `keyword_card_view_input_is_closed`、`keyword_card_view_from_composed_is_not_clickable`；`kw_fields` 10/4/5/6/2/2；`keyword_card/` 顶层类型=8、顶层 `pub fn`=3；`api.rs`=4；`--rule=record` 退 0 | W-B4-57.3、58.1、58.2、63.1 | pending | passed |
| AC-B4-76 | `keyword_card_view_hides_body_under_screen_share`、`keyword_card_view_hides_body_under_emergency_mask`；`body=null`、`badge_count≥1` | W-B4-58.3、63.2 | pending | passed |
| AC-B4-77 | `keyword_card_view_click_state_is_downgrade_only`、`keyword_card_view_never_upgrades_disabled`；`ClickDisabledReason::ALL.len()`=6 | W-B4-59.1、59.3、63.3 | pending | passed |
| AC-B4-78 | `keyword_card_view_detail_is_local_only`、`keyword_card_view_detail_has_no_outbound_side_effect`；禁词 0；`ClickOutcome` XOR | W-B4-60.1、60.2、63.4 | pending | passed |
| AC-B4-79 | `keyword_card_view_withdrawn_is_not_revived`、`keyword_card_view_stale_or_cross_session_is_disabled`、`keyword_card_view_failed_alignment_stays_visible_but_disabled`；100 次一致 | W-B4-59.2、58.3、63.4 | pending | passed |
| AC-B4-80 | `keyword_card_view_conclusion_is_none_without_passed`、`keyword_card_view_conclusion_preserved_for_passed`；`ConclusionView` 变体=3 | W-B4-58.5、63.5 | pending | passed |
| AC-B4-81 | `keyword_card_view_zero_transcript_boundary`、`keyword_card_view_debug_is_whitelisted`；`--rule=zero-transcript` 退 0 | W-B4-61.1、63.5、64.2 | pending | passed |
| AC-B4-82 | `keyword_card_view_is_offline_and_platform_free`、`keyword_card_view_ignores_reference_order`；`--rule=source` 退 0；`cargo tree` 禁词 0 | W-B4-58.4、61.2、63.6、64.1 | pending | passed |
| AC-B4-83 | `keyword_card_view_ownership_is_outside_gate_and_cards`、`keyword_card_view_public_types_are_readonly`；7 禁词 0；`git diff` 三类零 diff；`tests/replay/` 不存在 | W-B4-57.4、61.2、63.6、64.5 | pending | passed |
| AC-B4-84 | `keyword_card_view` target 全绿；`NEW_TARGETS`=23 且含 `keyword_card_view`；AC 汇总=84（既有口径已达 94，本行只断言"≥84 且 75~84 全在"）；U-05 保留"规格已定"并追加 P7 关闭登记；`check-b4-gates.sh` 内 `keyword_card_view` 命中 0 | W-B4-64.3、64.4、64.5 | pending | passed |

> 覆盖口径：AC-B4-75~84 十条**全部**由 P7 承接；19 个用例名与 `spec.md` §7.14 / `design.md` §7.9.1 / `requirements.md` §13.5 **逐字一致**。**不判定**真实像素渲染、真实焦点、真实三形态/投屏 API、产品级交互质量（D-B4-08）。
> **U-05 归属（P7 部分）**：U-05 的"点击 UI"一半由 P7 关闭，"设置向导"一半由 P8 关闭；**U-05 整行直到 P8 完成才可标 `closed`**，P7 只登记"点击 UI 已落盘（见 §18），设置向导仍待实现（见 §17）"。

### 0.6.5 P8 增量 AC 覆盖矩阵（AC-B4-85~94 → 任务，10/10）

| AC | 主判据（用例名 / 命令） | 任务 | 实现前 | 实现后 |
|---|---|---|---|---|
| AC-B4-85 | `settings_view_is_closed_and_single_step`、`settings_defaults_need_only_api_key`；`kw_fields` 8/6/5/4；顶层类型=18、`pub fn`=7；`SECTION_ORDER.len()`=4 | W-B4-65.1~65.5、67.1、67.3、71.2 | pending | passed |
| AC-B4-86 | `settings_api_key_inline_errors_are_lenient`、`settings_errors_only_after_submit`；15/16 边界 | W-B4-66.3、71.3 | pending | passed |
| AC-B4-87 | `settings_api_key_is_masked_without_payload`、`settings_debug_and_errors_never_leak_key`；1/16/64/256 掩码逐字节相同 | W-B4-66.1、66.2、71.3 | pending | passed |
| AC-B4-88 | `settings_persists_intent_only_and_clears_with_confirmation`、`settings_repo_has_no_plaintext_key`；`std::fs`/`keyring`/`card_store`/`Sqlite` 命中 0 | W-B4-68.1、68.2、71.4 | pending | passed |
| AC-B4-89 | `settings_model_preset_or_custom_is_closed`、`settings_model_name_is_not_inlined`；模型名正则命中 0 | W-B4-67.2、71.4、72.2 | pending | passed |
| AC-B4-90 | `settings_endpoint_is_allowlist_index_only`、`settings_endpoint_never_expands_allowlist`；地址/域名/白名单常量名 0；`infrastructure/src/llm` 零 diff | W-B4-67.3、71.4、72.4 | pending | passed |
| AC-B4-91 | `settings_probe_state_machine_is_single_flight`、`settings_probe_cancel_and_cooldown`；100 次一致；`--rule=source` 退 0 | W-B4-69.1~69.3、71.5、72.1 | pending | passed |
| AC-B4-92 | `settings_failure_messages_are_humanized`、`settings_failure_keeps_user_input`；文案表 8 且一一对应 | W-B4-70.1、70.2、71.5 | pending | passed |
| AC-B4-93 | `settings_boundary_is_offline_and_credential_safe`、`settings_debug_is_whitelisted`；`--rule=zero-transcript` 退 0；`cargo tree` 禁词 0 | W-B4-61.2（口径复用）、W-B4-71.6、72.2、72.4 | pending | passed |
| AC-B4-94 | `llm_settings_view` target 全绿；`NEW_TARGETS`=24 且含 `llm_settings_view`；AC 汇总=94；U-05 含"设置向导"与"规格已定"并登记 D-B4-P75~P84；`rule_targets` 函数体内 0 命中；13 项封闭面与 `api.rs` 4 不变；`Cargo.toml`/`Cargo.lock` 零 diff | W-B4-72.3~72.5 | pending | passed |

> 覆盖口径：AC-B4-85~94 十条**全部**由 P8 承接；18 个用例名与 `spec.md` §7.15 / `design.md` §7.9.1 / `requirements.md` §14.5 **逐字一致**。**不判定**真实 UI 渲染、真实焦点/键盘可达性、真实操作系统安全存储写入、真实网关连通性与错误码映射准确性（D-B4-09）。
> **U-05 收口**：P8 完成后 U-05 整行标 `closed（W-B4-57~72）`，同时**保留** §10.1/§16/§17 的"规格已定"原文（AC-94 的 `grep -c 'U-05.*设置向导'` 与 AC-84 的 `grep -c 'U-05.*规格已定'` 都依赖它）。

### 0.7 P0 出口 3 条硬判据（缺一即 P0 未完成）

1. `"$CARGO" test -p meeting-copilot-domain` → 退出 **0**（31 项契约、23 项码表、`GateRecord` 14 字段 JSON 往返、`GateBudget` 7 字段、`GateCounters` 32 项、`counter_at` 静态表全绿）。
2. `bash scripts/check-b4-gates.sh --rule=record` → 退出 **0**（`GateRecord` 14 业务字段集合相等、`GateRejectReason` 23 变体、`skipped_stages ⊆ {G2,G3}`）。
3. `bash scripts/check-b4-gates.sh --rule=budget` → 退出 **0**（`GateBudget` 7 字段齐备、`gate/` 与 `gate*.rs` 中 `LLM_LIVE_CALL_CAP`/`LLM_`/`std::env` 命中 0）。

> **旁证五条（不替代上述三条，口径已按 D-B4-D07 修正）**：`--rule=source` / `--rule=outbound` / `--rule=zero-transcript` / `--rule=doc-plan` 退 **0**；`--rule=targets` 在 P0 退 **1** 且输出恰 6 行 `TARGETS-MISSING: gate_*`（6 个 target 在 P1~P3 逐个转绿，P3 收口退 0）。CI 不得把 P0 的 `targets` 红灯视为缺陷。另有零 diff 取证（`store.rs`/`deduplicator.rs`、`Cargo.toml`/`Cargo.lock`）与 `cargo fmt --check` / `cargo clippy -D warnings` 退 0。

### 0.8 两个顺序风险（design §2.7 R-07/R-08，各自有专章）

| 风险 | 错误做法 | 机器判据 | 专章 |
|---|---|---|---|
| **R-07 先脱敏再截断** | 把 81/201 字原样交给脱敏器，靠 `SanitizedQuery` 内部报错 | 脱敏替身收到的入参长度必须 == 80（自动）/200（按需），且 `truncated=true`；超长路径替身调用次数按"先截断"口径计算 | **W-B4-25** |
| **R-08 先放行再写审计** | 先产出 `OutboundIntent`，失败再补审计 | 审计替身 `append` 回调时下游 `submitted==0`；`append` 返回 `Err` 时 `decision=Deny{AuditUnavailable}` 且 `outbound_attempts=0` | **W-B4-26** |

### 0.9 最易被忽略的风险与处置

1. **扫描面缺失导致门禁永久退 2（design R-01/R-03 + D-B4-D02）**：`crates/domain/src/gap/` 与 `crates/application/src/proactive/` **本轮都不存在**。若 `--rule=source` / `--rule=outbound` 照 spec 字面"任一扫描面缺失退 2"，两条规则将永久红。处置见 W-B4-7.2/7.5：扫描面清单以"本轮实际存在的面"为准，缺失面在脚本内显式登记并打印 `SCAN-ABSENT: <path>`，**只登记不判失败**；同时在 `b4-status.md` 登记 D-B4-D02。
2. **扁平落位（R-01）**：`model/gate.rs`、`model/gate_replay.rs` 不得建成子目录，否则 `gate*.rs` 扫描面缺失退 2 → AC-43/47 全挂。见 W-B4-2.1/2.2/2.4。
3. **4 个 `pub fn` 分散（R-02）**：`grep -c` 多文件会输出多行 `file:count`，断言不稳。见 W-B4-6.4。
4. **`--rule=record` 的字段抽取被测试代码污染**：抽取前必须用 `strip_tests()` 截断 `#[cfg(test)]`，且"集合相等"而非 `≥14`；否则 P1 追加类型或测试常量会让规则永久红或静默放行。见 W-B4-7.1/7.4。
5. **`--rule=targets` 在 P0 必然红（D-B4-D07）**：6 个 `gate_*` target 在 P1~P3 才创建；P0 出口判据是"退 1 且 6 项可枚举"，不得预建空 target 制造假绿。见 W-B4-7.7、§0.7。
6. **`--self-test` 污染工作区**：反例必须在 `mktemp -d` 内构造并 `trap` 清理；禁止临时改真实源码（含白名单加 `G4`、注入鉴权头）。见 W-B4-7.7、W-B4-31.1。

### 0.10 P9 交付后的实况口径（2026-11-22 复核）

#### 0.10.1 P0~P9 完成情况与 checkbox 口径

- **完成情况（实测）**：P0~P9 全部交付；AC-B4-01~104 中 **103 条 `passed`、1 条 `not-applicable`**（AC-B4-45，macOS 无设备），**`pending`=0**；`ci_deny_repair` 批次追加的 AC-B4-105~110 本地全部 `passed`（AC-B4-109 的**远端 CI 取证**尚待 push 后补，见 §0.10.3）。
- **checkbox 不作为进度来源**：本文件 350 个子任务（P0~P9 的 339 项 + P10 的 5 项 + P11 的 6 项）仍写作 `- [ ]`（计划态），**不代表未完成**。任务三态以 `docs/progress/b4-status.md` 的阶段快照为准：P0 §1~§6、P1 §7、P2 §8、P3 §9、P4 §11、P5 §13、P6 §15、P7 §18、P8 §19、P9 §23、CI §22、P10 §25、P11 §26。禁止以 `grep -c '^- \[ \]'` 判定缺口。

#### 0.10.2 P9 提交后判据复跑口径（X-29）

P9 已作为单提交 `1b0dbf7` 落库（5 个文件：`gate/replay.rs`、`gate/mod.rs`、`tests/gate_replay_driver.rs`、`scripts/test-name-inventory.sh`、`docs/progress/b4-status.md`）。因此 P9 章节内所有 `git diff --quiet HEAD -- …` / `git diff --numstat HEAD -- …` 判据**在提交后恒为 0/`0 0`，不再有判别力**。复跑时统一前移一层基线：

```bash
P9BASE=1b0dbf7^
git diff --numstat "$P9BASE" 1b0dbf7 -- crates/application/src/gate/mod.rs        # 期望 `1 0`
git diff --numstat "$P9BASE" 1b0dbf7 -- scripts/test-name-inventory.sh            # 期望 `1 1`（单行替换）
git diff --stat "$P9BASE" 1b0dbf7                                                 # 期望 5 files changed
```

| 判据 | 提交后复跑命令 | 实测 |
|---|---|---|
| `gate/mod.rs` 仅 +1 行 | `git diff --numstat 1b0dbf7^ 1b0dbf7 -- …/gate/mod.rs` | `1 0` |
| token 单行替换 | `git diff --numstat 1b0dbf7^ 1b0dbf7 -- scripts/test-name-inventory.sh` | `1 1` |
| 提交面 5 文件 | `git diff --stat 1b0dbf7^ 1b0dbf7` | 5 files，+1380/−4 |
| 22 用例 | `cargo test -p meeting-copilot-application --test gate_replay_driver` | 22 passed / 0 failed |
| 契约计数 | `replay.rs` 顶层 `pub`=7、顶层 `pub fn`=0；`ReplayCase`4/`ReplayRun`3/`ReplayFailure`2/5 码/3 错误 | 7/0/4/3/2/5/3 |
| 全量 | `cargo test --workspace`；`--list` | 825 passed / 0 failed / 9 ignored；834 |
| `tests/replay/` | `test ! -d crates/application/tests/replay` | 不存在 |

#### 0.10.3 仍未完成项与"需用户提供条件"（P10 之后）

| 编号 | 项 | 现状（实测） | 缺失证据 | 阻塞原因 / 所需条件 |
|---|---|---|---|---|
| U-07-41.5 | 负反馈加权（`crates/application/src/proactive/feedback.rs`） | 未做；`spec.md` §7.5-6、N-04 明确"不做 41.5"，仅有 `docs/sdd/tasks.md` 3 行粗条目 | 无 b4 级契约（字段/枚举/AC/零转录口径） | **需用户批准规格增量**：是否允许改写 G2 `score`/G3 抑制、"无用×2 降权 0.5"的整数口径、是否落审计 |
| U-07-42 | 缓存 24h / 断网 outbox（`query_cache.rs`、`outbox.rs`） | 未做；B4 明确不实现缓存存储 | 无持久化介质与表结构；"重启不丢"无验收口径 | **需用户确认存储介质**（SQLCipher/表结构/加密与删除语义）与 outbox 重放顺序 |
| U-07-43 | 按需补全实体（`on_demand*.rs`） | 未做；B4 只定义"按需补全在 G3 `Suppressed` 时可 Allow"的契约 | 无真实检索源；8s 首卡 p95 无法离线判定 | **需用户提供检索源**（端点/白名单/凭据）与 ≥200 次时延样本 |
| U-07-52 | 审计事件接入与哈希链存储（`audit_hooks.rs`、`write_path.rs`、`trace_persist.rs`、`retention.rs`） | 未做；B4 仅有 `GateAuditEvent` 7 字段与 fail-closed 契约 | 无落库介质、无哈希链算法/归档格式 | **需用户确认落库介质**与 180 天归档校验口径（`52.4` 明确"策略不可改"） |
| U-08 | 真实端点 / 真机 | 未判定；9 个 `#[ignore]` 保持 | 无真实 LLM 端点/额度、无真机与会议平台样本 | **需用户提供**：`LLM_*` 端点与凭据、Windows 真机、投屏/紧急遮蔽可复现样本 |
| AC-B4-109 | CI `compliance` 远端取证 | 本地 `cargo deny check licenses bans sources advisories` 退 0 | 无远端 run 结论 | **需用户授权 push** 并触发 CI（`gh`/网页均可） |
| P10-W-B4-82 | README/进度实况收口 | 见 P10 | — | **可离线直接执行**（本批唯一无需外部条件的任务） |

> 结论：**本仓库当前无剩余可离线推进的 Rust 功能编码项**。U-07 四项在 b4_gate 规格中均为显式"不做"且无契约级规格，直接编码会违反"不猜测推进"；U-08 与 AC-B4-109 依赖真实外部条件。

#### 0.10.4 本轮判据补正登记（X-29 ~ X-33）

| 编号 | 原判据/表述 | 实况 | 补正（最小改动） |
|---|---|---|---|
| **X-29** | P9 全章 `git diff … HEAD` 类判据 | P9 已提交，相对 HEAD 恒为 0 | 新增 §0.10.2，统一改用 `1b0dbf7^` 基线；判据语义不变 |
| **X-30** | W-B4-81.1"`git diff --stat HEAD` 仅 **1 行**变更" | 单行内容替换 → `--numstat` = `1 1` | 改为 `1 1`（并在证据块补断言） |
| **X-31** | §0.1 未说明 `--diff` 的环境依赖 | 未前置 PATH 时 `ui-bridge` 退 101 / `failed=4` | §0.1 增补 PATH 前置说明（实测：前置后退 0） |
| **X-32** | 全文 `- [ ]` 无状态说明（补正后共 344 项） | P0~P9 已交付，易被误读为数百项未完成 | 新增 §0.10.1：checkbox 为计划态，状态以 `b4-status.md` 快照为准 |
| **X-33** | P9 附表"P9 子任务 30" | 实测 30（73.0~73.4、74.1~74.3、75.1~75.3、76.1~76.3、77.1~77.3、78.1~78.3、79.1~79.3、80.1~80.3、81.1~81.4） | 不变，仅补明清单以免与"9 主任务"混淆 |

#### 0.10.5 P11 判据补正登记（X-34 ~ X-35）

| 编号 | 原判据/表述 | 实况（cargo 1.100.0 实测） | 补正（最小改动） |
|---|---|---|---|
| **X-34** | `design.md` §8.2.2 的"`u32` 转换纪律"与 W-B4-73.2 / W-B4-76.2 的 `map_or(u32::MAX, \|n\| n)` 写法 | `clippy::map_or_identity` 在 `replay.rs:81,82` 各报 1 处，`--all-targets -D warnings` 因此退 1 | 等价改写 `unwrap_or(u32::MAX)`（`map_or` 的恒等闭包与 `unwrap_or` 对 `Ok/Err` 两支逐一等价）；`u32::MAX` 溢出哨兵与 `saturating_sub` 口径不变。P9 判据 `grep -cE 'panic!\(\|unwrap\(\)\|expect\(' "$RP"`=0 **不误伤**：`unwrap_or(` 不含 `unwrap()`，`--rule=source` 禁词表也无 `unwrap_or`。**原文两行不改**，按本表复跑 |
| **X-35** | W-B4-73.1/73.2 未规定**测试目标**的 clippy 口径；`ReplayDriver` 的 `Default` 派生仅由 17 处 `ReplayDriver::default()` 使用 | 临时 allow 后 `tests/gate_replay_driver.rs` 暴露 22 处：`dead_code` 1（`Arena.reference` 只写不读）/ `field_reassign_with_default` 3 / `default_constructed_unit_structs` 17 / `unnecessary_min_or_max` 1 | ① 删 `Arena.reference` 字段与初始化行（局部 `reference` 仍被 `fragment(...)` 与 `stable_prefix_ms.insert(...)` 读取，**不产生 `unused_variables`**）；② 3 处改结构体更新语法；③ 16 处 `ReplayDriver::default()` → `ReplayDriver`，第 17 处（`replay_driver_types_are_closed`）留 `let driver: ReplayDriver = Default::default();` 作 `Default` 可用性证据（该写法**不触发** lint，lint 只匹配 `Type::default()` 调用路径）；④ `0.max(len)` → `len`（`usize` 下界恒等，`mismatched as usize == mismatch_cases.len()` 断言语义不变）。**不得以 `#[allow]`/`#[expect]` 规避** |
| **P11 基线指针** | P10 收口提交 | `52c4bd3`（`docs(status): 校准 README 实况并登记 P10 收口（§25）`，2 文件） | P11 全部 `git diff … HEAD` 类判据的基线 = **`52c4bd3`**；语义与阈值不变。P11 提交面**恰 3 文件**，`design.md`/`tasks.md` 的登记不进入代码提交 |

> 结论：X-34 是**纯 lint 等价改写**（2 行），X-35 是**测试目标告警收敛 + `Default` 证据保留**（约 25 行）。两者都不改变任何 AC、用例名、断言数值、target 数量与 `#[ignore]` 数量；`feature-coverage (C2)` 属于 `needs: rust` 的级联失败，不改 CI。

---

## P0 · 码表与领域契约（不可压缩）

### W-B4-1 C-01 码表与三套码迁移

> 前置：无（HEAD `63caa17`）。本任务是 P1 全部原因码与计数映射的唯一依赖。

- [ ] **W-B4-1.1 定义 23 项封闭 `GateRejectReason`** · `crates/domain/src/model/gate.rs`
  - 追溯：AC-B4-08、AC-B4-47 · design §2.2.3（唯一映射表 23 行）、§2.3.2-A
  - DoD：枚举恰 23 变体，顺序 = design §2.2.3 表 #1..#23；`serde(rename_all="snake_case")`；`pub const ALL: [Self; 23]` 长度与顺序一致；未知字符串反序列化返回 `Err`
  - 证据：`"$CARGO" test -p meeting-copilot-domain gate_reject_reason_all_is_23_in_table_order` 退 0；`"$CARGO" test -p meeting-copilot-domain reject_reason_rejects_unknown_string` 退 0

- [ ] **W-B4-1.2 `to_deny_code()` 12→11 唯一投影** · `crates/domain/src/model/gate.rs`
  - 追溯：AC-B4-08、AC-B4-40 · design §2.2.3（覆盖矩阵 11 项 / 12 来源）
  - DoD：`to_deny_code()` 返回 `Some` 的项数 **12**、`None` 的项数 **11**；`DenyCode::ALL` 11 项每项至少 1 个来源；`RateLimited` 来源恰 2 项（`quota_exhausted`、`budget_exhausted`）；`DenyCode` 枚举与 `ALL` **零改**
  - 证据：`"$CARGO" test -p meeting-copilot-domain deny_code_mapping_covers_11_with_rate_limited_twice` 退 0；`"$CARGO" test -p meeting-copilot-domain deny_code_mapping_has_exactly_12_some` 退 0

- [ ] **W-B4-1.3 `ui_message_key()` / `visibility()` 23 项全覆盖** · `crates/domain/src/model/gate.rs`
  - 追溯：AC-B4-08、AC-B4-34 · design §2.2.3（UI 文案表 23 条）
  - DoD：`ui_message_key()` 恒为 `gate.reason.<serde 值>`，23 项全有键、无重复；`visibility()` 为 `Public` 12 项 / `Internal` 11 项，集合与 design 表"可见性"列逐项一致
  - 证据：`"$CARGO" test -p meeting-copilot-domain ui_message_key_is_23_and_matches_serde_value` 退 0；`"$CARGO" test -p meeting-copilot-domain visibility_is_12_public_11_internal` 退 0

- [ ] **W-B4-1.4 内部码不进入 `DenyCode`、不复用泛化文案** · `crates/domain/src/model/gate.rs`
  - 追溯：AC-B4-08、AC-B4-40 · design §2.2.3（"为什么不让三套码并存"）
  - DoD：`consent_missing`/`silent_period`/`screen_share_protection`/`sensitive_term`/`outbound_not_granted`/`sanitize_failed`/`provider_not_allowed`/`domain_blocked`/`policy_unavailable`/`retention_expired` 复用既有 `DenyCode::user_message`；`quota_exhausted`/`budget_exhausted` **不**复用 `RateLimited` 泛化文案（各有独立键）
  - 证据：`"$CARGO" test -p meeting-copilot-domain quota_and_budget_do_not_reuse_rate_limited_text` 退 0

- [ ] **W-B4-1.5 design 12 码降级为历史别名并登记** · `docs/progress/b4-status.md`
  - 追溯：AC-B4-08、AC-B4-50 · design §2.2.3、§2.7（D-B4-D04）、R-05
  - DoD：`b4-status.md` 登记 D-B4-D04（12 码降级为历史别名、3 处改名 + 1 处改 `LowConfidence`）；新代码中旧名 0 命中（`NotAuthorized`/`QuietPeriod`/`ScreenShareMode` 不作为变体出现）
  - 证据：`! grep -rqE "NotAuthorized|QuietPeriod|ScreenShareMode" crates/domain/src/model/gate.rs crates/application/src/gate` 判定通过（期望 0 命中）；`grep -c "D-B4-D04" docs/progress/b4-status.md` ≥1

### W-B4-2 扁平落位与 domain 契约骨架

> 前置：W-B4-1 已交付（HEAD `ade9111`，码表 23 项 + 既有 7 单测零改）。本任务只落**类型骨架**（字段/derive/常量），不写判定行为；行为方法分别由 W-B4-3/4/5 追加。
> 顺序：2.1（S1 编译）→ 2.2 → 2.3（S2 domain 测试退 0）→ 2.4（扁平落位静态断言）。

- [ ] **W-B4-2.1 `gate.rs` 追加 29 项类型（既有 2 项零改）** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-01/02 · AC-B4-43/47 · design §3.3.1（31 项清单）、§3.3.2（10 条实现约定）
  - 改动内容：在既有 `Visibility`、`GateRejectReason` **之后**追加 29 项：`GateStage`、`StageResult`、`GapRule`、`RelevanceScore`、`ThresholdProfile`、`FrequencyProfile`、`GatePolicy`、`GatePolicyError`、`AuthorizationStatus`、`GateContentType`、`QueryLimit`、`GateAuthorization`、`NetworkState`、`GateBudget`、`GateCounter`、`GateCounters`、`StageTrace`、`PerStageResults`、`GateDecision`、`DecisionKind`、`GateRecord`、`OutboundIntent`、`GateAuditEvent`、`AuditResult`、`LlmCapabilityView`、`ProviderView`、`EntityKey`、`ScoredCandidate`、`ClusterSource`（= 既有 2 + 新增 29 = **31**）。
  - 关键约束：`GateBudget.topic_window_used: BTreeMap<u32,u8>`、`entity_last_seen_ms: BTreeMap<EntityKey,u64>`（禁 `HashMap`）；`PerStageResults(pub [StageTrace;5])`、`GateCounters([u64;32])`（禁 `Vec`）；新增行零 `f32`/`f64`；不新建 `ReasonVisibility`（沿用既有 `Visibility`，requirements §8.2-1）；不新增 `pub skip_g*` 字段；`GatePolicy`/`ThresholdProfile`/`FrequencyProfile` 本步**不加** `derive(Default)`（默认值由 3.1 手写，避免 cap=0 等静默错值）。
  - DoD：`test -f .../gate.rs && test ! -d .../gate` 退 0；`grep -cE "^pub (struct|enum) "` ≥ 31；`grep -c HashMap` = 0；`grep -c "pub skip_g"` = 0；`"$CARGO" test -p meeting-copilot-domain` 退 0（既有 7 例不回归）
  - 证据：`test -f crates/domain/src/model/gate.rs && test ! -d crates/domain/src/model/gate; echo "exit=$?"` → `exit=0`；`"$CARGO" test -p meeting-copilot-domain; echo "exit=$?"` → `exit=0`
  - 证据产物：命令与退出码抄录到 `docs/progress/b4-status.md` §2（8.1 建立骨架后回填）

- [ ] **W-B4-2.2 `gate_replay.rs` 12 项回放夹具（扁平文件、零 runner）** · `crates/domain/src/model/gate_replay.rs`
  - 追溯：REQ-B4P0-03 · AC-B4-48/49 · design §3.4（12 项表 + 8 行覆盖自检）
  - 改动内容：定义 `ClockFixture`、`FragmentFixture`、`InsightFixture`、`BudgetFixture`、`AuthorizationFixture`、`RelevanceFixture`、`SuppressionFixture`、`CandidateSeedFixture`、`GateInputFixture`、`ExpectedDecision`、`GateReplayCase`、`ReplayReport`。`GateInputFixture` 按 design §3.4 表落 **23 字段**（`now_ms`、`clock`、`trigger`、`session_state`、`silent_period`、`screen_share`、`consent_confirmed`、`auto_suggestion_enabled`、`fragments`、`insights`、`budget`、`authorization`、`provider_enabled_count`、`provider_ids`、`llm_cooling`、`llm_cooling_until_ms`、`network`、`audit_available`、`relevance`、`suppression`、`candidate_seeds`、`cache_age_ms`、`refresh_available`）；`BudgetFixture` 用 `Vec<(u32,u8)>`/`Vec<(String,u64)>` 有序对；`InsightFixture.reference_digest`/`CandidateSeedFixture.dedupe_key` 为 32 hex 字符串。
  - 禁止：任何 runner（`run`/`main`/`replay_all`）、`std::fs`、`println!`、`rand`、`Instant`、原文/URL/`request_id`/模型名/Key 字段。
  - DoD：`grep -cE "^pub (struct|enum) "` = 12；`grep -cE "fn (run|main|replay_all)"` = 0；`test ! -d .../gate_replay` 退 0；单测 `gate_input_fixture_covers_all_input_state`（构造 8 类状态并 `serde_json` 往返字节一致）退 0
  - 证据：`test -f crates/domain/src/model/gate_replay.rs && test ! -d crates/domain/src/model/gate_replay; echo "exit=$?"` → `exit=0`；`"$CARGO" test -p meeting-copilot-domain gate_input_fixture_covers_all_input_state; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-2.3 `model/mod.rs` 仅追加 3 行（删除列必须为 0）** · `crates/domain/src/model/mod.rs`
  - 追溯：REQ-B4P0-04 · AC-B4-43 · design §3.2.2（改动行清单）
  - 改动内容：在既有 `pub mod gate;`（第 8 行）之后追加 `pub mod gate_replay;`；新增两条独立 `pub use` 行——`pub use gate_replay::{BudgetFixture, AuthorizationFixture, CandidateSeedFixture, ClockFixture, ExpectedDecision, FragmentFixture, GateInputFixture, GateReplayCase, InsightFixture, RelevanceFixture, ReplayReport, SuppressionFixture};` 与 `pub use gate::{...29 项新增符号...};`。**既有 `pub use gate::{GateRejectReason, Visibility};` 原样保留**，不得就地改写（否则 1 删 1 增，删除列 >0）。
  - DoD：`git diff --numstat HEAD -- crates/domain/src/model/mod.rs` → 删除列 `0`、新增列 ≤ `3`；12 个夹具名与 29 个契约名可从 `model::` 解析；`"$CARGO" test -p meeting-copilot-domain` 退 0
  - 证据：`git diff --numstat HEAD -- crates/domain/src/model/mod.rs` → 输出 `0	3	crates/domain/src/model/mod.rs`；`grep -c "^pub use gate::{GateRejectReason, Visibility};$" crates/domain/src/model/mod.rs` = 1
  - 证据产物：`git diff --numstat` 原始两行（改动前后）粘贴到 `b4-status.md` §2

- [ ] **W-B4-2.4 扁平落位静态断言（不依赖未生成脚本）** · `crates/domain/src/model/`
  - 追溯：REQ-B4P0-01 · AC-B4-43/47 · design R-01、§3.8.3
  - 依赖：2.1、2.2、2.3。**时序说明**：原 tasks 此步要求 `--rule=source/budget/record` 退 0，但脚本在 W-B4-7 才生成；本阶段拆为"静态落位断言（此处）"+"三规则退 0（8.3 复跑）"。
  - DoD：`gate.rs`/`gate_replay.rs` 是文件而非目录；`crates/domain/src/model/gate/` 与 `gate_replay/` 均不存在；模型目录下无同名 `.rs` 之外的冲突文件
  - 证据：`test -f crates/domain/src/model/gate.rs && test ! -d crates/domain/src/model/gate && test -f crates/domain/src/model/gate_replay.rs && test ! -d crates/domain/src/model/gate_replay; echo "exit=$?"` → `exit=0`
  - 证据产物：三条 `--rule` 的退出码在 8.3 补齐（P0 出口口径不变）

### W-B4-3 `GatePolicy` 校验与 `skip_*` 防静默吸收

> 前置：W-B4-2.1、W-B4-2.3。三子任务都追加到既有 `gate.rs`，不改既有 23 变体与 7 单测。

- [ ] **W-B4-3.1 `GatePolicy` 8 字段默认值 + `validate()` + 三档投影** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-05 · AC-B4-11/24 · design §3.3.1 #7/#8/#9/#10、§3.3.2-8
  - 改动内容：手写 `impl Default for GatePolicy`（`relevance_enabled=true`、`dedupe_enabled=true`、`threshold_profile=Balanced`、`frequency_profile=Medium`、`session_outbound_cap=120`、`auto_query_limit=80`、`on_demand_query_limit=200`、`max_cache_age_ms=86_400_000`）；`impl Default for ThresholdProfile`→`Balanced`、`FrequencyProfile`→`Medium`；`ThresholdProfile::milli()`（720/620/520）；`ThresholdProfile::from_milli(u16) -> Result<Self, GatePolicyError>`（三值命中，其余 `ThresholdProfileInvalid(milli)`，使该变体可达）；`FrequencyProfile::per_window()`（1/3/5）；`QueryLimit::chars()`（80/200）；`GatePolicy::threshold_milli()` 委托；`GatePolicy::validate() -> Result<(), GatePolicyError>` 数值分支顺序固定：`session_outbound_cap`(0..=600) → `auto_query_limit==80 && on_demand_query_limit==200`（`QueryLimitNotFixed{auto,on_demand}`）→ `threshold_profile.milli() ∈ {520,620,720}`（恒真，防御性保留）。
  - DoD：空 JSON `{}` 反序列化后 `validate()` 为 `Ok` 且 8 字段等于上述默认值；`cap=601`/`auto=81`/`on_demand=201` 分别返回 `SessionCapOutOfRange(601)`/`QueryLimitNotFixed`；`cap=0` 合法；`from_milli(500)` 返回 `ThresholdProfileInvalid(500)`
  - 证据：`"$CARGO" test -p meeting-copilot-domain policy_validate_rejects_out_of_range_and_fixes_query_limits; echo "exit=$?"` → `exit=0`；`"$CARGO" test -p meeting-copilot-domain threshold_profile_from_milli_is_reachable; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-3.2 `unknown` 透传容器 + `skip_` 前缀拒绝（先 skip 后数值）** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-06 · AC-B4-08 · design §3.3.2-5、§2.4.4（R-09）
  - 改动内容：`GatePolicy` 增 `#[serde(default, flatten)] pub unknown: BTreeMap<String, serde_json::Value>`（**不计入 8 业务字段**，`--rule=record` 的白名单已排除该名）；`validate()` **第一步**扫 `unknown` 键：`skip_g1`→`MandatoryStageCannotSkip{stage:1}`、`skip_g4`→`{stage:4}`、`skip_g5`→`{stage:5}`、其它 `skip_` 前缀→`{stage:0}`；**第二步**才做 3.1 的数值检查（保证 `skip_g1=true` 叠加 `cap=601` 报 `MandatoryStageCannotSkip`，安全优先）。
  - DoD：`GatePolicy` 类型上无 `skip_g*` 字段；含 `skip_g1/skip_g4/skip_g5` 的 JSON 反序列化后 `validate()` 分别返回 stage=1/4/5 的 `MandatoryStageCannotSkip`；任意 `skip_foo` → stage=0；`skip_g1 + cap=601` → 仍是 `MandatoryStageCannotSkip`
  - 证据：`"$CARGO" test -p meeting-copilot-domain policy_rejects_skip_g1_g4_g5; echo "exit=$?"` → `exit=0`；`grep -c "pub skip_g" crates/domain/src/model/gate.rs` → `0`

- [ ] **W-B4-3.3 非 `skip_` 未知字段往返保留** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-07 · AC-B4-47 · design §3.3.2-5
  - DoD：反序列化 `{"future_flag":1,"future_nested":{"a":"b"}}` 后 `unknown` 含两键；重新序列化后 `future_flag=1` 与嵌套值原样出现；8 个既有字段语义零变化
  - 证据：`"$CARGO" test -p meeting-copilot-domain policy_keeps_unknown_fields_roundtrip; echo "exit=$?"` → `exit=0`

### W-B4-4 决策、记录、意图与审计事件契约

> 前置：W-B4-2.1、W-B4-2.3。domain 侧只落类型、谓词与 JSON 往返；`GateError`/`validate_record` 的 application 部分由 W-B4-6.6 承接（D-B4-D10）。

- [ ] **W-B4-4.1 五级轨迹固定 5 元素 + `empty()` 构造** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-08 · AC-B4-47 · design §3.3.1 #19/#20、§3.5.3（`PerStageResults::empty`）
  - 改动内容：`StageTrace{calls:u16,result:StageResult,reason:Option<GateRejectReason>,elapsed_ms:u32}`；`PerStageResults(pub [StageTrace;5])`；`PerStageResults::empty()` 返回 `[StageTrace{calls:0,result:NotReached,reason:None,elapsed_ms:0};5]`；`GateStage::ALL: [Self;5]` 顺序 G1..G5；`StageResult` 5 变体。
  - DoD：序列化任意 `PerStageResults` 后数组长度恒 5；`empty().0.iter().all(|t| t.calls==0 && t.result==NotReached && t.reason.is_none())`；`GateStage::ALL.len()==5`
  - 证据：`"$CARGO" test -p meeting-copilot-domain per_stage_serializes_exactly_five_entries; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-4.2 `GateRecord` 14 业务字段 + flatten 透传 + 三次往返字节相等** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-09 · AC-B4-47 · design §3.3.1 #23、§3.3.2-4、R-04
  - 改动内容：14 业务字段逐字为 `schema_version:u16`、`trace_id:TraceId`、`session_id:SessionId`、`candidate_ref:Option<DedupeKey>`、`insight_ref:Option<InsightRef>`、`trigger:TriggerKind`、`per_stage:PerStageResults`、`decision:DecisionKind`、`stopped_at:Option<GateStage>`、`budget_remaining:u16`、`cooldown_until_ms:Option<u64>`、`authorization_status:AuthorizationStatus`、`skipped_stages:Vec<GateStage>`、`decided_at_ms:u64`；外加 `#[serde(default, flatten)] pub unknown: BTreeMap<String, serde_json::Value>`（**不计入 14**）。`#[serde(default)]` 仅覆盖 `candidate_ref`/`insight_ref`/`cooldown_until_ms`/`skipped_stages`/`unknown`。
  - DoD：`grep -cE "^    pub [a-z_]+:"` 在 `GateRecord` 块内 = **15**（14 + `unknown`）；剔除 `unknown` 后集合等于 14 项白名单；`serde_json` 序列化→反序列化→再序列化，两次字节串相等；`schema_version` 存在且默认写 1
  - 证据：`"$CARGO" test -p meeting-copilot-domain gate_record_roundtrips_with_closed_fields; echo "exit=$?"` → `exit=0`；`"$CARGO" test -p meeting-copilot-domain gate_contract_types_are_31; echo "exit=$?"` → `exit=0`（交叉锁定 `GateStage::ALL=5`、`GateCounter::ALL=32`、`GateRejectReason::ALL=23`、测试私有 `RecordField::ALL=14`）

- [ ] **W-B4-4.3 `skipped_stages_are_valid()` 仅允许 G2/G3** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-10 · AC-B4-47 · design §3.3.2-7
  - 改动内容：`GateRecord::skipped_stages_are_valid(&self) -> bool`：空集合合法；非空时逐项 ∈ {`G2`,`G3`}；`Vec` 中的重复项不使结果失真（仍按集合语义判定）。
  - DoD：`[]`、`[G2]`、`[G3]`、`[G2,G3]` 均返回 true；含 `G1`/`G4`/`G5` 任一项返回 false；不 panic
  - 证据：`"$CARGO" test -p meeting-copilot-domain skipped_stages_only_g2_g3; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-4.4 `OutboundIntent` / `GateAuditEvent` 零转录结构** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-12 · AC-B4-34/29/42 · design §3.3.1 #24/#25/#26/#12/#13
  - 改动内容：`OutboundIntent` 7 字段（`sanitized_query_len:usize`、`content_type:GateContentType`、`provider_ids:Vec<ProviderId>`、`authorization_status:AuthorizationStatus`、`truncated:bool`、`queued:bool`、`cooldown_until_ms:Option<u64>`）；`GateAuditEvent` 7 字段 + `occurred_at_ms:u64`（`authorization_status`、`granted_at_ms`、`content_type`、`query_len`、`provider_ids`、`result:AuditResult`、`reason:Option<GateRejectReason>`）；`GateContentType` **仅** `TextQuery`；`AuditResult` 2 项；`QueryLimit` 2 项。
  - 禁止：`OutboundIntent` 内不得出现 `query`/`text`/`url`/`request_id`/`model`/`api_key`/`SanitizedQuery`；`GateAuditEvent` 内不得出现 `stopped_at`。
  - DoD：两结构序列化后上述禁止字段名命中 0；`GateContentType` 反序列化 `"audio"` 返回 `Err`；`"text_query"` 返回 `Ok`
  - 证据：`"$CARGO" test -p meeting-copilot-domain outbound_intent_and_audit_event_have_no_content_fields; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-4.5 `refs_are_valid()` + `decision_tag_matches_stopped_at()` 谓词** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-11 · AC-B4-47 · design §3.3.2-7、§3.5.3（Q4 可识别口径）
  - 改动内容：`refs_are_valid(&self) -> bool` = `candidate_ref.is_some() || insight_ref.is_some()`；`decision_tag_matches_stopped_at(&self) -> bool` = `(decision==Deny) == stopped_at.is_some()`。**不加错误类型**（31 项封闭数不变，D-B4-D08）。
  - DoD：双空 → `refs_are_valid()==false` 且不 panic；任一非空 → true；`Deny+Some`/`Allow+None` → 匹配 true；`Deny+None`/`Allow+Some` → false
  - 证据：`"$CARGO" test -p meeting-copilot-domain record_rejects_empty_refs; echo "exit=$?"` → `exit=0`

### W-B4-5 独立预算与 32 项计数

> 前置：W-B4-2.1、W-B4-2.3。5.1~5.3 只改 `gate.rs`；5.4 拆为 **5.4a（domain，本节）** 与 **5.4b（application `record::bump_by`，见 W-B4-6.6）**（Q1 裁决）。

- [ ] **W-B4-5.1 `EntityKey::parse` + `GateBudget` 7 字段 + 饱和剩余 + 判定外 `commit`** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-13 · AC-B4-22/23/24/27 · design §3.3.2-1/6、§3.6.2-3、§3.7.3
  - 改动内容：`EntityKey::parse(raw:&str) -> Option<Self>`（恰 32 位十六进制、大小写归一为小写，否则 `None`；**不新增错误类型**，D-B4-D08）；`GateBudget` 7 字段（`session_outbound_used:u16`、`global_window_used:u8`、`topic_window_used:BTreeMap<u32,u8>`、`entity_last_seen_ms:BTreeMap<EntityKey,u64>`、`inflight:u8`、`tokens_milli:u16`、`window_start_ms:u64`）；`session_remaining_with(cap)` 用 `saturating_sub`；`is_within_limits() -> bool`（`used≤600`、`inflight≤3`、`tokens_milli≤3000`、每桶 ≤2、`global_window_used≤5`）；`commit(&mut self,&GateDecision)` **仅**在 `Allow` 且 `outbound.is_some()` 时 `session_outbound_used +=1`、`global_window_used` 饱和 +1、`tokens_milli` 饱和 -1000；`Deny` 时零变化。
  - 边界：P0 **不实现**"越界 → `Deny{BudgetExhausted}`"（G4，属 W-B4-19）；本节只提供谓词，越界到 Deny 的映射由 P2 承接。
  - DoD：`used=120,cap=120` → 剩余 0；`used=121,cap=120` → 剩余 0（非 65535）；`used=601`/`inflight=4`/`tokens=3001`/桶=3 时 `is_within_limits()==false`；`commit(Deny)` 后 7 字段逐项相等；`grep -c "\.commit(" crates/application/src/gate` = 0（P0 不被门控调用）
  - 证据：`"$CARGO" test -p meeting-copilot-domain budget_rejects_corrupted_snapshot_and_saturates_remaining; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-5.2 `GateCounter` 32 变体 + `GateCounters([u64;32])` 单一 bump** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-14 · AC-B4-08/13/24/35 · design §3.3.1 #17/#18、§3.3.2-2、§3.3.3
  - 改动内容：`GateCounter` 恰 32 变体，顺序 = requirements §6.4 的 1..32（`GateL0Candidates`…`GateReplayMismatch`）；`pub const ALL: [Self;32]`；`pub const fn index(self) -> usize`；`GateCounters { counts: [u64;32] }`（`#[serde(transparent)]`）；`bump(&mut self, GateCounter)` 为唯一写入口（饱和 `+1`，不回绕、不清零）；`get(&self,..)->u64`、`snapshot(&self)->[(GateCounter,u64);32]`、`from_snapshot([(GateCounter,u64);32]) -> Self` 只读/恢复。
  - DoD：`GateCounter::ALL.len()==32` 且首项 `GateL0Candidates`、末项 `GateReplayMismatch`；`index()` 单调 0..31；连续 `bump` 同项 3 次 → `get==3`、`snapshot().len()==32`；`from_snapshot(snapshot())` 往返相等；`grep -c HashMap gate.rs` = 0
  - 证据：`"$CARGO" test -p meeting-copilot-domain gate_counter_all_is_32_in_spec_order; echo "exit=$?"` → `exit=0`；`"$CARGO" test -p meeting-copilot-domain gate_counters_bump_is_monotonic_and_snapshot_is_32; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-5.3 32 项 serde 名逐项显式 `rename`（D-B4-D11）** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-14 · AC-B4-08/42 · design §3.3.2-3
  - 改动内容：**每个变体**写一条 `#[serde(rename="...")]`（32 条），不使用 `rename_all` 推导管数字边界。逐项对照表（左 = 变体，右 = serde 名）：

    | # | 变体 / serde 名 | # | 变体 / serde 名 |
    |---|---|---|---|
    | 1 | `GateL0Candidates` / `gate_l0_candidates` | 17 | `GateL1NoProvider` / `gate_l1_no_provider` |
    | 2 | `GateL0LowConfidence` / `gate_l0_low_confidence` | 18 | `GateL1SensitiveTerm` / `gate_l1_sensitive_term` |
    | 3 | `GateL0QuestionAnsweredSkipped` / `gate_l0_question_answered_skipped` | 19 | `GateL2LowRelevance` / `gate_l2_low_relevance` |
    | 4 | `GateL0SensitiveSeedBlocked` / `gate_l0_sensitive_seed_blocked` | 20 | `GateL3Duplicate` / `gate_l3_duplicate` |
    | 5 | `GateAllowTotal` / `gate_allow_total` | 21 | `GateL3Suppressed` / `gate_l3_suppressed` |
    | 6 | `GateDenyTotal` / `gate_deny_total` | 22 | `GateL3Stale` / `gate_l3_stale` |
    | 7 | `GateStageSkippedTotal` / `gate_stage_skipped_total` | 23 | `GateL3Quota` / `gate_l3_quota` |
    | 8 | `GateL1AutoDisabled` / `gate_l1_auto_disabled` | 24 | `GateL4BudgetExhausted` / `gate_l4_budget_exhausted` |
    | 9 | `GateL1ConsentMissing` / `gate_l1_consent_missing` | 25 | `GateL4GatewayCooling` / `gate_l4_gateway_cooling` |
    | 10 | `GateL1Paused` / `gate_l1_paused` | 26 | `GateL4Offline` / `gate_l4_offline` |
    | 11 | `GateL1SilentPeriod` / `gate_l1_silent_period` | 27 | `GateL5NotGranted` / `gate_l5_not_granted` |
    | 12 | `GateL1ScreenShare` / `gate_l1_screen_share` | 28 | `GateL5SanitizeFailed` / `gate_l5_sanitize_failed` |
    | 13 | `GateL1LowConfidence` / `gate_l1_low_confidence` | 29 | `GateL5AuditUnavailable` / `gate_l5_audit_unavailable` |
    | 14 | `GateL1Duplicate` / `gate_l1_duplicate` | 30 | `GateL5ProviderOrDomainBlocked` / `gate_l5_provider_or_domain_blocked` |
    | 15 | `GateL1Suppressed` / `gate_l1_suppressed` | 31 | `GateReplayRuns` / `gate_replay_runs` |
    | 16 | `GateL1QuotaExhausted` / `gate_l1_quota_exhausted` | 32 | `GateReplayMismatch` / `gate_replay_mismatch` |

  - DoD：32 个 serde 值与 requirements §6.4 列表**逐项字符串相等**（集合与顺序双断言）；`serde_json::to_string(&GateCounter::X)` 的引号内文本等于上表右列；无 `rename_all` 依赖
  - 证据：`"$CARGO" test -p meeting-copilot-domain gate_counter_names_match_spec_field_names; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-5.4a `GateRejectReason::counter_at(GateStage)` 静态映射（5.4a）** · `crates/domain/src/model/gate.rs`
  - 追溯：REQ-B4P0-15 · AC-B4-08/13/16/17/20/25/35/36 · design §3.7.2 表 B、§3.10.1-Q1
  - 改动内容：在**既有** `impl GateRejectReason` 末尾追加 `pub const fn counter_at(self, stage: GateStage) -> Option<GateCounter>`；唯一参数是"产生级"，非产生级返回 `None`。映射：G1 七项 `PreFilterLowConfidence→GateL1LowConfidence`、`ConsentMissing→GateL1ConsentMissing`、`AutoDisabled→GateL1AutoDisabled`、`Paused→GateL1Paused`、`SilentPeriod→GateL1SilentPeriod`、`ScreenShareProtection→GateL1ScreenShare`、`SensitiveTerm→GateL1SensitiveTerm`；**分流三项** `Duplicate`（G1→`GateL1Duplicate`/G3→`GateL3Duplicate`）、`Suppressed`（G1→`GateL1Suppressed`/G3→`GateL3Suppressed`）、`QuotaExhausted`（G1→`GateL1QuotaExhausted`/G3→`GateL3Quota`）；`NoProviderEnabled` 仅 G1→`GateL1NoProvider`；`LowRelevance` 仅 G2→`GateL2LowRelevance`；`Stale` 仅 G3→`GateL3Stale`；`BudgetExhausted`/`GatewayCooling`/`Offline` 仅 G4；`OutboundNotGranted`/`SanitizeFailed`/`AuditUnavailable`/`ProviderNotAllowed`+`DomainBlocked` 仅 G5（后两者共用 `GateL5ProviderOrDomainBlocked`）；`PolicyUnavailable`/`RetentionExpired` **恒 None**（不新增计数项）。
  - 禁止：`counter_at` 不得返回 `GateAllowTotal`/`GateDenyTotal`（总量在 6.6 按 `DecisionKind` 互斥 +1）。
  - DoD：23 个原因码 × 5 级 = 115 组合逐一与 design 表 B 相等；`Duplicate+ G4 == None`；`PolicyUnavailable` 全 5 级 `None`；既有 7 单测零回归
  - 证据：`"$CARGO" test -p meeting-copilot-domain bump_by_reason_matches_static_table; echo "exit=$?"` → `exit=0`；`"$CARGO" test -p meeting-copilot-domain duplicate_splits_by_stage; echo "exit=$?"` → `exit=0`；`"$CARGO" test -p meeting-copilot-domain no_provider_is_counted_once; echo "exit=$?"` → `exit=0`
  - 证据产物：P0 断言落 domain 侧（Q2 收窄）；P1 在 `gate_prefilter` target 追加同名集成用例，不删除本用例

- [ ] **W-B4-5.4b `record::bump_by`（application 侧薄包装）** · `crates/application/src/gate/record.rs` → 见 **W-B4-6.6**
  - 追溯：REQ-B4P0-15 · design §3.10.1-Q1、§3.7.2 表 C
  - 说明：依赖 `gate/` 目录与 `mod.rs`（W-B4-6.5），故不在本节执行；本行仅保留 5.4b 编号锚点，避免下游引用断裂。

### W-B4-6 `gate/**` 落位与 `gate/api.rs` 契约（4 个 `pub fn` 单文件冻结）

> 前置：W-B4-2~5 的 domain 侧完成（类型可解析）。**本任务必须一次成形**，否则 AC-46 的 grep 断言会在 P2/P3 反复返工。
> 执行顺序：6.1 ∥ 6.2 → 6.3 → 6.4 → 6.5（首个 `cargo build` 退 0）→ 6.6 → 6.7。

- [ ] **W-B4-6.1 输入类型骨架（4 类型，含 D-B4-D05 三字段）** · `crates/application/src/gate/input.rs`
  - 追溯：REQ-B4P0-16/17 · AC-B4-38/46 · design §3.5.1、§3.10.1-Q3
  - 改动内容：`GateSnapshot<'a>` 与 `GateInput<'a>` 各 **16 字段** = 草案 13 字段（`session:&'a SessionId`、`trigger`、`insights:&'a [Insight]`、`fragments:&'a [NormalizedFragment]`、`policy:&'a EffectivePolicy`、`authorization:&'a GateAuthorization`、`provider:ProviderView`、`llm:LlmCapabilityView`、`network:NetworkState`、`now_ms:u64`、`budget:GateBudget`、`audit_available:bool`、`scan:GapScanInput<'a>`）+ `candidate_ref:Option<&'a DedupeKey>`、`insight_ref:Option<&'a InsightRef>`、`trace_id:&'a TraceId`；`GapScanInput<'a>` 按 design §2.3.2-C 逐项落 `session`、`fragments`、`insights`、`lexicon_names`、`explained_terms`、`acronym_whitelist`、`sensitive_terms`、`question_wait_ms`（固定 10_000）、`stable_prefix_ms`、`answered_at_ms`、`now_ms`（design §3.5.1 记 12 项；若执行时发现草案与 12 项不一致，**以草案字段清单为准**并在 `b4-status.md` §4 登记，不新增未定义字段）；`RelevanceInput<'a>` 7 字段（`candidate`、`semantic_milli`、`entity_milli`、`gain_milli`、`source_trust_milli`、`delta_ms`、`threshold_profile`）。
  - 禁止：`InsightDraft`、`InsightStore`、`Vec<Insight>` 所有权、任何写入口。
  - DoD：`cargo build -p meeting-copilot-application` 退 0；`grep -rc "InsightDraft" crates/application/src/gate` 合计 0；`GapScanInput::question_wait_ms` 默认构造为 10_000
  - 证据：`"$CARGO" build -p meeting-copilot-application; echo "exit=$?"` → `exit=0`；`! grep -rq "InsightDraft\|InsightStore" crates/application/src/gate` 判定通过

- [ ] **W-B4-6.2 时钟端口 `GateClock` + `FixedClock`（无 OS 计时）** · `crates/application/src/gate/clock.rs`
  - 追溯：REQ-B4P0-16/17 · AC-B4-07/43/48 · design §3.6.2-2
  - 改动内容：`pub trait GateClock: Send + Sync { fn now_ms(&self) -> u64; }`；`pub struct FixedClock { anchor_ms:u64, tick_ms:u64, calls: Cell<u64> }`，`now_ms` 返回 `anchor_ms + n*tick_ms`（第 0 次 = `anchor_ms`）；提供 `new(anchor_ms,tick_ms)`。**P0 不实现**生产镜像时钟（P1 的 W-B4-10.3）；`evaluate_gate` 占位实现不得调用 `clock.now_ms()`。
  - DoD：`Cell<u64>` 实现（`Send` 但非 `Sync`）；无 `Mutex`/原子/`Instant::now`/`SystemTime::now`/`thread::sleep`；连续 5 次取值 = `anchor, anchor+tick, …, anchor+4tick`
  - 证据：`! grep -qE "Instant::now|SystemTime::now|thread::sleep|Mutex|AtomicU64" crates/application/src/gate/clock.rs` 判定通过；`"$CARGO" test -p meeting-copilot-application --lib fixed_clock_is_deterministic_sequence; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-6.3 7 个 ≤5 行骨架文件（D-B4-D09）** · `crates/application/src/gate/{prefilter,relevance,dedupe,budget,lease,outbound,replay}.rs`
  - 追溯：REQ-B4P0-16/17 · AC-B4-43/46/49 · design §3.2.1（#9~#15）、§3.10.3-D-B4-D09
  - 改动内容：每个文件仅一行 `//! <职责一句话>；P0 骨架，P1~P3 填充实现。`（≤5 行，无代码、无 `pub fn`、无四个公开名、无 `todo!`）。理由：`gate/mod.rs` 一次性声明 11 个 `mod`，文件缺失即编译失败；`test-name-inventory.sh` 与 6 个 `gate_*` target **P0 不动**。
  - DoD：7 文件均存在且 `wc -l` ≤5；7 文件中 `grep -cE "fn (detect_gaps|evaluate_gate|score_relevance|record_from)"` = 0
  - 证据：`for f in prefilter relevance dedupe budget lease outbound replay; do test -f "crates/application/src/gate/$f.rs" || echo "MISSING $f"; done; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-6.4 `api.rs` 4 个 `pub fn` 逐字冻结 + fail-closed 占位实现** · `crates/application/src/gate/api.rs`
  - 追溯：REQ-B4P0-16/17 · AC-B4-43/46 · design §3.5.2（签名）、§3.5.3（占位语义）、§3.8.2（source 规则）
  - 改动内容：**顶格、无缩进、不嵌套 `mod`、不加属性行**，逐字写 4 个签名：
    ```rust
    pub fn detect_gaps(input: &GapScanInput<'_>) -> Vec<GapCandidate>;
    pub fn evaluate_gate(input: &GateInput<'_>, policy: &GatePolicy, clock: &dyn GateClock) -> GateDecision;
    pub fn score_relevance(input: &RelevanceInput<'_>, policy: &GatePolicy) -> RelevanceScore;
    pub fn record_from(input: &GateInput<'_>, decision: &GateDecision, budget: &GateBudget) -> GateRecord;
    ```
    占位语义（逐项可断言）：`detect_gaps` → `Vec::new()`；`evaluate_gate` → `Deny{reason:PolicyUnavailable, stopped_at:G1, per_stage:PerStageResults::empty(), cooldown_until_ms:None}`（**不读 `clock`**，参数名保留 `clock` 但不得调用 `now_ms()`）；`score_relevance` → 6 项全 0（`policy` 用 `_policy` 避免未使用告警）；`record_from` → 14 字段记录（`schema_version=1`、`trace_id` 取 `input.trace_id`、`session_id` 取 `input.session.clone()`、`candidate_ref` 取 `input.candidate_ref` 否则决策首个候选的 `dedupe_key`、`insight_ref` 只取 `input.insight_ref`、`per_stage` 克隆决策轨迹、`decision` 投影为 `DecisionKind`、`stopped_at` 取决策、`budget_remaining = budget.session_remaining_with(600).min(600)`、`authorization_status = input.authorization.status`、`skipped_stages=vec![]`、`decided_at_ms = input.now_ms`、`cooldown_until_ms` 取决策；双空 refs 时 `unknown["gate_record_violation"] = "refs_empty"`）。
  - 禁止：`todo!`/`unimplemented!`/`panic!`；`allow` 分支；任何 IO/时钟读取；在 `api.rs` 放判定细节。
  - clippy 口径：未使用参数**不得改签名参数名**，在函数体首行 `let _ = (input, policy, clock);`（按实际未使用者裁剪）消除 `unused_variables`，保证 8.5 的 `clippy -- -D warnings` 退 0。
  - DoD：`grep -c '^pub fn '` = **4**；四名匹配 = **4**；四名定义只出现在 **1** 个文件；`cargo build -p meeting-copilot-application` 退 0
  - 证据：`[ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ]`；`[ "$(grep -cE 'pub fn (detect_gaps|evaluate_gate|score_relevance|record_from)\(' crates/application/src/gate/api.rs)" = "4" ]`；`[ "$(grep -rlE 'pub fn (detect_gaps|evaluate_gate|score_relevance|record_from)\(' crates/application/src/gate/*.rs | wc -l | tr -d ' ')" = "1" ]` → 三条均退 0

- [ ] **W-B4-6.5 门面 `gate/mod.rs` + `lib.rs` 仅 1 行（首个 build 退 0）** · `crates/application/src/gate/mod.rs`、`crates/application/src/lib.rs`
  - 追溯：REQ-B4P0-16/17 · AC-B4-46/49 · design §3.5.4
  - 改动内容：`mod.rs` 声明 11 个子模块（`api`/`input` 用 `pub mod`，其余 `pub(crate) mod`，顺序按 design §3.5.4）；导出 `pub use api::{detect_gaps, evaluate_gate, record_from, score_relevance};`、`pub use clock::{FixedClock, GateClock};`、`pub use input::{GapScanInput, GateInput, GateSnapshot, RelevanceInput};`；`lib.rs` **仅追加 1 行** `pub mod gate;`（不改既有三行与既有 `pub use`）。
  - 禁止：`mod.rs` 不 re-export `OutboundLease`/`GateAuditSink`/`GateCounters`（回放面不扩散）；P0 **不创建** `GateAuditSink`/`OutboundLease`/`LeaseId`/`LeaseOutcome`/`GateFeature`/`GateError`（P2/P3 落地）。
  - DoD：`grep -cE "^(pub |pub\(crate\) )?mod (api|input|clock|prefilter|relevance|dedupe|budget|lease|outbound|record|replay);"` = **11**；`grep -c "^pub mod gate;$" lib.rs` = **1**；`cargo build -p meeting-copilot-application` 退 0
  - 证据：`"$CARGO" build -p meeting-copilot-application; echo "exit=$?"` → `exit=0`；`[ "$(grep -c '^pub mod gate;$' crates/application/src/lib.rs)" = "1" ]`

- [ ] **W-B4-6.6 `record.rs`：`validate_record` + `bump_by`（5.4b）+ 纯 Rust 单测** · `crates/application/src/gate/record.rs`
  - 追溯：REQ-B4P0-10/11/15 · AC-B4-08/13/16/17/20/25/35/36/47 · design §3.7.1（D-B4-D10）、§3.7.2 表 C、§3.10.1-Q2
  - 改动内容：内部 `pub(crate) enum RecordViolation { RefsEmpty, IllegalSkipStage }`；`pub(crate) fn validate_record(&GateRecord) -> Option<RecordViolation>`（先读 `unknown["gate_record_violation"] == "refs_empty"`，其次 `refs_are_valid()`，最后 `skipped_stages_are_valid()`）；`pub(crate) fn bump_by(&mut GateCounters, &GateDecision)`：`Allow` → `GateAllowTotal+1` 且 deny 不增；`Deny` → `GateDenyTotal+1` 且 `counter_at(reason, stopped_at)` 若 `Some` 则 +1；两者互斥。
  - 硬约束（Q2 收窄）：`application` **无 `serde_json` 且不得新增**；`#[cfg(test)]` **禁止任何 JSON 往返**，只允许纯 Rust 断言；JSON 往返一律在 domain 侧。
  - DoD：双空 refs → `Some(RefsEmpty)`；`skipped_stages` 含 `G4` → `Some(IllegalSkipStage)`（优先 RefsEmpty 时按先读顺序返回 RefsEmpty）；单次 `Allow` 后 allow 增量 1 / deny 增量 0，`Deny` 反之；同一次 `Deny{Duplicate, stopped_at:G3}` 只使 `gate_l3_duplicate+1` 与 `gate_deny_total+1`
  - 证据：`"$CARGO" test -p meeting-copilot-application --lib record_rejects_empty_refs; echo "exit=$?"` → `exit=0`；`"$CARGO" test -p meeting-copilot-application --lib bump_by_reason_matches_static_table; echo "exit=$?"` → `exit=0`；`"$CARGO" test -p meeting-copilot-application --lib allow_and_deny_are_mutually_exclusive; echo "exit=$?"` → `exit=0`；`grep -c "serde_json" crates/application/src/gate/record.rs` = 0

- [ ] **W-B4-6.7 契约静态断言（4/4/1 + 回放面 + 三类宏）** · `crates/application/src/gate/**`
  - 追溯：REQ-B4P0-16/17 · AC-B4-43/46/49 · design §3.5.2、§3.5.4、§3.6.1
  - DoD：4/4/1 三条 grep 退 0；`gate/mod.rs` 的 `pub use` 中 `OutboundLease`/`GateAuditSink`/`GateCounters` 命中 0；`gate/**` 中 `todo!\(`/`unimplemented!\(`/`panic!\(` 命中 0（`#[cfg(test)]` 之外的源码）；`gate/**` 中 `reqwest::`/`hyper::`/`TcpStream`/`std::net`/`Instant::now`/`SystemTime::now`/`thread::sleep`/`std::env`/`LLM_` 命中 0
  - 证据：`grep -cE "pub use .*(OutboundLease|GateAuditSink|GateCounters)" crates/application/src/gate/mod.rs` → `0`；`grep -rcE "todo!\(|unimplemented!\(|panic!\(" crates/application/src/gate | grep -v ":0" | wc -l` → `0`（源码头，测试行由脚本 `strip_tests` 处理；强制脚本口径见 8.3 的 `--rule=source`）

### W-B4-7 `scripts/check-b4-gates.sh` 七条规则与自检

> 前置：W-B4-2（扫描面存在）、W-B4-6.5（`gate/**` 可编译）。退出码语义：**0=通过 / 1=命中违规 / 2=必需扫描面缺失或参数非法**。
> 脚本约定：`set -uo pipefail`（**不用 `-e`**，否则 `grep` 无命中即退出，无法区分 0/1）；`ROOT` 用 `cd "$(dirname "${BASH_SOURCE[0]}")/.."`；`fail()`/`missing()`/`need_dir()` **局部复制**（不 source 既有脚本）。

- [ ] **W-B4-7.1 脚本骨架：参数、累积变量、退出码收敛、`strip_tests()`** · `scripts/check-b4-gates.sh`
  - 追溯：REQ-B4P0-18/19 · AC-B4-43 · design §3.8.1、§3.8.3
  - 改动内容：`--rule=<source|zero-transcript|budget|outbound|record|targets|doc-plan>` 与 `--self-test` 解析；未知参数/未给规则 → `exit 2`；`FAIL=0/1`、`MISSING=0/1`、`ABSENT` 列表累积；收敛顺序 **MISSING→2，FAIL→1，否则→0**；`strip_tests()` 对每个文件用 `awk` 截断到首个 `#[cfg(test)]` 之前（7 条规则共用）；`SCAN-ABSENT: <path>` 走 **stdout**，`SCAN-MISSING: <path>` 走 **stderr**，`TARGETS-MISSING: <name>` 走 **stdout**。
  - DoD：`chmod +x` 可执行；未知参数退 2；`--rule=__nope__` 退 2；`-h/--help` 打印 7 条规则与退出码（退 0）
  - 证据：`bash scripts/check-b4-gates.sh; echo "exit=$?"` → `exit=2`；`bash scripts/check-b4-gates.sh --rule=__nope__; echo "exit=$?"` → `exit=2`

- [ ] **W-B4-7.2 `--rule=source`（含 `SCAN-ABSENT: crates/domain/src/gap`）** · `scripts/check-b4-gates.sh`
  - 追溯：REQ-B4P0-18/19 · AC-B4-07/43 · design §3.8.2（source 实现要点）、§3.6.1
  - 改动内容：必需面 = `crates/application/src/gate`（目录）、`crates/domain/src/model/gate.rs`、`crates/domain/src/model/gate_replay.rs`；可选面 = `crates/domain/src/gap`（不存在 → `SCAN-ABSENT`，不判失败）；`gate*.rs` 用 `--include=gate*.rs` 匹配**扁平文件**（不匹配 `model/gate/` 目录，使"建成目录"天然退 2）。禁止符号命中必须 0：`reqwest::`、`hyper::`、`TcpStream`、`std::net`、`Instant::now`、`SystemTime::now`、`thread::sleep`、`tokio::time`、`std::env`、`env::var`、`LLM_`、`todo!\(`、`unimplemented!\(`、`panic!\(`、`static mut`、`OnceLock`、`lazy_static`、`thread_local`、`Mutex`、`RwLock`、`AtomicU64`（后 4 项仅 `gate/**`）。
  - DoD：退 **0** 且输出含 `SCAN-ABSENT: crates/domain/src/gap`；伪造 `model/gate/` 目录（在临时副本）时退 **2**（由 7.7 自检覆盖，不污染真实源码）
  - 证据：`bash scripts/check-b4-gates.sh --rule=source; echo "exit=$?"` → `exit=0`，输出含 `SCAN-ABSENT: crates/domain/src/gap`

- [ ] **W-B4-7.3 `--rule=budget`（`LLM_` 零耦合 + 7 字段齐备）** · `scripts/check-b4-gates.sh`
  - 追溯：REQ-B4P0-18 · AC-B4-22/23/24 · design §3.8.2
  - 改动内容：扫描面同 7.2 三面（均必需）；`LLM_LIVE_CALL_CAP`、`LLM_`、`std::env` 命中 0；`GateBudget` 7 字段名各 ≥1：`session_outbound_used`、`global_window_used`、`topic_window_used`、`entity_last_seen_ms`、`inflight`、`tokens_milli`、`window_start_ms`。
  - DoD：退 **0**；人为在 `gate.rs` 注入 `std::env::var` 的临时副本应退 1（真实源码不改）
  - 证据：`bash scripts/check-b4-gates.sh --rule=budget; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-7.4 `--rule=record`（字段集合相等 + 23 变体 + 白名单）** · `scripts/check-b4-gates.sh`
  - 追溯：REQ-B4P0-18 · AC-B4-47 · design §3.8.2（record 六步抽取）、R-04
  - 改动内容：① 定位 `^pub struct GateRecord {` 到首个 `^}`；② 抽取 `^    pub [a-z_]+:` 字段名；③ 剔除 `unknown`；④ 与 14 项白名单做**集合相等**（缺项/多项均 FAIL 并打印差集）；⑤ `^pub enum GateRejectReason {` 到首个 `^}` 之间统计 `^    [A-Z][A-Za-z0-9]*,$` = 23；⑥ 抽取 `skipped_stages` 白名单并断言仅含 `G2`/`G3`。**不数"31"**（避免 P1 追加类型即红）。
  - DoD：退 **0**；将白名单临时改为含 `G4`（临时副本）退 1；字段多一个（临时副本）退 1
  - 白名单（14 项，与 W-B4-4.2 同源）：`schema_version`、`trace_id`、`session_id`、`candidate_ref`、`insight_ref`、`trigger`、`per_stage`、`decision`、`stopped_at`、`budget_remaining`、`cooldown_until_ms`、`authorization_status`、`skipped_stages`、`decided_at_ms`（`unknown` 显式排除）
  - 证据：`bash scripts/check-b4-gates.sh --rule=record; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-7.5 `--rule=outbound`（`proactive/` 缺失只登记）** · `scripts/check-b4-gates.sh`
  - 追溯：REQ-B4P0-19 · AC-B4-33/49 · design §3.8.2、D-B4-D02
  - 改动内容：扫描 `gate/**` 协议客户端调用 = 0；`crates/application/src/proactive` 缺失 → `SCAN-ABSENT` 并只扫 `gate/`，不判失败、不退 2；`issue_query` 与 `AuditLog::append` 出现文件数：P0 允许 **0**（P3 必须 1）。
  - DoD：退 **0**；`grep -rln "issue_query\|AuditLog::append" crates/application/src/gate | wc -l` = 0
  - 证据：`bash scripts/check-b4-gates.sh --rule=outbound; echo "exit=$?"` → `exit=0`；`grep -rln "issue_query\|AuditLog::append" crates/application/src/gate | wc -l` → `0`

- [ ] **W-B4-7.6 `--rule=zero-transcript` + `--rule=doc-plan`** · `scripts/check-b4-gates.sh`
  - 追溯：REQ-B4P0-18/24 · AC-B4-34/42/50 · design §3.8.2、§3.9.4
  - 改动内容：`zero-transcript` 扫 `crates/application/src/gate/**`、`crates/application/tests/**`、`docs/progress/b4-status.md`（后两者缺失 → `SCAN-ABSENT` 只登记；P0 出口要求 `b4-status.md` 已存在，见 8.1），内置 5 类模式（协议前缀、Key 前缀、鉴权头、模型名前缀、请求标识）命中 0；`doc-plan` 断言 `.codeartsdoer/specs/b4_gate/spec.md` §8 表格行数 ≥1，每行第 1 列路径存在**或**该行动作含"新增"。
  - DoD：`zero-transcript` 退 **0**；`doc-plan` 退 **0**（spec §8 两处"新增"行允许路径暂不存在）；在 `b4-status.md` 临时注入鉴权头示例（临时副本）应退 1
  - 证据：`bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`；`bash scripts/check-b4-gates.sh --rule=doc-plan; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-7.7 `--rule=targets`（退 1 + 6 项可枚举）+ `--self-test`（退 1 + 零污染）** · `scripts/check-b4-gates.sh`
  - 追溯：REQ-B4P0-18/20 · AC-B4-43/47 · design §3.8.2（targets）、§3.8.4（self-test）、D-B4-D07
  - 改动内容：`targets` 从 `scripts/test-name-inventory.sh` 提取 `NEW_TARGETS='...'` 单引号词表，逐项检查 `gate_prefilter`、`gate_relevance`、`gate_dedupe`、`gate_budget`、`gate_outbound`、`gate_replay`；缺项打印 `TARGETS-MISSING: <name>` 并 `FAIL=1`（最终**退 1**）。`--self-test`：`tmp="$(mktemp -d)"` + `trap 'rm -rf "${tmp}"' EXIT`，在 `tmp` 内构造 4 个反例（网络调用、`LLM_`、`GateRecord` 少 1 字段、白名单含 `G4`），用同一组正则断言**至少 1 条检出**且退出码恰为 **1**，同时断言 `git status --porcelain` 行数为 **0**。
  - 禁止：不提供 `--expect-fail`；不改真实源码；不残留临时文件。
  - DoD：`--rule=targets` 退 **1** 且 stdout 恰 **6** 行 `TARGETS-MISSING: gate_*`（顺序可任意）；`--self-test` 退 **1**；自检后 `git status --short` 为空
  - 证据：`bash scripts/check-b4-gates.sh --rule=targets; echo "exit=$?"` → `exit=1`，`| grep -c "^TARGETS-MISSING: "` = 6；`bash scripts/check-b4-gates.sh --self-test; echo "exit=$?"` → `exit=1`；`git status --porcelain | wc -l` → `0`

### W-B4-8 P0 门禁与零 diff 取证

> 前置：W-B4-2~7。出口判据见 §0.7。
> **执行顺序**：8.1 最迟必须在 8.3 复跑前完成（`b4-status.md` 同时是 `--rule=zero-transcript` 与 `--rule=doc-plan` 的必需面，Q6）；8.2→8.3→8.4→8.5→8.6 串行。

- [ ] **W-B4-8.1 `docs/progress/b4-status.md` 最小骨架（6 段 + 偏差登记）** · `docs/progress/b4-status.md`
  - 追溯：REQ-B4P0-24 · AC-B4-50 · design §3.9.4、§3.10.2/3
  - 改动内容：新建 90~140 行，含 6 段：§1 阶段与基线（`HEAD=ade9111`、P0 范围 W-B4-2~8、Windows 单平台 / macOS "不适用（无设备）"）；§2 三条硬判据快照（E-01/E-02/E-03 命令、退出码、日期）；§3 旁证快照（5 条，含 `targets` 退 1 且 6 项可枚举）；§4 偏差登记（`D-B4-D02`、`D-B4-D04`、`R-01`、`D-B4-D05~D11`）；§5 未判定清单（G2/G3/G4 状态 = "未判定"；13.3 本体、38/39/41.5/42/43/51/52 未做）；§6 复跑方法（§3.9.1 环境前置 + §3.9.2 串联命令，可粘贴）。
  - 零转录硬约束：不得出现查询原文、完整 URL、模型名、请求标识、鉴权头示例、Key 示例。
  - DoD：6 段齐全；`grep -cE "D-B4-D0[2-9]"` ≥ 6；`grep -c "SCAN-ABSENT"` ≥ 1；`grep -c "targets"` ≥ 1；G2/G3/G4 行状态值均为"未判定"
  - 证据：`grep -cE "D-B4-D0[2-9]" docs/progress/b4-status.md` → ≥6；`bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`（该文件已被纳入必需面）

- [ ] **W-B4-8.2 P0 出口 3 条硬判据一次性串联复跑** · `crates/domain/src/model/gate.rs`、`scripts/check-b4-gates.sh`
  - 追溯：REQ-B4P0-21 · AC-B4-08/22/47 · design §3.9.2（E-01/E-02/E-03）
  - DoD：`cargo test -p meeting-copilot-domain` = 0；`--rule=record` = 0；`--rule=budget` = 0；三者串联总退出码 = 0
  - 证据：`"$CARGO" test -p meeting-copilot-domain && bash scripts/check-b4-gates.sh --rule=record && bash scripts/check-b4-gates.sh --rule=budget; echo "exit=$?"` → `exit=0`
  - 证据产物：命令 + 退出码 + 日期写入 `b4-status.md` §2（3 行齐全）

- [ ] **W-B4-8.3 七条规则 + 自检逐条取证（6×0 + targets=1 + self-test=1）** · `scripts/check-b4-gates.sh`
  - 追溯：REQ-B4P0-18/19/20 · AC-B4-07/22/33/34/42/43/47/50 · design §3.9.2、§3.11 #7/#8
  - DoD：`source`/`budget`/`record`/`outbound`/`zero-transcript`/`doc-plan` 各退 **0**；`zero-transcript` 输出**不得**含 `SCAN-ABSENT: docs/progress/b4-status.md`（该文件已在 8.1 就位）；`targets` 退 **1** 且 stdout 恰 6 行 `TARGETS-MISSING: gate_*`；`--self-test` 退 **1** 且 `git status --porcelain` 为 0 行
  - 证据：逐条执行 `for r in source budget record outbound zero-transcript doc-plan; do bash scripts/check-b4-gates.sh --rule=$r; echo "$r=$?"; done` → 6 个 `=0`；`bash scripts/check-b4-gates.sh --rule=targets >/tmp/t.txt 2>&1; echo "exit=$?"; grep -c "^TARGETS-MISSING: " /tmp/t.txt` → `exit=1` 且 `6`
  - 证据产物：8 行结果（6×0/1/1）写入 `b4-status.md` §3

- [ ] **W-B4-8.4 零改动校验矩阵（12 项）** · `crates/**`、`scripts/**`、`Cargo.toml`、`Cargo.lock`
  - 追溯：REQ-B4P0-22 · AC-B4-37/40/41 · design §3.9.3
  - DoD（逐项）：① `git diff --quiet HEAD -- crates/application/src/understand/store.rs crates/application/src/understand/deduplicator.rs` 退 0；② `git status --porcelain` 对两文件行数 0；③ `git diff --stat HEAD -- Cargo.toml Cargo.lock | wc -l` = 0；④ `grep -c serde_json crates/application/Cargo.toml` = 0；⑤ 6 个常量各恰 1：`MIN_INSIGHT_CONFIDENCE`（`utterance.rs`）、`pub const ALL: [Self; 11]`（`error.rs`）、`pub enum GapKind`（`enums.rs`）、`ShortText::LIMIT`、`UtteranceText::LIMIT`、`URL_LIMIT`；⑥ `GapKind` 枚举块内变体 = 6；⑦ `cargo test --workspace -- --list --ignored | grep -c ": test"` = 9；⑧ 7 个既有脚本 `git diff --quiet HEAD` 退 0（`check-b3-gates.sh`/`check-capture-offline.sh`/`check-mac-scope.sh`/`scan-secrets.sh`/`scan-secrets-docs.sh`/`check-g2-claims.sh`/`check-layering.sh`）；⑨ `scripts/test-name-inventory.sh` 零 diff（P0 不改 `NEW_TARGETS`）；⑩ 既有门禁串联退 0（见 8.5）；⑪ `git status --short` 仅列本阶段新增/改动文件（无临时反例残留）；⑫ `spec.md`/`requirements.md`/`tasks.md` 零 diff（design §3.11 #13：编码阶段不改设计文档）。
  - **⑫ 基线口径**：以"本轮 tasks.md 更新提交后的 HEAD"为 P0 编码基线；开工前先提交规划文档，此后 §8 全部取证命令均以该 HEAD 比较。若未提交，本项只允许 `tasks.md` 的规划性改动，`spec.md`/`requirements.md` 必须 0 行。
  - 证据：`git diff --quiet HEAD -- crates/application/src/understand/store.rs crates/application/src/understand/deduplicator.rs; echo "exit=$?"` → 0；`git diff --stat HEAD -- Cargo.toml Cargo.lock | wc -l` → 0；`"$CARGO" test --workspace -- --list --ignored 2>/dev/null | grep -c ": test"` → 9
  - 证据产物：12 行矩阵结果写入 `b4-status.md` §3/§4；未通过项必须回退改动，不得只记录

- [ ] **W-B4-8.5 既有门禁不回退（fmt / clippy / layering / secrets / b3 六规则 / deny）** · `crates/**`、`scripts/**`
  - 追溯：REQ-B4P0-23 · AC-B4-39/43/44/45 · design §3.9.3
  - DoD：`cargo fmt --all -- --check` = 0；`cargo clippy --workspace --all-targets -- -D warnings` = 0；`check-layering.sh` = 0；`scan-secrets.sh` = 0；`check-b3-gates.sh` 六条规则各 = 0（`--self-test` 预期 1）；`cargo deny check` 四项通过；`check-mac-scope.sh --rule=claims` = 0；`cargo test --workspace` 退 0 且 `failed=0`/`ignored=9`
  - 证据：`"$CARGO" fmt --all -- --check && "$CARGO" clippy --workspace --all-targets -- -D warnings && bash scripts/check-layering.sh && bash scripts/scan-secrets.sh; echo "exit=$?"` → `exit=0`；`"$CARGO" test --workspace; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-8.6 交付自检 13 项收口（文档类 #11/#12/#13 + 工作区）** · `docs/progress/b4-status.md`
  - 追溯：REQ-B4P0-24 · AC-B4-50 · design §3.11（#1~#13）
  - DoD：31 项类型 `grep -cE "^pub (struct|enum) " gate.rs` ≥31；12 项夹具 = 12；扁平落位退 0；4 函数 4/4/1；封闭数三件套（23/32/14）单测 + 脚本双绿；`--rule=source` = 0；7 规则退出码 6×0 + targets=1；`--self-test`=1 且工作区空；三条硬判据 `exit=0`；零 diff/零依赖/零改常量达标；`grep -c "D-B4-D0[2-9]" b4-status.md` ≥6；G2/G3/G4 状态均为"未判定"；指定设计文档零 diff
  - 证据：`grep -cE "^pub (struct|enum) " crates/domain/src/model/gate.rs` → ≥31；`grep -cE "^pub (struct|enum) " crates/domain/src/model/gate_replay.rs` → 12；`grep -nE "G2|G3|G4" docs/progress/b4-status.md` 中状态列 → 均"未判定"；`git status --short` → 仅预期文件
  - 证据产物：自检 13 项结论写入 `b4-status.md` §5/§6；**不宣称** G2/G3/G4 通过

---

## P1 · 输入投影 + 时钟 + G1 预筛

### W-B4-9 `GateInput` / `GateSnapshot` / `GateInputFixture` 同源构造

> 前置：W-B4-1~8。**门控只读 `Insight`，`InsightDraft` 禁入。**

- [ ] **W-B4-9.1 `GateInput` 11 必填 + 2 同源子视图** · `crates/application/src/gate/input.rs`
  - 追溯：AC-B4-38 · spec §6.1 · design §2.1.3.1-3、D-B4-D01
  - DoD：`session/trigger/insights/fragments/policy/authorization/provider/llm/network/now_ms` 10 项 + `budget: GateBudget`（值拷贝）+ `audit_available: bool` + `scan: GapScanInput`；`insights: &'a [Insight]`（**不可变借用**）；字段类型中不出现 `InsightDraft`、`InsightStore`、`Vec<Insight>` 所有权
  - 证据：`"$CARGO" build -p meeting-copilot-application` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_prefilter gate_input_has_no_insight_draft` 退 0

- [ ] **W-B4-9.2 `GateSnapshot → GateInput` 单入口，禁双份不一致** · `crates/application/src/gate/input.rs`
  - 追溯：AC-B4-38、AC-B4-37 · design §2.1.3.1-3、§2.2.2-C
  - DoD：`GateSnapshot::into_input(self) -> GateInput<'a>` 为唯一构造路径；`insights`/`fragments`/`scan.insights`/`scan.fragments` 指向同一借用，长度一致性有断言；跨会话输入被拒绝（`session` 不匹配 → `Deny{PolicyUnavailable}` 路径）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter gate_input_is_single_source` 退 0

- [ ] **W-B4-9.3 `InsightDraft` 零进入（类型层 + 静态层）** · `crates/application/src/gate/**`
  - 追溯：AC-B4-38 · spec §1.5、§1.11-N-03 · design §2.3.2-D
  - DoD：`gate/**` 中 `InsightDraft` 命中 **0**；不出现 `InsightStore`（避免拿到写方法）；`gate/**` 中 `insert(`/`update(` 对 `InsightStore` 的调用 0
  - 证据：`! grep -rq "InsightDraft" crates/application/src/gate` 判定通过；`! grep -rq "InsightStore" crates/application/src/gate` 判定通过

- [ ] **W-B4-9.4 `GapScanInput` / `RelevanceInput` 只读子视图** · `crates/application/src/gate/input.rs`
  - 追溯：AC-B4-01、AC-B4-09 · design §2.3.2-C
  - DoD：`GapScanInput` 含 `session/fragments/insights/lexicon_names/explained_terms/acronym_whitelist/sensitive_terms/question_wait_ms(const 10_000)/stable_prefix_ms/answered_at_ms/now_ms`；`RelevanceInput` 含 `candidate/semantic_milli/entity_milli/gain_milli/source_trust_milli/delta_ms/threshold_profile`；两者全部字段只读
  - 证据：`"$CARGO" build -p meeting-copilot-application` 退 0

- [ ] **W-B4-9.5 `GateInputFixture` 及 8 个 fixture 结构（C-02 输入闭包）** · `crates/domain/src/model/gate_replay.rs`
  - 追溯：AC-B4-48 · spec §1.9、§6.8 · design §2.3.3 映射表 8 行
  - **P0 状态（D-B4-D09 配套）**：12 项夹具已在 W-B4-2.2 创建并随 2.3 导出；本节**不重复创建文件**，只做"8 类状态覆盖复核 + 缺失字段补齐 + P1 语义校验"，禁止改名、禁止移动文件。
  - DoD：`GateInputFixture` 覆盖预算 7 项、授权 7 项、时钟序列、网络/冷却/源视图、审计可用性、抑制与缓存；`BudgetFixture`/`AuthorizationFixture`/`RelevanceFixture`/`SuppressionFixture`/`CandidateSeedFixture`/`FragmentFixture`/`InsightFixture`/`ClockFixture` 齐备；**无原文/URL/请求标识字段**
  - 证据：`"$CARGO" test -p meeting-copilot-domain gate_input_fixture_covers_all_input_state` 退 0

- [ ] **W-B4-9.6 `GateInput::from_fixture`（零拷贝借用）** · `crates/application/src/gate/replay.rs`
  - 追溯：AC-B4-46、AC-B4-48 · design §2.3.3（fixture 持有所有权，input 借用）
  - DoD：`GateInput::from_fixture(&'a GateReplayCase) -> GateInput<'a>`；`topic_window_used`/`entity_last_seen_ms` 由 `Vec` 构造为 `BTreeMap`（排序去重）；`EntityKey` 32 hex 校验失败即 `GateError::InvalidEntityKey`；P1 阶段只要求最小可用（200 例在 W-B4-28.3 完成）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_replay from_fixture_builds_input_without_owning_text` 退 0

- [ ] **W-B4-9.7 策略非法即 fail-closed（`PolicyUnavailable`）** · `crates/application/src/gate/api.rs`
  - 追溯：AC-B4-08 · spec §4.2-1、§5.1.3-1 · design §2.1.3.2-0
  - DoD：`policy.validate()` 失败时 `evaluate_gate` 返回 `Deny{PolicyUnavailable, stopped_at:G1}`，五级 `calls` 全 0、`result=NotReached`；不 panic；`gate_deny_total` +1
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter invalid_policy_fails_closed` 退 0

### W-B4-10 `GateClock` / `FixedClock` / 生产镜像时钟

> 前置：W-B4-6。**门控不读 `Instant::now`/`SystemTime::now`。**

- [ ] **W-B4-10.1 `GateClock` trait（毫秒纯取值）** · `crates/application/src/gate/clock.rs`
  - 追溯：AC-B4-07、AC-B4-43 · spec §1.2-9、§6.1-10 · design §2.2.2-B
  - **P0 状态**：`GateClock` 与 `FixedClock` 已在 W-B4-6.2 落地；本节只做签名复核与"trait 内无系统时间 API"的静态断言，不重写类型。
  - DoD：`pub trait GateClock: Send + Sync { fn now_ms(&self) -> u64; }`；无 `Result`、无副作用；trait 内不出现系统时间 API
  - 证据：`"$CARGO" build -p meeting-copilot-application` 退 0

- [ ] **W-B4-10.2 `FixedClock{anchor_ms, tick_ms}`（回放确定性）** · `crates/application/src/gate/clock.rs`
  - 追溯：AC-B4-48 · design §1.2.7、§2.3.3
  - DoD：第 n 次取值 == `anchor_ms + n × tick_ms`；第 0 次 == `anchor_ms`；内部只用 `Cell<u64>`（无 `Mutex`、无原子、无 await）；同构造两次运行序列完全相同
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_replay fixed_clock_is_deterministic_sequence` 退 0

- [ ] **W-B4-10.3 生产镜像时钟（装配方推进）** · `crates/application/src/gate/clock.rs`
  - 追溯：AC-B4-43 · design §2.2.2-B（`AtomicU64` 镜像）
  - **P1 口径提示（与 §3.6.1 冲突处置）**：`--rule=source` 要求 `gate/**` 内 `AtomicU64`/`Mutex`/`RwLock` 命中 0（AC-B4-43 优先）。若 `MirrorClock` 必须用 `AtomicU64`，应放在 `gate/` 之外的装配模块并以 `GateClock` 实现注入；不得为通过该任务放宽 `--rule=source`。
  - DoD：`MirrorClock` 从 `AtomicU64` 读取外部推进值，`advance_to(ms)` 只增不减；不读 `Instant`/`SystemTime`；单调性有断言
  - 证据：`! grep -qE "Instant::now|SystemTime::now" crates/application/src/gate/clock.rs` 判定通过

- [ ] **W-B4-10.4 时钟纯性静态断言** · `crates/application/src/gate/**`
  - 追溯：AC-B4-07 · spec §4.1-4、§4.2-2 · design §2.1.2
  - DoD：`gate/**` 中真实计时、线程睡眠、`std::env`、网络类型命中 **0**；单次判定内无 `sleep`、无跨级持锁
  - 证据：`bash scripts/check-b4-gates.sh --rule=source` 退 0

### W-B4-11 G1 六类规则与基础分（AC-01/06/07）

> 前置：W-B4-9、W-B4-10。产出 `gate/prefilter.rs` 的 `run`（实现体名不得为 4 个公开名）。

- [ ] **W-B4-11.1 `GapRule` 7 变体与基础分（毫值）** · `crates/domain/src/model/gate.rs`
  - 追溯：AC-B4-01 · spec §5.1.1-1 · design §2.1.3.3-B
  - DoD：`ProperNoun=700`、`Abbreviation=650`、`Institution=600`、`PolicyStandard=800`、`Metric=750`、`UnansweredQuestion=850`、`Acronym=650`；`ALL` 长度 7；`base_milli()` 为 `const fn`
  - 证据：`"$CARGO" test -p meeting-copilot-domain gap_rule_base_scores_match_spec` 退 0

- [ ] **W-B4-11.2 六规则各一条正例 → 6 候选** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-01 · spec §5.1.1-1a · design §2.1.3.3-B
  - DoD：六类各一条正例输入 → 候选数 **6**，`kind` 与基础分逐项匹配（700/650/600/800/750/850）；`confidence >= 0.50`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter prefilter_scores_six_gap_rules` 退 0

- [ ] **W-B4-11.3 `Institution → ProperNoun`，`GapKind` 仍 6 变体** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-06 · spec §5.1.1-2 · design §2.3.2-A（J-08 / R-12）
  - DoD：`GapRule::Institution` 命中时 `GapCandidate.kind == proper_noun`；`GapKind` 变体数 == 6（不得新增"机构/职务"）；`GapRule` 不进入 `GapCandidate.kind` 的序列化值
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter gap_kind_is_closed_enum_and_institution_maps_to_proper_noun` 退 0

- [ ] **W-B4-11.4 缩写白名单可配置** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-01 · spec §5.1.1-1 · design §2.1.3.3-B
  - DoD：`acronym_whitelist` 含某缩写 → 该缩写不产候选；移除后产候选；白名单为空不 panic
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter acronym_whitelist_is_configurable` 退 0

- [ ] **W-B4-11.5 固定排序 + `dedupe_key` 32 hex + 100 次一致** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-06、AC-B4-07 · spec §4.2-2 · design §2.1.3.3-B（排序三元组）
  - DoD：输出排序 = `confidence` 降序 → `trigger_span.start_ms` 升序 → `dedupe_key` 字典序；`DedupeKey` 长度 32 且全 hex；同输入 100 次逐字段相等；`trigger_span.start < end`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter prefilter_is_pure_and_deterministic` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_prefilter dedupe_key_is_32_hex` 退 0

### W-B4-12 G1 首现 / 敏感 / 10s / 2s / 置信（AC-02~05）

> 前置：W-B4-11。

- [ ] **W-B4-12.1 首现三重并集** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-02 · spec §5.1.1-3、§5.1.3-2 · design §2.1.3.3-B
  - DoD：同一实体二次出现 → 候选数 0 且 G1 计数不增（交给 G3）；术语表为空时退化为"本次会话首现"，仍产候选且不 panic
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter first_seen_suppresses_second_candidate` 退 0

- [ ] **W-B4-12.2 敏感术语永不成为种子** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-02 · spec §5.1.1-4a · design §2.1.3.3-B
  - DoD：输入仅含 `sensitive=true` 术语 → 候选数 **0** 且 `gate_l0_sensitive_seed_blocked == 1`；敏感术语不进入查询构造（`detect_gaps` 输出中 0 次）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter sensitive_term_is_not_a_seed` 退 0

- [ ] **W-B4-12.3 未答提问 9 999 / 10 000 ms 边界** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-03 · spec §5.1.1-5a · design §2.1.3.3-B（`now - asked >= 10_000`）
  - DoD：9 999 ms 且无回应 → 候选 **0**；10 000 ms → **1**；9 999 ms 内出现回应 → 0 且 `gate_l0_question_answered_skipped == 1`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter unanswered_question_yields_after_ten_seconds` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_prefilter answered_question_is_skipped` 退 0

- [ ] **W-B4-12.4 `confidence` 0.49 / 0.50 边界** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-04 · spec §5.1.1-6a · design §2.1.3.3-B
  - DoD：片段 `confidence=0.49` → 0 候选且 `gate_l0_low_confidence == 1`；`0.50` → 1 候选；候选侧 `confidence=0.49` 同样 0 候选
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter below_half_gap_confidence_yields_no_candidate` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_prefilter exactly_half_gap_confidence_passes` 退 0

- [ ] **W-B4-12.5 稳定前缀后 1 999 / 2 000 ms** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-05 · spec §5.1.1-7 · design §2.1.3.3-B（`stable_prefix + 2_000 <= now`）
  - DoD：1 999 ms → 候选 **0**；2 000 ms → **1**；以虚拟时钟断言，不测真实 p95
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter candidate_is_ready_within_two_seconds_of_stable_prefix` 退 0

### W-B4-13 G1 十一项硬闸门（AC-08）

> 前置：W-B4-12。**一个输入只产一个原因码，按固定优先级取首个。**

- [ ] **W-B4-13.1 11 项优先级固定为 1..11** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-08 · spec §5.1.1-8 · design §2.1.3.3-A
  - DoD：优先级顺序 = design 表 #1..#11（`AutoDisabled`→`ConsentMissing`→`Paused`→`SilentPeriod`→`ScreenShareProtection`→`PreFilterLowConfidence`→`Suppressed`→`QuotaExhausted`→`Duplicate`→`NoProviderEnabled`→`SensitiveTerm`）；同时命中两项时只返回优先级靠前者；`G1` 只看会话级状态（主题/来源/实体抑制不在 G1）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter hard_gate_priority_returns_first_only` 退 0

- [ ] **W-B4-13.2 11 种条件 → 11 个不同原因码** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-08 · spec §5.1.1-8 · design §2.1.3.3-A
  - DoD：依次注入 11 种条件 → 11 个**互不相同**的 `GateRejectReason`，各出现 **1** 次；`stopped_at == G1`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter prefilter_denies_eleven_hard_cases` 退 0

- [ ] **W-B4-13.3 短路：G2~G5 `calls=0` 且 `NotReached`** · `crates/application/src/gate/api.rs`
  - 追溯：AC-B4-08 · spec §4.1-2、§6.4-3 · design §2.1.3.2-1
  - DoD：任一 `Deny` 后，`stopped_at` 之后各级 `StageTrace.calls == 0` 且 `result == NotReached`；`per_stage` 数组长度恒 5
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter deny_short_circuits_later_stages` 退 0

- [ ] **W-B4-13.4 G1 阶段外发/检索/渲染次数恒 0** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-08、AC-B4-35 · spec §5.1.1-10a · design §2.1.3.3
  - DoD：G1 路径 `outbound_attempts == 0`、`llm_calls == 0`、`network_calls == 0`；11 种硬闸门用例内三者全 0
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter prefilter_denies_eleven_hard_cases` 退 0（用例内含上述断言）

### W-B4-14 `gate_prefilter` target 与 AC-01~08 全绿

> 前置：W-B4-11~13。

- [ ] **W-B4-14.1 target 落位 + `NEW_TARGETS` 第 1 项（13→14）** · `crates/application/tests/gate_prefilter.rs`、`scripts/test-name-inventory.sh`
  - 追溯：AC-B4-39 · spec §7.9-1/2 · design §2.6-P1 产出
  - DoD：target 文件存在；`NEW_TARGETS` 追加 `gate_prefilter`，长度 14；`--rule=targets` 对已登记项通过
  - 证据：`grep -c "gate_prefilter" scripts/test-name-inventory.sh` ≥1；`bash scripts/check-b4-gates.sh --rule=targets; echo "exit=$?"`（P1 阶段允许因缺其余 5 项退 1，但日志不得含 `gate_prefilter` 缺失）

- [ ] **W-B4-14.2 AC-01~08 的 15 条用例全绿** · `crates/application/tests/gate_prefilter.rs`
  - 追溯：AC-B4-01~08 · design §2.5.1
  - DoD：`cargo test -p meeting-copilot-application --test gate_prefilter` 退出 **0**，用例名与 §0.6 矩阵逐一对应；无 `#[ignore]`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-14.3 P1 出口静态复验** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-07、AC-B4-43 · design §2.6-P1 出口
  - DoD：`--rule=source` = 0；`gate/**` 中 `InsightDraft`/`InsightStore` = 0；`api.rs` 的 `pub fn` 计数仍为 4（未因 G1 实现被污染）
  - 证据：`bash scripts/check-b4-gates.sh --rule=source && [ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ]; echo "exit=$?"` → `exit=0`

---

## P2 · G2 相关性 + G3 时效/去重 + G4 预算与冷却

### W-B4-15 G2 加权公式与整数衰减（AC-09/10/12）

> 前置：W-B4-14。全部整数运算，禁浮点入参与 `NaN`。

- [ ] **W-B4-15.1 400/250/200/150 千分位加权 + 四舍五入** · `crates/application/src/gate/relevance.rs`
  - 追溯：AC-B4-09 · spec §5.2.1-1 · design §2.1.3.4
  - DoD：`base = (400·s + 250·e + 200·g + 150·t + 500) / 1000`；四注入值全 1000 且 `Δt=0` → `score_milli == 1000`；权重常量可被测试逐项断言（400/250/200/150）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_relevance relevance_weights_match_spec` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_relevance relevance_full_score_is_thousand` 退 0

- [ ] **W-B4-15.2 整数衰减表（180 s → 367/368‰，0 s → 1000‰）** · `crates/application/src/gate/relevance.rs`
  - 追溯：AC-B4-10 · spec §5.2.1-2 · design §2.1.3.4（180 桶 + 线性插值，禁 `f64::exp`）
  - DoD：`decay_milli(180_000) ∈ {367, 368}`；`decay_milli(0) == 1000`；衰减值对同一 `delta_ms` 恒定；`gate/relevance.rs` 中 `f64`/`f32` 命中 **0**
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_relevance time_decay_at_180_seconds_is_367_milli` 退 0；`! grep -qE "\bf32\b|\bf64\b" crates/application/src/gate/relevance.rs` 判定通过

- [ ] **W-B4-15.3 六分项必须同时输出** · `crates/application/src/gate/relevance.rs`
  - 追溯：AC-B4-12 · spec §5.2.1-4 · design §2.3.2-A
  - DoD：`RelevanceScore` 六字段全部存在且序列化键名稳定；任一字段缺失即编译失败（结构体非 `Option`）；`score_milli ∈ 0..=1000`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_relevance relevance_parts_are_all_reported` 退 0

- [ ] **W-B4-15.4 越界钳制（不新增计数项）** · `crates/application/src/gate/relevance.rs`
  - 追溯：AC-B4-09、AC-B4-13 · spec §5.2.3-1 · design §2.2.2-A（钳制到 [0,1000]）
  - DoD：注入 >1000 或负值被钳制到边界；不 panic、不返 `NaN`；不新增计数项（32 项固定）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_relevance out_of_range_parts_are_clamped` 退 0

### W-B4-16 G2 阈值档位与低相关拒绝（AC-11/13/14）

> 前置：W-B4-15。

- [ ] **W-B4-16.1 720/620/520 与 619/620 边界** · `crates/application/src/gate/relevance.rs`
  - 追溯：AC-B4-11 · spec §5.2.1-3a · design §2.1.3.4
  - DoD：`Strict=720`、`Balanced=620`（默认）、`Loose=520`；平衡档 `score_milli=619` → `Deny`、`620` → `Allow`；非法档位由 `validate()` 拒绝并回落 620
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_relevance threshold_profiles_are_072_062_052` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_relevance balanced_threshold_boundary` 退 0

- [ ] **W-B4-16.2 低相关 → `Deny{LowRelevance}` + 计数 +1** · `crates/application/src/gate/relevance.rs`
  - 追溯：AC-B4-13 · spec §5.2.1-5 · design §2.1.3.2-G2
  - DoD：`score < threshold` → `Deny{LowRelevance, stopped_at:G2}`，`gate_l2_low_relevance == 1`；G3~G5 `calls == 0`；不出卡（无 `OutboundIntent`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_relevance low_relevance_is_denied` 退 0

- [ ] **W-B4-16.3 空主题/空实体按 0，不 panic** · `crates/application/src/gate/relevance.rs`
  - 追溯：AC-B4-13 · spec §5.2.1-6 · design §2.1.3.4
  - DoD：`semantic=0`、`entity=0`；除零不可达（整数运算、无除法分母为 0 风险）；100 次结果一致
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_relevance empty_topic_is_deterministic` 退 0

- [ ] **W-B4-16.4 无 LLM/无网络 + 中英混排 100 次一致** · `crates/application/src/gate/relevance.rs`
  - 追溯：AC-B4-14 · spec §5.2.1-7a · design §2.1.3.4
  - DoD：G2 路径 `llm_calls == 0`、`network_calls == 0`；中英混排输入 100 次 `score_milli` 与六分项逐字段相等；`gate/relevance.rs` 无网络/环境变量符号
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_relevance relevance_never_calls_llm_or_network` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_relevance relevance_is_deterministic_for_mixed_text` 退 0

### W-B4-17 G3 语义聚类与去重键（AC-15/16）

> 前置：W-B4-14。`gate/dedupe.rs` 实现体名 `apply`。

- [ ] **W-B4-17.1 聚类阈值 880‰（879 不归 / 880 归）** · `crates/application/src/gate/dedupe.rs`
  - 追溯：AC-B4-15 · spec §5.3.1-1 · design §2.1.3.5-①
  - DoD：注入相似度 880 → 同 `topic_cluster_id`；879 → 不同；缺失相似度快照时用 `domain::merge::ngram_cosine_milli` 兜底（`ClusterSource::Fallback`），命中注入时为 `Injected`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe semantic_cluster_threshold_is_088` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_dedupe cluster_boundary_08799_08800` 退 0

- [ ] **W-B4-17.2 去重键 599 999 / 600 000 ms** · `crates/application/src/gate/dedupe.rs`
  - 追溯：AC-B4-16 · spec §5.3.1-2a · design §2.1.3.5-②
  - DoD：同键间隔 599 999 ms → `Deny{Duplicate}` 且 `gate_l3_duplicate == 1`；600 000 ms → `Allow`；非法键（非 32 hex）→ `Deny{PolicyUnavailable}`，不 panic
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe duplicate_key_blocks_within_ten_minutes` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_dedupe duplicate_key_expires_at_ten_minutes` 退 0

- [ ] **W-B4-17.3 聚类来源标记（不新增计数项）** · `crates/domain/src/model/gate.rs`
  - 追溯：AC-B4-15 · design §2.1.3.5（`topic_cluster_source` 裁决）
  - DoD：`ScoredCandidate.topic_cluster_source ∈ {Injected, Fallback}` 可序列化；32 项计数集合不变；`GapCandidate` 零改
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe cluster_source_is_reported_for_fallback` 退 0

### W-B4-18 G3 配额 / 冷却 / 抑制 / 缓存 / 收敛（AC-17~21）

> 前置：W-B4-17。

- [ ] **W-B4-18.1 主题 ≤2 / 全局 ≤3（10 分钟）** · `crates/application/src/gate/dedupe.rs`
  - 追溯：AC-B4-17 · spec §5.3.1-3 · design §2.1.3.5-③
  - DoD：单主题第 3 次 → `Deny{QuotaExhausted}` 且 `gate_l3_quota == 1`；全局第 4 次 → 同样拒绝；窗口按 600 000 ms
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe topic_quota_is_two_per_ten_minutes` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_dedupe global_quota_is_three_per_ten_minutes` 退 0

- [ ] **W-B4-18.2 实体冷却 300 000 ms + `cooldown_until_ms`** · `crates/application/src/gate/dedupe.rs`
  - 追溯：AC-B4-18 · spec §5.3.1-4 · design §2.1.3.5-④
  - DoD：299 999 ms → `Deny{Duplicate}` 且 `cooldown_until_ms == last_seen + 300_000`；300 000 ms → `Allow`；`cooldown_until_ms` 写入 `GateRecord` 与 `GateDecision`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe entity_cooldown_is_five_minutes` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_dedupe entity_cooldown_expires_at_five_minutes` 退 0

- [ ] **W-B4-18.3 抑制三类：自动拒 / 按需放行 + 提示** · `crates/application/src/gate/dedupe.rs`
  - 追溯：AC-B4-19 · spec §5.3.1-5 · design §2.1.3.5-⑤
  - DoD：主题/来源/实体任一类命中：`trigger=Auto` → `Deny{Suppressed}` 且 `gate_l3_suppressed == 1`；`trigger=OnDemand` → `Allow` 且 `suppressed_notice == true`（不得静默拦截）；抑制存储只读（写入 0）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe suppression_blocks_auto_allows_on_demand` 退 0

- [ ] **W-B4-18.4 缓存 86 400 000 / 86 400 001 ms** · `crates/application/src/gate/dedupe.rs`
  - 追溯：AC-B4-20 · spec §5.3.1-6a · design §2.1.3.5-⑥
  - DoD：80 000 000 ms 与 86 400 000 ms → `Allow`；86 400 001 ms 且 `refresh_available=false` → `Deny{Stale}` 且 `gate_l3_stale == 1`；有刷新来源时放行
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe fresh_cache_is_allowed` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_dedupe stale_cache_is_denied` 退 0

- [ ] **W-B4-18.5 候选收敛：10 全进 / 11 取 3 丢 8** · `crates/application/src/gate/dedupe.rs`
  - 追溯：AC-B4-21 · spec §5.3.1-7 · design §2.1.3.5-⑦
  - DoD：候选 **10** → `kept=10`、`dropped=0`；候选 **11** → `kept=3`、`dropped=8`、`gate_l3_quota` 增量 **8**；排序 `score` 降序 → `dedupe_key` 升序
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe candidate_convergence_keeps_top_three` 退 0

- [ ] **W-B4-18.6 `dedupe_enabled=false` 只关四步，抑制/缓存仍生效** · `crates/application/src/gate/dedupe.rs`
  - 追溯：AC-B4-19、AC-B4-20 · spec §1.6（仅 G3-dedupe 可关） · design §2.1.3.9
  - DoD：关闭后跳过 ①去重键 ③配额 ④冷却 ⑦收敛；⑤抑制与⑥缓存年龄**仍生效**；`skipped_stages` 含 `G3`、`gate_stage_skipped_total` +1；`G3` 计数不因跳过而虚增
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe dedupe_switch_keeps_suppression_and_cache` 退 0

### W-B4-19 G4 预算独立性与上限（AC-22/23/24）

> 前置：W-B4-18。`gate/budget.rs` 实现体名 `check`。**只读快照，推进在判定外。**

- [ ] **W-B4-19.1 与 `LLM_LIVE_CALL_CAP` 零耦合** · `crates/application/src/gate/budget.rs`
  - 追溯：AC-B4-22 · spec §1.7、§5.4.1-1 · design §2.1.3.6、§2.1.3.9
  - DoD：`gate/**` 与 `gate*.rs` 中 `LLM_LIVE_CALL_CAP`、`LLM_`、`std::env` 命中 **0**；该变量缺失或为 0 时同一输入判定结果逐字段不变（对照用例两次运行）
  - 证据：`bash scripts/check-b4-gates.sh --rule=budget` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_budget budget_is_independent_of_llm_live_cap` 退 0

- [ ] **W-B4-19.2 会话上限第 120 次放行 / 第 121 次拒绝** · `crates/application/src/gate/budget.rs`
  - 追溯：AC-B4-23 · spec §5.4.1-2 · design §2.1.3.6-④
  - DoD：`session_outbound_used=119` → `Allow`（`budget_remaining` 递减）；`=120` → `Deny{BudgetExhausted}` 且 `gate_l4_budget_exhausted == 1`；cap 可配 0..=600，`cap=0` 时首次即拒
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_budget session_outbound_cap_is_120` 退 0

- [ ] **W-B4-19.3 整数令牌桶 3 000 毫令牌 + 档位 1/3/5** · `crates/application/src/gate/budget.rs`
  - 追溯：AC-B4-24 · spec §5.4.1-3/4 · design §2.1.3.6（refill 整数公式）
  - DoD：容量上限 **3000**；补充 `per_window/600_000`；档位 `Low=1`/`Medium=3`/`High=5`；第 4 次（默认档）→ `Deny{BudgetExhausted}`；`tokens_milli` 为 `u16`、无浮点
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_budget token_bucket_capacity_and_refill` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_budget frequency_profiles_are_1_3_5` 退 0

- [ ] **W-B4-19.4 `GateBudget::commit` 判定外推进 + `session_remaining`** · `crates/domain/src/model/gate.rs`
  - 追溯：AC-B4-23、AC-B4-27 · design §2.1.3.6（关键裁决 1）、§2.3.3-3
  - DoD：`evaluate_gate` 不修改传入 `GateBudget`（`&` 只读）；`commit(&mut self, &GateDecision)` 只在 `Allow` 时推进 `session_outbound_used +1`、窗口 +1、令牌扣减；`session_remaining_with(cap)` 用 saturating；`Deny` 时 `commit` 零变化
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_budget evaluate_gate_does_not_mutate_budget` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_budget commit_advances_only_on_allow` 退 0

### W-B4-20 G4 冷却、离线、恢复与排队（AC-25/26/27）

> 前置：W-B4-19。

- [ ] **W-B4-20.1 冷却 → `Deny{GatewayCooling}`，0 调用 0 重试** · `crates/application/src/gate/budget.rs`
  - 追溯：AC-B4-25 · spec §5.4.1-6 · design §2.1.3.6-③
  - DoD：`llm.cooling=true` → `Deny{GatewayCooling}`，`gate_l4_gateway_cooling == 1`，`llm_calls == 0`、`retries == 0`；`cooldown_until_ms` 取注入值；单次判定只读能力快照一次
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_budget gateway_cooling_denies_without_retry` 退 0

- [ ] **W-B4-20.2 冷却不阻塞片段准入** · `crates/application/tests/gate_budget.rs`
  - 追溯：AC-B4-26 · spec §5.4.1-7 · design §2.1.3.6（关键裁决 3）
  - DoD：冷却期间 B3 准入替身被调用 ≥1 次（`admitted_during_cooldown >= 1`）；门控不引入熔断器、不 sleep、不阻塞调用线程
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_budget gateway_cooling_does_not_block_admission` 退 0

- [ ] **W-B4-20.3 推进 600 001 ms 预算恢复** · `crates/application/src/gate/budget.rs`
  - 追溯：AC-B4-27 · spec §5.4.1-8 · design §2.1.3.6（恢复）
  - DoD：窗口起点后 600 001 ms → 令牌补满、`Allow` 恢复；599 999 ms 仍拒；冷却由能力位转 false 恢复（`Deny` → `Allow`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_budget budget_recovers_after_window` 退 0

- [ ] **W-B4-20.4 离线拒绝 / `Unknown` 可尝试 / 第 4 并发 `queued=true`** · `crates/application/src/gate/budget.rs`
  - 追溯：AC-B4-27 · spec §1.8-5、§5.4.3-3 · design §2.1.3.6-①⑧
  - DoD：`Offline` → `Deny{Offline}` 且 `gate_l4_offline == 1`、外发 0；`Unknown` → 不因网络拒绝；`inflight >= 3` → `Allow{queued=true}`（**不拒绝、不丢候选**），判定侧只读 `inflight`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_budget offline_and_unknown_network_paths` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_budget fourth_concurrent_request_is_queued` 退 0

### W-B4-21 C-03 `OutboundLease`（3 槽 + 单排队位）

> 前置：W-B4-20。`gate/lease.rs`。判定与租约分离。

- [ ] **W-B4-21.1 固定 3 槽 + 单调 `LeaseId`** · `crates/application/src/gate/lease.rs`
  - 追溯：AC-B4-27 · design §2.3.4（C-03 裁决）
  - DoD：`slots: [Option<LeaseId>; 3]`（固定数组，禁 `HashMap`）；`LeaseId(u64)` 单调 +1；`try_acquire` 返回 `Acquired{lease_id, inflight}`；`inflight ∈ 0..=3`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_budget lease_acquires_three_slots_then_queues` 退 0

- [ ] **W-B4-21.2 第 4 → `Queued{position:1, queue_len:1}`；第 5 不增长** · `crates/application/src/gate/lease.rs`
  - 追溯：AC-B4-27 · spec §5.4.1-5、§5.4.3-3 · design §2.3.4
  - DoD：第 4 次 `Queued{position:1, queue_len:1}`；第 5 次仍 `queue_len == 1`（不增长、不拒绝）；`queue.len() <= 1` 恒成立
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_budget fifth_request_does_not_grow_queue` 退 0

- [ ] **W-B4-21.3 `release` 提升队首到最小空槽** · `crates/application/src/gate/lease.rs`
  - 追溯：AC-B4-27 · design §2.3.4（release 伪代码）
  - DoD：按 `lease_id` 找到槽位清空；队首提升到**最小空槽索引**；返回被提升的 `LeaseId`；未知 `lease_id` 返回 `None` 且不改变槽位
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_budget release_promotes_queue_head_to_lowest_free_slot` 退 0

- [ ] **W-B4-21.4 无锁 / 无 await / 无定时器** · `crates/application/src/gate/lease.rs`
  - 追溯：AC-B4-27 · spec §4.1-4 · design §2.3.4（"为什么不用计数信号量"）
  - DoD：`lease.rs` 中 `Mutex`/`RwLock`/`async`/`await`/`tokio`/`thread::sleep` 命中 **0**；`try_acquire`/`release` 均为 `&mut self` 同步函数
  - 证据：`! grep -qE "Mutex|RwLock|async |\.await|tokio|thread::sleep" crates/application/src/gate/lease.rs` 判定通过

### W-B4-22 G3/G4 同源窗口（design P2 依赖风险）

> 前置：W-B4-18、W-B4-19。**同一窗口数据，两处复核，不得各建一套。**

- [ ] **W-B4-22.1 窗口状态单一数据源（只读 `GateBudget`）** · `crates/application/src/gate/{dedupe,budget}.rs`
  - 追溯：AC-B4-17、AC-B4-18、AC-B4-23 · design §2.6-P2（依赖风险）、§2.1.3.6-⑥⑦
  - DoD：`dedupe::apply` 的窗口查询只接受 `&GateBudget`（不得自建 `BTreeMap`）；`budget::check` 复核同源字段；两处对"主题第 3 次""实体 299 999 ms"的结论一致
  - 证据：`! grep -q "BTreeMap::new" crates/application/src/gate/dedupe.rs` 判定通过

- [ ] **W-B4-22.2 单点修改、两处同步变化（行为断言）** · `crates/application/tests/gate_dedupe.rs`
  - 追溯：AC-B4-17、AC-B4-18 · design §2.6-P2
  - DoD：同一 `GateBudget` 快照分别喂给 G3 与 G4：主题第 3 次在两处均拒；把 `topic_window_used` 改为 0 后两处均放行（证明无第二份窗口）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe single_window_drives_both_stages` 退 0

### W-B4-23 三个 target 与 AC-09~27 全绿

> 前置：W-B4-15~22。

- [ ] **W-B4-23.1 `NEW_TARGETS` 追加 3 项（13→17）** · `scripts/test-name-inventory.sh`
  - 追溯：AC-B4-39 · spec §7.9-2 · design §2.6-P2 产出
  - DoD：`NEW_TARGETS` 含 `gate_prefilter`、`gate_relevance`、`gate_dedupe`、`gate_budget`（长度 17）；三个 target 文件均存在
  - 证据：`for t in gate_prefilter gate_relevance gate_dedupe gate_budget; do grep -q "$t" scripts/test-name-inventory.sh || exit 1; done; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-23.2 AC-09~27 的 25 条用例全绿** · `crates/application/tests/gate_{relevance,dedupe,budget}.rs`
  - 追溯：AC-B4-09~27 · design §2.5.2~2.5.4
  - DoD：三个 target 分别退出 **0**；用例名与 §0.6 矩阵逐一对应；无 `#[ignore]`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_relevance && "$CARGO" test -p meeting-copilot-application --test gate_dedupe && "$CARGO" test -p meeting-copilot-application --test gate_budget; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-23.3 P2 出口静态复验** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-22、AC-B4-43 · design §2.6-P2 出口
  - DoD：`--rule=source` = 0；`--rule=budget` = 0；`gate/**` 中 `LLM_`/`std::env`/`reqwest`/真实计时命中 0；`api.rs` `pub fn` 计数仍 4
  - 证据：`bash scripts/check-b4-gates.sh --rule=source && bash scripts/check-b4-gates.sh --rule=budget && [ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ]; echo "exit=$?"` → `exit=0`

---

## P3 · G5 授权/审计 + 回放 + 回归与文档

### W-B4-24 G5 七字段授权（AC-28/29/30/32）

> 前置：W-B4-21（G4 `Allow` 路径）。`gate/outbound.rs` 实现体名 `authorize`。

- [ ] **W-B4-24.1 7 字段齐备 + `is_complete` fail-closed** · `crates/domain/src/model/gate.rs`
  - 追溯：AC-B4-28 · spec §6.10、§5.5.1-1 · design §2.3.2-A
  - DoD：`status/granted_at_ms/content_types/max_query_chars/provider_ids/notice_version/revoked_at_ms` 7 字段齐备；任一缺失 → `is_complete()==false`；`status != Granted` 时 `is_complete()` 即使字段齐备也为 false；`content_types` 仅 `TextQuery`（含 `Audio` 即非法，类型上不可表达）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound authorization_records_seven_required_fields` 退 0

- [ ] **W-B4-24.2 未授权 → 0 外发** · `crates/application/src/gate/outbound.rs`
  - 追溯：AC-B4-29 · spec §1.8-2、§5.5.1-2 · design §2.1.3.7-①②
  - DoD：`status ∈ {Revoked, Expired, Missing}` → `Deny{OutboundNotGranted}`，`outbound_attempts == 0`，`gate_l5_not_granted == 1`；自动与按需两条路径计数均为 0
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound unauthorized_outbound_is_zero` 退 0

- [ ] **W-B4-24.3 撤销不可自恢复（四入口）+ 重授权递增** · `crates/application/src/gate/outbound.rs`
  - 追溯：AC-B4-30 · spec §1.8-3、§5.5.1-3 · design §2.1.3.8（授权状态机）
  - DoD：`revoked_at_ms.is_some()` 后，启动/升级/会话开始/策略重载四个入口读入的记录 `status` 恒为 `Revoked`（不写 `Granted`）；恢复需新授权：`granted_at_ms` 更新且 `notice_version` 严格递增；`notice_version` 不递增的"新授权"被判非法
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound revoked_authorization_is_not_auto_restored` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_outbound reauthorization_bumps_notice_version` 退 0

- [ ] **W-B4-24.4 变更即失效（新增源 / 提高上限）** · `crates/application/src/gate/outbound.rs`
  - 追溯：AC-B4-32 · spec §1.8-4、§6.10-5 · design §2.1.3.7-④
  - DoD：`provider_ids` 非空且不是当前启用源子集 → `Deny{ProviderNotAllowed}` 且 `gate_l5_provider_or_domain_blocked == 1`；`max_query_chars` 与当前档位不一致 → `status=Expired` → 下一次 `Deny{OutboundNotGranted}`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound authorization_change_invalidates_old_grant` 退 0

### W-B4-25 【顺序风险 R-07】先截断再脱敏（AC-31）

> 前置：W-B4-24。**顺序不可交换**：`状态 → 范围 → 截断 → 脱敏 → 审计 → 放行`。

- [ ] **W-B4-25.1 `truncate_chars` 80/200 + `truncated` 标记** · `crates/application/src/gate/outbound.rs`
  - 追溯：AC-B4-31 · spec §5.5.1-4a · design §2.1.3.7-⑤
  - DoD：自动 81 字 → 输出 80 字且 `truncated=true`；按需 201 字 → 200 字；按 `chars().count()` 截断（与 `ShortText`/`SanitizedQuery` 同口径，不按字节）；未超长时 `truncated=false`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound auto_query_is_capped_at_80` 退 0；`"$CARGO" test -p meeting-copilot-application --test gate_outbound on_demand_query_is_capped_at_200` 退 0

- [ ] **W-B4-25.2 脱敏替身收到的入参长度 == 80/200（顺序硬断言）** · `crates/application/tests/gate_outbound.rs`
  - 追溯：AC-B4-31 · design §2.1.3.7（关键裁决 2）、R-07
  - DoD：`SpySanitizer` 记录 `received_len`；自动路径断言 `received_len == 80`（**不是 81**）、按需 `== 200`（**不是 201**）；`call_count == 1`；`OutboundIntent.sanitized_query_len` 与 `received_len` 相等
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound sanitizer_receives_truncated_input_only` 退 0

- [ ] **W-B4-25.3 超长串 0 次到达脱敏器（先脱敏即失败）** · `crates/application/tests/gate_outbound.rs`
  - 追溯：AC-B4-31 · spec §5.5.1-4 · design R-07
  - DoD：构造超过 200 字的输入：脱敏替身 `call_count == 1` 且 `received_len <= 200`；"直接交给脱敏器"路径的 `call_count` 为 0（负证明：若先脱敏则 `received_len > 200`，断言立即失败）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound oversize_never_reaches_sanitizer` 退 0

### W-B4-26 【顺序风险 R-08】先审计再放行（AC-34/35/36）

> 前置：W-B4-25。**审计永远在放行之前**；审计不可用与 `AuditGuard` 降级都归一为 `AuditUnavailable`，但文案不同。

- [ ] **W-B4-26.1 审计不可用 → `Deny` + 0 外发 + 缓冲 ≥1** · `crates/application/src/gate/outbound.rs`
  - 追溯：AC-B4-35 · spec §1.8-6、§5.5.1-8 · design §2.1.3.7-⑦
  - DoD：`input.audit_available=false` → `Deny{AuditUnavailable}`，`outbound_attempts == 0`，`gate_l5_audit_unavailable == 1`，缓冲长度 ≥1；不调用 `append`；用户文案**不**冒用 `E-PRIV-004`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound audit_unavailable_blocks_outbound` 退 0

- [ ] **W-B4-26.2 `append` 返回 `Err` 同样 fail-closed** · `crates/application/src/gate/outbound.rs`
  - 追溯：AC-B4-35 · spec §5.5.3-2 · design §2.1.3.7（关键裁决 5）
  - DoD：`audit_available=true` 但 `GateAuditSink::append` 返回 `Err` → `Deny{AuditUnavailable}`，`gate_l5_audit_unavailable == 1`，事件入缓冲，外发 0；`AuditGuard` 降级与 sink 失败归一到同一原因码
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound audit_append_error_blocks_outbound` 退 0

- [ ] **W-B4-26.3 审计调用早于外发提交（顺序硬断言）** · `crates/application/tests/gate_outbound.rs`
  - 追溯：AC-B4-34、AC-B4-35 · spec §5.5.1-11a · design R-08
  - DoD：`SpyAuditSink::append` 回调内读取 `dispatcher.submitted`，断言 == **0**（此刻尚未提交外发）；放行后再断言 == **1**；`append_calls == 1`；若实现"先放行后审计"，该断言立即失败
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound audit_is_appended_before_release` 退 0

- [ ] **W-B4-26.4 脱敏失败 → `Deny` + 审计 1 条 + `E-PRIV-004`** · `crates/application/src/gate/outbound.rs`
  - 追溯：AC-B4-36 · spec §1.8-7、§5.5.3-3 · design §2.1.3.7-⑥（关键裁决 4）
  - DoD：替身 `issue_query` 返回 `Err` → `Deny{SanitizeFailed}`，`outbound_attempts == 0`，`gate_l5_sanitize_failed == 1`，审计 **1** 条（`result=Deny`），`GateError::user_error_code()` 映射既有 `PrivacyViolation`（错误码 `E-PRIV-004`，不新增 `ErrorCode` 变体）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound sanitize_failure_blocks_outbound` 退 0

### W-B4-27 G5 唯一出口与审计白名单（AC-33/34）

> 前置：W-B4-26。

- [ ] **W-B4-27.1 `issue_query`/`AuditLog::append` 仅 1 个文件** · `crates/application/src/gate/outbound.rs`
  - 追溯：AC-B4-33 · spec §5.5.1-6 · design §2.1.3.1-4、R-03
  - DoD：`gate/**` 中 `issue_query` 与 `AuditLog::append` 只出现在 `outbound.rs`（`grep -rln` 输出文件数 == **1**）；`gate/**` 中协议客户端调用 == 0；`proactive/` 不存在时按 D-B4-D02 只扫 `gate/`
  - 证据：`bash scripts/check-b4-gates.sh --rule=outbound` 退 0；`[ "$(grep -rln 'issue_query\|AuditLog::append' crates/application/src/gate | wc -l | tr -d ' ')" = "1" ]` 退 0

- [ ] **W-B4-27.2 审计 7 字段 + 无原文/无完整链接** · `crates/application/src/gate/outbound.rs`
  - 追溯：AC-B4-34 · spec §5.5.1-7 · design §2.2.3（审计口径）
  - DoD：放行 1 次 → 审计 1 条，含授权状态、授权时间、内容类型、长度、源 ID 列表、结果、原因码（+发生时间）；记录中查询原文与完整链接字段 0 出现；`reason` 写 `GateRejectReason` 的 serde 值（`Allow` 时为 `None`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound outbound_audit_has_required_fields_without_text` 退 0

- [ ] **W-B4-27.3 `GateAuditSink` 适配既有 `AuditLog`（不改签名）** · `crates/application/src/gate/outbound.rs`
  - 追溯：AC-B4-34、AC-B4-33 · spec §3.2-6 · design §1.1.2、§2.2.2-B
  - DoD：适配器把 7 字段编码为 `action=<固定动作>;k=v;...` 的 `summary` 传给既有 `AuditLog::append(action, summary)`；`port/governance.rs` 中 `AuditLog`/`Sanitizer` 签名 **零改**；适配器不持有 `AuditGuard` 锁（判定内只读一次布尔快照）
  - 证据：`git diff --quiet HEAD -- crates/domain/src/port/governance.rs; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-27.4 零转录全量检查（源码 + 夹具 + 进度记录）** · `crates/application/src/gate/**`、`crates/application/tests/**`、`docs/progress/b4-status.md`
  - 追溯：AC-B4-34、AC-B4-42 · spec §4.3-1、§7.8 · design §2.1.3.10（日志白名单）
  - DoD：`--rule=zero-transcript` 退 **0**（内置模式表命中 0）；日志只允许 `trace_id`、原因码、计数、耗时毫秒、域名；`GateRecord`/`OutboundIntent`/`GateAuditEvent`/回放夹具中原文与完整链接 0 命中
  - 证据：`bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`

### W-B4-28 回放：4 纯函数 + 3 结构 + 200 例（AC-46/48/49）

> 前置：W-B4-9.6、W-B4-27。**不建 `crates/application/tests/replay/`**；13.3 只依赖 4 函数 + 3 结构。

- [ ] **W-B4-28.1 4 函数计数复验（单文件 == 4）** · `crates/application/src/gate/api.rs`
  - 追溯：AC-B4-46 · spec §1.9 · design R-02
  - DoD：P3 结束时 `api.rs` 仍只有 4 个 `pub fn`；四名匹配计数 == 4；只出现在 1 个文件；`replay_interface_is_io_free` 用例绿（4 函数体内无 IO 调用）
  - 证据：`[ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ]`；`"$CARGO" test -p meeting-copilot-application --test gate_replay replay_interface_is_io_free` 退 0

- [ ] **W-B4-28.2 `GateReplayCase`/`ExpectedDecision`/`ReplayReport` 与 13.3 依赖面收敛** · `crates/domain/src/model/gate_replay.rs`
  - 追溯：AC-B4-48、AC-B4-46 · spec §1.9、§6.8 · design §2.3.2-B
  - **P0 状态**：12 项（含本节 3 结构）已在 W-B4-2.2 落地并导出；本节只做"13.3 依赖面收敛复核"（禁止出现 `OutboundLease`/`GateAuditSink`/`GateCounters`），不重复创建结构。
  - DoD：3 结构齐备；`case_id` 唯一；`expected` 含 `Allow` 或 `Deny{reason, stopped_at}`；`ReplayReport` 含 `total/matched/mismatched/mismatch_cases/seed`；`mismatch_cases` 只存 `case_id`；`gate_replay.rs` 中不出现 `OutboundLease`/`GateAuditSink`/`GateCounters`
  - 证据：`! grep -qE "OutboundLease|GateAuditSink|GateCounters" crates/domain/src/model/gate_replay.rs crates/application/src/gate/replay.rs` 判定通过

- [ ] **W-B4-28.3 200 例固定种子，`total=200` / `mismatched=0`** · `crates/application/tests/gate_replay.rs`
  - 追溯：AC-B4-48 · spec §6.8-5 · design §2.3.3
  - DoD：`GateReplayCase` 恰 **200** 个（覆盖 G1~G5 各拒绝码与 Allow 路径，含 `audit_available=false`）；`run` 结果 `total=200`、`mismatched=0`；`seed` 固定且记录；不联网、不读系统时间
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_replay replay_200_cases_is_byte_identical` 退 0

- [ ] **W-B4-28.4 两次运行 `ReplayReport` 逐字节一致** · `crates/application/tests/gate_replay.rs`
  - 追溯：AC-B4-48 · spec §4.2-2、§6.8-5 · design §2.3.3
  - DoD：同一 200 例连续运行两次，`serde_json` 序列化字节相等（按键序、无 `HashMap` 迭代序）；中途无全局状态残留
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_replay replay_runs_are_byte_identical_twice` 退 0

- [ ] **W-B4-28.5 13.3 本体与 38/39/41.5/42/43 符号 0 命中** · `crates/application/tests/`、`crates/application/src/gate/**`
  - 追溯：AC-B4-49 · spec §1.4-1、§1.11-N-01/N-04 · design §2.1.3.1-4
  - DoD：`crates/application/tests/replay/` **不存在**；`gate/**` 中 38/39/41.5/42/43 相关符号（卡片合成器、校验器、outbox、按需补全、反馈权重）命中 0；`NEW_TARGETS` 追加 `gate_outbound`、`gate_replay` 后长度 **19**（P4 再追加 `gate_keyword` → **20**，见 W-B4-42.2；本条在 P4 时点按 20 复核）
  - 证据：`test ! -d crates/application/tests/replay; echo "exit=$?"` → `exit=0`；`! grep -rqE "composer|validator|outbox|on_demand|feedback_weights" crates/application/src/gate` 判定通过；`grep -c "NEW_TARGETS=" scripts/test-name-inventory.sh` == 1 且该行 target 数 ∈ {19（P3 时点）, 20（P4 后）}

### W-B4-29 B3 兼容与零回写（AC-37/38）

> 前置：W-B4-27。

- [ ] **W-B4-29.1 只读消费 `Insight`（行为断言）** · `crates/application/tests/gate_outbound.rs`
  - 追溯：AC-B4-37 · spec §1.5 · design §1.2.2（三重证明）
  - DoD：`gate_consumes_insights_readonly`：判定前后 `InsightStore` 快照逐字段相等；`insight_write_back == 0`；`GateDecision`/`GateRecord` 中无可写回引用；`manual_locked=true` 的要点正常参与（不影响判定）；`Deleted` 要点不进入（经 `visible_for_relevance` 过滤）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound gate_consumes_insights_readonly` 退 0

- [ ] **W-B4-29.2 零 diff 终验（取证命令）** · `crates/application/src/understand/{store,deduplicator}.rs`
  - 追溯：AC-B4-37 · spec §1.4-3 · design §2.4.2-1
  - DoD：相对 HEAD 零 diff（工作区 + 暂存区）；`--stat` 输出为空；`service.rs`/`insight_ops.rs` 亦零 diff（spec §1.4-3 的更强口径）
  - 证据：`git diff --quiet HEAD -- crates/application/src/understand/store.rs crates/application/src/understand/deduplicator.rs; echo "exit=$?"` → `exit=0`；`git status --porcelain -- crates/application/src/understand/ | wc -l` → `0`

- [ ] **W-B4-29.3 `InsightDraft` 0 命中复验** · `crates/application/src/gate/**`
  - 追溯：AC-B4-38 · spec §1.5 · design §2.3.2-D
  - DoD：`gate/**` 中 `InsightDraft` 命中 **0**；`InsightStore` 命中 **0**；编译期类型断言用例绿
  - 证据：`! grep -rq "InsightDraft" crates/application/src/gate` 判定通过；`"$CARGO" test -p meeting-copilot-application --test gate_prefilter gate_input_has_no_insight_draft` 退 0

### W-B4-30 兼容与红线（AC-39~45）

> 前置：W-B4-29。

- [ ] **W-B4-30.1 全量回归 passed ≥584 / failed=0 / ignored=9** · `crates/**`
  - 追溯：AC-B4-39 · spec §4.5-1 · design §2.5.6
  - DoD：`cargo test --workspace` 退 0；`--list | grep -c ": test"` ≥ **593**；输出中 `passed` ≥584、`failed=0`、`ignored=9`
  - 证据：`"$CARGO" test --workspace; echo "exit=$?"` → `exit=0`；`"$CARGO" test --workspace -- --list 2>/dev/null | grep -c ": test"` ≥593

- [ ] **W-B4-30.2 用例名差集 `removed=0`** · `scripts/test-name-inventory.sh`
  - 追溯：AC-B4-39 · spec §4.5-1 · design §1.2.8
  - DoD：`--diff docs/progress/test-names-before.txt` 退 **0** 且输出含 `removed=0`；6 个新 target 不计入旧清单差集（由 `NEW_TARGETS` 排除）
  - 证据：`CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-30.3 既有常量与 B3 48 条 AC 零回归** · `crates/domain/**`、`.codeartsdoer/specs/b3_understanding/spec.md`
  - 追溯：AC-B4-40 · spec §4.5-2/4 · design §2.4.2-3
  - DoD：5 个常量各恰 1 处（0.35/60/2000/200/11）；`GapKind` 变体 6；`grep -c "AC-B3-" b3 spec` ≥ **48**；B3 的 48 条 AC 对应用例全绿（`cargo test --workspace` 覆盖）
  - 证据：`grep -c "AC-B3-" .codeartsdoer/specs/b3_understanding/spec.md` ≥48；`grep -c "pub const ALL: \[Self; 11\]" crates/domain/src/error.rs` ==1

- [ ] **W-B4-30.4 零新依赖 + `cargo deny` 四项** · `Cargo.toml`、`Cargo.lock`、`deny.toml`
  - 追溯：AC-B4-41 · spec §4.5-3 · design §2.4.2-2
  - DoD：`Cargo.toml`/`Cargo.lock` 依赖集合不变（新增依赖行 0）；`cargo deny check licenses bans sources advisories` 四项全过；domain/application 依赖闭包不含网络/存储/平台实现
  - 证据：`git diff --stat HEAD -- Cargo.toml Cargo.lock | wc -l` → `0`；`"$CARGO" deny check licenses bans sources advisories; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-30.5 密钥/明文/文档扫描三项 0 命中** · `scripts/scan-secrets.sh`、`scripts/scan-secrets-docs.sh`
  - 追溯：AC-B4-42 · spec §4.3-1 · design §2.5.6
  - DoD：`scan-secrets.sh` 退 0；`--rule=zero-transcript` 退 0；`scan-secrets-docs.sh --all-md` 退 0；门控源码、夹具、`b4-status.md` 三者命中 0
  - 证据：`bash scripts/scan-secrets.sh && bash scripts/check-b4-gates.sh --rule=zero-transcript && bash scripts/scan-secrets-docs.sh --all-md; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-30.6 9 个 `#[ignore]` 不变 + 不宣称 G4 + `--rule=source` 退 0** · `crates/**`、`scripts/check-g2-claims.sh`
  - 追溯：AC-B4-44、AC-B4-43 · spec §1.4-10、§7.1 · design §2.5.7
  - DoD：`--list --ignored` 计数 **9** 且名称与基线一致；本轮未新增 `#[ignore]`；`check-g2-claims.sh` 退 0（无"G4 通过"类宣称）；`--rule=source` 退 **0**（不是 2）
  - 证据：`"$CARGO" test --workspace -- --list --ignored 2>/dev/null | grep -c ": test"` == 9；`bash scripts/check-g2-claims.sh; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-30.7 平台口径：Windows 单平台，mac 不适用** · `scripts/check-mac-scope.sh`、`.codeartsdoer/specs/b4_gate/spec.md`
  - 追溯：AC-B4-45 · spec §1.10 · design §2.5.7
  - DoD：`check-mac-scope.sh --rule=claims` 退 0；`gate/**` 与 `scripts/check-b4-gates.sh` 中 mac 专属路径/依赖/分支 0；规格"不适用（无设备）"命中 ≥1
  - 证据：`bash scripts/check-mac-scope.sh --rule=claims; echo "exit=$?"` → `exit=0`；`grep -cF "不适用（无设备）" .codeartsdoer/specs/b4_gate/spec.md` ≥1

### W-B4-31 记录与文档口径（AC-47/50）

> 前置：W-B4-30。

- [ ] **W-B4-31.1 `--rule=record` 终验（反例改由自检证明）** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-47 · spec §6.6 · design R-04、§3.8.4、§9-Q5
  - DoD：正常态退 0（14 字段 / 23 变体 / `skipped ⊆ {G2,G3}`）；反例**不得**临时修改真实源码：由 `--self-test` 在 `mktemp -d` 内构造"白名单含 `G4`"并断言退 1；如需额外验证，复制 `gate.rs` 到临时目录后修改副本再扫描
  - 证据：`bash scripts/check-b4-gates.sh --rule=record; echo "exit=$?"` → `exit=0`；`bash scripts/check-b4-gates.sh --self-test; echo "exit=$?"` → `exit=1`；`git status --porcelain | wc -l` → `0`
  - 证据产物：两条退出码与"零污染"结论写入 `b4-status.md` §3

- [ ] **W-B4-31.2 `--rule=doc-plan` + `b4-status.md` 50 条 AC 三态登记** · `scripts/check-b4-gates.sh`、`docs/progress/b4-status.md`
  - 追溯：AC-B4-50 · spec §8 · design §2.7（D-B4-D01~D04）
  - **P0 状态**：6 段骨架已在 W-B4-8.1 创建（含 D-B4-D02/D04/D05~D11/R-01）；本节只补 50 行 AC 三态、D-B4-D01~D04 与 macOS 口径，不重建文件、不改 §1~§4 已有编号。
  - DoD：spec §8 表格行数 ≥1，每行"文件 + 章节 + 动作"可定位（存在或标"新增"）；`b4-status.md` 含 50 行三态（`passed`/`not-applicable`/`pending`），D-B4-01~D-B4-05 + D-B4-D01~D04 全部登记；macOS 全部标"不适用（无设备）"
  - 证据：`bash scripts/check-b4-gates.sh --rule=doc-plan; echo "exit=$?"` → `exit=0`；`grep -c "AC-B4-" docs/progress/b4-status.md` ≥50

### W-B4-32 回滚开关（B4 关闭时行为不变）

> 前置：W-B4-27。追溯 design §2.4.1、spec §1.6/§6.9（无独立 AC，服务于 AC-B4-39 的零回归）。

- [ ] **W-B4-32.1 `GateFeature::Disabled` 为默认** · `crates/application/src/gate/mod.rs`
  - 追溯：AC-B4-39 · spec §1.6 · design §2.1.3.9、§2.4.1
  - DoD：`GateFeature` 默认 `Disabled`；`Disabled` 时装配方不调用 `evaluate_gate`、不创建 `GateCounters`、不写 `GateRecord`、不调 `issue_query`；`gate/**` 无全局状态（`static mut`/`OnceLock`/`thread::spawn` 命中 0）
  - 证据：`! grep -qE "static mut|OnceLock|thread::spawn|once_cell" crates/application/src/gate` 判定通过

- [ ] **W-B4-32.2 零副作用用例** · `crates/application/tests/gate_outbound.rs`
  - 追溯：AC-B4-39 · design §2.4.1（`gate_feature_disabled_is_zero_effect`）
  - DoD：关闭态断言 `gate_*` 计数全 0、`outbound_attempts == 0`、`records.is_empty()`；与 B4 之前行为逐项一致；开启后同一输入才产生记录
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_outbound gate_feature_disabled_is_zero_effect` 退 0

### W-B4-33 50 条 AC 收口与全量门禁

> 前置：W-B4-1~32。**本任务不写代码，只做收口复验与登记。**

- [ ] **W-B4-33.1 6 个 target 全绿** · `crates/application/tests/gate_*.rs`
  - 追溯：AC-B4-01~50 · spec §7.9 · design §2.6-P3 出口
  - DoD：6 个 `gate_*` target 分别退出 0；`NEW_TARGETS` 长度 **19**（P3 时点；P4 后为 **20** 且共 7 个 target，见 W-B4-42.2/42.5）；无 `#[ignore]` 新增
  - 证据：`for t in gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay; do "$CARGO" test -p meeting-copilot-application --test "$t" || exit 1; done; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-33.2 `check-b4-gates.sh` 七条规则全 0** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-43、AC-B4-47、AC-B4-50、AC-B4-33、AC-B4-22、AC-B4-34、AC-B4-39 · spec §7.8
  - DoD：`source`/`zero-transcript`/`budget`/`outbound`/`record`/`targets`/`doc-plan` 七条各退 **0**；`--self-test` 退 **1**
  - 证据：`for r in source zero-transcript budget outbound record targets doc-plan; do bash scripts/check-b4-gates.sh --rule="$r" || exit 1; done; echo "exit=$?"` → `exit=0`；`bash scripts/check-b4-gates.sh --self-test; echo "exit=$?"` → `exit=1`

- [ ] **W-B4-33.3 既有门禁五项不回退** · `scripts/**`
  - 追溯：AC-B4-39、AC-B4-41、AC-B4-42 · spec §4.5 · design §2.5 统一前置
  - DoD：`fmt --check` = 0；`clippy -D warnings` = 0；`check-layering.sh` = 0；`scan-secrets.sh` = 0；`cargo deny` 四项 = 0
  - 证据：`"$CARGO" fmt --all -- --check && "$CARGO" clippy --workspace --all-targets -- -D warnings && bash scripts/check-layering.sh && bash scripts/scan-secrets.sh && "$CARGO" deny check licenses bans sources advisories; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-33.4 B3 门禁六条 + 差集 `removed=0`** · `scripts/check-b3-gates.sh`、`scripts/test-name-inventory.sh`
  - 追溯：AC-B4-39、AC-B4-40 · spec §4.5-1/4 · design §1.2.8
  - DoD：B3 六条规则各退 0（`--self-test` 预期 1）；`test-name-inventory.sh --diff` 退 0 且 `removed=0`；594+ 用例名零删除
  - 证据：`for r in source no-truncate zero-transcript fallback-untouched new-targets exports; do bash scripts/check-b3-gates.sh --rule="$r" || exit 1; done; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-33.5 50/50 三态登记与出口结论** · `docs/progress/b4-status.md`
  - 追溯：AC-B4-50 · spec §7.10 · design §2.9 交付自检
  - DoD：`b4-status.md` 中 AC-B4-01~50 每行有终态（`passed` 或 `not-applicable`）；`passed` 行附用例名或命令；`not-applicable` 行附 D-B4-0x 编号；明确"离线通过 ≠ G4 通过、替身通过 ≠ 真实端点已验证"，不宣称 G4
  - 证据：`grep -cE "^\\| AC-B4-(0[1-9]|[1-4][0-9]|50) \\|" docs/progress/b4-status.md` ≥50；`! grep -q "G4 通过" docs/progress/b4-status.md` 判定通过

---

---

## P4 · 关键词卡片契约与 200 例回放（增量 AC-B4-48 / 51~57）

> 阶段：第三阶段（编码任务规划）的**增量批次**。需求来源：`requirements.md` §10、`spec.md` §5.6/§6.11~6.13/§7.11/§7.10、`design.md` §四（4.0~4.10）。进度基线：`docs/progress/b4-status.md` §9（AC-48/51~57 均 `pending`）、§10.1 U-01/U-02。
> **范围**：只做 B4 的**契约与前置**。不做 38 合成、39 对齐、51 UI、13.3 本体、52 存储；不改 `GateRecord`（14）/`GateAuditEvent`（7）/`GateInputFixture`（23）/`ReplayReport`（5）/`GateCounters`（32）/`GateRejectReason`（23）。
> **与 U-01~U-08 的关系**：U-01 由 **W-B4-41** 闭合；U-02 由 **W-B4-34~W-B4-40** 闭合；U-03/U-04/U-05 只保留 design §4.5.2 的签名预留，**P4 不建文件**；U-06 保持 `tests/replay/` 不存在；U-07 不承接；U-08 保持 9 个 `#[ignore]`、不宣称 G4。
> **P4 出口判据**：① `gate_keyword` target 13 个用例全绿；② `gate_replay` 的 AC-48 两用例绿（`total=200`/`mismatched=0`/`seed=7`/两次逐字节相等）；③ 七条规则 7×0、`--rule=targets` 退 0 且 `NEW_TARGETS` = 20；④ `api.rs` 4 个 `pub fn` 且零 diff、五个封闭字段数不变；⑤ 全量 `cargo test --workspace` `failed=0`/`ignored=9`、`removed=0`。

### P4 公共断言（每个 shell 会话执行一次，后续任务直接引用）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
KW_FILE="crates/domain/src/model/gate_keyword.rs"

# 结构体业务字段数（仅 ^    pub <name>:，不含 flatten/私有字段）
kw_fields() { awk -v s="pub struct $1 {" 'index($0,s)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    pub [a-z_]+:'; }
# 枚举变体数（含带载荷变体）
kw_variants() { awk -v e="pub enum $1 {" 'index($0,e)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    [A-Z][A-Za-z0-9]*(\(|,)'; }
# gate_keyword.rs 顶层类型项数（目标 11 = 4 struct + 7 enum）
kw_items() { grep -cE '^pub (struct|enum) ' "$KW_FILE"; }
```

> **零转录纪律**：`gate/**`、`tests/gate_*.rs`、`docs/progress/b4-status.md` 中不得出现 URL、鉴权头、模型名前缀、`request_id:` 形式；测试内的构造词只允许 `format!("术语{index}")`、`format!("{}%", n)` 这类短词，禁止 `assert_eq!` 直接打印文本。
> **禁词纪律**：`crates/application/src/gate/**` 中 `composer|validator|presenter|card_store|outbox|on_demand|feedback_weights` 命中必须为 0（**注释里也不许写**）；`TriggerKind::OnDemand` 是 CamelCase，不命中 `on_demand`，可安全使用。

### W-B4-34 domain 关键词契约落位（AC-B4-51/53/56）

> 前置：无（P4 第一项）。落点 `crates/domain/src/model/gate_keyword.rs`（新文件），`gate.rs`/`gate_replay.rs`/`enums.rs` 零 diff。

- [ ] **W-B4-34.1 新增 `KeywordRef` 与 `NormalizedTerm` 值对象** · `crates/domain/src/vo/ids.rs`、`crates/domain/src/vo/text.rs`、`crates/domain/src/vo/mod.rs`
  - 追溯：AC-B4-51、AC-B4-53 · requirements §10.4.1-1/3 · design §4.3.1（D-B4-P21）
  - 改动内容：`ids.rs` 在 `id_type!(ReferenceRef, …)` 后追加 `id_type!(KeywordRef, "关键词引用");`；`text.rs` 在 `bounded_text!(UtteranceText, …)` 后追加 `bounded_text!(NormalizedTerm, "归一化术语", 120);`；`vo/mod.rs` 的 `pub use ids::{…}` 补 `KeywordRef`、`pub use text::{…}` 补 `NormalizedTerm`
  - DoD：`cargo build -p meeting-copilot-domain` 退 0；`grep -c 'id_type!(KeywordRef' ids.rs` = 1；`grep -c 'bounded_text!(NormalizedTerm' text.rs` = 1；`NormalizedTerm::LIMIT == 120`；`KeywordRef::new("a b")` 返回 `Err`；新增单测 `keyword_ref_rejects_whitespace`、`normalized_term_limit_is_120` 绿
  - 证据：`"$CARGO" test -p meeting-copilot-domain keyword_ref_rejects_whitespace normalized_term_limit_is_120` 退 0；`grep -n 'KeywordRef' crates/domain/src/vo/mod.rs | wc -l` ≥1
  - 证据产物：退出码写入 `b4-status.md` §11.1

- [ ] **W-B4-34.2 7 个 B4 专属封闭枚举与单向投影** · `crates/domain/src/model/gate_keyword.rs`
  - 追溯：AC-B4-51、AC-B4-53、AC-B4-55 · spec §6.11-4、§6.12-8/9、§6.13-2/3 · design §4.3.2（D-B4-P20/K-02）
  - 改动内容：`KeywordKind`(5)/`KeywordCardStatus`(3)/`ReferenceTrustTier`(4)/`ReferenceAccessStatus`(2)/`CitationAlignment`(3)/`ClickState`(2)/`ClickDisabledReason`(6)，全部 `#[serde(rename_all="snake_case")]`；投影函数 `keyword_kind_of(GapRule) -> Option<KeywordKind>`、`card_status_from(enums::CardStatus)`、`trust_tier_from(enums::TrustTier)`、`access_status_from(enums::AccessStatus)`；`ClickDisabledReason::ALL: [Self; 6]`
  - 禁止：不改 `crates/domain/src/model/enums.rs`（零 diff）；不复用 `CardStatus::Shown`（serde 值为 `shown`，与 `displayed` 冲突）
  - DoD：`kw_variants KeywordKind` = 5、`KeywordCardStatus` = 3、`ReferenceTrustTier` = 4、`ReferenceAccessStatus` = 2、`CitationAlignment` = 3、`ClickDisabledReason` = 6；`keyword_kind_of(GapRule::Institution) == Some(InstitutionRole)`、`keyword_kind_of(GapRule::UnansweredQuestion) == None`；`AccessStatus` 5 态中 4 态投影为 `Unavailable`
  - 证据：`kw_variants KeywordKind "$KW_FILE"` = 5（依次核对 7 个枚举）；`"$CARGO" test -p meeting-copilot-domain keyword_enums_are_closed_and_projected` 退 0；`git diff --quiet HEAD -- crates/domain/src/model/enums.rs; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-34.3 `AllowTraceRef`（只由 `Allow` 构造）** · `crates/domain/src/model/gate_keyword.rs`
  - 追溯：AC-B4-52、AC-B4-53 · spec §6.12-7 · design §4.3.3（R-K7）· 本文件 D-B4-P27
  - 改动内容：`pub struct AllowTraceRef { trace_id: TraceId, stopped_at: Option<GateStage> }`（字段**私有**）；`pub fn new(decision: &GateDecision) -> Option<Self>`（仅 `GateDecision::Allow` 返回 `Some`，`Deny` 返回 `None`）；`pub fn trace_id(&self) -> &TraceId`；`Serialize/Deserialize` 只暴露 `trace_id`/`stopped_at`
  - DoD：`AllowTraceRef` 计入 `kw_items` → 文件顶层类型 **11** 项（4 struct + 7 enum）；`Deny` 输入返回 `None`；`Allow` 输入返回 `Some` 且 `trace_id` 等于输入 `trace_id`；结构体内 `pub [a-z_]+:` 计数 = 0（无公有字段）
  - 证据：`kw_items` = 11；`"$CARGO" test -p meeting-copilot-domain allow_trace_ref_is_only_from_allow` 退 0

- [ ] **W-B4-34.4 `KeywordCandidate`（10 字段）+ 构造期校验** · `crates/domain/src/model/gate_keyword.rs`
  - 追溯：AC-B4-51、AC-B4-53 · spec §6.11-1~10 · requirements §10.4.1 · design §4.3.1
  - 改动内容：字段顺序 `keyword_ref`/`keyword_text`(TitleText)/`normalized_term`(NormalizedTerm)/`keyword_kind`/`trigger_span`(TextRange)/`confidence`(Confidence)/`dedupe_key`(DedupeKey)/`state`(GapState)/`session_id`(SessionId)/`source_insight_ref`(Option<InsightRef>)；`pub fn validate(&self) -> bool`（`confidence >= 0.50`、`start_ms < end_ms`、`state != GapState::Pushed`、`keyword_text` 不含空白整句）
  - DoD：`kw_fields KeywordCandidate` = **10**；`validate()` 对 `confidence=0.49` 返回 false、`0.50` 返回 true；`state=Pushed` 返回 false；`start==end` 返回 false
  - 证据：`kw_fields KeywordCandidate "$KW_FILE"` = 10；`"$CARGO" test -p meeting-copilot-domain keyword_candidate_has_ten_fields keyword_candidate_rejects_pushed_state` 退 0

- [ ] **W-B4-34.5 `ReferenceView`（5 字段，引用只存 ID）** · `crates/domain/src/model/gate_keyword.rs`
  - 追溯：AC-B4-53、AC-B4-56 · spec §6.13-1~6 · requirements §10.4.3
  - 改动内容：`reference_id`(ReferenceRef)/`trust_tier`(ReferenceTrustTier)/`access_status`(ReferenceAccessStatus)/`published_at_ms`(Option<u64>)/`alignment`(CitationAlignment)
  - 禁止：不得出现 `url`/`title`/`excerpt`/`content`/`query`/`snippet` 字段或同义字段
  - DoD：`kw_fields ReferenceView` = **5**；`grep -cEi '^    pub (url|title|excerpt|content|query|snippet):'` = 0
  - 证据：`kw_fields ReferenceView "$KW_FILE"` = 5；`"$CARGO" test -p meeting-copilot-domain keyword_reference_view_has_five_fields` 退 0

- [ ] **W-B4-34.6 `KeywordCardDraft`（13 字段）+ 3 个谓词** · `crates/domain/src/model/gate_keyword.rs`
  - 追溯：AC-B4-52、AC-B4-53、AC-B4-55、AC-B4-56 · spec §6.12-1~11 · design §4.3.1/§4.2.4（I-01/I-03/I-04）
  - 改动内容：字段顺序 = `card_id`(CardRef)/`keyword_text`(TitleText)/`normalized_term`(NormalizedTerm)/`keyword_kind`/`trigger`(TriggerKind)/`triggered_at_ms`(u64)/`confidence`(Confidence)/`summary`(Option<ConclusionText>)/`references`(Vec<ReferenceView>)/`citation_alignment`/`gate_decision_ref`(AllowTraceRef)/`card_status`(KeywordCardStatus)/`click_state`(ClickState)；常量 `pub const NO_RELIABLE_SOURCE: &str = "未找到可靠来源";`；谓词 `summary_is_consistent()`（`summary.is_some() ⇒ alignment == Passed ∨ summary == NO_RELIABLE_SOURCE`）、`click_state_is_consistent()`（`Enabled ⇒ allow_ref 存在 ∧ alignment == Passed ∧ card_status != Withdrawn`）、`is_withdrawn()`
  - DoD：`kw_fields KeywordCardDraft` = **13**；`summary_is_consistent` 对 `Failed + 结论文本` 返回 false、对 `Failed + NO_RELIABLE_SOURCE` 返回 true；`click_state_is_consistent` 对 `Enabled + Failed` 返回 false；谓词全部返回 `bool`（不 panic、不 `unwrap`）
  - 证据：`kw_fields KeywordCardDraft "$KW_FILE"` = 13；`"$CARGO" test -p meeting-copilot-domain keyword_card_has_thirteen_fields keyword_card_predicates_fail_closed` 退 0

- [ ] **W-B4-34.7 domain 契约单测 6 条（字段封闭/投影/谓词）** · `crates/domain/src/model/gate_keyword.rs`
  - 追溯：AC-B4-53 · design §4.10-1/2 · spec §6.12-1
  - DoD：6 个用例名齐备且绿：`keyword_card_has_thirteen_fields`、`keyword_reference_view_has_five_fields`、`keyword_candidate_has_ten_fields`、`keyword_enums_are_closed_and_projected`、`allow_trace_ref_is_only_from_allow`、`keyword_card_predicates_fail_closed`；单测块内 `unwrap()` 只允许出现在 `#[cfg(test)]`（`--rule=source` 会 `strip_tests`）
  - 证据：`"$CARGO" test -p meeting-copilot-domain keyword_` 退 0（6 passed）

- [ ] **W-B4-34.8 模块导出与非破坏面** · `crates/domain/src/model/mod.rs`、`crates/domain/src/model/gate.rs`、`crates/domain/src/model/gate_replay.rs`
  - 追溯：AC-B4-56、AC-B4-57 · design §4.8.2（D-B4-D08 收窄）· D-B4-P19/P24
  - 改动内容：`model/mod.rs` 追加 `pub mod gate_keyword;` 与 `pub use gate_keyword::*;`（各 1 行，共 +2）
  - DoD：`gate.rs` 零 diff 且 `grep -cE '^pub (struct|enum) ' gate.rs` 仍 ≥31；`gate_replay.rs` 零 diff；`GateRecord` 字段集合仍 14（`--rule=record` 退 0）；`gate_keyword.rs` 内无 `panic!(`/`todo!(`/`unwrap(`（非测试区）
  - 证据：`git diff --quiet HEAD -- crates/domain/src/model/gate.rs crates/domain/src/model/gate_replay.rs; echo "exit=$?"` → `exit=0`；`bash scripts/check-b4-gates.sh --rule=record; echo "exit=$?"` → `exit=0`

### W-B4-35 G1 同源规则标签与跳过级投影（AC-B4-51/52）

> 前置：W-B4-34。**这是 AC-B4-51 可达性的唯一解**：若继续用 `diagnostics::rule_of(GapKind)`，`Institution` 被并入 `ProperNoun`，`InstitutionRole` 永不可达。

- [ ] **W-B4-35.1 `scan_candidates_with_rule`（保留 `GapRule` + 词）** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-51 · design §4.1.2（K-01）、§4.2.3 · 本文件 D-B4-P30
  - 改动内容：新增 `pub(crate) struct ScannedCandidate { pub rule: GapRule, pub term: String, pub candidate: GapCandidate }`；把现有 `fn scan_candidates` 主体改名为 `pub(crate) fn scan_candidates_with_rule(scan, counters) -> Vec<ScannedCandidate>`，在每个 `out.push(GapCandidate { … })` 处同时携带 `rule` 与 `term.clone()`（未答提问的 `term` 用 `"unanswered_question"`）；排序口径保持 `(confidence desc, start_ms asc, dedupe_key asc)` 不变
  - 禁止：不改 `GapScanInput` 字段（11 项）；不改 `derive_dedupe_key`（dedupe 键必须与既有逐位相同，否则 AC-B4-16 回归）；不改 `GapKind`（6 变体）
  - DoD：`grep -c 'pub(crate) fn scan_candidates_with_rule'` = 1；`grep -c 'pub(crate) struct ScannedCandidate'` = 1；`dedupe_keys_are_32_hex_and_deterministic` 仍绿；`gate_prefilter` **20 passed / 0 failed**
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter` 退 0 且 `20 passed`；`git diff --quiet HEAD -- crates/application/src/gate/api.rs; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-35.2 `scan_candidates` 改为复用（`detect_gaps` 行为零变）** · `crates/application/src/gate/prefilter.rs`
  - 追溯：AC-B4-01~08（零回归）、AC-B4-51 · D-B4-P23
  - 改动内容：`fn scan_candidates(scan, counters) -> Vec<GapCandidate> { scan_candidates_with_rule(scan, counters).into_iter().map(|item| item.candidate).collect() }`（≈3 行）
  - DoD：`grep -c 'fn scan_candidates('` = 1（仅一处定义）；`scan_candidates_with_rule(` 出现 ≥2 次（定义 + 复用）；`detect_gaps` 签名零变；`prefilter_scores_six_gap_rules`、`prefilter_is_pure_and_deterministic`、`prefilter_denies_eleven_hard_cases` 全绿
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_prefilter` 退 0（20 passed）；`grep -c '^pub fn ' crates/application/src/gate/api.rs` = 4

- [ ] **W-B4-35.3 G2 跳过投影 `StageResult::Skipped`** · `crates/application/src/gate/diagnostics.rs`
  - 追溯：AC-B4-52 · spec §5.6.5 · design §4.2.3（R-K6）· 本文件 D-B4-P28
  - 改动内容：G2 段前判断 `if !policy.relevance_enabled { per_stage.0[1] = trace(StageResult::Skipped, None, 0); } else { …既有 G2 循环… }`；跳过时不产 `GateL2LowRelevance` 计数、不因 `LowRelevance` 拒绝；不新增 `GateCounter`（32 项不变）
  - DoD：`relevance_enabled=false` 时 `per_stage.0[1].result == Skipped` 且决策可到 G3；`relevance_enabled=true`（默认）时既有 `gate_relevance` **11 passed / 0 failed**；`grep -c 'pub enum GateCounter' gate.rs` 不变、`snapshot()` 仍 32
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_relevance` 退 0；跳过投影的用例断言由 **W-B4-38.1**（`g2_skip_marks_stage_skipped`）与 **W-B4-38.5**（`keyword_card_requires_gate_allow` 的第二段）承接——本条不依赖尚未创建的 target

- [ ] **W-B4-35.4 G3 跳过投影 `StageResult::Skipped`** · `crates/application/src/gate/diagnostics.rs`
  - 追溯：AC-B4-52 · design §4.2.3（R-K6）· D-B4-P28
  - 改动内容：把 `per_stage.0[2] = trace(StageResult::Allow, None, g3_elapsed);` 改为按 `g3.skipped` 分支：`if g3.skipped { trace(StageResult::Skipped, None, g3_elapsed) } else { trace(StageResult::Allow, None, g3_elapsed) }`；`GateStageSkippedTotal` 计数逻辑不变
  - DoD：`dedupe_enabled=false` 时 `per_stage.0[2].result == Skipped`；`dedupe_enabled=true` 时既有 `gate_dedupe` **15 passed / 0 failed**；`dedupe_switch_keeps_suppression_and_cache` 绿
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_dedupe` 退 0；跳过投影的用例断言由 **W-B4-38.1**（`g3_skip_marks_stage_skipped`）与 **W-B4-38.5** 承接

### W-B4-36 关键词候选派生（AC-B4-51）

> 前置：W-B4-35。落点 `crates/application/src/gate/keyword.rs`（新文件，**不在 `api.rs`**）。

- [ ] **W-B4-36.1 模块骨架与 `KeywordCandidateSet`** · `crates/application/src/gate/keyword.rs`
  - 追溯：AC-B4-51 · design §4.3.3（D-B4-P23/K-04）
  - 改动内容：`//! 关键词候选派生与出卡准入：只读、无 IO、无随机。`；`pub struct KeywordCandidateSet { pub new_candidates: Vec<KeywordCandidate>, pub reobservations: Vec<KeywordReobservation> }`；`pub struct KeywordReobservation { pub keyword_ref: KeywordRef, pub dedupe_key: DedupeKey, pub session_id: SessionId }`（**不含任何文本字段**，零转录）
  - 禁止：本文件不得出现 `evaluate_gate`、`issue_query`、`AuditLog::append`、`reqwest`、`Instant::now`、`SystemTime::now`、`Mutex`、`RwLock`、`Atomic*`、`panic!(`、`todo!(`，以及禁词表 7 项
  - DoD：`grep -c 'pub struct KeywordCandidateSet'` = 1；`grep -cE 'keyword_text|summary' keyword.rs` 只允许出现在 `KeywordCardDraft` 相关签名的借用处（若为 0 亦可）；`cargo build -p meeting-copilot-application` 退 0
  - 证据：`"$CARGO" build -p meeting-copilot-application; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-36.2 五类映射 + 未答提问过滤** · `crates/application/src/gate/keyword.rs`
  - 追溯：AC-B4-51 · spec §5.6.1-1/§6.11-4 · design §4.2.3（映射表、R-K5）
  - 改动内容：`pub fn derive_keyword_candidates(scan: &GapScanInput<'_>) -> KeywordCandidateSet`；内部 `let mut counters = GateCounters::default();` 调用 `prefilter::scan_candidates_with_rule(scan, &mut counters)`；`keyword_kind_of(item.rule)` 为 `None`（即 `UnansweredQuestion`）时**直接丢弃**；`keyword_text = TitleText::new(item.term)`（超 40 字则丢弃，不截断）；`normalized_term = NormalizedTerm::new(normalize_text(&item.term))`（非法/空则丢弃）
  - DoD：五类正例（专名/缩写/机构/政策/指标）各产 1，`new_candidates.len() == 5` 且 `keyword_kind` 五项各 1；仅未答提问时 `new_candidates.is_empty()`；`grep -c 'UnansweredQuestion' keyword.rs` 出现且仅用于过滤分支
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_candidates_cover_five_kinds` 退 0；同 `unanswered_question_is_not_a_keyword_card` 退 0

- [ ] **W-B4-36.3 归一化首见唯一 + `reobservations`** · `crates/application/src/gate/keyword.rs`
  - 追溯：AC-B4-51 · spec §5.6.1-2/§6.11-3 · design §4.2.3（R-K3/R-K4）、§4.1.2
  - 改动内容：`BTreeSet<(SessionId, NormalizedTerm)>` 作首见键（键 = `(scan.session.clone(), normalized_term.clone())`）；二次命中 → 推入 `reobservations`（复用首次的 `keyword_ref` 与 `dedupe_key`，**不重算** `derive_dedupe_key`）；`keyword_ref` 由 `derive_keyword_ref(session, &dedupe_key)` 派生（`fnv1a64` 双段，session 参与，跨会话必不同）
  - DoD：同会话同术语第二次出现 → `new_candidates` 增量 0、`reobservations.len() == 1`、`reobservations[0].dedupe_key` 与首次逐位相等；两个会话同术语 → 各 1 个候选且 `keyword_ref` 不相等
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_candidate_keeps_first_seen_only` 退 0

- [ ] **W-B4-36.4 确定性排序与纯性** · `crates/application/src/gate/keyword.rs`
  - 追溯：AC-B4-51、AC-B4-07（同款纯性口径）· design §4.2.3（R-K1/R-K8）
  - 改动内容：`new_candidates.sort_by(...)` 按 `(confidence desc, trigger_span.start_ms asc, dedupe_key asc)`；函数体内不读时间、不写全局、不分配有序容器以外的东西（`BTreeSet`/`Vec` 允许）
  - DoD：同输入 100 次调用逐字段相等（含 `Vec` 顺序）；`grep -c 'evaluate_gate' keyword.rs` = 0；新增单测 `keyword_derivation_is_pure_and_deterministic` 绿
  - 证据：`"$CARGO" test -p meeting-copilot-application --lib keyword_derivation_is_pure_and_deterministic` 退 0；`bash scripts/check-b4-gates.sh --rule=source; echo "exit=$?"` → `exit=0`（W-B4-42.1 把 `gate_keyword.rs` 纳入扫描面后为本条硬判据）

### W-B4-37 出卡准入、点击状态与卡片 ID（AC-B4-52/54/55）

> 前置：W-B4-34、W-B4-36。

- [ ] **W-B4-37.1 `keyword_card_admission`（Allow 才 1 卡，`click_state` 独立）** · `crates/application/src/gate/keyword.rs`
  - 追溯：AC-B4-52、AC-B4-54、AC-B4-55 · spec §5.6.4/§5.6.6 · design §4.2.4/§4.3.3 · 本文件 D-B4-P31/P32
  - 改动内容：`pub struct KeywordCardEnv<'a> { pub authorization_status: AuthorizationStatus, pub authorization_revoked: bool, pub screen_share: bool, pub emergency_masked: bool, pub current_session: &'a SessionId, pub card_status: KeywordCardStatus }`；`pub struct KeywordCardAdmission { pub card_drafts: bool, pub click_state: ClickState, pub allow_ref: Option<AllowTraceRef>, pub body_visible: bool }`；`pub fn keyword_card_admission(candidate, decision, trace_id, alignment, env) -> KeywordCardAdmission`：`Deny` → `card_drafts=false`、`allow_ref=None`、`Disabled{NotAllowed}`、`body_visible=false`；`Allow` → `card_drafts=true`、`allow_ref=AllowTraceRef::new(decision)`，`click_state=resolve_click_state(facts)`，`body_visible = !screen_share && !emergency_masked`
  - 禁止：`Deny` 路径不得返回 `Some(allow_ref)`；不得因 `alignment != Passed` 把 `card_drafts` 置 false（否则违反 spec §5.6.3-2"卡片仍可展示但不可点击"）
  - DoD：五级 Allow → `card_drafts=true` + `Enabled`；任一级 Deny → `card_drafts=false` + `Disabled{NotAllowed}` + `allow_ref.is_none()`；Allow + `Failed` → `card_drafts=true` + `Disabled{ReferenceUnverified}`；投屏 → `body_visible=false`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_card_requires_gate_allow deny_never_emits_clickable_card` 退 0

- [ ] **W-B4-37.2 `resolve_click_state` 六因固定优先级** · `crates/application/src/gate/keyword.rs`
  - 追溯：AC-B4-55 · spec §6.12-9/10 · design §4.2.4（K-05/D-B4-P22）
  - 改动内容：`pub struct ClickFacts { pub allow_ref_present: bool, pub alignment: CitationAlignment, pub card_status: KeywordCardStatus, pub session_matches: bool, pub authorization_granted: bool, pub authorization_revoked: bool, pub screen_share: bool, pub emergency_masked: bool, pub reference_stale: bool }`；`pub fn resolve_click_state(facts: ClickFacts) -> ClickState`，判定顺序 `reference_stale || !session_matches → StaleReference`；`authorization_revoked → AuthorizationRevoked`；`!allow_ref_present || card_status == Withdrawn → NotAllowed`；`emergency_masked → EmergencyMasked`；`screen_share → ScreenShareProtection`；`alignment != Passed → ReferenceUnverified`；否则 `Enabled`
  - DoD：6 因逐个构造各得唯一原因；两因叠加时返回**优先级更高**者（至少 3 组叠加断言）；`Enabled` 仅当 7 项合取全真
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword click_permission_is_fail_closed` 退 0

- [ ] **W-B4-37.3 `derive_card_id` 确定性派生** · `crates/application/src/gate/keyword.rs`
  - 追溯：AC-B4-52、AC-B4-53 · spec §6.12-2 · design §4.3.3（I-02/I-09）
  - 改动内容：`pub fn derive_card_id(session: &SessionId, dedupe_key: &DedupeKey) -> CardRef`：`fnv1a64(session.as_bytes() ++ dedupe_key.as_bytes())` 得 `hi`，`fnv1a64(dedupe_key.as_bytes() ++ session.as_bytes())` 得 `lo`，输出 `format!("card-{hi:016x}{lo:016x}")`（36 字符）
  - DoD：同一 `(session, dedupe_key)` 恒同值；跨会话不同；长度 = 36 且无空白；`CardRef::new` 恒 `Ok`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_card_has_closed_fields` 退 0（其中含 `card_id` 派生断言）；`"$CARGO" test -p meeting-copilot-application --lib keyword_card_id_is_deterministic` 退 0

- [ ] **W-B4-37.4 门面导出与回放面零扩散** · `crates/application/src/gate/mod.rs`
  - 追溯：AC-B4-46、AC-B4-57 · design §4.1.4、§4.5.1 · D-B4-P25
  - 改动内容：`mod.rs` 追加 `pub mod keyword;`（1 行）；**不**在 `pub use` 中列 `keyword::*`（避免回放面扩散断言误伤）
  - DoD：`grep -c '^pub fn ' api.rs` = 4；`api.rs` 零 diff；`grep -c 'pub mod keyword;'` = 1；`gate/**` 中禁词 7 项命中 0
  - 证据：`git diff --quiet HEAD -- crates/application/src/gate/api.rs; echo "exit=$?"` → `exit=0`；`grep -rnE "composer|validator|presenter|card_store|outbox|on_demand|feedback_weights" crates/application/src/gate | wc -l` → `0`

### W-B4-38 `gate_keyword` target 与 AC-51/52 用例（6 条）

> 前置：W-B4-36、W-B4-37。新增文件 `crates/application/tests/gate_keyword.rs`（第 7 个 target）。**用例名与 spec §7.11 逐字一致**。

- [ ] **W-B4-38.1 target 骨架 + 两个跳过投影用例** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-52 · spec §7.11 · design §4.7.1 · D-B4-P26/X-02
  - 改动内容：`mod common;` + `use common::Harness;`；顶部 `//!` 说明 13 个 AC 用例的归属；本地 helper `fn five_kind_fragments() -> Harness`、`fn sample_card(...) -> KeywordCardDraft`、`fn counting_ports() -> (CountingSanitizer, CountingSink)`（测试内定义，只计调用次数）；`#[test] fn g2_skip_marks_stage_skipped()`、`#[test] fn g3_skip_marks_stage_skipped()`（支撑 W-B4-35.3/35.4）
  - DoD：`cargo test --test gate_keyword` 可编译并跑通（≥2 passed）；文件中不含 URL/`request_id:`/模型名/鉴权头（`--rule=zero-transcript` 覆盖）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword` 退 0；`bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-38.2 `keyword_candidates_cover_five_kinds`（AC-51）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-51 · spec §5.6.1-1 · design §4.7.1
  - DoD：五条片段各命中一类 → `new_candidates.len() == 5`；`keyword_kind` 集合 = `{ProperNoun, Acronym, InstitutionRole, PolicyStandard, Metric}` 各 1；无 `UnansweredQuestion`；`card_drafts` 在该用例内未调用（候选不等卡片）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_candidates_cover_five_kinds` 退 0

- [ ] **W-B4-38.3 `keyword_candidate_keeps_first_seen_only`（AC-51）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-51 · spec §5.6.1-2/§6.11-3 · design §4.1.2（K-04）
  - DoD：同会话同术语二次 → `new_candidates` 增量 0、`reobservations.len() == 1`、`dedupe_key` 与首次相等；以该键重跑 `evaluate_gate`（`duplicate_last_seen_ms = now_ms - 599_999`）→ `Deny{Duplicate}` 且 `GateL3Duplicate == 1`；两个 `SessionId` 各 1 候选且 `keyword_ref` 不等
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_candidate_keeps_first_seen_only` 退 0

- [ ] **W-B4-38.4 `unanswered_question_is_not_a_keyword_card`（AC-51）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-51 · spec §5.6.1-1 · design §4.2.3（R-K5）
  - DoD：仅一条未答问句（`now_ms - end_ms >= 10_000`）→ `detect_gaps` 含 `UnansweredQuestion` 候选，但 `new_candidates.is_empty()`；`keyword_card_admission` 未被调用（`card_drafts == false`）；`keyword_kind_of(UnansweredQuestion).is_none()`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword unanswered_question_is_not_a_keyword_card` 退 0

- [ ] **W-B4-38.5 `keyword_card_requires_gate_allow`（AC-52）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-52 · spec §5.6.5 · design §4.2.3（R-K6）· D-B4-P28
  - DoD：五级 Allow → `card_drafts == true` 且 `click_state == Enabled`；`policy.dedupe_enabled = false`（G3 跳过）→ `per_stage[2] == Skipped` 且仍 1 卡；`policy.relevance_enabled = false`（G2 跳过）→ `per_stage[1] == Skipped` 且仍 1 卡；跳过级**不豁免** G1/G4/G5（任一 Deny 时 `card_drafts == false`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_card_requires_gate_allow` 退 0

- [ ] **W-B4-38.6 `deny_never_emits_clickable_card`（AC-52）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-52 · spec §5.6.5/§5.6.3-1 · design §4.2.4（R-K2/R-K7）
  - DoD：G1/G2/G3/G4/G5 逐级 Deny 共 5 例，每例 `card_drafts == false`、`click_state == Disabled{NotAllowed}`、`allow_ref.is_none()`；`Deny` 后不再调用后续级（`per_stage[i+1..] == NotReached`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword deny_never_emits_clickable_card` 退 0

### W-B4-39 AC-53/54 用例（4 条）

> 前置：W-B4-38。

- [ ] **W-B4-39.1 `keyword_card_has_closed_fields`（AC-53）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-53 · spec §6.12-1/9 · design §4.3.1（I-01/I-05）
  - DoD：`serde_json::to_value(card)` 的**顶层键集合**（排序后）等于 13 项白名单逐项相等；`keyword_text.chars().count() <= 40`、`summary.chars().count() <= 80`；`Disabled` 原因 ∈ `ClickDisabledReason::ALL`（6 项）；`card_id` 长度 36
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_card_has_closed_fields` 退 0

- [ ] **W-B4-39.2 `keyword_card_reference_is_id_only`（AC-53）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-53 · spec §6.13-1~6 · requirements §10.4.3
  - DoD：`ReferenceView` JSON 键集合 = `{reference_id, trust_tier, access_status, published_at_ms, alignment}` 5 项相等；`grep -cEi 'url|title|excerpt|content|query'` 在该 JSON 键集合命中 0；`trust_tier` 4 档、`access_status` 2 态各可达
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_card_reference_is_id_only` 退 0

- [ ] **W-B4-39.3 `clicking_keyword_card_is_local_only`（AC-54）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-54 · spec §5.6.7 · design §4.7.1
  - 改动内容：测试内 `CountingSanitizer`/`CountingSink`（`AtomicU64` 计数，测试局部，不进 `gate/`）；"点击"= 读取 `card.keyword_text`/`card.references`/`card.citation_alignment` 并组装本地详情视图（不调用 `evaluate_gate`、不调 `issue_query`、不调 `append`）
  - DoD：`outbound_attempts == 0`、`llm_calls == 0`、`sanitizer.calls == 0`、`sink.calls == 0`；`grep -cE 'issue_query|AuditLog::append' crates/application/src/gate/keyword.rs` = 0；未新建检索（`evaluate_gate` 调用计数 0）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword clicking_keyword_card_is_local_only` 退 0

- [ ] **W-B4-39.4 `continue_search_reenters_gate`（AC-54）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-54 · spec §5.6.7 · design §4.5.2（"继续检索"复用 G5 审计）
  - DoD：旧卡 `allow_ref.trace_id()` = t1；新 `GateInput`（新 `TraceId` t2）判 `Deny` → 新 `admission.card_drafts == false` + `Disabled{NotAllowed}` + `allow_ref.is_none()` 且外发计数 0；再构造 `Allow` → 新 `allow_ref.trace_id() == t2 != t1`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword continue_search_reenters_gate` 退 0

### W-B4-40 AC-55/56/57 用例与静态判据（3 条用例 + 5 条静态）

> 前置：W-B4-39。

- [ ] **W-B4-40.1 `click_permission_is_fail_closed`（AC-55）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-55 · spec §5.6.8 · design §4.2.4（K-05）
  - DoD：六因逐项命中且唯一：`reference_stale→StaleReference`、`authorization_revoked→AuthorizationRevoked`、`allow_ref_present=false→NotAllowed`、`emergency_masked→EmergencyMasked`、`screen_share→ScreenShareProtection`、`alignment=Failed→ReferenceUnverified`；3 组叠加（`stale+revoked→Stale`、`revoked+masked→Revoked`、`masked+screen→Masked`）各返回优先级更高者；`Enabled` 仅在 7 项合取全真时出现
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword click_permission_is_fail_closed` 退 0

- [ ] **W-B4-40.2 `keyword_card_hidden_under_screen_share`（AC-55）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-55 · spec §5.6.8 · D-B4-P32
  - DoD：`screen_share=true` → `body_visible == false` + `Disabled{ScreenShareProtection}`；`emergency_masked=true` → `body_visible == false` + `Disabled{EmergencyMasked}`；两者同时为真 → `Disabled{EmergencyMasked}`（优先级）；`body_visible=false` 时不得读取 `keyword_text` 组装正文（测试内以 `visible_body_len == 0` 断言）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_card_hidden_under_screen_share` 退 0

- [ ] **W-B4-40.3 `keyword_card_zero_transcript_boundary`（AC-56）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-56 · spec §5.6.9/§10.3.4 · design §4.4.1
  - DoD：`GateRecord` 14 键、`GateAuditEvent` 7 键、`ReplayReport` 5 键、`GateInputFixture` 23 键的**键集合**均不含 `keyword_text`/`summary`/`references`/`query`/`normalized_term`；`--rule=zero-transcript` 退 0；`--rule=record` 退 0
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_card_zero_transcript_boundary` 退 0；`bash scripts/check-b4-gates.sh --rule=zero-transcript && bash scripts/check-b4-gates.sh --rule=record; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-40.4 `keyword_card_ownership_is_outside_gate`（AC-57）** · `crates/application/tests/gate_keyword.rs`
  - 追溯：AC-B4-57 · spec §5.6.10 · design §4.5.1（P-01~P-03）
  - DoD：`gate/**` 禁词 4 项（`composer|validator|presenter|card_store`）命中 0；`tests/replay/` 不存在；38/39/51 实体文件不存在（`crates/application/src/cards/`、`crates/application/src/citation/`、`crates/ui-bridge/src/card_surface*` 均 `test ! -e`）；`gate/**` 无 UI/窗口句柄/焦点符号
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_keyword keyword_card_ownership_is_outside_gate` 退 0；`grep -rnE "composer|validator|presenter|card_store" crates/application/src/gate | wc -l` → `0`；`test ! -d crates/application/tests/replay; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-40.5 静态判据 S-01~S-05 同批复跑** · `crates/application/src/gate/**`、`crates/domain/src/model/gate_keyword.rs`
  - 追溯：AC-B4-46、AC-B4-56、AC-B4-57 · design §4.7.2
  - DoD：S-01 `grep -c '^pub fn ' api.rs` = **4**；S-02 `api.rs` 零 diff；S-03 57 禁词 4 项 = 0；S-04 49 禁词 5 项 = 0；S-05 `test ! -d tests/replay` 退 0
  - 证据：`[ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ] && git diff --quiet HEAD -- crates/application/src/gate/api.rs && [ "$(grep -rnE 'composer|validator|presenter|card_store' crates/application/src/gate | wc -l | tr -d ' ')" = "0" ] && [ "$(grep -rnE 'composer|validator|outbox|on_demand|feedback_weights' crates/application/src/gate | wc -l | tr -d ' ')" = "0" ] && test ! -d crates/application/tests/replay; echo "exit=$?"` → `exit=0`

### W-B4-41 AC-B4-48 200 例回放（方案 A）

> 前置：W-B4-37（需 Allow 决策可用）。落点 `crates/application/tests/gate_replay.rs`（**追加**，复用 `mod common`）；`crates/application/src/gate/replay.rs` 保持零 diff（D-B4-P29）。

- [ ] **W-B4-41.1 测试内本地构造器（`NormalizedFragment` / `Insight` / 端口替身）** · `crates/application/tests/gate_replay.rs`
  - 追溯：AC-B4-48 · requirements §10.6-1/2/5 · design §4.6.1/§4.6.4（D-B4-P13 收窄）
  - 改动内容：`fn fragment(index: u32) -> NormalizedFragment`（`text = format!("{}%", 10 + index % 90)`，`session = SessionId::new("replay-session")`、`reference = UtteranceRef::new(format!("r{index:03}"))`、`span = TextRange{start_ms: index as u64 * 10, end_ms: … + 100}`、`confidence = 0.90`、`speaker = SpeakerRef::anonymous("A")`、`dual_channel=false`、`origin_pair=None`）；`fn insight(index: u32) -> Insight`（`Insight::new(InsightRef::new(format!("i{index:03}")), InsightKind::KeyInfo, ShortText::new(format!("要点{index}")), Confidence::new(0.9), vec![UtteranceRef::new(format!("r{index:03}"))], WallClock::from_unix_millis(0))`）；`struct FailingSanitizer`/`struct FailingSink`（返回 `Err`，复用 `gate_outbound.rs` 范式）；`const SEED: u64 = 7;`
  - 禁止：`std::fs`/`File::`/`println!`/`dbg!`/`Insight::now()`/`WallClock::now()`；`GateInputFixture` 不得新增文本字段（23 字段不变）
  - DoD：`grep -c 'std::fs\|File::' tests/gate_replay.rs` = 0；`grep -c 'println!\|dbg!'` = 0；`grep -c 'WallClock::now'` = 0；`GateInputFixture` 字段数仍 23
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_replay` 退 0（编译通过 + 既有 3 用例仍绿）

- [ ] **W-B4-41.2 静态期望映射与"不可构造"登记** · `crates/application/tests/gate_replay.rs`
  - 追溯：AC-B4-48 · requirements §10.6-3/6 · design §4.6.2/§4.6.3/§4.6.5
  - 改动内容：`const EXPECTED_CONSTRUCTIBLE: [GateRejectReason; 21]`（G1 11：`PreFilterLowConfidence`/`ConsentMissing`/`AutoDisabled`/`Paused`/`SilentPeriod`/`ScreenShareProtection`/`SensitiveTerm`/`Duplicate`/`Suppressed`/`QuotaExhausted`/`NoProviderEnabled`；+`PolicyUnavailable`；+`LowRelevance`；+`Stale`；G4 3：`BudgetExhausted`/`GatewayCooling`/`Offline`；G5 4：`OutboundNotGranted`/`SanitizeFailed`/`AuditUnavailable`/`ProviderNotAllowed`）；`const NON_CONSTRUCTIBLE: [GateRejectReason; 2] = [DomainBlocked, RetentionExpired];`；`fn expected_for(stage_slot: u32, variant: u32) -> ExpectedDecision`（**纯静态 match**，不调用任何 4 函数，R-K-08）
  - DoD：`EXPECTED_CONSTRUCTIBLE.len() + NON_CONSTRUCTIBLE.len() == GateRejectReason::ALL.len()`（23）；两数组无交集（运行期 `iter().all(|r| !EXPECTED_CONSTRUCTIBLE.contains(r))`）；`expected_for` 不含 `evaluate_gate|detect_gaps|score_relevance|record_from` 调用
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_replay --no-run; echo "exit=$?"` → `exit=0`（常量与静态映射编译通过）；覆盖断言在 **W-B4-41.4** 落地

- [ ] **W-B4-41.3 200 例生成器 + 测试内 runner** · `crates/application/tests/gate_replay.rs`
  - 追溯：AC-B4-48 · requirements §10.6-3 · design §4.6.2（两维派生）+ D-B4-P29
  - 改动内容：`fn build_cases() -> Vec<GateReplayCase>`：`for index in 0..200 { stage_slot = index % 6; variant = (index / 6) % 12; phase = index / 72; case_id = format!("case-{index:03}") }`，`fixture` 由 `base_fixture(phase)` 改造（`now_ms = 10_000 + [0, 600_000, 86_400_001][phase]`、`budget.session_outbound_used = [0, 119, 0][phase]`、`budget.global_window_used = [0, 0, 4][phase]`）；`expected = expected_for(stage_slot, variant)`；`fn run(cases: &[GateReplayCase]) -> ReplayReport`（对每例：`Harness` 复原 → `detect_gaps` → `evaluate_gate` → 与 `expected` 比对 `DecisionKind`/`reason`/`stopped_at`，累计 `total/matched/mismatched/mismatch_cases`，`seed = SEED`）
  - 禁止：runner 不得写文件、不得打印文本、不得读系统时间；`mismatch_cases` 只存 `case_id`
  - DoD：`build_cases().len() == 200`；`case_id` 唯一（`BTreeSet` 长度 200）；`run` 返回 `total == 200`、`mismatched == 0`、`seed == 7`；新增 smoke 用例 `replay_runner_reports_two_hundred_cases`（非 AC 名，先于 41.4/41.5 可验收）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_replay replay_runner_reports_two_hundred_cases` 退 0

- [ ] **W-B4-41.4 `replay_200_cases_cover_all_reject_reasons`（AC-48 覆盖）** · `crates/application/tests/gate_replay.rs`
  - 追溯：AC-B4-48 · requirements §10.6-3 · design §4.6.3/§4.6.5
  - DoD：从 200 例的实际 `Deny{reason}` 收集集合 → 覆盖 `EXPECTED_CONSTRUCTIBLE` **21/21**（每项 ≥1）；`Allow` ≥1 且 4 子形各 ≥1（`Auto` 常规 / `OnDemand` / `per_stage[1..2]=Skipped` / `queued=true`）；`NON_CONSTRUCTIBLE` 两项**不出现**在 200 例的 expected 与 actual 中；跳级子形断言 `per_stage[1]/[2] == Skipped` 且决策 `Allow`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_replay replay_200_cases_cover_all_reject_reasons` 退 0

- [ ] **W-B4-41.5 `replay_200_cases_is_byte_identical`（AC-48 字节一致）** · `crates/application/tests/gate_replay.rs`
  - 追溯：AC-B4-48 · requirements §10.6-4 · design §4.6.4
  - DoD：同一 `let cases = build_cases();` 连续 `run(&cases)` 两次 → `serde_json::to_vec(&first).unwrap() == serde_json::to_vec(&second).unwrap()`（`Vec<u8>` 比较，不用 `Debug` 字符串）；`total=200`/`matched=200`/`mismatched=0`/`seed=7`；`mismatch_cases.is_empty()`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test gate_replay replay_200_cases_is_byte_identical -- --nocapture` 退 0；连跑两次同命令均退 0

- [ ] **W-B4-41.6 零转录与"不落盘"断言** · `crates/application/tests/gate_replay.rs`
  - 追溯：AC-B4-48、AC-B4-56 · requirements §10.6-5 · design §4.6.4
  - DoD：测试体内无 `std::fs`/`File::`/`OpenOptions`；断言消息只含 `case_id`/`index`/原因码（`assert!(cond, "case-{index:03} {reason:?}")` 形式）；`--rule=zero-transcript` 退 0；`crates/application/tests/gate_replay.rs` 的 `ReplayReport` 键集合仍 5
  - 证据：`bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`；`grep -cE 'std::fs|File::|OpenOptions' crates/application/tests/gate_replay.rs` = 0

### W-B4-42 脚本扫描面、target 清单与三态收口

> 前置：W-B4-34~41 全绿。**本任务是 P4 唯一串行收口点**：一次改脚本、一次登记，避免中途红绿抖动。

- [ ] **W-B4-42.1 `GATE_KEYWORD` 纳入扫描面（X-01）** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-56、AC-B4-57 · design §4.8.3（X-01）、§4.1.2 · D-B4-P26
  - 改动内容：新增 `GATE_KEYWORD="crates/domain/src/model/gate_keyword.rs"`（紧邻 `GATE_REPLAY`）；`require_core_faces()` 增加 `need_file "${GATE_KEYWORD}" || ok=1`；`core_files()` 的 `printf` 增加 `${GATE_KEYWORD}`；`rule_zero_transcript` 的 `files` 增加 `${GATE_KEYWORD}`；**规则数仍 7 条、`--rule` 名称与退出码语义不变**
  - 禁止：不得修改 `check-capture-offline.sh`/`check-b3-gates.sh`/`check-mac-scope.sh` 的规则语义；不得新增第 8 条 `--rule`
  - DoD：`grep -c 'GATE_KEYWORD=' check-b4-gates.sh` = 1；`grep -c 'GATE_KEYWORD'` ≥3（定义 + `require_core_faces` + `core_files` + `zero-transcript` = 4）；`--rule=source`/`--rule=budget`/`--rule=zero-transcript` 各退 0；删除 `gate_keyword.rs` 时 `--rule=source` 退 **2**（扫描面缺失，`mktemp` 内模拟或 review 口径）
  - 证据：`bash scripts/check-b4-gates.sh --rule=source && bash scripts/check-b4-gates.sh --rule=budget && bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`；`grep -n 'GATE_KEYWORD' scripts/check-b4-gates.sh` 输出 ≥3 行

- [ ] **W-B4-42.2 `NEW_TARGETS` 19→20（X-02）** · `scripts/test-name-inventory.sh`
  - 追溯：AC-B4-57 · design §4.8.3（X-02）、§4.1.2 · D-B4-P26
  - 改动内容：`NEW_TARGETS='… gate_outbound gate_replay gate_keyword'`（尾部追加，token 数 19→20）
  - DoD：`grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//" | wc -w` = **20**；`gate_keyword` 唯一出现 1 次；`git diff --stat HEAD -- scripts/test-name-inventory.sh` 仅 1 行变更
  - 证据：`grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//" | wc -w` → `20`；`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt; echo "exit=$?"` → `exit=0` 且输出含 `removed=0`

- [ ] **W-B4-42.3 `--rule=targets` 口径复核（6 项断言 + 20 项清单）** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-57、AC-B4-50 · spec §7.8/§7.9-2 · X-02
  - DoD：`--rule=targets` 退 **0**（6 项 `gate_*` 均存在，循环列表**不改**）；清单 20 项且含 `gate_keyword`；`--rule=targets` 不因多出的 target 报错
  - 证据：`bash scripts/check-b4-gates.sh --rule=targets; echo "exit=$?"` → `exit=0`；`bash scripts/check-b4-gates.sh --rule=targets | wc -l` → `0`（无 `TARGETS-MISSING` 输出）

- [ ] **W-B4-42.4 `b4-status.md` 新增 §11（P4 快照 + 7 条 AC 三态 + U-01/U-02 关闭）** · `docs/progress/b4-status.md`
  - 追溯：AC-B4-48、AC-B4-51~57 · spec §7.10/§7.11 · design §4.7.3/§4.10
  - 改动内容：追加 `## §11 P4 增量快照（W-B4-34 ~ W-B4-42）`，含 §11.1 命令结果表（`gate_keyword` 13 用例、`gate_replay` 5 用例、7 规则、7 target）、§11.2 七行三态（AC-B4-48/51~57 → `passed`，各附用例名）、§11.3 U-01/U-02 关闭登记（U-03~U-08 保持 open 及理由）、§11.4 偏差 D-B4-P19~P32、§11.5 不宣称清单（离线通过 ≠ 38/39/51 质量）
  - 禁止：§11 内不得写任何构造关键词文本/摘要/URL/姓名；只写 AC 编号、用例名、原因码、计数
  - DoD：`grep -c '^| AC-B4-' b4-status.md` ≥ **57**；`sed -n '/^## §11/,$p' b4-status.md | grep -cE 'AC-B4-(48|5[1-7])'` ≥ 8（标题行 + 7 行）；`grep -c 'U-0[1-8]'` ≥ 8；`--rule=zero-transcript` 退 0；`--rule=doc-plan` 退 0
  - 证据：`bash scripts/check-b4-gates.sh --rule=zero-transcript && bash scripts/check-b4-gates.sh --rule=doc-plan; echo "exit=$?"` → `exit=0`；`grep -c '^| AC-B4-' docs/progress/b4-status.md` ≥57

- [ ] **W-B4-42.5 P4 出口收口（7 target / 7 规则 / 全量回归）** · `crates/**`、`scripts/**`
  - 追溯：AC-B4-39、AC-B4-43、AC-B4-46、AC-B4-48、AC-B4-51~57 · design §4.7.2（S-13/S-14/S-15）、§4.10
  - DoD：7 个 target（含 `gate_keyword`）逐个退 0；七条规则 7×0 且 `--self-test` 退 1；`cargo test --workspace` 退 0、`failed=0`、`ignored=9`；`removed=0`；`fmt --check`/`clippy -D warnings`/`check-layering.sh`/`scan-secrets.sh`/`cargo deny` 五项全 0；`Cargo.toml`/`Cargo.lock` 零 diff；`understand/store.rs`/`deduplicator.rs` 零 diff
  - 证据：`for t in gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay gate_keyword; do "$CARGO" test -p meeting-copilot-application --test "$t" || exit 1; done; echo "exit=$?"` → `exit=0`；`for r in source zero-transcript budget outbound record targets doc-plan; do bash scripts/check-b4-gates.sh --rule="$r" || exit 1; done; echo "exit=$?"` → `exit=0`
  - 证据产物：全部退出码与日期写入 `b4-status.md` §11.1；阶段结论写入 §11.2

---

## P5 · 39 引用对齐校验契约与只读判定（增量 AC-B4-58~64）

> 阶段：编码任务规划的**第三个增量批次**（P4 之后）。需求来源：`requirements.md` §11（REQ-B4C-01~04、AC-B4-58~64）、`spec.md` §5.7 / §6.14~6.15 / §7.9-5 / §7.10 / §7.12、`design.md` §五（5.0~5.11，含 §5.10.4 裁决 X-03~X-06）。进度基线：`docs/progress/b4-status.md` §10.1 U-04（**规格已定，待实现**）、§11（P4 快照：721 passed / 0 failed / 9 ignored、`NEW_TARGETS`=20、`gate_keyword` 14 passed、`api.rs` 4 函数）、§12（D-B4-P34~P37、AC-B4-58~64 全 `pending`）。
> **范围**：只做 39 的**domain 契约 + application 只读判定 + 唯一投影 + 点击前置证据**。**不建** 38 合成（`crates/application/src/cards/`）、**不建** 51 UI（`crates/ui-bridge/**` 卡片面）、**不建** 52 存储、**不建** `crates/application/tests/replay/`；**不新增** trait/端口；**不新增** crate。
> **与 U-04 的关系**：**U-04 由本 P5 全部闭合**（W-B4-43~48）；AC-B4-58~64 七条由 `pending` 转 `passed`。
> **与 U-03 / U-05 的关系**：**均保持 open，不得顺手关闭**。U-03（38 卡片合成）—— AC-58 的"卡片只读"由测试内**本地构造**的 `KeywordCardDraft`（13 字段）验证，不依赖 38 实体；U-05（51 点击 UI）—— AC-60 只断言 `ClickState` **契约值**（`Enabled` / `Disabled{reason}`）与 `card_drafts` / `body_visible`，不建 UI 文件、不验交互质量与投屏表现。U-06 保持 `tests/replay/` 不存在；U-07 / U-08 不承接（9 个 `#[ignore]` 不变）。
> **P5 出口判据（缺一即 P5 未完成）**：① `citation_align` target **12 passed / 0 failed**；② `basis` **11/11** 可构造且每输入唯一；③ 投影 3→3 单射、`CONTRACT_UNVERIFIED` 为唯一入口；④ 七条规则 7×0、`--rule=targets` 退 0 且 `NEW_TARGETS`=**21**；⑤ `api.rs` 4 个 `pub fn` 且零 diff、14/7/23/5/32/23 全不变、`tests/replay/` 仍不存在；⑥ 全量 `cargo test --workspace` `failed=0` / `ignored=9`、`--diff` `removed=0`。

### P5 公共断言（每个 shell 会话执行一次，后续任务直接引用）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
CITE_MODEL="crates/domain/src/model/citation.rs"
CITE_APP_DIR="crates/application/src/citation"
CITE_TEST="crates/application/tests/citation_align.rs"

# 与 P4 同口径（沿用 P4 公共断言的 kw_fields / kw_variants 定义，勿重复定义）
kw_fields() { awk -v s="pub struct $1 {" 'index($0,s)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    pub [a-z_]+:'; }
kw_variants() { awk -v e="pub enum $1 {" 'index($0,e)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    [A-Z][A-Za-z0-9]*(\(|,)'; }
# 39 契约顶层类型项数：目标 6 = 3 struct + 3 enum（ClaimRef / CitationCheckInput / CitationAlignmentResult + 3 enum）
cite_items() { grep -cE '^pub (struct|enum) ' "$CITE_MODEL"; }
# NEW_TARGETS token 数：目标 21
new_targets_count() { grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//" | wc -w; }
```

> **零转录纪律（P5 加严）**：`$CITE_MODEL`、`$CITE_APP_DIR/**`、`$CITE_TEST`、`docs/progress/b4-status.md` §13 内**不得**出现协议前缀、`sk-` 形态 Key、`Authorization: Bearer`、模型名前缀、`request_id:` 形态。反例常量只允许 `"a://b"`（非法 scheme）、`"user@host"`（@）、`"has space"`（空白）、`""`（空）这类**不含被禁模式**的短串。理由：W-B4-47.2 把 `$CITE_TEST` 纳入 `--rule=zero-transcript` 后，测试文件与源码同权（design §5.9.1）。
> **禁词纪律**：`$CITE_APP_DIR/**` 与 `$CITE_MODEL` 中 `composer|presenter|card_store|outbox|on_demand|feedback_weights` 命中必须为 0（**注释里也不许写**）；`$CITE_APP_DIR/**` **不得**出现 `use crate::gate`（分层倒挂，AC-63 / design §5.2.2）。
> **X-03 提交纪律（硬约束）**：`check-b4-gates.sh` 的 `require_core_faces()` 一旦加入 `CITATION_APP_DIR`/`CITATION_MODEL`，39 文件缺失即令**全部** `--rule` 退 **2**。因此 **W-B4-47 必须在 W-B4-43/44 落盘之后执行，并与 43/44/45/46/47 同批提交**；禁止先改脚本再建文件，禁止单独提交脚本改动（否则 P4 基线由绿转红，7 条规则中途退 2）。
> **X-05 口径（已冻结，不再权衡）**：`--rule=targets` 的循环列表**保持现状 7 项**（`gate_prefilter … gate_keyword`，P4 实况已含 `gate_keyword`），**不追加** `citation_align`；`citation_align` 由 `NEW_TARGETS` token 计数（AC-64）与 target 实跑断言。`grep -c 'citation_align' scripts/check-b4-gates.sh` 必须为 **0**。

### P5 依赖图（单向，不可倒置）

```text
W-B4-43 ─► W-B4-44 ─► W-B4-45 ─► W-B4-46 ─► W-B4-47 ─► W-B4-48
内部：43.1 ─► {43.2 ∥ 43.3} ─► 43.4 ─► 43.5 ─► 43.6
      44.1 ─► 44.2 ─► 44.3 ─► 44.4
      45.1 ─► 45.2
      46.1 ─► {46.2 ∥ 46.3} ─► 46.4 ─► 46.5 ─► 46.6 ─► 46.7
      47.1 ─► 47.2 ─► {47.3 ∥ 47.4}
      48.1 ─► 48.2 ─► 48.3 ─► 48.4
```

- **不可压缩三点**：43.6 的 `model/mod.rs` +2 行必须先于 44.1（否则 `meeting_copilot_domain::model::citation` 不可见）；44.4 的 `CitationDiagnostics` 必须先于 46.x（否则测试无法断言 `basis` 分布）；45.1 的 `project_alignment` 必须先于 46.3/46.4（AC-59/60 的唯一投影入口）。
- **可并行**：46.2（AC-58 只读）与 46.3（AC-59 三态）互不依赖；47.3（inventory）与 47.4（targets 口径）互不依赖。
- **唯一串行收口**：W-B4-47 与 W-B4-48 必须最后执行（X-03 扫描面 + token + 三态登记一次收口）。

### W-B4-43 domain 契约落位（AC-B4-58/59）

> 前置：无（P5 第一项）。落点 `$CITE_MODEL`（**新文件**，6 顶层类型）；`gate.rs` / `gate_keyword.rs` / `gate_replay.rs` / `enums.rs` 零 diff。

- [ ] **W-B4-43.1 三个 39 专属封闭枚举与 `ALL`/`verdict()`/`index()`** · `$CITE_MODEL`
  - 追溯：AC-B4-59 · spec §6.15-1/2 · requirements §11.4.2-1/2 · design §5.3.1、§5.5.2、D-B4-P42
  - 改动内容：新建文件，顶端 `//! 39 引用对齐契约：只读、离线、确定性。`；`AlignmentVerdict`（`Passed`/`Failed`/`Unknown`）、`AlignmentBasis`（11 项，**顺序严格等于 §5.5.2 表号 1..11**：`SupportedByOfficial`/`SupportedByAuthority`/`SupportedByGeneral`/`Contradicted`/`TermMismatch`/`ConflictingSources`/`NoReference`/`ReferenceUnavailable`/`InsufficientEvidence`/`JudgeUnavailable`/`InputContractViolation`）、`JudgeCapability`（`Available`/`Unavailable`）；三个枚举均 `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]` + `#[serde(rename_all = "snake_case")]`；`impl AlignmentVerdict { pub const ALL: [Self; 3] }`；`impl AlignmentBasis { pub const ALL: [Self; 11]; pub const fn verdict(self) -> AlignmentVerdict; pub const fn index(self) -> usize }`
  - 禁止：不得新增第 4 个 verdict 或第 12 个 basis；不得出现 `panic!(` / `todo!(` / `unimplemented!(`（`--rule=source` 覆盖本文件）；不得写 `LLM_` 前缀常量（`--rule=budget`）
  - DoD：`cite_items` ≥3（本任务后=3）；`kw_variants AlignmentVerdict "$CITE_MODEL"`=3、`AlignmentBasis`=11、`JudgeCapability`=2；`AlignmentBasis::ALL` 逐项 `verdict()` 与 §5.5.2 映射表一致（11/11）；`index()` 为 0..=10 且互不重复
  - 证据：`"$CARGO" build -p meeting-copilot-domain; echo "exit=$?"` → `exit=0`；`kw_variants AlignmentBasis "$CITE_MODEL"` → `11`
  - 证据产物：退出码写入 `b4-status.md` §13.1

- [ ] **W-B4-43.2 `ClaimRef` 值对象（构造期拒绝非法形态）** · `$CITE_MODEL`
  - 追溯：AC-B4-58、AC-B4-61 · spec §6.14-8/9 · design §5.3.1、§5.5.4、D-B4-P41
  - 改动内容：`pub struct ClaimRef(String)`（字段**私有**）；`pub enum ClaimRefError { Empty, Whitespace, AtSign, SchemeSeparator, TooLong }`（`Debug` + `Display`，**不打印入参原文**）；`pub fn new(raw: impl Into<String>) -> Result<Self, ClaimRefError>`：空 → `Empty`；`chars().count() > 128` → `TooLong`；含任意空白 → `Whitespace`；含 `@` → `AtSign`；含 `"://"` → `SchemeSeparator`；`pub fn as_str(&self) -> &str`
  - 禁止：**不得** `Serialize`/`Deserialize`（防外泄）；错误信息不得包含入参；不得出现 `title` / `excerpt` / `content` / `snippet` / `query` 字面量（含注释，`grep -cEi` 判据）
  - DoD：`"a://b"`→`Err(SchemeSeparator)`、`"user@host"`→`Err(AtSign)`、`"has space"`→`Err(Whitespace)`、`""`→`Err(Empty)`、129 字→`Err(TooLong)`、`"claim-0001"`→`Ok` 且 `as_str()` 等值
  - 证据：`"$CARGO" test -p meeting-copilot-domain claim_ref_` 退 0；`grep -cEi 'url|title|excerpt|content|snippet|query' "$CITE_MODEL"` → `0`

- [ ] **W-B4-43.3 `CitationCheckInput`（封闭 8 字段）+ 手写 `Debug` + `validate_shape`** · `$CITE_MODEL`
  - 追溯：AC-B4-58、AC-B4-61 · spec §6.14-1~8 · design §5.3.1、§5.7.1、D-B4-P39/P41
  - 改动内容：`pub struct CitationCheckInput` 字段**顺序固定**为 `card_id: CardRef` / `keyword_ref: KeywordRef` / `normalized_term: NormalizedTerm` / `keyword_kind: KeywordKind` / `references: Vec<ReferenceView>` / `now_ms: u64` / `judge_capability: JudgeCapability` / `claim_ref: Option<ClaimRef>`（**恰 8 个公有字段**）；手写 `impl core::fmt::Debug`，白名单 7 项：`card_id`、`keyword_ref`、`keyword_kind`、`reference_count`（由 `references.len()` 现算）、`now_ms`、`judge_capability`、`has_claim_ref`（`bool`）；`pub fn validate_shape(&self) -> bool` **只判**"引用 ID 不重复"（`BTreeSet` 长度 == `references.len()`），不读 `normalized_term`、不排序
  - 禁止：**不得** `#[derive(Debug)]`；**不得** `Serialize`/`Deserialize`；`Debug` 输出不得含 `normalized_term` 的取值、不得含 `claim_ref` 文本；`validate_shape` 内不得 `sort`/`sort_by`（排序归 application，design §5.4.1）；不得静默去重
  - DoD：`kw_fields CitationCheckInput "$CITE_MODEL"` = **8**；`grep -c 'derive(Debug' "$CITE_MODEL"` = 0；`format!("{:?}", input)` 不含 `normalized_term.as_str()` 且不含 `claim_ref.as_str()`；重复 `reference_id` → `validate_shape()==false`；唯一 → `true`
  - 证据：`"$CARGO" test -p meeting-copilot-domain citation_input_` 退 0；`kw_fields CitationCheckInput "$CITE_MODEL"` → `8`

- [ ] **W-B4-43.4 `CitationAlignmentResult`（封闭 6 字段）+ 版本常量** · `$CITE_MODEL`
  - 追溯：AC-B4-58、AC-B4-61、AC-B4-62 · spec §6.15-1~9 · design §5.3.1、D-B4-P42
  - 改动内容：`pub const CITATION_CONTRACT_VERSION: u16 = 1;`；`pub struct CitationAlignmentResult { pub alignment: AlignmentVerdict, pub basis: AlignmentBasis, pub reference_count: u32, pub accessible_reference_count: u32, pub checked_at_ms: u64, pub contract_version: u16 }`，`#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]`
  - 禁止：不得含 `references` / `normalized_term` / 任何文本字段；`contract_version` 不得为字符串/模型名/请求标识
  - DoD：`kw_fields CitationAlignmentResult "$CITE_MODEL"` = **6**；`serde_json::to_value(&result)` 键集合排序后等于 `{alignment, basis, checked_at_ms, contract_version, reference_count, accessible_reference_count}`；`CITATION_CONTRACT_VERSION == 1`
  - 证据：`"$CARGO" test -p meeting-copilot-domain citation_result_` 退 0；`kw_fields CitationAlignmentResult "$CITE_MODEL"` → `6`

- [ ] **W-B4-43.5 投影纯映射、`CONTRACT_UNVERIFIED` 与 4 个谓词（X-04 方案 A）** · `$CITE_MODEL`
  - 追溯：AC-B4-59 · spec §5.7.1-6 / §6.15-1 · requirements §11.4.3-1 · design §5.3.2、§5.6.1、D-B4-P38（**X-04 已冻结：方案 A，不改既有枚举**）
  - 改动内容：`impl AlignmentVerdict { pub const fn to_contract(self) -> CitationAlignment }`（`Passed→Passed`、`Failed→Failed`、`Unknown→CitationAlignment::Unknown`）；`pub const CONTRACT_UNVERIFIED: CitationAlignment = CitationAlignment::Unknown;`（语义名 `Unverified`）；`impl CitationAlignment { pub const ALL: [Self; 3] }`（**写在 `$CITE_MODEL`，`gate_keyword.rs` 零 diff**）；`pub const fn is_official_tier(t: ReferenceTrustTier) -> bool`（`== Official`）、`is_authority_tier`（`Official ∨ AuthoritativeMedia`）、`is_general_tier`（`GeneralWeb ∨ UserGenerated`）、`is_accessible(s: ReferenceAccessStatus) -> bool`（`== Available`）
  - 禁止：**不得**改 `crates/domain/src/model/gate_keyword.rs`（零 diff）；**不得**新增 `Unverified` 变体或改 serde 值；**不得**出现第二个 `impl CitationAlignment` 常量定义（`grep -c 'impl CitationAlignment' "$CITE_MODEL"` 必须=1）
  - DoD：`to_contract` 三输入 → 三个**互不相等**的 `CitationAlignment`（单射）；`to_contract(Unknown) == CONTRACT_UNVERIFIED`；`AlignmentVerdict::ALL.len()==3` 且 `CitationAlignment::ALL.len()==3`；4 谓词 × 4 档（3 档 + `Unavailable`）真值表逐项正确
  - 证据：`"$CARGO" test -p meeting-copilot-domain alignment_projection_ citation_tier_` 退 0；`git diff --quiet HEAD -- crates/domain/src/model/gate_keyword.rs; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-43.6 domain 单测 6 条 + `model/mod.rs` +2 行** · `$CITE_MODEL`、`crates/domain/src/model/mod.rs`
  - 追溯：AC-B4-58、AC-B4-59 · design §5.3.4（I-39-01/02/06/07）、§5.9.3
  - 改动内容：`#[cfg(test)] mod tests` 内 6 个用例名**逐字**：`citation_input_has_eight_fields`、`citation_result_has_six_fields`、`citation_basis_has_eleven_variants`、`claim_ref_rejects_illegal_shapes`、`alignment_projection_is_injective`、`citation_tier_predicates_match_alias_table`；`model/mod.rs` 在 `pub mod gate_replay;` 后追加 `pub mod citation;` 与在 `pub use gate_replay::*;` 后追加 `pub use citation::*;`（**+2 / -0**）
  - 禁止：测试块必须紧跟 `#[cfg(test)]` 行（`strip_tests()` 依赖 `^#[cfg(test)]`）；测试内不得 `println!`/`dbg!`；不得改 `gate.rs` / `gate_keyword.rs` / `gate_replay.rs` / `enums.rs`
  - DoD：6 用例全绿；`cargo test -p meeting-copilot-domain` 退 0（既有用例不回归）；`git diff --numstat HEAD -- crates/domain/src/model/mod.rs` 为 `2  0`；四文件零 diff
  - 证据：`"$CARGO" test -p meeting-copilot-domain citation_ claim_ref_ alignment_projection_` 退 0；`git diff --numstat HEAD -- crates/domain/src/model/mod.rs`
  - 证据产物：`6 passed` 与 numstat 写入 `b4-status.md` §13.1

### W-B4-44 application 只读校验（AC-B4-59/62）

> 前置：W-B4-43 全绿（`citation` 已可 `use`）。落点 `$CITE_APP_DIR`（**新目录**，2 文件）+ `lib.rs` +1 行；`gate/**` 零 diff。

- [ ] **W-B4-44.1 模块落位与最小可编译骨架** · `$CITE_APP_DIR/mod.rs`、`$CITE_APP_DIR/align.rs`、`crates/application/src/lib.rs`
  - 追溯：AC-B4-62、AC-B4-63 · design §5.2.2、§5.4.1/§5.4.3、D-B4-P43
  - 改动内容：`align.rs` 顶端 `//! 39 引用对齐：只读、离线、确定性；不生成卡片、不写记录。`，实现 `pub fn check_citation_alignment(input: &CitationCheckInput) -> CitationAlignmentResult` 的**最小合法实现**（6 字段装配，`alignment=Unknown`、`basis=InsufficientEvidence`，**不用占位宏**）与 `pub fn project_alignment(result: &CitationAlignmentResult) -> CitationAlignment { result.alignment.to_contract() }`；`mod.rs` **4 行**：`pub mod align;` + `pub use align::{check_citation_alignment, project_alignment, CitationDiagnostics};`；`lib.rs` 在 `pub mod capture;` 前插入 `pub mod citation;`（**+1 / -0**，**不**加 `pub use`）
  - 禁止：**不**在 `lib.rs` 写 `pub use citation::*`；**不**在 `gate/mod.rs` 增行；`$CITE_APP_DIR/**` 不得 `use crate::gate`；不得定义 trait/端口；文件名必须 `mod.rs`
  - DoD：`test -d "$CITE_APP_DIR"` 退 0；`grep -c 'pub mod citation;' crates/application/src/lib.rs` = 1；`cargo build -p meeting-copilot-application` 退 0；`grep -rn 'use crate::gate' "$CITE_APP_DIR" | wc -l` → `0`
  - 证据：`"$CARGO" build -p meeting-copilot-application; echo "exit=$?"` → `exit=0`；`git diff --quiet HEAD -- crates/application/src/gate; echo "exit=$?"` → `exit=0`
  - 说明：本任务的唯一目的是让 W-B4-47 的 `require_core_faces()` 有面可依（X-03）；逻辑在 44.2~44.4 填充，**签名不变**。

- [ ] **W-B4-44.2 判定主流程 S0~S4（结构/存在性/可访问性/能力位）** · `$CITE_APP_DIR/align.rs`
  - 追溯：AC-B4-59 · spec §6.15-3/4/7 · design §5.2.3（机器口径 1~4）、§5.5.1、§5.5.4
  - 改动内容：私有 `fn sort_references(&[ReferenceView]) -> Vec<ReferenceView>`（`sort_by_key(|r| r.reference_id.clone())`，**稳定**，不 `dedup`）；私有 `fn resolve_basis(input, refs) -> (AlignmentVerdict, AlignmentBasis)`；序 1 `!input.validate_shape()` → `Unknown/InputContractViolation`；序 2 `refs.is_empty()` → `Unknown/NoReference`；序 3 `accessible.is_empty()` → `Unknown/ReferenceUnavailable`；序 4 `judge_capability == Unavailable` → `Unknown/JudgeUnavailable`；结果 4 个派生字段一次装配（`reference_count = refs.len() as u32`、`accessible_reference_count = count(is_accessible)`、`checked_at_ms = input.now_ms`、`contract_version = CITATION_CONTRACT_VERSION`）
  - 禁止：不得 `SystemTime::now` / `Instant::now` / `std::env` / `std::fs` / `HashMap`；不得 `unwrap` / `expect` / `panic!`（非测试区）；不得返回 `Result` / `Option`；序 2 必须早于序 4（空引用优先于能力位，§5.5.3）
  - DoD：4 条路径各构造 1 例（含 `references=[]` 且 `capability=Unavailable` 同时成立 → `NoReference`）；`reference_count == input.references.len()`；`accessible_reference_count` 只计 `Available`；`checked_at_ms == input.now_ms`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align` 退 0（此时 ≥4 passed）；`grep -c 'SystemTime\|Instant::now\|std::env' "$CITE_APP_DIR/align.rs"` → `0`

- [ ] **W-B4-44.3 S5~S9（术语结构、反驳、互斥、General、无信号）** · `$CITE_APP_DIR/align.rs`
  - 追溯：AC-B4-59 · spec §5.7.1-3/4/5 · design §5.3.3、§5.5.1/§5.5.2、D-B4-P40（**X-06 已接受：`ReferenceView.alignment` 是 `Failed` 三因的唯一上游信号**）
  - 改动内容：私有 `fn detect_term_mismatch(input: &CitationCheckInput) -> bool` 按 §5.3.3 五类**结构**规则：`Metric` 含 ASCII 数字或中文数字（一二三四五六七八九十百千万亿）/ `PolicyStandard` 含 `gb`/`iso`/`iec`/`t/`/`q/` + 数字或"第…号" / `Acronym` 归一化后纯 ASCII 字母且长度 2..=10 / `ProperNoun` 长度 ≥2 且非纯数字 / `InstitutionRole` 含机构职务后缀（公司/大学/学院/院/局/委/部/中心/主任/经理/教授）；序 5 命中 → `Failed/TermMismatch`；序 6 可访问引用中**恰一条** `alignment==Failed` 且 `Passed` 计数为 0 → `Failed/Contradicted`；序 7 `failed_n >= 2` 或（`failed_n >= 1 && passed_n >= 1`）→ `Failed/ConflictingSources`；序 8 可访问引用**全部** `is_general_tier` → `Unknown/SupportedByGeneral`；序 9 可访问引用 `alignment` 全为 `Unknown` → `Unknown/InsufficientEvidence`
  - 禁止：不得读取引用正文（8 项输入无此字段）；不得引入语义相似度/分词/向量；`TermMismatch` 只判结构（§5.3.3 口径声明）；序 5 必须早于序 6、序 6 早于序 7、序 7 早于序 8、序 8 早于序 9
  - DoD：五类反例各 1 例 → `Failed/TermMismatch`；`Contradicted` / `ConflictingSources`（两种触发）/ `SupportedByGeneral` / `InsufficientEvidence` 各 ≥1 例；叠加反例 2 组（术语冲突 + 单条反驳 → `TermMismatch`；全 General + 单条 Failed → `Contradicted`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align citation_alignment_is_three_state_and_fail_closed` 退 0（11 项 basis 全覆盖）

- [ ] **W-B4-44.4 S10~S11 支撑门槛 + `CitationDiagnostics`** · `$CITE_APP_DIR/align.rs`
  - 追溯：AC-B4-59、AC-B4-62 · spec §6.15-7 · design §5.5.2/§5.5.3、§5.4.2
  - 改动内容：序 10 存在可访问且 `is_official_tier` 且 `alignment==Passed` → `Passed/SupportedByOfficial`（**优先**）；序 11 存在可访问且 `is_authority_tier` 且 `alignment==Passed` → `Passed/SupportedByAuthority`；序 12 兜底 `Unknown/InsufficientEvidence`；`pub struct CitationDiagnostics { pub checks: u32, pub passed: u32, pub failed: u32, pub unknown: u32, pub bases: [u32; 11] }` + `impl CitationDiagnostics { pub fn record(&mut self, result: &CitationAlignmentResult) }`（`bases[result.basis.index()] += 1`）
  - 禁止：**不新增** `GateCounter`（`GateCounters::default().snapshot().len()` 仍 32）；`bases` 长度必须 11 且下标只由 `AlignmentBasis::index()` 给出；`record` 只增计数，不改判定
  - DoD：Official 与 Authority 同时可访问且均 `Passed` → `SupportedByOfficial`；仅 Authority → `SupportedByAuthority`；`snapshot().len()==32`；`record` 连调 3 次 → `checks==3` 且 `bases` 求和 == 3、`passed+failed+unknown==3`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align citation_basis_is_unique_and_ordered citation_counts_match_inputs` 退 0

### W-B4-45 唯一投影与点击前置边界（AC-B4-59/60）

> 前置：W-B4-44。**不改库内 `gate/**` 任何符号**；AC-60 的合取验证全部在测试侧。

- [ ] **W-B4-45.1 `project_alignment` 唯一入口收口** · `$CITE_APP_DIR/align.rs`、`$CITE_APP_DIR/mod.rs`
  - 追溯：AC-B4-59、AC-B4-60 · spec §5.7.1-6 · requirements §11.3.2-3 · design §5.6.1、D-B4-P38/P43
  - 改动内容：确认 `project_alignment` 为 `result.alignment.to_contract()` 单行转发（44.1 已建）；`mod.rs` 的 `pub use` 恰 3 个符号（`check_citation_alignment`/`project_alignment`/`CitationDiagnostics`）；文件内注释说明"38 只能调用本函数回填 `citation_alignment` 一项"
  - 禁止：不得返回 `Result`/`Option`；不得出现第二个 `pub fn project_alignment`；`$CITE_APP_DIR/**` 内不得出现裸 `CitationAlignment::Unknown`（只允许经 `CONTRACT_UNVERIFIED`）
  - DoD：`grep -rn 'pub fn project_alignment' crates/application/src | wc -l` → `1`；`grep -rn 'CitationAlignment::Unknown' "$CITE_APP_DIR" | wc -l` → `0`；`grep -c 'pub use align::' "$CITE_APP_DIR/mod.rs"` = 1（一行导出 3 符号）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align unknown_projects_to_unverified_only` 退 0；三条 grep

- [ ] **W-B4-45.2 点击前置边界与禁词的静态复核（只读复用，零库内改动）** · `$CITE_APP_DIR/**`
  - 追溯：AC-B4-60、AC-B4-63 · requirements §11.3.2-1/2/4 · design §5.6.2/§5.6.3/§5.6.4、§4.2.4（K-05/D-B4-P22）
  - 改动内容：**零库内符号新增**。仅在 `align.rs` 顶端 `//!` 补 2~3 行边界声明："39 只输出 `alignment`；可点击由 `keyword_card_admission` 的 7 项合取决定；`Failed`/`Unknown` 不撤回卡片"。点击矩阵 11 行（design §5.6.2）由 W-B4-46.4 用例逐行断言
  - 禁止：`$CITE_APP_DIR/**` 不得出现 `ClickState` / `KeywordCardStatus` / `KeywordCardDraft` / `click_state` / `card_status` / `summary` / `keyword_text` / `TitleText` / `ConclusionText`；**不得**改 `crates/application/src/gate/keyword.rs`（零 diff）；**不得**新增 `GateCounter`
  - DoD：`grep -rnE 'ClickState|KeywordCardStatus|KeywordCardDraft|click_state|card_status' "$CITE_APP_DIR" | wc -l` → `0`；`grep -rnE 'summary|keyword_text|TitleText|ConclusionText' "$CITE_APP_DIR" | wc -l` → `0`；`git diff --quiet HEAD -- crates/application/src/gate/keyword.rs; echo "exit=$?"` → `exit=0`
  - 证据：两条 grep 均为 `0`；`grep -c '^pub fn ' crates/application/src/gate/api.rs` → `4`

### W-B4-46 `citation_align` target 与 12 个用例（AC-B4-58~64）

> 前置：W-B4-43、W-B4-44、W-B4-45。新增 `$CITE_TEST`（**第 8 个 target**）。用例名与 design §5.8.1 **逐字一致**，共 12 个。

- [ ] **W-B4-46.1 target 骨架 + 本地构造器** · `$CITE_TEST`
  - 追溯：AC-B4-58、AC-B4-64 · design §5.8.1、§5.9.3
  - 改动内容：文件头 `//! 39 引用对齐校验（AC-B4-58~64，12 用例）。`；`mod common;`（仅 46.4 用 `common::Harness`）；本地构造器 `fn sample_card() -> KeywordCardDraft`（13 字段本地拼装：`summary: None`、`click_state: ClickState::Disabled(ClickDisabledReason::ReferenceUnverified)`、`gate_decision_ref: AllowTraceRef::with_trace(TraceId::new("t-card").unwrap(), None)`）、`fn reference(id: &str, tier: ReferenceTrustTier, status: ReferenceAccessStatus, alignment: CitationAlignment) -> ReferenceView`、`fn input_for(card: &KeywordCardDraft, refs: Vec<ReferenceView>, capability: JudgeCapability, now_ms: u64) -> CitationCheckInput`、`fn term_for(kind: KeywordKind, index: usize) -> String`（只允许 `format!("术语{index}")` 这类短词）
  - 禁止：不得 `#[ignore]`；不得 `std::fs` / `File::` / `println!` / `dbg!`；不得出现协议前缀（`//` 之外不得有 `://` 前缀块）；`assert_eq!` 不得直接打印 `normalized_term`/`claim_ref` 文本
  - DoD：`"$CARGO" test -p meeting-copilot-application --test citation_align --no-run` 退 0；`grep -c 'fn sample_card\|fn reference\|fn input_for\|fn term_for' "$CITE_TEST"` ≥4
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align --no-run; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-46.2 `citation_input_is_readonly_and_closed`（AC-58）** · `$CITE_TEST`
  - 追溯：AC-B4-58 · spec §7.12 AC-58 · design §5.8.1、§5.3.1
  - DoD：取一张 13 字段卡片，`let before = serde_json::to_value(&card)`；由卡片取 8 项构造 `CitationCheckInput` 并调用 39；`let after = serde_json::to_value(&card)`；断言 `before == after`（键集合与值前后全等）；断言输入 8 键（`serde_json::to_value` 不适用，改用 `kw_fields` 口径 + 构造处 8 个具名字段）；`grep -c '^pub fn ' crates/application/src/gate/api.rs` = 4；`--rule=record` 退 0
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align citation_input_is_readonly_and_closed` 退 0；`bash scripts/check-b4-gates.sh --rule=record; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-46.3 AC-59 两用例：三态 fail-closed 与投影唯一** · `$CITE_TEST`
  - 追溯：AC-B4-59 · spec §7.12 AC-59 · design §5.8.1、§5.5.2、§5.6.1
  - DoD（`citation_alignment_is_three_state_and_fail_closed`）：11 个输入覆盖 11 项 `basis`，每例断言 `(alignment, basis)` 唯一且与 §5.5.2 一致；收集 `BTreeSet` 后 `len()==11`；4 类非 `Passed` 反例（空引用 / 全不可访问 / 能力不可用 / 结构违规）`alignment != Passed`；`Passed ⇒ reference_count ≥1 ∧ accessible_reference_count ≥1`
  - DoD（`unknown_projects_to_unverified_only`）：5 个 `Unknown` basis（`NoReference`/`ReferenceUnavailable`/`JudgeUnavailable`/`InsufficientEvidence`/`InputContractViolation`）→ `project_alignment` 全等于 `CONTRACT_UNVERIFIED`；`CitationAlignment::ALL.len()==3` 且 `AlignmentVerdict::ALL.len()==3`；`to_contract` 三输入映射三**不同**变体
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align citation_alignment_is_three_state_and_fail_closed unknown_projects_to_unverified_only` 退 0

- [ ] **W-B4-46.4 AC-60 三用例：点击矩阵与"可展示不可点"** · `$CITE_TEST`
  - 追溯：AC-B4-60 · spec §7.12 AC-60 · requirements §11.3.2 · design §5.6.2/§5.6.3、§5.8.1、D-B4-P31/P32
  - 改动内容：复用 `common::Harness` 取 Allow 决策与候选（`Harness::new(10_000)` + 五条片段 + `outcome()`），再用 `keyword_card_admission(candidate, &decision, &TraceId::new("t-new").unwrap(), project_alignment(&result), &env)` 走既有 7 项合取；`KeywordCardEnv` 6 字段逐例改写
  - DoD（`citation_passed_enables_click_only_with_other_facts`）：39 `Passed` + 六项全真 → `Enabled`；`authorization_revoked=true` → `AuthorizationRevoked`；`Deny` 决策（无 `allow_ref`）→ `NotAllowed`；`screen_share=true` → `ScreenShareProtection`；断言 `Passed != Enabled`（`Enabled` 需要 7 项合取）
  - DoD（`citation_failed_still_displayed_but_disabled`）：39 `Failed` + Allow → `card_drafts == true`、`Disabled{ReferenceUnverified}`、`summary` 为 `None` 或 `KeywordCardDraft::NO_RELIABLE_SOURCE`、`summary_is_consistent() == true`
  - DoD（`citation_unknown_is_conservative`）：4 个 `Unknown` 来源（`NoReference`/`ReferenceUnavailable`/`JudgeUnavailable`/`InsufficientEvidence`）→ 投影 `CONTRACT_UNVERIFIED`、`card_drafts == true`、`Disabled{ReferenceUnverified}`、`!= CitationAlignment::Passed`
  - 禁止：不得新建 UI/合成/存储文件；不得在库内 import `gate::keyword`（只在测试侧调用）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align citation_passed_enables_click_only_with_other_facts citation_failed_still_displayed_but_disabled citation_unknown_is_conservative` 退 0

- [ ] **W-B4-46.5 AC-61 + AC-63 两用例：ID-only 与归属不越界** · `$CITE_TEST`
  - 追溯：AC-B4-61、AC-B4-63 · spec §7.12 AC-61/63 · design §5.7.1/§5.7.4、§5.6.4、§5.8.1
  - DoD（`citation_result_is_id_only`）：`serde_json::to_value(&result)` 键集恰 6 项；`format!("{:?}", &input)` 不含 `normalized_term.as_str()` 与 `claim_ref.as_str()`；`ReferenceView` JSON 键集恰 5 项；对结果 JSON 与 `Debug` 串做 6 类禁词检查命中 0
  - DoD（`citation_ownership_is_outside_b4_and_ui`）：`$CITE_APP_DIR` + `$CITE_MODEL` 的 6 项禁词（`composer`/`presenter`/`card_store`/`outbox`/`on_demand`/`feedback_weights`）命中 0；`test ! -d crates/application/tests/replay`；`GateCounters::default().snapshot().len() == 32`；`GateRecord` 14 / `GateAuditEvent` 7 / `GateInputFixture` 23 / `ReplayReport` 5 键集不含 39 侧字段（`alignment`/`basis`/`reference_count`/`accessible_reference_count`/`checked_at_ms`/`contract_version`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align citation_result_is_id_only citation_ownership_is_outside_b4_and_ui` 退 0

- [ ] **W-B4-46.6 AC-62 两用例：离线确定性与引用顺序无关** · `$CITE_TEST`
  - 追溯：AC-B4-62 · spec §7.12 AC-62 · design §5.7.2/§5.7.3、§5.8.1
  - DoD（`citation_check_is_offline_and_deterministic`）：同输入 100 次 `serde_json::to_vec(&result)` 逐字节相等（`Vec<u8>` 比较，**不用** `Debug` 字符串）；`basis` 与 `alignment` 100 次相同
  - DoD（`citation_ignores_reference_order`）：同一 `Vec<ReferenceView>` 的 3 种排列（原序、逆序、首尾交换）→ 结果 `serde_json::to_vec` 逐字节相等且 `basis` 相等
  - 禁止：测试内不得读系统时间/环境变量/文件；不得用 `HashSet` 收集
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align citation_check_is_offline_and_deterministic citation_ignores_reference_order` 退 0

- [ ] **W-B4-46.7 辅助两用例：`basis` 唯一优先级与计数一致** · `$CITE_TEST`
  - 追溯：AC-B4-59、AC-B4-64 · design §5.5.3、§5.8.1、§5.3.4（I-39-03/06）
  - DoD（`citation_basis_is_unique_and_ordered`）：8 组多因并存逐组断言唯一结果（空引用+能力不可用→`NoReference`；全不可访问+能力不可用→`ReferenceUnavailable`；术语冲突+单条反驳→`TermMismatch`；单条反驳+混合信号→`Contradicted`；全 General+Failed→`Contradicted` 或 `ConflictingSources`；Official+Failed→`Contradicted`；Official 与 Authority 均 `Passed`→`SupportedByOfficial`；全 `Unknown`+Official→`InsufficientEvidence`）
  - DoD（`citation_counts_match_inputs`）：0 / 1 / 3 条引用与混合可访问组合 → `reference_count == 输入 len()`、`accessible_reference_count` 只计 `Available`、`checked_at_ms == input.now_ms`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test citation_align citation_basis_is_unique_and_ordered citation_counts_match_inputs` 退 0；全量本 target 输出 `12 passed / 0 failed`

### W-B4-47 脚本扫描面与 target 清单（AC-B4-58/62/63/64）

> 前置：W-B4-43、W-B4-44（**X-03**：文件未落盘前改脚本会让 7 条规则全部退 2）。**本任务必须与 43/44/45/46 同批提交**。

- [ ] **W-B4-47.1 三个扫描面变量 + `citation_files()` + `require_core_faces()` + `core_files()`** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-62、AC-B4-63 · design §5.9.1、D-B4-P37、**X-03**
  - 改动内容：变量区（`GATE_KEYWORD=` 之后）新增 `CITATION_APP_DIR="crates/application/src/citation"`、`CITATION_MODEL="crates/domain/src/model/citation.rs"`、`CITATION_TEST="crates/application/tests/citation_align.rs"`；新增 `citation_files() { for file in "${CITATION_APP_DIR}"/*.rs; do [[ -f "${file}" ]] && printf '%s\n' "${file}"; done }`；`require_core_faces()` 追加 `need_dir "${CITATION_APP_DIR}" || ok=1` 与 `need_file "${CITATION_MODEL}" || ok=1`；`core_files()` 在 `gate_files` 后追加 `printf '%s\n' "${CITATION_MODEL}"` 与 `citation_files`
  - 禁止：**规则仍 7 条**；`--rule` 名称集合与退出码语义（0/1/2）不变；不得改 `rule_source` / `rule_budget` 的模式串；不得改 `check-capture-offline.sh` / `check-b3-gates.sh` / `check-mac-scope.sh`
  - DoD：`grep -c 'CITATION_MODEL=' scripts/check-b4-gates.sh` = 1；`grep -c 'CITATION_APP_DIR=' scripts/check-b4-gates.sh` = 1；`grep -c 'citation_files' scripts/check-b4-gates.sh` ≥ 3（定义 + `core_files` 调用 + 47.2 的 `zero-transcript` 调用）；`--rule=source` 与 `--rule=budget` 各退 0
  - 证据：`bash scripts/check-b4-gates.sh --rule=source && bash scripts/check-b4-gates.sh --rule=budget; echo "exit=$?"` → `exit=0`
  - 反例演练（不入库）：在 `mktemp -d` 内复制仓库并把 `$CITE_MODEL` 改名 → `--rule=source` 必须退 **2** 且打印 `SCAN-MISSING`

- [ ] **W-B4-47.2 `zero-transcript` 扫描面纳入 39 契约/实现/测试** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-61 · design §5.9.1、§5.7.1、D-B4-P37
  - 改动内容：`rule_zero_transcript()` 的 `files` 数组追加 `${CITATION_MODEL}`、`< <(citation_files)` 遍历结果、以及 `if [[ -f "${CITATION_TEST}" ]]; then files+=("${CITATION_TEST}"); fi`
  - 禁止：不得新增/放宽禁止模式；不得把 `$CITE_TEST` 改名以躲开扫描（`gate_*.rs` glob 不会覆盖它，必须显式加入）
  - DoD：`--rule=zero-transcript` 退 0；`grep -c 'CITATION_TEST' scripts/check-b4-gates.sh` ≥ 2（定义 + 引用）
  - 证据：`bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`
  - 反例演练（`mktemp -d`，不入库）：把含协议前缀的一行追加到 `$CITE_TEST` 副本 → 退 1

- [ ] **W-B4-47.3 `NEW_TARGETS` 20 → 21（追加 `citation_align`）** · `scripts/test-name-inventory.sh`
  - 追溯：AC-B4-64 · spec §7.9-5 · design §5.9.2、D-B4-P37
  - 改动内容：`NEW_TARGETS='… gate_replay gate_keyword'` 尾部追加 ` citation_align`
  - 禁止：不得改动既有 20 个 token 的顺序/拼写；`--diff` 语义不变
  - DoD：`new_targets_count` = **21**；`grep -c 'citation_align' scripts/test-name-inventory.sh` = 1；`git diff --stat HEAD -- scripts/test-name-inventory.sh` 仅 1 行变更
  - 证据：`new_targets_count` → `21`；`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt; echo "exit=$?"` → `exit=0` 且输出含 `removed=0`

- [ ] **W-B4-47.4 `--rule=targets` 循环**不改**（X-05 冻结）+ 七规矩复跑** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-58、AC-B4-64 · spec §7.9-5（"`--rule=targets` 仍只断言既有 6 项存在，多出的 `gate_keyword`/`citation_align` 不影响既有断言"）· design §5.9.1、§5.10.4 **X-05（已接受）**
  - 改动内容：**零改动**。`rule_targets()` 的循环保持现状 7 项：`gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay gate_keyword`
  - 禁止：不得把 `citation_align` 加入该循环（否则断言项 7→8，与 spec §7.9-5 字面冲突，且 `--rule=targets` 的失败输出由 0 行变 1 行）
  - DoD：`--rule=targets` 退 0 且输出行数 0；`grep -c 'citation_align' scripts/check-b4-gates.sh` = **0**；七条规则 7×0
  - 证据：`bash scripts/check-b4-gates.sh --rule=targets; echo "exit=$?"` → `exit=0`；`bash scripts/check-b4-gates.sh --rule=targets | wc -l` → `0`；`for r in source zero-transcript budget outbound record targets doc-plan; do bash scripts/check-b4-gates.sh --rule="$r" || exit 1; done; echo "exit=$?"` → `exit=0`

### W-B4-48 状态登记与 P5 出口收口（AC-B4-64）

> 前置：W-B4-43~47 全绿。

- [ ] **W-B4-48.1 `b4-status.md` 新增 §13（P5 快照 + 7 条 AC 三态 + U-04 关闭）** · `docs/progress/b4-status.md`
  - 追溯：AC-B4-64 · spec §7.10/§7.12 · design §5.8.3
  - 改动内容：追加 `## §13 P5 增量快照（W-B4-43 ~ W-B4-48）`，含 §13.1 命令结果表（`citation_align` 12 用例、8 target、7 规则、`NEW_TARGETS`=21、`api.rs`=4、`cargo test --workspace` 全量、`--diff` `removed=0`）、§13.2 七行 AC 三态（AC-B4-58~64 → `passed`，各附用例名）、§13.3 U-04 关闭登记 + U-03/U-05 保持 open 及理由、§13.4 偏差 D-B4-P38~P43 与 X-03~X-06、§13.5 不宣称清单（离线通过 ≠ 真实引用可访问、≠ 语义对齐质量、≠ 51 交互质量；不宣称 G3/G4/真机）
  - 禁止：§13 内不得写任何构造术语文本、协议前缀、姓名、联系方式；只写 AC 编号、用例名、原因码、计数
  - DoD：`grep -c '^## §13' docs/progress/b4-status.md` = 1；`sed -n '/^## §13/,$p' docs/progress/b4-status.md | grep -cE 'AC-B4-(5[89]|6[0-4])'` ≥ 7；`--rule=zero-transcript` 退 0；`--rule=doc-plan` 退 0
  - 证据：`bash scripts/check-b4-gates.sh --rule=zero-transcript && bash scripts/check-b4-gates.sh --rule=doc-plan; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-48.2 U-04 关闭登记（保留 AC-64 的可 grep 判据）** · `docs/progress/b4-status.md`
  - 追溯：AC-B4-64（"`b4-status.md` 的 U-04 口径 shall 为'规格已定，待实现'"）· requirements §11.7-1 · design §5.8.3
  - 改动内容：**保留** §10.1 的 U-04 行原文字"规格已定，待实现"，并在行末追加"，已于 P5 关闭（见 §13）"；§11 状态表的 U-04 行改为 `closed（W-B4-43~48）`，并保留一句"历史口径见 §10.1"；§12.3 的 AC-58~64 七行各在原状态后追加 `→ passed（§13）`（**不删除**原文 `pending`）。**U-03 / U-05 行不得改动**
  - 禁止：不得删除 §10.1 中"规格已定"字样（AC-64 的 `grep -c 'U-04.*规格已定'` ≥ 1 依赖它）；不得把 U-03/U-05 改成 closed
  - DoD：`grep -c 'U-04.*规格已定'` ≥ 1；`grep -c 'U-04.*closed'` ≥ 1；`grep -cE 'U-0[35].*open'` ≥ 2；`grep -c '^| AC-B4-' docs/progress/b4-status.md` ≥ 64
  - 证据：四条 grep 全部满足
  - 说明：这是"AC 判据与状态更新"的唯一折中——AC-64 冻结了 U-04 的字面口径，实现完成后只能**追加**关闭登记，不能**改写**历史行，否则 AC-64 自相矛盾。

- [ ] **W-B4-48.3 零 diff 与封闭面复核矩阵（design §5.9.4）** · 仓库根
  - 追溯：AC-B4-58、AC-B4-60、AC-B4-63 · design §5.1.4、§5.9.4
  - DoD（**每条都必须退 0**）：
    - `git diff --quiet HEAD -- crates/application/src/gate/api.rs` → 0，且 `grep -c '^pub fn '` = 4
    - `git diff --quiet HEAD -- crates/application/src/gate` → 0
    - `git diff --quiet HEAD -- crates/domain/src/model/gate_keyword.rs crates/domain/src/model/gate.rs crates/domain/src/model/gate_replay.rs crates/domain/src/model/enums.rs` → 0
    - `git diff --stat -- Cargo.toml Cargo.lock` → 0 行
    - `git diff --quiet HEAD -- crates/application/src/understand/store.rs crates/application/src/understand/deduplicator.rs` → 0
    - `bash scripts/check-b4-gates.sh --rule=record` → 0（14 / 7 / 23 / 5 / 32 / 23 不变）
    - `test ! -d crates/application/tests/replay` → 0
  - 证据：一条串联命令（见下方），任一失败即 `exit=1`
  - 证据命令：
    ```bash
    set -e
    [ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ]
    git diff --quiet HEAD -- crates/application/src/gate/api.rs
    git diff --quiet HEAD -- crates/application/src/gate
    git diff --quiet HEAD -- crates/domain/src/model/gate_keyword.rs crates/domain/src/model/gate.rs crates/domain/src/model/gate_replay.rs crates/domain/src/model/enums.rs
    [ "$(git diff --stat -- Cargo.toml Cargo.lock | wc -l | tr -d ' ')" = "0" ]
    git diff --quiet HEAD -- crates/application/src/understand/store.rs crates/application/src/understand/deduplicator.rs
    bash scripts/check-b4-gates.sh --rule=record
    test ! -d crates/application/tests/replay
    echo "exit=$?"
    ```

- [ ] **W-B4-48.4 P5 出口收口（8 target / 7 规则 / 全量回归 / 五项既有门禁）** · `crates/**`、`scripts/**`
  - 追溯：AC-B4-58~64 · design §5.8.2/§5.8.3、§5.11
  - DoD：8 个 target 逐个退 0（`gate_prefilter`/`gate_relevance`/`gate_dedupe`/`gate_budget`/`gate_outbound`/`gate_replay`/`gate_keyword`/`citation_align`）；七条规则 7×0 且 `--self-test` 退 1；`cargo test --workspace` 退 0、`failed=0`、`ignored=9`；`--diff` `removed=0`；`fmt --check` / `clippy -D warnings` / `check-layering.sh` / `scan-secrets.sh` / `cargo deny` 五项全 0；`Cargo.toml`/`Cargo.lock` 零 diff
  - 证据：
    ```bash
    for t in gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay gate_keyword citation_align; do
      "$CARGO" test -p meeting-copilot-application --test "$t" || exit 1
    done
    for r in source zero-transcript budget outbound record targets doc-plan; do
      bash scripts/check-b4-gates.sh --rule="$r" || exit 1
    done
    "$CARGO" fmt --all -- --check || exit 1
    "$CARGO" clippy --workspace --all-targets -- -D warnings || exit 1
    bash scripts/check-layering.sh || exit 1
    bash scripts/scan-secrets.sh || exit 1
    "$CARGO" deny check || exit 1
    "$CARGO" test --workspace
    bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
    echo "exit=$?"
    ```
  - 证据产物：全部退出码与日期写入 `b4-status.md` §13.1；阶段结论写入 §13.2/§13.3

---

## P6 · 38 卡片合成（增量 AC-B4-65~74）

> 阶段：编码任务规划的**第四个增量批次**（P5 之后）。需求来源：`requirements.md` §12（REQ-B4S-01~04、AC-B4-65~74、§12.4 的 7/11/2/6 字段）、`spec.md` §5.8 / §6.16~6.18 / §7.13 / §9.1 J-21~J-26、`design.md` 第六章（§6.0~§6.13，含 §6.11.3 裁决 X-07~X-09）、`docs/progress/b4-status.md` §13（P5 快照）、§14（D-B4-P45~P52、AC-B4-65~74 全 `pending`）。
> 进度基线：P5 后 `cargo test --workspace` **721 passed / 0 failed / 9 ignored**；`citation_align` 12 passed；`NEW_TARGETS`=**21**；`api.rs` 4 函数零 diff；`tests/replay/` 不存在；`spec.md` 的 `^| AC-B4-` 行数=**74**。
> **范围**：只做 38 的 4 契约类型 + 2 纯函数 + 1 诊断 + 14 用例 + 2 处脚本扩展 + 1 处状态登记。**不建** 51 UI（`crates/ui-bridge/src/keyword_card/`）、**不建** 52 存储、**不建** `crates/application/tests/replay/`；**不新增** domain 契约、**不新增** crate/依赖、**不改** `gate/**`。
> **与 U-03 的关系**：**U-03 由本 P6 闭合**（W-B4-49~56）；AC-B4-65~74 由 `pending` 转 `passed`；§10.1 的"规格已定"原文**保留**，只追加关闭登记（AC-74 的 `grep -c 'U-03.*规格已定'` 依赖它）。
> **与 U-05 / U-06 / U-07 / U-08 的关系**：**均保持 open，不得顺手关闭**。U-05（51 点击 UI）—— AC-68 只断言 38 输出**不含** `gate_decision_ref`/`click_state`，`click_state` 的 7 项合取仍由既有 `keyword_card_admission` 承担（38 不调用），不建 UI 文件、不验交互质量与投屏表现；U-06（13.3 回放本体）—— `test ! -d crates/application/tests/replay` 必须退 0；U-07/U-08 不承接，9 个 `#[ignore]` 名称与数量不变。
> **P6 出口判据（缺一即 P6 未完成）**：① `card_compose` target **14 passed / 0 failed**；② `kw_fields CardComposeInput`=**7**、`ComposedCard`=**11**、`CardComposeOutcome`=**2**、`ComposeRejectReason` 变体=**6**；③ `NEW_TARGETS`=**22**、`--rule=targets` 退 0 且 `grep -c 'card_compose' check-b4-gates.sh`=**0**；④ 七条规则 7×0，且 `--rule=source`/`zero-transcript` 扫描面含 `cards/**` 与 `tests/card_compose.rs`（扫描面缺失退 2）；⑤ `api.rs` 4 函数零 diff、`gate/**` 零 diff、`domain/src/model/**` 零 diff、14/7/23/5/32/23 不变、`tests/replay/` 不存在；⑥ 全量 `cargo test --workspace` `failed=0` / `ignored=9`，`--diff` `removed=0`；⑦ `spec.md` AC 汇总=**74**、AC-65~74 全 `passed`、U-03 保留"规格已定"并追加 `closed（W-B4-49~56）`。

**P6 需求 → AC → 用例 → 任务映射（REQ-B4S-01~04 全覆盖）**

| REQ | AC | 用例名（逐字） | 任务 |
|---|---|---|---|
| REQ-B4S-01 确定性组装 | AC-B4-65 | `card_compose_input_is_readonly_and_closed` | W-B4-49、50.1、53.1 |
| REQ-B4S-01 | AC-B4-66 | `card_compose_title_and_summary_limits` | W-B4-50.5、51.2、53.2 |
| REQ-B4S-01 | AC-B4-66 | `card_compose_preserves_confidence_and_trigger` | W-B4-51.3、51.4、53.2 |
| REQ-B4S-01 | AC-B4-67 | `card_compose_requires_gate_allow` | W-B4-50.2、53.3 |
| REQ-B4S-01 | AC-B4-67 | `card_compose_is_idempotent_per_candidate` | W-B4-51.3、53.4 |
| REQ-B4S-01/03 | AC-B4-67 | `card_compose_withdrawn_is_not_revived` | W-B4-50.4、50.5、53.4 |
| REQ-B4S-02 | AC-B4-68 | `card_compose_aligns_unknown_to_unverified` | W-B4-51.1、54.1 |
| REQ-B4S-02 | AC-B4-68 | `card_compose_leaves_click_state_to_b4` | W-B4-49.2、51.3、54.2 |
| REQ-B4S-02 | AC-B4-69 | `card_compose_failed_stays_displayed_without_conclusion` | W-B4-51.2、54.3 |
| REQ-B4S-03 | AC-B4-70 | `card_compose_references_are_id_only_and_sorted` | W-B4-50.1、51.1、54.4 |
| REQ-B4S-04 | AC-B4-71 | `card_compose_is_offline_and_deterministic` | W-B4-51.3、55.1 |
| REQ-B4S-04 | AC-B4-71 | `card_compose_ignores_reference_order` | W-B4-51.1、55.2 |
| REQ-B4S-04 | AC-B4-72 | `card_compose_zero_transcript_boundary` | W-B4-49.2、55.3、56.2 |
| REQ-B4S-04 | AC-B4-73 | `card_compose_ownership_is_outside_gate_and_ui` | W-B4-49.4、55.4 |
| REQ-B4S-04 | AC-B4-74 | （target 实跑，无独立用例） | W-B4-56.1~56.5 |

### P6 公共断言（每个 shell 会话执行一次，后续任务直接引用）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
CARDS_APP_DIR="crates/application/src/cards"
CARDS="crates/application/src/cards/compose.rs"
CARDS_MOD="crates/application/src/cards/mod.rs"
CARDS_TEST="crates/application/tests/card_compose.rs"

# 与 P4/P5 同口径（沿用 P4 公共断言的 kw_fields / kw_variants 定义，勿重复定义）
kw_fields() { awk -v s="pub struct $1 {" 'index($0,s)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    pub [a-z_]+:'; }
kw_variants() { awk -v e="pub enum $1 {" 'index($0,e)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    [A-Z][A-Za-z0-9]*(\(|,)'; }
new_targets_count() { grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//" | wc -w; }
# 38 顶层 pub 类型：目标 6 = 4 契约 + CardStatusSignal + CardComposeDiagnostics（D-B4-P64）
cards_items() { grep -cE '^pub (struct|enum) ' "$CARDS"; }
# 38 顶层函数：目标 2 = compose_keyword_card + next_card_status（impl 内方法为 4 空格缩进，不计）
cards_fns() { grep -c '^pub fn ' "$CARDS"; }
# 38 禁词/倒挂一次性扫描：目标 0
cards_hits() { grep -rnE 'presenter|card_store|outbox|on_demand|feedback_weights|reqwest|ClickState|KeywordCardDraft|gate_decision_ref|click_state|use crate::gate' "$CARDS_APP_DIR" 2>/dev/null | wc -l; }
# 38 用例数：目标 14（全部以 card_compose_ 开头）
cards_cases() { grep -c '^fn card_compose_' "$CARDS_TEST"; }
```

> **零转录纪律（P6 加严）**：`$CARDS_APP_DIR/**`、`$CARDS_TEST`、`docs/progress/b4-status.md` §15 内**不得**出现协议前缀、`sk-` 形态 Key、`Authorization: Bearer`、模型名前缀、`request_id:` 形态、`LLM_` 前缀常量。测试内反例常量只允许 `"ref-1"`/`"card-1"`/`"t-1"`/`"术语1"` 这类不含被禁模式的短串。
> **`--rule=source` 加严（R-38-01）**：`CARDS_TEST` 进 `core_files()` 后被**整体**扫描（测试文件无 `#[cfg(test)]` 行，`strip_tests()` 不生效），故测试内**禁止** `panic!(` / `todo!(` / `unimplemented!(` / `SystemTime::now` / `Instant::now` / `std::env` / `std::fs` / `reqwest` / `LLM_`；只用 `assert!`/`assert_eq!`（其展开不含上述字面量），路径存在性判断用 `std::path::Path::new(...).exists()`（P5 的 AC-63 同口径）。
> **`compose.rs` 内单测位置**：允许 `#[cfg(test)] mod tests`，但**必须位于文件末尾**且紧跟 `^#[cfg(test)]` 行（`strip_tests()` 依赖该锚点从该行剥离到 EOF），否则单测会被 `--rule=source` 当作生产代码扫描。
> **X-03 提交纪律（硬约束）**：`check-b4-gates.sh` 的 `require_core_faces()` 一旦加入 `CARDS_APP_DIR`，38 目录缺失即令**全部** `--rule` 退 **2**。因此 **W-B4-56.1/56.2 必须在 W-B4-49~55 落盘之后执行，并与 49~56 同批提交**；禁止先改脚本再建文件，禁止单独提交脚本改动。
> **X-05 口径（已冻结，不再权衡）**：`--rule=targets` 的循环列表**保持现状 7 项**（`gate_prefilter … gate_keyword`），**不追加** `card_compose`；`card_compose` 由 `NEW_TARGETS` token（21→22）与 target 实跑断言。`grep -c 'card_compose' scripts/check-b4-gates.sh` 必须为 **0**。

### P6 待裁决 Y-01（`trigger` 来源，**必须在 W-B4-49.1 之前定稿**）

| 事实 | 证据 |
|---|---|
| `KeywordCandidate`（10 字段）**不含** `trigger` | `crates/domain/src/model/gate_keyword.rs:169-181`；`kw_fields KeywordCandidate`=10 |
| `CardComposeInput`（7 项）**不含** `trigger` | spec §6.16-1~7、design §6.3.1 |
| spec/design 却要求 `ComposedCard.trigger` "与候选恒等" | spec §6.17-5、design §6.3.1 表第 5 行、I-38-05 |
| `KeywordCardDraft`（13 字段）**含** `trigger`，目前无生产来源 | `gate_keyword.rs:202-216`；`gate/**` 无构造点 |

| 方案 | 做法 | 代价 | 判定 |
|---|---|---|---|
| **C（推荐）** | 38 只合成 `Auto` 候选（会中监听）；`ComposedCard.trigger = TriggerKind::Auto` 常量；OnDemand 候选**不进 38**（由装配方走既有 13 字段草稿路径）。AC-66 的"恒等"收窄为"`trigger==Auto` + 其余 5 项逐项恒等" | 需在 spec §6.17-5 与 AC-66 判据加一句口径声明（`tasks.md` 与 `b4-status.md` §15.4 登记，不改 AC 编号/不变体数） | 7/11/2/6 全不变；**最小侵入** |
| A | `CardComposeInput` 增第 8 项 `trigger: TriggerKind` | 破坏 AC-65 的"7 项"字面与 §6.16 封闭，需同步改 spec/requirements/AC-65 判据 | 不推荐 |
| B | `ComposedCard` 删 `trigger`，改由装配方补 3 项 | 破坏 11 项与"11+2=13"，AC-66/68 需改判据 | 不推荐 |

> **执行约定**：W-B4-49.0 只做"确认方案 + 在 `b4-status.md` §15.4 登记裁决"。**用户确认前不得进入 W-B4-49.1**（否则 11 项组装的 `trigger` 写法会返工）。若用户选 A/B，按上表"代价"列同步修正 W-B4-49.1、51.3、51.4、53.2 的 DoD（已在各任务中标注 `[Y-01-C]` / `[Y-01-A]` 分叉）。

### P6 依赖图（单向，不可倒置）

```text
W-B4-49 ─► W-B4-50 ─► W-B4-51 ─► W-B4-52 ─► W-B4-53 ─► W-B4-54 ─► W-B4-55 ─► W-B4-56
内部：49.0 ─► 49.1 ─► {49.2 ∥ 49.3} ─► 49.4
      50.1 ─► 50.2 ─► 50.3 ─► 50.4 ─► 50.5
      51.1 ─► 51.2 ─► 51.3 ─► 51.4
      52.1 ─► 52.2
      53.1 ─► 53.2 ─► 53.3 ─► 53.4
      54.1 ─► 54.2 ─► 54.3 ─► 54.4
      55.1 ─► 55.2 ─► 55.3 ─► 55.4
      56.1 ─► 56.2 ─► {56.3 ∥ 56.4} ─► 56.5
```

- **不可压缩三点**：49.0 的 Y-01 裁决 → 49.1 骨架 → `lib.rs` +1（否则 `crate::cards` 不可见）；50.5 的 `next_card_status` 真值表必须先于 53.4（AC-67 撤回用例）；51.1 的 `project_alignment` 调用必须先于 54.1（AC-68 唯一投影）。
- **可并行**：49.2（Debug 白名单）与 49.3（诊断计数）互不依赖；53/54/55 三组用例断言互不依赖；56.3（inventory）与 56.4（targets 口径复核）互不依赖。
- **唯一串行收口**：56.1/56.2 必须与 49~55 **同批提交**（X-03）；56.5 必须在 49~56 全绿后一次性登记。

### W-B4-49 application 契约落位与最小可编译骨架（AC-B4-65/68）

> 前置：W-B4-49.0（Y-01）。落点 `$CARDS_APP_DIR`（**新目录**，2 文件）+ `lib.rs` +1 行；`gate/**`、`domain/**` 零 diff。

- [ ] **W-B4-49.0 Y-01 裁决确认与登记（进入编码的唯一闸门）** · `docs/progress/b4-status.md` §15.4（草稿）
  - 追溯：AC-B4-66 · spec §6.17-5 · design §6.3.1 表第 5 行、§6.12 冲突清单 · **Y-01（待用户裁决）**
  - 改动内容：与用户确认 `trigger` 方案（推荐 C：38 只合成 `Auto`，`ComposedCard.trigger = TriggerKind::Auto`）；把"事实/方案/代价"三行写入 `b4-status.md` §15.4 草稿（**本步不改任何代码与脚本**）
  - 禁止：不得在未确认时进入 49.1；不得自行把 `trigger` 加进 7 项输入（方案 A）或从 11 项输出删除（方案 B）；不得改 `gate_keyword.rs`
  - DoD：用户明确回复"采用 C / 采用 A / 采用 B"；`grep -c 'Y-01' .codeartsdoer/specs/b4_gate/tasks.md` ≥ 3（本节 + 裁决表 + 51.4）
  - 证据：以用户回复为准；`grep -n 'Y-01' .codeartsdoer/specs/b4_gate/tasks.md`
  - 证据产物：裁决结论写入 `b4-status.md` §15.4（56.5 定稿）

- [ ] **W-B4-49.1 `cards/mod.rs` + `cards/compose.rs` 骨架 + `lib.rs` +1 行** · `$CARDS_MOD`、`$CARDS`、`crates/application/src/lib.rs`
  - 追溯：AC-B4-65、AC-B4-68 · spec §6.16/§6.17/§6.18 · design §6.2.2、§6.3.1、§6.4.2、D-B4-P53/P61/**D-B4-P64**
  - 改动内容：`compose.rs` 顶端 `//! 38 卡片合成：只读 7 项、确定性 11 项、不判定点击。`；**派生口径**：`ComposeRejectReason` `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`（逐行 6 变体）、`CardComposeInput` `#[derive(Clone)]` + **手写 `Debug`**（不 `Serialize`/`Deserialize`）、`ComposedCard` `#[derive(Debug, Clone, PartialEq, Serialize)]`（**不** `Deserialize`）、`CardComposeOutcome` `#[derive(Debug, Clone, PartialEq)]`、`CardStatusSignal` `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`、`CardComposeDiagnostics` `#[derive(Debug, Clone, Default, PartialEq, Eq)]`；`ComposeRejectReason`（**6 变体逐行声明**，顺序 `GateNotAllowed`/`CandidateInvalid`/`TitleTooLong`/`DuplicateCard`/`CardWithdrawn`/`InputContractViolation`）+ `impl { pub const ALL: [Self; 6]; pub const fn index(self) -> usize }`；`CardComposeInput`（**7 公有字段、顺序固定**：`card_id: CardRef`/`candidate: KeywordCandidate`/`allow_ref: Option<AllowTraceRef>`/`alignment_result: CitationAlignmentResult`/`references: Vec<ReferenceView>`/`current_card_status: KeywordCardStatus`/`suppressed_notice: bool`）；`ComposedCard`（**11 公有字段、顺序固定**：`card_id`/`keyword_text`/`normalized_term`/`keyword_kind`/`trigger`/`triggered_at_ms`/`confidence`/`summary`/`references`/`citation_alignment`/`card_status`）；`CardComposeOutcome { pub card, pub reason }`；`pub enum CardStatusSignal { Present, Withdraw }`；`pub struct CardComposeDiagnostics { pub composes: u32, pub succeeded: u32, pub rejected: u32, pub reasons: [u32; 6] }`；`pub fn compose_keyword_card(&CardComposeInput) -> CardComposeOutcome` 与 `pub fn next_card_status(KeywordCardStatus, CardStatusSignal) -> KeywordCardStatus` 的**最小合法实现**（本步允许"全拒绝 + 恒 Displayed"，**不用** `todo!`/`unimplemented!`/`panic!`）；`mod.rs` **4 行**：`pub mod compose;` + `pub use compose::{compose_keyword_card, next_card_status, CardComposeDiagnostics, CardComposeInput, CardComposeOutcome, CardStatusSignal, ComposedCard, ComposeRejectReason};`；`lib.rs` 在 `pub mod capture;` 与 `pub mod citation;` 之间插入 `pub mod cards;`（**+1 / -0**，**不加** `pub use cards::*`）
  - 禁止：**不**在 `lib.rs` 加顶层 `pub use cards::*`；**不**在 `gate/mod.rs` 增行；`$CARDS_APP_DIR/**` 不得 `use crate::gate`；不得定义 trait/端口；不得实现 §4.5.2 的 `CardSynthesis`；文件名必须 `mod.rs`；`ComposedCard` 不得 `Deserialize`；`CardComposeInput` 不得 `Serialize`
  - DoD：`test -d "$CARDS_APP_DIR"` 退 0；`grep -c 'pub mod cards;' crates/application/src/lib.rs`=1；`grep -c 'pub mod compose;' "$CARDS_MOD"`=1；`grep -c 'pub use compose::' "$CARDS_MOD"`=1；`cards_items`=**6**（D-B4-P64）；`cards_fns`=**2**；`kw_fields CardComposeInput "$CARDS"`=**7**、`ComposedCard`=**11**、`CardComposeOutcome`=**2**、`CardComposeDiagnostics`=**4**；`kw_variants ComposeRejectReason "$CARDS"`=**6**、`CardStatusSignal`=**2**；`grep -c 'Deserialize' "$CARDS"`=0；`"$CARGO" build -p meeting-copilot-application` 退 0
  - 证据：`"$CARGO" build -p meeting-copilot-application; echo "exit=$?"` → `exit=0`；`cards_items; cards_fns`；三条 `kw_fields`；`git diff --quiet HEAD -- crates/application/src/gate; echo "exit=$?"` → `exit=0`
  - 证据产物：退出码与 6/2/7/11/2/6 计数写入 `b4-status.md` §15.1

- [ ] **W-B4-49.2 `CardComposeInput` 手写 `Debug` 白名单 + `ComposedCard.summary_is_consistent()`** · `$CARDS`
  - 追溯：AC-B4-68、AC-B4-72 · spec §5.8.1-9/10 · requirements §12.3.4-3 · design §6.4.2、§6.8.1、D-B4-P52
  - 改动内容：手写 `impl core::fmt::Debug for CardComposeInput`，白名单**9 项**：`card_id`、`keyword_ref`（取 `candidate.keyword_ref`）、`keyword_kind`、`reference_count`（`references.len()`）、`has_allow_ref`、`current_card_status`、`suppressed_notice`、`alignment`/`basis`（取 `alignment_result`）；`impl ComposedCard { pub fn summary_is_consistent(&self) -> bool }`（`None → true`；`Some → citation_alignment == CitationAlignment::Passed`）
  - 禁止：`CardComposeInput` **不得** `#[derive(Debug)]`、不得 `Serialize`/`Deserialize`；`Debug` 输出不得含 `normalized_term`、`keyword_text`、`summary` 取值、`trigger_span` 文本面、`basis` 之外的文本；不得在 `Debug` 里 `format!` 拼接展示词
  - DoD：`grep -c 'impl core::fmt::Debug for CardComposeInput' "$CARDS"`=1；`grep -B1 'pub struct CardComposeInput' "$CARDS" | grep -c 'Debug'`=0；`grep -c 'impl ComposedCard' "$CARDS"`≥1 且 `grep -c 'fn summary_is_consistent' "$CARDS"`=1；`format!("{:?}", input)` 不含 `normalized_term.as_str()`/`keyword_text.as_str()`/`summary` 文本（用例断言见 55.3）；`summary_is_consistent()` 4 组合（`None`/`Some+Passed`/`Some+Failed`/`Some+Unknown`）→ `true/true/false/false`
  - 证据：`grep -c 'impl core::fmt::Debug for CardComposeInput' "$CARDS"` → `1`；`grep -c 'fn summary_is_consistent' "$CARDS"` → `1`

- [ ] **W-B4-49.3 `ComposeRejectReason::ALL/index` + `CardComposeDiagnostics::record`（不进 GateCounter）** · `$CARDS`
  - 追溯：AC-B4-65、AC-B4-67 · spec §6.18-2/3/4 · design §6.3.1、D-B4-P60
  - 改动内容：`ComposeRejectReason::ALL` 长度恰 6、`index()` 输出 `0..=5` 互不重复；`CardComposeDiagnostics::record(&mut self, outcome: &CardComposeOutcome)`：`composes += 1`，`card.is_some()` → `succeeded += 1` 否则 `rejected += 1` 且 `reasons[reason.index()] += 1`（全部 `saturating_add`）
  - 禁止：**不新增** `GateCounter`、**不并入** `GateRejectReason`；`reasons` 长度必须 `[u32; 6]` 且下标只由 `index()` 给出；`record` 只增计数、不改 outcome、无 `panic!`
  - DoD：`kw_variants ComposeRejectReason`=6；`ComposeRejectReason::ALL.len()==6`；`index()` 6 值集合 == `{0,1,2,3,4,5}`；`record` 连调 3 次（1 成功 + 2 不同原因）→ `composes==3`、`succeeded==1`、`rejected==2`、`reasons` 求和==2；`grep -c 'GateCounter' "$CARDS"`=0；`grep -c 'GateRejectReason' "$CARDS"`=0
  - 证据：`grep -c 'GateCounter\|GateRejectReason' "$CARDS"` → `0`；`"$CARGO" test -p meeting-copilot-application --lib cards::` 退 0

- [ ] **W-B4-49.4 顶层类型/函数面复核 + P6 首轮零 diff** · 仓库根
  - 追溯：AC-B4-65、AC-B4-73 · design §6.1.4、§6.10.3、§6.10.4、D-B4-P53/P61/**D-B4-P64**
  - 改动内容：**零代码改动**（只复核）。登记"4 契约 + 2 辅助 = 6 顶层 pub 类型、2 顶层 `pub fn`"口径
  - DoD（每条都必须退 0）：
    - `cards_items`=6、`cards_fns`=2、`cards_hits`=0
    - `grep -c '^pub fn ' crates/application/src/gate/api.rs`=4，且 `git diff --quiet HEAD -- crates/application/src/gate` 退 0
    - `git diff --quiet HEAD -- crates/domain/src/model` 退 0
    - `git diff --stat -- Cargo.toml Cargo.lock` 行数=0
    - `grep -c 'pub mod cards;' crates/application/src/lib.rs`=1 且 `git diff --numstat HEAD -- crates/application/src/lib.rs` 为 `1 0`
  - 证据：
    ```bash
    set -e
    [ "$(cards_items)" = "6" ] && [ "$(cards_fns)" = "2" ] && [ "$(cards_hits)" = "0" ]
    [ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ]
    git diff --quiet HEAD -- crates/application/src/gate
    git diff --quiet HEAD -- crates/domain/src/model
    [ "$(git diff --stat -- Cargo.toml Cargo.lock | wc -l | tr -d ' ')" = "0" ]
    echo "exit=$?"
    ```

### W-B4-50 S0~S5 判定链与状态真值表（AC-B4-65/67/70）

> 前置：W-B4-49。**判定顺序不可重排**：S0 结构 → S1 Allow → S2 候选 → S3 已撤回 → S4 重复 → S5 标题（design §6.2.3/§6.6.2）。所有失败分支均"0 卡 + 唯一原因码"。

- [ ] **W-B4-50.1 S0 输入结构校验（引用唯一性 + 元素 5 字段）→ `InputContractViolation`** · `$CARDS`
  - 追溯：AC-B4-65、AC-B4-70 · spec §5.8.3-5 / §6.16-5 · design §6.2.3（S0）、§6.6.1（DG-5）
  - 改动内容：私有 `fn validate_references(references: &[ReferenceView]) -> bool`：`BTreeSet<ReferenceRef>` 长度必须等于 `references.len()`（判重复），**不排序、不去重、不读越界字段**；`compose_keyword_card` 第一步 `if !validate_references(&input.references) { return rejected(InputContractViolation) }`
  - 禁止：S0 内不得 `sort`/`sort_by`/`dedup`；不得读取 `ReferenceView` 之外的任何字段；空集合必须返回 `true`；不得 `unwrap`/`expect`/`panic!`
  - DoD：空集合 → `true`（出卡路径继续）；3 条唯一 ID → `true`；重复 ID（2 条同 `reference_id`）→ `false` 且 `compose` 返回 `reason=InputContractViolation`、`card=None`；`grep -c 'BTreeSet' "$CARDS"` ≥1；`grep -c 'dedup' "$CARDS"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-application --lib cards::compose::tests::compose_s0_rejects_duplicate_reference_id` 退 0

- [ ] **W-B4-50.2 S1 `allow_ref = None` → `GateNotAllowed`（唯一 Allow 前置）** · `$CARDS`
  - 追溯：AC-B4-67 · spec §5.8.1-1 / §5.8.3-1 · requirements §12.3.1-1 ... design §6.6.1（DG-1）、§6.7.1、D-B4-P54
  - 改动内容：S1 `if input.allow_ref.is_none() { return rejected(GateNotAllowed) }`；`Some(_)` 继续 S2
  - 禁止：不得新增"决策"字段；不得检查 `AllowTraceRef.stopped_at` 的内容（38 只判存在性）；不读 `GateDecision`
  - DoD：`None` → `GateNotAllowed`；`Some(AllowTraceRef::with_trace(TraceId::new("t-1")?, None))` + 合法候选 → `card=Some`；`None` + 候选非法 → `GateNotAllowed`（S1 优先于 S2）；`kw_fields CardComposeInput`=7（无第 8 字段）
  - 证据：`"$CARGO" test -p meeting-copilot-application --lib cards::compose::tests::compose_s1_requires_allow_trace` 退 0

- [ ] **W-B4-50.3 S2 候选合法性（复用 `KeywordCandidate::validate()`）→ `CandidateInvalid`** · `$CARDS`
  - 追溯：AC-B4-67 · spec §5.8.1-1 / §5.8.3-2 · design §6.6.1（DG-2）
  - 改动内容：S2 `if !input.candidate.validate() { return rejected(CandidateInvalid) }`
  - 禁止：**不得**自行复算阈值（`0.50` 只能来自 `validate()`）；**不得**修正候选（改写 `state`/`confidence`/`trigger_span`）；不得 `clamp`
  - DoD：置信度 0.49 → `CandidateInvalid`；`start_ms == end_ms` → `CandidateInvalid`；`state == Pushed` → `CandidateInvalid`；`keyword_text` 空白（类型层不可构造空白 `TitleText`，改判 `validate()` 的 `trim().is_empty()` 分支由类型层覆盖，登记为不可构造防御分支）→ 断言 `validate()` 对合法候选返回 `true` 即可；合法候选 → 继续 S3
  - 证据：`"$CARGO" test -p meeting-copilot-application --lib cards::compose::tests::compose_s2_rejects_invalid_candidate` 退 0

- [ ] **W-B4-50.4 S3 已撤回优先 / S4 重复（`suppressed_notice`）→ `CardWithdrawn` / `DuplicateCard`** · `$CARDS`
  - 追溯：AC-B4-67 · spec §5.8.1-2 / §5.8.3-6 · requirements §12.3.3-3 · design §6.2.3、§6.6.1（DG-6）、D-B4-P55/P56（**X-08 已接受**）
  - 改动内容：S3 `if input.current_card_status == KeywordCardStatus::Withdrawn { return rejected(CardWithdrawn) }`；S4 `if input.suppressed_notice { return rejected(DuplicateCard) }`
  - 禁止：S3 必须**早于** S4（已撤回优先于重复）；38 **不得**维护/读取卡片表；`suppressed_notice` 不得进入 11 项输出
  - DoD：`current=Withdrawn` + `suppressed_notice=true` → `CardWithdrawn`；`current=Displayed` + `suppressed_notice=true` → `DuplicateCard`；`current=Collapsed` + `suppressed_notice=false` → 继续 S5（不得输出 `Collapsed`）；`grep -c 'card_store\|card_table' "$CARDS"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-application --lib cards::compose::tests::compose_s3_withdrawn_precedes_duplicate` 退 0

- [ ] **W-B4-50.5 S5 标题防御分支 + `next_card_status` 6 行真值表** · `$CARDS`
  - 追溯：AC-B4-66、AC-B4-67 · spec §5.8.1-3 / §6.17-11 · design §6.3.2、§6.6.1（DG-3/DG-4）、D-B4-P51/**P57（X-09 已接受）**
  - 改动内容：私有 `fn title_fits_limit(candidate: &KeywordCandidate) -> bool { candidate.keyword_text.char_count() <= TitleText::LIMIT }`；S5 `if !title_fits_limit(...) { return rejected(TitleTooLong) }`；`next_card_status` 按真值表：`(Displayed, Present)→Displayed`、`(Displayed, Withdraw)→Withdrawn`、`(Collapsed, Present)→Displayed`、`(Collapsed, Withdraw)→Withdrawn`、`(Withdrawn, Present)→Withdrawn`、`(Withdrawn, Withdraw)→Withdrawn`
  - 禁止：**不得**截断标题（无 `truncated`/`chars().take(40)`）；**不得**输出 `Collapsed`；`Withdrawn` 不得复活；S5 必须早于 S6~S8
  - DoD：`TitleText::new("字".repeat(41))` 返回 `Err`；`TitleText::new("字".repeat(40))` 返回 `Ok`；`TitleText::LIMIT == 40`；`title_fits_limit` 对 40 字候选 `true`（41 字候选类型层不可构造，防御分支由直调单测覆盖）；`next_card_status` 6 组合逐行等于真值表；`grep -c 'truncated' "$CARDS"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-application --lib cards::compose::tests::next_card_status_truth_table cards::compose::tests::title_limit_is_defensive` 退 0

### W-B4-51 S6~S11 组装、唯一投影与解释降级（AC-B4-66/68/69/70/72）

> 前置：W-B4-50。**唯一投影入口** `crate::citation::align::project_alignment`（不 `use crate::gate`）；**排序在判定前**（S7 早于 S8）。

- [ ] **W-B4-51.1 S6 唯一投影 + S7 稳定排序（`sorted_references`，不 dedup）** · `$CARDS`
  - 追溯：AC-B4-68、AC-B4-70 · spec §5.8.1-5/7 · design §6.2.3（S6/S7）、§6.1.2、D-B4-P62
  - 改动内容：私有 `fn sorted_references(references: &[ReferenceView]) -> Vec<ReferenceView>`（`sort_by_key(|view| view.reference_id.clone())`，稳定，**不** `dedup`）；S6 `let citation_alignment = crate::citation::align::project_alignment(&input.alignment_result);`；S7 `let references = sorted_references(&input.references);`
  - 禁止：**不得**复用 39 的私有 `sorted_references`（跨模块调用私有项）；不得 `use crate::gate`；`project_alignment` 调用必须恰 1 处（注释中不得重复书写该函数名，否则 grep 计数 >1）；不得把 `basis` 写入 `ComposedCard`（只允许在 `CardComposeInput` 手写 `Debug` 中读取）
  - DoD：`grep -rn 'pub fn project_alignment' crates/application/src | wc -l`=1；`grep -c 'project_alignment' "$CARDS"`=1；`grep -c 'use crate::gate' "$CARDS"`=0；`awk '/^pub struct ComposedCard \{/{f=1;next} f&&/^\}/{exit} f' "$CARDS" | grep -c 'basis'`=**0**；3 条乱序输入 → `references` 逐项按 `reference_id` 升序
  - 证据：`grep -rn 'pub fn project_alignment' crates/application/src | wc -l` → `1`；`"$CARGO" test -p meeting-copilot-application --lib cards::compose::tests::references_are_sorted_by_id` 退 0

- [ ] **W-B4-51.2 S8 `compose_summary`（固定模板 + `normalized_term`；>80 或非 Passed → `None`）** · `$CARDS`
  - 追溯：AC-B4-66、AC-B4-69 · spec §5.8.1-4 / §6.17-8 · requirements §12.3.2-2 · design §6.5（解释模板）、§6.6.1（DG-4）、D-B4-P48/**P58**
  - 改动内容：`pub const SUMMARY_TEMPLATE_PREFIX: &str = "该术语可参考可靠来源：";`；私有 `fn compose_summary(alignment: CitationAlignment, term: &NormalizedTerm, references: &[ReferenceView]) -> Option<ConclusionText>`：仅当 `alignment == CitationAlignment::Passed && !references.is_empty()` 时用 `format!("{SUMMARY_TEMPLATE_PREFIX}{}", term.as_str())` 拼接，`chars().count() <= ConclusionText::LIMIT` 才 `ConclusionText::new(...).ok()`，否则 `None`
  - 禁止：模板**不得**含 URL/标题/摘要/正文；**不得**调用 LLM/语义生成；**不得**复制引用内容；**不得**截断（无 `take(80)`）；`Failed`/`Unknown` 一律 `None`（不输出 `NO_RELIABLE_SOURCE`）
  - DoD：`Passed` + 3 条引用 + 短术语 → `summary=Some`；`Failed`/`Unknown` → `None`；`Passed` + 空引用 → `None`；`Passed` + 长术语（拼接 >80）→ `None`；`ConclusionText::LIMIT == 80`；`grep -c 'llm\|LLM_\|reqwest' "$CARDS"`=0；`grep -c 'NO_RELIABLE_SOURCE' "$CARDS"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-application --lib cards::compose::tests::summary_requires_passed_and_refs` 退 0

- [ ] **W-B4-51.3 S9~S11 组装 11 项（6 项透传 + `triggered_at_ms=end_ms` + XOR）** · `$CARDS`
  - 追溯：AC-B4-66、AC-B4-67、AC-B4-68、AC-B4-72 · spec §5.8.1-6 / §6.17-1~11 · design §6.2.3（S9~S11）、§6.3.1、**D-B4-P63**、Y-01
  - 改动内容：私有 `fn assemble_card(...) -> ComposedCard`；S9 `card_status = next_card_status(input.current_card_status, CardStatusSignal::Present)`（**38 不写 `Collapsed`**）；S10 逐项组装：`card_id=candidate 注入值`、`keyword_text/keyword_kind/normalized_term/confidence` 与候选恒等、`triggered_at_ms = candidate.trigger_span.end_ms`、`summary` 取 S8、`references` 取 S7、`citation_alignment` 取 S6、`trigger` 按 **Y-01-C 取 `TriggerKind::Auto`**（`[Y-01-A]` 若用户选 A 则取 `input.trigger`）；S11 `CardComposeOutcome { card: Some(card), reason: None }`
  - 禁止：不得重算 `confidence`（无 `Confidence::new`）、不得改 `keyword_kind`、不得读系统时间（`triggered_at_ms` 只取 `end_ms`）；`card_status` 只 `Displayed`/`Withdrawn`；`summary.is_some() ⇒ citation_alignment == Passed`
  - DoD：11 字段齐备且 `kw_fields ComposedCard`=11；`confidence.value()` 与候选逐位相等（`0.87`）；`trigger == TriggerKind::Auto`（`[Y-01-A]` 改为 `== input.trigger`）；`triggered_at_ms == candidate.trigger_span.end_ms`；`card_id/keyword_kind/normalized_term` 恒等；`CardComposeOutcome` XOR（`card.is_some() != reason.is_some()`）；`grep -c 'Confidence::new' "$CARDS"`=0；`grep -c 'SystemTime\|Instant::now' "$CARDS"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-application --lib cards::compose::tests::assemble_card_passes_through_six_fields` 退 0

- [ ] **W-B4-51.4 `trigger` 口径落地与全流程复核（Y-01 收口）** · `$CARDS`
  - 追溯：AC-B4-66 · spec §6.17-5 · **Y-01**、D-B4-P61
  - 改动内容：按 49.0 的裁决落 `trigger`（推荐 C：常量 `Auto`，并在 `compose.rs` 注释写明"38 只合成 `Auto` 候选；OnDemand 由装配方走既有草稿路径"）；复核 `compose_keyword_card` 无 `Result`/`Option` 返回、无 `unwrap`/`expect`/`panic!`（非测试区）
  - 禁止：不得新增第 8 输入或删第 11 输出；不得在 `cards/**` 出现 `OnDemand` 取值分支（`grep -c 'OnDemand' "$CARDS_APP_DIR"/*.rs` 允许 0；若注释提及不计）
  - DoD：`grep -c 'TriggerKind::Auto' "$CARDS"` ≥1；`cards_fns`=2；`grep -c 'unwrap()\|expect(' "$CARDS"`=0（非测试区，`strip_tests` 后）；`cards_hits`=0
  - 证据：`grep -c 'TriggerKind::Auto' "$CARDS"`；`awk '/^#\[cfg\(test\)\]/{exit} {print}' "$CARDS" | grep -c 'unwrap()\|expect('` → `0`

### W-B4-52 `card_compose` target 骨架与本地构造器（AC-B4-74 前置）

> 前置：W-B4-51。新增 `$CARDS_TEST`（**第 9 个 target**）；**不** `mod common`（38 用例只需 domain 类型与 `cards` 模块，避免 `common` 的 `Harness` 引入 `gate` 依赖到 38 测试面）。

- [ ] **W-B4-52.1 文件头 + 4 个本地构造器** · `$CARDS_TEST`
  - 追溯：AC-B4-65、AC-B4-74 · design §6.1.3、§6.9.1、§6.10.3
  - 改动内容：文件头 `//! 38 卡片合成（AC-B4-65~74，14 用例）。`；`use meeting_copilot_application::cards::{...};` 与 `use meeting_copilot_domain::model::{...};`、`use meeting_copilot_domain::vo::{...};`；构造器：`fn candidate(term: &str, confidence: f64, end_ms: u64) -> KeywordCandidate`（`keyword_ref=KeywordRef::new("kw-1")`、`keyword_text=TitleText::new(term)`、`normalized_term=NormalizedTerm::new(term)`、`dedupe_key=DedupeKey::new("0".repeat(32))`、`state=GapState::Detected`、`trigger_span=TextRange::new(0, end_ms)`、`session_id=SessionId::new("s-1")`、`source_insight_ref=None`）、`fn reference(id: &str, tier, status, alignment) -> ReferenceView`、`fn result(verdict: AlignmentVerdict, basis: AlignmentBasis, refs: u32, now_ms: u64) -> CitationAlignmentResult`（`reference_count=refs`、`accessible_reference_count=refs`、`checked_at_ms=now_ms`、`contract_version=CITATION_CONTRACT_VERSION`）、`fn input_for(candidate: KeywordCandidate, refs: Vec<ReferenceView>, verdict: AlignmentVerdict, status: KeywordCardStatus, suppressed: bool) -> CardComposeInput`
  - 禁止：不得 `#[ignore]`；不得 `std::fs` / `File::` / `println!` / `dbg!` / `panic!(`；不得出现协议前缀；`assert_eq!` 不得直接打印 `normalized_term` 文本（用 ID/枚举/计数比较）
  - DoD：`"$CARGO" test -p meeting-copilot-application --test card_compose --no-run` 退 0；`grep -c '^fn ' "$CARDS_TEST"` ≥4；`grep -c '#\[ignore\]' "$CARDS_TEST"`=0；`grep -c 'std::fs\|panic!(' "$CARDS_TEST"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose --no-run; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-52.2 构造器自检用例 `sample_input_is_closed_and_valid`（不计入 14）** · `$CARDS_TEST`
  - 追溯：AC-B4-65 · design §6.3.3（I-38-01/02/03）、§6.9.1
  - 改动内容：1 个非 AC 自检用例：`assert!(input.candidate.validate())`；`assert!(!input.allow_ref.is_none())`；`assert_eq!(input.references.len(), 3)`；`assert_eq!(input.suppressed_notice, false)`；`assert_eq!(input.current_card_status, KeywordCardStatus::Displayed)`
  - 禁止：不得把该用例命名成 `card_compose_*` 前缀（否则 `cards_cases` 计数虚高）；不得依赖 `common`
  - DoD：`grep -c '^fn card_compose_' "$CARDS_TEST"`=**0**（此时尚未写 AC 用例）；`"$CARGO" test -p meeting-copilot-application --test card_compose sample_input_is_closed_and_valid` 退 0（1 passed）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose; echo "exit=$?"` → `1 passed`

### W-B4-53 AC-65 / AC-66 / AC-67 用例（5 个）

> 前置：W-B4-52。用例名与 design §6.9.1 **逐字一致**；每个用例至少 1 条真断言，禁止空断言。

- [ ] **W-B4-53.1 `card_compose_input_is_readonly_and_closed`（AC-65）** · `$CARDS_TEST`
  - 追溯：AC-B4-65 · spec §7.13 AC-65 · requirements §12.5 AC-65 · design §6.2.3、§6.3.3（I-38-01/14/15/16）
  - 改动内容：构造 7 项输入并调用；`let snapshot = input.candidate.clone();` 调用后 `assert_eq!(input.candidate, snapshot)`（只读）；`assert_eq!(GateCounters::default().snapshot().len(), 32)`；`assert!(!std::path::Path::new("crates/application/tests/replay").exists())`；`assert!(!std::path::Path::new("crates/ui-bridge/src/keyword_card").exists())`；成功后 `assert!(outcome.card.is_some())`
     - 【**P7 加注**】末条 `crates/ui-bridge/src/keyword_card` 存在性断言在 P7 落盘后失效，**由 W-B4-57.1（X-12）统一改写为 `include_str!` 源码断言**；本行原文保留为 P6 时点历史。
  - 禁止：不得用 `serde_json`（D-B4-P63）；不得断言 `GateRecord` 字段（由外部 `--rule=record` 承担）；不得 `panic!`
  - DoD：本用例退 0；外部 `kw_fields CardComposeInput`=7；`grep -c '^pub fn ' gate/api.rs`=4；`--rule=record` 退 0；`git diff --quiet HEAD -- crates/domain/src/model` 退 0
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_input_is_readonly_and_closed` 退 0；`bash scripts/check-b4-gates.sh --rule=record; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-53.2 AC-66 两用例：长度上限与 5 项透传（`[Y-01-C]`）** · `$CARDS_TEST`
  - 追溯：AC-B4-66 · spec §5.8.1-3/6 / §6.17-2/6/8 · design §6.9.1、§6.3.3（I-38-05/06/07）、D-B4-P57/P58/**P63**、Y-01
  - DoD（`card_compose_title_and_summary_limits`）：`TitleText::LIMIT==40`、`ConclusionText::LIMIT==80`；`TitleText::new("字".repeat(41)).is_err()`；`TitleText::new("字".repeat(40)).is_ok()`；`Passed` + 长术语（拼接 >80）→ `card.summary.is_none()`；`Failed` → `summary.is_none()`；`card.summary_is_consistent()` 恒 `true`
  - DoD（`card_compose_preserves_confidence_and_trigger`）：`confidence.value()==0.87` 逐位相等；`trigger==TriggerKind::Auto`（**`[Y-01-A]` 若选 A → `== input.trigger`**）；`triggered_at_ms == candidate.trigger_span.end_ms`；`card_id==input.card_id`；`keyword_kind`/`normalized_term` 恒等
  - 禁止：不得截断断言（不比较 `truncated`）；不得用浮点容差（`Confidence` 是 `f64` 但从同一值透传，直接 `==`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_title_and_summary_limits card_compose_preserves_confidence_and_trigger` 退 0

- [ ] **W-B4-53.3 `card_compose_requires_gate_allow`（AC-67 唯一前置）** · `$CARDS_TEST`
  - 追溯：AC-B4-67 · spec §5.8.1-1 / §5.8.3-1 · design §6.6.2（比较对 1/2）、§6.7.1、D-B4-P54
  - DoD：`allow_ref=None` + 合法候选 → `reason==Some(GateNotAllowed)` 且 `card.is_none()`；`None` + 非法候选（0.49）→ 仍 `GateNotAllowed`（S1 优先）；`Some` + 合法候选 → `card.is_some()` 且 11 字段齐备（逐项访问）；`Some` + 非法候选 → `CandidateInvalid`；`None` + 引用重复 → `InputContractViolation`（S0 优先）
  - 禁止：不得构造"来自 Deny 的 allow_ref"（类型层不可构造，`AllowTraceRef::new(&Deny)` 返回 `None`，可断言 `is_none()`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_requires_gate_allow` 退 0

- [ ] **W-B4-53.4 AC-67 两用例：幂等 100 次 + 已撤回不复活（含 S3 优先于 S4）** · `$CARDS_TEST`
  - 追溯：AC-B4-67 · spec §5.8.1-2/8 / §5.8.3-6 · design §6.3.2 真值表、§6.6.2（比较对 4/5）、D-B4-P51/P55/P56
  - DoD（`card_compose_is_idempotent_per_candidate`）：同一输入 100 次 → `format!("{:?}", card)` 100 次相等（**D-B4-P63：无 `serde_json`**）且 11 字段逐项 `assert_eq!`；`card_drafts` 不增长（38 无计数，断言 `outcome.card.is_some()` 恒真且 `reason.is_none()`）
  - DoD（`card_compose_withdrawn_is_not_revived`）：`current=Withdrawn` → `reason==Some(CardWithdrawn)`；`next_card_status(Withdrawn, Present)==Withdrawn`；`next_card_status(Withdrawn, Withdraw)==Withdrawn`；`current=Withdrawn` + `suppressed_notice=true` → `CardWithdrawn`（S3 优先于 S4）；`current=Collapsed` → 出卡且 `card_status==Displayed`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_is_idempotent_per_candidate card_compose_withdrawn_is_not_revived` 退 0

### W-B4-54 AC-68 / AC-69 / AC-70 用例（4 个）

> 前置：W-B4-53。

- [ ] **W-B4-54.1 `card_compose_aligns_unknown_to_unverified`（AC-68 唯一投影）** · `$CARDS_TEST`
  - 追溯：AC-B4-68 · spec §5.8.1-5 / §6.17-10 · requirements §12.3.2-1 · design §6.6.2、D-B4-P38
  - DoD：`AlignmentVerdict::Unknown` → `card.citation_alignment == CitationAlignment::Unknown`（语义 `Unverified`，`CONTRACT_UNVERIFIED`）；`Passed`/`Failed` → 同名；三态单射（3 个不同输入 → 3 个互不相同的 `CitationAlignment`）；`CitationAlignment::ALL.len()==3`；`grep -c 'citation_alignment:' "$CARDS"`=1（只在 `ComposedCard` 结构体声明）；`grep -c '= CitationAlignment::' "$CARDS"`=**0**（组装处无任何手写三态赋值）；`grep -c 'CitationAlignment::Passed' "$CARDS"`=1（**仅** `summary_is_consistent` 的谓词比较）
  - 禁止：不得 `Unknown→Passed`；不得读 `basis` 解释（断言 `grep -c 'basis'` 在 `ComposedCard` 组装函数内为 0）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_aligns_unknown_to_unverified` 退 0

- [ ] **W-B4-54.2 `card_compose_leaves_click_state_to_b4`（AC-68 不写点击）** · `$CARDS_TEST`
  - 追溯：AC-B4-68 · spec §5.8.1-9 · requirements §12.3.2-4 · design §6.7.2、D-B4-P46
  - DoD：`let dbg = format!("{:?}", card);` → `assert!(!dbg.contains("gate_decision_ref"))`、`assert!(!dbg.contains("click_state"))`；11 字段名中不含两者（`ComposedCard` derive `Debug` 输出字段名，可比对）；外部 `cards_hits`=0；`grep -rn 'pub fn project_alignment' crates/application/src | wc -l`=1
  - 禁止：**不得**在 38 测试里构造 `KeywordCardDraft`（那是装配方职责）；不得断言 `ClickState` 取值（38 无该字段）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_leaves_click_state_to_b4` 退 0；`cards_hits` → `0`

- [ ] **W-B4-54.3 `card_compose_failed_stays_displayed_without_conclusion`（AC-69）** · `$CARDS_TEST`
  - 追溯：AC-B4-69 · spec §5.8.1-4 / §5.8.3-4 · requirements §12.3.2-3 · design §6.5、§6.6.1（DG-4）、D-B4-P58
  - DoD：`Failed` → `card.is_some()`、`card.summary.is_none()`、`card.card_status==Displayed`、`card.summary_is_consistent()==true`；`Unknown` → 同上且 `citation_alignment==CitationAlignment::Unknown`；`Failed` + 空引用 → `references.is_empty()` 且出卡；`card_status != Withdrawn`
  - 禁止：不得把 `NO_RELIABLE_SOURCE` 写入 `summary`（38 一律 `None`）；不得因对齐失败返回 `reason`
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_failed_stays_displayed_without_conclusion` 退 0

- [ ] **W-B4-54.4 `card_compose_references_are_id_only_and_sorted`（AC-70）** · `$CARDS_TEST`
  - 追溯：AC-B4-70 · spec §5.8.1-7 / §6.17-9 · requirements §12.3.3-1/2 · design §6.2.3（S0/S7）、D-B4-P62
  - DoD：3 种排列（原序、逆序、首尾交换）→ `card.references` 逐项相等（`Vec<ReferenceView>` 比较）且按 `reference_id` 升序；重复 ID → `reason==Some(InputContractViolation)`；空集合 → `card.is_some()` 且 `references.is_empty()`；`kw_fields ReferenceView crates/domain/src/model/gate_keyword.rs`=5；`card.references.iter().all(|r| r.reference_id.as_str().starts_with("ref-"))`
  - 禁止：不得 `dedup`（重复已在 S0 拒绝）；不得复制 URL/标题/摘要/正文（类型层只有 5 字段，断言 `format!("{:?}", r)` 不含 `http`/`title`/`excerpt`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_references_are_id_only_and_sorted` 退 0

### W-B4-55 AC-71 / AC-72 / AC-73 用例（4 个）

> 前置：W-B4-54。

- [ ] **W-B4-55.1 `card_compose_is_offline_and_deterministic`（AC-71）** · `$CARDS_TEST`
  - 追溯：AC-B4-71 · spec §5.8.1-11 · requirements §12.3.4-4 · design §6.8.2/§6.8.3、D-B4-P63
  - DoD：同输入 100 次 → 11 字段逐项相等 + `format!("{:?}", card)` 100 次相等；无 `SystemTime`/`Instant` 参与（`triggered_at_ms` 恒 `end_ms`）；8 类禁止符号由外部 `--rule=source` 断言（`cards/**` + `tests/card_compose.rs` 同权）
  - 禁止：测试内不得读系统时间/环境变量/文件；不得用 `HashSet` 收集（用 `BTreeSet`）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_is_offline_and_deterministic` 退 0；`bash scripts/check-b4-gates.sh --rule=source; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-55.2 `card_compose_ignores_reference_order`（AC-71 顺序无关）** · `$CARDS_TEST`
  - 追溯：AC-B4-71 · spec §5.8.1-7/11 · design §6.8.3-4、D-B4-P62
  - DoD：同一引用集合的 3 种排列 → `format!("{:?}", card)` 逐字节相等（`String` 比较）且 `references` 逐项相等；`citation_alignment`/`summary`/`card_status` 三态相等
  - 禁止：不得在排序前做判定（`summary` 的"引用非空"以排序后集合为准，但空/非空与顺序无关；用 3 条非空 + 1 条空两组断言）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_ignores_reference_order` 退 0

- [ ] **W-B4-55.3 `card_compose_zero_transcript_boundary`（AC-72）** · `$CARDS_TEST`
  - 追溯：AC-B4-72 · spec §5.8.1-10 · requirements §12.3.4-2/3 · design §6.8.1、D-B4-P52/P63
  - DoD：`format!("{:?}", input)` 不含 `input.candidate.normalized_term.as_str()`、不含 `input.candidate.keyword_text.as_str()`、不含 `input.alignment_result.basis` 之外的文本；`format!("{:?}", card)` 含 11 个字段名（`card_id`/`keyword_text`/`normalized_term`/`keyword_kind`/`trigger`/`triggered_at_ms`/`confidence`/`summary`/`references`/`citation_alignment`/`card_status`）；外部 `--rule=zero-transcript` 退 0；`kw_fields ComposedCard`=11
  - 禁止：`Debug` 断言的"参照串"不得写入 `b4-status.md`（只写"命中 0"）；测试内不得写完整转写/姓名/联系方式
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_zero_transcript_boundary` 退 0；`bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-55.4 `card_compose_ownership_is_outside_gate_and_ui`（AC-73）** · `$CARDS_TEST`
  - 追溯：AC-B4-73 · spec §5.8.1-12 · requirements §12.3.4-1 · design §6.1.4、§6.10.4、D-B4-P53/P60
  - DoD：`assert_eq!(GateCounters::default().snapshot().len(), 32)`；`assert!(!std::path::Path::new("crates/application/tests/replay").exists())`；`assert!(!std::path::Path::new("crates/ui-bridge/src/keyword_card").exists())`；测试成功后外部断言 `cards_hits`=0、`grep -rn 'use crate::gate' "$CARDS_APP_DIR" | wc -l`=0、`grep -c '^pub fn ' gate/api.rs`=4
     - 【**P7 加注**】`crates/ui-bridge/src/keyword_card` 存在性断言在 P7 落盘后失效，**由 W-B4-57.1（X-12）改为 `include_str!("../src/cards/compose.rs")` 不含 `keyword_card`**；AC-73 的归属判据仍由 `cards_hits`=0 + `use crate::gate`=0 承担，语义不削弱。
  - 禁止：不得新建 UI/存储/回放文件；不得在库内 import `gate::keyword`（38 源码 `grep 'gate::keyword'`=0）
  - 证据：`"$CARGO" test -p meeting-copilot-application --test card_compose card_compose_ownership_is_outside_gate_and_ui` 退 0；`cards_hits` → `0`

### W-B4-56 脚本扫描面、target 清单与 P6 出口收口（AC-B4-71/72/73/74）

> 前置：W-B4-49~55 全绿（**X-03**：文件未落盘前改脚本会让 7 条规则全部退 2）。**本任务必须与 49~55 同批提交**。

- [ ] **W-B4-56.1 `CARDS_APP_DIR`/`CARDS_TEST` + `cards_files()` + `require_core_faces()` + `core_files()`** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-71、AC-B4-73 · design §6.10.1、D-B4-P59、**X-03**
  - 改动内容：变量区（`CITATION_TEST=` 之后）新增 `CARDS_APP_DIR="crates/application/src/cards"`、`CARDS_TEST="crates/application/tests/card_compose.rs"`；新增 `cards_files() { for file in "${CARDS_APP_DIR}"/*.rs; do [[ -f "${file}" ]] && printf '%s\n' "${file}"; done }`；`require_core_faces()` 追加 `need_dir "${CARDS_APP_DIR}" || ok=1`（**不**要求 `CARDS_TEST` 存在，避免 target 骨架前退 2）；`core_files()` 在 `citation_files` 后追加 `cards_files` 与 `printf '%s\n' "${CARDS_TEST}"`
  - 禁止：**规则仍 7 条**；`--rule` 名称集合与退出码语义（0/1/2）不变；不得改 `rule_source`/`rule_budget` 的模式串；不得改 `check-capture-offline.sh`/`check-b3-gates.sh`/`check-mac-scope.sh`
  - DoD：`grep -c 'CARDS_APP_DIR=' scripts/check-b4-gates.sh`=1；`grep -c 'CARDS_TEST=' scripts/check-b4-gates.sh`=1；`grep -c 'cards_files' scripts/check-b4-gates.sh` ≥3（定义 + `core_files` + 56.2 的 `zero-transcript`）；`--rule=source` 与 `--rule=budget` 各退 0
  - 证据：`bash scripts/check-b4-gates.sh --rule=source && bash scripts/check-b4-gates.sh --rule=budget; echo "exit=$?"` → `exit=0`
  - 反例演练（`mktemp -d`，不入库）：复制仓库后把 `$CARDS_APP_DIR` 改名 → `--rule=source` 必须退 **2** 且打印 `SCAN-MISSING`

- [ ] **W-B4-56.2 `zero-transcript` 扫描面纳入 `cards/**` 与 `tests/card_compose.rs`** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-72 · design §6.10.1、§6.8.1、D-B4-P59
  - 改动内容：`rule_zero_transcript()` 的 `files` 数组追加 `while IFS= read -r file; do files+=("${file}"); done < <(cards_files)` 与 `if [[ -f "${CARDS_TEST}" ]]; then files+=("${CARDS_TEST}"); fi`
  - 禁止：不得新增/放宽禁止模式；不得把 `$CARDS_TEST` 改名以躲开扫描
  - DoD：`--rule=zero-transcript` 退 0；`grep -c 'CARDS_TEST' scripts/check-b4-gates.sh` ≥2（定义 + 引用）
  - 证据：`bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`
  - 反例演练（`mktemp -d`，不入库）：在 `$CARDS_TEST` 副本追加一行含 `https://` 的文本 → 退 **1**

- [ ] **W-B4-56.3 `NEW_TARGETS` 21 → 22（追加 `card_compose`）** · `scripts/test-name-inventory.sh`
  - 追溯：AC-B4-74 · spec §7.9-5 / §7.13 AC-74 · design §6.10.2、D-B4-P50
  - 改动内容：`NEW_TARGETS='… gate_keyword citation_align'` 尾部追加 ` card_compose`
  - 禁止：不得改动既有 21 个 token 的顺序/拼写；`--diff` 语义不变
  - DoD：`new_targets_count`=**22**；`grep -c 'card_compose' scripts/test-name-inventory.sh`=1；`git diff --stat HEAD -- scripts/test-name-inventory.sh` 仅 1 行变更
     - 【**P7/P8 加注**】`new_targets_count` 的时点口径：P6 收口=**22**；**P7 后=23**（追加 `keyword_card_view`）；**P8 后=24**（追加 `llm_settings_view`）。P7/P8 复跑 P6 出口时按当期值断言，不回头改本行。
  - 证据：`new_targets_count` → `22`；`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt; echo "exit=$?"` → `exit=0` 且输出含 `removed=0`

- [ ] **W-B4-56.4 `--rule=targets` 循环保持 7 项不变（X-05）+ 七规则复跑** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-71、AC-B4-74 · spec §7.9-5（"`--rule=targets` 仍只断言既有 7 项存在，多出的 `card_compose` 不影响既有断言"）· design §6.10.1、**X-05 / D-B4-P50**
  - 改动内容：**零改动**。`rule_targets()` 循环保持现状 7 项：`gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay gate_keyword`
  - 禁止：不得把 `card_compose` 加入该循环（否则断言项 7→8，与 spec 字面冲突）
  - DoD：`--rule=targets` 退 0 且输出行数 0；`grep -c 'card_compose' scripts/check-b4-gates.sh`=**0**；七条规则 7×0
  - 证据：`bash scripts/check-b4-gates.sh --rule=targets | wc -l` → `0`；`for r in source zero-transcript budget outbound record targets doc-plan; do bash scripts/check-b4-gates.sh --rule="$r" || exit 1; done; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-56.5 `b4-status.md` §15 + 零 diff 矩阵 + P6 出口收口（AC-B4-74）** · `docs/progress/b4-status.md`、仓库根
  - 追溯：AC-B4-65~74 · spec §7.10/§7.13 · design §6.9.2、§6.10.4、§6.13
  - 改动内容：追加 `## §15 P6 增量快照（W-B4-49 ~ W-B4-56）`：§15.1 命令结果表（`card_compose` 14 用例、9 target、7 规则、`NEW_TARGETS`=22、`api.rs`=4、全量 `cargo test --workspace`、`--diff` `removed=0`、`cards_items`=6/`cards_fns`=2/`cards_hits`=0）；§15.2 十行 AC 三态（AC-B4-65~74 → `passed`，各附用例名）；§15.3 U-03 关闭登记（**保留** §10.1 原文"规格已定，待实现"，行末追加"，已于 P6 关闭（见 §15）"；§14.3 的十行各在原状态后追加 `→ passed（§15）`，**不删除**原文 `pending`）；§15.4 偏差 D-B4-P53~P64 + X-07~X-09 + Y-01 裁决结论；§15.5 不宣称清单（离线通过 ≠ 解释语义正确 ≠ 真实引用可访问 ≠ 51 渲染质量；不宣称 G3/G4/真机，不宣称产品级"可点击关键词卡片"已交付）
  - 禁止：§15 内不得写任何构造术语文本、协议前缀、姓名、联系方式；只写 AC 编号、用例名、原因码、计数；**不得**把 U-05/U-06/U-07/U-08 改成 closed
  - DoD：
    - `grep -c '^## §15' docs/progress/b4-status.md`=1
    - `sed -n '/^## §15/,$p' docs/progress/b4-status.md | grep -cE 'AC-B4-(6[5-9]|7[0-4])'` ≥**10**
    - `grep -c 'U-03.*规格已定' docs/progress/b4-status.md` ≥1（**保留历史**）
    - `grep -c 'U-03.*closed' docs/progress/b4-status.md` ≥1
    - `grep -cE 'U-0[5678].*open' docs/progress/b4-status.md` ≥2
    - `cards_cases`=**14**、`cards_fns`=**2**、`cards_hits`=**0**、`kw_fields CardComposeInput "$CARDS"`=7、`kw_fields ComposedCard "$CARDS"`=11
    - `--rule=zero-transcript` 退 0、`--rule=doc-plan` 退 0
    - 零 diff 串联命令全部退 0（见下）
    - 9 个 target 逐个退 0；`cargo test --workspace` `failed=0`/`ignored=9`；`--diff` `removed=0`
  - 证据：
    ```bash
    set -e
    [ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ]
    git diff --quiet HEAD -- crates/application/src/gate
    git diff --quiet HEAD -- crates/domain/src/model
    [ "$(git diff --stat -- Cargo.toml Cargo.lock | wc -l | tr -d ' ')" = "0" ]
    git diff --quiet HEAD -- crates/application/src/understand/store.rs crates/application/src/understand/deduplicator.rs
    bash scripts/check-b4-gates.sh --rule=record
    test ! -d crates/application/tests/replay
    test ! -d crates/ui-bridge/src/keyword_card   # 【P7 加注】P7 后改为 `test -d crates/ui-bridge/src/keyword_card`（预期交付）
    [ "$(cards_hits)" = "0" ]
    for t in gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay gate_keyword citation_align card_compose; do
      "$CARGO" test -p meeting-copilot-application --test "$t" || exit 1
    done
    for r in source zero-transcript budget outbound record targets doc-plan; do
      bash scripts/check-b4-gates.sh --rule="$r" || exit 1
    done
    "$CARGO" fmt --all -- --check || exit 1
    "$CARGO" clippy --workspace --all-targets -- -D warnings || exit 1
    bash scripts/check-layering.sh || exit 1
    bash scripts/scan-secrets.sh || exit 1
    "$CARGO" deny check || exit 1
    "$CARGO" test --workspace
    bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
    echo "exit=$?"
    ```
  - 证据产物：全部退出码与日期写入 `b4-status.md` §15.1；阶段结论写入 §15.2/§15.3

---

## P7 · 51 点击 UI（增量 AC-B4-75~84）

> 阶段：编码任务规划的**第五个增量批次**（P6 之后）。需求来源：`requirements.md` §13（REQ-B4V-01~05、AC-B4-75~84、§13.4 的 10/4/5/6/2/2/3/3/2/2 封闭面）、`spec.md` §5.9 / §6.19~6.23 / §7.14 / §9.1 J-27~J-30、`design.md` §四（§4.2.2 落位、§4.3 类型、§4.5.2 接口预留、§4.7 验收命令与三态、§4.9 实施顺序）与 §7.11.4（X-11 顺序）、`docs/progress/b4-status.md` §16（D-B4-P65~P74、AC-B4-75~84 全 `pending`）。
> 进度基线（P6 收口）：`card_compose` 14 passed；`NEW_TARGETS`=**22**；`api.rs` 4 函数零 diff；`tests/replay/` 与 `crates/ui-bridge/src/keyword_card/` 不存在；`crates/ui-bridge/src/` 仅 `lib.rs`（9 行）+ `dto.rs`（65 行）；`crates/ui-bridge/tests/` 仅 `schema.rs`；`Cargo.toml` 仅 `serde` + `serde_json`（dev）。
> **范围**：只做 8 个视图类型 + 3 个纯函数 + 19 个用例 + 1 个新 target + 2 处脚本扩展 + 1 处状态登记；`keyword_card/{mod,view}.rs` 两文件。**不建** `settings/`（P8 负责）、**不建** 52 存储、**不建** `crates/application/tests/replay/`、**不改** `gate/**`/`cards/**`（源码）/`domain/**`。
> **与 U-05 的关系**：U-05 = "51 点击 UI 与设置向导"。**P7 只关闭"点击 UI"一半**（AC-B4-75~84 由 `pending` 转 `passed`）；U-05 整行**保持非 closed**，直到 P8 完成。P7 收口时在 `b4-status.md` §18 写"点击 UI 已落盘；设置向导仍待实现（见 §17）"，**不得**把 U-05 改成 `closed`。
> **与 U-06/U-07/U-08 的关系**：均保持 open，不得顺手关闭。U-06 —— `test ! -d crates/application/tests/replay` 必须退 0；U-07/U-08 —— 不承接，9 个 `#[ignore]` 名称与数量不变，不宣称 G3/G4/真机。
> **P7 出口判据（缺一即 P7 未完成）**：① `keyword_card_view` target **19 passed / 0 failed**（+1 自检用例）；② `kw_fields KeywordCardView`=**10**、`CardBodyView`=**4**、`CardDetailView`=**5**、`ViewFacts`=**6**、`CardViewOutcome`=**2**、`ClickOutcome`=**2**；`ConclusionView` 变体=**3**、`ViewRejectReason`=**3**；③ `keyword_card/view.rs` 顶层 `pub` 类型=**8**、顶层 `pub fn`=**3**；④ `NEW_TARGETS`=**23**、`--rule=targets` 退 0 且 `grep -c 'keyword_card_view' scripts/check-b4-gates.sh`=**0**；⑤ 七条规则 7×0，且 `--rule=source`/`--rule=zero-transcript` 扫描面含 `keyword_card/**` 与 `tests/keyword_card_view.rs`（目录缺失退 2）；⑥ `api.rs` 4 函数零 diff、`gate/**`/`cards/**`（源码）/`domain/src/model/**`/`infrastructure/src/llm/**` 零 diff、13 项封闭面不变、`tests/replay/` 不存在；⑦ 全量 `cargo test --workspace` `failed=0` / `ignored=9`（**含 `card_compose` 14 用例仍全绿**，X-12 已处置）、`--diff` `removed=0`；⑧ AC-75~84 十行全 `passed`、U-05 保留"规格已定"并追加"点击 UI 已落盘（W-B4-57~64）"。

**P7 需求 → AC → 用例 → 任务映射（REQ-B4V-01~05 全覆盖）**

| REQ | AC | 用例名（逐字） | 任务 |
|---|---|---|---|
| REQ-B4V-01 视图投影与字段封闭 | AC-B4-75 | `keyword_card_view_input_is_closed` | W-B4-57.3、58.1、63.1 |
| REQ-B4V-01 | AC-B4-75 | `keyword_card_view_from_composed_is_not_clickable` | W-B4-58.2、63.1 |
| REQ-B4V-03 隐藏/禁用 | AC-B4-76 | `keyword_card_view_hides_body_under_screen_share` | W-B4-58.3、63.2 |
| REQ-B4V-03 | AC-B4-76 | `keyword_card_view_hides_body_under_emergency_mask` | W-B4-58.3、63.2 |
| REQ-B4V-02 点击状态机 | AC-B4-77 | `keyword_card_view_click_state_is_downgrade_only` | W-B4-59.1、63.3 |
| REQ-B4V-02 | AC-B4-77 | `keyword_card_view_never_upgrades_disabled` | W-B4-59.3、63.3 |
| REQ-B4V-02 本地展开 | AC-B4-78 | `keyword_card_view_detail_is_local_only` | W-B4-60.1、63.4 |
| REQ-B4V-02 | AC-B4-78 | `keyword_card_view_detail_has_no_outbound_side_effect` | W-B4-60.2、63.4 |
| REQ-B4V-03 | AC-B4-79 | `keyword_card_view_withdrawn_is_not_revived` | W-B4-59.2、63.4 |
| REQ-B4V-03 | AC-B4-79 | `keyword_card_view_stale_or_cross_session_is_disabled` | W-B4-59.1、63.4 |
| REQ-B4V-03 | AC-B4-79 | `keyword_card_view_failed_alignment_stays_visible_but_disabled` | W-B4-58.3、59.1、63.4 |
| REQ-B4V-03 | AC-B4-80 | `keyword_card_view_conclusion_is_none_without_passed` | W-B4-58.5、63.5 |
| REQ-B4V-03 | AC-B4-80 | `keyword_card_view_conclusion_preserved_for_passed` | W-B4-58.5、63.5 |
| REQ-B4V-04 零转录 | AC-B4-81 | `keyword_card_view_zero_transcript_boundary` | W-B4-61.1、63.5、64.2 |
| REQ-B4V-04 | AC-B4-81 | `keyword_card_view_debug_is_whitelisted` | W-B4-61.1、63.5 |
| REQ-B4V-04 平台边界 | AC-B4-82 | `keyword_card_view_is_offline_and_platform_free` | W-B4-61.2、63.6、64.1 |
| REQ-B4V-04 | AC-B4-82 | `keyword_card_view_ignores_reference_order` | W-B4-58.4、63.6 |
| REQ-B4V-05 归属与接口 | AC-B4-83 | `keyword_card_view_ownership_is_outside_gate_and_cards` | W-B4-57.4、61.2、63.6 |
| REQ-B4V-05 | AC-B4-83 | `keyword_card_view_public_types_are_readonly` | W-B4-57.4、63.6 |
| REQ-B4V-05 | AC-B4-84 | （target 实跑，无独立用例） | W-B4-64.1~64.5 |

### P7 公共断言（每个 shell 会话执行一次，后续任务直接引用）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
UI_KEYWORD_APP_DIR="crates/ui-bridge/src/keyword_card"
UI_KW_VIEW="${UI_KEYWORD_APP_DIR}/view.rs"
UI_KW_MOD="${UI_KEYWORD_APP_DIR}/mod.rs"
UI_KW_TEST="crates/ui-bridge/tests/keyword_card_view.rs"

# 与 P4/P5/P6 同口径（沿用 P4 公共断言的 kw_fields，勿重复定义）
kw_fields() { awk -v s="pub struct $1 {" 'index($0,s)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    pub [a-z_]+:'; }
# P7 变体计数：必须把结构体变体 `Variant { ... }` 计入（D-B4-P100 同口径）
kw_variants2() { awk -v e="pub enum $1 {" 'index($0,e)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    [A-Z][A-Za-z0-9]*[[:space:]]*([({]|,)'; }
new_targets_count() { grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//" | wc -w; }
# 51 视图面：顶层 pub 类型 8 / 顶层 pub fn 3
kw_items() { grep -rcE '^pub (struct|enum) ' "${UI_KEYWORD_APP_DIR}"/*.rs | awk -F: '{s+=$2} END {print s}'; }
kw_fns()   { grep -rhcE '^pub fn ' "${UI_KEYWORD_APP_DIR}"/*.rs | awk '{s+=$1} END {print s}'; }
# 51 用例数：目标 19（全部以 keyword_card_view_ 开头；自检用例不带该前缀）
kw_cases() { grep -c '^fn keyword_card_view_' "$UI_KW_TEST"; }
# 51 禁词/倒挂一次性扫描：目标 0
kw_hits() { grep -rnE 'use crate::gate|use meeting_copilot_application::gate|compose_keyword_card|project_alignment|keyword_card_admission|card_store|AuditLog|reqwest|TcpStream|std::net' "$UI_KEYWORD_APP_DIR" 2>/dev/null | wc -l; }
```

> **只读纪律（P7 加严）**：`keyword_card/**` 只 `use meeting_copilot_domain::model::*`、`meeting_copilot_domain::vo::*`、`meeting_copilot_application::cards::ComposedCard`、`core`/`alloc`/`serde`；**禁止** `use crate::dto`（`dto` 与 51 视图互不耦合）、**禁止** `use crate::settings`（P8 面）、**禁止** `application::gate`/`domain::model::gate`。
> **零转录纪律（P7 加严）**：`$UI_KEYWORD_APP_DIR/**`、`$UI_KW_TEST`、`docs/progress/b4-status.md` §18 内**不得**出现协议前缀、`sk-` 形态 Key、`Authorization: Bearer`、模型名前缀、`request_id:` 形态、`LLM_` 前缀常量；测试内构造词只允许 `"kw1"`/`"术语1"`/`"ref-1"`/`"card-1"` 这类短串，断言消息只输出 ID/枚举/计数。
> **`--rule=source` 加严（X-14）**：`$UI_KEYWORD_APP_DIR/**` 进 `core_files()` 后**整体**扫描（源码无 `#[cfg(test)]` 时 `strip_tests()` 不生效），故源码内**禁止** `panic!(`/`todo!(`/`unimplemented!(`/`std::env`/`LLM_`/`Instant::now`/`SystemTime::now`/`thread::sleep`/`std::net`/`tokio::time`/`static mut`/`OnceLock`/`lazy_static`/`thread_local`；`view.rs` 内若放 `#[cfg(test)] mod tests`，**必须位于文件末尾**且紧跟 `^#[cfg(test)]` 行（`strip_tests()` 依赖该锚点）。
> **X-03 提交纪律（硬约束）**：`check-b4-gates.sh` 的 `require_core_faces()` 一旦加入 `UI_KEYWORD_APP_DIR`，目录缺失即令**全部** `--rule` 退 **2**。因此 **W-B4-64.1/64.2 必须在 57~63 落盘之后执行，并与 57~63 同批提交**；禁止先改脚本，禁止单独提交脚本改动。
> **X-05 口径（已冻结）**：`--rule=targets` 的循环列表**保持 7 项**（`gate_prefilter … gate_keyword`），**不追加** `keyword_card_view`；由 `NEW_TARGETS` token（22→23）与 target 实跑断言。`grep -c 'keyword_card_view' scripts/check-b4-gates.sh` 必须为 **0**。
> **X-11 口径（已冻结）**：P7 **先于** P8；P8 开工前 P7 必须已提交（`git log -1 --format=%s` 含 `keyword_card_view` 或 P7 提交标题）。

### W-B4-57 提交闸门、X-12 断言修正与视图骨架落位（AC-B4-75/83/84 前置）

> 前置：P6 已全绿但**尚未提交**（当前 `git status` 有 `crates/application/src/cards/` 未跟踪 + 3 个脚本/文档改动）。本任务是 P7 的唯一入口。

- [ ] **W-B4-57.0 P6 提交闸门（P7 开工前置）** · 仓库根
  - 追溯：AC-B4-84 · `b4-status.md` §15 · **X-11/X-13**
  - 改动内容：**零代码改动**。确认 P6 的 7 个文件面已提交（`crates/application/src/cards/{mod,compose}.rs`、`crates/application/tests/card_compose.rs`、`crates/application/src/lib.rs`、`scripts/check-b4-gates.sh`、`scripts/test-name-inventory.sh`、`docs/progress/b4-status.md`）；提交前复跑 `--rule=source`/`--rule=zero-transcript`/`--rule=targets` 三条
  - 禁止：不得把 P7 的任何文件混进 P6 的提交；不得 `--amend`/`--force`；不得跳过 hook
  - DoD：`git status --short` 为空（或仅有 `.tmp/` 忽略项）；`git log -1 --format=%s` 含 `card_compose` 或 P6 标题；`new_targets_count`=22；`card_compose` 14 passed
  - 证据：
    ```bash
    set -e
    [ -z "$(git status --porcelain)" ]
    new_targets_count
    "$CARGO" test -p meeting-copilot-application --test card_compose
    echo "exit=$?"
    ```
  - 证据产物：提交哈希与三条规则退出码写入 `b4-status.md` §18.1

- [ ] **W-B4-57.1 X-12：`card_compose` 的 51 面存在性断言改为源码文本断言** · `crates/application/tests/card_compose.rs`
  - 追溯：AC-B4-73、AC-B4-83 · spec §5.8.1-12 · design §6.10.4 · **X-12**
  - 改动内容：把 `card_compose_ownership_is_outside_gate_and_ui` 内的
    `assert!(!std::path::Path::new("crates/ui-bridge/src/keyword_card").exists());`
    改为**编译期源码断言**（路径相对 `tests/`）：
    ```rust
    // 38 只读 B4 契约，不引用 51 视图面（51 面是否落盘由 51 阶段决定）。
    let cards_source = include_str!("../src/cards/compose.rs");
    assert!(!cards_source.contains("keyword_card"));
    assert!(!cards_source.contains("ui_bridge"));
    ```
    同时**保留**同用例内的 `assert!(!std::path::Path::new("crates/application/tests/replay").exists())` 与 `GateCounters` 32 断言
  - 禁止：**不得**改 `crates/application/src/cards/**`（源码零 diff）；**不得**引入 `std::fs`（P6 的 `grep -c 'std::fs'`=0 口径继续成立）；不得改用例名（`cards_cases` 必须仍 14）；不得新增 `#[ignore]`
  - DoD：`grep -c 'ui-bridge/src/keyword_card' crates/application/tests/card_compose.rs`=**0**；`grep -c 'include_str!("../src/cards/compose.rs")'`=1；`grep -c 'std::fs'`=0；`git diff --numstat HEAD -- crates/application/tests/card_compose.rs` 为 `3 1`（或 `2 1`，取决于是否保留注释行，新增 ≤4）；`cards_cases`=14；`card_compose` 14 passed；`--rule=source` 退 0
  - 证据：
    ```bash
    set -e
    grep -c 'ui-bridge/src/keyword_card' crates/application/tests/card_compose.rs
    "$CARGO" test -p meeting-copilot-application --test card_compose
    bash scripts/check-b4-gates.sh --rule=source
    echo "exit=$?"
    ```
  - 证据产物：diff 行数与 14 passed 写入 `b4-status.md` §18.1

- [ ] **W-B4-57.2 `Cargo.toml` +2 path 依赖 + `lib.rs` +1 + `keyword_card/mod.rs` 门面** · `crates/ui-bridge/Cargo.toml`、`crates/ui-bridge/src/lib.rs`、`$UI_KW_MOD`
  - 追溯：AC-B4-75、AC-B4-83 · spec §5.9 / §9.1 J-27 · design §4.5.2（接口预留）、§4.2.2（落位）、D-B4-P65/**P73**
  - 改动内容：
    1. `crates/ui-bridge/Cargo.toml` 的 `[dependencies]` 在 `serde.workspace = true` 之后追加两行（**path 依赖，非外部 crate**）：
       ```toml
       meeting-copilot-application = { version = "0.1.0", path = "../application" }
       meeting-copilot-domain = { version = "0.1.0", path = "../domain" }
       ```
       若 application 的 `version` 与 workspace 一致，可写 `version.workspace = true`；**不得**新增任何 `[dependencies]` 之外的段
    2. `crates/ui-bridge/src/lib.rs` 在 `pub mod dto;`（第 6 行）之后插入 `pub mod keyword_card;`（**+1 / -0**，**不加** `pub use keyword_card::*`、不改 `IPC_PROTOCOL_VERSION`）
    3. 新建 `$UI_KW_MOD`（4~8 行）：`//! 51 点击 UI：只读视图投影 + 本地点击展开（0 外发/0 LLM/0 审计）。` + `pub mod view;` + 一行 `pub use view::{open_keyword_card_detail, project_composed_card_view, project_keyword_card_view, CardBodyView, CardDetailView, CardViewOutcome, ClickOutcome, ConclusionView, KeywordCardView, ViewFacts, ViewRejectReason};`（**11 个符号**）
  - 禁止：**不得**在 `lib.rs` 加顶层 `pub use`；**不得**新增外部 crate（`cargo tree` 只应多出 workspace 内 2 个 path crate，不含 `tauri`/`windows`/`objc2`/`reqwest`/`hyper`/`sqlx`）；**不得**改 `dto.rs`；`mod.rs` 只做声明与再导出，不定义类型/函数
  - DoD：`grep -c 'pub mod keyword_card;' crates/ui-bridge/src/lib.rs`=1；`git diff --numstat HEAD -- crates/ui-bridge/src/lib.rs` 为 `1 0`；`grep -c 'meeting-copilot-application' crates/ui-bridge/Cargo.toml`=1、`grep -c 'meeting-copilot-domain'`=1；`git diff --numstat HEAD -- crates/ui-bridge/Cargo.toml` 为 `2 0`；`cargo tree -p meeting-copilot-ui-bridge --edges normal --prefix none | grep -cE 'tauri|windows|objc2|reqwest|hyper|sqlx'`=0；`"$CARGO" build -p meeting-copilot-ui-bridge` 退 0（`view.rs` 先建空文件亦可，见 57.3）
  - 证据：
    ```bash
    set -e
    grep -c 'pub mod keyword_card;' crates/ui-bridge/src/lib.rs
    git diff --numstat HEAD -- crates/ui-bridge/src/lib.rs crates/ui-bridge/Cargo.toml
    "$CARGO" tree -p meeting-copilot-ui-bridge --edges normal --prefix none | grep -cE 'tauri|windows|objc2|reqwest|hyper|sqlx' || true
    "$CARGO" build -p meeting-copilot-ui-bridge
    echo "exit=$?"
    ```
  - 证据产物：`+1/-0`、`+2/-0`、`cargo tree` 禁词 0 三行写入 `b4-status.md` §18.1

- [ ] **W-B4-57.3 `keyword_card/view.rs` 8 类型 + 3 函数签名骨架（最小可编译）** · `$UI_KW_VIEW`
  - 追溯：AC-B4-75 · spec §6.19~6.23 · design §4.3.1、§4.5.2、D-B4-P65/**P67**/**P102**
  - 改动内容：文件头 `//! 51 视图模型：只读投影 13 字段卡片/11 项预览，点击只展开本地详情。`；**8 个顶层 `pub` 类型**（顺序即 `grep` 计数口径）：
    ```rust
    pub struct KeywordCardView {   // 10 字段，顺序固定
        pub card_id: CardRef,
        pub keyword_kind: KeywordKind,
        pub trigger: TriggerKind,
        pub triggered_at_ms: u64,
        pub card_status: KeywordCardStatus,
        pub body_visible: bool,
        pub body: Option<CardBodyView>,
        pub click_state: ClickState,
        pub citation_alignment: CitationAlignment,
        pub badge_count: u32,
    }
    pub struct CardBodyView {      // 4 字段
        pub title: TitleText,
        pub conclusion: ConclusionView,
        pub confidence: Confidence,
        pub references: Vec<ReferenceView>,
    }
    pub enum ConclusionView { Available(ConclusionText), NoReliableSource, Absent }   // 3 变体
    pub struct CardDetailView {    // 5 字段
        pub card_id: CardRef,
        pub title: TitleText,
        pub conclusion: ConclusionView,
        pub reference_ids: Vec<ReferenceRef>,
        pub citation_alignment: CitationAlignment,
    }
    pub struct ViewFacts {         // 6 字段
        pub session_matches: bool,
        pub authorization_revoked: bool,
        pub reference_stale: bool,
        pub screen_share: bool,
        pub emergency_masked: bool,
        pub visible_card_count: u32,
    }
    pub struct CardViewOutcome { pub view: Option<KeywordCardView>, pub reason: Option<ViewRejectReason> }  // 2 字段
    pub enum ViewRejectReason { InputContractViolation, ReferenceContractViolation, CardWithdrawn }          // 3 变体
    pub struct ClickOutcome { pub detail: Option<CardDetailView>, pub reason: Option<ClickDisabledReason> }  // 2 字段
    ```
    派生口径：`KeywordCardView`/`CardBodyView`/`CardDetailView` = `Clone + PartialEq + Serialize` + **手写 `Debug`**；`ConclusionView` = `Clone + PartialEq + Serialize` + **手写 `Debug`（61.1，不输出结论文本）**；`ViewFacts` = `Debug + Clone + Copy + PartialEq + Eq`；`CardViewOutcome` = `Debug + Clone + PartialEq`；`ViewRejectReason` = `Debug + Clone + Copy + PartialEq + Eq + Serialize`；`ClickOutcome` = `Debug + Clone + PartialEq`。**3 个顶层 `pub fn` 签名逐字**：
    ```rust
    pub fn project_keyword_card_view(card: &KeywordCardDraft, facts: &ViewFacts) -> CardViewOutcome;
    pub fn project_composed_card_view(card: &ComposedCard, facts: &ViewFacts) -> CardViewOutcome;
    pub fn open_keyword_card_detail(view: &KeywordCardView) -> ClickOutcome;
    ```
    本步允许"恒返回 `CardViewOutcome { view: None, reason: Some(InputContractViolation) }`"的**最小合法实现**（**不用** `todo!`/`unimplemented!`/`panic!`）；`open_keyword_card_detail` 恒返回 `ClickOutcome { detail: None, reason: Some(ClickDisabledReason::NotAllowed) }`
  - 禁止：**不得**加第 9 个顶层类型（含 `CardSurface` trait）；**不得** `use crate::dto`/`use crate::settings`/`use meeting_copilot_application::gate`；**不得** `derive(Debug)` 到 `KeywordCardView`/`CardBodyView`/`CardDetailView`/`ConclusionView`；`KeywordCardView.body` 不得改成非 `Option`；`ViewFacts` 不得加 `paused`/`silent`/`now_ms`
  - DoD：`kw_items`=**8**；`kw_fns`=**3**；`kw_fields KeywordCardView`=10、`CardBodyView`=4、`CardDetailView`=5、`ViewFacts`=6、`CardViewOutcome`=2、`ClickOutcome`=2；`kw_variants2 ConclusionView`=3、`ViewRejectReason`=3；`grep -c 'derive(Debug' "$UI_KW_VIEW"` 对三个视图类型为 0（可 `grep -B1 'pub struct KeywordCardView' | grep -c 'Debug'`=0）；`kw_hits`=0；`"$CARGO" build -p meeting-copilot-ui-bridge` 退 0
  - 证据：
    ```bash
    set -e
    kw_items; kw_fns
    kw_fields KeywordCardView "$UI_KW_VIEW"; kw_fields CardBodyView "$UI_KW_VIEW"
    kw_variants2 ConclusionView "$UI_KW_VIEW"; kw_variants2 ViewRejectReason "$UI_KW_VIEW"
    "$CARGO" build -p meeting-copilot-ui-bridge
    echo "exit=$?"
    ```
  - 证据产物：8/3/10/4/5/6/2/2/3/3 十项计数写入 `b4-status.md` §18.1

- [ ] **W-B4-57.4 P7 首轮零 diff 与依赖审计（X-13 收窄口径）** · 仓库根
  - 追溯：AC-B4-75、AC-B4-83 · design §4.5.2（三条硬边界 P-01~P-03）、§7.10.4 · **X-13**
  - 改动内容：**零代码改动**（只复核）。登记 P7 的零 diff 口径：根 `Cargo.toml` 零 diff、`crates/ui-bridge/Cargo.toml` 仅 +2 行、`Cargo.lock` 仅 `meeting-copilot-ui-bridge` 依赖条目变化
  - 禁止：不得为了让 `Cargo.lock` 零 diff 而手改 `Cargo.lock`（`cargo build` 自动生成即合法）；不得改根 `Cargo.toml`；不得引入 `[patch]`/`[profile]` 改动
  - DoD（每条退 0）：`git diff --quiet HEAD -- Cargo.toml`；`git diff --quiet HEAD -- crates/application/src/gate`；`git diff --quiet HEAD -- crates/application/src/cards`；`git diff --quiet HEAD -- crates/domain/src/model`；`git diff --quiet HEAD -- crates/infrastructure/src/llm`；`git diff --quiet HEAD -- crates/ui-bridge/src/dto.rs crates/ui-bridge/tests/schema.rs`；`grep -c '^pub fn ' crates/application/src/gate/api.rs`=4；`git diff --quiet HEAD -- crates/ui-bridge/src/keyword_card` **不适用**（新目录，用 `test -d` 反向断言）
  - 证据：
    ```bash
    set -e
    git diff --quiet HEAD -- Cargo.toml
    git diff --quiet HEAD -- crates/application/src/gate
    git diff --quiet HEAD -- crates/application/src/cards
    git diff --quiet HEAD -- crates/domain/src/model
    git diff --quiet HEAD -- crates/infrastructure/src/llm
    git diff --quiet HEAD -- crates/ui-bridge/src/dto.rs crates/ui-bridge/tests/schema.rs
    [ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ]
    git diff --numstat HEAD -- Cargo.lock | awk '{print $1" "$2}'
    echo "exit=$?"
    ```
  - 证据产物：六条 `git diff` 退出码 + `Cargo.lock` 行数写入 `b4-status.md` §18.1/§18.4（X-13 登记）

### W-B4-58 S0~S6 视图投影：契约校验、屏蔽态、引用 ID 面（AC-B4-75/76/79/80/82）

> 前置：W-B4-57。**判定顺序不可重排**（D-B4-P104）：`Withdrawn` 短路 → `InputContractViolation` → `ReferenceContractViolation` → 投影（屏蔽态 → 只降不升 → 组装）。

- [ ] **W-B4-58.1 S0 输入契约校验（3 类违规 → 唯一原因）** · `$UI_KW_VIEW`
  - 追溯：AC-B4-75、AC-B4-79、AC-B4-80 · spec §5.9.3-1/5 · §6.22-2 · requirements §13.3.1.3 · design **D-B4-P104**
  - 改动内容：私有 `fn validate_draft(card: &KeywordCardDraft) -> Option<ViewRejectReason>`：`!card.summary_is_consistent()` → `Some(InputContractViolation)`；`card.citation_alignment == CitationAlignment::Passed && card.references.is_empty()` → `Some(InputContractViolation)`；`card.card_status == KeywordCardStatus::Withdrawn` → `Some(CardWithdrawn)`（**最先判**）；否则 `None`。私有 `fn validate_composed(card: &ComposedCard) -> Option<ViewRejectReason>`：`!card.summary_is_consistent()` 或 `card.card_status == KeywordCardStatus::Collapsed` → `InputContractViolation`；`Passed && references.is_empty()` → `InputContractViolation`。`project_keyword_card_view` 首步调用之并短路
  - 禁止：**不得**改写输入字段（`card` 是 `&`）；不得把 `Passed`+空引用静默放行；不得对 `summary_is_consistent()==false` 做"降级展示"；不得在此步排序引用
  - DoD：`Withdrawn` 且 `summary` 不一致 → `CardWithdrawn`（撤回优先，D-B4-P69）；`Failed`+`summary=Some(非固定文案)` → `InputContractViolation`；`Passed`+空 `references` → `InputContractViolation`；`ComposedCard.card_status=Collapsed` → `InputContractViolation`；合法输入 → `reason=None`
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::contract_violation_is_single_reason` 退 0
  - 证据产物：用例名与退出码写入 `b4-status.md` §18.2

- [ ] **W-B4-58.2 `ComposedCard` 只读预览恒不可点（不补 2 项）** · `$UI_KW_VIEW`
  - 追溯：AC-B4-75 · spec §5.9 三条接口约定-① · §6.19-8 · design §4.5.2（P-03）、**D-B4-P66/P103**
  - 改动内容：`project_composed_card_view`：`click_state` 在**默认事实**（`!screen_share && !emergency_masked`）下恒 `ClickState::Disabled(ClickDisabledReason::NotAllowed)`；屏蔽事实下按 58.3 落更严格原因；`body`/`card_status`/`citation_alignment` 按 58.3/58.5 投影；**不构造** `AllowTraceRef`、**不读** `gate_decision_ref`
  - 禁止：**不得**为 `ComposedCard` 补 `gate_decision_ref`/`click_state`；**不得**调用 `keyword_card_admission`；**不得** `use meeting_copilot_application::gate`；**不得**把预览 `click_state` 落成 `Enabled`
  - DoD：默认事实下 `project_composed_card_view(...).view.unwrap().click_state == ClickState::Disabled(ClickDisabledReason::NotAllowed)`；`grep -c 'AllowTraceRef' "$UI_KW_VIEW"`=0；`grep -c 'gate_decision_ref' "$UI_KW_VIEW"`=0；`kw_hits`=0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::composed_preview_is_never_clickable` 退 0

- [ ] **W-B4-58.3 屏蔽态：投屏/紧急遮蔽隐藏正文 + 角标** · `$UI_KW_VIEW`
  - 追溯：AC-B4-76、AC-B4-79 · spec §5.9.1-2 · §6.19-6/7/10 · §6.23-1 · requirements §13.3.3-1 · design **D-B4-P70**
  - 改动内容：私有 `fn mask_state(facts: &ViewFacts) -> Option<ClickDisabledReason>`（`screen_share → ScreenShareProtection`、`emergency_masked → EmergencyMasked`，紧急优先）；投影时：命中屏蔽 → `body=None`、`body_visible=false`、`badge_count = facts.visible_card_count.max(1)`、`click_state=Disabled{该原因}`；非屏蔽 → `body=Some(CardBodyView{...})`、`body_visible=true`、`badge_count` 同样 `max(1)`；**授权撤销不隐藏正文**（只影响 `click_state`，见 59.1）
  - 禁止：屏蔽态**不得**保留 `title`/`conclusion`/`references` 任何取值（`body=None` 从结构上保证）；不得把 `badge_count` 写成 0；不得在屏蔽态返回 `Enabled`
  - DoD：`screen_share=true` → `body.is_none()`、`body_visible=false`、`badge_count`≥1、`click_state=Disabled{ScreenShareProtection}`；`emergency_masked=true` → `Disabled{EmergencyMasked}`；两者均 false → `body_visible=true`；`authorization_revoked=true` 且非屏蔽 → `body_visible=true`
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::masked_state_hides_body` 退 0

- [ ] **W-B4-58.4 引用 ID 面：5 字段、ID 升序、重复判违规（顺序无关）** · `$UI_KW_VIEW`
  - 追溯：AC-B4-82、AC-B4-75 · spec §6.20-4 · requirements §13.3.1.3-3 · design §4.3.1、**D-B4-P62（复用）**
  - 改动内容：私有 `fn sorted_references(references: &[ReferenceView]) -> Vec<ReferenceView>`（`sort_by_key(|r| r.reference_id.clone())`，稳定，**不** `dedup`）；私有 `fn has_duplicate_reference_id(references: &[ReferenceView]) -> bool`（`BTreeSet<ReferenceRef>` 长度 != `references.len()`）；`project_keyword_card_view` 在 58.1 之后、组装之前：重复 → `CardViewOutcome { view: None, reason: Some(ReferenceContractViolation) }`；否则 `references = sorted_references(...)`
  - 禁止：不得 `dedup`（重复必须在 S0 拒绝，不静默合并）；不得复制 URL/标题/摘要/正文（`ReferenceView` 类型层只有 5 字段，断言 `format!("{:?}", r)` 不含 `http`/`title`/`excerpt`）；不得读取 `ReferenceView` 之外的字段
  - DoD：3 条乱序输入（原序/逆序/首尾交换）→ `body.unwrap().references` 逐项相等且按 `reference_id` 升序；重复 ID → `reason=ReferenceContractViolation` 且 `view=None`；`kw_fields ReferenceView crates/domain/src/model/gate_keyword.rs`=5；`grep -c 'dedup' "$UI_KW_VIEW"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::references_are_sorted_and_id_only` 退 0

- [ ] **W-B4-58.5 结论三态与 `body_visible` 一致性** · `$UI_KW_VIEW`
  - 追溯：AC-B4-80、AC-B4-75 · spec §6.20（`ConclusionView` 3 变体）· §6.23-3 · requirements §13.3.1.1-2 · design **D-B4-P67/P102**
  - 改动内容：私有 `fn conclusion_of(alignment: CitationAlignment, summary: &Option<ConclusionText>) -> ConclusionView`：`Passed && Some` → `Available(text.clone())`；`Some` 且 `text.as_str() == KeywordCardDraft::NO_RELIABLE_SOURCE` → `NoReliableSource`；`None` → `Absent`；其余（非 `Passed` 的 `Some`）**在 58.1 已被 `InputContractViolation` 拦截**，此分支不可达（防御返回 `Absent`，不 `panic!`）；`body_visible = body.is_some()` 由组装处一次性赋值
  - 禁止：**不得** `Unknown→Passed`；不得把 `Failed`/`Unknown` 的 `summary` 展示为 `Available`；不得截断/改写 `ConclusionText`；不得新增第 4 个 `ConclusionView` 变体
  - DoD：`Passed`+`Some` → `Available` 且文本逐字节等于输入；`Passed`+`None` → `Absent`；`Failed`+`None` → `Absent`；`Failed`+`Some(NO_RELIABLE_SOURCE)` → `NoReliableSource`；`kw_variants2 ConclusionView`=3；`body_visible == body.is_some()` 对 4 组事实恒成立
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::conclusion_requires_passed` 退 0

### W-B4-59 点击状态机：只降不升（AC-B4-77/79）

> 前置：W-B4-58.1（`CardViewOutcome` 形态定稿）。**唯一入口** `resolve_ui_click_state`（私有），禁止在组装处分叉。

- [ ] **W-B4-59.1 八步固定优先级 `resolve_ui_click_state`** · `$UI_KW_VIEW`
  - 追溯：AC-B4-77、AC-B4-79 · spec §5.9.1-3/7 · §6.23-1 · requirements §13.3.2.1-1/2 · design **D-B4-P68**
  - 改动内容：私有 `fn resolve_ui_click_state(input: ClickState, alignment: CitationAlignment, card_status: KeywordCardStatus, facts: &ViewFacts) -> ClickState`，**严格按序**：
    1. `card_status == Withdrawn` → 由 58.1 短路（本函数不产生视图）；2. `facts.screen_share` → `Disabled(ScreenShareProtection)`；3. `facts.emergency_masked` → `Disabled(EmergencyMasked)`；4. `facts.authorization_revoked` → `Disabled(AuthorizationRevoked)`；5. `!facts.session_matches || facts.reference_stale` → `Disabled(StaleReference)`；6. 输入 `Disabled(reason)` → **保持** `Disabled(reason)`（不升级）；7. `alignment != Passed` → `Disabled(ReferenceUnverified)`；8. 输入 `Enabled` 且 1~7 全不命中 → `Enabled`
  - 禁止：**不得**把 `Disabled` 升为 `Enabled`；**不得**在 `Enabled` 分支再复用 B4 的 7 项合取（`gate_decision_ref` 51 不可见）；**不得**改变原因集合（`ClickDisabledReason` 仍 6 项，不新增）；不得让两个原因同时输出
  - DoD：输入 `Disabled(ReferenceUnverified)` + 事实全真 → 仍 `Disabled(ReferenceUnverified)`；输入 `Enabled` + `authorization_revoked` → `Disabled(AuthorizationRevoked)`；输入 `Enabled` + `screen_share` → `Disabled(ScreenShareProtection)`；输入 `Disabled(StaleReference)` + `screen_share` → `Disabled(ScreenShareProtection)`（更高优先级覆盖为更严格）；`ClickDisabledReason::ALL.len()`=6；`grep -c 'fn resolve_ui_click_state' "$UI_KW_VIEW"`=1
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::click_state_is_downgrade_only` 退 0

- [ ] **W-B4-59.2 `Withdrawn` 短路与不复活（100 次一致）** · `$UI_KW_VIEW`
  - 追溯：AC-B4-79 · spec §5.9.1-6 · §6.23-2 · requirements §13.3.3-5 · design **D-B4-P69**
  - 改动内容：`project_keyword_card_view` 在 `validate_draft` 返回 `CardWithdrawn` 时**直接** `CardViewOutcome { view: None, reason: Some(CardWithdrawn) }`，不进入 58.3~59.1 的任何分支；同输入连续 100 次调用结果逐字节一致（同一 `reason`、`view=None`）
  - 禁止：**不得**把 `Withdrawn` 映射为 `Displayed`/`Collapsed`；**不得**因 `authorization_revoked=false` 或对齐 `Passed` 而复活；**不得**在 `keyword_card/**` 写 `KeywordCardStatus::Displayed` 到输入卡片（`card` 是 `&`，编译期即不可写）
  - DoD：`Withdrawn` → `view=None` 且 `reason=CardWithdrawn`（100 次）；`card_status=Collapsed` 的 `KeywordCardDraft` → 视图 `card_status==Collapsed`（可展示，但不可点，见 59.1/7）；`grep -c 'KeywordCardStatus::Displayed' "$UI_KEYWORD_APP_DIR"/*.rs`=**0**
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::withdrawn_never_revives` 退 0

- [ ] **W-B4-59.3 `Disabled` 原原因保持 + 禁止升级复核** · `$UI_KW_VIEW`
  - 追溯：AC-B4-77 · spec §5.9.1-3 · requirements §13.3.2.1-1 · design **D-B4-P68**
  - 改动内容：对 6 个 `ClickDisabledReason` 逐一构造"输入 `Disabled(r)` + 事实全真"→ 输出恒 `Disabled(r)`；再构造"输入 `Enabled` + 事实全真 + 对齐 `Passed`"→ 输出 `Enabled`（唯一允许的 `Enabled` 路径）；复核 `resolve_ui_click_state` 是唯一写 `ClickState` 的位置（组装处只读它的返回）
  - 禁止：不得在组装处再次 `match click_state` 改写；不得新增"UI 开关"参数（无 `force_enabled`/`override`）；不得把 `Passed` 单独等价于 `Enabled`
  - DoD：6 原因 × 1 组事实 → 6 个 `Disabled` 恒等（逐项 `assert_eq!`）；`Enabled` 仅 1 条路径；`grep -c 'ClickState::Enabled' "$UI_KW_VIEW"`=**1**（仅 `resolve_ui_click_state` 的末分支）；`grep -cE 'force_enabled|override|reveal|show_plain' "$UI_KEYWORD_APP_DIR"/*.rs`=0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::disabled_reason_is_preserved` 退 0

### W-B4-60 本地详情与 `ClickOutcome`（AC-B4-78）

> 前置：W-B4-59。点击只读展开，**0 外发/0 LLM/0 审计/0 存储**。

- [ ] **W-B4-60.1 `open_keyword_card_detail`（`CardDetailView` 5 项）** · `$UI_KW_VIEW`
  - 追溯：AC-B4-78 · spec §5.9.1-4 · §6.20（`CardDetailView` 5 项）· requirements §13.3.2.1-3 · design **D-B4-P70/P67**
  - 改动内容：`open_keyword_card_detail(view: &KeywordCardView) -> ClickOutcome`：`ClickState::Enabled` 且 `body=Some` → `ClickOutcome { detail: Some(CardDetailView { card_id, title, conclusion, reference_ids, citation_alignment }), reason: None }`，`reference_ids` 从 `body.references` 取 `reference_id` 并按升序（复用 58.4 的排序结果）；`body=None` 时**不得** `unwrap`；`Disabled(reason)` 或 `body=None` → `ClickOutcome { detail: None, reason: Some(reason 或 NotAllowed) }`
  - 禁止：不得检索、不得调 LLM、不得外发、不得写审计/存储、不得抢焦点、不得重读原文/输入卡片（只用 `view`）；`click_state=Enabled` 但 `body=None`（内部不一致）→ fail-closed `reason=NotAllowed`，**不得** `panic!`
  - DoD：`Enabled`+`body=Some` → `detail.is_some()` 且 5 字段齐备、`reference_ids` 升序；`Disabled(r)` → `detail=None` 且 `reason==Some(r)`；`Enabled`+`body=None` → `reason=Some(NotAllowed)`；`grep -c 'unwrap()\|expect(' "$UI_KW_VIEW"`（`strip_tests` 后）=0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::detail_is_built_from_view_only` 退 0

- [ ] **W-B4-60.2 `ClickOutcome` XOR 与零副作用断言口径** · `$UI_KW_VIEW`
  - 追溯：AC-B4-78 · spec §6.22-3 · requirements §13.3.2.1-4 · design §4.4.2（卡片不新增审计）
  - 改动内容：`impl ClickOutcome { pub fn is_xor(&self) -> bool { self.detail.is_some() != self.reason.is_some() } }`（4 空格缩进，**不计** 3 个顶层 `pub fn`）；同上为 `CardViewOutcome` 加 `is_xor()`；两函数在任何构造点都必须满足 XOR
  - 禁止：不得在 `open_keyword_card_detail` 内产生任何 `Result`/`Err`；不得新增计数类型（51 不新增 `GateCounter`）；不得写 `b4-status.md` 之外的日志
  - DoD：两类型 4 组构造（成功/失败/双空/双非空）下 `is_xor()` 分别为 `true/true/false/false`；`grep -c 'pub fn ' "$UI_KW_VIEW"`（行首）=3（`is_xor` 为缩进 4 空格）；`grep -cE 'GateCounter|AuditLog|card_store' "$UI_KEYWORD_APP_DIR"/*.rs`=0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::outcome_is_xor` 退 0

### W-B4-61 `Debug` 白名单与零转录/平台边界（AC-B4-81/82/83）

> 前置：W-B4-58~60（三类型定稿）。本任务只加 `Debug` 与静态断言口径，不改投影语义。

- [ ] **W-B4-61.1 三视图类型手写 `Debug` 白名单（不输出标题/结论）** · `$UI_KW_VIEW`
  - 追溯：AC-B4-81 · spec §5.9.1-8 · requirements §13.3.4.1-2 · design **D-B4-P71**
  - 改动内容：手写
    ```rust
    impl core::fmt::Debug for KeywordCardView { /* card_id, keyword_kind, trigger, triggered_at_ms, card_status, body_visible, has_body(布尔), click_state, citation_alignment, badge_count */ }
    impl core::fmt::Debug for CardBodyView { /* title_chars(长度), conclusion 标签(不含文本), confidence, reference_count */ }
    impl core::fmt::Debug for CardDetailView { /* card_id, title_chars, conclusion 标签, reference_id_count, citation_alignment */ }
    ```
    `ConclusionView` 允许 `derive(Debug)`？**不允许**——它含 `Available(ConclusionText)`，`derive` 会输出文本，故 `ConclusionView` 也**手写 `Debug`**（输出 `Available(<文本长度>)`/`NoReliableSource`/`Absent`）
  - 禁止：**不得**对上述 4 类型 `derive(Debug)`；`Debug` 输出不得含 `title.as_str()`/`conclusion` 文本/`reference_id` 之外的引用字段；不得在 `Debug` 内 `format!` 拼接展示词
  - DoD：`grep -B1 'pub struct KeywordCardView' "$UI_KW_VIEW" | grep -c 'Debug'`=0；同理 `CardBodyView`/`CardDetailView`/`pub enum ConclusionView` 各 0；`grep -c 'impl core::fmt::Debug for ' "$UI_KW_VIEW"`=**4**；`format!("{:?}", view)` 不含注入的标题/结论/术语文本（用例断言见 63.5）；`kw_items`=8、`kw_fns`=3（不变）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib keyword_card::view::tests::debug_is_whitelisted` 退 0
  - 证据产物：4 个 `impl core::fmt::Debug` 计数写入 `b4-status.md` §18.2

- [ ] **W-B4-61.2 零转录与平台边界静态判据（记录面 0 写入、8 类符号 0 命中）** · 仓库根
  - 追溯：AC-B4-81、AC-B4-82、AC-B4-83 · spec §5.9.1-8/9 · requirements §13.3.4.1-3 · design §4.4.1/§4.4.3、D-B4-P71
  - 改动内容：**零代码改动**（只建断言口径）。登记：①`keyword_card/**` 不写 `GateRecord`/`GateAuditEvent`/`ReplayReport`/夹具；②无 `tauri`/`windows`/`objc2`/`reqwest`/`TcpStream`/`std::net`/`Instant::now`/`SystemTime::now`/`thread::sleep`/`std::env`/`std::fs`/`issue_query`/`AuditLog`/`card_store`
  - 禁止：不得以注释形式写入上述禁词字面量（`--rule=source` 也扫注释）；不得把 `title`/`conclusion` 写进 `b4-status.md` §18（只写 AC 编号/用例名/计数）
  - DoD：`kw_hits`=0；`grep -rnE 'tauri|windows|objc2|std::fs|issue_query|GateRecord|GateAuditEvent|ReplayReport' "$UI_KEYWORD_APP_DIR"`=0；`grep -rnE 'title|conclusion' docs/progress/b4-status.md | sed -n '/§18/,$p' | grep -cE '=\s*"|：.*[一-龥]{6,}'`=0（只人工复核，机器判据以 `--rule=zero-transcript` 为准）
  - 证据：
    ```bash
    set -e
    [ "$(kw_hits)" = "0" ]
    ! grep -rqE 'tauri|windows|objc2|std::fs|issue_query|GateRecord|GateAuditEvent|ReplayReport' crates/ui-bridge/src/keyword_card
    echo "exit=$?"
    ```

### W-B4-62 `keyword_card_view` target 骨架与本地夹具（AC-B4-84 前置）

> 前置：W-B4-57~61 全部编译通过。新增 `$UI_KW_TEST`（**第 10 个 target**）；**不** `mod common`（51 只依赖 domain/application 公开类型）。

- [ ] **W-B4-62.1 文件头 + 4 个本地构造器** · `$UI_KW_TEST`
  - 追溯：AC-B4-75、AC-B4-84 · design §4.5.2、§4.7.1
  - 改动内容：文件头 `//! 51 点击 UI（AC-B4-75~84，19 用例 + 1 自检）。`；`use meeting_copilot_ui_bridge::keyword_card::{...};`、`use meeting_copilot_domain::model::{...};`、`use meeting_copilot_domain::vo::{...};`、`use meeting_copilot_application::cards::{ComposedCard, ...};`；构造器（**全部可用**，不得 `#[ignore]`）：
    - `fn draft(card_id: &str, term: &str, alignment: CitationAlignment, status: KeywordCardStatus, click: ClickState, summary: Option<&str>, refs: &[&str]) -> KeywordCardDraft`（`gate_decision_ref = AllowTraceRef::with_trace(TraceId::new("t-1").unwrap(), Some(GateStage::G5)).unwrap()` 或等价合法构造；`triggered_at_ms = 1_000`；`confidence = Confidence::new(0.87).unwrap()`；`keyword_text = TitleText::new(term).unwrap()`；`normalized_term = NormalizedTerm::new(term).unwrap()`）
    - `fn facts(session: bool, revoked: bool, stale: bool, share: bool, masked: bool, visible: u32) -> ViewFacts`
    - `fn composed(card_id: &str, term: &str, alignment: CitationAlignment, status: KeywordCardStatus) -> ComposedCard`（11 项，`trigger = TriggerKind::Auto`）
    - `fn reference(id: &str) -> ReferenceView`
  - 禁止：不得 `std::fs`/`File::`/`println!`/`dbg!`/`panic!(`；不得出现协议前缀/模型名/`sk-` 形态；`assert_eq!` 不得直接打印术语文本（比较 ID/枚举/计数/长度）；不得依赖 `common::Harness`（会引入 `gate` 依赖到 51 测试面）
  - DoD：`"$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view --no-run` 退 0；`grep -c '^fn ' "$UI_KW_TEST"` ≥4；`grep -c '#\[ignore\]' "$UI_KW_TEST"`=0；`grep -cE 'std::fs|panic!\(' "$UI_KW_TEST"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view --no-run; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-62.2 构造器自检用例 `sample_fixtures_are_closed_and_valid`（不计入 19）** · `$UI_KW_TEST`
  - 追溯：AC-B4-75 · design §4.3.1、I-01/I-04/I-07
  - 改动内容：1 个非 AC 自检用例：`assert_eq!(kw_fields_like(draft_fields), 13)`（用本地常量断言 13 个字段名存在）、`assert!(draft.summary_is_consistent())`、`assert!(draft.click_state_is_consistent() || matches!(click, ClickState::Disabled(_)))`、`assert_eq!(composed.references.len(), 0)`、`assert_eq!(facts.visible_card_count, 0)`
  - 禁止：**不得**命名成 `keyword_card_view_*` 前缀（否则 `kw_cases` 虚高）；不得断言任何文本取值
  - DoD：`kw_cases`=**0**（此时尚未写 AC 用例）；`"$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view sample_fixtures_are_closed_and_valid` 退 0（1 passed）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view; echo "exit=$?"` → `1 passed`

### W-B4-63 AC-75~83 用例（19 个）

> 前置：W-B4-62。用例名与 `spec.md` §7.14 / `design.md` §4.7.1 / `requirements.md` §13.5 **逐字一致**；每个用例至少 1 条真断言，禁止空断言；断言消息只输出 ID/枚举/计数/长度。

- [ ] **W-B4-63.1 AC-75 两用例：字段封闭与 `ComposedCard` 不可点** · `$UI_KW_TEST`
  - 追溯：AC-B4-75 · spec §7.14 AC-75、§6.19~6.22 · requirements §13.5 AC-75 · design §4.7.1、**D-B4-P66/P67/P103**
  - DoD（`keyword_card_view_input_is_closed`）：投影 13 字段 `draft` 后 `view.is_some()`；`view` 的 10 个字段逐项可访问；`input` 未被改写（`assert_eq!(draft, snapshot)`）；`assert_eq!(GateCounters::default().snapshot().len(), 32)`；`assert!(!std::path::Path::new("crates/application/tests/replay").exists())`；成功后 `assert!(matches!(outcome.view, Some(_)))`
  - DoD（`keyword_card_view_from_composed_is_not_clickable`）：`composed(...)` + 默认事实 → `view.unwrap().click_state == ClickState::Disabled(ClickDisabledReason::NotAllowed)`；`assert!(!dbg.contains("Enabled"))`（`format!("{:?}", view)`）
  - 禁止：不得用 `serde_json` 断言字段数（由外部 `kw_fields` 承担）；不得构造 `ComposedCard` 的 `gate_decision_ref`
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view keyword_card_view_input_is_closed keyword_card_view_from_composed_is_not_clickable` 退 0

- [ ] **W-B4-63.2 AC-76 两用例：投屏/紧急遮蔽隐藏正文** · `$UI_KW_TEST`
  - 追溯：AC-B4-76 · spec §7.14 AC-76、§6.23-1 · requirements §13.5 AC-76 · design **D-B4-P70**
  - DoD（`keyword_card_view_hides_body_under_screen_share`）：`facts(share=true)` → `body.is_none()`、`body_visible=false`、`badge_count>=1`、`click_state==Disabled(ScreenShareProtection)`；序列化 `serde_json::to_value(view)["body"]` 为 `null`（`serde_json` 是 ui-bridge 的 dev-dep，允许）
  - DoD（`keyword_card_view_hides_body_under_emergency_mask`）：`facts(masked=true)` → 同上但 `Disabled(EmergencyMasked)`；两者均 true → `EmergencyMasked`（紧急优先）
  - 禁止：不得断言 `title` 在屏蔽态仍存在；不得用 `format!` 打印文本
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view keyword_card_view_hides_body_under_screen_share keyword_card_view_hides_body_under_emergency_mask` 退 0

- [ ] **W-B4-63.3 AC-77 两用例：只降不升与禁止升级** · `$UI_KW_TEST`
  - 追溯：AC-B4-77 · spec §7.14 AC-77、§5.9.1-3 · requirements §13.5 AC-77 · design **D-B4-P68**
  - DoD（`keyword_card_view_click_state_is_downgrade_only`）：`Enabled` + `authorization_revoked` → `Disabled(AuthorizationRevoked)`；`Enabled` + `!session_matches` → `Disabled(StaleReference)`；`Enabled` + `reference_stale` → `Disabled(StaleReference)`；`Enabled` + `alignment=Failed` → `Disabled(ReferenceUnverified)`；`Enabled` + `screen_share` → `Disabled(ScreenShareProtection)`；`assert_eq!(ClickDisabledReason::ALL.len(), 6)`
  - DoD（`keyword_card_view_never_upgrades_disabled`）：6 个原因逐一"`Disabled(r)` + 事实全真"→ 恒 `Disabled(r)`；`Disabled(NotAllowed)` + `alignment=Passed` → 仍 `Disabled(NotAllowed)`
  - 禁止：不得出现"`Disabled` → `Enabled`"的断言反例（即不得写允许升级的用例）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view keyword_card_view_click_state_is_downgrade_only keyword_card_view_never_upgrades_disabled` 退 0

- [ ] **W-B4-63.4 AC-78 两用例 + AC-79 三用例：本地详情、无外发、撤回/过期/对齐** · `$UI_KW_TEST`
  - 追溯：AC-B4-78、AC-B4-79 · spec §7.14 AC-78/79、§5.9.1-4/6/7、§6.20、§6.23-1/2 · requirements §13.5 · design **D-B4-P69/P70**
  - DoD（`keyword_card_view_detail_is_local_only`）：`Enabled`+`body=Some` → `open_keyword_card_detail` 返回 `detail=Some`，5 字段齐备、`reference_ids` 升序、`detail.reason.is_none()`；`ClickOutcome` XOR
  - DoD（`keyword_card_view_detail_has_no_outbound_side_effect`）：连续调用 100 次 → `format!("{:?}", detail)` 100 次相等；`assert_eq!(GateCounters::default().snapshot().len(), 32)`；无外发计数可读（51 无该字段）→ 用静态断言替代：`assert!(!format!("{:?}", click).contains("outbound"))`
  - DoD（`keyword_card_view_withdrawn_is_not_revived`）：`status=Withdrawn` → 100 次 `view.is_none()` 且 `reason==Some(CardWithdrawn)`
  - DoD（`keyword_card_view_stale_or_cross_session_is_disabled`）：`session=false` → `Disabled(StaleReference)`；`stale=true` → `Disabled(StaleReference)`
  - DoD（`keyword_card_view_failed_alignment_stays_visible_but_disabled`）：`Failed`/`Unknown` → `body_visible=true`（非投屏/非撤回）且 `click_state==Disabled(ReferenceUnverified)`；`view.is_some()`
  - 禁止：不得在详情用例里读输入卡片原文；不得断言 `outbound_attempts` 等 51 不存在的字段（用 `GateCounters` 32 + `Debug` 不含 outbound 替代）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view keyword_card_view_detail_is_local_only keyword_card_view_detail_has_no_outbound_side_effect keyword_card_view_withdrawn_is_not_revived keyword_card_view_stale_or_cross_session_is_disabled keyword_card_view_failed_alignment_stays_visible_but_disabled` 退 0

- [ ] **W-B4-63.5 AC-80 两用例 + AC-81 两用例：结论降级、零转录、`Debug` 白名单** · `$UI_KW_TEST`
  - 追溯：AC-B4-80、AC-B4-81 · spec §7.14 AC-80/81、§6.20、§6.23-3 · requirements §13.5 · design **D-B4-P71/P102**
  - DoD（`keyword_card_view_conclusion_is_none_without_passed`）：`Failed`+`summary=None` → `Absent`；`Failed`+`Some(NO_RELIABLE_SOURCE)` → `NoReliableSource`；`Passed`+`Some(合法短文本)` → 不 `Available`？**注意**：`Passed`+`Some` 必须 `Available`（另一用例）；本用例断言"非 `Passed` 不出现 `Available`"，并断言 `kw_variants2 ConclusionView`=3（外部）
  - DoD（`keyword_card_view_conclusion_preserved_for_passed`）：`Passed`+`summary=Some("已核对")` → `Available` 且文本逐字节等于输入
  - DoD（`keyword_card_view_zero_transcript_boundary`）：`format!("{:?}", view)` 不含 `draft.keyword_text.as_str()`、不含结论文本；`format!("{:?}", detail)` 同理；`assert!(!format!("{:?}", view).contains(draft.keyword_text.as_str()))`
  - DoD（`keyword_card_view_debug_is_whitelisted`）：`format!("{:?}", view)` 含 `click_state`/`badge_count`/`has_body` 等白名单键；不含 `标题`/结论文本；`format!("{:?}", body)` 含 `title_chars` 而不含文本
  - 禁止：断言消息不得打印文本；不得把文本写入 `b4-status.md`
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view keyword_card_view_conclusion_is_none_without_passed keyword_card_view_conclusion_preserved_for_passed keyword_card_view_zero_transcript_boundary keyword_card_view_debug_is_whitelisted` 退 0
  - 外部：`bash scripts/check-b4-gates.sh --rule=zero-transcript` 退 0（W-B4-64.2 落盘后）

- [ ] **W-B4-63.6 AC-82 两用例 + AC-83 两用例：离线确定性、顺序无关、归属、只读公开面** · `$UI_KW_TEST`
  - 追溯：AC-B4-82、AC-B4-83 · spec §7.14 AC-82/83、§5.9.1-8/9、§6.23-4/5 · requirements §13.5 · design §4.5.2（P-01~P-03）
  - DoD（`keyword_card_view_is_offline_and_platform_free`）：同输入 100 次 → `serde_json::to_vec(&view)` 逐字节相等；`body`/`click_state`/`badge_count` 逐项相等；不读系统时间（`triggered_at_ms` 恒等于输入）
  - DoD（`keyword_card_view_ignores_reference_order`）：3 种排列 → `body.references` 逐项相等且升序；`format!("{:?}", view)` 相等
  - DoD（`keyword_card_view_ownership_is_outside_gate_and_cards`）：`assert_eq!(GateCounters::default().snapshot().len(), 32)`；`assert!(!std::path::Path::new("crates/application/tests/replay").exists())`；`assert_eq!(ComposeRejectReason::ALL.len(), 6)`（38 仍 6）；外部 `kw_hits`=0
  - DoD（`keyword_card_view_public_types_are_readonly`）：`KeywordCardView`/`CardBodyView`/`CardDetailView` 的字段均为 `pub` 但**无 `pub fn` 写入入口**（断言 `kw_fns`=3 且 3 个函数名恰为 3 个投影/展开函数）；`assert!(!format!("{:?}", view).contains("gate_decision_ref"))`；`assert!(!format!("{:?}", view).contains("trace_id"))`
  - 禁止：不得断言 `keyword_card/**` 不存在的目录（已落盘）；不得新建 UI/存储/回放文件
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view keyword_card_view_is_offline_and_platform_free keyword_card_view_ignores_reference_order keyword_card_view_ownership_is_outside_gate_and_cards keyword_card_view_public_types_are_readonly` 退 0

### W-B4-64 脚本扫描面、target 清单与 P7 出口收口（AC-B4-81/82/83/84）

> 前置：W-B4-57~63 全绿（**X-03**：文件未落盘前改脚本会让 7 条规则全部退 2）。**本任务必须与 57~63 同批提交**。

- [ ] **W-B4-64.1 `UI_KEYWORD_APP_DIR`/`UI_KEYWORD_TEST`（拼接写法）+ `ui_keyword_files()` + `require_core_faces()` + `core_files()`** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-82、AC-B4-83 · design §4.7.2、§7.10.1（同范式）、**D-B4-P72**、X-03
  - 改动内容：变量区（`CARDS_TEST=` 之后）新增（**路径用字符串拼接**，使脚本文件内不含完整 target 名）：
    ```bash
    UI_KEYWORD_APP_DIR="crates/ui-bridge/src/keyword_card"
    UI_KEYWORD_TEST="crates/ui-bridge/tests/keyword""_card_view.rs"
    ```
    新增 `ui_keyword_files() { local file; for file in "${UI_KEYWORD_APP_DIR}"/*.rs; do [[ -f "${file}" ]] && printf '%s\n' "${file}"; done }`；`require_core_faces()` 末尾追加 `need_dir "${UI_KEYWORD_APP_DIR}" || ok=1`（**不**要求测试文件存在）；`core_files()` 在 `cards_files` 之后追加 `ui_keyword_files`（**只加源码**，不加测试）
  - 禁止：**规则仍 7 条**；`--rule` 名称集合与退出码语义（0/1/2）不变；不得改 `rule_source`/`rule_budget`/`rule_record` 模式串；不得改 `check-capture-offline.sh`/`check-b3-gates.sh`/`check-mac-scope.sh`；**不得**在脚本内写出完整的 `keyword_card_view` 字符串
  - DoD：`grep -c 'UI_KEYWORD_APP_DIR=' scripts/check-b4-gates.sh`=1；`grep -c 'UI_KEYWORD_TEST=' scripts/check-b4-gates.sh`=1；`grep -c 'ui_keyword_files' scripts/check-b4-gates.sh` ≥3（定义 + `core_files` + 64.2）；`grep -c 'keyword_card_view' scripts/check-b4-gates.sh`=**0**；`--rule=source` 与 `--rule=budget` 各退 0
  - 证据：`bash scripts/check-b4-gates.sh --rule=source && bash scripts/check-b4-gates.sh --rule=budget; echo "exit=$?"` → `exit=0`
  - 反例演练（`mktemp -d`，不入库）：复制仓库后把 `$UI_KEYWORD_APP_DIR` 改名 → `--rule=source` 必须退 **2** 且打印 `SCAN-MISSING`

- [ ] **W-B4-64.2 `zero-transcript` 扫描面纳入 `keyword_card/**` 与 `tests/keyword_card_view.rs`** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-81 · design §4.4.3（静态层）、**D-B4-P72**
  - 改动内容：`rule_zero_transcript()` 的 `files` 数组在 `cards_files` 之后追加：
    ```bash
    while IFS= read -r file; do files+=("${file}"); done < <(ui_keyword_files)
    if [[ -f "${UI_KEYWORD_TEST}" ]]; then files+=("${UI_KEYWORD_TEST}"); fi
    ```
  - 禁止：不得新增/放宽禁止模式；不得把 `$UI_KEYWORD_TEST` 改名躲扫描；不得把测试文件加进 `core_files()`（与 38 的 D-B4-P59 不同）
  - DoD：`--rule=zero-transcript` 退 0；`grep -c 'UI_KEYWORD_TEST' scripts/check-b4-gates.sh` ≥2（定义 + 引用）；`grep -c 'UI_KEYWORD_TEST' <(core_files 定义段)`=0（测试不进 source 面）
  - 证据：`bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`
  - 反例演练（`mktemp -d`，不入库）：在 `$UI_KEYWORD_TEST` 副本追加一行含 `https://` 的文本 → 退 **1**

- [ ] **W-B4-64.3 `NEW_TARGETS` 22 → 23（追加 `keyword_card_view`）** · `scripts/test-name-inventory.sh`
  - 追溯：AC-B4-84 · spec §7.14（target 名与扫描面）· design §4.7.2、§7.10.2、**D-B4-P72/X-11**
  - 改动内容：`NEW_TARGETS='… card_compose'` 尾部追加 ` keyword_card_view`
  - 禁止：不得改动既有 22 个 token 的顺序/拼写；`--diff` 语义不变；不得提前追加 `llm_settings_view`（P8 负责）
  - DoD：`new_targets_count`=**23**；`grep -c 'keyword_card_view' scripts/test-name-inventory.sh`=1；`grep -c 'llm_settings_view' scripts/test-name-inventory.sh`=0；`git diff --stat HEAD -- scripts/test-name-inventory.sh` 仅 1 行变更
  - 证据：`new_targets_count` → `23`；`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt; echo "exit=$?"` → `exit=0` 且输出含 `removed=0`

- [ ] **W-B4-64.4 `--rule=targets` 循环保持 7 项（X-05）+ 七规则复跑** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-82、AC-B4-84 · spec §7.14（"`--rule=targets` 循环仍为既有 7 项，不追加 `keyword_card_view`"）· **X-05**
  - 改动内容：**零改动**。`rule_targets()` 循环保持现状 7 项：`gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay gate_keyword`
  - 禁止：不得把 `keyword_card_view` 加入该循环（否则断言项 7→8，与 spec 字面冲突）
  - DoD：`--rule=targets` 退 0 且输出行数 0；`grep -c 'keyword_card_view' scripts/check-b4-gates.sh`=0；七条规则 7×0；`"$CARGO" tree -p meeting-copilot-ui-bridge --edges normal --prefix none | grep -cE 'tauri|windows|objc2|reqwest|hyper|sqlx'`=0
  - 证据：
    ```bash
    set -e
    [ -z "$(bash scripts/check-b4-gates.sh --rule=targets)" ]
    for r in source zero-transcript budget outbound record targets doc-plan; do
      bash scripts/check-b4-gates.sh --rule="$r" || exit 1
    done
    echo "exit=$?"
    ```
    `"$CARGO" tree -p meeting-copilot-ui-bridge --edges normal --prefix none` 禁词命中 0

- [ ] **W-B4-64.5 `b4-status.md` §18 + 零 diff 矩阵 + P7 出口收口（AC-B4-75~84）** · `docs/progress/b4-status.md`、仓库根
  - 追溯：AC-B4-75~84 · spec §7.14 · design §4.7.3、§7.10.4、§7.13
  - 改动内容：追加 `## §18 P7 增量快照（W-B4-57 ~ W-B4-64）`：§18.1 命令结果表（`keyword_card_view` 19+1 用例、10 target、7 规则、`NEW_TARGETS`=23、`api.rs`=4、`kw_items`=8/`kw_fns`=3/`kw_hits`=0、`Cargo.toml +2`/`Cargo.lock` 依赖条目、全量 `cargo test --workspace`、`--diff` `removed=0`）；§18.2 十行 AC 三态（AC-B4-75~84 → `passed`，各附用例名）；§18.3 U-05 部分关闭登记（**保留** §10.1/§16 原文"规格已定，待实现"，行末追加"（点击 UI 部分已于 P7 关闭，见 §18；设置向导仍待 P8）"，**不得**写 `closed`）；§18.4 偏差登记 X-12（`card_compose` 路径断言改写）、X-13（`Cargo.lock` 零 diff 收窄）、D-B4-P102~P104；§18.5 不宣称清单（离线通过 ≠ 真实像素/焦点/三形态/投屏 API/产品级交互质量；不宣称 G3/G4/真机，不宣称产品级"可点击关键词卡片"已交付）
  - 禁止：§18 内不得写任何构造术语/标题/结论/协议前缀/姓名/联系方式；只写 AC 编号、用例名、原因码、计数；**不得**把 U-05 改成 `closed`；不得把 U-06/U-07/U-08 改成 closed
  - DoD：
    - `grep -c '^## §18' docs/progress/b4-status.md`=1
    - `sed -n '/^## §18/,$p' docs/progress/b4-status.md | grep -cE 'AC-B4-(7[5-9]|8[0-4])'` ≥**10**
    - `grep -c '^| AC-B4-' .codeartsdoer/specs/b4_gate/spec.md`=**94**（**X-16 收窄**：AC-84 原字面"=84"在 85~94 追加后不可成立；本行改为与 P8 收口同值 94）
    - `grep -c 'U-05.*规格已定' docs/progress/b4-status.md` ≥1（保留历史）
    - `grep -c 'U-05.*设置向导' docs/progress/b4-status.md` ≥1
    - `grep -cE 'U-0[5678].*open' docs/progress/b4-status.md` ≥2（历史行仍在）
    - `kw_cases`=**19**、`kw_items`=**8**、`kw_fns`=**3**、`kw_hits`=**0**
    - `kw_fields KeywordCardView`=10、`CardBodyView`=4、`CardDetailView`=5、`ViewFacts`=6、`CardViewOutcome`=2、`ClickOutcome`=2
    - `--rule=zero-transcript` 退 0、`--rule=doc-plan` 退 0
    - 零 diff 串联命令全部退 0（含 `card_compose` 14 用例仍绿）
    - 10 个 target 逐个退 0；`cargo test --workspace` `failed=0`/`ignored=9`；`--diff` `removed=0`
  - 证据：
    ```bash
    set -e
    [ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ]
    git diff --quiet HEAD -- crates/application/src/gate
    git diff --quiet HEAD -- crates/application/src/cards
    git diff --quiet HEAD -- crates/domain/src/model
    git diff --quiet HEAD -- crates/infrastructure/src/llm
    git diff --quiet HEAD -- crates/ui-bridge/src/dto.rs crates/ui-bridge/tests/schema.rs
    git diff --quiet HEAD -- Cargo.toml
    bash scripts/check-b4-gates.sh --rule=record
    test ! -d crates/application/tests/replay
    test ! -d crates/ui-bridge/src/settings
    [ "$(kw_cases)" = "19" ] && [ "$(kw_items)" = "8" ] && [ "$(kw_fns)" = "3" ] && [ "$(kw_hits)" = "0" ]
    for t in gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay gate_keyword citation_align card_compose keyword_card_view; do
      "$CARGO" test -p meeting-copilot-ui-bridge --test "$t" 2>/dev/null || "$CARGO" test -p meeting-copilot-application --test "$t" || exit 1
    done
    for r in source zero-transcript budget outbound record targets doc-plan; do
      bash scripts/check-b4-gates.sh --rule="$r" || exit 1
    done
    "$CARGO" fmt --all -- --check || exit 1
    "$CARGO" clippy --workspace --all-targets -- -D warnings || exit 1
    bash scripts/check-layering.sh || exit 1
    bash scripts/scan-secrets.sh || exit 1
    "$CARGO" deny check || exit 1
    "$CARGO" test --workspace
    bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
    echo "exit=$?"
    ```
  - 证据产物：全部退出码与日期写入 `b4-status.md` §18.1；阶段结论写入 §18.2/§18.3

---

## P8 · 51 设置向导（增量 AC-B4-85~94）

> 阶段：编码任务规划的**第六个增量批次**（P7 之后，**串行**）。需求来源：`requirements.md` §14（REQ-B4W-01~07、AC-B4-85~94、§14.4 的 8/6/5/3/6/5/8/6/2/4 封闭面）、`spec.md` §5.10 / §6.24~6.28 / §7.15 / §9.1 J-31~J-34、`design.md` §七（§7.0~§7.13，含 §7.11.2 D-B4-P85~P101 与 §7.11.4 X-10/X-11）、`docs/progress/b4-status.md` §17（D-B4-P75~P84、AC-B4-85~94 全 `pending`）。
> 进度基线（P7 收口后）：`keyword_card_view` 19 passed；`NEW_TARGETS`=**23**；`api.rs` 4 函数零 diff；`crates/ui-bridge/src/` = `lib.rs` + `dto.rs` + `keyword_card/{mod,view}.rs`；`crates/ui-bridge/Cargo.toml` 含 2 条 path 依赖；`crates/ui-bridge/src/settings/` 不存在；`crates/infrastructure/src/llm/**` 零 diff（`request_timeout_ms` 默认 6_000）。
> **范围**：只做 18 个顶层类型 + 7 个顶层纯函数 + 3 常量 + 18 个用例 + 1 个新 target + 2 处脚本扩展 + 1 处状态登记；`settings/{mod,view,input,probe}.rs` 四文件。**不建**平台句柄/密钥链适配/52 存储/`tests/replay/`；**不新增依赖**；**不改** `keyword_card/**`、`gate/**`、`cards/**`、`domain/**`、`infrastructure/src/llm/**`。
> **与 U-05 的关系**：P8 关闭 U-05 的"设置向导"一半；**P8 完成后 U-05 整行才标 `closed（W-B4-57~72）`**。§10.1/§16/§17 的"规格已定"原文**保留**（AC-94 的 `grep -c 'U-05.*设置向导'` 与 AC-84 的 `grep -c 'U-05.*规格已定'` 依赖它）。
> **与 U-06/U-07/U-08 的关系**：均保持 open；`tests/replay/` 仍不存在；9 个 `#[ignore]` 名称与数量不变；不宣称 G3/G4/真机。
> **P8 出口判据（缺一即 P8 未完成）**：① `llm_settings_view` target **18 passed / 0 failed**；② `kw_fields SettingsView`=**8**、`SettingsFacts`=**6**、`SettingsInput`=**5**、`SettingsDefaults`=**4**、`ProbeRequest`=**4**；③ `settings/**` 顶层 `pub` 类型=**18**、顶层 `pub fn`=**7**；`SECTION_ORDER.len()`=**4**；`SettingsFormState`=6、`ProbeEvent`=5、`ProbeFailureKind`=8、`FieldIssue`=6、`ProbeOutcome`=2、`SettingsIntent`=3、`MaskedKey`=2、`ModelSelectionView`=2、`EndpointSelectionView`=2、`ModelDraft`=3、`EndpointDraft`=2、`PersistTarget`=2；④ `NEW_TARGETS`=**24**、`--rule=targets` 退 0 且 `grep -c 'llm_settings_view' scripts/check-b4-gates.sh`=**0**；⑤ 七条规则 7×0，`--rule=source` 覆盖 `settings/**`（缺目录退 2）、`--rule=zero-transcript` 覆盖 `settings/**` + `tests/llm_settings_view.rs`；⑥ `Cargo.toml`/`Cargo.lock` 零 diff（以 P7 提交后的 HEAD 为基线）；`api.rs` 4、13 项封闭面不变；⑦ 全量 `cargo test --workspace` `failed=0`/`ignored=9`、`--diff` `removed=0`；⑧ AC-85~94 十行全 `passed`、U-05 标 `closed` 且保留"规格已定"。

**P8 需求 → AC → 用例 → 任务映射（REQ-B4W-01~07 全覆盖）**

| REQ | AC | 用例名（逐字） | 任务 |
|---|---|---|---|
| REQ-B4W-01 分区与一步式默认 | AC-B4-85 | `settings_view_is_closed_and_single_step` | W-B4-65.1、65.2、67.1、71.2 |
| REQ-B4W-01 | AC-B4-85 | `settings_defaults_need_only_api_key` | W-B4-67.1、67.3、71.2 |
| REQ-B4W-02 Key 内联校验 | AC-B4-86 | `settings_api_key_inline_errors_are_lenient` | W-B4-66.1、66.3、71.3 |
| REQ-B4W-02 | AC-B4-86 | `settings_errors_only_after_submit` | W-B4-66.3、71.3 |
| REQ-B4W-02 固定掩码 | AC-B4-87 | `settings_api_key_is_masked_without_payload` | W-B4-66.2、71.3 |
| REQ-B4W-02 | AC-B4-87 | `settings_debug_and_errors_never_leak_key` | W-B4-66.2、65.3、71.3 |
| REQ-B4W-06 保存/清除 | AC-B4-88 | `settings_persists_intent_only_and_clears_with_confirmation` | W-B4-68.1、71.4 |
| REQ-B4W-06 | AC-B4-88 | `settings_repo_has_no_plaintext_key` | W-B4-68.2、71.4 |
| REQ-B4W-03 模型 | AC-B4-89 | `settings_model_preset_or_custom_is_closed` | W-B4-67.2、71.4 |
| REQ-B4W-03 | AC-B4-89 | `settings_model_name_is_not_inlined` | W-B4-67.2、71.4、72.2 |
| REQ-B4W-03 端点 | AC-B4-90 | `settings_endpoint_is_allowlist_index_only` | W-B4-67.3、71.4 |
| REQ-B4W-03 | AC-B4-90 | `settings_endpoint_never_expands_allowlist` | W-B4-67.3、71.4、72.4 |
| REQ-B4W-04 状态机 | AC-B4-91 | `settings_probe_state_machine_is_single_flight` | W-B4-69.1、69.2、71.5 |
| REQ-B4W-04 | AC-B4-91 | `settings_probe_cancel_and_cooldown` | W-B4-69.1、69.3、71.5 |
| REQ-B4W-05 失败人话化 | AC-B4-92 | `settings_failure_messages_are_humanized` | W-B4-70.1、71.5 |
| REQ-B4W-05 | AC-B4-92 | `settings_failure_keeps_user_input` | W-B4-70.2、71.5 |
| REQ-B4W-06 零转录/边界 | AC-B4-93 | `settings_boundary_is_offline_and_credential_safe` | W-B4-68.2、71.6、72.2 |
| REQ-B4W-06 | AC-B4-93 | `settings_debug_is_whitelisted` | W-B4-65.2、65.3、71.6 |
| REQ-B4W-07 归属与接口 | AC-B4-94 | （target 实跑，无独立用例） | W-B4-65.5、72.1~72.5 |

### P8 公共断言（每个 shell 会话执行一次，后续任务直接引用）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
SETTINGS="crates/ui-bridge/src/settings"
SET_MOD="${SETTINGS}/mod.rs"
SET_VIEW="${SETTINGS}/view.rs"
SET_INPUT="${SETTINGS}/input.rs"
SET_PROBE="${SETTINGS}/probe.rs"
UI_SETTINGS_TEST="crates/ui-bridge/tests/llm_set""tings_view.rs"

# 沿用 P4/P7 的 kw_fields（勿重复定义）
kw_fields() { awk -v s="pub struct $1 {" 'index($0,s)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    pub [a-z_]+:'; }
# settings 口径：结构体变体 `{ ... }` 必须计入（D-B4-P100）
kw_variants2() { awk -v e="pub enum $1 {" 'index($0,e)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    [A-Z][A-Za-z0-9]*[[:space:]]*([({]|,)'; }
new_targets_count() { grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//" | wc -w; }
# 18 项顶层类型 / 7 项顶层 pub fn（行首匹配；impl 方法缩进 4 空格不计）
set_items() { grep -rhE '^pub (struct|enum) ' "${SETTINGS}" | wc -l | tr -d ' '; }
set_fns()   { grep -rhE '^pub fn ' "${SETTINGS}" | wc -l | tr -d ' '; }
# settings 用例数：目标 18（全部以 settings_ 开头）
set_cases() { grep -c '^fn settings_' "$UI_SETTINGS_TEST"; }
# 凭据/存储/日志/网络/时钟一次性扫描：目标 0
set_danger() { grep -rnE 'std::fs|File::|keyring|card_store|Sqlite|outbox|println!|eprintln!|dbg!|tracing|log::|reqwest|hyper|TcpStream|std::net|Instant::now|SystemTime::now|thread::sleep|std::env|LLM_|keyword_card' "$SETTINGS" 2>/dev/null | wc -l; }
```

> **零依赖纪律（P8 硬约束）**：`crates/ui-bridge/Cargo.toml` 与 `Cargo.lock` **零 diff**；`settings/**` **不 `use meeting_copilot_application::*`、不 `use meeting_copilot_domain::*`**（只用 `std`/`core`/`serde` 与 `crate::settings::*`）。这是与 P7 的关键差异：settings 只消费注入的 `String`/布尔/毫值。
> **凭据纪律（P8 加严）**：Key 明文只存在于 `SecretInput` 内存态；**无** `as_str`/`len`/`prefix`/`suffix`/`last4` 访问器；`Debug` 恒 `***`；`MaskedKey::Masked` 无载荷；错误文案/测试输出/进度文档不含任何 Key 子串。
> **零转录 + 样本纪律**：`$SETTINGS/**`、`$UI_SETTINGS_TEST`、`b4-status.md` §19 内不得出现协议前缀/`sk-` 形态/`Authorization: Bearer`/模型名前缀/`request_id:`/`LLM_`；测试假 Key 统一 `test-key-` 前缀且 ≥16 字符（如 `test-key-000000000000`）；模型/端点夹具只用 `model-a`/`model-b`、`endpoint-a`/`endpoint-b` 这类短串（D-B4-P96/**X-15**）。
> **`--rule=source` 加严（X-14）**：`$SETTINGS/**` 进 `core_files()` 后整体扫描，源码内**禁止** `panic!(`/`todo!(`/`unimplemented!(`/`std::env`/`LLM_`/`Instant::now`/`SystemTime::now`/`thread::sleep`/`std::net`/`tokio::time`/`static mut`/`OnceLock`/`lazy_static`/`thread_local`；注释同样不得写这些字面量（用中文描述"网络符号/环境变量/系统时间"）。
> **X-03 提交纪律（硬约束）**：`require_core_faces()` 加入 `UI_SETTINGS_DIR` 后，目录缺失即令全部 `--rule` 退 **2**；**W-B4-72.1/72.2 必须在 65~71 落盘之后执行，并与 65~71 同批提交**；禁止先改脚本、禁止单独提交脚本。
> **X-05 口径**：`--rule=targets` 循环**仍 7 项**，不追加 `llm_settings_view`；`grep -c 'llm_settings_view' scripts/check-b4-gates.sh` 必须为 **0**（用拼接写法规避）。
> **X-11 口径**：P8 必须在 P7 **已提交**之后开工；`NEW_TARGETS` 23→24 的断言以 P7 收口值为基数。

### W-B4-65 settings 落位与最小可编译骨架（AC-B4-85/93）

> 前置：P7 已提交（X-11）。落点 `$SETTINGS`（**新目录**，4 文件）+ `lib.rs` +1 行；`Cargo.toml`/`Cargo.lock` **零改动**。

- [ ] **W-B4-65.1 `settings/mod.rs` 门面 + 3 常量** · `$SET_MOD`
  - 追溯：AC-B4-85 · spec §5.10.1-2、§6.24~6.28 · design §7.2.2、§7.3.1-D、D-B4-P75/**P76**
  - 改动内容：`//! 51 设置向导：凭据/模型/端点的确定性投影与意图产出（0 IO/0 网络/0 落盘）。` + `pub mod view;` + `pub mod input;` + `pub mod probe;` + 一个 `pub use` 块导出 **7 函数 + 18 类型** + 3 常量：
    ```rust
    pub const SECTION_ORDER: [&str; 4] = ["粘贴 Key", "选择模型", "服务地址", "测试并保存"];
    pub const MASK_TOKEN: &str = "***";
    pub const PERSIST_FALLBACK_NOTICE: &str = "已保存，但仅本次运行有效";
    ```
    `pub use` 逐字：`view::{apply_defaults, mask_api_key, project_settings_view, resolve_model_name, validate_settings_input, EndpointSelectionView, FieldIssue, MaskedKey, ModelSelectionView, SettingsDefaults, SettingsFacts, SettingsFormState, SettingsView}`；`input::{EndpointDraft, ModelDraft, PersistTarget, ProbeRequest, SecretInput, SettingsInput, SettingsIntent}`；`probe::{humanize_probe_failure, next_probe_state, ProbeEvent, ProbeFailureKind, ProbeOutcome}`
  - 禁止：`mod.rs` **不定义**类型/函数（只声明与再导出）；**不**再导出私有步骤/`SecretInput` 内部态/文案表；**不**加第 4 个 `pub mod`（源文件恰 4 个）；**不** `use crate::keyword_card`/`use crate::dto`
  - DoD：`test -d "$SETTINGS"` 退 0；`grep -c '^pub mod ' "$SET_MOD"`=**3**；`grep -c 'SECTION_ORDER' "$SET_MOD"`=1；`grep -c 'MASK_TOKEN' "$SET_MOD"`=1；`grep -c 'PERSIST_FALLBACK_NOTICE' "$SET_MOD"`=1；`SECTION_ORDER.len()`=4（用例断言）；`grep -cE '^pub (struct|enum|fn) ' "$SET_MOD"`=**0**
  - 证据：`grep -cE '^pub (struct|enum|fn) ' "$SET_MOD"` → `0`；`"$CARGO" build -p meeting-copilot-ui-bridge` 退 0（其余文件先占位亦可，见 65.2~65.4）

- [ ] **W-B4-65.2 `settings/view.rs` 12 类型 + 5 函数签名骨架** · `$SET_VIEW`
  - 追溯：AC-B4-85、AC-B4-93 · spec §6.24~6.28 · design §7.3.1、§7.3.4、§7.4.2、D-B4-P76/**P94**/**P98**/**P102（复用）**
  - 改动内容：**12 个顶层 `pub` 类型**（8/6/4 + 4 子类型）：
    - `SettingsView`（8 字段：`form_state`/`has_key`/`key_display`/`model_choice`/`endpoint_choice`/`inline_errors`/`can_test`/`can_save`）= `Clone + PartialEq + Serialize` + **手写 `Debug`**
    - `SettingsFacts`（6 字段：`has_stored_key`/`model_count`/`endpoint_count`/`now_ms`/`cooldown_until_ms`/`probe_in_flight`）= `Debug + Clone + Copy + PartialEq + Eq`（**不** `Serialize`）
    - `SettingsDefaults`（4 字段：`model_index`/`endpoint_index`/`timeout_ms`/`auto_save_on_verified`）= `Debug + Clone + Copy + PartialEq + Eq` + **手写 `Default` → `single_step()`**（D-B4-P107）
    - `MaskedKey`（2 变体 `Masked`（**无载荷**）/`None`）= `Clone + Copy + PartialEq + Eq` + **手写 `Debug`/`Serialize`**
    - `ModelSelectionView`（2 变体 `Preset{display_name: String}`/`Custom{name: String}`）= `Clone + PartialEq + Serialize` + **手写 `Debug`**
    - `EndpointSelectionView`（2 变体 `Option{display_name: String}`/`Unavailable`）= `Clone + PartialEq + Serialize` + **手写 `Debug`**
    - `FieldIssue`（6 变体，顺序 `ApiKeyMissing`/`ApiKeyTooShort`/`ApiKeyHasWhitespace`/`ModelMissing`/`ModelTooLong`/`EndpointUnavailable`）= `Debug + Clone + Copy + PartialEq + Eq + Ord + PartialOrd + Serialize` + `impl { pub const ALL: [Self; 6] }`
    - `SettingsFormState`（6 变体 `NotConfigured`/`ConfiguredUnverified`/`Testing`/`Verified`/`Failed{failure: ProbeFailureKind}`/`Cooling{remaining_ms: u64}`）= `Debug + Clone + Copy + PartialEq + Eq + Serialize`
    - 5 个顶层 `pub fn` 签名**逐字**：`apply_defaults(&SettingsFacts, &SettingsDefaults) -> SettingsInput`、`project_settings_view(&SettingsFacts, &SettingsInput, SettingsFormState, &[String], &[String]) -> SettingsView`、`validate_settings_input(&SettingsFacts, &SettingsInput, &[String], &[String]) -> Vec<FieldIssue>`、`mask_api_key(&SecretInput) -> MaskedKey`、`resolve_model_name(&ModelDraft, &[String]) -> ModelSelectionView`
    - 本步允许最小合法实现（`SettingsView` 可用 8 个占位值组装；**不用** `todo!`/`unimplemented!`/`panic!`）
  - 禁止：**不得** `derive(Debug)` 到 `SettingsView`/`MaskedKey`/`ModelSelectionView`/`EndpointSelectionView`；`SettingsView` **不得**加第 9 字段（`notice`/`sections`/`persist_outcome` 都不允许）；`SettingsFacts` 不得加 `form_state`/`api_key`/`host`/`url`；不得定义第 13 个顶层类型（如 `SettingsContext`）
  - DoD：`kw_fields SettingsView`=8、`SettingsFacts`=6、`SettingsDefaults`=4；`kw_variants2 SettingsFormState`=6、`FieldIssue`=6、`MaskedKey`=2、`ModelSelectionView`=2、`EndpointSelectionView`=2；`grep -c 'impl core::fmt::Debug for ' "$SET_VIEW"` ≥2（`SettingsView` + `MaskedKey`）；`SettingsDefaults::default().timeout_ms`=**6_000** 且 `auto_save_on_verified=true`；`"$CARGO" build -p meeting-copilot-ui-bridge` 退 0
  - 证据：
    ```bash
    set -e
    kw_fields SettingsView "$SET_VIEW"; kw_fields SettingsFacts "$SET_VIEW"; kw_fields SettingsDefaults "$SET_VIEW"
    kw_variants2 SettingsFormState "$SET_VIEW"; kw_variants2 FieldIssue "$SET_VIEW"
    "$CARGO" build -p meeting-copilot-ui-bridge
    echo "exit=$?"
    ```

- [ ] **W-B4-65.3 `settings/input.rs` 6 类型 + 意图方法骨架（凭据零回显）** · `$SET_INPUT`
  - 追溯：AC-B4-85、AC-B4-87、AC-B4-88 · spec §6.26 · design §7.3.1、§7.5.1、§7.5.3、§7.8.1、D-B4-P77/**P87**/**P88**/**P94**
  - 改动内容：**6 个顶层 `pub` 类型**：
    - `SecretInput`（私有 2 态 `Draft(Option<String>)`/`Stored`）= `Clone` + **手写 `PartialEq`** + **手写 `Debug`（恒 `***`）**；**不** `Serialize`/`Deserialize`/`Display`；`impl` 方法（4 空格缩进，不计 7）：`empty()`/`stored()`/`from_draft(&str)`（内部 `trim`，空 → `Draft(None)`）/`is_provided()`/`is_stored()` + **`pub(crate) fn draft_issue(&self) -> KeyDraftIssue`**（D-B4-P105）
    - `pub(crate) enum KeyDraftIssue { Empty, TooShort, HasWhitespace, Ok }`（`pub(crate)`，**不**计入 18）
    - `SettingsInput`（5 字段：`api_key_draft`/`model_draft`/`endpoint_draft`/`submitted`/`clear_confirmed`）= `Clone + PartialEq` + **手写 `Debug`**；**不** `Serialize`
    - `ModelDraft`（3 变体 `PresetIndex(usize)`/`CustomName(String)`/`None`）= `Clone + PartialEq` + **手写 `Debug`**（不输出 name）
    - `EndpointDraft`（2 变体 `Index(usize)`/`None`）= `Clone + PartialEq` + **手写 `Debug`**
    - `SettingsIntent`（3 变体 `Persist{target: PersistTarget}`/`Clear`/`Probe{request: ProbeRequest}`）= `Clone` + **手写 `Debug`**；**不** `Serialize`/`Deserialize`
    - `PersistTarget`（2 变体 `Memory`/`OsSecureStore`）= `Debug + Clone + Copy + PartialEq + Eq` + `impl { pub const ALL: [Self; 2] }`
    - `ProbeRequest`（4 字段：`credential: SecretInput`/`model: ModelDraft`/`endpoint_index: Option<usize>`/`timeout_ms: u64`）= `Clone` + **手写 `Debug`**；**不** `Serialize`
    - `impl SettingsInput`：`pub fn persist_intent(&self, target: PersistTarget) -> Option<SettingsIntent>`（骨架恒 `None`）、`pub fn clear_intent(&self) -> Option<SettingsIntent>`（骨架恒 `None`）
  - 禁止：**不得** `Serialize`/`Deserialize` 出现在本文件（`grep -c`=0）；**不得** `Display`；**不得** `as_str`/`len`/`chars`/`prefix`/`suffix`/`last4` 访问器；`SecretInput` 的 `Debug`/嵌套 `Debug` 输出恒 `***`；不得把 `KeyDraftIssue` 写成 `pub`（会破坏 18 计数）
  - DoD：`kw_fields SettingsInput`=5、`ProbeRequest`=4；`kw_variants2 ModelDraft`=3、`EndpointDraft`=2、`SettingsIntent`=3、`PersistTarget`=2；`grep -c 'Serialize\|Deserialize' "$SET_INPUT"`=**0**；`grep -cE 'fn as_str|fn len|fn prefix|fn suffix|fn last4' "$SET_INPUT"`=0；`format!("{:?}", SecretInput::from_draft("test-key-000000000000"))` == `"***"`；`"$CARGO" build -p meeting-copilot-ui-bridge` 退 0
  - 证据：`grep -c 'Serialize\|Deserialize' "$SET_INPUT"` → `0`；`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::input::tests::secret_debug_is_masked` 退 0

- [ ] **W-B4-65.4 `settings/probe.rs` 3 类型 + 文案表 + 2 函数签名骨架** · `$SET_PROBE`
  - 追溯：AC-B4-85、AC-B4-91、AC-B4-92 · spec §6.27 · design §7.3.3、§7.3.4、§7.6.3、§7.7.1、D-B4-P80/**P100**
  - 改动内容：**3 个顶层 `pub` 类型**：
    - `ProbeEvent`（5 变体 `UserTest`/`UserCancel`/`ProbeSucceeded`/`ProbeFailed{failure: ProbeFailureKind}`/`ClockTick{now_ms: u64}`）= `Debug + Clone + Copy + PartialEq + Eq`
    - `ProbeFailureKind`（8 变体，顺序 `MissingKey`/`KeyRejected`/`EndpointUnreachable`/`Timeout`/`RateLimited`/`QuotaExceeded`/`ServerError`/`Unknown`）= `Debug + Clone + Copy + PartialEq + Eq + Ord + Serialize` + `impl { pub const ALL: [Self; 8] }`
    - `ProbeOutcome`（2 变体 `Succeeded{elapsed_ms: u64}`/`Failed{failure: ProbeFailureKind}`）= `Debug + Clone + Copy + PartialEq + Eq` + `impl { pub fn to_event(self) -> ProbeEvent }`
    - 私有 `const FAILURE_MESSAGES: [&str; 8]`（8 条固定中文文案，逐字见 design §7.3.4 表）
    - 2 个顶层 `pub fn` 签名**逐字**：`next_probe_state(&SettingsFacts, &SettingsInput, SettingsFormState, ProbeEvent, &[String], &[String], &SettingsDefaults) -> (SettingsFormState, Option<SettingsIntent>)`、`humanize_probe_failure(ProbeFailureKind) -> &'static str`
  - 禁止：**不得** import 网关/网络/计时符号；**不得**新增第 4 个顶层类型（如 `ProbeClient`/`CoolingTimer`）；文案不得含协议前缀/Key 形态/模型名/域名/`request_id`/内部错误码
  - DoD：`kw_variants2 ProbeEvent`=5、`ProbeFailureKind`=8、`ProbeOutcome`=2；`grep -c 'FAILURE_MESSAGES' "$SET_PROBE"` ≥1；`grep -cE 'reqwest|TcpStream|std::net|Instant|SystemTime|thread::sleep' "$SET_PROBE"`=0；`"$CARGO" build -p meeting-copilot-ui-bridge` 退 0
  - 证据：`kw_variants2 ProbeFailureKind "$SET_PROBE"` → `8`；`"$CARGO" build -p meeting-copilot-ui-bridge; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-65.5 `lib.rs` +1 行 + 18/7 计数复核 + 依赖零 diff** · `crates/ui-bridge/src/lib.rs`、仓库根
  - 追溯：AC-B4-85、AC-B4-94 · design §7.2.2、§7.4.4-2、D-B4-P75/**P84**
  - 改动内容：`crates/ui-bridge/src/lib.rs` 在 `pub mod keyword_card;` 之后插入 `pub mod settings;`（**+1 / -0**，**不加** `pub use settings::*`）；随后复核 `set_items`=18、`set_fns`=7
  - 禁止：**不得**改 `Cargo.toml`/`Cargo.lock`（`git diff --quiet` 必须退 0）；不得加顶层 `pub use`（会干扰 18/7 计数）；不得改 `dto.rs`
  - DoD：`grep -c 'pub mod settings;' crates/ui-bridge/src/lib.rs`=1；`git diff --numstat HEAD -- crates/ui-bridge/src/lib.rs` 为 `1 0`；`git diff --quiet HEAD -- crates/ui-bridge/Cargo.toml Cargo.lock` 退 0；`set_items`=**18**；`set_fns`=**7**；`grep -cE '^pub fn ' "$SETTINGS"/*.rs` 逐文件分布为 `view.rs`=5、`probe.rs`=2（`input.rs`=0）；`"$CARGO" build -p meeting-copilot-ui-bridge` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(set_items)" = "18" ] && [ "$(set_fns)" = "7" ]
    [ "$(grep -c '^pub fn ' "$SET_VIEW")" = "5" ]
    [ "$(grep -c '^pub fn ' "$SET_PROBE")" = "2" ]
    git diff --quiet HEAD -- crates/ui-bridge/Cargo.toml Cargo.lock
    git diff --numstat HEAD -- crates/ui-bridge/src/lib.rs
    echo "exit=$?"
    ```
  - 证据产物：18/7/5/2 与 `+1/-0` 写入 `b4-status.md` §19.1

### W-B4-66 Key 校验与固定掩码（AC-B4-86/87）

> 前置：W-B4-65.3（`SecretInput` 与 `draft_issue`）。**明文不出内存**：本任务所有函数只返回枚举/布尔/掩码态。

- [ ] **W-B4-66.1 `SecretInput` 2 私有态与公开面白名单** · `$SET_INPUT`
  - 追溯：AC-B4-87 · spec §6.26-1 · design §7.5.1、D-B4-P77/**P88**/**P105**
  - 改动内容：完成 `SecretInput` 实现：`from_draft(raw)` **只 `trim`**（保留内部空白，交由 `draft_issue` 判 `HasWhitespace`）；`is_provided()`：`Draft(Some(_)) | Stored` → `true`，`Draft(None)` → `false`；`is_stored()`：仅 `Stored` → `true`；手写 `PartialEq` 比较内部态（`Draft(Some(x))` 逐字节比较，**不在 `Debug`/`Display` 输出**）；手写 `Debug` 恒 `***`；`draft_issue()`：`Draft(None)` → `Empty`；`Stored` → `Ok`（已存 Key 视为有值）；`Some(s)` 内部含 `char::is_whitespace` → `HasWhitespace`；`s.chars().count() < 16` → `TooShort`；否则 `Ok`
  - 禁止：不得提供 `as_str`/`len`/`chars`/`prefix`/`suffix`/`last4`/`bytes`；不得 `Serialize`/`Deserialize`/`Display`；`from_draft` 不得把内部空白裁掉（否则 `ApiKeyHasWhitespace` 永不可达）；不得 `panic!`
  - DoD：`from_draft("  ")` `is_provided()`=false；`from_draft(" test-key-000000000000 ")` `is_provided()`=true 且 `draft_issue()==Ok`；`from_draft("short")` → `TooShort`；`from_draft("test key 000000000000")` → `HasWhitespace`；`stored()` → `is_stored()=true`、`draft_issue()=Ok`；`format!("{:?}", ...)` == `"***"`；`grep -cE 'fn (as_str|len|prefix|suffix|last4)' "$SET_INPUT"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::input::tests::secret_issue_is_classified_without_leaking` 退 0

- [ ] **W-B4-66.2 `MaskedKey` 无载荷 + 手写 `Serialize`/`Debug` + `mask_api_key`** · `$SET_VIEW`
  - 追溯：AC-B4-87 · spec §5.10.1-4、§6.24-3 · design §7.5.2、D-B4-P77/**P94**
  - 改动内容：`MaskedKey`：`Masked` **无载荷**；手写 `Serialize`：`Masked` → `serializer.serialize_str(MASK_TOKEN)`、`None` → `serializer.serialize_none()`；手写 `Debug`：`Masked` → `"***"`、`None` → `"None"`。`mask_api_key(input: &SecretInput) -> MaskedKey`：`is_provided()` → `Masked`，否则 `None`（**不看长度/字符集/前后缀**）
  - 禁止：**不得**给 `Masked` 加载荷（`last4`/`len`/`prefix`）；不得让掩码长度随输入变化；不得用 `format!("{:.4}", ...)` 之类截断（等价泄露）；`MASK_TOKEN` 只从 `crate::settings::MASK_TOKEN` 引用，**不得**在 `view.rs` 另写 `"***"` 字面量第二处
  - DoD：1/16/64/256 长度输入 → `serde_json::to_vec(&mask_api_key(&s))` 均逐字节等于 `"\"***\""`；`format!("{:?}", MaskedKey::Masked)` == `"Masked"`（不含输入）；`serde_json::to_value(MaskedKey::None)` == `Value::Null`；`grep -c '"\\*\\*\\*"' "$SET_VIEW"`=**0**（只用常量）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::view::tests::mask_is_constant_regardless_of_length` 退 0

- [ ] **W-B4-66.3 `validate_settings_input` 6 项 + 稳定排序去重 + `submitted=false` 恒空** · `$SET_VIEW`
  - 追溯：AC-B4-86 · spec §5.10.1-3、§6.24-6 · design §7.3.4、§7.6.4、D-B4-P93/**P98**/**P101**
  - 改动内容：`validate_settings_input(facts, input, models, endpoints) -> Vec<FieldIssue>`：`if !input.submitted { return Vec::new(); }`；否则按下述顺序**收集**并 `BTreeSet` 去重后按声明序输出：
    - `input.api_key_draft.draft_issue()`：`Empty` → `ApiKeyMissing`；`TooShort` → `ApiKeyTooShort`；`HasWhitespace` → `ApiKeyHasWhitespace`；`Ok` → 无
    - 模型：`resolve_model_name(&input.model_draft, models)` 为 `Custom{name}` 时：`name.trim().is_empty()` → `ModelMissing`；`name.chars().count() > 128` → `ModelTooLong`；含 `[A-Za-z0-9._:-]` 之外的字符（含空格/中文/URL 标点）→ `ModelMissing`；`Preset{..}` → 无
    - 端点：`endpoints.is_empty()` 或 `EndpointDraft::Index(i)` 且 `i >= endpoints.len()` 或 `None` → `EndpointUnavailable`
  - 禁止：**不得**校验 Key 前缀/真伪/联网预检；**不得**在 `submitted=false` 时产出任何错误（含 `ModelMissing`/`EndpointUnavailable`）；不得用 `HashSet`（用 `BTreeSet`）；不得对同一问题产出两个不同码（`ModelTooLong` 与 `ModelMissing` 互斥：先判长度再判字符集，超长直接 `ModelTooLong` 并 `continue`）
  - DoD：`submitted=false` → `Vec::new()`（即使清单为空/Key 为空）；`true` + 空 Key → 含 `ApiKeyMissing`；15 字符 → 含 `ApiKeyTooShort`；16 字符 → **不含** `ApiKeyTooShort`；内部空白 → 含 `ApiKeyHasWhitespace`；129 字符模型名 → 含 `ModelTooLong` 且**不含** `ModelMissing`；含空格模型名 → 含 `ModelMissing`；`endpoints=[]` → 含 `EndpointUnavailable`；重复问题只出现 1 次；输出顺序为声明序（6 项全命中时 `[Missing, TooShort?...]` 按 `Ord` 升序）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::view::tests::key_issues_are_lenient_and_deduped` 退 0；`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::view::tests::errors_only_after_submit` 退 0

### W-B4-67 一步式默认与视图投影（AC-B4-85/89/90）

> 前置：W-B4-66。**S0 归一化必须最先**（`has_key=false` 时不出现 `can_test=true`）。

- [ ] **W-B4-67.1 `SettingsDefaults::single_step()` + `apply_defaults`** · `$SET_VIEW`
  - 追溯：AC-B4-85 · spec §6.27-6、§6.28-1 · design §7.2.3（S0）、§7.3.2、§7.4.3、D-B4-P78/**P107**
  - 改动内容：`SettingsDefaults::single_step()` 返回 `{ model_index: 0, endpoint_index: 0, timeout_ms: 6_000, auto_save_on_verified: true }`；`impl Default` 转调之；`apply_defaults(facts, defaults) -> SettingsInput`：`api_key_draft = if facts.has_stored_key { SecretInput::stored() } else { SecretInput::empty() }`；`model_draft = if facts.model_count > defaults.model_index { ModelDraft::PresetIndex(defaults.model_index) } else { ModelDraft::None }`；`endpoint_draft = if facts.endpoint_count > defaults.endpoint_index { EndpointDraft::Index(defaults.endpoint_index) } else { EndpointDraft::None }`；`submitted=false`；`clear_confirmed=false`
  - 禁止：**不得** `derive(Default)`；不得在清单为空时制造越界下标（返回 `None`）；不得读系统时间/环境变量/文件；不得设置 `submitted=true`
  - DoD：`SettingsDefaults::default()` 的 4 项 == `single_step()`；`apply_defaults` 后 `model_draft==PresetIndex(0)`（`model_count>0` 时）否则 `None`；`endpoint_draft` 同理；`submitted=false`、`clear_confirmed=false`；`has_stored_key=true` → `api_key_draft.is_stored()==true`
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::view::tests::defaults_are_single_step` 退 0

- [ ] **W-B4-67.2 `resolve_model_name`（2 态封闭 + 越界 fail-closed）** · `$SET_VIEW`
  - 追溯：AC-B4-89 · spec §6.24-4 · design §7.6.2、D-B4-P97/**P99**
  - 改动内容：`resolve_model_name(draft, models) -> ModelSelectionView`：`PresetIndex(i)` 且 `i < models.len()` → `Preset { display_name: models[i].clone() }`（**逐字节**等于清单项）；`PresetIndex(i >= len)` → `Custom { name: String::new() }`；`CustomName(n)` → `Custom { name: n.clone() }`；`None` → `Custom { name: String::new() }`
  - 禁止：不得把越界回退到"第一个"；不得修改清单（`models` 只读）；不得在源码内联模型名（`grep '(deepseek|claude|gpt)-[A-Za-z0-9]'`=0）
  - DoD：`PresetIndex(0)` + `["model-a"]` → `Preset{display_name=="model-a"}`（逐字节）；`PresetIndex(5)` → `Custom{name==""}`；`CustomName("m-1")` → `Custom{name=="m-1"}`；`None` → `Custom{""}`；`kw_variants2 ModelSelectionView`=2；模型名正则命中 0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::view::tests::model_is_two_state_and_bounds_closed` 退 0

- [ ] **W-B4-67.3 `project_settings_view` S0~S10（归一化优先级 + `can_test`/`can_save`）** · `$SET_VIEW`
  - 追溯：AC-B4-85、AC-B4-90 · spec §6.24-7/8、§6.28-1~5 · design §7.2.3（S0~S10 表）、**D-B4-P89/P97/P99/P101**
  - 改动内容：按 design §7.2.3 **严格顺序**实现：
    1. `has_key = input.api_key_draft.is_provided() || facts.has_stored_key`
    2. `form_state`：`!has_key` → `NotConfigured`（**P0 最高**）；否则 `facts.probe_in_flight` → `Testing`（**P1**）；否则 `cooldown_until_ms > facts.now_ms` → `Cooling { remaining_ms: cooldown_until_ms.saturating_sub(facts.now_ms) }`（**P2**）；否则采用传入 `state`，若 `state==NotConfigured && has_key` → `ConfiguredUnverified`（**P3**）
    3. `key_display = mask_api_key(&input.api_key_draft)`（`!has_key` → `None`）
    4. `model_choice = resolve_model_name(&input.model_draft, models)`；`endpoint_choice`：`endpoints.is_empty()` 或 `Index(i>=len)` 或 `None` → `Unavailable`，否则 `Option { display_name: endpoints[i].clone() }`
    5. `inline_errors = validate_settings_input(...)`（`submitted=false` → 空）
    6. 模型有效 = `model_choice` 非空名 且 无 `ModelMissing`/`ModelTooLong`；端点可用 = `endpoint_choice != Unavailable`
    7. `can_test = has_key && !matches!(form_state, Testing|Cooling) && 模型有效 && 端点可用`；`can_save = has_key && 模型有效 && 端点可用`
  - 禁止：**不得**改变 S0~S10 顺序（尤其 S0 必须最先、S1 早于 S4、S5/S6 早于 S7/S8）；**不得**要求测试通过才 `can_save`；**不得**读系统时间（只用 `facts.now_ms`）；**不得**把 `Setup` 的 `facts.*_count` 与清单长度不一致时按 `facts` 走（**以 `models.len()`/`endpoints.len()` 为准**，fail-closed）
  - DoD：仅填 Key（清单各 1 项）→ `can_test=true`、`can_save=true`、`form_state=ConfiguredUnverified`；`has_key=false` → `form_state=NotConfigured`、`key_display=None`、`can_test=false`、`can_save=false`；`probe_in_flight=true` + `has_key=true` → `Testing` 且 `can_test=false`；`cooldown_until_ms > now_ms` → `Cooling{remaining>0}` 且 `can_test=false`；`endpoint_count=0` → `endpoint_choice=Unavailable`；仅填 Key → `can_test=true`（AC-85 的核心判据）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::view::tests::projection_normalization_priority` 退 0；`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::view::tests::only_key_enables_test` 退 0

### W-B4-68 保存/清除意图与降级口径（AC-B4-88/93）

> 前置：W-B4-66.1、W-B4-67。**只产意图，不执行 IO**。

- [ ] **W-B4-68.1 `SettingsInput::persist_intent` / `clear_intent`（唯一意图入口）** · `$SET_INPUT`
  - 追溯：AC-B4-88 · spec §5.10.1-5、§6.26-4 · design §7.8.1、§7.8.2、D-B4-P82/**P87**
  - 改动内容：`persist_intent(&self, target) -> Option<SettingsIntent>`：`api_key_draft.is_provided()` → `Some(SettingsIntent::Persist { target })`，否则 `None`；`clear_intent(&self) -> Option<SettingsIntent>`：`clear_confirmed && api_key_draft.is_provided()` → `Some(SettingsIntent::Clear)`，否则 `None`（**未确认 → `None` 且不改变任何状态**）
  - 禁止：不得在意图方法内做 IO/改 `self`（`&self`）；不得把 `SettingsIntent` 变成 `Serialize`；不得新增第 3 个意图方法（`probe_intent` 由 `next_probe_state` 产出，`impl` 内已属 2 个）
  - DoD：`is_provided()=true` + `persist_intent(Memory)` → `Some(Persist{Memory})`；`persist_intent(OsSecureStore)` → `Some(Persist{OsSecureStore})`；`is_provided()=false` → `None`（任意 target）；`clear_confirmed=false` → `clear_intent()==None`；`clear_confirmed=true` + 有 Key → `Some(Clear)`；`kw_variants2 SettingsIntent`=3；`kw_variants2 PersistTarget`=2
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::input::tests::intents_require_key_and_confirmation` 退 0

- [ ] **W-B4-68.2 不落盘/不日志/不序列化证明（四类证据）** · `$SETTINGS`、仓库根
  - 追溯：AC-B4-88、AC-B4-93 · spec §5.10.1-5/10 · design §7.8.4（四类证据）、D-B4-P94/**P95**
  - 改动内容：**零代码改动**（只建断言口径）。四类证据：①无文件/密钥链/存储；②无日志/打印；③意图不可序列化；④全仓无明文 Key 形态
  - 禁止：不得以注释写 `std::fs`/`keyring`/`println!` 等字面量（`--rule=source` 扫注释）；不得在测试里写真实 Key 形态
  - DoD：`grep -rnE 'std::fs|File::|keyring|card_store|Sqlite|outbox' "$SETTINGS"`=0；`grep -rnE 'println!|eprintln!|dbg!|tracing|log::' "$SETTINGS"`=0；`grep -c 'Serialize\|Deserialize' "$SET_INPUT"`<｜begin▁of▁sentence｜>=0；`git grep -nE 'sk-[A-Za-z0-9]{8,}' -- crates`=0
  - 证据：
    ```bash
    set -e
    [ "$(grep -rnE 'std::fs|File::|keyring|card_store|Sqlite|outbox' "$SETTINGS" | wc -l | tr -d ' ')" = "0" ]
    [ "$(grep -rnE 'println!|eprintln!|dbg!|tracing|log::' "$SETTINGS" | wc -l | tr -d ' ')" = "0" ]
    [ "$(grep -c 'Serialize\|Deserialize' "$SET_INPUT")" = "0" ]
    [ "$(git grep -nE 'sk-[A-Za-z0-9]{8,}' -- crates | wc -l | tr -d ' ')" = "0" ]
    echo "exit=$?"
    ```

- [ ] **W-B4-68.3 X-10 降级口径落地（`PERSIST_FALLBACK_NOTICE` + `Memory` 意图）** · `$SET_MOD`、`$SET_INPUT`、`documentation`
  - 追溯：AC-B4-88、AC-B4-92 · spec §5.10.3-8 · design §7.8.3、**D-B4-P92 / X-10（已接受）**
  - 改动内容：`PERSIST_FALLBACK_NOTICE` 常量已在 65.1 落位（逐字 `"已保存，但仅本次运行有效"`）；本步只做**口径固化**：`impl SettingsInput { pub fn persist_fallback_intent(&self) -> Option<SettingsIntent> }`？→ **不允许**（会新增第 8 个顶层函数以外的入口，且 `impl` 方法已够用）。**正确做法**：不新增方法，降级由 UI 调用既有 `persist_intent(PersistTarget::Memory)`；在 `mod.rs` 常量旁写一行中文注释说明"安全存储拒绝后由 UI 以 Memory 意图降级并渲染本常量"。**不新增** `PersistOutcome`/`notice` 字段
  - 禁止：**不得**新增第 19 个顶层类型（`PersistOutcome`/`PersistResult`）；**不得**给 `SettingsView` 加第 9 字段；**不得**在 51 侧读/写安全存储；**不得**静默失败
  - DoD：`persist_intent(Memory)` 返回 `Some(Persist{Memory})`（AC-92 降级判据）；`grep -c '仅本次运行有效' "$SET_MOD"`=1；`kw_fields SettingsView`=**8**（未扩）；`set_items`=**18**（未增第 19）；`grep -cE 'PersistOutcome|notice' "$SETTINGS"/*.rs`=0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c '仅本次运行有效' "$SET_MOD")" = "1" ]
    [ "$(kw_fields SettingsView "$SET_VIEW")" = "8" ]
    [ "$(set_items)" = "18" ]
    ! grep -rqE 'PersistOutcome|notice' crates/ui-bridge/src/settings
    echo "exit=$?"
    ```

### W-B4-69 连通性测试状态机（AC-B4-91）

> 前置：W-B4-67（`SettingsFormState`/`SettingsDefaults`）与 65.3（`SettingsIntent`/`ProbeRequest`）。**单飞、可取消、无隐式重试、冷却只读**。

- [ ] **W-B4-69.1 `next_probe_state` 6×5 转移表** · `$SET_PROBE`
  - 追溯：AC-B4-91 · spec §5.10.1-8、§6.27-1/2 · design §7.7.1（转移表）、§7.7.2、D-B4-P80/**P91**
  - 改动内容：逐字实现 design §7.7.1 的 6×5 表；三条不可违反：①`facts.probe_in_flight==true` 时 `UserTest` → `(state, None)`（不改变状态、不产意图）；②`Testing` + `UserCancel` → `(ConfiguredUnverified, None)`；③`ProbeSucceeded`/`ProbeFailed` 是权威（回执优先于本地状态）；`UserTest` 可测判定 = `has_key && 非 Testing/Cooling && 模型有效 && 端点可用`（复用 67.3 口径：`has_key` 用 `input.api_key_draft.is_provided() || facts.has_stored_key`），产出 `Some(SettingsIntent::Probe { ProbeRequest { credential: input.api_key_draft.clone(), model: input.model_draft.clone(), endpoint_index: 按 endpoint_draft 解析（None/越界 → None，则不产 Probe）, timeout_ms: defaults.timeout_ms } })`
  - 禁止：不得在 `probe_in_flight=true` 时仍产意图；不得排队/合并/自动重试；不得 `thread::sleep`/`Instant::now`；不得在 `NotConfigured` 下响应 `UserTest` 产出 Probe（`has_key=false` → `(state, None)`）；`Probe` 的 `endpoint_index` 必须是 `Some(i)` 且 `i < endpoints.len()`，否则 `None`
  - DoD：`probe_in_flight=true` + `UserTest` → `(state, None)`（状态逐字节不变）；`ConfiguredUnverified` + `UserTest` → `Testing` + `Some(Probe{..})`；`Testing` + `UserCancel` → `ConfiguredUnverified` + `None`；`Verified` + `UserTest` → `Testing` + `Probe`；`Cooling{r}` + `UserTest` → 不变 + `None`；`NotConfigured` + `UserTest` → 不变 + `None`；100 次同输入转移逐字节一致
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::probe::tests::transition_table_is_single_flight` 退 0

- [ ] **W-B4-69.2 `ProbeOutcome::to_event` + 成功后自动保存意图** · `$SET_PROBE`
  - 追溯：AC-B4-91 · spec §6.27-5 · design §7.3.3（XOR + `to_event`）、§7.7.4、**D-B4-P100**
  - 改动内容：`impl ProbeOutcome { pub fn to_event(self) -> ProbeEvent }`：`Succeeded{..}` → `ProbeSucceeded`；`Failed{failure}` → `ProbeFailed{failure}`（**唯一映射，不丢 `elapsed_ms` 之外的任何语义**，`elapsed_ms` 不进入事件）；`next_probe_state` 在 `ProbeSucceeded` 且 `defaults.auto_save_on_verified==true` 时返回 `(Verified, Some(SettingsIntent::Persist { target: PersistTarget::OsSecureStore }))`；`false` 时 `(Verified, None)`
  - 禁止：不得让 `to_event` 产生 `UserTest`/`UserCancel`/`ClockTick`；不得在 `auto_save_on_verified=false` 时仍产 `Persist`；不得把 `elapsed_ms` 写进意图/Debug 之外的任何载体
  - DoD：`to_event` 两分支逐字；`ProbeSucceeded` + `auto_save=true` → `(Verified, Some(Persist{OsSecureStore}))`；`auto_save=false` → `(Verified, None)`；`ProbeFailed` + 无冷却 → `(Failed{f}, None)`；`kw_variants2 ProbeOutcome`=2
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::probe::tests::autosave_on_verified` 退 0

- [ ] **W-B4-69.3 `ClockTick` 冷却重算与 `remaining_ms`（saturating）** · `$SET_PROBE`
  - 追溯：AC-B4-91 · spec §6.28-3 · design §7.7.3、**D-B4-P90/P91**
  - 改动内容：`ClockTick{now_ms}` 用**事件自带** `now_ms`：`remaining = facts.cooldown_until_ms.unwrap_or(0).saturating_sub(now_ms)`；`remaining == 0` → `ConfiguredUnverified`（不自动重试、不产意图）；否则 `Cooling { remaining_ms: remaining }`；其余事件一律用 `facts.now_ms` 判定冷却；`ProbeFailed{f}` 在 `cooldown_until_ms > facts.now_ms` 时 → `Cooling { remaining_ms: cooldown_until_ms - facts.now_ms }`，否则 `Failed{f}`；`Cooling` 期间任何 `UserTest` → 不变 + `None`
  - 禁止：不得读 `Instant::now`/`SystemTime::now`；不得建计时器/熔断计数；不得 `unwrap`（用 `unwrap_or(0)`/`saturating_sub`）；不得在冷却期自动重试
  - DoD：`Cooling{r}` + `ClockTick{now = until}` → `ConfiguredUnverified` + `None`；`ClockTick{now < until}` → `Cooling{until-now}`；`ProbeFailed` + `until > facts.now_ms` → `Cooling{until-facts.now_ms}`；`cooldown_until_ms=None` + `ProbeFailed` → `Failed{f}`；`grep -cE 'Instant|SystemTime|thread::sleep' "$SET_PROBE"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::probe::tests::cooldown_is_read_only_mirror` 退 0

### W-B4-70 失败人话化与可恢复（AC-B4-92）

> 前置：W-B4-69。

- [ ] **W-B4-70.1 8 条固定文案 + `humanize_probe_failure`** · `$SET_PROBE`
  - 追溯：AC-B4-92 · spec §5.10.1-9、§6.27-3/6 · design §7.3.4（文案表）、§7.6.3、D-B4-P81/**P96**
  - 改动内容：`FAILURE_MESSAGES: [&str; 8]` 按下标与 `ProbeFailureKind::ALL` 一一对应，逐字为：`"请先粘贴 API Key"`/`"API Key 无效或已过期，请重新复制粘贴"`/`"连不上服务器，请检查网络或代理"`/`"服务器响应太慢，已超时，请稍后重试"`/`"请求太频繁，请稍后再试"`/`"账户额度不足，请检查余额"`/`"服务器暂时不可用，请稍后重试"`/`"连接失败，请检查网络后重试"`；`humanize_probe_failure(kind) -> &'static str` = `FAILURE_MESSAGES[Self::index(kind)]`（私有 `fn index(kind) -> usize` 用 `match` 常量表，不 `as usize` 依赖声明序不确定性）
  - 禁止：文案**不得**含 `test-key`/`sk-`/协议前缀/模型名/域名/`request_id`/内部错误码数字；**不得**提示"把 Key 发给我们/上传/邮箱/截图"；**不得**返回 `String`（必须 `&'static str`）；不得缺项/重复
  - DoD：`ProbeFailureKind::ALL.len()`=8；遍历 `ALL` 得到的 8 条文案**互不相同**；`humanize_probe_failure(MissingKey)=="请先粘贴 API Key"`（逐字）；文案拼接串不含 `test-key`/`http`/`sk-`；`grep -cE '发送|上传|邮箱|截图' "$SET_PROBE"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::probe::tests::failure_messages_are_unique` 退 0

- [ ] **W-B4-70.2 失败后保留输入 + 非冷却期 `can_test` 恢复** · `$SET_VIEW`
  - 追溯：AC-B4-92 · spec §5.10.1-9 · design §7.6.4-5、§7.7.4、D-B4-P81
  - 改动内容：`project_settings_view` 在 `state=Failed{f}` 时：`has_key`/`model_choice`/`endpoint_choice` **不变**（投影只读输入，天然满足）；`can_test` 按 67.3 公式在非 `Testing`/`Cooling` 时为 `true`；`Cooling{remaining_ms}` 时 `can_test=false` 且 `remaining_ms` 原样暴露（UI 取 `remaining_ms / 1000` 向上取整，**51 只给毫值**）
  - 禁止：不得在失败态清空草稿/回首屏；不得把 `Failed` 当"凭据无效"的唯一解释（`Failed{f}` 保留 8 类语义）；不得在投影里读秒表
  - DoD：`Failed{KeyRejected}` → `has_key=true`、`model_choice`/`endpoint_choice` 与失败前逐项相等、`can_test=true`（非冷却）；`Cooling{5_000}` → `can_test=false` 且 `form_state` 携带 5_000；同输入投影 100 次逐字节一致（`serde_json::to_vec`）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --lib settings::view::tests::failure_keeps_input_and_retry` 退 0

### W-B4-71 `llm_settings_view` target 与 18 个用例（AC-B4-85~93）

> 前置：W-B4-65~70 全部编译通过。新增 `$UI_SETTINGS_TEST`（**第 11 个 target**）；**不** `mod common`；`serde_json` 为 ui-bridge 的 dev-dep（已存在，**非新增依赖**）。

- [ ] **W-B4-71.1 文件头 + 本地夹具（`test-key-` 前缀样本）** · `$UI_SETTINGS_TEST`
  - 追溯：AC-B4-85、AC-B4-94 · design §7.1.3-E、§7.9.1（样本纪律）、**X-15**
  - 改动内容：文件头 `//! 51 设置向导（AC-B4-85~94，18 用例）。`；`use meeting_copilot_ui_bridge::settings::{...};`（只经门面）；夹具：
    - `const MODELS: [&str; 2] = ["model-a", "model-b"];`、`const ENDPOINTS: [&str; 2] = ["endpoint-a", "endpoint-b"];`（**不得**写真实模型名/域名）
    - `fn key_draft(raw: &str) -> SecretInput { SecretInput::from_draft(raw) }`、`fn input_with(key: &str, submitted: bool) -> SettingsInput`（`model_draft=PresetIndex(0)`、`endpoint_draft=Index(0)`）
    - `fn facts(count_models: usize, count_endpoints: usize, now: u64, cooldown: Option<u64>, inflight: bool) -> SettingsFacts`
    - `fn defaults() -> SettingsDefaults { SettingsDefaults::single_step() }`
    - `const GOOD_KEY: &str = "test-key-000000000000";`（16 字符，`test-key-` 前缀，**不命中** `sk-` 形态）
  - 禁止：不得 `#[ignore]`；不得 `std::fs`/`println!`/`dbg!`/`panic!(`；不得出现协议前缀/真实模型名/真实域名/`sk-` 形态；`assert_eq!` 不得打印 Key 明文（比较 `is_provided()`/`draft_issue()`/掩码枚举）
  - DoD：`"$CARGO" test -p meeting-copilot-ui-bridge --test llm_settings_view --no-run` 退 0；`grep -c '^fn ' "$UI_SETTINGS_TEST"` ≥5；`grep -c '#\[ignore\]' "$UI_SETTINGS_TEST"`=0；`grep -cE 'std::fs|panic!\(' "$UI_SETTINGS_TEST"`=0；`grep -c 'test-key-' "$UI_SETTINGS_TEST"` ≥1；`grep -cE '(deepseek|claude|gpt)-[A-Za-z0-9]' "$UI_SETTINGS_TEST"`=0
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test llm_settings_view --no-run; echo "exit=$?"` → `exit=0`

- [ ] **W-B4-71.2 AC-85 两用例：字段封闭 + 一步式默认** · `$UI_SETTINGS_TEST`
  - 追溯：AC-B4-85 · spec §7.15 AC-85、§6.24~6.28 · requirements §14.5 · design §7.9.1
  - DoD（`settings_view_is_closed_and_single_step`）：`SECTION_ORDER.len()==4` 且逐字顺序；投影后 `view` 8 字段逐项可访问；`set_items()` 由外部断言 18、`set_fns()`=7；`SettingsDefaults::single_step()` 4 项 == `0/0/6_000/true`；`format!("{:?}", view)` 含 `form_state`/`can_test` 等白名单键
  - DoD（`settings_defaults_need_only_api_key`）：`apply_defaults(&facts(1,1,0,None,false), &defaults())` → `model_draft==PresetIndex(0)`、`endpoint_draft==Index(0)`、`submitted=false`；把 `api_key_draft` 换成 `key_draft(GOOD_KEY)` 后投影 → `can_test==true`（**只填 Key 即可测**）
  - 禁止：不得断言 `view` 有 `sections`/`notice` 字段；不得用真实模型名构造清单
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test llm_settings_view settings_view_is_closed_and_single_step settings_defaults_need_only_api_key` 退 0

- [ ] **W-B4-71.3 AC-86 / AC-87 四用例：Key 宽松校验、提交时机、固定掩码、零泄露** · `$UI_SETTINGS_TEST`
  - 追溯：AC-B4-86、AC-B4-87 · spec §7.15 · §5.10.1-3/4 · design §7.6.4、§7.9.1
  - DoD（`settings_api_key_inline_errors_are_lenient`）：`submitted=true` + 空 Key → 含 `ApiKeyMissing`；15 字符 → 含 `ApiKeyTooShort`；16 字符 → 不含 `ApiKeyTooShort` 也不含 `ApiKeyMissing`；内部空格 → 含 `ApiKeyHasWhitespace`；首尾空白 + 换行 → **不**报 `HasWhitespace`（`from_draft` 已 trim）
  - DoD（`settings_errors_only_after_submit`）：`submitted=false` → `inline_errors.is_empty()`（含清单为空/Key 为空两种情形）；`submitted=true` → 非空
  - DoD（`settings_api_key_is_masked_without_payload`）：1/16/64/256 长度 → `serde_json::to_vec(&mask_api_key(&k))` 均等于 `b"\"***\""`；`serde_json::to_value(...)` 为 `Value::String("***")`；`MaskedKey` 无载荷（`kw_variants2`=2 + 无字段）
  - DoD（`settings_debug_and_errors_never_leak_key`）：`format!("{:?}", input)`、`format!("{:?}", intent)`、`format!("{:?}", view)` 均不含 `"test-key"`；8 条文案拼接不含 `"test-key"`
  - 禁止：不得用真实 Key 形态；不得断言掩码长度随输入变化（必须恒等）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test llm_settings_view settings_api_key_inline_errors_are_lenient settings_errors_only_after_submit settings_api_key_is_masked_without_payload settings_debug_and_errors_never_leak_key` 退 0

- [ ] **W-B4-71.4 AC-88 / AC-89 / AC-90 六用例：意图、明文扫描、模型、端点** · `$UI_SETTINGS_TEST`
  - 追溯：AC-B4-88、AC-B4-89、AC-B4-90 · spec §7.15 · §6.26/§6.24-4/5 · design §7.8.1、§7.6.1/7.6.2
  - DoD（`settings_persists_intent_only_and_clears_with_confirmation`）：`persist_intent(Memory)`/`persist_intent(OsSecureStore)` 两态逐字；`clear_confirmed=false` → `clear_intent()==None`；`true` → `Some(Clear)`；无 Key → 两方法均 `None`
  - DoD（`settings_repo_has_no_plaintext_key`）：用 `std::fs` 读 `crates/ui-bridge/src/settings/*.rs`（**测试文件不在 `core_files`**，允许 `std::fs`）逐个断言不含 `std::fs`/`keyring`/`card_store`/`Sqlite`；并断言源码不含 `sk-` 形态（正则用 `"s" + "k-"` 拼接，避免测试自身命中）
  - DoD（`settings_model_preset_or_custom_is_closed`）：`PresetIndex(0)` → `Preset{display_name=="model-a"}` 逐字节；`PresetIndex(9)` → `Custom{""}`；`CustomName` 129 字符 → 校验含 `ModelTooLong`；含空格/中文 → 含 `ModelMissing`；`kw_variants2 ModelSelectionView`=2
  - DoD（`settings_model_name_is_not_inlined`）：外部 `grep '(deepseek|claude|gpt)-[A-Za-z0-9]'` 对 `$SETTINGS` + 本测试文件 + `b4-status.md` 命中 0；测试内清单只来自 `MODELS`/`ENDPOINTS` 常量
  - DoD（`settings_endpoint_is_allowlist_index_only`）：`endpoints.len()==2` → `Index(0)`/`Index(1)` 均 `Option{...}` 逐字节等于清单项；`endpoints=[]` → `Unavailable` 且校验含 `EndpointUnavailable`；`Index(9)` → `Unavailable`；`kw_variants2 EndpointSelectionView`=2
  - DoD（`settings_endpoint_never_expands_allowlist`）：外部 `grep -rnE 'https?|\.com|DEEPSEEK|PATH_CONSTANT' "$SETTINGS"`=0；`git diff --quiet HEAD -- crates/infrastructure/src/llm` 退 0
  - 禁止：不得在测试内写 URL/域名/白名单常量名（连注释也不行——本测试进 `zero-transcript`，不进 `source`，但白名单常量名会被 AC-90 的 grep 命中）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test llm_settings_view settings_persists_intent_only_and_clears_with_confirmation settings_repo_has_no_plaintext_key settings_model_preset_or_custom_is_closed settings_model_name_is_not_inlined settings_endpoint_is_allowlist_index_only settings_endpoint_never_expands_allowlist` 退 0

- [ ] **W-B4-71.5 AC-91 / AC-92 四用例：单飞、取消、冷却、8 文案、保留输入** · `$UI_SETTINGS_TEST`
  - 追溯：AC-B4-91、AC-B4-92 · spec §7.15 · §6.27/§6.28-2~5 · design §7.7、§7.6.3
  - DoD（`settings_probe_state_machine_is_single_flight`）：`probe_in_flight=true` + `UserTest` → `(state 不变, None)`；`ConfiguredUnverified` + `UserTest` → `Testing` + `Some(Probe{..})`，且 `Probe.credential.is_provided()`、`endpoint_index==Some(0)`、`timeout_ms==6_000`；同输入 100 次转移 `format!("{:?}")` 相等
  - DoD（`settings_probe_cancel_and_cooldown`）：`Testing` + `UserCancel` → `ConfiguredUnverified` + `None`；`Cooling{5_000}` 投影 → `can_test=false`；`ClockTick{now==until}` → `ConfiguredUnverified`；`ClockTick{now<until}` → `Cooling{until-now}`
  - DoD（`settings_failure_messages_are_humanized`）：8 条互不相同、逐条与 design §7.3.4 表逐字；`humanize_probe_failure` 覆盖 `ALL` 8 项；拼接串不含 `test-key`/`http`/`sk-`
  - DoD（`settings_failure_keeps_user_input`）：`Failed{KeyRejected}` 后 `has_key=true`、`model_choice`/`endpoint_choice` 与失败前相等、`can_test=true`（非冷却）；`Cooling` 时 `can_test=false`
  - 禁止：不得用 `thread::sleep` 模拟冷却（用注入 `now_ms`）；不得断言真实秒数展示（51 只给毫值）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test llm_settings_view settings_probe_state_machine_is_single_flight settings_probe_cancel_and_cooldown settings_failure_messages_are_humanized settings_failure_keeps_user_input` 退 0

- [ ] **W-B4-71.6 AC-93 两用例：离线/凭据安全边界、`Debug` 白名单** · `$UI_SETTINGS_TEST`
  - 追溯：AC-B4-93 · spec §7.15 · §5.10-DFX、§5.10.1-10 · design §7.5.3、§7.5.4、§7.9.1
  - DoD（`settings_boundary_is_offline_and_credential_safe`）：外部 `--rule=zero-transcript` 退 0；`cargo tree` 禁词 0；测试内断言 `format!("{:?}", view)` 不含 Key 片段/端点串/模型名取值；`format!("{:?}", input)` 不含 `"test-key"`；`set_danger()`=0
  - DoD（`settings_debug_is_whitelisted`）：`format!("{:?}", SettingsView)` 含 `form_state`/`has_key`/`can_test`/`can_save`/`inline_errors`/`model_choice` 标签/`endpoint_choice` 标签；**不含** `model-a`/`endpoint-a`（展示名）；`format!("{:?}", ModelSelectionView::Preset{display_name:"model-a"})` 不含 `model-a`
  - 禁止：不得为了让测试通过而改 `Debug` 输出文本取值（必须改实现）；不得断言 `Display`（`SecretInput` 无 `Display`，用 `compile_fail` 文档测试**不引入**）
  - 证据：`"$CARGO" test -p meeting-copilot-ui-bridge --test llm_settings_view settings_boundary_is_offline_and_credential_safe settings_debug_is_whitelisted` 退 0

### W-B4-72 脚本扫描面、target 清单与 P8 出口收口（AC-B4-91/93/94）

> 前置：W-B4-65~71 全绿（**X-03**：文件未落盘前改脚本会让 7 条规则全部退 2）。**本任务必须与 65~71 同批提交**。

- [ ] **W-B4-72.1 `UI_SETTINGS_DIR`/`UI_SETTINGS_TEST`（拼接写法）+ `ui_settings_files()` + `require_core_faces()` + `core_files()`** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-91、AC-B4-93 · design §7.10.1（变量表 + `require_core_faces` + `core_files`）、**D-B4-P83/P95**、X-03
  - 改动内容：变量区（`UI_KEYWORD_TEST=` 之后）新增（**拼接写法**）：
    ```bash
    UI_SETTINGS_DIR="crates/ui-bridge/src/settings"
    UI_SETTINGS_TEST="crates/ui-bridge/tests/llm_set""tings_view.rs"
    ```
    新增 `ui_settings_files() { local file; for file in "${UI_SETTINGS_DIR}"/*.rs; do [[ -f "${file}" ]] && printf '%s\n' "${file}"; done }`；`require_core_faces()` 末尾追加 `need_dir "${UI_SETTINGS_DIR}" || ok=1`（**不**要求测试文件存在）；`core_files()` 在 `ui_keyword_files` 之后追加 `ui_settings_files`（**只加源码**，不加测试）
  - 禁止：**规则仍 7 条**；`--rule` 名称集合与退出码语义不变；不得改 `rule_source`/`rule_budget` 模式串；不得改其它三个脚本；**不得**在脚本内写出完整的 `llm_settings_view` 字符串
  - DoD：`grep -c 'UI_SETTINGS_DIR=' scripts/check-b4-gates.sh`=1；`grep -c 'UI_SETTINGS_TEST=' scripts/check-b4-gates.sh`=1；`grep -c 'ui_settings_files' scripts/check-b4-gates.sh` ≥3；`grep -c 'llm_settings_view' scripts/check-b4-gates.sh`=**0**；`--rule=source` 与 `--rule=budget` 各退 0
  - 证据：`bash scripts/check-b4-gates.sh --rule=source && bash scripts/check-b4-gates.sh --rule=budget; echo "exit=$?"` → `exit=0`
  - 反例演练（`mktemp -d`，不入库）：把 `$UI_SETTINGS_DIR` 改名 → `--rule=source` 必须退 **2** 且打印 `SCAN-MISSING`

- [ ] **W-B4-72.2 `zero-transcript` 扫描面纳入 `settings/**` 与 `tests/llm_settings_view.rs`** · `scripts/check-b4-gates.sh`
  - 追溯：AC-B4-89、AC-B4-93 · design §7.10.1、**D-B4-P83/P95/P96**
  - 改动内容：`rule_zero_transcript()` 的 `files` 数组在 `ui_keyword_files`/`UI_KEYWORD_TEST` 之后追加：
    ```bash
    while IFS= read -r file; do files+=("${file}"); done < <(ui_settings_files)
    if [[ -f "${UI_SETTINGS_TEST}" ]]; then files+=("${UI_SETTINGS_TEST}"); fi
    ```
  - 禁止：不得新增/放宽禁止模式；测试**不得**进 `core_files()`（D-B4-P95 与 38 的 P59 不同）
  - DoD：`--rule=zero-transcript` 退 0；`grep -c 'UI_SETTINGS_TEST' scripts/check-b4-gates.sh` ≥2；`awk '/^core_files\(\)/{f=1} f&&/^\}/{exit} f' scripts/check-b4-gates.sh | grep -c 'UI_SETTINGS_TEST'`=**0**
  - 证据：`bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "exit=$?"` → `exit=0`
  - 反例演练（`mktemp -d`，不入库）：在 `$UI_SETTINGS_TEST` 副本追加一行含 `https://` → 退 **1**

- [ ] **W-B4-72.3 `NEW_TARGETS` 23 → 24（追加 `llm_settings_view`）** · `scripts/test-name-inventory.sh`
  - 追溯：AC-B4-94 · spec §7.15 · design §7.10.2、**D-B4-P83/X-11**
  - 改动内容：`NEW_TARGETS='… keyword_card_view'` 尾部追加 ` llm_settings_view`
  - 禁止：不得改动既有 23 个 token 的顺序/拼写；`--diff` 语义不变
  - DoD：`new_targets_count`=**24**；`grep -c 'llm_settings_view' scripts/test-name-inventory.sh`=1；`grep -c 'keyword_card_view' scripts/test-name-inventory.sh`=1；`git diff --stat HEAD -- scripts/test-name-inventory.sh` 仅 1 行变更
  - 证据：`new_targets_count` → `24`；`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt; echo "exit=$?"` → `exit=0` 且含 `removed=0`

- [ ] **W-B4-72.4 `--rule=targets` 保持 7 项 + 七规则 + `cargo tree` + 白名单零 diff** · `scripts/check-b4-gates.sh`、仓库根
  - 追溯：AC-B4-89、AC-B4-90、AC-B4-93、AC-B4-94 · spec §7.15 · **X-05**、D-B4-P79
  - 改动内容：**零改动**。`rule_targets()` 循环仍 7 项；`crates/infrastructure/src/llm/**` 零 diff
  - 禁止：不得把 `llm_settings_view` 加入 `rule_targets()`；不得扩白名单/改路径常量/改 `request_timeout_ms` 默认值（6_000 只读）
  - DoD：`--rule=targets` 退 0 且输出行数 0；`awk '/^rule_targets\(\)/{f=1} f&&/^\}/{exit} f' scripts/check-b4-gates.sh | grep -c 'llm_settings_view'`=**0**；七规则 7×0；`cargo tree -p meeting-copilot-ui-bridge --edges normal --prefix none | grep -cE 'tauri|windows|objc2|reqwest|hyper|sqlx'`=0；`git diff --quiet HEAD -- crates/infrastructure/src/llm` 退 0
  - 证据：
    ```bash
    set -e
    [ -z "$(bash scripts/check-b4-gates.sh --rule=targets)" ]
    [ "$(awk '/^rule_targets\(\)/{f=1} f&&/^\}/{exit} f' scripts/check-b4-gates.sh | grep -c 'llm_settings_view')" = "0" ]
    for r in source zero-transcript budget outbound record targets doc-plan; do
      bash scripts/check-b4-gates.sh --rule="$r" || exit 1
    done
    git diff --quiet HEAD -- crates/infrastructure/src/llm
    echo "exit=$?"
    ```
    `"$CARGO" tree -p meeting-copilot-ui-bridge --edges normal --prefix none` 禁词命中 0

- [ ] **W-B4-72.5 `b4-status.md` §19 + 零 diff 矩阵 + P8 出口收口与 U-05 关闭（AC-B4-85~94）** · `docs/progress/b4-status.md`、仓库根
  - 追溯：AC-B4-85~94 · spec §7.15 / §8 · design §7.9.2、§7.10.4、§7.13
  - 改动内容：追加 `## §19 P8 增量快照（W-B4-65 ~ W-B4-72）`：§19.1 命令结果表（`llm_settings_view` 18 用例、11 target、7 规则、`NEW_TARGETS`=24、`api.rs`=4、`set_items`=18/`set_fns`=7/`set_danger`=0、`Cargo.toml`/`Cargo.lock` 零 diff、全量 `cargo test --workspace`、`--diff` `removed=0`）；§19.2 十行 AC 三态（AC-B4-85~94 → `passed`，各附用例名）；§19.3 **U-05 关闭登记**（**保留** §10.1/§16/§17 原文"规格已定，待实现"，在 §10.1 该行行末追加"，已于 P8 关闭（见 §18/§19）"；§16/§17 结论段追加"→ 已实现（§18/§19）"，**不删除**原文）；§19.4 偏差登记 X-10（降级常量+意图口径落地）、X-11（22→23→24 顺序落地）、D-B4-P85~P107 中实际采用项；§19.5 不宣称清单（离线通过 ≠ 真实 UI/焦点/安全存储/网关连通性；不宣称"设置向导已可用"、不宣称 G3/G4/真机）
  - 禁止：§19 内不得写任何 Key 片段/模型名/域名/请求标识；只写 AC 编号、用例名、枚举、计数、毫值；**不得**改 U-06/U-07/U-08 的 open 状态；**不得**删除 §16/§17 的"规格已定"原文
  - DoD：
    - `grep -c '^## §19' docs/progress/b4-status.md`=1
    - `sed -n '/^## §19/,$p' docs/progress/b4-status.md | grep -cE 'AC-B4-(8[5-9]|9[0-4])'` ≥**10**
    - `grep -c 'U-05.*规格已定' docs/progress/b4-status.md` ≥1（保留历史）
    - `grep -c 'U-05.*设置向导' docs/progress/b4-status.md` ≥1
    - `grep -c 'U-05.*closed' docs/progress/b4-status.md` ≥1
    - `grep -cE 'U-0[678].*open' docs/progress/b4-status.md` ≥2
    - `set_cases`=**18**、`set_items`=**18**、`set_fns`=**7**、`set_danger`=**0**
    - `kw_fields SettingsView`=8、`SettingsFacts`=6、`SettingsInput`=5、`SettingsDefaults`=4、`ProbeRequest`=4
    - `--rule=zero-transcript` 退 0、`--rule=doc-plan` 退 0
    - 零 diff 串联命令全部退 0（含 P7 的 `keyword_card_view` 19 用例仍绿）
    - 11 个 target 逐个退 0；`cargo test --workspace` `failed=0`/`ignored=9`；`--diff` `removed=0`
  - 证据：
    ```bash
    set -e
    [ "$(grep -c '^pub fn ' crates/application/src/gate/api.rs)" = "4" ]
    git diff --quiet HEAD -- crates/application/src/gate
    git diff --quiet HEAD -- crates/application/src/cards
    git diff --quiet HEAD -- crates/domain/src/model
    git diff --quiet HEAD -- crates/infrastructure/src/llm
    git diff --quiet HEAD -- crates/ui-bridge/src/dto.rs crates/ui-bridge/tests/schema.rs
    git diff --quiet HEAD -- crates/ui-bridge/Cargo.toml Cargo.lock
    git diff --quiet HEAD -- crates/ui-bridge/src/keyword_card
    bash scripts/check-b4-gates.sh --rule=record
    test ! -d crates/application/tests/replay
    [ "$(set_cases)" = "18" ] && [ "$(set_items)" = "18" ] && [ "$(set_fns)" = "7" ] && [ "$(set_danger)" = "0" ]
    "$CARGO" test -p meeting-copilot-ui-bridge --test keyword_card_view
    for t in gate_prefilter gate_relevance gate_dedupe gate_budget gate_outbound gate_replay gate_keyword citation_align card_compose; do
      "$CARGO" test -p meeting-copilot-application --test "$t" || exit 1
    done
    "$CARGO" test -p meeting-copilot-ui-bridge --test llm_settings_view
    for r in source zero-transcript budget outbound record targets doc-plan; do
      bash scripts/check-b4-gates.sh --rule="$r" || exit 1
    done
    "$CARGO" fmt --all -- --check || exit 1
    "$CARGO" clippy --workspace --all-targets -- -D warnings || exit 1
    bash scripts/check-layering.sh || exit 1
    bash scripts/scan-secrets.sh || exit 1
    "$CARGO" deny check || exit 1
    "$CARGO" test --workspace
    bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
    echo "exit=$?"
    ```
  - 证据产物：全部退出码与日期写入 `b4-status.md` §19.1；阶段结论写入 §19.2/§19.3

---

## P9 · 13.3 回放本体（增量 AC-B4-95~104）

> 阶段：编码任务规划的**第七个增量批次**（P8 之后，**串行**）。需求来源：`requirements.md` §15（REQ-B4R-01~05、AC-B4-95~104、§15.4 的 4/1+2/1/3/2/5/3 封闭面）、`spec.md` §5.11 / §6.29 / §7.16 / §9.1 J-35~J-44、D-B4-10、N-01、`design.md` 第八章（§8.0~§8.11，含 §8.9.2 D-B4-P118~P130）、`docs/progress/b4-status.md` §20（D-B4-P108~P117，AC-B4-95~104 全 `pending`）。
> 进度基线（P8 收口后）：`NEW_TARGETS`=**24**；11 个 target 全绿；七规则 7×0；`api.rs` 4 函数零 diff；`crates/application/src/gate/replay.rs` **仅 1 行注释**（`//! 回放驱动骨架；P0 骨架，P3 填充 200 例对比。`）；`gate/mod.rs:35` 已有 `pub(crate) mod replay;`；`tests/gate_replay.rs` **6 用例**、`total=200`/`mismatched=0`/`seed=7`；`crates/application/tests/replay/` **不存在**；`GateRecord` 14（`kw_fields`=15 含 `unknown`）、`ReplayReport` 5、`GateInputFixture` 23、`GateCounter` 32、`GateRejectReason` 23。
> **范围**：只做 7 个顶层类型 + 1 个 `run` 方法 + 3 个私有助手 + `gate/mod.rs` **1 行** + 22 个测试函数 + 24 例全绿集 + 5 例探针集 + 1 个新 target + 1 处 token + 1 处状态登记。**不建** `tests/replay/`、不做分组报表、不新增依赖、不改 `api.rs` 与其余 `gate/**`。
> **与 U-06 的关系**：P9 **关闭 U-06**（回放本体）。`b4-status.md` §10.1 的"规格已定，待实现"原文**保留**，行末追加"，已于 P9 closed（作废 U-06 open，见 §23）"；§20 结论段追加"→ 已实现（§23）"，**不删除**原文（AC-104 的 `grep -c 'U-06.*回放本体'` 与 AC-103 的历史证据依赖它）。
> **与 U-07/U-08 的关系**：均保持 `open`；不宣称 41.5/42/43/52、真实端点、6s/10s p95、误触发率、G3/G4/真机；9 个 `#[ignore]` 名称与数量不变；**不产生**任何新 `#[ignore]`。
> **提交后复跑口径（X-29）**：P9 已作为单提交 `1b0dbf7` 落库（`feat(gate): 落地 13.3 回放本体与 22 例自检集`）。本章内所有 `git diff [--quiet|--numstat|--stat] HEAD -- …` 判据在提交后恒为 0，**复跑时把基线前移为 `1b0dbf7^`**（命令与实测见 §0.10.2）；其余判据（`cargo test`、`grep`、`kw_fields*`、目录断言、`NEW_TARGETS`）不受影响。**P10 及其后的零 diff 基线 = `1b0dbf7`**。
> **P9 出口判据（缺一即 P9 未完成）**：① `gate_replay_driver` target **22 passed / 0 failed**；② 顶层 `pub` 类型=**7**、顶层 `pub fn`=**0**；`kw_fields_life ReplayCase`=**4**/`kw_fields ReplayRun`=**3**/`kw_fields ReplayFailure`=**2**；`kw_variants2 ReplayMismatchCode`=**5**/`ReplayLoadError`=**3**；③ `gate/mod.rs` `+1/-0`；`api.rs` 4；其余 **12** 个 `gate/*.rs` 零 diff（**含 `api.rs`**，即除 `replay.rs` 与 `mod.rs` 外的 12 个文件，与 `gate_12_zero()` 的列表逐字一致）；④ `NEW_TARGETS`=**25**、`--rule=targets` 退 0 且 `grep -c 'gate_replay_driver' scripts/check-b4-gates.sh`=**0**；⑤ 七规则 7×0；`--rule=zero-transcript` 覆盖 `tests/gate_replay_driver.rs`；⑥ `crates/domain/src/model/**`、`crates/application/src/cards/**`、`crates/ui-bridge/**`、`crates/infrastructure/**`、`crates/application/tests/gate_replay.rs`、`scripts/check-b4-gates.sh`、`Cargo.toml`/`Cargo.lock`/`crates/application/Cargo.toml` 零 diff；⑦ 全量 `cargo test --workspace` `failed=0`/`ignored=9`、`--diff` `removed=0`；⑧ AC-B4-95~104 十行 `passed`、U-06 标 `closed` 且保留"规格已定"；⑨ 13 项封闭面 `GateRecord` 14 / 审计 7 / `GateRejectReason` 23 / `GateInputFixture` 23 / `ReplayReport` 5 / `GateCounter` 32 / `ComposedCard` 11 / `KeywordCardView` 10 / `SettingsView` 8 不变。

**P9 需求 → AC → 用例 → 任务映射（REQ-B4R-01~05 全覆盖；22 用例逐字）**

| REQ | AC | 用例名（逐字） | 任务 |
|---|---|---|---|
| REQ-B4R-01 驱动契约与交付形态 | AC-B4-95 | `replay_driver_types_are_closed` | W-B4-73.1、73.4、80.1 |
| REQ-B4R-02 双层用例模型 | AC-B4-96 | `replay_case_layers_are_separated` | W-B4-73.1、79.1、80.1 |
| REQ-B4R-02 装载与排序 | AC-B4-97 | `replay_case_source_rejects_empty_set` | W-B4-74.1、74.3 |
| REQ-B4R-02 | AC-B4-97 | `replay_case_source_rejects_invalid_case_id` | W-B4-74.1、74.2、74.3 |
| REQ-B4R-02 | AC-B4-97 | `replay_case_source_rejects_duplicate_ids` | W-B4-74.1、74.3 |
| REQ-B4R-02 | AC-B4-97 | `replay_case_set_is_sorted_by_case_id` | W-B4-74.2、74.3 |
| REQ-B4R-03 驱动与 4 函数 | AC-B4-98 | `replay_driver_uses_only_four_functions` | W-B4-75.2、75.3、80.1 |
| REQ-B4R-03 | AC-B4-98 | `replay_driver_injects_constant_clock` | W-B4-75.1、75.3 |
| REQ-B4R-04 记录核验 | AC-B4-99 | `replay_driver_records_are_closed` | W-B4-77.1、77.3 |
| REQ-B4R-04 | AC-B4-99 | `replay_driver_record_violation_is_reported` | W-B4-77.1、77.3 |
| REQ-B4R-04 | AC-B4-99 | `replay_record_check_rejects_illegal_skip_stage` | W-B4-77.1、77.3 |
| REQ-B4R-04 失败报告 | AC-B4-100 | `replay_failure_reports_case_id_and_code_only` | W-B4-76.2、76.3 |
| REQ-B4R-04 | AC-B4-100 | `replay_failure_codes_cover_all_mismatch_kinds` | W-B4-76.1、76.3、79.2 |
| REQ-B4R-04 | AC-B4-100 | `replay_driver_reports_totals_are_consistent` | W-B4-76.2、76.3 |
| REQ-B4R-03 确定性 | AC-B4-101 | `replay_driver_is_deterministic_twice` | W-B4-78.1、78.2、78.3 |
| REQ-B4R-03 | AC-B4-101 | `replay_driver_is_order_insensitive` | W-B4-78.2、78.3（**X-17**） |
| REQ-B4R-03 | AC-B4-101 | `replay_driver_seed_is_report_only` | W-B4-78.1、78.3 |
| REQ-B4R-05 零转录 | AC-B4-102 | `replay_driver_zero_transcript_boundary` | W-B4-77.2、80.2、80.3 |
| REQ-B4R-05 | AC-B4-102 | `replay_driver_debug_is_whitelisted` | W-B4-77.2、80.2 |
| REQ-B4R-02/05 只读装载 | AC-B4-97/102 | `replay_driver_load_is_io_free_and_read_only` | W-B4-74.3、80.1 |
| REQ-B4R-05 与 200 例并存 | AC-B4-103 | `replay_driver_coexists_with_gate_replay_fixture` | W-B4-80.1、81.3 |
| REQ-B4R-05 | AC-B4-103 | `replay_driver_does_not_touch_200_cases` | W-B4-80.2、81.3 |
| REQ-B4R-05 target 与门禁 | AC-B4-104 | （target 实跑，无独立用例） | W-B4-81.1~81.4 |

> **22 个函数名单**（顺序即 `#[test]` 出现顺序，实施时逐字照抄）：① `replay_driver_types_are_closed`；② `replay_case_layers_are_separated`；③ `replay_case_source_rejects_empty_set`；④ `replay_case_source_rejects_invalid_case_id`；⑤ `replay_case_source_rejects_duplicate_ids`；⑥ `replay_case_set_is_sorted_by_case_id`；⑦ `replay_driver_uses_only_four_functions`；⑧ `replay_driver_injects_constant_clock`；⑨ `replay_driver_records_are_closed`；⑩ `replay_driver_record_violation_is_reported`；⑪ `replay_record_check_rejects_illegal_skip_stage`；⑫ `replay_failure_reports_case_id_and_code_only`；⑬ `replay_failure_codes_cover_all_mismatch_kinds`；⑭ `replay_driver_reports_totals_are_consistent`；⑮ `replay_driver_is_deterministic_twice`；⑯ `replay_driver_is_order_insensitive`；⑰ `replay_driver_seed_is_report_only`；⑱ `replay_driver_zero_transcript_boundary`；⑲ `replay_driver_debug_is_whitelisted`；⑳ `replay_driver_coexists_with_gate_replay_fixture`；㉑ `replay_driver_does_not_touch_200_cases`；㉒ `replay_driver_load_is_io_free_and_read_only`。

### P9 公共断言（每个 shell 会话执行一次，后续任务直接引用）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
GATE="crates/application/src/gate"
RP="$GATE/replay.rs"
GATE_MOD="$GATE/mod.rs"
DRV="crates/application/tests/gate_replay_driver.rs"

# 沿用 P4/P8 的 kw_fields / kw_variants2（勿重复定义）
kw_fields() { awk -v s="pub struct $1 {" 'index($0,s)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    pub [a-z_]+:'; }
# 泛型结构（如 `pub struct ReplayCase<'a> {`）不能用 kw_fields（其 s 含 " {" 且要求行首全等），必须用下方泛型版
kw_fields_life() { awk -v s="pub struct $1<" 'index($0,s)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    pub [a-z_]+:'; }
kw_variants2() { awk -v e="pub enum $1 {" 'index($0,e)==1 {f=1; next} f && /^\}/{exit} f' "$2" | grep -cE '^    [A-Z][A-Za-z0-9]*[[:space:]]*([({]|,)'; }
new_targets() { grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//"; }
rp_items() { grep -cE '^pub (struct|enum|trait) ' "$RP"; }
rp_fns()   { grep -cE '^pub fn ' "$RP"; }
drv_cases() { grep -c '^fn replay_' "$DRV"; }
drv_list()  { "$CARGO" test -p meeting-copilot-application --test gate_replay_driver -- --list 2>/dev/null; }
drv_list_count() { drv_list | grep -c ': test'; }
gate_12_zero() { git diff --quiet HEAD -- "$GATE/api.rs" "$GATE/prefilter.rs" "$GATE/relevance.rs" "$GATE/dedupe.rs" \
  "$GATE/budget.rs" "$GATE/lease.rs" "$GATE/outbound.rs" "$GATE/record.rs" "$GATE/diagnostics.rs" \
  "$GATE/input.rs" "$GATE/clock.rs" "$GATE/keyword.rs"; }
```

> **X-18 加严（`--rule=source`）**：`strip_tests` 只截到文件内**第一个** `#[cfg(test)]`；`gate/replay.rs` 若不含 `#[cfg(test)]`（本设计不要求内含单测，`record_check` 单测放测试文件），则**全文**都在扫描面内。因此 `$RP` 的代码与**注释**都不得出现 `panic!(`/`todo!(`/`unimplemented!(`/`std::env`/`Instant::now`/`SystemTime::now`/`thread::sleep`/`std::net`/`tokio::time`/`static mut`/`OnceLock`/`lazy_static`/`thread_local`/`LLM_`。
> **零转录与样本纪律**：arena 示例文本统一为**唯一恒串** `"内部示例文本-13r"`（片段文本、`lexicon_names`、`r11` 的 `sensitive_terms`、`query_seed` 同串复用；**不再使用另一套 `"示例词-13r"`**）；ID 用 `session-13r`/`trace-13r`/`dedupe-13r`；`case_id` 用 `r01-`/`p01-` 形态；`seed` 用 `13`/`29`。这些串**不**命中 `zero-transcript` 的五类模式（协议前缀/`sk-` 形态/鉴权头/模型名前缀/`request_id:`）。
> **断言消息三段模板（测试内只允许）**：`assert!(..., "{} {:?}", case_id, code)`、`assert_eq!(..., ..., "case={}", case_id)`、`assert_eq!(..., ..., "iter={}", iter)`；**禁止**拼接 `expected`/`decision` 结构。
> **依赖纪律**：`Cargo.toml`/`Cargo.lock`/`crates/application/Cargo.toml` **零 diff**；不引 `serde_json`/`rand`/`getrandom`；确定性只用 `PartialEq` 逐项比较。
> **提交纪律（D-B4-P128）**：`scripts/test-name-inventory.sh` 的 token 必须与 73~80 的源码/测试**同批提交**；否则 `--diff` 会把 `gate_replay_driver` 判为"新增未登记"。`scripts/check-b4-gates.sh` **零改动**，无需同批（D-B4-P115）。

### P9 依赖图（单向，不可倒置）

```text
P9: W-B4-73.0 ─► W-B4-73.1 ─► W-B4-73.2 ─► W-B4-73.3 ─► W-B4-73.4
                                             └─► W-B4-74 ─► W-B4-75 ─► {W-B4-76 ∥ W-B4-77} ─► W-B4-78 ─► W-B4-79 ─► W-B4-80 ─► W-B4-81
```

- **不可压缩四点**：① 73.0（提交 §20 登记）先于 73.1；② 73.1（7 类型可编译）先于 73.2/74/75；③ 73.3（`pub use replay::*;`）先于 79/80（集成测试从 crate 外 `use`）；④ 75.1（常量时钟 + 三调用）先于 76/77（决策与记录都来自该步）。
- **可并行**：76（决策比对）∥ 77（记录核验）；79.1（`Arena`）∥ 80.2（自扫模板）；81.1（token）∥ 81.2（targets/七规则复核）。
- **串行收口**：81.4 必须在 73~81.3 全绿后一次性登记。

### W-B4-73 契约骨架与门面（AC-B4-95/96/100）

> 前置：W-B4-73.0 提交闸门。落点 `$RP`（唯一改写）+ `$GATE_MOD`（+1 行）；`Cargo.*` **零改动**。

- [ ] **W-B4-73.0 提交闸门：先提交 `b4-status.md` §20 增量登记** · 仓库根
  - 追溯：AC-B4-104 · design §8.8.5-1 · P9 开工前置（§0 加注）
  - 改动内容：把 `docs/progress/b4-status.md` 的 §20（13.3 需求增量登记，含 §20.6 X-19~X-24 补正）单独提交；提交信息沿用仓库风格（`docs(status): 登记 B4 13.3 回放本体需求增量`）；**不提交任何代码**；提交后 HEAD 即为 P9 零 diff 基线（当前前置指针 `f153d7f`）
  - 禁止：不得在闸门内改 `crates/**`/`scripts/**`；不得 `--amend`；不得把 P9 代码混入本提交
  - DoD：`[ -z "$(git status --short)" ]` 退 0；`git log -1 --format=%s` 命中 `13.3` 或 `回放`；`grep -c '^| AC-B4-' spec.md`=**104**；`new_targets | wc -w`=**24**；`git diff --quiet HEAD -- "$RP" "$GATE_MOD"` 退 0（此时 `replay.rs` 仍是 1 行骨架）
  - 证据：
    ```bash
    set -e
    git status --short
    git log -1 --format=%s
    [ "$(grep -c '^| AC-B4-' .codeartsdoer/specs/b4_gate/spec.md)" = "104" ]
    [ "$(new_targets | wc -w | tr -d ' ')" = "24" ]
    [ "$(grep -c '' "$RP")" = "1" ]
    echo "exit=$?"
    ```
  - 证据产物：提交哈希、日期、`replay.rs` 1 行事实写入 `b4-status.md` §23.1

- [ ] **W-B4-73.1 7 项顶层契约（4/1/1/3/2/5/3）+ 派生与 `Debug` 白名单** · `$RP`
  - 追溯：AC-B4-95、AC-B4-96、AC-B4-100 · spec §5.11.1-1/2/8、§6.29-1~7 · design §8.3.1、§8.3.2、§8.3.3、§8.4.1、D-B4-P108/**P109**/**P119**/**P129**
  - 改动内容：把 1 行骨架注释改为 `//! 13.3 回放本体：驱动/装载/比对/报告（0 IO/0 网络/0 随机/0 生产计数）。`；定义**恰 7 项**顶层 `pub`：
    1. `pub struct ReplayCase<'a>`（4 字段 `case_id: &'a str` / `input: GateInput<'a>` / `policy: GatePolicy` / `expected: ExpectedDecision`）= `Clone`（**本步不写 `Debug`**，由 77.2 手写）；
    2. `pub trait ReplayCaseSource`（2 方法 `fn seed(&self) -> u64` / `fn cases(&self) -> Vec<ReplayCase<'_>>`）；
    3. `pub struct ReplayDriver;` = `Debug + Clone + Copy + Default`，`impl { pub fn run(...) }` 见 73.2；
    4. `pub struct ReplayRun`（3 字段 `report: ReplayReport` / `failures: Vec<ReplayFailure>` / `records: Vec<GateRecord>`）= `Clone + PartialEq`（**本步不写 `Debug`**，由 77.2 手写）；
    5. `pub struct ReplayFailure`（2 字段 `case_id: String` / `code: ReplayMismatchCode`）= `Debug + Clone + PartialEq + Eq`（派生即白名单）；
    6. `pub enum ReplayMismatchCode`（5 变体，顺序 `ExpectedAllowGotDeny`/`ExpectedDenyGotAllow`/`ReasonMismatch`/`StageMismatch`/`RecordViolation`）= `Debug + Clone + Copy + PartialEq + Eq` + `impl { pub const ALL: [Self; 5] }`；
    7. `pub enum ReplayLoadError`（3 变体，顺序 `EmptyCaseSet`/`DuplicateCaseId`/`InvalidCaseId`）= `Debug + Clone + Copy + PartialEq + Eq` + `impl { pub const ALL: [Self; 3] }`。
    导入只用：`crate::gate::{detect_gaps, evaluate_gate, record_from, FixedClock, GateInput}`（如需 trait 对象再加 `crate::gate::AuditSink`）、`meeting_copilot_domain::model::gate::{GateBudget, GateDecision, GatePolicy, GateRecord, GateStage}`、`meeting_copilot_domain::model::gate_replay::{ExpectedDecision, ReplayReport}`；不 `use` `prefilter`/`relevance`/`dedupe`/`budget`/`lease`/`outbound`。
  - 禁止：不得出现第 8 个顶层 `pub`（含 `pub const`/`pub type`）；不得有顶层 `pub fn`；`ReplayCase`/`ReplayRun` **不得** `derive(Debug)`；`ReplayRun` 不得加第 4 字段、不得 `Serialize`；不得出现 `from_fixture`/`InsightDraft`；不得 `mod common`；**上述禁词不要写进源码注释**（`--rule=source` 与 73.2 的 `grep` 会命中注释文本）
  - DoD：`rp_items`=**7**；`rp_fns`=**0**；`kw_fields_life ReplayCase "$RP"`=**4**（泛型行 `pub struct ReplayCase<'a> {` 不匹配 `kw_fields`，必须用 `kw_fields_life`）；`kw_fields ReplayRun`=**3**；`kw_fields ReplayFailure`=**2**；`kw_variants2 ReplayMismatchCode`=**5**；`kw_variants2 ReplayLoadError`=**3**；`grep -c 'impl core::fmt::Debug for Replay' "$RP"`=**0**（此时点；77.2 补至 2）；`grep -c 'from_fixture\|InsightDraft' "$RP"`=**0**；`"$CARGO" build -p meeting-copilot-application` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(rp_items)" = "7" ] && [ "$(rp_fns)" = "0" ]
    [ "$(kw_fields_life ReplayCase "$RP")" = "4" ]
    [ "$(kw_fields ReplayRun "$RP")" = "3" ]
    [ "$(kw_fields ReplayFailure "$RP")" = "2" ]
    [ "$(kw_variants2 ReplayMismatchCode "$RP")" = "5" ]
    [ "$(kw_variants2 ReplayLoadError "$RP")" = "3" ]
    [ "$(grep -c 'impl core::fmt::Debug for Replay' "$RP")" = "0" ]
    "$CARGO" build -p meeting-copilot-application
    echo "exit=$?"
    ```
  - 证据产物：7/0/4/3/2/5/3 写入 `b4-status.md` §23.1

- [ ] **W-B4-73.2 `ReplayDriver::run` 骨架：D0~D7 唯一顺序** · `$RP`
  - 追溯：AC-B4-97、AC-B4-98、AC-B4-101 · spec §5.11.1-3/4/7 · design §8.2.3、§8.2.4、§8.5.1、D-B4-P112/**P120**/**P123**
  - 改动内容：`impl ReplayDriver { pub fn run(&self, source: &dyn ReplayCaseSource) -> Result<ReplayRun, ReplayLoadError> }`，严格 D0~D7：D0 `seed = source.seed()` → D1 `cases = source.cases()` → D2 空集 → D3 形态 → D4 唯一 → D5 按 `case_id` 升序**稳定**排序 → D6 逐例 `detect_gaps(&case.input.scan)`、`evaluate_gate(&case.input, &case.policy, &FixedClock::new(case.input.now_ms, 0))`、`record_from(&case.input, &decision, &case.input.budget)` → D6b 比对 → `records.push(record)` → D7 汇总 `ReplayReport`；`total = u32::try_from(cases.len()).map_or(u32::MAX, |n| n)`；`matched = total.saturating_sub(mismatched)`（与 76.2 同口径，73.2 即用饱和减法，避免后续替换实现）
  - 禁止：不得 `panic!`/`unwrap()`/`expect()`；不得 `sort_unstable_by`；不得跨例共享 `FixedClock`；不得复制级序/阈值/预算/冷却；`detect_gaps` 结果不进 `ReplayRun`；不得写审计/计数/文件
  - DoD：`grep -c 'FixedClock::new' "$RP"`=**1**；`grep -cE 'GateStage::G[1-5]' "$RP"` ≤**2**；`grep -cE 'prefilter::|relevance::|dedupe::|budget::|lease::|outbound::' "$RP"`=**0**；`grep -cE 'panic!\(|unwrap\(\)|expect\(' "$RP"`=**0**；`grep -c 'sort_unstable_by' "$RP"`=**0**；`bash scripts/check-b4-gates.sh --rule=source` 退 **0**；`"$CARGO" build -p meeting-copilot-application` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'FixedClock::new' "$RP")" = "1" ]
    [ "$(grep -cE 'GateStage::G[1-5]' "$RP")" -le 2 ]
    [ "$(grep -cE 'prefilter::|relevance::|dedupe::|budget::|lease::|outbound::' "$RP")" = "0" ]
    [ "$(grep -cE 'panic!\(|unwrap\(\)|expect\(' "$RP")" = "0" ]
    bash scripts/check-b4-gates.sh --rule=source
    "$CARGO" build -p meeting-copilot-application
    echo "exit=$?"
    ```

- [ ] **W-B4-73.3 `gate/mod.rs` +1 行 `pub use replay::*;`** · `$GATE_MOD`
  - 追溯：AC-B4-95 · design §8.8.1、§8.8.3、D-B4-P125
  - 改动内容：在既有 `pub use relevance::decay_milli;` 之后插入**单行** `pub use replay::*;`；保留 `:35` 的 `pub(crate) mod replay;`；其余 **9 条 `pub use` 语句（共 11 行，`input` 一条跨 3 行）** 一字不改
  - 禁止：不得逐项 `pub use replay::{...}`（rustfmt 会拆成 7 行，破坏 +1）；不得改 `pub(crate) mod replay;`；不得加 `pub use` 到 `api`/`clock` 等既有行
  - DoD：`git diff --numstat HEAD -- "$GATE_MOD"` = `1	0`（1 增 0 删）；`grep -c 'pub use replay' "$GATE_MOD"`=**1**；`grep -c 'pub(crate) mod replay;' "$GATE_MOD"`=**1**；`"$CARGO" build -p meeting-copilot-application` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(git diff --numstat HEAD -- "$GATE_MOD" | awk '{print $1" "$2}')" = "1 0" ]
    [ "$(grep -c 'pub use replay' "$GATE_MOD")" = "1" ]
    [ "$(grep -c 'pub(crate) mod replay;' "$GATE_MOD")" = "1" ]
    echo "exit=$?"
    ```

- [ ] **W-B4-73.4 契约封闭与只读面静态复核（不改代码）** · 仓库根
  - 追溯：AC-B4-95、AC-B4-96 · design §8.1.4、§8.8.4
  - 改动内容：只跑静态判据；若任一不满足，回到 73.1~73.3 修正
  - 禁止：不得为过判据而改 `api.rs`/`domain/**`/`scripts/**`
  - DoD：`grep -c '^pub fn ' "$GATE/api.rs"`=**4**；`kw_fields GateInputFixture crates/domain/src/model/gate_replay.rs`=**23**；`kw_fields ReplayReport crates/domain/src/model/gate_replay.rs`=**5**；`kw_fields GateRecord crates/domain/src/model/gate.rs`=**15** 且剔除 `unknown` 后=**14**；`gate_12_zero` 退 0；`git diff --quiet HEAD -- crates/domain/src/model` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c '^pub fn ' "$GATE/api.rs")" = "4" ]
    [ "$(kw_fields GateInputFixture crates/domain/src/model/gate_replay.rs)" = "23" ]
    [ "$(kw_fields ReplayReport crates/domain/src/model/gate_replay.rs)" = "5" ]
    [ "$(kw_fields GateRecord crates/domain/src/model/gate.rs)" = "15" ]
    [ "$(awk '/^pub struct GateRecord \{/{f=1;next} f&&/^\}/{exit} f' crates/domain/src/model/gate.rs \
         | grep -oE '^    pub [a-z_]+:' | grep -vc '^    pub unknown:$')" = "14" ]
    gate_12_zero
    git diff --quiet HEAD -- crates/domain/src/model
    echo "exit=$?"
    ```
  - 证据产物：4/23/5/15→14 与零 diff 结果写入 `b4-status.md` §23.1

### W-B4-74 用例装载与升序排序（AC-B4-97）

> 前置：W-B4-73（7 类型可编译 + `pub use` 生效）。本任务只改 `$RP` 的私有助手，**不增顶层 `pub` 项**。

- [ ] **W-B4-74.1 私有 `load_guard`：空集 → 形态 → 唯一 的单射顺序** · `$RP`
  - 追溯：AC-B4-97 · spec §5.11.1-3、§5.11.3-1/2、§6.29-7 · design §8.3.4、D-B4-P120（`InvalidCaseId` **优先于** `DuplicateCaseId`）
  - 改动内容：`fn load_guard(cases: &[ReplayCase<'_>]) -> Result<(), ReplayLoadError>`（无 `pub`）：① `cases.is_empty()` → `EmptyCaseSet`；② 逐项形态（`case_id.is_empty()` / `len() > 64` / 非 `[A-Za-z0-9_-]`）→ `InvalidCaseId`；③ `BTreeSet<&str>` 插入失败 → `DuplicateCaseId`
  - 禁止：不得排序（排序在 74.2）；不得执行判定；不得 `panic!`/`unwrap()`；不得回显 `case_id` 内容（错误变体无载荷）
  - DoD：`grep -c 'fn load_guard' "$RP"`=**1**；`grep -c 'BTreeSet' "$RP"` ≥**1**；`grep -cE 'panic!\(|unwrap\(\)' "$RP"`=**0**；**函数体内**三判决出现顺序为 `EmptyCaseSet` → `InvalidCaseId` → `DuplicateCaseId`（用 `awk '/fn load_guard/{f=1} f{print} f&&/^}/{exit}'` 截取函数体后再 `grep -oE`；**不得**用全文件 `grep -n`，因 73.1 的 `ReplayLoadError` 枚举定义序为 `EmptyCaseSet`/`DuplicateCaseId`/`InvalidCaseId`，会污染行号比较）；`"$CARGO" build -p meeting-copilot-application` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'fn load_guard' "$RP")" = "1" ]
    [ "$(awk '/fn load_guard/{f=1} f{print} f&&/^}/{exit}' "$RP" | grep -oE 'EmptyCaseSet|InvalidCaseId|DuplicateCaseId' | head -3 | tr '\n' ' ')" = "EmptyCaseSet InvalidCaseId DuplicateCaseId " ]
    "$CARGO" build -p meeting-copilot-application
    echo "exit=$?"
    ```
  - 证据产物：三判决顺序写入 `b4-status.md` §23.1

- [ ] **W-B4-74.2 升序稳定排序与 ID 形态判据** · `$RP`
  - 追溯：AC-B4-97 · spec §5.11.1-3、§6.29-1 · design §8.5.1、D-B4-P120
  - 改动内容：`cases.sort_by(|a, b| a.case_id.cmp(b.case_id))`（**稳定**排序，禁止 `sort_unstable_by`）；形态判据按**字节**：`case_id.len() <= 64` 且 `chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')`
  - 禁止：不得二次排序（`failures` 直接按遍历顺序 `push`）；不得用 `HashSet`（迭代序无保证）；不得做 Unicode 归一化
  - DoD：`grep -c 'sort_by' "$RP"` ≥**1**；`grep -c 'sort_unstable_by' "$RP"`=**0**；`grep -c 'is_ascii_alphanumeric' "$RP"`=**1**；`grep -c 'len() <= 64' "$RP"`=**1**
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'sort_by' "$RP")" -ge 1 ]
    [ "$(grep -c 'sort_unstable_by' "$RP")" = "0" ]
    [ "$(grep -c 'is_ascii_alphanumeric' "$RP")" = "1" ]
    echo "exit=$?"
    ```

- [ ] **W-B4-74.3 装载/排序/只读 5 个用例** · `$DRV`
  - 追溯：AC-B4-97、AC-B4-102 · spec §5.11.1-3、§7.16 AC-97 · design §8.6.5-3~6、§8.7.1、D-B4-P111/**P120**
  - 改动内容：新建 `$DRV` 文件头（`//! 13.3 回放本体的 22 个自检用例（0 IO/0 网络/0 随机/不建 tests/replay/）。`）+ 5 个 `#[test]`：
    1. `replay_case_source_rejects_empty_set` → `Err(EmptyCaseSet)`，且 `source.cases()` 二次调用仍原样（零判定、零记录）；
    2. `replay_case_source_rejects_invalid_case_id` → 空 ID / 65 字节 / 含空格三态均 `InvalidCaseId`；
    3. `replay_case_source_rejects_duplicate_ids` → 两个合法同名 `DuplicateCaseId`；"空 ID + 重复"仍报 `InvalidCaseId`（单射）；
    4. `replay_case_set_is_sorted_by_case_id` → 逆序装载后 `failures`/`records` 与升序 `case_id` 对齐；
    5. `replay_driver_load_is_io_free_and_read_only` → 同 source 两次 `cases()` 集合与数量相等，本文件不含 `std::fs`/`File::`/`std::env`/`Instant::now`/`SystemTime::now`/`thread::sleep`
  - 禁止：不得 `mod common;`；不得复制 `tests/gate_replay.rs` 的 `Kind`/`build_cases`/`run`；断言消息只用三段模板；不得 `println!`
  - DoD：5 个用例逐个退 0；`drv_cases`（仅本任务后）≥**5**；`grep -c 'mod common' "$DRV"`=**0**；`grep -cE 'fn build_cases|enum Kind' "$DRV"`=**0**；`grep -cE 'std::fs|File::|std::env|Instant::now|SystemTime::now|thread::sleep' "$DRV"`=**0**（用例 5 的 IO 边界由 shell 判据承担；`include_str!` 自扫在 80.2 才引入）；`--rule=zero-transcript` 退 0
  - 证据：
    ```bash
    set -e
    for c in replay_case_source_rejects_empty_set replay_case_source_rejects_invalid_case_id \
             replay_case_source_rejects_duplicate_ids replay_case_set_is_sorted_by_case_id \
             replay_driver_load_is_io_free_and_read_only; do
      "$CARGO" test -p meeting-copilot-application --test gate_replay_driver "$c" || exit 1
    done
    [ "$(grep -c 'mod common' "$DRV")" = "0" ]
    [ "$(grep -cE 'std::fs|File::|std::env|Instant::now|SystemTime::now|thread::sleep' "$DRV")" = "0" ]
    bash scripts/check-b4-gates.sh --rule=zero-transcript
    echo "exit=$?"
    ```

### W-B4-75 门控驱动与 4 函数调用（AC-B4-98/99）

> 前置：W-B4-73.2（D6 骨架）+ W-B4-74（装载已可靠）。本任务把"骨架可编译"收敛为"**只调 3 函数 + 常量时钟 + 每例 1 条记录**"。

- [ ] **W-B4-75.1 逐例三调用顺序 + 每例新建 `FixedClock::new(now_ms, 0)`** · `$RP`
  - 追溯：AC-B4-98、AC-B4-99 · spec §5.11.1-4、§6.29-3 · design §8.2.4、§8.5.2、D-B4-P112
  - 改动内容：D6 内固定顺序 `detect_gaps(&case.input.scan)` → `evaluate_gate(&case.input, &case.policy, &FixedClock::new(case.input.now_ms, 0))` → `record_from(&case.input, &decision, &case.input.budget)` → 比对 → `records.push(record)`；`FixedClock` 为**每例局部变量**（不跨例共享、不使用 `TestClock`）；`records` 顺序与 `case_id` 升序一致。**职责边界**：73.2 已落"三调用 + 汇总"骨架，本任务只把骨架收敛为 DoD 口径（每例一次调用、每例一个新时钟、`elapsed_ms==0` 由行为断言）并补齐 `detect_gaps` 的处置（75.2），**不重复实现另一套调用链**
  - 禁止：不得把 `FixedClock` 提到 `run` 外层复用（否则 100 次结果漂移）；不得用 `tick_ms != 0`；不得跳过 `detect_gaps`；不得改写 `decision`
  - DoD：`grep -c 'FixedClock::new' "$RP"`=**1**；`grep -cE 'detect_gaps\(' "$RP"`=**1**（import 行无括号，故 1 次调用）；`grep -cE 'evaluate_gate\(' "$RP"`=**1**；`grep -cE 'record_from\(' "$RP"`=**1**；`grep -c 'elapsed_ms' "$RP"`=**0**（行为证据由测试断言，不在源码里"自证"）；`"$CARGO" build` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'FixedClock::new' "$RP")" = "1" ]
    [ "$(grep -cE 'detect_gaps\(' "$RP")" = "1" ]
    [ "$(grep -cE 'evaluate_gate\(' "$RP")" = "1" ]
    [ "$(grep -cE 'record_from\(' "$RP")" = "1" ]
    [ "$(grep -c 'elapsed_ms' "$RP")" = "0" ]
    echo "exit=$?"
    ```

- [ ] **W-B4-75.2 `detect_gaps` 结果处置：仅链首调用 + 上限 `debug_assert`** · `$RP`
  - 追溯：AC-B4-98 · design §8.2.3 注、§8.2.4、D-B4-P123
  - 改动内容：`let gaps = detect_gaps(&case.input.scan); debug_assert!(gaps.len() <= 64);`，**不**把 `gaps` 放入 `ReplayRun`、**不**与 `expected` 比对、**不**产失败码；`score_relevance` 不在驱动内调用
  - 禁止：不得给 `ReplayRun` 加 `gap_counts`/`gaps` 字段；不得用 `gaps` 修正决策；不得 `use crate::gate::score_relevance`
  - DoD：`grep -c 'debug_assert!' "$RP"`=**1**；`grep -c 'gaps.len()' "$RP"`=**1**；`grep -c 'gap_counts\|gaps:' "$RP"`=**0**；`grep -c 'score_relevance' "$RP"`=**0**；`"$CARGO" build` 退 0（`debug_assert` 在 debug 构建生效）
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'debug_assert!' "$RP")" = "1" ]
    [ "$(grep -c 'gaps.len()' "$RP")" = "1" ]
    [ "$(grep -c 'gap_counts\|gaps:' "$RP")" = "0" ]
    [ "$(grep -c 'score_relevance' "$RP")" = "0" ]
    echo "exit=$?"
    ```

- [ ] **W-B4-75.3 AC-98 两用例：4 函数关系 + 常量时钟** · `$DRV`
  - 追溯：AC-B4-98 · spec §5.11.1-5、§7.16 AC-98 · design §8.6.5-7/8、§8.7.1、D-B4-P112/**P123**
  - 改动内容：
    1. `replay_driver_uses_only_four_functions`：同一 `RelevanceInput` 调 `score_relevance` 的结果 == `Allow.candidates[0].relevance`（证明驱动未改写）；把 `policy` 的 `threshold_profile` 改为 `Strict`（阈值 720）使相关性 700 的用例由 `Allow` 变 `Deny{LowRelevance,G2}`（决策随 `evaluate_gate` 变化，非自证）；该 700 场景**另构**局部输入（`RelevanceSnapshot` 四注入值均 700、`delta_ms = 0`，`RelevanceScore` 5 项逐字段相等），不改 24 例主集；
    2. `replay_driver_injects_constant_clock`：对 24 例全绿集逐条断言 `record.per_stage.0[*].elapsed_ms == 0`（5 级全覆盖）
  - 禁止：不得在测试内重算阈值/衰减；不得用 `Instant`/`SystemTime`；断言消息只用 `"case={}"`
  - DoD：2 用例退 0；`bash scripts/check-b4-gates.sh --rule=source` 退 0；`grep -c 'score_relevance' "$DRV"` ≥**1**
  - 证据：
    ```bash
    set -e
    "$CARGO" test -p meeting-copilot-application --test gate_replay_driver replay_driver_uses_only_four_functions
    "$CARGO" test -p meeting-copilot-application --test gate_replay_driver replay_driver_injects_constant_clock
    bash scripts/check-b4-gates.sh --rule=source
    echo "exit=$?"
    ```

### W-B4-76 决策比对与失败报告封闭（AC-B4-100）

> 前置：W-B4-75（每例已有 `decision`）。本任务实现 4 码互斥与汇总不变量，**不碰记录核验**（77）。

- [ ] **W-B4-76.1 私有 `compare_decision`：5 码互斥优先级（码不同永远 `ReasonMismatch`）** · `$RP`
  - 追溯：AC-B4-100 · spec §5.11.1-8、§6.29-6 · design §8.3.2、§8.3.4、D-B4-P121/**P130**
  - 改动内容：`fn compare_decision(expected: &ExpectedDecision, actual: &GateDecision) -> Option<ReplayMismatchCode>`（无 `pub`），固定顺序：`expected=Allow && actual=Deny` → `ExpectedAllowGotDeny`；`expected=Deny && actual=Allow` → `ExpectedDenyGotAllow`；双方 `Deny` 且 `reason` 不同 → `ReasonMismatch`（**码不同永远报此码**）；`reason` 相同但 `stopped_at` 不同 → `StageMismatch`；其余 `None`
  - 禁止：不得叠加两码；不得比较 `per_stage` 数组；不得读记录；`GateStage::G[1-5]` 字面量总数仍 ≤2
  - DoD：`grep -c 'fn compare_decision' "$RP"`=**1**；`grep -cE 'GateStage::G[1-5]' "$RP"` ≤**2**；`grep -cE 'panic!\(|unwrap\(\)' "$RP"`=**0**；`"$CARGO" build` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'fn compare_decision' "$RP")" = "1" ]
    [ "$(grep -cE 'GateStage::G[1-5]' "$RP")" -le 2 ]
    echo "exit=$?"
    ```

- [ ] **W-B4-76.2 汇总不变量与 `mismatch_cases` 逐项一致** · `$RP`
  - 追溯：AC-B4-100 · spec §6.29-4、§5.11.1-8 · design §8.3.5 I-13R-01~04、D-B4-P113/P121
  - 改动内容：D7 汇总 `total = u32::try_from(cases.len()).map_or(u32::MAX, |n| n)`；`mismatched = failures.len() as u32`；`matched = total.saturating_sub(mismatched)`；`mismatch_cases = failures.iter().map(|f| f.case_id.clone()).collect()`（顺序即 `case_id` 升序，不二次排序）
  - 禁止：不得给 `ReplayReport` 加字段；不得按码分组/统计；不得用 `HashMap` 聚合
  - DoD：`grep -c 'mismatch_cases' "$RP"` ≥**1**；`grep -cE 'let mismatched = .*failures\.len\(\)' "$RP"`=**1**；`grep -c 'saturating_sub' "$RP"`=**1**；`grep -c 'BTreeMap\|HashMap' "$RP"`=**0**；`kw_fields ReplayReport crates/domain/src/model/gate_replay.rs`=**5**
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'mismatch_cases' "$RP")" -ge 1 ]
    [ "$(grep -cE 'let mismatched = .*failures\.len\(\)' "$RP")" = "1" ]
    [ "$(grep -c 'saturating_sub' "$RP")" = "1" ]
    [ "$(grep -c 'BTreeMap\|HashMap' "$RP")" = "0" ]
    echo "exit=$?"
    ```

- [ ] **W-B4-76.3 AC-100 三用例：2 项失败 / 5 码覆盖 / 计数一致** · `$DRV`
  - 追溯：AC-B4-100 · spec §5.11.1-8、§7.16 AC-100 · design §8.3.5 I-13R-02~04、§8.6.3、§8.6.5-12~14、D-B4-P126
  - 改动内容：
    1. `replay_failure_reports_case_id_and_code_only`：`report.mismatch_cases == failures.iter().map(|f| f.case_id.clone())`；`format!("{:?}", failure)` 只含 `case_id` 与码名（不含 `expected`/`decision` 结构）；`mismatched == failures.len()`；
    2. `replay_failure_codes_cover_all_mismatch_kinds`：5 例探针集（79.2）→ `failures.len()==5`、码集合与 `ReplayMismatchCode::ALL` 相等、每例恰 1 条；
    3. `replay_driver_reports_totals_are_consistent`：`matched + mismatched == total`、`mismatched == failures.len()`、`total == records.len()`（24 例主集）
  - 禁止：不得在断言消息里拼 `expected`/`decision`；不得为凑 5 码写第 6 个变体；不得修改 `GateRejectReason`
  - DoD：3 用例退 0；`kw_fields ReplayFailure "$RP"`=**2**；`kw_fields ReplayRun "$RP"`=**3**；`kw_variants2 ReplayMismatchCode "$RP"`=**5**
  - 证据：
    ```bash
    set -e
    for c in replay_failure_reports_case_id_and_code_only replay_failure_codes_cover_all_mismatch_kinds \
             replay_driver_reports_totals_are_consistent; do
      "$CARGO" test -p meeting-copilot-application --test gate_replay_driver "$c" || exit 1
    done
    [ "$(kw_fields ReplayFailure "$RP")" = "2" ]
    [ "$(kw_fields ReplayRun "$RP")" = "3" ]
    echo "exit=$?"
    ```

### W-B4-77 记录核验与 `Debug` 白名单（AC-B4-99/102）

> 前置：W-B4-75（每例已有 `record`）。记录核验**只读**，不改 `GateRecord`、不读 `unknown` 私有键。

- [ ] **W-B4-77.1 私有 `record_check`：`schema_version` + 3 谓词（不读 `unknown`）** · `$RP`
  - 追溯：AC-B4-99 · spec §5.11.1-6、§6.29-5 · design §8.2.2 助手表、§8.3.5 I-13R-08、D-B4-P116/**P122**
  - 改动内容：`fn record_check(record: &GateRecord) -> Option<ReplayMismatchCode>`（无 `pub`）：`record.schema_version != 1` → `Some(RecordViolation)`；`!record.refs_are_valid()` → `Some(RecordViolation)`；`!record.skipped_stages_are_valid()` → `Some(RecordViolation)`；`!record.decision_tag_matches_stopped_at()` → `Some(RecordViolation)`；否则 `None`
  - 禁止：不得读 `unknown` / `unknown["gate_record_violation"]`；不得改记录；不得写审计/存储；不得把 `skipped_stages` 为空判成违规（`r24` 跳级时 `skipped_stages` 仍空且合法，design §8.6.4 注）
  - DoD：`grep -c 'fn record_check' "$RP"`=**1**；`grep -c 'gate_record_violation' "$RP"`=**0**；`grep -c 'refs_are_valid\|skipped_stages_are_valid\|decision_tag_matches_stopped_at\|schema_version' "$RP"` ≥**4**；`"$CARGO" build` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'fn record_check' "$RP")" = "1" ]
    [ "$(grep -c 'gate_record_violation' "$RP")" = "0" ]
    [ "$(grep -c 'refs_are_valid\|skipped_stages_are_valid\|decision_tag_matches_stopped_at\|schema_version' "$RP")" -ge 4 ]
    echo "exit=$?"
    ```

- [ ] **W-B4-77.2 `ReplayCase`/`ReplayRun` 手写 `Debug`；`ReplayFailure` 派生** · `$RP`
  - 追溯：AC-B4-102 · spec §5.11.1-9 · design §8.3.3、§8.5.4 L2、D-B4-P129
  - 改动内容：`impl core::fmt::Debug for ReplayCase<'_> { write!(f, "ReplayCase {{ case_id: {} }}", self.case_id) }`；`impl core::fmt::Debug for ReplayRun { write!(f, "ReplayRun {{ report: {:?}, failures: {}, records: {} }}", self.report, self.failures.len(), self.records.len()) }`；`ReplayFailure` 保持 `derive(Debug, ...)`
  - 禁止：不得让 `ReplayRun::Debug` 展开 `records`（`GateRecord::Debug` 会带 `per_stage`/`trace_id`）；不得 `derive(Debug)` 到 `ReplayCase`/`ReplayRun`；不得实现 `Serialize`
  - DoD：`grep -c 'impl core::fmt::Debug for Replay' "$RP"`=**2**；`grep -B3 'pub struct ReplayCase' "$RP" | grep -c 'Debug'`=**0**；`grep -B3 'pub struct ReplayRun' "$RP" | grep -c 'Debug'`=**0**；`grep -c 'Serialize' "$RP"`=**0**
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'impl core::fmt::Debug for Replay' "$RP")" = "2" ]
    [ "$(grep -B3 'pub struct ReplayCase' "$RP" | grep -c 'Debug')" = "0" ]
    [ "$(grep -B3 'pub struct ReplayRun' "$RP" | grep -c 'Debug')" = "0" ]
    [ "$(grep -c 'Serialize' "$RP")" = "0" ]
    echo "exit=$?"
    ```

- [ ] **W-B4-77.3 AC-99/102 五用例：记录封闭、违规可达、跳过级违规、`Debug` 白名单、零转录边界** · `$DRV`
  - 追溯：AC-B4-99、AC-B4-102 · spec §5.11.1-6/9、§7.16 AC-99/102 · design §8.3.5 I-13R-01/08/10、§8.6.3 p05、§8.6.5-9~11/18~19
  - 改动内容：
    1. `replay_driver_records_are_closed`：24 例主集 `records.len()==report.total`；逐条 `schema_version==1` 且 3 谓词为真；`format!("{:?}", record)` 不含 `"内部示例文本-13r"`（唯一 arena 文本）；
    2. `replay_driver_record_violation_is_reported`：探针 `p05-record-refs-empty`（`candidate_ref=None` 且 `insight_ref=None`）→ `failures == [RecordViolation]`，且该例记录仍入 `records`（不丢弃）；
    3. `replay_record_check_rejects_illegal_skip_stage`（**X-26 等价证明**）：在 `$DRV` 内合成两条 `GateRecord`——① `skipped_stages=[G4]`，② `decision=Deny` 且 `stopped_at=None`；分别断言 `record.skipped_stages_are_valid()==false`、`record.decision_tag_matches_stopped_at()==false`（这两个 **pub 谓词**正是 `record_check` 的判定输入），并由 `--rule=source` 退 0 + `grep -c 'skipped_stages_are_valid'`/`grep -c 'decision_tag_matches_stopped_at'` ≥1 证明 `$RP` 的 `record_check` 确实消费它们；**不直接调用** `record_check`（`$RP` 内私有 `fn` 对集成测试 crate 不可见；把它升为第 8 个公开项会破坏 AC-95 的 7/0）；
    4. `replay_driver_debug_is_whitelisted`：`format!("{:?}", case)` 只含 `case_id`；`format!("{:?}", run)` 含 `failures: N`/`records: N` 且不含 arena 文本；`format!("{:?}", failure)` 只 2 项；
    5. `replay_driver_zero_transcript_boundary`：本文件不含协议前缀/`sk-` 形态/鉴权头/模型名前缀/`request_id:`
  - 禁止：不得在断言里输出记录/文本；不得改 `GateRecord`；不得为 `record_check` 单测新建第 8 类型
  - DoD：5 用例退 0；`format!("{:?}", run)` 禁词 0；`--rule=zero-transcript` 退 0；`grep -c 'skipped_stages_are_valid' "$RP"` ≥**1**、`grep -c 'decision_tag_matches_stopped_at' "$RP"` ≥**1**（X-26 的静态调用证据）；`kw_fields GateRecord`=15（剔除 `unknown`=14）
  - 证据：
    ```bash
    set -e
    for c in replay_driver_records_are_closed replay_driver_record_violation_is_reported \
             replay_record_check_rejects_illegal_skip_stage replay_driver_debug_is_whitelisted \
             replay_driver_zero_transcript_boundary; do
      "$CARGO" test -p meeting-copilot-application --test gate_replay_driver "$c" || exit 1
    done
    [ "$(grep -c 'skipped_stages_are_valid' "$RP")" -ge 1 ]
    [ "$(grep -c 'decision_tag_matches_stopped_at' "$RP")" -ge 1 ]
    bash scripts/check-b4-gates.sh --rule=zero-transcript
    echo "exit=$?"
    ```

### W-B4-78 确定性：两次 / 乱序 / 100 次（AC-B4-101）

> 前置：W-B4-76/77（比对与核验已定稿）。**不新增依赖**：确定性只用 `PartialEq` 逐项相等（D-B4-P124）。

- [ ] **W-B4-78.1 `seed` 只写 `report.seed`，不参与排序/抽样** · `$RP`
  - 追溯：AC-B4-101 · spec §5.11.1-7、§6.29-2 · design §8.5.3、D-B4-P114
  - 改动内容：D0 取 `seed` 后只在 D7 写入 `report.seed`；排序、筛选、`matched` 计算均不读 `seed`
  - 禁止：不得用 `seed` 打乱顺序；不得 `rand`/`getrandom`；不得把 `seed` 写进 `ReplayFailure`/`GateRecord`
  - DoD：`grep -c 'seed' "$RP"` ≥**2**；`grep -cE 'rand::|getrandom|thread_rng|shuffle' "$RP" "$DRV"`=**0**；`"$CARGO" build` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'seed' "$RP")" -ge 2 ]
    [ "$(grep -cE 'rand::|getrandom|thread_rng|shuffle' "$RP" "$DRV" | awk -F: '{s+=$2} END {print s}')" = "0" ]
    echo "exit=$?"
    ```

- [ ] **W-B4-78.2 两次/乱序/100 次的实现前提：排序后执行 + 每例新时钟** · `$RP`
  - 追溯：AC-B4-101 · spec §5.11.3-5 · design §8.5.1、§8.5.3、D-B4-P114/P120
  - 改动内容：确认 D5 排序在 D6 之前（乱序无关）；每例 `FixedClock` 局部新建（两次/100 次无关）；`failures` 顺序只由遍历顺序决定
  - 禁止：不得缓存上一次运行的中间结果；不得用全局静态/`OnceLock`；不得在 `run` 内保留跨调用状态
  - DoD：`grep -cE 'static mut|OnceLock|lazy_static|thread_local' "$RP"`=**0**；`grep -c 'Cell<' "$RP"`=**0**（时钟状态只在 `FixedClock` 内）；`"$CARGO" build` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -cE 'static mut|OnceLock|lazy_static|thread_local' "$RP")" = "0" ]
    [ "$(grep -c 'Cell<' "$RP")" = "0" ]
    echo "exit=$?"
    ```

- [ ] **W-B4-78.3 AC-101 三用例（含 100 次段与 X-17 用例名）** · `$DRV`
  - 追溯：AC-B4-101 · spec §5.11.1-7、§7.16 AC-101 · design §8.5.3、§8.6.5-15~17、**X-17**
  - 改动内容：
    1. `replay_driver_is_deterministic_twice`：同一 source 连续 `run` 两次 → `first.report == second.report`、`first.failures == second.failures`、`first.records == second.records`；再循环 **100** 次以第 1 次为基准，断言消息 `"iter={}", iter`；
    2. `replay_driver_is_order_insensitive`（**X-17 唯一名**）：source A 正序、source B 逆序（同一 arena、同一 `seed`）→ 三者逐项相等；
    3. `replay_driver_seed_is_report_only`：同一 cases，`seed=13` vs `seed=29` → 仅 `report.seed` 不同，其余 4 字段与 `failures`/`records` 相等
  - 禁止：不得比较 `Debug` 字符串（白名单只承载计数）；不得构造随机顺序（逆序即 `(0..n).rev()`）；不得另建 `replay_driver_order_insensitive`（X-17）
  - DoD：3 用例退 0；`grep -c 'replay_driver_order_insensitive' "$DRV"`=**0**；`grep -c 'replay_driver_is_order_insensitive' "$DRV"`=**1**；`drv_list_count` ≥**20**
  - 证据：
    ```bash
    set -e
    for c in replay_driver_is_deterministic_twice replay_driver_is_order_insensitive replay_driver_seed_is_report_only; do
      "$CARGO" test -p meeting-copilot-application --test gate_replay_driver "$c" || exit 1
    done
    [ "$(grep -c 'replay_driver_order_insensitive' "$DRV")" = "0" ]
    [ "$(drv_list_count)" -ge 20 ]
    echo "exit=$?"
    ```

### W-B4-79 自检集 `Arena` 与双层数据集（AC-B4-96/103）

> 前置：W-B4-73.3（`pub use` 生效）+ W-B4-78（数据集断言口径已定）。**禁止** `mod common`、**禁止**复刻 200 例构造器。

- [ ] **W-B4-79.1 本地 `Arena` + `Source` + `FailingSanitizer`（不复用 `tests/common`）** · `$DRV`
  - 追溯：AC-B4-96、AC-B4-103 · spec §5.11.1-2/10 · design §8.6.4、§8.4.4 并存三纪律、D-B4-P119/**P127**
  - 改动内容：`struct Arena` 持有基准形态全部借用源：`session-13r`/`trace-13r`/1 条 `NormalizedFragment`（文本 `"内部示例文本-13r"`、置信 0.90、span `0..100`、单通道）/`lexicon_names = ["内部示例文本-13r"]`（**与片段文本同串**，`text.contains(name)` 才产候选）/`explained_terms = []`/`acronym_whitelist = []`/`sensitive_terms = []`（基准必须为空，`r11` 唯一设 `["内部示例文本-13r"]`）/`question_wait_ms = 10_000`/`stable_prefix_ms = {u1: 0}`/`answered_at_ms = {}`/空 `insights`/`GatePolicy::default()`/`EffectivePolicy::fail_closed()` 后置 `auto_suggestion = Enabled`、`consent = Explicit`（`screen_share_protection_default` 保持默认 `true`，与 `r05` 的 `screen_share=true` 配合产生 `Deny{ScreenShareProtection,G1}`）/G1 输入基准：`auto_suggestion_enabled=true`、`consent_confirmed=true`、`session_state = SessionState::Listening`（**枚举无 `Active` 变体**）、`silent_period=false`、`screen_share=false`、`session_suppressed=false`、`duplicate_detected=false`、`trigger=Auto`（`r01`~`r12`/`r23` 各只改其中 1 项）/`GateAuthorization{status: Granted, granted_at_ms: Some(0), content_types: vec![GateContentType::TextQuery], max_query_chars: Some(200), provider_ids: vec![ProviderId::new("provider-1")...], notice_version: 1, revoked_at_ms: None}`（**7 字段齐备**；`provider_ids` 同样是 `Vec<ProviderId>`，须经 `ProviderId::new`；缺 `granted_at_ms` 会使 `is_complete()` 为 false）/`ProviderView{enabled_count: 1, provider_ids: vec![ProviderId::new("provider-1")...]}`（`ProviderId` 无 `From<&str>`，须经 `ProviderId::new(...)`；测试内允许 `expect`/`map_err`，`$RP` 内不涉及；必须与授权白名单同集）/`LlmCapabilityView{cooling: false, cooling_until_ms: None}`（该结构**无 `Default`**，两字段必须都写；`r16` 只把 `cooling` 改 `true`）/`NetworkState::Online`/`GateBudget{session_outbound_used: 0, global_window_used: 0, topic_window_used: 空, entity_last_seen_ms: 空, inflight: 0, tokens_milli: 3000, window_start_ms: 0}`（**7 字段逐项写全**，该结构无 `Default`；除 `tokens_milli=3000` 外其余 6 项为 0/空）/`GateSignals{relevance: RelevanceSnapshot{四值 1000, delta_ms:0}（5 子字段）, 其余 `similarity_milli`/`duplicate_last_seen_ms`/`cluster_id`/`entity_keys`/`suppression_topics`/`suppression_providers`/`suppression_entities`/`cache_age_ms`/`refresh_available` 共 9 项用 `..GateSignals::default()`（合计 10 字段）}/`PassthroughSanitizer`/`NoopSink`/`FailingSanitizer`/`query_seed = "内部示例文本-13r"`/`now_ms = 10_000`；`fn case<'a>(&'a self, id: &'a str, policy: GatePolicy, expected: ExpectedDecision, mutate: impl FnOnce(&mut GateInput<'a>)) -> ReplayCase<'a>`（`mutate` 只改值与切片指向，不改 arena 本身）；`struct Source<'a> { arena: &'a Arena, order: &'a [usize], seed: u64 }` + `impl ReplayCaseSource`（`cases()` 内先按 `order` 索引 `build_self_check`/`build_probe` 的结果，再**逐项重借用** `ReplayCase { case_id, input: c.input.clone(), policy: c.policy.clone(), expected: c.expected }`——`Vec<T>` 不保证可把 `Vec<ReplayCase<'a>>` 直接当 `Vec<ReplayCase<'_>>` 返回，禁止 `transmute`/`unsafe`）；`struct FailingSanitizer` 本地实现 `Sanitizer` 4 方法（仅 `r19` 用，`sanitize_query` 返回 `Err`）；**变体载体（Arena 必须额外持有，否则 24 例中借用型字段无法替换）**：`fragments_low_confidence: Vec<NormalizedFragment>`（0.49 置信，`r06` 同时替换 `input.fragments` 与 `input.scan.fragments`，保持同源）、`sensitive_terms_nonempty: Vec<String>`（`r11` 用，值 = `["内部示例文本-13r"]`）、`authorization_revoked: GateAuthorization`（`r18`，`status=Revoked`，其余 6 字段同基准）、`authorization_blocked: GateAuthorization`（`r21`，`provider_ids=vec![ProviderId::new("provider-2")]`，其余 6 字段同基准）
  - 禁止：不得 `mod common;`/`use common::Harness`；不得 `use meeting_copilot_application::tests::`；不得复制 `tests/gate_replay.rs` 的 `Kind`/`build_cases`/`run`/`FailingSanitizer` 命名（本地实现可同名但**不得** `include!` 他文件）；不得 `std::fs`
  - DoD：`grep -c 'mod common' "$DRV"`=**0**；`grep -c 'struct Arena' "$DRV"`=**1**；`grep -c 'struct FailingSanitizer' "$DRV"`=**1**；`grep -c 'impl ReplayCaseSource for' "$DRV"` ≥**1**；`grep -c 'EffectivePolicy::fail_closed' "$DRV"`=**1**（基准 arena 必含 `effective_policy`）；`grep -c 'GateSignals' "$DRV"` ≥**1**（全 10 字段，可 `..GateSignals::default()`）；`grep -c 'SessionState::Listening' "$DRV"`=**1**（**实况无 `Active` 变体**，X-24）；`grep -c 'granted_at_ms' "$DRV"`=**1**（授权 7 字段齐备）；`grep -c 'sensitive_terms' "$DRV"` ≥**1**（基准空，`r11` 设同串）；`"$CARGO" test -p meeting-copilot-application --test gate_replay_driver --no-run` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'mod common' "$DRV")" = "0" ]
    [ "$(grep -c 'struct Arena' "$DRV")" = "1" ]
    [ "$(grep -c 'struct FailingSanitizer' "$DRV")" = "1" ]
    [ "$(grep -c 'EffectivePolicy::fail_closed' "$DRV")" = "1" ]
    [ "$(grep -c 'GateSignals' "$DRV")" -ge 1 ]
    [ "$(grep -c 'SessionState::Listening' "$DRV")" = "1" ]
    [ "$(grep -c 'granted_at_ms' "$DRV")" = "1" ]
    [ "$(grep -c 'sensitive_terms' "$DRV")" -ge 1 ]
    "$CARGO" test -p meeting-copilot-application --test gate_replay_driver --no-run
    echo "exit=$?"
    ```

- [ ] **W-B4-79.2 `build_self_check()` 24 例 + `build_probe()` 5 例（逐字 `case_id`）** · `$DRV`
  - 追溯：AC-B4-96、AC-B4-100、AC-B4-103 · spec §5.11.1-10 · design §8.6.1、§8.6.2、§8.6.3、D-B4-P126
  - 改动内容：
    - `fn build_self_check(arena: &Arena) -> Vec<ReplayCase<'_>>`：**24** 例 `r01-g1-auto-disabled` … `r24-allow-skip-g3`，逐例只改 1~2 项输入（G1×12 / G2×1 / G3×1 / G4×3 / G5×4 / Allow×3）；`r24` 的 `policy.dedupe_enabled=false` 且 `record_from.skipped_stages` 仍空（合法）。**逐例差异以 design §8.6.2 表为准**，其中借用型字段的替换口径：`r06` → `Arena.fragments_low_confidence`（同时换 `input.fragments`/`input.scan.fragments`）、`r11` → `Arena.sensitive_terms_nonempty`、`r18` → `Arena.authorization_revoked`、`r21` → `Arena.authorization_blocked`（授权 `provider_ids=["provider-2"]`，`provider.provider_ids` 保持 `["provider-1"]`，令 `provider_scope_valid()` 失败 → `Deny{ProviderNotAllowed,G5}`）；值型字段（`budget`/`llm`/`network`/`trigger`/各布尔）直接在 `mutate` 内改；`r12` 改的是 `ReplayCase.policy`（`session_outbound_cap=601`），不是 `GateInput.policy`；
    - `fn build_probe(arena: &Arena) -> Vec<ReplayCase<'_>>`：**5** 例 `p01-expect-allow-got-deny`/`p02-expect-deny-got-allow`/`p03-reason-differs`/`p04-stage-differs`/`p05-record-refs-empty`（故意写错 `expected` 或留空 refs）
  - 禁止：不得 `fn build_cases`（200 例私有名）；不得 `enum Kind`；不得把 5 探针混入 24 主集（否则 `mismatched>0` 失去主集回归价值）；不得生成 >24/≥5 的额外用例
  - DoD：`grep -c 'fn build_self_check' "$DRV"`=**1**；`grep -c 'fn build_probe' "$DRV"`=**1**；`grep -cE '"r[0-9]{2}-' "$DRV"`=**24**；`grep -cE '"p0[1-5]-' "$DRV"`=**5**；`grep -cE 'fn build_cases|enum Kind' "$DRV"`=**0**
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'fn build_self_check' "$DRV")" = "1" ]
    [ "$(grep -c 'fn build_probe' "$DRV")" = "1" ]
    [ "$(grep -cE '"r[0-9]{2}-' "$DRV")" = "24" ]
    [ "$(grep -cE '"p0[1-5]-' "$DRV")" = "5" ]
    echo "exit=$?"
    ```

- [ ] **W-B4-79.3 两个数据集被既有用例消费（不新增第 23 个函数）** · `$DRV`
  - 追溯：AC-B4-100、AC-B4-103 · design §8.6.1、D-B4-P126（22 函数固定）
  - 改动内容：24 例主集供 `replay_driver_records_are_closed`/`replay_driver_injects_constant_clock`/`replay_driver_reports_totals_are_consistent`/`replay_driver_is_deterministic_twice` 使用；5 例探针集供 `replay_failure_codes_cover_all_mismatch_kinds`/`replay_driver_record_violation_is_reported` 使用；**不**新增独立"数据集自检"测试函数
  - 禁止：不得新增第 23 个 `#[test]`；不得用 `#[ignore]` 挂起任何数据集
  - DoD：`grep -c 'build_self_check' "$DRV"` ≥**2**；`grep -c 'build_probe' "$DRV"` ≥**2**；`drv_cases`=**22**（最终口径，由 W-B4-80.1 收口）；主集断言 `total=24`/`matched=24`/`mismatched=0`/`seed=13` 与探针 `total=5`/`matched=0`/`mismatched=5` 在用例内成立
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'build_self_check' "$DRV")" -ge 2 ]
    [ "$(grep -c 'build_probe' "$DRV")" -ge 2 ]
    "$CARGO" test -p meeting-copilot-application --test gate_replay_driver replay_failure_codes_cover_all_mismatch_kinds
    echo "exit=$?"
    ```

### W-B4-80 22 个用例收敛与自扫取证（AC-B4-95~103）

> 前置：W-B4-79（两个数据集就绪）。本任务只补齐剩余用例函数并做自扫，**不改 `$RP` 的公开面**。

- [ ] **W-B4-80.1 22 个 `#[test]` 齐备（逐字，见 P9 映射表）** · `$DRV`
  - 追溯：AC-B4-95、AC-B4-96、AC-B4-98、AC-B4-103 · design §8.6.5、§8.7.1、D-B4-P126
  - 改动内容：补齐 79/78/77/76/74 未覆盖的 `replay_driver_types_are_closed`（7 类型可用 + `ALL.len()` 5/3 + `ReplayDriver::default()`）、`replay_case_layers_are_separated`（`ReplayCase` 4 项 + `GateInputFixture` 23 + 本文件不引用 `GateInputFixture`/`from_fixture`）、`replay_driver_coexists_with_gate_replay_fixture`（`GateReplayCase` 4 字段 + `ReplayReport` 5 字段均可构造、本 target 独立编译）、`replay_driver_does_not_touch_200_cases`（`include_str!` 自扫，见 80.2）等，使总数**恰 22**
  - 禁止：不得出现第 23 个 `#[test]`；不得 `#[ignore]`；不得改名（X-17 只允许 `replay_driver_is_order_insensitive`）
  - DoD：`drv_cases`=**22**；`drv_list_count`=**22**（≥20）；`grep -c '#\[ignore' "$DRV"`=**0**；`grep -c 'replay_driver_order_insensitive' "$DRV"`=**0**
  - 证据：
    ```bash
    set -e
    [ "$(drv_cases)" = "22" ]
    [ "$(drv_list_count)" = "22" ]
    [ "$(grep -c '#\[ignore' "$DRV")" = "0" ]
    echo "exit=$?"
    ```
  - 证据产物：22/22 与用例名清单写入 `b4-status.md` §23.1

- [ ] **W-B4-80.2 `include_str!` 自扫：证明未复刻 200 例构造器** · `$DRV`
  - 追溯：AC-B4-103 · design §8.6.4 自扫证据、§8.6.5-21、D-B4-P110/**P127**
  - 改动内容：在 `replay_driver_does_not_touch_200_cases` 内 `const SELF_SRC: &str = include_str!("gate_replay_driver.rs");`，断言 `SELF_SRC` 不含 `"fn build_cases"`/`"enum Kind"`/`"mod common"`/`"gate_replay::"`/`"use common"`；同用例再断言 `GateReplayCase` 4 字段（`case_id`/`input_fixture`/`policy_fixture`/`expected`）与 `ReplayReport` 5 字段（`total`/`matched`/`mismatched`/`mismatch_cases`/`seed`）**均可构造**。构造口径：`GateInputFixture` 及其 8 个子夹具**无 `Default`**（仅 `SuppressionFixture` 有），须逐字段给出基准值（`clock`/`fragments`/`insights`/`budget`/`authorization`/`relevance`/`suppression`/`candidate_seeds`），构造后立即 `drop` 或不绑定；`ReplayReport` 直接逐字段构造
  - 禁止：不得 `std::fs::read_to_string`（运行期 IO）；不得读 `target/`；`include_str!` 路径只能是同目录文件名
  - DoD：`grep -c 'include_str!' "$DRV"`=**1**；`grep -c 'fn build_cases' "$DRV"`=**0**；`grep -c 'enum Kind' "$DRV"`=**0**；用例退 0；`--rule=source` 退 0（`include_str!` 不触发 `std::fs`）
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'include_str!' "$DRV")" = "1" ]
    [ "$(grep -c 'fn build_cases' "$DRV")" = "0" ]
    [ "$(grep -c 'enum Kind' "$DRV")" = "0" ]
    "$CARGO" test -p meeting-copilot-application --test gate_replay_driver replay_driver_does_not_touch_200_cases
    echo "exit=$?"
    ```

- [ ] **W-B4-80.3 断言消息与样本零转录复核** · `$DRV`
  - 追溯：AC-B4-102 · spec §5.11.1-9、§7.16 AC-102 · design §8.5.4 L3/L4、**X-18**
  - 改动内容：逐条检查 `assert!`/`assert_eq!` 消息只用三段模板；arena 文本统一为唯一恒串 `"内部示例文本-13r"`；`case_id` 只用 `r01-`/`p01-` 形态；不含真实姓名/URL/查询原文
  - 禁止：不得在消息里拼 `expected`/`decision`/记录内容；不得出现协议前缀/`sk-` 形态/鉴权头/模型名前缀/`request_id:`；不得 `println!`/`eprintln!`/`dbg!`
  - DoD：`grep -cE 'https?://|sk-[A-Za-z0-9]{8,}|Authorization:[[:space:]]*Bearer|Bearer [A-Za-z0-9._-]{8,}|(deepseek|gpt-4|gpt-3|claude)-[A-Za-z0-9._-]+|request_id[[:space:]]*[:=]' "$DRV"`=**0**（**与 `scripts/check-b4-gates.sh` 的 `rule_zero_transcript` 正则逐字一致**，比旧口径多覆盖 `http://`、`Bearer <token>`、`request_id=`）；`grep -cE 'println!|eprintln!|dbg!' "$DRV"`=**0**；`--rule=zero-transcript` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -cE 'https?://|sk-[A-Za-z0-9]{8,}|Authorization:[[:space:]]*Bearer|Bearer [A-Za-z0-9._-]{8,}|(deepseek|gpt-4|gpt-3|claude)-[A-Za-z0-9._-]+|request_id[[:space:]]*[:=]' "$DRV")" = "0" ]
    [ "$(grep -cE 'println!|eprintln!|dbg!' "$DRV")" = "0" ]
    bash scripts/check-b4-gates.sh --rule=zero-transcript
    echo "exit=$?"
    ```

### W-B4-81 target 清单、门禁与状态收口（AC-B4-104）

> 前置：73~80 全绿。**`scripts/check-b4-gates.sh` 零改动**（D-B4-P115/P128）；token 必须与 73~80 同批提交。**提交口径**：81.1~81.4 全绿后，P9 的 5 个文件面（`gate/replay.rs`、`gate/mod.rs`、`tests/gate_replay_driver.rs`、`scripts/test-name-inventory.sh`、`docs/progress/b4-status.md`）一次性提交为**单个提交**（不得把 token 或进度表单独先提交；73.0 的文档闸门提交不算 P9 代码提交）。本任务执行后 P9 关闭。

- [ ] **W-B4-81.1 `NEW_TARGETS` 24 → 25（尾部追加 `gate_replay_driver`）** · `scripts/test-name-inventory.sh`
  - 追溯：AC-B4-104 · spec §5.11.1-11、§7.16 AC-104 · design §8.8.2、D-B4-P115/**P128**
  - 改动内容：`NEW_TARGETS='… llm_settings_view'` 尾部追加 ` gate_replay_driver`（顺序在 `llm_settings_view` 之后）
  - 禁止：不得改既有 24 个 token 的顺序/拼写；不得把 `gate_replay_driver` 写进 `scripts/check-b4-gates.sh`；`--diff` 语义不变
  - DoD：`new_targets | wc -w`=**25**；`new_targets | tr ' ' '\n' | grep -qx gate_replay_driver` 退 0；`grep -c 'gate_replay_driver' scripts/test-name-inventory.sh`=**1**；`git diff --numstat 1b0dbf7^ 1b0dbf7 -- scripts/test-name-inventory.sh` = **`1 1`**（单行内容替换：1 增 1 删；**X-30**：提交后相对 HEAD 恒为 `0 0`，不可判定，故基线前移）；`--diff` 输出 `removed=0`
  - 证据：
    ```bash
    set -e
    [ "$(new_targets | wc -w | tr -d ' ')" = "25" ]
    new_targets | tr ' ' '\n' | grep -qx gate_replay_driver
    [ "$(grep -c 'gate_replay_driver' scripts/test-name-inventory.sh)" = "1" ]
    [ "$(git diff --numstat 1b0dbf7^ 1b0dbf7 -- scripts/test-name-inventory.sh | awk '{print $1" "$2}')" = "1 1" ]
    bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
    echo "exit=$?"
    ```

- [ ] **W-B4-81.2 脚本零 diff + `--rule=targets` 仍 7 项 + 七规则** · `scripts/check-b4-gates.sh`（零改动）、仓库根
  - 追溯：AC-B4-104 · spec §5.11.1-11 · design §8.8.1、§8.8.4、**X-05**、D-B4-P115
  - 改动内容：**零改动**；仅复核 `gate_files()`/`core_files()`/`rule_zero_transcript()` 的既有 glob 已覆盖 `gate/replay.rs` 与 `tests/gate_replay_driver.rs`
  - 禁止：不得把 `gate_replay_driver` 加入 `rule_targets()`；不得新增第 8 条规则；不得改退出码语义（0/1/2）；不得改 `rule_source` 模式串
  - DoD：`git diff --quiet HEAD -- scripts/check-b4-gates.sh` 退 **0**；`grep -c 'gate_replay_driver' scripts/check-b4-gates.sh`=**0**；`awk '/^rule_targets\(\)/{f=1} f&&/^\}/{exit} f' scripts/check-b4-gates.sh | grep -c 'gate_replay_driver'`=**0**；`-z "$(bash scripts/check-b4-gates.sh --rule=targets)"` 为真；七规则 7×0
  - 证据：
    ```bash
    set -e
    git diff --quiet HEAD -- scripts/check-b4-gates.sh
    [ "$(grep -c 'gate_replay_driver' scripts/check-b4-gates.sh)" = "0" ]
    [ "$(awk '/^rule_targets\(\)/{f=1} f&&/^\}/{exit} f' scripts/check-b4-gates.sh | grep -c 'gate_replay_driver')" = "0" ]
    for r in source budget record outbound zero-transcript targets doc-plan; do
      bash scripts/check-b4-gates.sh --rule="$r" || exit 1
    done
    echo "exit=$?"
    ```

- [ ] **W-B4-81.3 零 diff 矩阵 + 13 项封闭面 + 全量回归** · 仓库根
  - 追溯：AC-B4-95、AC-B4-103、AC-B4-104 · spec §5.11.1-1/10/12 · design §8.8.4 串联命令、§8.1.4、D-B4-P117/P122
  - 改动内容：只跑判据；任一失败回到 73~80 修正
  - 禁止：不得为过判据改 `api.rs`/`domain/**`/`cards/**`/`ui-bridge/**`/`infrastructure/**`/`tests/gate_replay.rs`/`Cargo.*`
  - DoD：`grep -c '^pub fn ' "$GATE/api.rs"`=**4**；`git diff --numstat HEAD -- "$GATE_MOD"`=`1 0`；`gate_12_zero` 退 0；`git diff --quiet HEAD -- crates/application/src/cards crates/ui-bridge crates/infrastructure crates/domain/src/model` 退 0；`git diff --quiet HEAD -- crates/application/tests/gate_replay.rs Cargo.toml Cargo.lock crates/application/Cargo.toml` 退 0；`test ! -d crates/application/tests/replay` 退 0；`--rule=record` 退 0（14/7/23/5/32/23）；`cargo test --workspace` `failed=0`/`ignored=9`；`--diff` `removed=0`；`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`check-layering.sh`、`scan-secrets.sh`、`cargo deny check` 全退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c '^pub fn ' "$GATE/api.rs")" = "4" ]
    [ "$(git diff --numstat HEAD -- "$GATE_MOD" | awk '{print $1" "$2}')" = "1 0" ]
    gate_12_zero
    git diff --quiet HEAD -- crates/application/src/cards crates/ui-bridge crates/infrastructure crates/domain/src/model
    git diff --quiet HEAD -- crates/application/tests/gate_replay.rs Cargo.toml Cargo.lock crates/application/Cargo.toml
    test ! -d crates/application/tests/replay
    bash scripts/check-b4-gates.sh --rule=record
    "$CARGO" test -p meeting-copilot-application --test gate_replay
    "$CARGO" test -p meeting-copilot-application --test gate_replay_driver
    "$CARGO" fmt --all -- --check
    "$CARGO" clippy --workspace --all-targets -- -D warnings
    bash scripts/check-layering.sh
    bash scripts/scan-secrets.sh
    "$CARGO" deny check
    "$CARGO" test --workspace
    bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
    echo "exit=$?"
    ```
  - 证据产物：全部退出码 + 计数（22/25/4/7/14/23/5/32/11/10/8）写入 `b4-status.md` §23.1

- [ ] **W-B4-81.4 `b4-status.md` §23 + AC-B4-95~104 三态 + U-06 关闭** · `docs/progress/b4-status.md`
  - 追溯：AC-B4-104 · spec §7.16 AC-104 · design §8.7.3、§8.8.5-7、D-B4-P117
  - 改动内容：追加 `## §23 P9 增量快照（W-B4-73 ~ W-B4-81）`（**X-22 已裁决：P9 用 §23**，因 `b4-status.md` 的 `## §21` 已被 CI cargo-deny 需求登记占用、`## §22` 已被 CI 修复快照占用）：§23.1 命令结果表（22 用例、12 target、7 规则、`NEW_TARGETS`=25、`api.rs`=4、7/0/4/3/2/5/3、`gate/mod.rs` `1 0`、`kw_fields` 15→14、全量 `cargo test --workspace`、`--diff` `removed=0`）；§23.2 十行 AC 三态（AC-B4-95~104 → `passed`，各附用例名）；§23.3 **U-06 关闭登记**（**保留** §10.1 原文"规格已定，待实现"，行末追加"，已于 P9 closed（作废 U-06 open，见 §23）"；§20 结论段追加"→ 已实现（§23）"，**不删除**原文；U-07/U-08 保持 `open`）；§23.4 偏差登记 **X-17~X-26 全量 10 项**（X-17 用例名以 spec 为准、X-18 `--rule=source` 全文扫描、X-19 `GateRecord` 双口径 `kw_fields`=15/剔除 `unknown`=14、X-20 `effective_policy` 补项、X-21 表述错位、X-22 P9 快照用 §23、X-23 G1 8 项基准、X-24 基准 arena 实况补正、X-25 判据精确化、X-26 判据可执行性）及 **D-B4-P108~P117 与 D-B4-P118~P130 同时登记**（P108~P117 为 §20.2 已登记项的实现态回填，P118~P130 为 P9 实施期新增裁决）；§23.5 不宣称清单（离线通过 ≠ 分组报表/真实端点时延/6s/10s p95/误触发率/产品级回放平台；不宣称 G3/G4/真机）
  - 禁止：§23 内不得写任何 Key 片段/模型名/域名/请求标识/查询原文；只写 AC 编号、用例名、枚举、计数、毫值；**不得**改 U-07/U-08 的 open 状态；**不得**删除 §10.1/§20 的"规格已定"原文；**不得**占用/改名既有 §21、§22
  - DoD：`grep -c '^## §23' docs/progress/b4-status.md`=**1**；`sed -n '/^## §23/,$p' docs/progress/b4-status.md | grep -cE 'AC-B4-(9[5-9]|10[0-4])'` ≥**10**；`grep -c 'U-06.*回放本体' docs/progress/b4-status.md` ≥**1**；`grep -c 'U-06.*规格已定' docs/progress/b4-status.md` ≥**1**；`grep -c 'U-06.*closed' docs/progress/b4-status.md` ≥**1**；`grep -cE 'U-0[78].*open' docs/progress/b4-status.md` ≥**2**；`grep -c 'X-17' docs/progress/b4-status.md` ≥**1**；`sed -n '/^## §23/,$p' docs/progress/b4-status.md | grep -c 'D-B4-P108'` ≥**1**；`sed -n '/^## §23/,$p' docs/progress/b4-status.md | grep -c 'D-B4-P130'` ≥**1**；`--rule=doc-plan` 退 **0**；`--rule=zero-transcript` 退 **0**
  - 证据：
    ```bash
    set -e
    [ "$(grep -c '^## §23' docs/progress/b4-status.md)" = "1" ]
    [ "$(sed -n '/^## §23/,$p' docs/progress/b4-status.md | grep -cE 'AC-B4-(9[5-9]|10[0-4])')" -ge 10 ]
    [ "$(grep -c 'U-06.*回放本体' docs/progress/b4-status.md)" -ge 1 ]
    [ "$(grep -c 'U-06.*规格已定' docs/progress/b4-status.md)" -ge 1 ]
    [ "$(grep -c 'U-06.*closed' docs/progress/b4-status.md)" -ge 1 ]
    [ "$(grep -cE 'U-0[78].*open' docs/progress/b4-status.md)" -ge 2 ]
    [ "$(sed -n '/^## §23/,$p' docs/progress/b4-status.md | grep -c 'D-B4-P108')" -ge 1 ]
    [ "$(sed -n '/^## §23/,$p' docs/progress/b4-status.md | grep -c 'D-B4-P130')" -ge 1 ]
    bash scripts/check-b4-gates.sh --rule=doc-plan
    bash scripts/check-b4-gates.sh --rule=zero-transcript
    echo "exit=$?"
    ```
  - 证据产物：§23.1~§23.5 全部落盘；P9 出口结论写入 §23.2/§23.3

---

## P10 · 文档实况收口（收尾批次，无 Rust 实现）

> 阶段：编码任务规划的**收尾批次**（P9 之后）。**本批不含任何 Rust 功能实现**，只把 `README.md` 的现值口径与仓库实况对齐，并在 `docs/progress/b4-status.md` 追加 §25 时点补正与"需用户提供条件"登记。依据：`docs/progress/b4-status.md` §23/§24/§22、`spec.md` §7.10/§7.16、`requirements.md` §15.7、§0.10 实测（825 passed / 834 用例 / 25 token / AC 104 条 103 passed + 1 n/a）。
> **P10 依赖图（单向）**：`W-B4-82.0 ─► 82.1 ─► 82.2 ─► 82.3 ─► 82.4`（82.3 的复跑必须在 82.1/82.2 改完后执行；82.4 最后一次性提交）。
> **P10 三条硬边界**：① 只改 `README.md` 与 `docs/progress/b4-status.md`：`b4-status.md` 必须**追加式**（`git diff --numstat` 删除列=0），`README.md` 只允许 **≤4 处逐行替换**（`git diff --numstat` 单文件、新增+删除 ≤20 行）；② `crates/**`、`scripts/**`、`Cargo.toml`/`Cargo.lock`、`deny.toml`、`spec.md`/`requirements.md` **零 diff**；③ 不新增 target、不改 `NEW_TARGETS`(25)、不新增 `#[ignore]`、不改 AC 编号/规则条数、不写任何 Key/URL/模型名/请求标识。
> **P10 出口判据**：① README 中 `803 passed`=0、`pending`=0、`仍为占位`=0、`825 passed`=1；② `--list`=834、`cargo test --workspace` `failed=0`/`ignored=9`；③ 五条文档门禁退 0（`doc-plan`/`zero-transcript`/`scan-secrets-docs`/`check-g2-claims`/README 相对链接）；④ `b4-status.md` 出现 `## §25` 且 `U-07`/`U-08` 各至少 1 处 `open`；⑤ `git diff --quiet HEAD -- crates scripts Cargo.toml Cargo.lock deny.toml` 退 0。

### W-B4-82 README / 进度实况收口（AC-B4-104 收尾，无新 AC）

- [ ] **W-B4-82.0 提交闸门与实况基线复核** · 仓库根（只读）
  - 追溯：AC-B4-104 · docs/progress/b4-status.md §23 · §0.10.2 · design §8.9.5（P10 基线）
  - 改动内容：无（只读复核）；若工作区非空或 HEAD 不是 `1b0dbf7`，先停下向用户确认，不自行提交/回退
  - 禁止：不得 `git commit`/`git checkout`/`git stash`；不得为凑基线改动任何文件
  - DoD：`git status --short` 输出为空；`git rev-parse --short HEAD`=`1b0dbf7`；`new_targets | wc -w`=**25**；`grep -c '^| AC-B4-' .codeartsdoer/specs/b4_gate/spec.md`=**104**；`test ! -d crates/application/tests/replay` 退 0
  - 证据：
    ```bash
    set -e
    export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
    export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
    [ -z "$(git status --short)" ]
    [ "$(git rev-parse --short HEAD)" = "1b0dbf7" ]
    [ "$(grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//" | wc -w | tr -d ' ')" = "25" ]
    [ "$(grep -c '^| AC-B4-' .codeartsdoer/specs/b4_gate/spec.md)" = "104" ]
    test ! -d crates/application/tests/replay
    echo "exit=$?"
    ```
  - 证据产物：HEAD/25/104/目录 4 个结果写入 `b4-status.md` §25.1

- [ ] **W-B4-82.1 README 三处现值口径校准 + U-06 移入已交付** · `README.md`
  - 追溯：AC-B4-104 · status §23/§24 · §0.10.1 · spec §7.10（汇总 104）
  - 改动内容（**仅这 4 处，逐字替换**）：
    1. 第 15 行表格 `| B4 门控与关键词卡片 | **部分完成**：AC-B4-01~94 离线通过；95~104 仍 \`pending\` |` → `| B4 门控与关键词卡片 | **契约闭环**：AC-B4-01~104 离线通过（AC-B4-45 为 macOS 不适用）；38/39/51 只交付契约与视图模型 |`
    2. 第 18 行 `最近一次全量回归：**803 passed / 0 failed / 9 ignored**；…` → 计数改为 **`825 passed / 0 failed / 9 ignored`**（其余门禁表述保持）
    3. 第 22 行 `- **U-06 13.3 回放本体**：\`crates/application/src/gate/replay.rs\` 仍为占位，\`gate_replay_driver\` target 未建；AC-B4-95~104 为 \`pending\`。` → `- **U-06 13.3 回放本体**：**已交付**（\`gate/replay.rs\` 7 项封闭契约 + \`gate_replay_driver\` 22 用例；AC-B4-95~104 \`passed\`）。`
    4. "仍未完成"小节：U-07/U-08 两行**原样保留**；U-06 行按第 3 条改写（位置可留在该小节但须标注"已交付"）
  - 禁止：不得改写章节顺序/新增小节；不得出现 `pending`、`803`、`仍为占位`；不得内联 URL/模型名/Key 片段/`request_id:`；不得改动"快速开始/门禁与 CI"命令块
  - DoD：`grep -c '803 passed' README.md`=**0**；`grep -c '825 passed' README.md`=**1**；`grep -c 'pending' README.md`=**0**；`grep -c '仍为占位' README.md`=**0**；`grep -c 'AC-B4-01~104' README.md` ≥**1**；`grep -c 'gate_replay_driver' README.md` ≥**1**；`git diff --numstat HEAD -- README.md` 只有 **1 行**（单文件）且 `新增+删除 ≤ 20`（逐行替换必然有删除，故不设"删除=0"）
  - 证据：
    ```bash
    set -e
    [ "$(grep -c '803 passed' README.md)" = "0" ]
    [ "$(grep -c '825 passed' README.md)" = "1" ]
    [ "$(grep -c 'pending' README.md)" = "0" ]
    [ "$(grep -c '仍为占位' README.md)" = "0" ]
    [ "$(grep -c 'AC-B4-01~104' README.md)" -ge 1 ]
    [ "$(grep -c 'gate_replay_driver' README.md)" -ge 1 ]
    [ "$(git diff --numstat HEAD -- README.md | wc -l | tr -d ' ')" = "1" ]
    [ "$(git diff --numstat HEAD -- README.md | awk '{print $1+$2}')" -le 20 ]
    echo "exit=$?"
    ```
  - 证据产物：6 个计数写入 `b4-status.md` §25.1；README 的"新增/删除行数"以 `git diff --numstat` 记录

- [ ] **W-B4-82.2 `b4-status.md` 追加 §25 时点补正与未完成项登记** · `docs/progress/b4-status.md`
  - 追溯：AC-B4-104 · status §23.3/§24 · §0.10.3 · spec §7.16 不宣称清单
  - 改动内容：**追加** `## §25 P10 文档实况收口（W-B4-82）`，四个子节：
    - §25.1 命令结果表：`HEAD=1b0dbf7`、`825/0/9`、`--list=834`、`NEW_TARGETS=25`、`AC 104（103 passed + 1 n/a）`、`tests/replay/` 不存在、README 6 计数
    - §25.2 **时点补正**：`§24` 的"803 passed / `NEW_TARGETS` 24 项 / AC-B4-95~104 pending"是 **P9 提交前时点快照**，现值以 §23+§25 为准；**不删 §24 原文**
    - §25.3 未完成项与用户条件（U-07 四项、U-08、AC-B4-109 远端）：逐条写"所需条件"，口径同 §0.10.3
    - §25.4 不宣称：离线通过 ≠ 负反馈/缓存/outbox/按需补全/哈希链存储已交付；不宣称真实端点/真机/G3/G4/远端 CI 已绿
  - 禁止：不得改 §1~§24 任何一行（追加式：删除列必须 0）；不得把 U-07/U-08 写成 closed；不得写 Key/URL/模型名/请求标识/查询原文；不得占用 §26 作其他用途
  - DoD：`grep -c '^## §25' docs/progress/b4-status.md`=**1**；`grep -c '^### §25\.[1-4]' docs/progress/b4-status.md`=**4**；`sed -n '/^## §25/,$p' docs/progress/b4-status.md | grep -cE 'U-0[78].*open'` ≥**2**；`grep -c '825 passed' docs/progress/b4-status.md` ≥**1**；`git diff --numstat HEAD -- docs/progress/b4-status.md` 删除列=**0**；`bash scripts/check-b4-gates.sh --rule=doc-plan` 退 0
  - 证据：
    ```bash
    set -e
    [ "$(grep -c '^## §25' docs/progress/b4-status.md)" = "1" ]
    [ "$(grep -c '^### §25\.[1-4]' docs/progress/b4-status.md)" = "4" ]
    [ "$(sed -n '/^## §25/,$p' docs/progress/b4-status.md | grep -cE 'U-0[78].*open')" -ge 2 ]
    [ "$(grep -c '825 passed' docs/progress/b4-status.md)" -ge 1 ]
    [ "$(git diff --numstat HEAD -- docs/progress/b4-status.md | awk '{print $2}')" = "0" ]
    bash scripts/check-b4-gates.sh --rule=doc-plan
    echo "exit=$?"
    ```
  - 证据产物：§25.1~§25.4 落盘

- [ ] **W-B4-82.3 实况托底复跑（全量 + 五条文档门禁 + 链接）** · 仓库根
  - 追溯：AC-B4-104 · status §23.1 · README 链接约束（status §24 判据）
  - 改动内容：只跑判据；任一失败回到 82.1/82.2 修正，**不得**为过判据改源码或脚本
  - 禁止：不得改 `crates/**`/`scripts/**`/`Cargo.*`；不得引入 `#[ignore]`；不得把 `--diff` 在未前置 PATH 的会话里跑（X-31）
  - DoD：`cargo test --workspace` 的 `test result` 汇总 = **825 passed / 0 failed / 9 ignored**；`--list`=**834**；`--list --ignored`=**9**；`--rule=doc-plan`/`--rule=zero-transcript`/`--rule=record` = 0；`scan-secrets-docs.sh --all-md` = 0；`check-g2-claims.sh` = 0；`check-layering.sh` = 0；README 相对链接缺失=**0**；`test-name-inventory.sh --diff` `removed=0`
  - 证据：
    ```bash
    set -e
    export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
    export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
    export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
    "$CARGO" test --workspace 2>&1 | grep -E "^test result:" | awk '{ok+=$4; fail+=$6; ign+=$8} END {print "passed="ok, "failed="fail, "ignored="ign}'
    "$CARGO" test --workspace -- --list 2>/dev/null | grep -c ': test'
    "$CARGO" test --workspace -- --list --ignored 2>/dev/null | grep -c ': test'
    for r in doc-plan zero-transcript record; do bash scripts/check-b4-gates.sh --rule="$r" || exit 1; done
    bash scripts/scan-secrets-docs.sh --all-md
    bash scripts/check-g2-claims.sh
    bash scripts/check-layering.sh
    miss=0; for p in $(grep -oE '\]\([^)#]+' README.md | sed 's/^](//' | sort -u); do
      case "$p" in http*) continue;; esac; [ -e "$p" ] || miss=$((miss+1)); done; [ "$miss" = "0" ]
    CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
    echo "exit=$?"
    ```
  - 证据产物：`passed=825 failed=0 ignored=9`、834/9、五门禁 0、`miss=0`、`removed=0` 写入 §25.1

- [ ] **W-B4-82.4 单提交收口（README + §25）** · 仓库根
  - 追溯：AC-B4-104 · status §23.5 不宣称口径 · §0.10.3
  - 改动内容：把 82.1（`README.md`）与 82.2（`b4-status.md` §25）作为**一个提交**落库；提交信息建议 `docs(status): 校准 README 实况并登记 P10 收口（§25）`
  - 禁止：不得把 `crates/**`/`scripts/**`/`Cargo.*` 混入；不得 `--amend`；不得 push（U-08/AC-B4-109 的远端取证须用户另行授权）
  - DoD：`git status --short` 为空；`git show --stat --format=%s HEAD` 仅 `README.md` 与 `docs/progress/b4-status.md`；`git show --stat HEAD | grep -oE '[0-9]+ files? changed'` = **`2 files changed`**；`git log -1 --format=%s` 命中 `P10` 或 `§25`
  - 证据：
    ```bash
    set -e
    git status --short
    git show --stat --format=%s HEAD
    [ "$(git show --stat HEAD | grep -oE '[0-9]+ files? changed' | head -1)" = "2 files changed" ]
    git log -1 --format=%s
    echo "exit=$?"
    ```
  - 证据产物：提交哈希与 2 文件面写入 §25.1（提交后追加一行"§25.5 提交记录"，允许该次提交内自指）

---

## P11 · clippy 全目标告警收敛（质量收口批次，无功能实现）

> 阶段：编码任务规划的**质量收口批次**（P10 之后）。**本批不含任何功能实现**，只把 `cargo clippy --workspace --all-targets -D warnings` 从"CI 红"收敛到"0 告警"。依据：CI `rust` matrix `clippy` 步骤（`.github/workflows/ci.yml:93-94`）、`feature-coverage (C2)` 的 `needs: rust`（同文件 `:177-179`）、`docs/progress/b4-status.md` §25、`design.md` §8.9.6。
> **为何不改 `ci.yml`**：`feature-coverage (C2)` 断言依赖 `rust` job 产出的 `llm-https-coverage-*` artifact；rust 在 clippy 步骤即失败，artifact 未产出，C2 必失败。因此 C2 是**级联失败**，rust 转绿即自愈，`ci.yml` 与 matrix/命令**零 diff**。
> **P11 依赖图（单向）**：`W-B4-83.0 ─► 83.1 ─► 83.2 ─► 83.3 ─► 83.4 ─► 83.5`（83.2 的"clippy 全绿"判据必须 83.1 已落；83.4 必须在 83.1+83.2+83.3 全绿后执行；83.5 最后一次性提交）。
> **P11 会话环境（每个 shell 会话一次）**：全部命令沿用 83.0 导出的 `PATH`/`TEMP`/`TMP`/`CARGO`/`RP`/`DRV`；83.3 另用 `ST=docs/progress/b4-status.md`。`RP`=`crates/application/src/gate/replay.rs`、`DRV`=`crates/application/tests/gate_replay_driver.rs`（与 P9 同名变量口径一致，勿重复定义）。
> **P11 五条硬边界**：① 提交面**恰 3 文件**（`crates/application/src/gate/replay.rs`、`crates/application/tests/gate_replay_driver.rs`、`docs/progress/b4-status.md`）；② 零 diff 面 = `.github/workflows/ci.yml`、`Cargo.toml`/`Cargo.lock`、`crates/*/Cargo.toml`、`deny.toml`、`scripts/**`、`crates/domain/**`、`crates/application/src/cards/**`、`crates/ui-bridge/**`、`crates/infrastructure/**`、`crates/application/tests/gate_replay.rs`、其余 11 个 `gate/*.rs`（含 `api.rs`）；③ **不得以 `allow` 规避**：两个改动文件的 `#[allow(clippy…)]`/`#![allow(clippy…)]`/`#[expect(clippy…)]` 命中数必须为 0；④ 不新增 target（`NEW_TARGETS` **仍 25**）、不新增 AC（汇总 **仍 104**）、不新增 `#[ignore]`（`--list --ignored` **仍 9**、`grep -rn '#\[ignore' --include=*.rs crates/ | wc -l` **仍 12**）、不改用例名/断言数值、`check-b4-gates.sh` 规则**仍 7 条**；⑤ `replay.rs` 非测试区仍不得出现 `panic!(`/`todo!(`/`unimplemented!(`/`std::env`/`Instant::now`/`SystemTime::now`/`thread::sleep`/`std::net`/`tokio::time`/`static mut`/`OnceLock`/`lazy_static`/`thread_local`/`LLM_`（X-18）。
> **P11 出口判据**：`clippy --workspace --all-targets -D warnings`=**0**；`cargo test --workspace`=**825 passed / 0 failed / 9 ignored**；`gate_replay_driver`=**22 passed / 0 failed**；`cargo fmt --all -- --check`=0；七条规则 **7×0**；`--list`=**834**、`--list --ignored`=**9**；`NEW_TARGETS`=**25**；AC=**104**；`test-name-inventory.sh --diff` `removed=0`；零 diff 面全部退 0。

### W-B4-83 clippy 全目标告警收敛（无新 AC / 无新 target）

- [ ] **W-B4-83.0 基线闸门与告警面复核** · 仓库根（**只读**）
  - 追溯：CI `rust` matrix `clippy`（`ci.yml:93-94`）· `ci.yml:177-179`（C2 `needs: rust`）· status §25 · design §8.9.6 · §0.10.5（X-34/X-35）
  - 改动文件：**无**（只读复核）。若 `git status --short` 非空或 HEAD ≠ `52c4bd3`，**先停下向用户确认**，不自行提交/回退/清理
  - 改动内容：
    1. 记录基线 HEAD（应为 P10 单提交 `52c4bd3`）与工作区状态（应为空）
    2. 记录 `NEW_TARGETS`=25、AC=104、`#[ignore]` grep=12
    3. 记录 **lib 侧**告警基线：`map_or_identity` 命中 **2**（`replay.rs:81,82`）
    4. 记录 **test 侧**告警面：**22 处**（`dead_code` 1 / `field_reassign_with_default` 3 / `default_constructed_unit_structs` 17 / `unnecessary_min_or_max` 1）；**只允许**用命令行 `-A` 复现，**不得**改文件加 allow
  - 禁止：不得 `git commit`/`git checkout`/`git stash`/`git clean`；不得为凑基线改任何文件；不得在 `.tmp/` 之外写文件
  - DoD：
    - `git status --short` 输出为空
    - `git rev-parse --short HEAD`=`52c4bd3`
    - `git log -1 --format=%s` 命中 `P10` 或 `§25`
    - `new_targets | wc -w`=**25**
    - `grep -c '^| AC-B4-' spec.md`=**104**
    - `grep -rn '#\[ignore' --include=*.rs crates/ | wc -l`=**12**
    - `cargo --version` 含 **`1.100.0`**（与 CI `rust` matrix 同版本；不一致时**停下确认**，不得自行升级/降级工具链）
  - 证据：
    ```bash
    set -e
    export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
    export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
    export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
    export RP="crates/application/src/gate/replay.rs"
    export DRV="crates/application/tests/gate_replay_driver.rs"
    export SPEC=".codeartsdoer/specs/b4_gate/spec.md"
    new_targets() { grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//"; }

    [ -z "$(git status --short)" ]
    [ "$(git rev-parse --short HEAD)" = "52c4bd3" ]
    git log -1 --format=%s | grep -qE 'P10|§25'
    [ "$(new_targets | wc -w | tr -d ' ')" = "25" ]
    [ "$(grep -c '^| AC-B4-' "$SPEC")" = "104" ]
    [ "$(grep -rn '#\[ignore' --include=*.rs crates/ | wc -l | tr -d ' ')" = "12" ]
    "$CARGO" --version | grep -q '1\.100\.0'
    "$CARGO" clippy -p meeting-copilot-application --lib -- -D warnings 2>&1 | grep -c 'map_or_identity'
    echo "exit=$?"
    ```
  - 证据产物：HEAD/25/104/12/1.100.0/`map_or_identity`=2 与"test 侧 22 处分组"写入 `b4-status.md` §26.1
  - 依赖：无（P11 起点）

- [ ] **W-B4-83.1 `replay.rs` 两处 `map_or_identity` 收敛（第 81、82 行）** · `$RP`
  - 追溯：CI `clippy` 步骤 · X-34 · design §8.9.6 · AC-B4-100/101（报告汇总语义不变）
  - 改动文件：`crates/application/src/gate/replay.rs`（**仅第 81、82 行**，其余一字不动）
  - 改动内容（逐行等价替换）：
    - `:81` `let total = u32::try_from(cases.len()).map_or(u32::MAX, |len| len);` → `let total = u32::try_from(cases.len()).unwrap_or(u32::MAX);`
    - `:82` `let mismatched = u32::try_from(failures.len()).map_or(u32::MAX, |len| len);` → `let mismatched = u32::try_from(failures.len()).unwrap_or(u32::MAX);`
  - 等价性论证（写入 §26.2）：`Result::map_or(u32::MAX, |len| len)` 对 `Ok(v)` 得 `v`、对 `Err(_)` 得 `u32::MAX`；`Result::unwrap_or(u32::MAX)` 对同一两支结果完全相同（`|len| len` 为恒等闭包）。`u32::MAX` 溢出哨兵、`matched = total.saturating_sub(mismatched)` 与 `ReplayReport` 5 字段语义**不变**
  - 既有判据不误伤（本次校验）：P9 的 `grep -cE 'panic!\(|unwrap\(\)|expect\(' "$RP"`=0 —— `unwrap_or(` 不匹配 `unwrap()`；`--rule=source` 禁词表不含 `unwrap_or`。**不需要**放宽该判据，也**不得**为通过判据改判据
  - 禁止：不得改注释、`pub` 项、`impl ReplayDriver` 其它行、`FixedClock`/`detect_gaps`/`evaluate_gate`/`record_from` 调用顺序；不得引入 `unwrap()`/`expect()`/`panic!`；不得加 `#[allow]`
  - DoD：
    - `grep -c 'map_or(' "$RP"`=**0**；`grep -c 'unwrap_or(u32::MAX)' "$RP"`=**2**
    - `grep -cE 'panic!\(|unwrap\(\)|expect\(' "$RP"`=**0**
    - `grep -c 'FixedClock::new' "$RP"`=**1**；`grep -cE '^pub (struct|enum|trait) ' "$RP"`=**7**；`grep -c '^pub fn ' "$RP"`=**0**
    - `grep -cE '#!?\[(allow|expect)\(clippy' "$RP"`=**0**
    - `bash scripts/check-b4-gates.sh --rule=source` 退 **0**
    - `cargo clippy -p meeting-copilot-application --lib -- -D warnings` 退 **0**（lib 目标单独转绿；`--all-targets` 仍待 83.2）
    - `git diff --numstat HEAD -- "$RP"` = **`2 2`**（2 增 2 删）
  - 证据：
    ```bash
    set -e

    [ "$(grep -c 'map_or(' "$RP")" = "0" ]
    [ "$(grep -c 'unwrap_or(u32::MAX)' "$RP")" = "2" ]
    [ "$(grep -cE 'panic!\(|unwrap\(\)|expect\(' "$RP")" = "0" ]
    [ "$(grep -c 'FixedClock::new' "$RP")" = "1" ]
    [ "$(grep -cE '^pub (struct|enum|trait) ' "$RP")" = "7" ]
    [ "$(grep -c '^pub fn ' "$RP")" = "0" ]
    [ "$(grep -cE '#!?\[(allow|expect)\(clippy' "$RP")" = "0" ]
    [ "$(git diff --numstat HEAD -- "$RP" | awk '{print $1" "$2}')" = "2 2" ]
    bash scripts/check-b4-gates.sh --rule=source
    "$CARGO" clippy -p meeting-copilot-application --lib -- -D warnings
    echo "exit=$?"
    ```
  - 证据产物：`map_or`=0、`unwrap_or`=2、`$RP` `2 2`、`--rule=source`=0 写入 §26.1/§26.2
  - 依赖：W-B4-83.0

- [ ] **W-B4-83.2 测试目标 22 处告警收敛** · `$DRV`
  - 追溯：X-35 · design §8.9.6 · AC-B4-95~104（用例名/断言/数量全不变）
  - 改动文件：`crates/application/tests/gate_replay_driver.rs`（单文件；预计新增+删除 ≤ 40 行）
  - 改动内容（四组，锚点文本定位；行号基于 `52c4bd3` 仅供定位，改后行号会平移）：
    - **A 组 `dead_code` 1 处**：删除 `struct Arena` 的 `reference: UtteranceRef,` 字段（`:43`）与 `Arena::new` 的 `Self { …, reference: reference.clone(), … }` 初始化行（`:99`）。**保留** `let reference = UtteranceRef::new(REFERENCE).expect("reference");`（`:71`，属测试内既有 `expect`，`--rule=zero-transcript` 覆盖，不受 `--rule=source` 约束）：该局部变量仍被 `stable_prefix_ms.insert(reference.clone(), 0)`（`:88`）与 `fragment(&session, &reference, …)`（`:100-101`）读取，因此**不产生 `unused_variables`**（本次校验结论，已核对全部 `reference` 引用点）
    - **B 组 `field_reassign_with_default` 3 处**：改为结构体更新语法，`Default` 语义不变：
      - `build_self_check`：`let mut invalid_policy = GatePolicy::default();` + `invalid_policy.session_outbound_cap = 601;` → `let invalid_policy = GatePolicy { session_outbound_cap: 601, ..GatePolicy::default() };`
      - 同函数：`let mut skip_dedupe = GatePolicy::default();` + `skip_dedupe.dedupe_enabled = false;` → `let skip_dedupe = GatePolicy { dedupe_enabled: false, ..GatePolicy::default() };`
      - `replay_driver_uses_only_four_functions`：`let mut strict = GatePolicy::default();` + `strict.threshold_profile = ThresholdProfile::Strict;` → `let strict = GatePolicy { threshold_profile: ThresholdProfile::Strict, ..GatePolicy::default() };`
      - `let default_policy = GatePolicy::default;`（`:232`、`:398`，**函数指针**）**保持不动**
    - **C 组 `default_constructed_unit_structs` 17 处**：`ReplayDriver` 是 `pub struct ReplayDriver;`（unit struct，`replay.rs:47` 派生 `Debug, Clone, Copy, Default`），`ReplayDriver::default()` 与 `ReplayDriver` 是**同一个零尺寸值**，调用形态等价：
      - **留证点 1 处**：`replay_driver_types_are_closed` 中 `let driver = ReplayDriver::default();`（`:519`）→ `let driver: ReplayDriver = Default::default();`。该写法**不触发** `default_constructed_unit_structs`（lint 只匹配 `Type::default()` 的调用路径），同时保留"`Default` 派生仍可用"的**编译期证据**。**备选退路**（仅当该写法在不同 clippy 实现下仍被标记时启用）：在同一用例内追加 `let _default_probe: fn() -> ReplayDriver = ReplayDriver::default;`（函数指针引用，不触发 `default_constructed_unit_structs`，`_` 前缀避免 `unused_variables`），并同步把 X-35 的留证口径改为该行
      - **其余 16 处**（参考行号 `:538`、`:550`、`:590`、`:638`、`:706`、`:721`、`:738`、`:750`、`:767`、`:793`、`:819`、`:849`、`:869`、`:885`、`:906`、`:925`）→ `ReplayDriver`；若原式为 `let run = ReplayDriver::default().run(&source);` 则改为 `let run = ReplayDriver.run(&source);`（不拆行、不改调用链顺序、不改 `Source::forward(…, 13)` 的 seed）
    - **D 组 `unnecessary_min_or_max` 1 处**：`assert_eq!(report.mismatched as usize, 0.max(report.mismatch_cases.len()));`（`:839-842` 区间）→ `assert_eq!(report.mismatched as usize, report.mismatch_cases.len());`。**断言语义不变**：`mismatch_cases.len()` 为 `usize`，`0` 是 `usize` 下界，`0.max(n)` 恒等于 `n`；被测不变量仍为"`mismatched` 与失败清单条目数相等"（本次校验结论）
  - 禁止：不得用 `#[allow]`/`#![allow]`/`#[expect]`；不得改用例名（`^fn replay_` 仍 22）、`assert` 数值、断言消息模板、`Arena` 其它字段、`TEXT`/`REFERENCE`/`SESSION`/`TRACE`/`PROVIDER` 常量值、`seed` 13/29；不得增删测试函数；不得新增 `#[ignore]`；不得改其它测试文件
  - DoD：
    - `cargo clippy --workspace --all-targets -- -D warnings` 退 **0**（须 83.1 已落）
    - `grep -c 'ReplayDriver::default()' "$DRV"`=**0**
    - `grep -c 'let driver: ReplayDriver = Default::default();' "$DRV"`=**1**
    - `grep -c 'arena.reference' "$DRV"`=**0**；`grep -c '^    reference: UtteranceRef,' "$DRV"`=**0**（`Arena` 字段已删；`fragment()` 内 `NormalizedFragment` 的 `reference:` 赋值仍保留 2 处，属正常面）
    - `grep -c '0\.max(' "$DRV"`=**0**
    - `grep -cE '#!?\[(allow|expect)\(clippy' "$DRV"`=**0**
    - `grep -c '^fn replay_' "$DRV"`=**22**；`"$CARGO" test -p meeting-copilot-application --test gate_replay_driver -- --list 2>/dev/null | grep -c ': test'`=**22**
    - `cargo test -p meeting-copilot-application --test gate_replay_driver` = **22 passed / 0 failed**
    - `cargo fmt --all -- --check` 退 **0**
    - `git diff --numstat HEAD -- "$DRV"` 单行且 `新增+删除 ≤ 40`
  - 证据：
    ```bash
    set -e
    [ "$(grep -c 'ReplayDriver::default()' "$DRV")" = "0" ]
    [ "$(grep -c 'let driver: ReplayDriver = Default::default();' "$DRV")" = "1" ]
    [ "$(grep -c 'arena.reference' "$DRV")" = "0" ]
    [ "$(grep -c '0\.max(' "$DRV")" = "0" ]
    [ "$(grep -cE '#!?\[(allow|expect)\(clippy' "$DRV")" = "0" ]
    [ "$(grep -c '^fn replay_' "$DRV")" = "22" ]
    [ "$("$CARGO" test -p meeting-copilot-application --test gate_replay_driver -- --list 2>/dev/null | grep -c ': test')" = "22" ]
    "$CARGO" clippy --workspace --all-targets -- -D warnings
    "$CARGO" test -p meeting-copilot-application --test gate_replay_driver
    "$CARGO" fmt --all -- --check
    echo "exit=$?"
    ```
  - 证据产物：22 用例 22/22、`clippy`=0、`Default::default()` 留证 1 处、`$DRV` 行数写入 §26.1/§26.2
  - 依赖：W-B4-83.1

- [ ] **W-B4-83.3 `b4-status.md` 追加 §26 收敛快照** · `docs/progress/b4-status.md`
  - 追溯：X-34/X-35 · design §8.9.6 · status §25 · CI C2 级联判定
  - 改动文件：`docs/progress/b4-status.md`（**追加式**；§1~§25 零改）
  - 改动内容：追加 `## §26 clippy 全目标告警收敛（W-B4-83）`，五个子节：
    - **§26.1 命令结果表**：修复前 **24 处**（2 lib `map_or_identity` + 22 test）/ 修复后 **0**；`cargo test --workspace`=`825/0/9`；`--list`=834；`--list --ignored`=9；`NEW_TARGETS`=25；AC=104；`#[ignore]` grep=12；`gate_replay_driver`=22/22；`fmt --check`=0
    - **§26.2 改动面与等价性**：3 文件 + `git diff --numstat` 三行（`replay.rs`=`2 2`；`gate_replay_driver.rs` 新增+删除 ≤40；本节文件删除列 0）；逐项写 A/B/C/D 四组处置与 `map_or`→`unwrap_or` 的等价论证、`0.max(len)`→`len` 的断言语义不变论证
    - **§26.3 判据补正登记**：X-34（`map_or` 写法）、X-35（test-target 22 处 + `Default` 留证口径）、P11 基线 `52c4bd3`，并指向 `design.md` §8.9.6
    - **§26.4 不宣称清单**：不改 `ci.yml`/matrix/命令；`feature-coverage (C2)` 是 `needs: rust` 级联失败（无 `llm-https-coverage-*` artifact），rust 转绿即自愈，不宣称"修好了 C2"；不新增功能/target/AC/`#[ignore]`；不宣称远端 CI 已绿（AC-B4-109 仍待用户授权 push）
    - **§26.5 提交记录**：单提交 **3 文件**、未 push
  - 禁止：不得改 §1~§25 任何一行（`git diff --numstat` 删除列必须 0）；不得占用 §27；不得把 U-07/U-08 写成 closed；不得写 Key/URL/模型名/`request_id:`/查询原文；不得把设计侧登记再抄一遍全文（只给编号与指向）
  - DoD：
    - `grep -c '^## §26' docs/progress/b4-status.md`=**1**
    - `grep -c '^### §26\.[1-5]' docs/progress/b4-status.md`=**5**
    - `git diff --numstat HEAD -- docs/progress/b4-status.md | awk '{print $2}'`=**0**
    - `sed -n '/^## §26/,$p' docs/progress/b4-status.md | grep -c 'X-34'` ≥**1**；`… | grep -c 'X-35'` ≥**1**；`… | grep -c 'feature-coverage'` ≥**1**；`… | grep -c '52c4bd3'` ≥**1**
    - `bash scripts/check-b4-gates.sh --rule=doc-plan` 退 **0**
    - `bash scripts/scan-secrets-docs.sh --all-md` 退 **0**
  - 证据：
    ```bash
    set -e
    export ST="docs/progress/b4-status.md"
    [ "$(grep -c '^## §26' "$ST")" = "1" ]
    [ "$(grep -c '^### §26\.[1-5]' "$ST")" = "5" ]
    [ "$(git diff --numstat HEAD -- "$ST" | awk '{print $2}')" = "0" ]
    [ "$(sed -n '/^## §26/,$p' "$ST" | grep -c 'X-34')" -ge 1 ]
    [ "$(sed -n '/^## §26/,$p' "$ST" | grep -c 'X-35')" -ge 1 ]
    [ "$(sed -n '/^## §26/,$p' "$ST" | grep -c 'feature-coverage')" -ge 1 ]
    [ "$(sed -n '/^## §26/,$p' "$ST" | grep -c '52c4bd3')" -ge 1 ]
    bash scripts/check-b4-gates.sh --rule=doc-plan
    bash scripts/scan-secrets-docs.sh --all-md
    echo "exit=$?"
    ```
  - 证据产物：§26.1~§26.5 落盘；删除列 0
  - 依赖：W-B4-83.2（命令表需含最终实测值）

- [ ] **W-B4-83.4 全量复跑（clippy + 全量测试 + fmt + 七规则 + 零 diff 面）** · 仓库根
  - 追溯：CI `rust` 步骤全序列（`clippy`→`build`→`test`）· P11 出口判据 · AC 全量
  - 改动文件：**无**（只跑判据）；任一失败回到 83.1/83.2/83.3 修正，**不得**为过判据改源码/脚本/CI
  - 改动内容：按 P11 出口判据逐项复跑并记录
  - 禁止：不得改 `crates/**`/`scripts/**`/`.github/**`/`Cargo.*`；不得引入 `#[ignore]`；不得在未前置 PATH 的会话跑 `--diff`（X-31）；不得跳过 `--all-targets`
  - DoD：
    - `cargo clippy --workspace --all-targets -- -D warnings` 退 **0**（零告警）
    - `cargo test --workspace` 的 `test result` 汇总 = **825 passed / 0 failed / 9 ignored**
    - `cargo test -p meeting-copilot-application --test gate_replay_driver` = **22 passed / 0 failed**
    - `cargo fmt --all -- --check` 退 **0**
    - 七条规则 **7×0**（`source`/`budget`/`record`/`outbound`/`zero-transcript`/`targets`/`doc-plan`）
    - `--list`=**834**、`--list --ignored`=**9**
    - `NEW_TARGETS`=**25**、`grep -c '^| AC-B4-' spec.md`=**104**
    - `test-name-inventory.sh --diff docs/progress/test-names-before.txt` 输出 `removed=0`
    - 零 diff 面：`git diff --quiet HEAD -- .github/workflows/ci.yml Cargo.toml Cargo.lock deny.toml scripts crates/domain crates/application/src/cards crates/ui-bridge crates/infrastructure crates/application/tests/gate_replay.rs` 退 **0**；其余 11 个 `gate/*.rs` 逐个零 diff（`gate_12_zero` 口径 + `replay.rs` 为本批唯一改写）
    - 附加门禁：`check-layering.sh`=0、`scan-secrets-docs.sh --all-md`=0、`check-g2-claims.sh`=0
  - 证据：
    ```bash
    set -e
    export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
    export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
    export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
    "$CARGO" clippy --workspace --all-targets -- -D warnings
    "$CARGO" test --workspace 2>&1 | grep -E "^test result:" | awk '{ok+=$4; fail+=$6; ign+=$8} END {print "passed="ok, "failed="fail, "ignored="ign}'
    "$CARGO" test -p meeting-copilot-application --test gate_replay_driver 2>&1 | grep -E "^test result:"
    "$CARGO" fmt --all -- --check
    "$CARGO" test --workspace -- --list 2>/dev/null | grep -c ': test'
    "$CARGO" test --workspace -- --list --ignored 2>/dev/null | grep -c ': test'
    for r in source budget record outbound zero-transcript targets doc-plan; do bash scripts/check-b4-gates.sh --rule="$r" || exit 1; done
    bash scripts/check-layering.sh
    bash scripts/scan-secrets-docs.sh --all-md
    bash scripts/check-g2-claims.sh
    CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
    git diff --quiet HEAD -- .github/workflows/ci.yml Cargo.toml Cargo.lock deny.toml scripts crates/domain crates/application/src/cards crates/ui-bridge crates/infrastructure crates/application/tests/gate_replay.rs
    echo "exit=$?"
    ```
  - 证据产物：`passed=825 failed=0 ignored=9`、22/22、834/9、7×0、`removed=0`、零 diff=0 写入 §26.1
  - 依赖：W-B4-83.1 + W-B4-83.2 + W-B4-83.3

- [ ] **W-B4-83.5 单提交收口（恰 3 文件）** · 仓库根
  - 追溯：P11 五条硬边界①· D-B4 提交纪律（不加 `--amend`、不 push）· status §25.5 口径
  - 改动文件：把 83.1（`crates/application/src/gate/replay.rs`）、83.2（`crates/application/tests/gate_replay_driver.rs`）、83.3（`docs/progress/b4-status.md` §26）作为**一个提交**落库
  - 提交信息建议：`fix(gate): 收敛 clippy 全目标告警（W-B4-83，§26）`
  - 禁止：不得 `--amend`/`push`/`rebase`；不得把 `design.md`/`tasks.md`/`ci.yml`/`Cargo.*`/`scripts/**` 混入；不得拆成多提交；不得跳过 hooks
  - DoD：
    - `git status --short` 为空
    - `git show --stat --format=%s HEAD` 仅 3 个路径：`crates/application/src/gate/replay.rs`、`crates/application/tests/gate_replay_driver.rs`、`docs/progress/b4-status.md`
    - `git show --stat HEAD | grep -oE '[0-9]+ files? changed'` = **`3 files changed`**
    - `git log -1 --format=%s` 命中 `clippy` 或 `§26`
    - `git show --numstat HEAD | awk '{s+=$1+$2} END {print s}'` ≤ **70**（`replay.rs` 4 行 + 测试 ≤40 行 + §26 追加 ≤26 行）
  - 证据：
    ```bash
    set -e
    git status --short
    git show --stat --format=%s HEAD
    [ "$(git show --stat HEAD | grep -oE '[0-9]+ files? changed' | head -1)" = "3 files changed" ]
    git log -1 --format=%s | grep -qE 'clippy|§26'
    echo "exit=$?"
    ```
  - 证据产物：提交哈希与 3 文件面写入 §26.1（提交后追加"§26.5 提交记录"一行，允许该提交内自指）
  - 依赖：W-B4-83.4

## 附：阶段出口一览

| 阶段 | 主任务 | 子任务 | 出口判据 |
|---|---|---|---|
| P0 | W-B4-1~8 | 42（W-B4-1 已交付 5 + 本阶段执行 36 + 5.4b 锚点 1） | §0.7 三条硬判据 + 五条旁证（`targets`=1）+ 12 项零改动矩阵 + fmt/clippy + B3 门禁 |
| P1 | W-B4-9~14 | 28 | `gate_prefilter` 退 0（AC-01~08）；`--rule=source` 退 0；`pub fn` 计数 =4 |
| P2 | W-B4-15~23 | 34 | 三个 target 退 0（AC-09~27）；`--rule=budget` 退 0；单窗口断言绿 |
| P3 | W-B4-24~33 | 39 | 6 target 全绿；50/50 三态；584/9 + `removed=0`；七条规则 7×0 + 五项既有门禁全 0 |
| P4（增量） | W-B4-34~42 | 46 | `gate_keyword` 13 个 AC 用例（+2 跳过投影）与 `gate_replay` 5 用例全绿；AC-48/51~57 七行 `passed`；`total=200`/`mismatched=0`/`seed=7`/两次字节相等；7 target + 7 规则 + 20 项 target 清单；`api.rs` 4 函数且零 diff；57/57 三态登记 |
| P5（增量） | W-B4-43~48 | 27 | `citation_align` **12 用例全绿**；`basis` 11/11、投影 3→3 单射；8 target + 7 规则（`--rule=targets` 仍 7 项、退 0）+ `NEW_TARGETS`=21；`api.rs` 4 函数零 diff、14/7/23/5/32/23 不变、`tests/replay/` 不存在；`cargo test --workspace` failed=0/ignored=9、`removed=0`；AC-58~64 七行 `passed`、**U-04 关闭**（U-03/U-05 保持 open）；`Cargo.toml`/`Cargo.lock` 零 diff |
| P6（增量） | W-B4-49~56 | 33 | `card_compose` **14 用例全绿**；`CardComposeInput`=7 / `ComposedCard`=11 / `CardComposeOutcome`=2 / `ComposeRejectReason`=6；9 target + 7 规则（`--rule=targets` 仍 7 项、退 0）+ `NEW_TARGETS`=**22**；`api.rs` 4 函数零 diff、`gate/**`/`domain/src/model/**` 零 diff、14/7/23/5/32/23 不变、`tests/replay/` 与 `ui-bridge/src/keyword_card/` 不存在；`cargo test --workspace` failed=0/ignored=9、`removed=0`；AC 汇总 74、AC-65~74 十行 `passed`、**U-03 关闭**（U-05/U-06/U-07/U-08 保持 open）；`Cargo.toml`/`Cargo.lock` 零 diff；`Y-01` 已裁决 |
| P7（51 点击 UI） | W-B4-57~64 | 30 | `keyword_card_view` **19 用例 + 1 自检全绿**；`KeywordCardView`=10 / `CardBodyView`=4 / `CardDetailView`=5 / `ViewFacts`=6 / `CardViewOutcome`=2 / `ClickOutcome`=2 / `ConclusionView`=3 / `ViewRejectReason`=3；`keyword_card/` 顶层类型 8、顶层 `pub fn` 3；10 target + 7 规则（`--rule=targets` 仍 7 项、退 0）+ `NEW_TARGETS`=**23**；`api.rs` 4 零 diff、`gate/**`/`cards/**`（源码）/`domain/src/model/**`/`infrastructure/src/llm/**` 零 diff、13 项封闭面不变、`tests/replay/` 与 `settings/` 不存在；`card_compose` 14 用例仍全绿（X-12 已处置）；`cargo test --workspace` failed=0/ignored=9、`removed=0`；AC-75~84 十行 `passed`、**U-05 保留"规格已定"且未整行 closed**（设置向导待 P8）；`Cargo.toml` 根零 diff + `ui-bridge/Cargo.toml` +2 path 依赖（X-13） |
| P8（51 设置向导） | W-B4-65~72 | 30 | `llm_settings_view` **18 用例全绿**；`SettingsView`=8 / `SettingsFacts`=6 / `SettingsInput`=5 / `SettingsDefaults`=4 / `ProbeRequest`=4；`settings/` 顶层类型 **18**、顶层 `pub fn` **7**；`SECTION_ORDER.len()`=4；`SettingsFormState`=6 / `ProbeEvent`=5 / `ProbeFailureKind`=8 / `FieldIssue`=6 / `ProbeOutcome`=2 / `SettingsIntent`=3 / `MaskedKey`=2 / `ModelSelectionView`=2 / `EndpointSelectionView`=2 / `ModelDraft`=3 / `EndpointDraft`=2 / `PersistTarget`=2；11 target + 7 规则（`--rule=targets` 仍 7 项、退 0）+ `NEW_TARGETS`=**24**；`Cargo.toml`/`Cargo.lock` 零 diff（以 P7 HEAD 为基线）、`keyword_card/**` 零 diff、`api.rs` 4、13 项封闭面不变；`cargo test --workspace` failed=0/ignored=9、`removed=0`；AC-85~94 十行 `passed`、**U-05 关闭**（保留"规格已定"原文；U-06/U-07/U-08 保持 open） |
| P9（13.3 回放本体） | W-B4-73~81 | 30 | `gate_replay_driver` **22 用例全绿**（+24 例主集/5 例探针集）；顶层 `pub` 类型 **7**、顶层 `pub fn` **0**；`ReplayCase`=4 / `ReplayRun`=3 / `ReplayFailure`=2 / `ReplayMismatchCode`=5 / `ReplayLoadError`=3；`gate/replay.rs` 唯一改写、`gate/mod.rs` **+1 行**、`api.rs` 4 零 diff、其余 12 个 `gate/*.rs` 零 diff（含 `api.rs`）；12 target + 7 规则（`--rule=targets` 仍 7 项、退 0）+ `NEW_TARGETS`=**25**、`check-b4-gates.sh` **零 diff**、`grep -c 'gate_replay_driver' scripts/check-b4-gates.sh`=0；`domain/**`/`cards/**`/`ui-bridge/**`/`infrastructure/**`/`tests/gate_replay.rs`/`Cargo.*` 零 diff、`tests/replay/` 不存在；13 项封闭面 14/7/23/5/32/23/11/10/8 不变；`cargo test --workspace` failed=0/ignored=9、`--diff` `removed=0`；AC-B4-95~104 十行 `passed`、**U-06 关闭**（保留"规格已定"原文；U-07/U-08 保持 open）；`X-17`~`X-26` 已登记 |
| P10（文档实况收口） | W-B4-82 | 5 | README 现值口径校准（`825 passed` / `AC-B4-01~104` / U-06 已交付，`pending`/`803`/`仍为占位` 归零）；`b4-status.md` **追加** `## §25`（§25.1 命令表 / §25.2 时点补正 / §25.3 未完成项与用户条件 / §25.4 不宣称 / §25.5 提交记录）；`NEW_TARGETS` 仍 **25**、AC 汇总仍 **104**、不新增 target/AC/`#[ignore]`；`crates/**`/`scripts/**`/`Cargo.*`/`deny.toml` 零 diff；全量 `825/0/9`、`--list`=834、五条文档门禁 0、README 链接缺失 0、`--diff` `removed=0`；U-07/U-08 保持 `open` 并在 §25.3 登记所需条件；`X-29`~`X-33` 已登记 |
| P11（clippy 全目标收敛） | W-B4-83 | 6 | 24 处告警归零（2 lib `map_or_identity` + 22 test：`dead_code`1 / `field_reassign_with_default`3 / `default_constructed_unit_structs`17 / `unnecessary_min_or_max`1）；`clippy --workspace --all-targets -D warnings`=0；`cargo test --workspace` **825/0/9**、`gate_replay_driver` **22/22**、`--list`=834、`--list --ignored`=9；`fmt --check`=0；七规则 7×0；`NEW_TARGETS` 仍 **25**、AC 仍 **104**、`#[ignore]` grep 仍 **12**；提交面恰 **3 文件**（`gate/replay.rs` / `tests/gate_replay_driver.rs` / `b4-status.md` §26），`.github/workflows/ci.yml`/`Cargo.*`/`scripts/**`/`domain/**`/`cards/**`/`ui-bridge/**`/`infrastructure/**`/`tests/gate_replay.rs` 零 diff；`feature-coverage (C2)` 为 `needs: rust` 级联失败、rust 转绿自愈、不改 CI；`X-34`/`X-35` 已登记（design §8.9.6） |
| 合计 | **82** | **344** | P0~P10 分项之和（原表"42/189"为 P0~P3 时点口径，P4~P10 增量逐次追加；P7/P8/P9/P10 **串行**，各自独立提交） |