# Protection and targeting compatibility boundary

Status: UNVALIDATED source proposal. No build, compiler, test, formatter, code
probe, artifact generation, browser execution or replay has run. This succeeds
published PR 863's artifact 9 / public audit digest 5 / audit protocol 22.

The coordinated successor is artifact **10**, public audit digest **6**, and
signed audit protocol **23**. The source cohorts and their admission counts are
tracked separately; changing these gates does not itself prove a card works.

## Compiled definitions

`ProtectionFrom` appends `OwnColors`, `ColorsAmong { filter,
reference_source }`, and `ColorsAmongAtResolution` after the prior variants.
Published variant positions 0–12 remain intact; the additions occupy 13–15.
Live recipient colors, live population/source binding, and a population captured
at resolution are different instructions. They must not be relabeled as one
another or silently defaulted when loading an older definition.

`Restriction::PlayerHexproofFrom(PlayerFilter, ObjectFilter)` is appended to
separate the retained spell/ability controller from the physical source's
current or last-known characteristics. The previous `BeTargetedPlayerFrom`
remains a source-quality restriction for protection. Existing ordinals and
payloads are not repurposed. Recompile old definitions from their original
source so the compiler can express the intended rule; checksum refresh or
version substitution cannot perform that migration.

The compatibility fingerprint is
`e27b521de2a44a2c1c3349a8b8cbf5392da6882c14270112de24faecced0adc9`,
SHA-256 of the exact UTF-8 bytes in
[`protection-targeting-schema.descriptor`](protection-targeting-schema.descriptor),
with no trailing newline. Its parent is published schema
`b3895accd8d36443d5ec72a6985ebf4e96650975eaa53d8581e484040dd312d7`.
This explicit descriptor is not an automatically generated exhaustive Rust
schema. Format 9 and format 10 with the old fingerprint are independently
rejected, including with freshly recomputed checksums.

## Public evidence, peers and replay

The public audit vocabulary changes even without new outer checkpoint fields.
Restricted mana contains typed `PaymentTransaction.on_spend` programs. A
`GrantNextSpellAbilityEffect` can carry a static protection ability through the
native effect/ability encoder into that public evidence. The new protection and
targeting forms therefore require digest 6, as well as artifact 10.

Both transcript and match must use protocol 23 before current-engine replay.
All three live peer admission routes use the same current constant. Protocol 22
is explicitly retained with 14/16/17/18/19/20/21 for signature-only verification;
its original digest 5 and signed bytes are never modified. Passing a replay
callback requires the current protocol even with `requireEngineReplay: false`.
Current replay rejects missing, string-valued and older digest versions before
engine mutation, including on subsequent actions. The checkpoint hash domain
and normalization are unchanged. Old signatures are not regenerated or
reinterpreted by this patch.

Native root and inactive-lane savepoints, branch exchange, and authenticated
full-genesis action replay remain the gameplay recovery mechanisms. Public audit
evidence is redacted and has no gameplay importer. The new native ordered draw
ledger is not a serialized checkpoint field: cloning retains exact chronology;
missing chronology remains unknown, while fresh-game/new-turn creation records
known empty chronology. No integer storage width is changed by that ledger.

## Exact source binding and deferred validation

Compiler-produced population protection starts with `reference_source: None`.
The continuous static-grant owner binds `Some(ObjectId)` for its exact source
incarnation. That owner does not recursively bind a mana on-spend payload, so the
eight reviewed protection bodies do not introduce bound IDs into restricted
mana. Native callers can nevertheless construct such a bound payload directly.
The authored public encoder cases distinguish absent and two exact bindings and
check repeated projection; they do not claim cross-peer allocation independence
for that generic caller-supplied construction. Its raw ID normalization remains
a narrow API question for consolidated validation, not a reason to rewrite
artifact/native exact-incarnation semantics to a stable ID.

The authored tests cover old-format/schema refusal, historical 22/5 signature
verification and tampering, mixed 23/22 replay refusal in both directions, current
and old per-action digest admission, all three peer routes, typed protection
on-spend programs, and refusal of an unencodable program. Full source cohort
scenarios, rollback and merged-owner regressions remain unrun. Regenerate actual
current artifacts/catalogs/text/checksums and digest expectations only after
execution is authorized. The missing historical `fixtures/v5.json` golden
reference remains the previously recorded provisioning prerequisite; no
historical fixture has been invented or relabeled here.
