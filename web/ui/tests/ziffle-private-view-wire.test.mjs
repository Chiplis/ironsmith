import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { webcrypto } from 'node:crypto';
import { createAuditEncryptionKey, exportAuditEncryptionPublicKey, encryptPrivateAuditPayload,
  decryptPrivateAuditPayload, buildPrivateDeckManifest, buildDeckSlotOpening } from '../src/lib/multiplayer-audit.js';

const source = readFileSync(new URL('../src/hooks/peer-lobby/crypto-resync.js', import.meta.url), 'utf8');
function extract(start, end) {
  const from = source.indexOf(start), to = source.indexOf(end, from);
  assert.ok(from >= 0 && to > from);
  return source.slice(from, to);
}

test('encrypted private view exposes only its ciphertext position, and recipient rejects swapped headers', async () => {
  const encryptionKeyPair = await createAuditEncryptionKey(webcrypto);
  const publicKey = await exportAuditEncryptionPublicKey(encryptionKeyPair, webcrypto);
  const manifest = await buildPrivateDeckManifest({ matchId: 'match', owner: 0,
    deck: ['Island', 'Swamp', 'Mountain'], saltForSlot: slot => `secret-${slot}` }, webcrypto);
  const opening = { ...await buildDeckSlotOpening({ manifest, slot: 2 }, webcrypto),
    objectId: 100, position: 0, positionCommitment: 'ziffle:fresh:0' };
  const requirement = { id: 'private_open:0:library:2:100', type: 'private_open', owner: 0,
    viewer: 1, zone: 'library', slot: 2, objectId: 100, commitment: opening.commitment, card: opening.card };
  const disclosures = [];
  const context = {
    useCallback: value => value, gameRef: { current: { exportHiddenCardOpening: async () => opening } },
    resolveLocalCryptoPlayerIndex: () => 1, isOwnerPrivateViewRequirement: () => false,
    cryptoMaterialResponsibleSeat: value => value.viewer, auditEncryptionPublicKeyForPlayer: () => publicKey,
    stateRef: { current: {} }, wasmObjectIdArg: value => value,
    buildLocalOpeningFromRequirement: async () => ({ opening, owner: 0, position: 0, positionCommitment: opening.positionCommitment }),
    sanitizeObjectBoundOpening: async value => value, currentAuditMatchId: () => 'match',
    currentHiddenCardMetadataForObject: async () => null,
    encryptPrivateAuditPayload: args => encryptPrivateAuditPayload(args, webcrypto),
    rememberPrivateViewDisclosure: value => disclosures.push(value),
    ziffleDeckHashFromCommitment: value => /^ziffle:([^:]+):\d+$/.exec(value || '')?.[1] || '',
  };
  const build = new Function(...Object.keys(context), `${extract(
    '\t  const buildLocalPrivateViewProofsForRequirements =', '\n\t  const buildLocalCryptoMaterialForRequirements =')}
    return buildLocalPrivateViewProofsForRequirements;`)(...Object.values(context));
  const [proof] = await build([requirement], { liveState: {}, seq: 8 });
  for (const key of ['slot', 'commitment', 'requirementId', 'card']) assert.equal(Object.hasOwn(proof, key), false, key);
  assert.equal(proof.positionCommitment, 'ziffle:fresh:0');
  const plaintext = await decryptPrivateAuditPayload({ keyPair: encryptionKeyPair, encrypted: proof.encryptedOpening }, webcrypto);
  assert.equal(plaintext.opening.slot, 2);
  assert.equal(plaintext.opening.card, 'Mountain');
  assert.equal(plaintext.requirementId, requirement.id);
  assert.equal(disclosures.length, 1);

  const decryptContext = { ...context, ensureAuditIdentity: async () => ({ encryptionKeyPair }),
    decryptPrivateAuditPayload: args => decryptPrivateAuditPayload(args, webcrypto) };
  const decrypt = new Function(...Object.keys(decryptContext), `${extract(
    '  async function privateOpeningFromEncryptedProof(', '\n  async function privateOpeningFromProof(')}
    return privateOpeningFromEncryptedProof;`)(...Object.values(decryptContext));
  assert.equal((await decrypt(proof, {}, { persistDisclosure: false })).card, 'Mountain');
  for (const change of [{ position: 1 }, { positionCommitment: 'ziffle:other:0' }, { owner: 1 }]) {
    await assert.rejects(decrypt({ ...proof, ...change }, {}, { persistDisclosure: false }), /position header/);
  }

  const auditSource = readFileSync(new URL('../src/hooks/peer-lobby/audit-material.js', import.meta.url), 'utf8');
  const from = auditSource.indexOf('  const verifyAuditSatisfiesCryptoRequirements = useCallback(');
  const to = auditSource.indexOf('  function previewAuditOpeningInInspector(', from);
  const auditContext = { ...context, matchStartPayloadRef: { current: { protocolVersion: 15 } },
    localRevealedOpeningForRequirement: () => null,
    zifflePositionFromCommitment: value => Number(value.split(':')[2]) };
  const verify = new Function(...Object.keys(auditContext), `${auditSource.slice(from, to)}
    return verifyAuditSatisfiesCryptoRequirements;`)(...Object.values(auditContext));
  const requirements = [{ ...requirement, publicCommitment: 'ziffle:fresh:0' }];
  await verify({ requirements, audit: { privateViewProofs: [proof] } });
  for (const change of [{ position: 1 }, { positionCommitment: 'ziffle:old:0' },
    { slot: 2 }, { commitment: opening.commitment }, { requirementId: requirement.id }, { card: 'Mountain' }]) {
    await assert.rejects(verify({ requirements, audit: { privateViewProofs: [{ ...proof, ...change }] } }), /ciphertext position|private manifest/);
  }
});
