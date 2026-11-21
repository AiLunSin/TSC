# B3 理解层 · 阶段进度记录

> 日期：2026-11-21
> 范围：B3 理解层（28 片段归一化、29 滚动摘要、30 要点抽取、31 去重与引用回溯）
> 结论：**部分完成**。P0~P3b 的离线纯逻辑与存储层已落地；网关结构化抽取、service 接线与门禁回归尚未完成。

## 1. 已完成

### 1.1 P0 领域类型

| 文件 | 内容 |
|---|---|
| `crates/domain/src/model/insight_policy.rs` | `InsightPolicy` 13 字段、默认值、固定顺序 `validate`、四类错误 |
| `crates/domain/src/model/utterance_text.rs` | `SplitChunk`、`SplitError`、`split_utterance`（2000 硬上限、句读优先、I1~I6） |
| `crates/domain/src/model/insight_kind.rs` | `classify_kind` 全函数 + 四类直映 + 未知兜底 |
| `crates/domain/src/model/insight_similarity.rs` | `DedupeBasis{Identical,Cosine}` |
| `crates/domain/src/model/rolling_summary.rs` | `SummaryVersionPair` / `SummaryStatus` / `SummaryParse` / `SummaryVersionError` |
| `crates/domain/src/model/insight.rs`（追加） | `InsightSources` 非空集合、`InsightDraft`、`TodoFields.action`、`Insight.supplement_of`、`TodoActionMissing`/`OversizeText` |

### 1.2 P1 片段准入

| 文件 | 内容 |
|---|---|
| `understand/counters.rs` | 28 项计数 + 派生值 `rule_fallback_rate` |
| `understand/normalize.rs` | 状态、来源/平台、空段、0.35/0.50 两步门槛、静音比例、说话人兜底、2000 切分 |
| `understand/service.rs` | `UnderstandService`、`Disabled` 零副作用、幂等、只读视图 |
| `understand/ports.rs` | `UnderstandClock`、`UnderstandFeature`、`SilenceSignal`、`LexiconView` |
| `understand/types.rs` | `UnderstandInput`（含 `session_platform`）、`RejectReason`、`UnderstandOutcome`、`NormalizedFragment` |

集成用例 `understanding_normalize`：7 条全绿。

### 1.3 P2 滚动摘要

| 文件 | 内容 |
|---|---|
| `understand/summary_parser.rs` | C-01 的 P1~P7（分句、80 字硬切、术语/二元组、三级排序、实体、空主题） |
| `understand/rolling_summary.rs` | 消费式窗口、20 条/30 s 双触发、`covered`、`SessionSummaryState` |
| `understand/summary_guard.rs` | 增量触发、超时、发布裁决（空主题、乱序、版本 +1、保留 2 版） |

用例：`summary_parser` 6 条、`rolling_summary`（src）6 条、`rolling_summary`（target）12 条。

### 1.4 P3a 要点抽取（纯逻辑部分）

| 文件 | 内容 |
|---|---|
| `understand/insight_extractor.rs` | 三信号常量、`prefilter_hit`、`classify_signals` |
| `understand/rule_fallback.rs` | 应用层兜底草稿（kind 由信号定、前 60 字、sources 非空、Todo 带 action） |
| `understand/insight_splitter.rs` | 60 字强制拆分（复用 C-02、逐字无损） |
| `understand/field_filler.rs` | 整数毫值惩罚、疑问字段补全 |

用例：4 个模块共 11 条。

### 1.5 P3b 去重与回溯（纯逻辑与存储）

| 文件 | 内容 |
|---|---|
| `understand/deduplicator.rs` | `project_sources`、四步 `similarity`、`is_visible`、`merge_drafted` 三态、`supplement_draft`、`apply_supplement` |
| `understand/store.rs` | 会话隔离、`ref → session`、`get`/`insert`/`update`/`visible` |
| `understand/insight_ops.rs` | 标记 / 删除 / 筛选 / 三消费者统一 `is_visible` |

用例：`deduplicator` 9 条、`store` 3 条、`insight_ops` 4 条、`insight_dedup` target 12 条。

## 2. 最近一次全量回归

| 项 | 结果 |
|---|---|
| `cargo test --workspace` | 42 targets / **515 passed** / 0 failed / 9 ignored（P3a 出口口径） |
| `fmt --check` | 0 |
| `clippy -D warnings`（application `--tests`） | 0 |
| `understand/**` 的 `ShortText::truncated` | 0 命中 |
| `infrastructure/llm/fallback.rs` | 相对 HEAD 零 diff |

> 此后新增 `deduplicator`（+1）、`store`（+3）、`insight_ops`（+4）、`insight_dedup`（+12）等用例，尚未重跑全量。

## 3. 未完成

| 编号 | 内容 | 说明 |
|---|---|---|
| W-B3-28 | 构造 `InsightExtract` 请求、能力前置、Schema 严格校验、违规后兜底 | 依赖 `StructuredLlmGateway` 替身 |
| W-B3-29.3 | 四类计数与 `unclassified=0` | 需接入抽取链路 |
| W-B3-31 | 字段补全闭环（决策 owner、待办 assignee、疑问 10 s） | 部分已实现（`fill_fields`），未接入链路 |
| W-B3-35.4 | service 侧“合并单元只提炼一次” | 需与 `CaptureChannelService` 装配方协作 |
| W-B3-40 | `realistic_corpus_has_zero_source_missing`、`duplicate_text_draft_is_counted_once` | 未实现 |
| P3c | 6 条规则、全量回归、四处回滚演练 | 未开始 |
| D-B3-12 | 三类网关失败分类（不可用/Schema/冷却） | 仅覆盖空主题与乱序 |

## 4. 偏差登记

| 编号 | 偏差 | 处置 |
|---|---|---|
| D-B3-09 | `TextRange` 实际在 `model::session`，不是 `vo` | 已修正导入 |
| D-B3-10 | `UtteranceText` 自身限 2000 字，无法承载超长原文 | 切分入参改为 `&str`，输出仍为 `UtteranceText` |
| D-B3-11 | 新建 `understand/mod.rs` 时覆盖既有文件 | 已恢复 `stable_prefix` 模块与 `StablePrefix` 导出 |
| D-B3-12 | 网关失败分类仅两类 | 留待 `StubGateway` 接入 |
| D-B3-13 | `merge_drafted` 的窗口由调用方传入 `created_ms` | `WallClock` 无公开毫秒读取接口；不新增字段 |

## 5. 结论

- 离线可判定的理解层内核（归一化、摘要、抽取规则、拆分、去重三态、补充要点、存储与操作）已闭环并有测试。
- **不宣称 B3 完成**：网关结构化抽取与 service 接线缺失，P3c 门禁未跑。
- macOS 相关能力一律“不适用（无设备）”。
## 6. 门禁与回滚演练

### 6.1 静态门禁（`scripts/check-b3-gates.sh`）

| 规则 | 结果 |
|---|---|
| `source`（无真实计时/睡眠） | 0 |
| `no-truncate`（无 `ShortText::truncated`） | 0 |
| `zero-transcript`（日志不含原文/说话人/端点） | 0 |
| `fallback-untouched`（`fallback.rs` 零 diff） | 0 |
| `new-targets`（3 个新 target 已登记） | 0 |
| `exports`（门面/守护/存储/合并 API 已导出） | 0 |
| `--self-test` | 1（植入违规被识别，符合预期） |

### 6.2 回滚演练

| # | 单元 | 破坏后 | 恢复后 |
|---|---|---|---|
| 1 | `NEW_TARGETS` 去掉 `insight_dedup` | `new-targets` = 1 | 0 |
| 2 | 删除 `InsightStore` 导出 | `exports` = 1 | 0 |
| 3 | B3 开关关闭 | 由 `disabled_feature_is_zero_effect` 证明 | — |
| 4 | 领域类型（`InsightSources` 非空） | 由 `insight_sources_reject_empty` 证明 | — |

演练均使用 `.tmp` 备份恢复，未残留改动。
