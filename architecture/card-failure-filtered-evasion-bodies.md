# Filtered evasion: complete frozen bodies

Source-only proposal based on reviewed central commit
`b8b0a5e18c52d3a277e60170d009c83acde37f9f`. No build, compilation, compiler
probe, test, formatter, engine scenario, or corpus execution was run. The
authored scenarios below are unvalidated; this document does not claim measured
compile recovery and does not promote the central coverage ledger.

## Frozen scope

`fixtures/filtered_evasion_bodies.json.fixture` contains the complete printed
bodies and identifying metadata copied from
`fixtures/card-failure-campaign/cards-20261003.json.xz`:

- Cavern Stomper: entry scry 2 and the `{3}{G}` activation prohibiting blockers
  with power 2 or less for the current turn.
- Harvesttide Sentry: your beginning-of-combat trigger, intervening Coven
  condition requiring at least three different current creature powers, and
  the same power-filtered evasion through the end of that turn.
- Sungold Sentinel: entry-or-attack optional graveyard exile; `{1}{W}` activation
  gated by Coven; choosing a color; matching-color hexproof and matching-color
  blocker prohibition with their end-of-turn duration.
- Verdant Outrider: `{1}{G}` activation and the power-2-or-less blocker filter
  lasting this turn.
- White Tiger, Ava Ayala: one card, including typed Power-up identity, the
  once-per-object activation limit, source-mana-cost reduction for the turn of
  entry, a +1/+1 counter on the source, and the legendary green 4/4 Cat God token
  named The Tiger God with a maximum of one blocker.

## Source corrections and ownership

The semantic-output baseline omitted the `can't be blocked` marker, but current
source inspection also found behavioral gaps.

1. The shared restriction renderer now describes a source-only
   `BlockSpecificAttacker` as the attacking creature's evasion. The blocker
   filter and authored source surface remain typed; rendering does not select
   semantics or change a filter/duration.
2. Resolved `BlockSpecificAttacker` effects bind chosen blocker qualities at
   resolution and lock the attacker identity. The ordinary blocker predicate
   stays live, so creatures entering later and changed blocker power/colors
   are evaluated when blocks are declared. A later activation choosing another
   color cannot rewrite an earlier restriction. The existing shared protection
   binder now freezes absent color choices to a no-match filter instead of
   retaining a relative lookup. A fixed-color filter and a different chosen
   color remain conjunctive, admitting matching multicolor objects and
   preserving already required colors. Gone source identities cannot
   transfer the resolved evasion to another incarnation.
3. The token embedded-rule grammar recognizes a complete self-referenced
   maximum-blocker rule. It verifies the subject against the token's identity,
   and reuses the ordinary complete blocking grammar for the exact quantity.
   A typed `MaximumBlockers` token AST lowers to the existing native
   `cant_be_blocked_by_more_than` payload. The token's name remains in its
   definition; its static ability uses canonical bounded-evasion rendering.
   Alternate names/counts are supported; unrelated subjects and incomplete or
   extra clauses are not accepted as this rule.

The remaining complete-body operations use existing owners: ordinary scry;
entry/attack triggers and optional graveyard targets; current-power distinct-set
conditions in the condition evaluator; typed conditional activation legality;
chosen-color hexproof grants; typed Power-up costs and per-object activation
history; counters; token characteristic lowering and native blocker declaration.
No companion instruction is intentionally held or omitted in this proposal.

## Serialization boundary

The new token rule is compiler AST only. It lowers to already supported static
ability payloads. Chosen-color binding materializes existing object-filter
fields in runtime restrictions and protection. No serialized field/default or
format gate is changed: artifact 6, checkpoint 3, and audit 19 remain the
established boundaries. No label, card name, diagnostic string, or Debug output
drives native behavior.

## Authored, unrun evidence

`crates/ironsmith-compiler-runtime/tests/filtered_evasion_bodies.rs` independently
invokes direct runtime compilation and artifact compilation for every full
fixture, then serializes/deserializes, validates, and materializes the artifact.
Each path checks loss reports and absence of unimplemented content. These are
future test operations, not operations performed during source repair.

The native scenarios cover real activated payment and insufficient payment,
source-only filtered evasion before/after payment, threshold boundaries,
counter-modified and later-entering blockers, cleanup expiry, actual entry scry
partitioning, Coven's distinct current powers, opponent-turn nontriggers,
phasing before intervening-if resolution, and persistence after Coven ceases.
Sungold scenarios cover separate color choices, both retained hexproof/evasion
sets, own-source targeting exemptions, optional entry and attack graveyard
exile, and departure/return before the source's activation resolves.

White Tiger scenarios cover the reduced and unreduced actual payment, canceled
payment with restored resources and unused activation allowance, exact token
characteristics, one legal blocker, two illegal blockers with declaration
rollback, persistence of the once-per-object limit across turns, and a fresh
allowance after a new incarnation. Token-rule grammar and restriction/renderer
unit scenarios add alternate token names/counts, incomplete-clause declines,
late blocker characteristic changes, absent-choice freezing, and fixed/chosen
color conjunctions with multicolor positives, monocolor negatives, existing
required colors, and later-choice stability. All remain unrun pending the
campaign's execution gate.
