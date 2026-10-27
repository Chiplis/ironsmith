# Effect composition contracts

Public effect payloads remain the authored and serialized vocabulary. Runtime adapters may expand them with `EffectExecutor::canonical_program`, or execute shared children inside a retained rules boundary. The internal programs in `effects/composition` are transient execution structures; they are reconstructed from public payloads, not serialized as additional effect kinds.

## Choose the owner before expanding a body

An ordinary sequence is suitable when each child completes before the next instruction starts. Keep the existing prepared owner when an instruction requires original actions to commit before replacement additions, simultaneous actions, entry-time modifications, or a single casting/payment lifecycle. Shared helpers may execute inside that boundary without changing who publishes completion.

Examples:

- Connive binds the exact discard receipt before counting nonlands.
- Manifest dread keeps prepared face-down entry and commits the other original moves before their additions.
- Exile-then-grant installs permission for exact arrivals within original completion, before added programs can move them again.
- Unearth installs its departure behavior for the exact battlefield arrival.
- Amass and empower recheck eligible objects after replacement-aware token creation, without an intervening state-based-action check.

Do not replace these boundaries with a sequence merely to remove an executor name.

## Bindings and results

`ScopedProgram` freezes player bindings before installing them, owns lexical object tags, and restores caller bindings on success, suspension and error. `ScopedOutcomeProgram` does the same for an instruction result slot. `OutcomeBranchProgram` branches on a producer's instruction result instead of an unrelated aggregate count.

Use `TaggedSnapshotCount` to count retained characteristics, even after the card has left its original zone. Use exact current IDs for permission and casting eligibility; never follow stable card identity to authorize a later incarnation. An original action summary may retain arrival snapshots after a subsequent move (`retain_result_snapshots`), while `ExactTaggedObjects` only returns still-current eligible objects. These are different contracts.

The original action's primary result and the combined events/facts of its children are also different contracts. Preserve both using `CompletedEffectOutputs`, and return neutral outputs on suspension through the existing transaction machinery. Do not discard additional-effect facts or count them as the original action.

Casualty sacrifice evidence is keyed by the complete typed optional-cost reference, including its printed-instance discriminator, and recorded when payment completes. Both native and compiler builders bind that same reference into the copy adapter. Numbered sacrifice-choice tags are not proof that a sacrifice paid a particular optional cost.

## Structural inspection

The core `effect_capabilities` table is shared by compiler and runtime. Every registered payload must have a deliberate library-movement policy; its registry coverage test prevents omissions. Unknown native callbacks conservatively disqualify mana handling.

- `Leaf`: the instruction does not cross a library boundary.
- `Moves`: authored semantics include such movement.
- `Inspect`: inspect parameters, immediate children and lexical origins.
- `Deferred`: owned bodies run later and do not execute during this instruction.
- `Unknown`: no declared contract; conservatively treated as movement.

`visit_child_effects` exposes owned structure. `visit_resolution_child_effects` excludes deferred bodies. These relations must not be interchanged: granting a future draw or mana ability does not execute that ability now. Native bodies with branch-local bindings can implement `inspect_library_movement_override`; the shared analysis isolates alternatives and merges their possible origins.

Structural visitors must not allocate card/object identities or perform gameplay mutations. Token adapters retain or build inspection-only blueprints without advancing identity counters.

## Shared payload options

- `ChooseObjectsEffect.public_selection` opens selected hidden identities for choice replay without inventing a rules reveal event.
- `ExileEffect.remember_on_source` records imprint association at original exile completion.
- `GrantPlayTaggedEffect.exact_tagged_objects` forbids stable-identity rebinding.
- `CopySpellEffect.set_base_loyalty` evaluates a value before creating copies and stores a copiable starting-loyalty override.
- `VariableCasualtyPlaneswalkerCopyEffect.cost_ref` binds the exact optional-cost instance. Its old unit payload remains readable with the historical generic reference.

Defaults preserve the prior behavior of payloads that omit these fields. The complete artifact envelope is version 19 and rejects prior versions; rebuild compiled caches from source. The historical version-18 fixture and descriptor remain immutable.


## Resolved owners added by the 2026-10-13 backlog

- `commit_retarget` accepts an exact spell/ability identity and a validated assignment. It prepares distribution remapping before updating the stack, refreshes cast LKI, removes stale pending targeting observations, and returns one event per distinct new target. Choice and legality stay with the facades.
- `ResolvedRestrictions` registers frozen restrictions with independent timestamps and a caller-selected refresh boundary. Ordinary replacements use `ApplyReplacementEffect`; followed pre-cast objects and next-damage-occurrence consumption keep their specialized store owners.
- Fixed-kind player counters share `PlayerCountersEffect::execute_selected_with_outputs` without inheriting its simultaneous-action capabilities or decision-controller reset policy.
- `select_typed_option` returns a typed transient result. Strict single answers and first-index-or-none answers are separate policies. Persistent chosen state remains in each facade; planned mana and secret subtype paths retain their own protocols.
- `execute_production` resolves the same `ManaProduction` descriptor used by planning, chooses through the existing color/type protocol, and credits through replacement-aware mana receipts. Symbol, authored-count, and credited-count projections are explicit. Single-color manual prompts ask once; planned witnesses describe the entire output.
- Library ordering captures exact surviving incarnations from one owner and normalizes permutations independently of top/bottom orientation. Position changes remain distinct from zone movement.
- `register_resolved_continuous_batch` never refreshes between captured writes. Text exchanges capture both overlays first. Multiple ability grants retain separate instructions and timestamps through `apply_captured_modifications`.
- `commit_control_assignments` prepares all assignments against pre-change characteristics, registers the complete batch, and reconciles control once. Exchange uses all-or-nothing eligibility; directional rotation permits independently selected participants. Callers retain pending-versus-returned event policy.
- Optional-cost power is read from the complete printed cost reference using context/live-spell/cast-LKI precedence. `WithOptionalCostPower` binds a lexical result slot and restores it on completion/error/suspension; it does not overwrite the source spell's X.
- Captured object selection accepts an explicit chooser. Contributor selection distinguishes contribution thresholds from exact cardinality; payment and completion remain in the tap-cost owner.
- `ResolvedCastOffer` owns optional confirmation, face selection, payment and cast completion, with explicit failure projection. Candidate discovery and hidden-hand opening remain outside it. `FirstMatchCastProgram` owns consultation, optional prelude, exact-card offer, replacement-aware hand fallback and cleanup. Its callers retain lexical tags and original-result projection. `offer_refreshed_casts` rebuilds legal options after every successful cast and captures triggers before another payment can change watchers.

## Remaining adapter consolidation

- Combat prevention uses one body builder for execution and structural inspection. The to-and-from adapter captures exact subjects before composing the two shields, and retains a transaction boundary around the pair.
- Selected counter-removal adapters share `execute_selected_removal_transaction`. It preserves the neutral result on suspension, including suspension accompanied by an error; selectors, cost capabilities and simultaneous proposals remain separate.
- Reconfigure delegates detach to `UnattachObjectsEffect` and attach to the resolved attachment owner shared with `AttachToEffect`. Its exact battlefield-source guard and targetless detach remain local. Color choice on attachment is an explicit caller policy.
- Scheduling adapters share `ScheduleOperation` and `execute_schedule_operation`. Eligibility is checked before claiming the team operation; existing instruction claim keys, counted skips, current-turn gates and extra-turn insertion order are retained.
- Regeneration builds the same replacement body for registration and structural inspection. Follow-up player bindings are frozen at registration, and payload children are excluded from resolution-time inspection because they execute only when the shield is consumed.

## Resolution adapters and collected choices

- `ResolvedPowerToughness` captures numeric values only for resolution-created
  continuous effects. Its additive and setting builders remain distinct (layer
  7c versus 7b). Callers retain their target validation and frozen mass-target
  sets. Counter placement, static/CDA evaluation, copying and transforming do
  not use this helper.
- `execute_payable_choice` rebuilds a menu of currently payable child actions,
  waits for a completed choice, and executes the selected child through the
  shared output-preserving gateway. Its eligibility selector is also used by
  Forage validation. It does not replace atomic cost payment or keyword
  completion; Forage retains source LKI and completion for each repetition.
- `collect_choices_before_commit` captures each participant's selection in the
  supplied order and returns a plan only after all choices finish. Clash and
  EachPlayerScry use it under their existing transaction boundaries, before any
  selected library arrangement is applied. Disclosure and keyword completion
  remain with the mechanic. `choose_library_bottom_partition` selects from
  exact captured identities without revealing, moving or emitting an event.

## Captured choices and lexical bindings

- `select_typed_menu` pairs values with display metadata. `First`, strict
  single-answer validation, and first-option fallback are separate policies.
  Suspension always returns no selection before applying a fallback. The
  decision-maker/source interface also serves library replacement replay.
- `choose_captured_object` retains the caller's selection protocol: direct
  menus versus decision specs, explicit choices, singleton and empty-menu
  prompts, first/first-valid/last answer handling, and fallback behavior.
  Hidden hand selection remains with its private-information owner.
- Attachment assignments, directional control, friend/foe labels and named
  player options collect complete plans before publishing their results.
  Player ordering explicitly distinguishes seating order, filtered turn order
  with seating fallback, APNAP rank, and selected-player completion.
- Source scopes accept an already-resolved source/snapshot pair. Actor scopes
  replace the controller and optionally the cause. They preserve unrelated
  successful outputs; die payments retain the enclosing cause, while life
  auction winner programs temporarily replace it.
- Object-tag scopes restore lexical bindings in removal order. Correlated
  iteration separately exports its result tag on success and restores it on
  failure, including legacy alias behavior. Temporary card scopes surround
  their materialization transaction and clean up only the exact unused copy.
- Filter-reference traversal has bounded field policies. Controller/owner,
  spell facets, and the wider authored-cost facets are distinct. Wrapper
  recursion, unresolved-reference fallback, tagged-object identity and numeric
  comparison freezing are chosen by the registration owner.
- `retain_extrema_by_key` evaluates each metric once, excludes absent values,
  and keeps every extreme tie in input order. Both retained per-player mana
  values and current-characteristic minima use it. Association, snapshot
  retention, direction and tag cleanup remain explicit at the call site.
- Resolved destruction completion accepts an exact authorized incarnation and
  retains the replacement payload's decision scope and no-receipt projection.
  Ordinary target destruction still owns target validation and LKI capture.
  Manifest selection and entry share a replayable transaction; source-return
  summary projection preserves observations and the original instruction result.
- Delayed exile shares its body builder with structural inspection while
  keeping one watcher/body per exact subject. Its children are deferred and
  excluded from resolution-time traversal.
