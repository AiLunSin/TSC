import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const schemaPath = join(root, "app", "src", "bridge", "schema.json");
const typesPath = join(root, "app", "src", "bridge", "types.ts");

const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
const source = readFileSync(typesPath, "utf8");

function interfaceFields(name) {
  const pattern = new RegExp(`export interface ${name}\\s*{([\\s\\S]*?)}`, "m");
  const block = pattern.exec(source);
  if (block === null) {
    return null;
  }
  return [...block[1].matchAll(/^\s*([A-Za-z_$][\w$]*)\??\s*:/gm)].map(
    (match) => match[1],
  );
}

const problems = [];
for (const [name, expected] of Object.entries(schema)) {
  const actual = interfaceFields(name);
  if (actual === null) {
    problems.push(`${name}: types.ts 中缺少 interface`);
    continue;
  }
  const missing = expected.filter((field) => !actual.includes(field));
  const extra = actual.filter((field) => !expected.includes(field));
  if (missing.length > 0) {
    problems.push(`${name}: 缺少字段 ${missing.join(", ")}`);
  }
  if (extra.length > 0) {
    problems.push(`${name}: 多出字段 ${extra.join(", ")}`);
  }
}

const versionPattern = /IPC_PROTOCOL_VERSION\s*=\s*(\d+)/;
const version = versionPattern.exec(source);
if (version === null || Number(version[1]) !== 1) {
  problems.push("IPC_PROTOCOL_VERSION 必须为 1");
}

if (problems.length > 0) {
  console.error("绑定契约不一致：");
  for (const problem of problems) {
    console.error(` - ${problem}`);
  }
  process.exit(1);
}

console.log(`绑定契约一致：${Object.keys(schema).length} 个 DTO`);