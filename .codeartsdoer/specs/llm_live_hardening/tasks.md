# 编码任务清单 · llm_live_hardening

> **唯一基线**：`.codeartsdoer/specs/llm_live_hardening/spec.md`（32 条 `AC-LV-01~AC-LV-32`）与 `design.md`（2.2 接口清单、2.4 八项裁决、2.5 用例清单、2.6 脚本伪代码、2.7 AC 映射、2.8 阶段、2.9 回滚）。
> **冲突优先级**：spec 的 `AC-LV-*` 判定 > design 的落地方式。二者不一致处见第 1 节，逐条标注处置，禁止静默放宽。
> **编号约定**：阶段 `P0~P3`；主任务 `W-LV-<n>`；子任务 `W-LV-<n>.<m>`；不复用 `AC-LV-*` / `AC-CI-*` / `AC-LLM-*` / `D-L31-1` / `X-LV-*`。
> **本文件边界**：只描述"改哪里 → 跑什么 → 期望什么"，不含代码实现；`docs/sdd/**` 只读；本阶段不修改任何代码、脚本、`ci.yml`。

## 0. 判定环境（本地命令可直接执行）

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
export CARGO="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"
```

固定判据：同一命令连续 3 次结论必须一致（`AC-LV-23` 判定环境约束 3）；任何"未执行/超时/扫描面缺失/`not_measured`"不得记为通过（spec 8.7-4）。

## 1. 冲突与偏差处置表（spec 优先）

| 编号 | 冲突事实 | 处置（写入验收记录） | 关联任务 |
|---|---|---|---|
| `C-LV-01` | `AC-LV-01` 书面区间 `800~960 ms`，实现 `jittered_backoff` 为 `800 ±20%` → `640~960 ms`（`degrade.rs`） | **不改生产退避算法**（是既有 `AC-LLM-23` 离线结论载体）；断言取"`>0` 且 ∈ `[640,960]`"，并登记偏差 `D-LV-01`："`AC-LV-01` 下界 `800` 未达成，实测合法域下界 `640`"。不得把 `[640,800)` 的实测值写成"达标" | `W-LV-11.1`、`W-LV-13.1`、`W-LV-22.1` |
| `C-LV-02` | `AC-LV-25` 上限 `≤2` 次；design 核算实际增量 `0` | 取 spec 上限 `≤2` 为门禁，实测记录 `增量=0（优于上限 2）` | `W-LV-20.1` |
| `C-LV-03` | `AC-LV-03` 判定 `status ∈ {measured, not_triggered}`；`admit` 为 Skip 时无真实采样 | 主判定按 spec；Skip 场景按 design 2.1.3-b 写 `not_measured` + `gate` 原因码，并在记录中标注"未执行"，不判通过 | `W-LV-16.2`、`W-LV-20.3` |
| `C-LV-04` | `scan-secrets-docs.sh --artifacts` 无参时目录缺失 `return 0`；`AC-LV-17` 要求缺失即 `2` | 仅**显式传目录**时缺失 → `2`；无参默认路径保持旧行为（CI compliance 只用 `--all-md`，不受影响）；`run-llm-live.sh` 一律显式传目录 | `W-LV-03.3` |
| `C-LV-05` | `AC-LV-30` 要求同一次运行三项三态共存，`record.rs:76-82` 为"任一 `not_measured` ⇒ 零调用" | 以 `AC-LV-30` 为准：规则收窄为"全部子项 `not_measured`（含空集合）⇒ 零调用"；`calls_used` 语义仍为"整次运行实际请求数"。**必须在 P3 前完成**（`X-LV-01`） | `W-LV-09`、`W-LV-15` |
| `C-LV-06` | P3 新增 1 个 `#[ignore]` → `ignored` 由 8 变为 9 | "既有门禁不回退"判据为"既有 399 条 passed 不减少、`ignored` 增量等于新增 ignore 用例数（+1）"，不得把 `ignored=9` 记为回退 | `W-LV-21.1` |

## 2. 只能由真实端点判定的项（L3 清单）

> 以下项**禁止**用离线注入（`FakeTransport`/`Step`/`FakeClock`）的结论替代；离线只证明"机制面"。

| AC | 只能真实判定的事实 | 离线不可替代的原因 |
|---|---|---|
| `AC-LV-03` | 采样窗口内是否实际出现 429；`ac23` 的真实取值（`measured` / `not_triggered`） | 端点行为不可构造，spec 5.1 明确"只被动观测" |
| `AC-LV-07` | 真实单任务单次：`calls_used` 增量 == 1、`samples.jsonl` 新增 == 1 行 | `U3` 的 3×`Step::Delay` 只验截止优先，不构成真实单次（design 2.5-U3 注 3） |
| `AC-LV-08` | 实测返回与 100 ms 截止的偏差（`total_ms ≤ 300`） | 偏差由真实 TLS+生成耗时决定 |
| `AC-LV-09` | 100 ms 内是否返回 → 是否落 `below_floor_100ms` | 取决于真实端点速度 |
| `AC-LV-13` | 真实 `run.json` 中 `ac25` 落盘值为 `not_measured` + `construction_conflict` | 静态裁决为 L1，落盘观测为 L3 |
| `AC-LV-24` | `calls_used` 与真实 HTTP 请求数（含重试）一致 | 离线台账只验计数点逻辑 |
| `AC-LV-25` | 会话总增量（既有基数 + 新增）| 需真实运行目录核对 |
| `AC-LV-26` | 首个失败后样本序列无后续请求；触顶 `budget_hit` | 需真实错误序列 |
| `AC-LV-30` | 真实 `run.json` 三项三态同时存在 | 单测只验结构 |
| `AC-LV-31` | 真实目录 `seq` 唯一单调 | 需真实多次追加 |
| `AC-LV-27` | 运行期四泄漏面（日志/审计/错误/命令行） | 归属 `AC-26`，本特性只修扫描脚本 |
| 对照请求的端点实际响应 | **本特性裁决不做**（`X-裁决-3`），永久 `not_measured` | 构造隔离与 `AC-43`/`AC-46` 冲突 |

**反向红线**：`AC-LV-01/02/11/12` 的离线结论只证明"治理机制/解码机制"，验收记录禁止写成"真实端点在限流/结构违规下的表现"。

## 3. 任务总览与关键路径

| 阶段 | 主任务 | 子任务数 | 出口判据 |
|---|---|---|---|
| `P0` 记录根与扫描脚本 | `W-LV-01`~`W-LV-06` | 21 | 四态退出码 `0/1/2/2` 精确；`TMPDIR` 与 `-R` 静态 0 命中；`--scan-only` ≤10 s |
| `P1` 泄漏面验收 / 关闭 `D-L31-1` | `W-LV-07`~`W-LV-08` | 6 | 四态 + 7 组优先级矩阵全绿；`D-L31-1` 标记关闭；基线 365 行哈希不变 |
| `P2` 离线增强与装配扩展 | `W-LV-09`~`W-LV-14` | 22 | 新增 4 个离线用例退出 0；12 条既有用例增强落点闭合；`validate` 收窄单测通过 |
| `P3` 真实观测与 CI | `W-LV-15`~`W-LV-23` | 35 | 9 条 ignore；三态/预算/`seq`/零转录全绿；既有门禁不回退 |
| 合计 | 23 个主任务 | **84 个子任务** | — |

**关键路径（串行不可跳）**：

```
W-LV-01 → W-LV-02 → W-LV-03 → W-LV-04 → W-LV-05 → W-LV-06
   → W-LV-07 → W-LV-08            (P1 依赖 P0：否则验收落在错误扫描面)
   → W-LV-09 → W-LV-10            (X-LV-01 收窄必须先于 P3 三态写盘)
   → W-LV-11 → W-LV-12 → W-LV-13 → W-LV-14
   → W-LV-15 → W-LV-16 → W-LV-17 → W-LV-18 → W-LV-19
   → W-LV-20 → W-LV-21 → W-LV-22 → W-LV-23
```

并行许可：`P0`（脚本）与 `P2`（离线/装配）无代码耦合，可由不同人力并行；`P3` 必须在 `P0`+`P1`+`P2` 之后。

---

## P0 记录根修复与扫描脚本改造

### W-LV-01 记录根唯一来源与入口解析

**W-LV-01.1 `record_roots()` 单一来源函数** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-16` · design 2.4.2 / 2.6.1
  - DoD：脚本内记录根路径字面量只出现在 `record_roots()` 一处；函数输出两行（`crates/infrastructure/.tmp/llm-live` 在前、`.tmp/llm-live` 在后），仓库相对、正斜杠。
  - 证据：`grep -c 'crates/infrastructure/.tmp/llm-live' scripts/run-llm-live.sh` 为 1；`bash -c 'source scripts/run-llm-live.sh --print-record-roots'`（或等价 `--scan-only` 日志）输出恰 2 行且 `scan_face=crate` 在首行。

**W-LV-01.2 `--scan-only` / `--record-root` / `--selftest-sample-rc` 入口** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-16` · `AC-LV-23` · design 2.6.2 / 2.2.2-E
  - DoD：`--scan-only` 跳过全部 `LLM_LIVE`/`LLM_BASE_URL`/`LLM_MODEL`/`LLM_LIVE_CALL_CAP` 检查与 `cargo` 调用；未知参数 → `2`。
  - 证据：`bash scripts/run-llm-live.sh --bogus; echo $?` 为 `2`；`PATH=/usr/bin:/bin bash scripts/run-llm-live.sh --scan-only --record-root scripts/fixtures/scan/clean; echo $?` 为 `0`（PATH 遮蔽 `cargo` 仍成功 = 未调用 `cargo`）。

**W-LV-01.3 `--record-root` 越界拒绝** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-16` · design 2.4.2-(d) / 2.2.2-E
  - DoD：解析后的绝对路径不以 `ROOT/` 开头 → `2` + `record_root_outside_repo`，且不执行任何 `grep`。
  - 证据：`bash scripts/run-llm-live.sh --scan-only --record-root ../outside; echo $?` 为 `2`；同命令 stderr 含 `record_root_outside_repo` 且不含 `leak_hits=`。

**W-LV-01.4 空/纯空白凭据拦截（`X-LV-03`）** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-17` · design 2.6.3-(1) / `X-LV-03`
  - DoD：`LLM_API_KEY` 为空或仅空白 → `2` + `credential_missing`；不得执行空 pattern 扫描。
  - 证据：`LLM_API_KEY='   ' bash scripts/run-llm-live.sh --scan-only --record-root scripts/fixtures/scan/clean; echo $?` 为 `2`，输出含 `credential_missing`，不含 `leak_hits=0`。

### W-LV-02 逐目录扫描与三态严格区分

**W-LV-02.1 `timeout` 包裹与总预算（`X-LV-06`）** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-18` · `AC-LV-23` · design 2.6.1 / 2.6.3-(4)
  - DoD：默认总预算 `10` s（`LLM_LIVE_SCAN_TIMEOUT_S` 可覆盖）；每次扫描使用剩余预算，剩余 ≤0 直接判 `scan_timeout`；超时唯一来源为 `timeout` 的 `rc=124`。
  - 证据：`grep -nE 'LLM_LIVE_SCAN_TIMEOUT_S|SECONDS|124' scripts/run-llm-live.sh` 三者各 ≥1；`LLM_LIVE_SCAN_TIMEOUT_S=10` 为默认值。

**W-LV-02.2 命中只计数、不回显** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-19` · design 2.6.3-(0/1 分支) / 6.3-5
  - DoD：命中 → 输出 `leak_hits=<N>`（`N≥1`），不输出文件内容、不输出凭据任意连续 8 字符。
  - 证据：`bash scripts/run-llm-live.sh --scan-only --record-root scripts/fixtures/scan/leak; echo $?` 为 `1`；输出匹配 `leak_hits=[1-9]`；同一输出对夹具假值做"连续 8 字符"grep 为 0 命中。

**W-LV-02.3 缺失 / 不可读 / grep 错误 → 不可判定** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-17` · `AC-LV-18` · design 2.6.3-(2/3/`case`)
  - DoD：两处记录根皆缺失 → `2` + `record_dir_missing`；`rc=124` → `2` + `scan_timeout`；`grep` 自身 `rc=2` → `2`；三者均不得输出 `leak_hits=0`。
  - 证据：`bash scripts/run-llm-live.sh --scan-only --record-root scripts/fixtures/scan/absent; echo $?` 为 `2` 且输出含 `record_dir_missing`；`LLM_LIVE_TIMEOUT_BIN=scripts/fixtures/timeout-124.sh bash scripts/run-llm-live.sh --scan-only --record-root scripts/fixtures/scan/clean; echo $?` 为 `2` 且输出含 `scan_timeout`。

**W-LV-02.4 `-R` → `-r` 不跟随符号链接** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-21` · design 2.6.3 要点 / 4.5-4
  - DoD：脚本中无 `grep` 大写 `-R`；存在小写 `-rIlF`；符号链接目标内容不计入 `leak_hits`。
  - 证据：`grep -nE 'grep -[A-Za-z]*R' scripts/run-llm-live.sh` 0 命中；`grep -nE 'grep -rIlF' scripts/run-llm-live.sh` ≥1；`bash scripts/run-llm-live.sh --scan-only --record-root scripts/fixtures/scan/symlink` 退出码与 `leak_hits` 证明链接目标未被计入。

### W-LV-03 退出码三分与优先级

**W-LV-03.1 `finish()` 固定优先级 `2>1>采样>0`** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-22` · design 2.6.4
  - DoD：`UNDETERMINED` 只置不消；命中优先于采样失败；零命中 + 采样失败保留采样 `rc`。
  - 证据：`--record-root scripts/fixtures/scan/leak --selftest-sample-rc 7` → `1`；`LLM_LIVE_TIMEOUT_BIN=...timeout-124.sh ... --selftest-sample-rc 7` → `2`；`--record-root scripts/fixtures/scan/clean --selftest-sample-rc 7` → `7`。

**W-LV-03.2 文档/产物扫描协作（显式目录）** · `scripts/scan-secrets-docs.sh`
  - 追溯：`AC-LV-17` · design 2.6.6 / 1.1.2
  - DoD：`scan_artifacts()` 在**显式传入目录**且目录不存在/不可读时返回 `2`；`--all-md` 与无参 `--artifacts` 行为不变。
  - 证据：`bash scripts/scan-secrets-docs.sh --artifacts scripts/fixtures/scan/absent; echo $?` 为 `2`；`bash scripts/scan-secrets-docs.sh --all-md; echo $?` 为 `0`。

**W-LV-03.3 任一 `2` 不被后续 `0` 覆盖** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-22` · design 2.6.3 要点 / 5.4.3-5
  - DoD：`scan-secrets-docs.sh` 返回 `2` 后，即使 `scan-secrets.sh` 返回 `0`，最终仍 `2`。
  - 证据：以 `LLM_LIVE_TIMEOUT_BIN` 桩使文档扫描面缺失的执行序列中，`echo $?` 为 `2`，日志含"不可判定 reason="。

### W-LV-04 自检夹具与超时桩

**W-LV-04.1 三态夹具目录** · `scripts/fixtures/scan/{clean,leak,absent}`
  - 追溯：`AC-LV-16`~`AC-LV-20` · design 2.6.5
  - DoD：`clean` 无敏感串；`leak` 写入**明显假值常量**（不匹配任何真实凭据前缀、不含真实片段）；`absent` 不创建；三者均位于仓库内相对路径。
  - 证据：`ls scripts/fixtures/scan`；`bash scripts/scan-secrets.sh; echo $?` 为 `0`（夹具不触发仓库级密钥扫描）。

**W-LV-04.2 `timeout` 桩** · `scripts/fixtures/timeout-124.sh`
  - 追溯：`AC-LV-18` · design 2.6.5
  - DoD：单行 `exit 124`；可执行；LF 行尾。
  - 证据：`bash scripts/fixtures/timeout-124.sh; echo $?` 为 `124`；`file scripts/fixtures/timeout-124.sh` 不含 `CRLF`。

**W-LV-04.3 符号链接夹具降级（`X-LV-07`）** · `scripts/fixtures/scan/symlink`
  - 追溯：`AC-LV-21` · design `X-LV-07`
  - DoD：优先 `ln -s`，失败降级目录联接，两者皆失败 → `2` + `symlink_unsupported_on_host`；**不得判通过**。
  - 证据：`bash scripts/run-llm-live.sh --scan-only --record-root scripts/fixtures/scan/symlink; echo $?` ∈ `{0,1,2}`，当且仅当输出含 `symlink_unsupported_on_host` 时退出码为 `2`。

**W-LV-04.4 夹具零转录** · `scripts/fixtures/scan/**`
  - 追溯：`AC-LV-27` · design 2.4.1
  - DoD：夹具不含完整端点 URL、真实模型名字面量、`request_id` 值、任何真实凭据片段。
  - 证据：`bash scripts/scan-secrets.sh; echo $?` 为 `0`；`bash scripts/scan-secrets-docs.sh --all-md; echo $?` 为 `0`。

### W-LV-05 离线守卫用例（记录根强绑定）

**W-LV-05.1 `record_root_matches_crate_dir`** · `crates/infrastructure/tests/llm_live_offline.rs`
  - 追溯：`AC-LV-16` · design 2.4.2-(c)-2 / 2.5-U4
  - DoD：断言 `env!("CARGO_MANIFEST_DIR")` 目录名为 `infrastructure`、父目录名为 `crates`；`LiveRecorder::new("t").root()` 以 `.tmp` + `llm-live` 开头；`samples_path()` 文件名为 `samples.jsonl`；用例退出码 `0`。
  - 证据：`cargo test -p meeting-copilot-infrastructure --test llm_live_offline record_root_matches_crate_dir; echo $?` 为 `0`。

**W-LV-05.2 守卫用例零 IO** · `crates/infrastructure/tests/llm_live_offline.rs`
  - 追溯：`AC-LV-16` · design 2.5-U4 注 4
  - DoD：用例不创建目录、不读写 `samples.jsonl`；执行后工作副本无新增 `.tmp/llm-live/t`。
  - 证据：执行上一条后 `test -e .tmp/llm-live/t && echo EXISTS || echo ABSENT` 输出 `ABSENT`（在 crate 目录内执行）。

### W-LV-06 P0 出口硬判据

**W-LV-06.1 四态矩阵 + 三次一致** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-16`~`AC-LV-23` · spec 4.2-1 / design 2.6.5
  - DoD：`clean/leak/absent/timeout桩` 依次得 `0/1/2/2`；连续执行 3 次退出码完全相同。
  - 证据：同一命令组重复 3 次，`echo $?` 序列为 `0 1 2 2`（三次相同）。

**W-LV-06.2 静态扫描面断言** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-16` · `AC-LV-21`
  - DoD：`TMPDIR` 零命中；大写 `-R` 零命中；小写 `-r` ≥1。
  - 证据：`grep -nE 'TMPDIR' scripts/run-llm-live.sh` 0 命中；`grep -nE 'for target in.*TMPDIR'` 0 命中。

**W-LV-06.3 扫描耗时有上限** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-23` · design 2.6.1
  - DoD：`--scan-only` 在存在记录目录时 wall time ≤10 s；无 30 s 挂起。
  - 证据：`time bash scripts/run-llm-live.sh --scan-only` 的 `real` ≤ `10s`。

**W-LV-06.4 关闭 `D-L31-1` 的前置证据** · `docs/progress/b1-status.md`
  - 追溯：`AC-LV-16` · `D-L31-1` · design 2.8-P0
  - DoD：P0 完成后，`run-llm-live.sh` 不再含 `${TMPDIR}`，且四态自检可复现；此为 P1 关闭偏差的唯一依据。
  - 证据：`grep -c 'TMPDIR' scripts/run-llm-live.sh` 为 `0`；`llm_live` 8 个用例名与 `--list --ignored` 计数未变（`8`）。

---

## P1 泄漏面验收与关闭 `D-L31-1`

### W-LV-07 泄漏面四态验收

**W-LV-07.1 四态 + 优先级矩阵验收执行** · `docs/progress/b1-status.md`
  - 追溯：`AC-LV-16`~`AC-LV-23` · design 2.6.5
  - DoD：记录 7 组结果：`clean→0`、`leak→1`、`absent→2`、`timeout桩→2`、`leak+rc7→1`、`timeout桩+rc7→2`、`clean+rc7→7`；每组含命令、退出码、`marker`。
  - 证据：7 条命令逐条 `echo $?` 与期望逐一对齐；同组连续 3 次一致。

**W-LV-07.2 验收记录写入（不含敏感面）** · `docs/progress/b1-status.md`
  - 追溯：`AC-LV-27` · `AC-LV-32` · design 2.4.1
  - DoD：新增小节只写"命令 / 退出码 / `leak_hits=` / `record_root=` / `scan_face=`"，不写凭据值、完整端点 URL、真实模型名、`request_id`。
  - 证据：`bash scripts/scan-secrets-docs.sh --all-md; echo $?` 为 `0`。

**W-LV-07.3 真实收尾（零新增请求）** · `scripts/run-llm-live.sh`
  - 追溯：`AC-LV-25` · `AC-LV-23` · design 2.6.5
  - DoD：以 `--scan-only` 对**已存在的真实记录根**执行，退出码 ∈ `{0,1,2}`；若记录根不存在则必须为 `2`（不得 `0`）；不产生任何 HTTP 请求。
  - 证据：`bash scripts/run-llm-live.sh --scan-only; echo $?`；同时记录目录文件数与执行前一致（`find <root> -type f | wc -l` 不变）。

### W-LV-08 关闭 `D-L31-1`

**W-LV-08.1 偏差关闭说明** · `docs/progress/b1-status.md`
  - 追溯：`AC-LV-16` · `D-L31-1` · design 2.8-P1
  - DoD：原文"未扫 `${TMPDIR}`、`timeout 30` 返回 124"标注为**已关闭**，并写"修复方式：扫描面限定仓库内两处记录根，系统临时目录永久移出扫描面；缺失/超时退 `2`"；保留历史实测数值作为背景，不改写为"已通过"。
  - 证据：`grep -n 'D-L31-1' docs/progress/b1-status.md` 命中行含"已关闭"；`grep -nE '未扫|待修' docs/progress/b1-status.md` 在第 10 节 0 命中。

**W-LV-08.2 `AC-26` 既有结论不被改写** · `docs/progress/b1-status.md`
  - 追溯：`AC-LV-32` · design 2.9.1
  - DoD：第 10.1 节四项运行期核查结论保持原样（不新增、不删改）；仅新增"收尾扫描可自动完成"的说明。
  - 证据：`git diff --stat docs/progress/b1-status.md` 仅显示新增行（无删除行）。

**W-LV-08.3 基线文档零漂移** · `docs/progress/test-names-before.txt`
  - 追溯：`AC-LV-29` · `AC-LV-32` · design 2.9.1
  - DoD：行数仍为 365；`sha256` 不变；`llm_live` 8 条逐字保留。
  - 证据：`wc -l < docs/progress/test-names-before.txt` 为 `365`；`sha256sum docs/progress/test-names-before.txt` 与 P0 前记录一致；`git status --short docs/progress/test-names-before.txt` 为空。

---

## P2 离线注入增强与装配扩展

### W-LV-09 `validate` 收窄（`X-LV-01`，P3 前置硬约束）

**W-LV-09.1 规则改判** · `crates/infrastructure/src/llm/live/record.rs`
  - 追溯：`AC-LV-30` · design 2.4.8 `X-LV-01` / 2.2.2-C
  - DoD：`validate()` 由"任一 `NotMeasured` ⇒ `calls_used==0 && !budget_hit`"改为"**全部**子项 `NotMeasured`（空集合视为全部）⇒ `calls_used==0 && !budget_hit`"；`schema_version`/`data_terms_*`/`tls_trust_root` 校验不变。
  - 证据：`grep -nE 'all\(|any\(' crates/infrastructure/src/llm/live/record.rs` 在 `validate` 段显示 `all`；`cargo test -p meeting-copilot-infrastructure --lib record::; echo $?` 为 `0`。

**W-LV-09.2 新增单测 `partial_not_measured_allows_nonzero_calls`** · `crates/infrastructure/src/llm/live/record.rs`
  - 追溯：`AC-LV-30` · design 2.2.2-C / 2.10-R7
  - DoD：构造 `sub_status = [ac23: measured, ac25: not_measured(construction_conflict)]` + `calls_used = 3` → `validate()` 为 `Ok`；同时保留"全 `not_measured` + `calls_used=1` → `Err`"的反向断言。
  - 证据：`cargo test -p meeting-copilot-infrastructure --lib partial_not_measured_allows_nonzero_calls; echo $?` 为 `0`。

**W-LV-09.3 三种既有组合回归** · `crates/infrastructure/src/llm/live/record.rs`
  - 追溯：`AC-LV-30` · design 2.2.2-C 兼容证明
  - DoD：① 全 `not_measured` + `calls_used=0`（`record.rs` 既有 + `llm_live_offline`）仍通过；② 全 `measured` + 非零调用仍通过；③ 部分 `not_measured` + 非零调用新增通过。
  - 证据：`cargo test -p meeting-copilot-infrastructure --lib record::; cargo test -p meeting-copilot-infrastructure --test llm_live_offline record_fields_complete_and_secret_free` 两条退出码均为 `0`。

### W-LV-10 装配层扩展（只增不改）

**W-LV-10.1 `LiveOptions` 结构** · `crates/infrastructure/src/llm/live/mod.rs`
  - 追溯：`AC-LV-06` · `AC-LV-24` · design 2.2.2-A
  - DoD：四字段全 `Option`（`deadline_override`/`samples_override`/`concurrency_override`/`ledger`）；`Default` 全 `None`；不新增 crate 依赖。
  - 证据：`cargo build -p meeting-copilot-infrastructure --features llm-https; echo $?` 为 `0`；`git diff --stat Cargo.toml crates/infrastructure/Cargo.toml` 为空。

**W-LV-10.2 `assemble_live_with` + `assemble_live` 委托** · `crates/infrastructure/src/llm/live/mod.rs`
  - 追溯：`AC-LV-24` · design 2.1.2-(b) / 2.2.2-B
  - DoD：`assemble_live_with` 为唯一新增装配入口；`assemble_live` 签名、返回、错误语义不变，内部以 `LiveOptions::default()` 委托；既有两个调用点零改动。
  - 证据：`git diff --stat crates/infrastructure/tests/llm_live.rs crates/infrastructure/tests/llm_live_baseline.rs` 在该任务提交中不出现 `assemble_live` 调用行改动；`--max_concurrency` 恒为 1（`grep -n 'max_concurrency' crates/infrastructure/src/llm/live/mod.rs`）。

**W-LV-10.3 截止覆盖边界与默认值** · `crates/infrastructure/src/llm/live/config.rs`
  - 追溯：`AC-LV-06` · design 2.4.5 / 2.5-U3
  - DoD：`try_new(Some(100))` 通过、`Some(99)` 与 `Some(8001)` 拒绝、`None` 仍为 `6000`。
  - 证据：`cargo test -p meeting-copilot-infrastructure --lib overrides_are_bounded; echo $?` 为 `0`；`Some(8001)` 断言已加入该用例。

### W-LV-11 新增离线 429 用例（U1 / U2）

**W-LV-11.1 U1 `rate_limit_backoff_jitter_without_retry_after`** · `crates/infrastructure/tests/llm_degrade_inject.rs`
  - 追溯：`AC-LV-01` · design 2.5-U1 / `X-LV-02`
  - DoD：`Step::Status(429, 无 Retry-After)` + 1 条合法响应；`requests==2`、`attempts==2`、`!degraded`；`FakeClock` 前后差值 `>0` 且 ∈ `[640,960]` ms（偏差 `D-LV-01` 已登记）。
  - 证据：`cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject rate_limit_backoff_jitter_without_retry_after; echo $?` 为 `0`；该用例需以 `gateway_with_config(config, transport, clock.clone())` 装配以读取时钟差值。

**W-LV-11.2 U2 `rate_limit_retry_after_exceeds_remaining_deadline`** · `crates/infrastructure/tests/llm_degrade_inject.rs`
  - 追溯：`AC-LV-02` · design 2.5-U2
  - DoD：`Retry-After=1` + `deadline=300`；`requests==1`、`failure==RateLimited`、`degraded==true`、`attempt==1`（命中"剩余不足"分支而非 `MAX_RETRY_AFTER` 分支）。
  - 证据：`cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject rate_limit_retry_after_exceeds_remaining_deadline; echo $?` 为 `0`。

### W-LV-12 新增离线截止用例（U3）

**W-LV-12.1 `deadline_floor_100ms_only`** · `crates/infrastructure/tests/llm_live_offline.rs`
  - 追溯：`AC-LV-06` · `AC-LV-07`（机制面）· design 2.5-U3
  - DoD：`LiveRunConfig::try_new(Some(100))` → `deadline_ms==100`；`3×Step::Delay(3s)` + `request(deadline=100)` → `failure==Timeout`、`degraded==true`、`requests<=3`；用例实际耗时 <1 s（`Step::Delay` 不真实睡眠）。
  - 证据：`cargo test -p meeting-copilot-infrastructure --test llm_live_offline deadline_floor_100ms_only; echo $?` 为 `0`。

**W-LV-12.2 请求截止字段二次断言** · `crates/infrastructure/tests/llm_live_offline.rs`
  - 追溯：`AC-LV-06` · design 2.4.5
  - DoD：断言 `request` 的 `deadline` 值为 `100`（防止装配层回落默认值）。
  - 证据：同用例内断言存在；`grep -n '100' crates/infrastructure/tests/llm_live_offline.rs` 在 `deadline_floor_100ms_only` 段 ≥2 处（配置值与请求值）。

### W-LV-13 既有 4 条离线用例增强（不改名）

**W-LV-13.1 `rate_limit_backoff` 补精确等待断言** · `crates/infrastructure/tests/llm_degrade_inject.rs`
  - 追溯：`AC-LV-01` · design 2.5.2
  - DoD：合法 `Retry-After=1` → `FakeClock` 差值恰为 `1000 ms`（不走抖动）；`requests==2`、`attempts==2`、`!degraded`。
  - 证据：`cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject rate_limit_backoff; echo $?` 为 `0`。

**W-LV-13.2 `rate_limit_beyond_remaining_budget_degrades` 补 `attempt==1`** · `crates/infrastructure/tests/llm_degrade_inject.rs`
  - 追溯：`AC-LV-02` · design 2.5.2
  - DoD：保留 `Retry-After=5` + `deadline=2000` 语义；新增 `attempt==1` 与"无第二次请求"断言。
  - 证据：`cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject rate_limit_beyond_remaining_budget_degrades; echo $?` 为 `0`。

**W-LV-13.3 `invalid_schema_twice_degrades_with_same_shape` 补同类型断言** · `crates/infrastructure/tests/llm_degrade_inject.rs`
  - 追溯：`AC-LV-11` · design 2.5.2
  - DoD：`attempts==2`、`failure==InvalidSchema`、`degraded==true`，新增 `outcome.body.task() == request.task`。
  - 证据：`cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject invalid_schema_twice_degrades_with_same_shape; echo $?` 为 `0`。

**W-LV-13.4 `invalid_schema_repair` 补修复提示最小化静态断言** · `crates/infrastructure/tests/llm_degrade_inject.rs`
  - 追溯：`AC-LV-12` · design 2.5.2
  - DoD：`attempts==2`、`!degraded`；修复提示构造点只使用字段路径与规则名，不含载荷片段与响应正文。
  - 证据：`cargo test -p meeting-copilot-infrastructure --test llm_degrade_inject invalid_schema_repair; echo $?` 为 `0`；`grep -rnE 'path|rule' crates/infrastructure/src/llm/repair*.rs`（实际文件以实现为准）显示提示构造不使用 `payload`/`body`。

### W-LV-14 既有 8 条真实用例增强落点（行为判定移交 P3）

> 判定口径：本主任务只做**改造落点 + 编译级与静态级 DoD**；行为判定由 `W-LV-20` 在真实终端闭合。逐字保留用例名（`docs/progress/test-names-before.txt:303-310`）。

**W-LV-14.1 `run_gate_skips_without_live`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-30` · `AC-LV-27` · design 2.5.2 / 2.4.1
  - DoD：Skip/Reject 分支只打印固定码（`reason.code()`）；`registry` 三项缺省 `not_measured` 全量写盘；不打印错误 `Display` 原文。
  - 证据：`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live --no-run; echo $?` 为 `0`；`grep -n 'run_gate_skips_without_live' crates/infrastructure/tests/llm_live.rs` = 1 且逐字等于基线。

**W-LV-14.2 `first_token_latency_p50_p95`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-24` · `AC-LV-31` · design 2.5.2
  - DoD：样本 `seq = next_seq()`；台账取 `shared_ledger(ticket.call_limit)`；请求数保持 1。
  - 证据：`--no-run` 编译通过；`--list --ignored` 中该名存在；`grep -n 'seq: 1' crates/infrastructure/tests/llm_live.rs` 0 命中。

**W-LV-14.3 `total_latency_four_tasks`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-24` · `AC-LV-26` · `AC-LV-31`
  - DoD：每次追加前取 `next_seq()`；入口处检查 `abort_flag`；四任务请求数保持 4。
  - 证据：`--no-run` 通过；`grep -n 'abort' crates/infrastructure/tests/llm_live.rs` ≥1。

**W-LV-14.4 `schema_conformance_rate`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-24` · `AC-LV-26` · `AC-LV-31`
  - DoD：同 `W-LV-14.3` 的 `seq` 递推、共享台账与中止检查；不改变 `AC-22` 的分母口径。
  - 证据：`--no-run` 通过；`git diff docs/sdd` 为空（不改写 `AC-22` 口径）。

**W-LV-14.5 `rate_limit_observation`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-03` · `AC-LV-04` · `AC-LV-05`
  - DoD：现有 1 次请求外包只读装饰器（请求数仍 1）；`ac23` 三态 upsert；`http_status` 保证整数；并发常量仍为 1。
  - 证据：`--no-run` 通过；`grep -nE 'loop.*429|while.*429|retry_until_rate_limit|concurrency *= *[2-9]' crates/infrastructure/src/llm/live crates/infrastructure/tests/llm_live.rs` 0 命中。

**W-LV-14.6 `deadline_forced_timeout`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-06` ~ `AC-LV-10`
  - DoD：改用 `LiveOptions{deadline_override: Some(100)}`；单任务单次；`ac24` 三态；标注"人为压低截止时间";不使用默认 `6000`。
  - 证据：`grep -n 'deadline_forced_timeout' crates/infrastructure/tests/llm_live.rs` = 1；`--no-run` 通过。

**W-LV-14.7 `schema_violation_probe_once`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-13` · `AC-LV-14` · `AC-LV-15`
  - DoD：保持 1 次合法请求与请求形态不变；不构造对照请求；`ac25` 由缺省 `not_measured` + `construction_conflict` 给出。
  - 证据：`--no-run` 通过；`git diff crates/infrastructure/src/llm/schema.rs` 不含删除 `required`/`deny_unknown_fields` 的行。

**W-LV-14.8 `record_is_recomputable`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-24` · `AC-LV-31`
  - DoD：`seq` 递推；共享台账；`per_task` 按 `samples.jsonl` 实时行数；请求数保持 1。
  - 证据：`--no-run` 通过；`grep -n 'samples_path' crates/infrastructure/tests/llm_live.rs` 仍指向 `samples.jsonl`。

---

## P3 真实观测隔离、三态/预算与 CI

### W-LV-15 `run_probe` 会话级改造（测试侧脚手架）

**W-LV-15.1 失败信息码化 `safe_code`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-27` · design 2.4.1
  - DoD：失败分支统一映射为固定码（`assembly_failed`/`gateway_failed`/`sample_write_failed`/`unknown`），不透传错误原文。
  - 证据：`grep -nE 'println!.*\{error|format!\("\{error\}"' crates/infrastructure/tests/llm_live.rs` 0 命中。

**W-LV-15.2 进程级共享台账 `shared_ledger`** · `crates/infrastructure/tests/support/live_harness.rs`
  - 追溯：`AC-LV-24` · `AC-LV-25` · design 2.2.2-F / 2.4.4
  - DoD：`OnceLock<Arc<BudgetLedger>>` 进程内唯一；9 个用例共用；`run.json.calls_used` 为会话累计；`call_limit` 仍为 `min(samples×12, cap)`。
  - 证据：`grep -n 'OnceLock' crates/infrastructure/tests/support/live_harness.rs` ≥1；`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live --no-run; echo $?` 为 `0`。

**W-LV-15.3 `next_seq(recorder)` 递推** · `crates/infrastructure/tests/support/live_harness.rs`
  - 追溯：`AC-LV-31` · design 2.1.3-f / 2.2.2-F
  - DoD：`seq = samples.jsonl 现有行数 + 1`；串行前提（`--test-threads=1`）在脚本与文档中固定。
  - 证据：`grep -n 'fn next_seq' crates/infrastructure/tests/support/live_harness.rs` ≥1；`grep -n 'test-threads=1' scripts/run-llm-live.sh` ≥1。

**W-LV-15.4 `SubStatusRegistry` 三项全量快照** · `crates/infrastructure/tests/support/live_harness.rs`
  - 追溯：`AC-LV-30` · design 2.1.3-a / 2.1.3-f-2
  - DoD：`ac23/ac24/ac25` 固定三槽；每次 `write_run` 前 `snapshot()` 返回三项；缺省 `ac23/ac24 = not_measured/not_executed`、`ac25 = not_measured/construction_conflict`；`ac35` 保持既有。
  - 证据：`grep -nE '"ac2[345]"' crates/infrastructure/tests/support/live_harness.rs` 3 处；`grep -n 'not_executed\|construction_conflict' ...` 各 ≥1。

**W-LV-15.5 中止标志 `abort_flag`** · `crates/infrastructure/tests/support/live_harness.rs`
  - 追溯：`AC-LV-26` · design 2.4.7
  - DoD：401/402/403、网络不可达、TLS 失败后置位；`run_probe` 在 `admit` 之后、构造传输之前检查并直接返回，写 `reason_code=aborted_*` 与 `Sample.excluded=aborted_*`。
  - 证据：`grep -nE 'aborted_unauthorized|aborted_network|aborted_tls' crates/infrastructure/tests/llm_live.rs crates/infrastructure/tests/support/live_harness.rs` 各 ≥1。

**W-LV-15.6 `summary.md` 一次性整体生成** · `crates/infrastructure/tests/support/live_harness.rs`
  - 追溯：`AC-LV-30` · design 2.1.3-f-5
  - DoD：`finalize()` 在写 `run.json` 同一步一次性生成内容；首行保留既有风险文案；内容只含 `ac23_*`/`ac24_*` 固定键值与人为截止标注。
  - 证据：`grep -n 'write_summary' crates/infrastructure/tests/llm_live.rs` 在 `finalize` 路径只出现 1 次调用。

### W-LV-16 `ObservingTransport` 与 `ac23` 三态

**W-LV-16.1 只读装饰器与观测载体** · `crates/infrastructure/tests/support/live_observe.rs`
  - 追溯：`AC-LV-03` · `AC-LV-27` · design 2.1.2-(b) / 2.2.2-F
  - DoD：`ObservingTransport` 位于 `Bearer → Observing → Tls` 之间；只采集状态码、`Retry-After` **形式枚举**（`seconds`/`http_date`/`missing`/`invalid`）、两次请求时刻差；不保存 header 原文；`RateLimitObservation` 不落盘。
  - 证据：`grep -nE 'enum .*Form|seconds|http_date|missing|invalid' crates/infrastructure/tests/support/live_observe.rs` 四种形式齐全；`grep -n 'headers' ...` 不出现保存原文的字段。

**W-LV-16.2 `rate_limit_observation` 接线 `ac23`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-03` · `AC-LV-04` · `AC-LV-05` · design 2.1.3-b
  - DoD：请求数保持 1；429 → `ac23=measured / rate_limited_observed`；未出现 → `ac23=not_triggered / window_no_429`；Skip → `ac23=not_measured` + gate 原因码（`C-LV-03`）。
  - 证据：真实运行后 `grep -n '"item": "ac23"' <run 目录>/run.json` 命中且 `status` ∈ `{measured, not_triggered, not_measured}`；`samples.jsonl` 该任务 `http_status` 为整数（`jq 'select(.task=="rate_limit_observation") | .http_status' samples.jsonl` 无非整数）。

**W-LV-16.3 `ac23_not_triggered` 标记与禁用词** · `<run 目录>/summary.md`
  - 追溯：`AC-LV-04` · design 2.1.3-b / 2.4.1
  - DoD：未出现 429 时 `summary.md` 含 `ac23_not_triggered`；`ac23` 相关行不含"通过/达标/pass"；`samples.jsonl` 无 `source=offline`。
  - 证据：`grep -n 'ac23_not_triggered' <run 目录>/summary.md` ≥1；`grep -nE '通过|达标|pass' <run 目录>/summary.md` 在 `ac23` 行 0 命中；`grep -n 'source' <run 目录>/samples.jsonl` 0 命中。

### W-LV-17 `deadline_forced_timeout` 100 ms 单次与 `ac24`

**W-LV-17.1 截止覆盖生效二次断言** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-06` · design 2.4.5
  - DoD：`session.config.deadline_ms == 100` 且请求 `deadline == 100`，否则用例失败（panic）。
  - 证据：`cargo test ... --test llm_live -- --ignored --test-threads=1 deadline_forced_timeout` 退出 `0`（L3，用户终端）。

**W-LV-17.2 超时分类与偏差** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-08` · design 2.1.3-c
  - DoD：`failure_class=="timeout"`、`degraded==true`、`total_ms <= 300`；`outcome.body` 为同类型兜底或空，无半截结构化结果。
  - 证据：`jq 'select(.task=="card_conclusion") | {failure_class, degraded, total_ms}' samples.jsonl` 显示三字段满足；`total_ms` 由单调时钟实测（非 0 常量）。

**W-LV-17.3 单任务单次与 `ac24` 三态** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-07` · `AC-LV-09` · design 2.1.3-c
  - DoD：该用例 `calls_used` 增量 == 1，`samples.jsonl` 新增 == 1 行；超时 → `ac24=measured`；100 ms 内返回 → `ac24=not_triggered / below_floor_100ms` 且不追加请求。
  - 证据：执行前后 `calls_used` 差值为 `1`；`wc -l samples.jsonl` 差值 `1`；`grep -n '"item": "ac24"' run.json` 的 `status`/`reason_code` 合法。

**W-LV-17.4 人为性标注与超时率禁用词** · `<run 目录>/summary.md`
  - 追溯：`AC-LV-10` · design 2.1.3-c
  - DoD：含"人为压低截止时间，非端点自身超时率"；`ac24` 行不含"端点超时率"/`timeout_rate`。
  - 证据：`grep -n '人为压低截止时间' <run 目录>/summary.md` ≥1；`grep -nE '端点超时率|timeout_rate' <run 目录>/summary.md` 在 `ac24` 行 0 命中。

### W-LV-18 新增 1 个 `#[ignore]` 用例（U5）

**W-LV-18.1 `ac25_control_request_skipped_construction_conflict`** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-13` · `AC-LV-14` · design 2.4.3 / 2.5-U5
  - DoD：`#[ignore = "需用户终端注入 LLM_* 环境变量"]`；执行前后 `ledger.used()` 增量 == `0`；零 HTTP 请求；`registry[ac25] == not_measured / construction_conflict`。
  - 证据：`cargo test ... --test llm_live -- --list --ignored | grep -c 'ac25_control_request_skipped_construction_conflict: test'` 为 `1`；真实执行后 `jq '.sub_status' run.json` 含 `ac25` 项。

**W-LV-18.2 三项三态落盘** · `<run 目录>/run.json`
  - 追溯：`AC-LV-30` · design 2.1.3-a
  - DoD：`sub_status` 同时含 `ac23`/`ac24`/`ac25`；三项 `status` ∈ `{measured, not_triggered, not_measured}`；非 `measured` 时 `reason_code` 非空；顶层键仍 23。
  - 证据：`jq '.sub_status | map(.item)' run.json` 含三个 `ac2*`；`jq 'keys | length' run.json` 为 `23`；`cargo test -p ... --lib v2_key_set_is_closed; echo $?` 为 `0`。

**W-LV-18.3 `--list --ignored` 计数由 8 → 9** · `crates/infrastructure/tests/llm_live.rs`
  - 追溯：`AC-LV-28` · `AC-LV-29` · design 2.5.3
  - DoD：`--list --ignored` 恰 9 条；8 个原名逐字存在；不出现改名。
  - 证据：`cargo test -p meeting-copilot-infrastructure --features llm-https --test llm_live -- --list --ignored | grep -cE ': test$'` 为 `9`；8 个原名逐个 `grep -q "::<name>: test$"` 全部成功。

### W-LV-19 CI 静态断言同步

**W-LV-19.1 下界 `-ge 8` → `-ge 9`** · `.github/workflows/ci.yml`
  - 追溯：`AC-LV-28` · design 2.6.7
  - DoD：`llm-https-compile (C2)` 的 `live_count` 断言为 `-ge 9`；`llm_live_baseline` 的 `-ge 1` 不变。
  - 证据：`grep -nE 'test "\$\{live_count\}" -ge' .github/workflows/ci.yml` 显示 `9`。

**W-LV-19.2 8 个原名逐字断言** · `.github/workflows/ci.yml`
  - 追溯：`AC-LV-29` · design 2.6.7
  - DoD：8 个原名以 `grep -q "::<name>: test$"` 逐行断言；任一缺失即失败。
  - 证据：`grep -cE 'grep -q .*::(run_gate_skips_without_live|first_token_latency_p50_p95|total_latency_four_tasks|schema_conformance_rate|rate_limit_observation|deadline_forced_timeout|schema_violation_probe_once|record_is_recomputable)' .github/workflows/ci.yml` 为 `8`（或 8 行循环体）。

**W-LV-19.3 静态边界精确正则（`R9`）** · `.github/workflows/ci.yml`
  - 追溯：`AC-LV-28` · design 2.6.7 / 2.10-R9
  - DoD：`--ignored` 仅允许作为 `--list --ignored` 出现（先剔除合法列举行再断言剩余为空）；`--nocapture`、`run-llm-live`、`LLM_(LIVE|API_KEY|BASE_URL|MODEL|MODEL_CONFIRMED|ALLOWLIST)` 全部 0 命中。
  - 证据：`grep -nE '\-\-ignored' .github/workflows/ci.yml | grep -vE '\-\-list[[:space:]]+\-\-ignored'` 输出为空；`grep -nE '\-\-nocapture|run-llm-live' .github/workflows/ci.yml` 0 命中；`grep -nE 'LLM_(LIVE|API_KEY|BASE_URL|MODEL|MODEL_CONFIRMED|ALLOWLIST)' .github/workflows/ci.yml` 0 命中。

**W-LV-19.4 `feature-coverage (C2)` 产物契约不变** · `.github/workflows/ci.yml`
  - 追溯：`AC-LV-28` · design 1.2.8
  - DoD：`llm-https-coverage-*.txt` 仍含 `compiled=1`、三平台行与 `collect: ...::baseline_30_samples_per_task: test`。
  - 证据：`grep -n 'baseline_30_samples_per_task' .github/workflows/ci.yml` ≥2（生成与校验各一处）；`git diff` 未改 `assert-feature-coverage` 段。

### W-LV-20 真实记录验收（L3，用户终端）

**W-LV-20.1 预算与增量** · `<run 目录>/run.json`
  - 追溯：`AC-LV-24` · `AC-LV-25` · design 2.4.4
  - DoD：`call_limit==360`（`samples=30`、`cap=360`）；`calls_used == 实际请求数`；本特性相对既有 8 探针增量 `0`（优于上限 2，`C-LV-02`）；`budget_hit` 仅在触顶时为 `true`。
  - 证据：`jq '{call_limit, calls_used, budget_hit}' run.json`；`calls_used` 与 `samples.jsonl` 的 `attempt` 求和一致（允许差值等于 `excluded` 行数并需说明）。

**W-LV-20.2 `seq` 唯一单调与 `per_task`** · `<run 目录>/samples.jsonl`
  - 追溯：`AC-LV-31` · design 2.1.3-f
  - DoD：`seq` 无重复；`run.json.per_task == samples.jsonl 行数`。
  - 证据：`jq -r '.seq' samples.jsonl | sort -n | uniq -d` 输出为空；`jq '.per_task' run.json` 等于 `wc -l < samples.jsonl`。

**W-LV-20.3 三态与中止原因** · `<run 目录>/run.json`
  - 追溯：`AC-LV-30` · `AC-LV-26` · design 2.1.3-a / 2.4.7
  - DoD：三项三态合法且非 `measured` 有 `reason_code`；若发生 401/402/403/网络/TLS 失败，`reason_code` 含 `aborted_*` 且首个失败后无后续请求；触顶时 `budget_hit==true`。
  - 证据：`jq -e '.sub_status | length >= 3' run.json` 为真；`jq '.sub_status[] | select(.status != "measured") | .reason_code' run.json` 全非空。

**W-LV-20.4 零转录六面 grep（含记录面例外）** · `<run 目录>`、`docs/**`
  - 追溯：`AC-LV-27` · design 2.4.1 / spec 6.5
  - DoD：记录目录对凭据、完整端点 URL、`request_id` **0 命中**；模型名只允许出现在 `run.json.model`（运行期实际值），`summary.md`/报告/日志 0 命中；`docs/**` 与 `.codeartsdoer/**` 四面 0 命中。
  - 证据：`grep -rnE 'https?://|request_id' <run 目录>` 0 命中；以 `scripts/scan-secrets-docs.sh` 的 `PATTERNS` 首项（凭据前缀模式）对 `<run 目录>` 扫描 0 命中；`bash scripts/scan-secrets.sh; echo $?` 为 `0`；`bash scripts/scan-secrets-docs.sh --all-md; echo $?` 为 `0`；`grep -rn '<真实模型名>' <run 目录>/summary.md` 0 命中（模型名以运行期值为准，不写入文档）。

### W-LV-21 既有门禁不回退

**W-LV-21.1 全量测试基线** · `crates/**`
  - 追溯：`AC-LV-32` · spec 4.5-1 / `C-LV-06`
  - DoD：`cargo test --workspace` 的 `failed=0`；`passed >= 399 + 4`（既有 399 + 新增 4 个离线用例）；`ignored` 由 `8` 增至 `9`（新增 1 条 ignore，属预期增量，不得记为回退）。
  - 证据：`cargo test --workspace 2>&1 | grep -E 'test result'`；`grep -cE 'ignored'` 与 `--list --ignored` 计数一致。

**W-LV-21.2 静态与编译门禁** · 全仓库
  - 追溯：`AC-LV-32` · CI `rust` 作业
  - DoD：`fmt`/`clippy`/`build` 三者退出码均为 `0`；`clippy` 带 `-D warnings`。
  - 证据：`cargo fmt --all --check; echo $?`、`cargo clippy --workspace --all-targets -- -D warnings; echo $?`、`cargo build --workspace; echo $?` 均为 `0`。

**W-LV-21.3 合规五门禁** · `scripts/**`、`docs/**`
  - 追溯：`AC-LV-27` · `AC-LV-32` · CI `compliance` 作业
  - DoD：`check-layering`、`scan-secrets`、`cargo deny check licenses bans sources advisories`、`check-capture-offline --all`、`check-g2-claims` 全部退出 `0`。
  - 证据：五条命令逐条 `echo $?` 均为 `0`；`cargo deny` 不新增告警项。

**W-LV-21.4 用例名差集 `removed=0`** · `scripts/test-name-inventory.sh`
  - 追溯：`AC-LV-29` · design 2.4.6
  - DoD：差集退出 `0`，输出含 `removed=0`；新增条目只计入 added。
  - 证据：`CARGO="$CARGO" bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt; echo $?` 为 `0`，输出含 `removed=0`。

**W-LV-21.5 文档面零漂移** · `docs/sdd/**`、`docs/progress/test-names-before.txt`
  - 追溯：`AC-LV-32` · design 2.9.1
  - DoD：`docs/sdd/**` 无改动；基线 365 行与 `sha256` 不变。
  - 证据：`git status --short docs/sdd docs/progress/test-names-before.txt` 为空；`git diff --stat docs/sdd` 为空。

### W-LV-22 L3 边界复核与偏差登记

**W-LV-22.1 L3 清单与偏差登记** · `docs/progress/b1-status.md`
  - 追溯：`AC-LV-01`~`AC-LV-32` · spec 6.4 / 8.7-4 · `C-LV-01`
  - DoD：验收记录含第 2 节 L3 清单全项；离线项标注"机制面已判定"；`AC-LV-13` 标注"真实面 `not_measured`"；`D-LV-01`（抖动下界差异）已登记且写明"未达标、未放宽"。
  - 证据：`grep -n 'D-LV-01' docs/progress/b1-status.md` ≥1；`grep -nE '机制面' docs/progress/b1-status.md` ≥1；`grep -n 'not_measured' docs/progress/b1-status.md` 在 `AC-LLM-25` 行 ≥1。

**W-LV-22.2 离线结论不得冒充真实结论** · `<run 目录>`、`docs/progress/b1-status.md`
  - 追溯：`AC-LV-04` · `AC-LV-14` · spec 6.4
  - DoD：真实 `samples.jsonl` 无离线注入标记；`AC-LV-01/02/11/12` 的离线结论未写入真实列；`AC-LLM-25` 行同时含"机制面=已判定"与"真实面=not_measured"。
  - 证据：`grep -nE 'source=offline|injected=true' <run 目录>/samples.jsonl` 0 命中；`grep -nE '真实端点在限流|端点违反结构' docs/progress/b1-status.md` 0 命中。

**W-LV-22.3 离线可判定项的最低证据集** · `<run 目录>`、离线用例输出
  - 追溯：`AC-LV-01/02/05/06/11/12/15/16~23/28/29`
  - DoD：每条 L1/L2 项都有"用例名 + 退出码"或"静态 grep 0/≥1 命中"两类证据之一；无遗漏 AC。
  - 证据：`AC-LV-01~32` 逐条在验收表中出现，且每条 `证据` 列为可执行命令。

### W-LV-23 回滚单元与演练

**W-LV-23.1 四个独立回滚单元** · `scripts/**`、`crates/infrastructure/src/llm/live/**`、`crates/infrastructure/tests/**`、`.github/workflows/ci.yml`
  - 追溯：design 2.9.4
  - DoD：R-A 脚本（`run-llm-live.sh` + `scan-secrets-docs.sh` + `scripts/fixtures/**`）、R-B 记录/装配层（`LiveOptions`/`assemble_live_with` + `validate` 收窄）、R-C 测试层（`tests/support/live_*.rs` + 5 新增用例 + 12 增强）、R-D CI（`-ge 9` → `-ge 8` + 8 原名断言保留），四处可**分别**回退且互不阻塞。
  - 证据：每个单元给出 `git checkout -- <文件列表>` 或删除清单（`tests/support/live_observe.rs`、`tests/support/live_harness.rs`），并声明 R-B 与 R-C 的回退顺序（先 C 后 B）。

**W-LV-23.2 回滚后基线判定** · 全仓库
  - 追溯：design 2.9.4 / 2.9.1
  - DoD：仅回退 R-B/R-C 后 `--list --ignored` 回到 `8`；`cargo test --workspace` 回到 `399 passed / 8 ignored`；仅回退 R-A 后四态自检不可用但其余测试不受影响。
  - 证据：回滚演练后 `cargo test --workspace 2>&1 | grep 'test result'` 显示 `passed=399`、`ignored=8`；`--list --ignored | grep -cE ': test$'` 为 `8`。

**W-LV-23.3 `validate` 与三态的同退约束** · `crates/infrastructure/src/llm/live/record.rs`
  - 追溯：design 2.9.4 R-D / `X-LV-01`
  - DoD：若单独恢复"任一 `not_measured` ⇒ 零调用"，则三项三态写盘必须同时回退（否则 `write_run` 失败）；演练须证明"只回退 `validate` 不回退三态"会失败并可定位。
  - 证据：演练记录含 `write_run` 返回错误的固定码（`run.json` 写入失败），且说明"必须与 `W-LV-15.4` 同步回退"。

---

## 附录 A 阶段出口汇总（可机器判定）

| 阶段 | 出口硬判据（全部满足） |
|---|---|
| `P0` | ① `clean/leak/absent/timeout桩` → `0/1/2/2` 且三次一致；② `grep -nE 'TMPDIR' scripts/run-llm-live.sh` 与 `grep -nE 'grep -[A-Za-z]*R'` 均 0 命中、`grep -nE 'grep -rIlF'` ≥1；③ `cargo test -p meeting-copilot-infrastructure --test llm_live_offline record_root_matches_crate_dir` 退出 0 且 `time bash scripts/run-llm-live.sh --scan-only` ≤10 s |
| `P1` | 7 组优先级矩阵全绿；`D-L31-1` 标记"已关闭"；`test-names-before.txt` 365 行与 `sha256` 不变 |
| `P2` | 新增 4 个离线用例退出 0；`partial_not_measured_allows_nonzero_calls` 退出 0；`assemble_live` 两处调用点零改动；12 条增强落点全部编译通过 |
| `P3` | `--list --ignored` 为 9 且 8 原名逐字存在；`ci.yml` 四条静态断言成立；真实记录三项三态 + `seq` 唯一 + 零转录六面 0 命中；`cargo test --workspace` `failed=0` 且既有 399 不回退；差集 `removed=0` |

## 附录 B 回滚单元速查

| 单元 | 回退对象 | 判定 |
|---|---|---|
| `R-A` 脚本 | `scripts/run-llm-live.sh`、`scripts/scan-secrets-docs.sh`、`scripts/fixtures/**` | 四态自检失效，其余测试不受影响 |
| `R-B` 记录/装配层 | `live/mod.rs`（`LiveOptions`/`assemble_live_with`）、`live/record.rs`（`validate` 收窄 + 新单测） | 需与 `R-C` 顺序回退（先 C 后 B） |
| `R-C` 测试层 | `tests/support/live_observe.rs`、`tests/support/live_harness.rs`、5 个新增用例、12 条增强 | `--list --ignored` 回 `8` |
| `R-D` CI | `.github/workflows/ci.yml`（`-ge 9` → `-ge 8`） | C2 通过，但丧失"只增不减"保护 |

## 附录 C 禁止事项（全阶段）

1. 不重命名或删除既有 8 个 `llm_live` 用例；不改 `docs/sdd/**` 与 `docs/progress/test-names-before.txt`。
2. 不新增 `run.json` 顶层键（保持 23 键）；三态与中止原因只进 `sub_status[].reason_code` 与 `Sample.excluded`。
3. 不新增第三方依赖（含 mock 框架、超时库）；不新写第二套 `FakeTransport`。
4. 不绕过 `BudgetedTransport` 直接构造传输；不提高并发、不重复请求直到 429。
5. 不把离线注入结果写入真实 `samples.jsonl`；不把"未执行/超时/缺失"记为通过。
6. 不让 CI 执行 `--ignored`、`--nocapture`、`run-llm-live.sh`，或注入任何 `LLM_*`。