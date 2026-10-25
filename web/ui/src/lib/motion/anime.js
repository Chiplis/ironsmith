// Easing helpers only. Playback lives in native-effects/compositor; importing
// this module must not bring the Anime.js frame engine into the UI bundle.
import { spring } from "animejs/easings";
import { stagger } from "animejs/utils";
export { spring, stagger };

export function uiSpring(options = {}) {
  const { duration = 440, bounce = 0.18, ...rest } = options;
  return spring({ duration, bounce, ...rest });
}

export function snappySpring(options = {}) {
  const { duration = 320, bounce = 0.14, ...rest } = options;
  return spring({ duration, bounce, ...rest });
}

export function cancelMotion(animation) {
  if (!animation) return;
  if (typeof animation.cancel === "function") {
    animation.cancel();
  } else if (typeof animation.pause === "function") {
    animation.pause();
  }
}
