import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { createDeckCatalog } from "../tools/player-mcp/decks.mjs";

async function fixture(t) {
  const root = await mkdtemp(path.join(os.tmpdir(), "ironsmith-mcp-decks-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const detail = { id: "mtgtop8-1-2", format: "modern", name: "Boros Burn",
    source: "mtgtop8", sourceUrl: "https://mtgtop8.com/event?e=1&d=2&f=MO",
    event: "MTGO Challenge", date: "2026-09-16", placement: 3,
    mainboard: [{ name: "Mountain", count: 56 }, { name: "Lightning Bolt", count: 4 }],
    sideboard: [{ name: "Lórien Revealed", count: 2 }], commander: [], hash: "0123456789abcdef" };
  const entry = { ...detail, mainboardCount: 60, sideboardCount: 2, commanderCount: 0,
    cardNames: ["Mountain", "Lightning Bolt", "Lórien Revealed"], detail: "details/mtgtop8-1-2.json" };
  await mkdir(path.join(root, "modern", "details"), { recursive: true });
  const indexPath = path.join(root, "modern", "index.json");
  const detailPath = path.join(root, "modern", entry.detail);
  const index = { format: "modern", generatedAt: "2026-09-26T00:00:00Z", decks: [entry] };
  await writeFile(indexPath, JSON.stringify(index));
  await writeFile(detailPath, JSON.stringify(detail));
  return { root, index, indexPath, detail, detailPath,
    ...createDeckCatalog({ catalogRoots: [root] }) };
}

test("lists local tournament metadata and searches accented card names", async t => {
  const catalog = await fixture(t);
  const found = await catalog.listDecks({ format: "Modern", query: "lorien revealed", limit: 1 });
  assert.equal(found.length, 1);
  assert.equal(found[0].id, catalog.detail.id);
  assert.equal(found[0].event, "MTGO Challenge");
  assert.equal(found[0].sourceUrl, catalog.detail.sourceUrl);
  assert.deepEqual(found[0].counts, { mainboard: 60, sideboard: 2, commander: 0 });
  assert.deepEqual(await catalog.listDecks({ query: "counterspell" }), []);
  assert.deepEqual(await catalog.listDecks({ limit: 0 }), []);
});

test("returns exact deck sections and provenance for browser import", async t => {
  const catalog = await fixture(t);
  const deck = await catalog.getDeck({ id: catalog.detail.id });
  assert.equal(deck.deckText, "Deck\n56 Mountain\n4 Lightning Bolt\n\nSideboard\n2 Lórien Revealed");
  assert.equal(deck.commanderText, "");
  assert.deepEqual(deck.mainboard, catalog.detail.mainboard);
  assert.equal(deck.sourceMetadata.catalogPath, catalog.detailPath);
  assert.equal(deck.sourceMetadata.hash, catalog.detail.hash);
});

test("prefers source catalog, supports copied assets, and reads updates freshly", async t => {
  const catalog = await fixture(t);
  const fallback = createDeckCatalog({ catalogRoots: [path.join(catalog.root, "missing"), catalog.root] });
  assert.equal((await fallback.getDeck({ id: catalog.detail.id })).id, catalog.detail.id);
  catalog.index.decks[0].name = "Updated Burn";
  await writeFile(catalog.indexPath, JSON.stringify(catalog.index));
  assert.equal((await fallback.listDecks())[0].name, "Updated Burn");
});

test("rejects missing catalogs, unknown ids, and path traversal", async t => {
  const catalog = await fixture(t);
  await assert.rejects(catalog.listDecks({ format: "../modern" }), /Invalid catalog format/);
  await assert.rejects(catalog.getDeck({ id: "../deck" }), /valid catalog deck id/);
  await assert.rejects(catalog.getDeck({ id: "unknown" }), /not found/);
  await assert.rejects(catalog.listDecks({ format: "legacy" }), /No local tournament deck catalog/);
  catalog.index.decks[0].detail = "../../outside.json";
  await writeFile(catalog.indexPath, JSON.stringify(catalog.index));
  await assert.rejects(catalog.getDeck({ id: catalog.detail.id }), /escapes its catalog/);
});

test("rejects invalid or mismatched details instead of silently changing a deck", async t => {
  const catalog = await fixture(t);
  catalog.detail.mainboard[0].count = 0;
  await writeFile(catalog.detailPath, JSON.stringify(catalog.detail));
  await assert.rejects(catalog.getDeck({ id: catalog.detail.id }), /Invalid mainboard/);
  catalog.detail.id = "other";
  await writeFile(catalog.detailPath, JSON.stringify(catalog.detail));
  await assert.rejects(catalog.getDeck({ id: "mtgtop8-1-2" }), /does not match/);
});
