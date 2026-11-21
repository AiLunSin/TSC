# B2 离线闭环验收记录（b2-offline-closure）

> 依据：`docs/sdd/tasks-b2-offline-closure.md`、`.codeartsdoer/specs/b2_offline_closure/{spec,design}.md`
> 记录日期：2026-11-21
> 结论：**部分完成**（离线项全部判定；8 个真实 LLM 用例未实测）
> 本文不宣称 B2 G2 通过；本轮通过 ≠ B2 G2 通过。

## 1. 环境偏差（AC-B2-OC-02）

| 项 | 实测值 |
|---|---|
| toolchain_dir | `<USER_HOME>\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin` |
| cargo_executable | `<USER_HOME>\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\cargo.exe` |
| 原因 | 直接调用 cargo shim 报 `STATUS_DLL_NOT_FOUND`（0xc0000135） |
| 等价性说明 | toolchain 内 `cargo.exe` 与 shim 解析到同一版本，仅绕过加载器问题，不改变编译/测试语义 |
| 隔离临时目录 | `<REPO>/.tmp/test-temp` |
| 默认目录问题 | 进程默认 TEMP 目录不可写导致依赖磁盘的用例失败；设置 `TEMP/TMP` 后同一批用例转通过 |

## 2. 基线复现与失败归因（AC-B2-OC-05/03）

| 口径 | 记录值 | 实测值 |
|---|---|---|
| passed | 329 | **399** |
| failed | 0 | **0** |
| ignored | 8 | **8** |
| target 数（ok） | — | 38 |

失败归因：**无失败用例**。`cargo test --workspace` 退出码 0，失败用例清单为空。

## 3. 改动白名单与不重写核验（AC-B2-OC-04）

既有文件修改（6）：

| 文件 | 新增 | 删除 | 判定 |
|---|---|---|---|
| `crates/application/src/capture/types.rs` | 155 | 3 | 追加 |
| `crates/application/src/capture/capture_ports.rs` | 53 | 0 | 追加 |
| `crates/application/src/capture/service.rs` | 292 | 60 | 追加/改写接线，非整文件重写 |
| `crates/application/src/capture/mod.rs` | 24 | 2 | 追加导出 |
| `crates/application/tests/capture_asr_backfill.rs` | 8 | 3 | 仅收紧夹具（D12） |
| `crates/application/tests/support/mod.rs` | 77 | 4 | 追加装配 |

新增文件：`capture_gap_gate.rs`、`degraded_guard.rs`、`diagnostics.rs`、`status_publisher.rs`、`subscriptions.rs`、`tests/support/fakes.rs`、6 个集成测试文件、1 个 infrastructure 契约测试、3 个脚本、2 份文档。

单文件删除行均未超过新增行 50%，无整文件重写。

## 4. 逐工作项三态（43 项）

| 编号 | 三态 | 用例/命令 | 退出码 | 备注 |
|---|---|---|---|---|
| OC-01 | 已判定 | `cargo test --workspace` | 0 | 399 passed / 0 failed |
| OC-02 | 已判定 | `test -x "$TC_BIN/cargo"` + 本文第 1 节 | 0 | 路径与临时目录可复现 |
| OC-03 | 已判定 | 全量日志失败清单为空 | 0 | 无失败用例 |
| OC-04 | 已判定 | `git diff --stat` | 0 | 仅追加/收紧，无整文件重写 |
| OC-05 | 已判定 | `cargo test --workspace` | 0 | 实测 399 覆盖记录 329 |
| C01 | 已判定 | `short_gap_500ms_is_not_persisted` | 0 | 0 条落盘、`short_gap_dropped==1` |
| C02 | 已判定 | `long_gap_30s_persists_single_half_open_record` | 0 | 1 条、时长 30000ms |
| C03 | 已判定 | `two_short_gaps_merge_then_persist`、`merged_short_gaps_pass_gate` | 0 | 合并后 700ms 落 1 条 |
| C04 | 已判定 | `resume_from_is_close_minus_capped_duration` | 0 | `resume=close-8000` |
| C05 | 已判定 | `gap_300s_keeps_session_and_insight_ids` | 0 | 哨兵写/删/迁移全 0 |
| C06 | 已判定 | `insight_read_failure_skips_check_without_delete` | 0 | 跳过计数 1、删除 0 |
| C07 | 已判定 | `single_channel_recovery_within_5s` | 0 | 恢复间隔 5000ms |
| C08 | 已判定 | `repeated_flapping_pushes_unavailable_once` | 0 | 不可用推送 1 次 |
| C09 | 已判定 | `intervals_do_not_overlap_and_touch_within_boundary`、`interval_violation_is_reported_for_overlap` | 0 | 400ms 边界与违规项 |
| D01 | 已判定 | `same_state_ten_times_pushes_once_and_updates_reason` | 0 | 事件 1、原因已更新 |
| D02 | 已判定 | `snapshot_visible_inside_subscriber_callback` | 0 | 回调内快照与事件一致 |
| D03 | 已判定 | `hundred_changes_share_one_increasing_sequence`、`channel_and_availability_share_one_sequence` | 0 | 100 条严格递增；共用序号 |
| D04 | 已判定 | `seq_near_upper_bound_is_rejected` | 0 | `SeqExhausted`、溢出计数 1 |
| D05 | 已判定 | `slow_subscriber_does_not_block_others`、`handler_error_only_counts` | 0 | 慢订阅者留队、失败只计数 |
| D06 | 已判定 | `released_subscriber_receives_zero_events` | 0 | 释放后 0 回调 |
| D07 | 已判定 | `latency_p99_and_max_within_5s`、`latency_p99_uses_expected_index` | 0 | 200 样本、p99/max ≤5s |
| D08 | 已判定 | `payload_whitelist_field_counts`、`payload_scan_hits_no_text_or_sample` | 0 | 7/6/4 字段；文本命中 0 |
| D09 | 已判定 | `no_subscriber_still_updates_snapshot` | 0 | 无订阅者仍更新快照 |
| D10 | 已判定 | `reason_changes_inside_gap_do_not_repush` | 0 | 原因变化 5 次仍推送 1 次 |
| E01 | 已判定 | `fakes_expose_minimal_method_surface`、`--rule=fakes` | 0 | 方法面最小化 |
| E02 | 已判定 | `switch_log_append_is_monotonic_and_same_ms_idempotent` | 0 | 同毫秒合并 1 条 |
| E03 | 已判定 | `gap_log_query_merges_adjacent_within_1s` | 0 | 1000ms 窗口合并 |
| E04 | 已判定 | `asr_delivery_counts_by_kind`、`audit_event_has_no_payload` | 0 | 三类计数、审计无载荷 |
| E05 | 已判定 | `system_clock_fake_is_monotonic_non_decreasing` | 0 | 后者 ≥ 前者 |
| E06 | 已判定 | `offline_harness_is_reusable_across_tests`、`--rule=harness` | 0 | 复用文件数 4 |
| E07 | 已判定 | `offline_min_path_has_zero_connections` | 0 | 建连 0 |
| E08 | 已判定 | `bash scripts/check-capture-offline.sh --rule=deps` | 0 | 依赖闭包洁净 |
| E09 | 已判定 | `bash scripts/check-capture-offline.sh --rule=source` | 0 | 禁用调用 0 |
| E10 | 已判定 | `cargo test --workspace --test capture_contract_freeze` | 0 | 6 + 1 例通过 |
| E11 | 已判定 | `bash scripts/test-name-inventory.sh --diff` | 0 | removed=0，新增 42 |
| E12 | 已判定 | `bash scripts/check-g2-claims.sh` / `--self-test` | 0 / 1 | 正常 0 命中；植入退出 1 |
| E13 | 已判定 | `fakes_have_no_network_device_or_credential_calls` | 0 | 网络/设备/凭据 0 |
| F01 | 已判定 | `cargo test --workspace` | 0 | 399 ≥ 396 |
| F02 | 已判定 | 默认执行输出 8 ignored | 0 | 见第 6 节未实测清单 |
| F03 | 已判定 | fmt / clippy / layering / secrets / deny | 0 | 五项全 0 |
| F04 | 已判定 | `bash scripts/check-capture-offline.sh --rule=placeholders` | 0 | 占位实现 0 |
| F05 | 已判定 | 本文五节齐备 | 0 | 基线/三态/门禁/偏差/不宣称 |
| F06 | 已判定 | 结论审阅 | 0 | 存在未实测项，结论为部分完成 |

## 5. 门禁结果（AC-B2-OC-F03）

| 门禁 | 命令 | 退出码 |
|---|---|---|
| 格式 | `cargo fmt --all --check` | 0 |
| 静态检查 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 全量测试 | `cargo test --workspace` | 0（399 passed / 0 failed / 8 ignored） |
| 分层 | `bash scripts/check-layering.sh` | 0 |
| 密钥 | `bash scripts/scan-secrets.sh` | 0 |
| 许可/来源 | `cargo deny --offline check licenses bans sources advisories` | 0 |
| 采集离线 | `bash scripts/check-capture-offline.sh` | 0 |
| G2 宣称 | `bash scripts/check-g2-claims.sh` | 0 |
| 用例名差集 | `bash scripts/test-name-inventory.sh --diff` | 0 |
| 自检 | `--self-test`（采集离线 / G2 宣称） | 1 / 1（预期） |

## 6. 未实测清单（AC-B2-OC-F02）

8 个用例因需真实 LLM 端点与凭据，默认 `ignored`：

1. `deadline_forced_timeout` — 需用户终端注入 LLM 环境变量
2. `first_token_latency_p50_p95` — 同上
3. `rate_limit_observation` — 同上
4. `record_is_recomputable` — 同上
5. `run_gate_skips_without_live` — 同上
6. `schema_conformance_rate` — 同上
7. `schema_violation_probe_once` — 同上
8. `total_latency_four_tasks` — 同上

三项均标注"未实测"，不以离线结果替代。

## 7. 偏差清单（D1~D12）

| 编号 | 内容 | 理由 | 改变对外行为 |
|---|---|---|---|
| D1 | 收紧短缺口落盘口径，`gap_min_ms` 生效 | 需求要求合并后 ≥1000ms 才落盘 | 是（缺口记录数量） |
| D2 | 状态端口留应用层并扩展 | 域层冻结 | 否 |
| D3 | 失效保护取独立模块 `degraded_guard.rs` | X9 允许独立模块 | 否 |
| D4 | 先合并后判门限 | K1 单点决策 | 是（短缺口合并语义） |
| D5 | 门禁使用 toolchain 绝对路径与隔离临时目录 | 环境限制 | 否 |
| D6 | `adjacent_gaps_merge_before_write` 注入 `gap_min_ms=400` 并新增默认策略 0 条断言，用例名保留 | 保留"合并先于落盘"语义 | 否（仅夹具） |
| D7 | `non_adjacent_gaps_remain_two_records` 间隔由 490ms 调整为 1001ms，用例名保留 | 新口径下 <1000ms 必须合并 | 否（仅夹具） |
| D8 | 契约冻结跨两包执行 | `Metric::ALL` 位于 infrastructure | 否 |
| D9 | 状态事件保留 `key` 复合字段，白名单在测试侧静态表展开计数 | 避免重写既有替身 | 否 |
| D10 | 订阅采用有界队列 + 显式 `drain_status(now)` | 离线可判定、无 async runtime | 是（需调度方定期 drain） |
| D11 | `CaptureAvailabilityEvent` 为新类型，旧 `availability()` 保留 | 避免改写既有返回类型 | 否（新增能力） |
| D12 | `capture_asr_backfill` 两个既有用例的缺口时长由 110ms 调整为 1110ms | 门限生效后短缺口不再投递回补 | 否（仅夹具） |

## 8. 结论与不宣称

- 离线可判定项（OC-01~05、C01~C09、D01~D10、E01~E13、F01~F06）全部取得用例名/命令与退出码。
- `docs/progress/test-names-before.txt` 已落盘（365 行，≥337），差集 removed=0。
- **p99 为逻辑上限**，由注入时钟推导，不代表真实 UI 线程时延。
- **本轮通过 ≠ B2 G2 通过**：三平台真实运行、真实 5 秒回退、真实双通道去重均未实测。
- 由于 8 个真实 LLM 用例未实测，整体结论为**部分完成**。