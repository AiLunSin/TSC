# 会议副驾 × DeepSeek Harness 本地插件桥

## 它是什么

桌面应用里已经有会中那几件事：选中查义、卡片、转写、联网检索、AI 提醒。DSH 插件不重写它们，只把 DSH 的工具调用翻成一个本机 HTTP 请求，交给应用去做。

所以转写、卡片库、凭据、限额仍然只有一份，插件也不落盘会议内容。出网路径同样没变：查义、检索、提醒发不发出去、发给谁，还是看应用里的配置，插件自己没有新的口子。

## 结构

```
app/src-tauri/src/plugin_bridge/
  mod.rs    启停、描述符、真实能力（直接调 ipc 层的 *_impl）
  http.rs   回环 HTTP 解析、鉴权、路由（API 可替换，便于单测）
plugins/meeting-copilot-dsh/
  index.js   宿主侧：六个工具、开关状态、/meeting-copilot/tools 路由
  client.js  浏览器侧：左侧栏底部的开关
```

插件不依赖 DSH 的宿主包，也不带 Rust 二进制；Rust 侧只多了 `plugin_bridge` 一个模块。

## 启动与发现

1. 应用启动完成时（`setup` 末尾）在 `127.0.0.1:0` 上开一个临时端口。
2. 生成 32 字节随机令牌，把端口、令牌、pid、协议版本、应用版本写进
   `%APPDATA%\com.meetingcopilot.desktop\plugin-bridge.json`（先写临时文件再替换）。
   环境变量 `MEETING_COPILOT_BRIDGE_FILE` 可以把这份文件指到别处（插件读同一个变量），
   无窗口验收和便携运行靠它。
3. 插件每次调用前重读该文件，所以应用重启、端口和令牌变了也不用重装插件。
4. 退出时（`RunEvent::Exit`）只删本进程写的那份，别的实例写的不动。

描述符示例：

```json
{
  "protocol": 1,
  "port": 51234,
  "token": "…64 位十六进制…",
  "pid": 12345,
  "appVersion": "0.1.3",
  "startedAtMs": 1760000000000
}
```

## 左侧开关

插件的浏览器半侧在 `sidebar.footer.action` 注册一个开关，位置在左侧栏底部、Settings 上方。展开时带「会议副驾」文字，收成 56px 轨道时只剩开关本身。

- 关闭时宿主注销六个工具，模型看不到；打开时重新注册。
- 状态写在 `~/.dsh/meeting-copilot/tools.json`（`{ "enabled": boolean }`），DSH 重启后保持。
- 开关只调宿主插件的 `/meeting-copilot/tools`：`GET` 读状态，`POST { enabled }` 改状态。请求要带 `x-meeting-copilot: 1`，跨站表单伪造不出来，跨源 `fetch` 会先倒在预检上。
- 工具关掉不影响开关本身。浏览器半侧独立于工具注册，随时能点开。

## HTTP 约定

- 只走 `HTTP/1.1`，一个连接一个请求，`Connection: close`
- `Host` 只认 `127.0.0.1:<port>` 和 `localhost:<port>`
- 每个请求都要带 `Authorization: Bearer <token>`
- 带 `Origin` 头的一律 403，挡的是浏览器和 DNS 重绑定
- 请求体上限 64 KiB，请求头 16 KiB，读超时 5 s，写超时 65 s
- 出错统一返回 `{ code, message, retryable }`，跟桌面 IPC 的 `IpcErrorDto` 同构

### 端点

| 方法 | 路径 | 请求 | 说明 |
|---|---|---|---|
| GET | `/v1/status` | — | 协议、版本、pid、转写状态、卡片数、隐私态 |
| POST | `/v1/explain` | `{ text, context? }` | 选中查义。没配模型 Key 时本机回退，`source` 为 `local` |
| GET | `/v1/cards?limit=` | — | 卡片快照，默认 30，最多 100 |
| POST | `/v1/cards/detail` | `{ cardId }` | 卡片详情，跟桌面端点开一张卡的行为一致 |
| POST | `/v1/cards/save` | `{ word, explanation, sources? }` | 存一张卡片，返回 `{ added }` |
| GET | `/v1/transcript?limit=` | — | 转写尾部，默认 50，最多 200 |
| POST | `/v1/search` | `{ query }` | 博查 / Tavily，受应用的小时限额管着 |
| POST | `/v1/assist` | `{ focus? }` | 提醒与回应，要配模型 Key |
| GET | `/v1/asr/status` | — | 是否在转写、最近错误、是否自动出卡 |
| POST | `/v1/asr/start` / `/v1/asr/stop` | — | 开始 / 停止转写 |

除 `plugin_bridge` 之外，DSH 宿主插件自己还挂了一个只给左栏开关用的路由：

| 方法 | 路径 | 请求 | 说明 |
|---|---|---|---|
| GET / POST | `/meeting-copilot/tools` | `{ enabled }`（POST） | 读 / 切换六个工具是否注册；要带 `x-meeting-copilot: 1` |

协议版本在 `plugin_bridge::PROTOCOL_VERSION` 和插件 `PROTOCOL` 两处，必须同步。不兼容的改动要递增，插件读到对不上的版本会拒绝并提示升级。

## DSH 工具映射

| 工具 | 端点 |
|---|---|
| `meeting_lookup` | `/v1/explain` |
| `meeting_cards`（list / detail） | `/v1/cards`、`/v1/cards/detail` |
| `meeting_save_card` | `/v1/cards/save` |
| `meeting_transcript`（read / start / stop / status） | `/v1/transcript`、`/v1/asr/*` |
| `meeting_web_search` | `/v1/search` |
| `meeting_assist` | `/v1/assist` |

输出只留模型要用的字段：卡片压成 `{id,title,conclusion,confidence,clickable,badgeCount}`，详情保留引用和对齐状态，转写按时间正序。

## 无窗口模式

`MEETING_COPILOT_HEADLESS=1` 时清掉窗口配置，只留运行态和插件桥，自动化验收或后台常驻用。正常启动还是原来的单窗口应用。

## 验证

- Rust 单测 21 项，`plugin_bridge` 覆盖鉴权、Host/Origin 拒绝、路由、方法、请求体上限、错误码、Content-Length 和描述符读写。
- 插件冒烟用假桥跑六个工具，查请求路径、输入输出映射和错误文案，共 14 条断言。
- 真机（无窗口启动，描述符指到工作区）：`/v1/status`、`/v1/cards`、`/v1/transcript`、`/v1/asr/status` 都拿到桌面端的真实运行态；不带令牌和带错令牌都是 401。
- 装进 DSH 实测：六个工具出现在当前会话；`meeting_cards`（list / detail）读到本机真实卡片，`meeting_transcript`（status / read）给出空转写提示；把应用停掉后 `meeting_lookup` 报「会议副驾没有在运行」。
- 左栏开关：`GET /meeting-copilot/tools` 返回 `{enabled:true}`，不带自定义头是 403，请求体写错是 400，方法不对是 405；`POST {enabled:false}` 后状态落盘、`meeting_cards` 变成 unknown tool，`POST {enabled:true}` 后六个工具回来。浏览器半侧只在模拟加载器里验过注册参数，真实渲染没看过——环境里没有浏览器控制，这条算未验证。
- 没覆盖：`meeting_lookup`、`meeting_web_search`、`meeting_assist` 的成功路径没在真实网络上跑（会烧用户自己的模型 Key 和检索额度），由 Rust 方法和假桥测试兜着。

## 已知限制

- 插件不负责启动应用。应用没跑就是「未连接」。
- 两个应用实例会互相覆盖描述符，后启动的赢。先启动实例的桥还能直连，但插件只认最新那份。
- `/v1/cards/detail` 只对当前快照里的草案卡开放，跟桌面端点开卡片的行为一样。
- 描述符靠用户配置目录的默认 ACL 保护。同一用户下的别的进程能读到令牌，这跟项目一直以来的本机单用户前提一致。
- 用沙箱里的自动化进程拉起应用时，应用可能没权限写 `%APPDATA%`；把 `MEETING_COPILOT_BRIDGE_FILE` 指到可写目录就能验，正常启动不受影响。
