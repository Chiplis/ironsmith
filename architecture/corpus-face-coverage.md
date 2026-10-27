# Corpus face coverage

`sync_card_status_db` compiles and scores each face independently. Both its
`--cards` input and its default SQLite registry input expand `card_faces` without
restricting the layout. A single-faced card contributes one face observation.
Malformed named faces abort loading rather than silently disappearing.

The canonical card remains one row in `latest_card_compilation`:

- Any failed face makes the canonical card `parse_failed`.
- Its similarity and coverage values are the minimum across its faces.
- Semantic mismatch, unimplemented, and parse-loss flags are combined across faces.
- Multi-face source and compiled text are labeled with face names. If a face
  fails, the aggregate has no compiled text or compiled definition; the successful
  faces remain available separately.

`latest_card_face_compilation` exposes each face's name, zero-based index, source,
compiled text, definition, parse diagnostics, similarity, coverage, and loss
provenance. `card_name` is the canonical parent name; `face_name` is the independently
compiled name. Face names need not be globally unique.

Schema version 12 stores full face snapshots in `card_face_compilation`, keyed by
parent compilation ID and face index. Aggregate and face writes commit together.
Multi-face aggregate hashes include every face snapshot, so a change to a back
face invalidates the aggregate even if another face still fails. Old observations
remain in history; history cleanup also deletes their face snapshots. Worker/PR
flags and canonical card counts retain their existing meaning.

Migration alone does not manufacture per-face scores for old observations. Run:

```sh
cargo run --release -p ironsmith-tools --bin sync_card_status_db -- --strict-only
```

Use `--cards cards.json` to read the source file instead of the stored registry.
Name/tag filters select canonical cards and compile all of their faces.

The baked registry exporter reads independently validated face scores and never
falls back from a missing face score to the combined card score. An old database
without the face view must be refreshed before exporting.

Find actual coverage gaps with:

```sql
SELECT card_name, face_index, face_name, parse_status, similarity_score, parse_error
FROM latest_card_face_compilation
WHERE parse_status <> 'strict_compiled' OR similarity_score < 0.99
ORDER BY card_name, face_index;
```

Check whole-card coverage with:

```sql
SELECT parse_status, count(*) AS cards,
       sum(similarity_score < 0.99) AS below_threshold
FROM latest_card_compilation
GROUP BY parse_status;
```

Similarity remains a comparison of compiled text against source text. It does
not establish that runtime game behavior is correct.
