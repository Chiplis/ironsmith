# Empower Jace, final CR 701.71

Status: **UNVALIDATED** source implementation and authored regressions. No build,
compilation, or test execution was performed. All coverage numbers below are
proposed and must be measured by the deferred full-card replay.

## Authoritative rule

The final rules are in the September 18, 2026 release notes:
https://magic.wizards.com/en/news/feature/reality-fracture-release-notes
and the CR 701.71 update:
https://www.magic.wizards.com/en/news/announcements/reality-fracture-update-bulletin

The qualifying permanent must be a controlled **token**, a planeswalker, and a
Jace. Only when no such permanent exists does the action create the predefined
blue, nonlegendary token named Jace Token (CR 111.4) with zero printed loyalty and the -1 Surveil 1 and
-3 Draw a card loyalty abilities. The player then chooses one eligible token and
puts the instructed number of loyalty counters on it. Existing tokens from copy
effects qualify; nontokens and opponent-controlled tokens do not. Existing tokens
cannot be bypassed by choosing to create a new one. An all-targets-illegal spell
never executes this instruction.

## Frozen candidates and exclusions

`fixtures/empower_jace.json.fixture` retains full frozen source inputs and Oracle
IDs for all **35** unsupported records containing Empower in `stack07-bc9e56e2`.
Thirty-one are complete-card candidates with the shared missing action as their
identified blocker. Four remain explicitly partial and are not counted as full
coverage:

- Jace's Machinations: independent temporary loyalty timing permission wording.
- Theorist's Sanctum: optional behold entry replacement is the frozen first error.
- Violent Echoes: independent excess-damage predicate/where-X binding is the first error.
- Way of the Mind Sculptor: independent predicate inspecting loyalty-counter
  payment on the activating ability.

Complex later clauses on the thirty-one candidates still need the deferred replay;
no successful whole-card compilation is claimed at this stage.

## Typed implementation and source review

- Adds `EmpowerJace { amount: Value }` to the compiler keyword action algebra and
  `EmpowerJaceEffect { amount: Value }` to the shared runtime payload model.
- The keyword grammar owns the complete `empower Jace N` instruction and its
  where-X binding. Amount/reference visitors, modal handling, effect ownership,
  lowering, chosen-object antecedents, and compiled-text rendering are updated.
- Preprocessing protects the Jace subtype immediately following `empower` from
  being rewritten as a source name, including on Jace, Reality Sculptor.
- The runtime effect uses ordinary `CreateTokenEffect` and `PutCountersEffect`
  pipelines, so token-entry and counter replacement effects remain authoritative.
  No state-based-action procedure is inserted between entry and counter placement.
- Candidate selection uses live token kind, current control, current permanent
  types and subtypes, and excludes phased-out objects.
- Token names are preserved by the creation pipeline; the definition explicitly
  uses the CR 111.4 default name Jace Token. Selection does not inspect names, and
  the copy-token regression retains a different copied name.
- Printed token loyalty is zero. Its two actual activated abilities remove one
  or three loyalty counters, use loyalty timing/once-per-turn tracking, and run
  the real surveil/draw effect programs.
- A new `EmpowerJace` keyword-action enum value is appended, preserving the ordinal
  positions of existing values. The effect emits one actor/source/amount event.
- Entire action state and execution context are restored on errors or a pending
  choice; pending creation does not leak tokens into a retry.
- The decoder family table, typed permanent decoder, card-ID mapping path,
  compiler-to-runtime interpreter, runtime direct decoder/serializer type macro,
  and public exports include the new payload. Existing payload shapes are unchanged.

## Authored deferred regressions

Public compiler/runtime tests cover thirty-one complete-card candidates with typed
JSON artifact transport; the four known independent blockers are retained in the
fixture without being counted as complete. Focused real cast/stack scenarios cover
new token characteristics, loyalty payment and abilities, repeated empower using
the existing token, opponent/nontoken exclusion, a real token-copy candidate,
choice among multiple tokens, token/counter doublers, zero-loyalty survival during
the action and normal subsequent state-based actions, illegal-target fizzle, and
where-X from prior exiled creatures. An interrupted-choice test checks rollback
and replay without duplicate creation. Grammar/preprocess tests cover X binding,
source-name safety, and rejecting other names or a missing amount.

Deferred commands:

`cargo test -p ironsmith-compiler-grammar --lib empower_jace`

`cargo test -p ironsmith-compiler-runtime --test empower_jace`

## Sanctum Lurker tail

A typed, controller-scoped static rule suppresses only the zero-loyalty SBA,
without granting an ability to protected planeswalkers. The SBA cache tracks
planeswalkers and rule sources separately, revisiting protected objects when a
source leaves, phases, changes control, or loses its ability. Other death rules
remain in place. Authored full-card tests cover actual +2 loyalty payment at zero,
damage/life-gain resolution, once-per-turn limits, ownership/control distinction,
phasing, and removal of the last rule source. UNVALIDATED.
