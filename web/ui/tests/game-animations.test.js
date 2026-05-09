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

test("destroy animation builds the angelic inspector flight for matching runtime events", () => {
  const result = resolveGameAnimations({
    previews: [destroyPreview()],
    state: { snapshot_id: 4, battlefield_transitions: [{ stable_id: 43, kind: "destroyed" }] },
    previousCardRects: new Map([["stable:43", rect]]),
  });

  assert.equal(result.previews[0].animationKind, "angelic-destroy");
  assert.equal(result.previews[0].inspectorShaderReveal, true);
  assert.equal(result.previews[0].animationStaggerMs, 0);
  assert.equal(result.previews[0].inspectorRevealScope, "inspector");
  assert.equal(result.previews[0].inspectorRevealDelayMs, 420);
  assert.deepEqual(
    result.visualEffects.map((effect) => [effect.id, effect.kind, effect.travelsToInspector]),
    [
      ["destroy-angelic:p0:stable:43", "angelic-destroy", true],
    ],
  );
  assert.equal(result.visualEffects[0].targetToken, "p0:stable:43");
  assert.equal(result.visualEffects[0].targetScope, "inspector");
});

test("destroy animation staggers angelic flights while preserving flight effect duration", () => {
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
    [0, 140, 280],
  );
  assert.equal(result.previews[0].inspectorRevealDelayMs, 700);
  assert.deepEqual(
    result.visualEffects.map((effect) => effect.startDelayMs),
    [0, 140, 280],
  );
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
    result.visualEffects.map((effect) => effect.startDelayMs),
    [0, 140, 280, 420, 560, 700],
  );
});

test("destroy animation requires both a destroyed runtime event and battlefield-to-graveyard state", () => {
  const previousCardRects = new Map([["stable:43", rect]]);
  const withoutEvent = resolveGameAnimations({
    previews: [destroyPreview()],
    state: { snapshot_id: 5, battlefield_transitions: [] },
    previousCardRects,
  });
  assert.equal(withoutEvent.visualEffects.length, 0);
  assert.equal(withoutEvent.previews[0].animationKind, undefined);

  const withWrongState = resolveGameAnimations({
    previews: [destroyPreview({ toZone: "exile", title: "Battlefield -> Exile" })],
    state: { snapshot_id: 6, battlefield_transitions: [{ stable_id: 43, kind: "destroyed" }] },
    previousCardRects,
  });
  assert.equal(withWrongState.visualEffects.length, 0);
  assert.equal(withWrongState.previews[0].animationKind, undefined);
});
