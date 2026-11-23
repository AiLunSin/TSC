# 关于 Meeting Copilot（会议副驾）

Meeting Copilot 是一个 Windows 桌面 AI 助手：监听会议对话，识别值得解释的关键词，经门控判定后主动提示，并支持在本地查看关键词详情。仓库名为 `TSC`。

本项目按“先契约、后真机”的方式推进：先把离线链路、门控规则、隐私边界与验收口径做扎实，再接入真实采集与真实模型端点。

## 当前状态（2026-11-23）

- **仓库**：已公开（GitHub），双许可 MIT OR Apache-2.0。
- **CI**：8/8 job 全绿（web、rust×3、test-name-inventory、feature-coverage、compliance、sqlcipher）。
- **测试基线**：877 passed / 0 failed / 10 ignored；`fmt --check`、`clippy -D warnings`、分层/密钥/许可证门禁通过。
- **历史清理**：历史提交中的本机绝对路径已替换为 `<USER_HOME>` / `<REPO>`；同日完成工作区整理，移除过期 SBOM 中间产物与临时日志，保留可重建的 `target/`、`models/`、`node_modules/` 与 ASR 模型。
- **能力边界**：离线契约、门控与设置/卡片前端已闭环；系统音频实时转写链路（WASAPI → vosk → jieba → 卡片）已接通；UIA 字幕/OCR、卡片渲染与投屏遮蔽真机结论、真实检索端点仍未交付。

## 已交付

- B0 工程基座与合规前置（Windows 验证）。
- B1 LLM 网关技术验证：结构化任务、降级链路、离线基线。
- B2 离线闭环与双通道合并：状态机、仲裁、缺口、43 项 AC。
- B3 理解层离线内核：片段准入、滚动摘要、要点抽取、去重。
- B4 门控与关键词卡片：G1~G5 门控、38/39/51 契约与视图模型，AC-B4-01~104 离线通过。
- P-UI 设置向导与关键词卡片前端：14 个 IPC 命令（含打开日志、会议检测、采集状态、文本出卡、实时转写）、Windows 凭据存储、原子 4 键配置、卡片组件，AC-B4-111~130 离线通过；设置页真机第 1~6、9、10 项通过。
- U-06 回放契约与驱动用例（AC-B4-95~104）。
- 实时转写链路：WASAPI 共享模式环回 → 16 kHz 单声道 → vosk 流式识别 → `jieba-rs` 关键词 → 关键词卡片；另保留“粘贴会议文本 → 生成卡片”手动入口。

## 未交付

- **U-05 产品化接入**：卡片渲染（第 7 项）与投屏/紧急遮蔽（第 8 项）缺真实样本未判定，整体保持 `open`。
- **U-07**：负反馈、缓存/outbox、按需补全、审计存储。
- **U-08**：真实检索源、真实 LLM 冷却、真机性能与误触发率；真实 LLM 采样子项已取证。
- **平台层**：`platform-win` 的 WASAPI 环回已实现；UIA 字幕与 OCR 仍为接口占位；`platform-mac` 不适用，仅保留占位。
- **口径限制**：38/39/51 只交付契约与视图，不宣称“产品级可点击卡片”或真机可用。

## 技术栈

- 桌面：Rust + Tauri 2（`app/src-tauri`，14 个 IPC 命令）
- 前端：React 19 + TypeScript + Vite（`app/`）
- 核心分层：`crates/domain`、`crates/application`、`crates/infrastructure`、`crates/platform-win`（含 WASAPI 环回）、`crates/ui-bridge`
- 存储：默认 SQLite + 业务载荷字段级 AES-256-GCM；可选 `sqlcipher` 页级加密（FTS 索引需该模式才能覆盖），凭据写入 Windows 凭据管理器

## 隐私原则

- 凭据不入库、不进日志、不进审计、不进回放。
- 日志仅允许白名单字段；原文、完整地址、查询全文不落日志。
- 数据库打开前必须提供 OS 密钥库引用与实际派生密钥，禁止明文口令，也不再允许无密钥打开文件库。
- 不内置 AGPL/GPL 组件；检索源仅通过开放 SPI 接入。

## 更多信息

- 能力与验收口径以 [README](../README.md) 与 `docs/progress/` 为准。
- 需求规格：[spec.md](sdd/spec.md)；技术方案：[design.md](sdd/design.md)；任务清单：[tasks.md](sdd/tasks.md)。
- 未完成事项汇总：[open-items.md](progress/open-items.md)。
- 许可证：[MIT](../LICENSE-MIT) / [Apache-2.0](../LICENSE-APACHE)。
