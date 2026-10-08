# Final independent source review: conditional untap

## Verdict

**All requested source-review blockers are closed at the reviewed anchor.** No production-source defect or authored-test API error was identified. The bounded conditional-untap repair and its authored full-body evidence are acceptable for the source-only repair stack. This is not successful test execution, measured regression clearance, semantic-score credit, or catalogue/source admission.

Exact final anchor: `f476216129bfa193f2a5af1a7d8362a1df07d6e4`.
Base: `1dd81cd84c62f272479f26e16d74719fff24b97b`.
Independently reviewed history: `a3535aea84df55135e50ce3e81f7bc23e524e825` → `ef985b4490d4c3f3313d5f93c90f8201e7d57489` → `ad46ff7186bf644dd21b410da36514f2afecc676` → `f476216129bfa193f2a5af1a7d8362a1df07d6e4`.

Detailed earlier reports in this directory are `conditional-untap-independent-review-a3535aea.md` and `conditional-untap-independent-review-ad46ff718.md`. Their open evidence findings are superseded by this verdict.

## Final two closures

1. `spell_mastery_counts_qualifying_cards_in_only_your_graveyard` now varies `zone` as well as owner/type. Own instant-plus-sorcery witnesses in hand and exile expect no freeze. They remain outside the graveyard during condition evaluation; the resolving spell's eventual movement cannot substitute for those witnesses. `Zone` derives `Copy`, so passing it repeatedly into the object helper is valid by source inspection.
2. `conditional_freeze_is_checked_at_resolution_and_binds_only_legal_targets` now creates an always-unselected existing B-controlled creature for every cast/resolution predicate combination and zero/partial/all-illegal target row. It checks that resolution did not tap this creature, independently taps it, and checks normal untap after B's first step. The sentinel is never selected or removed. It closes the previously vacuous empty/all-illegal object-set boundary while retaining surviving selected-target checks and the late-entrant check.

No production change was made in the follow-up.

## Combined source evidence

- Production dispatch keeps leading `if`/`unless` predicates out of the whole-document bare-restriction reader, consistently with the existing statement-level ownership boundary. Specialized low-level conditional restrictions remain unchanged.
- Grammar tests preserve conditional AST ownership and prior-object binding and reject invalid predicates, unsupported durations, and trailing junk.
- Both complete frozen bodies preserve metadata and labels, independently compile through direct and artifact routes, validate/round-trip artifacts, and inspect conditional restriction nesting. These are authored assertions, not observed successes.
- Resolution-time predicates are discriminated from cast-time and continuing predicates. Send to Sleep has threshold, same-type/mixed OR, nonqualifying type, graveyard owner, and zone negatives; Icy Blast has controller and inclusive power threshold cases.
- Target coverage includes own/opposing creatures, noncreature exclusion, zero/one/two selections, partial/all-illegal selections, existing unselected and late-created sentinels, plus Icy Blast X=3 with exact counts.
- Per-controller first/second untap expectations are authored through both routes. Existing standalone cohort coverage supplies additional controller-change, blink, and skipped-step scenarios; these are advisory inherited support, not newly executed evidence.
- Strict baker full-body coverage retains the unchanged `allow_unsupported: false` policy, loss checks, validation/materialization, no-unimplemented checks, and absent semantic score. Malformed whole bodies must error or record loss on both compile routes.

## Limits

All tests, builds, compiler probes, corpus refresh, and codegen remain **UNRUN**. No positive evidence is inferred from compile acceptance, and compile acceptance itself has not been observed here. No source edits, remote writes, or policy/admission changes were made by this reviewer. The worktree was clean at final anchor inspection; only ignored independent Markdown review reports were created.
