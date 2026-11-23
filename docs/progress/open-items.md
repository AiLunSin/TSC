# 未完成事项总表（2026-11-23）

> 口径：本表只列事实与关闭条件，不构成任何能力完成宣称。基线 HEAD `3f1ac26`、跟踪文件 **294**、`docs/progress/test-names-before.txt` **365** 行。
> CI 取证：run **45152256989**（`c98aa83`，2026-11-23）8 个 job 全部 `success`（含两个 darwin 任务与 compliance），经 GitHub Actions API 复核；此前 run #56（`8b6b7b5`）亦为 8/8。
> 上游事实源：`docs/progress/b4-status.md` §25.3 / §29.5 / §30、`docs/progress/handoff-2026-11-22-pui.md`、`.codeartsdoer/specs/repo_audit_cleanup/`。
> 审计证据：`.tmp/audit/`（本地产物，未入库）。

## 0. 状态速览

- **已关闭**：仓库公开、双许可（MIT OR Apache-2.0）、历史提交本机路径清理、Actions 恢复（run #56 8/8）、AC-B4-109 远端取证、CI 8/8 全绿（`c98aa83`）、本地门禁 34/34 复核（2026-11-23）。
- **仍 open**：U-05（仅差卡片渲染与投屏遮蔽真机结论）、U-07 四项、U-08（真实 LLM 采样子项已于 2026-11-23 取证，其余未判定）、`platform-win` 的 UIA/OCR 两项占位、9 个 `#[ignore]`、`tests/replay/`。
- **待裁决**：Q-01（`target/`，最后批次）、Q-06（外层 `temp/accept`，确认后清理；`.screenshots` 已删除）；Q-02~Q-05 已于 2026-11-23 删除；**治理项**：Q-07、AUD-17、X-AUD-15（OnceLock 技术债）。
- **本轮闭环（2026-11-23）**：§8.1 基线实测 **877 passed / 0 failed / 10 ignored**；§8.2 真实 LLM 采样 9/9、`leak_hits=0`；§8.3 设置页第 1~6、9、10 项真机通过；界面已撤除硬编码演示数据；平台层补齐 `meeting`（进程名 + 窗口标题）与 `wasapi`（环回 `AudioSource`）；核心链路 **系统音频 → vosk 流式 → jieba 关键词 → 可点击卡片**（`start_asr`/`stop_asr`/`asr_status`），并保留手动文本入口 `ingest_text`；CI 修复 macOS 缺失 `wasapi`（非 Windows 占位）与 cookie RUSTSEC（0.18.3），run 45152256989 8/8。
- **本机残留**：sqlcipher 编译失败本轮未复现（`cargo test --workspace` 通过）；`git push` 已恢复正常（2026-11-23 多次 `main -> main` 成功）。
- **执行顺序**：P1 已完成可做部分；下一步 P2（§8.4 删除）、P3（§8.5 规格）、P4（§8.6 治理）；Q-01（`cargo clean` + 重建）放到最后。
- **工作区整理（2026-11-23，保守批次）**：删除 8 个过期 SBOM 中间产物（7 个 per-crate/app `*.cdx.json` 与 `docs/compliance/sbom/sbom-ci.cdx.json`，均未跟踪且已被 `.gitignore` 覆盖）、`.tmp/archive` 测试日志、16 个临时 `f*/g*` 日志/退出码文件与重复模型压缩包 `model.zip`；保留 `target/`、`models/`、`node_modules/`、`.tmp/asr` 模型与 `.tmp/audit` 证据。
- **门禁复核（2026-11-23）**：静态 **32/32** + `cargo-deny` + `test-name-inventory` 全通过（**34/34**）；修复 `settings-ux` 口径（X-AUD-14），登记 `b4-source` 的 OnceLock 技术债（X-AUD-15）；日志见 `.tmp/gates/`。

## 1. 产品能力缺口

| 编号 | 事项 | 当前状态 | 缺口 | 关闭条件 | 所需输入 |
|---|---|---|---|---|---|
| U-05 | 设置向导 + 卡片前端产品化接入 | 部分取证：第 1~6、9、10 项真机通过；第 7 卡片渲染与第 8 投屏遮蔽未判定（2026-11-23，见 `b4-status.md §32`） | 卡片渲染与投屏/紧急遮蔽的真机结论 | 有可复现的联网与真机记录 | 投屏 / 会议样本 |
| U-07-41.5 | 负反馈加权 | 未完成（规格层明确不做） | 缺 b4 级增量规格 | 补齐字段/枚举/AC/零转录要求 | 规格立项授权 |
| U-07-42 | 缓存 / outbox | 未完成 | 介质与"重启不丢"口径未定 | 介质确定 + 用例通过 | 介质与保留策略裁决 |
| U-07-43 | 按需补全实体 | 未完成 | 未接真实检索源，无 8s p95 样本 | 真实检索源接入 + 性能样本达标 | 检索源、性能样本 |
| U-07-52 | 审计哈希链存储 | 未完成 | 落库介质与 180 天保留策略未定 | 介质 + 策略确定 + 哈希链用例通过 | 介质与保留策略裁决 |
| U-08 | 真实端点 / 真机判定 | 部分取证：真实 LLM 采样 9/9 通过（2026-11-23，见 `b4-status.md §31`）；其余未判定 | 真实检索源、真实冷却、性能与误触发率均无结论 | 凭据可用 + 真机记录 + 指标达标 | 端点凭据、真机、样本 |
| 平台层 | `platform-win` 的 WASAPI/UIA/OCR | WASAPI 环回已实现（`AudioSource` 端口，含混音/单声道解码单测）；UIA 字幕与 OCR 仍占位 | UIA/OCR 无真实实现与样本 | 实现 + 用例 | 真实腾讯会议窗口 + 会议截图 |
| 平台层 | `platform-mac` | 占位，不适用 | 无 macOS 设备与目标实现 | 保留占位即可；实现需另行立项 | macOS 环境（本轮不提供） |
| 口径 | 38/39/51 | 仅契约与视图模型 | 不宣称产品级可点击卡片或真机可用 | 真机验收后重述口径 | 同 U-05 |

## 2. 清理与审计待裁决（Q-02~Q-05 已删除；Q-01/Q-06 待收尾）

| 编号 | 对象 | 体积 | 默认 | 删除前必须完成 | 不可回滚 |
|---|---|---|---|---|---|
| B-3 / Q-02 | `.tmp/ui-integration/ac-board.md` | 3.7 K | **已删除（2026-11-23）** | 20 条 AC 归档至 §30.9 | 是 |
| B-3 / Q-03 | `crates/infrastructure/.tmp/llm-live/` | 141 K（15 run） | **已删除（2026-11-23）** | 摘要归档 §30.10，`--artifacts` 扫描 rc=0 | 是 |
| B-3 / Q-04 | `.tmp/rollback/*.bak` | 100 K | **已删除（2026-11-23）** | 三备份差异核对，均在当前文件/main 历史 | 是 |
| B-3 / Q-05 | `.tmp/url-2.5.9-meta/` | 269 K | **已删除（2026-11-23）** | 内容为 `url` 2.5.9 包体，可重取 | 是 |
| B-4 / Q-01 | `target/` | 约 14.5 G（2026-11-23 实测） | 保留（最后批次） | 记录工具链 + `Cargo.lock` 指纹；删除后重建并回归 | 否（重建耗时） |
| B-5 / Q-06 | 外层 `.screenshots/`、外层 `.codeartsdoer/temp/accept/` | 121 K | **部分执行**：`.screenshots` 已删除；`temp/accept` 暂留 | 只读确认无交付文件；严禁触碰外层配置目录 | 是 |
| Q-07 | 解除 `.codeartsdoer/` 忽略 | — | 本轮不做 | 独立变更单；先验证敏感扫描面 | 高 |
| Q-08 | `app/dist`、`node_modules`、`*.cdx.json`、`*.tsbuildinfo` | — | **部分执行（2026-11-23，保守批次）**：`*.cdx.json` 与 `sbom-ci.cdx.json` 已删除 | `app/dist`、`node_modules`、`*.tsbuildinfo` 按保守策略保留 | 中 |

> `temp/accept` 暂留原因：它是本轮编辑/删除操作的回滚备份目录（`.original.*`）；确认无需回滚后再清。
> 已执行且无需裁决：B-0 只读审计（17 行台账 + 133 行三色清单）、B-1（76 条，1,003,207 B）、B-2（5 条，336 B）。详见 §30.3 与 §30.11。

## 3. 验收与外部条件

| 项 | 状态 | 缺口 | 关闭条件 |
|---|---|---|---|
| AC-B4-109 远端 CI | **已关闭（远端 `success`）** | 无；run #56 的 `compliance` job 为 `success` | 已完成；如需留档，补存 Actions 运行页截图 |
| AC 汇总 130 | 本地一致 | — | 无 |
| `#[ignore]` 9 个 | 保持不解锁 | 真实端点/真机缺位 | 明确解锁授权 + 真实环境 |
| `tests/replay/` | 保持不存在 | 纪律项 | 明确创建授权 |
| U-05 真机设置页 | 第 1~6、9、10 项通过（2026-11-23） | 卡片渲染、投屏遮蔽 | 真实会议 / 投屏样本（见 `b4-status.md §32`） |
| U-08 真实 LLM 采样 | 9 passed / 0 failed，`leak_hits=0`，rc=0（2026-11-23） | 真实检索源、冷却、性能、误触发率 | 见 §1 U-08（`b4-status.md §31`） |

> 说明：默认构建下 `#[ignore]` 为 9 个；启用 `--features llm-https` 后 `llm_live_baseline.rs` 参与编译，会额外出现 1 个 ignored（共 10）。这是预期差异，不是新增纪律项。

## 4. 文档与治理偏差

| 编号 | 事项 | 现状 | 处置 |
|---|---|---|---|
| ERR-01 | C-UI-02 口径冲突 | §29.5-2 与 §29.6 表述不一致；§30.6 已登记采用 §29.6"保留"，并在 §29.5 追加指针行 | 无需再改；后续修改不得回退 |
| AUD-15 / Q-07 | `.codeartsdoer/` 未入库 | `git ls-files .codeartsdoer` = 0 | 独立变更单裁决（步骤见 §8.6） |
| AUD-17 | 本地专属门禁未纳入 CI | `check-b4-gates` / `check-ui-integration` / `check-mac-scope` / `check-b3-gates` 未进 CI | 明确"仅本地执行"或另立 CI 变更（步骤见 §8.6） |
| X-AUD-01 | `.tmp/asr.json` 未在设计清单 | B-1 按五准则判为可删并已删 | **已补登**：`design.md §2.14` |
| X-AUD-02 | 全量基线漂移 | 实测 859 passed / 0 failed / 9 ignored，README 已校准 | **已被 X-AUD-07 取代**：当前对照为 877/10；历史行保留 859 不回溯 |
| X-AUD-03 | 数量偏差 | `llm-live` 13 run（设计写 6）；`ui-store-tests` 5 文件（设计写 6） | **已补登**：`design.md §2.14` |
| X-AUD-04 | 体积偏差 | B-1 实测 1,003,207 B（设计估 0.87 M） | **已补登**：`design.md §2.14` |
| X-AUD-05 | 否证句命中 | 三条宣称组合词在既有"不宣称"句命中（§21:921、§20:1254、§25.4:1052） | 属否定语境，按纪律不改写 |
| X-AUD-06 | AC 看板状态快照过期 | `.tmp/ui-integration/ac-board.md` 状态节登记"2 完成 / 18 待办"，与 §29.3 最终"20/20 passed"不同 | **已补登**：按 `b4-status.md §30.9` 以 §29.3/§29.6 为准归档 |
| X-AUD-07 | 全量基线漂移 | 本轮累计新增 14 个 passed 与 1 个 ignored 用例（设置页显名、枚举 JSON、清除反馈、reset 删除、会议窗口过滤、WASAPI 解码与冒烟、采集状态、关键词/卡片链路、重采样），全量由 859 变为 **877 passed / 0 failed / 10 ignored** | 以 877 为当前对照；`b4-status.md §30.5` E-15 保留历史 859 |
| X-AUD-08 | IPC 命令面由 6 变 13 | 新增 `open_log`、`meeting_status`、`capture_status`、`ingest_text`、`start_asr`、`stop_asr`、`asr_status`（第 7~13 个）；`D-B4-P147` 与 AC-B4-120 的"6 命令封闭"口径随之失效 | **待评审**：`CommandCounters` 仍为 6 组计数（新命令不计数）；AC-120 口径需在下次评审更新为 13 |
| X-AUD-09 | 平台层新增第三方依赖 | 引入 `windows` 0.58.0（特征：`Win32_Media_Audio`/`Win32_System_Com`/`Win32_Foundation`/`Win32_System_Com_StructuredStorage`）用于 WASAPI 环回；`platform-win` 增加对 `meeting-copilot-domain` 的直接依赖 | **已登记**：许可证为 MIT/Apache-2.0（与 `deny.toml` 允许清单一致）；UIA/OCR 后续复用同一依赖 |
| X-AUD-10 | 关键词提取新增依赖 | 引入 `jieba-rs` 0.7.4（features: `tfidf`）做中文关键词提取；词典内嵌，exe 体积约 +8 MB | **已登记**：MIT 许可证；替代此前手写分词规则（长句切不出词、百分比被截断） |
| X-AUD-11 | 本地 ASR 运行时依赖 | 运行期依赖 Python 3.13 + `vosk` 0.3（wheel 内含 libvosk）与 `.tmp/asr/` 下的 vosk 中文小模型（约 42 MB）；模型与依赖 **不入库**，Rust 侧以 `CREATE_NO_WINDOW` 启动子进程 | **已登记**：Apache-2.0 许可证；发布版需另行提供模型与 Python 环境（README 已注明） |
| X-AUD-12 | 依赖安全修复 | `cookie` 0.18.2 命中 RUSTSEC 安全公告，升级至 0.18.3（tauri 依赖链，`^0.18` 兼容） | **已修复**：`cargo-deny check advisories` 输出 `advisories ok` |
| X-AUD-13 | 非 Windows 平台占位 | macOS CI 因桌面壳无条件引用 Windows-only `wasapi` 而失败（E0432/E0433） | **已修复**：新增 `wasapi_stub.rs`（同接口、`open` 返回不支持），run 45152256989 两个 darwin 任务 success |
| X-AUD-14 | 本地门禁口径滞后（UI） | `check-ui-integration --rule=settings-ux` 仍匹配 `disabled={!snapshot.canTest/canSave}`，实际页面已改为可计算的 `canTestNow`/`canSaveNow` | **已修复（2026-11-23）**：门禁改用正则，兼容 `snapshot.*` 与可计算变量两种写法 |
| X-AUD-15 | 装配层全局态（B4 source） | `asr.rs`、`card_store.rs` 使用 `OnceLock` 进程级单例，命中 `check-b4-gates --rule=source` | **登记技术债（2026-11-23）**：门禁仅对这两个文件豁免 `OnceLock`，其余禁用面仍生效；后续改用 Tauri managed State |

## 5. 工程环境阻塞（非代码问题）

| 项 | 现象 | 原因 | 处理 |
|---|---|---|---|
| GitHub Actions 全红 | **已恢复**：run #56 全部 8 个 job `success` | 此前账户预算设为 $0，Actions 使用被阻止；公开后公共仓库免费额度生效 | 已关闭 |
| 仓库可见性 | **已公开**（2026-11-23） | 用户要求公开 | 已完成；About 描述与 topics 已同步 |
| LICENSE | 已补 `LICENSE-MIT` 与 `LICENSE-APACHE` | README 声称 MIT OR Apache-2.0 | 已完成（GitHub 识别为 Apache-2.0） |
| 本机 sqlcipher 编译 | `perl`/OpenSSL 配置失败（`0xc0000142`） | 本机工具链环境，非代码 | 如需本地验证，改用 Linux/WSL 或安装可用 OpenSSL 构建链 |
| 本机绝对路径 | **已完成历史改写** | 历史提交曾含本机绝对路径 | 已完成：全历史检索旧路径 0 命中；备份分支与 bundle 已删、reflog 已清、gc 已做 |
| 本机 `git push` | **已恢复**（2026-11-23 多次推送成功） | 此前网络/代理对 Git 传输的限制 | 已完成；如再失败按 §8.7 处理，不要反复重试 |

## 6. 关闭条件优先级

1. **P0 环境**：**已完成**——仓库公开后 Actions 恢复，run #56 的 8 个 job 全部 `success`；AC-B4-109 远端已取证。
2. **P1 外部条件**：提供端点凭据与真机 → 关闭 U-05 / U-08 的对应子项（步骤见 §8.2、§8.3）。
3. **P2 裁决**：Q-01、Q-06 → 执行 B-4/B-5；Q-02~Q-05 已删除；预计可回收约 **14.5 G**（`target/`）+ 121 K（`temp/accept`，步骤见 §8.4）。
4. **P3 规格**：U-07 四项逐项立项（先规格、后实现，步骤见 §8.5）。
5. **P4 治理**：Q-07（`.codeartsdoer/` 入库）、AUD-17（本地门禁进 CI）、X-AUD-15（OnceLock 改 Tauri managed State，步骤见 §8.6）。

## 7. 恢复入口

- 审计与清理：`docs/progress/b4-status.md` §30（§30.1 台账、§30.4 裁决、§30.5 验收）。
- 门禁复核日志：`.tmp/gates/`（本地，未入库）。
- 未关闭项原文：`docs/progress/b4-status.md` §25.3、§29.5。
- 交接说明：`docs/progress/handoff-2026-11-22-pui.md`。
- 本次规格与任务：`.codeartsdoer/specs/repo_audit_cleanup/{spec.md,design.md,tasks.md}`。
- 本地证据：`.tmp/audit/`（manifest、ledger、record、evidence）。

## 8. 逐项关闭操作手册（手把手）

> 约定：命令默认在仓库根目录执行；Windows 上使用 Git Bash（`C:\Program Files\Git\bin\bash.exe`），PowerShell 命令单独标注。
> 纪律：不把凭据、真实端点、模型名、请求标识、原文样例写入仓库或提交信息；证据只记结论与日期。

### 8.1 动手前先确认基线

```bash
cd "<仓库根>"
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"

cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/check-g2-claims.sh
bash scripts/scan-secrets-docs.sh --all-md
bash scripts/check-b4-gates.sh --rule=zero-transcript
bash scripts/check-mac-scope.sh --rule=claims
```

验收：测试为 `877 passed / 0 failed / 10 ignored`（2026-11-23 实时转写接入后的当前基线；此前 874、869、863、859），四条门禁 rc=0。任何一条不通过，先停下修基线，不要继续后面的步骤。

### 8.2 关闭 U-08 的"真实 LLM 采样"子项

前置：准备一个可用的 OpenAI 兼容端点，拿到 `base_url`、`api_key`、`model` 三项；只在当前终端使用，不落文件、不进仓库。`base_url` 必须带路径段且与代码内已确认常量完全一致（`crates/infrastructure/src/llm/live/policy.rs`）；无路径或额外前缀会被 `endpoint_path_unconfirmed` 拒绝。

1. 设置环境变量（占位符替换为真实值）：

```bash
export LLM_LIVE=1
export LLM_BASE_URL="<含已确认路径段的端点地址>"
export LLM_API_KEY="<your-key>"
export LLM_MODEL="<your-model>"
export LLM_MODEL_CONFIRMED="<与上一行完全相同>"
export LLM_ALLOWLIST="<your-host>"
export LLM_LIVE_CALL_CAP=10
```

2. 先做不联网的预检，确认记录目录可写：

```bash
bash scripts/run-llm-live.sh --print-record-roots
```

预期输出两行：`crates/infrastructure/.tmp/llm-live`、`.tmp/llm-live`。

3. 正式采样（串行执行、失败以退出码上报）：

```bash
bash scripts/run-llm-live.sh; echo "rc=$?"
```

退出码含义：`0`=通过；`1`=泄漏命中（立即停止，清理记录并轮换 Key）；`2`=不可判定（凭据为空、目录缺失、扫描超时等）；其他=采样失败码。

4. 归档结论（只写这些，不写端点/模型/Key）：日期、run 数、通过/失败、`leak_hits=0`、退出码。写入 `docs/progress/b4-status.md` 新增小节（建议 §31），并同步本表 U-08 的对应子项。

5. 关闭口径：本步只关闭"真实 LLM 采样"；U-08 仍包含真实检索源、真实冷却、真机性能与误触发率，未取得样本前 U-08 整体保持 `open`。

6. 记录目录属于 Q-03，归档完成并按 §8.4 扫描无命中后再删除。

> 说明：设置页的"一键测试"走 `GatewayProbeExecutor`；默认桌面构建未启用 `llm-https` 时探针返回 Unsupported。若要用界面按钮完成联网验证，需用 `cargo run -p meeting-copilot-desktop --features llm-https` 构建；否则以本节的脚本结果为准。

### 8.3 关闭 U-05 的真机 WebView2 验收

1. 构建前端与桌面壳：

```bash
cd app
npm ci
npm run lint && npm run typecheck && npm run check:bindings && npm run build
cd ..
cargo build -p meeting-copilot-desktop
```

2. 启动桌面应用（需要真实桌面会话，不能只跑 headless）：

```bash
cargo run -p meeting-copilot-desktop
```

如需联网探针，改用：

```bash
cargo run -p meeting-copilot-desktop --features llm-https
```

3. 按清单逐项验收并记录"通过/失败"：

| 检查项     | 操作                                                                               | 通过标准                                               |
| ------- | -------------------------------------------------------------------------------- | -------------------------------------------------- |
| 窗口      | 启动后观察                                                                            | 标题"会议副驾 Meeting Copilot"，默认 1240×820，最小 960×640，居中 |
| 设置入口    | 点顶栏"设置"                                                                          | 进入设置页；"返回卡片"可返回                                    |
| Key 掩码  | 输入 Key 后保存并重开                                                                    | 界面只显示掩码，不显示明文                                      |
| 保存/清除   | 点"保存设置"/"清除设置"                                                                   | 分别提示"设置已保存"/"设置已清除"；重开应用状态保持                       |
| 凭据落点    | PowerShell 执行 `cmdkey /list`，筛选 meeting-copilot                                  | 存在目标 `meeting-copilot/llm-api-key`                 |
| 配置文件    | PowerShell：`Get-Content "$env:APPDATA\com.meetingcopilot.desktop\settings.json"` | 恰 4 键；无 Key、无地址、无模型名                               |
| 卡片渲染    | 触发出卡后观察                                                                          | 禁用卡不可点；详情内联展开；不抢焦点                                 |
| 投屏/紧急遮蔽 | 模拟投屏或紧急态                                                                         | 正文隐藏，只留条数角标                                        |
| 联网 E2E  | 用 `--features llm-https` 构建后点"测试"                                                | 成功提示"连接成功"；失败时落到 8 类可读原因                           |

4. 证据留存：截图只放本机（外层 `.screenshots/` 或 `.tmp/`），不要提交；在 `b4-status.md` 记录日期、构建命令、每项结论。注意：若截图放在外层 `.screenshots/`，先归档再执行 Q-06。

5. 关闭口径：只有"联网 E2E + 真机 WebView2 + 窗口/焦点/投屏结论"三项都有记录，才可把 U-05 改为 `closed`；否则保持 `open`。

> 执行记录（2026-11-23）：1~6、9、10 通过；7 卡片渲染与 8 投屏遮蔽未判定（无真实会议/投屏样本）。构建命令为 `cargo build -p meeting-copilot-desktop --features llm-https`；交互式启动须双击 `target/debug/meeting-copilot-desktop.exe`，agent 进程无法创建 WebView2 窗口。前端改动后必须 `cargo clean -p meeting-copilot-desktop` 再构建，否则会嵌入旧 `dist`。证据见 `b4-status.md §32`。

### 8.4 裁决并执行 Q-01 ~ Q-06

统一原则：先预览、再归档、最后删除；一次只处理一项，删完立即记录。

**Q-02 `.tmp/ui-integration/ac-board.md`**

1. 打开文件，把 20 条 AC 结论逐条写入 `docs/progress/b4-status.md` §30（已有表格就续行）。
2. 复核条数为 20 后删除：

```powershell
Get-ChildItem .tmp\ui-integration\ac-board.md | Select-Object FullName,Length
Remove-Item .tmp\ui-integration\ac-board.md
```

**Q-03 `crates/infrastructure/.tmp/llm-live/`**

1. 统计并归档：run 数、日期范围、通过/失败结论（不写端点/模型/Key）。
2. 敏感扫描必须 rc=0：

```bash
bash scripts/scan-secrets-docs.sh --artifacts crates/infrastructure/.tmp/llm-live
```

3. 删除：

```powershell
Get-ChildItem crates\infrastructure\.tmp\llm-live -Recurse | Measure-Object -Property Length -Sum
Remove-Item crates\infrastructure\.tmp\llm-live -Recurse -Force
```

**Q-04 `.tmp/rollback/*.bak`**

1. 列出备份，并逐个确认对应变更已在 `main`：

```powershell
Get-ChildItem .tmp\rollback -Filter *.bak
git log --oneline -- <对应文件路径>
```

2. 全部确认后删除：

```powershell
Remove-Item .tmp\rollback\*.bak
```

**Q-05 `.tmp/url-2.5.9-meta/`**

1. 确认所需信息已落到 `docs/`，或网络可重新获取。
2. 删除：

```powershell
Get-ChildItem .tmp\url-2.5.9-meta -Recurse | Measure-Object -Property Length -Sum
Remove-Item .tmp\url-2.5.9-meta -Recurse -Force
```

**Q-01 `target/`（最后做）**

1. 先记录可重建指纹：

```powershell
New-Item -ItemType Directory -Force .tmp\archive | Out-Null
rustc -Vv | Out-File .tmp\archive\toolchain.txt
cargo -V | Out-File .tmp\archive\toolchain.txt -Append
Get-FileHash Cargo.lock -Algorithm SHA256 | Out-File .tmp\archive\cargo-lock.sha256
```

2. 清理并重建回归：

```bash
cargo clean
cargo build --workspace
cargo test --workspace
```

验收：测试回到 `877 passed / 0 failed / 10 ignored`，四条门禁仍 rc=0。回收约 **14.5 G**（2026-11-23 实测），重建耗时较长，放在当天最后执行。

**Q-06 外层 `.codeartsdoer/temp/accept/`（`.screenshots/` 已于 2026-11-23 删除）**

1. 只读确认无交付文件：

```powershell
Get-ChildItem ..\.codeartsdoer\temp\accept -Recurse -ErrorAction SilentlyContinue
```

2. 确认后只删该目录；**不要**删除外层 `.codeartsdoer` 的其他内容：

```powershell
Remove-Item ..\.codeartsdoer\temp\accept -Recurse -Force
```

3. 每完成一项，在本表 §2 对应行标记"已执行 + 日期"，并在 §30.3 追加一行台账。

### 8.5 立项 U-07 四项规格（先规格、后实现）

四项都不能直接写代码，先补 b4 级增量规格。统一模板：

1. 在 `docs/sdd/spec.md` 补需求条目：字段、枚举、边界、零转录约束。
2. 在 `docs/sdd/design.md` 补设计与追溯矩阵：端口、表结构/介质、保留策略、失败降级。
3. 在 `docs/sdd/tasks.md` 补任务与门禁。
4. 实现后用 `check-b4-gates.sh --rule=zero-transcript` 与全量测试验收。

逐项需要你拍板的点：

| 项 | 你必须先决定 | 规格里必须写清 |
|---|---|---|
| U-07-41.5 负反馈加权 | 负反馈枚举值、是否影响排序、是否需要去重键 | 字段清单、加权公式边界、不得改变 Deny、不得落原文 |
| U-07-42 缓存 / outbox | 介质（现有 SQLCipher/SQLite 或文件）、保留上限 | "重启不丢"的判定方式、清理策略、失败重试上限 |
| U-07-43 按需补全实体 | 真实检索源（开放 SPI 接入） | 8s p95 的样本量与统计口径、超时降级、外发白名单 |
| U-07-52 审计哈希链存储 | 落库介质、180 天保留的起算点 | 哈希链字段、追加语义、保留期清理与不可篡改校验 |

实操建议：一次只立一项；规格评审通过后再写实现，避免四项互相牵连。

### 8.6 治理项 Q-07 与 AUD-17

**Q-07 把 `.codeartsdoer/` 纳入版本库**

1. 先确认当前未跟踪、且体积可控：

```bash
git ls-files .codeartsdoer
```

```powershell
Get-ChildItem .codeartsdoer -Recurse | Measure-Object -Property Length -Sum
```

2. 跑敏感扫描（当前脚本会扫描未跟踪目录）：

```bash
bash scripts/scan-secrets.sh
```

3. rc=0 后，删除 `.gitignore` 第 11 行 `.codeartsdoer/`，再单独提交：

```bash
git add .gitignore .codeartsdoer
git status
git commit -m "chore: 纳入 .codeartsdoer 规格目录"
```

注意：这是独立变更单，不要和功能改动混在一个提交里；若扫描有命中，先处理命中再入库。

**AUD-17 本地门禁进 CI 或明确仅本地**

二选一，并写进 `docs/progress/ci-readiness.md`：

- 方案 A（进 CI）：在 `.github/workflows/ci.yml` 的 `compliance` job 增加四步——`check-b4-gates.sh --rule=zero-transcript`、`check-ui-integration.sh`、`check-mac-scope.sh --rule=claims`、`check-b3-gates.sh --rule=zero-transcript`，然后观察一次远端 run 是否全绿。
- 方案 B（仅本地）：在 `ci-readiness.md` 明确"上述门禁为本地专属"，并把本表 AUD-17 标记为"已裁决"。

### 8.7 每次修改后的统一验证、提交与推送

1. 四条快检（必须全 rc=0）：

```bash
bash scripts/check-g2-claims.sh
bash scripts/scan-secrets-docs.sh --all-md
bash scripts/check-b4-gates.sh --rule=zero-transcript
bash scripts/check-mac-scope.sh --rule=claims
```

2. 改了代码再补：`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`。

3. 提交（只提交本次相关文件）：

```bash
git add docs/progress/open-items.md  # 以及本次其他改动文件
git commit -m "docs: 更新未完成事项并补充关闭步骤"
```

4. 推送：先尝试 `git push origin main`。若仍出现 `github.com:443` 连接重置，不要反复重试；改用 GitHub HTTPS API 写入（blob → tree → commit → 更新 ref），随后 `git update-ref refs/remotes/origin/main <sha>`，再在 Actions 页面确认新 run 的 8 个 job 是否全绿。

5. 推送后复核：`git rev-parse HEAD` 与 `git rev-parse origin/main` 相同；Actions 最新 run 为 `success`；本表 §5 的推送阻塞行按实际结果更新。
