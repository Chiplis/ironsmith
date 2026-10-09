import test from 'node:test';
import assert from 'node:assert/strict';
import { playerBoardLights } from '../src/components/board/forge/player-board-lights.js';
const rect = (left, top, right, bottom) => ({ left, top, right, bottom });

test('each duel light originates beyond its own edge and uses its player accent', () => {
  const lights = playerBoardLights({ width: 1200, height: 800, seats: [
    { owner: '7', rect: rect(0, 0, 1200, 380) }, { owner: '3', rect: rect(0, 420, 1200, 800) },
  ] }, [{ id: 3, color: '#b79cff' }, { id: 7, color: '#ff3b30' }], 3);
  assert.equal(lights[0].color, '#ff3b30');
  assert.ok(lights[0].y < 0 && lights[0].targetY > 0);
  assert.equal(lights[1].color, '#b79cff');
  assert.ok(lights[1].y > 800 && lights[1].targetY < 800);
  assert.ok(lights.every(light => light.intensity > 0 && light.z > 0));
});

test('multiplayer seats get separate sources; hidden mobile opponents have no light', () => {
  const players = Array.from({ length: 4 }, (_, id) => ({ id, color: ['#bb99ff', '#ff3333', '#33ff55', '#ffaa22'][id] }));
  const layout = { width: 1200, height: 800, seats: [
    ...[1, 2, 3].map((id, index) => ({ owner: String(id), rect: rect(index * 400, 0, (index + 1) * 400, 360) })),
    { owner: '0', rect: rect(0, 400, 1200, 800) },
  ] };
  const lights = playerBoardLights(layout, players, 0);
  assert.equal(lights.length, 4);
  assert.deepEqual(lights.slice(0, 3).map(light => light.x), [200, 600, 1000]);
  const mobile = playerBoardLights({ ...layout, seats: layout.seats.filter(seat => ['0', '2'].includes(seat.owner)) }, players, 0);
  assert.deepEqual(mobile.map(light => light.owner), ['2', '0']);
  assert.equal(playerBoardLights(layout, [], 0).length, 0);
});

test('viewport scaling preserves inverse-square illumination and spectator edge placement', () => {
  const players = [{ id: 0, color: '#b79cff' }];
  const small = playerBoardLights({ width: 800, height: 600, seats: [{ owner: '0', rect: rect(0, 300, 800, 600) }] }, players)[0];
  const large = playerBoardLights({ width: 1600, height: 1200, seats: [{ owner: '0', rect: rect(0, 600, 1600, 1200) }] }, players)[0];
  assert.ok(small.y > 600 && large.y > 1200);
  assert.ok(Math.abs(small.intensity / small.distance ** 2 - large.intensity / large.distance ** 2) < 1e-10);
});
