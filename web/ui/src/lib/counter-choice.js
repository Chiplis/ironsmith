const U32_MAX = 4294967295n;
const U64_MAX = 18446744073709551615n;
function natural(value, maximum) {
  if (typeof value !== "string" || !/^\d+$/.test(value)) return null;
  const number = BigInt(value);
  return number <= maximum ? number : null;
}

export function initialCounterDraft(decision) {
  let remaining = natural(decision.min_total, U64_MAX) ?? 0n;
  return (decision.options || []).map((option) => {
    const available = natural(String(option.max_count ?? 0), U32_MAX) ?? 0n;
    const count = option.legal === false ? 0n : (available < remaining ? available : remaining);
    remaining -= count;
    return { index: option.index, value: count.toString() };
  });
}

export function updateCounterDraft(entries, index, value) {
  const old = entries.find((entry) => entry.index === index);
  const next = { index, value };
  // Retain the order in which positive quantities were selected.
  if (old && !(/^\d+$/.test(old.value) && BigInt(old.value) > 0n) && /^\d+$/.test(value) && BigInt(value) > 0n) {
    return [...entries.filter((entry) => entry.index !== index), next];
  }
  return entries.map((entry) => entry.index === index ? next : entry);
}

export function parseCounterAllocationChoice(decision, entries) {
  const min = natural(decision.min_total, U64_MAX);
  const max = natural(decision.max_total, U64_MAX);
  if (min === null || max === null || min > max || !Array.isArray(entries)) return null;
  const options = new Map((decision.options || []).map((option) => [option.index, option]));
  const seen = new Set();
  const allocations = [];
  let total = 0n;
  for (const entry of entries) {
    if (!Number.isSafeInteger(entry.index) || entry.index < 0 || seen.has(entry.index)) return null;
    seen.add(entry.index);
    const option = options.get(entry.index);
    if (!option) return null;
    const count = natural(entry.value, U32_MAX);
    const available = natural(String(option.max_count ?? 0), U32_MAX);
    if (count === null || available === null || count > available || (option.legal === false && count > 0n)) return null;
    total += count;
    if (count > 0n) allocations.push({ index: entry.index, count: Number(count) });
  }
  if (total < min || total > max) return null;
  return { command: { type: "select_counters", allocations }, total: total.toString() };
}
