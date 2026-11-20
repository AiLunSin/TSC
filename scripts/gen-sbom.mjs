import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

const [, , rootArg, versionArg, outArg] = process.argv;
const root = resolve(rootArg ?? ".");
const pkg = JSON.parse(readFileSync(join(root, "app", "package.json"), "utf8"));
const version = versionArg ?? pkg.version;
const outFile = outArg ?? join(root, "docs", "compliance", "sbom", `sbom-${version}.cdx.json`);

const components = new Map();

function add(component) {
  const key = `${component.purl}`;
  if (!components.has(key)) {
    components.set(key, component);
  }
}

const npmLock = JSON.parse(readFileSync(join(root, "app", "package-lock.json"), "utf8"));
for (const [path, meta] of Object.entries(npmLock.packages ?? {})) {
  if (!path.startsWith("node_modules/") || typeof meta.version !== "string") {
    continue;
  }
  const name = path.split("node_modules/").filter(Boolean).pop();
  if (name === undefined) {
    continue;
  }
  add({
    type: "library",
    name,
    version: meta.version,
    purl: `pkg:npm/${name}@${meta.version}`,
    properties: [{ name: "ecosystem", value: "npm" }],
  });
}

const cargoLock = readFileSync(join(root, "Cargo.lock"), "utf8");
for (const block of cargoLock.split("[[package]]").slice(1)) {
  const name = /name = "([^"]+)"/.exec(block)?.[1];
  const crateVersion = /version = "([^"]+)"/.exec(block)?.[1];
  if (name === undefined || crateVersion === undefined) {
    continue;
  }
  add({
    type: "library",
    name,
    version: crateVersion,
    purl: `pkg:cargo/${name}@${crateVersion}`,
    properties: [{ name: "ecosystem", value: "cargo" }],
  });
}

if (components.size === 0) {
  console.error("SBOM 为空：未解析到任何依赖");
  process.exit(1);
}

const document = {
  bomFormat: "CycloneDX",
  specVersion: "1.5",
  version: 1,
  metadata: {
    timestamp: new Date().toISOString(),
    tools: [{ name: "meeting-copilot-lockfile-sbom", version: "1" }],
    component: { type: "application", name: "meeting-copilot", version },
  },
  components: [...components.values()],
};

mkdirSync(dirname(outFile), { recursive: true });
writeFileSync(outFile, `${JSON.stringify(document, null, 2)}\n`, "utf8");
console.log(`SBOM: ${outFile}（${components.size} 个组件）`);