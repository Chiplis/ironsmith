import test from "node:test";
import assert from "node:assert/strict";

import {
  collectRuntimeAnimationEvents,
  RIFT_DISSOLVE_EXILE_INSPECTOR_REVEAL_DELAY_MS,
  resolveGameAnimations,
} from "../src/lib/game-animations.js";

const rect = {
  left: 10,
  top: 20,
  width: 100,
  height: 140,
  sourceCloneHtml: "<div></div>",
  sourceImageUrl: "https://example.test/card.jpg",
};

function exilePreview(overrides = {}) {
  return {
    token: "p0:stable:42",
    objectId: 100,
    fromObjectId: 99,
    toObjectId: 100,
    fromZone: "battlefield",
    toZone: "exile",
    playerKey: "0",
    card: {
      id: 100,
      stable_id: 42,
      name: "Test Card",
    },
    trackingKeys: ["stable:42", "object:99", "object:100"],
    title: "Battlefield -> Exile",
    ...overrides,
  };
}

function destroyPreview(overrides = {}) {
  return {
    token: "p0:stable:43",
    objectId: 101,
    fromObjectId: 98,
    toObjectId: 101,
    fromZone: "battlefield",
    toZone: "graveyard",
    playerKey: "0",
    card: {
      id: 101,
      stable_id: 43,
      name: "Destroyed Card",
    },
    trackingKeys: ["stable:43", "object:98", "object:101"],
    title: "Battlefield -> Graveyard",
    ...overrides,
  };
}

test("collects runtime animation events from battlefield transitions", () => {
  const events = collectRuntimeAnimationEvents({
    snapshot_id: 7,
    battlefield_transitions: [
      { stable_id: 42, kind: "exiled" },
      { stable_id: 43, kind: "destroyed" },
    ],
  });

  assert.deepEqual(
    events.map((event) => [event.type, event.kind, event.stableId]),
    [
      ["battlefield_transition", "exiled", "42"],
      ["battlefield_transition", "destroyed", "43"],
    ],
  );
});

test("exile animation requires both an exiled runtime event and battlefield-to-exile state", () => {
  const previousCardRects = new Map([["stable:42", rect]]);
  const withoutEvent = resolveGameAnimations({
    previews: [exilePreview()],
    state: { snapshot_id: 1, battlefield_transitions: [] },
    previousCardRects,
  });
  assert.equal(withoutEvent.visualEffects.length, 0);
  assert.equal(withoutEvent.previews[0].animationKind, undefined);

  const withWrongState = resolveGameAnimations({
    previews: [exilePreview({ toZone: "graveyard", title: "Battlefield -> Graveyard" })],
    state: { snapshot_id: 2, battlefield_transitions: [{ stable_id: 42, kind: "exiled" }] },
    previousCardRects,
  });
  assert.equal(withWrongState.visualEffects.length, 0);
  assert.equal(withWrongState.previews[0].animationKind, undefined);
});

test("exile animation builds source and flight effects for matching runtime events", () => {
  const result = resolveGameAnimations({
    previews: [exilePreview()],
    state: { snapshot_id: 3, battlefield_transitions: [{ stable_id: 42, kind: "exiled" }] },
    previousCardRects: new Map([["stable:42", rect]]),
  });

  assert.equal(result.previews[0].animationKind, "rift-dissolve-exile");
  assert.equal(result.previews[0].inspectorShaderReveal, true);
  assert.deepEqual(
    result.visualEffects.map((effect) => [effect.id, effect.kind, effect.travelsToInspector]),
    [
      ["exile-source:p0:stable:42", "rift-dissolve-exile", false],
      ["exile-flight:p0:stable:42", "rift-dissolve-exile", true],
    ],
  );
  assert.equal(result.previews[0].inspectorRevealScope, "inspector");
  assert.equal(
    result.previews[0].inspectorRevealDelayMs,
    RIFT_DISSOLVE_EXILE_INSPECTOR_REVEAL_DELAY_MS,
  );
  assert.equal(result.visualEffects[1].targetScope, "inspector");
});

test("exile inspector reveal waits for the shared particle arrival window", () => {
  const previews = [
    exilePreview({
      token: "p0:stable:42",
      card: { id: 100, stable_id: 42, name: "Near Exile" },
      trackingKeys: ["stable:42"],
    }),
    exilePreview({
      token: "p0:stable:43",
      card: { id: 101, stable_id: 43, name: "Far Exile" },
      trackingKeys: ["stable:43"],
    }),
  ];
  const result = resolveGameAnimations({
    previews,
    state: {
      snapshot_id: 32,
      battlefield_transitions: [
        { stable_id: 42, kind: "exiled" },
        { stable_id: 43, kind: "exiled" },
      ],
    },
    previousCardRects: new Map([
      ["stable:42", { ...rect, left: 820, top: 500 }],
      ["stable:43", { ...rect, left: 20, top: 80 }],
    ]),
  });

  assert.deepEqual(
    result.previews.map((preview) => preview.inspectorRevealDelayMs),
    [RIFT_DISSOLVE_EXILE_INSPECTOR_REVEAL_DELAY_MS, undefined],
  );
  assert.deepEqual(
    result.visualEffects
      .filter((effect) => effect.travelsToInspector)
      .map((effect) => effect.startDelayMs || 0),
    [0, 0],
  );
});

test("exile animation uses the exact previous object rect before stable-id fallbacks", () => {
  const stableFallbackRect = { ...rect, left: 500, x: 500 };
  const sourceObjectRect = { ...rect, left: 120, x: 120 };
  const result = resolveGameAnimations({
    previews: [exilePreview()],
    state: { snapshot_id: 31, battlefield_transitions: [{ stable_id: 42, kind: "exiled" }] },
    previousCardRects: new Map([
      ["stable:42", stableFallbackRect],
      ["object:99", sourceObjectRect],
    ]),
  });

  assert.deepEqual(
    result.visualEffects.map((effect) => effect.rect.left),
    [120, 120],
  );
});

test("destroy animation builds a collapse plus an inspector-bound stream", () => {
  const result = resolveGameAnimations({
    previews: [destroyPreview()],
    state: { snapshot_id: 4, battlefield_transitions: [{ stable_id: 43, kind: "destroyed" }] },
    previousCardRects: new Map([["stable:43", rect]]),
  });

  assert.equal(result.previews[0].animationKind, "death-collapse");
  assert.equal(result.previews[0].inspectorShaderReveal, true);
  assert.equal(result.previews[0].inspectorRevealScope, "inspector");
  assert.equal(result.previews[0].inspectorRevealDelayMs, 1250);
  assert.equal(result.previews[0].animationStaggerMs, 0);
  assert.deepEqual(
    result.visualEffects.map((effect) => [effect.id, effect.kind, effect.travelsToInspector]),
    [
      ["death-collapse:p0:stable:43", "death-collapse", false],
      ["death-collapse-stream:p0:stable:43", "marquee-stream", true],
    ],
  );
  assert.equal(result.visualEffects[0].collapseVariant, "destroyed");
  assert.equal(result.visualEffects[0].sourceImageUrl, rect.sourceImageUrl);
  assert.equal(result.visualEffects[0].sourceCloneHtml, rect.sourceCloneHtml);
  assert.equal(result.visualEffects[1].streamProfile, "death");
  assert.equal(result.visualEffects[1].targetToken, "p0:stable:43");
  assert.equal(result.visualEffects[1].targetScope, "inspector");
});

test("death animation follows battlefield-to-graveyard moves without a destroyed event", () => {
  const result = resolveGameAnimations({
    previews: [destroyPreview()],
    state: { snapshot_id: 5, battlefield_transitions: [] },
    previousCardRects: new Map([["stable:43", rect]]),
  });

  assert.equal(result.previews[0].animationKind, "death-collapse");
  assert.match(result.previews[0].animationEventId, /^death-collapse:preview:/);
  assert.deepEqual(
    result.visualEffects.map((effect) => [effect.id, effect.kind, effect.travelsToInspector]),
    [
      ["death-collapse:p0:stable:43", "death-collapse", false],
      ["death-collapse-stream:p0:stable:43", "marquee-stream", true],
    ],
  );
});

test("sacrifice gets the violet collapse variant and its own stream profile", () => {
  const result = resolveGameAnimations({
    previews: [destroyPreview()],
    state: { snapshot_id: 9, battlefield_transitions: [{ stable_id: 43, kind: "sacrificed" }] },
    previousCardRects: new Map([["stable:43", rect]]),
  });

  assert.equal(result.previews[0].animationKind, "sacrifice-collapse");
  assert.equal(result.previews[0].inspectorShaderReveal, true);
  assert.equal(result.visualEffects.length, 2);
  assert.deepEqual(
    result.visualEffects.map((effect) => [effect.kind, effect.collapseVariant ?? effect.streamProfile]),
    [
      ["death-collapse", "sacrificed"],
      ["marquee-stream", "sacrifice"],
    ],
  );
});

test("countered spells shatter from the stack and stream to the inspector", () => {
  const preview = {
    token: "p0:stack:77",
    objectId: 300,
    fromObjectId: 300,
    toObjectId: 301,
    fromZone: "stack",
    toZone: "graveyard",
    playerKey: "0",
    card: { id: 301, stable_id: 77, name: "Countered Spell" },
    trackingKeys: ["stable:77", "object:300", "object:301"],
    title: "Stack -> Graveyard",
  };
  const result = resolveGameAnimations({
    previews: [preview],
    state: {
      snapshot_id: 10,
      battlefield_transitions: [],
      effect_events: [{ id: 5, kind: "spell_countered", stable_ids: [77] }],
    },
    previousCardRects: new Map([["object:300", rect]]),
  });

  assert.equal(result.previews[0].animationKind, "counter-shatter");
  assert.equal(result.previews[0].inspectorShaderReveal, true);
  assert.deepEqual(
    result.visualEffects.map((effect) => [effect.id, effect.kind, effect.travelsToInspector]),
    [
      ["counter-shatter:p0:stack:77", "counter-shatter", false],
      ["counter-shatter-stream:p0:stack:77", "marquee-stream", true],
    ],
  );
  assert.equal(result.visualEffects[1].streamProfile, "counter");
});

test("three or more simultaneous deaths emit a single wipe wave", () => {
  const previews = Array.from({ length: 3 }, (_, index) => (
    destroyPreview({
      token: `p0:stable:${43 + index}`,
      card: { id: 101 + index, stable_id: 43 + index, name: `Wiped ${index + 1}` },
      trackingKeys: [`stable:${43 + index}`],
    })
  ));
  const result = resolveGameAnimations({
    previews,
    state: {
      snapshot_id: 11,
      battlefield_transitions: previews.map((preview) => ({
        stable_id: preview.card.stable_id,
        kind: "destroyed",
      })),
    },
    previousCardRects: new Map(previews.map((preview, index) => [
      `stable:${preview.card.stable_id}`,
      { ...rect, left: index * 200 },
    ])),
  });

  const waves = result.visualEffects.filter((effect) => effect.kind === "wipe-wave");
  assert.equal(waves.length, 1);
  assert.equal(waves[0].rect.left, 0);
  assert.equal(waves[0].rect.width, 200 * 2 + rect.width);
});

test("destroy animation staggers collapses by source position", () => {
  const previews = [
    destroyPreview({ token: "p0:stable:43", card: { id: 101, stable_id: 43, name: "Destroyed A" }, trackingKeys: ["stable:43"] }),
    destroyPreview({ token: "p0:stable:44", card: { id: 102, stable_id: 44, name: "Destroyed B" }, trackingKeys: ["stable:44"] }),
    destroyPreview({ token: "p0:stable:45", card: { id: 103, stable_id: 45, name: "Destroyed C" }, trackingKeys: ["stable:45"] }),
  ];
  const result = resolveGameAnimations({
    previews,
    state: {
      snapshot_id: 7,
      battlefield_transitions: [
        { stable_id: 43, kind: "destroyed" },
        { stable_id: 44, kind: "destroyed" },
        { stable_id: 45, kind: "destroyed" },
      ],
    },
    previousCardRects: new Map([
      ["stable:43", rect],
      ["stable:44", rect],
      ["stable:45", rect],
    ]),
  });

  assert.deepEqual(
    result.previews.map((preview) => preview.animationStaggerMs),
    [0, 90, 180],
  );
  assert.deepEqual(
    result.visualEffects
      .filter((effect) => effect.kind === "death-collapse")
      .map((effect) => effect.startDelayMs),
    [0, 90, 180],
  );
  assert.deepEqual(
    result.visualEffects
      .filter((effect) => effect.kind === "marquee-stream")
      .map((effect) => effect.startDelayMs),
    [0, 90, 180],
  );
  assert.equal(result.previews[0].inspectorRevealDelayMs, 1250 + 180);
});

test("destroy animation does not reuse stagger start times for adjacent visual sources", () => {
  const previews = Array.from({ length: 6 }, (_, index) => (
    destroyPreview({
      token: `p0:stable:${43 + index}`,
      card: { id: 101 + index, stable_id: 43 + index, name: `Destroyed ${index + 1}` },
      trackingKeys: [`stable:${43 + index}`],
    })
  ));
  const result = resolveGameAnimations({
    previews,
    state: {
      snapshot_id: 8,
      battlefield_transitions: previews.map((preview) => ({
        stable_id: preview.card.stable_id,
        kind: "destroyed",
      })),
    },
    previousCardRects: new Map(previews.map((preview, index) => [
      `stable:${preview.card.stable_id}`,
      { ...rect, left: index * 120 },
    ])),
  });

  assert.deepEqual(
    result.visualEffects
      .filter((effect) => effect.kind === "death-collapse")
      .map((effect) => effect.startDelayMs),
    [0, 90, 180, 270, 360, 450],
  );
});

test("death animation requires battlefield-to-graveyard state", () => {
  const previousCardRects = new Map([["stable:43", rect]]);
  const withWrongState = resolveGameAnimations({
    previews: [destroyPreview({ toZone: "exile", title: "Battlefield -> Exile" })],
    state: { snapshot_id: 6, battlefield_transitions: [{ stable_id: 43, kind: "destroyed" }] },
    previousCardRects,
  });
  assert.equal(withWrongState.visualEffects.length, 0);
  assert.equal(withWrongState.previews[0].animationKind, undefined);
});
