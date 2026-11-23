# 仓库审计与清理 · 实施任务清单（T-AUD-01 ~ T-AUD-35）

> 上游依据：`spec.md`（需求规格）、`design.md`（技术设计，决策 D-AUD-01~18、验收 E-01~16、批次 B-0~B-5、裁决项 Q-01~08）。
> 事实基线（2026-11-23 只读实测）：仓库根 `TSC/`，分支 `main`，HEAD `87eea33`，工作区干净，`main...origin/main` 0/0，跟踪文件 **281**，`docs/progress/test-names-before.txt` **365 行**，`b4-status.md` 最大章节为 `§29`（`§30` 不存在）。
> 本清单只描述"做什么 / 禁止什么 / 怎么验"，不含任何源码改动。

---

## 0. 执行总则（非任务，全部批次共享）

### 0.1 批次与授权形态

| 批次 | 范围 | 是否需用户裁决 | 是否需执行授权 |
|---|---|---|---|
| B-0 | 清单生成 + 试运行（只读，零删除） | 否 | 否（可直接执行） |
| B-1 | 仓库内脚本产物（`.tmp` 顶层文件 / `ui-integration` 非看板 / `test-temp` 内容） | 否（五准则已全满足） | **是**（一次授权） |
| B-2 | 测试夹具 `app/src-tauri/.tmp/ui-store-tests/**` | 否 | **是**（随 B-1 同批授权） |
| B-3 | 灰名单已授权项（`ac-board.md` / `llm-live` / `rollback` / `url-2.5.9-meta`） | **是**（Q-02~Q-05 逐项） | **是** |
| B-4 | 构建缓存 `target/` | **是**（Q-01） | **是（额外授权）** |
| B-5 | 仓库外残留（外层 `.screenshots/`、外层 `.codeartsdoer/temp/accept/`） | **是**（Q-06） | **是** |

### 0.2 四重守护断言（每个批次执行后必跑，失败即中止后续批次）

```bash
# 一次性定义（粘贴到当前 shell，禁止落成仓库内脚本文件）
BASE_TNI_SHA=$(sha256sum docs/progress/test-names-before.txt | cut -d' ' -f1)
guard() {
  test -z "$(git status --porcelain)" || { echo "GUARD FAIL: 工作区不干净"; return 1; }
  test "$(git ls-files | wc -l | tr -d ' ')" = "281" || { echo "GUARD FAIL: 跟踪文件数变化"; return 1; }
  test "$(git rev-parse --short HEAD)" = "87eea33" || { echo "GUARD FAIL: HEAD 变化"; return 1; }
  test "$(wc -l < docs/progress/test-names-before.txt | tr -d ' ')" = "365" || { echo "GUARD FAIL: 基线路数变化"; return 1; }
  test "$(sha256sum docs/progress/test-names-before.txt | cut -d' ' -f1)" = "$BASE_TNI_SHA" || { echo "GUARD FAIL: 基线指纹变化"; return 1; }
  echo "GUARD PASS"
}
guard
```

### 0.3 全局禁止项（任一违反即判越界并回滚对应批次）

1. 禁止新增 `scripts/**` 下任何文件；可执行检查只以"命令清单 + 文档模板"落地，编排辅助只允许写 `.tmp/audit/`（未跟踪）。
2. **禁止使用 `git grep`**：`.codeartsdoer/`（`git ls-files`=0）与 `.tmp/` 均未跟踪，`git grep` 会漏引用 → 一律 `grep -rIn --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules`。
3. 禁止改动源码、`Cargo.toml`/`Cargo.lock`、`deny.toml`、`.github/workflows/ci.yml`、`.gitignore`、`scripts/**`、`app/src/**`。
4. 禁止改写 `b4-status.md` 的 `§25`/`§28`/`§29` 既有字符（只允许在 `§30` 追加；口径修正以"追加指针 + 勘误登记"实现）。
5. 禁止在 `§30` 出现 URL 形态串、`sk-` 前缀串、`Bearer` 授权串、模型名字面量、`request_id` 赋值形态（`check-b4-gates.sh --rule=zero-transcript` 会把 `b4-status.md` 纳入扫描面）。
6. 禁止 `git push`、禁止解锁任何 `#[ignore]`、禁止创建 `tests/replay/`、禁止改写 9/9 ignored 口径。
7. 禁止删除 `docs/progress/test-names-before.txt`、`docs/sdd/*.md`、`docs/progress/*.md`、`scripts/**`、`deny.toml`、`models/**`、`.tmp/test-temp/`（目录本身）。
8. 禁止回显引用扫描命中的正文，报告只记 `文件:行号`。
9. 禁止在无批次清单（manifest）的情况下删除任何路径。
10. 禁止 commit（提交需用户另行指令）。

---

## 1. 前置：事实基线、审计台账与三色清单（B-0，只读）

**组出口判据**：`.tmp/audit/` 下具备基线、来源抽取、17 行台账、manifest.tsv、triage.md、dryrun.txt 六类产物；`guard` 通过；仓库内零删除、零改动（`git status --porcelain` 为空）。

- [ ] **T-AUD-01 采集并冻结事实基线（只读）**
  - 目标：为后续全部任务提供唯一可比对基线（HEAD/跟踪数/洁净度/基线路数与指纹/§30 编号可用性）。
  - 做什么：建 `.tmp/audit/`（含 `sources/`）；记录分支、HEAD、`git status --porcelain`、`git ls-files` 清单与计数、与 `origin/main` 的领先/落后计数、基线文件行数与 sha256；校验 `§29` 恰 1 处、`§30` 恰 0 处。
  - 禁止：不得写入 `docs/**`；不得 fetch/push；不得把 `AUDIT_OUT` 建在仓库外。
  - 命令：
    ```bash
    export AUDIT_ROOT="$PWD"; export AUDIT_OUT="$AUDIT_ROOT/.tmp/audit"
    mkdir -p "$AUDIT_OUT/sources"
    export AUDIT_SNAPSHOT_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    git rev-parse --abbrev-ref HEAD | tee "$AUDIT_OUT/branch.txt"
    git rev-parse HEAD | tee "$AUDIT_OUT/head.txt"
    git status --porcelain > "$AUDIT_OUT/status.txt"; wc -l < "$AUDIT_OUT/status.txt"
    git ls-files > "$AUDIT_OUT/tracked.txt"; wc -l < "$AUDIT_OUT/tracked.txt"
    git rev-list --left-right --count origin/main...HEAD | tee "$AUDIT_OUT/ahead-behind.txt"
    wc -l < docs/progress/test-names-before.txt | tee "$AUDIT_OUT/tni.lines"
    sha256sum docs/progress/test-names-before.txt | tee "$AUDIT_OUT/tni.sha256"
    printf 'sec29=%s sec30=%s\n' "$(grep -cE '^## §29' docs/progress/b4-status.md)" "$(grep -cE '^## §30' docs/progress/b4-status.md)"
    guard
    ```
  - 验收判据：`status.txt` 0 行；跟踪计数 `281`；`tni.lines` = `365`；`sec29=1 sec30=0`；`guard` 输出 `GUARD PASS`。
  - 回滚路径：本任务零写入仓库，仅 `.tmp/audit/` 可整体 `rm -rf` 重来，无回滚需求。
  - 依赖：无。

- [ ] **T-AUD-02 抽取六处来源原文（只读，防人工比对漏项）**
  - 目标：把台账的输入证据固化为可复核的区间快照，并校正行区间。
  - 做什么：先 `grep -n` 定位六个来源的章节起始行，再用 `sed -n` 精确抽区间到 `sources/`；实际区间以本次 `grep -n` 输出为准并写入 `sources/INDEX.txt`。
  - 禁止：不得整目录复制 `docs/progress`；不得修改任何来源文件；抽取结果不得包含凭据形态正文的二次回显（`b4-status.md` 已无该形态，若发现立即停并上报）。
  - 命令：
    ```bash
    grep -nE '^### §(25\.3|25\.4|29\.5|29\.6)' docs/progress/b4-status.md
    grep -nE '^## §(4|5)' docs/progress/handoff-2026-11-22-pui.md docs/progress/handoff-2026-11-22.md
    grep -nE '^## §19|^### §19' .codeartsdoer/specs/ui_integration/tasks.md
    grep -nE '^## ' README.md | head -20
    # 用上一步真实行号替换下面的区间数字
    sed -n '1036,1060p' docs/progress/b4-status.md > "$AUDIT_OUT/sources/b4-25.txt"
    sed -n '1307,1330p' docs/progress/b4-status.md > "$AUDIT_OUT/sources/b4-29.txt"
    sed -n '28,48p'   docs/progress/handoff-2026-11-22-pui.md > "$AUDIT_OUT/sources/handoff-pui.txt"
    sed -n '36,54p'   docs/progress/handoff-2026-11-22.md      > "$AUDIT_OUT/sources/handoff.txt"
    sed -n '1272,1286p' .codeartsdoer/specs/ui_integration/tasks.md > "$AUDIT_OUT/sources/tasks19.txt"
    sed -n '1,40p'    README.md > "$AUDIT_OUT/sources/readme.txt"
    wc -l "$AUDIT_OUT"/sources/*.txt | tee "$AUDIT_OUT/sources/INDEX.txt"
    ```
  - 验收判据：`sources/` 下 6 个文件均非空（行数 > 0）；`INDEX.txt` 记录每份来源的"文件 + 实际行区间"；`b4-25.txt` 含 `§25.3` 与 `§25.4` 标题，`b4-29.txt` 含 `§29.5` 与 `§29.6` 标题。
  - 回滚路径：零仓库写入；`rm -rf .tmp/audit/sources` 后重跑。
  - 依赖：T-AUD-01。

- [ ] **T-AUD-03 生成审计台账 17 行草稿（AUD-01~AUD-17）并做封闭集合与差集校验**
  - 目标：产出 `design.md §2.4.2` 的 17 行台账草稿（8 列齐全），并证明无漏项、无重复、无模糊状态词。
  - 做什么：按 §2.4.2 静态表逐行落 `.tmp/audit/ledger.md`（列序：编号/事项/状态/关闭条件/所需用户条件/取证命令/冲突/来源）；执行四处校验——编号唯一数=17、表行数=17、状态封闭集合外词=0、`AUD-01~07` 中 `closed|passed` 计数=0（仅 AUD-08 允许"本地 passed/远端未取证"）。冲突列对 AUD-16 必须填三处出处。
  - 禁止：不得使用 `AC-B4-` 前缀编号；不得改写文档既有事项名（`U-05`/`U-07-42`/`AC-B4-109` 等原样）；不得把 AUD-01~07 标为 `closed`/`passed`；不得新增第 18 行。
  - 命令：
    ```bash
    grep -E '^\| AUD-' "$AUDIT_OUT/ledger.md" | wc -l                                    # 期望 17
    grep -oE 'AUD-[0-9]{2}' "$AUDIT_OUT/ledger.md" | sort -u | wc -l                     # 期望 17
    grep -E '^\| AUD-' "$AUDIT_OUT/ledger.md" | grep -cE '基本完成|大致可用'              # 期望 0
    grep -E '^\| AUD-0[1-7] ' "$AUDIT_OUT/ledger.md" | awk -F'|' '{print $4}' | grep -cE 'closed|passed'   # 期望 0
    grep -c 'AUD-16' "$AUDIT_OUT/ledger.md"
    grep -cE "U-05|U-07|U-08|AC-B4-109|C-UI-02|C-UI-03|platform-win|platform-mac|codeartsdoer|replay|ignore" "$AUDIT_OUT/ledger.md"
    ```
  - 验收判据：17 / 17 / 0 / 0 四条全满足；`AUD-16` 冲突列含 `§29.5-2`、`§29.6`、`handoff-2026-11-22-pui.md §4-2` 三处出处；覆盖关键词命中行数 ≥ 11（覆盖平台占位、纪律项、治理缺口、口径漂移、CI 缺口）。
  - 回滚路径：仅 `.tmp/audit/ledger.md`，删除重写即可；零仓库影响。
  - 依赖：T-AUD-01、T-AUD-02。

- [ ] **T-AUD-04 三色判定五条准则命令化（C1~C5 可重放）**
  - 目标：让每条候选路径的"可删性"可被独立重放复核，而不是凭印象判定。
  - 做什么：在 shell 中定义 `judge()` 函数（不落文件），对每条候选路径依次输出 C1~C5 结果；C3 允许三种通过形态（有生成者命令 / `cmp` 与受控文件一致 / 判定不可重建→黑名单）。
  - 禁止：不得使用 `git grep`；不得把 `AUDIT_OUT` 下自身产物算作引用命中（需排除 `--exclude-dir=audit`）；不得对 `>1 GB` 目录做逐文件 `grep`（`target/` 只用条目计数判定）。
  - 命令：
    ```bash
    judge() {
      local p="$1"
      if git ls-files --error-unmatch "$p" >/dev/null 2>&1; then echo "C1=FAIL(TRACKED)"; return; fi
      echo "C1=PASS"
      local n
      n=$(grep -rIn --fixed-strings -- "$p" . \
            --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules --exclude-dir=audit \
            2>/dev/null | grep -v "^\./${p#./}" | wc -l)
      echo "C2=$([ "$n" -eq 0 ] && echo PASS || echo "HIT(${n})")"
      echo "C3=需登记生成者或 cmp 比对"
      n=$(grep -rInE 'AC-B4-1[12][0-9]|U-0[578]' "$p" 2>/dev/null | wc -l)
      echo "C4=$([ "$n" -eq 0 ] && echo PASS || echo "HIT(${n})")"
      n=$(grep -rIn --fixed-strings -- "$p" .github scripts docs README.md .codeartsdoer/specs 2>/dev/null | wc -l)
      echo "C5=$([ "$n" -eq 0 ] && echo PASS || echo "HIT(${n})")"
    }
    judge .tmp/deny.toml.bak
    judge .tmp/ui-integration/ac-board.md
    ```
  - 验收判据：`.tmp/deny.toml.bak` 输出 `C1=PASS`、`C2=PASS`、C4/C5 为 `PASS`；`.tmp/ui-integration/ac-board.md` 的 `C2`/`C5` 命中非 0（证伪"可删"，应归灰名单）。
  - 回滚路径：函数仅存在于当前 shell，关闭会话即消失；无仓库影响。
  - 依赖：T-AUD-01。

- [ ] **T-AUD-05 四类载体引用扫描（含未跟踪目录，禁用 `git grep`）**
  - 目标：为三色清单每行提供"引用命中"列的可复核证据，并专门覆盖 `.codeartsdoer/specs`（未跟踪、`git grep` 必漏）。
  - 做什么：对候选路径逐条执行 5 条扫描（全量兜底 / 代码 / 脚本与 CI / 文档 / 规格），再追加"仅文件名反向扫描"（引用可能只写 basename）；输出只保留 `文件:行号`。
  - 禁止：禁止 `git grep`；禁止把命中正文写入报告；禁止把 `.tmp/audit/**` 自身命中计入。
  - 命令：
    ```bash
    scan() { grep -rIn --fixed-strings -- "$1" . \
        --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules --exclude-dir=audit \
        2>/dev/null | grep -v "^\./${1#./}" | cut -d: -f1,2; }
    scanbyname() { grep -rIn --fixed-strings -- "$(basename "$1")" docs scripts .github .codeartsdoer/specs README.md 2>/dev/null | grep -v "^\./${1#./}" | cut -d: -f1,2; }
    scan .tmp/ui-integration/ac-board.md
    scan crates/infrastructure/.tmp/llm-live
    scan docs/progress/test-names-before.txt
    scanbyname .tmp/rollback/ci.bak
    ```
  - 验收判据：`ac-board.md` 命中含 `.codeartsdoer/specs/ui_integration/tasks.md` 行号；`llm-live` 命中含 `crates/infrastructure/src/llm/live/record.rs` 与 `scripts/run-llm-live.sh`；`test-names-before.txt` 命中含 `scripts/test-name-inventory.sh` 与 `.github/workflows/ci.yml`，三者均登记为禁删/灰名单而非可删。每条命中仅 `文件:行号`，无正文。
  - 回滚路径：零写入，重跑即可。
  - 依赖：T-AUD-01。

- [ ] **T-AUD-06 体积与指纹取证（分层口径：≤1 GB 精确、`target/` 估算）**
  - 目标：为批次报告提供"删除前体积/条目数/指纹"前映像，并为 E-07 提供对照值。
  - 做什么：对 `.tmp`、`app/src-tauri/.tmp`、`crates/infrastructure/.tmp` 用 `du -sk` + `find -type f | wc -l`；对 `target/` 先 `timeout 30 du -sk`，超时则改"条目数 + 抽样单文件均值×条目数"并标"估算"；对 `deny.toml.bak` 做 `cmp` 与 `sha256sum` 双取证。
  - 禁止：不得对 `target/` 做全目录逐文件 `sha256sum`；不得跨出会话工作区统计；不得把估算值写成精确值。
  - 命令：
    ```bash
    du -sk .tmp app/src-tauri/.tmp crates/infrastructure/.tmp 2>/dev/null | tee "$AUDIT_OUT/sizes.txt"
    find .tmp -type f | wc -l
    find app/src-tauri/.tmp -type f | wc -l
    find crates/infrastructure/.tmp -type f | wc -l
    timeout 30 du -sk target 2>/dev/null | tee "$AUDIT_OUT/target.size.txt" || echo "target: TIMEOUT -> 改用估算"
    find target -type f | wc -l | tee -a "$AUDIT_OUT/target.size.txt"
    sha256sum .tmp/deny.toml.bak deny.toml | tee "$AUDIT_OUT/deny-bak.sha256"
    cmp -s .tmp/deny.toml.bak deny.toml && echo "deny.toml.bak IDENTICAL" || echo "deny.toml.bak DIFFER"
    ```
  - 验收判据：`sizes.txt` 三行均非空；`deny.toml.bak IDENTICAL`；`.tmp` 顶层 `find .tmp -maxdepth 1 -type f | wc -l` 为 **64**（记录在案）；`target.size.txt` 明确标注"精确"或"估算"。
  - 回滚路径：零写入；`.tmp/audit/*.txt` 可删重跑。
  - 依赖：T-AUD-01。

- [ ] **T-AUD-07 生成 `manifest.tsv` 与 `triage.md`（P-01~P-38 逐条）**
  - 目标：把三色清单落成机器可核销的 15 列 TSV，供 B-1~B-5 逐条勾销。
  - 做什么：按 `design.md §2.3.3(B)` 固定列序 `pid/path/kind/bytes/file_count/tracked/ref_code/ref_script_ci/ref_doc/ref_spec/rebuildable/color/action/basis/batch` 逐行生成；黑名单 P-01~P-13、灰名单 P-20~P-26、可删 P-30~P-38 全部展开（`.tmp` 顶层 64 个文件必须逐个成行，不得合并为通配）；另出 `triage.md` 供评审人目视。
  - 禁止：不得遗漏任何一条顶层文件；`tracked=true` 的行其 `color` 必须为 `BLACK`；`color=GRAY` 的行 `action` 不得为"直接删除"；不得把 `.tmp/audit/**` 自身列进可删（它保留至 T-AUD-33 之后）。
  - 命令：
    ```bash
    # 顶层文件逐条展开（用于核对 64 行是否齐全）
    find .tmp -maxdepth 1 -type f | sort | tee "$AUDIT_OUT/tmp-top.txt" | wc -l
    # manifest 行数与列数自检
    awk -F'\t' 'NF!=15{print "BAD_COLS line "NR": "NF}' "$AUDIT_OUT/manifest.tsv" | head
    awk -F'\t' 'NR>1{print $13}' "$AUDIT_OUT/manifest.tsv" | sort | uniq -c
    awk -F'\t' 'NR>1 && $12=="GRAY" && $13=="直接删除"{print "VIOLATION "NR": "$2}' "$AUDIT_OUT/manifest.tsv"
    awk -F'\t' 'NR>1 && $6=="true" && $12!="BLACK"{print "VIOLATION "NR": "$2}' "$AUDIT_OUT/manifest.tsv"
    grep -cE '^P-3[0-9]' "$AUDIT_OUT/manifest.tsv"
    ```
  - 验收判据：`tmp-top.txt` = 64 行；列数异常 0 条；两条 VIOLATION 均为 0 条；`color` 计数为 BLACK 13 + GRAY 7 + DELETABLE ≥ 40（P-30 展开后总数）；`triage.md` 含"黑/灰/可删"三段与体积合计（可删 ≈ 0.87 M、灰名单上限 ≈ 6.02 G）。
  - 回滚路径：零仓库写入；删除 `.tmp/audit/manifest.tsv`、`triage.md` 重生成。
  - 依赖：T-AUD-04、T-AUD-05、T-AUD-06。

- [ ] **T-AUD-08 试运行（dryrun，零删除）并抽检 3 条存在性**
  - 目标：满足"先清单后执行"，证明计划可执行且未提前动任何文件。
  - 做什么：输出计划（每条路径 + 动作 + 批次）到 `dryrun.txt`；抽检 3 条可删路径（`.tmp/deny.toml.bak`、`.tmp/test-temp` 内任一文件、`app/src-tauri/.tmp/ui-store-tests` 内任一文件）仍存在；再复核黑名单 9 个代表路径存在。
  - 禁止：试运行阶段禁止任何 `rm`/`mv`/`git checkout`；禁止"顺手"清理 `AUDIT_OUT` 以外的目录。
  - 命令：
    ```bash
    awk -F'\t' 'NR>1{printf "%-8s %-12s %-60s batch=%s\n",$1,$13,$2,$15}' "$AUDIT_OUT/manifest.tsv" \
      > "$AUDIT_OUT/dryrun.txt"
    wc -l < "$AUDIT_OUT/dryrun.txt"
    for p in .tmp/deny.toml.bak "$(find .tmp/test-temp -type f | head -1)" "$(find app/src-tauri/.tmp/ui-store-tests -type f | head -1)"; do
      test -e "$p" && echo "EXISTS: $p" || echo "MISSING(!): $p"
    done
    for p in docs/progress/test-names-before.txt docs/sdd/spec.md deny.toml Cargo.toml Cargo.lock README.md .github/workflows/ci.yml models; do
      test -e "$p" && echo "BLACKLIST OK: $p" || echo "BLACKLIST MISSING: $p"
    done
    git status --porcelain | wc -l
    ```
  - 验收判据：`dryrun.txt` 行数 = manifest 行数 - 1；3 条抽检均 `EXISTS`；8 条黑名单均 `BLACKLIST OK`；`git status --porcelain` 0 行。
  - 回滚路径：零删除，无需回滚；若出现 MISSING 立即判越界并停。
  - 依赖：T-AUD-07。

- [ ] **T-AUD-09 灰名单裁决登记骨架（Q-01~Q-08，默认全部保留）**
  - 目标：把 8 个裁决项的"对象/证据/默认动作/前置条件/风险/不可回滚条目数"固化为待填表格，使裁决可逐项勾选，且未裁决项默认保留。
  - 做什么：生成 `decisions.md`，每行格式 `Q-NN | 对象 | 快照证据 | 默认=保留 | 若批准的前置动作 | 风险等级 | 裁决=PENDING`；同时把 Q-07（`.codeartsdoer/` 忽略）与 Q-08（`app/dist` 等四类）明确标注"本轮不做、不得并入清理批次"。
  - 禁止：不得替用户预填 PASS；不得把"默认保留"解释为"已裁决保留"（状态词只能用 `保留`/`待裁决`）；不得在未裁决时把灰名单路径写入任何批次清单的"待删"集合。
  - 命令：
    ```bash
    grep -cE '^Q-0[1-8] ' "$AUDIT_OUT/decisions.md"                 # 期望 8
    grep -c '裁决=PENDING' "$AUDIT_OUT/decisions.md"                # 期望 8
    grep -nE '^Q-0(7|8) ' "$AUDIT_OUT/decisions.md"
    grep -cE '^Q-0[1-8] .*默认=保留' "$AUDIT_OUT/decisions.md"      # 期望 8
    ```
  - 验收判据：8 条 Q 项齐全、全部 `裁决=PENDING`、全部 `默认=保留`；Q-07/Q-08 行含"独立变更单/不并入清理批次"字样。
  - 回滚路径：仅 `.tmp/audit/decisions.md`，可删重建。
  - 依赖：T-AUD-07。

- [ ] **T-AUD-10 B-0 出口校验并生成给用户的一页授权摘要**
  - 目标：确认 B-0 只读完成、零改动，并把"哪些可立即执行、哪些需授权"压缩成一页供用户决策。
  - 做什么：按 `spec §5.3.1-1` 复核"试运行后抽检 3 条仍存在"；复核 `.tmp` 顶层仍 64 文件、`.tmp/ui-integration/ac-board.md` 仍存在、`crates/infrastructure/.tmp/llm-live` 仍存在；生成 `authorize.md`（B-1/B-2 一次授权范围 + B-3/B-4/B-5 逐项裁决项 + 预估回收体积）。
  - 禁止：不得在此任务内执行任何删除；不得把 `target/` 与灰名单项默认写入"已授权"。
  - 命令：
    ```bash
    find .tmp -maxdepth 1 -type f | wc -l                                  # 期望 64
    test -e .tmp/ui-integration/ac-board.md && echo "KEEP ac-board OK"
    test -d crates/infrastructure/.tmp/llm-live && echo "KEEP llm-live OK"
    test -d .tmp/test-temp && echo "KEEP test-temp DIR OK"
    guard
    ls -1 "$AUDIT_OUT"
    ```
  - 验收判据：64 / 三条 KEEP / `GUARD PASS`；`authorize.md` 含"B-1+B-2 预估回收 ≈ 0.88 M、不可回滚 0 条"与"B-3 ≤0.52 M、B-4 ≈5.6 G（估算）、B-5 127 K"。
  - 回滚路径：零写入仓库；`.tmp/audit/**` 可整体清除。
  - 依赖：T-AUD-08、T-AUD-09。

---

## 2. 仓库内脚本产物清理（B-1：需一次执行授权）

**组出口判据**：`.tmp` 顶层文件 64 → 0；`.tmp/ui-integration/ac-board.md` 与 `.tmp/test-temp/` 目录仍在；`guard` 通过；批次记录含逐条三态与前后体积。

- [ ] **T-AUD-11 冻结 B-1 批次清单并逐条复核受控性（授权后第一步）**
  - 目标：把 B-1 待删集合从 manifest 中固化成不可再改的批次前映像，并证明每条均未被跟踪。
  - 做什么：从 `manifest.tsv` 过滤 `batch=B-1` 生成 `b1-manifest.tsv`；对每条执行 `git ls-files --error-unmatch`（必须非 0）；记录每条 `sha256sum`（文件）与条目数/字节数（目录内容）；把 `b1-manifest.tsv` 的 sha256 写入 `b1-freeze.txt`。**明确排除 `.tmp/audit/**`**。
  - 禁止：不得在冻结后再向本批次增删条目；不得把 `.tmp/audit/**`、`.tmp/ui-integration/ac-board.md`、`.tmp/rollback/`、`.tmp/url-2.5.9-meta/` 纳入 B-1。
  - 命令：
    ```bash
    awk -F'\t' 'NR==1 || $15=="B-1"' "$AUDIT_OUT/manifest.tsv" > "$AUDIT_OUT/b1-manifest.tsv"
    awk -F'\t' 'NR>1{print $2}' "$AUDIT_OUT/b1-manifest.tsv" | while read -r p; do
      if git ls-files --error-unmatch "$p" >/dev/null 2>&1; then echo "TRACKED(!) $p"; fi
    done
    grep -E 'audit|ac-board|rollback|url-2\.5\.9-meta' "$AUDIT_OUT/b1-manifest.tsv"
    sha256sum "$AUDIT_OUT/b1-manifest.tsv" | tee "$AUDIT_OUT/b1-freeze.txt"
    ```
  - 验收判据：无 `TRACKED(!)` 输出；上述 `grep` 无命中（证明排除项未被纳入）；`b1-freeze.txt` 含 manifest 指纹。
  - 回滚路径：本任务不删文件；清单作废只需重新从 manifest 派生。
  - 依赖：**用户授权 B-1**；T-AUD-10。

- [ ] **T-AUD-12 删除 `.tmp` 顶层 64 个文件（日志/门禁输出/备份/自检/verify）**
  - 目标：回收 P-30~P-37 顶层部分（约 770.8 K），使 `find .tmp -maxdepth 1 -type f` = 0。
  - 做什么：先记录删除前文件清单与逐文件字节数，再对顶层 `-type f` 执行删除；删除后计数；确认 `.tmp` 顶层目录（`ui-integration`/`test-temp`/`audit`/`rollback`/`url-2.5.9-meta` 等）全部保留。
  - 禁止：不得用 `rm -rf .tmp`；不得 `-maxdepth` 缺省（会递归删子目录内容）；不得删 `.tmp/audit/`、`.tmp/ui-integration/ac-board.md`、`.tmp/rollback/`、`.tmp/url-2.5.9-meta/`、`.tmp/test-temp/`（目录）。
  - 命令：
    ```bash
    find .tmp -maxdepth 1 -type f -printf '%s\t%p\n' | sort -k2 > "$AUDIT_OUT/b1-before-files.tsv"
    wc -l < "$AUDIT_OUT/b1-before-files.tsv"          # 期望 64
    find .tmp -maxdepth 1 -type f -print0 | xargs -0 -r rm -f
    find .tmp -maxdepth 1 -type f | wc -l             # 期望 0
    find .tmp -maxdepth 1 -type d | sort
    guard
    ```
  - 验收判据：删除后顶层文件 0；顶层目录清单仍含 `audit`、`test-temp`、`ui-integration`、`rollback`（如存在）、`url-2.5.9-meta`（如存在）；`guard` 输出 `GUARD PASS`。
  - 回滚路径：不可回滚（未跟踪），可重建——重跑 `bash scripts/check-b4-gates.sh --rule=<r>`、`cargo test`、`npm run build` 即再生对应输出；`.tmp/deny.toml.bak` 若需要可用 `cp deny.toml .tmp/deny.toml.bak` 等价重建。
  - 依赖：T-AUD-11。

- [ ] **T-AUD-13 删除 `.tmp/ui-integration` 非看板 3 文件与 `verify*.{sh,ps1}`**
  - 目标：回收 P-36/P-37（<7 K），同时完好保留取证看板 `ac-board.md`。
  - 做什么：显式列名删除 `lib.sh`、`baseline.txt`、`deny.txt` 与 `verify*.sh`/`verify*.ps1`；删除后立即断言 `ac-board.md` 存在、`ac-board.md` 行数/指纹不变。
  - 禁止：不得通配删除 `ui-integration/*`（会误删 `ac-board.md`）；不得删除 `ac-board.md`（Q-02 默认保留，删除须 B-3 裁决 + 归档）。
  - 命令：
    ```bash
    sha256sum .tmp/ui-integration/ac-board.md > "$AUDIT_OUT/ac-board.before.sha256"
    wc -l < .tmp/ui-integration/ac-board.md
    rm -f .tmp/ui-integration/lib.sh .tmp/ui-integration/baseline.txt .tmp/ui-integration/deny.txt
    rm -f .tmp/verify*.sh .tmp/verify*.ps1
    ls -1 .tmp/ui-integration/
    sha256sum -c "$AUDIT_OUT/ac-board.before.sha256"
    guard
    ```
  - 验收判据：`(1) ui-integration` 目录仅剩 `ac-board.md`；`sha256sum -c` 输出 `OK`；`ls .tmp/verify*.sh` 报"不存在"；`guard` 通过。
  - 回滚路径：不可回滚（未跟踪）；`lib.sh`/`verify*.sh` 为会话脚手架，必要时按设计文档命令块重建，不影响任何门禁。
  - 依赖：T-AUD-11。

- [ ] **T-AUD-14 清空 `.tmp/test-temp` 内容并保留目录本身**
  - 目标：回收 P-35（75 K / 9 文件），同时保留 `TEMP/TMP` 约定根（D-AUD-17）。
  - 做什么：删除 `test-temp` 下一级子项（含子目录），随后 `mkdir -p` 保证目录存在；断言目录存在且内容条目数 0；复跑 `guard`。
  - 禁止：不得删除 `.tmp/test-temp` 目录本身（会使后续 `cargo test` 需要额外 `mkdir`，违背 D-AUD-17）；不得删除 `.tmp` 目录本身。
  - 命令：
    ```bash
    find .tmp/test-temp -mindepth 1 -maxdepth 1 -print0 | xargs -0 -r rm -rf
    mkdir -p .tmp/test-temp
    test -d .tmp/test-temp && echo "DIR KEPT OK"
    find .tmp/test-temp -mindepth 1 | wc -l     # 期望 0
    guard
    ```
  - 验收判据：`DIR KEPT OK`；内容条目 0；`guard` 通过；`ls -d .tmp` 仍存在。
  - 回滚路径：内容不可回滚；目录保留即满足约定，内容由 `cargo test` 再生。
  - 依赖：T-AUD-11。

- [ ] **T-AUD-15 B-1 收尾：逐条三态核销、前后体积对比、批次记录落盘**
  - 目标：产出 B-1 的可审计批次记录（逐条状态，不允许只有汇总数量），并给出"不可回滚条目数 = 0"的依据。
  - 做什么：对 `b1-manifest.tsv` 每条判"已删/失败/跳过"（依据文件存在性）；统计删除前后 `.tmp` 体积与条目数；写入 `b1-record.md`（含批次号、时点、基线提交 `87eea33`、条目数、前/后体积、已删/失败/跳过、不可回滚条目数、结论）；未删条目附原因与解除建议。
  - 禁止：不得只写"已清理 64 个文件"而无逐条状态；不得把失败条目算作成功；不得改写 `AUDIT_OUT` 之外的记录。
  - 命令：
    ```bash
    awk -F'\t' 'NR>1{print $2}' "$AUDIT_OUT/b1-manifest.tsv" | while read -r p; do
      if [ -e "$p" ]; then echo "SKIP/FAIL $p"; else echo "DELETED $p"; fi
    done | tee "$AUDIT_OUT/b1-states.txt"
    grep -c '^DELETED ' "$AUDIT_OUT/b1-states.txt"
    grep -c '^SKIP/FAIL ' "$AUDIT_OUT/b1-states.txt"
    du -sk .tmp | tee -a "$AUDIT_OUT/b1-record.md"
    find .tmp -type f | wc -l
    guard
    ```
  - 验收判据：`b1-states.txt` 行数 = manifest 数据行数；`DELETED` + `SKIP/FAIL` = 全部条目；`b1-record.md` 含 7 个必填字段且"不可回滚条目数 = 0"；`guard` 通过。
  - 回滚路径：批次内失败条目可重试；整批不可回滚但全部可重建（生成者见 manifest `rebuildable` 列）。
  - 依赖：T-AUD-12、T-AUD-13、T-AUD-14。

---

## 3. 测试夹具清理（B-2：随 B-1 同批授权）

**组出口判据**：`app/src-tauri/.tmp/ui-store-tests` 下文件条目 0（目录保留）；`guard` 通过；后续 `cargo test` 能重建夹具。

- [ ] **T-AUD-16 删除 `app/src-tauri/.tmp/ui-store-tests/**` 内容并核销**
  - 目标：回收 P-38（9 K / 6 文件），保留目录结构以免测试需额外建目录。
  - 做什么：记录前映像（逐文件 sha256）→ 删除内容 → 断言条目 0 且目录存在 → 写 `b2-record.md`（逐条三态、前后体积、不可回滚 0）→ 复跑 `guard`。
  - 禁止：不得删除 `app/src-tauri/.tmp` 与 `ui-store-tests` 目录本身；不得删除 `app/src-tauri/src/**`；B-2 未获授权时本任务必须标"未执行（待授权）"，不得静默跳过。
  - 命令：
    ```bash
    find app/src-tauri/.tmp/ui-store-tests -type f -exec sha256sum {} + > "$AUDIT_OUT/b2-before.sha256"
    find app/src-tauri/.tmp/ui-store-tests -type f | wc -l
    find app/src-tauri/.tmp/ui-store-tests -mindepth 1 -maxdepth 1 -print0 | xargs -0 -r rm -rf
    test -d app/src-tauri/.tmp/ui-store-tests && echo "DIR KEPT OK"
    find app/src-tauri/.tmp/ui-store-tests -mindepth 1 | wc -l   # 期望 0
    du -sk app/src-tauri/.tmp | tee -a "$AUDIT_OUT/b2-record.md"
    guard
    ```
  - 验收判据：`DIR KEPT OK`；条目 0；`b2-before.sha256` 行数 = 删除前文件数（6）；`guard` 通过；若未授权则批次记录明确写"未执行（待裁决）"。
  - 回滚路径：不可回滚（未跟踪），由 `cargo test -p <tauri crate> ` 运行时重建；测试夹具为纯输入，重建失败则视为回归风险并上报。
  - 依赖：**用户授权 B-1+B-2**；T-AUD-15。

---

## 4. 灰名单已授权项（B-3：需逐项裁决 + 先归档后删除）

**组出口判据**：每个被批准项都有"结论已归档 + 归档后门禁通过 + 不可回滚条目数显式计数"；未批准项状态为"保留"；`guard` 通过。

- [ ] **T-AUD-17 回填灰名单裁决结果（Q-02~Q-05，逐项 PASS/REJECT/保留）**
  - 目标：把用户裁决与前置动作完成证据绑定，避免"口头同意即删除"。
  - 做什么：在 `decisions.md` 的对应 Q 行填写 `裁决 | 裁决人 | 时点 | 前置动作完成证据（命令+退出码） | 不可回滚条目数`；若为 PASS，逐条核对前置动作是否全部完成（Q-02 需 20/20 AC 归档、Q-03 需 artifacts 扫描命中数已记录、Q-04 需 `git log` 证明三份备份对应变更已在历史、Q-05 需网络可重取或复核结束）。
  - 禁止：不得在缺少前置动作证据时写 PASS；不得把 Q-07/Q-08 并入本批次；不得用"默认保留"掩盖"未裁决"。
  - 命令：
    ```bash
    grep -nE '^Q-0[2345] ' "$AUDIT_OUT/decisions.md"
    grep -c '裁决=PASS' "$AUDIT_OUT/decisions.md"
    grep -cE '^Q-0[1-8] .*裁决=(PENDING|保留|REJECT)' "$AUDIT_OUT/decisions.md"
    grep -cE '^\| AC-B4-1[12][0-9]' .tmp/ui-integration/ac-board.md    # Q-02 佐证：期望 20
    find crates/infrastructure/.tmp/llm-live -maxdepth 1 -type d | wc -l  # Q-03 佐证：期望 7
    git log --oneline -3 -- .github/workflows/ci.yml
    ```
  - 验收判据：每个 Q-02~Q-05 行含"裁决 + 证据 + 不可回滚条目数"三段；写 PASS 的项其前置动作证据齐备；Q-07/Q-08 仍为"本轮不做"。
  - 回滚路径：不删文件，仅改 `decisions.md`，可覆盖重写。
  - 依赖：**用户给出 Q-02~Q-05 裁决**；T-AUD-09。

- [ ] **T-AUD-18 归档结论到 §30 表 C（删除前置，仅追加）**
  - 目标：在删除任何取证产物前，先把它承载的**结论**（不是文件内容）登记到受控文档，满足"取证产物归档优先"。
  - 做什么：把获批项的结论写成 §30 表 C 的"结论归档"列（AC 勾选状态、run 数量与日期、样本统计口径、扫描命中数）；不复制 `samples.jsonl`、不复制 `run.json` 正文；写后立即跑 `scan-secrets-docs.sh --all-md`、`check-g2-claims.sh`、`check-b4-gates.sh --rule=zero-transcript` 三项自检。
  - 禁止：不得复制原始内容/样本行/请求体；不得在 §30 出现五类禁词（URL 形态串、`sk-` 前缀、`Bearer` 串、模型名字面量、`request_id` 赋值）；归档失败（写不进/自检失败）时禁止删除原件，必须转"保留"。
  - 命令：
    ```bash
    bash scripts/scan-secrets-docs.sh --all-md; echo "rc=$?"
    bash scripts/check-g2-claims.sh; echo "rc=$?"
    bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "rc=$?"
    grep -rInE 'https?[:]//' docs/progress/b4-status.md | wc -l        # 期望 0
    grep -rInE 'request_id[:]' docs/progress/b4-status.md | wc -l      # 期望 0
    ```
  - 验收判据：三项门禁均 `rc=0`；两条 grep 命中 0；§30 表 C 中每个获批项均有"结论归档"文字（非"见原文"）。
  - 回滚路径：`b4-status.md` 受控，`git restore -- docs/progress/b4-status.md` 即可回滚本节新增内容。
  - 依赖：T-AUD-17。

- [ ] **T-AUD-19 执行 B-3 已授权项删除（归档完成后才允许）**
  - 目标：仅删除"已 PASS 且已归档"的灰名单项，其余保持保留。
  - 做什么：按获批清单逐项删除（`ac-board.md` 单文件；`llm-live` 目录；`rollback/*.bak` 三文件；`url-2.5.9-meta/` 目录），每项删除前 `git ls-files --error-unmatch` 复核未跟踪，删除后立即 `guard`；对 `llm-live` 删除前必须记录 `scan-secrets-docs.sh --artifacts` 的命中数。
  - 禁止：未获批项一律不得删除；不得删除 `crates/infrastructure/src/**`（源码引用该路径，仅删 `.tmp` 下的记录产物）；不得删除 `.tmp/ui-integration` 目录本身；不得合并删除"审批未覆盖"的相邻路径。
  - 命令：
    ```bash
    bash scripts/scan-secrets-docs.sh --artifacts crates/infrastructure/.tmp/llm-live; echo "hits_rc=$?"
    for p in .tmp/ui-integration/ac-board.md .tmp/rollback/ci.bak .tmp/rollback/design.bak .tmp/rollback/tni.bak; do
      git ls-files --error-unmatch "$p" >/dev/null 2>&1 && echo "TRACKED(!) $p"
    done
    # 逐项按裁决结果执行，示例（仅当 Q-02=PASS 且 T-AUD-18 已通过）：
    # rm -f .tmp/ui-integration/ac-board.md
    # rm -rf crates/infrastructure/.tmp/llm-live
    # rm -f .tmp/rollback/ci.bak .tmp/rollback/design.bak .tmp/rollback/tni.bak
    # rm -rf .tmp/url-2.5.9-meta
    guard
    ```
  - 验收判据：每项删除后 `guard` 通过；未获批项仍存在（`test -e` 断言）；`b3-record.md` 记录每项"已删/失败/跳过"与不可回滚条目数（= 已删且未备份条数，逐项计数而非估算）。
  - 回滚路径：**不可回滚**（未跟踪、无备份）；唯一弥补是 T-AUD-18 的归档结论。此即本批次"不可回滚条目数"必须显式写出的原因。
  - 依赖：T-AUD-18；Q-02~Q-05 裁决。

- [ ] **T-AUD-20 B-3 收尾核销与结论归档列回填**
  - 目标：批次记录与 §30 表 C 对齐，形成"裁决 → 归档 → 删除 → 核销"闭环。
  - 做什么：写 `b3-record.md`（逐条三态 + 前后体积 + 不可回滚条目数 + 结论）；把最终结果回填 §30 表 C；对"失败/跳过"项写明原因（文件占用/权限）与解除建议。
  - 禁止：不得把"未执行（待裁决）"写成"失败"；不得在结论列出现"已完成/已交付"等产品宣称。
  - 命令：
    ```bash
    du -sk .tmp crates/infrastructure/.tmp 2>/dev/null | tee -a "$AUDIT_OUT/b3-record.md"
    test -e .tmp/ui-integration/ac-board.md && echo "KEPT ac-board" || echo "DELETED ac-board"
    test -d crates/infrastructure/.tmp/llm-live && echo "KEPT llm-live" || echo "DELETED llm-live"
    grep -cE '^Q-0[2345] ' "$AUDIT_OUT/decisions.md"
    guard
    ```
  - 验收判据：`b3-record.md` 含批次号/时点/基线提交/条目数/前体积/后体积/已删/失败/跳过/不可回滚条目数/结论 11 项；§30 表 C 与 record 结论一致。
  - 回滚路径：记录文件可重写；已删内容不可回滚（依赖归档结论）。
  - 依赖：T-AUD-19。

---

## 5. 构建缓存 `target/`（B-4：需额外授权 + 绑定重建回归）

**组出口判据**：删除前已记录工具链版本与 `Cargo.lock` 指纹；删除后 `cargo build` 成功且 `cargo test --workspace` 回到 825 passed / 0 failed / 9 ignored；`guard` 通过。

- [ ] **T-AUD-21 B-4 前置取证：工具链版本、锁文件指纹、条目数与体积口径**
  - 目标：让"删缓存后可重建"具备可验证前提，避免事后无法复现。
  - 做什么：记录 `rustc --version`、`cargo --version`、`Cargo.lock` sha256、`target/` 条目数、体积（精确或"估算"标注）、可用磁盘空间；写 `b4-preflight.md`。
  - 禁止：未获 Q-01 授权时不得执行本任务之后的删除动作；不得修改 `Cargo.lock`；不得在记录中省略"估算"标注。
  - 命令：
    ```bash
    rustc --version; cargo --version
    sha256sum Cargo.lock | tee "$AUDIT_OUT/cargo-lock.sha256"
    find target -type f | wc -l | tee "$AUDIT_OUT/target.files"
    timeout 30 du -sk target | tee "$AUDIT_OUT/target.size.txt"
    df -h . | tail -1
    ```
  - 验收判据：`b4-preflight.md` 含工具链版本、锁文件指纹、条目数、体积（含精确/估算标注）、磁盘余量五项；Q-01 授权记录存在。
  - 回滚路径：零删除，无需回滚。
  - 依赖：**用户授权 Q-01（额外授权）**；T-AUD-10。

- [ ] **T-AUD-22 删除 `target/` 并立即触发重建**
  - 目标：回收约 5.6 G（估算），同时用一次全量构建证明"可重建"成立。
  - 做什么：`rm -rf target`（失败则用 PowerShell `Remove-Item -Recurse -Force target` 并记录失败原因）；删除后立刻 `cargo build --workspace`；断言 `target/` 重新出现且 `target/debug` 有产物。
  - 禁止：不得在删除与重建之间执行其他批次；不得跳过重建直接宣称成功；不得删除 `~/.cargo/registry`；不得删除 `models/`。
  - 命令：
    ```bash
    rm -rf target || echo "RM FAILED -> 检查文件占用/长路径"
    test -d target && echo "STILL EXISTS(!)" || echo "TARGET REMOVED"
    export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
    cargo build --workspace 2>&1 | tee "$AUDIT_OUT/b4-build.log"; echo "build_rc=$?"
    test -d target && echo "TARGET REGROWN OK"
    ```
  - 验收判据：`TARGET REMOVED`；`build_rc=0`；`TARGET REGROWN OK`；重建耗时与磁盘变化记入 `b4-record.md`。
  - 回滚路径：可重建（`cargo build`/`cargo fetch`）；不可逆副作用仅为重建耗时与依赖拉取需求，必须在记录中声明。
  - 依赖：T-AUD-21。

- [ ] **T-AUD-23 B-4 重建回归（825/0/9）与批次核销**
  - 目标：证明删除缓存未破坏任何构建输入或用例数量。
  - 做什么：`cargo test --workspace`，抽取各 crate 的 `test result:` 汇总行，合计应为 825 passed / 0 failed / 9 ignored；与 §25.2 基线比对；写 `b4-record.md`（含不可回滚条目数 0 与"重建耗时"说明）；`guard`。
  - 禁止：不得为凑数修改基线计数；不得忽略 `failed` 非 0；不得用 `--no-run` 替代实跑。
  - 命令：
    ```bash
    export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
    export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
    cargo test --workspace 2>&1 | tee "$AUDIT_OUT/b4-test.log"; echo "test_rc=$?"
    grep -E 'test result:' "$AUDIT_OUT/b4-test.log" | tail -20
    grep -cE 'test result: .* 0 failed' "$AUDIT_OUT/b4-test.log"
    guard
    ```
  - 验收判据：`test_rc=0`；`test result:` 行合计 passed=825、failed=0、ignored=9；`guard` 通过；`b4-record.md` 结论为"通过"或"未通过（附偏离项编号）"。
  - 回滚路径：若偏离基线，先判是否为"构建输入缺失"（恢复对应缓存目录/`cargo fetch` 后重跑），禁止直接改写基线；仍失败则按既有偏差流程登记。
  - 依赖：T-AUD-22。

---

## 6. 仓库外残留（B-5：需授权，且严禁越界）

**组出口判据**：仅两条外层路径被处置；外层 `.codeartsdoer` 其余内容（`.codebase`/`skills`/`mcp`/`rule`/`agents`）零改动；处置前后目录清单存档。

- [ ] **T-AUD-24 B-5 越界校验与目录内容存档**
  - 目标：在执行任何外层删除前，证明路径落在会话工作区内且无交付文件。
  - 做什么：校验 `WS_ROOT` 前缀为 `C:/Users/11403/Desktop/xiangmu/TSC`；`ls -R` 存档 `.screenshots` 与 `.codeartsdoer/temp/accept`；确认 `.screenshots` 下仅空 `.tmp`；确认 `temp/accept` 仅临时文件；记录外层 `.codeartsdoer` 顶层条目清单用于事后比对。
  - 禁止：禁止操作 `C:/Users/11403` 主目录、系统目录、仓库外其他项目；禁止触碰 `.codeartsdoer/{.codebase,skills,mcp,rule,agents}`；禁止递归删除 `.codeartsdoer`。
  - 命令：
    ```bash
    WS_ROOT="C:/Users/11403/Desktop/xiangmu/TSC"
    case "$WS_ROOT" in "C:/Users/11403/Desktop/xiangmu/TSC") echo "IN_WORKSPACE OK";; *) echo "OUT_OF_SCOPE(!)"; exit 1;; esac
    ls -R "$WS_ROOT/.screenshots" 2>/dev/null || echo "no .screenshots"
    find "$WS_ROOT/.screenshots" -mindepth 1 | sort | tee "$AUDIT_OUT/b5-screenshots.txt"
    find "$WS_ROOT/.codeartsdoer/temp/accept" -type f 2>/dev/null | tee "$AUDIT_OUT/b5-accept.txt" | wc -l
    ls -1 "$WS_ROOT/.codeartsdoer" | tee "$AUDIT_OUT/b5-codeartsdoer-top.txt"
    ```
  - 验收判据：`IN_WORKSPACE OK`；`b5-screenshots.txt` 仅 1 条（空 `.tmp` 目录）；`b5-accept.txt` 4 条文件且无交付文档特征；`b5-codeartsdoer-top.txt` 已存档。
  - 回滚路径：零删除；存档文件可删。
  - 依赖：**用户授权 Q-06**；T-AUD-10。

- [ ] **T-AUD-25 B-5 处置外层残留并核销**
  - 目标：回收 127 K 与空目录，且事后可证明未误动外层配置。
  - 做什么：删除空 `.screenshots/.tmp` 后删 `.screenshots`（仅当确认为空）；删除 `temp/accept` 目录；随后重列 `.codeartsdoer` 顶层与 `b5-codeartsdoer-top.txt` 比对（必须完全一致）；写 `b5-record.md`（不可回滚条目数 = 实际删除且未备份条数）。
  - 禁止：不得使用通配删除外层目录；不得在 `.codeartsdoer` 下执行任何 `rm -rf *`；发现任何交付文件立即停并上报（"遇到缺依据先停"）。
  - 命令：
    ```bash
    WS_ROOT="C:/Users/11403/Desktop/xiangmu/TSC"
    rmdir "$WS_ROOT/.screenshots/.tmp" 2>/dev/null
    rmdir "$WS_ROOT/.screenshots" 2>/dev/null
    rm -rf "$WS_ROOT/.codeartsdoer/temp/accept"
    ls -1 "$WS_ROOT/.codeartsdoer" > "$AUDIT_OUT/b5-codeartsdoer-after.txt"
    diff "$AUDIT_OUT/b5-codeartsdoer-top.txt" "$AUDIT_OUT/b5-codeartsdoer-after.txt" && echo "OUTER_CONFIG UNCHANGED"
    test -d "$WS_ROOT/.codeartsdoer/temp/accept" && echo "ACCEPT STILL EXISTS(!)" || echo "ACCEPT REMOVED"
    guard
    ```
  - 验收判据：`OUTER_CONFIG UNCHANGED`（diff 为空）；`ACCEPT REMOVED`；`guard` 通过；`b5-record.md` 写明"外层无 git → 不可回滚条目数 = N"。
  - 回滚路径：**不可回滚**（外层无版本控制）；空目录可用 `mkdir -p` 重建，IDE 临时文件由工具再生。
  - 依赖：T-AUD-24。

---

## 7. 门禁与全量回归（只读执行，逐规则调用）

**组出口判据**：`spec §5.3.1-6/7` 的全部命令退 0；`test-name-inventory.sh --diff` 输出 `removed=0`，`cargo test --workspace` 为 825/0/9。

- [ ] **T-AUD-26 静态门禁回归：B4 七规则 + P-UI 六规则（逐 `--rule=` 调用）**
  - 目标：证明清理未破坏既有静态门禁判定，且未触碰门禁语义。
  - 做什么：逐个规则调用两个门禁脚本（**不使用 `--all`**，两脚本均无该开关）；任一非零即记录规则名与命中行。
  - 禁止：禁止使用 `--all`（会退 2，属误用而非门禁失败）；禁止新增或修改规则；禁止在门禁失败时通过改脚本"修复"。
  - 命令：
    ```bash
    for r in source budget record outbound zero-transcript targets doc-plan; do
      bash scripts/check-b4-gates.sh --rule="$r"; echo "b4:$r rc=$?"
    done
    for r in credential-redline settings-ux card-render body-hidden frontend-offline zero-transcript; do
      bash scripts/check-ui-integration.sh --rule="$r"; echo "pui:$r rc=$?"
    done
    ```
  - 验收判据：13 条 `rc=0`；若 `doc-plan` 退 2，先确认 `.codeartsdoer/specs/b4_gate/spec.md` 存在（不得删除 `.codeartsdoer/`），再复跑。
  - 回滚路径：门禁为只读；失败时先按命中行定位，若因误删受控文件则 `git restore` 后复跑。
  - 依赖：B-0 完成（无需清理完成即可先跑一次作对照）；最终需在 T-AUD-25 之后复跑。

- [ ] **T-AUD-27 CI 同源门禁回归 + 用例名零回归差集**
  - 目标：覆盖与 `ci.yml` 同入口的 6 个脚本，并证明用例名集合零回归。
  - 做什么：按 `design §2.9.3-4/5` 顺序执行 6 个脚本 + `test-name-inventory.sh --diff`；记录每项退出码与 `removed=`/`added=` 数值。
  - 禁止：不得改用 `git grep` 或自造扫描面；不得修改 `test-names-before.txt`；不得把 `removed` 非 0 解释为"无害"。
  - 命令：
    ```bash
    bash scripts/check-layering.sh;            echo "layering rc=$?"
    bash scripts/scan-secrets.sh;              echo "secrets rc=$?"
    bash scripts/check-capture-offline.sh --all; echo "capture rc=$?"
    bash scripts/check-g2-claims.sh;           echo "g2 rc=$?"
    bash scripts/scan-secrets-docs.sh --all-md; echo "docs-secrets rc=$?"
    bash scripts/check-mac-scope.sh --rule=claims; echo "mac rc=$?"
    bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt | tee "$AUDIT_OUT/tni-diff.txt"
    grep -E 'removed=|added=' "$AUDIT_OUT/tni-diff.txt"
    ```
  - 验收判据：6 条 `rc=0`；`removed=0`（`added` 允许非 0 但必须登记，且不得由本任务引入新用例名）。
  - 回滚路径：只读；失败时定位是"误删基线/扫描面"还是"文档新增触发宣称扫描"，分别用 `git restore` 或改写措辞解决。
  - 依赖：T-AUD-26。

- [ ] **T-AUD-28 格式、clippy 与全量测试回归（825/0/9）**
  - 目标：证明清理与文档收口未影响编译、告警与用例数量。
  - 做什么：`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`；从小结行合计 passed/failed/ignored。
  - 禁止：不得用 `--fix`/自动格式化改动源码；不得放宽 clippy 告警等级；不得 `--no-run`。
  - 命令：
    ```bash
    export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
    export TEMP="$PWD/.tmp/test-temp"; export TMP="$TEMP"; mkdir -p "$TEMP"
    cargo fmt --all --check; echo "fmt rc=$?"
    cargo clippy --workspace --all-targets -- -D warnings; echo "clippy rc=$?"
    cargo test --workspace 2>&1 | tee "$AUDIT_OUT/final-test.log"; echo "test rc=$?"
    grep -E 'test result:' "$AUDIT_OUT/final-test.log" | tail -20
    ```
  - 验收判据：`fmt rc=0`、`clippy rc=0`、`test rc=0`；合计 825 passed / 0 failed / 9 ignored（若期间基线已变，以 T-AUD-01 实测为对照并登记偏差，禁止改基线）。
  - 回滚路径：只读；失败时区分"构建输入缺失"与"真实用例失败"，前者恢复缓存目录、后者登记偏离。
  - 依赖：T-AUD-27；若执行 B-4 则为 T-AUD-23 的重复确认。

---

## 8. 文档口径收口与 §30 登记（仅改文档，受控可回滚）

**组出口判据**：`§30` 编号唯一并含 §30.1~§30.8；`§25/§28/§29` 原字符零改动（仅允许行尾追加指针）；README 未删未完成段落；五类禁词命中 0；三项门禁退 0。

- [ ] **T-AUD-29 C-UI-02 口径冲突举证与 ERR-01 勘误登记**
  - 目标：把 `§29.5-2`"未裁决"、`§29.6`"保留（已裁决）"、`handoff-pui §4-2`"未裁决"三处冲突登记为 ERR-01，并按"用户裁决 > 批次状态 > 交接"取 `§29.6` 为最终口径。
  - 做什么：三处并列取证（只读）；在 §30.6 表 E 写 ERR-01（冲突双方"文件 + 章节"、采用口径、依据出处）；采用口径描述为"保留 macOS CI 编译观察项，`ci.yml`/`deny.toml`/`platform-mac` 零改动"。
  - 禁止：禁止删除或改写 `§29.5-2`、`§29.6` 原句；禁止只写"已统一"而不列双方出处；禁止在勘误中出现模型名/URL/凭据形态。
  - 命令：
    ```bash
    grep -n "C-UI-02" docs/progress/b4-status.md
    grep -n "C-UI-02" docs/progress/handoff-2026-11-22-pui.md
    grep -n "C-UI-02" .codeartsdoer/specs/ui_integration/tasks.md
    grep -c "C-UI-02.*未裁决" docs/progress/b4-status.md
    grep -c "C-UI-02.*保留" docs/progress/b4-status.md
    ```
  - 验收判据：举证输出显示 `§29.5`（未裁决）与 `§29.6`（保留）并存；§30.6 表 E 的 ERR-01 行同时含三个出处与最终口径；`git diff` 显示 `§29.5/§29.6` 段无删除行。
  - 回滚路径：`git restore -- docs/progress/b4-status.md`。
  - 依赖：T-AUD-03（AUD-16 已登记）。

- [ ] **T-AUD-30 追加 §30 小节（§30.1~§30.8，只追加不改写）**
  - 目标：把台账、三色摘要、批次记录、验收证据、勘误、不宣称、交接八块落地为受控文档的独立小节。
  - 做什么：追加前校验 `§30` 计数 0 与 `§29` 计数 1；按 `design §2.10.3` 骨架写 8 个子节；表 A 放 17 行台账（来自 T-AUD-03）、表 B 放黑/灰/可删计数与灰名单裁决结果、表 C 放 B-1~B-5 批次记录（含"未执行（待裁决）"标记）、表 D 放 E-01~E-16、表 E 放 ERR-01、§30.7 原样放不宣称八条、§30.8 放交接四要素；可选行尾指针：在 `§29.5-2` 句末**追加**"（口径见 §30.6 ERR-01）"。
  - 禁止：禁止改写 `§25/§28/§29` 任何字符（**仅允许行尾追加指针**，不得替换原句）；禁止新增受控文件；禁止在表 C 中把"未执行"写成"通过"；禁止逐条抄写 `UI/tasks.md §19` 的"--all"表述（该表述与脚本能力不符，应写"逐 `--rule=` 调用"）。
  - 命令：
    ```bash
    grep -cE '^## §30' docs/progress/b4-status.md        # 追加前期望 0，追加后期望 1
    grep -cE '^## §29' docs/progress/b4-status.md        # 期望 1
    grep -cE '^### §30\.[1-8] ' docs/progress/b4-status.md
    grep -cE '^\| AUD-[0-9]{2} \|' docs/progress/b4-status.md      # 期望 17
    git diff --numstat -- docs/progress/b4-status.md
    git diff -U0 -- docs/progress/b4-status.md | grep -cE '^-[^-]'   # 期望仅 §29.5 指针行的 0 或 1
    ```
  - 验收判据：`§30` 恰 1 个、`§30.1~§30.8` 恰 8 个、`AUD-` 表行 17；`git diff` 删除行数为 0（若采用行尾指针则为 0，因为只是追加同一行的后半句时会产生 1 行替换 → 该情形必须改为"另起一行追加"以保持删除行 0）。
  - 回滚路径：`git restore -- docs/progress/b4-status.md`。
  - 依赖：T-AUD-20、T-AUD-25、T-AUD-28、T-AUD-29。

- [ ] **T-AUD-31 校准 README「当前进度」（仅替换表述，不删段落）**
  - 目标：使 README 进度表述与台账状态一致，且不产生任何能力完成宣称。
  - 做什么：先读出「当前进度」段并逐句比对台账；仅替换与台账不一致的状态措辞；若已一致则本任务标 `not-applicable` 并记录理由（不得为"显得有产出"而硬改）。
  - 禁止：不得删除"未完成"段落、平台占位表述、§文档索引链接；不得新增"已完成/已交付/已通过"组合；不得改动 `README.md` 之外的章节。
  - 命令：
    ```bash
    grep -nE '^## ' README.md | head -20
    sed -n '1,40p' README.md
    git diff --numstat -- README.md
    grep -nE "U-05.*(已完成|已交付)|U-07.*(已完成|已交付)|U-08.*(已完成|已交付)" README.md | wc -l   # 期望 0
    ```
  - 验收判据：`git diff` 仅含替换/追加行、删除行 0（或仅为被替换的同义句，需在记录中逐条说明）；禁止组合命中 0；"未完成"字样仍存在。
  - 回滚路径：`git restore -- README.md`。
  - 依赖：T-AUD-30。

- [ ] **T-AUD-32 §30 禁词与宣称自检（写后立即执行）**
  - 目标：确保新增小节不触发 `zero-transcript` / `g2-claims` / `mac-scope` 三条约束。
  - 做什么：对 `b4-status.md` 与 `README.md` 执行五类禁词扫描（URL 形态串 / `sk-` 前缀 / `Bearer` 串 / 模型名字面量 / `request_id` 赋值），再复跑三项门禁。
  - 禁止：不得为了让扫描通过而删除已登记的台账行（应改写措辞）；不得使用被 `check-mac-scope.sh:114` 禁止的字面量表达三平台结论。
  - 命令：
    ```bash
    grep -rInE 'https?[:]//' docs/progress/b4-status.md README.md | wc -l
    grep -rInE 'sk-[A-Za-z0-9]{8,}' docs/progress/b4-status.md README.md | wc -l
    grep -rInE 'Bearer[ ]' docs/progress/b4-status.md README.md | wc -l
    grep -rInE 'request_id[:]' docs/progress/b4-status.md README.md | wc -l
    grep -rInE '(deepseek|gpt-4|gpt-3|claude)-[A-Za-z0-9]' docs/progress/b4-status.md README.md | wc -l
    bash scripts/check-b4-gates.sh --rule=zero-transcript; echo "zero rc=$?"
    bash scripts/check-g2-claims.sh; echo "g2 rc=$?"
    bash scripts/check-mac-scope.sh --rule=claims; echo "mac rc=$?"
    bash scripts/scan-secrets-docs.sh --all-md; echo "md-secrets rc=$?"
    ```
  - 验收判据：5 条 grep 命中均为 0；4 条门禁 `rc=0`。**说明**：以上命令中的禁词模式仅用于自检，禁止复制进 `§30` 正文。
  - 回滚路径：改写措辞后重跑；仍失败则 `git restore` 该小节并重写。
  - 依赖：T-AUD-30、T-AUD-31。

---

## 9. 收口验收与交接

**组出口判据**：E-01~E-16 全部有"命令 + 退出码 + 关键输出"；不宣称清单逐条核对通过；交接四要素齐全；批次完成度矩阵与用户授权范围一致。

- [ ] **T-AUD-33 回填验收证据表（E-01~E-16）**
  - 目标：把每条验收判据变成可复现证据行，供评审人独立重放。
  - 做什么：逐条执行 `design §2.9.1` 的 E-01~E-16 命令，记录退出码与关键输出行，写入 §30.5 表 D 与 `evidence.md`。
  - 禁止：不得只写"已通过"而无命令与输出；不得在 E-04 中把 AUD-08 的"本地 passed/远端未取证"算作失败（该条允许 1 例）；不得改写 E-15 基线计数。
  - 命令：
    ```bash
    grep -cE '^\| AUD-[0-9]{2} \|' docs/progress/b4-status.md                       # E-01: 17
    grep -oE 'AUD-[0-9]{2}' docs/progress/b4-status.md | sort -u | wc -l            # E-02: 17
    grep -E '^\| AUD-' docs/progress/b4-status.md | grep -cE '基本完成|大致可用'      # E-03: 0
    grep -E '^\| AUD-0[1-8] ' docs/progress/b4-status.md | awk -F'|' '{print $4}' | grep -cE 'closed|passed'  # E-04: 1
    grep -n "AC 汇总.*130" docs/progress/b4-status.md                                # E-05
    grep -c "^NEW_TARGETS='" scripts/test-name-inventory.sh                          # E-06: 1
    wc -l < docs/progress/test-names-before.txt                                      # E-07: 365
    git status --porcelain | wc -l                                                   # E-08
    git ls-files | wc -l                                                             # E-09: 281
    git rev-parse --short HEAD                                                        # E-10
    find .tmp -maxdepth 1 -type f | wc -l                                             # E-12
    grep -E 'removed=|added=' "$AUDIT_OUT/tni-diff.txt"                               # E-13
    grep -E 'test result:' "$AUDIT_OUT/final-test.log" | tail -20                      # E-15
    grep -rInE 'https?[:]//' docs/progress/b4-status.md | wc -l                        # E-16: 0
    ```
  - 验收判据：E-01=17、E-02=17、E-03=0、E-04=1、E-05 含 130、E-06=1、E-07=365、E-09=281、E-11 黑名单全在、E-13 `removed=0`、E-15=825/0/9、E-16=0；`git status` 仅含预期的 `b4-status.md`/`README.md` 改动（提交前）。
  - 回滚路径：只读为主；文档改动 `git restore`。
  - 依赖：T-AUD-28、T-AUD-32。

- [ ] **T-AUD-34 不宣称清单逐条核对与关键词自检**
  - 目标：防止"文件已清理/门禁全绿"被误读为"能力完成"。
  - 做什么：按 `design §2.9.2` 八条逐条核对 §30.7 与 README；执行三条关键词自检；把核对结果写入 `evidence.md`。
  - 禁止：不得删除 §30.7 任一条；不得出现"U-05/U-07/U-08 + 已完成/已交付"、"真机/真实联网 + 已通过/已验证"组合；不得宣称远端 CI 绿。
  - 命令：
    ```bash
    grep -nE "U-05.*(已完成|已交付)|U-07.*(已完成|已交付)|U-08.*(已完成|已交付)" docs/progress/b4-status.md README.md | wc -l
    grep -nE "真机.*(已通过|已验证)|真实联网.*(已通过|已验证)" docs/progress/b4-status.md README.md | wc -l
    grep -nE "远端 ?CI.*(已绿|通过)" docs/progress/b4-status.md README.md | wc -l
    grep -cE '^[0-9]\. ' <(sed -n '/^### §30.7/,/^### §30.8/p' docs/progress/b4-status.md)
    ```
  - 验收判据：三条组合命中均为 0；§30.7 条目数 = 8。
  - 回滚路径：改写措辞；必要时 `git restore` 该小节。
  - 依赖：T-AUD-33。

- [ ] **T-AUD-35 出口判定矩阵、交接四要素与最终冻结**
  - 目标：给出"本任务做到哪一步"的确定性结论，并留下新会话可直接续用的交接信息。
  - 做什么：生成批次完成度矩阵（B-0 必完成；B-1/B-2/B-3/B-4/B-5 标注"完成/未执行（待授权）/部分完成"）；核对 §30.8 交接四要素（当前 HEAD、跟踪文件数、保留目录约定、未完成项入口路径）；写入 `AUD-15`/`AUD-17` 的未关闭状态与 Q-07/Q-08 的"本轮不做"记录；最后跑一次 `guard` 与三项门禁。
  - 禁止：不得把"未执行（待裁决）"标为"完成"；不得在结论中出现能力完成宣称；不得在未获指令时执行 `git add`/`git commit`。
  - 命令：
    ```bash
    grep -cE '^\| B-[0-5] ' docs/progress/b4-status.md          # 批次矩阵行数
    grep -nE 'HEAD|跟踪文件|保留目录|未完成项入口' docs/progress/b4-status.md | sed -n '/§30.8/,$p'
    grep -nE 'AUD-15|AUD-17' docs/progress/b4-status.md
    guard
    bash scripts/test-name-inventory.sh --diff docs/progress/test-names-before.txt | grep -E 'removed=|added='
    git status --porcelain
    ```
  - 验收判据：批次矩阵 B-0~B-5 六行齐全且状态与本轮实际执行一致；§30.8 四要素齐全；`AUD-15`/`AUD-17` 仍为未关闭；`guard` 通过；`removed=0`；`git status` 仅含预期文档改动。
  - 回滚路径：文档改动 `git restore -- docs/progress/b4-status.md README.md`；清理批次已执行部分按各批次 record 回滚/重建。
  - 依赖：T-AUD-34。

---

## 10. 需要用户授权才能执行的任务（出口判据）

> 下表列出**不能由执行体自行决定**的任务。未获授权时，对应任务状态必须写"未执行（待授权/待裁决）"，不计入回收体积，且**不影响** T-AUD-01~T-AUD-10、T-AUD-26~T-AUD-35 的收口判定（可交付"仅 B-0 + 文档收口"形态）。

| 任务 | 授权类型 | 授权对象 | 出口判据（DoD） | 不可回滚 |
|---|---|---|---|---|
| T-AUD-11 ~ T-AUD-15 | 一次执行授权 | B-1 范围（`.tmp` 顶层 64 文件 + `ui-integration` 非看板 3 文件 + `verify*.{sh,ps1}` + `test-temp` 内容） | `.tmp` 顶层文件 64→0；`ac-board.md` 与 `.tmp/test-temp/` 仍在；`b1-record.md` 逐条三态、不可回滚条目数 = 0；四重守护通过 | 否（全部可重建） |
| T-AUD-16 | 随 B-1 同批授权 | B-2 范围（`ui-store-tests/**`） | 目录保留、内容条目 0；`b2-record.md` 齐全；后续 `cargo test` 不再缺夹具 | 否（测试重建） |
| T-AUD-17 ~ T-AUD-20 | 逐项裁决 Q-02/Q-03/Q-04/Q-05 + 执行授权 | B-3 灰名单 | 每项有"裁决 + 前置动作证据 + 结论已归档 + 归档后门禁 rc=0"；未获批项仍存在；`b3-record.md` 逐项显式写出不可回滚条目数 | **是**（未跟踪、无备份；唯一弥补为归档结论） |
| T-AUD-21 ~ T-AUD-23 | **额外授权**（Q-01，需独立于 B-1 授权） | B-4 `target/`（≈5.6 G 估算） | 删除前记录工具链版本 + `Cargo.lock` sha256 + 条目数 + 体积口径（精确/估算）+ 磁盘余量；删除后 `cargo build --workspace` rc=0；`cargo test --workspace` 回到 825/0/9；重建耗时已登记 | 否（可重建；不可逆副作用为重建耗时/依赖拉取） |
| T-AUD-24 ~ T-AUD-25 | 裁决 Q-06 + 执行授权 | B-5 外层 `.screenshots/`（空）+ 外层 `.codeartsdoer/temp/accept/`（127 K） | 目标路径前缀校验为会话工作区内；外层 `.codeartsdoer` 顶层 `diff` 为空；无交付文件；`b5-record.md` 写明不可回滚条目数 | **是**（外层无 git；空目录可 `mkdir -p` 重建） |
| （不做，仅登记） | 裁决 Q-07 / Q-08 | 解除 `.codeartsdoer/` 忽略；`app/dist`、`app/node_modules`、`*.cdx.json`、`*.tsbuildinfo` 清理 | 本轮仅登记为"本轮不做"，Q-07 必须走独立变更单，不得与清理批次合并 | — |

### 10.1 可立即执行（无需任何裁决/授权）的任务范围

- **全量**：T-AUD-01 ~ T-AUD-10（B-0，只读，零删除）。
- **门禁与回归**（只读执行）：T-AUD-26、T-AUD-27、T-AUD-28。
- **文档收口**（仅改受控文档，可 `git restore` 回滚）：T-AUD-29、T-AUD-30、T-AUD-31、T-AUD-32。
- **收口验收**：T-AUD-33、T-AUD-34、T-AUD-35。
- 该范围单独完成时，交付形态为"审计台账 + 三色清单 + 试运行 + 批次定义 + 门禁全绿 + §30 登记"，**回收体积为 0**，B-1~B-5 全部标记"未执行（待授权）"，属合法收口形态（`design §2.8.1`）。

### 10.2 任务依赖拓扑（关键路径）

```
T-AUD-01 → 02 → 03 → 07 → 08 → 10
T-AUD-01 → 04 ↗        ↗
T-AUD-01 → 05 ↗        ↗
T-AUD-01 → 06 ↗        ↗
T-AUD-09 ↗
T-AUD-10 → [授权] → 11 → 12 → 15 → 16(B-2)
                       13 ↗
                       14 ↗
T-AUD-09 → 17 → 18 → 19 → 20
T-AUD-10 → [Q-01] → 21 → 22 → 23
T-AUD-10 → [Q-06] → 24 → 25
{done B-1..B-5} → 26 → 27 → 28
T-AUD-03 → 29 → 30 → 31 → 32
{28,32} → 33 → 34 → 35
```

### 10.3 任务总量

- 主任务组 **10** 组（1~10，其中第 10 组为授权汇总，非任务）。
- 可执行任务 **35** 项（T-AUD-01 ~ T-AUD-35）。
  - 无需授权：**20** 项（T-AUD-01~10、T-AUD-26~28、T-AUD-29~32、T-AUD-33~35）。
  - 需先授权/裁决：**15** 项（T-AUD-11~25），按批次归并为 **4 个授权单元**（B-1+B-2 / B-3 / B-4 / B-5）；其中**实际发生删除动作的有 7 项**：T-AUD-12、T-AUD-13、T-AUD-14（B-1）、T-AUD-16（B-2）、T-AUD-19（B-3）、T-AUD-22（B-4）、T-AUD-25（B-5）。
- 需求覆盖：spec §5.1（审计 17 行）→ T-AUD-02/03；§5.2（三色）+§5.2.3（名单）→ T-AUD-04/05/06/07；§5.3（执行与验收）→ T-AUD-08/11~25/26~28；§5.4（文档收口）→ T-AUD-29~32；§6（数据约束）→ T-AUD-03/07/15/20/23/33。