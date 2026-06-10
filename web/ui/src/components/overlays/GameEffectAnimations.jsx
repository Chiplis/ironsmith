import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { HIDDEN_CARD_BACK_IMAGE_URL } from "@/lib/scryfall";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { buildPlayerVitals, collectGameEffectEvents } from "@/lib/game-effect-events";
import { getZoneAnchorRect, rectCenter } from "@/lib/zone-anchors";
import { getPlayerTargetRect } from "@/hooks/useCardPositions";
import { createTimeline, cancelMotion } from "@/lib/motion/anime";

const FLIGHT_MS = 880;
const DRAW_FLIGHT_MS = 760;
const DRAW_STAGGER_MS = 110;
const FLIGHT_STAGGER_MS = 150;
const SHUFFLE_MS = 940;
const PEEK_RISE_MS = 300;
const PEEK_HOLD_MS = 1250;
const PEEK_SETTLE_MS = 280;
const LIFE_PULSE_MS = 1400;
const CLEANUP_TAIL_MS = 200;

const FLIGHT_CARD_WIDTH = 68;
const FLIGHT_CARD_HEIGHT = 95;

let nextEffectInstanceId = 0;

function flightArcOffset(fromCenter, toCenter) {
  const dx = toCenter.x - fromCenter.x;
  const dy = toCenter.y - fromCenter.y;
  const distance = Math.hypot(dx, dy) || 1;
  const arc = Math.min(110, Math.max(30, distance * 0.16));
  // Perpendicular to the flight path, biased upward so arcs read as a toss.
  let px = (-dy / distance) * arc;
  let py = (dx / distance) * arc;
  if (py > 0) {
    px = -px;
    py = -py;
  }
  return { dx, dy, px, py };
}

function useStableCallbackRef(callback) {
  const ref = useRef(callback);
  useLayoutEffect(() => {
    ref.current = callback;
  });
  return ref;
}

function CardFlight({ effect, onDone }) {
  const shellRef = useRef(null);
  const flipRef = useRef(null);
  const motionRef = useRef(null);
  const flipMotionRef = useRef(null);
  const onDoneRef = useStableCallbackRef(onDone);
  const startedAtRef = useRef(null);
  const artUrl = useScryfallImageUrl(effect.cardName || "", "normal");
  const showsFace = Boolean(effect.cardName) && Boolean(artUrl);
  const startsFaceDown = effect.revealsFace || !showsFace;

  useLayoutEffect(() => {
    const node = shellRef.current;
    if (!node) return undefined;

    startedAtRef.current = performance.now();
    const fromCenter = rectCenter(effect.fromRect);
    const toCenter = rectCenter(effect.toRect);
    const { dx, dy, px, py } = flightArcOffset(fromCenter, toCenter);
    const duration = effect.durationMs;
    const spin = effect.tumble ? (effect.seed % 2 === 0 ? 200 : -160) : (effect.seed % 2 === 0 ? 10 : -10);

    motionRef.current = createTimeline({ autoplay: true }).add(node, {
      keyframes: [
        { translateX: 0, translateY: 0, scale: 0.5, opacity: 0, rotate: 0, duration: 0 },
        { scale: 0.96, opacity: 1, duration: duration * 0.18, ease: "out(2)" },
        {
          translateX: dx * 0.55 + px,
          translateY: dy * 0.55 + py,
          scale: 1.04,
          rotate: spin * 0.55,
          duration: duration * 0.42,
          ease: "inOut(1.4)",
        },
        {
          translateX: dx,
          translateY: dy,
          scale: 0.5,
          rotate: spin,
          opacity: 0,
          duration: duration * 0.4,
          ease: "in(1.7)",
        },
      ],
      delay: effect.delayMs,
    });

    const timer = window.setTimeout(
      () => onDoneRef.current(),
      effect.delayMs + duration + CLEANUP_TAIL_MS
    );
    return () => {
      window.clearTimeout(timer);
      cancelMotion(motionRef.current);
    };
  }, [effect, onDoneRef]);

  // The face art can resolve asynchronously after the flight starts; the flip
  // is scheduled separately so a late-arriving image doesn't restart the
  // flight.
  useLayoutEffect(() => {
    const flipNode = flipRef.current;
    if (!flipNode || !effect.revealsFace || !showsFace) return undefined;

    const elapsed = startedAtRef.current == null ? 0 : performance.now() - startedAtRef.current;
    const flipAt = Math.max(0, effect.delayMs + effect.durationMs * 0.3 - elapsed);
    flipMotionRef.current = createTimeline({ autoplay: true }).add(flipNode, {
      rotateY: [0, 180],
      duration: effect.durationMs * 0.4,
      delay: flipAt,
      ease: "inOut(2)",
    });
    return () => {
      cancelMotion(flipMotionRef.current);
    };
  }, [effect, showsFace]);

  return (
    <div
      ref={shellRef}
      className="game-fx-flight"
      style={{
        left: rectCenter(effect.fromRect).x - FLIGHT_CARD_WIDTH / 2,
        top: rectCenter(effect.fromRect).y - FLIGHT_CARD_HEIGHT / 2,
        width: FLIGHT_CARD_WIDTH,
        height: FLIGHT_CARD_HEIGHT,
        opacity: 0,
      }}
    >
      <div
        ref={flipRef}
        className="game-fx-flight-flip"
        style={!startsFaceDown ? { transform: "rotateY(180deg)" } : undefined}
      >
        <img className="game-fx-flight-face game-fx-flight-back" src={HIDDEN_CARD_BACK_IMAGE_URL} alt="" />
        {showsFace && (
          <img
            className="game-fx-flight-face game-fx-flight-front"
            src={artUrl}
            alt=""
            referrerPolicy="no-referrer"
          />
        )}
      </div>
    </div>
  );
}

function LibraryPeek({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionsRef = useRef([]);
  const onDoneRef = useStableCallbackRef(onDone);

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;

    const cards = Array.from(root.querySelectorAll(".game-fx-peek-card"));
    const count = cards.length;
    const totalMs = PEEK_RISE_MS + PEEK_HOLD_MS + PEEK_SETTLE_MS + (count - 1) * 70;
    motionsRef.current = cards.map((card, index) => {
      const offset = index - (count - 1) / 2;
      const fan = offset * 16;
      const lift = 58 + Math.abs(offset) * -6;
      return createTimeline({ autoplay: true }).add(card, {
        keyframes: [
          { translateY: 0, translateX: 0, rotate: 0, opacity: 0, scale: 0.82, duration: 0 },
          {
            translateY: -lift,
            translateX: offset * 26,
            rotate: fan,
            opacity: 1,
            scale: 1,
            duration: PEEK_RISE_MS,
            ease: "out(2.2)",
          },
          { duration: PEEK_HOLD_MS },
          {
            translateY: 0,
            translateX: 0,
            rotate: 0,
            opacity: 0,
            scale: 0.84,
            duration: PEEK_SETTLE_MS,
            ease: "in(1.8)",
          },
        ],
        delay: index * 70,
      });
    });

    const label = root.querySelector(".game-fx-peek-label");
    if (label) {
      motionsRef.current.push(createTimeline({ autoplay: true }).add(label, {
        keyframes: [
          { opacity: 0, translateY: 6, duration: 0 },
          { opacity: 1, translateY: 0, duration: 240, ease: "out(2)" },
          { duration: PEEK_HOLD_MS },
          { opacity: 0, translateY: -4, duration: 240, ease: "in(2)" },
        ],
        delay: 60,
      }));
    }

    const timer = window.setTimeout(() => onDoneRef.current(), totalMs + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      for (const motion of motionsRef.current) cancelMotion(motion);
      motionsRef.current = [];
    };
  }, [effect, onDoneRef]);

  const center = rectCenter(effect.rect);
  return (
    <div
      ref={rootRef}
      className="game-fx-peek"
      style={{ left: center.x, top: effect.rect.top }}
    >
      <div className="game-fx-peek-label-wrap">
        <div className="game-fx-peek-label">{effect.label}</div>
      </div>
      {Array.from({ length: effect.count }, (_, index) => (
        <img
          key={index}
          className="game-fx-peek-card"
          src={HIDDEN_CARD_BACK_IMAGE_URL}
          alt=""
          style={{ zIndex: index }}
        />
      ))}
    </div>
  );
}

function ShuffleBurst({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionsRef = useRef([]);
  const onDoneRef = useStableCallbackRef(onDone);

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;

    const cards = Array.from(root.querySelectorAll(".game-fx-shuffle-card"));
    motionsRef.current = cards.map((card, index) => {
      const side = index % 2 === 0 ? -1 : 1;
      const spread = 14 + (index % 3) * 5;
      const tilt = side * (5 + (index % 3) * 3);
      return createTimeline({ autoplay: true }).add(card, {
        keyframes: [
          { translateX: 0, translateY: 0, rotate: 0, opacity: 0, duration: 0 },
          { translateX: side * spread, translateY: -4 - (index % 3) * 2, rotate: tilt, opacity: 1, duration: 150, ease: "out(2)" },
          { translateX: -side * (spread * 0.55), rotate: -tilt * 0.6, duration: 130, ease: "inOut(1.6)" },
          { translateX: side * (spread * 0.3), rotate: tilt * 0.35, duration: 120, ease: "inOut(1.6)" },
          { translateX: 0, translateY: 0, rotate: 0, duration: 170, ease: "out(2.4)" },
          { opacity: 0, duration: 180, ease: "in(2)" },
        ],
        delay: index * 26,
      });
    });

    const timer = window.setTimeout(() => onDoneRef.current(), SHUFFLE_MS + cards.length * 26 + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      for (const motion of motionsRef.current) cancelMotion(motion);
      motionsRef.current = [];
    };
  }, [effect, onDoneRef]);

  const center = rectCenter(effect.rect);
  return (
    <div ref={rootRef} className="game-fx-shuffle" style={{ left: center.x, top: center.y }}>
      {Array.from({ length: 6 }, (_, index) => (
        <img
          key={index}
          className="game-fx-shuffle-card"
          src={HIDDEN_CARD_BACK_IMAGE_URL}
          alt=""
          style={{ zIndex: index }}
        />
      ))}
    </div>
  );
}

function LifePulse({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionsRef = useRef([]);
  const onDoneRef = useStableCallbackRef(onDone);
  const gained = effect.delta > 0;

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;

    const text = root.querySelector(".game-fx-life-text");
    const ring = root.querySelector(".game-fx-life-ring");
    if (text) {
      motionsRef.current.push(createTimeline({ autoplay: true }).add(text, {
        keyframes: [
          { translateY: 6, scale: 0.7, opacity: 0, duration: 0 },
          { translateY: -10, scale: 1.12, opacity: 1, duration: 260, ease: "out(2.4)" },
          { translateY: -16, scale: 1, duration: 480 },
          { translateY: -34, opacity: 0, duration: 520, ease: "in(1.6)" },
        ],
      }));
    }
    if (ring) {
      motionsRef.current.push(createTimeline({ autoplay: true }).add(ring, {
        keyframes: [
          { scale: 0.4, opacity: 0.85, duration: 0 },
          { scale: 1.65, opacity: 0, duration: 700, ease: "out(2)" },
        ],
      }));
    }

    const timer = window.setTimeout(() => onDoneRef.current(), LIFE_PULSE_MS + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      for (const motion of motionsRef.current) cancelMotion(motion);
      motionsRef.current = [];
    };
  }, [effect, onDoneRef]);

  const center = rectCenter(effect.rect);
  return (
    <div
      ref={rootRef}
      className={`game-fx-life ${gained ? "game-fx-life--gain" : "game-fx-life--loss"}`}
      style={{ left: center.x, top: center.y }}
    >
      <div className="game-fx-life-ring" />
      <div className="game-fx-life-text-wrap">
        <div className="game-fx-life-text">
          {gained ? `+${effect.delta}` : `−${Math.abs(effect.delta)}`}
        </div>
      </div>
    </div>
  );
}

function buildSpawnedEffects(events, { perspective }) {
  const spawned = [];
  for (const event of events) {
    const isPerspective = String(perspective ?? "") === event.playerKey;
    if (event.type === "zone-flight") {
      const fromRect = getZoneAnchorRect(event.playerKey, event.fromZone, { isPerspective });
      const toRect = getZoneAnchorRect(event.playerKey, event.toZone, { isPerspective });
      if (!fromRect || !toRect) continue;
      spawned.push({
        component: "flight",
        id: `${event.id}:${nextEffectInstanceId += 1}`,
        fromRect,
        toRect,
        cardName: event.cardName,
        revealsFace: event.revealsFace,
        tumble: event.kind === "mill" || event.kind === "discard",
        durationMs: FLIGHT_MS,
        delayMs: (spawned.filter((effect) => effect.component === "flight").length) * FLIGHT_STAGGER_MS,
        seed: nextEffectInstanceId,
      });
    } else if (event.type === "draw-burst") {
      const fromRect = getZoneAnchorRect(event.playerKey, "library", { isPerspective });
      const toRect = getZoneAnchorRect(event.playerKey, "hand", { isPerspective });
      if (!fromRect || !toRect) continue;
      for (let index = 0; index < event.count; index += 1) {
        spawned.push({
          component: "flight",
          id: `${event.id}:${index}:${nextEffectInstanceId += 1}`,
          fromRect,
          toRect,
          cardName: "",
          revealsFace: false,
          tumble: false,
          durationMs: DRAW_FLIGHT_MS,
          delayMs: index * DRAW_STAGGER_MS,
          seed: nextEffectInstanceId,
        });
      }
    } else if (event.type === "shuffle") {
      const rect = getZoneAnchorRect(event.playerKey, "library", { isPerspective });
      if (!rect) continue;
      spawned.push({
        component: "shuffle",
        id: `${event.id}:${nextEffectInstanceId += 1}`,
        rect,
      });
    } else if (event.type === "library-peek") {
      const rect = getZoneAnchorRect(event.playerKey, "library", { isPerspective });
      if (!rect) continue;
      spawned.push({
        component: "peek",
        id: `${event.id}:${nextEffectInstanceId += 1}`,
        rect,
        count: event.count,
        label: event.label,
      });
    } else if (event.type === "life") {
      const rect = getPlayerTargetRect(event.playerKey);
      if (!rect) continue;
      spawned.push({
        component: "life",
        id: `${event.id}:${nextEffectInstanceId += 1}`,
        rect,
        delta: event.delta,
      });
    }
  }
  return spawned;
}

export default function GameEffectAnimations({ suspended = false }) {
  const { state } = useGame();
  const [effects, setEffects] = useState([]);
  const previousVitalsRef = useRef(null);
  const previousViewedSignatureRef = useRef("");
  const processedTransitionIdsRef = useRef(new Set());
  const reducedMotion = useMemo(
    () => typeof window !== "undefined"
      && Boolean(window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches),
    []
  );

  const removeEffect = useCallback((effectId) => {
    setEffects((current) => current.filter((effect) => effect.id !== effectId));
  }, []);

  useLayoutEffect(() => {
    if (!state) return;

    const vitals = buildPlayerVitals(state.players || []);
    const { events, viewedSignature } = collectGameEffectEvents({
      state,
      previousVitals: previousVitalsRef.current,
      vitals,
      processedTransitionIds: processedTransitionIdsRef.current,
      previousViewedSignature: previousViewedSignatureRef.current,
      frameToken: String(state.snapshot_id ?? performance.now()),
    });
    const firstFrame = previousVitalsRef.current === null;
    const playerSetChanged = !firstFrame
      && Object.keys(previousVitalsRef.current).join(",") !== Object.keys(vitals).join(",");
    previousVitalsRef.current = vitals;
    previousViewedSignatureRef.current = viewedSignature;

    if (suspended || firstFrame || playerSetChanged || reducedMotion || events.length === 0) return;

    const spawnedEffects = buildSpawnedEffects(events, { perspective: state.perspective });
    if (spawnedEffects.length === 0) return;
    setEffects((current) => [...current, ...spawnedEffects]);
  }, [reducedMotion, state, suspended]);

  // Debug/E2E hook: inject raw events through the same spawn path, e.g.
  // window.dispatchEvent(new CustomEvent("ironsmith:game-effect-animations-debug",
  //   { detail: { events: [{ type: "shuffle", id: "x", playerKey: "0" }] } }))
  const perspectiveRef = useStableCallbackRef(state?.perspective);
  useEffect(() => {
    const onDebugEvents = (event) => {
      const spawnedEffects = buildSpawnedEffects(
        Array.isArray(event?.detail?.events) ? event.detail.events : [],
        { perspective: perspectiveRef.current }
      );
      if (spawnedEffects.length === 0) return;
      setEffects((current) => [...current, ...spawnedEffects]);
    };
    window.addEventListener("ironsmith:game-effect-animations-debug", onDebugEvents);
    return () => {
      window.removeEventListener("ironsmith:game-effect-animations-debug", onDebugEvents);
    };
  }, [perspectiveRef]);

  // When the workspace unmounts mid-flight, drop everything.
  useEffect(() => () => setEffects([]), []);

  if (effects.length === 0) return null;

  return (
    <div className="game-fx-layer">
      {effects.map((effect) => {
        const onDone = () => removeEffect(effect.id);
        if (effect.component === "flight") {
          return <CardFlight key={effect.id} effect={effect} onDone={onDone} />;
        }
        if (effect.component === "peek") {
          return <LibraryPeek key={effect.id} effect={effect} onDone={onDone} />;
        }
        if (effect.component === "shuffle") {
          return <ShuffleBurst key={effect.id} effect={effect} onDone={onDone} />;
        }
        if (effect.component === "life") {
          return <LifePulse key={effect.id} effect={effect} onDone={onDone} />;
        }
        return null;
      })}
    </div>
  );
}
