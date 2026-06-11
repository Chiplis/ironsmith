import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { HIDDEN_CARD_BACK_IMAGE_URL } from "@/lib/scryfall";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { buildPlayerVitals, collectGameEffectEvents } from "@/lib/game-effect-events";
import { getZoneAnchorRect, rectCenter } from "@/lib/zone-anchors";
import { getCardRect, getPlayerTargetRect } from "@/hooks/useCardPositions";
import { animate, createTimeline, cancelMotion } from "@/lib/motion/anime";

const FLIGHT_MS = 880;
const DRAW_FLIGHT_MS = 760;
const DRAW_STAGGER_MS = 110;
const FLIGHT_STAGGER_MS = 150;
const SHUFFLE_MS = 940;
const PEEK_RISE_MS = 300;
const PEEK_HOLD_MS = 1250;
const PEEK_SETTLE_MS = 280;
const LIFE_PULSE_MS = 1400;
const COIN_FLIP_MS = 1900;
const CONTROL_FLIGHT_MS = 1100;
const PHASE_GHOST_MS = 900;
const LEVEL_UP_MS = 1100;
const HUD_FLOURISH_MS = 1800;
const BOARD_TINT_MS = 2200;
const TURN_BANNER_MS = 1900;
const CARD_FLIP_MS = 1000;
const LIFE_SWAP_MS = 1300;
const MANA_MOTE_MS = 900;
const CLEANUP_TAIL_MS = 200;

const FLIGHT_CARD_WIDTH = 68;
const FLIGHT_CARD_HEIGHT = 95;

const MANA_MOTE_COLORS = {
  W: "#f6efce",
  U: "#7fb8ff",
  B: "#a07ec2",
  R: "#ff8a5e",
  G: "#7ed692",
  C: "#cfd6dd",
};

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

function elementRectByStableId(stableId) {
  if (stableId == null || typeof document === "undefined") return null;
  const id = String(stableId);
  for (const el of document.querySelectorAll(".game-card[data-stable-id], .game-card[data-member-stable-ids]")) {
    if (el.closest(".zone-move-effects-layer, .game-fx-layer")) continue;
    const members = String(el.getAttribute("data-member-stable-ids") || "").split(",").map((v) => v.trim());
    if (el.getAttribute("data-stable-id") === id || members.includes(id)) {
      const rect = el.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) return rect;
    }
  }
  return null;
}

function elementByStableId(stableId) {
  if (stableId == null || typeof document === "undefined") return null;
  const id = String(stableId);
  for (const el of document.querySelectorAll(".game-card[data-stable-id], .game-card[data-member-stable-ids]")) {
    if (el.closest(".zone-move-effects-layer, .game-fx-layer")) continue;
    const members = String(el.getAttribute("data-member-stable-ids") || "").split(",").map((v) => v.trim());
    if (el.getAttribute("data-stable-id") === id || members.includes(id)) return el;
  }
  return null;
}

function parseManaMotes(text, fallbackCount) {
  const pips = String(text || "").match(/\{([WUBRGC0-9]+)\}/gi) || [];
  const colors = pips
    .map((pip) => pip.replace(/[{}]/g, "").toUpperCase())
    .flatMap((symbol) => (
      /^[0-9]+$/.test(symbol)
        ? Array.from({ length: Math.min(Number(symbol), 4) }, () => "C")
        : [symbol]
    ))
    .filter((symbol) => MANA_MOTE_COLORS[symbol]);
  if (colors.length > 0) return colors.slice(0, 6);
  const count = Math.max(1, Math.min(Number(fallbackCount) || 1, 6));
  return Array.from({ length: count }, () => "C");
}

/* ----------------------------- flights ----------------------------- */

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
      className={`game-fx-flight ${effect.tint ? `game-fx-flight--${effect.tint}` : ""}`}
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

/* --------------------------- library FX ---------------------------- */

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

/* ----------------------------- HUD FX ------------------------------ */

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

function LifeSwap({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionsRef = useRef([]);
  const onDoneRef = useStableCallbackRef(onDone);

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;
    const orbs = Array.from(root.querySelectorAll(".game-fx-lifeswap-orb"));
    const a = rectCenter(effect.fromRect);
    const b = rectCenter(effect.toRect);
    const paths = [
      { from: a, to: b, lift: -70 },
      { from: b, to: a, lift: 70 },
    ];
    const motions = orbs.map((orb, index) => {
      const path = paths[index % paths.length];
      orb.style.left = `${path.from.x}px`;
      orb.style.top = `${path.from.y}px`;
      const dx = path.to.x - path.from.x;
      const dy = path.to.y - path.from.y;
      return createTimeline({ autoplay: true }).add(orb, {
        keyframes: [
          { translateX: 0, translateY: 0, opacity: 0, scale: 0.5, duration: 0 },
          { opacity: 1, scale: 1.05, duration: LIFE_SWAP_MS * 0.18, ease: "out(2)" },
          {
            translateX: dx * 0.5,
            translateY: dy * 0.5 + path.lift,
            duration: LIFE_SWAP_MS * 0.38,
            ease: "inOut(1.5)",
          },
          { translateX: dx, translateY: dy, scale: 0.55, opacity: 0, duration: LIFE_SWAP_MS * 0.44, ease: "in(1.6)" },
        ],
      });
    });
    motionsRef.current = motions;
    const timer = window.setTimeout(() => onDoneRef.current(), LIFE_SWAP_MS + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      for (const motion of motions) cancelMotion(motion);
    };
  }, [effect, onDoneRef]);

  return (
    <div ref={rootRef} className="game-fx-lifeswap">
      <div className="game-fx-lifeswap-orb" />
      <div className="game-fx-lifeswap-orb game-fx-lifeswap-orb--second" />
    </div>
  );
}

function HudFlourish({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionsRef = useRef([]);
  const onDoneRef = useStableCallbackRef(onDone);

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;
    const badge = root.querySelector(".game-fx-hud-badge");
    const ring = root.querySelector(".game-fx-hud-ring");
    if (badge) {
      motionsRef.current.push(createTimeline({ autoplay: true }).add(badge, {
        keyframes: [
          { translateY: 10, scale: 0.6, opacity: 0, duration: 0 },
          { translateY: -12, scale: 1.08, opacity: 1, duration: 320, ease: "out(2.6)" },
          { translateY: -16, scale: 1, duration: HUD_FLOURISH_MS - 1000 },
          { translateY: -30, opacity: 0, duration: 540, ease: "in(1.7)" },
        ],
      }));
    }
    if (ring) {
      motionsRef.current.push(createTimeline({ autoplay: true }).add(ring, {
        keyframes: [
          { scale: 0.3, opacity: 0.9, duration: 0 },
          { scale: 1.9, opacity: 0, duration: 820, ease: "out(2)" },
        ],
      }));
    }
    const motions = motionsRef.current;
    const timer = window.setTimeout(() => onDoneRef.current(), HUD_FLOURISH_MS + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      for (const motion of motions) cancelMotion(motion);
      motionsRef.current = [];
    };
  }, [effect, onDoneRef]);

  const center = rectCenter(effect.rect);
  return (
    <div
      ref={rootRef}
      className={`game-fx-hud game-fx-hud--${effect.flavor}`}
      style={{ left: center.x, top: effect.rect.top }}
    >
      <div className="game-fx-hud-ring" />
      <div className="game-fx-hud-badge">
        <span className="game-fx-hud-icon" aria-hidden="true" />
        <span className="game-fx-hud-label">{effect.label}</span>
      </div>
    </div>
  );
}

function CoinFlip({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionsRef = useRef([]);
  const onDoneRef = useStableCallbackRef(onDone);
  const isDie = effect.flavor === "die";

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;
    const chip = root.querySelector(".game-fx-coin-chip");
    const label = root.querySelector(".game-fx-coin-result");
    if (chip) {
      motionsRef.current.push(createTimeline({ autoplay: true }).add(chip, {
        keyframes: [
          { translateY: 18, rotateX: 0, scale: 0.5, opacity: 0, duration: 0 },
          { translateY: -46, rotateX: 720, scale: 1.05, opacity: 1, duration: 620, ease: "out(1.6)" },
          { translateY: -8, rotateX: isDie ? 1066 : 1080, scale: 1, duration: 430, ease: "in(1.4)" },
          { translateY: -14, rotateX: isDie ? 1080 : 1080, duration: 140, ease: "out(2)" },
          { translateY: -10, duration: 120, ease: "in(2)" },
          { duration: 380 },
          { opacity: 0, translateY: -26, duration: 240, ease: "in(2)" },
        ],
      }));
    }
    if (label) {
      motionsRef.current.push(createTimeline({ autoplay: true }).add(label, {
        keyframes: [
          { opacity: 0, scale: 0.7, duration: 0 },
          { duration: 1080 },
          { opacity: 1, scale: 1.06, duration: 200, ease: "out(2.4)" },
          { scale: 1, duration: 200 },
          { duration: 200 },
          { opacity: 0, duration: 220, ease: "in(2)" },
        ],
      }));
    }
    const motions = motionsRef.current;
    const timer = window.setTimeout(() => onDoneRef.current(), COIN_FLIP_MS + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      for (const motion of motions) cancelMotion(motion);
      motionsRef.current = [];
    };
  }, [effect, isDie, onDoneRef]);

  const center = rectCenter(effect.rect);
  return (
    <div
      ref={rootRef}
      className={`game-fx-coin game-fx-coin--${effect.flavor}`}
      style={{ left: center.x, top: effect.rect.top }}
    >
      <div className="game-fx-coin-chip">
        <span className="game-fx-coin-face">{isDie ? "⬡" : "◉"}</span>
      </div>
      <div className="game-fx-coin-result-wrap">
        <div className="game-fx-coin-result">{effect.label}</div>
      </div>
    </div>
  );
}

function ControlFlight({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionRef = useRef(null);
  const onDoneRef = useStableCallbackRef(onDone);
  const [toRect, setToRect] = useState(null);

  // The permanent re-renders on the other side of the board; wait two frames
  // for layout to settle before resolving its destination rect.
  useLayoutEffect(() => {
    let cancelled = false;
    const resolve = () => {
      if (cancelled) return;
      const rect = elementRectByStableId(effect.stableId);
      setToRect(rect || effect.fromRect);
    };
    const frame1 = requestAnimationFrame(() => requestAnimationFrame(resolve));
    return () => {
      cancelled = true;
      cancelAnimationFrame(frame1);
    };
  }, [effect]);

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node || !toRect) return undefined;
    const from = rectCenter(effect.fromRect);
    const to = rectCenter(toRect);
    const { dx, dy, px, py } = flightArcOffset(from, to);

    motionRef.current = createTimeline({ autoplay: true }).add(node, {
      keyframes: [
        { translateX: 0, translateY: 0, opacity: 0, scale: 0.7, duration: 0 },
        { opacity: 1, scale: 1, duration: CONTROL_FLIGHT_MS * 0.16, ease: "out(2)" },
        {
          translateX: dx * 0.55 + px,
          translateY: dy * 0.55 + py,
          duration: CONTROL_FLIGHT_MS * 0.42,
          ease: "inOut(1.5)",
        },
        { translateX: dx, translateY: dy, scale: 0.85, opacity: 0, duration: CONTROL_FLIGHT_MS * 0.42, ease: "in(1.6)" },
      ],
    });
    const arrivalEl = elementByStableId(effect.stableId);
    let arrivalTimer = null;
    if (arrivalEl) {
      arrivalTimer = window.setTimeout(() => {
        arrivalEl.classList.add("control-change-arrival");
        window.setTimeout(() => arrivalEl.classList.remove("control-change-arrival"), 900);
      }, CONTROL_FLIGHT_MS * 0.8);
    }
    const timer = window.setTimeout(() => onDoneRef.current(), CONTROL_FLIGHT_MS + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      if (arrivalTimer) window.clearTimeout(arrivalTimer);
      cancelMotion(motionRef.current);
    };
  }, [effect, onDoneRef, toRect]);

  return (
    <div
      ref={rootRef}
      className="game-fx-control-flight"
      style={{
        left: effect.fromRect.left,
        top: effect.fromRect.top,
        width: effect.fromRect.width,
        height: effect.fromRect.height,
        opacity: 0,
      }}
    >
      <div className="game-fx-control-card" />
      <div className="game-fx-control-trail" />
    </div>
  );
}

function PhaseGhost({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionRef = useRef(null);
  const onDoneRef = useStableCallbackRef(onDone);
  const [rect, setRect] = useState(effect.kind === "phase_out" ? effect.rect : null);

  useLayoutEffect(() => {
    if (effect.kind !== "phase_in") return undefined;
    let cancelled = false;
    const frame = requestAnimationFrame(() => requestAnimationFrame(() => {
      if (cancelled) return;
      setRect(elementRectByStableId(effect.stableId) || effect.rect || null);
    }));
    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
    };
  }, [effect]);

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node || !rect) return undefined;
    const out = effect.kind === "phase_out";
    motionRef.current = createTimeline({ autoplay: true }).add(node, {
      keyframes: out
        ? [
          { opacity: 0.9, scale: 1, duration: 0 },
          { opacity: 0.4, scale: 0.97, filter: "blur(2px)", duration: PHASE_GHOST_MS * 0.5, ease: "inOut(1.6)" },
          { opacity: 0, scale: 0.94, filter: "blur(5px)", duration: PHASE_GHOST_MS * 0.5, ease: "in(1.6)" },
        ]
        : [
          { opacity: 0, scale: 0.94, filter: "blur(5px)", duration: 0 },
          { opacity: 0.55, scale: 1.01, filter: "blur(1px)", duration: PHASE_GHOST_MS * 0.55, ease: "out(1.8)" },
          { opacity: 0, scale: 1, filter: "blur(0px)", duration: PHASE_GHOST_MS * 0.45, ease: "in(1.4)" },
        ],
    });
    const timer = window.setTimeout(() => onDoneRef.current(), PHASE_GHOST_MS + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      cancelMotion(motionRef.current);
    };
  }, [effect, onDoneRef, rect]);

  if (!rect) return null;
  return (
    <div
      ref={rootRef}
      className="game-fx-phase-ghost"
      style={{ left: rect.left, top: rect.top, width: rect.width, height: rect.height, opacity: 0 }}
    />
  );
}

function LevelUpRing({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionsRef = useRef([]);
  const onDoneRef = useStableCallbackRef(onDone);

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;
    const ring = root.querySelector(".game-fx-levelup-ring");
    const label = root.querySelector(".game-fx-levelup-label");
    if (ring) {
      motionsRef.current.push(createTimeline({ autoplay: true }).add(ring, {
        keyframes: [
          { scale: 0.4, opacity: 0.95, duration: 0 },
          { scale: 1.7, opacity: 0, duration: 760, ease: "out(2.2)" },
        ],
      }));
    }
    if (label) {
      motionsRef.current.push(createTimeline({ autoplay: true }).add(label, {
        keyframes: [
          { translateY: 8, opacity: 0, scale: 0.8, duration: 0 },
          { translateY: -8, opacity: 1, scale: 1.05, duration: 280, ease: "out(2.4)" },
          { translateY: -12, scale: 1, duration: 420 },
          { translateY: -24, opacity: 0, duration: 320, ease: "in(1.8)" },
        ],
      }));
    }
    const el = elementByStableId(effect.stableId);
    if (el) {
      motionsRef.current.push(animate(el, {
        keyframes: [
          { "--card-jolt-scale": 1.12, "--card-flash-brightness": 1.4, duration: 200, ease: "out(2)" },
          { "--card-jolt-scale": 1, "--card-flash-brightness": 1, duration: 520, ease: "out(3)" },
        ],
        onComplete: () => {
          el.style.removeProperty("--card-jolt-scale");
          el.style.removeProperty("--card-flash-brightness");
        },
      }));
    }
    const motions = motionsRef.current;
    const timer = window.setTimeout(() => onDoneRef.current(), LEVEL_UP_MS + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      for (const motion of motions) cancelMotion(motion);
      motionsRef.current = [];
    };
  }, [effect, onDoneRef]);

  const center = rectCenter(effect.rect);
  return (
    <div ref={rootRef} className="game-fx-levelup" style={{ left: center.x, top: center.y }}>
      <div className="game-fx-levelup-ring" />
      <div className="game-fx-levelup-label-wrap">
        <div className="game-fx-levelup-label">{effect.label}</div>
      </div>
    </div>
  );
}

function BoardTint({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionRef = useRef(null);
  const onDoneRef = useStableCallbackRef(onDone);

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node) return undefined;
    motionRef.current = createTimeline({ autoplay: true }).add(node, {
      keyframes: [
        { opacity: 0, duration: 0 },
        { opacity: 1, duration: BOARD_TINT_MS * 0.3, ease: "out(1.6)" },
        { duration: BOARD_TINT_MS * 0.3 },
        { opacity: 0, duration: BOARD_TINT_MS * 0.4, ease: "in(1.6)" },
      ],
    });
    const timer = window.setTimeout(() => onDoneRef.current(), BOARD_TINT_MS + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      cancelMotion(motionRef.current);
    };
  }, [effect, onDoneRef]);

  return (
    <div
      ref={rootRef}
      className={`game-fx-board-tint game-fx-board-tint--${effect.flavor}`}
      style={{ opacity: 0 }}
    >
      <div className="game-fx-board-tint-label">{effect.flavor === "night" ? "NIGHT" : "DAY"}</div>
    </div>
  );
}

function TurnBanner({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionRef = useRef(null);
  const onDoneRef = useStableCallbackRef(onDone);

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node) return undefined;
    motionRef.current = createTimeline({ autoplay: true }).add(node, {
      keyframes: [
        { translateY: 18, opacity: 0, scale: 0.85, duration: 0 },
        { translateY: 0, opacity: 1, scale: 1, duration: 360, ease: "out(2.6)" },
        { duration: TURN_BANNER_MS - 980 },
        { translateY: -22, opacity: 0, duration: 420, ease: "in(1.8)" },
      ],
    });
    const timer = window.setTimeout(() => onDoneRef.current(), TURN_BANNER_MS + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      cancelMotion(motionRef.current);
    };
  }, [effect, onDoneRef]);

  const center = rectCenter(effect.rect);
  return (
    <div ref={rootRef} className="game-fx-turn-banner" style={{ left: center.x, top: effect.rect.top }}>
      <span className="game-fx-turn-banner-text">{effect.label}</span>
    </div>
  );
}

function CardFlipFlash({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionsRef = useRef([]);
  const onDoneRef = useStableCallbackRef(onDone);

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;
    const sheen = root.querySelector(".game-fx-flip-sheen");
    if (sheen) {
      motionsRef.current.push(createTimeline({ autoplay: true }).add(sheen, {
        keyframes: [
          { rotateY: 0, opacity: 0, duration: 0 },
          { rotateY: 75, opacity: 0.95, duration: CARD_FLIP_MS * 0.4, ease: "in(1.7)" },
          { rotateY: 105, opacity: 0.95, duration: CARD_FLIP_MS * 0.08 },
          { rotateY: 180, opacity: 0, duration: CARD_FLIP_MS * 0.52, ease: "out(1.7)" },
        ],
      }));
    }
    const el = elementByStableId(effect.stableId);
    if (el) {
      motionsRef.current.push(animate(el, {
        keyframes: [
          { "--card-flash-brightness": 1.0, duration: CARD_FLIP_MS * 0.36 },
          { "--card-flash-brightness": 1.75, "--card-jolt-scale": 1.05, duration: CARD_FLIP_MS * 0.14, ease: "out(2)" },
          { "--card-flash-brightness": 1, "--card-jolt-scale": 1, duration: CARD_FLIP_MS * 0.5, ease: "out(2.4)" },
        ],
        onComplete: () => {
          el.style.removeProperty("--card-flash-brightness");
          el.style.removeProperty("--card-jolt-scale");
        },
      }));
    }
    const motions = motionsRef.current;
    const timer = window.setTimeout(() => onDoneRef.current(), CARD_FLIP_MS + CLEANUP_TAIL_MS);
    return () => {
      window.clearTimeout(timer);
      for (const motion of motions) cancelMotion(motion);
      motionsRef.current = [];
    };
  }, [effect, onDoneRef]);

  return (
    <div
      ref={rootRef}
      className="game-fx-flip"
      style={{
        left: effect.rect.left,
        top: effect.rect.top,
        width: effect.rect.width,
        height: effect.rect.height,
      }}
    >
      <div className="game-fx-flip-sheen" />
    </div>
  );
}

function ManaMotes({ effect, onDone }) {
  const rootRef = useRef(null);
  const motionsRef = useRef([]);
  const onDoneRef = useStableCallbackRef(onDone);

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;
    const motes = Array.from(root.querySelectorAll(".game-fx-mana-mote"));
    const from = rectCenter(effect.fromRect);
    const to = rectCenter(effect.toRect);
    const motions = motes.map((mote, index) => {
      const jitterX = ((index * 37) % 24) - 12;
      const jitterY = ((index * 53) % 18) - 9;
      mote.style.left = `${from.x + jitterX}px`;
      mote.style.top = `${from.y + jitterY}px`;
      const dx = to.x - (from.x + jitterX);
      const dy = to.y - (from.y + jitterY);
      return createTimeline({ autoplay: true }).add(mote, {
        keyframes: [
          { translateX: 0, translateY: 0, opacity: 0, scale: 0.4, duration: 0 },
          { opacity: 1, scale: 1, translateY: -10, duration: MANA_MOTE_MS * 0.25, ease: "out(2)" },
          {
            translateX: dx * 0.55,
            translateY: dy * 0.55 - 26,
            duration: MANA_MOTE_MS * 0.35,
            ease: "inOut(1.4)",
          },
          { translateX: dx, translateY: dy, opacity: 0, scale: 0.4, duration: MANA_MOTE_MS * 0.4, ease: "in(1.7)" },
        ],
        delay: index * 60,
      });
    });
    motionsRef.current = motions;
    const timer = window.setTimeout(
      () => onDoneRef.current(),
      MANA_MOTE_MS + motes.length * 60 + CLEANUP_TAIL_MS
    );
    return () => {
      window.clearTimeout(timer);
      for (const motion of motions) cancelMotion(motion);
    };
  }, [effect, onDoneRef]);

  return (
    <div ref={rootRef} className="game-fx-mana">
      {effect.colors.map((symbol, index) => (
        <span
          key={index}
          className="game-fx-mana-mote"
          style={{ "--mote-color": MANA_MOTE_COLORS[symbol] || MANA_MOTE_COLORS.C }}
        />
      ))}
    </div>
  );
}

/* --------------------------- orchestration ------------------------- */

function playDamageLunge(event) {
  const [sourceId, targetId] = event.stableIds;
  const sourceEl = elementByStableId(sourceId);
  if (!sourceEl) return;
  const sourceRect = sourceEl.getBoundingClientRect();
  const targetRect = targetId != null
    ? elementRectByStableId(targetId)
    : (event.playerKey !== "" ? getPlayerTargetRect(event.playerKey) : null);
  if (!targetRect) return;
  const from = rectCenter(sourceRect);
  const to = rectCenter(targetRect);
  const distance = Math.hypot(to.x - from.x, to.y - from.y) || 1;
  const lungeX = ((to.x - from.x) / distance) * 11;
  const lungeY = ((to.y - from.y) / distance) * 11;
  animate(sourceEl, {
    keyframes: [
      { "--card-jolt-x": "0px", "--card-jolt-y": "0px", duration: 60 },
      { "--card-jolt-x": `${-lungeX * 0.4}px`, "--card-jolt-y": `${-lungeY * 0.4}px`, duration: 90, ease: "out(2)" },
      { "--card-jolt-x": `${lungeX}px`, "--card-jolt-y": `${lungeY}px`, duration: 110, ease: "in(2.4)" },
      { "--card-jolt-x": "0px", "--card-jolt-y": "0px", duration: 280, ease: "out(3)" },
    ],
    onComplete: () => {
      sourceEl.style.removeProperty("--card-jolt-x");
      sourceEl.style.removeProperty("--card-jolt-y");
    },
  });
}

function flightSpawn(event, spawned, { isPerspective }) {
  const fromRect = getZoneAnchorRect(event.playerKey, event.fromZone, { isPerspective });
  let toRect = null;
  if (event.toZone === "battlefield" && event.objectId != null) {
    toRect = getCardRect(event.objectId);
  }
  if (!toRect) {
    toRect = getZoneAnchorRect(event.playerKey, event.toZone, { isPerspective });
  }
  if (!fromRect || !toRect) return;
  spawned.push({
    component: "flight",
    id: `${event.id}:${nextEffectInstanceId += 1}`,
    fromRect,
    toRect,
    cardName: event.cardName,
    revealsFace: event.revealsFace,
    tumble: event.kind === "mill" || event.kind === "discard",
    tint: event.kind === "reanimate" ? "necro" : null,
    durationMs: FLIGHT_MS,
    delayMs: (spawned.filter((effect) => effect.component === "flight").length) * FLIGHT_STAGGER_MS,
    seed: nextEffectInstanceId,
  });
}

const HUD_FLOURISH_LABELS = {
  monarch: { label: "The Monarch", flavor: "monarch" },
  initiative: { label: "The Initiative", flavor: "initiative" },
  emblem: { label: "Emblem", flavor: "emblem" },
};

function engineEffectSpawn(event, spawned, previousRectsByStableId) {
  if (event.kind === "damage") {
    playDamageLunge(event);
    return;
  }

  if (event.kind === "coin_flip" || event.kind === "die_roll") {
    const rect = getPlayerTargetRect(event.playerKey)
      || new DOMRect(window.innerWidth / 2 - 40, window.innerHeight * 0.3, 80, 40);
    const label = event.kind === "coin_flip"
      ? (event.text ? event.text.toUpperCase() : (event.value === 1 ? "WON" : "LOST"))
      : `${event.value ?? "?"}`;
    spawned.push({
      component: "coin",
      id: `${event.id}:${nextEffectInstanceId += 1}`,
      rect,
      flavor: event.kind === "die_roll" ? "die" : "coin",
      label,
    });
    return;
  }

  if (event.kind === "control_change") {
    const stableId = event.stableIds[0];
    const fromRect = previousRectsByStableId.get(String(stableId)) || elementRectByStableId(stableId);
    if (!fromRect) return;
    spawned.push({
      component: "control-flight",
      id: `${event.id}:${nextEffectInstanceId += 1}`,
      fromRect,
      stableId,
    });
    return;
  }

  if (event.kind === "phase_out" || event.kind === "phase_in") {
    for (const stableId of event.stableIds) {
      const rect = event.kind === "phase_out"
        ? (previousRectsByStableId.get(String(stableId)) || elementRectByStableId(stableId))
        : null;
      if (event.kind === "phase_out" && !rect) continue;
      spawned.push({
        component: "phase-ghost",
        id: `${event.id}:${stableId}:${nextEffectInstanceId += 1}`,
        kind: event.kind,
        rect,
        stableId,
      });
    }
    return;
  }

  if (event.kind === "transform" || event.kind === "turned_face_up") {
    const stableId = event.stableIds[0];
    const rect = elementRectByStableId(stableId) || previousRectsByStableId.get(String(stableId));
    if (!rect) return;
    spawned.push({
      component: "card-flip",
      id: `${event.id}:${nextEffectInstanceId += 1}`,
      rect,
      stableId,
    });
    return;
  }

  if (event.kind === "level_up") {
    const stableId = event.stableIds[0];
    const rect = elementRectByStableId(stableId) || previousRectsByStableId.get(String(stableId));
    if (!rect) return;
    spawned.push({
      component: "level-up",
      id: `${event.id}:${nextEffectInstanceId += 1}`,
      rect,
      stableId,
      label: (event.text || "Level Up").replace(/_/g, " "),
    });
    return;
  }

  if (HUD_FLOURISH_LABELS[event.kind]) {
    const rect = getPlayerTargetRect(event.playerKey);
    if (!rect) return;
    spawned.push({
      component: "hud-flourish",
      id: `${event.id}:${nextEffectInstanceId += 1}`,
      rect,
      ...HUD_FLOURISH_LABELS[event.kind],
    });
    return;
  }

  if (event.kind === "day_night") {
    spawned.push({
      component: "board-tint",
      id: `${event.id}:${nextEffectInstanceId += 1}`,
      flavor: event.text === "night" ? "night" : "day",
    });
    return;
  }

  if (event.kind === "extra_turn") {
    const rect = getPlayerTargetRect(event.playerKey)
      || new DOMRect(window.innerWidth / 2 - 60, 60, 120, 30);
    spawned.push({
      component: "turn-banner",
      id: `${event.id}:${nextEffectInstanceId += 1}`,
      rect,
      label: "EXTRA TURN",
    });
    return;
  }

  if (event.kind === "life_exchange") {
    const fromRect = getPlayerTargetRect(event.playerKey);
    const toRect = getPlayerTargetRect(event.otherPlayerKey);
    if (!fromRect || !toRect) return;
    spawned.push({
      component: "life-swap",
      id: `${event.id}:${nextEffectInstanceId += 1}`,
      fromRect,
      toRect,
    });
    return;
  }

  if (event.kind === "mana_added") {
    const stableId = event.stableIds[0];
    const fromRect = (stableId != null && elementRectByStableId(stableId))
      || getPlayerTargetRect(event.playerKey);
    const toRect = getPlayerTargetRect(event.playerKey);
    if (!fromRect || !toRect) return;
    spawned.push({
      component: "mana-motes",
      id: `${event.id}:${nextEffectInstanceId += 1}`,
      fromRect,
      toRect,
      colors: parseManaMotes(event.text, event.value),
    });
  }
}

function buildSpawnedEffects(events, { perspective, previousRectsByStableId }) {
  const spawned = [];
  for (const event of events) {
    const isPerspective = String(perspective ?? "") === event.playerKey;
    if (event.type === "zone-flight") {
      flightSpawn(event, spawned, { isPerspective });
    } else if (event.type === "engine-effect") {
      engineEffectSpawn(event, spawned, previousRectsByStableId);
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
          tint: null,
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

function collectRectsByStableId() {
  const rects = new Map();
  if (typeof document === "undefined") return rects;
  for (const el of document.querySelectorAll(".game-card[data-stable-id]")) {
    if (el.closest(".zone-move-effects-layer, .game-fx-layer")) continue;
    const rect = el.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) continue;
    const stableId = el.getAttribute("data-stable-id");
    if (stableId) rects.set(stableId, rect);
    for (const memberId of String(el.getAttribute("data-member-stable-ids") || "").split(",")) {
      const trimmed = memberId.trim();
      if (trimmed && !rects.has(trimmed)) rects.set(trimmed, rect);
    }
  }
  return rects;
}

const EFFECT_COMPONENTS = {
  flight: CardFlight,
  peek: LibraryPeek,
  shuffle: ShuffleBurst,
  life: LifePulse,
  "life-swap": LifeSwap,
  coin: CoinFlip,
  "control-flight": ControlFlight,
  "phase-ghost": PhaseGhost,
  "level-up": LevelUpRing,
  "hud-flourish": HudFlourish,
  "board-tint": BoardTint,
  "turn-banner": TurnBanner,
  "card-flip": CardFlipFlash,
  "mana-motes": ManaMotes,
};

export default function GameEffectAnimations({ suspended = false }) {
  const { state } = useGame();
  const [effects, setEffects] = useState([]);
  const previousVitalsRef = useRef(null);
  const previousViewedSignatureRef = useRef("");
  const processedTransitionIdsRef = useRef(new Set());
  const previousRectsByStableIdRef = useRef(new Map());
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

    const previousRectsByStableId = previousRectsByStableIdRef.current;
    // Re-snapshot card rects for the NEXT frame after the browser lays this
    // one out; control-change/phase-out need the pre-move rect.
    const rectFrame = requestAnimationFrame(() => {
      previousRectsByStableIdRef.current = collectRectsByStableId();
    });

    if (suspended || firstFrame || playerSetChanged || reducedMotion || events.length === 0) {
      return () => cancelAnimationFrame(rectFrame);
    }

    const spawnedEffects = buildSpawnedEffects(events, {
      perspective: state.perspective,
      previousRectsByStableId,
    });
    if (spawnedEffects.length > 0) {
      setEffects((current) => [...current, ...spawnedEffects]);
    }
    return () => cancelAnimationFrame(rectFrame);
  }, [reducedMotion, state, suspended]);

  // Debug/E2E hook: inject raw events through the same spawn path, e.g.
  // window.dispatchEvent(new CustomEvent("ironsmith:game-effect-animations-debug",
  //   { detail: { events: [{ type: "shuffle", id: "x", playerKey: "0" }] } }))
  const perspectiveRef = useStableCallbackRef(state?.perspective);
  useEffect(() => {
    const onDebugEvents = (event) => {
      const spawnedEffects = buildSpawnedEffects(
        Array.isArray(event?.detail?.events) ? event.detail.events : [],
        {
          perspective: perspectiveRef.current,
          previousRectsByStableId: previousRectsByStableIdRef.current,
        }
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
        const Component = EFFECT_COMPONENTS[effect.component];
        if (!Component) return null;
        return <Component key={effect.id} effect={effect} onDone={() => removeEffect(effect.id)} />;
      })}
    </div>
  );
}
