# ironsmith-wasm

Browser-ready WebAssembly bindings for the Ironsmith Magic: The Gathering engine.

This is the lean Ironsmith build: engine code is included, but the global card registry is not. A host application supplies the rules data for the cards it needs, which keeps the package substantially smaller and lets applications such as Manabrew use their existing deck data.

## Install

Pin an exact version while the public API is pre-1.0:

```sh
npm install --save-exact ironsmith-wasm@0.1.0
```

## Initialize

`wasm-pack`'s web target resolves the adjacent `.wasm` file when `init()` is called. Vite and other modern ESM bundlers understand this pattern.

```ts
import init, { WasmGame } from "ironsmith-wasm";

await init();
const engine = new WasmGame();
```

If your host manages WASM assets itself, the binary is also exported as `ironsmith-wasm/ironsmith_bg.wasm`; pass its URL, bytes, response, or compiled module to `init`.

## Load a Manabrew deck

Manabrew deck cards can be registered directly before validation or match startup:

```ts
const registration = engine.registerManabrewDeckSources([aliceDeck, bobDeck]);
if (registration.failed.length > 0) {
  throw new Error(JSON.stringify(registration.failed));
}

const validation = engine.validateManabrewMatchConfig(matchConfig);
const initialState = engine.startManabrewMatch(matchConfig);
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

- `registerManabrewDeckSources(decks)`
- `validateManabrewMatchConfig(config)`
- `startManabrewMatch(config)`
- `manabrewView(viewer?)`
- `manabrewRespond(player, promptId, output)`
- `manabrewApplyDirective(player, directive)`

The generated TypeScript declaration file documents the complete `WasmGame` API.
