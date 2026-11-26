/*
 * README 截图用的假 IPC。
 *
 * 前端只用 window.__TAURI__.core.invoke 这一个入口（见 app/src/bridge/tauri.ts），
 * 所以在浏览器里把这个对象顶掉，真实界面就能渲染出有内容的工作页。
 *
 * 注意：页面的 CSP 是 script-src 'self'，必须作为独立文件加载，
 * 写成内联 <script> 会被浏览器拦掉，界面只剩空状态。
 *
 * 这里的会议内容全是编的示例，不含任何真实转写。
 *
 * URL 带 ?demo=1 时进入演示模式：初始只有两张卡，查完意思再生成第三张，
 * 配合 demo.js 录 README 的动图；不带参数时是静态截图用的三张卡。
 */
(function () {
  var now = Date.now();
  var DEMO = new URLSearchParams(location.search).has("demo");
  var cards = [
    {
      cardId: "c1",
      keywordKind: "proper_noun",
      trigger: "SLA",
      triggeredAtMs: now - 62_000,
      cardStatus: "shown",
      bodyVisible: true,
      body: {
        title: "SLA 99.9%",
        conclusion: { available: { text: "服务可用性承诺，按月统计，未达标按合同约定赔付。" } },
        confidence: 0.9,
        referenceIds: ["utt-14"]
      },
      clickState: "enabled",
      disabledReason: null,
      badgeCount: 0
    },
    {
      cardId: "c2",
      keywordKind: "metric",
      trigger: "灰度范围",
      triggeredAtMs: now - 38_000,
      cardStatus: "shown",
      bodyVisible: true,
      body: {
        title: "灰度范围",
        conclusion: { available: { text: "这次先放量的站点，会上提到华东两个站点，第二批未定。" } },
        confidence: 0.82,
        referenceIds: ["utt-11", "utt-19"]
      },
      clickState: "enabled",
      disabledReason: null,
      badgeCount: 0
    },
    {
      cardId: "c3",
      keywordKind: "acronym",
      trigger: "回滚预案",
      triggeredAtMs: now - 15_000,
      cardStatus: "shown",
      bodyVisible: true,
      body: {
        title: "回滚预案",
        conclusion: { available: { text: "上线出问题时的回退步骤与责任人清单，承诺下周三前给出。" } },
        confidence: 0.76,
        referenceIds: ["utt-21"]
      },
      clickState: "enabled",
      disabledReason: null,
      badgeCount: 0
    }
  ];

  var lines = [
    { index: 1, text: "先对一下今天的三件事：灰度节奏、SLA 口径、回滚预案。", atMs: now - 96_000 },
    { index: 2, text: "这版灰度我们分两批放，先看华东两个站点。", atMs: now - 84_000 },
    { index: 3, text: "华东京东机房上周抖动过一次，运维那边加了告警。", atMs: now - 71_000 },
    { index: 4, text: "SLA 还是按 99.9% 谈，月度统计算，赔付条款法务看过。", atMs: now - 58_000 },
    { index: 5, text: "回滚预案下周三前给出来，责任人我这边先挂着。", atMs: now - 44_000 },
    { index: 6, text: "还有一个小问题，客户侧的网络策略要提前打招呼。", atMs: now - 29_000 },
    { index: 7, text: "灰度范围我这边再确认一下站点，晚点发群里。", atMs: now - 16_000 }
  ];

  var explained = false;

  var handlers = {
    keyword_cards_snapshot: function () {
      if (!DEMO) {
        return cards;
      }
      // 演示：SLA 这张是"查完意思再生成"的，初始不该出现。
      return explained ? cards : cards.slice(1);
    },
    explain_selection: function (args) {
      return {
        text: (args && args.input && args.input.text) || "SLA 99.9%",
        explanation: "服务可用性承诺：按月统计，未达标按合同约定赔付。这句是客户方口径，和上季度的 99.95% 不是同一个数。",
        source: "local"
      };
    },
    ingest_explanation: function () {
      explained = true;
      return 1;
    },
    transcript_snapshot: function () {
      return lines;
    },
    search_config: function () {
      return {
        provider: "bocha",
        hasKey: true,
        hasKeyBocha: true,
        hasKeyTavily: false,
        auto: false,
        ready: true,
        hourlyLimit: 20,
        usedThisHour: 3
      };
    },
    // 截图里用常显，悬停模式在静态图上只剩几个圆点，看不出是什么。
    ui_prefs: function () {
      return { labelMode: "always" };
    },
    meeting_status: function () {
      return ["腾讯会议", "产品双周评审"];
    },
    capture_status: function () {
      return [
        ["字幕文本通道", "degraded"],
        ["系统音频通道", "ready"],
        ["本地 ASR", "ready"]
      ];
    },
    asr_status: function () {
      return [true, "灰度范围我这边再确认一下", "", "音频 8.2s（816 片）/ 写入 255KB / 稳定句 7", false];
    },
    quality_metrics: function () {
      return {
        autoShown: 3,
        feedback: 1,
        useful: 1,
        useless: 0,
        wrong: 0,
        misTriggerRate: 0,
        adoptRate: 1,
        latencySamples: 3,
        latencyP50Ms: 820,
        latencyP95Ms: 1240,
        latencyP99Ms: 1240
      };
    }
  };

  window.__TAURI__ = {
    core: {
      invoke: function (command) {
        var handler = handlers[command];
        return Promise.resolve(handler ? handler() : null);
      }
    }
  };
})();
