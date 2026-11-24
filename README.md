# Meeting Copilot（会议副驾）

监听会议对话、识别值得解释的关键词、经门控判定后主动提示，并可本地查看关键词详情的桌面 AI 助手。

> 仓库名 `TSC`；产品名 **Meeting Copilot（会议副驾）**。

[![CI](https://github.com/AiLunSin/TSC/actions/workflows/ci.yml/badge.svg)](https://github.com/AiLunSin/TSC/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#许可证)
![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6)
![Rust 1.89+](https://img.shields.io/badge/rust-1.89%2B-000000)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB)
![React 19](https://img.shields.io/badge/React-19-61DAFB)

当前定位：**离线契约、门控与设置/卡片前端已闭环；系统音频实时转写链路已接通；检索源接入（U-09 A~F）已实现，真实端点验收待进行；设置页真机第 1~6、9、10 项已通过，UIA 字幕适配器已接线但腾讯会议窗口无可读文本、OCR 已实现（仅前台会议窗口底部区域），卡片渲染/投屏遮蔽真机结论仍未交付**。请以“当前进度”一节为准，不要把规划能力当作已实现能力。

仓库状态（2026-11-24）：**已公开**；双许可 MIT OR Apache-2.0；CI 8/8 全绿（2026-11-23：web、rust×3、test-name-inventory、feature-coverage、compliance、sqlcipher）；历史提交中的本机绝对路径已清理。2026-11-23 完成工作区整理：移除过期 per-crate SBOM 中间产物、CI 瞬时 SBOM、空日志与重复模型压缩包；`target/`、`models/`、`node_modules/` 与 ASR 模型按保守策略保留。2026-11-24 完成检索源接入 A~F 与归档保留期确认（方案 B，在线 180 天 + 归档 5 年，U-07-52 关闭）。

## 当前进度

| 批次 | 状态 |
|---|---|
| B0 工程基座与合规前置 | 完成（Windows 验证） |
| B1 LLM 网关技术验证 | 完成：结构化任务、降级链路、离线基线 |
| B2 离线闭环与双通道合并 | 完成：状态机、仲裁、缺口、43 项 AC |
| B3 理解层（离线内核） | 完成：片段准入、滚动摘要、要点抽取、去重 |
| B4 门控与关键词卡片 | **契约闭环**：AC-B4-01~104 离线通过（AC-B4-45 为 macOS 不适用）；38/39/51 只交付契约与视图模型 |
| P-UI 设置向导 + 关键词卡片前端 | **离线落地 + 设置页真机 + 会议检测 + 实时出卡**：AC-B4-111~130 离线通过；14 命令（含版本、打开日志、会议检测、采集状态、文本出卡、实时转写）、3+3 端口、Windows 凭据存储、原子 4 键配置、卡片组件已接入；设置页真机 1~6/9/10 通过，卡片渲染与投屏遮蔽未判定（U-05 `open`） |
| CI 门禁加固 | 完成：B3/B4/P-UI 静态门禁、许可证、SBOM、密钥扫描 |
| U-09 检索源接入 | **A~F 实现完成（2026-11-24）**：默认零源、`providers.json` 校验、注册表、健康分类、HTTPS JSON 适配器、按需 runner；真实端点 H-34-G 与 200 条样本待验收 |
| 仓库发布 | 完成（2026-11-23）：公开、双许可、CI 8/8 全绿、历史路径清理 |

最近一次全量基线（2026-11-24 实测）：**996 passed / 0 failed / 12 ignored**（含 P-UI 桌面层 **43 例**、平台层 **27 例**另 1 个 ignored 冒烟；两者均可单列运行）；`fmt --check`、`clippy -D warnings`、分层、密钥、许可证门禁通过。

U-06 13.3 回放本体已交付：`gate/replay.rs` 7 项封闭契约 + `gate_replay_driver` 22 用例；AC-B4-95~104 `passed`。

仍未完成：

- **U-05 产品化接入**：设置向导与卡片前端已离线落地（AC-B4-111~130）；设置页真机第 1~6、9、10 项通过（含联网 E2E）；卡片渲染（7）与投屏/紧急遮蔽（8）缺真实样本未判定，**U-05 保持 `open`**。
- **U-07**：负反馈加权、缓存/outbox 已实现；按需补全机制完成，真实 200 样本未采集（U-07-43 `open`）；审计哈希链与 5 年归档已关闭（U-07-52）。
- **U-08**：真实检索源（由 U-09 承接，待 H-34-G）、真实 LLM 冷却、真机性能与误触发率；12 个 `#[ignore]` 用例保持不跑（含 H-34-G 两个手工入口）。
- **平台层**：`platform-win` 已提供 Windows 凭据管理器（`secure_store`）、窗口 + 进程映像名会议检测（`meeting`）与 WASAPI 共享模式环回采集（`wasapi`，`AudioSource` 实现，含混音/单声道解码单测）；UIA 字幕通道已实现（窗口探测、500ms 差量泵、`SubtitleSource` 适配、文本直连出卡、采集状态投影），实测腾讯会议/飞书窗口 UIA 元素数为 0、真实字幕文本未取得，按降级状态展示；OCR 已实现（`Windows.Media.Ocr`，仅当前台会议窗口客户区底部 40% 区域；实测可识别中英文，未在真实会议字幕上验证；默认关闭，`OCR_FALLBACK=1` 时才作为 UIA 回退，避免与开源 ASR 并行重复出卡）；`platform-mac` 不适用（保留占位）。
- **产品能力口径**：38/39/51 只交付契约与视图；前端已只读消费 DTO，但仍不宣称“产品级可点击卡片”或真机可用。

## 目录结构

```
crates/
  domain/            领域模型、值对象、端口 trait（禁止依赖实现）
  application/       应用编排：采集、治理、理解层、门控、卡片、引用对齐、设置端口
  infrastructure/    存储、加密、审计链、度量、日志、LLM 传输、检索源装配
  platform-win/      Windows 适配（凭据管理、会议检测、WASAPI 环回已实现；UIA 字幕已实现但暂无可用文本；OCR 已实现，仅前台窗口底带）
  platform-mac/      macOS 适配（不适用，保留占位）
  ui-bridge/         IPC DTO、关键词卡片视图、设置向导视图
app/                 React 19 + TypeScript + Vite 前端
app/src-tauri/       Tauri 2 桌面壳（14 个 IPC 命令与装配）
docs/sdd/            需求规格、技术方案、任务清单
docs/progress/       各批次状态与验收记录
docs/baseline/       LLM 与 ASR 基线
docs/compliance/     许可证复核、SBOM
scripts/             静态门禁、SBOM、用例名差集
```

## 快速开始

### 环境（Windows）

本机 rustup shim 可能不可用；直接使用工具链内可执行文件，或前置 PATH：

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
```

### Rust

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

统计被忽略的真实端点用例：

```bash
cargo test --workspace -- --list --ignored
```

### 前端

```bash
cd app
npm ci
npm run lint
npm run typecheck
npm run check:bindings
npm run build
```

### 桌面壳

```bash
cd app
npm run dev          # 仅启动前端
cargo build -p meeting-copilot-desktop --features llm-https
```

交互式启动（必须是桌面会话）：

```
target\debug\meeting-copilot-desktop.exe
```

前端改动后需 `cargo clean -p meeting-copilot-desktop` 再构建，否则 Tauri 会复用旧 `dist` 资源；debug 构建已固定为单窗口（无控制台）。

## 门禁与 CI

本地静态门禁：

```bash
bash scripts/check-layering.sh
bash scripts/scan-secrets.sh
bash scripts/scan-secrets-docs.sh --all-md
bash scripts/check-capture-offline.sh --all
bash scripts/check-mac-scope.sh --rule=na
bash scripts/check-ui-integration.sh
bash scripts/check-g2-claims.sh
cargo deny check licenses bans sources advisories
```

B3 六条与 B4 七条：

```bash
for r in source no-truncate zero-transcript fallback-untouched new-targets exports; do
  bash scripts/check-b3-gates.sh --rule="$r" || exit 1
done
for r in source budget record outbound zero-transcript targets doc-plan; do
  bash scripts/check-b4-gates.sh --rule="$r" || exit 1
done
bash scripts/check-b4-gates.sh --self-test   # 预期退 1（反例检出）
```

P-UI 六规则：

```bash
for r in credential-redline settings-ux card-render body-hidden frontend-offline zero-transcript; do
  bash scripts/check-ui-integration.sh --rule="$r" || exit 1
done
```

用例名差集：

```bash
CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe" \
  bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
```

CI 任务：`web`（lint/typecheck/bindings/build）、`rust`（Windows 与两个 Darwin 交叉编译）、`test-name-inventory`、`feature-coverage`、`compliance`（licenses/SBOM/secrets/layering）、`sqlcipher`。`compliance` 与本地使用同一 cargo-deny 版本与同一命令；许可证允许清单见 `deny.toml`。

## LLM 配置入口

- 默认关闭；端点、模型、凭据只在运行期经环境变量注入，**不写入仓库**。
- 相关变量统一 `LLM_*` 命名；调用上限与冷却由网关侧约束。
- 设置向导契约位于 `crates/ui-bridge/src/settings/`，装配与命令位于 `app/src-tauri/src/ipc/`（14 命令，含 `ipc_version`、`open_log`、`meeting_status`、`capture_status`、`ingest_text`、`start_asr`/`stop_asr`/`asr_status`），页面位于 `app/src/settings/`；提供 API Key 输入、预设模型、固定服务地址、一键测试、保存/清除与"打开日志"。
- 采集状态栏由 `capture_status` 驱动：系统音频通道读取 WASAPI 环回健康度（可用即"就绪"），本地 ASR 读取实时管线运行状态（运行中即"就绪"），字幕文本通道尚未实现，如实显示"未启用"。
- 会议检测分两步：先按进程名命中腾讯会议（`wemeetapp.exe`/`TencentMeeting.exe`）、微信（`WeChat.exe`/`Weixin.exe`）、飞书（`Feishu.exe`/`Lark.exe`）、钉钉（`DingTalk.exe`），再按可见窗口过滤——常驻客户端（微信/钉钉）要求窗口标题含会议关键词（会议/通话/视频/语音/直播/共享/Meet/Call/Meeting/Video/Voice），腾讯会议/飞书存在可见窗口即判定；每 2 秒轮询，命中后顶部显示应用名并开始计时。只读取进程名与窗口标题，不读取会议内容。
- 凭据经 `crates/platform-win/src/secure_store.rs` 写入 Windows 凭据管理器（唯一目标名）；非敏感配置为 `settings.json` **恰 4 键**、原子写、损坏回默认；Key 固定掩码 `masked`、不落盘明文、不进日志/审计/回放。
- 探针经 `ProbeExecutor` 端口执行；**生产探针默认关闭**，仅 `--features llm-https` 显式开启；设置页真机第 1~6、9、10 项已通过，卡片渲染与投屏遮蔽仍未判定，**不宣称整体界面已验收**。

## 检索源接入

- 默认零源：不内置任何检索源；未配置时按需补全直接返回 `NoSource`，不外发、不出卡。
- 配置仅在运行期注入：`SEARCH_PROVIDERS_FILE` 指向本地 `providers.json`（不入库），`authRef` 指向保存令牌的环境变量名；端点必须是 HTTPS，外发仅允许配置中的主机。
- 手动验收入口（默认 `#[ignore]`）：

```bash
cargo test -p meeting-copilot-infrastructure --features llm-https --test search_live -- --ignored
```

  需同时设置 `SEARCH_PROVIDERS_FILE`、`SEARCH_QUERIES_FILE` 与 `<authRef>`；可选 `SEARCH_REPORT_PATH` 输出只含计数与时延的 JSON。`smoke_three_rounds` 对应 H-34-G 的 3 次验收，`collect_200_real_samples` 对应 200 条真实样本。
- 报告与日志不记录查询原文、URL 查询串、页面正文或标题结论。

## 关键词卡片

链路：`B3 片段/要点 → B4 门控（G1~G5）→ 候选派生 → 38 卡片数据 → 39 引用对齐 → 51 视图/点击`。

- 实时入口：卡片区"开始实时转写"——WASAPI 环回 → 16 kHz 单声道 → vosk 流式识别 → `jieba-rs` 关键词 → B4 门控 → 卡片；端到端延迟约 1~2 秒。运行期从 `resource_dir/asr` 读取脚本与模型，缺失时回退到 `app/src-tauri/asr/`、再回退 `.tmp/asr/`；仍依赖 PATH 中的 Python 3 与 `vosk`。
- 打包：`asr/stream_asr.py` 随仓库入库并默认打包；vosk 模型（约 52MB）不入库——发布前复制到 `app/src-tauri/asr/vosk-model-small-cn-0.22/`，再叠加 `app/src-tauri/tauri.asr.conf.json`（Tauri CLI 参数 `--config tauri.asr.conf.json`）打包。基础配置不含模型，因此 CI 与 `cargo clippy`/`cargo test` 无需模型即可编译。
- 手动入口："粘贴会议文本 → 生成卡片"，复用同一 `ingest_text` 链路。
- 卡片列表每 1 秒自动刷新；来源为 `UserGenerated`（本地片段），因此可点击，不伪造外部检索来源。

- 仅 B4 `Allow` 才允许出卡/可点击；任一 Deny → 0 卡。手动出卡与用户主动启动的实时转写按 `OnDemand` 显式授权执行 G1~G5，卡片引用真实 Allow trace 与停止级。
- 38 只产 11 项卡片数据；39 只产 `citation_alignment`（`Passed`/`Failed`/`Unknown`，`Unknown` 唯一投影为 `Unverified`）；51 只产视图（10 项）与点击详情（5 项）。
- `Failed`/`Unverified` 仍可展示但不可点；投屏/紧急遮蔽隐藏正文；撤回不可复活。
- 前端 `app/src/cards/KeywordCard.tsx` / `CardDetail.tsx` 只读消费 10/5 项 DTO；仅 `enabled` 绑点击，禁用卡 `aria-disabled`，隐藏态只留条数角标，详情内联不抢焦点。
- 点击只展开本地详情，不发起检索、不调 LLM、不外发、不写审计。

## 隐私与凭据

- 凭据不入库、不进日志、不进审计、不进回放；错误信息不含 Key 片段。
- 日志仅允许白名单字段；原文、完整地址、查询全文不得落日志。
- 桌面日志写入 `%LOCALAPPDATA%\com.meetingcopilot.desktop\logs\desktop.log`（超过 5 MiB 自动重置），设置页"打开日志"在资源管理器中定位；日志不发往网络。
- 数据库打开前必须提供 OS 密钥库引用与实际派生密钥，禁止明文口令，也不再允许无密钥打开文件库。
- 默认后端对 `sessions`/`utterances`/`insights`/`summaries`/`cards` 的业务载荷执行字段级 AES-256-GCM；FTS5 原文索引需 `sqlcipher` 页级加密才能覆盖，`query_cache`/`proactive_outbox` 的 JSON 字段尚未接入字段级加密。
- 审计只追加，不随会话删除；在线保留 180 天，归档保留 5 年（项目自用设置），保留期内不自动销毁。
- 许可证：不内置 AGPL/GPL 组件；检索源仅通过开放 SPI 接入。

## 文档索引

| 文档 | 说明 |
|---|---|
| `docs/ABOUT.md` | 项目简介、当前状态与能力边界 |
| `docs/sdd/spec.md` | 需求规格与验收标准 |
| `docs/sdd/design.md` | 技术方案与追溯矩阵 |
| `docs/sdd/tasks.md` | 分批次任务与门禁 |
| `docs/progress/b0-status.md` | 工程基座状态 |
| `docs/progress/b1-status.md` | LLM 技术验证与基线 |
| `docs/progress/b2-status.md` | 通道状态机 |
| `docs/progress/b2-offline-closure.md` | 离线闭环 43 项 AC |
| `docs/progress/b2-24-merge.md` | 双通道合并验收 |
| `docs/progress/b3-status.md` | 理解层进度 |
| `docs/progress/b4-status.md` | 门控/卡片/设置向导与 AC 三态（§29 为 P-UI 收口） |
| `docs/progress/handoff-2026-11-22.md` | 批次交接说明 |
| `docs/progress/handoff-2026-11-22-pui.md` | P-UI 落地交接（C1~C7） |
| `docs/progress/open-items.md` | 未完成事项总表与逐项关闭操作手册 |
| `docs/progress/ci-readiness.md` | CI 预检与缺口 |
| `docs/baseline/asr.md` | ASR 基线与模型说明 |
| `docs/sdd/tasks-b2-offline-closure.md` | B2 离线闭环任务清单 |
| `docs/compliance/license-review.md` | 许可证边界 |
| `docs/compliance/retention-review.md` | 归档保留期复核与法务意见 |

## 贡献

1. 修改后本地跑完 Rust、前端与静态门禁。
2. 不提交凭据、真实端点、模型名、请求标识或原文样例。
3. 新增测试 target 时同步 `scripts/test-name-inventory.sh` 的 `NEW_TARGETS`。
4. 能力状态必须与 `docs/progress/b4-status.md` 的 AC 三态一致，禁止把未完成态写成已交付。

## 许可证

本项目采用 Rust 生态常见的双许可，任选其一：

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

除非另有明确说明，任何有意提交以纳入本项目的贡献，均按上述双许可授权。