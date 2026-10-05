# Resolution-time dynamic base characteristics

Status: **UNVALIDATED source proposal; no builds or tests executed.**

## Nine proposed full-card identities

Belligerent Yearling; Eldrazi Mimic; Exuberant Wolfbear; Riptide Mangler;
Sentinel; Sita Varma, Masked Racer; Wall of Tombstones; PuPu UFO; Shape Stealer.

All nine are frozen baseline failures whose base-characteristic assignment
reaches an unsupported `become` clause. Exact full Oracle text, reminders and
metadata are retained in `fixtures/dynamic_base_pt_quantities.json.fixture`.
Expected transition: failure -> strict compiled, non-lossy. This is not a measured
result and does not imply coverage of the other animation or copy failures.

## Shared correction

The become grammar now recognizes an explicitly named base **power**, base
**toughness**, or base **power and toughness** destination. Both possessive and
postnominal (`the base ... of ...`) subject forms retain the real destination.
This branch is separate from the leading P/T animation descriptor/grant branch.

The right-hand quantity remains a typed Value. A paired reference supplies power
and toughness independently. A single scalar assigned to both axes supplies that
same scalar to both, as on Sita Varma. Existing arithmetic/count parsers retain
controller, zone and sign. Unconsumed operands/durations are rejected.

A numeric reference such as `target creature's power` declares a real target,
then binds the value to that target's exact tagged identity. A numeric reference
is not mistaken for the destination of the base-stat change. The ordinary target
pipeline still prevents an illegal targeted object from supplying LKI. In contrast,
a non-targeted triggering object/source may supply exact departure LKI after it
leaves; a later incarnation of that card cannot replace the old object.

Lowering now binds value references from the incoming reference environment
before the destination advances object memory. It uses the existing SetBasePower,
SetBaseToughness and SetBasePowerToughness actions and the existing
`resolve_set_pt_values_at_resolution` mechanism. No runtime marker, new serialized
variant, dynamic layer reevaluation or continuous-P/T copy is introduced.

The interpreter evaluates both values before installing the one paired setting
modification. Thus a copied source's later pump, control/zone change, or a later
change to a counted zone cannot change the fixed assignment. Only the selected
axis is replaced on single-axis instructions. Existing counters/pumps remain in
their proper later P/T layers. Mass effects lock their affected object set at
resolution, and their supplied Until duration controls expiration.

## Authored unrun coverage

- Normal tools `dynamic_base_pt_quantities`: all nine exact metadata-bearing cards.
- Normal compiler-runtime target of the same name: eleven tests, both direct and
  JSON-transported artifacts; real entry effects, attacker/blocker declarations,
  mana/X/exhaust and tap activation costs, counters/pumps, public zone/control
  changes, LKI versus forbidden blink-follow, optional decline, illegal numeric
  target, signed power, layer preservation, frozen mass membership and expiration.
- Grammar tests for axes, postnominal/possessive scopes, paired versus single values,
  separate numeric target declarations and rejection of incomplete quantities.
- Lowering test confirms the incoming antecedent remains in each base-stat value
  instead of silently becoming the destination tag.

## Explicit partials

- Halfdane needs **end** of the next upkeep, whereas the existing YourNextUpkeep
  duration expires at its beginning. This patch must not count Halfdane or silently
  approximate that duration. Its numeric target/pair form is representable.
- Woodcaller Automaton also requires its Treefolk creature/haste/still-land animation
  descriptor/grant composition. Its dynamic paired RHS is reserved for this lane,
  but it is not included in the nine.
- Behind the Mask remains a separate conditional/self-replacement proposal. Its
  collected-evidence branch must replace only the P/T while preserving the
  artifact-creature transformation; ordinary 4/3 animation alone is insufficient.
- Trench Gorger, sticker aggregates and copy-exception abilities are not counted.
