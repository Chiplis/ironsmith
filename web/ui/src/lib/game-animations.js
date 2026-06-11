const RUNTIME_EVENT_BATTLEFIELD_TRANSITION = "battlefield_transition";
const RUNTIME_EVENT_EFFECT = "effect_event";

export const RIFT_DISSOLVE_EXILE_EFFECT_MS = 3000;
export const RIFT_DISSOLVE_EXILE_INSPECTOR_REVEAL_DELAY_MS = 1500;
export const RIFT_DISSOLVE_EXILE_BOARD_HOLD_MS = 3300;

// Marquee leave animations (death, sacrifice, counter) pair an in-place DOM
// collapse with a WebGL particle stream that converges on the card inspector
// as it unmasks. The DOM collapse finishes inside the board hold; the stream
// keeps running to the inspector afterwards.
export const DEATH_COLLAPSE_EFFECT_MS = 1500;
// The hold must expire BEFORE the overlay effect unmounts (effect duration +
// cleanup tail), otherwise the held card flashes back for a frame when the
// overlay's hold-hiding class is removed.
export const DEATH_COLLAPSE_BOARD_HOLD_MS = 1600;
export const MARQUEE_STREAM_EFFECT_MS = 2000;
export const MARQUEE_INSPECTOR_REVEAL_DELAY_MS = 1250;
export const WIPE_WAVE_EFFECT_MS = 900;
export const WIPE_WAVE_MIN_DEATHS = 3;

const MARQUEE_STAGGER_MS = 90;

function staggerDelayForIndex(index) {
  return Math.max(0, index) * MARQUEE_STAGGER_MS;
}

function stableIdFromEvent(event) {
  if (event?.stableId != null) return String(event.stableId);
  if (event?.stable_id != null) return String(event.stable_id);
  return null;
}

function transitionStableIds(transition) {
  const ids = new Set();
  for (const key of transition?.trackingKeys || []) {
    const match = String(key).match(/^stable:(.+)$/);
    if (match) ids.add(match[1]);
  }
  if (transition?.card?.stable_id != null) ids.add(String(transition.card.stable_id));
  for (const stableId of transition?.card?.member_stable_ids || []) {
    ids.add(String(stableId));
  }
  return ids;
}

function transitionMatchesStableId(transition, stableId) {
  if (stableId == null) return false;
  return transitionStableIds(transition).has(String(stableId));
}

function transitionMatchesAnyStableId(transition, stableIds) {
  const ids = transitionStableIds(transition);
  return (stableIds || []).some((stableId) => ids.has(String(stableId)));
}

export function collectRuntimeAnimationEvents(state) {
  const snapshotId = state?.snapshot_id ?? "unknown";
  const transitionEvents = (Array.isArray(state?.battlefield_transitions) ? state.battlefield_transitions : [])
    .map((transition, index) => {
      const stableId = stableIdFromEvent(transition);
      const kind = String(transition?.kind || "");
      if (!stableId || !kind) return null;
      return {
        id: `${snapshotId}:${RUNTIME_EVENT_BATTLEFIELD_TRANSITION}:${index}:${stableId}:${kind}`,
        type: RUNTIME_EVENT_BATTLEFIELD_TRANSITION,
        kind,
        stableId,
        raw: transition,
      };
    })
    .filter(Boolean);

  const effectEvents = (Array.isArray(state?.effect_events) ? state.effect_events : [])
    .map((event) => {
      const kind = String(event?.kind || "");
      if (!kind || event?.id == null) return null;
      return {
        id: `${snapshotId}:${RUNTIME_EVENT_EFFECT}:${event.id}:${kind}`,
        type: RUNTIME_EVENT_EFFECT,
        kind,
        stableIds: (Array.isArray(event.stable_ids) ? event.stable_ids : []).map(String),
        player: event.player ?? null,
        otherPlayer: event.other_player ?? null,
        value: event.value ?? null,
        text: event.text ?? null,
        raw: event,
      };
    })
    .filter(Boolean);

  return [...transitionEvents, ...effectEvents];
}

function exileAnimationMatches({ event, transition }) {
  return (
    event?.type === RUNTIME_EVENT_BATTLEFIELD_TRANSITION
    && event.kind === "exiled"
    && transition?.fromZone === "battlefield"
    && transition?.toZone === "exile"
    && transitionMatchesStableId(transition, event.stableId)
  );
}

function destroyAnimationMatches({ event, transition }) {
  return (
    event?.type === RUNTIME_EVENT_BATTLEFIELD_TRANSITION
    && event.kind === "destroyed"
    && transition?.fromZone === "battlefield"
    && transition?.toZone === "graveyard"
    && transitionMatchesStableId(transition, event.stableId)
  );
}

function sacrificeAnimationMatches({ event, transition }) {
  return (
    event?.type === RUNTIME_EVENT_BATTLEFIELD_TRANSITION
    && event.kind === "sacrificed"
    && transition?.fromZone === "battlefield"
    && transition?.toZone === "graveyard"
    && transitionMatchesStableId(transition, event.stableId)
  );
}

function counterShatterMatches({ event, transition }) {
  return (
    event?.type === RUNTIME_EVENT_EFFECT
    && event.kind === "spell_countered"
    && transition?.fromZone === "stack"
    && (transition?.toZone === "graveyard" || transition?.toZone === "exile")
    && transitionMatchesAnyStableId(transition, event.stableIds)
  );
}

function rectFromTransitionPreview(preview, previousCardRects) {
  const sourceObjectIds = [
    preview?.fromObjectId,
    preview?.objectId,
  ].filter((objectId) => objectId != null);

  for (const objectId of sourceObjectIds) {
    const rect = previousCardRects?.get(`object:${objectId}`);
    if (rect) return rect;
  }

  for (const key of preview?.trackingKeys || []) {
    const rect = previousCardRects?.get(key);
    if (rect) return rect;
  }

  const fallbackObjectId = preview?.fromObjectId ?? preview?.objectId;
  if (fallbackObjectId != null) {
    return previousCardRects?.get(`object:${fallbackObjectId}`) || null;
  }

  return null;
}

function annotateAngelicExilePreviews(previews, events) {
  let inspectorRevealAssigned = false;
  return previews.map((transition) => {
    const event = events.find((candidate) => exileAnimationMatches({ event: candidate, transition }));
    if (!event) {
      return transition;
    }

    const annotated = {
      ...transition,
      animationKind: riftDissolveExileAnimation.id,
      animationEventId: event.id,
      inspectorShaderReveal: !inspectorRevealAssigned,
      inspectorRevealScope: !inspectorRevealAssigned ? "inspector" : undefined,
      inspectorRevealDelayMs: !inspectorRevealAssigned
        ? RIFT_DISSOLVE_EXILE_INSPECTOR_REVEAL_DELAY_MS
        : undefined,
    };
    inspectorRevealAssigned = true;
    return annotated;
  });
}

function compareTransitionsBySourceRect(left, right, previousCardRects) {
  const leftRect = rectFromTransitionPreview(left.transition, previousCardRects);
  const rightRect = rectFromTransitionPreview(right.transition, previousCardRects);
  if (leftRect && rightRect) {
    const rowDelta = leftRect.top - rightRect.top;
    if (Math.abs(rowDelta) > 8) return rowDelta;
    const columnDelta = leftRect.left - rightRect.left;
    if (Math.abs(columnDelta) > 8) return columnDelta;
  } else if (leftRect) {
    return -1;
  } else if (rightRect) {
    return 1;
  }
  return left.previewIndex - right.previewIndex;
}

// Shared annotation for the marquee collapse animations (death + sacrifice).
// Cards are staggered by board position; the first matched preview carries
// the inspector reveal so the WebGL stream has a destination to converge on.
function annotateMarqueeCollapsePreviews(previews, events, { previousCardRects } = {}, animation) {
  const matchedTransitions = [];
  previews.forEach((transition, previewIndex) => {
    const event = events.find((candidate) => animation.matches({ event: candidate, transition }));
    if (event) matchedTransitions.push({ transition, event, previewIndex });
  });
  const staggerIndexByTransition = new Map(
    [...matchedTransitions]
      .sort((left, right) => compareTransitionsBySourceRect(left, right, previousCardRects))
      .map((entry, index) => [entry.transition, index])
  );
  const eventByTransition = new Map(
    matchedTransitions.map((entry) => [entry.transition, entry.event])
  );
  const lastStaggerDelayMs = staggerDelayForIndex(Math.max(0, matchedTransitions.length - 1));
  let inspectorRevealAssigned = previews.some((preview) => preview?.inspectorShaderReveal === true);
  return previews.map((transition) => {
    const staggerIndex = staggerIndexByTransition.get(transition);
    if (staggerIndex == null) {
      return transition;
    }
    const event = eventByTransition.get(transition);

    const annotated = {
      ...transition,
      animationKind: animation.id,
      animationEventId: event.id,
      animationStaggerMs: staggerDelayForIndex(staggerIndex),
      inspectorShaderReveal: !inspectorRevealAssigned,
      inspectorRevealScope: !inspectorRevealAssigned ? "inspector" : undefined,
      inspectorRevealDelayMs: !inspectorRevealAssigned
        ? MARQUEE_INSPECTOR_REVEAL_DELAY_MS + lastStaggerDelayMs
        : undefined,
    };
    inspectorRevealAssigned = true;
    return annotated;
  });
}

function unionRects(rects) {
  if (rects.length === 0) return null;
  const left = Math.min(...rects.map((rect) => rect.left));
  const top = Math.min(...rects.map((rect) => rect.top));
  const right = Math.max(...rects.map((rect) => rect.left + rect.width));
  const bottom = Math.max(...rects.map((rect) => rect.top + rect.height));
  return { left, top, width: right - left, height: bottom - top };
}

// Shared visual-effect builder for death and sacrifice: one in-place DOM
// collapse per card plus one WebGL stream per card converging on the
// inspector reveal. Mass deaths additionally emit a single battlefield-wide
// shockwave the shader sweeps across the union of the source rects.
function buildMarqueeCollapseVisualEffects(previews, previousCardRects, animation) {
  if (!Array.isArray(previews) || previews.length === 0) return [];
  const animatedPreviews = previews.filter((preview) => preview?.animationKind === animation.id);
  if (animatedPreviews.length === 0) return [];
  const inspectorRevealPreview = animatedPreviews.find((preview) => preview?.inspectorShaderReveal);

  const targetToken = inspectorRevealPreview?.token || null;
  const groupId = `${animation.id}-group:${targetToken || animatedPreviews[0].token}`;
  const effects = [];
  const sourceRects = [];

  for (const preview of animatedPreviews) {
    const sourceRect = rectFromTransitionPreview(preview, previousCardRects);
    if (!sourceRect) continue;
    sourceRects.push(sourceRect);

    effects.push({
      id: `${animation.id}:${preview.token}`,
      kind: "death-collapse",
      collapseVariant: animation.collapseVariant,
      rect: sourceRect,
      travelsToInspector: false,
      includeSourceClone: true,
      sourceCloneHtml: sourceRect.sourceCloneHtml || null,
      sourceImageUrl: sourceRect.sourceImageUrl || null,
      card: preview.card,
      playerKey: preview.playerKey,
      objectId: preview.fromObjectId ?? preview.objectId ?? null,
      startDelayMs: preview.animationStaggerMs || 0,
    });

    if (targetToken) {
      effects.push({
        id: `${animation.id}-stream:${preview.token}`,
        kind: "marquee-stream",
        streamProfile: animation.streamProfile,
        rect: sourceRect,
        travelsToInspector: true,
        includeSourceClone: false,
        sourceCloneHtml: null,
        sourceImageUrl: sourceRect.sourceImageUrl || null,
        card: preview.card,
        playerKey: preview.playerKey,
        objectId: preview.fromObjectId ?? preview.objectId ?? null,
        targetToken,
        targetScope: "inspector",
        groupId,
        startDelayMs: preview.animationStaggerMs || 0,
      });
    }
  }

  if (animation.emitsWipeWave && sourceRects.length >= WIPE_WAVE_MIN_DEATHS) {
    const waveRect = unionRects(sourceRects);
    if (waveRect) {
      effects.push({
        id: `wipe-wave:${groupId}`,
        kind: "wipe-wave",
        rect: waveRect,
        travelsToInspector: false,
        includeSourceClone: false,
        card: {},
        playerKey: animatedPreviews[0].playerKey,
        startDelayMs: 0,
      });
    }
  }

  return effects;
}

function annotateCounterShatterPreviews(previews, events) {
  let inspectorRevealAssigned = previews.some((preview) => preview?.inspectorShaderReveal === true);
  return previews.map((transition) => {
    const event = events.find((candidate) => counterShatterMatches({ event: candidate, transition }));
    if (!event) {
      return transition;
    }

    const annotated = {
      ...transition,
      animationKind: counterShatterAnimation.id,
      animationEventId: event.id,
      animationStaggerMs: 0,
      inspectorShaderReveal: !inspectorRevealAssigned,
      inspectorRevealScope: !inspectorRevealAssigned ? "inspector" : undefined,
      inspectorRevealDelayMs: !inspectorRevealAssigned
        ? MARQUEE_INSPECTOR_REVEAL_DELAY_MS
        : undefined,
    };
    inspectorRevealAssigned = true;
    return annotated;
  });
}

function buildCounterShatterVisualEffects(previews, previousCardRects) {
  if (!Array.isArray(previews) || previews.length === 0) return [];
  const animatedPreviews = previews.filter((preview) => preview?.animationKind === counterShatterAnimation.id);
  if (animatedPreviews.length === 0) return [];
  const inspectorRevealPreview = animatedPreviews.find((preview) => preview?.inspectorShaderReveal);
  const targetToken = inspectorRevealPreview?.token || null;
  const groupId = `counter-shatter-group:${targetToken || animatedPreviews[0].token}`;
  const effects = [];

  for (const preview of animatedPreviews) {
    const sourceRect = rectFromTransitionPreview(preview, previousCardRects);
    if (!sourceRect) continue;

    effects.push({
      id: `counter-shatter:${preview.token}`,
      kind: "counter-shatter",
      rect: sourceRect,
      travelsToInspector: false,
      includeSourceClone: true,
      sourceCloneHtml: sourceRect.sourceCloneHtml || null,
      sourceImageUrl: sourceRect.sourceImageUrl || null,
      card: preview.card,
      playerKey: preview.playerKey,
      objectId: preview.fromObjectId ?? preview.objectId ?? null,
      startDelayMs: 0,
    });

    if (targetToken) {
      effects.push({
        id: `counter-shatter-stream:${preview.token}`,
        kind: "marquee-stream",
        streamProfile: "counter",
        rect: sourceRect,
        travelsToInspector: true,
        includeSourceClone: false,
        sourceCloneHtml: null,
        sourceImageUrl: sourceRect.sourceImageUrl || null,
        card: preview.card,
        playerKey: preview.playerKey,
        objectId: preview.fromObjectId ?? preview.objectId ?? null,
        targetToken,
        targetScope: "inspector",
        groupId,
        startDelayMs: 0,
      });
    }
  }

  return effects;
}

export const riftDissolveExileAnimation = {
  id: "rift-dissolve-exile",
  eventTypes: [RUNTIME_EVENT_BATTLEFIELD_TRANSITION],
  stateConditions: [
    { fromZone: "battlefield", toZone: "exile" },
  ],
  matches: exileAnimationMatches,
  annotatePreviews: annotateAngelicExilePreviews,
  buildVisualEffects: (previews, previousCardRects) => {
    if (!Array.isArray(previews) || previews.length === 0) return [];
    const animatedPreviews = previews.filter((preview) => preview?.animationKind === riftDissolveExileAnimation.id);
    const inspectorRevealPreview = animatedPreviews.find((preview) => preview?.inspectorShaderReveal);
    if (animatedPreviews.length === 0 || !inspectorRevealPreview) return [];

    const targetToken = inspectorRevealPreview.token;
    const groupId = `exile-group:${targetToken}`;
    const effects = [];

    for (const preview of animatedPreviews) {
      const sourceRect = rectFromTransitionPreview(preview, previousCardRects);
      if (!sourceRect) continue;

      effects.push({
        id: `exile-source:${preview.token}`,
        kind: "rift-dissolve-exile",
        rect: sourceRect,
        travelsToInspector: false,
        includeSourceClone: true,
        sourceCloneHtml: sourceRect.sourceCloneHtml || null,
        sourceImageUrl: sourceRect.sourceImageUrl || null,
        card: preview.card,
        playerKey: preview.playerKey,
        objectId: preview.fromObjectId ?? preview.objectId ?? null,
        targetToken: null,
        groupId,
      });

      effects.push({
        id: `exile-flight:${preview.token}`,
        kind: "rift-dissolve-exile",
        rect: sourceRect,
        travelsToInspector: true,
        includeSourceClone: false,
        sourceCloneHtml: null,
        sourceImageUrl: sourceRect.sourceImageUrl || null,
        card: preview.card,
        playerKey: preview.playerKey,
        objectId: preview.fromObjectId ?? preview.objectId ?? null,
        targetToken,
        targetScope: "inspector",
        groupId,
      });
    }

    return effects;
  },
};

export const deathCollapseAnimation = {
  id: "death-collapse",
  eventTypes: [RUNTIME_EVENT_BATTLEFIELD_TRANSITION],
  stateConditions: [
    { fromZone: "battlefield", toZone: "graveyard" },
  ],
  collapseVariant: "destroyed",
  streamProfile: "death",
  emitsWipeWave: true,
  matches: destroyAnimationMatches,
  annotatePreviews: (previews, events, context) => (
    annotateMarqueeCollapsePreviews(previews, events, context, deathCollapseAnimation)
  ),
  buildVisualEffects: (previews, previousCardRects) => (
    buildMarqueeCollapseVisualEffects(previews, previousCardRects, deathCollapseAnimation)
  ),
};

export const sacrificeCollapseAnimation = {
  id: "sacrifice-collapse",
  eventTypes: [RUNTIME_EVENT_BATTLEFIELD_TRANSITION],
  stateConditions: [
    { fromZone: "battlefield", toZone: "graveyard" },
  ],
  collapseVariant: "sacrificed",
  streamProfile: "sacrifice",
  emitsWipeWave: false,
  matches: sacrificeAnimationMatches,
  annotatePreviews: (previews, events, context) => (
    annotateMarqueeCollapsePreviews(previews, events, context, sacrificeCollapseAnimation)
  ),
  buildVisualEffects: (previews, previousCardRects) => (
    buildMarqueeCollapseVisualEffects(previews, previousCardRects, sacrificeCollapseAnimation)
  ),
};

export const counterShatterAnimation = {
  id: "counter-shatter",
  eventTypes: [RUNTIME_EVENT_EFFECT],
  stateConditions: [
    { fromZone: "stack", toZone: "graveyard" },
    { fromZone: "stack", toZone: "exile" },
  ],
  matches: counterShatterMatches,
  annotatePreviews: annotateCounterShatterPreviews,
  buildVisualEffects: buildCounterShatterVisualEffects,
};

const GAME_ANIMATIONS = [
  riftDissolveExileAnimation,
  deathCollapseAnimation,
  sacrificeCollapseAnimation,
  counterShatterAnimation,
];

export function resolveGameAnimations({ previews, state, previousCardRects }) {
  const runtimeEvents = collectRuntimeAnimationEvents(state);
  let annotatedPreviews = Array.isArray(previews) ? previews : [];
  let visualEffects = [];

  for (const animation of GAME_ANIMATIONS) {
    annotatedPreviews = animation.annotatePreviews(annotatedPreviews, runtimeEvents, { previousCardRects });
    visualEffects = [
      ...visualEffects,
      ...animation.buildVisualEffects(annotatedPreviews, previousCardRects),
    ];
  }

  return {
    previews: annotatedPreviews,
    visualEffects,
    runtimeEvents,
  };
}
