// Every asset loader resolves against the deployed Vite base: the site is
// served from a subdirectory, where root-absolute paths would miss. The guard
// keeps this usable from plain `node --test`, where import.meta.env is unset.
export function baseAssetUrl() {
  const configured = typeof import.meta !== "undefined"
    ? import.meta.env?.BASE_URL
    : null;
  const base = configured || "/";
  return new URL(base, globalThis?.location?.href || "http://localhost/").href;
}
