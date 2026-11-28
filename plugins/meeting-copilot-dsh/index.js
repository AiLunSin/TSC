/**
 * 会议副驾（Meeting Copilot）的 DSH 工具。
 *
 * 插件自己不转写也不存卡，只把调用转给桌面应用里的插件桥
 * （127.0.0.1，令牌每次启动换一个）。应用没在运行时，工具会直接说没连上。
 */

import fs from 'node:fs'
import http from 'node:http'
import os from 'node:os'
import path from 'node:path'

/** 与 Rust 侧 `PROTOCOL_VERSION` 保持一致。 */
const PROTOCOL = 1
/** Tauri 的应用标识，决定配置目录名。 */
const APP_DIR = 'com.meetingcopilot.desktop'
/** 描述符文件名，与 Rust 侧 `DESCRIPTOR_FILE` 一致。 */
const DESCRIPTOR_FILE = 'plugin-bridge.json'
/** 左侧栏开关使用的宿主路由。 */
const TOGGLE_PATH = '/meeting-copilot/tools'
/** 只有带这个请求头的调用才算数：跨站表单无法伪造，跨源 fetch 会先被预检挡住。 */
const TOGGLE_HEADER = 'x-meeting-copilot'

/** 插件自己抛出的、可以直接展示给用户的错误。 */
class BridgeError extends Error {}

function descriptorPath() {
  const override = process.env.MEETING_COPILOT_BRIDGE_FILE
  if (override) return override
  const appData = process.env.APPDATA
  if (!appData) {
    throw new BridgeError('找不到 APPDATA，无法定位会议副驾的插件桥；可用 MEETING_COPILOT_BRIDGE_FILE 指定描述符路径。')
  }
  return path.join(appData, APP_DIR, DESCRIPTOR_FILE)
}

function readDescriptor() {
  const file = descriptorPath()
  let raw
  try {
    raw = fs.readFileSync(file, 'utf8')
  } catch {
    throw new BridgeError('会议副驾没有在运行，或插件桥还没启动。请先启动 Meeting Copilot 桌面应用。')
  }
  let descriptor
  try {
    descriptor = JSON.parse(raw)
  } catch {
    throw new BridgeError('会议副驾的插件桥描述符损坏；重启会议副驾会重新生成。')
  }
  if (descriptor.protocol !== PROTOCOL) {
    throw new BridgeError(`会议副驾的插件桥协议是 ${descriptor.protocol}，本插件只支持 ${PROTOCOL}；请升级插件或应用。`)
  }
  if (!Number.isInteger(descriptor.port) || descriptor.port <= 0 || typeof descriptor.token !== 'string' || descriptor.token.length === 0) {
    throw new BridgeError('会议副驾的插件桥描述符缺少端口或令牌；重启会议副驾再试。')
  }
  return descriptor
}

/** 调一次插件桥；非 2xx 一律转成带应用错误码的可读错误。 */
async function call(method, target, body, timeoutMs = 15_000) {
  const descriptor = readDescriptor()
  const payload = body === undefined ? null : Buffer.from(JSON.stringify(body), 'utf8')
  const headers = {
    Authorization: `Bearer ${descriptor.token}`,
  }
  if (payload !== null) {
    headers['Content-Type'] = 'application/json'
    headers['Content-Length'] = String(payload.length)
  }
  return await new Promise((resolve, reject) => {
    const request = http.request({
      host: '127.0.0.1',
      port: descriptor.port,
      path: target,
      method,
      headers,
      timeout: timeoutMs,
    }, response => {
      const chunks = []
      response.on('data', chunk => { chunks.push(chunk) })
      response.on('end', () => {
        const text = Buffer.concat(chunks).toString('utf8')
        let parsed
        try {
          parsed = text.length === 0 ? {} : JSON.parse(text)
        } catch {
          reject(new BridgeError('会议副驾的插件桥返回了无法解析的响应。'))
          return
        }
        if (response.statusCode >= 200 && response.statusCode < 300) {
          resolve(parsed)
          return
        }
        const message = typeof parsed.message === 'string' && parsed.message.length > 0
          ? parsed.message
          : `会议副驾拒绝了这次调用（HTTP ${response.statusCode}）。`
        reject(new BridgeError(message))
      })
    })
    request.on('timeout', () => {
      request.destroy(new BridgeError('会议副驾的插件桥响应超时，请稍后重试。'))
    })
    request.on('error', error => {
      if (error instanceof BridgeError) {
        reject(error)
        return
      }
      if (error.code === 'ECONNREFUSED') {
        reject(new BridgeError('连不上会议副驾的本地插件桥（应用可能已经退出）。请重新启动 Meeting Copilot。'))
        return
      }
      reject(new BridgeError(`调用会议副驾失败：${error.message}`))
    })
    if (payload !== null) request.write(payload)
    request.end()
  })
}

function clampInt(value, fallback, min, max) {
  const parsed = Number.parseInt(String(value ?? ''), 10)
  if (!Number.isInteger(parsed)) return fallback
  return Math.min(max, Math.max(min, parsed))
}

/** 把 `ConclusionDto` 的几种形态压成一行文字。 */
function conclusionText(conclusion) {
  if (conclusion === undefined || conclusion === null) return ''
  if (typeof conclusion === 'string') return ''
  if (typeof conclusion.available === 'object' && conclusion.available !== null) {
    return typeof conclusion.available.text === 'string' ? conclusion.available.text : ''
  }
  return ''
}

function mapCard(view) {
  const body = view.body ?? {}
  return {
    id: String(view.cardId ?? ''),
    title: String(body.title ?? view.trigger ?? ''),
    conclusion: conclusionText(body.conclusion),
    confidence: typeof body.confidence === 'number' ? body.confidence : 0,
    clickable: view.clickState === 'enabled',
    badgeCount: Number.isInteger(view.badgeCount) ? view.badgeCount : 0,
  }
}

function mapDetail(detail) {
  return {
    id: String(detail.cardId ?? ''),
    title: String(detail.title ?? ''),
    conclusion: conclusionText(detail.conclusion),
    references: Array.isArray(detail.referenceIds) ? detail.referenceIds.map(String) : [],
    citationAlignment: String(detail.citationAlignment ?? ''),
  }
}

const cardSchema = {
  type: 'object',
  additionalProperties: false,
  properties: {
    id: { type: 'string' },
    title: { type: 'string' },
    conclusion: { type: 'string' },
    confidence: { type: 'number' },
    clickable: { type: 'boolean' },
    badgeCount: { type: 'integer' },
  },
  required: ['id', 'title', 'conclusion', 'confidence', 'clickable', 'badgeCount'],
}

const lookup = {
  name: 'meeting_lookup',
  description: '解释会议里选中的词、缩写或整段话。优先用会议副驾已配置的模型；没配 Key 时只给本机上下文，并在结果里标出来源。只外发选中的文字和一小段上下文。需要 Meeting Copilot 桌面应用在运行。',
  parameters: {
    text: { type: 'string', required: true, description: '要解释的词或短语，最多 200 字；选中的是整段话时会自动改概括。' },
    context: { type: 'string', description: '可选：选中处前后各一句。省略时由会议副驾从本次转写里取。' },
  },
  output: {
    schema: {
      type: 'object',
      additionalProperties: false,
      properties: {
        text: { type: 'string', description: '被解释的原文。' },
        explanation: { type: 'string' },
        source: { type: 'string', enum: ['llm', 'local'], description: '答案来源：模型或本机上下文。' },
      },
      required: ['text', 'explanation', 'source'],
    },
    render: (_args, value) => [{
      type: 'text',
      text: `${value.explanation}\n\n来源：${value.source === 'llm' ? '模型' : '本机上下文'}`,
    }],
  },
  async execute(args) {
    const text = String(args.text ?? '').trim()
    if (text.length === 0) throw new Error('text 不能为空。')
    const result = await call('POST', '/v1/explain', {
      text,
      context: String(args.context ?? ''),
    }, 25_000)
    return {
      text: String(result.text ?? text),
      explanation: String(result.explanation ?? ''),
      source: result.source === 'llm' ? 'llm' : 'local',
    }
  },
}

const cards = {
  name: 'meeting_cards',
  description: '查看会议副驾本机加密库里的卡片：列出最近卡片，或按 id 打开一张的详情。数据只在会议副驾里，不联网。需要 Meeting Copilot 桌面应用在运行。',
  parameters: {
    action: { type: 'string', required: true, enum: ['list', 'detail'], description: 'list 列最近卡片；detail 按 cardId 打开详情。' },
    cardId: { type: 'string', description: 'action=detail 时必填。' },
    limit: { type: 'integer', description: 'action=list 时返回张数，默认 20，最多 100。' },
  },
  output: {
    schema: {
      type: 'object',
      additionalProperties: false,
      properties: {
        action: { type: 'string', enum: ['list', 'detail'] },
        total: { type: 'integer', description: '本机存着的卡片总数（仅 list）。' },
        cards: { type: 'array', description: '卡片列表（仅 list）。', items: cardSchema },
        card: {
          type: 'object',
          description: '单张卡片详情（仅 detail）。',
          additionalProperties: false,
          properties: {
            id: { type: 'string' },
            title: { type: 'string' },
            conclusion: { type: 'string' },
            references: { type: 'array', items: { type: 'string' } },
            citationAlignment: { type: 'string' },
          },
          required: ['id', 'title', 'conclusion', 'references', 'citationAlignment'],
        },
      },
      required: ['action'],
    },
    render: (_args, value) => {
      if (value.action === 'detail' && value.card) {
        const card = value.card
        const references = card.references.length === 0 ? '无' : card.references.length
        return [{ type: 'text', text: `${card.title}\n${card.conclusion}\n引用：${references}` }]
      }
      const list = value.cards ?? []
      if (list.length === 0) {
        return [{ type: 'text', text: '本机还没有卡片。选中文字查意思后可以存成卡片。' }]
      }
      const lines = list.map(card => `- [${card.id}] ${card.title}${card.conclusion ? ` — ${card.conclusion}` : ''}`)
      return [{ type: 'text', text: `最近 ${list.length} 张（本机共 ${value.total ?? list.length} 张）：\n${lines.join('\n')}` }]
    },
  },
  async execute(args) {
    if (args.action === 'list') {
      const limit = clampInt(args.limit, 20, 1, 100)
      const result = await call('GET', `/v1/cards?limit=${limit}`)
      const list = Array.isArray(result.cards) ? result.cards : []
      return { action: 'list', total: Number.isInteger(result.count) ? result.count : list.length, cards: list.map(mapCard) }
    }
    if (args.action === 'detail') {
      const cardId = String(args.cardId ?? '').trim()
      if (cardId.length === 0) throw new Error('action=detail 时必须给 cardId。')
      const detail = await call('POST', '/v1/cards/detail', { cardId })
      return { action: 'detail', card: mapDetail(detail) }
    }
    throw new Error('action 只能是 list 或 detail。')
  },
}

const saveCard = {
  name: 'meeting_save_card',
  description: '把一条解释存成会议副驾的卡片，进入本机加密卡片库（最近 200 张）。只写本机，不联网。需要 Meeting Copilot 桌面应用在运行。',
  parameters: {
    word: { type: 'string', required: true, description: '卡片的关键词或原句。' },
    explanation: { type: 'string', required: true, description: '卡片结论，通常来自 meeting_lookup。' },
    sources: { type: 'array', description: '可选：来源站点名。', items: { type: 'string' } },
  },
  output: {
    schema: {
      type: 'object',
      additionalProperties: false,
      properties: {
        added: { type: 'integer', description: '新写入的卡片数；0 表示同一内容已存在。' },
      },
      required: ['added'],
    },
    render: (_args, value) => [{
      type: 'text',
      text: value.added > 0 ? `已存成 ${value.added} 张卡片。` : '这张卡片已经存在，没有重复写入。',
    }],
  },
  async execute(args) {
    const word = String(args.word ?? '').trim()
    const explanation = String(args.explanation ?? '').trim()
    if (word.length === 0) throw new Error('word 不能为空。')
    if (explanation.length === 0) throw new Error('explanation 不能为空。')
    const sources = Array.isArray(args.sources) ? args.sources.map(String) : []
    const result = await call('POST', '/v1/cards/save', { word, explanation, sources })
    return { added: Number.isInteger(result.added) ? result.added : 0 }
  },
}

const transcript = {
  name: 'meeting_transcript',
  description: '读会议副驾当前的实时转写，或开始/停止转写。转写只在会议副驾进程内存里，退出即清空。需要 Meeting Copilot 桌面应用在运行。',
  parameters: {
    action: { type: 'string', required: true, enum: ['read', 'start', 'stop', 'status'], description: 'read 读最近转写；start/stop 控制转写；status 看是否在转写和错误。' },
    limit: { type: 'integer', description: 'action=read 时读取的行数，默认 30，最多 200。' },
  },
  output: {
    schema: {
      type: 'object',
      additionalProperties: false,
      properties: {
        action: { type: 'string', enum: ['read', 'start', 'stop', 'status'] },
        total: { type: 'integer', description: '转写总行数（仅 read）。' },
        lines: {
          type: 'array',
          description: '按时间正序的转写（仅 read）。',
          items: {
            type: 'object',
            additionalProperties: false,
            properties: {
              index: { type: 'integer' },
              text: { type: 'string' },
              atMs: { type: 'integer' },
            },
            required: ['index', 'text', 'atMs'],
          },
        },
        running: { type: 'boolean' },
        displayText: { type: 'string' },
        lastError: { type: 'string' },
        autoCards: { type: 'boolean' },
      },
      required: ['action'],
    },
    render: (_args, value) => {
      if (value.action === 'read') {
        const lines = value.lines ?? []
        if (lines.length === 0) return [{ type: 'text', text: '转写还是空的：先开始转写，或确认会议副驾正在采集系统音频。' }]
        return [{ type: 'text', text: lines.map(line => line.text).join('\n') }]
      }
      if (value.action === 'status') {
        const state = value.running ? '正在转写' : '未在转写'
        const detail = value.lastError ? `；最近错误：${value.lastError}` : ''
        return [{ type: 'text', text: `${state}${detail}` }]
      }
      return [{ type: 'text', text: value.running ? '转写已开始。' : '转写已停止。' }]
    },
  },
  async execute(args) {
    if (args.action === 'read') {
      const limit = clampInt(args.limit, 30, 1, 200)
      const result = await call('GET', `/v1/transcript?limit=${limit}`)
      const lines = Array.isArray(result.lines) ? result.lines : []
      return { action: 'read', total: Number.isInteger(result.total) ? result.total : lines.length, lines }
    }
    if (args.action === 'status') {
      const result = await call('GET', '/v1/asr/status')
      return {
        action: 'status',
        running: Boolean(result.running),
        displayText: String(result.displayText ?? ''),
        lastError: String(result.lastError ?? ''),
        autoCards: Boolean(result.autoCards),
      }
    }
    if (args.action === 'start' || args.action === 'stop') {
      const result = await call('POST', `/v1/asr/${args.action}`)
      return { action: args.action, running: Boolean(result.running) }
    }
    throw new Error('action 只能是 read、start、stop 或 status。')
  },
}

const webSearch = {
  name: 'meeting_web_search',
  description: '用会议副驾配置的检索服务（博查或 Tavily）查一个词，只发 80 字以内；受每小时限额约束。用于给卡片补引用来源。需要 Meeting Copilot 桌面应用在运行。',
  parameters: {
    query: { type: 'string', required: true, description: '检索词，最多 80 字。' },
  },
  output: {
    schema: {
      type: 'object',
      additionalProperties: false,
      properties: {
        provider: { type: 'string' },
        hits: {
          type: 'array',
          items: {
            type: 'object',
            additionalProperties: false,
            properties: {
              title: { type: 'string' },
              site: { type: 'string' },
              snippet: { type: 'string' },
              url: { type: 'string' },
            },
            required: ['title', 'site', 'snippet', 'url'],
          },
        },
      },
      required: ['provider', 'hits'],
    },
    render: (_args, value) => {
      if (value.hits.length === 0) return [{ type: 'text', text: `没有查到结果（${value.provider}）。` }]
      const lines = value.hits.map(hit => `- ${hit.title}（${hit.site}）${hit.url ? `\n  ${hit.url}` : ''}`)
      return [{ type: 'text', text: `${value.provider} 返回 ${value.hits.length} 条：\n${lines.join('\n')}` }]
    },
  },
  async execute(args) {
    const query = String(args.query ?? '').trim()
    if (query.length === 0) throw new Error('query 不能为空。')
    const result = await call('POST', '/v1/search', { query }, 15_000)
    const hits = Array.isArray(result.hits) ? result.hits : []
    return {
      provider: String(result.provider ?? ''),
      hits: hits.map(hit => ({
        title: String(hit.title ?? ''),
        site: String(hit.site ?? ''),
        snippet: String(hit.snippet ?? ''),
        url: String(hit.url ?? ''),
      })),
    }
  },
}

const assist = {
  name: 'meeting_assist',
  description: '看会议最近 12 句（或选中一句），给出两条要留意的提醒和一两句能直接说出口的话。需要会议副驾已配置模型 Key，并且桌面应用在运行。',
  parameters: {
    focus: { type: 'string', description: '可选：刚选中或正在想说的一句；省略时只看最近转写。' },
  },
  output: {
    schema: {
      type: 'object',
      additionalProperties: false,
      properties: {
        source: { type: 'string' },
        points: { type: 'array', items: { type: 'string' } },
        replies: { type: 'array', items: { type: 'string' } },
        note: { type: 'string' },
      },
      required: ['source', 'points', 'replies', 'note'],
    },
    render: (_args, value) => {
      const parts = []
      if (value.points.length > 0) parts.push(`提醒：\n${value.points.map(point => `- ${point}`).join('\n')}`)
      if (value.replies.length > 0) parts.push(`可以这样说：\n${value.replies.map(reply => `- ${reply}`).join('\n')}`)
      if (value.note.length > 0) parts.push(value.note)
      return [{ type: 'text', text: parts.length === 0 ? '这次没有给出提醒。' : parts.join('\n\n') }]
    },
  },
  async execute(args) {
    const focus = String(args.focus ?? '').trim()
    const result = await call('POST', '/v1/assist', { focus }, 25_000)
    return {
      source: String(result.source ?? ''),
      points: Array.isArray(result.points) ? result.points.map(String) : [],
      replies: Array.isArray(result.replies) ? result.replies.map(String) : [],
      note: String(result.note ?? ''),
    }
  },
}

/** 开关状态文件：放在 DSH 主目录，重启后仍保持用户的选择。 */
function toggleStatePath() {
  const home = process.env.DSH_HOME
  const base = home !== undefined && home.trim().length > 0 ? home : path.join(os.homedir(), '.dsh')
  return path.join(base, 'meeting-copilot', 'tools.json')
}

function readEnabled() {
  try {
    const parsed = JSON.parse(fs.readFileSync(toggleStatePath(), 'utf8'))
    if (typeof parsed.enabled === 'boolean') return parsed.enabled
  } catch {
    // 没有状态文件、或文件损坏：默认开启，和安装后的初始状态一致。
  }
  return true
}

function writeEnabled(enabled) {
  const file = toggleStatePath()
  try {
    fs.mkdirSync(path.dirname(file), { recursive: true })
    fs.writeFileSync(file, `${JSON.stringify({ enabled }, null, 2)}\n`, 'utf8')
  } catch {
    // 写不进去时，开关在本次 DSH 运行内仍然生效。
  }
}

function respondJson(response, status, payload, headOnly = false) {
  response.statusCode = status
  response.setHeader('content-type', 'application/json; charset=utf-8')
  response.setHeader('cache-control', 'no-store')
  response.end(headOnly ? undefined : JSON.stringify(payload))
}

/** 左栏开关的宿主路由：GET 读状态，POST 改状态。 */
function installToggleRoute(ctx, getEnabled, setEnabled) {
  return ctx.webServer.register({
    kind: 'exact',
    path: TOGGLE_PATH,
    handler(request, response) {
      if (request.headers[TOGGLE_HEADER] !== '1') {
        respondJson(response, 403, { error: 'forbidden' })
        return
      }
      if (request.method === 'GET' || request.method === 'HEAD') {
        respondJson(response, 200, { enabled: getEnabled() }, request.method === 'HEAD')
        return
      }
      if (request.method !== 'POST') {
        response.setHeader('allow', 'GET, HEAD, POST')
        respondJson(response, 405, { error: 'method_not_allowed' })
        return
      }
      const chunks = []
      let size = 0
      request.on('data', chunk => {
        size += chunk.length
        if (size > 2048) {
          respondJson(response, 413, { error: 'body_too_large' })
          request.destroy()
          return
        }
        chunks.push(chunk)
      })
      request.on('end', () => {
        if (size > 2048) return
        let next
        try {
          const body = chunks.length === 0 ? {} : JSON.parse(Buffer.concat(chunks).toString('utf8'))
          if (typeof body.enabled !== 'boolean') throw new Error('enabled must be boolean')
          next = body.enabled
        } catch {
          respondJson(response, 400, { error: 'invalid_body' })
          return
        }
        setEnabled(next)
        respondJson(response, 200, { enabled: getEnabled() })
      })
    },
  })
}

const TOOLS = [lookup, cards, saveCard, transcript, webSearch, assist]

export const inject = ['tools']

export function apply(ctx) {
  let enabled = readEnabled()
  let disposers = []
  const disposeTools = () => {
    const pending = disposers
    disposers = []
    for (const dispose of pending) {
      try {
        dispose()
      } catch {
        // 已经卸载的工具再释放一次不应影响其它工具。
      }
    }
  }
  const applyEnabled = () => {
    disposeTools()
    if (!enabled) return
    disposers = TOOLS.map(tool => ctx.tools.register({
      name: tool.name,
      description: tool.description,
      parameters: tool.parameters,
      output: tool.output,
      execute: tool.execute,
    }))
  }
  applyEnabled()
  ctx.effect(() => () => { disposeTools() }, 'meeting-copilot: tool registration')
  ctx.inject(['webServer'], webCtx => {
    webCtx.effect(
      () => installToggleRoute(
        webCtx,
        () => enabled,
        next => {
          enabled = next
          writeEnabled(next)
          applyEnabled()
        },
      ),
      'meeting-copilot: tools toggle route',
    )
  })
}
