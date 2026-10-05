# Grouped and repeated coin instructions

UNVALIDATED source work. No builds, compiler probes, formatters, tests, or corpus
execution were run for this patch.

## Frozen scope

`fixtures/grouped_coin_flips.json.fixture` preserves eight frozen source entries:
Crazed Firecat, Ral Zarek, Goblin Traprunner, Two-Headed Giant, Okaun, Eye of Chaos,
Zndrsplt, Eye of Wisdom, and the reversible-card aliases of the two partners. The
aliases retain the exact first-face body and identify that source face explicitly.
These are six Oracle identities and eight campaign compile entries. Each entry
retains its baseline category, parse error, and content hash from
`baseline-e8740178.snapshot.json.gz`; none is a verified recovery.

All eight are proposed complete, contingent on deferred executable validation.
The partner-with ETB search, both partners' per-win secondary triggers, and all
three Ral loyalty abilities are part of the claimed full body. The prior source
already owns partner-with search, power/toughness doubling, tokens attacking,
and extra turns; this patch composes those owners with grouped coin evidence.

Mutalith's secondary body; Yusri's chosen count and casting permission; and Ral Zarek, Guest Lecturer's
skip-turn body remain outside this claimed cohort. Krark's Thumb and Edgar are
modifier interaction scenarios, not additional claimed frozen recoveries here.

## Semantic boundaries

A called flip preserves the physical face, call, actual flipper, winner, and
loser independently. A face-only heads result normally has no winner. Edgar's
explicit result override can award a win even when there was no call.

One instruction exports one ordered typed receipt of retained flips. Fixed
multi-coin instructions share a simultaneous-batch identity; repeated one-coin
instructions have separate batches and finish only when a retained flip is
lost. Every retained physical event gets unique provenance, a turn ordinal,
and an instruction ordinal. Ignored replacement coins never reach completed
history, the receipt, or per-flip triggers.

Follow-up counts and face predicates bind to the exact local producer. An
unrelated gain-life effect does not replace that producer. A later coin
instruction replaces the binding, including an optional instruction whose
declined result must not reuse the earlier receipt. Standalone consumers fail
closed without a compatible producer.

Counters or tokens created "for each" result must remain a single native
counter-placement or token-creation instruction with a derived count. The
number of coin wins does not authorize splitting one simultaneous creation into
multiple independent instructions.

## Authored native scenarios

`crates/ironsmith-compiler-runtime/tests/grouped_coin_flips.rs` compiles complete
bodies directly and through serialized, validated, rematerialized artifacts.
It exercises:

- Firecat's real paid cast, ETB, terminal loss, and win-count counters.
- Traprunner's native attack trigger and tapped attacking token count.
- Giant's four face pairs, source-only grants, and cleanup expiration.
- Ral's paid loyalty abilities, distinct tap/untap targets, damage, and exact
  extra-turn count without calls for face-only flips.
- Every partner entry's real cast/ETB search, optional decline, exact named card,
  controller's combat step, and distinct per-win stack entries.
- Observer controller versus actual flipper, opponent wins, and ordinary
  face-only heads producing no win trigger.
- Typed grouped receipt fields, unique event provenance, simultaneous batch
  identity, instruction ordinals, player-specific turn ordinals, and turn reset.
- Suspension at each call, native state restoration, a native suspended ETB
  stack entry, typed capacity failure after a partial repeat, and clean retry.
- Thumb's ignored physical results, suspension after physical coins but before
  the keep choice, and Edgar's fixed-batch versus repeated-batch boundary.
- Separate instruction bindings, intervening unrelated actions, optional
  decline, and unbound consumers.

Native GameState checkpoints retain the forced face queue, random state,
completed history, and event ownership. Tests assert that suspension or failure
cannot publish partial receipt facts, UI events, counters, or win triggers.
These scenarios are authored evidence requirements, not execution results.

The shared random-result binding owner also serves existing numeric-die
consumers; coin and die namespaces stay separate across reference frames.
First-batch semantics follow the official [FINAL FANTASY release notes](https://magic.wizards.com/en/news/feature/final-fantasy-release-notes): a fixed multi-coin
batch is modified in full, and its explicit win override can give face-only
coins winners. A repeated-until-loss instruction performs successive batches.

## Mirror March continuation

`fixtures/grouped_coin_followups.json.fixture` adds Mirror March as a separate
UNVALIDATED full-body source proposal. Its quantified copy is one existing token
creation instruction; the shared token followup owner attaches haste and delayed
exile to that exact group. The original copy source and the enchantment may both
leave before resolution; the copy owner retains exact source LKI. Typed target
resolution errors now survive rather than being discarded during LKI fallback.

The shared copy owner distinguishes separate haste grants from copy exceptions:
“those tokens gain haste” uses an undated continuous ability grant for original
and replacement-added tokens, so copying one again does not inherit haste.
Authored native scenarios cover zero and multiple wins, both source departures,
recopy semantics, a control change before cleanup, and delayed exile limited to
the actual created group. No added scenario has run.
