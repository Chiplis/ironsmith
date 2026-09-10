// Older translation buckets join both faces, even under a single-face alias.
// Some buckets store escaped newlines, so normalize them before splitting.
const faces = value => String(value || '').replace(/\\n/g, '\n').split(/\s*\/\/\s*/).map(s => s.trim());
const normalized = value => value.normalize('NFKC').replace(/\s+/g, ' ').trim().toLowerCase();

export function translationForFace(translation, cardName) {
  if (!translation) return translation;
  const names = faces(translation.englishName);
  if (names.length < 2) return translation;
  const index = names.findIndex(name => normalized(name) === normalized(String(cardName || '')));
  if (index < 0) return translation;
  const field = key => {
    const values = faces(translation[key]);
    // A missing face in an old bucket is ambiguous. Let the caller fall back
    // to the visible face's English text rather than showing the other face.
    return values.length === names.length ? values[index] : '';
  };
  return {...translation, englishName: names[index], name: field('name'),
    typeLine: field('typeLine'), oracleText: field('oracleText')};
}

// A translation carries something to show for the requested face.
export function hasTranslatedFields(translation) {
  return Boolean(translation && (String(translation.name || '').trim() || String(translation.typeLine || '').trim() || String(translation.oracleText || '').trim()));
}
