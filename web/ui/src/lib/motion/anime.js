import { animate } from "animejs/animation";
import { createLayout } from "animejs/layout";
import { createScope } from "animejs/scope";
import { createTimeline } from "animejs/timeline";
import { createDrawable } from "animejs/svg";
import { spring } from "animejs/easings";
import { stagger } from "animejs/utils";

export { animate, createDrawable, createLayout, createScope, createTimeline, spring, stagger };

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
