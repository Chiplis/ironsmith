#!/usr/bin/env node

import { cp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const crateDir = resolve(root, "crates/ironsmith-wasm");
const templateDir = resolve(root, "npm/ironsmith-wasm");
const productDirs = [
  { name: "engine", crateDir, features: ["wasm-lean"] },
  { name: "compiler", crateDir: resolve(root, "crates/ironsmith-compiler-wasm"), features: [] },
  { name: "verifier", crateDir: resolve(root, "crates/ironsmith-verifier-wasm"), features: [] },
];

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
  const partsDir = resolve(root, "target/npm/ironsmith-wasm-parts");
  await rm(partsDir, { recursive: true, force: true });
  await mkdir(partsDir, { recursive: true });
  for (const product of productDirs) {
    const productOut = resolve(partsDir, product.name);
    const wasmPackArgs = [
      "build",
      "--target",
      "web",
      "--release",
      "--out-dir",
      relative(product.crateDir, productOut),
      "--out-name",
      product.name,
    ];
    if (noOpt) wasmPackArgs.push("--no-opt");
    wasmPackArgs.push(product.crateDir, "--no-default-features");
    if (product.features.length > 0) {
      wasmPackArgs.push("--features", product.features.join(","));
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
      throw new Error(
        `wasm-pack failed for ${product.name} with status ${result.status ?? "unknown"}`
      );
    }
    for (const entry of await readdir(productOut)) {
      if (entry === "package.json" || entry === "README.md" || entry === ".gitignore") continue;
      await cp(resolve(productOut, entry), resolve(outDir, entry), { recursive: true });
    }
  }
  await cp(resolve(templateDir, "split-facade.js"), resolve(outDir, "ironsmith.js"));
  await cp(resolve(templateDir, "split-facade.d.ts"), resolve(outDir, "ironsmith.d.ts"));
}

const requiredFiles = [
  "ironsmith.js",
  "ironsmith.d.ts",
  "engine.js",
  "engine_bg.wasm",
  "compiler.js",
  "compiler_bg.wasm",
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
