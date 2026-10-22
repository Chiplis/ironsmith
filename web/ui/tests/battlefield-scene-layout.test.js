import test from 'node:test';
import assert from 'node:assert/strict';
import { allocateBattlefieldScene, fitBattlefieldScene, advanceBattlefieldPose, battlefieldSceneCellAtPoint } from '../src/lib/battlefield-scene-layout.js';

const card = (id, lane = 'creatures', extra = {}) => ({ id, stable_id: id, lane, ...extra });
const viewport = { width: 900, height: 400 };

test('compact lands retain a six pixel gap with reserved visual padding', () => {
  const layout = allocateBattlefieldScene([card(1, 'lands'), card(2, 'lands')]);
  const geometry = fitBattlefieldScene(layout, viewport, { compactLands: true, paddingX: 12, paddingY: 12 });
  const poses = [...geometry.poses.values()].sort((a, b) => a.x - b.x);
  assert.ok(Math.abs(poses[1].x - poses[0].x - poses[0].faceWidth * poses[0].scale - 6) < 1e-6);
});

test('removal, reorder and dense threshold do not move surviving logical cells', () => {
  const cards = Array.from({ length: 18 }, (_, i) => card(i));
  const first = allocateBattlefieldScene(cards);
  const next = allocateBattlefieldScene([card(100), ...cards.slice(1).reverse()], first);
  for (const c of cards.slice(1)) assert.deepEqual(next.gridPositionById.get(String(c.id)), first.gridPositionById.get(String(c.id)));
  assert.deepEqual(next.gridPositionById.get('100'), first.gridPositionById.get('0'));
  const crowded = allocateBattlefieldScene([...next.orderedCards, card(101)], next);
  assert.equal(crowded.gridPositionById.get('1').column, first.gridPositionById.get('1').column);
});

test('group representative changes preserve scene identity, blink gets a new object', () => {
  const first = allocateBattlefieldScene([card(2, 'creatures', { member_ids: [2, 3], member_stable_ids: [20, 30] })]);
  const next = allocateBattlefieldScene([card(3, 'creatures', { stable_id: 30 })], first);
  assert.equal(next.visualKeyById.get('3'), first.visualKeyById.get('2'));
  const blink = allocateBattlefieldScene([card(4, 'creatures', { stable_id: 30 })], next);
  assert.notEqual(blink.visualKeyById.get('4'), next.visualKeyById.get('3'));
  assert.deepEqual(blink.gridPositionById.get('4'), next.gridPositionById.get('3'));
});

test('front rows face the center; type changes migrate only that object', () => {
  const first = allocateBattlefieldScene([card(1), card(2, 'lands')], null, { side: 'top' });
  assert.equal(first.gridPositionById.get('1').row, 2);
  assert.equal(first.gridPositionById.get('2').row, 1);
  const next = allocateBattlefieldScene([card(1), card(2, 'creatures')], first, { side: 'top' });
  assert.deepEqual(next.gridPositionById.get('1'), first.gridPositionById.get('1'));
  assert.equal(next.gridPositionById.get('2').groupId, 'front');
});

test('huge boards retain readable objects and expose scrollable extent', () => {
  const layout = allocateBattlefieldScene(Array.from({ length: 200 }, (_, i) => card(i)));
  const geometry = fitBattlefieldScene(layout, viewport);
  assert.ok(geometry.cardWidth >= 44);
  assert.ok(geometry.height > viewport.height);
  assert.equal(new Set([...geometry.poses.values()].map(p => `${p.x}:${p.y}`)).size, 200);
  for (const p of geometry.poses.values()) assert.ok(p.x >= 0 && p.x + p.width <= geometry.width + .001);
});

test('interrupted animation is continuous and reduced motion reaches the destination', () => {
  const first = { x: 0, y: 0, scale: 1 }, target = { x: 100, y: 60, scale: .5 };
  const middle = advanceBattlefieldPose(first, target, 16);
  const reversed = advanceBattlefieldPose(middle, first, 16);
  assert.ok(reversed.x > 0 && reversed.x < middle.x);
  assert.ok(reversed.scale > middle.scale && reversed.scale < 1);
  assert.deepEqual(advanceBattlefieldPose(middle, target, 16, true), target);
});

test('drop slots and rendered target geometry share coordinates', () => {
  const layout = allocateBattlefieldScene([card(1), card(2, 'lands')]);
  const geometry = fitBattlefieldScene(layout, viewport, { top: 13, bottom: 60 });
  for (const p of geometry.poses.values()) assert.deepEqual(battlefieldSceneCellAtPoint(geometry, p.x + p.width / 2, p.y + p.height / 2), { row: p.row, column: p.column });
  assert.equal(battlefieldSceneCellAtPoint(geometry, -1, -1), null);
});

test('splitting grouped permanents never duplicates a visual identity', () => {
  const first = allocateBattlefieldScene([card(1, 'creatures', { member_ids: [1, 2], member_stable_ids: [1, 2] })]);
  const split = allocateBattlefieldScene([card(2), card(1)], first);
  assert.equal(new Set(split.visualKeyById.values()).size, 2);
  assert.equal(new Set([...split.gridPositionById.values()].map(p => `${p.row}:${p.column}`)).size, 2);
});

test('attachments fan behind their host; cycles and foreign hosts stay independently accessible', () => {
  const layout = allocateBattlefieldScene([card(1), card(2, 'enchantments', { attached_to_ids: [1] }),
    card(3, 'artifacts', { attached_to_ids: [2] }), card(4, 'enchantments', { attached_to_ids: [99] })]);
  assert.deepEqual(layout.gridPositionById.get('1'), layout.gridPositionById.get('3'));
  const geometry = fitBattlefieldScene(layout, viewport);
  const host = geometry.poses.get(layout.visualKeyById.get('1'));
  const attachment = geometry.poses.get(layout.visualKeyById.get('2'));
  assert.ok(attachment.x > host.x && attachment.y > host.y);
  assert.equal(layout.attachmentDepthById.has('4'), false);
  const cycle = allocateBattlefieldScene([card(1, 'artifacts', { attached_to_ids: [2] }), card(2, 'artifacts', { attached_to_ids: [1] })]);
  assert.equal(cycle.attachmentDepthById.size, 0);
});

test('remembered manual placement survives reallocation', () => {
  const placementSlots = new Map([['stable:1', { row: 2, column: 6 }]]);
  const layout = allocateBattlefieldScene([card(1), card(2)], null, { placementSlots });
  assert.equal(layout.gridPositionById.get('1').column, 6);
  assert.equal(layout.gridPositionById.get('1').row, 2);
});

test('reserved Stack space participates in card fitting and drop coordinates', () => {
  const layout = allocateBattlefieldScene(Array.from({ length: 8 }, (_, i) => card(i)));
  for (const alignStart of [true, false]) {
    const geometry = fitBattlefieldScene(layout, viewport, { leftInset: 320, paddingX: 20, alignStart });
    for (const pose of geometry.poses.values()) {
      assert.ok(pose.x - 20 * pose.scale >= 320);
      assert.ok(pose.x + (pose.faceWidth + 20) * pose.scale <= geometry.width + .01);
      assert.deepEqual(battlefieldSceneCellAtPoint(geometry, pose.x + 1, pose.y + 1), { row: pose.row, column: pose.column });
    }
    assert.equal(battlefieldSceneCellAtPoint(geometry, 10, 10), null);
  }
});
