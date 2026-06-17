import test from "node:test";
import assert from "node:assert/strict";
import {
  HIDDEN_CARD_BACK_IMAGE_URL,
  customCardArtUrl,
  preloadCardArt,
  resolveScryfallImageUrl,
  scryfallImageUrl,
  setCustomCardArtUrls,
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

test("custom card art overrides Scryfall image lookup by card name", () => {
  installLocalStorageMock();
  setCustomCardArtUrls([
    { name: "Forge Test", artUrl: "https://example.test/art.jpg" },
  ]);

  assert.equal(customCardArtUrl("forge test"), "https://example.test/art.jpg");
  assert.equal(scryfallImageUrl("Forge Test", "art_crop"), "https://example.test/art.jpg");
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
  globalThis.fetch = async (url) => {
    calls += 1;
    assert.match(String(url), /^http:\/\/localhost\/cards\/cache-test-card\.json$/);
    return {
      ok: true,
      json: async () => ({
        scryfall: {
          image_uris: {
            normal: "https://cards.example.test/cache-test-normal.jpg",
            art_crop: "https://cards.example.test/cache-test-art.jpg",
          },
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
