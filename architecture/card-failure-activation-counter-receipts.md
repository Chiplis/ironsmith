# Activation counter-payment quantities

Status: independently source-reviewed recovery, **EXECUTION UNVALIDATED**. This recovery is based on `cc915c3fb4ef9162bfda86fbf69c2549c1e37c85`. Fresh independent source review cleared all five bodies at `7292b05bd0d0da7d9dc0450fa2ae7579840c9508`. No builds, compiler probes, tests, formatters or corpus execution were run. Coverage and publication belong to the campaign coordinator.

The frozen `event-derived-amount` root contained fifteen rows, with Aether Revolt handled separately. This coherent five-card subset retains the complete Oracle bodies from `cards-20261003.json.xz` in `fixtures/activation_counter_receipts.json.fixture`:

| Card | Oracle identity | Reconstructed owner |
| --- | --- | --- |
| Jar of Eyeballs | `3075dadd-240f-4455-9286-9f1d48f53a3f` | Actual all-eyeball payment determines the looked pool |
| Rasputin, the Oneiromancer | `c16fd2b9-9147-43bc-aa08-6d1bddf1d3e8` | Actual dream payment drives mana and the fixed Knight cost |
| The Astonishing Ant-Man | `6c7e4e51-099a-4398-a4b7-e2f0ddd36429` | Actual chosen +1/+1 payment determines token count |
| Hankyu | `940909b2-f59e-4f87-a119-675efd60a5bc` | Both grants retain the exact Equipment's aim counters |
| Simic Manipulator | `8f06fcc9-9018-4c55-af63-c44350a6cfeb` | Prospective declaration admits targets; actual payment governs resolution |

## Exact retained producer

`EffectId::ACTIVATION_COUNTER_COST` identifies one counter-removal component in the selected activation. Its semantic import retains counter kind and declaration capability through reference frames, branches, exports and lowering. Multiple producers, alternative branches and opaque compiler effect costs do not acquire an inferred producer. Named counter metrics must match that kind and paid scope in both early and sequence binding owners.

Only activation-cost materialization wraps the retained component with `WithIdEffect`. Body and optional costs cannot overwrite it. The removed legacy quantity-to-X rewrites no longer conflate counter removal with announced mana X. Existing `has_announced_x` and `declared_target_references` contracts remain intact. Resolution recovery excludes this reserved cost ID when looking for absent body producers.

The existing outcome map passes through direct/deferred mana abilities, selected-cost adapters, in-context payment, activation pending state, stack entries and resolution contexts. Cost-side value projections preserve incoming receipts. WithId only publishes completed outcomes; in-context failure or pending execution restores both the game and execution snapshot. Missing reserved receipt values produce `IncompleteEvidence`; a completed zero remains an explicit zero. Numeric evaluation retains the wide i64 and checked unsigned event boundaries, including values above i32::MAX.

## Exact granting Equipment

The shared grammar quote-scope owner carries an attachment grant across coordinated quoted abilities. Both text and token source-name normalization use the same quote scopes and action/counter operand rule. The counter-cost CST preserves the authored all quantity and existing typed GrantingSource reference.

Lowering retains an exact counter target and `CountersOn` amount naming that same object. Native grant materialization binds both to the Equipment incarnation under the retained outcome wrapper. Written counter ownership does not add a controller restriction. The equipped creature owns its tap cost and damage source, including lifelink and its source snapshot. A departed or phased-out granting object cannot pay even an otherwise zero removal.

Hankyu's frozen body uses aim counters. Arrow counters, host counters and another Equipment's counters remain distinct.

## Prospective declaration and actual payment

`CounterRemovalDeclaration` belongs to one pending activation and is separate from X and the paid outcome map. Current source counters of the required kind bound the declaration. The existing ChoosingCostReferences stage asks before target collection. Only announcement targeting views read the declaration; ordinary resolution views require the real payment receipt.

The supported admission contract is one required object with power at most the counter quantity. The largest payable declaration is a finite monotone feasibility witness; each actual target is priced separately. Unsupported equality/other comparisons, multiple target requirements, distributed/non-source declaration costs and ambiguous producers retain their boundary.

The captured original cost survives repricing. After declaration it becomes one fixed requested removal under the same producer, with no second quantity or replacement source choice. Repricing validates that exact requested kind, amount, source and identity. Stale target responses and current source shortages fail transactionally. Pending responses publish no choice or receipt, and duplicate numeric answers cannot overwrite the declaration or mana X.

[CR 118.11](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf) permits replacement effects to modify a cost action. Completion therefore accepts the actual original-instruction removal result, including prevented/replaced zero, without requiring it to equal the declaration. Resolution rechecks target power against the actual paid result and can fizzle. The actual result survives source departure and native snapshots.

## Authored scenarios

The runtime suite independently invokes strict `compile_to_runtime_definition` and strict artifact compilation with separate parse-loss capture, then JSON-decodes and rematerializes the artifact. Native snapshots are exercised at real announcement/payment boundaries.

- Ant-Man: draws add counters; real mana/tap/zero-or-positive removal; exact Insects; source changes and departure.
- Jar: controlled versus foreign deaths; zero and four-counter payment; exact looked pool, chosen Hand successor identity and bottom ordering of unchosen cards; unlooked Library identities/order preserved.
- Rasputin: entry for both opponents; each Goblin; one-or-more dream mana payment; white 2/2 Knight and protection from red; foreign counters cannot pay source costs.
- Hankyu: real equip of two copies; both grants; exact aim versus arrow/host/other-Equipment identity; foreign Equipment controller; zero; native recovery; departure, reattachment, target fizzle and recipient lifelink.
- Simic: full Evolve trigger and intervening recheck; declaration before target menu; no duplicate quantity; counter changes before payment; current-target recheck, cancellation, invalid/pending/duplicate answers; actual-result fizzle and source departure; independent mana X and target-dependent price.
- Replacement-modified Simic costs: smaller/larger count, prevention, instead action, and a zero-power target with actual zero payment.
- Native payment owners: completed zero/wide counts, incoming receipt projections, later life payment, failed/pending suffix rollback after an observed paid prefix, and independent mana X larger than the available counters.

All scenarios are authored and unrun. This file does not claim execution recovery or update the campaign matrix.
