# Custom keyword references — implementation plan

Status: proposal, 2026-10-27. Prerequisite: the uncommitted keyword-reminder refactor
(see `reports/keyword-reminder-generic-audit/`) is fixed up and committed first —
the open regressions (duplicate `Enchant` statics from `CardDefinitionBuilder::enchants`,
13 cards that stopped compiling, 64 lib tests vs `HEAD`) must not be built upon.

## Problem

A custom keyword defined by reminder text ("Soar (This creature can't be blocked except by
creatures with flying or reach.)") compiles to the right behavior and carries a separate
identity (`StaticAbilityPayload::CustomKeywordIdentity { name }`). But no other text can
*refer* to it: "creatures with soar", "target creature gains soar", "using its blink
ability" are all rejected (verified: "You may cast this card from your graveyard using its
blink ability." → `unsupported-line-family`). Every reference grammar resolves keyword
names against the fixed official list only.

## Scope: the common families

Corpus scan (all English faces, reminders stripped, basic actions excluded) of places a
keyword name appears outside its declaration:

| # | Family | Corpus occ. | Keywords | In scope |
|---|---|---:|---:|---|
| F1 | has / have / had {kw} — characteristic test | ~2,000 | 109 | yes |
| F2 | gains / gain {kw} — grant | ~2,400 | 56 | yes |
| F3 | with / without {kw} — object filter | ~1,700 | 58 | yes |
| F4 | loses / lose {kw} — removal | 79 | 14 | yes |
| F5 | its {kw} cost / {kw} cost | 74 | 28 | yes |
| F6 | {kw} ability / abilities, using its {kw} ability | 34 | 17 | yes |
| F7 | if it was {kw}ed / {kw}ed spells | 380 | 18 | phase 2 |
| F8 | whenever you {kw} | 33 | 15 | phase 2 |
| — | {kw} counters, for each time you {kw} | 158 | — | out (counter kinds are a closed CR list; tiny) |

F1–F4 cover ~95% of occurrences and share one mechanism. F5–F6 need ability tagging.
F7–F8 need event/history tagging and are a second phase.

## Design

### 1. One keyword reference type

Add to `ironsmith-core`:

```rust
pub enum KeywordRef {
    Native(KeywordIdentity),   // today's official identity (StaticAbilityId, ActivatedAbilityKeyword, …)
    Custom(SharedStr),         // normalized custom keyword name
}
```

`Native` wraps what each family already uses, so official cards lower exactly as before.
Custom never aliases a native identity (the identity rule agreed for the refactor:
"Soar" with Flying's text is not Flying).

### 2. Name resolution in the parser

The compiler currently learns custom keywords only through
`CardDefinitionBuilder::with_custom_keyword` (wasm `undo.rs`), i.e. after declaration
lines. Make the declared set available to parsing:

- Add a `custom_keywords: Vec<CustomKeywordDecl>` field to the compile request /
  `CardDefinitionBuilder` (name + its compiled definition, see §3), populated by
  `web/ui/src/lib/custom-mechanics.js` from the mechanics registry it already maintains
  (`mergeMechanics`), not only from the card being compiled — "creatures with soar" can
  appear on a card that doesn't itself have soar.
- One grammar primitive, `keyword_ref(stream) -> KeywordRef`, tries the official keyword
  table first, then the custom set (longest match, case-insensitive, word-boundary).
  Every family below calls it instead of its own official-name matcher.
- Collision rule stays as in `validateMechanic`: custom names can't shadow official ones,
  so resolution is unambiguous.

### 3. Compiled custom keyword definitions

Granting (F2) needs the *behavior* of a custom keyword, not just its name. Compile each
custom mechanic's expanded definition once into a reusable template:

```rust
pub struct CustomKeywordDefinition {
    pub name: SharedStr,
    pub abilities: Vec<Ability>, // lowered from the expanded reminder, each tagged (§4)
}
```

Lowering a declaration ("Soar") becomes: push `custom_keyword_identity(name)` + the
template abilities. This replaces the current text-substitution route for keyword lines
and gives one artifact the engine can grant, remove and test against.

### 4. Ability provenance tagging

Tag every ability lowered from a custom definition with `origin: KeywordRef::Custom(name)`
(a field on `Ability`, defaulting to `None`; native keyword builders set
`Native(..)` where they already have an identity, e.g. `ActivatedAbilityKeyword::Equip`,
`Fortify`, `PresentationKeyword::Poisonous`). This is the single hook F5/F6 query.

Remember the gotcha fixed this week: `StaticAbility::try_map` once dropped nested
`presentation_label`s — the new field must survive every `try_map` / `map_*` and the
wire/artifact encoders (`ironsmith-compiled-artifact`, `artifact_materializer.rs`).

### 5. Family implementations

**F1 has / F3 with/without** — filters. Today `ObjectFilter` has
`static_abilities: Vec<StaticAbilityId>` and `ability_markers: Vec<String>`. Add
`keywords: Vec<KeywordRef>` / `excluded_keywords` (or migrate `ability_markers` to it).
Matching for `Custom(name)`: the object currently has a `CustomKeywordIdentity { name }`
static (`StaticAbility::custom_keyword_name()` already exists). Conditions ("if it has
soar", "as long as …") reuse the same predicate.

**F2 gains / F4 loses** — `GrantKeywordAction` / `RemoveKeywordAction` gain a
`KeywordRef::Custom` arm that grants/removes the identity static *plus* the definition's
tagged abilities as one unit (layer 6). Removal removes by `origin`, so "loses soar"
removes exactly what "gains soar" added, including the identity.

**F5 its {kw} cost** — the JS binder already rewrites "its X cost" from the declaration
(`bindReminderCostFromDeclaration`). Move the authority to Rust: a cost reference resolves
to the abilities/alternative casts with `origin == Custom(name)` on the source, so text on
*other* cards ("…for its soar cost") works and the price stays in one place.

**F6 {kw} ability / using its {kw} ability** — generalize
`NativeAlternativeCastFromZone { method }` and the "{kw} abilities you activate cost …"
selectors (`cost_modifiers.rs` keyword match) to `KeywordRef`, selecting by `origin`.
Timeline Culler's line then works for custom names unchanged.

**Phase 2 — F7/F8** — keyword-action events (`KeywordActionKind`) and optional-cost refs
(`OptionalCostRef`, e.g. kicked) get a `Custom(name)` variant emitted when a tagged
ability/cost from a custom definition resolves or is paid; "if it was blinked" /
"whenever you blink" query those.

### 6. Rendering

`ironsmith-text` renders `KeywordRef::Custom(name)` with the authored casing
("creatures with Soar" → keep the card's printed word). Add to the keyword-name tables the
renderer consults; no Debug fallback (see runtime-prompt-wording memory).

## Test & audit automation (build first)

1. **Corpus parity harness.** Extend `crates/ironsmith-compiler/examples/audit_keyword_reminders.rs`
   (or a sibling example) to accept `custom_keywords` declarations. Generator script
   `scripts/audit_custom_keyword_references.py`:
   - for each corpus sentence in families F1–F8 referencing official keyword K,
     substitute a custom name (e.g. `Zq` + K) declared with K's selected generic reminder
     (`reports/keyword-reminder-generic-audit/probes.json`);
   - compile both versions; record per family × keyword: compiled?, reference resolved to
     `Custom(name)`?, structure equal to the native compile modulo identity.
   Output `reports/custom-keyword-references/{summary,results}.json` + README like the
   reminder audit. This is the progress tracker and the regression gate; run it before
   starting so phase gains are measured.
2. **Behavior tests** in `crates/ironsmith-compiler-runtime/src/custom_keyword_reference_tests.rs`,
   ≤3 per family (fix-then-test workflow), each asserting both directions of the identity
   rule: a custom-keyword reference matches the custom keyword and *not* the native one
   with identical text, and vice versa.
3. **No-regression gate:** the official-card corpus compile (HEAD vs tree, same harness
   used in the 2026-10-27 risk review) must show zero changed behaviors for cards without
   custom keywords.

## Phases

| Phase | Work | Done when |
|---|---|---|
| 0 | Fix/commit current refactor; build the parity harness; record baseline | baseline report exists |
| 1 | `KeywordRef`, parser resolution, `CustomKeywordDefinition`, ability `origin` tagging (+ encoders) | official corpus diff = 0; reminder_identity 149/149 |
| 2 | F1 + F3 (filters/conditions) | harness: F1/F3 ≥ 95% of sampled sentences resolve |
| 3 | F2 + F4 (grant/remove as a unit) | grant→remove round-trip tests pass |
| 4 | F5 + F6 (cost and ability references) | Timeline-Culler-style custom line compiles and only enables the custom method |
| 5 | Rendering + UI (custom-mechanics.js passes registry, not text substitution) | compiled text shows custom names |
| 6 (later) | F7 + F8 events/history | harness F7/F8 |

## Risks

- **Shared-name collisions across sets:** two custom sets defining "Soar" differently.
  Resolution must be scoped to the game's mechanics registry; `mechanicConflict` already
  rejects conflicting definitions within one registry.
- **Hidden-information / multiplayer determinism:** the custom registry must be part of
  the agreed game setup (hashed like decks), or peers can compile different behavior.
- **Encoder drift:** new fields on `Ability`/`ObjectFilter` must round-trip through the
  compiled-artifact wire format; add golden tests.
- **Grant scope:** granting a custom keyword whose definition includes casting-time
  abilities (alternative costs) to a permanent is meaningless but harmless — match native
  behavior (granted Flashback on a battlefield permanent does nothing).
