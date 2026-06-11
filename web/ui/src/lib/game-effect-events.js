// Derives discrete game-effect animation events (draws, mills, discards,
// shuffles, library peeks, life changes) from successive UI state snapshots.
// Pure data logic: DOM anchor resolution and rendering live in
// components/overlays/GameEffectAnimations.jsx.

// Zone pairs (from the engine's runtime zone_transitions stream) that get a
// card-flight animation. Battlefield-involved pairs are excluded: leaving the
// battlefield is handled by ZoneMoveEffects/ghost cards, and entering it by
// the new-card entry spring.
const ZONE_FLIGHT_KINDS = new Map([
  ["library->graveyard", "mill"],
  ["hand->graveyard", "discard"],
  ["hand->exile", "exile-from-hand"],
  ["library->exile", "exile-from-library"],
  ["graveyard->hand", "regrowth"],
  ["graveyard->library", "recycle"],
  ["hand->library", "tuck"],
  ["library->hand", "draw"],
  ["graveyard->battlefield", "reanimate"],
]);

// Engine-side effect events (state.effect_events) that the overlay animates.
// Anything not listed is ignored client-side.
const ENGINE_EFFECT_KINDS = new Set([
  "coin_flip",
  "die_roll",
  "control_change",
  "transform",
  "turned_face_up",
  "phase_out",
  "phase_in",
  "level_up",
  "emblem",
  "monarch",
  "initiative",
  "day_night",
  "extra_turn",
  "life_exchange",
  "damage",
  "mana_added",
]);

// Flights whose card identity is hidden in the source zone travel face down
// and flip up mid-flight; the rest fly face up the whole way.
const FACE_DOWN_SOURCE_ZONES = new Set(["library", "hand"]);

function toCount(value) {
  const count = Number(value);
  return Number.isFinite(count) && count >= 0 ? count : 0;
}

function normalizeZone(zone) {
  return String(zone || "").trim().toLowerCase();
}

function playerKeyOf(player) {
  const key = player?.id ?? player?.index;
  return key == null ? "" : String(key);
}

export function buildPlayerVitals(players) {
  const vitals = {};
  for (const player of players || []) {
    const key = playerKeyOf(player);
    if (!key) continue;
    vitals[key] = {
      handSize: toCount(player.hand_size),
      librarySize: toCount(player.library_size),
      graveyardSize: toCount(player.graveyard_size),
      life: Number(player.life) || 0,
    };
  }
  return vitals;
}

export function viewedCardsSignature(viewedCards) {
  if (!viewedCards || typeof viewedCards !== "object") return "";
  const ids = Array.isArray(viewedCards.card_ids) ? viewedCards.card_ids.join(",") : "";
  return [
    viewedCards.subject ?? "",
    normalizeZone(viewedCards.zone),
    viewedCards.visibility ?? "",
    viewedCards.description ?? "",
    ids,
  ].join("|");
}

export function collectZoneFlightEvents(state, processedTransitionIds) {
  const transitions = Array.isArray(state?.zone_transitions) ? state.zone_transitions : [];
  const events = [];
  for (const transition of transitions) {
    const transitionId = transition?.id;
    if (transitionId == null) continue;
    const idKey = String(transitionId);
    if (processedTransitionIds?.has(idKey)) continue;
    processedTransitionIds?.add(idKey);

    const fromZone = normalizeZone(transition.from_zone ?? transition.fromZone);
    const toZone = normalizeZone(transition.to_zone ?? transition.toZone);
    const kind = ZONE_FLIGHT_KINDS.get(`${fromZone}->${toZone}`);
    if (!kind) continue;

    const playerKey = String(transition.owner ?? transition.controller ?? "");
    events.push({
      type: "zone-flight",
      kind,
      id: `flight:${idKey}`,
      playerKey,
      fromZone,
      toZone,
      cardName: String(transition?.card?.name || ""),
      objectId: transition.new_object_id ?? transition.newObjectId ?? transition?.card?.id ?? null,
      revealsFace: FACE_DOWN_SOURCE_ZONES.has(fromZone),
    });
  }
  return events;
}

export function collectEngineEffectEvents(state, processedEffectEventIds) {
  const rawEvents = Array.isArray(state?.effect_events) ? state.effect_events : [];
  const events = [];
  for (const raw of rawEvents) {
    if (raw?.id == null) continue;
    const idKey = `engine:${raw.id}`;
    if (processedEffectEventIds?.has(idKey)) continue;
    processedEffectEventIds?.add(idKey);

    const kind = String(raw.kind || "");
    if (!ENGINE_EFFECT_KINDS.has(kind)) continue;
    events.push({
      type: "engine-effect",
      kind,
      id: idKey,
      playerKey: raw.player == null ? "" : String(raw.player),
      otherPlayerKey: raw.other_player == null ? "" : String(raw.other_player),
      stableIds: (Array.isArray(raw.stable_ids) ? raw.stable_ids : []).map(String),
      value: raw.value ?? null,
      text: raw.text == null ? "" : String(raw.text),
    });
  }
  return events;
}

function describeLibraryPeek(viewedCards) {
  const description = String(viewedCards?.description || "").trim();
  if (description) return description;
  const count = Array.isArray(viewedCards?.cards) ? viewedCards.cards.length : 0;
  return count > 1 ? `Looking at ${count} cards` : "Looking at the library";
}

export function collectGameEffectEvents({
  state,
  previousVitals,
  vitals,
  processedTransitionIds,
  previousViewedSignature,
  frameToken = "",
}) {
  const events = [];
  const flightEvents = collectZoneFlightEvents(state, processedTransitionIds);
  events.push(...flightEvents);
  events.push(...collectEngineEffectEvents(state, processedTransitionIds));

  const flightCounts = new Map();
  for (const flight of flightEvents) {
    const key = `${flight.playerKey}:${flight.kind}`;
    flightCounts.set(key, (flightCounts.get(key) || 0) + 1);
  }

  // A jump this large is a board reset / deck load, not gameplay — suppress
  // the frame's diff-driven events instead of showering the screen.
  const looksLikeBoardReset = Boolean(previousVitals) && Object.entries(vitals || {}).some(([playerKey, current]) => {
    const previous = previousVitals[playerKey];
    if (!previous) return false;
    return Math.abs(current.librarySize - previous.librarySize) >= 20
      || Math.abs(current.life - previous.life) >= 30;
  });

  if (previousVitals && !looksLikeBoardReset) {
    for (const [playerKey, current] of Object.entries(vitals || {})) {
      const previous = previousVitals[playerKey];
      if (!previous) continue;

      const handDelta = current.handSize - previous.handSize;
      const libraryDelta = current.librarySize - previous.librarySize;
      const lifeDelta = current.life - previous.life;

      // Draws not already represented by a visible library->hand flight
      // (opponent draws are hidden, so they never stream a transition).
      const animatedDraws = flightCounts.get(`${playerKey}:draw`) || 0;
      const drawCount = Math.min(Math.max(0, handDelta), Math.max(0, -libraryDelta)) - animatedDraws;
      if (drawCount > 0) {
        events.push({
          type: "draw-burst",
          id: `draw:${frameToken}:${playerKey}`,
          playerKey,
          count: Math.min(drawCount, 8),
        });
      }

      // A library that grew by more than one card in a single frame was
      // almost certainly shuffled (wheel, Brainstorm put-backs come one at a
      // time as tuck flights, searches shuffle on completion).
      const animatedReturns = (flightCounts.get(`${playerKey}:tuck`) || 0)
        + (flightCounts.get(`${playerKey}:recycle`) || 0);
      if (libraryDelta - animatedReturns >= 2) {
        events.push({
          type: "shuffle",
          id: `shuffle:${frameToken}:${playerKey}`,
          playerKey,
        });
      }

      if (lifeDelta !== 0) {
        events.push({
          type: "life",
          id: `life:${frameToken}:${playerKey}`,
          playerKey,
          delta: lifeDelta,
        });
      }
    }
  }

  const viewedCards = state?.viewed_cards || null;
  const viewedSignature = viewedCardsSignature(viewedCards);
  if (
    viewedCards
    && viewedSignature
    && viewedSignature !== previousViewedSignature
    && normalizeZone(viewedCards.zone) === "library"
  ) {
    const count = Array.isArray(viewedCards.cards) ? viewedCards.cards.length : 0;
    if (count > 0) {
      events.push({
        type: "library-peek",
        id: `peek:${viewedSignature}`,
        playerKey: String(viewedCards.subject ?? ""),
        count: Math.min(count, 5),
        label: describeLibraryPeek(viewedCards),
      });
    }
  }

  return { events, viewedSignature };
}
