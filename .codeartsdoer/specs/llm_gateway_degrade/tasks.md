# LLM 网关降级 · 编码任务清单

> **追溯**：`.codeartsdoer/specs/llm_gateway_degrade/spec.md`（下称 SPEC，含 AC-01~AC-27、8.3、第 11 章）、`.codeartsdoer/specs/llm_gateway_degrade/design.md`（下称 DESIGN v1.1，1976 行）。
> **对应上级任务**：`docs/sdd/tasks.md` 任务 **13.1**（结构化生成）、**13.2**（降级与兜底），并为其下游 **13.4**（LLM 基线）准备可复核数据入口。
> **当前基线**：B0 已交付（Rust 测试 80 个全绿，clippy / fmt / cargo deny / layering / secrets 全绿）；B1 仅 12.2 完成；`crates/infrastructure` 尚无 `llm` 模块，`crates/infrastructure/tests/` 尚不存在。
> **编号规则**：主任务 `L1`…`L13`，子任务 `L1.1`…`L13.4`；两级，不再向下嵌套。所有子任务均以 `- [ ]` 呈现，可独立勾选。
> **标记约定**：`★` = 关键路径；`⟂` = 可并行；`⛔` = 外部阻塞；`〔D1〕` = 落地时需确认的最小设计偏差。

---

## 0. 总览与约束

### 0.1 阶段划分

| 阶段 | 任务范围 | 覆盖验收项 | 是否可立即执行 | 结束标志 |
|---|---|---|---|---|
| **A：离线主体** | L1~L8 | AC-01~AC-18（AC-18 仅登记"待实测"） | 是（无外部输入） | 全门禁绿 + 既有 80 测试不减 |
| **B：联调载体** | L9~L12 | AC-19、AC-26（静态面）、AC-27（机制面）；为 AC-20~AC-25 交付载体 | 是（零新增依赖、零公网） | `llm_live_offline` 全绿；`llm_live` 可编译且默认跳过 |
| **C：真实实测** | L13 | AC-20~AC-25 的实测值与 AC-26 运行期抓取 | **否**（需许可签字 + M1~M5） | 记录可复核并移交 13.4 |
| **D：服务商锁定 + TLS + 阶段 C 实测** | L14~L22 | AC-28~AC-32 全部；AC-20~AC-27 的执行入口与记录 v2 | **离线部分：是**；实测部分：**否**（C1~C5 / M6 / M7） | G-SECRET-DOC、G-CAND、G-TEST-TLS 全绿 + 既有 195 测试不回退 |

### 0.1.1 阶段状态（v1.2 增补，**不改上表既有行**）

| 阶段 | 状态 | 证据 / 说明 |
|---|---|---|
| **A：离线主体** | **已完成（基线，不重开）** | L1~L8 全部勾选；`crates/infrastructure/src/llm/{schema,transport,degrade,fallback,gateway,mod}.rs` 与 4 个集成测试已落地 |
| **B：联调载体** | **已完成（基线，不重开）** | L9~L12 全部勾选；`live/{gate,config,stats,record}.rs`、`tests/llm_{live_offline,live}.rs`、两个脚本已落地；`llm-https` 当时为空 feature |
| **C：真实实测** | **未执行（⛔）** | L13.1~L13.4 保持未勾选；其**执行前置**由本版 L19.1/L21.2/L22 细化（见附录 G），**DoD 文字不重写** |
| **D：服务商锁定 + TLS + 阶段 C 实测** | **本版新增** | L14~L22；离线判定可立即执行，实测部分被 C1~C5/M6/M7 阻塞 |

> **不重开声明（强制）**：阶段 D 不得回改 L1~L13 任何子任务的标题、DoD 文字或勾选状态。阶段 D 与既有条目的**取代/前置关系**只在附录 G 登记；L12.2 的"空 feature 依赖数组为空"断言被 L16.1 取代、L13.3/L13.4 的执行前置被 L22 更新，均以"登记"方式表达，不以"重写条目"方式表达。

> **硬边界**：阶段 A 与阶段 B 的任何结果都**不得**用于宣称 13.1/13.2 已完成真实端点验证，也**不得**替代 AC-01~AC-18 的 mock 判定（SPEC 7.2-11、8.3 首段、DESIGN 2.9.5）。

### 0.2 关键路径与并行矩阵

```
★ 关键路径（最长链）：
L1.1 → L1.2 → L1.3 → L1.4 → L1.5 → L3.1 → L3.2 → L6.2 → L6.3 → L7.1 → L7.2 → L8.2

⟂ 并行簇：
  P1（L1.5 完成后）：L2.*（Schema 线） ‖ L3.*（传输线） ‖ L5.*（兜底线）
  P2（L3.1 完成后）：L4.*（降级决策线）可与 L2/L5 并行
  P3（L6.5 完成后）：L7.1 ‖ L7.2 ‖ L7.3（三个集成测试文件互不共享可写状态）
  P4（L9.1 完成后）：L10.1 ‖ L11.1（用合成样本先验统计与记录口径）
  P5：L11.3 ‖ L11.4（两个脚本互不依赖）
串行汇聚点：L6（依赖 L2/L3/L4/L5 全部）、L8.2（依赖 L1~L7）、L13（依赖 L9~L12）
```

**不建议并行**：L1.2 / L1.3 / L1.4 同写 `model/llm.rs`，须串行编辑；L6.2 与 L6.3 同写 `gateway.rs`，须串行。

### 0.3 文件清单（以 DESIGN 2.5.3 最终清单为准）

#### 新增 23 个文件

| # | 文件 | 承载职责 | 阶段 | AC | 任务 |
|---|---|---|---|---|---|
| 1 | `crates/domain/src/model/llm.rs` | 任务/请求/结果/事件/分类/观测点领域类型 | A | 01/02/10 | L1.2~L1.4 |
| 2 | `crates/domain/src/port/llm.rs` | `StructuredLlmGateway`、`CancelToken`、`StreamCollector`、`LlmObserver` | A | 02/03 | L1.5 |
| 3 | `crates/infrastructure/src/llm/mod.rs` | 模块注册、装配、配置快照、观测适配 | A | 02/15 | L6.1/L6.4/L6.5 |
| 4 | `crates/infrastructure/src/llm/gateway.rs` | 一次性与流式编排、SSE 解析、首 token、终止唯一 | A | 02/03/14 | L6.2/L6.3 |
| 5 | `crates/infrastructure/src/llm/schema.rs` | 四类 Schema 派生、严格解码、跨字段规则 | A | 01/07/13 | L2.1 |
| 6 | `crates/infrastructure/src/llm/transport.rs` | `HttpTransport` SPI、`Http1Codec`、回环实现、错误归一化 | A | 04/09 | L3.1/L3.2 |
| 7 | `crates/infrastructure/src/llm/degrade.rs` | 分类、重试、退避、截止时间、冷却、并发 | A | 04/05/06/08/09 | L4.1~L4.4 |
| 8 | `crates/infrastructure/src/llm/fallback.rs` | 四类规则兜底 | A | 09/10/11/12/13 | L5.1/L5.2 |
| 9 | `crates/infrastructure/tests/llm_schema.rs` | Schema 与类型一致性 | A | 01 | L2.2 |
| 10 | `crates/infrastructure/tests/llm_gateway_mock.rs` | 一次性/流式/不阻塞采集 | A | 02/03/14 | L7.1 |
| 11 | `crates/infrastructure/tests/llm_degrade_inject.rs` | 故障注入与降级 | A | 04~13 | L7.2 |
| 12 | `crates/infrastructure/tests/llm_logging_redaction.rs` | 日志/审计/脱敏 | A | 15 | L7.3 |
| 13 | `crates/infrastructure/tests/support/mod.rs` | 测试脚手架（mock 端点 + 替身 + 断言工具） | A | 02~15 | L3.3 |
| 14 | `crates/infrastructure/src/llm/live/mod.rs` | 联调装配 `assemble_live()` | B | 19 | L9.1 |
| 15 | `crates/infrastructure/src/llm/live/gate.rs` | `LiveGate` / `EnvView` / `SkipReason` | B | 19 | L9.1 |
| 16 | `crates/infrastructure/src/llm/live/config.rs` | `LiveRunConfig` / `SecretString` / URL 收敛为 host | B | 19/26 | L9.2 |
| 17 | `crates/infrastructure/src/llm/live/stats.rs` | p50/p95、原生成功率、六类分布 + 未归类 | B | 20/21/22 | L10.1 |
| 18 | `crates/infrastructure/src/llm/live/record.rs` | `run.json`/`samples.jsonl`/`summary.md` 原子写与三态 | B | 23/26/27 | L10.2 |
| 19 | `crates/infrastructure/tests/llm_live_offline.rs` | 默认路径：开关矩阵、统计口径、掩码、记录完备性 | B | 19/26/27 | L11.1 |
| 20 | `crates/infrastructure/tests/llm_live.rs` | `#[ignore]` 真实采样目标 | B（编）/ C（跑） | 20~25 | L11.2/L13.3 |
| 21 | `scripts/run-llm-live.sh` | 前置门检查、串行执行、四泄漏面抓取 | B | 19/26 | L11.3 |
| 22 | `scripts/llm-live-recompute.mjs` | 由 `samples.jsonl` 独立重算 p50/p95 | B | 27 | L11.4 |
| 23 | `crates/infrastructure/src/llm/transport_https.rs` | TLS 传输（`feature="llm-https"`，复用 `Http1Codec`） | C ⛔ | 20~25 | L13.1 |

#### 修改 8 个文件 / 11 处（去重后）

| # | 文件 | 改动内容 | 阶段 | 本清单是否执行 |
|---|---|---|---|---|
| 1 | `crates/domain/src/model/mod.rs` | `pub mod llm;` + 导出 | A | 是（L1.1） |
| 2 | `crates/domain/src/port/mod.rs` | `pub mod llm;` + 导出新端口 | A | 是（L1.5） |
| 3 | `crates/infrastructure/src/lib.rs` | `pub mod llm;` | A | 是（L6.1） |
| 4 | `crates/infrastructure/Cargo.toml` | Phase A：**空** feature `llm-https = []`；Phase C 改 `["dep:native-tls"]` | A / C | 是（L6.1）；C 阻塞（L13.2） |
| 5 | `crates/domain/Cargo.toml` | 增 `schemars = "1"`（精确 1.2.3） | A | 是（L1.1） |
| 6 | `docs/compliance/license-review.md` | 登记 `schemars 1.2.3` + "HTTPS 挂起"；B 阶段追加 TLS 条目 | A / B | 是（L8.1、L12.1） |
| 7 | `docs/progress/b1-status.md` | 13.1/13.2 状态、证据、待实测三态 | A / B | 是（L8.3） |
| 8 | `docs/sdd/tasks.md` | 仅勾选 13.1/13.2 并附 DoD 证据 | A | **否**（见下） |

> **关于 `docs/sdd/tasks.md`**：DESIGN 2.5.3 将其列为修改项，但本次执行约束明确"不改 `docs/sdd/*.md`"。因此本清单**不创建该任务**，仅登记为"变更外移交项"：门禁全绿且 user 明确授权后，由发布责任人手工勾选。同理 `docs/sdd/spec.md`、`docs/sdd/design.md` 全程禁改。

### 0.4 验证门禁（GATE，每个阶段收口必跑）

| 代号 | 命令 | 通过条件 |
|---|---|---|
| G-FMT | `cargo fmt --all --check` | 退出码 0、无输出差异 |
| G-CLIPPY | `cargo clippy --workspace --all-targets -- -D warnings` | 退出码 0、零 warning |
| G-CLIPPY-F | `cargo clippy -p meeting-copilot-infrastructure --all-targets --features llm-https -- -D warnings` | 退出码 0；Phase A 空 feature 不得触发 `unexpected_cfgs` |
| G-TEST | `cargo test --workspace` | `failed=0`；`passed` 数量 **≥ 80 + 本阶段新增下限** |
| G-LAYER | `bash scripts/check-layering.sh` | 退出码 0 |
| G-SECRET | `bash scripts/scan-secrets.sh` | 退出码 0 |
| G-DENY | `cargo deny check licenses bans sources advisories` | 退出码 0（bans 的 multiple-versions 只允许 warn） |
| G-COUNT | `cargo test --workspace -- --list \| grep -c ": test"` | ≥ 80 + 新增下限（用例只增不减，AC-17） |

**新增测试数量口径（下限，非预测值）**

| 阶段 | 文件 | 下限 | 依据 |
|---|---|---|---|
| A | `llm_schema.rs` | ≥ 5 个用例 | DESIGN 2.4.2 AC-01 的三段判定 + 四类样例正/反各一组 |
| A | `llm_gateway_mock.rs` | ≥ 3 个用例 | DESIGN 2.4.2 具名 `complete_ok`/`stream_ok`/`non_blocking_capture` |
| A | `llm_degrade_inject.rs` | ≥ 12 个用例 | DESIGN 2.4.2 具名 10 个（AC-04~AC-13）+ AC-06 的两分支（声明 5 s/剩余 2 s；未声明） |
| A | `llm_logging_redaction.rs` | ≥ 2 个用例 | AC-15 的日志面 + 审计面 |
| A | 各模块内联单测（domain/degrade/fallback/schema/transport） | ≥ 10 个 | DESIGN 2.4.1 分层职责、2.4.3 的 6 类验证手段 |
| A 合计（默认执行） | — | **≥ 32**，故 G-TEST 的 `passed ≥ 112` | 上表求和 |
| B | `llm_live_offline.rs` | ≥ 7 个用例 | DESIGN 2.9.2 具名 7 个（AC-19 三个 + AC-26 三个 + AC-27 一个） |
| B | `llm_live.rs`（`#[ignore]`） | ≥ 7 个用例 | DESIGN 2.9.2 具名：AC-19 跳过面 1 + AC-20~AC-25 各 1 |
| B 合计（默认执行） | — | **≥ 7**，故 G-TEST 的 `passed ≥ 119` | 忽略用例不计入默认执行数 |

> 说明：上表是**下限口径**，用于"只增不减"的机器判定；实际条数以 `cargo test --workspace` 输出为准，不得低于下限。

### 0.5 禁止事项（违反即视为任务失败）

1. 禁改 `crates/application/**`（含测试装配之外的任何调整），不得引入网络依赖。
2. 禁改 `crates/platform-win/**`、`crates/platform-mac/**`、`crates/ui-bridge/**`、`app/src-tauri/**` 与任何 TS 绑定文件；不新增跨端 DTO / IPC 契约。
3. 禁改 `docs/sdd/spec.md`、`docs/sdd/design.md`、`docs/sdd/tasks.md`（任务勾选亦需单独授权）。
4. 禁改 `crates/domain/src/port/understand.rs` 中的 `LlmGateway`/`LlmRequest`/`LlmResponse` 签名；新能力一律并存新增。
5. 禁改 `crates/domain/src/model/enums.rs`（审计动作复用既有 `AuditAction::OutboundQuery`，不新增枚举变体）。
6. 禁改 `crates/infrastructure/src/metrics/mod.rs` 的 `Metric` 枚举与 10 项断言（新指标走独立 `LlmMetric`）。
7. 任何文件、夹具、脚本、日志、示例中不得出现真实凭据值或其编号；测试夹具只用明显假值（如 `sk-test-0000`）。
8. 真实联调默认关闭：CI 不设 `LLM_LIVE`、不注入 `LLM_API_KEY`；未开启时**不得构造任何传输对象**。
9. 不得把实测产物写入 `docs/`、`crates/` 或仓库根；只允许 `.tmp/llm-live/<run-id>/`（`.gitignore:17` 已覆盖）。
10. 不得把 mock 数值填入"实测"列，不得把"未触发"写成"通过"，不得删改/择优选样原始样本。
11. 阶段 A/B 不得声明或修改 `native-tls` 依赖（许可签字前，feature 与依赖均不得写入 `Cargo.toml`）。
12. 网关注释与代码中不得出现具体服务商域名、模型家族名、区域或价格信息。

### 0.6 阻塞项登记（M1~M5 与许可门）

| 编号 | 阻塞内容 | 登记位置 | 未补齐时的行为 | 补齐后的执行入口 |
|---|---|---|---|---|
| M1 | Base URL（`LLM_BASE_URL`） | L13.3 的 `Skip(EndpointMissing)`；`docs/progress/b1-status.md` 标"未实测（端点未配置）" | fail-closed，零外发 | 当前 shell 设 `LLM_BASE_URL` → `bash scripts/run-llm-live.sh` |
| M2 | 模型名（`LLM_MODEL`） | L13.3 的 `Skip(ModelMissing)`；同上 | 同上 | 当前 shell 设 `LLM_MODEL` → 同上 |
| M3 | 域名外发白名单授权 | L9.2 的 `AllowlistMissing` + L12.1 许可记录 | 白名单为空即拒绝一切外发 | 配置者确认后由 host 派生白名单，无需改代码 |
| M4 | 采样预算 / 调用次数上限 | L9.2 的 `call_budget` 校验 + L10.1 的 `n<30` / `n<10` 口径 | 按"样本不足"降级表述，不输出 p95 结论 | 设 `LLM_LIVE_SAMPLES` → 同上 |
| M5 | 凭据使用授权与轮换责任人 | L9.1 的 `Skip(CredentialMissing)`；AC-26 抓取面 | 不开启真实联调 | 受限 shell 注入 `LLM_API_KEY` → 同上 |
| P9 | `native-tls` 许可签字（安全 + 法务） | L12.1 新增条目；L13.1/L13.2 标 ⛔ | `llm-https` 保持空 feature；`Skip(TransportUnavailable)` | 签字后执行 L13.1/L13.2，再跑 `--features llm-https` |
| P-合规 | 云端数据不训练条款确认（`docs/sdd/tasks.md:1458`） | `docs/progress/b1-status.md` 标"未实测（合规未确认）" | 不执行真实采样 | 由配置者确认后解除 |

**登记纪律**：以上阻塞项不得写入 `docs/sdd/*.md`，只登记在 `docs/progress/b1-status.md` 与 `.tmp/llm-live/` 产物中；也不得新增 issue 类文件。

### 0.7 阶段 D 总览（v1.2 增补）

> **源文档**：SPEC 8.4（AC-28~AC-32）、SPEC 11.9（11.9.1~11.9.8）、DESIGN 2.10~2.13（v1.2）。本节只登记阶段 D 的组/依赖/门禁/阻塞，不改 0.1~0.6 的任何既有条目。
> **编号映射**：阶段 D 组号 `D1`~`D9` 与主任务 `L14`~`L22` 一一对应，子任务 `D1.1 ≡ L14.1`。正文一律以 `L14.1` 编号，保持 `L1`~`L13` 之后的连续性。

#### 0.7.1 组与主任务映射

| 组 | 主任务 | 主题 | 覆盖 AC | DESIGN 小节 |
|---|---|---|---|---|
| **D1** | L14 | 服务商锁定运行门（policy + admit） | AC-28、AC-29、AC-30 | 2.10.1、2.10.4~2.10.7、2.10.9、2.10.10 |
| **D2** | L15 | 预算账本、触顶与错误通路 | AC-31 | 2.10.8、2.13.2-AC-31 |
| **D3** | L16 | TLS 传输与 feature 真实依赖 | AC-30、AC-17 | 2.11.1~2.11.7 |
| **D4** | L17 | 六载体泄漏扫描与候选标注脚本 | AC-32、AC-29 | 2.13.4、2.10.6 |
| **D5** | L18 | 记录契约 v2（含数据外发口径与接受日期） | AC-27、AC-31、AC-32 | 2.12.4、2.13.4 |
| **D6** | L19 | C1~C3 fail-closed 与单一执行入口 | AC-28~AC-31、C1~C3 | 2.12.1~2.12.5 |
| **D7** | L20 | 许可登记与治理状态 | AC-17、AC-30 | 2.11.1、2.5.4 |
| **D8** | L21 | 兼容修改、门禁收口与禁止项审计 | AC-17 | 2.13.5 |
| **D9** | L22 | 阶段 C 实测（⛔ 输入阻塞） | AC-20~AC-27 | 2.12.2、2.9.4、2.13.6 |

#### 0.7.2 关键路径与并行矩阵

```
★ 关键路径（阶段 D 主链，8 跳）：
L14.1 → L14.2 → L19.1 → L19.2 → L19.3 → L19.4 → L21.2 → L21.3

★ 次长链（不进入主链，但 L19.3 前需 L16.1/L17.3 完成）：
TLS 链：L16.1 → L16.2 → L16.3 → L16.4
预算链：L15.1 → L15.2 → L15.3
扫描链：L17.1 → L17.3（L14.2 之后并入 L19.3）

⟂ 并行簇：
  Q1（无前置，立即可做）：L14.1（policy） ‖ L15.1（budget） ‖ L16.1（Cargo feature） ‖ L17.1（文档扫描） ‖ L18.1（记录 v2 骨架）
  Q2（L17.1 完成后）：L17.2（候选标注） ‖ L17.3（argv 泄漏修正）
  Q3（文件互不共享）：L14.3（需 L14.2） ‖ L16.4（需 L16.3） ‖ L18.2（需 L18.1） ‖ L18.3（需 L18.1） ‖ L19.1（需 L14.2+L15.1）
  Q4（L16.1 完成后）：L20.1（许可登记） ‖ L20.3（deny 复核）
串行汇聚点：L14.3（需 L14.2）、L15.3（需 L14.3+L15.2）、L19.2（需 admit+config+feature）、L19.3（需 L17.3+admit+config）、L21（需全部）、L22（需 L21+C1~C3）
```

**不可并行（同文件串行）**：L14.2 与 L14.3 之外的 `live/gate.rs` 编辑须串行；L15.2 与 L16.2 同写 `transport.rs`，L15.2 必须先落 `BudgetExhausted`；L14.3/L15.3/L19.4 同写 `tests/llm_live_gate.rs`，须串行；L18.2 与 L19.2 同属 `tests/llm_live*.rs`，但文件不同可并行。

#### 0.7.3 阶段 D 文件清单（对齐 DESIGN 2.5.3 的 #24~#30 与修改 #12~#23）

**新增 7 个文件**

| # | 文件 | 承载职责 | AC | 任务 |
|---|---|---|---|---|
| 24 | `crates/infrastructure/src/llm/live/policy.rs` | host 精确集合、scheme、模型逐字节比对（**不复用** `DomainPattern::matches`） | 28/29/30 | L14.1 |
| 25 | `crates/infrastructure/src/llm/live/budget.rs` | `BudgetLedger`、`Permit`、`BudgetedTransport` | 31 | L15.1、L15.2 |
| 26 | `crates/infrastructure/src/llm/transport_tls.rs` | `TlsTransport` / `TlsStreamReader`（`feature="llm-https"`） | 30、20~25 | L16.3 |
| 27 | `crates/infrastructure/tests/llm_live_gate.rs` | AC-28~AC-31 离线判定 + 三探针 | 28/29/30/31 | L14.3、L15.3、L19.4 |
| 28 | `crates/infrastructure/tests/llm_transport_tls.rs` | TLS 离线负向用例（`#![cfg(feature="llm-https")]`） | 30 | L16.4 |
| 29 | `scripts/scan-secrets-docs.sh` | 覆盖 `*.md` 与 `.tmp/llm-live/**` 的补充泄漏扫描 | 32 | L17.1 |
| 30 | `scripts/check-model-candidates.sh` | 候选模型名标注与字面量检查 | 29 | L17.2 |

**修改 12 个文件 / 12 处（#12~#23，去重后；其中 #17/#20 各含两文件）**

| # | 文件 | 改动内容 | AC | 任务 |
|---|---|---|---|---|
| 12 | `crates/infrastructure/src/llm/live/gate.rs` | 新增 `admit` / `Admission` / `GateReject` / `AdmissionTicket`；`evaluate` 与 6 个 SkipReason 零改动 | 28~31 | L14.2 |
| 13 | `crates/infrastructure/src/llm/live/config.rs` | 端点必须含已确认 path；`call_limit = min(samples×3×4, cap)`；接入确认凭条 | 28/29/31 | L19.1 |
| 14 | `crates/infrastructure/src/llm/transport.rs` | `TransportError::BudgetExhausted`；I/O 辅助泛化；`json_post_secure` | 31/30 | L15.2、L16.2 |
| 15 | `crates/infrastructure/src/llm/degrade.rs` | `classify(BudgetExhausted) == None` | 31 | L15.2 |
| 16 | `crates/infrastructure/src/llm/gateway.rs` | 一次性/流式 `Err` 分支前置"预算停止"判定 | 31 | L15.2 |
| 17 | `crates/infrastructure/Cargo.toml` | `llm-https = ["dep:native-tls"]` + Windows/macOS optional 依赖 | 30/17 | L16.1 |
| 18 | `docs/compliance/license-review.md` | `native-tls` 三项登记 + 不打包 OpenSSL/ring | 30/17 | L20.1 |
| 19 | `scripts/run-llm-live.sh` | argv 泄漏修正（`grep -f -`）、门顺序与 admit 对齐、C1~C3 存在性检查、调用文档补扫 | 32/19 | L17.3、L19.3 |
| 20 | `crates/infrastructure/tests/llm_live_offline.rs`、`tests/llm_live.rs` | 追加 v2 字段断言；入口改用 `admit` | 27/28~31 | L18.2、L19.2 |
| 21 | `crates/infrastructure/src/llm/live/record.rs` | `llm.live.v2` + 预算/确认/声明字段（只增不删） | 27/31/32 | L18.1 |
| 22 | `crates/infrastructure/src/llm/live/mod.rs` | 导出 `policy`/`budget`/`admit` | 28~31 | L14.1、L14.2 |
| 23 | `crates/domain/src/model/llm.rs` | `DegradeCause::BudgetExhausted`（`FailureClass` 六值不变） | 31 | L15.2 |

**清单外必要补充（DESIGN 2.12.4 明确要求，登记为补充项）**

| 补充 | 文件 | 原因 | 任务 |
|---|---|---|---|
| S1 | `scripts/llm-live-recompute.mjs` | 记录 v2 新增字段必须同步键表与未知键检测 | L18.3 |
| S2 | `crates/infrastructure/tests/llm_live_offline.rs` | v2 键集断言（只追加，不改既有 6 个用例） | L18.2 |
| S3 | `crates/infrastructure/tests/llm_live.rs` | 入口从 `evaluate` 换为 `admit`（8 个 `#[ignore]` 用例名不变） | L19.2 |

> **不列入修改**：`crates/application/**`、`platform-win/mac`、`ui-bridge`、`app/src-tauri`、TS 绑定、`docs/sdd/spec.md`、`docs/sdd/design.md`、`docs/sdd/tasks.md`、`deny.toml`（预期零改动，见 L20.3）、`crates/domain/src/model/enums.rs`（复用 `OutboundQuery`）。

#### 0.7.4 阶段 D 门禁阈值

| 代号 | 命令 | 通过条件 | 备注 |
|---|---|---|---|
| G-FMT | `cargo fmt --all --check` | 退出码 0 | 同既有 |
| G-CLIPPY | `cargo clippy --workspace --all-targets -- -D warnings` | 退出码 0、零 warning | 同既有 |
| G-CLIPPY-F | `cargo clippy -p meeting-copilot-infrastructure --all-targets --features llm-https -- -D warnings` | 退出码 0、零 warning | 阶段 D 起 feature 含真实依赖 |
| G-TEST | `cargo test --workspace` | `failed=0`、`passed ≥ 208` | **195 基线不回退** + ≥13 新增默认用例 |
| G-TEST-TLS | `cargo test -p meeting-copilot-infrastructure --features llm-https` | `failed=0`；`llm_transport_tls` ≥6 用例 | 仅 Windows/macOS 执行 |
| G-LAYER | `bash scripts/check-layering.sh` | 退出码 0 | `native-tls` 不得进入 domain 闭包 |
| G-SECRET | `bash scripts/scan-secrets.sh` | 退出码 0 | 既有仓库扫描 |
| G-SECRET-DOC | `bash scripts/scan-secrets-docs.sh --all-md --artifacts` | **仅退出码 0 通过；退出码 2 = 失败（不可判定）** | 新增，覆盖 `*.md` 与 `.tmp/llm-live/**` |
| G-CAND | `bash scripts/check-model-candidates.sh` | 退出码 0 | 候选不得写成默认值/示例值 |
| G-DENY | `cargo deny check licenses bans sources advisories` | 退出码 0 | native-tls 条目必须已登记 |
| G-OPENSSL | `cargo tree -p meeting-copilot-infrastructure --features llm-https -e normal \| grep -c openssl` | 计数 **== 0** | Windows/macOS 本机；Linux target 探测同（允许构建失败，但不得出现 openssl） |
| G-PRIVKEY | `git grep -nE "BEGIN (RSA \|EC \|OPENSSH )?PRIVATE KEY"` | **0 命中** | 禁止引入测试证书私钥块 |

**新增默认执行用例下限（用于 G-TEST 的 208 阈值）**

| 文件 / 位置 | 下限 | 依据 |
|---|---|---|
| `tests/llm_live_gate.rs` | ≥5 个用例 | DESIGN 2.13.3 具名 5 个 |
| `live/policy.rs` 内联 | ≥3 个用例 | host 精确/scheme/模型比对 |
| `live/budget.rs` 内联 | ≥3 个用例 | limit 并发/不退票/零上限 |
| `live/record.rs` 内联（v2 新增） | ≥2 个用例 | v2 键集、缺接受日期即拒绝 |
| **合计** | **≥13** | 195 + 13 = 208 |

> TLS 用例（≥6）**不计入** 208，只在 `--features llm-https` 下计入 G-TEST-TLS，避免默认构建被 feature 影响（AC-17）。

#### 0.7.5 阶段 D 禁止事项（追加于 0.5，同等强制）

1. 禁改 `docs/sdd/spec.md`、`docs/sdd/design.md`、`docs/sdd/tasks.md`（勾选亦需单独授权）。
2. 不得提交任何密钥、Key 任何可见片段（前缀/中段/末四位）或 Tracking ID 的任何片段；扫描命中时只输出`文件:行号`，禁止回显内容。
3. 真实联调默认关闭；CI 不设 `LLM_LIVE`、不注入 `LLM_API_KEY`；未设开关时不得构造传输、不得建 socket。
4. 不得把候选模型名写成默认值、示例可用值、回退链前项或自动探测目标（SPEC 11.9.2-2）。
5. 不得引入测试证书/私钥夹具（含 `BEGIN ... PRIVATE KEY` 块、自签证书生成依赖）。
6. 不得为"让 Linux 构建通过"而把 `native-tls` 加入通用 target（会在 Linux 拉入 OpenSSL，违反 11.9.3-3 的交付边界）。
7. 不得在 `live/policy.rs` 复用 `DomainPattern::matches`（其"相等或后缀匹配"语义会把 `evil.<host>` 放行，直接违反 AC-28）。
8. 不得回改 L1~L13 已完成的任何条目；阶段 D 的取代/前置关系只在附录 G 登记。

#### 0.7.6 输入阻塞项（C1~C5、M6、M7；**不阻塞离线任务**）

| 编号 | 阻塞内容 | 影响 AC | 未确认时的 fail-closed 行为 | 补齐后的执行入口 | 是否阻塞 L14~L21 |
|---|---|---|---|---|---|
| C1 | 端点路径（host 之后的 path） | AC-28（只判 host）；请求可用性 | `E-PROVIDER-009`，detail 含 `endpoint_path_unconfirmed`；0 请求；记录 `endpoint_path_unconfirmed` | 当前 shell 将 `LLM_BASE_URL` 设为"host + 已确认 path" → `bash scripts/run-llm-live.sh` | 否 |
| C2 | API 模型名（三个候选中的一个，或其它确认值） | AC-29、AC-20/21/22 模型维度 | `Reject(ModelNotConfirmed)`；0 请求；记录 `model_not_confirmed`；不得填任何候选 | 设 `LLM_MODEL` 与 `LLM_MODEL_CONFIRMED`（逐字节相同）→ 同上 | 否 |
| C3 | 采样预算与单次运行调用上限 | AC-31、AC-20/21/22 样本量 | `Reject(BudgetNotConfirmed)`；0 请求；记录 `budget_not_confirmed` | 设 `LLM_LIVE_CALL_CAP`（并可选 `LLM_LIVE_SAMPLES`）→ 同上 | 否 |
| C4 | 其它 DeepSeek 官方域名 | AC-28 域名集合 | 集合仅 `api.deepseek.com`；其余 `endpoint_not_allowed` | 在 `LLM_ALLOWLIST` 登记精确主机名 | 否 |
| C5 | 凭据是否已在服务商侧轮换 | AC-32 前置风险 | 不阻塞判定；记录必须写 `credential_rotated=false` 与"凭据未轮换·存在既暴露风险" | 轮换后置 `credential_rotated=true` 写入记录 | 否 |
| M6 | 确认凭条机制与"凭据读取"口径（`LLM_MODEL_CONFIRMED`；`SecretString` 构造/expose 计数口径） | AC-29、AC-28 拒绝路径断言 | 按 DESIGN 2.10.4-3 口径执行（判空不计入凭据读取）；若用户要求"连 `get` 也为 0"，需改顺序并重跑 AC-19 断言 | 用户书面确认后按〔D6〕口径执行 | 否（按默认口径） |
| M7 | 是否允许非 443 端口 | AC-28 拒绝形态 | 默认拒绝（fail-closed） | 用户确认后调整 `policy` 的端口规则并补用例 | 否（按默认拒绝） |

> 既有 M1~M5（0.6）继续有效；其中 M2 与 C2 同源、M4 与 C3 同源，登记为"同一输入的两个面向"，不重复计数。
> **纪律**：C1~C5/M6/M7 的登记只写入 `docs/progress/b1-status.md` 与 `.tmp/llm-live/`，不写入 `docs/sdd/*.md`。

---

# 阶段 A：离线主体（L1~L8）

## L1 实现领域模型与端口（`crates/domain`）

**任务组目标**：把 SPEC 6.1~6.3 的数据约束前移到构造期，产出四类强类型结果与并存端口；既有 `LlmGateway` 一个字节不改。
**新增文件**：#1 `model/llm.rs`、#2 `port/llm.rs`。**修改文件**：`model/mod.rs`、`port/mod.rs`、`domain/Cargo.toml`。

- [ ] **L1.1 挂载领域依赖与模块（★）**
  - 追溯：SPEC 9.2-1、9.2-2；DESIGN 2.5.3 修改 #1/#2/#5(即 #8)；AC-01（同源 Schema 前置）
  - 前置：无（本清单唯一无前置任务）
  - 并行：否（其余 L1 子任务与全部阶段 A 任务均以本任务为前置）
  - 触达：`crates/domain/Cargo.toml`（改）、`crates/domain/src/model/mod.rs`（改）、`crates/domain/src/port/mod.rs`（改）
  - DoD：
    - `crates/domain/Cargo.toml` 的 `[dependencies]` 增加 `schemars = "1"`（解析到锁文件既有 1.2.3；若 derive 未默认启用则加 `features = ["derive"]`）
    - `model/mod.rs` 增加 `pub mod llm;` 与 `pub use llm::{...};`，且既有 `pub use` 行一行不改
    - `port/mod.rs` 增加 `pub mod llm;` 与 `pub use llm::{...};`，既有 `LlmGateway` 导出保持原样
    - `cargo build -p meeting-copilot-domain` 退出码 0
    - `cargo tree -p meeting-copilot-domain --depth 1` 输出含 `schemars v1.2.x`；若为 1.2.3 以外的 1.x，记录版本与许可证仍为 MIT
    - `git diff --stat Cargo.lock` 无新增 crate 版本条目（只允许依赖关系指针变化）
    - `bash scripts/check-layering.sh` 退出码 0（`schemars` 不在禁依赖列表，domain 不引入网络/存储）
  - 备注〔D1〕：若执行者选择"Schema 仅由 domain 暴露、infrastructure 不直接依赖 schemars"，则 `crates/infrastructure/Cargo.toml` 不加 `schemars`，改为 domain 增 `serde_json` 正式依赖并暴露 `schema_for(task) -> serde_json::Value`。二选一，必须在 L8.1 记录选择；两方案均不新增下载包。

- [ ] **L1.2 任务与请求模型**
  - 追溯：SPEC 5.1.1-1/9/10、6.1-1~6.1-7；DESIGN 2.2.2-A；AC-02、AC-10
  - 前置：L1.1
  - 并行：否（与 L1.3/L1.4 同写 `model/llm.rs`，须串行）
  - 触达：`crates/domain/src/model/llm.rs`
  - DoD（内联 `mod tests` 断言，`cargo test -p meeting-copilot-domain` 全绿）：
    - `LlmTask` 为四变体封闭枚举，`ALL.len() == 4`，`as_str()` 四值互不相同
    - `default_budget()` 四任务默认值符合 DESIGN 2.1.3(10)：`InsightExtract 512/200`、`Summary 400/150`、`CardConclusion 256/100`、`MinutesDraft 1024/200`
    - `RedactedPayload::new("", _, 8000)` → `Err(EmptyPayload)`；`8001` 字符 → `Err(PayloadTooLong{actual:8001,limit:8000})`；`8000` 字符 → `Ok` 且 `char_count() == 8000`
    - `OutputBudget::new(0)` 与 `new(4097)` → `Err`；`new(1)`/`new(4096)` → `Ok`
    - `TemperatureMilli::new(1001)` → `Err`；`new(0)`/`new(1000)` → `Ok`
    - `DeadlineMs::new(0)` 与 `new(8001)` → `Err`；`new(1)`/`new(8000)` → `Ok`
    - `task != Summary && purpose.is_some()` → `Err(PurposeMismatch)`；`task == Summary && purpose.is_none()` → `Err(PurposeMismatch)`
    - 多字节载荷按 `chars()` 计数（如 8000 个汉字通过；8001 个汉字拒绝）
    - 全部构造失败路径不 panic（用例以 `matches!` 断言错误变体）

- [ ] **L1.3 四类结构化结果与辅助类型**
  - 追溯：SPEC 5.1.1-4/5/6/7/8、6.4~6.6；DESIGN 2.2.2-B；AC-10、AC-13
  - 前置：L1.2
  - 并行：否（同文件串行）
  - 触达：`crates/domain/src/model/llm.rs`（复用 `vo/{text,confidence,ids,time}`、`model/{enums,card,session,utterance}`，均不修改）
  - DoD：
    - `InsightExtract{ insights: Vec<InsightItem> }`，`InsightItem{kind: InsightKind, text: ShortText, sources: Vec<UtteranceRef>, confidence: Confidence, decision: Option<DecisionFields>, todo: Option<TodoFields>}`
    - `Summary{purpose: SummaryPurpose, text: ShortText, covered: Option<TextRange>, version: u32, carried_over: bool, produced: bool}`，`purpose` 恒存在不可省
    - `CardConclusion{title: TitleText, conclusion: Option<ConclusionText>, references: Vec<ReferenceRef>, confidence: Confidence, conflict: Option<ConflictView>, notice: Option<ConclusionNotice>}`
    - `MinutesDraft{summary: ShortText, insights: MinutesInsights, timeline: Timeline, open_questions: Vec<PendingItem>, sources: Vec<UtteranceRef>, unrefined_count: u32, completion_hint: Option<CompletionHint>}`
    - `MinutesInsights{key_info, decision, todo, question: Vec<ShortText>}` 四区均存在（类型层面不可缺区）
    - `Timeline{start: Option<AudioClock>, end: Option<AudioClock>, entries: Vec<TimelineEntry>}`，允许空区间
    - `StructuredBody` 四变体 + `StructuredBody::task() -> LlmTask`，四变体映射与请求任务一一对应
    - `ConclusionNotice::{MissingSource, GenerationUnavailable}`，`PendingReason::{AwaitingOwner, AwaitingDue, Unrefined}`，`AuditPhase::{Intent, Result}`
    - 每类结果带 `schema_version: &'static str`（值与 `LlmTask::schema_version()` 一致）
    - `cargo build -p meeting-copilot-domain` 退出码 0；`cargo clippy -p meeting-copilot-domain --all-targets -- -D warnings` 退出码 0

- [ ] **L1.4 结果载体、故障分类与降级原因**
  - 追溯：SPEC 5.3.1-1/5/6/7、5.4.1-2、6.2、6.3；DESIGN 2.2.1-4、2.2.2-C；AC-09、AC-10
  - 前置：L1.3
  - 并行：否（同文件串行）
  - 触达：`crates/domain/src/model/llm.rs`
  - DoD：
    - `FailureClass` 严格六变体（`Timeout/RateLimited/InvalidSchema/Unavailable/ContentFiltered/Offline`），**不含任何自由文本字段**；`FailureClass::ALL.len() == 6`
    - `DegradeCause::{Failed(FailureClass), Cooling, QueueTimeout, PayloadTooLarge, AuditUnavailable, ConfigInvalid}` 与 `FailureClass` 分离
    - `StructuredOutcome{request_id, task, body: Option<StructuredBody>, model: String, tokens_used: Option<u32>, degraded: bool, degrade_cause: Option<DegradeCause>, failure: Option<FailureClass>, attempts: u8, truncated: bool, schema_version}`
    - 不变量用例：`degraded == true` 时 `degrade_cause.is_some()`；`degraded == false` 时 `degrade_cause.is_none()`；`degrade_cause == Some(Failed(c))` 时 `failure == Some(c)`
    - `StreamEvent{sequence: u32, kind: StreamEventKind}`；`StreamEventKind::{Delta{text, first_token}, Completed{outcome}, Failed{failure, degraded, cause}}`
    - `StreamOutcome{request_id, terminated: Termination, discarded_deltas: u32, cancelled: bool, first_token_ms: Option<u32>}`；`Termination::{Completed(StructuredOutcome), Failed{...}}`
    - 观测点：`LlmMetric` 六值（`FirstTokenLatencyMs/TotalLatencyMs/RetryCount/FailureCount/DegradeCount/MetricsDropped`）、`LlmMetricPoint`、`LlmAuditRecord`（十字段，仅元数据）、`LlmLogRecord`（构造期白名单校验）、`CircuitState::{Closed, Open{opened_at: Instant}}`
    - 用例：`LlmAuditRecord` 的 `summary` 格式化函数只产出 `phase/task/chars/model/endpoint/elapsed/outcome/degraded/attempts`，断言不含载荷占位串
    - `cargo test -p meeting-copilot-domain` 全绿（新增用例计入 ≥10 内联单测下限）

- [ ] **L1.5 领域端口（★，与既有 `LlmGateway` 并存）**
  - 追溯：SPEC 4.5-2、5.2.1-2/5；DESIGN 2.2.2-D；AC-02、AC-03
  - 前置：L1.4
  - 并行：否（L2/L3/L5 均需本任务的端口类型）
  - 触达：`crates/domain/src/port/llm.rs`（新增）、`crates/domain/src/port/mod.rs`（改）
  - DoD：
    - `StructuredLlmGateway: Send + Sync`，三方法：`complete_structured(&self, &LlmGenerationRequest) -> Result<StructuredOutcome, CopilotError>`、`stream_structured(&self, &LlmGenerationRequest, &CancelToken, &mut (dyn FnMut(StreamEvent) + Send)) -> Result<StreamOutcome, CopilotError>`、`capability(&self) -> LlmCapability`
    - 方法均非泛型（`Arc<dyn StructuredLlmGateway>` 可实例化），并有编译期断言用例
    - `CancelToken`：`new()/cancel()/is_cancelled()`；克隆后共享同一标志（用例断言）
    - `StreamCollector<F: FnMut(StreamEvent) + Send>`：`new/emit/discarded/termination`；`emit` 在终止后再调用只增 `discarded`、不回调内层
    - `LlmObserver: Send + Sync`，`record_metric -> Result`、`append_audit -> Result`、`log(&LlmLogRecord)`（日志无返回值，字段已在构造期校验）
    - `LlmCapability{task, endpoint_configured, model_configured, cooling}`
    - `crates/domain/src/port/understand.rs` 的 `git diff` 为空
    - 用例：构造一个 `struct NullGateway` 实现 trait 并赋给 `Arc<dyn StructuredLlmGateway>`，编译通过
    - `cargo test -p meeting-copilot-domain` 全绿

## L2 Schema 派生与严格解码

**任务组目标**：类型是唯一真源；一份派生结构同时服务"提示词下发"与"结果校验"，杜绝双份手写。
**新增文件**：#5 `schema.rs`、#9 `tests/llm_schema.rs`。**并行**：L1.5 之后可与 L3、L5 三线并行。

- [ ] **L2.1 实现 `SchemaRegistry`（派生 + 严格解码 + 跨字段规则）**
  - 追溯：SPEC 5.1.1-2/3/4/6/8、6.4~6.6；DESIGN 2.2.2-F、2.4.2 AC-01/AC-07/AC-13；AC-01、AC-07、AC-13、AC-25（修复提示只含 path/rule）
  - 前置：L1.5
  - 并行：⟂ 与 L3.*、L5.* 并行
  - 触达：`crates/infrastructure/src/llm/schema.rs`（新增）；按 D1 选择可能触及 `crates/infrastructure/Cargo.toml`
  - DoD（内联 `mod tests`）：
    - `SchemaRegistry::json_schema(task) -> &'static serde_json::Value`，四任务各一份，根节点含 `"additionalProperties": false`
    - `decode(task, raw)` 对四类合法样例均 `Ok(body)`，且 `body.task() == task`
    - 六组非法夹具全部返回 `Err(SchemaError{rule, path})`，`rule` 命中 `NotJson/UnknownField/TypeMismatch/RangeViolation/CrossField`：
      1. 非 JSON 文本 → `NotJson`
      2. 含未知字段（幻觉字段）→ `UnknownField`
      3. 字段类型错（`confidence` 传字符串）→ `TypeMismatch`
      4. 要点正文 61 字 / 置信度 1.2 → `RangeViolation`
      5. 结论非空但 `references` 为空 → `CrossField`
      6. 纪要缺"未决问题"区、或"待确认"条目出现在结论区 → `CrossField`
    - `SchemaError.path` 形如 `insights[0].sources`，且**不含**任何载荷片段（用标记串 `SENSITIVE-TOKEN` 注入后断言 `path` 与 `rule` 均不含该串）
    - 修复提示构造只输出 `{"path":..., "rule":...}` 两字段（AC-25 前置）
    - 模型标识为空时由网关回退为配置 `model`，本模块不编造
    - `cargo test -p meeting-copilot-infrastructure schema::` 全绿

- [ ] **L2.2 编写 Schema 一致性用例（AC-01）**
  - 追溯：SPEC 8.1 AC-01；DESIGN 2.4.2 AC-01 与 2.4.2 末段"判定口径说明"
  - 前置：L2.1
  - 并行：⟂ 与 L3.3 并行
  - 触达：`crates/infrastructure/tests/llm_schema.rs`（新增；首次创建 `crates/infrastructure/tests/` 目录）
  - DoD：
    - 用例 `four_schemas_are_derived_from_types`：四类样例 `decode` 全部 `Ok`
    - 断言 `json_schema(task)` 根 `properties` 的键集 == 对应领域类型的字段集（用固定期望键集常量，任何一侧漂移即失败）
    - 断言四类 schema 的 `schema_version` 与 `LlmTask::schema_version()` 一致
    - 断言四类"非法样例"均被拒（每类 ≥1 例）
    - 快照：四份 schema 的规范化 JSON 与测试内快照常量逐字比对（类型改动未同步 → 失败）
    - `cargo test -p meeting-copilot-infrastructure --test llm_schema` 全绿，用例数 ≥5

## L3 HTTP 传输边界与回环实现

**任务组目标**：传输是唯一知道 socket 的模块；错误枚举是 `FailureClass` 的唯一来源（R10）。
**新增文件**：#6 `transport.rs`、#13 `tests/support/mod.rs`。**并行**：L1.5 之后与 L2、L5 并行。

- [ ] **L3.1 实现 `Http1Codec` 与 `TransportError`（★）**
  - 追溯：SPEC 4.3-5；DESIGN 2.2.2-E、2.8.2；AC-04、AC-09
  - 前置：L1.5
  - 并行：⟂ 与 L2.1、L5.1 并行
  - 触达：`crates/infrastructure/src/llm/transport.rs`（新增）
  - DoD：
    - `TransportError` 十变体（`DnsFailure/NetworkUnreachable/ConnectRefused/ConnectTimeout/ReadTimeout/TlsFailure/PeerClosed/TooLarge/Malformed/Cancelled`）
    - `Http1Codec` 只处理 `&[u8]`：请求行 + 头 + 体序列化；状态行 + 头 + `chunked`/定长体解析；不完整报文与超长报文分别返回 `Malformed`/`TooLarge`
    - `HttpRequest` 的 `Debug` 实现遮蔽 `Authorization` 头与 `body`（用例断言 `format!("{:?}")` 不含标记串）
    - 内联单测 `codec_contract`：同一份字节流在"回环产生"与"内存 buffer 构造"两种来源上解析结果完全一致（≥3 组：定长体 / chunked / 分块到达）
    - 内联单测：截断报文 → `Malformed`；声明长度 > `max_response_bytes` → `TooLarge`
    - 不跟随 3xx、不读系统代理：`HttpRequest` 无代理字段，代码中不出现 `HTTP_PROXY` 读取
    - `cargo test -p meeting-copilot-infrastructure transport::` 全绿

- [ ] **L3.2 实现 `LoopbackTransport`（回环 socket，超时/取消/白名单）（★）**
  - 追溯：SPEC 4.1-4/6、4.3-5、5.3.1-4；DESIGN 2.1.3-（5）（6）、2.8.2；AC-04、AC-09、AC-14
  - 前置：L3.1
  - 并行：否（L4 的分类映射需要本任务的 `TransportError` 产出路径）
  - 触达：`crates/infrastructure/src/llm/transport.rs`
  - DoD：
    - `HttpTransport::send(&HttpRequest, deadline: Instant, &CancelToken) -> Result<HttpResponse, TransportError>` 与 `open_stream(...) -> Result<Box<dyn StreamReader>, TransportError>`
    - `TcpStream::connect_timeout` + `set_read_timeout/set_write_timeout` 由**剩余预算**驱动；读取循环每 ≤50 ms 复查 `deadline` 与 `cancel`
    - 到期返回 `ReadTimeout`，取消返回 `Cancelled`；`StreamReader::drop` 立即释放连接（用例断言 mock 端看到 EOF）
    - 白名单校验在发出前完成：host 未命中 → 返回配置错误（`CopilotError::ProviderConfigInvalid`），不建 socket（用例以"mock 收到 0 请求"断言）
    - 内联单测：`budget_to_timeout(Duration::from_millis(500))` 落到 socket 超时且不为 0；`Duration::ZERO` 不 panic
    - 与 `FakeTransport` 的契约对齐：超时、取消、超长、对端关闭四场景，`LoopbackTransport` 与 `FakeTransport` 的分类结果一致（DESIGN 2.8.3 可验证证明 2）
    - `cargo test -p meeting-copilot-infrastructure transport::` 全绿

- [ ] **L3.3 搭建测试脚手架与进程内 mock 端点**
  - 追溯：SPEC 3.2、8.2-3/4；DESIGN 2.4.1"mock 端点设计"与"测试脚手架"；AC-02~AC-15 全部
  - 前置：L3.2
  - 并行：⟂ 与 L2.2 并行
  - 触达：`crates/infrastructure/tests/support/mod.rs`（新增）
  - DoD：
    - `TcpListener::bind("127.0.0.1:0")` 随机端口 + 1 个接受线程；记录"请求数 / 到达时刻 / 请求体长度"
    - 脚本能力齐备：`Json(code, body)`、`Status(code, headers)`、`Delay(dur, then)`、`SseChunks(chunks, interval)`、`AbortMidStream`、`Malformed`、`SlowBody`
    - 所有写失败只忽略错误，不 panic；测试结束关闭 listener 使接受线程退出（用例断言无残留线程 panic）
    - 提供 `FakeTransport`（脚本化 `TransportError`，覆盖 DNS/拒绝/TLS/超长）
    - 提供断言工具：`assert_timeout_within(elapsed, deadline, tolerance=200ms)`、`SENSITIVE-TOKEN` 泄漏断言助手
    - 连续 3 次执行同一用例集结果一致（`cargo test --test llm_gateway_mock` 三次全绿，L7.1 后验证）
    - 脚手架不进入产物：`cargo build -p meeting-copilot-infrastructure` 的产物中无 `support` 符号；`git status` 无额外文件

## L4 降级决策（分类、重试、退避、冷却、并发）

**任务组目标**：纯决策逻辑 + 可注入时钟，让 AC-05/06/08 不依赖真实睡眠。
**新增文件**：#7 `degrade.rs`。**并行**：L3.1 之后与 L2、L5 并行。

- [ ] **L4.1 实现六类故障分类（唯一判定点）**
  - 追溯：SPEC 5.3.1-1/7、5.3.3-5、4.1-5；DESIGN 2.1.3-（3）分类映射表；AC-04、AC-07、AC-09
  - 前置：L3.1
  - 并行：⟂ 与 L2.1、L5.1 并行
  - 触达：`crates/infrastructure/src/llm/degrade.rs`（新增）
  - DoD：
    - `classify(TransportError) -> FailureClass`：`DnsFailure|NetworkUnreachable → Offline`；`ConnectRefused|TlsFailure|PeerClosed → Unavailable`；`ConnectTimeout|ReadTimeout → Timeout`；`TooLarge|Malformed → InvalidSchema`；`Cancelled` 不进入分类（返回 `None`）
    - `classify_status(u16, headers, body_len) -> Option<FailureClass>`：`429 → RateLimited`；`5xx → Unavailable`；`3xx → Unavailable`（不跟随）；`400 且内容策略类错误码 / finish_reason=content_filter → ContentFiltered`；其余 2xx → `None`
    - 分类**只**由错误变体与状态码决定，不出现字符串匹配响应正文（用例以 `SENSITIVE-TOKEN` 正文断言分类不受影响）
    - 内联单测：`TransportError` 十变体逐个映射断言；`429/500/302` 三状态码断言
    - `cargo test -p meeting-copilot-infrastructure degrade::` 全绿

- [ ] **L4.2 实现重试策略、429 退避与截止时间优先（★）**
  - 追溯：SPEC 5.3.1-2/3/4/9、5.3.3-1/2；DESIGN 2.1.3-（4）表；AC-05、AC-06
  - 前置：L4.1
  - 并行：否（AC-05/06 的核心）
  - 触达：`crates/infrastructure/src/llm/degrade.rs`
  - DoD：
    - 每类重试上限：`Timeout/Unavailable → 2`（总 3 次尝试）、`RateLimited → 1`、`InvalidSchema → 1`（修复）、`ContentFiltered/Offline → 0`
    - `RetryDecision::{Retry{delay}, Degrade{cause}}`；`MAX_ATTEMPTS` 与 `MIN_ATTEMPT_BUDGET=50ms` 常量有断言
    - 截止时间优先：`remaining < delay + MIN_ATTEMPT_BUDGET` → `Degrade`，不重试
    - `Retry-After` 解析：纯数字秒 → `Duration`；HTTP-date → 视为未声明；非法/缺失 → 未声明；声明值 >2 s → 不重试
    - 抖动：由 `request_id` 哈希派生 ±20%（确定性），同一 `request_id` 两次计算结果相同（用例断言）
    - 内联单测（假时钟，零真实睡眠）：
      - 连续 `Timeout` → `attempts == 3` 后 `Degrade`
      - `429 + Retry-After: 1` → 第 2 次尝试前等待 ≥900 ms 且 ≤2 s（用假时钟推进断言）
      - `429 + Retry-After: 5` 且剩余 2 s → 不重试、立即降级
      - `429` 无 `Retry-After` → 抖动退避 ≤2 s，且 `delay > 0`
      - `Offline`/`ContentFiltered` → `attempts == 1`
      - `InvalidSchema` → 仅 1 次修复重试
    - `cargo test -p meeting-copilot-infrastructure degrade::` 全绿

- [ ] **L4.3 实现冷却状态机、并发闸门与 `Clock`（★）**
  - 追溯：SPEC 4.1-6、4.2-2、5.3.1-5/6、5.4-9；DESIGN 2.1.3-（4）（6）、2.3.2 端点冷却状态机；AC-08、AC-14
  - 前置：L4.2
  - 并行：否
  - 触达：`crates/infrastructure/src/llm/degrade.rs`
  - DoD：
    - `Clock` trait：`now() -> Instant`、`sleep(Duration)`；`SystemClock` 与 `FakeClock`（测试用，`advance(d)`）
    - `EndpointCircuit`：`on_final_failure()`（连续 3 次 → `Open{opened_at}`）、`on_success()`（清零）、`state(now)`（≥60 s 自动回 `Closed` 并清零）
    - 冷却期内：不发起请求、不占并发许可、不累加失败计数（用例断言 `send` 调用次数为 0）
    - 并发闸门：上限默认 3、FIFO、RAII 释放；以剩余预算为条件变量超时；准入估算超预算 → `QueueTimeout` 且**不入队**
    - 内联单测（`FakeClock`）：
      - 3 次最终失败 → `state == Open`；第 4 次直接 `Degrade{Cooling}`，`FakeTransport` 调用 0 次
      - 推进 60 s → `state == Closed`，下一次调用恢复发送
      - 成功一次 → 计数清零（后续 2 次失败仍不进入 Open）
      - 并发 4 请求、上限 3：第 4 个排队；若其剩余预算不足 → `QueueTimeout`
      - `CancelToken` 置位后排队中的请求立即返回，不再获取许可
    - `cargo test -p meeting-copilot-infrastructure degrade::` 全绿

- [ ] **L4.4 补齐降级决策的失败路径用例（AC-09 前置）**
  - 追溯：SPEC 4.2-1、5.3.1-8；DESIGN 2.4.3-4、2.1.3-（5）"不 panic"；AC-09
  - 前置：L4.3
  - 并行：⟂ 与 L5.2 并行
  - 触达：`crates/infrastructure/src/llm/degrade.rs`（内联 `mod tests`）
  - DoD：
    - 六类故障各注入一次（`FakeTransport` 提供 DNS/拒绝/TLS/超长；`FakeClock` 提供超时；脚本提供 429/5xx/非法结构），断言均返回分类结果、`catch_unwind` 为 `Ok`、无 `panic!`
    - 断言 `degrade.rs` 中不出现 `unwrap()/expect()`（可用 `grep -c` 作为辅助检查，但不作为唯一判据）
    - `cargo test -p meeting-copilot-infrastructure degrade::` 全绿；用例数 ≥10（累计）

## L5 规则兜底（四类任务的本地替代产出）

**任务组目标**：无网络、无 IO、单次 ≤50 ms；四类结果与正常结果同型。
**新增文件**：#8 `fallback.rs`。**并行**：L1.5 之后与 L2、L3 并行。

- [ ] **L5.1 实现四类规则兜底**
  - 追溯：SPEC 5.4.1-2/3/4/5/6/7/9、5.1.1-5、6.6；DESIGN 2.1.3-（8）表；AC-10、AC-11、AC-12、AC-13
  - 前置：L1.5
  - 并行：⟂ 与 L2.1、L3.1 并行
  - 触达：`crates/infrastructure/src/llm/fallback.rs`（新增）
  - DoD：
    - `RuleFallback::produce(task, input) -> StructuredBody`，`input` 仅含本次请求快照（片段 ≤200 条、单条 ≤2000 字）
    - `InsightExtract`：≥1 条；每条 `sources` 非空且引用命中输入片段；`confidence ∈ {0.35, 0.5}`（≤0.5）；决策人/负责人保留"待确认"，不填人名
    - `Summary`：有上一版 → 原样沿用、`carried_over=true`、`produced=true`、`version` 不变；无上一版 → `produced=false`、文案"未生成"
    - `CardConclusion`：`conclusion == None` 恒成立；`notice ∈ {MissingSource, GenerationUnavailable}`；引用为空时结论必空
    - `MinutesDraft`：五区骨架齐全（概要/四类要点/时间轴/未决问题/来源清单）；`unrefined_count > 0` 且 `completion_hint.is_some()`；"待确认"条目只出现在 `open_questions`
    - 除 `Summary` 外不得读取任何历史结果缓存（函数签名层不提供历史入口）
    - 纯同步、零 IO：函数体内不出现 `std::net`/`std::fs`/`std::thread::sleep`（`grep` 辅助 + 代码审查）
    - 内联单测：四类结果变体断言 + 每类硬约束断言（≥8 用例）
    - `cargo test -p meeting-copilot-infrastructure fallback::` 全绿

- [ ] **L5.2 兜底性能与"以旧充新"禁令用例**
  - 追溯：SPEC 5.4.1-7/9；DESIGN 2.1.3-（8）"兜底实现约束"与"以旧充新禁止"；AC-09、AC-11
  - 前置：L5.1
  - 并行：⟂ 与 L4.4 并行
  - 触达：`crates/infrastructure/src/llm/fallback.rs`（内联 `mod tests`）
  - DoD：
    - 输入上限（200 条 × 2000 字）下四类兜底各执行 20 次，单次耗时 p95 ≤50 ms（`Instant` 计时，无网络）
    - 无网络证明：用例运行期间 mock 端点请求计数为 0（传入未使用的 transport 探针，断言 0）
    - `Summary` 有历史 vs 无历史两分支产出可区分；其余三类在同一输入下重复调用结果一致（不引入历史）
    - 空输入（既无片段也无上一版）→ 返回"空结果 + 未生成"标识，不 panic、不抛错（SPEC 5.4.3-1）
    - `cargo test -p meeting-copilot-infrastructure fallback::` 全绿

## L6 网关编排与装配

**任务组目标**：唯一编排点；所有失败路径以 `StructuredOutcome` 返回；网关对"联调"零感知。
**新增文件**：#3 `mod.rs`、#4 `gateway.rs`。**修改文件**：`crates/infrastructure/src/lib.rs`、`crates/infrastructure/Cargo.toml`。

- [ ] **L6.1 配置契约、模块注册与空 feature 声明（★）**
  - 追溯：SPEC 4.3-1/5、4.5-1/4；DESIGN 2.2.2-G、2.5.3 修改 #3/#4/#10、2.8.1"Phase A 的 feature 声明方式"；AC-15、AC-16
  - 前置：L1.5
  - 并行：⟂ 与 L2.2、L3.3 并行（但 L6.2 必须等全部前置完成）
  - 触达：`crates/infrastructure/src/llm/mod.rs`（新增）、`crates/infrastructure/src/lib.rs`（改）、`crates/infrastructure/Cargo.toml`（改）
  - DoD：
    - `LlmConfig` 字段齐备（`schema_version/endpoint_id/endpoint/model/domain_allowlist/request_timeout_ms/max_retries/cooling_threshold/cooling_ms/max_concurrency/payload_char_limit/max_response_bytes/max_stream_chars/defaults/credential_ref`）
    - `validate()` 校验：`domain_allowlist` 非空、endpoint 为 `http(s)://` 且 host 命中白名单（精确或 `*.suffix`）、`model` 非空且 ≤128 字符、`request_timeout_ms ∈ 100..=8000`、`max_concurrency ∈ 1..=8`、`cooling_threshold ∈ 1..=10`、`cooling_ms ∈ 1000..=600000`；全部失败 → `CopilotError::ProviderConfigInvalid`（`E-PROVIDER-009`），错误文本只含字段名与规则
    - `credential_ref` 命中既有 `DomainPattern`（`model/provider.rs:90-127`）的通配能力，语义解释为白名单
    - `CredentialRef`：`Debug`/`Display` 恒 `***`；不实现 `Serialize`（编译期断言：`fn assert_not_serializable<T: serde::Serialize>()` 无法实例化，以 trait 排除法用例表达）
    - `LlmConfig` 以 `Arc<RwLock<...>>` 快照保存；`replace(config)` 整体替换、在途请求持有开始时的快照
    - `Cargo.toml` 的 `[features]` 增加**空** feature：`llm-https = []`；`default` 仍为 `["bundled-sqlite"]`，`sqlcipher` 不变
    - `crates/infrastructure/src/lib.rs` 增加 `pub mod llm;`，既有模块顺序不变
    - `cargo build -p meeting-copilot-infrastructure` 退出码 0；`cargo clippy -p meeting-copilot-infrastructure --all-targets --features llm-https -- -D warnings` 退出码 0（无 `unexpected_cfgs`）

- [ ] **L6.2 一次性结构化生成编排（★）**
  - 追溯：SPEC 5.1.1-2/10、5.3.1-2~8、5.4.1-1、5.5.3-2；DESIGN 2.1.3-（1）流程图与关键判定规则；AC-02、AC-04~AC-10、AC-14
  - 前置：L2.1、L3.2、L4.3、L5.1、L6.1
  - 并行：否（编排汇聚点）
  - 触达：`crates/infrastructure/src/llm/gateway.rs`（新增）
  - DoD（编排顺序不可调换）：
    1. 请求/配置校验（失败 → `Err(ProviderConfigInvalid)`，零外发、不进入冷却计数）
    2. 审计意图写入（失败 → 置审计不可用、走兜底 `AuditUnavailable`、零外发）
    3. 冷却检查（`Open` → 兜底 `Cooling`，不占许可、不加计数）
    4. 并发准入（超预算 → 兜底 `QueueTimeout`，不入队）
    5. 传输 + 分类 + 重试（受 `RetryDecision` 与剩余预算约束）
    6. `SchemaRegistry::decode`（失败 → `InvalidSchema`，修复重试 1 次）
    7. 成功 → 清零冷却、写结果审计 + 度量、返回正常结果
    8. 耗尽 → 累加最终失败、兜底 `Failed(class)`、写结果审计 + 降级度量
    - 载荷超限（`RedactedPayload::new` 失败中的 `PayloadTooLong`）→ `Ok{degraded:true, cause:PayloadTooLarge}`，不发起外发（SPEC 5.1.1-10）
    - 外部故障一律 `Ok(StructuredOutcome{degraded:true})`，`Err` 只表达请求/配置非法
    - `attempts` 如实记录（1..=3）；`degraded=true` 时 `degrade_cause` 必填
    - 网关内无 `unwrap/expect`；线程 panic 由闸门捕获 → `Unavailable`
    - 独立请求线程 + 调用方条件变量等待（AC-14 的执行模型前提）：调用方线程不执行 socket 读
    - 内联单测（`FakeTransport` + `FakeClock`）：成功路径、审计失败路径、冷却路径、排队超时路径、修复重试成功/失败路径各 1 例（≥5）
    - `cargo test -p meeting-copilot-infrastructure gateway::` 全绿

- [ ] **L6.3 流式结构化生成编排（★）**
  - 追溯：SPEC 5.2.1-1~6、5.2.3-1/2、6.3-1~5；DESIGN 2.1.3-（2）时序与 8 条流式解析规则；AC-03、AC-14
  - 前置：L6.2
  - 并行：否（同写 `gateway.rs`）
  - 触达：`crates/infrastructure/src/llm/gateway.rs`
  - DoD（全部为可判定断言）：
    - SSE 解析：只处理 `data:` 行；多行 `data:` 以换行拼接；`[DONE]` 为结束；`choices[0].delta.content` 提取
    - 空增量 / 心跳 / `role` 帧不产生 `Delta`、不占序号
    - 序号由网关自持，从 1 递增、不跳号；重复内容丢弃并计 `discarded_deltas`
    - `CancelToken` 置位后 ≤50 ms 停止交付、关闭连接、不重试、不回滚已交付序号
    - 首个增量即记录 `first_token_ms`；若首个增量到达已越过 deadline → 不交付该增量，直接 `Failed{Timeout}`
    - `Content-Type: application/json` → 完整 JSON 作为唯一增量交付，再 `Completed`（非流式回退，拼接结果与一次性等价）
    - 拼接完成后整体 `decode`；失败 → `Failed{InvalidSchema}`，不交付半截结果
    - 唯一终止：`StreamCollector` 保证第二次终止被丢弃并计数
    - 累计字符 ≥ `max_stream_chars`（默认 64 KiB）→ 停止读取、`truncated=true` → 按 `InvalidSchema` 终止
    - 流中 `PeerClosed` → `Failed{Unavailable}`；空闲超预算 → `Failed{Timeout}`
    - 内联单测（假流）：正常 3 增量、重复增量、取消、越界首 token、非流式回退、流中断（≥6）
    - `cargo test -p meeting-copilot-infrastructure gateway::` 全绿

- [ ] **L6.4 观测适配（度量缓冲、审计链适配、白名单日志）**
  - 追溯：SPEC 4.3-2/3/4、4.4-1/4、5.5.1-1~6、5.5.3-1/2；DESIGN 2.1.3-（9）、1.2.3、1.2.4；AC-15、AC-17
  - 前置：L6.2
  - 并行：⟂ 与 L6.3 不可并行（同模块文件）；与 L7.3 可并行（不同文件）
  - 触达：`crates/infrastructure/src/llm/mod.rs`（`LlmMetricsBuffer`、审计适配、日志适配）
  - DoD：
    - `LlmMetricsBuffer`：有界环形，每指标保留最近 1024 点；满则丢最旧并累加 `MetricsDropped`；提供 `p95(metric)` 与 `failure_rate(class)`
    - 只读投影：`TotalLatencyMs` → 既有 `Metric::InsightLatency`（通过 `MetricsStore::record`），既有 `Metric` 枚举与 10 项断言不变
    - 审计适配：复用 `AuditChain::append` 与 `AuditAction::OutboundQuery`；`AuditId = request_id + phase`；`summary` 仅元数据；两次 append（Intent → Result）
    - 审计失败：`append_audit` 返回 `Err` → 置审计不可用状态（后续请求阻断，SPEC 5.5.3-2）
    - 度量失败：只丢弃 + 计数，不影响结果返回（SPEC 5.5.3-1）
    - 日志适配：`LlmLogRecord` 构造期调用既有 `validate_field`；只允许 `failure_class/error_code/latency_ms/provider_id`；非白名单字段构造即 `Err`，不写
    - 用例：注入 `SENSITIVE-TOKEN` 载荷与响应正文，捕获所有日志与审计 `summary`，断言不含该串、不含 URL 全文、不含凭据
    - `cargo test -p meeting-copilot-infrastructure` 全绿，且既有 `metrics`/`audit`/`telemetry` 用例数不减少

- [ ] **L6.5 装配入口 `assemble()` 与能力查询**
  - 追溯：SPEC 9.1-3；DESIGN 2.1.2、2.2.2-D 调用示例；AC-02、AC-03
  - 前置：L6.4
  - 并行：否
  - 触达：`crates/infrastructure/src/llm/mod.rs`
  - DoD：
    - `assemble(config: Arc<RwLock<LlmConfig>>, transport: Arc<dyn HttpTransport>, observer: Arc<dyn LlmObserver>) -> Result<Arc<dyn StructuredLlmGateway>, CopilotError>`
    - 返回 `Arc<dyn StructuredLlmGateway>`，`capability()` 返回的 `endpoint_configured`/`model_configured`/`cooling` 与配置/冷却状态一致
    - `assemble` 不读取任何环境变量（`grep` 断言 `std::env::` 只在测试目录出现）
    - 用例：以假 transport/observer 装配后 `complete_structured` 与 `stream_structured` 均可调用（各 1 例）
    - `cargo test -p meeting-copilot-infrastructure` 全绿

## L7 集成测试（mock 与故障注入）

**任务组目标**：把 AC-02~AC-15 的判定落到可重复执行的测试目标；全部只连本机回环。
**新增文件**：#10~#12。**并行**：L6.5 完成后三文件可并行。

- [ ] **L7.1 一次性 / 流式 / 不阻塞采集用例（AC-02、AC-03、AC-14）**
  - 追溯：SPEC 8.1 AC-02/AC-03/AC-14、8.2-1~4；DESIGN 2.4.2 同名行；AC-02、AC-03、AC-14
  - 前置：L3.3、L6.5
  - 并行：⟂ 与 L7.2、L7.3 并行
  - 触达：`crates/infrastructure/tests/llm_gateway_mock.rs`（新增）
  - DoD：
    - `complete_ok`：mock 返回合法 JSON → `body` 变体正确、`degraded == false`、`schema_version` 正确、`attempts == 1`、mock 请求数 == 1
    - `stream_ok`：mock 发 ≥2 个 SSE 增量 + `[DONE]` → 序号 1..n 连续、唯一 `Completed`、拼接结果与 `complete_ok` 的 `body` 相等（`assert_eq!`）
    - `non_blocking_capture`：mock `Delay(1s)`；主线程以 5 ms 间隔处理 100 条片段并测本地耗时 → p95 ≤200 ms；网关调用在独立线程，主线程总耗时 <1 s（证明未阻塞）
    - 所有用例只连 `127.0.0.1` 随机端口；断网可跑；连续 3 次执行结果一致
    - `cargo test -p meeting-copilot-infrastructure --test llm_gateway_mock` 全绿，用例数 ≥3

- [ ] **L7.2 故障注入与降级用例（AC-04~AC-13）**
  - 追溯：SPEC 8.1 AC-04~AC-13、8.2-2/4；DESIGN 2.4.2 同名行、2.4.3；AC-04~AC-13
  - 前置：L3.3、L6.5
  - 并行：⟂ 与 L7.1、L7.3 并行
  - 触达：`crates/infrastructure/tests/llm_degrade_inject.rs`（新增）
  - DoD（每个用例一条可复述断言）：
    - `timeout_deadline`（AC-04）：mock `Delay(3s)`、deadline 500 ms → `elapsed ≤ 700 ms`、`failure == Timeout`、mock 连接被关闭
    - `retry_cap`（AC-05）：连续超时 → `attempts == 3`、`degraded == true`、mock 请求数 == 3
    - `rate_limit_backoff`（AC-06）：`429 + Retry-After: 1` → 两次请求间隔 ≥900 ms 且 ≤2 s、总请求数 == 2、`attempts == 2`；另测"声明 5 s、剩余 2 s → 不重试、请求数 == 1"
    - `invalid_schema_repair`（AC-07）：首次非法、第二次合法 → `degraded == false`、`attempts == 2`；两次均非法 → `failure == InvalidSchema`、`degraded == true`、body 为同任务兜底
    - `circuit_open`（AC-08）：3 次最终失败后，冷却期内 mock 请求数 == 0、`degrade_cause == Cooling`；推进 60 s 后请求恢复（请求数 +1）
    - `all_failures_no_panic`（AC-09）：六类各注入 1 次 → 均返回分类结果、`catch_unwind` 为 `Ok`、测试进程退出码 0、兜底耗时 ≤50 ms
    - `degrade_same_shape`（AC-10）：四类任务分别降级 → `body.task() == request.task`、`degraded == true`、`degrade_cause.is_some()`
    - `fallback_insight`（AC-11）：≥1 条、每条 `sources` 非空且属于输入、`confidence.value() ≤ 0.5`
    - `fallback_summary`（AC-12）：有上一版 → `carried_over == true`；无上一版 → `produced == false`
    - `fallback_conclusion`（AC-13）：`conclusion.is_none()`、`notice.is_some()`；引用空 → 结论恒空
    - `cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject` 全绿，用例数 ≥12

- [ ] **L7.3 日志与审计脱敏用例（AC-15）**
  - 追溯：SPEC 4.3-2/4、4.4-4、5.5.1-4/6、8.1 AC-15；DESIGN 2.4.2 AC-15；AC-15
  - 前置：L6.4
  - 并行：⟂ 与 L7.1、L7.2 并行
  - 触达：`crates/infrastructure/tests/llm_logging_redaction.rs`（新增）
  - DoD：
    - 载荷与响应正文均含 `SENSITIVE-TOKEN`；捕获全部 `LlmObserver::log` 与审计 `summary` → 断言不含该串
    - 断言不含 URL 全文（捕获文本中不出现 `http://` / `https://` 开头的完整地址）、不含凭据（夹具假值 `sk-test-0000` 不出现）
    - 日志字段只出现白名单四项（`failure_class/error_code/latency_ms/provider_id`）
    - 审计记录字段齐备：request_id/task/char_count/model/endpoint/elapsed/outcome/degraded/attempts
    - 度量维度只含 endpoint_id/model_id/task/failure_class，不含正文（断言维度值不含 `SENSITIVE-TOKEN`）
    - `cargo test -p meeting-copilot-infrastructure --test llm_logging_redaction` 全绿，用例数 ≥2

## L8 治理、门禁与收口

**任务组目标**：让"文档 + 门禁 + 状态"与代码同批落地，不产生"代码绿、治理红"。

- [ ] **L8.1 依赖许可登记（schemars + HTTPS 挂起结论）**
  - 追溯：SPEC 8.1 AC-17、9.2-4；DESIGN 2.5.2、2.8.4；AC-17
  - 前置：L1.1
  - 并行：⟂ 可与 L2~L7 并行
  - 触达：`docs/compliance/license-review.md`（改；只追加条目，不改既有签字状态行）
  - DoD：
    - 新增条目：`schemars 1.2.3`（MIT、仅用于 Schema 派生、不引入运行时校验引擎、无额外传递依赖）
    - 新增结论行：生产 HTTPS 传输挂起，`llm-https` 为空 feature，锁文件与产物零变化
    - 记录 D1 的选择（L1.1 备注：schemars 落在 domain 还是 domain+infrastructure）
    - `cargo deny check licenses bans sources advisories` 退出码 0
    - 文件内不出现任何凭据、域名、模型名

- [ ] **L8.2 全量门禁执行（AC-16、AC-17）**
  - 追溯：SPEC 8.1 AC-16/AC-17、8.2-1~4；DESIGN 2.9.3-5；AC-16、AC-17
  - 前置：L7.1、L7.2、L7.3
  - 并行：否（收口门禁，串行）
  - 触达：无源码改动（只执行与记录）
  - DoD（逐条粘贴输出到 `docs/progress/b1-status.md` 的证据段）：
    - G-FMT、G-CLIPPY、G-CLIPPY-F、G-LAYER、G-SECRET、G-DENY 全部退出码 0
    - G-TEST：`failed=0`、`passed ≥ 112`（≥80 既有 + ≥32 新增下限）
    - G-COUNT ≥112
    - 仓库检索无真实凭据：`bash scripts/scan-secrets.sh` 通过 + 人工确认夹具只用 `sk-test-0000`
    - 连续 3 次 `cargo test --workspace` 结果一致（`failed=0`）

- [ ] **L8.3 更新进度记录（13.1/13.2 证据 + AC-18 待实测）**
  - 追溯：SPEC 9.2-6、8.1 AC-18、10.1 R1；DESIGN 2.5.3 修改 #6/#11、2.5.4 P1~P10；AC-18
  - 前置：L8.2
  - 并行：⟂ 可与 L9~L11 并行
  - 触达：`docs/progress/b1-status.md`（改；不动其它任务行）
  - DoD：
    - 13.1/13.2 行更新为"网关侧已验证（mock）"，附文件清单与门禁证据摘要
    - AC-18 固定写"待实测（需外部端点与有效凭据）"，并注明"mock 数值不得替代"
    - 三态标注：AC-19/AC-26 静态面/AC-27 机制面在阶段 B 完成后补"离线已判定"；AC-20~AC-25 标"未实测（传输未启用 / 端点未配置）"
    - 登记 M1~M5 与 P9、合规确认的当前状态
    - 不出现任何真实域名、模型名、凭据
    - `git diff docs/progress/b1-status.md` 只涉及 13.1/13.2 与真实联调段

- [ ] **L8.4 变更外移交登记（不执行 `docs/sdd/tasks.md` 勾选）**
  - 追溯：SPEC 9.2-7；DESIGN 2.5.3 修改 #7；本清单 0.3 说明
  - 前置：L8.3
  - 并行：⟂
  - 触达：无（仅在交付说明中登记）
  - DoD：
    - 在交付说明中列出"门禁通过证据包"（命令 + 输出摘要 + commit 短哈希），供发布责任人手工勾选 13.1/13.2
    - 确认 `git status --short` 中 `docs/sdd/` 零改动
    - 若 user 明确授权改 `docs/sdd/tasks.md`，则单独开启新任务，不在本清单内自行执行

---

# 阶段 B：真实联调载体（L9~L12）

> **前置声明**：本阶段零新增依赖、零公网请求；`LLM_LIVE` 未设为 `"1"` 时全部跳过。AC-19、AC-26（静态面）、AC-27（机制面）在本阶段即可判定；AC-20~AC-25 只交付**载体**，数值留给阶段 C。

## L9 运行门与联调配置

**新增文件**：#14 `live/mod.rs`、#15 `live/gate.rs`、#16 `live/config.rs`。

- [ ] **L9.1 实现 `LiveGate` 与 `EnvView`（AC-19 核心）**
  - 追溯：SPEC 11.4-2/3/4、11.5.1、8.3 AC-19；DESIGN 2.6.2、2.6.3-（3）、2.7.2；AC-19
  - 前置：L6.1（需空 feature 声明）
  - 并行：⟂ 与 L10.1 并行
  - 触达：`crates/infrastructure/src/llm/live/mod.rs`（新增）、`crates/infrastructure/src/llm/live/gate.rs`（新增）
  - DoD：
    - `SkipReason` 六值封闭枚举（`LiveDisabled/CredentialMissing/EndpointMissing/ModelMissing/AllowlistMissing/TransportUnavailable`）+ `label()` 中文文案 + `code()` 机器码
    - `LiveDecision::{Run, Skip(SkipReason)}`；`evaluate(&dyn EnvView, transport_available: bool) -> LiveDecision` 为纯函数（无 IO、无 `std::env::`）
    - 判定顺序固定为 2.7.2 表的 1→6；`LLM_LIVE` **严格等于 `"1"`**（`"true"`/`"yes"`/`" 1"`/`"1 "`/空/缺失 → `LiveDisabled`）
    - `ProcessEnv::snapshot()` 为生产实现，启动时读一次并与 `EnvView` 解耦；测试用假视图，**禁止** `set_var`
    - `Skip` 路径不构造传输、不建 socket、不写数值（以探针计数器断言构造次数为 0）
    - `live/mod.rs` 只做模块声明与 `assemble_live()` 的空壳（Run 分支在 L9.2/L13 之后接入）
    - 内联单测 ≥6：开关矩阵（缺失/空/`"0"`/`"true"`/`" 1"`/`"1"`）+ 凭据缺失 + 端点缺失 + 模型缺失 + 白名单缺失 + 传输不可用 + 优先级顺序
    - `cargo test -p meeting-copilot-infrastructure live::gate` 全绿

- [ ] **L9.2 实现 `LiveRunConfig` 与 `SecretString`（AC-19/AC-26）**
  - 追溯：SPEC 11.3.2、11.4-1/6/7、11.5.8、8.3 AC-19/AC-26；DESIGN 2.7.3、2.6.3-（3）3/6/9；AC-19、AC-26
  - 前置：L9.1
  - 并行：否
  - 触达：`crates/infrastructure/src/llm/live/config.rs`（新增）
  - DoD：
    - `SecretString`：`Debug`/`Display` 恒 `***`；**不实现** `Serialize`/`Deserialize`（编译期排除法用例）
    - `LiveRunConfig::try_new(env, deadline_override, samples_override, concurrency_override) -> Result<Self, CopilotError>`
    - `endpoint_host`：仅保留 host，丢弃 path/query/fragment；非法 URL → `ProviderConfigInvalid`
    - 覆盖变量只认三个：`LLM_LIVE_SAMPLES`（1..=200）、`LLM_LIVE_DEADLINE_MS`（100..=8000）、`LLM_LIVE_CONCURRENCY`（1..=3，默认 1）；越界即拒绝启动
    - `call_budget = samples_per_task × 3 × 4`，超过配置上限即拒绝启动（不"先跑再说"）
    - 凭据只在装配时读取一次，不进入 `LlmConfig`、不进入日志/审计/记录
    - 模型名非空且 ≤128 字符；`example.invalid` 之类保留 TLD 只允许出现在测试夹具
    - 内联单测 ≥7：URL 收敛（含 path/query 的输入 → 输出只有 host）、越界覆盖值、样本上限、凭据掩码、非序列化、`endpoint_host` 不含路径
    - `cargo test -p meeting-copilot-infrastructure live::config` 全绿

## L10 采样统计与记录

**新增文件**：#17 `live/stats.rs`、#18 `live/record.rs`。

- [ ] **L10.1 实现 `LiveStats`（纯函数统计，AC-20/21/22/27 口径）**
  - 追溯：SPEC 11.5.2-1、11.5.3-1/3、11.5.4-1/2/3/4、11.7.2-6/7/8/9；DESIGN 2.7.4"统计口径"5 条；AC-20、AC-21、AC-22、AC-27
  - 前置：L9.2（样本结构类型可先定义，但口径需与 config 对齐）
  - 并行：⟂ 与 L11.1 并行（L11.1 可用合成样本先验）
  - 触达：`crates/infrastructure/src/llm/live/stats.rs`（新增）
  - DoD：
    - `Sample{seq, task, attempt, first_token_ms, total_ms, tls_handshake_ms, outcome, failure_class, http_status, retry_after_form, wait_ms, degraded, excluded, reason}`
    - `p(q) = 升序第 ceil(q × n) 个值`（nearest-rank），`q ∈ {0.50, 0.95}`，同时输出 `n`
    - 原生成功率 = 原生通过 Schema 样本数 ÷ 有效请求数；`degraded=true` 计入分母不计入分子，另列"兜底次数"
    - 正常耗时分布**排除** `degraded=true` 与 `excluded` 非空样本，并单列排除计数与原因
    - 失败分布：六类计数 + "未归类"计数（附原因码）；结构违规再拆"非 JSON"/"字段不符"
    - 样本不足：`n<30` 只出 p50 并标"样本不足·不作分布结论"；`n<10` 不出成功率
    - 纯函数、零 IO（函数体内不出现 `std::fs`/`std::net`）
    - 内联单测 ≥8：nearest-rank 边界（n=1/2/30/100）、降级排除、兜底计数、六类 + 未归类、n<30、n<10、结构违规拆分
    - `cargo test -p meeting-copilot-infrastructure live::stats` 全绿

- [ ] **L10.2 实现 `LiveRecorder`（三态记录与原子写，AC-26/27 前置）**
  - 追溯：SPEC 11.6-6、11.7.1/11.7.2、11.5.8-2；DESIGN 2.7.4 文件布局与字段表；AC-23、AC-26、AC-27
  - 前置：L10.1
  - 并行：否
  - 触达：`crates/infrastructure/src/llm/live/record.rs`（新增）
  - DoD：
    - 产物固定写 `.tmp/llm-live/<run-id>/{run.json,samples.jsonl,summary.md}`；`run-id = UTC 时间戳 + commit 短哈希`；**拒绝**任何其它路径（用例断言写入 `docs/` 被拒）
    - 原子写：先写 `.tmp` 再 `rename`；`run.json` 与 `summary.md` 各一次；`samples.jsonl` 逐行追加
    - `run.json` 字段齐备（11 项）：`schema_version/endpoint_host/model/started_at/ended_at/tz_offset/commit_short/config/per_task/sub_status/samples_path/stats_rule/build`
    - `sub_status` 三态：`measured/not_triggered/not_measured + reason_code`；`not_measured` **不得**带数值（构造期拒绝或序列化前断言）
    - `samples.jsonl` 每行 11 字段；`seq` 替代请求编号；不含载荷、响应正文、URL 路径/查询参数、凭据
    - `summary.md` 只由 `run.json` 派生，不作为唯一载体
    - `samples_path` 为相对路径（不含用户名/主目录）
    - 内联单测 ≥6：run-id 生成、原子写、三态序列化、相对路径、非 `.tmp` 路径拒绝、字段完备性断言
    - `cargo test -p meeting-copilot-infrastructure live::record` 全绿

## L11 联调测试与脚本

**新增文件**：#19 `tests/llm_live_offline.rs`、#20 `tests/llm_live.rs`、#21 `scripts/run-llm-live.sh`、#22 `scripts/llm-live-recompute.mjs`。

- [ ] **L11.1 编写离线联调用例（AC-19、AC-26 静态面、AC-27 机制面）**
  - 追溯：SPEC 8.3 AC-19/AC-26/AC-27、11.4-4/6/7、11.6-2/3；DESIGN 2.9.2 三行、2.9.3-1；AC-19、AC-26、AC-27
  - 前置：L9.2、L10.2
  - 并行：⟂ 与 L10.1 的后续细化并行
  - 触达：`crates/infrastructure/tests/llm_live_offline.rs`（新增）
  - DoD（默认执行、零 socket）：
    - `gate_default_off_skips`：未设 `LLM_LIVE` → `Skip(LiveDisabled)`、输出含"未实测（未开启真实联调）"、退出码 0
    - `gate_switch_on_credential_missing_skips`：`LLM_LIVE=1` + 凭据空 → `Skip(CredentialMissing)`、输出含"未实测（凭据缺失）"
    - `gate_precedence_is_deterministic`：多变量同时缺失 → 结果稳定为表中最高优先级原因；连续 3 次一致
    - 传输探针构造次数 == 0；全程无 socket（用例内不出现 `TcpListener`/`TcpStream`）
    - `secret_masked_in_debug_display`：`format!("{:?}")` 与 `{}` 均只含 `***`
    - `record_structurally_cannot_hold_secret`：承载凭据的类型不满足 `Serialize`（排除法编译期断言）
    - `error_text_has_no_secret_fragment`：制造鉴权失败错误 → 错误文本不含凭据任意 ≥8 字符切片，且不回显前缀/长度/后四位
    - `record_fields_complete`（AC-27 机制面）：用合成样本生成 `run.json` → 11 项字段齐备、`samples.jsonl` 行数 == `n`、`samples_path` 相对路径位于 `.tmp/llm-live`
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_offline` 全绿，用例数 ≥7

- [ ] **L11.2 编写 `#[ignore]` 真实采样目标（AC-20~AC-25 载体）**
  - 追溯：SPEC 8.3 AC-20~AC-25、11.5.2~11.5.7、11.6-1/4；DESIGN 2.6.3-（2）采样主流程、2.9.2、2.9.3-3；AC-19~AC-25
  - 前置：L10.2、L11.1
  - 并行：⟂ 与 L11.3、L11.4 并行
  - 触达：`crates/infrastructure/tests/llm_live.rs`（新增）
  - DoD：
    - 全部用例 `#[ignore]`；`cargo test --workspace` **不执行**它们
    - 入口先 `evaluate(&ProcessEnv::snapshot(), cfg!(feature = "llm-https"))`；`Skip` 时打印"未实测（原因） code=…"并正常返回（退出码 0），**不构造传输**
    - `run_gate_skips_without_live`（AC-19 L 面）：无 `LLM_LIVE` 时不外发、不写数值
    - `first_token_latency_p50_p95`（AC-20）：每任务 ≥30（不足 → `not_measured`）；记录 p50/p95 + host/model/日期时区/样本量 + "达标/未达标"字段
    - `total_latency_four_tasks`（AC-21）：四类各 ≥30；降级样本排除并单列；预算判定 ∈ {可行, 不可行, 边界}；"不可行"时必须写"摘要兜底出卡"初判依据字段
    - `schema_conformance_rate`（AC-22）：每类 ≥10；成功率分子不含兜底；失败按六类 + 未归类；结构违规拆两类
    - `rate_limit_observation`（AC-23）：出现 429 → `retry_after_form ∈ {seconds,http_date,missing,invalid}`、`wait_ms ≤2000`、是否越过截止、分类 `RateLimited`；未出现 → `not_triggered` + 采样窗口 + 请求数
    - `deadline_forced_timeout`（AC-24）：截止压至 ≤300 ms → 分类 `Timeout`、`|返回时刻-截止时刻| ≤100 ms`（容差 200 ms）、无结果体交付、记录含"人为压低截止时间"标注
    - `schema_violation_probe_once`（AC-25）：1 次对照请求；违规 → `attempts ≤ 2`、同类型兜底 + `degraded=true` + `Failed(InvalidSchema)`；记录含"人为构造的违规请求"
    - 鉴权失败（401/403）→ 中止采样并标"未实测（凭据无效）"，不继续消耗配额
    - 端点不兼容 → 记"未归类：端点不兼容（待评估）"，不计入 `InvalidSchema` 分布
    - 无 `llm-https` 时本文件仍编译通过（TLS 调用点 `cfg` 门控，运行时 `Skip(TransportUnavailable)`）
    - `cargo test -p meeting-copilot-infrastructure --test llm_live -- --ignored --list` 可列出 ≥7 个用例；不带 `--ignored` 时执行 0 个

- [ ] **L11.3 编写联调执行脚本（前置门 + 串行执行 + 四泄漏面抓取）**
  - 追溯：SPEC 11.4-5/9、11.6-1/3/6、11.5.8、8.3 AC-19/AC-26；DESIGN 2.6.1 边界表、2.7.2、2.9.4；AC-19、AC-26
  - 前置：L11.2
  - 并行：⟂ 与 L11.4 并行
  - 触达：`scripts/run-llm-live.sh`（新增；bash）
  - DoD：
    - 前置段：检查 `LLM_LIVE == "1"` 与三项变量；任一不满足 → 打印"未实测（原因）"并 `exit 0`，**不发生构建与网络**
    - 前置段：检查 `cargo deny` 与 `--features llm-https` 可用性；不可用 → 打印"未实测（传输未启用）"并 `exit 0`
    - 执行：`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --ignored --test-threads=1`
    - 结束段四泄漏面抓取：日志文件、`.tmp/llm-live/<run-id>/run.json` + `samples.jsonl`、错误输出、进程命令行；断言不含凭据值与凭据编号（只输出命中次数，**不回显命中内容**）
    - 断言 `git status --short` 无新增凭据相关文件；调用 `bash scripts/scan-secrets.sh`
    - 脚本不接收凭据作为位置参数（不使用 `$1`/`$2` 承载凭据）；不写临时凭据文件
    - `bash scripts/run-llm-live.sh` 在未设 `LLM_LIVE` 的环境下退出码 0，输出含"未实测（未开启真实联调）"，且无任何出网
    - `shellcheck` 可用时零 error（不可用则跳过并在输出说明）

- [ ] **L11.4 编写统计重算脚本（AC-27 复核入口）**
  - 追溯：SPEC 11.5.9-3、11.7.2-7；DESIGN 2.7.4"统计口径"、2.9.2 AC-27；AC-27
  - 前置：L10.2
  - 并行：⟂ 与 L11.3 并行
  - 触达：`scripts/llm-live-recompute.mjs`（新增；Node 26 / 零依赖）
  - DoD：
    - 输入 `samples.jsonl` 路径（默认取最新 `.tmp/llm-live/<run-id>/`），输出每任务 p50/p95 与 `n`
    - 口径与 `LiveStats` 完全一致（nearest-rank、降级样本排除、兜底计数、未归类）
    - 只读文件，不写 `.tmp` 以外位置；不打印任何样本正文
    - `node scripts/llm-live-recompute.mjs --self-test` 用内置合成样本自检（预期值硬编码），退出码 0
    - 与 Rust 侧同口径：用同一份合成样本，Node 输出与 `LiveStats` 输出差 ≤0.1 ms

## L12 许可门准备

- [ ] **L12.1 登记 TLS 传输许可条目（Phase B 前置，不引入依赖）**
  - 追溯：SPEC 10.1 R2、8.1 AC-17；DESIGN 2.5.2、2.8.4、2.5.3 修改 #9；AC-17、AC-20~AC-25（阻塞前置）
  - 前置：L8.1
  - 并行：⟂ 与 L9~L11 并行
  - 触达：`docs/compliance/license-review.md`（改；第 2 节新增条目，既有十项与签字状态行不改）
  - DoD：
    - 新增"TLS 传输组件（`native-tls`，Phase B）"条目：许可证（MIT/Apache-2.0）、平台绑定（Windows schannel / macOS Security.framework）、"不打包 OpenSSL/ring"结论
    - 明确"未签字时 feature 为空、依赖不写入 `Cargo.toml`、锁文件零变化"
    - 记录备选 `rustls + ring` 仅在系统 TLS 不可用时评估
    - `cargo deny check licenses bans sources advisories` 仍退出码 0
    - 文件内不出现域名、模型名、凭据

- [ ] **L12.2 验证空 feature 的构建与告警纪律**
  - 追溯：SPEC 8.1 AC-17；DESIGN 2.8.1"Phase A 的 feature 声明方式"、2.9.3-3/4；AC-17、AC-19
  - 前置：L6.1、L11.2
  - 并行：否
  - 触达：`crates/infrastructure/Cargo.toml`（只读校验）
  - DoD：
    - `cargo build -p meeting-copilot-infrastructure` 与 `--features llm-https` 均退出码 0
    - G-CLIPPY-F 退出码 0，无 `unexpected_cfgs` 告警
    - `cargo tree -p meeting-copilot-infrastructure --features llm-https 2>&1` 中不出现 `native-tls`/`openssl`/`schannel`/`rustls`（依赖闭包零变化）
    - `cargo metadata --format-version 1` 输出的 `llm-https` feature 依赖数组为空
    - 阶段 B 收口：G-FMT / G-CLIPPY / G-CLIPPY-F / G-TEST（`passed ≥ 119`）/ G-LAYER / G-SECRET / G-DENY 全绿

---

# 阶段 C：真实实测（L13，外部阻塞）

## L13 真实传输与实测记录（⛔ 全部阻塞）

> **本阶段全部 ⛔**：需 `native-tls` 许可双签（P9）+ M1~M5 + 合规条款确认。未满足时**不得**开始，也不得用 mock 数据顶替。以下任务只登记，不预先执行。

- [ ] **L13.1 ⛔ 实现 TLS 传输 `TlsTransport`（复用 `Http1Codec`）**
  - 追溯：SPEC 4.3-5、11.4-1/7、8.3 AC-20~AC-25；DESIGN 2.2.2-E、2.8.2、2.8.4 Phase B；AC-20~AC-25
  - 前置：L12.1 签字 + L12.2 + L3.1
  - 并行：否
  - 触达：`crates/infrastructure/src/llm/transport_https.rs`（新增，#23）
  - DoD：
    - 实现 `HttpTransport` 全两方法，错误仍映射到既有 `TransportError` 十变体（不新增变体）
    - `native_tls::TlsConnector`：SNI=host、最低 TLS 1.2、系统信任根；**禁止**跳过证书校验、禁止自定义 CA
    - 不跟随重定向、不读系统代理、不做 HTTP/2、不缓存 DNS、不复用连接
    - `HttpRequest` 的 `Debug` 遮蔽 `Authorization` 与 body（沿用 L3.1）
    - 关闭连接时 `StreamReader::drop` 立即释放
    - `cargo build -p meeting-copilot-infrastructure --features llm-https` 退出码 0
    - G-CLIPPY-F、G-DENY、G-SECRET 全绿

- [ ] **L13.2 ⛔ 切换 feature 为真实依赖并复跑许可门**
  - 追溯：DESIGN 2.5.3 修改 #10、2.8.1；SPEC 10.1 R2；AC-17
  - 前置：L13.1 + 许可双签
  - 并行：否
  - 触达：`crates/infrastructure/Cargo.toml`（`llm-https = ["dep:native-tls"]` + `native-tls` optional）
  - DoD：
    - 仅在签字记录存在时修改；`git diff` 只含该 feature 与 optional 依赖两处
    - `cargo deny check licenses bans sources advisories` 退出码 0（若出现新许可证，先回 L12.1 补登记）
    - 记录新增下载包清单与平台绑定（不打包 OpenSSL/ring）
    - `cargo build --workspace` 默认 feature 下依赖闭包不含 `native-tls`

- [ ] **L13.3 ⛔ 执行真实采样并产出三态记录（M1~M5 齐备后）**
  - 追溯：SPEC 11.5.2~11.5.7、11.7、11.8；DESIGN 2.6.3-（2）、2.9.4；AC-20~AC-26
  - 前置：L13.2 + M1/M2/M3/M4/M5 + 合规确认
  - 并行：否（串行采样，`--test-threads=1`、`max_concurrency=1`）
  - 触达：无源码改动；产物 `.tmp/llm-live/<run-id>/`
  - DoD：
    - 先 `LLM_LIVE_SAMPLES=1` 冒烟：验证连通、凭据有效、端点兼容；鉴权失败即止
    - 再按预算采样：四类任务各 ≥30（不足则按 `n<30` 降级表述）；先跑 `epoch` 预热样本不计入统计（DESIGN 2.8.2）
    - `run.json`/`samples.jsonl` 完整；`sub_status` 逐子项三态
    - 失败分类分布含六类 + "未归类"；结构违规拆"非 JSON"/"字段不符"
    - AC-23 未出现 429 → `not_triggered` + 采样窗口 + 请求数（不得写"通过"）
    - AC-24/AC-25 记录必须带"人为压低截止时间"/"人为构造的违规请求"标注
    - `bash scripts/run-llm-live.sh` 退出码 0；四泄漏面断言全通过；`git status --short` 零新增凭据文件

- [ ] **L13.4 ⛔ 记录复核与移交 13.4（不写 `docs/`）**
  - 追溯：SPEC 11.5.9、11.7.2、11.8-6；DESIGN 2.9.4-6/7、2.9.5；AC-27
  - 前置：L13.3
  - 并行：否
  - 触达：无（复核只读；移交内容写入交付说明与 `docs/progress/b1-status.md` 的联调段）
  - DoD：
    - `run.json` 11 项字段齐备；`samples.jsonl` 行数 == `n`
    - `node scripts/llm-live-recompute.mjs` 重算 p50/p95 与记录差 ≤0.1 ms
    - `samples_path` 为相对路径且位于 `.tmp/llm-live`
    - 三态结论移交 13.4：AC-20/21/22/27 为"实测"或如实标注；AC-18 状态变更**不由本清单执行**（属 13.4）
    - 不写 `docs/baseline/*`、不改 `docs/sdd/*.md`

---

# 阶段 D：DeepSeek 锁定 + TLS + 阶段 C 实测（L14~L22）

> **前置声明**：
> 1. 本阶段**不重开** L1~L13；阶段 A/B 的勾选状态即为终态（见 0.1.1）。L12.2 的空 feature 断言、L13.3/L13.4 的执行前置，其 v1.2 变更只在附录 G 登记。
> 2. 新增依赖**仅** `native-tls`（Windows/macOS optional，`llm-https` 门控）；默认构建与 `cargo test --workspace` 的依赖闭包零增量。
> 3. AC-28~AC-32 是**补充门槛项**（SPEC 8.4 首段）：全部可断网判定，不计入 8.1 完成判据；任一项不通过只关闭真实联调路径（fail-closed），不改变 AC-01~AC-27 的任何结论。
> 4. 本阶段**只有一处**政策判定（`live/gate.rs::admit`）与**一个**计数点（`live/budget.rs` 的 `BudgetedTransport`）；脚本层只做存在性提示，不做业务判定。

## L14 服务商锁定运行门（D1 · AC-28/29/30）

**任务组目标**：把"允许对哪个 host、用哪个模型、走不走 HTTPS"从口头约定变成断网可判定的纯函数闸门；拒绝路径上既无 socket、也无凭据值进入内存对象。

- [ ] **L14.1 实现 `live/policy.rs`（host 精确锁定 + scheme + 模型逐字节比对）（★）**
  - 追溯：AC-28、AC-29、AC-30；SPEC 11.9.1-2/3/4、11.9.2-1/2、11.9.3-1；DESIGN 2.10.1、2.10.5、2.10.6、2.10.7、2.10.9；新增文件 #24
  - 前置：L9.1（既有 `EnvView`）、L9.2（既有 `host_from_url`，只读复用其解析片段）
  - 并行：⟂ Q1：与 L15.1、L16.1、L17.1、L18.1 并行（不同文件）
  - 触达：`crates/infrastructure/src/llm/live/policy.rs`（新增）、`crates/infrastructure/src/llm/live/mod.rs`（改，导出模块 #22）
  - DoD（内联 `mod tests`；`cargo test -p meeting-copilot-infrastructure live::policy` 退出码 0）：
    - `HostVerdict::{Allowed, NotAllowed}`；`host_is_locked(raw_url: &str, allowed: &[&str]) -> HostVerdict`
    - 解析顺序：取 `scheme://` 后的 authority → 含 `@`（userinfo）→ `NotAllowed`；端口存在且 `!= 443` → `NotAllowed`；取 host 后**唯一**归一化为 `to_ascii_lowercase()`；比较为 `host == allowed[i]`（逐字节）
    - 拒绝形态全部落 `NotAllowed`：子域 `x.<host>`、后缀拼接 `<host>.<other>`、前缀拼接 `<prefix><host>`、国别域变体、相似拼写、其它域名、userinfo、非 443 端口、尾随点、IP 字面量、punycode/IDN
    - `SchemeVerdict::{Https, InsecureRejected, Unparsable}`；`scheme_verdict(raw_url)`：`http://` → `InsecureRejected`；`https://` → `Https`
    - `model_confirmed(actual: &str, confirmed: Option<&str>) -> bool`：逐字节（`as_bytes()`）；**不** trim、不做大小写归一化；`confirmed=None` → `false`
    - 内联用例 ≥3：`host_is_locked_is_exact_not_suffix`（构造 `evil.` + 允许 host → `NotAllowed`）、`scheme_verdict_three_states`、`model_confirmed_five_values`（空/仅空白/未确认候选占位/不相等/完全相同）
    - `rg -n "DomainPattern::matches" crates/infrastructure/src/llm/live/policy.rs` 退出码 1（不得复用该匹配语义）
    - 允许集合以 `const DEEPSEEK_HOSTS: &[&str] = &["api.deepseek.com"]` 单点表达（SPEC 11.9.1-3 唯一放行的具体 host）；同文件**不得**出现"host + 路径"拼接的完整 URL 字面量（`rg -n "api\\.deepseek\\.com/"` 退出码 1）
    - `cargo clippy -p meeting-copilot-infrastructure --all-targets -- -D warnings` 退出码 0
  - 备注〔D4〕：`api.deepseek.com` 是 SPEC 11.9.1-3 明确允许出现的**唯一**服务商 host；除该常量外，代码/注释/夹具不得出现任何其它服务商域名、区域或价格。

- [ ] **L14.2 实现 `gate.rs::admit` 政策闸（固定 11 步）（★）**
  - 追溯：AC-28、AC-29、AC-30、AC-31；SPEC 11.9.1-1、11.9.2-1、11.9.3-1、11.9.4-4；DESIGN 2.10.3、2.10.4、2.10.9、2.10.10、2.12.1、2.12.3；修改文件 #12、#22
  - 前置：L14.1（`call_limit` 只由 `samples` 与 `LLM_LIVE_CALL_CAP` 计算，不依赖 `BudgetLedger` 实例）
  - 并行：否（关键路径汇聚点）
  - 触达：`crates/infrastructure/src/llm/live/gate.rs`（改，只新增）、`crates/infrastructure/src/llm/live/mod.rs`（导出 `admit`/`Admission`/`GateReject`）
  - DoD：
    - 新增 `GateReject::{EndpointNotAllowed, ModelNotConfirmed, InsecureTransportRejected, BudgetNotConfirmed, BudgetExhausted}`；`code()` 分别返回 `endpoint_not_allowed` / `model_not_confirmed` / `insecure_transport_rejected` / `budget_not_confirmed` / `budget_exhausted`
    - `GateReject::to_error()` 统一 `CopilotError::ProviderConfigInvalid`（`E-PROVIDER-009`）；`detail` 只含字段名 + 标识字符串，不含任何配置值/URL 全文/凭据/载荷（用例断言 detail 不含 `SENSITIVE-TOKEN`）
    - `Admission::{Admit(AdmissionTicket), Skip(SkipReason), Reject(GateReject)}`；`AdmissionTicket{ endpoint_host: String, scheme: Scheme, call_limit: u64 }`（不含凭据）
    - `admit(env: &dyn EnvView, transport_available: bool) -> Admission`；顺序严格按 DESIGN 2.10.4 的 11 步：开关 → 端点存在 → host 精确集合 → scheme → 凭据判空 → 模型非空 → 模型确认 → 白名单 → 传输可用 → 预算已确认 → `Admit`
    - 第 3/4 步（host、scheme）**必须**前置于凭据值读取；`Admission::Reject` / `Skip` 路径上 `SecretString` 构造次数为 0（由 L14.3 的 `CredentialProbe` 断言）
    - `ProcessEnv::snapshot()` 的 key 列表**只追加** `LLM_MODEL_CONFIRMED`、`LLM_LIVE_CALL_CAP`（既有 5 个 key 的读取行为不变）
    - `evaluate` 与 6 个 `SkipReason` 的 `code()`/`label()`/判定顺序**零改动**（`git diff` 该文件只出现新增行；既有 6 个 SkipReason 断言用例不修改）
    - 内联用例 ≥4：`admit_order_is_deterministic_inline`（连续 3 次同结果）、`host_reject_precedes_credential`、`scheme_reject_precedes_credential`、`reject_has_no_ticket`
    - `cargo test -p meeting-copilot-infrastructure live::gate` 退出码 0（既有 + 新增全绿）
    - `cargo clippy -p meeting-copilot-infrastructure --all-targets --features llm-https -- -D warnings` 退出码 0

- [ ] **L14.3 编写 `tests/llm_live_gate.rs`（AC-28/29/30 离线用例 + 三探针）（★）**
  - 追溯：AC-28、AC-29、AC-30；SPEC 8.4 判定表、11.9.1-3、11.9.2-1、11.9.3-1；DESIGN 2.13.2、2.13.3；新增文件 #27
  - 前置：L14.2（AC-31 的 `budget_cap_enforced` 由 L15.3 在同文件追加，届时需 L15.2）
  - 并行：⟂ Q3：与 L16.4、L18.2、L18.3、L19.1 并行（不同文件）
  - 触达：`crates/infrastructure/tests/llm_live_gate.rs`（新增）
  - DoD（**全部零 socket、零真实凭据**）：
    - 测试脚手架（仅本文件内）：`CountingEnv`（统计各变量 `get` 次数）、`CountingTransport`（构造次数 / `send` / `open_stream` 次数）、`CredentialProbe`（`SecretString::new` 与 `expose` 计数）
    - `endpoint_host_locked`（AC-28）：①同域无路径、含路径 → `Admit`；②DESIGN 2.10.5 矩阵的 11 种拒绝形态 → `Reject(EndpointNotAllowed)`；③每个拒绝断言 `transports_constructed == 0`、`sockets_opened == 0`、`credential_reads == 0`
    - `model_explicitly_confirmed`（AC-29）：五种取值（空 / 仅空白 / 未确认候选占位 / 与确认值不逐字节相同 / 完全相同）→ 前四 `Reject(ModelNotConfirmed)` 且 `requests == 0`，第五 `Admit`
    - `insecure_transport_rejected`（AC-30）：`http://<host>` → `Reject(InsecureTransportRejected)` 且 `sockets_opened == 0`；`https://<host>` → `Admit`（不判连通）
    - `admit_order_is_deterministic`：同一输入连续 3 次得到同一 `Admission`（用 `matches!` 断言变体与 code）
    - 文件内 `rg -c "TcpListener|TcpStream" crates/infrastructure/tests/llm_live_gate.rs` 退出码 1（零 socket 的机器判定）
    - 文件内不出现真实凭据与真实模型名；夹具只用 `sk-test-0000` 与占位模型名
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_gate` 退出码 0，用例数 ≥5（此时 AC-31 用例尚在 L15.3 补齐）

## L15 预算账本、触顶与错误通路（D2 · AC-31）

**任务组目标**：把"能花多少钱"变成**唯一计数点**的结构性保证：重试天然计入、触顶即停、已得样本不删改。

- [ ] **L15.1 实现 `BudgetLedger` 与 `Permit`（★）**
  - 追溯：AC-31；SPEC 11.9.4-1/2/3/4/5/6；DESIGN 2.10.8、2.10.9、2.13.2-AC-31；新增文件 #25
  - 前置：L9.2（既有配置类型）
  - 并行：⟂ Q1：与 L14.1、L16.1、L17.1、L18.1 并行
  - 触达：`crates/infrastructure/src/llm/live/budget.rs`（新增）、`live/mod.rs`（导出）
  - DoD（内联 `mod tests`）：
    - `BudgetLedger{ used: AtomicU64, limit: u64 }`；`new(limit)` / `try_acquire(&self) -> Result<Permit, BudgetExhausted>` / `used()` / `limit()`
    - `try_acquire` 使用比较-交换循环：`used < limit` 才 `fetch_add(1)`，**不 overshoot**；第 `limit+1` 次返回 `Err(BudgetExhausted)`
    - `limit == 0` → 首次即 `Err`；`used()` 恒 ≤ `limit()`
    - `Permit` **不回收**：`drop(Permit)` 后 `used()` 不减少（防止"失败不计费"的错误口径）
    - `BudgetExhausted` 为 `budget.rs` 内的局部标识（不依赖 `gate`，保持 DESIGN 2.10.3 的依赖方向）；由 `gate::admit` 映射为 `GateReject::BudgetExhausted`
    - 内联用例 ≥3：`ledger_never_exceeds_limit`（8 线程 × 100 次 `try_acquire`，成功次数恰为 `limit`）、`permit_drop_does_not_refund`、`zero_limit_rejects`
    - `cargo test -p meeting-copilot-infrastructure live::budget` 退出码 0
  - 备注〔D5〕：DESIGN 2.10.9 把 `try_acquire` 的返回类型写作 `Result<Permit, GateReject>`，但 2.10.3 要求 `budget` **不依赖** `gate`。落地采用"`budget` 定义局部 `BudgetExhausted`，`gate` 侧映射"以消除循环依赖；语义不变，登记为受控偏差。

- [ ] **L15.2 实现 `BudgetedTransport` 与 `BudgetExhausted` 错误通路（★）**
  - 追溯：AC-31；SPEC 11.9.4-2/3/5；DESIGN 2.10.8"唯一计数点/触顶语义"、2.13.2-AC-31；修改文件 #14、#15、#16、#23
  - 前置：L15.1、L3.2（既有 `HttpTransport`）、L6.2/L6.3（既有网关编排）
  - 并行：否（与 L16.2 同写 `transport.rs`，本任务先落）
  - 触达：`live/budget.rs`（追加 `BudgetedTransport`）、`crates/infrastructure/src/llm/transport.rs`、`degrade.rs`、`gateway.rs`、`crates/domain/src/model/llm.rs`
  - DoD：
    - `TransportError` 新增 `BudgetExhausted` 变体（既有 10 变体与映射保持不变）
    - `degrade::classify(BudgetExhausted) == None`（与 `Cancelled` 同类：非外部故障）；既有 8 条 `TransportError → FailureClass` 断言零改动
    - `DegradeCause` 新增 `BudgetExhausted`；`FailureClass::ALL.len() == 6` 不变；`crates/domain` 内联"该变体不进入六类分布"断言
    - `BudgetedTransport::new(inner: Arc<dyn HttpTransport>, ledger: Arc<BudgetLedger>)`；`send` 与 `open_stream` **入口**先 `try_acquire`，失败立即返回 `TransportError::BudgetExhausted`，**不调用 inner**（用例断言 `inner_send_calls` 不增）
    - 一次性与流式 `Err(error)` 分支最前判 `error == BudgetExhausted` → `StructuredOutcome{degraded:true, failure:None, degrade_cause:Some(BudgetExhausted), body: 同任务兜底}`；`gateway.rs` 不出现任何"是否联调"分支
    - 触顶不累加端点失败计数（不触发冷却）
    - `FakeTransport` 脚本化"首次超时 + 第二次成功" → `used == 2`；`limit = 1` 重跑 → 第二次在装饰器入口短路、`used == 1`
    - 内联用例 ≥2（`BudgetedTransport` 层）：`retry_counts_through_single_point`、`exhausted_short_circuits_before_inner`
    - `cargo test -p meeting-copilot-infrastructure` 退出码 0；`cargo test -p meeting-copilot-domain` 退出码 0

- [ ] **L15.3 补齐 `budget_cap_enforced` 用例（AC-31）**
  - 追溯：AC-31；SPEC 8.4-AC-31、11.9.4-3/4；DESIGN 2.13.2-AC-31、2.13.3
  - 前置：L15.2、L14.3（同文件已存在）
  - 并行：否（同写 `tests/llm_live_gate.rs`）
  - 触达：`crates/infrastructure/tests/llm_live_gate.rs`（追加）
  - DoD：
    - `budget_cap_enforced`：①`samples=30`、`cap=200` → `call_limit == 200`；`cap=1000` → `call_limit == 360`（`min(360, cap)`）；②`BudgetLedger` 第 `limit+1` 次 → `Err`；③经 `BudgetedTransport` 驱动到上限后继续提交 → 实际 `send` 次数 `== limit`；④缺 `LLM_LIVE_CALL_CAP` → `Reject(BudgetNotConfirmed)` 且 `requests == 0`；⑤触顶后合成样本 `.jsonl` 行数不减少（只追加"非样本事件"）
    - 用例不连接任何真实端点（`CountingTransport` 或 `FakeTransport` 即可）
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_gate` 退出码 0，用例数 ≥6

## L16 TLS 传输与 feature 真实依赖（D3 · AC-30）

**任务组目标**：把 `llm-https` 从空声明升级为真实的系统 TLS 依赖，同时保持"默认构建零增量、Linux 不拉 OpenSSL、协议只有一份实现"。

- [ ] **L16.1 把 `llm-https` 改为真实依赖（Windows/macOS 条件）（★）**
  - 追溯：AC-30、AC-17；SPEC 11.9.3-2/3/4；DESIGN 2.11.1、2.11.2、2.11.7；修改文件 #17
  - 前置：L12.1（既有许可条目已存在）；L20.1 须在同批完成登记（不阻塞编译）
  - 并行：⟂ Q1：与 L14.1、L15.1、L17.1 并行
  - 触达：`crates/infrastructure/Cargo.toml`（改：`llm-https = ["dep:native-tls"]` + 两个 target 段）
  - DoD：
    - `[features] llm-https = ["dep:native-tls"]`（替换既有第 16 行的空数组 `[]`）
    - `native-tls = { version = "0.2", optional = true }` **只**出现在 `[target.'cfg(windows)'.dependencies]` 与 `[target.'cfg(target_os = "macos")'.dependencies]`；**不得**出现在通用 `[dependencies]`
    - 默认构建零增量：`cargo tree -p meeting-copilot-infrastructure -e normal | grep -c native-tls` == 0（退出码 1 视为通过）
    - Windows：`cargo build -p meeting-copilot-infrastructure --features llm-https` 退出码 0
    - `cargo tree -p meeting-copilot-infrastructure --features llm-https -e normal | grep -c openssl` == 0；`grep -c openssl-sys` == 0
    - Linux 探测：`cargo tree -p meeting-copilot-infrastructure --features llm-https --target x86_64-unknown-linux-gnu -e normal` 的输出中 `openssl` 计数 == 0（允许该命令因 Linux target 未声明 `native-tls` 而失败；**失败输出也不得含 `openssl`**）
    - `Cargo.lock` 只新增 `native-tls` 及其平台绑定；包清单差异写入 L20.1
    - `cargo deny check licenses bans sources advisories` 退出码 0
  - 备注〔D6〕：Linux 上 `--features llm-https` **预期构建失败**（fail-closed），不是缺陷；禁止通过把 `native-tls` 加入通用 target 来"修好"Linux（会在 Linux 拉入 OpenSSL）。

- [ ] **L16.2 泛化 `transport.rs` 的 I/O 辅助并新增 `json_post_secure`（★）**
  - 追溯：AC-30（对称检查）、AC-04/AC-09（不回归）；DESIGN 2.10.7、2.11.4；修改文件 #14
  - 前置：L15.2（同文件，先落 `BudgetExhausted`）
  - 并行：否（同文件串行）
  - 触达：`crates/infrastructure/src/llm/transport.rs`
  - DoD：
    - 新增内部 `TimedIo` trait（`set_read_timeout` / `set_write_timeout`），`TcpStream` 实现一次；`TlsStream<TcpStream>` 的实现留给 L16.3
    - 写请求体、读整体、逐行读头、超长判定四个辅助泛化为 `W: Write` / `R: Read`；`Http1Codec` 的编码/解析逻辑与 `codec_contract` 的 3 组断言**零改动**
    - 新增 `HttpRequest::json_post_secure(...)`（`secure = true`）；既有 `json_post` 保持 `secure = false`，行为与签名不变
    - `LoopbackTransport` 的 `secure == true → TransportError::TlsFailure` 检查零改动（第二层防线保留）
    - `cargo test -p meeting-copilot-infrastructure transport::` 退出码 0
    - `cargo test -p meeting-copilot-infrastructure --test llm_gateway_mock` 与 `--test llm_degrade_inject` 退出码 0（协议与分类不回归）
    - `cargo clippy -p meeting-copilot-infrastructure --all-targets -- -D warnings` 退出码 0

- [ ] **L16.3 实现 `transport_tls.rs`（`TlsTransport` / `TlsStreamReader`）（★）**
  - 追溯：AC-30、AC-20~AC-25（运行期）、P11；SPEC 11.9.3-1/2/5；DESIGN 2.11.3、2.11.4、2.11.5、2.11.6；新增文件 #26
  - 前置：L16.1、L16.2
  - 并行：否（关键路径）
  - 触达：`crates/infrastructure/src/llm/transport_tls.rs`（新增，模块级 `#[cfg(feature = "llm-https")]`）、`llm/mod.rs`（条件注册模块）
  - DoD：
    - 实现 `HttpTransport::{send, open_stream}`；错误全部落在既有 `TransportError` 变体（不新增）
    - `HttpRequest.secure == false` → `TlsFailure`，**且不建立 TCP**（用例以 listener 连接数 0 断言）
    - host 不精确命中 `DEEPSEEK_HOSTS` → `Malformed`（发出前拒绝，复用 `policy::host_is_locked`）
    - 连接：`TcpStream::connect_timeout` 分段 ≤50 ms 复查 deadline；底层 `TcpStream` 读/写超时 = 剩余预算；每 ≤50 ms 复查 `deadline` 与 `CancelToken`
    - TLS 基线：`TlsConnector::builder().min_protocol_version(Some(Tlsv12))`；**不**调用 `danger_accept_invalid_certs` / `danger_accept_invalid_hostnames`；**不**设置自定义 `Certificate`/`Identity`；SNI = 已校验 host
    - 握手失败/证书不受信/协议版本不符 → `TlsFailure`；握手超时 → `ConnectTimeout`；对端关闭 → `PeerClosed`；超长 → `TooLarge`
    - 协议编解码与读写循环**复用** L16.2 的泛型函数（不复制 codec）；每样本冷连接（不复用连接、不缓存 DNS、不跟随 3xx、不读系统代理）
    - `TlsStreamReader::drop` 立即释放底层连接
    - 提供只读策略结构（`TLS_MIN_VERSION` / `ALLOW_INVALID_CERTS=false` / `CUSTOM_CA=false` / `SNI=host`）供 L16.4 离线断言，不需真实握手
    - `cargo build -p meeting-copilot-infrastructure --features llm-https` 退出码 0
    - `cargo clippy -p meeting-copilot-infrastructure --all-targets --features llm-https -- -D warnings` 退出码 0
    - 未启用 feature：`cargo build -p meeting-copilot-infrastructure` 退出码 0（文件不参与编译）

- [ ] **L16.4 编写 `tests/llm_transport_tls.rs`（离线负向用例）（★）**
  - 追溯：AC-30、P11；SPEC 11.9.3-2；DESIGN 2.11.6 #1~#7、2.13.3；新增文件 #28
  - 前置：L16.3
  - 并行：⟂ Q3：与 L14.3、L18.2、L18.3、L19.1 并行
  - 触达：`crates/infrastructure/tests/llm_transport_tls.rs`（新增，文件首行 `#![cfg(feature = "llm-https")]`）
  - DoD（全部只连 `127.0.0.1`）：
    - `tls_connector_policy_is_pinned`：最低 TLS 1.2、无 danger 开关、无自定义 CA、SNI=host（断言只读策略结构）
    - `plaintext_peer_is_rejected`：listener 回写明文 HTTP → `Err(TlsFailure)`，且不会把明文响应当合法响应
    - `garbage_peer_is_rejected`：listener 回写随机字节 → `Err(TlsFailure)`
    - `connect_refused_is_classified`：连接已关闭端口 → `Err(ConnectRefused)`
    - `handshake_timeout_respects_deadline`：listener `accept` 后不发字节 → `Err(ConnectTimeout)`，返回时刻 ≤ `deadline + 200 ms`
    - `insecure_request_is_rejected_by_tls_transport`：直接传 `secure = false` → `Err(TlsFailure)` 且 listener 连接数 `0`
    - `codec_contract_is_shared`：内存 buffer 上跑既有 codec 断言，与回环结果一致
    - **不引入任何证书/私钥夹具**：`rg -n "PRIVATE KEY|-----BEGIN" crates/infrastructure/tests/llm_transport_tls.rs` 退出码 1
    - `cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_transport_tls` 退出码 0，用例数 ≥6
    - 未启用 feature：`cargo test -p meeting-copilot-infrastructure --test llm_transport_tls` 退出码 0 且执行 0 个用例（`--list` 输出为空）

## L17 六载体泄漏扫描与候选标注（D4 · AC-32/AC-29）

**任务组目标**：补上既有 `scan-secrets.sh` 排除 `*.md`、不扫 `.tmp` 的缺口，并把"候选模型名不得写成事实"变成可执行检查。

- [ ] **L17.1 实现 `scripts/scan-secrets-docs.sh`（★）**
  - 追溯：AC-32；SPEC 8.4-AC-32、11.9.5-2、11.5.8-2；DESIGN 2.13.4；新增文件 #29
  - 前置：L10.2（既有记录字段表）、L18.1（v2 键表；键表断言可在 L18.3 前补齐）
  - 并行：⟂ Q1：与 L14.1、L15.1、L16.1、L18.1 并行
  - 触达：`scripts/scan-secrets-docs.sh`（新增，bash）
  - DoD：
    - 用法：`--all-md`（全仓库 `*.md`，含 `.codeartsdoer/specs/**` 与 `docs/**`）/ `--artifacts [DIR]`（默认 `.tmp/llm-live`）/ 两者可并用 / `--self-test`
    - 规则 7 条（结构模式，非真实值）：凭据完整 `sk-[A-Za-z0-9_-]{16,}`；凭据遮蔽形态；凭据末四位（需同行含 `key|凭据|credential`）；Tracking ID 完整 UUID 形态（需同行含 `tracking|credential|key id|凭据编号`）；Tracking ID 片段（≥4 位连续 + 上下文词）；完整 URL 含查询参数 `https?://[^ \t\"']+\?[^ \t\"']+`；`danger_accept_invalid|--insecure|CERT_NONE`
    - 输出纪律：命中时**只**输出 `文件:行号`（管道 `cut -d: -f1,2`），**绝不**回显命中内容或上下文行
    - 退出码（强制）：`0` = 0 命中；`1` = 有命中；`2` = 扫描面缺失或不可读（`--all-md` 时 `git ls-files '*.md'` 为空；或"已声明实测"但 artifacts 目录不可读）。**禁止把 2 当 0**
    - `.tmp/llm-live` 不存在且未声明实测 → `--artifacts` 退 0（不是 2）；"已声明实测却无产物"由 L18.2/AC-27 判记录不完整，与本脚本不混用
    - `run.json` / `samples.jsonl` 未知键检测：与 `RunRecord` v2 / `Sample` 的键表比对，未知键 >0 → 退出码 1（只输出键名，不输值）
    - `--self-test`：用 `sk-test-0000` 等明显假值验证"能报 `文件:行号` 且不回显内容"；退出码 0
    - `bash scripts/scan-secrets-docs.sh --all-md` 在当前仓库退出码 0
    - `bash scripts/scan-secrets-docs.sh --self-test` 退出码 0
    - 脚本自身不含任何真实凭据/编号/片段；`bash -n scripts/scan-secrets-docs.sh` 退出码 0

- [ ] **L17.2 实现 `scripts/check-model-candidates.sh`（AC-29）**
  - 追溯：AC-29；SPEC 11.9.2、11.9.8-C2；DESIGN 2.10.6"候选标注检查"、2.13.2-AC-29；新增文件 #30
  - 前置：L17.1（脚本骨架与输出纪律复用）
  - 并行：⟂ Q2：与 L17.3 并行
  - 触达：`scripts/check-model-candidates.sh`（新增，bash）
  - DoD：
    - 规格侧：在 `spec.md` 中定位 `11.9.2` 小节（从标题到下一个 `###`），该小节候选表中**每个**候选行必须含"待确认"字样；缺任一 → 退出码 1
    - 代码侧：在 `crates/`、`app/src/`、`scripts/` 检索候选字面量，命中数必须为 `0`；命中 → 退出码 1
    - 输出只给"计数"或 `文件:行号`，**不回显候选所在整行**（避免把候选写成事实）
    - 退出码：`0` 通过；`1` 失败；`2` 扫描面缺失（spec 文件不存在或 11.9.2 小节缺失）
    - 脚本自身不含候选字面量（以 `--self-test` 的假候选验证检测能力）；`--self-test` 退出码 0
    - `bash scripts/check-model-candidates.sh` 退出码 0
    - 不校验候选本身的可用性（属 C2，由用户确认；脚本只校验"标注与不写死"）

- [ ] **L17.3 修正 `scripts/run-llm-live.sh` 的 argv 泄漏（AC-32 硬要求）**
  - 追溯：AC-32（进程命令行面）；SPEC 8.4-AC-32、11.9.5-2；DESIGN 2.12.5；修改文件 #19
  - 前置：L17.1
  - 并行：⟂ Q2：与 L17.2 并行（不同文件段落，但同文件编辑须在 L19.3 之前完成）
  - 触达：`scripts/run-llm-live.sh`
  - DoD：
    - 第 43 行旧写法 `grep -RIl -- "${LLM_API_KEY}" ...` 必须消失（凭据进入 argv）：`git grep -n "grep -RIl -- \"" scripts/run-llm-live.sh` 退出码 1
    - 改为模式经 stdin：`printf '%s\n' "${LLM_API_KEY}" | grep -RIlF -f - "${target}"`；argv 只含 `-f -`
    - 收尾抓取自身命令行（`ps -o args= -p $$`，Windows 用 `wmic process where processid=... get commandline`），与 stdin 提供的值比对，**只输出命中计数**
    - 所有命中输出统一为"计数 + `文件:行号`"，禁止回显整行
    - 退出码：泄漏命中 → 非零；文档扫描面缺失（`scan-secrets-docs.sh` 返回 2）→ 原样传播为 2，不得吞成 0
    - 未设 `LLM_LIVE`：`bash scripts/run-llm-live.sh` 退出码 0，输出含"未实测（未开启真实联调）"，且脚本在该分支**之前**不发生构建与网络调用
    - `bash -n scripts/run-llm-live.sh` 退出码 0；`shellcheck` 可用时 0 error（不可用则跳过并说明）

## L18 记录契约 v2（D5 · AC-27/31/32）

**任务组目标**：在既有 `llm.live.v1` 上**只增不删**地补齐预算、确认、数据外发口径与接受日期，使 13.4 能复核"花了多少、模型是否确认、条款何时接受"。

- [ ] **L18.1 升级 `RunRecord` 到 `llm.live.v2`（★）**
  - 追溯：AC-27、AC-31、AC-32；SPEC 11.7.2、11.9.6-2（第 12 项）；DESIGN 2.12.4、2.13.4；修改文件 #21
  - 前置：L10.2（既有 13 字段）、L15.1（`call_limit`/`used` 快照）
  - 并行：⟂ Q1：与 L14.1、L15.1、L16.1、L17.1 并行
  - 触达：`crates/infrastructure/src/llm/live/record.rs`
  - DoD：
    - `schema_version` 改为 `"llm.live.v2"`；既有 13 个字段的名称/顺序/语义/三态序列化**零改动**（`git diff` 只新增行）
    - 新增字段（显式结构体，无 `serde_json::Value` 自由字段）：`path_confirmed: bool`、`call_limit: u64`、`calls_used: u64`、`budget_hit: bool`、`model_confirmed: bool`、`host_locked: bool`、`data_terms_declaration: String`、`data_terms_accepted_at: String`、`credential_rotated: bool`、`tls_trust_root: Option<String>`
    - `data_terms_declaration` 取值固定 `自用；已接受服务商条款`；`data_terms_accepted_at` 为 RFC3339 含时区偏移；二者任一缺失/空白 → 构造 `Err(CopilotError::StoreFailure)`（即"记录不完整"）
    - `sub_status == NotMeasured` 时，v2 数值字段必须为 `0`/`None`（构造期拒绝或序列化前断言，二者择一并在代码注释注明）
    - `tls_trust_root` 只允许 `ok` / `tls_trust_root` / `other` 三值，不含证书内容
    - 记录仍只写 `.tmp/llm-live/<run-id>/`；写入 `docs/` 被拒（既有断言保留）
    - 内联用例 ≥2（新增）：`v2_key_set_is_closed`（键集快照，多一键或少一键即失败）、`missing_terms_accepted_at_is_rejected`
    - `cargo test -p meeting-copilot-infrastructure live::record` 退出码 0（既有 ≥6 用例不减）
  - 备注〔D2〕：DESIGN 2.12.4 只列 `data_terms_declaration` 一个字段；SPEC 11.9.6-2 要求"含接受日期（含时区）"。故拆为"口径 + 接受日期"两字段。落地时同步 L18.2 键集与 L18.3 脚本键表，并在 L20.2 记录该受控偏差（不得回改 design.md）。

- [ ] **L18.2 追加 `tests/llm_live_offline.rs` 的 v2 断言**
  - 追溯：AC-27、AC-32；DESIGN 2.12.4、2.13.3"既有测试的改动边界"；修改文件 #20
  - 前置：L18.1、L11.1（既有离线用例）
  - 并行：⟂ Q3：与 L14.3、L16.4、L18.3、L19.1 并行
  - 触达：`crates/infrastructure/tests/llm_live_offline.rs`（**只追加**）
  - DoD：
    - 追加断言：`run.json` 键集 == v2 字段清单；`data_terms_declaration` 与 `data_terms_accepted_at` 均存在且非空；`sub_status=not_measured` 时 `call_limit/calls_used/budget_hit` 不产生误导性数值（`calls_used == 0`、`budget_hit == false`）
    - 既有 6 个离线用例的名称与断言**零改动**（`git diff` 只新增行）
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_offline` 退出码 0，用例数 ≥7
    - 文件内仍零 socket（`rg -c "TcpListener|TcpStream"` 退出码 1）

- [ ] **L18.3 同步 `scripts/llm-live-recompute.mjs` 到 v2**
  - 追溯：AC-27；SPEC 11.5.9-3；DESIGN 2.12.4"新增字段必须同步脚本"；清单外补充 S1
  - 前置：L18.1、L11.4（既有重算脚本）
  - 并行：⟂ Q3：与 L18.2 并行
  - 触达：`scripts/llm-live-recompute.mjs`
  - DoD：
    - `run.json` 读取时只校验 v2 新字段存在与类型，**不**参与 p50/p95 重算（预算字段不参与统计）
    - `samples.jsonl` 每行键集与 `Sample` 一致；未知键 >0 → 退出码 1，只输出键名（不输出值）
    - 仍只读文件、不写 `.tmp` 以外位置、不打印任何样本正文
    - `node scripts/llm-live-recompute.mjs --self-test` 退出码 0
    - 用 v2 合成样本，Node 重算 p50/p95 与 Rust `LiveStats` 差 ≤0.1 ms

## L19 C1~C3 fail-closed 与单一执行入口（D6）

**任务组目标**：C1/C2/C3 未确认时**零请求**并如实标注"未实测"；确认后只有一条入口（`run-llm-live.sh` → `llm_live` → `admit`），不存在第二处业务判定。

- [ ] **L19.1 `config.rs::try_new` 落 C1 路径校验与 C3 上限（★）**
  - 追溯：AC-28、AC-31、C1、C3；SPEC 11.9.1-2/4/5、11.9.4-4、11.9.8-C1/C3；DESIGN 2.12.1、2.12.3；修改文件 #13
  - 前置：L9.2（既有 `try_new`）、L15.1（`BudgetLedger`）、L14.2（`AdmissionTicket` 字段）
  - 并行：⟂ Q3：与 L14.3、L16.4、L18.2、L18.3 并行
  - 触达：`crates/infrastructure/src/llm/live/config.rs`
  - DoD：
    - `LLM_BASE_URL` 必须显式含 path，且 path 不能为空或仅为 `/`；否则 `Err(ProviderConfigInvalid{ detail: "LLM_BASE_URL: endpoint_path_unconfirmed" })`
    - **不**内置默认路径、**不**拼接 `chat/completions` 段、**不**在缺 path 时补全（`rg -n "chat/completions|v1/chat" crates/infrastructure/src/llm` 退出码 1）
    - `endpoint` 只用于构造 `HttpRequest` 的 path；记录只写 `endpoint_host` 与 `path_confirmed` 布尔，**不写路径原文**
    - `call_limit = min(samples_per_task × 3 × 4, LLM_LIVE_CALL_CAP)`；`LLM_LIVE_CALL_CAP` 缺失/非数字/为 0 → `Err(ProviderConfigInvalid{ detail: "LLM_LIVE_CALL_CAP: budget_not_confirmed" })`（与 `admit` 第 10 步双保险，语义一致）
    - `concurrency > 1` → 拒绝；`samples` 1..=200；`deadline_ms` 100..=8000（既有校验保留）
    - 凭据仍只在 `try_new` 内构造一次 `SecretString`；不进入 `LlmConfig`/日志/审计/记录
    - 内联用例 ≥4（新增）：缺 path、path 仅为 `/`、cap 缺失、cap < 理论值 → `call_limit == cap`
    - `cargo test -p meeting-copilot-infrastructure live::config` 退出码 0（既有 ≥7 用例不减）

- [ ] **L19.2 `tests/llm_live.rs` 入口改用 `admit`（★）**
  - 追溯：AC-19（不回归）、AC-28~AC-31（入口唯一）；DESIGN 2.12.2、2.13.3；修改文件 #20
  - 前置：L14.2、L15.1、L16.1、L19.1
  - 并行：否（关键路径）
  - 触达：`crates/infrastructure/tests/llm_live.rs`
  - DoD：
    - 每个 `#[ignore]` 用例在采样前 `match live::admit(&env, cfg!(feature = "llm-https"))`：`Skip` → 打印"未实测（label） code=…"并正常返回；`Reject` → 打印"拒绝 code=…"并正常返回；两者均**不构造传输**
    - 8 个 `#[ignore]` 用例名不变；`cargo test -p meeting-copilot-infrastructure --test llm_live -- --ignored --list` 输出 ≥7 个用例、退出码 0
    - 不带 `--ignored` 时执行 0 个（默认 `cargo test --workspace` 不受影响）
    - `rg -c "evaluate\\(" crates/infrastructure/tests/llm_live.rs` 退出码 1（入口不得仍是 `evaluate`）
    - 未启用 `llm-https` 时文件编译通过（TLS 调用点 `cfg` 门控，运行时 `Skip(TransportUnavailable)`）
    - 不写任何数值到输出或记录（Skip/Reject 路径）

- [ ] **L19.3 `run-llm-live.sh` 门顺序与 C1~C3 存在性检查（★）**
  - 追溯：AC-19、AC-28~AC-31、C1~C3；DESIGN 2.12.2、2.12.3、2.12.5；修改文件 #19
  - 前置：L17.3、L14.2、L19.1
  - 并行：⟂ 与 L18.*、L16.4 并行（同文件已在 L17.3 完成一次编辑）
  - 触达：`scripts/run-llm-live.sh`
  - DoD：
    - 存在性检查（只提示、不判定业务）：`LLM_LIVE`、`LLM_BASE_URL`（含 path）、`LLM_API_KEY`、`LLM_MODEL`、`LLM_MODEL_CONFIRMED`、`LLM_LIVE_CALL_CAP`；顺序与 `admit` 的 11 步一致
    - 提示文案与 `SkipReason::code()` / `GateReject::code()` 由同一张表驱动（脚本内维护；L21.2 用测试表比对，防止两处文案漂移）
    - 只读取 2.12.3 总表内的 `LLM_*` 变量；不接收位置参数；不写临时凭据文件
    - 收尾调用 `bash scripts/scan-secrets-docs.sh --all-md --artifacts` 与 `bash scripts/scan-secrets.sh`；前者返回 2 时脚本退出码为 2（不得吞掉）
    - 未设开关分支在**任何**构建/网络调用之前；`bash scripts/run-llm-live.sh` 退出码 0、输出含"未实测（未开启真实联调）"
    - `bash -n scripts/run-llm-live.sh` 退出码 0

- [ ] **L19.4 C1~C3 未确认的 fail-closed 用例（★）**
  - 追溯：C1、C2、C3；AC-28、AC-29、AC-31；SPEC 11.9.8；DESIGN 2.12.1、2.13.2
  - 前置：L19.1、L14.3、L15.3、L18.1
  - 并行：否（同写 `tests/llm_live_gate.rs`）
  - 触达：`crates/infrastructure/tests/llm_live_gate.rs`（追加）
  - DoD（用例 `c1c2c3_fail_closed_matrix`，四个分支各一条断言）：
    - 缺 path / path 为 `/` → `Err`，detail 含 `endpoint_path_unconfirmed`；`requests == 0`；写入 `sub_status=not_measured` + `endpoint_path_unconfirmed`，且无数值
    - `LLM_MODEL` 空 / 仅空白 / 缺 `LLM_MODEL_CONFIRMED` / 与确认值不相等 → `Reject(ModelNotConfirmed)`；`requests == 0`
    - 缺 `LLM_LIVE_CALL_CAP` → `Reject(BudgetNotConfirmed)`；`requests == 0`
    - 生成的 `run.json` 中 `model` 只取运行值；三个 reason_code 分别出现；记录与输出均不出现任何候选模型名
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_gate` 退出码 0

## L20 许可登记与治理状态（D7）

- [ ] **L20.1 更新 `license-review.md` 的 `native-tls` 三项登记**
  - 追溯：AC-30、AC-17；SPEC 11.9.3-3/4；DESIGN 2.11.1、2.11.2；修改文件 #18
  - 前置：L16.1（拿到实际包清单）
  - 并行：⟂ Q4：与 L20.3 并行
  - 触达：`docs/compliance/license-review.md`（第 2 节新增/更新条目，既有十项与签字状态行不改）
  - DoD：
    - 三项齐备：①许可结论（MIT / Apache-2.0）；②系统 TLS 依赖边界（Windows → SChannel、macOS → Security.framework、不额外打包 OpenSSL）；③"个人自用、未做商业分发合规评审"说明
    - 记录新增下载包清单（与 `git diff Cargo.lock` 一致）与"备选 `rustls + ring` 仅在系统 TLS 不可用时评估"
    - 既有十项条目与签字状态行零改动（`git diff` 只涉及该行附近的新增）
    - 文件内无域名、模型名、凭据；`bash scripts/scan-secrets-docs.sh --all-md` 退出码 0
    - `cargo deny check licenses bans sources advisories` 退出码 0

- [ ] **L20.2 `docs/progress/b1-status.md` 阶段 D 状态与输入阻塞登记**
  - 追溯：SPEC 11.9.8、11.7.1；DESIGN 2.5.4 P11~P13、2.12.1、2.13.6
  - 前置：L18.1、L19.4
  - 并行：⟂ 与 L21.1 并行（不同文件）
  - 触达：`docs/progress/b1-status.md`（追加段落，不改既有任务行）
  - DoD：
    - 追加"阶段 D（v1.2）"段：AC-28~AC-31"离线已判定"、AC-32"静态五面已判定 / 运行期四面待实测"
    - P11（真实握手与信任根）、P12（真实调用数与费用核算）、P13（C1/C2/C3 确认值）逐项标注
    - C1~C5、M6、M7 逐项登记：未确认 → 对应 fail-closed 标识 + "不阻塞离线任务"
    - 登记两处受控偏差：〔D2〕记录拆"口径 + 接受日期"；〔D5〕`budget` 局部错误类型映射
    - 不写 `docs/sdd/*.md`；`git status --short docs/sdd` 输出为空
    - 文件内不含域名、模型名、凭据

- [ ] **L20.3 `deny.toml` 只读复核**
  - 追溯：AC-17；SPEC 11.9.3-4；DESIGN 2.11.1
  - 前置：L16.1
  - 并行：⟂ Q4：与 L20.1 并行
  - 触达：`deny.toml`（预期**零改动**）
  - DoD：
    - `git diff deny.toml` 为空；若确需新增 allow 条目，只能追加 MIT/Apache-2.0 类，**不得**放宽 GPL/AGPL/SSPL
    - `cargo deny check licenses bans sources advisories` 退出码 0
    - `cargo deny check bans` 的 multiple-versions 只允许 warn（沿用既有口径）

## L21 兼容修改与门禁收口（D8）

- [ ] **L21.1 既有测试与指标维度的兼容修改**
  - 追溯：AC-17；DESIGN 2.10.8-4、2.13.3、2.13.5
  - 前置：L15.2、L18.2
  - 并行：⟂ 与 L20.2 并行
  - 触达：`crates/infrastructure/src/llm/{degrade.rs,gateway.rs}`、`metrics` 适配处（只补 match 分支）
  - DoD：
    - `degrade.rs` 既有 8 条映射断言零改动；新增 `BudgetExhausted → None` 断言 1 条
    - 所有对 `DegradeCause` 的穷尽 `match` 补 `BudgetExhausted` 分支（编译通过即为证明）；`Metric::ALL.len() == 10` 与既有 10 项断言不变
    - `cargo test -p meeting-copilot-infrastructure --test llm_schema`、`--test llm_gateway_mock`、`--test llm_degrade_inject`、`--test llm_logging_redaction` 均退出码 0 且用例数不减
    - `cargo test -p meeting-copilot-infrastructure` 退出码 0
    - 不出现"是否联调"分支：`rg -n "cfg!\\(feature = \"llm-https\"\\)" crates/infrastructure/src/llm/{gateway,degrade,fallback}.rs` 退出码 1

- [ ] **L21.2 全量门禁执行（★）**
  - 追溯：AC-17、AC-28~AC-32；本清单 0.7.4
  - 前置：L14~L20 全部任务
  - 并行：否（收口串行）
  - 触达：无源码改动（只执行与记录）
  - DoD（逐条把命令与输出摘要粘贴到 `docs/progress/b1-status.md` 证据段）：
    - G-FMT 0、G-CLIPPY 0、G-CLIPPY-F 0、G-LAYER 0、G-SECRET 0、G-SECRET-DOC 0（**退出码 2 记为失败**）、G-CAND 0、G-DENY 0
    - G-TEST：`failed=0`、`passed ≥ 208`（195 基线 + ≥13）；`cargo test --workspace -- --list | grep -c ": test"` ≥208
    - G-TEST-TLS：`cargo test -p meeting-copilot-infrastructure --features llm-https` `failed=0`；`llm_transport_tls` ≥6
    - G-OPENSSL：`cargo tree -p meeting-copilot-infrastructure --features llm-https -e normal` 的 `openssl` 与 `openssl-sys` 计数均为 0；Linux target 探测输出同样为 0
    - G-PRIVKEY：`git grep -nE "BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY"` 0 命中
    - 连续 3 次 G-TEST 结果一致（`failed=0` 且 `passed` 计数相同）

- [ ] **L21.3 禁止项审计与变更外移交登记**
  - 追溯：SPEC 9.3-3、11.6-3、11.9.1-5、11.9.2-2、11.9.5-2；DESIGN 2.8.5、2.13.5
  - 前置：L21.2
  - 并行：否
  - 触达：无（只审计与登记）
  - DoD：
    - `git status --short` 中无 `.tmp/llm-live` 入库、无新增凭据文件
    - `git diff --stat docs/sdd` 输出为空（零改动）
    - CI 零依赖：`rg -n "LLM_LIVE|LLM_API_KEY" .github` 0 命中（退出码 1 视为通过）
    - 默认值审计：`rg -n "LLM_MODEL\\s*=|default_model\\s*=|deepseek-(chat|reasoner)" crates app/src scripts` 0 命中
    - 交付说明登记"变更外移交项"：`docs/sdd/tasks.md` 的勾选由发布责任人手工执行，本清单不执行
    - 登记阶段 D 的"不得据此宣称"清单：13.1/13.2 真实端点已验证、13.4 完成、AC-18 已实测

## L22 阶段 C 实测（D9 · ⛔ 输入阻塞）

> **本组全部 ⛔**：需 L21.2 门禁全绿 + C1~C5 确认 + M6/M7 决定。未满足时**不得**开始，也不得用 mock 数据顶替。本组不重写 L13.3/L13.4 的 DoD，只把它们的执行入口更新为 L19/L21.2 之后的 `admit` 单入口。

- [ ] **L22.1 ⛔ 冒烟与预算内采样（单一入口演练）**
  - 追溯：SPEC 11.9.8、11.5.2~11.5.7、11.9.4；DESIGN 2.12.2、2.9.4、2.6.3-（2）；AC-20~AC-25、AC-31
  - 前置：L21.2 + C1/C2/C3 确认 + C4/C5 登记 + M6/M7 决定
  - 并行：否（串行采样，`--test-threads=1`、`concurrency=1`）
  - 触达：无源码改动；产物 `.tmp/llm-live/<run-id>/`
  - DoD：
    - 入口只能是 `bash scripts/run-llm-live.sh`；先 `LLM_LIVE_SAMPLES=1` 冒烟
    - 冒烟成功 → `calls_used == 1`、`budget_hit == false`；鉴权失败（401/403）→ 中止、`sub_status=not_measured`、不继续消耗配额
    - 四类任务按预算采样；触顶 → `budget_hit == true`、`calls_used == call_limit`、`samples.jsonl` 行数不减少
    - 429 未出现 → `not_triggered` + 采样窗口 + 请求数（不得写"通过"）
    - AC-24/AC-25 记录必须带"人为压低截止时间"/"人为构造的违规请求"标注
    - 四泄漏面抓取全通过；进程命令行 0 命中
  - 备注：C1~C3 任一未确认时，`admit`/`try_new` 已保证 0 请求；本任务**不得**以"先跑后补"方式启动。

- [ ] **L22.2 ⛔ 记录 v2 复核与三态移交 13.4**
  - 追溯：AC-27、AC-32；SPEC 11.5.9、11.7、11.9.6；DESIGN 2.13.6、2.9.4-6/7
  - 前置：L22.1
  - 并行：否
  - 触达：无（只读复核；结论写入交付说明与 `docs/progress/b1-status.md` 的联调段）
  - DoD：
    - `run.json` v2 全字段齐备（含 `data_terms_declaration`、`data_terms_accepted_at`、`call_limit`、`calls_used`、`budget_hit`、`credential_rotated`）
    - `samples.jsonl` 行数 == 有效样本数；`samples_path` 为相对路径且位于 `.tmp/llm-live`
    - `node scripts/llm-live-recompute.mjs` 重算 p50/p95 与记录差 ≤0.1 ms
    - `bash scripts/scan-secrets-docs.sh --all-md --artifacts` 退出码 0（退出码 2 判"不可判定"）
    - 三态结论移交 13.4；AC-18 状态变更**不由本清单执行**
    - 不写 `docs/baseline/*`、不改 `docs/sdd/*.md`

---

# 附录 A：AC-01~AC-27 覆盖矩阵

| AC | 判定命令 / 用例 | 主要任务 | 阶段 | 备注 |
|---|---|---|---|---|
| AC-01 | `cargo test -p meeting-copilot-infrastructure --test llm_schema` | L1.1、L2.1、L2.2 | A | Schema 与类型同源 + 快照 |
| AC-02 | `--test llm_gateway_mock` 的 `complete_ok` | L6.2、L7.1 | A | 强类型 + Schema 通过 |
| AC-03 | 同文件 `stream_ok` | L6.3、L7.1 | A | ≥2 增量、唯一完成、与一次性等价 |
| AC-04 | `--test llm_degrade_inject` 的 `timeout_deadline` | L3.2、L4.1、L4.2、L7.2 | A | 延迟 3 s / 截止 500 ms / ≤700 ms 返回 |
| AC-05 | `retry_cap` | L4.2、L7.2 | A | `attempts == 3` + 降级 |
| AC-06 | `rate_limit_backoff` | L4.2、L7.2 | A | 不立即重试、≤2 s、仅 1 次 |
| AC-07 | `invalid_schema_repair` | L2.1、L6.2、L7.2 | A | 修复成功 / 两次均非法 |
| AC-08 | `circuit_open` | L4.3、L7.2 | A | 冷却期请求数 0 + `Cooling` |
| AC-09 | `all_failures_no_panic` | L4.1、L4.4、L5.1、L7.2 | A | 六类各一次、退出码 0 |
| AC-10 | `degrade_same_shape` | L1.4、L5.1、L7.2 | A | 四类同型 + 原因码 |
| AC-11 | `fallback_insight` | L5.1、L5.2、L7.2 | A | ≥1 条、有引用、≤0.5 |
| AC-12 | `fallback_summary` | L5.1、L7.2 | A | 沿用上一版 / 未生成 |
| AC-13 | `fallback_conclusion` | L2.1、L5.1、L7.2 | A | 结论恒空 + 说明 |
| AC-14 | `non_blocking_capture` | L3.2、L6.2、L7.1 | A | p95 ≤200 ms |
| AC-15 | `--test llm_logging_redaction` | L6.4、L7.3 | A | 白名单 + 无标记串 |
| AC-16 | `bash scripts/scan-secrets.sh` + 仓库检索 | 全阶段夹具、L8.2 | A | 只用假值 |
| AC-17 | G-FMT/G-CLIPPY/G-CLIPPY-F/G-TEST/G-LAYER/G-DENY/G-COUNT | L8.2（+每任务 DoD） | A/B | 既有 80 不减 |
| AC-18 | `docs/progress/b1-status.md` 固定"待实测" | L8.3（状态由 13.4 变更） | A | 不得填 mock 数值 |
| AC-19 | `--test llm_live_offline` 的 `gate_*` 3 例 + `llm_live::run_gate_skips_without_live` + 脚本前置段 | L9.1、L11.1、L11.2、L11.3 | B | 双重开关、零构造、退 0 |
| AC-20 | `llm_live::first_token_latency_p50_p95` | L10.1、L10.2、L11.2 | B 编 / C 跑 | 记录型；p95 是否达标不使测试失败 |
| AC-21 | `llm_live::total_latency_four_tasks` | L10.1、L11.2 | B/C | 四类各 p50/p95 + 6 s 判定 |
| AC-22 | `llm_live::schema_conformance_rate` + `stats::degraded_counts_in_denominator_not_numerator` | L10.1、L11.2 | B/C | 成功率 + 六类分布 |
| AC-23 | `llm_live::rate_limit_observation` + `retry_after::parse` | L4.2、L10.2、L11.2 | B/C | 未出现 → `not_triggered` |
| AC-24 | `llm_live::deadline_forced_timeout` | L11.2 | B/C | 人为压低截止标注 |
| AC-25 | `llm_live::schema_violation_probe_once` + `schema::repair_hint_only_path_and_rule` | L2.1、L11.2 | B/C | 提示只含 path/rule |
| AC-26 | `llm_live_offline` 的 3 个静态面 + 脚本四泄漏面抓取 | L9.2、L10.2、L11.1、L11.3 | B（静态）/ C（运行） | 与 AC-16 互补 |
| AC-27 | `llm_live::record_is_recomputable` + `scripts/llm-live-recompute.mjs` | L10.1、L10.2、L11.1、L11.4 | B（机制）/ C（真实样本） | 字段完备 + 重算一致 |

**覆盖结论**：27 条 AC 全部有唯一主责任务与可执行判定命令；其中 AC-18、AC-20~AC-27 的真实数值部分在阶段 C，未完成前一律按三态标注，不得记为通过。

# 附录 B：依赖图（简版）

```
L1.1 ─┬─> L1.2 ─> L1.3 ─> L1.4 ─> L1.5 ─┬─> L2.1 ─> L2.2
      │                                   ├─> L3.1 ─> L3.2 ─> L3.3
      │                                   │        └─> L4.1 ─> L4.2 ─> L4.3 ─> L4.4
      │                                   └─> L5.1 ─> L5.2
      └─> L8.1
L1.5 + L6.1
L2.1 + L3.2 + L4.3 + L5.1 + L6.1 ─> L6.2 ─> L6.3 ─┬─> L6.4 ─> L6.5
                                                   └─> L6.4 ─> L7.3
L3.3 + L6.5 ─> L7.1、L7.2
L7.1 + L7.2 + L7.3 ─> L8.2 ─> L8.3 ─> L8.4
L6.1 ─> L9.1 ─> L9.2 ─> L10.1 ─> L10.2 ─┬─> L11.1
                                          ├─> L11.2
                                          ├─> L11.3
                                          └─> L11.4
L8.1 ─> L12.1 ─> L13.2（签字）
L6.1 + L11.2 ─> L12.2
L12.1 + L12.2 + L3.1 ─> L13.1 ─> L13.2 ─> L13.3（M1~M5）─> L13.4
```

# 附录 C：阶段 A 立即可执行的首批任务

| 顺序 | 任务 | 动作 | 文件 |
|---|---|---|---|
| 1 | L1.1 | 加 `schemars`、挂载 `model::llm` 与 `port::llm` | `crates/domain/Cargo.toml`、`crates/domain/src/model/mod.rs`、`crates/domain/src/port/mod.rs` |
| 2 | L1.2 | 任务/请求类型 + 构造期校验 | `crates/domain/src/model/llm.rs` |
| 3 | L1.3 | 四类结果与辅助类型 | `crates/domain/src/model/llm.rs` |
| 4 | L1.4 | 结果载体/故障/降级/观测点 | `crates/domain/src/model/llm.rs` |
| 5 | L1.5 | 端口 + `StreamCollector` | `crates/domain/src/port/llm.rs`、`crates/domain/src/port/mod.rs` |
| 6（并行三线） | L2.1 / L3.1 / L5.1 | Schema 派生 / HTTP 编解码 / 四类兜底 | `crates/infrastructure/src/llm/{schema.rs,transport.rs,fallback.rs}` |

**首批执行前置确认**：开工前只需确认〔D1〕（`schemars` 落在 domain，还是 domain + infrastructure）；其余无外部依赖。
# 附录 D：AC-28~AC-32 覆盖矩阵（阶段 D）

| AC | 判定命令 / 用例 | 主要任务 | 阶段 | 追溯（SPEC / DESIGN） | 备注 |
|---|---|---|---|---|---|
| AC-28 | `cargo test -p meeting-copilot-infrastructure --test llm_live_gate endpoint_host_locked` | L14.1、L14.2、L14.3 | D（离线） | SPEC 8.4-AC-28、11.9.1-1/3；DESIGN 2.10.5、2.13.2 | 11 种拒绝形态 + 三探针 0 值；`evil.<host>` 必拒 |
| AC-29 | 同文件 `model_explicitly_confirmed`；`bash scripts/check-model-candidates.sh` | L14.1、L14.2、L14.3、L17.2 | D（离线） | SPEC 8.4-AC-29、11.9.2；DESIGN 2.10.6、2.13.2 | 五取值；候选行均带"待确认"；代码命中 0 |
| AC-30 | 同文件 `insecure_transport_rejected`；`--features llm-https --test llm_transport_tls insecure_request_is_rejected_by_tls_transport`；G-OPENSSL | L14.2、L14.3、L16.1、L16.3、L16.4 | D（离线） | SPEC 8.4-AC-30、11.9.3-1/2；DESIGN 2.10.7、2.11.5、2.13.2 | `http://` 拒绝且 0 socket；`https://` 只放行"允许发起" |
| AC-31 | `budget_cap_enforced`（同文件）；`live::budget` 内联；`BudgetedTransport` 重试用例 | L15.1、L15.2、L15.3、L19.1、L19.4 | D（离线） | SPEC 8.4-AC-31、11.9.4；DESIGN 2.10.8、2.13.2 | `limit = min(360, cap)`；触顶即停；未确认 → 0 请求 |
| AC-32 | `bash scripts/scan-secrets-docs.sh --all-md --artifacts`；`run-llm-live.sh` 收尾段；`llm_live_offline` 静态面 | L17.1、L17.3、L18.1、L18.2、L19.3、L21.2 | D（静态）/ C（运行期四面） | SPEC 8.4-AC-32、11.9.5-2、11.5.8；DESIGN 2.13.4 | 退出码 2 = 不可判定（记为失败）；只输出`文件:行号` |
| AC-27（v2 扩展） | `llm_live_offline` 的 v2 键集断言；`node scripts/llm-live-recompute.mjs` | L18.1、L18.2、L18.3、L22.2 | D（机制）/ C（真实样本） | SPEC 11.7.2、11.9.6-2；DESIGN 2.12.4 | 新增 `data_terms_declaration` + `data_terms_accepted_at` |
| AC-20~AC-25（入口更新） | `llm_live` 的 8 个 `#[ignore]` 用例经 `admit` 进入 | L19.2、L22.1 | D（编）/ C（跑） | SPEC 11.5.2~11.5.7；DESIGN 2.12.2 | 用例名与 DoD 不变，只换入口 |

**覆盖结论（阶段 D）**：AC-28~AC-31 全部可在断网下判定，均有唯一主责任务与可执行命令；AC-32 的静态五面（文档/代码/记录/报告/脚本自检）在断网下判定，运行期四面（日志/审计/错误/命令行）随 L22.1 抓取。任一项不通过只关闭真实联调路径，**不改变** AC-01~AC-27 的任何判定（SPEC 8.4 首段）。

# 附录 E：阶段 D 依赖图（简版）

```
L14.1 ──> L14.2 ──> L14.3 ───────────> L15.3
             │                         ↑
L15.1 ──> L15.2 ──────────────────────┘

L15.1 ───> L18.1 ─┬─> L18.2
                  ├─> L18.3
                  └─> L19.1 ─> L19.2

L16.1 ───> L16.2 ─> L16.3 ─> L16.4
L16.1 ─┬─> L20.1
       └─> L20.3

L17.1 ─┬─> L17.2
       └─> L17.3

L14.2 + L16.1 + L17.3 + L19.1 ─> L19.3
L14.3 + L15.2 + L18.1 + L19.1 ─> L19.4

L15.2 + L18.2 ─> L21.1
L14~L20 全部 ─> L21.2 ─> L21.3

L21.2 + C1~C5 + M6/M7 ─> L22.1 ─> L22.2
```

**关键路径（D 版主链，8 跳）**：`L14.1 → L14.2 → L19.1 → L19.2 → L19.3 → L19.4 → L21.2 → L21.3`。
**次长链**：TLS `L16.1 → L16.2 → L16.3 → L16.4`；预算 `L15.1 → L15.2 → L15.3`；扫描 `L17.1 → L17.3`。三链均不进入主链，但 L19.3 前必须完成 L16.1 与 L17.3。

**并行簇**：Q1 = L14.1/L15.1/L16.1/L17.1/L18.1；Q2 = L17.2/L17.3；Q3 = L14.3/L16.4/L18.2/L18.3/L19.1；Q4 = L20.1/L20.3。

# 附录 F：阶段 D 可立即执行的首批任务

| 顺序 | 任务 | 动作 | 文件 | 前置 |
|---|---|---|---|---|
| 1 | **L14.1** | host 精确锁定 + scheme + 模型比对（纯函数） | `crates/infrastructure/src/llm/live/policy.rs`（新增）、`live/mod.rs` | 无（仅用既有 `EnvView`） |
| 2 | **L15.1** | `BudgetLedger` / `Permit`（原子计数、不退票） | `crates/infrastructure/src/llm/live/budget.rs`（新增）、`live/mod.rs` | 无 |
| 3 | **L16.1** | `llm-https = ["dep:native-tls"]` + Windows/macOS optional | `crates/infrastructure/Cargo.toml` | L12.1（既有） |
| 4 | **L17.1** | 覆盖 `*.md` 与 `.tmp/llm-live/**` 的泄漏扫描 | `scripts/scan-secrets-docs.sh`（新增） | L10.2（既有） |

> 四项**互不共享文件**，可四人并行或单人顺序执行。第二批（依赖首批）：L14.2、L15.2、L17.2、L17.3、L18.1。
> **开工前置**：M6（确认凭条机制与"凭据读取"口径）、M7（是否允许非 443 端口）只需在 L14.1/L14.2 落地前给出"采用默认口径/默认拒绝"的确认；未确认时按默认值执行，并在 L20.2 登记。

# 附录 G：阶段 A/B 状态保持与既有条目变更登记

> **总原则**：阶段 D 只**追加**，不回改。下表登记"既有条目在 v1.2 下被谁接续"，用于审计可追溯，不改变任何既有勾选状态与 DoD 文字。

| 既有条目 | 既有内容（保持不变） | v1.2 接续关系 | 处理方式 |
|---|---|---|---|
| L1~L11、L12.1、L8.1~L8.4 | 全部勾选状态与 DoD 文字 | 无变化 | **不重开、不修改** |
| **L12.2** | "空 feature 的构建与告警纪律"：断言 `--features llm-https` 时 `cargo tree` 不出现 `native-tls`、`cargo metadata` 的 feature 依赖数组为空 | 该断言仅对**阶段 B** 成立；阶段 D 的 L16.1 把 feature 改为真实依赖 | 保留 L12.2 勾选与文字；在 L21.2 以 G-OPENSSL/G-CLIPPY-F 作为**新**断言，**不**以"L12.2 失败"表述 |
| **L13.1** | `transport_https.rs`（#23）实现 TLS 传输 | v1.2 把文件与职责改为 `transport_tls.rs`（#26），并新增对称 `secure=false` 检查 | 保留 L13.1 未勾选；由 L16.3 接续实现，交付说明中注明"文件改名 #23→#26" |
| **L13.2** | 切换 feature 为真实依赖并复跑许可门 | 由 L16.1（feature）+ L20.1（登记）接续；因个人项目口径免双签，改为"登记即可" | 保留 L13.2 未勾选；不重写其 DoD |
| **L13.3** | 真实采样并产出三态记录（M1~M5 齐备后） | 执行入口更新为 `admit` 单入口 + `BudgetedTransport`；DoD 不变 | 保留未勾选；前置由 **L21.2 + L22.1** 更新（见 L22 首段） |
| **L13.4** | 记录复核与移交 13.4 | 记录升级为 v2；复核增加 `scan-secrets-docs.sh` 与 v2 键集 | 保留未勾选；前置由 **L22.2** 更新，DoD 不变 |
| **L8.3** | AC-18 固定写"待实测" | 无变化；阶段 D 不改变 AC-18 表述 | **不修改** |
| **0.6-M2 / 0.6-M4** | 模型名 / 采样预算 | 与 C2 / C3 同源，属"同一输入的两个面向" | 不重复计数；在 L20.2 合并登记 |

**受控偏差登记（落地时执行，禁止回改 spec.md / design.md）**

| 编号 | 偏差 | 原因 | 登记位置 |
|---|---|---|---|
| 〔D2〕 | `RunRecord v2` 把 design 的单一 `data_terms_declaration` 拆为"口径 + 接受日期"两字段 | SPEC 11.9.6-2 明确要求含接受日期（含时区） | L18.1 备注、L20.2 |
| 〔D4〕 | `live/policy.rs` 以常量 `DEEPSEEK_HOSTS = &["api.deepseek.com"]` 表达封闭集合 | SPEC 11.9.1-3 是唯一允许出现具体 host 的位置；不得拼接完整 URL | L14.1 备注、L20.2 |
| 〔D5〕 | `budget` 定义局部 `BudgetExhausted`，由 `gate` 映射为 `GateReject::BudgetExhausted` | DESIGN 2.10.3 要求 `budget` 不依赖 `gate`，避免循环依赖 | L15.1 备注、L20.2 |
| 〔D6〕 | Linux 上 `--features llm-https` 预期构建失败 | 不得在 Linux 拉入 OpenSSL（SPEC 11.9.3-3）；禁止"修好"Linux | L16.1 备注、L21.2 |
| 〔D7〕 | M6 口径采用 DESIGN 2.10.4-3："凭据读取"= `SecretString` 构造/expose，`EnvView::get` 判空不计入 | 若把判空计入，会与 AC-19"凭据缺失即跳过"冲突 | L14.2、L20.2 |

---

# 附录 H：阶段 D 收口检查单（复制即用）

```
[ ] G-FMT         cargo fmt --all --check                                         → 0
[ ] G-CLIPPY      cargo clippy --workspace --all-targets -- -D warnings           → 0
[ ] G-CLIPPY-F    cargo clippy -p meeting-copilot-infrastructure --all-targets \
                    --features llm-https -- -D warnings                            → 0
[ ] G-TEST        cargo test --workspace                                          → failed=0, passed ≥ 208
[ ] G-TEST-TLS    cargo test -p meeting-copilot-infrastructure --features llm-https → failed=0
[ ] G-LAYER       bash scripts/check-layering.sh                                  → 0
[ ] G-SECRET      bash scripts/scan-secrets.sh                                    → 0
[ ] G-SECRET-DOC  bash scripts/scan-secrets-docs.sh --all-md --artifacts          → 0（2 = 失败）
[ ] G-CAND        bash scripts/check-model-candidates.sh                          → 0
[ ] G-DENY        cargo deny check licenses bans sources advisories               → 0
[ ] G-OPENSSL     cargo tree ... --features llm-https -e normal | grep -c openssl → 0
[ ] G-PRIVKEY     git grep -nE "BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY"            → 0 命中
[ ] 状态保持      git diff --stat docs/sdd                                        → 空
[ ] 零入库        git status --short | grep '.tmp/llm-live'                       → 空
[ ] 候选未写死    rg -n "deepseek-(chat|reasoner)" crates app/src scripts         → 0 命中
```
---

# **阶段 E：确认收敛与真实采样落地（v1.3 增量，L23~L31）**

> **追加声明（强制）**：本阶段**只追加**于文末，不改动 L1~L22 的**任何标题、DoD 文字、勾选状态与编号**；0.1~0.7 与附录 A~H 一字不改。阶段 E 与既有条目的接续关系只在 0.8.7 与附录 I 登记。
> **源文档**：SPEC 8.5（AC-33~AC-37）、SPEC 11.10（11.10.1~11.10.8）；DESIGN v1.3 的 2.14~2.22（AC-33~AC-37、R31~R35、D-1~D-4、D-DEADLINE）。
> **判定性质**：AC-33~AC-37 属 **补充门槛项续篇**，**不计入** 8.1 本轮完成判据，且不改动 AC-01~AC-32 的任何编号、判定方式与通过条件。任一项不通过时只关闭真实联调路径或使相关 AC 不得宣称满足。
> **零转录（硬约束）**：本阶段正文**不出现**完整端点 URL、host 与 path 拼接的 URL 字面量、任何模型名字面量、凭据任何可见片段与 Tracking ID 任何片段。路径只以路径段常量 `"/chat/completions"` 表达；host 只在代码常量 `DEEPSEEK_HOSTS` 中。

## 0.8 阶段 E 总览

### 0.8.1 组与主任务映射

| 组 | 主任务 | 主题 | 覆盖 AC | SPEC 小节 | DESIGN 小节 |
|---|---|---|---|---|---|
| **E1** | L23 | 真实请求体固定关闭思考模式 | AC-33 | 8.5-AC-33、11.10.2 | 2.14.1~2.14.5、2.20.1 |
| **E2** | L24 | `PATH_CONSTANT` 与 `path_confirmed` 单一计算点 | AC-34 | 8.5-AC-34、11.10.4、11.9.1-5 | 2.15.1~2.15.5、2.12.1 |
| **E3** | L25 | 增量 SSE 读取（首增量即可交付） | AC-36 | 8.5-AC-36、11.10.6、5.2.1-4 | 2.17.1~2.17.6、R34 |
| **E4** | L26 | 真实采样装配与 `live/auth.rs` | AC-35、AC-31 | 8.5-AC-35、11.10.3、11.9.4 | 2.16.1~2.16.7、D-2 |
| **E5** | L27 | `LLM_ALLOWLIST` 语义收敛与候选脚本重定义 | AC-37、AC-29 | 8.5-AC-37、11.10.5、11.9.2-3 | 2.18.1~2.18.5、R32、R35、D-3、D-4 |
| **E6** | L28 | C5（未轮换）留痕与记录声明 | AC-32 前置风险 | 11.10.7、11.9.5-3、11.9.8-C5 | 2.19.1 |
| **E7** | L29 | 测试补齐与网关字段集回归 | AC-33~AC-37、AC-17 | 8.5 判定环境约束、11.10.2-4 | 2.20.1、2.14.4 |
| **E8** | L30 | 门禁阈值与收口审计 | AC-17、AC-33~AC-37 | 8.5、11.10.8 | 2.20.3、2.13.5 |
| **E9** | L31 | 真实执行（环境受限，非输入受限） | AC-35、AC-20~AC-25、AC-31 | 11.10.3、11.10.7、11.7 | 2.16.4、2.20.3、R33 |

### 0.8.2 关键路径与并行矩阵

```
★ 关键路径（阶段 E 主链，9 跳）：
L24.1 → L24.2 → L24.3 → L26.2 → L26.3 → L30.1 → L31.1 → L31.2 → L31.3

★ 次长链（不进入主链，但 L26.2 前需完成）：
增量读取链：L25.1 → L25.2 → L25.3
装配前置链：L26.1 → L26.2
思考模式链：L23.1 → L23.2（需 L26.1/L26.2 落点）

⟂ 并行簇（不同文件）：
  R1（无源码前置，立即可做）：L23.1（gateway.rs） ‖ L24.1（policy.rs） ‖ L25.1（transport.rs） ‖ L26.1（auth.rs 新增） ‖ L27.1（config.rs）
  R2（依赖 R1）：L24.2（config.rs） ‖ L25.2（transport 系） ‖ L27.3（脚本） ‖ L23.2（record/stats）
  R3（不同测试文件）：L24.4（llm_live_gate） ‖ L25.3（llm_stream_incremental） ‖ L26.4（llm_live_offline） ‖ L29.1（llm_gateway_mock）
  R4：L27.2（需 L24.4，同文件串行） ‖ L27.4（mod/脚本）
串行汇聚点：L26.2（需 L24.3+L26.1+L15.2+L16.3）、L26.3（需 L26.2+L23.2+L25.2）、L29.2（需 L23~L27）、L30（需 L23~L29）、L31（需 L30）
```

**不可并行（同文件须串行）**
- `crates/infrastructure/src/llm/transport.rs`：L25.1 → L25.2。
- `crates/infrastructure/src/llm/live/config.rs`：L24.2 → L27.1 → L26.2（`from_ticket`）。
- `crates/infrastructure/src/llm/live/gate.rs`：L24.3 → L27.4。
- `crates/infrastructure/src/llm/live/record.rs`：L24.3（夹具）→ L23.2 → L28.1。
- `crates/infrastructure/tests/llm_live_gate.rs`：L24.4 → L27.2（L23 的用例与 L24.4 同文件，插入顺序由执行者决定，但不得并发编辑）。
- `crates/infrastructure/src/llm/live/mod.rs`：L26.1（导出）→ L26.2（装配）→ L27.4（提示）。

### 0.8.3 阶段 E 文件清单

**新增 2 个文件**

| # | 文件 | 承载职责 | AC | 任务 |
|---|---|---|---|---|
| 31 | `crates/infrastructure/src/llm/live/auth.rs` | `BearerTransport`：发送时注入 `Authorization` 请求副本；不实现 `Serialize`/`Display`（DESIGN D-2 建议命名，采纳） | 35、26 | L26.1 |
| 32 | `crates/infrastructure/tests/llm_stream_incremental.rs` | AC-36 离线增量读取 5 用例（默认构建、只连本机回环、零证书夹具） | 36、03 | L25.3 |

**修改 16 个文件（去重后）**

| # | 文件 | 改动内容 | AC | 任务 |
|---|---|---|---|---|
| E-1 | `crates/infrastructure/src/llm/gateway.rs` | 新增模块级 `request_body` / `thinking_verdict` 与 `THINKING_FIELD`/`THINKING_DISABLED_VALUE`；`build_http` 改为调用（无 `live` 分支、无运行期开关） | 33 | L23.1 |
| E-2 | `crates/infrastructure/src/llm/transport.rs` | 新增泛型 `IncrementalBodyReader<R: Read>` 与 `ChunkedDecoder` 三态；`LoopbackStreamReader` 改用它；`decode_chunked` 整包路径保留给 `parse_response` | 36、03 | L25.1、L25.2 |
| E-3 | `crates/infrastructure/src/llm/transport_tls.rs` | `open_stream` 改用 `IncrementalBodyReader<TlsStream<TcpStream>>`，删除整包 `MemoryStream` 交付路径 | 36 | L25.2 |
| E-4 | `crates/infrastructure/src/llm/live/policy.rs` | 新增 `PATH_CONSTANT` 与 `path_confirmed` 纯函数；新增 2 条内联用例 | 34、37 | L24.1、L27.2 |
| E-5 | `crates/infrastructure/src/llm/live/config.rs` | 新增 `path_confirmed` 字段与单一计算点；新增 `from_ticket`；**移除** `allowlist` 字段与解析；`deadline` 默认值保持代码现值、不写死建议值 | 34、35、37 | L24.2、L27.1、L26.2 |
| E-6 | `crates/infrastructure/src/llm/live/gate.rs` | `AdmissionTicket` 增 `path_confirmed: bool`；`Admit` 分支写 `host_locked` 与启动提示 | 34、37 | L24.3、L27.4 |
| E-7 | `crates/infrastructure/src/llm/live/record.rs` | `path_confirmed` 夹具改为函数调用；`credential_rotated` 固定 `false` 与风险文案；新增 1 条内联用例 | 34、32 | L24.3、L28.1 |
| E-8 | `crates/infrastructure/src/llm/live/stats.rs` | `excluded` 允许值追加 `thinking_not_disabled`（封闭集合，只增不删） | 33、35 | L23.2 |
| E-9 | `crates/infrastructure/src/llm/live/mod.rs` | 导出 `auth`；`assemble_live` 由占位改为真实装配（票据 → 凭据 → Bearer → 预算 → 网关 → 记录器）；`LiveSession` | 35、31 | L26.1、L26.2、L27.4 |
| E-10 | `crates/infrastructure/tests/llm_live_gate.rs` | 追加 AC-33/AC-34/AC-37 三用例（同文件串行） | 33、34、37 | L24.4、L27.2 |
| E-11 | `crates/infrastructure/tests/llm_gateway_mock.rs` | 追加 `request_body_field_set`（必需字段存在 + 允许未知字段；既有断言不删） | 33、17 | L29.1 |
| E-12 | `crates/infrastructure/tests/llm_live_offline.rs` | `path_confirmed` 夹具改函数调用；追加 `live_assembly_offline_drivable` 与非占位 4 断言 | 34、35 | L24.3、L26.4 |
| E-13 | `crates/infrastructure/tests/llm_live.rs` | 8 个 `#[ignore]` 用例接入真实采样链路；删除占位 `println!`；`thinking_verdict` 前置判定；触顶即停 | 35、20~25 | L26.3 |
| E-14 | `scripts/check-model-candidates.sh` | 重定义为"零模型字面量 + 零默认/回退模型值"四面检查；退出码 0/1/2 | 29、37 | L27.3 |
| E-15 | `scripts/run-llm-live.sh` | 追加 `LLM_ALLOWLIST` 语义提示；收尾文档补扫返回值 2 原样传播（不吞成 0） | 37、32 | L27.4、L31.2 |
| E-16 | `docs/progress/b1-status.md` | 追加阶段 E 段：AC-33/34/36/37 离线状态、AC-35 载体状态、C5 与 D-DEADLINE 登记、门禁证据、用例清单 | 17、32 | L28.2、L29.2、L30.1 |

**明确不在本阶段修改**：`docs/sdd/spec.md`、`docs/sdd/design.md`、`docs/sdd/tasks.md`（勾选亦需单独授权）；`crates/application/**`；`crates/domain/**`（阶段 E 不新增任何领域类型或错误码）；`platform-win`/`platform-mac`/`ui-bridge`/`app/src-tauri`/TS 绑定；`deny.toml`；`.github/**`；`crates/domain/src/model/enums.rs`；`ErrorCode::ALL`。

### 0.8.4 阶段 E 门禁阈值

| 代号 | 命令 | 通过条件 | 与阶段 D 的差异 |
|---|---|---|---|
| G-FMT | `cargo fmt --all --check` | 退出码 0 | 同 |
| G-CLIPPY | `cargo clippy --workspace --all-targets -- -D warnings` | 退出码 0、零 warning | 同 |
| G-CLIPPY-F | `cargo clippy -p meeting-copilot-infrastructure --all-targets --features llm-https -- -D warnings` | 退出码 0、零 warning | 同 |
| G-TEST | `cargo test --workspace` | `failed=0`；`passed ≥ 238` | **222 基线不回退** + ≥16 新增默认用例 |
| G-COUNT | `cargo test --workspace -- --list \| grep -c ": test"` | ≥ 238 | 同上 |
| G-TEST-TLS | `cargo test -p meeting-copilot-infrastructure --features llm-https` | `failed=0`；`passed ≥ 152`（feature 基线不回退）；`llm_transport_tls` 用例数不减 | 新增"152 不回退" |
| G-LAYER | `bash scripts/check-layering.sh` | 退出码 0 | 同 |
| G-SECRET | `bash scripts/scan-secrets.sh` | 退出码 0 | 同 |
| G-SECRET-DOC | `bash scripts/scan-secrets-docs.sh --all-md --artifacts` | **仅退出码 0 通过；退出码 2 = 失败（不可判定不得记 0）** | 明确"2 判失败" |
| G-CAND | `bash scripts/check-model-candidates.sh` | **退出码 0**（阶段 D 遗留的 `2` 必须消除） | 判定面重定义，不再依赖规格小节 |
| G-DENY | `cargo deny check licenses bans sources advisories` | 退出码 0 | 同 |
| G-OPENSSL | `cargo tree -p meeting-copilot-infrastructure --features llm-https -e normal \| grep -cE "openssl\|openssl-sys"` | 计数 **== 0** | 同 |
| G-PRIVKEY | `git grep -nE "BEGIN (RSA \|EC \|OPENSSH )?PRIVATE KEY"` | **0 命中** | 新增用例不得引入证书/私钥夹具 |
| G-MODEL-LIT | `bash scripts/check-model-candidates.sh`（判定面 B/C 已覆盖代码/文档/脚本三面） | 模型名字面量 **0**；只输出`文件:行号`+规则号 | **不在清单中重复写模型家族正则**（避免规则描述行自我命中） |

**新增默认执行用例下限（用于 G-TEST 的 238 阈值）**

| 文件 / 位置 | 下限 | 依据（DESIGN 2.20.1 与 2.13.3） |
|---|---|---|
| `tests/llm_live_gate.rs` | **+3** | `thinking_is_fixed_disabled`（AC-33）、`path_confirmed_derived_from_constant`（AC-34）、`allowlist_cannot_widen_hosts`（AC-37） |
| `tests/llm_stream_incremental.rs` | **+5** | 2.17.4 具名五例：首增量 ≤200 ms、取消 ≤50 ms、截止、chunked 跨块、超长 |
| `tests/llm_live_offline.rs` | **+1** | `live_assembly_offline_drivable`（AC-35 默认关闭路径 + 装配可驱动） |
| `tests/llm_gateway_mock.rs` | **+1** | `request_body_field_set`（AC-33 字段集放宽、断言不删） |
| `live/auth.rs` 内联 | **+2** | Bearer 头注入、`Debug` 遮蔽 |
| `live/policy.rs` 内联 | **+2** | `path_confirmed` 五取值、`LLM_ALLOWLIST` 不改变 host 集合 |
| `gateway.rs` 内联 | **+1** | `request_body_thinking_field_is_fixed` |
| `live/record.rs` 内联 | **+1** | `unrotated_flag_does_not_force_not_measured`（C5 不触发 `not_measured` 约束） |
| **合计** | **≥16** | 222 + 16 = **238** |

> `llm_live.rs` 的 8 个 `#[ignore]` 与 `llm_transport_tls.rs` 的 ≥6 个 feature 用例**不计入** 238：前者默认不执行，后者只在 `--features llm-https` 下计入 G-TEST-TLS（AC-17 的"默认构建不受 feature 影响"继续成立）。

### 0.8.5 阶段 E 禁止事项（追加于 0.5、0.7.5，同等强制）

1. 不得写完整端点 URL、host 与 path 拼接的 URL 字面量、任何模型名字面量、凭据任何可见片段（前缀/中段/末四位）与 Tracking ID 任何片段；扫描命中时只输出`文件:行号`，禁止回显内容。
2. 不得把 `LLM_LIVE_DEADLINE_MS` 的建议值 **8000 写成默认常量或 `unwrap_or(8_000)`**；未获最终确认前沿用代码现值（`config.rs:124` 的 `unwrap_or(6_000)`）。
3. 不得**声称 AC-20 已在 AC-36 完成前满足**；AC-36 未通过时，AC-20 只能写"未实测（流式读取未增量）"。
4. 不得在 `check-model-candidates.sh` 的规则描述行里放真实模型名；占位白名单只允许通用占位词（如 `test-model`、`fake-model`、`fixture-model`、`mock-model`、`placeholder-model`）。
5. 不得**让 mock 与真实使用两套请求体构造**（SPEC 11.10.2-5）：`thinking` 字段只能由 `request_body` 单点产出，禁止"真实路径写入、mock 路径省略"。
6. 不得在读取器内解析 SSE（会形成第二份协议实现，破坏 2.11.4 与 AC-03 的判定意义）；读取器只交付字节。
7. 不得为通过 AC-36 而放宽 AC-03 的语义断言；只允许调整等待上限，不允许放宽"序号递增、唯一终止、重复丢弃、`[DONE]` 结束"。
8. 不得删除/放宽既有断言以让 `thinking` 字段通过（SPEC 11.10.2-4）：`llm_gateway_mock` 只能改为"必需字段存在 + 允许未知字段"，**不得删除**原断言。
9. 不得移除 `LLM_ALLOWLIST` 变量本身或其 `Skip(AllowlistMissing)` 语义（SPEC 11.10.5-4）；只能移除它到"允许集合"的数据流。
10. 不得引入任何证书/私钥夹具或证书生成依赖（`rcgen` 等）。
11. 不得回改 L1~L22 的任何条目；阶段 E 的接续/取代关系只在 0.8.7 与附录 I 登记。
12. 不得修改 `docs/sdd/*.md`；阶段 E 的进度登记只写 `docs/progress/b1-status.md` 与 `.tmp/llm-live/` 产物。

### 0.8.6 输入确认状态与执行约束

| 编号 | 事项 | 状态（本阶段生效） | 对阶段 E 的影响 |
|---|---|---|---|
| C1 | 端点路径 | **已确认**：只允许路径段常量 `"/chat/completions"`；完整 URL 只由运行期值提供 | L24 只做常量比对，不内置默认路径 |
| C2 | API 模型名 | **已确认（运行期）**：`LLM_MODEL` 与 `LLM_MODEL_CONFIRMED` 逐字节相等；值不落仓库；原候选表作废 | L27.3 改为"零字面量"检查 |
| C3 | 采样预算 | **已确认**：`LLM_LIVE_CALL_CAP`，`limit = min(samples × 12, cap)` | L26/L31 沿用，不改口径 |
| C4 | 其它官方域名 | **已确认**：集合仅代码常量 `DEEPSEEK_HOSTS`；`LLM_ALLOWLIST` 不是判定来源 | L27 落实 AC-37 |
| C5 | 凭据是否已轮换 | **已答复：未轮换** | L28 固定 `credential_rotated = false` 与风险文案 |
| M6 | `path_confirmed` 来源 | **已确认**：由常量比对计算，不得手写 `true` | L24.3 静态断言 |
| M7 | 非 443 端口 | **已确认**：默认拒绝，无放行开关 | 阶段 E 不改 `policy` 端口规则 |
| **D-DEADLINE** | `LLM_LIVE_DEADLINE_MS` 默认值 | **未最终确认**：8000 仅为建议值 | 沿用代码现值（`config.rs:124` 的 6000）；`8000` 只允许出现在范围校验 `100..=8_000` 中，不得作为运行时默认 |

> **显著性说明**：SPEC 11.10.7 写"沿用既有默认（4000）"，而代码现值（`crates/infrastructure/src/llm/live/config.rs:124`）为 `unwrap_or(6_000)`。两者不一致，本阶段以**代码现值 6000** 为执行依据并登记为受控偏差〔E4〕，不回改 spec/design，也不把 8000 写死。

### 0.8.7 与既有条目的接续关系（只登记，不重写）

| 既有条目 | 既有内容（保持不变） | 阶段 E 接续关系 | 处理方式 |
|---|---|---|---|
| L6.2/L6.3 | 一次性与流式编排、`build_http` 内联 JSON | L23.1 把请求体抽出为 `request_body`，`build_http` 只多一次调用 | 保留隐藏状态；交付说明注明"编排顺序不变" |
| L13.1/L16.3 | `transport_tls.rs` 实现 TLS 传输 | L25.2 把 `open_stream` 由整包 `MemoryStream` 改为增量读取器 | 保留未勾选/已勾选原状；由 L25.2 接续 |
| L19.1 | `config.rs::try_new` 的 path 存在性校验与 `call_limit` | L24.2 增 `path_confirmed`；L26.2 增 `from_ticket`；既有校验不删 | DoD 文字不改，由 L24.2/L26.2 接续 |
| L22.1 | 阶段 C 冒烟与预算内采样 | L31 以"环境受限"口径接续执行 | 保留未勾选；前置由 L30.1 更新（见 L31 首段） |
| L17.2 | `check-model-candidates.sh` 依赖 11.9.2 候选表 | L27.3 重定义判定面，消除退出码 2 | 保留 L17.2 文字；由 L27.3 取代其判定口径（登记为〔E3〕） |
| L18.1 | `RunRecord` v2 键集 | L24.3 只改夹具构造方式（`path_confirmed` 由函数产出），键集不变 | 不重写 L18.1 |
| 0.7.6-C5 | "凭据是否已轮换" | L28 落实"未轮换"记录与风险标注 | 不重复计数 |

---

# 阶段 E：确认收敛与真实采样落地（L23~L31）

## L23 真实请求体固定关闭思考模式（E1 · AC-33）

**任务组目标**：把 `thinking.type == "disabled"` 从"口头约定"变成**单点构造 + 纯函数判定**：mock 与真实共用同一份请求体规则，字段不可运行期配置，判定失败即样本无效。

- [ ] **L23.1 抽出 `request_body` 纯函数并固定 `thinking` 字段（★）**
  - 追溯：AC-33；SPEC 8.5-AC-33、11.10.2-1/2/5、5.1.1-9、4.1-3；DESIGN 2.14.1、2.14.2、2.14.3；R31
  - 前置：L6.2、L6.3（既有 `build_http` 与网关编排已落地）
  - 并行：⟂ R1：与 L24.1、L25.1、L26.1、L27.1 并行（不同文件）；不可与任何 `gateway.rs` 编辑并发
  - 触达：`crates/infrastructure/src/llm/gateway.rs`
  - DoD（`cargo test -p meeting-copilot-infrastructure gateway::` 退出码 0）：
    - 新增模块级常量 `pub const THINKING_FIELD: &str = "thinking";` 与 `pub const THINKING_DISABLED_VALUE: &str = "disabled";`
    - 新增 `pub fn request_body(model: &str, request: &LlmGenerationRequest, stream: bool) -> serde_json::Value`：**签名不含 `thinking` 参数**，函数体内不读环境变量、不读配置、不做 IO、不出现 `live` 符号
    - `request_body(..)["thinking"]["type"] == "disabled"`：一次性（`stream=false`）与流式（`stream=true`）各断言一次
    - 新增 `pub fn thinking_verdict(body: &serde_json::Value) -> Option<&'static str>`：正例返回 `None`；缺失 `thinking`、`thinking.type != "disabled"`、`thinking == {}` 三种反例一律返回 `Some("thinking_not_disabled")`
    - 反例必须由正例 body 派生（删除/改写），**不得另写一份请求体**（2.14.3 反例构造纪律）
    - `build_http` 改为调用 `request_body(&config.model, request, stream)`；`git diff` 中 header、`json_post`、序列化方式均不变，差异只有"删除原内联 JSON 组装、新增一次调用"
    - `rg -n '"enabled"' crates/infrastructure/src/llm/gateway.rs` **退出码 1**（无其它 `thinking.type` 取值字面量）
    - `rg -n "thinking.*(env|LLM_)" crates/infrastructure/src/llm/gateway.rs` **退出码 1**（无运行期开关）
    - 内联新增用例 ≥1：`request_body_thinking_field_is_fixed`
    - `cargo clippy -p meeting-copilot-infrastructure --all-targets -- -D warnings` 退出码 0

- [ ] **L23.2 `thinking_not_disabled` 标注常量与记录允许值**
  - 追溯：AC-33、AC-35；SPEC 8.5-AC-33、11.10.2-3/4；DESIGN 2.14.5
  - 前置：L23.1
  - 并行：⟂ R2：与 L24.2、L25.2、L27.3 并行（不同文件）
  - 触达：`crates/infrastructure/src/llm/live/stats.rs`（`excluded` 允许值）、`crates/infrastructure/src/llm/live/record.rs`（`reason_code` 允许值常量，只追加）
  - DoD：
    - `excluded` 的封闭允许集合追加 `thinking_not_disabled`；既有取值（`auth_failed`/`endpoint_incompatible`/`record_failed`/`deadline_forced` 等）**一个不删**
    - 记录侧新增常量（如 `REASON_THINKING_NOT_DISABLED: &str = "thinking_not_disabled"`），`sub_status = not_measured` 时可用该 `reason_code` 且**不带任何数值**
    - `RunRecord::validate` 既有"`NotMeasured ⇒ calls_used == 0`"约束不被破坏（内联或离线用例断言）
    - 不新增 `ErrorCode` 变体：`rg -n "ErrorCode::.*thinking" crates/` 退出码 1
    - `cargo test -p meeting-copilot-infrastructure live::stats live::record` 退出码 0（既有用例数不减）
  - 备注：真实接入（采样写入前判定）在 **L26.3** 完成；本任务只落"可表达"的部分，避免与 L26.3 争抢 `llm_live.rs`。

## L24 `PATH_CONSTANT` 与 `path_confirmed` 单一计算点（E2 · AC-34）

**任务组目标**：把 `path_confirmed` 从"手写布尔"变成"单一常量比对 + 票据传递"；路径原文不进入记录，生产代码中手写 `true` 命中为 0。

- [ ] **L24.1 `policy.rs` 增加 `PATH_CONSTANT` 与 `path_confirmed` 纯函数（★）**
  - 追溯：AC-34；SPEC 8.5-AC-34、11.10.4-1/2/4、11.9.1-5；DESIGN 2.15.1、2.15.2
  - 前置：L14.1（既有 `policy.rs` 与 `host_is_locked`）
  - 并行：⟂ R1：与 L23.1、L25.1、L26.1、L27.1 并行
  - 触达：`crates/infrastructure/src/llm/live/policy.rs`
  - DoD：
    - 新增 `pub const PATH_CONSTANT: &str = "/chat/completions";`（唯一比对基准，只含路径段、不含 host）
    - 新增 `pub fn path_confirmed(path: &str) -> bool`：`path.as_bytes() == PATH_CONSTANT.as_bytes()`；**不 trim**、不做大小写归一化、不做尾斜杠归一化
    - 五取值断言：`"/chat/completions"` → `true`；`"/"`、`"/v1/chat/completions"`、`""`、"仅 query/fragment（解析后 path 为空）" → `false`
    - 内联新增用例 ≥1：`path_confirmed_five_values`
    - 同文件不出现 host 与 path 拼接的完整 URL 字面量：`rg -n "chat/completions" crates/infrastructure/src/llm/live/policy.rs` 只命中常量定义 **1 处**
    - `cargo test -p meeting-copilot-infrastructure live::policy` 退出码 0（既有 ≥3 用例不减）

- [ ] **L24.2 `config.rs` 解析 path 并只保留布尔（★）**
  - 追溯：AC-34、C1、M6；SPEC 11.10.4-1/2/3、11.9.1-2；DESIGN 2.15.2、2.15.3、2.12.1
  - 前置：L24.1、L19.1（既有 path 存在性校验与 `call_limit`）
  - 并行：否（同写 `config.rs`，先于 L27.1）
  - 触达：`crates/infrastructure/src/llm/live/config.rs`
  - DoD：
    - `LiveRunConfig` 新增 `pub path_confirmed: bool`，取值只由 `policy::path_confirmed(path_from_url(&url))` **计算一次**
    - 既有 fail-closed 保留：path 缺失或仅为 `/` → `Err(CopilotError::ProviderConfigInvalid{ detail: "LLM_BASE_URL: endpoint_path_unconfirmed" })`，**零请求**
    - path 存在但不等于常量（如 `/v1/chat/completions`）→ `Ok` 且 `path_confirmed == false`（**不拒绝**，AC-28 只锁 host）
    - `endpoint` 原文不进入 `Debug`/记录：`format!("{:?}", config)` 不含 query 串与可见路径段（断言不含 `"?"` 与不含除常量外的路径样本）
    - 内联新增用例 ≥2：`path_confirmed_true_for_constant`、`path_present_but_not_constant_is_not_rejected`
    - `cargo test -p meeting-copilot-infrastructure live::config` 退出码 0（既有 ≥7 不减）

- [ ] **L24.3 票据与记录传递链路（禁止手写 `true`）（★）**
  - 追溯：AC-34、M6；SPEC 11.10.4-2/3；DESIGN 2.15.3、2.15.4
  - 前置：L24.2、L14.2（`AdmissionTicket`）、L18.1（`RunRecord.path_confirmed`）
  - 并行：否（同写 `gate.rs`、`record.rs` 夹具；`llm_live_offline.rs` 可随后串行）
  - 触达：`crates/infrastructure/src/llm/live/gate.rs`、`crates/infrastructure/src/llm/live/record.rs`、`crates/infrastructure/tests/llm_live_offline.rs`
  - DoD：
    - `AdmissionTicket { endpoint_host, scheme, call_limit, path_confirmed }`（只加布尔，不携带 path 原文）
    - `admit` 在 `Admit` 分支用 `policy::path_confirmed(path_from_url(base_url))` 填充，**只计算一次**
    - `LiveRunConfig` 与 `RunRecord` **不得再次调用**比对函数：`rg -n "path_confirmed\\(" crates/infrastructure/src/llm/live/config.rs crates/infrastructure/src/llm/live/record.rs` 退出码 1（仅允许 `policy.rs` 定义与 `gate.rs` 调用）
    - `record.rs:213` 与 `llm_live_offline.rs:135` 的 `path_confirmed: true` 改为 `path_confirmed(PATH_CONSTANT)` 或与函数返回值比对；`llm_live_gate.rs:276` 的 `false` 可保留（不属禁止项）
    - 静态断言：`grep -rnE 'path_confirmed[[:space:]]*[:=][[:space:]]*true' crates/infrastructure/src` **0 命中**（退出码 1）
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_gate` 与 `--test llm_live_offline` 退出码 0，用例数不减；`llm_live_offline` 只改夹具构造方式，**断言文字不改**

- [ ] **L24.4 AC-34 用例 `path_confirmed_derived_from_constant`**
  - 追溯：AC-34；SPEC 8.5-AC-34；DESIGN 2.20.1、2.15.2、2.15.4
  - 前置：L24.3
  - 并行：⟂ R3：与 L25.3、L26.4、L29.1 并行（不同文件）；与 L27.2 同文件须串行
  - 触达：`crates/infrastructure/tests/llm_live_gate.rs`（追加）
  - DoD：
    - 注入五种 path → 仅 `"/chat/completions"` 得 `true`，其余四种得 `false`
    - v2 记录的 `path_confirmed` 与 `path_confirmed(..)` 返回值**一致**（不经过任何手写常量）
    - 生产代码手写 `true` 命中 0（复用 L24.3 的 grep 断言）
    - 文件内仍零 socket：`rg -c "TcpListener|TcpStream" crates/infrastructure/tests/llm_live_gate.rs` 退出码 1
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_gate` 退出码 0，用例数 = 开工前基线 + ≥1

## L25 增量 SSE 读取（E3 · AC-36）

**任务组目标**：把 `open_stream` 从"整包缓冲后一次性交付"改成"首增量到达即可交付"；读取器只交付字节、不解析 SSE，回环与 TLS 共用同一份实现。

- [ ] **L25.1 实现泛型 `IncrementalBodyReader<R: Read>` 与 `ChunkedDecoder`（★）**
  - 追溯：AC-36；SPEC 8.5-AC-36、11.10.6-2/3/5；DESIGN 2.17.2、2.17.3；R34
  - 前置：L3.1、L3.2（既有 `Http1Codec` / `StreamReader` / `read_all_budgeted`）、L16.2（泛型 I/O 辅助）
  - 并行：⟂ R1：与 L23.1、L24.1、L26.1、L27.1 并行；与 L25.2 同文件须串行
  - 触达：`crates/infrastructure/src/llm/transport.rs`
  - DoD（`cargo test -p meeting-copilot-infrastructure transport::` 退出码 0）：
    - 新增 `pub struct IncrementalBodyReader<R: Read>`，实现 `StreamReader`（`next_chunk` / `status` / `headers`）
    - `next_chunk` 顺序：先查 `cancel`（→ `Err(Cancelled)`，≤50 ms）、再查 `Instant::now() >= deadline`（→ `Err(ReadTimeout)`）、buffer 内已有可交付单元则**立即** `Ok(Some(bytes))`
    - `set_timeout(remaining.min(50ms))`；读超时后**复查** deadline 再继续（不得把超时当 EOF）
    - `Ok(0)`：framing 未终止 → `Err(Malformed)`（TLS 侧对端关闭可为 `PeerClosed`，与既有语义一致）；framing 已终止 → `Ok(None)`
    - 累计字节 > `max_bytes` → `Err(TooLarge)`
    - 新增 `ChunkedDecoder` 三态（`NeedMore` / `Item` / `Done`），语义与既有整包 `decode_chunked` 一致；整包 `decode_chunked` **保留**给 `parse_response`
    - 读取器内**不解析 SSE**：`rg -n "data:" crates/infrastructure/src/llm/transport.rs` 不得新增 SSE 相关行（只允许既有非 2xx 标记相关行）
    - `codec_contract` 3 组断言**零改动**；内联新增用例 ≥2（增量交付、跨块 chunked 一致性）
    - `cargo clippy -p meeting-copilot-infrastructure --all-targets -- -D warnings` 退出码 0

- [ ] **L25.2 回环与 TLS 两处 `open_stream` 切换到增量读取器（★）**
  - 追溯：AC-36、AC-03 回归；SPEC 11.10.6-5；DESIGN 2.17.2、2.17.5
  - 前置：L25.1
  - 并行：否（`transport.rs` 与 `transport_tls.rs` 串行编辑）
  - 触达：`crates/infrastructure/src/llm/transport.rs`（`LoopbackStreamReader`）、`crates/infrastructure/src/llm/transport_tls.rs`（`open_stream`）
  - DoD：
    - `LoopbackStreamReader` 改为选择 `IncrementalBodyReader<TcpStream>`；chunked 分支不再"先读完整包再 `decode_chunked`"：`rg -n "decode_chunked\\(&bytes\\)" crates/infrastructure/src/llm/transport.rs` 退出码 1
    - `TlsTransport::open_stream` 返回 `Box::new(IncrementalBodyReader::new(tls_stream, headers, max_bytes))`，**不调用** `read_all_tls`
    - `MemoryStream` 不再出现在 `open_stream` 路径：`rg -n "MemoryStream" crates/infrastructure/src/llm/transport_tls.rs` 退出码 1（若保留仅供测试，须 `#[cfg(test)]` 且不出现在 `open_stream`）
    - 非 2xx 首包标记 `HTTPSTATUS:` 语义不变（复用既有选择器行为）
    - `cargo test -p meeting-copilot-infrastructure --test llm_gateway_mock` 与 `--test llm_degrade_inject` 退出码 0（`stream_ok` 断言零修改）
    - `cargo build -p meeting-copilot-infrastructure` 退出码 0（默认构建不引入 TLS）
    - `cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_transport_tls` 退出码 0（既有 ≥6 用例不减）

- [ ] **L25.3 新增 `tests/llm_stream_incremental.rs`（AC-36 离线五例）（★）**
  - 追溯：AC-36；SPEC 8.5-AC-36、11.10.6-2；DESIGN 2.17.4、2.20.1
  - 前置：L25.2
  - 并行：⟂ R3：与 L24.4、L26.4、L29.1 并行（不同文件）
  - 触达：`crates/infrastructure/tests/llm_stream_incremental.rs`（新增 #32）
  - DoD（全部只连 `127.0.0.1`、零证书夹具、默认构建）：
    - 文件首行不带 `#![cfg(feature = "llm-https")]`（必须默认执行）
    - `first_delta_incremental_loopback`：listener 写 **1 个** SSE 事件后 `sleep 500 ms` 且**保持连接不关闭** → 首个 `Delta` 在事件写入后 **≤200 ms** 内交付；服务端不关闭时**不阻塞**
    - `cancel_is_prompt`：读循环中途 `cancel` → `Err(Cancelled)`，返回时刻 ≤ `cancel + 100 ms`（判定用 100 ms 上限 + 200 ms 容差）
    - `deadline_is_respected`：对端静默 + `deadline = now + 100 ms` → `Err(ReadTimeout)`，返回时刻 ≤ `deadline + 200 ms`
    - `chunked_boundaries`：半块/跨块分片写 chunk → 与整包 `decode_chunked` 结果**逐字节一致**
    - `too_large_is_rejected`：累计超过 `max_bytes` → `Err(TooLarge)`
    - 文件内不含任何私钥块：`rg -n "PRIVATE KEY|BEGIN " crates/infrastructure/tests/llm_stream_incremental.rs` 退出码 1
    - `cargo test -p meeting-copilot-infrastructure --test llm_stream_incremental` 退出码 0，用例数 ≥5
    - `cargo test -p meeting-copilot-infrastructure --test llm_gateway_mock` 退出码 0（AC-03 不回归）

## L26 真实采样装配与 `live/auth.rs`（E4 · AC-35）

**任务组目标**：把 `llm_live` 的 8 个 `#[ignore]` 用例从"只打印占位"变成"经 `admit` → 凭据 → Bearer → 预算 → TLS → 网关"的真实采样链路；默认关闭路径保持 0 构造、0 socket、0 样本、退出码 0。

- [ ] **L26.1 新增 `live/auth.rs::BearerTransport`（★）**
  - 追溯：AC-35、AC-26；SPEC 11.10.3-1、11.4-5/6；DESIGN 2.16.3、D-2
  - 前置：L9.2（既有 `SecretString`）、L3.2（既有 `HttpTransport`）
  - 并行：⟂ R1：与 L23.1、L24.1、L25.1、L27.1 并行
  - 触达：`crates/infrastructure/src/llm/live/auth.rs`（新增 #31）、`crates/infrastructure/src/llm/live/mod.rs`（导出）
  - DoD：
    - `pub struct BearerTransport { inner: Arc<dyn HttpTransport>, secret: SecretString }` 实现 `HttpTransport::{send, open_stream}`
    - 每次**实际发送**时 `secret.expose()` 一次，构造带 `Authorization` 的**请求副本**；对端/上游仍收到原 `HttpRequest` 语义
    - `Debug` 不输出该头（复用 `HttpRequest` 既有遮蔽）；`format!("{:?}", bearer)` 不含夹具假值 `sk-test-0000` 的任何 ≥4 字符片段
    - 不实现 `Serialize`/`Display`（沿用 `SecretString` 语义）
    - `gateway.rs` 与 `transport.rs` **零 diff**（纯装配层装饰器）
    - 内联新增用例 ≥2：`bearer_header_injected_on_send`、`bearer_debug_is_masked`
    - `cargo test -p meeting-copilot-infrastructure live::auth` 退出码 0

- [ ] **L26.2 `assemble_live` 由占位改为真实装配（★）**
  - 追溯：AC-35、AC-31；SPEC 11.10.3-1/2/5；DESIGN 2.16.2、2.16.6、2.12.2
  - 前置：L26.1、L24.3（票据含 `path_confirmed`）、L15.2（`BudgetedTransport`）、L16.3（`TlsTransport`）、L27.1（`config.rs` 已定稿）
  - 并行：否（关键路径汇聚点）
  - 触达：`crates/infrastructure/src/llm/live/mod.rs`（替换 `assemble_live` 占位签名与实现）
  - DoD：
    - 新增 `pub struct LiveSession { gateway: Arc<dyn StructuredLlmGateway>, ledger: Arc<BudgetLedger>, recorder: Arc<LiveRecorder>, config: Arc<LiveRunConfig> }`
    - 新签名：`pub fn assemble_live(ticket: &AdmissionTicket, env: &dyn EnvView, transport: Arc<dyn HttpTransport>, observer: Arc<dyn LlmObserver>, clock: Arc<dyn Clock>, run_id: &str) -> Result<LiveSession, CopilotError>`
    - 装配顺序固定：`LiveRunConfig::from_ticket`（唯一凭据读点）→ `BearerTransport` → `BudgetedTransport`（**Bearer 外层**）→ `StructuredLlmGateway` → `LiveRecorder`
    - 占位分支消失：`rg -n "transport_unavailable" crates/infrastructure/src/llm/live/mod.rs` 退出码 1；`rg -n "阶段 C 未接入" crates/infrastructure/src/llm/live/mod.rs` 退出码 1
    - `LlmConfig` 由会话派生：`endpoint` 含运行期 path、`model` 取运行值、`domain_allowlist` = 票据单一精确 host、`max_concurrency = 1`、`credential_ref` 只放引用名
    - 默认构建可编译：`assemble_live` 只依赖 `HttpTransport` trait；`rg -n "transport_tls" crates/infrastructure/src/llm/live/mod.rs` 只出现在 `#[cfg(feature = "llm-https")]` 块内
    - 内联/离线用例 ≥1：`assemble_live_with_mock_transport_is_drivable`（注入 `MockTransport`，不建 socket）
    - `cargo build -p meeting-copilot-infrastructure` 与 `cargo build -p meeting-copilot-infrastructure --features llm-https` 均退出码 0
    - `cargo clippy -p meeting-copilot-infrastructure --all-targets --features llm-https -- -D warnings` 退出码 0

- [ ] **L26.3 8 个 `#[ignore]` 用例接入真实采样链路（禁止占位）（★）**
  - 追溯：AC-35、AC-19、AC-20~AC-25；SPEC 11.10.3-1/2/4/5/6/7；DESIGN 2.16.4、2.16.5、2.16.7
  - 前置：L26.2、L23.1/L23.2（thinking 判定与标注）、L25.2（增量读取）
  - 并行：否（关键路径）
  - 触达：`crates/infrastructure/tests/llm_live.rs`
  - DoD：
    - 8 个 `#[ignore]` 用例名**不变**；每个用例采样前调用 `admit`；`Skip`/`Reject` → 打印原因码后正常 `return`，**不构造传输**
    - 删除全部"阶段 C 未接入真实传输"占位输出：`rg -n "阶段 C 未接入" crates/infrastructure/tests/llm_live.rs` 退出码 1
    - 共享采样骨架：`run_id`（时间戳 + commit 短号）、`seq` 全局单调跨用例不重置、`attempt = outcome.attempts.max(1)`、`total_ms = total_ms.max(1)`、样本只追加
    - 每样本写入前以**与真实发送同源**的 `request_body` 生成 body 并调用 `thinking_verdict`；`Some(_)` → `excluded = Some("thinking_not_disabled")`、`first_token_ms = None`、不写任何时延数值
    - 鉴权失败（401/403）→ 中止采样、`sub_status = not_measured`、**不继续消耗配额**、不回显凭据任何片段
    - 触顶 → `budget_hit = true`、`calls_used == call_limit`、`samples.jsonl` 行数**不减少**
    - `run_gate_skips_without_live`：未设开关 → 0 传输构造、0 socket、0 样本、退出码 0
    - 默认路径：不带 `--ignored` 执行 **0** 个；`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --ignored --list` 退出码 0 且列出 ≥7
    - `rg -c "evaluate\\(" crates/infrastructure/tests/llm_live.rs` 退出码 1（入口只能是 `admit`）
    - `cargo test --workspace` 的 `passed` 计数**不回退**

- [ ] **L26.4 AC-35 非占位与默认关闭路径离线用例**
  - 追溯：AC-35；SPEC 8.5-AC-35；DESIGN 2.16.7、2.20.1
  - 前置：L26.2、L26.3
  - 并行：⟂ R3：与 L24.4、L25.3、L29.1 并行（不同文件）
  - 触达：`crates/infrastructure/tests/llm_live_offline.rs`（**只追加**）
  - DoD：
    - `live_assembly_offline_drivable`：注入 `MockTransport` 可构造 `LiveSession` 并完成一次调用（零 socket）；默认关闭路径断言 `transports_constructed == 0`、`sockets_opened == 0`、样本数 `== 0`、退出码 0、输出含"未实测（原因）"
    - 非占位 4 断言（合成样本或显式样本）：`samples.jsonl` 行数 ≥1、每行 `attempt ≥ 1`、`total_ms > 0`、`outcome` 不含 `not_measured`/`未实测`/`placeholder`；`run.json` 为 `llm.live.v2` 且键集封闭
    - 既有离线用例名与断言**零改动**（`git diff` 只新增行）
    - 文件内零 socket：`rg -c "TcpListener|TcpStream" crates/infrastructure/tests/llm_live_offline.rs` 退出码 1
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_offline` 退出码 0，用例数 = 基线 + ≥1

## L27 `LLM_ALLOWLIST` 语义与候选脚本重定义（E5 · AC-37）

**任务组目标**：让"环境变量不能扩大允许 host 集合"成为**结构上不存在的数据流**；把 `check-model-candidates.sh` 从"依赖已被删除的候选表"改为"零模型字面量 + 零默认/回退模型值"。

- [ ] **L27.1 移除 `LiveRunConfig.allowlist` 数据流（★）**
  - 追溯：AC-37；SPEC 8.5-AC-37、11.10.5-1/2/3/4；DESIGN 2.18.1、2.18.2、2.18.3；D-3
  - 前置：L14.2（既有 `admit` 第 8 步只判存在性）
  - 并行：⟂ R1：与 L23.1、L24.1、L25.1、L26.1 并行；`config.rs` 与 L24.2 串行（L24.2 先）
  - 触达：`crates/infrastructure/src/llm/live/config.rs`、`crates/infrastructure/src/llm/live/mod.rs`（如有引用）
  - DoD：
    - `LiveRunConfig` 不再含 `allowlist: Vec<String>` 与对应解析：`rg -n "allowlist" crates/infrastructure/src/llm/live/config.rs` 退出码 1
    - `admit` 第 8 步仍只判 `LLM_ALLOWLIST` 存在性 → `Skip(AllowlistMissing)`；其**取值不进入** `host_is_locked` 的第二参数
    - `rg -n "LLM_ALLOWLIST" crates/infrastructure/src` 只命中 `gate.rs`（存在性判断 + 注释）与既有测试夹具
    - `DEEPSEEK_HOSTS` 定义仍**仅 1 处**：`rg -c "pub const DEEPSEEK_HOSTS" crates/infrastructure/src` 输出 `1`
    - `TlsTransport::with_allowed_hosts` 明确标注"仅供离线回环"或置于 `#[cfg(test)]`，不得成为第二来源：`rg -n "with_allowed_hosts" crates/infrastructure/src` 命中 ≤1 且位于 test/文档注释
    - `cargo test -p meeting-copilot-infrastructure live::config live::gate` 退出码 0（既有 ≥7 + ≥4 不减）

- [ ] **L27.2 AC-37 用例 `allowlist_cannot_widen_hosts`（★）**
  - 追溯：AC-37；SPEC 8.5-AC-37；DESIGN 2.18.4、2.20.1
  - 前置：L27.1、L24.4（同文件串行）
  - 并行：否（同写 `tests/llm_live_gate.rs`）
  - 触达：`crates/infrastructure/tests/llm_live_gate.rs`（追加）
  - DoD（三断言全过）：
    - ①`LLM_ALLOWLIST` 设为**非官方 host**（占位域名）→ 该 host 仍 `host_is_locked == NotAllowed`；只有常量内 host 可 `Admit`
    - ②`LLM_ALLOWLIST` 缺失或为空 → `host_is_locked` 判定结果**不变**（直接对纯函数断言，不经过 `admit` 的 Skip 分支）
    - ③静态：集合唯一来源是 `DEEPSEEK_HOSTS`（`rg -c "pub const DEEPSEEK_HOSTS"` == 1；`rg -n "LLM_ALLOWLIST" crates/infrastructure/src` 只 `gate.rs`）
    - 文件仍零 socket
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_gate` 退出码 0，用例数 = 基线 + ≥1

- [ ] **L27.3 重定义 `scripts/check-model-candidates.sh`（消除退出码 2）（★）**
  - 追溯：AC-29、AC-37、R35；SPEC 11.10.5-2、11.9.2-3；DESIGN 2.18.5；D-4
  - 前置：L17.2（既有脚本）、L27.1
  - 并行：⟂ R2：与 L24.2、L25.2、L23.2 并行（不同文件）
  - 触达：`scripts/check-model-candidates.sh`（重写判定面）
  - DoD：
    - 不再依赖规格小节：`rg -n "11\\.9\\.2" scripts/check-model-candidates.sh` 退出码 1
    - **判定面 A（默认/回退）**：联调路径（`crates/infrastructure/src/llm/live/**` 与 `gateway.rs`）出现 `LLM_MODEL` 的 `||` 回退、`model: ... unwrap_or`、`unwrap_or_else(... "...")` → 退出码 1
    - **判定面 B（代码字面量）**：扫描面限定 `crates app/src scripts`；模型字段被赋字符串字面量且**不在占位白名单**（`test-model|fake-model|fixture-model|mock-model|placeholder-model`）→ 退出码 1
    - **判定面 C（文档表述面）**：`docs .codeartsdoer/specs` 中出现"候选模型/默认模型/回退模型"与具体模型字面量并列的结构 → 退出码 1；规则描述行以固定标记豁免（见备注〔E3〕）
    - **判定面 D（自举）**：脚本自身进入扫描面且不含任何真实模型名；`--self-test` 用假候选验证"能报 `文件:行号` 且不回显内容"
    - 退出码：`0` = 四面通过；`1` = 任一命中；`2` = 扫描面缺失/不可读（**禁止当 0**）；命中输出只含 `文件:行号` + 规则编号
    - `LlmConfig::default()` 的离线默认占位处加注释"离线默认占位·不得用于真实请求"，并在脚本中登记为"非联调路径例外"（唯一例外，须逐行列出）
    - `bash scripts/check-model-candidates.sh` **退出码 0**（消除阶段 D 遗留的 `2`）
    - `bash scripts/check-model-candidates.sh --self-test` 退出码 0；`bash -n scripts/check-model-candidates.sh` 退出码 0
  - 备注〔E3〕：判定面 B 的扫描面相对 DESIGN 2.18.5 的 B 命令**收窄**为 `crates app/src scripts`（代码面），文档面改由判定面 C 承担；理由是 B 的原正则会把规格/任务清单中的**规则描述行**判为命中。该收窄登记为受控偏差，须在 L28.2 记录；**不得**用收窄掩盖真实模型字面量。

- [ ] **L27.4 `host_locked` 观测与 `LLM_ALLOWLIST` 启动提示**
  - 追溯：AC-37；SPEC 11.10.5-1；DESIGN 2.18.2、2.18.3-3；R32
  - 前置：L27.1、L18.1（`RunRecord.host_locked`）
  - 并行：⟂ R4：与 L27.2 并行（不同文件）
  - 触达：`crates/infrastructure/src/llm/live/gate.rs`（`Admit` 分支）、`scripts/run-llm-live.sh`（提示一行）
  - DoD：
    - `Admit` 分支的 `host_locked = true` 由 `host_is_locked` 的**既有返回值**推导，不新增第二次判定
    - 启动提示固定文案含"`LLM_ALLOWLIST` 仅作存在性检查，不参与 host 判定"：`rg -n "仅作存在性检查" crates/infrastructure/src scripts` 命中 ≥1
    - `Reject`/`Skip` 路径不写任何数值（沿用 v2 三态约束）
    - `bash -n scripts/run-llm-live.sh` 退出码 0
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_offline` 退出码 0

## L28 C5（未轮换）留痕（E6 · AC-32 前置风险）

**任务组目标**：把"凭据未轮换"从口头风险变成**记录里固定的布尔 + 固定风险文案**，且不污染三态语义、不触发 `not_measured` 的数值约束。

- [ ] **L28.1 记录固定 `credential_rotated = false` 与风险文案（★）**
  - 追溯：C5、AC-32；SPEC 11.10.7-C5-事实、11.9.8-C5、11.9.5-3；DESIGN 2.19.1
  - 前置：L18.1（v2 字段已存在）、L26.2（记录器已接入装配）
  - 并行：⟂ R2/R3：与 L23.2、L27.3、L25.3 并行（不同文件）
  - 触达：`crates/infrastructure/src/llm/live/record.rs`（常量与 `summary.md` 首行）、`crates/infrastructure/src/llm/live/mod.rs`（装配时写默认值）
  - DoD：
    - 本次运行所有 `run.json` 的 `credential_rotated` 恒为 `false`，**不由任何环境变量控制**：`rg -n "LLM_.*ROTAT" crates scripts` 退出码 1
    - `summary.md` **首行**固定含"凭据未轮换·存在既暴露风险"：`rg -c "凭据未轮换·存在既暴露风险" crates/infrastructure/src/llm/live/record.rs` ≥1
    - `credential_not_rotated` **不进入** `sub_status`：`rg -n "credential_not_rotated" crates/infrastructure/src/llm/live/record.rs` 只出现在注释/声明常量，不出现在 `SubStatus.reason_code` 允许集合
    - 既有约束不被破坏：`NotMeasured ⇒ calls_used == 0`（`record.rs` 既有 `validate` 分支零改动）；内联新增用例 ≥1：`unrotated_flag_does_not_force_not_measured`
    - 记录与脚本不含凭据任何片段：`bash scripts/scan-secrets-docs.sh --all-md --artifacts` 退出码 0（退出码 2 判失败）
    - `cargo test -p meeting-copilot-infrastructure live::record` 退出码 0（既有 ≥6 不减）

- [ ] **L28.2 阶段 E 状态登记与 D-DEADLINE 留痕（不写 `docs/sdd`）**
  - 追溯：C5、D-DEADLINE、AC-33~AC-37；SPEC 11.10.7、11.10.8-4；DESIGN 2.22
  - 前置：L28.1
  - 并行：⟂ 与 L29.*（不同文件）
  - 触达：`docs/progress/b1-status.md`（追加段落，不改既有任务行）
  - DoD：
    - 追加"阶段 E（v1.3）"段：AC-33/AC-34/AC-36/AC-37"离线已判定"；AC-35"载体已落地 / 真实样本待执行"；C5"未轮换已登记（`credential_rotated = false`）"
    - D-DEADLINE 登记："未最终确认；8000 仅为建议值；沿用代码现值 6000（`config.rs:124`）；SPEC 11.10.7 所述 4000 与代码现值不一致，以代码现值为准（受控偏差〔E4〕）"
    - 登记三处受控偏差：〔E1〕`allowlist` 字段移除、〔E2〕`BearerTransport` 落 `live/auth.rs`、〔E3〕候选脚本代码面扫描收窄
    - `git diff --stat docs/sdd` 输出为空
    - 文件内不含 host、模型名、凭据片段：`bash scripts/scan-secrets-docs.sh --all-md` 退出码 0

## L29 测试补齐与网关字段集回归（E7）

**任务组目标**：把 AC-33~AC-37 的离线判定落到可重复执行的测试目标；mock 路径的"字段集"断言只在**允许未知字段**的方向放宽，不删任何既有断言。

- [ ] **L29.1 `tests/llm_gateway_mock.rs` 追加请求体字段集用例**
  - 追溯：AC-33、AC-17；SPEC 11.10.2-4/5；DESIGN 2.14.4、2.20.1
  - 前置：L23.1
  - 并行：⟂ R3：与 L24.4、L25.3、L26.4 并行（不同文件）
  - 触达：`crates/infrastructure/tests/llm_gateway_mock.rs`（**只追加**）
  - DoD：
    - `request_body_field_set`：捕获 mock 收到的请求体 → 必需字段存在（`model` / `temperature` / `max_tokens` / `stream` / `messages` / `response_format`）+ `thinking.type == "disabled"`；断言方式为"必需字段存在 + **允许未知字段**"，**不得**做精确字段集相等
    - 既有 `complete_ok` / `stream_ok` / `non_blocking_capture` 的名称与断言**零改动**（`git diff` 只新增行）
    - `cargo test -p meeting-copilot-infrastructure --test llm_gateway_mock` 退出码 0，用例数 = 基线 + ≥1（既有 ≥3 不减）

- [ ] **L29.2 阶段 E 用例清单与 AC 追溯对表**
  - 追溯：AC-33~AC-37；DESIGN 2.20.1、2.13.3
  - 前置：L23.1、L24.4、L25.3、L26.4、L27.2、L28.1、L29.1
  - 并行：否（汇总）
  - 触达：无源码改动（清单写入 `docs/progress/b1-status.md` 的用例段）
  - DoD：
    - 清单逐条列出并附"命令 + 期望退出码 0"：`thinking_is_fixed_disabled`、`request_body_thinking_field_is_fixed`、`path_confirmed_derived_from_constant`、`path_confirmed_five_values`、`allowlist_cannot_widen_hosts`、`llm_stream_incremental` 五例、`live_assembly_offline_drivable`、`bearer_header_injected_on_send`、`bearer_debug_is_masked`、`request_body_field_set`、`unrotated_flag_does_not_force_not_measured`
    - 默认执行新增计数实测：`cargo test --workspace -- --list | grep -c ": test"` ≥238
    - 不计入默认执行的用例单列：`llm_live.rs` 的 8 个 `#[ignore]`、`llm_transport_tls` 的 ≥6 个 feature 用例
    - 每条用例标注"断言核心"与"对应 AC"，不得出现"以 mock 数值顶替真实"的表述

## L30 门禁阈值与收口审计（E8 · AC-17）

**任务组目标**：让"代码绿、治理红"不可能发生：门禁阈值显式提升到 238，`G-SECRET-DOC` 的退出码 2 判失败，`G-CAND` 的遗留 2 必须消除。

- [ ] **L30.1 全量门禁执行（★）**
  - 追溯：AC-17、AC-33~AC-37；本清单 0.8.4；SPEC 11.10.8
  - 前置：L23~L29 全部
  - 并行：否（收口串行）
  - 触达：无源码改动（只执行与记录）
  - DoD（逐条把命令与输出摘要粘贴到 `docs/progress/b1-status.md` 证据段）：
    - G-FMT 0；G-CLIPPY 0；G-CLIPPY-F 0；G-LAYER 0；G-SECRET 0
    - G-SECRET-DOC：`bash scripts/scan-secrets-docs.sh --all-md --artifacts` **退出码 0**；**退出码 2 记为失败**（不可判定不得记 0）
    - G-CAND：`bash scripts/check-model-candidates.sh` **退出码 0**（阶段 D 遗留的 `2` 已消除）
    - G-TEST：`cargo test --workspace` `failed=0`、`passed ≥ 238`；`cargo test --workspace -- --list | grep -c ": test"` ≥238
    - G-TEST-TLS：`cargo test -p meeting-copilot-infrastructure --features llm-https` `failed=0`、`passed ≥ 152`（feature 基线不回退）、`llm_transport_tls` 用例数不减
    - G-DENY 0；G-OPENSSL 的 `openssl` 与 `openssl-sys` 计数均为 0；G-PRIVKEY 0 命中
    - 模型字面量 0：由 G-CAND 的判定面 B/C 承担（不在本清单重复写模型家族正则）
    - 连续 3 次 G-TEST 结果一致（`failed=0` 且 `passed` 计数相同）

- [ ] **L30.2 禁止项审计与变更外移交登记**
  - 追溯：AC-17、AC-32；SPEC 9.3、11.6-3、11.10.8-4；DESIGN 2.20.3
  - 前置：L30.1
  - 并行：否
  - 触达：无（只审计与登记）
  - DoD：
    - `git status --short` 中无 `.tmp/llm-live` 入库、无新增凭据文件
    - `git diff --stat docs/sdd` 输出为空
    - CI 零依赖：`rg -n "LLM_LIVE|LLM_API_KEY" .github` 0 命中（退出码 1 视为通过）
    - 默认值审计：`rg -n "unwrap_or\\(8_000\\)|LLM_LIVE_DEADLINE_MS.*8000" crates scripts` 0 命中（退出码 1 视为通过）
    - 交付说明登记"变更外移交项"：`docs/sdd/tasks.md` 的勾选由发布责任人手工执行，本清单不执行
    - 登记"不得据此宣称"清单：13.1/13.2 真实端点已验证、13.4 完成、AC-18 已实测、**AC-20 已实测（AC-36 未通过前）**

## L31 真实执行（E9 · 环境受限，非输入受限）

> **前置声明（强制）**：
> 1. 本组**不是输入阻塞**：C1~C4 已确认、M6/M7 已确认、C5 已答复"未轮换"；阻塞只来自**执行环境**（沙箱 bash 不可用，R33）。
> 2. `LLM_LIVE_DEADLINE_MS` **未最终确认**：不得写死 8000；未设置该变量时沿用代码现值（`config.rs:124` 的 6000）。
> 3. 沙箱 bash 不可用时，**允许直接**执行 `cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --ignored --test-threads=1`；但 `LLM_LIVE` / `LLM_BASE_URL` / `LLM_MODEL` / `LLM_MODEL_CONFIRMED` / `LLM_API_KEY` / `LLM_LIVE_CALL_CAP` / `LLM_ALLOWLIST` **必须由用户在自己的终端注入**，不得由 agent 写入代码、文件或命令行参数。
> 4. **AC-20 在 AC-36 通过前不得标"实测"**；AC-35 通过也不等于 AC-20 通过。

- [ ] **L31.1 环境受限登记与执行前置检查（★）**
  - 追溯：AC-35、AC-20~AC-25；SPEC 11.10.3、11.10.7；DESIGN 2.20.3、R33
  - 前置：L30.1
  - 并行：否
  - 触达：无源码改动（只登记）
  - DoD：
    - 前置清单逐项勾选并留痕：C1（路径段常量已确认）、C2（`LLM_MODEL` 与 `LLM_MODEL_CONFIRMED` 逐字节相等）、C3（`LLM_LIVE_CALL_CAP` 已确认）、C4（host 集合仅代码常量）、C5（未轮换已登记）
    - `LLM_LIVE_DEADLINE_MS` 未最终确认 → 不设该变量；`rg -n "8_000" crates/infrastructure/src/llm/live/config.rs` 只允许出现在范围校验 `100..=8_000` 中，**不得**出现在 `unwrap_or`
    - 沙箱 bash 不可用（Win32 error 5）→ 脚本类门禁（G-SECRET-DOC / G-CAND / G-TEST-TLS）标"未验证（环境受限）"，**不记 0、不记通过**
    - 登记"环境受限 ≠ 输入受限"：输入侧无未决项，仅执行环境受限；替代入口见 L31.2
    - 若 `cargo` 在沙箱可用而 bash 不可用，先执行 `cargo test --workspace` 与 `cargo test --features llm-https` 取证，再按替代入口执行 L31.2

- [ ] **L31.2 真实采样执行（用户终端注入环境变量）（★）**
  - 追溯：AC-35、AC-20~AC-25、AC-31；SPEC 11.10.3-1/4/5、11.9.4；DESIGN 2.16.4、2.16.6、2.20.3
  - 前置：L31.1；用户终端已注入全部必需变量
  - 并行：否（串行采样，`--test-threads=1`、`concurrency=1`）
  - 触达：无源码改动；产物 `.tmp/llm-live/<run-id>/`
  - DoD：
    - 首选入口：`bash scripts/run-llm-live.sh`（普通终端 / WSL）
    - **沙箱 bash 不可用时的替代入口**：`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --ignored --test-threads=1`；环境变量由用户终端注入（agent 只读不写）
    - 冒烟：`LLM_LIVE_SAMPLES=1` → `calls_used == 1`、`budget_hit == false`、`samples.jsonl` ≥1 行（`attempt ≥ 1`、`total_ms > 0`、无占位标记）
    - 鉴权失败（401/403）→ 中止、`sub_status = not_measured`、**不继续消耗配额**、错误文本不含凭据任何片段
    - 四类任务按预算采样；触顶 → `budget_hit = true`、`calls_used == call_limit`、`samples.jsonl` 行数**不减少**
    - 429 未出现 → `not_triggered` + 采样窗口 + 请求数（**不得写"通过"**，不得填 mock 的 429 结果）
    - AC-24/AC-25 记录必须带"人为压低截止时间"/"人为构造的违规请求"标注
    - 进程命令行 0 命中凭据；四泄漏面抓取通过
    - 退出码 0；**AC-20 在 AC-36 通过前只能标"未实测（流式读取未增量）"**

- [ ] **L31.3 记录 v2 复核与三态移交（不改 `docs`）**
  - 追溯：AC-27、AC-32、AC-35；SPEC 11.10.3-7、11.7、11.5.9；DESIGN 2.16.7、2.9.4
  - 前置：L31.2
  - 并行：否
  - 触达：无（只读复核；结论写入 `docs/progress/b1-status.md` 的联调段）
  - DoD：
    - `run.json` v2 全字段齐备（含 `path_confirmed`、`credential_rotated = false`、`call_limit`、`calls_used`、`budget_hit`、`model_confirmed`、`host_locked`、`tls_trust_root`）
    - `samples.jsonl` 行数 == 有效样本数；每行 `attempt ≥ 1`、`total_ms > 0`；`samples_path` 为相对路径且位于 `.tmp/llm-live`
    - `node scripts/llm-live-recompute.mjs` 重算 p50/p95 与记录差 ≤0.1 ms
    - `bash scripts/scan-secrets-docs.sh --all-md --artifacts` 退出码 0（2 = 不可判定，记失败）
    - 三态结论移交 13.4；AC-18 状态变更**不由本清单执行**
    - 不写 `docs/baseline/*`、不改 `docs/sdd/*.md`

---

# 附录 I：AC-33~AC-37 覆盖矩阵（阶段 E）

| AC | 判定命令 / 用例 | 主要任务 | 类别 | 追溯（SPEC / DESIGN） | 备注 |
|---|---|---|---|---|---|
| AC-33 | `cargo test -p meeting-copilot-infrastructure --test llm_live_gate thinking_is_fixed_disabled`；`gateway::request_body_thinking_field_is_fixed`；`llm_gateway_mock::request_body_field_set` | L23.1、L23.2、L29.1 | O | SPEC 8.5-AC-33、11.10.2；DESIGN 2.14、2.20.1 | 正例 2（一次性/流式）+ 反例 3（缺失/非 disabled/空对象）；`thinking_not_disabled` 不进 `ErrorCode` |
| AC-34 | `--test llm_live_gate path_confirmed_derived_from_constant`；`live::policy::path_confirmed_five_values`；静态 grep | L24.1、L24.2、L24.3、L24.4 | O | SPEC 8.5-AC-34、11.10.4；DESIGN 2.15、2.20.1 | 仅 `"/chat/completions"` 得 `true`；生产代码手写 `true` 0 命中；path 原文不入记录 |
| AC-35 | `cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --ignored --test-threads=1`；`--test llm_live_offline live_assembly_offline_drivable` | L26.1、L26.2、L26.3、L26.4、L31.2 | L + O | SPEC 8.5-AC-35、11.10.3；DESIGN 2.16、2.20.1 | 显式路径：`samples.jsonl ≥1`、`attempt ≥1`、`total_ms > 0`、v2 键集；默认路径：0 构造/0 socket/0 样本/退出 0 |
| AC-36 | `--test llm_stream_incremental` 五例 | L25.1、L25.2、L25.3 | O | SPEC 8.5-AC-36、11.10.6；DESIGN 2.17、2.20.1 | 首增量 ≤200 ms；取消 ≤50 ms；截止 ≤deadline+200 ms；跨块一致；`TooLarge` |
| AC-37 | `--test llm_live_gate allowlist_cannot_widen_hosts`；`live::policy` 内联；`scripts/check-model-candidates.sh` | L27.1、L27.2、L27.3、L27.4 | O | SPEC 8.5-AC-37、11.10.5；DESIGN 2.18、2.20.1 | 三断言；`DEEPSEEK_HOSTS` 定义唯一；`LLM_ALLOWLIST` 只判存在性 |
| AC-29 回归 | `bash scripts/check-model-candidates.sh` | L27.3 | O | SPEC 11.9.2-3；DESIGN 2.18.5 | 退出码 0/1/2；不再依赖 11.9.2 小节 |
| AC-32 前置风险 | `run.json.credential_rotated == false`；`summary.md` 首行风险文案 | L28.1 | O | SPEC 11.10.7-C5、11.9.8-C5；DESIGN 2.19.1 | 不阻塞离线判定；风险标注不得隐去 |
| AC-17 | G-FMT/G-CLIPPY/G-CLIPPY-F/G-TEST(≥238)/G-LAYER/G-DENY/G-COUNT | L30.1 | O | SPEC 8.1-AC-17；DESIGN 2.20.3 | 222 基线不回退；只增不减 |

**覆盖结论（阶段 E）**：AC-33/AC-34/AC-36/AC-37 全部可在断网下判定，均有唯一主责任务与可执行命令；AC-35 的"载体可用、非占位、默认 0 请求"在离线可判定，真实样本需 L31 在用户终端执行；AC-32 的 C5 留痕为声明面，不改变任何既有判定。**AC-20 在 AC-36 通过前不得标"实测"**。

# 附录 J：阶段 E 依赖图（简版）

```
L23.1 ─> L23.2 ─────────────┐
L24.1 ─> L24.2 ─> L24.3 ─┬─> L24.4 ─> L27.2
                          ├─> L26.2 ─> L26.3 ─┐
L25.1 ─> L25.2 ─> L25.3 ──┤                   ├─> L29.2 ─> L30.1 ─> L30.2
L26.1 ────────────────────┘                   │
L27.1 ─┬─> L27.3                              │
       └─> L27.4                              │
L28.1 ─> L28.2 ───────────────────────────────┘
L29.1 ────────────────────────────────────────┘

L30.1 ─> L31.1 ─> L31.2 ─> L31.3
```

**关键路径（E 版主链，9 跳）**：`L24.1 → L24.2 → L24.3 → L26.2 → L26.3 → L30.1 → L31.1 → L31.2 → L31.3`。
**次长链**：增量读取 `L25.1 → L25.2 → L25.3`；思考模式 `L23.1 → L23.2`；候选脚本 `L27.1 → L27.3`。
**并行簇**：R1 = L23.1/L24.1/L25.1/L26.1/L27.1；R2 = L24.2/L25.2/L27.3/L23.2；R3 = L24.4/L25.3/L26.4/L29.1；R4 = L27.2/L27.4。

# 附录 K：阶段 E 可立即执行的首批任务

| 顺序 | 任务 | 动作 | 文件 | 前置 |
|---|---|---|---|---|
| 1 | **L23.1** | 抽出 `request_body` / `thinking_verdict`，固定 `thinking.type = disabled` | `crates/infrastructure/src/llm/gateway.rs` | 无（既有 L6.2/L6.3） |
| 2 | **L24.1** | 新增 `PATH_CONSTANT` 与 `path_confirmed` 纯函数 | `crates/infrastructure/src/llm/live/policy.rs` | 无（既有 L14.1） |
| 3 | **L25.1** | 泛型 `IncrementalBodyReader` + `ChunkedDecoder` | `crates/infrastructure/src/llm/transport.rs` | 无（既有 L3.1/L3.2/L16.2） |
| 4 | **L26.1** | 新增 `BearerTransport`（凭据注入装饰器） | `crates/infrastructure/src/llm/live/auth.rs`（新增 #31）、`live/mod.rs` | 无（既有 L9.2/L3.2） |
| 5 | **L27.1** | 移除 `LiveRunConfig.allowlist` 数据流 | `crates/infrastructure/src/llm/live/config.rs` | 无（既有 L14.2） |

> 五项**互不共享可写文件**（`config.rs` 在 L24.2 之前只被 L27.1 触碰，故 L24.2 与 L27.1 需按 L24.2 → L27.1 顺序），可并行或单人顺序执行。第二批（依赖首批）：L23.2、L24.2、L25.2、L27.2/L27.3、L28.1、L29.1。

# 附录 L：阶段 E 收口检查单（复制即用）

```
[ ] G-FMT         cargo fmt --all --check                                          → 0
[ ] G-CLIPPY      cargo clippy --workspace --all-targets -- -D warnings            → 0
[ ] G-CLIPPY-F    cargo clippy -p meeting-copilot-infrastructure --all-targets \
                    --features llm-https -- -D warnings                             → 0
[ ] G-TEST        cargo test --workspace                                           → failed=0, passed ≥ 238
[ ] G-COUNT       cargo test --workspace -- --list | grep -c ": test"              → ≥ 238
[ ] G-TEST-TLS    cargo test -p meeting-copilot-infrastructure --features llm-https → failed=0, passed ≥ 152
[ ] G-LAYER       bash scripts/check-layering.sh                                   → 0
[ ] G-SECRET      bash scripts/scan-secrets.sh                                     → 0
[ ] G-SECRET-DOC  bash scripts/scan-secrets-docs.sh --all-md --artifacts            → 0（2 = 失败）
[ ] G-CAND        bash scripts/check-model-candidates.sh                            → 0（2 = 失败，阶段 D 遗留需消除）
[ ] G-DENY        cargo deny check licenses bans sources advisories                → 0
[ ] G-OPENSSL     cargo tree ... --features llm-https -e normal | grep -cE "openssl|openssl-sys" → 0
[ ] G-PRIVKEY     git grep -nE "BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY"             → 0 命中
[ ] 模型字面量   由 G-CAND 判定面 B/C 覆盖（不重复写正则）                          → 0
[ ] 未写死 8000   rg -n "unwrap_or\(8_000\)|LLM_LIVE_DEADLINE_MS.*8000" crates scripts → 0 命中
[ ] 状态保持      git diff --stat docs/sdd                                         → 空
[ ] 零入库        git status --short | grep '.tmp/llm-live'                        → 空
[ ] 不得宣称      AC-20 在 AC-36 通过前不得标"实测"；AC-35 ≠ AC-20                  → 记录在案
```
---

# **阶段 F：请求体 JSON 约束、4xx 分类与可观测落盘（v1.4 增量，L32~L38）**

> **追加声明（强制）**：本阶段**只追加**于文末，不改动 L1~L31 的**任何标题、DoD 文字、勾选状态与编号**；0.1~0.9 之外，0.1~0.8 与附录 A~L 一字不改。阶段 F 与既有条目的接续/取代关系只在 0.9.7 与附录 N 登记。
> **源文档**：SPEC 8.6（AC-38~AC-42）、SPEC 11.11（11.11.1~11.11.7）；DESIGN v1.4 的 2.23~2.31（AC-38~AC-42、R36~R40、D-5~D-9、Q-1~Q-4）。
> **判定性质**：AC-38~AC-42 属 **补充门槛项续篇**，**不计入** 8.1 本轮完成判据，且不改动 AC-01~AC-37 的任何编号、判定方式与通过条件。任一项不通过时只使相关真实采样结论不得标"实测成功"，不改变 8.1~8.5 的既有结论。
> **零转录（硬约束）**：本阶段正文**不出现**完整端点 URL、host 与 path 拼接的 URL 字面量、任何模型名字面量、凭据任何可见片段与 Tracking ID 任何片段。路径只以路径段常量 `"/chat/completions"` 表达；host 只在代码常量 `DEEPSEEK_HOSTS` 中。
> **阶段 E 状态（不重开）**：`gateway::request_body`（thinking disabled）、`policy::PATH_CONSTANT`/`path_confirmed`、`BearerTransport`、`assemble_live`、`IncrementalBodyReader`、记录 v2、8 个 `#[ignore]` 真实采样用例均已落地；门禁证据为 `cargo test --workspace` **238 passed / 0 failed**，fmt/clippy（含 feature）/层禁/密钥/文档扫描/候选脚本/deny 通过，openssl 0。本阶段以此为**基线**，只增不减。

## 0.9 阶段 F 总览

### 0.9.1 组与主任务映射

| 组 | 主任务 | 主题 | 覆盖 AC | SPEC 小节 | DESIGN 小节 |
|---|---|---|---|---|---|
| **F1** | L32 | system prompt 显式 JSON 指令 | AC-38 | 8.6-AC-38、11.11.2-1~6 | 2.23.1~2.23.6、2.30.1-D-9、R36 |
| **F2** | L33 | 4xx 分类矩阵纠正 + AC-09 夹具修正 | AC-39 | 8.6-AC-39、11.11.3-1~9、11.11.1-3 | 2.24.1~2.24.7、R37 |
| **F3** | L34 | `http_status` 全构造点落盘 | AC-40 | 8.6-AC-40、11.11.4-1/6、11.11.5-1 | 2.25.1~2.25.4、D-5、D-6、R38 |
| **F4** | L35 | `total_ms` 来自单调时钟、亚毫秒不补记 | AC-41 | 8.6-AC-41、11.11.4-2/3 | 2.26.1~2.26.5、D-8、R40 |
| **F5** | L36 | `run_id` 隔离与记录合并 | AC-42 | 8.6-AC-42、11.11.4-4/5 | 2.27.1~2.27.6、D-7、R39 |
| **F6** | L37 | 五条 AC 的测试与夹具补齐 | AC-38~AC-42、AC-09 | 8.6 全表 | 2.28.1~2.28.3 |
| **F7** | L38 | 门禁阈值与真实重跑（环境受限） | AC-17、AC-38~AC-42 真实面 | 8.6 判定环境约束、11.11.5/6 | 2.28.2、2.31.4、R33 |

### 0.9.2 关键路径与并行矩阵

```
★ 关键路径（阶段 F 主链，9 跳）：
L33.1 → L33.2 → L34.1 → L34.2 → L34.3 → L35.1 → L36.3 → L38.1 → L38.2

★ 次长链（不进入主链，但 L34.3 前必须完成）：
请求体链：L32.1 → L32.2 → L32.3
运行隔离链：L36.1 → L36.2 → L36.3

⟂ 并行簇（不同文件）：
  S1（无源码前置，立即可做）：L32.1（gateway.rs） ‖ L33.1（degrade.rs） ‖ L34.1（domain llm.rs） ‖ L36.1（live/run_id.rs 新增）
  S2（依赖 S1）：L32.2（gateway.rs） ‖ L33.3（llm_degrade_inject.rs 夹具，需 L33.2） ‖ L34.2（gateway.rs，需 L33.2） ‖ L36.2（record.rs）
  S3（不同测试文件）：L37.1（llm_live_gate） ‖ L37.2（llm_degrade_inject） ‖ L37.3（llm_gateway_mock） ‖ L37.4（llm_live_offline）
  S4（同文件串行）：L36.4 → L37.4；L33.3 → L37.2
串行汇聚点：L32.3/L34.3/L35.1/L36.3（同写 tests/llm_live.rs）、L38.1（需 L32~L37）、L38.2（需 L38.1 + 用户注入）、L38.3（需 L38.1）
```

**不可并行（同文件须串行）**

- `crates/infrastructure/src/llm/gateway.rs`：L32.1 → L32.2 → L33.2 → L34.2。
- `crates/infrastructure/tests/llm_live.rs`：L32.3 → L34.3 → L35.1 → L36.3。
- `crates/infrastructure/tests/llm_degrade_inject.rs`：L33.3 → L37.2。
- `crates/infrastructure/tests/llm_live_offline.rs`：L36.4 → L37.4。
- `crates/infrastructure/src/llm/live/record.rs`：L36.2（`sample_count`/`per_task`）先于 L37.4 的断言。
- `crates/infrastructure/src/llm/live/mod.rs`：L36.1（导出 `run_id`）。

### 0.9.3 阶段 F 文件清单

**新增 1 个文件（编号接阶段 E 的 #32）**

| # | 文件 | 承载职责 | AC | 任务 |
|---|---|---|---|---|
| 33 | `crates/infrastructure/src/llm/live/run_id.rs` | `new_run_id` / `runtime_entropy` 纯函数；零依赖、默认构建、不含 host/路径/模型/凭据 | 42 | L36.1 |

**修改 11 个文件（F-1~F-11，对齐 DESIGN 2.31.2）**

| # | 文件 | 改动内容 | 不改动内容 | AC | 任务 |
|---|---|---|---|---|---|
| F-1 | `crates/infrastructure/src/llm/gateway.rs` | `system_prompt` 接入 JSON 指令；新增 `json_instruction_verdict`/`ensure_json_instruction` 与常量；4xx 分支改调 `classify_http`；成功/降级构造点填 `http_status` | SSE 解析、首 token、终止唯一、重试/冷却/截止编排不变 | 38/39/40 | L32.1、L32.2、L33.2、L34.2 |
| F-2 | `crates/infrastructure/src/llm/degrade.rs` | 新增 `ContentPolicyMarker`、`classify_http`、`content_policy_marker`；`content_filtered()` 调用点收敛 | `classify`/`classify_status`/8 条传输错误映射/重试上限不变 | 39 | L33.1 |
| F-3 | `crates/domain/src/model/llm.rs` | `StructuredOutcome` 新增 `http_status: Option<u16>`；内联构造补字段 | 既有字段、`FailureClass`、`DegradeCause`、`ErrorCode` 不变 | 40 | L34.1 |
| F-4 | `crates/infrastructure/src/llm/live/stats.rs` | 注释明确 `total_ms` 整数毫秒与 `precision_insufficient` 语义 | `Sample` 字段、nearest-rank、降级排除不变 | 41 | L35.2 |
| F-5 | `crates/infrastructure/src/llm/live/record.rs` | 新增 `sample_count()`；`write_run` 前取累计值；`per_task` 语义收紧注释 | 原子写、路径校验、三态序列化、键集不变 | 42 | L36.2 |
| F-6 | `crates/infrastructure/src/llm/live/mod.rs` | 导出 `run_id` 模块；会话共享 `BudgetLedger` 与 `run_id` 的装配说明 | `assemble_live` 签名（已含 `run_id`）不变 | 42 | L36.1 |
| F-7 | `crates/infrastructure/tests/llm_live.rs` | `OnceLock` 共享 `run_id`；`Instant` 计时；`seq` 运行级递增；真实状态写入样本；401/402/403 中止；`run.json` 合并写 | 8 个 `#[ignore]` 用例名不变；环境变量仍由用户终端注入 | 39/40/41/42 | L32.3、L34.3、L35.1、L36.3 |
| F-8 | `crates/infrastructure/tests/llm_live_gate.rs` | 追加 AC-38 正例/反例用例 | 既有 AC-33/34/37 用例不改 | 38 | L37.1 |
| F-9 | `crates/infrastructure/tests/llm_degrade_inject.rs` | AC-09 内容过滤夹具改结构化标记；追加 AC-39 矩阵与 AC-40 状态断言 | `all_failures_no_panic` 的用例数与六类判定不变 | 39/40 | L33.3、L37.2 |
| F-10 | `crates/infrastructure/tests/llm_gateway_mock.rs` | `request_body_field_set` 追加 JSON 指令断言；追加 `http_status == None` 断言 | 既有断言不删、只放宽精确文本/字段集比较 | 38/40 | L37.3 |
| F-11 | `crates/infrastructure/tests/llm_live_offline.rs` | 追加 `total_ms` 假时钟、`run_id`/目录合并、`http_status` 键集用例；`per_task` 夹具与行数一致 | 既有 6 个离线用例与运行门断言不改 | 40/41/42 | L36.4、L37.4 |

**仅状态登记（不改 `docs/sdd`）**：`docs/progress/b1-status.md`（L38.3 追加阶段 F 段）。

**明确不在本阶段修改**：`docs/sdd/spec.md`、`docs/sdd/design.md`、`docs/sdd/tasks.md`（勾选亦需单独授权）；`crates/application/**`；`deny.toml`；`scripts/llm-live-recompute.mjs`（键集已含 `http_status`）；`scripts/check-model-candidates.sh`；`crates/domain/src/model/enums.rs`；`ErrorCode::ALL`；环境变量集合（不新增 `LLM_*`）；`platform-*`、`ui-bridge`、`app/src-tauri`、TS 绑定。

### 0.9.4 阶段 F 门禁阈值

| 代号 | 命令 | 通过条件 | 依据 / 与阶段 E 的差异 |
|---|---|---|---|
| G-FMT | `cargo fmt --all --check` | 退出码 0 | 同 |
| G-CLIPPY | `cargo clippy --workspace --all-targets -- -D warnings` | 退出码 0、零 warning | 同 |
| G-CLIPPY-F | `cargo clippy -p meeting-copilot-infrastructure --all-targets --features llm-https -- -D warnings` | 退出码 0、零 warning | 同 |
| G-TEST | `cargo test --workspace` | `failed=0`；`passed ≥ 250` | **238 基线不回退** + ≥12 新增默认用例（DESIGN 2.28.3） |
| G-COUNT | `cargo test --workspace -- --list \| grep -c ": test"` | ≥ 250 | 同上 |
| G-TEST-TLS | `cargo test -p meeting-copilot-infrastructure --features llm-https` | `failed=0`；`passed ≥ 164`（152 基线 + 12）；`llm_transport_tls` 用例数不减 | 12 个新增 O 用例全部落在 infrastructure crate 默认编译面 |
| G-LAYER | `bash scripts/check-layering.sh` | 退出码 0 | 同 |
| G-SECRET | `bash scripts/scan-secrets.sh` | 退出码 0 | 同 |
| G-SECRET-DOC | `bash scripts/scan-secrets-docs.sh --all-md --artifacts` | **仅退出码 0 通过；退出码 2 = 失败（不可判定不得记 0）** | 同 |
| G-CAND | `bash scripts/check-model-candidates.sh` | **退出码 0**（2 亦判失败） | 同 |
| G-DENY | `cargo deny check licenses bans sources advisories` | 退出码 0 | 同 |
| G-OPENSSL | `cargo tree -p meeting-copilot-infrastructure --features llm-https -e normal \| grep -cE "openssl\|openssl-sys"` | 计数 **== 0** | 同 |
| G-PRIVKEY | `git grep -nE "BEGIN (RSA \|EC \|OPENSSH )?PRIVATE KEY"` | **0 命中** | 新增用例不得引入证书/私钥夹具 |
| G-PLACEHOLDER | `rg -n "max\\(1\\)\|unwrap_or\\(1\\)\|let total_ms = [0-9]" crates/infrastructure/tests/llm_live.rs` | **0 命中**（退出码 1 视为通过） | AC-41：禁止 1 ms 补齐 |
| G-FMT-4XX | `rg -n "400\\.\\.500" crates/infrastructure/src/llm/gateway.rs` | **0 命中**（退出码 1 视为通过） | AC-39：禁止"4xx 非 429 → ContentFiltered"兜底 |

**新增默认执行用例下限（用于 G-TEST 的 250 阈值；逐 AC 计数沿用 DESIGN 2.28.3 的 2/3/3/2/2）**

| 文件 / 位置 | 下限 | 依据（DESIGN 2.28.1、2.28.3） |
|---|---|---|
| `tests/llm_live_gate.rs` | **+2** | `json_instruction_present_for_four_tasks`（AC-38 正例）、`json_instruction_negative_cases`（AC-38 四种反例） |
| `src/llm/degrade.rs` 内联 | **+2** | `classify_http_priority_is_fixed`、`content_policy_marker_rejects_free_text`（AC-39） |
| `tests/llm_degrade_inject.rs` | **+2** | `http_status_classification_matrix`（AC-39）、`outcome_carries_http_status`（AC-40） |
| `tests/llm_gateway_mock.rs` | **+1** | 新增 `mock_outcome_http_status_is_none`（AC-40）；`request_body_field_set` 只追加断言，不计新用例 |
| `tests/llm_live_offline.rs` | **+3** | `samples_http_status_key_is_stable`（AC-40）、`total_ms_from_clock_not_constant`（AC-41，含亚毫秒分支）、`single_run_directory_and_merged_count`（AC-42） |
| `src/llm/live/run_id.rs` 内联 | **+2** | `run_id_is_time_and_entropy_derived`、`run_id_format_is_path_safe`（AC-42） |
| **合计** | **≥12** | 238 + 12 = **250** |

> 说明 1：AC-41 的"静态无字面量"由 G-PLACEHOLDER 承担，不计用例数；AC-42 的"两次独立运行"属真实面（L38.2），不计默认用例数。
> 说明 2：`llm_live.rs` 的 8 个 `#[ignore]` 与 `llm_transport_tls.rs` 的 ≥6 个 feature 用例**不计入** 250；后者只在 `--features llm-https` 下计入 G-TEST-TLS。
> 说明 3：若执行时发现 152 基线已包含阶段 E 的新增用例，G-TEST-TLS 的判定改为"`passed` 不低于阶段 E 实测值 + 本阶段新增默认用例数 12"，差值须在 `docs/progress/b1-status.md` 证据段说明，不得直接放松为 152。

### 0.9.5 阶段 F 禁止事项（追加于 0.5、0.7.5、0.8.5，同等强制）

1. 不得写完整端点 URL、host 与 path 拼接的 URL 字面量、任何模型名字面量、凭据任何可见片段与 Tracking ID 任何片段；扫描命中时只输出`文件:行号`，禁止回显内容。
2. **不得用 1 ms 补齐耗时**：禁止 `max(1)`、`unwrap_or(1)`、`let total_ms = 1`、"+1 凑正数"；亚毫秒只允许 `total_ms = 0` + `excluded = precision_insufficient`（D-8）。
3. **不得出现"`response_format` 无 JSON 指令"的请求体**：`response_format.type == "json_object"` 与 system prompt 含 `json`/`JSON` 必须同时存在；禁止为规避 4xx 而移除 `response_format`、改非 `json_object` 或关闭结构化输出（SPEC 11.11.2-5）。
4. 不得把所有非 429 的 4xx 统一归 `ContentFiltered`；不得按响应正文自由文本猜测内容策略（SPEC 11.11.3-1/8）。
5. 不得让 401/402/403 进入重试（`retry_allowance(Unavailable)=2` 只适用于 5xx 与传输错误）；4xx 分支必须直接 `break`（R37）。
6. 不得让 `http_status` 进入 Schema 校验、六类分布、成功率分子、审计字段或日志白名单；不得为流式 2xx 伪造 `Some(200)`（R38、D-5）。
7. 不得以"同一次运行的 8 个用例各写一目录/各写 `per_task=1`"冒充合并；`run.json.per_task` 必须等于 `samples.jsonl` 行数（D-7）。
8. 不得以进程号作为 `run_id` 的唯一来源；同一次运行内 8 个用例必须共用同一 `run_id`（AC-42）。
9. 不得新增 `LLM_*` 环境变量；不得由 agent 向代码、文件或命令行写入任何注入值（凭据、端点、模型、预算）。
10. 不得删除/放宽既有断言以让新字段或新分类通过；mock 断言只允许"精确文本/精确字段集"放宽为"必需片段存在 + 允许新增字段"（AC-17）。
11. 不得回改 L1~L31 的任何条目；阶段 F 的接续/取代关系只在 0.9.7 与附录 N 登记。
12. 不得修改 `docs/sdd/*.md`；阶段 F 的进度登记只写 `docs/progress/b1-status.md` 与 `.tmp/llm-live/` 产物。

### 0.9.6 输入确认状态与执行约束

| 编号 | 事项 | 状态（本阶段生效） | 对阶段 F 的影响 |
|---|---|---|---|
| J1 | JSON 指令最小要求 | **已确认：更严**——四类任务的 system prompt 各至少一处 `json`/`JSON` | L32.2 只认 `role=system` 的 content，不认 user/Schema/`response_format` |
| J2 | 402 语义 | **已确认：余额/额度不足 → `Unavailable`** | L33.1 优先级 3；记录保留原始 402 供复盘 |
| J3 | 401/402/403 | **已确认：中止采样**，不仅记录后继续 | L33.2 4xx 直接 break；L36.3 中止标志 |
| J4 | `total_ms` 精度 | **已确认：整数毫秒** | L35.1 `as_millis()`；亚毫秒 0 + 标注 |
| J5 | 离线 `http_status` | **已确认：允许 `null` 或省略**（本版固定 `null`，保键存在） | L34.2 本地短路/传输错误填 `None`；L37.4 键集断言 |
| Q-1 | D-5 流式 `Completed` 的 `http_status=None` | **未最终确认 → 按默认执行**：流式样本 `not_measured` + `reason=stream_status_unavailable`；一次性照常落盘 | L34.3 |
| Q-2 | D-7 `per_task` 语义收紧 | **未最终确认 → 按默认执行**：不新增 `samples_count` 键，键集与 `schema_version` 不变 | L36.2 |
| Q-3 | D-6 domain 新增 `http_status` | **未最终确认 → 按 AC-40 原文执行**（必须新增）；登记为受控偏差〔F1〕 | L34.1 |
| Q-4 | 内容策略错误码白名单 | **未最终确认 → 白名单只含 `content_filter` 结构化标记**；新增取值需规格变更 | L33.1 |

**执行环境约束（强制）**

1. **真实重跑前置**：`LLM_LIVE`、`LLM_BASE_URL`（含已确认 path）、`LLM_API_KEY`、`LLM_MODEL`、`LLM_MODEL_CONFIRMED`、`LLM_LIVE_CALL_CAP`、`LLM_ALLOWLIST` 七项**必须由用户在自己的终端注入**。
2. **agent 会话无凭据**：agent 只能执行离线门禁与静态断言；**不得**读取、回显、写入任何注入值；不得把凭据放进命令行参数、临时文件或脚本。
3. **环境受限 ≠ 输入受限**：输入侧 J1~J5 已确认、Q-1~Q-4 有默认口径；真实面阻塞只来自执行环境。bash 不可用时，脚本类门禁标"未验证（环境受限）"，**不记 0、不记通过**。
4. **AC-20 仍不因本次修复自动满足**：AC-40/AC-41 通过只表示"状态与耗时如实落盘"；AC-20 仍需真实有效样本，且本次真实采样若全部 `degraded=true`，只能标"未实测（全部降级）"（SPEC 11.11.5-2）。

### 0.9.7 与既有条目的接续关系（只登记，不重写）

| 既有条目 | 既有内容（保持不变） | 阶段 F 接续关系 | 处理方式 |
|---|---|---|---|
| L23.1/L23.2 | `request_body` 与 `thinking` 固定、`thinking_not_disabled` 标注 | L32.1/L32.2 在**同一函数**内追加 JSON 指令与判定，`thinking` 字段零改动 | 保留勾选状态；`build_http` 只多一处前置校验 |
| L29.1 | `request_body_field_set`（必需字段存在 + 允许未知字段） | L37.3 只**追加**"system 含 JSON 指令"与 `http_status == None` 断言 | 不删除既有断言 |
| L26.3 | 8 个 `#[ignore]` 接入真实采样链路 | L32.3/L34.3/L35.1/L36.3 在**同一文件**补语义；8 个用例名不变 | 保留勾选状态；不新增第 9 个用例名 |
| L18.1 | `RunRecord` v2 键集 | L36.2 只改 `per_task` 取值语义，键集/`schema_version` 零改动 | 不重写 L18.1 |
| L10.2 | `LiveRecorder` 原子写与三态 | L36.2 新增 `sample_count()`，原子写与三态序列化不变 | 只增方法 |
| L28.1 | `credential_rotated=false` 与风险文案 | 阶段 F 不改；L38.2 的真实重跑仍沿用 | 不重复计数 |
| L30.1 | 阶段 E 门禁（≥238） | L38.1 以 **≥250** 接续；238 不回退 | 保留 L30.1 文字 |
| L31.1~L31.3 | 阶段 E 真实执行（环境受限） | L38.2 以同一"用户终端注入"口径接续；只更新三态，不改旧 AC | 保留 L31 未勾选原状 |
| L13.3/L13.4 | 真实采样与复核 | 执行入口不变（`run-llm-live.sh` / `llm_live --ignored`）；新增字段进入复核项 | 不重写其 DoD |

**受控偏差登记（落地时执行，禁止回改 spec.md / design.md）**

| 编号 | 偏差 | 原因 | 登记位置 |
|---|---|---|---|
| 〔F1〕 | `crates/domain/src/model/llm.rs` 新增 `StructuredOutcome.http_status: Option<u16>`，突破阶段 E"不改 domain"声明（D-6） | AC-40 要求结构化结果携带真实状态；只加一个字段，不改既有字段/枚举/错误码 | L34.1 备注、L38.3 |
| 〔F2〕 | `run.json.per_task` 语义由"配置的每任务样本数"收紧为"实际样本行数"（D-7） | AC-42 要求样本计数等于 `samples.jsonl` 行数 | L36.2 备注、L38.3 |
| 〔F3〕 | 亚毫秒 `total_ms = 0` + `excluded = precision_insufficient`；无 HTTP 响应样本 `http_status = None` + `excluded = no_http_response`（D-8） | 消解 AC-35"每行 `total_ms > 0`"/AC-40"每行整数"与 AC-41/传输失败的张力 | L35.1、L34.2 备注、L38.3 |
| 〔F4〕 | 反例 body 仅经 `#[cfg(test)]` 派生构造器生成（D-9） | 生产构建不得存在"关闭 JSON 指令"的路径 | L32.2 备注 |
| 〔F5〕 | 流式 `Completed` 的 `http_status = None` 并标 `not_measured`（D-5） | 读取器不为 2xx 暴露状态；不伪造 200 | L34.3 备注、L38.3 |

---

# 阶段 F：请求体契约、4xx 分类与可观测落盘（L32~L38）

## L32 请求体 system prompt 显式 JSON 指令（F1 · AC-38）

**任务组目标**：把"`json_object` 模式下提示词必须含 JSON 指令"从服务商隐式行为变为**单点构造 + 纯函数判定 + 发送前拦截**；`thinking` 与 `response_format` 三者共存，mock 与真实共用同一份 body。

- [ ] **L32.1 抽出 `system_prompt` 并注入 JSON 指令常量（★）**
  - 追溯：AC-38；SPEC 8.6-AC-38、11.11.2-1/3/4/5、5.1.1-1/2；DESIGN 2.23.1、2.23.2、2.23.5；R36；J1
  - 前置：L23.1（既有 `request_body`/`THINKING_FIELD`）、L29.1（既有字段集用例）
  - 并行：⟂ S1：与 L33.1（`degrade.rs`）、L34.1（`domain`）、L36.1（`run_id.rs` 新增）并行；**不可**与任何 `gateway.rs` 编辑并发
  - 触达：`crates/infrastructure/src/llm/gateway.rs`
  - DoD（`cargo test -p meeting-copilot-infrastructure gateway::` 退出码 0）：
    - 新增 `pub const JSON_INSTRUCTION_MISSING: &str = "json_instruction_missing";`（判定标识，不进入 `ErrorCode`）
    - 新增 `pub const JSON_INSTRUCTION_FRAGMENT: &str`，其值固定含 `json` 字样；**不得**从配置或环境变量读取（无运行期开关）
    - 新增 `pub fn system_prompt(request: &LlmGenerationRequest) -> String`：四类任务共用，**不按 task 分支**；返回内容含 `JSON_INSTRUCTION_FRAGMENT`
    - `request_body` 的 `messages[role=system].content` 改由 `system_prompt(request)` 产出；`messages[role=user].content` 仍为 `request.payload.expose()`
    - 三者共存断言（内联用例）：`body["response_format"]["type"] == "json_object"`、`body["thinking"]["type"] == "disabled"`、`system content` 含 `json`/`JSON`
    - `rg -c "\"json_object\"" crates/infrastructure/src/llm/gateway.rs` 输出 **1**（唯一构造点，不得出现条件移除）
    - `rg -n "JSON_INSTRUCTION_FRAGMENT.*(env|LLM_)" crates/infrastructure/src/llm/gateway.rs` 退出码 1（无运行期开关）
    - `rg -n "\"enabled\"" crates/infrastructure/src/llm/gateway.rs` 退出码 1（`thinking.type` 无第二取值）
    - 内联新增用例 `system_prompt_contains_json_instruction_for_four_tasks`（`LlmTask::ALL` 逐一断言）
    - `cargo clippy -p meeting-copilot-infrastructure --all-targets -- -D warnings` 退出码 0

- [ ] **L32.2 `json_instruction_verdict` 与 `ensure_json_instruction` 发送前拦截（★）**
  - 追溯：AC-38；SPEC 8.6-AC-38、11.11.2-1/2/6、11.11.6-4；DESIGN 2.23.2、2.23.3、2.23.4、2.30.1-D-9；J1
  - 前置：L32.1
  - 并行：否（同写 `gateway.rs`，先于 L33.2）
  - 触达：`crates/infrastructure/src/llm/gateway.rs`
  - DoD：
    - `pub fn json_instruction_verdict(body: &serde_json::Value) -> Option<&'static str>`：仅读 `messages[*]` 中 `role == "system"` 的 `content`；大小写不敏感子串匹配 `json`；命中返回 `None`，否则返回 `Some(JSON_INSTRUCTION_MISSING)`
    - 判定**不认** `role=user`、Schema 字段名、`response_format` 字段名、`model` 取值（J1 更严口径）
    - `pub fn ensure_json_instruction(body: &serde_json::Value) -> Result<(), CopilotError>`：失败返回 `CopilotError::ProviderConfigInvalid { detail: JSON_INSTRUCTION_MISSING }`（复用 `E-PROVIDER-009`）
    - `build_http` 在 `serde_json::to_vec` 与 `HttpRequest::json_post` **之前**调用 `ensure_json_instruction`；失败即返回 `Err`，**零构造、零 socket**
    - 反例构造仅允许 `#[cfg(test)]` 派生入口：由正例 body 删除/移动/改写生成，**不得**另写请求体（D-9）
    - 内联新增用例 `json_instruction_verdict_rejects_derived_negatives`：反例 A（删 system 中 `json` 字样）、B（只放 user 载荷）、C（仅 Schema/`response_format` 含 `json`）、D（删除 `role=system` 消息）四者均返回 `Some(JSON_INSTRUCTION_MISSING)`
    - `cargo test -p meeting-copilot-infrastructure gateway::` 退出码 0
    - `rg -n "ensure_json_instruction" crates/infrastructure/src/llm/gateway.rs` 命中 **≥2**（定义 + `build_http` 调用）
    - 备注〔F4〕：反例入口仅在测试构建存在；生产构建无"关闭指令"路径。

- [ ] **L32.3 采样侧 `json_instruction_missing` 落点（★）**
  - 追溯：AC-38、AC-35；SPEC 8.6-AC-38、11.11.2-6、11.11.4-3；DESIGN 2.23.4；J1、J4
  - 前置：L32.2、L26.3（既有 8 个 `#[ignore]` 采样骨架）
  - 并行：否（同写 `tests/llm_live.rs`，本任务先于 L34.3/L35.1/L36.3）
  - 触达：`crates/infrastructure/tests/llm_live.rs`
  - DoD：
    - 采样前用**与真实发送同源**的 `request_body` 生成 body 并调用 `json_instruction_verdict`；`Some(_)` → 该样本 `excluded = Some("json_instruction_missing")`、`http_status = None`、`total_ms = 0`
    - 该行 `total_ms = 0` **仅在 `excluded` 非空时允许**（不参与耗时分布与 AC-41 判定）；**不得**改写为 1
    - 对应子项 `sub_status = not_measured` + `reason_code = json_instruction_missing`，且**不带任何时延数值**
    - 8 个 `#[ignore]` 用例名不变；不新增第 9 个用例名
    - `rg -n "json_instruction_missing" crates/infrastructure/tests/llm_live.rs` 命中 **≥1**
    - `rg -n "max\(1\)\|unwrap_or\(1\)\|let total_ms = 1" crates/infrastructure/tests/llm_live.rs` 在本任务后仍 **0 命中**（退出码 1）
    - `cargo test --workspace` 的 `passed` 计数不回退（本任务不新增默认用例）

## L33 4xx 分类矩阵纠正与 AC-09 夹具修正（F2 · AC-39）

**任务组目标**：把 4xx 分类收敛到**唯一判定点** `classify_http`：401/402/403 → `Unavailable`（且不重试）、400/404/422 → `InvalidSchema`、仅"400 + 显式结构化 `content_filter`"→ `ContentFiltered`、429 不变；同时把 AC-09 的内容过滤夹具改为结构化标记，判定方式与用例数不变。

- [ ] **L33.1 `degrade.rs` 新增 `ContentPolicyMarker` / `content_policy_marker` / `classify_http`（★）**
  - 追溯：AC-39；SPEC 8.6-AC-39、11.11.3-1~6、11.11.1-3；DESIGN 2.24.2、2.24.3、2.24.4；J2、Q-4
  - 前置：L4.1（既有 `classify`/`classify_status`/`content_filtered`）
  - 并行：⟂ S1：与 L32.1、L34.1、L36.1 并行
  - 触达：`crates/infrastructure/src/llm/degrade.rs`
  - DoD（`cargo test -p meeting-copilot-infrastructure degrade::` 退出码 0）：
    - `pub enum ContentPolicyMarker { None, Explicit }`（封闭两值）
    - `pub fn content_policy_marker(body: &str) -> ContentPolicyMarker`：**只**解析结构化 JSON 字段（如 `error.code`/`error.type`）**精确等于** `content_filter`（或 Q-4 白名单）；正文非 JSON、缺字段、自由文本（"违规/敏感/policy"）一律 `None`
    - `pub fn classify_http(status: u16, marker: ContentPolicyMarker) -> Option<FailureClass>`，优先级固定：
      1. `200..=299` → `None`
      2. `429` → `RateLimited`
      3. `401/402/403` → `Unavailable`（**无论正文**；J2）
      4. `400` 且 `marker == Explicit` → `ContentFiltered`
      5. `400/404/422` 且无标记 → `InvalidSchema`
      6. 其它 4xx → `InvalidSchema`（保守，不得归 `ContentFiltered`）
      7. `300..=399` → `Unavailable`
      8. `500..=599` → `Unavailable`
      9. 其余 → `Unavailable`
    - `classify_status` **保留且语义不变**（4xx 仍返回 `None`）；`content_filtered()` 保留且生产调用点收敛为 `classify_http` 内 **1 处**
    - 内联新增用例 ≥2：`classify_http_priority_is_fixed`（401/402/403、400+标记、400 普通、404/422、404+标记、429、500）、`content_policy_marker_rejects_free_text`
    - `rg -n "contains\(\"content_filter\"\)" crates/infrastructure/src/llm/degrade.rs` 退出码 1（禁止自由文本子串猜测）
    - `cargo clippy -p meeting-copilot-infrastructure --all-targets -- -D warnings` 退出码 0

- [ ] **L33.2 `gateway.rs` 4xx 分支改调 `classify_http` 并禁止重试（★）**
  - 追溯：AC-39；SPEC 8.6-AC-39、11.11.3-7/8；DESIGN 2.24.2、2.24.3-3、2.24.5、2.24.7；R37、J3
  - 前置：L32.2（同文件串行）、L33.1、L34.2（`http_status` 形参；若 L34.2 未完成，先保留 `last_status` 局部变量并在 L34.2 接入）
  - 并行：否（同写 `gateway.rs`）
  - 触达：`crates/infrastructure/src/llm/gateway.rs`
  - DoD：
    - 4xx 分支改为：`let marker = content_policy_marker(&response.body_text()); let class = classify_http(response.status, marker).unwrap_or(FailureClass::Unavailable);`
    - 4xx 分支**直接 `break`**，不进入 `maybe_retry`；`attempt` 不再增长；401/402/403 不得触发 2 次重试（R37）
    - 删除"`(400..500)` 一律 `ContentFiltered`"的兜底：`rg -n "400\\.\\.500" crates/infrastructure/src/llm/gateway.rs` 退出码 1
    - `gateway.rs` 的 4xx 分支**不得**出现 `FailureClass::ContentFiltered` 字面量：`rg -n "FailureClass::ContentFiltered" crates/infrastructure/src/llm/gateway.rs` 退出码 1（分类只由 `classify_http` 产出）
    - 429/3xx/5xx 分支与既有退避/重试上限不变（AC-06/AC-04 断言不改）
    - 一次性与流式两条路径共用 `classify_http`：流式非 2xx 的 `classify_status(status)` 分支保留（5xx/429/3xx），4xx 不再另写判断
    - `cargo build -p meeting-copilot-infrastructure` 退出码 0
    - `cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject` 在 L33.3 完成后退出码 0（AC-04/05/06/07/08 断言不改）

- [ ] **L33.3 AC-09 内容过滤夹具改结构化标记（判定方式不变）（★）**
  - 追溯：AC-09、AC-39；SPEC 8.6-AC-39、11.11.3-9；DESIGN 2.24.6
  - 前置：L33.2
  - 并行：否（同写 `tests/llm_degrade_inject.rs`，先于 L37.2）
  - 触达：`crates/infrastructure/tests/llm_degrade_inject.rs`
  - DoD：
    - `all_failures_no_panic` 的内容过滤注入由纯文本正文改为**结构化错误体**，显式携带 `content_filter` 固定字段与精确值（如 `{"error":{"type":"invalid_request_error","code":"content_filter"}}`）
    - 用例名 `all_failures_no_panic`、六类各注入 **1 次**、断言 `failure == Some(ContentFiltered)`、`catch_unwind` 为 `Ok`、进程退出码 0 —— **全部不变**
    - 用例数量不变（不新增、不删除注入项）
    - `rg -n "\"content_filter\"" crates/infrastructure/tests/llm_degrade_inject.rs` 命中结构化体的字段值（不再出现纯文本单值正文）
    - `cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject all_failures_no_panic` 退出码 0
    - 既有 `timeout_deadline`/`retry_cap`/`rate_limit_backoff`/`invalid_schema_repair`/`circuit_open` 断言零改动（`git diff` 只涉及内容过滤夹具那一段）

## L34 `http_status` 全构造点落盘（F3 · AC-40）

**任务组目标**：让"本次结果对应的最后一次 HTTP 响应状态"从传输层一路带到 `StructuredOutcome` 与 `samples.jsonl`；无响应路径为 `None`，mock 固定 `null`，流式按 D-5 标 `not_measured`。

- [ ] **L34.1 `StructuredOutcome` 新增 `http_status` 并补齐领域内联构造（★）**
  - 追溯：AC-40；SPEC 8.6-AC-40、11.11.4-1、6.2；DESIGN 2.25.1、2.30.1-D-6；J5、Q-3
  - 前置：L1.4（既有 `StructuredOutcome`）
  - 并行：⟂ S1：与 L32.1、L33.1、L36.1 并行
  - 触达：`crates/domain/src/model/llm.rs`
  - DoD：
    - `StructuredOutcome` 新增 `pub http_status: Option<u16>`（**只加字段**；既有字段、`FailureClass` 六值、`DegradeCause`、`ErrorCode::ALL` 零改动）
    - 领域内联 3 处字面量构造补 `http_status: None`；既有断言文字不改
    - 语义注释固定："与本次 outcome 对应的最后一次 HTTP 响应状态；无 HTTP 响应为 `None`；不参与 Schema/六类分布/成功率/审计/日志"
    - `cargo test -p meeting-copilot-domain` 退出码 0
    - `rg -n "http_status" crates/domain/src/model/llm.rs` 命中 **≥4**（字段 + 3 处构造）
    - `rg -n "ErrorCode::.*http" crates/` 退出码 1（不新增错误码）
    - 备注〔F1〕：本任务突破阶段 E"不改 `crates/domain/**`"的声明，属 AC-40 必要最小增量；在 L38.3 显式登记。

- [ ] **L34.2 `degraded_outcome` 增形参与全调用点填充（★）**
  - 追溯：AC-40；SPEC 11.11.4-1、11.11.3-6；DESIGN 2.25.2、2.25.1 不变量；R38、J5
  - 前置：L34.1、L33.2（分类路径已产出 `status`）
  - 并行：否（同写 `gateway.rs`）
  - 触达：`crates/infrastructure/src/llm/gateway.rs`
  - DoD：
    - `degraded_outcome` 增加 `http_status: Option<u16>` 形参；全部调用点（约 8 处）同步
    - 一次性成功（2xx）：`Some(response.status)`
    - 一次性 429 / 4xx / 5xx 降级：`Some(status)`
    - 传输错误 / 离线 / 超时：`None`
    - 本地短路（冷却/排队/载荷超限/审计不可用/预算触顶）：`None`
    - 不变量：`degraded == true` 且 `failure ∈ {InvalidSchema, ContentFiltered, RateLimited, Unavailable}` 的**HTTP 来源**样本必须 `Some`；本地短路与传输错误必须 `None`
    - `cargo build -p meeting-copilot-infrastructure` 退出码 0（新增字段使所有字面量构造编译失败，强制逐点确认）
    - `rg -n "http_status: Some" crates/infrastructure/src/llm/gateway.rs` 命中 **≥3**（成功/429/4xx-5xx 路径）
    - 不把 `http_status` 写入 `LlmAuditRecord` 或 `LlmLogRecord`：`rg -n "http_status" crates/infrastructure/src/llm/mod.rs crates/domain/src/model/llm.rs` 只命中结构体与构造，不出现在审计/日志字段
    - 备注〔F3〕：无 HTTP 响应的样本 `http_status = None`，采样侧必须带 `excluded = no_http_response`（L34.3）。

- [ ] **L34.3 采样侧写入真实状态与流式 D-5 标注（★）**
  - 追溯：AC-40；SPEC 8.6-AC-40、11.11.4-1/6；DESIGN 2.25.4、2.30.1-D-5；Q-1、J5
  - 前置：L34.2、L32.3（同文件串行）
  - 并行：否（同写 `tests/llm_live.rs`）
  - 触达：`crates/infrastructure/tests/llm_live.rs`
  - DoD：
    - 一次性样本：`Sample.http_status = outcome.http_status`（删除写死 `None`）；有 HTTP 响应的样本必须为整数
    - 无响应样本（传输失败/离线）：`http_status = None` 且 `excluded = Some("no_http_response")`；**不进入** AC-40"每行整数"判定分母（D-8）
    - 流式 `Completed`：`http_status = None`，样本 `excluded = Some("stream_status_unavailable")`、子项 `sub_status = not_measured`；**不得**伪造 `Some(200)`（D-5、Q-1）
    - 8 个 `#[ignore]` 用例名不变
    - `rg -n "http_status: None" crates/infrastructure/tests/llm_live.rs` 命中只出现在"无响应/流式"分支，且同行或邻近注释含 `no_http_response`/`stream_status_unavailable`
    - `rg -n "stream_status_unavailable" crates/infrastructure/tests/llm_live.rs` 命中 **≥1**
    - `cargo test -p meeting-copilot-infrastructure --test llm_live -- --ignored --list` 退出码 0 且仍列出 **≥7**
    - 备注〔F5〕：流式状态不可得为已知偏差，登记于 L38.3。

## L35 `total_ms` 实测耗时与亚毫秒口径（F4 · AC-41）

**任务组目标**：`total_ms` 只能来自采样侧 `Instant` 的单调时钟差；禁止 `1 ms` 补齐；亚毫秒保留"0 + 精度不足"标注，不进入分布。

- [ ] **L35.1 `Instant` 计时替换写死 `total_ms`（★）**
  - 追溯：AC-41；SPEC 8.6-AC-41、11.11.4-2/3；DESIGN 2.26.1、2.26.2、2.26.3、2.30.1-D-8；J4、R40
  - 前置：L34.3（同文件串行）
  - 并行：否（同写 `tests/llm_live.rs`）
  - 触达：`crates/infrastructure/tests/llm_live.rs`
  - DoD：
    - 调用 `complete_structured` **之前**取 `let started = std::time::Instant::now();`；返回后立即 `let elapsed = started.elapsed();`
    - `elapsed.as_millis() >= 1` → `total_ms = elapsed.as_millis() as u64`（整数毫秒，J4）
    - `elapsed.as_millis() == 0` → `total_ms = 0`、`excluded = Some("precision_insufficient")`；**禁止**补记为 0/1 以外的值，**禁止** `max(1)`/`unwrap_or(1)`/`+1`
    - 重试口径：整次耗时覆盖首次发出 → 最终返回（含重试与退避）；不把每次尝试拆成新样本
    - 与网关内部 `Clock` 不混用：采样侧只用 `Instant`；不改网关审计 `elapsed_ms`
    - 删除 `let total_ms = 1_u64;`：`rg -n "let total_ms = [0-9]" crates/infrastructure/tests/llm_live.rs` 退出码 1
    - `rg -n "max\(1\)\|unwrap_or\(1\)" crates/infrastructure/tests/llm_live.rs` 退出码 1
    - `rg -n "Instant::now" crates/infrastructure/tests/llm_live.rs` 命中 **≥1**
    - 【F3〕`precision_insufficient` 行不进入 `LiveStats` 的 p50/p95 分母（依赖 L35.2）

- [ ] **L35.2 `live/stats.rs` 口径注释与排除校验（不改字段）**
  - 追溯：AC-41、AC-35；SPEC 8.6-AC-41、11.11.4-3；DESIGN 2.26.3、2.26.5、2.30.1-D-8
  - 前置：L35.1、L10.1（既有 `Sample`/`LiveStats`）
  - 并行：⟂ S2：与 L36.2、L37.1 并行（不同文件）
  - 触达：`crates/infrastructure/src/llm/live/stats.rs`
  - DoD：
    - `Sample` 字段**不变**（`http_status` 与 `total_ms` 已存在）
    - 注释明确：`total_ms` 为整数毫秒；`precision_insufficient` 与 `no_http_response` 样本经 `excluded.is_some()` 排除，不进入耗时分布与成功率分子
    - `valid` 过滤保持 `!degraded && excluded.is_none()`（既有语义零改动）
    - `rg -n "excluded.is_none" crates/infrastructure/src/llm/live/stats.rs` 命中 **≥1**
    - `cargo test -p meeting-copilot-infrastructure live::stats` 退出码 0（既有用例数不减）
    - 不新增统计字段、不改变 nearest-rank 口径（AC-27 重算脚本无需改）

## L36 `run_id` 隔离与记录合并（F5 · AC-42）

**任务组目标**：让"一次运行"成为可唯一标识、目录唯一、样本合并的单元：`run_id` 由毫秒时间戳 + 熵派生（进程号只作熵的一部分），同次运行 8 用例共用一个 `run_id` 与一个目录，`run.json.per_task` 等于 `samples.jsonl` 行数。

- [ ] **L36.1 新增 `live/run_id.rs`（纯函数、零依赖）(★）**
  - 追溯：AC-42；SPEC 8.6-AC-42、11.11.4-4；DESIGN 2.27.1、2.31.1；R39
  - 前置：L9.1（既有 `EnvView`/`live` 模块结构）
  - 并行：⟂ S1：与 L32.1、L33.1、L34.1 并行
  - 触达：`crates/infrastructure/src/llm/live/run_id.rs`（新增 #33）、`crates/infrastructure/src/llm/live/mod.rs`（导出）
  - DoD（`cargo test -p meeting-copilot-infrastructure live::run_id` 退出码 0）：
    - `pub fn new_run_id(at_ms: u128, entropy: u64) -> String`：输出 `{at_ms}-{entropy:016x}`，只含 ASCII 字母数字与 `-`，不含 `/`、`\`、`..`、空格
    - `pub fn runtime_entropy() -> u64`：由 `SystemTime` 纳秒 + 进程号 + 进程内原子计数混合；进程号**不是唯一来源**
    - 不含 host、路径、模型、凭据、`request_id`；不含任何环境变量读取（`rg -n "LLM_" crates/infrastructure/src/llm/live/run_id.rs` 退出码 1）
    - 不新增依赖：`rg -n "^rand" crates/infrastructure/Cargo.toml` 退出码 1；`git diff Cargo.toml Cargo.lock` 为空
    - 内联新增用例 ≥2：`run_id_is_time_and_entropy_derived`（同 `entropy` 不同 `at_ms` → 不同；同 `at_ms` 不同 `entropy` → 不同；同输入 → 同值）、`run_id_format_is_path_safe`（字符集断言）
    - `cargo clippy -p meeting-copilot-infrastructure --all-targets -- -D warnings` 退出码 0

- [ ] **L36.2 `record.rs` 新增 `sample_count()` 并收紧 `per_task` 语义（★）**
  - 追溯：AC-42；SPEC 8.6-AC-42、11.11.4-5；DESIGN 2.27.2、2.27.3、2.30.1-D-7；Q-2
  - 前置：L18.1（`RunRecord` v2）、L10.2（既有 `LiveRecorder`）
  - 并行：⟂ S2：与 L35.2、L37.1 并行（不同文件）
  - 触达：`crates/infrastructure/src/llm/live/record.rs`
  - DoD：
    - 新增 `pub fn sample_count(&self) -> Result<usize, CopilotError>`：只读 `samples.jsonl` 非空行数，不缓存、不改文件
    - `write_run` 前由调用方以累计值填充 `per_task`；`per_task` 语义固定为"本次运行实际写入的样本行数"，配置的样本数保留在 `config` 字符串
    - `RunRecord` 键集、字段顺序、`schema_version = "llm.live.v2"` **零改动**；不新增 `samples_count`
    - `write_run` 的原子替换、路径校验、三态序列化不变；`append_sample` 只追加
    - `rg -n "samples_count" crates/infrastructure/src/llm/live/record.rs` 退出码 1
    - `rg -n "samples_count\|per_task" scripts/llm-live-recompute.mjs` 只命中既有键表，不新增读值逻辑
    - 内联新增用例 `sample_count_reads_appended_lines`（追加 3 行 → 计数 3；再 `write_run` 不改变行数）
    - `cargo test -p meeting-copilot-infrastructure live::record` 退出码 0（既有 ≥6 用例不减）
    - 备注〔F2〕：`per_task` 只改取值、不改键，`schema_version` 不变。

- [ ] **L36.3 `tests/llm_live.rs` 共享 `run_id`、`seq`、中止标志与合并写（★）**
  - 追溯：AC-42、AC-39、AC-40、AC-41；SPEC 11.11.4-4/5、11.11.3-7；DESIGN 2.27.1、2.27.3、2.27.4；J3、R39
  - 前置：L35.1、L36.1、L36.2
  - 并行：否（同写 `tests/llm_live.rs`）
  - 触达：`crates/infrastructure/tests/llm_live.rs`
  - DoD：
    - 进程级 `static RUN_ID: OnceLock<String>`：首次以 `new_run_id(now_ms, runtime_entropy())` 初始化；8 个 `#[ignore]` 用例复用同一值
    - 不再以 `format!("live-{}", std::process::id())` 作为 `run_id`：`rg -n "std::process::id\(\)" crates/infrastructure/tests/llm_live.rs` 退出码 1
    - `seq` 改为运行级 `AtomicU32`，跨用例全局单调递增、不重置：`rg -n "seq: 1" crates/infrastructure/tests/llm_live.rs` 退出码 1
    - 同次运行复用同一 `LiveSession`（`OnceLock<LiveSession>`）或同一 `Arc<BudgetLedger>`/`Arc<LiveRecorder>`：8 用例之间 `calls_used` 不重置、`run_id` 不重置、样本只追加
    - 运行级中止标志 `Arc<AtomicBool>`：首样本状态 ∈ `{401, 402, 403}` → 置位；后续用例在 `admit` 后立即 `return`（不构造传输、不发请求）
    - `run.json` 每次 `write_run` 写累计视图（`per_task` 由 L36.2 取行数）；`samples.jsonl` 行数不因 `write_run` 减少
    - `rg -n "OnceLock" crates/infrastructure/tests/llm_live.rs` 命中 **≥1**；`rg -n "AtomicBool\|AtomicU32" crates/infrastructure/tests/llm_live.rs` 命中 **≥2**
    - 8 个 `#[ignore]` 用例名不变；`cargo test -p meeting-copilot-infrastructure --test llm_live -- --ignored --list` 退出码 0 且 **≥7**
    - `cargo test --workspace` 的 `passed` 计数不回退

- [ ] **L36.4 `llm_live_offline.rs` 的 `per_task` 夹具与行数一致**
  - 追溯：AC-42、AC-27；SPEC 11.11.4-5；DESIGN 2.27.3、2.30.1-D-7
  - 前置：L36.2
  - 并行：否（同写 `tests/llm_live_offline.rs`，先于 L37.4）
  - 触达：`crates/infrastructure/tests/llm_live_offline.rs`
  - DoD：
    - 既有 `per_task: 30` 夹具改为与该用例实际写入的样本行数一致（如 1 或 2）；`config` 字符串仍保留配置样本数
    - 既有 6 个离线用例名与断言零改动（`git diff` 只改夹具取值）
    - `rg -n "per_task: 30" crates/infrastructure/tests/llm_live_offline.rs` 退出码 1
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_offline` 退出码 0，用例数不减
    - 文件内仍零 socket：`rg -c "TcpListener|TcpStream" crates/infrastructure/tests/llm_live_offline.rs` 退出码 1

## L37 测试与夹具补齐（F6 · AC-38~AC-42）

**任务组目标**：把五条 AC 的离线判定落到可重复执行的测试目标；新增默认用例 ≥12；所有断言只增不减，mock 断言只允许"必需片段/字段存在 + 允许新增"的方向放宽。

- [ ] **L37.1 `tests/llm_live_gate.rs` 追加 AC-38 正例/反例（★）**
  - 追溯：AC-38；SPEC 8.6-AC-38；DESIGN 2.28.1、2.23.3
  - 前置：L32.2
  - 并行：⟂ S3：与 L37.2、L37.3、L37.4 并行（不同文件）；与既有 AC-33/34/37 用例同文件，须串行插入
  - 触达：`crates/infrastructure/tests/llm_live_gate.rs`（追加）
  - DoD（零 socket、零真实凭据）：
    - `json_instruction_present_for_four_tasks`：四类任务 ×（一次性、流式）各构造一次；四个 `system` content 均含 `json`/`JSON`；`json_instruction_verdict == None`；`response_format.type == "json_object"` 与 `thinking.type == "disabled"` 同时存在
    - `json_instruction_negative_cases`：四种反例（删 system 中 `json` 字样 / 只放 user 载荷 / 仅 Schema 或 `response_format` 含 `json` / 删除 `role=system`）→ 均 `Some("json_instruction_missing")`；`CountingTransport` 的 `send` 与 `open_stream` 计数均为 **0**
    - 既有 `endpoint_host_locked`/`model_explicitly_confirmed`/`insecure_transport_rejected`/`budget_cap_enforced`/`c1c2c3_fail_closed_matrix`/`thinking_is_fixed_disabled`/`path_confirmed_derived_from_constant`/`allowlist_cannot_widen_hosts` 名称与断言**零改动**
    - 文件内零 socket：`rg -c "TcpListener\|TcpStream" crates/infrastructure/tests/llm_live_gate.rs` 退出码 1
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_gate` 退出码 0，用例数 = 基线 + **≥2**

- [ ] **L37.2 `tests/llm_degrade_inject.rs` 追加 AC-39 矩阵与 AC-40 状态（★）**
  - 追溯：AC-39、AC-40；SPEC 8.6-AC-39/AC-40、11.11.3-2/3/4/5、11.11.4-1；DESIGN 2.28.1、2.24.3、2.25.4
  - 前置：L33.3、L34.2
  - 并行：⟂ S3（不同文件）；与 L33.3 同文件，须串行
  - 触达：`crates/infrastructure/tests/llm_degrade_inject.rs`（追加）
  - DoD：
    - `http_status_classification_matrix`：401、402、403、400（普通 `invalid_request_error`）、400（显式 `content_filter` 结构化标记）、404、422、429 各注入一次 →
      - 401/402/403 → `Unavailable`，`attempts == 1`、`mock 请求数 == 1`（不重试，R37）
      - 400/404/422（无标记）→ `InvalidSchema`，`attempts == 1`
      - 400 + 显式标记 → `ContentFiltered`
      - 404 + 显式标记 → `InvalidSchema`（路径/语义不匹配优先）
      - 429 → `RateLimited`（既有退避不变）
      - 每次 `outcome.http_status == Some(该状态码)`
    - `outcome_carries_http_status`：400/429/500 三例 `outcome.http_status` 分别为 `Some(400)`/`Some(429)`/`Some(500)`；`FakeTransport` 的 DNS/拒绝/超时路径为 `None`
    - 既有 `all_failures_no_panic` 用例数、六类各一次、无 panic 断言零改动
    - `cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject` 退出码 0，用例数 = 基线 + **≥2**

- [ ] **L37.3 `tests/llm_gateway_mock.rs` 追加 JSON 指令与 mock 状态断言（★）**
  - 追溯：AC-38、AC-40、AC-17；SPEC 11.11.2-6、11.11.4-1；DESIGN 2.23.5、2.25.4、2.28.1
  - 前置：L32.2、L34.2
  - 并行：⟂ S3（不同文件）
  - 触达：`crates/infrastructure/tests/llm_gateway_mock.rs`（**只追加**）
  - DoD：
    - `request_body_field_set` 追加断言：`system` content 含 `json`/`JSON`；`response_format.type == "json_object"`；`thinking.type == "disabled"`；断言方式为"必需字段/片段存在 + **允许未知字段/新增文本**"，**不得**做精确字段集相等或精确文本相等
    - 新增 `mock_outcome_http_status_is_none`：`complete_ok` 与 `stream_ok` 的 `outcome.http_status == None`（J5）
    - 既有 `complete_ok`/`stream_ok`/`non_blocking_capture` 名称与断言零改动（`git diff` 只新增行）
    - `cargo test -p meeting-copilot-infrastructure --test llm_gateway_mock` 退出码 0，用例数 = 基线 + **≥1**（既有 ≥3 不减）

- [ ] **L37.4 `tests/llm_live_offline.rs` 追加状态键集、假时钟与目录合并（★）**
  - 追溯：AC-40、AC-41、AC-42；SPEC 8.6-AC-40/AC-41/AC-42、11.11.4-1/2/5；DESIGN 2.25.4、2.26.4、2.27.5
  - 前置：L36.4、L34.2、L35.2
  - 并行：⟂ S3（不同文件）；与 L36.4 同文件，须串行
  - 触达：`crates/infrastructure/tests/llm_live_offline.rs`（**只追加**）
  - DoD（零 socket）：
    - `samples_http_status_key_is_stable`：合成 `Some(200)` 与 `None` 两行样本 → 序列化后键存在，`null` 可被解析；`run.json` 键集不变
    - `total_ms_from_clock_not_constant`：假时钟连续 3 次推进 40/41/42 ms → 记录值分别为 40/41/42 且互不相同；推进 400 µs → `total_ms == 0` + `excluded == Some("precision_insufficient")`，**不出现 1**
    - `single_run_directory_and_merged_count`：同一 `run_id` 追加 8 行、每行后 `write_run` → `.tmp/llm-live` 下该运行**只有 1 个目录**；`run.json.per_task == 8` 且等于 `samples.jsonl` 行数；反例（每用例写 `per_task=1`）→ 判 `run_id_not_unique`
    - 既有 6 个离线用例名与断言零改动（`git diff` 只新增行）
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_offline` 退出码 0，用例数 = 基线 + **≥3**
    - 文件内零 socket：`rg -c "TcpListener|TcpStream" crates/infrastructure/tests/llm_live_offline.rs` 退出码 1

- [ ] **L37.5 用例清单与默认计数核对（≥12）**
  - 追溯：AC-17、AC-38~AC-42；DESIGN 2.28.3
  - 前置：L37.1、L37.2、L37.3、L37.4、L36.1、L33.1
  - 并行：否（汇总）
  - 触达：无源码改动（清单写入 `docs/progress/b1-status.md` 用例段）
  - DoD：
    - 逐条列出并附"命令 + 期望退出码 0"：`json_instruction_present_for_four_tasks`、`json_instruction_negative_cases`、`classify_http_priority_is_fixed`、`content_policy_marker_rejects_free_text`、`http_status_classification_matrix`、`outcome_carries_http_status`、`mock_outcome_http_status_is_none`、`samples_http_status_key_is_stable`、`total_ms_from_clock_not_constant`、`single_run_directory_and_merged_count`、`run_id_is_time_and_entropy_derived`、`run_id_format_is_path_safe`
    - 默认执行新增计数实测：`cargo test --workspace -- --list | grep -c ": test"` **≥250**
    - 逐文件计数与 0.9.4 下限表一致（合计 ≥12）；不得以"新增断言"冒充"新增用例"
    - 不计入默认执行的用例单列：`llm_live.rs` 的 8 个 `#[ignore]`、`llm_transport_tls` 的 ≥6 个 feature 用例
    - 每条用例标注"断言核心"与"对应 AC"，不得出现"以 mock 数值顶替真实"的表述

## L38 门禁与真实重跑（F7 · 环境受限，非输入受限）

> **前置声明（强制）**：
> 1. 本组**不是输入阻塞**：J1~J5 已确认，Q-1~Q-4 有默认口径；阻塞只来自**执行环境**（agent 会话无凭据、沙箱 bash 可能不可用）。
> 2. 真实重跑所需七项环境变量**必须由用户在自己的终端注入**；agent **不得**读取、回显、写入任何注入值。
> 3. 真实重跑只更新三态与 AC-38~AC-42 的真实面结论；**不改** AC-01~AC-37 的任何判定，**不使 AC-20 自动满足**。

- [ ] **L38.1 全量离线门禁执行（★）**
  - 追溯：AC-17、AC-38~AC-42；本清单 0.9.4；SPEC 11.11.6
  - 前置：L32~L37 全部
  - 并行：否（收口串行）
  - 触达：无源码改动（只执行与记录）
  - DoD（逐条把命令与输出摘要粘贴到 `docs/progress/b1-status.md` 证据段）：
    - G-FMT 0；G-CLIPPY 0；G-CLIPPY-F 0；G-LAYER 0；G-SECRET 0
    - G-SECRET-DOC：`bash scripts/scan-secrets-docs.sh --all-md --artifacts` **退出码 0**；**退出码 2 记为失败**（不可判定不得记 0）
    - G-CAND：`bash scripts/check-model-candidates.sh` **退出码 0**（2 亦判失败）
    - G-DENY 0；G-OPENSSL 的 `openssl` 与 `openssl-sys` 计数均为 0；G-PRIVKEY **0 命中**
    - G-TEST：`cargo test --workspace` `failed=0`、`passed ≥ 250`（238 不回退 + ≥12）；`cargo test --workspace -- --list | grep -c ": test"` ≥250
    - G-TEST-TLS：`cargo test -p meeting-copilot-infrastructure --features llm-https` `failed=0`、`passed ≥ 164`（152 + 12；若基线口径不同，按 0.9.4 说明 3 处理）；`llm_transport_tls` 用例数不减
    - G-PLACEHOLDER：`rg -n "max\(1\)\|unwrap_or\(1\)\|let total_ms = [0-9]" crates/infrastructure/tests/llm_live.rs` **0 命中**
    - G-FMT-4XX：`rg -n "400\\.\\.500" crates/infrastructure/src/llm/gateway.rs` **0 命中**；`rg -n "FailureClass::ContentFiltered" crates/infrastructure/src/llm/gateway.rs` **0 命中**
    - 连续 3 次 G-TEST 结果一致（`failed=0` 且 `passed` 计数相同）
    - bash 不可用时：G-SECRET-DOC/G-CAND/G-LAYER 标"未验证（环境受限）"，**不记 0**；cargo 类门禁照常执行

- [ ] **L38.2 真实重跑（用户终端注入；只更新三态）（⛔ 环境受限）**
  - 追溯：AC-38~AC-42 真实面、AC-20（不自动满足）、AC-31；SPEC 8.6 判定环境约束、11.11.4、11.11.5；DESIGN 2.28.2、2.31.4
  - 前置：L38.1；用户终端已注入 `LLM_LIVE`、`LLM_BASE_URL`（含已确认 path）、`LLM_API_KEY`、`LLM_MODEL`、`LLM_MODEL_CONFIRMED`、`LLM_LIVE_CALL_CAP`、`LLM_ALLOWLIST`
  - 并行：否（串行采样，`--test-threads=1`、`concurrency=1`）
  - 触达：无源码改动；产物 `.tmp/llm-live/<run-id>/`
  - DoD：
    - 首选入口：`bash scripts/run-llm-live.sh`；**沙箱 bash 不可用时的替代入口**：`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --ignored --test-threads=1`
    - 环境变量只由用户终端注入；`rg -n "LLM_API_KEY\s*=\|LLM_BASE_URL\s*=" crates scripts` 不得出现赋值（agent 不写入）
    - 冒烟：`LLM_LIVE_SAMPLES=1` → `calls_used == 1`、`budget_hit == false`、`samples.jsonl` ≥1 行（`attempt ≥ 1`、`total_ms ≥ 1` 或显式 `precision_insufficient`）
    - 首次样本 401/402/403 → 中止、`sub_status = not_measured`、不继续消耗配额、分类为 `Unavailable`（J2/J3）
    - AC-40：有 HTTP 响应的真实样本每行 `http_status` 为整数且与同次 `outcome.http_status` 相等；缺失判 `http_status_missing`
    - AC-41：≥2 条有效真实样本的 `total_ms` 不得全部相同；任一固定为 1 → `total_ms_placeholder`；亚毫秒为 0 + `precision_insufficient`，不补记
    - AC-42：连续两次**跨进程**独立运行的 `run_id` 不相同、目录并存、先运行样本不被覆盖；同一次运行内 8 用例只对应 1 个目录，`run.json.per_task == samples.jsonl` 行数
    - 429 未出现 → `not_triggered` + 采样窗口 + 请求数（**不得写"通过"**）
    - 全部样本 `degraded=true` → 相关子项标"未实测（全部降级）"，**不得**标"实测成功"；结论必须附 `degraded` 计数、失败分类分布与 `http_status` 分布（SPEC 11.11.5-2/3）
    - **AC-20 仍不得标"实测"**：本次只更新 AC-38~AC-42 的三态；AC-20 维持"未实测（需真实有效样本/流式读取未增量）"，除非另有独立证据且 AC-36 已判定
    - 环境受限（agent 无凭据）：标"未执行（无凭据/环境受限）"，**不记 0、不记通过**；不得用离线样例顶替真实样本
    - 退出码 0；四泄漏面抓取通过；进程命令行 0 命中

- [ ] **L38.3 阶段 F 状态登记与禁止项审计（不写 `docs/sdd`）**
  - 追溯：AC-17、AC-32、AC-38~AC-42；SPEC 11.11.6、11.11.7；DESIGN 2.31.4
  - 前置：L38.1（L38.2 若未执行，按"未执行（环境受限）"登记）
  - 并行：否
  - 触达：`docs/progress/b1-status.md`（追加段落，不改既有任务行）
  - DoD：
    - 追加"阶段 F（v1.4）"段：AC-38/AC-39/AC-40/AC-41/AC-42 的**离线面**标"离线已判定"；真实面按三态（`measured`/`not_triggered`/`not_measured`）逐项标注，未执行写"未执行（环境受限）"
    - 明确写"**AC-20 不因本次修复自动满足**；AC-40/AC-41 通过只表示状态与耗时如实落盘"
    - 登记受控偏差 〔F1〕domain 新增 `http_status`（D-6、Q-3）、〔F2〕`per_task` 语义收紧（D-7、Q-2）、〔F3〕亚毫秒/无响应口径（D-8）、〔F4〕测试专用反例入口（D-9）、〔F5〕流式 `http_status=None`（D-5、Q-1）
    - 登记 Q-1~Q-4 的"未最终确认 → 默认执行"口径；Q-4 白名单只含 `content_filter`
    - 门禁证据段含 250/164 实测值、G-SECRET-DOC/G-CAND 退出码（2 判失败）、G-PLACEHOLDER/G-FMT-4XX 0 命中
    - 禁止项审计：`git status --short` 无 `.tmp/llm-live` 入库、无新增凭据文件；`git diff --stat docs/sdd` 为空；`rg -n "LLM_LIVE|LLM_API_KEY" .github` 0 命中；`rg -n "LLM_MODEL\s*=|default_model\s*=|deepseek-(chat|reasoner)" crates app/src scripts` 0 命中
    - 交付说明登记"变更外移交项"：`docs/sdd/tasks.md` 勾选由发布责任人手工执行，本清单不执行
    - 登记"不得据此宣称"清单：13.1/13.2 真实端点已验证、13.4 完成、AC-18 已实测、**AC-20 已实测**

---

# 附录 M：AC-38~AC-42 覆盖矩阵（阶段 F）

| AC | 判定命令 / 用例 | 主要任务 | 类别 | 追溯（SPEC / DESIGN） | 备注 |
|---|---|---|---|---|---|
| AC-38 | `--test llm_live_gate json_instruction_present_for_four_tasks`、`json_instruction_negative_cases`；`gateway::system_prompt_contains_json_instruction_for_four_tasks`、`json_instruction_verdict_rejects_derived_negatives`；`--test llm_gateway_mock request_body_field_set` | L32.1、L32.2、L32.3、L37.1、L37.3 | O | SPEC 8.6-AC-38、11.11.2；DESIGN 2.23、2.28.1、D-9 | 四类 × 一次性/流式正例；4 反例 → `json_instruction_missing` 且请求计数 0；`response_format` 与 `thinking` 共存 |
| AC-39 | `--test llm_degrade_inject http_status_classification_matrix`；`degrade::classify_http_priority_is_fixed`、`content_policy_marker_rejects_free_text`；`all_failures_no_panic`（夹具修正） | L33.1、L33.2、L33.3、L37.2 | O（矩阵）/ L（中止） | SPEC 8.6-AC-39、11.11.3-1~9；DESIGN 2.24、2.24.6、R37 | 401/402/403→`Unavailable` 且不重试；400/404/422→`InvalidSchema`；仅 400+显式标记→`ContentFiltered`；429 不变 |
| AC-40 | `--test llm_degrade_inject outcome_carries_http_status`；`--test llm_gateway_mock mock_outcome_http_status_is_none`；`--test llm_live_offline samples_http_status_key_is_stable`；真实逐行比对 | L34.1、L34.2、L34.3、L37.2、L37.3、L37.4 | O（字段/结构）/ L（真实一致） | SPEC 8.6-AC-40、11.11.4-1/6；DESIGN 2.25、D-5、D-6、R38 | 真实有响应样本必须整数；mock/离线 `null`；流式 `None` + `not_measured`；无响应 `None` + `no_http_response` |
| AC-41 | `--test llm_live_offline total_ms_from_clock_not_constant`；G-PLACEHOLDER 静态检查；真实 ≥2 样本 | L35.1、L35.2、L37.4、L38.2 | O（假时钟/静态）/ L（真实） | SPEC 8.6-AC-41、11.11.4-2/3；DESIGN 2.26、D-8、R40 | 整数毫秒；亚毫秒 0 + `precision_insufficient`；禁止 `max(1)`；真实样本不得全部同值 |
| AC-42 | `live::run_id` 内联两例；`--test llm_live_offline single_run_directory_and_merged_count`；两次跨进程真实运行 | L36.1、L36.2、L36.3、L36.4、L37.4、L38.2 | O（机制/模拟）/ L（两次运行） | SPEC 8.6-AC-42、11.11.4-4/5；DESIGN 2.27、D-7、R39 | `run_id` 毫秒+熵（进程号只作熵的一部分）；同次 1 目录；`per_task == 行数` |
| AC-09（夹具回归） | `--test llm_degrade_inject all_failures_no_panic` | L33.3 | O | SPEC 11.11.3-9；DESIGN 2.24.6 | 用途例数、六类各一次、无 panic 不变；仅正文改结构化标记 |
| AC-17（回归） | G-FMT/G-CLIPPY/G-CLIPPY-F/G-TEST(≥250)/G-COUNT/G-LAYER/G-DENY/G-SECRET/G-SECRET-DOC(2=失败)/G-CAND/G-OPENSSL/G-PRIVKEY | L38.1 | O | SPEC 8.1-AC-17；DESIGN 2.28.3 | 238 基线不回退；新增默认用例 ≥12 |

**覆盖结论（阶段 F）**：AC-38、AC-39（矩阵）、AC-40（字段/记录结构）、AC-41（假时钟与静态）、AC-42（`run_id` 与合并模拟）全部可在断网下判定，均有唯一主责任务与可执行命令；AC-39 的真实中止、AC-40 的真实一致性、AC-41 的 ≥2 真实样本、AC-42 的两次独立运行需 L38.2 在用户终端执行。**AC-20 不因本阶段任何修复自动满足**。

# 附录 N：阶段 F 依赖图（简版）

```
L32.1 ─> L32.2 ─> L32.3 ───────────────┐
                                       │
L33.1 ─> L33.2 ─> L33.3 ─> L37.2       │
            │                          │
            └> L34.1 ─> L34.2 ─> L34.3 ─> L35.1 ─> L36.3 ─┐
                                                           │
L36.1 ─> L36.2 ─> L36.4 ─> L37.4                           │
                                       │                   │
L37.1 ─────────────────────────────────┤                   │
L37.3 ─────────────────────────────────┤                   │
                                       └─> L37.5 ─> L38.1 ─> L38.2 ─> L38.3
```

**关键路径（F 版主链，9 跳）**：`L33.1 → L33.2 → L34.1 → L34.2 → L34.3 → L35.1 → L36.3 → L38.1 → L38.2`。
**次长链**：请求体 `L32.1 → L32.2 → L32.3`；运行隔离 `L36.1 → L36.2 → L36.3`；测试 `L37.1/L37.2/L37.3/L37.4 → L37.5`。
**并行簇**：S1 = L32.1/L33.1/L34.1/L36.1；S2 = L32.2/L33.3/L34.2/L36.2；S3 = L37.1/L37.2/L37.3/L37.4。

# 附录 O：阶段 F 可立即执行的首批任务

| 顺序 | 任务 | 动作 | 文件 | 前置 |
|---|---|---|---|---|
| 1 | **L32.1** | `system_prompt` + JSON 指令常量，四类共用 | `crates/infrastructure/src/llm/gateway.rs` | 无（既有 L23.1） |
| 2 | **L33.1** | `ContentPolicyMarker` / `content_policy_marker` / `classify_http` | `crates/infrastructure/src/llm/degrade.rs` | 无（既有 L4.1） |
| 3 | **L34.1** | `StructuredOutcome.http_status` + 内联构造补字段 | `crates/domain/src/model/llm.rs` | 无（既有 L1.4） |
| 4 | **L36.1** | 新增 `run_id.rs`（毫秒 + 熵，零依赖） | `crates/infrastructure/src/llm/live/run_id.rs`（新增 #33）、`live/mod.rs` | 无（既有 L9.1） |

> 四项**互不共享可写文件**，可并行或单人顺序执行。第二批（依赖首批）：L32.2、L33.2、L34.2、L36.2、L37.1、L37.3。
> **开工前置**：Q-1~Q-4 的默认口径无需等待确认（按 0.9.6 执行）；真实面执行前必须由用户终端注入七项变量，agent 不写入任何值。

# 附录 P：阶段 F 收口检查单（复制即用）

```
[ ] G-FMT         cargo fmt --all --check                                          → 0
[ ] G-CLIPPY      cargo clippy --workspace --all-targets -- -D warnings            → 0
[ ] G-CLIPPY-F    cargo clippy -p meeting-copilot-infrastructure --all-targets \
                    --features llm-https -- -D warnings                             → 0
[ ] G-TEST        cargo test --workspace                                           → failed=0, passed ≥ 250
[ ] G-COUNT       cargo test --workspace -- --list | grep -c ": test"              → ≥ 250
[ ] G-TEST-TLS    cargo test -p meeting-copilot-infrastructure --features llm-https → failed=0, passed ≥ 164
[ ] G-LAYER       bash scripts/check-layering.sh                                   → 0
[ ] G-SECRET      bash scripts/scan-secrets.sh                                     → 0
[ ] G-SECRET-DOC  bash scripts/scan-secrets-docs.sh --all-md --artifacts            → 0（2 = 失败）
[ ] G-CAND        bash scripts/check-model-candidates.sh                            → 0（2 = 失败）
[ ] G-DENY        cargo deny check licenses bans sources advisories                → 0
[ ] G-OPENSSL     cargo tree ... --features llm-https -e normal | grep -cE "openssl|openssl-sys" → 0
[ ] G-PRIVKEY     git grep -nE "BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY"             → 0 命中
[ ] G-PLACEHOLDER rg -n "max\(1\)|unwrap_or\(1\)|let total_ms = [0-9]" tests/llm_live.rs → 0 命中
[ ] G-FMT-4XX     rg -n "400\.\.500" src/llm/gateway.rs                            → 0 命中
[ ] 状态保持      git diff --stat docs/sdd                                         → 空
[ ] 零入库        git status --short | grep '.tmp/llm-live'                        → 空
[ ] 无注入值      rg -n "LLM_API_KEY\s*=|LLM_BASE_URL\s*=" crates scripts          → 0 命中
[ ] 不得宣称      AC-20 不因本次修复满足；AC-40/AC-41 通过 ≠ 首 token/整次耗时达标  → 记录在案
```

---

# **阶段 G：Schema 注入与模型输出域收敛（v1.5 增量，L39~L44）**

> **追加声明（强制）**：本阶段**只追加**于文末，不改动 L1~L38 的**任何标题、DoD 文字、勾选状态与编号**；0.1~0.9 与附录 A~P 一字不改。阶段 G 与既有条目的接续/取代关系只在 **0.10.7 与附录 R** 登记。
> **源文档**：`spec.md` **8.7（AC-43~AC-46）**、**11.12（11.12.1~11.12.8）**；`design.md` **v1.5 的 2.32~2.41**（R41~R45、D-10~D-14、K1~K4、Q-5~Q-8）。
> **判定性质**：AC-43~AC-46 属 **补充门槛项续篇**，**不计入** 8.1 本轮完成判据，且不改动 AC-01~AC-42 的任何编号、判定方式与通过条件。任一项不通过时只使相关真实采样结论不得标"实测成功"，不改变 8.1~8.6 的既有结论。
> **零转录（硬约束）**：本阶段正文**不出现**完整端点 URL、host、任何模型名字面量、凭据任何可见片段、Tracking ID 任何片段，也**不出现**审计的编号字段名；只允许出现 Schema 片段名 `properties`/`required`、字段名、路径段常量与既有判定标识（`schema_not_injected`/`prompt_leaked`/`normalization_conflict`/`model_domain_widened`/`request_body_too_large`）。
> **阶段 F 状态（不重开）**：`request_body` 已含 JSON 指令、`thinking.type == disabled`、`response_format.type == json_object`；`classify_http` 4xx 矩阵、`StructuredOutcome.http_status`、`total_ms` 单调时钟、`run_id` 合并写均已落地。真实采样事实：要点抽取 3 次成功（HTTP 200、`degraded=false`、527~929 ms）；摘要、提示卡结论、纪要草稿均 HTTP 200 但**第 2 次尝试后判 `invalid_schema`**。门禁证据为 `cargo test --workspace` **240 passed / 0 failed**，fmt/clippy（含 feature）/层禁/密钥/文档扫描/候选脚本/deny 通过，openssl 0。本阶段以此为**基线**，只增不减。
> **触因（如实记录，不得改写）**：鉴权、模型、端点、网络、TLS 与 JSON 前置均正常；失败点收敛为"模型输出结构与严格 Schema 不一致"——当前 system prompt 仅含任务标识 + Schema 版本号 + JSON 指令，**不含 Schema 本体**，模型无从得知 `purpose`/`version`/`carried_over`/`produced`/`sources`/`unrefined_count` 等服务端元数据。

## 0.10 阶段 G 总览

### 0.10.1 组与主任务映射

| 组 | 主任务 | 主题 | 覆盖 AC | SPEC 小节 | DESIGN 小节 |
|---|---|---|---|---|---|
| **G1** | L39 | Schema 本体注入 + wire 严格解码载体 | AC-43、AC-46（解码侧） | 8.7-AC-43、11.12.3、11.12.5-1/2/3 | 2.32.1~2.32.3、2.33.1~2.33.5、2.34.1、2.34.2、2.36.2 |
| **G2** | L40 | 模型可输出域与网关归一（Summary / Card / Minutes） | AC-44、AC-45、AC-46（闭环侧） | 8.7-AC-44/AC-45、11.12.2、11.12.4、11.12.6 | 2.34.3~2.34.9、2.39.1-D-10/D-11/D-14、K1、K2、K4 |
| **G3** | L41 | 请求体总字节独立预算（拒绝不截断） | AC-43-6 | 11.12.3-6、11.12.8-K3 | 2.35.1~2.35.3、2.39.1-D-12、R41、R43 |
| **G4** | L42 | 严格性不回退与差异集合判定 | AC-46 | 8.7-AC-46、11.12.5、11.12.6-4/5/6 | 2.36.1~2.36.4、2.34.9、R45、D-13、D-14 |
| **G5** | L43 | 测试载体与 mock/真实双路径一致性 | AC-43~AC-46、AC-17 | 8.7 判定环境约束、11.12.6 | 2.37.1~2.37.3、2.34.8、2.40.2-G-6~G-9、R45 |
| **G6** | L44 | 门禁阈值与真实重跑（环境受限） | AC-17、AC-43~AC-46 真实面 | 8.7 判定环境约束 4/5/6、11.12.7 | 2.37.2/2.37.3、2.40.3、2.41 |

### 0.10.2 关键路径与并行矩阵

```
★ 关键路径（阶段 G 主链，12 跳）：
L39.1 → L39.2 → L39.4 → L39.5 → L40.2 → L40.3 → L40.4 → L40.6 → L43.2 → L43.6 → L44.1 → L44.2

★ 次长链（不进入主链，但 L40.6 前必须完成）：
上下文链：L40.1（domain 增量）
闭环链：  L40.5（verify_full）
注入拦截链：L39.3（构造期拦截）
字节预算链：L41.1 → L41.2 → L41.3
严格性链：  L42.1 → L42.2 → L42.4

⟂ 并行簇（以"可写文件互不重叠"为准）：
  S1（无前置，立即可做）：L39.1（gateway.rs）‖ L39.4（wire.rs 新增 + mod.rs 导出 1 行）‖ L40.1（domain llm.rs）
  S2（依赖 S1）：L39.2（gateway.rs）‖ L40.2（normalize.rs 新增，需 wire 类型）‖ L41.1（mod.rs，须串行于 L39.4 的导出行）‖ L42.1（llm_strictness.rs 新增，需 wire 常量）
  S3（依赖 S2）：L39.3（gateway.rs）‖ L39.5（wire.rs + schema.rs 导出口）‖ L40.5（schema.rs）‖ L41.4（llm_live_offline.rs 追加）
  S4（依赖 S3）：L40.3（normalize.rs）‖ L42.2（llm_strictness.rs）‖ L41.2（gateway.rs）
  S5（依赖 S4）：L40.4（normalize.rs）‖ L42.3（llm_strictness.rs）‖ L41.3（llm_live.rs）
  S6（测试文件互不重叠）：L43.1（llm_schema_injection.rs）‖ L43.2（llm_normalize.rs）‖ L43.4（support + llm_gateway_mock）
  S7（同文件串行）：L43.2 → L43.3；L42.1 → L42.2 → L42.3 → L42.4
 串行汇聚点：L40.6（gateway.rs 单点）、L43.5（llm_live.rs）、L43.6、L44.1、L44.2、L44.3
```

**不可并行（同文件须串行）**

- `crates/infrastructure/src/llm/gateway.rs`：L39.1 → L39.2 → L39.3 → L41.2 → L40.6。
- `crates/infrastructure/src/llm/wire.rs`（新增）：L39.4 → L39.5。
- `crates/infrastructure/src/llm/normalize.rs`（新增）：L40.2 → L40.3 → L40.4。
- `crates/infrastructure/src/llm/mod.rs`：L39.4（`pub mod wire;` 一行）→ L40.2（`pub mod normalize;` 一行）→ L41.1（`max_request_bytes`）。
- `crates/infrastructure/tests/llm_normalize.rs`（新增）：L43.2 → L43.3。
- `crates/infrastructure/tests/llm_strictness.rs`（新增）：L42.1 → L42.2 → L42.3 → L42.4。
- `crates/infrastructure/tests/llm_live.rs`：L41.3 → L43.5。

### 0.10.3 阶段 G 文件清单

**新增 5 个文件（编号接阶段 F 的 #33）**

| # | 文件 | 承载职责 | AC | 任务 |
|---|---|---|---|---|
| 34 | `crates/infrastructure/src/llm/wire.rs` | 四类 wire 类型、`ModelOutputDomain` 封闭常量、`decode_wire`（复用未知字段检查） | 44/45/46 | L39.4、L39.5 |
| 35 | `crates/infrastructure/src/llm/normalize.rs` | `normalize` 纯函数、`NormalizationError`、`NormalizedBody`；逐字段显式赋值 | 44/45 | L40.2、L40.3、L40.4 |
| 36 | `crates/infrastructure/tests/llm_schema_injection.rs` | AC-43 正例/反例/六面抓取（O 类） | 43 | L43.1 |
| 37 | `crates/infrastructure/tests/llm_normalize.rs` | AC-44/45 归一与冲突（O 类） | 44/45 | L43.2、L43.3 |
| 38 | `crates/infrastructure/tests/llm_strictness.rs` | AC-46 非法输出、差异集合、静态检查（O 类） | 46 | L42.1~L42.4 |

**修改 10 个文件（G-1~G-10，对齐 DESIGN 2.40.2）**

| # | 文件 | 改动内容 | 不改动内容 | AC | 任务 |
|---|---|---|---|---|---|
| G-1 | `crates/infrastructure/src/llm/gateway.rs` | 抽出 `system_prompt` 并注入完整 Schema；新增 `SCHEMA_BLOCK_SEPARATOR`/`SCHEMA_NOT_INJECTED`/`extract_injected_schema`/`schema_injection_verdict`/`ensure_schema_injected`；`build_http` 追加字节上限；一次性与流式两处改为 `decode_wire → normalize → verify_full`；成功/降级构造点填 `normalization` | SSE 解析、首 token、终止唯一、重试/冷却/截止、4xx 分类、`thinking`/`response_format`、JSON 指令判定不变 | 43/44/45/46 | L39.1~L39.3、L41.2、L40.6 |
| G-2 | `crates/infrastructure/src/llm/schema.rs` | 新增 `verify_full`（序列化后走既有 `decode` 闭环）；导出 `check_unknown` 复用入口 | `decode` 的规则与错误映射不变 | 43/44/46 | L39.5、L40.5 |
| G-3 | `crates/domain/src/model/llm.rs` | 新增 `GenerationContext`/`SummaryContext`/`UnrefinedState`；请求新增 `context` 与 `new_with_context`；新增 `NormalizedField`/`NormalizationSource`/`NormalizationTrace`；结果新增 `normalization`；内联构造补字段 | 既有字段、`FailureClass` 六值、`DegradeCause`、`ErrorCode::ALL`、`CompletionHint` 不变 | 44/45 | L40.1、L40.4 |
| G-4 | `crates/infrastructure/src/llm/mod.rs` | 导出 `wire`/`normalize`；新增 `StructureMode`；`assemble`/`assemble_with_clock` 固定 `LiveModelDomain`；新增 `#[doc(hidden)] assemble_for_fixture`；新增 `LlmConfig.max_request_bytes` 与 `validate` 范围 | 装配不含环境变量读取；既有 `assemble` 签名不变 | 43/44/45 | L39.4、L40.2、L40.6、L41.1 |
| G-5 | `crates/infrastructure/src/llm/fallback.rs` | 仅注释对齐（降级不进入归一、`produced=false`、`notice` 语义） | 兜底逻辑与既有断言不变 | 44/45 | L40.6 |
| G-6 | `crates/infrastructure/tests/support/mod.rs` | `request` 提供 `GenerationContext`（摘要四字段、纪要三字段所需）；`gateway` 改用夹具模式；新增 `live_gateway` 装配 | 既有夹具值不变、不删断言 | 43~46 | L43.4 |
| G-7 | `crates/infrastructure/tests/llm_live.rs` | 8 个用例改用 `new_with_context`；共享骨架追加"注入成立 + 归一生效"断言；字节预算 `excluded` 落点 | 8 个 `#[ignore]` 用例名不变；环境变量仍由用户终端注入 | 43~46 | L41.3、L43.5 |
| G-8 | `crates/infrastructure/tests/llm_gateway_mock.rs` | `request_body_field_set` 追加"system 含完整 Schema"；追加"夹具模式归一不生效"断言 | 既有断言不删、不改变交付值 | 43/44 | L43.4 |
| G-9 | `crates/infrastructure/tests/llm_live_offline.rs` | 追加字节预算用例；追加归一留痕键集/来源枚举用例 | 既有 6 个离线用例与键集断言不改 | 43/44/45 | L41.4、L43.4 |
| G-10 | `docs/progress/b1-status.md` | 阶段 G 状态与三态登记（本阶段仅登记，不改 `docs/sdd`） | `docs/sdd/*.md` 零改动 | 43~46 | L44.3 |

**仅状态登记（不改 `docs/sdd`）**：`docs/progress/b1-status.md`（L44.3 追加阶段 G 段）。

**明确不在本阶段修改**：`docs/sdd/spec.md`、`docs/sdd/design.md`、`docs/sdd/tasks.md`（勾选亦需单独授权）；`crates/application/**`；`crates/domain/src/model/enums.rs`；`deny.toml`；`scripts/llm-live-recompute.mjs`（键集已含所需字段）；`scripts/check-model-candidates.sh`；`ErrorCode::ALL`；环境变量集合（不新增 `LLM_*`）；`platform-*`、`ui-bridge`、`app/src-tauri`、TS 绑定。

### 0.10.4 阶段 G 门禁阈值

| 代号 | 命令 | 通过条件 | 依据 / 与阶段 F 的差异 |
|---|---|---|---|
| G-FMT | `cargo fmt --all --check` | 退出码 0 | 同 |
| G-CLIPPY | `cargo clippy --workspace --all-targets -- -D warnings` | 退出码 0、零 warning | 同 |
| G-CLIPPY-F | `cargo clippy -p meeting-copilot-infrastructure --all-targets --features llm-https -- -D warnings` | 退出码 0、零 warning | 同 |
| G-TEST | `cargo test --workspace` | `failed=0`；**硬约束 `passed ≥ 240`（阶段 F 实测不回退，逐文件基线不减）**；**目标 `passed ≥ 265`** | 见下方"265 的来历与缺口处置" |
| G-COUNT | `cargo test --workspace -- --list \| grep -c ": test"` | ≥ 265（同时 ≥ 240 基线） | 同上 |
| G-TEST-TLS | `cargo test -p meeting-copilot-infrastructure --features llm-https` | `failed=0`；`passed ≥` 阶段 F 实测值 + 15（按阶段 F 门禁文字 164 推算即 **≥179**）；`llm_transport_tls` 用例数不减 | 本阶段新增 O 用例全部落在 infrastructure crate 默认编译面 |
| G-LAYER | `bash scripts/check-layering.sh` | 退出码 0 | 同 |
| G-SECRET | `bash scripts/scan-secrets.sh` | 退出码 0 | 同 |
| G-SECRET-DOC | `bash scripts/scan-secrets-docs.sh --all-md --artifacts` | **仅退出码 0 通过；退出码 2 = 失败（不可判定不得记 0）** | 同 |
| G-CAND | `bash scripts/check-model-candidates.sh` | **退出码 0**（2 亦判失败） | 同 |
| G-DENY | `cargo deny check licenses bans sources advisories` | 退出码 0 | 同；本阶段**零新增依赖** |
| G-OPENSSL | `cargo tree -p meeting-copilot-infrastructure --features llm-https -e normal \| grep -cE "openssl\|openssl-sys"` | 计数 **== 0** | 同 |
| G-PRIVKEY | `git grep -nE "BEGIN (RSA \|EC \|OPENSSH )?PRIVATE KEY"` | **0 命中** | 新增用例不得引入证书/私钥夹具 |
| G-PLACEHOLDER | `rg -n "max\\(1\\)\|unwrap_or\\(1\\)\|let total_ms = [0-9]" crates/infrastructure/tests/llm_live.rs` | **0 命中**（退出码 1 视为通过） | AC-41 不回退 |
| G-FMT-4XX | `rg -n "400\\.\\.500" crates/infrastructure/src/llm/gateway.rs` | **0 命中**（退出码 1 视为通过） | AC-39 不回退 |
| **G-SCHEMA-INJECT**（新） | `cargo test -p meeting-copilot-infrastructure --test llm_schema_injection` + `cargo test -p meeting-copilot-infrastructure --lib -- --exact system_prompt_contains_full_schema_for_four_tasks` | 两条均退出码 0；四类任务 system prompt 的 `properties` 键集合与 `required` 项集合**分别等于** `json_schema(task)`；反例 `send == 0` 且 `open_stream == 0` | AC-43；DESIGN 2.37.1 |
| **G-NORM**（新） | `cargo test -p meeting-copilot-infrastructure --test llm_normalize` + `cargo test -p meeting-copilot-infrastructure --test llm_strictness -- --exact model_domain_difference_set_is_exact` | 两条均退出码 0；归一字段差异集合**恰等于** `purpose`/`version`/`carried_over`/`produced`/`notice`/`sources`/`unrefined_count`/`completion_hint` 八项，出现其它差异即判 `model_domain_widened` | AC-44/45/46；DESIGN 2.36.1 |
| **G-STRICT**（新） | 三条静态检查 + `--test llm_strictness -- --exact strictness_static_checks` | ①`rg -c "deny_unknown_fields" crates/domain/src/model/llm.rs crates/infrastructure/src/llm/wire.rs` 合计 **≥ 基线 + 4**；②`rg -n "#\\[serde\\(default\\)\\]" crates/infrastructure/src/llm/wire.rs` 命中项**只**落在可选字段白名单（`covered`/`conclusion`/`conflict`/`decision`/`todo`）；③`rg -n "unwrap_or_default\|unwrap_or\\(" crates/infrastructure/src/llm/normalize.rs` **0 命中** | AC-46；DESIGN 2.36.3 |

**265 的来历与缺口处置（必须原样保留）**

1. DESIGN 2.37.3 的口径为"阶段 F **门禁文字**基线 250 + 新增 O 用例 ≥15 = **≥265**"。新增 15 例的构成：AC-43 4 + AC-44 4 + AC-45 3 + AC-46 4。
2. 本仓库阶段 F 的**实测值**为 **240 passed**（门禁文字写 250，实测 240）。因此：`240 + 15 = 255 < 265`，要把 265 变成可判定的通过条件，本阶段新增默认用例需 **≥25**；本清单按"DESIGN 具名用例 + 参数化外壳拆分"两种形态都允许，预计新增 ≥18，差额（≥7）由 AC-46 的 16 组非法输出按"非法类别 × 代表任务"拆分为具名用例承担。
3. **缺口处置（强制）**：若实测新增只到 DESIGN 的 ≥15 下限（即 `passed == 255`），必须在 `docs/progress/b1-status.md` 记"**未达 265（新增 15/25）**"并附逐 AC 用例清单与差额原因；**不得**把门禁阈值下调为 255，**不得**删除既有用例、删除既有断言或把"新增断言"计为"新增用例"来凑数（AC-17）。
4. 若阶段 F 的 240 在执行时已随其他分支变化，以**当次执行前实测值**为硬基线，目标值仍固定 **265**，差值口径同上。

**新增默认执行用例下限（用于 265 阈值；逐 AC 计数沿用 DESIGN 2.37.1/2.37.3）**

| 文件 / 位置 | 下限 | 依据（DESIGN 2.37.1） |
|---|---|---|
| `tests/llm_schema_injection.rs` | **+3** | `schema_injected_for_four_tasks`、`schema_injection_negative_cases`、`schema_prompt_not_observed`（AC-43） |
| `tests/llm_gateway_mock.rs` | **+1** | `request_body_field_set` 追加（AC-43）+ 夹具归一不生效（AC-44） |
| `tests/llm_normalize.rs` | **+7** | AC-44 四例（minimal/full/conflicts/trace）+ AC-45 三例（notice/minutes/conflicts） |
| `tests/llm_strictness.rs` | **+4** | `illegal_outputs_rejected_per_task`、`model_domain_difference_set_is_exact`、`strictness_static_checks`、`fixture_mode_not_reachable_from_config`（AC-46） |
| `tests/llm_live_offline.rs` | **+1** | `request_body_byte_budget_is_enforced`（AC-43-6） |
| `src/llm/normalize.rs` / `src/llm/wire.rs` / `src/llm/mod.rs` 内联 | **+2** | 归一纯函数负例、`decode_wire` 缺必填、`max_request_bytes` 边界（三者取 ≥2） |
| **合计** | **≥18**（DESIGN 下限 15） | 240 + 18 = 258；达 265 需按 §3 拆分或增补 |

> 说明 1：`llm_live.rs` 的 8 个 `#[ignore]` 与 `llm_transport_tls` 的 ≥6 个 feature 用例**不计入** 265；后者只在 `--features llm-https` 下计入 G-TEST-TLS。
> 说明 2：G-SCHEMA-INJECT / G-NORM / G-STRICT 是对既有用例的**再断言入口**，不单独计入 265 的分子；其判断失败一律判该 AC 不通过。
> 说明 3：`G-DENY` 为退出码 0 而非计数 0；`G-OPENSSL` 与 `G-PRIVKEY` 为计数/命中 0；`G-SECRET-DOC`/`G-CAND` 的退出码 **2 判失败**，不得记 0。上述五项的"全 0"表述统一按此口径执行。

### 0.10.5 阶段 G 禁止事项（追加于 0.5、0.7.5、0.8.5、0.9.5，同等强制）

1. 不得写完整端点 URL、host、任何模型名字面量、凭据可见片段、Tracking ID 片段与审计编号字段名；扫描命中时只输出`文件:行号`，禁止回显内容。
2. **不得以放宽 Schema 换取通过**：禁止删除/改写 `required`、把模型可输出域实现为"全部可选"、在域内必填字段上用 `serde(default)`/`unwrap_or_default()`、关闭严格解析、只截取 JSON 前缀、改非结构化输出、删除 `response_format`、把四类任务合并为一种宽松结构（SPEC 11.12.5-6）。
3. **不得先补字段再解码**：归一必须发生在严格解码成功之后；解码失败直接进入既有结构违规处理，不产生任何归一字段（SPEC 11.12.4-4）。
4. **不得写成"缺什么补什么"的通用归一**：禁止遍历 `required` 依次填默认值；必须按字段名逐项显式赋值（SPEC 11.12.4-6）。
5. 不得新增失败分类或错误码：`schema_not_injected`/`prompt_leaked`/`normalization_conflict`/`model_domain_widened`/`request_body_too_large` 均为**判定结果标识**，结构违规继续用 `InvalidSchema`、配置类错误复用 `E-PROVIDER-009`。
6. **不得截断**：超字节上限时拒绝构造，不得截断 Schema、不得截断用户载荷、不得删除 `required` 后发送；字节上限不得为"通过用例"而上调（R41、R43）。
7. 不得让提示词原文、Schema 全文、载荷文本进入日志、审计、错误对象、样本记录、报告与进程命令行（`prompt_leaked`）；`NormalizationTrace` 只允许字段名 + 来源枚举 + 两个布尔。
8. 不得新增第二套端口/结果类型/失败分类/错误码：mock 与真实必须共用 `StructuredLlmGateway` 与 `StructuredBody`，差异只允许"校验域"与"归一是否生效"两点（SPEC 11.12.6-3）。
9. 不得让 `FixtureFull` 可由配置、环境变量或请求参数切换；生产装配固定 `LiveModelDomain`（D-11、Q-8）。
10. 不得新增 `LLM_*` 环境变量；不得由 agent 向代码、文件或命令行写入任何注入值（凭据、端点、模型、预算）。
11. 不得回改 L1~L38 的任何条目；阶段 G 的接续/取代关系只在 0.10.7 与附录 R 登记。
12. 不得修改 `docs/sdd/*.md`；阶段 G 的进度登记只写 `docs/progress/b1-status.md` 与 `.tmp/llm-live/` 产物。

### 0.10.6 输入确认状态与执行约束

| 编号 | 事项 | 状态（本阶段生效） | 对阶段 G 的影响 |
|---|---|---|---|
| K1 | 模型越域输出服务端归一字段 | **已确认：一致 → 留痕并以网关值为准；不一致 → 拒绝且不交付** | L40.4 三态处置；L42.4 冲突仅作记录标识 |
| K2 | `Summary.version` 来源 | **已确认：有上一版取上一版 + 1，否则取 1；必须来自请求/上下文** | L40.2 `checked_add(1)`；禁止模型决定、禁止饱和掩盖溢出 |
| K3 | Schema 注入计入 8000 字符上限 | **已确认：不计入用户载荷字符上限；请求体总字节受独立上限约束，超限拒绝不截断** | L41.1/L41.2；8000 字符约束仍由载荷构造期保证 |
| K4 | `completion_hint` 在 0 未细化时的取值 | **已确认：0 → 无；存在待人工复核 → `human_review`；否则 `more_utterances`** | L40.3 优先级固定 |
| Q-5 | 请求体总字节上限默认值 | **已确认：接受 64 KiB（`65_536`）** | L41.1；调整须附真实字节分布证据，不得为通过用例上调 |
| Q-6 | `NormalizationTrace` 进入 `StructuredOutcome` | **已确认：接受进入结果载体（内存）** | L40.1/L40.4；仍不得记录任何文本值 |
| Q-7 | `unrefined_count` 与 `open_questions` 未细化条目数口径 | **已确认：wire 提供时必须相等；未提供时不触发该等式** | L40.3 |
| Q-8 | `assemble_for_fixture` 作为 `#[doc(hidden)]` 测试装配入口 | **已确认：接受** | L40.6/L43.4；既有 mock 夹具值不改、AC-17 结论不变 |

**执行环境约束（强制）**

1. **真实重跑前置**：`LLM_LIVE`、`LLM_BASE_URL`（含已确认 path）、`LLM_API_KEY`、`LLM_MODEL`、`LLM_MODEL_CONFIRMED`、`LLM_LIVE_CALL_CAP`、`LLM_ALLOWLIST` 七项**必须由用户在自己的终端注入**；agent 会话**无凭据**。
2. **agent 只做离线**：agent 只能执行离线门禁与静态断言；**不得**读取、回显、写入任何注入值；不得把凭据放进命令行参数、临时文件或脚本。
3. **环境受限 ≠ 输入受限**：输入侧 K1~K4 与 Q-5~Q-8 均已确认；真实面阻塞只来自执行环境。bash 不可用时，脚本类门禁标"未验证（环境受限）"，**不记 0、不记通过**。
4. **AC-20 仍不因本次修复自动满足**：AC-43~AC-46 通过只表示"Schema 已注入、归一成立、严格性未回退"，**不表示**首 token 或整次耗时达标；AC-20 仍需真实有效样本，且 **AC-36 通过前只能标"未实测（流式读取未增量）"**。
5. **不得伪成功**：重跑后仍有任务 `degraded=true` 时，必须记录失败分类、`http_status` 与尝试次数；四类任务全部 `degraded=false` 之前，不得宣称"真实端点结构化成功"。

### 0.10.7 与既有条目的接续关系（只登记，不重写）

| 既有条目 | 既有内容（保持不变） | 阶段 G 接续关系 | 处理方式 |
|---|---|---|---|
| L32.1/L32.2 | `system_prompt` 与 JSON 指令片段、`json_instruction_verdict`/`ensure_json_instruction` | L39.1 **在返回内容中追加** Schema 本体；`json_instruction_verdict` 的判定范围与语义不变；`build_http` 只多一处前置校验 | 保留勾选状态与 DoD 文字 |
| L29.1 | `request_body_field_set`（必需字段存在 + 允许未知字段） | L43.4 只**追加**"system 含完整 Schema 对象"与"夹具模式归一不生效"断言 | 不删除既有断言 |
| L26.3 | 8 个 `#[ignore]` 接入真实采样链路 | L41.3/L43.5 在**同一文件**补 `new_with_context` 与骨架断言；8 个用例名不变 | 不新增第 9 个用例名 |
| L4.1/L33.1 | `classify`/`classify_http` 与六类分类 | L42.4 只断言"归一失败 → `InvalidSchema`"，**不新增** `FailureClass` 变体 | 不改 L4.1/L33.1 |
| L2.1 | `json_schema`/`decode` 与跨字段规则 | L39.5 复用 `check_unknown`；L40.5 新增 `verify_full` 但**不改** `decode` | 只增函数 |
| L1.4 | `StructuredOutcome` 字段集（含 `http_status`） | L40.1 **只追加** `normalization` 字段与上下文类型；既有字段/枚举/错误码不变 | 登记受控偏差〔G1〕 |
| L30.1 | 阶段 E 门禁（≥238） | L44.1 以 **≥240 硬基线 / ≥265 目标** 接续；238 与 240 均不回退 | 保留 L30.1 文字 |
| L38.1 | 阶段 F 门禁（≥250） | L44.1 以同一套 G-* 代号接续，并新增 G-SCHEMA-INJECT / G-NORM / G-STRICT | 保留 L38.1 文字 |
| L38.2 | 阶段 F 真实重跑（环境受限） | L44.2 以同一"用户终端注入"口径接续；**只更新 AC-38~AC-46 三态** | 保留 L38.2 未勾选原状 |
| DESIGN 2.40.4 的建议编号 L39~L43 | 5 个建议任务 | 本阶段按用户要求拆为 **6 组 L39~L44**：DESIGN 的 L39→本 L39、L40→本 L39.5+L42、L41→本 L40、L42→本 L41、L43→本 L43 | 只登记映射，不改 design |

**受控偏差登记（落地时执行，禁止回改 spec.md / design.md）**

| 编号 | 偏差 | 原因 | 登记位置 |
|---|---|---|---|
| 〔G1〕 | `crates/domain/src/model/llm.rs` 新增 `GenerationContext`/`SummaryContext`/`UnrefinedState`/`NormalizedField`/`NormalizationSource`/`NormalizationTrace` 与请求 `context`、结果 `normalization`（D-10） | 归一来源与可追溯在既有类型中无载体；只新增字段/类型，不改既有字段、枚举、错误码 | L40.1 备注、L44.3 |
| 〔G2〕 | 新增 `LlmConfig.max_request_bytes = 65_536` 并为建议值（D-12、R43、Q-5） | 64 KiB 来自可复核推导而非实测标定；未标定前 fail-closed | L41.1/L41.2 备注、L44.3 |
| 〔G3〕 | `StructureMode::FixtureFull` 与 `#[doc(hidden)] assemble_for_fixture`（D-11、Q-8） | 既有 mock 夹具 `notice` 与 AC-45 归一规则冲突；SPEC 11.12.6-1 允许 mock 走完整校验、归一不生效 | L40.6/L43.4 备注、L44.3 |
| 〔G4〕 | 归一失败不新增失败分类，映射为 `SchemaError{CrossField}` → `InvalidSchema`（D-14） | 保持 `FailureClass` 六值与 `ErrorCode::ALL` 不变（AC-17） | L40.5/L42.4 备注、L44.3 |
| 〔G5〕 | `NormalizationTrace` 进入 `StructuredOutcome`（Q-6、R44） | AC-44/45 要求"字段名与来源可查"；trace 只含枚举与布尔，不含文本值 | L40.1/L40.4 备注、L44.3 |

---

# 阶段 G：Schema 注入与模型输出域收敛（L39~L44）

## L39 Schema 本体注入与 wire 严格解码载体（G1 · AC-43 / AC-46 解码侧）

**任务组目标**：让模型"看得见"该任务的完整 Schema——`system_prompt` 由"版本号 + 措辞"扩展为"JSON 指令 + 完整 Schema 本体"；把注入判定做成**纯函数 + 发送前拦截**，注入不成立时请求计数为 0；同时落下"模型可输出域"的封闭常量与 wire 严格解码，为 L40 的归一提供唯一载体。

- [ ] **L39.1 `system_prompt` 注入完整 Schema 本体（★）**
  - 追溯：AC-43；SPEC 8.7-AC-43、11.12.3-1/2/3/4、11.12.7-7；DESIGN 2.32.1-S-1/S-2/S-4、2.33.1、2.33.4、2.32.3-1/2、R41
  - 前置：L32.1（既有 `system_prompt`、`JSON_INSTRUCTION_FRAGMENT`）
  - 并行：⟂ S1：与 L39.4、L40.1 并行（可写文件互不重叠）
  - 触达：`crates/infrastructure/src/llm/gateway.rs`
  - DoD：
    - 新增 `pub const SCHEMA_BLOCK_SEPARATOR: &str`：模块常量，不含业务语义、不含端点/模型名；用于分隔"JSON 指令片段"与"Schema 本体"
    - `system_prompt(request)` 返回"JSON 指令片段 + `SCHEMA_BLOCK_SEPARATOR` + 该任务 Schema 的**规范序列化**"；Schema 文本唯一来源 `SchemaRegistry::json_schema(request.task)`（`schema.rs:52-77`）
    - `system_prompt` 内**无** `if task`/`match task` 选择模板：只允许 `json_schema(task)` 的注册表选择（DESIGN 2.33.4）
    - 注入块必须是**可独立解析的 JSON 对象**，且只做一次 `serde_json::to_string(&Value)`；**禁止**"转义字符串再二次转义"
    - 与既有固定字段共存：同一份 `request_body` 中 `thinking.type == "disabled"`、`response_format.type == "json_object"`、system 含 `json`/`JSON` **同时存在**
    - 内联新增用例 `system_prompt_contains_full_schema_for_four_tasks`：`LlmTask::ALL` 四类逐一调用 `extract_injected_schema`，`properties` 键集合与 `required` 项集合分别等于 `json_schema(task)`（顺序不要求）
    - `rg -n "system_prompt\(" crates/infrastructure/src` 命中 **2**（定义 1 + `request_body` 调用 1）
    - `rg -c "\"json_object\"" crates/infrastructure/src/llm/gateway.rs` 输出 **1**；`rg -n "\"enabled\"" crates/infrastructure/src/llm/gateway.rs` 退出码 1
    - `rg -n "include_str!\|read_to_string" crates/infrastructure/src/llm/gateway.rs` 退出码 1（无第二份 Schema 来源）
    - `cargo test -p meeting-copilot-infrastructure --lib` 退出码 0；`cargo clippy -p meeting-copilot-infrastructure --all-targets -- -D warnings` 退出码 0

- [ ] **L39.2 `extract_injected_schema` / `schema_injection_verdict` / `ensure_schema_injected`（★）**
  - 追溯：AC-43；SPEC 8.7-AC-43（四反例）、11.12.3-1/3、11.12.5-6；DESIGN 2.33.2、2.33.3、2.33.5、2.32.3-3/4/5
  - 前置：L39.1
  - 并行：否（同写 `gateway.rs`）
  - 触达：`crates/infrastructure/src/llm/gateway.rs`
  - DoD：
    - 新增 `pub const SCHEMA_NOT_INJECTED: &str = "schema_not_injected";`（判定标识，**不进** `ErrorCode`）
    - `pub fn extract_injected_schema(system_content: &str) -> Option<serde_json::Value>`：只返回结构，**不返回**提示词原文
    - `pub fn schema_injection_verdict(body: &serde_json::Value, task: LlmTask) -> Option<&'static str>`：`None` = 通过；`Some(SCHEMA_NOT_INJECTED)` = 该样本无效
    - 五个判定点逐条实现：可解析性 → `properties` 键集合相等 → `required` 项集合相等 → 任务匹配（不得混入其它任务的 `properties`）→ 既有 `json_instruction_verdict` 仍为 `None`
    - **识别边界强制**：只认 `messages[role=system].content` 的注入块；user 中的 Schema 字样、`response_format` 字段、"符合 Schema"措辞、只给版本号**一律不计入**（对应反例①②）
    - `pub fn ensure_schema_injected(body: &serde_json::Value, task: LlmTask) -> Result<(), CopilotError>`：失败返回 `CopilotError::ProviderConfigInvalid { detail: SCHEMA_NOT_INJECTED }`（复用 `E-PROVIDER-009`）
    - 四类反例的 body 只允许由 `#[cfg(test)]` 派生构造入口生成（由正例删/改/混入得到），**不得**另写请求体；生产构建无"关闭注入"路径
    - 内联新增用例 `schema_injection_verdict_rejects_derived_negatives`：①仅版本号 ②仅措辞无 `properties` ③有 `properties` 无 `required` ④混入其它任务 Schema —— 四者均 `Some(SCHEMA_NOT_INJECTED)`
    - `rg -n "schema_not_injected" crates/infrastructure/src` 只命中 `gateway.rs`（常量 + 判定处），不出现于 `degrade.rs`/`mod.rs`
    - `cargo test -p meeting-copilot-infrastructure --lib` 退出码 0

- [ ] **L39.3 `build_http` 构造期拦截与"请求计数为 0"（★）**
  - 追溯：AC-43；SPEC 8.7-AC-43（`send`/`open_stream` 计数为 0）、11.12.3-1、11.12.5-6；DESIGN 2.33.3、2.32.3-7
  - 前置：L39.2
  - 并行：否（同写 `gateway.rs`；须先于 L41.2 的字节上限校验点）
  - 触达：`crates/infrastructure/src/llm/gateway.rs`（`build_http`）
  - DoD：
    - 校验顺序固定为：`ensure_json_instruction`（L32.2）→ `ensure_schema_injected`（L39.2）→ 请求体字节上限（L41.2 接入位；本任务只留调用位与函数名，**不写** `todo!()`/`unimplemented!()`）
    - 校验位置在 `serde_json::to_vec` 与 `HttpRequest::json_post` **之前**；失败即 `Err`，不构造请求、不进入传输层
    - `CountingTransport` 断言：一次性与流式的注入失败路径 `send == 0` 且 `open_stream == 0`
    - 既有 AC-38 拦截不弱化：`rg -n "ensure_json_instruction" crates/infrastructure/src/llm/gateway.rs` 命中 **≥2**
    - `rg -n "todo!\|unimplemented!" crates/infrastructure/src/llm/gateway.rs` 退出码 1
    - `cargo build -p meeting-copilot-infrastructure` 退出码 0；`cargo test -p meeting-copilot-infrastructure --test llm_live_gate` 退出码 0（AC-33/34/37/38 既有断言不变）

- [ ] **L39.4 新增 `llm/wire.rs`：四类 wire 容器 + `ModelOutputDomain`（★，新增 #34）**
  - 追溯：AC-44/45/46；SPEC 11.12.2 表、11.12.5-1/4；DESIGN 2.34.1、2.34.2、2.39.1-D-13、2.40.1-#34
  - 前置：无（只依赖既有 `schema.rs` 与领域类型）
  - 并行：⟂ S1：新建文件；`mod.rs` 的 `pub mod wire;` 一行由本任务追加
  - 触达：`crates/infrastructure/src/llm/wire.rs`（新增 #34）、`crates/infrastructure/src/llm/mod.rs`（仅导出 1 行）
  - DoD：
    - `pub enum WireBody { Insight(...), Summary(...), Card(...), Minutes(...) }`：四类封闭；**不是**第二套结果类型，只是传输校验载体（交付值仍为 `StructuredBody`）
    - `pub struct ModelOutputDomain { pub model_fields: &'static [&'static str], pub model_required: &'static [&'static str], pub normalized_fields: &'static [&'static str], pub normalized_required: &'static [&'static str] }`；`pub fn model_output_domain(task: LlmTask) -> &'static ModelOutputDomain`
    - 逐任务域清单与 DESIGN 2.34.1 表逐字一致：摘要域 `text*`/`covered`、归一 `purpose*`/`version*`/`carried_over*`/`produced*`；结论域 `title*`/`conclusion`/`references*`/`confidence*`/`conflict`、归一 `notice`；纪域 `summary*`/`insights*`/`timeline*`/`open_questions*`、归一 `sources`/`unrefined_count`/`completion_hint`；要点抽取**不裁剪**、归一为空
    - 四类容器各 1 处 `#[serde(deny_unknown_fields)]`（合计 +4，用于 G-STRICT 基线）
    - **域内必填无 `#[serde(default)]`**；域外归一字段一律用 `Option<T>` 表示"模型是否提供"，`Option` **不参与**域内必填判定
    - 集合恒等式 1~4 以代码内常量表达（并集 = `properties`、交集为空、必填并集 = `required`、差异集合 = `normalized_fields`）
    - 归一字段全集恰为 8 项：`purpose`/`version`/`carried_over`/`produced`/`notice`/`sources`/`unrefined_count`/`completion_hint`
    - 零新增依赖：`git diff Cargo.toml Cargo.lock` 为空
    - `cargo build -p meeting-copilot-infrastructure` 退出码 0；`cargo clippy -p meeting-copilot-infrastructure --all-targets -- -D warnings` 退出码 0
    - 备注〔G3〕：本任务只落类型与常量，不断言恒等式（正式断言见 L42.1），不改任何既有交付值。

- [ ] **L39.5 `decode_wire` 严格解码（★）**
  - 追溯：AC-46、AC-44/45；SPEC 11.12.5-1/2/3、11.12.2-2；DESIGN 2.34.2、2.36.2、2.32.1-S-5
  - 前置：L39.4
  - 并行：⟂ S3：与 L39.3、L40.5 并行（不同文件）；`normalize`（L40.2）只依赖本任务的签名
  - 触达：`crates/infrastructure/src/llm/wire.rs`、`crates/infrastructure/src/llm/schema.rs`（仅导出复用入口）
  - DoD：
    - `pub fn decode_wire(task: LlmTask, raw: &str) -> Result<WireBody, SchemaError>`：先 `check_unknown(task, &Value)`（按**完整 Schema 属性集**），再 wire 严格解码
    - 四类错误映射：完整 Schema 未声明字段 → `UnknownField`；类型错 → `TypeMismatch`；值对象越界 → `RangeViolation`；跨字段（结论非空但引用空、待确认条目进分区）→ `CrossField`
    - 域内必填缺失（`text`/`insights`/`title`/`references`/`confidence`/`summary`/`timeline`/`open_questions`）一律解码失败，**不补默认值、不补空串**；缺必填沿用既有 `SchemaError` 规则命名，**不新增规则**
    - 归一字段的 `Option` 存在与否不影响解码通过与否，也不得被当作"域内必填"
    - `schema.rs` 只**新增导出**，`decode` 的规则与错误映射零改动（`git diff` 只新增行）
    - 内联新增用例 `decode_wire_rejects_missing_required_without_default`：摘要响应缺 `text` → 解码失败，产出中**不存在**空 `text`；四类非法输出各 1 组冒烟（正式 16 组见 L42.2）
    - `cargo test -p meeting-copilot-infrastructure --lib` 退出码 0

## L40 模型可输出域与网关归一（G2 · AC-44 / AC-45）

**任务组目标**：把"模型负责内容、网关负责元数据"落成**两套字段集合 + 一个纯函数 + 一次闭环复核**：模型只对 `text`/`covered`、`title`/`conclusion`/`references`/`confidence`/`conflict`、`summary`/`insights`/`timeline`/`open_questions` 负责；`purpose`/`version`/`carried_over`/`produced`/`notice`/`sources`/`unrefined_count`/`completion_hint` 由网关按请求上下文与生成状态**逐字段显式填充**，并在结果载体留痕。

- [ ] **L40.1 domain 最小增量：上下文、归一留痕与 `StructuredOutcome.normalization`（★）**
  - 追溯：AC-44/45；SPEC 11.12.4、6.2；DESIGN 2.34.3、2.39.1-D-10、2.40.2-G-3；Q-6
  - 前置：无（既有 `LlmGenerationRequest`、`StructuredOutcome`）
  - 并行：⟂ S1：`crates/domain/src/model/llm.rs`，与 gateway/wire/normalize 不共享文件
  - 触达：`crates/domain/src/model/llm.rs`
  - DoD：
    - 新增 `GenerationContext { sources: Vec<UtteranceRef>, summary: Option<SummaryContext>, unrefined: UnrefinedState }`、`SummaryContext { version_base: Option<u32>, carried_over: bool }`、`UnrefinedState { count: u32, human_review_pending: bool }`
    - `LlmGenerationRequest` 新增 `context: GenerationContext`；新增 `new_with_context(...)`；既有构造入口保留，其上下文取"空引用集合 / 无上一版 / 0 未细化"（fail-closed 的显式默认，不是给模型输出的补齐）
    - 新增 `NormalizedField`（8 值：`Purpose`/`Version`/`CarriedOver`/`Produced`/`Notice`/`Sources`/`UnrefinedCount`/`CompletionHint`）、`NormalizationSource`（8 值）、`NormalizationTrace { field, source, model_provided, model_value_matched }`
    - `StructuredOutcome` 新增 `pub normalization: Vec<NormalizationTrace>`；语义注释固定："空列表 = 未归一（夹具完整结构或降级兜底）"
    - **只新增**字段/类型：既有字段、`FailureClass` 六值、`DegradeCause`、`ErrorCode::ALL`、`CompletionHint` 零改动
    - `rg -n "ErrorCode::" crates/domain/src/model/llm.rs` 不出现新变体；`cargo test -p meeting-copilot-domain` 退出码 0
    - 全部字面量构造点因编译失败被强制补字段（`cargo build --workspace` 逐个确认）
    - 备注〔G1〕：本任务突破阶段 F 隐含的"不改 `crates/domain/**`"范围，属 D-10 必要最小增量，在 L44.3 登记；**不得**借此改动任何既有字段或枚举。

- [ ] **L40.2 新增 `llm/normalize.rs`：纯函数骨架与 Summary 四字段（AC-44）（★，新增 #35）**
  - 追溯：AC-44；SPEC 8.7-AC-44、11.12.4-1、11.12.8-K2；DESIGN 2.34.3、2.34.4、2.39.2-K2、2.40.1-#35
  - 前置：L39.4、L39.5、L40.1
  - 并行：⟂ S2：新建文件，与 L39.2/L41.1 并行
  - 触达：`crates/infrastructure/src/llm/normalize.rs`（新增 #35）、`crates/infrastructure/src/llm/mod.rs`（导出 1 行）
  - DoD：
    - `pub fn normalize(task: LlmTask, wire: WireBody, request: &LlmGenerationRequest, production: ProductionState) -> Result<NormalizedBody, NormalizationError>`
    - 纯函数：不读环境、不读时钟、不发请求、无全局状态、不写文件
    - `Summary` 四字段：`purpose ← request.purpose`（摘要构造期已保证 `Some`）；`version ← summary.version_base.checked_add(1)`，无上一版 → `1`；`carried_over ← summary.carried_over`；`produced ← production == ModelProduced`
    - 模型提供时逐项比对：不相等 → `PurposeMismatch` / `VersionMismatch`；**不修正模型值、不静默覆盖、不交付**；`checked_add` 溢出 → `VersionOverflow`（**禁止**饱和掩盖）
    - `text`/`covered` **不填充、不改写**（归一前后序列化逐字节相等）
    - `Summary` 缺 `summary` 上下文 → `SourceUnavailable`（不交付、不猜测）
    - 归一失败不新增分类：`NormalizationError` 由 L40.5 的**唯一映射点**转为 `SchemaError{CrossField}`（本任务只定义错误枚举，不改 `FailureClass`）
    - 逐字段显式 `match` 赋值：`rg -n "unwrap_or_default\|unwrap_or\\(" crates/infrastructure/src/llm/normalize.rs` **0 命中**
    - 内联新增用例 `summary_minimal_wire_is_normalized`（仅 `text`/`covered` → `purpose`/`version`/`carried_over`/`produced` 全等于期望且 `report` 4 条）、`summary_version_without_previous_is_one`（无上一版 → 1）、`summary_normalization_conflicts`（①② 冲突 + ③ 缺 `text` 解码失败）
    - `cargo test -p meeting-copilot-infrastructure normalize::` 退出码 0

- [ ] **L40.3 Card `notice` 与 Minutes 三字段（AC-45）（★）**
  - 追溯：AC-45；SPEC 8.7-AC-45、11.12.4-2/3、11.12.8-K4；DESIGN 2.34.5、2.34.6、2.39.2-K4；Q-7
  - 前置：L40.2
  - 并行：否（同写 `normalize.rs`）
  - 触达：`crates/infrastructure/src/llm/normalize.rs`
  - DoD：
    - `Card`：`references` 为空 **且** `conclusion` 为空 → `MissingSource`；其余 → `None`；`notice` **不得**携带任何结论文本（AC-13 不回退）
    - `Minutes`：`sources ← context.sources` **去重且保持首次出现顺序**（同一输入两次归一逐项相等）；`unrefined_count ← context.unrefined.count`
    - `completion_hint` 优先级固定：`human_review_pending` → `HumanReview`；否则 `count > 0` → `MoreUtterances`；否则 `None`（K4：0 → 无）
    - 三反例：载荷外 `sources` → `SourcesOutOfPayload`；wire 提供 `unrefined_count` 且与 `open_questions` 中未细化条目数矛盾 → `UnrefinedCountMismatch`（Q-7：**未提供时不触发**该等式）；`completion_hint` 与计算值不符 → `CompletionHintMismatch`
    - 反例④（结论非空但引用为空）在 `decode_wire` 的跨字段规则即判 `CrossField`，**不进入归一**、不得因归一而放宽
    - `Minutes` 缺 `unrefined` 上下文 → `SourceUnavailable`
    - 内联新增用例 `conclusion_notice_normalization`（`MissingSource` / 模型一致时交付网关值 / 不一致冲突 / 反例④ `CrossField`）、`minutes_service_fields_normalization`（去重 + 顺序稳定 + 三态 `completion_hint`）、`minutes_normalization_conflicts`（①②③）
    - `cargo test -p meeting-copilot-infrastructure normalize::` 退出码 0

- [ ] **L40.4 K1 越域比对与 `NormalizationTrace` 零转录（★）**
  - 追溯：AC-44/45；SPEC 8.7-AC-44/45（可追溯、不含文本值）、11.12.4-5、11.12.5-5；DESIGN 2.34.7、R44；K1、Q-6
  - 前置：L40.2、L40.3
  - 并行：否（同写 `normalize.rs`）
  - 触达：`crates/infrastructure/src/llm/normalize.rs`、`crates/domain/src/model/llm.rs`（仅 `Debug`/`Display` 实现，只输出枚举名）
  - DoD：
    - 三态处置落齐：未提供 → `model_provided = false` 并按来源填充；提供且**一致** → 允许通过、交付**网关值**、`model_provided = true` + `model_value_matched = true`；提供且**不一致** → 冲突、不交付、不采用模型值
    - `NormalizationTrace` 只含 `NormalizedField` + `NormalizationSource` + 两个布尔：**不含**字段值、不含路径拼接文本、不含任何 `.to_string()` 的字段内容
    - 归一不得改写模型内容：域内字段（`text`/`insights`/`title`/`conclusion`/`references`/`summary`/`timeline`/`open_questions`）归一前后序列化逐字节相等
    - 内联新增用例 `normalization_trace_has_no_text`：序列化 trace 后对摘要正文/要点正文/结论正文的样本值 **0 命中**；`report` 的字段名与来源枚举可逐项读出
    - `rg -n "\.to_string\(\)" crates/infrastructure/src/llm/normalize.rs` 只允许出现在"错误信息不含值"的位置，逐条在 `b1-status.md` 说明
    - `cargo test -p meeting-copilot-infrastructure normalize::` 退出码 0

- [ ] **L40.5 `schema.rs` 新增 `verify_full` 闭环复核（★）**
  - 追溯：AC-44/45、AC-01；SPEC 11.12.2-4、11.12.4-4；DESIGN 2.34.3（末行）、2.34.9、2.40.2-G-2；D-14
  - 前置：L40.2、L40.1
  - 并行：⟂ S3：与 L39.3、L39.5 并行（不同文件）
  - 触达：`crates/infrastructure/src/llm/schema.rs`
  - DoD：
    - `pub fn verify_full(task: LlmTask, body: &StructuredBody) -> Result<(), SchemaError>`：序列化后走**既有** `decode`（类型同源闭环，AC-01 不变）
    - `decode` 的规则与错误映射零改动（`git diff` 只新增函数与导出）
    - 归一后仍不合法 → 按结构违规处理（`InvalidSchema`），不得降级为更宽松规则
    - `NormalizationError → SchemaError { rule: CrossField, path: <字段名> }` 的映射落**唯一**位置（本文件），`gateway.rs` 只调用不重复实现
    - 内联新增用例 `verify_full_rejects_unnormalized_minimal_summary`（只含 `text`/`covered` 的归一前 body → 失败）、`verify_full_accepts_normalized_body`（归一后 → 通过）
    - `cargo test -p meeting-copilot-infrastructure schema::` 退出码 0；既有 `llm_schema` 一致性用例零改动

- [ ] **L40.6 gateway 两处插入与 `StructureMode` 装配（★）**
  - 追溯：AC-44/45/46；SPEC 11.12.6-1/2/3/4/5、11.12.4-4；DESIGN 2.34.8、2.34.9、2.39.1-D-11、2.40.2-G-4/G-5；Q-8
  - 前置：L39.3、L40.2、L40.3、L40.4、L40.5
  - 并行：否（`gateway.rs` 单点串行；必须晚于 L39.3 与 L41.2 的校验点）
  - 触达：`crates/infrastructure/src/llm/gateway.rs`、`crates/infrastructure/src/llm/mod.rs`、`crates/infrastructure/src/llm/fallback.rs`（仅注释）
  - DoD：
    - `pub enum StructureMode { LiveModelDomain, FixtureFull }`；`assemble`/`assemble_with_clock` **固定** `LiveModelDomain`；`FixtureFull` 只经 `#[doc(hidden)] assemble_for_fixture` 可达；**不接受**配置字段、环境变量或请求参数切换
    - 一次性与流式两处（既有一次性成功路径与流式拼接后校验处）改为 `decode_wire → normalize → verify_full`；`FixtureFull` 走既有 `decode` 且**归一不生效**（交付值即夹具值）
    - 成功构造点填 `normalization`（`LiveModelDomain` 为 report，`FixtureFull` 为空列表）；降级/兜底构造点填空列表，`produced = false`
    - 失败路径不变：任一步失败 → 既有 `InvalidSchema` → 修复重试 ≤1 → 仍失败则 `fallback`；尝试次数口径与 AC-05/AC-07 一致（不得因归一增加或减少）
    - 生产代码 `FixtureFull` 只 1 处：`rg -n "FixtureFull" crates/infrastructure/src` 命中 **2**（定义 1 + 生产固定值 1）
    - `rg -n "structure_mode\|STRUCTURE_MODE" crates/infrastructure/src` **0 命中**（无运行期开关）
    - `cargo test -p meeting-copilot-infrastructure --test llm_gateway_mock` 退出码 0（`complete_ok`/`stream_ok`/`non_blocking_capture` 断言不变）
    - `cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject` 退出码 0（`invalid_schema_repair`、两次均非法 → 降级，不变）

## L41 请求体总字节独立预算（G3 · AC-43-6）

**任务组目标**：把"Schema 注入不计入 8000 字符载荷上限"与"请求体总字节受独立上限约束、超限拒绝不截断"落成两条**并行不放松**的约束：字符上限由载荷构造期保证，字节上限由 `build_http` 在序列化之后、发送之前判定。

- [ ] **L41.1 `LlmConfig.max_request_bytes` 与范围校验（★）**
  - 追溯：AC-43-6；SPEC 11.12.3-6、11.12.8-K3；DESIGN 2.35.1、2.35.2、2.35.3、2.39.1-D-12；Q-5、R43
  - 前置：L39.4（同写 `mod.rs` 的导出行，须串行）
  - 并行：⟂ S2（`mod.rs`，晚于 L39.4 的 `pub mod wire;` 一行）
  - 触达：`crates/infrastructure/src/llm/mod.rs`
  - DoD：
    - 新增 `pub max_request_bytes: usize`，默认 **`65_536`**（64 KiB）；注释固定"建议值，来源为可复核推导而非实测标定；未标定前 fail-closed"
    - `validate` 限定 `16_384..=1_048_576`；越界 → `ProviderConfigInvalid`（复用 `E-PROVIDER-009`），判定标识不新增 `ErrorCode`
    - 与 `payload_char_limit` **完全独立**：Schema 注入文本**不计入** 8000 字符上限（K3）；8000 字符约束仍由载荷构造期保证
    - 既有 `..LlmConfig::default()` 构造点不受影响；完整字面量构造点因新字段编译失败，逐个确认
    - 内联新增用例 `max_request_bytes_range_is_enforced`：`16_384` 与 `1_048_576` 通过；`16_383` 与 `1_048_577` 拒绝
    - `rg -n "max_request_bytes" crates/infrastructure/src/llm/mod.rs` 命中 **≥3**（字段 + 默认值 + 校验）
    - `cargo test -p meeting-copilot-infrastructure --lib` 退出码 0
    - 备注〔G2〕：64 KiB 为 Q-5 已确认的建议值；调整必须附真实请求体字节分布证据，不得为通过用例上调。

- [ ] **L41.2 `build_http` 总字节校验（拒绝不截断）（★）**
  - 追溯：AC-43-6；SPEC 11.12.3-6、11.12.5-6；DESIGN 2.35.1、2.35.3、R41、R43
  - 前置：L39.3、L41.1
  - 并行：否（同写 `gateway.rs`）
  - 触达：`crates/infrastructure/src/llm/gateway.rs`
  - DoD：
    - 计入口径：`serde_json::to_vec(body)` 的**最终序列化字节**（含 system 提示词、Schema 本体、载荷、`thinking`、`response_format`、`model` 与 JSON 转义膨胀）
    - 校验位置：`to_vec` 之后、`HttpRequest::json_post` **之前**；`bytes.len() > config.max_request_bytes` → `Err(CopilotError::ProviderConfigInvalid { detail: "request_body_too_large" })`
    - **严禁截断**：不截断 Schema、不截断载荷、不删 `required` 后发送；`rg -n "truncate\(\|\[\\.\\.\|\.take\\(" crates/infrastructure/src/llm/gateway.rs` **0 命中**（若命中既有非 Schema 位置，逐条在 `b1-status.md` 说明且不得新增）
    - 失败时 `send`/`open_stream` 计数 **0**；不进入冷却计数（配置类错误不得把端点打成冷却）
    - 内联新增用例 `build_http_rejects_over_budget_body`：上限内可构造；构造"载荷取字符上限 + 完整 Schema 注入"使总字节超限 → 拒绝，且断言序列化字节数在拒绝前后一致（未截断）
    - `cargo test -p meeting-copilot-infrastructure --lib` 退出码 0；`cargo test -p meeting-copilot-infrastructure --test llm_gateway_mock` 退出码 0

- [ ] **L41.3 采样侧 `request_body_too_large` 落点（★）**
  - 追溯：AC-43-6；SPEC 11.12.3-6；DESIGN 2.35.3、2.37.1
  - 前置：L41.2、L26.3（既有 8 个 `#[ignore]` 采样骨架）
  - 并行：否（同写 `tests/llm_live.rs`；须先于 L43.5）
  - 触达：`crates/infrastructure/tests/llm_live.rs`
  - DoD：
    - 采样前用**与真实发送同源**的构造生成 body 并判定字节预算；捕获 `request_body_too_large` → 样本 `excluded = Some("request_body_too_large")`、`http_status = None`、`total_ms = 0`（`total_ms = 0` **仅在 `excluded` 非空时允许**）
    - 子项 `sub_status = not_measured` + `reason_code = request_body_too_large`，**不写**请求体内容或字段长度明细以外的文本
    - 8 个 `#[ignore]` 用例名不变；不新增第 9 个用例名
    - `rg -n "request_body_too_large" crates/infrastructure/tests/llm_live.rs` 命中 **≥1**
    - `rg -n "max\(1\)\|unwrap_or\(1\)" crates/infrastructure/tests/llm_live.rs` 仍 **0 命中**（G-PLACEHOLDER 不回退）
    - `cargo test -p meeting-copilot-infrastructure --test llm_live -- --ignored --list` 退出码 0 且列出 **≥7**

- [ ] **L41.4 `tests/llm_live_offline.rs` 追加字节预算用例**
  - 追溯：AC-43-6；SPEC 11.12.3-6；DESIGN 2.37.1、2.37.2
  - 前置：L41.2
  - 并行：⟂ S3（不同文件）
  - 触达：`crates/infrastructure/tests/llm_live_offline.rs`（**只追加**）
  - DoD：
    - `request_body_byte_budget_is_enforced`：上限内可构造并通过；超限返回 `request_body_too_large`；拒绝路径零 socket；样本 `excluded` 标识落盘且 `http_status = null`
    - 既有 6 个离线用例名与断言零改动（`git diff` 只新增行）
    - 文件内零 socket：`rg -c "TcpListener\|TcpStream" crates/infrastructure/tests/llm_live_offline.rs` 退出码 1
    - `cargo test -p meeting-copilot-infrastructure --test llm_live_offline` 退出码 0，用例数 = 基线 + **≥1**

## L42 严格性不回退与差异集合判定（G4 · AC-46）

**任务组目标**：把"域被悄悄放宽"变成**可计算失败**：差异集合恒等式 1~4 以代码常量与 `json_schema(task)` 的计算结果比对；四类非法输出必须分别落到 `UnknownField`/`TypeMismatch`/`RangeViolation`/`CrossField`；静态检查禁止 `serde(default)` 补齐必填、禁止通用补值、禁止 `FixtureFull` 外泄。

- [ ] **L42.1 差异集合恒等式与 `model_domain_difference_set_is_exact`（★）**
  - 追溯：AC-46；SPEC 8.7-AC-46（通过条件③）、11.12.5-4、11.12.2-1；DESIGN 2.36.1、2.34.1、2.39.1-D-13
  - 前置：L39.4、L39.5
  - 并行：⟂ S2（新建 #38，与 `wire.rs`/`normalize.rs` 不同文件）
  - 触达：`crates/infrastructure/tests/llm_strictness.rs`（新增 #38）
  - DoD：
    - 四类任务各断言恒等式 1~4：`model_fields ∪ normalized_fields == properties(full_schema)`；`model_fields ∩ normalized_fields == ∅`；`model_required ∪ normalized_required == required(full_schema)`；`normalized_fields` 恰为 8 项归一字段
    - 差异集合 `properties(full) − model_fields` **恰等于** `normalized_fields`；出现其它差异 → `model_domain_widened`
    - 断言以常量与 `json_schema(task)` 计算结果比对，**不靠人工阅读**
    - 文件内零 socket：`rg -c "TcpListener\|TcpStream" crates/infrastructure/tests/llm_strictness.rs` 退出码 1
    - `cargo test -p meeting-copilot-infrastructure --test llm_strictness` 退出码 0

- [ ] **L42.2 四类非法输出映射（四任务 × 四类）（★）**
  - 追溯：AC-46；SPEC 8.7-AC-46（通过条件②）、11.12.5-1/2；DESIGN 2.36.2、2.34.9
  - 前置：L42.1、L40.6（须走既有重试/降级路径）
  - 并行：否（同写 `llm_strictness.rs`）
  - 触达：`crates/infrastructure/tests/llm_strictness.rs`
  - DoD：
    - 表驱动 16 组（四任务 × 四类）：未知字段 → `UnknownField`；字符串冒充数字/数组冒充对象 → `TypeMismatch`；置信度越界/正文超长/区间倒置 → `RangeViolation`；结论非空引用空 / 待确认进分区 / 未细化计数与 `open_questions` 矛盾 → `CrossField`
    - 每组断言"未交付"+"进入既有修复重试/降级"：首次 + 1 次修复；仍失败 → `InvalidSchema` + 同类型降级（与 AC-07 一致）
    - 不得新增失败分类/错误码：`rg -n "FailureClass::" crates/infrastructure/tests/llm_strictness.rs` 只出现既有六值
    - 用例 `illegal_outputs_rejected_per_task` 单独可跑：`cargo test -p meeting-copilot-infrastructure --test llm_strictness -- --exact illegal_outputs_rejected_per_task` 退出码 0
    - `cargo test -p meeting-copilot-infrastructure --test llm_strictness` 退出码 0

- [ ] **L42.3 静态检查与 `FixtureFull` 不可切换（G-STRICT 载体）（★）**
  - 追溯：AC-46；SPEC 11.12.5-3/4、11.12.6-3；DESIGN 2.36.3、2.36.4、2.39.1-D-11、R45；Q-8
  - 前置：L39.4、L40.2、L40.6
  - 并行：否（同写 `llm_strictness.rs`）
  - 触达：`crates/infrastructure/tests/llm_strictness.rs`
  - DoD（用例 `strictness_static_checks`，逐条从源码文本断言，失败即该 AC 不通过）：
    1. `deny_unknown_fields` 合计 ≥ 基线 + 4（基线值在 L44.1 首次执行时记录到 `b1-status.md`）
    2. `wire.rs` 的 `#[serde(default)]` 只允许出现在可选字段白名单（`covered`/`conclusion`/`conflict`/`decision`/`todo`）；出现于 `text`/`insights`/`title`/`references`/`confidence`/`summary`/`timeline`/`open_questions` 任一项即判不通过
    3. `normalize.rs` **无** `unwrap_or_default` / `unwrap_or(`
    4. `system_prompt(` 生产引用 1 处（定义 1 + 调用 1）
    5. `FixtureFull` 生产引用 1 处
    6. `schema_not_injected` 只出现于 `gateway.rs`
    7. `gateway.rs` 无 `truncate(`/`[..`/`.take(` 作用于 Schema 或载荷
    - 用例 `fixture_mode_not_reachable_from_config`：构造含任意键的配置 JSON、注入任意环境变量快照、传任意请求参数，`StructureMode` 均为 `LiveModelDomain`；`assemble_for_fixture` 只经 `#[doc(hidden)]` 测试入口可达
    - `cargo test -p meeting-copilot-infrastructure --test llm_strictness -- --exact strictness_static_checks` 退出码 0；`-- --exact fixture_mode_not_reachable_from_config` 退出码 0

- [ ] **L42.4 重试口径不变、禁用项守护与成功率口径（★）**
  - 追溯：AC-46、AC-05/AC-07、AC-22；SPEC 11.12.6-5/6、11.12.5-6；DESIGN 2.34.9、2.36.4、2.37.3；D-14
  - 前置：L42.2
  - 并行：否（同写 `llm_strictness.rs`）
  - 触达：`crates/infrastructure/tests/llm_strictness.rs`、`crates/infrastructure/tests/llm_degrade_inject.rs`（仅追加断言）
  - DoD：
    - 持续非法结构：尝试次数 **恰为 2**（首次 + 1 次修复），与阶段 F 一致；归一**不增减**尝试次数
    - 归一失败 → `InvalidSchema`；`normalization_conflict` 仅作日志/记录标识：`rg -n "normalization_conflict" crates/infrastructure/src` 只命中标识常量与记录处，**不出现**于 `FailureClass`/`ErrorCode`
    - 禁用项守护用例 `forbidden_relaxations_absent`：源码文本中不存在"关闭严格解析 / 只截取 JSON 前缀 / 删除 `response_format` / 四类合并为宽松结构 / 遍历 `required` 补默认值"的实现片段（逐条给出不命中断言）
    - 成功率口径：复核说明中以 `outcome.normalization` 是否为空区分"模型原生通过"与"归一后通过"，**不新增** `samples.jsonl` / `run.json` 键，也不改 `schema_version`；统计脚本 `scripts/llm-live-recompute.mjs` 不改
    - 既有 `invalid_schema_repair`、`retry_cap` 断言零改动（`git diff` 只新增行）
    - `cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject` 退出码 0；`cargo test -p meeting-copilot-infrastructure --test llm_strictness` 退出码 0

## L43 测试载体与 mock/真实双路径一致性（G5 · AC-43~AC-46 / AC-17）

**任务组目标**：把五条 AC 的离线判定落到**可重复执行的测试目标**；新增默认用例 ≥15（DESIGN 2.37.3 下限）；mock 与真实共用同一端口、同一结果类型、同一失败分类，差异只允许"校验域"与"归一是否生效"两点；所有断言只增不减。

- [ ] **L43.1 新增 `tests/llm_schema_injection.rs`（AC-43 三例）（★，新增 #36）**
  - 追溯：AC-43；SPEC 8.7-AC-43（判定方式）、11.12.3-5；DESIGN 2.37.1、2.33.5
  - 前置：L39.3、L41.2
  - 并行：⟂ S6（不同文件）
  - 触达：`crates/infrastructure/tests/llm_schema_injection.rs`（新增 #36）
  - DoD（零 socket、零真实凭据）：
    - `schema_injected_for_four_tasks`：四类任务 ×（一次性、流式）共 8 次构造；每次 `extract_injected_schema` 的 `properties` 键集合与 `required` 项集合**分别等于** `json_schema(task)`；`json_instruction_verdict == None`；`thinking.type == "disabled"` 与 `response_format.type == "json_object"` 同时存在
    - `schema_injection_negative_cases`：四反例（仅版本号 / 仅措辞无 `properties` / 有 `properties` 无 `required` / 混入其它任务）→ 均 `Some(SCHEMA_NOT_INJECTED)`；`CountingTransport` 的 `send == 0` 且 `open_stream == 0`
    - `schema_prompt_not_observed`：六面抓取（日志 / 审计 / 错误对象 `format!("{err:?}")` / 样本记录序列化 / 报告生成串 / `std::env::args()`）对提示词原文与 Schema 全文 **0 命中**；只允许任务标识、Schema 版本号、字符数与判定标识；任一命中判 `prompt_leaked` 且用例失败
    - 文件内零 socket：`rg -c "TcpListener\|TcpStream" crates/infrastructure/tests/llm_schema_injection.rs` 退出码 1
    - `cargo test -p meeting-copilot-infrastructure --test llm_schema_injection` 退出码 0，用例数 **≥3**

- [ ] **L43.2 新增 `tests/llm_normalize.rs`（AC-44 四例）（★，新增 #37）**
  - 追溯：AC-44；SPEC 8.7-AC-44、11.12.6-1；DESIGN 2.37.1、2.34.8
  - 前置：L40.6
  - 并行：⟂ S6（不同文件）
  - 触达：`crates/infrastructure/tests/llm_normalize.rs`（新增 #37）
  - DoD（脚本化 200 响应，可用回环或直接注响应体；不得公网）：
    - `summary_minimal_wire_is_normalized`：响应仅 `text`/`covered` → 成功、`degraded == false`、`attempt == 1`；`purpose == request.purpose`；`version == version_base + 1`（无上一版为 `1`）；`carried_over == context.summary.carried_over`；`produced == true`
    - `summary_full_fixture_is_unchanged`：`FixtureFull` 下完整结构响应的交付值**等于夹具值**，`normalization` 为空；既有 mock 结论不变
    - `summary_normalization_conflicts`：① `purpose` 不一致 → `normalization_conflict`（不修正、不覆盖、不交付）；② `version` 不一致 → 冲突；③ 缺 `text` → 结构违规（不补默认值、不补空串）
    - `normalization_trace_has_no_text`：`outcome.normalization` 含字段名与来源枚举，不含摘要/要点/结论正文样本值
    - `cargo test -p meeting-copilot-infrastructure --test llm_normalize` 退出码 0

- [ ] **L43.3 `tests/llm_normalize.rs` 追加 AC-45 三例（★）**
  - 追溯：AC-45；SPEC 8.7-AC-45、11.12.4-2/3、11.12.8-K4；DESIGN 2.37.1
  - 前置：L43.2（同文件串行）
  - 并行：否（同写 `llm_normalize.rs`）
  - 触达：`crates/infrastructure/tests/llm_normalize.rs`
  - DoD：
    - `conclusion_notice_normalization`：引用空且结论空 → `notice == MissingSource`；模型提供一致 `notice` → 交付网关值；不一致 → `normalization_conflict`；结论非空且引用空 → `CrossField`（结构违规，**不**因归一放宽）
    - `minutes_service_fields_normalization`：`sources` 取自请求载荷引用集合、去重、顺序稳定（两次调用逐项相等）；`unrefined_count == context.unrefined.count`；`completion_hint` 三态：有未细化 → `MoreUtterances`、为 0 → 无、有待复核 → `HumanReview`
    - `minutes_normalization_conflicts`：①载荷外 `sources` → `SourcesOutOfPayload`；②未细化计数与 `open_questions` 矛盾 → 冲突；③`completion_hint` 不符 → 冲突；④结论非空引用空 → 结构违规
    - `cargo test -p meeting-copilot-infrastructure --test llm_normalize` 退出码 0，用例数 **≥7**

- [ ] **L43.4 测试装配（`support`）与 mock 断言只增不减（★）**
  - 追溯：AC-43/44/45/17；SPEC 11.12.6-1/3/4；DESIGN 2.40.2-G-6/G-8、2.34.8、R45；Q-8
  - 前置：L39.3、L40.6
  - 并行：⟂ S6（`support/mod.rs` 与 `llm_gateway_mock.rs`；两者须串行于自身文件）
  - 触达：`crates/infrastructure/tests/support/mod.rs`、`crates/infrastructure/tests/llm_gateway_mock.rs`（**只追加**）
  - DoD：
    - `support::request(...)` 提供 `GenerationContext`：摘要四字段所需（`version_base`、`carried_over`）与纪要三字段所需（`sources`、`unrefined.count`、`human_review_pending`）；既有夹具值不变
    - `support::gateway(...)` 改用夹具模式（`assemble_for_fixture`）；新增 `support::live_gateway(...)` 走 `assemble`（`LiveModelDomain`）；两条装配**共用**同一 `StructuredLlmGateway` 与 `StructuredBody`
    - `llm_gateway_mock.rs` 的 `request_body_field_set` 追加断言：system 含**完整 Schema 对象**（`properties`/`required` 存在）；断言方式为"必需片段/字段存在 + **允许新增文本/未知字段**"，**不得**做精确字段集或精确文本相等
    - 新增 `mock_fixture_mode_does_not_normalize`：夹具模式下 `outcome.normalization` 为空且交付值等于夹具值（AC-44 第 1 条语义）
    - 既有 `complete_ok`/`stream_ok`/`non_blocking_capture` 名称与断言零改动（`git diff` 只新增行）
    - `cargo test -p meeting-copilot-infrastructure --test llm_gateway_mock` 退出码 0，用例数 = 基线 + **≥1**

- [ ] **L43.5 `tests/llm_live.rs` 8 用例改 `new_with_context` 与共享骨架断言（★）**
  - 追溯：AC-43~AC-46 真实面、AC-35/AC-42；SPEC 8.7 判定环境约束 4/5、11.12.6-6；DESIGN 2.37.1（末三行）、2.37.3、2.40.2-G-7
  - 前置：L41.3、L40.6
  - 并行：否（同写 `tests/llm_live.rs`）
  - 触达：`crates/infrastructure/tests/llm_live.rs`
  - DoD：
    - 8 个 `#[ignore]` 用例全部改用 `new_with_context`，上下文由用例夹具提供（摘要：`version_base`/`carried_over`；纪要：`sources`/`unrefined`）；**不**从响应体推断任何归一值
    - 共享骨架追加断言（不依赖真实响应即可成立的部分用离线构造先验）：system 含完整 Schema（以判定函数返回 `None` 判定）、`thinking`/`response_format` 与 JSON 指令共存、`normalization` 仅在真实成功路径非空
    - 样本记录仍只写标识：注入判定失败 → `excluded = Some(SCHEMA_NOT_INJECTED)`、`http_status = None`、`total_ms = 0`；归一冲突 → `excluded = Some("normalization_conflict")` 且记录失败分类与 `http_status`
    - 8 个 `#[ignore]` 用例名不变；不新增第 9 个用例名
    - `rg -n "new_with_context" crates/infrastructure/tests/llm_live.rs` 命中 **≥8**
    - `rg -n "schema_not_injected\|normalization_conflict" crates/infrastructure/tests/llm_live.rs` 命中 **≥2**（仅作标识，不写内容）
    - `cargo test -p meeting-copilot-infrastructure --test llm_live -- --ignored --list` 退出码 0 且 **≥7**

- [ ] **L43.6 用例清单与默认计数核对（≥15 新增 / 目标 265）**
  - 追溯：AC-17、AC-43~AC-46；DESIGN 2.37.3；本清单 0.10.4
  - 前置：L43.1~L43.5、L42.3
  - 并行：否（汇总）
  - 触达：无源码改动（清单写入 `docs/progress/b1-status.md` 用例段）
  - DoD：
    - 逐条列出并附"命令 + 期望退出码 0"：`schema_injected_for_four_tasks`、`schema_injection_negative_cases`、`schema_prompt_not_observed`、`summary_minimal_wire_is_normalized`、`summary_full_fixture_is_unchanged`、`summary_normalization_conflicts`、`normalization_trace_has_no_text`、`conclusion_notice_normalization`、`minutes_service_fields_normalization`、`minutes_normalization_conflicts`、`illegal_outputs_rejected_per_task`、`model_domain_difference_set_is_exact`、`strictness_static_checks`、`fixture_mode_not_reachable_from_config`、`request_body_byte_budget_is_enforced`、`mock_fixture_mode_does_not_normalize`
    - 默认执行新增计数实测：`cargo test --workspace -- --list \| grep -c ": test"` **≥265**（并同时 ≥240 基线）；若未达 265，按 0.10.4 §3 登记缺口，**不得**下调阈值
    - 逐文件计数与 0.10.4 下限表一致（合计 ≥15，实测 ≥18）；不得以"新增断言"冒充"新增用例"
    - 不计入默认执行的用例单列：`llm_live.rs` 的 8 个 `#[ignore]`、`llm_transport_tls` 的 ≥6 个 feature 用例
    - 每条用例标注"断言核心"与"对应 AC"；不得出现"以 mock 数值顶替真实"的表述

## L44 门禁与真实重跑（G6 · 环境受限，非输入受限）

> **前置声明（强制）**：
> 1. 本组**不是输入阻塞**：K1~K4 与 Q-5~Q-8 均已确认，阻塞只来自**执行环境**（agent 会话无凭据、沙箱 bash 可能不可用）。
> 2. 真实重跑所需七项环境变量**必须由用户在自己的终端注入**；agent **不得**读取、回显、写入任何注入值。
> 3. 真实重跑**只更新** AC-38~AC-46 的三态与真实面结论；**不改** AC-01~AC-37 的任何判定，**不使 AC-20 自动满足**。

- [ ] **L44.1 全量离线门禁执行（★）**
  - 追溯：AC-17、AC-43~AC-46；本清单 0.10.4；SPEC 8.7 判定环境约束 1/3、11.12.7
  - 前置：L39~L43 全部
  - 并行：否（收口串行）
  - 触达：无源码改动（只执行与记录）
  - DoD（逐条把命令与输出摘要粘贴到 `docs/progress/b1-status.md` 证据段）：
    - G-FMT 0；G-CLIPPY 0；G-CLIPPY-F 0；G-LAYER 0；G-SECRET 0；G-DENY 0
    - G-SECRET-DOC：`bash scripts/scan-secrets-docs.sh --all-md --artifacts` **退出码 0**；**退出码 2 记为失败**
    - G-CAND：`bash scripts/check-model-candidates.sh` **退出码 0**（2 亦判失败）
    - G-OPENSSL：`openssl` 与 `openssl-sys` 计数均为 **0**；G-PRIVKEY：**0 命中**
    - G-TEST：`cargo test --workspace` `failed=0`、`passed ≥ 240`（阶段 F 实测不回退）且**目标 ≥265**；`cargo test --workspace -- --list \| grep -c ": test"` ≥265
    - G-TEST-TLS：`cargo test -p meeting-copilot-infrastructure --features llm-https` `failed=0`、`passed ≥` 阶段 F 实测值 + 15（按 164 推算即 ≥179）；`llm_transport_tls` 用例数不减
    - G-SCHEMA-INJECT 0；G-NORM 0；G-STRICT 0（三条静态检查按 0.10.4 逐条给出计数/命中）
    - G-PLACEHOLDER **0 命中**；G-FMT-4XX **0 命中**
    - 连续 3 次 G-TEST 结果一致（`failed=0` 且 `passed` 计数相同）
    - bash 不可用时：G-SECRET-DOC/G-CAND/G-LAYER 标"未验证（环境受限）"，**不记 0**；cargo 类门禁照常执行

- [ ] **L44.2 真实重跑（用户终端注入；只更新三态）（⛔ 环境受限）**
  - 追溯：AC-43~AC-46 真实面、AC-20（不自动满足）、AC-36；SPEC 8.7 判定环境约束 2/4/5/6、11.12.7-2/3；DESIGN 2.37.2、2.41
  - 前置：L44.1；用户终端已注入 `LLM_LIVE`、`LLM_BASE_URL`（含已确认 path）、`LLM_API_KEY`、`LLM_MODEL`、`LLM_MODEL_CONFIRMED`、`LLM_LIVE_CALL_CAP`、`LLM_ALLOWLIST`
  - 并行：否（串行采样，`--test-threads=1`、`concurrency=1`）
  - 触达：无源码改动；产物 `.tmp/llm-live/<run-id>/`
  - DoD：
    - 首选入口：`bash scripts/run-llm-live.sh`；**沙箱 bash 不可用时的替代入口**：`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --ignored --test-threads=1`
    - 环境变量只由用户终端注入；agent 不写入任何赋值（`rg -n "LLM_API_KEY\s*=\|LLM_BASE_URL\s*=" crates scripts` 不得出现赋值）
    - **AC-43 真实面**：每类任务的样本记录可判"注入成立"（只记判定标识与字符数，样本内**不含**提示词原文与 Schema 全文）；注入不成立 → `excluded = Some(SCHEMA_NOT_INJECTED)`、不发出请求
    - **AC-44/AC-45 真实面**：最小结构响应成功且 `degraded == false` 时才可标"实测"；否则记录失败分类、`http_status` 与尝试次数；`normalization` 字段名与来源枚举可查，且不含任何文本字段值
    - **AC-46 真实面**：真实重跑中非法输出仍走结构违规路径（`InvalidSchema` → 修复重试 ≤1 → 降级）；**不得**以放宽 Schema 让"四类全部 200 且 `degraded=false`"虚报通过
    - 若仍有任务 `degraded=true`：该任务相关子项标"未实测（降级）"，并附 `degraded` 计数、失败分类分布与 `http_status` 分布；四类全部 `degraded=false` 之前不得宣称"真实端点结构化成功"
    - **AC-20 仍不得标"实测"**：本次只更新 AC-38~AC-46 三态；AC-20 维持"未实测（需真实有效样本）"，且 **AC-36 通过前只能标"未实测（流式读取未增量）"**
    - 环境受限（agent 无凭据）：标"未执行（无凭据/环境受限）"，**不记 0、不记通过**；不得用离线样例顶替真实样本
    - 退出码 0；四泄漏面抓取通过；进程命令行 0 命中

- [ ] **L44.3 阶段 G 状态登记与禁止项审计（不写 `docs/sdd`）**
  - 追溯：AC-17、AC-32、AC-43~AC-46；SPEC 11.12.7；DESIGN 2.40.3、2.41
  - 前置：L44.1（L44.2 若未执行，按"未执行（环境受限）"登记）
  - 并行：否
  - 触达：`docs/progress/b1-status.md`（追加段落，不改既有任务行）
  - DoD：
    - 追加"阶段 G（v1.5）"段：AC-43/AC-44/AC-45/AC-46 的**离线面**标"离线已判定"；真实面按三态（`measured`/`not_triggered`/`not_measured`）逐项标注，未执行写"未执行（环境受限）"
    - 明确写"**AC-20 不因本次修复自动满足**；AC-43~AC-46 通过只表示注入成立、归一成立、严格性未回退"与"AC-36 通过前流式首 token 只能标未实测"
    - 登记受控偏差：〔G1〕domain 增量（D-10、Q-6）、〔G2〕字节上限 64 KiB 建议值（D-12、R43、Q-5）、〔G3〕`FixtureFull` 测试装配入口（D-11、Q-8）、〔G4〕归一失败不新增分类（D-14）、〔G5〕trace 进 `StructuredOutcome`（Q-6、R44）
    - 门禁证据段含 240/265/179 实测值、G-SCHEMA-INJECT / G-NORM / G-STRICT 的计数与命中、G-SECRET-DOC/G-CAND 退出码（2 判失败）、G-PLACEHOLDER/G-FMT-4XX 0 命中
    - 禁止项审计：`git status --short` 无 `.tmp/llm-live` 入库、无新增凭据文件；`git diff --stat docs/sdd` 为空；`rg -n "LLM_LIVE\|LLM_API_KEY" .github` 0 命中；端点 URL / 模型名 / 凭据 / 编号字段名在新增代码与文档中 0 命中
    - 交付说明登记"变更外移交项"：`docs/sdd/tasks.md` 勾选由发布责任人手工执行，本清单不执行
    - 登记"不得据此宣称"清单：13.1/13.2 真实端点已验证、13.4 完成、AC-18 已实测、**AC-20 已实测**、"四类全部 200 且 `degraded=false`"

---

# 附录 Q：AC-43~AC-46 覆盖矩阵（阶段 G）

| AC | 判定命令 / 用例 | 主要任务 | 类别 | 追溯（SPEC / DESIGN） | 备注 |
|---|---|---|---|---|---|
| AC-43 | `--test llm_schema_injection schema_injected_for_four_tasks`、`schema_injection_negative_cases`、`schema_prompt_not_observed`；`--lib system_prompt_contains_full_schema_for_four_tasks`、`schema_injection_verdict_rejects_derived_negatives`；`--test llm_live_offline request_body_byte_budget_is_enforced` | L39.1、L39.2、L39.3、L39.4、L41.1、L41.2、L41.4、L43.1、L43.4 | O（注入/反例/六面/字节上限）/ L（真实注入成立） | SPEC 8.7-AC-43、11.12.3；DESIGN 2.33、2.35、2.37.1 | 四类 × 一次性/流式 8 次；`properties`/`required` 集合相等；四反例 `send == 0` 且 `open_stream == 0`；提示词与 Schema 全文六面 0 命中 |
| AC-44 | `--test llm_normalize summary_minimal_wire_is_normalized`、`summary_full_fixture_is_unchanged`、`summary_normalization_conflicts`、`normalization_trace_has_no_text` | L40.1、L40.2、L40.4、L40.5、L40.6、L43.2、L43.4 | O（归一/冲突/留痕）/ L（真实最小结构） | SPEC 8.7-AC-44、11.12.2、11.12.4-1；DESIGN 2.34.3~2.34.4、K2 | 仅 `text`/`covered` → `degraded=false`、`attempt=1`；`version` 有上一版 +1 否则 1；①② 冲突、③ 缺 `text` 结构违规 |
| AC-45 | `--test llm_normalize conclusion_notice_normalization`、`minutes_service_fields_normalization`、`minutes_normalization_conflicts` | L40.3、L40.4、L40.6、L43.3 | O（notice/三字段/冲突）/ L（真实最小结构） | SPEC 8.7-AC-45、11.12.4-2/3、6.5、6.6；DESIGN 2.34.5~2.34.6、K1、K4 | `notice` 不得承载结论文本；`sources` 去重且顺序稳定；`completion_hint` 三态固定；反例④ 仍 `CrossField` |
| AC-46 | `--test llm_strictness illegal_outputs_rejected_per_task`、`model_domain_difference_set_is_exact`、`strictness_static_checks`、`fixture_mode_not_reachable_from_config`、`forbidden_relaxations_absent`；G-STRICT 三条静态检查 | L39.4、L39.5、L40.5、L42.1、L42.2、L42.3、L42.4 | O（非法输出/差异集合/静态）/ L（真实非法输出） | SPEC 8.7-AC-46、11.12.5；DESIGN 2.36、2.34.9、D-13、D-14 | 四任务 × 四类 = 16 组；差异集合恰为 8 个归一字段；`deny_unknown_fields` 不减；域内必填无 `serde(default)`；尝试次数仍为 2 |
| AC-17（回归） | G-FMT / G-CLIPPY / G-CLIPPY-F / G-TEST（≥240 硬基线、≥265 目标）/ G-COUNT / G-TEST-TLS / G-LAYER / G-DENY / G-SECRET / G-SECRET-DOC（2=失败）/ G-CAND（2=失败）/ G-OPENSSL / G-PRIVKEY / G-SCHEMA-INJECT / G-NORM / G-STRICT | L44.1 | O | SPEC 8.1-AC-17、11.12.7-1；DESIGN 2.37.3 | 240 基线不回退；新增默认用例 ≥15（DESIGN 下限）；mock 断言只增不减 |
| AC-20（不变） | 真实有效样本；本轮**不判定** | L44.2（只登记三态） | L | SPEC 8.7 判定环境约束 4、11.12.7-2 | **不因本次修复自动满足**；AC-36 通过前只能标"未实测（流式读取未增量）" |
| AC-38~AC-42（回归） | 阶段 F 既有用例与门禁代号 | L44.1（同套门禁） | O / L | SPEC 11.12.7-3 | 只更新三态，不改既有结论 |

**覆盖结论（阶段 G）**：AC-43 的注入/反例/六面/字节上限、AC-44/45 的归一与冲突、AC-46 的非法输出与差异集合**全部可在断网下判定**，均有唯一主责任务与可执行命令；AC-43~AC-46 的**真实面**需 L44.2 在用户终端注入环境变量后执行。**AC-20 不因本阶段任何修复自动满足**；四类任务全部 `degraded=false` 之前不得宣称"真实端点结构化成功"。

# 附录 R：阶段 G 依赖图（简版）

```
L39.1 ─> L39.2 ─> L39.3 ──────────────────────────────┐
L39.4 ─> L39.5 ─┐                                     │
                ├─> L40.2 ─> L40.3 ─> L40.4 ─> L40.6 ─> L43.2 ─> L43.3 ─┐
L40.1 ──────────┘                          ▲                            │
L40.5 ─────────────────────────────────────┘                            │
L41.1 ─> L41.2 ─> L41.3 ─────────────────────────────────────────────────┤
              └─> L41.4 ──────────────────────────────────────────────────┤
L39.4 ─> L42.1 ─> L42.2 ─> L42.4 ──────────────────────────────────────────┤
                └─> L42.3 ──────────────────────────────────────────────────┤
L43.1 ─────────────────────────────────────────────────────────────────────┤
L43.4 ─────────────────────────────────────────────────────────────────────┤
                                                                          └─> L43.5 ─> L43.6 ─> L44.1 ─> L44.2 ─> L44.3
```

**关键路径（G 版主链，12 跳）**：`L39.1 → L39.2 → L39.4 → L39.5 → L40.2 → L40.3 → L40.4 → L40.6 → L43.2 → L43.6 → L44.1 → L44.2`。
**次长链**：注入拦截 `L39.1 → L39.2 → L39.3`；字节预算 `L39.4 → L41.1 → L41.2 → L41.3`；严格性 `L39.4 → L42.1 → L42.2 → L42.4`。
**并行簇**：S1 = L39.1 / L39.4 / L40.1；S2 = L39.2 / L40.2 / L41.1 / L42.1；S6 = L43.1 / L43.2 / L43.4。
**串行汇聚点**：`gateway.rs`（L39.1 → L39.2 → L39.3 → L41.2 → L40.6）、`normalize.rs`（L40.2 → L40.3 → L40.4）、`llm_live.rs`（L41.3 → L43.5）。

# 附录 S：阶段 G 可立即执行的首批任务

| 顺序 | 任务 | 动作 | 文件 | 前置 |
|---|---|---|---|---|
| 1 | **L39.1** | `system_prompt` 追加完整 Schema 本体 + 分隔常量，四类共用 | `crates/infrastructure/src/llm/gateway.rs` | 无（L32.1 已落地） |
| 2 | **L39.4** | 新建 `wire.rs`：四类 wire 容器 + `ModelOutputDomain` + `mod.rs` 导出 1 行 | `crates/infrastructure/src/llm/wire.rs`（新增 #34）、`crates/infrastructure/src/llm/mod.rs` | 无 |
| 3 | **L40.1** | domain 最小增量：上下文三类型 + trace 三类型 + 请求 `context` + 结果 `normalization` | `crates/domain/src/model/llm.rs` | 无 |
| 4 | **L41.1** | `max_request_bytes`（默认 64 KiB）+ `validate` 范围 + 默认值内联用例 | `crates/infrastructure/src/llm/mod.rs` | L39.4（同文件导出行，须串行） |

> 第 1~3 项**互不共享可写文件**，可并行或单人顺序执行；第 4 项与第 2 项同写 `mod.rs`，必须排在 L39.4 之后。
> 第二批（依赖首批）：L39.2、L39.5、L40.2、L42.1。第三批：L39.3、L40.3、L40.5、L41.2、L41.4。
> **开工前置**：K1~K4 与 Q-5~Q-8 已确认，无需等待；真实面执行前必须由用户终端注入七项变量，agent 不写入任何值。

# 附录 T：阶段 G 收口检查单（复制即用）

```
[ ] G-FMT          cargo fmt --all --check                                     → 0
[ ] G-CLIPPY       cargo clippy --workspace --all-targets -- -D warnings        → 0
[ ] G-CLIPPY-F     cargo clippy -p meeting-copilot-infrastructure --all-targets \
                     --features llm-https -- -D warnings                        → 0
[ ] G-TEST         cargo test --workspace                                      → failed=0, passed ≥ 240（硬基线）
[ ] G-TEST-目标    cargo test --workspace                                      → passed ≥ 265（DESIGN 2.37.3 口径；未达须记缺口）
[ ] G-COUNT        cargo test --workspace -- --list | grep -c ": test"         → ≥ 265
[ ] G-TEST-TLS     cargo test -p meeting-copilot-infrastructure --features llm-https → failed=0, passed ≥ 阶段 F 实测 + 15（参考 179）
[ ] G-SCHEMA-INJECT --test llm_schema_injection 全绿；--lib system_prompt_contains_full_schema_for_four_tasks → 0
[ ] G-NORM         --test llm_normalize 全绿；--test llm_strictness model_domain_difference_set_is_exact → 0
[ ] G-STRICT       deny_unknown_fields ≥ 基线+4；wire 必填无 serde(default)；normalize 无 unwrap_or_default → 0
[ ] G-LAYER        bash scripts/check-layering.sh                              → 0
[ ] G-SECRET       bash scripts/scan-secrets.sh                                → 0
[ ] G-SECRET-DOC   bash scripts/scan-secrets-docs.sh --all-md --artifacts       → 0（2 = 失败）
[ ] G-CAND         bash scripts/check-model-candidates.sh                       → 0（2 = 失败）
[ ] G-DENY         cargo deny check licenses bans sources advisories           → 0
[ ] G-OPENSSL      cargo tree -p meeting-copilot-infrastructure --features llm-https -e normal | grep -cE "openssl|openssl-sys" → 0
[ ] G-PRIVKEY      git grep -nE "BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY"        → 0 命中
[ ] G-PLACEHOLDER  rg -n "max\(1\)|unwrap_or\(1\)|let total_ms = [0-9]" tests/llm_live.rs → 0 命中
[ ] G-FMT-4XX      rg -n "400\.\.500" src/llm/gateway.rs                       → 0 命中
[ ] G-STRICT-NO-DEF rg -n "unwrap_or_default|unwrap_or\(" src/llm/normalize.rs → 0 命中
[ ] 状态保持       git diff --stat docs/sdd                                   → 空
[ ] 零入库         git status --short | grep '.tmp/llm-live'                   → 空
[ ] 无注入值       rg -n "LLM_API_KEY\s*=|LLM_BASE_URL\s*=" crates scripts     → 0 命中
[ ] 环境受限       无凭据时真实面标"未执行（无凭据）"，不记 0、不记通过          → 记录在案
[ ] 不得宣称       AC-20 不因本次修复满足；AC-43~AC-46 通过 ≠ 首 token/整次耗时达标；AC-36 未通过前流式只能标未实测 → 记录在案
```
## 阶段 G 执行状态（2026-11-21）

> 本节记录实际状态，不逐项勾选上方 28 个子任务；未勾选项不代表已完成。

### 已落地
- system prompt 注入完整 JSON Schema；`InsightKind` 导出为 Schema 枚举。
- `Summary`/`MinutesDraft` 文本上限由 60 提升到 2000。
- Summary/Minutes/Card/Insight 容错 wire 解码；剥离 ```json 代码围栏。
- 真实探针载荷改为中文会议片段；一次性 `tests/llm_debug_raw.rs` 已删除。

### 真实结果（单样本链路）
- 8 个 ignored 用例：**8 passed / 0 failed**，整轮约 16s。
- 四类任务全部 `degraded=false`，`http_status=200`，`failure_class=null`。

### 30 样本基线（2026-11-21）
- `RUN_ID=18ea0235a172d040-524-0`；`samples=120`；`calls=122`。
- insight/summary/card：30/30 原生成功；首 token p95 1053/1039/1121 ms；总耗时 p95 1721/1451/1634 ms。
- minutes：29/30（96.7%）；首 token p95 969 ms；总耗时 p95 2006 ms；1 次 timeout（seq=1）。
- **AC-20 达标**（四类首 token p95 ≤1.2s，30 样本/任务）；**AC-21 可行**（总 p95 ≤6s）；**AC-22 已记录**（119/120 原生成功）。

### 已落地（续）
- **64 KiB 请求体字节预算**：默认 65536，范围 1024..=1048576；`build_http` 在发出前拒绝超限，请求计数 0；新增离线用例 `request_body_byte_budget_rejects_without_send` 已通过。
- **normalization trace**：`StructuredOutcome` 增加 `normalization: Vec<NormalizationEntry>`；只含字段名 + 来源枚举（Request/Context/Computed），不含任何文本值；Summary/Card/Minutes 归一均有留痕。

### 未完成
- AC-23/24/25 仍未实测；AC-18 由 13.4 判定。

### 当前门禁基线
- `cargo test --workspace`：**265 passed / 0 failed**（阶段 G 门禁达成：244 基线 + 21 个具名收口用例）。
- fmt / clippy（含 feature）/ 层禁 / 密钥扫描 / 文档扫描 / 候选脚本 / deny / openssl：最近一次均通过。