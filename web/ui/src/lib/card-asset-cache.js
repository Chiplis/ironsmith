import { RUNTIME_VERSION } from "./runtime-version.js";

// Card assets have stable, unhashed URLs and are served with max-age=0, so by
// default every read is a revalidation round trip. A production build knows
// the runtime version, which changes whenever the card assets do; stamping it
// into the URL makes the cached copy safe to reuse without asking the server.
// Development keeps revalidating, since cards can be rebuilt under a running
// dev server without changing its version.
const VERSIONED = typeof import.meta !== "undefined"
  && import.meta.env?.PROD === true
  && RUNTIME_VERSION !== "development-unversioned";

// Whether a fetched card asset is reused from cache without revalidation.
export const CARD_ASSETS_REUSABLE = VERSIONED;

export const CARD_ASSET_FETCH_OPTIONS = VERSIONED ? { cache: "force-cache" } : { cache: "no-cache" };

export function versionedCardAssetUrl(url) {
  if (!VERSIONED) return url;
  const versioned = new URL(url);
  versioned.searchParams.set("v", RUNTIME_VERSION.slice(0, 16));
  return versioned.href;
}

// Strict UTF-8 decode: a body that is not valid UTF-8 (a truncated or
// corrupted cache entry) is rejected instead of silently decoded with
// replacement characters.
const utf8Decoder = typeof TextDecoder === "function"
  ? new TextDecoder("utf-8", { fatal: true })
  : null;

// UTF-8 text that was decoded as Latin-1/Windows-1252 somewhere upstream
// (e.g. U+00C3 U+00A9 for U+00E9). A cached body like this is refetched.
const MOJIBAKE_PATTERN = /\u00C3[\u0080-\u00BF]|\u00E2\u20AC|\u00C2[\u00A0-\u00BF]/;

// Returned by fetchCardAssetJson only for a definitive 404.
export const CARD_ASSET_MISSING = Symbol("card-asset-missing");

async function readCardAssetJson(response, validate) {
  if (response.status === 404) return { missing: true };
  if (!response.ok) return { error: new Error(`HTTP ${response.status}`), http: true };
  let text;
  try {
    const bytes = await response.arrayBuffer();
    text = utf8Decoder ? utf8Decoder.decode(bytes) : await new Response(bytes).text();
  } catch (error) {
    return { error: new Error(`body is not valid UTF-8 (${error?.message || error})`) };
  }
  let payload;
  try {
    payload = JSON.parse(text);
  } catch (error) {
    const contentType = String(response.headers.get("content-type") || "").toLowerCase();
    return { error: new Error(`body is not JSON (content-type "${contentType}"): ${error?.message || error}`) };
  }
  if (validate && !validate(payload)) {
    return { error: new Error("body is not a valid card asset") };
  }
  return { payload, suspicious: MOJIBAKE_PATTERN.test(text) };
}

// Fetch and parse a card asset JSON body, always decoding it as UTF-8.
// Resolves to the payload, or CARD_ASSET_MISSING only for a definitive 404.
// A non-JSON body (SPA HTML fallback, captive portal), a truncated or corrupt
// cached body, invalid UTF-8, or a mojibake body is refetched once bypassing
// the HTTP cache (`cache: "reload"` also overwrites the bad cache entry). If
// that still fails this throws, so callers treat the failure as transient
// instead of caching a miss.
export async function fetchCardAssetJson(url, { validate = null, fetchImpl = fetch } = {}) {
  const first = await readCardAssetJson(await fetchImpl(url, CARD_ASSET_FETCH_OPTIONS), validate);
  if (first.missing) return CARD_ASSET_MISSING;
  if (first.payload !== undefined && !first.suspicious) return first.payload;
  const retry = await readCardAssetJson(await fetchImpl(url, { cache: "reload" }), validate);
  if (retry.missing) return CARD_ASSET_MISSING;
  if (retry.payload !== undefined) return retry.payload;
  // A valid-but-suspicious first body is better than a failed reload.
  if (first.payload !== undefined) return first.payload;
  const error = new Error(`Card asset fetch failed for ${url}: ${retry.error?.message || first.error?.message || "invalid body"}`);
  // Distinguishes "the server answered with something that is not this asset"
  // from an HTTP error status (network failures reject the fetch itself).
  error.cardAssetInvalidBody = !retry.http;
  throw error;
}
