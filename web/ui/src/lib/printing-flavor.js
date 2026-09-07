// The rules translation may come from a different reprint. Flavor must match
// the displayed printing and face, or it can describe entirely different art.
export function localizedPrintingFlavor(source, translated, locale) {
  if (!source || !translated || translated.lang !== locale
      || translated.set !== source.set
      || translated.collector_number !== source.collector_number
      || translated.oracle_id !== source.oracle_id) return '';
  const face=translated.card_faces?.find(face=>face.name===source.name);
  if (translated.card_faces && !face) return '';
  return String((face || translated).flavor_text || '');
}
