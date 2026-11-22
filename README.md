# Meeting Copilot（会议副驾）

监听会议对话、识别值得解释的关键词、经门控判定后主动提示，并可本地查看关键词详情的桌面 AI 助手。

当前定位：**离线契约、门控与设置/卡片前端已闭环；真实采集、真机验收与真实端点仍未交付**。请以“当前进度”一节为准，不要把规划能力当作已实现能力。

## 当前进度

| 批次 | 状态 |
|---|---|
| B0 工程基座与合规前置 | 完成（Windows 验证） |
| B1 LLM 网关技术验证 | 完成：结构化任务、降级链路、离线基线 |
| B2 离线闭环与双通道合并 | 完成：状态机、仲裁、缺口、43 项 AC |
| B3 理解层（离线内核） | 完成：片段准入、滚动摘要、要点抽取、去重 |
| B4 门控与关键词卡片 | **契约闭环**：AC-B4-01~104 离线通过（AC-B4-45 为 macOS 不适用）；38/39/51 只交付契约与视图模型 |
| P-UI 设置向导 + 关键词卡片前端 | **离线落地**：AC-B4-111~130 离线通过；6 命令、3+3 端口、Windows 凭据存储、原子 4 键配置、卡片组件已接入；真机与真实联网未验证（U-05 `open`） |
| CI 门禁加固 | 完成：B3/B4/P-UI 静态门禁、许可证、SBOM、密钥扫描 |

最近一次全量基线：**859 passed / 0 failed / 9 ignored**（含 P-UI 桌面层 **24 例**，`cargo test -p meeting-copilot-desktop` 单列通过）；`fmt --check`、`clippy -D warnings`、分层、密钥、许可证门禁通过。

仍未完成：

- **U-06 13.3 回放本体**：**已交付**（`gate/replay.rs` 7 项封闭契约 + `gate_replay_driver` 22 用例；AC-B4-95~104 `passed`）。
- **U-05 产品化接入**：设置向导与卡片前端已离线落地（AC-B4-111~130）；真实联网 E2E、真机 WebView2 验收仍未完成，**U-05 保持 `open`**。
- **U-07**：负反馈、缓存/outbox、按需补全、审计存储。
- **U-08**：真实检索源、真实 LLM 冷却、真机性能与误触发率；9 个 `#[ignore]` 用例保持不跑。
- **平台层**：`platform-win` 已提供 Windows 凭据管理器（`secure_store`），WASAPI/UIA/OCR 仍为接口占位；`platform-mac` 不适用（保留占位）。
- **产品能力口径**：38/39/51 只交付契约与视图；前端已只读消费 DTO，但仍不宣称“产品级可点击卡片”或真机可用。

## 目录结构

```
crates/
  domain/            领域模型、值对象、端口 trait（禁止依赖实现）
  application/       应用编排：采集、治理、理解层、门控、卡片、引用对齐、设置端口
  infrastructure/    存储、加密、审计链、度量、日志、LLM 传输
  platform-win/      Windows 适配（凭据管理已实现；采集与 OCR 仍占位）
  platform-mac/      macOS 适配（不适用，保留占位）
  ui-bridge/         IPC DTO、关键词卡片视图、设置向导视图
app/                 React 19 + TypeScript + Vite 前端
app/src-tauri/       Tauri 2 桌面壳（6 个 IPC 命令与装配）
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
cargo build -p meeting-copilot-desktop
```

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

CI 任务：`web`（lint/typecheck/bindings/build）、`rust`（Windows 与两个 Darwin 交叉编译）、`test-name-inventory`、`compliance`（licenses/SBOM/secrets/layering）、`sqlcipher`。`compliance` 与本地使用同一 cargo-deny 版本与同一命令；许可证允许清单见 `deny.toml`。

## LLM 配置入口

- 默认关闭；端点、模型、凭据只在运行期经环境变量注入，**不写入仓库**。
- 相关变量统一 `LLM_*` 命名；调用上限与冷却由网关侧约束。
- 设置向导契约位于 `crates/ui-bridge/src/settings/`，装配与命令位于 `app/src-tauri/src/ipc/`（6 命令），页面位于 `app/src/settings/`；提供 API Key 输入、预设模型、固定服务地址、一键测试与保存/清除。
- 凭据经 `crates/platform-win/src/secure_store.rs` 写入 Windows 凭据管理器（唯一目标名）；非敏感配置为 `settings.json` **恰 4 键**、原子写、损坏回默认；Key 固定掩码 `***`、不落盘明文、不进日志/审计/回放。
- 探针经 `ProbeExecutor` 端口执行；**生产探针默认关闭**，仅 `--features llm-https` 显式开启；真机 WebView2 与真实联网未验证，**不宣称界面已验收**。

## 关键词卡片

链路：`B3 片段/要点 → B4 门控（G1~G5）→ 候选派生 → 38 卡片数据 → 39 引用对齐 → 51 视图/点击`。

- 仅 B4 `Allow` 才允许出卡/可点击；任一 Deny → 0 卡。
- 38 只产 11 项卡片数据；39 只产 `citation_alignment`（`Passed`/`Failed`/`Unknown`，`Unknown` 唯一投影为 `Unverified`）；51 只产视图（10 项）与点击详情（5 项）。
- `Failed`/`Unverified` 仍可展示但不可点；投屏/紧急遮蔽隐藏正文；撤回不可复活。
- 前端 `app/src/cards/KeywordCard.tsx` / `CardDetail.tsx` 只读消费 10/5 项 DTO；仅 `enabled` 绑点击，禁用卡 `aria-disabled`，隐藏态只留条数角标，详情内联不抢焦点。
- 点击只展开本地详情，不发起检索、不调 LLM、不外发、不写审计。

## 隐私与凭据

- 凭据不入库、不进日志、不进审计、不进回放；错误信息不含 Key 片段。
- 日志仅允许白名单字段；原文、完整地址、查询全文不得落日志。
- 数据库打开前必须提供 OS 密钥库引用，禁止明文口令。
- 审计只追加，不随会话删除；保留期以实际配置为准。
- 许可证：不内置 AGPL/GPL 组件；检索源仅通过开放 SPI 接入。

## 文档索引

| 文档 | 说明 |
|---|---|
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
| `docs/progress/handoff-2026-11-22-pui.md` | P-UI 落地交接（C1~C7） |
| `docs/progress/ci-readiness.md` | CI 预检与缺口 |
| `docs/compliance/license-review.md` | 许可证边界 |

## 贡献

1. 修改后本地跑完 Rust、前端与静态门禁。
2. 不提交凭据、真实端点、模型名、请求标识或原文样例。
3. 新增测试 target 时同步 `scripts/test-name-inventory.sh` 的 `NEW_TARGETS`。
4. 能力状态必须与 `docs/progress/b4-status.md` 的 AC 三态一致，禁止把未完成态写成已交付。

## 许可证

MIT OR Apache-2.0