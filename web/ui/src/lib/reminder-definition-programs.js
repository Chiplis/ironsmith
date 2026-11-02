// Finite independent static clauses in a custom definition remain separate
// ability programs. Quoted child programs and ordinary multi-sentence effects
// are opaque; this reader does not split at arbitrary sentence boundaries.
export function independentAbilityPrograms(value) {
  const text = String(value).trim();
  const programs = [];
  let start = 0, quote = null, brackets = 0;
  for (let index = 0; index < text.length; index++) {
    const char = text[index];
    if (quote) { if (char === quote || (quote === '“' && char === '”')) quote = null; continue; }
    if (char === '"' || char === '“') { quote = char; continue; }
    if (char === '[') { brackets++; continue; }
    if (char === ']') { brackets--; continue; }
    if (char !== '.' || brackets) continue;
    const tail = text.slice(index + 1);
    // This complete native static form has no imperative subject. Its
    // declaration is a sibling of the preceding activated/triggered program.
    const subtype = /^\s+This (?:card|creature|permanent|spell) is (?:a|an) [\p{L}][\p{L}\p{N} '’‘−-]* in addition to its other types\.(?:\s|$)/iu.test(tail);
    // A bounded reveal offer ends at its own closing bracket. A following
    // ordinary trigger is a sibling, never another part of the offered body.
    const reveal = /^As you reveal this card from a hidden zone\b[^:]*as follows:\s*\[/i.test(text.slice(start,index).trim())
      && text.slice(start,index).trimEnd().endsWith(']') && /^\s+(?:When|Whenever|At)\b/i.test(tail);
    if (!subtype && !reveal) continue;
    programs.push(text.slice(start, index + 1).trim());
    start = index + 1;
  }
  const last = text.slice(start).trim();
  if (last) programs.push(last);
  return programs;
}

// Labels designate an ability group; they are not part of its ordinary trigger
// or static syntax. Classification strips labels, emitted programs retain them.
export function unlabelledAbilityProgram(value) {
  let text = String(value).trim();
  const label = /^(?:Keyword ability\s+"[a-z][a-z '-]*"|[a-z][a-z'-]*(?:\s+[a-z][a-z'-]*){0,4})\s*[—–]\s*/i;
  for (;;) { const body = text.replace(label, ''); if (body === text) return text; text = body; }
}
