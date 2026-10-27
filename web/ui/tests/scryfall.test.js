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
  setCompiledCardNames,
  isCompiledCardName,
  preferredCardPrint,
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

test("hidden card names use the bundled Magic card back instead of a card lookup", () => {
  assert.equal(scryfallImageUrl("Hidden Card", "art_crop"), HIDDEN_CARD_BACK_IMAGE_URL);
  assert.equal(scryfallImageUrl("hidden card"), HIDDEN_CARD_BACK_IMAGE_URL);
  assert.match(HIDDEN_CARD_BACK_IMAGE_URL, /\/assets\/mtg-card-back\.jpg$/);
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
          frame: "2015",
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

test("generated local metadata supplies hand art before any Scryfall request", async () => {
  const originalFetch = globalThis.fetch;
  const name = "Generated Local Hand Card";
  const urls = [];
  globalThis.fetch = async (url) => {
    urls.push(String(url));
    assert.equal(String(url), "http://localhost/cards/generated-local-hand-card.json");
    return {
      ok: true,
      json: async () => ({
        scryfall: {
          standard_printing: true,
          frame: "2015",
          image_uris: {
            normal: "https://cards.example.test/generated-local-hand-card.jpg",
          },
        },
      }),
    };
  };

  try {
    assert.equal(
      await resolveScryfallImageUrl(name, "normal"),
      "https://cards.example.test/generated-local-hand-card.jpg",
    );
    assert.deepEqual(urls, ["http://localhost/cards/generated-local-hand-card.json"]);
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
  retro: { standard_printing: true, frame: "1997" },
  original: { standard_printing: true, frame: "1993" },
  unverifiedFrame: { standard_printing: true },
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
        { name, frame: "1997", image_uris: { normal: "retro-search" } },
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
        { frame: "1997", image_uris: { normal: "retro-localized" } },
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

test("retro printing remains available when no standard alternative exists", async () => {
  const originalFetch = globalThis.fetch;
  const name = "Retro Only Policy";
  let searched = false;
  globalThis.fetch = async url => {
    if (String(url).startsWith("http://localhost/")) return { ok: false, status: 404 };
    if (String(url).includes("/search?")) {
      assert.match(new URL(url).searchParams.get("q"), /-frame:1993 -frame:1997/);
      searched = true;
      return { ok: false, status: 404 };
    }
    assert.ok(searched);
    return { ok: true, json: async () => ({ name, frame: "1993", image_uris: { normal: "only-retro" } }) };
  };
  try {
    assert.equal(await resolveScryfallImageUrl(name), "only-retro");
  } finally { globalThis.fetch = originalFetch; }
});

test("404 API resources are not downloaded again on later card lookups", async () => {
  const originalFetch = globalThis.fetch;
  const requests = new Map();
  const name = "Missing Resource Regression Card";
  globalThis.fetch = async url => {
    if (String(url).startsWith("https://api.scryfall.com/")) {
      requests.set(String(url), (requests.get(String(url)) || 0) + 1);
    }
    return new Response(null, { status: 404 });
  };
  try {
    await assert.rejects(resolveScryfallImageUrl(name));
    const firstCount = requests.size;
    assert.ok(firstCount > 0);
    await Promise.all([
      assert.rejects(resolveScryfallImageUrl(name)),
      assert.rejects(resolveScryfallImageUrl(name, "art_crop")),
    ]);
    assert.equal(requests.size, firstCount);
    assert.ok([...requests.values()].every(count => count === 1));
  } finally { globalThis.fetch = originalFetch; }
});

test("token searches include extras and retain the matching face artwork", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async url => {
    if (!String(url).startsWith('https://api.scryfall.com/')) return new Response(null,{status:404});
    const params = new URL(url).searchParams;
    assert.equal(params.get('include_extras'),'true');
    return {ok:true,json:async()=>({data:[{name:'Goblin // Treasure',layout:'double_faced_token',frame:'2015',
      card_faces:[{name:'Goblin',image_uris:{normal:'goblin-normal',art_crop:'goblin-art'}},
        {name:'Treasure',image_uris:{normal:'treasure-normal',art_crop:'treasure-art'}}]}]})};
  };
  try {
    assert.equal(await resolveScryfallImageUrl('Treasure','art_crop'),'treasure-art');
    assert.equal(await resolveScryfallImageUrl('Treasure','normal'),'treasure-normal');
  } finally { globalThis.fetch=originalFetch; }
});

test("full-art tokens resolve when standard and named lookups miss", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async url => {
    if (!String(url).startsWith('https://api.scryfall.com/')) return new Response(null,{status:404});
    const params = new URL(url).searchParams;
    if (!(params.get('q')||'').includes('is:token')) return new Response(null,{status:404});
    assert.equal(params.get('include_extras'),'true');
    return {ok:true,json:async()=>({data:[{name:'Servo',layout:'token',full_art:true,image_uris:{normal:'servo-normal',art_crop:'servo-art'}}]})};
  };
  try { assert.equal(await resolveScryfallImageUrl('Servo','art_crop'),'servo-art'); }
  finally { globalThis.fetch=originalFetch; }
});

test("engine token suffix resolves token-only artwork and caches both image sizes", async () => {
  const originalFetch = globalThis.fetch;
  const requests = [];
  globalThis.fetch = async url => {
    const parsed = new URL(url, 'http://localhost');
    requests.push(String(url));
    if (parsed.searchParams.get('q') !== '!"Monk" is:token') return new Response(null, {status:404});
    assert.equal(parsed.searchParams.get('include_extras'), 'true');
    return {ok:true,json:async()=>({data:[
      {name:'Monk',layout:'normal',image_uris:{normal:'wrong-card',art_crop:'wrong-art'}},
      {name:'Monk',layout:'token',full_art:true,image_uris:{normal:'monk-scan',art_crop:'monk-art'}},
    ]})};
  };
  try {
    assert.equal(await resolveScryfallImageUrl('Monk Token', 'art_crop'), 'monk-art');
    const count = requests.length;
    assert.equal(await resolveScryfallImageUrl('Monk Token'), 'monk-scan');
    assert.equal(scryfallImageUrl('Monk Token', 'art_crop'), 'monk-art');
    assert.equal(requests.length, count);
  } finally { globalThis.fetch = originalFetch; }
});

test("a suffixed double-faced token uses the matching face rather than the first", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async url => {
    if (new URL(url, 'http://localhost').searchParams.get('q') !== '!"Phyrexian" is:token') return new Response(null, {status:404});
    return {ok:true,json:async()=>({data:[{name:'Incubator // Phyrexian',layout:'double_faced_token',
      card_faces:[{name:'Incubator',image_uris:{normal:'incubator-scan',art_crop:'incubator-art'}},
        {name:'Phyrexian',image_uris:{normal:'phyrexian-scan',art_crop:'phyrexian-art'}}]}]})};
  };
  try {
    assert.equal(await resolveScryfallImageUrl('Phyrexian Token','art_crop'), 'phyrexian-art');
    assert.equal(await resolveScryfallImageUrl('Phyrexian Token'), 'phyrexian-scan');
  } finally { globalThis.fetch = originalFetch; }
});


test("full storage retains compiled names, art and print preferences for the session", () => {
  const previous = globalThis.localStorage;
  const persisted = new Map([["ironsmith-compiled-card-names", JSON.stringify({ "previous custom card": "compiled" })]]);
  let writable = false;
  globalThis.localStorage = {
    getItem: key => persisted.get(key) ?? null,
    setItem(key, value) {
      if (!writable) throw new DOMException("Storage full", "QuotaExceededError");
      persisted.set(key, value);
    },
    removeItem(key) {
      if (!writable) throw new DOMException("Storage blocked", "SecurityError");
      persisted.delete(key);
    },
  };
  try {
    assert.doesNotThrow(() => setCompiledCardNames(["IronSmith Sandboxer"]));
    assert.equal(isCompiledCardName("IronSmith Sandboxer"), true);
    setCompiledCardNames(["Another sandbox card"]);
    assert.equal(isCompiledCardName("IronSmith Sandboxer"), true);
    assert.equal(isCompiledCardName("Previous Custom Card"), true);
    assert.equal(isCompiledCardName("Another sandbox card"), true);
    setCustomCardArtUrls([{ name: "Quota art", artUrl: "https://example.test/quota.jpg" }]);
    assert.equal(customCardArtUrl("Quota art"), "https://example.test/quota.jpg");
    setCustomCardArtUrls([{ name: "Quota art", artUrl: "" }]);
    assert.equal(customCardArtUrl("Quota art"), "");
    setPreferredCardPrints([{ name: "Quota print", setCode: "fdn" }]);
    assert.deepEqual(preferredCardPrint("Quota print"), { setCode: "fdn" });
    writable = true;
    setCompiledCardNames(["Persisted after recovery"]);
    const names = JSON.parse(persisted.get("ironsmith-compiled-card-names"));
    assert.ok(names["ironsmith sandboxer"] && names["another sandbox card"] && names["persisted after recovery"]);
  } finally { globalThis.localStorage = previous; }
});

test("compiled names and custom art work with storage unavailable", () => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  Object.defineProperty(globalThis, "localStorage", { configurable: true, get() { throw new DOMException("Disabled", "SecurityError"); } });
  try {
    setCompiledCardNames(["Private sandbox card"]);
    assert.equal(isCompiledCardName("Private sandbox card"), true);
    setCustomCardArtUrls([{ name: "Private sandbox card", artUrl: "https://example.test/private.jpg" }]);
    assert.equal(customCardArtUrl("Private sandbox card"), "https://example.test/private.jpg");
  } finally {
    if (descriptor) Object.defineProperty(globalThis, "localStorage", descriptor);
    else delete globalThis.localStorage;
  }
});
