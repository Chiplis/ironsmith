#!/usr/bin/env node
// Bundles the Verified session (web/ui/src/lib/verified-session) into one ES
// module for the npm package, so hosts other than the web app (Manabrew) can
// run the same committed-deck / signed-action protocol.
//
//   node scripts/build-verified-session.mjs <out-dir>
//
// Needs web/ui's dependencies installed (esbuild comes from there).
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { readFile, writeFile } from "node:fs/promises";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const uiDir = resolve(root, "web/ui");
const outDir = resolve(process.argv[2] ?? resolve(root, "target/npm/ironsmith-wasm"));

async function loadEsbuild() {
  const requireFromUi = createRequire(resolve(uiDir, "package.json"));
  for (const request of ["esbuild", "vite/node_modules/esbuild"]) {
    try { return requireFromUi(request); } catch { /* try the next location */ }
  }
  const { readdirSync } = await import("node:fs");
  const store = resolve(uiDir, "node_modules/.pnpm");
  const entry = readdirSync(store).find((name) => name.startsWith("esbuild@"));
  if (entry) return createRequire(resolve(store, entry, "node_modules/esbuild/package.json"))("esbuild");
  throw new Error("esbuild is not installed in web/ui; run `pnpm install` there first");
}

const esbuild = await loadEsbuild();
const outfile = resolve(outDir, "verified-session.js");
await esbuild.build({
  entryPoints: [resolve(uiDir, "src/lib/verified-session/index.js")],
  bundle: true,
  format: "esm",
  platform: "browser",
  target: "es2022",
  minify: true,
  legalComments: "none",
  // React is only touched when the hooks run inside a React render; a host
  // without React never reaches it, and one with React shares its copy.
  external: ["react"],
  outfile,
  logLevel: "warning",
});
await writeFile(
  resolve(outDir, "verified-session.d.ts"),
  await readFile(resolve(root, "npm/ironsmith-wasm/verified-session.d.ts"), "utf8"),
);
console.log(`Built ${outfile}`);
