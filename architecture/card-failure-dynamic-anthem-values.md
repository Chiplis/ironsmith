# Dynamic anthem value bindings

**UNVALIDATED.** This implementation-first checkpoint has authored regressions,
source review and formatting checks only. No build, compile, test, or corpus
replay result is claimed.

## Frozen membership and proposed scope

`fixtures/dynamic_anthem_values.json.fixture` preserves all 44 exact stack-07
where-X candidates, including complete metadata, original Oracle text, source
identities/links and diagnostics. Nine are proposed full-card repairs:

- Death's Shadow and The Last Ride: negative controller life total
- Kagemaro's Clutch and Meishin, the Mind Cage: negative controller hand size
- Carrion Grub and Coram, the Undertaker: greatest creature-card power in the
  controller's graveyard or all graveyards, respectively
- Greven, Predator Captain: controller's life lost this turn
- Kinbinding: creatures entering under the controller's control this turn
- Knowledge Is Power: controller's cards drawn this turn

The other 35 remain pending. In particular, this change does not claim Hedron
Matrix's affected-object reference, Hancock's source counter binding, Bludgeon
Brawl's generated Equipment program, or resolution-local where-X expressions.

## Root cause and typed repair

The shared where-X reader already returns typed `Value` nodes for these inputs.
The static anthem reader then insisted on converting every result to the smaller
`AnthemCountExpression` enum and rejected valid life, hand, aggregate-power and
turn-history values. `AnthemValue::Dynamic(Value)` already exists for this job.

Existing count-representable values retain their original path and renderer.
The fallback admits tag-free game-state scalar/aggregate/history expressions
and their arithmetic compositions through Dynamic. Component signs are applied
once through the existing scaler. Resolution-local event/result values and
unresolved object tags are still rejected instead of becoming zero or acquiring
an invented source/recipient binding.

The native anthem renderer now separates a leading negative multiplier from
its basis: negative X is rendered as `-X` with the positive binding, rather than
rendering `+X` whose binding is a second textual negation. Existing for-each and
count-specific renderer branches are unchanged.

## Continuous execution and cache dependencies

Dynamic anthem values previously materialized as fixed numbers during static
ability discovery. They now remain typed `ModifyPowerToughnessValue` expressions
and evaluate in layer 7c. This keeps supported game-state bindings live rather
than freezing a discovered number.

The turn-context classifier now reads dynamic P/T modifiers and typed
`TurnHistoryCount`. History publication also invalidates an already-clean
characteristic cache when its numeric modifiers can read turn context. Physical
mutations occur before event publication; a reveal callback can warm a cached
characteristic in between. Publishing the draw/life/zone history must therefore
invalidate those warmed values, even if no second physical mutation occurs.
The invalidator skips already-dirty state and does not blanket-invalidate all
continuous effects on every event.

These game-state bindings retain the ability source's controller. Attachment
recipients do not substitute their controller for “your hand.” Source/affected
object operands require a separate, explicit binding repair before admission.

## Authored regression evidence

Normal compiler-runtime target `dynamic_anthem_values` covers:

- All nine full metadata inputs, typed Dynamic nodes, serialized artifact
  equality/materialization, preserved signs and printed characteristics
- Public life loss/gain and source controller changes
- Actual draw/discard actions, Aura attachment/controller independence, source
  departure and global negative-power versus P/T effects
- Public zone moves into/out of graveyards, own/all-graveyard scope, empty sets
- Life lost rather than net life change, draw history rather than hand size,
  repeated creature-entry events rather than current population, and next-turn
  resets, with source controller changes
- An actual first-draw reveal callback warms a recipient's power before the
  draw event is published; the subsequent result must reflect the new history

Normal tools target `dynamic_anthem_values` requires all nine full payloads to
compile strictly without falling back to metadata-free Oracle text. Grammar
regressions assert typed bases, single negative scaling, and rejection of
unresolved references/event amounts. These assertions are intentionally unrun
until the user-authorized deferred validation phase.

## Review correction: preserve legacy Dynamic evaluation paths

The initial blanket Dynamic-to-layer conversion was too broad. Source review
found preexisting Dynamic construction in `parse_dynamic_xy_anthem_values`
(arbitrary typed X/Y operands), for-each party/color/counter paths, and direct
runtime constructors. The layer object-number adapter accepts only a subset of
ChooseSpec and its source anchor can be the affected object. Those constructions
are not proven equivalent to their previous discovery-time evaluation.

Compiler admission and runtime conversion now share the core
`supports_controller_state_anthem_value` capability check. Only the bounded
controller scalar, supported history, and source-independent aggregate bindings
move to the layer path. A positive projection of aggregate filters excludes all
unlisted semantics, especially source/target relations, `other`, chosen values
and nested filters. Presentation surfaces are not semantic restrictions.
Every other preexisting Dynamic retains its original conversion result (`None`)
and thus its original evaluation path. New unrun core/runtime regressions assert
that source power, source mana value/counters, tagged operands, event amounts,
and party size are not silently rerouted. The nine-card proposal is unchanged.
