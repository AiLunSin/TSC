# b3_understanding · 编码任务清单（第三阶段）

> 阶段：第三阶段（编码任务规划）。本文件**只产出任务**，不修改任何代码、不修改 `docs/sdd/**`、不修改 `docs/progress/**`。
> 需求基线：`.codeartsdoer/specs/b3_understanding/spec.md`（820 行，AC-B3-01~48）。
> 设计基线：`.codeartsdoer/specs/b3_understanding/design.md`（1563 行；重点 2.0 裁决 C-01/C-02/C-03、2.2 五组接口、2.3 类图与 Rust 草案、2.4 计数、2.5 回滚、2.6 AC 映射、2.7 阶段、2.8 用例名兼容、2.11 偏差）。
> 范围：`docs/sdd/tasks.md` 批次 B3 的 **28 / 29 / 30 / 31**；**不含** 32（缺口识别）与 33（持久化/恢复/背压）。
> 既有基线（不得回归）：**40 targets / 442 passed / 0 failed / 9 ignored**、`removed=0`、macOS 一律"不适用（无设备）"、G2 未通过且不宣称。
> 冲突处置：若 spec 与 design 冲突，**以 spec 的 AC 为准**并在任务内标注；已裁决项 C-01/C-02/C-03 与"新增 28 类型 / 错误类型不进 `ErrorCode`"直接采用，不重新权衡。

---

## 0. 使用说明

### 0.1 编号与书写格式

| 项 | 规则 |
|---|---|
| 阶段 | `P0`（先决）→ `P1`（28 准入）→ `P2`（29 摘要）→ `P3a`（30 抽取）→ `P3b`（31 去重）→ `P3c`（门禁/回归） |
| 主任务 | `W-B3-<n>`，全局连续 1..47，不重复、不跳号 |
| 子任务 | `W-B3-<n>.<m>`，**两级为止**，是最小可勾选单元 |
| AC 编号 | 只引用 `AC-B3-01~48`；`AC-DD-08` 仅在承接合并侧结论时引用，**不复用、不新增** |
| 书写格式 | `**W-B3-n.m 标题** · \`目标文件\`` + `追溯` / `DoD` / `证据` 三行 |
| DoD 判据 | 只允许五类：退出码、用例名、计数/产物字段、grep 计数、脚本对照；禁止"已完成/正常" |
| 证据 | 必须是可直接复制执行的命令（前置 §0.2 环境已生效） |
| 顺序 | 任务顺序即执行顺序；被依赖者在前；31 不得先于 28/30 |

### 0.2 环境准备（git-bash；每条命令执行前先跑一次）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
export RUSTFMT="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/rustfmt.exe"
```

- `scripts/test-name-inventory.sh` 支持 `CARGO` 覆盖（脚本 `:17`），上表导出即生效；不得依赖 rustup shim。
- `cargo fmt --all --check` 需要 PATH 前置的 rustfmt；若局部环境仍失败，用 `RUSTFMT` 显式指定后再跑。
- `TEMP/TMP` 必须落在仓库 `.tmp/test-temp`（`.tmp/` 已在 `.gitignore`）。
- 所有 `--rule=` 系列允许退出 2 表示"扫描面缺失"，**本轮验收不允许出现 2**（出现 2 即目录/文件未按任务落地）。
- 命令统一在项目根 `C:\Users\11403\Desktop\xiangmu\TSC\TSC` 下执行（git-bash 中为 `/c/Users/11403/Desktop/xiangmu/TSC/TSC`）。

### 0.3 执行纪律（违反即不得合并）

1. 不修改 `crates/application/src/capture/**`、`crates/domain/src/merge/**`、`crates/infrastructure/src/llm/**` 的语义与默认值；`fallback.rs` 必须零 diff。
2. `crates/application/src/understand/**` 禁止 `Instant::now` / `SystemTime::now` / `thread::sleep` / 网络 API / `panic!` / `todo!` / `unimplemented!`。
3. `crates/application/src/understand/**` **禁止出现 `ShortText::truncated`**（R-03 假通过陷阱；`infrastructure/llm/wire.rs:137` 既有用法不动）。
4. 禁止读取 `StructuredOutcome.model` / `request_id` / `http_status` 并写入任何对象、计数、日志或夹具；测试替身用占位串 `"stub"`。
5. 不改既有 `#[test]` 函数名，不新增 `#[ignore]`；既有 9 个 `#[ignore]` 数量与名称不变。
6. 不新增 crate 依赖、不新增 Cargo feature、不新增 `ErrorCode`（`InsightPolicyError`/`SplitError`/`SummaryVersionError`/`UnderstandInitError` 均不进冻结集）。
7. 不改 `MIN_INSIGHT_CONFIDENCE=0.35`、`ShortText::LIMIT=60`、`UtteranceText::LIMIT=2000`、`MergePolicy` 10 字段默认值。
8. 零转录红线：不出现 API Key、完整端点 URL、模型名、`request_id`、原始音频/音频特征/声纹。
9. 判定只读计数与用例，不解析日志文本；日志只允许会话哈希前缀、片段引用、计数、原因码、耗时毫秒。
10. 声明口径：离线通过 ≠ G3 通过；替身通过 ≠ 真实端点已验证；不得出现"三平台真机稳定运行"。

### 0.4 阶段、关键路径与出口

```
P0（C-01/C-02/C-03 类型与纯函数，阻塞全部）
  └── P1（28 准入/切分/幂等）
        ├── P2（29 窗口/版本对/15s 守护/主题解析）
        ├── P3a（30 预筛/抽取/拆分/字段/兜底）
        │        └── P3b（31 相似度/窗口/三态/回溯）
        └── P3c（6 规则门禁 + 全量回归 + 回滚演练）
```

| 阶段 | 出口（全部满足才可进入下一阶段） |
|---|---|
| P0 | 3 条硬判据（见 §P0 出口）+ 依赖零新增 |
| P1 | AC-B3-01~12 全绿；`--rule=targets` 0；差集 `removed=0` |
| P2 | AC-B3-13~21 全绿；`--rule=source` 0 |
| P3a | AC-B3-22~34 全绿；`fallback.rs` diff=0；`truncated` 0 命中 |
| P3b | AC-B3-35~42 全绿；`0.87 不合并`成立 |
| P3c | AC-B3-43~48 全绿；442/0/9；`removed=0`；既有门禁全 0 |

### 0.5 任务总览

| 阶段 | 主任务范围 | 主任务数 | 子任务数 | 核心 AC |
|---|---|---|---|---|
| P0 | W-B3-1 ~ W-B3-8 | 8 | 30 | AC-02/03/04/06/09/10/16/17/18/25/27/29/30/31/39 |
| P1 | W-B3-9 ~ W-B3-19 | 11 | 35 | AC-B3-01~12、AC-44（局部） |
| P2 | W-B3-20 ~ W-B3-26 | 7 | 23 | AC-B3-13~21 |
| P3a | W-B3-27 ~ W-B3-34 | 8 | 30 | AC-B3-22~34 |
| P3b | W-B3-35 ~ W-B3-41 | 7 | 25 | AC-B3-35~42 |
| P3c | W-B3-42 ~ W-B3-47 | 6 | 24 | AC-B3-43~48 |
| **合计** | — | **47** | **167** | **48 条全覆盖** |

### 0.6 AC 覆盖索引

| 范围 | AC | 主任务落点 |
|---|---|---|
| 28 片段归一化与准入 | AC-B3-01~12 | W-B3-9~19（类型在 P0 W-B3-2/3） |
| 29 滚动摘要 | AC-B3-13~21 | W-B3-7（解析器）、W-B3-20~26 |
| 30 要点抽取 | AC-B3-22~34 | W-B3-27~34 |
| 31 去重与引用回溯 | AC-B3-35~42 | W-B3-35~41 |
| 全局兼容与红线 | AC-B3-43~48 | W-B3-42~47 |
| 合并侧承接 | AC-DD-08（只引用） | W-B3-13、W-B3-35 |

### 0.7 与 design 的唯一文件归属微调

- design 2.1.2 把 `parse_summary` 归入 `summary_guard.rs`；为满足"P0 必须先把 C-01 接口定死并可单测"的顺序约束，拆出同签名的纯函数模块 `crates/application/src/understand/summary_parser.rs`，P2 的 `summary_guard.rs` 只调用不重写。接口签名、确定性规则（P1~P7）、章节号均不变，仅文件归属变化，登记于此。

---

## P0 · 先决层：领域类型、策略、观测底座与 C-01 解析器

**目标**：把 C-01/C-02/C-03 三项裁决落成**可编译、可单测的纯类型与纯函数**；不写编排、不接线、不触发网关。
**前置**：无。
**出口（3 条硬判据，缺一不可）**：
1. `cargo test -p meeting-copilot-domain --lib` 退出 0，且既有 76 个 domain 用例名 0 变更。
2. `cargo fmt --all --check` 与 `cargo clippy --workspace --all-targets -- -D warnings` 均退出 0（0 error）。
3. 新增类型的边界单测全绿（策略 13 字段、切分 I1~I6、版本对单调、非空来源），且 `git diff --stat -- Cargo.toml Cargo.lock` 新增依赖行 = 0。

### W-B3-1 · 理解层策略 `InsightPolicy`

**W-B3-1.1 定义 13 字段与默认值** · `crates/domain/src/model/insight_policy.rs`
- 追溯：AC-B3-03 · AC-B3-06 · AC-B3-09 · AC-B3-13 · AC-B3-17 · AC-B3-19 · AC-B3-27 · AC-B3-35 · AC-B3-36 · spec 6.4-1~13 · design 2.2.2-B / 2.3.2
- DoD：13 字段名、类型与默认值逐字等于 spec 6.4（500 / 5000 / 800 / 2000 / 30000 / 20 / 15000 / 2 / 150 / 880 / 600000 / 8 / true）；`Default` 实现存在；文件内不出现 `MergePolicy`、`CapturePolicy`、`MIN_INSIGHT_CONFIDENCE`
- 证据：`cargo test -p meeting-copilot-domain --lib insight_policy_defaults_match_spec` 退出 0；`grep -cE "MergePolicy|CapturePolicy|MIN_INSIGHT_CONFIDENCE" crates/domain/src/model/insight_policy.rs` 输出 0

**W-B3-1.2 实现 `validate()` 的固定顺序与越界变体** · `crates/domain/src/model/insight_policy.rs`
- 追溯：spec 6.4-1~12 · design 2.2.2-B「validate 顺序固定（字段 1 → 13）」/ 2.3.2 校验边界表
- DoD：按字段 1→13 顺序返回首个越界；`insight_confidence_min_milli < 350` → `ConfidenceBelowSpeculativeBound`；`split_char_limit > 2000` → `SplitExceedsTextLimit`；`summary_retained_versions != 2` → `RetainedVersionsFixed`；其余越界 → `OutOfRange{field,value,min,max}`
- 证据：`cargo test -p meeting-copilot-domain --lib insight_policy_` 退出 0（含 `insight_policy_confidence_lower_bound`、`insight_policy_retained_versions_fixed`、`insight_policy_split_exceeds_text_limit`、`insight_policy_first_violation_wins`）

**W-B3-1.3 序列化契约与错误归属** · `crates/domain/src/model/insight_policy.rs`
- 追溯：spec 4.5-4 · design 2.2.2-B「`InsightPolicyError` 不进 `ErrorCode` 冻结集」
- DoD：结构标注 `deny_unknown_fields`，未知字段反序列化失败；13 字段 round-trip 相等；`InsightPolicyError` 不出现在 `crates/domain/src/error.rs`
- 证据：`cargo test -p meeting-copilot-domain --lib insight_policy_rejects_unknown_field` 退出 0；`grep -c "InsightPolicyError" crates/domain/src/error.rs` 输出 0

### W-B3-2 · 超长切分纯函数（C-02）

**W-B3-2.1 定义 `SplitChunk` / `SplitError` / 边界常量** · `crates/domain/src/model/utterance_text.rs`
- 追溯：AC-B3-09 · AC-B3-10 · spec 5.1.1-9~11 · design 2.0 C-02
- DoD：`SplitChunk{index,start_char,end_char,text,span}` 五字段；`SplitError` 三变体（`EmptyText`/`LimitOutOfRange`/`SpanTooShort`）；`PRIMARY_BREAKS` 与 `SECONDARY_BREAKS` 为全大写常量；`limit==0` 或 `limit>2000` 返回 `LimitOutOfRange`；空文本返回 `EmptyText`
- 证据：`cargo test -p meeting-copilot-domain --lib split_limits_are_enforced` 退出 0；`grep -c "const PRIMARY_BREAKS" crates/domain/src/model/utterance_text.rs` 输出 1；`grep -c "const SECONDARY_BREAKS"` 输出 1

**W-B3-2.2 实现切分算法主体** · `crates/domain/src/model/utterance_text.rs`
- 追溯：AC-B3-09 · AC-B3-11 · spec 5.1.1-9~11 · design 2.0 C-02 伪代码
- DoD：标量值 2000 为硬上限；在 `[start+1,end]` 内**从右向左**优先落一级句读之后，其次二级，均无则硬切；`cut<=start` 时回退 `end`；4001 字输出 3 段；每段 1..=2000；无边界文本按上限硬切
- 证据：`cargo test -p meeting-copilot-domain --lib split_` 退出 0（含 `split_4001_chars_into_three_chunks`、`split_without_punctuation_hard_cuts_at_limit`）

**W-B3-2.3 断言 I1~I6 与时间轴分配** · `crates/domain/src/model/utterance_text.rs`
- 追溯：AC-B3-09 · AC-B3-10 · spec 5.1.1-10 · design 2.0 C-02 I1~I6
- DoD：I1 逐段 `char_count()<=limit`；I2 拼接**精确等于**原文（不 trim）；I3 `seg[0].start_char==0`、`seg[i+1].start_char==seg[i].end_char`、末段 `end_char==total`；I4 首段 `span.start==原 start`、末段 `span.end==原 end`、逐段 `end>start`、整数比例分配且末段吸收残差；I6 同输入 100 次输出逐字节相同
- 证据：`cargo test -p meeting-copilot-domain --lib split_is_reversible_and_exact_concat split_spans_are_contiguous split_timeline_is_proportional split_is_deterministic_across_100_runs` 退出 0（分数条命令分别执行亦可）

**W-B3-2.4 边界语料与组合字符偏差可观测** · `crates/domain/src/model/utterance_text.rs`
- 追溯：AC-B3-11 · spec 5.1.3-3 · design 2.0 C-02「不柔化的边界」/ D-B3-01
- DoD：2000 / 2001 / 4001 / 全角标点 / 英文 / emoji ZWJ / 无标点 各 ≥1 用例；切片落在组合字符内部时仍满足 I1~I3；`split_grapheme_boundary` 由调用方计数（纯函数不写计数），偏离登记 D-B3-01
- 证据：`cargo test -p meeting-copilot-domain --lib split_` 退出 0；`grep -c "split_grapheme_boundary" crates/domain/src/model/utterance_text.rs` 输出 0（纯函数不持有计数）

### W-B3-3 · 分类兜底与去重依据（C-03）

**W-B3-3.1 实现全函数 `classify_kind`** · `crates/domain/src/model/insight_kind.rs`
- 追溯：AC-B3-27 · AC-B3-28 · spec 5.3.1-6/7 · design 2.1.3-3「四类分类兜底规则（封闭表）」
- DoD：`classify_kind(Option<&str>) -> KindClassification` 对任意输入返回四类之一；`key_info|decision|todo|question` 直映且 `fallback=false`；空串 / `None` / `note` / 大小写变体 → `KeyInfo` 且 `fallback=true`
- 证据：`cargo test -p meeting-copilot-domain --lib kind_classify_is_total` 退出 0；`grep -c "fn classify_kind" crates/domain/src/model/insight_kind.rs` 输出 1

**W-B3-3.2 断言映射表 6 行与 `kind_from_wire` 等价** · `crates/domain/src/model/insight_kind.rs`
- 追溯：AC-B3-27 · spec 5.3.1-7 · design 2.1.3-3 注释「映射表逐项等价」
- DoD：6 行（4 直映 + 2 兜底）逐项成立；对 `wire.rs:118-125` 的 4 个字面量与 1 个兜底分支给出等价断言（只断言行为，不调用私有函数）
- 证据：`cargo test -p meeting-copilot-domain --lib kind_fallback_table_matches_spec` 退出 0

**W-B3-3.3 定义 `DedupeBasis` 两值封闭枚举** · `crates/domain/src/model/insight_similarity.rs`
- 追溯：AC-B3-35 · spec 5.4.1-1 · design 2.0 C-03 / 2.2.2-C
- DoD：仅 `Identical` 与 `Cosine{milli:u16}` 两值；文件内不 import `classify_equivalence` / `PairBasis` / `MergePolicy`；`milli` 语义为千分位且 ≤1000
- 证据：`grep -cE "classify_equivalence|PairBasis|MergePolicy" crates/domain/src/model/insight_similarity.rs` 输出 0；`cargo test -p meeting-copilot-domain --lib dedupe_basis_is_closed` 退出 0

### W-B3-4 · 领域追加（只追加、带 serde 默认）

**W-B3-4.1 非空来源集合 `InsightSources`** · `crates/domain/src/model/insight.rs`
- 追溯：AC-B3-25 · AC-B3-30 · AC-B3-41 · spec 5.3.1-5 / 6.3-3 · design 2.3.2
- DoD：`new(vec![])` 返回 `Err(InsightError::MissingSource)`；`serde(try_from/into)` 使空数组反序列化失败；`with_parent` 去重保序追加；`len()/contains()/slice()` 存在
- 证据：`cargo test -p meeting-copilot-domain --lib insight_sources_` 退出 0（含 `insight_sources_reject_empty`、`insight_sources_serde_rejects_empty_array`、`insight_sources_with_parent_dedupes`）

**W-B3-4.2 `InsightDraft` 与运行期二次断言** · `crates/domain/src/model/insight.rs`
- 追溯：AC-B3-25 · AC-B3-29 · AC-B3-31 · spec 5.3.1-5/8 · design 2.3.2
- DoD：`validate()` 同时检查 `text.char_count()<=60`、`sources` 非空、待办时 `fields.todo.action.is_some()`；违者分别返回 `OversizeText{actual,limit}` / `MissingSource` / `TodoActionMissing`；草稿含 `kind_fallback: bool`
- 证据：`cargo test -p meeting-copilot-domain --lib insight_draft_validate_` 退出 0（含 `insight_draft_validate_rejects_oversize`、`insight_draft_validate_rejects_todo_without_action`）

**W-B3-4.3 `TodoFields` 追加 `action`** · `crates/domain/src/model/insight.rs`
- 追溯：AC-B3-31 · spec 5.3.1-11 · design 2.0 D-01
- DoD：`pub action: Option<ShortText>` 且 `#[serde(default)]`；`TodoFields::default()` 仍是 `{action:None,assignee:None,due:None}`；无 `action` 的旧 JSON 反序列化成功
- 证据：`cargo test -p meeting-copilot-domain --lib todo_fields_` 退出 0（含 `todo_fields_action_is_optional_for_old_payload`）

**W-B3-4.4 `Insight` 追加 `supplement_of`** · `crates/domain/src/model/insight.rs`
- 追溯：AC-B3-39 · spec 6.3-10 · design 2.0 D-01 / 2.3.2
- DoD：`pub supplement_of: Option<InsightRef>` 且 `#[serde(default)]`；旧 JSON 反序列化成功；既有字段类型/顺序/语义不变
- 证据：`cargo test -p meeting-copilot-domain --lib insight_old_payload_deserializes` 退出 0

**W-B3-4.5 既有 76 个 domain 用例名零变更** · `crates/domain/src/model/insight.rs`
- 追溯：AC-B3-02 · AC-B3-04 · AC-B3-38 · AC-B3-40 · spec 1.4-8 · design 2.8.2
- DoD：`low_confidence_is_filtered`、`placeholder_text_is_filtered`、`speaker_fallback_is_explicitly_unidentified`、`manual_edit_is_not_overwritten_by_merge`、`deleted_insight_cannot_merge`、`insight_requires_source` 等既有名逐字保留且通过
- 证据：`cargo test -p meeting-copilot-domain --lib` 退出 0；`grep -c "fn manual_edit_is_not_overwritten_by_merge" crates/domain/src/model/insight.rs` 输出 1；`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt` 输出含 `removed=0`

### W-B3-5 · 摘要版本模型

**W-B3-5.1 定义版本对与状态枚举** · `crates/domain/src/model/rolling_summary.rs`
- 追溯：AC-B3-16 · AC-B3-17 · AC-B3-18 · spec 6.2 · design 2.2.2-D / 2.3.2
- DoD：`SummaryVersionPair{current,previous:Option<RollingSummary>}`；`SummaryStatus` 六值 `Ok/Timeout/GatewayUnavailable/SchemaViolation/PublishBlocked/OutOfOrder`；`SummaryParse{topics,entities}`；`RollingSummary` 仍留在 `insight.rs`（D-B3-05），`topics` 非空约束不放宽
- 证据：`grep -c "pub struct RollingSummary" crates/domain/src/model/insight.rs` 输出 1；`grep -c "pub struct RollingSummary" crates/domain/src/model/rolling_summary.rs` 输出 0；`cargo test -p meeting-copilot-domain --lib summary_version_pair_` 退出 0

**W-B3-5.2 `publish` 强制版本单调与 covered 不回退** · `crates/domain/src/model/rolling_summary.rs`
- 追溯：AC-B3-16 · AC-B3-18 · spec 5.2.1-3/5 · design 2.2.2-D
- DoD：`next.version != current.version+1` → `Err(VersionNotMonotonic{expected,actual})`；`next.covered.start_ms < current.covered.start_ms` → `Err(CoveredRegressed)`；首个成功版本为 1；失败不改 `current`/`previous`
- 证据：`cargo test -p meeting-copilot-domain --lib summary_version_` 退出 0（含 `summary_version_must_be_dense`、`summary_covered_must_not_regress`、`summary_first_version_is_one`）

**W-B3-5.3 保留上限恒为 2** · `crates/domain/src/model/rolling_summary.rs`
- 追溯：AC-B3-17 · spec 5.2.1-4 / 6.2-5 · design 2.1.3-2
- DoD：第 4 次发布后 `current.version==4`、`previous.version==3`、`retained_versions()==2`，版本 ≤2 不可达；无第三槽位
- 证据：`cargo test -p meeting-copilot-domain --lib summary_retains_only_two_versions` 退出 0

### W-B3-6 · 观测底座

**W-B3-6.1 理解层计数（28 项 `u64` + 只增不减）** · `crates/application/src/understand/counters.rs`
- 追溯：AC-B3-08 · AC-B3-20 · AC-B3-21 · AC-B3-34 · AC-B3-43 · spec 6.5-1~27 · design 2.4（含 D-B3-04 补足第 28 项）
- DoD：27 项字段名与 spec 6.5 逐字一致；另含第 28 项 `split_span_degraded`（不占用派生值编号）；写入唯一入口 `bump(Counter,1)`，`Counter` 为封闭枚举（禁止字符串键）；`saturating_add`；`CounterSnapshot` 为 `Copy` 值拷贝
- 证据：`cargo test -p meeting-copilot-application --lib counters_are_monotonic_and_copyable` 退出 0；`grep -c "pub source_platform_mismatch: u64" crates/application/src/understand/counters.rs` 输出 1；`grep -c "split_span_degraded" crates/application/src/understand/counters.rs` 输出 1

**W-B3-6.2 派生值 `rule_fallback_rate`** · `crates/application/src/understand/counters.rs`
- 追溯：AC-B3-34 · spec 6.5-28 · design 2.4
- DoD：公式为 `rule_fallback_count / max(gateway_calls + rule_fallback_count, 1)`；分母为 0 时取 1；返回值落在 0..=1；不含浮点累加
- 证据：`cargo test -p meeting-copilot-application --lib rule_fallback_rate_formula` 退出 0（断言 4 次尝试 1 次兜底 ≈0.25，±0.001）

**W-B3-6.3 端口与仿真：时钟 / 开关 / 静音 / 术语快照** · `crates/application/src/understand/ports.rs`
- 追溯：AC-B3-21 · AC-B3-43 · spec 1.6 · design 2.2.2-E / 2.3.2
- DoD：`UnderstandClock` 同步 trait；`VirtualClock` 支持 `advance_ms`/`set_ms` 且只增；`UnderstandFeature` 默认 `Disabled`；`SilenceSignal::none()/from_intervals/interval_count`；`LexiconView::from_names` 去重、trim、字节序排序且 ≤64 项
- 证据：`cargo test -p meeting-copilot-application --lib virtual_clock_is_monotonic lexicon_view_is_deterministic silence_signal_none_is_empty` 退出 0

### W-B3-7 · C-01 主题解析器（应用层确定性解析）

**W-B3-7.1 定死 `parse_summary` 签名与 NoTopics 语义** · `crates/application/src/understand/summary_parser.rs`
- 追溯：AC-B3-18 · spec 5.2.1-6 / 6.2-3 · design 2.0 C-01（P1~P7）
- DoD：`parse_summary(text: &str, lexicon: &LexiconView) -> SummaryParse`；**不接收** `covered`/`version`；P1..P5 后 `topics` 仍为空时返回 `topics=[]`（调用方据此不发布）；签名单一入口
- 证据：`grep -c "fn parse_summary" crates/application/src/understand/summary_parser.rs` 输出 1；`cargo test -p meeting-copilot-application --lib parse_summary_` 退出 0

**W-B3-7.2 P1 分句与 80 字二级硬切** · `crates/application/src/understand/summary_parser.rs`
- 追溯：AC-B3-18 · design 2.0 C-01 P1
- DoD：按 `。！？；!?;\n` 切分并丢弃空段、段首尾 trim；单段 >80 标量值时按 `, . ， 、` 硬切到 ≤80；同一输入 100 次同结果
- 证据：`cargo test -p meeting-copilot-application --lib summary_parser_splits_sentences_deterministically` 退出 0

**W-B3-7.3 P2 术语命中 / P3 高频二元组回退** · `crates/application/src/understand/summary_parser.rs`
- 追溯：AC-B3-18 · design 2.0 C-01 P2/P3
- DoD：命中术语的句子中所有命中词为候选，权重基数为命中次数；无术语命中的句子取 `normalize_text` 后长度 2..=12、出现 ≥2 次的连续子串中最高频者 1 个；不引入分词库
- 证据：`cargo test -p meeting-copilot-application --lib summary_parser_prefers_lexicon_hits summary_parser_falls_back_to_frequent_bigrams` 退出 0

**W-B3-7.4 P4/P5 三级全序排序、截断与权重** · `crates/application/src/understand/summary_parser.rs`
- 追溯：AC-B3-18 · AC-B3-11（同输入同输出精神） · design 2.0 C-01 P4/P5
- DoD：排序为 `count` 降序 → 首次出现位置升序 → 字节字典序升序（三级全序，不依赖 HashMap 迭代序）；`topics` 取前 5；`weight = count as f64 / total_count as f64`（整数计数相除，不累加浮点）
- 证据：`cargo test -p meeting-copilot-application --lib summary_topics_order_is_total_and_stable summary_topics_are_truncated_to_five` 退出 0

**W-B3-7.5 P6 实体抽取 / P7 空主题** · `crates/application/src/understand/summary_parser.rs`
- 追溯：AC-B3-18 · spec 6.2-4 · design 2.0 C-01 P6/P7
- DoD：实体候选 = ASCII `[A-Za-z0-9_-]{2,}` ∪ `LexiconView` 命中词，保留在 ≥2 句出现者，`normalized=normalize_text`，同名去重保序，取前 10；全空白/全标点/单字文本返回 `topics=[]`
- 证据：`cargo test -p meeting-copilot-application --lib summary_entities_are_normalized_and_deduped summary_without_topics_returns_empty` 退出 0

### W-B3-8 · P0 出口门禁

**W-B3-8.1 domain lib 全绿与既有名零变更** · `crates/domain/src/model/**`
- 追溯：AC-B3-04 · AC-B3-44 · spec 4.5-1 · design 2.7.1 P0 DoD
- DoD：`cargo test -p meeting-copilot-domain --lib` 退出 0；基线中 domain 的 76 个 `pkg::lib::name` 全部仍在
- 证据：上述命令；`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt 2>&1 | grep -q "removed=0"` 退出 0

**W-B3-8.2 格式与静态检查** · 全仓库
- 追溯：design 2.7.1 P0 DoD / 2.9.3
- DoD：`cargo fmt --all --check` 退出 0；`cargo clippy --workspace --all-targets -- -D warnings` 退出 0（0 error）
- 证据：上述两条命令（前置 §0.2 的 `RUSTFMT`/PATH）

**W-B3-8.3 依赖零新增与新增用例名唯一** · `Cargo.toml` / `Cargo.lock` / 新增文件
- 追溯：AC-B3-47 · spec 1.4-5 · design 2.5.2
- DoD：`Cargo.toml`/`Cargo.lock` 无新增依赖行；`meeting-copilot-application` 仅依赖 `meeting-copilot-domain`；每个新增 `#[test]` 名 `grep -c` 唯一（1）
- 证据：`git diff --stat -- Cargo.toml Cargo.lock` 输出为空；`cargo tree -p meeting-copilot-application --edges normal --prefix none | grep -cE "infrastructure|platform-(win|mac)|reqwest|hyper|sqlx|native-tls|tauri"` 输出 0；`grep -c "fn insight_policy_defaults_match_spec" crates/domain/src/model/insight_policy.rs` 输出 1

**W-B3-8.4 回滚锚点 R0（领域类型独立可回退）** · `crates/domain/src/model/insight_policy.rs`
- 追溯：design 2.7.3 第一行 · 附录 A 回滚单元
- DoD：删除 `ConfidenceBelowSpeculativeBound` 分支后单测 `insight_policy_confidence_lower_bound` 必须失败；恢复后必须 0 失败；演练只记录，不改基线
- 证据：演练后 `cargo test -p meeting-copilot-domain --lib insight_policy_confidence_lower_bound` 先 1 后 0；结论写入 `docs/progress/b3-status.md`（W-B3-47.1）

---

## P1 · 28 片段归一化与准入

**目标**：把 `MergedUnit` 经"状态 → 来源/平台 → 说话人 → 幂等 → 硬/软准入 → 静音 → 空段 → 2000 切分"串成唯一入口，产出 `Admitted{fragments}` 与 12 类计数。
**前置**：P0。
**出口**：AC-B3-01~12 全绿；4 个新 target 已登记；既有 capture/merge 用例名零变更。

### W-B3-9 · 门面骨架（唯一入口）

**W-B3-9.1 入口/结论/原因码类型** · `crates/application/src/understand/types.rs`
- 追溯：spec 1.5-1~5 · design 2.2.2-A / 2.3.2
- DoD：`UnderstandInput{session,unit,state,silence}`；`UnderstandOutcome` 五变体 `Disabled/Duplicate/Rejected/StoredOnly/Admitted`；`RejectReason` 六值 `Paused/SourcePlatformMismatch/Speculative/Silence/Empty/LowConfidence`；`UnderstandInitError` 不进 `ErrorCode`
- 证据：`cargo test -p meeting-copilot-application --lib understand_types_` 退出 0；`grep -c "UnderstandOutcome" crates/domain/src/error.rs` 输出 0

**W-B3-9.2 `UnderstandService::new` 与 feature 直通** · `crates/application/src/understand/service.rs`
- 追溯：AC-B3-08 · spec 4.2-1 · design 2.2.2-A / 2.5.1
- DoD：`new` 只校验 `policy.validate()`；`UnderstandFeature::Disabled` 时 `observe` 返回 `Disabled` 且零网关调用、零计数、零对象
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize disabled_feature_is_zero_effect` 退出 0

**W-B3-9.3 `observe` 串行编排骨架（P1 只到准入）** · `crates/application/src/understand/service.rs`
- 追溯：AC-B3-12 · spec 5.1.1-12 · design 2.1.3-1「判定顺序固定」
- DoD：单线程串行；返回顺序与调用顺序一致；`observe` 不返回 `Result`、不 panic、不等待睡眠；P1 阶段下游（摘要/抽取/去重）以空实现占位且不改变 `Admitted` 语义
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize admission_preserves_input_order` 退出 0；`grep -cE "thread::sleep|Instant::now" crates/application/src/understand/service.rs` 输出 0

**W-B3-9.4 只读视图与"不接入 capture"** · `crates/application/src/understand/service.rs`
- 追溯：spec 1.4-1 / 1.5-1 · design 2.5.1
- DoD：`counters()`/`insights(session)`/`summary_view(session)` 返回值拷贝；`crates/application/src/capture/**` 不出现 `UnderstandService`；`with_merge`/`ingest_text`/`advance_to` 零改动
- 证据：`grep -rc "UnderstandService" crates/application/src/capture | grep -v ":0"` 无输出；`git diff --quiet HEAD -- crates/application/src/capture/service.rs` 退出 0

### W-B3-10 · 会话状态裁决（AC-07/08）

**W-B3-10.1 `Paused`/`Deleted` 丢弃并计 `paused_drop`** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-07 · spec 5.1.1-7 / 5.1.3 · tasks 28.2
- DoD：`Paused` 下连续 3 条输入 → `paused_drop==3`、草稿 0、`gateway_calls==0`、`admitted==0`；`Deleted` 同路径
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize paused_session_drops_all_fragments` 退出 0

**W-B3-10.2 `Silent` 照常记录且可外发恒 0** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-08 · spec 5.1.1-8 / 4.3-5 · design 2.1.3-1「Silent 不停机」
- DoD：`Silent` → `Admitted` 且 `admitted>=1`；`dispatch_candidates==0`（结构上无 `dispatchable` 字段）；不产生任何外发/推送标记
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize silent_period_still_records_without_dispatch` 退出 0

### W-B3-11 · 来源与平台一致性（AC-01）

**W-B3-11.1 平台通道必填且等于会话绑定平台** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-01 · spec 5.1.1-1a / 5.1.3-1
- DoD：`PlatformSubtitle`/`PlatformAudio` 且 `platform.is_none()` 或 `!= session.platform` → `Rejected(SourcePlatformMismatch)`；不产生子片段
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize platform_subtitle_requires_matching_platform` 退出 0

**W-B3-11.2 麦克风/外部转写必须无平台** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-01 · spec 5.1.1-1b / 6.1-6
- DoD：`Microphone`/`ExternalTranscription` 且 `platform.is_some()` → 同一拒绝路径
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize microphone_rejects_platform_binding` 退出 0

**W-B3-11.3 计数与诊断原因码（不含原文）** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-01 · spec 4.4-2 · design 2.1.3-1
- DoD：两次不一致后 `source_platform_mismatch==2`；诊断项只含原因码与计数，不含片段文本/说话人/端点
- 证据：两用例连续执行后断言 `snapshot().source_platform_mismatch==2`；`grep -cE "text|speaker|url|model" crates/application/src/understand/normalize.rs` 不作为判据，改由 `--rule=zero-transcript`（P3c）兜底

### W-B3-12 · 说话人兜底（AC-02）

**W-B3-12.1 无标识 → "未知说话人" + `identified=false`** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-02 · spec 5.1.1-2 / 4.3-3 / 6.1-4
- DoD：缺说话人或 `label` 去空白为空 → `SpeakerRef::unknown()`；不得生成任何姓名
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize missing_speaker_becomes_unknown_and_unidentified` 退出 0

**W-B3-12.2 既有兜底用例与语义保留** · `crates/domain/src/model/utterance.rs`
- 追溯：AC-B3-02 · design 2.8.2-3
- DoD：`SpeakerRef::unknown()` 与 `speaker_fallback_is_explicitly_unidentified` 逐字不变；domain lib 全绿
- 证据：`grep -c "fn speaker_fallback_is_explicitly_unidentified" crates/domain/src/model/utterance.rs` 输出 1；`cargo test -p meeting-copilot-domain --lib` 退出 0

### W-B3-13 · 幂等与顺序（AC-12 / AC-42）

**W-B3-13.1 会话内幂等键集合** · `crates/application/src/understand/service.rs`
- 追溯：AC-B3-12 · AC-B3-42 · spec 1.5-2 / 5.1.1-12 · AC-DD-08
- DoD：键为 `(SessionId, UtteranceRef)`；首次插入成功才继续；跨会话同 ref 不互相抑制；子片段派生 ref 也登记
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize duplicate_delivery_is_idempotent` 退出 0

**W-B3-13.2 判定顺序：幂等先于置信度** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-12 · design 2.1.3-1「为什么幂等放在置信度之前」
- DoD：同一 ref 第二次投递 → `Duplicate` + `duplicate_delivery_ignored==1`，且 `skipped_low_confidence` 不增；`Admitted` 仅 1 组
- 证据：断言 `duplicate_delivery_ignored==1 && skipped_low_confidence==0`；`cargo test -p meeting-copilot-application --test understanding_normalize duplicate_low_confidence_counts_duplicate` 退出 0

**W-B3-13.3 批量顺序保持** · `crates/application/src/understand/service.rs`
- 追溯：AC-B3-12 · spec 5.1.1-12 · design 1.1.3-A「顺序保持」
- DoD：20 条批量投递 → 输出 `fragments` 的 reference 序列与输入序列逐项相等；无并行重排序
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize admission_preserves_input_order` 退出 0

### W-B3-14 · 两步置信度门槛（AC-03/04）

**W-B3-14.1 硬准入复用既有 `is_eligible_for_insight`** · `crates/application/src/understand/low_confidence.rs`
- 追溯：AC-B3-04 · spec 1.6-1 · design 1.2.1
- DoD：`!unit.utterance.is_eligible_for_insight()` → `Rejected(Speculative)` + `speculative_skip+=1`；**不得**自写 `<0.35` 判定
- 证据：`grep -c "is_eligible_for_insight" crates/application/src/understand/low_confidence.rs` 输出 1；`grep -c "0.35" crates/application/src/understand/low_confidence.rs` 输出 0

**W-B3-14.2 软准入 `[350,500)` 只落盘** · `crates/application/src/understand/low_confidence.rs`
- 追溯：AC-B3-03 · spec 1.6-2 / 5.1.1-4 / 6.1-5 · tasks 28.4
- DoD：`confidence_milli ∈ [350,500)` → `StoredOnly{LowConfidence}` + `skipped_low_confidence+=1`；0 条草稿；不触发任何外发
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize below_half_confidence_is_stored_not_extracted` 退出 0

**W-B3-14.3 边界 0.50 放行且既有常量零改动** · `crates/application/src/understand/low_confidence.rs`
- 追溯：AC-B3-03 · AC-B3-04 · spec 1.6-3 · design 2.5.2
- DoD：`confidence==0.50` → `Admitted` 且 `admitted>=1`；`MIN_INSIGHT_CONFIDENCE: f64 = 0.35` 全仓库唯一命中
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize exactly_half_confidence_passes` 退出 0；`grep -rc "MIN_INSIGHT_CONFIDENCE: f64 = 0.35" crates | grep -v ":0" | wc -l` 输出 1

### W-B3-15 · 静音 / 空段 / 静默期信号（AC-05/06）

**W-B3-15.1 静音区间完全覆盖 → 丢弃** · `crates/application/src/understand/silence_filter.rs`
- 追溯：AC-B3-05 · spec 5.1.1-5 / 5.1.3-2
- DoD：静音区间 6000 ms 覆盖片段区间 → `silence_dropped+=1`、`admitted==0`、`store.insights(session)` 为空
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize silence_window_covering_fragment_is_dropped` 退出 0

**W-B3-15.2 部分重叠阈值（800‰）与部分重叠计数** · `crates/application/src/understand/silence_filter.rs`
- 追溯：AC-B3-05 · spec 5.1.3-2b · design 2.1.3-1「整数比较，避免浮点」
- DoD：`overlap_ms*1000/span_ms >= 800` → `silence_dropped+=1`；`0 < ratio < 800` → 准入并 `silence_partial_overlap+=1`；`overlap==0` 不计数
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize silence_partial_overlap_is_counted` 退出 0

**W-B3-15.3 `<300 ms` 且无实质文本 → 空段** · `crates/application/src/understand/silence_filter.rs`
- 追溯：AC-B3-05 · spec 5.1.1-5 / 5.1.3-4 · design 2.1.3-1
- DoD：时长 <300 ms 且去空白后为空/仅标点 → `Rejected(Empty)` + `empty_chunk_dropped+=1`；同一计数也用于切分后空段（W-B3-16.4），两处语义在注释中写明
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize short_punctuation_only_fragment_is_dropped` 退出 0

**W-B3-15.4 正常停顿不误判 + 状态上报无文本** · `crates/application/src/understand/silence_filter.rs`
- 追溯：AC-B3-06 · spec 5.1.1-6 / 4.4-2 · design 2.6.1
- DoD：两片段间隔 400 ms 且均有实质文本 → 两条均 `Admitted`、`silence_dropped==0`；"静音中"状态记录序列化键集为 `{silent,since_ms,dropped}`（无文本键）；既有 `silence_requires_three_blocks_and_no_vad` 逐字保留
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize short_pause_is_not_counted_as_silence silence_status_is_reported_without_text` 退出 0；`grep -rc "fn silence_requires_three_blocks_and_no_vad" crates | grep -v ":0" | wc -l` 输出 1

### W-B3-16 · 切分接入与子片段派生（AC-09/10/11）

**W-B3-16.1 `NormalizedFragment` 投影与 ref 派生规则** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-10 · spec 5.1.1-10 · design 2.0 C-02 I5 / 2.0 D-02
- DoD：子片段 `reference` 为 `"{parent}#{i}"`，`i==0` 沿用 parent 原值；`session/speaker(含 identified)/source/platform/dual_channel/origin_pair/reading_mode/confidence` 逐字段复制；不产生新 `SpeakerRef`
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize split_is_reversible_and_preserves_attributes` 退出 0

**W-B3-16.2 策略上限接入与 `SpanTooShort` 退化** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-09 · spec 6.4-4 · design 2.2.2-C「SpanTooShort 处置」/ 2.3.2
- DoD：`limit = policy.split_char_limit`；`SpanTooShort` 时不切分、整段作为 1 条并 `split_span_degraded+=1`（不新增 `ErrorCode`、不丢片段）
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize split_span_too_short_degrades_to_single_fragment` 退出 0

**W-B3-16.3 `admitted` 口径与 ref→session 映射写入** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-09 · spec 6.5-1 · design 2.0 D-02 / 2.3.2 对象生命周期
- DoD：`admitted += chunks.len()`（按子片段计）；准入时一次性写入全部派生 ref 的 `ref→session` 映射，避免前缀解析歧义；映射在 `observe` 返回后仍可查
- 证据：断言 4001 字单条输入后 `admitted==3` 且 `store.session_of("{parent}#2")` 有值；`cargo test -p meeting-copilot-application --test understanding_normalize admitted_counts_split_chunks` 退出 0

**W-B3-16.4 切分后空段丢弃但不丢整条** · `crates/application/src/understand/normalize.rs`
- 追溯：AC-B3-09 · spec 5.1.3-5 · design 2.1.3-1
- DoD：某子段去空白后为空/仅标点 → 该段 `empty_chunk_dropped+=1` 且不进提炼；其余子段正常；父单元不被整体丢弃
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize empty_chunk_is_dropped_independently` 退出 0

### W-B3-17 · `understanding_normalize` target（AC-01~12）

**W-B3-17.1 替身与夹具（只记录 `(task,purpose)`）** · `crates/application/tests/support/understand_fakes.rs`
- 追溯：AC-B3-01~12 · design 2.6.1「替身装配」/ 2.1.3-6 纪律 3
- DoD：`StubGateway` 记录 `(task,purpose)` 与调用次数，`model` 固定 `"stub"`；`single(text,span,conf,source,platform,speaker)` 与 `merged(sub,asr,pairing)` 夹具；`VirtualClock`；不写 `request_id`/`http_status` 到任何对象
- 证据：`grep -c "request_id\|http_status" crates/application/tests/support/understand_fakes.rs` 输出 0；`grep -c "\"stub\"" crates/application/tests/support/understand_fakes.rs` 输出 ≥1

**W-B3-17.2 12 条 AC 用例逐条落地** · `crates/application/tests/understanding_normalize.rs`
- 追溯：AC-B3-01~12 · design 2.6.1 全表
- DoD：用例名逐字：`platform_subtitle_requires_matching_platform`、`microphone_rejects_platform_binding`、`missing_speaker_becomes_unknown_and_unidentified`、`below_half_confidence_is_stored_not_extracted`、`exactly_half_confidence_passes`、`silence_window_covering_fragment_is_dropped`、`short_pause_is_not_counted_as_silence`、`silence_status_is_reported_without_text`、`paused_session_drops_all_fragments`、`silent_period_still_records_without_dispatch`、`overlong_text_splits_with_contiguous_spans`、`split_is_reversible_and_preserves_attributes`、`duplicate_delivery_is_idempotent`、`admission_preserves_input_order` 全部存在并通过
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize` 退出 0；上述 14 个名字逐个 `grep -c` 均 1

**W-B3-17.3 512 条语料与 100 次确定性（AC-11）** · `crates/application/tests/understanding_normalize.rs`
- 追溯：AC-B3-11 · spec 5.1.1-11 · design 2.6.1
- DoD：512 条固定语料（CJK/英文/全角标点/emoji/无标点/2000/2001/4001）切分失败数 0；同一 4001 字文本连续 100 次得到完全相同子片段组与边界
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize split_succeeds_on_fixed_corpus split_is_deterministic_across_repeats` 退出 0

**W-B3-17.4 分支审计用例：每个拒绝路径都留计数** · `crates/application/tests/understanding_normalize.rs`
- 追溯：spec 4.2-4 · design 2.4「异常路径也必须留码」
- DoD：`every_rejection_path_bumps_a_counter` 覆盖 Paused / SourcePlatformMismatch / Speculative / Silence / Empty / LowConfidence / Duplicate 七条路径，每条 `snapshot()` 对应计数 ≥1
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize every_rejection_path_bumps_a_counter` 退出 0

### W-B3-18 · `NEW_TARGETS` 登记与用例名差集

**W-B3-18.1 一次性把 4 项追加到 `NEW_TARGETS`（6→10）** · `scripts/test-name-inventory.sh`
- 追溯：AC-B3-44 · spec 1.7-1/2 · design 2.8.2-1
- DoD：第 18 行 `NEW_TARGETS` 在既有 6 项**末尾**追加 `understanding_normalize rolling_summary insight_extract insight_dedup`，顺序固定；不做多行拆分、不改既有 6 项
- 证据：`grep -n "NEW_TARGETS=" scripts/test-name-inventory.sh` 输出单行且含 10 项；`grep -c "merge_dedupe understanding_normalize rolling_summary insight_extract insight_dedup" scripts/test-name-inventory.sh` 输出 1

**W-B3-18.2 target 数 40→44 与基线只读** · `crates/application/tests/**` / `docs/progress/test-names-before.txt`
- 追溯：spec 1.7-1 · design 2.8.2-2/3
- DoD：新增 4 个 `tests/*.rs` 文件即新增 4 个 target（40→44）；`docs/progress/test-names-before.txt` 365 行零改动；新增用例计入 `added`、不计入通过判定
- 证据：`ls crates/application/tests/*.rs | wc -l` 输出 16；`git diff --quiet HEAD -- docs/progress/test-names-before.txt` 退出 0；`wc -l docs/progress/test-names-before.txt` 输出 365

**W-B3-18.3 差集 `removed=0`** · `scripts/test-name-inventory.sh`
- 追溯：AC-B3-44 · AC-B3-45 · spec 4.5-1 · design 2.8.3
- DoD：`--diff` 退出 0 且输出含 `removed=0`；`added` 为 4 个新 target 的用例数之和（不参与判定）；不得通过改基线规避
- 证据：`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt` 退出 0 且 `grep "removed=0"`

### W-B3-19 · P1 出口

**W-B3-19.1 12 条 AC 与既有 capture/merge 名零变更** · 全仓库
- 追溯：AC-B3-01~12 · AC-B3-44 · spec 1.4-8
- DoD：`understanding_normalize` 全绿；`merge_dedupe`、`capture_*` 既有用例名与行为不变
- 证据：`cargo test -p meeting-copilot-application --test understanding_normalize` 退出 0；`cargo test -p meeting-copilot-application --test merge_dedupe` 退出 0

**W-B3-19.2 lib 单测、fmt、clippy** · 全仓库
- 追溯：design 2.9.3
- DoD：`cargo test -p meeting-copilot-domain --lib`、`cargo test -p meeting-copilot-application --lib`、`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings` 均退出 0
- 证据：四条命令

**W-B3-19.3 回滚锚点 R1（幂等顺序错位）** · `crates/application/src/understand/normalize.rs`
- 追溯：design 2.7.3「P1 把幂等判断挪到置信度判定之后」
- DoD：把幂等判定挪到软准入之后 → `duplicate_delivery_is_idempotent` 必须失败（计成 `skipped_low_confidence`）；恢复后 0 失败
- 证据：演练记录写入 `docs/progress/b3-status.md`（W-B3-47.1）

---

## P2 · 29 滚动摘要

**目标**：实现"30 s 或 20 条先到者触发 → 只压缩增量 + 上一版 → 版本严格单调 → 仅保留 2 版 → 15 s 超时丢弃且不阻塞准入 → topics 为空不发布"。
**前置**：P1（需要 `UnderstandService`、`admitted` 与 `VirtualClock`）。
**出口**：AC-B3-13~21 全绿；`--rule=source` 对 `understand/**` 0 命中；P1 的 12 条用例与既有名不回退。

### W-B3-20 · 摘要窗口与触发

**W-B3-20.1 `SummaryWindow` 消费式窗口** · `crates/application/src/understand/rolling_summary.rs`
- 追溯：AC-B3-15 · spec 5.2.1-2 · design 2.1.3-2「触发即 clear」
- DoD：`accumulate(ref,span,now_ms)` O(1) 追加；`take(now_ms)` 返回当前 pending 并清空；触发后同一窗口不可二次取用
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary window_take_is_consuming` 退出 0

**W-B3-20.2 双触发条件（20 条或 30 s，先到者）** · `crates/application/src/understand/rolling_summary.rs`
- 追溯：AC-B3-13 · AC-B3-14 · spec 5.2.1-1 · tasks 29.1
- DoD：`pending>=20` 或 `elapsed>=30000` 且 `inflight==false` 才触发；19 条且 <30 s 不触发；30 s 且无新片段按策略仍触发一次（`flush`/周期语义），推进一步 0 ms 不重复
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary refresh_triggers_at_thirty_seconds_once refresh_triggers_at_twenty_utterances nineteen_utterances_do_not_trigger` 退出 0

**W-B3-20.3 `covered` 计算（排序后首起末止）** · `crates/application/src/understand/rolling_summary.rs`
- 追溯：AC-B3-18 · spec 5.2.1-5 / 5.2.3-4 · design 2.1.3-2
- DoD：delta 按 `span.start_ms` 稳定排序后取首段 start、末段 end；乱序输入不在此处丢弃（交由 W-B3-24.3 计 `summary_out_of_order`）
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary covered_span_equals_delta_window` 退出 0

### W-B3-21 · 增量请求构造

**W-B3-21.1 只提交"新增片段引用 + 上一版主题"** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-15 · spec 5.2.1-2 · design 1.1.3-B
- DoD：请求载荷只含本次 delta（≤20 条 ref）与上一版 `topics`（版本号随请求）；不含已压缩旧片段；第 3 次刷新 7 条新增时 `utterance_refs.len()==7` 且 `previous.is_some()`
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary incremental_refresh_consumes_delta_plus_previous` 退出 0

**W-B3-21.2 请求元数据：`Summary` 任务与 ≤8 s 网关预算** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-19 · spec 6.4-7 · design 2.1.3-2 / 2.2.2-D
- DoD：`task=Summary`、`purpose=RollingWindow`；网关 `DeadlineMs = min(summary_timeout_ms,8000)`；**不修改** `DeadlineMs::MAX`；15 s 由应用层记账
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary gateway_deadline_is_capped_at_eight_seconds` 退出 0；`grep -c "DeadlineMs::MAX" crates/application/src/understand/summary_guard.rs` 输出 0

**W-B3-21.3 不读模型/请求/状态元数据** · `crates/application/src/understand/summary_guard.rs`
- 追溯：spec 4.3-1 · design 1.2.4 / 2.1.3-6 纪律 3
- DoD：只读 `outcome.body` 与 `outcome.failure/degrade_cause`；不读 `model`/`request_id`/`http_status`/`tokens_used`；不写入任何对象
- 证据：`grep -cE "\.model|request_id|http_status|tokens_used" crates/application/src/understand/summary_guard.rs` 输出 0

### W-B3-22 · 串行状态机与不阻塞

**W-B3-22.1 每会话状态位** · `crates/application/src/understand/rolling_summary.rs`
- 追溯：AC-B3-13 · AC-B3-20 · spec 5.2.3-3 · design 2.1.3-2 状态机
- DoD：`SessionSummaryState{pair:Option<SummaryVersionPair>, inflight:Option<InflightRefresh>, status:Option<SummaryStatus>}`；同一会话刷新串行（单通道），不同会话互不阻塞
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary same_session_refresh_is_serialized` 退出 0

**W-B3-22.2 同窗口不重复刷新（Inflight 互斥）** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-13 · design 2.1.3-2「清空 + Inflight 互斥双重保证」
- DoD：触发后进入 `Inflight` 并清空 pending；未完成前再次满足条件不产生第二次刷新；完成后回到 `Idle`
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary refresh_triggers_at_thirty_seconds_once` 退出 0（推进 30000 ms 后 `summary_refresh_ok==1`，再推 0 ms 仍为 1）

**W-B3-22.3 准入路径不等待摘要** · `crates/application/src/understand/service.rs`
- 追溯：AC-B3-20 · AC-B3-21 · spec 5.2.1-9 · design 2.1.3-2「不阻塞」
- DoD：`observe` 只做 `accumulate` + `maybe_trigger`（O(1)）；准入与刷新无共享锁等待；超时期间同一时段 `admitted>=1`
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary timeout_does_not_block_admission` 退出 0

### W-B3-23 · 15 s 守护与失败分类

**W-B3-23.1 发起前 deadline 记账** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-19 · spec 5.2.1-8 / 6.4-7 · design 2.1.3-2
- DoD：`deadline_ms = clock.now_millis() + policy.summary_timeout_ms`，在调用网关**之前**写入 `Inflight`；返回后 `now > deadline` 即超时
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary timeout_boundary_is_exclusive` 退出 0（15000 ms 内成功，15001 ms 超时）

**W-B3-23.2 超时丢弃、版本不变、计数 +1** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-19 · spec 5.2.1-8 / 5.2.3-1
- DoD：`summary_timeout+=1`；`current.version` 与 `previous` 不变；结果丢弃；不重试、不排队补做；用户侧无阻塞提示
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary timeout_discards_and_keeps_previous` 退出 0

**W-B3-23.3 失败按原因分列（不可用/Schema/冷却）** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-20 · spec 5.2.1-10 / 5.2.3-5 · design 2.2.2-D `SummaryStatus`
- DoD：`capability().endpoint_configured==false` 或 `cooling` → 不发起调用，`summary_failed_gateway_unavailable+=1`；Schema 违规 → `schema_violation+=1` 且不改版本；三类互斥
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary summary_failures_are_classified_by_cause` 退出 0

**W-B3-23.4 无 sleep、无真实计时、无 panic** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-43 · spec 4.2-1 · design 2.1.3-6 纪律 1
- DoD：目录内 `Instant::now`/`SystemTime::now`/`thread::sleep`/`panic!`/`todo!`/`unimplemented!` 命中 0；超时由虚拟时钟驱动
- 证据：`grep -rcE "Instant::now|SystemTime::now|thread::sleep|panic!\(|todo!\(|unimplemented!\(" crates/application/src/understand | grep -v ":0"` 无输出

### W-B3-24 · 发布裁决与版本

**W-B3-24.1 成功发布：version+1、旧当前版降为上一版** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-16 · spec 5.2.1-3 · design 2.2.2-D「publish 是唯一改版本号入口」
- DoD：成功且 `topics` 非空 → `pair.publish(next)`；`summary_refresh_ok+=1`；`status=Ok`
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary version_is_monotonic_and_dense` 退出 0（`versions==[1,2,3]`）

**W-B3-24.2 `topics` 为空不发布（C-01 处置）** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-18 · spec 5.2.1-6 / 5.2.3-2 / 6.2-3 · design 2.0 C-01「失败处置固定」
- DoD：`parse_summary` 返回空 `topics` → **不发布**、版本不变、沿用上一版、`summary_publish_blocked+=1`；**不得**用"会议"等兜底主题填充
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary empty_topics_blocks_publish` 退出 0；`grep -cE "\"会议\"|fallback_topic" crates/application/src/understand/summary_guard.rs` 输出 0

**W-B3-24.3 覆盖区间回退 → 乱序不发布** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-18 · spec 5.2.3-4 · design 2.1.3-2「乱序」
- DoD：排序后 `start < previous.covered.start_ms` → 不发布 + `summary_out_of_order+=1`；`covered.start >= previous.start` 正常发布
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary out_of_order_window_is_not_published` 退出 0

**W-B3-24.4 保留 2 版与只读视图** · `crates/application/src/understand/summary_guard.rs`
- 追溯：AC-B3-17 · spec 4.1-5 · design 2.2.2-A `SummaryVersionView`
- DoD：第 4 次刷新后 `current.version==4`、`previous.version==3`、`retained_versions()==2`；`summary_view` 只含 `version/previous_version/covered/topics_len/entities_len/status`
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary only_two_versions_are_retained` 退出 0

### W-B3-25 · `rolling_summary` target（AC-13~21）

**W-B3-25.1 `GatewayScript` 替身（延迟/结果/故障/能力）** · `crates/application/tests/support/understand_fakes.rs`
- 追溯：AC-B3-19 · AC-B3-20 · design 2.6.2「替身装配」
- DoD：可设 `delay_reported_ms` / `body` / `failure` / `degrade_cause` / `capability().endpoint_configured=false`；不引入真实计时（延迟通过虚拟时钟报告，不 sleep）
- 证据：`grep -cE "thread::sleep|Instant::now" crates/application/tests/support/understand_fakes.rs` 输出 0；`cargo test -p meeting-copilot-application --test rolling_summary` 退出 0

**W-B3-25.2 9 条 AC 用例逐条落地** · `crates/application/tests/rolling_summary.rs`
- 追溯：AC-B3-13~21 · design 2.6.2 全表
- DoD：`refresh_triggers_at_thirty_seconds_once`、`refresh_triggers_at_twenty_utterances`、`nineteen_utterances_do_not_trigger`、`incremental_refresh_consumes_delta_plus_previous`、`version_is_monotonic_and_dense`、`only_two_versions_are_retained`、`covered_span_equals_delta_window`、`empty_topics_blocks_publish`、`timeout_discards_and_keeps_previous`、`timeout_does_not_block_admission`、`summary_failures_are_classified_by_cause`、`hour_long_session_refreshes_at_least_120_times` 全部存在并通过
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary` 退出 0；12 个名字 `grep -c` 均 1

**W-B3-25.3 1 小时会话 ≥120 次刷新（虚拟时钟）** · `crates/application/tests/rolling_summary.rs`
- 追溯：AC-B3-21 · spec 4.1-3
- DoD：以 30 s 步长推进 3600 s（120 次触发），`summary_refresh_ok>=120`；全程无真实 `Instant::now`/`thread::sleep`
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary hour_long_session_refreshes_at_least_120_times` 退出 0

**W-B3-25.4 源码无真实计时（脚本侧留待 P3c 同口径）** · `crates/application/src/understand/**`
- 追溯：AC-B3-21 · AC-B3-43 · spec 7.7 `--rule=source`
- DoD：P2 结束时先以 grep 自查 `Instant::now`/`thread::sleep` 命中 0；P3c 由脚本固化
- 证据：`grep -rcE "Instant::now|thread::sleep" crates/application/src/understand | grep -v ":0"` 无输出

### W-B3-26 · P2 出口

**W-B3-26.1 9 条 AC 与 P1 不回退** · 全仓库
- 追溯：AC-B3-13~21 · AC-B3-01~12
- DoD：`rolling_summary` 与 `understanding_normalize` 均全绿；`summary_refresh_ok` 与 `admitted` 在同一时段可同时增长
- 证据：`cargo test -p meeting-copilot-application --test rolling_summary --test understanding_normalize` 退出 0（分两条执行亦可）

**W-B3-26.2 回滚锚点 R2（把 15 s 改成改 `DeadlineMs`）** · `crates/application/src/understand/summary_guard.rs`
- 追溯：design 2.7.3「P2 把 15 s 守护改成改 DeadlineMs」
- DoD：改动 `DeadlineMs` 上限而非应用层记账时，`insight_policy_timeout_exceeds_deadline_limit` 与 AC-B3-19 必须双失败；恢复后均 0
- 证据：演练记录写入 `docs/progress/b3-status.md`

---

## P3a · 30 要点抽取

**目标**：实现"规则预筛 → 结构化抽取 → Schema 校验 → 分类兜底 → 60 字拆分 → 字段补全 → 疑问闭环 → 应用层规则兜底"。
**前置**：P2（`RollingSummary` 作为抽取上下文；类型已在 P0 定死）。
**出口**：AC-B3-22~34 全绿；`fallback.rs` diff=0；`understand/**` 中 `ShortText::truncated` 命中 0。

### W-B3-27 · 规则预筛（AC-22/23）

**W-B3-27.1 三信号固定词表（全大写常量）** · `crates/application/src/understand/insight_extractor.rs`
- 追溯：AC-B3-22 · spec 5.3.1-2 / 4.5-5 · design 2.1.3-3
- DoD：`STATEMENT_SIGNALS`=确认/上线/决定/是/为/将/已经；`ACTION_SIGNALS`=我来/跟进/负责/下周/安排/待办；`QUESTION_SIGNALS`=？/?/吗/是否/如何/为什么/能否；在**原始文本**上 `contains`（全角/半角都命中）；中英混排同输入同输出
- 证据：`grep -c "const STATEMENT_SIGNALS" crates/application/src/understand/insight_extractor.rs` 输出 1（ACTION/QUESTION 同）；`cargo test -p meeting-copilot-application --test insight_extract prefilter_detects_` 退出 0

**W-B3-27.2 未命中短路（0 次网关 + `prefilter_miss`）** · `crates/application/src/understand/insight_extractor.rs`
- 追溯：AC-B3-23 · spec 5.3.1-3 / 5.3.3-3
- DoD：三信号全不命中（如"嗯，好的"）→ 0 草稿、`gateway_calls==0`、`prefilter_miss==1`；不得用兜底强行造要点
- 证据：`cargo test -p meeting-copilot-application --test insight_extract prefilter_miss_skips_gateway_call` 退出 0

**W-B3-27.3 `prefilter_enabled=false` 全部放行（对照回归）** · `crates/application/src/understand/insight_extractor.rs`
- 追溯：spec 6.4-13 · design 2.1.3-3
- DoD：关闭预筛后"嗯，好的"也进入抽取（`gateway_calls==1`）；仅用于对照，不参与 AC 判定
- 证据：`cargo test -p meeting-copilot-application --test insight_extract prefilter_disabled_passes_all` 退出 0

### W-B3-28 · 结构化抽取与 Schema 严格校验（AC-24/25/26/28）

**W-B3-28.1 构造 `InsightExtract` 请求** · `crates/application/src/understand/insight_extractor.rs`
- 追溯：AC-B3-24 · spec 5.3.2 · design 1.1.3-C
- DoD：载荷 = 片段文本 + 当前 `RollingSummary.topics`（可空）+ 术语名；`purpose` 明确；每子片段 ≤1 次调用；不读模型元数据
- 证据：`cargo test -p meeting-copilot-application --test insight_extract extract_request_carries_summary_topics` 退出 0

**W-B3-28.2 能力前置与 `gateway_calls` 计数** · `crates/application/src/understand/insight_extractor.rs`
- 追溯：AC-B3-23 · AC-B3-33 · design 1.2.4 / 2.1.3-3
- DoD：`endpoint_configured==false` 或 `cooling` → 不发起调用、直接兜底、`gateway_calls` 不增；真实调用一次 `gateway_calls+=1`
- 证据：`cargo test -p meeting-copilot-application --test insight_extract gateway_unavailable_skips_call_and_falls_back` 退出 0

**W-B3-28.3 严格校验：text≤60、sources 非空、kind 四类** · `crates/application/src/understand/insight_extractor.rs`
- 追溯：AC-B3-24 · AC-B3-25 · AC-B3-26 · AC-B3-28 · spec 5.3.1-4/5/6 · design 2.1.3-3
- DoD：结果含未知字段/类型不符/正文 >60/`sources=[]` 任一 → 整次作废；未知字段由 `wire.rs` 的 `deny_unknown_fields` 拒绝（不改 `wire.rs`）；`insights=[]` → `extract_empty+=1`
- 证据：`cargo test -p meeting-copilot-application --test insight_extract schema_violation_rejects_whole_result draft_without_source_is_rejected extract_empty_is_counted` 退出 0

**W-B3-28.4 违规后进入规则兜底且不逐条救活** · `crates/application/src/understand/insight_extractor.rs`
- 追溯：AC-B3-24 · spec 5.3.3-1 · design 2.1.3-3
- DoD：`schema_violation+=1` → 0 条模型草稿；随后 `rule_fallback_count>=1` 且最终草稿 ≥1；不发布 >60 字草稿
- 证据：`cargo test -p meeting-copilot-application --test insight_extract schema_violation_falls_back_to_rules` 退出 0

**W-B3-28.5 来源回填与弱校验** · `crates/application/src/understand/insight_extractor.rs`
- 追溯：AC-B3-25 · AC-B3-41 · spec 5.3.1-5 · design 2.1.3-5「对齐校验」
- DoD：草稿 `sources` 由 `project_sources(unit)` 回填（子片段追加 `{parent}#{i}`）；模型给的 `sources` 只做"与本片段引用交集非空"弱校验，**不直接采用**；回填后非空
- 证据：`cargo test -p meeting-copilot-application --test insight_extract model_sources_are_not_adopted_verbatim` 退出 0

### W-B3-29 · 四类分类兜底（AC-27/28）

**W-B3-29.1 接入 `classify_kind` 并保留 `kind_fallback` 标记** · `crates/application/src/understand/field_filler.rs`
- 追溯：AC-B3-27 · AC-B3-28 · spec 5.3.1-7 · design 2.1.3-3
- DoD：模型 kind 经 `classify_kind` 全函数映射；`fallback=true` 时草稿保留并携带标记；`unclassified` 在类型上不可表示
- 证据：`cargo test -p meeting-copilot-application --test insight_extract unknown_kind_falls_back_to_key_info no_unclassified_draft_is_emitted missing_kind_keeps_the_draft` 退出 0

**W-B3-29.2 整数 milli 惩罚（默认 150）** · `crates/application/src/understand/field_filler.rs`
- 追溯：AC-B3-27 · spec 5.3.1-7 / 6.4-9 · design 2.1.3-3「惩罚用整数」
- DoD：`confidence_milli = max(0, 原值 - penalty)`；0.80→0.65（penalty=150）；penalty=100 时 0.80→0.70；0.10→0.00；无浮点误差
- 证据：`cargo test -p meeting-copilot-application --test insight_extract kind_fallback_penalty_is_configurable` 退出 0

**W-B3-29.3 四类计数与 `unclassified=0` 可判定** · `crates/application/src/understand/counters.rs`
- 追溯：AC-B3-26 · AC-B3-28 · spec 5.3.1-6 / 4.4-1
- DoD：`kind_counts` 四类之和等于草稿数；`unclassified` 不提供计数字段（封闭枚举即证明）；10 条草稿时四类计数和 = 10
- 证据：`cargo test -p meeting-copilot-application --test insight_extract four_kinds_are_classified` 退出 0

### W-B3-30 · 60 字强制拆分（AC-29/30，含 R-03 防假通过）

**W-B3-30.1 `split_insight_text` 复用 C-02 算法（limit=60）** · `crates/application/src/understand/insight_splitter.rs`
- 追溯：AC-B3-29 · spec 5.3.1-8 / 6.3-2 · design 2.1.3-3
- DoD：超 60 字才拆；每段 1..=60；一级/二级边界同 C-02；150 字输出 ≥3 段；**不做 UI 截断、不整段复制**
- 证据：`cargo test -p meeting-copilot-application --test insight_extract overlong_insight_is_split` 退出 0

**W-B3-30.2 拆分后来源独立且并集覆盖** · `crates/application/src/understand/insight_splitter.rs`
- 追溯：AC-B3-30 · spec 5.3.1-9 · design 2.1.3-3
- DoD：每段 `sources = draft.sources.clone()`（非空）；3 段并集覆盖原片段引用；单段可共享同一引用但不为空
- 证据：`cargo test -p meeting-copilot-application --test insight_extract split_drafts_keep_source_refs` 退出 0

**W-B3-30.3 禁止 `ShortText::truncated` 假通过（R-03 三段断言）** · `crates/application/src/understand/insight_splitter.rs`
- 追溯：AC-B3-29 · AC-B3-30 · design 2.5.3 R-03 / 2.1.3-6 纪律 2
- DoD：① 目录内 `grep -c "ShortText::truncated"` = 0；② 拆分失败（空/超限）走 `empty_chunk_dropped`/`oversize_draft_blocked`，**不回退截断**；③ 用例断言 150 字拆出的各段拼接逐字等于原文、每段 `char_count<=60`、`oversize_draft_blocked==0`（三条同时成立才算通过）
- 证据：`grep -rn "ShortText::truncated" crates/application/src/understand | wc -l` 输出 0；`cargo test -p meeting-copilot-application --test insight_extract overlong_insight_is_split split_text_is_lossless` 退出 0

**W-B3-30.4 超限残留拦截但其余发布** · `crates/application/src/understand/insight_splitter.rs`
- 追溯：AC-B3-29 · spec 5.3.3-5 / 6.5-19 · design 2.1.3-3
- DoD：拆分后仍 >60 的段（理论不可达）→ 该段不发布、`oversize_draft_blocked+=1`；其余段正常；正常回归该计数必须为 0
- 证据：`cargo test -p meeting-copilot-application --test insight_extract oversize_remnant_is_blocked_but_others_published` 退出 0

### W-B3-31 · 字段补全与疑问闭环（AC-31/32）

**W-B3-31.1 决策缺 owner/范围 → `None` + "待确认"语义** · `crates/application/src/understand/field_filler.rs`
- 追溯：AC-B3-31 · spec 5.3.1-10 / 6.3-7 · design 2.1.3-3
- DoD：`decision.owner=None` 且界面语义为"待确认"；草稿仍产出；不得留空串、不得丢要点
- 证据：`cargo test -p meeting-copilot-application --test insight_extract decision_missing_owner_is_pending` 退出 0

**W-B3-31.2 待办缺 assignee/due → `None`，`action` 必填** · `crates/application/src/understand/field_filler.rs`
- 追溯：AC-B3-31 · spec 5.3.1-11 · design 2.0 D-01
- DoD：`assignee=None`/`due=None` 时草稿保留；`todo.action` 缺失 → `Err(TodoActionMissing)` 且 0 草稿；**不得**用"待确认"回填 `action`
- 证据：`cargo test -p meeting-copilot-application --test insight_extract todo_missing_assignee_is_pending todo_without_action_is_rejected` 退出 0

**W-B3-31.3 疑问初始 `unanswered=true`** · `crates/application/src/understand/field_filler.rs`
- 追溯：AC-B3-32 · spec 5.3.1-12 / 6.3-7/8 · design 2.1.3-3
- DoD：`Question` 草稿 `fields.question.unanswered==true`；非疑问类 `question=None`
- 证据：`cargo test -p meeting-copilot-application --test insight_extract question_starts_unanswered` 退出 0

**W-B3-31.4 10 s 内回应 → `unanswered=false` + `sources` 追加** · `crates/application/src/understand/field_filler.rs`
- 追溯：AC-B3-32 · spec 5.3.1-12 · design 2.1.3-3 / J-05
- DoD：同会话、同一疑问要点、回应片段在 10 s 内到达 → 置 `false` 且 `sources.len()+1`（引用回应片段）；**不新增字段**；超 10 s 不改
- 证据：`cargo test -p meeting-copilot-application --test insight_extract question_closes_on_reply_with_source` 退出 0

### W-B3-32 · 应用层规则兜底与兜底率（AC-33/34）

**W-B3-32.1 四类结构完整的应用层兜底** · `crates/application/src/understand/rule_fallback.rs`
- 追溯：AC-B3-33 · AC-B3-26 · spec 5.3.1-13 / 4.2-1 · design 1.1.3-C / J-04
- DoD：网关不可用/断网/超时/冷却时产出 ≥1 条四类草稿（按预筛信号定 kind，正文取片段前 ≤60 字）；`sources` 非空；不抛错、不阻塞采集；只依赖 domain 类型
- 证据：`cargo test -p meeting-copilot-application --test insight_extract rule_fallback_produces_draft_when_gateway_unavailable` 退出 0

**W-B3-32.2 `infrastructure/llm/fallback.rs` 零改动** · `crates/infrastructure/src/llm/fallback.rs`
- 追溯：AC-B3-33 · spec 1.4-4 · design 2.5.2
- DoD：该文件相对 HEAD 无 diff；应用层不调用 `RuleFallback::produce`；兜底语义独立
- 证据：`git diff --quiet HEAD -- crates/infrastructure/src/llm/fallback.rs` 退出 0

**W-B3-32.3 兜底率可读且范围 0..=1** · `crates/application/src/understand/counters.rs`
- 追溯：AC-B3-34 · spec 5.3.1-14 / 6.5-28 · design 2.4
- DoD：4 次尝试 1 次兜底 → `rate≈0.25`（±0.001）；`counters()` 可读；分母含 `rule_fallback_count`
- 证据：`cargo test -p meeting-copilot-application --test insight_extract rule_fallback_rate_is_recorded` 退出 0

**W-B3-32.4 三入口（不可用/超时/冷却）均不 panic** · `crates/application/src/understand/rule_fallback.rs`
- 追溯：AC-B3-33 · spec 5.3.3-2 · design 2.1.3-3
- DoD：`Err(CopilotError)`、`degraded+Failed(Unavailable)`、`capability().cooling==true` 三条入口都产出草稿；`gateway_cooling`/`gateway_unavailable` 计数各自递增；无 panic
- 证据：`cargo test -p meeting-copilot-application --test insight_extract fallback_on_cooling_and_timeout_never_panics` 退出 0

### W-B3-33 · `insight_extract` target（AC-22~34）

**W-B3-33.1 `StubGateway` 四形态与固定 `LexiconView`** · `crates/application/tests/support/understand_fakes.rs`
- 追溯：AC-B3-24 · AC-B3-27 · AC-B3-33 · design 2.6.3「替身装配」
- DoD：可构造 `StructuredBody::Insight(...)`、`Err(CopilotError)`、`degraded+DegradeCause`、`capability().endpoint_configured=false` 四形态；`model="stub"`；不写 `request_id`/`http_status`
- 证据：`cargo test -p meeting-copilot-application --test insight_extract` 退出 0

**W-B3-33.2 13 条 AC 用例逐条落地** · `crates/application/tests/insight_extract.rs`
- 追溯：AC-B3-22~34 · design 2.6.3 全表
- DoD：`prefilter_detects_statement_signal`、`prefilter_detects_action_signal`、`prefilter_detects_question_signal`、`prefilter_miss_skips_gateway_call`、`schema_violation_rejects_whole_result`、`schema_violation_falls_back_to_rules`、`draft_without_source_is_rejected`、`realistic_corpus_has_zero_source_missing`、`four_kinds_are_classified`、`unknown_kind_falls_back_to_key_info`、`kind_fallback_penalty_is_configurable`、`no_unclassified_draft_is_emitted`、`missing_kind_keeps_the_draft`、`overlong_insight_is_split`、`split_drafts_keep_source_refs`、`decision_missing_owner_is_pending`、`todo_missing_assignee_is_pending`、`todo_without_action_is_rejected`、`question_starts_unanswered`、`question_closes_on_reply_with_source`、`rule_fallback_produces_draft_when_gateway_unavailable`、`rule_fallback_rate_is_recorded` 全部存在并通过
- 证据：`cargo test -p meeting-copilot-application --test insight_extract` 退出 0；上述名字 `grep -c` 均 1

**W-B3-33.3 映射表与禁用截断的静态断言** · `crates/application/tests/insight_extract.rs`
- 追溯：AC-B3-27 · AC-B3-29 · design 2.1.3-3 / 2.5.3 R-03
- DoD：`kind_fallback_table_matches_spec` 6 行逐项；`splitter_never_calls_truncated` 由脚本侧 `--rule=source` 固化（测试内以行为断言替代，见 W-B3-30.3）
- 证据：`cargo test -p meeting-copilot-application --test insight_extract kind_fallback_table_matches_spec` 退出 0

**W-B3-33.4 48 条真实语料 `source_missing==0`** · `crates/application/tests/insight_extract.rs`
- 追溯：AC-B3-25 · spec 6.5-25
- DoD：48 条真实形态语料跑完后 `source_missing==0`、`cross_session_ref==0`、`oversize_draft_blocked==0`
- 证据：`cargo test -p meeting-copilot-application --test insight_extract realistic_corpus_has_zero_source_missing` 退出 0

### W-B3-34 · P3a 出口

**W-B3-34.1 13 条 AC 全绿且 P1/P2 不回退** · 全仓库
- 追溯：AC-B3-22~34 · AC-B3-01~21
- DoD：三个 target（normalize/rolling/extract）全绿；domain/application lib 全绿
- 证据：`cargo test -p meeting-copilot-application --test insight_extract --test rolling_summary --test understanding_normalize` 退出 0

**W-B3-34.2 兜底与来源约束** · 全仓库
- 追溯：AC-B3-33 · AC-B3-25
- DoD：`fallback.rs` diff=0；正常回归 `source_missing=0`；`understand/**` 中 `ShortText::truncated` 0 命中
- 证据：`git diff --quiet HEAD -- crates/infrastructure/src/llm/fallback.rs` 退出 0；`grep -rn "ShortText::truncated" crates/application/src/understand | wc -l` 输出 0

**W-B3-34.3 回滚锚点 R3（缺 action 时回填"待确认"）** · `crates/application/src/understand/field_filler.rs`
- 追溯：design 2.7.3「P3a 让 field_filler 在缺 action 时回填"待确认"」
- DoD：把缺 `action` 改成回填占位 → `todo_without_action_is_rejected` 必须失败；恢复后 0 失败
- 证据：演练记录写入 `docs/progress/b3-status.md`

---

## P3b · 31 去重与引用回溯

**目标**：实现"来源投影 → 同会话断言 → 880‰ + 10 分钟窗口 → 同 kind 才合并 → 锁定只加计数 → 冲突并列补充 → 删除不可见"。
**前置**：P1（`MergedUnit`/`origin_pair` 与子片段派生）+ P3a（`InsightDraft` 结构）。
**出口**：AC-B3-35~42 全绿；0.87 相似度不合并；`source_missing`/`cross_session_ref` 正常回归为 0。

### W-B3-35 · 来源投影与内存要点集合（AC-41/42）

**W-B3-35.1 `project_sources`：`origin_pair` → `sources`** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-41 · spec 1.5-3 / 5.4.1-8 / 6.3-3 · design 2.1.3-5
- DoD：`Single` → 1 条（自身 ref）；`Merged` → 去重保序 `[utterance.reference, subtitle_ref, asr_ref]`；因 `build_merged` 恒等实际 2 条不同引用；子片段追加 `{parent}#{i}`；不回写合并层
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup dual_channel_insight_keeps_origin_pair_refs` 退出 0；`grep -c "fn project_sources" crates/application/src/understand/deduplicator.rs` 输出 1

**W-B3-35.2 `InsightStore` 内存结构与 ref→session** · `crates/application/src/understand/store.rs`
- 追溯：AC-B3-41 · AC-B3-36 · spec 1.4-3 · design 2.0 D-02 / 2.3.2
- DoD：`Mutex<Inner{insights: Vec<Insight>, ref_session: HashMap<UtteranceRef,SessionId>}>`；`register_refs`/`session_of`/`within`/`insert`/`visible`/`update` 齐全；插入不覆盖已有引用
- 证据：`cargo test -p meeting-copilot-application --lib store_` 退出 0；`cargo test -p meeting-copilot-application --test insight_dedup store_is_session_scoped` 退出 0

**W-B3-35.3 来源非空与跨会话拒绝** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-41 · AC-B3-25 · spec 5.4.1-7 / 6.3-3 · design 2.1.3-4
- DoD：`sources` 空 → 拒绝 + `source_missing+=1`；任一 ref 不属于 `draft.session` → 拒绝 + `cross_session_ref+=1`；不得补造引用、不得拿摘要当来源
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup source_missing_is_rejected_at_dedup cross_session_ref_is_rejected` 退出 0

**W-B3-35.4 单条合并单元只提炼一次（承接 AC-DD-08）** · `crates/application/src/understand/service.rs`
- 追溯：AC-B3-42 · spec 1.5-2 · AC-DD-08（只引用）
- DoD：合并单元 + 被合并侧同批进入 → 1 组草稿；被合并侧不进入；同一 ref 重复投递 → `duplicate_delivery_ignored+=1` 且不产生第二条要点
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup merged_unit_triggers_insight_once duplicate_merged_ref_is_ignored` 退出 0

### W-B3-36 · 相似度判定（C-03）

**W-B3-36.1 `insight_similarity` 四步判定** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-35 · AC-B3-36 · spec 5.4.1-1 / 5.4.3-5 · design 2.0 C-03 判定函数
- DoD：① `normalize_text` 后相等 → `Identical`；② 短于 `short_text_merge_min_chars(8)` 且不相等 → `ShortTextSkipped`；③ `ngram_cosine_milli>=880` → `Cosine{milli}`；④ 否则 `Below`；**不调用** `classify_equivalence`/`PairBasis`
- 证据：`cargo test -p meeting-copilot-application --lib insight_similarity_` 退出 0；`grep -cE "classify_equivalence|PairBasis" crates/application/src/understand/deduplicator.rs` 输出 0

**W-B3-36.2 0.90 合并 / 0.87 不合并（880‰ 边界）** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-35 · spec 5.4.1-1 · design 2.0 C-03
- DoD：0.90 相似同 kind → 1 条 + `mention_count==2`；0.87 → 2 条；阈值取自 `InsightPolicy.similarity_min_milli`（默认 880），不改 `MergePolicy.cosine_min_milli`
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup similar_insights_merge_and_increment below_similarity_threshold_does_not_merge` 退出 0

**W-B3-36.3 短文本退化与计数** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-35 · spec 5.4.3-5 / 6.5-27 · design 2.0 C-03 步骤 2
- DoD：`<8` 字且不完全相等 → 不启用余弦、不合并、`short_text_merge_skipped+=1`；`<8` 字但完全相等 → 仍 `Identical` 合并（顺序保证）
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup short_text_only_merges_when_identical` 退出 0

**W-B3-36.4 类型不同永不合并** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-37 · spec 5.4.1-3 · design 2.1.3-4
- DoD：候选先按 `kind` 过滤；`Decision` vs `Todo` 即使相似度 0.95 → 2 条；不因文本相似跨类型合并
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup different_kinds_are_never_merged` 退出 0

### W-B3-37 · 10 分钟窗口与三态合并（AC-36/37/38/40）

**W-B3-37.1 窗口以要点创建时刻计（599/601 s 边界）** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-36 · spec 5.4.1-2 · design 2.1.3-4「用要点创建时刻」
- DoD：`now - candidate.created_at <= dedupe_window_ms(600000)` 才候选；599000 ms 合并、601000 ms 不合并；超窗同义内容 → 新要点、`mention_count=1`
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup merge_window_boundary_merges merge_window_expires_after_ten_minutes` 退出 0

**W-B3-37.2 `Merged`/`Locked` 走既有 `merge_repeated`** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-38 · AC-B3-35 · spec 5.4.1-4 · design 1.2.3
- DoD：未锁定同义 → `merge_repeated` + `Merged{count}`；锁定（`manual_locked` 或 `status==Edited`）→ 只 `mention_count+1`，正文/类型/字段逐字不变，来源不追加，返回 `Locked{count}`
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup locked_insight_only_increments_count` 退出 0；既有 `manual_edit_is_not_overwritten_by_merge` 仍通过

**W-B3-37.3 删除不可见且拒绝合并** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-40 · spec 5.4.1-6 / 6.3-9 · design 2.1.3-4
- DoD：`is_visible(&Insight) -> bool` 纯函数（`status != Deleted`）；候选为 `Deleted` 时跳过并继续下一个；删除后同义内容 → 新可见要点 1 条，原条仍 `Deleted`；既有 `deleted_insight_cannot_merge` 逐字保留
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup deleted_insight_is_hidden_everywhere deleted_insight_rejects_merge` 退出 0

**W-B3-37.4 正文与来源均相同的草稿计数** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-35 · spec 5.3.3-4 / 6.5-18 · design 2.1.3-4
- DoD：正文与来源都与既有可见要点完全相同的草稿 → `duplicate_text_draft+=1`，但仍按 `Identical → Merged` 处理，不二次产出卡片
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup duplicate_text_draft_is_counted_once` 退出 0

### W-B3-38 · 冲突并列"补充信息"（AC-39）

**W-B3-38.1 "同主题"确定性判定常量 400‰** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-39 · design 2.1.3-4「冲突并列」/ D-B3-08
- DoD：常量 `SUPPLEMENT_TOPIC_OVERLAP_MILLI = 400` 全大写；判定为归一化后共同 2-gram 覆盖 `|A∩B|/min(|A|,|B|) >= 400‰`；**不参与合并阈值**、不新增策略字段
- 证据：`grep -c "SUPPLEMENT_TOPIC_OVERLAP_MILLI" crates/application/src/understand/deduplicator.rs` 输出 ≥1；`grep -c "SUPPLEMENT_TOPIC_OVERLAP_MILLI" crates/domain/src/model/insight_policy.rs` 输出 0

**W-B3-38.2 冲突内容新增补充要点并关联原要点** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-39 · spec 5.4.1-5 / 5.4.3-4 · design 2.1.3-4
- DoD：锁定要点收到"相似 <880 但同主题"内容 → 新建 `Insight{supplement_of:Some(原 ref), mention_count:1}` 并标记"补充信息"；返回 `Supplement{reference}`；原锁定正文逐字不变
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup conflicting_content_is_listed_as_supplement` 退出 0

**W-B3-38.3 补充要点不改变锁定状态与来源** · `crates/application/src/understand/deduplicator.rs`
- 追溯：AC-B3-38 · AC-B3-39 · spec 5.4.1-4/5
- DoD：原要点 `status` 仍 `Edited`、`manual_locked==true`、`sources` 长度不变；新补充要点 `supplement_of` 指向原 `InsightRef`
- 证据：用例断言三者同时成立（并入 W-B3-38.2 用例的第二段断言）

### W-B3-39 · 标记 / 删除 / 筛选（AC-40）

**W-B3-39.1 重要标记** · `crates/application/src/understand/insight_ops.rs`
- 追溯：AC-B3-40 · spec 5.4.1-10 · tasks 31.3
- DoD：`flag_important(ref, bool)` 只改 `flagged_important`，不改 `mention_count`/`status`/正文
- 证据：`cargo test -p meeting-copilot-application --lib insight_ops_flag_` 退出 0

**W-B3-39.2 删除只改状态、不物理移除** · `crates/application/src/understand/insight_ops.rs`
- 追溯：AC-B3-40 · spec 5.4.1-6 · design 2.3.2 对象生命周期
- DoD：`delete(ref)` 置 `status=Deleted`，条目仍在 `Vec`；`mention_count` 与审计信息保留；不触发合并
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup deleted_insight_is_hidden_everywhere` 退出 0

**W-B3-39.3 按类型/关键词筛选为只读** · `crates/application/src/understand/insight_ops.rs`
- 追溯：AC-B3-40 · spec 5.4.1-10/11
- DoD：筛选返回子集；不改变任何 `flagged_important`/`mention_count`/`status`；筛选结果不含 `Deleted`
- 证据：`cargo test -p meeting-copilot-application --lib insight_ops_filter_is_read_only` 退出 0

**W-B3-39.4 可见性三消费者统一调用 `is_visible`** · `crates/application/src/understand/insight_ops.rs`
- 追溯：AC-B3-40 · spec 6.3-9 · design 2.1.3-4
- DoD：要点流、纪要输入、相关性评分输入三处都调用同一纯函数；`Deleted` 在三处均不可见；不得依赖 UI 过滤
- 证据：`grep -rc "is_visible" crates/application/src/understand/insight_ops.rs` 输出 ≥3；`cargo test -p meeting-copilot-application --test insight_dedup deleted_insight_is_hidden_everywhere` 退出 0

### W-B3-40 · `insight_dedup` target（AC-35~42）

**W-B3-40.1 `draft(...)` 构造器与 `VirtualClock` 窗口控制** · `crates/application/tests/support/understand_fakes.rs`
- 追溯：AC-B3-35~42 · design 2.6.4「替身装配」
- DoD：构造器可指定 `kind/text/sources/created_at`；`VirtualClock` 可 `+599000`/`+601000`；`InsightStore` 内存单锁；不触网
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup` 退出 0

**W-B3-40.2 8 条 AC 用例逐条落地** · `crates/application/tests/insight_dedup.rs`
- 追溯：AC-B3-35~42 · design 2.6.4 全表
- DoD：`similar_insights_merge_and_increment`、`below_similarity_threshold_does_not_merge`、`merge_window_expires_after_ten_minutes`、`merge_window_boundary_merges`、`different_kinds_are_never_merged`、`locked_insight_only_increments_count`、`conflicting_content_is_listed_as_supplement`、`deleted_insight_is_hidden_everywhere`、`deleted_insight_rejects_merge`、`dual_channel_insight_keeps_origin_pair_refs`、`source_missing_is_rejected_at_dedup`、`merged_unit_triggers_insight_once`、`duplicate_merged_ref_is_ignored` 全部存在并通过
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup` 退出 0；上述名字 `grep -c` 均 1

**W-B3-40.3 既有锁定/删除用例名保留** · `crates/domain/src/model/insight.rs`
- 追溯：AC-B3-38 · AC-B3-40 · design 2.8.2-3
- DoD：`manual_edit_is_not_overwritten_by_merge` 与 `deleted_insight_cannot_merge` 逐字保留并通过；domain lib 全绿
- 证据：`grep -c "fn manual_edit_is_not_overwritten_by_merge" crates/domain/src/model/insight.rs` 输出 1；`grep -c "fn deleted_insight_cannot_merge"` 输出 1；`cargo test -p meeting-copilot-domain --lib` 退出 0

**W-B3-40.4 产物不变量（跨用例统一）** · `crates/application/tests/insight_dedup.rs`
- 追溯：AC-B3-41 · AC-B3-29 · AC-B3-26 · design 2.6.4「产物不变量」
- DoD：任何 `Insight` 的 `sources` 非空且同会话、`text.char_count()<=60`、`kind` 四类之一、`mention_count>=1`；`source_missing==0`/`cross_session_ref==0`（注入用例除外）
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup invariant_all_insights_are_well_formed` 退出 0

### W-B3-41 · P3b 出口

**W-B3-41.1 8 条 AC 全绿且前序不回退** · 全仓库
- 追溯：AC-B3-35~42 · AC-B3-01~34
- DoD：四个 target 全绿；`source_missing`/`cross_session_ref`/`oversize_draft_blocked` 正常路径为 0
- 证据：`cargo test -p meeting-copilot-application --test insight_dedup --test insight_extract --test rolling_summary --test understanding_normalize` 退出 0

**W-B3-41.2 回滚锚点 R4（用 `classify_equivalence` 替代自有判定）** · `crates/application/src/understand/deduplicator.rs`
- 追溯：design 2.7.3「P3b 用 classify_equivalence 替代 insight_similarity」
- DoD：替换后 0.87 相似文本被 `NearHash` 命中而合并 → `below_similarity_threshold_does_not_merge` 必须失败；恢复自有判定后 0 失败
- 证据：演练记录写入 `docs/progress/b3-status.md`

---

## P3c · 门禁、全量回归与回滚演练

**目标**：落地 6 条规则的新门禁、收窄零转录扫描面、跑全量回归与四类独立回滚，并把 48 条 AC 三态写入进度记录。
**前置**：P1~P3b。
**出口**：AC-B3-43~48 全绿；`cargo test --workspace` ≥442 passed / 0 failed / 9 ignored；`removed=0`；既有门禁 0 违规；所有 `--rule=` 退出 0（不得出现 2）。

### W-B3-42 · 新门禁脚本骨架与 source/deps/targets 规则

**W-B3-42.1 脚本骨架、参数与退出码** · `scripts/check-understanding-offline.sh`
- 追溯：AC-B3-43~48 · spec 7.7 · design 2.6.5
- DoD：支持 `--rule={source|deps|zero-transcript|targets|ui-truncate|doc-plan}`；退出码 0=通过 / 1=命中违规 / 2=扫描面缺失；任一扫描目录不存在即 2；`--help`/未知规则输出用法并退出 1；不修改既有 `check-capture-offline.sh`/`check-mac-scope.sh`
- 证据：`bash scripts/check-understanding-offline.sh --rule=source` 退出 0；`bash scripts/check-understanding-offline.sh --rule=unknown` 退出 1；`git diff --quiet HEAD -- scripts/check-capture-offline.sh scripts/check-mac-scope.sh` 退出 0

**W-B3-42.2 `--rule=source`：8 类违规 + `truncated` 补充断言** · `scripts/check-understanding-offline.sh`
- 追溯：AC-B3-43 · AC-B3-21 · design 2.5.3 R-03 / 2.6.5
- DoD：`crates/application/src/understand` 中 `reqwest::`/`TcpStream`/`native_tls::`/`Instant::now`/`SystemTime::now`/`thread::sleep`/`panic!(`/`todo!(`/`unimplemented!(` 命中 0；额外断言 `insight_splitter.rs` 中 `truncated(` 命中 0；目录缺失退出 2
- 证据：`bash scripts/check-understanding-offline.sh --rule=source` 退出 0

**W-B3-42.3 `--rule=deps`：domain/application 依赖闭包白名单** · `scripts/check-understanding-offline.sh`
- 追溯：AC-B3-47 · spec 1.4-5 / 7.7 · design 2.5.2
- DoD：`cargo tree -p meeting-copilot-domain` 与 `-p meeting-copilot-application` 不含 `infrastructure`/`platform-win|mac`/`reqwest`/`hyper`/`sqlx`/`native-tls`/`tauri`；含则退出 1
- 证据：`bash scripts/check-understanding-offline.sh --rule=deps` 退出 0（前置 `CARGO`）

**W-B3-42.4 `--rule=targets`：`NEW_TARGETS` 四项反查** · `scripts/check-understanding-offline.sh`
- 追溯：AC-B3-44 · spec 1.7-2 / 7.7
- DoD：解析 `scripts/test-name-inventory.sh` 的 `NEW_TARGETS`，断言 4 项齐全；缺任一项退出 1；脚本文件缺失退出 2
- 证据：`bash scripts/check-understanding-offline.sh --rule=targets` 退出 0；临时删一项（演练后恢复）退出 1

**W-B3-42.5 脚本自检（植入违规 → 1，移除 → 0）** · `scripts/check-understanding-offline.sh`
- 追溯：design 2.6.5 · 既有脚本 `--self-test` 惯例
- DoD：`--self-test` 在 `.tmp/` 注入 `Instant::now`、`truncated(`、缺失 target、`https://` 四类样本，逐类期望 1；清空后期望 0；自检不污染仓库
- 证据：`bash scripts/check-understanding-offline.sh --self-test` 退出 0（内部逐条断言）

### W-B3-43 · 零转录扫描面收窄（AC-45 / R-01 / D-B3-03）

**W-B3-43.1 扫描面 S1 固定且可判定** · `scripts/check-understanding-offline.sh`
- 追溯：AC-B3-45 · spec 4.3-1 / 7.7 · design 2.5.3 R-01 / 2.11 D-B3-03
- DoD：`--rule=zero-transcript` 只扫三处：`crates/application/src/understand/**`、`crates/application/tests/{understanding_normalize,rolling_summary,insight_extract,insight_dedup}.rs` 与 `crates/application/tests/support/understand_fakes.rs`；任一目录/文件缺失 → 2；**不扫描** `.codeartsdoer/**` 正文（另按 W-B3-43.2 处理）
- 证据：`bash scripts/check-understanding-offline.sh --rule=zero-transcript` 退出 0；`grep -c ".codeartsdoer" scripts/check-understanding-offline.sh` 输出 ≤1 且仅用于文档分支

**W-B3-43.2 代码/夹具只查"字面量值"，文档只查"可执行凭据形态"** · `scripts/check-understanding-offline.sh`
- 追溯：AC-B3-45 · design 2.5.3 R-01「收窄口径」/ D-B3-03
- DoD：代码/夹具跳过 `//`、`///`、`//!`、`/* */` 注释行后，断言 `https?://`、`request_id` 赋值、`sk-`、`Bearer ` 命中 0；`.codeartsdoer/specs/b3_understanding/*.md` 只匹配"长随机串与赋值号相邻"的凭据形态（如 `sk-[A-Za-z0-9]{16,}`、`Bearer [A-Za-z0-9._-]{20,}`、非占位 URL），反引号包裹的字段名引用不计
- 证据：`bash scripts/check-understanding-offline.sh --rule=zero-transcript` 退出 0；自查 `bash scripts/scan-secrets.sh` 退出 0；`bash scripts/scan-secrets-docs.sh` 退出 0

**W-B3-43.3 排除自身与 `.tmp`，并提供负向自检** · `scripts/check-understanding-offline.sh`
- 追溯：spec 7.7「自检脚本自身排除」 · design 2.5.3 R-01
- DoD：排除 `scripts/check-understanding-offline.sh` 自身、`.tmp/**`、`target/**`；`--self-test` 注入 `sk-` + 32 位串与占位外 URL → 期望 1；移除后 0
- 证据：`bash scripts/check-understanding-offline.sh --self-test` 退出 0（内部负向断言通过）；`ls .tmp/zero-transcript-selftest* 2>/dev/null | wc -l` 输出 0（自检后清理）

**W-B3-43.4 与既有秘密扫描并存、不互相替代** · `scripts/scan-secrets.sh` / `scripts/scan-secrets-docs.sh`
- 追溯：AC-B3-45 · spec 4.3-1
- DoD：三脚本各自 0 命中；新增门禁不改既有脚本；不存在"因为新门禁通过所以跳过旧门禁"的路径
- 证据：三条命令均退出 0

### W-B3-44 · `ui-truncate` 与 `doc-plan` 规则

**W-B3-44.1 `--rule=ui-truncate`：前端无截断** · `scripts/check-understanding-offline.sh`
- 追溯：AC-B3-30 · spec 7.7 · design 2.6.5 / 2.11 D-B3-02
- DoD：`app/src` 中 `truncate(` 命中 0；目录缺失退出 2；若未来出现正当截断（标题省略），把断言收窄为"要点正文渲染路径无截断"并登记 D-B3-02，不得直接删规则
- 证据：`bash scripts/check-understanding-offline.sh --rule=ui-truncate` 退出 0；`grep -rn "truncate(" app/src | wc -l` 输出 0

**W-B3-44.2 `--rule=doc-plan`：变更清单完整性与路径存在性** · `scripts/check-understanding-offline.sh`
- 追溯：AC-B3-48 · spec 7.7 / 第 8 章 · design 2.10
- DoD：解析 `spec.md` 第 8 章表格：行数 ≥1；每行"文件"列路径存在，或"动作"列为"新增"；缺项/路径不存在退出 1；spec 文件缺失退出 2
- 证据：`bash scripts/check-understanding-offline.sh --rule=doc-plan` 退出 0

**W-B3-44.3 规则间互不短路** · `scripts/check-understanding-offline.sh`
- 追溯：design 2.6.5「退出码 0/1/2，扫描面缺失退 2」
- DoD：任一规则失败不影响其他规则单独可执行；缺省（无 `--rule`）跑全部 6 条并聚合退出码（有 1 → 1；无 1 但有 2 → 2）
- 证据：`bash scripts/check-understanding-offline.sh` 退出 0；删任一扫描目录（演练后恢复）→ 退出 2

### W-B3-45 · 全量回归与既有门禁不回退

**W-B3-45.1 Rust 全量：passed ≥442 / failed=0 / ignored=9** · 全仓库
- 追溯：AC-B3-44 · AC-B3-04 · spec 4.5-1 · design 2.5.2
- DoD：`cargo test --workspace` 退出 0；结果行 passed ≥442、failed=0、ignored=9；`--list --ignored` 条目数 = 9
- 证据：`cargo test --workspace` 退出 0；`cargo test --workspace -- --list --ignored | grep -c ": test$"` 输出 9

**W-B3-45.2 用例名差集 `removed=0`（365 基线只读）** · `docs/progress/test-names-before.txt`
- 追溯：AC-B3-44 · spec 1.7-3/4 · design 2.8.3
- DoD：`--diff` 退出 0 且 `removed=0`；基线 365 行零改动；不存在"改基线规避"的提交
- 证据：`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt` 退出 0；`git diff --quiet HEAD -- docs/progress/test-names-before.txt` 退出 0

**W-B3-45.3 既有七类门禁全 0** · `scripts/**`
- 追溯：AC-B3-46 · AC-B3-47 · spec 4.5-2/3 · design 2.9.3
- DoD：`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`bash scripts/check-layering.sh`、`bash scripts/scan-secrets.sh`、`bash scripts/scan-secrets-docs.sh`、`cargo deny check licenses bans sources advisories`、`bash scripts/check-capture-offline.sh`、`bash scripts/check-mac-scope.sh --rule=claims`、`bash scripts/check-g2-claims.sh` 全部退出 0
- 证据：上述九条命令逐条执行（`check-capture-offline.sh` 不带参数跑全规则）

**W-B3-45.4 依赖与禁改文件零 diff** · `Cargo.toml` / `Cargo.lock` / `capture/**` / `merge/**` / `llm/**`
- 追溯：AC-B3-47 · AC-B3-33 · spec 1.4-1/4/5 · design 2.5.2
- DoD：`Cargo.toml`/`Cargo.lock` 新增依赖行 0；`fallback.rs`、`capture/**`、`merge/**`、`wire.rs`/`gateway.rs` 相对 HEAD 无 diff（允许新增 `understand/**` 文件）
- 证据：`git diff --stat -- Cargo.toml Cargo.lock` 输出为空；`git diff --quiet HEAD -- crates/infrastructure/src/llm/fallback.rs crates/infrastructure/src/llm/wire.rs crates/infrastructure/src/llm/gateway.rs crates/domain/src/merge crates/application/src/capture` 退出 0

### W-B3-46 · 四类独立回滚演练

**W-B3-46.1 回滚单元 1：B3 开关** · `crates/application/src/understand/service.rs`
- 追溯：design 2.5.1 / 2.7.3
- DoD：装配方不调用 `observe` 或传 `Disabled` 时，`capture/**` 行为与现状逐字一致；`capture_service`/`merge_dedupe` 用例全绿；恢复后 0 失败
- 证据：`cargo test -p meeting-copilot-application --test capture_service --test merge_dedupe` 退出 0；`grep -rc "UnderstandService" crates/application/src/capture | grep -v ":0"` 无输出

**W-B3-46.2 回滚单元 2：领域类型（append-only）** · `crates/domain/src/model/**`
- 追溯：design 2.3.1-4 / 2.5.2 / 2.7.3
- DoD：撤销 `Insight.supplement_of`/`TodoFields.action` 与新增模块后，domain lib 76 个既有名仍全绿；恢复后 0 失败；`RollingSummary` 未搬移
- 证据：`cargo test -p meeting-copilot-domain --lib` 先 0 后演练再 0；`bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt` 输出 `removed=0`

**W-B3-46.3 回滚单元 3：测试目标** · `crates/application/tests/**` / `scripts/test-name-inventory.sh`
- 追溯：design 2.8.2 / 2.7.3
- DoD：同时撤掉 4 个 target 文件与 `NEW_TARGETS` 4 项 → 全量回到 40 targets / 442 / 9；只撤 `NEW_TARGETS` 不撤文件 → `--rule=targets` 必须退出 1（证明门禁有效）
- 证据：两次 `cargo test --workspace` 记录；`bash scripts/check-understanding-offline.sh --rule=targets` 先 1 后 0

**W-B3-46.4 回滚单元 4：门禁脚本** · `scripts/check-understanding-offline.sh`
- 追溯：design 2.7.3「P3c 删除 --rule=ui-truncate」
- DoD：删除该规则后 AC-B3-30 的脚本侧不可判定（脚本退出 2 或规则缺失退 1），**不得**记为通过；整脚本删除时所有 `--rule=` 退出 2；恢复后全 0
- 证据：演练记录（退出码三联：删规则 / 删脚本 / 恢复）写入 `docs/progress/b3-status.md`

### W-B3-47 · 进度记录与最终验收

**W-B3-47.1 `docs/progress/b3-status.md`：48 条 AC 三态 + 计数快照** · `docs/progress/b3-status.md`
- 追溯：AC-B3-48 · spec 8 章 · design 2.7.1 P3c / 2.10
- DoD：48 条每行标注"通过 / 不适用（本轮不判定） / 未通过"；附 `CounterSnapshot` 关键字段（`admitted`/`summary_refresh_ok`/`source_missing=0`/`cross_session_ref=0`/`oversize_draft_blocked=0`/`dispatch_candidates=0`）；附四条回滚演练结论
- 证据：文件存在且 `grep -c "AC-B3-" docs/progress/b3-status.md` 输出 ≥48

**W-B3-47.2 偏差登记 D-B3-01~08 全覆盖** · `docs/progress/b3-status.md`
- 追溯：design 2.11 · spec 9.3
- DoD：逐条登记且每条含"处置 + 证据链接"，编号必须完整出现：
  - `D-B3-01` 标量切分可能切断 ZWJ/组合符 → `split_grapheme_boundary` 可观测、不引入字素簇依赖
  - `D-B3-02` `topics` 规则抽取准确率低于 LLM → 只要求"非空可消费"，质量留 30.6
  - `D-B3-03` `zero-transcript` 与 spec 自含字段名冲突 → 扫描面收窄（W-B3-43.2）
  - `D-B3-04` spec 6.5 固定 27 项、补第 28 项 `split_span_degraded` → 派生值编号不变
  - `D-B3-05` tasks 路径与现状不一致 → 按现状追加不搬移
  - `D-B3-06` `RollingSummary` 无 `text` → 本轮不新增
  - `D-B3-07` `StructuredOutcome.model` 禁读 → 替身用 `"stub"`
  - `D-B3-08` 补充判定常量 400‰ → 仅用于"同主题"，不参与合并阈值
- 证据：`grep -oE "D-B3-0[1-8]" docs/progress/b3-status.md | sort -u | wc -l` 输出 8；`grep -c "未登记" docs/progress/b3-status.md` 输出 0

**W-B3-47.3 平台口径：Windows 单平台 + macOS 不适用 + G2 未通过** · `docs/progress/b3-status.md`
- 追溯：AC-B3-46 · spec 4.5-3 / 7.1 · design 2.5.2
- DoD：出现"不适用（无设备）"≥1；无"三平台真机稳定运行""G2 已通过""G3 已通过"类表述；`check-mac-scope.sh --rule=claims` 与 `check-g2-claims.sh` 均 0
- 证据：`grep -F "不适用（无设备）" docs/progress/b3-status.md | wc -l` 输出 ≥1；两条脚本退出 0

**W-B3-47.4 最终命令序列与 AC 判定汇总** · 全仓库
- 追溯：AC-B3-43~48 · design 2.9.3
- DoD：§附录 C 的 20 条命令全部退出 0（`--rule=` 不得为 2）；48 条 AC 判定表齐全；不宣称 G2/G3
- 证据：按附录 C 顺序执行并留档；`cargo test --workspace` 与 `--diff` 的关键行贴入 `b3-status.md`

---

## 附录 A · 回滚单元总表（四处独立可回退）

| # | 单元 | 回滚动作 | 破坏后的期望门禁 | 恢复后 |
|---|---|---|---|---|
| R1 | B3 开关 | 不调用 `observe` 或传 `Disabled` | `capture_service`/`merge_dedupe` 全绿且行为逐字一致 | 0 |
| R2 | 领域类型 | 撤销 `supplement_of`/`TodoFields.action`/新增模块 | domain lib 76 既有名仍全绿 | 0 |
| R3 | 测试目标 | 同时撤 4 个 target 与 `NEW_TARGETS` 4 项 | 回到 40 targets/442/9；只撤一项时 `--rule=targets`=1 | 0 |
| R4 | 门禁脚本 | 删单条规则或整脚本 | AC 不可判定（退出 1/2），不得记通过 | 0 |

## 附录 B · `NEW_TARGETS` 登记与用例名关系

- 位置：`scripts/test-name-inventory.sh:18`，当前 6 项 → **第 7~10 项**依次追加 `understanding_normalize`、`rolling_summary`、`insight_extract`、`insight_dedup`（同一行末尾，顺序固定）。
- 机制：`inventory(exclude_new=yes)` 会跳过 `NEW_TARGETS`（脚本 `:134-136`），因此 `--diff` 只比对基线已存在的 target；新 target 的用例计入 `added`，不参与通过判定。
- target 数：40 → 44（Cargo 由 `tests/*.rs` 自动发现，不写 `[[test]]`）。
- 基线：`docs/progress/test-names-before.txt` 365 行（application 90 / domain 76 / infrastructure 196 / ui-bridge 3）**只读**；`removed=0` 是硬门禁，出现 `removed>0` 只允许回退改名，不允许改基线。
- 新增单测命名：domain 侧统一前缀 `insight_policy_*`/`split_*`/`kind_*`/`summary_version_*`/`supplement_*`/`insight_sources_*`/`todo_fields_*`，保证 `grep -c` 唯一。
- 不新增 `#[ignore]`：既有 9 个（`llm_live.rs` 8 + `ac25_control_request_skipped_construction_conflict` 1）数量与名称不变。

## 附录 C · 标准验收命令序列（全部退出 0，`--rule=` 不得为 2）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
export RUSTFMT="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/rustfmt.exe"

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p meeting-copilot-domain --lib
cargo test -p meeting-copilot-application --test understanding_normalize
cargo test -p meeting-copilot-application --test rolling_summary
cargo test -p meeting-copilot-application --test insight_extract
cargo test -p meeting-copilot-application --test insight_dedup
cargo test --workspace
cargo test --workspace -- --list --ignored | grep -c ': test$'
bash scripts/check-understanding-offline.sh --rule=source
bash scripts/check-understanding-offline.sh --rule=deps
bash scripts/check-understanding-offline.sh --rule=zero-transcript
bash scripts/check-understanding-offline.sh --rule=targets
bash scripts/check-understanding-offline.sh --rule=ui-truncate
bash scripts/check-understanding-offline.sh --rule=doc-plan
bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
bash scripts/scan-secrets.sh
bash scripts/scan-secrets-docs.sh
bash scripts/check-mac-scope.sh --rule=claims
bash scripts/check-g2-claims.sh
bash scripts/check-layering.sh
bash scripts/check-capture-offline.sh
cargo deny check licenses bans sources advisories
git diff --stat -- Cargo.toml Cargo.lock
git diff --quiet HEAD -- crates/infrastructure/src/llm/fallback.rs
```

期望：`--list --ignored | grep -c` 输出 **9**；`--diff` 输出含 `removed=0`；`git diff --stat` 为空。

## 附录 D · 最易被忽略的风险（实现期自检清单）

| 风险 | 触发方式 | 必须成立的断言 |
|---|---|---|
| R-03 `ShortText::truncated` 假通过 | 拆分器误用截断后 `oversize_draft_blocked` 恒 0，而正文被静默截短 | `understand/**` 中 `truncated(` 0 命中 + 150 字拆分各段拼接逐字等于原文 + 每段 ≤60 |
| R-01 zero-transcript 扫描面含文档 | 照字面实现把 `.md` 正文也算入，spec 自身字段名导致 AC-45 必然失败 | 代码/夹具只查字面量值；文档只查"长随机串与赋值号相邻"的凭据形态 |
| 幂等与置信度顺序错位 | 先判置信度后判幂等，第二次投递被计成 `skipped_low_confidence` | `duplicate_delivery_ignored==1 && skipped_low_confidence==0` |
| C-01 用兜底主题填充 | 为了让版本发布成功给"会议"兜底主题，导致 AC-18 失效 | `topics` 为空即不发布 + `summary_publish_blocked+=1`，无 fallback 主题常量 |
| C-03 复用 `classify_equivalence` | `NearHash` 先于余弦短路，0.87 被合并 | 0.87 不合并；`deduplicator.rs` 不 import `PairBasis` |
| 摘要超时改 `DeadlineMs` | 把 15 s 塞进 `DeadlineMs`（上限 8 s），编译期即炸或语义错位 | 应用层记账；网关 `DeadlineMs<=8000` |
| `NEW_TARGETS` 与文件不同步 | 只加文件不加清单，`--diff` 把新用例当 `removed` 噪声 | 4 项同批追加；`--rule=targets` 0；`removed=0` |
| 单元测试用真实时间 | `Instant::now`/`thread::sleep` 进入 `understand/**` | `--rule=source` 8 类 + 真实计时 2 类命中 0 |
