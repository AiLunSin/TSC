# Meeting Copilot（会议副驾）

监听会议对话、自动捕捉要点、主动联网补全信息的桌面 AI 助手。

首版约束：全自动主动提示；强制对接腾讯会议 / 飞书 / Zoom；桌面客户端（Tauri 2 + Rust + React）；检索源开放可配置。

## 当前状态

| 批次 | 状态 |
|---|---|
| B0 工程基座与合规前置 | 完成（Windows x64 验证） |
| B1 技术验证（LLM 网关） | 完成：四类结构化任务、降级链路、真实 30 样本基线 |
| B2 离线闭环 | 完成：通道状态机、仲裁、缺口与恢复、状态推送（43 项 AC） |
| B2.24 双通道去重合并 | 完成：来源投影、四步相似度、三态合并、补充要点 |
| B3 理解层（离线内核） | 完成：片段准入、滚动摘要、要点抽取、去重与回溯 |
| CI 门禁加固 | 完成：C1~C4、B3 六条静态门禁 |

最近一次全量回归：**45 targets / 560 passed / 0 failed / 9 ignored**；`fmt`、`clippy -D warnings`、分层、密钥、许可证、用例名差集均通过。

未完成项按依赖分组记录在 [`docs/progress/b3-status.md`](docs/progress/b3-status.md) 与 [`docs/sdd/tasks.md`](docs/sdd/tasks.md)。

### 仍未实测（需真实环境）

- 真实端点的 429 观测、100ms 截止实测、三项三态落盘
- Windows 真机上的音频捕获、字幕适配、OCR 与双通道去重
- **G2**：Windows 单平台上腾讯/飞书/Zoom 各 30 分钟稳定运行
- macOS 相关能力一律“不适用（无设备）”，CI 仅保留交叉编译观察，不作交付门禁

## 目录结构

```
crates/
  domain/            领域模型、值对象、端口 trait（禁止依赖实现）
    src/model/       会话、片段、要点、通道、理解层策略与滚动摘要
    src/merge/       双通道去重合并的纯函数内核
  application/       应用编排：采集、治理、理解层（understand/）
  infrastructure/    存储、加密、审计链、度量、日志、LLM 传输
  platform-win/      Windows 适配（WASAPI / UIA / OCR）与能力矩阵
  platform-mac/      macOS 适配（不适用，保留接口与能力矩阵）
  ui-bridge/         IPC DTO 与绑定契约
app/                 React 19 + TypeScript + Vite 前端
app/src-tauri/       Tauri 2 桌面壳
docs/sdd/            需求规格、技术方案、任务清单（权威，只读）
docs/progress/       各批次状态、验收记录与 CI 就绪
docs/baseline/       LLM 与 ASR 基线
docs/compliance/     许可证复核、SBOM
scripts/             门禁：分层、密钥、许可证、采集离线、mac 范围、B3、用例名、SBOM
```

## 本地开发

### 环境注意（Windows）

本机 rustup shim 可能不可用；直接使用工具链内的可执行文件，或前置 PATH：

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
```

### Rust

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
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

### 门禁脚本

```bash
bash scripts/check-layering.sh
bash scripts/scan-secrets.sh
bash scripts/scan-secrets-docs.sh --all-md
bash scripts/check-capture-offline.sh --all
bash scripts/check-mac-scope.sh --rule=na
bash scripts/check-b3-gates.sh --rule=source
bash scripts/check-g2-claims.sh
CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe" \
  bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt
cargo deny --offline check licenses bans sources advisories
```

### 桌面端

```bash
cd app
npm run dev
cargo build -p meeting-copilot-desktop
```

## 文档索引

| 文档 | 说明 |
|---|---|
| `docs/sdd/spec.md` | 需求规格（EARS / 验收标准） |
| `docs/sdd/design.md` | 技术方案（架构、时序、选型、追溯矩阵） |
| `docs/sdd/tasks.md` | 分批次编码任务与门禁 |
| `docs/progress/b0-status.md` | B0 执行状态与偏差 |
| `docs/progress/b1-status.md` | B1 技术验证、LLM 联调与真实采样 |
| `docs/progress/b2-status.md` | B2 通道状态机进度 |
| `docs/progress/b2-offline-closure.md` | B2 离线闭环 43 项 AC 与偏差 |
| `docs/progress/b2-24-merge.md` | 双通道去重合并验收记录 |
| `docs/progress/b3-status.md` | B3 理解层进度、门禁与回滚演练 |
| `docs/progress/ci-readiness.md` | CI 预检结果与配置缺口 |
| `docs/compliance/license-review.md` | 高风险组件许可证边界 |

## 安全与合规

- 不内置、不捆绑 AGPL/GPL 组件；检索源仅通过开放 SPI 接入。
- 数据库打开前必须提供 OS 密钥库引用，禁止明文口令。
- 日志仅允许白名单字段，原文、URL 全文、查询全文不得落日志。
- 审计记录只追加，保留 180 天，不随会话删除。
- LLM 默认关闭；端点、模型与凭据只经运行期环境变量注入，**不写入仓库任何文件**。测试记录也不得包含凭据、完整端点、模型名与 request_id。

## 许可证

MIT OR Apache-2.0