# Keyword action bodies: source candidate, unvalidated

This cohort uses the complete frozen Oracle bodies in
`fixtures/card-failure-campaign/cards-20261003.json.xz`, copied without alteration
into `fixtures/keyword_action_bodies.json.fixture`. No build, test, compiler
probe, formatter, engine probe or corpus execution was run. This document does
not assert measured compile recovery or change campaign accounting.

| Exact Oracle ID | Body | Frozen failure and source change |
| --- | --- | --- |
| 221240c5-3c97-438c-b906-2508eacf190f | Confront the Unknown | Standalone Investigate consumed the comma-then continuation as a count. It now yields the complete chain to the chain owner, preserving the creature target, post-investigate Clue count and end-of-turn duration. |
| ead0820e-fc6b-4ab2-9110-048d6d53985d | Panther Pounce | Investigate lowering rejected a target player. It now uses the existing targeted player subject/choice owner, followed by the independent creature target, +1/+0, flying and untap reference. |
| ca4936cf-48d4-4469-9243-8dbf8ec4cdca | Roalesk, Apex Hybrid | The count grammar rejected “again.” This spelling now denotes one additional action. The two proliferations remain sequential, independently chosen actions; flying, trample and the other-creature ETB counter target remain in the full body. |
| 55c7ce94-3cd3-42f3-9dd8-0c118fa626c8 | Secrets of the Key | The complete keyword/conditional-instead pair lacked an owner. A named pair grammar preserves both mutually exclusive Investigate arms and the typed cast-origin predicate. Printed Flashback is retained. |
| 5753f4d6-08d0-4e12-85c5-875ee6441626 | Tidings of War | The same pair grammar preserves Amass Goblins 1 versus Amass Goblins 3, selected by actual graveyard cast origin. Printed Flashback and its exile disposition are retained. |
| ab41243d-b178-4ebe-a7ac-bea37427be99 | Wojek Investigator | A player predicate fell through to a permanent count and lost the hand comparison. Shared count recognition now uses CountPlayers with CardsInHandAtLeastMoreThanYou(Opponent, 1), evaluated against current hands. The renderer preserves the full qualifier. |

All six are source-complete candidates, not verified recoveries. The existing
InvestigateEffect, ProliferateEffect, AmassEffect, CountPlayers, qualified player
filter, SelfReplacement and ThisSpellWasCastFromZone owners are reused. No
semantic payload, artifact version, staged gate, campaign coverage or published
stack metadata changes are required.

## Subsequent native naming review

The five token-producing candidates were held after independent review found
that their native keyword owners bypassed compiler name/profile retention.
The source repair in `native-keyword-token-names.md` changes the real native
names to `Clue Token` and `Goblin Army Token`, retains typed word roles and
corrects the authored name witnesses. These five remain subject to independent
review; this document does not readmit them or alter coverage. Roalesk's
separate Proliferate review remains independent.

## Runtime ownership reviewed

- Investigate resolves its actor once and runs a distinct CreateToken action for
  each repetition. Its token lifecycle transaction restores game state and
  resolution memory on pending choices or errors. The selected player owns the
  Clues and their entry choices/payments. The real Clue blueprint supplies the
  `Clue Token` name, artifact/Clue type and the {2}, sacrifice-this-token draw ability.
- Proliferate already enumerates eligible live players and non-phased
  permanents afresh for every action, validates and deduplicates each choice,
  and applies every existing counter kind. Its rollback checkpoint includes
  earlier iterations. Separate authored actions retain ordinary sequence
  execution and choice order; the second can choose different objects.
- Amass uses the controller's current non-phased Army creatures, chooses one
  when several are present, or creates the real 0/0 black Goblin Army Token. Counters
  are placed before the additional Goblin subtype. The subtype change uses a
  continuous effect and does not alter copiable characteristics. Its token
  lifecycle transaction owns token-entry and counter replacement suspension.
- Cast-origin predicates use the existing casting method and recorded cast
  origin. The conditional pair is a SelfReplacement, not a default action
  followed by an additional conditional action. The Flashback casting owner
  handles its own costs and post-resolution exile.
- Numeric player enumeration formerly used the context-only matcher, which
  drops game-dependent hand predicates. It now calls the game-aware matcher,
  agreeing with the continuous-value owner on current hands and team opponents. Malformed participant-count tails cannot be misinterpreted
  as battlefield object filters.

## Authored, unrun regressions

`crates/ironsmith-compiler-runtime/tests/keyword_action_bodies.rs` independently
constructs direct, compiled-artifact JSON and native-definition JSON routes and
executes the whole frozen bodies. Scenarios assert:

- Clue creation precedes the size calculation and only the caster's Clues count
- Different player and creature targets retain recipient, power/toughness,
  flying, untap and cleanup behavior; a departed creature target preserves the
  remaining legal Panther action, while Confront wholly fizzles
- Hand and graveyard casts execute exactly one arm and have the proper final
  zone; uncast copies use the default arm; Amass selects one eligible Army and
  retains prior/copied subtypes
- Roalesk's real ETB target is another creature, and its real death trigger
  presents two choices with the second observing the first's counters
- Wojek ignores other players' upkeep and counts current strict hand advantage
  at resolution, excluding tied hands and opponents who ceased being ahead
- Direct and dispatched repeated Investigate roll back life payments, tokens,
  IDs and emitted events on suspension/resource failure, then replay once
- Repeated Proliferate rolls back the first choice when the second is pending,
  then replays with two independently selected objects

Local grammar regressions exercise complete pairs, both branches, invalid
suffixes, chain retention, “again,” strict qualified counts and malformed player
comparisons. Text regressions assert the hand qualifier and repetition survive
rendering. No assertion in this packet has been executed.


## Current prepared-main disposition

The bounded corrections and their prepared-main port are source-reviewed and
admitted as unvalidated proposals in `card-failure-stage97-source-admission.md`.
That record supersedes the earlier pending-admission wording above. The complete
frozen body fixtures and all authored scenarios remain unrun; measured recovery
is unchanged. The coordinated source boundary is artifact9 / digest5 / audit22.
