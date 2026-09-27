import test from "node:test";
import assert from "node:assert/strict";
import {
  installEmbeddedCardCatalog,
  readEmbeddedCardCatalogIndex,
  readEmbeddedCardSource,
} from "../src/lib/embedded-card-catalog.js";
import { loadLocalCardArt } from "../src/lib/catalog-client.js";
import { fetchScryfallCardMeta, resolveScryfallImageUrl } from "../src/lib/scryfall.js";
import { loadRandomGameIndex, collectRandomGameCards } from "../src/lib/random-game-catalog.js";
import { randomGameDefaults } from "../src/lib/random-game.js";

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

test("catalogue reads wait for engine readiness and share its index", async () => {
  const ready = deferred();
  const calls = [];
  const release = installEmbeddedCardCatalog({
    ready: ready.promise,
    getIndexJson: () => { calls.push("index"); return '{"cards":[{"route":"bolt"}]}'; },
    getSourceJson: (route) => { calls.push(route); return '{"group":{"name":"Bolt"}}'; },
  });
  try {
    const source = readEmbeddedCardSource("bolt");
    const index = readEmbeddedCardCatalogIndex();
    await Promise.resolve();
    assert.deepEqual(calls, []);
    ready.resolve();
    assert.deepEqual(await source, { group: { name: "Bolt" } });
    assert.deepEqual(await index, { cards: [{ route: "bolt" }] });
    assert.deepEqual(calls, ["index", "bolt"]);
  } finally { release(); }
});

test("only a runtime without an embedded catalogue permits asset fallback", async () => {
  assert.equal(await readEmbeddedCardSource("unknown"), undefined);
  const releaseMissing = installEmbeddedCardCatalog({
    ready: Promise.resolve(),
    getIndexJson: () => null,
    getSourceJson: () => { throw new Error("source must not be read"); },
  });
  try { assert.equal(await readEmbeddedCardSource("unknown"), undefined); }
  finally { releaseMissing(); }
  const release = installEmbeddedCardCatalog({
    ready: Promise.resolve(),
    getIndexJson: () => '{"cards":[]}',
    getSourceJson: () => null,
  });
  try { assert.equal(await readEmbeddedCardSource("unknown"), null); }
  finally { release(); }
});

test("unmount rejects waiting reads without releasing a newer provider", async () => {
  const oldReady = deferred();
  const releaseOld = installEmbeddedCardCatalog({
    ready: oldReady.promise,
    getIndexJson: () => '{"cards":[]}',
    getSourceJson: () => null,
  });
  const waiting = readEmbeddedCardSource("pending");
  const rejection = assert.rejects(waiting, /terminated/);
  const releaseNew = installEmbeddedCardCatalog({
    ready: Promise.resolve(),
    getIndexJson: () => '{"cards":[{"route":"new"}]}',
    getSourceJson: () => '{"canonicalName":"New"}',
  });
  try {
    releaseOld(new Error("worker terminated"));
    await rejection;
    assert.deepEqual(await readEmbeddedCardSource("new"), { canonicalName: "New" });
  } finally { releaseNew(); }
});

test("initialization errors reject pending catalogue reads", async () => {
  const ready = deferred();
  const release = installEmbeddedCardCatalog({
    ready: ready.promise,
    getIndexJson: () => '{"cards":[]}',
    getSourceJson: () => null,
  });
  try {
    const request = readEmbeddedCardCatalogIndex();
    const rejection = assert.rejects(request, /engine unavailable/);
    ready.reject(new Error("engine unavailable"));
    await rejection;
  } finally { release(); }
});

test("cleanup rejects a source call already waiting on the worker", async () => {
  const source = deferred();
  const started = deferred();
  const release = installEmbeddedCardCatalog({
    ready: Promise.resolve(),
    getIndexJson: () => '{"cards":[]}',
    getSourceJson: () => { started.resolve(); return source.promise; },
  });
  const request = readEmbeddedCardSource("pending");
  const rejection = assert.rejects(request, /worker stopped/);
  await started.promise;
  release(new Error("worker stopped"));
  await rejection;
  assert.equal(await readEmbeddedCardCatalogIndex(), undefined);
});

test("legacy catalogue absence falls back to HTTP after readiness", async () => {
  const ready = deferred();
  const release = installEmbeddedCardCatalog({
    ready: ready.promise,
    getIndexJson: () => null,
    getSourceJson: () => { throw new Error("legacy runtime has no source method"); },
  });
  const urls = [];
  const fetchImpl = async (url) => {
    urls.push(String(url));
    return { ok: true, json: async () => ({ cards: [{ name: "Legacy", route: "legacy" }] }) };
  };
  try {
    const request = loadRandomGameIndex({ fetchImpl });
    await Promise.resolve();
    assert.deepEqual(urls, []);
    ready.resolve();
    assert.deepEqual(await request, [{ name: "Legacy", route: "legacy" }]);
    assert.equal(urls.length, 1);
    assert.match(urls[0], /\/cards\/index\.json/);
  } finally { release(); }
});

test("metadata, art and random card reads use embedded data without card HTTP requests", async () => {
  const name = "Embedded Catalog Test Card";
  const route = "embedded-catalog-test-card";
  const imageUrl = "https://cards.example.test/embedded.jpg";
  const payload = {
    canonicalName: name,
    group: { name, score: 1, block: "Type: Creature\nEmbedded source text." },
    artifacts: [{ payload: { definition: { card: {
      card_types: ["Creature"], supertypes: [], subtypes: [], mana_cost: null,
      is_token: false, linked_face_layout: "None",
    } } } }],
    scryfall: {
      standard_printing: true, frame: "2015", oracle_text: "Embedded source text.",
      mana_cost: "{R}", produced_mana: [], image_uris: { normal: imageUrl, art_crop: imageUrl },
    },
  };
  const release = installEmbeddedCardCatalog({
    ready: Promise.resolve(),
    getIndexJson: () => JSON.stringify({ cards: [{ name, route, score: 1 }] }),
    getSourceJson: (requested) => requested === route ? JSON.stringify(payload) : null,
  });
  const originalFetch = globalThis.fetch;
  const requests = [];
  globalThis.fetch = async (url) => {
    requests.push(String(url));
    if (String(url).includes("random-card-pool.json")) return { ok: false, status: 404 };
    throw new Error(`Unexpected HTTP request: ${url}`);
  };
  try {
    assert.equal(await resolveScryfallImageUrl(name), imageUrl);
    assert.deepEqual(await fetchScryfallCardMeta(name), {
      mana_cost: "{R}", oracle_text: "Embedded source text.", produced_mana: [],
    });
    assert.equal(await loadLocalCardArt(name), imageUrl);
    assert.equal(await loadLocalCardArt("Unknown Embedded Card"), "");
    assert.deepEqual(await loadRandomGameIndex(), [{ name, route, score: 1 }]);
    const { cards } = await collectRandomGameCards({ config: randomGameDefaults(), want: 1 });
    assert.equal(cards[0]?.name, name);
    assert.deepEqual(requests.filter((url) => !url.includes("random-card-pool.json")), []);
  } finally {
    release();
    globalThis.fetch = originalFetch;
  }
});
