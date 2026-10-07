# Static linked-exile permissions

Status: source-authored, UNVALIDATED. Based on
0393efafb0851a8c27f7044c4e4f3990a4828cb8. No build, compilation, compiler probe,
engine/corpus execution, test, or formatter was run. No baseline, campaign
ledger, published stack, checkpoint or artifact version was changed.

## Bounded full-body proposal

Only Nightveil Specter is proposed by this packet. Its exact frozen full body
is retained alongside seven deliberately held neighbors in
`fixtures/linked_exile_static_permissions.json.fixture`. The shared defect is
that a static play permission selected the source object's union of exiled
cards, including cards exiled by independently acquired abilities.

The new surface production consumes the complete generic “play lands and cast
spells from among cards exiled with this ...” sentence. It emits a typed static
GrantSpec requiring definition-pair binding. Lowering proves exactly one
face-up exile-top trigger and one complete static whole-pool play permission, with only
unrelated leaf flying/menace abilities permitted around them. Multiple
producers, riders, activated costs, static producers, private inspection,
levels, alternate zones and other unidentified scopes remain unbound. The
existing Bishop executable/scalar binder is unchanged.

The new static scope uses the existing immutable typed-definition SHA-256,
pair descriptor and runtime acquisition namespace. Its producer records the
actual resulting exile ObjectId under the captured owner. Current static
characteristics provide the reader's complete AbilityOrigin. The reader emits
conjunctive per-member grants only for those exact live exile incarnations.
Neither the source-wide union nor stable card identity can supplement those
members. Both existing grant evaluation paths receive the same exact targets.
The current controller owns the static permission; losing/phasing the source
or the ability removes it. Returning a source or victim as a new incarnation
cannot adopt an old member. Normal land limits, spell costs and timing apply.

GrantSpec carries both the typed scope requirement and its optional bound
pair through model mapping, direct materialization and artifact decoding.
The default false scope marker and absent pair are omitted from serialization,
preserving the pre-existing format-6 GrantSpec byte shape and payload checksum
when those keys were absent. A deferred legacy-static artifact scenario checks
that neither key is introduced by typed deserialization/reserialization.
Within this scope, missing pairing, acquisition or imported ownership state
records IncompleteEvidence at the checked legal-action boundary. The existing
producer admission check rejects a missing captured owner before mutation.
Native GameState clones retain pending owner and membership; a source-only
exile-map import is insufficient for these readers.

## Compatibility boundary

The new admission check is gated by the new typed scope requirement or an
explicitly supplied pair. Existing GrantSpec constructors default to their
previous route, and their source-wide behavior is not claimed as repaired.
Paired producers still publish the legacy source-wide links for compatibility.
An independently acquired unmarked legacy reader can therefore see those
members and unrelated links through that old route. A mixed-reader scenario
checks exact permission identities: only the typed identity is pair-restricted,
and removing the legacy reader removes its unrelated-card permission.
In particular, the existing compiled/native paths for Intrepid Paleontologist,
Ian Malcolm, Chaotician; Rona, Disciple of Gix; and Valgavoth, Terror Eater do
not acquire the new requirement and are not newly rejected. No names select
that boundary. The initial broad guard was narrowed before committing.

Relevant old constructor owners are
`permission_helpers.rs::parse_permission_clause_spec` (plural filtered source
spells), `costs_replacements_and_permissions.rs::parse_you_may_static_grant_line`
(the separately recognized active-player/any-mana branch), and the source
exiled life-payment permission production in that same file. Their defaults
remain unchanged. `game_loop/tests/source_linked_exile_permissions.rs` retains
its existing isolated Intrepid permission fixture byte-for-byte; that fixture
is not evidence for the entire card or for exact paired ownership.

Textual inspection of the frozen failed-card ledger and the coordinator's
current ledger found no prior proposed identity newly rejected by this bounded
change. Those four named cards are absent from that failed-card ledger;
Dawnhand Dissident is already partial_not_counted. This is a source-boundary
observation, not an executed regression result.

## Explicit holds

- Bane Alley Broker: paired hand-exile producer, persistent private inspection
  entitlement, and its separate return activation.
- Kheru Mind-Eater: hand selection/exile and private inspection, plus the
  combined static play sentence's shared antecedent.
- Colfenor's Plans: private inspection, seven-card producer, draw-step skip
  and one-spell-per-turn restriction must all survive the complete body.
- Intellect Devourer: opponent-specific simultaneous hand selections,
  source-leaves return records and permission-specific any-color payment.
- Rogue Class: private inspection, level scope and the acquisition relation
  between the base producer and level-three static permission, plus any-color.
- Intet, the Dreamer: optional payment, persistent private inspection and a
  free-play effect whose exact source-incarnation lifetime differs from the
  exiled-card lifetime. This is not the static reader addressed here.
- Elder Brain: attacked-player whole-hand receipt, exact replacement-sensitive
  draw quantity, persistent effect permission and permission-specific mana.

## Deferred scenarios

`compiler-runtime/tests/linked_exile_static_permissions.rs` exercises the full
Nightveil body through independent direct and serialized-artifact paths:
actual combat and damaged-player library ownership; both lands and spells;
normal timing and costs; unrelated source links; current control; phasing;
ability loss/restoration; source/victim blink; copied and pending triggers;
separate effect and borrowed producers; complete acquired pairs versus later
acquisitions; native savepoints; source-only imports; missing owner/pair;
multiple producer rejection; separately scoped same-source replacement exiles;
and failing replacement additions with rollback
and recovery. The exile-top primitive's existing native additional-effect
error/pending/replay scenarios now also assert exact pair membership rollback.
The grammar scenarios reject duration, price, narrower-subject and nonself tails.
All scenarios are authored and unrun. Independent source review and the deferred
complete-card execution pass remain required before claiming correctness.
