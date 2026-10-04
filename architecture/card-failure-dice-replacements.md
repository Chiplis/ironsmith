# Extra dice and ignored lowest rolls

UNVALIDATED source proposal. No build, compiler probe or test was executed.

Exactly three frozen failures share the complete replacement instruction:
Pixie Guide (b6003419-1a2a-46f3-b32c-47b825324f78), Barbarian Class
(59faa1b9-17aa-4f2c-a8f8-17ab50392b36), and Wyll, Blade of Frontiers
(4fb254cf-b60c-4a3e-b753-4213cb572837). Exact full Oracle bodies, metadata and
frozen source hashes are in `fixtures/dice_replacements.json.fixture`. The root
was `unsupported roll clause (clause: 'that many dice plus one')`. All three are proposed complete after bounded independent source review of
`6633f3667`. They remain unvalidated; no authored scenario has run.

## Implementation

A complete grammar owner recognizes the bounded replacement sentence before
conditional/action splitting. It emits an appended `ExtraDieIgnoreLowest`
static payload with an affected-player filter and an additional-dice count.
Unknown participants, altered ignore instructions and extra tails do not match.
The payload survives the shared static mapper and artifact serialization. Its
native numerical roll owner reads typed current abilities, not Oracle text.

The shared RollDie/RollDiceChooseResult/Attraction pre-roll boundary takes a
checked continuous snapshot. It excludes phased-out sources, uses the current
controller to interpret the affected-player filter, and lets the actual roller
order competing replacements. Each exact source/ability occurrence applies once
to that original batch; separate occurrences of a shared ability model remain
separate. Count overflow and allocator failure are typed incomplete execution.

All physical dice are rolled together. Replacement ignore instructions complete
inside-out; the roller chooses among tied lowest natural results. Ignored dice
are removed before reroll and numeric modifiers and never enter completed-roll
history, ordinals, per-die triggers, grouped totals or the explicit result choice.
The original count of retained dice remains unchanged. Existing effect/turn
owners restore the game/context/queues on suspension or errors, preserving their
same-attempt random-disclosure contract.

CR 614.5 and 706.2b/706.6 make same-event rerolls result modifications, so they do
not reapply an already used replacement. An authored new roll (including a real
results-table “Roll again”, CR 706.3c) is a new operation with fresh replacements.
CR 706.7 leaves planar rolls outside numerical-lowest replacements. The official
[Forgotten Realms release notes](https://magic.wizards.com/en/news/feature/adventures-forgotten-realms-release-notes-2021-07-09)
confirm that multiple instances add multiple extra dice and ignore that many low
results. The source rules are pinned in the campaign's September 2026 CR file.

## Full bodies and authored scenarios

Direct and JSON-artifact scenarios cover all three full cards. Pixie retains
flying. Wyll's actual one-or-more counter trigger fires once for the retained
batch, while ignored low-result observers do not trigger. Its existing Partner
variant descriptor preserves Choose a Background; the existing pregame
commander-pair owner recognizes that descriptor and the other commander's
Background type. No deck-construction rule was replaced by a battlefield effect.
Barbarian's real paid class activations gate the level-two targeted +2/+0 and
menace trigger and the level-three controlled-creature haste grant; cleanup
removes only the temporary pump/menace.

Other authored scenarios cover current-controller changes, a foreign effect
controller versus actual roller, phasing, multiple replacements/occurrences,
multiple original dice, tied-lowest choice suspension/native replay, modifier
and paid-reroll order, representation rollback, and actual Attraction/planar
producers. Tests are authored and unrun in `dice_replacements.rs` and the grammar
surface tests. This batch does not claim additional die-replacement wordings,
planar-result replacement mechanics or an arbitrary-precision dice engine.
