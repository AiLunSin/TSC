#!/usr/bin/env python3
"""U-07-43 D 路径：本机检索桥（必应 RSS → `{"results":[{"url","title"}]}`）。

为什么需要它：`http_json` 适配器只发 GET `?q=<query>&format=json`，并严格要求
`{"results":[{"url":"https://...","title":"..."}]}`。必应 RSS 是真实检索、无需
账号与 Key，但输出是 XML，所以由本桥把两侧对齐。

安全与隐私：
- 查询原文只在内存里拼接上游 URL，不写日志、不落盘。
- 设置 `BRIDGE_TOKEN` 后必须带 `Authorization: Bearer <token>`，与 Worker 路径一致。
- 上游无可用结果时返回 502 而不是空列表：空列表会被适配器当成一次“成功”样本。

用法（通常由 `start-bing-bridge.sh` 启动，不必手敲）：
    BRIDGE_TOKEN=xxx python scripts/u07-43/local-bridge.py --port 8788
"""

import argparse
import json
import os
import sys
import time
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

MAX_RESULTS = 5
MAX_TITLE_CHARS = 40
MAX_QUERY_CHARS = 200
UPSTREAM_TIMEOUT = 2.2
UPSTREAM_ATTEMPTS = 3
UPSTREAM_HOST = "https://cn.bing.com/search"
UA = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/124.0 Safari/537.36"
)


def log(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


def host_of(url: str) -> str:
    try:
        return urllib.parse.urlsplit(url).hostname or ""
    except ValueError:
        return ""


def normalize(url: str, title: str) -> dict | None:
    if not url.startswith(("http://", "https://")) or any(ch.isspace() for ch in url):
        return None
    host = host_of(url)
    if not host:
        return None
    name = (title or "").strip() or host
    return {"url": url, "title": name[:MAX_TITLE_CHARS]}


def fetch_bing_rss(query: str) -> list[dict]:
    url = UPSTREAM_HOST + "?" + urllib.parse.urlencode(
        {"q": query, "format": "rss", "count": str(MAX_RESULTS * 2)}
    )
    request = urllib.request.Request(
        url, headers={"User-Agent": UA, "Accept": "application/rss+xml"}
    )
    with urllib.request.urlopen(request, timeout=UPSTREAM_TIMEOUT) as response:
        if response.status != 200:
            raise RuntimeError(f"upstream status {response.status}")
        body = response.read(2 * 1024 * 1024 + 1)
    if len(body) > 2 * 1024 * 1024:
        raise RuntimeError("upstream body too large")
    root = ET.fromstring(body)
    hits: list[dict] = []
    seen: set[str] = set()
    for item in root.iter("item"):
        hit = normalize((item.findtext("link") or "").strip(), item.findtext("title") or "")
        if hit and hit["url"] not in seen:
            seen.add(hit["url"])
            hits.append(hit)
        if len(hits) >= MAX_RESULTS:
            break
    return hits


class Handler(BaseHTTPRequestHandler):
    server_version = "u07-43-bridge/1.0"

    def log_message(self, fmt: str, *args) -> None:  # 不打印 query 原文
        # BaseHTTPRequestHandler 的 fmt 展开后是完整请求行（含 URL 编码的 query），
        # 直接 fmt % args 会把查询原文写进 bridge.log，与文件头"不写日志"的约定冲突。
        # 只保留 args 里的状态码与响应长度，路径取 urlsplit 后的纯 path。
        status = args[1] if len(args) > 1 else "?"
        size = args[2] if len(args) > 2 else "-"
        path = urllib.parse.urlsplit(self.path).path
        # quick tunnel 是公开 URL，扫描器会不断打 /、/favicon.ico 等路径并吃 401；
        # 这些噪音不逐条落日志，只保留 /search 与 /health 的访问记录。
        if path not in ("/search", "/health"):
            return
        log(f"{self.command} {path} {status} {size}")

    def reply(self, body: dict, status: int = 200) -> None:
        payload = json.dumps(body, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("content-type", "application/json; charset=utf-8")
        self.send_header("cache-control", "no-store")
        self.send_header("content-length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self) -> None:  # noqa: N802
        parsed = urllib.parse.urlsplit(self.path)
        if parsed.path == "/health":
            self.reply(
                {
                    "ok": True,
                    "upstream": "bing-rss",
                    "authRequired": bool(os.environ.get("BRIDGE_TOKEN")),
                }
            )
            return

        token = os.environ.get("BRIDGE_TOKEN", "")
        if token:
            if self.headers.get("authorization", "") != f"Bearer {token}":
                self.reply({"error": "unauthorized"}, 401)
                return

        params = urllib.parse.parse_qs(parsed.query)
        query = (params.get("q") or [""])[0].strip()
        if not query:
            self.reply({"error": "missing_query"}, 400)
            return
        if len(query) > MAX_QUERY_CHARS:
            self.reply({"error": "query_too_long"}, 400)
            return

        started = time.monotonic()
        hits: list[dict] = []
        last_error = ""
        for attempt in range(UPSTREAM_ATTEMPTS):
            try:
                hits = fetch_bing_rss(query)
            except Exception as exc:  # noqa: BLE001
                hits = []
                last_error = type(exc).__name__
            if hits:
                break
            if attempt + 1 < UPSTREAM_ATTEMPTS:
                time.sleep(0.2 * (attempt + 1))
        elapsed = int((time.monotonic() - started) * 1000)
        if not hits:
            log(f"no results after {UPSTREAM_ATTEMPTS} attempts in {elapsed}ms last_error={last_error}")
            return self.reply({"error": "empty_results"}, 502)
        log(f"ok hits={len(hits)} elapsed={elapsed}ms")
        self.reply({"results": hits})


def main() -> int:
    parser = argparse.ArgumentParser(description="U-07-43 本机检索桥（必应 RSS）")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8788)
    args = parser.parse_args()
    server = ThreadingHTTPServer((args.host, args.port), Handler)
    log(f"bridge listening on http://{args.host}:{args.port} upstream=bing-rss")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())