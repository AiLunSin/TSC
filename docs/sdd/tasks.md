# AI 会议辅助桌面客户端（Meeting Copilot）—— 编码任务清单

> **依据文档**
> - 需求规格：`.codeartsdoer/specs/meeting_copilot/spec.md`（941 行，5.1–5.8 核心能力 + 6.1–6.10 数据约束）
> - 技术方案：`.codeartsdoer/specs/meeting_copilot/design.md`（1699 行，含 2.1 实现模型、2.2 接口、2.3 数据模型、2.5 平台接入、2.6 门控、2.7 合规）
> - 里程碑口径：design 2.8.2（M0 3 周 / M1 6 周 / M2 5 周 / M3 4 周 + 4 周缓冲）
>
> **本文边界**：仅做任务拆解与验收定义，不含实现代码。编码由后续阶段按本清单执行。

---

## 0. 使用说明

### 0.1 编号与层级规则

| 项 | 规则 |
|---|---|
| 批次 | `B0 … B10`，按依赖顺序排列，共 11 批 |
| 主任务 | 全局连续编号 `1 … 79`，不可重复、不可跳号 |
| 子任务 | 形如 `1.1`、`1.2`，**两级为止**，是最小可执行与可勾选单元 |
| 追溯 | 每个子任务标注 `追溯 SPEC x.y.z-n · DESIGN m.n`，格式与 design 1.1 一致（`SPEC 5.3.1-2` = 5.3.1 业务规则第 2 条） |
| DoD | Definition of Done，必须是可机器判定或可人工复现的客观标准，禁止"已完成/正常"式表述 |

### 0.2 图例

| 标记 | 含义 |
|---|---|
| 🔴 CP | 关键路径任务，延期直接推迟里程碑 |
| 🟢 PAR | 可与其他任务并行，无数据依赖 |
| ⛔ GATE | 门禁任务，未通过不得进入下一批次 |
| 🧩 SPI | 需第三方/开放适配的任务（检索源、平台适配器） |
| 🛡 合规 | 涉及隐私、审计、许可，须安全/法务参与 |

### 0.3 路径约定

仓库当前为空项目（design 1 前置说明已确认），以下路径为 design 2.1.2 / 2.4.1 约定的目标结构，首次落地时以「任务 1」评审结论为准：

```
app/                          # Tauri 2 应用壳
  src/                        # React + TS 前端
  src-tauri/                  # Rust 主程序与 IPC 注册
crates/
  domain/                     # 领域层：模型、值对象、端口 trait、Guardrail 纯函数
  application/                # 应用层：SessionOrchestrator / InsightService / ProactivePipeline 等
  infrastructure/             # 基础设施：存储、审计、LLM/检索/抓取、度量、通知
  platform-win/               # Windows：WASAPI、UIA、Windows.Media.Ocr
  platform-mac/               # macOS：ScreenCaptureKit、AX、Vision
  ui-bridge/                  # DTO 与 serde/TS 绑定
docs/
  m0/                         # 技术验证实测报告
  baseline/                   # 性能与 ASR 基线
  compliance/                 # 许可、SBOM、隐私检查记录
scripts/                      # CI 辅助脚本（分层校验、SBOM、残留扫描）
```

### 0.4 批次总览

| 批次 | 周期 | 主任务 | 交付主题 | 前置批次 | 并行度 |
|---|---|---|---|---|---|
| **B0** 工程基座与合规前置 | W1–W2 | 1–6 | 可构建/可加密/可审计的空壳；契约冻结 | — | 高（4 条泳道） |
| **B1** M0 技术验证 | W3–W5 | 7–14 | 三平台字幕可行性、双系统音频、ASR/LLM 基线 | B0 | 高（平台互不依赖） |
| **B2** 采集接入工程化 | W6–W11 | 15–27 | 三平台通道 + 仲裁 + 双通道合并 + 会话状态机 | B1 | 中（Win/mac/三平台并行） |
| **B3** 理解层 | W9–W11 | 28–33 | 归一化、摘要、要点、缺口识别、恢复 | B2.24 | 中（可与 B2 后半并行） |
| **B4** 检索与全自动提示 | W12–W16 | 34–43 | 开放检索源、抓取、脱敏、五级门控、去噪 | B3、B0 | 中（检索层与生成层可并行） |
| **B5** 呈现与交互 | W14–W17 | 44–49 | 三形态推送、投屏保护、紧急遮蔽、要点流 | B4.40 | 高（前端独立推进） |
| **B6** 治理 | W15–W17 | 50–54 | 策略引擎、授权、审计接入、权限、留存删除 | B0.5/0.6 | 中 |
| **B7** 纪要与导出 | W17–W19 | 55–58 | 纪要合成、确认归档、三格式导出、历史检索 | B3、B6 | 中 |
| **B8** 配置与质量 | W18–W19 | 59–64 | 术语表、源配置、平台配置、偏好、质量报表 | B4、B6 | 高（页面独立） |
| **B9** 测试与门禁 | W19–W21 | 65–75 | 单测/集成/E2E、三平台矩阵、性能、故障注入、隐私 | 全部功能批次 | 高 |
| **B10** 打包发布 | W21–W22 | 76–79 | Win/mac 交付物、发布门禁、适配 SOP | B9 | 低 |

### 0.5 关键路径

```
1.1 → 3.1 → 4.2 → 5.1 → 15.x(Win 音频) ─┐
                      → 16.x(mac 音频) ─┼→ 23.x(仲裁) → 24.x(合并) → 26.x(会话状态机)
                      → 17.x(字幕框架) → 18/19/20(三平台) ─┘
                                                      ↓
28.x(归一化) → 30.x(要点提炼) → 32.x(缺口识别) → 40.x(门控管线) → 44.x(推送)
                                                      ↓
                                        55.x(纪要) → 72.x(性能门禁) → 78.x(发布门禁)
```

**关键路径判据**：
1. 三平台字幕可行性（B1 任务 9/10/11）是整个计划的**最大不确定性**，任一平台无可用路径将使 M1 范围与指标承诺失真（design 2.8.2 末尾判断）。
2. 任务 24（双通道去重）与 40（五级门控）是两个质量瓶颈，返工成本最高，必须预留缓冲。
3. 任务 72 性能门禁依赖 15/16/17/30/40 全部就绪，任何一项延后都直接压缩 M3。

### 0.6 可并行泳道（人员视角）

| 泳道 | 主要负责批次 | 说明 |
|---|---|---|
| L1 Rust 桌面/音频 | B0.1/B0.5、B2.15/16/25/26、B9.70/71 | 与 L2 在 B2 期间共享 crate 边界 |
| L2 Windows 平台适配 | B1.7/9、B2.18/21/22、B9.67/70 | 🟢 与 L3 完全并行，仅共享 `PlatformAdapter` 抽象 |
| L3 macOS 平台适配 | B1.8/9、B2.18/21/22、B9.67/68/71 | 🟢 与 L2 完全并行 |
| L4 前端 | B0.1.2、B5 全部、B8 全部 | 🟢 B5 起可完全独立于后端（依赖 IPC DTO 冻结） |
| L5 AI/算法 | B1.12/13、B3 全部、B4.38/39/40/41 | 🟢 B3 可用回放集离线开发，不阻塞采集 |
| L6 QA/自动化 | B0.1.4、B9 全部 | 🟢 门禁脚本自 B0 起持续建设 |
| L7 安全与合规 | B0.2/0.6、B6 全部、B9.74、B10.78 | 🛡 需在 M0/M2/M3 三个节点签署 |

---

## 批次 B0 · 工程基座与合规前置（W1–W2）

**目标**：得到可构建、可加密落盘、可审计、可观测的空壳客户端，并冻结接口契约，使后续 7 条泳道可以并行。
**前置**：无。
**⛔ 出口门禁 G0**：三平台可启动空壳；端口契约 v1 评审通过；CI 中许可红线与分层校验生效；密钥扫描 0 命中。
**并行**：任务 1/2/3/4 可并行；5、6 依赖 3 的类型定义。

### 1. 搭建 Tauri 2 桌面工程与五层骨架 🔴 CP · 🟢 PAR

- [ ] **1.1 建立 Rust workspace 与五层 crate 分层** — `app/src-tauri/Cargo.toml`、`crates/{domain,application,infrastructure,platform-win,platform-mac,ui-bridge}`
  - 追溯 SPEC 1.4-1、4.5-1 · DESIGN 2.1.2、2.4.1
  - DoD：`cargo build --workspace` 在三平台四架构（win-x64、win-arm64、mac-arm64、mac-x64）全部通过；`scripts/check-layering.sh` 断言 `domain` 的依赖闭包不含 `infrastructure`/`platform-*`/网络 crate；`cargo tree -d` 无重复大版本共存
- [ ] **1.2 前端脚手架 React + TS + Vite（严格模式）** — `app/src/`、`tsconfig.json`、`eslint.config.js`
  - 追溯 SPEC 4.1-5 · DESIGN 2.4.1
  - DoD：`tsconfig` 开启 `strict`、`noUncheckedIndexedAccess`、`exactOptionalPropertyTypes`；ESLint 规则 `no-explicit-any: error`；`pnpm build` 产物 gzip ≤ 800KB（不含系统 WebView）；lint 0 error
- [ ] **1.3 定义 IPC 通道与 DTO 契约** — `crates/ui-bridge/src/**`、`app/src/bridge/**`
  - 追溯 SPEC 6.1–6.9 · DESIGN 2.2.1
  - DoD：所有跨 IPC 业务 DTO 为强类型 struct/enum，配置类保留未知字段透传、非配置类 `#[serde(deny_unknown_fields)]`；`pnpm gen:bindings` 生成 TS 类型与实际 DTO 逐字段一致（编译期断言 + 快照测试）；IPC 调用统一走 `invoke` 封装，前端无裸 `any`
- [ ] **1.4 建立三平台 CI 矩阵** — `.github/workflows/ci.yml`、`scripts/ci/*`
  - 追溯 SPEC 4.5-1 · DESIGN 2.8.2
  - DoD：push 触发 4 个 target 的 build + test + `clippy -D warnings` + `fmt --check`；单次流水线 ≤15 min；失败时输出可复现命令；缓存命中率 ≥70%

### 2. 依赖许可与 SBOM 门禁 🛡 合规

- [ ] **2.1 配置 cargo-deny 白名单与红线** — `deny.toml`
  - 追溯 SPEC 4.5-1 · DESIGN 2.4.2、2.8.1-R9
  - DoD：`cargo deny check licenses bans sources advisories` 通过；显式拒绝 AGPL/GPL/SSPL 的**链接与分发**（允许 `license = "GPL-3.0"` 仅出现在 dev/示例说明）；命中即 CI 失败
- [ ] **2.2 生成并归档 CycloneDX SBOM** — `scripts/sbom.sh`、`docs/compliance/sbom/`
  - 追溯 SPEC 4.5-1 · DESIGN 2.4.2
  - DoD：每次发布构建产出 `sbom.cdx.json`（含 Rust + npm 两侧）；SBOM 可被 `cyclonedx-cli validate` 通过；归档路径按版本号命名
- [ ] **2.3 建立高风险组件清单与使用边界** — `docs/compliance/license-review.md`
  - 追溯 SPEC 1.4-3 · DESIGN 2.4.2、2.1.1
  - DoD：SearXNG（AGPL-3.0）标注"仅外部 HTTP 接入、不内置不分发"；BlackHole（GPL-3.0）标注"不随包、仅文档提及"；obs-studio（GPL-2.0）标注"只读参考"；每项有复核人签字栏（M0 前完成签署）

### 3. 领域模型、值对象与错误模型 🔴 CP

- [ ] **3.1 实现约束型值对象** — `crates/domain/src/vo/`
  - 追溯 SPEC 6.3-4、6.4-3/4、6.5-2、6.2-4 · DESIGN 2.2.2-C、2.2.1
  - DoD：`ShortText` 构造期校验 ≤60 字、`TitleText` ≤40 字、`ConclusionText` ≤80 字、`UtteranceText` ≤2000 字、`Confidence` ∈[0,1]、`SanitizedQuery` 私有构造仅 `Sanitizer` 可产出；属性测试（proptest）覆盖边界值与越界拒绝；越界返回 `Err` 不 panic
- [ ] **3.2 实现领域聚合与枚举** — `crates/domain/src/model/`
  - 追溯 SPEC 6.1–6.10 · DESIGN 2.3.2
  - DoD：`Session/PlatformBinding/Utterance/Correction/RollingSummary/Insight/InsightFields/GapCandidate/SuggestionCard/Reference/TraceChain/LexiconTerm/EffectivePolicy/AuditRecord/ProviderProfile/OutboxTask` 全部落地；`SessionState` 六值、`InsightKind` 四值、`SourceKind` 四值、`TriggerKind` 二值、`CardStatus` 三值、`TrustTier` 四值、`DenyCode` 十一值；时间用 `AudioClock`/`WallClock` 两个 newtype 区分（DESIGN 2.3.1 关键建模决策）
- [ ] **3.3 实现错误模型与错误码映射** — `crates/domain/src/error.rs`
  - 追溯 SPEC 5.1.3、5.2.3、5.3.3、5.5.3、5.6.3、5.7.3 · DESIGN 2.2.1
  - DoD：`CopilotError` 覆盖 `E-AUDIO-001/002`、`E-ASR-002`、`E-STORE-003`、`E-PRIV-004`、`E-SEARCH-005`、`E-MIN-006`、`E-CONSENT-007`、`E-AUTH-008`、`ProviderConfigInvalid`、`AdapterVersionIncompatible`、`NetworkOffline`；每个变体有 `code()`、中文 `user_message()`、`is_retryable()`；单元测试保证 11 个码无遗漏
- [ ] **3.4 实现枚举未知值降级策略** — `crates/domain/src/model/unknown.rs`
  - 追溯 SPEC 4.5-8 · DESIGN 2.2.1
  - DoD：读取旧数据遇未知枚举值映射为 `Unknown(raw)` 并产生一条告警，不 panic、不丢弃整条记录；序列化往返测试通过（未知值原样保留）

### 4. 定义领域端口与适配器 SPI 🔴 CP

- [ ] **4.1 采集端口** — `crates/domain/src/port/capture.rs`
  - 追溯 SPEC 5.8.1-2 · DESIGN 2.2.2-A
  - DoD：`SubtitleSource`（descriptor/probe/subscribe/health/release）、`AudioSource`（descriptor/open/next_chunk/devices/health）与 `SubtitleEvent`/`SubtitleCapability`/`AudioChunk` 定义完成；`Subscription` 令牌在 `release` 后拒绝回调（有单测）
- [ ] **4.2 理解与生成端口** — `crates/domain/src/port/understand.rs`
  - 追溯 SPEC 5.2.1、5.3.1-2 · DESIGN 2.2.2-B/C
  - DoD：`Transcriber`+`TranscriptEvent{Partial,Final,Correction}`、`RollingSummarizer`、`InsightExtractor`、`GapDetector`、`Deduplicator`、`LlmGateway`、`CardComposer` 定义完成；`ExtractionInput`/`InsightDraft`/`GapCandidate`/`CardDraft` 字段与 DESIGN 2.2.2 一致
- [ ] **4.3 检索端口（开放可配置的核心）** 🧩 SPI — `crates/domain/src/port/search.rs`
  - 追溯 SPEC 4.5-6、5.3.1-7、6.9 · DESIGN 2.2.2-D
  - DoD：`SearchProvider`（profile_id/kind/test_connection/search）、`SearchBroker`、`PageFetcher`、`ProviderRequest`（含 `SanitizedQuery`、limit 1..=10、timeout ≤8s、blocked_domains）、`ProviderResponse`、`FetchOutcome{Ok,Skipped,Failed}` 定义完成；SPI 不含任何具体服务商常量（断言：源码中无 `searxng|tavily|brave` 等字样于 `domain`）
- [ ] **4.4 治理与存储端口** — `crates/domain/src/port/governance.rs`、`.../repository.rs`
  - 追溯 SPEC 5.6.1、4.3、6.7/6.8 · DESIGN 2.2.2-F/G
  - DoD：`PolicyResolver`、`Guardrail::evaluate -> Decision{Allow|Deny{code}}`（纯函数，无 IO）、`Sanitizer`（sanitize_query/sanitize_output/scan）、`AuditLog`、`Presenter`（push_card/set_mode/emergency_blackout/set_screen_share_protection/notify）、四个 Repository 定义完成
- [ ] **4.5 Mock 实现与端口一致性测试** — `crates/domain/tests/contracts/`
  - 追溯 SPEC 4.5-6、5.8.1-1 · DESIGN 2.1.2（"便于单测与替换"）
  - DoD：每个端口有 `Mock*` 实现；端口契约测试（同一套断言可跑 mock 与后续真实实现）就位；`spiVersion` 不兼容时返回 `AdapterVersionIncompatible` 且有测试

### 5. 加密本地存储与密钥分层 🔴 CP · 🛡 合规

- [ ] **5.1 SQLCipher 接入与迁移框架** — `crates/infrastructure/src/store/`
  - 追溯 SPEC 4.3-3、4.5-8 · DESIGN 2.3.1、2.7.3
  - DoD：SQLCipher 打开需密钥，无密钥打开返回失败（不得静默降级为明文）；WAL 开启；`schema_version` 表与只读升级框架就位；迁移脚本具备"旧库→新库"回放测试（v0→v1 样本库）
- [ ] **5.2 密钥分层与轮换** — `crates/infrastructure/src/crypto/`
  - 追溯 SPEC 4.3-3 · DESIGN 2.7.3
  - DoD：L0 主密钥 32B 随机，存 Windows DPAPI（当前用户）/ macOS Keychain（`AfterFirstUnlockThisDeviceOnly`）；L1 由 L0+应用标识+随机盐经 HKDF-SHA256 派生且仅内存持有；L2 字段级 AES-256-GCM 信封加密，AAD 绑定 `session_id`；`rekey` 失败即拒绝启动（有测试）
- [ ] **5.3 实现 Repository 与三写同事务** — `crates/infrastructure/src/store/repo/`
  - 追溯 SPEC 6.1–6.5、5.5.1-5 · DESIGN 2.3.1、2.3.2
  - DoD：`SessionRepository`（create/transition/get/delete_irrecoverable）、`UtteranceRepository`、`InsightRepository`、`CardRepository` 实现完成；Utterance 与 Insight 的主表 + FTS5 + sqlite-vec 在同一事务内写入，回滚测试（注入写失败）后三者一致
- [ ] **5.4 密钥与明文残留扫描** — `scripts/scan-secrets.sh`、`crates/infrastructure/tests/no_plaintext.rs`
  - 追溯 SPEC 4.3-3 · DESIGN 2.7.8-3
  - DoD：扫描安装目录、配置、日志、崩溃转储，密钥/数据库口令命中 0；校验配置文件内 `authRef` 仅为引用（`keychain://` / `dpapi://`）不含明文；该脚本接入 CI

### 6. 审计链与可观测底座 🛡 合规

- [ ] **6.1 只追加审计表与哈希链** — `crates/infrastructure/src/audit/`
  - 追溯 SPEC 6.8-1~7、4.3-7 · DESIGN 2.7.4
  - DoD：表字段 `(audit_id, action, actor, occurred_at, target, sanitized_summary, prev_hash, self_hash)`；`self_hash = H(prev_hash ‖ canonical(fields))`；每 1000 条写锚点；篡改任意一条后 `verify()` 能定位到该条（测试）；删除/更新 API 不存在（编译期不可达）
- [ ] **6.2 审计失败降级与敏感操作禁用** — `crates/application/src/governance/audit_guard.rs`
  - 追溯 SPEC 5.6.3-4 · DESIGN 2.1.3(9)、2.7.4
  - DoD：注入审计写失败后，导出/上传/删除/配置变更四类操作被拒绝并返回明确原因；恢复后自动补写缓冲并标记 `backfilled=true`；状态可被 UI 查询
- [ ] **6.3 脱敏日志与诊断包** — `crates/infrastructure/src/telemetry/`
  - 追溯 SPEC 4.4-2、4.4-5 · DESIGN 2.7.1、2.7.3-L4
  - DoD：结构化字段白名单（禁止 session 原文、URL 全文、查询全文、凭据）；环形日志默认 ≤50MB 自动滚动；1000 条抽样扫描中姓名/电话/邮箱/身份证/URL/查询全文命中 0；诊断包含平台类型、客户端版本、通道状态、失败原因分类（满足 4.4-5）
- [ ] **6.4 本地度量库与指标落点** — `crates/infrastructure/src/metrics/`
  - 追溯 SPEC 4.4-1、4.4-4 · DESIGN 2.6.6
  - DoD：10 项指标（要点时延 p95、提示时延 p95/p99、自动触发率、误触发率、转写失败率、分源失败率、分平台字幕失效率、采纳率、误报率、活跃会话数）全部有埋点与存储；按会话/平台/主题三维度可聚合；指标计算不上传原文（代码审计确认）

---

## 批次 B1 · M0 技术验证（W3–W5）

**目标**：用**真实会议**验证三平台字幕可读性、双系统音频可捕获性、ASR/LLM 指标可达性，产出可引用的实测数据与 CapabilityMatrix 初版。
**前置**：B0（G0 通过）。
**⛔ 出口门禁 G1（M0）**：三平台各取得 ≥1 条可用字幕路径并给出实测延迟；Win/mac 音频均跑通；基线报告证明关键指标可达（允许含"需云端转写"的结论）。
**并行**：7 与 8 并行；9/10/11 三平台并行（各自独立环境）；12 与 13 并行。

### 7. Windows 系统音频捕获原型（WASAPI）🟢 PAR

- [ ] **7.1 设备级 Loopback 采集** — `crates/platform-win/src/audio/proto/device_loopback.rs`
  - 追溯 SPEC 5.8.1-2a · DESIGN 2.5.4、1.2.1-1
  - DoD：默认渲染设备 48kHz/16bit/2ch 采集成功，重采样为 16kHz mono；连续 30 分钟采集丢帧率 ≤1%；能录到腾讯会议与 Zoom 播放的会议声
- [ ] **7.2 进程级 ProcessLoopback（Win10 2004+）** — `crates/platform-win/src/audio/proto/process_loopback.rs`
  - 追溯 SPEC 5.8.1-2a · DESIGN 1.2.1-1
  - DoD：按会议客户端 PID 隔离采集，同期播放的音乐/通知声在输出中**不可检出**（频谱 + 人耳双验证）；能力探测能正确识别系统版本，Win10 1909 上返回"不支持进程级"
- [ ] **7.3 失败检测与麦克风回退** — `crates/platform-win/src/audio/proto/fallback.rs`
  - 追溯 SPEC 5.8.3-5 · DESIGN 1.2.1-1、2.5.4
  - DoD：构造独占模式/虚拟声卡冲突场景，系统返回 `AudioCaptureUnavailable` 并自动回退麦克风、标记"仅麦克风"；不崩溃、不静默吞错；每种失败有独立原因分类
- [ ] **7.4 输出 Windows 音频可行性结论** — `docs/m0/win-audio.md`
  - 追溯 SPEC 4.2-7 · DESIGN 2.8.2-M0
  - DoD：记录各回退链实测时延、丢帧、CPU、混入非会议声源比例；明确"设备级 fallback 的混音问题"的告知文案建议；结论可被 M1 直接引用

### 8. macOS 系统音频捕获原型（ScreenCaptureKit）🟢 PAR

- [ ] **8.1 SCStream 音频采集与应用过滤** — `crates/platform-mac/src/audio/proto/sck_stream.rs`
  - 追溯 SPEC 5.8.1-2a、4.5-1 · DESIGN 2.5.4、1.2.1-2
  - DoD：`SCContentFilter` 按应用过滤后采集到腾讯会议/Zoom 音频；48kHz/2ch → 16kHz mono；macOS 13.4 与 14.x 均连续 30 分钟无中断
- [ ] **8.2 屏幕录制权限申请与拒绝引导** — `crates/platform-mac/src/audio/proto/permission.rs`
  - 追溯 SPEC 4.3-1、5.8.1-3 · DESIGN 1.2.1-2、2.5.4
  - DoD：未授权返回 `E-AUDIO-001` 且不崩溃；提供系统设置深链（`x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture`）；授权后无需重启应用即可采集（或明确记录需重启）
- [ ] **8.3 静音看门狗与 stream 自动重建** — `crates/platform-mac/src/audio/proto/watchdog.rs`
  - 追溯 SPEC 5.8.3-4、4.4-5 · DESIGN 1.2.1-2、2.5.4
  - DoD：模拟音频流中断（≥3s 静音 + VAD 无活动），2s 内检测并重建 stream；重建次数、时刻、版本写入诊断；重建后 2s 内恢复采集
- [ ] **8.4 输出 macOS 音频可行性结论** — `docs/m0/mac-audio.md`
  - 追溯 SPEC 4.2-7 · DESIGN 2.8.2-M0
  - DoD：记录 13.0/13.4/14.x 差异、首启权限拒绝率、重建频率；明确"不得捆绑 BlackHole"的实现约束已在代码中体现（无 GPL 依赖）

### 9. 腾讯会议字幕读取原型 🔴 CP · 🟢 PAR 🧩 SPI

- [ ] **9.1 窗口/浮窗发现与控件树导出** — `docs/m0/tencent-control-tree.md`、`crates/platform-win/src/adapters/tencent/proto/discovery.rs`
  - 追溯 SPEC 5.8.1-2b、5.8.1-4 · DESIGN 2.5.2、2.5.3-L1
  - DoD：字幕浮窗可被稳定发现（≥3 个客户端版本各验证 1 次）；导出控件树快照（角色/名称/层级/文本属性）；窗口标题正则写入 `SelectorProfile` 草案
- [ ] **9.2 UIA TextPattern / AX 事件读取** — `.../proto/uia_read.rs`、`crates/platform-mac/src/adapters/tencent/proto/ax_read.rs`
  - 追溯 SPEC 5.8.1-2b · DESIGN 2.5.3-L2
  - DoD：真实会议（开启字幕）连续 10 分钟读取，字符准确率 ≥90%；输出 `SubtitleEvent` 雏形（text + 观测时间）；事件去抖 250–400ms 有效
- [ ] **9.3 OCR ROI 兜底最小实现** — `.../proto/ocr_fallback.rs`
  - 追溯 SPEC 5.8.3-4 · DESIGN 2.5.3-L4、R2
  - DoD：自绘/不可读场景下 OCR 可出字，准确率 ≥80%；帧率 ≤2fps 时 CPU 增量 ≤5%；产出 ROI 几何计算方式说明
- [ ] **9.4 输出腾讯会议实测报告** — `docs/m0/tencent.md`
  - 追溯 SPEC 5.8.1-7、4.4-5 · DESIGN 2.5.6
  - DoD：包含字幕延迟实测（p50/p95）、准确率、需用户手动开启字幕、共享屏幕遮挡行为、版本矩阵；结论写入 CapabilityMatrix 草案

### 10. 飞书 / Lark 字幕读取原型 🔴 CP · 🟢 PAR 🧩 SPI

- [ ] **10.1 国内版与国际版 UI 差异实测** — `docs/m0/feishu-ui-diff.md`
  - 追溯 SPEC 5.8.1-1、4.5-2 · DESIGN 2.5.2
  - DoD：Feishu（国内）与 Lark（国际）各自记录字幕入口、窗口形态、是否独立浮窗、控件层级差异；确认"两套版本矩阵需分列维护"
- [ ] **10.2 Electron 无障碍树可用性验证** — `crates/platform-win/src/adapters/feishu/proto/a11y_probe.rs`
  - 追溯 SPEC 5.8.1-2b · DESIGN 1.2.2、R2
  - DoD：实测无障碍树是否默认开启、开启后能否读出字幕文本；输出 `text_binding_score` 判定规则（<0.4 即走 OCR）；结论明确"OCR 是必选而非可选"的依据
- [ ] **10.3 OCR 主路径与去抖断句** — `.../proto/ocr_main.rs`
  - 追溯 SPEC 5.8.1-2b · DESIGN 2.5.3-L4、1.2.2
  - DoD：国内版与 Lark 各跑通 ≥1 条路径；字符准确率 ≥85% 或明确写出"不可用"；去抖后同一句不重复出字
- [ ] **10.4 输出飞书实测报告** — `docs/m0/feishu.md`
  - 追溯 SPEC 5.8.1-7 · DESIGN 2.5.6
  - DoD：含客户端版本、字幕延迟、OCR 占比、CPU 增量；明确推荐通道为「字幕(OCR) + 音频并行」的实测依据

### 11. Zoom Closed Caption 读取原型 🟢 PAR 🧩 SPI

- [ ] **11.1 CC 独立窗口发现（5.17+）** — `crates/platform-win/src/adapters/zoom/proto/cc_window.rs`
  - 追溯 SPEC 5.8.1-2b、4.5-2 · DESIGN 2.5.2
  - DoD：CC 开启后可稳定发现窗口；CC 未开启时明确返回"无 CC 窗口"而非错误；≥3 个 Zoom 版本（5.17/5.18/6.x）验证
- [ ] **11.2 UIA/AX 读取与增量 diff** — `.../proto/cc_read.rs`
  - 追溯 SPEC 5.8.1-2b · DESIGN 2.5.3-L2/L3
  - DoD：字符准确率 ≥95%；增量 diff 正确处理"仅当前句"与"滚动多句"两种 CC 呈现；去重后无重复句
- [ ] **11.3 CC 未开启 / 共享屏幕遮挡场景** — `.../proto/cc_edge_cases.rs`
  - 追溯 SPEC 5.8.3-3、5.8.3-7 · DESIGN 2.5.4-仲裁规则
  - DoD：两种场景均能判定字幕不可用并给出回退建议；不是崩溃或静默无输出
- [ ] **11.4 输出 Zoom 实测报告** — `docs/m0/zoom.md`
  - 追溯 SPEC 5.8.1-7 · DESIGN 2.5.6
  - DoD：含 CC 延迟（目标 1–1.5s）、说话人标识缺失情况、版本矩阵；结论写入 CapabilityMatrix

### 12. 流式 ASR 与 VAD 基线 🔴 CP · 🟢 PAR

- [ ] **12.1 sherpa-onnx 双语流式接入** — `crates/infrastructure/src/asr/sherpa_stream.rs`
  - 追溯 SPEC 4.1-1、4.5-4 · DESIGN 1.2.3、2.4.1
  - DoD：中英混合流式识别可用，输出 `Partial`/`Final` 两类事件；语言标签随事件输出；CPU 限制 2 线程；模型加载内存 ≤550MB
- [x] **12.2 稳定前缀策略** — `crates/application/src/understand/stable_prefix.rs`
  - 追溯 SPEC 4.4-4（控制误报率）、5.2.1-8 · DESIGN 1.2.3
  - DoD：连续两次一致的文本才提交给下游；抖动场景下提交文本的回退（撤销）率 ≤5%；单测覆盖"逐字追加/整句替换/尾部修正"三种流式模式
- [ ] **12.3 Silero VAD 静音与重叠检测** — `crates/infrastructure/src/audio/vad.rs`
  - 追溯 SPEC 5.1.1-7、5.1.3-4 · DESIGN 2.4.1
  - DoD：5 秒静音判定准确率 ≥95%；切句边界误差 ≤300ms；双人重叠场景能标记 overlap 起点（召回 ≥80%）
- [ ] **12.4 输出 ASR 基线报告** — `docs/baseline/asr.md`
  - 追溯 SPEC 4.1-1/2/6/7 · DESIGN 2.8.2-M0、2.1.3(7)
  - DoD：给出 RTF、partial 首字时延 p95、final 时延 p95、中英混合 CER、内存与 CPU 均值/峰值；明确"音频通道是否必须依赖云端转写"的结论

### 13. LLM 网关与门控离线回放框架 🟢 PAR

- [x] **13.1 OpenAI 兼容网关与结构化输出** — `crates/infrastructure/src/llm/gateway.rs`
  - 追溯 SPEC 6.3/6.4 结构化约束 · DESIGN 2.2.2-E、2.4.1
  - DoD：`InsightExtract`/`Summary`/`CardConclusion`/`MinutesDraft` 四类 JSON Schema 由 `schemars` 生成并与 struct 同源；`complete_structured`/`stream_structured` 均可用；首 token p95 ≤1.2s（可配模型，记录实测值）
  - 进度（2026-11-21）：已完成，提交 `29102c8`、`73a138a`。四类 Schema 与 struct 同源，一次性/流式均可用；真实端点 30 样本/任务下首 token p95 均 ≤1.2s，`minutes_draft` 稳定性单列跟踪。证据：`docs/progress/b1-status.md` 第 6/7 节
- [x] **13.2 超时、限流与 schema 违规降级** — `crates/infrastructure/src/llm/degrade.rs`
  - 追溯 SPEC 4.2-8 · DESIGN 1.2.4、2.1.3(9)
  - DoD：注入超时/429/schema 违规三类故障，系统分别走"重试→降级→规则兜底"，均不 panic、不阻塞采集；`deadline` 到期立即返回
  - 进度（2026-11-21）：已完成，提交 `29102c8`、`73a138a`。注入式三类故障 14 例全绿，六类失败分类与"重试→降级→规则兜底"均已判定；真实端点 429 观测归 AC-LLM-23（未实测）。证据：`docs/progress/b1-status.md` 第 6 节
- [ ] **13.3 门控离线回放框架** — `crates/application/tests/replay/`
  - 追溯 SPEC 4.4-4 · DESIGN 2.8.2-M0、2.6.1
  - DoD：输入片段集，可离线跑"缺口识别→L1 门控→L2 相关性→出卡"并输出候选/拒绝原因/时延；固定随机种子下结果可复现；支持按平台、版本、场景（字幕/音频）分组回放
- [ ] **13.4 输出 LLM 与门控基线** — `docs/baseline/llm-gate.md`
  - 追溯 SPEC 4.1-3 · DESIGN 2.8.2-M0
  - DoD：给出首 token、整卡生成、门控各级耗时，验证 6s 预算可行性；若不可行，明确"摘要兜底出卡"的必要性

### 14. M0 go/no-go 评审 ⛔ GATE

- [ ] **14.1 三平台字幕可行性结论** — `docs/m0/gate-review.md`
  - 追溯 SPEC 5.8.1-1、5.8.1-7 · DESIGN 2.8.2-M0
  - DoD：三平台逐项给出「可用/降级可用/不可用」，并附实测数据；**任一平台字幕不可行时**，以"音频通道可用 + UI 明确标注字幕不可用"作为对接通过口径，写入评审记录
- [ ] **14.2 双系统音频可行性结论** — `docs/m0/gate-review.md`
  - 追溯 SPEC 4.2-7、5.8.3-5 · DESIGN 2.8.2-M0
  - DoD：明确各回退链可用性、混音风险、权限拒绝率；对 Win10 <2004 的设备级回退给出告知文案
- [ ] **14.3 CapabilityMatrix 初版冻结** — `crates/platform-win/resources/capability_matrix.toml`、`crates/platform-mac/resources/capability_matrix.toml`
  - 追溯 SPEC 5.8.1-7、6.10-6 · DESIGN 2.5.2、2.5.6
  - DoD：三平台各字段（字幕可读性、延迟、说话人、需开字幕、推荐通道、已知限制）填写实测值；UI 可直接读取并展示；未实测项标注"待校准"而非填默认值

---

## 批次 B2 · 采集接入工程化（W6–W11）

**目标**：把 M0 原型工程化为可插拔的 `PlatformAdapter` 与稳定会话主链，达成 M1 前半出口。
**前置**：B1（G1 通过）。
**⛔ 出口门禁 G2（M1 前半）**：三平台均可建立会话并稳定运行 30 分钟；通道状态可见且 5s 内更新；字幕失效 5s 内回退音频；双通道不重复计入。
**并行**：15 与 16 并行；17 先于 18/19/20；21 可与 18–20 并行；25 依赖 15/16；26 依赖 23/24。

> **离线闭环增补（2026-11-21）**：W-00 / W-C / W-D / W-E / W-F 的子任务与验收定义见 `docs/sdd/tasks-b2-offline-closure.md`，实测三态与偏差见 `docs/progress/b2-offline-closure.md`（399 passed / 0 failed / 8 ignored，43 项 AC，结论"部分完成"）。本清单 23.x 的离线部分据此回写；真机项（15~22、24、26/27）未开始。

### 15. Windows 音频通道工程化 🔴 CP 🟢 PAR

- [ ] **15.1 实现 `AudioSource`（Win）** — `crates/platform-win/src/audio/source.rs`
  - 追溯 SPEC 5.8.1-2a、6.1-7 · DESIGN 2.2.2-A、2.5.4
  - DoD：通过端口契约测试（与 mock 同一套断言）；输出 200–400ms PCM 块；设备枚举与默认设备切换可用；`health()` 反映真实状态
- [ ] **15.2 实现回退链与能力探测** — `crates/platform-win/src/audio/fallback_chain.rs`
  - 追溯 SPEC 5.8.3-5、5.8.1-4 · DESIGN 2.5.4、1.2.1-1
  - DoD：进程级 → 设备级 → 麦克风三级自动降级，每级切换 ≤2s；降级原因写入 `Session.channel` 记录；UI 可读到"当前级 + 降级原因"
- [ ] **15.3 实现健康检查与"疑似无声"判定** — `crates/platform-win/src/audio/health.rs`
  - 追溯 SPEC 5.8.3-5 · DESIGN 2.5.4
  - DoD：连续 3 块静音且 VAD 无活动 → 5s 内产出"疑似无声"事件并提示（可能是独占模式/路由问题）；不误判正常会议静默（用真实静默样本回归）
- [ ] **15.4 设备切换/拔出处理** — `crates/platform-win/src/audio/device_watch.rs`
  - 追溯 SPEC 5.1.3-2 · DESIGN 2.5.4
  - DoD：会话中拔插设备，1s 内切默认设备并恢复推流；缺口时间段标记"音频缺失"并写入会话通道记录；UI 提示"设备已切换/音频中断 N 秒"

### 16. macOS 音频通道工程化 🔴 CP 🟢 PAR

- [ ] **16.1 实现 `AudioSource`（mac）** — `crates/platform-mac/src/audio/source.rs`
  - 追溯 SPEC 5.8.1-2a · DESIGN 2.2.2-A、2.5.4
  - DoD：通过端口契约测试；`SCContentFilter` 应用过滤生效；13.0/13.4/14.x 三版本各冒烟 10 分钟无中断
- [ ] **16.2 权限状态机与引导** — `crates/platform-mac/src/audio/permission.rs`
  - 追溯 SPEC 4.3-1、5.8.1-4 · DESIGN 2.5.4
  - DoD：四态（未询问/已授权/已拒绝/受限）可区分；拒绝时返回 `E-AUDIO-001` 并给深链；权限中途被回收能在 5s 内感知并停止推流
- [ ] **16.3 看门狗与重建工程化** — `crates/platform-mac/src/audio/watchdog.rs`
  - 追溯 SPEC 4.2-7、4.4-5 · DESIGN 1.2.1-2、2.5.4
  - DoD：每 30s 校验 stream 状态；异常重建 ≤2s 恢复；重建次数/原因进诊断；连续重建 >3 次/小时触发告警并建议切麦克风
- [ ] **16.4 设备切换与麦克风回退** — `crates/platform-mac/src/audio/devices.rs`
  - 追溯 SPEC 5.1.3-2、5.8.3-5 · DESIGN 2.5.4
  - DoD：AVCaptureDevice 麦克风回退可用并标记"仅麦克风"；切换事件与缺口写入会话记录；UI 提示文案正确

### 17. 字幕通道通用框架（平台无关）🔴 CP

- [ ] **17.1 `SelectorProfile` 定义与热加载** — `crates/infrastructure/src/subtitle/selector_profile.rs`
  - 追溯 SPEC 5.8.1-2b、4.5-2 · DESIGN 2.5.3-L1、R1
  - DoD：支持窗口标题正则、控件角色、文本属性、容器层级、`versionRange`；配置可从本地文件加载并在会话中热更新（不重启）；非法配置被拒绝并保留上一版；含 3 平台示例配置
- [ ] **17.2 L2 事件读取抽象** — `crates/infrastructure/src/subtitle/event_reader.rs`
  - 追溯 SPEC 5.8.1-2b · DESIGN 2.5.3-L2
  - DoD：Windows 侧封装 `UIA_TextPattern` + `StructureChanged`，macOS 侧封装 `AXObserver` + `AXValueChanged`；去抖 250–400ms 可配；同一容器连续变更合并为一次
- [ ] **17.3 L3 轮询兜底** — `crates/infrastructure/src/subtitle/poller.rs`
  - 追溯 SPEC 5.8.1-2b · DESIGN 2.5.3-L3
  - DoD：轮询 ≤4 次/秒且仅针对字幕容器；全量读取 + 后缀 diff 正确；事件可用时自动停用轮询（避免双倍 CPU）
- [ ] **17.4 文本归一化管线** — `crates/domain/src/subtitle/normalizer.rs`
  - 追溯 SPEC 5.1.1-4、4.5-4、6.2-3 · DESIGN 2.5.3（文本归一化 1–5）
  - DoD：噪声行（水印/时间戳/参会人列表）过滤有效率 ≥80%；被平台断行的同句可合并；"增量覆盖型"与"追加型"两种模式各有单测；语言检测输出 `LanguageTag`；无可靠起始时间时 `estimated=true`
- [ ] **17.5 `SubtitleSource` 基类与订阅生命周期** — `crates/application/src/capture/subtitle_base.rs`
  - 追溯 SPEC 5.8.1-2b · DESIGN 2.2.2-A
  - DoD：订阅/释放语义正确，`release` 后 0 回调（压力测试 10 万次回调注入）；`Subscription` 令牌跨线程安全；`health()` 与通道状态机联动

### 18. 腾讯会议平台适配器 🧩 SPI 🔴 CP

- [ ] **18.1 descriptor（进程/版本）** — `crates/platform-win/src/adapters/tencent/descriptor.rs`、`crates/platform-mac/src/adapters/tencent/descriptor.rs`
  - 追溯 SPEC 5.8.1-1、6.10-1/2 · DESIGN 2.5.2
  - DoD：识别 `wemeetapp.exe` / `TencentMeeting.app`；版本从文件版本资源 / `CFBundleShortVersionString` 读取；输出 `PlatformDescriptor{platform, version, subtitleAvailable, audioAvailable, capabilityLevel}`
- [ ] **18.2 字幕适配器（含 OCR 回退）** — `.../adapters/tencent/subtitle.rs`
  - 追溯 SPEC 5.8.1-2b、5.8.3-4 · DESIGN 2.5.3-L1~L4
  - DoD：选择器（UIA/AX）→ 轮询 → OCR 四级降级链完整；`versionRange` 外自动置 `unavailable`；`supports_speaker_label` 按版本如实上报
- [ ] **18.3 音频目标解析（PID）** — `.../adapters/tencent/audio_target.rs`
  - 追溯 SPEC 5.8.1-2a · DESIGN 2.5.2
  - DoD：能从进程树正确定位会议客户端主进程 PID（排除更新器/子进程）；PID 失效时回退设备级并上报
- [ ] **18.4 平台特有边界处理** — `.../adapters/tencent/edge_cases.rs`
  - 追溯 SPEC 5.8.3-7 · DESIGN 2.5.6
  - DoD：共享屏幕导致字幕浮窗被遮挡时，5s 内标记字幕降级并保留音频；该限制在 UI 能力差异区可见

### 19. 飞书 / Lark 平台适配器 🧩 SPI

- [ ] **19.1 descriptor（国内版/国际版分列矩阵）** — `.../adapters/feishu/descriptor.rs`
  - 追溯 SPEC 5.8.1-1、4.5-2、6.10-2 · DESIGN 2.5.2
  - DoD：识别 `Feishu.exe`/`Lark.exe`/`Feishu.app`/`Lark.app`；国内版与国际版**各自维护**版本矩阵与适配记录；两者 UI 差异以配置区分而非代码分支
- [ ] **19.2 字幕适配器（OCR 主路径）** — `.../adapters/feishu/subtitle.rs`
  - 追溯 SPEC 5.8.1-2b · DESIGN 1.2.2、2.5.3
  - DoD：默认走 OCR 主路径；若无障碍树可用且 `text_binding_score ≥0.6` 则优先 UIA/AX；评分规则有单测；两路径切换不中断字幕流
- [ ] **19.3 音频目标解析** — `.../adapters/feishu/audio_target.rs`
  - 追溯 SPEC 5.8.1-2a · DESIGN 2.5.2
  - DoD：正确区分飞书主进程与 Electron 辅助进程；多开（国内版+国际版同时运行）时按会话绑定选择正确 PID
- [ ] **19.4 能力差异声明** — `.../adapters/feishu/capability.rs`
  - 追溯 SPEC 5.8.1-7 · DESIGN 2.5.6
  - DoD：UI 明确显示"字幕依赖 OCR、延迟与准确率低于原平台字幕"；不宣称与其他平台能力一致（文案经产品确认）

### 20. Zoom 平台适配器 🧩 SPI

- [ ] **20.1 descriptor（≥5.17）** — `.../adapters/zoom/descriptor.rs`
  - 追溯 SPEC 4.5-2、6.10-2 · DESIGN 2.5.2
  - DoD：识别 `Zoom.exe`/`zoom.us.app`；<5.17 返回"版本过低"且字幕 `unavailable`、仅尝试音频；支持范围硬编码为常量并集中维护
- [ ] **20.2 CC 字幕适配器** — `.../adapters/zoom/subtitle.rs`
  - 追溯 SPEC 5.8.1-2b · DESIGN 2.5.3
  - DoD：CC 窗口发现 + UIA/AX 读取 + 增量 diff 完整；CC 未开启时"非错误地"返回不可用；`estimated_lag_ms` 按实测填写
- [ ] **20.3 音频目标解析** — `.../adapters/zoom/audio_target.rs`
  - 追溯 SPEC 5.8.1-2a · DESIGN 2.5.2
  - DoD：正确获取 Zoom 主进程 PID；Zoom 分屏/多显示器场景下仍能定位
- [ ] **20.4 多平台并行互斥验证** — `.../adapters/zoom/multi_platform.rs`
  - 追溯 SPEC 5.8.1-6、5.1.1-11 · DESIGN 2.5.7-4
  - DoD：同时运行腾讯会议 + Zoom 时，开始会话强制选择其一；会话内检测到第二平台来源硬拦截写入并提示新建会话

### 21. OCR 兜底引擎 🟢 PAR

- [ ] **21.1 系统 OCR 封装（Win/mac）** — `crates/platform-win/src/ocr/windows_ocr.rs`、`crates/platform-mac/src/ocr/vision_ocr.rs`
  - 追溯 SPEC 5.8.3-4 · DESIGN 2.4.1、1.2.2-2
  - DoD：Windows.Media.Ocr 与 Apple Vision 均可对 ROI 小图出文本行 + 置信度；无额外运行时依赖（不引入 Tesseract 为默认）；单次识别 ≤150ms（小 ROI）
- [ ] **21.2 ROI 几何计算与自适应** — `crates/infrastructure/src/ocr/roi.rs`
  - 追溯 SPEC 5.8.3-4 · DESIGN 2.5.3-L4
  - DoD：由窗口几何计算字幕区 ROI（含 DPI 缩放、多显示器、窗口移动）；字幕区域变化时自动重算；ROI 越界自动收敛到窗口内
- [ ] **21.3 帧差与文本行重组** — `crates/infrastructure/src/ocr/frame_diff.rs`
  - 追溯 SPEC 5.8.3-4 · DESIGN 2.5.3-L4、1.2.2-2
  - DoD：≤2fps 采样；帧差阈值 0.02；与上一帧相似度 >0.95 的行被忽略；文本行按位置聚类重组成句；CPU 增量 ≤5%（实测记录）
- [ ] **21.4 OCR 质量回归集与降级** — `crates/infrastructure/tests/ocr_corpus/`
  - 追溯 SPEC 4.4-1、5.8.3-4 · DESIGN 2.1.3(2)
  - DoD：≥100 张真实字幕截图回归集，字符准确率 ≥85%；OCR 失败率 >50% 时自动切音频并触发通道状态迁移

### 22. 平台探测与版本矩阵

- [ ] **22.1 进程与版本探测** — `crates/infrastructure/src/platform/discovery.rs`
  - 追溯 SPEC 5.8.1-4、6.10-2 · DESIGN 2.5.2
  - DoD：sysinfo + 文件版本资源 / CFBundle 读取；探测耗时 ≤300ms；不读取平台私有数据目录、不读凭据（代码审计确认）
- [ ] **22.2 CapabilityMatrix 与适配记录** — `crates/infrastructure/src/platform/matrix.rs`
  - 追溯 SPEC 6.10-6/7、4.5-2 · DESIGN 2.5.2、2.5.6
  - DoD：矩阵包含平台 + 版本范围 + 能力差异 + 最近验证时间；低于范围时字幕置 `unavailable`；矩阵可随选择器配置一起热更新
- [ ] **22.3 多平台互斥判定** — `crates/application/src/capture/multi_platform_guard.rs`
  - 追溯 SPEC 5.8.1-6、5.1.1-11 · DESIGN 2.5.7-4、R14
  - DoD：检测到 ≥2 平台在会中 → 暂停自动绑定并要求选择；会话内出现第二平台来源 → 拒绝写入 + 提示新建会话 + 审计
- [ ] **22.4 平台策略遵从** — `crates/application/src/capture/platform_policy.rs`
  - 追溯 SPEC 5.8.1-8 · DESIGN 2.5.7-1
  - DoD：检测到平台禁止录制/第三方采集提示时，仅保留音频或停止采集并明确告知；不得绕过（无 hook/注入代码路径，审计确认）

### 23. 通道仲裁与状态机 🔴 CP

- [x] **23.1 实现通道状态机** — `crates/application/src/capture/channel_state.rs`
  - 追溯 SPEC 5.8.1-4、4.2-7 · DESIGN 2.1.3(2)
  - DoD：四状态（探测中/可用/降级/不可用）+ 设计文档全部迁移边均有单测；非法迁移被拒绝；每平台每通道独立实例
  - 进度（2026-11-21）：离线已完成，提交 `d683b7a`。四状态表驱动用例 `crates/domain/tests/channel_state_table.rs` 覆盖全部迁移边，非法迁移拒绝有单测。证据：`docs/progress/b2-offline-closure.md` §4
- [ ] **23.2 字幕优先与回退/切回** — `crates/application/src/capture/channel_arbiter.rs`
  - 追溯 SPEC 4.2-7、5.8.1-5 · DESIGN 2.5.4-仲裁规则 1/2/3
  - DoD：字幕可用时音频仅用于校验与缺口回补（ASR 调用数下降可测）；字幕 5s 无有效文本 → 切音频；恢复后连续 10s 稳定 → 切回；切换点写入会话记录
  - 进度（2026-11-21）：**部分完成**。离线仲裁与切换日志已落地（`5731801`），恢复间隔由注入时钟判定（`single_channel_recovery_within_5s`）；真实平台字幕 5s 失效与 10s 切回、ASR 调用数下降未实测，故保持未勾选
- [x] **23.3 双通道均失效的保护** — `crates/application/src/capture/degraded_guard.rs`
  - 追溯 SPEC 4.2-7 · DESIGN 2.5.4-仲裁规则 5
  - DoD：两会话通道均不可用时，会话保留、已产生要点不丢、UI 明确"采集不可用"；恢复后可续采且片段不重不漏
  - 进度（2026-11-21）：离线已完成，提交 `4a4e430`。`gap_300s_keeps_session_and_insight_ids` 证明要点零写/零删/零迁移；恢复续采与回补窗口由 `resume_from_is_close_minus_capped_duration` 判定（偏差 D1/D4 见验收记录）
- [ ] **23.4 状态变更 5s 内推送** — `crates/application/src/capture/status_publisher.rs`
  - 追溯 SPEC 5.8.1-4 · DESIGN 2.1.3(2)、2.1.3(4)
  - DoD：状态变更到 UI 的时延 p99 ≤5s（打点验证）；事件去重（同一状态不重复推）；订阅者可查询当前快照
  - 进度（2026-11-21）：**部分完成**。去重、快照可见、会话内单序号流、p99 下标语义均已判定（`capture_status_push` 13 例）；但 p99 为注入时钟推导的**逻辑上限**，真实 UI 线程时延未实测，故保持未勾选

### 24. 双通道去重合并 🔴 CP

- [ ] **24.1 音频时钟统一与配对** — `crates/domain/src/merge/clock_align.rs`
  - 追溯 SPEC 5.1.1-5、6.2-9 · DESIGN 2.5.5-1
  - DoD：所有字幕/ASR 事件映射到音频时钟；配对窗口 ±2.5s（字幕 `estimated` 时 ±3.5s）；配对结果可解释（输出配对得分与依据）
- [ ] **24.2 三级等价判定** — `crates/domain/src/merge/equivalence.rs`
  - 追溯 SPEC 5.1.1-5 · DESIGN 2.5.5-2/3
  - DoD：文本归一化（去标点/空格、繁简统一、数字与英文小写）；完全相同→合并；SimHash 汉明距离 ≤3 且长度比 ∈[0.6,1.6]→合并；否则余弦 ≥0.86（仅 >8 字片段）→合并；每条规则有单测
- [ ] **24.3 合并策略与置信度上限** — `crates/domain/src/merge/merger.rs`
  - 追溯 SPEC 6.2-9、5.1.1-4 · DESIGN 2.5.5-4
  - DoD：以字幕文本为主、音频提供精确起止时间；`dualChannel=true`、`source=平台字幕`、`confidence=max()` 且 ≤0.95；来源标注不静默覆盖（保留两路原始引用）
- [ ] **24.4 只计一次与重叠标记** — `crates/application/src/capture/dedupe_guard.rs`
  - 追溯 SPEC 5.1.1-5、5.1.3-4 · DESIGN 2.5.5-5/6
  - DoD：合并后的 Utterance 只触发 1 次要点提炼与 1 次缺口识别（计数验证）；不可合并且音色不同的片段双方标 `overlap=true` 且不进高置信要点
- [ ] **24.5 合并质量回归集** — `crates/domain/tests/merge_corpus/`
  - 追溯 SPEC 5.1.1-5 · DESIGN 2.8.2-M1 门禁
  - DoD：≥200 对真实字幕/ASR 样本；合并准确率 ≥95%；误合并率 ≤3%；回归集可增量追加

### 25. 音频前处理与资源护栏 🟢 PAR

- [ ] **25.1 重采样与轻降噪** — `crates/infrastructure/src/audio/preprocess.rs`
  - 追溯 SPEC 4.1-7 · DESIGN 2.5.4（音频前处理链）
  - DoD：rubato 48k→16k mono；直流/低频去除 + 轻降噪；端到端音频链路引入延迟 ≤300ms；CPU 增量 ≤8%；无音质明显劣化（ASR CER 对比恶化 ≤1%）
- [ ] **25.2 分块与环形缓冲** — `crates/infrastructure/src/audio/chunker.rs`
  - 追溯 SPEC 4.2-5 · DESIGN 2.5.4、2.1.3(8)
  - DoD：200–400ms 分块；8s 环形缓冲用于字幕缺失回补；缓冲区内存 ≤120MB（60s 上限）
- [ ] **25.3 有界队列与背压** — `crates/application/src/capture/bounded_queue.rs`
  - 追溯 SPEC 4.1-6、4.2-8 · DESIGN 2.1.3(8)
  - DoD：容量 2000，满则丢弃最旧并计数（不阻塞采集线程）；背压信号可被提炼侧消费；丢弃条数进入诊断
- [ ] **25.4 峰值保护器** — `crates/infrastructure/src/resource/governor.rs`
  - 追溯 SPEC 4.1-7 · DESIGN 2.1.3(8)
  - DoD：连续 10s CPU >55% 时按顺序降级（降 OCR 帧率 → 提高 ASR 分块 → 暂停摘要）；降级动作写入诊断；恢复后逐步回升且不震荡（含滞回）

### 26. 会话编排与状态机 🔴 CP

- [ ] **26.1 实现六状态机与唯一写入点** — `crates/application/src/session/state_machine.rs`
  - 追溯 SPEC 6.1-4 · DESIGN 2.1.3(1)
  - DoD：`SessionState` 六值；`transition()` 为唯一写入点（其他模块无法直接改 state，类型可见性保证）；属性测试无非法迁移；非法迁移返回明确错误
- [ ] **26.2 启动守卫** — `crates/application/src/session/start_guard.rs`
  - 追溯 SPEC 5.1.1-1/2/3、5.8.1-6 · DESIGN 2.1.3(1) 守卫
  - DoD：`consentConfirmed && channelReady && platformBound && singlePlatformActive` 四守卫全满足才进入监听中；缺任一项返回对应原因码；用例矩阵 2^4=16 组合全覆盖
- [ ] **26.3 暂停/恢复/静默期** — `crates/application/src/session/modes.rs`
  - 追溯 SPEC 5.1.1-8/9、5.4.1-3 · DESIGN 2.1.3(1)
  - DoD：暂停期入流内容被丢弃（要点与自动检索调用数为 0）；静默期要点继续记录但自动检索/推送为 0；静默结束合并展示不逐条弹
- [ ] **26.4 30 分钟无输入保护** — `crates/application/src/session/idle_guard.rs`
  - 追溯 SPEC 5.1.1-10 · DESIGN 2.1.3(1)
  - DoD：连续 30 分钟无有效输入 → 提示"是否结束会话"；禁止静默终止（断言：无用户确认则状态不变）；时钟可注入以便测试
- [ ] **26.5 结束与删除编排** — `crates/application/src/session/close_flow.rs`
  - 追溯 SPEC 5.5.1-8、6.1-3 · DESIGN 2.1.3(1)、2.7.5
  - DoD：结束需用户确认；结束后触发纪要草稿生成；删除需二次确认且不可取消；删除调用 `delete_irrecoverable` 并记录审计

### 27. 采集告知与首次确认 🛡 合规 🔴 CP

- [ ] **27.1 告知页与文案** — `app/src/features/consent/`、`crates/application/src/session/consent.rs`
  - 追溯 SPEC 4.3-1、5.1.1-3、5.6.1-1 · DESIGN 2.7.1、2.7.7
  - DoD：首次使用必须展示；组织策略设"每次确认"时每次会话都展示；文案包含采集范围、用途、本地处理、在场人员提示；不可跳过按钮
- [ ] **27.2 ConsentRecord 持久化与校验** — `crates/infrastructure/src/store/consent_repo.rs`
  - 追溯 SPEC 4.3-1、6.1 · DESIGN 2.1.3(1)
  - DoD：记录确认时间、方式（首次/每次）、策略版本；状态丢失时守卫拒绝启动；不可被配置文件直接置为已确认
- [ ] **27.3 未确认拒绝与错误处理** — `crates/application/src/session/consent_guard.rs`
  - 追溯 SPEC 5.6.3-1 · DESIGN 2.7.1
  - DoD：未确认时 100% 无法进入监听中，返回 `E-CONSENT-007`；错误提示含"需确认采集告知后才能开始"；该拒绝写审计

---

## 批次 B3 · 理解层（W9–W11，可与 B2 后半并行）

**目标**：把 `Utterance` 转换为可回溯的要点与滚动摘要，并建立缺口识别能力。
**前置**：B0（端口/存储/LLM 网关）、B2.24（合并后的 Utterance）。**不依赖**平台适配器具体实现，可用回放集离线开发。
**⛔ 出口门禁 G3**：三平台任一接入下，片段→要点 p95 ≤1.5s（字幕）/ ≤3s（音频）；要点约束 100% 合规；异常退出可恢复 60s 上下文。
**并行**：28 先于 30；29 与 30 可并行；31/32 可与 30 并行（依赖模型而非编排）。

### 28. 片段归一化与来源标注

- [ ] **28.1 `Utterance` 构建与一致性校验** — `crates/domain/src/model/utterance.rs`、`crates/application/src/understand/normalize.rs`
  - 追溯 SPEC 5.1.1-4/6、6.2-2/5/7/8 · DESIGN 2.3.2、2.2.2-A
  - DoD：`source` 与 `platform` 一致性校验（平台通道必填且等于会话绑定平台）；无说话人标识时统一置"未知说话人"（禁止推断姓名）；置信度 <0.5 的片段 `is_eligible_for_insight()==false`
- [ ] **28.2 静音过滤与状态提示** — `crates/application/src/understand/silence_filter.rs`
  - 追溯 SPEC 5.1.1-7 · DESIGN 2.5.4
  - DoD：连续静音 >5s 不产生空片段、不计入要点统计；UI 显示"静音中"；真实会议中的正常停顿不误判（回归样本）
- [ ] **28.3 正文超长切分** — `crates/domain/src/model/utterance_text.rs`
  - 追溯 SPEC 6.2-4 · DESIGN 2.3.2
  - DoD：>2000 字按标点/时间边界切分，切分后每条 ≤2000 字且时间区间连续不重叠；切分失败案例 0（模糊测试）
- [ ] **28.4 低置信片段分流** — `crates/application/src/understand/low_confidence.rs`
  - 追溯 SPEC 6.2-6、5.3.1-15 · DESIGN 2.1.3(5)
  - DoD：置信度 <0.5 只落盘不提炼、不触发自动外发；计入"因识别质量跳过"统计；UI 片段旁显示"识别质量低，未提炼要点"

### 29. 滚动摘要器

- [ ] **29.1 触发条件与增量压缩** — `crates/application/src/understand/rolling_summary.rs`
  - 追溯 SPEC 5.2.1-9、4.1-9 · DESIGN 2.2.2-C、2.1.3(7)
  - DoD：每 30s **或** 累计 20 条新片段触发；只压缩"新增片段 + 上一版摘要"；1 小时会话刷新 ≥120 次且不阻塞主链
- [ ] **29.2 版本对比与保留** — `crates/domain/src/model/rolling_summary.rs`
  - 追溯 SPEC 5.2.1-9 · DESIGN 2.2.2-C
  - DoD：保留上一版用于对比，`version` 单调递增；摘要 `topics`/`entities` 可被相关性评分消费（字段非空约束）
- [ ] **29.3 超时丢弃与诊断** — `crates/application/src/understand/summary_guard.rs`
  - 追溯 SPEC 5.2.3-4 · DESIGN 2.6.6
  - DoD：生成超过 15s 未完成 → 沿用上一版、本次丢弃并记录一次失败；用户无阻塞提示；诊断面板可见"摘要状态异常"与失败次数

### 30. 要点提炼器 🔴 CP

- [ ] **30.1 规则预筛 + 结构化抽取** — `crates/application/src/understand/insight_extractor.rs`
  - 追溯 SPEC 5.2.1-1~3 · DESIGN 2.2.2-C、2.6.5
  - DoD：规则预筛（陈述性/行动性/疑问信号）→ LLM 结构化抽取 → Schema 校验三段式；`InsightDraft.sources` 非空（编译期 newtype + 运行期断言）；无来源要点 0
- [ ] **30.2 四类分类与兜底** — `crates/domain/src/model/insight_kind.rs`
  - 追溯 SPEC 5.2.1-2、6.3-3 · DESIGN 2.3.2
  - DoD：关键信息/决策/待办/疑问四类；无法判定归"关键信息"并降置信（幅度可配，默认 −0.15）；未分类要点 0（枚举封闭）
- [ ] **30.3 60 字强制拆分** — `crates/application/src/understand/insight_splitter.rs`
  - 追溯 SPEC 5.2.1-3、6.3-4 · DESIGN 2.2.2-C
  - DoD：超长必须在提炼器内拆分为多条并各自保留原文引用；UI 层不做截断（断言：UI 无 truncate 逻辑）；输出 0 条超 60 字
- [ ] **30.4 决策/待办/疑问字段补全** — `crates/application/src/understand/field_filler.rs`
  - 追溯 SPEC 5.2.1-4/5/6、6.3-10/11 · DESIGN 2.3.2
  - DoD：决策缺决策人/范围 → "待确认"；待办缺负责人/期限 → "待确认"；疑问初始 `answered=false`，后续出现回应片段时置 `true` 并关联回应片段（有单测）
- [ ] **30.5 规则提炼兜底** — `crates/application/src/understand/rule_fallback.rs`
  - 追溯 SPEC 4.2-8 · DESIGN 2.1.3(9)
  - DoD：LLM 不可用/断网时仍能产出四类要点（准确率可降但结构完整）；兜底开关可观测（指标 `insight.rule_fallback.rate`）；不抛错、不阻塞采集
- [ ] **30.6 要点质量回归集** — `crates/application/tests/insight_corpus/`
  - 追溯 SPEC 4.4-4 · DESIGN 2.6.6
  - DoD：≥300 条标注样本；四类分类准确率 ≥85%；误报率 ≤15%；回归集按平台/语言分组

### 31. 要点去重与人工锁定

- [ ] **31.1 语义合并与提及计数** — `crates/application/src/understand/deduplicator.rs`
  - 追溯 SPEC 5.2.1-7、6.3-9 · DESIGN 2.2.2-C
  - DoD：余弦 ≥0.88 合并，`mentionCount` 递增；10 分钟窗口同主题合并；合并准确率 ≥90%（回归集）
- [ ] **31.2 人工编辑锁定** — `crates/domain/src/model/insight_lock.rs`
  - 追溯 SPEC 5.2.1-10、6.3-7 · DESIGN 2.3.2
  - DoD：`status=已编辑` 后 `mergeRepeated` 只加计数不改正文（断言）；后续冲突内容作为"补充信息"并列展示，不覆盖人工版本
- [ ] **31.3 重要标记/删除/筛选** — `crates/application/src/understand/insight_ops.rs`
  - 追溯 SPEC 5.2.1-10、5.2.2 · DESIGN 2.1.3(5)
  - DoD：标记重要、删除（`status=已删除`）、按类型/关键词筛选均可用；删除后不再出现在要点流与纪要（除审计摘要外）

### 32. 信息缺口识别器（L0）🔴 CP

- [ ] **32.1 六类规则实现** — `crates/domain/src/gap/rules/*.rs`
  - 追溯 SPEC 5.3.1-2 · DESIGN 2.6.2
  - DoD：首次专名（0.70）/缩写（0.65）/机构职务（0.60）/法规标准（0.80）/数据指标（0.75）/未答提问（0.85）六类各有实现与基础分；缩写白名单（OK/API 等）可配；每条规则有正负样本测试
- [ ] **32.2 NER、术语表差集与首现标记** — `crates/domain/src/gap/ner.rs`、`.../first_seen.rs`
  - 追溯 SPEC 5.3.1-2、6.6-4 · DESIGN 2.6.2
  - DoD：NER 覆盖 PER/ORG/PROD；"首次出现"依据术语表 + 本次会话已解释集合判定；敏感术语不进入查询种子（与脱敏器联动断言）
- [ ] **32.3 问句检测与 10s 无回应判定** — `crates/domain/src/gap/question.rs`
  - 追溯 SPEC 5.3.1-2b · DESIGN 2.6.2
  - DoD：问句检测（中英）召回 ≥85%；10s 内出现回应片段则不产候选；仅"需外部事实支撑"的提问才产候选（对在场者的提问被排除）
- [ ] **32.4 2s 内产出与并行约束** — `crates/application/src/proactive/gap_pipeline.rs`
  - 追溯 SPEC 5.3.1-2a、4.1-3 · DESIGN 2.6.2（硬性时延）
  - DoD：缺口识别消费 ASR partial（不等 final）；片段稳定后 2s 内产出候选（p95 实测）；与采集线程无锁竞争（压测无线程饥饿）
- [ ] **32.5 低置信跳过与统计** — `crates/application/src/proactive/gap_guard.rs`
  - 追溯 SPEC 5.3.1-15 · DESIGN 2.6.1-L0
  - DoD：片段置信度 <0.5 或候选 `confidence <0.5` 时不产候选，计入"识别质量跳过"；该计数在诊断面板可查
- [ ] **32.6 缺口识别回归集** — `crates/domain/tests/gap_corpus/`
  - 追溯 SPEC 4.4-4 · DESIGN 2.6.6
  - DoD：六类各 ≥20 条样本；召回 ≥80%；误报 ≤20%；产出时延 p95 ≤2s

### 33. 要点持久化、恢复与背压

- [ ] **33.1 骨架 1s 落盘与异步补全** — `crates/application/src/insight/persist_skeleton.rs`
  - 追溯 SPEC 4.2-2 · DESIGN 2.1.2（InsightService 决策）
  - DoD：先写最小可回溯骨架（id/类型/时间戳/引用/原始文本引用）并 1s 内落盘，字段异步补全；1s 落盘率 ≥99.9%（打点）；补全失败不丢骨架
- [ ] **33.2 异常退出与 60s 上下文恢复** — `crates/application/src/insight/recovery.rs`
  - 追溯 SPEC 4.2-2 · DESIGN 2.1.3(9)
  - DoD：`kill -9` 后重启可恢复最近 60s 上下文（误差 ≤60s）；恢复后不重复生成同要点（去重键复用）；恢复过程写审计
- [ ] **33.3 提炼积压背压** — `crates/application/src/insight/backpressure.rs`
  - 追溯 SPEC 5.2.3-3、4.1-6 · DESIGN 2.1.3(8)
  - DoD：积压 >200 条时暂停拉取、改为"只落盘不提炼"，UI 提示"提炼落后 N 条"；采集线程不受影响（延迟不升）；积压回落后自动恢复
- [ ] **33.4 会话结束补提炼** — `crates/application/src/insight/catch_up.rs`
  - 追溯 SPEC 5.2.3-5 · DESIGN 2.1.3(7)(8)
  - DoD：结束时会 30s 内消化队列；超时未完成标记"未提炼"并计入待复核清单；纪要页提示"有 N 条片段未完成提炼，可手动补全"

---

## 批次 B4 · 检索与全自动主动提示（W12–W16）

**目标**：打通"识别缺口 → 脱敏 → 检索 → 抓取 → 合成 → 校验 → 推送"全自动主路径，并满足 6s/10s 时延与误报门禁。
**前置**：B3（GapCandidate）、B0（端口/审计/度量）。
**⛔ 出口门禁 G4（M2 核心）**：自动提示 p95 ≤6s、p99 ≤10s；误触发率 ≤30%（内测 ≥20 场）；零源"仅要点"模式可用；脱敏失败 100% 阻断。
**并行**：34/35/36/37 可并行（仅依赖端口）；38 依赖 35/36；40 依赖 37/38/39；41/42/43 可与 40 并行开发、在 40 之后集成。

### 34. 检索源配置模型与适配器注册表 🧩 SPI

- [ ] **34.1 `ProviderProfile` 校验与持久化** — `crates/domain/src/provider/profile.rs`
  - 追溯 SPEC 6.9-1~10、4.5-6 · DESIGN 2.2.2-H、2.3.2
  - DoD：`schemaVersion`（缺省按 1.0）、`id` 唯一、`priority` 同一配置内不重复、`weight` ∈[0,100]、`timeoutMs` ≤8000、`rateLimit` 必填、`blockedDomains` 支持通配；`authRef` 仅存引用不含明文；非法配置返回字段级错误
- [ ] **34.2 适配器注册表与热加载** — `crates/infrastructure/src/search/registry.rs`
  - 追溯 SPEC 5.3.1-7c、5.7.1-2 · DESIGN 2.2.2-D（契约示例）
  - DoD：`upsert(profile)` 后下一次检索即生效（无需重启，实测验证）；`spiVersion` 不匹配返回 `AdapterVersionIncompatible` 并拒绝加载；注册表内无任何内置默认源（断言启用列表初始为空）
- [ ] **34.3 连接测试与健康状态** — `crates/infrastructure/src/search/health.rs`
  - 追溯 SPEC 5.7.3-5、6.9-10 · DESIGN 2.2.2-D
  - DoD：测试连接返回 `ProviderHealth` 或失败分类（Timeout/RateLimited/AuthFailed/MalformedResponse/Unreachable）；失败源标记"不可用"且不参与调度；健康信息可在配置页读取
- [ ] **34.4 零源"仅要点"模式** — `crates/application/src/proactive/no_provider_mode.rs`
  - 追溯 SPEC 4.2-4、5.3.3-2、4.5-7 · DESIGN 2.6.1
  - DoD：启用源数为 0 时，全部自动与按需检索被拒绝（`Deny{NoProviderEnabled}`），要点与纪要仍正常；UI 常驻"未配置检索源，当前为仅要点模式"；切换到有源后无需重启恢复
- [ ] **34.5 至少 5 源并行配置验证** — `crates/infrastructure/tests/multi_provider.rs`
  - 追溯 SPEC 4.5-6 · DESIGN 2.8.2-M2
  - DoD：同时配置 5 个源（不同类型），优先级 1–5 无重复；调度顺序可按配置调整；任一源停用后其余顺序不变

### 35. 多源调度与结果融合 🔴 CP

- [ ] **35.1 优先级降级链与 3s 切换** — `crates/infrastructure/src/search/broker/degrade.rs`
  - 追溯 SPEC 4.2-3、5.3.3-1 · DESIGN 2.2.2-D、2.1.3(9)
  - DoD：当前源超时/限流/不可用 → 3s 内切至下一优先级（打点验证）；熔断窗口内不再调度该源；全部不可用进入降级队列
- [ ] **35.2 8s 超时硬截断与并发上限** — `crates/infrastructure/src/search/broker/limits.rs`
  - 追溯 SPEC 6.9-8、4.1-4 · DESIGN 2.1.3(8)
  - DoD：单次检索超时上限 8s（Profile 可配但不得超过）；单会话并发 ≤3（注入 10 并发，峰值计数 ≤3）；排队不导致提示延迟 >15s
- [ ] **35.3 令牌桶与权重融合** — `crates/infrastructure/src/search/broker/fusion.rs`
  - 追溯 SPEC 6.9-7、5.3.1-7a · DESIGN 2.6.3（sourceTrust）
  - DoD：按 `maxRequests/windowSeconds` 限流，超限降级；多源结果按 `weight/100 × 域名可信度` 排序；融合结果去重（同 URL 只留最高分）
- [ ] **35.4 屏蔽域名过滤** — `crates/infrastructure/src/search/broker/blocklist.rs`
  - 追溯 SPEC 6.9-9 · DESIGN 2.6.4-8
  - DoD：命中屏蔽域的结果被丢弃并计入过滤统计；通配符（`*.example.com`）正确匹配；大小写与 IDN 处理有测试
- [ ] **35.5 SSRF 与出站防护** — `crates/infrastructure/src/net/egress_guard.rs`
  - 追溯 SPEC 5.7.1-2 · DESIGN 2.8.1-R12
  - DoD：默认拒绝环回、链路本地、私网、云元数据地址（用户显式"允许内网"才放行）；DNS 重绑定防护（解析后校验 + 连接 IP 固定）；重定向跨域不自动跟随；响应体大小上限（默认 2MB）

### 36. 网页抓取器

- [ ] **36.1 robots.txt 判定** — `crates/infrastructure/src/fetch/robots.rs`
  - 追溯 SPEC 5.3.1-12 · DESIGN 1.2.5、2.4.1（抓取合规）
  - DoD：按 RFC 9309 解析并缓存（TTL 可配）；Disallow 命中返回 `Skipped{RobotsDisallowed}`；解析失败按保守策略（视为禁止）并记录
- [ ] **36.2 登录墙/付费墙识别** — `crates/infrastructure/src/fetch/wall_detect.rs`
  - 追溯 SPEC 5.3.1-12 · DESIGN 2.2.2-D
  - DoD：识别登录/付费墙特征返回 `Skipped{LoginWall|Paywall}`，来源清单注明"不可访问"；不尝试绕过（无 Cookie 注入、无 UA 伪装代码路径）
- [ ] **36.3 正文抽取与元信息** — `crates/infrastructure/src/fetch/extract.rs`
  - 追溯 SPEC 5.3.1-12、6.5-2/5 · DESIGN 2.4.1（html5ever + scraper）
  - DoD：输出标题（≤200 字）/正文/发布时间/语言；与 trafilatura 基线对比 F1 ≥0.90；发布时间无法解析时置空并标注"发布时间未知"
- [ ] **36.4 限速、UA 声明与超时** — `crates/infrastructure/src/fetch/policy.rs`
  - 追溯 SPEC 5.3.1-12 · DESIGN 2.4.1、2.1.1（HTTP 超时 5s）
  - DoD：单域名限速（默认 1 req/s）与全局并发 3；UA 明确标识客户端与版本；单页超时 5s；超时/4xx/5xx 返回 `Failed{cause}` 不抛出
- [ ] **36.5 并行抓取前 3 条高相关 URL** — `crates/infrastructure/src/fetch/parallel.rs`
  - 追溯 SPEC 4.1-3 · DESIGN 2.1.3(6)
  - DoD：并行抓取 3 条，抓取阶段 p95 ≤1.2s；任一失败不影响其余；4s 未完成则触发摘要兜底（与 38.6 联动）

### 37. 脱敏器与统一出口网关 🔴 CP 🛡 合规

- [ ] **37.1 词典 + NER + 规则脱敏** — `crates/infrastructure/src/privacy/sanitizer.rs`
  - 追溯 SPEC 4.3-4、5.3.1-6、5.6.1-3 · DESIGN 2.7.1
  - DoD：覆盖敏感词表、参会人姓名、术语表敏感标记；NER 覆盖人名/机构；规则覆盖手机号/邮箱/身份证/IP；命中去重后统一打码
- [ ] **37.2 一致性掩码与角色化** — `crates/infrastructure/src/privacy/masking.rs`
  - 追溯 SPEC 5.6.1-3 · DESIGN 2.7.1
  - DoD：同一实体在同一会话内使用同一掩码；人名采用角色化（"张总"→"某负责人"）以保持检索可用性；掩码映射与会话绑定，既不写日志也不随导出泄露
- [ ] **37.3 `SanitizedQuery` 私有构造与长度约束** — `crates/domain/src/privacy/sanitized_query.rs`
  - 追溯 SPEC 4.3-4、5.3.1-5 · DESIGN 2.2.2-D
  - DoD：仅 `Sanitizer` 可构造（模块私有 + 构造器不可见）；自动触发 ≤80 字、按需 ≤200 字，超长截断并提示"已按最小上下文外发"；其他 crate 无法伪造（编译失败用例）
- [ ] **37.4 五类出口统一判定（fail-closed）** — `crates/application/src/privacy/outbound_gateway.rs`
  - 追溯 SPEC 4.3-4、5.6.3-2、4.4-2 · DESIGN 2.7.1
  - DoD：查询/卡片/纪要/导出/日志五类出口全部经同一入口（代码审计：无绕过路径）；脱敏器异常时阻断外发并返回 `E-PRIV-004`；阻断事件写审计
- [ ] **37.5 脱敏回归集与抓包验证** — `crates/infrastructure/tests/privacy_corpus/`
  - 追溯 SPEC 4.3-4、2.7.8-1 · DESIGN 2.7.8
  - DoD：≥200 条含姓名/电话/邮箱/证件/公司名的样本，脱敏召回 ≥99%、误伤 ≤5%；默认配置抓包验证无未脱敏查询（人工复核记录归档）

### 38. 提示卡合成器

- [ ] **38.1 标题与结论长度约束** — `crates/application/src/proactive/composer.rs`
  - 追溯 SPEC 6.4-3/4、5.4.1-6 · DESIGN 2.2.2-E
  - DoD：标题 ≤40 字、结论 ≤80 字由 `TitleText`/`ConclusionText` 构造期保证；超长时重新生成一次，仍超长则不出卡（不静默截断）
- [ ] **38.2 引用必备与无来源处理** — `crates/application/src/proactive/composer_refs.rs`
  - 追溯 SPEC 5.3.1-8、6.4-5 · DESIGN 2.6.5
  - DoD：每张卡 ≥1 条 `Reference`；无可用结果时 `conclusion` 为空并出"未找到可靠来源"提示；不生成推测结论（样本验证）
- [ ] **38.3 可信度分级与抓取时间** — `crates/domain/src/model/reference.rs`
  - 追溯 SPEC 5.3.1-9、6.5-6/4 · DESIGN 2.6.3、2.3.2
  - DoD：四档（官方/权威媒体/一般网页/用户生成内容）由域名规则 + 源权重计算，规则可配；抓取时间必填且不晚于当前时间；UI 同时展示等级与时间
- [ ] **38.4 时效标注** — `crates/application/src/proactive/freshness.rs`
  - 追溯 SPEC 5.3.1-11 · DESIGN 2.6.5
  - DoD：发布时间早于 12 个月 → 标注"信息较早，请核实"；发布时间未知 → "发布时间未知"；时效标签随来源逐条展示
- [ ] **38.5 来源冲突并列** — `crates/application/src/proactive/conflict.rs`
  - 追溯 SPEC 5.3.3-5 · DESIGN 2.6.5
  - DoD：两个 ≥权威媒体级来源结论矛盾时并列展示双方与来源，不给单边结论；UI 显示"来源结论不一致，请自行判断"；冲突样本回归 ≥20 组
- [ ] **38.6 分层出卡兜底** — `crates/application/src/proactive/partial_card.rs`
  - 追溯 SPEC 4.1-3 · DESIGN 2.1.3(6) 分层出卡兜底
  - DoD：抓取 4s 未完成时先出"结论待补全"卡；正文到达后原地增补（不重复推送、去重键不变）；实测该路径使 p95 保持在 6s 内
- [ ] **38.7 合成质量回归** — `crates/application/tests/card_corpus/`
  - 追溯 SPEC 4.4-4 · DESIGN 2.6.6
  - DoD：≥100 组"正文+触发上下文"样本；结论-来源支撑率 ≥95%；合成耗时 p95 ≤500ms

### 39. 提示卡校验器与撤回闭环

- [ ] **39.1 引用对齐校验** — `crates/application/src/proactive/validator.rs`
  - 追溯 SPEC 5.3.1-8 · DESIGN 2.6.5（结论-来源对齐）
  - DoD：从正文抽取支撑句，与结论余弦 ≥0.60 且实体一致；不通过则重写一次，仍不通过不出结论；对齐失败率可观测
- [ ] **39.2 合规终检与来源丢弃** — `crates/application/src/proactive/content_filter.rs`
  - 追溯 SPEC 5.3.3-7、5.3.1-16 · DESIGN 2.6.5（合规过滤）
  - DoD：正文命中敏感词/违规内容 → 丢弃该来源并联审计；剩余来源不足则不出卡；卡片显示"该来源内容被过滤"（若整卡被过滤则不出卡）
- [ ] **39.3 反馈驱动撤回与下架** — `crates/application/src/proactive/withdraw.rs`
  - 追溯 SPEC 5.4.1-8、5.4.3-3 · DESIGN 2.6.4-9、2.6.5（撤回）
  - DoD：标记"错误" → 卡片 `已撤回`（灰化 + 原因）；同主题同来源内容不再推送；写质量清单；撤回后可被审计查询
- [ ] **39.4 卡片生命周期状态完整性** — `crates/domain/src/model/card_state.rs`
  - 追溯 SPEC 6.4-7 · DESIGN 2.1.3(3)
  - DoD：业务状态仅"已展示/已收起/已撤回"；内部状态（候选/排队/生成中/校验中/已放弃/已拒绝）只写链路追踪不写业务表（断言：CardRepository 无内部状态字段）

### 40. 全自动提示管线与五级门控 🔴 CP

- [ ] **40.1 L1 硬门控纯函数** — `crates/domain/src/guardrail/evaluate.rs`
  - 追溯 SPEC 5.3.1-3/14/15/16、5.4.1-3/5/11 · DESIGN 2.6.1-L1、2.2.2-F
  - DoD：`evaluate(snapshot) -> Decision` 无 IO、无锁、可重放；覆盖 11 个 `DenyCode`；同一输入必同输出（属性测试）；每个拒绝写链路追踪并计数
- [ ] **40.2 L2 相关性评分** — `crates/domain/src/guardrail/relevance.rs`
  - 追溯 SPEC 5.4.1-2 · DESIGN 2.6.3
  - DoD：`0.40×semantic + 0.25×entity + 0.20×gain + 0.15×sourceTrust` 后乘时间衰减 `exp(-Δt/180s)`；阈值档位 0.72/0.62/0.52；各分项可单独输出（可解释）；端侧嵌入模型 ≤80MB
- [ ] **40.3 管线编排与时延预算** — `crates/application/src/proactive/pipeline.rs`
  - 追溯 SPEC 4.1-3、5.3.1-1 · DESIGN 2.1.3(6)(7)
  - DoD：缺口识别与 ASR partial 并行；检索/抓取流水线化；6s 预算硬截断（超时走摘要兜底或放弃）；各阶段打点齐全
- [ ] **40.4 全自动零操作路径验证** — `crates/application/tests/e2e/auto_path.rs`
  - 追溯 SPEC 5.3.1-1a、5.4.1-1 · DESIGN 2.1.3(6)
  - DoD：在无任何用户操作下，回放 ≥200 次触发，自动产出并推送卡片成功率 ≥95%；端到端 p95 ≤6s、p99 ≤10s（样本 ≥200）
- [ ] **40.5 全自动关闭后行为** — `crates/application/tests/e2e/auto_off.rs`
  - 追溯 SPEC 5.3.1-1b、5.4.1-11 · DESIGN 2.1.3(6)
  - DoD：关闭自动模式后自动检索调用 0、主动推送 0；要点与按需补全正常；不得"以手动优先替代自动"（产品断言）
- [ ] **40.6 链路追踪 `TraceChain`** — `crates/application/src/proactive/trace.rs`
  - 追溯 SPEC 4.4-3、6.4-1 · DESIGN 2.3.2 `TraceChain`
  - DoD：每张卡携带可追溯编号，可沿"触发片段→尝试的源→脱敏查询→引用→展示→反馈"完整回溯；抽样 10 张卡链路完整率 100%

### 41. 去噪与限流策略组合

- [ ] **41.1 语义聚类与去重键** — `crates/domain/src/gap/dedupe_key.rs`
  - 追溯 SPEC 6.4-9、5.3.1-10 · DESIGN 2.3.2（去重键工程定义）
  - DoD：`DedupeKey = SHA-256(topic_cluster_id ‖ normalized_entities ‖ primary_domain)[:16]`；`topic_cluster_id` 由余弦 ≥0.88 聚类得出（非关键词哈希）；10 分钟窗口同键 0 重复推送
- [ ] **41.2 主题配额与令牌桶** — `crates/application/src/proactive/quota.rs`
  - 追溯 SPEC 5.4.1-1、5.7.1-6 · DESIGN 2.6.4-3/4
  - DoD：单主题 10min ≤2、全局 ≤3（默认档）；频率档位低 1/中 3/高 5 生效；桶容量与补充速率可配；超限候选被合并而非丢弃
- [ ] **41.3 实体冷却与候选收敛** — `crates/application/src/proactive/cooldown.rs`
  - 追溯 SPEC 5.3.3-9 · DESIGN 2.6.4-5/6
  - DoD：同实体 5min 内不重复触发；候选 >10 时按 score 取前 3，其余丢弃并计数；无阻塞提示，UI 显示"已收敛自动提示数量"
- [ ] **41.4 抑制列表** — `crates/application/src/proactive/suppression.rs`
  - 追溯 SPEC 5.3.1-14、5.4.3-8 · DESIGN 2.6.4-8
  - DoD：主题/来源/实体三类抑制规则可增删；自动推送受限、按需补全不受限但提示"已抑制"；"不再提示此类内容"写入抑制规则并立即生效
- [ ] **41.5 负反馈加权** — `crates/application/src/proactive/feedback.rs`
  - 追溯 SPEC 5.4.1-8、4.4-4 · DESIGN 2.6.4-9
  - DoD："无用"×2 → 该主题降权 0.5；"错误" → 撤回并下架同源同类；采纳率与误触发率按会话/平台/主题统计正确
- [ ] **41.6 提示风暴压测** — `crates/application/tests/load/storm.rs`
  - 追溯 SPEC 5.3.3-9、4.1-4 · DESIGN 2.6.1（漏斗容量预算）
  - DoD：10 分钟内注入 50 个候选，实际推送 ≤3；合并提示"N 条已收起"正确；CPU 增量 ≤10%；UI 无卡顿（>200ms 帧 0 次）

### 42. 缓存与断网 outbox

- [ ] **42.1 24h 查询缓存** — `crates/infrastructure/src/cache/query_cache.rs`
  - 追溯 SPEC 5.3.1-13、5.3.1-10 · DESIGN 2.3.2
  - DoD：同脱敏查询 24h 内优先复用并标注"结果来自 X 小时前"；命中缓存不发起新检索（打点验证）；缓存随会话删除一并清除
- [ ] **42.2 断网 outbox 与网络状态机** — `crates/application/src/proactive/outbox.rs`
  - 追溯 SPEC 4.2-5 · DESIGN 2.1.3(9)
  - DoD：断网期间采集/要点/摘要继续；检索任务持久化入 outbox（重启不丢）；外发尝试数为 0；网络恢复被正确识别（≤3s）
- [ ] **42.3 恢复后 30s 内补齐** — `crates/application/src/proactive/backfill.rs`
  - 追溯 SPEC 4.2-5、4.2-3 · DESIGN 2.1.3(9)
  - DoD：恢复后 30s 内补齐率 ≥98%；补齐并发 ≤3 且按优先级；补齐产生的卡片去重正确（不重复推送）
- [ ] **42.4 全源不可用降级队列** — `crates/application/src/proactive/degrade_queue.rs`
  - 追溯 SPEC 4.2-3、5.3.3-1 · DESIGN 2.1.3(3)(9)
  - DoD：全部源不可用时进降级队列 + 20s 内自动重试一次；UI 明确显示降级状态；恢复后 30s 内补齐

### 43. 按需补全（补充路径）

- [ ] **43.1 发起入口与查询构造** — `crates/application/src/proactive/on_demand.rs`
  - 追溯 SPEC 5.3.1-4、4.3-4 · DESIGN 2.1.3(7)
  - DoD：可从片段/要点/术语发起；查询 ≤200 字经统一脱敏出口；卡片标注"按需补全"；与自动卡片分别统计
- [ ] **43.2 8s 首卡预算** — `crates/application/src/proactive/on_demand_budget.rs`
  - 追溯 SPEC 4.1-3 · DESIGN 2.1.3(7)
  - DoD：发起到首卡展示 p95 ≤8s（≥200 次样本）；超时给出"检索较慢"提示与重试入口
- [ ] **43.3 抑制列表对按需的限制语义** — `tests/e2e/on_demand_suppressed.rs`
  - 追溯 SPEC 5.3.1-14 · DESIGN 2.6.4-8
  - DoD：被抑制主题的按需补全**允许执行**但提示"该主题已加入抑制列表"；不静默拦截

---

## 批次 B5 · 呈现与交互（W14–W17，前端可提前启动）

**目标**：实现非模态低打扰推送、投屏保护、紧急遮蔽与会中要点流，满足交互 ≤200ms 与不抢焦点。
**前置**：B0.1.3（IPC 冻结）、B4.40（门控输出）。
**⛔ 出口门禁 G5**：焦点检测 0 次抢占；共享屏幕 1s 内正文不可见；紧急遮蔽 p99 ≤1s；交互 ≤200ms。
**并行**：44/45/46/47/48/49 之间仅共享 UI 状态层，可按页面并行；47 依赖 31/33 的要点接口。

### 44. 低打扰推送与三形态展示

- [ ] **44.1 三形态切换与状态保持** — `app/src/features/presenter/`
  - 追溯 SPEC 5.4.1-9 · DESIGN 2.1.2 表现层、2.2.2-G
  - DoD：卡片流/侧边悬浮窗/角标三形态可切换；切换后提示列表与滚动位置保持；切换耗时 ≤200ms
- [ ] **44.2 非模态与不抢焦点** — `crates/platform-win/src/window/no_activate.rs`、`crates/platform-mac/src/window/nonactivating_panel.rs`
  - 追溯 SPEC 5.4.1-10 · DESIGN 1.1.1（Tauri 托盘/通知行）
  - DoD：Windows 使用 `WS_EX_NOACTIVATE`（必要时 `WS_EX_TOOLWINDOW`），macOS 使用非激活 panel/styleMask；自动化脚本推送 100 次卡片，焦点切换次数 0；输入法/键盘输入不被打断
- [ ] **44.3 配额合并展示** — `app/src/features/presenter/overflow.tsx`
  - 追溯 SPEC 5.4.3-1 · DESIGN 2.6.4-7
  - DoD：超额候选折叠为"另有 N 条低优先级提示已收起"，可展开；展开不触发新检索
- [ ] **44.4 渲染虚拟化与滚动性能** — `app/src/features/presenter/virtual_list.tsx`
  - 追溯 SPEC 4.1-5 · DESIGN 2.1.3(8)
  - DoD：1000 张卡滚动帧率 ≥50fps；展开/标记/滚动响应 ≤200ms（性能面板实测）；内存增量 ≤100MB

### 45. 投屏保护与紧急遮蔽

- [ ] **45.1 共享屏幕模式** — `crates/application/src/present/share_guard.rs`、`app/src/features/presenter/share_mode.tsx`
  - 追溯 SPEC 5.4.1-5、5.4.3-2 · DESIGN 2.1.3(3)、1.1.3-14
  - DoD：开启后主屏可见区域仅剩"有 N 条提示"角标；截图比对证明正文不可见；提示"已进入投屏保护模式"
- [ ] **45.2 显示拓扑变化自动保护** — `crates/infrastructure/src/display/watcher.rs`
  - 追溯 SPEC 5.4.1-5 · DESIGN 1.1.3-14
  - DoD：检测到投屏/外接显示器/共享开始时 1s 内进入保护；投屏结束后不自动退出保护（需用户确认，避免误泄露）
- [ ] **45.3 紧急遮蔽 1s 执行** — `crates/application/src/present/blackout.rs`
  - 追溯 SPEC 5.4.1-7、5.6.1-8 · DESIGN 2.2.2-G
  - DoD：触发后 p99 ≤1s 清空全部会话文本与提示；同时调用 `CaptureCoordinator.stop()` 与 `ProactivePipeline.suspend()`（断言两调用均发生）；状态持久化，重启后仍为遮蔽态
- [ ] **45.4 遮蔽失败强制兜底** — `crates/application/src/present/blackout_fallback.rs`
  - 追溯 SPEC 5.4.3-4 · DESIGN 1.1.3-14
  - DoD：注入 UI 阻塞，1s 未完成则强制关闭提示面板与字幕窗口、停止采集与自动检索；提示"已强制停止采集，请检查界面残留"；兜底路径可测试

### 46. 一键暂停/恢复自动提示

- [ ] **46.1 暂停/恢复开关** — `crates/application/src/present/auto_toggle.rs`
  - 追溯 SPEC 5.4.1-4 · DESIGN 2.1.3(6)
  - DoD：一键暂停后自动外发检索与推送调用数立即为 0；要点继续记录（计数验证）；恢复需用户操作
- [ ] **46.2 常驻状态指示** — `app/src/features/presenter/status_bar.tsx`
  - 追溯 SPEC 5.4.1-4 · DESIGN 2.5.6
  - DoD：暂停时界面常驻"自动提示已暂停"；角标模式下也可辨识；状态与后端一致（订阅事件，非轮询）
- [ ] **46.3 关闭后不可自动恢复** — `crates/application/src/present/switch_persistence.rs`
  - 追溯 SPEC 4.3-6、5.4.1-11 · DESIGN 2.7.2
  - DoD：关闭态持久化；重启、升级、新建会话三条入口均校验为关闭（三条各 1 条自动化用例）；任何配置项无法静默开启

### 47. 会中要点流 UI

- [ ] **47.1 要点卡片与置信度样式** — `app/src/features/insights/card.tsx`
  - 追溯 SPEC 5.2.1-1/2/8、6.3-6 · DESIGN 2.1.3(5)
  - DoD：展示类型、时间戳、置信度、来源跳转；低置信显示"待确认"样式；点击时间戳跳转原文并高亮
- [ ] **47.2 编辑/删除/重要标记** — `app/src/features/insights/actions.tsx`
  - 追溯 SPEC 5.2.1-10 · DESIGN 2.3.2
  - DoD：编辑后锁定（`status=已编辑`）；删除即时从流中移除；标记重要置顶或加徽标；上述操作 ≤200ms 响应
- [ ] **47.3 筛选与原文跳转** — `app/src/features/insights/filter.tsx`
  - 追溯 SPEC 5.2.2 · DESIGN 2.1.2
  - DoD：按类型/关键词筛选即时生效；筛选结果可跳转原文；清空筛选恢复完整列表与位置
- [ ] **47.4 要点过载折叠** — `app/src/features/insights/overflow.tsx`
  - 追溯 SPEC 5.2.3-3 · DESIGN 2.1.3(8)
  - DoD：10 分钟内 >30 条时按重要度与类型配额收敛，其余折叠为"更多要点"；提示"要点较多，已折叠次要内容"
- [ ] **47.5 字幕流与来源/静音/重叠标签** — `app/src/features/subtitles/stream.tsx`
  - 追溯 SPEC 5.1.1-4/6/7、5.1.3-4 · DESIGN 2.5.5
  - DoD：每条片段显示来源类型（麦克风/系统音频/平台字幕/外部转写/双通道）；"未知说话人"如实显示；静音时显示"静音中"；重叠片段显示"重叠发言，参考价值有限"

### 48. 状态、错误与诊断 UI

- [ ] **48.1 会话状态卡** — `app/src/features/session/status_card.tsx`
  - 追溯 SPEC 5.8.1-4、6.1-7 · DESIGN 2.1.3(4)
  - DoD：展示平台名称、客户端版本、各通道状态（可用/降级/不可用）、字幕是否开启、当前生效通道；状态变更 5s 内可见
- [ ] **48.2 平台能力差异声明** — `app/src/features/session/capability.tsx`
  - 追溯 SPEC 5.8.1-7 · DESIGN 2.5.6
  - DoD：按平台展示字幕可读性、典型延迟、说话人标识支持、需用户开启字幕、推荐通道、已知限制；数据来自 CapabilityMatrix 而非前端硬编码
- [ ] **48.3 错误码文案与重试入口** — `app/src/features/common/error_banner.tsx`
  - 追溯 SPEC 5.1.3、5.3.3、5.6.3、5.7.3 · DESIGN 2.2.1
  - DoD：11 个错误码 100% 有中文文案与建议动作；可重试错误提供重试按钮；文案不暴露内部路径与凭据
- [ ] **48.4 诊断面板** — `app/src/features/diagnostics/panel.tsx`
  - 追溯 SPEC 4.4-1/5 · DESIGN 2.6.6
  - DoD：展示通道失效率（分平台）、源失败率（分源）、提炼落后条数、摘要状态、遮蔽状态、平台版本与失败原因分类；数据来自本地度量库

### 49. 托盘、通知与快捷键

- [ ] **49.1 托盘常驻与菜单** — `app/src-tauri/src/tray.rs`
  - 追溯 SPEC 5.7.1-4 · DESIGN 2.4.1
  - DoD：关闭悬浮窗后托盘仍可唤起；菜单含开始/暂停/显示提示/退出；退出时若有会话则二次确认
- [ ] **49.2 桌面通知（不抢焦点）** — `crates/infrastructure/src/notify/mod.rs`
  - 追溯 SPEC 1.3-6、5.4.1-10 · DESIGN 2.2.2-G
  - DoD：主动提示、留存到期、通道异常三类通知可发；通知不获取键盘焦点（自动化验证）；通知内容经脱敏出口
- [ ] **49.3 全局快捷键** — `crates/platform-win/src/hotkey.rs`、`crates/platform-mac/src/hotkey.rs`
  - 追溯 SPEC 5.7.1-4 · DESIGN 2.4.1
  - DoD：紧急遮蔽、暂停提示、唤起面板三个动作可配；冲突检测并提示；macOS 需辅助功能权限时给出引导
- [ ] **49.4 开机自启开关** — `app/src-tauri/src/autostart.rs`
  - 追溯 SPEC 5.7.1-4 · DESIGN 2.4.1
  - DoD：默认关闭；开启后写入系统登录项（Win 注册表/Run 或计划任务、macOS LoginItems）；关闭可完全移除；状态可回读

---

## 批次 B6 · 治理（W15–W17）

**目标**：策略不可绕过、授权不可静默恢复、审计完整、删除不可恢复。
**前置**：B0.5（存储/密钥）、B0.6（审计链）、B2.26（会话状态机）。
**⛔ 出口门禁 G6**：隐私检查清单 1–6（DESIGN 2.7.8）全部通过；撤销授权三入口均不恢复；删除 ≤1 分钟且残留 0。
**并行**：50/51/53/54 可并行；52 依赖 50/51（策略与授权决定审计字段）。

### 50. 策略引擎（组织优先、不可绕过）

- [ ] **50.1 签名策略包验签** — `crates/infrastructure/src/policy/signed_bundle.rs`
  - 追溯 SPEC 5.6.1-6、6.7-1 · DESIGN 2.7.6
  - DoD：Ed25519 验签（内置公钥 + 支持轮换）；`scope="org"`/`rules`/`mandatory`/`issued_at`/`nonce` 字段校验；篡改任意字节 → 拒绝加载并提示（≥20 组篡改用例）；`nonce` 防重放
- [ ] **50.2 取严求值与 mandatory 锁定** — `crates/domain/src/policy/resolver.rs`
  - 追溯 SPEC 6.7-3/4/8、5.6.3-3 · DESIGN 2.7.6
  - DoD：`org.mandatory` 直接锁定；其余逐项取严（留存取小、告知取严、脱敏取开）；强制脱敏不可关（断言：个人配置无法覆盖）；`auditRetentionDays` 固定 180 且不可改
- [ ] **50.3 四处重求值与 60s 缓存上限** — `crates/application/src/policy/refresh.rs`
  - 追溯 SPEC 5.6.1-6 · DESIGN 2.7.6
  - DoD：启动、策略文件变更、会话开始、导出前四个时点重新求值（各 1 条用例）；任何模块缓存策略不得超过 60s（代码断言）
- [ ] **50.4 冲突提示与来源展示** — `app/src/features/policy/conflict.tsx`
  - 追溯 SPEC 5.6.3-3、5.7.3-2 · DESIGN 2.7.6
  - DoD：冲突项显示"该项由组织策略限定"与具体来源；合法项仍按原计划保存；提示不阻断其他配置保存

### 51. 自动外发授权模型 🛡 合规

- [ ] **51.1 授权向导与范围展示** — `app/src/features/consent/outbound_grant.tsx`
  - 追溯 SPEC 4.3-5、5.3.1-3 · DESIGN 2.7.2
  - DoD：逐项展示内容类型（仅文本查询）、长度上限（自动 80/按需 200）、启用源清单；未逐项确认不可完成；完成后生成 `OutboundGrant` 并写审计
- [ ] **51.2 未授权时仅本地要点** — `crates/application/src/policy/grant_guard.rs`
  - 追溯 SPEC 5.3.1-3a、5.3.3-10 · DESIGN 2.7.2
  - DoD：未授权 → `Deny{NotAuthorized}`，自动检索调用 0；界面显示"未授权自动外发，自动提示暂不可用"；授权入口常驻可点
- [ ] **51.3 撤销不可自动恢复（三重校验）** — `crates/application/src/policy/revocation.rs`
  - 追溯 SPEC 4.3-6、5.4.1-11 · DESIGN 2.7.2
  - DoD：`revoked_at` 持久化；重启/升级/新建会话三条入口均校验为撤销（三条自动化用例）；恢复必须重走向导并重新写审计
- [ ] **51.4 授权变更即时失效** — `crates/application/src/policy/grant_invalidate.rs`
  - 追溯 SPEC 4.3-5 · DESIGN 2.7.2
  - DoD：新增源/提高上限/更换源集合时旧授权立即失效并回落 `TextQueryOnly + 最小上限`；失效后自动外发调用 0，直至重新确认
- [ ] **51.5 音频上传逐次确认** — `app/src/features/consent/audio_upload.tsx`
  - 追溯 SPEC 4.3-2、5.6.1-2 · DESIGN 2.7.2
  - DoD：默认音频不离开本机（抓包 0 音频上行）；每次上传需单独确认并写审计；未确认时 `Transcriber` 仅用本地实现
- [ ] **51.6 授权范围与实际外发一致性** — `crates/application/tests/e2e/grant_scope.rs`
  - 追溯 SPEC 4.3-5、2.7.8-5 · DESIGN 2.7.2
  - DoD：抓包过滤验证外发仅含已授权源、仅文本、长度 ≤上限；越界请求 0 次

### 52. 审计事件接入与全链路追踪 🛡 合规

- [ ] **52.1 八类操作接入** — `crates/application/src/governance/audit_hooks.rs`
  - 追溯 SPEC 4.3-7、6.8-2 · DESIGN 2.7.4
  - DoD：会话开始/结束、检索、提示推送、纪要导出、会话删除、策略与源变更、越权尝试八类全部产生记录；每条含 `actor`（角色+账户）、`occurred_at`（秒）、`target`、脱敏摘要
- [ ] **52.2 1s 内落库与正文隔离** — `crates/infrastructure/src/audit/write_path.rs`
  - 追溯 SPEC 5.6.1-5、6.8-6 · DESIGN 2.7.4
  - DoD：敏感操作到落库 p99 ≤1s；摘要禁止原文（检索只记"长度+主题标签+源 ID"）；1000 条审计扫描 0 命中会话原文
- [ ] **52.3 卡片全链路编号** — `crates/application/src/proactive/trace_persist.rs`
  - 追溯 SPEC 4.4-3 · DESIGN 2.3.2 `TraceChain`
  - DoD：卡片编号串联触发片段→检索请求→引用→展示→反馈；按编号可 1 次查询取回完整链路（接口 + UI 入口）；链路不含原文
- [ ] **52.4 180 天保留与到期归档** — `crates/infrastructure/src/audit/retention.rs`
  - 追溯 SPEC 4.3-7、6.8-7 · DESIGN 2.7.4
  - DoD：保留期固定 180 天（策略不可改）；到期整体加密封存到归档区而非删除；归档包可校验哈希链完整性

### 53. 权限与最小访问

- [ ] **53.1 会话归属校验** — `crates/application/src/access/ownership.rs`
  - 追溯 SPEC 4.3-9、5.6.1-4 · DESIGN 2.7.6
  - DoD：普通使用者仅能访问自己参与的会话（列表与详情双重校验）；越权返回 `E-AUTH-008` 并写审计
- [ ] **53.2 管理员视图边界** — `app/src/features/admin/`、`crates/application/src/access/admin_scope.rs`
  - 追溯 SPEC 4.3-9、3.1-3 · DESIGN 2.7.6
  - DoD：管理员默认仅能配置策略与查看统计；尝试读取原文被拒绝；界面不提供原文入口
- [ ] **53.3 二次授权与留痕** — `crates/application/src/access/secondary_grant.rs`
  - 追溯 SPEC 4.3-9 · DESIGN 2.7.6
  - DoD：原文访问需组织二次授权 + 二次确认；每次访问产生独立审计（含授权凭证引用）；授权有效期可配
- [ ] **53.4 权限矩阵测试** — `crates/application/tests/access_matrix.rs`
  - 追溯 SPEC 4.3-9 · DESIGN 2.9（权限矩阵测试）
  - DoD：角色 × 资源 × 操作矩阵用例全通过；越权用例 100% 拒绝并留痕

### 54. 留存、提醒与删除执行器 🛡 合规

- [ ] **54.1 每日清理与到期提醒** — `crates/application/src/retention/scheduler.rs`
  - 追溯 SPEC 5.5.1-7 · DESIGN 2.7.5
  - DoD：每日 02:00 触发（时钟可注入）；到期前 7 天发桌面提醒（仅一次）；提醒记录进审计
- [ ] **54.2 近期访问顺延** — `crates/application/src/retention/access_guard.rs`
  - 追溯 SPEC 5.5.3-4 · DESIGN 2.7.5
  - DoD：最近 60 分钟内被访问的会话本次跳过并顺延 1 天；无打扰提示；审计记录"因近期访问顺延"
- [ ] **54.3 不可恢复删除事务** — `crates/application/src/retention/purge.rs`
  - 追溯 SPEC 4.3-8、5.5.1-8 · DESIGN 2.7.5
  - DoD：顺序为"销毁会话密钥盐 → 删正文与索引 → 置已删除"；1 万片段库删除 ≤60s；确认后不可取消；进度可见
- [ ] **54.4 删除残留扫描** — `scripts/scan-residue.sh`、`crates/infrastructure/tests/residue.rs`
  - 追溯 SPEC 4.3-8、2.7.8-4 · DESIGN 2.7.5
  - DoD：删除后对 DB 文件、WAL、FTS、向量索引全文扫描，该会话文本命中 0；仅保留不含原文的审计摘要；脚本进 CI
- [ ] **54.5 留存上限与策略联动** — `crates/domain/src/policy/retention.rs`
  - 追溯 SPEC 6.1-11、6.7-4 · DESIGN 2.7.6
  - DoD：`retentionUntil` 不得晚于组织上限；个人设置超过上限被拒绝并提示来源；1–365 边界值测试通过

---

## 批次 B7 · 会话纪要与回顾（W17–W19）

**目标**：会话结束后 30s 内产出可确认、可导出、可检索、可归档的纪要。
**前置**：B3（要点/摘要）、B6（策略/留存/审计）。
**⛔ 出口门禁 G7**：1h 会话纪要 ≤30s；三格式导出内容一致；未确认 0 归档；待确认条目 0 进入结论区。
**并行**：55 与 58 可并行；56/57 依赖 55。

### 55. 纪要合成器 🔴 CP

- [ ] **55.1 概要、四类清单与完整时间轴** — `crates/application/src/minutes/composer.rs`
  - 追溯 SPEC 5.5.1-1/3、4.1-9 · DESIGN 2.1.2、1.1.2（纪要导出）
  - DoD：生成概要 + 四类要点清单 + 时间轴；时间轴覆盖整段会话（含"无要点区间"，不跳格、不压缩刻度）；时间轴与要点时间戳 100% 对齐
- [ ] **55.2 未决问题与待跟进区** — `crates/application/src/minutes/followups.rs`
  - 追溯 SPEC 5.5.1-4、6.3-11 · DESIGN 2.6.5
  - DoD：未答复疑问与"待确认"字段自动汇总到"待跟进事项"区；该区位于纪要顶部；计数与实际条目一致
- [ ] **55.3 来源清单与引用** — `crates/application/src/minutes/references.rs`
  - 追溯 SPEC 5.5.1-1、6.5 · DESIGN 2.3.2
  - DoD：纪要附来源清单（标题/地址/抓取时间/可信度）；每条结论可追溯到卡片编号；无来源的推测内容 0 条进入"结论"部分
- [ ] **55.4 30s 生成与草稿状态** — `crates/application/src/minutes/generate.rs`
  - 追溯 SPEC 5.5.1-1、4.1-9 · DESIGN 2.1.3(7)
  - DoD：1h 会话（≥2000 片段）生成 ≤30s；生成后状态为"草稿"；生成期间 UI 可交互（不阻塞主线程）
- [ ] **55.5 部分纪要与重试** — `crates/application/src/minutes/partial.rs`
  - 追溯 SPEC 5.5.3-1 · DESIGN 2.1.3(9)
  - DoD：提炼队列未完成或超时时生成"部分纪要"并标注缺失区间；提供重新生成入口；返回 `E-MIN-006`
- [ ] **55.6 待确认条目的隔离** — `crates/application/src/minutes/confidence_gate.rs`
  - 追溯 SPEC 5.5.1-9 · DESIGN 2.6.5
  - DoD：置信度为"待确认"的条目只出现在"待跟进事项"区，不进入"结论"部分（渲染层断言 + 数据层标记）

### 56. 纪要确认与归档

- [ ] **56.1 编辑与勾选跟进** — `app/src/features/minutes/editor.tsx`
  - 追溯 SPEC 5.5.2 · DESIGN 2.1.2
  - DoD：可编辑概要、补充结论、勾选/取消待跟进事项；编辑保留版本记录（可回溯谁在何时改了什么）；保存 ≤200ms
- [ ] **56.2 确认后转正式** — `crates/application/src/minutes/confirm.rs`
  - 追溯 SPEC 5.5.1-2 · DESIGN 2.1.2
  - DoD：未确认状态禁止归档（接口层拒绝，非仅 UI 禁用）；确认动作写审计（主体/时间/版本）；确认后可撤销但留痕
- [ ] **56.3 归档到外部知识库/日历** — `crates/infrastructure/src/archive/kb_client.rs`
  - 追溯 SPEC 1.3-8、5.5.1-2 · DESIGN 2.1.1、2.7.1（归档出口）
  - DoD：仅发送"已确认正式"纪要摘要与待办；内容经脱敏出口；不含音频与原文；失败可重试且不改变原纪要状态

### 57. 纪要导出（Markdown / TXT / JSON）

- [ ] **57.1 `MinutesDocument` 中间模型与三 Renderer** — `crates/domain/src/minutes/document.rs`、`crates/application/src/export/{markdown,plaintext,json}.rs`
  - 追溯 SPEC 4.5-5、5.5.1-6 · DESIGN 1.1.2（纪要导出）、2.1.2
  - DoD：三格式内容一致（字段差异仅限格式表达，有白名单比对测试）；JSON 带 `schemaVersion`；Markdown 层级清晰可被常规解析器读取
- [ ] **57.2 导出前脱敏复核与打码确认** — `crates/application/src/export/precheck.rs`
  - 追溯 SPEC 5.5.1-6、5.5.3-3 · DESIGN 2.7.1（纪要/导出出口）
  - DoD：导出前全量 `scan`；命中默认打码并生成脱敏说明，需用户确认后才输出；未确认时导出被阻断；提示"已发现 N 处敏感信息，已打码"
- [ ] **57.3 导出失败重试** — `crates/application/src/export/retry.rs`
  - 追溯 SPEC 5.5.3-2 · DESIGN 2.1.2
  - DoD：目标路径不可写/被占用/超限时保留导出任务，允许换路径重试；不修改原会话数据；错误文案明确
- [ ] **57.4 PDF 可选导出** — `crates/application/src/export/pdf.rs`
  - 追溯 SPEC 4.5-5、2.8.3-P3 · DESIGN 1.1.2
  - DoD：开关默认关闭；开启后可从 `MinutesDocument` 渲染 PDF（中文字体嵌入）；PDF 与 MD 内容一致；不引入 GPL 组件

### 58. 历史纪要与检索

- [ ] **58.1 多维筛选** — `crates/application/src/minutes/search.rs`
  - 追溯 SPEC 5.5.1-5 · DESIGN 2.3.1-3
  - DoD：支持关键词、时间范围、要点类型、参会人（如已知）、会话标签组合筛选；10 万片段库 p95 ≤2s；结果分页
- [ ] **58.2 FTS5 检索与上下文高亮** — `crates/infrastructure/src/store/fts.rs`
  - 追溯 SPEC 5.5.1-5 · DESIGN 2.3.1-3
  - DoD：中文分词可用（FTS5 trigram 或分词器方案二选一并记录理由）；命中片段返回上下文并高亮；高亮位置与实际文本索引一致
- [ ] **58.3 空结果建议** — `app/src/features/minutes/empty_state.tsx`
  - 追溯 SPEC 5.5.3-5 · DESIGN 2.1.2
  - DoD：无结果时显示"未找到结果，试试缩短关键词或扩大时间范围"；提供一键放宽条件操作

---

## 批次 B8 · 配置、个性化与质量反馈（W18–W19）

**目标**：所有配置"开放可配置且不可绕过策略"，质量指标可视、可告警、可收敛。
**前置**：B4（源）、B6（策略）。
**⛔ 出口门禁 G8**：5 源配置与热生效；策略冲突项被拒；10 项指标可查；周度抽检流程可执行。
**并行**：59/60/61/62/63/64 为独立页面/模块，可并行（共享 IPC DTO）。

### 59. 术语表导入与维护

- [ ] **59.1 CSV 导入与校验** — `crates/application/src/lexicon/importer.rs`
  - 追溯 SPEC 5.7.1-1、6.6-1/2 · DESIGN 1.1.2
  - DoD：校验列缺失、编码异常、空值、重复；报告成功/失败条数与逐行错误；支持"整体拒绝"与"仅导入合法行"两种模式
- [ ] **59.2 错误清单下载** — `app/src/features/lexicon/import_result.tsx`
  - 追溯 SPEC 5.7.3-1 · DESIGN 2.1.2
  - DoD：失败行可下载 CSV（含行号与原因）；提示"导入失败 N 行，可下载错误清单"；重试仅处理失败行
- [ ] **59.3 术语增删改与启停/敏感标记** — `crates/application/src/lexicon/crud.rs`
  - 追溯 SPEC 6.6-3/4/5 · DESIGN 2.3.2
  - DoD：类型（产品/人名/缩写/行业词汇/其他）、敏感标记、启停均可维护；敏感术语不进入外发查询与导出（与 37.1 联动断言）；停用术语不参与纠正

### 60. 检索源配置 UI 🧩 SPI

- [ ] **60.1 新增/编辑/启停/排序** — `app/src/features/providers/editor.tsx`
  - 追溯 SPEC 5.7.1-2、6.9-5/6 · DESIGN 2.2.2-D
  - DoD：可维护名称（≤50 字）、类型（四类）、优先级（唯一）、权重（0–100）、超时（≤8s）、限额、屏蔽域名；排序可拖拽且保存后即时生效
- [ ] **60.2 连接测试与失败原因** — `app/src/features/providers/test_connection.tsx`
  - 追溯 SPEC 5.7.1-2a、5.7.3-5 · DESIGN 2.2.2-D
  - DoD：测试返回可用或失败分类；失败不保存为启用态但保留草稿；提示"连接测试失败：鉴权失败（401）"并可修改重试
- [ ] **60.3 组织策略范围限制** — `crates/application/src/providers/scope_guard.rs`
  - 追溯 SPEC 5.7.1-2c、6.7-5 · DESIGN 2.7.6
  - DoD：组织限定范围内时超出范围的源被拒绝并说明策略来源；拒绝不丢弃已填内容（存草稿）；不指定唯一服务商（界面无"推荐/默认"暗示）
- [ ] **60.4 密钥引用与导出脱敏** — `crates/infrastructure/src/providers/secret_ref.rs`
  - 追溯 SPEC 6.9-4 · DESIGN 2.2.2-H
  - DoD：`authRef` 仅存引用，密钥入 OS 密钥库；配置导出不含明文（导出文件扫描 0 命中）；导入时密钥需重新录入

### 61. 会议平台接入配置

- [ ] **61.1 平台多选与通道优先级** — `app/src/features/platforms/config.tsx`
  - 追溯 SPEC 5.7.1-3、6.10-1/3/4 · DESIGN 2.5.2
  - DoD：三平台可多选维护（单会话择一）；通道优先级（字幕优先/音频优先）默认字幕优先；关闭字幕通道时提示"延迟将上升"
- [ ] **61.2 识别方式与可用状态** — `crates/application/src/platforms/binding_config.rs`
  - 追溯 SPEC 5.7.1-3、6.10-5/6 · DESIGN 2.5.2
  - DoD：自动识别/手动指定可切换；未安装或版本低于矩阵时标"不可用"且不出现在开始会话列表；可用性来源可查（探测定时刷新）
- [ ] **61.3 适配记录与版本提示** — `app/src/features/platforms/adaptation.tsx`
  - 追溯 SPEC 6.10-7、4.5-2 · DESIGN 2.5.2
  - DoD：展示平台主版本与最近一次适配验证时间；超出矩阵时提示"客户端版本过低，字幕通道不可用，请升级到受支持版本"

### 62. 偏好、语言与提示档位

- [ ] **62.1 桌面偏好** — `app/src/features/settings/preferences.tsx`
  - 追溯 SPEC 5.7.1-4 · DESIGN 2.4.1
  - DoD：开机自启、托盘常驻、悬浮窗位置与透明度、全局快捷键、通知方式均可配并持久化；重启后一致
- [ ] **62.2 语言偏好与识别结果标注** — `crates/application/src/understand/language.rs`
  - 追溯 SPEC 5.7.1-5、4.5-4 · DESIGN 2.5.3-4
  - DoD：中文/英文/自动识别可选；自动模式下在会话信息中显示识别结果与置信度；混合语料识别不降级（回归样本）
- [ ] **62.3 提示频率与展示形态** — `crates/application/src/present/frequency.rs`
  - 追溯 SPEC 5.7.1-6、5.4.1-9 · DESIGN 2.6.4-4
  - DoD：低（1/10min）/中（3/10min）/高（5/10min）生效；主动提示开关默认开；关闭后保留按需补全且自动检索为 0
- [ ] **62.4 置信度档位与影响说明** — `app/src/features/settings/confidence.tsx`
  - 追溯 SPEC 5.7.1-9、2.6.3 · DESIGN 2.6.3
  - DoD：严格 0.72/平衡 0.62/宽松 0.52 可选并即时生效；切换时展示"误报减少、漏报增加"（或反向）说明；变更写审计

### 63. 质量度量报表与告警

- [ ] **63.1 指标看板** — `app/src/features/quality/dashboard.tsx`
  - 追溯 SPEC 4.4-1、5.7.1-7 · DESIGN 2.6.6
  - DoD：10 项指标按会话/平台/主题三维度展示；时间范围可选；数据来自本地度量库且不含原文
- [ ] **63.2 阈值告警与自动收敛** — `crates/application/src/quality/alerting.rs`
  - 追溯 SPEC 4.4-4、5.7.3-3 · DESIGN 2.6.6
  - DoD：采纳率 <20%、误报率 >15%、误触发率 >30% 触发告警；自动收敛（阈值 +0.04/+0.06、频率降档）且写审计；不覆盖用户显式设置
- [ ] **63.3 周度抽检流程** — `crates/application/src/quality/sampling.rs`
  - 追溯 SPEC 5.7.1-7a、4.4-4 · DESIGN 2.6.6
  - DoD：每周抽取 ≥20 条卡片供人工评估；输出准确率、主要错误类型分布与改进建议；样本不足时标注"样本不足，结论仅供参考"
- [ ] **63.4 误触发原因分类统计** — `crates/application/src/quality/deny_stats.rs`
  - 追溯 SPEC 4.4-4、5.4.3-5 · DESIGN 2.2.2-F（DenyCode 复用）
  - DoD：按 11 个 `DenyCode` 与 L0/L2/L3 各级失败原因统计；可定位"自动提示已停止"的具体原因（授权/源/限流/开关）

### 64. 配置审计与策略冲突

- [ ] **64.1 前后值审计** — `crates/application/src/governance/config_audit.rs`
  - 追溯 SPEC 5.7.1-8 · DESIGN 2.7.4
  - DoD：策略、检索源、平台接入三类变更记录变更人、时间、前后值（脱敏后）；密钥类字段只记录"已变更"不记录值
- [ ] **64.2 禁止绕过校验** — `crates/application/src/governance/config_guard.rs`
  - 追溯 SPEC 5.7.1-10 · DESIGN 2.7.6
  - DoD：与组织策略冲突的配置被拒绝保存并提示冲突项；不存在关闭审计/静默开启自动外发的配置项（配置 schema 断言）
- [ ] **64.3 配置项 schema 版本与向后兼容** — `crates/infrastructure/src/config/schema.rs`
  - 追溯 SPEC 4.5-8 · DESIGN 2.2.1（外部配置契约）
  - DoD：配置带 `schemaVersion`，未知字段保留透传；旧配置可被新版本读取（v0→v1 回放测试）；降级读取旧版本时不丢字段

---

## 批次 B9 · 测试、性能与合规门禁（W19–W21）

**目标**：把 design 2.9「验收执行口径」与 2.7.8「隐私检查清单」变成可重复执行的门禁。
**前置**：全部功能批次（B0–B8）。
**⛔ 出口门禁 G9（M3 前）**：性能、可靠性、隐私、平台矩阵四类门禁全绿；P0/P1 缺陷清零。
**并行**：67/68/69 三平台并行；70/71 双系统并行；72/73/74/75 依赖功能完成后并行。

### 65. 单元与约束测试体系

- [ ] **65.1 领域约束测试** — `crates/domain/tests/constraints/`
  - 追溯 SPEC 6.1–6.10 · DESIGN 2.9（数据约束校验）
  - DoD：枚举取值、长度上限、必填、时间顺序、引用同会话、去重键格式逐条有断言；覆盖率 `domain` ≥85%（tarpaulin/llvm-cov）
- [ ] **65.2 状态机与门控纯函数测试** — `crates/domain/tests/state_guard/`
  - 追溯 SPEC 6.1-4、5.3.1 · DESIGN 2.1.3(1)、2.6.1-L1
  - DoD：会话状态机全迁移路径覆盖；`Guardrail::evaluate` 11 个 DenyCode 各 ≥1 用例；属性测试（随机快照）无 panic
- [ ] **65.3 端口契约测试** — `crates/domain/tests/contracts/`
  - 追溯 SPEC 4.5-6、4.5-8 · DESIGN 2.2.1（稳定性分级）
  - DoD：每端口 mock 与真实实现共用同一套断言；`spiVersion` 兼容/不兼容各 1 用例；契约变更需同步更新测试（CI 校验）

### 66. 集成与端到端主链测试

- [ ] **66.1 采集→要点→提示→纪要 主链** — `tests/integration/main_chain.rs`
  - 追溯 SPEC 5.1–5.5、4.1-1/2/3 · DESIGN 2.1.3(5)(6)(7)
  - DoD：用真实录制的三平台样本回放，主链端到端跑通；各阶段时延打点齐全；产物（要点/卡片/纪要）满足 SPEC 6.x 约束
- [ ] **66.2 三通道组合矩阵** — `tests/integration/channel_matrix.rs`
  - 追溯 SPEC 5.8.1-5、4.2-7 · DESIGN 2.5.4-仲裁规则
  - DoD：字幕可用/仅音频/双通道/均不可用 4 种组合 × 3 平台各 1 用例；切换点与会话记录一致
- [ ] **66.3 回放集管理** — `tests/fixtures/README.md`、`scripts/record_fixture.sh`
  - 追溯 SPEC 4.1-1/2/3、4.4-3 · DESIGN 2.9（验收执行口径 1）
  - DoD：提供录制、脱敏、入库、版本化管理流程；回放集不含真实姓名与敏感信息（入库前自动扫描）；单场景样本 ≥200 次

### 67. 腾讯会议接入验证矩阵 🧩 SPI

- [ ] **67.1 版本 × 通道矩阵** — `docs/test/tencent-matrix.md`
  - 追溯 SPEC 4.5-2、5.8.1-1/4 · DESIGN 2.9（验收执行口径 5）
  - DoD：当前主版本及前 2 个主版本 ×（字幕开/关/共享屏幕）×（Win/mac）≤12 组合全部执行并记录；组合不支持的项注明原因
- [ ] **67.2 会话级验收** — `tests/e2e/platform_tencent.rs`
  - 追溯 SPEC 5.8.1-1/4/5 · DESIGN 2.8.2-M1/M2
  - DoD：可建会话并稳定 30 分钟；通道状态正确；字幕→要点 p95 ≤1.5s；双通道不重复
- [ ] **67.3 异常场景演练** — `docs/test/tencent-failures.md`
  - 追溯 SPEC 5.8.3-1~7 · DESIGN 2.9（故障注入矩阵）
  - DoD：未运行/版本过低/字幕未开/升级失效/音频失败/多平台/共享屏幕 7 类场景逐项验证并有结论

### 68. 飞书 / Lark 接入验证矩阵 🧩 SPI

- [ ] **68.1 国内版与国际版分别验证** — `docs/test/feishu-matrix.md`
  - 追溯 SPEC 5.8.1-1、4.5-2 · DESIGN 2.5.2、2.5.6
  - DoD：Feishu 与 Lark 各自 ≥12 组合（版本 × 通道 × 平台）；两套矩阵不混用；OCR 占比与 CPU 增量记录
- [ ] **68.2 会话级验收** — `tests/e2e/platform_feishu.rs`
  - 追溯 SPEC 5.8.1-4/7 · DESIGN 2.8.2-M1
  - DoD：可建会话并稳定 30 分钟；UI 明确标注"字幕依赖 OCR、能力低于原生字幕"；无障碍树开启/关闭两态都可用
- [ ] **68.3 异常场景演练** — `docs/test/feishu-failures.md`
  - 追溯 SPEC 5.8.3-1~7 · DESIGN 2.9（故障注入矩阵）
  - DoD：7 类场景覆盖；OCR 失败率 >50% 时正确切音频并上报

### 69. Zoom 接入验证矩阵 🧩 SPI

- [ ] **69.1 版本矩阵（≥5.17）** — `docs/test/zoom-matrix.md`
  - 追溯 SPEC 4.5-2、5.8.1-1 · DESIGN 2.5.2
  - DoD：5.17/5.18/6.x ×（CC 开/关/共享屏幕）×（Win/mac）≥12 组合执行；<5.17 明确判"字幕不可用、仅音频"
- [ ] **69.2 会话级验收** — `tests/e2e/platform_zoom.rs`
  - 追溯 SPEC 5.8.1-4/5/6 · DESIGN 2.8.2-M1
  - DoD：可建会话并稳定 30 分钟；CC 延迟实测 p95 ≤1.5s；与腾讯会议同时开会时互斥正确
- [ ] **69.3 异常场景演练** — `docs/test/zoom-failures.md`
  - 追溯 SPEC 5.8.3-3/4/6 · DESIGN 2.9（故障注入矩阵）
  - DoD：CC 未开启、CC 窗口被遮挡、版本过低、多平台并行 4 类场景逐项验证

### 70. Windows 平台兼容与权限验证

- [ ] **70.1 系统版本矩阵** — `docs/test/win-os-matrix.md`
  - 追溯 SPEC 4.5-1、5.8.1-4 · DESIGN 1.2.1-1
  - DoD：Win10 1909（设备级回退）/ Win10 2004+（进程级）/ Win11 各验证；能力差异在 UI 如实展示；x64 与 ARM64 均安装运行
- [ ] **70.2 权限与隐私设置场景** — `docs/test/win-permission.md`
  - 追溯 SPEC 5.1.3-1、4.3-1 · DESIGN 2.5.4
  - DoD：麦克风权限拒绝/回收、屏幕录制不适用时的行为正确；错误码与重试入口可用；不静默降级采集
- [ ] **70.3 独占模式与虚拟声卡冲突** — `docs/test/win-audio-conflict.md`
  - 追溯 SPEC 5.8.3-5 · DESIGN 1.2.1-1
  - DoD：构造冲突场景验证回退麦克风并标记"仅麦克风"；提示 `E-AUDIO-002`；会话不中断

### 71. macOS 平台兼容与权限验证

- [ ] **71.1 系统版本与架构矩阵** — `docs/test/mac-os-matrix.md`
  - 追溯 SPEC 4.5-1、5.8.1-4 · DESIGN 1.2.1-2
  - DoD：macOS 13.0/13.4/14.x/15.x × Apple Silicon / Intel 验证；SCK 稳定性记录（重建次数）；不支持版本明确拦截
- [ ] **71.2 屏幕录制与辅助功能权限** — `docs/test/mac-permission.md`
  - 追溯 SPEC 4.3-1、5.8.1-2b · DESIGN 2.5.4
  - DoD：屏幕录制（音频）、辅助功能（字幕）两类权限的未授权/拒绝/受限三态行为正确；深链可跳转；授权后采集可恢复
- [ ] **71.3 休眠/唤醒/多显示器** — `docs/test/mac-lifecycle.md`
  - 追溯 SPEC 4.2-7 · DESIGN 1.2.1-2
  - DoD：系统休眠唤醒后 5s 内恢复采集或明确降级；多显示器/切换主屏时 ROI 与悬浮窗位置正确

### 72. 性能门禁 🔴 CP

- [ ] **72.1 端到端时延门禁** — `tests/perf/latency.rs`、`docs/perf/latency-report.md`
  - 追溯 SPEC 4.1-1/2/3 · DESIGN 2.1.3(7)、2.9（口径 1）
  - DoD：字幕→要点 p95 ≤1.5s；音频→要点 p95 ≤3s；触发→提示 p95 ≤6s / p99 ≤10s；按需→首卡 p95 ≤8s；单场景样本 ≥200，三平台分别统计
- [ ] **72.2 资源门禁（低端机 + 主流机）** — `tests/perf/resources.rs`、`docs/perf/resources-report.md`
  - 追溯 SPEC 4.1-7/8 · DESIGN 2.1.3(8)、2.9（口径 2）
  - DoD：2 小时连续运行内存峰值 ≤2GB、均值统计；空闲 ≤400MB；CPU 均值 ≤35%、10s 峰值 ≤60%；4 核/8GB/集显与主流机双档各出报告
- [ ] **72.3 容量与长稳门禁** — `tests/perf/soak.rs`
  - 追溯 SPEC 4.1-6、4.1-9 · DESIGN 2.1.3(8)
  - DoD：2 小时会话、≥10000 片段不掉点；滚动摘要刷新间隔 ≤30s；1h 纪要生成 ≤30s；内存无单调增长（斜率检验）
- [ ] **72.4 交互性能门禁** — `tests/perf/ui_perf.rs`
  - 追溯 SPEC 4.1-5 · DESIGN 2.1.3(8)
  - DoD：滚动/展开/标记 ≤200ms（p99）；1000 卡虚拟化滚动帧率 ≥50fps；提示风暴场景无 >200ms 掉帧

### 73. 故障注入与降级验证

- [ ] **73.1 故障注入矩阵** — `tests/fault/injection.rs`、`docs/test/fault-matrix.md`
  - 追溯 SPEC 4.2-1~8 · DESIGN 2.1.3(9)、2.9（口径 3）
  - DoD：断网、源超时、源限流、抓取失败、ASR 连续失败、字幕关闭、库只读、审计不可写、脱敏不可用 9 类逐项注入；每类验证"隔离边界生效 + 用户可见降级态"
- [ ] **73.2 主链隔离验证** — `tests/fault/isolation.rs`
  - 追溯 SPEC 4.2-8 · DESIGN 2.1.3(9)
  - DoD：注入检索/抓取/LLM 全失败，要点捕捉、滚动摘要、纪要产出仍正常（各 1 条 e2e 断言）；卡片不出但主链无感
- [ ] **73.3 恢复与补齐时效** — `tests/fault/recovery.rs`
  - 追溯 SPEC 4.2-3/5 · DESIGN 2.1.3(9)
  - DoD：源恢复 30s 内补齐；断网恢复 30s 内补齐；转写失败 3 次后切"仅文本输入"并通知；字幕失效 5s 内切音频

### 74. 隐私与合规检查清单 🛡 合规 ⛔ GATE

- [ ] **74.1 抓包与日志扫描** — `scripts/privacy-gate.sh`、`docs/compliance/privacy-check.md`
  - 追溯 SPEC 4.3-2/4/5、4.4-2 · DESIGN 2.7.8-1/2
  - DoD：默认配置抓包证明无原文外发、无音频外发、无未脱敏查询；1000 条日志抽样 0 命中姓名/电话/邮箱/URL 全文/查询全文
- [ ] **74.2 密钥与删除检查** — `docs/compliance/key-and-delete.md`
  - 追溯 SPEC 4.3-3/8 · DESIGN 2.7.8-3/4
  - DoD：安装目录/配置/日志/崩溃转储无密钥材料；删除后 1 分钟内 DB 与索引不可检出该会话文本
- [ ] **74.3 授权与策略检查** — `docs/compliance/grant-policy.md`
  - 追溯 SPEC 4.3-5/6、5.6.1-6 · DESIGN 2.7.8-5/6
  - DoD：撤销授权后重启/升级/新建会话均不恢复外发；篡改签名策略包被拒绝加载；组织强制项无法被覆盖
- [ ] **74.4 合规映射复核与签署** — `docs/compliance/legal-signoff.md`
  - 追溯 SPEC 1.4-7、2.7.7 · DESIGN 2.7.7
  - DoD：告知同意、最小必要、存储限制、数据主体权利、跨境传输五项映射逐条确认；法务签署（非法律意见声明保留）；残留风险登记

### 75. 数据兼容与迁移回归

- [ ] **75.1 旧数据可读（6 个月兼容期）** — `tests/compat/old_data.rs`
  - 追溯 SPEC 4.5-8 · DESIGN 2.3.1-5
  - DoD：v0 样本库在新版本可完整读取（会话/片段/要点/卡片/审计）；未知枚举值降级不 panic；只读升级不改原字段
- [ ] **75.2 schema 迁移回放** — `tests/compat/migration.rs`
  - 追溯 SPEC 4.5-8 · DESIGN 2.3.1-5
  - DoD：升级→使用→降级读旧版本 三段回放通过；迁移失败可回滚（备份机制）；迁移耗时（10 万片段）≤60s
- [ ] **75.3 跨架构数据一致性** — `tests/compat/cross_arch.rs`
  - 追溯 SPEC 4.5-1 · DESIGN 2.4.1
  - DoD：Win x64/ARM64、mac arm64/x64 四架构导出的 JSON 纪要字段一致；SQLCipher 库不可跨架构共用时明确报错而不损坏

---

## 批次 B10 · 打包、发布与交付（W21–W22）

**目标**：产出可安装、可公证、可审计的交付物，并建立发布与平台适配 SOP。
**前置**：B9（G9 通过）。
**⛔ 出口门禁 G10（V1 发布）**：SBOM + 法务签署；三平台版本矩阵全部验证；灰度 2 周无 P0；性能与隐私门禁全绿。
**并行**：76 与 77 并行；79 可与 78 并行准备。

### 76. Windows 打包与签名

- [ ] **76.1 MSI / NSIS 双产物（x64 + ARM64）** — `app/src-tauri/tauri.conf.json`、`scripts/build-win.ps1`
  - 追溯 SPEC 4.5-1 · DESIGN 2.4.1（Tauri Bundler）
  - DoD：X64 与 ARM64 各产出 MSI 与 NSIS 安装包；安装/卸载/升级/降级路径均验证；升级保留用户配置与数据
- [ ] **76.2 代码签名与 SmartScreen** — `docs/release/win-signing.md`
  - 追溯 SPEC 4.5-1 · DESIGN 2.4.1
  - DoD：全部二进制（含 ASR 库/模型）签名；签名链可验证；SmartScreen 首次安装无"未知发布者"
- [ ] **76.3 WebView2 运行时策略** — `docs/release/webview2.md`
  - 追溯 SPEC 4.5-1 · DESIGN 2.4.1（核实提示）
  - DoD：明确"系统自带/引导安装/随包分发"三选一并记录条款依据；无 WebView2 时的引导可用；离线安装包体积记录
- [ ] **76.4 安装目录与数据目录规范** — `docs/release/win-layout.md`
  - 追溯 SPEC 4.3-3 · DESIGN 2.7.3
  - DoD：程序与数据分离；数据目录权限为当前用户；卸载默认不删会话数据（需显式勾选）；目录内无明文密钥

### 77. macOS 打包与公证

- [ ] **77.1 Universal DMG（arm64 + x86_64）** — `scripts/build-mac.sh`
  - 追溯 SPEC 4.5-1 · DESIGN 2.4.1
  - DoD：Universal 二进制（或双架构分发包）；DMG 可挂载安装；最低系统版本声明 13.0；Apple Silicon 与 Intel 真机安装运行通过
- [ ] **77.2 公证（notarization）与 Gatekeeper** — `docs/release/mac-notarize.md`
  - 追溯 SPEC 4.5-1 · DESIGN 2.4.1
  - DoD：公证成功并可 staple；首次打开无"无法验证开发者"；Hardened Runtime 开启且音频/无障碍功能不受影响
- [ ] **77.3 权限说明与首次引导** — `app/src-tauri/Info.plist`、`app/src/features/onboarding/permissions.tsx`
  - 追溯 SPEC 4.3-1、5.8.1-4 · DESIGN 2.5.4
  - DoD：`NSMicrophoneUsageDescription`、屏幕录制与辅助功能引导文案齐备；首次启动引导覆盖三类权限；拒绝后可从设置页重新引导
- [ ] **77.4 不捆绑 GPL 组件核验** — `docs/release/mac-gpl-check.md`
  - 追溯 SPEC 1.4-3 · DESIGN 1.2.1-2
  - DoD：安装包内无 BlackHole 等 GPL 虚拟声卡；SBOM 中无 GPL/AGPL 直接链接；用户自装兼容说明仅出现在文档

### 78. 发布门禁与灰度 ⛔ GATE

- [ ] **78.1 发布检查清单** — `docs/release/release-checklist.md`
  - 追溯 SPEC 4.3-7、4.4-1、4.5-2 · DESIGN 2.9、2.7.8
  - DoD：SBOM、法务签署、隐私检查、性能门禁、三平台矩阵、兼容回归、P0/P1 缺陷清零七项逐项勾选并附证据链接
- [ ] **78.2 灰度发布与回滚** — `docs/release/rollout-plan.md`
  - 追溯 SPEC 4.2-1 · DESIGN 2.8.2-M3（灰度 2 周无 P0）
  - DoD：定义灰度分档（内测→5%→25%→100%）；崩溃率/采纳率/关停率监控阈值；回滚步骤可在 30 分钟内执行并演练一次
- [ ] **78.3 上线后质量观测** — `docs/release/post-launch.md`
  - 追溯 SPEC 4.4-4 · DESIGN 2.6.6
  - DoD：10 项指标上线后可观测（用户显式导出诊断包后才外传）；阈值告警通道就绪；每周抽检流程已排期

### 79. 交付文档与平台适配 SOP

- [ ] **79.1 用户与管理员文档** — `docs/user/`
  - 追溯 SPEC 3.1、5.7 · DESIGN 2.7.7
  - DoD：安装与权限引导、开始会话流程、检索源配置示例（不含内置源暗示）、隐私说明、常见错误码解释齐备；截图与当前版本 UI 一致
- [ ] **79.2 平台适配 SOP（15 工作日）** — `docs/ops/platform-adaptation-sop.md`
  - 追溯 SPEC 4.5-2、4.4-5 · DESIGN 2.8.1-R1
  - DoD：平台新主版本发布后的适配流程（实测→选择器更新→矩阵更新→回归→灰度）可执行；明确"15 个工作日内完成适配或走回退通道"；选择器热更新通道有演练记录
- [ ] **79.3 诊断与故障排查手册** — `docs/ops/troubleshooting.md`
  - 追溯 SPEC 4.4-5 · DESIGN 2.6.6、2.7.3-L4
  - DoD：按错误码与平台分类给出排查步骤；诊断包字段说明；不得包含会泄露原文的采集建议
- [ ] **79.4 检索源接入示例与适配器开发指南** 🧩 SPI — `docs/dev/search-provider-guide.md`
  - 追溯 SPEC 4.5-6、5.3.1-7c · DESIGN 2.2.2-D
  - DoD：给出 SPI 实现最小示例（自建服务 + 开源引擎两类）；说明 `spiVersion`、`authRef`、超时与限额约束；示例中不出现"官方推荐/内置默认"表述

---

## 附录 A · 需求覆盖矩阵

| SPEC 章节 | 主任务 | SPEC 章节 | 主任务 |
|---|---|---|---|
| 4.1 性能（时延/内存/CPU/容量） | 3.1、30.3、32.4、40.3、40.4、47.4、72.1–72.4 | 5.4 会中提示与推送 | 40.1、40.2、41.1–41.5、44.1–44.4、45.1–45.4、46.1–46.3 |
| 4.2 可靠性（可用性/持久化/降级） | 33.1–33.4、35.1、42.1–42.4、73.1–73.3 | 5.5 纪要与回顾 | 55.1–55.6、56.1–56.3、57.1–57.4、58.1–58.3、54.1–54.3 |
| 4.3 安全性（告知/本地/加密/脱敏/授权/审计/删除） | 5.1–5.4、27.1–27.3、37.1–37.5、51.1–51.6、52.1–52.4、53.1–53.4、54.3–54.4 | 5.6 隐私与合规控制 | 27.1–27.3、37.4、50.1–50.4、51.5、53.1–53.4、74.1–74.4 |
| 4.4 可维护性（指标/日志/追溯/告警/平台定位） | 6.3、6.4、40.6、52.3、63.1–63.4、78.3 | 5.7 配置、个性化与质量反馈 | 59.1–59.3、60.1–60.4、61.1–61.3、62.1–62.4、63.1–63.4、64.1–64.3 |
| 4.5 兼容性（平台/架构/导出/检索开放/兼容） | 1.1、1.4、21.1、34.4、34.5、57.1、75.1–75.3、76.1、77.1 | 5.8 指定平台对接 | 9.1–9.4、10.1–10.4、11.1–11.4、18.1–18.4、19.1–19.4、20.1–20.4、22.1–22.4、67.1–67.3、68.1–68.3、69.1–69.3 |
| 5.1 会话监听与采集 | 15.1–15.4、16.1–16.4、23.1–23.4、24.1–24.5、26.1–26.5、27.1–27.3、28.1–28.4 | 6.1 会话数据约束 | 3.2、5.3、26.1、61.2、54.5 |
| 5.2 要点捕捉与滚动摘要 | 29.1–29.3、30.1–30.6、31.1–31.3、33.1–33.4、47.1–47.5、65.1 | 6.2 发言片段约束 | 3.1、5.3、24.3、28.1–28.4 |
| 5.3 联网信息补全（全自动主路径） | 32.1–32.6、34.1–34.5、35.1–35.5、36.1–36.5、37.1–37.5、38.1–38.7、39.1–39.4、40.1–40.6、41.1–41.6、42.1–42.4、43.1–43.3 | 6.3 要点约束 / 6.4 提示卡约束 | 3.1、30.2–30.4、31.1–31.2、38.1–38.2、39.4、52.3 |
| 5.1.2 / 5.2.2 / 5.3.2 交互流程 | 26.1–26.5、30.1、40.3、44.1–44.4 | 6.5 引用约束 / 6.6 术语 / 6.7 策略 | 3.1、38.3、55.3、59.3、50.1–50.4、54.5 |
| 5.1.3 / 5.2.3 / 5.3.3 异常场景 | 15.3–15.4、16.3、21.4、28.2、29.3、33.3–33.4、35.1、36.4、39.3、42.4、73.1 | 6.8 审计 / 6.9 检索源 / 6.10 平台配置 | 6.1–6.2、34.1、52.1–52.4、60.1–60.4、61.1–61.3、22.2 |
| 5.4.3 / 5.5.3 / 5.7.3 异常场景 | 41.2、41.4、45.4、55.5、57.2–57.3、58.3、60.2、63.2 | 1.4 职责边界（不注入/不内置源/不采凭据） | 2.3、4.3、22.1、22.4、34.2、36.2、77.4、79.4 |

## 附录 B · 风险登记与任务对应

| 风险（DESIGN 2.8.1） | 缓解任务 | 预警指标落点 |
|---|---|---|
| R1 平台版本升级致控件漂移 | 17.1（选择器热更新）、21.x（OCR 兜底）、23.2（5s 回退）、79.2（15 工作日 SOP） | 6.4（分平台字幕失效率） |
| R2 飞书 Electron 自绘读不到 | 10.2/10.3、19.2（OCR 主路径）、21.4（OCR 回归集） | 48.4 诊断面板、68.1 矩阵 |
| R3 macOS SCK 不稳定/权限拒绝高 | 8.3、16.2/16.3、71.2 | 16.3 重建计数 |
| R4 Win10 <2004 无法进程级环回 | 7.1/7.3、15.2（回退链）、61.2（能力差异） | 48.1 通道状态卡 |
| R5 低端设备内存/CPU 超限 | 25.3/25.4、72.2、78.2 | 25.4 峰值保护器日志 |
| R6 自动提示误报导致关停 | 40.1/40.2、41.1–41.5、63.2/63.4、78.3 | 63.1 采纳率与误触发率 |
| R7 检索慢导致 6s 不可达 | 35.1–35.3、36.5、38.6（分层出卡）、72.1 | 72.1 分阶段时延 |
| R8 幻觉/错误结论 | 38.2/38.5、39.1/39.3、63.3（抽检） | 63.3 准确率 |
| R9 AGPL/GPL 许可污染 | 2.1–2.3、77.4、78.1 | CI cargo-deny |
| R10 平台 ToS 限制采集 | 22.4、2.3、9.4/10.4/11.4 结论 | 74.4 合规映射 |
| R11 说话人缺失/重叠 | 28.1、24.4、30.4 | 63.1 待确认比例 |
| R12 开放源 SSRF/内网探测 | 35.5、60.1（配置校验） | 35.5 出站拒绝计数 |
| R13 密钥丢失/轮换失败 | 5.2、75.2（迁移备份） | 启动失败日志（脱敏） |
| R14 多平台误绑定 | 22.3、20.4、26.2 守卫 | 53.4 越权/异常审计 |
| R15 提示风暴与 UI 卡顿 | 41.2/41.3/41.6、44.4、72.4 | 41.6 压测报告 |

## 附录 C · 范围裁剪顺序（进度承压时）

| 优先级 | 能力 | 可裁性 | 对应任务 |
|---|---|---|---|
| P0 | 三平台对接（至少音频）+ 自动要点 + 全自动提示 + 加密存储 + 审计 + 采集告知 | **不可裁** | 15/16/18/19/20、30、40、5、6、27 |
| P1 | 字幕全平台可用、双通道合并、投屏保护、紧急遮蔽、授权与策略 | 不可裁（可降级） | 17、24、45、51、50 |
| P2 | 多源优先级调度（≥2 源） | 可简化 | 34.5/35.3 可降为"单源 + 缓存"，但 SPI 与配置必须保留 |
| P3 | PDF 导出、知识库/日历归档 | 可裁 | 57.4、56.3 |
| P4 | 周度质量报表可视化 | 可简化 | 63.3 保留指标采集与告警，界面可简化 |
| P5 | 本地 LLM 降级 | 可裁 | 30.5 可退化为"仅规则"，断网时"仅要点"仍满足 SPEC 4.2-5 |

## 附录 D · 验收执行口径（可直接转成脚本）

1. **时延**：真实会议回放 + 打点埋点计算 p95/p99，单场景样本 ≥200 次；**不得**用合成文本替代字幕通道实测。
2. **资源**：低端基准机（4 核 / 8GB / 集显）与主流机双档跑 2 小时，记录均值与 10s 峰值。
3. **可靠性**：故障注入矩阵（断网、源超时、ASR 失败、字幕关闭、库只读、审计不可写、脱敏不可用）逐项验证隔离效果。
4. **隐私**：抓包 + 日志扫描 + 密钥扫描 + 删除残留扫描四件套，作为发布硬门禁。
5. **平台兼容**：三平台 × 版本矩阵 ×（字幕开/关/共享屏幕）×（Win/mac）最小回归集，每平台 ≥12 组合。

## 附录 E · 假设与待确认项

| # | 假设/待确认 | 影响任务 | 关闭时点 |
|---|---|---|---|
| 1 | 三平台进程名与窗口结构以实测为准（design 2.5.2 标注"需实测确认"） | 18.1、19.1、20.1、22.1 | M0（任务 14） |
| 2 | 腾讯会议/飞书版本支持矩阵的具体版本号需 M0 实测后填定 | 61.2、67.1、68.1 | M0 |
| 3 | sherpa-onnx 各模型权重许可证需逐个核实（代码 Apache-2.0 ≠ 模型许可） | 2.1、2.3、12.1 | M0 前法务签署 |
| 4 | 云端 LLM 的数据不训练条款需在采购/配置阶段确认 | 13.1、74.4 | M2 前 |
| 5 | 是否允许用户显式"允许内网检索源"（默认拒绝）需产品确认 | 35.5、60.1 | M2 |
| 6 | 纪要归档目标（知识库/日历）的具体协议与鉴权方式未定 | 56.3 | M2 末 |
| 7 | 组织策略签名公钥的分发与轮换流程未定 | 50.1 | M2 |
| 8 | 提示采纳率基线需在灰度期建立（design 2.6.6 标注"基线化"） | 63.1、78.3 | 灰度第 1 周 |
| 9 | 三平台字幕延迟在 M0 前不得对外宣称具体数值（design 2.5.6 注） | 9.4、10.4、11.4、48.2 | M0 |
| 10 | 低端基准机的具体型号与最低配置声明需产品与 QA 共同定 | 72.2、78.1 | M2 末 |

## 附录 F · 里程碑门禁速查

| 门禁 | 时点 | 通过条件 | 关联任务 |
|---|---|---|---|
| **G0** | W2 末 | 空壳可跑、契约冻结、许可与分层 CI 生效、密钥扫描 0 | 1–6 |
| **G1（M0）** | W5 末 | 三平台各 ≥1 条字幕路径 + Win/mac 音频跑通 + 指标可达 | 7–14 |
| **G2（M1 前半）** | W11 末 | 三平台建会话稳定 30min；通道状态 5s 更新；5s 回退 | 15–27 |
| **G3（M1）** | W11 末 | 要点 p95 ≤1.5s/≤3s；约束 100% 合规；60s 恢复 | 28–33 |
| **G4（M2 核心）** | W16 末 | 自动提示 p95 ≤6s/p99 ≤10s；误触发 ≤30%；零源模式可用 | 34–43 |
| **G5** | W17 末 | 0 次抢焦点；投屏 1s 隐藏；遮蔽 p99 ≤1s；交互 ≤200ms | 44–49 |
| **G6** | W17 末 | 隐私清单 1–6 通过；授权三入口不恢复；删除 ≤1min 残留 0 | 50–54 |
| **G7** | W19 末 | 纪要 ≤30s；三格式一致；未确认 0 归档 | 55–58 |
| **G8** | W19 末 | 5 源热生效；冲突被拒；10 指标可查 | 59–64 |
| **G9（M3 前）** | W21 末 | 性能/可靠性/隐私/平台矩阵四类门禁全绿；P0/P1 清零 | 65–75 |
| **G10（V1）** | W22 末 | SBOM + 法务签署 + 灰度 2 周无 P0 | 76–79 |

---

**文档结束**。本清单共 **11 个批次 / 79 个主任务**，覆盖 `spec.md` 4.1–4.5、5.1–5.8、6.1–6.10 全部章节与 `design.md` 2.1–2.9 全部设计章节；后续编码阶段可按批次顺序执行，批次内按标注并行度分配。