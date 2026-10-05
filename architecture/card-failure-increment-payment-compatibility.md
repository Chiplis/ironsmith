# Increment caster-payment evidence compatibility

Status: **UNVALIDATED** source review and authored regressions only. No builds,
compilation, tests, or compiler probes were run.

Increment needs the amount of mana the caster actually spent. The existing
`mana_spent_to_cast` pool includes Assist contributions from other players and
retains its original meaning. New casts capture `caster_mana_spent_to_cast` as
`Some(amount)`, including `Some(0)` for a zero-mana payment. Copies and unavailable
historical evidence retain `None`; neither the total nor mana value can substitute
for this missing fact.

## Two independent wire boundaries

Compiled card artifacts use format version 5 and `ENGINE_SCHEMA_HASH`
(`ironsmith-compiled-artifact/src/lib.rs`). Those checks do not version runtime
saved-game retained payloads. The caster-specific value variant is appended to
the core value enum, preserving existing variant ordinals.

WASM `SyncCheckpoint` uses version 2 and includes `RetainedOccurrenceLiveObject`
through its executable-state graph. Its version check rejects version 1 for a
separate unrecoverable initial-controller ambiguity. Version 2 also deliberately
accepts selected additive legacy defaults; it has no separate retained-payload
schema hash or blanket policy rejecting all older version-2 saves.

The retained cast-payment and historical-snapshot carriers require their prior
fields, including explicit nulls for prior optional facts. Making this newly
introduced optional fact required would implicitly reject old version-2 saves
without a checkpoint-version migration. Therefore only the new caster-payment
field accepts an absent key and deserializes it as `None`. The public snapshot
already uses this same default. Existing strict-field checks remain in force for
all prior fields.

Importing missing payer evidence does not reconstruct a correct historical
Increment comparison. The caster-specific value evaluator reports an
`UnresolvableValue` error for unknown evidence, including when an event snapshot
exists but lacks the field; it never falls back to the Assist-inclusive total or
later live-object data. Existing total-payment consumers remain usable. This is
an explicitly unavailable fact, not an inferred zero or a claim that old saves
can reproduce rules decisions needing evidence they never captured.

Authored regressions cover absent and explicit-null new fields, unchanged total
payment, strict rejection of missing prior fields, independent caster/total
round trips, and the evaluator's unknown-evidence error. They remain unrun.
