#!/usr/bin/env node

import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

if (process.argv.length < 4) {
  console.error("Usage: node scripts/cpuprofile_to_folded.mjs <input.cpuprofile> <output.folded.txt>");
  process.exit(1);
}

const [, , inputPath, outputPath] = process.argv;

const raw = await readFile(inputPath, "utf8");
const profile = JSON.parse(raw);

if (!Array.isArray(profile.nodes) || !Array.isArray(profile.samples)) {
  throw new Error("Unsupported CPU profile format: expected nodes[] and samples[]");
}

const nodesById = new Map(profile.nodes.map((node) => [node.id, node]));
const stackCache = new Map();
const weights = new Map();
const timeDeltas = Array.isArray(profile.timeDeltas) ? profile.timeDeltas : [];

function frameLabel(node) {
  const callFrame = node.callFrame || {};
  const fn = callFrame.functionName || "(anonymous)";
  const url = callFrame.url ? path.basename(callFrame.url) : "";
  const line = Number.isFinite(callFrame.lineNumber) ? callFrame.lineNumber + 1 : null;
  if (!url) return fn;
  if (line == null) return `${fn} ${url}`;
  return `${fn} ${url}:${line}`;
}

function stackFor(id) {
  if (stackCache.has(id)) return stackCache.get(id);

  const frames = [];
  let current = nodesById.get(id);
  while (current) {
    frames.push(frameLabel(current));
    current = current.parent != null ? nodesById.get(current.parent) : null;
  }
  frames.reverse();
  const filtered = frames.filter(
    (frame) =>
      frame !== "(root)" &&
      frame !== "(program)" &&
      frame !== "(idle)" &&
      frame !== "(garbage collector)"
  );
  stackCache.set(id, filtered);
  return filtered;
}

for (let index = 0; index < profile.samples.length; index += 1) {
  const sampleId = profile.samples[index];
  const stack = stackFor(sampleId);
  if (!stack.length) continue;
  const weight = Math.max(1, Math.round(timeDeltas[index] ?? 1));
  const key = stack.join(";");
  weights.set(key, (weights.get(key) ?? 0) + weight);
}

const lines = [...weights.entries()]
  .sort((left, right) => right[1] - left[1])
  .map(([stack, weight]) => `${stack} ${weight}`)
  .join("\n");

await writeFile(outputPath, `${lines}\n`, "utf8");
