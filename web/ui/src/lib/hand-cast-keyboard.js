import { battlefieldPlacementForDrag } from "./battlefield-layout.js";
import { handCardSourcePoint, plainRect, shouldBeginTargetCastIntent } from "./hand-drag-intent.js";

/** A keyboard cast leaves the hand through the same request the pointer uses. */
export const HAND_KEYBOARD_CAST_EVENT = "ironsmith:hand-card-keyboard-cast";

/**
 * What the activation key should do for a hand card the player has selected.
 *
 * A pointer gesture aims before it commits, so a drag can decide everything at
 * the release point. A key press has aimed at nothing yet, so the card's own
 * play options decide what comes next:
 *  - `choose`: several ways to play it, so the picker resolves that first;
 *  - `target`: one targeted cast, which hands the mouse to the engine's
 *    targeting arrow as soon as the spell is on the stack;
 *  - `place`: one permanent or land, whose battlefield slot follows the mouse;
 *  - `cast`: nothing left to aim, so cast it and let payment take over.
 */
export function handKeyboardCastPlan({ actions, card } = {}) {
  const plays = (Array.isArray(actions) ? actions : []).filter(Boolean);
  if (plays.length === 0) return null;
  if (plays.length > 1) return { kind: "choose", actions: plays, action: null };
  const [action] = plays;
  if (shouldBeginTargetCastIntent(plays)) return { kind: "target", actions: plays, action };
  if (battlefieldPlacementForDrag({ actions: plays, card })) return { kind: "place", actions: plays, action };
  return { kind: "cast", actions: plays, action };
}

/** Only a battlefield slot still needs the pointer once the key is pressed. */
export function handKeyboardCastNeedsPointer(plan) {
  return plan?.kind === "place";
}

/**
 * Hold the card the way a drag holds it, anchored on the card itself rather
 * than a pointer that never went down. The hand container is given no extent,
 * so the gesture counts as having left the hand from its first frame and the
 * placement preview tracks the mouse immediately.
 */
export function keyboardPlacementDragArgs({ card, actions, glowKind, rect, viewport = globalThis }) {
  const anchor = plainRect(rect);
  const x = anchor ? anchor.left + (anchor.width / 2) : (Number(viewport?.innerWidth) || 0) / 2;
  const y = anchor ? anchor.top + (anchor.height / 2) : (Number(viewport?.innerHeight) || 0) / 2;
  return [
    card?.id,
    card?.name,
    actions,
    glowKind,
    x,
    y,
    anchor,
    { ...card, id: card?.id, name: card?.name },
    { left: 0, top: 0, right: 0, bottom: 0 },
    handCardSourcePoint(anchor),
    { keyboard: true },
  ];
}
