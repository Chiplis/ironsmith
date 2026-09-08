import test from "node:test";
import assert from "node:assert/strict";
import { LobbyRoom } from "../src/index.js";

test("worker source exposes an authoritative lobby object", async () => {
  assert.equal(typeof LobbyRoom, "function");
  assert.ok(LobbyRoom.prototype.handleMessage);
  assert.ok(LobbyRoom.prototype.replay);
});
