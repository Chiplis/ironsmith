# Mechanics Implementation Guide

This document defines how we implement mechanics in code so behavior stays general, testable, and reusable.

## 1. Ground Rules

1. A mechanic must be represented in engine primitives, not in card-specific glue.
2. If one mechanic appears on many cards, treat it as shared structure:
   - parser-level detection
   - AST lowering into reusable types
   - runtime evaluation via existing effect and cost primitives
   - text rendering via generic compaction/normalization
3. Prefer `--raw` correctness over renderer string matching.
4. Don’t add parsing exceptions that hide real semantics issues.
5. Avoid card-specific branches in runtime checks; if a mechanic fails on one card, implement against general rule behavior.
6. If a mechanic is reminder-text-only or non-rule text in Oracle, do not model it in game logic.
7. When behavior can be represented as:
   - cost predicate
   - timing limit
   - legality filter
   - target/choice requirement
   - existing effect execution
   compose those primitives instead of inventing a new mechanism.

## 2. Mechanic Taxonomy

1. Static mechanic
   - Ongoing state effect, usually zone-wide or object-wide.
   - Represent as `StaticAbility`/core static ability IDs where possible.
2. Activated mechanic
   - Requires a player action + cost + possible targets.
   - Represent as `AbilityKind::Activated` with:
     - `mana_cost`
     - `effects`
     - `choices`
     - `timing`
     - `additional_restrictions` (fallback text constraints)
3. Triggered mechanic
   - Event-driven with optional intervening-if semantics.
   - Prefer `TriggeredAbility` with `intervening_if` over ad-hoc state flags.
4. Replacement / replacement-like mechanic
   - Must use or compose replacement-style primitives, not special-case execution paths.

## 3. Preferred Build Pipeline

1. Parse source text to grammar-level artifact.
2. Lower to existing AST/effect primitives.
3. Attach mechanic semantics using:
   - standard filters/values
   - standard flags (timing/restriction/caps)
4. Generate compiled abilities via existing constructors.
5. Assert with:
   - `--raw` contains equivalent semantic structure
   - rendered text can be normalized without dropping required gameplay text

## 4. Composition Requirements

1. Use existing primitives first:
   - `IfThen`, `Unless`, `Choose`, `ForEach`, `May`, `Filter`, value ops, counters/effects, targeting specs.
2. New primitive is justified only when:
   - existing primitives cannot express the rule accurately
   - composition would fail timing/continuity/copying/state interactions
   - mechanic has at least two independent future-relevant cards
3. Runtime checks should be mechanical and reusable, not per-card string matching.
4. Keep representation compact and canonical:
   - If mechanic has multiple textual variants, normalize them during parse/compile.

## 5. Activation-Capacity and Restrictions

1. Use `ActivationTiming` for standard intervals (sorcery-speed, once per turn, combat windows, etc.).
2. Use `additional_restrictions` for orthogonal constraints that don’t deserve new primitive fields.
3. Prefer declarative restriction predicates over hand-written UI-only checks.
4. Every non-trivial restriction should be covered by:
   - compile path tests
   - legality/action gating tests
   - raw ability snapshots

## 6. Reminder Text Policy

1. Reminder text is not a mechanic by itself.
2. Mechanics with reminder text still need a modeled rule object; reminder text can be ignored in semantic comparison.
3. For `--raw`, only model game-relevant behavior.

## 7. Validation Cadence

1. For every implemented mechanic change:
   - add a parser lowerings test (success + failure paths)
   - add semantic test on representative card examples
   - verify via raw+rendered diff on oracle example
   - regenerate semantic report files with:
     `cargo run --quiet --no-default-features --bin rebuild_reports -- --threshold 0.99`
2. For cluster work, measure impact by true mismatch movement, not by moving cards into failure buckets.
3. Do not use `./rebuild-wasm.sh` for report-only iterations; reserve it for when wasm package artifacts are required.

## 8. Implementation Example: Boast in Current Engine

Boast is currently implemented as a composition of existing activated-ability primitives:

1. Parse path marks the line as a labeled activated ability (`Boast` label preserved for text output).
2. Core behavior uses a normal activated ability (`AbilityKind::Activated`) with parsed effects and costs.
3. “Activate only…” constraints are handled by existing timing + restriction wiring.
4. No custom per-card Boast execution opcode is used.

This means:
- it is generic activation plumbing, not a special-case card branch
- reminder text is not treated as standalone semantics
