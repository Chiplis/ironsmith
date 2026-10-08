# Oct8 clean-main compiler refresh: compact audit packet

Measured source: `1dd81cd84c62f272479f26e16d74719fff24b97b`; tree `ebac6ecfc19ceef218323c68f74b7e9b2b80d393`. This report-only packet preserves the completed one-time authoritative refresh, not later repair validation.

## Results and interpretation

- 32,209 compile entries / 32,138 unique Oracle IDs; 1,824 unique compile-failing IDs; 1,925 unresolved IDs (1,926 entries) under the strict, non-lossy, no-unimplemented gate.
- Entry statuses: 30,377 strict-compiled, 1,825 parse-failed, 7 permissive fallback. Of strict-compiled entries, 94 are lossy and excluded from supported results.
- Versus Oct7: 175 recoveries, 95 gate regressions (88 strict-lossy + 7 parse failures), 1,830 still unresolved, 30,038 still supported. All 4 prior strict-lossy entries remain; 88 previously supported entries and 2 previous failures account for the other 90. New loss detection can expose pre-existing parsing problems; these are not 88 demonstrated new gameplay defects.
- Versus the frozen original baseline: 1,445 measured recoveries, 1,788 still unresolved, 137 regressions, 28,768 still supported. The 1,445 is a new measured exact-ID result, not a historical source-eligible/source-proposed counter. Historical ledgers and the Oct7 packet are unchanged; source proposals are not measured successes.
- 182 suspected rendered-text flags (180 strict, 2 permissive). Versus Oct7: 49 newly flagged, 1,210 no longer flagged, 133 still flagged. These text heuristics are not confirmed gameplay defects or gameplay recoveries.
- 899 full per-entry failure signatures partition unresolved entries; 911 overlapping diagnostic causes retain exact memberships. Cause memberships must not be summed as unique cards or independent proven defects.

Compile acceptance does not establish runtime/gameplay correctness. No runtime scenarios or broad tests were performed by the refresh, and no compilation, tests, probes, code generation, or corpus execution was performed to create this packet. Later repair source is not validated here.

## Input equivalence and face coverage

All 32,138 identities have unchanged Oracle text and loader-semantic inputs; no new, missing, renamed, or alias-scope-changed identities. Full printing records differ in 32,093 cases outside the semantic projection. Original/Oct7/current semantic projection SHA-256: `8bcc5e77799e10c86a66efc1719c11645633985f985b5d16cfd5c3c26bb8b517`.

71 reversible alias entries collapse through unambiguous face Oracle IDs for unique-ID counts only. 33,127 source face payloads correspond to 32,209 canonical compile entries. The 918 supplemental linked/back/adventure/prepared face payloads were not independently executed; exact membership is retained in `linked-face-coverage.json.gz`.

Official Scryfall source updated `2026-10-08T09:05:44.946+00:00`; pinned download and metadata are in `data/download-provenance.json`. Filtered data SHA-256: `bae465b9d536fffa24c656daff5577a87f5a963dcb2160be0c1dc9ba8e225750`. Fresh snapshot SHA-256: `f7ab0ec3bd02283dcc0f723f1526fed3659f5a85ddb6d85e995fbbd0009314e1`.

## Evidence map

- `summary.json`, `dataset-summary.json`: original measured counts and input equivalence. `packet-summary.json` adds the explicitly derived, disjoint 88 strict-lossy / 7 parse-failed regression memberships. The 1,855 parse-loss flags across all statuses are separate from the 94 strict-lossy entries.
- `current-unresolved-entries.json.gz`: every unresolved entry, full primary/fallback diagnostics, parse-loss reasons, signature and identity.
- `original-identity-comparison.json.gz`, `oct7-identity-comparison.json.gz`: complete exact-ID transition partitions, including still-supported IDs.
- `regression-and-lossy-detail.json.gz`: exact 7 parse regressions, strict-lossy accounting; its historical `lossy_gate_regressions` key contains 95 parse-loss-flagged rows, overlapping the 7 parse failures. Filter that array to `parse_status == strict_compiled` to obtain the 88 strict-lossy regressions; all 94 current lossy rows are in the complete unresolved inventory.
- `suspected-rendered-text-signals.json.gz`, `oct7-rendered-signal-transitions.json.gz`, `original-entry-signal-transitions.json.gz`: heuristic details and transitions.
- `overlapping-diagnostic-causes.json.gz`: complete cause memberships.
- `input-index.json.gz`, `original-input-index.json.gz`, `input-partitions.json.gz`, `changed-input-evidence.json`, `linked-face-coverage.json.gz`: identity, alias, unchanged-input and unexecuted-face evidence.
- `analysis-README.md`, `unpublished-scope-current-main.md`: original analysis narrative and source-scope review, retained verbatim as historical evidence at the measured commit.
- `provenance.json`, `build-manifest.json`, `audit-authoritative/run.json`, `data/*.json`, `fresh-compilation-source-review.md`: original build/run, fresh-compilation and download provenance. Absolute paths describe the original local run, not portable locations.
- `analysis-provenance.json`, `source-analysis-manifest.json`: original analysis provenance and original file digests, including omitted local logs.
- `packet-provenance.json`: each preserved source path, original bytes/hash and transport encoding. Individual gzip streams use mtime=0 and decompress byte-for-byte to their source files.
- `manifest.json`: bytes/SHA-256 for every packet file except itself. `verify_packet.py` checks packet integrity and JSON accounting without invoking the compiler or reading the raw corpus.

Oct7 support is reconstructed from its complete retained unresolved inventory and cross-checked exact-ID outcomes. Its full snapshot was not retained; no full Oct7 compiled-definition or similarity-score comparison is claimed.

## Offline reproduction

Run `python3 reports/card-failure-campaign/refresh-20261008-main1dd81cd/verify_packet.py` from the repository to check this compact packet only.

`analyze_exact_ids.py` and `finalize_report.py` are preserved byte-for-byte, including their original path assumptions. To reproduce the full offline analysis, use a disposable checkout of the measured source and restore the omitted current filtered data and snapshot at `reports/current-refresh-20261008/data/cards-current.json` and `reports/current-refresh-20261008/audit-authoritative/snapshot.json`, verifying the hashes above. Restore `audit-authoritative/run.json` and `data/download-provenance.json` at the same original relative paths. Copy both analysis scripts into `reports/current-refresh-20261008/analysis/`. The frozen original data/snapshot, Oct7 packet and imported `scripts/card_failure_campaign.py` must match that measured checkout. From its root, run with `PYTHONDONTWRITEBYTECODE=1`:

1. `python3 reports/current-refresh-20261008/analysis/analyze_exact_ids.py prepare`
2. `python3 reports/current-refresh-20261008/analysis/analyze_exact_ids.py analyze`
3. `python3 reports/current-refresh-20261008/analysis/finalize_report.py`

These are offline analysis commands, not compiler runs. Do not execute the original `download-current.py`, `build.sh` or `run-audit.sh` merely to verify this packet: they are retained only as exact historical reproduction recipes and can download/build/run. They were not rerun during packaging. Generated full-directory manifests naturally depend on which ancillary files are restored.

## Intentionally omitted

No raw bulk download, filtered/full card corpus, full snapshot, SQLite database, executable, target/build output, toolchain, cache, or archive bundle is included. The original provenance retains hashes for the major omitted evidence. Build/command logs are omitted (their original recorded hashes remain in provenance where available); the complete unresolved/heuristic diagnostics and build/run metadata are preserved. The compact packet independently verifies retained report consistency, but cannot rederive all input semantics or compile outcomes without those omitted inputs. No new tags or later-branch coverage are claimed. Local report commit only; no remote writes were made by packet preparation.

## Former unpublished NEXT07 reconciliation

`next07-reconciliation.json` retains the exact six selected snapshot rows. All six satisfy the measured strict non-lossy gate. Halfdane alone has a heuristic flag and its rendering omits the required next-upkeep endpoint. The other five are unflagged, which does not validate gameplay. Ilysian Caryatid immediate-mana and Witch's Clinic cancellation/rollback concerns remain; all prior source/runtime holds are unchanged. This is no credit to missing unpublished source and does not justify replaying the old batch.
