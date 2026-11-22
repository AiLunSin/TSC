# 交接说明：P-UI 落地与收口（2026-11-22）

## 1. 当前状态

- 仓库根：`TSC/TSC`（会话工作区 `TSC` 仅为其父目录）
- 分支：`main`；本地相对 `origin/main` 领先多个提交，**未 push**
- 最终 HEAD：以 `git log -1` 为准（C7 文档提交）
- 工作区：C7 提交后干净

## 2. 提交清单（C1~C7）

| 批次 | 提交 | 内容 |
|---|---|---|
| C1 | `ed0a644` | application 3 端口 + 3 事实源；Windows 凭据存储（`windows-sys 0.45.0`） |
| C2 | `411337d` | 6 组只读 DTO、`AppState`、目录清单 |
| C3a | `0d1e09f` | 4 键配置存储、设置快照/保存/清除 |
| C3b | `9c4a87e` | 探针单飞、8 类映射、冷却相对值、`llm-https` 适配 |
| C3c | `f4b0085` | 卡片快照/详情、`ClickState` 手工拆载 |
| C4 | `4fd8b48` | 6 命令注册、`AppFactsSource`、`withGlobalTauri` |
| C5a | `3e2e6fc` | 7 组前端 DTO、`tauri.ts` 手写全局调用 |
| C5b | `3eeeaa2` | 设置页、卡片/详情组件、快照驱动 `App` |
| C6a | `f6b7ba3` | 六规则静态门禁 `check-ui-integration.sh` |
| C6b | `c3d99b4` | `--rule=source` 扩面（ipc 受限 + 两个严格面） |

## 3. 已完成与取证

- 桌面层 **24 passed / 0 failed**；application settings **6**；platform-win 凭据 **4**。
- 六规则 `check-ui-integration.sh` **PASS / 0 命中**。
- `check-b4-gates.sh --rule=source` **0**；`budget/record/outbound/zero-transcript/targets/doc-plan` 全 **0**。
- 前端 `typecheck` / `build`（34 modules）通过；`check:bindings` 为 **11 个 DTO** 一致。
- AC 汇总 **130**；`NEW_TARGETS` **25**。

## 4. 未关闭项（必须原样保留）

1. **U-05 整体 `open`**：未做真实联网 E2E、未做真机 WebView2 验证。
2. **C-UI-02**（macOS CI 编译观察项存废）未裁决；`ci.yml` / `deny.toml` / `platform-mac` 零 diff。
3. **C-UI-03**（生产探针）默认关闭，AC 仅用替身取证。
4. 9 个 `#[ignore]` 与 `tests/replay/` **未解锁**。
5. 未执行 `git push`。

## 5. 新会话开场（可直接粘贴）

继续 P-UI 收尾；先读 `.codeartsdoer/specs/ui_integration/{spec.md,design.md,tasks.md}` 与 `docs/progress/b4-status.md §28~§29`；
从 C7 收口继续：核对 AC=130、`NEW_TARGETS`=25、六规则 PASS 与 `--rule=source` 退 0；
注意实况：`windows-sys 0.45.0`、`withGlobalTauri`、`ClickState` 拆为 `clickState`+`disabledReason`、C6 脚本必须晚于 C1~C5；
不要解锁 `#[ignore]`/`tests/replay/`，不要把 U-05 写成 `closed`；提交只 commit、不 push，推送需用户明确说"上传/推送"。