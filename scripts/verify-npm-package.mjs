#!/usr/bin/env node

import { cp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
let packageDir = resolve(root, "target/npm/ironsmith-wasm");
let skipVite = false;

for (let index = 2; index < process.argv.length; index += 1) {
  const argument = process.argv[index];
  if (argument === "--package-dir") {
    packageDir = resolve(root, process.argv[++index]);
  } else if (argument === "--skip-vite") {
    skipVite = true;
  } else {
    throw new Error(`unknown argument: ${argument}`);
  }
}

const manifest = JSON.parse(await readFile(resolve(packageDir, "package.json"), "utf8"));
if (manifest.name !== "ironsmith-wasm" || !manifest.version) {
  throw new Error("generated package manifest has an invalid name or version");
}

const module = await import(pathToFileURL(resolve(packageDir, "ironsmith.js")));
const [engineBytes, compilerBytes, verifierBytes] = await Promise.all([
  readFile(resolve(packageDir, "engine_bg.wasm")),
  readFile(resolve(packageDir, "compiler_bg.wasm")),
  readFile(resolve(packageDir, "verifier_bg.wasm")),
]);
await module.default({ engine: engineBytes, compiler: compilerBytes, verifier: verifierBytes });
const engine = new module.WasmGame();
if (engine.registrySize() !== 0) {
  throw new Error("npm artifact is not lean: registry should be empty before card loading");
}

const externalResult = engine.registerExternalCardSources({
  canonicalName: "npm Smoke Test",
  group: {
    kind: "single",
    name: "npm Smoke Test",
    block: "Mana cost: {1}{U}\nType: Creature — Wizard\nPower/Toughness: 2/2",
    score: 1
  }
});
if (externalResult.loaded !== 1 || !engine.isKnownCardName("npm Smoke Test")) {
  throw new Error(`external card source smoke test failed: ${JSON.stringify(externalResult)}`);
}

const manabrewResult = engine.registerManabrewDeckSources([{
  name: "npm smoke deck",
  cards: [{
    identity: { name: "Manabrew Smoke Test" },
    manaCost: "{G}",
    types: ["Creature"],
    subtypes: ["Plant"],
    power: "1",
    toughness: "1",
    text: ""
  }]
}]);
if (manabrewResult.loaded !== 1 || !engine.isKnownCardName("Manabrew Smoke Test")) {
  throw new Error(`Manabrew deck source smoke test failed: ${JSON.stringify(manabrewResult)}`);
}
engine.free();

const pack = spawnSync("npm", ["pack", "--dry-run", "--json"], {
  cwd: packageDir,
  encoding: "utf8"
});
if (pack.status !== 0) {
  process.stderr.write(pack.stderr);
  throw new Error(`npm pack --dry-run failed with status ${pack.status ?? "unknown"}`);
}
const packReport = JSON.parse(pack.stdout)[0];
const packedFiles = new Set(packReport.files.map((entry) => entry.path));
for (const file of [
  "package.json",
  "README.md",
  "ironsmith.js",
  "ironsmith.d.ts",
  "engine.js",
  "engine_bg.wasm",
  "compiler.js",
  "compiler_bg.wasm",
  "verifier.js",
  "verifier_bg.wasm"
]) {
  if (!packedFiles.has(file)) {
    throw new Error(`npm tarball is missing ${file}`);
  }
}

if (!skipVite) {
  const fixtureDir = resolve(root, "target/npm/ironsmith-wasm-vite-smoke");
  await rm(fixtureDir, { recursive: true, force: true });
  await mkdir(resolve(fixtureDir, "node_modules"), { recursive: true });
  await cp(packageDir, resolve(fixtureDir, "node_modules/ironsmith-wasm"), {
    recursive: true
  });
  await writeFile(
    resolve(fixtureDir, "package.json"),
    `${JSON.stringify({ private: true, type: "module" }, null, 2)}\n`
  );
  await writeFile(
    resolve(fixtureDir, "index.html"),
    '<!doctype html><html><body><script type="module" src="/src.js"></script></body></html>\n'
  );
  await writeFile(
    resolve(fixtureDir, "src.js"),
    'import init, { WasmGame } from "ironsmith-wasm";\nexport async function createEngine() { await init(); return new WasmGame(); }\n'
  );
  await writeFile(
    resolve(fixtureDir, "vite.config.mjs"),
    'export default { build: { target: "esnext" } };\n'
  );

  const localVite = resolve(root, "web/ui/node_modules/vite/bin/vite.js");
  let vite;
  try {
    await readFile(localVite);
    vite = spawnSync(process.execPath, [localVite, "build"], {
      cwd: fixtureDir,
      stdio: "inherit"
    });
  } catch {
    vite = spawnSync("vite", ["build"], { cwd: fixtureDir, stdio: "inherit" });
  }
  if (vite.status !== 0) {
    throw new Error(
      "Vite consumer smoke test failed; install Vite 7.3.1 or pass --skip-vite"
    );
  }
  const assets = await readdir(resolve(fixtureDir, "dist/assets"));
  if (!assets.some((file) => file.endsWith(".wasm"))) {
    throw new Error("Vite consumer build did not emit the Ironsmith WASM asset");
  }
}

console.log(
  `Verified ${manifest.name}@${manifest.version}: Node, card loading, npm tarball${
    skipVite ? "" : ", and Vite consumer"
  }`
);
