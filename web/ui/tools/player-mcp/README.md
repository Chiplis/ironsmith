# Ironsmith player MCP

A local MCP server for playing Ironsmith through the real browser interface.
Each player has a separate browser context, storage, hand, and network identity.
The adapter reads rendered DOM and performs browser clicks, typing, selections,
hovers, and pointer drags. It does not read React stores, expose engine savepoints, inject
commands, choose shuffle seeds, or bypass multiplayer verification.

## Install and run

Install the normal UI dependencies and generated WASM packages first. Then:

```sh
cd web/ui/tools/player-mcp
npm ci
node server.mjs
```

The default transport is MCP over stdio. A client must keep that process alive
to retain its player sessions. For multiple local clients sharing a table:

```sh
node server.mjs --http
```

The HTTP listener is restricted to loopback. The startup message supplies its
MCP endpoint. `cli.mjs` is an MCP client for that endpoint, useful for scripts
and inspecting tools without reconnecting the browsers.

```sh
node cli.mjs --list
node cli.mjs --compact list_decks '{"format":"modern","query":"burn"}'
node setup-game.mjs
```

`setup-game.mjs` prepares two browser players with the catalog's Boros Burn
list, explicitly selects Verified, joins the second seat by the real invite
link, and presses Start game. It prints both player IDs and leaves them open
for an agent to play. It does not choose opening hands or gameplay actions.
Use `--deck CATALOG_ID` to select another list, `--headless` to hide the browser,
and `--url http://127.0.0.1:PORT/mcp` for a different MCP endpoint. A returned
`phase: "starting"` means setup was submitted; inspect both peers to confirm
cryptographic setup succeeded.

For subsequent manual calls, `--compact` prints the tool's parsed JSON and
omits verbose DOM styling. `screenshot --output /absolute/path.png` saves an
image. Normal UI downloads and the tool journal are kept under the evidence
directory reported by `start_local_table`.

## Play a game

1. Call `list_decks` to find a tournament list and `get_deck` to inspect it.
2. Call `start_local_table` for a local UI and PeerJS signaling server, or use
   the URL of an existing Ironsmith deployment.
3. Call `open_player` with the URL, player name, and deck. Headed browsers are
   the default; `headless: true` is also supported. `deckId` imports all deck
   sections, including commanders. For manual imports, supply `deckText` and,
   when needed, `commanderText`; both survive joining a lobby.
4. Use `observe` to read the page. `act` takes a control `ref` and the matching
   `observationId`, plus `click`, `fill`, `select`, `press`, `hover`,
   `pointer_click`, or `drag`.
   If the page changes, inspect the returned fresh observation before choosing
   another action. Never reuse an old action blindly.
5. To host, open **Create Lobby**, select **Verified**, and create the lobby.
   Its **Invite Link** is exposed as the readonly input's value. The default
   lobby mode is Trusted, so selecting Verified is essential.
6. Open a second player and call `join_lobby` with that invite. Complete the
   visible **Join Lobby** form if prompted, then start from the host when the
   **Start game** button becomes enabled.
   A full invite URL selects its deployment; a bare lobby code uses the
   player's current deployment. For a remote code, open that deployment first.
7. Play using the visible decision controls, card actions, targets, and combat
   choices. An agent controlling one seat must observe only that seat; it may
   use public information about the opponent but must not inspect the other
   browser's hand.
8. After a natural game result, inspect both peers' winner and hidden-card
   disclosure status. Use the UI's **Verify Match → Current Match** and
   **Export Match** actions to verify and save the audit transcript.

Hand-card clicks inspect a card. To play a land or permanent, use `drag` with
the card's ref and a `position: {"x": 500, "y": 700}` inside your visible
battlefield. Coordinates are CSS pixels relative to the viewport; choose them
from the current screenshot or the battlefield surface's observed `rect`, not
from this example. For keyboard placement, press Enter on the card, observe
again, then use `pointer_click` with the battlefield surface ref and a point
inside it. The adapter checks that pointer positions are on screen and visible.
It never forces clicks through an overlay. Instants and sorceries may instead
open their normal casting/target decisions on Enter.

Peer-to-peer **Normal** games accept constructed tournament decks. This is
different from the **Tournament match** transport, which needs organizer
certificates and a configured relay. `start_local_table` runs the production
UI and real cryptography; it does not enable the test fixture or E2E hooks.

## Codex connection

Add a stdio entry to your Codex configuration using absolute paths:

```toml
[mcp_servers.ironsmith_player]
command = "/absolute/path/to/node"
args = ["/absolute/path/to/ironsmith/web/ui/tools/player-mcp/server.mjs"]
startup_timeout_sec = 20
tool_timeout_sec = 120
```

The stdio server starts on demand, so a separately running HTTP service is not
required. Reload MCP connections or start a fresh chat after adding it.
See the [official MCP configuration documentation](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).

## Boundaries

This server is a player interface, not a rules oracle or a strategy engine.
An agent still decides how to play from its observations. Screenshots and
hover previews help with visual board state and card text. A successful game
is evidence for that playthrough, not proof that every card or multiplayer
scenario works. Keep game transcripts when reporting a synchronization error.

Tests run from `web/ui`:

```sh
node --test tools/player-mcp/browser.test.mjs tools/player-mcp/transport.test.mjs tests/player-mcp-decks.test.mjs
```

The opt-in network regression starts real Verified games through MCP with the
unaltered tournament deck. It covers guest mulligans under both naturally
selected starting players, checks the redraw and named bottom-card choices,
and waits for both peers to reach the main phase without synchronization errors:

```sh
IRONSMITH_MCP_VERIFIED_GAME=1 node --test tools/player-mcp/verified-game.test.mjs
```

It uses separate ports and browser contexts and saves its journal and failure
evidence under `reports/player-mcp/`. This startup regression does not replace
playing and verifying a complete game.
