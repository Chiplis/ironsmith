# No-size object templates and complete ability grants

Status: UNVALIDATED source proposal. No build, compiler probe or test execution.

Five whole-card source proposals: Chimeric Mass, Druid Class, Svogthos, the
Restless Tomb, Warden of the First Tree and Frodo, Sauron's Bane. Exact frozen
metadata/Oracle text and IDs are in `fixtures/unsized_object_templates.json.fixture`.
Vraska, Betrayal's Sting is an explicit partial fixture: its transformation
primitive is covered, but its complete poison-difference/Compleated program is
not promoted by this family. No card name affects executable behavior.

## Typed absence and complete grants

The shared transformation AST already uses an optional base-power/toughness
pair. The new complete token grammar accepts an unsized descriptor followed by
its whole granted ability list, including quoted static, activated or triggered
programs. No dummy zero, inherited current size, or unsupported marker replaces
an absent base characteristic. Keyword-only lists keep every item. Unknown
leading descriptors and unsupported full grants fail closed.

Outer type/color retention and ability-removal tails are split only outside
quoted rules text. Creature subtypes without a card type retain existing card
types, whereas an explicit Treasure artifact never becomes a creature. Named
subtypes replace only their actual subtype families. Existing `still a land`
follow-up handling annotates this same atomic transformation.

`loses all other card types and abilities` sets the actual authored type and
uses an appended compiled RemoveAllAbilities modifier before its new grants in
the same ordered effect group. The old runtime `loses all abilities` modifier
still applies after grants. This distinction preserves the new Treasure mana
ability while removing the recipient's old abilities. Presentation follows the
actual creature/artifact/permanent template rather than always saying creature.

## Reuse the native receiving-object and layer semantics

The existing `generate_granted_late_static_effects` path already converts
ordinary granted self-P/T abilities to layer7b and uses the granting effect's
timestamp. `CharacteristicDefiningPt` generates an exact Specific(receiver)
target and receiving source/controller; it does not capture the grantor. Values
remain live when counters, graveyards or control change. Genuine printed/copy/
token-definition CDAs retain their existing layer7a path. No broad global CDA
rewrite is necessary or included.

Authored direct/artifact tests cover complete five-card bodies, Chimeric's live
charge counters and earlier/later7b settings, Druid's actual class-level event,
receiver control changes and departed Class, Svogthos graveyard/controller and
cleanup, Warden's conditional stages/retained size/keywords/counters, and Frodo's
combat-only recipient/Ring-history behavior. A generic noncreature scenario
checks type-family cleanup and clear-before-grant ordering. Normal strict tools
fixtures and grammar negatives are authored too. All remain unrun.

These tests focus on the implemented semantic boundary; they do not establish
that every existing payment, choice, copy, layer or multiplayer route is correct.
Full baseline replay, supported-card regressions and execution of the authored
scenarios remain mandatory after the campaign's implementation-first gate.
