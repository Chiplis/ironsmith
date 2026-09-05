import { useEffect, useLayoutEffect, useRef } from "react";

/** Slide retained hand wrappers without cloning, scaling, or fading the fan. */
export default function useHandReflow(rootRef, signature, disabled = false) {
  const positionsRef = useRef(new Map());
  const animationsRef = useRef(new Map());

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    const previous = positionsRef.current;
    const next = new Map();
    const animations = animationsRef.current;
    const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    for (const node of root.querySelectorAll(".hand-layout-item")) {
      const prior = previous.get(node);
      // Continue from the visible position if another draw interrupts the slide.
      const translation = animations.has(node) ? getComputedStyle(node).translate.split(" ").map(parseFloat) : [0, 0];
      animations.get(node)?.cancel();
      animations.delete(node);
      const rect = node.getBoundingClientRect();
      next.set(node, { x: rect.x, y: rect.y });
      if (!prior || disabled || reduceMotion) continue;
      const dx = prior.x + (translation[0] || 0) - rect.x;
      const dy = prior.y + (translation[1] || 0) - rect.y;
      if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
      const animation = node.animate([
        { translate: `${dx}px ${dy}px` },
        { translate: "0px 0px" },
      ], { duration: 220, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" });
      animations.set(node, animation);
      animation.onfinish = () => {
        if (animations.get(node) === animation) animations.delete(node);
      };
    }
    for (const [node, animation] of animations) {
      if (!next.has(node)) {
        animation.cancel();
        animations.delete(node);
      }
    }
    positionsRef.current = next;
  }, [disabled, rootRef, signature]);

  useEffect(() => () => {
    for (const animation of animationsRef.current.values()) animation.cancel();
    animationsRef.current.clear();
    positionsRef.current.clear();
  }, []);
}
