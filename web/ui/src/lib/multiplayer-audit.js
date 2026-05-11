const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

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
  openings = [],
  rngReveals = [],
  shuffleProofs = [],
  privateViewProofs = [],
}, cryptoImpl = globalThis.crypto) {
  return sha256Hex(
    canonicalJson({
      domain: "ironsmith-ui-audit-state-v1",
      matchId,
      seq,
      prevStateHash,
      command,
      openings,
      rngReveals,
      shuffleProofs,
      privateViewProofs,
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
  openings = [],
  rngReveals = [],
  shuffleProofs = [],
  privateViewProofs = [],
}, cryptoImpl = globalThis.crypto) {
  const nextStateHash = await auditStateHash({
    matchId,
    seq,
    prevStateHash,
    command,
    openings,
    rngReveals,
    shuffleProofs,
    privateViewProofs,
  }, cryptoImpl);
  const payload = {
    matchId,
    seq,
    actor,
    signer,
    prevStateHash,
    command,
    openings,
    rngReveals,
    shuffleProofs,
    privateViewProofs,
    nextStateHash,
  };
  return {
    ...payload,
    signatureAlgorithm: "ecdsa-p256-sha256",
    signature: await signAuditPayload(keyPair, payload, cryptoImpl),
  };
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
  const payload = matchGenesisPayload(match);
  const payloadHash = await sha256Hex(canonicalJson(payload), cryptoImpl);
  if (payloadHash !== String(genesis.payloadHash || "")) {
    throw new Error("Match genesis payload hash mismatch");
  }
  const players = Array.isArray(match?.players) ? match.players : [];
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
    checkpoint,
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

export async function verifyLiveAuditTranscript(transcript, cryptoImpl = globalThis.crypto) {
  if (!transcript || typeof transcript !== "object") {
    throw new Error("Missing audit transcript");
  }
  if (transcript.kind !== "ironsmith-live-browser-audit-v1") {
    throw new Error("Unsupported live audit transcript kind");
  }
  if (transcript.genesis) {
    await verifySignedMatchGenesis({
      ...(transcript.match || {}),
      protocolVersion: transcript.protocolVersion,
      auditMatchId: transcript.matchId,
      lobbyId: transcript.lobbyId,
      players: transcript.match?.players || transcript.players?.map((player) => ({
        peerId: player.peerId,
        name: player.name,
        index: player.seat,
        auditPublicKey: player.auditPublicKey,
        auditEncryptionPublicKey: player.auditEncryptionPublicKey,
        playerGenesisSignature: player.playerGenesisSignature,
        deckAuditManifest: (transcript.deckAuditManifests || [])[Number(player.seat)],
        ziffleKey: player.ziffleKey || null,
      })) || [],
      deckAuditManifests: transcript.match?.deckAuditManifests || transcript.deckAuditManifests || [],
      genesis: transcript.genesis,
    }, cryptoImpl);
  }
  const players = new Map(
    (transcript.players || []).map((player) => [
      Number(player.seat),
      String(player.auditPublicKey || ""),
    ]),
  );
  let stateHash = String(transcript.initialStateHash || "0".repeat(64));
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

    const signer = Number(audit.signer ?? audit.actor);
    if (signer !== Number(audit.actor)) {
      throw new Error(`Action ${expectedSeq} was not signed by the acting player`);
    }
    const signerKey = await importAuditPublicKey(players.get(signer), cryptoImpl);
    const envelopePayload = {
      matchId: audit.matchId,
      seq: Number(audit.seq),
      actor: Number(audit.actor),
      signer,
      prevStateHash: audit.prevStateHash,
      command: audit.command,
      openings: audit.openings || [],
      rngReveals: audit.rngReveals || [],
      shuffleProofs: audit.shuffleProofs || [],
      privateViewProofs: audit.privateViewProofs || [],
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
    const computedHash = await auditStateHash({
      matchId: audit.matchId,
      seq: Number(audit.seq),
      prevStateHash: audit.prevStateHash,
      command: audit.command,
      openings: audit.openings || [],
      rngReveals: audit.rngReveals || [],
      shuffleProofs: audit.shuffleProofs || [],
      privateViewProofs: audit.privateViewProofs || [],
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
