// Animate complete transform strings, not CSS variables: the browser can run
// transform/opacity interpolation on the compositor without a JS frame loop.
export function compositorEase(ease) {
  // Sample only once at setup. CSS linear() retains the existing spring/power
  // curves while letting the browser interpolate them throughout playback.
  return `linear(${Array.from({ length: 101 }, (_, i) => ease(i / 100).toFixed(6)).join(',')})`;
}

export function powerEase(direction, power) {
  return compositorEase((t) => direction === 'in' ? t ** power
    : direction === 'out' ? 1 - (1 - t) ** power
      : t < 0.5 ? (2 * t) ** power / 2 : 1 - (2 * (1 - t)) ** power / 2);
}

export function animateCompositor(node, keyframes, { onComplete, commit = true, ...timing }) {
  const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const animation = node.animate(keyframes, { fill: 'both', easing: 'linear', ...timing,
    ...(reduced ? { duration: 0, delay: 0, iterations: 1 } : {}),
  });
  animation.onfinish = () => {
    // Keep the final pose without leaving finished animations attached. Commit
    // before the callback so it can clear transforms or start the next motion.
    try {
      if (commit && node.isConnected) animation.commitStyles();
    } catch (error) {
      // A parent can hide the element before the finish event is delivered.
      // commitStyles rejects non-rendered targets; cleanup must still run.
      if (error.name !== 'InvalidStateError') throw error;
    } finally {
      animation.cancel();
      onComplete?.();
    }
  };
  return animation;
}
