// Re-express a decision prompt in the player's language.
//
// The engine phrases prompts by quoting the source card's own compiled text
// (`runtime_display::effect_sentences`), so a prompt is a verbatim slice of a
// string the snapshot already carries. That makes translation a lookup rather
// than a translation: find which printed sentence the prompt came from, then
// take the same sentence from the card's official localized text, which the
// card-i18n buckets already provide for the inspector.
//
// Everything here is positional and refuses to guess. When the localized text
// does not line up sentence for sentence, the caller keeps the English prompt.

// Card text a player reads is one ability per line.
export function splitPrintedLines(text) {
  return String(text || "")
    .replace(/\\n/g, "\n")
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

// Mirrors the engine's split: a terminator ends a sentence only when the line
// ends or whitespace follows, so "2." inside a cost or "Ivy, Gleeful" stay put.
export function splitPrintedSentences(line) {
  const sentences = [];
  const text = String(line || "");
  let current = "";
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    current += char;
    if (".!?".includes(char)) {
      const next = text[index + 1];
      if (next === undefined || /\s/.test(next)) {
        if (current.trim()) sentences.push(current.trim());
        current = "";
      }
    }
  }
  if (current.trim()) sentences.push(current.trim());
  return sentences;
}

// Official printings carry reminder text the engine's compiled text omits, and
// a reminder can be a whole sentence of its own. Dropping it on both sides is
// what keeps "Vuela.\nSiempre que… (Una copia… se convierte en una ficha.)"
// aligned with the two sentences the engine compiled.
function withoutReminder(sentence) {
  // A reminder's own full stop sits inside the parentheses, so the split above
  // leaves it attached to whichever sentence it neighbours — strip from both
  // ends, and repeat for a line that stacks two of them.
  let stripped = String(sentence || "").trim();
  for (;;) {
    const next = stripped
      .replace(/^\([^()]*\)\s*/u, "")
      .replace(/\s*\([^()]*\)$/u, "")
      .trim();
    if (next === stripped) return stripped;
    stripped = next;
  }
}

// The sentences of one printed line, reminder text removed.
export function printedSentences(line) {
  return splitPrintedSentences(line).map(withoutReminder).filter(Boolean);
}

// The word that introduces an optional action, per locale. The engine strips
// the English one when it phrases a "may" prompt, so the localized sentence
// has to lose its own equivalent to stay parallel.
const OPTIONAL_MARKERS = {
  en: /\byou\s+may\b|\bmay\b/iu,
  es: /\bpuedes\b|\bpuedas\b|\bpuede\b|\bpueden\b|\bpodr[áa]s\b|\bpod[ée]is\b/iu,
};

function normalize(text) {
  return String(text || "")
    .normalize("NFKC")
    .replace(/\s+/g, " ")
    .replace(/[.!?]+$/, "")
    .trim()
    .toLowerCase();
}

function capitalizeFirst(text) {
  const value = String(text || "");
  return value ? value[0].toUpperCase() + value.slice(1) : value;
}

function stripTerminator(text) {
  return String(text || "").replace(/[.!?]+$/, "").trim();
}

// Drop everything up to and including the locale's optional-action marker, the
// way the engine drops "you may". Without a marker the whole sentence stands —
// still the right sentence, just with its lead-in.
function optionalClause(sentence, locale) {
  const marker = OPTIONAL_MARKERS[locale];
  if (!marker) return sentence;
  const match = marker.exec(sentence);
  if (!match) return sentence;
  return sentence.slice(match.index + match[0].length).trim() || sentence;
}

// Where a prompt sentence sits in the printed text: an exact sentence, or the
// tail of one whose lead-in ("…, you may") the engine removed.
function locateSentence(sentence, lines, { allowSuffix }) {
  const needle = normalize(sentence);
  if (!needle) return null;
  for (let line = 0; line < lines.length; line += 1) {
    const sentences = printedSentences(lines[line]);
    for (let index = 0; index < sentences.length; index += 1) {
      const candidate = normalize(sentences[index]);
      if (candidate === needle) return { line, index, suffix: false };
      if (allowSuffix && candidate.endsWith(needle)) return { line, index, suffix: true };
    }
  }
  return null;
}

// The activation cost clause of a printed line — "Pay 1 life, Sacrifice another
// creature" out of "Pay 1 life, Sacrifice another creature: Put a -1/-1
// counter…". Costs are comma-separated and keep their order in every printing,
// so a cost prompt maps the same way a sentence does.
export function costSegments(line) {
  const text = String(line || "");
  const colon = text.indexOf(":");
  if (colon <= 0) return [];
  return text
    .slice(0, colon)
    .split(",")
    .map((segment) => withoutReminder(segment))
    .filter(Boolean);
}

// A prompt that names one cost of one printed ability, or null when the cost
// appears in more than one ability and the prompt cannot say which.
function locateCostSegment(prompt, lines) {
  const needle = normalize(prompt);
  if (!needle) return null;
  let found = null;
  for (let line = 0; line < lines.length; line += 1) {
    const segments = costSegments(lines[line]);
    for (let index = 0; index < segments.length; index += 1) {
      if (normalize(segments[index]) !== needle) continue;
      if (found) return null;
      found = { line, index };
    }
  }
  return found;
}

/**
 * Translate one engine prompt by position, or return null to keep the English.
 *
 * `englishText` is the source object's compiled card text as the snapshot
 * carries it; `translatedText` is the same card's localized printed text.
 */
export function translatePrintedPrompt({ prompt, englishText, translatedText, locale }) {
  if (!locale || locale === "en") return null;
  const promptSentences = printedSentences(String(prompt || "").trim());
  if (promptSentences.length === 0) return null;

  const englishLines = splitPrintedLines(englishText);
  const translatedLines = splitPrintedLines(translatedText);
  if (englishLines.length === 0 || englishLines.length !== translatedLines.length) return null;

  // The context line under a prompt is the source's whole text box, which the
  // engine joins with "; ". That is not a sentence, so match it as the card.
  const wholeText = normalize(String(prompt || ""));
  for (const separator of ["; ", " "]) {
    if (wholeText === normalize(englishLines.join(separator))) {
      return translatedLines.join(separator);
    }
  }

  const located = promptSentences.map((sentence, position) =>
    locateSentence(sentence, englishLines, { allowSuffix: position === 0 })
  );
  if (located.some((match) => match === null)) {
    // Not a sentence: a cost prompt names one component of a cost clause.
    const cost = locateCostSegment(prompt, englishLines);
    if (!cost) return null;
    const translatedCosts = costSegments(translatedLines[cost.line]);
    if (translatedCosts.length !== costSegments(englishLines[cost.line]).length) return null;
    return capitalizeFirst(stripTerminator(translatedCosts[cost.index]));
  }

  // A prompt quotes one ability, in order. Anything else means the match was
  // coincidental and the English text is the honest thing to show.
  const line = located[0].line;
  const inOrder = located.every(
    (match, position) =>
      match.line === line && (position === 0 || match.index > located[position - 1].index)
  );
  if (!inOrder) return null;

  const englishSentences = printedSentences(englishLines[line]);
  const translatedSentences = printedSentences(translatedLines[line]);
  if (englishSentences.length !== translatedSentences.length) return null;

  const parts = located.map((match) => {
    const sentence = translatedSentences[match.index];
    return stripTerminator(match.suffix ? optionalClause(sentence, locale) : sentence);
  });
  if (parts.some((part) => !part)) return null;

  return capitalizeFirst(parts.join(". "));
}
