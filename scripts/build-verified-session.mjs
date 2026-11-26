#!/usr/bin/env node
// Bundles the Verified session (web/ui/src/lib/verified-session) into one ES
// module for the npm package, so hosts other than the web app (Manabrew) can
// run the same committed-deck / signed-action protocol.
//
//   node scripts/build-verified-session.mjs <out-dir>
//
// Needs esbuild: from web/ui's dependencies, or installed globally (the npm
// release workflow installs it with vite).
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
  // The release workflow has no web/ui install: use the esbuild next to the
  // globally installed tools.
  try {
    const { execFileSync } = await import("node:child_process");
    const globalRoot = execFileSync("npm", ["root", "-g"], { encoding: "utf8" }).trim();
    for (const location of [resolve(globalRoot, "esbuild"), resolve(globalRoot, "vite/node_modules/esbuild")]) {
      try { return createRequire(resolve(location, "package.json"))("esbuild"); } catch { /* next */ }
    }
  } catch { /* npm unavailable */ }
  const { readdirSync } = await import("node:fs");
  const store = resolve(uiDir, "node_modules/.pnpm");
  const entry = readdirSync(store).find((name) => name.startsWith("esbuild@"));
  if (entry) return createRequire(resolve(store, entry, "node_modules/esbuild/package.json"))("esbuild");
  throw new Error("esbuild is not installed in web/ui; run `pnpm install` there first");
}

const esbuild = await loadEsbuild();
const outfile = resolve(outDir, "verified-session.js");
// Two modules the session shares with the web app (foil-cards, lobby-deck)
// also export React hooks beside the plain functions the session calls. The
// bundle must build without React installed and must not carry it: `react`
// resolves to the session's own hook runtime, and a hook that runtime does
// not provide fails loudly if anything ever calls it headless.
const hookRuntime = resolve(uiDir, "src/lib/verified-session/hook-runtime.js");
const headlessReact = {
  name: "headless-react",
  setup(build) {
    build.onResolve({ filter: /^react$/ }, () => ({ path: "react", namespace: "headless-react" }));
    build.onLoad({ filter: /.*/, namespace: "headless-react" }, () => ({
      resolveDir: uiDir,
      contents: `
        export { useState, useRef, useCallback, useEffect } from ${JSON.stringify(hookRuntime)};
        const unavailable = (name) => () => {
          throw new Error(\`React's \${name} is not available in the headless Verified session\`);
        };
        export const useMemo = unavailable("useMemo");
        export const useSyncExternalStore = unavailable("useSyncExternalStore");
      `,
    }));
  },
};
await esbuild.build({
  entryPoints: [resolve(uiDir, "src/lib/verified-session/index.js")],
  bundle: true,
  format: "esm",
  platform: "browser",
  target: "es2022",
  minify: true,
  legalComments: "none",
  plugins: [headlessReact],
  outfile,
  logLevel: "warning",
});
await writeFile(
  resolve(outDir, "verified-session.d.ts"),
  await readFile(resolve(root, "npm/ironsmith-wasm/verified-session.d.ts"), "utf8"),
);
console.log(`Built ${outfile}`);
