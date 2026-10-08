# Intervening predicate cohort: source proposal, UNVALIDATED / UNRUN

## Current NEXT03 disposition

Independent final source review cleared `92168f50060c362c88d5c1ecf6477dbbffa11839`. The [NEXT03 admission](../../architecture/card-failure-next-series-03-source-admission.md) records only the bounded source proposals: predicate 5, copy 4 IDs / 5 entries, suspended 4, numeric 4, plural untap 5. All held neighbors remain excluded. All executable scenarios are **UNRUN**, the source remains **UNVALIDATED**, and no new measured recovery is claimed. The historical scoped-work notes below describe their original stages; coordinated compatibility is now artifact 14 / digest 9 / audit 27.


## Scope and provenance

- Base: `441771fd5c9562250e503d7e17260ce1b320c298`, tree `7e88f686d23b0503d0d1c755e1801abfdd9c340c`.
- Isolated branch/worktree: `source/intervening-predicates`, `ironsmith-intervening-predicates`.
- Failure membership: retained `refresh-20261007-main5cc46c1/current-failures.json.gz` in the adjacent `ironsmith-next-series-01` worktree. All seven rows are current `parser_failure` rows for unsupported intervening-if predicates.
- Full text and metadata: frozen `fixtures/card-failure-campaign/cards-20261003.json.xz`. Reminder text was retained in the new bounded fixture.
- No build, test, probe, formatter, corpus run, artifact generation, compatibility-constant edit, remote write, reset, or discard was performed. Authored scenarios are not passing measurements. No source-coverage ledger or published packet was changed.

## Disposition

Five full-body proposals now have bounded final source clearance:

| Card | Oracle ID | Proposed ownership |
| --- | --- | --- |
| Aurora Champion | a4499cf9-04a0-45c5-9c73-19f8e7e95da6 | Team-scoped other-subtype existential |
| Bull-Rush Bruiser | d5a7412a-2258-4d3e-9e0f-a150cc69081e | Team-scoped other-subtype existential |
| Sickle Dancer | 836bfcad-014a-40cc-ab79-906fcab723a0 | Team-scoped other-subtype existential |
| Dragonfly Swarm | 83abf1d0-04e4-49e5-bf63-59f77829ffd1 | Contracted owned-graveyard existential |
| Walltop Sentries | fb3a0910-a582-41ef-b5b9-3cda1f1cd5ce | Contracted owned-graveyard existential |

Two candidates remain held, with no admitted-for-review completion claim:

| Card | Oracle ID | Substantive blocker |
| --- | --- | --- |
| Consul's Lieutenant | a38b672f-4739-4b7e-8958-2a455510656e | Recognizing the contracted renowned clause is insufficient. `Condition::SourceIsRenowned` consults `game.is_renowned(source)`, a live battlefield flag. Departure removes that flag and `ObjectSnapshot` has no renowned designation. The attack trigger must still resolve using the departed source's last known information; a general pronoun predicate also requires reference-aware binding instead of coercing every `it` to source. No source-only grammar shortcut was admitted. |
| The Notary Hobbits | dc20b29b-856e-4d45-99cd-5e12cfc2324d | The contracted plural subject is not among the source identity recognizer's supported references. The complete body also requires `them` to identify the same copied source, plural nonlegendary exceptions, and suppression of the copied tokens' own ETB triggers. Those correlated references were not established by this bounded change. No global pronoun normalization or partial card admission was made. |

This is a 7-candidate / 5-proposed-for-review / 2-held packet. Independent source review is complete; executable validation and a new measurement remain outstanding. It is not a claim of five newly passing cards.

## Typed owners and semantics

### Independent-review correction

The initial commit `f3d3d4d29527fba642d897cd39ea9e8b2835a950` is insufficient on its own. Independent review identified two blockers addressed by the subsequent correction commit:

1. A bare `ObjectFilter.other` is target-context-sensitive. While one Aurora trigger resolves with another targeted trigger from the same attacker still on the stack, the latter's target can enter the filter context. The old representation could exclude that target and incorrectly count Aurora itself. The replacement below uses exact-source subset cardinality, without changing global `other` behavior or requiring target emptiness.
2. `compile_to_artifact`'s second returned definition comes from artifact materialization. The initial helper incorrectly called that result the independent direct route. The corrected helper separately invokes `compile_to_runtime_definition` on the original complete metadata/body source, captures that call's parse loss independently, then separately compiles, validates, JSON-decodes, and materializes the artifact. Every scenario runs against those two independently produced definitions. Neither route has been executed under this source-only restriction.

Independent source re-review cleared correction commit `92713682ad6407ff137681630702a2d16c41e0ad` for these two findings. The subsequent test/report-only coverage addition asserts each proposed card's output mana cost, subtype vector, and printed power/toughness, including Dragonfly Swarm's printed star. It also adds unsupported `and` tails alongside `or` tails in both predicate-only and complete triggered-line negatives. Production semantics, frozen fixtures, and the five-proposed/two-held disposition are unchanged; source clearance is not executable validation.

### Owned graveyard existence

The existing single-card and independently-articled conjunction readers now accept the exact contracted existential `there's` alongside `there is` and `there are`. Their bounded object and location captures are otherwise unchanged. The result remains `PredicateAst::Player(PlayerControls { You, filter })`, with `filter.zone = Graveyard` and `filter.owner = You`.

Keeping the conjunction reader in step prevents `there's an instant card and a sorcery card in your graveyard` from being weakened to one disjunctive card requirement. Unknown locations and unconsumed tails remain unsupported by these readers.

The runtime owner is the existing `Condition::PlayerControls` zone-aware evaluator, where a nonbattlefield zone uses the named player's cards. This retains the trigger controller's own graveyard, including for a creature owned by another player. It does not pool teammates' graveyards.

### Team control existence

The existing control-predicate registry reading recognizes exactly `your team controls another <one recognized subtype>`. It emits an existing `ValueComparison(Count(team_filter) > Count(source_filter))`. Both filters require the same battlefield zone, named subtype, and `PlayerFilter::your_team()` controller scope; the right-hand filter additionally requires `source = true`. Neither filter sets `other`. It neither assumes that a Warrior is necessarily a creature nor adds an ownership restriction.

`PlayerControls { You, filter }` would be incorrect for this family: its runtime owner first narrows objects to the single named player's controlled permanents, which would exclude teammates before the filter could match. The chosen count uses the existing union controller filter, honors the current controlling player of each qualifying permanent, and consults the ability controller's team. In free-for-all, that filter includes only you. The right-hand count is either zero or one and uses the existing exact `ObjectId` source check in `filter/matching.rs`. Thus the comparison requires at least one matching permanent other than that exact source incarnation. Targets and sibling stack entries cannot change this exclusion. A departed source is absent from both sets; a returned blink incarnation may legitimately qualify as a different permanent despite sharing a stable card ID. A source that leaves the original controller's team is likewise absent from both sets, while the ability retains its original controller.

The structural renderer recognizes only the exact paired-filter comparison and emits `your team controls another <subtype>`. An extra owner restriction, missing exact-source requirement, different relation, or other filter distinction does not receive that simplified rendering. No Oracle-text echo, card-name dispatch, no-op, or placeholder was added.

### Routes and wire impact

The predicate AST owners, resolver cases, runtime conditions, value/filter evaluators, compiler-runtime bridge, and artifact materializer already support these variants. This change adds no AST, wire, effect, condition, value, filter, snapshot, or protocol variant. Existing serialized `Value::Count`, `ValueComparisonOperator::GreaterThan`, and `ObjectFilter.source` carry the exact-source subtraction-by-comparison; global runtime filter logic is untouched. Compatibility constants are unchanged. The correction changes the typed predicate payload emitted for the three team cards and equivalent bounded predicates. No artifact was generated or migrated, and artifacts compiled from the initial commit's unsafe predicate are not validated by this source correction.

## Authored coverage, all UNRUN

- `fixtures/intervening_predicate_cohort.json.fixture`: all seven frozen full bodies and metadata, including both explicit held records.
- Grammar tests: exact typed graveyard ownership/zone/subtype; contracted and uncontracted forms; independently-articled conjunction; exact team count; unknown subtype, quantity, location, and suffix negatives, including both unsupported `and` and `or` tails.
- Runtime integration tests: complete original bodies through independent `compile_to_runtime_definition` and separately compiled JSON-restored artifact materialization, independent parse-loss capture for both compilations, artifact structural equality and validation, no unimplemented content, independent literal mana-cost/subtype/printed-P/T expectations for every proposed card on both routes, and independent expected trigger rendering.
- Team runtime scenarios: trigger-time and resolution-time checks; self exclusion; own and teammate qualifiers; either opponent negative; noncreature Kindred Warrior and token Warrior positive; wrong zone/subtype; no retrospective trigger after the condition becomes true; source-only first strike/pump; exact tap recipient and land-target exclusion; duration cleanup; phasing, departure, and enemy control of the qualifier; unchanged ability controller after source control changes; replacement qualifier; cloned-state continuation; free-for-all.
- Additional review regression: a real attack-trigger doubler creates two separately targeted Aurora triggers. With the lower targeted sibling remaining on the stack, resolving the top must fail after the sole qualifying Warrior leaves or changes to an opposing controller. Explicit controls cover source control change with/without a remaining teammate Warrior, noncreature Kindred Warrior qualification, source departure, and a blinked new source incarnation with the same stable card ID. Both copies resolve on both independent routes and cloned continuations.
- Lesson runtime scenarios: trigger-time and resolution-time checks after real zone movement; own/teammate/both opponent graveyard distinctions; exile and non-Lesson negatives; no retrospective trigger; stolen source's last controller and reward recipient; qualifying Lesson removed or replaced before resolution; cloned-state continuation.
- Full-body companions: Dragonfly Swarm's live noncreature/nonland graveyard characteristic power, controller-relative count, flying, ward, actual mana payment/countering and teammate targeting exclusion; Walltop Sentries' reach and deathtouch remain represented on the runtime object.
- Tools metadata tests: all five proposed full raw bodies must be `StrictCompiled` without lossy metadata fallback. No first-clause or stripped-body proxy is used.
- Renderer unit test: exact paired-count team existential surface plus missing exact-source and additional-owner negative controls.

## Future validation gate

The requested source-only restriction prevented execution. The focused grammar tests, text renderer test, compiler-runtime integration file, and tools metadata file must be compiled and run by an authorized validator before executable validation. Review any failing assertion against independently authored semantics; do not update expected output merely to match observed output. Measured whole-card corpus status remains unchanged until a fresh authorized measurement; source-only admission counts are tracked separately in NEXT03.
