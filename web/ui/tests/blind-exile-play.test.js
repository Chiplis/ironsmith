// Authored source-only. No JavaScript or engine execution during this campaign.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { actionRefObjectId, actionRefWithObjectId, resolveOpaqueExilePlayCommand, localOpaqueExilePlayCommand, opaqueExileOriginReference } from '../src/lib/sync-object-identity.js';
import { castingMethodOrigin, isDecisionCommandCompatible, serializePriorityCommand, sameActionRef, resolveSyncedCommand } from '../src/lib/sync-commands.js';
import { formatPriorityActionLabel } from '../src/lib/priority-action-groups.js';
import { localReplayCommand } from '../src/lib/audit-replay.js';
import { assertPaymentDisclosureAuthority, createPaymentDisclosureJournal } from '../src/lib/payment-disclosure-journal.js';
const source = path => readFileSync(new URL('../src/' + path, import.meta.url), 'utf8');
const reference = { kind: 'open_exiled_card_for_play', card_id: 41, incarnation: 0, permission: { source: 10, index: 0 } };
const command = { type: 'priority_action', action_ref: reference, object_id: 41,
  object_hidden_ref: { owner: 0, zone: 'exile', slot: 7, commitment: 'exile-incarnation' } };
const action = { index: 3, label: 'Play exiled card', kind: reference.kind, object_id: null,
  from_zone: null, to_zone: null, action_ref: reference };
const decision = { kind: 'priority', player: 1, analysis_complete: true, actions: [action] };
const intent = (overrides = {}) => ({ matchId: 'blind-exile', seq: 4, actorIndex: 1,
  prevStateHash: 'accepted-3', attemptId: 'attempt-A', preActionPublicCheckpointHash: 'public-before',
  signature: 'signature-A', command, ...overrides });
const opening = { owner: 0, slot: 7, objectId: 41, card: 'Opened card', commitment: 'exile-incarnation' };
function storage() { const values = new Map(); return { getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) }; }

// Evaluate the real standalone selector, using the same source-loading pattern
// as the existing foretell test. This never substitutes an engine/proof verdict.
const shared = source('hooks/peer-lobby/shared.js');
function declaration(name) { const start = shared.indexOf(`export function ${name}(`);
  const end = shared.indexOf('\nexport ', start + 1); return shared.slice(start, end).replace(/^export /, ''); }
const collect = new Function('actionRefObjectId', 'castingMethodOrigin',
  ['isFaceDownCastCommand', 'isForetellCommand', 'collectCommandObjectIds'].map(declaration).join('\n')
  + '\nreturn collectCommandObjectIds;')(actionRefObjectId, castingMethodOrigin);

test('opaque opening has one generic label and binds the exact public grant selector', () => {
  assert.equal(formatPriorityActionLabel(action), 'Play exiled card');
  assert.equal(actionRefObjectId(reference), 41);
  assert.deepEqual([...collect(command)], [41]);
  assert.deepEqual(actionRefWithObjectId(reference, 91), { ...reference, card_id: 91 });
  assert.deepEqual(serializePriorityCommand(command, decision), { type: 'priority_action', action_ref: reference, object_id: 41 });
  assert.ok(isDecisionCommandCompatible(decision, command));
  assert.ok(isDecisionCommandCompatible({ ...decision, analysis_complete: false, actions: [] }, command));
  for (const permission of [{ source: 11, index: 0 }, { source: 10, index: 1 }]) {
    const forged = { ...command, action_ref: { ...reference, permission } };
    assert.equal(sameActionRef(reference, forged.action_ref), false);
    assert.equal(isDecisionCommandCompatible(decision, forged), false);
  }
  assert.equal(isDecisionCommandCompatible(decision, { ...command, action_ref: { ...reference, card_id: 42 } }), false);
});

test('new controller owns the signed opening while the card owner supplies its material', () => {
  const backing = storage(); const journal = createPaymentDisclosureJournal(backing);
  const head = { matchId: 'blind-exile', lastAppliedSequence: 3, prevStateHash: 'accepted-3', decisionPlayer: 1 };
  for (const changed of [{ actorIndex: 0 }, { seq: 5 }, { prevStateHash: 'old-head' }]) {
    assert.throws(() => assertPaymentDisclosureAuthority(intent(changed), head));
  }
  assertPaymentDisclosureAuthority(intent(), head);
  journal.pin(intent(), { openings: [opening], evidence: { actionIntent: intent() }, timing: {
    intent: intent(), firstObservedAtMs: 1000, observedElapsedAtIntentMs: 600,
    evidence: { requestId: 'first-material-request', requestedAtMs: 1100, responseTimeoutMs: 500 },
  } });
  const recovered = createPaymentDisclosureJournal(backing);
  assert.deepEqual(recovered.assertCompatible(intent()).openings, [opening]);
  for (const changed of [
    { command: { type: 'cancel_decision' } },
    { command: { ...command, action_ref: { ...reference, card_id: 42 } } },
    { command: { ...command, action_ref: { ...reference, permission: { source: 10, index: 1 } } } },
    { attemptId: 'new-attempt' }, { preActionPublicCheckpointHash: 'new-public-prefix' },
  ]) assert.throws(() => recovered.assertCompatible(intent(changed)));
  recovered.pin(intent(), { timing: { intent: intent(), firstObservedAtMs: 9000, observedElapsedAtIntentMs: 50 } });
  assert.equal(recovered.lookup(intent()).timing.firstObservedAtMs, 1000);
  assert.equal(recovered.lookup(intent()).timing.observedElapsedAtIntentMs, 600);
  assert.equal(recovered.lookup(intent()).signedIntent.attemptId, 'attempt-A');
  recovered.accepted('blind-exile', 3); assert.ok(recovered.lookup(intent()));
  recovered.accepted('blind-exile', 4); assert.equal(recovered.lookup(intent()), null);
});

test('public identity is pinned before responder transmission and before receiver hydration', () => {
  const crypto = source('hooks/peer-lobby/crypto-resync.js');
  const answer = crypto.slice(crypto.indexOf('const answerCryptoMaterialRequest'), crypto.indexOf('const collectRemoteCryptoMaterialForRequirements'));
  assert.ok(answer.indexOf('pinVerifiedPaymentEnvelope(actionIntent, material.openings') < answer.indexOf('type: "crypto_material_response"'));
  const collect = crypto.slice(crypto.indexOf('const collectRemoteCryptoMaterialForRequirements'), crypto.indexOf('const collectRemoteCryptoMaterialForRequirements') + 14000);
  assert.match(collect, /disclosureRecovery\.then\(\(\) => servicesRef\.current\.pinVerifiedPaymentEnvelope/);
  assert.ok(collect.indexOf('await retain;') < collect.indexOf('return response;'));
  const connection = source('hooks/peer-lobby/connections.js');
  const incoming = connection.slice(connection.indexOf('async function pinVerifiedPaymentEnvelope'), connection.indexOf('async function restorePaymentDisclosureAtHead'));
  assert.ok(incoming.indexOf('validatePaymentDisclosureAuthority') < incoming.indexOf('revealAuditOpenings'));
  assert.ok(incoming.indexOf('verifyAuditOpeningsAgainstManifests') < incoming.indexOf('revealAuditOpenings'));
  assert.ok(incoming.indexOf('paymentDisclosureForCommand(localCommand)') < incoming.indexOf('pinPaymentDisclosureIntent(intent'));
});

test('local and incoming optimistic paths classify disclosure before provisional identity use', () => {
  const optimistic = source('hooks/peer-lobby/optimistic-state.js');
  const calculation = optimistic.slice(optimistic.indexOf('calculate: async candidate'), optimistic.indexOf('      restoreVerified,'));
  assert.ok(calculation.indexOf('getPaymentDisclosureForCommand(candidate.command)') < calculation.indexOf('return calculateOptimisticAction'));
  assert.match(calculation, /if \(disclosure\?\.required \|\| disclosure\?\.active\) return null/);
  assert.ok(optimistic.indexOf('getPaymentDisclosureForCommand(command)') < optimistic.indexOf('const publicClaims = []'));
  assert.match(optimistic, /if \(paymentDisclosure\) return;/);
});



test('reveal shares durably pin the original signed opening before a full face payload exists', () => {
  const backing = storage(); const journal = createPaymentDisclosureJournal(backing);
  journal.pin(intent(), { openings: [], evidence: { actionIntent: intent() }, timing: {
    intent: intent(), firstObservedAtMs: 1000, observedElapsedAtIntentMs: 500,
  } });
  const recovered = createPaymentDisclosureJournal(backing);
  assert.throws(() => recovered.assertCompatible(intent({ command: { type: 'cancel_decision' } })), /exact command/);
  recovered.pin(intent(), { openings: [opening], timing: { intent: intent(), firstObservedAtMs: 9000, observedElapsedAtIntentMs: 200 } });
  assert.deepEqual(recovered.lookup(intent()).openings, [opening]);
  assert.equal(recovered.lookup(intent()).timing.firstObservedAtMs, 1000);
  const validation = source('hooks/peer-lobby/validation.js');
  const response = validation.slice(validation.indexOf('async function answerZiffleRevealTokenRequest'), validation.indexOf('function ziffleRoutePeerCandidates'));
  assert.ok(response.indexOf('pinBlindExileOpeningIntent(signedIntent)') < response.indexOf('() => buildLocalZiffleRevealTokens'));
  const connection = source('hooks/peer-lobby/connections.js');
  const pin = connection.slice(connection.indexOf('async function pinBlindExileOpeningIntent'), connection.indexOf('async function pinVerifiedPaymentEnvelope'));
  for (const check of ['validatePaymentDisclosureAuthority(intent)', 'verifySignedActionIntent(intent', 'paymentDisclosureForCommand(localCommand)']) {
    assert.ok(pin.indexOf(check) < pin.indexOf('pinPaymentDisclosureIntent(intent'));
  }
  assert.ok(pin.indexOf('pinPaymentDisclosureIntent(intent') < pin.indexOf('retainPaymentDisclosure(localCommand)'));
});

test('public receipt hashes keep the original stable subject after its runtime ObjectId changes', async () => {
  const { publicCheckpointHash, CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION } = await import('../src/lib/multiplayer-audit.js');
  const { webcrypto } = await import('node:crypto');
  const checkpoint = (cardId, sourceId) => ({ version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION,
    objects: [{ id: cardId + 1, stableId: 77 }, { id: sourceId, stableId: 88 }], players: [], stack: [],
    priorityRuntime: { openedExilePlay: { cardId, cardStableId: 77, player: 1,
      permission: { source: sourceId, index: 0 }, permissionSourceStableId: 88, choicePending: false } } });
  assert.equal(await publicCheckpointHash(checkpoint(41, 10), webcrypto), await publicCheckpointHash(checkpoint(91, 30), webcrypto));
  const changed = checkpoint(41, 10); changed.priorityRuntime.openedExilePlay.permission.index = 1;
  assert.notEqual(await publicCheckpointHash(checkpoint(41, 10), webcrypto), await publicCheckpointHash(changed, webcrypto));
});


// These readers provide committed identity metadata only. The actual shared
// live/transcript adapters run below; no engine legality or proof success is
// replaced by a fake verdict.
const originCheckpoint = (id, incarnation = 0) => ({ objects: [{ id, stableId: 700,
  zone: 'exile', hiddenCard: { owner: 0, slot: 7, commitment: 'exile-incarnation', incarnation } }] });
const metadataReader = checkpoint => ({ getHiddenCardState: async () => checkpoint });

test('live and transcript origin adapters remap legitimate peer IDs without changing the frozen witness', async () => {
  const remote = originCheckpoint(91);
  const wire = JSON.parse(JSON.stringify(command));
  const expected = { ...wire, object_id: 91, action_ref: { ...wire.action_ref, card_id: 91 } };
  assert.deepEqual(resolveOpaqueExilePlayCommand(wire, remote), expected);
  assert.deepEqual(await localOpaqueExilePlayCommand(metadataReader(remote), wire), expected);
  assert.deepEqual(await localReplayCommand(metadataReader(remote), wire), expected);
  assert.deepEqual(wire, command, 'the signed wire command is immutable');
  assert.ok(isDecisionCommandCompatible({ ...decision, actions: [{ ...action, action_ref: expected.action_ref }] }, expected));
  const journal = createPaymentDisclosureJournal(storage());
  journal.pin(intent({ command: wire }), { openings: [opening], evidence: { actionIntent: intent({ command: wire }) } });
  for (let retry = 0; retry < 2; retry++) {
    journal.assertCompatible(intent({ command: wire }));
    assert.deepEqual(await localOpaqueExilePlayCommand(metadataReader(remote), wire), expected);
    assert.deepEqual(await localReplayCommand(metadataReader(remote), wire), expected);
  }
  assert.deepEqual(journal.lookup(intent()).command, wire);
});

test('a stale offered card re-signed at the current head fails live and replay before journal or shares', async () => {
  // Actual native HiddenMove preserves these identity fields but increments the
  // new public counter: exile(0) -> hand(1) -> exile(2).
  const returned = originCheckpoint(91, 2);
  const newlySigned = intent({ seq: 10, prevStateHash: 'accepted-9', attemptId: 'current-head-attempt' });
  const head = { matchId: 'blind-exile', lastAppliedSequence: 9, prevStateHash: 'accepted-9', decisionPlayer: 1 };
  assert.doesNotThrow(() => assertPaymentDisclosureAuthority(newlySigned, head));
  for (const resolve of [localOpaqueExilePlayCommand, localReplayCommand]) {
    const journal = createPaymentDisclosureJournal(storage()); let shares = 0;
    await assert.rejects(async () => {
      assertPaymentDisclosureAuthority(newlySigned, head);
      await resolve(metadataReader(returned), newlySigned.command);
      journal.pin(newlySigned, { evidence: { actionIntent: newlySigned } });
      shares++;
    }, /obsolete or unknown hidden incarnation/);
    assert.deepEqual(journal.entries('blind-exile'), []); assert.equal(shares, 0);
  }
  // The generation is also checked when the runtime ObjectId happens to agree.
  assert.throws(() => resolveOpaqueExilePlayCommand(command, originCheckpoint(41, 2)), /obsolete or unknown/);
});

test('unknown generation and stale identity metadata never become generation zero or a stable fallback', async () => {
  for (const bad of [null, undefined, -1, 0.5, Number.MAX_SAFE_INTEGER + 1]) {
    for (const resolve of [localOpaqueExilePlayCommand, localReplayCommand]) {
      const unknown = originCheckpoint(91); unknown.objects[0].hiddenCard.incarnation = bad;
      await assert.rejects(resolve(metadataReader(unknown), command), /obsolete or unknown/);
      await assert.rejects(resolve(metadataReader(originCheckpoint(91)), { ...command,
        action_ref: { ...reference, incarnation: bad } }), /obsolete or unknown/);
    }
  }
  const noIdentity = { ...command, object_stable_id: 700, object_hidden_ref: undefined };
  await assert.rejects(localReplayCommand(metadataReader(originCheckpoint(91)), noIdentity), /exact current exile origin/);
  const wrong = { ...command, object_hidden_ref: { ...command.object_hidden_ref, commitment: 'other-card' } };
  assert.throws(() => resolveOpaqueExilePlayCommand(wrong, originCheckpoint(91)), /exact current exile origin/);
  // Ordinary cast remapping retains its established stable-card behavior.
  const ordinary = { type: 'priority_action', object_stable_id: 700,
    action_ref: { kind: 'cast_spell', spell_id: 41, from_zone: 'exile', casting_method: { kind: 'normal' } } };
  assert.equal((await localReplayCommand(metadataReader(originCheckpoint(91)), ordinary)).action_ref.spell_id, 91);
});

test('the shared incarnation gate precedes blind replay openings and token authorization', () => {
  const replay = source('lib/audit-replay.js');
  const apply = replay.slice(replay.indexOf('async function applyCurrentAuditReplayAction'), replay.indexOf('// Mirrors END_OF_MATCH_DISCLOSURE_DOMAIN'));
  assert.ok(apply.indexOf('command = await localReplayCommand') < apply.indexOf('await revealAuditOpenings'));
  const validation = source('hooks/peer-lobby/validation.js');
  const token = validation.slice(validation.indexOf('async function ziffleRevealAuthorizedByAction'), validation.indexOf('async function answerZiffleRevealTokenRequest'));
  assert.ok(token.indexOf('localOpaqueExilePlayCommand(gameRef.current, auth.command)') < token.indexOf('const disclosureIntent'));
  const crypto = source('hooks/peer-lobby/crypto-resync.js');
  const authorize = crypto.slice(crypto.indexOf('const authorizedCryptoMaterialRequirementsForRequest'), crypto.indexOf('const answerCryptoMaterialRequest'));
  assert.ok(authorize.indexOf('localOpaqueExilePlayCommand(gameRef.current, command)') < authorize.indexOf('pinBlindExileOpeningIntent(actionIntent)'));
});


test('public ciphertext origin remaps across private hydration without a secret name lookup', async () => {
  const metadata = { owner: 0, zone: 'exile', slot: 7, commitment: 'private-manifest-7',
    publicSlot: 13, publicCommitment: 'ziffle:accepted-epoch:13', incarnation: 4 };
  const ref = opaqueExileOriginReference(metadata);
  assert.deepEqual(ref, { owner: 0, zone: 'exile', slot: 13, commitment: 'ziffle:accepted-epoch:13' });
  const offered = { ...command, action_ref: { ...reference, incarnation: 4 }, object_hidden_ref: ref };
  const unknown = { objects: [{ id: 91, zone: 'exile', hiddenCard: {
    owner: 0, slot: 13, commitment: 'ziffle:accepted-epoch:13', incarnation: 4 } }] };
  const hydrated = { objects: [{ id: 101, zone: 'exile', hiddenCard: metadata }] };
  for (const resolve of [localOpaqueExilePlayCommand, localReplayCommand]) {
    assert.equal((await resolve(metadataReader(unknown), offered)).action_ref.card_id, 91);
    assert.equal((await resolve(metadataReader(hydrated), offered)).action_ref.card_id, 101);
    await assert.rejects(resolve(metadataReader(hydrated), { ...offered,
      object_hidden_ref: { ...ref, slot: 7 } }), /exact current exile origin/, 'mixed private-slot/public-commitment pair is not identity');
    await assert.rejects(resolve(metadataReader(hydrated), { ...offered,
      object_hidden_ref: { owner: 0, zone: 'exile' } }), /complete public hidden identity/);
  }
});

test('the separate uniform face-down intent uses exact incarnation remapping without a public opening', async () => {
  const down = { ...command, action_ref: { ...reference, kind: 'cast_exiled_card_face_down' } };
  assert.deepEqual([...collect(down)], []);
  assert.equal(actionRefObjectId(down.action_ref), 41);
  assert.equal((await localOpaqueExilePlayCommand(metadataReader(originCheckpoint(91)), down)).action_ref.card_id, 91);
  assert.equal((await localReplayCommand(metadataReader(originCheckpoint(91)), down)).action_ref.card_id, 91);
  await assert.rejects(localOpaqueExilePlayCommand(metadataReader(originCheckpoint(91, 2)), down), /obsolete or unknown/);
  await assert.rejects(localReplayCommand(metadataReader(originCheckpoint(91, 2)), down), /obsolete or unknown/);
  const downAction = { ...action, kind: down.action_ref.kind, action_ref: down.action_ref, label: 'Cast exiled card face down' };
  assert.equal(formatPriorityActionLabel(downAction), 'Cast exiled card face down');
  assert.ok(isDecisionCommandCompatible({ ...decision, actions: [downAction] }, down));
  assert.ok(isDecisionCommandCompatible({ ...decision, actions: [], analysis_complete: false }, down));
});


test('tracked same-ID commands require the full paired origin before live or replay material', async () => {
  for (const kind of ['open_exiled_card_for_play', 'cast_exiled_card_face_down']) {
    for (const id of [41, 91]) {
      for (const object_hidden_ref of [undefined, { owner: 0, zone: 'exile' },
        { owner: 0, zone: 'exile', slot: 7 }, { owner: 0, zone: 'exile', commitment: 'exile-incarnation' },
        { owner: 0, zone: 'exile', slot: 7, public_commitment: 'exile-incarnation' }]) {
        const malformed = { ...command, action_ref: { ...reference, kind }, object_hidden_ref };
        for (const resolve of [localOpaqueExilePlayCommand, localReplayCommand]) {
          let materialReleased = false;
          await assert.rejects(async () => {
            await resolve(metadataReader(originCheckpoint(id)), malformed);
            materialReleased = true;
          }, /complete public hidden identity|exact current exile origin/);
          assert.equal(materialReleased, false);
        }
      }
    }
  }
  const untracked = { ...command, action_ref: { ...reference, incarnation: null }, object_hidden_ref: undefined };
  assert.deepEqual(resolveOpaqueExilePlayCommand(untracked, { objects: [{ id: 41, zone: 'exile' }] }), untracked);
});

test('declaration kind hashes normalize captured effect sources after removal on root and inactive lanes', async () => {
  const { publicCheckpointHash, CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION } = await import('../src/lib/multiplayer-audit.js');
  const { webcrypto } = await import('node:crypto');
  const make = (cardId, sourceId, lane) => {
    const receipt = { cardId, incarnation: 2, cardStableId: 77, player: 1,
      permission: { source: sourceId, index: 0 }, permissionSourceStableId: 88,
      kinds: [{ kind: 'morph', permissionSource: null, permissionSourceStableId: null },
        { kind: 'permission', permissionSource: sourceId, permissionSourceStableId: 88 }],
      declaredKind: { kind: 'permission', permissionSource: sourceId, permissionSourceStableId: 88 }, choicePending: true };
    return { version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION, objects: [], players: [], stack: [],
      ...(lane ? { grandMelee: { markers: [{ number: 7, exileFaceDown: receipt }] } }
        : { priorityRuntime: { exileFaceDown: receipt } }) };
  };
  for (const lane of [false, true]) {
    const left = make(41, 10, lane); const right = make(91, 30, lane);
    assert.equal(await publicCheckpointHash(left, webcrypto), await publicCheckpointHash(right, webcrypto));
    const receipt = lane ? right.grandMelee.markers[0].exileFaceDown : right.priorityRuntime.exileFaceDown;
    receipt.declaredKind.permissionSourceStableId = 99;
    assert.notEqual(await publicCheckpointHash(left, webcrypto), await publicCheckpointHash(right, webcrypto));
    receipt.declaredKind.permissionSourceStableId = null;
    await assert.rejects(publicCheckpointHash(right, webcrypto), /captured public permission source identity/);
  }
});


test('wire normalization preserves malformed opaque pairs for strict live and replay rejection', async () => {
  const checkpoint = originCheckpoint(41);
  checkpoint.objects[0].hiddenCard.commitment = 'ziffle:epoch:7';
  for (const hidden of [
    { owner: 0, zone: 'exile', slot: 7, public_commitment: 'ziffle:epoch:7' },
    { owner: 0, zone: 'exile', slot: 7, commitment: 'ziffle:epoch:7', public_slot: 99 },
  ]) {
    const wire = { ...command, object_hidden_ref: hidden };
    const normalized = resolveSyncedCommand(wire);
    assert.deepEqual(normalized.object_hidden_ref, hidden);
    for (const resolve of [localOpaqueExilePlayCommand, localReplayCommand]) {
      await assert.rejects(resolve(metadataReader(checkpoint), normalized), /complete public hidden identity/);
    }
  }
});

test('index-only opaque commands fail before replay opening and live material authorization', async () => {
  for (const kind of ['open_exiled_card_for_play', 'cast_exiled_card_face_down']) {
    const offered = { ...action, action_ref: { ...reference, kind }, kind };
    const currentDecision = { ...decision, actions: [offered] };
    const indexed = { type: 'priority_action', action_index: offered.index };
    assert.equal(isDecisionCommandCompatible(currentDecision, indexed), false);
    // A local UI can still deliberately serialize its row before signing.
    assert.equal(serializePriorityCommand(indexed, currentDecision).action_ref.kind, kind);
    const game = { ...metadataReader(originCheckpoint(41)), uiState: async () => ({ decision: currentDecision }) };
    for (const resolve of [localOpaqueExilePlayCommand, localReplayCommand]) {
      let materialReleased = false;
      await assert.rejects(async () => {
        await resolve(game, resolveSyncedCommand(indexed));
        materialReleased = true;
      }, /explicit frozen action reference/);
      assert.equal(materialReleased, false);
    }
  }
  const replay = source('lib/audit-replay.js');
  const apply = replay.slice(replay.indexOf('async function applyCurrentAuditReplayAction'), replay.indexOf('// Mirrors END_OF_MATCH_DISCLOSURE_DOMAIN'));
  assert.ok(apply.indexOf('command = await localOpaqueExilePlayCommand') < apply.indexOf('await revealAuditOpenings'));
});
