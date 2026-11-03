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
  if (!view || view.inspector_only || view.inspectorOnly) return null;
  // Revealing our own hand gives opponents information, not this perspective.
  // Use the source zone: a library reveal is still relevant even if
  // the revealed card has since moved into our hand.
  const ownHandReveal = view.visibility === "public"
    && String(view.zone).toLowerCase() === "hand"
    && view.subject != null && state?.perspective != null
    && String(view.subject) === String(state.perspective);
  return ownHandReveal ? null : view;
}

// Whether this perspective knows where any card sits in this library.
export function knowsLibraryPositions(cards) {
  return cards.some((card) => card.position != null || card.position_range);
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

const playerKey = (player) => String(player?.id ?? player?.index ?? "");

// The zone a looked-at card sits in now. Snapshots from older WASM bundles
// carry no zone, so a temporary view falls back to the zone it looked at.
function lookedCardZone(card, fallback = null) {
  return String(card?.zone ?? fallback ?? "").toLowerCase();
}

// Every card this perspective has looked at that is still in `player`'s
// `zone`: the live view plus what the engine remembers. Library cards come
// ordered top first; other zones keep no order of their own.
export function lookedZoneCards(state, player, zone) {
  const owner = playerKey(player);
  const view = temporaryLookView(state);
  // A library view only stands in for the deck while its prompt is open; an
  // acknowledged search must not leave the whole library on display.
  const liveView = view && String(view.subject) === owner
    && !(zone === "library" && view.acknowledged);
  const fromView = liveView
    ? (view.cards || []).filter((card) => lookedCardZone(card, view.zone) === zone)
    : [];
  if (zone === "library" && Array.isArray(player?.library_memory) && fromView.length === 0) {
    return libraryMemoryCards(player);
  }
  const remembered = (player?.persistent_look_cards || [])
    .filter((card) => lookedCardZone(card) === zone);
  const cards = mergeLookCards(remembered, fromView);
  if (zone === "library") {
    cards.sort((left, right) => (left.position ?? Infinity) - (right.position ?? Infinity));
  }
  return cards;
}

// The library as this perspective knows it, top first. Exact positions keep
// their object; a randomly placed copy knows only its span; the copies of our
// own deck whose slots are unknown share one stack per name; and the rest of
// an opponent's deck is backs. Only exact cards carry an object id, and the
// order inside a span or among the unknowns is not the real one: those cards
// just fill the slots no exact card claims.
export function libraryMemoryCards(player) {
  const owner = playerKey(player);
  const exact = [];
  const spans = [];
  const unknown = [];
  for (const row of player?.library_memory || []) {
    if (row.position != null) exact.push({ ...row, slot: row.position });
    else if (row.range_start != null) spans.push(row);
    else unknown.push(row);
  }
  const taken = new Set(exact.map((card) => card.slot));
  const claim = (from) => {
    let slot = from;
    while (taken.has(slot)) slot += 1;
    taken.add(slot);
    return slot;
  };
  const cards = exact.map(({ id, name, slot }) => ({ id, name, position: slot, slot }));
  let index = 0;
  let named = cards.length;
  const add = (card, copies = 1) => {
    cards.push({ id: `library-memory-${owner}-${index}`, synthetic: true, ...card });
    index += 1;
    named += copies;
  };
  for (const row of spans) {
    for (let copy = 0; copy < Math.max(1, row.count); copy += 1) {
      add({ name: row.name, position_range: [row.range_start, row.range_end], slot: claim(row.range_start) });
    }
  }
  for (const row of unknown) {
    const copies = Math.max(1, row.count);
    const slot = claim(0);
    for (let copy = 1; copy < copies; copy += 1) claim(slot);
    add({ name: row.name, unknown_position: true, stack_count: copies, slot }, copies);
  }
  // Whatever we can't name -- an opponent's deck, or ours while the engine
  // only holds hidden placeholders -- is a card back with no known slot.
  const size = Math.max(0, Number(player?.library_size) || 0);
  while (named < size) add({ card_back: true, unknown_position: true, slot: claim(0) });
  return cards.sort((left, right) => left.slot - right.slot);
}

// Whether the live view (not yet acknowledged) put cards into this zone.
export function freshLookInZone(state, player, zone) {
  const view = temporaryLookView(state);
  if (!view || view.acknowledged) return false;
  return String(view.subject) === playerKey(player)
    && (view.cards || []).some((card) => lookedCardZone(card, view.zone) === zone);
}

// Known identities of face-down permanents, keyed by object id.
export function knownFaceDownNames(state) {
  const names = new Map();
  for (const player of state?.players || []) {
    for (const card of player.persistent_look_cards || []) {
      if (lookedCardZone(card) === "battlefield" && visibleName(card)) names.set(String(card.id), card.name);
    }
  }
  return names;
}

// Hand cards this perspective can name: an open hand, or ones it saw before
// (a Thoughtseize look) that are still in that hand.
export function knownHandCards(player) {
  const known = new Map();
  for (const card of [...(player?.hand_cards || []),
    ...(player?.persistent_look_cards || []).filter((card) => lookedCardZone(card) === "hand")]) {
    if (visibleName(card)) known.set(String(card.id), card);
  }
  return [...known.values()];
}
