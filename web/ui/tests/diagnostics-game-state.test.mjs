import test from 'node:test';
import assert from 'node:assert/strict';
import { exportDiagnostics } from '../src/lib/action-diagnostics.js';

test('exports a detached, untruncated game snapshot including deep card and decision data', () => {
  const state = {
    turn_number: 7,
    players: [{ hand_cards: Array.from({ length: 20 }, (_, id) => ({ id, abilities: [{ effects: [{ condition: { card_types: ['Instant', 'Sorcery'] } }] }] })) }],
    decision: { kind: 'targets', requirements: [{ legal_targets: Array.from({ length: 30 }, (_, object) => ({ kind: 'object', object })) }] },
    stack_objects: [{ id: 99, name: 'Lightning Bolt' }],
  };
  const report = exportDiagnostics({ game: { turn: 7 } }, state);
  assert.deepEqual(report.gameState, state);
  assert.equal(report.gameStateSource, 'last_published_ui_snapshot');
  state.players[0].hand_cards[0].id = 900;
  assert.equal(report.gameState.players[0].hand_cards[0].id, 0);
  assert.equal(JSON.parse(JSON.stringify(report)).gameState.decision.requirements[0].legal_targets.length, 30);
});

test('reports remain exportable without state or if state serialization fails', () => {
  assert.equal(exportDiagnostics().gameState, null);
  const state = {}; state.self = state;
  const report = exportDiagnostics(null, state);
  assert.equal(report.gameState, null);
  assert.ok(report.gameStateError);
  assert.ok(report.exportedAt);
});
