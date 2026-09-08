// PeerJS-compatible connection facade for the authoritative lobby Worker.
// Keeping this small adapter separate lets the existing validation/resync
// pipeline consume WebSocket messages without depending on browser globals in
// tests or leaking transport details into the game engine.

export function createWebSocketConnection(url, { peer = "realtime", WebSocketImpl = globalThis.WebSocket } = {}) {
  if (typeof WebSocketImpl !== "function") {
    throw new Error("WebSocket is not available in this environment");
  }
  const listeners = new Map();
  const socket = new WebSocketImpl(url);
  const connection = {
    peer,
    open: false,
    reliable: true,
    serialization: "json",
    socket,
    on(event, handler) {
      if (typeof handler !== "function") return connection;
      const handlers = listeners.get(event) || new Set();
      handlers.add(handler);
      listeners.set(event, handlers);
      return connection;
    },
    off(event, handler) {
      listeners.get(event)?.delete(handler);
      return connection;
    },
    send(message) {
      if (socket.readyState !== WebSocketImpl.OPEN) {
        throw new Error("WebSocket connection is not open");
      }
      socket.send(JSON.stringify(message));
    },
    close() {
      socket.close();
    },
  };
  const emit = (event, ...args) => {
    for (const handler of listeners.get(event) || []) handler(...args);
  };
  socket.addEventListener("open", () => {
    connection.open = true;
    emit("open");
  });
  socket.addEventListener("message", (event) => {
    let message;
    try {
      message = typeof event.data === "string" ? JSON.parse(event.data) : event.data;
    } catch {
      emit("error", new Error("Invalid JSON from realtime lobby"));
      return;
    }
    emit("data", message);
  });
  socket.addEventListener("error", (event) => emit("error", event));
  socket.addEventListener("close", (event) => {
    connection.open = false;
    emit("close", event);
  });
  return connection;
}
