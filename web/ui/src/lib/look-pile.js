export const LOOK_DONE_EVENT = "ironsmith:look-done";
export const LOOK_FADE_MS = 3000;

export function lookViewKey(view, decision) {
  const identity = [decision?.kind, decision?.player, decision?.source_id,
    decision?.source_name, decision?.reason, decision?.description,
    decision?.context_text, decision?.consequence_text];
  return view ? JSON.stringify([identity, view]) : "";
}

export function temporaryLookView(state) {
  const view = state?.viewed_cards;
  return view && !view.inspector_only && !view.inspectorOnly ? view : null;
}

function visibleName(card) {
  return Boolean(card?.name) && !/^(hidden card|face.down card|unknown card)$/i.test(card.name.trim());
}

export function persistentLookCards(state) {
  const cards = [];
  for (const player of state?.players || []) {
    if (Array.isArray(player.persistent_look_cards)) {
      cards.push(...player.persistent_look_cards);
      continue;
    }
    // Compatibility with snapshots from older WASM bundles.
    if (player.can_view_library_top && player.library_top) {
      cards.push({ id: `look-top-${player.id ?? player.index}`, name: player.library_top });
    }
    // Face-down exile cards only have names when this perspective can see them.
    cards.push(...(player.exile_cards || []).filter((card) => card.face_down && visibleName(card)));

  }
  const view = state?.viewed_cards;
  if (view?.inspector_only || view?.inspectorOnly) cards.push(...(view.cards || []));
  return mergeLookCards(cards);
}

export function mergeLookCards(...groups) {
  return [...new Map(groups.flat().filter(visibleName).map((card) => [String(card.id), {
    ...card, face_down: false, is_face_down: false,
  }])).values()];
}
