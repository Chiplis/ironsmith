import test from "node:test";
import assert from "node:assert/strict";
import {
  HIDDEN_CARD_BACK_IMAGE_URL,
  customCardArtUrl,
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
  assert.match(scryfallImageUrl("Forge Test", "art_crop"), /^https:\/\/api\.scryfall\.com\/cards\/named\?/);
});

test("hidden card names use the local SVG cardback instead of Scryfall", () => {
  assert.equal(scryfallImageUrl("Hidden Card", "art_crop"), HIDDEN_CARD_BACK_IMAGE_URL);
  assert.equal(scryfallImageUrl("hidden card"), HIDDEN_CARD_BACK_IMAGE_URL);
  assert.match(HIDDEN_CARD_BACK_IMAGE_URL, /^data:image\/svg\+xml;charset=utf-8,/);
});
