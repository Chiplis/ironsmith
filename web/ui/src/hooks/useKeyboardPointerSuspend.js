import { useCallback, useEffect, useRef } from "react";

// Keyboard hand navigation owns selection until the pointer actually moves.
export default function useKeyboardPointerSuspend(onResume) {
  const suspended = useRef(false);
  const position = useRef(null);
  const resumeRef = useRef(onResume);
  useEffect(() => { resumeRef.current = onResume; }, [onResume]);

  useEffect(() => {
    const restore = () => {
      suspended.current = false;
      document.documentElement.removeAttribute("data-hand-pointer-suspended");
    };
    const move = (event) => {
      if (event.pointerType === "touch") return;
      const previous = position.current;
      position.current = { x: event.clientX, y: event.clientY };
      const moved = previous
        ? previous.x !== event.clientX || previous.y !== event.clientY
        : Boolean(event.movementX || event.movementY);
      if (suspended.current && moved) {
        restore();
        resumeRef.current();
      } else if (suspended.current) {
        event.stopImmediatePropagation();
      }
    };
    const block = (event) => {
      // Preserve keyboard-generated clicks and touch input.
      if (!suspended.current || event.pointerType === "touch"
        || (event.type === "click" && event.detail === 0)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
    };
    const events = ["mousemove", "pointerdown", "pointerup", "mousedown", "mouseup", "click", "dblclick", "contextmenu", "pointerover", "pointerout", "mouseover", "mouseout"];
    window.addEventListener("pointermove", move, true);
    events.forEach((name) => window.addEventListener(name, block, true));
    return () => {
      restore();
      window.removeEventListener("pointermove", move, true);
      events.forEach((name) => window.removeEventListener(name, block, true));
    };
  }, []);

  return useCallback(() => {
    suspended.current = true;
    document.documentElement.setAttribute("data-hand-pointer-suspended", "true");
  }, []);
}
