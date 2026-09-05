/** Keep phone thresholds aligned with the landscape rules in responsive.css. */
export function classifyViewport(width, height) {
  const landscape = width > height;
  const phoneWidth = width <= 720;
  const landscapeMobileViewport = landscape && (phoneWidth || (width <= 1023 && height <= 540));
  const nonDesktopViewport = phoneWidth || landscapeMobileViewport;
  const tabletCompactViewport = width >= 721 && width <= 1023 && !landscapeMobileViewport;
  return {
    portraitCompactViewport: phoneWidth && !landscape,
    landscapeMobileViewport,
    nonDesktopViewport,
    tabletCompactViewport,
    smallDesktopViewport: width >= 1024 && width <= 1439,
    largeDesktopViewport: width >= 1800,
    compactViewport: tabletCompactViewport || nonDesktopViewport,
  };
}
