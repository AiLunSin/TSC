# 关于 Meeting Copilot（会议副驾）

Meeting Copilot 是一个 Windows 桌面 AI 助手：监听会议对话，识别值得解释的关键词，经门控判定后主动提示，并支持在本地查看关键词详情。仓库名为 `TSC`。

本项目按“先契约、后真机”的方式推进：先把离线链路、门控规则、隐私边界与验收口径做扎实，再接入真实采集与真实模型端点。

## 当前状态（2026-11-24）

- **仓库**：已公开（GitHub），双许可 MIT OR Apache-2.0。
- **CI**：8/8 job 全绿（web、rust×3、test-name-inventory、feature-coverage、compliance、sqlcipher）。
- **测试基线**：1000 passed / 0 failed / 12 ignored（P-UI 桌面层 47 例、平台层 27 例另 1 个 ignored 冒烟，均可单列运行）；`fmt --check`、`clippy -D warnings`、分层/密钥/许可证门禁通过。
- **历史清理**：历史提交中的本机绝对路径已替换为 `<USER_HOME>` / `<REPO>`；同日完成工作区整理，移除过期 SBOM 中间产物与临时日志，保留可重建的 `target/`、`models/`、`node_modules/` 与 ASR 模型。
- **能力边界**：离线契约、门控与设置/卡片前端已闭环；系统音频实时转写链路（WASAPI → vosk → 转写文字，按需出卡）已接通；选中查义、联网查证（博查/Tavily 预设，需自备 Key）与大模型辅助回复已接通；UIA 字幕与 OCR 已实现但真实会议字幕文本未取得；卡片渲染与投屏/紧急遮蔽的真机结论、联网检索的真实 Key 验收仍未交付。

## 已交付

- B0 工程基座与合规前置（Windows 验证）。
- B1 LLM 网关技术验证：结构化任务、降级链路、离线基线。
- B2 离线闭环与双通道合并：状态机、仲裁、缺口、43 项 AC。
- B3 理解层离线内核：片段准入、滚动摘要、要点抽取、去重。
- B4 门控与关键词卡片：G1~G5 门控、38/39/51 契约与视图模型，AC-B4-01~104 离线通过。
- P-UI 设置向导与关键词卡片前端：23 个 IPC 命令（含打开日志、会议检测、采集状态、文本出卡、实时转写、转写全文、选中释义、释义出卡、自动出卡开关、清空卡片、检索配置、联网查询、辅助回复）、双系统凭据目标（模型 Key 与检索 Key 分开）、原子 4 键配置、卡片组件，AC-B4-111~130 离线通过；设置页真机第 1~6、9、10 项通过。
- U-06 回放契约与驱动用例（AC-B4-95~104）。
- 实时转写链路：WASAPI 共享模式环回 → 16 kHz 单声道 → vosk 流式识别 → 完整转写文字（单会话内存、不落库）；卡片默认按需生成（选中文字查意思后出卡），自动出卡需在界面显式勾选；另保留“粘贴会议文本 → 生成卡片”手动入口。主栏按功能分成四块（关键词卡片 / 联网查资料〔默认收起〕/ 完整转写 / 会议文本），窗口窄于 1180px 时隐藏长期空白的“要点时间轴”。
- UIA 字幕通道与 OCR 兜底：窗口文字探测、500ms 差量泵、`SubtitleSource` 适配；OCR 仅前台会议窗口客户区底带，默认关闭（`OCR_FALLBACK=1` 时作为回退）。
- U-08 真实 LLM 采样：2026-11-24 用一次性临时凭据复测 9 个 live 用例与 4 任务 × 30 样本基线，`leak_hits=0`，结果见 `docs/progress/b4-status.md §34`。

## 未交付

- **U-05 产品化接入**：卡片渲染（第 7 项）与投屏/紧急遮蔽（第 8 项）缺真实样本未判定，整体保持 `open`；第 8 项的开发期注入入口已就绪（`PRIVACY_SCREEN_SHARE` / `PRIVACY_EMERGENCY_MASKED`）。
- **U-07**：负反馈、缓存/outbox、按需补全、审计存储。
- **U-08**：真实检索源、真实 LLM 冷却、真机性能与误触发率；真实 LLM 采样子项已取证。
- **平台层**：`platform-win` 的 WASAPI 环回、UIA 字幕与 OCR 兜底均已实现（UIA 实测腾讯会议/飞书窗口可读元素为 0，真实字幕文本未取得）；`platform-mac` 不适用，仅保留占位。
- **口径限制**：38/39/51 只交付契约与视图，不宣称“产品级可点击卡片”或真机可用。

## 技术栈

- 桌面：Rust + Tauri 2（`app/src-tauri`，23 个 IPC 命令）
- 前端：React 19 + TypeScript + Vite（`app/`）
- 核心分层：`crates/domain`、`crates/application`、`crates/infrastructure`、`crates/platform-win`（含 WASAPI 环回）、`crates/ui-bridge`
- 存储：默认 SQLite + 业务载荷字段级 AES-256-GCM；可选 `sqlcipher` 页级加密（FTS 索引需该模式才能覆盖），凭据写入 Windows 凭据管理器

## 隐私原则

- 凭据不入库、不进日志、不进审计、不进回放。
- 日志仅允许白名单字段；原文、完整地址、查询全文不落日志。
- 数据库打开前必须提供 OS 密钥库引用与实际派生密钥，禁止明文口令，也不再允许无密钥打开文件库。
- 投屏与紧急遮蔽的真机取证只经开发期环境变量注入（`PRIVACY_SCREEN_SHARE` / `PRIVACY_EMERGENCY_MASKED`，只认 `"1"`、默认关、启动时读取一次），该入口本身不新增 IPC 命令，发布版不得开启。
- 不内置 AGPL/GPL 组件；检索源仅通过开放 SPI 接入。

## 更多信息

- 能力与验收口径以 [README](../README.md) 与 `docs/progress/` 为准。
- 需求规格：[spec.md](sdd/spec.md)；技术方案：[design.md](sdd/design.md)；任务清单：[tasks.md](sdd/tasks.md)。
- 未完成事项汇总：[open-items.md](progress/open-items.md)。
- 许可证：[MIT](../LICENSE-MIT) / [Apache-2.0](../LICENSE-APACHE)。
