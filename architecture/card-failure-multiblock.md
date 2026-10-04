# Multi-block permissions: source implementation ledger

**UNVALIDATED.** Builds, compilation, and test execution are deferred under the
current campaign workflow. This ledger records frozen candidates and proposed
semantic coverage, not measured recoveries.

The `bc9e56e2` stack-07 snapshot contains 18 unsupported cards with “block an
additional” (8) or “block any number” (10). Exact frozen metadata, Oracle text,
Oracle IDs, source links, and diagnostic strings are preserved in
`fixtures/multiblock_permissions.json.fixture`.

## First bounded subset: seven proposed full-card repairs

- Avatar of Hope
- Entangler
- Ironfist Crusher
- Palace Guard
- Thoughtweft Trio
- Valor Made Real
- Wall of Glare

The engine previously only summed fixed additional-blocker allowances.
`CanBlockAnyNumber` is an explicit typed static permission, not a giant numeric
allowance encoded in a card. The actual blocker-capacity query consumes it;
this query is shared by declaration validation and the maximum-satisfiable
must-block requirement search. Duplicate pairs, evasion, tapped state,
defending-player scope, and global limits remain independently checked.

A strict shared clause grammar feeds source/static, attached/static, filtered
static, and temporary targeted grants through the existing AST and materializer.
Temporary permission retains `Until::EndOfTurn`. Complete-predicate rendering
preserves “can block” instead of “has/gains can block.”

Authored, unrun regressions:

- Grammar: exact complete clause, explicit duration, compound/tail rejection.
- Runtime target `multiblock_permissions`: full metadata/artifact round-trip;
  eight actual assignments, ordinary/finite limits, duplicate/tapped/flying
  restrictions; must-block solver and distinct-blocker cap; Entangler attachment
  movement and departure independent of Aura controller; actual Valor cast,
  printed cost, chosen recipient and cleanup; filtered grants under controller
  changes/source departure; damage from three blocked attackers to Wall of Glare.
- Tools target `multiblock_permissions`: all seven exact metadata-bearing payloads
  must compile strictly without an Oracle-only fallback.

## Remaining exact candidates

- Act of Heroism: temporary shared-subject pump plus additional-block tail.
- Blaze of Glory: temporary unlimited permission plus a mandatory-block rule.
- Echo Circlet: attached fixed additional-block permission.
- Entourage of Trest: source permission conditional on monarch status.
- Foriysian Totem: conditional source permission while it is a creature.
- Give No Ground: temporary shared-subject pump plus unlimited-block tail.
- Guardian of the Gateless: unlimited source permission and an independent
  trigger counting the attackers it blocks.
- Hundred-Handed One: conditional reach plus 99 additional-block permission.
- Iona's Blessing: attached pump, vigilance, and additional-block permission.
- Kemba's Legion: dynamic additional count based on attached Equipment.
- Vanguard's Shield: attached pump plus additional-block permission.

These 11 are not counted as covered by the first subset. Further source changes
and regressions must preserve their complete surrounding instructions; stripping
a conditional, suppressing a diagnostic, or recognizing only a marker is not
coverage. Full-card and gameplay outcomes await the deferred validation phase.

## Second bounded subset: eight additional proposed full-card repairs

The shared leaf now distinguishes `AnyNumber` from `Additional(u32)` and accepts
both authored cardinal placements (“an additional ninety-nine creatures” and
“two additional creatures”). It consumes the complete duration/tail and rejects
unknown counts or qualifications. Static recognition carries leading/trailing
conditions and conjoined pump/keyword predecessors without dropping any sibling.
The ordinary typed predicate and attachment-grant paths retain their scopes.
Temporary effect recognition admits bare carried `can block ...` clauses and
carries the exact pump target into the independent permission effect.

Proposed additions: Act of Heroism, Echo Circlet, Entourage of Trest,
Foriysian Totem, Give No Ground, Hundred-Handed One, Iona's Blessing,
and Vanguard's Shield. The fixture's group records this source-level proposal;
there is no measured post-change compilation result.

Additional unrun scenarios cover additive attached allowances with P/T and
vigilance retained; live monarch changes; actual Totem animation and expiry;
actual Monstrosity payment with 100 successful versus 101 rejected assignments;
and actual Act of Heroism/Give No Ground casts preserving untap, pumps, capacity,
recipient scope, and cleanup. Full-card/artifact assertions cover all eight.

Only Blaze of Glory (mandatory blocks for every attacker), Guardian of the
Gateless (counting attackers blocked by source), and Kemba's Legion (dynamic
attached-Equipment capacity) remain pending within the frozen 18-card family.
These are independent missing semantic shapes, not reasons to flatten their
surrounding instructions.

## Current blocked-attacker count: one further proposed repair

Guardian of the Gateless uses a typed `Value::Count` of attacking creatures
currently in combat with its bound blocker antecedent. The nested `it` reference
uses the existing source/selected-object reference resolver. This does not use a
whole-turn history count or the source's own blocker count.

Both current-combat relation matchers previously consulted only the first
attacker blocked by a creature. They now inspect the complete attacker-to-blocker
edge for the candidate. The source-LKI fallback is unchanged in this commit;
a parallel combat-event family is independently tightening that history scope.

The new authored scenario uses the actual blocker declaration/event/stack path,
asserts one trigger for three blocks, removes an attacker before resolution,
excludes another creature's block, retains the resolved pump after another
attacker leaves, and checks cleanup. It runs both source-name variants and both
direct/artifact definitions when deferred validation is enabled.

Proposed coverage is now **16/18**, still UNVALIDATED. Blaze of Glory and Kemba's
Legion remain pending.

## Dynamic attachment capacity and complete forced blocks

Kemba's Legion now has an explicit typed `CanBlockAdditionalForEach` payload
containing an additional-per-match number and a complete permanent filter. The
blocking validator and requirement solver query it against the actual blocker
and current battlefield. The ordinary fixed-capacity API remains available;
its new contextual sibling delegates to it by default. Equipment movement,
departure, controller scope and ability removal therefore cannot leave a cached
printed allowance behind. The new ID and payload are appended to preserve all
preexisting serialized variant ordinals.

The capacity grammar retains the entire `for each` filter. Dynamic temporary
forms have not been added; their reader rejects rather than flattening that
count to a fixed allowance. The Kemba scenario checks opponent-controlled
attached Equipment, unattached/other-host Equipment, Aura exclusion, live
movement/departure, and ability loss/restoration through an actual spell.

Blaze of Glory's newly supported unlimited-capacity first instruction composes
with the existing typed `MustBlockSpecificAttacker` restriction: its blocker is
the same target antecedent and its attacker filter is each attacking creature.
The authored full-card scenario verifies the real cast, all legal mandatory
assignments, inability to block flying/tapped, both expirations, and the printed
before-blockers casting restriction. No unconditional “must block something”
substitution or new name-based special case is used.

All **18 frozen candidates now have proposed source coverage and authored
full-card/artifact regressions**. This is still UNVALIDATED: no compilation,
test execution, or replay result has established the expected recoveries. The
frozen diagnostic strings remain in the fixture for the eventual comparison.
