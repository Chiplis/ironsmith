function configuredAssetBaseUrl() {
  return typeof import.meta !== "undefined" ? import.meta.env?.BASE_URL : null;
}

function runtimeAssetBaseUrl(baseUrl = configuredAssetBaseUrl()) {
  const configured = String(baseUrl || "/");
  const locationHref = globalThis?.location?.href || "http://localhost/";
  return new URL(configured, locationHref).href;
}

/** Resolve a generated frontend asset without escaping the deployed base path. */
export function resolveAssetUrl(path, { baseUrl } = {}) {
  const relativePath = String(path || "").replace(/^\/+/, "");
  return new URL(relativePath, runtimeAssetBaseUrl(baseUrl)).href;
}

export function resolveCardAssetUrl(route, options) {
  const normalizedRoute = String(route || "").replace(/^\/+/, "");
  return resolveAssetUrl(`cards/${normalizedRoute}.json`, options);
}
