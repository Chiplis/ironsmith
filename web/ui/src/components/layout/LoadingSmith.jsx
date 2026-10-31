import { useEffect, useRef } from "react";

const CYCLE_MS = 1600;
const INTRO_MS = 960;
const MIN_HITS = 6;
const STRIKES = new Set(["strike", "grip", "mouth-shape", "brow-shape", "sparks", "head-effort"]);

// A shared clock keeps the hammer, expression, sparks and mask synchronized.
export default function LoadingSmith({ progress = 0, ready = false, onComplete }) {
  const objectRef = useRef(null);
  const startedRef = useRef(false);
  const inputs = useRef({ progress, ready, onComplete });
  useEffect(() => { inputs.current = { progress, ready, onComplete }; }, [progress, ready, onComplete]);

  useEffect(() => {
    if (!ready) return;
    // A missing/blocked decorative SVG must never prevent entering the game.
    if (matchMedia('(prefers-reduced-motion: reduce)').matches) {
      onComplete?.();
      return;
    }
    const timeout = setTimeout(() => {
      if (!startedRef.current) onComplete?.();
    }, 3000);
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
      startedRef.current = true;
      const motion = matchMedia('(prefers-reduced-motion: reduce)');
      const animations = doc.getAnimations();
      for (const animation of animations) {
        animation.pause();
        if (STRIKES.has(animation.animationName)) animation.effect.updateTiming({ iterations: Infinity });
      }
      let previous;
      let clock = 0;
      let speed = 1;
      let lastProgress = inputs.current.progress;
      let sampleTime;
      let rate = 0;
      let finishing = false;
      let outro = 0;
      const tick = now => {
        if (disposed) return;
        const delta = previous === undefined ? 0 : Math.min(now - previous, 100);
        previous = now;
        sampleTime ??= now;
        const { progress: currentProgress, ready: isReady, onComplete: complete } = inputs.current;
        if (now - sampleTime >= 250) {
          const measured = Math.max(0, currentProgress - lastProgress) / ((now - sampleTime) / 1000);
          rate = rate * 0.6 + measured * 0.4;
          lastProgress = currentProgress;
          sampleTime = now;
        }
        // Aim for six blows over the measured loading duration, with a
        // readable cadence even for stalled or cached downloads.
        const target = isReady ? 3 : Math.max(0.65, Math.min(3, rate * MIN_HITS * CYCLE_MS / 1000));
        speed += (target - speed) * (1 - Math.exp(-delta / 350));
        const before = clock;
        clock += delta * speed;
        const cycles = Math.max(0, (clock - INTRO_MS) / CYCLE_MS);
        if (!finishing && isReady && cycles >= MIN_HITS &&
            Math.floor(cycles) > Math.floor(Math.max(0, (before - INTRO_MS) / CYCLE_MS))) {
          finishing = true;
        }
        if (finishing) outro += delta * speed;
        for (const animation of animations) {
          // Keep the mask closed until the last complete hammer cycle.
          animation.currentTime = STRIKES.has(animation.animationName)
            ? (finishing ? INTRO_MS : clock)
            : (finishing ? 5760 + outro : Math.min(clock, INTRO_MS));
        }
        if ((motion.matches && isReady) || (finishing && outro >= 960)) {
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
      startedRef.current = false;
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
