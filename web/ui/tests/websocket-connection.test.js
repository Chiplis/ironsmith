import test from "node:test";
import assert from "node:assert/strict";
import { createWebSocketConnection } from "../src/hooks/peer-lobby/websocket-connection.js";

class FakeWebSocket {
  static OPEN = 1;
  constructor() { this.readyState = 0; this.handlers = new Map(); this.sent = []; }
  addEventListener(event, handler) { this.handlers.set(event, handler); }
  open() { this.readyState = FakeWebSocket.OPEN; this.handlers.get("open")?.(); }
  receive(data) { this.handlers.get("message")?.({ data }); }
  send(value) { this.sent.push(value); }
  close() { this.readyState = 3; this.handlers.get("close")?.({ code: 1000 }); }
}

test("WebSocket facade exposes PeerJS-like open, data, send and close events", () => {
  let socket;
  const connection = createWebSocketConnection("wss://example.test/lobby?id=x", {
    WebSocketImpl: class extends FakeWebSocket { constructor(url) { super(); socket = this; this.url = url; } },
    peer: "p1",
  });
  const received = [];
  connection.on("data", (message) => received.push(message));
  socket.open();
  assert.equal(connection.open, true);
  connection.send({ type: "ping" });
  socket.receive('{"type":"pong"}');
  assert.deepEqual(received, [{ type: "pong" }]);
  assert.deepEqual(JSON.parse(socket.sent[0]), { type: "ping" });
  connection.close();
  assert.equal(connection.open, false);
});
