# Meeting Copilot（会议副驾）

监听会议对话、自动捕捉要点、主动联网补全信息的桌面 AI 助手。

首版约束：全自动主动提示；强制对接腾讯会议 / 飞书 / Zoom；桌面客户端（Tauri 2 + Rust + React）；检索源开放可配置。

## 当前状态

B0（工程基座与合规前置）已落地并在 Windows x64 验证：

- 五层 Rust 架构：`domain` → `application` / `infrastructure` / `platform-*` → `ui-bridge` → 桌面壳
- 约束型值对象、领域模型、端口 SPI、错误码模型
- Tauri 2 桌面壳可启动
- SQLite 加密门禁 + 四 Repository（主表 / FTS5 / 向量表同事务）
- HKDF 密钥派生与 AES-256-GCM 字段加密
- 审计哈希链、脱敏日志白名单、10 项度量
- CI：web / Rust 三 target / 合规三段流水线

未闭合项见 [`docs/progress/b0-status.md`](docs/progress/b0-status.md) 与 [`docs/progress/ci-readiness.md`](docs/progress/ci-readiness.md)。

## 目录结构

```
crates/
  domain/            领域模型、值对象、端口 trait（禁止依赖实现）
  application/       应用编排（审计降级守卫）
  infrastructure/    存储、加密、审计链、度量、日志
  platform-win/      Windows 适配（WASAPI / UIA / OCR）
  platform-mac/      macOS 适配（ScreenCaptureKit / AX / Vision）
  ui-bridge/         IPC DTO 与绑定契约
app/                 React 19 + TypeScript + Vite 前端
app/src-tauri/       Tauri 2 桌面壳
docs/sdd/            需求规格、技术方案、任务清单
docs/progress/       B0 状态与 CI 就绪记录
docs/compliance/     许可证复核、SBOM
scripts/             图标、SBOM、密钥扫描、分层校验、绑定校验
```

## 本地开发

```bash
# Rust
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
rustfmt --edition 2021 --check $(git ls-files '*.rs')

# 前端
cd app
npm ci
npm run lint
npm run typecheck
npm run check:bindings
npm run build

# 合规
bash scripts/scan-secrets.sh
bash scripts/check-layering.sh
```

桌面端开发调试：

```bash
cd app
npm run dev          # 前端开发服务器
cargo build -p meeting-copilot-desktop
```

## 文档索引

| 文档 | 说明 |
|---|---|
| `docs/sdd/spec.md` | 需求规格（EARS / 验收标准） |
| `docs/sdd/design.md` | 技术方案（架构、时序、选型、追溯矩阵） |
| `docs/sdd/tasks.md` | 分批次编码任务与门禁 |
| `docs/progress/b0-status.md` | B0 执行状态与偏差 |
| `docs/progress/ci-readiness.md` | CI 预检结果与剩余问题方案 |
| `docs/compliance/license-review.md` | 高风险组件许可证边界 |

## 安全与合规

- 不内置、不捆绑 AGPL/GPL 组件；检索源仅通过开放 SPI 接入。
- 数据库打开前必须提供 OS 密钥库引用，禁止明文口令。
- 日志仅允许白名单字段，原文、URL 全文、查询全文不得落日志。
- 审计记录只追加，保留 180 天，不随会话删除。

## 许可证

MIT OR Apache-2.0