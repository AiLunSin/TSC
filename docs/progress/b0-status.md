# B0 批次执行状态（工程基座与合规前置）

> 依据：`.codeartsdoer/specs/meeting_copilot/tasks.md` 批次 B0（主任务 1–6）
> 最近更新：2026-11-20（第三轮：SQLCipher 页级加密接入）
> 门禁 G0 结论：**Windows 单平台基本达成，整体未闭合**（mac 未验证；SQLCipher 以 feature 接入，Windows 系统级 NASM 仍未安装）

## 1. 环境基线

| 项 | 值 | 说明 |
|---|---|---|
| Node / npm | 26.13.0 / 11.20.0 | 前端构建与依赖安装 |
| Rust | 1.100.0 stable-x86_64-pc-windows-msvc | rustup 1.29.3 |
| MSVC | VS 18 Community（VC Tools x86/x64） | 已具备 |
| WebView2 | 157.0.4341.60 | Tauri 运行依赖 |
| clippy | 0.1.100 | `-D warnings` 通过 |
| rusqlite | 0.41 + bundled SQLite 3.54.1 + FTS5(trigram) | 编译通过（含 RUSTSEC-2026-0459 修复） |
| rustfmt | 可用 | 已通过 rustup 安装，`cargo fmt --all --check` 通过 |
| NASM | 便携版 2.16.03（临时目录） | 系统安装仍失败；SQLCipher 构建用便携版验证，CI 由 apt 安装 |

## 2. 子任务状态

| 子任务 | 状态 | 证据 / 缺口 |
|---|---|---|
| 1.1 Rust workspace 与五层 crate | 完成（win） | `cargo build --workspace` 通过；Tauri 2.12.7 壳已接入；空壳实际启动并存活 5 秒；mac 两 target 未验证 |
| 1.2 前端脚手架 | 完成 | `npm run build` 通过，JS gzip 71.44 KB；lint / typecheck 0 error |
| 1.3 IPC 通道与 DTO 契约 | 部分完成 | 业务 DTO `deny_unknown_fields`、协议版本与 `ipc_version` 命令已注册；TS 绑定生成未落地 |
| 1.4 CI 矩阵 | 完成（未实跑） | web + 3×rust target + compliance 三段流水线 |
| 2.1 cargo-deny 红线 | 完成 | allow 白名单之外默认拒绝，覆盖 AGPL/GPL/SSPL/EUPL/OSL |
| 2.2 CycloneDX SBOM | 完成（未实跑） | 生成与合并脚本 |
| 2.3 高风险组件清单 | 完成 | 签字栏待法务 |
| 3.1 约束型值对象 | 完成 | 构造期与反序列化双重校验 |
| 3.2 领域聚合与枚举 | 完成 | 聚合根、枚举、时间双 newtype |
| 3.3 错误模型 | 完成 | 12 个错误码 + 11 个 DenyCode，测试无遗漏 |
| 3.4 未知枚举降级 | 完成 | `Degraded<T>` 保留原值 |
| 4.1 采集端口 | 完成 | 释放后回调失效 |
| 4.2 理解与生成端口 | 完成 | 7 组能力端口 |
| 4.3 检索 SPI | 完成 | 无服务商常量；`SanitizedQuery` 阻断未脱敏外发 |
| 4.4 治理与存储端口 | 完成 | 纯函数门控契约、受控脱敏构造 |
| 4.5 Mock 与端口契约测试 | **完成** | `crates/domain/tests/contracts.rs`：字幕/音频/脱敏/检索四组同一套断言；SPI 版本不兼容被拒绝 |
| 5.1 SQLCipher 与迁移框架 | 完成（页级加密由 feature 提供） | 无密钥拒绝打开、WAL、`user_version` 迁移、空库→v1 已验证；`sqlcipher` feature 下 `PRAGMA key` 与错误密钥拒绝已覆盖；v0→v1 样本库回放未做 |
| 5.2 密钥分层与轮换 | **完成** | HKDF-SHA256 L1 派生、AES-256-GCM L2 信封加密、AAD 绑定 session_id；错 AAD 解密失败有测试 |
| 5.3 Repository 三写同事务 | **完成** | 四 Repository 落地；主表+FTS5+向量表同事务；注入失败后三表全部回滚（测试）；被引用片段禁止删除（测试） |
| 5.4 密钥与明文残留扫描 | 完成 | 实跑 0 命中 |
| 6.1 审计表与哈希链 | 完成（内存实现） | SHA-256 链、锚点、篡改定位 |
| 6.2 审计失败降级 | 完成 | 四类敏感操作拒绝与回填 |
| 6.3 脱敏日志 | 完成（白名单） | 12 允许 / 6 禁止字段 |
| 6.4 本地度量 | 完成（内存实现） | 10 项指标、p95、维度聚合 |

## 3. 验证结果

| 检查 | 命令 | 结果 |
|---|---|---|
| 前端构建 | `npm run build` | 通过 |
| 前端规范 | `npm run lint` / `typecheck` | 0 error |
| Rust 构建 | `cargo build --workspace` | 通过 |
| Rust 测试 | `cargo test --workspace` | 通过，**80 个测试** |
| Rust 静态检查 | `cargo clippy --workspace --all-targets -- -D warnings` | 通过 |
| 桌面空壳启动 | 运行 `meeting-copilot-desktop.exe` | 进程存活 5 秒，窗口正常 |
| 密钥扫描 | `scripts/scan-secrets.sh` | 0 命中 |
| 格式检查 | `cargo fmt --all --check` | 通过 |

## 4. SQLCipher 页级加密（feature 开关）

`crates/infrastructure` 的 `rusqlite` 依赖保持 `0.41`，通过 feature 选择后端：

| feature | 行为 |
|---|---|
| `bundled-sqlite`（默认） | 内置 SQLite，库文件为明文，依赖"密钥门禁 + 字段级 AES-256-GCM" |
| `sqlcipher` | 切换为 `rusqlite/bundled-sqlcipher-vendored-openssl`，库文件页级加密 |

打开链路：

1. `SqliteStore::open_with_key(config, Some(&key))`，`key` 为 `crypto::derive_l1` 的 32B 输出；
2. `prepare` 首先执行 `PRAGMA key = "x'<64 位十六进制>'"`，随后读一次 `sqlite_master` 校验密钥；
3. 启用 `sqlcipher` 后 `open()` 不带密钥返回 `StoreError::MissingKey`，错误密钥返回 `StoreError::WrongKey`，不会静默生成明文库；
4. 内存库不落盘，保持无需密钥。

构建前置：Windows MSVC 需 NASM + Perl（`openssl-src`）；Linux CI 用 `apt-get install nasm` 后执行：

```bash
cargo test -p meeting-copilot-infrastructure --features sqlcipher
```

## 5. 仍待完成（阻塞 G0 完全闭合）

1. **macOS 两 target 编译与空壳启动**：需要 mac 机器或 CI runner。
2. **SQLCipher 页级加密**：代码与测试已接入；Windows 需安装 NASM 后以 `--features sqlcipher` 构建，CI 已覆盖。
3. **rustfmt 格式门禁**：已在本机安装并通过 `cargo fmt --all --check`。
4. **TS 绑定生成**（1.3）：`pnpm gen:bindings` 对应的生成脚本未落地。
5. **v0→v1 迁移回放测试**（5.1）。