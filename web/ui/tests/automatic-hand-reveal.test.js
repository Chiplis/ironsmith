import test from 'node:test';
import assert from 'node:assert/strict';
import { automaticHandRevealCommand } from '../src/lib/automatic-hand-reveal.js';

function fixture() {
  return { perspective: 1, decision: { kind: 'select_objects', player: 1,
    automatic_public_reveal: true, reveal_policy: 'public', min: 7, max: 7,
    candidates: Array.from({ length: 7 }, (_, i) => ({ id: i + 10, legal: true })) } };
}
test('a forced seven-card reveal submits the entire set in one ordinary command', () => {
  assert.deepEqual(automaticHandRevealCommand(fixture()), {
    type: 'select_objects', object_ids: [10, 11, 12, 13, 14, 15, 16],
  });
});
test('other seats, optional reveals, actual selections, and invalid sets stay manual', () => {
  for (const change of [
    s => { s.perspective = 0; },
    s => { s.perspective = null; },
    s => { s.decision.automatic_public_reveal = false; },
    s => { s.decision.min = 0; },
    s => { s.decision.max = 1; },
    s => { s.decision.allow_partial_completion = true; },
    s => { s.decision.reveal_policy = 'none'; },
    s => { s.decision.candidates[0].legal = false; },
    s => { s.decision.candidates[0].reveal_policy = 'none'; },
    s => { s.decision.candidates[0].id = 11; },
  ]) {
    const state = fixture(); change(state);
    assert.equal(automaticHandRevealCommand(state), null);
  }
});
