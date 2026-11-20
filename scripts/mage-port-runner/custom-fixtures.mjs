import { ensureCardSourcesForNames } from "../wasm-test-harness.mjs";

const fixtureSourcesByGame = new WeakMap();

export function registerMageFixtureSource(game, fixture) {
  const { name, typeLine, manaCost, power, toughness, oracleText } = fixture;
  const lines = [];
  if (manaCost) lines.push(`Mana cost: ${manaCost}`);
  if (typeLine) lines.push(`Type: ${typeLine}`);
  if (power != null && toughness != null) lines.push(`Power/Toughness: ${power}/${toughness}`);
  if (oracleText) lines.push(oracleText);
  const block = lines.join("\n");
  let sources = fixtureSourcesByGame.get(game);
  if (!sources) {
    sources = new Map();
    fixtureSourcesByGame.set(game, sources);
  }
  const key = name.toLowerCase();
  if (sources.has(key)) {
    if (sources.get(key) !== block) throw new Error(`conflicting XMage fixture sources for ${name}`);
    return;
  }
  const registered = game.registerExternalCardSources({
    canonicalName: name,
    group: { kind: "single", name, block, score: 1 },
  });
  if (registered.loaded !== 1 || registered.failed.length) {
    throw new Error(`could not register XMage fixture ${name}: ${JSON.stringify(registered.failed)}`);
  }
  sources.set(key, block);
  // Arena conjure creates a named catalog card at resolution; the engine reads
  // that card's definition from the game's registered catalog, so register it
  // with the fixture that names it (as a host preloads a deck's dependencies).
  const conjured = [...String(oracleText || "").matchAll(
    /\bconjures? (?:a|an|one|two|three|four|five|six|seven|x|\d+) (.+?) cards? (?:into|onto|on)\b/gi,
  )].map(match => match[1]);
  if (conjured.length) ensureCardSourcesForNames(game, conjured);
}
