// U-07-43 采样用的 HTTPS 检索适配端点（Cloudflare Workers / Deno Deploy 通用）。
//
// 背景：`infrastructure::search::http_json` 只发 GET `?q=<query>&format=json`，
// 且要求响应形如 `{"results":[{"url":"https://...","title":"..."}]}`；
// 博查 / Tavily 都是 POST + 自有响应形状，无法直接接进 `search_live.rs`。
// 本 Worker 把两侧对上，让已有的检索 Key 能用于 200 条真实样本采集。
//
// 部署（Cloudflare Workers）：
//   wrangler deploy
//   wrangler secret put UPSTREAM_KEY       # 博查/Tavily 的 Key
//   wrangler secret put SHARED_TOKEN       # 自定义一段随机串，给采样脚本当 Bearer
//   wrangler secret put UPSTREAM_PROVIDER  # bocha 或 tavily（也可用 vars 配置）
//
// 然后把 Worker 的公开 HTTPS 地址填给采样脚本的 SEARCH_ENDPOINT，
// 例如 `https://u07-43-search.<your-subdomain>.workers.dev/search`。
//
// 注意：不要把任何 Key 写进本文件、providers.json 或提交信息。
// 查询原文只在请求内存中使用，不写日志；失败时只回状态码。

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
  const title = typeof raw.title === "string" ? raw.title : typeof raw.name === "string" ? raw.name : "";
  const published = raw.publishedDate ?? raw.published_date ?? raw.dateLastCrawled ?? null;
  // 适配器只接受 http(s) 绝对地址且不含空白，这里先过滤，避免整批被判 MalformedResponse。
  if (!/^https?:\/\//i.test(url) || /\s/.test(url)) return null;
  const hit = { url, title: title.slice(0, 200) };
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
    const query = (url.searchParams.get("q") ?? "").trim();

    const expected = env.SHARED_TOKEN;
    if (expected) {
      const auth = request.headers.get("authorization") ?? "";
      if (auth !== `Bearer ${expected}`) {
        return json({ error: "unauthorized" }, 401);
      }
    }
    if (!query) return json({ error: "missing_query" }, 400);

    const provider = (env.UPSTREAM_PROVIDER ?? "bocha").toLowerCase();
    const config = UPSTREAM[provider];
    if (!config) return json({ error: "unknown_provider" }, 500);
    if (!env.UPSTREAM_KEY) return json({ error: "upstream_key_missing" }, 500);

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
