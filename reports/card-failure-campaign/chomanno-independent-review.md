# Independent source review: Cho-Manno whole-body evidence

Reviewed commit `544bdc3bd6197d2d3d3b39c477390dbf5972bc50` against `94e075587` in `ironsmith-chomanno-full-body`.

## Verdict

Bounded source-clear: no concrete blocking defect identified in the new fixture/test/report patch. This is **not runtime validation**, a recovered-card admission, or a coverage-count update. No builds, tests, probes, formatters, corpus execution, code generation, or remote writes were performed. Review used source reads and a read-only extraction of the frozen record. Checkout HEAD matched the requested commit and was clean.

The diff adds exactly three files: the 10-line fixture, 163-line integration test, and source-review report. It changes no production implementation.

## Evidence inspected

- `fixtures/chomanno_full_body.json.fixture`: every supplied field matches the local `fixtures/card-failure-campaign/cards-20261003.json.xz` record for `91af5e35-b3b8-43ce-b1ea-997ed74e4ad2`, including the shortened self-reference, full name, `{2}{W}{W}`, `Legendary Creature — Human Rebel`, 2/2, and Scryfall URI. No text rewrite substitutes generic self-language in the fixture.
- Test lines 20–60 verify source identity/body and compile full metadata plus Oracle text twice. Lines 35–42 independently call `compile_to_runtime_definition(..., false)` and `compile_to_artifact(..., false)` with separate parse-loss captures. Lines 43–48 validate, serialize, deserialize, compare, revalidate and materialize the separately compiled artifact. They do not generate the artifact from the first runtime result. Compiler-runtime implementation at `src/lib.rs:660–745` confirms these are distinct public compile paths and strict policy is passed through.
- Lines 49–60 reject unimplemented content, require the intended creature/2/2 shape, no spell program, and exactly one native `PreventAllDamageToSelf` static ability. All three test functions iterate over both independently obtained definitions.
- Lines 82–109 require repeatable prevention for both combat flag values, damage sources owned by either player in battlefield/stack/graveyard, and prevention amount/source/controller events. Same-name abilityless peer and both players remain unprotected; unpreventable damage remains 3 with no prevention event. Stack emptiness checks absence of activation work.
- Lines 112–141 require current-controller attribution after A-to-B control change; disabled/re-enabled prevention across phasing; continued protection after artifact type replacement; loss after `RemoveAllAbilities`; inactivity in graveyard; distinct returned incarnation; old-ID nonprotection; and native returned protection attributed to owner A. Return uses `move_object_by_effect`, not test-side ability reinstallation.
- Lines 144–162 call the native `execute_effect` path three times, assert zero marked damage, then remove abilities and require one marked damage from the same effect. This guards against treating assignment-only assertions as actual damage execution.

## Relevant production-source cross-checks

- `events/damage/matchers.rs:880–914`: `DamageToSelfMatcher` matches exact source object identity and rejects player recipients.
- `static_abilities/misc.rs:3329–3352`: native self-prevention emits `ReplacementAction::PreventDamage` with that matcher.
- `replacement_ability_processor.rs:23–141`: collection uses current characteristics/controller, skips phased-out battlefield objects, and respects functional zones/current abilities.
- `events/processing/mod.rs:6356–6380,7241–7286`: test helper calls the real assignment pipeline; it refreshes replacement effects and passes combat/unpreventable flags through the damage proposal.
- `game_state/zones_and_characteristics.rs:1290–1297`: native zone movement assigns a fresh object ID and restores owner as initial controller outside stack-to-battlefield transitions. Continuous modifications in the test target the old exact object ID, leaving the printed ability available to the returned incarnation.
- `game_state.rs:859–872` and `object.rs:463+`: shared definition handles are reused only after matching the complete definition; the same-card-ID abilityless clone does not automatically reuse the host's ability vector.

## Boundaries and remaining evidence

1. Execute `cargo test -p ironsmith-compiler-runtime --test chomanno_full_body` on the authorized target commit/environment, retain its exact commit and result, and require all three tests to pass. Source review cannot establish Rust compilation or assertion success.
2. Run the applicable authorized aggregate/regression gates before broader integration or gameplay-admission claims. No aggregate outcome is established here.
3. Preserve identity-level admission/accounting discipline: the prior measured compile recovery is not gameplay proof; this review does not independently verify the 130-ID inventory or authorize subtracting this identity from a residual count.
4. The combat matrix is assignment-layer evidence with `is_combat=true`, not an end-to-end combat-step scenario. Inactive/missing-recipient calls examine replacement behavior, not legal targeting or actual damage to nonexistent objects. The separate execution case covers real noncombat damage application to a battlefield creature.
5. Optional hardening, not a blocking finding: assert compiled mana cost, Legendary supertype, and Human/Rebel subtypes in addition to checking their input fields; assert the post-type-change current type; and include `DamagePreventedEvent.damage_source`, `target`, and `is_combat` in the event helper's returned tuple. Current checks cover only prevention amount/source/controller and damage amounts, so those additional metadata/event fields are not independently proven by these tests.

## Final-head hardening re-review

Final reviewed HEAD: `fd7106e0b75ce30ea6a57b6104b9c848edabfe44` (delta from `544bdc3bd6197d2d3d3b39c477390dbf5972bc50`). The checkout was clean. This follow-up reviewed only the hardening delta and related API definitions; no execution was performed.

**Verdict remains bounded source-clear, with no concrete blocking defect identified.** The earlier optional hardening item 5 is now addressed in source:

- Both compiled definitions must have exact `{2}{W}{W}` via `ManaCost::from_symbols`, Legendary supertype, and Human/Rebel subtypes. Their imports and constructor are available in the inspected source.
- After `SetCardTypes`, public `calculated_characteristics(host)` must expose precisely `[Artifact]` before the continued-prevention assertion. This eliminates the formerly unasserted type-change precondition.
- Every captured prevention event now checks actual damage source, recipient, combat flag, absence of a created-shield identity, and exactly one application whose source/target/amount/combat flag agree. Existing exact expected event tuples still require amount, prevention source, and controller. Thus the added assertions are not vacuous on expected prevention paths.
- `DamageTarget` is `Copy + PartialEq + Eq`; the event helper's reuse after the assignment call is source-consistent. The static-prevention event producer constructs `DamagePreventedEvent::new` with one application and only adds shield identity for the shield-specific replacement action, supporting the authored no-shield assertion.
- The report accurately labels these assertions UNRUN. The delta changes only the test and its report, with no production changes.

All remaining execution, aggregate, identity-admission, and assignment-versus-combat-step boundaries above still apply. In particular, this final-head review does not establish a passing test or justify moving the identity into verified recovery accounting.
