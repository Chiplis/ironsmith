# Exact source evidence ledger (reporting only)

This adds one standalone Python 3.11+ report generator. It does not change coverage policy, fixtures, runtime, or tests. The immutable 971 source variants remain the primary denominator. Derived candidate IDs resolve through `source_inventory_id`; original keyword/reminder and source scope must match the seed. Corrected executable keyword metadata does not change source identity.

`real_source_keys` is discovery only. Explicit `source_inventory_selection.exact_source_keys` / `exact_source_faces` establishes the selected physical-card and face denominator and supersedes discovery for that row. Extra candidates never enlarge that denominator. Exact full-card exclusions require an `explicit_user_scope_decision` fixture; they remain visible with evidence hashes and are reported separately. No keyword-wide exclusions are inferred.

Each source independently reports frontend availability, registered strict source verification by exercised route, independent-route verification, and gameplay verification. A uniquely agreed complete candidate face list can validate an exact per-source binding when discovery has no face metadata; it never establishes an explicit whole-row source denominator. `support_status`, candidate existence, registered target existence, and old logs are never native pass proof. Gameplay receipts remain separate from the strict source contract; partial or non-strict source gameplay never promotes strict full-source coverage. Artifact-only compilation and real artifact-runtime gameplay can qualify independently; missing direct-route independence does not erase a passing artifact scenario. A row is complete only when explicitly selected and every in-scope selected physical source has attributable evidence. An entirely excluded row does not become vacuously complete.

Run from the repository root:

```sh
python3 scripts/custom-mechanics/source-evidence-ledger.py --root . \
  --inventory /absolute/path/full-source-row-fulfillment.json \
  --inventory /absolute/path/selected-source-coverage.json \
  --fixtures fixtures/custom_mechanics \
  --bindings /absolute/path/source-test-bindings.json \
  --runs /absolute/path/native-run-receipts.json \
  --output /absolute/path/source-evidence.json
```

The optional binding file has a `bindings` array. Every binding records `source_inventory_id`, exact `source_key`, ordered `faces`, registered `target`, exact test `cases`, `fixture`, `fixture_sha256`, canonical `candidate_card_sha256`, `test_file_sha256`, `kind` (`strict_full_source_compile` or `gameplay`), `routes` (`["artifact"]`, `["native"]`, or both), `independent_native_artifact` (true only for separately exercised adapters), `review_reference`, and true `strict_word_claims`, `parse_loss_free`, `complete_source_faces`. If route implementation lives in a separate helper, also supply `route_adapter` and `route_adapter_sha256`. Canonical card hashes use sorted compact UTF-8 JSON with ensure_ascii=False. These binding declarations require reviewer attestation that the test covers that actual source through genuinely independent routes; the generator does not infer semantics from test filenames or function bodies.

The optional run file has a `runs` array. Every receipt records `target`, `log`, `log_sha256`, `exit_code`, `code_sha256`, and `inputs_sha256` (path-to-SHA256 map). `code_sha256` is emitted in every report and fingerprints current crates Rust/TOML/TSV plus root Cargo files/toolchain. Capture it at the tested revision and retain it with the result, never restamp a historical result. Inputs must include every bound fixture/test/route helper at their exact hashes. A qualifying log must identify `Running tests/<target>.rs`, have exit zero, a successful native summary, and individually passed named cases. A changed source or declared input invalidates current evidence while preserving the receipt and its validation errors. Separate processes/targets need separate receipts; aggregate Cargo output must not be attributed to an individual target.

No native runs were performed for this package. Generated sample reports intentionally have zero native verification because no immutable source/test/run receipts were supplied. Missing bindings are an evidence gap, not a claim that mechanics are unsupported. Frontend availability is text completeness only, not a parse or semantic claim.

The parent batch helper's actual pre-build protocol is also accepted through `--native-batch /absolute/output/batch.json`. The batch must declare `fingerprints_captured_before_build:true`, identify its captured `input-fingerprints.json` by path and SHA256, and retain sibling `results.jsonl`. The reader validates the exact captured path/hash array and current complete input inventory using the helper's enumeration scheme. Those actual captured inputs replace the legacy per-run code-fingerprint check; they are never synthesized for old batches. Normal log/target/case/success checks remain required. Changed or newly added inputs invalidate current proof. Historical case observations carried by bindings remain separate and cannot satisfy native source verification.
