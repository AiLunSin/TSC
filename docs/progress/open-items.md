# 未完成事项总表（2026-11-23）

> 口径：本表只列事实与关闭条件，不构成任何能力完成宣称。基线 HEAD `48f0725`（U-07 后）、跟踪文件 **363**、`docs/progress/test-names-before.txt` **365** 行。
> CI 取证：run **45152256989**（`c98aa83`，2026-11-23）8 个 job 全部 `success`（含两个 darwin 任务与 compliance），经 GitHub Actions API 复核；此前 run #56（`8b6b7b5`）亦为 8/8。
> 上游事实源：`docs/progress/b4-status.md` §25.3 / §29.5 / §30、`docs/progress/handoff-2026-11-22-pui.md`、`.codeartsdoer/specs/repo_audit_cleanup/`。
> 审计证据：`.tmp/audit/`（本地产物，未入库）。

## 0. 状态速览

- **已关闭**：仓库公开、双许可（MIT OR Apache-2.0）、历史提交本机路径清理、Actions 恢复（run #56 8/8）、AC-B4-109 远端取证、CI 8/8 全绿（`c98aa83`）、本地门禁 34/34 复核（2026-11-23）、Q-01/Q-06 清理与重建回归（2026-11-23）、Q-07 入库、AUD-17 门禁进 CI、X-AUD-15 OnceLock 改造（2026-11-23）、U-07-52 归档保留期（2026-11-24，方案 B：5 年，个人自用简化确认）、**U-07-43 真实 200 样本达标关闭（2026-11-24，`b4-status.md §50`）**、**U-05 设置向导 + 卡片前端关闭（2026-11-25，第 7 项 4/4、第 8 项 3/3，见 `b4-status.md §53`）**。
- **仍 open**：U-08（真实 LLM 采样子项已于 2026-11-23 与 2026-11-24 两次取证，AC-20/AC-22 基线 120 样本 success=1.0；其余未判定）、U-09（A~F 已实现并推送 `f3d66a4`，H-34-G 手工验收入口已就绪，待真实端点验收）、`platform-win` 的 UIA 字幕适配器已实现并与采集状态接线（2026-11-24；腾讯会议/飞书窗口 UIA 元素数为 0，真实字幕文本未取得，保持 open）、OCR 已实现（前台会议窗口底部区域，2026-11-24；未在真实会议字幕上验证）、11 个 `#[ignore]` 待真实端点（原 12 个中的 WASAPI 本机冒烟已于 2026-11-25 跑通，含 H-34-G 的两个真实源入口仍未跑）。
- **待裁决**：Q-01、Q-06 已于 2026-11-23 执行关闭；Q-02~Q-05 已于 2026-11-23 删除；治理项 Q-07、AUD-17、X-AUD-15 已于 2026-11-23 执行。
- **本轮闭环（2026-11-23）**：§8.1 基线实测 **877 passed / 0 failed / 10 ignored**；§8.2 真实 LLM 采样 9/9、`leak_hits=0`；§8.3 设置页第 1~6、9、10 项真机通过；界面已撤除硬编码演示数据；平台层补齐 `meeting`（进程名 + 窗口标题）与 `wasapi`（环回 `AudioSource`）；核心链路 **系统音频 → vosk 流式 → jieba 关键词 → 可点击卡片**（`start_asr`/`stop_asr`/`asr_status`），并保留手动文本入口 `ingest_text`；CI 修复 macOS 缺失 `wasapi`（非 Windows 占位）与 cookie RUSTSEC（0.18.3），run 45152256989 8/8。
- **本轮闭环（2026-11-24）**：平台层新增 UIA 字幕与 OCR 兜底（提交 `9c6ca08`）；`meeting` 改为窗口驱动（`OpenProcess` + `QueryFullProcessImageNameW`），修复 Toolhelp 快照漏掉 64 位主进程；OCR 默认关闭（`OCR_FALLBACK=1` 开启），ASR 仍为主链路；新增开发期隐私注入入口 `PRIVACY_SCREEN_SHARE` / `PRIVACY_EMERGENCY_MASKED`（`app/src-tauri/src/privacy.rs`，默认关、只认 `"1"`、不加 IPC 命令），为设置页第 8 项备好取证手段；用一次性临时凭据跑通 `llm_live` 9 例与 120 样本基线（`b4-status.md §34`）；全量基线 **1000 passed / 0 failed / 12 ignored**，改动包 47（desktop）+ 27（platform-win，串行）通过，clippy/fmt 与四条门禁 rc=0。真实会议字幕文本未取得，U-05/U-08 保持 open。
- **本轮闭环（2026-11-24 续）**：把实时转写从"启动了却一直没文本"修到可用四步：① 脚本主动收句（partial 稳定 0.8s / 单句 4s）且 Rust 侧接收 partial（`6d5c2a2`）；② 会议命中即驱动字幕通道、识别异常不再静默（`46e0d6d`）；③ UIA 抽取加三层过滤，控件树不再冒充字幕，字幕文本与 ASR 文本分开存放（`a45adb2`）；④ 0 片显式报"未采到系统声音"、子进程 stderr 落 `asr.log`（`42abbf5`）；⑤ 完整转写 + 选中查义 + 释义出卡（`9d4c40f`）；⑥ 自动出卡默认关闭、界面加开关与"清空"（`3596a37`）；IPC 17→19 命令（`set_auto_cards`、`clear_cards`）。真机已确认：转写逐句出文本、选中可查义、卡片不再自动刷屏。桌面层单测 **60 passed**；clippy/fmt、B4 零转录、分层、UI 集成、G2 宣称、`check-bindings`(13 DTO)、tsc/eslint/vite build 全绿。CI：run #80（`46e0d6d`）success，run #81（`3596a37`）**success（3m29s，8 job）**。
- **本轮闭环（2026-11-24 收尾）**：① 释义按"多句/整段"分支（≥24 字且 ≥2 句走概括：≤180 字、概括 + 最多两个关键信息 + 术语半句解释；否则术语模式 ≤120 字）；② 跨句选中上下文改为"整段 → 最长句 → 最长连续词块"三级匹配，出现次数与上下文同口径；③ 自动出卡限流（同词 90 秒冷却 + 每分钟 8 张，手动路径不受限）；④ U-05 第 7/8 项取证改为**不依赖真实会议**并附判定表（§8.3）。desktop bins **70 passed**；clippy/fmt/四门禁/`check-bindings`/tsc/eslint/vite build 全绿；构建 16:41。
- **本轮新增：联网查证 + 辅助回复（2026-11-24）**：① 检索 Key 与模型 Key 分两个系统凭据目标，配置落 `search.json`（不含 Key）；② 联网查只走两家固定预设（博查/Tavily），只发选中词（≤80 字、8s 超时、前 5 条），默认手动触发，勾选"出卡自动补来源"后仅在点"生成卡片"时现查；③ 点过联网查的卡片引用标 `GeneralWeb`（ID 带站点），来源非法时退回本机引用；④ 新命令 `assist_reply`：最近 12 句 + 选中句 → 2~3 条提醒 + 1~2 条可直接说的话，JSON 解析失败不装成功。IPC 19→23；desktop bins **90 passed**；platform-win 30 passed；两套 clippy、fmt、六门禁、`check-bindings`(17 DTO)、tsc/eslint/build 全绿。**未决**：真实检索 Key 下的召回质量与配额、外链点击打开、辅助回复跨话题时的上下文范围（现固定最近 12 句）均待真机手测。
- **真机截图复核（2026-11-24 17:00）**：截图暴露四处"链路通但结果不对"，均已修：① 卡片恒「引用已过期/不可点击」——`pending_view()` 把 `session_matches` 硬编码 `false`，改为 `active_view(privacy)`，并把隐私态注入 `body_visible`；② 卡片标签直接显示 `proper_noun`，前端补中文映射；③ vosk 中文词间空格原样入库，新增 `tidy_cjk_spaces`（只在中文相邻时合上，英文短语保留）；④ 摘要把英文词切成 `educational c`，新增 `fit_text` 按句读收口 + 省略号。desktop bins **73 passed**（新增 3 例）；全部门禁与前端检查 rc=0；构建 16:55。
- **辅助回复上下文分层（2026-11-24 17:30）**：提醒窗口收到最近 5 句，回应仍看最近 12 句（≤900 字），两段在同一次请求里分标注（「更早的对话（只作背景，不要据此写提醒）」/「最近 5 句（提醒只依据这几句）」），不额外多花一次调用；不足 5 句时只给一段。面板小字同步为"提醒看最近 5 句，回应参考最近 12 句"。desktop bins **91 passed**（新增 1 例覆盖窗口切分），两套 clippy、fmt、门禁、`check-bindings`、tsc/eslint 全绿。**未决**：5/12 这组数在真实会议里是否够用，仍需手测。
- **主栏版式减密（2026-11-24 18:00）**：用户反馈"太密集"。中栏由 1 个 `panel` + `.settings-section` 分隔线拆成 4 张卡片（关键词卡片／联网查资料／完整转写／会议文本），联网查改为默认收起的折叠块（标题行显示状态），选中操作栏提到转写框正下方；面板与卡片内边距、转写行距整体加大；响应式断点 1080→1180px（窄窗口隐藏长期空白的"要点时间轴"，宽度让给主栏）。只动版式，未动 IPC/业务；前端 tsc/eslint/build、六门禁、`check-bindings` 全绿；无头 Edge 截图复核 1082/1440 两档宽度。产物 18:01。**未决**：真机 125%/150% 缩放下的手感。
- **检索限额与窗口可调（2026-11-24 18:40）**：用户要求"额度不用管、加限额""辅助回复让用户自己调"。① `search.json` 增 `hourlyLimit`（默认 20，0=不限，上限 200），计数落 `search-quota.json`（整小时窗口），先扣后出网，超限返回 `search_quota_exceeded`；② 新命令 `search_clear_key`（`SecureStore::delete`，配置保留），界面加"清除 Key"；③ 新 `assist.json` + `assist_config`/`assist_config_save`，提醒 3~10（默认 5）、上下文 5~20（默认 12），越界夹取且提醒不长于上下文；设置页新增"辅助回复"一节。IPC 23→**26**，DTO 17→**20**；desktop bins **93 passed**；两套 clippy、fmt、六门禁、`check-bindings`、tsc/eslint/build 全绿；产物 18:42。**未决**：真实 Key 下限额的计费口径（本地按次数，非 token）与真实召回质量仍待手测。
- **本轮闭环（2026-11-25）**：① 本机 WASAPI 音频冒烟 `wasapi::tests::loopback_opens_and_captures_when_audio_is_active` 显式 `--ignored` 跑通，1 passed / 0 failed，rc=0（播放 30 秒双声道正弦波，0.05s 即返回，低于 200×10ms 空转上限，证明首帧即捕获真实环回包，`sample_rate>0`、`sequence>0` 断言通过）；② 12 个 `#[ignore]` 全量盘点，源码字面 13 处，差异来自 `llm_live_baseline.rs` 的 `#![cfg(feature = "llm-https")]` 整文件门控（默认特性 12、开 `llm-https` 13）；③ 仓库根 `tests/replay/` 经授权创建（只放语料与取证登记，`crates/application/tests/replay/` 仍不存在，AC-B4-49/57/63 口径不变）；④ X-AUD-08 口径由 13 更正为 **26**；⑤ 清理 U-05 临时脚本与配置备份（证据截图保留）。详见 `b4-status.md §54`。
- **本机残留**：sqlcipher 编译失败本轮未复现（`cargo test --workspace` 通过）；`git push` 已恢复正常（2026-11-23 多次 `main -> main` 成功）。
- **执行顺序**：P1 已完成可做部分；P2 已完成（2026-11-23）；P3 规格与实现已完成（2026-11-23；U-07-43 真实 200 样本已于 2026-11-24 达标关闭，见 `b4-status.md §50`）；P4 治理已完成（2026-11-23，见 §8.6）；P5 检索源接入 A~F 已实现并推送（2026-11-24，`f3d66a4`），H-34-G 手工入口已就绪，待真实端点。
- **工作区整理（2026-11-23，保守批次）**：删除 8 个过期 SBOM 中间产物（7 个 per-crate/app `*.cdx.json` 与 `docs/compliance/sbom/sbom-ci.cdx.json`，均未跟踪且已被 `.gitignore` 覆盖）、`.tmp/archive` 测试日志、16 个临时 `f*/g*` 日志/退出码文件与重复模型压缩包 `model.zip`；当时保留 `target/`、`models/`、`node_modules/`、`.tmp/asr` 模型与 `.tmp/audit` 证据（`target/` 后于 Q-01 清理）。
- **门禁复核（2026-11-23）**：静态 **32/32** + `cargo-deny` + `test-name-inventory` 全通过（**34/34**）；修复 `settings-ux` 口径（X-AUD-14）；X-AUD-15 已改为 Tauri managed State 并移除 `b4-source` 豁免。

## 1. 产品能力缺口

| 编号 | 事项 | 当前状态 | 缺口 | 关闭条件 | 所需输入 |
|---|---|---|---|---|---|
| U-05 | 设置向导 + 卡片前端产品化接入 | **已关闭（2026-11-25）**：第 1~6、9、10 项真机通过（联网 E2E 见 `b4-status.md §32` 第 9 项 passed）；第 7 项 **4/4**（正文/可点、详情内联展开、清空归零三格真机截图；禁用卡一格按 `resolve_click_state` 六因单测 + `KeywordCard.tsx` 渲染分支判定，真机不可达的原因见 §8.3）；第 8 项 **3/3**（投屏 0 卡 + 「已生成 0 张卡片。」；紧急遮蔽 7 卡全遮蔽、`dark_text=0`、无详情面板），2026-11-25 三张真机截图，见 `b4-status.md §53` | 无 | 已满足 | — |
| U-07-41.5 | 负反馈加权 | **已实现并提交（`c96f381`）**：事件/权重表、加权与硬抑制、三维统计、门禁扫描面 | 无（真实使用数据由后续会议累积） | 已满足 | — |
| U-07-42 | 缓存 / outbox | **已实现并提交（`c96f381`）**：TTL/LRU、重启恢复、3 次退避、500 上限 | “≤3 秒识别断网、30 秒补齐率 ≥98%”需真实断网取证 | 真实断网演练记录 | 断网环境 |
| U-07-43 | 按需补全实体 | **已关闭（2026-11-24）**：机制 `c96f381`；四路径采样编排（A/B/C/D，D 为零账号零 Key 的本机桥 + 免账号 quick tunnel）；**三轮独立运行均 `real_samples=200/200`，p95 4461/4538/4759 ms、p99 5731/6789 ms、超时率 ≤0.61%**（见 `b4-status.md §50`） | 无（≥3 时段已满足） | 已满足 | — |
| U-07-52 | 审计哈希链存储 | **已实现并提交（`c96f381`）**：只追加表、触发器、哈希链、月度归档；G-52-D 运行时摘要扫描已接入；**已关闭（2026-11-24）** | 无（方案 B：在线 180 天 + 归档 5 年；个人自用简化确认） | 已满足 | — |
| U-08 | 真实端点 / 真机判定 | 部分取证：真实 LLM 采样 9/9 通过（2026-11-23，见 `b4-status.md §31`）；**2026-11-24 复测 9/9 + 基线 120 样本、4 任务 success_rate=1.0、0 degraded、`leak_hits=0`**（见 `§34`）；`ac23` 窗口内未触发限流 | 真实检索源、真实冷却、真机性能与误触发率均无结论 | 凭据可用 + 真机记录 + 指标达标 | 端点凭据、真机、样本 |
| U-09 | 检索源接入（任务 34 最小闭环） | **A~F 已实现（2026-11-24）**：`providers.json` 校验、注册表、健康分类、`http_json`、零源模式与按需 runner；H-34-G 手工入口 `search_live.rs` 已就绪并编译通过 | 真实端点与凭据；3 次手工检索；200 条真实样本 | 自动化用例已过；需真实端点验收 | 真实端点与凭据 |
| 平台层 | `platform-win` 的 WASAPI/UIA/OCR | WASAPI 环回已实现（`AudioSource` 端口，含混音/单声道解码单测），**本机环回冒烟 2026-11-25 显式 `--ignored` 跑通：1 passed / 0 failed，rc=0，首帧即捕获真实环回包**（§8.9-D）；**UIA 字幕已实现（2026-11-24）**：窗口探测、`SubtitleSource` 适配器、500ms 差量泵、文本直连 `CardStore` 出卡、采集状态投影；`meeting` 改为窗口驱动（`OpenProcess` + `QueryFullProcessImageNameW`），修复快照漏掉 64 位主进程；**OCR 已实现（2026-11-24）**：屏幕 DC 截取前台会议窗口底部 40% → `Windows.Media.Ocr` → 中文空格合并，作为 UIA 无文本时的回退；实测可识别窗口文字（腾讯会议首页 10 行、底带 20 行），未在真实会议字幕上验证 | 腾讯会议按自绘渲染，UIA 树为空；OCR 默认关闭（`OCR_FALLBACK=1` 开启），只在会议窗口位于前台时生效，底带可能混入按钮/菜单文字 | UIA 需真实会议且开启字幕取得文本；OCR 需真实会议字幕样本与 ROI 校准 | 真实腾讯会议窗口 + 会议截图 |
| 平台层 | `platform-mac` | 占位，不适用 | 无 macOS 设备与目标实现 | 保留占位即可；实现需另行立项 | macOS 环境（本轮不提供） |
| 口径 | 38/39/51 | 仅契约与视图模型 | 不宣称产品级可点击卡片或真机可用 | 真机验收后重述口径 | 同 U-05 |

> 本轮变更（2026-11-24）：`platform-win` 新增 `subtitle` 模块（UIA 窗口文本轮询 + TextPattern 回退 + 降级状态 + 500ms 差量泵），`meeting` 改为窗口驱动（`OpenProcess` + `QueryFullProcessImageNameW`，修复 Toolhelp 仅返回 32 位进程导致漏检 64 位主进程），飞书改为要求标题含会议关键词，`AsrPipeline` 启动时同步开启字幕通道并投影真实状态；新增 `ocr` 模块（屏幕 DC 截取前台会议窗口底部 40% → `Windows.Media.Ocr` → 中文空格合并），作为 UIA 无文本时的回退，实测整窗识别 10 行、底带 20 行；OCR 回退默认关闭（`OCR_FALLBACK=1` 开启），避免与开源 ASR 并行重复出卡；OCR 仅在前台窗口生效，未在真实会议字幕上验证。真实探测腾讯会议窗口 UIA 元素数为 0、未取得字幕文本，故 U-05/U-08 口径与 open 状态不变。改动包测试 43（desktop）+ 27（platform-win，串行）passed，全量 996/0/12，clippy 与门禁 rc=0。

## 2. 清理与审计待裁决（Q-02~Q-05 已删除；Q-01/Q-06 已执行）

| 编号 | 对象 | 体积 | 默认 | 删除前必须完成 | 不可回滚 |
|---|---|---|---|---|---|
| B-3 / Q-02 | `.tmp/ui-integration/ac-board.md` | 3.7 K | **已删除（2026-11-23）** | 20 条 AC 归档至 §30.9 | 是 |
| B-3 / Q-03 | `crates/infrastructure/.tmp/llm-live/` | 141 K（15 run） | **已删除（2026-11-23）** | 摘要归档 §30.10，`--artifacts` 扫描 rc=0 | 是 |
| B-3 / Q-04 | `.tmp/rollback/*.bak` | 100 K | **已删除（2026-11-23）** | 三备份差异核对，均在当前文件/main 历史 | 是 |
| B-3 / Q-05 | `.tmp/url-2.5.9-meta/` | 269 K | **已删除（2026-11-23）** | 内容为 `url` 2.5.9 包体，可重取 | 是 |
| B-4 / Q-01 | `target/` | 14.7 GiB（2026-11-23 实测） | **已执行（2026-11-23）**：`cargo clean` 删除 37,872 文件 | 指纹存 `.tmp/archive/`；重建、877/0/10、门禁 34/34 均通过 | 否（可重建） |
| B-5 / Q-06 | 外层 `.screenshots/`、外层 `.codeartsdoer/temp/accept/` | 121 K | **已执行（2026-11-23）**：25 个 `.original.*` 备份与空 `.screenshots` 已删除 | 已确认无交付文件；未触碰外层其他配置 | 是（已删） |
| Q-07 | 解除 `.codeartsdoer/` 忽略 | 43 文件 / 4.9 MB | **已执行（2026-11-23）**：`temp/`、`.codebase/` 继续忽略，specs/mcp/rule 入库；`scan-secrets` rc=0 | 已满足 | 已执行 |
| Q-08 | `app/dist`、`node_modules`、`*.cdx.json`、`*.tsbuildinfo` | — | **部分执行（2026-11-23，保守批次）**：`*.cdx.json` 与 `sbom-ci.cdx.json` 已删除 | `app/dist`、`node_modules`、`*.tsbuildinfo` 按保守策略保留 | 中 |

> `temp/accept` 已清：25 个 `.original.*` 回滚备份已确认无需回滚后删除（2026-11-23）。
> 已执行且无需裁决：B-0 只读审计（17 行台账 + 133 行三色清单）、B-1（76 条，1,003,207 B）、B-2（5 条，336 B）。详见 §30.3 与 §30.11。

## 3. 验收与外部条件

| 项 | 状态 | 缺口 | 关闭条件 |
|---|---|---|---|
| AC-B4-109 远端 CI | **已关闭（远端 `success`）** | 无；run #56 的 `compliance` job 为 `success` | 已完成；如需留档，补存 Actions 运行页截图 |
| AC 汇总 130 | 本地一致 | — | 无 |
| `#[ignore]` 默认特性 12 个（2026-11-25 复测）；其中 WASAPI 本机冒烟 1 个已跑通，余 11 个待真实端点 | 保持不解锁（不解锁 ≠ 不能显式 `--ignored` 运行） | 真实端点缺位 | 明确解锁授权 + 真实环境（逐类跑法见 §8.9） |
| `tests/replay/` | **仓库根目录已创建（2026-11-25，用户授权）**：仅语料与取证登记，见 `tests/replay/README.md`；`crates/application/tests/replay/` 仍不存在 | 无（AC-B4-49/57/63 针对后者，口径不变） | 已满足 |
| U-05 真机设置页 | 第 1~6、9、10 项通过（2026-11-23）；第 7 项 4/4、第 8 项 3/3（2026-11-25，`§53`） | 无 | 见 `b4-status.md §32`、`§49`、`§53` |
| U-08 真实 LLM 采样 | 两次通过：9 passed / 0 failed，`leak_hits=0`，rc=0（2026-11-23）；复测 9 passed + 基线 120 样本 / 4 任务 success=1.0 / 0 degraded（2026-11-24，`§34`） | 真实检索源、冷却、性能、误触发率 | 见 §1 U-08（`b4-status.md §31`、`§34`） |

> 说明：`#[ignore]` 数量以全量测试实测为准（`cargo test --workspace -- --list --ignored`，2026-11-25 复测默认特性 **12** 个）；源码字面 `#[ignore]` 共 **13** 处，差值来自 `llm_live_baseline.rs` 顶部的 `#![cfg(feature = "llm-https")]` 整文件门控——启用 `--features llm-https` 后 `baseline_30_samples_per_task` 才参与编译，此时为 **13** 个。逐类跑法见 §8.9。

## 4. 文档与治理偏差

| 编号 | 事项 | 现状 | 处置 |
|---|---|---|---|
| ERR-01 | C-UI-02 口径冲突 | §29.5-2 与 §29.6 表述不一致；§30.6 已登记采用 §29.6"保留"，并在 §29.5 追加指针行 | 无需再改；后续修改不得回退 |
| AUD-15 / Q-07 | `.codeartsdoer/` 未入库 | **已执行（2026-11-23）**：入库 43 文件；嵌套 `.gitignore` 改为仅忽略 `temp/`、`.codebase/` | 已关闭（步骤见 §8.6） |
| AUD-17 | 本地专属门禁未纳入 CI | **已执行（2026-11-23）**：`compliance` job 新增 b4-zero-transcript、ui-integration、mac-scope-claims、b3-zero-transcript 四步 | 已关闭（步骤见 §8.6） |
| X-AUD-01 | `.tmp/asr.json` 未在设计清单 | B-1 按五准则判为可删并已删 | **已补登**：`design.md §2.14` |
| X-AUD-02 | 全量基线漂移 | 实测 859 passed / 0 failed / 9 ignored，README 已校准 | **已被 X-AUD-07 取代**：当前对照为 877/10；历史行保留 859 不回溯 |
| X-AUD-03 | 数量偏差 | `llm-live` 13 run（设计写 6）；`ui-store-tests` 5 文件（设计写 6） | **已补登**：`design.md §2.14` |
| X-AUD-04 | 体积偏差 | B-1 实测 1,003,207 B（设计估 0.87 M） | **已补登**：`design.md §2.14` |
| X-AUD-05 | 否证句命中 | 三条宣称组合词在既有"不宣称"句命中（§21:921、§20:1254、§25.4:1052） | 属否定语境，按纪律不改写 |
| X-AUD-06 | AC 看板状态快照过期 | `.tmp/ui-integration/ac-board.md` 状态节登记"2 完成 / 18 待办"，与 §29.3 最终"20/20 passed"不同 | **已补登**：按 `b4-status.md §30.9` 以 §29.3/§29.6 为准归档 |
| X-AUD-07 | 全量基线漂移 | 本轮累计新增 14 个 passed 与 1 个 ignored 用例（设置页显名、枚举 JSON、清除反馈、reset 删除、会议窗口过滤、WASAPI 解码与冒烟、采集状态、关键词/卡片链路、重采样），全量由 859 变为 **877 passed / 0 failed / 10 ignored** | 以 877 为当前对照；`b4-status.md §30.5` E-15 保留历史 859 |
| X-AUD-08 | IPC 命令面由 6 变 **26**（2026-11-25 复核 `main.rs:170-197`） | 首次增补 `open_log`、`meeting_status`、`capture_status`、`ingest_text`、`start_asr`、`stop_asr`、`asr_status`（第 7~13 个）；其后又增 `set_auto_cards`、`transcript_snapshot`、`explain_selection`、`ingest_explanation`、`search_config`、`search_config_save`、`search_clear_key`、`web_search`、`assist_reply`、`assist_config`、`assist_config_save` 与 `clear_cards`、`ipc_version`，现注册 26 个入口；`D-B4-P147` 与 AC-B4-120 的"6 命令封闭"口径随之失效 | **待评审**：`CommandCounters` 仍为 6 组 12 项计数（`state.rs:17-30`，仅覆盖 `settings_snapshot/probe/save/clear` 与 `keyword_cards_snapshot/card_open`，其余 20 个命令不计数）；AC-120 口径需在下次评审更新为 26，并裁决计数面是否同步扩大。本条此前写"13"，2026-11-25 更正 |
| X-AUD-09 | 平台层新增第三方依赖 | 引入 `windows` 0.58.0（特征：`Win32_Media_Audio`/`Win32_System_Com`/`Win32_Foundation`/`Win32_System_Com_StructuredStorage`）用于 WASAPI 环回；`platform-win` 增加对 `meeting-copilot-domain` 的直接依赖 | **已登记**：许可证为 MIT/Apache-2.0（与 `deny.toml` 允许清单一致）；UIA/OCR 后续复用同一依赖 |
| X-AUD-10 | 关键词提取新增依赖 | 引入 `jieba-rs` 0.7.4（features: `tfidf`）做中文关键词提取；词典内嵌，exe 体积约 +8 MB | **已登记**：MIT 许可证；替代此前手写分词规则（长句切不出词、百分比被截断） |
| X-AUD-11 | 本地 ASR 运行时依赖 | 运行期依赖 Python 3.13 + `vosk` 0.3（wheel 内含 libvosk）与 `.tmp/asr/` 下的 vosk 中文小模型（约 42 MB）；模型与依赖 **不入库**，Rust 侧以 `CREATE_NO_WINDOW` 启动子进程 | **已登记**：Apache-2.0 许可证；发布版需另行提供模型与 Python 环境（README 已注明） |
| X-AUD-12 | 依赖安全修复 | `cookie` 0.18.2 命中 RUSTSEC 安全公告，升级至 0.18.3（tauri 依赖链，`^0.18` 兼容） | **已修复**：`cargo-deny check advisories` 输出 `advisories ok` |
| X-AUD-13 | 非 Windows 平台占位 | macOS CI 因桌面壳无条件引用 Windows-only `wasapi` 而失败（E0432/E0433） | **已修复**：新增 `wasapi_stub.rs`（同接口、`open` 返回不支持），run 45152256989 两个 darwin 任务 success |
| X-AUD-14 | 本地门禁口径滞后（UI） | `check-ui-integration --rule=settings-ux` 仍匹配 `disabled={!snapshot.canTest/canSave}`，实际页面已改为可计算的 `canTestNow`/`canSaveNow` | **已修复（2026-11-23）**：门禁改用正则，兼容 `snapshot.*` 与可计算变量两种写法 |
| X-AUD-15 | 装配层全局态（B4 source） | **已修复（2026-11-23）**：`AsrPipeline`/`CardStore` 改为 Tauri managed State（`Arc`），移除 `OnceLock`；门禁豁免同步删除 | 已关闭；`b4-source` 对其他禁用面仍生效 |
| X-AUD-16 | 开发期隐私注入入口 | 为关闭 U-05 第 8 项，新增 `app/src-tauri/src/privacy.rs`，读 `PRIVACY_SCREEN_SHARE` / `PRIVACY_EMERGENCY_MASKED`（只认 `"1"`，默认关）。读取点放在 `src/` 根而非 `ipc/`，避免命中装配层"禁环境变量"规则（`check-b4-gates --rule=source` 已复核 rc=0）；注入未改变当时命令面（14，2026-11-25 复核为 26，均与此开关无关） | **已登记**：属开发期验收开关，不投影给前端、不落库；真机结论出来后可移除以消除入口 |

## 5. 工程环境阻塞（非代码问题）

| 项 | 现象 | 原因 | 处理 |
|---|---|---|---|
| GitHub Actions 全红 | **已恢复**：run #56 全部 8 个 job `success` | 此前账户预算设为 $0，Actions 使用被阻止；公开后公共仓库免费额度生效 | 已关闭 |
| 仓库可见性 | **已公开**（2026-11-23） | 用户要求公开 | 已完成；About 描述与 topics 已同步 |
| LICENSE | 已补 `LICENSE-MIT` 与 `LICENSE-APACHE` | README 声称 MIT OR Apache-2.0 | 已完成（GitHub 识别为 Apache-2.0） |
| 本机 sqlcipher 编译 | `perl`/OpenSSL 配置失败（`0xc0000142`） | 本机工具链环境，非代码 | 如需本地验证，改用 Linux/WSL 或安装可用 OpenSSL 构建链 |
| 本机绝对路径 | **已完成历史改写** | 历史提交曾含本机绝对路径 | 已完成：全历史检索旧路径 0 命中；备份分支与 bundle 已删、reflog 已清、gc 已做 |
| 本机 `git push` | **已恢复**（2026-11-23 多次推送成功） | 此前网络/代理对 Git 传输的限制 | 已完成；如再失败按 §8.7 处理，不要反复重试 |
| 本机 MSVC 重编译 | `zstd-sys` 报 `D8050`/`LNK1104` | 系统 `%TEMP%` 与调试记录写入异常 | **已绕过（2026-11-23）**：设 `TEMP/TMP=.tmp\msvc-temp` 与 `CARGO_PROFILE_DEV_DEBUG=0`；重建、测试均通过 |
| 本机 Rust 代理 | `cargo --version` 无输出、bash `rc=127`；加 PATH 后仍输出 `rustfmt 1.11.0`、`cargo metadata` 报 `Unrecognized option: 'no-deps'` | **根因（2026-11-24 21:48 查明）**：`~/.cargo/bin` 下 16 个 exe 全是同一个二进制（SHA256 一致），它依赖 210 MB 的 `rustc_driver-7e26db7803845d2b.dll`（只在工具链 `bin` 下），且实际行为是 rustfmt 而非 rustup 代理。PATH 只有 `~/.cargo/bin` → `0xC0000135`；补上 DLL 也只是跑成 rustfmt | **已修复（2026-11-24 21:48）**：把 `%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin` **前置**到 `HKCU\Environment\Path`，`~/.cargo/bin` 保留在后段（cargo-deny 等第三方工具仍可用）。原值备份于 `.tmp/rust-path-backup-20261124-214821.txt`。验证 `cargo 1.100.0`、`rustc 1.100.0`、`cargo metadata --no-deps` rc=0；21:57 已重建 exe。**残留**：`rustup` 自身仍是坏代理，`rustup update` 等功能需另跑 `rustup-init` 重装 |

## 6. 关闭条件优先级

1. **P0 环境**：**已完成**——仓库公开后 Actions 恢复，run #56 的 8 个 job 全部 `success`；AC-B4-109 远端已取证。
2. **P1 外部条件**：提供端点凭据与真机 → 关闭 U-05 / U-08 的对应子项（步骤见 §8.2、§8.3）。
3. **P2 裁决**：**已完成（2026-11-23）**——Q-01、Q-06 已执行；回收 14.7 GiB；重建后测试 877/0/10、门禁 34/34（步骤见 §8.4）。
4. **P3 规格**：U-07 四项逐项立项（先规格、后实现，步骤见 §8.5）。
5. **P4 治理**：**已完成（2026-11-23）**——Q-07 入库、AUD-17 四道门禁进 CI、X-AUD-15 改 Tauri managed State（步骤见 §8.6）。

## 7. 恢复入口

- 审计与清理：`docs/progress/b4-status.md` §30（§30.1 台账、§30.4 裁决、§30.5 验收）。
- 门禁复核日志：`.tmp/gates/`（本地，未入库）。
- Q-01/Q-06 执行日志与指纹：`.tmp/archive/`（本地，未入库）。
- 未关闭项原文：`docs/progress/b4-status.md` §25.3、§29.5。
- 交接说明：`docs/progress/handoff-2026-11-22-pui.md`。
- 平台层实现：`crates/platform-win/src/subtitle.rs`（UIA 字幕）、`crates/platform-win/src/ocr.rs`（OCR 兜底）；提交 `9c6ca08`（2026-11-24）。
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
# 若 cargo 输出 fsmonitor_ipc__send_query 错误，用环境变量临时关闭（不改 git 配置）
export GIT_CONFIG_COUNT=1; export GIT_CONFIG_KEY_0=core.fsmonitor; export GIT_CONFIG_VALUE_0=false

cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/check-g2-claims.sh
bash scripts/scan-secrets-docs.sh --all-md
bash scripts/check-b4-gates.sh --rule=zero-transcript
bash scripts/check-mac-scope.sh --rule=claims
bash scripts/check-b3-gates.sh --rule=zero-transcript
bash scripts/check-ui-integration.sh
bash scripts/check-layering.sh
bash scripts/test-name-inventory.sh
```

> 并发注意：`platform-win` 的 3 个 `secure_store` 用例并行写同一凭据目标会互踩（`test-key-idempotent` vs `test-key-roundtrip`）。若随机失败，先按串行复核：`cargo test -p meeting-copilot-platform-win -p meeting-copilot-desktop -- --test-threads=1`。这是既存的测试隔离缺陷，不是本轮改动引入。

验收：以 README「当前进度」的最近一次全量基线为准（当前 `1049 passed / 0 failed / 12 ignored`，桌面层 93 例、平台层 30 例）；四条门禁 rc=0。任何一条不通过，先停下修基线，不要继续后面的步骤。

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
export LLM_LIVE_CALL_CAP=24
```

> cap 取值按实际要跑的用例数定，不要沿用固定的 10：`llm_live` 的 9 个用例合计 **14 次**调用（`total_latency_four_tasks` 与 `schema_conformance_rate` 各 4 次，其余各 1 次，`ac25` 为 0），cap=10 会在中途 `budget_exhausted`；`llm_live_baseline` 每任务 30 次 × 4 任务 = **120 次**，cap 需 ≥120（代码内理论上限为 `samples*3*4`，30 样本即 360）。

2. 先做不联网的预检，确认记录目录可写（两个根目录必须事先存在，否则最后会以 `record_dir_missing` 退出 2）：

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

**执行记录（2026-11-24）**

- 结果：`llm_live` **9 passed / 0 failed**（16.93s）；`llm_live_baseline` **1 passed**（190.95s，`SAMPLES=120`、`CALLS=121`）；`leak_hits=0`；脚本退出码 0。逐任务 p50/p95 与子项状态见 `b4-status.md §34`。
- 本机编译前置：`export TEMP="$PWD/.tmp/msvc-temp"; export TMP="$TEMP"; export CARGO_PROFILE_DEV_DEBUG=0`，否则 MSVC 报 `D8050`（见 §8.11-2）；`cargo` 仍需 rustup toolchain 的真实二进制。
- 记录根必须先建：`mkdir -p .tmp/llm-live crates/infrastructure/.tmp/llm-live`，否则脚本在最后一步以 `record_dir_missing` 退出 2（用例其实已经跑完）。
- 凭据不落文件：`scripts/scan-secrets.sh` 会对工作区做 `sk-[A-Za-z0-9]{20,}` 全量匹配，把 Key 写进 `.tmp/*.sh` 之类脚本会让该项门禁失败；只在当前终端 `export`。
- `ac23` 本次 `window_no_429`（未触发限流），不作限流结论；`ac24` 按 100 ms 截止降级、`ac25` 零调用，均符合设计。

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

**第 7 项：卡片渲染（真机，手把手）**

> 2026-11-24 更新：自动出卡已默认关闭，**取证不再需要真实会议**。两条路径任选，路径 A 最快（约 1 分钟）。

1. 重新构建（避免嵌入旧 `dist`）：

```powershell
cd "<仓库根>"
cargo clean -p meeting-copilot-desktop
cargo build -p meeting-copilot-desktop --features llm-https
```

2. 双击 `target\debug\meeting-copilot-desktop.exe`（不要用 agent 启动，agent 进程无法创建 WebView2 窗口）。

3. **路径 A（手动出卡，推荐）**：卡片区下方"会议文本"输入框粘贴
   `我们确认 SLA 99.9%，灰度范围限定华东两站点，回滚预案下周三前给。`
   → 点"生成卡片"。预期：卡片列表出现若干张，每张有标题与结论摘要。
4. **路径 B（转写 + 选中查义）**：点"开始实时转写"，让系统播一段中文语音（或放一段会议录音）；转写区逐句出现文字 → 鼠标选中一个词 → "查意思" → 解释面板出结果 → "生成卡片"。预期：卡片标题就是选中的词。
5. 逐项看并记录：
   - 卡片正文可见；点击可展开详情；不抢焦点；
   - 卡片区标题右侧显示"共 N 张"，点"清空"后列表变空且计数归零；
   - 禁用卡（`clickState=disabled`）不可点，并显示原因文案（"引用已过期/授权已撤回/当前卡片不可点击/紧急遮蔽中/投屏保护中/引用未经核实"）。
6. 记录：日期、路径（A/B）、卡片标题、每项通过/失败、截图路径（截图只留本机，参照本节第 4 条）。

判定表（填完即可在 `b4-status.md` 追加结论）：

| 检查点 | 预期 | 实测 | 结论 |
|---|---|---|---|
| 卡片正文与结论 | 可见、非空 | 2026-11-25：7 张卡正文、类型、引用与置信度角标齐全（`u05-A-cards.png`） | 通过 |
| 点击展开详情 | 展开、不抢焦点 | 2026-11-25：内联展开 `引用一致性：passed`、`local-1`，无模态、不抢输入焦点（`u05-C-detail-visible.png`） | 通过 |
| 数量与清空 | "共 N 张"，清空后为空 | 2026-11-25：清空后列表为空、显示"暂无卡片"、按钮灰化（`u05-D-cleared.png`） | 通过 |
| 禁用卡不可点 | 点击无响应 + 原因文案 | 真机不可达：桌面单会话 `session_matches` 恒 true（`main.rs:55`、`card_store.rs:341` 同一 `gate.session_id`），造不出 StaleReference；遮蔽/投屏 `body_visible=false` 时 `KeywordCard.tsx:46` 只渲染角标、不渲染按钮与 `card-reason`。以 `resolve_click_state` 六因优先级单测 + 渲染分支判定 | 通过（代码判定） |

**第 8 项：投屏/紧急遮蔽（2026-11-25 真机结论已补，3/3 通过）**

- 代码事实：`crates/application/src/gate/prefilter.rs:76` 在 `screen_share` 且策略 `screen_share_protection_default` 为真时按 G1 `ScreenShareProtection` 拒绝；`crates/application/src/gate/keyword.rs:233` 为 `body_visible = !env.screen_share && !env.emergency_masked`，同文件 `:186` 对紧急遮蔽给出 `ClickState::Disabled(EmergencyMasked)`；`app/src/cards/KeywordCard.tsx:33` 在 `bodyVisible=false` 时只渲染"有 N 条"角标。
- 注入入口（2026-11-24 就绪）：`app/src-tauri/src/privacy.rs` 读两个开发期开关，**只认严格的 `"1"`**，默认关闭；不落库、不进日志、不投影给前端；注入开关本身不新增 IPC 命令（当前命令面 19 个）。
- 两种状态的实际表现**并不相同**（由单测固定，见下）：
  - `PRIVACY_SCREEN_SHARE=1` → G1 直接拒绝，本地入口**不出卡**（比"隐藏正文"更严格）；
  - `PRIVACY_EMERGENCY_MASKED=1` → 照常出卡，但 `click_state=Disabled(EmergencyMasked)`、正文隐藏。
- 真机操作（PowerShell，两选一）。**先设环境变量再启动已构建的 exe**，不要用 `cargo run`（会重编且可能不带 `llm-https`）：

```powershell
cd "<仓库根>"
$env:PRIVACY_SCREEN_SHARE = "1"                    # 投屏态
# 或 $env:PRIVACY_EMERGENCY_MASKED = "1"          # 紧急遮蔽态
& ".\target\debug\meeting-copilot-desktop.exe"
```

  用第 7 项的**路径 A**触发一次出卡（粘贴文本 → 生成卡片），然后：

  - 投屏态：确认**没有新卡片**；手动粘贴路径提示「已生成 0 张卡片。」（`App.tsx:505`），走"选中文字→查意思"路径时才是"这条内容未通过卡片生成条件。"（`App.tsx:321`）——两条路径分别成立，不要混用；
  - 紧急态：确认**有卡片但正文隐藏**（只留"有 N 条"角标），点卡片无响应。

判定表：

| 状态 | 启动变量 | 预期 | 实测 | 结论 |
|---|---|---|---|---|
| 投屏 | `PRIVACY_SCREEN_SHARE=1` | 0 张新卡 + 拒绝提示 | 2026-11-25：粘贴同一句并点"生成卡片"，卡片区仍"暂无卡片"，提示「已生成 0 张卡片。」；右上角"按需生成，不随转写自动出卡"、"清空"灰化（`u05-EVIDENCE-screen-deny.png`） | 通过 |
| 紧急遮蔽 | `PRIVACY_EMERGENCY_MASKED=1` | 有卡、正文隐藏、角标 N 条 | 2026-11-25：7 张卡全部只剩「有 1 条」角标，卡片区 `dark_text=0`、`blue_badge=259`，无正文无按钮（`u05-EVIDENCE-emergency-20261125.png`） | 通过 |
| 紧急遮蔽 | 同上 | 点击无响应、无详情面板 | 同上：`body_visible=false` 走 `KeywordCard.tsx:46` 分支只渲染角标，无"查看详情"可点元素、无详情面板 | 通过 |

- 两个变量都是**进程启动时读取**，改完必须重启应用；取证完关掉终端即失效，不写配置、不落库。

> 取证记录（2026-11-24 19:55）：`verify-privacy.ps1 -Mode emergency`（跑了两次）与 `-Mode screen` 均输出 `[verify] launched from <仓库根>`，说明注入变量与启动都成立；但两张截图里应用窗口只露出标题栏，卡片区不在画面内，**无法判定遮蔽表现，第 8 项仍 open**。
>
> 补齐只需两张图，每张约 30 秒：
>
> 1. 紧急遮蔽：`$env:PRIVACY_EMERGENCY_MASKED="1"` 启动 exe → 粘贴同一句 → 点"生成卡片" → 7 张卡全部只剩「有 1 条」角标、无正文无按钮、无详情面板 → **通过**（2026-11-25）。
> 2. 投屏：`$env:PRIVACY_SCREEN_SHARE="1"` 启动 exe → 同样粘贴并点"生成卡片" → 卡片区仍"暂无卡片"、提示「已生成 0 张卡片。」→ **通过**（2026-11-25）。
>
> 截图存本机（`.tmp/u05-EVIDENCE-*.png`），不入库；两格已于 2026-11-25 补齐，见 `b4-status.md §53`。

- 已有覆盖：契约层 `cargo test -p meeting-copilot-application --test gate_keyword`（断言 `!admission.body_visible`）；桌面层 `cargo test -p meeting-copilot-desktop --bins ipc::card_store`（`screen_share_blocks_card_creation_at_g1`、`emergency_mask_keeps_card_but_disables_click`）。
- 关闭口径（2026-11-25 已满足）：第 8 项三格均有真机截图与结论，U-05 据此关闭（见 `b4-status.md §53`）。两态实现不同——投屏在 G1 直接不出卡，紧急遮蔽出卡但隐藏正文——不能互相替代。

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

**Q-01 `target/`（已执行 2026-11-23）**

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

验收：测试回到 `877 passed / 0 failed / 10 ignored`，四条门禁仍 rc=0。回收约 **14.7 GiB**（2026-11-23 实测）。

**执行结果（2026-11-23）**：`cargo clean` 删除 37,872 文件 / 14.7 GiB；`cargo build --workspace` 通过；`cargo test --workspace` 为 **877/0/10**；门禁 **34/34**；重建后 `target/` 约 2.0 GB。指纹见 `.tmp/archive/{rustc.txt,cargo.txt,Cargo.lock.sha256}`。本机重编译需设 `TEMP/TMP=.tmp\msvc-temp` 与 `CARGO_PROFILE_DEV_DEBUG=0`，否则 MSVC 报 `D8050`/`LNK1104`。

**Q-06 外层 `.codeartsdoer/temp/accept/`（已执行 2026-11-23）**

1. 只读确认无交付文件：

```powershell
Get-ChildItem ..\.codeartsdoer\temp\accept -Recurse -ErrorAction SilentlyContinue
```

2. 确认后只删该目录；**不要**删除外层 `.codeartsdoer` 的其他内容：

```powershell
Remove-Item ..\.codeartsdoer\temp\accept -Recurse -Force
```

**执行结果（2026-11-23）**：`temp/accept` 25 个文件（均为 `.original.*` 备份/日志）已删除；外层空 `.screenshots/` 已删除；未触碰 `.codeartsdoer` 其他内容。

3. 每完成一项，在本表 §2 对应行标记"已执行 + 日期"，并在 §30.3 追加一行台账。

### 8.5 立项 U-07 四项规格（先规格、后实现）

> 状态（2026-11-24）：四项规格已评审通过并完成实现（提交 `c96f381`）；本节保留为流程模板。实现裁决见 `design.md` 2.10.6，任务勾选见 `tasks.md` 附录 G；G-43-D 五类异常与 G-52-D 运行时摘要扫描均已关闭，U-07-43 真实样本未达标前保持 open。其后的检索源接入 U-09 已按 SPEC 第 8 章、DESIGN 2.11、TASKS 附录 H 实现 A~F 并推送 `f3d66a4`，H-34-G 待真实端点验收。

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

**执行结果（2026-11-24）**：四项增量规格已写入 `docs/sdd/spec.md` 第 7 章、`docs/sdd/design.md` 2.10、`docs/sdd/tasks.md` 附录 G；四项实现已提交 `c96f381`。U-07-43 的真实检索源已由 U-09 承接：A~F 已实现并推送 `f3d66a4`，待 H-34-G 真实端点验收后启动 200 条采样；U-07-52 的归档保留期按方案 B（5 年）确认，个人自用简化处理，已于 2026-11-24 关闭。

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

**执行结果（2026-11-23）**

- Q-07：`.codeartsdoer` 43 文件 / 4.9 MB；`scan-secrets` rc=0；根 `.gitignore` 改为仅忽略 `temp/`、`.codebase/`，嵌套 `.codeartsdoer/.gitignore` 同步收窄；`mcp_settings.json` 为空配置、`rule/metadata.properties` 无敏感项。
- AUD-17：采用方案 A；`compliance` job 增加四步并把超时从 15 分钟提到 20 分钟；本地四条门禁均 rc=0。
- X-AUD-15：`AsrPipeline` 与 `CardStore` 改为 Tauri managed State（`Arc`），`main.rs` 统一 `manage`；`check-b4-gates --rule=source` 删除两文件 `OnceLock` 豁免后仍 rc=0；桌面端 36/36 通过。

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
### 8.8 验证 UIA / OCR 字幕通道（2026-11-24 新增）

**默认行为（推荐）**：只跑开源 ASR，OCR 不参与。

```bash
cargo run -p meeting-copilot-desktop
```

点"开始"即可；字幕通道未监听窗口时为 `off`。

**开启 OCR 实验性兜底**（仅当 UIA 读不到文本时才截图识别）：

PowerShell：

```powershell
$env:OCR_FALLBACK = "1"
cargo run -p meeting-copilot-desktop
```

Git Bash：

```bash
export OCR_FALLBACK=1
cargo run -p meeting-copilot-desktop
```

**前提与限制**：

- 会议窗口必须在**前台且未被遮挡**；`ocr.rs` 会检查 `GetForegroundWindow`，不满足直接跳过（避免读其它窗口内容）。
- 只截客户区**底部 40%**（`bottom_band`）；按钮/菜单文字可能混入，真实会议需按字幕实际位置校准 ROI。
- 只处理内存位图，不落盘原图；识别文本经 `compact_cjk` 合并汉字间空格。

**观察点**：

1. 前端"字幕文本通道"状态：`off`（未启动/未找到窗口）、`degraded`（窗口有但无文本，健康原因会带已遍历元素数）、`ready`（读到文本）。
2. 命令行确认会议窗口是否存在：

```powershell
Get-Process WeMeetApp -ErrorAction SilentlyContinue | Select-Object Id,MainWindowHandle,MainWindowTitle
```

`MainWindowHandle=0` 表示最小化到托盘，需先还原窗口再测。

**评级口径**：

- 只有"真实会议进行中 + 会议窗口前台 + 读到字幕文本并出卡"才算通过；
- 只识别到客户端首页按钮文字**不算**通过（只能证明截图与 OCR 链路可用）；
- 未通过前，平台层 UIA/OCR 行保持 `open`。

### 8.9 跑 `#[ignore]`（默认特性 12 个，逐类，手把手）

> 2026-11-25 盘点：默认特性 12 个 = `llm_live.rs` 9 + `search_live.rs` 2 + `wasapi.rs` 1；`llm_live_baseline.rs` 的 1 个受 `#![cfg(feature = "llm-https")]` 门控，开特性后共 13 个。**D 类（本机音频）已跑通**，A/B/C 三类仍待真实端点与凭据。

**A. LLM 真实采样**（`llm_live.rs` / `llm_live_baseline.rs`）

变量模板见 §8.2；只在当前终端设置。

```bash
export LLM_LIVE=1
export LLM_BASE_URL="<含已确认路径段的端点地址>"
export LLM_API_KEY="<your-key>"
export LLM_MODEL="<your-model>"
export LLM_MODEL_CONFIRMED="<与上一行完全相同>"
export LLM_ALLOWLIST="<your-host>"
export LLM_LIVE_CALL_CAP=360   # llm_live 需 14 次；llm_live_baseline 每任务 30×4=120 次

cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --ignored --test-threads=1
cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live_baseline -- --ignored --test-threads=1
```

> 实测（2026-11-24）：`llm_live` 9 用例共 14 次调用、`llm_live_baseline` 120 次；cap 小于实际调用数会在中途 `budget_exhausted` 并让 `run.json` 标 `budget_hit=true`。记录根须先 `mkdir -p .tmp/llm-live crates/infrastructure/.tmp/llm-live`。

**B. 检索源冒烟**（`search_live.rs::smoke_three_rounds`，H-34-G）

```bash
# 三个 SEARCH_* 路径必须是绝对路径：cargo test 进程 CWD 是 crate 根目录
export SEARCH_PROVIDERS_FILE="<providers.json 绝对路径>"
export SEARCH_QUERIES_FILE="<查询文件绝对路径，至少 3 条>"
export <authRef>="<Bearer 凭据>"
export SEARCH_REPORT_PATH="<报告绝对路径>"   # 可选，只含计数与时延

# 必须带 --features llm-https：缺省特性下没有真实 TLS 传输，检索全部 Unreachable
cargo test -p meeting-copilot-infrastructure --features llm-https \
  --test search_live smoke_three_rounds -- --ignored --test-threads=1 --nocapture
```

把 `<authRef>` 替换成 `providers.json` 里登记的真实变量名；不要把凭据写进仓库或提交信息。
推荐直接使用 `bash scripts/run-u07-43-samples.sh --smoke-only` 完成上述编排（含路径与特性修正）。

**C. 200 条真实样本**（`search_live.rs::collect_200_real_samples`，U-07-43）

三条接入路径都有落地脚本，填完端点与凭据后直接跑（配置自动写入 `.tmp/u07-43/config.env`，主脚本自动加载）：

```bash
# A. 自建 SearXNG
bash scripts/u07-43/setup.sh --mode a --endpoint "https://<SearXNG 公网域名>/search"

# B. 博查/Tavily Key：部署转发 Worker（自动注入 secret 并写好 config.env）
UPSTREAM_KEY='<检索服务 Key>' bash scripts/u07-43/deploy-worker.sh --provider bocha

# C. 自定义 providers.json（模板 scripts/u07-43/providers.example.json）
bash scripts/u07-43/setup.sh --mode c --providers <绝对路径> --token '<凭据>'

# 三条路径最后都跑这一条
bash scripts/run-u07-43-samples.sh
```

- B：`deploy-worker.sh` 用 `wrangler.toml` 部署 `search-proxy.worker.js`，`SHARED_TOKEN` 缺省时随机生成；首次需 `wrangler login`，`--dry-run` 只做本机检查。Worker 的 `/health` 可自检 secret 是否注入。
- C：识别到 `SEARCH_PROVIDERS_FILE` 指向已存在的文件即进入自定义模式，**不重建、不覆盖**；`preflight.py` 校验字段、枚举、`id`/`priority` 唯一性、公网 HTTPS 主机与每个 `authRef` 的环境变量。多个 `authRef` 用 `--env NAME=VALUE` 逐个给值。
- 不写配置文件也可手动 `export SEARCH_ENDPOINT` / `SEARCH_AUTH_REF` / `<凭据>` / `SEARCH_PROVIDERS_FILE`，或给主脚本 `--config FILE` / `--no-config`。

主脚本默认生成 400 条互不重复查询（`U07_QUERY_COUNT` 可覆盖；单独调用 `gen-queries.py` 时默认 240 条）、先跑 3 次冒烟、再跑 200 条，最后用
`scripts/u07-43/verdict.py` 判定；`--check-only` 只预检。等价的裸命令（务必带特性与绝对路径）：

```bash
export SEARCH_QUERIES_FILE="<至少 200 条纯文本绝对路径>"
cargo test -p meeting-copilot-infrastructure --features llm-https \
  --test search_live collect_200_real_samples -- --ignored --test-threads=1 --nocapture
```

判定：真实成功样本 `≥ 200`（`search_live.rs` 结尾断言与 `verdict.py` 的 `real < target` 同口径；多源配置下一次 `run` 可能写入多条，写死 `=200` 会误判失败）；缓存命中、失败、超时都不计。报告只含 `real_samples`、`p50_ms`、`p95_ms`、`p99_ms`、`timeouts`、`failures`、`cache_hits`、`status` 等计数与时延，不含查询原文。

**D. 本机音频冒烟**（`wasapi::tests::loopback_opens_and_captures_when_audio_is_active`）

```bash
cargo test -p meeting-copilot-platform-win --lib wasapi -- --ignored --nocapture
```

前置：默认输出设备可用且有音频播放。

> 实测（2026-11-25）：**1 passed / 0 failed，rc=0**。播放 30 秒 440 Hz 双声道正弦波（`.tmp/wasapi_smoke_tone.wav`，本机临时产物）后运行，0.05s 返回——低于 200×10ms 的空转上限，说明首个 `next_chunk()` 即取得真实环回包，`sample_rate>0`、`sequence>0` 断言通过，不是无音频空转退出。该用例保持 `#[ignore]` 不解锁（CI 无音频设备），需要时按本节显式重跑。

**收尾**：每类跑完，在 §1 对应行记录"日期 + 通过/失败 + 退出码"，不写端点、Key、模型名。

### 8.10 新增检索源 5 项核对

1. HTTPS 绝对端点，不含账号密码。
2. 显示名 ≤50 字。
3. `authRef`：环境变量名；值只在终端设置。
4. 限流 `perMinute` / `perDay`（缺省 10 / 1000）。
5. 200 条查询纯文本文件路径（每行一条）。

核对后先做敏感扫描，再按 §8.9-B 跑 3 次冒烟；冒烟未通不要直接跑 200 条。

```bash
bash scripts/scan-secrets-docs.sh --all-md
```

### 8.11 本机常见坑（照抄即可）

1. `cargo` 报 `rustc_driver-*.dll` 缺失，或 `cargo --version` 输出 `rustfmt 1.11.0` → 把 rustup toolchain 的 `bin` **前置**（必须排在 `~/.cargo/bin` **之前**；2026-11-24 已写入用户级 PATH 持久修复，见 §5）：

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$HOME/.cargo/bin:$PATH"
```

2. MSVC `D8050` / `LNK1104` → 设 `TEMP/TMP` 与 `CARGO_PROFILE_DEV_DEBUG=0`（见 §8.1）。
3. `fsmonitor_ipc__send_query` → 用 `GIT_CONFIG_*` 环境变量临时关闭（见 §8.1），不要改 git 配置。
4. `secure_store` 用例偶发互踩 → 加 `--test-threads=1`。
5. 前端改动后启动仍是旧界面 → `cargo clean -p meeting-copilot-desktop` 再 `cargo build`。
6. agent 进程无法创建 WebView2 → 真机必须双击 `target\debug\meeting-copilot-desktop.exe`。
7. `search_live` 报「providers.json 没有可用检索源」→ 多半是 `SEARCH_PROVIDERS_FILE` 用了相对路径：`cargo test` 进程 CWD 是 `crates/infrastructure/`，`ProfileStore::load()` 按「文件不存在」返回空列表，不会报错。用绝对路径（`run-u07-43-samples.sh` 已自动转换）。
8. `search_live` 三次检索全部 `Unreachable` → 忘了 `--features llm-https`；缺省特性下 `HttpJsonProvider::with_default_transport` 不装 TLS 传输。
9. `run-u07-43-samples.sh` 复用上一次的占位 `providers.json` → 换端点时脚本会检测不一致并重建；也可删掉 `.tmp/u07-43/providers.json` 后重跑。
10. `infrastructure --lib` 里 `audit/retention`、`profile_store`、`store/repo` 的临时文件用例报「拒绝访问 / CannotOpen」→ `TEMP` 用了 MSYS 风格 `/c/...`。改成 Windows 风格绝对路径（如 `C:\...\TSC\.tmp\msvc-temp`）后 149 passed / 0 failed；`run-u07-43-samples.sh` 已按此处理。
11. B 路径的 Worker 首次部署顺序是**先 `wrangler deploy` 再 `wrangler secret put`**：Worker 不存在时 secret 写入会失败。`deploy-worker.sh` 已按此顺序；secret 是绑定，注入后立即生效，无需重新 deploy。
12. 自定义 `providers.json` 里 `health` 写成 `degraded` 仍算可用源（`ProviderProfile::is_usable` 只排除 `unavailable`），`preflight.py` 与 Rust 侧口径一致；写成 `unavailable` 会被预检拒绝。
13. 配置写进 `.tmp/u07-43/config.env` 后主脚本会**默认自动加载**；换路径跑时留意残留的旧配置（可用 `--no-config` 忽略或 `--config` 指定）。换端点时脚本检测到与现有 `providers.json` 不一致会自动重建；C 路径下则完全不碰用户文件。
