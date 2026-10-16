import { mergeLookCards } from './look-pile.js';

// Persistent views also contain hands and face-down exile cards. Those objects
// must never become the face of a library, even when they share its top's name.
export function visibleLibraryCards(state, player) {
  const elsewhere = new Set((state?.players || [player]).flatMap((entry) =>
    ['battlefield', 'hand_cards', 'graveyard_cards', 'exile_cards', 'command_cards', 'ante_cards']
      .flatMap((zone) => entry[zone] || [])).map((card) => String(card.id)));
  const persistent = player.can_view_library_top
    ? (player.persistent_look_cards || []).filter((card) => !elsewhere.has(String(card.id)))
    : [];
  const view = state?.viewed_cards;
  const viewingLibrary = String(view?.zone).toLowerCase() === 'library'
    && String(view?.subject) === String(player.id ?? player.index);
  const temporary = viewingLibrary
    ? (view.cards || []).filter((card) => !elsewhere.has(String(card.id))) : [];
  const cards = mergeLookCards(persistent, temporary);
  if (!cards.length && player.can_view_library_top && player.library_top
    && !Array.isArray(player.persistent_look_cards)) {
    cards.push({ id: `look-top-${player.id ?? player.index}`, name: player.library_top });
  }
  return cards;
}
