import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [, , tmpDir, outFile] = process.argv;
if (tmpDir === undefined || outFile === undefined) {
  console.error("用法: node merge-sbom.mjs <tmpDir> <outFile>");
  process.exit(1);
}

const parts = readdirSync(tmpDir)
  .filter((name) => name.endsWith(".cdx.json") || name.endsWith(".json"))
  .map((name) => {
    const raw = readFileSync(join(tmpDir, name), "utf8");
    return { name, doc: JSON.parse(raw) };
  });

const components = new Map();
for (const { doc } of parts) {
  const list = Array.isArray(doc.components) ? doc.components : [];
  for (const component of list) {
    const key = component["bom-ref"] ?? `${component.name}@${component.version ?? "unknown"}`;
    if (!components.has(key)) {
      components.set(key, component);
    }
  }
}

const merged = {
  bomFormat: "CycloneDX",
  specVersion: "1.5",
  version: 1,
  metadata: {
    timestamp: new Date().toISOString(),
    tools: [{ name: "meeting-copilot-sbom-merger" }],
    component: { type: "application", name: "meeting-copilot" },
  },
  components: [...components.values()],
};

writeFileSync(outFile, `${JSON.stringify(merged, null, 2)}\n`, "utf8");
console.log(`merged ${String(components.size)} components -> ${outFile}`);