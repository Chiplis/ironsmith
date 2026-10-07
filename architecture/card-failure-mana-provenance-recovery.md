# Mana-unit checkpoint boundary

Current-owner note (2026-10-06): the serialized gameplay checkpoint APIs described
below were removed by `2511818a28ddb00d7ec96e85bf345eb170b88fdb`. This report is
historical. Use [current recovery and validation boundaries](card-failure-current-recovery-boundaries.md)
for native savepoints, transcript replay and the distinct public audit version 3.

Status: source correction; all authored scenarios are UNRUN. No build, compilation, test or parser probe was executed.

## Concrete omission

`SyncCheckpoint` carries six aggregate pool counts and restricted-mana rules. It carries neither `ManaSourceProvenance.snapshot` nor per-unit retention duration; import recreates restricted provenance with no production snapshot and never recreates unrestricted provenance. Resetting ordinary turn history does not remove a retained mana unit, so a generic-history guard alone does not close this boundary.

The public `Player::has_runtime_mana_provenance` predicate exposes only whether an exact native carrier is needed. It does not disclose any producer identity, snapshot, hidden characteristic or duration. Wire export rejects any such player, including malformed unpaired restrictions, and requires the existing exact native RuntimeSavepoint/local-analysis or accepted signed-transcript replay path. Public audit projections remain projections, not importable evidence of omitted state.

Import requires an explicit `mana_provenance_empty: true` completeness field before mutation, rejects false/legacy omission, and also rejects a contradictory nonempty restricted-unit carrier. This is a format completeness contract, not authentication: peer recovery still replays the complete accepted transcript from accepted genesis, preserving material/opening verification and same-attempt disclosure commitments. A host assertion or public hash does not authorize skipping that replay.

## Exact known affected proposals (non-exhaustive)

- Imperiosaur, Myr Superion, Security Rhox (`consumer-mana-spending`): source-qualified payment requires the production snapshot. The snapshot cannot be reconstructed from a producer's current characteristics, later incarnation or an empty history after a turn reset.
- Ashling, Flame Dancer (`source-context-and-ward`): the live red-mana retention rule can keep real produced units across the history reset. Its retained producer evidence remains relevant to source-qualified and Snow payment.
- Glittering Frost (`negative-characteristics`): its authored production-time Snow scenario relies on the retained snapshot after the Aura leaves. A count-only checkpoint loses that Snow evidence.

These names identify concrete evidence owners, not an exhaustive list of every card that can spend or change the source of mana. Arcum's Weathervane, Melting and Thermal Flux can likewise change Snow characteristics after a source produces mana; their Snow-mutating bodies are already proposed under `negative-characteristics`. All real produced units now fail closed irrespective of card identity.

## Authored gates

New Wasm scenarios retain produced mana through ordinary history reset, reject wire capture, preserve it in an exact native savepoint and reject missing/false completeness before changing a receiving world. The older Cavern test still verifies the empty-pool wire transfer of the chosen creature type; its floated restricted-unit half now uses native recovery, with wire rejection asserted. Existing signed-genesis replay and local-analysis fallback owners are unchanged. No new carrier serializes hidden metadata.
