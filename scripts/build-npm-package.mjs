#!/usr/bin/env node

import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const crateDir = resolve(root, "crates/ironsmith-wasm");
const templateDir = resolve(root, "npm/ironsmith-wasm");

let outDir = resolve(root, "target/npm/ironsmith-wasm");
let expectedVersion;
let noOpt = false;
let skipBuild = false;

for (let index = 2; index < process.argv.length; index += 1) {
  const argument = process.argv[index];
  if (argument === "--out-dir") {
    outDir = resolve(root, process.argv[++index]);
  } else if (argument === "--expected-version") {
    expectedVersion = process.argv[++index];
  } else if (argument === "--no-opt") {
    noOpt = true;
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
  const outDirFromCrate = relative(crateDir, outDir);
  const wasmPackArgs = [
    "build",
    crateDir,
    "--target",
    "web",
    "--release",
    "--out-dir",
    outDirFromCrate,
    "--out-name",
    "ironsmith",
    "--no-default-features",
    "--features",
    "wasm-lean"
  ];
  if (noOpt) {
    wasmPackArgs.push("--no-opt");
  }
  const result = spawnSync("wasm-pack", wasmPackArgs, {
    cwd: root,
    env: {
      ...process.env,
      CARGO_TARGET_DIR:
        resolve(root, process.env.CARGO_TARGET_DIR ?? "target/npm-cargo")
    },
    stdio: "inherit"
  });
  if (result.status !== 0) {
    throw new Error(`wasm-pack failed with status ${result.status ?? "unknown"}`);
  }
}

const requiredFiles = [
  "ironsmith.js",
  "ironsmith_bg.wasm",
  "ironsmith.d.ts",
  "ironsmith_bg.wasm.d.ts"
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
