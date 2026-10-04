# Exact token creation and incomplete-execution boundaries

Status: source-authored, UNVALIDATED. No build, compilation, test, or corpus replay
was run for this change. It adds no proposed card identities and no measured
recoveries. The currently tracked 14 token-dependent proposals (the original 13-token
cohort plus Rakish Revelers) retain an explicit correctness gate until the
deferred exact-count and execution validation completes. Later token-producing
proposals inherit the same gate.

## Removed gameplay clamp

The old per-controller 500-token clamp silently changed an otherwise successful
creation. It is removed from ordinary tokens, copied tokens, Incubate, and mixed
named/template groups. Affordable 501+ creations now take the complete final
replacement proposal. Every group keeps its original creation instruction's
entry state, counters, linked references, and applicable delayed cleanup.

Token proposal totals use exact `u128` arithmetic. Doubling, multiplication,
additions, and per-created substitutions check both each group's `u32`
representation and the complete proposal. Overflow returns a typed execution
error before a later replacement can turn a saturated value into a plausible
success. Subtraction still floors at zero because that is the authored operation.
Published trigger/history consumers receive only preflighted, committed totals.
This does not redesign the engine's broader signed scalar/history domain.

## Host computation limits

`TokenCreationLimits` lives in the runtime cache, outside serialized rules state.
Its default profile permits 16,384 created tokens, 65,536 live objects, 4,096 units
of token-instruction/repetition work, and 64 nested token instructions per outer
computation. Hosts can set another explicit profile. These are computational
boundaries, never Magic rules: exceeding one returns `ResourceLimitExceeded`,
with resource/request/limit, rather than fewer tokens, prevention, a neutral
outcome, or a successful zero. Fallible owner-buffer reservations return
`ResourceAllocationFailed` if the allocator declines them.

The meter is shared by nested GameState checkpoints while an attempt is active.
Ordinary effect dispatch and whole stack resolution/program execution establish
outer scopes. The three concrete token owners establish scopes for direct calls.
Investigate, Populate, and Amass also own atomic scopes, so repeated direct
children cannot reset their allowance. Repeated keyword work is reserved before
buffer allocation; actual complete token groups are reserved before any token
object, entry event, or cleanup registration is committed. Reservations include
unmaterialized outer siblings when entry programs create nested tokens. Removed
objects do not refund this conservative computation budget.

When an attempt finishes or suspends, its shared meter is closed. A saved hidden
continuation can share work during the attempt but must obtain a fresh allowance
when replayed later. The meter is neither a per-player permanent cap nor a
persisted game counter.

## Atomic failure contract

Existing token/event receipt checkpoints restore game state and owned resolution
context on every propagated error or unanswered decision. Resource errors also
restore the outer dispatcher checkpoint; stack/program owners restore the whole
resolution and trigger queue. This includes earlier tokens and Incubate iterations,
life/counter changes from entry or replacement programs, object ID allocation,
links, one-shot replacement consumption, entry/zone-change observations, inherited
instructions, and delayed cleanup registrations. The spell remains unresolved on
resource exhaustion. `GameLoopError::ExecutionFailed` preserves the typed error;
no conversion to an impossible action or successful empty result is introduced.

Decision-maker prompt/answer state is retained according to the existing replay
contract. It is not a committed token/event receipt. Source assertions cover a
resource failure followed by a fresh, larger-budget attempt and a saved-continuation
meter lifecycle. Fatal process-wide allocator aborts are not made recoverable by
this patch; bounded work and fallible principal buffers do not establish an
unbounded or compressed-token engine.

## Authored scenarios (all unrun)

- Exact 501 ordinary tokens followed by another two, rather than 500 then zero.
- Direct and restored compiled Quina: 501 Soldiers plus one Frog, all tapped,
  502 cleanup registrations, and one complete 502-token creation event.
- Direct and restored Divine Visitation: exactly 501 Angel templates.
- Direct and restored copy creation: 501 copies plus one added Frog with outer
  haste, tapped state, and cleanup instructions on all 502 objects.
- Incubate 501 times: 501 tokens with counters and 501 distinct creation events.
- Checked doubling/addition/template/aggregate representation overflow.
- Nested work/depth and reserved-live-object limits.
- Incubate failure after prior successful iterations; no objects/events survive.
- Nested entry payload exhaustion after life gain, preserving the one-shot effect
  on rollback, then exact success with a larger host allowance.
- Investigate and Populate cannot reset budget between repeated direct children.
- Dispatcher and whole-stack failure restore prefix effects and keep the spell.
- Saved continuation clones cannot reuse an already closed attempt's allowance.

These are authored regression scenarios, not passing evidence or final coverage.

## Source-review corrections

The live-object reservation now tracks outstanding token slots separately and
rechecks them at each actual insertion. Non-token objects allocated by an entry
program cannot hide unmaterialized siblings. Cumulative-upkeep preflight and
ordinary/simultaneous unless-payments preserve typed execution errors instead of
running their unpaid consequences. Interactive sacrifice-or-redirect also
propagates errors and restores earlier sacrifices. Added source-authored scenarios
cover these boundaries; they remain unrun. Mana planning/payment owners now carry `EffectExecutionFailed` with the original
execution error. Manual activations, plan execution, and PayMana preserve it.
Synchronous and sliced planner queries have one independent query meter shared
by their simulated branches, so dry runs do not double-charge physical creation.
A resource-failure latch prevents an internal Option/boolean branch from turning
an incomplete simulation into a no-plan result. Nested queries report that failure
to an enclosing execution scope; its checkpoint prevents commitment. The latch is
closed with its attempt/query and is not persisted as a game rule. Added query,
manual-payment, PayMana, and deliberately absorbed-child-error scenarios remain
unrun; independent source review and eventual execution are still gates.

Bounded-X PayMana preflight now propagates resource unknown rather than lowering
X's affordable maximum. Direct PayMana is an atomic resource owner. Manual source
usefulness and activation inventories have independent query-owned budgets, and
checked inventory APIs expose resource errors. Actual manual/priority activation
uses the checked API, so an affordable two-token activation does not spend its
allowance once in preview and again in execution. Added bounded-X and exact-budget
success scenarios are authored and unrun. Legacy Vec-returning UI inventory
adapters and resumable legality error propagation remain under separate source
review; they must not be treated as completed negative-affordability evidence.

### Legal-action and payment-UI boundary

The resumable legality session retains the original typed execution failure,
never completes it as an unaffordable payment, and refuses to memoize a fact
computed during that failed branch. Both bound and nested/unbound planners
report the same failure. Synchronous ordinary and commander legality queries
own independent simulation scopes and return an error if any inspected branch
could not be computed. Commander analysis and mandatory-loop optional-window
proofs now preserve that `Result` instead of accepting an incomplete action set.

Wasm priority jobs inspect the session failure before publishing a confirmed
menu. Eager snapshots, deferred payment-option requests and Manabrew prompts
use the checked inventory APIs; they return explicit errors rather than cache
empty inventories. The complete/error distinction is retained across the JSON
host snapshot boundary. Payment preview ID counters are restored on failure,
request preferences are not overwritten, and successful later fresh analysis
remains possible with sufficient host allowance. Existing test-only view
conveniences unwrap checked results so fixture failures remain visible.

New native and Wasm scenarios are authored and **unrun**: sliced/synchronous
failure and memo rejection, root legal-action error and later recovery, eager
and deferred inventory failures, unchanged preferences/ID counters, Manabrew
failure and an incomplete priority menu rather than a cached negative result.

## Stage43 combined source review

The corrected chain is integrated through `c8232347` and received a bounded
read-only combined-owner review. Event receipt capture/history staging stays
inside the resource wrapper; replacement and per-player completion owners
propagate typed errors and restore originals, captured trigger queues and
context. Both scheduled-turn and resource GameState implementations survived
integration. No build, compiler probe or test was executed.

The current frozen matrix has 21 unique token-dependent proposals (22 entries
including the reversible Jinnie alias). Their known silent-cap source gap is
now source-addressed, while all exact-count/rollback/runtime gates remain
deferred. Later token-dependent proposals inherit this unvalidated resource
primitive; larger or exhausted operations remain explicitly incomplete, never
silently shortened. The separate u32 damage-saturation gap remains open.
