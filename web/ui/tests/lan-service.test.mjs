import test from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { createLanService, LAN_PATH } from "../scripts/lan/service.mjs";

test("LAN directory authenticates signaling, hides disconnected hosts, and expires leases", async () => {
  const service = createLanService({ leaseMs: 100 });
  const server = createServer((req, res) => service.middleware(req, res, () => { res.writeHead(404); res.end(); }));
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}${LAN_PATH}`;
  const token = "a".repeat(48);
  const query = `?id=host&token=${token}`;
  const abort = new AbortController();
  try {
    const events = await fetch(`${base}/events${query}`, { signal: abort.signal });
    const reader = events.body.getReader();
    assert.match(new TextDecoder().decode((await reader.read()).value), /"type":"open"/);
    const post = (endpoint, body, suffix = query, headers = {}) => fetch(`${base}/${endpoint}${suffix}`, {
      method: "POST", headers: { "Content-Type": "application/json", ...headers }, body: JSON.stringify(body),
    });
    const lobby = { available: true, name: "Table", format: "commander", desiredPlayers: 4, playerCount: 1 };
    assert.equal((await post("lobby", lobby)).status, 200);
    assert.equal((await (await fetch(`${base}/lobbies`)).json()).lobbies[0].name, "Table");
    assert.equal((await post("lobby", lobby, "?id=host&token=" + "b".repeat(48))).status, 403);
    assert.equal((await post("lobby", lobby, query, { Origin: "https://evil.example" })).status, 403);
    assert.equal((await fetch(`${base}/lobbies`, { headers: { "Sec-Fetch-Site": "cross-site" } })).status, 403);
    assert.equal((await fetch(`${base}/events?id=host&token=${"b".repeat(48)}`)).status, 409);
    assert.equal((await post("signal", { type: "offer", to: "absent", connectionId: "c" })).status, 404);
    assert.equal((await post("signal", { type: "offer", to: "host", from: "forged", connectionId: "c" })).status, 200);
    assert.match(new TextDecoder().decode((await reader.read()).value), /"from":"host"/);
    await post("lobby", { available: false });
    assert.deepEqual((await (await fetch(`${base}/lobbies`)).json()).lobbies, []);
    await post("lobby", lobby);
    abort.abort();
    await new Promise((resolve) => setTimeout(resolve, 250));
    assert.deepEqual((await (await fetch(`${base}/lobbies`)).json()).lobbies, []);
    assert.equal((await post("lobby", lobby)).status, 403);
  } finally {
    abort.abort();
    service.close();
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
});
