import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { findZiffleDisclosureOrigin, ziffleDisclosureDueForPlayer } from '../src/lib/ziffle-disclosure-origin.js';
import { ziffleOriginAnchorFromMetadata } from '../src/lib/multiplayer-audit.js';

const opening = { owner: 1, position: 51, positionCommitment: 'ziffle:latest:51',
  originPosition: 23, originPositionCommitment: 'ziffle:initial:23' };
const requirement = { type: 'public_open', owner: 1, object_id: 211, slot: 4, commitment: 'original-4',
  public_slot: 51, public_commitment: opening.positionCommitment,
  origin_slot: 23, origin_commitment: opening.originPositionCommitment };
const state = { game_over: true, players: [{ id: 0 }, { id: 1, has_lost: true }] };

test('departed cards and anchor-only disclosure entries retain their exact trusted origin', () => {
  for (const object_id of [211, undefined]) {
    const found = findZiffleDisclosureOrigin({ opening, state, requirements: [{ ...requirement, object_id }] });
    assert.equal(found.originPosition, 23);
    assert.equal(found.originPositionCommitment, opening.originPositionCommitment);
    assert.equal(found.objectId, object_id ?? null);
  }
});

test('disclosure lookup cannot authorize active players, unrelated positions or peer-claimed origins', () => {
  const lookup = (candidate, values = [requirement], status = state) => findZiffleDisclosureOrigin({ opening: candidate, requirements: values, state: status });
  assert.equal(lookup(opening, [requirement], { players: [{ id: 1 }] }), null);
  assert.equal(lookup(opening, [requirement], { players: [{ id: 0, has_lost: true }, { id: 1 }] }), null);
  assert.equal(lookup({ ...opening, owner: 0 }), null);
  assert.equal(lookup({ ...opening, position: 50 }), null);
  assert.equal(lookup({ ...opening, positionCommitment: 'ziffle:other:51' }), null);
  assert.equal(lookup(opening, [{ ...requirement, type: 'private_open' }]), null);
  const forged = lookup({ ...opening, originPosition: 24, originPositionCommitment: 'ziffle:initial:24' });
  assert.equal(forged.originPosition, 23, 'selection uses independently trusted current position, not the claim');
  assert.throws(() => lookup(opening, [requirement, { ...requirement, origin_slot: 24, origin_commitment: 'ziffle:initial:24' }]), /ambiguous/);
});

test('live origin service consults final engine requirements only in explicit disclosure context after loss', async () => {
  const source = readFileSync(new URL('../src/hooks/peer-lobby/audit-material.js', import.meta.url), 'utf8');
  const start = source.indexOf('  const currentZiffleOriginForOpening = useCallback(');
  const end = source.indexOf('const sanitizeObjectBoundOpening', start);
  const declaration = source.slice(start, end).trim();
  let calls = 0, currentState = { players: [{ id: 1 }] };
  const context = { useCallback: fn => fn, gameRef: { current: {
    getHiddenCardState: async () => ({ objects: [] }),
    getHiddenCardMetadataAtPosition: async () => [], uiState: async () => currentState,
    endOfMatchDisclosureRequirements: async owner => { assert.equal(owner, 1); calls++; return [requirement]; },
  } }, zifflePositionFromCommitment: value => Number(String(value).slice(String(value).lastIndexOf(':') + 1)),
  hiddenCardMetadataForObjectFromCheckpoint: () => null, ziffleOriginAnchorFromMetadata,
  findZiffleDisclosureOrigin, ziffleDisclosureDueForPlayer };
  const lookup = new Function(...Object.keys(context), `${declaration}\nreturn currentZiffleOriginForOpening;`)(...Object.values(context));
  assert.equal(await lookup(opening, { endOfMatchDisclosure: true, requirements: [requirement] }), null);
  assert.equal(calls, 0);
  currentState = state;
  assert.equal(await lookup(opening), null);
  assert.equal(calls, 0);
  assert.equal((await lookup(opening, { endOfMatchDisclosure: true, requirements: [] })).originPosition, 23);
  assert.equal(calls, 1);
});
