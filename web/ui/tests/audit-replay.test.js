import test from "node:test";
import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { replayAuditTranscriptWithGame } from "../src/lib/audit-replay.js";
import { publicCheckpointHash } from "../src/lib/multiplayer-audit.js";

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function checkpointFor({ config, commands = [], openings = [], seeds = [], shuffles = [], forfeits = [] }) {
  return {
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
    this.checkpoint = { live: true };
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

  async exportSyncCheckpoint() {
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

  async importSyncCheckpoint(snapshot, perspectiveIndex = 0) {
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
        id: "shuffle-1",
        type: "verifiable_shuffle",
        owner: 0,
        zone: "library",
        afterOrder: [2, 1, 0],
      },
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
  const requirements = await game.previewCryptoRequirements(action.command);
  await game.injectTranscriptRandomSeeds({
    seeds: [
      String(action.audit.shuffleProofs[0].deckHash),
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
  await game.applyVerifiedHiddenLibraryShuffle({
    owner: 0,
    deckHash: "deck-hash-1",
    afterOrder: requirements[0].afterOrder,
  });
  return publicCheckpointHash(await game.exportPublicAuditCheckpoint(), webcrypto);
}

test("replays transcript actions through the engine and restores the live checkpoint", async () => {
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
      shuffleProofs: [
        {
          requirementId: "shuffle-1",
          owner: 0,
          zone: "library",
          deckHash: "deck-hash-1",
          afterOrder: [2, 1, 0],
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
    match,
    initialPublicCheckpointHash,
    actions: [action],
  };
  const game = new FakeReplayGame();
  const liveCheckpoint = await game.exportSyncCheckpoint();

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
  assert.deepEqual(await game.exportSyncCheckpoint(), {
    ...liveCheckpoint,
    perspective: 1,
  });
  assert.equal(game.restoredPerspective, 1);
});

test("restores the live checkpoint after replay rejects an initial hash mismatch", async () => {
  const game = new FakeReplayGame();
  const liveCheckpoint = await game.exportSyncCheckpoint();

  await assert.rejects(
    () => replayAuditTranscriptWithGame({
      game,
      transcript: {
        match: replayMatch(),
        initialPublicCheckpointHash: "wrong-hash",
        actions: [],
      },
      perspectiveIndex: 1,
      cryptoImpl: webcrypto,
    }),
    /initial public checkpoint hash does not match/
  );

  assert.deepEqual(await game.exportSyncCheckpoint(), {
    ...liveCheckpoint,
    perspective: 1,
  });
  assert.equal(game.restoredPerspective, 1);
});
