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
