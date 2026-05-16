const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();
const INITIAL_MATCH_CLOCK_HASH = "0".repeat(64);
const MATCH_CLOCK_AUDIT_DOMAIN = "ironsmith-match-clock-audit-v1";
const ACTION_QUORUM_CERTIFICATE_TYPE = "ironsmith-action-quorum-v1";
const ACTION_QUORUM_VOTE_DOMAIN = "ironsmith-action-quorum-vote-v1";
const DISCONNECT_FORFEIT_CERTIFICATE_TYPE = "ironsmith-disconnect-forfeit-v1";
const DISCONNECT_FORFEIT_VOTE_DOMAIN = "ironsmith-disconnect-forfeit-vote-v1";
export const LEGACY_DISCONNECT_FORFEIT_REASON = "peer_claimed_disconnect_timeout";
export const DISCONNECT_FORFEIT_REASON = "disconnect_timeout_policy";
export const DISCONNECT_AUTO_FORFEIT_MS = 60 * 1000;
export const CURRENT_AUDIT_PROTOCOL_VERSION = 13;
export const CURRENT_AUDIT_MIN_PLAYERS = 2;
export const CURRENT_AUDIT_MAX_PLAYERS = 4;
export const ZIFFLE_OPENING_PROOF_TYPE = "ziffle_position_opening_v1";
const P256_ORDER = BigInt("0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551");
const P256_HALF_ORDER = P256_ORDER >> 1n;

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

export function isDisconnectForfeitReason(reason) {
  const normalized = String(reason || "");
  return normalized === DISCONNECT_FORFEIT_REASON
    || normalized === LEGACY_DISCONNECT_FORFEIT_REASON;
}

export function cryptoMaterialRequirementType(requirement) {
  return String(requirement?.type || requirement?.requirement_type || "");
}

export function isCryptoMaterialRequirement(requirement) {
  return [
    "public_open",
    "private_open",
    "private_view_window",
  ].includes(cryptoMaterialRequirementType(requirement));
}

export function isOwnerPrivateCryptoMaterialRequirement(requirement) {
  const type = cryptoMaterialRequirementType(requirement);
  return (
    (type === "private_open" || type === "private_view_window")
    && requirement?.owner != null
    && requirement?.viewer != null
    && Number(requirement.owner) === Number(requirement.viewer)
  );
}

function cryptoMaterialRequirementId(requirement) {
  return String(
    requirement?.id
      || requirement?.requirementId
      || requirement?.requirement_id
      || ""
  );
}

function cryptoMaterialRequirementReplayKey(requirement) {
  if (!requirement || typeof requirement !== "object") return "";
  return canonicalJson({
    id: requirement.id ?? requirement.requirementId ?? requirement.requirement_id ?? null,
    type: requirement.type ?? requirement.requirement_type ?? null,
    owner: requirement.owner ?? null,
    viewer: requirement.viewer ?? null,
    zone: requirement.zone ?? null,
    slot: requirement.slot ?? null,
    objectId: requirement.objectId ?? requirement.object_id ?? null,
    commitment: requirement.commitment ?? null,
    count: requirement.count ?? null,
    reason: requirement.reason ?? null,
  });
}

function numericRequirementFieldMatches(authorized, requested, field) {
  const requestedValue = requested?.[field] ?? (
    field === "objectId" ? requested?.object_id : undefined
  );
  const authorizedValue = authorized?.[field] ?? (
    field === "objectId" ? authorized?.object_id : undefined
  );
  if (requestedValue === null || requestedValue === undefined) return true;
  if (authorizedValue === null || authorizedValue === undefined) return false;
  return Number(authorizedValue) === Number(requestedValue);
}

function stringRequirementFieldMatches(authorized, requested, field, fallback = "") {
  const requestedValue = requested?.[field];
  if (requestedValue === null || requestedValue === undefined || String(requestedValue || "") === "") {
    return true;
  }
  const authorizedValue = authorized?.[field];
  return String(authorizedValue ?? fallback) === String(requestedValue);
}

export function cryptoMaterialRequirementMatchesAuthorization(authorized, requested) {
  if (!authorized || !requested) return false;
  if (!isCryptoMaterialRequirement(authorized) || !isCryptoMaterialRequirement(requested)) {
    return false;
  }
  if (cryptoMaterialRequirementReplayKey(authorized) === cryptoMaterialRequirementReplayKey(requested)) {
    return true;
  }
  const authorizedId = cryptoMaterialRequirementId(authorized);
  const requestedId = cryptoMaterialRequirementId(requested);
  if (!authorizedId || !requestedId || authorizedId !== requestedId) {
    return false;
  }
  if (cryptoMaterialRequirementType(authorized) !== cryptoMaterialRequirementType(requested)) {
    return false;
  }
  if (Number(authorized.owner) !== Number(requested.owner)) return false;
  if (!numericRequirementFieldMatches(authorized, requested, "viewer")) return false;
  if (!numericRequirementFieldMatches(authorized, requested, "slot")) return false;
  if (!numericRequirementFieldMatches(authorized, requested, "objectId")) return false;
  if (!numericRequirementFieldMatches(authorized, requested, "count")) return false;
  if (!stringRequirementFieldMatches(authorized, requested, "zone")) return false;
  if (!stringRequirementFieldMatches(authorized, requested, "commitment")) return false;
  if (!stringRequirementFieldMatches(authorized, requested, "reason")) return false;
  return true;
}

export function mergeCryptoMaterialRequirements(...requirementLists) {
  const merged = new Map();
  for (const requirement of requirementLists.flat()) {
    if (!isCryptoMaterialRequirement(requirement)) continue;
    const key = String(cryptoMaterialRequirementId(requirement) || cryptoMaterialRequirementReplayKey(requirement));
    if (!merged.has(key)) {
      merged.set(key, normalizeForJson(requirement));
    }
  }
  return [...merged.values()];
}

export function localAnswerableCryptoMaterialRequirements(requirements = [], localSeat = null) {
  const seat = Number(localSeat);
  if (!Number.isInteger(seat)) return [];
  return (requirements || []).filter((requirement) =>
    isCryptoMaterialRequirement(requirement)
    && !isOwnerPrivateCryptoMaterialRequirement(requirement)
    && Number(requirement?.owner) === seat
  );
}

export function authorizeCryptoMaterialRequestRequirements({
  localSeat,
  requestedRequirements = [],
  previewedRequirements = [],
} = {}) {
  const requestedLocalRequirements = localAnswerableCryptoMaterialRequirements(
    Array.isArray(requestedRequirements)
      ? requestedRequirements.filter(isCryptoMaterialRequirement)
      : [],
    localSeat,
  );
  const answerablePreviewed = localAnswerableCryptoMaterialRequirements(
    mergeCryptoMaterialRequirements(previewedRequirements),
    localSeat,
  );
  const authorizedRequestedRequirements = [];
  for (const requested of requestedLocalRequirements) {
    const authorized = answerablePreviewed.find((entry) =>
      cryptoMaterialRequirementMatchesAuthorization(entry, requested)
    );
    if (authorized) {
      authorizedRequestedRequirements.push(authorized);
    }
  }
  const unauthorizedRequirements = requestedLocalRequirements.filter((requested) =>
    !answerablePreviewed.some((authorized) =>
      cryptoMaterialRequirementMatchesAuthorization(authorized, requested)
    )
  );
  if (unauthorizedRequirements.length > 0) {
    throw new Error("Cryptographic material request asks for unauthorized hidden-card material");
  }

  if (requestedLocalRequirements.length === 0) {
    return answerablePreviewed;
  }
  return mergeCryptoMaterialRequirements(authorizedRequestedRequirements);
}

export function fairRandomCombinedSeedPayload({
  matchId,
  seq,
  requirementId,
  commits = [],
  reveals = [],
}) {
  return {
    domain: "ironsmith-combined-rng-v2",
    matchId: String(matchId || ""),
    seq: Number(seq),
    requirementId: String(requirementId || ""),
    commits: (Array.isArray(commits) ? commits : [])
      .map((entry) => ({
        player: Number(entry?.player),
        commitmentHex: String(entry?.commitmentHex || ""),
      }))
      .sort((left, right) => Number(left.player) - Number(right.player)),
    reveals: (Array.isArray(reveals) ? reveals : [])
      .map((entry) => ({
        player: Number(entry?.player),
        nonceHex: String(entry?.nonceHex || ""),
        commitmentHex: String(entry?.commitmentHex || ""),
      }))
      .sort((left, right) => Number(left.player) - Number(right.player)),
  };
}

export async function fairRandomCombinedSeedHex(args, cryptoImpl = globalThis.crypto) {
  return sha256Hex(canonicalJson(fairRandomCombinedSeedPayload(args)), cryptoImpl);
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

function publicStableObjectId(object) {
  const stable = Number(object?.stableId ?? object?.stable_id);
  if (Number.isSafeInteger(stable) && stable >= 0) return stable;
  const id = Number(object?.id);
  return Number.isSafeInteger(id) && id >= 0 ? id : null;
}

function normalizePublicAttachmentTarget(target, stableIdByRuntimeId) {
  if (!target || typeof target !== "object") return target;
  const mapped = { ...target };
  if (mapped.object != null) {
    mapped.object = stableIdByRuntimeId.get(String(mapped.object)) ?? mapped.object;
  }
  if (mapped.objectId != null) {
    mapped.objectId = stableIdByRuntimeId.get(String(mapped.objectId)) ?? mapped.objectId;
  }
  if (mapped.object_id != null) {
    mapped.object_id = stableIdByRuntimeId.get(String(mapped.object_id)) ?? mapped.object_id;
  }
  return mapped;
}

function normalizePublicObjectIdList(ids, stableIdByRuntimeId, { sort = false } = {}) {
  const normalized = (Array.isArray(ids) ? ids : []).map((id) =>
    stableIdByRuntimeId.get(String(id)) ?? id
  );
  return sort
    ? normalized.sort((left, right) => Number(left) - Number(right))
    : normalized;
}

function normalizePublicCheckpointForHash(checkpoint) {
  const stripped = stripTransientMetadata(checkpoint);
  if (!stripped || typeof stripped !== "object" || !Array.isArray(stripped.objects)) {
    return stripped;
  }

  const stableIdByRuntimeId = new Map();
  for (const object of stripped.objects) {
    const stableId = publicStableObjectId(object);
    if (stableId != null && object?.id != null) {
      stableIdByRuntimeId.set(String(object.id), stableId);
    }
  }

  const normalizeObject = (object) => {
    const stableId = publicStableObjectId(object);
    const normalized = {
      ...object,
      ...(stableId != null ? { id: stableId, stableId } : {}),
    };
    delete normalized.stable_id;
    if (Array.isArray(normalized.attachments)) {
      normalized.attachments = normalizePublicObjectIdList(
        normalized.attachments,
        stableIdByRuntimeId,
        { sort: true }
      );
    }
    if (normalized.attachedTo) {
      normalized.attachedTo = normalizePublicAttachmentTarget(
        normalized.attachedTo,
        stableIdByRuntimeId
      );
    }
    if (normalized.attached_to) {
      normalized.attached_to = normalizePublicAttachmentTarget(
        normalized.attached_to,
        stableIdByRuntimeId
      );
    }
    return normalized;
  };

  const normalizePlayer = (player) => {
    const normalized = { ...player };
    for (const key of ["graveyard", "commanders"]) {
      if (Array.isArray(normalized[key])) {
        normalized[key] = normalizePublicObjectIdList(normalized[key], stableIdByRuntimeId);
      }
    }
    return normalized;
  };

  const normalizeStackEntry = (entry) => {
    const objectId = entry?.objectId ?? entry?.object_id;
    const normalized = {
      ...entry,
      objectId:
        objectId == null
          ? objectId
          : stableIdByRuntimeId.get(String(objectId)) ?? objectId,
      targets: (entry?.targets || []).map((target) =>
        normalizePublicAttachmentTarget(target, stableIdByRuntimeId)
      ),
    };
    delete normalized.object_id;
    return normalized;
  };

  const normalized = {
    ...stripped,
    players: (stripped.players || []).map(normalizePlayer),
    objects: (stripped.objects || [])
      .map(normalizeObject)
      .sort((left, right) => Number(left.id ?? 0) - Number(right.id ?? 0)),
    stack: (stripped.stack || []).map(normalizeStackEntry),
  };
  for (const key of ["battlefield", "publicExile", "public_exile", "command"]) {
    if (Array.isArray(normalized[key])) {
      normalized[key] = normalizePublicObjectIdList(normalized[key], stableIdByRuntimeId, {
        sort: key !== "stack",
      });
    }
  }
  if (Array.isArray(normalized.public_exile) && !Array.isArray(normalized.publicExile)) {
    normalized.publicExile = normalized.public_exile;
  }
  delete normalized.public_exile;
  return normalized;
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

function fixedWidthBigIntHex(value, byteLength) {
  const hex = value.toString(16);
  if (hex.length > byteLength * 2) {
    throw new Error("Integer does not fit in fixed-width field");
  }
  return hex.padStart(byteLength * 2, "0");
}

function signatureScalar(bytes, offset) {
  return BigInt(`0x${bytesToHex(bytes.slice(offset, offset + 32)) || "0"}`);
}

function canonicalP256SignatureBytes(signatureBytes) {
  if (!(signatureBytes instanceof Uint8Array) || signatureBytes.length !== 64) {
    throw new Error("P-256 signatures must be 64 raw bytes");
  }
  const r = signatureScalar(signatureBytes, 0);
  const s = signatureScalar(signatureBytes, 32);
  if (r <= 0n || r >= P256_ORDER || s <= 0n || s >= P256_ORDER) {
    throw new Error("P-256 signature scalar is out of range");
  }
  if (s <= P256_HALF_ORDER) {
    return signatureBytes;
  }
  const canonical = new Uint8Array(signatureBytes);
  canonical.set(hexToBytes(fixedWidthBigIntHex(P256_ORDER - s, 32)), 32);
  return canonical;
}

function isCanonicalP256SignatureBytes(signatureBytes) {
  if (!(signatureBytes instanceof Uint8Array) || signatureBytes.length !== 64) return false;
  const r = signatureScalar(signatureBytes, 0);
  const s = signatureScalar(signatureBytes, 32);
  return r > 0n && r < P256_ORDER && s > 0n && s <= P256_HALF_ORDER;
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
  return bytesToHex(canonicalP256SignatureBytes(new Uint8Array(signature)));
}

export async function verifyAuditPayload(
  publicKey,
  payload,
  signatureHex,
  cryptoImpl = globalThis.crypto,
) {
  const canonical = canonicalJson(payload);
  let signatureBytes = null;
  try {
    signatureBytes = hexToBytes(signatureHex);
  } catch {
    return false;
  }
  if (!isCanonicalP256SignatureBytes(signatureBytes)) {
    return false;
  }
  return cryptoImpl.subtle.verify(
    {
      name: "ECDSA",
      hash: "SHA-256",
    },
    publicKey,
    signatureBytes,
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
  if (count === 2) return 0;
  if (count === 3) return 2;
  return 3;
}

export function isDisconnectForfeitCommand(command) {
  return command?.type === "forfeit_player"
    && isDisconnectForfeitReason(command?.reason);
}

export function disconnectForfeitVoteThreshold(nonTargetPlayerCount) {
  const count = Math.max(0, Number(nonTargetPlayerCount || 0));
  return count;
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

export function disconnectForfeitVotePayload({
  matchId,
  basisSequence,
  forfeitedPlayer,
  forfeitedPeerId,
  disconnectTimeoutMs,
  disconnectedAtMs,
  eligibleAtMs,
  signedAtMs,
  voter,
}) {
  return {
    domain: DISCONNECT_FORFEIT_VOTE_DOMAIN,
    matchId: String(matchId || ""),
    basisSequence: Number(basisSequence || 0),
    forfeitedPlayer: Number(forfeitedPlayer),
    forfeitedPeerId: String(forfeitedPeerId || ""),
    disconnectTimeoutMs: Math.max(0, Math.floor(Number(disconnectTimeoutMs || 0))),
    disconnectedAtMs: Math.max(0, Math.floor(Number(disconnectedAtMs || 0))),
    eligibleAtMs: Math.max(0, Math.floor(Number(eligibleAtMs || 0))),
    signedAtMs: Math.max(0, Math.floor(Number(signedAtMs || 0))),
    voter: Number(voter),
  };
}

export async function buildSignedDisconnectForfeitVote({
  keyPair,
  matchId,
  basisSequence,
  forfeitedPlayer,
  forfeitedPeerId,
  disconnectTimeoutMs,
  disconnectedAtMs,
  eligibleAtMs = Number(disconnectedAtMs || 0) + Number(disconnectTimeoutMs || 0),
  signedAtMs = Date.now(),
  voter,
}, cryptoImpl = globalThis.crypto) {
  const payload = disconnectForfeitVotePayload({
    matchId,
    basisSequence,
    forfeitedPlayer,
    forfeitedPeerId,
    disconnectTimeoutMs,
    disconnectedAtMs,
    eligibleAtMs,
    signedAtMs,
    voter,
  });
  return {
    ...payload,
    signatureAlgorithm: "ecdsa-p256-sha256",
    signature: await signAuditPayload(keyPair, payload, cryptoImpl),
  };
}

export async function verifyDisconnectForfeitVote({
  vote,
  expected,
  players = [],
}, cryptoImpl = globalThis.crypto) {
  const voter = Number(vote?.voter);
  if (!Number.isInteger(voter) || voter < 0) {
    throw new Error("Disconnect forfeit vote contains an invalid voter");
  }
  const disconnectedAtMs = Math.max(0, Math.floor(Number(vote?.disconnectedAtMs || 0)));
  const disconnectTimeoutMs = Math.max(0, Math.floor(Number(
    expected?.disconnectTimeoutMs ?? vote?.disconnectTimeoutMs ?? DISCONNECT_AUTO_FORFEIT_MS
  )));
  const eligibleAtMs = Math.max(0, Math.floor(Number(vote?.eligibleAtMs || 0)));
  const signedAtMs = Math.max(0, Math.floor(Number(vote?.signedAtMs || 0)));
  const nowMs = Math.max(0, Math.floor(Number(expected?.nowMs ?? Date.now())));
  const maxFutureSkewMs = Math.max(0, Math.floor(Number(expected?.maxFutureSkewMs || 0)));
  if (disconnectedAtMs <= 0) {
    throw new Error("Disconnect forfeit vote is missing a disconnect observation timestamp");
  }
  if (eligibleAtMs !== disconnectedAtMs + disconnectTimeoutMs) {
    throw new Error("Disconnect forfeit vote has an invalid eligibility timestamp");
  }
  if (signedAtMs < eligibleAtMs) {
    throw new Error("Disconnect forfeit vote was signed before the disconnect timeout elapsed");
  }
  if (signedAtMs > nowMs + maxFutureSkewMs) {
    throw new Error("Disconnect forfeit vote is signed in the future");
  }
  const payload = disconnectForfeitVotePayload({
    matchId: expected?.matchId,
    basisSequence: expected?.basisSequence,
    forfeitedPlayer: expected?.forfeitedPlayer,
    forfeitedPeerId: expected?.forfeitedPeerId,
    disconnectTimeoutMs,
    disconnectedAtMs,
    eligibleAtMs,
    signedAtMs,
    voter,
  });
  if (
    String(vote?.domain || "") !== DISCONNECT_FORFEIT_VOTE_DOMAIN
    || String(vote?.matchId || "") !== payload.matchId
    || Number(vote?.basisSequence) !== payload.basisSequence
    || Number(vote?.forfeitedPlayer) !== payload.forfeitedPlayer
    || String(vote?.forfeitedPeerId || "") !== payload.forfeitedPeerId
    || Number(vote?.disconnectTimeoutMs) !== payload.disconnectTimeoutMs
    || (
      expected?.disconnectedAtMs != null
      && Number(vote?.disconnectedAtMs) !== Math.max(0, Math.floor(Number(expected.disconnectedAtMs || 0)))
    )
  ) {
    throw new Error("Disconnect forfeit vote does not match the forfeit claim");
  }
  const player = (players || []).find((entry) =>
    Number(entry?.index ?? entry?.seat) === voter
  );
  if (!player?.auditPublicKey || voter === payload.forfeitedPlayer) {
    throw new Error(`Disconnect forfeit voter ${voter + 1} is not eligible`);
  }
  const publicKey = await importAuditPublicKey(player.auditPublicKey, cryptoImpl);
  const valid = await verifyAuditPayload(
    publicKey,
    payload,
    vote.signature || "",
    cryptoImpl,
  );
  if (!valid) {
    throw new Error("Disconnect forfeit vote signature is invalid");
  }
  return voter;
}

export async function verifyDisconnectForfeitCertificate({
  certificate,
  command,
  players = [],
  threshold: requiredThreshold = null,
}, cryptoImpl = globalThis.crypto) {
  if (!isDisconnectForfeitCommand(command)) return { valid: true, threshold: 0, voters: [] };
  const roster = Array.isArray(players) ? players : [];
  const threshold = requiredThreshold == null
    ? disconnectForfeitVoteThreshold(roster.length)
    : Math.max(0, Number(requiredThreshold || 0));
  if (threshold <= 0) {
    return { valid: true, threshold: 0, voters: [] };
  }
  if (!certificate || typeof certificate !== "object") {
    throw new Error("Disconnect forfeit is missing its quorum certificate");
  }
  if (String(certificate.type || "") !== DISCONNECT_FORFEIT_CERTIFICATE_TYPE) {
    throw new Error("Disconnect forfeit has an unsupported certificate");
  }
  const expected = {
    matchId: String(command.matchId || certificate.matchId || ""),
    basisSequence: Number(command.basis_sequence ?? certificate.basisSequence ?? 0),
    forfeitedPlayer: Number(command.player),
    forfeitedPeerId: String(command.disconnected_peer_id || certificate.forfeitedPeerId || ""),
    disconnectedAtMs: null,
    disconnectTimeoutMs: Math.max(
      0,
      Math.floor(Number(command.disconnect_timeout_ms ?? certificate.disconnectTimeoutMs ?? DISCONNECT_AUTO_FORFEIT_MS))
    ),
    nowMs: Math.max(0, Math.floor(Number(command.nowMs ?? Date.now()))),
    maxFutureSkewMs: Math.max(0, Math.floor(Number(command.maxFutureSkewMs || 0))),
  };
  if (
    String(certificate.matchId || "") !== expected.matchId
    || Number(certificate.basisSequence) !== expected.basisSequence
    || Number(certificate.forfeitedPlayer) !== expected.forfeitedPlayer
    || String(certificate.forfeitedPeerId || "") !== expected.forfeitedPeerId
    || Number(certificate.disconnectTimeoutMs) !== expected.disconnectTimeoutMs
  ) {
    throw new Error("Disconnect forfeit certificate does not match the command");
  }
  const votes = Array.isArray(certificate.votes) ? certificate.votes : [];
  if (votes.length < threshold) {
    throw new Error(
      `Disconnect forfeit certificate has ${votes.length} vote(s), expected at least ${threshold}`
    );
  }
  const seen = new Set();
  for (const vote of votes) {
    const voter = await verifyDisconnectForfeitVote({
      vote,
      expected,
      players: roster,
    }, cryptoImpl);
    if (seen.has(voter)) {
      throw new Error("Disconnect forfeit certificate contains a duplicate voter");
    }
    seen.add(voter);
  }
  if (seen.size < threshold) {
    throw new Error(
      `Disconnect forfeit certificate has ${seen.size} unique vote(s), expected at least ${threshold}`
    );
  }
  return {
    valid: true,
    threshold,
    voters: [...seen].sort((left, right) => left - right),
  };
}

function sequencedActionSignedPayload(action) {
  const audit = action?.audit || {};
  const signer = Number(audit.signer ?? audit.actor);
  return {
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
}

function sequencedActionSignedPayloadFingerprint(action) {
  return canonicalJson(sequencedActionSignedPayload(action));
}

async function verifySequencedActionEnvelope({
  entry,
  players,
  expectedSeq = null,
  expectedPrevStateHash = null,
}, cryptoImpl = globalThis.crypto) {
  const action = entry || {};
  if (!action.audit || typeof action.audit !== "object") {
    throw new Error("Dispute action is missing its audit envelope");
  }
  const audit = action.audit;
  if (expectedSeq != null && Number(audit.seq) !== Number(expectedSeq)) {
    throw new Error("Dispute action sequence does not match the fork evidence");
  }
  if (
    expectedPrevStateHash != null
    && String(audit.prevStateHash || "") !== String(expectedPrevStateHash || "")
  ) {
    throw new Error("Dispute action previous hash does not match the fork evidence");
  }
  if (canonicalJson(audit.command) !== canonicalJson(action.command)) {
    throw new Error("Dispute action command does not match its audit envelope");
  }
  const signer = Number(audit.signer ?? audit.actor);
  if (signer !== Number(audit.actor)) {
    throw new Error("Dispute action was not signed by the acting player");
  }
  const signerKey = await importAuditPublicKey(players.get(signer)?.auditPublicKey || "", cryptoImpl);
  const envelopePayload = sequencedActionSignedPayload(action);
  const sequencedValid = await verifyAuditPayload(
    signerKey,
    envelopePayload,
    audit.signature || "",
    cryptoImpl,
  );
  if (!sequencedValid) {
    throw new Error("Dispute action signature is invalid");
  }
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
    throw new Error("Dispute action next state hash is invalid");
  }
  return action;
}

function sequencedActionsConflict(left, right) {
  if (!left || !right) return false;
  const leftAudit = left.audit || {};
  const rightAudit = right.audit || {};
  return (
    Number(leftAudit.seq) === Number(rightAudit.seq)
    && String(leftAudit.prevStateHash || "") === String(rightAudit.prevStateHash || "")
    && sequencedActionSignedPayloadFingerprint(left) !== sequencedActionSignedPayloadFingerprint(right)
  );
}

function votersFromCertificate(action) {
  const certificate = action?.audit?.quorumCertificate || action?.quorumCertificate || null;
  return new Set((certificate?.votes || []).map((vote) => Number(vote?.voter)));
}

function cloneAuditPayload(value) {
  return JSON.parse(JSON.stringify(normalizeForJson(value)));
}

export function buildActionForkDisputeEvidence({
  sequence,
  existingAction,
  conflictingAction,
  reason = "Conflicting signed actions were observed for the same sequence",
  detectedAt = Date.now(),
} = {}) {
  const existing = cloneAuditPayload(existingAction || {});
  const conflicting = cloneAuditPayload(conflictingAction || {});
  const accused = new Set();
  if (
    Number(existing?.audit?.actor) === Number(conflicting?.audit?.actor)
    && sequencedActionsConflict(existing, conflicting)
  ) {
    accused.add(Number(existing.audit.actor));
  }
  const leftVoters = votersFromCertificate(existing);
  const rightVoters = votersFromCertificate(conflicting);
  for (const voter of leftVoters) {
    if (rightVoters.has(voter)) accused.add(voter);
  }
  return {
    type: "action_fork_v1",
    sequence: Number(sequence || existing?.seq || existing?.audit?.seq || 0),
    reason: String(reason || ""),
    detectedAt: Number(detectedAt || Date.now()),
    accusedPlayers: [...accused].sort((left, right) => left - right),
    existingAction: existing,
    conflictingAction: conflicting,
  };
}

async function verifyActionForkDispute(dispute, players, cryptoImpl = globalThis.crypto) {
  if (!dispute || typeof dispute !== "object") {
    throw new Error("Dispute evidence is malformed");
  }
  if (String(dispute.type || "") !== "action_fork_v1") {
    throw new Error("Unsupported dispute evidence type");
  }
  const sequence = Number(dispute.sequence);
  if (!Number.isSafeInteger(sequence) || sequence <= 0) {
    throw new Error("Dispute evidence has an invalid sequence");
  }
  const existing = await verifySequencedActionEnvelope({
    entry: dispute.existingAction || dispute.existing,
    players,
    expectedSeq: sequence,
  }, cryptoImpl);
  const conflicting = await verifySequencedActionEnvelope({
    entry: dispute.conflictingAction || dispute.conflicting,
    players,
    expectedSeq: sequence,
    expectedPrevStateHash: existing.audit?.prevStateHash || "",
  }, cryptoImpl);
  if (!sequencedActionsConflict(existing, conflicting)) {
    throw new Error("Dispute evidence does not contain conflicting actions");
  }

  const roster = [...players.entries()].map(([index, player]) => ({
    index,
    auditPublicKey: player.auditPublicKey,
  }));
  const threshold = actionQuorumThreshold(roster.length);
  const quorumReports = [];
  if (threshold > 0) {
    quorumReports.push(await verifyActionQuorumCertificate({
      certificate: existing.audit?.quorumCertificate || existing.quorumCertificate,
      action: existing,
      players: roster,
      threshold,
    }, cryptoImpl));
    quorumReports.push(await verifyActionQuorumCertificate({
      certificate: conflicting.audit?.quorumCertificate || conflicting.quorumCertificate,
      action: conflicting,
      players: roster,
      threshold,
    }, cryptoImpl));
  }

  const accused = new Set();
  if (
    Number(existing.audit?.actor) === Number(conflicting.audit?.actor)
    && sequencedActionsConflict(existing, conflicting)
  ) {
    accused.add(Number(existing.audit.actor));
  }
  if (threshold > 0) {
    const leftVoters = votersFromCertificate(existing);
    const rightVoters = votersFromCertificate(conflicting);
    for (const voter of leftVoters) {
      if (rightVoters.has(voter)) accused.add(voter);
    }
  }
  const accusedPlayers = [...accused].sort((left, right) => left - right);
  if (accusedPlayers.length === 0) {
    throw new Error("Dispute evidence does not identify any equivocating signer");
  }
  const claimed = Array.isArray(dispute.accusedPlayers)
    ? dispute.accusedPlayers.map(Number).sort((left, right) => left - right)
    : accusedPlayers;
  if (canonicalJson(claimed) !== canonicalJson(accusedPlayers)) {
    throw new Error("Dispute evidence accused players do not match the signed actions");
  }
  return {
    type: "action_fork_v1",
    sequence,
    accusedPlayers,
    quorumVoters: quorumReports.map((report) => report.voters),
  };
}

async function verifyTranscriptDisputes(disputes, players, cryptoImpl = globalThis.crypto) {
  const entries = Array.isArray(disputes) ? disputes : [];
  const reports = [];
  for (const dispute of entries) {
    reports.push(await verifyActionForkDispute(dispute, players, cryptoImpl));
  }
  return reports;
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
  if (!String(match?.hostPeerId || "").trim()) {
    throw new Error("Match genesis is missing its host peer id");
  }
  const playerSeats = new Set(seats);
  const peerIds = new Set();
  const ceremonyOwners = new Set();
  for (const player of players) {
    const seat = Number(player?.index);
    const peerId = String(player?.peerId || "").trim();
    if (!peerId) {
      throw new Error(`Match genesis player ${seat + 1} is missing peer id`);
    }
    if (peerIds.has(peerId)) {
      throw new Error("Match genesis requires unique player peer ids");
    }
    peerIds.add(peerId);
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
  const host = players.find((player) => Number(player?.index) === Number(genesis.hostSeat));
  if (!host?.auditPublicKey) {
    throw new Error("Match genesis host signer is missing");
  }
  const payload = matchGenesisPayload(match);
  const payloadHash = await sha256Hex(canonicalJson(payload), cryptoImpl);
  if (payloadHash !== String(genesis.payloadHash || "")) {
    throw new Error("Match genesis payload hash mismatch");
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
    checkpoint: normalizePublicCheckpointForHash(checkpoint),
  }), cryptoImpl);
}

function publicCheckpointPlayerId(player) {
  const id = Number(player?.id ?? player?.index ?? player?.seat);
  return Number.isInteger(id) && id >= 0 ? id : null;
}

function publicCheckpointPlayerFlag(player, camelKey, snakeKey) {
  return Boolean(player?.[camelKey] ?? player?.[snakeKey]);
}

function matchOutcomeFromPublicCheckpoint(checkpoint) {
  if (!checkpoint || typeof checkpoint !== "object") return null;
  const players = Array.isArray(checkpoint.players) ? checkpoint.players : [];
  if (players.length === 0) return null;
  const winners = players.filter((player) =>
    publicCheckpointPlayerFlag(player, "hasWon", "has_won")
  );
  if (winners.length === 1) {
    const winner = winners[0];
    return {
      status: "winner",
      winner: publicCheckpointPlayerId(winner),
      winnerName: String(winner?.name || ""),
    };
  }
  if (winners.length > 1) {
    return {
      status: "draw",
      winners: winners.map(publicCheckpointPlayerId).filter((id) => id != null),
    };
  }
  const activePlayers = players.filter((player) =>
    !publicCheckpointPlayerFlag(player, "hasLost", "has_lost")
    && !publicCheckpointPlayerFlag(player, "hasLeftGame", "has_left_game")
  );
  const eliminatedPlayers = players.length - activePlayers.length;
  if (activePlayers.length === 1 && eliminatedPlayers > 0) {
    const winner = activePlayers[0];
    return {
      status: "winner",
      winner: publicCheckpointPlayerId(winner),
      winnerName: String(winner?.name || ""),
    };
  }
  if (activePlayers.length === 0 && eliminatedPlayers > 0) {
    return { status: "draw" };
  }
  return null;
}

function normalizeOutcomeStatus(status) {
  const normalized = String(status || "").trim().toLowerCase();
  if (normalized === "stalled") return "stalled_or_incomplete";
  return normalized;
}

function verifyTranscriptOutcome({
  transcript,
  checkpointOutcome,
  disputeReports,
  finalStateHash,
  finalPublicCheckpointHash,
}) {
  const disputeAccusedPlayers = Array.from(new Set(
    disputeReports.flatMap((report) => report.accusedPlayers || [])
  )).sort((left, right) => left - right);
  const derived = disputeReports.length > 0
    ? {
        status: "disputed",
        disputed: true,
        accusedPlayers: disputeAccusedPlayers,
      }
    : (
      checkpointOutcome || {
        status: "stalled_or_incomplete",
        stalled: true,
      }
    );
  const outcome = transcript?.outcome;
  if (outcome && typeof outcome === "object") {
    const claimedStatus = normalizeOutcomeStatus(outcome.status);
    if (claimedStatus && claimedStatus !== derived.status) {
      throw new Error("Match outcome does not match verifiable transcript evidence");
    }
    if (
      derived.status === "winner"
      && outcome.winner != null
      && Number(outcome.winner) !== Number(derived.winner)
    ) {
      throw new Error("Match outcome winner does not match the final public checkpoint");
    }
    if (derived.status === "disputed" && Array.isArray(outcome.accusedPlayers)) {
      const claimedAccused = outcome.accusedPlayers
        .map(Number)
        .sort((left, right) => left - right);
      if (canonicalJson(claimedAccused) !== canonicalJson(disputeAccusedPlayers)) {
        throw new Error("Match outcome accused players do not match dispute evidence");
      }
    }
  }
  return {
    ...derived,
    finalStateHash,
    finalPublicCheckpointHash,
  };
}

export async function transcriptActionsHash(actions = [], cryptoImpl = globalThis.crypto) {
  return sha256Hex(canonicalJson({
    domain: "ironsmith-resync-actions-v1",
    actions,
  }), cryptoImpl);
}

function transcriptLastSequence(actions = []) {
  if (!Array.isArray(actions) || actions.length === 0) return 0;
  const lastSequence = Number(actions.at(-1)?.seq || 0);
  if (!Number.isSafeInteger(lastSequence) || lastSequence < 0) {
    throw new Error("Resync action log has an invalid final sequence");
  }
  return lastSequence;
}

export function assertResyncActionsExtendLocalTranscript({
  actionEntries = [],
  localActions = [],
  localLastSequence = 0,
} = {}) {
  const localSequence = Number(localLastSequence || 0);
  if (!Number.isSafeInteger(localSequence) || localSequence < 0) {
    throw new Error("Local transcript has an invalid sequence");
  }

  const remoteActions = Array.isArray(actionEntries) ? actionEntries : [];
  const finalSequence = transcriptLastSequence(remoteActions);
  if (finalSequence < localSequence) {
    throw new Error(
      `Resync transcript is older than the local transcript. Expected at least ${localSequence}, received ${finalSequence}.`
    );
  }

  const localPrefix = (Array.isArray(localActions) ? localActions : [])
    .filter((entry) => {
      const seq = Number(entry?.seq || 0);
      return seq > 0 && seq <= localSequence;
    })
    .sort((left, right) => Number(left?.seq || 0) - Number(right?.seq || 0));

  if (localSequence > 0 && localPrefix.length !== localSequence) {
    throw new Error("Local transcript is incomplete; refusing resync without continuity proof");
  }

  for (let index = 0; index < localPrefix.length; index += 1) {
    const localEntry = localPrefix[index];
    const remoteEntry = remoteActions[index];
    const expectedSeq = Number(localEntry?.seq || 0);
    if (expectedSeq !== index + 1) {
      throw new Error("Local transcript is incomplete; refusing resync without continuity proof");
    }
    if (!remoteEntry || Number(remoteEntry?.seq || 0) !== expectedSeq) {
      throw new Error(`Resync transcript does not include local action ${expectedSeq}`);
    }
    if (canonicalJson(remoteEntry) !== canonicalJson(localEntry)) {
      throw new Error(`Resync transcript action ${expectedSeq} does not match local transcript`);
    }
  }

  return {
    localSequence,
    finalSequence,
    checkedActions: localPrefix.length,
  };
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
  const actionLastSequence = transcriptLastSequence(actions);
  const claimedLastSequence = Number(lastSequence ?? actionLastSequence);
  if (claimedLastSequence !== actionLastSequence) {
    throw new Error("Resync last sequence does not match action log");
  }
  const payload = {
    domain: "ironsmith-resync-envelope-v1",
    matchId: String(matchId || ""),
    signer: Number(signer),
    lastSequence: actionLastSequence,
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
  const expectedLastSequence = transcriptLastSequence(actions);
  if (payload.lastSequence !== expectedLastSequence) {
    throw new Error("Resync last sequence does not match action log");
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
  decklistSalt = null,
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
  const normalizedDecklistSalt = decklistSalt == null
    ? randomHex(cryptoImpl, 32)
    : String(decklistSalt);
  const decklistCommitment = await sha256Hex(canonicalJson({
    domain: "ironsmith-ui-audit-decklist-commitment-v2",
    matchId,
    owner,
    deck: normalizedDeck,
    sideboard: normalizedSideboard,
    commanders: normalizedCommanders,
    salt: normalizedDecklistSalt,
  }), cryptoImpl);
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
    domain: "ironsmith-ui-audit-deck-commitment-root-v2",
    matchId,
    owner,
    decklistCommitment,
    slots,
  }), cryptoImpl);
  return {
    matchId,
    owner,
    deckCount: normalizedDeck.length,
    sideboardCount: normalizedSideboard.length,
    commanderCount: normalizedCommanders.length,
    decklistHash,
    decklistSalt: normalizedDecklistSalt,
    decklistCommitment,
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
    decklistCommitment: String(manifest.decklistCommitment || ""),
    commitmentRoot: String(manifest.commitmentRoot || ""),
    slotCommitments: Array.isArray(manifest.slotCommitments)
      ? manifest.slotCommitments.map((slot) => ({
          slot: Number(slot.slot),
          commitment: String(slot.commitment || ""),
        }))
      : [],
  };
}

function ziffleDeckHashFromCommitment(commitment) {
  const normalized = String(commitment || "");
  if (!normalized.startsWith("ziffle:")) return "";
  const lastColon = normalized.lastIndexOf(":");
  return lastColon > "ziffle:".length
    ? normalized.slice("ziffle:".length, lastColon)
    : "";
}

function zifflePositionFromCommitment(commitment) {
  const normalized = String(commitment || "");
  if (!normalized.startsWith("ziffle:")) return null;
  const lastColon = normalized.lastIndexOf(":");
  if (lastColon <= "ziffle:".length) return null;
  const position = Number(normalized.slice(lastColon + 1));
  return Number.isSafeInteger(position) && position >= 0 ? position : null;
}

function ziffleRuntimeCommitment(deckHash, position) {
  return `ziffle:${String(deckHash || "")}:${Number(position)}`;
}

function normalizeZiffleKeys(keys = []) {
  return (Array.isArray(keys) ? keys : [])
    .map((key) => ({
      player: Number(key?.player),
      publicKeyHex: String(key?.publicKeyHex || ""),
    }))
    .sort((left, right) => Number(left.player) - Number(right.player));
}

function normalizeZiffleRevealTokens(tokens = [], position = null) {
  return (Array.isArray(tokens) ? tokens : [])
    .filter((token) =>
      position == null
      || token?.cardPosition == null
      || Number(token.cardPosition) === Number(position)
    )
    .map((token) => ({
      player: Number(token?.player),
      publicKeyHex: String(token?.publicKeyHex || ""),
      tokenHex: String(token?.tokenHex || ""),
      proofHex: String(token?.proofHex || ""),
    }))
    .sort((left, right) => Number(left.player) - Number(right.player));
}

function normalizeShuffleOrder(value) {
  return (Array.isArray(value) ? value : [])
    .map((entry) => Number(entry))
    .filter((entry) => Number.isSafeInteger(entry) && entry >= 0);
}

function validatedShuffleOrderForProof(proof, seq) {
  const deckCount = Number(proof?.deckCount || 0);
  const beforeOrder = normalizeShuffleOrder(proof?.beforeOrder ?? proof?.before_order);
  const afterOrder = normalizeShuffleOrder(proof?.afterOrder ?? proof?.after_order);
  if (deckCount > 1 && (beforeOrder.length !== deckCount || afterOrder.length !== deckCount)) {
    throw new Error(`Shuffle proof at sequence ${seq} is missing its object order`);
  }
  if (beforeOrder.length > 0 && new Set(beforeOrder).size !== beforeOrder.length) {
    throw new Error(`Shuffle proof at sequence ${seq} contains duplicate source objects`);
  }
  if (afterOrder.length > 0 && new Set(afterOrder).size !== afterOrder.length) {
    throw new Error(`Shuffle proof at sequence ${seq} contains duplicate shuffled objects`);
  }
  if (beforeOrder.length > 0 && afterOrder.length > 0) {
    if (beforeOrder.length !== afterOrder.length) {
      throw new Error(`Shuffle proof at sequence ${seq} object order length mismatch`);
    }
    const beforeSet = new Set(beforeOrder);
    if (!afterOrder.every((objectId) => beforeSet.has(objectId))) {
      throw new Error(`Shuffle proof at sequence ${seq} changes the shuffled object set`);
    }
  }
  return { beforeOrder, afterOrder };
}

function ziffleCeremonyFromShuffleProof(proof, seq) {
  const { beforeOrder, afterOrder } = validatedShuffleOrderForProof(proof, seq);
  return {
    owner: Number(proof?.owner),
    deckCount: Number(proof?.deckCount || 0),
    context: String(proof?.context || ""),
    keyContext: String(proof?.keyContext || proof?.context || ""),
    keys: proof?.keys || [],
    steps: proof?.steps || [],
    deckHash: String(proof?.deckHash || ""),
    beforeOrder,
    afterOrder,
    authenticatedOrder: true,
  };
}

export function buildZiffleOpeningProof({
  opening,
  ceremony,
  position = opening?.position,
  originalSlot = opening?.slot,
  shuffleOriginalSlot = originalSlot,
  positionCommitment = opening?.positionCommitment,
  tokens = [],
}) {
  const proof = {
    type: ZIFFLE_OPENING_PROOF_TYPE,
    owner: Number(opening?.owner ?? ceremony?.owner),
    position: Number(position),
    originalSlot: Number(originalSlot),
    positionCommitment: String(
      positionCommitment
      || ziffleRuntimeCommitment(ceremony?.deckHash, position)
    ),
    commitment: String(opening?.commitment || ""),
    deckCount: Number(ceremony?.deckCount || 0),
    deckHash: String(ceremony?.deckHash || ""),
    context: String(ceremony?.context || ""),
    keyContext: String(ceremony?.keyContext || ceremony?.context || ""),
    keys: normalizeZiffleKeys(ceremony?.keys || []),
    tokens: normalizeZiffleRevealTokens(tokens, position),
  };
  if (opening?.objectId != null || opening?.object_id != null) {
    proof.objectId = Number(opening.objectId ?? opening.object_id);
  }
  if (Number(shuffleOriginalSlot) !== Number(originalSlot)) {
    proof.shuffleOriginalSlot = Number(shuffleOriginalSlot);
  }
  return proof;
}

function ziffleObjectOrderLinksOpening(ceremony, shuffleOriginalSlot, position, opening) {
  if (ceremony?.authenticatedOrder !== true) return false;
  const objectId = Number(opening?.objectId ?? opening?.object_id);
  if (!Number.isSafeInteger(objectId) || objectId < 0) return false;
  const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
  const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
  if (beforeOrder.length === 0 && afterOrder.length === 0) return false;
  const beforeMatches =
    beforeOrder.length === 0
    || Number(beforeOrder[Number(shuffleOriginalSlot)]) === objectId;
  const afterMatches =
    afterOrder.length === 0
    || Number(afterOrder[Number(position)]) === objectId;
  return beforeMatches && afterMatches;
}

function ziffleRevealMatchesOpening(ceremony, revealOriginalSlot, position, opening) {
  if (Number(revealOriginalSlot) === Number(opening?.slot)) return true;
  return ziffleObjectOrderLinksOpening(ceremony, revealOriginalSlot, position, opening);
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
  const combinedSeedHex = await fairRandomCombinedSeedHex({
    matchId,
    seq,
    requirementId: reveal?.requirementId,
    commits: reveal.commits || [],
    reveals: reveal.reveals || [],
  }, cryptoImpl);
  if (combinedSeedHex !== String(reveal?.combinedSeedHex || "")) {
    throw new Error("Fair-random combined seed is invalid");
  }
}

async function verifyAuditOpenings({
  openings = [],
  manifests,
  ziffleCeremonies = [],
  verifyZiffleOpening,
  expectedZiffleKeys = [],
  expectedMatchId = "",
  players = new Map(),
  seq = 0,
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
    await verifyZifflePositionOpening({
      opening,
      manifest,
      ziffleCeremonies,
      verifyZiffleOpening,
      expectedZiffleKeys,
      expectedMatchId,
      players,
      seq,
    });
  }
}

async function verifyZifflePositionOpening({
  opening,
  manifest,
  ziffleCeremonies = [],
  verifyZiffleOpening,
  expectedZiffleKeys = [],
  expectedMatchId = "",
  players = new Map(),
  seq = 0,
}) {
  const explicitPosition = opening?.position == null ? null : Number(opening.position);
  const commitmentPosition = zifflePositionFromCommitment(opening?.positionCommitment);
  const position = explicitPosition ?? commitmentPosition;
  const positionCommitment = String(opening?.positionCommitment || "");
  const usesZifflePosition =
    position != null
    || Boolean(positionCommitment && ziffleDeckHashFromCommitment(positionCommitment))
    || Boolean(opening?.ziffleReveal || opening?.ziffleProof || opening?.positionOpeningProof);
  if (!usesZifflePosition) return;
  if (!Number.isSafeInteger(position) || position < 0) {
    throw new Error(`Ziffle opening at sequence ${seq} is missing a valid shuffled position`);
  }
  const proof = opening?.ziffleReveal || opening?.ziffleProof || opening?.positionOpeningProof;
  if (!proof || typeof proof !== "object") {
    throw new Error(`Ziffle opening at sequence ${seq} is missing its position reveal proof`);
  }
  if (String(proof.type || "") !== ZIFFLE_OPENING_PROOF_TYPE) {
    throw new Error(`Ziffle opening at sequence ${seq} has an unsupported position proof`);
  }
  const owner = Number(opening.owner);
  if (!Number.isInteger(owner) || !players.has(owner)) {
    throw new Error(`Ziffle opening at sequence ${seq} references an unknown owner`);
  }
  if (Number(proof.owner) !== owner) {
    throw new Error(`Ziffle opening at sequence ${seq} proof owner mismatch`);
  }
  if (Number(proof.position) !== Number(position)) {
    throw new Error(`Ziffle opening at sequence ${seq} proof position mismatch`);
  }
  if (Number(proof.originalSlot) !== Number(opening.slot)) {
    throw new Error(`Ziffle opening at sequence ${seq} proof slot mismatch`);
  }
  if (opening.commitment && proof.commitment && String(proof.commitment) !== String(opening.commitment)) {
    throw new Error(`Ziffle opening at sequence ${seq} proof card commitment mismatch`);
  }
  if (Number(proof.deckCount) !== Number(manifest.deckCount || 0)) {
    throw new Error(`Ziffle opening at sequence ${seq} proof deck count mismatch`);
  }
  const matchId = String(expectedMatchId || "");
  const proofContext = String(proof.context || "");
  const proofKeyContext = String(proof.keyContext || proof.context || "");
  if (!matchId || (proofContext !== matchId && !proofContext.startsWith(`${matchId}:`))) {
    throw new Error(`Ziffle opening at sequence ${seq} is bound to a different match`);
  }
  if (proofKeyContext !== matchId) {
    throw new Error(`Ziffle opening at sequence ${seq} uses a mismatched ziffle key context`);
  }
  const ceremony = (Array.isArray(ziffleCeremonies) ? ziffleCeremonies : []).find((entry) =>
    Number(entry?.owner) === owner
    && String(entry?.context || "") === proofContext
    && String(entry?.deckHash || "") === String(proof.deckHash || ziffleDeckHashFromCommitment(positionCommitment))
  );
  if (!ceremony) {
    throw new Error(`Ziffle opening at sequence ${seq} references an unknown shuffle ceremony`);
  }
  const expectedPositionCommitment = ziffleRuntimeCommitment(ceremony.deckHash, position);
  if (positionCommitment && positionCommitment !== expectedPositionCommitment) {
    throw new Error(`Ziffle opening at sequence ${seq} position commitment mismatch`);
  }
  if (String(proof.positionCommitment || "") !== expectedPositionCommitment) {
    throw new Error(`Ziffle opening at sequence ${seq} proof position commitment mismatch`);
  }
  if (String(proof.deckHash || "") !== String(ceremony.deckHash || "")) {
    throw new Error(`Ziffle opening at sequence ${seq} proof deck hash mismatch`);
  }
  const expectedKeysJson = canonicalJson(normalizeZiffleKeys(expectedZiffleKeys || []));
  if (
    expectedKeysJson !== canonicalJson(normalizeZiffleKeys(ceremony.keys || []))
    || expectedKeysJson !== canonicalJson(normalizeZiffleKeys(proof.keys || []))
  ) {
    throw new Error(`Ziffle opening at sequence ${seq} is not bound to the signed ziffle key roster`);
  }
  if (!Array.isArray(proof.tokens) || proof.tokens.length === 0) {
    throw new Error(`Ziffle opening at sequence ${seq} is missing reveal tokens`);
  }
  if (typeof verifyZiffleOpening !== "function") {
    throw new Error("Live audit transcript contains ziffle position openings but no verifier was provided");
  }
  const verified = await verifyZiffleOpening({
    proof,
    opening,
    ceremony,
    seq,
  });
  const verifiedOriginalSlot = Number(
    typeof verified === "object" && verified
      ? verified.originalSlot
      : verified
  );
  const proofShuffleOriginalSlot = Number(proof.shuffleOriginalSlot ?? proof.originalSlot);
  if (!Number.isSafeInteger(proofShuffleOriginalSlot) || proofShuffleOriginalSlot < 0) {
    throw new Error(`Ziffle opening at sequence ${seq} proof shuffle slot mismatch`);
  }
  if (verifiedOriginalSlot !== proofShuffleOriginalSlot) {
    throw new Error(`Ziffle opening at sequence ${seq} reveals a different shuffle slot`);
  }
  if (!ziffleRevealMatchesOpening(ceremony, verifiedOriginalSlot, position, opening)) {
    throw new Error(`Ziffle opening at sequence ${seq} reveals a different committed slot`);
  }
}

async function verifyPrivateViewProofStructure(
  proofs = [],
  players,
  cryptoImpl = globalThis.crypto,
) {
  for (const proof of proofs || []) {
    const type = String(proof?.type || "");
    if (type !== "encrypted_private_opening" && type !== "encrypted_private_view") {
      throw new Error("Unsupported private-view proof type");
    }
    const viewer = Number(proof.viewer);
    if (!Number.isInteger(viewer) || !players.has(viewer)) {
      throw new Error("Private-view proof references an unknown viewer");
    }
    if (type === "encrypted_private_opening") {
      if (!proof?.encryptedOpening?.ciphertextHex || !proof?.encryptedOpening?.plaintextHash) {
        throw new Error("Private-view proof is missing encrypted opening material");
      }
      const viewerKey = players.get(viewer)?.auditEncryptionPublicKey || "";
      if (viewerKey && String(proof.encryptedOpening.recipientPublicKey || "") !== viewerKey) {
        throw new Error("Private-view proof targets the wrong viewer key");
      }
      continue;
    }

    const owner = Number(proof.owner);
    if (!Number.isInteger(owner) || !players.has(owner)) {
      throw new Error("Private-view proof references an unknown owner");
    }
    const count = Number(proof.count || 0);
    if (!Number.isInteger(count) || count < 0) {
      throw new Error("Private-view proof contains an invalid view count");
    }
    if (!Array.isArray(proof.openingHashes)) {
      throw new Error("Private-view proof is missing opening hashes");
    }
    if (proof.materialHash) {
      const material = { ...proof };
      delete material.materialHash;
      const expectedHash = await sha256Hex(canonicalJson(material), cryptoImpl);
      if (expectedHash !== String(proof.materialHash || "")) {
        throw new Error("Private-view proof material hash mismatch");
      }
    }
  }
}

async function verifyShuffleProofList({
  shuffleProofs = [],
  verifyShuffleProof,
  expectedZiffleKeys = [],
  expectedMatchId = "",
  players = new Map(),
  seq,
}) {
  const expectedKeysJson = canonicalJson(expectedZiffleKeys || []);
  const matchId = String(expectedMatchId || "");
  const verifiedCeremonies = [];
  for (const proof of shuffleProofs || []) {
    if (String(proof?.type || "") !== "ziffle_shuffle") {
      throw new Error(`Unsupported shuffle proof type at sequence ${seq}`);
    }
    const owner = Number(proof?.owner);
    if (!Number.isInteger(owner) || !players.has(owner)) {
      throw new Error(`Shuffle proof at sequence ${seq} references an unknown owner`);
    }
    if (String(proof?.zone || "library") !== "library") {
      throw new Error(`Shuffle proof at sequence ${seq} references an unsupported zone`);
    }
    if (!Number.isInteger(Number(proof?.deckCount)) || Number(proof?.deckCount) < 0) {
      throw new Error(`Shuffle proof at sequence ${seq} contains an invalid deck count`);
    }
    if (!Array.isArray(proof?.keys) || canonicalJson(proof.keys) !== expectedKeysJson) {
      throw new Error(`Shuffle proof at sequence ${seq} is not bound to the signed ziffle key roster`);
    }
    const proofContext = String(proof?.context || "");
    const proofKeyContext = String(proof?.keyContext || proof?.context || "");
    if (!matchId || !proofContext.startsWith(`${matchId}:`)) {
      throw new Error(`Shuffle proof at sequence ${seq} is bound to a different match`);
    }
    if (proofKeyContext !== matchId) {
      throw new Error(`Shuffle proof at sequence ${seq} uses a mismatched ziffle key context`);
    }
    if (Number(proof?.epoch) !== Number(seq)) {
      throw new Error(`Shuffle proof at sequence ${seq} is bound to a different action`);
    }
    if (typeof verifyShuffleProof !== "function") {
      throw new Error("Live audit transcript contains shuffle proofs but no verifier was provided");
    }
    const ceremony = ziffleCeremonyFromShuffleProof(proof, seq);
    await verifyShuffleProof(proof);
    verifiedCeremonies.push(ceremony);
  }
  return verifiedCeremonies;
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
  const expectedZiffleKeys = Array.isArray(transcript.match?.ziffleKeys)
    ? transcript.match.ziffleKeys
    : [];
  const expectedShuffleMatchId = String(
    transcript.match?.auditMatchId
      || transcript.matchId
      || transcript.match?.matchId
      || "",
  );
  const verifiedZiffleCeremonies = (Array.isArray(transcript.match?.ziffleCeremonies)
    ? transcript.match.ziffleCeremonies
    : []
  ).map((ceremony) => ({
    ...ceremony,
    authenticatedOrder: false,
  }));
  let stateHash = String(transcript.initialStateHash || "0".repeat(64));
  let clockHash = INITIAL_MATCH_CLOCK_HASH;
  let finalPublicCheckpointHash = String(transcript.initialPublicCheckpointHash || "");
  let expectedSeq = 1;
  const actions = Array.isArray(transcript.actions) ? transcript.actions : [];
  for (const entry of actions) {
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
    const disconnectForfeit = isDisconnectForfeitCommand(audit.command);
    const actionQuorumPlayers = forfeitTarget == null
      ? activeQuorumPlayers
      : activeQuorumPlayers.filter((player) =>
          Number(player.index) !== forfeitTarget
        );
    const actionQuorumThresholdOverride = forfeitTarget == null
      ? null
      : (
        disconnectForfeit || activeQuorumPlayers.length < 3 || Number(audit.actor) === forfeitTarget
          ? 0
          : actionQuorumPlayers.length
      );
    if (disconnectForfeit) {
      await verifyDisconnectForfeitCertificate({
        certificate: audit.command?.disconnect_certificate || audit.command?.disconnectCertificate,
        command: {
          ...audit.command,
          matchId: audit.matchId,
        },
        players: actionQuorumPlayers,
      }, cryptoImpl);
    }
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
    const actionZiffleCeremonies = await verifyShuffleProofList({
      shuffleProofs: audit.shuffleProofs || [],
      verifyShuffleProof: options.verifyShuffleProof,
      expectedZiffleKeys,
      expectedMatchId: expectedShuffleMatchId,
      players,
      seq: expectedSeq,
    });
    await verifyAuditOpenings({
      openings: audit.openings || [],
      manifests,
      ziffleCeremonies: [
        ...verifiedZiffleCeremonies,
        ...actionZiffleCeremonies,
      ],
      verifyZiffleOpening: options.verifyZiffleOpening,
      expectedZiffleKeys,
      expectedMatchId: expectedShuffleMatchId,
      players,
      seq: expectedSeq,
    }, cryptoImpl);
    await verifyPrivateViewProofStructure(audit.privateViewProofs || [], players, cryptoImpl);
    for (const reveal of audit.rngReveals || []) {
      await verifyFairRandomReveal({
        reveal,
        expectedPlayers,
        players,
        matchId: audit.matchId,
        seq: audit.seq,
      }, cryptoImpl);
    }
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
    finalPublicCheckpointHash = String(audit.publicCheckpointHash || "");
    verifiedZiffleCeremonies.push(...actionZiffleCeremonies);
    expectedSeq += 1;
  }
  if (
    transcript.finalStateHash
    && String(transcript.finalStateHash || "") !== stateHash
  ) {
    throw new Error("Live audit transcript final state hash does not match verified actions");
  }
  let checkpointOutcome = null;
  if (transcript.finalPublicCheckpoint) {
    const checkpointHash = await publicCheckpointHash(transcript.finalPublicCheckpoint, cryptoImpl);
    if (checkpointHash !== finalPublicCheckpointHash) {
      throw new Error("Live audit transcript final public checkpoint hash mismatch");
    }
    checkpointOutcome = matchOutcomeFromPublicCheckpoint(transcript.finalPublicCheckpoint);
  }
  if (
    transcript.finalPublicCheckpointHash
    && String(transcript.finalPublicCheckpointHash || "") !== finalPublicCheckpointHash
  ) {
    throw new Error("Live audit transcript declared final public checkpoint hash does not match verified actions");
  }
  const replayTranscript = typeof options.replayTranscript === "function"
    ? options.replayTranscript
    : null;
  const requireEngineReplay = options.requireEngineReplay !== false;
  if (requireEngineReplay && !replayTranscript) {
    throw new Error("Live audit transcript verification requires engine replay");
  }
  let engineReplay = null;
  if (replayTranscript) {
    const replayReport = await replayTranscript({
      transcript,
      match: transcript.match,
      actions,
      initialPublicCheckpointHash: transcript.initialPublicCheckpointHash || "",
      finalPublicCheckpointHash,
      finalPublicCheckpoint: transcript.finalPublicCheckpoint || null,
    });
    const normalizedReplayReport =
      typeof replayReport === "string"
        ? { finalPublicCheckpointHash: replayReport }
        : replayReport || {};
    const replayedActions = Array.isArray(normalizedReplayReport.actions)
      ? normalizedReplayReport.actions
      : Array.isArray(normalizedReplayReport.actionReports)
        ? normalizedReplayReport.actionReports
        : [];
    if (requireEngineReplay && actions.length > 0) {
      if (replayedActions.length !== actions.length) {
        throw new Error("Engine replay must report every action in the transcript");
      }
      const replayedSeqs = new Set(replayedActions.map((entry) => Number(entry?.seq)));
      const expectedSeqs = new Set(actions.map((entry) => Number(entry?.audit?.seq ?? entry?.seq)));
      if (
        replayedSeqs.size !== expectedSeqs.size
        || [...expectedSeqs].some((seq) => !replayedSeqs.has(seq))
      ) {
        throw new Error("Engine replay action coverage does not match the transcript");
      }
    }
    if (replayedActions.length > 0) {
      const expectedActionHashes = new Map(actions.map((entry) => [
        Number(entry?.audit?.seq ?? entry?.seq),
        String(entry?.audit?.publicCheckpointHash || ""),
      ]));
      for (const replayedAction of replayedActions) {
        const seq = Number(replayedAction?.seq);
        const expectedHash = expectedActionHashes.get(seq);
        if (!expectedHash) {
          throw new Error(`Engine replay reported an unknown action sequence ${seq}`);
        }
        const actualHash = String(replayedAction?.publicCheckpointHash || "");
        if (actualHash !== expectedHash) {
          throw new Error(`Engine replay public checkpoint hash mismatch at sequence ${seq}`);
        }
      }
    }
    const replayFinalPublicCheckpointHash = String(
      normalizedReplayReport.finalPublicCheckpointHash
        || replayedActions.at(-1)?.publicCheckpointHash
        || "",
    );
    if (!replayFinalPublicCheckpointHash && requireEngineReplay && actions.length > 0) {
      throw new Error("Engine replay did not report a final public checkpoint hash");
    }
    if (
      replayFinalPublicCheckpointHash
      && replayFinalPublicCheckpointHash !== finalPublicCheckpointHash
    ) {
      throw new Error("Engine replay final public checkpoint hash does not match verified transcript");
    }
    engineReplay = {
      verified: true,
      replayedActions: Number(
        normalizedReplayReport.replayedActions
          ?? (replayedActions.length > 0 ? replayedActions.length : actions.length)
      ),
      finalPublicCheckpointHash: replayFinalPublicCheckpointHash || finalPublicCheckpointHash,
    };
  }
  const disputeReports = await verifyTranscriptDisputes(
    transcript.disputes || transcript.disputeEvidence || [],
    players,
    cryptoImpl,
  );
  const outcome = verifyTranscriptOutcome({
    transcript,
    checkpointOutcome,
    disputeReports,
    finalStateHash: stateHash,
    finalPublicCheckpointHash,
  });
  return {
    valid: true,
    verifiedActions: expectedSeq - 1,
    initialPublicCheckpointHash: transcript.initialPublicCheckpointHash || "",
    finalPublicCheckpointHash,
    finalStateHash: stateHash,
    outcome,
    engineReplay,
    disputes: disputeReports,
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
