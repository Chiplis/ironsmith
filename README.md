# Ironsmith

Ironsmith is a Magic: The Gathering rules engine, oracle-text compiler, and browser-playable game runtime built primarily in Rust. You can try it out (and even play with your friends!) at [https://chiplis.com/ironsmith](https://chiplis.com/ironsmith)

The project is organized around one central idea: card behavior should come from parsed rules text whenever possible, not from hand-written one-off logic. That design shows up everywhere in the codebase:

- card definitions are funneled through a parser and lowering pipeline
- effects are represented as structured runtime executors
- game actions emit typed events
- replacement, prevention, triggers, and state-based checks consume those events
- rule evaluation is split into reusable subsystems instead of being hidden inside a single game loop

The result is a codebase that is useful both as a playable engine and as a tooling platform for parser iteration, semantic audits, and registry generation.

## What This Repository Contains

At a high level, this repo contains:

- a Rust engine crate in [`src/`](src/)
- a CLI package that exposes the interactive `ironsmith` binary in [`crates/ironsmith-cli/`](crates/ironsmith-cli/)
- a tooling package with parser/report/audit binaries in [`crates/ironsmith-tools/`](crates/ironsmith-tools/)
- a wasm bridge and browser UI in [`src/wasm_api.rs`](src/wasm_api.rs) and [`web/ui/`](web/ui/)
- Python and shell scripts that generate registries, stream Scryfall data, and rebuild frontend artifacts in [`scripts/`](scripts/) and [`rebuild-wasm.sh`](rebuild-wasm.sh)

## Architecture Overview

Ironsmith is easiest to understand as four cooperating layers:

1. Card text ingestion and compilation
2. Runtime execution of effects and abilities
3. Event-driven replacement, prevention, and trigger handling
4. Rule enforcement through combat, damage, state-based actions, and the game loop

The sections below focus on the subsystems that matter most when working on the engine.

## Parser and Card Compilation Pipeline

The parser is the front door for most of the engine.

The entry point most tooling uses is `CardDefinitionBuilder::parse_text(...)` in [`src/cards/builders.rs`](src/cards/builders.rs). From there, the pipeline looks roughly like this:

1. Raw card text is tokenized and split into lines, clauses, and sentence fragments.
2. The parser builds a card AST describing static abilities, triggered abilities, activated abilities, modal text, additional costs, and statement effects.
3. The AST is normalized into a form that is easier to lower consistently.
4. Reference analysis resolves words like “it”, “that creature”, “that player”, and tagged objects produced by earlier effects.
5. Effects and triggers are lowered into runtime engine structures such as `Effect`, `Trigger`, `StaticAbility`, `OptionalCost`, and `AlternativeCastingMethod`.
6. The compiled result is finalized into a `CardDefinition`.

The main hubs for that pipeline are:

- [`src/cards/builders.rs`](src/cards/builders.rs): public builder surface and parser entrypoints
- [`src/cards/builders/effect_pipeline.rs`](src/cards/builders/effect_pipeline.rs): `parse_text_with_annotations`, normalization, and lowering orchestration
- [`src/cards/builders/card_ast.rs`](src/cards/builders/card_ast.rs): parsed AST types
- [`src/cards/builders/parse_parsing/`](src/cards/builders/parse_parsing): tokenization, clause parsing, sentence parsing, filters, targets, and rule dispatch
- [`src/cards/builders/reference_model.rs`](src/cards/builders/reference_model.rs): import/export model for pronoun and tag resolution across effect sequences
- [`src/cards/builders/parse_compile.rs`](src/cards/builders/parse_compile.rs): lowering from parser AST into runtime triggers/effects/static abilities

### Parser Design Notes

Several project choices are worth calling out because they strongly shape how parser work gets added:

- The parser is rule-index driven, not a giant chain of ad hoc `if` statements. [`src/cards/builders/parse_parsing/rule_engine.rs`](src/cards/builders/parse_parsing/rule_engine.rs) defines reusable keyed rule tables with priorities and diagnostics for unsupported patterns.
- Reference tracking is explicit. The `ReferenceEnv`, `ReferenceImports`, and `ReferenceExports` model lets a sequence like “destroy target creature. Its controller loses 2 life.” carry meaning across clauses without fragile string hacks.
- The pipeline distinguishes parsing from lowering. That makes it easier to inspect and normalize parser output before committing to runtime structures.
- Unsupported content can be preserved intentionally. `parse_text_allow_unsupported(...)` and parser annotations are there so tooling can keep moving while coverage improves.

### Hand-Written Definitions Still Go Through The Parser

Even the hand-maintained cards in [`src/cards/definitions/`](src/cards/definitions) are meant to stay on the parser path.

[`build.rs`](build.rs) enforces a boundary that prevents definitions from bypassing the text compiler and directly hardcoding most effect/ability wiring. In practice that means handwritten definition files are primarily for metadata plus oracle text, while runtime behavior should still come from the same parse/normalize/lower flow used for generated cards.

That boundary is important because it keeps the engine honest: parser coverage improves only if real cards actually depend on it.

### Generated Registry and `cards.json`

The repo expects a local, gitignored [`cards.json`](cards.json) Scryfall-style dump for bulk parsing and registry generation.

- [`scripts/stream_scryfall_blocks.py`](scripts/stream_scryfall_blocks.py) streams playable cards as parser input blocks.
- [`scripts/generate_baked_registry.py`](scripts/generate_baked_registry.py) converts bulk card data into generated Rust source used by the registry build.
- [`build.rs`](build.rs) generates a stub registry unless the `generated-registry` feature is enabled.

This keeps normal engine iteration fast while still supporting “compile the world” style workflows when needed.

## Events System

Ironsmith’s event system is the bridge between action execution and rules processing.

The central pieces are:

- [`src/events/mod.rs`](src/events/mod.rs): typed event modules and the `Event` wrapper
- [`src/events/traits.rs`](src/events/traits.rs): `GameEventType`, `ReplacementMatcher`, and `EventKind`
- [`src/events/raw_event.rs`](src/events/raw_event.rs): shared event envelope used by replacement and trigger pipelines
- [`src/event_processor.rs`](src/event_processor.rs): replacement processing and event application

### How Events Work

Each event type implements `GameEventType`. Examples include:

- damage events
- zone changes and enters-the-battlefield events
- card draw and discard events
- life gain and loss events
- counter movement events
- spell cast / spell copied / ability activated events
- combat and phase-step events

Events are wrapped in `RawEvent`, which adds provenance metadata and gives both the trigger system and replacement system a shared envelope to work with.

That shared envelope matters because the engine often needs to answer questions like:

- what happened?
- who was affected?
- what object or player was involved?
- what last-known-information snapshot should matching use?
- which source action caused this event?

### Why The Event Model Matters

The event layer is doing more than logging. It is the coordination point for:

- replacement effects
- prevention effects
- trigger matching
- UI/provenance tracing
- “what happened this turn” bookkeeping

Because events are strongly typed and categorized with `EventKind`, the engine can avoid a lot of brittle special-case coupling between effect execution and downstream rules logic.

## Effects System

The effects system is the executable vocabulary of the engine.

Core files:

- [`src/effect.rs`](src/effect.rs): the `Effect` wrapper, IDs, values, and outcomes
- [`src/effects/mod.rs`](src/effects/mod.rs): modular effect organization and exports
- [`src/effects/executor_trait.rs`](src/effects/executor_trait.rs): the `EffectExecutor` trait
- [`src/executor.rs`](src/executor.rs): runtime execution context and effect dispatch helpers

### Effect Model

An effect in Ironsmith is not just “do a thing.” It carries enough structure to support:

- target requirements
- dynamic values like `X`
- conditional “if you do” follow-ups
- tagging objects and players for later clauses
- choice prompts and modal branching
- cost execution
- emitted game events
- structured outcomes that later effects can inspect

Execution returns an `EffectOutcome`, which combines:

- control-flow status
- structured payloads
- emitted triggerable events
- non-triggerable execution facts

That outcome model is what allows the engine to express patterns like:

- “destroy target creature. If you do, draw a card.”
- “you may”
- “choose one or more”
- “for each”
- “unless”
- “repeat this process”

without collapsing everything into handwritten one-off spell logic.

### Modular Effect Families

Effect implementations are grouped by domain in [`src/effects/`](src/effects):

- `cards/`: draw, mill, discard, reveal, search, surveil, scry
- `combat/`: PT changes, fight, damage prevention, goad, enter attacking
- `composition/`: sequencing, conditionals, loops, tags, votes, choice orchestration
- `counters/`: add/remove/move/proliferate counters
- `damage/`: direct damage, redirection, prevention shields
- `life/`: gain, lose, exchange, set life totals
- `mana/`: add mana, pay mana, choose colors, commander color identity support
- `permanents/`: tap/untap, transform, regenerate, ninjutsu, soulbond, renown, saddle
- `player/`: extra turns, monarch, energy, poison, “you win/lose the game”, casting permissions
- `replacement/`, `delayed/`, `continuous/`, `stack/`, `tokens/`, `zones/`

The `composition` family is especially important. A lot of “real card text” complexity is not in primitive verbs like damage or draw, but in how smaller actions get stitched together. That is where effects like `SequenceEffect`, `ConditionalEffect`, `MayEffect`, `ForEachTaggedEffect`, `UnlessPaysEffect`, and `ReflexiveTriggerEffect` become the real grammar of the engine.

## Replacement and Trigger Flow

Relevant files:

- [`src/replacement.rs`](src/replacement.rs)
- [`src/replacement_ability_processor.rs`](src/replacement_ability_processor.rs)
- [`src/triggers/mod.rs`](src/triggers/mod.rs)
- [`src/triggers/check.rs`](src/triggers/check.rs)

Ironsmith uses typed matchers for both triggers and replacement effects:

- replacement matchers decide whether an event is modified, prevented, redirected, or replaced
- trigger matchers decide whether an event should enqueue triggered abilities

`event_processor.rs` implements a rules-aware replacement loop modeled around MTG rules 614-616:

- find applicable replacement effects
- sort by priority
- let the affected player choose when multiple effects are tied
- apply one effect at a time
- prevent one-shot replacements from reapplying indefinitely

This is the part of the engine that turns a simple event like “object would enter the battlefield” into more realistic outcomes such as:

- enters tapped
- enters with counters
- enters as a copy
- discard/pay-life or redirect interactions
- “instead” effects that replace the event with a new effect sequence

## Rules System

The rules layer is where executable effects meet broader game legality and state maintenance.

Core files:

- [`src/rules/mod.rs`](src/rules/mod.rs)
- [`src/rules/combat.rs`](src/rules/combat.rs)
- [`src/rules/damage.rs`](src/rules/damage.rs)
- [`src/rules/state_based.rs`](src/rules/state_based.rs)
- [`src/game_loop/mod.rs`](src/game_loop/mod.rs)

### Combat Rules

The combat rules module handles legality and combat-specific heuristics such as:

- attack/block restrictions
- flying, reach, shadow, horsemanship, fear, intimidate, skulk
- menace and minimum blockers
- protection-based blocking failures
- “can’t block”, “can’t be blocked”, and related restrictions

The module works with calculated characteristics, not just printed card state, so static abilities and continuous effects can change combat outcomes correctly.

### Damage Rules

The damage subsystem handles keyword-sensitive damage processing:

- deathtouch
- lifelink
- infect
- wither
- trample excess calculations

This layer is intentionally separated from raw effect execution so the rest of the engine can ask consistent questions like “what does 3 damage from this source actually mean?”

### State-Based Actions

[`src/rules/state_based.rs`](src/rules/state_based.rs) checks and applies state-based actions such as:

- lethal damage and zero toughness deaths
- planeswalkers with zero loyalty
- players losing for life, poison, or commander damage
- legend rule enforcement
- Auras or Equipment falling off
- token/copy cleanup
- counter annihilation
- saga sacrifice
- commander command-zone handling

This is a major part of what makes the engine feel like MTG instead of just a spell resolver.

### The Game Loop

The game loop in [`src/game_loop/`](src/game_loop) integrates:

- priority passing
- casting and activation decisions
- stack resolution
- target selection
- state-based action checks
- triggered ability queuing
- combat steps and combat damage
- turn advancement

The rules modules are deliberately separate, but this is where they get composed into an actual playable game.

## Project Structure

Here is the most important repository structure at a glance:

### Engine Core

- [`src/lib.rs`](src/lib.rs): public engine surface and re-exports
- [`src/game_state.rs`](src/game_state.rs): authoritative mutable game state
- [`src/game_loop/`](src/game_loop): stack, priority, combat, targeting, and turn execution
- [`src/turn.rs`](src/turn.rs): phase/step sequencing and priority helpers
- [`src/decision.rs`](src/decision.rs) and [`src/decisions/`](src/decisions): decision interfaces and decision-context payloads

### Parser and Card Definition Stack

- [`src/cards/builders.rs`](src/cards/builders.rs): builder API plus parser/compiler pipeline entrypoints
- [`src/cards/builders/`](src/cards/builders): parser, normalization, lowering, reference analysis
- [`src/cards/definitions/`](src/cards/definitions): handwritten metadata/oracle sources
- [`src/cards/tokens/`](src/cards/tokens): built-in token definitions
- [`src/compiled_text/`](src/compiled_text): oracle-like rendering of compiled output for audit and comparison tooling

### Runtime Semantics

- [`src/effects/`](src/effects): modular effect executors
- [`src/events/`](src/events): typed event definitions and matchers
- [`src/triggers/`](src/triggers): modular trigger matchers
- [`src/rules/`](src/rules): combat, damage, and SBAs
- [`src/static_abilities/`](src/static_abilities): static ability representation and helpers
- [`src/targeting/`](src/targeting): targeting validation, ward, and legal-target computations

### Frontend and Wasm

- [`src/wasm_api.rs`](src/wasm_api.rs): wasm-facing game wrapper
- [`web/ui/`](web/ui): React/Vite UI for browser play and inspection
- [`web/wasm_demo/`](web/wasm_demo): lightweight wasm demo output
- [`pkg/`](pkg): generated wasm package artifacts

### Tooling and Reports

- [`crates/ironsmith-cli/`](crates/ironsmith-cli): package that exposes the interactive CLI binary
- [`crates/ironsmith-tools/`](crates/ironsmith-tools): package containing parser and audit binaries
- [`src/bin/`](src/bin): source for the binaries listed below
- [`scripts/`](scripts): Python helpers for Scryfall streaming and registry generation
- [`reports/`](reports): generated parser/error/cluster reports

## Available Binary Utilities

The repo ships several useful binaries. Most of them live in the `ironsmith-tools` package; the interactive game CLI lives in `ironsmith-cli`.

### Interactive CLI

- `ironsmith`
  - Package: `ironsmith-cli`
  - Purpose: launch an interactive two-player game in the terminal, with optional custom hands/decks/battlefield setup

### Parser Inspection and Conversion

- `compile_oracle_text`
  - Package: `ironsmith-tools`
  - Purpose: parse card text and print compiled/oracle-like output, optionally with traces or raw debug output

- `parse_card_text`
  - Package: `ironsmith-tools`
  - Purpose: batch-parse `Name: ...` card blocks from stdin and summarize failures, error buckets, and pattern matches

- `export_compiled_oracle_csv`
  - Package: `ironsmith-tools`
  - Purpose: export CSVs comparing source oracle text against compiled oracle-like output

### Audit and Coverage Utilities

- `audit_compiled_cards`
  - Package: `ironsmith-tools`
  - Purpose: inspect compiled output for parse failures, unimplemented markers, and object-filter usage

- `audit_oracle_clusters`
  - Package: `ironsmith-tools`
  - Purpose: cluster oracle text, compare parser output semantically, and produce JSON/CSV audits for large card sets

- `audit_parsed_mechanics`
  - Package: `ironsmith-tools`
  - Purpose: tally which mechanics and fallback reasons appear across parsed cards

- `audit_unimplemented_partition`
  - Package: `ironsmith-tools`
  - Purpose: analyze a subset/partition of cards that still contain unimplemented or fallback content

- `report_replacement_effect_parse_status`
  - Package: `ironsmith-tools`
  - Purpose: produce a focused parse-status report for replacement-effect-heavy cards

- `dump_false_positive_texts`
  - Package: `ironsmith-tools`
  - Purpose: dump oracle text and compiled output for a list of suspected semantic false positives

### Report Rebuild and Data Export

- `rebuild_reports`
  - Package: `ironsmith-tools`
  - Purpose: orchestrate parser report regeneration, including semantic audit artifacts and cluster/error CSVs

- `export_cedh_support_report`
  - Package: root `ironsmith` crate
  - Purpose: fetch cEDH event/deck data and generate support coverage reports for popular cards
  - Notes: requires the `tooling` feature

## Helper Scripts

Not everything in the repo is a Rust binary. A few non-binary helpers are part of the normal workflow:

- [`rebuild-wasm.sh`](rebuild-wasm.sh): rebuild wasm artifacts, refresh semantic score caches, and sync frontend assets
- [`scripts/stream_scryfall_blocks.py`](scripts/stream_scryfall_blocks.py): stream playable Scryfall entries into parser-friendly blocks
- [`scripts/generate_baked_registry.py`](scripts/generate_baked_registry.py): generate the parser-backed registry used when `generated-registry` is enabled
- [`web/ui/scripts/peer-server.mjs`](web/ui/scripts/peer-server.mjs): local PeerJS signaling server for multiplayer UI development

## Development Workflow

### Requirements

You will typically want:

- Rust/Cargo
- Python 3
- `wasm-pack` for wasm builds
- `pnpm` for the React UI
- a local `cards.json` dump for bulk parsing, registry generation, and several audit tools

`cards.json` is intentionally gitignored, along with most generated reports and CSV/JSON outputs.

### Common Commands

Run the interactive CLI:

```bash
cargo run -p ironsmith-cli --bin ironsmith --
```

Probe the parser for a single card:

```bash
cargo run -p ironsmith-tools --bin compile_oracle_text -- \
  --name "Lightning Bolt" \
  --text $'Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target.'
```

Batch-parse card blocks from `cards.json`:

```bash
python3 scripts/stream_scryfall_blocks.py --cards cards.json \
  | cargo run -p ironsmith-tools --bin parse_card_text --
```

Regenerate wasm artifacts and semantic caches:

```bash
./rebuild-wasm.sh --threshold 0.99
```

Run the web UI:

```bash
cd web/ui
pnpm install
pnpm dev
```

Run the local multiplayer signal server:

```bash
cd web/ui
pnpm signal
```

Run tests:

```bash
cargo test
```

## Features

Important Cargo features from [`Cargo.toml`](Cargo.toml):

- `serialization`: enables serde-based serialization support
- `tooling`: enables tooling-oriented binaries and support paths
- `generated-registry`: generates and bakes the parser-backed registry from `cards.json`
- `wasm`: enables the wasm API and generated-registry-backed browser build
- `engine-integration-tests`: enables larger engine integration test coverage
- `parser-tests` / `parser-tests-full`: parser-focused test gates

## Frontend Notes

The browser UI is a React/Vite app that talks to the engine through wasm exports. The wasm-facing bridge lives in [`src/wasm_api.rs`](src/wasm_api.rs), where game state is converted into UI-friendly snapshots and grouped battlefield representations.

The frontend package is in [`web/ui/package.json`](web/ui/package.json). It includes:

- `pnpm dev`
- `pnpm build`
- `pnpm preview`
- `pnpm lint`
- `pnpm signal`

The existing [`web/ui/README.md`](web/ui/README.md) also has notes on PeerJS signaling configuration for multiplayer development.

## Why This Project Is Structured This Way

Ironsmith is doing two jobs at once:

1. It is a game engine that must execute complicated MTG interactions correctly.
2. It is a parser and tooling platform that needs feedback loops for coverage, diagnostics, and semantic comparison.

That is why the repo contains both:

- “play the game” code such as `game_loop`, `rules`, `events`, and `effects`
- “improve the compiler” code such as `compiled_text`, parser audits, report rebuilders, cluster analysis, and generated-registry tooling

If you are new to the codebase, the most productive reading order is usually:

1. [`src/cards/builders.rs`](src/cards/builders.rs)
2. [`src/cards/builders/effect_pipeline.rs`](src/cards/builders/effect_pipeline.rs)
3. [`src/effect.rs`](src/effect.rs)
4. [`src/events/mod.rs`](src/events/mod.rs)
5. [`src/event_processor.rs`](src/event_processor.rs)
6. [`src/rules/state_based.rs`](src/rules/state_based.rs)
7. [`src/game_loop/mod.rs`](src/game_loop/mod.rs)
