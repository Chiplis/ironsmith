#!/usr/bin/env node
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { initWasmGame, createNewGameAndPlayers } from "./wasm-test-harness.mjs";

const FIXTURE_DIR_URL = new URL("../fixtures/determinism/", import.meta.url);
const UPDATE_FIXTURES = process.env.UPDATE_DETERMINISM_FIXTURES === "1";

function canonical(value) {
  return JSON.stringify(value);
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function normalizedPublicAuditCheckpoint(game) {
  const checkpoint = game.exportPublicAuditCheckpoint();
  delete checkpoint.snapshotSerial;
  return checkpoint;
}

function canonicalSyncCheckpoint(game) {
  const checkpoint = game.exportSyncCheckpoint();
  delete checkpoint.snapshotSerial;
  return canonical(checkpoint);
}

function assertPeersEqual(a, b, label, roots) {
  const auditA = normalizedPublicAuditCheckpoint(a);
  const auditB = normalizedPublicAuditCheckpoint(b);
  assert.equal(
    canonical(auditA),
    canonical(auditB),
    `${label}: public audit checkpoints diverged`,
  );
  assert.equal(
    canonicalSyncCheckpoint(a),
    canonicalSyncCheckpoint(b),
    `${label}: sync checkpoints diverged`,
  );
  roots.push({
    label,
    public_audit_sha256: sha256(canonical(auditA)),
    hidden_zone_roots: auditA.hiddenZones.map((zone) => ({
      owner: zone.owner,
      zone: zone.zone,
      commitmentRoot: zone.commitmentRoot,
    })),
  });
}

function fixtureUrlForScenario(scenario) {
  return new URL(`${scenario}.json`, FIXTURE_DIR_URL);
}

async function verifyGoldenRoots(scenario, roots) {
  const payload = {
    schemaVersion: 1,
    scenario,
    roots,
  };
  const fixtureUrl = fixtureUrlForScenario(scenario);
  const serialized = `${JSON.stringify(payload, null, 2)}\n`;
  if (UPDATE_FIXTURES) {
    await mkdir(FIXTURE_DIR_URL, { recursive: true });
    await writeFile(fixtureUrl, serialized, "utf8");
    console.log(`updated ${fixtureUrl.pathname}`);
    return;
  }

  let expected;
  try {
    expected = JSON.parse(await readFile(fixtureUrl, "utf8"));
  } catch (error) {
    throw new Error(
      `missing determinism fixture ${fixtureUrl.pathname}; run UPDATE_DETERMINISM_FIXTURES=1 node scripts/lockstep-sim.mjs to create it`,
      { cause: error },
    );
  }
  assert.deepEqual(payload, expected, "golden commitment-root fixture changed");
}

const baseSetup = {
  playerNames: ["Alice", "Bob"],
  startingLife: 20,
  seed: 0x5eed,
  format: "normal",
  openingHandSize: 0,
  decks: [[], []],
};

const smallDeck = [
  "Forest",
  "Island",
  "Mountain",
  "Plains",
  "Swamp",
  "Ornithopter",
  "Grizzly Bears",
  "Lightning Bolt",
];
const reversedSmallDeck = [...smallDeck].reverse();

const scenarios = [
  {
    name: "lockstep-basic",
    setup: baseSetup,
    actions: [
      (game) => game.addCardToZone(0, "Ornithopter", "battlefield", true),
      (game) => game.addCardToZone(1, "Grizzly Bears", "hand", true),
      (game) => game.addCardToZone(0, "Lightning Bolt", "graveyard", true),
    ],
  },
  {
    name: "public-zone-spread",
    setup: { ...baseSetup, seed: 0x5eed01 },
    actions: [
      (game) => game.addCardToZone(0, "Forest", "battlefield", true),
      (game) => game.addCardToZone(0, "Lightning Bolt", "graveyard", true),
      (game) => game.addCardToZone(1, "Counterspell", "exile", true),
      (game) => game.addCardToZone(1, "Ornithopter", "command", true),
    ],
  },
  {
    name: "hidden-hands",
    setup: { ...baseSetup, seed: 0x5eed02 },
    actions: [
      (game) => game.addCardToZone(0, "Forest", "hand", true),
      (game) => game.addCardToZone(0, "Island", "hand", true),
      (game) => game.addCardToZone(1, "Mountain", "hand", true),
      (game) => game.addCardToZone(1, "Plains", "hand", true),
    ],
  },
  {
    name: "hidden-libraries",
    setup: { ...baseSetup, seed: 0x5eed03 },
    actions: [
      (game) => game.addCardToZone(0, "Forest", "library", true),
      (game) => game.addCardToZone(0, "Island", "library", true),
      (game) => game.addCardToZone(1, "Mountain", "library", true),
      (game) => game.addCardToZone(1, "Plains", "library", true),
    ],
  },
  {
    name: "draw-from-library",
    setup: { ...baseSetup, seed: 0x5eed04 },
    actions: [
      (game) => game.addCardToZone(0, "Forest", "library", true),
      (game) => game.addCardToZone(0, "Island", "library", true),
      (game) => game.addCardToZone(1, "Mountain", "library", true),
      (game) => game.drawCard(0),
      (game) => game.drawCard(1),
    ],
  },
  {
    name: "startmatch-shuffle-small",
    setup: {
      ...baseSetup,
      seed: 0x5eed05,
      decks: [smallDeck, reversedSmallDeck],
    },
    actions: [],
  },
  {
    name: "opening-hands-shuffled",
    setup: {
      ...baseSetup,
      seed: 0x5eed06,
      openingHandSize: 3,
      decks: [smallDeck, reversedSmallDeck],
    },
    actions: [],
  },
  {
    name: "post-setup-draws",
    setup: {
      ...baseSetup,
      seed: 0x5eed07,
      decks: [smallDeck, reversedSmallDeck],
    },
    actions: [
      (game) => game.drawCard(0),
      (game) => game.drawCard(0),
      (game) => game.drawCard(1),
    ],
  },
  {
    name: "battlefield-permanent-shape",
    setup: { ...baseSetup, seed: 0x5eed08 },
    actions: [
      (game) => game.addCardToZone(0, "Ornithopter", "battlefield", true),
      (game) => game.addCardToZone(1, "Blood Artist", "battlefield", true),
      (game) => game.addCardToZone(0, "Grizzly Bears", "battlefield", true),
    ],
  },
  {
    name: "mixed-visible-hidden",
    setup: {
      ...baseSetup,
      seed: 0x5eed09,
      decks: [smallDeck.slice(0, 4), smallDeck.slice(4)],
    },
    actions: [
      (game) => game.addCardToZone(0, "Ornithopter", "battlefield", true),
      (game) => game.drawCard(0),
      (game) => game.addCardToZone(1, "Lightning Bolt", "graveyard", true),
      (game) => game.drawCard(1),
    ],
  },
];

async function runScenario({ name, setup, actions }) {
  const roots = [];
  const { game: peerA } = await initWasmGame();
  const { game: peerB } = await initWasmGame();

  createNewGameAndPlayers(peerA, setup);
  createNewGameAndPlayers(peerB, setup);
  assertPeersEqual(peerA, peerB, "after setup", roots);

  for (const [index, mutate] of actions.entries()) {
    mutate(peerA);
    mutate(peerB);
    assertPeersEqual(peerA, peerB, `after action ${index + 1}`, roots);
  }

  const { game: lateJoiner } = await initWasmGame();
  lateJoiner.importSyncCheckpoint(peerB.exportSyncCheckpoint());
  assertPeersEqual(peerB, lateJoiner, "late joiner import", roots);
  await verifyGoldenRoots(name, roots);
}

async function main() {
  for (const scenario of scenarios) {
    await runScenario(scenario);
  }

  console.log("lockstep determinism suite passed");
}

main().catch((error) => {
  console.error(error?.stack || error);
  process.exit(1);
});
