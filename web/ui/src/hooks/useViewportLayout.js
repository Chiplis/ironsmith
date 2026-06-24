import { useEffect, useState } from "react";

const PORTRAIT_COMPACT_QUERY = "(max-width: 720px) and (orientation: portrait)";
const PHONE_WIDTH_QUERY = "(max-width: 720px)";
const LANDSCAPE_MOBILE_QUERY = "(max-width: 720px) and (orientation: landscape)";
const TABLET_COMPACT_QUERY = "(min-width: 721px) and (max-width: 1023px)";
const SMALL_DESKTOP_QUERY = "(min-width: 1024px) and (max-width: 1439px)";
const LARGE_DESKTOP_QUERY = "(min-width: 1800px)";

function readViewportLayout() {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return {
      portraitCompactViewport: false,
      landscapeMobileViewport: false,
      nonDesktopViewport: false,
      tabletCompactViewport: false,
      smallDesktopViewport: false,
      largeDesktopViewport: false,
      compactViewport: false,
    };
  }

  const portraitCompactViewport = window.matchMedia(PORTRAIT_COMPACT_QUERY).matches;
  const phoneWidthViewport = window.matchMedia(PHONE_WIDTH_QUERY).matches;
  const landscapeMobileViewport = window.matchMedia(LANDSCAPE_MOBILE_QUERY).matches;
  const tabletCompactViewport = window.matchMedia(TABLET_COMPACT_QUERY).matches;
  const smallDesktopViewport = window.matchMedia(SMALL_DESKTOP_QUERY).matches;
  const largeDesktopViewport = window.matchMedia(LARGE_DESKTOP_QUERY).matches;

  return {
    portraitCompactViewport,
    landscapeMobileViewport,
    nonDesktopViewport: phoneWidthViewport,
    tabletCompactViewport,
    smallDesktopViewport,
    largeDesktopViewport,
    compactViewport: tabletCompactViewport || phoneWidthViewport,
  };
}

export default function useViewportLayout() {
  const [viewportLayout, setViewportLayout] = useState(() => readViewportLayout());

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
      return undefined;
    }

    const portraitMedia = window.matchMedia(PORTRAIT_COMPACT_QUERY);
    const phoneWidthMedia = window.matchMedia(PHONE_WIDTH_QUERY);
    const landscapeMedia = window.matchMedia(LANDSCAPE_MOBILE_QUERY);
    const tabletMedia = window.matchMedia(TABLET_COMPACT_QUERY);
    const smallDesktopMedia = window.matchMedia(SMALL_DESKTOP_QUERY);
    const largeDesktopMedia = window.matchMedia(LARGE_DESKTOP_QUERY);
    const updateViewportLayout = () => {
      setViewportLayout(readViewportLayout());
    };

    updateViewportLayout();
    portraitMedia.addEventListener("change", updateViewportLayout);
    phoneWidthMedia.addEventListener("change", updateViewportLayout);
    landscapeMedia.addEventListener("change", updateViewportLayout);
    tabletMedia.addEventListener("change", updateViewportLayout);
    smallDesktopMedia.addEventListener("change", updateViewportLayout);
    largeDesktopMedia.addEventListener("change", updateViewportLayout);
    window.addEventListener("resize", updateViewportLayout);

    return () => {
      portraitMedia.removeEventListener("change", updateViewportLayout);
      phoneWidthMedia.removeEventListener("change", updateViewportLayout);
      landscapeMedia.removeEventListener("change", updateViewportLayout);
      tabletMedia.removeEventListener("change", updateViewportLayout);
      smallDesktopMedia.removeEventListener("change", updateViewportLayout);
      largeDesktopMedia.removeEventListener("change", updateViewportLayout);
      window.removeEventListener("resize", updateViewportLayout);
    };
  }, []);

  return viewportLayout;
}
