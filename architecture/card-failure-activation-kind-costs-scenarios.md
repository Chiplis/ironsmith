# Activation-kind cost scenarios

Status: **UNRUN / UNVALIDATED**. These are authored source regressions, not
measured compilation or runtime coverage. No build, compilation, test, probe,
formatter, browser, replay, or generated-artifact command was run for this work.

## Frozen evidence

`fixtures/activation_kind_costs.json.fixture` retains exact identity and printed
metadata, complete Oracle bodies, and the original parse diagnostics from
`cards-20261003.json.xz` and `baseline-e8740178.snapshot.json.gz` for:

- Fluctuator
- Silver-Fur Master
- Dragonkin Berserker
- Boom Scholar
- Hulk, Gamma Goliath
- Eidolon of Obstruction
- Suppression Field
- Zirda, the Dawnwaker

All eight are source-complete candidates only. Source inspection found no
additional body blocker requiring a partial flag. This is not evidence of a
successful compiler run. In particular, Silver-Fur keeps its full Ninjutsu and
Ninja/Rogue anthem; Dragonkin keeps first strike and the Dragon-token boast;
Boom Scholar keeps the complete exhaust resolution; Hulk keeps its intrinsic
entry-turn discount and Power-up; and Zirda keeps its typed Companion condition
and targeted tap ability.

## Authored native scenarios

`crates/ironsmith-compiler-runtime/tests/activation_kind_costs.rs` routes complete
frozen cards through strict public compilation, parse-loss capture, validated
artifact JSON round trips, and native materialization. Synthetic participants
isolate the cost selectors without rewriting the real cards.

Scenarios assert the following:

- Cycling, Ninjutsu, Boast, Exhaust, and Power-up identities survive direct and
  artifact paths. An otherwise identical ordinary ability does not inherit a
  keyword discount from its sibling or from effect resemblance.
- Fluctuator plus Suppression Field composes the tax and reduction, preserves
  blue mana and the discard payment, stops at zero, and excludes opponents.
- Cycling and Ninjutsu payments obey replacements, consume them once, preserve
  their controller, and retain the resolution after the modifier leaves.
- Ninjutsu retains the unblocked-attacker requirement, returns a borrowed
  attacker to its owner's hand, preserves the defending player, and enters
  tapped and attacking after native recovery.
- Dragonkin counts only current controlled Dragons, rejects a stale action
  after a Dragon leaves, keeps red mana and boast timing/frequency, and creates
  the complete flying 5/5 Dragon.
- Boom Scholar's selector excludes itself and opponents, keeps both colors and
  tap/counter/life payments, and its full body grants trample to controlled
  creatures and Vehicles and puts two counters on itself.
- Nested mana payment cancellation restores taps, counters, life, mana, and the
  exhaust once limit. A sibling ordinary activation never borrows its discount.
- Hulk limits the external reduction to other controlled creatures' Power-up
  abilities. Its full counter effect and entry-turn intrinsic reduction remain
  independent of the external reduction.
- Eidolon taxes only opponent-controlled planeswalkers' loyalty abilities,
  retains loyalty counter payment and frequency, and excludes ordinary sibling
  abilities. Suppression Field exempts actual mana abilities; a loyalty ability
  that adds mana remains nonmana and taxed.
- Zirda uses the actual activator of a shared ability, keeps the one-mana floor
  and colored requirements, preserves nonmana payments, does not raise a
  mana-free activation to one, and does not reduce a mana ability. Its own
  targeted tap ability resolves the blocking restriction.
- A pending targeted exhaust activation survives replacement of its current
  ability slot with a different keyword, price, nonmana cost, and effect. Native
  game and pending-lane clones retain the original announced owner.
- X announcement locks the original X cost after a current-slot replacement,
  applies Zirda's one-mana floor to the announced amount, and resolves the
  original X-dependent program.
- Fluctuator plus Zirda permits the legal zero-mana Cycling total in either
  battlefield insertion order, with the actual discard and draw preserved.
- Both root and direct mana owners announce Exhaust X once, apply its keyword
  reduction after X locks, reserve the source's tap, and record its once limit.
- Power-up X combines Hulk's external reduction with the conditional intrinsic
  entry reduction and still puts the announced number of counters on the source.
- Both mana owners offer alternative original costs before flattening, select
  the requested branch once, and pay only its reduced mana plus mandatory tap.
- Missing original cost evidence returns typed IncompleteEvidence and restores
  the current lane without fabricating facts. Explicit root rollback restores
  resources and the once limit, allowing a new real activation.

## Deferred validation

When the campaign's execution gate is opened, run the targeted
`ironsmith-compiler-runtime` integration target `activation_kind_costs` together
with the grammar and WebAssembly scenarios owned by the implementation lane.
Until then, none of these source assertions is a passing-test claim.
