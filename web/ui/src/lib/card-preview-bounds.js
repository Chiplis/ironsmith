// The local hand and battlefield share the space beneath the phase controls.
export function localCardPreviewBounds(viewport = globalThis.window, root = globalThis.document) {
  const margin = 8;
  const visibleTop = selector => {
    const rect = root?.querySelector(selector)?.getBoundingClientRect();
    return rect?.width > 0 && rect.height > 0 ? rect.top : margin;
  };
  const top = Math.max(margin, visibleTop('.topbar-shell'), visibleTop('[data-my-zone] .my-zone-board-shell'));
  const viewportHeight = viewport?.innerHeight || 900;
  const viewportWidth = viewport?.innerWidth || 1440;
  const preferredHeight = viewportWidth <= 1180 ? 460 : Math.min(680, Math.max(520, viewportWidth * 0.32));
  const height = Math.max(1, Math.min(preferredHeight, viewportHeight - top - margin));
  const width = Math.min(height * 63 / 88, viewportWidth - margin * 2);
  return { top, width, height, margin };
}
