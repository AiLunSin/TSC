# B4 门控 · P0 进度与取证（W-B4-2 ~ W-B4-8）

## §1 阶段与基线

- 基线提交：`HEAD=ade9111`（W-B4-1 码表已交付，本阶段只追加不重写）。
- 本阶段范围：W-B4-2 ~ W-B4-8，共 7 个主任务；P1~P3 不在本轮范围。
- 平台口径：Windows 单平台；macOS 不适用（无设备）。
- 代码落点：`crates/domain/src/model/gate.rs`、`gate_replay.rs`、`crates/application/src/gate/**`、`scripts/check-b4-gates.sh`。
- 需求/设计基线：`.codeartsdoer/specs/b4_gate/{spec,requirements,design,tasks}.md`，P0~P3 编码阶段未改写其中 spec/requirements；**2026-11-22 增量需求修订**新增 AC-B4-51~57（可点击关键词卡片契约）与 AC-B4-48 方案 A 口径，既有 AC-B4-01~50 编号与语义不变。

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
| AC-B4-48 200 例回放 | closed（§11，W-B4-41；`replay_200_cases_*` 已交付） |
| AC-B4-51~57 关键词卡片契约 | closed（§11，W-B4-34~40） |
| AC-B4-58~64 39 引用对齐校验 | 规格已定、待实现（§12；AC-B4-58~64 为 `pending`；不建 39 实体） |
| 13.3 回放本体 | 未做 |
| 38 / 39 / 41.5 / 42 / 43 / 51 / 52 实体能力 | 未做（38/39/51 与关键词卡片的边界已在规格中定义） |
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
## §9 P3 快照与 AC 三态（W-B4-24 ~ W-B4-33；既有 50 + 增量 7，共 57 条）

日期：2026-11-22。

| 项 | 命令 | 结果 |
|---|---|---|
| G5 | `cargo test -p meeting-copilot-application --test gate_outbound` | 16 passed / 0 failed |
| 回放面 | `cargo test -p meeting-copilot-application --test gate_replay` | 3 passed / 0 failed |
| 六 target | 逐个 `--test gate_{prefilter,relevance,dedupe,budget,outbound,replay}` | 全 0 |
| 回放面函数 | `grep -c '^pub fn ' gate/api.rs` | 4 |
| targets | `--rule=targets` | 19 项，退 0 |
| 记录/文档 | `--rule=record`、`--rule=doc-plan` | 0 |

AC 三态（`passed` / `not-applicable` / `pending`）：

| AC | 状态 | 证据 |
|---|---|---|
| AC-B4-01 | passed | `prefilter_scores_six_gap_rules`、`acronym_whitelist_is_configurable` |
| AC-B4-02 | passed | `first_seen_suppresses_second_candidate`、`sensitive_term_is_not_a_seed` |
| AC-B4-03 | passed | `unanswered_question_yields_after_ten_seconds`、`answered_question_is_skipped` |
| AC-B4-04 | passed | `below_half_gap_confidence_yields_no_candidate`、`exactly_half_gap_confidence_passes` |
| AC-B4-05 | passed | `candidate_is_ready_within_two_seconds_of_stable_prefix` |
| AC-B4-06 | passed | `gap_kind_is_closed_enum_and_institution_maps_to_proper_noun`、`dedupe_key_is_32_hex` |
| AC-B4-07 | passed | `prefilter_is_pure_and_deterministic`、`--rule=source` |
| AC-B4-08 | passed | `prefilter_denies_eleven_hard_cases`、`deny_short_circuits_later_stages` |
| AC-B4-09 | passed | `relevance_weights_match_spec`、`relevance_full_score_is_thousand` |
| AC-B4-10 | passed | `time_decay_at_180_seconds_is_367_milli` |
| AC-B4-11 | passed | `threshold_profiles_are_072_062_052`、`balanced_threshold_boundary` |
| AC-B4-12 | passed | `relevance_parts_are_all_reported` |
| AC-B4-13 | passed | `low_relevance_is_denied`、`empty_topic_is_deterministic` |
| AC-B4-14 | passed | `relevance_never_calls_llm_or_network`、`relevance_is_deterministic_for_mixed_text` |
| AC-B4-15 | passed | `semantic_cluster_threshold_is_088`、`cluster_boundary_08799_08800` |
| AC-B4-16 | passed | `duplicate_key_blocks_within_ten_minutes`、`duplicate_key_expires_at_ten_minutes` |
| AC-B4-17 | passed | `topic_quota_is_two_per_ten_minutes`、`global_quota_is_three_per_ten_minutes` |
| AC-B4-18 | passed | `entity_cooldown_is_five_minutes`、`entity_cooldown_expires_at_five_minutes` |
| AC-B4-19 | passed | `suppression_blocks_auto_allows_on_demand`、`dedupe_switch_keeps_suppression_and_cache` |
| AC-B4-20 | passed | `fresh_cache_is_allowed`、`stale_cache_is_denied` |
| AC-B4-21 | passed | `candidate_convergence_keeps_top_three` |
| AC-B4-22 | passed | `budget_is_independent_of_llm_live_cap`、`--rule=budget` |
| AC-B4-23 | passed | `session_outbound_cap_is_120`、`commit_advances_only_on_allow` |
| AC-B4-24 | passed | `token_bucket_capacity_and_refill`、`frequency_profiles_are_1_3_5` |
| AC-B4-25 | passed | `gateway_cooling_denies_without_retry` |
| AC-B4-26 | passed | `gateway_cooling_does_not_block_admission` |
| AC-B4-27 | passed | `budget_recovers_after_window`、`offline_and_unknown_network_paths`、`fourth_concurrent_request_is_queued`、`lease` 三条 |
| AC-B4-28 | passed | `authorization_records_seven_required_fields` |
| AC-B4-29 | passed | `unauthorized_outbound_is_zero` |
| AC-B4-30 | passed | `revoked_authorization_is_not_auto_restored`、`reauthorization_bumps_notice_version` |
| AC-B4-31 | passed | `auto_query_is_capped_at_80`、`sanitizer_receives_truncated_input_only`、`oversize_never_reaches_sanitizer` |
| AC-B4-32 | passed | `authorization_change_invalidates_old_grant` |
| AC-B4-33 | passed | `--rule=outbound`、`grep -rln` 文件数 1 |
| AC-B4-34 | passed | `outbound_audit_has_required_fields_without_text`、`audit_is_appended_before_release` |
| AC-B4-35 | passed | `audit_unavailable_blocks_outbound`、`audit_append_error_blocks_outbound` |
| AC-B4-36 | passed | `sanitize_failure_blocks_outbound` |
| AC-B4-37 | passed | `gate_consumes_insights_readonly`、understand 目录零 diff |
| AC-B4-38 | passed | `gate_input_has_no_insight_draft`、`InsightDraft`/`InsightStore` 0 命中 |
| AC-B4-39 | passed | 全量回归 0 failed；六 target 全绿；`removed=0` |
| AC-B4-40 | passed | 5 常量各 1 处；`GapKind` 6 变体 |
| AC-B4-41 | passed | `Cargo.toml`/`Cargo.lock` 零 diff；`cargo deny` 四项 |
| AC-B4-42 | passed | `scan-secrets.sh`、`--rule=zero-transcript`、`scan-secrets-docs.sh --all-md` |
| AC-B4-43 | passed | `--rule=source` 退 0；9 个 `#[ignore]` 不变 |
| AC-B4-44 | passed | `--list --ignored` = 9；`check-g2-claims.sh` = 0 |
| AC-B4-45 | not-applicable | macOS 无设备（D-B4-04） |
| AC-B4-46 | passed | `replay_interface_is_io_free`；`api.rs` 4 个 `pub fn` |
| AC-B4-47 | passed | `--rule=record`；`--self-test` 反例 |
| AC-B4-48 | passed | 方案 A（D-B4-P13）：`gate_replay` 的 `replay_200_cases_is_byte_identical`、`replay_200_cases_cover_all_reject_reasons`；`total=200`/`mismatched=0`/`seed=7` |
| AC-B4-49 | passed | `tests/replay/` 不存在；禁符号 0 命中 |
| AC-B4-50 | passed | `--rule=doc-plan`；本表 57 行 |
| AC-B4-51 | passed | 见 §11：`keyword_candidates_cover_five_kinds`、`keyword_candidate_keeps_first_seen_only`、`unanswered_question_is_not_a_keyword_card` |
| AC-B4-52 | passed | 见 §11：`keyword_card_requires_gate_allow`、`deny_never_emits_clickable_card` |
| AC-B4-53 | passed | 见 §11：`keyword_card_has_thirteen_fields`、`keyword_reference_view_has_five_fields` |
| AC-B4-54 | passed | 见 §11：`clicking_keyword_card_is_local_only`、`continue_search_reenters_gate` |
| AC-B4-55 | passed | 见 §11：`click_permission_is_fail_closed`、`keyword_card_hidden_under_screen_share` |
| AC-B4-56 | passed | 见 §11：`keyword_card_zero_transcript_boundary`、`--rule=zero-transcript` |
| AC-B4-57 | passed | 见 §11：`--rule=source`、`tests/replay/` 不存在 |

> 结论口径：**离线通过 ≠ G4 达标、替身通过 ≠ 真实端点已验证**。AC-B4-48 与 51~57 已由 §11（2026-11-22 P4 快照）闭合为 `passed`；39 的 AC-B4-58~64 为 2026-11-22 需求增量，**规格已定、待实现**（见 §12），故 39 增量范围尚未闭合，既有 P0~P4 出口不受影响。

新增偏差：

| 编号 | 内容 | 处置 |
|---|---|---|
| D-B4-D01 | `GapKind` 缺"机构/职务" | `GapRule::Institution` → `proper_noun`；`GapKind` 保持 6 变体 |
| D-B4-D03 | 脱敏/审计替身不参与决策 | P3 用 `PassthroughSanitizer`/`NoopAuditSink` 作为装配默认；失败分支由 `gate_outbound` 替身覆盖 |
| D-B4-P13 | AC-48 的 200 例回放未实现 | **已裁决方案 A（2026-11-22）**：不改 `GateInputFixture`、不写原文；在 `crates/application/tests/gate_replay.rs` 内用本地构造的 `NormalizedFragment`/`Insight` 驱动 200 例；`total=200`、`mismatched=0`、`seed=7`、两次逐字节一致；实现仍为 pending |

| D-B4-P14 | `GateInput`/`GateSnapshot` 再增 `sanitizer`/`audit_sink`/`query_seed` | 端口只能经输入注入以保持 4 函数签名不变 |
| D-B4-P15 | 审计失败缓冲仅内部持有，未对外暴露 | `AuthorizeOutcome.buffered` 私有；测试以 Deny + 计数 + `append` 未调用替代 |
| D-B4-P16 | 审计端口在门面命名为 `AuditSink`（非 `GateAuditSink`） | 保持回放面"符号 0 命中"断言 |
| D-B4-P17 | `is_grantable` 只表达"撤销不可自恢复"；启动/升级/会话/策略四入口由装配方接入 | P3 不新增平台入口，避免越界实现 43/52 |
| D-B4-P18 | 关键词卡片仅定义契约，未实现 | 归属：B4 契约扩展 + 38 合成 + 39 校验 + 51 UI；AC-B4-51~57 为 pending；不创建卡片实体文件、不改 `GateRecord` 14 字段 |

## §10 未完成事项汇总（2026-11-22 增量裁决）

> 本节是当前"未完成项"的唯一汇总口径；§5 是分类视图，§9 是 AC 三态明细。**本轮只更新需求规格与进度登记，未改代码、未提交。**

### §10.1 阻断当前出口的未完成项

| 编号 | 未完成项 | 当前口径 | 归属 / 下一步 |
|---|---|---|---|
| U-01 | AC-B4-48 200 例回放 | **closed（W-B4-41）**：不改 `GateInputFixture`、不写原文；`gate_replay.rs` 内本地构造 `NormalizedFragment`/`Insight`；`total=200`、`mismatched=0`、`seed=7`、两次逐字节一致 | B4 测试面 / W-B4-28.3、28.4 |
| U-02 | AC-B4-51~57 关键词卡片契约 | **closed（W-B4-34~40）**：五类候选、首见唯一、13 项卡片字段、只有 `Allow` 可出卡/可点击、点击本地查看、零转录、归属拆分 | B4 契约扩展；实现任务待 tasks.md 增量 |
| U-03 | 38 卡片合成 | **规格已定，待实现（2026-11-22）**：`spec.md` §5.8/§6.16~6.18/§7.13 与 `requirements.md` §12 已定义 38 的 7 项输入、11 项可展示字段、6 项拒绝原因、固定模板解释、`Unknown→Unverified` 唯一回填、离线/零转录/归属边界与 AC-B4-65~74；尚未创建 38 实体文件，AC-B4-65~74 为 `pending` | 38（`crates/application/src/cards/**` + `card_compose` target；不建 UI/存储/回放目录、不新增 domain 契约） |
| U-04 | 39 引用对齐校验 | **规格已定，待实现（2026-11-22）**：`spec.md` §5.7/§6.14~6.15/§7.12 与 `requirements.md` §11 已定义 `Passed`/`Failed`/`Unknown` 三态（`Unknown→Unverified` 唯一投影）、引用只存 ID+4 档信任+2 态可访问+发布时间、零转录/离线/确定性边界与 AC-B4-58~64；尚未创建 39 实体文件，AC-B4-58~64 为 `pending` | 39（domain 契约 `citation.rs` + application 只读校验 `citation/align.rs` + `citation_align` target；不建 UI/合成/回放目录） |
| U-05 | 51 点击 UI 与设置向导 | **规格已定，待实现（2026-11-22）**：点击 UI——`spec.md` §5.9/§6.19~6.23/§7.14 与 `requirements.md` §13 已定义视图模型 10/4/5/6/2/2 字段、点击只降不升、投屏/遮蔽正文隐藏、撤回不复活、零转录/平台边界与 AC-B4-75~84；设置向导——`spec.md` §5.10/§6.24~6.28/§7.15 与 `requirements.md` §14 已定义 `SettingsView` 8 项、`SettingsFacts` 6 项、`SettingsInput` 5 项、固定无载荷掩码、一步式默认（只填 Key 即可测）、模型预设+自定义（≤128 字符）、端点仅注入清单下标（不扩白名单）、单飞测试状态机 6 态/5 事件、8 类失败人话化、明文不出内存与 AC-B4-85~94；尚未创建 51 实体文件，AC-B4-75~94 为 `pending` | 51（`crates/ui-bridge/src/keyword_card/**` + `crates/ui-bridge/src/settings/**` + `keyword_card_view`/`llm_settings_view` 两个 target；不建平台适配/52 存储/回放目录；settings 不新增依赖） |
| U-06 | 13.3 回放本体 | 未做；`tests/replay/` 仍不得创建 | 13.3 |
| U-07 | 41.5 / 42 / 43 / 52 | 未做；负反馈、缓存/outbox、按需补全实体、审计哈希链存储 | 对应章节 |
| U-08 | 真实端点 / 真机 | 未判定；真实检索源、真实 LLM 冷却、6s/10s p95、误触发率 | 保持 9 个 `#[ignore]`，不宣称 G4 |

### §10.2 口径约束

1. **AC 编号**：既有 AC-B4-01~50 不重排、不改语义；新增 AC-B4-51~57 只覆盖"契约与前置"，不判定 38/39/51 的实现质量。
2. **零转录**：`GateRecord` 仍 14 字段；`keyword_text`/`summary` 只能存在于卡片展示层与测试进程内，不得进入记录、审计、回放夹具、报告、日志或本文件。
3. **不越界**：B4 不新增卡片合成器、引用校验器、展示器、卡片存储；`gate/**` 中相关符号命中必须为 0。
4. **点击不等于外发**：点击只读本地详情，`outbound_attempts=0`、`llm_calls=0`；"继续检索"必须重新走 G1~G5。
5. **失败闭合**：门控拒绝、引用未对齐、授权撤销、投屏/遮蔽、跨会话/引用过期，一律 `click_state=Disabled{reason}`，不得展示可点击的 `Deny` 卡。
6. **39 引用对齐（2026-11-22 增量）**：39 只输出 `Passed`/`Failed`/`Unknown`（`Unknown→Unverified` **唯一投影**），引用只存 `reference_id`/`trust_tier`/`access_status`/`published_at_ms`/`alignment`；不联网抓正文、不写门控记录；AC-B4-58~64 为 `pending`；不改既有 AC 01~57 与任何封闭字段数。
7. **38 卡片合成（2026-11-22 增量）**：38 只做 7 项输入的确定性组装，输出 11 项可展示字段；`gate_decision_ref`/`click_state` 由 B4 准入补齐，38 不判定点击；解释只用固定模板 + 术语（不调 LLM、不复制引用正文）；`card_id` 由装配方注入、38 不重算；拒绝原因码 6 项为 38 私有，不并入 `GateRejectReason`（23）、不新增 `GateCounter`（32）；AC-B4-65~74 为 `pending`；不改既有 AC 01~64 与任何封闭字段数。

### §10.3 建议执行顺序（不含新任务编号）

1. 先完成 **U-01**：AC-B4-48 方案 A 的 200 例回放与两次逐字节一致性，闭合 P3 遗留 pending。
2. 再落 **U-02**：B4 关键词候选/卡片契约与 `click_state` 前置；只动契约与测试，不动 38/39/51 实体。
3. 随后按 **U-03（已关闭，见 §15）→ U-04（已关闭，见 §13）→ U-05（规格已定、待实现，见 §16 点击 UI 与 §17 设置向导）** 接入 38 合成、39 校验、51 UI；每步以"只消费 B4 契约、不反写记录"为门禁。
4. **U-06~U-08** 保持独立排期；在 U-01/U-02 未闭合前，不宣称关键词卡片或 B4 出口完成。

### §10.4 工作区与提交状态

- 本轮修改文件：`.codeartsdoer/specs/b4_gate/spec.md`、`.codeartsdoer/specs/b4_gate/requirements.md`、`docs/progress/b4-status.md`。
- 未修改：`design.md`、`tasks.md`、任何 `crates/**` 源码、`Cargo.toml`/`Cargo.lock`、既有门禁脚本。
- 未执行 `git commit`；如需把本次需求增量提交，请确认后单独执行（提交范围仅上述 3 个文档）。
## §11 P4 增量快照（W-B4-34 ~ W-B4-42）

日期：2026-11-22。

| 项 | 命令 | 结果 |
|---|---|---|
| 关键词 | `cargo test -p meeting-copilot-application --test gate_keyword` | 14 passed / 0 failed |
| 回放 | `cargo test -p meeting-copilot-application --test gate_replay` | 6 passed / 0 failed（含 200 例） |
| 六既有 target | `gate_{prefilter,relevance,dedupe,budget,outbound}` | 全 0 |
| 七规则 | `source/zero-transcript/budget/outbound/record/targets/doc-plan` | 7 × 0 |
| target 清单 | `NEW_TARGETS` | 20 项，含 `gate_keyword` |
| 回放面 | `grep -c '^pub fn ' gate/api.rs` | 4（零 diff） |
| 全量 | `cargo test --workspace` | 721 passed / 0 failed / 9 ignored |
| 静态 | `fmt --check`、`clippy -D warnings` | 0 |

AC 三态（增量）：

| AC | 状态 | 证据 |
|---|---|---|
| AC-B4-51 | passed | `keyword_candidates_cover_five_kinds`、`keyword_candidate_keeps_first_seen_only`、`unanswered_question_is_not_a_keyword_card` |
| AC-B4-52 | passed | `keyword_card_requires_gate_allow`、`deny_never_emits_clickable_card`、`g2_skip_marks_stage_skipped`、`g3_skip_marks_stage_skipped` |
| AC-B4-53 | passed | `keyword_card_has_thirteen_fields`、`keyword_reference_view_has_five_fields`、`keyword_candidate_has_ten_fields`、`keyword_card_has_deterministic_id` |
| AC-B4-54 | passed | `clicking_keyword_card_is_local_only`、`continue_search_reenters_gate`（本地查看，0 外发/0 检索，重过门） |
| AC-B4-55 | passed | `click_permission_is_fail_closed`、`keyword_card_hidden_under_screen_share` |
| AC-B4-56 | passed | `keyword_card_zero_transcript_boundary`、`--rule=zero-transcript`、`--rule=record` |
| AC-B4-57 | passed | `--rule=source`（禁词 0）、`gate/**` 无 38/39/51 实体、`tests/replay/` 不存在 |

未完成项状态：

| 编号 | 状态 |
|---|---|
| U-01 AC-48 200 例 | **closed**（W-B4-41） |
| U-02 AC-51~57 | **closed**（W-B4-34~40） |
| U-03 / U-05 | open（P4 时点：38 合成 / 51 UI 实体未承接；U-03 已于 2026-11-22 更新为"规格已定，待实现"，见 §14；U-05 于 2026-11-22 更新为"规格已定，待实现"，见 §16） |
| U-04 39 引用对齐校验 | **规格已定，待实现**（`spec.md` §5.7/§6.14~6.15/§7.12、`requirements.md` §11、AC-B4-58~64；待建 `citation.rs` + `citation/align.rs` + `citation_align` target） |
| U-06 13.3 回放本体 | open（`tests/replay/` 保持不存在） |
| U-07 41.5/42/43/52 | open |
| U-08 真实端点/真机 | open（9 ignored，不宣称 G4） |

新增偏差：

| 编号 | 内容 | 处置 |
|---|---|---|
| D-B4-P27 | `AllowTraceRef` 计入 `gate_keyword.rs`（11 项） | `new` 单参仅用于契约测试；装配路径用 `with_trace` 传入真实 `TraceId` |
| D-B4-P28 | `relevance_enabled=false → G2=Skipped`、`dedupe_enabled=false → G3=Skipped` | 已落 `diagnostics.rs`，不扩字段、不新增计数 |
| D-B4-P29 | 200 例 runner 与构造器在测试内 | `gate/replay.rs` 零 diff |
| D-B4-P30 | `scan_candidates_with_rule -> Vec<ScannedCandidate{rule,term,candidate}>` | 保留原始规则标签与词，`detect_gaps` 包装按 term 去重保持 AC-01 |
| D-B4-P31 | `card_drafts` 仅由 Allow 决定，`click_state` 独立 | 卡片可展示但不可点（AC-55） |
| D-B4-P32 | `KeywordCardAdmission.body_visible` | 投屏/紧急遮蔽为 false，渲染仍归 51 |
| D-B4-P33 | AC-48 的 200 例按 25 类循环 | 覆盖 21 个可构造拒绝码；Allow 子形由 `replay_allow_variants_are_explicit` 显式断言 Skipped/queued |

## §12 39 引用对齐校验需求增量登记（2026-11-22）

> 本节登记 39 的**需求规格增量**。本轮只更新 `spec.md`、`requirements.md` 与本文件，**未改代码、未建 39 实体文件、未提交**。

### §12.1 增量规格落点

| 文件 | 章节 | 内容 |
|---|---|---|
| `.codeartsdoer/specs/b4_gate/spec.md` | §5.7 | 39 职责、输入/输出、三态规则、异常/降级、零转录、归属与接口 |
| `.codeartsdoer/specs/b4_gate/spec.md` | §6.14~6.15 | `CitationCheckInput`（8 项）、`CitationAlignmentResult`（6 项）、唯一原因顺序与不变量 |
| `.codeartsdoer/specs/b4_gate/spec.md` | §7.12 | AC-B4-58~64（7 条，逐条可机器判定） |
| `.codeartsdoer/specs/b4_gate/requirements.md` | §11 | 39 增量需求（定位、术语、四组能力、数据约束、AC 映射、出口影响） |
| `docs/progress/b4-status.md` | §10.1 / §11 / §12 | U-04 口径更新、AC 三态、决策登记 |

### §12.2 新增决策（D-B4-P34 ~ D-B4-P37）

| 编号 | 决策 | 理由 | 影响面 |
|---|---|---|---|
| **D-B4-P34** | 39 独立交付形态 = domain 契约 `crates/domain/src/model/citation.rs` + application 只读校验 `crates/application/src/citation/align.rs`；**不建** UI、卡片合成、卡片存储、`tests/replay/` | 39 只做校验与判定；先落契约与只读函数即可被 38/51 消费，避免越界 | 新增 2 个文件 + 1 个 test target；`gate/**` 不改 |
| **D-B4-P35** | 39 API 三态用 `Passed`/`Failed`/`Unknown`；落到 B4 `CitationAlignment` 时 `Unknown→Unverified` **唯一投影**，其余同名；不重命名/不扩展既有 3 变体 | 用户口径用 `Unknown`；既有 AC-B4-53/55 与 `CitationAlignment` 已冻结为 `Unverified`，直接改名会破坏 57 条既有 AC | `CitationAlignment` 变体数不变；新增 `AlignmentVerdict`（39 侧）；AC-B4-59 断言投影唯一 |
| **D-B4-P36** | 39 判定只消费 ID 面：引用仅 `reference_id`/`trust_tier`/`access_status`/`published_at_ms`/`alignment`；`normalized_term` 仅内存比对、不落记录；**不联网抓正文**、不读系统时间/环境变量 | 满足零转录与离线红线；避免把页面标题/摘要/正文带入门控或回放 | `--rule=zero-transcript` 扫描面扩 39 两文件；AC-B4-61/62 |
| **D-B4-P37** | 新增第 8 个 target `citation_align`（`NEW_TARGETS` 20→21）；`check-b4-gates.sh` 的 `source`/`zero-transcript` 扫描面追加 39 两文件，**规则仍 7 条、退出码语义不变**；`api.rs` 4 函数、`GateRecord` 14、`GateAuditEvent` 7、`GateInputFixture` 23、`ReplayReport` 5、`GateCounter` 32、`GateRejectReason` 23 全部不变 | 否则 39 契约是静态盲区，AC-B4-58~63 不可机器判定；新增 target 不改变既有 6 项断言 | `scripts/` 两处扩展；AC-B4-64 |

### §12.3 AC 三态（增量 AC-B4-58~64）

| AC | 状态 | 证据口径 |
|---|---|---|
| AC-B4-58 | pending | 规格已定；待 `citation_input_is_readonly_and_closed` 与封闭字段数复核 |
| AC-B4-59 | pending | 规格已定；待 `citation_alignment_is_three_state_and_fail_closed`、`unknown_projects_to_unverified_only` |
| AC-B4-60 | pending | 规格已定；待 `citation_passed_enables_click_only_with_other_facts` 等 3 用例 |
| AC-B4-61 | pending | 规格已定；待 `citation_result_is_id_only` + `--rule=zero-transcript` |
| AC-B4-62 | pending | 规格已定；待 `citation_check_is_offline_and_deterministic`、`citation_ignores_reference_order` + `--rule=source` |
| AC-B4-63 | pending | 规格已定；待 `citation_ownership_is_outside_b4_and_ui` + 禁词/目录/计数复核 |
| AC-B4-64 | pending | 规格已定；待 `citation_align` target 全绿、`NEW_TARGETS`=21、本文件 U-04 口径复核 |

### §12.4 39 明确不做（本轮边界）

1. **不联网**：不请求真实检索源、不抓取/解析正文、不建立正文缓存。
2. **不生成卡片**：不算标题/解释/置信度、不组装引用集合、不维护 `card_status`（38 职责）。
3. **不做 UI**：不打开详情、不抢焦点、不处理投屏/紧急遮蔽界面（51 职责）。
4. **不改既有封闭面**：`GateDecision`/`GateRecord`（14）/`GateAuditEvent`（7）/`GateInputFixture`（23）/`ReplayReport`（5）/`GateCounter`（32）/`GateRejectReason`（23）/`api.rs` 4 个 `pub fn` 均不变。
5. **不建回放目录**：`crates/application/tests/replay/` 保持不存在；39 结果不进入回放夹具与报告。
6. **不宣称**：离线判定通过 ≠ 真实引用可访问、语义对齐质量或 51 交互质量通过；不宣称 G3/G4/真机。

### §12.5 工作区与提交状态

- 本轮修改文件：`.codeartsdoer/specs/b4_gate/spec.md`、`.codeartsdoer/specs/b4_gate/requirements.md`、`docs/progress/b4-status.md`。
- 未修改：`design.md`、`tasks.md`、任何 `crates/**` 源码、`Cargo.toml`/`Cargo.lock`、既有门禁脚本。
- 未执行 `git commit`；如需提交本次 39 需求增量，请确认后单独执行（提交范围仅上述 3 个文档）。

> 结论：AC-48/51~57 离线用例已通过；38 卡片合成、39 引用校验、51 点击 UI 与真实端点仍未做，**不宣称产品级"可点击关键词卡片"已交付**。后续：38 已于 §15 关闭，39 已于 §13 关闭，51 于 §16 更新为“规格已定、待实现”；真实端点仍未做。
## §13 P5 增量快照（W-B4-43 ~ W-B4-48）

日期：2026-11-22。

| 项 | 命令 | 结果 |
|---|---|---|
| 39 用例 | `cargo test -p meeting-copilot-application --test citation_align` | 12 passed / 0 failed |
| 域契约 | `cargo test -p meeting-copilot-domain --lib` | 153 passed / 0 failed |
| target 清单 | `NEW_TARGETS` | 21 项，含 `citation_align` |
| 七规则 | `source/zero-transcript/budget/outbound/record/targets/doc-plan` | 7 × 0 |
| 回放面 | `grep -c '^pub fn ' gate/api.rs` | 4（零 diff） |
| 静态 | `fmt --check`、`clippy -D warnings` | 0 |

AC 三态（增量）：

| AC | 状态 | 证据 |
|---|---|---|
| AC-B4-58 | passed | `citation_input_is_readonly_and_closed` |
| AC-B4-59 | passed | `citation_alignment_is_three_state_and_fail_closed`、`unknown_projects_to_unverified_only` |
| AC-B4-60 | passed | `citation_passed_enables_click_only_with_other_facts`、`citation_failed_still_displayed_but_disabled`、`citation_unknown_is_conservative` |
| AC-B4-61 | passed | `citation_result_is_id_only` |
| AC-B4-62 | passed | `citation_check_is_offline_and_deterministic`、`citation_ignores_reference_order` |
| AC-B4-63 | passed | `citation_ownership_is_outside_b4_and_ui`、`--rule=source` |
| AC-B4-64 | passed | `citation_basis_is_unique_and_ordered`、`citation_counts_match_inputs`、`--rule=targets` |

未完成项：

| 编号 | 状态 |
|---|---|
| U-04 39 引用对齐 | **closed（W-B4-43~48）**；历史口径见 §10.1 |
| U-03 38 卡片合成 | **规格已定，待实现**（见 §14） |
| U-05 51 点击 UI | **规格已定，待实现**（见 §16） |
| U-06 / U-07 / U-08 | open（不变） |

新增偏差：

| 编号 | 内容 | 处置 |
|---|---|---|
| D-B4-P38 | `CONTRACT_UNVERIFIED = CitationAlignment::Unknown` | 不改既有枚举；三态单射投影 |
| D-B4-P39 | `CitationCheckInput` 手写 `Debug`，无 `Serialize` | 防术语与 claim 外泄 |
| D-B4-P40 | `Failed` 三因以上游 `ReferenceView.alignment` 为唯一信号 | v1 可构造，不读正文 |
| D-B4-P41 | `ClaimRef` 类型层拒绝空/空白/`@`/scheme 分隔符/超长 | 非法引用不进入判定 |
| D-B4-P42 | `bases: [u32; 11]` 下标由 `AlignmentBasis::index()` 给出 | 顺序与 §5.5.2 一致 |
| D-B4-P43 | 39 不改 `gate/**`；点击前置仅在测试侧复用 `keyword_card_admission` | 分层不越界 |
| D-B4-P44 | 测试断言用类型值/`Debug` 白名单替代 `serde_json`（application 无该依赖） | 键封闭由 domain 单测与 `--rule=record` 保证 |

## §14 38 卡片合成需求增量登记（2026-11-22）

> 本节登记 38 的**需求规格增量**。本轮只更新 `spec.md`、`requirements.md` 与本文件，**未改代码、未建 38 实体文件、未提交**。

### §14.1 增量规格落点

| 文件 | 章节 | 内容 |
|---|---|---|
| `.codeartsdoer/specs/b4_gate/spec.md` | §5.8 | 38 职责、归属矩阵、7 项输入、11 项输出、合成规则、降级、零转录与接口约定 |
| `.codeartsdoer/specs/b4_gate/spec.md` | §6.16~6.18 | `CardComposeInput`（7 项）、`ComposedCard`（11 项）、`CardComposeOutcome`（2 项）、`ComposeRejectReason`（6 项） |
| `.codeartsdoer/specs/b4_gate/spec.md` | §7.13 | AC-B4-65~74（10 条，逐条可机器判定） |
| `.codeartsdoer/specs/b4_gate/requirements.md` | §12 | 38 增量需求（定位、术语、四组能力、数据约束、AC 映射、出口影响） |
| `docs/progress/b4-status.md` | §10.1 / §13 / §14 | U-03 口径更新、AC 三态、决策登记 |

### §14.2 新增决策（D-B4-P45 ~ D-B4-P52）

| 编号 | 决策 | 理由 | 影响面 |
|---|---|---|---|
| **D-B4-P45** | 38 独立交付形态 = `crates/application/src/cards/**`（1 个合成纯函数 + 1 个状态流转纯函数 + 只读类型）；**不新增 domain 契约**，复用 `KeywordCandidate`/`ReferenceView`/`CitationAlignment`/`AllowTraceRef` 与 39 结果；不建 UI/存储/回放目录 | 38 只做确定性组装；复用既有契约可保证 13/5/3 与零转录不漂移；新增 domain 会扩大封闭面 | 新增 2 个文件 + 1 个 test target；`lib.rs` +1 行；`domain/**` 零 diff |
| **D-B4-P46** | 38 输出 11 项"可展示卡片数据"，**不含** `gate_decision_ref`/`click_state`；由装配方用 B4 `keyword_card_admission` 补 2 项成 13 字段 `KeywordCardDraft` | 点击是 B4 的 7 项合取判定，38 无权决定；"11 + 2"保证与既有契约无损映射 | AC-B4-68；`cards/**` 中 `ClickState` 命中 0 |
| **D-B4-P47** | `card_id` 由装配方按 B4 派生规则生成并以只读字段注入；38 **不重算、不派生** | 避免 38 复制 `derive_card_id` 造成双实现漂移与"一候选多卡"；保证 AC-B4-53 的确定性 ID | AC-B4-67；38 输入含 `card_id` |
| **D-B4-P48** | 解释/结论只允许"固定模板常量 + `normalized_term`"确定性拼接；**不调用 LLM、不语义生成、不复制引用标题/摘要/正文**；超 80 字置 `None`（不截断） | 满足离线红线与零转录；避免 38 变成"第二个生成器"或引入正文 | `cards/**` 无 `llm`/`reqwest`；AC-B4-66/69 |
| **D-B4-P49** | 新增 38 私有 `ComposeRejectReason` 6 项；**不并入** `GateRejectReason`（23）、**不新增** `GateCounter`（32） | 拒绝原因属于合成器诊断，不是门控原因；扩展 B4 封闭集会破坏 AC-B4-08/47 | `GateRejectReason` 仍 23；`snapshot()` 仍 32 |
| **D-B4-P50** | 新增第 9 个 target `card_compose`（`NEW_TARGETS` 21→22）；`cards/**` 与 `tests/card_compose.rs` 纳入 `source`/`zero-transcript` 扫描面；`--rule=targets` 循环列表**不追加** `card_compose` | 否则 38 契约是静态盲区；沿用 X-05 口径可避免改变既有 7 项断言 | `scripts/` 两处扩展；规则仍 7 条 |
| **D-B4-P51** | 38 只决定 `Displayed`/`Withdrawn`；`Collapsed` 只由 51 交互产生；`Withdrawn` 不可复活 | 卡片状态与交互态分离，避免 38 越界渲染；与 spec §6.12-8 的"38/51 维护"一致 | `next_card_status` 只允许两态；AC-B4-67 |
| **D-B4-P52** | `CardComposeInput` 手写 `Debug` 白名单，不输出 `normalized_term`、展示词与解释文本 | 与 39 的 D-B4-P39 同口径；防 `derive(Debug)` 把术语打进 panic/测试失败输出 | AC-B4-72 |

### §14.3 AC 三态（增量 AC-B4-65~74）

| AC | 状态 | 证据口径 |
|---|---|---|
| AC-B4-65 | pending | 规格已定；待 `card_compose_input_is_readonly_and_closed` 与封闭字段数复核 |
| AC-B4-66 | pending | 规格已定；待 `card_compose_title_and_summary_limits`、`card_compose_preserves_confidence_and_trigger` |
| AC-B4-67 | pending | 规格已定；待 `card_compose_requires_gate_allow`、`card_compose_is_idempotent_per_candidate`、`card_compose_withdrawn_is_not_revived` |
| AC-B4-68 | pending | 规格已定；待 `card_compose_aligns_unknown_to_unverified`、`card_compose_leaves_click_state_to_b4` |
| AC-B4-69 | pending | 规格已定；待 `card_compose_failed_stays_displayed_without_conclusion` |
| AC-B4-70 | pending | 规格已定；待 `card_compose_references_are_id_only_and_sorted` |
| AC-B4-71 | pending | 规格已定；待 `card_compose_is_offline_and_deterministic`、`card_compose_ignores_reference_order` + `--rule=source` |
| AC-B4-72 | pending | 规格已定；待 `card_compose_zero_transcript_boundary` + `--rule=zero-transcript` |
| AC-B4-73 | pending | 规格已定；待 `card_compose_ownership_is_outside_gate_and_ui` + 禁词/目录复核 |
| AC-B4-74 | pending | 规格已定；待 `card_compose` target 全绿、`NEW_TARGETS`=22、AC 汇总=74、U-03 口径复核 |

### §14.4 38 明确不做（本轮边界）

1. **不渲染**：不打开详情、不抢焦点、不处理三形态、投屏与紧急遮蔽（51 职责）。
2. **不存储**：不建卡片表、不写缓存、不写展示/点击审计（52 职责）；`tests/replay/` 保持不存在（13.3 职责）。
3. **不联网、不读原文**：不调 `reqwest`/`TcpStream`/`std::net`，不抓正文、不缓存正文、不读完整转写与查询原文。
4. **不判定点击**：不写 `click_state`/`gate_decision_ref`，不 `use crate::gate`，不调用 `resolve_click_state`。
5. **不调 LLM、不语义生成**：解释只用固定模板 + 术语；不重算置信度、不改触发类型。
6. **不改封闭面**：`KeywordCardDraft` 13、`ReferenceView` 5、`CitationAlignment` 3、`api.rs` 4、`GateRecord` 14、`GateAuditEvent` 7、`GateInputFixture` 23、`ReplayReport` 5、`GateCounter` 32、`GateRejectReason` 23 全部不变。
7. **不宣称**：离线通过 ≠ 解释语义正确、≠ 真实引用可访问、≠ 51 渲染质量；不宣称 G3/G4/真机，不宣称产品级"可点击关键词卡片"已交付。

### §14.5 工作区与提交状态

- 本轮修改文件：`.codeartsdoer/specs/b4_gate/spec.md`、`.codeartsdoer/specs/b4_gate/requirements.md`、`docs/progress/b4-status.md`。
- 未修改：`design.md`、`tasks.md`、任何 `crates/**` 源码、`Cargo.toml`/`Cargo.lock`、既有门禁脚本。
- 未执行 `git commit`；如需提交本次 38 需求增量，请确认后单独执行（提交范围仅上述 3 个文档）。
- 下一步入口：实现前先确认 §14.2 的 D-B4-P45~P52，特别是 7/11/2/6 字段封闭、`card_id` 注入口径与"38 不写 click_state"。

## §15 P6 增量快照（W-B4-49 ~ W-B4-56）

日期：2026-11-22。

| 项 | 命令 | 结果 |
|---|---|---|
| 38 用例 | `cargo test -p meeting-copilot-application --test card_compose` | 14 passed / 0 failed |
| target 清单 | `NEW_TARGETS` | 22 项，含 `card_compose` |
| 七规则 | `source/zero-transcript/budget/outbound/record/targets/doc-plan` | 7 × 0 |
| 回放面 | `grep -c '^pub fn ' gate/api.rs` | 4（零 diff） |
| 静态 | `fmt --check`、`clippy -D warnings` | 0 |

AC 三态（增量）：

| AC | 状态 | 证据 |
|---|---|---|
| AC-B4-65 | passed | `card_compose_input_is_readonly_and_closed` |
| AC-B4-66 | passed | `card_compose_title_and_summary_limits`、`card_compose_preserves_confidence_and_trigger` |
| AC-B4-67 | passed | `card_compose_requires_gate_allow`、`card_compose_is_idempotent_per_candidate`、`card_compose_withdrawn_is_not_revived` |
| AC-B4-68 | passed | `card_compose_aligns_unknown_to_unverified`、`card_compose_leaves_click_state_to_b4` |
| AC-B4-69 | passed | `card_compose_failed_stays_displayed_without_conclusion` |
| AC-B4-70 | passed | `card_compose_references_are_id_only_and_sorted` |
| AC-B4-71 | passed | `card_compose_is_offline_and_deterministic`、`card_compose_ignores_reference_order` |
| AC-B4-72 | passed | `card_compose_zero_transcript_boundary`、`--rule=zero-transcript` |
| AC-B4-73 | passed | `card_compose_ownership_is_outside_gate_and_ui`、`--rule=source` |
| AC-B4-74 | passed | `--rule=targets`、`NEW_TARGETS=22` |

未完成项：

| 编号 | 状态 |
|---|---|
| U-03 38 卡片合成 | **closed（W-B4-49~56）**；历史口径见 §10.1 |
| U-05 51 点击 UI | **规格已定，待实现**（见 §16） |
| U-06 / U-07 / U-08 | open（不变） |

新增偏差：

| 编号 | 内容 | 处置 |
|---|---|---|
| D-B4-P53 | 38 契约落 application（domain 零 diff） | 类型只供渲染层消费 |
| D-B4-P54 | 只有 `allow_ref.is_some()` 才合成 | 唯一 Allow 前置 |
| D-B4-P55 | 已撤回优先于重复 | S3 早于 S4 |
| D-B4-P56 | 重复信号取 `suppressed_notice`（X-08） | 不读卡片表 |
| D-B4-P57 | 标题上限由类型层保证（X-09） | 防御分支仅单测 |
| D-B4-P58 | 解释固定模板 + 术语；非 Passed 一律 `None` | 不调用模型 |
| D-B4-P59 | 11 项输出不含 `click_state`/`gate_decision_ref` | 点击合取仍归 B4 准入 |
| D-B4-P60 | 诊断计数独立 `[u32; 6]` | 不扩 `GateCounter` |
| D-B4-P61 | `trigger` 常量 `Auto`（Y-01-C） | OnDemand 走既有草稿路径 |
| D-B4-P62 | 引用按 ID 稳定排序，`project_alignment` 唯一调用 | 顺序无关 |
| D-B4-P63 | `triggered_at_ms = candidate.trigger_span.end_ms` | 不读系统时间 |
| D-B4-P64 | 6 顶层 pub 类型 = 4 契约 + 2 辅助 | 面收敛 |

> 结论：38 卡片合成离线契约已闭环；51 点击 UI **规格已定、待实现（见 §16）**；真实端点仍 open，**不宣称产品级“可点击卡片”已交付**。

> 结论：39 的离线契约与只读判定已闭环；38 卡片合成已 closed；51 点击 UI **规格已定、待实现（见 §16）**，**不宣称产品级“可点击卡片”已交付**。

## §16 51 点击 UI 需求增量登记（2026-11-22）

> 本节登记 51 的**需求规格增量**。本轮只更新 `spec.md`、`requirements.md` 与本文件，**未改代码、未建 51 实体文件、未提交**。

### §16.1 增量规格落点

| 文件 | 章节 | 内容 |
|---|---|---|
| `.codeartsdoer/specs/b4_gate/spec.md` | §5.9 | 51 职责、归属矩阵、三条接口约定、视图投影/点击状态机/屏蔽态/零转录/平台边界/异常场景 |
| `.codeartsdoer/specs/b4_gate/spec.md` | §6.19~6.23 | `KeywordCardView`（10 项）、`CardBodyView`（4 项）、`ConclusionView`（3 变体）、`CardDetailView`（5 项）、`ViewFacts`（6 项）、`CardViewOutcome`（2 项）、`ViewRejectReason`（3 项）、`ClickOutcome`（2 项）、不变量 |
| `.codeartsdoer/specs/b4_gate/spec.md` | §7.14 | AC-B4-75~84（10 条，逐条可机器判定，含 target 名与扫描面） |
| `.codeartsdoer/specs/b4_gate/requirements.md` | §13 | 51 增量需求（定位、术语、五组能力、数据约束、AC 映射、出口影响） |
| `docs/progress/b4-status.md` | §10.1 / §13 / §15 / §16 | U-05 口径更新、AC 三态、决策登记 |

### §16.2 新增决策（D-B4-P65 ~ D-B4-P74）

| 编号 | 决策 | 理由 | 影响面 |
|---|---|---|---|
| **D-B4-P65** | 51 独立交付形态 = `crates/ui-bridge/src/keyword_card/`（`mod.rs` + `view.rs`）+ `ui-bridge/src/lib.rs` +1 行 + `crates/ui-bridge/tests/keyword_card_view.rs`；交付 3 个纯函数与 8 个视图类型；**不建**平台适配、52 存储、`tests/replay/` | 51 只做视图模型与点击契约；纯函数可在离线判定；避免把 UI 与存储/回放耦合 | 新增 2 个源文件 + 1 test target；`gate/**`/`cards/**`/`domain/**` 零 diff |
| **D-B4-P66** | 51 只读 13 字段 `KeywordCardDraft`（唯一权威）与 11 项 `ComposedCard`（只读预览）；`ComposedCard` 预览恒 `Disabled{NotAllowed}`；51 **不补** `gate_decision_ref`/`click_state` | 13 字段是 B4 补齐后的完整契约；11 项缺 2 项，不能判定点击；避免 51 复制 B4 准入形成双实现 | AC-B4-75；`ui-bridge/src/keyword_card` 无 `ClickState` 写入 |
| **D-B4-P67** | 视图模型封闭 `KeywordCardView` 10 项、`CardBodyView` 4 项、`CardDetailView` 5 项、`ViewFacts` 6 项、`CardViewOutcome` 2 项、`ClickOutcome` 2 项；`body_visible=false` 时 `body=None` | 字段封闭使 AC 可机器判定；隐藏态不下发正文，避免投屏/遮蔽泄露 | `kw_fields` 10/4/5/6/2/2；AC-B4-75/76 |
| **D-B4-P68** | `click_state` **只降不升**；固定优先级 `Withdrawn`（短路）→ `screen_share` → `emergency_masked` → `authorization_revoked` → `!session_matches || reference_stale` → 输入 `Disabled{reason}` → `alignment != Passed` → `Enabled` | 51 只消费 B4 权威，不重新实现 7 项合取；只允许更严格，避免 UI 绕过门控 | AC-B4-77；`Enabled` 反例矩阵 |
| **D-B4-P69** | `Withdrawn` 不生成视图（`view=None` + `CardWithdrawn`），不可因重新授权/刷新/重放复活；授权撤销/跨会话/引用过期/对齐未通过一律禁用 | 撤回是终态；授权撤销不自动恢复旧卡；与既有 AC-B4-55/60 同口径 | AC-B4-79；100 次投影一致 |
| **D-B4-P70** | 投屏/紧急遮蔽 `body_visible=false`、`body=None`、仅 `badge_count`；点击一律拒绝；暂停/静默**不进入** `ViewFacts`、不阻断已展示卡片查看 | 屏蔽态必须隐藏正文；暂停/静默只阻止新自动出卡，不应误伤本地查看 | AC-B4-76；`ViewFacts` 无 `paused`/`silent` |
| **D-B4-P71** | `KeywordCardView`/`CardBodyView`/`CardDetailView` 手写 `Debug` 白名单，不输出标题/结论取值；展示层可持有 `title`/`conclusion`，但不得写入记录/审计/夹具/日志/进度 | 与 39/38 的 `Debug` 白名单同口径；防测试失败/panic 输出泄露文本 | AC-B4-81；`format!("{:?}", view)` 断言 |
| **D-B4-P72** | 新增第 10 个 target `keyword_card_view`（`NEW_TARGETS` 22→23）；`check-b4-gates.sh` 的 `source`/`zero-transcript` 扫描面追加 `crates/ui-bridge/src/keyword_card` 与 `tests/keyword_card_view.rs`；`--rule=targets` 循环仍 7 项、**不追加** `keyword_card_view`；`tests/schema.rs` 不纳入 51 扫描面；**X-03 同口径：脚本改动必须与 51 文件落盘同批，禁止先改脚本** | 否则 51 视图模型是静态盲区；沿用 X-05 不改变既有 7 项断言；只扫 51 测试文件避免 `schema.rs` 的 `panic!` 误命中；脚本先改会让全部 `--rule` 退 2 | `scripts/` 两处扩展；AC-B4-82/84 |
| **D-B4-P73** | `crates/ui-bridge/Cargo.toml` 新增 `meeting-copilot-application`/`meeting-copilot-domain` 两条 **path 依赖**；不新增外部 crate；禁止 `use crate::gate`/`use meeting_copilot_application::gate`；`tauri`/`windows`/`objc2` 不进入 ui-bridge 依赖闭包 | 51 只消费 application/domain 公开类型；path 依赖不引入新包；平台适配留给 app/真实 UI 阶段 | `Cargo.lock` 仅依赖关系变化；AC-B4-82/83 |
| **D-B4-P74** | 51 **不实现** §4.5.2 的 `CardSurface` trait；改为 3 个纯函数 + 视图类型；真实像素/焦点/三形态/投屏 API 不进入本轮离线判定，也不宣称 | 纯函数可被 `cargo test` 稳定判定；trait/平台句柄会引入不可离线验证的实现；与 38 的"不实现 `CardSynthesis`"同口径 | §4.5.2 的 51 预留被收窄；AC-B4-75~84 只判定视图模型与交互契约 |

### §16.3 AC 三态（增量 AC-B4-75~84）

| AC | 状态 | 证据口径 |
|---|---|---|
| AC-B4-75 | pending | 规格已定；待 `keyword_card_view_input_is_closed`、`keyword_card_view_from_composed_is_not_clickable` 与 10/4/5/6/2/2 字段数复核 |
| AC-B4-76 | pending | 规格已定；待 `keyword_card_view_hides_body_under_screen_share`、`keyword_card_view_hides_body_under_emergency_mask` |
| AC-B4-77 | pending | 规格已定；待 `keyword_card_view_click_state_is_downgrade_only`、`keyword_card_view_never_upgrades_disabled` |
| AC-B4-78 | pending | 规格已定；待 `keyword_card_view_detail_is_local_only`、`keyword_card_view_detail_has_no_outbound_side_effect` |
| AC-B4-79 | pending | 规格已定；待 `keyword_card_view_withdrawn_is_not_revived`、`keyword_card_view_stale_or_cross_session_is_disabled`、`keyword_card_view_failed_alignment_stays_visible_but_disabled` |
| AC-B4-80 | pending | 规格已定；待 `keyword_card_view_conclusion_is_none_without_passed`、`keyword_card_view_conclusion_preserved_for_passed` |
| AC-B4-81 | pending | 规格已定；待 `keyword_card_view_zero_transcript_boundary`、`keyword_card_view_debug_is_whitelisted` + `--rule=zero-transcript` |
| AC-B4-82 | pending | 规格已定；待 `keyword_card_view_is_offline_and_platform_free`、`keyword_card_view_ignores_reference_order` + `--rule=source` + `cargo tree` |
| AC-B4-83 | pending | 规格已定；待 `keyword_card_view_ownership_is_outside_gate_and_cards`、`keyword_card_view_public_types_are_readonly` + 禁词/零 diff 复核 |
| AC-B4-84 | pending | 规格已定；待 `keyword_card_view` target 全绿、`NEW_TARGETS`=23、AC 汇总=84、U-05 口径复核 |

### §16.4 51 明确不做（本轮边界）

1. **不渲染像素、不做平台适配**：不依赖 `tauri`/`windows`/`objc2`，不建窗口句柄、不建焦点 API、不实现真实三形态/投屏 API。
2. **不联网、不检索、不调 LLM、不抓正文**：不请求真实检索源，不调用模型，不解析/缓存网页正文。
3. **不存储、不写审计**：不建卡片表/缓存/展示审计；`tests/replay/` 保持不存在（52/13.3 职责）。
4. **不读原文/系统时间/环境变量/文件**：不消费完整转写、查询原文；不读 `SystemTime::now`/`Instant::now`/`std::env`/`std::fs`。
5. **不合成、不校验、不判门控**：不调用 `compose_keyword_card`/`project_alignment`/`keyword_card_admission`；不补 `gate_decision_ref`/`click_state`；不写 `gate/**`/`cards/**`/`domain/**`。
6. **不改既有封闭面**：`KeywordCardDraft` 13、`ReferenceView` 5、`CitationAlignment` 3、`ComposedCard` 11、`CardComposeInput` 7、`api.rs` 4、`GateRecord` 14、审计 7、夹具 23、`ReplayReport` 5、`GateCounter` 32、`GateRejectReason` 23 全部不变。
7. **不宣称**：离线通过 ≠ 真实像素渲染/焦点/三形态/投屏 API/产品级交互质量通过；不宣称 G3/G4/真机，不宣称产品级“可点击卡片”已交付。

### §16.5 工作区与提交状态

- 本轮修改文件：`.codeartsdoer/specs/b4_gate/spec.md`、`.codeartsdoer/specs/b4_gate/requirements.md`、`docs/progress/b4-status.md`。
- 未修改：`design.md`、`tasks.md`、任何 `crates/**` 源码、`crates/ui-bridge/Cargo.toml`、根 `Cargo.toml`/`Cargo.lock`、既有门禁脚本。
- 未执行 `git commit`；如需提交本次 51 需求增量，请确认后单独执行（提交范围仅上述 3 个文档）。
- 下一步入口：实现前先确认 §16.2 的 D-B4-P65~P74，特别是 10/4/5/6/2/2 字段封闭、`ComposedCard` 预览恒 `Disabled{NotAllowed}`、只降不升优先级与“51 不写 `click_state`/不写 `card_status`”。

> 结论：51 点击 UI **规格已定，待实现**；AC-B4-75~84 全部 `pending`；**不宣称真实 UI 已渲染、不宣称产品级“可点击卡片”已交付**。
## §17 51 设置向导需求增量登记（2026-11-22）

> 本节登记 51 设置向导（API Key 输入、模型选择/自定义、端点仅白名单、一键连通性测试、保存/清除、失败人话化、傻瓜化界面）的**需求规格增量**。本轮只更新 `spec.md`、`requirements.md` 与本文件，**未改代码、未建 settings 实体文件、未新增依赖、未提交**。

### §17.1 增量规格落点

| 文件 | 章节 | 内容 |
|---|---|---|
| `.codeartsdoer/specs/b4_gate/spec.md` | §5.10 | 51 设置向导的定位、归属矩阵、四条接口约定、DFX 增量、11 条业务规则、交互流程、9 条异常场景 |
| `.codeartsdoer/specs/b4_gate/spec.md` | §6.24~6.28 | `SettingsView`（8 项）、`SettingsFacts`（6 项）、`SettingsInput`（5 项）/`SettingsIntent`（3 变体）、`SettingsFormState`（6 态）/`ProbeEvent`（5 项）/`ProbeFailureKind`（8 项）/`FieldIssue`（6 项）/`ProbeOutcome`（2 项）、不变量与既有封闭字段不变 |
| `.codeartsdoer/specs/b4_gate/spec.md` | §7.15 | AC-B4-85~94（10 条，逐条可机器判定，含 target 名、扫描面与样本纪律） |
| `.codeartsdoer/specs/b4_gate/spec.md` | §1.11 / §4.3 / §7.1 / §7.10 / §8 / §9.1 / §9.3 | 不做清单 N-11/N-12、凭据红线 2 条、离线判定覆盖面、AC 汇总 84→94、变更清单 6 行、裁决 J-31~J-34、偏差 D-B4-09 |
| `.codeartsdoer/specs/b4_gate/requirements.md` | §14 | 51 设置向导增量需求（定位、术语、七组能力、数据约束、AC 映射、出口影响） |
| `docs/progress/b4-status.md` | §10.1 / §10.3 / §17 | U-05 口径更新、执行顺序指引、本节登记 |

### §17.2 新增决策（D-B4-P75 ~ D-B4-P84）

| 编号 | 决策 | 理由 | 影响面 |
|---|---|---|---|
| **D-B4-P75** | 51 设置向导独立交付形态 = `crates/ui-bridge/src/settings/`（`mod.rs` + `view.rs` + `input.rs` + `probe.rs`）+ `ui-bridge/src/lib.rs` +1 行 + 一个 settings 测试 target；交付 7 个纯函数与 18 个顶层 `pub` 类型/枚举；**不建**平台句柄、安全存储适配、52 存储、`tests/replay/` | 纯函数可离线判定；把凭据与平台解耦，避免 UI 承担不可验证的 IO | 新增 4 个源文件 + 1 test target；`gate/**`/`cards/**`/`domain/**` 零 diff |
| **D-B4-P76** | 视图模型封闭 `SettingsView` 8 项、`SettingsFacts` 6 项、`SettingsInput` 5 项、`SettingsIntent` 3 变体；分区恰 4 段且顺序固定 | 字段封闭使 AC 可机器判定；分区固定保证"一步式"不被高级项破坏 | `kw_fields` 8/6/5；AC-B4-85 |
| **D-B4-P77** | 掩码 `MaskedKey::Masked` **无载荷**，输出恒为唯一常量，与输入长度/字符集/前后缀无关；`SecretInput` 不实现 `Serialize`/`Deserialize`/`Display`，`Debug` 恒 `***`，无长度/前后缀访问器 | 任何"前 3 后 4""已保存 N 位"都会泄露元数据；无载荷掩码从结构上杜绝泄露 | AC-B4-87；`format!("{:?}")` 断言 |
| **D-B4-P78** | 一步式默认：模型下标 0、端点下标 0、超时 6 000 ms、测试通过自动保存；只填 Key 即 `can_test=true`；清单为空保留自定义模型入口并禁用测试/保存 | 面向非技术用户"最少步骤"；避免空下拉死路 | AC-B4-85；默认草稿断言 |
| **D-B4-P79** | 预设模型名与端点显示名**只来自注入清单**；端点只允许下标，51 不解析地址、不比对 host、不内联域名/白名单常量；源码/测试/进度零模型名字面量 | 内联模型名/域名会让 `--rule=zero-transcript` 自身命中；下标化使"扩白名单"结构上不可达 | AC-B4-89/90；白名单与路径常量零 diff |
| **D-B4-P80** | 测试状态机封闭 6 态，事件 5 项，失败类别 8 项，字段问题 6 项，回执 2 项 XOR；单飞由 `probe_in_flight` 保证；取消回"未验证"；冷却只读注入镜像，不建熔断/计时器 | UI 不得自带重试与熔断，否则与网关冷却形成双实现；单飞避免重复计费 | AC-B4-91；100 次转移一致 |
| **D-B4-P81** | 失败人话化 8 条固定中文文案与 `ProbeFailureKind` 一一对应；文案不含 Key 子串、请求体、端点串、内部错误码；失败保留输入并恢复可测 | 非技术用户需要"下一步做什么"而非错误码；文案是零转录的高危面 | AC-B4-92；文案表断言 |
| **D-B4-P82** | 保存/清除只产出意图 `Persist{Memory\|OsSecureStore}`/`Clear`；清除需二次确认；安全存储拒绝时降级 `Memory` 并显式提示"仅本次运行有效"，禁止静默失败与回退明文落盘 | UI 不执行 IO 才能保证"明文不落盘"可静态断言；降级策略避免用户误以为已持久化 | AC-B4-88；`std::fs`/`keyring` 命中 0 |
| **D-B4-P83** | 新增 settings target `llm_settings_view`（`NEW_TARGETS` 23→24）；脚本新增 `UI_SETTINGS_DIR` 与 `UI_SETTINGS_TEST`（**拼接写法**，脚本内不含完整 target 名）与 `ui_settings_files()`；`require_core_faces()` 增目录断言（缺目录退 2）；`source` 扫描 settings 源码、`zero-transcript` 扫描 settings 源码与测试；规则仍 7 条；`--rule=targets` 循环**不追加**；测试只进 `zero-transcript`（沿用 39 口径，与 38 的 D-B4-P59 不同）；**X-03 同口径：脚本与文件同批提交** | 否则 settings 是静态盲区；测试文件含反例样本，不宜进 source 禁词面；先改脚本会让 7 条规则全部退 2 | `scripts/` 两处扩展；AC-B4-91/93/94 |
| **D-B4-P84** | 设置向导**不新增任何依赖**（`crates/ui-bridge/Cargo.toml` 与 `Cargo.lock` 零 diff），不复用 51 点击 UI 的两条 path 依赖；与 `keyword_card/**` 互不依赖；AC 追加 85~94（汇总 84→94）；U-05 口径更新为"点击 UI 与设置向导：规格已定，待实现"；既有 13 项封闭面与 `gate/api.rs` 4 个 `pub fn` 不变 | settings 只消费注入的纯数据，无需 application/domain 类型；避免 51 两个面互相牵连 | AC-B4-93/94；`Cargo.toml` 零 diff |

### §17.3 AC 三态（增量 AC-B4-85~94）

| AC | 状态 | 证据口径 |
|---|---|---|
| AC-B4-85 | pending | 规格已定；待 `settings_view_is_closed_and_single_step`、`settings_defaults_need_only_api_key` 与 8/6/5 字段数复核 |
| AC-B4-86 | pending | 规格已定；待 `settings_api_key_inline_errors_are_lenient`、`settings_errors_only_after_submit` 与 15/16 边界 |
| AC-B4-87 | pending | 规格已定；待 `settings_api_key_is_masked_without_payload`、`settings_debug_and_errors_never_leak_key` |
| AC-B4-88 | pending | 规格已定；待 `settings_persists_intent_only_and_clears_with_confirmation`、`settings_repo_has_no_plaintext_key` |
| AC-B4-89 | pending | 规格已定；待 `settings_model_preset_or_custom_is_closed`、`settings_model_name_is_not_inlined` |
| AC-B4-90 | pending | 规格已定；待 `settings_endpoint_is_allowlist_index_only`、`settings_endpoint_never_expands_allowlist` + 白名单零 diff |
| AC-B4-91 | pending | 规格已定；待 `settings_probe_state_machine_is_single_flight`、`settings_probe_cancel_and_cooldown` + `--rule=source` |
| AC-B4-92 | pending | 规格已定；待 `settings_failure_messages_are_humanized`、`settings_failure_keeps_user_input` |
| AC-B4-93 | pending | 规格已定；待 `settings_boundary_is_offline_and_credential_safe`、`settings_debug_is_whitelisted` + `--rule=zero-transcript` + `cargo tree` |
| AC-B4-94 | pending | 规格已定；待 `llm_settings_view` target 全绿、`NEW_TARGETS`=24、AC 汇总=94、U-05 口径复核、`Cargo` 零 diff |

### §17.4 51 设置向导明确不做（本轮边界）

1. **不落盘明文**：不写文件、不写仓库、不写日志/审计/回放/截图；只产出 `Persist`/`Clear` 意图，实际落盘由装配层执行。
2. **不扩展域名白名单**：端点只允许注入清单下标；不解析地址、不比对 host、不新增域名；既有白名单与路径常量零 diff。
3. **不直连网络**：不调 `reqwest`/`TcpStream`/`std::net`；连通性请求只经装配层调用既有网关端口。
4. **不做 macOS 适配**：平台只支持 Windows；macOS 一律"不适用（无设备）"，不新增任何 macOS 约束或分支。
5. **不改既有封闭字段**：13 项封闭面与 `gate/api.rs` 4 个 `pub fn` 不变；不新增 `GateCounter`/`GateRejectReason`；不建 52 存储与 `tests/replay/`。
6. **不新增依赖**：`crates/ui-bridge/Cargo.toml` 与 `Cargo.lock` 零 diff；不复用 51 点击 UI 的 path 依赖。
7. **不与卡片面耦合**：`settings/**` 与 `keyword_card/**` 互不依赖。
8. **不宣称**：离线通过不等于真实 UI 渲染、真实焦点、真实操作系统安全存储写入或真实网关连通性通过；不宣称"设置向导已可用"、不宣称 G3/G4/真机。

### §17.5 工作区与提交状态

- 本轮修改文件：`.codeartsdoer/specs/b4_gate/spec.md`、`.codeartsdoer/specs/b4_gate/requirements.md`、`docs/progress/b4-status.md`。
- 未修改：`.codeartsdoer/specs/b4_gate/design.md`、`.codeartsdoer/specs/b4_gate/tasks.md`、任何 `crates/**` 源码、`crates/ui-bridge/Cargo.toml`、根 `Cargo.toml`/`Cargo.lock`、既有门禁脚本。
- 未执行 `git commit`；如需提交本次 51 设置向导需求增量，请确认后单独执行（提交范围仅上述 3 个文档）。
- 下一步入口：实现前先确认 §17.2 的 D-B4-P75~P84，特别是 8/6/5 字段封闭、固定无载荷掩码、一步式默认、单飞状态机与"51 只产出意图不执行 IO"；随后由 51 阶段建 `crates/ui-bridge/src/settings/**` + `llm_settings_view` target + 2 处脚本扩展（同批提交）。

> 结论：51 设置向导 **规格已定，待实现**；AC-B4-85~94 全部 `pending`；**不宣称设置界面已可用、不宣称凭据已安全落盘、不宣称真实网关连通性已验证**。

## §18 P7 增量快照（51 点击 UI）

日期：2026-11-22。

| 项 | 结果 |
|---|---|
| `keyword_card_view` | 19 passed / 0 failed |
| 视图类型 | 8 项（10/4/3/5/6/2/3/2） |
| 纯函数 | 3 个（投影/预览/点击） |
| `NEW_TARGETS` | 23 |
| 七规则 | 7 × 0 |
| 回放面 | `api.rs` 4 函数 |
| 全量 | 780+ passed / 0 failed / 9 ignored |

AC 三态（增量）：

| AC | 状态 |
|---|---|
| AC-B4-75 | passed |
| AC-B4-76 | passed |
| AC-B4-77 | passed |
| AC-B4-78 | passed |
| AC-B4-79 | passed |
| AC-B4-80 | passed |
| AC-B4-81 | passed |
| AC-B4-82 | passed |
| AC-B4-83 | passed |
| AC-B4-84 | passed |

未完成项：

| 编号 | 状态 |
|---|---|
| U-05 51 点击 UI | **closed（P7）**；设置向导 P8 未做 |
| U-06 / U-07 / U-08 | open |

> 结论：51 点击 UI 的视图与点击契约已闭环；设置向导（API Key/模型）仍待 P8；不宣称真机交互质量。
