import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
const source = readFileSync(new URL('../src/hooks/peer-lobby/shared.js', import.meta.url), 'utf8');
function declaration(name) {
  const begin = source.indexOf(`export function ${name}(`);
  assert.ok(begin >= 0);
  const end = source.indexOf('\n}', begin);
  return source.slice(begin, end + 2).replace('export ', '');
}
function api() {
  const context = vm.createContext({
    Set, Number,
    isFaceDownCastCommand: () => false,
    isForetellCommand: () => false,
    actionRefObjectId: ref => ref?.action?.card_id ?? ref?.action?.source_id ?? ref?.spell_id,
  });
  vm.runInContext(declaration('isPrivateCostReservationCommand') + '\n' + declaration('collectCommandObjectIds'), context);
  return context;
}
for (const kind of ['prepare_exile', 'prepare_granted_exile', 'perform_intrinsic_action']) {
  test(`${kind} suppresses initial source opening; ordinary commands retain opening policy`, () => {
    const c = api();
    const action = { kind, card_id: 17, source_id: 17, cost_reservation: { actor: 0, source_stable_id: 17, hidden_incarnation: 2, cost: { kind: 'mana' } } };
    const command = { type: 'priority_action', action_ref: { kind: 'special_action', action } };
    assert.equal(c.isPrivateCostReservationCommand(command), true);
    assert.equal(c.collectCommandObjectIds(command).size, 0);
    delete action.cost_reservation;
    assert.equal(c.collectCommandObjectIds(command).has(17), false, 'missing reservation is rejected natively without disclosure');
    const ordinary = { type: 'priority_action', action_ref: { kind: 'cast_spell', spell_id: 22, cost_reservation: {} } };
    assert.equal(c.isPrivateCostReservationCommand(ordinary), false);
    assert.equal(c.collectCommandObjectIds(ordinary).has(22), true);
  });
}
test('reservation marker cannot suppress unrelated special action disclosure', () => {
  const c = api();
  const command = { type: 'priority_action', action_ref: { kind: 'special_action', action: { kind: 'suspend', card_id: 31, cost_reservation: {} } } };
  assert.equal(c.isPrivateCostReservationCommand(command), false);
  assert.equal(c.collectCommandObjectIds(command).has(31), true);
});
