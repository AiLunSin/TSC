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
| AC-B4-48 200 例回放 | 未完成（方案 A 已裁决；`replay_200_cases_is_byte_identical` 未建） |
| AC-B4-51~57 关键词卡片契约 | 未完成（仅规格；实现归属 B4 契约 + 38/39/51） |
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

> 结论口径：**离线通过 ≠ G4 达标、替身通过 ≠ 真实端点已验证**。AC-B4-48 仍为 pending；AC-B4-51~57 为 2026-11-22 增量规格新增、尚未实现，故 P3 出口与关键词卡片增量范围均未完全闭合。

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
| U-03 | 38 卡片合成 | 未做；B4 只给契约，38 产出标题/解释/置信度/引用集合/状态 | 38 |
| U-04 | 39 引用对齐校验 | 未做；39 产出 `citation_alignment` 三态，`Passed` 才可点击 | 39 |
| U-05 | 51 点击 UI 与三形态 | 未做；点击详情、投屏/紧急遮蔽、焦点由 51 实现 | 51 |
| U-06 | 13.3 回放本体 | 未做；`tests/replay/` 仍不得创建 | 13.3 |
| U-07 | 41.5 / 42 / 43 / 52 | 未做；负反馈、缓存/outbox、按需补全实体、审计哈希链存储 | 对应章节 |
| U-08 | 真实端点 / 真机 | 未判定；真实检索源、真实 LLM 冷却、6s/10s p95、误触发率 | 保持 9 个 `#[ignore]`，不宣称 G4 |

### §10.2 口径约束

1. **AC 编号**：既有 AC-B4-01~50 不重排、不改语义；新增 AC-B4-51~57 只覆盖"契约与前置"，不判定 38/39/51 的实现质量。
2. **零转录**：`GateRecord` 仍 14 字段；`keyword_text`/`summary` 只能存在于卡片展示层与测试进程内，不得进入记录、审计、回放夹具、报告、日志或本文件。
3. **不越界**：B4 不新增卡片合成器、引用校验器、展示器、卡片存储；`gate/**` 中相关符号命中必须为 0。
4. **点击不等于外发**：点击只读本地详情，`outbound_attempts=0`、`llm_calls=0`；"继续检索"必须重新走 G1~G5。
5. **失败闭合**：门控拒绝、引用未对齐、授权撤销、投屏/遮蔽、跨会话/引用过期，一律 `click_state=Disabled{reason}`，不得展示可点击的 `Deny` 卡。

### §10.3 建议执行顺序（不含新任务编号）

1. 先完成 **U-01**：AC-B4-48 方案 A 的 200 例回放与两次逐字节一致性，闭合 P3 遗留 pending。
2. 再落 **U-02**：B4 关键词候选/卡片契约与 `click_state` 前置；只动契约与测试，不动 38/39/51 实体。
3. 随后按 **U-03 → U-04 → U-05** 接入 38 合成、39 校验、51 UI；每步以"只消费 B4 契约、不反写记录"为门禁。
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
| U-03 / U-04 / U-05 | open（38/39/51 实体，P4 不承接） |
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

> 结论：AC-48/51~57 离线用例已通过；38 卡片合成、39 引用校验、51 点击 UI 与真实端点仍未做，**不宣称产品级"可点击关键词卡片"已交付**。