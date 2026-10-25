// A card that has its owner circle colors while building the deck ("As you
// create your deck, circle two of the colors below.", Cryptic Spires) records
// the choice in the decklist, after the card's name: "Cryptic Spires ({W}{B})".
// The note travels with the card name through the lobby and hidden-deck
// commitments; the engine splits it off and remembers the colors for that
// physical card. Mirrors `split_deck_construction_color_note` and
// `deck_construction_color_note_error` in the engine.

const COLOR_NOTE = /\s*\(\s*((?:\{[WUBRG]\}\s*)+)\)\s*$/i;
const COLOR_SYMBOL = /\{([WUBRG])\}/gi;
const EXAMPLE_SYMBOLS = ["{W}", "{U}", "{B}", "{R}", "{G}"];
const NUMBER_WORDS = { one: 1, two: 2, three: 3, four: 4, five: 5 };

/** `"Cryptic Spires ({W}{B})"` → `{ name: "Cryptic Spires", colors: ["W", "B"] }`. */
export function splitDeckColorNote(written) {
  const text = String(written ?? "").trim();
  const match = text.match(COLOR_NOTE);
  if (!match) return { name: text, colors: null };
  const name = text.slice(0, match.index).trim();
  if (!name) return { name: text, colors: null };
  const colors = [...match[1].matchAll(COLOR_SYMBOL)].map((symbol) => symbol[1].toUpperCase());
  return { name, colors };
}

/** The card a decklist entry names, without its circled-color note. */
export function deckCardBaseName(written) {
  return splitDeckColorNote(written).name;
}

/** How many colors a card's rules text has its owner circle, or null. */
export function circledColorCount(rulesText) {
  const match = String(rulesText ?? "").match(
    /as you create your deck,\s+(?:circle|choose)\s+(\w+)\s+of the colors/i,
  );
  if (!match) return null;
  const word = match[1].toLowerCase();
  return NUMBER_WORDS[word] ?? (Number.parseInt(word, 10) || null);
}

/**
 * Why a decklist entry's note does not fit its card, or null. A card that
 * circles N colors needs exactly N different colors; no other card takes one.
 */
export function deckColorNoteProblem(cardName, rulesText, written) {
  const required = circledColorCount(rulesText);
  const { colors } = splitDeckColorNote(written);
  const example = `${cardName} (${EXAMPLE_SYMBOLS.slice(0, required || 2).join("")})`;
  if (required == null) {
    return colors
      ? `${cardName} doesn't choose colors as you build your deck; remove the colors after its name`
      : null;
  }
  if (!colors) {
    return `${cardName} needs the ${required} colors circled for it, written after its name, e.g. ${example}`;
  }
  if (colors.length !== required || new Set(colors).size !== required) {
    return `${cardName} needs exactly ${required} different colors after its name, e.g. ${example}`;
  }
  return null;
}

/**
 * The engine's verdict on every circled-color note in a submission (main
 * deck, sideboard, commanders), as `[{ cardName, error }]`. The engine knows
 * which cards circle colors; an engine without the check reports nothing.
 */
export async function submissionColorNoteIssues(game, submission) {
  if (!game || typeof game.deckColorNoteIssues !== "function") return [];
  const names = [
    ...(submission?.deck || []),
    ...(submission?.sideboard || []),
    ...(submission?.commanders || []),
  ].map((name) => String(name || "").trim()).filter(Boolean);
  if (!names.length) return [];
  const issues = await game.deckColorNoteIssues([...new Set(names)]);
  return Array.isArray(issues) ? issues : [];
}

/** One status line for a list of note issues. */
export function describeColorNoteIssues(issues) {
  const errors = [...new Set((issues || []).map((issue) => issue?.error).filter(Boolean))];
  if (!errors.length) return "";
  const shown = errors.slice(0, 2).join(". ");
  return errors.length > 2 ? `${shown}. (+${errors.length - 2} more)` : `${shown}.`;
}
