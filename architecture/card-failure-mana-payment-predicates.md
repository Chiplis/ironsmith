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


## Reconstructed seven-card packet (2026-10-06)

Status: **UNVALIDATED source reconstruction** against published
69a946ec767deda59927d63f08dd17fabff470f3. The previous unpublished filesystem was
lost. These edits do not claim retention of its commit hashes. No build,
compilation, test, formatter, runtime probe, or corpus replay has run.

The exact frozen complete bodies are Qarsi Deceiver, Karfell Harbinger,
Unblinking Observer, Freya Crescent, Quinjet Technician, Ronin, Shadow Stalker,
and Niko Defies Destiny. The first six extend the existing fixture; Niko retains
its complete existing foretell-state fixture row. Central coverage accounting
is deliberately unchanged pending independent review.

### Exact payment evidence

The shared typed model distinguishes the Foretell special action from a later
cast and distinguishes the chosen Morph, Megamorph, Disguise and printed-mana
turn-up methods. Broad turn-face-up restrictions continue matching each method.
Qarsi admits Morph/Megamorph or the printed-cost method of the exact currently
manifested incarnation. Cloak has separate origin evidence and retains ordinary
printed-cost turn-up eligibility without qualifying for the Manifest predicate.

All three native Manifest/Cloak entry paths record the exact operation. Turning
face up, leaving the battlefield, or adopting face-up merged status clears both
origin sets. Phasing retains history but does not make a phased-out object a
legal payment target. Neither current Ward nor a later stable-card incarnation
reconstructs origin. Restricted mana retains its existing production controller
and remains usable under the same predicate after its producing source leaves.

Equip and Power-up now carry typed activated keyword identity across grammar,
generic AST maps, modal headers, lowered/native constructors, generated Equipment
tokens, continuous grants and artifact materialization. Attachment effects and
presentation labels are not keyword evidence. Keyword-aware payment reasons are
captured at announcement and preserved by checked admission, alternate/reference
cost queries, native payment, nested mana activation and WASM affordability.
Broad activation-purpose and life-payment rules still admit typed activations;
mana-ability self-funding exclusions remain in force.

Observer's Disturb arm reads the selected Disturb alternative on the proposed
stack object, not possession of the keyword. Its instant/sorcery arm is separate.
Karfell's Foretell action is similarly separate from its instant/sorcery arm.

### Selected casting proposals

The common payment owner projects the selected face, alternative price,
Prototype mana cost/P/T, caster, face-down status and origin snapshot into an
isolated stack proposal. Explicit Harmonize requests use that same owner while
retaining reserved tap resources. Continuous refresh errors remain typed
incomplete calculations rather than negative affordability answers.

A face-up cast from concealed exile clears the old zone's face-down state in the
isolated proposal before a declared face-down method reapplies its state. This
does not execute a turn-face-up action or publish a reveal. The real origin is
unchanged. The origin snapshot tests an authored origin zone only; it cannot
supply a printed color, type or keyword that the selected spell face lacks.

Resumable cache adoption requires the supplied Object to be the exact immutable
root borrow. This is an admission proof; addresses never enter the semantic key.
Admitted queries key the typed method and optional-cost declaration as well as
the complete payment request. Arbitrary supplied clones/provisional views stay
on the existing exact synchronous hypothetical path, so the same ID/method/pips
cannot alias different spell characteristics. Direct/sliced menu scenarios assert
equal answers, not incremental work for every hypothetical view.

Niko's capability filter is distinct from prior Foretell designation. A face-down
object does not have its physical card's Foretell capability, despite retaining
alternative methods for later reveal. Face-up ordinary and later Foretell casts
can qualify. Its first and third chapters retain the ordinary counted-life,
exact-owner/zone predicate, targeted move and Saga chapter/lifetime owners.

### Compatibility and authored scenarios

Executable card artifacts now require **format 6**. Plain core JSON may default
an absent optional keyword field to None, but old format-5 executable artifacts
are rejected and must be regenerated; otherwise old Equip modifiers would
silently change behavior. No label/effect migration guesses the missing identity.

The separate public audit boundary is **checkpoint 3 / signed protocol 19**.
Historical checkpoint payload hashing remains unchanged and supported historical
signatures can be checked without current-engine replay. See the accompanying
Manifest/Cloak audit compatibility document for exact replay/session guards.

Authored, unrun scenarios cover direct/artifact whole bodies; actual Manifest
and Cloak production and native turn-up methods; Qarsi face-down casting;
Karfell Adventure casting versus the creature front; Observer Disturb versus
ordinary possession of Disturb; selected Equipment casting; Equip versus an
ordinary attachment activation; generated/granted keyword identity; Quinjet's
restricted and unrestricted abilities; Freya's flying across turns; Ronin's
life/once-turn ability and attached-Equipment sacrifice/sorcery timing/-4/-4;
Prototype and Harmonize controls; exact-origin reset after blink; same-ID
different-view cache negatives; Niko's complete Saga and face-down capability;
and wire/version compatibility. None is a passing-test claim.
