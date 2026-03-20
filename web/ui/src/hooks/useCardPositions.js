export function getCardElement(objectId) {
  return document.querySelector(
    `[data-arrow-anchor="stack"][data-object-id="${objectId}"], .game-card[data-object-id="${objectId}"]`
  );
}

export function getCardRect(objectId) {
  const el = getCardElement(objectId);
  return el ? el.getBoundingClientRect() : null;
}

export function getPlayerTargetElement(playerIndex) {
  return document.querySelector(
    `[data-player-target-name="${playerIndex}"], ` +
    `[data-player-target="${playerIndex}"], ` +
    `[data-player-nav-target-name="${playerIndex}"], ` +
    `[data-player-nav-target="${playerIndex}"]`
  );
}

export function getPlayerTargetRect(playerIndex) {
  const el = getPlayerTargetElement(playerIndex);
  return el ? el.getBoundingClientRect() : null;
}

export function centerOf(rect) {
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}
