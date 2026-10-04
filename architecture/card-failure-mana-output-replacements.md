# Mana-output replacement root: unvalidated source proposal

Eleven exact frozen candidates are retained in the fixture: Contamination,
Harvest Mage, Infernal Darkness, Naked Singularity, Pale Moon, Pulse of Llanowar,
Quarum Trench Gnomes, Reality Twist, Ritual of Subdual, Hall of Gemstone and
False Dawn. All eleven have typed source proposals and full-body authored gates;
independent review and all runtime validation remain pending. No measured recovery
is claimed.

The implementation must distinguish replacing types while preserving quantity
from replacing type and quantity. Quarum changes white symbols only; Hall changes
colored symbols only and captures the resolved color for each turn-duration
registration; False Dawn is scoped to the controller of the mana-producing spell
or ability. Current source characteristics and recorded production snapshots
remain separate from whether a static replacement host is active.

The mana-production event, not the tap event or the printed mana template, owns
the rewrite. CR 106.12b, 614.5 and 616.1 govern production-time replacement,
one application per occurrence, and affected-player replacement ordering. Source:
[official comprehensive rules, effective 2026-09-25](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt).

Pulse and Harvest require a separate color decision by their instruction's
controller after the affected player chooses replacement ordering. Planner
branches and replay witnesses must retain that distinction and validate exact
choices; a payment planner cannot silently choose for another player. Source
restrictions, real mana receipts, typed incomplete queries, and transaction
rollback remain authoritative.

Authored validation will cover strict direct/artifact bodies, ordinary output,
planner reachability and actual paid execution, preserved versus fixed amounts,
colored/white-only rewrites, source phase/control/leave, exact target incarnation,
expiry, replacement ordering with multipliers, color-choice ownership, snapshots,
and pending/error rollback. All compilation, builds and execution remain deferred.

Chaos Moon's parity/count and delayed mana grant are outside this cohort. Ordinary
mana triggers, Piracy's foreign activation permission, and unspent-mana retention
are separate roots. No card-name dispatch or marker-only acceptance is permitted.

## Additional boundaries closed by this source patch

- Naked Singularity/Reality Twist use one basic-land output table per static
  occurrence. A dual-typed land selects one eligible color for the entire event.
- Replacement color selection follows ordering and keeps its actual chooser.
  Compact witnesses retain the selected color and reject stale/unavailable
  outputs. Old no-color payment hashes stay unchanged. Another player's needed
  choice is unknown to a compact payer plan; native recording preserves the
  existing checked incomplete-query boundary.
- False Dawn's independent spending instruction uses the existing real policy
  tracker. Its new explicit cleanup lifetime does not reinterpret legacy
  next-turn-bound permissions. White can pay colored costs, not colorless costs,
  and other mana retains its normal spending rules.
- Effect-granted spending permissions have no lossless checkpoint carrier.
  Export refuses them and import requires an explicit empty-state carrier;
  existing runtime savepoints and full authenticated genesis replay remain the
  recovery routes. A separate standalone-permission negative prevents the mana
  replacement guard from masking this omission. No peer assertion proves absence.
- Source filters use recorded production LKI after departure or phase-out, while
  live static hosts are still required to be active. Registered targets capture
  one incarnation and chosen-color registrations capture the resolved color.

The runtime fixture now authors actual activation costs, target announcements,
planner/paid-execution paths, normal color decisions, pending rollback, direct and
artifact bodies, upkeep secondary programs, source departure, and cleanup. Core
schema and grammar negatives plus native witness tests are also authored. Every
check is UNRUN; only source review, frozen-data selection and whitespace checking
were performed.
