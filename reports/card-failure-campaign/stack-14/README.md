# Stages 08–14: source implementation and deferred validation

Status: **UNVALIDATED against the frozen full corpus**. The latest completed
32,209-entry replay remains stage 07: 40 unique compile recoveries and 3,193
unique unresolved baseline cards. No recovery delta is claimed for this batch.

The exact identity matrix at `fixtures/card-failure-campaign/source-coverage.json`
records **141 additional unique baseline cards with proposed source coverage**.
This count is a planning estimate grounded in implemented families and exact
fixtures; it is not compile or gameplay acceptance. Front-face compile names are mapped to their frozen canonical entry identities;
shared cards are counted once. Mathemagics remains
unaddressed; its design document is not an implementation.

## Completed evidence before the workflow changed

Source commit: `657aa12ddf4f6d92d4fe718c7034286cddcf02ca`.

- 98 public runtime integration scenarios passed.
- Runtime library: 122 passed; the one previously reproduced Toxic failure
  remains. No new failure in that invocation.
- Grammar: 4,144 passed; exactly the same 38 failures as the frozen baseline.
- Core 130, lowering 59, and compiler-source 13 unit tests passed.
- Python campaign/tag/face harness tests: 88 passed.
- Five exact payload/host checks passed through a library-only harness using
  the repository's unchanged test modules. The ordinary tools invocation was
  blocked by the unrelated, unchanged `probe_badgermole_tap` binary's existing
  Option/EntryCommitResult type mismatch; it is not recorded as passing.

`validation.json`, `validation-log-index.json` and `validation-logs.tar.gz`
retain outcomes and hashed raw logs. Focused results do not establish full-card
rules correctness or complete regressions across the corpus.

## Deferred gates

The user changed the workflow to implementation-first on 2026-10-03. All further
builds, compilation probes and tests are deferred until source plausibly covers
all or at least a majority of remaining identities. The in-flight optimized
build was stopped with Ctrl-C (exit 130), preserving caches. Full-corpus replay,
all-face authoritative checks and broad validation are **not run** for stages
08–14. Draft titles/bodies make this limit explicit.

## Source families

08: Metadata/source identity, lexical pronouns, owned parser diagnostics, Ward
source-power costs, and preserved authored labels.

09: Shared keyword-action alternatives, passive noncombat damage, filtered
one-or-more discard, cycle/discard-other, source untap events, simultaneous
batch/source attribution.

10: Static clause dispatch, bare venture, correct replacement damage recipient,
conditional stat continuations and dungeon-completion cache invalidation.

11: Typed Job select token creation, controller choice and attachment.

12: Composed graveyard extra costs and dependent sequence-cost preflight.

13: Historical declaration-power Pack Tactics, real intervening-if gates and
coordinated stat/negative copular continuations.

14: Full-card Altaïr and Boseiju scenarios, Second Sunrise live-incarnation
correction, prior replay evidence, source coverage and workflow records.
