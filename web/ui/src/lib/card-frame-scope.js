// Only visible battlefield cards and the perspective player's hand need
// generated frames. Other zones keep their original printing previews.
export function cardNeedsFrame(state, objectId) {
  if (objectId == null) return false;
  const matches = card => [card?.id, ...(card?.member_ids || [])]
    .some(id => id != null && String(id) === String(objectId));
  return (state?.players || []).some(player =>
    (player.battlefield || []).some(matches)
    || (state?.perspective != null && String(player.id) === String(state.perspective)
      && (player.hand_cards || []).some(matches))
  );
}
