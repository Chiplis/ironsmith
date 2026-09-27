// A ciphertext reference names an INPUT to a shuffle. No output position is
// ever associated with an input object, stable ID, or manifest slot.
const clone = (value) => JSON.parse(JSON.stringify(value));
const canonical = (value) => JSON.stringify(value, (_key, entry) =>
  entry && typeof entry === "object" && !Array.isArray(entry)
    ? Object.fromEntries(Object.keys(entry).sort().map(key => [key, entry[key]]))
    : entry);

export const PRIVATE_SHUFFLE_PROTOCOL_VERSION = 15;

export function isPrivateZiffleEpoch(ceremony) {
  return ceremony?.inputDeck != null;
}

export function ziffleInputDeckFields(ceremony) {
  return isPrivateZiffleEpoch(ceremony) ? { inputDeck: clone(ceremony.inputDeck) } : {};
}

export function ziffleEpochNode(ceremony) {
  const node = {
    deckCount: Number(ceremony.deckCount),
    context: String(ceremony.context || ""),
    steps: clone(ceremony.steps || []),
  };
  if (isPrivateZiffleEpoch(ceremony)) node.sources = clone(ceremony.inputDeck.sources);
  return node;
}

export function acceptedZiffleEpochs(match, actions = [], owner, precedingProofs = []) {
  const genesis = (match?.ziffleCeremonies || []).find(entry => Number(entry.owner) === Number(owner));
  if (!genesis || isPrivateZiffleEpoch(genesis)) throw new Error("Missing signed initial encrypted deck");
  const result = [genesis];
  for (const proof of [...actions.flatMap(entry => (entry.audit || entry).shuffleProofs || []), ...precedingProofs]) {
    if (Number(proof.owner) !== Number(owner)) continue;
    if (!isPrivateZiffleEpoch(proof)) throw new Error("Linked legacy shuffles cannot seed a private ciphertext epoch");
    const existing = result.find(entry => entry.context === proof.context);
    if (existing) {
      if (canonical(ziffleEpochNode(existing)) !== canonical(ziffleEpochNode(proof))
        || existing.deckHash !== proof.deckHash || canonical(existing.inputDeck) !== canonical(proof.inputDeck)) {
        throw new Error("Conflicting encrypted shuffle at an accepted context");
      }
      continue;
    }
    assertZiffleEpochHistory(proof, result);
    result.push(proof);
  }
  return result;
}

export function assertZiffleEpochHistory(proof, accepted) {
  const input = proof?.inputDeck;
  if (!input || !accepted?.length) throw new Error("Missing authenticated shuffle input history");
  for (const key of ["beforeOrder", "afterOrder", "before_order", "after_order", "authenticatedOrder"]) {
    if (Object.hasOwn(proof, key)) throw new Error("Private shuffle contains a public object-order mapping");
  }
  if (Number(input.universeCount) !== Number(accepted[0].deckCount)
    || canonical(input.epochs) !== canonical(accepted.map(ziffleEpochNode))) {
    throw new Error("Encrypted shuffle history differs from the accepted signed transcript");
  }
  const consumed = new Set();
  for (let epoch = 1; epoch <= input.epochs.length; epoch++) {
    const sources = epoch === input.epochs.length ? input.sources : input.epochs[epoch].sources;
    const count = epoch === input.epochs.length ? Number(proof.deckCount) : Number(input.epochs[epoch].deckCount);
    if (!Array.isArray(sources) || sources.length !== count) throw new Error("Encrypted shuffle source count mismatch");
    for (const source of sources) {
      const sourceEpoch = source?.epoch;
      const position = source?.position;
      const key = `${sourceEpoch}:${position}`;
      if (!Number.isSafeInteger(sourceEpoch) || sourceEpoch < 0 || sourceEpoch >= epoch
        || !Number.isSafeInteger(position) || position < 0 || position >= Number(input.epochs[sourceEpoch].deckCount)
        || consumed.has(key)) {
        throw new Error("Encrypted shuffle uses an invalid or already consumed ciphertext");
      }
      consumed.add(key);
    }
  }
}

export function buildZiffleInputDeck(accepted, commitments) {
  if (!accepted?.length) throw new Error("Missing accepted ciphertext epochs");
  const hashes = new Map(accepted.map((entry, epoch) => [String(entry.deckHash), epoch]));
  const sources = commitments.map(commitment => {
    const match = /^ziffle:([^:]+):(\d+)$/.exec(String(commitment));
    const epoch = match && hashes.get(match[1]);
    const position = match ? Number(match[2]) : NaN;
    if (epoch == null || !Number.isSafeInteger(position) || position >= Number(accepted[epoch].deckCount)) {
      throw new Error("Live library input is not in an accepted encrypted deck");
    }
    return { epoch, position };
  }).sort((a, b) => a.epoch - b.epoch || a.position - b.position);
  const inputDeck = { universeCount: Number(accepted[0].deckCount), epochs: accepted.map(ziffleEpochNode), sources };
  assertZiffleEpochHistory({ deckCount: sources.length, inputDeck }, accepted);
  return inputDeck;
}

export function ziffleEpochInputCommitments(proof, accepted) {
  assertZiffleEpochHistory(proof, accepted);
  return proof.inputDeck.sources.map(({ epoch, position }) => `ziffle:${accepted[epoch].deckHash}:${position}`);
}

export function assertZiffleEpochInputs(proof, requirement, accepted) {
  const actual = requirement?.inputCommitments ?? requirement?.input_commitments;
  const expected = ziffleEpochInputCommitments(proof, accepted);
  if (!Array.isArray(actual) || actual.length !== Number(proof.deckCount)
    || canonical([...actual].sort()) !== canonical([...expected].sort())) {
    throw new Error("Encrypted shuffle inputs do not match the locally required library");
  }
  return expected;
}

export function assertZiffleEpochVerification(proof, verified, accepted) {
  assertZiffleEpochHistory(proof, accepted);
  if (Number(verified?.deckCount) !== Number(proof.deckCount)
    || String(verified?.deckHash || "") !== String(proof.deckHash || "")
    || String(verified?.rootDeckHash || "") !== String(accepted[0].deckHash)
    || String(verified?.rootContext || "") !== String(accepted[0].context)
    || Number(verified?.universeCount) !== Number(accepted[0].deckCount)) {
    throw new Error("Encrypted shuffle proof is not rooted in this owner's signed initial deck");
  }
}

export function ziffleEpochMaterial(proof, requirement, expectedInputs) {
  const randomCountBefore = Number(requirement?.randomCountBefore ?? requirement?.random_count_before);
  if (!Number.isSafeInteger(randomCountBefore) || randomCountBefore < 0) throw new Error("Shuffle boundary is missing its random counter");
  return { owner: Number(proof.owner), deckHash: String(proof.deckHash), count: Number(proof.deckCount), randomCountBefore, expectedInputs };
}
