const MAINBOARD_MINIMUM = 60;
const COMMANDER_MAINBOARD_SIZE = 99;
const SIDEBOARD_MAXIMUM = 15;

export class DeckCatalogImportError extends Error {
  constructor(errors) {
    super(`Deck cannot be imported: ${errors.join("; ")}`);
    this.name = "DeckCatalogImportError";
    this.errors = errors;
  }
}

function text(value) {
  return String(value ?? "").trim();
}

function normalizeCards(cards) {
  if (!Array.isArray(cards)) return [];
  return cards
    .map((card) => ({ name: text(card?.name), count: Number(card?.count) }))
    .filter((card) => card.name && Number.isInteger(card.count) && card.count > 0);
}

function cardLines(cards) {
  return cards.map((card) => `${card.count} ${card.name}`);
}

export function deckCatalogEntryToLobbyTexts(entry) {
  const mainboard = normalizeCards(entry?.mainboard);
  const sideboard = normalizeCards(entry?.sideboard);
  const commander = normalizeCards(entry?.commander);

  return {
    deckText: [
      "Deck",
      ...cardLines(mainboard),
      ...(sideboard.length ? ["", "Sideboard", ...cardLines(sideboard)] : []),
    ].join("\n"),
    commanderText: cardLines(commander).join("\n"),
  };
}

export function deckCatalogEntryToMtgoText(entry) {
  const mainboard = normalizeCards(entry?.mainboard);
  const sideboard = normalizeCards(entry?.sideboard);
  const commander = normalizeCards(entry?.commander);
  return [
    ...(commander.length ? ["Commander", ...cardLines(commander), ""] : []),
    ...cardLines(mainboard),
    ...(sideboard.length ? ["", "Sideboard", ...cardLines(sideboard)] : []),
  ].join("\n");
}

export function validateDeckCatalogEntry(entry, { format = entry?.format || "normal" } = {}) {
  const errors = [];
  const mainboard = normalizeCards(entry?.mainboard);
  const sideboard = normalizeCards(entry?.sideboard);
  const commander = normalizeCards(entry?.commander);
  const mainboardCount = mainboard.reduce((total, card) => total + card.count, 0);
  const sideboardCount = sideboard.reduce((total, card) => total + card.count, 0);
  const commanderCount = commander.reduce((total, card) => total + card.count, 0);
  const normalizedFormat = text(format).toLocaleLowerCase("en-US");

  if (!mainboard.length) errors.push("mainboard is empty");
  if (mainboard.some((card) => !Number.isInteger(card.count) || card.count < 1)) {
    errors.push("mainboard contains an invalid count");
  }
  if (sideboardCount > SIDEBOARD_MAXIMUM) errors.push("sideboard exceeds 15 cards");
  if (normalizedFormat === "commander") {
    if (mainboardCount !== COMMANDER_MAINBOARD_SIZE) errors.push("Commander mainboard must contain 99 cards");
    if (commanderCount < 1 || commanderCount > 2) errors.push("Commander must contain one or two commanders");
  } else if (mainboardCount < MAINBOARD_MINIMUM) {
    errors.push(`mainboard must contain at least ${MAINBOARD_MINIMUM} cards`);
  }

  return {
    valid: errors.length === 0,
    errors,
    counts: { mainboard: mainboardCount, sideboard: sideboardCount, commander: commanderCount },
  };
}

export function importDeckCatalogEntry(entry, options = {}) {
  const validation = validateDeckCatalogEntry(entry, options);
  if (!validation.valid) throw new DeckCatalogImportError(validation.errors);
  const deckName = text(entry?.name || entry?.archetype || "Deck sin nombre");
  return {
    ...deckCatalogEntryToLobbyTexts(entry),
    counts: validation.counts,
    deckId: text(entry?.id),
    deckName,
    archetype: text(entry?.archetype),
    sourceUrl: text(entry?.sourceUrl),
  };
}
