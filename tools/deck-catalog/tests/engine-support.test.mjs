import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { cardEngineStatus, pruneUnsupportedCommanderDecks } from "../engine-support.mjs";

async function withCardRoot(run) {
  const root = await mkdtemp(join(tmpdir(), "deck-catalog-cards-"));
  try {
    await writeFile(join(root, "working-commander.json"), JSON.stringify({ artifacts: [{ diagnostics: { errorCount: 0 } }] }));
    await writeFile(join(root, "broken-commander.json"), JSON.stringify({ artifactError: "no validated strict compilation snapshot" }));
    await writeFile(join(root, "front-face.json"), JSON.stringify({ artifactError: "failed" }));
    await run(root);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

test("reads compile status from the baked card asset", () => withCardRoot(async (root) => {
  assert.equal(await cardEngineStatus("Working Commander", { root }), "ok");
  assert.equal(await cardEngineStatus("Broken Commander", { root }), "unsupported");
  assert.equal(await cardEngineStatus("Front Face // Back Face", { root }), "unsupported");
  // No asset means no verdict, so an unbaked checkout keeps its catalog.
  assert.equal(await cardEngineStatus("Never Baked", { root }), "unknown");
}));

test("drops decks whose commander or partner the engine cannot load", () => withCardRoot(async (root) => {
  const formatDir = await mkdtemp(join(tmpdir(), "deck-catalog-format-"));
  try {
    await mkdir(join(formatDir, "details"));
    const deck = (id, ...commanders) => ({ id, name: id, commander: commanders.map((name) => ({ name, count: 1 })) });
    const decks = [deck("solo", "Working Commander"), deck("pair", "Working Commander", "Broken Commander"), deck("unknown", "Never Baked")];
    for (const { id } of decks) await writeFile(join(formatDir, "details", `${id}.json`), "{}");
    const { kept, removed } = await pruneUnsupportedCommanderDecks(decks, { formatDir, root });
    assert.deepEqual(kept.map((entry) => entry.id), ["solo", "unknown"]);
    assert.deepEqual(removed, [{ id: "pair", name: "pair", unsupported: ["Broken Commander"] }]);
    assert.deepEqual((await readdir(join(formatDir, "details"))).sort(), ["solo.json", "unknown.json"]);
  } finally {
    await rm(formatDir, { recursive: true, force: true });
  }
}));
