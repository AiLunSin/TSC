# B2.23 通道仲裁与状态机（离线可判定部分）· 编码任务清单

> **追溯**：`.codeartsdoer/specs/capture_channel_state/spec.md`（AC-B2-23-01~35、K1~K10 默认口径）、`.codeartsdoer/specs/capture_channel_state/design.md`（R9~R20、O1~O5、21 个新增文件 + 6 个修改文件）、`docs/sdd/tasks.md` 任务 23.1~23.4（批次 B2）。
> **权威依据**：`docs/sdd/spec.md` 4.2-7、5.1.1-11、5.8.1-4、5.8.1-5；`docs/sdd/design.md` 2.1.3(2)、2.1.3(4)、2.5.4（仲裁规则 1/2/3/5）、2.5.7-4。
> **本轮交付边界**：只交付"断网、无真实平台进程、无音频设备、无 UI"下可判定的逻辑；**不得**据本清单结论宣称 B2 G2 通过。
> **本文性质**：任务分解与验收判据，**不含实现代码**；不修改 `docs/sdd/*.md`、不修改 `spec.md`/`design.md`、不在本轮修改任何既有源码签名。

## 全局执行约定（适用于全部任务）

| 编号 | 约定 | 判定方式 |
|---|---|---|
| G-1 | **时间推进唯一入口**为 `CaptureChannelService::advance_to(now: MonoMillis)`，由 application 层统一调度：默认周期 **100 ms**（`CapturePolicy::advance_interval_ms`，允许 100~250，硬上限 500，SPEC 4.1-2）；每次 `observe` 处理后以证据时刻隐式推进一次；阈值判定只用 `MonoMillis`，**禁止** `WallClock` 参与比较 | 代码扫描：`crates/application/src/capture/` 无 `thread::sleep`、`Instant::now`、`SystemTime::now`；测试只显式调用 `advance_to` |
| G-2 | 测试**禁止真实等待**：一律注入 `CaptureClock`（`ManualClock`），以 `advance_by(ms)` 推进；断言用单调差值与边界值（4999/5000、9999/10000、999/1000），不得用墙钟或超时 | 离线门禁脚本 + 全量测试无 sleep |
| G-3 | **零新增依赖**：仅 `std` + 既有 `serde`/`thiserror`；`Cargo.toml`/`Cargo.lock` 零改动 | `git diff --name-only -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml'` 为空 |
| G-4 | **零转录**：不写凭据、端点、模型值；状态事件/快照/切换记录/缺口记录/审计事件只允许白名单字段 | 字段白名单测试 + `bash scripts/scan-secrets.sh` 退出码 0 |
| G-5 | **契约冻结**：`SourceHealth`、`Platform`、`ChannelPreference`、`Subscription`、四个仓储、`ErrorCode`（12 变体）、`Metric::ALL`（10 项）、`AuditAction` 一律不改 | `capture_contract_freeze` 测试 + `git diff` 名单 |
| G-6 | **不新增对外错误码**："采集不可用"复用 `CopilotError::AudioCaptureFailed`（`E-AUDIO-002`） | 契约冻结测试断言 `ErrorCode::ALL.len() == 12` |
| G-7 | 中间里程碑允许方法体暂为占位，但**阶段 F 前** `crates/{domain,application,infrastructure}/src/**` 中 `todo!`/`unimplemented!`/`panic!` 出现次数必须为 0 | 阶段 F 扫描脚本退出码 0 |

## 非目标（本轮不做，也不得据此宣称完成）

1. 不实现任何真实音频捕获（WASAPI / 进程级环回 / SCStream / 设备回退链）——属 B2.15/16。
2. 不实现任何字幕读取（UIA / AX / 轮询 / OCR / ROI / 帧差）——属 B2.17~22。
3. 不实现平台探测与版本矩阵的真实读取——属 B2.22；本轮只消费其结论。
4. 不实现双通道去重合并与 `dualChannel` 标记——属 B2.24。
5. 不实现音频前处理、200–400ms 分块、8s 环形缓冲与峰值保护——属 B2.25。
6. 不实现会话六状态机与唯一写入点——属 B2.26；本轮只**读**会话状态，且失效期间**零迁移请求**。
7. 不实现采集告知与同意（B2.27）。
8. 不实现任何 UI 渲染、托盘、悬浮窗、通知；`ui-bridge` 与 `app/src-tauri` **零改动**。
9. 不实现"仅麦克风"通道键（`Platform` 无该值，K4）。
10. 不引入异步运行时/网络库/设备库；不访问网络、不打开设备、不读凭据。
11. **不得**宣称三平台 30 分钟稳定、真实 5s 回退、真实 UI 推送时延、真实去重已验证（AC-35）。

---

## 阶段 A：domain 四状态 + 迁移表 + 穷举测试（AC-B2-23-01~09）

**阶段目标**：把"四状态、9 条迁移边、非法迁移拒绝、同状态无操作、降级内调整"下沉为纯规则，可在 domain 穷举判定。
**阶段退出条件**：`channel_state_table` 全绿，且 domain 内无任何 IO、无共享可变全局状态。

- [ ] **1. 建立通道领域类型与追加式导出**
  - 追溯：AC-B2-23-01/02/03；SPEC 5.1.1-1/2/3/14、6.1-1/2/3/4；DESIGN 2.1.3(2)
  - 前置：无；并行：1.1~1.4 同批可并行
  - 触达：`crates/domain/src/model/channel.rs`（新增 #1）、`crates/domain/src/model/mod.rs`（M1）、`crates/domain/src/vo/time.rs`（M2）、`crates/domain/src/vo/mod.rs`（M3）
  - DoD：`cargo build -p meeting-copilot-domain` 退出码 0；`cargo test -p meeting-copilot-domain --lib` 退出码 0

  - [ ] **1.1 定义 `ChannelKind` / `ChannelState` / `ChannelKey`**
    - 追溯：AC-01、AC-02；SPEC 5.1.1-1/2/3、6.1-1/2；DESIGN 2.1.3(2)
    - 前置：无；并行：1.2、1.3
    - 触达：`crates/domain/src/model/channel.rs`、`crates/domain/src/model/mod.rs`
    - DoD：`cargo test -p meeting-copilot-domain --lib` 退出码 0；断言 `ChannelKind` 取值恰 2、`ChannelState::ALL.len() == 4`、`is_usable()` 仅 `Available|Degraded` 为 true、`ChannelKey` 实现 `Ord + Hash + Serialize + Deserialize` 且 `serde(rename_all)` 稳定

  - [ ] **1.2 定义三类原因码（封闭、互不合并）**
    - 追溯：AC-05、AC-07、AC-12、AC-15、AC-21、AC-26；SPEC 6.1-4、6.2-3、6.3-4；DESIGN 2.2.2.1
    - 前置：1.1；并行：1.3
    - 触达：`crates/domain/src/model/channel.rs`
    - DoD：`cargo test -p meeting-copilot-domain --lib` 退出码 0；断言 `StateReason` 恰为 design 2.2.2.1 列出的 **14** 个值（覆盖 SPEC 6.1-4 全部成因；design 文字称"15 值"与枚举清单存在 1 之差，按清单执行并登记为待确认 X1）、`SwitchReason` 恰 9 值、`GapReason` 恰 2 值；三者不得携带 `String`

  - [ ] **1.3 追加 `MonoMillis` 值对象（只追加，不改既有）**
    - 追溯：AC-12、AC-14、AC-17、AC-21、AC-24；SPEC 4.2-6、6.1-5/6、6.2-4；DESIGN 2.2.2.2、D3
    - 前置：无；并行：1.1、1.2
    - 触达：`crates/domain/src/vo/time.rs`（M2）、`crates/domain/src/vo/mod.rs`（M3）
    - DoD：`cargo test -p meeting-copilot-domain --lib` 退出码 0；断言 `MonoMillis` 为 `u64` newtype、提供 `ZERO/from_millis/as_millis/saturating_sub/checked_add/to_wall`、实现 `Ord + Copy + Serialize`；`git diff --stat` 显示 `AudioClock`/`WallClock` 定义与既有 2 个测试**零改动**

  - [ ] **1.4 注册模块与追加导出**
    - 追溯：AC-34；SPEC 4.5-4；DESIGN 2.6.2-M1/M3
    - 前置：1.1~1.3；并行：无
    - 触达：`crates/domain/src/model/mod.rs`、`crates/domain/src/vo/mod.rs`
    - DoD：`cargo build -p meeting-copilot-domain` 退出码 0；`git diff` 中两个 mod 文件只出现新增行（无删除/改名），既有 `pub use` 行原样保留

- [ ] **2. 实现迁移表与迁移判定（唯一事实来源）**
  - 追溯：AC-B2-23-02/03/04/05/09；SPEC 5.1.1-4~15；DESIGN 2.1.3(2)、2.4.1
  - 前置：1.1、1.2；并行：2.1、2.2、2.3 需顺序（同文件）
  - 触达：`crates/domain/src/model/channel.rs`
  - DoD：`cargo test -p meeting-copilot-domain --test channel_state_table` 退出码 0

  - [ ] **2.1 落地 `legal_targets`（8 条状态变更边，自环单列）**
    - 追溯：AC-02、AC-03；SPEC 5.1.1-4~11；DESIGN 2.1.3(2)"9 条合法迁移边"
    - 前置：1.1；并行：2.2
    - 触达：`crates/domain/src/model/channel.rs`
    - DoD：断言 `Probing → [Available, Degraded, Unavailable]`、`Available → [Degraded, Unavailable]`、`Degraded → [Available, Unavailable]`、`Unavailable → [Probing]`；`legal_targets` 为 `&'static [ChannelState]` 且是唯一目标集合来源（`application` 中不得出现硬编码目标集合，由阶段 F 扫描断言）

  - [ ] **2.2 落地 `transition` 与 `TransitionOutcome` 的固定判定优先级**
    - 追溯：AC-02/03/04/05；SPEC 5.1.1-12/14、4.2-2；DESIGN 2.4.1
    - 前置：2.1；并行：2.3
    - 触达：`crates/domain/src/model/channel.rs`
    - DoD：断言优先级 **`InternalAdjustment` > `NoOp` > `Migrated` > `Rejected`**：`Degraded→Degraded` 且 reason ∈ {`OcrOnly`,`DeviceFallback`} 返回 `InternalAdjustment`；其余同状态返回 `NoOp`；目标 ∈ `legal_targets` 返回 `Migrated{from,to,reason}`；否则返回 `Rejected{from,to}`；`Rejected` 是**返回值而非 `Err`**，函数不产生任何副作用（无字段写入、无事件、无计数）

  - [ ] **2.3 定义 `ChannelStateError`（独立错误类型，不进 `ErrorCode`）**
    - 追溯：AC-31、AC-09；SPEC 4.5-4、1.4-9；DESIGN 2.2.2.1、K8
    - 前置：1.1；并行：无
    - 触达：`crates/domain/src/model/channel.rs`
    - DoD：断言变体恰为 `DuplicateInstance(ChannelKey)`、`ClockRegression{seen_max, at}`、`ForeignPlatform{bound, observed}`；`cargo test -p meeting-copilot-domain --test contracts` 退出码 0（`ErrorCode` 仍 12 变体）

- [ ] **3. 领域穷举测试（16 组合全覆盖）**
  - 追溯：AC-B2-23-02/03/04/05；SPEC 8.1 判定条款；DESIGN 2.1.3(2) 的 16 组合表
  - 前置：2.1~2.3；并行：无（同文件顺序新增）
  - 触达：`crates/domain/tests/channel_state_table.rs`（新增 #4）
  - DoD：`cargo test -p meeting-copilot-domain --test channel_state_table` 退出码 0，**4 个具名用例全部 passed**（下限 4）

  - [ ] **3.1 `nine_legal_edges_covered`（9 条边逐条）**
    - 追溯：AC-02；DESIGN 2.1.3(2) 边表
    - 前置：2.2；并行：3.2
    - 触达：`crates/domain/tests/channel_state_table.rs`
    - DoD：表驱动 9 行（#1~#9），逐行断言 `Migrated` 且 `to` 等于该边目标；#9 断言 `Degraded→Degraded` 返回 `InternalAdjustment`；任一行缺覆盖即失败

  - [ ] **3.2 `sixteen_combinations_exhaustive`（4×4 全组合）**
    - 追溯：AC-03；SPEC 5.1.1-14
    - 前置：2.2；并行：3.1
    - 触达：`crates/domain/tests/channel_state_table.rs`
    - DoD：断言分布恰为 `Migrated=8`、`InternalAdjustment=1`、`NoOp=3`、`Rejected=4`；4 条非法组合为 `Available→Probing`、`Degraded→Probing`、`Unavailable→Available`、`Unavailable→Degraded`，且 `Rejected` 后"前态"不变

  - [ ] **3.3 `same_state_is_noop`（同状态 10 次）**
    - 追溯：AC-04；SPEC 5.1.1-12
    - 前置：2.2；并行：3.4
    - 触达：`crates/domain/tests/channel_state_table.rs`
    - DoD：对 `Probing/Available/Unavailable` 各 10 次同状态上报，断言 30 次全部 `NoOp`、`Err` 计数 0；本测试不涉及推送（推送计数断言在 12.7）

  - [ ] **3.4 `degraded_self_loop_internal_adjustment`**
    - 追溯：AC-05；SPEC 5.1.1-13
    - 前置：2.2；并行：3.3
    - 触达：`crates/domain/tests/channel_state_table.rs`
    - DoD：断言 `OcrOnly` 与 `DeviceFallback` 两个 reason 返回 `InternalAdjustment`；其余 reason 的 `Degraded→Degraded` 返回 `NoOp`；断言 `InternalAdjustment` 携带当前状态与原因，不携带新状态

- [ ] **4. 阶段 A 收口**
  - 追溯：AC-B2-23-01~09、AC-34；DESIGN 2.6.2-M1/M2/M3
  - 前置：3.1~3.4；并行：无
  - 触达：`crates/domain/src/model/channel.rs`、`crates/domain/tests/channel_state_table.rs`
  - DoD：见子任务

  - [ ] **4.1 模块文档写清"9 边 / 阈值 / 禁原文"三件事**
    - 追溯：SPEC 4.3-1、4.4-2；DESIGN 2.2.1 命名与类型纪律
    - 前置：3.1~3.4；并行：无
    - 触达：`crates/domain/src/model/channel.rs`（模块级文档注释）
    - DoD：模块注释明确"状态值不承担成因、阈值只用 `MonoMillis`、类型中不存在文本/样本/姓名/凭据字段"；`cargo doc -p meeting-copilot-domain --no-deps` 退出码 0 且无 warning

  - [ ] **4.2 阶段 A 门禁**
    - 追溯：AC-34；DESIGN 2.5.3
    - 前置：4.1；并行：无
    - 触达：无新增文件
    - DoD：`cargo test -p meeting-copilot-domain --test channel_state_table` 退出码 0；`cargo clippy -p meeting-copilot-domain --all-targets -- -D warnings` 退出码 0；`git diff --name-only` 仅含 `channel.rs`、`model/mod.rs`、`vo/time.rs`、`vo/mod.rs`、`tests/channel_state_table.rs` 五个路径

---

## 阶段 B：application 时钟 / 实例 / 仲裁（AC-B2-23-10~17，含 01/04~09/31/32 的实例侧）

**阶段目标**：在 application 落地可注入时钟、每（平台×通道）实例推进、决策表、5s 回退、10s 切回、切换记录、ASR 预算与抖动抑制。
**阶段退出条件**：`capture_channel_state` 与 `capture_arbiter` 全绿，且无任何真实等待。

- [ ] **5. 建立应用层骨架与 DTO（只建结构，不写业务逻辑）**
  - 追溯：AC-01、AC-10~32；SPEC 9.3；DESIGN 2.2.2.7、2.6.1 #5/#6
  - 前置：阶段 A；并行：5.1、5.2、5.3 可同批
  - 触达：`crates/application/src/capture/mod.rs`（新增 #5）、`crates/application/src/capture/types.rs`（新增 #6）、`crates/application/src/lib.rs`（M5）
  - DoD：`cargo build -p meeting-copilot-application` 退出码 0

  - [ ] **5.1 落地 `CaptureChannelService` 门面签名与装配参数**
    - 追溯：SPEC 1.2-6、1.3；DESIGN 2.2.2.7
    - 前置：阶段 A；并行：5.2
    - 触达：`crates/application/src/capture/mod.rs`
    - DoD：签名恰含 `new(clock, switch_log, gap_log, status, asr, audit, policy)`、`attach_session`、`detach_session`、`observe`、`advance_to`、`set_preference`、`retry_probe`、`active_channel`、`availability`、`snapshot`、`stats`；所有端口以 `Arc<dyn _>` 注入（不出现 infrastructure 类型）；`cargo build` 退出码 0

  - [ ] **5.2 定义输入/输出 DTO 与封闭枚举**
    - 追溯：AC-10~32；SPEC 6.1~6.4；DESIGN 2.2.2.7、2.3.2
    - 前置：1.1~1.3；并行：5.1
    - 触达：`crates/application/src/capture/types.rs`
    - DoD：断言存在 `ChannelObservation`（`session: Option<SessionId>`、`platform`、`kind`、`at: MonoMillis`、`payload: ObservationPayload`）、`ObservationPayload` 封闭 11 类、`ChannelStatusEvent`（7 字段）、`ChannelSnapshot`、`CaptureAvailability`（`Available{channel}`/`Unavailable{reason, code}`）、`ActiveChannel`（3 值）、`ChannelSwitchRecord`、`CaptureGapRecord`、`AsrIntent`/`AsrIntentKind`（3 值）、`ResumeBoundary`、`SessionContextView`、`ObservationOutcome`（4 值）；全部 `#[serde(deny_unknown_fields)]`；无任何 `String` 承载字幕文本/样本

  - [ ] **5.3 `CapturePolicy` 默认口径集中定义（全部可注入、可下调）**
    - 追溯：AC-06/07/12/14/16/17/21/22；SPEC 4.1-2、5.2.1-9/10、5.3.1-4/8；DESIGN 2.2.2.7、2.4.3、K3/K5/K6/K9
    - 前置：5.2；并行：无
    - 触达：`crates/application/src/capture/types.rs`
    - DoD：默认值断言：`advance_interval_ms=100`（允许 100~250，>500 构造失败）、`silence_fallback_ms=5000`、`stable_switch_back_ms=10000`、`read_failure_threshold=3`、`silence_blocks=3`、`ocr_failure_numerator=1/denominator=2`（严格过半）、`cooldown_window_ms=60000`、`cooldown_threshold=3`、`cooldown_duration_ms=120000`、`gap_min_ms=1000`、`gap_tolerance_ms=400`、`backfill_max_ms=8000`、`validation_interval_ms=10000`（只可加大，最小 5000）；每个字段有非零默认与越界拒绝测试

  - [ ] **5.4 注册 `pub mod capture;`（纯追加）**
    - 追溯：AC-34；DESIGN 2.6.2-M5
    - 前置：5.1~5.3；并行：无
    - 触达：`crates/application/src/lib.rs`
    - DoD：`git diff` 中该文件只有 1 行新增、既有 `governance`/`understand` 导出原样；`cargo build -p meeting-copilot-application` 退出码 0

- [ ] **6. 时钟端口与可注入替身**
  - 追溯：AC-12/14/17/24；SPEC 4.2-6、1.2-6；DESIGN 2.2.2.2、D2/D3、R9
  - 前置：5.1；并行：6.1、6.2 同批
  - 触达：`crates/domain/src/port/capture_clock.rs`（新增 #2）、`crates/domain/src/port/mod.rs`（M4）、`crates/application/tests/support/mod.rs`（新增 #11）
  - DoD：`cargo test -p meeting-copilot-application --test capture_channel_state` 退出码 0（时钟用例先行）

  - [ ] **6.1 定义 `CaptureClock` 端口并追加导出**
    - 追溯：SPEC 4.2-6；DESIGN 2.2.2.2
    - 前置：无；并行：6.2
    - 触达：`crates/domain/src/port/capture_clock.rs`、`crates/domain/src/port/mod.rs`
    - DoD：断言 trait 恰有 `mono_now() -> Result<MonoMillis, ClockError>` 与 `wall_now() -> Result<WallClock, ClockError>`，`Send + Sync`；`port/mod.rs` 只新增导出、既有 6 个模块导出不变；`cargo build -p meeting-copilot-domain` 退出码 0

  - [ ] **6.2 实现 `ManualClock`（测试专用，禁止读系统时间）**
    - 追溯：AC-12/14/24；DESIGN 2.2.2.2 替身行为
    - 前置：6.1；并行：无
    - 触达：`crates/application/tests/support/mod.rs`
    - DoD：断言初始 `mono_now()==0`、`advance_by(4999)` 后为 `4999`、`wall_now()` 返回"基准 + 偏移"且可配置基准；注入 `fail_mono()` 时返回 `ClockError`；该文件中 `Instant::now`/`SystemTime::now` 出现次数为 0

  - [ ] **6.3 `advance_to` 调度契约与阈值推进顺序**
    - 追溯：AC-12/14/17/21/23；SPEC 4.1-1/2、5.2.1-3/5；DESIGN 2.1.3(5)、R9
    - 前置：5.3、6.2；并行：7.1
    - 触达：`crates/application/src/capture/mod.rs`
    - DoD：`advance_to(now)` 内部顺序固定为"①字幕 5s 静默 → ②音频静默/恢复 → ③`Degraded→Available` 10s → ④重算仲裁（含切回与暂缓到期）→ ⑤缺口关闭/合并 → ⑥推送 drain"；`observe` 结束前以证据时刻调用一次；`advance_to` 对 `now < last_advance` 不推进、只计数 `clock_regression`；断言无任何 sleep 调用

- [ ] **7. 实例注册表 `ChannelStateRegistry`（每平台×每通道独立）**
  - 追溯：AC-01/04/05/06/07/08/09/31/32；SPEC 5.1.1-2/3/6/7/8/9/10、5.1.3；DESIGN 2.1.3(1)、2.2.2.7
  - 前置：5.1~5.4、6.1~6.3；并行：7.1→7.2→…顺序（同文件）
  - 触达：`crates/application/src/capture/channel_state.rs`（新增 #7）、`crates/application/tests/capture_channel_state.rs`（新增 #12）
  - DoD：`cargo test -p meeting-copilot-application --test capture_channel_state` 退出码 0

  - [ ] **7.1 `attach_session` 创建两实例，初态 `Probing`**
    - 追溯：AC-01、AC-08；SPEC 5.1.1-3、6.1-5
    - 前置：6.3；并行：7.2 不可（同文件）
    - 触达：`channel_state.rs`
    - DoD：断言 attach 后字幕与音频两实例状态均 `Probing`、`entered_at == changed_at == attach 时刻`；每会话实例数 ≤2（上限 6 为跨会话/跨平台键的总约束）；重复 attach 沿用既有实例并 `duplicate_instance + 1`（不重建、不重置计数）

  - [ ] **7.2 实例隔离与 6 键独立推进**
    - 追溯：AC-01；SPEC 5.1.1-2；DESIGN 2.1.3(1) 实例独立
    - 前置：7.1；并行：7.3
    - 触达：`channel_state.rs`、`capture_channel_state.rs`
    - DoD：构造 3 平台 × 2 通道 6 条互不相同证据序列，断言 6 个终态与各自序列一一对应；再断言未注入实例保持 `Probing`；断言任一实例迁移不改变其它实例的 `state`/`reason`/`changed_at`

  - [ ] **7.3 证据 → 迁移候选映射（封闭 11 类）**
    - 追溯：AC-02/06/08；SPEC 6.4-4、5.1.1-4~11；DESIGN 2.1.3(1) 边表触发列
    - 前置：7.1；并行：7.2
    - 触达：`channel_state.rs`
    - DoD：`Probe{readable,version_supported,ocr_only}` 三分支映射正确（可读+受支持→`Available`；仅 OCR 或版本失配且可读→`Degraded`；不可读→`Unavailable`）；`ReadFailure`/`WindowRebuilt`/`Silence{blocks,vad}`/`OcrStats`/`Device`/`Permission`/`UserRetry`/`OcrRoiChanged`/`AudioFallback` 各自映射到 9 条边之一；未知 `payload` 计 `unrecognized_evidence + 1` 且状态零变化

  - [ ] **7.4 `SourceHealth` 只读映射（禁止产生 `Probing`）**
    - 追溯：AC-09；SPEC 4.5-1、5.1.1-15、1.2-1；DESIGN 1.2.1、2.1.3(1)
    - 前置：7.3；并行：7.5
    - 触达：`channel_state.rs`（只读映射函数）、`crates/domain/src/port/capture.rs`（**不修改**）
    - DoD：断言 `Ready→Available`、`Degraded(_)→Degraded`、`Unavailable(_)→Unavailable` 仅作为"候选"且原因必须由 `ObservationPayload` 显式携带；`SourceHealth` 的 `String` 在入口处被丢弃、不参与原因码、不写日志；扫描状态序列断言 `Probing` 只出现在 attach 与 `UserRetry` 之后（出现其它来源即失败）

  - [ ] **7.5 阈值计数器（每实例独立）**
    - 追溯：AC-06/07/08/12/14；SPEC 5.1.1-7/8/9/10；DESIGN 2.1.3(1) 计数器表、2.4.3
    - 前置：7.3；并行：7.6
    - 触达：`channel_state.rs`
    - DoD：断言 `consecutive_read_failures` 第 2 次不变、第 3 次触发 `Available→Degraded` 并归零；`last_valid_text_at` 仅在"字符数>0 且 `Confidence ≥ 0.5`"时更新；`stable_since` 在任何失效判定时重置；`ocr_window` 用 `failed*2 > total`（51% 迁移、50% 不迁移）；`silence_blocks` 3 块且 `vad=false` 才触发音频降级

  - [ ] **7.6 拒绝与丢弃路径（AC-31/32 的实现点）**
    - 追溯：AC-31、AC-32；SPEC 4.3-3/4、5.1.3-1~5、6.4-1/2/3；DESIGN 2.1.3(6)
    - 前置：7.5；并行：7.7
    - 触达：`channel_state.rs`、`crates/domain/src/port/capture_runtime.rs`（端口在阶段 D 定义，此处先以 `CaptureAuditSink` 注入）
    - DoD：判定顺序固定"缺 `session` → 丢弃 + 1 条审计（仅 `event_kind/count`）→ 平台不一致 → 拒绝 + `cross_platform_rejected + 1` → 时刻回退 → 拒绝 + `clock_regression + 1` → 未知证据 → 忽略 + 计数"；三类拒绝均断言状态零变化、无切换记录、无片段；审计字段集合不含 `payload`/文本/样本

  - [ ] **7.7 `capture_channel_state.rs` 用例（下限 10）**
    - 追溯：AC-01/04/05/06/07/08/09/31/32
    - 前置：7.1~7.6、12.x（事件计数所需）；并行：无
    - 触达：`crates/application/tests/capture_channel_state.rs`
    - DoD：9 条 AC 各有 ≥1 具名用例 + 1 条"重复注册与时钟回退"合并用例，共 **≥10**；全部断言"状态/计数/事件数"三要素；`cargo test -p meeting-copilot-application --test capture_channel_state` 退出码 0；AC-04/05 的"对外事件数 = 0"通过 `ChannelStatusSink` 替身计数断言

- [ ] **8. 仲裁器 `ChannelArbiter`（决策表 / 回退 / 切回 / 抑制）**
  - 追溯：AC-10~17；SPEC 5.2.1、5.8.1-5、4.2-7；DESIGN 2.1.3(2)、2.4.2、2.4.3、2.5.4-1/2/3
  - 前置：7.1~7.6；并行：8.1~8.7 顺序落地（同文件）
  - 触达：`crates/application/src/capture/channel_arbiter.rs`（新增 #8）、`crates/application/tests/capture_arbiter.rs`（新增 #13）
  - DoD：`cargo test -p meeting-copilot-application --test capture_arbiter` 退出码 0

  - [ ] **8.1 决策表 `active(pref, subtitle, audio)`（16 行等价 64 组合）**
    - 追溯：AC-10、AC-11；SPEC 5.2.1-1/2/11；DESIGN 2.4.2
    - 前置：7.1；并行：8.2
    - 触达：`channel_arbiter.rs`
    - DoD：`usable(s) = s ∈ {Available, Degraded}`；`Probing` 与 `Unavailable` 一律按不可用；4 偏好 × 2 × 2 的 16 行结果与 design 2.1.3(2) 决策表逐项一致；`SubtitleOnly` 缺字幕时**绝不**落音频、`AudioOnly` 缺音频时**绝不**落字幕；`gap_reason` 按偏好取值（`SubtitleOnly`/`AudioOnly` → `PreferenceRestricted`，否则 `BothChannelsUnavailable`）

  - [ ] **8.2 5 秒失效回退（≤5000 ms）与有效文本重置**
    - 追溯：AC-12、AC-13；SPEC 4.2-7、5.2.1-3/4；DESIGN 2.4.3
    - 前置：8.1；并行：8.3
    - 触达：`channel_arbiter.rs`
    - DoD：`Available` 且 `now - last_valid_text_at ≥ 5000` → 字幕 `Available→Unavailable`（`NoValidText`）→ 重算 → `active=Audio` → 追加切换记录（`SubtitleNoValidText`）；4999 ms 时保持 `Available` 且不切；低置信（<0.5）、空文本、噪声行**不重置**计时；4999 ms 处 1 条有效文本使回退不在该窗口发生

  - [ ] **8.3 10 秒稳定切回（9999 不切 / 10000 切）**
    - 追溯：AC-14；SPEC 5.2.1-5/6；DESIGN 2.4.3
    - 前置：8.1；并行：8.2
    - 触达：`channel_arbiter.rs`
    - DoD：切回同时满足"字幕 `Available` + 偏好允许字幕 + `now - stable_since ≥ 10000` + 不在暂缓"；9999 ms 时 `active` 仍为 `Audio`；10000 ms 时切回并写记录（`SubtitleStableRecovered`）；`Degraded` 状态下不得直接切回（必须先经 10s `Degraded→Available`）

  - [ ] **8.4 切换记录必写、同毫秒合并、写失败不冻结**
    - 追溯：AC-15；SPEC 6.2、5.2.1-7、5.2.3-3；DESIGN 2.1.3(2)"切换记录必写语义"、R12
    - 前置：8.1；并行：8.5
    - 触达：`channel_arbiter.rs`
    - DoD：一次生效通道变更 → `ChannelSwitchLog::append` 恰 1 条，字段 `from/to/reason/mono_at/wall_at` 与决策一致；同一毫秒内两次变化合并为 1 条（前值→最终后值，原因取首次）；无变化不写；`append` 返回 `Err` 时只累加 `record_missing`，`active` 不回滚、不冻结；初选"无→字幕/音频"写 `SessionInitial`（两通道均 `Probing` 时不写）

  - [ ] **8.5 ASR 投递预算（字幕生效期仅校验与回补）**
    - 追溯：AC-16；SPEC 5.2.1-8/9；DESIGN 2.1.3(2)"ASR 投递预算"、K9、D5
    - 前置：8.1；并行：8.4
    - 触达：`channel_arbiter.rs`
    - DoD：`active==Subtitle` 时只允许 `Validation` 与 `GapBackfill`，两者**共用**每 10s 窗口预算；60s 内（30 条字幕 + 300 块 200ms 音频）意图数 ≤6 且用途仅前两值（出现 `Primary` 即失败）；`GapBackfill` 窗口 ≤8000ms 且 `window_end > window_start`；`active==Audio` 时 `Primary` 不受预算限制；载荷不含样本

  - [ ] **8.6 抖动抑制（60s / 3 次 / 120s）**
    - 追溯：AC-17；SPEC 5.2.1-10、5.2.3-1/4；DESIGN 2.4.3、K5
    - 前置：8.2、8.4；并行：8.7
    - 触达：`channel_arbiter.rs`
    - DoD：60s 滑动窗口第 3 次"字幕→音频"回退时置 `cooldown_until = now + 120000` 并 `cooldown_triggered + 1`；暂缓期内"字幕恢复且稳定 10s"仍不切回；到期后注入稳定 10s 才切回（记录原因 `CooldownExpired`）；用户显式 `set_preference` 可立即解除暂缓并按用户指定记录 `UserSelected`；抑制**不改变**通道状态

  - [ ] **8.7 偏好变更即时生效与跨偏好约束**
    - 追溯：AC-11（偏好转音频优先）、SPEC 5.2.1-11/12、5.2.3-2；DESIGN R18、D10
    - 前置：8.1；并行：8.6
    - 触达：`channel_arbiter.rs`
    - DoD：`set_preference` 后不等新证据立即重算，变化则写记录（`PreferenceChanged`）；断言"仅字幕且字幕不可用"→`None` + `PreferenceRestricted`，"仅音频且音频不可用"→`None` + `PreferenceRestricted`；仲裁唯一输入取 `SessionContextView::channel_preference`（`Session.channel` 口径），与 `binding.preference` 不一致只计 `PreferenceDivergence`，不改写、不报错

- [ ] **9. 仲裁与实例测试（AC-10~17 + AC-01 完整判定）**
  - 追溯：AC-01、AC-10~17；SPEC 8.2；DESIGN 2.5.1、2.5.2
  - 前置：8.1~8.7；并行：无
  - 触达：`crates/application/tests/capture_arbiter.rs`（用例核心）、`crates/application/tests/capture_channel_state.rs`（AC-01 补充）
  - DoD：`cargo test -p meeting-copilot-application --test capture_arbiter` 退出码 0，具名用例 **≥11**

  - [ ] **9.1 `decision_table_all_sixty_four_combinations`（64 行表驱动）**
    - 追溯：AC-11；SPEC 5.2.1-2
    - 前置：8.1；并行：9.2
    - 触达：`capture_arbiter.rs`
    - DoD：4 × 4 × 4 = 64 行逐行断言，偏差行数必须为 0；每行同时断言 `active` 与 `CaptureAvailability`（含 `gap_reason`）

  - [ ] **9.2 边界对（4 组）与三段时序（AC-17）**
    - 追溯：AC-06/07/12/14/17；DESIGN 2.4.3
    - 前置：8.2~8.6；并行：9.1
    - 触达：`capture_arbiter.rs`
    - DoD：断言 4999/5000、9999/10000、50%/51%、999/1000 ms 四组边界；AC-17 三段时序（第 3 次回退→置暂缓、暂缓内不切回、到期后切回）全部覆盖；全部用 `ManualClock`，无 sleep

  - [ ] **9.3 `switch_record_written_once`（记录 1 条 + 10s 不增）**
    - 追溯：AC-15；SPEC 5.2.1-7
    - 前置：8.4；并行：9.4
    - 触达：`capture_arbiter.rs`
    - DoD：变更后记录数 +1（恰 1 条）；再 `advance_to` 推进 10s 且无状态变化时记录数不变；记录 `from != to`、`mono_at` 单调不减

  - [ ] **9.4 `six_instances_are_independent`（AC-01 完整版）**
    - 追溯：AC-01；SPEC 5.1.1-2/3
    - 前置：7.2；并行：9.3
    - 触达：`capture_channel_state.rs`
    - DoD：6 实例 × 6 独立序列，逐实例断言初态与终态；断言每个实例的 `reason` 与其序列最后一条证据一致；断言仅注入 1 个实例时其余 5 个 `changed_at` 不变

---

## 阶段 C：失效保护（AC-B2-23-18~23）

**阶段目标**：双通道失效时"会话保留、要点不丢、汇总可见、缺口准确、恢复不重不漏不补录"。
**阶段退出条件**：`capture_degraded_guard` 全绿；仓储替身显示结束/删除/清空类调用为 0。

- [ ] **10. 失效保护 `DegradedGuard`（汇总 / 缺口 / 恢复边界）**
  - 追溯：AC-18~23；SPEC 4.2-7、5.3.1、5.3.3；DESIGN 2.1.3(3)、2.5.4-5、R16
  - 前置：8.1、8.4、12.1（需 `CaptureGapLog` 与 `ChannelStatusSink` 端口）；并行：10.1→10.5 顺序
  - 触达：`crates/application/src/capture/degraded_guard.rs`（新增 #9）、`crates/application/tests/capture_degraded_guard.rs`（新增 #14）
  - DoD：`cargo test -p meeting-copilot-application --test capture_degraded_guard` 退出码 0

  - [ ] **10.1 汇总状态机（只在翻转时推送）**
    - 追溯：AC-20、AC-23；SPEC 5.3.1-3/6/9；DESIGN 2.1.3(3)、2.4.4
    - 前置：8.1；并行：10.2
    - 触达：`degraded_guard.rs`
    - DoD：`Available{channel}` ↔ `Unavailable{reason, code=AudioCaptureFailed}`；只在"可用↔不可用"翻转时各推 1 次；同一缺口内反复在 `Unavailable`/`Probing` 间变化时推送次数恒为 1；汇总变化后 ≤5000 ms 内由 `ChannelStatusSink` 扇出，且 `snapshot()` 可查到同一结论

  - [ ] **10.2 缺口区间：半开、≥1000ms 才落、合并**
    - 追溯：AC-21；SPEC 5.3.1-4/5、6.3；DESIGN 2.1.3(3)
    - 前置：10.1；并行：10.3
    - 触达：`degraded_guard.rs`
    - DoD：缺口为半开 `[open, close)`；30000 ms 失效 → 恰 1 条且时长 ∈ [29600, 30400]；500 ms 失效 → 0 条；相邻（间隔 <1000 ms）或重叠区间按并集合并（保留最早 `start` 与最晚 `end`），合并后同一会话内**无相互重叠**记录

  - [ ] **10.3 恢复边界 `ResumeBoundary` 与"不补录"**
    - 追溯：AC-22；SPEC 5.3.1-7/8；DESIGN 2.1.3(3)、R16/R19
    - 前置：10.2；并行：10.4
    - 触达：`degraded_guard.rs`
    - DoD：`resume_from = close - min(8000, gap_duration)`；缺口更早部分禁止回补（断言 `[t1, t2-8000)` 内不产生任何片段）；用合成片段做区间代数断言：两两不交、与缺口区间不重叠（仅允许 ≤400 ms 边界接触）、恢复后 10s 内片段数 ≥ 预期数 − 1

  - [ ] **10.4 会话零迁移与要点只读校验**
    - 追溯：AC-18、AC-19；SPEC 4.2-7、5.1.1-10、1.4-5；DESIGN 2.1.3(3)
    - 前置：10.1；并行：10.3
    - 触达：`degraded_guard.rs`、`crates/application/tests/support/mod.rs`（仓储计数替身）
    - DoD：失效 300 s 期间 `SessionRepository::transition` 与 `delete_irrecoverable` 调用次数为 **0**；`InsightRepository::save`/`save_summary` 调用次数为 0；失效前后要点 ID 集合与数量逐条一致；要点仓储读取失败时跳过"不丢失"校验并计 `SnapshotStale`/诊断告警，仍不发删除/清空指令

  - [ ] **10.5 单通道恢复即退出不可用（≤5000 ms）**
    - 追溯：AC-23；SPEC 5.3.1-6；DESIGN 2.1.3(3)
    - 前置：10.1~10.3；并行：无
    - 触达：`degraded_guard.rs`
    - DoD：音频 `Unavailable→Available` 且偏好 `SubtitleFirst` → 汇总 ≤5000 ms 内转为 `Available{Audio}`、`active=Audio`、缺口关闭并合并；断言切换发生时刻与恢复时刻之差 ≤5000 ms

- [ ] **11. 失效保护测试（AC-18~23，下限 8）**
  - 追溯：AC-18~23；SPEC 8.3
  - 前置：10.1~10.5；并行：无
  - 触达：`crates/application/tests/capture_degraded_guard.rs`
  - DoD：6 条 AC 各 ≥1 具名用例 + "500 ms 不落记录" + "缺口合并/重复告警" 2 条，共 **≥8**；`cargo test -p meeting-copilot-application --test capture_degraded_guard` 退出码 0

  - [ ] **11.1 会话与要点不变量（AC-18/19）**
    - 追溯：AC-18、AC-19；SPEC 8.3 判定
    - 前置：10.4；并行：11.2
    - 触达：`capture_degraded_guard.rs`
    - DoD：断言失效 300 s 期间迁移/删除/清空调用 0 次；要点数量与 ID 集合与失效前完全相同；任一删除类调用出现即用例失败

  - [ ] **11.2 汇总可见与缺口容差（AC-20/21）**
    - 追溯：AC-20、AC-21
    - 前置：10.1、10.2；并行：11.1
    - 触达：`capture_degraded_guard.rs`
    - DoD：断言推送时刻差 ≤5000 ms 且快照字段与推送一致；30000 ms → 1 条 ∈ [29600,30400]；500 ms → 0 条

  - [ ] **11.3 恢复不重不漏与单通道恢复（AC-22/23）**
    - 追溯：AC-22、AC-23
    - 前置：10.3、10.5；并行：11.4
    - 触达：`capture_degraded_guard.rs`
    - DoD：区间代数三断言（两两不交、与缺口不交、计数下限）全部通过；音频恢复后 5000 ms 内汇总转 `Available（音频）` 且 `active=Audio`

  - [ ] **11.4 重复失效不重复告警（SPEC 5.3.1-9 的机器判定）**
    - 追溯：AC-20 衍生；SPEC 5.3.1-9、5.3.3-4
    - 前置：10.1；并行：11.3
    - 触达：`capture_degraded_guard.rs`
    - DoD：失效 300 s 且通道在 `Unavailable`/`Probing` 反复时，"采集不可用"推送次数恰为 1；恢复后再次失效间隔 <1000 ms 时合并入上一缺口且不重复推送

---

## 阶段 D：状态推送 / 快照 / 订阅（AC-B2-23-24~30）

**阶段目标**：状态确定后 ≤5000 ms 推送、同状态去重、原因变化只更新快照、序号有序不丢、取消后 0 回调、载荷白名单。
**阶段退出条件**：`capture_status_push` 全绿；字段扫描与文本/样本子串命中为 0。

- [ ] **12. 状态端口与 `StatusPublisher`**
  - 追溯：AC-20、AC-24~30；SPEC 5.4.1、4.1-1、4.3-1；DESIGN 2.1.3(4)、2.2.2.4、D6
  - 前置：5.2、7.1；并行：12.1→12.6 顺序（12.1 可先行）
  - 触达：`crates/domain/src/port/capture_runtime.rs`（新增 #3）、`crates/domain/src/port/mod.rs`（M4）、`crates/application/src/capture/status_publisher.rs`（新增 #10）、`crates/application/tests/capture_status_push.rs`（新增 #15）
  - DoD：`cargo test -p meeting-copilot-application --test capture_status_push` 退出码 0

  - [ ] **12.1 定义运行期端口（单一能力，不设万能方法）**
    - 追溯：AC-15/16/20/24/29/31/32；SPEC 9.3；DESIGN 2.2.2.3/2.2.2.4/2.2.2.5/2.2.2.6、R17
    - 前置：阶段 A；并行：12.2
    - 触达：`capture_runtime.rs`、`port/mod.rs`
    - DoD：断言存在 `ChannelSwitchLog`（`append`/`list`）、`CaptureGapLog`（`upsert`/`list`）、`ChannelStatusSink`（`publish_state`/`publish_availability`/`subscribe`/`snapshot`）、`StatusSubscription`（`is_active`/`release`）、`AsrDeliverySink`（`deliver`/`counts`）、`CaptureAuditSink`（`record`/`counts`）；每个 trait 无 `update`/`delete`/通用 `send(Any)` 方法；`port/mod.rs` 只新增导出行

  - [ ] **12.2 `StatusPublisher`：去重 + 序号 + 先快照后扇出**
    - 追溯：AC-24/25/26/27/28；SPEC 5.4.1-1~9；DESIGN 2.1.3(4)
    - 前置：12.1；并行：12.3
    - 触达：`status_publisher.rs`
    - DoD：去重键 `(ChannelKey, ChannelState)`（原因不参与）；状态相同 → 只更新快照 `reason`/`changed_at` 并返回 `NoPush`；状态不同 → 先写快照 → 再分配序号 → 再扇出；序号每会话一条流、`u64` 严格递增、不回绕（接近 `u64::MAX` 时拒绝发布并计数）；`publish_state` 返回时快照已含该状态

  - [ ] **12.3 订阅扇出：有界队列、慢/抛错隔离、release 后 0 回调**
    - 追溯：AC-29、SPEC 5.4.3-1/2、4.2-4；DESIGN 2.1.3(4)"扇出模型"、D6
    - 前置：12.2；并行：12.4
    - 触达：`status_publisher.rs`
    - DoD：每订阅者一条容量 64 的有界队列，`publish` 后立即 `drain`；单订阅者处理超 50 ms 则留队返回、不阻塞状态机与其它订阅者；回调抛错只计 `SubscriberFailed` 并继续；`StatusSubscription::release()` 幂等，释放后回调次数 0，其它订阅者不受影响；队列溢出丢最旧并计数

  - [ ] **12.4 时延打点与 p99 样本（离线口径）**
    - 追溯：AC-24；SPEC 4.1-1、5.4.1-1；DESIGN 2.1.3(4)、R2/R11
    - 前置：12.2、12.3；并行：12.5
    - 触达：`status_publisher.rs`
    - DoD：事件携带 `determined_at`；回调返回后读取 `MonoMillis` 记录样本，环形保存最多 256 个；提供 p99 与 max 计算；200 次变更样本的 p99 ≤5000 且 max ≤5000；代码注释必须写明"离线只证逻辑上限，不代表真实 UI 线程时延"，且**不得**出现 G2 结论（AC-35）

  - [ ] **12.5 载荷白名单（事件 7 字段 / 快照 6 字段）**
    - 追溯：AC-30；SPEC 4.3-1/2、6.1、5.4.1-7；DESIGN 2.4.5
    - 前置：12.2；并行：12.4
    - 触达：`status_publisher.rs`、`crates/application/src/capture/types.rs`
    - DoD：事件字段集合 ⊆ `{platform, channel_kind, state, reason, sequence, determined_at, wall_at}`；快照条目 ⊆ `{platform, channel_kind, state, reason, entered_at, changed_at}`（无序号、无历史）；对输入过的字幕文本与音频样本做子串扫描，命中数为 0；`CaptureAvailabilityEvent` ⊆ `{availability, gap_reason, sequence, determined_at}`

  - [ ] **12.6 汇总事件与通道事件同流（序号约束）**
    - 追溯：AC-20；SPEC 5.4.1-8
    - 前置：12.2、12.3；并行：12.5
    - 触达：`status_publisher.rs`
    - DoD：汇总由可用转不可用时，订阅者 ≤5000 ms 内收到 1 条汇总事件，其序号大于此前所有通道事件；汇总与通道事件共用同一递增序列，不出现两套序号

- [ ] **13. 推送 / 快照测试（AC-24~30，下限 9）**
  - 追溯：AC-24~30；SPEC 8.4
  - 前置：12.1~12.6；并行：无
  - 触达：`crates/application/tests/capture_status_push.rs`
  - DoD：7 条 AC 各 ≥1 具名用例 + "慢/抛错订阅者" + "无订阅者仍更新快照" 2 条，共 **≥9**；`cargo test -p meeting-copilot-application --test capture_status_push` 退出码 0

  - [ ] **13.1 p99 / 去重 / 原因变化（AC-24/25/26）**
    - 追溯：AC-24、AC-25、AC-26
    - 前置：12.2、12.4；并行：13.2
    - 触达：`capture_status_push.rs`
    - DoD：200 样本 p99 与 max 均 ≤5000；同一 `(key,state)` 连续 10 次仅 1 条推送；状态保持 `Degraded` 而原因由 `OcrOnly` 变为 `VersionMismatch` 时事件数不变、快照 `reason == VersionMismatch`（引用枚举而非字面量）

  - [ ] **13.2 快照强一致 / 序号 / 取消（AC-27/28/29）**
    - 追溯：AC-27、AC-28、AC-29
    - 前置：12.2、12.3；并行：13.1
    - 触达：`capture_status_push.rs`
    - DoD：在回调内查询快照可见本次状态；6 键查询条目数恰为 6 且每键 1 条；同一键 100 次变更的序号严格递增且长度恰 100；取消后该订阅者 0 回调、其余各 10 次

  - [ ] **13.3 载荷白名单与"无历史"（AC-30）**
    - 追溯：AC-30；SPEC 5.4.1-7/9、4.3-1
    - 前置：12.5；并行：13.4
    - 触达：`capture_status_push.rs`
    - DoD：字段名集合白名单比对 0 越界；对预置的字幕文本与音频样本做子串扫描 0 命中；某通道变更 5 次后快照仅含 1 条最新值

  - [ ] **13.4 慢订阅者 / 抛错 / 无订阅者（SPEC 5.4.3 异常场景）**
    - 追溯：AC-24 衍生；SPEC 5.4.3-1/2/5
    - 前置：12.3；并行：13.3
    - 触达：`capture_status_push.rs`
    - DoD：慢订阅者不阻塞状态机与其它订阅者的回调次数；抛错订阅者失败计数 +1 且流不断；无订阅者时快照仍更新，后续订阅者可查到完整当前状态

---

## 阶段 E：infrastructure 替身 + 离线装配（AC-B2-23-31~35）

**阶段目标**：用内存替身与系统时钟完成端口实现，证明整套能力可在断网、无设备、无 UI 下装配并判定。
**阶段退出条件**：`capture_fakes`/`capture_offline_gate`/`capture_contract_freeze` 全绿；两个脚本退出码 0。

- [ ] **14. infrastructure 内存替身与 `SystemCaptureClock`（不含任何平台代码）**
  - 追溯：AC-15/16/20/21/24/31/32/33；SPEC 1.4-10；DESIGN 2.2.2、2.6.1 #18/#19
  - 前置：12.1；并行：14.1~14.5 同批可并行（不同结构体，同文件）
  - 触达：`crates/infrastructure/src/capture/mod.rs`（新增 #18）、`crates/infrastructure/tests/capture_fakes.rs`（新增 #19）、`crates/infrastructure/src/lib.rs`（M6）
  - DoD：`cargo test -p meeting-copilot-infrastructure --test capture_fakes` 退出码 0；`cargo build -p meeting-copilot-infrastructure` 退出码 0

  - [ ] **14.1 `InMemorySwitchLog`（只追加、不可改、按会话列出）**
    - 追溯：AC-15；SPEC 6.2-6；DESIGN 2.2.2.3
    - 前置：12.1；并行：14.2
    - 触达：`infrastructure/src/capture/mod.rs`
    - DoD：`append` 后 `list` 长度 +1；同一毫秒同一会话最多 1 条（由调用方保证，替身断言不出现两条同毫秒记录）；无 `update`/`delete` 方法；`Arc<Self>` 可跨线程共享（`Send + Sync`）

  - [ ] **14.2 `InMemoryGapLog`（`upsert` 合并语义）**
    - 追溯：AC-21；SPEC 5.3.1-5、6.3-5；DESIGN 2.2.2.3
    - 前置：12.1；并行：14.1
    - 触达：`infrastructure/src/capture/mod.rs`
    - DoD：相邻（间隔 <1000 ms）或重叠区间 `upsert` 后合并为 1 条、保留最早 `start` 与最晚 `end`；`list` 返回按 `start` 升序且两两不重叠；`end > start`

  - [ ] **14.3 `InMemoryAsrSink` / `InMemoryAudit`（只计数、无载荷）**
    - 追溯：AC-16、AC-32；SPEC 4.3-3；DESIGN 2.2.2.5/2.2.2.6
    - 前置：12.1；并行：14.4
    - 触达：`infrastructure/src/capture/mod.rs`
    - DoD：`deliver` 累计 `validation/backfill/primary` 三计数；`record` 累计 `CaptureStats` 六计数；审计事件结构体无 `payload` 字段；`counts()` 与投递次数一致

  - [ ] **14.4 `SystemCaptureClock`（`Instant` 偏移 + 墙钟）**
    - 追溯：AC-12/14；SPEC 4.2-6；DESIGN 2.7.2-5
    - 前置：6.1；并行：14.3
    - 触达：`infrastructure/src/capture/mod.rs`
    - DoD：`mono_now()` 单进程内单调不减（连续 2 次调用后者 ≥ 前者）；`wall_now()` 委托既有 `WallClock::now()`；失败映射既有 `ClockError`；不 panic；该文件不访问网络、不打开设备

  - [ ] **14.5 `capture_fakes.rs` 端口契约用例（下限 4）**
    - 追溯：AC-15/16/21/32；DESIGN 2.6.1 #19
    - 前置：14.1~14.4；并行：无
    - 触达：`crates/infrastructure/tests/capture_fakes.rs`
    - DoD：4 条具名用例（switch log 追加/不可变、gap 合并、asr+audit 计数、clock 单调）；`cargo test -p meeting-copilot-infrastructure --test capture_fakes` 退出码 0

- [ ] **15. 离线装配与离线门禁（AC-33）**
  - 追溯：AC-33；SPEC 8 判定环境约束、7.1-1~4；DESIGN 2.5.1、2.6.1 #16/#20、O5
  - 前置：12.1、14.1~14.4；并行：15.1→15.3 顺序
  - 触达：`crates/application/tests/capture_offline_gate.rs`（新增 #16）、`crates/application/tests/support/mod.rs`（#11）、`scripts/check-capture-offline.sh`（新增 #20）
  - DoD：`cargo test -p meeting-copilot-application --test capture_offline_gate` 退出码 0；`bash scripts/check-capture-offline.sh` 退出码 0

  - [ ] **15.1 全替身装配函数 `build_offline_service()`**
    - 追溯：SPEC 8 判定环境约束；DESIGN 2.1.1 上下文视图
    - 前置：14.1~14.4；并行：15.2
    - 触达：`crates/application/tests/support/mod.rs`
    - DoD：装配 `ManualClock` + 三个内存日志/投递/审计替身 + 内存 `ChannelStatusSink` + 默认 `CapturePolicy`；函数内不出现 `SystemCaptureClock`、不出现 `std::net`、不出现 sleep；同一函数可被 4 个测试文件复用（`mod support;`）

  - [ ] **15.2 `NetworkProbe` 与建连次数 0 断言**
    - 追溯：AC-33；DESIGN 2.1.3(6)"网络探针"
    - 前置：15.1；并行：15.3
    - 触达：`crates/application/tests/capture_offline_gate.rs`、`support/mod.rs`
    - DoD：`NetworkProbe{connections: AtomicUsize}` 在测试内被读取且 `== 0`；用例同时跑一遍 AC-01 最小路径（attach→probe→active）证明装配闭环

  - [ ] **15.3 `scripts/check-capture-offline.sh`（依赖闭包 + 源码禁用调用）**
    - 追溯：AC-33；DESIGN 2.6.1 #20
    - 前置：15.1；并行：15.4
    - 触达：`scripts/check-capture-offline.sh`
    - DoD：断言 `cargo tree -p meeting-copilot-domain/application --edges normal` 无 `reqwest|hyper|sqlx|rusqlite|native-tls|windows|objc2|tauri`；断言 `crates/application/src/capture/` 与 `crates/domain/src/model/channel.rs` 中 `std::net`/`TcpStream`/`thread::sleep`/`Instant::now`/`SystemTime::now` 出现次数为 0；脚本退出码 0，失败时打印命中行并退出 1

- [ ] **16. 既有契约冻结（AC-34）**
  - 追溯：AC-34；SPEC 4.5、4.5-5；DESIGN 2.5.2、2.6.3、R20
  - 前置：阶段 A~D；并行：16.1→16.3 顺序
  - 触达：`crates/application/tests/capture_contract_freeze.rs`（新增 #17）
  - DoD：`cargo test -p meeting-copilot-application --test capture_contract_freeze` 退出码 0，具名用例 **≥7**

  - [ ] **16.1 六个既有契约的签名/变体冻结**
    - 追溯：AC-34；SPEC 4.5-1~4
    - 前置：阶段 A~D；并行：16.2
    - 触达：`capture_contract_freeze.rs`
    - DoD：断言 `SourceHealth` 3 变体、`Platform` 3 值、`ChannelPreference` 4 值、`Subscription` 提供 `new/is_active/release/wrap`、四个仓储方法名集合与参数个数不变；断言 `ErrorCode::ALL.len() == 12`、`Metric::ALL.len() == 10`；6 条契约各有 1 条具名用例

  - [ ] **16.2 既有测试零删改（git 名单）**
    - 追溯：AC-34；DESIGN 2.6.3
    - 前置：16.1；并行：16.3
    - 触达：无新增文件
    - DoD：`git diff --name-only -- 'crates/*/tests'` 只含本清单新增的 8 个测试文件，不含任一既有测试文件；既有 `crates/domain/tests/contracts.rs` 内容 `git diff` 为空

  - [ ] **16.3 基线测试数不减**
    - 追溯：AC-34；SPEC 4.5-5
    - 前置：16.1；并行：16.2
    - 触达：无
    - DoD：`cargo test --workspace` 输出中 `passed` 总数 ≥ **320**（既有 265 + 新增 ≥55）；`failed == 0`；与改动前基线 265 逐 crate 对比无减少（记录在验收记录中，不写入 `docs/sdd`）

- [ ] **17. "不得宣称 G2 通过"扫描（AC-35）**
  - 追溯：AC-35；SPEC 7.2-11、8.5；DESIGN 2.5.2、2.7.4
  - 前置：16.3；并行：17.1→17.2
  - 触达：`scripts/check-g2-claims.sh`（新增 #21）
  - DoD：`bash scripts/check-g2-claims.sh` 退出码 0

  - [ ] **17.1 脚本：扫描产出文本与提交信息**
    - 追溯：AC-35；DESIGN 2.5.2（AC-35 行）
    - 前置：16.3；并行：无
    - 触达：`scripts/check-g2-claims.sh`
    - DoD：扫描范围 = `README.md`、`docs/progress/`、本轮新增源码、`git log -n 20 --pretty=%B`；**排除** `docs/sdd/*.md` 与三份规格文档（`spec.md`/`design.md`/`tasks.md`，它们本身在描述门禁与禁令原文）；禁用短语以字符串拼接方式构造、并排除脚本自身，避免自命中；命中即打印并退出 1

  - [ ] **17.2 产出物文字纪律核对**
    - 追溯：AC-35；SPEC 7.2-12
    - 前置：17.1；并行：无
    - 触达：新增源码注释、提交信息
    - DoD：本轮新增文件与提交信息中不出现"三平台 30 分钟已验证""真实 5 秒回退已实测""真实去重已验证"等结论性表述；不出现任何凭据/端点/模型值；`bash scripts/scan-secrets.sh` 退出码 0

---

## 阶段 F：门禁与收口

**阶段目标**：按固定顺序跑完全部门禁，形成可复核的验收记录；确认 21 个新增 + 6 个修改文件全部落位，非目标零触碰。
**阶段退出条件**：下列命令全部退出码 0（或登记明确 SKIP），且不出现 G2 结论性表述。

- [ ] **18. 测试门禁与计数**
  - 追溯：AC-34；SPEC 8.5；DESIGN 2.5.3、2.6.3
  - 前置：阶段 E；并行：18.1~18.3 顺序
  - 触达：无
  - DoD：见子任务

  - [ ] **18.1 分层测试顺序执行（先纯规则后编排）**
    - 追溯：DESIGN 2.5.3
    - 前置：阶段 E；并行：无
    - 触达：无
    - DoD：依次执行并以退出码 0 逐一确认：`cargo test -p meeting-copilot-domain --test channel_state_table`、`cargo test -p meeting-copilot-application --test capture_channel_state`、`--test capture_arbiter`、`--test capture_degraded_guard`、`--test capture_status_push`、`--test capture_offline_gate`、`--test capture_contract_freeze`、`cargo test -p meeting-copilot-infrastructure --test capture_fakes`

  - [ ] **18.2 全量回归与基线不减**
    - 追溯：AC-34；SPEC 4.5-5
    - 前置：18.1；并行：18.3
    - 触达：无
    - DoD：`cargo test --workspace` 退出码 0；`failed == 0`；passed ≥ 320；既有 265 所在 crate 的用例数逐个不减少

  - [ ] **18.3 新增用例下限逐文件核对**
    - 追溯：AC-01~35
    - 前置：18.2；并行：无
    - 触达：无
    - DoD：按文末"新增测试下限表"逐文件核对具名用例数，**任一行低于下限即失败**；每个 AC-B2-23-01~35 均能在表中找到承接文件（映射缺 1 条即失败）

- [ ] **19. 静态与合规门禁**
  - 追溯：AC-33/34/35；SPEC 4.3、4.5；DESIGN 2.5.3、2.6.1 #20/#21
  - 前置：18.2；并行：19.1、19.2 可并行，19.3 依赖两者
  - 触达：无
  - DoD：见子任务

  - [ ] **19.1 `cargo fmt --all --check`**
    - 追溯：DESIGN 2.5.3
    - 前置：18.2；并行：19.2
    - 触达：所有新增/修改文件
    - DoD：退出码 0，diff 为 0；不使用 `--check` 之外的自动重排掩盖问题（如需要，先 fmt 再复跑）

  - [ ] **19.2 clippy（默认 feature，`-D warnings`）**
    - 追溯：DESIGN 2.5.3
    - 前置：18.2；并行：19.1
    - 触达：无
    - DoD：`cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；新增代码无 `allow(...)` 规避（除已存在的裸指针/测试辅助白名单，需在验收记录注明）

  - [ ] **19.3 clippy（相关 feature）**
    - 追溯：DESIGN 2.5.3；CI `.github/workflows/ci.yml` rust job
    - 前置：19.2；并行：无
    - 触达：无
    - DoD：`cargo clippy -p meeting-copilot-infrastructure --all-targets --features llm-https -- -D warnings` 退出码 0；`--features sqlcipher` 在具备 NASM/OpenSSL 构建链时同样 0，否则记录 **SKIP + 原因**，不得记为通过

  - [ ] **19.4 层禁 / 离线 / 密钥 / G2 四脚本**
    - 追溯：AC-33/35；SPEC 4.3-1~5
    - 前置：19.1~19.3；并行：无
    - 触达：`scripts/check-layering.sh`、`scripts/check-capture-offline.sh`、`scripts/scan-secrets.sh`、`scripts/check-g2-claims.sh`
    - DoD：四脚本退出码均为 0；密钥扫描输出"PASS: 密钥/明文扫描 0 命中"；层禁输出"PASS: 分层依赖符合约束"

- [ ] **20. 交付核对**
  - 追溯：DESIGN 2.6.1/2.6.2/2.6.3；SPEC 7.2
  - 前置：19.4；并行：20.1、20.2 可并行，20.3、20.4 顺序
  - 触达：无
  - DoD：见子任务

  - [ ] **20.1 21 个新增文件 + 6 个修改文件逐一落位**
    - 追溯：DESIGN 2.6.1/2.6.2
    - 前置：19.4；并行：20.2
    - 触达：文末文件清单
    - DoD：`git status --porcelain` 中新增/修改路径与文末清单**逐一对应**，不多不少；21 个新增文件均为 `A`（或 `??`），6 个修改文件均为 `M` 且 `git diff` 只含追加行

  - [ ] **20.2 零新增依赖**
    - 追溯：G-3；SPEC 1.4-10、7.2-10
    - 前置：19.4；并行：20.1
    - 触达：`Cargo.toml`、`Cargo.lock`、`crates/*/Cargo.toml`
    - DoD：`git diff --name-only -- Cargo.toml Cargo.lock crates/domain/Cargo.toml crates/application/Cargo.toml crates/infrastructure/Cargo.toml` 输出为空；`cargo tree --workspace --edges normal` 包集合与改动前一致

  - [ ] **20.3 非目标零触碰**
    - 追溯：SPEC 7.2；DESIGN 2.6.3
    - 前置：20.1；并行：无
    - 触达：无
    - DoD：`git diff --name-only` 中不含 `crates/platform-win/**`、`crates/platform-mac/**`、`crates/ui-bridge/**`、`app/src-tauri/**`、`docs/sdd/**`、`.codeartsdoer/specs/capture_channel_state/spec.md`、`design.md`；不含 B2.24/25/26/27 相关路径（`merge/`、`dedupe_guard.rs`、`session` 状态机、`consent`）

  - [ ] **20.4 不宣称 G2 与验收记录落点**
    - 追溯：AC-35；SPEC 7.2-11/12
    - 前置：20.2、20.3；并行：无
    - 触达：无（**不新增 md 文档**）
    - DoD：提交信息与代码注释扫描 0 命中；验收记录只以"命令 + 退出码 + 计数"形式写入本任务清单的勾选状态或提交说明，**不得**写入 `docs/sdd/*.md`；报告中不含"G2 已通过""三平台 30 分钟已验证""真实 5 秒回退已实测""真实去重已验证"

---

## 新增测试下限表（依据：35 条 AC 一一对应 + design 2.5.3 的 40~55 预计）

| # | 文件 | 承接 AC | 基础用例 | 必需边界/异常 | 下限 |
|---|---|---|---|---|---|
| 1 | `crates/domain/tests/channel_state_table.rs` | AC-02/03（AC-04/05 的规则侧） | 2 | 同状态 NoOp×3、自环 InternalAdjustment、`legal_targets` 单一事实源 | **4** |
| 2 | `crates/application/tests/capture_channel_state.rs` | AC-01/04/05/06/07/08/09/31/32 | 9 | 重复注册 + 时钟回退（合并 1 条） | **10** |
| 3 | `crates/application/tests/capture_arbiter.rs` | AC-10/11/12/13/14/15/16/17 | 8 | 4999/5000 与 9999/10000 边界对、偏好变更即时生效 | **11** |
| 4 | `crates/application/tests/capture_degraded_guard.rs` | AC-18/19/20/21/22/23 | 6 | 500 ms 不落记录、缺口合并/重复告警 | **8** |
| 5 | `crates/application/tests/capture_status_push.rs` | AC-24/25/26/27/28/29/30 | 7 | 慢/抛错订阅者、无订阅者仍更新快照 | **9** |
| 6 | `crates/application/tests/capture_offline_gate.rs` | AC-33 | 1 | `NetworkProbe == 0` + 最小闭环 | **2** |
| 7 | `crates/application/tests/capture_contract_freeze.rs` | AC-34 | 6（六契约） | `ErrorCode=12` 与 `Metric::ALL=10` 计数 | **7** |
| 8 | `crates/infrastructure/tests/capture_fakes.rs` | 端口契约（AC-15/16/21/32 支撑） | 4 | — | **4** |
| 合计 | — | AC-01~35 全覆盖 | — | — | **55** |

**计数口径**：`#[test]` 具名用例（含表驱动的一次调用）；参数化循环只算 1 条。总数门禁 = 既有 265 + 新增 55 = **≥320**。

## 门禁阈值汇总

| 门禁 | 命令 | 阈值 |
|---|---|---|
| 既有基线 | `cargo test --workspace` | 既有 **265** 全部通过、逐 crate 不减少 |
| 新增用例 | 同上 | **≥55**（逐文件见上表），总数 **≥320**，`failed == 0` |
| 格式 | `cargo fmt --all --check` | diff 0，退出码 0 |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | warning 0，退出码 0 |
| clippy（feature） | `--features llm-https`；`--features sqlcipher` | 0 warning；sqlcipher 不可构建时登记 SKIP（不得记为通过） |
| 分层 | `bash scripts/check-layering.sh` | 输出 PASS，退出码 0 |
| 离线 | `bash scripts/check-capture-offline.sh` | 网络 crate 0、`sleep`/`Instant::now`/`SystemTime::now` 0，退出码 0 |
| 密钥 | `bash scripts/scan-secrets.sh` | 0 命中，退出码 0 |
| G2 声明 | `bash scripts/check-g2-claims.sh` | 0 命中，退出码 0；**不得声称 B2 G2 通过** |
| 依赖增量 | `git diff --name-only -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml'` | 输出为空 |
| 文件增量 | `git status --porcelain` | 恰为 21 新增 + 6 修改（见下节） |

## 文件清单（对齐 design 2.6.1 / 2.6.2）

**新增 21 个**

| # | 路径 | 落地任务 |
|---|---|---|
| 1 | `crates/domain/src/model/channel.rs` | 1.1~1.2、2.1~2.3 |
| 2 | `crates/domain/src/port/capture_clock.rs` | 6.1 |
| 3 | `crates/domain/src/port/capture_runtime.rs` | 12.1 |
| 4 | `crates/domain/tests/channel_state_table.rs` | 3.1~3.4 |
| 5 | `crates/application/src/capture/mod.rs` | 5.1、6.3 |
| 6 | `crates/application/src/capture/types.rs` | 5.2、5.3 |
| 7 | `crates/application/src/capture/channel_state.rs` | 7.1~7.6 |
| 8 | `crates/application/src/capture/channel_arbiter.rs` | 8.1~8.7 |
| 9 | `crates/application/src/capture/degraded_guard.rs` | 10.1~10.5 |
| 10 | `crates/application/src/capture/status_publisher.rs` | 12.2~12.6 |
| 11 | `crates/application/tests/support/mod.rs` | 6.2、10.4、15.1、15.2 |
| 12 | `crates/application/tests/capture_channel_state.rs` | 7.7、9.4 |
| 13 | `crates/application/tests/capture_arbiter.rs` | 9.1~9.3 |
| 14 | `crates/application/tests/capture_degraded_guard.rs` | 11.1~11.4 |
| 15 | `crates/application/tests/capture_status_push.rs` | 13.1~13.4 |
| 16 | `crates/application/tests/capture_offline_gate.rs` | 15.2 |
| 17 | `crates/application/tests/capture_contract_freeze.rs` | 16.1 |
| 18 | `crates/infrastructure/src/capture/mod.rs` | 14.1~14.4 |
| 19 | `crates/infrastructure/tests/capture_fakes.rs` | 14.5 |
| 20 | `scripts/check-capture-offline.sh` | 15.3 |
| 21 | `scripts/check-g2-claims.sh` | 17.1 |

**修改 6 个（只追加，不改既有签名/语义）**

| # | 路径 | 改动 | 落地任务 |
|---|---|---|---|
| M1 | `crates/domain/src/model/mod.rs` | `pub mod channel;` + 追加导出 | 1.4 |
| M2 | `crates/domain/src/vo/time.rs` | 追加 `MonoMillis` | 1.3 |
| M3 | `crates/domain/src/vo/mod.rs` | 追加 `MonoMillis` 导出 | 1.3、1.4 |
| M4 | `crates/domain/src/port/mod.rs` | 追加 `CaptureClock` 与运行期端口导出 | 6.1、12.1 |
| M5 | `crates/application/src/lib.rs` | `pub mod capture;` | 5.4 |
| M6 | `crates/infrastructure/src/lib.rs` | `pub mod capture;` | 14 收口 |

**明确不修改**：`docs/sdd/*.md`、本规格 `spec.md`/`design.md`、`crates/domain/src/port/capture.rs`、`crates/domain/src/model/enums.rs`、`crates/domain/src/error.rs`、`crates/domain/src/model/session.rs`、`crates/domain/src/port/repository.rs`、`crates/infrastructure/src/{metrics,telemetry,audit}/`、`crates/platform-*`、`crates/ui-bridge`、`app/src-tauri`、既有 265 个测试。

## 关键路径与并行安排

```text
A(1→2→3) ──► B5(骨架/DTO/Policy) ──► B6(时钟) ──► B7(实例) ──► B8(仲裁) ──► B9(仲裁测试)
                                                                    │
                                                                    ├──► C10(失效保护) ──► C11(其测试)
                                                                    │
                                                                    └──► D12(推送) ──► D13(推送测试)
                                                                              │
A ──► 12.1(端口) ──► E14(替身) ──► E15(离线装配/脚本) ──► E16(契约冻结) ──► E17(G2 扫描) ──► F18→F19→F20
```

- **关键路径**：`1.1 → 2.1 → 2.2 → 5.1 → 6.1 → 7.1 → 7.5 → 8.1 → 8.2 → 10.1 → 10.2 → 12.2 → 15.3 → 18.2 → 19.4`。
- **可并行**：A 阶段 1.1/1.2/1.3 与 3.1~3.4 之间可分段；`12.1`（端口定义）可与 `7.x` 并行；`E14`（infrastructure 替身）可在 `D12` 之前完成，只需端口已定义；`16.x` 与 `17.x` 可与 `11.x`/`13.x` 交错。
- **串行强制**：`2.1→2.2`（同一事实来源）、`8.2/8.3` 与 `8.4/8.6`（同一仲裁状态）、`10.1→10.5`（同一缺口状态机）、`12.2→12.3`（先快照后扇出）、`18.1→18.2→18.3`。

## 首批可执行任务（可按序直接开工）

1. **1.1** — `crates/domain/src/model/channel.rs`（新建）+ `crates/domain/src/model/mod.rs`（追加 `pub mod channel;`）。
2. **1.3** — `crates/domain/src/vo/time.rs`（追加 `MonoMillis`）+ `crates/domain/src/vo/mod.rs`（追加导出）。
3. **1.2** — 同 `channel.rs`（三类原因码，`StateReason` 按 design 2.2.2.1 的 14 个值落地）。
4. **2.1 → 2.2 → 2.3** — 同 `channel.rs`（`legal_targets` → `transition` → `ChannelStateError`）。
5. **3.1~3.4** — `crates/domain/tests/channel_state_table.rs`（新建），跑 `cargo test -p meeting-copilot-domain --test channel_state_table`。

## 仍需确认项（不阻塞开工，按 fail-closed 默认执行）

| 编号 | 事项 | 本清单执行口径 |
|---|---|---|
| X1 | design 2.2.2.1 称 `StateReason` "15 值"，但枚举清单与 SPEC 6.1-4 均为 14 个成因 | 按 **14 值** 落地；若后续确认第 15 值（如"探测中"专用原因），只追加变体、不改既有语义 |
| O1 | `advance_to` 的外部装配点（R9） | 本清单只要求 application 暴露 `advance_to` 并将默认周期定为 **100 ms**；真实调度装配属 B2.26/Tauri 壳，未装配则自动回退不会发生，**不得**以测试通过掩盖 |
| O2 | 校验与回补共用 10s 预算、缺口优先 | 按 design D5/R10 默认执行（共用预算、缺口优先） |
| O3 | 初选切换记录时点（D7/R12） | 按"首次 `None→字幕/音频` 时写 `SessionInitial`"，两通道均 `Probing` 时不写 |
| O4 | `Session.channel` 与 `binding.preference` 双字段（R18） | 仲裁唯一取 `Session.channel`；不一致只计 `PreferenceDivergence`，不自动改写（收敛属 B2.26） |
| O5 | AC-33 "网络建连 0" 的判定方式 | 采用"依赖闭包静态检查 + `NetworkProbe` 计数"组合；CI 运行期抓包暂不引入 |
| X2 | AC-35 扫描范围是否包含三份规格文档 | 排除 `docs/sdd/*.md` 与 `spec.md`/`design.md`/`tasks.md`（它们本身在描述门禁与禁令原文），只扫 README/`docs/progress`/新增源码/提交信息 |