# Complete paper sync and inventory admission: source review

Reviewed 2026-10-12 against prerequisite `ace151344c47ffea83f416ca06bc0a596cc12530`,
which is current main `03e91ae2b6d962df1905ca9ea7be3e82f77c0353` plus the missing
`ironsmith-text -> memchr` lockfile edge. This change is Python data admission
and audit orchestration only. It changes no Rust compiler or engine semantics.

## Failure and correction

The Python downloader and canonical campaign inventory omitted Pioneer and
recognized only `legal` status. Current `crates/ironsmith-tools/src/tooling.rs`
and `crates/ironsmith-card-source/src/lib.rs` recognize Commander, Standard,
Modern, Pioneer, Legacy, and Vintage, with both `legal` and `restricted` status.
The campaign's independently inspected fresh raw Scryfall input identifies 16
restricted-only canonical identities omitted by the old downloader, including
Black Lotus, Time Walk, Flash, and Tinker. That finding motivates a policy fix;
it is not a new compilation or gameplay result from this change.

`stream_scryfall_blocks.py` now owns the shared six-format/two-status policy.
`download_scryfall_cards.py`, `card_failure_campaign.py`,
`card_failure_faces.py`, and `card_failure_tag_clusters.py` consume that policy
through the appropriate admission boundary. This is not a card-name exception
list, and no support result, parser loss, or failed body is hidden.

## Preserved admission boundaries

- **Downloader:** Requires affirmative `legal` or `restricted` status in one of
  the six formats, then retains the existing `is_non_paper_print` exclusion.
  Digital cards and known Arena/MTGO-only printings remain excluded. Missing,
  empty, or non-object legalities do not become download-eligible. Missing or
  malformed `games` metadata retains the old fallback behavior; this patch
  does not redesign medium validation.
- **Canonical inventory:** Matches the Rust registry's default, non-explicit-
  include path. Only boolean `digital: true` excludes a source as digital;
  `games` is not an additional registry filter. Missing, empty, or non-object
  legalities are allowed for custom records, while a nonempty object must
  contain an accepted format/status. Name rejection precedes legality
  rejection, as in the Rust record constructor. Supplemental explicit-include
  exceptions are not silently enabled in full-corpus inventory.
- **Names:** Only missing/null root names fall back to the first face. An empty
  or whitespace root name does not fall back, and a malformed first face does
  not cause selection of a later face. Non-null JSON booleans, bounded integers,
  arrays, and objects stringify as compact JSON rather than being silently
  excluded or incorrectly replaced with a face name. JSON object keys are
  sorted, matching the current default `serde_json::Map` ordering. Names then
  use the existing trim/single-slash normalization. First eligible canonical
  name wins; different names with the same Oracle ID are not collapsed.
- **Fail-closed limits:** Non-string names containing floating-point numbers
  or integers outside serde_json's integer range raise an explicit inventory
  error. Python and Rust need not render these malformed numerical names
  identically, so the audit must not shard guessed names and report complete
  coverage. This is an explicit custom-input limitation, not a claim of
  byte-identical handling for every malformed JSON value. Non-object top-level
  source records remain rejected by the existing inventory contract.
  Both canonical and pinned-tag source JSON decoders preserve the raw integer
  token `-0` as floating `-0.0`, matching serde_json 1.0.149's `parse_number`
  branch in `src/de.rs`. Otherwise Python would erase the sign and permit
  deduplication against a distinct Rust `0` name before sharding. The existing
  numerical-name guard then rejects root, nested, and first-face fallback
  occurrences; ordinary positive zero and negative zero outside the effective
  canonical name remain allowed. This correction was independently identified
  during source review after the initial repair commit.
- **Source face metadata:** A non-array `card_faces` value is not interpreted
  as faces; non-object array members expose null face-name metadata. Valid
  root names remain eligible, matching the Rust first-face access boundary.
- **Tag report:** Still requires every pinned source entry and rejects any
  exclusion or deduplication. Hash/byte-count/name-count checks remain intact.
  Restricted/Pioneer entries no longer cause a false exclusion error.
- **Supplemental face report:** Canonical collision checks now recognize
  restricted and Pioneer cards. Explicit face matches continue to have the
  Rust lookup path's separate legality behavior; banned canonical records
  cannot incorrectly hide a later eligible explicit face route.

The stream block builder and generated-registry users were reviewed before
changing the shared helper. `generate_baked_registry.py` imports other stream
helpers, not the legality predicate. `is_non_playable`, Oracle marker logic,
block construction, and runtime/generator behavior are unchanged.

## Cache migration

Changing filtering alone is insufficient: the downloader formerly skipped a
local file whenever its upstream type/date and content hash still matched,
even if its selection policy was obsolete. Sidecars now record
`filter_policy_version: 2`; cache acceptance requires this exact version in
addition to existing identity checks. Version 1 denotes the previous implicit
five-format/legal-only policy. Absent, old, or unknown policy versions require
a refresh even when the upstream snapshot has not changed. Non-object sidecars
are also treated as cache misses. The metadata schema remains version 1 with
an additive policy field.

When retaining a standalone external copy of `card_failure_campaign.py`, also
retain its matching sibling `stream_scryfall_blocks.py`, or use an independently
reviewed self-contained equivalent. A harness copied without its new dependency
will fail to import; it must not be used as baseline evidence. A manually
corrected baseline sidecar should record the same policy version only when its
input really was selected under this policy. No baseline checkout or frozen
input was edited by this repair worker.

## Authored regression coverage

- `scripts/test_scryfall_scope.py`: all six formats and both accepted statuses;
  unsupported formats, banned/not-legal/malformed statuses; digital and known
  nonpaper exclusions; explicit unknown-legality boundary; format subsets;
  Oracle/name deduplication and English-print/first-paper-set selection;
  cache policy migration, source identity, and malformed sidecars.
- `scripts/test_card_failure_campaign.py`: full admitted format/status matrix,
  digital boolean semantics, registry-only missing-legality and games behavior,
  first eligible normalized-name selection, Oracle-ID non-deduplication,
  first-face fallback, JSON-valued names, numerical-name fail-closed behavior,
  malformed face metadata, and hand-authored raw negative-zero JSON names.
- `scripts/test_card_failure_faces.py`: restricted/Pioneer canonical collisions,
  ineligible canonical records versus explicit face routes, and the registry
  missing-legality boundary.
- `scripts/test_card_failure_tag_clusters.py`: pinned restricted/Pioneer
  admission, null root-name fallback, JSON boolean canonical names, and raw
  negative-zero names in both archived and restored source-input routes.

**Authored and source-reviewed only; none of these tests was executed.** No
build, compile/probe, corpus replay, generator execution, or Python import/
syntax-check invocation was performed. `git diff --check` is the only automated
change check performed for the initial repair; it verifies whitespace only.
The negative-zero correction was reviewed as source without running any checks.
A later authorized focused test invocation is:

```sh
PYTHONPATH=scripts python3 -m unittest \
  test_scryfall_scope test_card_failure_campaign \
  test_card_failure_faces test_card_failure_tag_clusters
```

The no-execution gate remains in force. This change establishes no recovered
card count, no new full-corpus result, and no gameplay correctness claim.

## Separate pre-existing face-audit limitation

Current Rust `load_card_payloads_by_names` groups all matching same-name faces
within a source record before inserting them. The older supplemental Python
`make_inventory` selects the first matching face and marks subsequent same-name
faces as shadowed; its existing reversible-card test codifies that older
behavior. This patch deliberately changes only that report's legality policy,
not its route accounting. Do not use the supplemental Python lookup simulation
to claim current exact duplicate-face coverage without separately reconciling
that behavior. Canonical full-corpus membership is unaffected by this issue.
