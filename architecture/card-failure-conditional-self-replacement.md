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

## Simultaneous sacrifice correction

Review found that the old `SacrificeProposal::commit` prepared and finished
each participant's zones and added programs before advancing to the next
participant. Epicenter and the previously proposed **By Invitation Only** were
held on that concrete shared path. Expel the Interlopers does not use it.

`PreparedSacrifices` now captures selected objects, eligibility and LKI before
any participant mutates state. `prepare_original` resolves all zone proposals
on the same game, preserving ordered consumption of one-shot replacements.
`commit_original` applies only those prepared originals; `ZoneInstructionDraws`
owns their completion and preserves deferred draws and additional programs.
The direct sacrifice owner uses the same preparation/commit/completion contract
inside its native transaction. Actual `OriginalSacrificeObjects` and sacrifice
events remain separate from chosen cost resources: redirected departures count,
wholly prevented/replaced originals are known empty, and modified payment may
still complete under CR 118.11.

Exact Epicenter and By Invitation Only scenarios use an early A departure whose
added observation must see later B/C originals already gone, and B's replacement
depends on A's pre-departure static power bonus. The Epicenter addition includes
an actual draw. Native direct/dispatcher scenarios also pin single-instruction
completion and shared one-shot consumption. Existing pending/resource failure
scenarios exercise the full original batch and retry. The Bog Down modified-cost
control now targets the actual zone-change proposal with its replacement matcher.
All scenarios remain unrun.

Each completed original sacrifice now allocates a distinct child Sacrifice
occurrence under the common instruction provenance. The shared history owner
can stage, commit, and republish each physical event without either collapsing
different sacrifices or counting a duplicate publication twice. Native direct
and dispatcher controls retain all three staged/committed originals and their
common parent/group; prevented and wholly replaced originals emit none.

The direct Sacrifice, SacrificeTarget and EachPlayerSacrifices entry points now
open the shared resource/routing transaction before count or selection and
refresh checked continuous state. Immutable participant preparation uses a
checked query frame. Selected/source LKI and attached-object characteristics
come from one checked batch before snapshot assembly. Discovery and scalar
range failures propagate before zero/empty results or choices are accepted.
The exact shared zone-preparation wrapper also uses the routing-preserving
restore, so an inner pending choice's actual controller survives every enclosing
rollback. Direct/dispatched numeric, incomplete-discovery and control-prefix
pending/retry scenarios are authored for these existing entry paths.
Known phased targets remain ineligible rather than becoming missing evidence;
phased ability sources use their exact retained source LKI while a live recipient
can still be sacrificed. Direct and dispatcher positive/zero controls cover both.
