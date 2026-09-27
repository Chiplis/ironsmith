// The UI reads the same catalogue as the engine through its worker. `undefined`
// means this runtime has no embedded catalogue; `null` means an unknown card in
// an embedded catalogue, which must not fall back to a potentially stale asset.
let activeCatalog = null;

function parseJson(value, label) {
  if (value == null) return null;
  const parsed = typeof value === "string" ? JSON.parse(value) : value;
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error(`Invalid embedded card ${label}`);
  }
  return parsed;
}

export function installEmbeddedCardCatalog({ ready, getIndexJson, getSourceJson }) {
  let rejectClosed;
  const closed = new Promise((_, reject) => { rejectClosed = reject; });
  // Cleanup may happen when nobody has requested a card yet.
  closed.catch(() => {});
  const catalog = {
    getSourceJson,
    wait: (operation) => Promise.race([Promise.resolve().then(operation), closed]),
    index: null,
  };
  catalog.index = catalog.wait(async () => {
    await ready;
    return parseJson(await getIndexJson(), "index") ?? undefined;
  });
  catalog.index.catch(() => {});
  activeCatalog = catalog;
  return (error = new Error("Embedded card catalogue worker is unavailable")) => {
    if (activeCatalog === catalog) activeCatalog = null;
    rejectClosed(error);
  };
}

export async function readEmbeddedCardCatalogIndex() {
  const catalog = activeCatalog;
  return catalog ? catalog.wait(() => catalog.index) : undefined;
}

export async function readEmbeddedCardSource(route) {
  const catalog = activeCatalog;
  if (!catalog) return undefined;
  return catalog.wait(async () => {
    if (await catalog.index === undefined) return undefined;
    return parseJson(await catalog.getSourceJson(String(route || "")), "source");
  });
}
