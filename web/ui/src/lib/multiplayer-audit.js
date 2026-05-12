const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();
const INITIAL_MATCH_CLOCK_HASH = "0".repeat(64);
const MATCH_CLOCK_AUDIT_DOMAIN = "ironsmith-match-clock-audit-v1";
const ACTION_QUORUM_CERTIFICATE_TYPE = "ironsmith-action-quorum-v1";
const ACTION_QUORUM_VOTE_DOMAIN = "ironsmith-action-quorum-vote-v1";
export const CURRENT_AUDIT_PROTOCOL_VERSION = 10;
export const CURRENT_AUDIT_MIN_PLAYERS = 2;
export const CURRENT_AUDIT_MAX_PLAYERS = 4;

export function normalizeAuditPlayerCount(playerCount) {
  const count = Number(playerCount);
  return Number.isInteger(count) ? count : 0;
}

export function isCurrentAuditPlayerCount(playerCount) {
  const count = normalizeAuditPlayerCount(playerCount);
  return count >= CURRENT_AUDIT_MIN_PLAYERS && count <= CURRENT_AUDIT_MAX_PLAYERS;
}

export function assertCurrentAuditPlayerCount(playerCount, context = "Current audit protocol") {
  const count = normalizeAuditPlayerCount(playerCount);
  if (!isCurrentAuditPlayerCount(count)) {
    throw new Error(
      `${context} requires 2, 3, or 4 players`
    );
  }
  return count;
}

export function canonicalJson(value) {
  return JSON.stringify(normalizeForJson(value));
}

function normalizeForJson(value) {
  if (Array.isArray(value)) {
    return value.map(normalizeForJson);
  }
  if (value && typeof value === "object") {
    return Object.keys(value)
      .sort()
      .reduce((out, key) => {
        const normalized = normalizeForJson(value[key]);
        if (normalized !== undefined) {
          out[key] = normalized;
        }
        return out;
      }, {});
  }
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new Error("Cannot canonicalize non-finite number");
    }
    return Object.is(value, -0) ? 0 : value;
  }
  if (typeof value === "bigint") {
    return value.toString();
  }
  if (typeof value === "function" || typeof value === "symbol") {
    return undefined;
  }
  return value;
}

function stripTransientMetadata(value) {
  if (Array.isArray(value)) {
    return value.map(stripTransientMetadata);
  }
  if (value && typeof value === "object") {
    return Object.keys(value)
      .sort()
      .reduce((out, key) => {
        if (String(key).startsWith("__")) return out;
        const stripped = stripTransientMetadata(value[key]);
        if (stripped !== undefined) out[key] = stripped;
        return out;
      }, {});
  }
  return value;
}

export async function sha256Hex(value, cryptoImpl = globalThis.crypto) {
  if (!cryptoImpl?.subtle) {
    throw new Error("WebCrypto subtle API is unavailable");
  }
  const bytes = typeof value === "string" ? textEncoder.encode(value) : value;
  const digest = await cryptoImpl.subtle.digest("SHA-256", bytes);
  return bytesToHex(new Uint8Array(digest));
}

export async function createAuditSessionKey(cryptoImpl = globalThis.crypto) {
  if (!cryptoImpl?.subtle) {
    throw new Error("WebCrypto subtle API is unavailable");
  }
  return cryptoImpl.subtle.generateKey(
    {
      name: "ECDSA",
      namedCurve: "P-256",
    },
    true,
    ["sign", "verify"],
  );
}

export async function exportAuditKeyPair(keyPair, cryptoImpl = globalThis.crypto) {
  return {
    privateJwk: await cryptoImpl.subtle.exportKey("jwk", keyPair.privateKey),
    publicJwk: await cryptoImpl.subtle.exportKey("jwk", keyPair.publicKey),
  };
}

export async function importAuditKeyPair(serialized, cryptoImpl = globalThis.crypto) {
  if (!serialized?.privateJwk) {
    throw new Error("Missing stored audit private key");
  }
  const privateJwk = {
    ...serialized.privateJwk,
    key_ops: ["sign"],
    ext: true,
  };
  const publicJwk = serialized.publicJwk
    ? {
        ...serialized.publicJwk,
        key_ops: ["verify"],
        ext: true,
      }
    : {
        kty: privateJwk.kty,
        crv: privateJwk.crv,
        x: privateJwk.x,
        y: privateJwk.y,
        key_ops: ["verify"],
        ext: true,
      };
  const [privateKey, publicKey] = await Promise.all([
    cryptoImpl.subtle.importKey(
      "jwk",
      privateJwk,
      {
        name: "ECDSA",
        namedCurve: "P-256",
      },
      true,
      ["sign"],
    ),
    cryptoImpl.subtle.importKey(
      "jwk",
      publicJwk,
      {
        name: "ECDSA",
        namedCurve: "P-256",
      },
      true,
      ["verify"],
    ),
  ]);
  return { privateKey, publicKey };
}

export async function exportAuditPublicKey(keyPair, cryptoImpl = globalThis.crypto) {
  const raw = await cryptoImpl.subtle.exportKey("raw", keyPair.publicKey);
  return bytesToHex(new Uint8Array(raw));
}

export async function createAuditEncryptionKey(cryptoImpl = globalThis.crypto) {
  if (!cryptoImpl?.subtle) {
    throw new Error("WebCrypto subtle API is unavailable");
  }
  return cryptoImpl.subtle.generateKey(
    {
      name: "ECDH",
      namedCurve: "P-256",
    },
    true,
    ["deriveKey"],
  );
}

export async function exportAuditEncryptionKeyPair(keyPair, cryptoImpl = globalThis.crypto) {
  return {
    encryptionPrivateJwk: await cryptoImpl.subtle.exportKey("jwk", keyPair.privateKey),
    encryptionPublicJwk: await cryptoImpl.subtle.exportKey("jwk", keyPair.publicKey),
    auditEncryptionPublicKey: await exportAuditEncryptionPublicKey(keyPair, cryptoImpl),
  };
}

export async function importAuditEncryptionKeyPair(serialized, cryptoImpl = globalThis.crypto) {
  const privateJwk = serialized?.encryptionPrivateJwk || serialized?.auditEncryptionPrivateJwk;
  if (!privateJwk) {
    throw new Error("Missing stored audit encryption private key");
  }
  const publicJwk = serialized?.encryptionPublicJwk || {
    kty: privateJwk.kty,
    crv: privateJwk.crv,
    x: privateJwk.x,
    y: privateJwk.y,
    key_ops: [],
    ext: true,
  };
  const [privateKey, publicKey] = await Promise.all([
    cryptoImpl.subtle.importKey(
      "jwk",
      {
        ...privateJwk,
        key_ops: ["deriveKey"],
        ext: true,
      },
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true,
      ["deriveKey"],
    ),
    cryptoImpl.subtle.importKey(
      "jwk",
      {
        ...publicJwk,
        key_ops: [],
        ext: true,
      },
      {
        name: "ECDH",
        namedCurve: "P-256",
      },
      true,
      [],
    ),
  ]);
  return { privateKey, publicKey };
}

export async function exportAuditEncryptionPublicKey(keyPair, cryptoImpl = globalThis.crypto) {
  const raw = await cryptoImpl.subtle.exportKey("raw", keyPair.publicKey);
  return bytesToHex(new Uint8Array(raw));
}

export async function importAuditEncryptionPublicKey(rawHex, cryptoImpl = globalThis.crypto) {
  return cryptoImpl.subtle.importKey(
    "raw",
    hexToBytes(rawHex),
    {
      name: "ECDH",
      namedCurve: "P-256",
    },
    true,
    [],
  );
}

async function derivePrivateAuditAesKey({
  privateKey,
  publicKey,
}, cryptoImpl = globalThis.crypto) {
  return cryptoImpl.subtle.deriveKey(
    {
      name: "ECDH",
      public: publicKey,
    },
    privateKey,
    {
      name: "AES-GCM",
      length: 256,
    },
    false,
    ["encrypt", "decrypt"],
  );
}

export async function privateViewPlaintextHash(payload, cryptoImpl = globalThis.crypto) {
  return sha256Hex(canonicalJson({
    domain: "ironsmith-private-view-plaintext-v1",
    payload,
  }), cryptoImpl);
}

export async function encryptPrivateAuditPayload({
  recipientPublicKey,
  payload,
}, cryptoImpl = globalThis.crypto) {
  if (!recipientPublicKey) {
    throw new Error("Missing private-view recipient encryption key");
  }
  const recipientKey = await importAuditEncryptionPublicKey(recipientPublicKey, cryptoImpl);
  const ephemeralKeyPair = await createAuditEncryptionKey(cryptoImpl);
  const aesKey = await derivePrivateAuditAesKey({
    privateKey: ephemeralKeyPair.privateKey,
    publicKey: recipientKey,
  }, cryptoImpl);
  const ivHex = randomHex(cryptoImpl, 12);
  const plaintext = canonicalJson(payload);
  const ciphertext = await cryptoImpl.subtle.encrypt(
    {
      name: "AES-GCM",
      iv: hexToBytes(ivHex),
    },
    aesKey,
    textEncoder.encode(plaintext),
  );
  return {
    scheme: "ecdh-p256-aes-gcm-sha256",
    recipientPublicKey: String(recipientPublicKey),
    ephemeralPublicKey: await exportAuditEncryptionPublicKey(ephemeralKeyPair, cryptoImpl),
    ivHex,
    ciphertextHex: bytesToHex(new Uint8Array(ciphertext)),
    plaintextHash: await privateViewPlaintextHash(payload, cryptoImpl),
  };
}

export async function decryptPrivateAuditPayload({
  keyPair,
  encrypted,
}, cryptoImpl = globalThis.crypto) {
  if (!keyPair?.privateKey) {
    throw new Error("Missing private-view decryption key");
  }
  if (String(encrypted?.scheme || "") !== "ecdh-p256-aes-gcm-sha256") {
    throw new Error("Unsupported private-view encryption scheme");
  }
  const ephemeralPublicKey = await importAuditEncryptionPublicKey(encrypted.ephemeralPublicKey, cryptoImpl);
  const aesKey = await derivePrivateAuditAesKey({
    privateKey: keyPair.privateKey,
    publicKey: ephemeralPublicKey,
  }, cryptoImpl);
  const plaintextBytes = await cryptoImpl.subtle.decrypt(
    {
      name: "AES-GCM",
      iv: hexToBytes(encrypted.ivHex),
    },
    aesKey,
    hexToBytes(encrypted.ciphertextHex),
  );
  const payload = JSON.parse(textDecoder.decode(plaintextBytes));
  const plaintextHash = await privateViewPlaintextHash(payload, cryptoImpl);
  if (plaintextHash !== String(encrypted.plaintextHash || "")) {
    throw new Error("Private-view plaintext hash mismatch");
  }
  return payload;
}

export async function verifyPrivateViewDisclosure({
  proof,
  disclosure,
  manifest,
}, cryptoImpl = globalThis.crypto) {
  if (!proof?.encryptedOpening?.plaintextHash) {
    throw new Error("Private-view proof is missing its plaintext hash");
  }
  const payload = disclosure?.payload || disclosure;
  const plaintextHash = await privateViewPlaintextHash(payload, cryptoImpl);
  if (plaintextHash !== String(proof.encryptedOpening.plaintextHash || "")) {
    throw new Error("Private-view disclosure hash does not match the signed proof");
  }
  const opening = payload?.opening;
  if (!opening) {
    throw new Error("Private-view disclosure is missing the card opening");
  }
  const valid = await verifyCardOpeningAgainstManifest({
    manifest,
    slot: opening.slot,
    card: opening.card,
    salt: opening.salt,
  }, cryptoImpl);
  if (!valid) {
    throw new Error("Private-view disclosure does not match the committed deck manifest");
  }
  if (proof.commitment && String(proof.commitment) !== String(opening.commitment || "")) {
    throw new Error("Private-view disclosure commitment does not match the signed proof");
  }
  return {
    valid: true,
    plaintextHash,
  };
}

export async function signAuditPayload(keyPair, payload, cryptoImpl = globalThis.crypto) {
  const canonical = canonicalJson(payload);
  const signature = await cryptoImpl.subtle.sign(
    {
      name: "ECDSA",
      hash: "SHA-256",
    },
    keyPair.privateKey,
    textEncoder.encode(canonical),
  );
  return bytesToHex(new Uint8Array(signature));
}

export async function verifyAuditPayload(
  publicKey,
  payload,
  signatureHex,
  cryptoImpl = globalThis.crypto,
) {
  const canonical = canonicalJson(payload);
  return cryptoImpl.subtle.verify(
    {
      name: "ECDSA",
      hash: "SHA-256",
    },
    publicKey,
    hexToBytes(signatureHex),
    textEncoder.encode(canonical),
  );
}

export async function importAuditPublicKey(rawHex, cryptoImpl = globalThis.crypto) {
  return cryptoImpl.subtle.importKey(
    "raw",
    hexToBytes(rawHex),
    {
      name: "ECDSA",
      namedCurve: "P-256",
    },
    true,
    ["verify"],
  );
}

export async function auditStateHash({
  matchId,
  seq,
  prevStateHash,
  command,
  clock,
  openings = [],
  rngReveals = [],
  shuffleProofs = [],
  privateViewProofs = [],
  publicCheckpointHash,
}, cryptoImpl = globalThis.crypto) {
  return sha256Hex(
    canonicalJson({
      domain: "ironsmith-ui-audit-state-v1",
      matchId,
      seq,
      prevStateHash,
      command,
      clock,
      openings,
      rngReveals,
      shuffleProofs,
      privateViewProofs,
      publicCheckpointHash,
    }),
    cryptoImpl,
  );
}

export async function buildSignedActionEnvelope({
  keyPair,
  matchId,
  seq,
  actor,
  signer = actor,
  prevStateHash,
  command,
  clock,
  openings = [],
  rngReveals = [],
  shuffleProofs = [],
  privateViewProofs = [],
  publicCheckpointHash,
}, cryptoImpl = globalThis.crypto) {
  const nextStateHash = await auditStateHash({
    matchId,
    seq,
    prevStateHash,
    command,
    clock,
    openings,
    rngReveals,
    shuffleProofs,
    privateViewProofs,
    publicCheckpointHash,
  }, cryptoImpl);
  const payload = {
    matchId,
    seq,
    actor,
    signer,
    prevStateHash,
    command,
    clock,
    openings,
    rngReveals,
    shuffleProofs,
    privateViewProofs,
    publicCheckpointHash,
    nextStateHash,
  };
  return {
    ...payload,
    signatureAlgorithm: "ecdsa-p256-sha256",
    signature: await signAuditPayload(keyPair, payload, cryptoImpl),
  };
}

export function actionQuorumThreshold(playerCount) {
  const count = assertCurrentAuditPlayerCount(playerCount, "Action quorum");
  return count < 3 ? 0 : 3;
}

function actionQuorumExpected(action) {
  const audit = action?.audit || action || {};
  return {
    matchId: String(audit.matchId || ""),
    seq: Number(audit.seq || action?.seq || 0),
    actor: Number(audit.actor ?? action?.actorIndex ?? 0),
    prevStateHash: String(audit.prevStateHash || ""),
    nextStateHash: String(audit.nextStateHash || ""),
    publicCheckpointHash: String(audit.publicCheckpointHash || ""),
    actionSignature: String(audit.signature || ""),
  };
}

export function actionQuorumVotePayload({
  matchId,
  seq,
  actor,
  voter,
  prevStateHash,
  nextStateHash,
  publicCheckpointHash,
  actionSignature,
}) {
  return {
    domain: ACTION_QUORUM_VOTE_DOMAIN,
    matchId: String(matchId || ""),
    seq: Number(seq || 0),
    actor: Number(actor),
    voter: Number(voter),
    prevStateHash: String(prevStateHash || ""),
    nextStateHash: String(nextStateHash || ""),
    publicCheckpointHash: String(publicCheckpointHash || ""),
    actionSignature: String(actionSignature || ""),
  };
}

export async function buildSignedActionQuorumVote({
  keyPair,
  action,
  voter,
}, cryptoImpl = globalThis.crypto) {
  const payload = actionQuorumVotePayload({
    ...actionQuorumExpected(action),
    voter,
  });
  return {
    ...payload,
    signatureAlgorithm: "ecdsa-p256-sha256",
    signature: await signAuditPayload(keyPair, payload, cryptoImpl),
  };
}

export async function verifyActionQuorumVote({
  vote,
  action,
  players = [],
}, cryptoImpl = globalThis.crypto) {
  const voter = Number(vote?.voter);
  if (!Number.isInteger(voter) || voter < 0) {
    throw new Error("Action quorum vote contains an invalid voter");
  }
  const expected = actionQuorumVotePayload({
    ...actionQuorumExpected(action),
    voter,
  });
  if (
    String(vote?.domain || "") !== ACTION_QUORUM_VOTE_DOMAIN
    || String(vote?.matchId || "") !== expected.matchId
    || Number(vote?.seq) !== expected.seq
    || Number(vote?.actor) !== expected.actor
    || String(vote?.prevStateHash || "") !== expected.prevStateHash
    || String(vote?.nextStateHash || "") !== expected.nextStateHash
    || String(vote?.publicCheckpointHash || "") !== expected.publicCheckpointHash
    || String(vote?.actionSignature || "") !== expected.actionSignature
  ) {
    throw new Error("Action quorum vote does not match the signed action");
  }
  const player = (players || []).find((entry) =>
    Number(entry?.index ?? entry?.seat) === voter
  );
  if (!player?.auditPublicKey) {
    throw new Error(`Action quorum voter ${voter + 1} is not in the match roster`);
  }
  const publicKey = await importAuditPublicKey(player.auditPublicKey, cryptoImpl);
  const valid = await verifyAuditPayload(
    publicKey,
    expected,
    vote.signature || "",
    cryptoImpl,
  );
  if (!valid) {
    throw new Error("Action quorum vote signature is invalid");
  }
  return voter;
}

export async function verifyActionQuorumCertificate({
  certificate,
  action,
  players = [],
  threshold: requiredThreshold = null,
}, cryptoImpl = globalThis.crypto) {
  const roster = Array.isArray(players) ? players : [];
  const hasThresholdOverride =
    requiredThreshold !== null
    && requiredThreshold !== undefined
    && requiredThreshold !== "";
  const threshold = hasThresholdOverride && Number.isInteger(Number(requiredThreshold))
    ? Math.max(0, Number(requiredThreshold))
    : actionQuorumThreshold(roster.length);
  if (threshold <= 0) {
    return {
      valid: true,
      threshold: 0,
      voters: [],
    };
  }
  if (!certificate || typeof certificate !== "object") {
    throw new Error("Sequenced action is missing its quorum certificate");
  }
  if (String(certificate.type || "") !== ACTION_QUORUM_CERTIFICATE_TYPE) {
    throw new Error("Sequenced action has an unsupported quorum certificate");
  }
  const expected = actionQuorumExpected(action);
  if (
    String(certificate.matchId || "") !== expected.matchId
    || Number(certificate.seq) !== expected.seq
    || Number(certificate.actor) !== expected.actor
    || String(certificate.prevStateHash || "") !== expected.prevStateHash
    || String(certificate.nextStateHash || "") !== expected.nextStateHash
    || String(certificate.publicCheckpointHash || "") !== expected.publicCheckpointHash
    || String(certificate.actionSignature || "") !== expected.actionSignature
  ) {
    throw new Error("Action quorum certificate does not match the signed action");
  }
  const votes = Array.isArray(certificate.votes) ? certificate.votes : [];
  if (votes.length < threshold) {
    throw new Error(
      `Action quorum certificate has ${votes.length} vote(s), expected at least ${threshold}`
    );
  }
  const seen = new Set();
  for (const vote of votes) {
    const voter = await verifyActionQuorumVote({
      vote,
      action,
      players: roster,
    }, cryptoImpl);
    if (seen.has(voter)) {
      throw new Error("Action quorum certificate contains a duplicate voter");
    }
    seen.add(voter);
  }
  if (seen.size < threshold) {
    throw new Error(
      `Action quorum certificate has ${seen.size} unique vote(s), expected at least ${threshold}`
    );
  }
  return {
    valid: true,
    threshold,
    voters: [...seen].sort((left, right) => left - right),
  };
}

export function rngCommitmentPayload({
  matchId,
  seq,
  requirementId,
  requestId,
  requester,
  player,
  commitmentHex,
}) {
  return {
    domain: "ironsmith-rng-commit-response-v1",
    matchId: String(matchId || ""),
    seq: Number(seq),
    requirementId: String(requirementId || ""),
    requestId: String(requestId || ""),
    requester: Number(requester),
    player: Number(player),
    commitmentHex: String(commitmentHex || ""),
  };
}

export function rngRevealPayload({
  matchId,
  seq,
  requirementId,
  requestId,
  commitRequestId,
  requester,
  player,
  nonceHex,
  commitmentHex,
}) {
  return {
    domain: "ironsmith-rng-reveal-response-v1",
    matchId: String(matchId || ""),
    seq: Number(seq),
    requirementId: String(requirementId || ""),
    requestId: String(requestId || ""),
    commitRequestId: String(commitRequestId || ""),
    requester: Number(requester),
    player: Number(player),
    nonceHex: String(nonceHex || ""),
    commitmentHex: String(commitmentHex || ""),
  };
}

async function verifyMatchClockAuditChainEntry({
  clock,
  expectedClockHash,
  expectedSeq,
  expectedActor,
}, cryptoImpl = globalThis.crypto) {
  if (!clock) return expectedClockHash;
  if (String(clock.type || "") !== "match_clock_v1") {
    throw new Error(`Unsupported match clock audit at sequence ${expectedSeq}`);
  }
  if (Number(clock.seq) !== Number(expectedSeq) || Number(clock.actor) !== Number(expectedActor)) {
    throw new Error(`Match clock audit does not match action ${expectedSeq}`);
  }
  if (String(clock.previousClockHash || "") !== String(expectedClockHash || INITIAL_MATCH_CLOCK_HASH)) {
    throw new Error(`Match clock hash mismatch at sequence ${expectedSeq}`);
  }
  const payload = { ...clock };
  delete payload.clockHash;
  const computedHash = await sha256Hex(canonicalJson({
    domain: MATCH_CLOCK_AUDIT_DOMAIN,
    clock: payload,
  }), cryptoImpl);
  if (computedHash !== String(clock.clockHash || "")) {
    throw new Error(`Match clock audit hash mismatch at sequence ${expectedSeq}`);
  }
  return String(clock.clockHash || "");
}

export function publicPlayerGenesisRecord(player) {
  if (!player || typeof player !== "object") return null;
  return {
    name: String(player.name || ""),
    index: Number(player.index ?? player.seat ?? 0),
    auditPublicKey: String(player.auditPublicKey || ""),
    auditEncryptionPublicKey: String(player.auditEncryptionPublicKey || ""),
    deckAuditManifest: publicDeckManifest(player.deckAuditManifest),
    ziffleKey: player.ziffleKey || null,
    deckCount: Number(player.deckCount || player.deckAuditManifest?.deckCount || 0),
    sideboardCount: Number(player.sideboardCount || player.deckAuditManifest?.sideboardCount || 0),
    commanderCount: Number(player.commanderCount || player.deckAuditManifest?.commanderCount || 0),
  };
}

export function playerGenesisPayload({
  matchId,
  protocolVersion,
  timeoutMs,
  player,
}) {
  const playerRecord = publicPlayerGenesisRecord(player);
  if (playerRecord) {
    delete playerRecord.index;
    if (playerRecord.ziffleKey && typeof playerRecord.ziffleKey === "object") {
      playerRecord.ziffleKey = {
        ...playerRecord.ziffleKey,
        player: undefined,
      };
    }
  }
  return {
    domain: "ironsmith-player-genesis-v1",
    matchId: String(matchId || ""),
    protocolVersion: Number(protocolVersion || 0),
    timeoutMs: Number(timeoutMs || 0),
    player: playerRecord,
  };
}

export async function buildSignedPlayerGenesis({
  keyPair,
  matchId,
  protocolVersion,
  timeoutMs,
  player,
}, cryptoImpl = globalThis.crypto) {
  const payload = playerGenesisPayload({
    matchId,
    protocolVersion,
    timeoutMs,
    player,
  });
  return {
    signer: Number(player?.index ?? player?.seat ?? 0),
    payloadHash: await sha256Hex(canonicalJson(payload), cryptoImpl),
    signatureAlgorithm: "ecdsa-p256-sha256",
    signature: await signAuditPayload(keyPair, payload, cryptoImpl),
  };
}

export async function verifySignedPlayerGenesis({
  player,
  matchId,
  protocolVersion,
  timeoutMs,
}, cryptoImpl = globalThis.crypto) {
  const signature = player?.playerGenesisSignature;
  if (!signature || typeof signature !== "object") {
    throw new Error(`Player ${Number(player?.index ?? 0)} is missing genesis signature`);
  }
  const payload = playerGenesisPayload({
    matchId,
    protocolVersion,
    timeoutMs,
    player,
  });
  const payloadHash = await sha256Hex(canonicalJson(payload), cryptoImpl);
  if (payloadHash !== String(signature.payloadHash || "")) {
    throw new Error(`Player ${Number(player?.index ?? 0)} genesis payload hash mismatch`);
  }
  const signerKey = await importAuditPublicKey(player?.auditPublicKey || "", cryptoImpl);
  const valid = await verifyAuditPayload(
    signerKey,
    payload,
    signature.signature || "",
    cryptoImpl,
  );
  if (!valid) {
    throw new Error(`Player ${Number(player?.index ?? 0)} genesis signature is invalid`);
  }
  return true;
}

export function matchGenesisPayload(match) {
  const players = Array.isArray(match?.players)
    ? [...match.players]
        .map(publicPlayerGenesisRecord)
        .sort((left, right) => Number(left.index) - Number(right.index))
    : [];
  return {
    domain: "ironsmith-match-genesis-v1",
    protocolVersion: Number(match?.protocolVersion || 0),
    matchId: String(match?.auditMatchId || match?.matchId || match?.lobbyId || match?.hostPeerId || ""),
    lobbyId: String(match?.lobbyId || ""),
    hostPeerId: String(match?.hostPeerId || ""),
    format: String(match?.format || ""),
    startingLife: Number(match?.startingLife || 0),
    openingHandSize: Number(match?.openingHandSize || 0),
    seed: Number(match?.seed || 0),
    timeoutMs: Number(match?.timeoutMs || 0),
    initialPublicCheckpointHash: String(match?.initialPublicCheckpointHash || ""),
    matchClockPolicy: match?.matchClockPolicy
      ? {
          type: String(match.matchClockPolicy.type || "per_player_match_clock_v1"),
          initialMs: Number(match.matchClockPolicy.initialMs || match?.timeoutMs || 0),
          graceMs: Number(match.matchClockPolicy.graceMs || 0),
        }
      : null,
    players,
    deckAuditManifests: Array.isArray(match?.deckAuditManifests)
      ? match.deckAuditManifests.map(publicDeckManifest)
      : [],
    ziffleKeys: Array.isArray(match?.ziffleKeys) ? match.ziffleKeys : [],
    ziffleCeremonies: Array.isArray(match?.ziffleCeremonies)
      ? match.ziffleCeremonies.map((ceremony) => ({
          owner: Number(ceremony.owner),
          deckCount: Number(ceremony.deckCount || 0),
          context: String(ceremony.context || ""),
          keyContext: String(ceremony.keyContext || ceremony.context || ""),
          keys: ceremony.keys || [],
          steps: ceremony.steps || [],
          deckHash: String(ceremony.deckHash || ""),
        }))
      : [],
  };
}

export async function buildSignedMatchGenesis({
  keyPair,
  match,
  hostSeat = 0,
}, cryptoImpl = globalThis.crypto) {
  const payload = matchGenesisPayload(match);
  return {
    kind: "ironsmith-match-genesis-v1",
    hostSeat: Number(hostSeat),
    payloadHash: await sha256Hex(canonicalJson(payload), cryptoImpl),
    signatureAlgorithm: "ecdsa-p256-sha256",
    hostSignature: await signAuditPayload(keyPair, payload, cryptoImpl),
  };
}

export async function verifySignedMatchGenesis(match, cryptoImpl = globalThis.crypto) {
  const genesis = match?.genesis;
  if (!genesis || genesis.kind !== "ironsmith-match-genesis-v1") {
    throw new Error("Match start payload is missing signed genesis");
  }
  if (Number(match?.protocolVersion || 0) !== CURRENT_AUDIT_PROTOCOL_VERSION) {
    throw new Error("Match genesis uses an unsupported protocol version");
  }
  const players = Array.isArray(match?.players) ? match.players : [];
  const playerCount = assertCurrentAuditPlayerCount(players.length, "Match genesis");
  const seats = players
    .map((player) => normalizeAuditPlayerCount(player?.index))
    .sort((left, right) => left - right);
  for (let offset = 0; offset < playerCount; offset += 1) {
    if (seats[offset] !== offset) {
      throw new Error("Match genesis requires contiguous player seats");
    }
  }
  if (!String(match?.initialPublicCheckpointHash || "")) {
    throw new Error("Match genesis is missing its initial public checkpoint hash");
  }
  if (!match?.matchClockPolicy || match.matchClockPolicy.type !== "per_player_match_clock_v1") {
    throw new Error("Match genesis is missing the current match clock policy");
  }
  const deckAuditManifests = Array.isArray(match?.deckAuditManifests)
    ? match.deckAuditManifests
    : [];
  if (deckAuditManifests.length !== playerCount) {
    throw new Error("Match genesis requires one deck audit manifest per player");
  }
  const ziffleKeys = Array.isArray(match?.ziffleKeys) ? match.ziffleKeys : [];
  if (ziffleKeys.length !== playerCount) {
    throw new Error("Match genesis requires one ziffle key per player");
  }
  const ziffleCeremonies = Array.isArray(match?.ziffleCeremonies)
    ? match.ziffleCeremonies
    : [];
  if (ziffleCeremonies.length !== playerCount) {
    throw new Error("Match genesis requires one ziffle ceremony per player");
  }
  const playerSeats = new Set(seats);
  const ceremonyOwners = new Set();
  for (const player of players) {
    const seat = Number(player?.index);
    if (!player?.auditPublicKey || !player?.auditEncryptionPublicKey) {
      throw new Error(`Match genesis player ${seat + 1} is missing audit keys`);
    }
    const manifest = publicDeckManifest(deckAuditManifests[seat]);
    if (!manifest || Number(manifest.owner) !== seat) {
      throw new Error(`Match genesis player ${seat + 1} is missing its deck audit manifest`);
    }
    if (canonicalJson(publicDeckManifest(player.deckAuditManifest)) !== canonicalJson(manifest)) {
      throw new Error(`Match genesis player ${seat + 1} deck manifest does not match the match manifest`);
    }
    if (String(manifest.matchId || "") !== String(match.auditMatchId || match.matchId || "")) {
      throw new Error(`Match genesis player ${seat + 1} deck manifest is bound to a different match`);
    }
    const ziffleKey = ziffleKeys[seat];
    if (
      !ziffleKey
      || Number(ziffleKey.player) !== seat
      || !String(ziffleKey.publicKeyHex || "")
      || !String(ziffleKey.ownershipProofHex || "")
    ) {
      throw new Error(`Match genesis player ${seat + 1} is missing its ziffle key`);
    }
    if (canonicalJson(player.ziffleKey || null) !== canonicalJson(ziffleKey)) {
      throw new Error(`Match genesis player ${seat + 1} ziffle key does not match the match key`);
    }
  }
  for (const ceremony of ziffleCeremonies) {
    const owner = Number(ceremony?.owner);
    if (!playerSeats.has(owner) || ceremonyOwners.has(owner)) {
      throw new Error("Match genesis ziffle ceremonies must map one-to-one to players");
    }
    ceremonyOwners.add(owner);
    const manifest = publicDeckManifest(deckAuditManifests[owner]);
    if (Number(ceremony.deckCount || 0) !== Number(manifest?.deckCount || 0)) {
      throw new Error(`Match genesis ziffle ceremony for player ${owner + 1} has the wrong deck count`);
    }
    if (String(ceremony.context || "") !== String(match.auditMatchId || match.matchId || "")) {
      throw new Error(`Match genesis ziffle ceremony for player ${owner + 1} is bound to a different match`);
    }
    if (String(ceremony.keyContext || ceremony.context || "") !== String(ceremony.context || "")) {
      throw new Error(`Match genesis ziffle ceremony for player ${owner + 1} has a mismatched key context`);
    }
    if (!String(ceremony.deckHash || "")) {
      throw new Error(`Match genesis ziffle ceremony for player ${owner + 1} is missing its deck hash`);
    }
    if (!Array.isArray(ceremony.keys) || ceremony.keys.length !== playerCount) {
      throw new Error(`Match genesis ziffle ceremony for player ${owner + 1} is missing player keys`);
    }
    if (canonicalJson(ceremony.keys) !== canonicalJson(ziffleKeys)) {
      throw new Error(`Match genesis ziffle ceremony for player ${owner + 1} uses different player keys`);
    }
    if (!Array.isArray(ceremony.steps) || ceremony.steps.length !== playerCount) {
      throw new Error(`Match genesis ziffle ceremony for player ${owner + 1} is missing shuffle steps`);
    }
    for (const step of ceremony.steps) {
      if (
        !playerSeats.has(Number(step?.shuffler))
        || !String(step?.deckHex || "")
        || !String(step?.proofHex || "")
      ) {
        throw new Error(`Match genesis ziffle ceremony for player ${owner + 1} has an invalid shuffle step`);
      }
    }
  }
  const payload = matchGenesisPayload(match);
  const payloadHash = await sha256Hex(canonicalJson(payload), cryptoImpl);
  if (payloadHash !== String(genesis.payloadHash || "")) {
    throw new Error("Match genesis payload hash mismatch");
  }
  const host = players.find((player) => Number(player?.index) === Number(genesis.hostSeat));
  if (!host?.auditPublicKey) {
    throw new Error("Match genesis host signer is missing");
  }
  const hostKey = await importAuditPublicKey(host.auditPublicKey, cryptoImpl);
  const hostValid = await verifyAuditPayload(
    hostKey,
    payload,
    genesis.hostSignature || "",
    cryptoImpl,
  );
  if (!hostValid) {
    throw new Error("Match genesis host signature is invalid");
  }
  await Promise.all(players.map((player) => verifySignedPlayerGenesis({
    player,
    matchId: payload.matchId,
    protocolVersion: payload.protocolVersion,
    timeoutMs: payload.timeoutMs,
  }, cryptoImpl)));
  return {
    valid: true,
    payloadHash,
    playerCount: players.length,
  };
}

export async function checkpointHash(checkpoint, cryptoImpl = globalThis.crypto) {
  return sha256Hex(canonicalJson({
    domain: "ironsmith-resync-checkpoint-v1",
    checkpoint: stripTransientMetadata(checkpoint),
  }), cryptoImpl);
}

export async function publicCheckpointHash(checkpoint, cryptoImpl = globalThis.crypto) {
  return sha256Hex(canonicalJson({
    domain: "ironsmith-public-audit-checkpoint-v1",
    checkpoint: stripTransientMetadata(checkpoint),
  }), cryptoImpl);
}

export async function transcriptActionsHash(actions = [], cryptoImpl = globalThis.crypto) {
  return sha256Hex(canonicalJson({
    domain: "ironsmith-resync-actions-v1",
    actions,
  }), cryptoImpl);
}

export async function buildSignedResyncEnvelope({
  keyPair,
  matchId,
  signer,
  lastSequence,
  finalStateHash,
  checkpoint,
  actions = [],
}, cryptoImpl = globalThis.crypto) {
  const payload = {
    domain: "ironsmith-resync-envelope-v1",
    matchId: String(matchId || ""),
    signer: Number(signer),
    lastSequence: Number(lastSequence || 0),
    finalStateHash: String(finalStateHash || ""),
    checkpointHash: await checkpointHash(checkpoint, cryptoImpl),
    actionsHash: await transcriptActionsHash(actions, cryptoImpl),
  };
  return {
    ...payload,
    signatureAlgorithm: "ecdsa-p256-sha256",
    signature: await signAuditPayload(keyPair, payload, cryptoImpl),
  };
}

export async function verifySignedResyncEnvelope({
  envelope,
  publicKey,
  checkpoint,
  actions = [],
}, cryptoImpl = globalThis.crypto) {
  if (!envelope || typeof envelope !== "object") {
    throw new Error("Resync payload is missing signed envelope");
  }
  const payload = {
    domain: "ironsmith-resync-envelope-v1",
    matchId: String(envelope.matchId || ""),
    signer: Number(envelope.signer),
    lastSequence: Number(envelope.lastSequence || 0),
    finalStateHash: String(envelope.finalStateHash || ""),
    checkpointHash: String(envelope.checkpointHash || ""),
    actionsHash: String(envelope.actionsHash || ""),
  };
  const expectedCheckpointHash = await checkpointHash(checkpoint, cryptoImpl);
  if (payload.checkpointHash !== expectedCheckpointHash) {
    throw new Error("Resync checkpoint hash mismatch");
  }
  const expectedActionsHash = await transcriptActionsHash(actions, cryptoImpl);
  if (payload.actionsHash !== expectedActionsHash) {
    throw new Error("Resync action log hash mismatch");
  }
  const valid = await verifyAuditPayload(
    publicKey,
    payload,
    envelope.signature || "",
    cryptoImpl,
  );
  if (!valid) {
    throw new Error("Resync envelope signature is invalid");
  }
  return {
    valid: true,
    checkpointHash: payload.checkpointHash,
    actionsHash: payload.actionsHash,
  };
}

export function sanitizeAuditCardList(cards) {
  if (!Array.isArray(cards)) return [];
  return cards
    .map((card) => String(card || "").trim())
    .filter(Boolean);
}

export async function decklistHashForCards({
  matchId,
  owner,
  deck = [],
  sideboard = [],
  commanders = [],
}, cryptoImpl = globalThis.crypto) {
  return sha256Hex(canonicalJson({
    domain: "ironsmith-ui-audit-decklist-v1",
    matchId,
    owner,
    deck: sanitizeAuditCardList(deck),
    sideboard: sanitizeAuditCardList(sideboard),
    commanders: sanitizeAuditCardList(commanders),
  }), cryptoImpl);
}

export async function buildPrivateDeckManifest({
  matchId,
  owner,
  deck = [],
  sideboard = [],
  commanders = [],
  saltForSlot = null,
}, cryptoImpl = globalThis.crypto) {
  const normalizedDeck = sanitizeAuditCardList(deck);
  const normalizedSideboard = sanitizeAuditCardList(sideboard);
  const normalizedCommanders = sanitizeAuditCardList(commanders);
  const decklistHash = await decklistHashForCards({
    matchId,
    owner,
    deck: normalizedDeck,
    sideboard: normalizedSideboard,
    commanders: normalizedCommanders,
  }, cryptoImpl);
  const slots = [];
  const slotSecrets = [];
  for (let slot = 0; slot < normalizedDeck.length; slot += 1) {
    const card = normalizedDeck[slot];
    const salt = saltForSlot
      ? String(await saltForSlot(slot, card))
      : randomHex(cryptoImpl, 32);
    const commitment = await cardSlotCommitment({
      matchId,
      owner,
      slot,
      card,
      salt,
    }, cryptoImpl);
    slots.push({
      slot,
      commitment,
    });
    slotSecrets.push({
      slot,
      card,
      salt,
      commitment,
    });
  }
  const commitmentRoot = await sha256Hex(canonicalJson({
    domain: "ironsmith-ui-audit-deck-commitment-root-v1",
    matchId,
    owner,
    decklistHash,
    slots,
  }), cryptoImpl);
  return {
    matchId,
    owner,
    deckCount: normalizedDeck.length,
    sideboardCount: normalizedSideboard.length,
    commanderCount: normalizedCommanders.length,
    decklistHash,
    commitmentRoot,
    slotCommitments: slots,
    slotSecrets,
  };
}

export async function cardSlotCommitment({
  matchId,
  owner,
  slot,
  card,
  salt,
}, cryptoImpl = globalThis.crypto) {
  return sha256Hex(canonicalJson({
    domain: "ironsmith-ui-audit-card-commitment-v1",
    matchId,
    owner,
    slot,
    card: String(card || "").trim(),
    salt,
  }), cryptoImpl);
}

export async function verifyCardOpeningAgainstManifest({
  manifest,
  slot,
  card,
  salt,
}, cryptoImpl = globalThis.crypto) {
  const target = (manifest?.slotCommitments || []).find(
    (entry) => Number(entry.slot) === Number(slot)
  );
  if (!target) return false;
  const commitment = await cardSlotCommitment({
    matchId: manifest.matchId,
    owner: manifest.owner,
    slot: Number(slot),
    card,
    salt,
  }, cryptoImpl);
  return commitment === target.commitment;
}

export async function buildDeckSlotOpening({
  manifest,
  slot,
  card,
}, cryptoImpl = globalThis.crypto) {
  const normalizedSlot = Number(slot);
  const secret = (manifest?.slotSecrets || []).find(
    (entry) => Number(entry.slot) === normalizedSlot
  );
  if (!secret) {
    throw new Error(`Missing private deck opening for slot ${normalizedSlot}`);
  }
  const normalizedCard = String(card || secret.card || "").trim();
  const commitment = await cardSlotCommitment({
    matchId: manifest.matchId,
    owner: manifest.owner,
    slot: normalizedSlot,
    card: normalizedCard,
    salt: secret.salt,
  }, cryptoImpl);
  if (secret.commitment && commitment !== secret.commitment) {
    throw new Error(`Private deck opening does not match slot ${normalizedSlot}`);
  }
  return {
    owner: Number(manifest.owner),
    slot: normalizedSlot,
    card: normalizedCard,
    salt: String(secret.salt || ""),
    commitment,
  };
}

export function publicDeckManifest(manifest) {
  if (!manifest || typeof manifest !== "object") return null;
  return {
    matchId: manifest.matchId,
    owner: manifest.owner,
    deckCount: Number(manifest.deckCount || 0),
    sideboardCount: Number(manifest.sideboardCount || 0),
    commanderCount: Number(manifest.commanderCount || 0),
    decklistHash: String(manifest.decklistHash || ""),
    commitmentRoot: String(manifest.commitmentRoot || ""),
    slotCommitments: Array.isArray(manifest.slotCommitments)
      ? manifest.slotCommitments.map((slot) => ({
          slot: Number(slot.slot),
          commitment: String(slot.commitment || ""),
        }))
      : [],
  };
}

function transcriptDeckManifestMap(transcript) {
  const manifests = transcript.match?.deckAuditManifests || [];
  return new Map(
    manifests
      .filter(Boolean)
      .map((manifest) => [Number(manifest.owner), manifest]),
  );
}

function sortedPlayerSeats(players) {
  return [...players.keys()].sort((left, right) => Number(left) - Number(right));
}

function requireExactSortedPlayerEntries(entries, expectedPlayers, label) {
  if (!Array.isArray(entries)) {
    throw new Error(`${label} must be an array`);
  }
  if (entries.length !== expectedPlayers.length) {
    throw new Error(`${label} must include every player exactly once`);
  }
  const seen = new Set();
  for (let index = 0; index < expectedPlayers.length; index += 1) {
    const expectedPlayer = Number(expectedPlayers[index]);
    const actualPlayer = Number(entries[index]?.player);
    if (!Number.isInteger(actualPlayer)) {
      throw new Error(`${label} contains an invalid player index`);
    }
    if (seen.has(actualPlayer)) {
      throw new Error(`${label} contains a duplicate player`);
    }
    seen.add(actualPlayer);
    if (actualPlayer !== expectedPlayer) {
      throw new Error(`${label} must be sorted by player and include every player`);
    }
  }
}

async function rngCommitmentForNonce(nonceHex, cryptoImpl) {
  return sha256Hex(canonicalJson({
    domain: "ironsmith-rng-commit-v1",
    nonceHex: String(nonceHex || ""),
  }), cryptoImpl);
}

async function verifyFairRandomReveal({
  reveal,
  expectedPlayers,
  players,
  matchId,
  seq,
}, cryptoImpl) {
  requireExactSortedPlayerEntries(reveal?.commits, expectedPlayers, "Fair-random commits");
  requireExactSortedPlayerEntries(reveal?.reveals, expectedPlayers, "Fair-random reveals");
  for (const entry of reveal.commits || []) {
    if (!entry?.signature) {
      throw new Error("Fair-random commitment is missing its player signature");
    }
    const player = Number(entry.player);
    const publicKey = await importAuditPublicKey(players.get(player)?.auditPublicKey || "", cryptoImpl);
    const valid = await verifyAuditPayload(
      publicKey,
      rngCommitmentPayload({
        matchId,
        seq,
        requirementId: reveal?.requirementId,
        requestId: entry.requestId,
        requester: entry.requester,
        player,
        commitmentHex: entry.commitmentHex,
      }),
      entry.signature,
      cryptoImpl,
    );
    if (!valid) {
      throw new Error("Fair-random commitment signature is invalid");
    }
  }
  for (const entry of reveal.reveals || []) {
    const expected = (reveal.commits || []).find(
      (commit) => Number(commit.player) === Number(entry.player)
    );
    const actual = await rngCommitmentForNonce(entry.nonceHex, cryptoImpl);
    if (!expected || actual !== String(expected.commitmentHex || "")) {
      throw new Error("Fair-random reveal does not match commitment");
    }
    if (entry.commitmentHex && actual !== String(entry.commitmentHex || "")) {
      throw new Error("Fair-random reveal commitment echo is invalid");
    }
    if (!entry?.signature) {
      throw new Error("Fair-random reveal is missing its player signature");
    }
    const player = Number(entry.player);
    const publicKey = await importAuditPublicKey(players.get(player)?.auditPublicKey || "", cryptoImpl);
    const valid = await verifyAuditPayload(
      publicKey,
      rngRevealPayload({
        matchId,
        seq,
        requirementId: reveal?.requirementId,
        requestId: entry.requestId,
        commitRequestId: entry.commitRequestId,
        requester: entry.requester,
        player,
        nonceHex: entry.nonceHex,
        commitmentHex: entry.commitmentHex,
      }),
      entry.signature,
      cryptoImpl,
    );
    if (!valid) {
      throw new Error("Fair-random reveal signature is invalid");
    }
  }
  const combinedSeedHex = await sha256Hex(canonicalJson({
    domain: "ironsmith-combined-rng-v1",
    matchId: String(matchId || ""),
    seq: Number(seq),
    requirementId: String(reveal?.requirementId || ""),
    commits: reveal.commits || [],
    reveals: reveal.reveals || [],
  }), cryptoImpl);
  if (combinedSeedHex !== String(reveal?.combinedSeedHex || "")) {
    throw new Error("Fair-random combined seed is invalid");
  }
}

async function verifyAuditOpenings({
  openings = [],
  manifests,
}, cryptoImpl) {
  for (const opening of openings || []) {
    const manifest = manifests.get(Number(opening?.owner));
    if (!manifest) {
      throw new Error(`Opening references unknown deck manifest for player ${opening?.owner}`);
    }
    const valid = await verifyCardOpeningAgainstManifest({
      manifest,
      slot: opening.slot,
      card: opening.card,
      salt: opening.salt,
    }, cryptoImpl);
    if (!valid) {
      throw new Error(`Opening does not match committed deck slot for player ${Number(opening?.owner) + 1}`);
    }
  }
}

function verifyPrivateViewProofStructure(proofs = [], players) {
  for (const proof of proofs || []) {
    if (String(proof?.type || "") !== "encrypted_private_opening") {
      throw new Error("Unsupported private-view proof type");
    }
    if (!proof?.encryptedOpening?.ciphertextHex || !proof?.encryptedOpening?.plaintextHash) {
      throw new Error("Private-view proof is missing encrypted opening material");
    }
    const viewer = Number(proof.viewer);
    const viewerKey = players.get(viewer)?.auditEncryptionPublicKey || "";
    if (viewerKey && String(proof.encryptedOpening.recipientPublicKey || "") !== viewerKey) {
      throw new Error("Private-view proof targets the wrong viewer key");
    }
  }
}

async function verifyShuffleProofList({
  shuffleProofs = [],
  verifyShuffleProof,
  seq,
}) {
  for (const proof of shuffleProofs || []) {
    if (String(proof?.type || "") !== "ziffle_shuffle") {
      throw new Error(`Unsupported shuffle proof type at sequence ${seq}`);
    }
    if (typeof verifyShuffleProof !== "function") {
      throw new Error("Live audit transcript contains shuffle proofs but no verifier was provided");
    }
    await verifyShuffleProof(proof);
  }
}

export async function verifyLiveAuditTranscript(
  transcript,
  cryptoImpl = globalThis.crypto,
  options = {},
) {
  if (!transcript || typeof transcript !== "object") {
    throw new Error("Missing audit transcript");
  }
  if (transcript.kind !== "ironsmith-live-browser-audit-v1") {
    throw new Error("Unsupported live audit transcript kind");
  }
  if (!transcript.match || typeof transcript.match !== "object" || !transcript.genesis) {
    throw new Error("Live audit transcript is missing current protocol match genesis");
  }
  if (Number(transcript.protocolVersion || 0) !== CURRENT_AUDIT_PROTOCOL_VERSION) {
    throw new Error("Live audit transcript uses an unsupported protocol version");
  }
  if (Number(transcript.match.protocolVersion || 0) !== CURRENT_AUDIT_PROTOCOL_VERSION) {
    throw new Error("Live audit transcript match uses an unsupported protocol version");
  }
  await verifySignedMatchGenesis({
    ...transcript.match,
    auditMatchId: transcript.match.auditMatchId || transcript.matchId,
    lobbyId: transcript.match.lobbyId || transcript.lobbyId,
    deckAuditManifests: transcript.match.deckAuditManifests || [],
    genesis: transcript.genesis,
  }, cryptoImpl);
  const transcriptPlayers = (transcript.match.players || []).map((player) => ({
    seat: Number(player.index),
    auditPublicKey: String(player.auditPublicKey || ""),
    auditEncryptionPublicKey: String(player.auditEncryptionPublicKey || ""),
  }));
  const players = new Map(
    transcriptPlayers.map((player) => [
      Number(player.seat),
      {
        auditPublicKey: String(player.auditPublicKey || ""),
        auditEncryptionPublicKey: String(player.auditEncryptionPublicKey || ""),
      },
    ]),
  );
  const playerList = transcriptPlayers.map((player) => ({
    index: Number(player.seat),
    auditPublicKey: String(player.auditPublicKey || ""),
  }));
  let activeQuorumPlayers = [...playerList];
  const expectedPlayers = sortedPlayerSeats(players);
  const manifests = transcriptDeckManifestMap(transcript);
  let stateHash = String(transcript.initialStateHash || "0".repeat(64));
  let clockHash = INITIAL_MATCH_CLOCK_HASH;
  let expectedSeq = 1;
  for (const entry of transcript.actions || []) {
    const audit = entry?.audit;
    if (!audit || typeof audit !== "object") {
      throw new Error(`Action ${expectedSeq} is missing sequenced audit envelope`);
    }
    if (Number(audit.seq) !== expectedSeq) {
      throw new Error(`Expected audit sequence ${expectedSeq}, received ${audit.seq}`);
    }
    if (audit.prevStateHash !== stateHash) {
      throw new Error(`Audit state hash mismatch at sequence ${expectedSeq}`);
    }
    if (canonicalJson(audit.command) !== canonicalJson(entry.command)) {
      throw new Error(`Audit command mismatch at sequence ${expectedSeq}`);
    }
    if (!audit.publicCheckpointHash) {
      throw new Error(`Action ${expectedSeq} is missing public checkpoint hash`);
    }

    const signer = Number(audit.signer ?? audit.actor);
    if (signer !== Number(audit.actor)) {
      throw new Error(`Action ${expectedSeq} was not signed by the acting player`);
    }
    clockHash = await verifyMatchClockAuditChainEntry({
      clock: audit.clock,
      expectedClockHash: clockHash,
      expectedSeq,
      expectedActor: audit.actor,
    }, cryptoImpl);
    const signerKey = await importAuditPublicKey(players.get(signer)?.auditPublicKey || "", cryptoImpl);
    const envelopePayload = {
      matchId: audit.matchId,
      seq: Number(audit.seq),
      actor: Number(audit.actor),
      signer,
      prevStateHash: audit.prevStateHash,
      command: audit.command,
      clock: audit.clock,
      openings: audit.openings || [],
      rngReveals: audit.rngReveals || [],
      shuffleProofs: audit.shuffleProofs || [],
      privateViewProofs: audit.privateViewProofs || [],
      publicCheckpointHash: audit.publicCheckpointHash,
      nextStateHash: audit.nextStateHash,
    };
    const sequencedValid = await verifyAuditPayload(
      signerKey,
      envelopePayload,
      audit.signature || "",
      cryptoImpl,
    );
    if (!sequencedValid) {
      throw new Error(`Sequenced audit signature is invalid at sequence ${expectedSeq}`);
    }
    const forfeitTarget = audit.command?.type === "forfeit_player"
      ? Number(audit.command.player)
      : null;
    const actionQuorumPlayers = forfeitTarget == null
      ? activeQuorumPlayers
      : activeQuorumPlayers.filter((player) =>
          Number(player.index) !== forfeitTarget
        );
    const actionQuorumThresholdOverride = forfeitTarget == null
      ? null
      : (
        activeQuorumPlayers.length < 3 || Number(audit.actor) === forfeitTarget
          ? 0
          : actionQuorumPlayers.length
      );
    await verifyActionQuorumCertificate({
      certificate: audit.quorumCertificate || entry.quorumCertificate,
      action: entry,
      players: actionQuorumPlayers,
      threshold: actionQuorumThresholdOverride,
    }, cryptoImpl);
    if (forfeitTarget != null) {
      activeQuorumPlayers = activeQuorumPlayers.filter((player) =>
        Number(player.index) !== forfeitTarget
      );
    }
    await verifyAuditOpenings({
      openings: audit.openings || [],
      manifests,
    }, cryptoImpl);
    verifyPrivateViewProofStructure(audit.privateViewProofs || [], players);
    for (const reveal of audit.rngReveals || []) {
      await verifyFairRandomReveal({
        reveal,
        expectedPlayers,
        players,
        matchId: audit.matchId,
        seq: audit.seq,
      }, cryptoImpl);
    }
    await verifyShuffleProofList({
      shuffleProofs: audit.shuffleProofs || [],
      verifyShuffleProof: options.verifyShuffleProof,
      seq: expectedSeq,
    });
    const computedHash = await auditStateHash({
      matchId: audit.matchId,
      seq: Number(audit.seq),
      prevStateHash: audit.prevStateHash,
      command: audit.command,
      clock: audit.clock,
      openings: audit.openings || [],
      rngReveals: audit.rngReveals || [],
      shuffleProofs: audit.shuffleProofs || [],
      privateViewProofs: audit.privateViewProofs || [],
      publicCheckpointHash: audit.publicCheckpointHash,
    }, cryptoImpl);
    if (computedHash !== audit.nextStateHash) {
      throw new Error(`Audit next state hash mismatch at sequence ${expectedSeq}`);
    }
    stateHash = audit.nextStateHash;
    expectedSeq += 1;
  }
  return {
    valid: true,
    verifiedActions: expectedSeq - 1,
    initialPublicCheckpointHash: transcript.initialPublicCheckpointHash || "",
    finalStateHash: stateHash,
  };
}

export function bytesToHex(bytes) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function randomHex(cryptoImpl, byteLength) {
  if (!cryptoImpl?.getRandomValues) {
    throw new Error("WebCrypto getRandomValues API is unavailable");
  }
  const bytes = new Uint8Array(byteLength);
  cryptoImpl.getRandomValues(bytes);
  return bytesToHex(bytes);
}

export function randomAuditHex(byteLength = 32, cryptoImpl = globalThis.crypto) {
  return randomHex(cryptoImpl, byteLength);
}

export function hexToBytes(hex) {
  const normalized = String(hex || "").trim();
  if (normalized.length % 2 !== 0) {
    throw new Error("Hex string has odd length");
  }
  const out = new Uint8Array(normalized.length / 2);
  for (let i = 0; i < normalized.length; i += 2) {
    const byte = Number.parseInt(normalized.slice(i, i + 2), 16);
    if (!Number.isFinite(byte)) {
      throw new Error("Invalid hex string");
    }
    out[i / 2] = byte;
  }
  return out;
}
