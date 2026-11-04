// Hand frames print a card's official Oracle text rather than the engine's
// rendering of its compiled abilities. Other zones keep the live text, which
// reflects granted and removed abilities.
export function withOfficialOracleText(card, source) {
  if (!card || !source) return card;
  const sourceFaces = Array.isArray(source.card_faces) ? source.card_faces : [];
  const faceText = name => sourceFaces.find(face => face.name === name)?.oracle_text;
  if (Array.isArray(card.card_faces) && card.card_faces.length) {
    return {...card, card_faces: card.card_faces.map(face => {
      const text = faceText(face.name);
      return text == null ? face : {...face, oracle_text: text};
    })};
  }
  const text = sourceFaces.length ? faceText(card.name) : source.oracle_text;
  return text == null ? card : {...card, oracle_text: text};
}
