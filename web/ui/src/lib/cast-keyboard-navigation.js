export function castNavigationKeyAllowed(event) {
  return !event.defaultPrevented && !event.isComposing
    && !event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey
    && !event.target?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="combobox"], [role="dialog"], [role="menu"]');
}

export function nextPlacementCell(cells, current, key) {
  if (!cells.length) return null;
  if (!current) return cells[0];
  const horizontal = key === "ArrowLeft" || key === "ArrowRight";
  const axis = horizontal ? "column" : "row";
  const cross = horizontal ? "row" : "column";
  const direction = key === "ArrowLeft" || key === "ArrowUp" ? -1 : 1;
  return cells.filter(cell => cell[cross] === current[cross]
    && (cell[axis] - current[axis]) * direction > 0)
    .sort((a, b) => Math.abs(a[axis] - current[axis]) - Math.abs(b[axis] - current[axis]))[0] || current;
}
