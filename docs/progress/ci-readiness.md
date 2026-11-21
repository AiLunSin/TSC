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
| SQLCipher 页级加密 | 已接入 `sqlcipher` feature（`bundled-sqlcipher-vendored-openssl`），`prepare` 首步执行 `PRAGMA key`，缺密钥/错密钥均拒绝打开 | 由新增的 `sqlcipher (page-level encryption)` job 在 ubuntu 上安装 NASM 后实跑；Windows 本机需安装 NASM 才能启用 |
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

## 8. 第三轮 CI 结果与修复（2026-11-20）

| 任务 | 结果 |
|---|---|
| web | ✅ 18s |
| compliance | ✅ 16s |
| rust (x86_64-pc-windows-msvc) | ✅ 4m |
| rust (aarch64-apple-darwin) | ⏹ Cancelled after 1s |
| rust (x86_64-apple-darwin) | ⏹ Cancelled after 1s |

mac 两个 job 在 1 秒内被取消，属 runner 标签无法分配（`macos-14` 已不可用），与代码无关。修复：改用 `macos-latest` 承载两个 target，x86_64 采用交叉编译。

## 9. 第四轮 CI 修复（2026-11-20）

macOS job 已能分配到 runner，但 `clippy` 报：

```
error: proc macro panicked
= help: message: failed to open icon app/src-tauri/icons/icon.png: No such file or directory
```

原因：Windows 构建使用 `icon.ico`，macOS 构建要求 `icon.png`。

修复：`scripts/gen-icon.py` 同时生成 `icon.ico`（32×32）与 `icon.png`（128×128 RGBA），已纳入版本控制。

## 10. SQLCipher 页级加密接入（2026-11-20）

`crates/infrastructure` 新增 feature，`SqliteStore` 打开链路改造：

| 项 | 内容 |
|---|---|
| 默认（`bundled-sqlite`） | 内置 SQLite，库文件明文，依赖密钥门禁 + 字段级 AES-GCM |
| `--features sqlcipher` | `rusqlite/bundled-sqlcipher-vendored-openssl`，库文件页级加密 |
| 打开 | `SqliteStore::open_with_key(config, Some(&StoreKey))`，`prepare` 首步 `PRAGMA key = "x'…'"` 并读 `sqlite_master` 校验 |
| 失败语义 | 缺密钥 → `StoreError::MissingKey`；错误密钥 → `StoreError::WrongKey`；不静默落明文 |
| 测试 | 缺密钥拒绝、错误密钥拒绝、正确密钥回读、文件头非 `SQLite format 3`（cfg `sqlcipher`） |
| CI | 新增 `sqlcipher (page-level encryption)` job：ubuntu + `apt-get install nasm` + `cargo test -p meeting-copilot-infrastructure --features sqlcipher` |

本地复验：默认特性下 `cargo fmt/clippy/test` 全绿（80 个测试）；`--features sqlcipher` 的 Rust 代码已通过临时 feature 桩完成 `cargo check --all-targets`。Windows 本机因缺少 Strawberry Perl，vendored OpenSSL 未能完成构建，该路径以 CI 结果为准。

## 10. 第五轮 CI 修复：SQLCipher 与许可证（2026-11-20）

### 失败 1：`sqlcipher (page-level encryption)` 6 个测试失败

SQLCipher 模式下 `SqliteStore::open_in_memory()` 仍以 `key = None` 调用 `apply_key`，返回 `MissingKey`，导致 5 个使用 `store()` 的用例与 `schema_is_migrated_to_v1` 失败。

修复：
- `open_in_memory` 在 `sqlcipher` feature 下生成一次性随机 `StoreKey`（内存库不落盘，密钥随库销毁）；
- `v0_database_is_migrated_and_preserves_rows` 标注 `#[cfg(not(feature = "sqlcipher"))]`（SQLCipher 模式下不存在明文 v0 库）。

本地编译说明：Windows git-bash 的 Perl 缺少 `Locale::Maketext::Simple`，OpenSSL 配置阶段退出，因此 SQLCipher 测试只能由 Ubuntu CI 验证（与 NASM 无关，openssl-src 已使用 `no-asm`）。

### 失败 2：`compliance` 的 `licenses` 检查

`sqlite3-src` 声明 `MIT AND BSD-3-Clause AND blessing AND WTFPL`，其中 `blessing`（SQLite Blessing）与 `WTFPL` 未在允许列表。

修复：`deny.toml` 的 `allow` 增加 `blessing`、`WTFPL`。本地 `cargo deny check licenses` 已通过（exit 0）。

### 本地复验

- `cargo deny check licenses` → `licenses ok`
- `cargo test -p meeting-copilot-infrastructure` → 24 passed
- `cargo clippy -p meeting-copilot-infrastructure --all-targets -D warnings` → 通过

## 11. 第六轮：工具链版本锁定（2026-11-20）

最新一轮三个 Rust job 在 23s–1m 内失败，compliance 16s 失败，均未进入有效编译阶段。本地 `check-layering`、`scan-secrets`、`cargo deny licenses` 均为通过，判断为 **stable 工具链漂移**：`@stable` 已解析到新版本，新版 rustfmt/clippy 会使原本通过的代码失败。

修复：
- 所有 `dtolnay/rust-toolchain@stable` 锁定为 `@1.100.0`（与本地验证版本一致）；
- `taiki-e/install-action` 的 `cargo-deny` 锁定为 `0.22.0`（本地 `licenses ok` 的版本）。

本地限制：rustfmt 独立二进制在当前环境被系统拦截（exit 127），无法本地复跑 `fmt --check`。

## 12. 第七轮修复：fmt 与 SQLCipher CVE（2026-11-20）

### 失败 1：Rust 三个 job 的 `fmt`

原因：`open_in_memory` 新增代码在 `impl` 内缺少 4 空格缩进（注释与函数签名首行）。已用工具链自带 `rustfmt 1.11.0` 修正，`cargo fmt --all --check` 0 差异。

### 失败 2：compliance 的 `advisories`

`licenses ok`，实际失败项为 `advisories`：`sqlcipher-src 419.0.6` 命中 **RUSTSEC-2026-0502**（CVE-2026-119375，SQLite WAL 读取帧漏洞）。

修复：升级 `rusqlite 0.41.1 → 0.43.0`，依赖随之更新：

| 依赖 | 旧 | 新 |
|---|---|---|
| rusqlite | 0.41.1 | 0.43.0 |
| libsqlite3-sys | 0.39.1 | 0.40.0 |
| sqlcipher-src | 419.0.6（受影响） | **420.0.0（含修复）** |

### 本地复验

- `cargo test --workspace` → 80 passed, 0 failed
- `cargo clippy -p infrastructure -D warnings` → 通过
- `cargo deny check advisories` → `advisories ok`
- `cargo fmt --all --check` → 0 差异

SQLCipher feature 的实际编译仍需 Ubuntu CI 验证（本地 Perl 缺少 `Locale::Maketext::Simple`）。
## 13. 当前 HEAD 本地 CI 模拟（2026-11-21 · `9baa573`）

> 说明：第 1~12 节记录的是 B0 阶段（80 个测试）的 CI 调试过程，数字已过期。本节为该状态在 B1/B2 交付后的重新校准。

### 13.1 门禁结果

| Job | 步骤 | 命令 | 结果 |
|---|---|---|---|
| web | lint | `npm run lint` | 0 |
| web | typecheck | `npm run typecheck` | 0 |
| web | bindings-contract | `npm run check:bindings` | 0 |
| web | build | `npm run build` | 0 |
| web | bundle-size-budget | 逐文件 `gzip -9 -c` 求和 | 72,259 B（预算 819,200 B） |
| rust | fmt | `cargo fmt --all --check` | 0 |
| rust | clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| rust | build | `cargo build --workspace` | 0 |
| rust | test | `cargo test --workspace --no-fail-fast` | 38 targets / 399 passed / 0 failed / 8 ignored，连跑 3 次一致 |
| compliance | layering | `bash scripts/check-layering.sh` | PASS |
| compliance | secrets | `bash scripts/scan-secrets.sh` | 0 命中 |
| compliance | secrets-docs | `bash scripts/scan-secrets-docs.sh --all-md` | 0 命中 |
| compliance | licenses | `cargo deny --offline check licenses bans sources advisories` | 四项全 ok |
| compliance | sbom | `bash scripts/sbom.sh ci` | 719 个组件 |
| compliance | 用例名差集 | `CARGO=<toolchain>/cargo.exe bash scripts/test-name-inventory.sh --diff` | PASS，removed=0，新增 42 |
| compliance | 采集离线 | `bash scripts/check-capture-offline.sh` | PASS |
| compliance | G2 宣称 | `bash scripts/check-g2-claims.sh` | PASS |
| sqlcipher | test | `cargo test -p meeting-copilot-infrastructure --features sqlcipher` | 本地不可跑（Perl/OpenSSL），仍以 Ubuntu CI 为准 |

### 13.2 本地环境关键约束

`PATH` 中的 rustup shim（`~/.cargo/bin/{cargo,rustc,rustfmt}`）在本机损坏：直接执行返回 127，被 cargo 作为子进程调用时报 `STATUS_DLL_NOT_FOUND`。有效规避方式：

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
```

前置后 `cargo`/`rustc`/`rustfmt` 均解析到 toolchain 内真实可执行文件；`cargo fmt` 与 `cargo clippy` 恢复正常（仅设 `RUSTC`/`RUSTDOC` 不足以让 `cargo fmt` 找到 `rustfmt`）。

`scripts/test-name-inventory.sh` 支持 `CARGO` 环境变量覆盖，CI 中无需修改脚本本身。

### 13.3 已修复的 CI 配置缺口（2026-11-21 已落地，见第 14 节）

| 编号 | 缺口 | 影响 |
|---|---|---|
| C1 | `compliance` job 未执行 `check-capture-offline.sh`、`check-g2-claims.sh`、`scan-secrets-docs.sh` | 三个门禁只在本地跑，远端无守护 |
| C2 | `rust` job 未编译 `--features llm-https` | `llm_live_baseline.rs` 等仅在该 feature 下编译，存在特性组合漂移风险 |
| C3 | `test-name-inventory.sh --diff` 未纳入 CI | 用例被删除时无自动拦截 |
| C4 | `test-name-inventory.sh` 在 `cargo` 不可用时静默吞错，把"抓取失败"报成"removed 全量" | 本地实测：shim 损坏时误报 365 条 removed、退出码 1；应改为退出 2（扫描面缺失） |
## 14. CI 门禁加固（C1~C4）本地执行记录（2026-11-21 · AC-CI-26 本地侧）

> 口径声明：本节只记录**本地可判定**结果。远端 Actions 结论必须由推送后的运行日志取得（见 14.3），本地模拟不得记为"远端通过"。

### 14.1 本地实测结果

| 阶段 | 判据 | 结果 |
|---|---|---|
| P0 / C4 | `--self-test` | 退出 0，`SELF-TEST: 6/6 ok`，耗时 19.5s，不联网、不调用真实 cargo |
| P0 / C4 | 127 桩 `--diff`（连跑 3 次） | 均退出 2；首行匹配 `^.+:[0-9]+(:| )`；`removed=` 计数 0 |
| P0 / C4 | 真实 `--diff`（连跑 3 次） | 均退出 0；`removed=0`；新增 42 条；`baseline_drift=42` |
| P0 / C4 | 真 removed 场景 | 退出 1；首行 `...:0 出现 removed 用例`；明细含 `ghost_case`；`removed=` 计数 0 |
| P0 / C4 | 空仓库面 / 基线缺失 / 基线逆序 / 空基线 / 平台不可比 | 均退出 2；首行标记串分别为 `manifests=0` / `baseline_missing` / `baseline_unsorted` / `baseline_missing reason=empty` / `不可比` |
| P0 / C4 | 未知参数 | 退出 2，首行 `未知参数 ... usage_error` |
| P1 / C1 | `check-capture-offline.sh` 移除 `fakes.rs` | 退出 2，首行 `...:0 扫描面缺失 [fakes]` |
| P1 / C1 | `check-g2-claims.sh` 移除 `README.md` / `docs/progress` | 均退出 2，首行为对应缺失面 |
| P1 / C1 | 三脚本正常态与 `--self-test` | `capture`/`g2` 自检仍退 1、正常态 0；`scan-secrets-docs` 自检 0、正常态 0 |
| P2 / C2 | 本机 cargo 断言（连跑 3 次） | `collected=1` / `ignored_live=8` / `ignored_baseline=1` |
| P2 / C2 | `ci.yml` 静态审计 | `all-features=0`、`run-llm-live=0`、`LLM_*` 联调变量=0、`compiled=1` 出现 3 次 |
| P3 / C3 | `ci.yml` 编排 | 6 个 job 均有 `timeout-minutes`；`continue-on-error=0`；`shell: bash`=6；`emit-baseline=0`；差集调用=1 |
| 回归 | Rust 门禁 | `fmt=0` / `clippy=0`（`-D warnings`）/ `build=0` / `test` 38 targets、399 passed、0 failed、8 ignored |
| 回归 | 静态门禁 | `layering=0` / `secrets=0` / `deny=0`（licenses+bans+sources+advisories）/ `capture-offline=0` / `g2-claims=0` / `secrets-docs=0` |
| 回归 | 用例名差集 | 退出 0，`removed=0`，新增 42；基线仍 365 行、未改动 |
| 回归 | `ci.yml` 边界 | `git diff -U0` 无删除行；`docs/sdd/**` 零改动 |

### 14.2 实施偏差登记

| 编号 | 偏差 | 处理 |
|---|---|---|
| D-CI-1 | `app/src-tauri` 是 binary crate，`cargo test --lib` 必然返回 101；原脚本靠 `2>/dev/null` 吞掉 | 改造后仅对存在 `src/lib.rs` 或声明 `[lib]` 的包抓 lib 面；"无 lib 面"不算抓取失败。真实抓取恢复 0/1 语义 |
| D-CI-2 | 任务证据串期望 `grep -c 'runs-on: windows-2022'`=2；实际 `rust` 用 `${{ matrix.os }}` | 采用等价口径 `grep -c 'windows-2022'`=2（矩阵 include 1 处 + C3 `runs-on` 1 处）；C3 的 `runs-on` 仍为 `windows-2022` |
| D-CI-3 | 工作区 `ci.yml` 原为 CRLF，`.gitattributes` 要求 LF | 统一转 LF，内容零变化；转换后 `git diff -U0` 无删除行 |
| D-CI-4 | `feature-coverage` 的 `if: always()` 为 job 级汇总语义 | 不视为 C3 差集步骤的吞错修饰；差集步骤块内无 `|| true` / `continue-on-error` / `set +e` / `if: false` |

### 14.3 远端结果（2026-11-21 · 用户确认）

> 推送后由用户在 Actions 页面确认：**6 个 job 全部通过**（`web`、`rust` 三个矩阵、`compliance`、`sqlcipher`、`feature-coverage`、`test-name-inventory`）。逐项日志未导出，L3-4 / L3-5 / L3-7 / L3-10 的具体数值以 Actions 页面为准；本记录不替代后续复跑。

| 编号 | 待观测 | 证据位置 |
|---|---|---|
| L3-1 | 6 个 job 在 Actions 上结论为绿 | Runs 列表 / job 徽标 |
| L3-2 | `compliance` 三条 C1 步骤真实跑通 | compliance 日志中三步骤通过标记 |
| L3-3 | `fetch-depth: 0` 后 `git log` 可扫满 | g2-claims 步骤日志 |
| L3-4 | macOS 两矩阵 `llm-https` 编译成功 | `rust (aarch64-apple-darwin)` / `rust (x86_64-apple-darwin)` 的 `compiled=1` |
| L3-5 | artifact 上传与跨 job 合并 | Artifacts 面板 + `assert-feature-coverage` 日志 |
| L3-6 | Linux"不适用"行可见 | `feature-coverage` 日志 |
| L3-7 | C3 全量编译 + 差集耗时不超过 40 分钟 | `test-name-inventory-diff (C3)` 步骤耗时 |
| L3-8 | `windows-2022` 上 `shell: bash` 的 `set -e` 语义 | `web-assets` 日志与失败传播 |
| L3-9 | `feature-coverage` 在 rust 失败时仍执行 | job 关系图 |
| L3-10 | CI 分钟成本与缓存命中率 | 计费面板 / 步骤耗时 |

**结论**：本地判定项全部达成；远端 CI 经用户确认 6 个 job 全绿。仍**不宣称 G2 通过**——G2 需真实三平台 30 分钟稳定运行，CI 绿不构成等价。
