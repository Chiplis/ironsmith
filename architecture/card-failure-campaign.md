# Full-corpus card-failure campaign

## Current measured baseline and closed execution gate (2026-10-07)

At the user's request, defer **all builds, compilation probes, corpus replays,
and test execution** until reviewed source changes plausibly cover all or at
least a majority of the original frozen residual failures, excluding semantic
holds and partial bodies. Author regression tests and
review source now; do not run them. The following audit commands document the
later validation phase, not instructions to execute at each draft.

`fixtures/card-failure-campaign/workflow.json` records the requested policy.
`source-coverage.json` preserves the original frozen failure identity matrix.
`current-residual-source-coverage.json` records the completed clean-main refresh,
current residual scope, inherited re-holds, and current source proposals.
A placeholder, rejection, no-op, ignored clause, or dropped semantic requirement
never counts as coverage. Shared failure cards are deduplicated by Oracle ID.

The one-time authoritative refresh on main
`5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1` completed for **32,209 entries /
32,138 unique Oracle IDs**. There are **1,997 parse-failed entries / 1,994 unique
IDs**, **7 permissive fallback IDs**, and **4 strict-but-lossy IDs**: **2,005
unique unresolved identities / 2,008 entries**. Strict/no-loss supported entries
total **30,201**. The [measurement packet](../reports/card-failure-campaign/refresh-20261007-main5cc46c1/README.md)
pins the official October 7 snapshot, compiler source/tree, binary, and evidence.

Of the original 3,233 unresolved identities, **1,379 now satisfy the compile
gate** and **1,854 remain unresolved**. Another **151 formerly supported
identities regressed** and are part of the current 2,005-ID scope. Original and
fresh compiler-facing source fields are unchanged. The 1,343 semantic-mismatch
heuristic flags (1,341 strict-compiled and 2 permissive) are separate from the
88 marker-rejected semantic-output failures; flags can overlap lossiness.
Compiler acceptance is not gameplay proof, and linked faces remain unmeasured.

The completed refresh re-held **55 inherited IDs / 56 entries**. Reviewed counter
and static-prevention corrections now restore ten original-failure source
proposals, leaving **45 IDs / 46 entries** re-held with their historical evidence
preserved. The eligible original-failure source union is **1,238 unique / 1,242
entries**, including already-supported identities. Current residual source scope
is **96 IDs / 97 entries**: 19 prior proposals, 61 source-counter IDs / 62 entries,
and 16 static-prevention IDs. It comprises **40 full-body source proposals** and
**56 shared blocker repair proposed identities**; neither tier is runtime proof.

Of the **151 measured regressions**, **67 have source proposals** and **84 remain
unaddressed**. They add no original-majority credit. **Eight already-supported
semantic corrections** remain separate, making **104 touched IDs / 105 entries**,
not 104 recoveries. All **13 neighboring fixture partials** and all other holds
remain excluded. The [combined admission](card-failure-next-series-01-source-admission.md)
pins exact identities, source reviews, fixture hashes and the reviewed artifact12 /
public digest8 / signed audit25 / Manabrew3 boundary. No new-source recovery is measured.

The active execution criterion remains **1,597 source-eligible identities out
of the original frozen 3,193 residual identities**, excluding semantic holds,
re-held identities, and partial bodies. The reconciled **1,238** source-eligible
identities leave **359** short. The broad execution gate is
**CLOSED_UNTIL_ORIGINAL_SOURCE_MAJORITY**. The **96/2,005** residual proposal scope
guides repair prioritization; it does not replace that stopping condition.
A mathematical majority of 2,005 would be 1,003, with a difference of 907 from
96, but those numbers have no execution authority or gating role.

The one-time clean-main override is completed and consumed; it authorizes no
execution of the new source stack. Historical stack07's **40 measured recoveries
/ 3,193 remaining** are preserved, and its **1,597 source threshold remains the
active criterion**. The eventual authorized corpus, face, regression, and runtime
checks remain required; the source stack is **UNVALIDATED**.

The optimized build at `657aa12d` was interrupted with exit 130 when this change
arrived. Its cache and all earlier completed test evidence are preserved; no
second-batch full-corpus result exists.

## Original frozen baseline and historical measurement

The original engine baseline is commit
`e8740178a7f7367ffa3147e7642607042079237c`.
The Scryfall Default Cards source was updated
`2026-10-03T09:05:39.295+00:00`; filtering produced 166,747,166 bytes with SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.
The exact bytes are preserved in
`fixtures/card-failure-campaign/cards-20261003.json.xz`, a deterministic
10,971,452-byte xz archive. The sibling manifest and source metadata identify
both compressed and original content. Do not replace this baseline as fixes land.

The frozen authoritative audit is complete: 3,197 entries fail compilation and
41 strict-compiled entries are lossy. After resolving reversible aliases, these
represent 3,192 unique Oracle compilation failures and 3,233 unresolved Oracle
cards including lossy results. Full per-card evidence and diagnostic groups are pinned in
`baseline-e8740178.snapshot.json.gz`; `baseline-summary.json` records its hash
and summary. Tag-enriched groups can be reproduced offline with
`scripts/card_failure_tag_clusters.py` and the pinned Oracle-tags archive. There are
6,226 primary/fallback diagnostic records, not that many distinct cards. Runtime
verification and repairs remain in progress.

Frozen membership:

- 32,209 source entries and canonical compile-entry names
- 32,138 distinct non-null top-level Oracle IDs
- 71 entries without a top-level Oracle ID, all `reversible_card`
- 33,127 source face entries, counting ordinary single-faced cards once
- 0 duplicate non-null top-level Oracle IDs

The canonical snapshot compiles the selected front payload, not every linked
face. Supplemental face-route evidence is therefore required before claiming
complete card coverage; a successful canonical entry alone does not establish
that the back, adventure, prepared, or other linked face is supported.

These are different units. A reversible card, linked face, diagnostic route, or
multiple errors must not silently inflate or shrink the unique failing-card
count. The canonical compiler currently selects one payload per normalized
source name; linked-face metadata does not create another status DB row. All
32,209 entries remain in the audit, including the 71 reversible entries. Their
face Oracle IDs also exist among ordinary entries, but that is not grounds for
silently excluding them.

Canonical-name-list SHA-256:
`a87aae7335c88574841edb5e65899b554e61e81a7e3348f005e81339509e7dd0`.

## Existing authoritative path

`scripts/download_scryfall_cards.py` normally produces ignored `cards.json`
and a Scryfall metadata sidecar. `sync_registry_db` can store canonical source
payloads in ignored `reports/engine-status.sqlite3`. For this campaign, run
`sync_card_status_db --cards <frozen file> --db-path <fresh file>` directly; a
registry sync is not required for that route.

The status command calls `compile_authoritative_snapshot_from_payload`, which
checks generated unsupported content and required semantic output markers in
addition to parser acceptance. Do not substitute `--strict-only`, the older
`audit_compiled_cards` stream tool, an incremental missing-only bake, or a
selected-name subset for the full-corpus gate.

`latest_card_compilation` contains one current observation per canonical name.
Read that view, not all historical `card_compilation` records. Keep full SQLite
and logs as run artifacts; do not commit repeated huge databases.

## Reproduce and audit

From the branch containing this harness, restore the exact corpus once:

```sh
python3 scripts/card_failure_campaign.py restore \
  --archive fixtures/card-failure-campaign/cards-20261003.json.xz \
  --out /tmp/ironsmith-campaign-cards.json \
  --sha256 9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c

python3 scripts/card_failure_campaign.py inventory \
  --cards /tmp/ironsmith-campaign-cards.json \
  --out /tmp/ironsmith-campaign-corpus.json
```

Use an otherwise clean, committed checkout at the original baseline, and an
unused output directory. The harness may live outside the baseline checkout:

```sh
python3 scripts/card_failure_campaign.py run \
  --repo /path/to/baseline-checkout \
  --expected-commit e8740178a7f7367ffa3147e7642607042079237c \
  --cards /tmp/ironsmith-campaign-cards.json \
  --out-dir /tmp/ironsmith-campaign-baseline
```

The default command is `cargo run --locked -p ironsmith-tools --bin
sync_card_status_db`; `--release` is optional. Build and run configurations are
recorded. Set `CARGO_BUILD_JOBS=1` and a shared `CARGO_TARGET_DIR` when memory or
build serialization matters. Each audit captures a fresh DB, stdout/stderr logs,
source commit/tree, dataset digest, corpus membership, and `snapshot.json`.
It fails closed if membership differs, compilation exits unsuccessfully, the
source/dataset changes while running, or required evidence is absent.

A prebuilt, frozen binary is also supported. After a successful build from a
clean checkout, copy the binary without modifying it and record a JSON build
manifest with `source: {commit, tree, status: ""}`, `binary_sha256`, and the actual
successful `build_command`. Preserve the build log and toolchain details in
additional manifest fields. Use:

```sh
python3 scripts/card_failure_campaign.py run \
  --repo /path/to/repository \
  --expected-commit e8740178a7f7367ffa3147e7642607042079237c \
  --cards /tmp/ironsmith-campaign-cards.json \
  --out-dir /tmp/ironsmith-campaign-baseline \
  --sync-bin /path/to/frozen/sync_card_status_db \
  --build-manifest /path/to/frozen/build-manifest.json
```

For faster full-corpus runs, add `--processes 4` with the frozen-binary options.
Names are partitioned deterministically into disjoint shards, each in an isolated
process with its own SQLite database and stdout/stderr logs. `run.json` records
all commands and name-list hashes. No SQLite database merging is needed: the
harness validates each shard's exact membership, then rejects any aggregate
missing or duplicated name before publishing the full snapshot. Each process
still uses one Rayon worker. Allow enough memory for each worker to load the
source corpus. The default remains one process.

This verifies binary bytes and commit/tree identity against recorded provenance;
it does not independently prove that an arbitrary supplied binary was built from
those sources. The coordinator is responsible for recording the actual completed
build. A frozen-binary audit can continue while another checkout advances.

Audits clear inherited `IRONSMITH_*` parser/semantic overrides and set
`RAYON_NUM_THREADS=1` for conservative repeatability. The original baseline
toggles a process-global allow-unsupported variable, although its active payload
path passes `false` explicitly and does not consume that variable. The campaign
fixes policy propagation and removes that global mutation; no pre-existing
strict/allow contamination is claimed without reproduction.

Run a new full audit at each candidate integration commit using the same corpus,
then compare all entries:

```sh
python3 scripts/card_failure_campaign.py compare \
  --baseline /tmp/ironsmith-campaign-baseline/snapshot.json \
  --current /tmp/ironsmith-campaign-current/snapshot.json \
  --out /tmp/ironsmith-campaign-progress.json
```

Exit 0 means the compile gate is complete; 1 means unresolved baseline entries
or regressions remain; 2 means invalid evidence or an operational error. A `run`
exit of 0 only means the audit itself completed, even when cards still fail.

## Classification and route accounting

A repaired compile entry must be `strict_compiled`, contain no unimplemented
content, contain no parse loss, and have no parse error. In particular,
`strict_compiled` plus `oracle_only_fallback` is **not** fully repaired: the
fallback has discarded metadata-bearing parse input. `compiled_with_allow_unsupported`
is also never a successful repair.

Each snapshot retains raw errors, Oracle text, compiled text, parse-loss reasons,
semantic scores, and a compiled-definition digest. Failures are categorized as
parser failure, unsupported generated mechanics, semantic-output failure,
compiler panic, permissive fallback, or lossy compilation. A parser's
`UnsupportedLine(...)` is not by itself proof of missing runtime semantics.
Read the actual error and implementation before assigning a mechanic family.

Combined errors are split into `parse_input` and `oracle_only` diagnostic
records. A card failing both routes still counts as one failing card. A fallback
success preserves its primary failure from `parse_loss_reasons`. Counts of these
records are reported separately; repeated strict/allow attempts are not fully
observable from the DB. Unique grammar/parser-route identities are not recorded
by this pipeline, so the harness reports that count as unknown rather than
mislabeling error groups as routes.

Diagnostic groups are only triage aids. They remove quoted input examples and
numbers but retain outer diagnostic reasons, including Rust Debug wrappers.
They are not assumed to be unique mechanics. Scryfall keywords and explicit
Tagger labels may assist grouping; they never override actual compiler evidence.

## Completion gates and family ledger

`compile_campaign_complete` is deliberately **only** the compile gate. It
requires every original failing entry to become fully supported, no previously
supported entry to fail, no new semantic mismatch, and no supported-card
similarity-score decrease beyond floating-point tolerance. The comparison covers
the entire fixed corpus and rejects missing entries or a changed dataset. Changed
compiled-definition hashes are surfaced for review; an unchanged score is not a
proof that behavior stayed correct.

Overall campaign completion additionally requires all of the following:

1. Every baseline failing entry is assigned to at least one reviewed family.
2. Each resolved family has an implementation commit and stack PR reference.
3. Focused grammar tests cover the general language shape and variants.
4. Focused runtime tests verify affected effects, targets, zones, timing,
   cardinality, and relevant interactions. Record actual commands and exit codes.
5. Family semantic coverage is reviewed and verified, with no remaining gaps.
6. Every relevant face/linked-card compilation route has supplemental coverage,
   preserving card identity separately from source aliases and route counts.
7. The final full-corpus comparison passes and supported-card regressions have
   been investigated rather than suppressed.

`fixtures/card-failure-campaign/family-ledger.json` starts empty with baseline measurement
complete. Its sibling JSON Schema defines machine-readable family IDs, baseline
member names, commits, PRs, grammar/runtime commands and results, semantic
coverage, and review evidence. An empty ledger is not completion. After baseline
measurement, populate actual families and preserve unresolved or unsupported
families as open work. Mark overall `verified` only after all the gates above,
not automatically from the comparison script.

No card-name-specific parser hacks, fake success, excluded troublesome aliases,
weakened diagnostics, suppression lists, or moving baselines count as repairs.

## Harness verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s scripts -p test_card_failure_campaign.py -v
```

These Python tests exercise corpus identity, exact archive restoration, distinct
failure categories, wrapped diagnostics, two-route accounting, complete DB
coverage, lossy fallback rejection, and regressions in previously supported
entries. They do not replace Rust compiler or engine tests.

## Pinned tag-enrichment report

For deterministic, no-network diagnostic-first clustering with Oracle-ID joins,
see [Offline Oracle-tag enrichment](card-failure-tags.md). It preserves all
compile entries, separates direct and inherited tags, and keeps functional labels
and keywords as secondary hints rather than correctness or completion evidence.

## Source checkpoint 42 (UNVALIDATED)

The frozen matrix now contains 563 additional proposed unique cards (564
compile entries), including 29 identities with explicitly recorded token-limit
or damage-representation gaps. The 534 outside those recorded gaps are still
source proposals, not verified recoveries or exhaustive runtime claims.
Stage 42 adds 14 identities through exact Aura/source-exile references, dynamic
control and return bounds, live name selectors, per-opponent tap/reflexive
scopes and spell-copy prohibitions. Wedding Ring remains partial at the nested
quantified replacement-draw iterator boundary, despite the integrated original
receipt capture and branch/scope continuations.

The token resource-remediation chain is held outside this stage for source
review of payment-query and UI failure propagation. No cap closure is claimed
by publishing this branch. No builds, compiler probes or tests were run.

Main advanced to `2c6fc93258aab06df9b778d707f0a36293179771` (ZKP sync fixes)
after the frozen baseline. A read-only comparison found overlapping payment
hydration, public-opening evidence, proof-preserving retry and replay-resync
changes, plus referenced ChooseObjects X-bound corrections. Those upstream
changes are not a new corpus baseline and have not been blindly rebased into
the draft stack. Eventual integration must retain both those changes and this
campaign's payment-disclosure transaction/source-reference invariants.
