# B2 离线闭环 · 编码任务清单（tasks-b2-offline-closure）

> 上游依据：`.codeartsdoer/specs/b2_offline_closure/spec.md`（第 8 章 AC、第 9 章偏差/冲突）、`.codeartsdoer/specs/b2_offline_closure/design.md`（第二章增量设计、第三章测试映射、第四章 P0~P5）。
> 本文件是 `docs/sdd/tasks.md` 的**子集增补清单**，只覆盖 W-00 / W-C / W-D / W-E / W-F；不修改 `docs/sdd/tasks.md` 既有条目（spec 9.4-3）。
> 本轮只做任务拆解与验收定义，不含实现代码。

## 0. 编号规则与执行口径

### 0.1 三级编号（与既有清单不冲突）

| 层级 | 形式 | 示例 | 说明 |
|---|---|---|---|
| 批次 | `W-<段>` | `W-00`、`W-C`、`W-D`、`W-E`、`W-F` | 对应 spec 5.1~5.5 |
| 主任务 | `W-<段>-<n>` | `W-C-3` | 批次内连续，不与 `docs/sdd/tasks.md` 的 `15.`~`75.` 数字主任务重叠 |
| 子任务 | `W-<段>-<n>.<m>` | `W-C-3.4` | 批次内全局唯一；`m` 在同一主任务内连续 |

隔离说明：`AC-B2-OC-*` 是验收编号、`W-*` 是任务编号、`D1~D11` 是偏差编号、`P0~P5` 是实施阶段，四套编号互不复用；引用 LLM 项一律写全 `AC-LLM-*`（spec X5）。
并列写法：同一追溯行内多个 AC 可继承前缀（如 `AC-B2-OC-C01、C03`），但**禁止跨族简写**（如 `AC-20`、`AC-23`）；每个 AC 在本文件至少有一处全前缀出现，供机器反查。

### 0.2 子任务书写格式（强制）

```
**W-X-n.m 标题** · `目标文件路径`
  - 追溯：AC-B2-OC-xx · design 章节号
  - DoD：可机器判定的完成条件（退出码 / 断言值 / 计数 / 差集为空）
  - 证据：用例名（含 AC 绑定）或门禁命令
```

- 每条子任务必须能回答"跑什么 → 期望什么"，禁止只写"完成实现"。
- 涉及新增用例的子任务，用例名必须以 `snake_case` 命名且能唯一反查 `AC-B2-OC-*`（spec R5）。
- 涉及既有文件修改的子任务，只允许"追加 / 收紧断言"，禁止删除既有用例名（spec 4.5-2、AC-B2-OC-E11）。

### 0.3 阶段与批次映射

| 批次 | 实施阶段（design 4.3） | 前置 | 完成判据 |
|---|---|---|---|
| W-00 | P0-pre（基线前置） | 无 | 基线数字与门禁口径固化，失败项归因完成 |
| W-C（C-1/C-2） | P0 | W-00 | 纯函数与策略 L1 用例全绿，既有 329 不回退 |
| W-C（C-3~C-6） | P1 | W-C-1、W-C-2 | `capture_degraded_guard` 12 例通过 |
| W-D | P2 | P0（可与 P1 并行） | `capture_status_push` 13 例通过，既有服务用例不回退 |
| W-E（E-1~E-4） | P3 | P1、P2 | `capture_fakes` 7 例 + `capture_offline_gate` 3 例通过，≥4 文件复用装配 |
| W-E（E-5~E-7） | P4 | P3 | 三脚本退出码 0、`--self-test` 退出 1、用例名差集为空 |
| W-F | P5 | P4 | F01~F06 三态填齐，结论仅"完成 / 部分完成" |

### 0.4 门禁命令模板（Windows + git-bash，spec X7/X8、design 3.4）

```bash
# 1) 工具链绝对路径：先查 rustup 目录下的实际 toolchain 名，避免 cargo shim 的 STATUS_DLL_NOT_FOUND
ls "$HOME/.rustup/toolchains/"
TC_BIN="$HOME/.rustup/toolchains/<resolved-toolchain>/bin"

# 2) 隔离临时目录（默认临时目录拒绝写入会导致依赖磁盘的用例失败）
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"

# 3) 门禁序列
"$TC_BIN/cargo" fmt --all --check
"$TC_BIN/cargo" clippy --workspace --all-targets -- -D warnings
"$TC_BIN/cargo" test --workspace --no-fail-fast
bash scripts/check-layering.sh
bash scripts/scan-secrets.sh
"$TC_BIN/cargo" deny --offline check licenses bans sources advisories
bash scripts/check-capture-offline.sh
bash scripts/check-g2-claims.sh
bash scripts/test-name-inventory.sh --diff
```

退出码语义：`0=PASS / 1=命中违规 / 2=扫描面缺失`（design 3.4）；失败输出必须为 `文件:行号`（spec 4.4-2）。

### 0.5 关键口径裁决（本轮任务实现必须遵守）

| 编号 | 口径 | 落实位置 |
|---|---|---|
| K1 | 先合并后判门限：`merge_for_persist` → `persist_gate`，顺序不可颠倒 | W-C-1.3 先于 W-C-3.4；合并与门限必须在同一决策点，禁止"先落盘旧记录再暂存新记录"（design 1.2.2） |
| K2 | 端口全部留 application 层，`crates/domain/**` 零改动 | W-C-2、W-D-5.1；载荷白名单静态表放**测试侧**常量，不在域层新增 `ChannelKey::field_names()`（覆盖 design 2.3.2 备选口径，登记为 D9 落实方式） |
| K3 | 失效保护取"独立模块"分支 | W-C-3 → `capture/degraded_guard.rs`（保留 `tasks.md` 23.3 的路径与名称） |
| K4 | 订阅用有界队列 + 显式 `drain_status(now)`，不启线程、不引 async | W-D-3；用例必须显式推进 drain（R8） |
| K5 | `Metric::ALL` 契约用例落 infrastructure 侧，跨包执行 | W-E-5.2 + W-E-5.3（D8） |
| K6 | D6/D7 只收紧夹具与断言，两个用例名保留 | W-C-4.6；`adjacent_gaps_merge_before_write` / `non_adjacent_gaps_remain_two_records` |

### 0.6 覆盖矩阵（批次 → AC → 任务量）

| 批次 | 覆盖 AC | 主任务 | 子任务 | 新增用例 |
|---|---|---|---|---|
| W-00 | OC-01~05 | 3 | 6 | 0 |
| W-C | C01~C09 | 6 | 36 | 32 |
| W-D | D01~D10 | 6 | 31 | 18 |
| W-E | E01~E13 | 7 | 31 | 17 |
| W-F | F01~F06、OC-01~05 | 3 | 12 | 0 |
| **合计** | **OC-01~05、C01~C09、D01~D10、E01~E13、F01~F06** | **25** | **116** | **67** |

新增用例分布核算：L1 纯函数 14 + 4 + 5 = 23；L1/L2 失效保护 12；L2 状态推送 13；L2 替身 7；L2 离线门禁 3；L3 契约冻结 7；既有文件新增 2 = **67**（与 design 3.3 一致）。

---

## 批次 W-00 · 基线校核与测试口径固化（P0-pre，AC-B2-OC-01~05）

**目标**：把"进度记录数字"降级为参考、把"实测 + 门禁退出码"升为唯一完成依据。
**前置**：无。**产出**：`docs/progress/b2-offline-closure.md` 的基线章节（W-F-3 复用）。

### W-00-1 环境偏差登记（AC-02）
- [ ] **W-00-1.1 登记工具链绝对路径与原因** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-02 · spec X7 · design 3.4
  - DoD：文档含 `toolchain_dir`、`cargo_executable`、`原因=cargo shim 报 STATUS_DLL_NOT_FOUND`、`等价性说明` 四项；路径可 `test -x "$TC_BIN/cargo"` 验证
  - 证据：`bash -c 'test -x "$TC_BIN/cargo"; echo $?'` 输出 0
- [ ] **W-00-1.2 登记隔离临时目录与重跑结果** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-02 · spec X8
  - DoD：记录 `TEMP/TMP` 实际路径、默认目录不可见原因、隔离后失败用例转通过的原用例名
  - 证据：`echo "$TEMP"` 输出位于 `$PWD/.tmp/` 下

### W-00-2 基线复现与失败归因（AC-01/03/05）
- [ ] **W-00-2.1 复现全量基线数字** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-05、AC-B2-OC-01 · design 3.2.1
  - DoD：`cargo test --workspace --no-fail-fast` 退出码 0；记录 `passed=329 / failed=0 / ignored=8`（或本次实测值，并以实测覆盖）；"记录值 / 实测值"两列并存
  - 证据：`"$TC_BIN/cargo" test --workspace --no-fail-fast`
- [ ] **W-00-2.2 失败用例根因分类表** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-03 · spec 5.1.3
  - DoD：无失败时写"无失败用例"并附退出码；有失败时每条含 `用例名 / 根因分类(代码缺陷|环境限制) / 复现命令 / 处置` 四列
  - 证据：`grep -c "环境限制\|代码缺陷" docs/progress/b2-offline-closure.md` ≥ 失败条数（或 0 失败时命中 0）

### W-00-3 既有实现不重写核验（AC-04）
- [ ] **W-00-3.1 建立改动白名单并核验 diff** · `crates/application/src/capture/`、`docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-04 · spec X1
  - DoD：`git diff --stat` 中既有 capture 文件无"整文件重写"（判定：单文件删除行 > 新增行的 50% 即判失败）；白名单为 `service.rs`、`types.rs`、`capture_ports.rs`、`mod.rs`、`capture_gap_merge.rs`
  - 证据：`git diff --stat -- crates/application/src/capture`
- [ ] **W-00-3.2 标注过期进度数字** · `docs/progress/b2-status.md`
  - 追溯：AC-B2-OC-01 · spec 4.4-4
  - DoD：`b2-status.md` 中被实测推翻的数字（`48 passed`、`265 基线`、`≥320`）逐条追加"已过期，以 b2-offline-closure 实测为准"
  - 证据：`grep -c "已过期" docs/progress/b2-status.md` ≥ 3

---

## 批次 W-C · 双通道失效保护（AC-B2-OC-C01~C09）

**目标**：缺口"合并 → 门限 → 落盘"只在单点决策；失效期间会话与要点零写入、零删除、零迁移。
**阶段**：W-C-1/C-2 = P0；W-C-3~C-6 = P1。**前置**：W-00。

### W-C-1 策略字段与门限/边界纯函数（P0）
- [ ] **W-C-1.1 追加 `CapturePolicy` 6 个新字段与校验** · `crates/application/src/capture/types.rs`
  - 追溯：AC-B2-OC-C01、C08、C09、D04、D05、D07 · design 2.2.2-E、2.3.2、R10
  - DoD：新增 `gap_reopen_merge_ms(1000,1..=2000)`、`status_queue_capacity(64,固定)`、`status_handler_timeout_ms(50,1..=500)`、`status_latency_ring(256,>=256)`、`seq_upper_bound_margin(4096,>=1)`、`boundary_contact_max_ms(400,0..=1000)`；`Default` 保持既有 14 字段取值不变；越界构造返回 `CapturePolicyError::OutOfRange`
  - 证据：`policy_default_keeps_existing_values`、`policy_rejects_out_of_range_new_fields`
- [ ] **W-C-1.2 实现缺口落盘门限纯函数** · `crates/application/src/capture/capture_gap_gate.rs`
  - 追溯：AC-B2-OC-C01、C02 · design 2.2.2-E、I1、K1
  - DoD：`persist_gate(start,end,1000)` 在 `duration=999`→`DropShort`、`1000/1001`→`Persist`；`end<=start` 时返回 `DropShort` 且不 panic；函数无 I/O、无时钟、无 `unwrap()`
  - 证据：`gap_gate_persist_boundary_999_1000_1001`、`gap_gate_rejects_non_positive_interval`
- [ ] **W-C-1.3 实现"先合并后判门限"纯函数** · `crates/application/src/capture/capture_gap_gate.rs`
  - 追溯：AC-B2-OC-C03 · design 2.2.2-E、D4、K1
  - DoD：`merge_for_persist(pending,incoming,reopen_merge_ms=1000)` 在 `incoming.start <= pending.end + 1000` 时合并，保留最早 `start`、最晚 `end`、`backfilled` 取逻辑或；否则返回 `None`
  - 证据：`merge_for_persist_merges_within_reopen_window`、`merge_for_persist_keeps_earliest_start_and_latest_end`、`merge_for_persist_rejects_beyond_window`
- [ ] **W-C-1.4 实现恢复边界纯函数** · `crates/application/src/capture/capture_gap_gate.rs`
  - 追溯：AC-B2-OC-C04 · design 2.2.2-E、I3
  - DoD：`resume_boundary(close,start,8000)` 满足 `resume >= start` 且 `close - resume <= 8000` 且 `close - resume == min(8000, close-start)`；覆盖 `duration<8000`、`=8000`、`>8000` 三例
  - 证据：`resume_boundary_short_gap_uses_full_duration`、`resume_boundary_caps_at_max_window`、`resume_boundary_exact_window_is_identity`
- [ ] **W-C-1.5 实现区间代数断言纯函数** · `crates/application/src/capture/capture_gap_gate.rs`
  - 追溯：AC-B2-OC-C09、C02 · design 2.2.2-E、I2
  - DoD：`validate_intervals(gaps,segments,400)` 对缺口两两重叠、片段两两重叠、片段与缺口重叠 >400ms 分别返回对应 `IntervalViolation`；`overlap_ms <= 400` 判通过；空集合判通过
  - 证据：`validate_intervals_rejects_gap_overlap`、`validate_intervals_allows_boundary_contact_400ms`、`validate_intervals_rejects_overlap_beyond_boundary`、`validate_intervals_accepts_empty_sets`
- [ ] **W-C-1.6 实现诊断计数模块** · `crates/application/src/capture/diagnostics.rs`
  - 追溯：AC-B2-OC-C01、C06、C08、D04、D05、E01 · design 2.2.2-C
  - DoD：`DiagnosticCounter` 为 13 项封闭枚举；`CountingDiagnostics::bump/snapshot` 只累加、只返回 `DiagnosticsSnapshot`；不含任何字符串事件名与载荷字段；`Mutex` 毒化用 `unwrap_or_else(into_inner)`
  - 证据：`diagnostics_bump_accumulates_per_counter`、`diagnostics_snapshot_has_no_payload_fields`
- [ ] **W-C-1.7 追加模块导出** · `crates/application/src/capture/mod.rs`
  - 追溯：design 2.1.2（模块导出行）
  - DoD：`capture_gap_gate`、`diagnostics`、`degraded_guard`、`status_publisher`、`subscriptions` 五个模块与所需符号可见；`cargo build -p meeting-copilot-application` 退出码 0；无 `pub use *` 之外的重命名
  - 证据：`"$TC_BIN/cargo" build -p meeting-copilot-application`
- [ ] **W-C-1.8 补齐 L1 纯函数用例（14 例）** · `crates/application/src/capture/capture_gap_gate.rs`（`#[cfg(test)]`）
  - 追溯：AC-B2-OC-C01~C04、C09 · design 3.3（L1 缺口纯函数 14 例）
  - DoD：14 例全部通过且每个用例名含 `gap` 或 `gate` 或 `resume` 或 `interval`；`cargo test -p meeting-copilot-application --lib` 的 lib 通过数较基线净增 ≥14
  - 证据：`"$TC_BIN/cargo" test -p meeting-copilot-application --lib`
- [ ] **W-C-1.9 补齐 L1 诊断/策略用例（4 例）** · `crates/application/src/capture/diagnostics.rs`、`types.rs`（`#[cfg(test)]`）
  - 追溯：AC-B2-OC-C01、D05、D07 · design 3.3（L1 诊断/策略 4 例）
  - DoD：覆盖计数累加、`Default` 取值不变、6 个新字段越界各 1 例（可合并为表驱动 1 例）
  - 证据：`policy_default_keeps_existing_values`、`policy_rejects_out_of_range_new_fields`、`diagnostics_bump_accumulates_per_counter`、`diagnostics_snapshot_has_no_payload_fields`

### W-C-2 端口扩展：只读要点 + 诊断计数（P0）
- [ ] **W-C-2.1 新增 `InsightInvariantPort`（只读）** · `crates/application/src/capture/capture_ports.rs`
  - 追溯：AC-B2-OC-C05、C06、E01 · design 2.2.2-C、K2
  - DoD：trait 仅含 `read_insight_ids(&self, &SessionId) -> Result<Vec<String>, InsightReadError>`；无 `save/update/delete/clear` 方法；`crates/domain` 零改动（`git diff --stat crates/domain` 为空）
  - 证据：`insight_port_surface_is_read_only`、`git diff --stat crates/domain`
- [ ] **W-C-2.2 新增 `DiagnosticsSink` 与封闭计数枚举** · `crates/application/src/capture/capture_ports.rs`
  - 追溯：AC-B2-OC-C01、C06、D05、D08、F04 · design 2.2.2-C
  - DoD：`bump(&self, DiagnosticCounter, u64)` + `snapshot()`；不接受 `&str`/`String` 事件名；枚举 13 项与 design 一致
  - 证据：`diagnostics_sink_accepts_only_closed_counter`
- [ ] **W-C-2.3 提供 Noop 只读实现** · `crates/application/src/capture/capture_ports.rs`
  - 追溯：design 2.2.2-D（旧构造函数转发）
  - DoD：`NoopInsight` 返回 `Ok(vec![])`、`NoopDiagnostics` 不 panic；`CaptureChannelService::new` / `with_ports` 行为不变（既有用例全绿）
  - 证据：`"$TC_BIN/cargo" test -p meeting-copilot-application --test capture_service`

### W-C-3 失效保护状态机（P1，独立模块）
- [ ] **W-C-3.1 建立 `DegradedGuard` 结构与构造** · `crates/application/src/capture/degraded_guard.rs`
  - 追溯：AC-B2-OC-C05 · design 2.2.2-A、2.1.3(1)、K3
  - DoD：字段仅含 `open/pending/last_seen/计数`，依赖集为 `Arc<dyn CaptureClock>` + `Arc<dyn InsightInvariantPort>` + `Arc<dyn DiagnosticsSink>` + `CapturePolicy`；**不含任何仓储写 trait**（结构性约束 I5）
  - 证据：`check-capture-offline.sh --rule=guard-deps` 命中 0
- [ ] **W-C-3.2 实现观测时刻回退检测** · `crates/application/src/capture/degraded_guard.rs`
  - 追溯：AC-B2-OC-C01（异常场景 4） · design 2.1.3(1) 时钟回退分支
  - DoD：`evidence_at < last_seen` 返回 `GuardError::ClockRegression`、`clock_regression += 1`，不改变 `open`/`pending`；合法时刻推进 `last_seen`
  - 证据：`clock_regression_is_rejected_and_counted`
- [ ] **W-C-3.3 实现 `on_availability` 四分支** · `crates/application/src/capture/degraded_guard.rs`
  - 追溯：AC-B2-OC-C01、C07、C08 · design 2.1.3(1) 开启/持续/关闭/重开
  - DoD：`Opened{first_push:true}` 仅首次、`Sustained` 不改起点不改原因、`Closed` 触发门限、`Reopened` 在 `start <= prev_end + gap_reopen_merge_ms` 时并入；`gap_opened/gap_reopened` 计数与分支一一对应
  - 证据：`flapping_within_reopen_window_merges_into_one_gap`、`sustained_unavailable_keeps_first_start_and_reason`
- [ ] **W-C-3.4 实现"合并 → 门限 → 落盘"单点决策** · `crates/application/src/capture/degraded_guard.rs`
  - 追溯：AC-B2-OC-C01、C02、C03 · design 2.1.3(2)、I1、K1
  - DoD：关闭时先 `merge_for_persist` 再 `persist_gate`；`Persisted` 时 `end > start` 且 `duration >= gap_min_ms`；`DroppedShort` 时 `short_gap_dropped == 1` 且**未调用** `gap_log.append`；不存在"先落盘旧记录"分支
  - 证据：`short_gap_500ms_is_not_persisted`、`merged_short_gaps_pass_gate`、`two_short_gaps_merge_then_persist`
- [ ] **W-C-3.5 实现恢复边界与回补意图构造** · `crates/application/src/capture/degraded_guard.rs`
  - 追溯：AC-B2-OC-C04、C09 · design 2.1.3(2)、I3/I4
  - DoD：`AsrIntent.window_start == resume_boundary(...)`、`window_end == close`、`chunk_count = ceil((close-resume)/400)`；更早区间 `[start, resume_from)` 不产生任何片段（返回集合为空）
  - 证据：`resume_from_is_close_minus_capped_duration`、`earlier_than_resume_window_produces_no_segment`
- [ ] **W-C-3.6 实现要点不变量校验与降级** · `crates/application/src/capture/degraded_guard.rs`
  - 追溯：AC-B2-OC-C05、C06 · design 2.1.3(1)、I7
  - DoD：开启与持续分支各调用 `read_insight_ids` 1 次；`Err` 时 `insight_check_skipped += 1` 且校验被跳过；任何分支都不调用写/删/迁移（哨兵计数恒 0）
  - 证据：`gap_300s_keeps_session_and_insight_ids`、`insight_read_failure_skips_check_without_delete`
- [ ] **W-C-3.7 暴露只读查询面** · `crates/application/src/capture/degraded_guard.rs`
  - 追溯：design 2.2.2-A（`pending` / `diagnostics`）
  - DoD：`pending(&SessionId) -> Option<CaptureGapRecord>` 与 `diagnostics() -> DiagnosticsSnapshot` 均为 `&self` 只读；无 `set/append/flush` 公开方法
  - 证据：`guard_public_surface_is_read_only`（静态方法集合比对）

### W-C-4 服务门面缺口链路接线（P1）
- [ ] **W-C-4.1 新增 `with_offline_ports` 并保留旧构造** · `crates/application/src/capture/service.rs`
  - 追溯：design 2.2.2-D · K2
  - DoD：新构造含 7 个端口 + policy；`new` 与 `with_ports` 转发到新构造（使用 Noop 只读实现）；既有 7 个 `capture_service` 用例与 2 个 `capture_gap_merge` 用例编译并通过
  - 证据：`"$TC_BIN/cargo" test -p meeting-copilot-application --test capture_service`
- [ ] **W-C-4.2 固化 `observe` 副作用顺序契约** · `crates/application/src/capture/service.rs`
  - 追溯：AC-B2-OC-D02、C07 · design 2.2.2-D（observe 顺序）
  - DoD：顺序为 `状态机 → 通道事件发布 → 仲裁/切换记录 → 汇总事件发布 → 守卫缺口判定 → 回补投递`；`Rejected/Dropped` 立即返回且不触碰发布与缺口；锁序固定为 `registry → publisher.snapshot → publisher.seq → subscriptions → guard → diagnostics`
  - 证据：`observe_order_is_state_then_publish_then_guard`、`rejected_observation_skips_publish_and_gap`
- [ ] **W-C-4.3 `handle_gap` 改由守卫决策** · `crates/application/src/capture/service.rs`
  - 追溯：AC-B2-OC-C01~C08 · design 1.2.2、2.1.3(2)
  - DoD：`handle_gap` 不再直接 `gaps.entry(...).or_insert`；改为调用 `guard.on_availability(session, availability, at)` 并依 `GuardTransition` 分派；`gap_start` 字段删除或降级为守卫内部状态（若删除，需确认无其它引用）
  - 证据：`short_gap_500ms_is_not_persisted`、`long_gap_30s_persists_single_half_open_record`
- [ ] **W-C-4.4 改写 `AsrIntent` 窗口起点** · `crates/application/src/capture/service.rs`
  - 追溯：AC-B2-OC-C04 · design 1.1.2（C04 扩展方向）
  - DoD：`window_start = resume_from`、`window_end = close`；`chunk_count` 按新窗口长度重算；预算拒绝时 `backfill_blocked += 1` 且 `backfilled=false`
  - 证据：`resume_from_is_close_minus_capped_duration`、`blocked_backfill_keeps_record_unbackfilled`
- [ ] **W-C-4.5 `flush_gaps` / `detach_session` 走同一门限** · `crates/application/src/capture/service.rs`
  - 追溯：AC-B2-OC-C01、C03 · design 1.2.1（detach 保留缺口语义）
  - DoD：会话结束强制关闭未结算缺口并执行 `merge_for_persist → persist_gate`；短缺口不落盘只计数；`detach_session` 不报错、不 panic
  - 证据：`detach_flushes_long_gap_but_drops_short_gap`
- [ ] **W-C-4.6 收紧既有 2 用例并新增门限用例** · `crates/application/tests/capture_gap_merge.rs`
  - 追溯：AC-B2-OC-C01、C03 · spec X2/X6 · design D6/D7 · K6
  - DoD：`adjacent_gaps_merge_before_write` 保留原名，夹具改为注入 `gap_min_ms=400` 的显式策略并新增"默认 1000 下 0 条"断言；`non_adjacent_gaps_remain_two_records` 保留原名，第二段起点由 `800` 改为 `1400`（间隔 490ms → 1090ms）；新增 2 条门限用例
  - 证据：`adjacent_gaps_merge_before_write`、`non_adjacent_gaps_remain_two_records`、`default_policy_drops_short_merged_gap`、`short_gap_only_counted_not_persisted`

### W-C-5 失效保护集成用例（P1，12 例）
- [ ] **W-C-5.1 C01 短缺口不落盘（1 例）** · `crates/application/tests/capture_degraded_guard.rs`
  - 追溯：AC-B2-OC-C01 · design 3.2.2
  - DoD：500ms 失效 → `gaps.len()==0` 且 `short_gap_dropped==1`
  - 证据：`short_gap_500ms_is_not_persisted`
- [ ] **W-C-5.2 C02 半开区间与时长（1 例）** · `crates/application/tests/capture_degraded_guard.rs`
  - 追溯：AC-B2-OC-C02 · design 3.2.2
  - DoD：30000ms 失效 → 记录数 1、`end > start`、`duration ∈ [29600,30400]`
  - 证据：`long_gap_30s_persists_single_half_open_record`
- [ ] **W-C-5.3 C03 先合并后判门限（2 例）** · `crates/application/tests/capture_degraded_guard.rs`
  - 追溯：AC-B2-OC-C03 · design 3.2.2、K1
  - DoD：600ms + 400ms 间隔 + 600ms → 合并 1 条 1600ms 并落盘；合并顺序不可反（纯函数层已由 W-C-1.3 守护）
  - 证据：`two_short_gaps_merge_then_persist`、`merged_short_gaps_pass_gate`
- [ ] **W-C-5.4 C04 恢复边界（2 例）** · `crates/application/tests/capture_degraded_guard.rs`
  - 追溯：AC-B2-OC-C04 · design 3.2.2
  - DoD：30000ms 缺口 → `resume_from == close - 8000`；更早 22000ms 区间片段数 0
  - 证据：`resume_from_is_close_minus_capped_duration`、`earlier_than_resume_window_produces_no_segment`
- [ ] **W-C-5.5 C05 会话与要点不变量（1 例）** · `crates/application/tests/capture_degraded_guard.rs`
  - 追溯：AC-B2-OC-C05 · design 3.2.2、I7
  - DoD：300s 失效期间哨兵仓储的迁移/删除/要点写入/摘要写入调用数为 0；要点 ID 集合与数量与失效前完全相等
  - 证据：`gap_300s_keeps_session_and_insight_ids`
- [ ] **W-C-5.6 C06 校验失败不降级为删除（1 例）** · `crates/application/tests/capture_degraded_guard.rs`
  - 追溯：AC-B2-OC-C06 · design 3.2.2
  - DoD：`read_insight_ids` 返回 `Err` → 删除/清空调用 0、`insight_check_skipped==1`
  - 证据：`insight_read_failure_skips_check_without_delete`
- [ ] **W-C-5.7 C07 单通道恢复 ≤5000ms（1 例）** · `crates/application/tests/capture_degraded_guard.rs`
  - 追溯：AC-B2-OC-C07 · design 3.2.2
  - DoD：音频通道恢复且偏好"字幕优先" → 汇总 `可用-音频`，`determined_at - evidence_at <= 5000`，缺口已关闭
  - 证据：`single_channel_recovery_within_5s`
- [ ] **W-C-5.8 C08 重复失效只告警 1 次（1 例）** · `crates/application/tests/capture_degraded_guard.rs`
  - 追溯：AC-B2-OC-C08 · design 3.2.2 · **依赖 W-D-2.2 汇总去重账本**
  - DoD：300s 内在"不可用/探测中"反复切换 → 汇总 `Unavailable` 推送次数恰 1，缺口记录 1 条
  - 证据：`repeated_flapping_pushes_unavailable_once`
- [ ] **W-C-5.9 C09 区间代数（2 例）** · `crates/application/tests/capture_degraded_guard.rs`
  - 追溯：AC-B2-OC-C09 · design 3.2.2、I2
  - DoD：片段两两不交；片段与缺口不重叠，仅允许 `<=400ms` 边界接触；违反时 `IntervalViolation` 被断言命中
  - 证据：`intervals_do_not_overlap_and_touch_within_boundary`、`interval_violation_is_reported_for_overlap`

### W-C-6 汇总可用性事件 DTO（P1 定义 / P2 发布）
- [ ] **W-C-6.1 新增 `CaptureAvailabilityEvent`（4 字段）** · `crates/application/src/capture/types.rs`
  - 追溯：AC-B2-OC-C07、C08、D08 · design 2.3.2、D11
  - DoD：字段恰为 `availability / gap_reason / code / sequence`；`Unavailable` 时 `code == E-AUDIO-002`；无 `text/target/reason_text`；旧 `availability()` 返回类型不变
  - 证据：`availability_event_has_four_whitelisted_fields`、`unavailable_event_reuses_audio_error_code`
- [ ] **W-C-6.2 定义 `GuardTransition.first_push` 语义** · `crates/application/src/capture/degraded_guard.rs`
  - 追溯：AC-B2-OC-C08、D10 · design 2.2.2-A
  - DoD：`Opened{first_push:true}` 仅缺口首次；`Sustained`/`Reopened` 的 `first_push` 恒 `false`；调用方据此决定是否发布汇总事件
  - 证据：`first_push_is_true_only_for_gap_open`

---

## 批次 W-D · 状态推送、快照与订阅（AC-B2-OC-D01~D10）

**目标**：单会话单序号流；先快照后扇出；订阅隔离、释放幂等、时延可离线判定。
**阶段**：P2（依赖 P0，可与 P1 并行）。**前置**：W-C-1。

### W-D-1 `StatusPublisher` 核心（P2）
- [ ] **W-D-1.1 建立发布器结构与构造** · `crates/application/src/capture/status_publisher.rs`
  - 追溯：AC-B2-OC-D01~D04 · design 2.2.2-B
  - DoD：字段私有：快照表 / 会话序号表 / 订阅表 / 时延环 / 去重账本；`new(clock, sink, policy)`；`Mutex` 毒化统一 `unwrap_or_else(into_inner)`；无 `unwrap()`
  - 证据：`publisher_constructs_with_noop_sink`
- [ ] **W-D-1.2 实现去重键与同状态不推送** · `crates/application/src/capture/status_publisher.rs`
  - 追溯：AC-B2-OC-D01 · design 2.1.3(3)
  - DoD：键 `(session, key)` + `state`；同状态返回 `NotPushed{SameState}`、序号不推进、订阅者 0 事件，但快照 `reason/changed_at` 已更新、`entered_at` 不变
  - 证据：`same_state_ten_times_pushes_once_and_updates_reason`
- [ ] **W-D-1.3 固化"快照 → 序号 → 扇出"顺序** · `crates/application/src/capture/status_publisher.rs`
  - 追溯：AC-B2-OC-D02 · design 2.1.3(3) 发布顺序硬约束
  - DoD：`Published` 返回时 `snapshot_for(session)` 必含本次状态；回调内查询快照可见本次 `state/reason`
  - 证据：`snapshot_visible_inside_subscriber_callback`
- [ ] **W-D-1.4 实现会话内单序号流** · `crates/application/src/capture/status_publisher.rs`
  - 追溯：AC-B2-OC-D03 · design 2.1.3(3)
  - DoD：序号按会话隔离、严格递增、通道事件与汇总事件共用；100 次变更后快照条目 1 条、序号长度 100 且无重复
  - 证据：`hundred_changes_share_one_increasing_sequence`
- [ ] **W-D-1.5 实现序号上界保护** · `crates/application/src/capture/status_publisher.rs`
  - 追溯：AC-B2-OC-D04 · design 2.2.2-B
  - DoD：序号距 `u64::MAX` 小于 `seq_upper_bound_margin` 时返回 `Rejected{SeqExhausted}`、`seq_overflow += 1`、快照不被覆盖、无回绕（下一序号仍小于上界）
  - 证据：`seq_near_upper_bound_is_rejected`

### W-D-2 汇总可用性发布（P2，C07/C08/D10 共用）
- [ ] **W-D-2.1 实现 `publish_availability`** · `crates/application/src/capture/status_publisher.rs`
  - 追溯：AC-B2-OC-D08、C07 · design 2.1.3(3)、D11
  - DoD：`Unavailable` 发 1 次并带 `E-AUDIO-002`；恢复后 `Available-*` 发 1 次；事件 `sequence` 来自同一会话序号流
  - 证据：`availability_transition_publishes_once_per_edge`
- [ ] **W-D-2.2 实现"同缺口只告警 1 次"去重账本** · `crates/application/src/capture/status_publisher.rs`
  - 追溯：AC-B2-OC-C08、D10 · design 2.1.3(1) 开启分支
  - DoD：同一缺口内原因变化 5 次 → 推送仍 1 次、快照条目 1 条、`StatusNotPushed` 计数 5
  - 证据：`repeated_flapping_pushes_unavailable_once`、`reason_changes_inside_gap_do_not_repush`
- [ ] **W-D-2.3 汇总事件与通道事件共用序号** · `crates/application/src/capture/status_publisher.rs`
  - 追溯：AC-B2-OC-D03 · design 2.3.2（共用序号流）
  - DoD：交替发布 1 个通道事件与 1 个汇总事件，得到 `[1,2]` 严格递增且无跳号冲突
  - 证据：`channel_and_availability_share_one_sequence`

### W-D-3 订阅与有界队列（P2）
- [ ] **W-D-3.1 实现每订阅者 64 容量有界队列** · `crates/application/src/capture/subscriptions.rs`
  - 追溯：AC-B2-OC-D05 · design 2.2.2-B、K4
  - DoD：容量固定 64（`policy.status_queue_capacity` 越界构造被拒）；满时丢最旧并 `queue_dropped += 1`；入队不阻塞
  - 证据：`full_queue_drops_oldest_and_counts`
- [ ] **W-D-3.2 实现 `subscribe` 与令牌登记** · `crates/application/src/capture/subscriptions.rs`
  - 追溯：AC-B2-OC-D05、D06 · design 2.2.2-B（复用域层 `Subscription`）
  - DoD：返回 `SubscriptionHandle`；每订阅者独立队列与独立失败计数；`crates/domain` 零改动
  - 证据：`subscribe_creates_independent_slots`
- [ ] **W-D-3.3 实现幂等 `release`** · `crates/application/src/capture/subscriptions.rs`
  - 追溯：AC-B2-OC-D06 · design 2.2.2-B
  - DoD：重复 `release` 无副作用；释放后该订阅者回调 0 次；其它订阅者不受影响
  - 证据：`released_subscriber_receives_zero_events`
- [ ] **W-D-3.4 实现 `drain(now)` 与订阅者隔离** · `crates/application/src/capture/subscriptions.rs`
  - 追溯：AC-B2-OC-D05 · design 2.1.3(3) drain 阶段、R9
  - DoD：drain 不持 `registry` 与 publisher 内部锁（先克隆句柄）；单订阅者抛错 → `subscriber_failure += 1` 且继续投递；单订阅者处理耗时超 `status_handler_timeout_ms` → 事件留在其队列并立即返回
  - 证据：`slow_subscriber_does_not_block_others`、`handler_error_only_counts`
- [ ] **W-D-3.5 实现 `DrainReport`** · `crates/application/src/capture/subscriptions.rs`
  - 追溯：AC-B2-OC-D05 · design R8
  - DoD：`DrainReport` 暴露 `delivered / pending / dropped / failures` 四项；未 drain 时 `pending > 0` 可观测
  - 证据：`drain_report_exposes_pending_before_drain`

### W-D-4 时延环形缓冲与统计（P2）
- [ ] **W-D-4.1 实现 256 容量时延环** · `crates/application/src/capture/status_publisher.rs`
  - 追溯：AC-B2-OC-D07 · design 2.1.3(4)
  - DoD：容量 `status_latency_ring`（>=256）固定；超出覆盖最旧；样本来源仅为注入时钟差值；无 `Instant::now`
  - 证据：`latency_ring_keeps_latest_256_samples`
- [ ] **W-D-4.2 实现 `LatencyStats{p50,p99,max}`** · `crates/application/src/capture/status_publisher.rs`
  - 追溯：AC-B2-OC-D07 · design 2.1.3(4)
  - DoD：`p99 = sorted[ceil(0.99*n)-1]`、`max = sorted[n-1]`；n=200 时 p99 取下标 197（用例以已知序列校验下标，不只看阈值）
  - 证据：`latency_p99_and_max_within_5s`、`latency_p99_uses_expected_index`

### W-D-5 门面接线与兼容（P2）
- [ ] **W-D-5.1 `with_offline_ports` 与旧构造转发** · `crates/application/src/capture/service.rs`
  - 追溯：design 2.2.2-D · K2、D2
  - DoD：既有 8 个公开方法签名不变；旧构造默认装配 Noop 只读实现 + 默认策略；`cargo build -p meeting-copilot-application` 退出码 0
  - 证据：`"$TC_BIN/cargo" build -p meeting-copilot-application`
- [ ] **W-D-5.2 新增 4 个只读/推进方法** · `crates/application/src/capture/service.rs`
  - 追溯：AC-B2-OC-D01~D09 · design 2.2.2-D
  - DoD：`availability_event`、`snapshot_for`、`drain_status(now)`、`subscribe_status(handler)` 均可用且不新增 `ErrorCode`；`observe` 返回值仍为 `ObservationOutcome`
  - 证据：`service_snapshot_for_matches_publisher_snapshot`
- [ ] **W-D-5.3 `snapshot()` 改读发布器快照** · `crates/application/src/capture/service.rs`
  - 追溯：AC-B2-OC-D09、D03 · design 1.2.3
  - DoD：无订阅者时发布 5 次不同状态 → `snapshot()` 条目状态为最后一次；后续订阅者 `snapshot_for` 结果一致；快照条目数恒为 1
  - 证据：`no_subscriber_still_updates_snapshot`

### W-D-6 状态推送集成用例（P2，13 例）
- [ ] **W-D-6.1 D01 去重与原因更新（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D01 · design 3.2.3
  - DoD：同键同状态 10 次 → 事件 1；原因由"仅 OCR"变为"版本不匹配"时事件数不变、快照原因已更新
  - 证据：`same_state_ten_times_pushes_once_and_updates_reason`
- [ ] **W-D-6.2 D02 先快照后扇出（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D02 · design 3.2.3
  - DoD：回调内 `snapshot_for` 的 `state/reason` 与事件字段完全一致
  - 证据：`snapshot_visible_inside_subscriber_callback`
- [ ] **W-D-6.3 D03 序号严格递增（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D03 · design 3.2.3
  - DoD：100 次变更 → 序号严格递增、长度 100、快照条目 1
  - 证据：`hundred_changes_share_one_increasing_sequence`
- [ ] **W-D-6.4 D03 补充：通道与汇总共用序号（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D03 · design 2.3.2
  - DoD：交替发布后序号集合为 `1..=n` 连续无重复
  - 证据：`channel_and_availability_share_one_sequence`
- [ ] **W-D-6.5 D04 序号上界保护（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D04 · design 3.2.3
  - DoD：注入近上界序号 → `Rejected{SeqExhausted}`、`seq_overflow==1`、快照条目与序号均未被覆盖
  - 证据：`seq_near_upper_bound_is_rejected`
- [ ] **W-D-6.6 D05 慢订阅者隔离（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D05 · design 3.2.3
  - DoD：A 正常、B 超时并向队列积压 → A 收到完整事件序列；B 的 `pending > 0`；发布方不阻塞（`drain_status` 立即返回）
  - 证据：`slow_subscriber_does_not_block_others`
- [ ] **W-D-6.7 D05 抛错只计数（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D05 · design 3.2.3
  - DoD：handler 返回失败 → `subscriber_failure==1`、其余订阅者仍收到事件
  - 证据：`handler_error_only_counts`
- [ ] **W-D-6.8 D06 释放后 0 回调（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D06 · design 3.2.3
  - DoD：释放者 0 次、其余各 10 次；重复 release 无变化
  - 证据：`released_subscriber_receives_zero_events`
- [ ] **W-D-6.9 D07 p99 与最大值（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D07 · design 3.2.3、R1
  - DoD：200 样本（注入时钟）→ `p99 <= 5000` 且 `max <= 5000` 且 `samples >= 200` 且环容量 256；用例注释声明"逻辑上限"
  - 证据：`latency_p99_and_max_within_5s`
- [ ] **W-D-6.10 D08 载荷字段计数（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D08 · design 2.3.2 字段白名单、K2
  - DoD：事件展开叶子字段 7、快照条目 6、汇总事件 4（测试侧静态白名单常量比对，域层零改动）
  - 证据：`payload_whitelist_field_counts`
- [ ] **W-D-6.11 D08 文本与样本扫描（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D08 · design 2.3.2
  - DoD：预置字幕文本与音频样本字节串在所有事件/快照/缺口记录的 `Debug` 输出中命中 0 次
  - 证据：`payload_scan_hits_no_text_or_sample`
- [ ] **W-D-6.12 D09 无订阅者仍更新快照（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D09 · design 3.2.3
  - DoD：5 次发布后快照为最后状态；之后订阅查询结果一致
  - 证据：`no_subscriber_still_updates_snapshot`
- [ ] **W-D-6.13 D10 缺口内原因变化不重复推送（1 例）** · `crates/application/tests/capture_status_push.rs`
  - 追溯：AC-B2-OC-D10 · design 3.2.3
  - DoD：原因变化 5 次 → 推送 1 次、快照 1 条、`StatusNotPushed` 计数 5
  - 证据：`reason_changes_inside_gap_do_not_repush`

---

## 批次 W-E · 离线装配、契约冻结与扫描门禁（AC-B2-OC-E01~E13）

**目标**：一套全替身装配被 ≥4 个测试文件复用；契约变体数有具名用例守护；三类扫描脚本可自检。
**阶段**：W-E-1~E-4 = P3；W-E-5~E-7 = P4。**前置**：W-C-4、W-D-5。

### W-E-1 能力替身集（P3）
- [ ] **W-E-1.1 `MemoryGapLog`：只追加 + 合并查询** · `crates/application/tests/support/fakes.rs`
  - 追溯：AC-B2-OC-E01、E03 · design 1.1.3-C、2.1.1
  - DoD：仅 `append` 与 `query(session)`；`query` 按 `gap_reopen_merge_ms(1000)` 合并相邻/重叠、按 `start` 升序、两两不重叠；无 `update/delete/remove/clear`
  - 证据：`gap_log_query_merges_adjacent_within_1s`
- [ ] **W-E-1.2 `MemorySwitchLog`：追加 + 同毫秒幂等** · `crates/application/tests/support/fakes.rs`
  - 追溯：AC-B2-OC-E01、E02 · design 1.1.2（E02 行）
  - DoD：`append` 后长度 +1；同会话同毫秒重复记录合并为 1 条（复用 `merge_same_millisecond` 语义）；无修改入口
  - 证据：`switch_log_append_is_monotonic_and_same_ms_idempotent`
- [ ] **W-E-1.3 `CountingAsrDelivery`：三类意图计数** · `crates/application/tests/support/fakes.rs`
  - 追溯：AC-B2-OC-E04 · design 1.1.3-C
  - DoD：按 `AsrIntentKind` 三类分别计数；`current_task` 返回最近任务；计数与实际 `deliver` 次数一致
  - 证据：`asr_delivery_counts_by_kind`
- [ ] **W-E-1.4 `CountingAudit`：无载荷审计** · `crates/application/tests/support/fakes.rs`
  - 追溯：AC-B2-OC-E04、D08 · design 1.1.3-C
  - DoD：审计条目结构体仅含 `kind / at / count` 类字段；不含 `text/samples/speaker/participants`
  - 证据：`audit_event_has_no_payload`
- [ ] **W-E-1.5 `CountingDiagnostics`：复用生产实现** · `crates/application/tests/support/fakes.rs`
  - 追溯：AC-B2-OC-E01、C01 · design 1.1.3-C
  - DoD：直接复用 `crate::capture::diagnostics::CountingDiagnostics`，不复制逻辑；`snapshot()` 可断言
  - 证据：`diagnostics_fake_reuses_production_counter`
- [ ] **W-E-1.6 `MonotonicSystemClock`：单进程单调不减** · `crates/application/tests/support/fakes.rs`
  - 追溯：AC-B2-OC-E05 · design 1.1.3-C
  - DoD：委托既有墙钟实现；连续两次 `mono_now` 后者 >= 前者；不 panic；不调用 `Instant::now`
  - 证据：`system_clock_fake_is_monotonic_non_decreasing`
- [ ] **W-E-1.7 `SentinelRepos`：写/删/迁移零调用哨兵** · `crates/application/tests/support/fakes.rs`
  - 追溯：AC-B2-OC-C05、C06 · design 2.1.3(1) I5
  - DoD：四仓储 trait 的写/删/迁移入口均累加计数；读取入口只读；用例可断言四项计数为 0
  - 证据：`sentinel_repos_count_zero_writes_during_gap`
- [ ] **W-E-1.8 `ConnectCounter`：建连计数为 0** · `crates/application/tests/support/fakes.rs`
  - 追溯：AC-B2-OC-E07 · design 1.1.3-C
  - DoD：提供 `connections() -> u64` 与 `record_attempt()`；替身装配下最小路径结束 `connections()==0`
  - 证据：`offline_min_path_has_zero_connections`
- [ ] **W-E-1.9 替身方法面审计（无更新/删除/通用发送）** · `crates/application/tests/support/fakes.rs`
  - 追溯：AC-B2-OC-E01、AC-B2-OC-E13 · design 2.1.1 结构性禁止
  - DoD：替身公开方法集合中不存在名为 `update*`/`delete*`/`remove*`/`clear*`/泛型 `send<...>` 的方法；无网络/设备/凭据调用
  - 证据：`check-capture-offline.sh --rule=fakes` 命中 0

### W-E-2 全替身离线装配（P3）
- [ ] **W-E-2.1 实现 `build_offline_capture`** · `crates/application/tests/support/mod.rs`
  - 追溯：AC-B2-OC-E06 · design 1.1.3-C、2.1.1
  - DoD：单函数返回装配好的 `CaptureChannelService` + 各替身句柄；函数体不含 `thread::sleep`/`Instant::now`/`SystemTime::now`/网络调用
  - 证据：`offline_harness_forbidden_calls_scan`、`check-capture-offline.sh --rule=harness`
- [ ] **W-E-2.2 完成 ≥4 个测试文件复用** · `crates/application/tests/`
  - 追溯：AC-B2-OC-E06 · design 3.2.4
  - DoD：`capture_degraded_guard.rs`、`capture_status_push.rs`、`capture_fakes.rs`、`capture_offline_gate.rs` 四个文件均调用 `build_offline_capture`；脚本统计复用文件数 >= 4
  - 证据：`bash scripts/check-capture-offline.sh --rule=harness`
- [ ] **W-E-2.3 装配函数静态扫描接入门禁** · `scripts/check-capture-offline.sh`
  - 追溯：AC-B2-OC-E06、AC-B2-OC-E09 · design 3.2.4
  - DoD：`--rule=harness` 对 `tests/support/mod.rs` 扫描禁用调用；命中时输出 `文件:行号` 并退出 1
  - 证据：`bash scripts/check-capture-offline.sh --rule=harness --self-test` 退出 1

### W-E-3 替身契约用例（P3，7 例）
- [ ] **W-E-3.1 E01 替身方法最小化（1 例）** · `crates/application/tests/capture_fakes.rs`
  - 追溯：AC-B2-OC-E01 · design 3.2.4
  - DoD：方法名黑名单扫描命中 0；`MemoryGapLog/MemorySwitchLog` 公开面仅含 append/query
  - 证据：`fakes_expose_minimal_method_surface`
- [ ] **W-E-3.2 E02 切换日志只追加（1 例）** · `crates/application/tests/capture_fakes.rs`
  - 追溯：AC-B2-OC-E02 · design 3.2.4
  - DoD：追加后长度 +1；同毫秒重复合并为 1 条；无修改入口
  - 证据：`switch_log_append_is_monotonic_and_same_ms_idempotent`
- [ ] **W-E-3.3 E03 缺口日志合并（1 例）** · `crates/application/tests/capture_fakes.rs`
  - 追溯：AC-B2-OC-E03 · design 3.2.4
  - DoD：两条间隔 <1000ms 区间 → 查询 1 条，保留最早起点与最晚终点，升序且不重叠
  - 证据：`gap_log_query_merges_adjacent_within_1s`
- [ ] **W-E-3.4 E04 投递计数（1 例）** · `crates/application/tests/capture_fakes.rs`
  - 追溯：AC-B2-OC-E04 · design 3.2.4
  - DoD：三类意图各投递 N 次 → 三类计数分别等于 N
  - 证据：`asr_delivery_counts_by_kind`
- [ ] **W-E-3.5 E04 审计无载荷（1 例）** · `crates/application/tests/capture_fakes.rs`
  - 追溯：AC-B2-OC-E04、D08 · design 3.2.4
  - DoD：审计条目字段白名单比对命中 0 个越界字段
  - 证据：`audit_event_has_no_payload`
- [ ] **W-E-3.6 E05 时钟单调（1 例）+ E13 替身零副作用（1 例）** · `crates/application/tests/capture_fakes.rs`
  - 追溯：AC-B2-OC-E05、AC-B2-OC-E13 · design 3.2.4
  - DoD：连续读取后者 >= 前者且不 panic；源码扫描网络/设备/凭据调用命中 0
  - 证据：`system_clock_fake_is_monotonic_non_decreasing`、`fakes_have_no_network_device_or_credential_calls`

### W-E-4 离线门禁用例（P3，3 例）
- [ ] **W-E-4.1 E06 装配可复用（1 例）** · `crates/application/tests/capture_offline_gate.rs`
  - 追溯：AC-B2-OC-E06 · design 3.2.4
  - DoD：经 `build_offline_capture` 建立的服务完成 attach → observe → 可用，用例通过；脚本统计复用文件 >= 4
  - 证据：`offline_harness_is_reusable_across_tests`
- [ ] **W-E-4.2 E07 建连为零（1 例）** · `crates/application/tests/capture_offline_gate.rs`
  - 追溯：AC-B2-OC-E07 · design 3.2.4
  - DoD：最小路径"装配 → 探测 → 可用"结束 `ConnectCounter::connections()==0`
  - 证据：`offline_min_path_has_zero_connections`
- [ ] **W-E-4.3 装配函数禁用调用（1 例）** · `crates/application/tests/capture_offline_gate.rs`
  - 追溯：AC-B2-OC-E06、AC-B2-OC-E09 · design 3.2.4
  - DoD：读取 `tests/support/mod.rs` 文本断言不含 `thread::sleep`/`Instant::now`/`SystemTime::now`
  - 证据：`offline_harness_forbidden_calls_scan`

### W-E-5 契约冻结用例（P4，7 例，跨包）
- [ ] **W-E-5.1 domain 契约 6 例** · `crates/application/tests/capture_contract_freeze.rs`
  - 追溯：AC-B2-OC-E10 · spec 4.5-1 · design 1.2.4
  - DoD：6 例分别断言 `SourceHealth` 3 变体、`Platform` 3 值、`ChannelPreference` 4 值、`Subscription` 4 方法可调用、四个仓储 trait 方法集合、`ErrorCode::ALL` 长度 12；取值与 spec 4.5 完全一致
  - 证据：`source_health_variants_frozen`、`platform_values_frozen`、`channel_preference_values_frozen`、`subscription_surface_frozen`、`repository_traits_frozen`、`error_code_all_len_frozen`
- [ ] **W-E-5.2 infrastructure 侧 `Metric::ALL` 1 例** · `crates/infrastructure/tests/capture_contract_freeze.rs`
  - 追溯：AC-B2-OC-E10 · design 1.2.4、D8、K5
  - DoD：`Metric::ALL.len()==10` 且 10 个 `name()` 互不重复；用例位于 infrastructure，避免应用层依赖违规
  - 证据：`metric_all_len_frozen`
- [ ] **W-E-5.3 跨包执行口径固化** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-E10 · design D8、K5
  - DoD：记录 `cargo test --workspace --test capture_contract_freeze` 一次执行同时收集两包 7 例；通过数 >= 7
  - 证据：`"$TC_BIN/cargo" test --workspace --test capture_contract_freeze`

### W-E-6 三类扫描脚本（P4）
- [ ] **W-E-6.1 `scripts/check-capture-offline.sh`** · `scripts/check-capture-offline.sh`
  - 追溯：AC-B2-OC-E01、AC-B2-OC-E06、AC-B2-OC-E08、AC-B2-OC-E09、AC-B2-OC-E13、AC-B2-OC-F04 · design 1.1.3-C、3.4
  - DoD：支持 `--rule=deps|source|fakes|harness|guard-deps|placeholders|payload` 与 `--self-test`；依赖闭包用 `cargo tree` 查询 domain/application，命中网络/数据库/TLS/平台/UI 依赖即退出 1；源码禁用调用（`thread::sleep`/`Instant::now`/`SystemTime::now`/网络/设备）命中即退出 1
  - 证据：`bash scripts/check-capture-offline.sh` 退出 0；`--self-test` 退出 1
- [ ] **W-E-6.2 `scripts/check-g2-claims.sh`** · `scripts/check-g2-claims.sh`
  - 追溯：AC-B2-OC-E12 · design 1.1.3-C
  - DoD：扫描面为 `docs/progress/**`、本轮新增源码、说明文档、`git log` 提交信息；排除 `.codeartsdoer/**` 与脚本自身；正常 0 命中退出 0，`--self-test` 植入禁用表述退出 1
  - 证据：`bash scripts/check-g2-claims.sh` 退出 0；`bash scripts/check-g2-claims.sh --self-test` 退出 1
- [ ] **W-E-6.3 `scripts/test-name-inventory.sh`** · `scripts/test-name-inventory.sh`
  - 追溯：AC-B2-OC-E11 · design 1.1.3-C
  - DoD：`cargo test -- --list` 归一化为 `pkg::target::name` 升序清单；`--diff <before>` 输出差集，非空退出 1
  - 证据：`bash scripts/test-name-inventory.sh --diff` 退出 0
- [ ] **W-E-6.4 脚本输出与退出码纪律** · `scripts/`
  - 追溯：spec 4.4-2、design 3.4
  - DoD：三个脚本失败时首行为 `文件:行号`；不打印凭据/端点/内容片段；退出码仅 `0/1/2`
  - 证据：`bash scripts/check-capture-offline.sh --self-test; echo $?` 输出 1 且首行含 `:`
- [ ] **W-E-6.5 `--self-test` 双向验证（正反各 1 次）** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-E12、E08 · design 3.4 判定纪律
  - DoD：记录两个脚本的"正常 0 / 植入 1"两次退出码；植入内容不落库（临时目录内完成）
  - 证据：`grep -c "self-test" docs/progress/b2-offline-closure.md` >= 2

### W-E-7 用例名清单冻结与差集（P4）
- [ ] **W-E-7.1 变更前清单落盘** · `docs/progress/test-names-before.txt`
  - 追溯：AC-B2-OC-E11 · design 1.1.2（E11 行）
  - DoD：文件为本轮改动前的归一化清单（基线 329 + 8 ignored 全含）；行数 >= 337；排序稳定（连续两次生成 diff 为空）
  - 证据：`bash scripts/test-name-inventory.sh > docs/progress/test-names-before.txt`
- [ ] **W-E-7.2 变更后差集为空** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-E11 · spec 4.5-2
  - DoD：`--diff` 仅含"新增"项、`removed` 为空；`capture_gap_merge` 两个原用例名出现在清单中
  - 证据：`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt`

---

## 批次 W-F · 总门禁与交付收口（AC-B2-OC-F01~F06、OC-01~05）

**目标**：一次全量 + 五类静态门禁 + 占位扫描全绿；验收记录五要素齐备，结论不虚报。
**阶段**：P5。**前置**：W-E 全部。

### W-F-1 全量测试与阈值（F01/F02）
- [ ] **W-F-1.1 全量测试达阈值** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-F01 · spec 9.2 X4 · design 3.3
  - DoD：`cargo test --workspace --no-fail-fast` 退出码 0、`failed=0`、`passed >= 396`（最低可接受 375，即最低新增 46；低于 360 直接判失败）
  - 证据：`"$TC_BIN/cargo" test --workspace --no-fail-fast`
- [ ] **W-F-1.2 忽略用例显式且标"未实测"** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-F02 · spec K7 · design 3.2.5
  - DoD：8 个真实环境用例（`llm_live*` 等）在默认执行下为 `ignored` 而非 `failed`；记录逐个用例名 + "未实测"标注 + 阻塞原因
  - 证据：`grep -c "未实测" docs/progress/b2-offline-closure.md` >= 8

### W-F-2 五类静态门禁与占位扫描（F03/F04）
- [ ] **W-F-2.1 `cargo fmt --all --check`** · 全工作区
  - 追溯：AC-B2-OC-F03 · design 3.4
  - DoD：退出码 0、无 diff 输出
  - 证据：`"$TC_BIN/cargo" fmt --all --check`
- [ ] **W-F-2.2 `cargo clippy --workspace --all-targets -- -D warnings`** · 全工作区
  - 追溯：AC-B2-OC-F03
  - DoD：退出码 0；不区分"可忽略"告警（spec 5.5.3-2）
  - 证据：`"$TC_BIN/cargo" clippy --workspace --all-targets -- -D warnings`
- [ ] **W-F-2.3 分层门禁** · `scripts/check-layering.sh`
  - 追溯：AC-B2-OC-F03、E10
  - DoD：退出码 0；`application` 不依赖 `infrastructure`/`platform-*`；`crates/domain` 无新增依赖
  - 证据：`bash scripts/check-layering.sh`
- [ ] **W-F-2.4 密钥扫描** · `scripts/scan-secrets.sh`
  - 追溯：AC-B2-OC-F03 · spec 4.3-2
  - DoD：退出码 0；命中清单为空；不回显敏感片段
  - 证据：`bash scripts/scan-secrets.sh`
- [ ] **W-F-2.5 许可证与来源检查** · `deny.toml`
  - 追溯：AC-B2-OC-F03 · design 3.4
  - DoD：`cargo deny --offline check licenses bans sources advisories` 退出码 0；新增依赖（无新增也应通过）许可证在白名单内
  - 证据：`"$TC_BIN/cargo" deny --offline check licenses bans sources advisories`
- [ ] **W-F-2.6 占位实现为零** · `scripts/check-capture-offline.sh --rule=placeholders`
  - 追溯：AC-B2-OC-F04 · spec 4.4-3
  - DoD：本轮新增源码中 `todo!`/`unimplemented!`/`panic!` 命中 0；命中时输出 `文件:行号` 并退出 1
  - 证据：`bash scripts/check-capture-offline.sh --rule=placeholders` 退出 0

### W-F-3 验收记录与结论（F05/F06、OC-01~05）
- [ ] **W-F-3.1 验收记录五要素齐备** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-F05 · spec 6.5
  - DoD：文档含 `基线数字 / 逐工作项三态 / 门禁结果 / 偏差清单 / 不宣称 G2 声明` 五节；每节非空
  - 证据：`grep -c "^## " docs/progress/b2-offline-closure.md` >= 5
- [ ] **W-F-3.2 逐项三态表（已判定/未触发/未实测）** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-F05、F06、OC-01 · spec 2 术语、6.5-2
  - DoD：行覆盖 `OC-01~05`、`C01~C09`、`D01~D10`、`E01~E13`、`F01~F06` 共 43 项；每行含 `编号 / 三态 / 用例名或命令 / 退出码 / 备注`；"未实测"行必含阻塞原因
  - 证据：`grep -c "已判定\|未触发\|未实测" docs/progress/b2-offline-closure.md` >= 43
- [ ] **W-F-3.3 偏差清单 D1~D11 登记** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-F05 · spec 9.3 · design 4.2
  - DoD：11 条偏差各含 `编号 / 内容 / 理由 / 是否改变对外行为`；D6/D7 明确"仅夹具与断言收紧、用例名保留"；D9 明确"白名单静态表在测试侧、域层零改动"
  - 证据：`grep -c "^| D[0-9]" docs/progress/b2-offline-closure.md` >= 11
- [ ] **W-F-3.4 不宣称声明与结论口径** · `docs/progress/b2-offline-closure.md`
  - 追溯：AC-B2-OC-F06 · spec 1.4-8、4.5、G2 宣称禁令
  - DoD：含"本轮通过 ≠ B2 G2 通过"与"p99 为逻辑上限、不代表真实 UI 线程时延"两段声明；结论仅取 `完成`/`部分完成`，存在未通过或未实测项时必须为"部分完成"并逐项列出
  - 证据：`bash scripts/check-g2-claims.sh` 退出 0；`grep -c "G2 通过\|逻辑上限" docs/progress/b2-offline-closure.md` >= 2

---

## 5. 依赖顺序与关键路径

### 5.1 P0~P5 顺序

```
W-00 (P0-pre) ── W-C-1 / W-C-2 (P0) ──┬── W-C-3 → W-C-4 → W-C-5 → W-C-6 (P1)
                                      └── W-D-1 → … → W-D-6 (P2，可与 P1 并行)
                                                    │
                              W-E-1 → W-E-2 → W-E-3 / W-E-4 (P3)
                                                    │
                              W-E-5 → W-E-6 → W-E-7 (P4)
                                                    │
                                        W-F-1 / W-F-2 → W-F-3 (P5)
```

### 5.2 关键路径（最长链）

`W-00-2.1` → `W-C-1.2/1.3` → `W-C-3.4` → `W-C-4.3` → `W-C-5.1/C03` → `W-E-1.1` → `W-E-2.1` → `W-E-5.1` → `W-E-6.1` → `W-F-1.1` → `W-F-3.2`

- 关键路径共 11 个节点，其中 `W-C-3.4`（合并→门限单点决策）与 `W-E-2.1`（装配函数）为两个不可压缩的收敛点。
- 次关键链：`W-D-1.3`（先快照后扇出）→ `W-D-6.2`；若 `W-D` 晚于 `W-C` 完成，则 P3 起点顺延，但因 `W-E` 同时依赖 P1/P2，实际总工期由**较晚者**决定。

### 5.3 并行与串行约束

| 可并行 | 必须串行 | 说明 |
|---|---|---|
| W-C-3 与 W-D-1（P0 之后） | W-C-4.2 ↔ W-D-5.x（同改 `service.rs`） | 两者合并时必须按 design 2.2.2-D 的 `observe` 顺序做一次整体复核，避免锁序反转（R9） |
| W-E-3 与 W-E-4（P3） | W-E-2 → W-E-3/E-4 | 替身与装配未完成前用例无法编译 |
| W-E-5 与 W-E-6（P4） | W-E-6.3 → W-E-7.1/7.2 | 清单脚本必须先可用 |
| W-F-2.1~2.6 六项 | W-F-1 → W-F-3 | 三态表必须引用全量与静态门禁的真实退出码 |

### 5.4 任务总量

| 统计项 | 数量 |
|---|---|
| 批次 | 5（W-00 / W-C / W-D / W-E / W-F） |
| 主任务 | 25 |
| 子任务 | 116 |
| 实施阶段 | 6（P0-pre、P0、P1、P2、P3、P4、P5，其中 P0-pre 为基线前置） |
| 新增测试用例 | 67（最低 46） |

---

## 6. 测试总量分解（329 → 396）

### 6.1 增量来源

| 组 | 文件 | 计划新增 | 最低新增 | 承载子任务 |
|---|---|---|---|---|
| L1 缺口纯函数 | `capture_gap_gate.rs`（新增） | 14 | 8 | W-C-1.2~1.5、1.8 |
| L1 诊断/策略 | `diagnostics.rs`、`types.rs` | 4 | 2 | W-C-1.1、1.6、1.9 |
| L1 发布/订阅内部 | `status_publisher.rs`、`subscriptions.rs` | 5 | 3 | W-D-1.2~1.5、D-3.1~3.5、D-4.1/4.2 |
| L2 失效保护 | `capture_degraded_guard.rs`（新增） | 12 | 9 | W-C-5.1~5.9 |
| L2 状态推送 | `capture_status_push.rs`（新增） | 13 | 10 | W-D-6.1~6.13 |
| L2 替身 | `capture_fakes.rs`（新增） | 7 | 5 | W-E-3.1~3.6 |
| L2 离线门禁 | `capture_offline_gate.rs`（新增） | 3 | 2 | W-E-4.1~4.3 |
| L3 契约冻结 | `capture_contract_freeze.rs` × 2（新增） | 7 | 7 | W-E-5.1、5.2 |
| L2 既有收紧 | `capture_gap_merge.rs`（修改） | 2 | 0 | W-C-4.6 |
| **合计** | — | **67** | **46** | — |

### 6.2 阈值核算

| 口径 | 计算 | 结果 | 判定 |
|---|---|---|---|
| 计划 | 329 + 67 | **396** | 满足 `>= 396`（AC-F01 目标值） |
| 最低 | 329 + 46 | **375** | 仍满足 `>= 360`（AC-F01 硬阈值） |
| 失败线 | 329 + 31 | 360 | 恰达阈值，**不得**作为交付目标 |
| 弱用例排除 | 每条用例名绑定 `AC-B2-OC-*` | — | 未绑定 AC 的用例不计入 396（spec R5） |

### 6.3 新增用例 → AC 反查

| AC | 用例所在的承载主任务 | 候选用例名（前缀见各子任务） |
|---|---|---|
| C01 | W-C-5.1、W-C-4.6、W-C-1.2 | `short_gap_500ms_is_not_persisted`、`default_policy_drops_short_merged_gap`、`short_gap_only_counted_not_persisted`、`gap_gate_persist_boundary_999_1000_1001` |
| C02 | W-C-5.2、W-C-1.2 | `long_gap_30s_persists_single_half_open_record`、`gap_gate_rejects_non_positive_interval` |
| C03 | W-C-5.3、W-C-1.3、W-C-4.6 | `two_short_gaps_merge_then_persist`、`merged_short_gaps_pass_gate`、`merge_for_persist_*`（3 例） |
| C04 | W-C-5.4、W-C-1.4、W-C-4.4 | `resume_from_is_close_minus_capped_duration`、`earlier_than_resume_window_produces_no_segment`、`resume_boundary_*`（3 例） |
| C05 | W-C-5.5、W-C-3.1、W-E-1.7 | `gap_300s_keeps_session_and_insight_ids`、`sentinel_repos_count_zero_writes_during_gap` |
| C06 | W-C-5.6、W-C-3.6 | `insight_read_failure_skips_check_without_delete` |
| C07 | W-C-5.7、W-C-6.1 | `single_channel_recovery_within_5s`、`availability_event_has_four_whitelisted_fields` |
| C08 | W-C-5.8、W-D-2.2 | `repeated_flapping_pushes_unavailable_once`、`first_push_is_true_only_for_gap_open` |
| C09 | W-C-5.9、W-C-1.5 | `intervals_do_not_overlap_and_touch_within_boundary`、`interval_violation_is_reported_for_overlap`、`validate_intervals_*`（4 例） |
| D01 | W-D-6.1 | `same_state_ten_times_pushes_once_and_updates_reason` |
| D02 | W-D-6.2 | `snapshot_visible_inside_subscriber_callback` |
| D03 | W-D-6.3、6.4 | `hundred_changes_share_one_increasing_sequence`、`channel_and_availability_share_one_sequence` |
| D04 | W-D-6.5 | `seq_near_upper_bound_is_rejected` |
| D05 | W-D-6.6、6.7 | `slow_subscriber_does_not_block_others`、`handler_error_only_counts` |
| D06 | W-D-6.8 | `released_subscriber_receives_zero_events` |
| D07 | W-D-6.9、W-D-4.2 | `latency_p99_and_max_within_5s`、`latency_p99_uses_expected_index` |
| D08 | W-D-6.10、6.11 | `payload_whitelist_field_counts`、`payload_scan_hits_no_text_or_sample` |
| D09 | W-D-6.12 | `no_subscriber_still_updates_snapshot` |
| D10 | W-D-6.13 | `reason_changes_inside_gap_do_not_repush` |
| E01 | W-E-3.1 | `fakes_expose_minimal_method_surface` |
| E02 | W-E-3.2 | `switch_log_append_is_monotonic_and_same_ms_idempotent` |
| E03 | W-E-3.3 | `gap_log_query_merges_adjacent_within_1s` |
| E04 | W-E-3.4、3.5 | `asr_delivery_counts_by_kind`、`audit_event_has_no_payload` |
| E05 | W-E-3.6 | `system_clock_fake_is_monotonic_non_decreasing` |
| E06 | W-E-4.1、4.3 | `offline_harness_is_reusable_across_tests`、`offline_harness_forbidden_calls_scan` |
| E07 | W-E-4.2 | `offline_min_path_has_zero_connections` |
| E10 | W-E-5.1、5.2 | `capture_contract_freeze` 7 例 |
| E13 | W-E-3.6 | `fakes_have_no_network_device_or_credential_calls` |
| E08/E09/E11/E12 | W-E-6.x、7.x | 脚本退出码与差集（无新增用例，L4 判定） |
| F01~F06 | W-F-1~3 | 命令退出码 + 文档三态（L4/L5 判定） |

---

## 7. 可离线完成性判定

### 7.1 当前 Windows 单机、断网、无设备即可完成（本轮全部工作量）

| 范围 | 子任务 | 备注 |
|---|---|---|
| 基线校核与记录 | W-00 全部（6） | 仅需本地 cargo 与 git |
| 纯函数与策略 | W-C-1 全部（9）、W-C-2（3） | 无外部依赖 |
| 守卫与接线 | W-C-3（7）、W-C-4（6）、W-C-5（9）、W-C-6（2） | 全部注入 `ManualClock` + 内存替身 |
| 发布与订阅 | W-D 全部（31） | 无 tokio、无线程，显式 `drain_status(now)` 推进 |
| 替身与装配 | W-E-1（9）、W-E-2（3）、W-E-3（6）、W-E-4（3） | 替身只做追加/合并/计数/单调时钟 |
| 契约冻结 | W-E-5（3） | `Metric::ALL` 跨包执行需一次性构建两个包 |
| 脚本门禁 | W-E-6（5）、W-E-7（2） | git-bash 可执行；`cargo tree` 离线可用 |
| 总门禁 | W-F-1（2）、W-F-2（6）、W-F-3（4） | `cargo deny --offline` 需本地 registry 缓存完备 |

合计 **116 / 116 子任务** 在本机离线可完成（`cargo deny --offline` 与 `cargo tree` 依赖本地缓存，属操作前提而非外部端点）。

### 7.2 需要真机或真实端点（本轮**不排**，仅保持"未实测"）

| 项 | 为何不能离线判定 | 本轮处理 |
|---|---|---|
| `llm_live*` 8 个 ignored 用例 | 需真实 LLM 端点与凭据 | W-F-1.2 登记"未实测"+ 阻塞原因；不执行 |
| B2.15/16 真实音频捕获 | 需 WASAPI/SCStream 真机与 30 分钟会议 | 不做（spec 7.2-1） |
| B2.17~B2.21 字幕读取与 OCR | 需三平台窗口、遮挡场景、≥100 张截图 | 不做（spec 7.2-2） |
| B2.22 版本矩阵真实读取 | 需安装三平台客户端，macOS 缺失 | 不做（spec 7.2-3） |
| B2.24 双通道去重合并 | 需 ≥200 对真实语料与独立规格 | 不做（spec 7.2-4） |
| B2.26/27 会话六状态机与告知同意 | 涉及 UI 与持久化 | 不做（spec 7.2-5） |
| B5 UI 推送时延 | 需真实 UI 线程 | 不做；D07 仅证明逻辑上限（R1） |
| B3 理解层 | 依赖合并后片段 | 不做（spec 7.2-7） |
| AC-LLM-23/24/25、13.4、13.3、12.1/12.3 | 需真实端点/门控/B4/CMake 工具链 | 保持"未实测"（spec 7.2-8~11） |
| macOS 编译、GitHub 推送与 CI 触发 | 无 mac 设备、网络不可达 | 不做（spec 7.2-12/13） |

### 7.3 三态填写纪律

- `已判定`：必须同时给出用例名（或命令）与退出码（AC-B2-OC-01）。
- `未触发`：能力存在但场景未出现（如备份/回补预算未用尽），须写明未触发条件。
- `未实测`：必须写阻塞原因与所需环境，禁止以离线结果替代（spec 2 术语、6.5-2）。
- 任一项未通过或未实测时，整体结论只能写"部分完成"（AC-B2-OC-F06）。

---

## 8. 交付边界

1. 本清单**只覆盖** W-00 / W-C / W-D / W-E / W-F；B2.15~22、B2.24、B2.26/27、B3、B5、AC-LLM-23/24/25 与 13.4 均不在本轮。
2. `docs/sdd/tasks.md`、`docs/sdd/spec.md`、`docs/sdd/design.md`、`.codeartsdoer/specs/capture_channel_state/**` 只读，不改判定口径（spec 9.4-3）。
3. `crates/domain/**` 零改动（K2）；如需迁移端口到域层，须另立规格。
4. 本轮产出不得出现"三平台稳定运行已验证""真实 5 秒回退已实测""真实去重已验证"等结论性表述（spec G2 宣称禁令、AC-B2-OC-E12）。