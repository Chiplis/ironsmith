// Source-authored production-path regression; not executed in this tranche.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const read = file => readFileSync(new URL(`../src/${file}`, import.meta.url), 'utf8');
function between(source, begin, end) {
  const start = source.indexOf(begin), finish = source.indexOf(end, start + begin.length);
  assert.ok(start >= 0 && finish > start, `Missing production boundary: ${begin}`);
  return source.slice(start, finish);
}
const shared = read('hooks/peer-lobby/shared.js');
const crypto = read('hooks/peer-lobby/crypto-resync.js');
const lobby = read('hooks/usePeerLobby.js');
const matchRequirement = between(shared, 'export function openingMatchesRequirement(', '\nexport function cachedOpeningMatchesZifflePosition')
  .replace(/^export /, '');
const selectionFunctions = between(crypto, '  function commandObjectHiddenRefs(', '\n  async function currentObjectIdForStableId(');
// Inputs use ordinary non-Ziffle commitment strings. Actual production matching
// and both outbound call sites are executed; only encoding-boundary helpers are
// supplied here because importing shared.js requires browser/Vite services.
const filter = new Function('normalizeSelectObjectHiddenRef', 'ziffleDeckHashFromCommitment', 'zifflePositionFromCommitment',
  'openingHasZifflePosition', 'requirementHasZifflePosition',
  `${matchRequirement}\n${selectionFunctions}\nreturn filterOpeningsForCommandHiddenRefs;`)(
    value => value, () => null, () => null, () => false, () => false,
  );
const calls = between(lobby, '        const selectedPostOpenings = filterOpeningsForCommandHiddenRefs(',
  '        const localPostOpeningState =');
function runCalls({ postOpenings, localRequirementOpenings, command, openingRequirements }, source = calls) {
  return new Function('postOpenings', 'localRequirementOpenings', 'command', 'openingRequirements', 'filterOpeningsForCommandHiddenRefs',
    `${source}\nreturn [selectedPostOpenings, selectedLocalRequirementOpenings];`)(
      postOpenings, localRequirementOpenings, command, openingRequirements, filter,
    );
}

test('both outbound paths keep required public proof despite a stale pre-hydration selection reference', () => {
  const required = { owner: 1, commitment: 'opened-current-identity', card: 'Forest' };
  const privateUnrelated = { owner: 1, commitment: 'unselected-private-card', card: 'Island' };
  const wrongOwner = { ...required, owner: 0 };
  const command = { type: 'select_objects', object_hidden_refs: [{ owner: 1, commitment: 'pre-hydration-identity' }] };
  const openingRequirements = [{ type: 'public_open', owner: 1, commitment: required.commitment }];
  const input = { postOpenings: [required, privateUnrelated, wrongOwner],
    localRequirementOpenings: [wrongOwner, required, privateUnrelated], command, openingRequirements };
  assert.deepEqual(runCalls(input), [[required], [required]]);
  // Negative control: the precise previous omission defeats both paths.
  const oldCalls = calls.replace(', command, openingRequirements)', ', command)')
    .replace('          openingRequirements,\n', '');
  assert.deepEqual(runCalls(input, oldCalls), [[], []]);
  assert.deepEqual(runCalls({ ...input, openingRequirements: [{ ...openingRequirements[0], type: 'private_open' }] }), [[], []]);
  assert.deepEqual(runCalls({ ...input, openingRequirements: [{ ...openingRequirements[0], commitment: 'another-identity' }] }), [[], []]);
});
