# B4 门控 · P0 进度与取证（W-B4-2 ~ W-B4-8）

## §1 阶段与基线

- 基线提交：`HEAD=ade9111`（W-B4-1 码表已交付，本阶段只追加不重写）。
- 本阶段范围：W-B4-2 ~ W-B4-8，共 7 个主任务；P1~P3 不在本轮范围。
- 平台口径：Windows 单平台；macOS 不适用（无设备）。
- 代码落点：`crates/domain/src/model/gate.rs`、`gate_replay.rs`、`crates/application/src/gate/**`、`scripts/check-b4-gates.sh`。
- 需求/设计基线：`.codeartsdoer/specs/b4_gate/{spec,requirements,design,tasks}.md`，编码阶段未改写其中 spec/requirements。

## §2 三条硬判据快照

日期：2026-11-22。命令均在项目根 `TSC/` 下执行。

| 编号 | 命令 | 期望 | 实测 |
|---|---|---|---|
| E-01 | `cargo test -p meeting-copilot-domain` | 0 | 0 |
| E-02 | `bash scripts/check-b4-gates.sh --rule=record` | 0 | 0 |
| E-03 | `bash scripts/check-b4-gates.sh --rule=budget` | 0 | 0 |

串联命令：`cargo test -p meeting-copilot-domain && bash scripts/check-b4-gates.sh --rule=record && bash scripts/check-b4-gates.sh --rule=budget`，退出码 0。

## §3 旁证与规则矩阵

| 规则 | 期望 | 实测 | 备注 |
|---|---|---|---|
| `--rule=source` | 0 | 0 | 输出含 `SCAN-ABSENT: crates/domain/src/gap` |
| `--rule=outbound` | 0 | 0 | 输出含 `SCAN-ABSENT: crates/application/src/proactive` |
| `--rule=zero-transcript` | 0 | 0 | 登记 `SCAN-ABSENT: crates/application/tests/gate_*.rs`（P3 前无门控测试） |
| `--rule=doc-plan` | 0 | 0 | spec 第 8 章表格逐行可定位 |
| `--rule=targets` | 1 | 1 | 输出恰 6 行 `TARGETS-MISSING: gate_*`（D-B4-D07） |
| `--self-test` | 1 | 1 | 反例 4 类均被击穿；临时目录隔离，工作区前后行数一致 |

零改动矩阵摘要：

- `crates/application/src/understand/store.rs` 与 `deduplicator.rs`：零 diff。
- `Cargo.toml` / `Cargo.lock`：零 diff，零新增依赖。
- `crates/application/Cargo.toml`：无 `serde_json`。
- 既有 7 个门禁脚本与 `test-name-inventory.sh`：零 diff。
- `cargo test --workspace` 保持 `failed=0`、`ignored=9`。
- `cargo fmt --check` / `cargo clippy -D warnings` / `check-layering.sh` / `scan-secrets.sh` / `check-b3-gates.sh`：全绿。

## §4 偏差登记

| 编号 | 内容 | 处置 |
|---|---|---|
| D-B4-D02 | `crates/domain/src/gap/`、`crates/application/src/proactive/` 本轮不存在 | 脚本打印 `SCAN-ABSENT` 只登记，不判失败、不退 2 |
| D-B4-D04 | 12 项历史码降级为别名，3 处改名 + 1 处改 `LowConfidence` | W-B4-1 已交付；新代码中旧名 0 命中 |
| R-01 | `gate.rs` / `gate_replay.rs` 必须扁平落位 | 已按文件落位，未建同名目录 |
| D-B4-D05 | `GateInput`/`GateSnapshot` 各增 `candidate_ref`、`insight_ref`、`trace_id` | 已落 16 字段；`record_from` 据此产出 14 字段记录 |
| D-B4-D06 | `budget_remaining` 上限由 120 修正为 600 | 与 `session_outbound_cap ∈ 0..=600` 自洽；默认 cap=120 行为不变 |
| D-B4-D07 | `--rule=targets` 在 P0 判退 1 且 6 项可枚举 | 不预建空 target，避免假绿；P1 起逐项转绿 |
| D-B4-D08 | `EntityKey::parse -> Option<Self>`；结构违规用谓词方法 | 31 项封闭数不变，不新增 domain 错误枚举 |
| D-B4-D09 | 7 个骨架文件在 P0 一次性创建 | 满足 `mod x;` 编译前提，各 ≤5 行 |
| D-B4-D10 | P0 用内部 `RecordViolation`，不落 `GateError` | P3 升格时函数签名不变 |
| D-B4-D11 | `GateCounter` 32 变体逐项显式 `serde(rename)` | 避免字母-数字边界漂移 |
| D-B4-D12 | `zero-transcript` 的 tests 面收窄为 `crates/application/tests/gate_*.rs` | 既有非门控测试含历史样例，不在本阶段改写；P3 新建 gate 测试后自动纳入 |
| D-B4-D13 | `--self-test` 的工作区断言由"绝对 0 行"改为"自检前后行数一致" | 未提交工作区下 7 行新增属本阶段正常改动；隔离语义不变，提交后基线复跑前后均为 0 |
| D-B4-D14 | `GapScanInput` 按草案落 11 字段，非设计文本所记 12 项 | 以草案字段清单为准；`question_wait_ms` 固定常量 `10_000`，未新增未定义字段 |
| D-B4-P01 | `GateInput`/`GateSnapshot` 16 → 23 字段，补 `session_state`/`silent_period`/`screen_share`/`consent_confirmed`/`auto_suggestion_enabled`/`session_suppressed`/`duplicate_detected` | G1 十一硬闸门需会话级状态，草案缺字段；与 `GateInputFixture` 同名同义 |
| D-B4-P02 | 公开诊断入口 `gate::detect_gaps_with_counters`（`prefilter.rs`） | 回放面仍只用 4 函数；`api.rs` 的 `Pub fn` 计数保持 4，不进入回放面 |
| D-B4-P03 | `evaluate_gate` 不对外返回 `GateCounters`；P1 计数断言经诊断入口完成 | P2 起由装配方持有计数，`bump_by` 在判定期调用 |
| D-B4-P04 | `evaluate_gate` 对"G1 有候选"暂返回 `Deny{PolicyUnavailable}` | G2~G5 未实现时 fail-closed，P2 接入 G2 后替换 |
| D-B4-P05 | W-B4-9.6 `from_fixture` 延后至 W-B4-28 | `GateInputFixture` 不持有 `Insight`/`NormalizedFragment` 本体，P1 无法零拷贝借用 |
| D-B4-P06 | W-B4-10.2/10.3 的 `--test gate_replay` 证据暂由 lib 单测覆盖 | `fixed_clock_is_deterministic_sequence`、`mirror_clock_is_monotonic` 均在 lib；target 与 W-B4-28 同批创建 |
| D-B4-P07 | `GateInput`/`GateSnapshot` 再增 `signals: GateSignals`（24 字段） | 承载 G2/G3/G4 的只读信号（relevance、similarity、duplicate_last、cluster、entities、suppression、cache）；不再扩张独立仓储 |
| D-B4-P08 | 公开诊断入口 `evaluate_gate_with_counters` + `GateOutcome` 落在 `gate/diagnostics.rs` | 4 个纯函数仍在 `api.rs`，`grep -c '^pub fn ' = 4` 不变 |
| D-B4-P09 | P2 的 `Allow` 不经 G5；仅第 4 并发产出 `queued=true` 的意图骨架 | G5 在 P3 接入；`outbound=None` 为 P2 常规放行，queued 意图用于 AC-27 可观测断言 |
| D-B4-P10 | `OutboundLease` 以别名 `LeaseTable` 导出 | 避免 `pub use .*OutboundLease` 触发回放面扩散断言（该断言仍为 0） |
| D-B4-P11 | AC-17 的"非法去重键 → `PolicyUnavailable`"未单测 | G1 派生的键恒为 32 hex，P2 无注入非法键的入口；P3 回放可覆盖 |
| D-B4-P12 | 全量 `cargo test --workspace` 偶发失败于既有性能用例 `thousand_pairs_under_200ms` | 本轮实测 219/221 ms（预算 200 ms），单 target 复跑 190 ms 通过；domain 代码零改动，属机器负载波动 |

## §5 未判定清单

| 项 | 状态 |
|---|---|
| G1 预筛 | 已实现（AC-B4-01~08，20 条用例全绿） |
| G2 相关性 | 已实现（AC-B4-09~14，11 条用例） |
| G3 时效/去重 | 已实现（AC-B4-15~21，15 条用例） |
| G4 预算与冷却 | 已实现（AC-B4-24~27，含租约 3 条；真机性能仍未判定） |
| 13.3 回放本体 | 未做 |
| 38 / 39 / 41.5 / 42 / 43 / 51 / 52 实体能力 | 未做 |
| 真实检索源连通 / 真实 LLM 冷却 | 未判定（仍为既有 9 个 ignored） |

## §6 复跑方法

环境前置（每个 shell 会话一次）：

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
```

三条硬判据串联：

```bash
"$CARGO" test -p meeting-copilot-domain \
  && bash scripts/check-b4-gates.sh --rule=record \
  && bash scripts/check-b4-gates.sh --rule=budget; echo "exit=$?"
```

七条规则逐条：

```bash
for r in source budget record outbound zero-transcript doc-plan; do
  bash scripts/check-b4-gates.sh --rule=$r; echo "$r=$?";
done
bash scripts/check-b4-gates.sh --rule=targets; echo "targets=$?"
bash scripts/check-b4-gates.sh --self-test; echo "self-test=$?"
```

零改动校验：

```bash
git diff --quiet HEAD -- crates/application/src/understand/store.rs crates/application/src/understand/deduplicator.rs; echo "store=$?"
git diff --stat HEAD -- Cargo.toml Cargo.lock | wc -l
"$CARGO" test --workspace -- --list --ignored 2>/dev/null | grep -c ": test"
```
## §7 P1 快照（W-B4-9 ~ W-B4-14）

日期：2026-11-22。

| 项 | 命令 | 结果 |
|---|---|---|
| G1 用例 | `cargo test -p meeting-copilot-application --test gate_prefilter` | 20 passed / 0 failed |
| 全量回归 | `cargo test --workspace` | 636 passed / 0 failed / 9 ignored，退出 0 |
| 七规则 | `--rule=source/budget/record/outbound/zero-transcript/doc-plan` | 6 × 0 |
| targets | `--rule=targets` | 退 1，仅剩 5 项（`gate_prefilter` 已登记，D-B4-D07 口径） |
| 回放面 | `grep -c '^pub fn ' gate/api.rs` | 4 |
| 纯性 | `--rule=source` | 0（`gate/**` 无网络/计时/睡眠/env/panic/锁） |
| B3 输入边界 | `grep -r "InsightDraft\|InsightStore" gate/` | 0 |
| 格式/静态 | `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings` | 0 |

P1 已覆盖：六类规则与基础分、机构映射、缩写白名单、首现/敏感/10s/2s/0.50 边界、11 项硬闸门优先级与短路、0 候选 Allow 路径、非法策略 fail-closed、`MirrorClock` 单调推进。
P1 未覆盖（登记见 §4）：`from_fixture`（D-B4-P05）、`gate_replay` target（D-B4-P06）、G2~G5。
## §8 P2 快照（W-B4-15 ~ W-B4-23）

日期：2026-11-22。

| 项 | 命令 | 结果 |
|---|---|---|
| G2 | `cargo test -p meeting-copilot-application --test gate_relevance` | 11 passed / 0 failed |
| G3 | `cargo test -p meeting-copilot-application --test gate_dedupe` | 15 passed / 0 failed |
| G4 + 租约 | `cargo test -p meeting-copilot-application --test gate_budget` | 14 passed / 0 failed |
| G1 回归 | `cargo test -p meeting-copilot-application --test gate_prefilter` | 20 passed / 0 failed |
| 七规则 | `--rule=source/budget/record/outbound/zero-transcript/doc-plan` | 6 × 0 |
| targets | `--rule=targets` | 退 1，仅剩 `gate_outbound`/`gate_replay` 2 项（D-B4-D07 口径） |
| 回放面 | `grep -c '^pub fn ' gate/api.rs` | 4（诊断入口已移至 `diagnostics.rs`） |
| 纯性/预算 | `--rule=source`、`--rule=budget` | 0；`gate/**` 中 `LLM_`/`std::env` 命中 0 |
| 格式/静态 | `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings` | 0 |

P2 已覆盖：400/250/200/150 加权与整数衰减（180 s = 367‰）、620/619 阈值边界、越界钳制、六分项；聚类 880‰、10 分钟去重键、主题 ≤2/全局 ≤3、实体 5 分钟冷却、抑制（自动拒/按需提示）、24h 缓存、11→3 收敛、`dedupe_enabled` 只关四步；会话 120 上限、令牌桶 3000/档位 1/3/5、冷却/离线/Unknown/第 4 并发排队、600 001 ms 恢复；3 槽租约+单排队位+队首提升。
P2 未覆盖：G5 授权/审计（P3）；`gate_outbound`/`gate_replay` target；回放 200 例。