# Conditional damage amount ownership (UNVALIDATED)

Base: `44e501aad0c0834c6f09e36adcc1c7c5e917f532` (PR866).

This is a source-only checkpoint. No compiler, parser, engine, build, test,
formatter, code generator, browser probe, corpus run, or publication was executed.
The one-time main audit is the sole measurement. No newly recovered-card credit
is claimed.

## Measured family and ownership

The one-time `original-identity-comparison.json` marks all eleven inspected
identities regressed. Their exact current errors in `current-failures.json` are
missing-target errors in the leading-if/anaphoric-damage readers (or the
separate-line conditional family for Adamant/Addendum).

The standalone conditional reader requires a complete damage instruction,
including a recipient. An amount-only replacement instead depends on the prior
damage instruction. The existing post-parse self-replacement binder never gets
that instruction because the independent conditional parse has already failed.

The new complete followup shape proves a leading or trailing conditional with
an omitted damage recipient. Its pre-parse owner requires one preceding typed
damage action, retains that complete action as the default, and clones it while
changing only the amount for the replacement. Existing `SelfReplacement`
lowering remains responsible for declaration/assignment ownership, conditions,
and selecting one branch. Explicit destinations, other source references,
additional bodies, unsupported event amounts, and incomplete tails are not
claimed. No generic reader priority or loss handling was weakened.

## Bounded proposed positive coverage

- Burst Lightning: `ac2086fe-98ee-4280-9c7c-c5c2d6548a8b`
- Roil Eruption: `6798163a-864f-4844-96b1-77585a7e7ab4`
- Shivan Fire: `854fd120-9a51-4d37-9922-7e4b0464e0f5`
- Frost Bite: `128170ed-c86a-4f02-9244-28197be90c10`
- Burning Hands: `51a0d1c2-ff29-4ac9-84a2-ad4f566fee8d`
- Voltage Surge: `1779af8e-38fc-4043-b5fa-ee16a7ea840c`
- Akoum Hellkite: `05b6f9e3-9acd-43a6-acff-caa811aaf0a1`

All full printed bodies and metadata are frozen in
`fixtures/conditional_damage_amounts.json.fixture`, copied from the pinned
`cards-20261003.json.xz` source. These seven are hypotheses for the pending gate,
not measured recoveries.

## Held identities

- Flame Discharge (`fa4755c2-e573-45c3-bd6a-61b2b35fbd24`): the existing
  `parse_you_controlled_as_cast_predicate` produces `PlayerControls` with an
  `as_you_cast_this_turn_surface` flag. The current runtime `PlayerControls`
  evaluator scans the current battlefield. That flag is rendering provenance,
  not cast-time evidence. The new binder explicitly rejects this condition
  instead of admitting incorrect timing. A real cast-time predicate owner is
  required before this card can join the positive set.
- Surtland Flinger (`f701ada1-e9e1-42ce-9a62-c11bcec03da7`): the replacement
  occurs inside a reflexive optional-sacrifice program and uses the exact
  sacrificed object's last-known power/subtype. This change does not bind that
  event or its `twice that much` amount.
- Slaying Fire (`1e94e647-9150-4b22-aa9f-d195f64fb20a`) and Summary Judgment
  (`ae198ce9-097b-4204-b599-12bdb36f5195`): separate Oracle lines require a
  document-level prior-instruction owner before the conditional body can parse.
  This patch does not synthesize a placeholder recipient across lines.

## Authored gate coverage, all UNRUN

`crates/ironsmith-compiler-runtime/tests/conditional_damage_amounts.rs` includes
independent strict `compile_to_runtime_definition` and `compile_to_artifact`
calls, parse-loss assertions, artifact validation/JSON roundtrip/materialization,
native definition encoding/JSON/materialization, and full-card rendering/body
retention. Seven independent source-authored rules snapshots assert every
printed instruction, amount, recipient, condition and optional-cost line rather
than comparing only compiler routes to one another. Metadata assertions check
printed mana, types, supertypes, subtypes and P/T, with exact kicker mana and the
optional artifact-sacrifice cost. Runtime scenarios exercise:

- actual casting and optional kicker/sacrifice costs;
- exactly one original target assignment and no replacement target prompt;
- one completed damage receipt with the original source and recipient;
- current snow population and target color changing after casting;
- the triggering land's current characteristics and exact departure LKI;
- illegal original recipients, partial/all-illegal multiple targets, and X;
- held full-card routes and cast-time-control rejection.

Grammar cases cover complete source/target/amount binding, ordinary negatives,
explicit alternate destinations, extra instructions, orphan replacement bodies,
unsupported event amounts, and duplicate replacement markers.

## Coordination requirements

No serialized carrier was added or changed. No damage transaction, receipt,
cause, condition evaluator, or target-legality runtime owner was modified.
No version or ledger was changed. The pending coordinated 12/8/25 gate must
include these grammar and full-card runtime cases because compilation semantics
changed. Independent source review and the authorized later execution gate are
still required; this checkpoint does not establish that tests compile or pass.
