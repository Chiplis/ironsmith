# Compound static bodies: source checkpoint

Base: `6b746fb1885a14289eedc510f3eaaff051fe7603`. This checkpoint preserves the base's Bedlam, Orb of Dreams, Noble's Purse, and Sphere of the Suns work; none is new coverage here.

No compiler, build, test, formatter, probe, or corpus execution was performed. The evidence below is source inspection plus newly authored, unexecuted scenarios. It does not establish compile recovery or runtime correctness. The coverage matrix is unchanged.

## Complete-body candidates for independent source review

- **Machinist's Arsenal:** The complete stat/type-addition owner retains the `+2/+2` artifact multiplier, the Equipment controller's count, and the Artificer type addition. The plain anthem reader yields only when that full owner succeeds. The raw full fixture includes Job select and the labeled equip ability; native scenarios exercise actual entry token creation/attachment and the four-mana equip activation, dynamic count changes, controller ownership, attachment movement, and source removal.
- **Spire Serpent:** The hypothetical `have defender` belongs to the attack-permission grammar. The competing anthem/keyword reader declines it. Both stat bonuses and the permission retain the metalcraft condition; defender itself remains. Native scenarios cross the controller-scoped artifact threshold in both directions.
- **Tek:** Comma-separated omitted-subject stat and keyword predicates retain five independent conditions. Plain-anthem and broad-grant readers cannot consume subsequent predicates as part of an earlier condition. Native scenarios add every basic land type independently, change land control, remove a land, and change Tek's controller.
- **Nighthowler:** The composed reader's where-X route returns the same two typed anthems as the distributive-subject owner. The single-subject reader declines a noncanonical subject conjunction. Native scenarios retain Bestow's actual cost/target/casting path, count creature cards across three graveyards, test attached Aura and detached creature forms, and check additive counters.
- **Expedition Lookout:** A named complete conditional attack-permission-plus-unblockable owner retains both predicates. The condition is the existing typed single-opponent graveyard threshold, so opponents' graveyards are not summed. Native scenarios verify both combat predicates below/at/below the threshold, including own-graveyard and split-opponent negative controls.
- **Wrecking Ball Arm:** A named complete base-P/T-plus-blocker-restriction owner uses the existing complete blocking grammar, retaining a typed blocker power comparator and the Equipment's live attached recipient. The base setting is a layer-7b modification; the unquoted blocking rule remains on the Equipment instead of becoming a removable recipient ability. Native scenarios cover both printed equip prices, legendary target restrictions, counters, current blocker power, recipient ability loss, attachment changes, phasing, and source departure.

All six candidates are represented by complete frozen raw bodies, a strict direct runtime route, an independently compiled serialized/restored artifact route, and authored native behavior scenarios in `crates/ironsmith-compiler-runtime/tests/compound_static_bodies.rs`. Parser-level scenarios in `crates/ironsmith-compiler-grammar/src/keyword_static/compound_static_body_tests.rs` cover registry ambiguity, exact predicate count, typed blocking ownership, and rejected unknown tails.

## Explicit partial

**Rope:** The stat/reach/maximum-blocker compound's ambiguity is repaired by preventing the subject/keyword-only reader from treating a preceding stat predicate as part of its subject. Full raw-body scenarios retain both real equip activation and sacrifice/draw. However, the existing maximum-blocker rule is still represented as a granted ability and queried from the recipient's calculated static abilities. Recipient ability removal would incorrectly erase this unquoted rule. The unignored `rope_unquoted_blocker_limit_must_survive_recipient_ability_loss` scenario records the required behavior. Rope is not a complete-body source candidate until the maximum-blocker rule layer is corrected and independently reviewed.

## Other assigned candidates

Avatar Destiny benefits incidentally from the generic stat/type-addition ownership repair, but its death trigger's power reference, mill, source return, and selected milled-creature return have not been established here; it remains partial. Collective Inferno, Displaced Dinosaurs, Lavabrink Venturer, and Aminatou, Veil Piercer are untouched and remain unaddressed by this checkpoint.

Existing Job select, Bestow, defender/attack-permission, equipment targeting, attachment, dynamic-count, and generic restriction owners were inspected; this checkpoint adds no effect/reference discriminants and does not change registry ambiguity policy or select an arbitrary winner.
