import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { chromium } from "playwright";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../..");
const auditModulePath = resolve(repoRoot, "web/ui/src/lib/multiplayer-audit.js");

const MATCH_ID_PREFIX = "browser-p2p-audit";
const INITIAL_AUDIT_STATE_HASH = "0".repeat(64);
const PROTOCOL_VERSION = 11;

function playerDeck(seat) {
  return [
    `Island ${seat + 1}`,
    `Mountain ${seat + 1}`,
    `Forest ${seat + 1}`,
    `Plains ${seat + 1}`,
    `Swamp ${seat + 1}`,
    `Lightning Bolt ${seat + 1}`,
    `Counterspell ${seat + 1}`,
  ];
}

async function startStaticServer() {
  const server = createServer((_req, res) => {
    res.writeHead(200, {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-store",
    });
    res.end("<!doctype html><title>Ironsmith P2P Browser Scenario</title><body></body>");
  });
  await new Promise((resolveListen) => server.listen(0, "127.0.0.1", resolveListen));
  const { port } = server.address();
  return {
    url: `http://127.0.0.1:${port}/`,
    close: () => new Promise((resolveClose) => server.close(resolveClose)),
  };
}

async function loadAuditBrowserBundle() {
  const source = await readFile(auditModulePath, "utf8");
  return `
${source.replace(/^export\s+/gm, "")}
window.IronsmithAudit = {
  auditStateHash,
  buildSignedMatchGenesis,
  buildDeckSlotOpening,
  buildPrivateDeckManifest,
  buildSignedActionEnvelope,
  buildSignedActionQuorumVote,
  buildSignedPlayerGenesis,
  buildSignedResyncEnvelope,
  checkpointHash,
  canonicalJson,
  createAuditEncryptionKey,
  createAuditSessionKey,
  CURRENT_AUDIT_PROTOCOL_VERSION,
  exportAuditEncryptionPublicKey,
  exportAuditPublicKey,
  importAuditPublicKey,
  matchGenesisPayload,
  publicDeckManifest,
  publicPlayerGenesisRecord,
  randomAuditHex,
  sha256Hex,
  transcriptActionsHash,
  verifyAuditPayload,
  verifyActionQuorumCertificate,
  verifyCardOpeningAgainstManifest,
  verifyLiveAuditTranscript,
  verifySignedMatchGenesis,
  verifySignedPlayerGenesis,
  verifySignedResyncEnvelope,
};
`;
}

function clientHarnessSource() {
  return `
window.installIronsmithP2PClient = async function installIronsmithP2PClient(config) {
  const audit = window.IronsmithAudit;
  const seat = Number(config.seat);
  const playerCount = Number(config.playerCount);
  const matchId = String(config.matchId);
  const deck = config.deck || [];
  const keyPair = await audit.createAuditSessionKey();
  const encryptionKeyPair = await audit.createAuditEncryptionKey();
  const auditPublicKey = await audit.exportAuditPublicKey(keyPair);
  const auditEncryptionPublicKey = await audit.exportAuditEncryptionPublicKey(encryptionKeyPair);
  const privateManifest = await audit.buildPrivateDeckManifest({
    matchId,
    owner: seat,
    deck,
    saltForSlot: async (slot, card) => \`\${matchId}:seat:\${seat}:slot:\${slot}:card:\${card}:salt\`,
  });
  const publicManifest = audit.publicDeckManifest(privateManifest);
  const ziffleKey = {
    player: seat,
    publicKeyHex: \`browser-ziffle-key-\${seat}\`,
    ownershipProofHex: \`browser-ziffle-proof-\${seat}\`,
  };
  const publicPlayer = {
    seat,
    index: seat,
    peerId: \`browser-peer-\${seat}\`,
    name: \`Player \${seat + 1}\`,
    auditPublicKey,
    auditEncryptionPublicKey,
    deckAuditManifest: publicManifest,
    ziffleKey,
    deckCount: publicManifest.deckCount,
    sideboardCount: publicManifest.sideboardCount,
    commanderCount: publicManifest.commanderCount,
  };
  publicPlayer.playerGenesisSignature = await audit.buildSignedPlayerGenesis({
    keyPair,
    matchId,
    protocolVersion: ${PROTOCOL_VERSION},
    timeoutMs: Number(config.timeoutMs || 300000),
    player: publicPlayer,
  });
  const state = {
    seat,
    matchId,
    connected: true,
    timeoutMs: Number(config.timeoutMs || 300000),
    clockGraceMs: Number(config.clockGraceMs || 5),
    remainingMsByPlayer: Array.from(
      { length: playerCount },
      () => Number(config.timeoutMs || 300000)
    ),
    clockHash: "${INITIAL_AUDIT_STATE_HASH}",
    timerStartedAt: Date.now(),
    warnings: [],
    players: [],
    publicDecks: [],
    stateHash: "${INITIAL_AUDIT_STATE_HASH}",
    lastSeq: 0,
    expectedActor: 0,
    transcript: [],
    rejected: [],
    duplicates: [],
    matchPayload: null,
  };

  async function verifyOpening(opening) {
    const owner = Number(opening?.owner);
    const manifest = state.publicDecks[owner];
    if (!manifest) {
      throw new Error(\`Missing public deck manifest for player \${owner}\`);
    }
    const ok = await audit.verifyCardOpeningAgainstManifest({
      manifest,
      slot: Number(opening.slot),
      card: opening.card,
      salt: opening.salt,
    });
    if (!ok) {
      throw new Error(\`Card opening for player \${owner} slot \${opening?.slot} does not match committed library\`);
    }
  }

  async function rngCommitmentForNonce(nonceHex) {
    return audit.sha256Hex(audit.canonicalJson({
      domain: "ironsmith-rng-commit-v1",
      nonceHex: String(nonceHex || ""),
    }));
  }

  async function verifyRngReveal(rngReveal, seq) {
    for (const reveal of rngReveal?.reveals || []) {
      const expected = (rngReveal.commits || []).find((commit) => Number(commit.player) === Number(reveal.player));
      const actual = await rngCommitmentForNonce(reveal.nonceHex);
      if (!expected || actual !== String(expected.commitmentHex || "") || actual !== String(reveal.commitmentHex || "")) {
        throw new Error("Transcripted fair-random reveal does not match commitment");
      }
    }
    const combinedSeedHex = await audit.sha256Hex(audit.canonicalJson({
      domain: "ironsmith-combined-rng-v1",
      matchId,
      seq: Number(seq),
      requirementId: String(rngReveal.requirementId || ""),
      commits: rngReveal.commits || [],
      reveals: rngReveal.reveals || [],
    }));
    if (combinedSeedHex !== String(rngReveal.combinedSeedHex || "")) {
      throw new Error("Transcripted fair-random combined seed is invalid");
    }
  }

  async function matchClockHash(clock) {
    const payload = { ...clock };
    delete payload.clockHash;
    return audit.sha256Hex(audit.canonicalJson({
      domain: "ironsmith-match-clock-audit-v1",
      clock: payload,
    }));
  }

  async function buildMatchClockAudit({ command, actor, seq }) {
    const timeoutForfeit = command?.type === "forfeit_player"
      && (
        command?.reason === "peer_claimed_match_clock_timeout"
        || command?.reason === "match_clock_timeout"
        || command?.reason === "peer_claimed_action_timeout"
        || command?.reason === "action_timeout"
      );
    const activePlayer = Number(state.expectedActor);
    const remaining = state.remainingMsByPlayer.slice();
    const observedElapsed = Math.max(0, Date.now() - state.timerStartedAt);
    const elapsedMs = timeoutForfeit
      ? Number(remaining[activePlayer] || 0)
      : Math.min(Number(remaining[activePlayer] || 0), observedElapsed);
    remaining[activePlayer] = Math.max(0, Number(remaining[activePlayer] || 0) - elapsedMs);
    const clock = {
      type: "match_clock_v1",
      version: 1,
      matchId,
      seq: Number(seq),
      actor: Number(actor),
      reason: timeoutForfeit ? "timeout_claim" : "action",
      policy: {
        type: "per_player_match_clock_v1",
        initialMs: state.timeoutMs,
        graceMs: state.clockGraceMs,
      },
      activePlayer,
      elapsedMs,
      remainingMsByPlayer: remaining,
      previousClockHash: state.clockHash,
      basisSequence: state.lastSeq,
    };
    clock.clockHash = await matchClockHash(clock);
    return clock;
  }

  async function verifyMatchClockAudit(clock, command, actor, seq) {
    if (!clock || typeof clock !== "object") {
      throw new Error("Missing match clock audit");
    }
    if (String(clock.previousClockHash || "") !== state.clockHash) {
      throw new Error("Match clock hash mismatch");
    }
    if (String(clock.clockHash || "") !== await matchClockHash(clock)) {
      throw new Error("Match clock audit hash mismatch");
    }
    if (Number(clock.seq) !== Number(seq) || Number(clock.actor) !== Number(actor)) {
      throw new Error("Match clock audit does not match action");
    }
    const activePlayer = Number(state.expectedActor);
    if (Number(clock.activePlayer) !== activePlayer) {
      throw new Error("Match clock active player mismatch");
    }
    const expectedRemaining = state.remainingMsByPlayer.slice();
    expectedRemaining[activePlayer] = Math.max(
      0,
      Number(expectedRemaining[activePlayer] || 0) - Number(clock.elapsedMs || 0)
    );
    if (audit.canonicalJson(expectedRemaining) !== audit.canonicalJson(clock.remainingMsByPlayer || [])) {
      throw new Error("Match clock remaining time mismatch");
    }
    const timeoutForfeit = command?.type === "forfeit_player"
      && (
        command?.reason === "peer_claimed_match_clock_timeout"
        || command?.reason === "match_clock_timeout"
        || command?.reason === "peer_claimed_action_timeout"
        || command?.reason === "action_timeout"
      );
    if (timeoutForfeit && Number((clock.remainingMsByPlayer || [])[activePlayer] || 0) !== 0) {
      throw new Error("Match clock has not expired");
    }
    state.remainingMsByPlayer = (clock.remainingMsByPlayer || []).map((value) => Math.max(0, Number(value || 0)));
    state.clockHash = String(clock.clockHash || "");
  }

  async function receive(message) {
    if (!state.connected) {
      return { status: "offline", seat };
    }
    try {
      if (!message || message.protocolVersion !== ${PROTOCOL_VERSION} || message.type !== "apply_action") {
        throw new Error("Unsupported P2P message");
      }
      const entry = {
        command: message.command,
        audit: message.audit,
      };
      const auditEnvelope = entry.audit;
      const seq = Number(auditEnvelope?.seq);
      if (seq <= state.lastSeq) {
        state.duplicates.push(seq);
        return { status: "duplicate", seq, lastSeq: state.lastSeq };
      }
      if (seq !== state.lastSeq + 1) {
        throw new Error(\`Expected audit sequence \${state.lastSeq + 1}, received \${seq}\`);
      }
      if (auditEnvelope.prevStateHash !== state.stateHash) {
        throw new Error(\`Audit state hash mismatch at sequence \${seq}\`);
      }
      if (audit.canonicalJson(auditEnvelope.command) !== audit.canonicalJson(entry.command)) {
        throw new Error(\`Audit command mismatch at sequence \${seq}\`);
      }
      const actor = Number(auditEnvelope.actor);
      const signer = Number(auditEnvelope.signer ?? actor);
      if (signer !== actor) {
        throw new Error(\`Action \${seq} was not signed by the acting player\`);
      }
      const timeoutForfeit = entry.command?.type === "forfeit_player"
        && (
          entry.command?.reason === "peer_claimed_match_clock_timeout"
          || entry.command?.reason === "match_clock_timeout"
          || entry.command?.reason === "peer_claimed_action_timeout"
          || entry.command?.reason === "action_timeout"
        );
      if (timeoutForfeit) {
        const forfeitedPlayer = Number(entry.command?.player);
        if (forfeitedPlayer !== state.expectedActor) {
          throw new Error(\`Timeout forfeit targets player \${forfeitedPlayer}, but player \${state.expectedActor} has priority\`);
        }
        const deadlineAt = Number(entry.command?.deadline_at_ms || (state.timerStartedAt + state.timeoutMs));
        if (Date.now() + 50 < deadlineAt) {
          throw new Error("Match clock has not expired");
        }
      } else if (actor !== state.expectedActor) {
        throw new Error(\`Action \${seq} was signed by player \${actor}, but player \${state.expectedActor} has priority\`);
      }
      await verifyMatchClockAudit(auditEnvelope.clock, entry.command, actor, seq);
      const player = state.players.find((candidate) => Number(candidate.seat) === signer);
      if (!player?.auditPublicKey) {
        throw new Error(\`Missing audit public key for player \${signer}\`);
      }
      const signerKey = await audit.importAuditPublicKey(player.auditPublicKey);
      const envelopePayload = {
        matchId: auditEnvelope.matchId,
        seq,
        actor,
        signer,
        prevStateHash: auditEnvelope.prevStateHash,
        command: auditEnvelope.command,
        clock: auditEnvelope.clock,
        openings: auditEnvelope.openings || [],
        rngReveals: auditEnvelope.rngReveals || [],
        shuffleProofs: auditEnvelope.shuffleProofs || [],
        privateViewProofs: auditEnvelope.privateViewProofs || [],
        publicCheckpointHash: auditEnvelope.publicCheckpointHash,
        nextStateHash: auditEnvelope.nextStateHash,
      };
      const valid = await audit.verifyAuditPayload(signerKey, envelopePayload, auditEnvelope.signature || "");
      if (!valid) {
        throw new Error(\`Sequenced audit signature is invalid at sequence \${seq}\`);
      }
      const quorumPlayers = timeoutForfeit
        ? state.players.filter((candidate) => Number(candidate.index ?? candidate.seat) !== Number(entry.command?.player))
        : state.players;
      const quorumThreshold = timeoutForfeit
        ? quorumPlayers.length
        : 3;
      await audit.verifyActionQuorumCertificate({
        certificate: auditEnvelope.quorumCertificate || message.quorumCertificate,
        action: {
          ...message,
          seq,
          actorIndex: actor,
        },
        players: quorumPlayers.map((candidate) => ({
          index: Number(candidate.index ?? candidate.seat),
          auditPublicKey: candidate.auditPublicKey,
        })),
        threshold: quorumThreshold,
      });
      const computedHash = await audit.auditStateHash({
        matchId: auditEnvelope.matchId,
        seq,
        prevStateHash: auditEnvelope.prevStateHash,
        command: auditEnvelope.command,
        clock: auditEnvelope.clock,
        openings: auditEnvelope.openings || [],
        rngReveals: auditEnvelope.rngReveals || [],
        shuffleProofs: auditEnvelope.shuffleProofs || [],
        privateViewProofs: auditEnvelope.privateViewProofs || [],
        publicCheckpointHash: auditEnvelope.publicCheckpointHash,
      });
      if (computedHash !== auditEnvelope.nextStateHash) {
        throw new Error(\`Audit next state hash mismatch at sequence \${seq}\`);
      }
      for (const opening of auditEnvelope.openings || []) {
        await verifyOpening(opening);
      }
      for (const rngReveal of auditEnvelope.rngReveals || []) {
        await verifyRngReveal(rngReveal, seq);
      }
      state.lastSeq = seq;
      state.stateHash = auditEnvelope.nextStateHash;
      state.expectedActor = Number(entry.command?.nextActor ?? (
        timeoutForfeit
          ? ((Number(entry.command?.player) + 1) % playerCount)
          : ((actor + 1) % playerCount)
      ));
      state.timerStartedAt = Date.now();
      state.transcript.push(entry);
      return {
        status: "accepted",
        seat,
        seq,
        stateHash: state.stateHash,
        expectedActor: state.expectedActor,
      };
    } catch (err) {
      const messageText = String(err?.message || err || "rejected");
      state.rejected.push(messageText);
      throw new Error(messageText);
    }
  }

  async function signedMessage({ command, actor = seat, signer = actor, seq = state.lastSeq + 1, prevStateHash = state.stateHash, openings = [], rngReveals = [], shuffleProofs = [], privateViewProofs = [] }) {
    const clock = await buildMatchClockAudit({ command, actor, seq });
    const envelope = await audit.buildSignedActionEnvelope({
      keyPair,
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
      publicCheckpointHash: \`public-checkpoint-\${seq}\`,
    });
    return {
      protocolVersion: ${PROTOCOL_VERSION},
      type: "apply_action",
      command,
      audit: envelope,
    };
  }

  window.client = {
    async configureTable({ matchPayload }) {
      state.matchPayload = matchPayload;
      state.players = matchPayload.players || [];
      state.publicDecks = matchPayload.deckAuditManifests || [];
      return true;
    },
    identity() {
      return publicPlayer;
    },
    async signMatchGenesis(matchPayload) {
      return audit.buildSignedMatchGenesis({
        keyPair,
        match: matchPayload,
        hostSeat: seat,
      });
    },
    async signActionQuorumVote(message) {
      return audit.buildSignedActionQuorumVote({
        keyPair,
        action: message,
        voter: seat,
      });
    },
    setConnected(connected, reason = "") {
      state.connected = Boolean(connected);
      if (!state.connected && reason) state.warnings.push(reason);
      if (state.connected && reason) {
        state.warnings = state.warnings.filter((warning) => warning !== reason);
      }
      return { connected: state.connected, warnings: state.warnings.slice() };
    },
    warn(message) {
      const normalized = String(message || "").trim();
      if (normalized && !state.warnings.includes(normalized)) state.warnings.push(normalized);
      return state.warnings.slice();
    },
    clearWarning(message) {
      state.warnings = state.warnings.filter((warning) => warning !== message);
      return state.warnings.slice();
    },
    async signPass() {
      const actor = seat;
      return signedMessage({
        command: {
          kind: "pass_priority",
          actor,
          nextActor: (actor + 1) % playerCount,
        },
      });
    },
    async signDraw(slot = 0) {
      const opening = await audit.buildDeckSlotOpening({ manifest: privateManifest, slot });
      const actor = seat;
      return signedMessage({
        command: {
          kind: "draw_card",
          actor,
          owner: seat,
          slot,
          card: opening.card,
          nextActor: (actor + 1) % playerCount,
        },
        openings: [opening],
      });
    },
    async forgeSignerMismatch() {
      return signedMessage({
        actor: (seat + 1) % playerCount,
        signer: seat,
        command: {
          kind: "pass_priority",
          actor: (seat + 1) % playerCount,
          forgedBy: seat,
          nextActor: (seat + 2) % playerCount,
        },
      });
    },
    async forgeOutOfTurn() {
      const actor = seat;
      return signedMessage({
        actor,
        signer: actor,
        command: {
          kind: "pass_priority",
          actor,
          nextActor: (actor + 1) % playerCount,
        },
      });
    },
    async forgeFutureSequence() {
      const actor = seat;
      return signedMessage({
        actor,
        signer: actor,
        seq: state.lastSeq + 2,
        command: {
          kind: "pass_priority",
          actor,
          nextActor: (actor + 1) % playerCount,
        },
      });
    },
    async signTimeoutForfeit(player = state.expectedActor, deadlineOffsetMs = 0) {
      const actor = seat;
      const startedAt = state.timerStartedAt;
      const deadlineAt = startedAt + state.timeoutMs + Number(deadlineOffsetMs || 0);
      return signedMessage({
        actor,
        signer: actor,
        command: {
          type: "forfeit_player",
          player: Number(player),
          reason: "peer_claimed_match_clock_timeout",
          timeout_ms: state.timeoutMs,
          match_clock_hash: state.clockHash,
          deadline_started_at_ms: startedAt,
          deadline_at_ms: deadlineAt,
          claimed_at_ms: Date.now(),
          basis_sequence: state.lastSeq,
          nextActor: (Number(player) + 1) % playerCount,
        },
      });
    },
    async forgeBadOpening() {
      const actor = seat;
      const opening = {
        owner: seat,
        slot: 0,
        card: "Black Lotus",
        salt: "not-the-committed-salt",
        commitment: "00",
      };
      return signedMessage({
        actor,
        signer: actor,
        command: {
          kind: "draw_card",
          actor,
          owner: seat,
          slot: 0,
          card: opening.card,
          nextActor: (actor + 1) % playerCount,
        },
        openings: [opening],
      });
    },
    async signFairRandom(tamper = false) {
      const actor = seat;
      const requirementId = \`fair_random:\${state.lastSeq + 1}:coin\`;
      const reveals = [];
      const commits = [];
      for (let player = 0; player < playerCount; player += 1) {
        const nonceHex = audit.randomAuditHex(32);
        const commitmentHex = await rngCommitmentForNonce(nonceHex);
        commits.push({ player, commitmentHex });
        reveals.push({ player, nonceHex, commitmentHex });
      }
      const combinedSeedHex = await audit.sha256Hex(audit.canonicalJson({
        domain: "ironsmith-combined-rng-v1",
        matchId,
        seq: state.lastSeq + 1,
        requirementId,
        commits,
        reveals,
      }));
      if (tamper) {
        reveals[0] = {
          ...reveals[0],
          nonceHex: audit.randomAuditHex(32),
        };
      }
      return signedMessage({
        actor,
        signer: actor,
        command: {
          kind: "fair_random_choice",
          actor,
          nextActor: (actor + 1) % playerCount,
        },
        rngReveals: [{
          type: "commit_reveal_random",
          requirementId,
          count: 1,
          commits,
          reveals,
          combinedSeedHex,
        }],
      });
    },
    async receive(message) {
      return receive(message);
    },
    async verifyTranscript() {
      return audit.verifyLiveAuditTranscript({
        version: 1,
        kind: "ironsmith-live-browser-audit-v1",
        match: state.matchPayload,
        matchId,
        lobbyId: matchId,
        protocolVersion: ${PROTOCOL_VERSION},
        signatureAlgorithm: "ecdsa-p256-sha256",
        genesis: state.matchPayload?.genesis,
        initialStateHash: "${INITIAL_AUDIT_STATE_HASH}",
        initialPublicCheckpointHash: state.matchPayload?.initialPublicCheckpointHash || "",
        actions: state.transcript,
      });
    },
    snapshot() {
      return {
        seat,
        connected: state.connected,
        warnings: state.warnings.slice(),
        lastSeq: state.lastSeq,
        expectedActor: state.expectedActor,
        stateHash: state.stateHash,
        transcriptLength: state.transcript.length,
        rejected: state.rejected.slice(),
        duplicates: state.duplicates.slice(),
      };
    },
  };
  return window.client.identity();
};
`;
}

async function launchChromium() {
  try {
    return await chromium.launch({ headless: true });
  } catch (err) {
    return chromium.launch({ channel: "chrome", headless: true });
  }
}

async function createTable({ browser, url, auditBundle, scenarioName }) {
  const matchId = `${MATCH_ID_PREFIX}:${scenarioName}:${Date.now()}`;
  const contexts = [];
  const pages = [];
  const identities = [];

  for (let seat = 0; seat < 4; seat += 1) {
    const context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(url);
    await page.addScriptTag({ content: auditBundle });
    await page.addScriptTag({ content: clientHarnessSource() });
    const identity = await page.evaluate(
      ({ seat: playerSeat, matchId: activeMatchId, deck }) =>
        window.installIronsmithP2PClient({
          seat: playerSeat,
          playerCount: 4,
          timeoutMs: 25,
          matchId: activeMatchId,
          deck,
        }),
      { seat, matchId, deck: playerDeck(seat) },
    );
    contexts.push(context);
    pages.push(page);
    identities.push(identity);
  }

  const players = identities.map((identity) => ({
    seat: identity.seat,
    index: identity.index,
    peerId: identity.peerId,
    name: identity.name,
    auditPublicKey: identity.auditPublicKey,
    auditEncryptionPublicKey: identity.auditEncryptionPublicKey,
    deckAuditManifest: identity.deckAuditManifest,
    ziffleKey: identity.ziffleKey,
    deckCount: identity.deckCount,
    sideboardCount: identity.sideboardCount,
    commanderCount: identity.commanderCount,
    playerGenesisSignature: identity.playerGenesisSignature,
  }));
  const publicDecks = identities.map((identity) => identity.deckAuditManifest);
  const ziffleKeys = players.map((player) => player.ziffleKey);
  const matchPayload = {
    protocolVersion: PROTOCOL_VERSION,
    auditMatchId: matchId,
    lobbyId: matchId,
    hostPeerId: players[0].peerId,
    format: "normal",
    startingLife: 20,
    openingHandSize: 7,
    seed: 1,
    timeoutMs: 25,
    matchClockPolicy: {
      type: "per_player_match_clock_v1",
      initialMs: 25,
      graceMs: 5,
    },
    initialPublicCheckpointHash: "browser-initial-public-checkpoint",
    players,
    deckAuditManifests: publicDecks,
    ziffleKeys,
    ziffleCeremonies: players.map((player) => ({
      owner: player.index,
      deckCount: player.deckCount,
      context: matchId,
      keyContext: matchId,
      keys: ziffleKeys,
      steps: players.map((shuffler) => ({
        shuffler: shuffler.index,
        deckHex: `browser-deck-${player.index}-${shuffler.index}`,
        proofHex: `browser-proof-${player.index}-${shuffler.index}`,
      })),
      deckHash: `browser-ziffle-deck-${player.index}`,
    })),
  };
  matchPayload.genesis = await pages[0].evaluate(
    (payload) => window.client.signMatchGenesis(payload),
    matchPayload,
  );
  await Promise.all(
    pages.map((page) => page.evaluate(
      (payload) => window.client.configureTable({ matchPayload: payload }),
      matchPayload,
    )),
  );

  return {
    matchId,
    pages,
    contexts,
    closed: false,
    connected: new Set([0, 1, 2, 3]),
    ledger: [],
    async close() {
      if (this.closed) return;
      this.closed = true;
      await Promise.all(contexts.map((context) => context.close()));
    },
  };
}

async function snapshot(table, seat) {
  return table.pages[seat].evaluate(() => window.client.snapshot());
}

async function snapshots(table) {
  return Promise.all(table.pages.map((_, seat) => snapshot(table, seat)));
}

async function sign(table, seat, method, ...args) {
  const message = await table.pages[seat].evaluate(
    ([methodName, methodArgs]) => window.client[methodName](...methodArgs),
    [method, args],
  );
  return certifyAction(table, message);
}

async function certifyAction(table, message) {
  if (!message || message.type !== "apply_action" || !message.audit) return message;
  const timeoutForfeit = message.command?.type === "forfeit_player"
    && (
      message.command?.reason === "peer_claimed_match_clock_timeout"
      || message.command?.reason === "match_clock_timeout"
      || message.command?.reason === "peer_claimed_action_timeout"
      || message.command?.reason === "action_timeout"
    );
  const allSeats = [0, 1, 2, 3];
  const voterSeats = timeoutForfeit
    ? allSeats.filter((voter) => voter !== Number(message.command?.player))
    : allSeats;
  const threshold = timeoutForfeit
    ? voterSeats.length
    : 3;
  const voters = voterSeats.slice(0, threshold);
  const votes = await Promise.all(voters.map((voter) =>
    table.pages[voter].evaluate(
      (incoming) => window.client.signActionQuorumVote(incoming),
      message,
    )
  ));
  message.audit.quorumCertificate = {
    type: "ironsmith-action-quorum-v1",
    matchId: String(message.audit.matchId || ""),
    seq: Number(message.audit.seq || 0),
    actor: Number(message.audit.actor ?? message.actorIndex ?? 0),
    prevStateHash: String(message.audit.prevStateHash || ""),
    nextStateHash: String(message.audit.nextStateHash || ""),
    publicCheckpointHash: String(message.audit.publicCheckpointHash || ""),
    actionSignature: String(message.audit.signature || ""),
    threshold,
    voters,
    votes,
  };
  return message;
}

async function deliver(table, targetSeat, message) {
  if (!table.connected.has(targetSeat)) {
    return { status: "transport-offline", seat: targetSeat };
  }
  return table.pages[targetSeat].evaluate((incoming) => window.client.receive(incoming), message);
}

async function broadcast(table, _fromSeat, message, { dropTo = [] } = {}) {
  const dropped = new Set(dropTo);
  const results = [];
  for (let seat = 0; seat < table.pages.length; seat += 1) {
    if (dropped.has(seat)) {
      results.push({ status: "dropped", seat });
      continue;
    }
    results.push(await deliver(table, seat, message));
  }
  if (results.some((result) => result.status === "accepted")) {
    table.ledger.push(message);
  }
  return results;
}

async function disconnect(table, seats, reason) {
  for (const seat of seats) {
    table.connected.delete(seat);
    await table.pages[seat].evaluate(
      ({ connected, warning }) => window.client.setConnected(connected, warning),
      { connected: false, warning: reason },
    );
  }
  const warnings = seats.map((seat) => `Player ${seat + 1} disconnected`);
  await Promise.all(table.pages.map((page, seat) => {
    if (seats.includes(seat)) return Promise.resolve();
    return Promise.all(warnings.map((warning) => page.evaluate((message) => window.client.warn(message), warning)));
  }));
}

async function reconnectAndResync(table, seats) {
  for (const seat of seats) {
    table.connected.add(seat);
    await table.pages[seat].evaluate(
      ({ connected, warning }) => window.client.setConnected(connected, warning),
      { connected: true, warning: "network disconnected" },
    );
    for (const message of table.ledger) {
      await deliver(table, seat, message);
    }
  }
  await Promise.all(table.pages.map((page) => Promise.all(
    seats.map((seat) => page.evaluate((warning) => window.client.clearWarning(warning), `Player ${seat + 1} disconnected`)),
  )));
}

async function expectReject(promise, pattern) {
  try {
    await promise;
  } catch (err) {
    const message = String(err?.message || err);
    assert.match(message, pattern);
    return message;
  }
  assert.fail(`Expected rejection matching ${pattern}`);
}

function assertSameHash(activeSnapshots) {
  const hashes = new Set(activeSnapshots.map((entry) => entry.stateHash));
  assert.equal(hashes.size, 1, `Expected one shared state hash, got ${Array.from(hashes).join(", ")}`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function verifyAllTranscripts(table, seats = [0, 1, 2, 3]) {
  const verifications = [];
  for (const seat of seats) {
    verifications.push(await table.pages[seat].evaluate(() => window.client.verifyTranscript()));
  }
  return verifications;
}

async function withTable(env, scenarioName, fn) {
  const table = await createTable({ ...env, scenarioName });
  try {
    return await fn(table);
  } finally {
    await table.close();
  }
}

async function runScenarios(env) {
  const results = [];

  results.push(await withTable(env, "honest-four-player-play", async (table) => {
    await broadcast(table, 0, await sign(table, 0, "signPass"));
    await broadcast(table, 1, await sign(table, 1, "signDraw", 0));
    await broadcast(table, 2, await sign(table, 2, "signPass"));
    await broadcast(table, 3, await sign(table, 3, "signDraw", 1));
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [4, 4, 4, 4]);
    assertSameHash(state);
    const verified = await verifyAllTranscripts(table);
    assert.deepEqual(verified.map((entry) => entry.verifiedActions), [4, 4, 4, 4]);
    return {
      name: "honest-four-player-play",
      outcome: "All four browser peers accepted the same signed action log and ended on one audit hash.",
      finalStateHash: state[0].stateHash,
      verifiedActions: 4,
    };
  }));

  results.push(await withTable(env, "non-acting-player-disconnects-and-resyncs", async (table) => {
    await disconnect(table, [2], "network disconnected");
    await broadcast(table, 0, await sign(table, 0, "signPass"));
    await broadcast(table, 1, await sign(table, 1, "signDraw", 0));
    let state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [2, 2, 0, 2]);
    assert(state[0].warnings.includes("Player 3 disconnected"));
    await reconnectAndResync(table, [2]);
    state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [2, 2, 2, 2]);
    assertSameHash(state);
    await verifyAllTranscripts(table);
    return {
      name: "non-acting-player-disconnects-and-resyncs",
      outcome: "Connected peers kept the canonical log; the returning browser caught up from redelivery and matched the audit hash.",
      warningShown: "Player 3 disconnected",
      finalStateHash: state[0].stateHash,
    };
  }));

  results.push(await withTable(env, "priority-holder-disconnects-stalls-game", async (table) => {
    await broadcast(table, 0, await sign(table, 0, "signPass"));
    await broadcast(table, 1, await sign(table, 1, "signPass"));
    await disconnect(table, [2], "network disconnected");
    const outOfTurn = await sign(table, 3, "forgeOutOfTurn");
    const rejection = await expectReject(deliver(table, 0, outOfTurn), /player 2 has priority/);
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [2, 2, 2, 2]);
    return {
      name: "priority-holder-disconnects-stalls-game",
      outcome: "No peer can advance priority for a disconnected active player; an out-of-turn substitute action is rejected.",
      rejection,
    };
  }));

  results.push(await withTable(env, "priority-holder-times-out-and-forfeits", async (table) => {
    await broadcast(table, 0, await sign(table, 0, "signPass"));
    await sleep(40);
    const timeoutForfeit = await sign(table, 2, "signTimeoutForfeit", 1);
    await broadcast(table, 2, timeoutForfeit);
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [2, 2, 2, 2]);
    assertSameHash(state);
    const verified = await verifyAllTranscripts(table);
    assert.deepEqual(verified.map((entry) => entry.verifiedActions), [2, 2, 2, 2]);
    return {
      name: "priority-holder-times-out-and-forfeits",
      outcome: "After the match clock expires, another peer can submit a signed timeout-forfeit action for the stalled priority holder.",
      finalStateHash: state[0].stateHash,
    };
  }));

  results.push(await withTable(env, "cheat-early-timeout-forfeit", async (table) => {
    await broadcast(table, 0, await sign(table, 0, "signPass"));
    const earlyTimeout = await sign(table, 2, "signTimeoutForfeit", 1, 10000);
    const rejection = await expectReject(deliver(table, 0, earlyTimeout), /Match clock has not expired/);
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [1, 1, 1, 1]);
    assertSameHash(state);
    return {
      name: "cheat-early-timeout-forfeit",
      outcome: "A peer cannot falsely forfeit the priority holder before the locally verified match clock has expired.",
      rejection,
    };
  }));

  results.push(await withTable(env, "multiple-players-disconnect-and-resync", async (table) => {
    await disconnect(table, [1, 3], "network disconnected");
    await broadcast(table, 0, await sign(table, 0, "signPass"));
    let state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [1, 0, 1, 0]);
    await reconnectAndResync(table, [1, 3]);
    state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [1, 1, 1, 1]);
    assertSameHash(state);
    return {
      name: "multiple-players-disconnect-and-resync",
      outcome: "Two offline browsers missed live delivery, then replayed the canonical log and converged.",
      finalStateHash: state[0].stateHash,
    };
  }));

  results.push(await withTable(env, "mesh-relay-defeats-host-censorship", async (table) => {
    await broadcast(table, 0, await sign(table, 0, "signPass"));
    const playerOneAction = await sign(table, 1, "signDraw", 0);
    await deliver(table, 1, playerOneAction);
    await deliver(table, 2, playerOneAction);
    await deliver(table, 3, playerOneAction);
    await deliver(table, 0, playerOneAction);
    table.ledger.push(playerOneAction);
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [2, 2, 2, 2]);
    assertSameHash(state);
    return {
      name: "mesh-relay-defeats-host-censorship",
      outcome: "The former host did not sequence or approve the action; direct mesh relay delivered the actor-signed action to everyone.",
      finalStateHash: state[0].stateHash,
    };
  }));

  results.push(await withTable(env, "cheat-forged-action-for-another-player", async (table) => {
    const forged = await sign(table, 0, "forgeSignerMismatch");
    const rejection = await expectReject(deliver(table, 1, forged), /not signed by the acting player/);
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [0, 0, 0, 0]);
    return {
      name: "cheat-forged-action-for-another-player",
      outcome: "A peer cannot sign an action on another player’s behalf.",
      rejection,
    };
  }));

  results.push(await withTable(env, "cheat-tampered-command-after-signature", async (table) => {
    const original = await sign(table, 0, "signPass");
    const tampered = structuredClone(original);
    tampered.command = { ...tampered.command, kind: "draw_card", card: "Black Lotus" };
    const rejection = await expectReject(deliver(table, 1, tampered), /Audit command mismatch/);
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [0, 0, 0, 0]);
    return {
      name: "cheat-tampered-command-after-signature",
      outcome: "Changing a signed command after the actor signs it is detected before state advances.",
      rejection,
    };
  }));

  results.push(await withTable(env, "cheat-bad-library-opening", async (table) => {
    const badOpening = await sign(table, 0, "forgeBadOpening");
    const rejection = await expectReject(deliver(table, 1, badOpening), /does not match committed library/);
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [0, 0, 0, 0]);
    return {
      name: "cheat-bad-library-opening",
      outcome: "A player cannot reveal a card that is not the committed card for that encrypted library slot.",
      rejection,
    };
  }));

  results.push(await withTable(env, "cheat-tampered-rng-reveal", async (table) => {
    const badRandom = await sign(table, 0, "signFairRandom", true);
    const rejection = await expectReject(deliver(table, 1, badRandom), /fair-random reveal does not match commitment/);
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [0, 0, 0, 0]);
    return {
      name: "cheat-tampered-rng-reveal",
      outcome: "A signed action with a malformed commit-reveal transcript is rejected before state advances.",
      rejection,
    };
  }));

  results.push(await withTable(env, "cheat-out-of-turn-action", async (table) => {
    const outOfTurn = await sign(table, 2, "forgeOutOfTurn");
    const rejection = await expectReject(deliver(table, 0, outOfTurn), /player 0 has priority/);
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [0, 0, 0, 0]);
    return {
      name: "cheat-out-of-turn-action",
      outcome: "A valid player signature is not enough; the action also has to match the current priority holder.",
      rejection,
    };
  }));

  results.push(await withTable(env, "cheat-future-sequence-gap", async (table) => {
    const future = await sign(table, 0, "forgeFutureSequence");
    const rejection = await expectReject(deliver(table, 1, future), /Expected audit sequence 1, received 2/);
    const state = await snapshots(table);
    assert.deepEqual(state.map((entry) => entry.lastSeq), [0, 0, 0, 0]);
    return {
      name: "cheat-future-sequence-gap",
      outcome: "Peers reject an action that skips an audit sequence and request/resync missing history instead.",
      rejection,
    };
  }));

  results.push(await withTable(env, "duplicate-replay-is-idempotent", async (table) => {
    const message = await sign(table, 0, "signPass");
    await broadcast(table, 0, message);
    const replay = await deliver(table, 1, message);
    const state = await snapshots(table);
    assert.equal(replay.status, "duplicate");
    assert.deepEqual(state.map((entry) => entry.lastSeq), [1, 1, 1, 1]);
    assert.equal(state[1].duplicates.at(-1), 1);
    assertSameHash(state);
    return {
      name: "duplicate-replay-is-idempotent",
      outcome: "Replayed traffic is recorded as a duplicate and does not mutate game state twice.",
      duplicateSequence: 1,
      finalStateHash: state[0].stateHash,
    };
  }));

  return results;
}

async function main() {
  const server = await startStaticServer();
  const auditBundle = await loadAuditBrowserBundle();
  const browser = await launchChromium();
  try {
    const results = await runScenarios({ browser, url: server.url, auditBundle });
    console.log(JSON.stringify({
      ok: true,
      environment: {
        browser: "chromium",
        players: 4,
        protocolVersion: PROTOCOL_VERSION,
        pageUrl: server.url,
      },
      scenarios: results,
    }, null, 2));
  } finally {
    await browser.close();
    await server.close();
  }
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
