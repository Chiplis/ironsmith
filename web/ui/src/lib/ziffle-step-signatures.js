// Signed ziffle shuffle steps.
//
// A Ziffle shuffle proof needs no secret key, so without a signature a single
// participant could author every step of a ceremony (and therefore know, or
// choose, the final order of any deck). Each shuffler signs, with its genesis
// audit key, a statement binding the ceremony (key context, context, owner,
// zone, size), its step index, the exact input it shuffled (the authenticated
// input deck for step 0, the previous step's ciphertexts otherwise) and the
// exact output and proof it produced. Every ceremony verifier requires one
// valid signature by the expected seat for every step, so an assembler cannot
// fabricate, reorder, drop, or transplant anyone else's step.
import {
  canonicalJson,
  importAuditPublicKey,
  sha256Hex,
  signAuditPayload,
  verifyAuditPayload,
} from "./multiplayer-audit.js";

export const ZIFFLE_SHUFFLE_STEP_SIGNATURE_DOMAIN = "ironsmith-ziffle-shuffle-step-v1";

function sortedZiffleKeySeats(keys) {
  return (Array.isArray(keys) ? keys : [])
    .map((key) => Number(key?.player))
    .sort((left, right) => left - right);
}

// `ceremony` needs: owner, zone (default library), deckCount, context,
// keyContext, inputDeck (optional). `previousSteps` are the steps before this
// one, exactly as they will appear in the ceremony.
export async function ziffleShuffleStepStatement(ceremony, previousSteps, step, cryptoImpl = globalThis.crypto) {
  const stepIndex = Array.isArray(previousSteps) ? previousSteps.length : 0;
  const previous = stepIndex > 0 ? previousSteps[stepIndex - 1] : null;
  const inputDigest = previous
    ? await sha256Hex(`deck:${String(previous.deckHex || "")}`, cryptoImpl)
    : await sha256Hex(canonicalJson({
      initial: true,
      inputDeck: ceremony?.inputDeck ?? null,
    }), cryptoImpl);
  const context = String(ceremony?.context || "");
  return {
    domain: ZIFFLE_SHUFFLE_STEP_SIGNATURE_DOMAIN,
    keyContext: String(ceremony?.keyContext || context),
    context,
    owner: Number(ceremony?.owner),
    zone: String(ceremony?.zone || "library"),
    deckCount: Number(ceremony?.deckCount),
    stepIndex,
    shuffler: Number(step?.shuffler),
    inputDigest,
    outputDigest: await sha256Hex(`deck:${String(step?.deckHex || "")}`, cryptoImpl),
    proofDigest: await sha256Hex(`proof:${String(step?.proofHex || "")}`, cryptoImpl),
  };
}

export async function signZiffleShuffleStep(keyPair, ceremony, previousSteps, step, cryptoImpl = globalThis.crypto) {
  const statement = await ziffleShuffleStepStatement(ceremony, previousSteps, step, cryptoImpl);
  return signAuditPayload(keyPair, statement, cryptoImpl);
}

const verifiedStepSignatureCache = new Set();
const VERIFIED_STEP_SIGNATURE_CACHE_LIMIT = 512;

// Throws unless the ceremony has exactly one step per roster key, the i-th
// step is attributed to the i-th seat in ascending key order (the order the
// Rust verifier enforces), and every step carries a valid signature by that
// seat's audit key. `auditPublicKeyForSeat(seat)` must resolve keys from the
// signed genesis roster only.
export async function verifyZiffleCeremonyStepSignatures(
  ceremony,
  auditPublicKeyForSeat,
  cryptoImpl = globalThis.crypto,
) {
  const steps = Array.isArray(ceremony?.steps) ? ceremony.steps : [];
  const seats = sortedZiffleKeySeats(ceremony?.keys);
  const label = `ziffle shuffle for player ${Number(ceremony?.owner) + 1}`;
  if (seats.length === 0 || steps.length !== seats.length) {
    throw new Error(`The ${label} must contain one shuffle step per player`);
  }
  for (let index = 0; index < steps.length; index += 1) {
    const step = steps[index];
    const seat = seats[index];
    if (Number(step?.shuffler) !== seat) {
      throw new Error(`The ${label} step ${index} is attributed to the wrong player`);
    }
    const signature = String(step?.signature || "");
    const publicKeyHex = String(auditPublicKeyForSeat(seat) || "");
    if (!signature || !publicKeyHex) {
      throw new Error(`The ${label} step ${index} is not signed by player ${seat + 1}`);
    }
    const statement = await ziffleShuffleStepStatement(ceremony, steps.slice(0, index), step, cryptoImpl);
    const cacheKey = await sha256Hex(canonicalJson({ statement, signature, publicKeyHex }), cryptoImpl);
    if (verifiedStepSignatureCache.has(cacheKey)) continue;
    const publicKey = await importAuditPublicKey(publicKeyHex, cryptoImpl);
    if (!await verifyAuditPayload(publicKey, statement, signature, cryptoImpl)) {
      throw new Error(`The ${label} step ${index} has an invalid signature from player ${seat + 1}`);
    }
    if (verifiedStepSignatureCache.size >= VERIFIED_STEP_SIGNATURE_CACHE_LIMIT) {
      verifiedStepSignatureCache.delete(verifiedStepSignatureCache.values().next().value);
    }
    verifiedStepSignatureCache.add(cacheKey);
  }
}

// Seat -> audit public key lookup over a signed genesis player roster.
export function genesisAuditKeyLookup(players) {
  const bySeat = new Map((Array.isArray(players) ? players : []).map((player) => [
    Number(player?.index ?? player?.seat),
    String(player?.auditPublicKey || ""),
  ]));
  return (seat) => bySeat.get(Number(seat)) || "";
}
