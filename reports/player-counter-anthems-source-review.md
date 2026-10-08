# Player-counter anthems: source-only recovery

Status: **UNVALIDATED / UNRUN**. No builds, tests, compiler probes, formatters,
corpus runs, generation, compatibility-constant changes, remote writes, or PR
publication were performed for this recovery.

Base: `b48c47847092989dbb2eee11db7f2a94ac74223b`.

## Candidate admissions

The shared typed anthem implementation and exact-source fixtures cover:

- Kalemne, Disciple of Iroas: source-only +1/+1 per controller experience counter.
- Kelsien, the Plague: source-only +1/+1 per controller experience counter.
- Minthara, Merciless Soul: controlled creatures receive +1/+0 per controller
  experience counter.
- Mycosynth Fiend: source-only +1/+1 per poison counter held by opponents.
- Vishgraz, the Doomhive: source-only +1/+1 per poison counter held by opponents.

These are source candidates, not verified recovered cards. All five remain held
from verified admission until the coordinated validation gate. No candidate was
removed from the inherited fixture, and no full-card success is asserted.

## Recovery findings and repairs

- Preserved and reviewed the surviving diff, including its typed
  `AnthemCountExpression::PlayerCounters` variant, runtime evaluation, rendering,
  text-change traversal, iterated-player validation, and scalar-range admission.
- Fixed the missing shared grammar for `counter your opponents have`; the
  inherited anthem reader only delegated to a parser supporting `you have`.
  Both for-each and number-of expressions now retain the opponent domain.
- Kept exact phrase consumption at the standalone anthem-reader boundary.
- Corrected an inherited display expectation that incorrectly capitalized the
  ordinary creature-filter subject.
- Player counts use the current ability controller, ignore players no longer in
  the game, and deduplicate shared Two-Headed Giant poison pools. Experience
  remains an individual-player counter.
- Wide aggregation and multiplication are checked before infallible effect
  generation, including signed modifiers and caps. Existing enum variants retain
  their positions; the new variant is appended. Compatibility constants remain
  unchanged for the coordinated gate.

## Authored independent expectations

- Typed grammar subjects, counter kinds, player domains, component signs,
  where-X surface, and rejection of unconsumed or mismatched possession tails.
- Native numeric boundaries, caps, negative multipliers, live-player filtering,
  multiplayer sums, and Two-Headed Giant poison-versus-experience behavior.
- Exact full-card compilation for all five fixtures, lossless parse reporting,
  JSON artifact reconstruction, typed anthem assertions, metadata preservation,
  rendering, and reparse checks.
- Direct and artifact-materialized runtime expectations for counter addition and
  removal, zero counts, irrelevant counter kinds, unaffected peer creatures,
  source-controller changes, and Minthara's changing recipients and power-only
  modifier.

Every expectation above is authored but unrun. Compilation, runtime behavior,
artifact compatibility, and full-card recovery are therefore unestablished.
The five cards' non-anthem abilities were retained verbatim and are exercised
only by the authored full-card compile/reparse checks, not by dedicated behavior
scenarios in this slice. Those abilities, and the coordinated compatibility
decision, remain validation work; no known additional source defect is claimed
fixed outside this anthem family.

## Independent-review correction

Source review identified that `compile_to_artifact` returns an
artifact-materialized companion, so using that companion as the "direct" case
did not cover an independent compilation path. The helper now separately calls
`compile_to_runtime_definition`, checks parse-loss reporting independently for
both routes, validates the JSON-decoded artifact, and runs the same authored
semantic scenarios against the independent direct and decoded-artifact results.
This correction is source-only and remains **UNVALIDATED / UNRUN**.
