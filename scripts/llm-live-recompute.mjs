#!/usr/bin/env node
// 由 samples.jsonl 独立重算 p50/p95（nearest-rank，排除降级与已排除样本）。
// 零依赖；只读文件，不写 .tmp 以外位置，不打印样本正文。

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

const args = process.argv.slice(2);

const SAMPLE_KEYS = new Set([
  "seq",
  "task",
  "attempt",
  "first_token_ms",
  "total_ms",
  "outcome",
  "failure_class",
  "http_status",
  "degraded",
  "excluded",
]);

const RUN_V2_KEYS = new Set([
  "schema_version",
  "endpoint_host",
  "model",
  "started_at",
  "ended_at",
  "tz_offset",
  "commit_short",
  "config",
  "per_task",
  "sub_status",
  "samples_path",
  "stats_rule",
  "build",
  "path_confirmed",
  "call_limit",
  "calls_used",
  "budget_hit",
  "model_confirmed",
  "host_locked",
  "data_terms_declaration",
  "data_terms_accepted_at",
  "credential_rotated",
  "tls_trust_root",
]);

function percentile(values, q) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const rank = Math.max(1, Math.ceil(q * sorted.length));
  return sorted[rank - 1];
}

function summarize(samples) {
  const byTask = new Map();
  for (const sample of samples) {
    if (!byTask.has(sample.task)) byTask.set(sample.task, []);
    byTask.get(sample.task).push(sample);
  }
  return [...byTask.entries()].map(([task, rows]) => {
    const valid = rows
      .filter((row) => !row.degraded && !row.excluded)
      .map((row) => row.total_ms);
    return {
      task,
      n: rows.length,
      p50_ms: percentile(valid, 0.5),
      p95_ms: percentile(valid, 0.95),
      degraded_count: rows.filter((row) => row.degraded).length,
    };
  });
}

function checkRunV2(path) {
  let run;
  try {
    run = JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return 1;
  }
  if (run.schema_version !== "llm.live.v2") return 2;
  const unknown = Object.keys(run).filter((key) => !RUN_V2_KEYS.has(key));
  if (unknown.length > 0) {
    for (const key of unknown) console.error(`unknown run key: ${key}`);
    return 1;
  }
  for (const key of [
    "call_limit",
    "calls_used",
    "budget_hit",
    "path_confirmed",
    "model_confirmed",
    "host_locked",
    "credential_rotated",
    "data_terms_declaration",
    "data_terms_accepted_at",
  ]) {
    if (!(key in run)) return 1;
  }
  return 0;
}

function selfTest() {
  const samples = Array.from({ length: 30 }, (_, index) => ({
    seq: index,
    task: "summary",
    attempt: 1,
    first_token_ms: 100,
    total_ms: index + 1,
    outcome: "ok",
    failure_class: null,
    http_status: 200,
    degraded: false,
    excluded: null,
  }));
  const [stats] = summarize(samples);
  if (stats.p95_ms !== 29 || stats.p50_ms !== 15 || stats.n !== 30) {
    console.error("self-test failed", stats);
    process.exit(1);
  }
  console.log("self-test ok", JSON.stringify(stats));
}

if (args.includes("--self-test")) {
  selfTest();
  process.exit(0);
}

const path = args[0];
if (!path) {
  console.error("未找到 samples.jsonl；请显式传入路径");
  process.exit(1);
}
const lines = readFileSync(resolve(path), "utf8")
  .split(/\r?\n/)
  .filter((line) => line.trim().length > 0);
const samples = lines.map((line) => JSON.parse(line));
let unknownCount = 0;
for (const sample of samples) {
  const unknown = Object.keys(sample).filter((key) => !SAMPLE_KEYS.has(key));
  if (unknown.length > 0) {
    for (const key of unknown) console.error(`unknown sample key: ${key}`);
    unknownCount += 1;
  }
}
if (unknownCount > 0) process.exit(1);

const runPath = join(dirname(resolve(path)), "run.json");
if (checkRunV2(runPath) !== 0) {
  console.error("run.json 缺失、schema 非 v2 或键集不完整");
  process.exit(1);
}

console.log(
  JSON.stringify({ stats_rule: "nearest-rank", per_task: summarize(samples) }, null, 2)
);