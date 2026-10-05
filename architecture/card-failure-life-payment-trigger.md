# Completed life-payment receipts (source proposal)

Status: UNVALIDATED; no compiler, build, tests or CLI replay executed.
One frozen identity, Font of Agonies (ab44c6fe-6a66-42f0-9e75-8b25641e1f13),
remains uncounted until independent source review. Its exact full input and
baseline hash/error are in `fixtures/life_payment_trigger.json.fixture`.
Hibernation's End and Shah of Naar Isle were delivered separately.

## Shared source path

`PlayerPaysLife` is an appended semantic/core trigger, backed by an appended
native `LifePaid` event and typed player matcher. The payment receipt retains
the payer and accepted nominal amount; the separate processed LifeLoss receipt retains the actual changed amount. Loss/damage never synthesizes a paid notice.
The matched amount supplies the full Font counter body; its ordinary mana plus
four-counter activation still destroys the selected creature. Grammar requires
a complete player-subject payment event, without swallowing trailing text.

The checked GameState owner prepares the real life-loss replacement action and commits its original result, retaining both the ordinary LifeLoss
companion and LifePaid receipt with distinct occurrence identities, stages the
complete original history, refreshes checked characteristics and captures
observers before following costs or instructions can remove/add them. Already
captured receipts remain available for local outcome metrics and are idempotent
when queued again. Simultaneous payments share an exact action batch, commit
all original deductions before matching, and retain separate payer notices.
Fixed/chosen each-player proposals use original-only commits; their action
iterator captures the complete payment unit before the next authored action.
Explicitly acknowledged zero life is a successful zero payment; absent life
costs (including zero-cost die modifier metadata) do not invent a payment.

Real producers are Cost::life (now PayLife instead of its legacy LoseLife
implementation), fixed and chosen life effects, aggregate life substitutions
in actual mana payments, shock-entry replacement acceptance, and paid numeric
die modifiers. The outer cast/activation, cost, entry and roll transactions
retain rollback. Native clones preserve matched payment observers. Existing
unrepresented-history/program guards remain the lossless recovery boundary.

## Checked APIs and query boundaries

GameState::pay_life, pay_life_simultaneously and all try_pay_mana_cost variants
return Result<bool, ExecutionError>. False means an actually unaffordable
payment; checked history/discovery failures propagate as execution failures.
Bulk mana commitment restores pools, restrictions, life, provenance and
receipts on failure. Actual cost/priority/combat adapters preserve this typed
failure. Planner-only score/Option callbacks latch incomplete errors through
the shared query scope; the exact Assist unit-commit continuation does the same
instead of returning false affordability. Existing assertion-only callers were
mechanically updated, including one WASM test; transport behavior is unchanged.

The native host still has bounded numeric representations. Life-payment amount
and accumulated loss-history failures are explicit transactional errors, not
smaller successful payments. This does not claim arbitrary-precision gameplay.

## Authored, unrun scenarios

- Full strict direct definition plus validated JSON artifact round trip.
- Fixed/chosen life payments put exact blood counters; real paid activation
  consumes four counters and mana, then destroys its target.
- Ordinary loss/damage and another player's payment do not trigger Font.
- Real bulk life-mana payment; query isolation and immutable event amount.
- Shock entry acceptance/decline/pause and paid die modifier/pause.
- Payment-time source followed by its sacrifice as another cost.
- Completed simultaneous payer history before following gains.
- Idempotent publication, native checkpoint retention and rejected payments.
- Typed history overflow restores direct and bulk payments, and speculative
  exact-unit/planner failures remain incomplete execution.

Deferred validation commands (DO NOT RUN during source-first campaign):

- cargo test -p ironsmith-compiler-runtime --test life_payment_trigger
- cargo test -p ironsmith-engine --lib life_payment
- cargo test -p ironsmith-engine --test u075_two_headed_giant
- cargo test -p ironsmith-compiler-grammar life_payment_trigger
- affected planner/cost/priority fixtures and full-corpus replay after the gate.

Source-only checks: rustfmt parser over changed Rust files, git diff --check,
exact frozen identity/fixture inspection. No runtime success is asserted.

Adjacent authenticated producer dependency: the price-cost worker is migrating
the two legacy DerivedAlternativeCast life-equal-mana-value constructors in
`grant.rs` from LoseLifeEffect to PayLifeEffect. These callers must be integrated
before Font is promoted as covering all actual life-price routes.

## Replacement-aware correction (source only)

Pinned September 2026 CR 118.11 says modified cost actions still pay the cost;
CR 119.4 explains the underlying life loss and CR 119.4b explicitly allows
zero payment even when positive life payment is prohibited. Consequently a
nominal payment of two that is doubled loses four life but emits LifePaid(2),
and an instead program may fulfill that payment without any LifeLoss receipt.
The new owner uses the existing life-change prepare/original/completion API.
Actual loss/gain facts retain processed recipients and amounts; added programs
remain outside the original receipt quantity. Payment observers freeze before
those additions even inside the nonmana cost's simultaneous scope. Each-player
payments still wait for all original participants and check aggregate shared
team affordability before preparing any replacement.

Actual fixed/chosen, shock, die and mana producers keep their real decision
maker and replacement context. New choice-aware bulk adapters restore suspended
payments, and the priority/plan owners preserve pending continuation rather
than reporting illegal payment. The nominal u32 native Cost::life constructor
is checked before conversion; an unrepresentable amount retains a typed host
representation error and cannot become zero or a portable fabricated cost.
Acknowledged min-zero chosen payments offer an explicit 0..0 choice under a
life-total restriction, with no receipt from a pending choice.

Additional unrun cases cover modified/instead payments, added programs and
rollback, cost-scope source departure, choice-aware bulk suspension, prohibited
zero choice and the oversized constructor boundary. The legacy alternative
life-price migration prerequisite is d0e170955 (price owner), whose source
chain also includes 5ab90387e. No proposed-complete promotion is made here.

## Remaining simultaneous composition boundary

The payment owner now opens/reuses one computation meter across all originals
and completions without charging a token instruction. Full inherited execution
scopes are carried through nested Cost::mana, PayMana and interactive mana
commitment, including temporary replacement rules and suppression history.

A simultaneous payment's single Instead life-gain/life-loss instruction is
prepared in its captured replacement context before any payer original, then
commits as a real original receipt. The authored two-payer regression requires
its gain observer to see the other payer's completed loss. A compound or other
unrepresented simultaneous Instead program is rejected during preparation,
before any originals or successful notice. That is a known shared runtime
partial, not a recovered card or successful no-op, and needs a general
replacement action-iterator owner before the final semantic gate. Font remains
uncounted pending coordinator/reviewer disposition of this boundary.

The independently usable single-payment meter/inherited-scope correction is
committed separately as dea54c578; this follow-up does not hold the individual
price-cast bodies on the still-partial simultaneous compound schedule.
