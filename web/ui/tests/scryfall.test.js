import test from "node:test";
import assert from "node:assert/strict";
import {
  HIDDEN_CARD_BACK_IMAGE_URL,
  customCardArtUrl,
  preloadCardArt,
  resolveScryfallImageUrl,
  resolveScryfallFlavorText,
  scryfallImageUrl,
  setCustomCardArtUrls,
  setPreferredCardPrints,
  fetchScryfallCardMeta,
  resolveScryfallLocalizedImageUrl,
} from "../src/lib/scryfall.js";

function installLocalStorageMock() {
  const store = new Map();
  globalThis.localStorage = {
    getItem: (key) => store.get(key) ?? null,
    setItem: (key, value) => {
      store.set(key, String(value));
    },
    removeItem: (key) => {
      store.delete(key);
    },
  };
}

test("flavor lookup follows the displayed printing and face, including an empty face", async () => {
  const originalFetch = globalThis.fetch;
  const id = "12345678-1234-1234-1234-123456789abc";
  const front = `https://cards.scryfall.io/art_crop/front/1/2/${id}.jpg`;
  const back = `https://cards.scryfall.io/art_crop/back/1/2/${id}.jpg`;
  let calls = 0;
  globalThis.fetch = async (url) => {
    calls += 1;
    assert.equal(url, `https://api.scryfall.com/cards/${id}`);
    return { ok: true, json: async () => ({ card_faces: [
      { image_uris: { art_crop: `${front}?new` }, flavor_text: "The front's story.\n—An author" },
      { image_uris: { art_crop: back } },
    ] }) };
  };
  try {
    assert.equal(await resolveScryfallFlavorText(`${front}?old`), "The front's story.\n—An author");
    assert.equal(await resolveScryfallFlavorText(back), "");
    assert.equal(await resolveScryfallFlavorText(`${front}?old`), "The front's story.\n—An author");
    assert.equal(await resolveScryfallFlavorText("https://example.test/custom.jpg"), "");
    assert.equal(await resolveScryfallFlavorText(HIDDEN_CARD_BACK_IMAGE_URL), "");
    assert.equal(calls, 1);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("custom art is stored but cannot bypass standard printing resolution", () => {
  installLocalStorageMock();
  setCustomCardArtUrls([
    { name: "Forge Test", artUrl: "https://example.test/art.jpg" },
  ]);

  assert.equal(customCardArtUrl("forge test"), "https://example.test/art.jpg");
  assert.equal(scryfallImageUrl("Forge Test", "art_crop"), "");
});

test("blank custom art removes an existing override", () => {
  installLocalStorageMock();
  setCustomCardArtUrls([
    { name: "Forge Test", artUrl: "https://example.test/art.jpg" },
  ]);
  setCustomCardArtUrls([
    { name: "Forge Test", artUrl: "" },
  ]);

  assert.equal(customCardArtUrl("Forge Test"), "");
  assert.equal(scryfallImageUrl("Forge Test", "art_crop"), "");
});

test("hidden card names use the local SVG cardback instead of Scryfall", () => {
  assert.equal(scryfallImageUrl("Hidden Card", "art_crop"), HIDDEN_CARD_BACK_IMAGE_URL);
  assert.equal(scryfallImageUrl("hidden card"), HIDDEN_CARD_BACK_IMAGE_URL);
  assert.match(HIDDEN_CARD_BACK_IMAGE_URL, /^data:image\/svg\+xml;charset=utf-8,/);
});

test("preloading resolves and caches Scryfall image URLs by card name", async () => {
  const originalFetch = globalThis.fetch;
  let calls = 0;
  globalThis.fetch = async (url, options) => {
    calls += 1;
    assert.match(String(url), /^http:\/\/localhost\/cards\/cache-test-card\.json$/);
    assert.equal(options?.cache, "no-cache");
    return {
      ok: true,
      json: async () => ({
        scryfall: {
          standard_printing: true,
          image_uris: {
            normal: "https://cards.example.test/cache-test-normal.jpg",
            art_crop: "https://cards.example.test/cache-test-art.jpg",
          },
          flavor_text: "A tale from this printing.",
        },
      }),
    };
  };

  try {
    await preloadCardArt(["Cache Test Card", "Cache Test Card"], {
      versions: ["normal"],
      concurrency: 2,
    });

    assert.equal(calls, 1);
    assert.equal(
      await resolveScryfallFlavorText("https://cards.example.test/cache-test-art.jpg"),
      "A tale from this printing."
    );
    assert.equal(
      scryfallImageUrl("Cache Test Card", "normal"),
      "https://cards.example.test/cache-test-normal.jpg"
    );
    assert.equal(
      scryfallImageUrl("Cache Test Card", "art_crop"),
      "https://cards.example.test/cache-test-art.jpg"
    );
    assert.equal(
      await resolveScryfallImageUrl("Cache Test Card", "normal"),
      "https://cards.example.test/cache-test-normal.jpg"
    );
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("Scryfall API fallback resolves CDN image URLs without using format=image", async () => {
  const originalFetch = globalThis.fetch;
  const urls = [];
  globalThis.fetch = async (url) => {
    urls.push(String(url));
    if (String(url).startsWith("http://localhost/cards/api-fallback-card.json")) {
      return { status: 404, ok: false, json: async () => ({}) };
    }
    assert.match(String(url), /^https:\/\/api\.scryfall\.com\/cards\/search\?/);
    assert.doesNotMatch(String(url), /format=image/);
    assert.match(String(url), /-is%3Afullart/);
    return {
      ok: true,
      json: async () => ({
        data: [
          {
            name: "API Fallback Card",
            image_uris: {
              normal: "https://cards.example.test/api-fallback-normal.jpg",
              art_crop: "https://cards.example.test/api-fallback-art.jpg",
            },
          },
        ],
      }),
      headers: { get: () => null },
    };
  };

  try {
    assert.equal(
      await resolveScryfallImageUrl("API Fallback Card", "normal"),
      "https://cards.example.test/api-fallback-normal.jpg"
    );
    assert.equal(urls.length, 2);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("full-art local metadata is skipped for default Scryfall art", async () => {
  const originalFetch = globalThis.fetch;
  const urls = [];
  globalThis.fetch = async (url) => {
    urls.push(String(url));
    if (String(url).startsWith("http://localhost/cards/full-art-local-card.json")) {
      return {
        ok: true,
        json: async () => ({
          scryfall: {
            full_art: true,
            image_uris: {
              normal: "https://cards.example.test/full-art-local-normal.jpg",
            },
          },
        }),
      };
    }
    assert.match(String(url), /^https:\/\/api\.scryfall\.com\/cards\/search\?/);
    assert.match(String(url), /-is%3Afullart/);
    return {
      ok: true,
      json: async () => ({
        data: [
          {
            name: "Full Art Local Card",
            full_art: false,
            image_uris: {
              normal: "https://cards.example.test/non-full-art-normal.jpg",
            },
          },
        ],
      }),
      headers: { get: () => null },
    };
  };

  try {
    assert.equal(
      await resolveScryfallImageUrl("Full Art Local Card", "normal"),
      "https://cards.example.test/non-full-art-normal.jpg"
    );
    assert.equal(urls.length, 2);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

for (const [label, treatment] of Object.entries({
  borderless: { border_color: "borderless" },
  showcase: { frame_effects: ["showcase"] },
  extended: { frame_effects: ["extendedart"] },
  textless: { textless: true },
  legacy: {},
})) {
  test(`${label} local art cannot leak into the cache through metadata lookup`, async () => {
    const originalFetch = globalThis.fetch;
    const name = `Policy ${label}`;
    globalThis.fetch = async url => {
      if (String(url).startsWith("http://localhost/")) return {
        ok: true, json: async () => ({ scryfall: {
          ...treatment, image_uris: { normal: "bad-local" },
        } }),
      };
      assert.match(new URL(url).searchParams.get("q"), /-border:borderless.*-frame:showcase.*-frame:extendedart.*-is:textless/);
      return { ok: true, json: async () => ({ data: [
        { name, full_art: true, image_uris: { normal: "bad-search" } },
        { name, image_uris: { normal: "standard" } },
      ] }) };
    };
    try {
      await fetchScryfallCardMeta(name);
      assert.equal(scryfallImageUrl(name), "");
      assert.equal(await resolveScryfallImageUrl(name), "standard");
      assert.equal(scryfallImageUrl(name), "standard");
    } finally { globalThis.fetch = originalFetch; }
  });
}

test("nonstandard exact preference searches other sets before falling back", async () => {
  installLocalStorageMock();
  const originalFetch = globalThis.fetch;
  const name = "Preferred Policy";
  setPreferredCardPrints([{ name, setCode: "tst", collectorNumber: "123" }]);
  const queries = [];
  globalThis.fetch = async url => {
    if (String(url).endsWith("/tst/123")) return { ok: true, json: async () => ({
      name, border_color: "borderless", image_uris: { normal: "bad-preference" },
    }) };
    const q = new URL(url).searchParams.get("q");
    queries.push(q);
    if (q.includes("set:tst")) return { ok: false, status: 404 };
    return { ok: true, json: async () => ({ data: [{ name, image_uris: { normal: "other-set" } }] }) };
  };
  try {
    assert.equal(await resolveScryfallImageUrl(name), "other-set");
    assert.equal(queries.length, 2);
    assert.equal(scryfallImageUrl(name), "other-set");
  } finally { globalThis.fetch = originalFetch; }
});

for (const status of [404, 503]) {
  test(`special art fallback ${status === 404 ? "accepts an empty search" : "rejects a search failure"}`, async () => {
    const originalFetch = globalThis.fetch;
    const name = `Fallback Policy ${status}`;
    let namedCalls = 0;
    globalThis.fetch = async url => {
      if (String(url).startsWith("http://localhost/")) return { ok: false, status: 404 };
      if (String(url).includes("/search?")) return { ok: false, status };
      namedCalls++;
      return { ok: true, json: async () => ({ name, full_art: true, image_uris: { normal: "only-print" } }) };
    };
    try {
      if (status === 404) assert.equal(await resolveScryfallImageUrl(name), "only-print");
      else await assert.rejects(resolveScryfallImageUrl(name), /Standard printing search failed/);
      assert.equal(namedCalls, status === 404 ? 1 : 0);
    } finally { globalThis.fetch = originalFetch; }
  });
}

test("localized image selection filters treatments independently of translated text", async () => {
  const originalFetch = globalThis.fetch;
  const name = "Localized Policy";
  globalThis.fetch = async url => {
    const q = new URL(url).searchParams.get("q");
    if (q.includes("lang:es")) {
      assert.match(q, /-is:fullart/);
      return { ok: true, json: async () => ({ data: [
        { border_color: "borderless", image_uris: { normal: "bad-localized" } },
        { image_uris: { normal: "standard-localized" } },
      ] }) };
    }
    return { ok: true, json: async () => ({ data: [{ name, oracle_id: "oracle-test", image_uris: { normal: "english" } }] }) };
  };
  try {
    assert.equal(await resolveScryfallLocalizedImageUrl(name, "es"), "standard-localized");
  } finally { globalThis.fetch = originalFetch; }
});

test("custom art is used only after standard alternatives have been exhausted", async () => {
  installLocalStorageMock();
  const originalFetch = globalThis.fetch;
  for (const available of [true, false]) {
    const name = `Custom Policy ${available}`;
    setCustomCardArtUrls([{ name, artUrl: "custom" }]);
    globalThis.fetch = async url => {
      if (String(url).startsWith("http://localhost/")) return { ok: false, status: 404 };
      if (String(url).includes("/search?")) return available
        ? { ok: true, json: async () => ({ data: [{ name, image_uris: { normal: "standard" } }] }) }
        : { ok: false, status: 404 };
      return { ok: true, json: async () => ({ name, full_art: true, image_uris: { normal: "special" } }) };
    };
    try {
      assert.equal(await resolveScryfallImageUrl(name), available ? "standard" : "custom");
      assert.equal(scryfallImageUrl(name), available ? "standard" : "custom");
    } finally { globalThis.fetch = originalFetch; }
  }
});

test("a prepared card's spell face never hijacks the real card's printing", async () => {
  installLocalStorageMock();
  const originalFetch = globalThis.fetch;
  const name = "Raise Dead";
  // Scryfall's exact-name search matches face names too, and release order
  // floats the newest "prepare" card above every printing of the real spell.
  const prepared = {
    name: "Cheerful Osteomancer // Raise Dead",
    layout: "prepare",
    oracle_id: "prepared-oracle",
    image_uris: { normal: "prepared-scan" },
    card_faces: [{ name: "Cheerful Osteomancer" }, { name: "Raise Dead" }],
  };
  const real = { name, oracle_id: "raise-dead-oracle", image_uris: { normal: "raise-dead-scan" } };
  globalThis.fetch = async url => {
    if (String(url).startsWith("http://localhost/")) return { ok: false, status: 404 };
    const q = new URL(url).searchParams.get("q") || "";
    if (q.includes("lang:es")) {
      assert.match(q, /oracleid:raise-dead-oracle/);
      return { ok: true, json: async () => ({ data: [{ image_uris: { normal: "raise-dead-es-scan" } }] }) };
    }
    return { ok: true, json: async () => ({ data: [prepared, real] }) };
  };
  try {
    assert.equal(await resolveScryfallImageUrl(name), "raise-dead-scan");
    assert.equal(await resolveScryfallLocalizedImageUrl(name, "es"), "raise-dead-es-scan");
    // The prepared card still resolves under its own name.
    assert.equal(await resolveScryfallImageUrl("Cheerful Osteomancer"), "prepared-scan");
  } finally { globalThis.fetch = originalFetch; }
});
