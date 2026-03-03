export function getCardRect(objectId) {
  const el = document.querySelector(`.game-card[data-object-id="${objectId}"]`);
  return el ? el.getBoundingClientRect() : null;
}

export function getPlayerTargetRect(playerIndex) {
  const el = document.querySelector(`[data-player-target="${playerIndex}"]`);
  return el ? el.getBoundingClientRect() : null;
}

export function centerOf(rect) {
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}
