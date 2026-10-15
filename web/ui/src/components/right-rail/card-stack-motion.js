// Choreography for CardStackPile: how each entry arrives on the stack and how
// it leaves, chosen from what put it there and where it goes. The light is
// drawn by lib/motion/stack-fx.js; the card itself moves with anime.js on its
// .card-stack-motion node (which owns that node's transform).

import { cancelMotion, createTimeline, spring } from "@/lib/motion/anime";
import {
  burst,
  chargePulse,
  launchComet,
  prefersReducedMotion,
  shockwave,
  sigil,
  ZONE_ACCENTS,
} from "@/lib/motion/stack-fx";
import { castOrigin } from "@/lib/stack-cast-origins";
import { getZoneAnchorRect } from "@/lib/zone-anchors";
import { getPlayerTargetRect } from "@/hooks/useCardPositions";
import { samePlayerId } from "@/lib/player-display";

const MATERIALIZE_MS = 1100;
const ARRIVAL_WATCHDOG_MS = 3500;
export const PILE_EXIT_MS = 620;
const BATTLEFIELD_IMPACT_REVEAL_MS = 120;

// Keep the destination measurable while the star is in flight. A rule also
// covers cards mounted/replaced during that flight, without owning their
// inline styles. The flight outlives the departing stack entry.
function holdBattlefieldReveal(objectId) {
  if (objectId == null) return () => {};
  const style = document.createElement("style");
  style.textContent = `.game-card[data-object-id="${CSS.escape(String(objectId))}"]:not([data-card-stack-pile] *) { visibility: hidden !important; }`;
  document.head.appendChild(style);
  const reveal = () => {
    window.clearTimeout(watchdog);
    style.remove();
  };
  const watchdog = window.setTimeout(reveal, ARRIVAL_WATCHDOG_MS);
  return reveal;
}

const PLAYER_ZONES = [
  ["battlefield", "battlefield"],
  ["hand", "hand_cards"],
  ["graveyard", "graveyard_cards"],
  ["exile", "exile_cards"],
  ["command", "command_cards"],
];

function zoneName(value) {
  return String(value || "").trim().toLowerCase();
}

function zoneOfObject(state, objectId) {
  if (objectId == null) return null;
  const needle = String(objectId);
  for (const player of state?.players || []) {
    for (const [zone, key] of PLAYER_ZONES) {
      const cards = player?.[key] || [];
      if (cards.some((card) => String(card?.id) === needle
        || (card?.member_ids || []).some((id) => String(id) === needle))) {
        return { zone, playerId: player.id };
      }
    }
  }
  return null;
}

function stackTransitions(state, entry, direction) {
  const stableId = entry?.stable_id;
  if (stableId == null) return null;
  return (state?.zone_transitions || []).find((transition) => (
    String(transition?.stable_id) === String(stableId)
    && zoneName(direction === "in" ? transition.to_zone : transition.from_zone) === "stack"
  )) || null;
}

/** A visible card for an object, outside the pile. */
function objectRect(objectId) {
  if (objectId == null) return null;
  for (const node of document.querySelectorAll(`.game-card[data-object-id="${objectId}"]`)) {
    if (node.closest("[data-card-stack-pile]")) continue;
    const rect = node.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) return rect;
  }
  return null;
}

function zoneRect(state, playerId, zone) {
  return getZoneAnchorRect(playerId, zone, { isPerspective: samePlayerId(playerId, state?.perspective) });
}

function isCycling(entry) {
  return /^\s*(\w+\s*)?cycling\b/i.test(String(entry?.source_ability_text || entry?.ability_text || ""));
}

/**
 * What put this entry on the stack, as a launch plan:
 * { from, style, accentRgb, preamble: () => Promise, copyOf }.
 */
async function planArrival({ entry, state, inspectId, owner, pileNode }) {
  const controller = entry.controller ?? owner;
  if (entry.ability_kind) {
    const kind = zoneName(entry.ability_kind);
    const sourceZone = zoneOfObject(state, inspectId);
    const sourceRect = sourceZone?.zone === "battlefield" ? objectRect(inspectId) : null;
    if (kind === "activated") {
      if (isCycling(entry)) {
        // The cycled card left the hand as the cost: it loops out of the hand.
        return { from: zoneRect(state, controller, "hand"), style: "spiral", accentRgb: ZONE_ACCENTS.graveyard };
      }
      if (sourceRect) {
        return { from: sourceRect, style: "bolt", preamble: (rgb) => chargePulse({ rect: sourceRect, rgb }) };
      }
      const zone = sourceZone?.zone || "hand";
      return { from: zoneRect(state, sourceZone?.playerId ?? controller, zone), style: "bolt", accentRgb: ZONE_ACCENTS[zone] };
    }
    // Triggered (and anything else the engine puts there on its own).
    const from = sourceRect
      || (sourceZone ? zoneRect(state, sourceZone.playerId, sourceZone.zone) : null)
      || getPlayerTargetRect(controller);
    return {
      from,
      style: "wisp",
      accentRgb: sourceZone && sourceZone.zone !== "battlefield" ? ZONE_ACCENTS[sourceZone.zone] : null,
      preamble: from ? (rgb) => sigil({ rect: from, rgb }) : null,
    };
  }

  const cast = stackTransitions(state, entry, "in");
  const fromZone = zoneName(cast?.from_zone);
  if (fromZone && fromZone !== "hand" && fromZone !== "stack") {
    // Flashback, foretell, cascade, a commander: from that zone, in its light.
    return { from: zoneRect(state, cast.owner ?? owner, fromZone), style: "star", accentRgb: ZONE_ACCENTS[fromZone] };
  }
  if (!cast && pileNode) {
    // No card moved: a copy. It splits off the spell it copies.
    const original = [...pileNode.querySelectorAll(".card-stack-slot:not([data-leaving]) .stack-card--frame")]
      .find((node) => node.dataset.cardName === entry.name && String(node.dataset.objectId) !== String(entry.id));
    if (original) return { copyOf: original.getBoundingClientRect() };
  }
  const origin = await castOrigin(entry.name);
  const from = origin?.point || origin?.rect || zoneRect(state, controller, "hand");
  return { from, style: "star", accentRgb: ZONE_ACCENTS.hand };
}

function nodeCenterRect(node) {
  return () => node.getBoundingClientRect();
}

/** The frame ignites where the comet struck: radial reveal, flash, settle. */
export function materialize(node, rgb, { subtle = false } = {}) {
  node.style.opacity = "";
  if (prefersReducedMotion()) return null;
  const rect = node.getBoundingClientRect();
  node.dataset.arriving = "true";
  node.dataset.materialize = "true";
  window.setTimeout(() => {
    delete node.dataset.arriving;
    delete node.dataset.materialize;
  }, MATERIALIZE_MS);
  shockwave({ at: rect, rgb, radius: Math.max(rect.width, rect.height) * (subtle ? 0.5 : 0.75) });
  burst({ at: { x: rect.left + rect.width / 2, y: rect.top + rect.height * 0.42 }, rgb, kind: "impact", count: subtle ? 18 : 46 });
  return createTimeline({
    autoplay: true,
    onComplete: () => {
      node.style.transform = "";
    },
  }).add(node, {
    scale: [subtle ? 1.05 : 1.16, 1],
    rotate: [subtle ? 0 : -2.5, 0],
    duration: 620,
    ease: spring({ duration: 620, bounce: 0.34 }),
  });
}

/**
 * Plays an entry's arrival. Returns a cancel function. The card stays hidden
 * until its comet lands -- for a targeted spell still being dragged, that is
 * after the release.
 */
// Occurrences that already made their entrance. The pile can remount (it
// steps aside for trigger ordering); a card already there just reappears.
const arrived = new Set();

/** Called when the stack empties: occurrence keys may then be reused. */
export function forgetArrivals() {
  arrived.clear();
}

export function playArrival({ node, entry, state, inspectId, owner, rgb, pileNode, occurrenceKey, onMotion }) {
  let cancelled = false;
  const key = occurrenceKey ?? entry.__timeline_key;
  if (arrived.has(key)) {
    // Already made its entrance (a re-issued entry, a remounted pile).
    node.style.opacity = "";
    return () => {
      cancelled = true;
    };
  }
  arrived.add(key);
  if (prefersReducedMotion()) {
    node.style.opacity = "";
    return () => {
      cancelled = true;
    };
  }
  let watchdog = null;
  node.style.opacity = "0";
  (async () => {
    const plan = await planArrival({ entry, state, inspectId, owner, pileNode });
    if (cancelled) return;
    // Once it is launching, the card shows within a few seconds whatever
    // happens to the light on its way.
    watchdog = window.setTimeout(() => {
      if (!cancelled && node.style.opacity === "0") node.style.opacity = "";
    }, ARRIVAL_WATCHDOG_MS);
    if (plan.copyOf) {
      const rect = node.getBoundingClientRect();
      burst({ at: plan.copyOf, rgb, kind: "sparks", count: 24 });
      node.style.opacity = "";
      onMotion(createTimeline({
        autoplay: true,
        onComplete: () => {
          node.style.transform = "";
          materialize(node, rgb, { subtle: true });
        },
      }).add(node, {
        x: [plan.copyOf.left - rect.left, 0],
        y: [plan.copyOf.top - rect.top, 0],
        opacity: [0.35, 1],
        scale: [1, 1],
        duration: 520,
        ease: spring({ duration: 520, bounce: 0.2 }),
      }));
      return;
    }
    if (plan.preamble) await plan.preamble(rgb);
    if (cancelled) return;
    if (plan.style === "star" || plan.style === "spiral") {
      // A flash where the card was let go (or the zone it leaves).
      shockwave({ at: plan.from, rgb, radius: 46, duration: 420 });
      burst({ at: plan.from, rgb, accentRgb: plan.accentRgb, kind: "impact", count: 22 });
    }
    await launchComet({ from: plan.from, to: nodeCenterRect(node), rgb, accentRgb: plan.accentRgb, style: plan.style });
    if (cancelled) return;
    onMotion(materialize(node, rgb));
  })().catch((error) => {
    // A failed flourish must never leave the card invisible.
    console.error("[card stack] arrival animation failed", error);
    if (!cancelled) node.style.opacity = "";
  });
  return () => {
    cancelled = true;
    window.clearTimeout(watchdog);
  };
}

/** Plays an entry's departure, chosen by where its card went. */
export function playExit({ node, entry, state, inspectId, owner, rgb, onMotion }) {
  node.style.opacity = node.style.opacity === "0" ? "0" : "";
  const rect = node.getBoundingClientRect();
  const visible = node.style.opacity !== "0" && rect.width > 0;
  if (prefersReducedMotion() || !visible) {
    onMotion(createTimeline({ autoplay: true }).add(node, { opacity: [visible ? 1 : 0, 0], duration: 160 }));
    return;
  }
  const countered = (state?.effect_events || []).some((event) => (
    event?.kind === "spell_countered"
    && (event.stable_ids || []).some((id) => String(id) === String(entry.stable_id))
  ));
  const moved = stackTransitions(state, entry, "out");
  const toZone = zoneName(moved?.to_zone);

  if (countered) {
    // The board's WebGL counter-shatter takes over this card's likeness;
    // the pile adds the breaking glass and lets the card go at once.
    burst({ rect, rgb: "255, 120, 110", accentRgb: rgb, kind: "shards", count: 38 });
    shockwave({ at: rect, rgb: "255, 120, 110", radius: rect.width * 0.9 });
    onMotion(createTimeline({ autoplay: true }).add(node, {
      opacity: [1, 0],
      scale: [1, 0.96],
      rotate: [0, -3],
      duration: 160,
      ease: "out(2)",
    }));
    return;
  }

  if (toZone === "battlefield") {
    // Collapse into a star that lands on the new permanent.
    const reveal = holdBattlefieldReveal(moved?.new_object_id);
    const target = () => objectRect(moved?.new_object_id) || zoneRect(state, moved?.controller ?? owner, "battlefield");
    window.setTimeout(async () => {
      try {
        const point = await launchComet({ from: rect, to: target, rgb, style: "star", size: 1.15 });
        if (!point) return;
        shockwave({ at: point, rgb, radius: 70 });
        burst({ at: point, rgb, kind: "impact", count: 30 });
        // Let the explosion bloom before the permanent becomes visible.
        await new Promise((resolve) => window.setTimeout(resolve, BATTLEFIELD_IMPACT_REVEAL_MS));
      } catch (error) {
        console.error("[card stack] battlefield arrival animation failed", error);
      } finally {
        reveal();
      }
    }, 160);
    onMotion(createTimeline({ autoplay: true }).add(node, {
      scale: [1, 1.06, 0.18],
      opacity: [1, 1, 0],
      duration: 360,
      ease: "in(3)",
    }));
    return;
  }

  if (toZone === "graveyard" || toZone === "exile") {
    // Burns away: embers rise off the face, the last spark drifts to its pile.
    const accent = ZONE_ACCENTS[toZone];
    burst({ rect, rgb, accentRgb: toZone === "exile" ? accent : "255, 170, 90", kind: "embers" });
    window.setTimeout(() => {
      launchComet({
        from: rect,
        to: () => zoneRect(state, moved?.owner ?? owner, toZone),
        rgb: accent,
        style: "spark",
        duration: 520,
      });
    }, 220);
    node.dataset.burning = "true";
    onMotion(createTimeline({ autoplay: true }).add(node, {
      opacity: [1, 1, 0],
      y: [0, -12],
      scale: [1, 1.03],
      duration: PILE_EXIT_MS - 40,
      ease: "in(2)",
    }));
    return;
  }

  // An ability (or a copy) simply finishes: it breaks into sparks, and a
  // mote returns to the permanent it came from.
  burst({ rect, rgb, kind: "sparks" });
  const sourceRect = entry.ability_kind ? objectRect(inspectId) : null;
  if (sourceRect) {
    window.setTimeout(() => {
      launchComet({ from: rect, to: sourceRect, rgb, style: "spark", duration: 420 })
        .then((point) => point && shockwave({ at: point, rgb, radius: 42, duration: 420 }));
    }, 120);
  }
  onMotion(createTimeline({ autoplay: true }).add(node, {
    opacity: [1, 0],
    scale: [1, 0.9],
    duration: 420,
    ease: "in(2)",
  }));
}

/** The entry the chevrons (or a click) brought into view springs forward. */
export function playViewPop(node) {
  if (!node || prefersReducedMotion() || node.dataset.arriving || node.style.opacity === "0") return null;
  return createTimeline({
    autoplay: true,
    onComplete: () => {
      node.style.transform = "";
    },
  }).add(node, {
    y: [0, -16, 0],
    scale: [1, 1.07, 1],
    duration: 460,
    ease: spring({ duration: 460, bounce: 0.3 }),
  });
}

/** The card left on top after the one above it resolved: it rises to meet it. */
export function playPromotion(node, rgb) {
  if (!node || prefersReducedMotion() || node.style.opacity === "0") return;
  const rect = node.getBoundingClientRect();
  shockwave({ at: rect, rgb, radius: rect.width * 0.7, duration: 600, width: 2 });
  node.dataset.promoted = "true";
  window.setTimeout(() => {
    delete node.dataset.promoted;
  }, 900);
}

export { cancelMotion };
