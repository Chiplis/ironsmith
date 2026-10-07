# One-time current-main compiler and card refresh

Source: `5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1`; tree `2ef96adada1e40b7f39922f24a1e4788de1a7386`. Dedicated detached validation checkout; no source patches.

## Outcome

The authoritative default audit completed for **32,209 compile entries / 32,138 unique Oracle cards**.
**1,994 unique Oracle cards fail default compilation; 2,005 remain unresolved when lossy and permissive results are included.**

Entry statuses: {'compiled_with_allow_unsupported': 7, 'parse_failed': 1997, 'strict_compiled': 30205}. There are 4 strict-compiled but lossy entries. Total parse-loss flags across all statuses: 1,916.
Semantic mismatch heuristic: 1,343 entries, separately from compiler acceptance. Category counts: {'allow_unsupported_fallback': 7, 'compiler_panic': 1, 'lossy_compilation': 4, 'parser_failure': 1905, 'semantic_output_failure': 88, 'strict_compiled': 30201, 'unsupported_mechanic': 3}.

Matched original identities: 1,379 recovered; 1,854 still failing; 151 formerly supported identities regressed; 28,754 remain supported. No selected Oracle text or loader metadata changed, so these differences are compiler-version differences for the same semantic inputs.
Additional entry-level regression signals: {'new_semantic_mismatch_heuristic': 593, 'previously_supported_entry_now_unresolved': 153, 'similarity_score_decrease': 854}. Changed compiled-definition hashes: 30,336. Similarity and definition changes require review; neither proves incorrect or correct gameplay.

The historical 3,233 unresolved identities, 40 measured recoveries / 3,193 remainder, and 1,256 source-proposed identities remain historical records. Source proposals were never substituted for measured success.
All 40 historical recoveries still satisfy the compile gate. Of the 1,256 historical source-proposed identities, 1,201 now satisfy the gate and 55 remain unresolved.

## Per-entry failure signatures and overlapping diagnostic causes

**912 normalized per-entry failure-signature groups** partition the 2,008 unresolved compile entries. Each entry has one signature, preserving both its primary error and Oracle-only fallback error when present; these are not primary-error-only groups. This is `summary.json`'s `entry_summary.diagnostic_group_count`, with exact signatures and entry membership retained in `current-failures.json.gz`'s `diagnostic_signature` fields.

The separate route-diagnostic inventory contains 3,812 records ({'oracle_only:compiled': 1, 'oracle_only:failed': 1905, 'parse_input:failed': 1906}). Records are not unique cards or proven parser-route identities. The pipeline does not record a distinct parser-route count.

Largest per-entry failure-signature groups (counts refer to the full signature; display abbreviated to the primary error, with exact primary/fallback errors and membership preserved in JSON):

- 247 entries: `parser_failure: parser does not yet support line family: <text> [rule-path=unsupported-line-family]`
- 62 entries: `parser_failure: could not find verb in effect clause (clause: <text>; known verbs: …) [rule-path=statement-line > triggered-line]`
- 62 entries: `parser_failure: could not find verb in effect clause (clause: <text>; known verbs: …)`
- 38 entries: `parser_failure: runtime compiler integration does not support effect conversion: no runtime conversion registered for compiler effect payload 'ironsmith_core::effect::mana_damage_and_control::RemoveAnyCountersFromSourceEffect'`
- 33 entries: `parser_failure: unsupported intervening-if predicate in triggered line: <text> [rule-path=statement-line > triggered-line]`
- 31 entries: `parser_failure: unsupported predicate (predicate: <text>) [rule-path=leading-if-conditional > sentence-reading] [rule-path=statement-line > triggered-line]`
- 30 entries: `parser_failure: could not find verb in effect clause (clause: <text>; known verbs: …) [rule-path=statement-line > statement-probe]`
- 25 entries: `parser_failure: could not find verb in effect clause (clause: <text>; known verbs: …) [rule-path=player-may > chain-reading] [rule-path=leading-player-may > sentence-reading] [rule-path=statement-line > triggered-line]`
- 24 entries: `parser_failure: cannot normalize authored definition graph: unknown compiled effect payload kind: RemoveAnyCountersFromSourceEffect`
- 21 entries: `parser_failure: unsupported predicate (predicate: <text>) [rule-path=leading-if-conditional > sentence-reading] [rule-path=statement-line > statement-probe]`
- 19 entries: `parser_failure: unsupported predicate (predicate: <text>) [rule-path=conditional-sentence-family > sentence-reading] [rule-path=statement-line]`
- 17 entries: `parser_failure: could not find verb in effect clause (clause: <text>; known verbs: …) [rule-path=conditional-sentence-family > legacy-document-registry-reading] [rule-path=statement-line]`
- 16 entries: `parser_failure: could not find verb in effect clause (clause: <text>; known verbs: …) [rule-path=player-may > chain-reading] [rule-path=leading-player-may > sentence-reading]`
- 15 entries: `parser_failure: unsupported predicate (predicate: <text>) [rule-path=conditional-sentence-family > legacy-document-registry-reading]`
- 14 entries: `parser_failure: unsupported triggered line: <text> [rule-path=statement-line > triggered-line]`

**923 overlapping diagnostic-cause groups** are a different view of the same unresolved entries. `diagnostic-cause-clusters.json.gz` separates individual primary/fallback diagnostics, or the authoritative-result diagnostic when route diagnostics are absent, and deduplicates Oracle IDs within each cause. This is `summary.json`'s `root_diagnostic_group_count`; `tag-enriched-diagnostic-priorities.json.gz` uses the same cause grouping. An entry or Oracle ID can belong to several causes, so cause memberships must not be summed as unique cards.

The largest cause, `parser_failure: parser does not yet support line family: <text> [rule-path=unsupported-line-family]`, covers **262 compile entries / 262 unique Oracle cards**, with 509 diagnostic records: 250 primary-input and 259 Oracle-only fallback records. The 247-entry signature above contains this cause in both primary and fallback positions; 15 additional entries contain it in only one position (3 primary-only, 12 fallback-only) and therefore have different full signatures. **247 and 262 are both correct for their respective groupings.** Neither grouping measures parser routes or proves distinct implementation defects.

## Concrete regression cluster

61 unique cards are rejected because `RemoveAnyCountersFromSourceEffect` lacks runtime conversion or compiled-payload recognition: 37 unique cards encounter conversion errors and 24 encounter authored-definition normalization errors. 56 of these cards were supported in the original baseline. `runtime-bridge-blockers.json` preserves exact identities and errors. This is a diagnosed source integration gap; no patch or additional execution was performed.

## Dataset and identity accounting

Official source: [Scryfall Default Cards](https://api.scryfall.com/bulk-data/default-cards), updated `2026-10-07T09:05:47.951+00:00`; [pinned raw download](https://data.scryfall.io/default-cards/default-cards-20261007090547.jsonl.gz). Download completed `2026-10-07T12:17:06.673884+00:00`.
- Raw: 78,779,281 bytes, SHA-256 `43da5fff200a7c8087eaa5b7bf80a64cc17998a7f92ad6b9d7508bafaef5f0d0`
- Filtered: 166,744,940 bytes, SHA-256 `de4cedd51e320d2983d80b38713213c0273fc7272d5e2a89908ec050061aeca5`
- 118,601 raw printing entries → 32,209 selected entries
- 32,138 distinct top-level Oracle IDs; 71 reversible entries without top-level IDs resolve unambiguously through their face IDs
- 0 genuinely new, 0 missing, 0 renamed/alias-changed identities; 0 Oracle-text changes; 0 semantic metadata changes
- 32,069 Scryfall records changed in other fields; full changed-field counts are recorded separately
- Original and current loader/Oracle semantic field projections are byte-identical after canonical JSON encoding: SHA-256 `8bcc5e77799e10c86a66efc1719c11645633985f985b5d16cfd5c3c26bb8b517`. This projection includes every canonical-loader input field and all source faces.

Selection uses the unchanged repository downloader: require legality in at least one of Commander, Standard, Modern, Legacy or Vintage; reject non-paper and digital prints; deduplicate by Oracle ID, otherwise exact name; prefer English/paper/non-digital/non-full-art then deterministic latest release/set/collector number/ID. Preserve earliest eligible first-printed-set metadata. Canonical loader membership is checked against the status DB.

Frozen source remains `e8740178a7f7367ffa3147e7642607042079237c`, filtered dataset SHA-256 `9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`; original checked-in archives and identities are unchanged. The built-in fixed-dataset comparison is intentionally not relabeled as run on a changed file digest; the supplemental comparison matches original Oracle identities and verifies unchanged semantic fields.

## Coverage limits

33,127 source face entries correspond to 32,209 canonical status rows. 918 extra face payloads are not independently executed in this refresh; layout counts: {'adventure': 151, 'flip': 20, 'modal_dfc': 98, 'prepare': 67, 'reversible_card': 71, 'split': 123, 'transform': 388}. Meld relationships are separately retained. A front-payload success does not prove linked/back/adventure/prepared-face coverage.

Compiler acceptance does not establish gameplay correctness. Runtime scenarios and broader tests were not executed. This was one bounded default authoritative corpus pass, including the pipeline’s own strict/allow-unsupported fallback; no duplicate strict-only full pass was run. Implementation-first/no-recurring-build policy resumes afterward.

## Reproduction and artifacts

`run-refresh.sh` records the exact single harness invocation. It runs `cargo run --locked -p ironsmith-tools --bin sync_card_status_db --release -- --cards <pinned-current-json> --db-path <fresh-status-db>` with Rust/Cargo 1.99.0, one build job, one Rayon thread, release opt-level 1, codegen-units 256 and debug 0. The harness removes inherited IRONSMITH overrides, validates clean commit/tree identity, exact corpus membership, data immutability and DB integrity. For reproduction, use a new unused output directory.

The clean-main release build completed successfully in 31m 46s; the one-process corpus pass took 2,445.852 seconds (2,434.608 seconds compiling snapshots). The fresh DB’s own before/after regression summary has no historical baseline; historical regressions above are independently joined against the frozen original snapshot.

Official minimal Rust was installed inside the ignored validation reports directory because the fresh executor initially lacked a Rust toolchain. A preflight also regenerated an already-tracked Python bytecode file; it was restored byte-for-byte and bytecode writes disabled before Cargo ran. Neither preflight launched compilation. No source patches, broad tests, merges, deployments, uploads or publication occurred.

## Additional inventory checks

The prior 55 proposed identities comprise 56 failing entries: 55 authoritative parse_failed entries / 54 IDs plus one permissive entry / one ID. Semantic-output-marker rejection is a subset of parse_failed: 14 entries / 13 IDs. Those rejections remain inside the authoritative gate; accepted semantic-mismatch signals are separate.

A read-only join of the new 27 local-cohort identities against this main audit finds 8 already supported on main and 19 unresolved: copular 2/11 supported/unresolved, destination 0/3, prevention 6/5. This does not execute or validate the new local source. Exact fixture/ledger hashes and statuses are preserved.

The verified, already-pinned October 3 Scryfall Tagger snapshot and selected categories enrich current diagnostic causes offline. Labels overlap and describe whole cards, not necessarily the failed ability; ancestor-only membership is derived. No fresh scrape or infrastructure was used.

## Compact publication packet

Report-only labeling correction, 2026-10-07: the distinction between failure-signature groups and overlapping diagnostic causes was checked by offline inspection of retained JSON and scripts. The measured source remains `5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1`; all measured JSON, archived evidence, and scripts are unchanged. No analysis, corpus pass, build, test, probe, or generation was rerun for this correction. Later source proposals remain **UNVALIDATED** by this report. The measured 1,994 compile-failing cards, 2,005 unresolved cards, 1,379 original recoveries, and 151 regressions are unchanged.

Report evidence only. No raw bulk download, full corpus, full snapshot, SQLite database, executable or build cache is included. Full local artifact hashes are in provenance.json. Analyze the retained full directory with `python3 analyze_refresh.py /path/to/reports/current-refresh-20261007`; this performs no compilation or tests. Gzip JSON streams are deterministic (mtime=0). Diagnostic cause memberships overlap and must not be summed as unique-card counts.

- `analyze_refresh.py`: 15,419 bytes
- `baseline-and-proposal-comparison.json.gz`: 195,494 bytes
- `current-failures.json.gz`: 468,167 bytes
- `current-family-counts.json`: 45,457 bytes
- `diagnostic-cause-clusters.json.gz`: 155,727 bytes
- `enrich_prior_and_causes.py`: 3,474 bytes
- `entry-regression-signals.json.gz`: 49,888 bytes
- `linked-face-coverage.json.gz`: 49,454 bytes
- `new-cohort-27-main-status.json.gz`: 3,990 bytes
- `prior-55-status-and-tags.json.gz`: 22,212 bytes
- `prior-proposals-55-unresolved.json.gz`: 20,656 bytes
- `provenance.json`: 16,800 bytes
- `runtime-bridge-blockers.json.gz`: 3,067 bytes
- `semantic-mismatch-signals.json.gz`: 205,494 bytes
- `source-family-results.json.gz`: 4,402 bytes
- `strict-lossy-results.json.gz`: 1,213 bytes
- `summary.json`: 10,082 bytes
- `tag-enriched-diagnostic-priorities.json.gz`: 88,731 bytes
