# Prospective public cost references (UNVALIDATED)

## Exact frozen scope

The full source inputs and Oracle IDs are retained in
`fixtures/prospective_cost_references.json.fixture`:

- Back from the Brink: `704aaf92-590f-46d3-8b39-4ab84e1c766d`
- Merseine: `b6b6e085-eef8-486d-9d66-1b7f1ee7d0dc`
- Shelob, Dread Weaver: `7eade76f-92e4-4d26-9eb8-d3d954845c04`
- Fishing Pole: `2c1d3e9e-213e-492a-ad5a-91139d70bc0f`
- Veteran's Voice: `68d25ba4-d710-4090-bceb-5cfbd6bb63a1`

The last two close the previously recorded activation-context partials. These
are five source-complete proposals, not measured compile/runtime recoveries.
No hidden-zone choice, arbitrary multi-object prospective cost, or unrelated
referenced-cost card is claimed.

## Source-backed boundary

A typed optional `DynamicManaCost.mana_cost_of` retains a referenced object's
actual mana symbols. Its constructor does not replace mana cost with mana
value. The source's own legacy mana-cost mode remains unchanged, old JSON
omitting the new field defaults to `None`, and new display/timing variants are
appended. Referenced X outside the stack is zero (CR 107.3g); an absent mana
cost remains unpayable (118.6). Copy-layer mana costs are read through the
existing current-characteristic API.

Activation announcement now has an explicit public cost-reference stage.
Mandatory single-object choices needed by pricing or target filters can be
bound before payment. Their map is transaction-local and stores exact object
incarnations. The public choice uses ObjectId replay identity and no reveal
policy. Hidden hand/library selections, optional choices and unsupported
multi-object announcements do not receive fabricated references. No cost,
reveal, life change, or mana payment occurs in this stage. Ordinary cancellation
restores the existing action checkpoint; WASM routes the new stage through the
live priority response path rather than effect replay.

Read-only availability checks enumerate admissible public references without
mutating the caller's game. Each is checked with the specific granted ability's
grantor snapshot, current source-linked Exile members, actual cost predicates,
normal price modifiers, mana affordability and its own target context. An
unbound tag does not become a wildcard. A target requiring mana value exactly X
has a finite set of possible X witnesses: those mana values are tried on
isolated game copies with normal prices. Actual activation still announces and
locks a single X, and its effects read that announcement.

Once selected, a cost object's original filter is retained and additionally
restricted to its exact ID. Payment revalidates that current incarnation and
uses the existing real payment/movement executors. A later stable-ID successor
cannot pay for a departed selection. The generic tagged exile/movement cost
prechecks now enforce this live-incarnation boundary as well. Source-linked
Exile collections are refreshed as live membership; chosen cost LKI remains
separate. Grantor-tagged choice/consumer pairs retain their context-aware
execution rather than being compacted into a tagless standalone selector.

Alternative activation branches stay separate: the selected branch supplies
its own reference choices and price. Choosing a plain-mana branch does not
inherit another branch's exile choice. Normal mana, hybrid/Phyrexian payment,
source/payer identity and the reviewed disclosure commitment remain in the
existing payment machinery.

## Full-card clauses

- Back from the Brink announces the graveyard card, prices its exact mana cost,
  and then exiles/copies that paid card through its original typed cost tag.
- Merseine prices the live enchanted creature, enters with three net counters,
  and allows only that creature's live controller to activate the Aura. The
  activator-relative permission is explicit; Aura ownership is not substituted.
- Shelob uses a typed chosen Exile-to-owner's-graveyard cost, retains its own
  link restrictions, and keeps the separate X-targeted tapped return ability.
- Fishing Pole's granted ability checks and taps its specific granting
  Equipment, not another Equipment with the same name.
- Veteran's Voice binds the enchanted creature before target announcement,
  excludes that exact cost object, and still revalidates its tap payment.

## Authored, unrun checks

`cargo test -p ironsmith-compiler-runtime --test prospective_cost_references -- --nocapture`

The public target compiles all five full payloads directly and through artifact
JSON transport. It exercises real casting, activation, payment and stack
resolution; both Back price boundaries, the token copy, Merseine's cross-player
permission/counter/untap body, Fishing Pole's equip/granted payment/untap token
body, Veteran's Voice's pre-payment target exclusion, and Shelob's death trigger,
owner-zone cost, counters/draw and X return. It also authors public-choice
cancellation and alternative-branch isolation checks.

Engine unit coverage in `cost/prospective_references.rs` checks preserved
hybrid/Phyrexian symbols, zero referenced X, exact leave-and-return rejection,
missing tags, and unchanged life/mana/graveyard state on failed preflight.
Grammar tests retain source/type/destination and reject unsupported tails.

Only source/API/exhaustive-consumer review, rustfmt parsing and diff whitespace
checks were performed. No build, compilation, tests or replay were run.

## Eventual upstream integration

Do not overwrite upstream `2c6fc932` X-aware chooser checks with group/reference
checks: both contexts must remain bound. Preserve `c6f621ee`'s ability-specific
target-prompt text when integrating `continue_activation`. The new stage does
not otherwise change that prompt or the signed payment/disclosure protocol.


## Bounded source-review follow-up (still unrun)

- The pending-activation creation gate explicitly includes a required reference
  stage even when that reference is the only cost and the ability has no target.
  Merseine cannot reach the direct-stack fast path without paying its price.
- Read-only reference preflight combines all mana pips into a single budget;
  its correctness does not rely on the ordinary price builder's coalescing.
- Alternative menu/response checks receive the captured raw reference branch
  separately from its display/effective branch. Modifiers are applied once.
  After targets, only the chosen raw branch is repriced; unchosen references
  are not demanded.
- Pending branch queries now use a Result-bearing complete-legality query for
  reference and ordinary fallback branches. Resource exhaustion is propagated
  as `GameLoopError::ExecutionFailed`, preserving pending state, rather than
  becoming an unpayable/disabled option or an action cancellation.
- Authored regressions cover combined mana budgets, legal modal subsets,
  taxed/reduced alternative branches, and a token-producing mana analysis that
  exceeds its resource allowance without mutating the original game.

This follow-up uses the central resource-query API through `c8232347` (cap
prerequisites `3d3001ed`, `80783cb0`, `599ce453`, `0b1b32c8`, `8037d28d`). The
original source commits remain on `fix/prospective-cost-references`; the checked
API follow-up was prepared on an isolated branch based on central `76d99768`.
