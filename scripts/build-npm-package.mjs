#!/usr/bin/env node

import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { prepareExactBuildSnapshots } from './prepare-exact-build-snapshots.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const crateDir = resolve(root, "crates/ironsmith-wasm");
const templateDir = resolve(root, "npm/ironsmith-wasm");
// Each product is a cdylib crate; `artifact` is its file name in the cargo
// output directory.
const products = [
  { name: "engine", package: "ironsmith-engine-wasm", artifact: "ironsmith_engine_wasm.wasm" },
  { name: "verifier", package: "ironsmith-verifier-wasm", artifact: "ironsmith_verifier_wasm.wasm" },
];
// The size-optimized profile (opt-level "z", fat LTO). Plain `release` yields
// an engine module about twice as large.
const cargoProfile = "wasm-dist";

let outDir = resolve(root, "target/npm/ironsmith-wasm");
let expectedVersion;
let skipBuild = false;

for (let index = 2; index < process.argv.length; index += 1) {
  const argument = process.argv[index];
  if (argument === "--out-dir") {
    outDir = resolve(root, process.argv[++index]);
  } else if (argument === "--expected-version") {
    expectedVersion = process.argv[++index];
  } else if (argument === "--no-opt") {
    // Accepted for compatibility; the build no longer runs a separate optimizer.
  } else if (argument === "--skip-build") {
    skipBuild = true;
  } else {
    throw new Error(`unknown argument: ${argument}`);
  }
}

const cargoManifest = await readFile(resolve(crateDir, "Cargo.toml"), "utf8");
const packageSection = cargoManifest.match(/\[package\]([\s\S]*?)(?:\n\[|$)/)?.[1];
const version = packageSection?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
if (!version) {
  throw new Error("could not read ironsmith-wasm version from Cargo.toml");
}
if (expectedVersion && expectedVersion !== version) {
  throw new Error(`release version mismatch: tag=${expectedVersion}, crate=${version}`);
}

if (!skipBuild) {
  await rm(outDir, { recursive: true, force: true });
  await mkdir(outDir, { recursive: true });
  const targetDir = resolve(root, process.env.CARGO_TARGET_DIR ?? "target/npm-cargo");
  const run = (command, args) => {
    const result = spawnSync(command, args, {
      cwd: root,
      env: { ...process.env, CARGO_TARGET_DIR: targetDir },
      stdio: "inherit"
    });
    if (result.status !== 0) {
      throw new Error(`${command} failed with status ${result.status ?? result.error ?? "unknown"}`);
    }
  };
  // wasm-pack cannot select a custom cargo profile, so build and bind directly.
  // The wasm-bindgen CLI must match the wasm-bindgen crate version in Cargo.lock.
  run("cargo", [
    "build",
    "--locked",
    "--profile",
    cargoProfile,
    "--target",
    "wasm32-unknown-unknown",
    ...products.flatMap((product) => ["-p", product.package]),
    "--no-default-features",
    "--features",
    "ironsmith-engine-wasm/wasm-lean",
  ]);
  for (const product of products) {
    run("wasm-bindgen", [
      resolve(targetDir, "wasm32-unknown-unknown", cargoProfile, product.artifact),
      "--target",
      "web",
      "--out-dir",
      outDir,
      "--out-name",
      product.name,
    ]);
  }
  await cp(resolve(templateDir, "split-facade.js"), resolve(outDir, "ironsmith.js"));
  await cp(resolve(templateDir, "split-facade.d.ts"), resolve(outDir, "ironsmith.d.ts"));
}

// The engine module compiles card source itself, so the package ships a stub in
// place of the standalone compiler module (and drops any stale binary).
await cp(resolve(templateDir, "compiler-stub.js"), resolve(outDir, "compiler.js"));
await cp(resolve(templateDir, "compiler-stub.d.ts"), resolve(outDir, "compiler.d.ts"));
await rm(resolve(outDir, "compiler_bg.wasm"), { force: true });
await rm(resolve(outDir, "compiler_bg.wasm.d.ts"), { force: true });

await prepareExactBuildSnapshots(outDir);

const requiredFiles = [
  "ironsmith.js",
  "ironsmith.d.ts",
  "engine.js",
  "engine_bg.wasm",
  "compiler.js",
  "verifier.js",
  "verifier_bg.wasm"
];
for (const file of requiredFiles) {
  await readFile(resolve(outDir, file));
}

const template = JSON.parse(
  await readFile(resolve(templateDir, "package.template.json"), "utf8")
);
await cp(resolve(templateDir, "README.md"), resolve(outDir, "README.md"));
await writeFile(
  resolve(outDir, "package.json"),
  `${JSON.stringify({ ...template, version }, null, 2)}\n`,
  "utf8"
);

console.log(`Built ${template.name}@${version} in ${outDir}`);
