import { hiddenCardMetadataForObjectFromCheckpoint } from "../src/lib/hidden-card-metadata.js";
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { assertZiffleOpeningOriginMatchesMetadata } from '../src/lib/multiplayer-audit.js';

const shared = readFileSync(new URL('../src/hooks/peer-lobby/shared.js', import.meta.url), 'utf8');
const audit = readFileSync(new URL('../src/hooks/peer-lobby/audit-material.js', import.meta.url), 'utf8');
function declaration(name) {
  const start = shared.indexOf(`export function ${name}(`);
  const end = shared.indexOf('\nexport ', start + 1);
  assert.ok(start >= 0 && end > start, `Missing ${name}`);
  return shared.slice(start, end).replace(/^export /, '');
}
const names = ['ziffleDeckHashFromCommitment', 'zifflePositionFromCommitment',
  'hiddenMetadataMatchesZifflePosition',
  'hiddenObjectIdForOpeningFromCheckpoint', 'checkpointObjectForId', 'checkpointObjectHiddenCard',
  'checkpointObjectName', 'checkpointObjectOpeningCardName', 'checkpointObjectIsRedactedHidden',
  'knownCheckpointObjectMatchesOpening'];
const helpers = new Function('hiddenCardMetadataForObjectFromCheckpoint', `${names.map(declaration).join('\n')}\nreturn {hiddenCardMetadataForObjectFromCheckpoint,${names.join(',')}};`)(hiddenCardMetadataForObjectFromCheckpoint);
const revealStart = audit.indexOf('  const revealAuditOpenings = useCallback(');
const revealEnd = audit.indexOf('  async function previewRequirementsForCommand(', revealStart);
assert.ok(revealStart >= 0 && revealEnd > revealStart);

function fixture(fields = {}) {
  const object = { id: 203, owner: 1, zone: 'battlefield', name: 'Grizzly Bears', originalCardName: 'Clone',
    hiddenCard: { owner: 1, slot: 4, commitment: 'salted-clone', publicSlot: 51,
      publicCommitment: 'ziffle:current:51', originSlot: 23, originCommitment: 'ziffle:genesis:23' }, ...fields };
  const checkpoint = { objects: [object] };
  const opening = { owner: 1, slot: 4, card: 'Clone', commitment: 'salted-clone', objectId: object.id,
    position: 51, positionCommitment: 'ziffle:current:51', originPosition: 23,
    originPositionCommitment: 'ziffle:genesis:23', timing: 'post' };
  return { object, checkpoint, opening };
}

for (const [name, originalCardName] of [
  ['Bala Ged Sanctuary', 'Bala Ged Recovery // Bala Ged Sanctuary'],
  ['Stomp', 'Bonecrusher Giant // Stomp'],
  ['Grizzly Bears', 'Clone'],
]) test(`opening lookup uses the physical identity while displayed as ${name}`, () => {
  const { checkpoint, opening } = fixture({ name, originalCardName });
  opening.card = originalCardName;
  opening.objectId = 202;
  assert.equal(helpers.hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening), 203);
});

test('physical opening names support checkpoint aliases without changing display names', () => {
  const { object, checkpoint, opening } = fixture({ originalCardName: undefined, original_card_name: 'Clone' });
  assert.equal(helpers.checkpointObjectOpeningCardName(object), 'Clone');
  assert.equal(helpers.checkpointObjectName(object), 'Grizzly Bears');
  assert.equal(helpers.hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening), 203);
  delete object.original_card_name;
  assert.equal(helpers.checkpointObjectOpeningCardName(object), 'Grizzly Bears');
  assert.equal(helpers.checkpointObjectOpeningCardName({ identity: { name: 'Legacy card' } }), 'Legacy card');
});

test('opening lookup retains legacy name rejection and strict physical identity checks', () => {
  const { object, checkpoint, opening } = fixture();
  assert.equal(helpers.hiddenObjectIdForOpeningFromCheckpoint(checkpoint, { ...opening, owner: 0 }), null);
  assert.equal(helpers.hiddenObjectIdForOpeningFromCheckpoint(checkpoint, {
    ...opening, slot: 5, commitment: 'another-copy',
  }), null, 'a shared public position cannot replace a known private identity');
  assert.equal(helpers.hiddenObjectIdForOpeningFromCheckpoint(checkpoint, {
    ...opening, position: 50, positionCommitment: 'ziffle:old:50',
  }), null, 'a stale public position does not match');
  object.originalCardName = 'Another physical card';
  assert.equal(helpers.hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening), null);
  delete object.originalCardName;
  assert.equal(helpers.hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening), null);
  object.name = 'Clone';
  assert.equal(helpers.hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening), 203);
});

test('objects without hidden metadata still require their current name', () => {
  const { object, opening } = fixture({ hiddenCard: undefined });
  assert.equal(helpers.knownCheckpointObjectMatchesOpening(object, opening), false);
  assert.equal(helpers.knownCheckpointObjectMatchesOpening(object, { ...opening, card: 'Grizzly Bears' }), true);
});

function applicationHarness() {
  const { object, checkpoint, opening } = fixture();
  const calls = [];
  const metadata = () => helpers.hiddenCardMetadataForObjectFromCheckpoint(checkpoint, object.id);
  const context = {
    ...helpers,
    useCallback: value => value,
    gameRef: { current: {
      getHiddenCardState: async () => checkpoint,
      revealHiddenSlot: async () => { throw new Error('Unexpected slot fallback'); },
      revealHiddenPosition: async input => {
        calls.push(input);
        assert.equal(input.objectId, object.id, 'the verified opening remains bound to the existing object');
        assert.equal(input.cardName, object.originalCardName);
        return { preservedName: object.name };
      },
    } },
    sanitizeObjectBoundOpening: async value => value,
    currentHiddenCardMetadataForObject: async id => helpers.hiddenCardMetadataForObjectFromCheckpoint(checkpoint, id),
    resolveLocalCryptoPlayerIndex: () => 0,
    // Proof validation is exercised by the origin/manifest suites. Here it must
    // still run before either object-name gate or any engine hydration call.
    verifyAuditOpeningsAgainstManifests: async openings => {
      for (const candidate of openings) assertZiffleOpeningOriginMatchesMetadata(candidate, metadata());
    },
    rememberLocalRevealedOpening: () => {},
    previewAuditOpeningInInspector: () => {},
    ziffleCeremonyForOwner: () => null,
    ziffleContextFromOpening: () => '',
    setState: () => {},
  };
  const reveal = new Function(...Object.keys(context), `${audit.slice(revealStart, revealEnd)}\nreturn revealAuditOpenings;`)(...Object.values(context));
  return { object, opening, calls, reveal };
}

for (const retired of [false, true]) test(`receiver applies the physical-card opening with ${retired ? 'a retired' : 'the current'} object id`, async () => {
  const h = applicationHarness();
  if (retired) h.opening.objectId = 202;
  const result = await h.reveal([h.opening], { timing: 'post', updateState: false, previewInspector: false });
  assert.equal(h.calls.length, 1);
  assert.equal(result.preservedName, 'Grizzly Bears');
});

test('origin mismatch still fails before reopening a renamed object', async () => {
  const h = applicationHarness();
  await assert.rejects(h.reveal([{ ...h.opening, originPosition: 22, originPositionCommitment: 'ziffle:genesis:22' }],
    { timing: 'post', updateState: false, previewInspector: false }), /trusted identity/);
  assert.deepEqual(h.calls, []);
});
