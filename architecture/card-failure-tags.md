# Offline Oracle-tag enrichment for the card-failure campaign

`scripts/card_failure_tag_clusters.py` adds reproducible investigation hints to
an **already completed authoritative full-corpus snapshot**. It does not compile
cards, access Scryfall, query Tagger GraphQL, update a database, change corpus
membership, or decide whether a mechanic is implemented.

## Preserved inputs

The campaign fixture directory contains the original public Oracle Tags gzip:

- Discovery: <https://api.scryfall.com/bulk-data>
- Metadata: <https://api.scryfall.com/bulk-data/bd8df61e-5d0a-47a2-9086-40137a645b98>
- Download: <https://data.scryfall.io/oracle-tags/oracle-tags-20261003090035.jsonl.gz>
- Updated: `2026-10-03T09:00:35.876+00:00`
- Bytes: `5,977,482`
- SHA-256: `1a6c699e871041a077a7f20f964fb1a9679906fb2184e7941b8bc2ea9c43c6a1`

`oracle-tags.metadata.json` preserves the original source pin and validation
research. Its historic local paths and artifact list also describe research
outputs; the redundant 5 MB Oracle-ID map, 2 MB catalog, and prototype helper are
not committed or required. The production script reconstructs the direct index
and parent graph from the verified raw snapshot. Weights and annotations remain
in the raw snapshot; this report does not interpret them as confidence scores.

`oracle-tags.selected-functional-categories.json` is a small, pinned, human
curation of relevant functional labels. It is **not** an official Scryfall
functional/nonfunctional classifier. Only its selected slugs define report
categories; its historical local counts and descriptions are informational.
Category memberships are recalculated from the verified source for every report.
Other direct tags, including cycles, are retained on each failed entry but never
promoted into a functional category automatically.

The script verifies the corpus compressed and decompressed hashes and byte
counts, raw tag hash and byte count, and functional-selection hash and byte
count before use. It validates tag counts, unique IDs/slugs, direct membership,
reciprocal parent/child edges, and an acyclic hierarchy. It rejects unknown
functional selections and unknown hierarchy edges.

## Generate a report

After the existing campaign audit has published `snapshot.json`:

```sh
python3 scripts/card_failure_tag_clusters.py \
  --snapshot /tmp/ironsmith-campaign-baseline/snapshot.json \
  --out /tmp/ironsmith-campaign-tag-clusters.json
```

The exact pinned card archive is read by default. To reuse an already restored
corpus, supply `--cards /tmp/ironsmith-campaign-cards.json`; its bytes must match
the same campaign pin. `--fixtures` selects another complete fixture directory
for tests. There are no downloads or refresh options. An existing output file is
never overwritten. Exit 0 means the report was generated, not that failures are
fixed; exit 2 means invalid input or an operational failure.

The report requires the authoritative full-corpus mode, complete coverage,
clean compiler commit/tree provenance, every expected canonical name exactly
once, matching dataset and canonical-name hashes, and a summary matching the
actual observations. Even an internally consistent smaller snapshot is refused.
The pinned corpus cannot require filtering or duplicate-name removal. Do not
construct a full-coverage flag around partial compiler output.

Identical inputs produce identical JSON bytes. Provenance includes the compiler
source, corpus and name-list hashes, corpus-manifest hash, exact snapshot-file
hash, canonical snapshot-content hash, and tag/selection/metadata hashes and
source URLs. Generated reports are local run artifacts; do not commit redundant
full reports or refresh the campaign's data pin between fixes.

## Interpretation and counting

- The primary partition is the existing campaign's normalized actual diagnostic
  signature, including both metadata-bearing and Oracle-only failures when
  available. Classification, acceptance, routes, and summaries are recomputed
  from authoritative row fields, ignoring claimed supported/category flags.
- Every failed canonical compile-entry name appears exactly once in that
  partition and once in `failed_compile_entries`, even when it has no tags.
  Raw error, parse-loss evidence, Oracle text, source-row position/hash, face
  names/IDs, and layout stay attached to the entry.
- Functional categories and Scryfall keywords are **secondary overlapping
  facets inside each diagnostic group**. An entry is counted once per category,
  even if multiple selected tags or faces match. Facet totals are not additive.
- Direct tag membership and derived ancestor-only membership are separate sorted
  lists. A tag with no direct members can still be a useful ancestor. Category
  membership is also reported as direct versus ancestor-only; if an entry has
  both kinds of evidence within one category, it counts once as direct.
- The join uses top-level Oracle IDs when present, otherwise distinct face IDs.
  It never uses name matching for tag lookup. The pinned corpus's 71 reversible
  entries resolve to Oracle IDs also held by ordinary entries; their compile
  observations remain distinct. Thus 32,209 compile entries represent 32,138
  distinct Oracle IDs. Failure-entry and unique-failing-Oracle-ID counts are
  reported separately; neither can be substituted for the other.
- Unknown tag membership stays explicit and is not an exclusion. Missing tags
  are unknown, not evidence that a mechanic is absent. A keyword/tag assigned to
  a whole card does not establish which ability failed.
- Semantic-mismatch counts include supported entries and are not silently
  recast as compile failures. Tag membership never changes acceptance, semantic
  scores, corpus membership, or campaign completion gates.

Use these hints to select failing representatives, related already-working
controls, and variations in targets, zones, optionality, quantities, duration,
and faces. Read the actual diagnostic and failing clause before declaring a
shared cause. General compiler fixes and focused runtime tests are still needed;
no tag proves a root cause, correct compilation, or correct gameplay.

## Tests

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s scripts -p 'test_card_failure*.py' -v
```

The additional tests use synthetic compiler evidence without Rust builds or
network calls. They cover graph integrity, direct/ancestor separation, unknown
IDs, alias and face identities, diagnostic-first partitioning, nonadditive
facets, source hashes, incomplete/changed evidence refusal, determinism, and all
71 aliases against the actual committed artifacts. These are tooling tests, not
proof that the compiler accepts the corpus.
