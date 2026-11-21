# B2 采集接入 · 当前状态

> 本文件只记录离线可判定的实现进度；不代表 B2 G2 通过。
> 规格/设计/任务：`.codeartsdoer/specs/capture_channel_state/`（本地 SDD 工作区，不入库）。

## B2.23 通道仲裁与状态机（离线部分）

### 已完成（2026-11-21）

**domain**

- `crates/domain/src/model/channel.rs`
  - `ChannelKind` / `ChannelState` / `ChannelKey`
  - `StateReason`（14）、`SwitchReason`（9）、`GapReason`（2）
  - `legal_targets` + `transition`（InternalAdjustment > NoOp > Migrated > Rejected）
- `crates/domain/src/vo/time.rs`：`MonoMillis`
- `crates/domain/src/port/capture_clock.rs`：`CaptureClock`
- `crates/domain/tests/channel_state_table.rs`：6 个穷举用例

**application**

- `capture/types.rs`：11 类 `ObservationPayload`、状态/切换/缺口/ASR DTO、`CapturePolicy`
- `capture/channel_state.rs`：实例注册表、6 键隔离、证据映射、阈值计数器、审计拒绝
- `capture/channel_arbiter.rs`：16 行决策表、5s 回退/10s 切回纯函数、SourceHealth 映射
- `capture/switch_log.rs`：切换记录、初选语义、写失败不回滚、同毫秒合并纯函数
- `capture/capture_budget.rs`：ASR 10s 预算、60s/3 次/120s 抖动抑制
- `capture/service.rs`：最小服务门面（attach/detach/observe/advance_to/set_preference/snapshot）
- `capture/capture_ports.rs`：gap/status/asr 端口与 Noop 实现

### 验证

- `cargo test -p meeting-copilot-application`：**48 passed / 0 failed**
  > 已过期，以 b2-offline-closure 实测为准（399 passed / 0 failed / 8 ignored）。
  - lib 29
  - `capture_channel_state` 10
  - `capture_service` 5
  - `capture_clock` 4
- clippy（application，all targets）：0
- fmt：0
- 既有 workspace 265 测试基线未回退（本轮未重跑全量）
  > 已过期，以 b2-offline-closure 实测为准（399 passed / 0 failed）。

### 未完成

- 阶段 C：双通道失效保护的完整缺口合并与“要点不丢”承接
  > 已过期：阶段 C 已在 b2-offline-closure 完成并判定。
- 阶段 D：状态推送订阅、p99 ≤5s 的离线测量、快照强一致
  > 已过期：阶段 D 已在 b2-offline-closure 完成并判定。
- 阶段 E：替身装配、契约冻结、G2 扫描脚本
  > 已过期：阶段 E 已在 b2-offline-closure 完成并判定。
- 阶段 F：总门禁（≥320）
  > 已过期，阈值收紧为 ≥360（实测 399）。
- 同毫秒合并尚未接入真实写入路径
- `advance_to` 仅实现 5s 回退子集

## 明确不可宣称

- 本轮通过 ≠ B2 G2 通过
- G2：Windows 单平台上腾讯/飞书/Zoom 各自 30 分钟稳定运行；mac/Linux 不适用