import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";

const STACK_ALERT_FADE_MS = 180;

export default function PlayerStackAlert({ visible, className = "" }) {
  const [shouldRender, setShouldRender] = useState(visible);
  const [isShown, setIsShown] = useState(visible);
  const hideTimeoutRef = useRef(null);

  useEffect(() => {
    if (hideTimeoutRef.current != null) {
      window.clearTimeout(hideTimeoutRef.current);
      hideTimeoutRef.current = null;
    }

    if (visible) {
      setShouldRender(true);
      const rafId = window.requestAnimationFrame(() => {
        setIsShown(true);
      });
      return () => window.cancelAnimationFrame(rafId);
    }

    setIsShown(false);
    hideTimeoutRef.current = window.setTimeout(() => {
      setShouldRender(false);
      hideTimeoutRef.current = null;
    }, STACK_ALERT_FADE_MS);

    return () => {
      if (hideTimeoutRef.current != null) {
        window.clearTimeout(hideTimeoutRef.current);
        hideTimeoutRef.current = null;
      }
    };
  }, [visible]);

  useEffect(() => () => {
    if (hideTimeoutRef.current != null) {
      window.clearTimeout(hideTimeoutRef.current);
      hideTimeoutRef.current = null;
    }
  }, []);

  if (!shouldRender) return null;

  return (
    <span
      className={cn(
        "player-stack-alert",
        isShown ? "player-stack-alert-visible" : "player-stack-alert-hidden",
        className
      )}
      aria-label="This player has something on the stack"
      title="This player has something on the stack"
    >
      <svg
        viewBox="0 0 16 16"
        aria-hidden="true"
        focusable="false"
        className="player-stack-alert-icon"
      >
        <path d="M8 3.25v5.6" />
        <path d="M8 12.1h0.01" />
      </svg>
    </span>
  );
}
