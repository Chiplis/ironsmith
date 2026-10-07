# Numeric and counted-counter compatibility boundary

Status: **UNVALIDATED, source-only integration**. This coordinates numeric5 at
`29487ada302c6fc9de518af1319c1152ef4d2953` with the published-owner/counted-counter
union at `32492c6eb8d3c048dfd6c53335a911c1309631e0`. It does not publish a release,
promote any card, reconcile the ledger, or establish compilation or recovery.
No builds, compilation, probes, formatters, tests, corpus, engine replay or
browser execution occurred. Authored regressions remain unrun.

## One successor to the stage93/stage94 contract

The staged contract is compiled artifact **8**, public audit digest **4**, signed
audit protocol **21**. Stage93/stage94's **7/3/20** contract is historical for
this union. These are independent surfaces coordinated in one boundary:

- Artifact8 admits typed source-number producer retention, immutable numeric
  program pairs, acquisition-bound chosen-number readers and exact local
  number/color/reveal receipts. It also admits the counted-transfer
  `CounterMoveAmount::All` and kind-specific `PriorEffectAction::CountersMoved`
  variants. Original-removal receipts, all-donor replacement ordering and checked
  snapshot evidence change executable behavior. Existing authored/intrinsic mana,
  selected-permission, typed text and canonical metadata requirements remain.
  The subsequent token integration adds ordinary-description word-role evidence
  and CR 111.4 subtype-derived token names to this same unpublished staged gate.
- Digest4 adds canonical `numericChoices` proof: per-source completed public
  group ordinals, immutable definition/pair and value records, plus current
  numeric ability-slot bindings to a group or explicit never-chosen evidence.
  Dormant groups remain evidence. Hidden identities suppress numeric metadata.
  Native allocation IDs and executable acquisitions are excluded. The numeric
  owner, not this compatibility gate, constructs and checks the proof.
- Protocol21 is required on both transcript and match before current-engine
  replay. The peer lobby derives its current version from this same constant;
  host, client and direct-peer handlers reject noncurrent messages before their
  dispatch. Protocol20 is now historical even when its signatures are valid.

`ENGINE_SCHEMA_HASH` changes from
`f4872928326e3ea14e25666b90d7eb4ce9add2d18c628c2c1936c96f100e4d96` to
`cb108f20f047d8702f593fc004a8f8e4cebc1c7cd29a9b0eef7b7590cf9adcad`.
This is an explicit schema compatibility fingerprint, not a generated full
Rust-schema inventory. It is SHA-256 of the following exact UTF-8 descriptor,
without a trailing newline:

```text
ironsmith-compiled-artifact-v8;parent=f4872928326e3ea14e25666b90d7eb4ce9add2d18c628c2c1936c96f100e4d96;source-number-acquisitions-and-exact-producer-receipts;counted-counter-transfer-all-and-kind-bound-original-removal;ordinary-token-authored-word-roles-and-derived-names
```

Unlike stage93's shape-preserving migration, the union adds typed variants and
fields. Both format and schema admission therefore change. A format7 artifact
with a fresh checksum is rejected; an artifact8 envelope retaining the previous
schema fingerprint is also rejected. A checksum proves byte integrity, not that
provenance or semantics were migrated. Relabeling both envelope fields cannot
repair an old definition and is not a supported migration.

The pre-token staged fingerprint
`3f9f096868c6ec7f6437ea2d249befbab23ffefe8604470d4ea4b0018f6c2b08`
is likewise rejected even with format8 and a refreshed checksum. The descriptor
extension was hashed directly from UTF-8 source bytes without a trailing newline;
no schema generator or engine ran. Public digest4 and audit21 are unchanged:
the token metadata lives in executable definitions, not new public proof fields.
See `card-failure-token-role-integration.md` for the retained native boundary.

## Historical evidence and current recovery

Signature-only verification continues to admit protocols14/16/17/18/19/20 with
their original signed payloads and canonical checkpoint bytes. No missing
numeric proof or cloak fields are injected into historical evidence, and neither
versions nor signatures are rewritten. Relabeling a signed transcript changes
its signed genesis payload; adding empty numeric proof changes its digest.
Historical signature validity cannot authorize current replay or live peers.
Current replay also requires digest4 at each existing checkpoint admission.

Gameplay recovery remains owned by exact native root and inactive-lane
savepoints, analysis branch exchange and rollback, followed where necessary by
verified accepted transcript replay from genesis. Current replay failure still
restores the caller's native state and cannot authorize a partially initialized
action session. Public digest4 is redacted audit evidence, not a gameplay
snapshot; no serializer/importer is introduced. Public/legacy snapshots cannot
reconstruct executable acquisition memory or ability origins. Missing required
numeric, activation, cast or movement evidence remains a typed failure.

This supersedes only the version descriptions in
`card-failure-current-recovery-boundaries.md`; its native recovery ownership and
absence of serialized gameplay checkpoint APIs remain authoritative. The source
details are in `card-failure-source-number-bodies.md`,
`card-failure-checked-snapshot-evidence.md` and
`counted-counter-transfer-bodies.md`. Earlier reports retain their historical
boundary statements rather than being retroactively rewritten.

## Required work after the execution gate opens

1. Restore or otherwise resolve the pre-existing absent
   `crates/ironsmith-compiled-artifact/fixtures/v5.json` prerequisite: the current
   `golden_json_is_stable` test still references it, while only `v3.json` exists.
   Preserve authentic legacy evidence; do not invent or relabel an old golden.
2. Recompile artifacts from the original frozen card inputs with the combined
   owner implementation. Regenerate the current artifact golden and update its
   reference together; regenerate affected artifact catalogs, canonical text,
   checksums and cached public-hash expectations using format8/schema8/digest4.
   A source-only cache key is insufficient admission for older artifacts.
   No artifact, golden, catalog, cache or signed transcript was regenerated here.
3. Execute independent full-body direct/native/artifact scenarios and typed-codec
   regressions, including prior counted-transfer and published-owner consumers.
   Exercise number/color/reveal receipt binding, acquisition/copy/expiry and
   checked snapshot capture, all-donor original/removal/replacement outcomes and
   rejected incompatible artifact envelopes before promoting any identity.
4. Execute authored audit regressions: original historical signature/hash
   preservation, protocol20/current-version mismatches, digest3 rejection,
   peer admission, numeric never-chosen/zero/large proof distinction, hidden
   proof suppression and allocation-order independence. Verify exact native
   root/inactive-lane retention, pending/error rollback, analysis cancellation,
   and current protocol21 transcript replay. Historical engine replay is not
   promised by signature-only verification.

## Source preservation

These SHA-256 values were inspected before the compatibility edits; no ledger
or reconciliation bytes are intentionally changed by this work:

- `source-coverage.json`: `534ebb2337f25d0e6da070c7a9e139926cd0316842235f25f20c6260bd724af2`
- `published-stack.json`: `e95791978394a5eceb8969286e503c9d23033399b568452f727707a12776244a`
- `reconciliation-20261005-second.json`: `0a44a2a5bc35c173db00767f9d9741b260897b8d01e849492acca7d68a4f5a1f`
- `reconciliation-20261006.json`: `47e9efea0337088470cb9caf55db5810a06dd8e9ec7da0c547b1d6970ba500f4`

All four files are under `fixtures/card-failure-campaign/`. No publication,
source-count increase or measured recovery is claimed.
