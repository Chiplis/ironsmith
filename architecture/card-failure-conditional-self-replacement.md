# Conditional scalar and target programs

Source-only campaign work from `d1737c6598429f9c7999f6737380ab18e6b1bde8`.
All scenarios below are authored and **unrun**. No compiler probe, build,
formatter, corpus execution, or test has been run.

## Initial proposal

The exact frozen bodies in `fixtures/conditional_self_replacement.json.fixture`
are copied from `cards-20261003.json.xz`, checked against the stage 68 evidence.

- **Bog Down:** one player declaration; two-card default and three-card kicked
  replacement; complete two-land sacrifice kicker, including modified payment;
  copies keep the paid choice without paying again.
- **Hypnotic Cloud:** complete printed mana and four-mana kicker; one versus
  three discards, target-controlled choice, copied choice, empty/illegal player,
  suspended choice and replay.
- **Haunting Hymn:** two versus four discards from actual main-phase cast
  evidence. The main-phase caster is typed receipt data; a different current
  controller fails the gate. Missing required caster evidence is an error.
  Spell copies retain optional-cost decisions but clear actual timing/foretell
  facts. Ability copies retain their source's receipt.
- **Whispers of Emrakul:** opponent target, random one/two-card discard, current
  controller-relative distinct card types, both threshold transitions and copy.

The fixed-discard pair reader consumes the complete action and reuses the
original player declaration. It produces the existing `SelfReplacement` AST.
The ordinary conditional consequence reader now handles a final sentence period
before its terminal `instead`, preserving the separate-line typed attachment
path. Original-token guards prevent malformed recognized discard references
from being reclaimed by broad readers. No discard effect ignores an `instead`
tail or executes the base before the replacement.

`OptionalCostsPaid::record_main_phase_cast` captures the caster during the native
cast transaction; the existing paid-reference evaluator checks that actor.
`clear_uncopied_cast_facts` is shared by the object and stack-entry copy owners.
Kicker, announced optional-cost branches and paid alternative-cost dates remain
copiable choices. Existing hand-authored timing tests now use complete receipts.

Independent direct compilation and artifact compilation/serialization/
materialization routes are authored in
`crates/ironsmith-compiler-runtime/tests/conditional_self_replacement.rs`.
Native scenarios include actual cast/payment, copying, state changes, current
actors, insufficient hand size, known empty, invalid sole target, pending replay,
and missing-evidence rollback. These cards have one printed target; partial
target legality is not an extra target slot in these exact bodies.

## Second proposal

- **Epicenter:** complete trailing threshold program, one original target even
  while threshold is already active at announcement, live graveyard count,
  one chosen land versus simultaneous all-player controlled-land sacrifices.
  Includes changed threshold, foreign ownership, nonland/zone exclusions, a
  copy reevaluating threshold, invalid sole target, and a late replacement
  suspension/resource failure rolling back earlier participants.
- **Bring the Ending:** existing controller-paid `{2}` counter remains the
  default. The replacement uses the same announced spell and tests its current
  controller's poison before any counter result exists. Only the complete local
  counter-replacement reader binds its typed antecedent to `ControllerOf(Target)`;
  the shared condition reader retains `ItsController` for selected and triggering
  antecedents. Includes real paid
  cast, changing poison/controller, copied target and all-illegal original,
  accepted/declined/unaffordable payment, uncounterability, pending payment,
  replacement-added resource failure, and retry.

The named trailing local-action reader consumes `instead if` as structure and
keeps the full existing `TrailingIf` AST for cross-line self-replacement
attachment. It does not add a second target, execute the default action to
obtain its reference, or change the counter/unless payment grammar. The existing
ForPlayers simultaneous-sacrifice owner and native stack rollback boundary are
reused. Original-token guards reject symbols, repeated markers and trailing
garbage before a broad verb reader can discard them.

Neither the source ledger nor the central checkout is edited here. All six
remain subject to the campaign's deferred execution gate; source review is
reported separately per checkpoint.
