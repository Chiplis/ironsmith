import test from "node:test";
import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import {
  auditStateHash,
  buildSignedMatchGenesis,
  buildSignedActionEnvelope,
  buildSignedPlayerGenesis,
  buildSignedResyncEnvelope,
  buildDeckSlotOpening,
  buildPrivateDeckManifest,
  canonicalJson,
  publicDeckManifest,
  createAuditSessionKey,
  createAuditEncryptionKey,
  encryptPrivateAuditPayload,
  exportAuditEncryptionPublicKey,
  exportAuditPublicKey,
  importAuditPublicKey,
  verifyCardOpeningAgainstManifest,
  verifyAuditPayload,
  verifyLiveAuditTranscript,
  verifyPrivateViewDisclosure,
  verifySignedMatchGenesis,
  verifySignedResyncEnvelope,
} from "../src/lib/multiplayer-audit.js";

test("canonicalJson sorts object keys recursively", () => {
  assert.equal(
    canonicalJson({ b: 2, a: { d: 4, c: 3 } }),
    "{\"a\":{\"c\":3,\"d\":4},\"b\":2}",
  );
});

test("auditStateHash is stable across key insertion order", async () => {
  const left = await auditStateHash({
    matchId: "m",
    seq: 1,
    prevStateHash: "p",
    command: { type: "pass_priority", actor: 0 },
  }, webcrypto);
  const right = await auditStateHash({
    command: { actor: 0, type: "pass_priority" },
    prevStateHash: "p",
    seq: 1,
    matchId: "m",
  }, webcrypto);
  assert.equal(left, right);
});

test("signed audit envelopes verify and reject tampering", async () => {
  const keyPair = await createAuditSessionKey(webcrypto);
  const envelope = await buildSignedActionEnvelope({
    keyPair,
    matchId: "m",
    seq: 1,
    actor: 0,
    prevStateHash: "p",
    command: { type: "pass_priority" },
  }, webcrypto);
  const publicKeyHex = await exportAuditPublicKey(keyPair, webcrypto);
  const publicKey = await importAuditPublicKey(publicKeyHex, webcrypto);
  const payload = {
    matchId: envelope.matchId,
    seq: envelope.seq,
    actor: envelope.actor,
    signer: envelope.signer,
    prevStateHash: envelope.prevStateHash,
    command: envelope.command,
    openings: envelope.openings,
    rngReveals: envelope.rngReveals,
    shuffleProofs: envelope.shuffleProofs,
    privateViewProofs: envelope.privateViewProofs,
    nextStateHash: envelope.nextStateHash,
  };

  assert.equal(
    await verifyAuditPayload(publicKey, payload, envelope.signature, webcrypto),
    true,
  );
  assert.equal(
    await verifyAuditPayload(
      publicKey,
      { ...payload, command: { type: "draw_cards", player: 0, count: 1 } },
      envelope.signature,
      webcrypto,
    ),
    false,
  );
});

test("private deck manifests commit slots without exposing card names publicly", async () => {
  const manifest = await buildPrivateDeckManifest({
    matchId: "m",
    owner: 1,
    deck: ["Forest", "Lightning Bolt"],
    sideboard: ["Duress"],
    commanders: [],
    saltForSlot: (slot) => `salt-${slot}`,
  }, webcrypto);
  const publicManifest = publicDeckManifest(manifest);

  assert.equal(publicManifest.deckCount, 2);
  assert.equal(publicManifest.sideboardCount, 1);
  assert.equal(JSON.stringify(publicManifest).includes("Lightning Bolt"), false);
  assert.equal(JSON.stringify(publicManifest).includes("salt-1"), false);
  assert.deepEqual(
    await buildDeckSlotOpening({
      manifest,
      slot: 1,
      card: "Lightning Bolt",
    }, webcrypto),
    {
      owner: 1,
      slot: 1,
      card: "Lightning Bolt",
      salt: "salt-1",
      commitment: publicManifest.slotCommitments[1].commitment,
    },
  );
  assert.equal(
    await verifyCardOpeningAgainstManifest({
      manifest: publicManifest,
      slot: 1,
      card: "Lightning Bolt",
      salt: "salt-1",
    }, webcrypto),
    true,
  );
  assert.equal(
    await verifyCardOpeningAgainstManifest({
      manifest: publicManifest,
      slot: 1,
      card: "Counterspell",
      salt: "salt-1",
    }, webcrypto),
    false,
  );
});

test("private-view encrypted disclosure binds signed proof to committed deck slot", async () => {
  const encryptionKey = await createAuditEncryptionKey(webcrypto);
  const recipientPublicKey = await exportAuditEncryptionPublicKey(encryptionKey, webcrypto);
  const manifest = await buildPrivateDeckManifest({
    matchId: "m-private",
    owner: 0,
    deck: ["Ponder"],
    saltForSlot: () => "private-view-salt",
  }, webcrypto);
  const opening = await buildDeckSlotOpening({ manifest, slot: 0 }, webcrypto);
  const payload = {
    type: "private_view_opening",
    matchId: "m-private",
    requirementId: "private_open:0:library:0:1",
    owner: 0,
    viewer: 0,
    zone: "library",
    objectId: 1,
    opening,
  };
  const encryptedOpening = await encryptPrivateAuditPayload({
    recipientPublicKey,
    payload,
  }, webcrypto);
  const proof = {
    type: "encrypted_private_opening",
    requirementId: payload.requirementId,
    owner: 0,
    viewer: 0,
    zone: "library",
    objectId: 1,
    slot: 0,
    commitment: opening.commitment,
    encryptedOpening,
  };
  const verified = await verifyPrivateViewDisclosure({
    proof,
    disclosure: payload,
    manifest: publicDeckManifest(manifest),
  }, webcrypto);
  assert.equal(verified.valid, true);
  await assert.rejects(
    () => verifyPrivateViewDisclosure({
      proof,
      disclosure: {
        ...payload,
        opening: {
          ...payload.opening,
          card: "Black Lotus",
        },
      },
      manifest: publicDeckManifest(manifest),
    }, webcrypto),
    /hash does not match/,
  );
});

test("live audit transcript verifier requires actor-signed actions", async () => {
  const actorKey = await createAuditSessionKey(webcrypto);
  const actorPublicKey = await exportAuditPublicKey(actorKey, webcrypto);
  const command = { type: "pass_priority" };
  const audit = await buildSignedActionEnvelope({
    keyPair: actorKey,
    matchId: "m",
    seq: 1,
    actor: 1,
    prevStateHash: "0".repeat(64),
    command,
  }, webcrypto);
  const transcript = {
    kind: "ironsmith-live-browser-audit-v1",
    initialStateHash: "0".repeat(64),
    players: [
      { seat: 1, auditPublicKey: actorPublicKey },
    ],
    actions: [
      {
        seq: 1,
        actorIndex: 1,
        command,
        label: "Pass",
        audit,
      },
    ],
  };

  const report = await verifyLiveAuditTranscript(transcript, webcrypto);
  assert.equal(report.valid, true);
  assert.equal(report.verifiedActions, 1);
  await assert.rejects(
    () => verifyLiveAuditTranscript({
      ...transcript,
      actions: [{ ...transcript.actions[0], command: { type: "draw_cards" } }],
    }, webcrypto),
    /command mismatch/,
  );
  await assert.rejects(
    () => verifyLiveAuditTranscript({
      ...transcript,
      actions: [
        {
          ...transcript.actions[0],
          audit: { ...audit, signer: 0 },
        },
      ],
      players: [
        { seat: 0, auditPublicKey: actorPublicKey },
        { seat: 1, auditPublicKey: actorPublicKey },
      ],
    }, webcrypto),
    /not signed by the acting player/,
  );
});

test("match genesis and resync envelopes bind roster and checkpoints", async () => {
  const hostKey = await createAuditSessionKey(webcrypto);
  const hostEncryptionKey = await createAuditEncryptionKey(webcrypto);
  const hostPublicKey = await exportAuditPublicKey(hostKey, webcrypto);
  const hostEncryptionPublicKey = await exportAuditEncryptionPublicKey(hostEncryptionKey, webcrypto);
  const manifest = await buildPrivateDeckManifest({
    matchId: "m",
    owner: 0,
    deck: ["Island", "Mountain"],
    saltForSlot: (slot) => `genesis-salt-${slot}`,
  }, webcrypto);
  const player = {
    peerId: "peer-host",
    name: "Host",
    index: 0,
    auditPublicKey: hostPublicKey,
    auditEncryptionPublicKey: hostEncryptionPublicKey,
    deckAuditManifest: publicDeckManifest(manifest),
    ziffleKey: { player: 0, publicKeyHex: "abc" },
    deckCount: 2,
    sideboardCount: 0,
    commanderCount: 0,
  };
  player.playerGenesisSignature = await buildSignedPlayerGenesis({
    keyPair: hostKey,
    matchId: "m",
    protocolVersion: 7,
    timeoutMs: 300000,
    player,
  }, webcrypto);
  const match = {
    protocolVersion: 7,
    auditMatchId: "m",
    lobbyId: "m",
    hostPeerId: "peer-host",
    format: "normal",
    startingLife: 20,
    openingHandSize: 7,
    seed: 123,
    timeoutMs: 300000,
    players: [player],
    deckAuditManifests: [publicDeckManifest(manifest)],
    ziffleKeys: [player.ziffleKey],
    ziffleCeremonies: [],
  };
  match.genesis = await buildSignedMatchGenesis({
    keyPair: hostKey,
    match,
    hostSeat: 0,
  }, webcrypto);

  assert.equal((await verifySignedMatchGenesis(match, webcrypto)).valid, true);
  await assert.rejects(
    () => verifySignedMatchGenesis({
      ...match,
      players: [{ ...player, name: "Impostor" }],
    }, webcrypto),
    /genesis payload hash mismatch|genesis signature/,
  );

  const checkpoint = { players: [{ id: 0, hand: [] }], objects: [] };
  const actions = [];
  const envelope = await buildSignedResyncEnvelope({
    keyPair: hostKey,
    matchId: "m",
    signer: 0,
    lastSequence: 0,
    finalStateHash: "0".repeat(64),
    checkpoint,
    actions,
  }, webcrypto);
  const hostPublicCryptoKey = await importAuditPublicKey(hostPublicKey, webcrypto);
  assert.equal(
    (await verifySignedResyncEnvelope({
      envelope,
      publicKey: hostPublicCryptoKey,
      checkpoint,
      actions,
    }, webcrypto)).valid,
    true,
  );
  await assert.rejects(
    () => verifySignedResyncEnvelope({
      envelope,
      publicKey: hostPublicCryptoKey,
      checkpoint: { ...checkpoint, forged: true },
      actions,
    }, webcrypto),
    /checkpoint hash mismatch/,
  );
});
