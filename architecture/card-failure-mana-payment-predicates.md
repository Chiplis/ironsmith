# Disjoint restricted-mana payment predicates

Status: **UNVALIDATED** source implementation and authored regressions. No build,
compiler probe, test or corpus replay was executed.

Four exact frozen whole-card proposals: Cultivator Drone, Discreet Retreat,
Overgrown Zealot and Tin Street Gossip. The fixture retains full metadata, Oracle
identities and the original dropped-spending-restriction diagnostic. The
remaining six members of that diagnostic family need actual transaction-method
or activated-ability-kind distinctions (equip, power-up, foretell, disturb,
manifest/morph/disguise); they are not silently generalized here.

## Shared typed shapes

The complete clause reader separates alternative **actions**, not arbitrary
occurrences of “or” inside a selector. Cast, activate, cost-symbol and face-up
branches lower to existing `ManaPaymentPredicate` combinations. Unknown tails
reject the entire new shape. The ordinary simple cast/activation readers retain
their established representations.

- Cultivator's first arm tests the spell's color; its second tests a battlefield
  permanent's color and activation purpose. Its third is a complete cost
  containing a true colorless symbol. This can pay a generic part of such a cost;
  it is not mistakenly a restriction to the colorless pip alone.
- Face-down spell casting and turning a permanent/creature face up remain
  independent transaction purposes, with current object state and zone filters.
  Face-up creature activation is not an allowed substitute.
- Outlaw source filters retain the inclusive five-subtype set. The Aura's
  granted mana ability uses the same real runtime payment pipeline.
- Rendering describes every admitted predicate and every branch. It never
  preserves an Oracle sentence as a substitute for executable restrictions.

## Production-time context

Restricted units now retain an optional producing ability/effect controller.
Native mana credits use the explicit ManaAddedEvent controller, which is distinct
from the receiving player. Projection and actual pool credit share that field;
activation preview initializes the same activator context, even when the physical
source has a different controller. Payment-source filter reads no
longer require the original mana source to remain in its zone, nor reinterpret
“You” after that source changes controller. Old units/checkpoints without the
new optional field retain the legacy fallback. WASM checkpoint conversion carries
it through both directions, and existing explicit literal fixtures acknowledge
that compatibility default.

The source object identity still means the original source; this change grants
no permission to follow a new blink incarnation. Existing snow provenance,
retention, chosen-creature type, payloads and payment-assignment rules remain.

## Authored deferred tests

- Whole-card strict metadata compilation with the required spending marker;
  direct and serialized artifact materialization for all four identities.
- Real mana activations and real colorless payment; each allowed and forbidden
  transaction arm, including floating mana after the source leaves.
- Separate face-down casting, turn-face-up and ordinary activation purposes.
- Actual Aura attachment and granted mana activation; all five Outlaw subtype
  arms and Human negatives, after Aura and mana-source departure.
- Production-controller versus recipient split, source theft and departure,
  agreeing between projected credits and authoritative pool assignment.
- Whole-clause malformed-tail grammar negatives.
- WASM unit-wire round trip for frozen controller and compatibility with an old
  unit lacking the optional field.

Deferred runtime target: `mana_payment_predicate_restrictions`; tools aggregate
has the same name. All scenarios remain authored, not passing claims.
