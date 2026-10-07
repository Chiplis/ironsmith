import test from "node:test";
import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { replayAuditTranscriptWithGame, startAuditTranscriptReplayWithGame } from "../src/lib/audit-replay.js";
import { CURRENT_AUDIT_PROTOCOL_VERSION, CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION,
  buildPrivateDeckManifest, publicCheckpointHash } from "../src/lib/multiplayer-audit.js";
import { buildZiffleRuntimeManifest } from "../src/lib/ziffle-runtime-manifest.js";

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function checkpointFor({ config, commands = [], openings = [], seeds = [], shuffles = [], forfeits = [] }) {
  return {
    version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION,
    config,
    commands,
    openings,
    seeds,
    shuffles,
    forfeits,
  };
}

class FakeReplayGame {
  constructor() {
    this.config = null;
    this.commands = [];
    this.openings = [];
    this.seeds = [];
    this.shuffles = [];
    this.forfeits = [];
    this.perspective = 0;
    this.checkpoint = { version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION, live: true };
    this.restoredPerspective = null;
  }

  refreshCheckpoint() {
    this.checkpoint = checkpointFor({
      config: clone(this.config),
      commands: clone(this.commands),
      openings: clone(this.openings),
      seeds: clone(this.seeds),
      shuffles: clone(this.shuffles),
      forfeits: clone(this.forfeits),
    });
  }

  async captureState() {
    return {
      checkpoint: clone(this.checkpoint),
      config: clone(this.config),
      commands: clone(this.commands),
      openings: clone(this.openings),
      seeds: clone(this.seeds),
      shuffles: clone(this.shuffles),
      forfeits: clone(this.forfeits),
      perspective: this.perspective,
    };
  }

  async restoreState(snapshot, perspectiveIndex = 0) {
    this.checkpoint = clone(snapshot.checkpoint);
    this.config = clone(snapshot.config);
    this.commands = clone(snapshot.commands);
    this.openings = clone(snapshot.openings);
    this.seeds = clone(snapshot.seeds);
    this.shuffles = clone(snapshot.shuffles);
    this.forfeits = clone(snapshot.forfeits);
    this.perspective = Number(perspectiveIndex);
    this.restoredPerspective = Number(perspectiveIndex);
    return clone(this.checkpoint);
  }

  async getHiddenCardState() { return this.captureState(); }
  async createRuntimeSavepoint() {
    this.savepoints ||= new Map();
    const handle = (this.nextSavepoint || 0) + 1;
    this.nextSavepoint = handle;
    this.savepoints.set(handle, await this.captureState());
    return handle;
  }
  async restoreRuntimeSavepoint(handle) {
    const state = this.savepoints.get(handle);
    assert.ok(state, "runtime savepoint exists");
    this.savepoints.delete(handle);
    return this.restoreState(state, state.perspective);
  }

  async startMatch(config) {
    this.config = clone(config);
    this.commands = [];
    this.openings = [];
    this.seeds = [];
    this.shuffles = [];
    this.forfeits = [];
    this.refreshCheckpoint();
    return clone(this.checkpoint);
  }

  async setPerspective(perspectiveIndex) {
    this.perspective = Number(perspectiveIndex);
  }

  async previewCryptoRequirements(command) {
    if (command?.type !== "priority_action") return [];
    return [
      {
        id: "rng-1",
        type: "fair_random",
      },
    ];
  }

  async injectTranscriptRandomSeeds({ seeds = [] }) {
    this.seeds.push(...seeds);
    this.refreshCheckpoint();
  }

  async revealHiddenSlot(opening) {
    this.openings.push(clone(opening));
    this.refreshCheckpoint();
    return clone(this.checkpoint);
  }

  async applyVerifiedHiddenLibraryShuffle(shuffle) {
    this.shuffles.push(clone(shuffle));
    this.refreshCheckpoint();
    return clone(this.checkpoint);
  }

  async dispatch(command) {
    this.commands.push(clone(command));
    this.refreshCheckpoint();
    return clone(this.checkpoint);
  }

  async forfeitPlayer(player) {
    this.forfeits.push(Number(player));
    this.refreshCheckpoint();
    return clone(this.checkpoint);
  }

  async exportPublicAuditCheckpoint() {
    return clone(this.checkpoint);
  }
}

function replayMatch() {
  return {
    protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
    players: [
      { name: "Alice" },
      { name: "Bob" },
    ],
    startingLife: 20,
    seed: "replay-seed",
    format: "normal",
    decks: [[], []],
    runtimeHiddenDeckManifests: [
      { owner: 0, slotCommitments: [] },
      { owner: 1, slotCommitments: [] },
    ],
    openingHandSize: 7,
  };
}

async function initialHashForMatch(match) {
  const game = new FakeReplayGame();
  await game.startMatch({
    playerNames: ["Alice", "Bob"],
    startingLife: 20,
    seed: "replay-seed",
    format: "normal",
    decks: [[], []],
    hiddenDeckManifests: clone(match.runtimeHiddenDeckManifests),
    openingHandSize: 7,
  });
  return publicCheckpointHash(await game.exportPublicAuditCheckpoint(), webcrypto);
}

async function actionHashForTranscript(match, action) {
  const game = new FakeReplayGame();
  await game.startMatch({
    playerNames: ["Alice", "Bob"],
    startingLife: 20,
    seed: "replay-seed",
    format: "normal",
    decks: [[], []],
    hiddenDeckManifests: clone(match.runtimeHiddenDeckManifests),
    openingHandSize: 7,
  });
  await game.injectTranscriptRandomSeeds({
    seeds: [
      String(action.audit.rngReveals[0].combinedSeedHex),
    ],
  });
  await game.revealHiddenSlot({
    owner: 0,
    slot: 2,
    cardName: "Island",
    commitment: "commitment-2",
    recomputeDecision: true,
  });
  await game.dispatch({ type: "priority_action", action_index: 0 });
  return publicCheckpointHash(await game.exportPublicAuditCheckpoint(), webcrypto);
}

test("replays transcript actions through the engine and restores the complete live runtime", async () => {
  const match = replayMatch();
  const initialPublicCheckpointHash = await initialHashForMatch(match);
  const action = {
    seq: 1,
    command: { type: "priority_action", action_index: 0 },
    audit: {
      seq: 1,
      command: { type: "priority_action", action_index: 0 },
      openings: [
        {
          owner: 0,
          slot: 2,
          card: "Island",
          commitment: "commitment-2",
          timing: "pre",
        },
      ],
      rngReveals: [
        {
          requirementId: "rng-1",
          combinedSeedHex: "rng-seed-1",
        },
      ],
    },
  };
  action.audit.publicCheckpointHash = await actionHashForTranscript(match, action);
  const transcript = {
    protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
    match,
    initialPublicCheckpointHash,
    actions: [action],
  };
  const game = new FakeReplayGame();
  const liveCheckpoint = await game.captureState();

  const report = await replayAuditTranscriptWithGame({
    game,
    transcript,
    perspectiveIndex: 1,
    cryptoImpl: webcrypto,
  });

  assert.equal(report.verified, true);
  assert.equal(report.replayedActions, 1);
  assert.deepEqual(report.actions, [
    {
      seq: 1,
      publicCheckpointHash: action.audit.publicCheckpointHash,
    },
  ]);
  assert.deepEqual(await game.captureState(), liveCheckpoint);
  assert.equal(game.restoredPerspective, liveCheckpoint.perspective);
});

test("restores the complete live runtime after replay rejects an initial hash mismatch", async () => {
  const game = new FakeReplayGame();
  const liveCheckpoint = await game.captureState();

  await assert.rejects(
    () => replayAuditTranscriptWithGame({
      game,
      transcript: {
        protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
        match: replayMatch(),
        initialPublicCheckpointHash: "wrong-hash",
        actions: [],
      },
      perspectiveIndex: 1,
      cryptoImpl: webcrypto,
    }),
    /initial public checkpoint hash does not match/
  );

  assert.deepEqual(await game.captureState(), liveCheckpoint);
  assert.equal(game.restoredPerspective, liveCheckpoint.perspective);
});

test("replay preserves complete sideboard slots in runtime manifests and the fallback", async () => {
  const manifest = await buildPrivateDeckManifest({
    matchId: "replay-sideboard", owner: 0, deck: ["Island", "Forest"], sideboard: ["Mountain"],
  }, webcrypto);
  const runtime = buildZiffleRuntimeManifest(manifest, { deckCount: 2, deckHash: "replay-deck" });
  for (const field of ["runtimeHiddenDeckManifests", "hiddenDeckManifests"]) {
    const game = new FakeReplayGame();
    const match = replayMatch();
    delete match.runtimeHiddenDeckManifests;
    match[field] = [runtime];
    await startAuditTranscriptReplayWithGame({ game,
      transcript: { protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION, match }, cryptoImpl: webcrypto });
    assert.deepEqual(game.config.hiddenDeckManifests, [runtime]);
    assert.equal(game.config.hiddenDeckManifests[0].slotCommitments[2].commitment, manifest.slotCommitments[2].commitment);
  }
});
