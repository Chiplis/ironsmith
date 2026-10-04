import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { ziffleOriginAnchorFromMetadata } from '../src/lib/multiplayer-audit.js';
import { normalizeSelectObjectHiddenRef } from '../src/lib/sync-commands.js';

const read = name => readFileSync(new URL(`../src/hooks/peer-lobby/${name}.js`, import.meta.url), 'utf8');
function between(source, start, end) {
  const first = source.indexOf(start), last = source.indexOf(end, first + start.length);
  assert.ok(first >= 0 && last > first, `Missing production function ${start}`);
  return source.slice(first, last).replaceAll('export function ', 'function ');
}
const matching = between(read('shared'), 'export function openingMatchesRequirement(', 'export function cachedOpeningMatchesZifflePosition(');
const openingMatchesRequirement = new Function(`${matching}\nreturn openingMatchesRequirement;`)();
const requirement = { type: 'public_open', owner: 0, slot: 47, commitment: 'ziffle:deck:47', card: 'The Mycosynth Gardens' };
const opening = { owner: 0, slot: 9, commitment: 'physical-card', positionCommitment: 'ziffle:deck:47', card: requirement.card };
function verifier(cached = opening) {
  const ctx = { useCallback: fn => fn, openingMatchesRequirement, ziffleOriginAnchorFromMetadata,
    localRevealedOpeningForRequirement: () => cached,
    auditEncryptionPublicKeyForPlayer: () => '', currentAuditMatchId: () => 'test' };
  const code = between(read('audit-material'), '  const verifyAuditSatisfiesCryptoRequirements = useCallback(', '  function previewAuditOpeningInInspector(');
  return new Function(...Object.keys(ctx), `${code}\nreturn verifyAuditSatisfiesCryptoRequirements;`)(...Object.values(ctx));
}
test('outbound public opening cannot be satisfied by a private/local cache', async () => {
  const verify = verifier();
  await assert.rejects(verify({ requirements: [requirement], audit: { openings: [] }, allowCachedPublicOpenings: false }), /Missing public_open audit opening/);
  await verify({ requirements: [requirement], audit: { openings: [opening] }, allowCachedPublicOpenings: false });
});
test('receiver replay retains its existing cached opening behavior', async () => {
  await verifier()({ requirements: [requirement], audit: { openings: [] } });
  await assert.rejects(verifier(null)({ requirements: [requirement], audit: { openings: [] } }), /Missing public_open audit opening/);
});
test('selection filtering preserves required proofs despite a mismatching reference', () => {
  const ctx = { normalizeSelectObjectHiddenRef, openingMatchesRequirement,
    commandObjectHiddenRefs: command => command.object_hidden_refs,
    ziffleDeckHashFromCommitment: value => /^ziffle:([^:]+):/.exec(value)?.[1] ?? null,
    zifflePositionFromCommitment: value => /^ziffle:[^:]+:(\d+)$/.test(value) ? Number(value.split(':')[2]) : null };
  const code = between(read('crypto-resync'), '  function openingMatchesCommandHiddenRef(', '  async function currentObjectIdForStableId(');
  const filter = new Function(...Object.keys(ctx), `${code}\nreturn filterOpeningsForCommandHiddenRefs;`)(...Object.values(ctx));
  const command = { type: 'select_objects', object_ids: [202], object_hidden_refs: [{ owner: 0, zone: 'hand', slot: 1, commitment: 'stale' }] };
  assert.deepEqual(filter([opening], command), []);
  assert.deepEqual(filter([opening], command, [requirement]), [opening]);
  const unrelated = { ...opening, commitment: 'other', positionCommitment: 'ziffle:deck:48' };
  assert.deepEqual(filter([opening, unrelated], command, [requirement]), [opening]);
});
test('submission checks preview-only public requirements even when applied requirements are nonempty', async () => {
  const submission = readFileSync(new URL('../src/hooks/usePeerLobby.js', import.meta.url), 'utf8');
  const phase = submission.indexOf('"submit_action:verify_audit_satisfies_crypto_requirements"');
  const start = submission.indexOf('() => verifyAuditSatisfiesCryptoRequirements({', phase);
  const marker = '\n          })';
  const end = submission.indexOf(marker, start) + marker.length;
  assert.ok(phase >= 0 && start > phase && end > start);
  const check = new Function('verifyAuditSatisfiesCryptoRequirements', 'cryptoRequirements', 'appliedRequirements', 'audit',
    `return (${submission.slice(start, end)});`);
  const applied = [{ type: 'hidden_move', owner: 0 }];
  await assert.rejects(check(verifier(), [requirement], applied, { openings: [] })(), /Missing public_open audit opening/);
  await check(verifier(), [requirement], applied, { openings: [opening] })();
});
