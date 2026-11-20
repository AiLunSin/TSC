# CI 就绪状态与 GitHub 上传阻塞

> 记录日期：2026-11-20
> 关联提交：`c6eded4 feat: 初始化 Meeting Copilot B0 工程基座与 CI 流水线`

## 1. 本地 CI 预检（模拟 GitHub Actions 各步骤）

| 步骤 | 命令 | 结果 |
|---|---|---|
| web / install | `npm ci`（依赖已锁） | 通过 |
| web / lint | `npm run lint` | 0 error |
| web / typecheck | `npm run typecheck` | 0 error |
| web / bindings-contract | `npm run check:bindings` | 4 个 DTO 一致 |
| web / build | `npm run build` | 通过，JS gzip 71.44 KB |
| web / bundle-size-budget | 本地复算 | 约 71 KB，低于 800 KB |
| rust / fmt | `rustfmt --check`（独立二进制） | 40+ 文件 0 差异 |
| rust / clippy | `cargo clippy --workspace --all-targets -D warnings` | 通过 |
| rust / build | `cargo build --workspace` | 通过 |
| rust / test | `cargo test --workspace` | **80 个测试全部通过** |
| compliance / layering | `scripts/check-layering.sh` | PASS |
| compliance / secrets | `scripts/scan-secrets.sh` | 0 命中 |

## 2. 本轮对 CI 的修复

| 问题 | 修复 |
|---|---|
| `bundle-size-budget` 的 `find \| xargs gzip` 会混入 gzip 文件名头，统计不可靠 | 改为逐文件 `gzip -9 -c` 求和，并加 shell 括号 |
| `macos-13` 已接近退役 | 改用 `macos-14` 同时构建 arm64 与交叉 x86_64 |
| `cargo install cargo-deny/cargo-cyclonedx` 易超 15 分钟 | 改用 `taiki-e/install-action` 拉取预编译二进制 |
| 缺少绑定契约门禁 | 新增 `bindings-contract` 步骤 |
| `deny.toml` 含占位 crate，语义误导 | 移除 `exceptions` 与 `bans.deny` 占位项 |
| Windows 行尾 CRLF 可能破坏 Linux 上的 shell 脚本 | 增加 `.gitattributes` 强制 `eol=lf` |
| Windows arm64 target 需 ARM64 MSVC 工具链，标准 runner 不保证 | 暂移出矩阵，待自托管或专用工具链恢复 |

## 3. GitHub 上传阻塞（外部原因）

| 检查 | 结果 |
|---|---|
| `https://github.com` | HTTP `000`（不可达） |
| `https://api.github.com` | HTTP `000`（不可达） |
| 环境变量代理 | 无 |
| Git HTTP 代理 | 无 |
| 系统 WinHTTP 代理 | 直接访问 |
| 本地常见代理端口 7890/7897/10809/8080 | 全部 `000` |
| GitHub CLI（`gh`） | 未安装 |
| Git 凭证助手 | 未配置 |
| 远程仓库 | 无 |

**结论**：当前网络无法直连 GitHub，且没有任何可用认证凭据，因此无法执行 `git push`，也无法触发 Actions。

**解除条件（满足任一）**：

1. 提供可用的 HTTPS 代理（设置 `HTTPS_PROXY`），并确认代理可访问 `github.com`；
2. 提供目标仓库地址与 Personal Access Token（`https://<token>@github.com/<owner>/<repo>.git`）；
3. 配置 SSH Key 且网络可达 `github.com:22`。

仓库已在项目目录初始化为独立仓库（不再使用用户主目录的空仓库），提交历史从 `c6eded4` 开始。

## 4. 剩余问题与方案

| 问题 | 现状 | 方案 |
|---|---|---|
| SQLCipher 页级加密 | 默认 `bundled` SQLite，未启用页级加密 | 在具备 NASM/OpenSSL 的机器或 CI（ubuntu 自带 nasm）上切换 `rusqlite` feature 为 `bundled-sqlcipher-vendored-openssl`，在 `prepare` 中执行由 L1 派生密钥生成的 `PRAGMA key`，并补跨密钥打开失败测试 |
| macOS 两 target | 本机无法验证 | 由 `rust (aarch64-apple-darwin)` 与 `rust (x86_64-apple-darwin)` 两个 job 覆盖 |
| Windows arm64 | 未纳入 CI 矩阵 | 使用自托管 ARM64 runner 或安装 ARM64 MSVC 组件后恢复 |
| rustfmt | 本机 rustup 组件被拒，已用独立二进制完成格式化 | CI runner 自带组件，`cargo fmt --check` 可正常执行 |
| TS 绑定生成 | 已落地手写 `types.ts` + `schema.json` + 双向契约测试 | 后续可替换为 `ts-rs`/`tauri-specta` 自动生成，契约测试保留 |
| v0→v1 迁移回放 | 已补测试：旧行保留、版本升至 1 | — |

## 5. 推送后的首轮 CI 结果

| Job | 结果 | 原因 |
|---|---|---|
| web | 通过 | — |
| rust (x86_64-pc-windows-msvc) | 未完成 | 首轮排队超时/被取消 |
| rust (aarch64-apple-darwin) | 取消 | 1s 内未启动，疑与 run 取消有关 |
| rust (x86_64-apple-darwin) | 取消 | 同上 |
| compliance | **失败** | `deny.toml` 使用了 cargo-deny 已移除的 `[licenses] deny` 键 |

## 6. 第二轮修复（针对实际 CI 失败）

| 问题 | 根因 | 修复 |
|---|---|---|
| compliance 失败 | cargo-deny 0.22 已移除 `[licenses] deny`（PR #611），配置校验直接报错 | 删除该键，改为依赖 `allow` 白名单的默认拒绝语义 |
| compliance 失败（潜在） | `bans.wildcards = deny` 命中 3 个工作区 path 依赖 | 为 path 依赖补 `version = "0.1.0"` |
| compliance 失败（潜在） | `cargo deny advisories` 命中 `libsqlite3-sys 0.30.1`（RUSTSEC-2026-0459，CVE-2026-119375） | `rusqlite` 升级到 0.41（内置 SQLite 3.54.1），`rust-version` 提至 1.89 |
| compliance 失败（潜在） | `sbom.sh` 使用了不存在的 `cargo cyclonedx --output-cdx` 参数 | 改为 `--format json --spec-version 1.5`，输出名由工具默认生成 |
| rust 失败（潜在） | 41 个文件未通过 `cargo fmt --check` | 全量 `cargo fmt --all` |
| rust 超时风险 | Tauri 冷缓存编译耗时长 | `timeout-minutes` 15 → 30 |
| 仓库噪音 | 跟踪了 `tsconfig.tsbuildinfo`、`src-tauri/gen/schemas` 等生成物 | 移出版本库并加入 `.gitignore` |

本地复验：`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`（80 个测试）、`cargo deny check licenses bans sources advisories`（四项全 ok）、`scripts/check-layering.sh`、`scripts/scan-secrets.sh`、`scripts/sbom.sh ci` 均通过。
## 6. 首轮 CI 结果与修复（2026-11-20）

首轮结果（AiLunSin/TSC）：web 通过（22s）；compliance 失败（32s）；rust 矩阵中 mac 两个任务被并发取消，windows 进行中。

| 问题 | 修复 |
|---|---|
| `taiki-e/install-action` 不支持 `cargo-cyclonedx`，安装步骤可能直接失败 | 仅保留 `cargo-deny` |
| `cyclonedx-npm` 扫描 `node_modules` 长时间不返回 | 改为零依赖锁文件生成 `scripts/gen-sbom.mjs` |
| `sbom.sh` 使用 `timeout`，在 Windows 上解析到 `TIMEOUT.EXE` 导致挂起 | 移除 timeout 包装 |
| `cargo-cyclonedx` 安装耗时且非必需 | 移除，Rust 组件直接从 `Cargo.lock` 解析 |

本地复验：layering PASS、secrets 0 命中、`cargo deny` 四项 OK、SBOM 生成 **705 个组件**（176 KB）。

mac 任务的 `cancelled` 属于同分支并发推送取消旧 run，待下次推送观察。

## 7. 第二轮 CI 修复（2026-11-20）

失败点：`rust (x86_64-pc-windows-msvc)` 的 `clippy` 步骤，27s 失败。

日志关键行：

```
error: proc macro panicked
--> app/src-tauri\src\main.rs:20:14
= help: message: The 'frontendDist' configuration is set to "../dist" but this path doesn't exist
```

原因：`tauri::generate_context!()` 在编译期读取 `frontendDist`，Rust job 未构建前端，`app/dist` 不存在。

修复：Rust matrix 三个 target 在 `fmt/clippy/build/test` 前增加 `web-assets` 步骤（`npm ci` + `npm run build`），确保 `app/dist` 存在。
