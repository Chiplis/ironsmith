const RUNTIME_EVENT_BATTLEFIELD_TRANSITION = "battlefield_transition";
export const RIFT_DISSOLVE_EXILE_EFFECT_MS = 3000;
export const RIFT_DISSOLVE_EXILE_INSPECTOR_REVEAL_DELAY_MS = 1400;
export const RIFT_DISSOLVE_EXILE_BOARD_HOLD_MS = 3300;
export const ANGELIC_DESTROY_BOARD_HOLD_MS = 4200;

const ANGELIC_DESTROY_STAGGER_MS = 140;
const INSPECTOR_ENTRY_SETTLE_MS = 420;

function staggerDelayForIndex(index) {
  return Math.max(0, index) * ANGELIC_DESTROY_STAGGER_MS;
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

export function collectRuntimeAnimationEvents(state) {
  const snapshotId = state?.snapshot_id ?? "unknown";
  return (Array.isArray(state?.battlefield_transitions) ? state.battlefield_transitions : [])
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

function annotateAngelicDestroyPreviews(previews, events, { previousCardRects } = {}) {
  const matchedTransitions = [];
  previews.forEach((transition, previewIndex) => {
    const event = events.find((candidate) => destroyAnimationMatches({ event: candidate, transition }));
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
      animationKind: angelicDestroyAnimation.id,
      animationEventId: event.id,
      animationStaggerMs: staggerDelayForIndex(staggerIndex),
      inspectorShaderReveal: !inspectorRevealAssigned,
      inspectorRevealScope: !inspectorRevealAssigned ? "inspector" : undefined,
      inspectorRevealDelayMs: !inspectorRevealAssigned
        ? INSPECTOR_ENTRY_SETTLE_MS + lastStaggerDelayMs
        : undefined,
    };
    inspectorRevealAssigned = true;
    return annotated;
  });
}

function buildAngelicExileVisualEffects(previews, previousCardRects) {
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
}

function buildAngelicDestroyVisualEffects(previews, previousCardRects) {
  if (!Array.isArray(previews) || previews.length === 0) return [];
  const animatedPreviews = previews.filter((preview) => preview?.animationKind === angelicDestroyAnimation.id);
  const inspectorRevealPreview = animatedPreviews.find((preview) => preview?.inspectorShaderReveal);
  if (animatedPreviews.length === 0 || !inspectorRevealPreview) return [];

  const targetToken = inspectorRevealPreview.token;
  const groupId = `destroy-angelic-group:${targetToken}`;
  const effects = [];

  for (const preview of animatedPreviews) {
    const sourceRect = rectFromTransitionPreview(preview, previousCardRects);
    if (!sourceRect) continue;

    effects.push({
      id: `destroy-angelic:${preview.token}`,
      kind: "angelic-destroy",
      rect: sourceRect,
      travelsToInspector: true,
      includeSourceClone: true,
      sourceCloneHtml: sourceRect.sourceCloneHtml || null,
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
  buildVisualEffects: buildAngelicExileVisualEffects,
};

export const angelicDestroyAnimation = {
  id: "angelic-destroy",
  eventTypes: [RUNTIME_EVENT_BATTLEFIELD_TRANSITION],
  stateConditions: [
    { fromZone: "battlefield", toZone: "graveyard" },
  ],
  matches: destroyAnimationMatches,
  annotatePreviews: annotateAngelicDestroyPreviews,
  buildVisualEffects: buildAngelicDestroyVisualEffects,
};

const GAME_ANIMATIONS = [
  riftDissolveExileAnimation,
  angelicDestroyAnimation,
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
