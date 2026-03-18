import { useEffect, useState } from "react";

const PORTRAIT_COMPACT_QUERY = "(max-width: 720px) and (orientation: portrait)";
const LANDSCAPE_MOBILE_QUERY = "(max-height: 480px) and (orientation: landscape)";

function readViewportLayout() {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return {
      portraitCompactViewport: false,
      landscapeMobileViewport: false,
      nonDesktopViewport: false,
    };
  }

  const portraitCompactViewport = window.matchMedia(PORTRAIT_COMPACT_QUERY).matches;
  const landscapeMobileViewport = window.matchMedia(LANDSCAPE_MOBILE_QUERY).matches;

  return {
    portraitCompactViewport,
    landscapeMobileViewport,
    nonDesktopViewport: portraitCompactViewport || landscapeMobileViewport,
  };
}

export default function useViewportLayout() {
  const [viewportLayout, setViewportLayout] = useState(() => readViewportLayout());

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
      return undefined;
    }

    const portraitMedia = window.matchMedia(PORTRAIT_COMPACT_QUERY);
    const landscapeMedia = window.matchMedia(LANDSCAPE_MOBILE_QUERY);
    const updateViewportLayout = () => {
      setViewportLayout(readViewportLayout());
    };

    updateViewportLayout();
    portraitMedia.addEventListener("change", updateViewportLayout);
    landscapeMedia.addEventListener("change", updateViewportLayout);
    window.addEventListener("resize", updateViewportLayout);

    return () => {
      portraitMedia.removeEventListener("change", updateViewportLayout);
      landscapeMedia.removeEventListener("change", updateViewportLayout);
      window.removeEventListener("resize", updateViewportLayout);
    };
  }, []);

  return viewportLayout;
}
