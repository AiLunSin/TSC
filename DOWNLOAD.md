# 下载与使用指南（傻瓜版）

适用系统：**Windows 10 / 11 64 位**。

先看 [Releases](https://github.com/AiLunSin/TSC/releases) 里有没有 `Meeting Copilot_<版本>_x64-setup.exe`：

- **有**：下载双击安装，装完直接跳到第 5 节做首次配置。下面的 1～4 节是从源码编译的路线，可以跳过。
- **没有**（还没有正式发版）：按 1～4 节从源码生成一个可以双击运行的 `meeting-copilot-desktop.exe`。整套工具只需要装一次，以后每次使用直接双击 exe。

不管走哪条路，**实时转写都要 Python 3.8+ 和 `pip install vosk`**：识别脚本是 Python 写的，程序会在 PATH 里找能 `import vosk` 的解释器。安装包只带识别模型，不带 Python 运行时。

> 这是开发预览版，不是成熟商业软件。请不要在参会人不知情的情况下录音或转写；遵守你所在公司制度和当地法律。

---

## 0. 先看要准备什么

| 项目 | 要求 |
|---|---|
| 系统 | Windows 10 64 位（建议较新的版本）或 Windows 11 |
| 内存 | 建议 8 GB 以上 |
| 磁盘 | 建议预留 5 GB 以上（编译依赖比较占空间） |
| 网络 | 首次编译需要联网下载依赖；日常转写可离线 |
| 账号 | DeepSeek API Key（可选，用于 AI 释义/提醒）；博查或 Tavily Key（可选，用于联网查） |
| 不需要 | 虚拟声卡、额外麦克风、账号注册；安装开发工具时需要管理员权限，日常运行不需要 |

程序抓的是**系统正在播放的声音**（WASAPI 环回），所以开会时只要电脑能听到对方声音，就能转写，不需要装 VB-Cable 之类的虚拟声卡。

---

## 1. 安装基础工具

下面每装完一个，都用"检查命令"确认一下，避免后面报错找不到原因。

### 1.1 WebView2（显示界面用）

Windows 11 一般自带。Windows 10 如果没有，打开：

https://developer.microsoft.com/microsoft-edge/webview2/

下载 **Evergreen Bootstrapper**，双击安装即可。

### 1.2 Visual Studio Build Tools（编译 C++ 依赖用）

打开：

https://visualstudio.microsoft.com/visual-cpp-build-tools/

下载安装器，运行后勾选：

- **使用 C++ 的桌面开发**

右侧默认会带 MSVC 编译器和 Windows SDK，保持默认即可，然后点安装。

如果你已经装过 Visual Studio 2022，打开 Visual Studio Installer → 修改 → 勾上同一个工作负载也可以。

### 1.3 Rust（把源码变成 exe 的工具）

打开：

https://rustup.rs/

下载 `rustup-init.exe`，双击，直接按回车选默认安装（default host 应是 `x86_64-pc-windows-msvc`）。

安装完成后，**关闭当前 PowerShell，重新开一个**，执行：

```powershell
cargo --version
rustc --version
```

正常应显示 `cargo 1.8x.x`、`rustc 1.8x.x`。本项目要求 Rust **1.89 或更高**。

### 1.4 Node.js（编译前端界面用）

打开：

https://nodejs.org/

下载 **LTS** 版本（20 或 22 都可以），一路下一步安装。安装时保持默认的 "Add to PATH"。

重新开一个 PowerShell，执行：

```powershell
node -v
npm -v
```

能显示版本号即可。

### 1.5 Python 与 vosk（本地语音识别用）

打开：

https://www.python.org/downloads/windows/

下载 **64 位 Python 3.10 或更高版本**。安装第一页务必勾选：

- **Add python.exe to PATH**

安装完成后，重新开 PowerShell，执行：

```powershell
py -3 --version
py -3 -m pip install --upgrade pip
py -3 -m pip install vosk
py -3 -c "import vosk; print('vosk OK')"
```

最后一行输出 `vosk OK` 才算成功。

> 不要用 Microsoft Store 里的 Python 存根。程序会依次尝试 `python`、`py -3`、`python3`，只认其中能 `import vosk` 的那个。

### 1.6 下载源码

打开项目主页：

https://github.com/AiLunSin/TSC

点绿色 **Code** → **Download ZIP**，解压后把文件夹放到：

```text
C:\TSC
```

也就是 `C:\TSC\Cargo.toml` 必须存在。**路径尽量短、不要带空格和特殊符号**，可以少踩很多坑。

如果你装过 Git，也可以直接：

```powershell
git clone https://github.com/AiLunSin/TSC.git C:\TSC
```

检查：

```powershell
Test-Path C:\TSC\Cargo.toml
```

返回 `True` 就对了。

---

## 2. 下载中文语音模型

程序不自带语音模型，需要单独下载一次（约 52 MB）。

在 PowerShell 里逐行执行：

```powershell
cd C:\TSC
New-Item -ItemType Directory -Force .tmp\asr | Out-Null
Invoke-WebRequest "https://alphacephei.com/vosk/models/vosk-model-small-cn-0.22.zip" -OutFile ".tmp\asr\model.zip"
Expand-Archive ".tmp\asr\model.zip" -DestinationPath ".tmp\asr" -Force
Remove-Item ".tmp\asr\model.zip"
```

检查目录：

```powershell
Test-Path C:\TSC\.tmp\asr\vosk-model-small-cn-0.22\am
```

返回 `True` 说明解压正确。

正确的层级是：

```text
C:\TSC\.tmp\asr\vosk-model-small-cn-0.22\am
C:\TSC\.tmp\asr\vosk-model-small-cn-0.22\conf
...
```

如果多套了一层（例如 `vosk-model-small-cn-0.22\vosk-model-small-cn-0.22`），程序会提示资源缺失，把里面那层挪出来即可。

---

## 3. 编译（复制粘贴即可）

在 `C:\TSC` 打开 PowerShell，依次执行：

```powershell
cd C:\TSC
cd app
npm ci
npm run build
cd ..
cargo build -p meeting-copilot-desktop --features llm-https
```

说明：

- `npm ci` 安装前端依赖，`npm run build` 生成界面文件。
- `cargo build ... --features llm-https` 编译桌面程序，并**启用真实网络传输**。
- **只改过前端、界面却没变**：Tauri 把界面文件在编译期压进 exe，个别情况下增量编译会继续用旧资源。最稳的做法是用仓库自带脚本一次做完（构建前端 → 清桌面 crate → 重编）：

```bash
bash scripts/build-desktop.sh
```不加这个参数，AI 释义和联网查会提示"未启用传输"。
- 第一次编译会下载大量 Rust 依赖，屏幕会滚动很久，属于正常现象；不要中途关窗口。
- 如果 PowerShell 报 `npm.ps1 无法加载，因为在此系统上禁止运行脚本`，把上面的 `npm` 换成 `npm.cmd`，或先执行：

```powershell
Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
```

然后重开 PowerShell。

检查是否生成成功：

```powershell
Test-Path C:\TSC\target\debug\meeting-copilot-desktop.exe
```

返回 `True` 即成功。

---

## 4. 启动程序

```powershell
cd C:\TSC
.\target\debug\meeting-copilot-desktop.exe
```

也可以直接在资源管理器里双击：

```text
C:\TSC\target\debug\meeting-copilot-desktop.exe
```

如果 Windows 弹出"Windows 已保护你的电脑"：

1. 点 **更多信息**。
2. 点 **仍要运行**。

这是因为本地编译的 exe 没有代码签名，不是病毒。若双击没反应，右键 exe → **属性** → 勾选 **解除锁定** → 确定，再试。

启动后是一个两页窗口，标题是"会议副驾 Meeting Copilot"，顶栏右侧可在 **工作 / 设置** 之间切换。工作页是三栏：左侧状态灯、中间转写区、右侧关键词卡片。程序不需要登录，也没有安装向导。

---

## 5. 首次配置

### 5.1 配置 AI 模型（可选）

不配置也能用本地转写、手动出卡；配置后才能用"查意思（模型释义）""AI 提醒 / 怎么回"。

1. 点顶栏右侧的 **设置** 页签。
2. 在「模型与凭据」卡的 **访问凭据** 里粘贴你的 **DeepSeek API Key**。
   - Key 在 DeepSeek 开放平台创建：https://platform.deepseek.com/api_keys
   - 这里必须是 DeepSeek 的 Key；模型服务地址是程序内置的，不能填其他厂商的地址。
3. 点 **测试连接**。
   - 显示"已验证"：成功。
   - 显示"验证失败"：看第 7 节。
4. 点 **保存设置**。
5. 点顶栏 **工作** 回到工作页。

说明：

- 密钥保存在 Windows 凭据管理器，界面只显示掩码，不落明文、不进日志。
- 模型名和服务地址两栏是只读的，显示 `deepseek-flash` 和"默认服务"属于正常，不用改。
- 如果测试提示模型不存在，可以在启动前临时换模型名（以 DeepSeek 官方当前可用模型名为准）：

```powershell
$env:LLM_MODEL = "deepseek-chat"
.\target\debug\meeting-copilot-desktop.exe
```

### 5.2 配置联网查（可选）

1. 切到 **设置** 页，找到「联网查资料」卡。
2. 选择服务：**博查 Bocha（中文）** 或 **Tavily（英文 / 通用）**。
3. 在「博查 Key」「Tavily Key」两栏分别粘贴对应服务的 Key（用哪家填哪家，留空不修改）。
4. 每小时上限默认 20 次，`0` 表示不限，最大 200。
5. 点 **保存**。

之后在转写区选中不超过 80 字的文字，点 **联网查**，才会真的发出请求。程序不会自动把整场会议发出去。

### 5.3 调整 AI 提醒窗口（可选）

设置页 **辅助回复窗口** 卡：

- 提醒看最近：默认 5 句，可调 3 到 10。
- 上下文：默认 12 句，可调 5 到 20。

不改也能用。改完点 **保存**。

### 5.4 选择状态与卡片的显示方式（可选）

设置页 **界面显示** 卡：

- **悬停显示**（默认）：左侧状态灯只显示颜色，右侧卡片只显示关键词，鼠标移上去才展开文字与详情。
- **常显**：两侧固定展开，一直能看到文字，代价是中间工作区变窄。

同卡还可以勾选 **转写时自动出卡** 与 **生成卡片时自动补来源**。选择会记在本机（`ui.json`），下次打开还是这个方式。

---

## 6. 开始使用

### 6.1 开会前

1. 打开腾讯会议、飞书、微信或钉钉，进入会议。
2. 确认电脑能正常听到对方声音（默认播放设备、音量、没有静音）。
3. 不需要虚拟声卡，也不需要把麦克风对着音箱。

程序顶部会自动检测会议窗口，命中后显示应用名并开始计时。腾讯会议和飞书只要存在可见窗口即可；微信和钉钉要求窗口标题里带"会议/通话/视频/语音/直播/共享"等关键词。

### 6.2 实时转写

1. 在中栏「实时转写」面板右上点 **开始实时转写**。
2. 看左侧状态灯（鼠标移上去，或在设置页选"常显"才能看到文字）：
   - **系统音频通道**：绿色 = 就绪。
   - **字幕文本通道**：琥珀色 = 降级，腾讯会议/飞书可能如此，属正常，见第 7 节。
   - **本地 ASR**：绿色 = 运行就绪，琥珀色 = 运行中降级。
3. 看面板标题下方的诊断信息，正常应看到：

```text
音频 12.3s（96 片）：...
```

数字会一直增长，说明正在采集系统声音。

4. 会议里有人说话时，中栏转写区会出现文字。刚开始可能是半句，稳定后成句。
5. 点 **停止转写** 结束。停止不会清空已转写内容，方便回看；关闭程序才会清空。

默认 **只转写、不出卡**。想要自动出卡，去设置页「界面显示」勾选 **转写时自动出卡**。即使打开，也有两道限流：同一个词 90 秒内不重复出卡，每分钟最多 8 张。

### 6.3 手动粘贴生成卡片

没有实时音频、或者只想整理一段文字时：

1. 在中栏底部展开 **手动粘贴会议文本生成卡片**。
2. 粘贴会议记录后点 **生成卡片**。
3. 卡片出现在右侧卡片栏里；鼠标移上去（或选"常显"）可看到结论与「查看详情」。

### 6.4 选中文字查意思

1. 在中栏转写区里用鼠标选中一个词或一句话（最多 200 字）。
2. 点转写框正下方出现的 **查意思**。
3. 结果会标注来源：
   - **模型释义**：已配置 Key 且编译启用了网络传输。
   - **本机上下文**：没有 Key、网络失败或未启用传输时的本地回退。
4. 需要留档就点 **生成卡片**，标题是选中的文字，结论是释义摘要。

选中多句或整段（至少 24 字且至少 2 句）时，会自动切换成"概括模式"，给出一句话概括和最多两个关键信息。

### 6.5 联网查

1. 选中不超过 80 字的词或短语。
2. 点 **联网查**。
3. 返回最多 5 条标题、站点、摘要和地址。
4. 再点 **生成卡片**，这次检索到的站点会作为来源写进卡片。

默认只有你点按钮才出网。也可以在设置页「界面显示」勾选 **生成卡片时自动补来源**，这样点"生成卡片"时会顺带查一次；查不到就静默退回本机来源。

### 6.6 AI 提醒 / 怎么回

1. 点转写框右下角的 **AI 提醒 / 怎么回**。
2. 程序只读取最近 12 句（不超过 900 字），其中提醒部分只看最后 5 句。
3. 返回：
   - 两条提醒：下一步注意什么、可能被追问什么。
   - 1 到 2 条可以直接说出口的回应，可逐条复制。

没配置 Key 时会直接提示去设置页配置，不会假装成功。

### 6.7 卡片怎么看

- 右侧卡片栏平时只显示关键词；鼠标移上去（或用 Tab 聚焦）展开类型、结论、引用条数、置信度与「查看详情」。
- 点 **查看详情** 会在中栏展开本地详情，不会发起检索、不会调模型、不会外发。
- 来源未验证或引用对齐失败的卡片仍会显示，但按钮为「不可点击」并写明原因。
- 卡片栏顶部的 **清空** 只清空卡片列表，不影响转写内容。

### 6.8 隐私相关

- 转写文字只在内存里，单次最多 600 句，关闭程序即清空，不落盘。
- 只有三种情况会出网，而且都要你主动点击：
  - **查意思**：选中片段（最多 200 字）加前后各一句。
  - **联网查**：选中片段（最多 80 字）。
  - **AI 提醒 / 怎么回**：最近 12 句加选中句（最多 900 字）。
- 不会自动上传整场转写。
- Key 存在 Windows 凭据管理器，两个 Key 分开存，互不覆盖。

---

## 7. 常见问题

### 7.1 编译阶段

| 现象 | 原因 | 怎么办 |
|---|---|---|
| `cargo` 不是内部或外部命令 | Rust 没装或 PATH 没生效 | 重装 rustup，关闭并重开 PowerShell |
| `cargo --version` 显示 `rustfmt`，或报 `Unrecognized option` | PATH 里有旧的 rustup 代理 | 把 `%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin` 放到 PATH 最前面，或重装 rustup |
| `link.exe not found` / `cl.exe` 找不到 | 没装 C++ 生成工具 | 回到 1.2，勾选"使用 C++ 的桌面开发" |
| `npm` 不是内部或外部命令 | Node 没装或 PATH 没生效 | 重装 Node LTS，重开 PowerShell |
| `npm.ps1 无法加载` | PowerShell 执行策略限制 | 用 `npm.cmd`，或执行 `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned` |
| 编译中 MSVC 内存不足 / 写 PDB 失败 | 调试信息占用太大 | 同一窗口先执行 `$env:CARGO_PROFILE_DEV_DEBUG='0'`，再重跑 `cargo build` |
| 编译成功但界面还是旧的 | Tauri 把界面压进 exe，增量编译复用了旧资源；或 WebView2 缓存了旧 index.html | ① 用 `bash scripts/build-desktop.sh` 重编（内含 `cargo clean -p`）；② 关掉程序后把 `%LOCALAPPDATA%\com.meetingcopilot.desktop\EBWebView` 改名备份，再启动 |

### 7.2 转写阶段

| 现象 | 原因 | 怎么办 |
|---|---|---|
| 点开始转写后一直没有文字 | Python 里没有 vosk | 执行 `py -3 -c "import vosk; print('vosk OK')"`；不行就重装 vosk |
| 提示"实时转写资源缺失" | 模型目录层级不对 | 确认 `C:\TSC\.tmp\asr\vosk-model-small-cn-0.22\am` 存在 |
| 显示 `音频 0.0s（0 片）：未采到系统声音` | 系统没有声音输出，或默认设备不对 | 播放一段声音；检查默认播放设备、音量、静音；换设备后重启程序 |
| 音频秒数在涨，但没有文字 | 声音太小、环境太吵，或会议用的是其他输出设备 | 调大音量；确认会议声音从默认设备播放；靠近麦克风说话测试 |
| 腾讯会议/飞书"字幕文本通道"一直降级 | 这两个窗口当前读不到可用字幕文本 | 正常现象，用"系统音频通道/本地 ASR"转写即可 |
| 自动出卡太多或太少 | 自动出卡默认关闭，且有 90 秒/每分钟 8 张限流 | 用"选中文字 → 查意思 → 生成卡片"手动控制 |

### 7.3 配置与网络

| 现象 | 原因 | 怎么办 |
|---|---|---|
| 测试连接提示密钥无效 | Key 错了、复制多了空格，或不是 DeepSeek Key | 重新复制，确认没有空格；确认是 DeepSeek 开放平台的 Key |
| 测试连接提示余额不足 | DeepSeek 账户没有可用余额 | 到 DeepSeek 平台充值或换 Key |
| 测试连接超时 | 网络不通、代理/VPN 干扰 | 换网络，暂时关闭代理/VPN 再试 |
| 测试提示模型不存在 | 内置模型名与当前账号可用模型不一致 | 按 5.1 的 `$env:LLM_MODEL` 方式换模型名启动 |
| 查意思只出"本机上下文" | 没配 Key，或编译没加 `llm-https` | 配 Key；用 `cargo build -p meeting-copilot-desktop --features llm-https` 重新编译 |
| 联网查提示"当前构建未启用传输" | 编译没加 `llm-https` | 同上，重新编译 |
| 联网查提示"未配置" | 没保存检索 Key | 展开"联网查资料"，填 Key 并保存 |
| 联网查超限 | 达到每小时上限 | 等下一个小时，或在折叠块里调大上限（最大 200，`0` 为不限） |

### 7.4 启动与系统

| 现象 | 原因 | 怎么办 |
|---|---|---|
| 双击后没窗口 | WebView2 缺失，或 exe 被系统锁定 | 安装 WebView2；右键 exe → 属性 → 解除锁定 |
| SmartScreen 拦截 | 本地编译的 exe 没有签名 | 更多信息 → 仍要运行 |
| 杀毒软件报警 | 无签名的新 exe 容易被误报 | 确认来源是你自己编译的；不要把整个杀毒软件长期关闭 |
| 程序启动后很快退出 | 缺少运行库或资源配置错误 | 用 PowerShell 启动，看窗口里的报错；设置页"打开日志"查看日志 |

日志位置：

```text
%LOCALAPPDATA%\com.meetingcopilot.desktop\logs\desktop.log
```

设置页的 **打开日志** 会直接在资源管理器里定位。

---

## 8. 更新、备份与卸载

### 8.1 更新

如果用的是 Git：

```powershell
cd C:\TSC
git pull
cd app
npm ci
npm run build
cd ..
cargo clean -p meeting-copilot-desktop
cargo build -p meeting-copilot-desktop --features llm-https
```

如果用的是 ZIP：把新 ZIP 解压到新目录，把旧的 `.tmp\asr\vosk-model-small-cn-0.22` 复制过去，再按第 3 节重新编译。

更新不会删除你的 Key 和设置，它们不在源码目录里。

### 8.2 备份

- Key 无法导出，换电脑需要重新在设置页填一次。
- 设置文件在：

```text
%APPDATA%\com.meetingcopilot.desktop
```

- 转写内容不落盘，没有历史记录可备份；需要留存请在界面里自行复制。

### 8.3 卸载

1. 打开程序，设置页点 **清除设置**；「联网查资料」卡按需点两行 Key 旁的 **清除**。
2. 关闭程序。
3. 删除整个 `C:\TSC` 文件夹。
4. 可选：删除 `%APPDATA%\com.meetingcopilot.desktop` 和 `%LOCALAPPDATA%\com.meetingcopilot.desktop`。
5. 可选：打开 Windows **凭据管理器** → **Windows 凭据**，删除：

```text
meeting-copilot/llm-api-key
meeting-copilot/search-api-key-bocha
meeting-copilot/search-api-key-tavily
```

用安装包装的，走「设置 → 应用 → 已安装的应用」卸载，会删掉程序目录和注册表项，但不会动 `%APPDATA%` 和系统凭据，上面第 1、4、5 步仍要手动做。
从源码跑的没有注册表项，直接删文件夹即可。

---

## 9. 能力边界

- 目前只有 Windows 版本，没有 macOS 版。
- 有安装包但没做代码签名，首次运行会被 SmartScreen 拦一下（点「更多信息 → 仍要运行」）；实时转写仍需自己装 Python 3.8+ 与 `pip install vosk`。
- 本地转写用 vosk 小模型，安静环境和清晰音频下效果更好；嘈杂环境会受影响。
- 腾讯会议、飞书窗口当前读不到可用字幕文本，"字幕文本通道"通常显示降级；实时转写请使用系统音频通道。
- AI 释义、AI 提醒依赖 DeepSeek 接口；联网查依赖博查或 Tavily；真实效果会受 Key、余额、网络和服务质量影响。
- 这是开发预览版，"产品级真实会议可用"尚未验收；请先在小范围、非关键会议中试用。
- 许可证为 MIT OR Apache-2.0；问题可以在 GitHub Issues 反馈：

https://github.com/AiLunSin/TSC/issues
