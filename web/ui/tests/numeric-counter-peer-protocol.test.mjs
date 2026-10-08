import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { CURRENT_AUDIT_PROTOCOL_VERSION } from "../src/lib/multiplayer-audit.js";

// Authored, unrun. Extract the production admission callbacks without React,
// transport, or engine setup, following the peer-message harness convention.
const messaging = readFileSync(new URL("../src/hooks/peer-lobby/messaging.js", import.meta.url), "utf8");
const shared = readFileSync(new URL("../src/hooks/peer-lobby/shared.js", import.meta.url), "utf8");
const protocolDeclaration = shared.match(/^export const PROTOCOL_VERSION = [^;]+;/m)?.[0];
assert.ok(protocolDeclaration);
const PROTOCOL_VERSION = new Function("CURRENT_AUDIT_PROTOCOL_VERSION",
  `${protocolDeclaration.replace("export ", "")} return PROTOCOL_VERSION;`)(CURRENT_AUDIT_PROTOCOL_VERSION);

function callback(name, endMarker) {
  const start = messaging.indexOf(`  const ${name} = useCallback(`);
  const bodyStart = messaging.indexOf("async (", start);
  const end = messaging.indexOf(endMarker, bodyStart);
  assert.ok(start >= 0 && bodyStart > start && end > bodyStart, `${name} production callback exists`);
  return messaging.slice(bodyStart, end) + "\n}";
}

function harness(name, endMarker) {
  const calls = [];
  const context = {
    PROTOCOL_VERSION,
    setStatus: (...args) => calls.push(["status", ...args]),
    safeSend: (_conn, message) => calls.push(["send", message]),
    receiveLobbyChat: entry => calls.push(["receive", entry]),
    publishLobbyChat: (peer, text) => calls.push(["publish", peer, text]),
  };
  const handler = new Function(...Object.keys(context), `return (${callback(name, endMarker)});`)(...Object.values(context));
  const conn = { peer: "remote", close: () => calls.push(["close"]) };
  return { calls, receive: message => name === "handleHostMessage" ? handler(message) : handler(conn, message) };
}

const routes = [
  { name: "handleHostMessage", end: "\n    },\n    [", type: "lobby_chat", accepted: ["receive", "entry"],
    rejected: [["status", "Lobby protocol version mismatch", true]] },
  { name: "handlePeerMessage", end: "\n  }, [", type: "lobby_chat", accepted: ["receive", "entry"],
    rejected: [["close"]] },
  { name: "handleClientMessage", end: "\n    },\n    [", type: "lobby_chat_send", accepted: ["publish", "remote", "text"],
    rejected: [["send", { type: "reject", protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
      reason: "Protocol version mismatch" }], ["close"]] },
];

for (const route of routes) {
  test(`${route.name} rejects peers before current compiler/cache semantics can execute`, async () => {
    for (const protocolVersion of [14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, CURRENT_AUDIT_PROTOCOL_VERSION + 1, undefined, null, String(CURRENT_AUDIT_PROTOCOL_VERSION)]) {
      for (const type of [route.type, "match_start", "state_resync", "apply_action", "signed_action_recovery_response", "crypto_material_request"]) {
        const h = harness(route.name, route.end);
        await h.receive({ type, protocolVersion, entry: "entry", text: "text" });
        assert.deepEqual(h.calls, route.rejected);
      }
    }
  });

  test(`${route.name} dispatches current protocol31 through the same admission gate`, async () => {
    assert.equal(PROTOCOL_VERSION, 31);
    const h = harness(route.name, route.end);
    await h.receive({ type: route.type, protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION, entry: "entry", text: "text" });
    assert.deepEqual(h.calls, [route.accepted]);
  });
}
