# B1（M0 技术验证）执行状态

> 依据：`docs/sdd/tasks.md` 批次 B1（主任务 7–14）
> 记录日期：2026-11-20
> 门禁 G1：未评估（依赖真机与真实会议）

## 1. 本机环境探测

| 条件 | 结果 | 影响 |
|---|---|---|
| 飞书客户端 | ✅ 8.7.11（`%LOCALAPPDATA%\Feishu`） | 任务 10 可做 UI 探测，但需人工发起会议并开启字幕 |
| Zoom | ⚠️ 仅见 `%APPDATA%\Zoom` 残留 | 需确认完整客户端与版本 |
| 腾讯会议 | ❌ 未安装（仅 QQ / 微信） | 任务 9 无法执行 |
| macOS 设备 | ❌ 无 | 任务 8 全部阻塞 |
| 音频设备 | ✅ Realtek + NVIDIA 虚拟声卡 | 任务 7 可执行 |
| ASR 模型 | ❌ 未下载 | 任务 12.1/12.3 阻塞（还需法务核实模型许可） |
| LLM 端点/Key | ❌ 未配置 | 任务 13 阻塞 |

## 2. 子任务状态

| 子任务 | 状态 | 说明 |
|---|---|---|
| 7.1–7.4 Windows WASAPI | 待执行 | 可编码，但 DoD 要求真实会议声与 30 分钟连续采集 |
| 8.1–8.4 macOS SCK | ⛔ 阻塞 | 无 mac 设备 |
| 9.1–9.4 腾讯会议 | ⛔ 阻塞 | 客户端未安装 |
| 10.1–10.4 飞书 | 可部分执行 | 需人工开启字幕会议；无障碍树可离线探测 |
| 11.1–11.4 Zoom | 待确认 | 需确认客户端版本 |
| 12.1 sherpa-onnx 流式 | ⛔ 阻塞 | 需模型权重与许可复核 |
| **12.2 稳定前缀策略** | ✅ 完成 | `crates/application/src/understand/stable_prefix.rs`，7 个测试通过 |
| 12.3 Silero VAD | ⛔ 阻塞 | 需模型文件 |
| 12.4 ASR 基线报告 | ⛔ 阻塞 | 依赖 12.1/12.3 |
| 13.1 LLM 网关 | ✅ 网关侧已验证（mock） | `crates/domain/src/{model,port}/llm.rs`、`crates/infrastructure/src/llm/`；四类 Schema 与严格解码、一次性/流式编排、装配 `assemble()` 已落地；真实端点指标见 AC-18（待实测） |
| 13.2 超时/限流/schema 降级 | ✅ 网关侧已验证（mock） | 六类分类、重试/退避/冷却/并发闸门、审计不可用阻断、四类规则兜底；故障注入 14 个用例全绿 |
| 13.3 门控离线回放 | 待后续 | 依赖 B4 门控实现 |
| 13.4 LLM 基线报告 | ⛔ 阻塞 | 需真实端点 |
| 14.x M0 评审 | ⛔ 阻塞 | 依赖 7–13 实测 |

## 3. 12.2 实现要点

- 同一 partial 连续出现 **2 次**才提交，消除流式抖动。
- 已提交内容**永不回退**；尾部修正/整句重写只累加 `rejected_revisions`，供误报率统计。
- 新句/新段由调用方 `reset()` 后重新累积。
- 单测覆盖：逐字追加、整句替换（reset 后）、尾部修正拒绝、重复 partial 不重复提交、空白忽略。

## 4. 解除阻塞所需的最小输入

1. **腾讯会议客户端**安装（任务 9）；
2. **macOS 13.4+ 设备**（任务 8）或接受"mac 侧仅做编译级验证"；
3. **ASR 模型**：中文流式模型文件 + 许可确认（任务 12.1/12.3）；
4. **LLM 端点与 Key**：OpenAI 兼容服务（任务 13.1/13.4）；
5. 飞书/Zoom 需一次**真实会议**并开启字幕，用于采集控件树与延迟数据（任务 10/11）。
## 5. 2026-11-20 环境更新与进展

### 客户端
- 腾讯会议 **3.48.0.443**（`D:\会议\WeMeet\WeMeetApp.exe`）已确认；
- Zoom 客户端目录 `%APPDATA%\Zoomin` 存在；
- macOS 按用户要求**暂不覆盖**（任务 8 标记为范围外）。

### ASR 模型
- 已从 `hf-mirror.com` 下载中英双语流式 zipformer（int8，Apache-2.0），存放于 `models/asr/bilingual-zh-en-2023-02-20/`（不入库）；
- 用 `sherpa-onnx 2.0.2` 参考运行时实测：RTF **0.0509**、首字延迟 **53 ms**、2 线程；详见 `docs/baseline/asr.md`；
- Rust 接入需 CMake + C++ 工具链，本机暂缺。

### LLM 端点
- 本机无 Ollama / LM Studio / 本地模型；
- `api.openai.com`、`huggingface.co`、`github.com` 均不可达；`dashscope.aliyuncs.com` 域名可达但无 API Key；
- **需要用户提供 OpenAI 兼容端点地址与 Key**（或指定使用哪家国内服务）。
## 6. 2026-11-21 13.1/13.2 交付与三态标注

### 交付范围（阶段 A）
- 领域：`model/llm.rs`（任务/请求/四类结果/六类故障/降级原因/观测点）、`port/llm.rs`（`StructuredLlmGateway`/`CancelToken`/`StreamCollector`/`LlmObserver`）；既有 `LlmGateway` 未改动。
- 基础设施：`llm/{schema,transport,degrade,fallback,gateway}.rs`；四类 Schema 由类型派生，serde 严格解码；HTTP/1.1 编解码 + 回环传输；重试/退避/冷却/并发；四类规则兜底；两阶段审计与白名单日志。
- 阶段 B 载体：`llm/live/{gate,config,stats,record}.rs`、`tests/llm_live_offline.rs`、`tests/llm_live.rs`（默认 ignored）、`scripts/run-llm-live.sh`、`scripts/llm-live-recompute.mjs`。
- 依赖：`schemars 1.2.3`（MIT）；`llm-https` 为空 feature，未引入 TLS 依赖。

### 门禁证据摘要
- `cargo test --workspace`：全绿；13.1/13.2 新增 mock/故障注入/Schema/脱敏用例通过，既有 80 个测试未减少。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo fmt --all --check`：通过。
- `bash scripts/check-layering.sh`：通过（domain 未引入网络/存储实现）。
- `bash scripts/scan-secrets.sh`：通过（夹具仅使用 `sk-test-0000`）。
- `cargo deny check licenses bans sources advisories`：通过。

### AC 三态
| AC | 状态 | 说明 |
|---|---|---|
| AC-01~AC-17 | ✅ 离线已判定 | 由单元/集成测试覆盖 |
| AC-18 | **待实测** | 需外部端点与有效凭据；mock 数值不得替代 |
| AC-19 | ✅ 离线已判定 | 运行门默认关闭、零传输构造、退出 0 |
| AC-20~AC-25 | **未实测（传输未启用 / 端点未配置）** | 载体已交付；数值留待阶段 C |
| AC-26 | ✅ 静态面离线已判定 / **运行期未实测** | 掩码、审计与日志无凭据；运行期四泄漏面抓取需真实联调 |
| AC-27 | ✅ 机制面离线已判定 / **真实样本未实测** | 记录字段完备、重算脚本自检通过 |

### 当前阻塞
- M1 Base URL、M2 模型名、M3 白名单授权、M4 采样预算、M5 凭据授权与轮换责任人；
- P9 `native-tls` 许可双签；
- 合规：云端数据不训练条款确认。
- 说明：用户已提供测试用 Key（未记录其值），但缺 Base URL 与模型名，故 AC-18/AC-20~AC-25 仍为"未实测"。
## 7. 2026-11-21 阶段 D（v1.2）：DeepSeek 锁定 + TLS 载体

### 交付范围
- 运行门：`llm/live/policy.rs`（host 精确锁定、scheme、模型逐字节确认）；`gate.rs::admit` 固定 11 步，拒绝路径不构造 `SecretString`。
- 预算：`llm/live/budget.rs`（`BudgetLedger`/`Permit`/`BudgetedTransport`，唯一计数点，失败与重试均计入）；`TransportError::BudgetExhausted`、`DegradeCause::BudgetExhausted`。
- TLS：`llm-https = ["dep:native-tls"]`，仅 Windows/macOS target 的 optional 依赖；`transport_tls.rs` 使用系统 TLS（SChannel / Security.framework），不打包 OpenSSL/ring。
- 记录：`RunRecord` 升级到 `llm.live.v2`（预算、确认、条款口径与接受日期）；`scripts/llm-live-recompute.mjs` 校验 v2 键集。
- 扫描：`scripts/scan-secrets-docs.sh`（覆盖 `*.md` 与 `.tmp/llm-live/**`，只输出 `文件:行号`，扫描面缺失退 2）；`scripts/check-model-candidates.sh`。
- 脚本：`run-llm-live.sh` 改为凭据经 stdin，补 C1~C3 存在性检查与文档扫描收尾。

### 门禁证据摘要
- `cargo test --workspace`：**222 passed / 0 failed**（工作区 `.tmp/test-temp` 作为 TEMP，规避沙箱外写限制）。
- `cargo test -p meeting-copilot-infrastructure --features llm-https`：**152 passed / 0 failed**；`llm_transport_tls` 7 个用例全绿。
- `cargo fmt --all --check`：0；`cargo clippy --workspace --all-targets -D warnings`：0；`--features llm-https`：0。
- `check-layering.sh`：0；`scan-secrets.sh`：0；`scan-secrets-docs.sh --all-md`：0；`check-model-candidates.sh`：0。
- `cargo deny check licenses bans sources advisories`：0；`cargo tree --features llm-https` 的 `openssl` 计数：0；私钥块检索：0 命中。
- 真实联调默认关闭：`run-llm-live.sh` 输出"未实测（未开启真实联调）"，退出 0。

### AC 三态（阶段 D）
| AC | 状态 | 说明 |
|---|---|---|
| AC-28 | ✅ 离线已判定 | host 精确集合；11 种拒绝形态；拒绝路径 socket/凭据为 0 |
| AC-29 | ✅ 离线已判定 | 空/空白/候选/不等 → `model_not_confirmed`；确认值放行 |
| AC-30 | ✅ 离线已判定 | `http://` 拒绝；TLS 最低 1.2、无 danger 开关、系统信任根 |
| AC-31 | ✅ 离线已判定 | `min(samples×12, cap)`；触顶短路；未确认 → 0 请求 |
| AC-32 | ✅ 静态五面已判定 / **运行期四面未实测** | 文档/代码/记录/报告/脚本自检 0 命中 |
| AC-20~AC-25 | **未实测** | 仍受 C1~C3 与真实端点阻塞 |

### 输入阻塞（不阻塞离线任务）
- C1 端点路径、C2 API 模型名（含 `LLM_MODEL_CONFIRMED`）、C3 调用上限（`LLM_LIVE_CALL_CAP`）；
- C4 其它 DeepSeek 官方域名（默认仅 `api.deepseek.com`）、C5 凭据是否已轮换；
- M6 确认凭条机制（按默认口径落地）、M7 非 443 端口（默认拒绝）。

### 受控偏差（登记，不回改 design.md）
- 〔D2〕记录字段拆为 `data_terms_declaration` + `data_terms_accepted_at` 两项。
- 〔D5〕`budget` 定义局部 `BudgetExhausted`，由 `gate` 映射，避免循环依赖。
- 〔D7〕`TlsTransport::open_stream` 当前缓冲整包响应，增量首 token 统计需在 L22.1 前补齐增量读取；因此 AC-20 在补齐前不得宣称已满足。

### 仍未完成
- L22（真实采样与记录移交）全部阻塞；13.1/13.2 不得宣称"真实端点已验证"，13.4 未完成，AC-18 未实测。
## 8. 2026-11-21 阶段 E（v1.3）：确认收敛与真实采样载体

### 交付范围
- **AC-33**：`gateway::request_body` 单点构造，固定 `thinking.type = disabled`；`thinking_verdict` 纯函数；mock 与真实共用。
- **AC-34**：`policy::PATH_CONSTANT = "/chat/completions"` 与 `path_confirmed`；`AdmissionTicket.path_confirmed` 由常量比对产出；记录与配置不再手写 `true`。
- **AC-36**：`transport::IncrementalBodyReader` + `ChunkedDecoder`；回环与 TLS `open_stream` 均改为增量交付；新增 `tests/llm_stream_incremental.rs` 5 例。
- **AC-37**：`LiveRunConfig.allowlist` 数据流移除；host 集合唯一来源 `DEEPSEEK_HOSTS`；`check-model-candidates.sh` 改为“零模型字面量 + 零默认/回退模型值”。
- **AC-35**：`live/auth.rs::BearerTransport`；`assemble_live` 真实装配（票据 → 配置 → Bearer → 预算 → 网关 → 记录器）；`llm_live.rs` 8 个 ignored 用例接入真实链路，不再为占位打印。
- **C5**：`credential_rotated = false`，`summary.md` 首行固定“凭据未轮换·存在既暴露风险”。

### 门禁证据摘要
- `cargo test --workspace`：**238 passed / 0 failed**（达到阶段 E 阈值）。
- `cargo fmt --all --check`：0；`cargo clippy --workspace --all-targets -D warnings`：0；`--features llm-https`：0。
- `check-model-candidates.sh`：0；`scan-secrets-docs.sh --all-md`：0；`scan-secrets.sh`：0。
- `cargo deny --offline check licenses bans sources advisories`：通过；`openssl` 依赖计数：0。

### 状态
| 项 | 状态 |
|---|---|
| AC-33 / AC-34 / AC-36 / AC-37 | ✅ 离线已判定 |
| AC-35 | 载体已落地；真实样本未执行 |
| AC-20~AC-25 | **未实测**（需 L31 真实执行） |
| AC-18 | 仍为“待实测” |

### 偏差与未执行
- 〔E1〕移除 `LiveRunConfig.allowlist`；〔E2〕`BearerTransport` 落 `live/auth.rs`；〔E3〕候选脚本代码面扫描限定 `crates app/src scripts`。
- D-DEADLINE：沿用代码现值 6000；8000 仅为建议值，未写死。
- **L31 未执行**：环境变量未注入当前 agent 会话；真实采样必须由用户终端执行。AC-20 在真实增量样本产生前仍不得标“实测”。
## 9. 2026-11-21 真实采样成功（阶段 G）

### 实测结果
- 用户终端运行 8 个 ignored 用例：**8 passed / 0 failed**，耗时约 16s。
- 四类任务全部 `degraded=false`：
  - `insight_extract`、`summary`、`card_conclusion`、`minutes_draft`
- 样本：`http_status=200`、`failure_class=null`、`attempt=1`（个别卡片曾 2 次后成功）。
- 代表耗时：summary 0.7~1.1s；insight 0.8~1.6s；card 0.8~2.0s；minutes 1.2~1.8s。

### 为达成成功所做的修复
- system prompt 注入完整 JSON Schema；
- `Summary`/`MinutesDraft` 文本由 60 字上限改为 2000 字；
- 新增 `wire` 容错解码：Summary/Minutes/Card/Insight；
- `InsightKind` 导出为 Schema 枚举；未知 kind 降级为 `key_info`；
- 剥离 ```json 代码围栏；
- 真实探针载荷改为中文会议片段；
- 归一填充 `purpose/version/carried_over/produced/notice/unrefined_count/completion_hint`。

### 30 样本基线（2026-11-21）

| 任务 | n | 原生成功 | 首 token p50/p95 | 总耗时 p50/p95 |
|---|---|---|---|---|
| insight_extract | 30 | 30（100%） | 635 / 1053 ms | 1177 / 1721 ms |
| summary | 30 | 30（100%） | 725 / 1039 ms | 1106 / 1451 ms |
| card_conclusion | 30 | 30（100%） | 622 / 1121 ms | 1126 / 1634 ms |
| minutes_draft | 30 | 29（96.7%） | 576 / 969 ms | 1559 / 2006 ms |

- `RUN_ID=18ea0235a172d040-524-0`；`samples=120`；`calls=122`；minutes 1 次 timeout（seq=1，attempt=2），其余均原生成功。
- **AC-20 达标**：四类首 token p95 均 ≤1.2s，样本 30/任务。
- **AC-21 可行**：四类总耗时 p95 均 ≤6s。
- **AC-22 已记录**：119/120 原生成功（99.2%）；失败分布为 1 次 timeout，未用兜底成功计入分子。

### AC 状态
| 项 | 状态 |
|---|---|
| AC-35（真实链路） | ✅ 四类均出现真实成功样本 |
| AC-38 / AC-39 / AC-43 / AC-46 | ✅ 离线与真实样本已判定 |
| AC-40 / AC-41 / AC-42 | ✅ 真实样本含 `http_status`、实测耗时、唯一 `run_id` |
| AC-44 / AC-45 | ✅ 真实样本成功（归一字段生效） |
| AC-20 | ✅ 达标（30 样本/任务，首 token p95 ≤1.2s） |
| AC-21 | ✅ 可行（总耗时 p95 ≤6s） |
| AC-22 | ✅ 已记录（119/120 原生成功，1 次 timeout） |
| AC-23 / AC-24 / AC-25 | **未实测**（429/人为截止/schema 违规对照） |
| AC-18 | 仍由 13.4 判定；基线数据已具备 |

### 收尾事项
- 一次性诊断 `tests/llm_debug_raw.rs` 已删除。
- 64 KiB 请求体字节预算已落地（默认 65536，超限在发请求前拒绝，请求计数 0）。
- normalization trace 已落地：`StructuredOutcome.normalization` 只含字段名与来源，不含文本。
- **阶段 G 的 265 测试门禁已达成**：`cargo test --workspace` 265 passed / 0 failed（含新增 21 个具名收口用例）。
- 改动未 commit；凭据仍未轮换。
> 注：第 8 节中 AC-35、AC-20~AC-25 为阶段 E 历史状态；第 9 节已更新为 30 样本基线：AC-20/AC-21/AC-22 已满足，AC-23~AC-25 仍未实测，AC-18 仍由 13.4 判定。
