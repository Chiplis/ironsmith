/** Keep phone thresholds aligned with the landscape rules in responsive.css. */
export function classifyViewport(width, height) {
  const landscape = width > height;
  const phoneWidth = width <= 720;
  const landscapeMobileViewport = landscape && (phoneWidth || (width <= 1023 && height <= 540));
  const nonDesktopViewport = phoneWidth || landscapeMobileViewport;
  // Tablets get the desktop table; only phones (and short landscape phones)
  // use the dedicated mobile layouts.
  const tabletCompactViewport = false;
  return {
    portraitCompactViewport: phoneWidth && !landscape,
    landscapeMobileViewport,
    nonDesktopViewport,
    tabletCompactViewport,
    smallDesktopViewport: !nonDesktopViewport && width <= 1439,
    largeDesktopViewport: width >= 1800,
    compactViewport: tabletCompactViewport || nonDesktopViewport,
  };
}
