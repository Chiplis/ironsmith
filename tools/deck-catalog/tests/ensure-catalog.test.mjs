import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { countSavedDecks, ensureCatalog } from "../ensure-catalog.mjs";

test("backfills distinct history pages until saved decks reach the minimum", async () => {
  const pages = [];
  const counts = [0, 48, 310, 500];
  const saved = await ensureCatalog({
    format: "modern", output: "/unused", count: async () => counts.shift(),
    sync: async (args) => pages.push(args[args.indexOf("--page") + 1]),
  });
  assert.equal(saved, 500);
  assert.deepEqual(pages, ["0", "2", "3"]);
});

test("refreshes existing full catalogs without downloading history", async () => {
  let calls = 0;
  await ensureCatalog({ format: "PAU", output: "/unused", count: async () => 510, sync: async () => calls++ });
  assert.equal(calls, 1);
});

test("fails instead of reporting success below the minimum", async () => {
  await assert.rejects(ensureCatalog({
    format: "vintage", output: "/unused", maxPages: 2,
    count: async () => 12, sync: async () => {},
  }), /only 12\/500 usable decks/);
});

test("counts unique usable detail files, not stale or duplicate index entries", async () => {
  const output = await mkdtemp(join(tmpdir(), "deck-catalog-test-"));
  try {
    await mkdir(join(output, "commander", "details"), { recursive: true });
    const valid = { id: "valid", format: "commander", mainboard: [{ name: "Island", count: 98 }], commander: [{ name: "Partner A", count: 1 }, { name: "Partner B", count: 1 }] };
    const invalid = { ...valid, id: "invalid", commander: [] };
    for (const deck of [valid, invalid]) await writeFile(join(output, "commander", "details", `${deck.id}.json`), JSON.stringify(deck));
    await writeFile(join(output, "commander", "index.json"), JSON.stringify({ decks: [
      { id: "valid", detail: "details/valid.json" }, { id: "valid", detail: "details/valid.json" },
      { id: "invalid", detail: "details/invalid.json" }, { id: "missing", detail: "details/missing.json" },
      { id: "unreferenced" }, { id: "wrong-id", detail: "details/valid.json" },
    ] }));
    assert.equal(await countSavedDecks(output, "commander"), 1);
    assert.equal(await countSavedDecks(output, "modern"), 0);
  } finally {
    await rm(output, { recursive: true, force: true });
  }
});
