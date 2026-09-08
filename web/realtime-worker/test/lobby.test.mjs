import test from "node:test";
import assert from "node:assert/strict";
import { LobbyRoom } from "../src/index.js";

test("worker source exposes an authoritative lobby object", async () => {
  assert.equal(typeof LobbyRoom, "function");
  assert.ok(LobbyRoom.prototype.handleMessage);
  assert.ok(LobbyRoom.prototype.replay);
});

function roomHarness() {
  const values = new Map();
  const sent = [];
  const state = { storage: {
    async get(key) { return values.get(key); },
    async put(key, value) { values.set(key, value); },
  } };
  const room = new LobbyRoom(state);
  const socket = { send(value) { sent.push(JSON.parse(value)); } };
  const session = { socket, playerId: "", lastSequence: 0 };
  room.sessions.set(socket, session);
  return { room, session, sent };
}

test("actions are sequenced, acknowledged, and idempotent", async () => {
  const { room, session, sent } = roomHarness();
  await room.handleMessage(session, JSON.stringify({ type: "join", playerId: "p1" }));
  await room.handleMessage(session, JSON.stringify({
    type: "action", actionId: "a1", payload: { command: "pass" },
  }));
  await room.handleMessage(session, JSON.stringify({
    type: "action", actionId: "a1", payload: { command: "pass" },
  }));
  const acknowledgements = sent.filter((message) => message.type === "action_ack");
  assert.deepEqual(acknowledgements.map((message) => message.sequence), [1, 1]);
  assert.equal(acknowledgements[1].duplicate, true);
});

test("resume replays actions after a reconnect", async () => {
  const { room, session } = roomHarness();
  await room.handleMessage(session, JSON.stringify({ type: "join", playerId: "p1" }));
  await room.handleMessage(session, JSON.stringify({ type: "action", actionId: "a1", payload: { n: 1 } }));
  const resumed = { socket: { send(value) { resumed.messages.push(JSON.parse(value)); } }, playerId: "p1", messages: [] };
  room.sessions.set(resumed.socket, resumed);
  await room.handleMessage(resumed, JSON.stringify({ type: "resume", lastSequence: 0 }));
  const replay = resumed.messages.find((message) => message.type === "replay");
  assert.equal(replay.actions.length, 1);
  assert.equal(replay.actions[0].actionId, "a1");
});
