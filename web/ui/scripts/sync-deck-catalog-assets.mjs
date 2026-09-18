import { cp, mkdir, rm, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const uiRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(uiRoot, "..", "..");
const source = path.join(repoRoot, "catalog");
const destination = path.join(uiRoot, "public", "catalog");

async function isDirectory(target) {
  try {
    return (await stat(target)).isDirectory();
  } catch (error) {
    if (error.code === "ENOENT") return false;
    throw error;
  }
}

// The catalog is generated, never committed, so a clean checkout has no
// catalog/ directory. Builds must still succeed: the deck browser reports the
// missing catalog itself and every other feature is unaffected.
if (!await isDirectory(source)) {
  await rm(destination, { recursive: true, force: true });
  console.log("No catalog/ directory found; skipping deck catalog assets. Run tools/deck-catalog/sync.mjs to generate one.");
} else {
  await mkdir(path.dirname(destination), { recursive: true });
  await rm(destination, { recursive: true, force: true });
  await cp(source, destination, { recursive: true });
  console.log(`Deck catalog assets copied to ${path.relative(repoRoot, destination)}`);
}
