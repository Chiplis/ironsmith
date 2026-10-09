import test from 'node:test';
import assert from 'node:assert/strict';
import { clipRect, settleForgeZones, forgeCornerSizes, forgeSignal } from '../src/lib/forge-board-layout.js';
const zone = (right, key = 'field') => ({ key, battlefield: true, rect: { left: 20, top: 20, right, bottom: 200 } });
test('first permanent and token swarm expand immediately; wipe shrinks after flights', () => {
  let zones = settleForgeZones(new Map(), [zone(200)], 0);
  zones = settleForgeZones(zones, [zone(950)], 100);
  assert.equal(zones.get('field').rect.right, 950);
  zones = settleForgeZones(zones, [zone(200)], 200);
  assert.equal(zones.get('field').rect.right, 950);
  zones = settleForgeZones(zones, [zone(200)], 1801);
  assert.equal(zones.get('field').rect.right, 200);
});
test('blink, reanimation, and rapid updates retain outgoing capacity', () => {
  let zones = settleForgeZones(new Map(), [zone(900)], 0);
  zones = settleForgeZones(zones, [], 100);
  assert.equal(zones.size, 1);
  zones = settleForgeZones(zones, [zone(920)], 300);
  assert.equal(zones.get('field').rect.right, 920);
  zones = settleForgeZones(zones, [], 500);
  zones = settleForgeZones(zones, [], 2200);
  assert.equal(zones.size, 0);
});
test('control changes protect both player areas and unequal multiplayer boards', () => {
  let zones = settleForgeZones(new Map(), [zone(800, 'a'), zone(200, 'b'), zone(300, 'c'), zone(150, 'd')], 0);
  zones = settleForgeZones(zones, [zone(200, 'a'), zone(800, 'b'), zone(300, 'c'), zone(150, 'd')], 100);
  assert.equal(zones.get('a').rect.right, 800);
  assert.equal(zones.get('b').rect.right, 800);
  assert.equal(zones.get('c').rect.right, 300);
});
test('target selection and dragging prevent contraction; reconnect/resize resets stale bounds', () => {
  let zones = settleForgeZones(new Map(), [zone(900)], 0);
  zones = settleForgeZones(zones, [zone(200)], 100);
  zones = settleForgeZones(zones, [zone(200)], 5000, { locked: true });
  assert.equal(zones.get('field').rect.right, 900);
  zones = settleForgeZones(zones, [zone(200)], 5100, { reset: true });
  assert.equal(zones.get('field').rect.right, 200);
});
test('attachments, hand and opened viewers evict corner scenery without shrinking cards', () => {
  assert.ok(forgeCornerSizes(1000, 700, [])[0] > 80);
  assert.equal(forgeCornerSizes(1000, 700, [{ left: 0, top: 0, right: 220, bottom: 200 }])[0], 0);
  assert.deepEqual(clipRect({ left: -20, top: -40, right: 1100, bottom: 800 }, 1000, 700), { left: 0, top: 0, right: 1000, bottom: 700 });
  assert.equal(clipRect({ left: 1100, top: 0, right: 1200, bottom: 100 }, 1000, 700), null);
});
test('turn and combat cues use public state only', () => {
  assert.deepEqual(forgeSignal({ turn_number: 3, active_player: 2, phase: 'Combat', hand: ['secret'] }), { turn: '3:2', combat: true });
});
