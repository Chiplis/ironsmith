import test from "node:test";
import assert from "node:assert/strict";
import { realtimeLobbyUrl } from "../src/hooks/peer-lobby/realtime-config.js";

test("builds a scoped WebSocket lobby URL", () => {
  assert.equal(
    realtimeLobbyUrl("abc 123", "wss://realtime.example.com"),
    "wss://realtime.example.com/lobby?id=abc+123"
  );
});

test("rejects non-WebSocket endpoints", () => {
  assert.equal(realtimeLobbyUrl("abc", "https://example.com"), "");
});
