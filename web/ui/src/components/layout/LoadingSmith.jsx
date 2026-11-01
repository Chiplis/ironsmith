import { useEffect, useRef } from "react";

const CYCLE_MS = 1600;
const INTRO_MS = 960;
const STRIKES = new Set(["strike", "grip", "mouth-shape", "brow-shape", "sparks", "head-effort"]);
const isStrike = name => STRIKES.has(name) || name.startsWith("visor-impact-") || name.startsWith("traveling-spark-");

// A shared clock keeps the hammer, expression, sparks and mask synchronized.
export default function LoadingSmith({ progress = 0, ready = false, onComplete }) {
  const objectRef = useRef(null);
  const completedRef = useRef(false);
  const inputs = useRef({ progress, ready, onComplete });
  useEffect(() => { inputs.current = { progress, ready, onComplete }; }, [progress, ready, onComplete]);

  useEffect(() => {
    if (!ready) return;
    // A missing, blocked, or failed decorative SVG must never gate a ready game.
    if (matchMedia('(prefers-reduced-motion: reduce)').matches) {
      onComplete?.();
      return;
    }
    const timeout = setTimeout(() => {
      if (!completedRef.current) {
        completedRef.current = true;
        onComplete?.();
      }
    }, 4500);
    return () => clearTimeout(timeout);
  }, [ready, onComplete]);

  useEffect(() => {
    const object = objectRef.current;
    let frame;
    let disposed = false;
    let started = false;
    const start = () => {
      if (started || disposed) return;
      const doc = object.contentDocument;
      if (!doc?.querySelector('.hammer-arm')) return;
      started = true;
      const motion = matchMedia('(prefers-reduced-motion: reduce)');
      const animations = doc.getAnimations();
      for (const animation of animations) {
        animation.pause();
        if (isStrike(animation.animationName)) animation.effect.updateTiming({ iterations: Infinity });
        if (animation.animationName === "forge-letter") {
          // The SVG's sampled contours have matching points. Interpolate
          // directly between the blank and finished glyph, without the old
          // three-strike holds baked into its CSS timeline.
          const frames = animation.effect.getKeyframes();
          // Safari can expose an empty list for unsupported SVG d animation.
          // Keep the SVG's static path instead of crashing the loading gate.
          if (frames.length >= 2 && frames[0].d && frames.at(-1).d) {
            animation.effect.setKeyframes([{ d: frames[0].d }, { d: frames.at(-1).d }]);
            animation.effect.updateTiming({ delay: 0, duration: 1, fill: "both" });
          }
        }
      }
      let previous;
      let clock = 0;
      let finishing = false;
      let outro = 0;
      let hits = 0;
      let letter = 0;
      let letterFrom = 0;
      let letterTarget = 0;
      let impactTime = 0;
      const tick = now => {
        if (disposed || completedRef.current) return;
        const delta = previous === undefined ? 0 : Math.min(now - previous, 100);
        previous = now;
        const { progress: currentProgress, ready: isReady, onComplete: complete } = inputs.current;
        // Keep the smith's rhythm steady; only the amount forged varies.
        const before = clock;
        clock += delta;
        const cycles = Math.max(0, (clock - INTRO_MS) / CYCLE_MS);
        const impactCount = Math.max(0, Math.floor(cycles - 0.44) + 1);
        if (!finishing && impactCount > hits) {
          hits = impactCount;
          letterFrom = letter;
          // Progress can jump on cached loads or regress between phases.
          // Never unforge the letter; the first hit after readiness finishes it.
          letterTarget = Math.max(letter,
            isReady ? 1 : Math.max(0, Math.min(0.95, currentProgress)),
          );
          impactTime = INTRO_MS + (hits - 1 + 0.44) * CYCLE_MS;
        }
        const morph = Math.max(0, Math.min(1, (clock - impactTime) / (CYCLE_MS * 0.2)));
        letter = letterFrom + (letterTarget - letterFrom) * morph * morph * (3 - 2 * morph);
        if (!finishing && isReady && letter >= 1 &&
            Math.floor(cycles) > Math.floor(Math.max(0, (before - INTRO_MS) / CYCLE_MS))) {
          finishing = true;
        }
        if (finishing) outro += delta;
        for (const animation of animations) {
          // Keep the mask closed until the last complete hammer cycle.
          animation.currentTime = animation.animationName === "forge-letter"
            ? letter
            : isStrike(animation.animationName)
              ? (finishing ? INTRO_MS : clock)
              : (finishing ? 5760 + outro : Math.min(clock, INTRO_MS));
        }
        if ((motion.matches && isReady) || (finishing && outro >= 960)) {
          completedRef.current = true;
          complete?.();
          return;
        }
        frame = requestAnimationFrame(tick);
      };
      frame = requestAnimationFrame(tick);
    };
    object.addEventListener('load', start);
    start();
    return () => {
      disposed = true;
      completedRef.current = false;
      cancelAnimationFrame(frame);
      object.removeEventListener('load', start);
    };
  }, []);

  return (
    <object ref={objectRef} className="game-loading-wordmark"
      data={`${import.meta.env.BASE_URL}theme/ironsmith-loading.svg#working`}
      type="image/svg+xml" role="img" aria-label="IronSmith" tabIndex={-1}>
      IronSmith
    </object>
  );
}
