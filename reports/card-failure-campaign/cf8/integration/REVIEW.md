# CF8 integration review — 2026-10-08

All twelve package heads are merged into `cf8/integration`. The main checkout is untouched. `merged-heads.json` records the exact inputs. Source proposals are not evidence of supported cards.

## Handoff mechanisms and semantic integration

- p01: the checkpoint warning describes a retired wire codec. Current runtime savepoints clone the complete GameState, including followed replacement objects. A regression restores both before casting and after casting before checking the eventual replacement. Public claim snapshots are not restoration checkpoints.
- p02: removed the Siren's Call-shaped token rewrite. Actual `Ignore this effect for each <filter>` tokens narrow a supported mass instruction through the existing exception machinery. A preceding typed controller antecedent resolves "that player"; the parser uses the full delayed-instruction grammar.
- p08/p07: exact-amount and half-damage prevention share NextTimeDamagePreventionPortion. The payment publishes its own amount. A typed independent-X flag distinguishes "pay any amount" from paying a spell's printed X. Liege's player-loop protocol collects payments before a simultaneous token batch; added runtime evidence checks every player's token count and batch identity.
- p09/p12: finished the uncommitted source-quality and delayed-death readers, then added full-card direct/artifact regressions. p06 and p11 had committed their latest source edits; no abandoned dirty source remained in their worktrees.
- Duplicate retarget restrictions use one Player/Object model. Damage multiplier grammar and replacement registration retain all merged amount and recipient fields. Frozen object identities preserve other filter restrictions rather than replacing the entire filter.
- Null Chamber now lets its controller choose the opponent who names the second card in multiplayer, rather than automatically selecting the next opponent.
- Cycling self-triggers function from their pre-discard hand snapshot, even when discard is replaced by exile or a library move. The other-card battlefield arm remains gated separately. This implements the hand-origin rule without checking the current graveyard/exile location.
- Artist's Talent's regression compiles a real source artifact rather than depending on an absent generated frontend JSON file.

## Compatibility

Artifact format advances once, from 17 to 18. `architecture/cf8-integrated-schema.descriptor` fingerprints the integrated wire models. Previous descriptors retain their original bytes. Format 17 and its schema are explicitly rejected. The current golden is regenerated from the current library fixture. The real historical v3 fixture remains untouched; the previously referenced v5 fixture is unavailable in repository history, so historical-byte coverage uses v3 instead of fabricating a v5 file.

## Source-ledger tally and validation decision

The latest package ledgers contain 721 source-proposed rows, plus 19 dependant-proposed and 11 collateral rows. Deduplicating these statuses yields 746 proposed/collateral cards. There are 1,934 distinct ledger oracle IDs and 66 cards with differing statuses across packages; these are preserved in `ledger-tally.json`, not resolved by blindly trusting the optimistic status. Package summaries have stale round counts; latest ledger rows are authoritative only for source work.

This is enough source coverage to justify full corpus validation before landing. A compiler acceptance result does not prove runtime correctness. Validation results and remaining failures will be recorded below before integration is finalized.

## Deliberately unsupported

Stromgald Spy's persistent public-hand permission needs a coherent hidden-information protocol. That mechanism is not implemented or silently marked supported in this integration. Other blocked/dependant-blocked ledger entries remain explicitly unsupported pending independent mechanisms.

## Validation

The requested workspace check has completed twice successfully; subsequent fixes still require a final repeat. Focused tests and package-referenced runtime targets are in progress with 64 MiB test stacks (the corpus compiler's worker-stack size). Initial 16 MiB test stack overflows are distinguished from parser failures.

Activation-time values now have a generic announcement sample carried in the resolution program and its execution context. A runtime regression verifies the value survives sacrifice costs, responses, and checkpoint cloning. Exact prevention portions now participate in the same prior-result detection as other amount consumers.
