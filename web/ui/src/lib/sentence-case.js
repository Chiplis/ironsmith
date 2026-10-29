// Card text from the engine (and some translations) can continue a sentence in
// lowercase after a period. Display-only: capitalize the first letter that
// follows a period and whitespace (spaces or line breaks). Closing reminder
// parens/quotes may sit between the period and the space, and opening ones
// between the space and the letter. Requiring whitespace keeps decimals and
// URLs untouched. Single-character case mapping only, so string length and
// keyword-helper offsets are preserved.
const SENTENCE_START_RE = /(\.[)\]"'”’»]*\s+[(\["'“‘«¡¿]*)(\p{Ll})/gu;

export function capitalizeSentences(text) {
  if (typeof text !== "string" || !text.includes(".")) return text;
  return text.replace(SENTENCE_START_RE, (match, lead, letter) => {
    const upper = letter.toUpperCase();
    return upper.length === letter.length ? lead + upper : match;
  });
}
