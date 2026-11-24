# Meeting Copilot（会议副驾）

监听会议对话、识别值得解释的关键词、经门控判定后主动提示，并可本地查看关键词详情的桌面 AI 助手。

> 仓库名 `TSC`；产品名 **Meeting Copilot（会议副驾）**。

[![CI](https://github.com/AiLunSin/TSC/actions/workflows/ci.yml/badge.svg)](https://github.com/AiLunSin/TSC/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#许可证)
![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6)
![Rust 1.89+](https://img.shields.io/badge/rust-1.89%2B-000000)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB)
![React 19](https://img.shields.io/badge/React-19-61DAFB)

当前定位：**离线契约、门控与设置/卡片前端已闭环；系统音频实时转写已接通，真机可出完整转写文本并支持选中查义、联网查证、按需出卡与大模型辅助回复，主栏已按“卡片 / 联网配置（默认收起）/ 转写 / 手动粘贴”分区；通用检索源适配器（U-09 A~F）已实现但未接入桌面卡片链路，桌面联网查走独立的两家预设（博查/Tavily），真实 Key 验收待进行；按需补全的 200 条真实检索样本已于 2026-11-24 用零账号 D 路径取证达标，三轮独立运行均 200/200、满足 ≥3 时段（见下文「检索源接入」与 `b4-status.md §50`）；桌面 exe 已于 21:57 重建，含停止态状态位文案修正；设置页真机第 1~6、9、10 项已通过，U-05 已于 2026-11-25 收口为 `closed`（卡片渲染第 7 项 4/4、投屏/紧急遮蔽第 8 项 3/3，见 `b4-status.md §53`）；UIA 字幕适配器已接线但腾讯会议窗口无可读文本、OCR 已实现（仅前台会议窗口底部区域）**。请以“当前进度”一节为准，不要把规划能力当作已实现能力。

仓库状态（2026-11-24）：**已公开**；双许可 MIT OR Apache-2.0；CI 8/8 全绿（2026-11-23：web、rust×3、test-name-inventory、feature-coverage、compliance、sqlcipher；**历史结论**：后续 run 45439657273 的 `x86_64-apple-darwin` job 因回环 mock 用例断言失败，已于 2026-11-25 修复测试脚手架，推送后 run 45447235508 复跑 **8/8 全绿**（含两个 darwin job），见 `b4-status.md §55.5`）；历史提交中的本机绝对路径已清理。2026-11-23 完成工作区整理：移除过期 per-crate SBOM 中间产物、CI 瞬时 SBOM、空日志与重复模型压缩包；`target/`、`models/`、`node_modules/` 与 ASR 模型按保守策略保留。2026-11-24 完成检索源接入 A~F 与归档保留期确认（方案 B，在线 180 天 + 归档 5 年，U-07-52 关闭）；同日用一次性临时凭据复测真实 LLM 采样——9 个 live 用例通过、120 样本基线落表（见 `docs/progress/b4-status.md §34`），临时凭据在验收后作废。同日新增零账号 D 路径（本机桥把必应 RSS 转成适配器 JSON + 免账号 quick tunnel），U-07-43 三轮真实 200 样本均达标并关闭（p95 4461~4759 ms、p99 ≤6789 ms、超时率 ≤0.61%，三时段 3/3 满足 G-43-C DoD，详见 `b4-status.md §50`）。21:48 查明本机 Rust 代理异常根因并把工具链 `bin` 前置到用户 PATH，21:57 重建桌面 exe；`b4-status.md §51` 记录收尾修复清单与残留限制（`rustup` 自身仍是坏代理，需 `rustup-init` 重装）。

## 当前进度

| 批次 | 状态 |
|---|---|
| B0 工程基座与合规前置 | 完成（Windows 验证） |
| B1 LLM 网关技术验证 | 完成：结构化任务、降级链路、离线基线 |
| B2 离线闭环与双通道合并 | 完成：状态机、仲裁、缺口、43 项 AC |
| B3 理解层（离线内核） | 完成：片段准入、滚动摘要、要点抽取、去重 |
| B4 门控与关键词卡片 | **契约闭环**：AC-B4-01~104 离线通过（AC-B4-45 为 macOS 不适用）；38/39/51 只交付契约与视图模型 |
| P-UI 设置向导 + 关键词卡片前端 | **离线落地 + 设置页真机 + 会议检测 + 实时转写 + 选中查义 + 联网查证 + 辅助回复**：AC-B4-111~130 离线通过；26 命令（含版本、打开日志、会议检测、采集状态、文本出卡、实时转写、转写全文、选中释义、释义出卡、自动出卡开关、清空卡片、检索配置/清 Key、联网查询、辅助回复/窗口设置）、3+3 端口、双系统凭据目标、原子 4 键配置、卡片组件已接入；设置页真机 1~6/9/10 通过；卡片渲染第 7 项 4/4、投屏/紧急遮蔽第 8 项 3/3（2026-11-25 三张真机截图 + 1 格代码判定，U-05 `closed`，见 `b4-status.md §53`） |
| CI 门禁加固 | 完成：B3/B4/P-UI 静态门禁、许可证、SBOM、密钥扫描 |
| U-09 检索源接入 | **A~F 实现完成（2026-11-24）**：默认零源、`providers.json` 校验、注册表、健康分类、HTTPS JSON 适配器、按需 runner；H-34-G 的 3 次手工验收仍待真实端点，200 条真实样本已由 U-07-43 用 D 路径完成（三轮均 200/200） |
| 仓库发布 | 完成（2026-11-23）：公开、双许可、CI 8/8 全绿、历史路径清理；后续 darwin job 失败已修复并复跑 8/8 全绿（2026-11-25，`b4-status.md §55.5`） |

最近一次全量基线（2026-11-24 22:25 复测，工具链修复与本轮脚本改动后数字不变）：**1049 passed / 0 failed / 12 ignored**（含 P-UI 桌面层 **93 例**、平台层 **30 例**另 1 个 ignored 冒烟；两者均可单列运行）；`fmt --check`、`clippy -D warnings`（默认特性与 `llm-https` 两套）、分层、密钥、许可证门禁通过。桌面层由 55 例增至 93 例：实时转写协议解析与半句拼接、中文词间空格合并、摘要按句读收口、自动出卡限流、检索响应解析与来源回退、辅助回复 JSON 解析、提醒窗口切分与越界夹取等；平台层另含 UIA 字幕过滤与 OCR 兜底。

U-06 13.3 回放本体已交付：`gate/replay.rs` 7 项封闭契约 + `gate_replay_driver` 22 用例；AC-B4-95~104 `passed`。

仍未完成（U-05 已于 2026-11-25 关闭，见 `b4-status.md §53`，不再列入）：
- **U-07**：负反馈加权、缓存/outbox 已实现；按需补全已闭环（U-07-43 三轮真实 200 样本均达标：p95 4461~4759 ms、p99 ≤6789 ms、超时率 ≤0.61%，三时段 3/3，见 `docs/progress/b4-status.md §50`）；审计哈希链与 5 年归档已关闭（U-07-52）。**整体仍 `open`**：AUD-03/AUD-04/AUD-06 三项未结，缓存/outbox 的「≤3 秒识别断网、30 秒补齐率 ≥98%」还缺真实断网演练记录。
- **U-08**：真实 LLM 采样子项已关闭（2026-11-24：9 例 live 通过 + 4 任务 × 30 样本基线，见 `docs/progress/b4-status.md §34`）；其余仍 open——检索源适配器已完成 200 条真实出网样本取证（U-07-43 D 路径，三轮均 200/200），但 **H-34-G 的 3 次手工验收仍待真实端点**、真实 LLM 冷却（本次窗口未触发 429，不作结论）、真机性能与误触发率；12 个 `#[ignore]` 用例默认不跑，其中 U-07-43 的两个检索用例已由采样脚本显式 `--ignored` 运行达标、本机 WASAPI 环回冒烟已于 2026-11-25 跑通（1 passed，rc=0），H-34-G 两个手工入口仍未跑。
- **桌面联网查与辅助回复**：链路、DTO、门禁都已落地（`web_search` / `assist_reply`），额度有每小时限额兜底（默认 20 次，可调 0~200），但两项都只有"代码可跑"，没有"效果已验收"——**真实检索 Key 下的中文召回质量未测**（需自备博查/Tavily Key；U-07-43 的 200 条样本是检索源适配器在 D 路径下的链路取证，不等于桌面两家预设的效果验收），**辅助回复在真实会议里的可用度未测**（提醒 5 句 / 上下文 12 句这组数已开放给用户自行调整，但好坏仍需手测）。检索结果只展示地址、不打开浏览器。
- **平台层**：`platform-win` 已提供 Windows 凭据管理器（`secure_store`）、窗口 + 进程映像名会议检测（`meeting`）与 WASAPI 共享模式环回采集（`wasapi`，`AudioSource` 实现，含混音/单声道解码单测）；UIA 字幕通道已实现（窗口探测、500ms 差量泵、`SubtitleSource` 适配、文本直连出卡、采集状态投影），实测腾讯会议/飞书窗口 UIA 元素数为 0、真实字幕文本未取得，按降级状态展示；OCR 已实现（`Windows.Media.Ocr`，仅当前台会议窗口客户区底部 40% 区域；实测可识别中英文，未在真实会议字幕上验证；默认关闭，`OCR_FALLBACK=1` 时才作为 UIA 回退，避免与开源 ASR 并行重复出卡）；`platform-mac` 不适用（保留占位）。
- **产品能力口径**：38/39/51 仍只交付契约与视图模型，不宣称产品级真实会议可用；但 U-05 界面层已按 `b4-status.md §53` 完成真机验收（第 7 项 4/4、第 8 项 3/3）——前端只读消费 DTO，卡片正文/可点、详情内联展开、清空归零、投屏与紧急遮蔽均有结论；其中「禁用卡不可点」一格按代码事实与单测判定，真机不可达。

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
app/src-tauri/       Tauri 2 桌面壳（26 个 IPC 命令与装配）
docs/sdd/            需求规格、技术方案、任务清单
docs/progress/       各批次状态与验收记录
docs/baseline/       LLM 与 ASR 基线
docs/compliance/     许可证复核、SBOM
scripts/             静态门禁、SBOM、用例名差集
scripts/u07-43/      真实检索采样：A/B/C/D 四路径配置、预检、判定、本机桥（D 路径零账号）与 Windows 构建包装
```

## 界面

三栏布局，窗口窄于 1180px 时隐藏右栏，宽度让给中栏：

- **左栏**：采集状态（字幕文本通道 / 系统音频通道 / 本地 ASR）与隐私三态。
- **中栏**：四张独立卡片——**关键词卡片**（转写开关、自动出卡开关、卡片列表、清空）、**联网查资料**（默认收起的折叠块，标题行直接显示"已就绪"或"未配置 · 点开填 Key"）、**完整转写**（转写框 → 选中操作栏 → 释义 / 联网结果 / AI 提醒面板）、**会议文本**（手动粘贴生成卡片）。选中文字后，操作按钮就在转写框正下方，不用往下找。
- **右栏**：要点时间轴（占位，当前恒为"暂无要点"，窄窗口下隐藏）。

## 快速开始

### 环境（Windows）

本机 `~/.cargo/bin` 下的 rustup 代理不可用（16 个 exe 的 SHA256 一致，实际是同一个 rustfmt 二进制，依赖只存在于工具链 `bin` 下的 `rustc_driver-*.dll`）。已查明根因并把工具链 `bin` **前置**到用户级 PATH 持久修复；`rustup` 自身仍是坏代理，`rustup update` 等功能需另跑 `rustup-init` 重装（见 `docs/progress/open-items.md §5`）。新开终端若仍异常，手动前置一次即可：

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$HOME/.cargo/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
```

顺序不能反：只把工具链 `bin` **追加**到末尾时 `cargo` 仍会命中 `~/.cargo/bin` 的伪代理，表现为 `cargo --version` 输出 `rustfmt 1.11.0`、`cargo metadata --no-deps` 报 `Unrecognized option`。

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
- 设置向导契约位于 `crates/ui-bridge/src/settings/`，装配与命令位于 `app/src-tauri/src/ipc/`（26 命令，含 `ipc_version`、`open_log`、`meeting_status`、`capture_status`、`ingest_text`、`clear_cards`、`start_asr`/`stop_asr`/`asr_status`/`set_auto_cards`、`transcript_snapshot`、`explain_selection`、`ingest_explanation`、`search_config`/`search_config_save`/`search_clear_key`/`web_search`、`assist_reply`/`assist_config`/`assist_config_save`），页面位于 `app/src/settings/`；提供 API Key 输入、预设模型、固定服务地址、一键测试、保存/清除与"打开日志"。
- 采集状态栏由 `capture_status` 驱动：系统音频通道读取 WASAPI 环回健康度（可用即"就绪"）；字幕文本通道由 UIA 轮询驱动，检测到会议即自动启动（窗口无可读文本时"降级"、无会议窗口时"未启用"）；本地 ASR 读取实时管线运行状态（运行中即"就绪"）。识别进程异常退出时，`asr_status` 第三项回传可见原因（缺 vosk、模型不可读、无音频设备），不再静默失败。
- UIA 字幕通道只接受"像一句话"的元素（长度、中英文标识符占比、固定控件标签词表三重过滤）；腾讯会议窗口里那些 `DanmuContainer`／`pageContainer`／`最小化` 之类的控件名会被全部丢弃，该通道因此常态为"降级"而不是假装就绪。字幕文本与 ASR 文本**分开存放**：文本位优先展示 ASR，ASR 为空才回退字幕，避免两条通道互相覆盖。
- 会议检测分两步：先按进程名命中腾讯会议（`wemeetapp.exe`/`TencentMeeting.exe`）、微信（`WeChat.exe`/`Weixin.exe`）、飞书（`Feishu.exe`/`Lark.exe`）、钉钉（`DingTalk.exe`），再按可见窗口过滤——常驻客户端（微信/钉钉）要求窗口标题含会议关键词（会议/通话/视频/语音/直播/共享/Meet/Call/Meeting/Video/Voice），腾讯会议/飞书存在可见窗口即判定；每 2 秒轮询，命中后顶部显示应用名并开始计时。只读取进程名与窗口标题，不读取会议内容。
- 凭据经 `crates/platform-win/src/secure_store.rs` 写入 Windows 凭据管理器（唯一目标名）；非敏感配置为 `settings.json` **恰 4 键**、原子写、损坏回默认；Key 固定掩码 `masked`、不落盘明文、不进日志/审计/回放。
- 探针经 `ProbeExecutor` 端口执行；**生产探针默认关闭**，仅 `--features llm-https` 显式开启；设置页真机第 1~6、9、10 项已通过，U-05 界面层已按 `b4-status.md §53` 完成真机验收（第 7 项 4/4、第 8 项 3/3），**仍不宣称产品级真实会议可用**。
- 真实采样入口：`bash scripts/run-llm-live.sh`（需 `LLM_LIVE=1` 与端点、模型、白名单、`LLM_LIVE_CALL_CAP` 齐备）；2026-11-24 复测 9 个 live 用例通过、4 任务 × 30 样本基线落表，结论与时延见 `docs/progress/b4-status.md §34`。基线需 `LLM_LIVE_CALL_CAP ≥ 120`，且记录根目录须先创建，否则脚本以 `record_dir_missing` 退出 2。

## 检索源接入

- 默认零源：不内置任何检索源；未配置时按需补全直接返回 `NoSource`，不外发、不出卡。
- 配置仅在运行期注入：`SEARCH_PROVIDERS_FILE` 指向本地 `providers.json`（不入库），`authRef` 指向保存令牌的环境变量名；端点必须是 HTTPS，外发仅允许配置中的主机。
- 手动验收入口（默认 `#[ignore]`）：

```bash
cargo test -p meeting-copilot-infrastructure --features llm-https --test search_live -- --ignored
```

  需同时设置 `SEARCH_PROVIDERS_FILE`、`SEARCH_QUERIES_FILE` 与 `<authRef>`；可选 `SEARCH_REPORT_PATH` 输出只含计数与时延的 JSON。`smoke_three_rounds` 对应 H-34-G 的 3 次验收，`collect_200_real_samples` 对应 200 条真实样本。**两个 `search_live` 用例都必须带 `--features llm-https`**：缺省特性下 `HttpJsonProvider` 没有真实 TLS 传输，每次检索都会 `Unreachable`；另外三个 `SEARCH_*` 路径必须传**绝对路径**（`cargo test` 的进程 CWD 是 crate 根目录，相对路径会被当成「文件不存在」而静默得到零源）。
- U-07-43 的 200 条采样可一键编排（预检 → 3 次冒烟 → 采样 → 判定）。**A / B / C / D 四条接入路径都已配好**；D 路径不需要任何账号或 Key：

```bash
# D. 零账号、零 Key：本机桥把必应 RSS 转成适配器要求的 JSON，再用免账号 quick tunnel 暴露成公网 HTTPS
bash scripts/u07-43/start-bing-bridge.sh --serve-only   # 建端点并保持（另开终端跑下一行）
bash scripts/run-u07-43-samples.sh
# 或一条命令跑完：bash scripts/u07-43/start-bing-bridge.sh

# A. 自建 / 可用的 SearXNG（需启用 format=json）
bash scripts/u07-43/setup.sh --mode a --endpoint "https://<SearXNG 公网域名>/search"

# B. 已有的博查 / Tavily Key：一键部署转发 Worker 并写好配置
UPSTREAM_KEY='<检索服务 Key>' bash scripts/u07-43/deploy-worker.sh --provider bocha

# C. 自定义 providers.json（模板 scripts/u07-43/providers.example.json）
cp scripts/u07-43/providers.example.json .tmp/u07-43/providers.json
# 改里面的 endpoint / authRef，然后：
bash scripts/u07-43/setup.sh --mode c --providers .tmp/u07-43/providers.json --token '<凭据>'

# A/B/C 最后都是同一条命令（自动加载 .tmp/u07-43/config.env）
bash scripts/run-u07-43-samples.sh
```

  - **D 路径**：`scripts/u07-43/local-bridge.py` 把 `GET ?q=&format=json` 转成必应 RSS 请求并规整为 `{"results":[{"url","title"}]}`；`scripts/u07-43/start-bing-bridge.sh` 启动桥 + `cloudflared` 免账号 quick tunnel（不需要 Cloudflare 账号），注入随机令牌、写 `config.env`、做公网自检后跑采样。启动前会检测 8788 端口，**已有桥实例时直接报错退出**，避免多实例各自起隧道、互相覆盖 `config.env` 造成端点漂移；health 通过后还会确认进程确实是本次启动的桥。必应无结果时返回 502，避免用空列表凑假样本；上游单次抖动在桥内重试 3 次；桥日志只记方法、去参路径、状态码与长度，不记 query 原文。quick tunnel 创建被限流（429）时会等待重试，仍不行才回退 localhost.run（后者域名会轮换，只适合短跑）。首次运行会自动把 `cloudflared` 下载到 `.tmp/u07-43/bin`；依赖 Python 3.8+，回退 localhost.run 时才需要 `ssh`。Windows 上采样经 `scripts/u07-43/run-cargo.ps1` 注入 MSVC 环境并关闭 dev debuginfo，避免 `sqlite3.c` 编译时因内存不足静默失败；该脚本还会在 `Enter-VsDevShell` 前对大小写重复的代理变量去重，避免 INCLUDE 为空导致 `libsqlite3-sys` 静默 exit 2。
  - **B 路径**：`deploy-worker.sh` 用 `scripts/u07-43/wrangler.toml` + `package.json` 部署 `search-proxy.worker.js`，自动注入 `UPSTREAM_KEY` / `SHARED_TOKEN` / `UPSTREAM_PROVIDER`（`SHARED_TOKEN` 缺省时随机生成，不回显）。Worker 把 `GET ?q=&format=json` 转成博查/Tavily 的 POST，提供 `/health` 自检；上游无可用结果返回 502 而不是空列表，避免凑出假样本。Worker 地址与令牌写入本地 `config.env`，不入库。首次需 `wrangler login`；`--dry-run` 可只做本机检查。
  - **C 路径**：`SEARCH_PROVIDERS_FILE` 指向已存在的文件即进入自定义模式，脚本**不重建、不覆盖**；`scripts/u07-43/preflight.py` 按 Rust 侧约束逐项校验字段/枚举/`id` 与 `priority` 唯一性/公网 HTTPS 主机/每个 `authRef` 的环境变量非空（`degraded` 也算可用源，与 `ProviderProfile::is_usable` 对齐）。多 `authRef` 用 `--env NAME=VALUE` 逐个给值。
  - 配置统一写 `.tmp/u07-43/config.env`（`.tmp/` 已忽略）；主脚本自动加载，可用 `--config FILE` 指定、`--no-config` 忽略；也支持手动 `export SEARCH_ENDPOINT` / `SEARCH_AUTH_REF` / `<凭据>` / `SEARCH_PROVIDERS_FILE`。
  - 脚本用 `scripts/u07-43/gen-queries.py` 生成互不重复查询（主脚本默认 400 条，可用 `U07_QUERY_COUNT` 覆盖；单独调用 `gen-queries.py` 默认 240 条，前 200 条固定覆盖 50 个术语 × 4 个角度）；采样会继续往后取查询，直到攒满 200 个真实成功样本，单条查询最多试 2 次——公网隧道偶发丢连接不会让整轮作废。`scripts/u07-43/verdict.py` 判定 `real_samples ≥ 200 / p95 ≤ 8s / p99 ≤ 12s / 超时率 ≤ 2%`（`search_live.rs` 结尾断言同为 `>= 200`，多源配置下一次写入多条不会误判失败）；`--check-only` 只预检不出网，`--smoke-only` 只跑冒烟。端点须为公网 HTTPS 且响应 `GET ?q=&format=json → {"results":[{"url","title"}]}`。
- 报告与日志不记录查询原文、URL 查询串、页面正文或标题结论。

## 关键词卡片

链路：`B3 片段/要点 → B4 门控（G1~G5）→ 候选派生 → 38 卡片数据 → 39 引用对齐 → 51 视图/点击`。

- 实时入口：「关键词卡片」卡片里的"开始实时转写"——WASAPI 环回 → 16 kHz 单声道 → vosk 流式识别 → 转写文字（**默认只进转写区，不出卡**）；只有在勾选"转写时自动出卡"后才继续 `jieba-rs` 关键词 → B4 门控 → 卡片。运行期从 `resource_dir/asr` 读取脚本与模型，缺失时回退到 `app/src-tauri/asr/`、再回退 `.tmp/asr/`。
- 出卡方式：**默认按需生成**。开会时每句稳定文本都抽关键词会产出大量碎片卡、把卡片区刷成噪音，因此自动出卡默认关闭（ASR 稳定句与 UIA 字幕两条自动入口都受同一个开关控制，`set_auto_cards`，开关态由 `asr_status` 第 5 位回传）；主路径是"转写 → 选中文字 → 查意思 → 生成卡片"。即使打开自动出卡也有两道限流：**同一个词 90 秒内不重复出卡，每分钟最多 8 张**（只作用于自动路径，手动"生成卡片"与释义出卡不受限）。卡片区显示当前张数并提供"清空"（`clear_cards`），清空幂等。
- Python 解释器依次探测 `python` / `py -3` / `python3`，**只选能 `import vosk` 的那个**：PATH 里第一个 `python` 常常没装 vosk（微软商店存根或工具自带解释器），旧逻辑会选中它、子进程立刻退出，表现为"点了开始转写却一直没有文本"。三个候选都不带 vosk 时报 `asr_python_missing` 并提示 `pip install vosk`。
- 识别协议：`stream_asr.py` 输出两种行——`{"partial": "..."}` 为中间结果，`{"text": "..."}` 为稳定句（超时强制收句时多一个 `"cut": "timeout"`）。vosk 只在静音端点给 final，连续说话时会长期只有 partial，因此脚本对 partial 做**稳定判定（约 0.8s 无变化）与 6s 超时强制收句**，主动产出稳定句。界面文本位跟随 partial 实时刷新，**稳定句进全文缓冲**（开启自动出卡时才抽关键词；半句不出卡，连续重复句去重）。超时收句切出的一定是半句，Rust 侧据此把下一段**接在同一行末尾**（去重叠词、单行 400 字封顶），不会读成"一句半"两行。
- 诊断：`asr_status` 第四项回传"音频 X s / N 片 / 写入 KB / 稳定句 M"，显示在按钮旁。**未采到音频时不再留空**，而是明确显示 `音频 0.0s（0 片）：未采到系统声音`——这种情况下问题在系统输出（未播放、音量静音、会议用的是非默认设备），与识别无关。
- 运行日志：转写启动、解释器选择、子进程 stderr、采集片数与稳定句计数写入 `%LOCALAPPDATA%\meeting-copilot\asr.log`（**只记状态与计数，不记任何识别文本或会议内容**）。真机排查"没有文本"时先看这个文件。
- 完整转写：稳定句同时进入内存全文缓冲（`transcript_snapshot`，单次会话最多 600 句，**不落库、退出即清空**），界面在「完整转写」卡片里逐条展示；停止转写不清空，便于回看。
- 选中查义：在转写区选中一段文字（≤200 字）后点"查意思"→ `explain_selection`。已配置 API Key 且构建启用 `llm-https` 时，只把**选中的文字 + 前后各一句**发给模型（20s 超时），返回不超过 120 字的释义；未配置 Key、未启用传输或网络失败时自动回退"本机上下文"（该内容出现次数与相关句子），面板上如实标注来源是"模型释义"还是"本机上下文"。选中**多句或整段**（≥24 字且 ≥2 句）时切换为概括模式：一句话说清在讲什么 + 最多两个关键信息（数字/结论/待办）+ 术语半句解释，上限 180 字，不逐句翻译。跨行选中时上下文定位按"整段 → 最长句 → 最长连续词块"逐级回退，避免跨句选区拿不到上下文。
- 释义出卡：解释面板的"生成卡片"→ `ingest_explanation`，标题为选中文字、结论为释义摘要，沿用同一套 B4 门控与隐私判定（投屏态照旧拒绝生成）。若这次点过"联网查"，检索到的站点会作为 `GeneralWeb` 引用写进卡片（详情里表现为 `web-0:example.com`）。
- 联网查证（可选）：「联网查资料」折叠块里选服务（博查 Bocha / Tavily）、填检索 Key 并保存 → `search_config_save`。之后选中文字点"联网查"→ `web_search`，只把**选中的那段文字（≤80 字）**发出去（8s 超时），返回最多 5 条标题/站点/摘要/地址。检索 Key 存在**独立的系统凭据目标**（`meeting-copilot/search-api-key`），与模型 Key 互不覆盖，配置文件里只有服务名与开关。
  - **不会自动出网**：默认只在用户点"联网查"时查询。想省掉这一步，可勾选"生成卡片时自动补来源"——只影响用户点"生成卡片"那一下，且查不到就静默退回本机引用。
  - 服务端点固定为两家预设，不接受任意地址，避免把查询发到不可控主机。
  - **每小时限额**：默认每小时最多查 20 次，可在同一折叠块里改（`0` = 不限，上限 200）。计数落在 `search-quota.json`（只有时间戳与次数），按整小时窗口重置；超限时直接拒绝并说明，不会悄悄继续消耗你的额度。折叠标题实时显示"本小时 N/上限"。
  - **清除 Key**：同一折叠块里的"清除 Key"从系统凭据中删除检索凭据，服务名、限额与"自动补来源"配置保留。
- 辅助回复的窗口可调：设置页"辅助回复"里改提醒句数（3~10，默认 5）与上下文句数（5~20，默认 12），保存后落 `assist.json`；越界值自动夹到范围内，提醒窗口不会长于上下文窗口。
- 辅助回复（大模型）：转写区右上"AI 提醒 / 怎么回" → `assist_reply`。上下文**固定最近 12 句（≤900 字）**，其中**提醒只看最后 5 句**，更早的句子只供回应参考；可带选中的一句。返回两条提醒（下一步注意什么、可能被追问什么）与 1~2 条可以直接说出口的回应，逐条可复制。两段在同一次请求里分开标注，不多花一次调用。模型不按 JSON 格式返回时不会装作成功，而是把原文交回面板。无 Key 时直接提示去设置页配置，不静默降级。
- 构建差异：`llm-https` 是**非默认特性**，只有带该特性构建的桌面壳才能真正出网（`cargo build -p meeting-copilot-desktop --features llm-https`）；不带特性的构建里"查意思"只会给出本机上下文，联网查与辅助回复会明确报"未启用传输"。
- 打包：`asr/stream_asr.py` 随仓库入库并默认打包；vosk 模型（约 52MB）不入库——发布前复制到 `app/src-tauri/asr/vosk-model-small-cn-0.22/`，再叠加 `app/src-tauri/tauri.asr.conf.json`（Tauri CLI 参数 `--config tauri.asr.conf.json`）打包。基础配置不含模型，因此 CI 与 `cargo clippy`/`cargo test` 无需模型即可编译。
- 手动入口："粘贴会议文本 → 生成卡片"，复用同一 `ingest_text` 链路。
- 卡片列表每 1 秒自动刷新；来源默认 `UserGenerated`（本地片段），点过"联网查"的卡片为 `GeneralWeb`（检索站点，引用 ID 带站点名），两者都可点，不伪造官方来源。

- 仅 B4 `Allow` 才允许出卡/可点击；任一 Deny → 0 卡。手动出卡与用户主动启动的实时转写按 `OnDemand` 显式授权执行 G1~G5，卡片引用真实 Allow trace 与停止级。
- 38 只产 11 项卡片数据；39 只产 `citation_alignment`（`Passed`/`Failed`/`Unknown`，`Unknown` 唯一投影为 `Unverified`）；51 只产视图（10 项）与点击详情（5 项）。
- `Failed`/`Unverified` 仍可展示但不可点；投屏态由 G1 直接拒卡（0 卡），紧急遮蔽照常出卡但隐藏正文且不可点；撤回不可复活。
- 前端 `app/src/cards/KeywordCard.tsx` / `CardDetail.tsx` 只读消费 10/5 项 DTO；仅 `enabled` 绑点击，禁用卡 `aria-disabled`，隐藏态只留条数角标，详情内联不抢焦点。
- 点击只展开本地详情，不发起检索、不调 LLM、不外发、不写审计。
- 联网查与辅助回复都由用户点击触发：联网查只发选中的词（≤80 字），辅助回复只发最近 12 句（提醒部分只依据最后 5 句，两句数均可在设置里改）+ 选中句（≤900 字）；两者都不发整场转写、不落盘、不写日志正文。检索 Key 与模型 Key 使用两个独立的系统凭据目标。

## 隐私与凭据

- 凭据不入库、不进日志、不进审计、不进回放；错误信息不含 Key 片段。
- 日志仅允许白名单字段；原文、完整地址、查询全文不得落日志。
- 桌面日志写入 `%LOCALAPPDATA%\com.meetingcopilot.desktop\logs\desktop.log`（超过 5 MiB 自动重置），设置页"打开日志"在资源管理器中定位；日志不发往网络。
- 数据库打开前必须提供 OS 密钥库引用与实际派生密钥，禁止明文口令，也不再允许无密钥打开文件库。
- 默认后端对 `sessions`/`utterances`/`insights`/`summaries`/`cards` 的业务载荷执行字段级 AES-256-GCM；FTS5 原文索引需 `sqlcipher` 页级加密才能覆盖，`query_cache`/`proactive_outbox` 的 JSON 字段尚未接入字段级加密。
- 外发都由用户主动点击触发，共三条，且每条只发最小片段：**查意思**（选中片段 ≤200 字 + 前后各一句 → 已配置的模型服务）、**联网查**（选中词 ≤80 字 → 所选检索服务）、**AI 提醒 / 怎么回**（最近 12 句 + 选中句，≤900 字 → 模型服务）。三者都不发整场转写；未配置 Key 或未启用 `llm-https` 时完全不出网。"自动外发""查询脱敏""审计"三项在界面上仍显示未接入——自动外发确实未做，不要把这理解成"全程不外发"。
- 审计只追加，不随会话删除；在线保留 180 天，归档保留 5 年（项目自用设置），保留期内不自动销毁。
- 投屏与紧急遮蔽的开发期注入入口：设置 `PRIVACY_SCREEN_SHARE=1` 后 G1 `ScreenShareProtection` 直接拒卡（0 卡）；`PRIVACY_EMERGENCY_MASKED=1` 时照常出卡，但卡片为禁用态、正文隐藏且不可点。两处只在进程启动时读取、只认严格 `"1"`，默认全关；仅为真机取证准备，本身不新增 IPC 命令，生产发布不得开启。
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
| `docs/progress/b4-status.md` | 门控/卡片/设置向导与 AC 三态（§29 P-UI 收口、§34 U-08 真实 LLM 采样基线、§49 U-05 真机截图复核、§50 U-07-43 三轮真实 200 样本取证、§51 工具链根因与收尾修复、§53 U-05 第 7/8 项真机取证完成） |
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