/**
 * 会议副驾 DSH 插件的浏览器半侧：左侧栏底部的开关。
 *
 * 开关不直接操作工具，只调用宿主插件的 `/meeting-copilot/tools` 路由；
 * 宿主根据状态注册或注销六个工具，状态写在 DSH 主目录里。
 */

window.__ModuleLoader__.load({
  id: 'meeting-copilot-dsh',
  factory(require) {
    const React = require('react')
    const h = React.createElement

    const TOGGLE_URL = '/meeting-copilot/tools'
    const TOGGLE_HEADER = 'x-meeting-copilot'

    /** 主进程只接受带自定义头的同源请求；跨源请求会在预检处失败。 */
    async function request(method, body) {
      const response = await fetch(TOGGLE_URL, {
        method,
        cache: 'no-store',
        headers: {
          [TOGGLE_HEADER]: '1',
          ...body === undefined ? {} : { 'content-type': 'application/json' },
        },
        body: body === undefined ? undefined : JSON.stringify(body),
      })
      if (!response.ok) throw new Error(`toggle request failed: ${response.status}`)
      return await response.json()
    }

    const styles = `
.mc-dsh-toggle{display:flex;align-items:center;gap:8px;box-sizing:border-box;width:100%;height:32px;margin:0;padding:0 8px;border:0;border-radius:8px;background:transparent;color:var(--dsw-alias-label-secondary,#667085);font:inherit;font-size:13px;line-height:1;cursor:pointer;text-align:left}
.mc-dsh-toggle:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.12));color:var(--dsw-alias-label-primary,#101828)}
.mc-dsh-toggle:focus-visible{outline:2px solid var(--dsw-focus-ring-color,#4d6bfe);outline-offset:1px}
.mc-dsh-toggle[data-wide="false"]{justify-content:center;width:36px;height:36px;padding:0}
.mc-dsh-toggle[data-state="error"]{color:var(--dsw-alias-state-error-primary,#d92d20)}
.mc-dsh-track{position:relative;flex:none;width:28px;height:16px;border-radius:999px;background:var(--dsw-alias-border-l3,#c9ccd4);transition:background .15s ease}
.mc-dsh-toggle[aria-checked="true"] .mc-dsh-track{background:var(--dsw-alias-brand-primary,#4d6bfe)}
.mc-dsh-toggle[data-state="loading"] .mc-dsh-track{opacity:.55}
.mc-dsh-knob{position:absolute;top:2px;left:2px;width:12px;height:12px;border-radius:50%;background:#fff;box-shadow:0 1px 2px rgba(0,0,0,.25);transition:transform .15s ease}
.mc-dsh-toggle[aria-checked="true"] .mc-dsh-knob{transform:translateX(12px)}
.mc-dsh-label{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
`

    /** 左侧栏底部、Settings 上方的开关行。 */
    function MeetingCopilotToggle(props) {
      const wide = props.wide !== false
      const [enabled, setEnabled] = React.useState(true)
      const [state, setState] = React.useState('loading')
      const [busy, setBusy] = React.useState(false)
      const alive = React.useRef(true)

      const load = React.useCallback(() => {
        request('GET').then(value => {
          if (!alive.current) return
          setEnabled(value.enabled !== false)
          setState('ready')
        }).catch(() => {
          if (alive.current) setState('error')
        })
      }, [])

      React.useEffect(() => {
        alive.current = true
        load()
        const onFocus = () => { load() }
        window.addEventListener('focus', onFocus)
        return () => {
          alive.current = false
          window.removeEventListener('focus', onFocus)
        }
      }, [load])

      const toggle = React.useCallback(async () => {
        if (busy) return
        if (state === 'error') {
          setState('loading')
          load()
          return
        }
        setBusy(true)
        try {
          const value = await request('POST', { enabled: !enabled })
          if (!alive.current) return
          setEnabled(value.enabled !== false)
          setState('ready')
        } catch {
          if (alive.current) setState('error')
        } finally {
          if (alive.current) setBusy(false)
        }
      }, [busy, enabled, load, state])

      const title = state === 'error'
        ? '会议副驾工具：连不上 DSH 宿主，点击重试'
        : state === 'loading'
          ? '会议副驾工具：读取中'
          : enabled
            ? '会议副驾工具：已开启，点击关闭'
            : '会议副驾工具：已关闭，点击开启'

      return h(React.Fragment, null,
        h('style', { key: 'styles' }, styles),
        h('button', {
          key: 'switch',
          type: 'button',
          role: 'switch',
          'aria-checked': enabled ? 'true' : 'false',
          'aria-label': '会议副驾工具',
          'data-wide': wide ? 'true' : 'false',
          'data-state': state,
          title,
          disabled: busy,
          className: 'mc-dsh-toggle',
          onClick: toggle,
        },
          h('span', { key: 'track', className: 'mc-dsh-track' },
            h('span', { key: 'knob', className: 'mc-dsh-knob' })),
          wide ? h('span', { key: 'label', className: 'mc-dsh-label' }, '会议副驾') : null,
        ),
      )
    }

    return {
      inject: ['slots'],
      apply(ctx) {
        ctx.slots.inject('sidebar.footer.action', () => ctx.slots.register({
          name: 'sidebar.footer.action',
          id: 'meeting-copilot-toggle',
          order: 10,
        }, MeetingCopilotToggle))
      },
    }
  },
})
