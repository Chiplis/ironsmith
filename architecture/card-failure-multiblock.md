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
