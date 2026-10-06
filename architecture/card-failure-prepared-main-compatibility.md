# Prepared main compatibility boundary

Status: **UNVALIDATED source proposal**. No builds, compilation, probes,
formatters, tests, artifact generation, browser scenarios, or replay ran.
This boundary composes main `f711839e5521c258ceccca52fa6b85037496b4ba`
with the preserved campaign through `c4c7818e5f64d60b6f7f08c674d5b53496d7023c`
and the reviewed prepared-owner corrections. Source admission is recorded separately in
`card-failure-stage97-source-admission.md`; this gate establishes no executed
runtime compatibility.

## One successor to published 8 / 4 / 21

The assembled source requires compiled artifact **9**, public audit digest
**5**, and signed audit protocol **22**. Each is an independent surface:

- Artifact 9 adds ordered `VoteEffect.payloads` / `VotePayload` programs, preserves
  predefined/native token names, word roles and ability occurrences, and executes
  through prepared selection/original/freeze/observation/completion owners.
  The prior numeric, mana-origin, selected-permission and required text metadata
  contracts remain. Format 8 is rejected even with a valid checksum; format 9
  with the previous schema fingerprint is also rejected. Definitions must be
  recompiled from their original source. Relabeling is not migration.
- Digest 5 reflects nested evidence encoding as well as existing public names.
  `SyncClaimContextObjects.contextObjects` serializes full `ObjectSnapshot`
  values (including `stack_kind`) and effect facts such as `ActionObjects`,
  `ResultObjectMemory`, `PreventedDamageReceipt` and `RequestedAmount`.
  The hidden-claim ledger hash commits those bytes even when the top-level
  checkpoint structure is unchanged. Public snapshot projection strips private
  and executable data. Native provenance receipt identities require canonical
  public projection; the version number alone cannot establish determinism.
- Protocol 22 is required on both transcript and match for current-engine replay
  and by host, direct-peer and client admission before dispatch. Historical21 is
  retained explicitly alongside 14/16/17/18/19/20 for signature-only verification.

The schema compatibility fingerprint is `b3895accd8d36443d5ec72a6985ebf4e96650975eaa53d8581e484040dd312d7`.
It is SHA-256 of the exact UTF-8 descriptor in
[`prepared-main-schema.descriptor`](prepared-main-schema.descriptor),
without a trailing newline:

```text
ironsmith-compiled-artifact-v9;parent=cb108f20f047d8702f593fc004a8f8e4cebc1c7cd29a9b0eef7b7590cf9adcad;ordered-vote-payloads;predefined-and-native-token-names-word-roles-and-ability-occurrences;prepared-action-original-observation-completion-and-full-snapshot-evidence
```

This is an explicit compatibility fingerprint, not generated exhaustive Rust
schema evidence. It succeeds published schema
`cb108f20f047d8702f593fc004a8f8e4cebc1c7cd29a9b0eef7b7590cf9adcad`.
The public checkpoint hash domain and normalization remain unchanged: the
version and actual evidence already participate in hashing.

## Historical evidence and recovery

Signature-only verification preserves the original signed genesis, actions and
checkpoint payloads. Protocol21/digest 4 evidence is not enriched with
`stack_kind`, new execution facts, or manufactured empty claims before hashing.
A replay callback, including one supplied with `requireEngineReplay: false`,
requires current versions before engine access. Relabeling old signed versions
invalidates their signed genesis; signature validity never permits old-engine
semantics to run in the current engine.

Missing action evidence and recorded empty evidence remain distinct. Likewise
`RequestedAmount(0)` is different from no quantity receipt. An empty claim
ledger continues to omit its digest rather than synthesizing an empty-ledger
hash. Convenience readers that intentionally expose an empty collection are not
proof that every missing-evidence consumer fails closed.

Exact native root/inactive-lane savepoints, branch exchange and authenticated
full-genesis replay remain the gameplay recovery owners. Public digest 5 is
redacted audit evidence, not a serialized gameplay checkpoint. This change
introduces no gameplay import/export serializer and no historical payload repair.

## Deferred validation and provisioning

Authored native/public claim projection, artifact admission, protocol 21
signature-only, mixed-version replay, current export and all three live peer
admission scenarios remain unrun. The final validation must include nested
snapshot redaction, exact known-empty receipts, public receipt alias identity,
allocation-order independence, pending/error rollback, and complete prepared
prefix/resume behavior together with the unchanged baseline corpus.

The pre-existing `golden_json_is_stable` reference to missing `fixtures/v5.json`
remains a provisioning prerequisite. Read-only local history found its introduction
in `59c3f5787d1d159f8fed2c5cba34e4061650b59b`; that tree contains only `v3.json`,
and no reachable local commit tracks `v5.json`. This is not evidence of an
available authentic v5 fixture. Do not fabricate or relabel a historical golden.
After execution is authorized, generate the actual current artifact golden and
update its reference together, retaining authentic older fixtures and negative
old-format/schema admission cases. Regenerate affected catalogs, text/checksums,
and current digest expectations from source, without rewriting signed transcripts.

The paired native-token 19 correction is included through reviewed port
`03a183a8be25c7fb7ec590fa648ce25adbb97028`. It adds definition/name/role values
within this unpublished token-semantics boundary and introduces no further
serialized shapes. Unintegrated protection 8, blind-exile/Rogue, Endure and broad
text-card work are excluded. Later compiled/public model changes require their
own explicit compatibility assessment.
