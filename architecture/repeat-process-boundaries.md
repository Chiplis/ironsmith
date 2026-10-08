# Repeat-process binding reconstruction (UNVALIDATED)

This source-only reconstruction starts from verified clean commit
`2f00b78b082d02179a3730b97143904712223e1b` on isolated branch
`source/restore-repeat-process`. That base includes actual main
`5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1`, the restored first-new-stack source
bodies, and staged artifact12/digest8/audit25 gate.

The retained previous payload ran from source base
`85fb650cd24bb97fcd6bff941125c96bb95c82fa` through unpublished checkpoint
`547ee2977e8ad355832d5f1a3d91aedce83e7b54`. The previous filesystem and commit
objects are unavailable. This reconstructs the retained implementation and all
authored scenarios from the task history; it does not claim the old commit or
tree identity, or byte-identical patches. Current production owners and API
anchors were inspected before targeted source edits; no whole upstream file
was replaced. The abandoned deferred captured-conditional implementation was
not restored: the final prepared-completion and concealed-guard correction is
included directly.

The frozen baseline remains `e8740178a7f7367ffa3147e7642607042079237c`, with
3,233 unresolved Oracle identities. Historical source-coverage and measured
recovery totals are not changed by this lane. No new card credit is granted
before fresh source admission. The separate one-time current-main audit is
owned by another task and is not evidence for this reconstruction.

No build, test execution, compiler/parser/engine/browser/replay probe, formatter,
code generation, ledger edit, version bump, or publication belongs to this lane.

## Exact scope and evidence

`fixtures/card-failure-campaign/repeat-process-boundaries.json` contains nine
exact Oracle-ID-selected bodies and metadata extracted again from
`cards-20261003.json.xz`, and both frozen diagnostic routes from
`baseline-e8740178.snapshot.json.gz`. All are single-faced entries. Restoration
provenance is explicitly distinguished from the former base reference.

Eight full-body source candidates, still unvalidated and awaiting admission:

| Oracle ID | Card | Binding and complete-body obligations |
| --- | --- | --- |
| 8095ca78-db19-4724-a6ff-eacc85fa2274 | Another Round | Initial complete exile/return, then X additional complete executions; new choices each pass, return to owner, X=0 still executes once |
| 1636c4d2-f699-4af7-8508-dbce2f0b7b52 | Countryside Crusher | Reveal, sample land gate, move if land, repeat; final nonland/empty reveal stops; separate graveyard counter trigger remains |
| 18f0cd0b-3e4f-4637-a62e-75dd1b2f3fce | Claim Jumper | Initial optional search, recheck relative land counts, one additional optional search, exactly one final shuffle if either search occurred; vigilance and intervening trigger condition remain |
| 97560adc-814e-450c-9ca7-f9364e910a0b | Grindstone | Latest actual mill collection has at least two objects sharing a color; multicolor overlap qualifies, colorless/short/empty sets do not; target and activation costs remain |
| 9df2e909-ed13-456a-9636-7398732009a9 | Professor Onyx | Seven complete opponent discard/fallback rounds; magecraft, look/select/rest move and greatest-power sacrifice remain separate complete abilities |
| 99eb50ef-352f-47c4-91e3-32813cbe0649 | Scalpelexis | Any distinct same-name pair within the latest actual exile collection qualifies; collections never accumulate; flying and combat-damage trigger remain |
| 50885640-cdf1-4c62-b3bc-f37db6ab38b5 | Trade Secrets | Both draws execute initially and every additional pass; the announced target opponent alone chooses continuation; controller retains its own up-to-four choice |
| 5bc66b18-22ac-4138-b527-fa711116e298 | Zimone and Dina | Initial draw and optional land, then live eight-land gate and one additional entire draw/land program; second-draw trigger and activation sacrifice remain |

Held exact partial:

- 3bcc378c-4470-4757-a0d5-025a32c918ea, Sin, Spira's Punishment. The generic live
  gate owner is available, but the frozen random graveyard permanent exile,
  exact tapped copy-token reference and exiled-card copula are not established
  end to end. Its frozen unsupported-predicate diagnostic is retained. It is
  not among the eight complete-body candidate tests or any claimed recovery.

The wider Belzenlok/Rally accumulation, Forgotten/Shrouded cross-iteration
exclusions, Timesifter ties, Struggle actors, and Thieves' Auction rotation
remain outside this cohort.

## Source ownership

- `grammar/effects/clause_primitive_shapes` recognizes finite additional counts
  and optional repetition surfaces with complete-token consumption. The new
  `RepeatThisProcessAdditional` marker is compiler-only and fail-closed if it
  reaches ordinary lowering unbound.
- `effect_ast_normalization` binds all prior process instructions, preserves
  suffixes, captures live gates before branch mutations, and wraps initial plus
  conditional second execution as one result owner for a final search receipt.
  Normalization is idempotent for already-bound loops.
- `lowering_support` retains the complete cross-sentence process even when a
  marker has a following suffix. `control_flow_handlers` attaches the receipt
  to the exact conditional gate. Targets remain outside repeated programs.
- `modal_results` decomposes numeric cardinality, pairwise relation, object
  filter and authored result action before lowering Grindstone's predicate.
  The demonstrative same-name surface becomes a typed exact-result predicate;
  it does not invent an exile action or consult ambient target memory.
- `RepeatProcessEffect` retains its transaction, fresh gate ID clearing and
  trigger capture owner. `RepeatEffectsEffect` retains finite-count selection,
  complete sequence boundaries and transaction/continuation ownership.
- The existing mill owner records actual public-zone results with exact library
  LKI, including public replacement destinations; it omits hidden/prevented
  results. The new sharing predicate consumes the current instruction receipt.

## Upcoming serialization boundary (not integrated here)

Three additive vocabulary changes require the next compatibility cohort;
this lane does not edit the inherited staged12/8/25 gate:

1. `ironsmith_core::ConditionalEffect<E>::capture_condition_result: bool`, appended
   with serde default false. Existing payloads retain branch-result semantics;
   marked loop gates export the condition sampled before branch execution.
2. `ironsmith_core::RepeatProcessPromptEffect::decider: Option<PlayerFilter>`,
   appended with serde default None. None retains the existing iterated-player /
   controller owner; explicit filters preserve an authored deciding player.
3. `EffectPredicate::AffectedObjectsShare { required_count, characteristic }`,
   appended after existing variants. It requires distinct members of one exact
   instruction's affected-object memory. Name matching reuses the existing
   split-name/nameless-aware subset matcher.

Core serde/tag walking, ordinary artifact decoding/card-graph traversal,
compiler lowering, runtime interpretation, actual native program encoding,
condition/prompt execution and compiled-text rendering retain these properties.
No version constants or historical signatures are changed. Captured conditions
use the normal `ConditionalProposal` branch owner and the shared
`adapt_original_outcome_with_outputs` completion adapter. The Boolean is sampled
at selection and projected only onto the completed packet; child originals,
added programs, observe/freeze phases, draw boundaries and retained participant
outputs keep their existing owners. Pending outcomes are neutral and errors
propagate inside the existing transaction boundary.

The opt-in capture contract has explicit unsupported combinations:

- A captured nonempty branch requested through simultaneous preparation must
  obtain a real prepared original owner. If it cannot, preparation fails with
  `IncompleteEvidence` before mutation; it does not execute the whole branch
  during an original commit. Ordinary sequential execution remains separate.
- Captured conditional programs cannot use the action-cursor, shared-damage or
  replacement-draw shortcut that would discard their completion projection.
- A captured condition with a concealed optional positive-identity guard fails
  with `IncompleteEvidence` before its branch executes. Its pre-reveal Boolean
  is not evidence of a completed positive claim; neither a successful claim
  nor an unproven placeholder is silently exported as false. A future owner
  would need to capture the completed guarded proof at its own boundary.

Only Countryside Crusher among these eight bodies uses this capture flag. Its
single-controller revealed-card move has a prepared branch owner and no guarded
optional identity claim. These explicit wider-scope rejections add no card
coverage. The prompt's absent decider still preserves its prior implicit
chooser, and the shared-object predicate remains an exact result collection
rather than an ambient or accumulated query.

The absent-field cases authored here are model-default checks, not claims that
historical artifact bytes were recovered or validated. The inherited
compatibility gate does not cover this later cohort.

## Authored validation (all UNRUN)

All retained scenario payloads are restored:

- Grammar: X/six/zero additional counts, optional actor wording, trailing-junk
  rejection, complete counted-sharing predicates and same-name subset syntax.
- Normalization: X=0 initial execution, suffix exclusion, idempotence, and a
  conditional second execution exporting the whole process receipt.
- Core models: explicit fields and subset predicate round-trip; absent fields
  preserve their defined historical defaults.
- Runtime primitive: condition is sampled before its branch changes life;
  distinct pair/short/zero/duplicate identity/nameless result collections.
- Complete frozen bodies through direct compiler, artifact round-trip and
  freshly native-encoded round-trip routes, with secondary abilities retained.
- Per-card scenarios: Another Round's fresh incarnations/owner control and X=0;
  Claim Jumper's first-only, second-only, both, neither and final shuffle;
  Countryside's land chain and nonland/empty termination; Grindstone's common
  color, multicolor/colorless/short/empty sets and public exile replacement;
  Onyx's seven opponent rounds; Scalpelexis's pair among four and a following
  singleton with a previous batch's name; Trade Secrets's chooser, zero
  additional rounds and pause rollback; Zimone's 6/7/8-land boundaries.
- Native prepared receipt correction: an added program must observe both
  participant originals; one sampled Boolean per participant survives additions
  changing the condition; unsupported prepared branches and concealed captured
  guards reject, preceding program work/receipts roll back, and paused proposals
  remain neutral.

These source scenarios are not evidence of passing compilation or behavior.
Later validation still needs all focused executable cases, frozen whole-corpus
comparison, semantic/rendering regression review and the compatibility gate.

## Previous and fresh review scope

The previous payload's initial independent review covered the eight bodies,
grammar/lowering, model/native transport, source-level type/API checks and
focused scenario assertions. It did not audit the simultaneous captured-
conditional lifecycle; its initial clearance did not cover the coordinator's
later finding. The subsequent bounded three-file review covered the completed
prepared-owner correction and authored rejection/rollback/pending scenarios.
Neither previous review establishes admission of this reconstruction. A fresh
current-owner source review is required before the coordinator admits it.

## Fresh current-main source corrections

Restoration review identified an additional current-main integration owner:
`continuous/text_change_programs.rs` exhaustively matches result predicates.
Its new shared-object predicate arm is explicitly wordless, while repeated
programs recurse through their typed predicate/children and prompt deciders
recurse through their player filters. Full model clones preserve the captured
condition flag, result ID and every unaffected field. A new UNRUN source test
checks those identities, immutable originals and black-to-blue nested filters.
This path is a targeted current-owner correction beyond the retained 34 paths,
not evidence that its bytes existed in the lost tree.

The Grindstone scenario now asserts numeric continuation count only when an
additional pass actually occurred. Initial-only cases retain exact destination
and library-size assertions without imposing a separate aggregate result API.

Fresh review also found that the retained repeat-once `EffectAst::Sequence`
wrapper was a flattening compiler container. A later searched-library result ID
could therefore name only the second conditional rather than the complete
process. The scoped correction uses existing `EffectAst::CommaThen`, whose
lowering preserves one ordered runtime `SequenceEffect` and its aggregate
receipt. Generic Sequence lowering is unchanged. The normalization assertion
now requires this real owner, and an additional UNRUN full-Claim materialization
case inspects the exact `SearchedLibrary` producer ID across all three routes.
First-only, second-only, both and neither-search runtime cases remain retained.
This is a fresh source correction; it was not validated in the lost checkpoint.

## Independent restored-owner review, 2026-10-07 (UNVALIDATED)

The fresh review starts at restored input
`8d58b7e8c423f418ce73d4aa2886e8833316fe0a`, tree
`c71bd8a4ac243fb62f49789e2a39b60836ee8742`. It reads the current typed grammar,
normalization and lowering owners; exact continuation result IDs; conditional
selection/original/completion paths; result-memory predicates; native encoding
and interpretation; text substitution; and compiled presentation. No historic
lost-tree byte identity or executable clearance is inferred.

One additional production presentation issue was found. The restored finite
processes store `1 + X` or `1 + six` total executions, but the existing generic
renderer appended the aggregate number of times to the last body clause. That
surface did not keep an explicit complete-process boundary. The scoped
`RepeatEffectsEffect` renderer now recognizes the existing nonnegative
initial-plus-additional count shape and renders the complete initial body,
followed by `Repeat this process N more times`. It introduces no new model,
serialized field, variant, or grammar fallback. Source-authored rendering cases
cover zero, six, X, and a two-instruction body. The full-body cases also require
the corresponding Another Round and Professor Onyx compiled-text boundaries.

The previous full-body test accepted minimum ability counts plus debug marker
substrings. The replacement selects the eight independent Oracle IDs exactly,
requires unique fixture membership and explicit exclusion of held Sin, checks
fixed printed costs/types/stats/loyalty, exact static/triggered/activated counts,
and typed finite/count-sharing/captured-gate/decider owners through the direct,
artifact and freshly native-encoded routes. Runtime expectations are authored
independently from the frozen bodies rather than copied from a compiler route:

- Printed spell payment, including Another Round's two X symbols; Trade
  Secrets announces its opponent once and keeps each numeric draw decision
  with the controller.
- Grindstone pays three and taps before resolution; its two-card shared-color
  continuation retains the actual typed mill receipt.
- Claim Jumper retains vigilance, real entry triggering and the intervening-if
  resolution recheck. Two accepted searches that find no Plains still produce
  two search events and exactly one final shuffle.
- Countryside Crusher uses its actual upkeep trigger, then the complete
  from-anywhere land-graveyard counter trigger, including hand/battlefield and
  opponent-graveyard controls.
- Scalpelexis retains flying and requires this source's combat damage, binding
  the actual damaged player rather than an ambient player or noncombat event.
- Professor Onyx pays each loyalty price, keeps look/select/rest destinations,
  greatest-power sacrifice per opponent, seven mixed discard/fallback rounds,
  and both ordinary cast and actual copied-spell magecraft events.
- Zimone and Dina pays tap plus another creature before either draw, and its
  separate second-draw trigger announces a target exactly once; a third draw
  adds no further trigger.

The old Trade Secrets pending-choice case discarded its execution context
before asserting rollback, so its name overstated its receipt coverage. It now
keeps that context and compares the full prior receipt map, reference-tag map,
exact library identities/order, and empty completed-event output. A new
Countryside missing-public-opening case requires `IncompleteEvidence`, rollback
of earlier life gain and exact context memory, unchanged hidden identity and
crypto-audit position. These are authored local runtime expectations, not an
authenticated replay run or a substitute for exact savepoint validation.

All added cases remain **UNRUN**. No build, test, compiler/parser/engine/browser
probe, formatter, corpus execution, code/artifact generation, remote write,
source-ledger change, or compatibility version/fingerprint change occurred.
The packet still has exactly **eight full-body source candidates and one held
partial**, with no source admission and no measured recovery credit. Its three
previously documented additive wire changes still require the next compatibility
cohort. The presentation fix additionally changes affected regenerated text and
checksums. Exact local memory savepoints, persisted same-instance anchors,
authenticated accepted-prefix/suffix replay and genesis fallback keep their
current owners; redacted public audit remains non-importable gameplay evidence.
