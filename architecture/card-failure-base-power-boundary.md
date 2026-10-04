# Retained base-power/toughness boundary

Status: **UNVALIDATED**. Source and authored tests only; no compilation or execution.

This is a prerequisite for Sovereign Okinec Ahau, not a full-card coverage claim yet. The remaining compiler work is an explicit BasePowerOf value and candidate-relative current-versus-base difference.

## Runtime correction

CalculatedCharacteristics now retains base power/toughness after layers 7a and 7b. All three evaluators seed from the post-copy characteristic view and update this metadata after characteristic-defining or setting effects, including timestamped level P/T. Modifiers, counters, and switching only change effective P/T. Existing cached characteristics carry the boundary, and calculated departure snapshots copy it into their existing base fields. No serialized enum variants or snapshot fields were added in this prerequisite.

Base-P/T object filters consume calculated metadata. Snapshot filters no longer try to recover base P/T by subtracting counters from final P/T, which cannot undo an anthem or a switch. The ordinary raw fallback remains for genuinely uncalculated views.

One authored engine regression compares batch, direct, and legacy evaluator results with a life-dependent characteristic-defining setting, an optional later 7b setting, a pump, a counter, and a 7d switch. Public life and counter actions exercise invalidation, followed by object/snapshot filtering and actual departure LKI. It is unrun.

Rules basis: [official Lost Caverns of Ixalan release notes](https://media.wizards.com/2023/downloads/LCI_Release_Notes_kUj28nYwbydD/EN_MTGLCI_ReleaseNotes_20231107.pdf), Sovereign Okinec Ahau. Characteristic-defining values and later setting effects determine base P/T; modifiers do not.

## Explicit numeric value prerequisite

BasePowerOf(ChooseSpec) is appended to the serialized Value enum. The ordinary execution/continuous number adapter reads calculated 7b metadata or exact departure snapshots. Source-possessive quantity grammar recognizes the explicit base-power axis. Target/reference walkers, dynamic filter RHS evaluation, dependency reads and text rendering carry the distinction. Two direct/artifact runtime scenarios are authored for live source settings/modifiers and exact source/tagged departure after blink; they are unrun. This still makes no Sovereign full-card claim: its derived amount and grouped per-object counter allocation remain to be connected.

## Curie full-card source proposal

Curie, Emergent Intelligence is the second exact frozen base-power failure, separate from the reserved seven comparison cards. `its base power` now preserves the ordinary prior/event object reference in BasePowerOf. Its activated exile cost and quoted copy exception use real existing payment/copy execution, with two bounded corrections:

- A non-targeted, exactly tagged copy source may use its departure copiable values from exile (or another recorded zone), not only from the battlefield. The lookup uses the original ObjectId and expected zone, never a stable-card follow-through. This also covers a constrained single tagged object reference.
- CopyOfWithAbilities is appended to the compiler, wire and runtime modification enums. The compiler emits this typed payload for authored ability exceptions. Materialization recursively converts its executable abilities; runtime inserts them into the layer-1 copiable values, and owned-effect traversal retains their programs. Text rendering preserves copy-exception wording. Ordinary resolving ability grants stay separate layer-6 modifications.

Exact fixture: `fixtures/copied_base_power_draw.json.fixture`. Five compiler-runtime tests are authored, direct and JSON-materialized: original combat-only draw amount; real mana/exile cost and exact exiled-object LKI; no source blink-follow; copy-of-copy under layer-6 ability removal, with an ordinary flying grant deliberately not inherited; and typed exception payload inspection. Normal tools fixture is authored. No builds, tests or compiler invocations ran. This proposes Curie complete, while Sovereign remains partial at grouped counter-transaction semantics.

## Sovereign pending counter transaction

A generic object loop currently reevaluates amounts and counter replacements as earlier iterations mutate state. Existing multi-object counter instructions share notification batch IDs but process replacement choices/conditions and commit each placement sequentially. A full Sovereign closure must freeze the qualifying set and all per-object current-minus-base amounts before placements, then prepare replacements against a common pre-event counter state while preserving shared consumption and replacement-added programs. It must commit actual results as one grouped instruction, with atomic suspension/retry, overflow checks, and no intermediate observer state. Merely sharing batch markers is insufficient. No Sovereign grammar workaround or success claim is included.
