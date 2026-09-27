import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { createAuditEncryptionKey, exportAuditEncryptionPublicKey, encryptPrivateAuditPayload,
  decryptPrivateAuditPayload, sha256Hex } from '../src/lib/multiplayer-audit.js';

const report = new URL('../../../reports/player-mcp/zkp-private-shuffles-2026-09-27/', import.meta.url);
const captured = JSON.parse(readFileSync(new URL('private-wire-hints-probe.json', report), 'utf8'));
// Reuse an actual post-shuffle hydrated engine requirement from the Serum
// Visions reproduction, with a nonowner viewer to exercise private-view proof
// generation. This is a function-level header probe, not a second card game.
const requirement = { ...captured.knownPrivateRequirements[0], viewer: 1 };
const opening = { owner: requirement.owner, slot: requirement.slot, commitment: requirement.commitment,
  card: requirement.card, objectId: requirement.objectId, position: requirement.publicSlot,
  positionCommitment: requirement.publicCommitment };
const keys = await createAuditEncryptionKey(webcrypto);
const publicKey = await exportAuditEncryptionPublicKey(keys, webcrypto);
const before = process.argv.includes('--before');
const source = readFileSync(before ? new URL('baseline-workspace/web/ui/src/hooks/peer-lobby/crypto-resync.js', report) : new URL('../src/hooks/peer-lobby/crypto-resync.js', import.meta.url), 'utf8');
const start = source.indexOf('const buildLocalPrivateViewProofsForRequirements = useCallback(');
const end = source.indexOf('const buildLocalCryptoMaterialForRequirements = useCallback(', start);
assert.ok(start >= 0 && end > start);
const disclosures = [];
const context = {
  useCallback: callback => callback,
  ziffleDeckHashFromCommitment: value => /^ziffle:([^:]+):[0-9]+$/.exec(value || '')?.[1] || '',
  gameRef: { current: null }, stateRef: { current: {} },
  resolveLocalCryptoPlayerIndex: () => 1,
  isOwnerPrivateViewRequirement: value => Number(value.viewer) === Number(value.owner),
  cryptoMaterialResponsibleSeat: value => Number(value.viewer),
  auditEncryptionPublicKeyForPlayer: () => publicKey,
  buildLocalOpeningFromRequirement: async () => ({ opening, owner: opening.owner,
    position: opening.position, positionCommitment: opening.positionCommitment }),
  sanitizeObjectBoundOpening: async value => value,
  currentAuditMatchId: () => 'private-shuffle-browser',
  currentHiddenCardMetadataForObject: async () => null,
  encryptPrivateAuditPayload: value => encryptPrivateAuditPayload(value, webcrypto),
  rememberPrivateViewDisclosure: value => disclosures.push(value),
  cloneMultiplayerPayload: structuredClone, wasmObjectIdArg: value => value,
  canonicalMultiplayerPayload: JSON.stringify, sha256Hex,
};
const build = new Function(...Object.keys(context), `${source.slice(start, end)}\nreturn buildLocalPrivateViewProofsForRequirements;`)(...Object.values(context));
const proofs = await build([requirement], { seq: 99, liveState: {} });
assert.equal(proofs.length, 1);
const proof = proofs[0];
const decrypted = await decryptPrivateAuditPayload({ keyPair: keys, encrypted: proof.encryptedOpening }, webcrypto);
assert.equal(decrypted.opening.slot, opening.slot);
if (before) {
  assert.equal(proof.slot, opening.slot);
  assert.equal(proof.commitment, opening.commitment);
  assert.equal(proof.requirementId, requirement.id);
  assert.notEqual(proof.slot, proof.position);
} else {
  for (const field of ['slot', 'commitment', 'requirementId']) assert.equal(Object.hasOwn(proof, field), false);
  assert.equal(proof.position, opening.position);
  assert.equal(proof.positionCommitment, opening.positionCommitment);
}
writeFileSync(new URL(before ? 'private-view-header-before.json' : 'private-view-header-after.json', report), JSON.stringify({ requirement,
  opening, proof, disclosure: disclosures[0], scope: 'production proof generator + actual captured engine identity + real AES-GCM; nonowner viewer supplied as fixture' }, null, 2));
console.log(JSON.stringify({ leakedManifestSlot: proof.slot, ciphertextPosition: proof.position,
  leakedRequirementId: proof.requirementId, leakedCommitment: proof.commitment, encrypted: !!proof.encryptedOpening.ciphertextHex }));
