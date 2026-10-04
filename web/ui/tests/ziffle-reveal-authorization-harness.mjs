import { hiddenCardMetadataForObjectFromCheckpoint } from "../src/lib/hidden-card-metadata.js";
import { acceptedZiffleEpochs, buildZiffleInputDeck, assertZiffleEpochInputs, assertZiffleEpochVerification, isPrivateZiffleEpoch, ziffleEpochMaterial, ziffleInputDeckFields } from "../src/lib/ziffle-private-epochs.js";
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { ziffleOriginAnchorFromOpening, ziffleOriginAnchorFromMetadata } from '../src/lib/multiplayer-audit.js';
const source = readFileSync(new URL('../src/hooks/peer-lobby/validation.js', import.meta.url), 'utf8');
const shared = readFileSync(new URL('../src/hooks/peer-lobby/shared.js', import.meta.url), 'utf8');
function declaration(name) {
  const start = shared.indexOf(`export function ${name}(`);
  const end = shared.indexOf('\nexport ', start + 1);
  assert.ok(start >= 0 && end > start, name);
  return shared.slice(start, end).replace(/^export /, '');
}
const helpers = ['ziffleDeckHashFromCommitment', 'zifflePositionFromCommitment', 'zifflePublicPositionFromSources',
  'normalizeShuffleOrder', 'hiddenMetadataMatchesZifflePosition',
  'hiddenObjectIdForOpeningFromCheckpoint', 'checkpointObjectOpeningCardName'];
const visibleStart = source.indexOf('  async function authorizedZiffleRevealPositionsForOwner(');
const visibleEnd = source.indexOf('  async function waitForAuthorizedZiffleRevealPositions(', visibleStart);
const start = source.indexOf('  async function ziffleRevealAuthorizedByOutboundCryptoRequest(');
const end = source.indexOf('  async function answerZiffleRevealTokenRequest(', start);
const verifyStart = source.indexOf('  async function verifyShuffleProofsForRequirements(');
const verifyEnd = source.indexOf('\n  // Applies verified ziffle shuffles', verifyStart);
assert.ok(start >= 0 && end > start);
const buildStart = source.indexOf('  function acceptedEpochsForProof(');
const buildEnd = source.indexOf('  const assertZiffleShuffleProofBoundToSignedMatch', buildStart);
const body = `${source.slice(buildStart, buildEnd)}\n${helpers.map(declaration).join('\n')}\n${source.slice(visibleStart, visibleEnd)}\n${source.slice(start, end)}\n${source.slice(verifyStart, verifyEnd)}\nreturn {
 visiblePositions:authorizedZiffleRevealPositionsForOwner, direct:ziffleRequirementsAuthorizeRevealPositions, metadata:ziffleRequirementsAuthorizeRevealPositionsByMetadata,
 authorize:ziffleRevealAuthorizedByAction, outbound:ziffleRevealAuthorizedByOutboundCryptoRequest,
 preview:previewZiffleActionRequirements, verify:verifyShuffleProofsForRequirements, build:buildLocalShuffleProofsForRequirements };`;
export const commitment = (position, hash = 'deck') => `ziffle:${hash}:${position}`;
export const ceremony = { owner: 0, deckCount: 60, deckHash: 'deck', context: 'match:initial' };
export const opening = (position, fields = {}) => ({ type: 'private_open', owner: 0, viewer: 0, zone: 'library',
  object_id: 100 + position, slot: position, commitment: commitment(position), ...fields });
export const shuffleProof = requirement => ({ owner: requirement.owner, zone: requirement.zone,
  requirementId: requirement.id, deckCount: (requirement.beforeOrder || requirement.before_order).length,
  context: `match:action:8:shuffle:${requirement.id}:${requirement.owner}:${requirement.zone}`,
  keyContext: 'match', deckHash: 'shuffled', keys: ['signed-roster'],
  beforeOrder: requirement.beforeOrder || requirement.before_order,
  afterOrder: requirement.afterOrder || requirement.after_order,
  steps: [{ deckHex: 'shuffled', proofHex: 'valid', signature: 'signed' }] });
export function authorizationHarness({ requirements = [], stored = [], checkpoint = { objects: [] },
  visible = [], visibleToOthers = [], viewable = () => false, lastSequence = 7, previewError = false, decisionPlayer = 0, pending = new Map(), materialRequirements = requirements, disclosureRequirements = [], disclosureDue = false, match = { protocolVersion: 14 }, history = [], overrides = {} } = {}) {
  const command = { type: 'priority_action', action_ref: { kind: 'pass_priority' } };
  const metadata = id => {
    const object = checkpoint.objects.find(value => Number(value.id) === Number(id));
    return object?.hiddenCard ? { objectId: object.id, ...object.hiddenCard } : null;
  };
  const materialCalls = [];
  const context = {
    hiddenCardMetadataForObjectFromCheckpoint,
    acceptedZiffleEpochs, isPrivateZiffleEpoch, buildZiffleInputDeck, ziffleInputDeckFields, assertZiffleEpochInputs, assertZiffleEpochVerification, ziffleEpochMaterial,
    acceptedEpochsForProof: (owner, _seq, preceding) => acceptedZiffleEpochs(match, history, owner, preceding),
    rememberLocalZiffleCeremonyForLookup: () => {},
    matchStartPayloadRef: { current: match },
    actionHistoryRef: { current: history },
    ziffleOriginAnchorFromOpening, ziffleOriginAnchorFromMetadata,
    currentAuditMatchId: () => 'match',
    disclosureDueForPlayer: () => disclosureDue, stateRef: { current: {} },
    sameShuffleOrder: (a, b) => JSON.stringify(a) === JSON.stringify(b),
    shuffleOrderIdMap: () => null, localizeShuffleOrder: order => order,
    shuffleProofMatchesRequirement: (proof, requirement) => proof.requirementId === requirement.id,
    verifiedShuffleProofsRef: { current: new Set() }, nowMonotonicMs: () => 0, recordZiffleShufflePerf: () => {},
    ziffleActionRevealLocksRef: { current: new Map() },
    fairRandomRevealLockConflict: () => false,
    currentHiddenCardMetadataForObject: async id => metadata(id),
    wasmObjectIdArg: value => value,
    gameRef: { current: { getHiddenCardState: async () => checkpoint,
      hiddenObjectViewableBy: async (id, viewer) => viewable(Number(id), Number(viewer)),
      endOfMatchDisclosureRequirements: async () => disclosureRequirements,
      ziffleVerifyShuffle: async input => {
        if (input.steps?.[0]?.proofHex !== 'valid') throw new Error('Invalid shuffle proof');
        return { deckHash: input.steps[0].deckHex, deckCount: input.deckCount,
          ...(input.inputDeck ? { rootDeckHash: input.inputDeck.epochs[0].steps.at(-1).deckHex,
            rootContext: input.inputDeck.epochs[0].context, universeCount: input.inputDeck.universeCount } : {}) };
      },
      previewCryptoRequirementsWithMaterial: async (command, material) => { materialCalls.push({ command, material }); return materialRequirements; },
      uiState: async () => ({ decision: { kind: 'priority', player: decisionPlayer } }) } },
    // Real step signatures are covered by multiplayer-audit.test.js; here a
    // step is signed when the fixture says so.
    assertZiffleCeremonyStepsSigned: async proof => {
      if (!(proof.steps || []).length || proof.steps.some(step => step.signature !== 'signed')) {
        throw new Error('Unsigned ziffle shuffle step');
      }
    },
    assertZiffleShuffleProofBoundToSignedMatch: proof => {
      if (proof.keyContext !== 'match' || JSON.stringify(proof.keys) !== '["signed-roster"]') throw new Error('Invalid roster/context');
    },
    cloneMultiplayerPayload: structuredClone,
    checkpointObjectName: object => object.name,
    checkpointObjectIsRedactedHidden: object => object.name === 'Hidden Card',
    multiplayerRef: { current: { lastAppliedSequence: lastSequence } },
    outboundCryptoMaterialRequestsRef: { current: pending },
    actionHistoryEntryForSequence: () => ({ command, actorIndex: 0 }),
    actionCryptoRequirementsForSequence: () => stored,
    canonicalMultiplayerPayload: JSON.stringify,
    waitForAuthorizedZiffleRevealPositions: async (owner, _deckHash, _positions, _timeout, requester = owner) =>
      new Set(Number(requester) === Number(owner) ? visible : visibleToOthers),
    isDecisionCommandCompatible: (_decision, value) => value.type === 'priority_action',
    verifySignedActionIntent: async intent => { if (intent?.signature !== 'valid') throw new Error('Invalid signature'); },
    verifySequencedActionAudit: async audit => { if (audit.signature !== 'valid') throw new Error('Invalid audit'); },
    currentPublicAuditCheckpointHash: async () => 'checkpoint',
    auditStateHashRef: { current: 'head' }, INITIAL_AUDIT_STATE_HASH: 'initial',
    previewRequirementsForCommand: async () => { if (previewError) throw new Error('Concealed card'); return requirements; },
    toErrorMessage: error => error.message,
    ...overrides,
  };
  const api = new Function(...Object.keys(context), body)(...Object.values(context));
  const message = { actionAuthorization: { matchId: 'match', seq: 8, requesterIndex: 0, actorIndex: 0,
    prevStateHash: 'head', preActionPublicCheckpointHash: 'checkpoint', command,
    requirements: [], actionIntent: { signature: 'valid' } } };
  return { ...api, message, materialCalls };
}
