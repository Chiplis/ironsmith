import test from "node:test";
import assert from "node:assert/strict";
import { createHash, webcrypto } from "node:crypto";
import { sha256Hex, publicCheckpointHash, createAuditSessionKey } from "../src/lib/multiplayer-audit.js";

test("HTTP SHA-256 matches standard vectors, padding boundaries, Unicode, and binary views", async () => {
  for (const value of ["", "abc", "a".repeat(1_000_000), "🌳á\u0000".repeat(100), ...[55, 56, 63, 64, 65, 119, 120].map((size) => "x".repeat(size))]) {
    assert.equal(await sha256Hex(value, {}), createHash("sha256").update(value).digest("hex"));
  }
  const bytes = new Uint8Array([0, 128, 255, 13, 42]);
  for (const value of [bytes.buffer, bytes.subarray(1, 4), new DataView(bytes.buffer, 1, 3)]) {
    assert.equal(await sha256Hex(value, {}), await sha256Hex(value, webcrypto));
  }
});

test("HTTP checkpoint hashes match HTTPS; audit signing still requires WebCrypto", async () => {
  const checkpoint = { players: [{ id: 0, life: 20 }], battlefield: [], turn: 1 };
  assert.equal(await publicCheckpointHash(checkpoint, {}), await publicCheckpointHash(checkpoint, webcrypto));
  await assert.rejects(createAuditSessionKey({}), /WebCrypto subtle API is unavailable/);
});
