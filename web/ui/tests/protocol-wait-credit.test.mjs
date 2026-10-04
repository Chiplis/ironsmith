import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import {
  canonicalJson, createAuditSessionKey, sha256Hex, signAuditPayload, verifyAuditPayload,
  CURRENT_AUDIT_PROTOCOL_VERSION, PROTOCOL_RESPONSE_TIMEOUT_MS,
} from "../src/lib/multiplayer-audit.js";
import { MAX_ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS } from "../src/lib/ziffle-timeouts.js";

// Exercise the production lifecycle with real hashes/signatures and fake
// transport/time, without a peer connection or game engine.
const source = readFileSync(new URL("../src/hooks/peer-lobby/connections.js", import.meta.url), "utf8");
const start = source.indexOf('  const PROTOCOL_WAIT_NOTICE_DOMAIN =');
const end = source.indexOf('  function rememberActionIntentObservation(', start);
assert.ok(start >= 0 && end > start);
function implementation(name) {
  const from = source.indexOf(`  function ${name}(`);
  const to = source.indexOf("\n  }\n", from);
  assert.ok(from >= 0 && to > from, `${name} is present`);
  return source.slice(from, to + 4);
}
const keys = await Promise.all([0, 1, 2].map(() => createAuditSessionKey()));
const PROTOCOL_VERSION = CURRENT_AUDIT_PROTOCOL_VERSION;
const requestTypes = [
  "crypto_material_request", "ziffle_reveal_token_request", "ziffle_shuffle_step_request",
  "rng_commit_request", "rng_reveal_request", "action_quorum_vote_request",
];
function harness(localPlayerIndex = 2) {
  let now = 1000;
  let timerId = 0;
  const timers = new Map(), sent = [], handled = [], announcements = [];
  const players = keys.map((_, index) => ({ index, peerId: `peer-${index}` }));
  const context = {
    Date: { now: () => now }, nowMonotonicMs: () => now,
    window: {
      setTimeout: (fn) => { const id = ++timerId; timers.set(id, fn); return id; },
      clearTimeout: (id) => timers.delete(id),
    },
    PROTOCOL_VERSION, PROTOCOL_RESPONSE_TIMEOUT_MS, MAX_ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS,
    multiplayerRef: { current: { matchStarted: true, lastAppliedSequence: 7,
      localPlayerIndex, localPeerId: `peer-${localPlayerIndex}`, players } },
    matchStartPayloadRef: { current: { players } },
    protocolWaitObservationsRef: { current: new Map() }, servicesRef: { current: {} },
    normalizePlayerIndex: (value) => value == null ? null : Number(value),
    resolveLocalPlayerIndex: (session) => session.localPlayerIndex,
    currentAuditMatchId: () => "match", reindexPlayers: (value) => value,
    canonicalMultiplayerPayload: canonicalJson, cloneMultiplayerPayload: structuredClone,
    payloadSizeBytes: (value) => Buffer.byteLength(canonicalJson(value)),
    sha256Hex, signAuditPayload, verifyAuditPayload,
    importCachedAuditPublicKey: async (key) => key,
    publicKeyForAuditSigner: (index) => keys[index].publicKey,
    ensureAuditIdentity: async () => ({ keyPair: keys[localPlayerIndex] }),
    routePeerIdForPlayer: (player) => player.peerId,
    sendDirectPeerMessage: (peerId, message) => { sent.push({ peerId, message }); return true; },
  };
  vm.createContext(context);
  vm.runInContext(source.slice(start, end) + ["openProtocolWaitsForRequester", "observedProtocolWaitMs"]
    .map(implementation).join("\n"), context);
  const announce = context.announceProtocolResponse;
  context.announceProtocolResponse = (...args) => {
    const pending = announce(...args);
    announcements.push(pending);
    return pending;
  };
  for (const name of ["CryptoMaterial", "ZiffleRevealToken", "ZiffleShuffleStep", "RngCommit", "RngReveal", "ActionQuorumVote"]) {
    context.servicesRef.current[`answer${name}Request`] = async (conn, request) => {
      handled.push({ name, request });
      context.protocolResponseConn(conn, request).send({
        type: request.type.replace(/_request$/, "_response"), requestId: request.requestId,
      });
    };
  }
  context.servicesRef.current.playerIndexForPeerId = (peer) => players.find((p) => p.peerId === peer)?.index;
  return { context, sent, handled, timers, entries: context.protocolWaitObservationsRef.current,
    credit: () => context.observedProtocolWaitMs(0, 1000),
    open: () => context.openProtocolWaitsForRequester(0).length,
    advance: (ms) => { now += ms; },
    runTimers: async () => {
      for (const [id, fn] of timers) { timers.delete(id); fn(); }
      await Promise.all(announcements);
    },
  };
}
function request(type = "rng_commit_request", extra = {}) {
  return { type, protocolVersion: PROTOCOL_VERSION, requestId: "request-1", ...extra };
}
async function noticeMessage(payload = request(), changes = {}, { omitPayload = false } = {}) {
  const notice = {
    domain: "ironsmith-protocol-wait-notice-v1", matchId: "match", basisSequence: 7,
    requester: 0, target: 1, requestType: payload.type, requestId: payload.requestId,
    requestPayloadHash: await sha256Hex(canonicalJson(payload)), responseTimeoutMs: 4000, ...changes,
  };
  return { notice, signature: await signAuditPayload(keys[notice.requester], notice),
    ...(omitPayload ? {} : { requestPayload: payload }) };
}
async function answerMessage(changes = {}) {
  const answer = {
    domain: "ironsmith-protocol-wait-answer-v1", matchId: "match", requester: 0, responder: 1,
    requestType: "rng_commit_request", requestId: "request-1", status: "answered",
    responseHash: await sha256Hex("response"), ...changes,
  };
  return { answer, signature: await signAuditPayload(keys[answer.responder], answer) };
}
for (const type of requestTypes) {
  test(`supported ${type} keeps bounded credit and fallback delivery`, async () => {
    const h = harness(1);
    // Production shuffle payloads lack both top-level identity fields.
    const extra = type === "ziffle_shuffle_step_request" ? { request: {} }
      : { requesterIndex: 0, ...(type.startsWith("rng_") ? { matchId: "match" } : {}) };
    await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request(type, extra)));
    h.advance(1000);
    assert.equal(h.credit(), 1000);
    assert.equal(h.open(), 1);
    await h.runTimers();
    assert.equal(h.handled.length, 1);
    assert.equal(h.open(), 0);
    assert.ok(h.sent.some(({ message }) => message.answer?.requestType === type));
    h.advance(1000);
    assert.equal(h.credit(), 1000, "the response closes the credited interval");
  });
}
for (const type of ["unsupported_request", "constructor", "toString", "__proto__"]) {
  test(`unsupported ${type} receives no credit, forwarding, or dispatch`, async () => {
    for (const localIndex of [1, 2]) {
      const h = harness(localIndex);
      await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request(type)));
      await h.context.handleProtocolWaitAnswerMessage(await answerMessage({ requestType: type }));
      h.advance(1000);
      assert.equal(h.credit(), 0);
      assert.equal(h.open(), 0);
      assert.equal(h.entries.size, 0);
      assert.equal(h.sent.length, 0);
      assert.equal(h.timers.size, 0);
    }
  });
}
for (const [name, modify] of [
  ["wrong protocol version", (m) => { m.requestPayload.protocolVersion--; }],
  ["mismatched requester", (m) => { m.requestPayload.requesterIndex = 2; }],
  ["mismatched match", (m) => { m.requestPayload.matchId = "different-match"; }],
  ["mismatched request type", (m) => { m.requestPayload.type = "rng_reveal_request"; }],
  ["mismatched request ID", (m) => { m.requestPayload.requestId = "other-request"; }],
  ["missing protocol version", (m) => { delete m.requestPayload.protocolVersion; }],
  ["null payload", (m) => { m.requestPayload = null; }],
  ["array payload", (m) => { m.requestPayload = []; }],
  ["oversize payload", (m) => { m.requestPayload.padding = " ".repeat(256 * 1024); }],
]) {
  test(`${name} cannot establish a credited wait with a matching signed hash`, async () => {
    const h = harness(), message = await noticeMessage();
    modify(message);
    message.notice.requestPayloadHash = await sha256Hex(canonicalJson(message.requestPayload));
    message.signature = await signAuditPayload(keys[0], message.notice);
    await h.context.handleProtocolWaitNoticeMessage(message);
    h.advance(1000);
    assert.equal(h.credit(), 0);
    assert.equal(h.open(), 0);
    assert.equal(h.entries.size, 0);
    assert.equal(h.sent.length, 0);
  });
}
test("bad hashes and signatures do not establish waits", async () => {
  const h = harness();
  await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request(), { requestPayloadHash: "bad-hash" }));
  const message = await noticeMessage();
  message.signature = await signAuditPayload(keys[1], message.notice);
  await assert.rejects(h.context.handleProtocolWaitNoticeMessage(message), /signature is invalid/);
  assert.equal(h.entries.size, 0);
});
test("duplicate notices cannot restart or rebind an observed request", async () => {
  const h = harness(), message = await noticeMessage();
  await h.context.handleProtocolWaitNoticeMessage(message);
  h.advance(1000);
  await h.context.handleProtocolWaitNoticeMessage(message);
  assert.equal(h.entries.get("0:request-1").observedAtMonoMs, 1000);
  for (const changes of [{ target: 2 }, { basisSequence: 8 }, { responseTimeoutMs: 8000 }]) {
    await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request(), changes));
  }
  h.advance(5000);
  assert.equal(h.credit(), 4000);
  assert.equal(h.open(), 0);
  assert.equal(h.sent.length, 1, "the valid notice is only forwarded once");
});
test("conflicting duplicates cannot attach a payload to a payload-less observation", async () => {
  const h = harness();
  await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request(), {}, { omitPayload: true }));
  await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request("rng_reveal_request")));
  h.advance(1000);
  assert.equal(h.credit(), 0);
  assert.equal(h.open(), 0);
  assert.equal(h.entries.get("0:request-1").requestPayload, null);
});
test("missing payload earns no credit until its matching target answer is observed", async () => {
  const h = harness();
  await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request(), {}, { omitPayload: true }));
  h.advance(1000);
  assert.equal(h.credit(), 0);
  assert.equal(h.open(), 0);
  for (const changes of [{ responder: 2 }, { requestType: "rng_reveal_request" }, { status: "unrecognized" }]) {
    await h.context.handleProtocolWaitAnswerMessage(await answerMessage(changes));
    assert.equal(h.credit(), 0);
  }
  await h.context.handleProtocolWaitAnswerMessage(await answerMessage());
  assert.equal(h.credit(), 1000);
  h.advance(1000);
  assert.equal(h.credit(), 1000);
});
test("early answers bind to both the target and request type", async () => {
  for (const changes of [{}, { responder: 2 }, { requestType: "rng_reveal_request" }]) {
    const h = harness();
    await h.context.handleProtocolWaitAnswerMessage(await answerMessage(changes));
    await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request(), {}, { omitPayload: true }));
    assert.equal(Boolean(h.entries.get("0:request-1").answerStatus), Object.keys(changes).length === 0);
    h.advance(1000);
    assert.equal(h.credit(), 0);
  }
});
test("a locally announced early response stays bound when its notice arrives", async () => {
  const h = harness(1);
  await h.context.announceProtocolResponse(0, request(), { type: "rng_commit_response", requestId: "request-1" });
  await h.context.handleProtocolWaitNoticeMessage(await noticeMessage());
  assert.equal(h.entries.get("0:request-1").answerStatus, "answered");
  assert.equal(h.timers.size, 0);
});
test("overlapping waits are unioned and declared timeouts remain capped", async () => {
  const h = harness();
  await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request(), { responseTimeoutMs: 1e9 }));
  h.advance(1000);
  await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request("rng_commit_request", { requestId: "request-2" })));
  h.advance(1000);
  assert.equal(h.credit(), 2000);
  h.advance(1e9);
  assert.equal(h.credit(), MAX_ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS);
  assert.equal(h.open(), 0);
});
test("local waits use the same request validation and hash binding", async () => {
  const h = harness(0);
  const claim = { requesterIndex: 0, targetPlayerIndex: 1, requestId: "request-1", basisSequence: 7,
    requestPayload: request(), responseTimeoutMs: 4000 };
  assert.equal(await h.context.openLocalProtocolWait({ ...claim, requestPayload: request("unsupported_request") }), null);
  assert.equal(await h.context.openLocalProtocolWait({ ...claim, requestPayloadHash: "bad-hash" }), null);
  assert.equal(h.entries.size, 0);
  await h.context.openLocalProtocolWait(claim);
  h.advance(1000);
  assert.equal(h.credit(), 1000);
});

for (const failValidation of [false, true]) {
  test(`the real quorum answerer announces ${failValidation ? "error" : "success"} and closes its wait`, async () => {
    const h = harness(1), responses = [];
    const quorumSource = readFileSync(new URL("../src/hooks/peer-lobby/crypto-resync.js", import.meta.url), "utf8");
    const from = quorumSource.indexOf("  async function answerActionQuorumVoteRequest(");
    const to = quorumSource.indexOf("\n  function actionHistoryEntryForSequence(", from);
    assert.ok(from >= 0 && to > from);
    Object.assign(h.context, {
      summarizeSequencedActionForPerf: () => ({}), recordPeerSyncPerf() {},
      playerIndexForPeerId: () => 0,
      timePeerSyncPhase: async (_name, _details, run) => run(),
      applySequencedActionMessage: async () => { if (failValidation) throw Error("invalid action"); },
      refreshPendingActionIntentEvidenceForAction: async () => {},
      signActionQuorumVoteForMessage: async () => ({ voter: 1 }),
      safeSend: (conn, payload) => conn.send(payload),
      toErrorMessage: (error) => error.message, isRejectedActionCheatReason: () => false,
    });
    h.context.servicesRef.current.protocolResponseConn = h.context.protocolResponseConn;
    vm.runInContext(quorumSource.slice(from, to), h.context);
    const payload = request("action_quorum_vote_request", { requesterIndex: 0,
      action: { type: "apply_action", seq: 8, audit: { matchId: "match" } } });
    await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(payload));
    h.advance(1000);
    await h.context.answerActionQuorumVoteRequest({ peer: "peer-0", send: (response) => responses.push(response) }, payload);
    await h.runTimers();
    assert.equal(responses.length, 1);
    assert.equal(h.open(), 0);
    assert.equal(h.credit(), 1000);
    assert.ok(h.sent.some(({ message }) => message.answer?.status === (failValidation ? "error" : "answered")));
  });
}

test("a locally announced mismatched answer cannot substantiate another wait", async () => {
  const h = harness(1);
  await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request(), {}, { omitPayload: true }));
  h.advance(1000);
  await h.context.announceProtocolResponse(0, request("rng_reveal_request"), { type: "rng_reveal_response" });
  assert.equal(h.credit(), 0);
  assert.equal(h.entries.get("0:request-1").answerStatus, "");
  assert.ok(h.sent.some(({ message }) => message.answer?.requestType === "rng_reveal_request"));
});

test("unsafe timing integers cannot establish a wait", async () => {
  for (const changes of [{ basisSequence: Number.MAX_SAFE_INTEGER + 1 }, { responseTimeoutMs: Number.MAX_SAFE_INTEGER + 1 }]) {
    const h = harness();
    await h.context.handleProtocolWaitNoticeMessage(await noticeMessage(request(), changes));
    assert.equal(h.entries.size, 0);
  }
});

test("oversize local waits require a matching answer before receiving credit", async () => {
  const h = harness(0);
  await h.context.openLocalProtocolWait({ requesterIndex: 0, targetPlayerIndex: 1,
    requestId: "request-1", requestPayload: request("rng_commit_request", { padding: " ".repeat(256 * 1024) }) });
  assert.ok(h.sent.every(({ message }) => !message.requestPayload));
  h.advance(1000);
  assert.equal(h.credit(), 0);
  assert.equal(h.open(), 0);
  await h.context.handleProtocolWaitAnswerMessage(await answerMessage());
  assert.equal(h.credit(), 1000);
});
