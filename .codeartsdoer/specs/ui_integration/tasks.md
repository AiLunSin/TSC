# ui_integration · 编码任务清单（U-05 产品化接入：设置向导 + 关键词卡片渲染）

> **阶段**：阶段 3：编码任务规划。本文件把 `.codeartsdoer/specs/ui_integration/spec.md`（AC-B4-111~130）与 `design.md`（D-B4-P144~P170、§2.1~§2.8）翻译为可执行、可验收的任务；**不写代码、不改配置、不提交**。
> **基线**：`HEAD=6d5b0b0`（`docs(handoff)`）；工作区干净（`git status --short` 为空）；上一版基线 `cargo test --workspace` 825 passed / 0 failed / 9 ignored；`NEW_TARGETS`=25；AC 编号汇总=130（111~130 全 `pending`）；U-05 整体 `open`。
> **实现范围**：2 项能力（设置向导接入、关键词卡片渲染）、6 个固定 IPC 命令、3+3 端口（6 类纯数据封闭 + `6/2/6` 事实源）、6 组只读 DTO（11/5/4/3/10/4/5）、1 个新门禁脚本（六规则）、1 处既有门禁扩面（规则仍 7 条）、2 份文档面。
> **任务编号**：主任务 `W-UI-01~W-UI-18`；子任务 `W-UI-xx.y`；共 **18 主任务 / 65 子任务**；AC 编号不重排、不改语义；本轮校准不重排任务编号。
> **实况口径（2026-11-23 校准，优先级高于旧稿）**：`windows-sys` 固化 **0.45.0**（单版本、仅 `cfg(windows)`、feature 仅 `Win32_Security_Credentials`+`Win32_Foundation`，X-UI-11）；`ClickState` 带载荷，DTO 必须拆 `clickState`+`disabledReason`（X/D-B4-P172）；`ProbeAttempt.remainingMs` 为相对值、`cooldown_until_ms` 为内部绝对镜像（X-UI-13）；`check-ui-integration.sh` 属 C6，必须晚于 C1~C5（X-UI-12）。

## 0. 执行总览

### 0.1 硬边界（任一破坏即本阶段不放行）

| 面 | 取值 | 动作 |
|---|---|---|
| `crates/ui-bridge/src/{settings,keyword_card}/**` | 18 类型 / 7 函数 / 3 常量；8 类型 / 3 函数 | 零 diff |
| `crates/ui-bridge/Cargo.toml`、`Cargo.lock` 的 ui-bridge 段 | — | 零 diff |
| `crates/application/src/gate/api.rs` | 4 个 `pub fn` | 零 diff |
| `GateRecord`/审计/夹具/`ReplayReport`/`GateCounter`/`GateRejectReason` | 14/7/23/5/32/23 | 零新增、零改 |
| `ComposedCard`/`KeywordCardView`/`SettingsView`/`KeywordCardDraft`/`ReferenceView`/`CitationAlignment` | 11/10/8/13/5/3 | 零新增字段 |
| `SettingsFacts`/`SettingsInput`/`SettingsIntent` | 6/5/3 变体 | 零 diff |
| `crates/application/src/{gate,cards}`、`crates/domain/src/model/**` | — | 零 diff |
| `crates/application/Cargo.toml` | — | 零 diff（只加模块、不加依赖） |
| `crates/infrastructure/src/llm/**` | — | 零 diff（只消费公开 API） |
| `crates/platform-win/Cargo.toml` 的 `windows-sys` | 唯一条目 **0.45.0**，仅 `[target.'cfg(windows)'.dependencies]`，features 仅 `Win32_Security_Credentials`+`Win32_Foundation` | 不升级 0.59/0.61、不引入第二版本（X-UI-11） |
| 端口纯数据 / 事实源 | `SettingsConfig` 4、`SecretBytes` 无载荷、`ProbeFailure` 8 态、`PortError` 4 类；`FactsSnapshot` 6 / `CardSnapshotEntry` 2 态 / `ViewFactsSnapshot` 6 | 新增字段/变体即结构违规（AC-119 E2/E5） |
| `scripts/check-ui-integration.sh`（新增）、`scripts/check-b4-gates.sh`（扩面） | 只在 **C6** 落盘 | 必须晚于 C1~C5（X-UI-12） |
| `crates/platform-mac/**`、`platform-win/resources/capability_matrix.toml` | — | 零 diff（仍 6 条） |
| `.github/workflows/ci.yml`、`deny.toml` | — | 零 diff |
| `app/package.json`、`app/package-lock.json` | — | 零 diff（不新增 `@tauri-apps/api`） |
| `NEW_TARGETS` | 25 | 不新增 cargo target |
| 既有 7 条门禁规则、退出码 0/1/2 | 7 | 规则数不变，只扩扫描面 |
| AC-B4-01~110 | — | 不重排、不改语义 |

### 0.2 AC 覆盖矩阵（AC-B4-111~130 → 主任务，20/20）

| AC | 主题 | 主任务 | 关键用例名（逐字） |
|---|---|---|---|
| AC-B4-111 | 设置快照 11 项封闭、无 Key | W-UI-06 | `ipc_settings_snapshot_is_closed_and_keyless`、`ipc_settings_snapshot_without_key_is_not_configured` |
| AC-B4-112 | 凭据管理器唯一目标名、读写删、幂等 | W-UI-03 | `secure_store_roundtrip_uses_fixed_target`、`secure_store_write_is_idempotent`、`secure_store_delete_is_idempotent` |
| AC-B4-113 | `settings.json` 恰 4 键、原子写、损坏回默认 | W-UI-05 | `ipc_settings_file_has_only_four_keys`、`ipc_settings_corruption_falls_back_to_defaults` |
| AC-B4-114 | 探针只经端口、复用既有网关、0 网络符号 | W-UI-07 | `probe_executor_is_port_only`、`probe_adapter_reuses_existing_gateway`、`ipc_probe_never_opens_socket` |
| AC-B4-115 | 先安全存储后配置、降级回执、清除确认与幂等 | W-UI-06 | `save_receipt_reports_memory_fallback_without_plaintext`、`clear_requires_confirmation_and_deletes_target`、`clear_is_idempotent` |
| AC-B4-116 | 设置页四段顺序、首屏折叠、按钮只取后端标志 | W-UI-11 | `settings_page_renders_four_fixed_sections`、`settings_page_buttons_follow_server_flags` |
| AC-B4-117 | 8 类失败固定文案、保留输入、冷却剩余 | W-UI-07 | `ipc_probe_failure_maps_eight_kinds`、`ipc_probe_failure_keeps_inputs`、`ipc_probe_cooling_reports_remaining_ms` |
| AC-B4-118 | Key 不进持久化/日志/审计/响应 | W-UI-06 | `ipc_key_is_absent_from_responses_and_logs`、`settings_api_key_is_masked_without_payload` |
| AC-B4-119 | 三端口在 application、实现分离、非 Windows `Unsupported` | W-UI-02、W-UI-03 | `application_declares_three_ports`、`sources_declare_three_ports`、`settings_config_rejects_extra_keys`、`secure_store_is_windows_only` |
| AC-B4-120 | 6 命令封闭、四目录零 diff、`NEW_TARGETS`=25、AC=130、README 不宣称 | W-UI-09、W-UI-16 | ——（计数与零 diff） |
| AC-B4-121 | 卡片快照 10 项 / 详情 5 项封闭、载荷已拆、`withdrawn` 不出现 | W-UI-08 | `ipc_card_snapshot_is_closed_and_read_only`、`ipc_card_detail_is_closed`、`keyword_card_view_failed_alignment_stays_visible_but_disabled` |
| AC-B4-122 | 仅 enabled 且可见绑定点击、禁用卡 `aria-disabled` | W-UI-12 | `card_component_binds_click_only_when_enabled`、`card_component_marks_disabled_semantics` |
| AC-B4-123 | 隐藏态只渲染角标、DOM/DTO 无正文 | W-UI-12、W-UI-08 | `card_component_hides_body_when_not_visible`、`ipc_card_snapshot_hides_body_under_screen_share` |
| AC-B4-124 | 点击只映射本地详情、0 副作用 | W-UI-08 | `ipc_card_open_has_zero_side_effects`、`keyword_card_view_detail_has_no_outbound_side_effect` |
| AC-B4-125 | 内联非模态详情、不抢焦点 | W-UI-12 | `card_detail_does_not_steal_focus` |
| AC-B4-126 | 前端 0 网络、CSP 无连接源、能力仅 core、白名单 6 | W-UI-10、W-UI-09 | `frontend_is_offline_and_minimal_privilege` |
| AC-B4-127 | 不持久化 Key/正文、不复活撤回卡 | W-UI-13、W-UI-08 | `frontend_persists_neither_key_nor_card_text`、`card_snapshot_does_not_revive_withdrawn` |
| AC-B4-128 | 既有封闭面全不变 | W-UI-17 | `existing_closed_faces_are_unchanged` |
| AC-B4-129 | Windows 单平台、mac 不适用、CI 零改动 | W-UI-17 | `platform_scope_is_windows_only` |
| AC-B4-130 | 六规则全 0、AC=130、`NEW_TARGETS`=25、§28 登记、全量测试 | W-UI-14、W-UI-15、W-UI-16、W-UI-18 | ——（聚合断言） |

### 0.3 执行批次与依赖顺序

| 批次 | 主任务 | 前置依赖 | 说明 |
|---|---|---|---|
| C1 | W-UI-01、W-UI-02、W-UI-03 | — | 基线 + 契约层 + 平台层；端口/事实源封闭与 `windows-sys` 0.45.0 同批 |
| C2 | W-UI-04 | C1 | 装配层依赖、DTO、状态、目录清单 |
| C3 | W-UI-05、W-UI-06、W-UI-07、W-UI-08 | C2 | 05→06；07、08 与 06 可并行；08 含 §2.3.3.1 手工拆载 |
| C4 | W-UI-09 | C3 | 6 命令注册与运行态装配、`withGlobalTauri` |
| C5 | W-UI-10、W-UI-11、W-UI-12、W-UI-13 | C4 | 10→11/12→13 |
| C6 | W-UI-14、W-UI-15 | **C1~C5 全部落盘** | 脚本必须后置；禁止先改脚本导致退 2（X-UI-12） |
| C7 | W-UI-16、W-UI-17 | C6 | 文档登记实现快照、零 diff 与计数取证 |
| 收口 | W-UI-18 | C7 | 全量回归 + 按 §0.7 编排 C1~C7 七个提交；**不新增第 8 个提交** |

> **串行红线**：`W-UI-14.1` / `W-UI-15.1` 的扫描面断言依赖被扫文件已存在；若先加脚本后落盘，六规则与 7 条规则会退 2，视为放行失败。C6 必须晚于 C1~C5（D-B4-P174 / X-UI-12）。

### 0.4 与 U-05 / U-07 / U-08 的关系

| 缺口 | 本轮动作 | 关闭口径 |
|---|---|---|
| **U-05**（51 点击 UI 与设置向导） | U-05-1（设置接入，AC-111~120）与 U-05-2（卡片渲染，AC-121~130）的**离线判据**由本增量实现并取证 | `b4-status.md` §29 登记"U-05-1/U-05-2 离线判据本地 passed"；**U-05 整体保持 open**（真机体验、真实联网不在本轮），不得写 `closed` |
| **U-07**（检索/理解/记忆等） | 不承接、不关闭任何子项；`CardSnapshotSource` 默认空快照，不与检索链路耦合；不实现负反馈、缓存/outbox、按需补全、审计哈希链 | §29 明确"U-07-41.5 / 42 / 43 / 52 仍 open" |
| **U-08**（真实端点/真机） | 9 个 `#[ignore]` 数量与名称不变；生产探针默认不启用（`llm-https` 关闭时返回 `Unsupported`）；AC-117 只用替身 | §29 明确"U-08 仍 open；不宣称真实公网、真机、限流/额度实测" |

### 0.5 公共断言（每个 shell 会话执行一次，后续任务直接引用）

```bash
set -uo pipefail
ROOT="$(git rev-parse --show-toplevel)"; cd "$ROOT"
export TEMP="$ROOT/.tmp/ui-integration"; mkdir -p "$TEMP"

dto_block()   { sed -n "/struct $1/,/^}/p" app/src-tauri/src/ipc/dto.rs; }
field_count() { dto_block "$1" | grep -cE '^    pub [a-z_A-Z]+:'; }
pblock()      { sed -n "/struct $2/,/^}/p" "$1"; }
pfields()     { pblock "$1" "$2" | grep -cE '^    pub [a-z_A-Z]+:'; }
variant_count() { sed -n "/pub enum $2/,/^}/p" "$1" | grep -cE '^    [A-Z][A-Za-z_]*'; }
new_targets() { grep -m1 '^NEW_TARGETS=' scripts/test-name-inventory.sh | sed -E "s/^NEW_TARGETS='//; s/'$//" | wc -w; }
ac_total()    { grep -ohE 'AC-B4-[0-9]{2,3}' .codeartsdoer/specs/b4_gate/spec.md .codeartsdoer/specs/ci_deny_repair/spec.md .codeartsdoer/specs/ui_integration/spec.md | sort -u | wc -l; }
no_hits()     { local out; out="$(grep -rnE "$1" "${@:2}" 2>/dev/null || true)"; if [[ -n "$out" ]]; then echo "FAIL: $1"; printf '%s\n' "$out"; return 1; fi; return 0; }
rule()        { bash scripts/check-ui-integration.sh "--rule=$1"; }
```

> 每个子任务的**证据**块均假设上述函数已定义；`dto_block` 要求 DTO 结构体块内不出现顶格 `}`。

### 0.6 风险与偏差登记（执行期必读）

| 编号 | 风险/偏差 | 触发点 | 处置 |
|---|---|---|---|
| **R-UI-01** | `windows-sys` 旧稿写 `0.59`、旧建议"优先 0.61"；实况 `Cargo.lock` 已锁 **0.45.0**（jni 闭包）与 0.61.2（tauri 闭包），无 0.59 | `W-UI-03.1` | **固化 0.45.0**：只复用已锁版本、单版本、仅 `cfg(windows)`；feature 仅 `Win32_Security_Credentials`+`Win32_Foundation`；不升级 0.59/0.61、不引入第二版本。版本号调整只登记 `X-UI-11`，不改 API 语义 |
| **R-UI-02** | `windows-sys` feature 名称与锁定版本不符 → 编译失败 | `W-UI-03.2` | 只允许调整 feature 名/版本号；不得改用 `keyring`/DPAPI/自管密文文件 |
| **R-UI-03** | `cargo deny` 对新依赖传递闭包许可证判定失败 | `W-UI-03.5`、`W-UI-18.3` | 实跑取证；若出现未白名单许可证，**停止**并走独立变更单，不得放宽 `deny.toml` |
| **R-UI-04** | `withGlobalTauri` 在 WebView2 + CSP 下注入失败 | `W-UI-09.3`、`W-UI-10.3` | 离线只保证类型与调用路径；若真机失败，前端显示固定"桌面壳不可用"，**不**改 CSP、**不**加 npm 依赖，回退需新裁决 |
| **R-UI-05** | `cargo test -p meeting-copilot-desktop` 需要 `generate_context!` 读取 `app/dist` | `W-UI-18.1` | `W-UI-01.1` 先断言 `app/dist/index.html` 存在；缺失则先 `npm --prefix app run build` |
| **R-UI-06** | `zero-transcript` 扫描 `app/src-tauri/src/ipc/**`，拼接端点 URL 会命中 `https?://` | `W-UI-04.4`、`W-UI-07.2` | 用常量拼接（`SCHEME` + `"://"`），禁止连续 `https://` 字面量；完整 URL 仅在装配层内存 |
| **R-UI-07** | Windows-only 用例在非 Windows 目标不可执行 | `W-UI-03.4`、`W-UI-18.1` | `#[cfg(windows)]` 3 例 + 跨平台 `secure_store_is_windows_only` 1 例；不宣称 mac 可用 |
| **R-UI-08** | 前端 `strict` + `exactOptionalPropertyTypes` 对可空字段报错 | `W-UI-10.1` | DTO 统一 `T \| null`（不用可选属性 `?`）；`IPC_COMMANDS` 用 `as const` |
| **R-UI-09** | `KeywordCardView`（51）含 `citation_alignment`，DTO 10 项含 `disabledReason`，**不是同名一对一** | `W-UI-08.1` | `click_state` → `clickState` + `disabledReason`（拆分）；`citationAlignment` 只进 `CardDetailDto`；禁止直接 `serde` 透传 |
| **R-UI-10** | `ClickState` 是带载荷枚举，`serde` 外部标签形如 `{"disabled":"..."}` | `W-UI-08.1` | DTO 侧手工映射为两个扁平字段；`KeywordCardViewDto` 禁止直接持有 `ClickState` |
| **X-UI-01** | AC-117 第二命令写 `settings_failure_messages_are_humanized`，实况用例名为 `llm_settings_view_probe_events_and_messages` | `W-UI-07.3`、`W-UI-16.1` | 用实况用例名取证；不改 AC 文本；§29 登记替代，防假绿 |
| **X-UI-11** | `windows-sys` 版本 `0.59` → 固化 **`0.45.0`**（不是 `0.61`；见 R-UI-01 / D-B4-P171） | `W-UI-03.1`、`W-UI-16.1` | C1 内实测编译；`Cargo.lock` 不新增 windows-sys 版本；§29 登记；`deny.toml` 仍零 diff |
| **X-UI-12** | `check-ui-integration.sh` 六规则与 `check-b4-gates.sh` 扩面必须在落盘后同批 | `W-UI-14.1`、`W-UI-15.1` | 提交顺序 C6 晚于 C1~C5；禁止先改脚本；退 2 即放行失败（D-B4-P174） |
| **X-UI-13** | 旧稿端口回执 `ProbeAttempt{cooldownUntilMs}`（绝对时间）与 spec §6.8-4 的 `remainingMs`（相对）不一致 | `W-UI-07.1`、`W-UI-07.3`、`W-UI-16.1` | 以 spec 为准：`ProbeAttempt` = `succeeded`/`failure`/`elapsedMs`/`remainingMs`（4 项）；`AppState.cooldown_until_ms` 降为内部镜像（`now_ms + remaining_ms`），**不跨端口、不进 DTO**；单测断言推导关系，防重复计时 |

### 0.7 提交顺序（7 个提交，均不 push）

| 提交 | 内容（主任务） | 建议 message | 门禁前置 |
|---|---|---|---|
| C1 | W-UI-01、W-UI-02、W-UI-03 | `feat(settings): 落地应用层端口与 Windows 凭据存储` | `cargo test -p meeting-copilot-application`、`-p meeting-copilot-platform-win`、`cargo deny check licenses` |
| C2 | W-UI-04 | `feat(ipc): 建立 6 命令 DTO 与运行态骨架` | `cargo check -p meeting-copilot-desktop` |
| C3 | W-UI-05、W-UI-06、W-UI-07、W-UI-08 | `feat(ipc): 落地设置与卡片命令实现` | `cargo test -p meeting-copilot-desktop ipc_` |
| C4 | W-UI-09 | `feat(ipc): 注册 6 命令并启用 withGlobalTauri` | `cargo build -p meeting-copilot-desktop` |
| C5 | W-UI-10、W-UI-11、W-UI-12、W-UI-13 | `feat(ui): 设置向导与关键词卡片只读渲染` | `node scripts/check-bindings.mjs`、`npm --prefix app run typecheck` |
| C6 | W-UI-14、W-UI-15 | `chore(gates): 新增六规则门禁并扩展 B4 扫描面` | 六规则 ×0、7 规则 ×0 |
| C7 | W-UI-16、W-UI-17 | `docs(status): 登记 U-05 产品化接入实现快照` | 文档 grep 断言；`U-05` 仍登记 `open` |

> `C6` 必须在 `C1~C5` 之后：否则 `require_core_faces()` 与 `check-ui-integration.sh` 的扫描面缺失会退 2。C7 后由用户决定是否 push（`AC-B4-109` 远端取证不在本轮）。
> 七个提交的内容归属与 spec §7.7.3 / design §2.5.6 逐字一致；`W-UI-18` 只负责编排这七个提交并在末次提交后补跑全量回归，**不产生第 8 个提交**。每个提交后复核 `NEW_TARGETS`=25。

### 0.8 本轮校准对齐点（2026-11-23：spec §6.7/§6.8/§7.7 ↔ design §2.8.5 ↔ 本文任务）

| # | 对齐点 | spec/design 依据 | 本文落点 | 不变式 |
|---|---|---|---|---|
| 1 | `windows-sys` 固化 **0.45.0**、单版本、仅 `cfg(windows)`、feature 仅 `Win32_Security_Credentials`+`Win32_Foundation` | spec §9.2 C-UI-04、§9.4.5 R-UI-01/X-UI-11；design §2.5.3、D-B4-P171 | §0.1、§0.6、W-UI-03.1 | 不升级 0.59/0.61；`Cargo.lock` 不新增该 crate 版本；`deny.toml` 零 diff |
| 2 | 端口纯数据 6 类封闭：`SettingsConfig` 4/`SecretBytes` 无载荷/`ProbeCall` 单次/`ProbeAttempt` 4/`ProbeFailure` 8/`PortError` 4 | spec §6.8-1~6；design §2.2.3 G、D-B4-P173 | W-UI-02.2、W-UI-02.4 | 新增字段/变体即结构违规；`SecretBytes` 无 serde/长度/明文访问器 |
| 3 | 事实源 3 个、快照 `6/2/6` | spec §6.8 事实源 1~4；design §2.2.3 H | W-UI-02.3、W-UI-02.4 | 不新增第 4 事实源、第 7 项事实；不在读取时合成卡 |
| 4 | `ClickState` 手工拆载为 `clickState`+`disabledReason`，DTO 禁持有 `ClickState`、禁 serde 标签透传 | spec §6.7；design §2.3.3.1、D-B4-P172 | W-UI-04.2、W-UI-08.1 | `KeywordCardViewDto`=10、`CardDetailDto`=5；`citationAlignment` 只进详情 |
| 5 | `ProbeAttempt.remainingMs`（相对）↔ `cooldown_until_ms`（绝对镜像） | spec §6.8-4；design §2.2.3 G/H、§2.3.4-13、X-UI-13 | W-UI-07.1、W-UI-07.3、W-UI-16.1 | 相对值跨端口，绝对值不进 DTO；禁止本地重复计时 |
| 6 | 补证用例名：`secure_store_delete_is_idempotent`、`ipc_probe_cooling_reports_remaining_ms`、`sources_declare_three_ports`、`settings_config_rejects_extra_keys` | spec §7.2 AC-112/117/119 判定列；§9.4.2-8 | §0.2、W-UI-02.4、W-UI-03.4、W-UI-07.3 | 不改 AC 陈述；用例名逐字使用 |
| 7 | AC-B4-117 文案取证用实况 `llm_settings_view_probe_events_and_messages` | spec X-UI-01；design §2.8.1 | W-UI-07.3、W-UI-16.1 | 不改 AC 文本，§29 登记替代，防假绿 |
| 8 | `check-ui-integration.sh` 六规则只在 C6、且晚于 C1~C5 | spec §7.5-2、§7.7.3；design §2.5.6、D-B4-P174 | §0.1、§0.3、§0.7、W-UI-14.1、W-UI-15.1 | 任何退 2 即放行失败 |
| 9 | 六个 IPC 命令闭集（`ipc_version` 不计入） | spec §5.3；design §2.2.2 | W-UI-04.2、W-UI-09.1 | 第 7 个 `settings_*`/`keyword_*` 命令即违规 |
| 10 | DTO 六组 `11/5/4/3/10/4/5`（含 `CardBodyDto` 4） | spec §6.6-2；design §2.3.2 | W-UI-04.2、W-UI-17.2 | 字段只增不改即违规 |
| 11 | `NEW_TARGETS`=25、AC 汇总=130、U-05 整体 `open`、9 个 `#[ignore]` 不变、`tests/replay/` 不解禁 | spec §7.6、§7.7.3；design §2.5.6、§2.8.4 | §0.3、W-UI-16.1、W-UI-17.3、W-UI-18.1 | 不新增 cargo target；不宣称产品可用 |
| 12 | 前端 `withGlobalTauri` + 零新增 npm；CSP/能力零 diff | spec §9.2 C-UI-01；design §2.5.3 | W-UI-09.3、W-UI-10.3 | `app/package.json`/`package-lock.json` 零 diff；无 `connect-src` |

---

## 1. 建立验收基线与公共断言（W-UI-01 · 批次 C1）

**AC 映射**：全部 AC 的取证前提。**依赖**：无。

### 1.1 记录基线快照与构建前置

- **做什么**：在未改任何源码前记录可复现基线；确认前端产物与工具链可用，并核对 `windows-sys` 实况版本集合（0.45.0/0.61.2，无 0.59）。
- **文件**：`.tmp/ui-integration/baseline.txt`（产物，不入库）。
- **禁止**：不修改任何受版本控制文件；不执行 `git commit`。
- **依赖**：无。
- **证据**：
```bash
{
  git rev-parse --short HEAD
  git status --short
  new_targets
  ac_total
  cargo test --workspace -- --list | tail -3
  ls app/dist/index.html
} | tee .tmp/ui-integration/baseline.txt
```
- **DoD**：`git rev-parse --short HEAD` 输出 **`6d5b0b0`**（若已推进，记录实际值并保持"工作区干净"）；`new_targets` = **25**；`ac_total` = **130**；`app/dist/index.html` 存在（缺失则先 `npm --prefix app run build` 并记录）；`git status --short` 为空；`baseline.txt` 生成。

### 1.2 固化公共断言函数

- **做什么**：把 §0.5 的 `dto_block`/`field_count`/`new_targets`/`ac_total`/`no_hits`/`rule` 写入会话脚本 `.tmp/ui-integration/lib.sh`，供后续任务 source。
- **文件**：`.tmp/ui-integration/lib.sh`（产物，不入库）。
- **禁止**：不把 helper 写入 `scripts/**`（`scripts` 只允许新增 `check-ui-integration.sh` 与扩 `check-b4-gates.sh`）。
- **依赖**：W-UI-01.1。
- **证据**：
```bash
bash -c 'set -uo pipefail; source .tmp/ui-integration/lib.sh; new_targets; ac_total'
```
- **DoD**：输出两行分别为 `25` 与 `130`；退出码 0。

### 1.3 建立 AC 覆盖看板

- **做什么**：生成"AC → 主任务 → 用例名 → 取证命令"的临时看板，逐条勾选；作为 W-UI-18 收口输入。
- **文件**：`.tmp/ui-integration/ac-board.md`（产物，不入库；内容抄 §0.2 + 每条证据命令）。
- **禁止**：看板不写入 `.codeartsdoer/specs/**`；正式登记在 W-UI-16。
- **依赖**：W-UI-01.1。
- **证据**：
```bash
grep -cE '^\| AC-B4-1[12][0-9]' .tmp/ui-integration/ac-board.md
```
- **DoD**：计数 = **20**（AC-B4-111~130 全覆盖）。

---

## 2. 定义应用层端口与只读事实源（W-UI-02 · 批次 C1）

**AC 映射**：AC-B4-119（E1 三端口在 application / E2 事实源 3 个 / E4 端口层 0 IO·网络·时钟 / E5 纯数据封闭；另为 AC-114/115/121 提供类型）。**依赖**：W-UI-01。
**批次**：C1。**落点**：`crates/application/src/settings/{mod,ports,sources}.rs`；`crates/application/Cargo.toml` 零 diff。

### 2.1 声明 settings 模块

- **做什么**：新建 `crates/application/src/settings/mod.rs`，声明 `pub mod ports; pub mod sources;` 并 re-export 端口与纯数据。
- **文件**：`crates/application/src/settings/mod.rs`（新增）、`crates/application/src/lib.rs`（+1 行 `pub mod settings;`）。
- **禁止**：`lib.rs` 除该 1 行外零改；不新增依赖到 `crates/application/Cargo.toml`。
- **依赖**：无（本任务先落盘模块声明，供 2.2/2.3 引用）。
- **证据**：
```bash
grep -c '^pub mod settings;' crates/application/src/lib.rs              # 1
git diff --quiet HEAD -- crates/application/Cargo.toml; echo $?         # 0
```
- **DoD**：`lib.rs` 命中 = 1；`crates/application/Cargo.toml` 零 diff（退出码 0）。

### 2.2 定义三端口 trait 与端口纯数据（6 类封闭）

- **做什么**：在 `crates/application/src/settings/ports.rs` 定义 3 个 `Send + Sync` trait 与 6 类纯数据，逐项按 spec §6.8 / design §2.2.3 G 封闭：
  1. `PortError`：**恰 4 类**（`Unsupported`/`Denied`/`Unavailable`/`Corrupted`），PascalCase 变体；禁止第 5 类；禁止携带 Key/端点/内部码。
  2. `SecretBytes`：只承载一次性凭据字节；**禁止** `Serialize`/`Deserialize`/`Display`；禁止 `len()`/前缀/后缀/明文取值访问器；`Debug` 恒输出固定掩码；不得 `Clone` 进全局状态。
  3. `SettingsConfig`：**恰 4 项**（`schema_version`/`model_index`/`endpoint_index`/`timeout_ms`）+ `Default = 1/0/0/6000` + `normalized()`（下标越界归 0，`timeout_ms` 越界归 6 000，区间 `100..=8_000`）；`#[serde(deny_unknown_fields)]`；禁止第 5 键、禁止字符串/Map 承载。
  4. `ProbeCall`：`credential: SecretBytes`、`model_name`、`endpoint_index`、`timeout_ms`；只在单次命令栈内存在；禁止进入 `AppState`/配置/日志/审计；不 `Serialize`。
  5. `ProbeAttempt`：**恰 4 项**（`succeeded`、`failure: Option<ProbeFailure>`、`elapsed_ms`、`remaining_ms: Option<u64>`）；`succeeded=true ⇒ failure=None`；`remaining_ms>0` 才表示冷却；**禁止**第 5 字段、禁止 `cooldown_until_ms` 绝对时间、禁止内部错误码文本。
  6. `ProbeFailure`：**恰 8 态**（`MissingKey`/`KeyRejected`/`EndpointUnreachable`/`Timeout`/`RateLimited`/`QuotaExceeded`/`ServerError`/`Unknown`），与 51 `ProbeFailureKind` 同名同序、恒等转换；禁止第 9 态、合并、改名或携带文本载荷。
  7. 3 个 trait：`SecureStore`（write/read/delete，删除幂等）、`SettingsConfigStore`（load/save/reset）、`ProbeExecutor`（execute）。
- **文件**：`crates/application/src/settings/ports.rs`（新增，约 200~260 行）。
- **禁止**：不出现 `std::fs`/`std::env`/`reqwest`/`TcpStream`/`std::net`/`Instant::now`/`SystemTime::now`（E4 `no_hits`）；不出现平台句柄、`serde_json::Value`、`HashMap` 业务载荷；`SecretBytes` 不得有载荷访问器；`PortError` 不得 `Display` 输出敏感信息。
- **依赖**：W-UI-02.1。
- **证据**：
```bash
grep -cE '^pub trait (SecureStore|SettingsConfigStore|ProbeExecutor)' crates/application/src/settings/ports.rs   # 3
pfields crates/application/src/settings/ports.rs SettingsConfig        # 4
pfields crates/application/src/settings/ports.rs ProbeAttempt          # 4
variant_count crates/application/src/settings/ports.rs ProbeFailure    # 8
variant_count crates/application/src/settings/ports.rs PortError       # 4
pblock crates/application/src/settings/ports.rs SecretBytes | grep -cE 'Serialize|Deserialize|Display|prefix|suffix|as_str|to_string'   # 0
grep -c 'impl fmt::Debug for SecretBytes' crates/application/src/settings/ports.rs         # 1
grep -c 'deny_unknown_fields' crates/application/src/settings/ports.rs                    # >=1（SettingsConfig 块）
grep -cE 'fn normalized' crates/application/src/settings/ports.rs                            # 1
grep -cE 'cooldown_until|cooldownUntil' crates/application/src/settings/ports.rs          # 0（相对剩余口径）
no_hits 'std::fs|std::env|reqwest|TcpStream|std::net|Instant::now|SystemTime::now' crates/application/src/settings/ports.rs
```
- **DoD**：trait = **3**；`SettingsConfig`=**4**、`ProbeAttempt`=**4**、`ProbeFailure`=**8 态**、`PortError`=**4 类**；`SecretBytes` 块内 serde/长度/明文访问器命中 = **0**、`Debug` 实现 = **1**；`normalized()` = **1**；`cooldown_until*` = **0**；E4 `no_hits` 退 0。

### 2.3 定义三个只读事实源与快照类型（6/2/6）

- **做什么**：在 `crates/application/src/settings/sources.rs` 定义 3 个事实源 trait 与只读快照，逐项按 spec §6.8 事实源 1~4 / design §2.2.3 H 封闭：
  1. `FactsSnapshot`：**恰 6 项**（`has_stored_key`/`model_count`/`endpoint_count`/`now_ms`/`cooldown_until_ms: Option<u64>`/`probe_in_flight`）；`has_stored_key` 只判存在；`cooldown_until_ms` 为**绝对毫值内部镜像**（与端口回执的相对 `remaining_ms` 区分）；禁止第 7 项、禁止可写。
  2. `ViewFactsSnapshot`：**恰 6 项**（`session_matches`/`authorization_revoked`/`reference_stale`/`screen_share`/`emergency_masked`/`visible_card_count`）；只来自注入。
  3. `CardSnapshotEntry`：**恰 2 态**（`Draft(KeywordCardDraft)` 13 字段草案 / `Preview(ComposedCard)` 11 项预览恒不可点）；不得在读取时合成卡片。
  4. 3 个 trait：`SettingsFactsSource`（`facts()`）、`CardSnapshotSource`（`cards()`）、`ViewFactsSource`（`view_facts()`），均 `Send + Sync`、只读。
- **文件**：`crates/application/src/settings/sources.rs`（新增，约 130~190 行）。
- **禁止**：不实现真实投屏/遮蔽 API；不在读取时合成卡片；不触发检索/LLM/审计；不新增第 4 个事实源、第 7 项事实；不出现 `std::fs`/`std::env`/网络/时钟直读（时钟由 `now_ms` 注入）。
- **依赖**：W-UI-02.1、W-UI-02.2（共用 `PortError` 语义）。
- **证据**：
```bash
grep -cE '^pub trait (SettingsFactsSource|CardSnapshotSource|ViewFactsSource)' crates/application/src/settings/sources.rs   # 3
pfields crates/application/src/settings/sources.rs FactsSnapshot        # 6
pfields crates/application/src/settings/sources.rs ViewFactsSnapshot    # 6
variant_count crates/application/src/settings/sources.rs CardSnapshotEntry   # 2
no_hits 'std::fs|std::env|reqwest|TcpStream|std::net|Instant::now|SystemTime::now' crates/application/src/settings/sources.rs
```
- **DoD**：trait = **3**；`FactsSnapshot`=**6**、`ViewFactsSnapshot`=**6**、`CardSnapshotEntry`=**2 态**；`sources.rs` IO/网络/时钟命中 = **0**。

### 2.4 添加端口/事实源单测（3 例，含 E5 封闭断言）

- **做什么**：在 `ports.rs` / `sources.rs` 的 `#[cfg(test)]` 内实现：
  1. `application_declares_three_ports`：断言 `SecureStore`/`SettingsConfigStore`/`ProbeExecutor` 三个 trait 名与 `Send + Sync` 约束存在（`include_str!` 源码字符串），且端口层不出现平台类型。
  2. `settings_config_rejects_extra_keys`：断言 `SettingsConfig` 恰 4 字段、`Default = 1/0/0/6000`、`normalized()` 对越界下标归 0 / `timeout_ms` 越界归 6 000（`100..=8_000`）、`deny_unknown_fields` 命中；并断言 `ProbeFailure` 恰 8 态、`PortError` 恰 4 类、`SecretBytes` 无载荷访问器。
  3. `sources_declare_three_ports`：断言 3 个事实源 trait 名、`FactsSnapshot`/`ViewFactsSnapshot` 各 6 项、`CardSnapshotEntry` 恰 2 态（`Draft`/`Preview`）。
- **文件**：同上两文件的 `#[cfg(test)]` 模块。
- **禁止**：不新增测试文件、不新增 cargo target（`NEW_TARGETS` 保持 25）；测试不写真实凭据形态；不用 `#[allow]` 规避告警。
- **依赖**：W-UI-02.2、W-UI-02.3。
- **证据**：
```bash
cargo test -p meeting-copilot-application application_declares_three_ports
cargo test -p meeting-copilot-application settings_config_rejects_extra_keys
cargo test -p meeting-copilot-application sources_declare_three_ports
new_targets   # 25
```
- **DoD**：3 个用例各退 0；`new_targets` = **25**；用例内断言 `4/4/8/4`（`SettingsConfig`/`ProbeAttempt`/`ProbeFailure`/`PortError`）与 `6/2/6`（`FactsSnapshot`/`CardSnapshotEntry`/`ViewFactsSnapshot`）全部成立。

---

## 3. 接入 Windows 凭据管理器（W-UI-03 · 批次 C1）

**AC 映射**：AC-B4-112、AC-B4-119。**依赖**：W-UI-02（`SecureStore`/`SecretBytes`/`PortError`）。

### 3.1 声明 platform-win 依赖与模块

- **做什么**：`crates/platform-win/Cargo.toml` 增 `meeting-copilot-application = { path = "../application" }`，并在 `[target.'cfg(windows)'.dependencies]` 增 `windows-sys = { version = "0.45.0", features = ["Win32_Security_Credentials", "Win32_Foundation"] }`；`src/lib.rs` +1 行 `pub mod secure_store;`；`PLATFORM_NAME` 不变。
- **文件**：`crates/platform-win/Cargo.toml`、`crates/platform-win/src/lib.rs`。
- **禁止**：`windows-sys` 不得放在普通 `[dependencies]`（保证非 Windows 不编译）；**版本必须是已锁的 `0.45.0`**，不得写 0.59/0.61、不得引第二版本、不得写 wildcard（`bans.wildcards=deny`）；不新增 `keyring`/`dpapi`/`windows` 其它 crate；不改 `resources/capability_matrix.toml`。
- **依赖**：W-UI-02.2。
- **证据**：
```bash
grep -c '^pub mod secure_store;' crates/platform-win/src/lib.rs              # 1
grep -c "target.'cfg(windows)'.dependencies" crates/platform-win/Cargo.toml  # 1
grep -c 'version = "0.45.0"' crates/platform-win/Cargo.toml                  # 1
grep -cE 'version = "0\.(59|61)' crates/platform-win/Cargo.toml              # 0
grep -A1 'name = "windows-sys"' Cargo.lock | grep 'version = "0.59' | wc -l  # 0（不新增 0.59）
grep -c 'Win32_Security_Credentials' crates/platform-win/Cargo.toml          # 1
grep -c 'Win32_Foundation' crates/platform-win/Cargo.toml                    # 1
git diff --quiet HEAD -- crates/platform-win/resources/capability_matrix.toml; echo $?   # 0
```
- **DoD**：模块声明 = 1；Windows 目标依赖段 = 1；版本 `0.45.0` = **1** 且 0.59/0.61 = **0**；`Cargo.lock` 无新增 0.59 条目；两个 feature 各 = 1（且仅这两个）；能力矩阵零 diff。
- **版本口径**：按 **R-UI-01 / X-UI-11** 固化 **`0.45.0`**（复用已锁版本、单版本、仅 `cfg(windows)`）；feature 名与 0.45.0 feature 表不符时**只调 feature 名**，不改 API 语义；不得升级、不得引入第二版本。

### 3.2 实现凭据管理器三操作

- **做什么**：`crates/platform-win/src/secure_store.rs` 实现 `WindowsCredentialStore`（`cfg(windows)`）：UTF-8 字节写入、同名覆盖、读取返回 `Option<SecretBytes>`、删除 NotFound 视为成功；凭据类型 `CRED_TYPE_GENERIC`、持久化 `CRED_PERSIST_LOCAL_MACHINE`；唯一目标名常量（全仓 1 个）。
- **文件**：`crates/platform-win/src/secure_store.rs`（新增，约 200~280 行）。
- **禁止**：不出现 `std::fs`、`keyring`、DPAPI、自管密文文件；目标名不得第二处出现；`Debug` 不得输出字节内容。
- **依赖**：W-UI-03.1。
- **证据**：
```bash
grep -c 'CredWriteW' crates/platform-win/src/secure_store.rs        # >=1
grep -c 'CredReadW' crates/platform-win/src/secure_store.rs         # >=1
grep -c 'CredDeleteW' crates/platform-win/src/secure_store.rs       # >=1
grep -c 'CRED_TYPE_GENERIC' crates/platform-win/src/secure_store.rs # >=1
grep -c 'CRED_PERSIST_LOCAL_MACHINE' crates/platform-win/src/secure_store.rs  # >=1
grep -c 'std::fs' crates/platform-win/src/secure_store.rs           # 0
grep -rc 'keyring' crates/platform-win/src | grep -v ':0$' | wc -l  # 0
```
- **DoD**：5 个 API 常量/函数命中均 ≥1；`std::fs` = **0**；`keyring` 文件数 = **0**。

### 3.3 提供非 Windows `Unsupported` 空壳

- **做什么**：`cfg(not(windows))` 同名 `WindowsCredentialStore` 实现三个方法，统一返回 `PortError::Unsupported`；类型与 trait 签名保持一致。
- **文件**：`crates/platform-win/src/secure_store.rs`（同文件另一 `cfg` 块）。
- **禁止**：不出现 `target_os = "macos"`；不宣称可用；不新增 mac 依赖。
- **依赖**：W-UI-03.2。
- **证据**：
```bash
grep -c 'Unsupported' crates/platform-win/src/secure_store.rs                 # >=1
grep -c 'target_os = "macos"' crates/platform-win/src/secure_store.rs         # 0
bash scripts/check-mac-scope.sh --rule=claims                                  # 0
```
- **DoD**：`Unsupported` ≥1；mac 分支 = **0**；mac 宣称门禁退 0。

### 3.4 添加凭据单测（3 + 1，含删除幂等）

- **做什么**：`#[cfg(windows)]` 下实现 `secure_store_roundtrip_uses_fixed_target`（写→读→删，独立测试目标名，结束即删）、`secure_store_write_is_idempotent`（两次写入后仍单条目）、`secure_store_delete_is_idempotent`（删除不存在目标返回 `Ok`，且第二次删除不报错）；跨平台实现 `secure_store_is_windows_only`（断言 `cfg!(windows)` 与源码不含 mac 分支）。前 3 例对应 AC-B4-112 的"读写删 + 幂等"四条判据。
- **文件**：`secure_store.rs` 的 `#[cfg(test)]`。
- **禁止**：测试不得使用真实凭据形态、不得留下条目、不得新增测试文件/target；不得把 `NotFound` 当错误。
- **依赖**：W-UI-03.2、W-UI-03.3。
- **证据**：
```bash
cargo test -p meeting-copilot-platform-win secure_store_roundtrip_uses_fixed_target
cargo test -p meeting-copilot-platform-win secure_store_write_is_idempotent
cargo test -p meeting-copilot-platform-win secure_store_delete_is_idempotent
cargo test -p meeting-copilot-platform-win secure_store_is_windows_only
bash scripts/scan-secrets.sh
```
- **DoD**：Windows 下 4 例全退 0（非 Windows 预期 1 例通过、3 例 `cfg` 不编译）；`delete_is_idempotent` 两次删除均 `Ok`；`scan-secrets.sh` 退 0。

### 3.5 取得 deny 许可证证据

- **做什么**：实跑 `cargo deny check licenses bans sources advisories`，确认新依赖闭包命中既有 15 项 allow；记录放行证据与条目数。
- **文件**：无源码改动；证据写入 `.tmp/ui-integration/deny.txt`（W-UI-16 引用）。
- **禁止**：不得修改 `deny.toml`（AC-129 零 diff）；不得放宽 allow/新增 exceptions。
- **依赖**：W-UI-03.1（依赖已解析）。
- **证据**：
```bash
cargo deny check licenses bans sources advisories 2>&1 | tee .tmp/ui-integration/deny.txt
grep -c '^  "' deny.toml   # 记为 allow 条目数（15）
git diff --quiet HEAD -- deny.toml; echo $?
```
- **DoD**：`cargo deny` 退 0；`deny.toml` 零 diff（退出码 0）；如失败按 **R-UI-03** 停止并上报。

---

## 4. 建立装配层依赖、DTO、状态与目录清单（W-UI-04 · 批次 C2）

**AC 映射**：AC-B4-111（字段封闭）、AC-B4-121（字段封闭）、AC-B4-126（config）。**依赖**：W-UI-02、W-UI-03。

### 4.1 声明装配层依赖与 feature

- **做什么**：`app/src-tauri/Cargo.toml` 增 `meeting-copilot-application`、`meeting-copilot-platform-win`、`meeting-copilot-infrastructure`（`default-features = false, features = ["bundled-sqlite"]`）、`serde.workspace`、`serde_json.workspace`；新增 `[features] llm-https = ["meeting-copilot-infrastructure/llm-https"]`（默认不启用）。
- **文件**：`app/src-tauri/Cargo.toml`、`Cargo.lock`（由 cargo 解析）。
- **禁止**：不把 `llm-https` 放进 `default`；不加 `@tauri-apps/api`；不改 `ui-bridge` 依赖段。
- **依赖**：W-UI-02、W-UI-03。
- **证据**：
```bash
grep -c 'meeting-copilot-application' app/src-tauri/Cargo.toml      # >=1
grep -c 'meeting-copilot-platform-win' app/src-tauri/Cargo.toml     # >=1
grep -c 'llm-https = ' app/src-tauri/Cargo.toml                     # 1
grep -n 'default = ' app/src-tauri/Cargo.toml                       # 不含 llm-https
git diff --quiet HEAD -- crates/ui-bridge/Cargo.toml; echo $?       # 0
```
- **DoD**：三依赖各命中 ≥1；`llm-https` feature 定义 = **1** 且不在 `default`；`ui-bridge/Cargo.toml` 零 diff。

### 4.2 定义 6 组只读 DTO 与 4 个输入 DTO（含扁平拆载）

- **做什么**：`app/src-tauri/src/ipc/dto.rs` 定义：
  1. `SettingsSnapshotDto`（**11**）：`form_state`/`has_key`/`key_display`/`model_choice`/`endpoint_choice`/`inline_errors`/`can_test`/`can_save`/`sections_order`/`advanced_collapsed`/`probe_report`。
  2. `ProbeReportDto`（**5**）：`succeeded`/`failure: Option<String>`（8 类 snake_case）/`message`/`elapsed_ms`/`remaining_ms: Option<u64>`（相对值，X-UI-13）。
  3. `SettingsSaveReceiptDto`（**4**）：`persisted`/`fallback: Option<String>`/`has_key`/`notice`。
  4. `SettingsClearReceiptDto`（**3**）：`cleared`/`has_key`/`notice`。
  5. `KeywordCardViewDto`（**10**）：`card_id`/`keyword_kind`/`trigger`/`triggered_at_ms`/`card_status`/`body_visible`/`body: Option<CardBodyDto>`/`click_state: String`（`enabled`/`disabled`）/`disabled_reason: Option<String>`（6 类）/`badge_count`；**`click_state` 与 `disabled_reason` 是扁平两字段，禁止持有 `ClickState` 类型**。
  6. `CardBodyDto`（**4**）：`title`/`conclusion`/`confidence`/`reference_ids`。
  7. `CardDetailDto`（**5**）：`card_id`/`title`/`conclusion`/`reference_ids`/`citation_alignment`（**唯一持有该字段的 DTO**）。
  8. 输入 DTO：`ProbeInputDto`/`SaveInputDto`/`ClearInputDto`/`OpenInputDto`；辅助联合：`ModelChoiceDto`/`EndpointChoiceDto`/`ConclusionDto`。
  9. 全部 `#[serde(rename_all = "camelCase", deny_unknown_fields)]`，`Option` 序列化为 `null`。
- **文件**：`app/src-tauri/src/ipc/dto.rs`（新增）。
- **禁止**：不出现禁用字段 `apiKey`/`keyLen`/`keyPrefix`/`keySuffix`/`endpointRaw`/`url`/`host`/`modelRaw`/`requestBody`/`requestId`/`normalizedTerm`/`gateDecisionRef`/`traceId`；**禁止** `ClickState` 类型字段、`#[serde(tag=...)]`/`#[serde(flatten)]` 间接透传、`serde_json::Value`/`HashMap` 转存枚举；不新增 `citation_alignment` 到 `KeywordCardViewDto`；字段一律 4 空格缩进 `pub`；结构体块内不得出现顶格 `}`。
- **依赖**：W-UI-04.1。
- **证据**：
```bash
field_count SettingsSnapshotDto        # 11
field_count ProbeReportDto             # 5
field_count SettingsSaveReceiptDto     # 4
field_count SettingsClearReceiptDto    # 3
field_count KeywordCardViewDto         # 10
field_count CardBodyDto                # 4
field_count CardDetailDto              # 5
dto_block SettingsSnapshotDto | grep -cE 'apiKey|keyLen|keyPrefix|keySuffix|endpointRaw|url|host|modelRaw|requestBody|requestId'   # 0
dto_block KeywordCardViewDto  | grep -cE 'normalizedTerm|gateDecisionRef|traceId|url|model|requestId|apiKey'                    # 0
dto_block KeywordCardViewDto  | grep -c 'ClickState'          # 0（禁止持有带载荷枚举）
dto_block KeywordCardViewDto  | grep -c 'citation_alignment'  # 0（只进详情）
dto_block CardDetailDto       | grep -c 'citation_alignment'  # 1
grep -cE 'serde\(tag|serde\(flatten' app/src-tauri/src/ipc/dto.rs   # 0
```
- **DoD**：7 个字段计数逐项等于 **11/5/4/3/10/4/5**；两组禁用字段命中 = **0**；`ClickState` 在 10 项块内 = **0**；`citation_alignment` 在快照块 = **0**、在详情块 = **1**；`serde(tag/flatten` = **0**。

### 4.3 定义运行态 `AppState` 与诊断计数

- **做什么**：`app/src-tauri/src/ipc/state.rs` 定义 `AppState`（6 个端口/事实源 `Arc<dyn ...>`、`catalog`、`memory_key: Mutex<Option<String>>`、`form_state: Mutex<SettingsFormState>`、`probe_in_flight: AtomicBool`、`cooldown_until_ms: Mutex<Option<u64>>`、`last_probe_report: Mutex<Option<ProbeReportDto>>`、`counters: Mutex<CommandCounters>`）；`ProbeGuard` 实现 `Drop` 复位；`CommandCounters` 为 6 组成功/拒绝 `u64`。
- **文件**：`app/src-tauri/src/ipc/state.rs`（新增）。
- **禁止**：不持有 Key 明文以外的凭据副本（仅 `memory_key` 降级）；不加第 7 项计数；不打印任何字段值。
- **依赖**：W-UI-04.2（`ProbeReportDto`）。
- **证据**：
```bash
grep -c 'pub struct AppState' app/src-tauri/src/ipc/state.rs                # 1
grep -c 'impl Drop for ProbeGuard' app/src-tauri/src/ipc/state.rs           # 1
grep -cE '^\s+pub [a-z_]+: u64,' <(sed -n '/pub struct CommandCounters/,/^}/p' app/src-tauri/src/ipc/state.rs)  # 12
```
- **DoD**：`AppState` = 1；`Drop` 复位 = 1；计数字段 = **12**（6 成功 + 6 拒绝）。

### 4.4 定义目录清单（不新增来源、不内联模型名/URL 原串）

- **做什么**：`app/src-tauri/src/ipc/catalog.rs` 定义 `SettingsCatalog{presets,endpoints}` 与 `EndpointTarget{display_name,url}`；`from_injected(presets: Vec<String>)` 生成 endpoints（既有白名单第 1 项 + `PATH_CONSTANT`，显示名固定"默认服务"）；手写 `Debug` 只输出下标与显示名。
- **文件**：`app/src-tauri/src/ipc/catalog.rs`（新增）。
- **禁止**：不新增 `LLM_*` 变量名；仓库内 0 模型名字面量；不出现连续 `https://` 字面量（**R-UI-06**）；`Debug` 不输出完整 URL/host。
- **依赖**：W-UI-04.2。
- **证据**：
```bash
grep -c 'LLM_' app/src-tauri/src/ipc/catalog.rs          # 0
grep -c 'https://' app/src-tauri/src/ipc/catalog.rs      # 0
grep -c '默认服务' app/src-tauri/src/ipc/catalog.rs      # 1
bash scripts/check-b4-gates.sh --rule=zero-transcript    # 0（扩面后由 W-UI-15 覆盖）
```
- **DoD**：`LLM_` = 0；`https://` = 0；显示名 = 1；`zero-transcript` 退 0（W-UI-15 完成后复跑）。

---

## 5. 实现非敏感配置原子读写（W-UI-05 · 批次 C3）

**AC 映射**：AC-B4-113。**依赖**：W-UI-04。

### 5.1 实现 `SettingsFileV1` 与原子写/损坏回默认

- **做什么**：`app/src-tauri/src/ipc/settings_store.rs` 定义 `SettingsFileV1`（恰 4 键：`schema_version`/`model_index`/`endpoint_index`/`timeout_ms`）；`FileSettingsConfigStore` 实现 `SettingsConfigStore`：`load`（JSON 非法或 `schemaVersion != 1` → 写 `.bak`（仅 4 键）→ 返回默认，不 panic）、`save`（`settings.json.tmp` + 同目录替换，失败保留旧文件）、`reset`（写回默认 4 键）。
- **文件**：`app/src-tauri/src/ipc/settings_store.rs`（新增，约 200~260 行）。
- **禁止**：不出现凭据/地址/主机名/模型名/显示名/请求标识键；不先删旧文件；不 panic。
- **依赖**：W-UI-04.1、W-UI-04.2。
- **证据**：
```bash
sed -n '/struct SettingsFileV1/,/^}/p' app/src-tauri/src/ipc/settings_store.rs | grep -cE 'apiKey|secret|endpointRaw|url|hostName|modelName|displayName|requestId'  # 0
grep -c 'settings.json.tmp' app/src-tauri/src/ipc/settings_store.rs   # >=1
grep -c 'std::fs::rename' app/src-tauri/src/ipc/settings_store.rs     # >=1
```
- **DoD**：禁止键 = **0**；临时文件与同目录替换各 ≥1。

### 5.2 添加配置存储单测

- **做什么**：实现 `ipc_settings_file_has_only_four_keys`（临时目录写入后解析 JSON，键数 = 4 且键名逐字匹配）、`ipc_settings_corruption_falls_back_to_defaults`（写入非法 JSON 与 `schemaVersion=2`，断言返回默认、生成 `.bak`、不 panic）。
- **文件**：`settings_store.rs` 的 `#[cfg(test)]`。
- **禁止**：不使用真实用户配置目录；不新增测试文件/target。
- **依赖**：W-UI-05.1。
- **证据**：
```bash
cargo test -p meeting-copilot-desktop ipc_settings_file_has_only_four_keys
cargo test -p meeting-copilot-desktop ipc_settings_corruption_falls_back_to_defaults
```
- **DoD**：2 例退 0；用例内断言 JSON 键集合 = `{schemaVersion, modelIndex, endpointIndex, timeoutMs}`。

---

## 6. 实现设置快照/保存/清除命令（W-UI-06 · 批次 C3）

**AC 映射**：AC-B4-111、AC-B4-115、AC-B4-118。**依赖**：W-UI-05、W-UI-07（探针回执由 07 提供；06 的 `probe` 组装可在 07 完成后接线）。

### 6.1 实现 `settings_snapshot_impl`

- **做什么**：读取 `facts` 与 `config`，用 `SettingsDefaults{model_index:config.model_index, endpoint_index:config.endpoint_index, timeout_ms:config.timeout_ms, auto_save_on_verified:true}` 调 `apply_defaults` 得到输入，再调 `project_settings_view` 投影 8 项；映射为 11 项 DTO（`sectionsOrder=SECTION_ORDER`、`advancedCollapsed=true`、`probeReport=state.last_probe_report` 且仅 `failed`/`cooling` 非空）。
- **文件**：`app/src-tauri/src/ipc/settings.rs`（新增，含 `fn settings_snapshot_impl`）。
- **禁止**：函数名 `settings_snapshot` 前缀只允许 1 处（其余辅助函数以 `snapshot_*` 命名）；不写存储；读取失败按 `hasKey=false`/默认值降级；不返回 Key/地址/模型原串。
- **依赖**：W-UI-05.1、W-UI-04.2/4.3/4.4。
- **证据**：
```bash
grep -c 'fn settings_snapshot' app/src-tauri/src/ipc/settings.rs   # 1
grep -c 'project_settings_view' app/src-tauri/src/ipc/settings.rs  # >=1
grep -c 'SECTION_ORDER' app/src-tauri/src/ipc/settings.rs          # >=1
```
- **DoD**：`fn settings_snapshot` 前缀 = **1**；51 投影与四段顺序常量各 ≥1。

### 6.2 实现 `settings_save_impl`（先安全存储、后配置、失败降级）

- **做什么**：`SecretInput::from_draft(&key)` → `draft_issue()` 非空则返回字段问题错误；`secure.write(&SecretBytes)` 成功则清 `memory_key` 并写 4 键配置；安全存储失败则 `memory_key=Some(key)` 且仍写配置，回执 `persisted=false`/`fallback="memory"`/`notice=PERSIST_FALLBACK_NOTICE`；配置写失败保留旧配置并回执"设置未保存"。
- **文件**：`app/src-tauri/src/ipc/settings.rs`。
- **禁止**：失败不得写明文/静默成功；`notice` 不得含 Key；不新增 IPC 调用。
- **依赖**：W-UI-06.1、W-UI-05.1。
- **证据**：
```bash
grep -c 'PERSIST_FALLBACK_NOTICE' app/src-tauri/src/ipc/settings.rs   # >=1
grep -c 'fallback' app/src-tauri/src/ipc/settings.rs                  # >=1
grep -c 'memory_key' app/src-tauri/src/ipc/settings.rs                # >=1
```
- **DoD**：三项各 ≥1；降级分支在 6.4 用例覆盖。

### 6.3 实现 `settings_clear_impl`（确认 + 幂等）

- **做什么**：`confirm != true` → `Err(IpcErrorDto{code:"confirmation_required", retryable:false})` 且状态不变；`confirm=true` → `secure.delete()`（NotFound 视为成功）+ `config.reset()` + `memory_key=None` + `last_probe_report=None` + `form_state=NotConfigured`，回执 `cleared=true`/`hasKey=false`。
- **文件**：`app/src-tauri/src/ipc/settings.rs`。
- **禁止**：未确认不得产生任何副作用；删除失败不得半配置。
- **依赖**：W-UI-06.1。
- **证据**：
```bash
grep -c 'confirmation_required' app/src-tauri/src/ipc/settings.rs  # >=1
grep -c 'config.reset' app/src-tauri/src/ipc/settings.rs            # >=1（或 .reset() 调用）
```
- **DoD**：两项各 ≥1；幂等由 6.4 用例覆盖。

### 6.4 添加设置命令单测（9 例）

- **做什么**：实现 `ipc_settings_snapshot_is_closed_and_keyless`、`ipc_settings_snapshot_without_key_is_not_configured`、`save_receipt_reports_memory_fallback_without_plaintext`（替身 `SecureStore` 返回 `Denied`）、`clear_requires_confirmation_and_deletes_target`、`clear_is_idempotent`、`ipc_key_is_absent_from_responses_and_logs`（序列化 6 组 DTO 断言不含测试 Key 子串；`include_str!` 断言非测试段 0 命中 `println!`/`eprintln!`/`dbg!`）、`settings_api_key_is_masked_without_payload`（1/16/64/256 长度输出同为 `"masked"`）、`existing_closed_faces_are_unchanged`、`platform_scope_is_windows_only`。
- **文件**：`settings.rs` 的 `#[cfg(test)]`。
- **禁止**：测试替身不得写入真实凭据库；测试 Key 统一 `test-key-` 前缀；`Command::new("git")` 只允许出现在 `#[cfg(test)]`。
- **依赖**：W-UI-06.1~6.3、W-UI-04.2（`ProbeReportDto` 已在 DTO 层定义，无需等待 W-UI-07）。
- **证据**：
```bash
cargo test -p meeting-copilot-desktop ipc_settings_snapshot_is_closed_and_keyless
cargo test -p meeting-copilot-desktop ipc_settings_snapshot_without_key_is_not_configured
cargo test -p meeting-copilot-desktop save_receipt_reports_memory_fallback_without_plaintext
cargo test -p meeting-copilot-desktop clear_requires_confirmation_and_deletes_target
cargo test -p meeting-copilot-desktop clear_is_idempotent
cargo test -p meeting-copilot-desktop ipc_key_is_absent_from_responses_and_logs
cargo test -p meeting-copilot-desktop settings_api_key_is_masked_without_payload
cargo test -p meeting-copilot-desktop existing_closed_faces_are_unchanged
cargo test -p meeting-copilot-desktop platform_scope_is_windows_only
```
- **DoD**：9 例全退 0；无 `#[allow]`/`#[expect]` 规避告警。

---

## 7. 实现连通性探针两段式（W-UI-07 · 批次 C3）

**AC 映射**：AC-B4-114、AC-B4-117。**依赖**：W-UI-04（DTO/状态）。

### 7.1 实现端口消费、8 类映射、冷却与单飞（相对剩余 + 绝对镜像）

- **做什么**：`app/src-tauri/src/ipc/probe.rs` 实现 `probe_settings_impl`：单飞（`state.probe_in_flight` `compare_exchange` + `ProbeGuard`）→ `SecretInput` 内联校验 → `next_probe_state(UserTest)` → `state.probe.execute(&ProbeCall)` → 映射 `ProbeFailure` 到 `ProbeReportDto{failure,message=humanize_probe_failure(...),remaining_ms}` → `next_probe_state(ProbeFailed/ProbeSucceeded)`。
  冷却口径（**X-UI-13**）：冷却期先检查内部镜像 `cooldown_until_ms > now_ms`，直接返回 `rate_limited` + `remaining_ms = cooldown_until_ms - now_ms`，不调用端口；探针回执只携带**相对** `remaining_ms`，`AppState.cooldown_until_ms` 由 `now_ms + remaining_ms` 推导并仅作内部镜像；**禁止**把绝对时间放入 `ProbeAttempt`/`ProbeReportDto`，**禁止**前端或命令层自建第二套计时器。
- **文件**：`app/src-tauri/src/ipc/probe.rs`（新增）。
- **禁止**：不新增第 9 类失败；不自建计时器/重试/熔断；不在探针路径写审计/指标/日志；不打印 Key/端点；`ProbeAttempt` 不得出现 `cooldown_until_ms` 字段。
- **依赖**：W-UI-04.2/4.3。
- **证据**：
```bash
grep -c 'humanize_probe_failure' app/src-tauri/src/ipc/probe.rs   # >=1
grep -c 'next_probe_state' app/src-tauri/src/ipc/probe.rs         # >=2
grep -c 'compare_exchange' app/src-tauri/src/ipc/probe.rs         # >=1
grep -c 'remaining_ms' app/src-tauri/src/ipc/probe.rs             # >=1
grep -c 'cooldown_until_ms' app/src-tauri/src/ipc/probe.rs        # >=1（仅内部镜像，不进回执）
grep -cE 'cooldown_until_ms.*ProbeAttempt|ProbeAttempt.*cooldown_until_ms' app/src-tauri/src/ipc/probe.rs   # 0
```
- **DoD**：前四项各达标；`cooldown_until_ms` 仅作内部镜像（不回执）；映射表覆盖 8 类且无第 9 类。

### 7.2 实现 `llm-https` 生产适配（no-op 观察者）

- **做什么**：仅在 `#[cfg(feature = "llm-https")]` 下实现 `GatewayProbeExecutor`：由 `catalog.endpoints[0].url` 与 `DEEPSEEK_HOSTS[0]` 构造 `LlmConfig`（`request_timeout_ms` 取入参、`max_retries` 沿用默认、`thinking` 沿用既有口径），`assemble(Arc<RwLock<LlmConfig>>, Arc<dyn HttpTransport>, Arc<dyn LlmObserver>)` + `BearerTransport::new(Arc::new(TlsTransport::default()), SecretString::new(key))`；实现 `NoopObserver`（`record_metric`/`append_audit` 返回 `Ok(())`、`log` 空）。未启用 feature 时返回 `PortError::Unsupported`，映射为 `unknown` 固定文案。
- **文件**：`app/src-tauri/src/ipc/probe.rs`。
- **禁止**：不修改 `crates/infrastructure/src/llm/**`；不使用 `LocalObserver`；不回落明文 HTTP；不新增 registry 包；不出现连续 `https://` 字面量（**R-UI-06**）。
- **依赖**：W-UI-07.1、W-UI-04.4。
- **证据**：
```bash
grep -c 'assemble' app/src-tauri/src/ipc/probe.rs            # >=1
grep -c 'BearerTransport' app/src-tauri/src/ipc/probe.rs     # >=1
grep -c 'TlsTransport' app/src-tauri/src/ipc/probe.rs        # >=1
grep -c 'LocalObserver' app/src-tauri/src/ipc/probe.rs       # 0
git diff --quiet HEAD -- crates/infrastructure/src/llm; echo $?   # 0
```
- **DoD**：`assemble`/`BearerTransport`/`TlsTransport` 各 ≥1；`LocalObserver` = **0**；infrastructure 零 diff。

### 7.3 添加探针单测（6 例，含 X-UI-01 替代与 X-UI-13 推导断言）

- **做什么**：实现 `probe_executor_is_port_only`（`include_str!` 断言命令侧只经 `.execute(`）、`probe_adapter_reuses_existing_gateway`（断言 `assemble`/`BearerTransport`/`TlsTransport` 字样）、`ipc_probe_never_opens_socket`（断言非测试段 0 命中 `reqwest|hyper::|TcpStream|std::net`）、`ipc_probe_failure_maps_eight_kinds`（逐一构造 8 类，断言 `message` 恰为 8 条且互不相同）、`ipc_probe_failure_keeps_inputs`（失败后 `hasKey`/`modelChoice`/`endpointChoice` 不变）、`ipc_probe_cooling_reports_remaining_ms`（**X-UI-13**：注入 `now_ms = N`、内部镜像 `cooldown_until_ms = N + 3000` ⇒ 首次返回 `rate_limited` + `remaining_ms = 3000`；`now` 前进 1000 后返回 `remaining_ms = 2000`（单调不增且 >0）；替身端口调用计数 = **0**；两次调用不出现重复起算）。
- **文件**：`probe.rs` 的 `#[cfg(test)]`。
- **禁止**：不调用真实网络；不新增测试文件/target；不用 AC 文本中的错误用例名；测试不把绝对 `cooldown_until_ms` 断言到 `ProbeReportDto`。
- **依赖**：W-UI-07.1、W-UI-07.2。
- **证据**：
```bash
cargo test -p meeting-copilot-desktop probe_executor_is_port_only
cargo test -p meeting-copilot-desktop probe_adapter_reuses_existing_gateway
cargo test -p meeting-copilot-desktop ipc_probe_never_opens_socket
cargo test -p meeting-copilot-desktop ipc_probe_failure_maps_eight_kinds
cargo test -p meeting-copilot-desktop ipc_probe_failure_keeps_inputs
cargo test -p meeting-copilot-desktop ipc_probe_cooling_reports_remaining_ms
# X-UI-01：文案用例用实况名（AC 文本名不存在，勿写假用例）
cargo test -p meeting-copilot-ui-bridge --test llm_settings_view llm_settings_view_probe_events_and_messages
```
- **DoD**：7 条命令全退 0；`cooling` 用例断言"相对剩余 + 绝对镜像推导 + 端口 0 调用"三者同时成立；`X-UI-01` 已在 W-UI-16.1 登记；`X-UI-13` 已在本用例与 §0.6 登记。

---

## 8. 实现卡片快照与本地详情命令（W-UI-08 · 批次 C3）

**AC 映射**：AC-B4-121、AC-B4-123、AC-B4-124、AC-B4-127。**依赖**：W-UI-04。

### 8.1 实现快照/详情映射（手工拆载 + `withdrawn` 过滤 + 拒绝码）

- **做什么**：`app/src-tauri/src/ipc/cards.rs` 实现 `keyword_cards_snapshot_impl`（遍历 `CardSnapshotEntry`：Draft → `project_keyword_card_view`，Preview → `project_composed_card_view`，`view=None` 过滤）与 `keyword_card_open_impl`（按 `cardId` 定位→重新投影→`open_keyword_card_detail`；`detail` 映射 5 项 DTO；`reason` 映射 `IpcErrorDto.code`）。
  **载荷拆分按 design §2.3.3.1 手工完成**：在 `cards.rs` 内写纯映射函数（如 `fn split_click_state(&ClickState) -> (String, Option<String>)`），`match` 逐变体展开 7 个分支（`Enabled` + 6 个 `Disabled(reason)`），输出 `clickState="enabled"`+`disabledReason=null` 或 `clickState="disabled"`+`disabledReason=<6 类之一>`；`ComposedCard` 预览恒 `disabled`+`not_allowed`；`citation_alignment` 只写入 `CardDetailDto`。
- **文件**：`app/src-tauri/src/ipc/cards.rs`（新增）。
- **禁止**：`KeywordCardViewDto` **禁止持有 `ClickState`**（R-UI-10）；禁止 `#[serde(tag=...)]`/`#[serde(flatten)]`/`serde_json::Value` 透传（R-UI-09）；禁止把预览升级为可点；不出现 `issue_query`/`AuditLog`/`reqwest`/`TcpStream`/`card_store`/`compose_keyword_card`/`keyword_card_admission` 字面量（含注释）；不生成卡、不检索、不写存储；`retryable=false`。
- **依赖**：W-UI-04.2、W-UI-04.3。
- **证据**：
```bash
grep -rnE 'issue_query|AuditLog|reqwest|TcpStream|card_store|compose_keyword_card|keyword_card_admission' app/src-tauri/src/ipc/cards.rs  # 0
grep -c 'project_keyword_card_view' app/src-tauri/src/ipc/cards.rs        # >=1
grep -c 'project_composed_card_view' app/src-tauri/src/ipc/cards.rs       # >=1
grep -c 'open_keyword_card_detail' app/src-tauri/src/ipc/cards.rs         # >=1
grep -c 'ClickState' app/src-tauri/src/ipc/cards.rs                       # >=1（只作为映射入参）
grep -c 'ClickState' app/src-tauri/src/ipc/dto.rs                         # 0（DTO 不得持有）
grep -cE 'serde\(tag|serde\(flatten' app/src-tauri/src/ipc/cards.rs       # 0
field_count KeywordCardViewDto                                            # 10
field_count CardDetailDto                                                 # 5
dto_block KeywordCardViewDto | grep -c '^    pub '                        # 10（字面双重断言）
dto_block CardDetailDto      | grep -c '^    pub '                        # 5（字面双重断言）
dto_block KeywordCardViewDto | grep -cE 'normalizedTerm|gateDecisionRef|traceId|url|model|requestId|apiKey|ClickState'   # 0
dto_block KeywordCardViewDto | grep -c 'citation_alignment'               # 0
dto_block CardDetailDto      | grep -c 'citation_alignment'               # 1
```
- **DoD**：禁用词 = **0**；三个 51 函数各 ≥1；`cards.rs` 有 `ClickState` 映射入参、`dto.rs` = **0**；`serde(tag/flatten` = **0**；**双重断言**：`field_count KeywordCardViewDto`=**10** 且该块禁词 = **0**；`CardDetailDto`=**5**；`citation_alignment` 仅在详情块 = **1**。

### 8.2 添加卡片命令单测（5 例）

- **做什么**：实现 `ipc_card_snapshot_is_closed_and_read_only`（序列化后 10 字段、无禁止字段、`clickState` 为扁平字符串且 `disabledReason` 可空、`withdrawn` 不出现）、`ipc_card_detail_is_closed`（5 字段）、`ipc_card_open_has_zero_side_effects`（替身计数 `outbound=0`/`llm=0`/`audit=0`/`store=0`）、`ipc_card_snapshot_hides_body_under_screen_share`（`bodyVisible=false`、`body=null`、原因 `screen_share_protection`）、`card_snapshot_does_not_revive_withdrawn`（同一快照重复读取仍不出现）。
- **文件**：`cards.rs` 的 `#[cfg(test)]`。
- **禁止**：不使用真实投屏 API；不新增测试文件/target；断言中不得出现 `{"disabled":` 形式的标签化输出。
- **依赖**：W-UI-08.1。
- **证据**：
```bash
cargo test -p meeting-copilot-desktop ipc_card_snapshot_is_closed_and_read_only
cargo test -p meeting-copilot-desktop ipc_card_detail_is_closed
cargo test -p meeting-copilot-desktop ipc_card_open_has_zero_side_effects
cargo test -p meeting-copilot-desktop ipc_card_snapshot_hides_body_under_screen_share
cargo test -p meeting-copilot-desktop card_snapshot_does_not_revive_withdrawn
cargo test -p meeting-copilot-ui-bridge --test keyword_card_view keyword_card_view_failed_alignment_stays_visible_but_disabled
cargo test -p meeting-copilot-ui-bridge --test keyword_card_view keyword_card_view_detail_has_no_outbound_side_effect
```
- **DoD**：7 条命令全退 0；快照用例断言 `clickState` 为 `enabled`/`disabled` 字符串、`disabledReason` 6 类可空、且序列化文本不含 `"disabled":`。

---

## 9. 注册 6 个 IPC 命令并装配运行态（W-UI-09 · 批次 C4）

**AC 映射**：AC-B4-120、AC-B4-126。**依赖**：W-UI-06、W-UI-07、W-UI-08。

### 9.1 定义 6 个命令薄注册

- **做什么**：`app/src-tauri/src/ipc/mod.rs` 声明 `pub mod` 子模块并定义**恰 6 个** `#[tauri::command] fn`：`settings_snapshot`、`settings_probe`、`settings_save`、`settings_clear`、`keyword_cards_snapshot`、`keyword_card_open`，各自只做参数解包 + 调用 `*_impl` + 计数累加。
- **文件**：`app/src-tauri/src/ipc/mod.rs`（新增）。
- **禁止**：不定义第 7 个以 `settings_*`/`keyword_*` 开头的 `fn`；不在命令层写业务判定。
- **依赖**：W-UI-06~08。
- **证据**：
```bash
grep -cE 'fn (settings_snapshot|settings_probe|settings_save|settings_clear|keyword_cards_snapshot|keyword_card_open)' app/src-tauri/src/ipc/mod.rs  # 6
```
- **DoD**：计数 = **6**。

### 9.2 装配 `AppState` 并注册 handler

- **做什么**：`app/src-tauri/src/main.rs` 增 `mod ipc;`、`fn build_state()`（构造 `WindowsCredentialStore`、`FileSettingsConfigStore`、探针执行器、`AppFactsSource`（与 `AppState` 共享 `Arc<Mutex<Option<String>>>`/`Arc<Mutex<Option<u64>>>`/`Arc<AtomicBool>`）、空卡片源、默认视图事实源、`SettingsCatalog::from_injected`）、`manage(state)`；`generate_handler![ipc_version, 6 个命令]`；`LLM_MODEL` 只在此处 `std::env::var` 读取一次。
- **文件**：`app/src-tauri/src/main.rs`。
- **禁止**：`std::env`/`LLM_` 只允许出现在 `main.rs`；`ipc/**` 零命中（W-UI-15 断言）；启动日志不得输出模型名/端点。
- **依赖**：W-UI-09.1、W-UI-04.3/4.4。
- **证据**：
```bash
grep -c 'mod ipc;' app/src-tauri/src/main.rs                     # 1
grep -c 'manage(' app/src-tauri/src/main.rs                      # >=1
grep -c 'ipc_version' app/src-tauri/src/main.rs                  # >=1
grep -c 'std::env' app/src-tauri/src/main.rs                     # 1
grep -rc 'std::env' app/src-tauri/src/ipc | grep -v ':0$' | wc -l # 0
cargo build -p meeting-copilot-desktop
```
- **DoD**：模块/`manage`/`ipc_version` 各达标；`main.rs` 的 `std::env` = **1**；`ipc/**` 命中文件数 = **0**；`cargo build` 退 0。

### 9.3 启用 `withGlobalTauri`

- **做什么**：`app/src-tauri/tauri.conf.json` 的 `app` 对象增 `"withGlobalTauri": true`；CSP 与 capabilities 保持零 diff。
- **文件**：`app/src-tauri/tauri.conf.json`。
- **禁止**：不新增 `connect-src`；不改窗口字段；不新增插件权限。
- **依赖**：W-UI-09.2。
- **证据**：
```bash
grep -c 'withGlobalTauri' app/src-tauri/tauri.conf.json        # 1
grep -c 'connect-src' app/src-tauri/tauri.conf.json            # 0
grep -cE 'fs:|http:|shell:|sql:' app/src-tauri/capabilities/default.json  # 0
```
- **DoD**：`withGlobalTauri` = **1**；`connect-src` = **0**；能力禁词 = **0**。

---

## 10. 建立前端类型桥与 6 命令白名单（W-UI-10 · 批次 C5）

**AC 映射**：AC-B4-126、AC-B4-127。**依赖**：W-UI-09。

### 10.1 扩展 DTO 接口与命令白名单

- **做什么**：`app/src/bridge/types.ts` 增 7 个响应 DTO interface（`SettingsSnapshotDto`/`ProbeReportDto`/`SettingsSaveReceiptDto`/`SettingsClearReceiptDto`/`KeywordCardViewDto`/`CardBodyDto`/`CardDetailDto`）+ 4 个输入 interface + `ModelChoiceDto`/`EndpointChoiceDto`/`ConclusionDto` 联合类型；定义 `export const IPC_COMMANDS = [...] as const`（恰 6 项）与 `IpcCommand` 类型。
- **文件**：`app/src/bridge/types.ts`。
- **禁止**：可空字段统一 `T | null`（不用 `?`）；不用 `any`；不改 `IPC_PROTOCOL_VERSION=1`；不引入 `@tauri-apps/api`。
- **依赖**：W-UI-09.1（命令名定稿）。
- **证据**：
```bash
grep -c 'IPC_COMMANDS' app/src/bridge/types.ts                     # >=1
grep -c 'export interface' app/src/bridge/types.ts                 # >=11（4 旧 + 7 新）
grep -cE ': any|as any' app/src/bridge/types.ts                    # 0
```
- **DoD**：`IPC_COMMANDS` ≥1；interface ≥11；`any` = **0**。

### 10.2 同步 `schema.json`

- **做什么**：`app/src/bridge/schema.json` 增 7 个 DTO 的字段集（与 `types.ts` 逐字一致），保留既有 4 个。
- **文件**：`app/src/bridge/schema.json`。
- **禁止**：不删除既有 4 个 DTO；字段名不得 camelCase 漂移。
- **依赖**：W-UI-10.1。
- **证据**：
```bash
node scripts/check-bindings.mjs
```
- **DoD**：输出 `绑定契约一致：11 个 DTO` 且退 0。

### 10.3 实现 `invokeCommand` 与全局对象类型

- **做什么**：`app/src/bridge/tauri.ts` 手写 `window.__TAURI__?.core.invoke<T>` 声明与 `invokeCommand<T>(command: IpcCommand, args?)`；`vite-env.d.ts` 增全局声明；失败统一抛出 `IpcErrorDto`（不打印）。
- **文件**：`app/src/bridge/tauri.ts`（新增）、`app/src/vite-env.d.ts`。
- **禁止**：不引入 npm 依赖；不出现 `console.log/debug/info`；不发起任何网络请求。
- **依赖**：W-UI-10.1。
- **证据**：
```bash
grep -c '__TAURI__' app/src/bridge/tauri.ts        # >=1
grep -c 'invokeCommand' app/src/bridge/tauri.ts    # >=1
grep -rnE 'console\.(log|debug|info)' app/src     # 0
```
- **DoD**：两项各 ≥1；console 命中 = **0**。

### 10.4 通过绑定与类型检查

- **做什么**：执行绑定校验与 TS 类型检查，修正可空类型/联合类型问题。
- **文件**：无新增；仅修复。
- **禁止**：不用 `@ts-ignore`/`eslint-disable` 绕过；不改 `tsconfig` 严格项。
- **依赖**：W-UI-10.1~10.3。
- **证据**：
```bash
node scripts/check-bindings.mjs
npm --prefix app run typecheck
```
- **DoD**：两条命令均退 0。

---

## 11. 渲染设置向导页（W-UI-11 · 批次 C5）

**AC 映射**：AC-B4-116。**依赖**：W-UI-10。

### 11.1 实现四段顺序与首屏折叠

- **做什么**：`app/src/settings/SettingsPage.tsx` 按 `snapshot.sectionsOrder` 渲染四段（粘贴 Key → 选择模型 → 服务地址（高级，`advancedCollapsed` 控制）→ 测试并保存）；默认路径动作数 ≤3（粘贴 → 测试 → 保存/自动保存）。
- **文件**：`app/src/settings/SettingsPage.tsx`（新增）。
- **禁止**：不硬编码段顺序（必须读 `sectionsOrder`）；不要求首屏填写地址；不写本地展开状态到后端（`advancedCollapsed` 恒取快照）。
- **依赖**：W-UI-10.1、W-UI-10.3。
- **证据**：
```bash
grep -c 'sectionsOrder' app/src/settings/SettingsPage.tsx        # >=1
grep -c 'advancedCollapsed' app/src/settings/SettingsPage.tsx    # >=1
bash scripts/check-ui-integration.sh --rule=settings-ux          # 0（W-UI-14 完成后）
```
- **DoD**：两项各 ≥1；规则退 0。

### 11.2 按钮只取后端布尔、内联错误只消费后端枚举

- **做什么**：测试/保存按钮 `disabled={!snapshot.canTest}` / `disabled={!snapshot.canSave}`；`inlineErrors` 映射为 6 条固定中文文案（文件内常量表），不自行推导模型/端点有效性。
- **文件**：`app/src/settings/SettingsPage.tsx`。
- **禁止**：不出现 `modelIndex ===`/`endpointIndex !==` 之类前端判定（AC-116 grep = 0）；不自行拼接错误文案。
- **依赖**：W-UI-11.1。
- **证据**：
```bash
grep -c 'canTest' app/src/settings/SettingsPage.tsx   # >=1
grep -c 'canSave' app/src/settings/SettingsPage.tsx   # >=1
grep -cE '(modelIndex|endpointIndex)[[:space:]]*(===|!==)' app/src/settings/*.tsx  # 0
```
- **DoD**：`canTest`/`canSave` 各 ≥1；下标比较 = **0**。

### 11.3 实现测试/保存/清除与降级提示

- **做什么**：调用 `probeSettings`/`saveSettings`/`clearSettings`；提交成功后清空 Key 输入框，失败保留；`receipt.fallback === "memory"` 时展示"仅本次运行有效"（用 `notice`）；清除前二次确认；`probeReport.failure` 用后端 `message` 展示并可重试；冷却期显示剩余秒并禁用。
- **文件**：`app/src/settings/SettingsPage.tsx`。
- **禁止**：不本地自建倒计时；不持久化 Key（无 `localStorage`/`sessionStorage`/`indexedDB`）；不展示 Key 任何片段/长度。
- **依赖**：W-UI-11.2。
- **证据**：
```bash
grep -c 'probeSettings' app/src/settings/SettingsPage.tsx    # >=1
grep -c 'saveSettings' app/src/settings/SettingsPage.tsx     # >=1
grep -c 'clearSettings' app/src/settings/SettingsPage.tsx    # >=1
grep -rnE 'localStorage|sessionStorage|indexedDB' app/src     # 0
```
- **DoD**：三个调用各 ≥1；本地存储 = **0**。

### 11.4 补充设置分区样式

- **做什么**：`app/src/styles.css` 新增设置分区/内联错误/高级折叠类；不改既有类语义。
- **文件**：`app/src/styles.css`。
- **禁止**：不改 `.card`/`.channel` 等既有类的既有声明语义；不引入外部字体/远程资源。
- **依赖**：W-UI-11.1。
- **证据**：
```bash
grep -c 'settings-section' app/src/styles.css   # >=3
npm --prefix app run typecheck                  # 0
```
- **DoD**：新增类 ≥3；`typecheck` 退 0。

---

## 12. 渲染关键词卡片与内联详情（W-UI-12 · 批次 C5）

**AC 映射**：AC-B4-122、AC-B4-123、AC-B4-125。**依赖**：W-UI-10。

### 12.1 实现 `KeywordCard`（可点条件、禁用语义、隐藏分支）

- **做什么**：`app/src/cards/KeywordCard.tsx` 以只读 `KeywordCardViewDto` 渲染：可点条件恒为 `clickState==="enabled" && bodyVisible===true`，写法固定为 `onClick={enabled ? handleOpen : undefined}`；其余 `aria-disabled="true"` 且不绑定回调；禁用/隐藏卡展示标题与禁用原因（`aria-describedby` 指向原因文本）；隐藏分支用 `BODY-HIDDEN-BEGIN`/`BODY-HIDDEN-END` 标记块包裹，块内只引用 `badgeCount`。
- **文件**：`app/src/cards/KeywordCard.tsx`（新增）。
- **禁止**：不补 `clickState`/`disabledReason`/`body`；隐藏块内不出现 `title`/`conclusion`/`referenceIds`；不用 `role="button"` + `tabIndex` 模拟禁用；不本地缓存正文。
- **依赖**：W-UI-10.1、W-UI-10.3。
- **证据**：
```bash
grep -c 'aria-disabled' app/src/cards/KeywordCard.tsx       # >=1
grep -c 'onClick={enabled' app/src/cards/KeywordCard.tsx    # >=1
grep -c 'BODY-HIDDEN-BEGIN' app/src/cards/KeywordCard.tsx   # 1
grep -c 'badgeCount' app/src/cards/KeywordCard.tsx          # >=1
bash scripts/check-ui-integration.sh --rule=card-render     # 0（W-UI-14 完成后）
bash scripts/check-ui-integration.sh --rule=body-hidden     # 0（W-UI-14 完成后）
```
- **DoD**：四项各达标；两条规则退 0。

### 12.2 实现 `CardDetail`（内联非模态、不抢焦点）

- **做什么**：`app/src/cards/CardDetail.tsx` 以 `<section role="region" aria-label="卡片详情">` 渲染 5 项 `CardDetailDto`（标题/结论态/引用 ID 列表/对齐状态），关闭按钮只做状态切换。
- **文件**：`app/src/cards/CardDetail.tsx`（新增）。
- **禁止**：不出现 `role="dialog"`/`showModal`/`autoFocus`/`.focus(`/`scrollIntoView`；不改变列表焦点。
- **依赖**：W-UI-12.1。
- **证据**：
```bash
grep -c 'role="dialog"' app/src/cards/CardDetail.tsx   # 0
grep -rnE 'autoFocus|\.focus\(|scrollIntoView|showModal' app/src   # 0
bash scripts/check-ui-integration.sh --rule=no-focus-steal         # 0（W-UI-14 完成后）
```
- **DoD**：`dialog` = **0**；焦点禁词命中 = **0**；规则退 0。

### 12.3 固化六类禁用原因常量与卡片样式

- **做什么**：`app/src/cards/KeywordCard.tsx` 内联 6 条禁用原因中文常量（`stale_reference`/`authorization_revoked`/`not_allowed`/`emergency_masked`/`screen_share_protection`/`reference_unverified`）；`styles.css` 新增卡片角标/禁用原因/详情面板类。
- **文件**：`app/src/cards/KeywordCard.tsx`、`app/src/styles.css`。
- **禁止**：不新增常量文件（设计 §2.1.3.4-8）；键集合不得与后端枚举漂移；不改既有类语义。
- **依赖**：W-UI-12.1。
- **证据**：
```bash
grep -c 'screen_share_protection' app/src/cards/KeywordCard.tsx     # >=1
grep -c 'reference_unverified' app/src/cards/KeywordCard.tsx        # >=1
grep -c 'card-badge' app/src/styles.css                             # >=1
```
- **DoD**：两枚举键各 ≥1；样式类 ≥1；6 键与 `ClickDisabledReason::ALL` 一致（W-UI-14 `card-render` 复验）。

---

## 13. 以快照驱动主界面接线（W-UI-13 · 批次 C5）

**AC 映射**：AC-B4-127。**依赖**：W-UI-11、W-UI-12。

### 13.1 删除静态卡片、改为快照驱动

- **做什么**：`app/src/App.tsx` 删除 `SEED_CARDS` 与 `InsightCard` 类型；挂载后 `loadKeywordCards()` 获取 `KeywordCardViewDto[]`，渲染列表（`role="list"`），空数组显示"暂无提示卡"；刷新/重新进入只依赖新快照。
- **文件**：`app/src/App.tsx`。
- **禁止**：不保留静态卡片兜底；不轮询；不缓存正文；不复活已撤回卡。
- **依赖**：W-UI-12.1、W-UI-12.2。
- **证据**：
```bash
grep -c 'SEED_CARDS' app/src/App.tsx          # 0
grep -c 'loadKeywordCards' app/src/App.tsx    # >=1
grep -c '暂无提示卡' app/src/App.tsx          # >=1
```
- **DoD**：`SEED_CARDS` = **0**；加载与空态各 ≥1。

### 13.2 接入设置页入口并保留静态只读区

- **做什么**：`App.tsx` 增设置页入口（按钮/面板切换）并渲染 `SettingsPage`；"采集状态/时间轴"保持静态只读，不新增数据源、不新增 IPC。
- **文件**：`app/src/App.tsx`。
- **禁止**：不为采集状态新增命令/数据源；不改 `CHANNELS` 语义。
- **依赖**：W-UI-11.1。
- **证据**：
```bash
grep -c 'SettingsPage' app/src/App.tsx        # >=1
grep -c 'CHANNELS' app/src/App.tsx            # >=1
grep -cE 'invokeCommand|__TAURI__' app/src/App.tsx | head -1   # 允许，但只经 bridge 封装
```
- **DoD**：`SettingsPage` ≥1；`CHANNELS` 保留；`App.tsx` 内不直接访问 `__TAURI__`（只经 `bridge/`）。

### 13.3 补充主界面样式并通过类型检查

- **做什么**：`styles.css` 新增设置入口切换/卡片列表空态类；运行 `typecheck` 与 `build`。
- **文件**：`app/src/styles.css`。
- **禁止**：不引入远程字体/图片；不改 CSP。
- **依赖**：W-UI-13.1、W-UI-13.2。
- **证据**：
```bash
npm --prefix app run typecheck
npm --prefix app run build
grep -rnE 'localStorage|sessionStorage|indexedDB|console\.(log|debug|info)' app/src   # 0
```
- **DoD**：`typecheck`/`build` 均退 0；本地存储与 console 命中 = **0**。

---

## 14. 新增六规则静态门禁（W-UI-14 · 批次 C6，必须晚于 C1~C5）

**AC 映射**：AC-B4-130（另为 AC-115/116/118/122/123/125/126/127 提供子断言）。**依赖**：W-UI-01~W-UI-13 全部落盘。

### 14.1 建立脚本骨架与退出码契约

- **做什么**：新建 `scripts/check-ui-integration.sh`：`--rule=<name>` 单规则、`--all` 聚合、`--self-test`；退出码 `0`=通过 / `1`=命中 / `2`=扫描面缺失或参数非法；扫描面清单（`app/src/bridge/types.ts`、`app/src/settings/SettingsPage.tsx`、`app/src/cards/{KeywordCard,CardDetail}.tsx`、`app/src-tauri/tauri.conf.json`、`app/src-tauri/capabilities/default.json`、`app/src-tauri/src/ipc/dto.rs`）任一缺失即退 2。
- **文件**：`scripts/check-ui-integration.sh`（新增）。
- **禁止**：不依赖 cargo、不访问网络；规则固定 6 条，不增删；不接入 `ci.yml`。
- **依赖**：W-UI-13（扫描面全部就绪）。
- **证据**：
```bash
bash scripts/check-ui-integration.sh --rule=unknown; echo $?   # 2
bash scripts/check-ui-integration.sh --rule=credential-redline; echo $?  # 0
grep -cE 'credential-redline|frontend-offline|no-focus-steal|card-render|body-hidden|settings-ux' scripts/check-ui-integration.sh  # >=6
```
- **DoD**：未知规则退 **2**；规则名集合 = **6**。

### 14.2 实现 `credential-redline`

- **做什么**：扫描 `app/src/**` 与 `app/src-tauri/src/**`，0 命中 `localStorage`/`sessionStorage`/`indexedDB`/`console.log|debug|info`/`println!`/`eprintln!`/`dbg!`；6 个响应 DTO 定义块 0 命中精确禁词（含 `modelRaw`，**不含裸 `model`**）；调用 `scan-secrets.sh` 与 `scan-secrets-docs.sh --all-md` 并要求均退 0；输出 `PASS: frontend_persists_neither_key_nor_card_text` / `PASS: settings_api_key_is_masked_without_payload`。
- **文件**：`scripts/check-ui-integration.sh`。
- **禁止**：不使用裸 `model` 作为禁词（避免误伤 `modelChoice`）；不以 `|| true` 吞掉子脚本失败。
- **依赖**：W-UI-14.1。
- **证据**：
```bash
bash scripts/check-ui-integration.sh --rule=credential-redline   # 0
bash scripts/check-ui-integration.sh --rule=credential-redline | grep -c '^PASS: '   # >=2
```
- **DoD**：退 0；`PASS:` 行 ≥2。

### 14.3 实现 `frontend-offline`

- **做什么**：扫描 `app/src/**` 0 命中 `fetch(`/`axios`/`XMLHttpRequest`/`WebSocket`/`EventSource`/`sendBeacon`/`@tauri-apps/api`；`tauri.conf.json` 无 `connect-src`；`capabilities/default.json` 仅 `core:default`；`IPC_COMMANDS` 恰 6 项且与 6 命令逐字一致；输出 `PASS: frontend_is_offline_and_minimal_privilege`。
- **文件**：`scripts/check-ui-integration.sh`。
- **禁止**：不把 `ipc_version` 计入 6 项；不把 `.md` 纳入扫描。
- **依赖**：W-UI-14.1、W-UI-10.1。
- **证据**：
```bash
bash scripts/check-ui-integration.sh --rule=frontend-offline   # 0
```
- **DoD**：退 0；命令集与 `types.ts` 完全一致。

### 14.4 实现 `no-focus-steal`

- **做什么**：扫描 `app/src/**` 0 命中 `autoFocus`/`.focus(`/`scrollIntoView`/`showModal`；`CardDetail.tsx` 0 命中 `role="dialog"`；输出 `PASS: card_detail_does_not_steal_focus`。
- **文件**：`scripts/check-ui-integration.sh`。
- **禁止**：不把注释中的历史字样排除得过宽（保持逐行判定）。
- **依赖**：W-UI-14.1、W-UI-12.2。
- **证据**：
```bash
bash scripts/check-ui-integration.sh --rule=no-focus-steal   # 0
```
- **DoD**：退 0。

### 14.5 实现 `card-render`

- **做什么**：断言 `KeywordCard.tsx` 含 `aria-disabled`、`onClick={enabled`、`bodyVisible`、`badgeCount`；禁用分支不绑定 `onClick`；预览/禁用卡仍渲染标题与禁用原因；6 类禁用原因键集合与后端 `ClickDisabledReason::ALL` 一致；输出 `PASS: card_component_binds_click_only_when_enabled` / `PASS: card_component_marks_disabled_semantics`。
- **文件**：`scripts/check-ui-integration.sh`。
- **禁止**：不以"文件存在"代替结构断言。
- **依赖**：W-UI-14.1、W-UI-12.1、W-UI-12.3。
- **证据**：
```bash
bash scripts/check-ui-integration.sh --rule=card-render   # 0
bash scripts/check-ui-integration.sh --rule=card-render | grep -c '^PASS: '   # >=2
```
- **DoD**：退 0；`PASS:` 行 ≥2。

### 14.6 实现 `body-hidden`

- **做什么**：提取 `BODY-HIDDEN-BEGIN`/`BODY-HIDDEN-END` 标记块，断言块内只引用 `badgeCount`，0 命中 `title`/`conclusion`/`referenceIds`；输出 `PASS: card_component_hides_body_when_not_visible`。
- **文件**：`scripts/check-ui-integration.sh`。
- **禁止**：标记块缺失时退 2（不得当作 0）。
- **依赖**：W-UI-14.1、W-UI-12.1。
- **证据**：
```bash
bash scripts/check-ui-integration.sh --rule=body-hidden   # 0
```
- **DoD**：退 0；标记缺失时手工验证退 2。

### 14.7 实现 `settings-ux` 与 `--all` 聚合、自检

- **做什么**：断言 `SettingsPage.tsx` 含 `sectionsOrder`/`advancedCollapsed`/`canTest`/`canSave`、0 命中 `(modelIndex|endpointIndex)===|!==`、四段顺序与 `SECTION_ORDER` 一致；`--all` 依次执行六规则，任一退 1/2 即总退非 0，并汇总 `PASS/FAIL` 行；`--self-test` 用临时文件构造正/负样本，验证规则可命中且工作区不脏。
- **文件**：`scripts/check-ui-integration.sh`。
- **禁止**：`--all` 不得短路后仍返回 0；自检不得修改工作区。
- **依赖**：W-UI-14.2~14.6。
- **证据**：
```bash
bash scripts/check-ui-integration.sh --rule=settings-ux   # 0
bash scripts/check-ui-integration.sh --all                # 0
bash scripts/check-ui-integration.sh --all | grep -c '^PASS: '   # >=8
bash scripts/check-ui-integration.sh --self-test
```
- **DoD**：三条主命令退 0；`PASS:` 行 ≥8；自检输出检出数 ≥1。

---

## 15. 扩展 B4 门禁扫描面（W-UI-15 · 批次 C6，必须晚于 C1~C5）

**AC 映射**：AC-B4-128、AC-B4-130。**依赖**：W-UI-02、W-UI-03、W-UI-04 落盘（否则退 2）。

### 15.1 增变量与必需扫描面

- **做什么**：`scripts/check-b4-gates.sh` 变量区增 `APP_SETTINGS_DIR="crates/application/src/settings"`、`PLATFORM_SECURE_STORE="crates/platform-win/src/secure_store.rs"`、`IPC_DIR="app/src-tauri/src/ipc"`；`require_core_faces()` 追加 `need_dir`/`need_file`/`need_dir` 三项。
- **文件**：`scripts/check-b4-gates.sh`。
- **禁止**：不改 `--rule` 名称集合；不改退出码语义；不删既有扫描面。
- **依赖**：W-UI-15 前置（文件已存在）、W-UI-03、W-UI-04。
- **证据**：
```bash
grep -c 'APP_SETTINGS_DIR=' scripts/check-b4-gates.sh            # 1
grep -c 'PLATFORM_SECURE_STORE=' scripts/check-b4-gates.sh       # 1
grep -c 'IPC_DIR=' scripts/check-b4-gates.sh                     # 1
bash scripts/check-b4-gates.sh --rule=source; echo $?            # 0
```
- **DoD**：三变量各 = 1；`source` 退 0。

### 15.2 扩 `core_files()` 与新增 `ipc_files()`

- **做什么**：`core_files()` 追加 `application/settings/*.rs` 与 `platform-win/src/secure_store.rs`（严格模式）；新增 `ipc_files()` 枚举 `app/src-tauri/src/ipc/*.rs`（受限模式用）。
- **文件**：`scripts/check-b4-gates.sh`。
- **禁止**：`ipc_files()` 不得包含 `main.rs`（`std::env` 读取点）。
- **依赖**：W-UI-15.1。
- **证据**：
```bash
grep -c 'ipc_files()' scripts/check-b4-gates.sh    # >=2（定义 + 使用）
bash scripts/check-b4-gates.sh --rule=budget; echo $?   # 0
```
- **DoD**：`ipc_files()` 出现 ≥2；`budget` 退 0。

### 15.3 实现 `--rule=source` 双模式

- **做什么**：对 `core_files()` 保持原严格禁词（含 `std::fs`/`std::env`/`Instant::now`/`SystemTime::now`/网络/全局态/占位 panic）；对 `ipc_files()` 使用装配层受限禁词：`reqwest::|hyper::|TcpStream|std::net|std::env|LLM_|todo!\(|unimplemented!\(|panic!\(|static mut|OnceLock|lazy_static|thread_local`（允许 IO 与时钟）。
- **文件**：`scripts/check-b4-gates.sh`。
- **禁止**：不把两模式混成一套更宽的禁词；不新增第 8 条规则。
- **依赖**：W-UI-15.2。
- **证据**：
```bash
bash scripts/check-b4-gates.sh --rule=source; echo $?      # 0
grep -c 'ipc_files' scripts/check-b4-gates.sh              # >=2
grep -cE 'rule_source|rule_zero_transcript' scripts/check-b4-gates.sh   # >=2
```
- **DoD**：`source` 退 0；双模式在两处规则均生效。

### 15.4 扩 `--rule=zero-transcript` 扫描面

- **做什么**：`rule_zero_transcript` 追加 `APP_SETTINGS_DIR`、`PLATFORM_SECURE_STORE`、`ipc_files()`；保持模式不变（URL/Key 形态/鉴权头/模型名/`request_id`）。
- **文件**：`scripts/check-b4-gates.sh`。
- **禁止**：不放宽模式；不把测试块排除（`strip_tests` 既有语义保留）。
- **依赖**：W-UI-15.2。
- **证据**：
```bash
bash scripts/check-b4-gates.sh --rule=zero-transcript; echo $?   # 0
```
- **DoD**：退 0（`catalog.rs` 无连续 `https://` 由 **R-UI-06** 保证）。

### 15.5 取证规则数与 targets 不变

- **做什么**：确认 `--rule=targets` 循环仍 7 项、不追加 `keyword_card_view`/`llm_settings_view`/`gate_replay_driver`；七条规则全 0；`check-b4-gates.sh --self-test` 仍检出。
- **文件**：无新增（校验）。
- **禁止**：不改 `scripts/test-name-inventory.sh`；不改 `NEW_TARGETS`。
- **依赖**：W-UI-15.1~15.4。
- **证据**：
```bash
for r in source budget record outbound zero-transcript targets doc-plan; do bash scripts/check-b4-gates.sh --rule=$r; echo "$r=$?"; done
grep -c 'keyword_card_view' scripts/check-b4-gates.sh   # 0
new_targets                                             # 25
```
- **DoD**：7 条规则均 `=0`；`keyword_card_view` 在门禁中命中 = **0**；`new_targets` = **25**。

---

## 16. 登记实现快照与口径文档（W-UI-16 · 批次 C7）

**AC 映射**：AC-B4-120、AC-B4-130（另承接 X-UI-01/X-UI-11 登记）。**依赖**：W-UI-14、W-UI-15。

### 16.1 追加 `b4-status.md` §28.8 与 §29（含 X-UI-01/11/12/13 登记）

- **做什么**：在 `docs/progress/b4-status.md` 追加 `### §28.8 实现快照（2026-11-23）` 与 `## §29 U-05 产品化接入实现收口`：命令结果表（fmt/clippy/test/六规则/7 规则/deny/typecheck/bindings）、AC-111~130 三态表、依赖面（**`windows-sys` 0.45.0** 与 `X-UI-11`）、**`X-UI-12` 的 C6 后置事实**、**`X-UI-13` 的 `remainingMs`/`cooldown_until_ms` 推导口径**、零 diff 证据、`X-UI-01` 替代用例登记、U-05-1/U-05-2 离线判据本地 passed 与 **U-05 整体 `open`**、U-07/U-08 仍 open、不宣称清单。
- **文件**：`docs/progress/b4-status.md`。
- **禁止**：不新增第二行 `^## §28`（AC-130 要求 =1）；不重排 §1~§28；不写"设置向导已可用"/"真机通过"/"真实联网 E2E 已过"/"U-05 closed"；不写 UUID/URL/Key 形态（`scan-secrets-docs.sh --all-md`）。
- **依赖**：W-UI-14.7、W-UI-15.5。
- **证据**：
```bash
grep -c '^## §28' docs/progress/b4-status.md          # 1
grep -c '^## §29' docs/progress/b4-status.md          # 1
grep -c 'U-05.*产品化' docs/progress/b4-status.md     # >=1
grep -c 'D-B4-P144' docs/progress/b4-status.md        # >=1
grep -c 'D-B4-P170' docs/progress/b4-status.md        # >=1
grep -c 'windows-sys.*0.45.0' docs/progress/b4-status.md   # >=1（X-UI-11）
grep -c 'X-UI-12' docs/progress/b4-status.md          # >=1（C6 后置）
grep -c 'X-UI-13' docs/progress/b4-status.md          # >=1（remainingMs 口径）
grep -c 'llm_settings_view_probe_events_and_messages' docs/progress/b4-status.md   # >=1（X-UI-01 替代）
grep -c '设置向导已可用' docs/progress/b4-status.md   # 0
bash scripts/scan-secrets-docs.sh --all-md            # 0
```
- **DoD**：§28 = **1**、§29 = **1**；U-05/D-B4-P144/D-B4-P170 各 ≥1；`windows-sys 0.45.0`/`X-UI-12`/`X-UI-13` 各 ≥1；X-UI-01 替代用例 ≥1；禁用宣称 = **0**；`U-05` 未被写成 `closed`；文档密钥扫描退 0。

### 16.2 改写 README 五处口径

- **做什么**：`README.md` 改写"当前进度/LLM 配置入口/关键词卡片/隐私与凭据/当前阶段"，口径为"接入实现已落地并离线验收，真机与真实联网未验收"。
- **文件**：`README.md`。
- **禁止**：不写"设置向导已可用"/"产品级卡片已交付"/"macOS 支持"；不改既有相对链接目标；`git diff --numstat` 控制在 ≤20 行。
- **依赖**：W-UI-16.1。
- **证据**：
```bash
grep -c '设置向导已可用' README.md                    # 0
grep -c '真机通过' README.md                          # 0
grep -c '产品级卡片已交付' README.md                  # 0
git diff --numstat -- README.md
```
- **DoD**：三项宣称 = **0**；diff 行数 ≤20；链接缺失 = 0。

---

## 17. 取证封闭面零 diff 与计数（W-UI-17 · 批次 C7）

**AC 映射**：AC-B4-128、AC-B4-129、AC-B4-120。**依赖**：W-UI-16。

### 17.1 零 diff 面逐项取证

- **做什么**：对 51、门控、卡片合成、domain、infrastructure、mac、CI、deny、能力矩阵、旧 DTO 逐一执行 `git diff --quiet HEAD -- <path>`。
- **文件**：无改动；证据写入 `.tmp/ui-integration/zero-diff.txt`。
- **禁止**：不使用 `git checkout`/`restore` 掩盖差异；发现差异即修复实现而不是改断言语义。
- **依赖**：W-UI-03、W-UI-09、W-UI-15。
- **证据**：
```bash
for p in crates/ui-bridge/src/settings crates/ui-bridge/src/keyword_card crates/ui-bridge/Cargo.toml crates/application/src/gate crates/application/src/cards crates/domain/src/model crates/infrastructure/src/llm crates/platform-mac .github/workflows/ci.yml deny.toml crates/platform-win/resources/capability_matrix.toml crates/ui-bridge/src/dto.rs crates/ui-bridge/tests/schema.rs; do git diff --quiet HEAD -- $p; echo "$p=$?"; done | tee .tmp/ui-integration/zero-diff.txt
```
- **DoD**：全部 `=0`；`zero-diff.txt` 生成。

### 17.2 封闭面计数取证

- **做什么**：逐项断言 `api.rs` 4 个 `pub fn`、`KeywordCardView` 10、`SettingsView` 8、`CardDetailView` 5、`CardBodyView` 4、`KeywordCardDraft` 13、`SettingsFacts` 6；DTO 侧 7 组字段数（同 W-UI-04.2）。
- **文件**：无改动。
- **禁止**：不通过新增字段"补齐"计数；计数口径以既有 `kw_fields` 语义（4 空格 `pub`）为准。
- **依赖**：W-UI-17.1。
- **证据**：
```bash
grep -c '^pub fn ' crates/application/src/gate/api.rs                        # 4
grep -cE '^    pub ' <(sed -n '/pub struct KeywordCardView/,/^}/p' crates/ui-bridge/src/keyword_card/view.rs)      # 10
grep -cE '^    pub ' <(sed -n '/pub struct SettingsView/,/^}/p' crates/ui-bridge/src/settings/view.rs)             # 8
grep -cE '^    pub ' <(sed -n '/pub struct CardDetailView/,/^}/p' crates/ui-bridge/src/keyword_card/view.rs)       # 5
grep -cE '^    pub ' <(sed -n '/pub struct CardBodyView/,/^}/p' crates/ui-bridge/src/keyword_card/view.rs)         # 4
grep -cE '^    pub ' <(sed -n '/pub struct KeywordCardDraft/,/^}/p' crates/domain/src/model/gate_keyword.rs)       # 13
grep -cE '^    pub ' <(sed -n '/pub struct SettingsFacts/,/^}/p' crates/ui-bridge/src/settings/view.rs)            # 6
```
- **DoD**：7 个计数逐项等于 **4/10/8/5/4/13/6**。

### 17.3 断言 AC 汇总、target 与平台口径

- **做什么**：校验 AC 编号汇总 = 130、`NEW_TARGETS` = 25、`--rule=targets` 退 0、mac 口径门禁退 0。
- **文件**：无改动。
- **禁止**：不改 `spec.md` 的 AC 编号；不改 inventory。
- **依赖**：W-UI-16.1。
- **证据**：
```bash
ac_total                                        # 130
new_targets                                     # 25
bash scripts/check-b4-gates.sh --rule=targets   # 0
bash scripts/check-mac-scope.sh --rule=claims   # 0
bash scripts/check-g2-claims.sh                 # 0
grep -rc 'target_os = "macos"' app/src-tauri crates/platform-win/src | grep -v ':0$' | wc -l   # 0
```
- **DoD**：`ac_total`=**130**、`new_targets`=**25**、三条门禁退 0、mac 分支文件数 = **0**。

---

## 18. 执行全量回归与分批提交（W-UI-18 · 批次收口，C7 之后）

**AC 映射**：AC-B4-128、AC-B4-129、AC-B4-130。**依赖**：W-UI-17。

### 18.1 编译、静态检查与全量测试

- **做什么**：执行 `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`；记录通过/失败/忽略数。
- **文件**：无改动；证据写入 `.tmp/ui-integration/test.txt`。
- **禁止**：不用 `#[allow]`/`#[expect]`/`--no-verify` 规避；不改既有用例名；`ignored` 必须仍为 9。
- **依赖**：W-UI-01~W-UI-17。
- **证据**：
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace 2>&1 | tee .tmp/ui-integration/test.txt
grep -oE '[0-9]+ (passed|failed|ignored)' .tmp/ui-integration/test.txt | sort -u
cargo test --workspace -- --list --ignored | tail -2
```
- **DoD**：前三条退 0；`failed=0`；`ignored=9`；`passed ≥854`（基线 825 + 预测新增 29）。

### 18.2 逐条跑 AC 验收命令组

- **做什么**：按 §0.2 逐条执行 AC-B4-111~130 的证据命令与用例名，结果回填 `.tmp/ui-integration/ac-board.md`。
- **文件**：无改动；产物 `.tmp/ui-integration/ac-board.md`（已勾选）。
- **禁止**：不跳条；不以"脚本已跑过"代替单条取证。
- **依赖**：W-UI-18.1。
- **证据**：
```bash
# 逐条执行 design §2.4.2 的命令组（0)~6)），任何非 0 即停止
bash scripts/check-ui-integration.sh --all
for r in source budget record outbound zero-transcript targets doc-plan; do bash scripts/check-b4-gates.sh --rule=$r || exit 1; done
bash scripts/scan-secrets.sh
bash scripts/scan-secrets-docs.sh --all-md
node scripts/check-bindings.mjs
npm --prefix app run typecheck
```
- **DoD**：上述全组退 0；`ac-board.md` 中 20 条 AC 全部勾选并注明证据路径。

### 18.3 依赖与合规终检

- **做什么**：实跑 `cargo deny check licenses bans sources advisories`；确认 `deny.toml` 零 diff；确认 `ui-bridge` 依赖树无新增；确认 `ci.yml` 零 diff。
- **文件**：无改动；证据 `.tmp/ui-integration/deny.txt`（覆盖 W-UI-03.5）。
- **禁止**：不放宽 allow/exceptions；不改 CI。
- **依赖**：W-UI-18.1。
- **证据**：
```bash
cargo deny check licenses bans sources advisories
git diff --quiet HEAD -- deny.toml .github/workflows/ci.yml; echo $?
cargo tree -p meeting-copilot-ui-bridge --edges normal --prefix none | grep -cE '^(tauri|windows|objc2|reqwest|hyper|sqlx)'   # 0
```
- **DoD**：`cargo deny` 退 0；零 diff 退出码 0；`ui-bridge` 新增依赖前缀命中 = **0**。

### 18.4 按 C1~C7 分批提交并核对回滚路径

- **做什么**：按 §0.7 顺序提交；每次提交后执行该批前置门禁；最后核对 `git status --short` 只剩 `.tmp/`（不入库）。
- **文件**：不新增。
- **禁止**：不 `push`；不 `--force`；不 `--no-verify`；不合并提交（脚本面必须独立于 C1~C5 之后）。
- **依赖**：W-UI-18.2、W-UI-18.3。
- **证据**：
```bash
git status --short
git log --oneline -7
git diff --stat HEAD~7..HEAD
```
- **DoD**：7 个提交顺序与 §0.7 一致；每个提交的门禁证据已记录；工作区无源码残留改动。
- **回滚路径**：若某批门禁失败，`git revert` 该批提交（不改历史）；若 `windows-sys` 编译失败，回退到 C1 前状态并登记 `X-UI-11`；若六规则误报，只调整脚本扫描面（不改被扫代码语义），且不得放宽禁词清单。

---

## 19. 出口判据（Definition of Done，缺一即本阶段未完成）

1. AC-B4-111~130 共 **20 条**全部具备可复现证据（命令退 0 / grep 计数达标 / 用例名存在且通过），`ac-board.md` 20/20 勾选。
2. 6 个命令封闭：`grep -cE 'fn (settings_snapshot|settings_probe|settings_save|settings_clear|keyword_cards_snapshot|keyword_card_open)' app/src-tauri/src/ipc/mod.rs` = **6**（`ipc_version` 不计入）。
3. DTO 封闭：字段计数 **11/5/4/3/10/4/5**；两组禁用字段命中 **0**；`ClickState` 在 `dto.rs` = **0**；`citation_alignment` 仅在 `CardDetailDto` = **1**。
4. 端口封闭：`ports.rs` 3 trait + 6 类纯数据（`SettingsConfig` **4**/`ProbeAttempt` **4**/`ProbeFailure` **8 态**/`PortError` **4 类**/`SecretBytes` 无载荷/`ProbeCall` 单次）；`sources.rs` 3 trait + `6/2/6`（`FactsSnapshot`/`CardSnapshotEntry`/`ViewFactsSnapshot`）；`application/settings/**` 0 IO/网络/时钟；`platform-win` 非 Windows 返回 `Unsupported`。
5. 凭据边界：`secure_store.rs` 含 `CredWriteW`/`CredReadW`/`CredDeleteW`/`CRED_TYPE_GENERIC`/`CRED_PERSIST_LOCAL_MACHINE`；`std::fs` = **0**；`keyring` = **0**；`windows-sys` **仅 0.45.0 单版本**、仅 `cfg(windows)`、feature 仅两项。
6. 探针两段式：`probe.rs` 含 `assemble`/`BearerTransport`/`TlsTransport`；无 `LocalObserver`；`ipc/**` 网络符号 = **0**；回执只含相对 `remaining_ms`，`cooldown_until_ms` 仅内部镜像（X-UI-13）。
7. 前端：六规则 `--all` 退 **0**；`IPC_COMMANDS` = **6**；`connect-src` = **0**；`aria-disabled`/`onClick={enabled` 各 ≥1；`SEED_CARDS` = **0**；`ClickState` 拆平的扁平两字段已在前端类型中体现。
8. 门禁：`check-b4-gates.sh` 7 条规则 ×**0**，规则数仍 **7**，`--rule=targets` 不追加 target；新增 `check-ui-integration.sh` 六规则 ×**0**，且两者**只在 C6（晚于 C1~C5）**落盘。
9. 依赖：`deny.toml` 零 diff；`cargo deny` 退 **0**；`ui-bridge` 依赖树零新增；`Cargo.lock` 的 `windows-sys` 版本集合与基线一致（无新增 0.59）。
10. 零 diff：§17.1 全部路径 `git diff --quiet` 退 **0**；`NEW_TARGETS` = **25**；AC 汇总 = **130**；U-05 整体仍 `open`。
11. 全量：`cargo fmt --check` / `cargo clippy -D warnings` / `cargo test --workspace` 均退 **0**，`failed=0`、`ignored=9`。
12. 文档：`b4-status.md` 恰 **1** 个 `^## §28`、**1** 个 `^## §29`；含 `X-UI-01` 替代用例、`windows-sys 0.45.0`（X-UI-11）、C6 后置（X-UI-12）、`remainingMs` 口径（X-UI-13）；README 三条宣称命中 **0**。
13. 不宣称：真实联网 E2E、真机体验、macOS 支持、"设置向导已可用"、"产品级卡片已交付"均未出现于任何交付文档；`tests/replay/` 仍不存在；9 个 `#[ignore]` 名称与数量不变。