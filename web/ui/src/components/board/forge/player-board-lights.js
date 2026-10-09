// Seat rectangles, rather than occupied card bounds, keep illumination steady
// when cards move or a battlefield is empty. Coordinates here are CSS pixels.
export function playerBoardLights(layout, players = [], perspective = null) {
  const colors = new Map(players.map(player => [String(player.id), player.color]));
  return (layout.seats || []).filter(seat => colors.has(seat.owner)).map(seat => {
    const { rect, owner } = seat;
    const x = (rect.left + rect.right) / 2;
    const y = (rect.top + rect.bottom) / 2;
    const bottom = perspective != null ? owner === String(perspective) : y > layout.height / 2;
    const z = Math.max(100, Math.min(layout.width, layout.height) * .42);
    const sourceY = bottom ? layout.height * 1.06 : -layout.height * .06;
    const distance = Math.hypot(sourceY - y, z + 60);
    return {
      owner, color: colors.get(owner), x, y: sourceY, z,
      targetX: x, targetY: y, targetZ: -60,
      // Inverse-square falloff: scale intensity with the pixel-coordinate world.
      intensity: distance * distance * 1.3,
      distance: distance * 2.6,
      angle: Math.min(1.15, Math.max(.55, Math.atan2((rect.right - rect.left) * .72, distance))),
    };
  });
}
