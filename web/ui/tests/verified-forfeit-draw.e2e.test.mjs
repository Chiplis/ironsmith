// Real engine, real proofs: a three-seat Verified match keeps drawing after a
// seat disconnects for good and is forfeited. It runs as a tournament room on
// the local relay, so no seat depends on WebRTC between browser tabs.
//
// Slow by nature: every action carries real proofs for three seats, and the
// engine asks each hidden hand about every event, so reaching the first draw
// after the forfeit takes about 45 actions (10 to 30 minutes). Run it alone:
//   E2E_PROGRESS=1 node --test tests/verified-forfeit-draw.e2e.test.mjs
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { webcrypto } from 'node:crypto';
import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { chromium } from 'playwright';
import { startRelay } from '../../relay/tests/runtime.mjs';
import { createAuditSessionKey, exportAuditPublicKey, randomAuditHex, signAuditPayload } from '../src/lib/multiplayer-audit.js';
import {
  encodeInviteCode, invitePayload, tournamentDescriptor, tournamentIdForDescriptor, witnessPublicKeyFromPrivateJwk,
} from '../src/lib/tournament/witness-protocol.js';
import { openFullUiPage, fullUiSnapshot, waitForFullUiSnapshot, sleep } from './peerjs-resync-harness.js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const PORT = 5199;
const NAMES = ['Alice', 'Bob', 'Cara'];

async function organizerInvites(names) {
  const keyPair = await createAuditSessionKey();
  const tournament = tournamentDescriptor({
    name: 'Friday Commander', organizerPublicKey: await exportAuditPublicKey(keyPair), createdAt: Date.now(), nonce: randomAuditHex(16),
  });
  const codes = [];
  for (const playerName of names) {
    const payload = invitePayload({ tournament, inviteId: randomAuditHex(16), playerName, expiresAt: Date.now() + 86400000 });
    codes.push(encodeInviteCode({ payload, signature: await signAuditPayload(keyPair, payload) }));
  }
  return { tournamentId: await tournamentIdForDescriptor(tournament), codes };
}

const DECK = { deckText: '99 Plains', commanderText: '1 Isamaru, Hound of Konda' };
const ACTION_ORDER = ['keep_opening_hand', 'play_land', 'pass_priority'];

const seatState = (snap, seat) => snap.state.players[seat];

// Submits the next routine action for whichever seat must decide. Priority
// menus fill in asynchronously and a page can lag an action behind, so anything
// not ready yet is simply tried again on the next pass.
let lastDriverNote = '';
const submitted = new Map();
async function advance(pages) {
  const snaps = await Promise.all(pages.map(fullUiSnapshot));
  const head = Math.max(...snaps.map(snap => snap?.multiplayer?.lastAppliedSequence ?? 0));
  for (const [index, snap] of snaps.entries()) {
    const multiplayer = snap?.multiplayer;
    if (!snap?.state?.decision || multiplayer.lastAppliedSequence !== head) continue;
    if (multiplayer.submittingAction || multiplayer.pendingVerification > 0) return false;
    if (Number(snap.state.decision.player) !== Number(multiplayer.localPlayerIndex)) continue;
    const actions = snap.state.decisionActions || [];
    let command = null;
    if (snap.state.decision.kind === 'select_objects') {
      // Optional prompts are declined; a forced one (discarding to hand size)
      // takes the first legal cards.
      const detail = snap.state.decisionDetail || {};
      const needed = /^You may/i.test(snap.state.decision.description || '') ? 0 : Number(detail.min ?? detail.min_count ?? 1);
      command = { type: 'select_objects',
        object_ids: (snap.state.decisionCandidates || []).filter(entry => entry.legal !== false).slice(0, needed).map(entry => entry.id) };
    }
    const action = ACTION_ORDER.map(kind => actions.find(entry => entry.action_ref?.kind === kind)).find(Boolean)
      || (actions.length === 1 ? actions[0] : null);
    if (action) command = { type: 'priority_action', action_ref: action.action_ref };
    // One answer per decision: a slow machine must not make this driver send a
    // second, different command for the same action number.
    const slot = `${multiplayer.localPlayerIndex}:${head}`;
    const earlier = submitted.get(slot);
    if (earlier && Date.now() - earlier.at < 90000) return false;
    if (earlier) command = earlier.command;
    if (command) submitted.set(slot, { command, at: Date.now() });
    if (!command) {
      lastDriverNote = `seat ${multiplayer.localPlayerIndex} has no routine action: ${JSON.stringify({
        decision: snap.state.decision, actions, candidates: snap.state.decisionCandidates })}`;
      return false;
    }
    try {
      await pages[index].evaluate(command => window.__ironsmithE2E.submitMultiplayerCommand(command), command);
    } catch (error) {
      lastDriverNote = `seat ${multiplayer.localPlayerIndex} ${JSON.stringify(command).slice(0, 200)}: ${String(error?.message || error).slice(0, 400)}`;
      return false;
    }
    return true;
  }
  return false;
}

async function playUntil(pages, predicate, label, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  let last = null;
  let reported = -1;
  while (Date.now() < deadline) {
    last = await Promise.all(pages.map(fullUiSnapshot));
    const head = Math.max(...last.map(snap => snap?.multiplayer?.lastAppliedSequence ?? 0));
    if (process.env.E2E_PROGRESS && head !== reported) {
      reported = head;
      const view = last.find(snap => snap?.multiplayer?.lastAppliedSequence === head)?.state;
      console.error(`[${new Date().toISOString().slice(11, 19)}] ${label}: action ${head} turn ${view?.turn_number} `
        + `${view?.phase}/${view?.step} active ${view?.active_player} decides ${view?.decision?.player} `
        + JSON.stringify(view?.players?.map(player => [player.hand_size, player.library_size, player.battlefield.length])));
    }
    if (last.every(Boolean) && predicate(last)) return last;
    if (!await advance(pages)) await sleep(300);
  }
  assert.fail(`${label}\n${lastDriverNote}\n${JSON.stringify(last?.map(snap => ({
    seat: snap?.multiplayer?.localPlayerIndex, seq: snap?.multiplayer?.lastAppliedSequence,
    status: snap?.status, decision: snap?.state?.decision, turn: snap?.state?.turn_number,
    submitting: snap?.multiplayer?.submittingAction, pending: snap?.multiplayer?.pendingVerification,
    actions: (snap?.state?.decisionActions || []).map(entry => entry.action_ref?.kind),
    players: snap?.state?.players?.map(player => [player.hand_size, player.library_size]),
  })), null, 2)}\n${pages.map(page => [...(page.__peerHarnessErrors || []).slice(-6),
    ...(page.__peerHarnessConsole || []).filter(line => /error|warn|fail|timed? ?out|reject|mismatch|invalid|missing|cannot/i.test(line)).slice(-25)]
    .map(line => line.slice(0, 600)).join('\n')).join('\n---\n')}`);
}

test('a three-seat Verified match keeps drawing after a seat is forfeited', { timeout: 3000000 }, async t => {
  const base = `http://127.0.0.1:${PORT}`;
  const { privateKey } = await webcrypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign', 'verify']);
  const jwk = await webcrypto.subtle.exportKey('jwk', privateKey);
  const relay = await startRelay(base, { WITNESS_SIGNING_KEY: JSON.stringify(jwk) });
  t.after(() => relay.dispose());
  const relayUrl = String(await relay.ready).replace(/\/$/, '');
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    define: {
      'import.meta.env.VITE_LOBBY_RELAY_URL': JSON.stringify(relayUrl),
      'import.meta.env.VITE_WITNESS_PUBLIC_KEYS': JSON.stringify(await witnessPublicKeyFromPrivateJwk(jwk)),
      'import.meta.env.VITE_E2E_TEST': JSON.stringify('true'),
    },
    server: { host: '127.0.0.1', port: PORT, strictPort: true, hmr: false, watch: null } });
  await server.listen();
  t.after(() => server.close());
  const browser = await chromium.launch();
  t.after(() => browser.close());

  const { tournamentId, codes } = await organizerInvites(NAMES);
  const pages = [];
  for (const [index, name] of NAMES.entries()) {
    const context = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
    // Relay-only keeps every seat on the room socket.
    await context.addInitScript(() => localStorage.setItem('ironsmith-relay-only-v1', 'true'));
    const page = await openFullUiPage(context, `${base}/`, name);
    await page.waitForFunction(() => Boolean(window.__ironsmithE2E?.createLobby), null, { timeout: 120000 });
    await waitForFullUiSnapshot(page, snap => !snap.loading && !snap.wasmError, `${name} engine ready`, 180000);
    await page.evaluate(code => window.__ironsmithE2E.redeemTournamentInvite(code), codes[index]);
    pages.push(page);
  }
  const [host, second, third] = pages;

  await host.evaluate(([tournamentId, deck]) => window.__ironsmithE2E.createLobby({
    name: 'ignored', transport: 'websocket', format: 'commander', desiredPlayers: 3, tournamentId, ...deck,
  }), [tournamentId, DECK]);
  const lobby = await waitForFullUiSnapshot(host, snap => snap.multiplayer.mode === 'lobby' && snap.multiplayer.lobbyId,
    'host opens a three-seat Verified lobby', 60000);
  assert.equal(lobby.multiplayer.securityMode, 'verified');
  for (const [index, page] of [second, third].entries()) {
    await page.evaluate(([lobbyId, deck]) => window.__ironsmithE2E.joinLobby({ name: 'ignored', lobbyId, ...deck }),
      [lobby.multiplayer.lobbyId, DECK]);
    await waitForFullUiSnapshot(host, snap => snap.multiplayer.players.length === index + 2, `seat ${index + 2} joins`, 60000);
  }
  await waitForFullUiSnapshot(host, snap => snap.canStartHostedMatch, 'every seat is ready', 120000);
  await host.evaluate(() => window.__ironsmithE2E.startHostedMatch());
  for (const page of pages) {
    await waitForFullUiSnapshot(page, snap => snap.multiplayer.matchStarted && Boolean(snap.state?.decision), 'match starts', 180000);
  }

  // The genesis every seat accepted carries one key-share deal per seat.
  const genesis = (await host.evaluate(() => window.__ironsmithE2E.auditTranscript())).match;
  assert.deepEqual(genesis.ziffleKeyShares.map(deal => [deal.dealer, deal.threshold, deal.shares.length]),
    [[0, 2, 2], [1, 2, 2], [2, 2, 2]]);

  // Every seat keeps its hand and the first turn gets under way. Real proofs
  // across three browser tabs take several seconds per action.
  const started = await playUntil(pages, snaps => snaps.every(snap => snap.multiplayer.lastAppliedSequence >= 8),
    'the match gets under way with every seat present', 900000);
  assert.ok(started.every(snap => snap.state.players.every(player => Number(player.hand_size) >= 7)));

  // The third seat disappears for good and the others forfeit it.
  await third.context().close();
  const remaining = [host, second];
  const forfeitIn = page => page.evaluate(async () =>
    (await window.__ironsmithE2E.auditTranscript())?.actions?.find(action => action.command?.type === 'forfeit_player') || null);
  let forfeit = null;
  for (const deadline = Date.now() + 480000; Date.now() < deadline && !forfeit;) {
    const seen = await Promise.all(remaining.map(forfeitIn));
    if (seen.every(Boolean)) forfeit = seen[0];
    else if (!await advance(remaining)) await sleep(500);
  }
  assert.equal(forfeit?.command.player, 2, 'the silent seat was forfeited in the accepted transcript');
  if (process.env.E2E_PROGRESS) console.error(`[${new Date().toISOString().slice(11, 19)}] forfeit accepted as action ${forfeit.seq}`);

  // The two remaining seats keep playing until one of them draws from its
  // hidden library. Opening that card needs a reveal token from every key,
  // the forfeited seat's included.
  const libraries = (await fullUiSnapshot(host)).state.players.slice(0, 2).map(player => Number(player.library_size));
  const lands = (await fullUiSnapshot(host)).state.players.slice(0, 2).map(player => player.battlefield.length);
  // A card leaving a hidden zone after the forfeit: drawn from a library, or
  // played from a hand (which opens it to the whole table).
  const opened = snap => [0, 1].find(seat => Number(seatState(snap, seat).library_size) < libraries[seat]
    || seatState(snap, seat).battlefield.length > lands[seat]);
  const after = await playUntil(remaining, snaps => snaps.every(snap => opened(snap) !== undefined),
    'a remaining seat opens a hidden card after the forfeit', 1500000);
  const seat = opened(after[0]);
  for (const snap of after) {
    assert.ok(seatState(snap, seat).battlefield.every(card => card.name === 'Plains'),
      `every seat sees what seat ${seat} played: ${JSON.stringify(seatState(snap, seat).battlefield)}`);
  }
  const hand = seatState(after[seat], seat).hand_cards;
  assert.ok(hand.length === Number(seatState(after[seat], seat).hand_size) && hand.every(card => card.name === 'Plains'),
    `seat ${seat} sees its own hand: ${JSON.stringify(hand)}`);
  if (process.env.E2E_PROGRESS) console.error(`[${new Date().toISOString().slice(11, 19)}] seat ${seat} opened a hidden card after the forfeit`);
  const [a, b] = await Promise.all(remaining.map(page => page.evaluate(() => window.__ironsmithE2E.auditTranscript())));
  assert.equal(a.finalStateHash, b.finalStateHash);
  assert.ok(a.actions.at(-1).seq > forfeit.seq + 2, 'play continued after the forfeit');
  for (const page of remaining) {
    assert.deepEqual((page.__peerHarnessErrors || []).filter(error => /ziffle|key share|reveal/i.test(error)), []);
  }
});
