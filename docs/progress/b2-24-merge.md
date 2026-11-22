# B2.24 双通道去重合并与 macOS 范围裁剪 · 验收记录

> 日期：2026-11-21
> 范围：B2.24 双通道去重合并 + macOS 范围裁剪（AC-MS-01~13、AC-DD-01~20）
> 结论：**Windows 单平台离线验收完成**；macOS 全部真机项标注“不适用（无设备）”；G2 未通过、不宣称。

## 1. 交付清单

### 1.1 领域内核（`crates/domain/src/merge/`）

| 文件 | 内容 |
|---|---|
| `policy.rs` | `MergePolicy` 10 字段、默认值、固定顺序边界校验、`MergePolicyError` 四变体 |
| `normalize.rs` | 全角转半角、ASCII 小写、去标点空白、24 对繁简映射 |
| `hash.rs` | FNV-1a 64、SimHash 64（CJK 2-gram + ASCII 词）、汉明距离 |
| `equivalence.rs` | 字符 3-gram 整数余弦（`u128` + 整数平方根）、三级判定 `PairBasis` |
| `merger.rs` | `Pairing`、`MergedUtterance`、`build_merged`（正文取字幕、区间取 ASR、置信度封顶、说话人取舍） |

### 1.2 应用层（`crates/application/src/capture/`）

| 文件 | 内容 |
|---|---|
| `merge_gate.rs` | `MergeSpan`、`MergeSetViolation`、`validate_merged_set`（顺序固定） |
| `merge_types.rs` | `AsrTextEvent`、`TextEvent`、`MergeIntake`、`MergedUnit`、`SingleUtterance`、计数与证据 |
| `dedupe_guard.rs` | `DedupeMergeGuard`：开关直通、空文本/非法区间、`seen` 幂等、窗口候选、超窗 `late_unmerged`、到期释放 |
| `service.rs` | `merge` 字段、`with_merge`、`ingest_text`、`advance_to` 合并释放（`deadline = end_ms + window`） |

### 1.3 脚本与配置

| 文件 | 内容 |
|---|---|
| `scripts/check-mac-scope.sh` | 8 条规则：`na/matrix/ci/g2/claims/contracts/docs/forbidden` + 自检 |
| `scripts/check-capture-offline.sh` | 新增 `--rule=merge-writes`（含 `NEW_TARGETS` 反向断言） |
| `scripts/test-name-inventory.sh` | `NEW_TARGETS` 追加 `merge_dedupe` |
| `scripts/check-g2-claims.sh` | PATTERN 追加 3 类 macOS 宣称 |
| `crates/platform-{win,mac}/resources/capability_matrix.toml` | 各 6 键；mac 全 `not_applicable` |
| `.github/workflows/ci.yml` | Linux 文案改为 `platform=linux`（`unknown-linux` 计数 0） |
| `docs/sdd/{spec,design,tasks}.md` | 87 行处置：保留 38 / 标注 35 / 降级 12 / 删除 2 |

## 2. 验收数据

| 项 | 结果 |
|---|---|
| 全量测试 | 40 targets / **442 passed** / 0 failed / 9 ignored |
| `merge_dedupe` | 11 passed |
| `merge` 领域单测 | 17 + 4（merger）+ 1（性能集成） |
| clippy `-D warnings` | 0 error |
| fmt | 通过 |
| layering / capture-offline / mac-scope forbidden | 0 / 0 / 0 |
| secrets / secrets-docs / deny | 0 / 0 / 0 |
| 用例名差集 | `removed=0`，新增 86 |
| 性能预算 | 1000 对判定 `elapsed_ms=111`（上限 200；CI 容差 500，见 `b4-status` §27 / X-36） |

## 3. 回滚演练

| # | 单元 | 破坏后 | 恢复后 |
|---|---|---|---|
| 1 | `merge_enabled=false` | — | 由 `merge_disabled_is_passthrough` 证明单路直通 |
| 2 | 删除 `removed-by` | `--rule=docs`=1 | 0 |
| 3 | `NEW_TARGETS` 去掉 `merge_dedupe` | `--rule=merge-writes`=1 | 0 |
| 4 | 删除一个 mac target | `--rule=ci`=1 | 0 |

## 4. 偏差登记

| 编号 | 偏差 | 处置 |
|---|---|---|
| D-MB-01 | `capture_gap_gate.rs` 实际 14 个 `#[test]`（任务书写 12） | 改动前后均为 14，按实际取数 |
| D-MB-02 | 性能用例含 `Instant::now`，与“源码禁计时”冲突 | 移入 `crates/domain/tests/merge_equivalence.rs`；CI 容差 500 ms 见 `b4-status` §27 / X-36（设计预算 200 ms 不变） |
| D-MB-03 | `MergeCounters` 用 9 个具名字段而非 `[u64; 9]` | 语义等价，便于直接断言 |
| D-MB-04 | `ObservationPayload::ValidText` 不含文本/引用/区间 | 新增 `ingest_text` 入口，`observe` 契约未改；“observe 第 6 步”未按形式落地 |

## 5. 未覆盖边界

- `service.ingest_text` 尚未在真实采集链路调用（仅测试层装配）。
- `docs/sdd` 87 行处置为机器标注，未做人工逐条复核；`GAP-1`（spec 74 vs design 87 行）以逐行表为准。
- 远端 CI 未验证本轮改动。
- macOS 真机项全部“不适用（无设备）”；CI 仅保留两个 mac target 编译，不作交付门禁。
- **G2 未通过**：Windows 单平台上腾讯/飞书/Zoom 各 30 分钟稳定运行尚未实测。