# Chosen-type domain regression repair (source-only)

Baseline: `1dd81cd84c62f272479f26e16d74719fff24b97b`.

Frozen raw bodies came from the completed 2026-10-08 refresh's
`current-unresolved-entries.json`, not an old candidate list:

- Arcane Adaptation: `2188c03f-11b6-4651-914a-b4fdd59127e3`
- Leyline of Transformation: `cfaa13d6-9992-4223-8bd7-c44046505555`
- Rukarumel, Biologist: `84391c32-5bb7-4c36-be50-bbb5f8732156`

## Root cause and change

The complete copular-characteristic subject reader replaced the former broad
filter reader in the merged source proposals. Its complete-simple nominal
recognition rejects the owned-card nonbattlefield suffix and the repeated,
independently controlled nominal union. The same-is-true preprocessing still
expands the three subjects, but the stricter reader cannot lower all of them.
The defect is present at current main; it is not attributed to the subsequent
ongoing-effects refactor, whose runtime effect-composition changes are untouched.

The repair reads independently scoped nominals into separate complete filters
and constructs their union directly, with only the authored connective on the
outer filter. It does not reparse that union through the broad filter reader:
independent review identified that broad reader's leading nontoken/tapped
propagation as unsafe for reversed arm order. Only the fully validated single
nonbattlefield-card nominal uses the existing richer seven-zone expansion.
Unknown words, duration tails, quotes, dangling conjunctions, and conflicting
explicit zones remain rejected. Existing simple filters keep their reader.
Domain unions are no longer given an outer battlefield restriction. Chosen land-type inference examines union arms
while an explicitly authored creature-type descriptor still wins.

## Executable scope being asserted

- Controlled battlefield creatures, or Rukarumel's union of controlled Slivers
  (including tokens) and controlled nontoken creatures.
- Controlled creature spells on the stack.
- Owned creature cards in the existing seven in-game nonbattlefield domains,
  including the stack; no outside-game arm. Ownership and control are distinct.
- Existing subtypes remain; source choice is read dynamically; grants function
  only while their source is on the battlefield and disappear when it leaves.

The existing AddChosenCreatureType runtime payload, native implementation,
filter domain implementation, and artifact admission checks are reused unchanged.
No artifact format/schema change, catalog regeneration, migration, detector
weakening, or unsupported gameplay claim is included. Old cached failures need
source regeneration after validation; relabeling old artifacts is not a repair.

## Authored verification, not executed

New grammar tests cover strict extended subjects, union qualifier retention,
negative admission, chosen land/creature family distinction, and independently
specified branch-local nontoken/tapped/other/control/ownership fields in either
arm order. New independent
compiler-runtime tests cover all three exact full bodies, direct and artifact
materialization routes, current artifact round trips, obsolete-format/schema
rejection, canonical text reparse, entry choice, native ownership/control/zone
and token witnesses, live control changes, and source departure. Follow-up
witnesses cover both nominal orders (token Slivers included, token Bears
excluded), untapped Slivers alongside tapped-only creature arms, and controlled
battlefield versus owned nonbattlefield cards. Canonical text reparse compares
executable chosen-type filters, not only ability counts.

All tests, builds, compiler probes, corpus sweeps, and codegen remain UNRUN by
instruction. Only source inspection and `git diff --check` were performed.
Leyline's whole-body zero-loss assertion also covers the independently observed
pregame speculative suffix-recovery issue; it may require the companion
speculative-copular-probe repair. No assertion was relaxed to hide that loss.
