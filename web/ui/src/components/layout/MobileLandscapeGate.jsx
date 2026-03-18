import { useCallback, useEffect, useMemo } from "react";
import useViewportLayout from "@/hooks/useViewportLayout";

function isStandaloneDisplayMode() {
  if (typeof window === "undefined") return false;
  const matchStandalone = typeof window.matchMedia === "function"
    && (
      window.matchMedia("(display-mode: standalone)").matches
      || window.matchMedia("(display-mode: fullscreen)").matches
    );
  return matchStandalone || window.navigator.standalone === true;
}

export default function MobileLandscapeGate() {
  const {
    portraitCompactViewport,
    nonDesktopViewport,
  } = useViewportLayout();
  const standaloneMode = useMemo(() => isStandaloneDisplayMode(), []);

  const tryLockLandscape = useCallback(async () => {
    if (!nonDesktopViewport || typeof window === "undefined") return false;
    const orientation = window.screen?.orientation;
    if (!orientation?.lock || typeof orientation.lock !== "function") return false;

    try {
      await orientation.lock("landscape");
      return true;
    } catch {
      return false;
    }
  }, [nonDesktopViewport]);

  useEffect(() => {
    if (!nonDesktopViewport || typeof window === "undefined") return undefined;

    void tryLockLandscape();

    const handleFirstInteraction = () => {
      void tryLockLandscape();
    };

    window.addEventListener("pointerdown", handleFirstInteraction, {
      once: true,
      passive: true,
    });

    return () => {
      window.removeEventListener("pointerdown", handleFirstInteraction);
    };
  }, [nonDesktopViewport, tryLockLandscape]);

  if (!portraitCompactViewport) return null;

  return (
    <div className="mobile-landscape-gate">
      <div className="mobile-landscape-gate-panel">
        <div className="mobile-landscape-gate-kicker">Landscape Required</div>
        <h2 className="mobile-landscape-gate-title">Rotate your phone</h2>
        <p className="mobile-landscape-gate-copy">
          Ironsmith is tuned for landscape play on phones. Turn your device sideways to continue.
        </p>
        <button
          type="button"
          className="stone-pill mobile-landscape-gate-button"
          onClick={() => {
            void tryLockLandscape();
          }}
        >
          Try landscape
        </button>
        <p className="mobile-landscape-gate-note">
          {standaloneMode
            ? "Standalone mode is active. Once the device rotates, the table will fill the screen."
            : "Safari browser tabs keep browser chrome visible. Add the app to your Home Screen for the most screen space."}
        </p>
      </div>
    </div>
  );
}
