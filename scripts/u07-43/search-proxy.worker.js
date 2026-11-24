// U-07-43 采样用的 HTTPS 检索适配端点（Cloudflare Workers）。
//
// 背景：`infrastructure::search::http_json` 只发 GET `?q=<query>&format=json`，
// 且要求响应形如 `{"results":[{"url":"https://...","title":"..."}]}`；
// 博查 / Tavily 都是 POST + 自有响应形状，无法直接接进 `search_live.rs`。
// 本 Worker 把两侧对上，让已有的检索 Key 能用于 200 条真实样本采集。
//
// 一键部署（推荐）：
//   UPSTREAM_KEY='<检索服务 Key>' bash scripts/u07-43/deploy-worker.sh --provider bocha
// 脚本会注入三个 secret、执行 wrangler deploy，并写好 .tmp/u07-43/config.env。
//
// 手工部署：wrangler deploy，然后依次
//   wrangler secret put UPSTREAM_KEY       # 博查/Tavily 的 Key
//   wrangler secret put SHARED_TOKEN       # 自定义随机串，采样脚本当 Bearer 用
//   wrangler secret put UPSTREAM_PROVIDER  # bocha 或 tavily
//
// 不要把任何 Key 写进本文件、providers.json 或提交信息。
// 查询原文只在请求内存中使用，不写日志；失败时只回状态码与错误标签。

const UPSTREAM = {
  bocha: {
    host: "api.bochaai.com",
    path: "/v1/web-search",
    buildBody: (q, limit) => ({ query: q, count: limit, summary: false }),
    extract: (data) => data?.data?.webPages?.value ?? [],
  },
  tavily: {
    host: "api.tavily.com",
    path: "/search",
    buildBody: (q, limit) => ({
      query: q,
      max_results: limit,
      search_depth: "basic",
    }),
    extract: (data) => data?.results ?? [],
  },
};

const MAX_RESULTS = 5;
const MAX_QUERY_CHARS = 200; // 与 domain::vo::SanitizedQuery 上限一致
const MAX_TITLE_CHARS = 40; // 与 domain::vo::TitleText::LIMIT 一致
// 采样端硬超时 8s；上游留 6.2s，给 TLS 与 JSON 解析留余量。
const UPSTREAM_TIMEOUT_MS = 6200;

function json(body, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "no-store",
    },
  });
}

function normalize(raw) {
  const url = typeof raw.url === "string" ? raw.url : "";
  // 适配器只接受 http(s) 绝对地址且不含空白，这里先过滤，
  // 避免整批结果被判 MalformedResponse。
  if (!/^https?:\/\//i.test(url) || /\s/.test(url)) return null;
  let host = "";
  try {
    host = new URL(url).hostname;
  } catch {
    return null;
  }
  if (!host) return null;
  const supplied =
    (typeof raw.title === "string" && raw.title.trim()) ||
    (typeof raw.name === "string" && raw.name.trim()) ||
    "";
  // TitleText::new 拒绝空串：上游偶尔不给标题，用主机名兜底，
  // 否则一条空标题就会让整个响应的 5 条结果全部失败。
  const title = (supplied || host).slice(0, MAX_TITLE_CHARS);
  const published = raw.publishedDate ?? raw.published_date ?? raw.dateLastCrawled ?? null;
  const hit = { url, title };
  if (published !== null && published !== undefined && published !== "") {
    hit.publishedDate = String(published).slice(0, 32);
  }
  return hit;
}

async function callUpstream(config, query, token) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), UPSTREAM_TIMEOUT_MS);
  try {
    const response = await fetch(`https://${config.host}${config.path}`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        accept: "application/json",
        authorization: `Bearer ${token}`,
      },
      body: JSON.stringify(config.buildBody(query, MAX_RESULTS)),
      signal: controller.signal,
    });
    if (!response.ok) return { status: response.status, hits: [] };
    const data = await response.json();
    const hits = config
      .extract(data)
      .map(normalize)
      .filter((hit) => hit !== null)
      .slice(0, MAX_RESULTS);
    return { status: 200, hits };
  } catch {
    return { status: 504, hits: [] };
  } finally {
    clearTimeout(timer);
  }
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // 部署自检：不消耗上游额度，也不回显任何凭据值。
    if (url.pathname === "/health") {
      return json({
        ok: Boolean(env.SHARED_TOKEN && env.UPSTREAM_KEY),
        provider: (env.UPSTREAM_PROVIDER ?? "bocha").toLowerCase(),
        authRequired: true,
      });
    }

    // 没配共享令牌就拒绝服务：否则任何人都能拿这个 Worker 消耗上游额度。
    const expected = env.SHARED_TOKEN;
    if (!expected) return json({ error: "proxy_not_configured" }, 503);
    const auth = request.headers.get("authorization") ?? "";
    if (auth !== `Bearer ${expected}`) {
      return json({ error: "unauthorized" }, 401);
    }
    if (!env.UPSTREAM_KEY) return json({ error: "upstream_key_missing" }, 500);

    const query = (url.searchParams.get("q") ?? "").trim();
    if (!query) return json({ error: "missing_query" }, 400);
    if (query.length > MAX_QUERY_CHARS) return json({ error: "query_too_long" }, 400);

    const provider = (env.UPSTREAM_PROVIDER ?? "bocha").toLowerCase();
    const config = UPSTREAM[provider];
    if (!config) return json({ error: "unknown_provider" }, 500);

    let result = await callUpstream(config, query, env.UPSTREAM_KEY);
    // 429 时等一小段重试一次；重试的耗时计入本次样本时延，是真实端到端开销。
    if (result.status === 429) {
      await new Promise((resolve) => setTimeout(resolve, 800));
      result = await callUpstream(config, query, env.UPSTREAM_KEY);
    }
    if (result.status !== 200) return json({ error: "upstream_failed" }, 502);
    // 上游没给可用结果就判失败，不用空列表凑一条"成功"样本。
    if (result.hits.length === 0) return json({ error: "empty_results" }, 502);
    return json({ results: result.hits });
  },
};
