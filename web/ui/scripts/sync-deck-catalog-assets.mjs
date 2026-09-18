import { cp, mkdir, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const uiRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(uiRoot, "..", "..");
const source = path.join(repoRoot, "catalog");
const destination = path.join(uiRoot, "public", "catalog");

await mkdir(path.dirname(destination), { recursive: true });
await rm(destination, { recursive: true, force: true });
await cp(source, destination, { recursive: true });

console.log(`Deck catalog assets copied to ${path.relative(repoRoot, destination)}`);
