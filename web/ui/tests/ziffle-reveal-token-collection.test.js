import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { collectZiffleRevealTokenGroups } from "../src/lib/ziffle-reveal-token-collection.js";

const source = readFileSync(new URL("../src/hooks/peer-lobby/validation.js", import.meta.url), "utf8");
const start = source.indexOf("  async function collectZiffleRevealTokensBatch(");
const end = source.indexOf("  async function buildLiveZiffleShuffleProofs(", start);
assert.ok(start >= 0 && end > start);
const collectorSource = source.slice(start, end);
const token = (player, cardPosition) => ({ player, cardPosition, publicKeyHex: `key-${player}`,
  tokenHex: `token-${player}-${cardPosition}`, proofHex: `proof-${player}-${cardPosition}` });
const gate = () => {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
};

function harness({ cached = [], buildLocal, requestRemote, keys = [{ player: 0 }, { player: 1 }] } = {}) {
  const cache = new Map(cached.map(value => [`${value.player}:${value.cardPosition}`, value]));
  const localBuilds = [], requests = [], waits = [], cacheReads = [];
  const responses = new Map();
  let requestId = 0;
  const context = {
    collectZiffleRevealTokenGroups,
    multiplayerRef: { current: { localPlayerIndex: 0, localPeerId: "peer-0", lastAppliedSequence: 8,
      players: keys.map(key => ({ index: key.player, peerId: `peer-${key.player}` })) } },
    resolveLocalPlayerIndex: session => session.localPlayerIndex,
    reindexPlayers: players => players,
    matchStartPayloadRef: { current: null },
    normalizeZiffleCardPositions: positions => [...new Set(positions)],
    summarizePeerCommand: command => command,
    recordPeerSyncPerf: () => {},
    currentAuditMatchId: () => "match-id",
    auditStateHashRef: { current: "state-hash" },
    INITIAL_AUDIT_STATE_HASH: "initial-hash",
    cloneMultiplayerPayload: structuredClone,
    cachedZiffleRevealTokens: (_ceremony, player, positions) => {
      cacheReads.push({ player, positions: [...positions] });
      const tokens = positions.map(position => cache.get(`${player}:${position}`));
      return tokens.every(Boolean) ? tokens.map(value => ({ ...value })) : null;
    },
    rememberZiffleRevealTokens: (_ceremony, tokens, positions) => {
      for (const value of tokens) {
        const cardPosition = value.cardPosition ?? (positions.length === 1 ? positions[0] : undefined);
        cache.set(`${value.player}:${cardPosition}`, { ...value, cardPosition });
      }
    },
    timePeerSyncPhase: (_name, _details, run) => run(),
    buildLocalZiffleRevealTokens: async (_ceremony, positions) => {
      localBuilds.push([...positions]);
      return buildLocal ? buildLocal(positions) : positions.map(position => token(0, position));
    },
    routePeerIdForPlayer: player => player.peerId,
    setStatus: () => {},
    makeZiffleRequestId: () => `request-${++requestId}`,
    ziffleRevealTokenTimeoutMs: () => 1000,
    zifflePositionsDetail: positions => positions.join(","),
    PROTOCOL_RESPONSE_TIMEOUT_MS: 4000,
    compactZiffleCeremonyForDiagnostics: ceremony => ceremony,
    actionIntentKey: () => "intent-key",
    waitForZiffleRevealToken: id => {
      const response = gate();
      responses.set(id, response);
      return response.promise;
    },
    PROTOCOL_VERSION: 14,
    payloadSizeBytes: payload => JSON.stringify(payload).length,
    sendDirectProtocolMessage: async (_peer, payload) => {
      requests.push(payload);
      const player = Number(_peer.slice("peer-".length));
      const result = requestRemote ? await requestRemote(player, payload) :
        payload.cardPositions.map(position => token(player, position));
      responses.get(payload.requestId).resolve(result);
    },
    waitForProtocolResponse: (promise, details) => { waits.push(details); return promise; },
  };
  const collect = new Function(...Object.keys(context), `${collectorSource}\nreturn collectZiffleRevealTokensBatch;`)(...Object.values(context));
  const ceremony = { owner: 0, deckCount: 60, context: "ceremony", deckHash: "deck-hash", keys };
  return { collect: (positions, options) => collect(ceremony, positions, options),
    localBuilds, requests, waits, cacheReads, ceremony };
}

test("starts remote reveal requests while the local proof build is still pending", async () => {
  const local = gate();
  const { collect, requests } = harness({
    buildLocal: async positions => { await local.promise; return positions.map(position => token(0, position)); },
  });
  const result = collect([2, 9]);
  await Promise.resolve();
  await Promise.resolve();
  const requestsBeforeLocalFinished = requests.length;
  local.resolve();
  await result;
  assert.equal(requestsBeforeLocalFinished, 1, "network exchange must overlap local proof generation");
});

test("requests only each signer's missing positions and merges in key/position order", async () => {
  const keys = [{ player: 2 }, { player: 0 }, { player: 1 }];
  const { collect, localBuilds, requests, cacheReads } = harness({ keys,
    cached: [token(0, 9), token(1, 4), token(2, 9), token(2, 2)],
    requestRemote: async (player, payload) => payload.cardPositions.slice().reverse().map(position => token(player, position)),
  });
  const result = await collect([9, 2, 4]);
  assert.deepEqual(localBuilds, [[2, 4]]);
  assert.deepEqual(requests.map(request => [request.cardPositions, request.requesterIndex]), [[[4], 0], [[9, 2], 0]]);
  assert.deepEqual(result, keys.flatMap(key => [9, 2, 4].map(position => token(key.player, position))));
  assert.ok(cacheReads.some(read => read.positions.length === 1), "reuse individual cached positions");
});

test("fully cached signers require no build or request", async () => {
  const { collect, localBuilds, requests } = harness({ cached: [0, 1].flatMap(player => [2, 9].map(position => token(player, position))) });
  assert.equal((await collect([2, 9])).length, 4);
  assert.deepEqual(localBuilds, []);
  assert.deepEqual(requests, []);
});

test("keeps action authorization and protocol wait metadata on partial requests", async () => {
  const { collect, requests, waits, ceremony } = harness({ cached: [token(1, 2)] });
  const options = { command: { type: "select_objects", object_ids: [12] }, seq: 9, actorIndex: 0,
    requirements: [{ type: "private_open", card: "private-card-name", slot: 55,
      commitment: "private-manifest-entry", id: "private_open:0:library:55:194" }], prevStateHash: "signed-head",
    preActionPublicCheckpointHash: "public-hash", actionIntent: { signature: "intent-signature" },
    actionAudit: { signature: "audit-signature" }, cryptoMaterialRequestId: "material-id" };
  await collect([2, 9], options);
  assert.deepEqual(requests[0].cardPositions, [9]);
  assert.deepEqual(requests[0].ceremony, ceremony);
  assert.equal(requests[0].cryptoMaterialRequestId, "material-id");
  assert.deepEqual(requests[0].actionAuthorization, { matchId: "match-id", seq: 9, requesterIndex: 0,
    actorIndex: 0, prevStateHash: "signed-head", preActionPublicCheckpointHash: "public-hash",
    command: options.command, actionIntent: options.actionIntent,
    actionAudit: options.actionAudit });
  assert.doesNotMatch(JSON.stringify(requests[0]), /private-card-name|private-manifest-entry|private_open:0:library:55/);
  assert.equal(waits[0].requestPayload, requests[0]);
  assert.equal(waits[0].basisSequence, 8);
});

test("accepts legacy scalar tokens for a single missing position", async () => {
  const { collect, requests } = harness({ cached: [token(1, 2)],
    requestRemote: async player => {
      const response = token(player, 9);
      delete response.cardPosition;
      return response;
    },
  });
  assert.deepEqual(await collect([2, 9]), [0, 1].flatMap(player => [2, 9].map(position => token(player, position))));
  assert.deepEqual(requests[0].cardPositions, [9]);
});

for (const [label, response] of [
  ["missing", [token(1, 2)]],
  ["wrong signer", [token(0, 2), token(0, 9)]],
  ["wrong position", [token(1, 2), token(1, 10)]],
  ["duplicate", [token(1, 2), token(1, 2)]],
  ["scalar for multiple positions", token(1, 2)],
  ["null", null],
]) {
  test(`rejects ${label} response tokens instead of returning an incomplete group`, async () => {
    const { collect } = harness({ requestRemote: async () => response });
    await assert.rejects(collect([2, 9]), /ziffle reveal tokens|Ziffle reveal tokens/);
  });
}
