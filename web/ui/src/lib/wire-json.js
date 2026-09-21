// Compare/hash the JSON representation that actually crosses the transport.
// In particular, undefined object fields disappear, while undefined array
// entries become null and retain their positions. Sorting keys must not
// replace those JSON rules with a hand-written recursive serializer.
export function canonicalWireJson(value) {
  return JSON.stringify(value, (_key, item) =>
    item && typeof item === "object" && !Array.isArray(item)
      ? Object.fromEntries(Object.keys(item).sort().map(key => [key, item[key]]))
      : item);
}
