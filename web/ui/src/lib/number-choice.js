/** Empty, fractional, and out-of-range values must never become engine choices. */
export function parseNumberChoice(value, min = 0, max = 999) {
  if (String(value).trim() === "") return null;
  const number = Number(value);
  return Number.isSafeInteger(number) && number >= min && number <= max ? number : null;
}
