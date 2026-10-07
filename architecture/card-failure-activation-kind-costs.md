# Activation-kind scoped cost modifiers

Status: source implementation and authored scenarios, **UNRUN / UNVALIDATED**.
No build, compilation, compiler probe, test, formatter, browser, replay, or
artifact generation was performed. No coverage ledger or version gate changes.
Base: `5d993c24f934059039f580bbfd2f6a466464681b`.

## Frozen family

`fixtures/activation_kind_costs.json.fixture` retains the exact frozen identities,
metadata, complete Oracle bodies and baseline-e8740178 diagnostics for Fluctuator,
Silver-Fur Master, Dragonkin Berserker, Boom Scholar, Hulk, Gamma Goliath, Eidolon
of Obstruction, Suppression Field and Zirda, the Dawnwaker. No card name selects
behavior. The corresponding runtime scenarios compile each full body directly
and independently through the typed artifact/JSON materializer path.

The shared missing capability is a cost selector on the activated ability being
announced: Cycling, Ninjutsu, Boast, Exhaust, Power-up, loyalty, or nonmana. These
are not properties of every ability of the same card. The complete grammar also
retains the activator, controlled/other source filter, generic amount, count of
matching Dragons, mana-ability exception and one-total-mana reduction floor.
Unconsumed qualifiers and extra sentences fail. Silver-Fur's obsolete prefix
rejection is removed only alongside this consuming grammar; a cost sentence
cannot fall through to a keyword marker.

## Typed identity and execution

The core `ActivatedAbilityKeyword` gains appended Cycling/Ninjutsu/Boast/Exhaust
identities after Equip/PowerUp. Cycling and Ninjutsu constructors stamp their
actual ability; the recognized Boast and Exhaust grammar stamps the same typed
identity independently of runtime presentation. Granted/copied abilities retain
it through the shared ability mapper and native clone paths. An ordinary
activation with equivalent costs or effects never acquires it by resemblance.

`ActivatedAbilityCostCondition` appends Keyword, NonManaAbility, LoyaltyAbility
and Activator. The runtime interpreter maps each gate exhaustively; the typed
text-change visitor traverses player/source/count filters while keeping keyword
identity unchanged. The count filter owns its rendered suffix, so changing an
authored subtype does not leave a duplicate stale count in the display.

`ActivationCostAbility` captures the selected keyword, runtime mana classification,
loyalty classification, actual activator and ability index. Costs are evaluated
relative to each modifier's controller/source. A source-owner filter and an
activator filter remain independent: for example, Fluctuator follows the player
who cycles, and Eidolon follows the controller of the selected planeswalker.

The existing total-cost owner in `decision/mana.rs` collects additional costs,
merges mana surcharges, and only then applies generic reductions and their
one-mana floor. Generic reductions are applied in the activator's least-cost
legal order: larger floors first, unbounded reductions last. Fluctuator plus
Zirda therefore has the same price in either battlefield insertion order.
Colored pips, taps, loyalty counters, discards and Ninjutsu's
unblocked-attacker return remain mandatory. A generic reduction cannot pay a
colored pip. X is locked before determining the actual total. Dragon counts
are read in the modifier's context at cost determination; payment does not
recount them. Power-up retains its existing conditional source-mana subtraction
and its once-per-acquisition execution owner. Its dynamic base also locks X
before the external reduction is determined.

Root/pending and direct/planner mana activations share original branch selection
and X announcement before flattening or paying a TotalCost. The selected branch
is priced after X locks, so an Exhaust mana ability can use Boom Scholar's
reduction on that X. The maximum uses the applicable reduction capacity and
fully repriced fixed candidates rather than a fixed trial X. A source's reserved
tap cannot fund its own activation. Only affordability queries may consider an
unannounced dynamic base at X=0; payment still requires the announced value.
Alternative branches remain isolated and only one is paid. The maximum's
read-only query retains shared incomplete/resource failures through the existing
checked legality owner instead of interpreting a truncated potential pool as zero.

Actual admission routes include legal action generation; direct and pending
activation; direct/pending mana activation; the mana planner; and the existing
prospective-reference and counter-declaration preflights. All real routes pass
facts about the selected ability. The general no-ability estimate API does not
invent a new keyword, nonmana, loyalty or activator gate.

## Pending original and recovery owners

`PendingActivation.announced_cost` retains the original typed ActivatedAbility
and pricing facts together before payment. Target, X, and prospective-reference
repricing read that original, never a changed current display slot. Losing the
required original produces the shared
`ExecutionFailed(ExecutionError::IncompleteEvidence(...))` error; it is not a
zero price or a current-object reconstruction. The public response transaction
restores the pre-response game, queue and pending lane after incomplete evidence;
explicit root cancellation restores the pre-activation resources and use limits.

The existing payment original/completion machinery remains responsible for
replacement-aware Cycling discards and Ninjutsu returns, subsequent mana and
nonmana costs, actual keyword events and stack effects. Failure/cancellation
restores the outer action; resolving/countering a paid activation does not refund
its costs. This family adds no payment shortcut or replacement bypass.

Native PriorityLoopState clones retain the entire original. WASM source scenarios
exercise RuntimeSavepoint capture/clone/exchange/restore, ReplayCheckpoint,
inactive Grand Melee host lanes, and suspended hosts, including a branch where
the original was deliberately removed. These are native owners, not serialized
gameplay recovery APIs. Accepted action replay reconstructs the original at
admission; replay execution remains deferred.

## Compatibility and limits

The coordinator owns the next compatibility decision after staged artifact10 /
public audit6 / signed protocol23. No version is changed here. The appended core
enum variants and newly populated keyword metadata are compiled semantic shapes;
old artifacts without these identities must be regenerated at the coordinated
boundary, not inferred from descriptions. Native pending originals have no wire
recovery encoding. Public audit does not export pending activation continuations;
the existing typed restricted-mana/effect projections may carry the enlarged
ability model and need the coordinator's independent compatibility review.

The bounded eight full-body candidates have no separately claimed partial body
in this patch. Unrelated first-activation reducers, colored self-reductions,
method-specific casting prices, morph/foretell/plot/unlock special-action prices,
and new keyword families are outside it. Synthetic witnesses exercise ownership
and failure boundaries but are not additional frozen-card proposals.

See `card-failure-activation-kind-costs-scenarios.md` for authored runtime cases.
Additional grammar cases live beside the consuming reader, and native owner
cases in `wasm_game_impl/activation_kind_cost_savepoints.rs`. All remain unrun.
