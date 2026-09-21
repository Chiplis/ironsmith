import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

// Exercise the messaging callbacks with the same session/transport boundaries
// used by the hook, without starting an engine or signaling server.
const source = readFileSync(new URL("../src/hooks/peer-lobby/messaging.js", import.meta.url), "utf8");
const callbacks = source.slice(source.indexOf("  const receiveLobbyChat ="), source.indexOf("  const handleHostMessage ="));
function harness(role = "host") {
  const multiplayerRef = { current: { role, localPeerId: "a", players: [
    { peerId: "a", name: "Alice" }, { peerId: "b", name: "Bob" },
  ], chatMessages: [] } };
  const sent = [];
  const api = new Function("useCallback", "multiplayerRef", "hostConnectionRef", "updateMultiplayer",
    "broadcastToClients", "safeSend", "PROTOCOL_VERSION", callbacks + "\nreturn { sendLobbyChat, publishLobbyChat, receiveLobbyChat };")(
    (fn) => fn, multiplayerRef, { current: {} },
    (update) => { multiplayerRef.current = update(multiplayerRef.current); },
    (message) => sent.push(message), (_, message) => { sent.push(message); return true; }, 1);
  return { ...api, sent, multiplayerRef };
}
test("host attributes remote messages to their connection and broadcasts once", () => {
  const h = harness();
  assert.equal(h.publishLobbyChat("b", " hello "), true);
  assert.equal(h.sent[0].entry.name, "Bob");
  assert.equal(h.sent[0].entry.text, "hello");
  assert.equal(h.publishLobbyChat("stranger", "spoof"), false);
  h.receiveLobbyChat(h.sent[0].entry);
  assert.equal(h.multiplayerRef.current.chatMessages.length, 1);
});
test("chat rejects empty/oversized messages and bounds retained history", () => {
  const h = harness();
  assert.equal(h.sendLobbyChat(" "), false);
  assert.equal(h.sendLobbyChat("x".repeat(501)), false);
  for (let i = 0; i < 110; i++) h.sendLobbyChat(String(i));
  assert.equal(h.multiplayerRef.current.chatMessages.length, 100);
  assert.equal(h.multiplayerRef.current.chatMessages[0].text, "10");
});
test("clients send to host and wait for canonical echo", () => {
  const h = harness("client");
  assert.equal(h.sendLobbyChat("hello"), true);
  assert.equal(h.sent[0].type, "lobby_chat_send");
  assert.equal(h.multiplayerRef.current.chatMessages.length, 0);
  h.receiveLobbyChat({ id: "1", name: "Alice", text: "hello" });
  assert.equal(h.multiplayerRef.current.chatMessages.length, 1);
});
