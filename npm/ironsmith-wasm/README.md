# ironsmith-wasm

Browser-ready WebAssembly bindings for the Ironsmith Magic: The Gathering engine.

This is the lean Ironsmith build: engine code is included, but the global card registry is not. A host application supplies the rules data for the cards it needs, which keeps the package substantially smaller and lets applications such as Manabrew use their existing deck data.

## Install

Pin an exact version while the public API is pre-1.0:

```sh
npm install --save-exact ironsmith-wasm@0.9.1
```

## Initialize

The package facade initializes the adjacent engine and verifier modules. Vite and other modern ESM bundlers understand these module-relative assets.

```ts
import init, { WasmGame } from "ironsmith-wasm";

await init();
const engine = new WasmGame();
```

If your host manages WASM assets itself, the two binaries are exported as `engine_bg.wasm` and `verifier_bg.wasm`. Pass their URLs, bytes, responses, or compiled modules to `init({ engine, verifier })`; `verifier: false` skips the verifier.

Card source is compiled by the engine module itself. Since 0.9.0 the package no longer ships the standalone compiler module (`compiler_bg.wasm`): source-based registration (`registerExternalCardSources`, `registerManabrewDeckSources`, `compileAndRegisterCardSources`) is unchanged, pre-compiled artifacts passed as `source.artifacts` still load, and `compileCardArtifact` / `validateCompiledCardArtifact` / `compileAndRegisterCard` throw.

## Load a Manabrew deck

Manabrew deck cards can be registered directly before validation or match startup:

```ts
const registration = engine.registerManabrewDeckSources([aliceDeck, bobDeck]);
if (registration.failed.length > 0) {
  throw new Error(JSON.stringify(registration.failed));
}

// The adapter speaks manabrew-protocol 6 (crate / @manabrew/protocol 6.5.0).
// Declaring the version is optional; a declared version must match.
if (engine.manabrewProtocolVersion() !== 6) throw new Error("Unsupported Manabrew protocol");
const protocolConfig = { ...matchConfig, protocolVersion: 6 };
const validation = engine.validateManabrewMatchConfig(protocolConfig);
const initialState = engine.startManabrewMatch(protocolConfig);
```

`validateManabrewMatchConfig` and `startManabrewMatch` also register the decks they receive, so calling `registerManabrewDeckSources` separately is optional. It is useful when an application wants to surface compilation failures before building the full match configuration.

Current Manabrew rules summaries contain enough information for single-faced cards. For a split, transform, modal double-faced, or Adventure card, include both faces as `cardFaces` (camel case) or `card_faces` (Scryfall shape):

```ts
const card = {
  identity: { name: "Front Face" },
  layout: "transform",
  cardFaces: [
    {
      name: "Front Face",
      manaCost: "{2}{U}",
      typeLine: "Creature — Wizard",
      power: "2",
      toughness: "2",
      oracleText: "When this enters, draw a card."
    },
    {
      name: "Back Face",
      typeLine: "Legendary Planeswalker — Wizard",
      loyalty: "4",
      oracleText: "+1: Draw a card."
    }
  ]
};
```

Face data accepts the existing Manabrew camel-case fields and Scryfall's `mana_cost`, `type_line`, and `oracle_text` fields. Printed `loyalty` and `defense` are retained in the parser input.

A deck card that carries the protocol's `backFace` summary (`name`, `manaCost`, `typeLine`, `oracleText`, `power`, `toughness`) is also registered as a linked two-face card, with the deck card itself as the front face.

The protocol's deck card has no field for printed loyalty or defense, and neither can be derived from a type line or rules text. The adapter therefore ships a table of them by face name (generated from Scryfall data by `scripts/generate_printed_loyalty.py`) and uses it for any planeswalker or battle face a host leaves without `loyalty` / `defense`; a value the host supplies wins. `manabrewPrintedLoyalty(name)` and `manabrewPrintedDefense(name)` expose the table.

A planeswalker or battle that has neither a supplied value nor a table entry (a card newer than the package, or a custom one) is reported by `validateManabrewMatchConfig` as an issue on that card, and `startManabrewMatch` refuses the deck, since the permanent would be put into the graveyard as it entered.

## Definition precedence

External data fills registry gaps; it does not replace a definition already embedded in a non-lean build or registered earlier in the session. The generic source API supports an explicit escape hatch when replacement is intentional:

```ts
engine.registerExternalCardSources({
  canonicalName: "Example Card",
  replaceExisting: true,
  group: {
    kind: "single",
    name: "Example Card",
    block: "Type: Creature — Example\nPower/Toughness: 2/2"
  }
});
```

Linked cards are registered atomically so their two internal face identifiers cannot be mixed across sources.

## Manabrew protocol surface

The main compatibility methods are:

- `manabrewProtocolVersion()`
- `registerManabrewDeckSources(decks)`
- `validateManabrewMatchConfig(config)`
- `startManabrewMatch(config)`
- `manabrewView(viewer?)`
- `manabrewRespond(player, promptId, output)`
- `manabrewApplyDirective(player, directive)`

The generated TypeScript declaration file documents the complete `WasmGame` API.

The optional Manabrew adapter implements the published `manabrew-protocol`
6.5.0 DTOs unmodified (a pinned crates.io dependency of
`ironsmith-web-session`).
`manabrewProtocolVersion()` returns that crate's major (`6`) and
`manabrewProtocolCrateVersion()` its exact version, which equals the
`@manabrew/protocol` npm `VERSION`. Both setup methods accept a config with no
`protocolVersion` (the published Manabrew adapter sends none) and reject a
declared version that differs, before registering sources. Returned Manabrew
views identify the version as `protocolVersion: 6`.

Adapter behavior worth knowing:

- Views and prompts describe the state the pending decision refers to. While a
  spell or ability is partway through resolving, that is the in-progress state
  (cards just drawn are in hand and can be chosen for a following discard).
- A decision with exactly one possible answer is answered by the adapter and
  never shown: an optional choice with no legal option (a kicker that cannot
  be paid), and an attack or block declaration with nothing to declare.
- A pass with `exhaustStack` or `until` grants the engine's own priority yield:
  the seat keeps passing until the stack empties or the named step arrives,
  and always stops for a required choice, a new stack object, or the end of
  the turn. A destination in a later turn yields for the rest of this one.
- London mulligan bottoming uses the `mulliganPutBack` prompt, and `mulligan`
  prompts report the real `mulliganCount`.
- Trigger ordering uses `reorder` with one item per pending ability; an item
  whose ability has no card of its own carries the source card and the
  ability's text in `oracle`.
- `payManaCost` offers only mana abilities that keep the cost payable.
- The game result is reported through `gameView.gameOver` and `winnerId`; the
  `gameOver` prompt is not sent.

Protocol features the engine does not implement are refused rather than
ignored: the checkpoint restore, snapshot recording and action repeat
directives return an `invalidShape` error and views carry no checkpoints.
`chooseBoardTargets` prompts are `cancellable` whenever the engine can take
the action chain back, and accept the `cancel` output then.

The package also exports `ironsmith-wasm/ironsmith_bg.wasm` as an alias of
`engine_bg.wasm`, so a host written against the single-module 0.1.x layout
(Manabrew imports `ironsmith-wasm/ironsmith_bg.wasm?url` and passes it to
`init({ module_or_path })`) keeps working; the verifier module loads from its
module-relative default location.

The in-repository browser game and multiplayer replay use `startMatch`, native
UI commands, and their own audit protocol. They do not call Manabrew setup or
inherit this adapter's protocol version. Standalone deck registration is also
independent of the Manabrew prompt protocol.
