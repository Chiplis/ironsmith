# Singular counter recipient grouping: source-only candidate

Base: `6ca01fa1616181766b654f4b3f4ee6cc74ed8757`. This is a bounded shared-owner
correction for All Will Be One, not a runtime completion or coverage claim.
All new scenarios are **UNRUN**. No builds, compilation, cargo commands, tests,
probes, formatters, artifact generation, publication, version edits, or ledger
count changes were performed.

## Original failure and retained producer ownership

`ProliferateEffect` freezes every selected recipient and existing counter kind,
then calls `execute_counter_batch_with_outputs`. That owner prepares all
placements before mutation, commits each original, assigns a shared instruction
batch to the actual `MarkersChangedEvent` receipts, freezes the complete original
world, and only then completes additions. A selected permanent with two kinds
therefore correctly produces two physical receipts in one batch. The old
`CounterPutOnTrigger` only grouped the separate wording "on one or more objects";
the singular "on a permanent or player" matched both receipts independently.

No producer, replacement, prepared-original, observation timing, or completion
code changes. Distinct Proliferate iterations and distinct instructions keep the
batch identities allocated by those owners. Reported-event capture and pending
event drain gather by those identities before matching. Unbatched instructions
continue through individual matching.

## Matching, queue, and value ownership

`CounterRecipient` is separate from the existing plural `CounterBatch` key. Its
recipient is the exact typed object/player target. Authored actor subjects also
include the physical receipt's actor, independent of source or recipient
ownership. Singular one-or-more counter matchers use it; each-counter and ordinal
matchers do not. Existing kind filters and actor matching run before grouping.

The ordinary simultaneous queue stages all matched singular-recipient groups
before publishing any queue entry. Per-event occurrence indices preserve
identical ability instances. First-receipt order is retained. Delayed
registrations group within each registration and watched source before deciding
which whole-recipient events are one-shot alternatives. Plural-object grouping
keeps its existing independent contract and quantity behavior.

A private native `RawEvent.counter_trigger_amount: Option<i64>` belongs only to
the queued trigger's projection. `checked_add` completes before writing even a
staged projection. The original marker kind, amount, count-after, occurrence,
batch, actor, snapshots, and turn-history receipt remain intact. Envelope clones
and payload enrichment retain the projection. `EventValue(Amount)` reads it via
the existing wide value evaluator; the generic `Option<i32>` trigger override is
unchanged. Damage keeps its existing checked u32 conversion and typed rejection
when a valid wider counter total cannot fit that consumer.

An i64 sum failure returns `ExecutionError::ResourceLimitExceeded`, recorded in
the existing execution failure meter by both ordinary and delayed queue owners.
The typed `capture_triggers_before_added_program` and
`drain_pending_trigger_events_with_dm` adapters return that exact failure and
restore their game/event/queue checkpoints. No part of the failing ordinary
batch is inserted, and delayed registrations are not consumed on failure.

The follow-on correction adds `try_drain_pending_trigger_events`, which keeps
the old matching-only behavior: it does not execute duration-end returns or
introduce decisions. Counter-capable reported-event and combat queues also
receive checked wrappers. Each owns its failure scope and game/queue checkpoint.
The internal instruction-boundary matcher now returns `Result<bool, ExecutionError>`;
its six production consumers propagate that result. Disabled/held boundaries
retain their cheap early returns. Queue paths formerly matching each receipt
independently now group counter batches only; other event families retain their
previous behavior.

Turn advancement and priority response/context entrypoints restore their native
state on incomplete execution. Previously selected priority actions keep their
existing error/pending rollback contract. Ordinary cancellation and completed
game-ending control flow are not converted into retries. Immediate mana-trigger,
special-action, combat mana-window, and convenience priority-loop adapters retain
the underlying execution failure instead of discarding or relabeling it.

The original infallible public helpers remain source-compatible, including their
lack of an error return when called standalone without an active scope. The
supported standalone entrypoint is now `try_drain_pending_trigger_events` (or the
existing decision-aware checked drain); combat publication similarly offers
`try_queue_combat_damage_triggers`. Remaining production uses of the old drain
are the two runtime matching internals, both enclosed by checked scopes. Other
old calls are tests and diagnostic probe binaries, which were not executed.

## Production caller inventory for the boundary correction

This list came from source reads and a full tracked-tree search, including
directories absent from the initial sparse checkout. The following original
drain callers now use the checked matching-only entrypoint:

- `turn_runner.rs`: `TurnRunner::advance` (five sites, now in `advance_inner`)
  and `apply_sbas_until_commander_choice` (four sites).
- `game_loop/priority_apply.rs`: `begin_mana_ability_activation` (two sites),
  `apply_priority_response_with_dm_inner` (three sites, including its land-entry
  observation callback).
- `game_loop/priority_cast.rs`: `finalize_pending_spell_cast`,
  `auto_pay_spell_tap_cost_steps_inner`, `continue_spell_cost_payment`,
  `apply_declaration_mana_ability_window_response`,
  `continue_activation_remove_counters_among_payment`,
  `continue_activation_cost_payment`, and `auto_pay_activation_tap_cost_steps_inner`.
- `game_loop/priority_mana.rs`: `execute_planned_mana_activations`,
  `execute_planned_keyword_payments`, `execute_pending_mana_ability` (two sites),
  `apply_sacrifice_target_response` (seven sites), and
  `apply_card_cost_choice_response` (seven sites).
- `game_loop/sba_triggers.rs`: `check_and_apply_sbas_with_inner` (two sites),
  `resolve_triggered_stack_entry_immediately`, and `resolve_pending_mana_triggers`.
- WASM `wasm_game_impl/dispatch.rs`: `force_turn_face_up_with_dm`,
  `add_card_to_zone_with_dm_inner` (three sites), and the legend-rule live branch
  in `dispatch_routed_command`. The latter restores the replay checkpoint and
  pending decision/action on failure before returning its existing JS error.
- WASM `wasm_game_impl/undo.rs`: `add_definition_to_zone_with_triggers_inner`
  (two sites), inside its existing replay checkpoint wrapper.

The runtime `match_triggers_at_instruction_boundary_inner` and
`capture_triggers_before_added_program_inner` keep their infallible internal
drains, now both guaranteed to have a locally owned failure scope and checkpoint.
Direct delayed-simultaneous discovery callers are single-event delayed matching,
reported-event batching, pending-event batching, and combat publication. Actual
counter-capable multi-event calls run inside the checked owners above.

Additional counter-capable reported-event callers are life-payment publication,
Saga lore placement, Attraction rolling, ordinary stack resolution, and immediate
mana-ability outputs. Combat publication runs against the existing hypothetical
game/queue before the turn runner publishes either. Pure control, monarch, tap,
attack, block, and Attraction-visit observations retain their old signatures;
those concrete event lists cannot enter counter-recipient aggregation.

## Native transport and exported views

`RawEvent` derives `Clone`; payload enrichment explicitly retains the private
projection. Completed-receipt enrichment begins from the receiver clone, keeping
a queued projection while never copying it into an ungrouped physical alias.
`TriggeredAbilityEntry`, `TriggerQueue`, `StackEntry`, and `GameState` clone the
full envelope. Stack creation and resolution pass that envelope unchanged through
`with_triggering_event`. `ExecutionContextCheckpoint` includes `triggering_event`
in its owned-field capture/restore list, and action-program participants retain
that checkpoint. WASM `ReplayCheckpoint` and `RuntimeSavepoint` clone full game,
trigger queue, priority state, and continuation state.

There is no supported serialized live-stack export-and-rehydrate carrier in the
tracked source: `RawEvent`, `StackEntry`, and `GameState` have no serde carrier.
`StackObjectSnapshot` and `SyncStackEntry` are Serialize-only presentation/public
consensus projections with no importer; recovery uses native savepoints or
signed action replay. Those exported views, and `RawEvent` diagnostic Debug,
do not expose the private counter sum. That is a visibility/public-projection
limitation, not evidence of lost state during gameplay recovery. This candidate
does not change those view schemas or their version gates.

## Authored regression coverage and compatibility

`counter_recipient_trigger_groups.rs` compiles the complete frozen All Will Be
One body directly and through serialized artifact materialization, then uses
actual Proliferate and DoubleCounters producers. Scenarios cover multiple kinds
on each of two permanents and a player; saved damage amounts and targets after
source-control/recipient-state changes; repeated proliferation and separate
instructions and unbatched receipts; actor identity; identical ability instances; plural-object,
each-counter, and kind-filter contracts; totals above i32; typed damage rejection
above u32 with stack recovery; delayed one-shot versus recurring watchers; and
target-choice suspension followed by native queue/stack checkpoint recovery.

Native arithmetic scenarios reach the i64 boundary by amplifying only a private
projection on real producer receipts, avoiding billions of allocated events.
They assert unchanged projections on checked-add failure and exact typed
capture/drain rollback, including delayed registration retention. Additional
top-level scenarios cover turn advancement, full-turn execution, a land-play
priority action, and the repeatable special-action instruction boundary.
WASM savepoint scenarios use the complete All Will Be One body and actual
Proliferate/DoubleCounters producers, retaining ordinary and above-i32 amounts
through queued, stacked, copied-stack, and restored native branches.

No serialized card model, artifact codec, artifact version, or gameplay wire
format changed. The native public `SimultaneousTriggerKey` enum gained the
`CounterRecipient { recipient, actor }` variant; `RawEvent` gained private
in-process projection state. Existing public function signatures are unchanged;
checked drain/combat functions are additive public APIs. The internal matcher,
per-event output queue, and activation-notification adapters now return Result.
All runtime verification remains deferred; additional native checkpoint cloning
has not been performance-measured under the no-execution constraint.
