# ironsmith-wasm

Browser-ready WebAssembly bindings for the Ironsmith Magic: The Gathering engine.

This is the lean Ironsmith build: engine code is included, but the global card registry is not. A host application supplies the rules data for the cards it needs, which keeps the package substantially smaller and lets applications such as Manabrew use their existing deck data.

## Install

Pin an exact version while the public API is pre-1.0:

```sh
npm install --save-exact ironsmith-wasm@0.1.0
```

## Initialize

The package facade initializes the adjacent engine, compiler, and verifier modules. Vite and other modern ESM bundlers understand these module-relative assets.

```ts
import init, { WasmGame } from "ironsmith-wasm";

await init();
const engine = new WasmGame();
```

If your host manages WASM assets itself, the three binaries are exported as `engine_bg.wasm`, `compiler_bg.wasm`, and `verifier_bg.wasm`. Pass their URLs, bytes, responses, or compiled modules to `init({ engine, compiler, verifier })`.

The facade preserves source-based registration methods by compiling source in the compiler module and loading versioned typed artifacts into the parser-free engine module. Linked faces are registered as one artifact batch, with their local IDs remapped atomically by each engine session.

## Load a Manabrew deck

Manabrew deck cards can be registered directly before validation or match startup:

```ts
const registration = engine.registerManabrewDeckSources([aliceDeck, bobDeck]);
if (registration.failed.length > 0) {
  throw new Error(JSON.stringify(registration.failed));
}

// The host and every Manabrew peer must implement protocol 3, including
// useResource/releaseResource with resource: "waterbend".
if (engine.manabrewProtocolVersion() !== 3) throw new Error("Unsupported Manabrew protocol");
const protocolConfig = { ...matchConfig, protocolVersion: 3 };
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

The optional Manabrew adapter uses a local version-3 extension of the pinned
upstream protocol. Both setup methods reject a missing or mismatched
`protocolVersion` before registering sources. A host must enable the version
only after confirming every peer implements its typed Waterbend resource;
copying the version number alone does not establish compatibility. Returned
Manabrew views identify the version as `protocolVersion: 3`.

The in-repository browser game and multiplayer replay use `startMatch`, native
UI commands, and their own audit protocol. They do not call Manabrew setup or
inherit this adapter's protocol version. Standalone deck registration is also
independent of the Manabrew prompt protocol.
