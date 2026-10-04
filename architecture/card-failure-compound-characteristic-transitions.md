# Compound characteristic transitions

Status: UNVALIDATED source proposal. No compiler, build, test or corpus probe was
run. Eight complete exact frozen sources have direct/serialized-artifact and
normal runtime scenarios authored. These are source estimates only.

## Reusable boundaries

- An object-template reader owns a complete quotation-aware descriptor: explicit
  card types, creature subtypes, added supertypes, stated color, optional base P/T,
  literal name and all granted abilities. It preserves absent characteristics.
  `and has base power and toughness` is the same typed size tail as `with base
  power and toughness`, without manufacturing a Creature card type.
- Subtype-only templates keep existing card types. The lowering owner no longer
  adds Creature solely because no card type was written. Color/supertype-only
  templates likewise leave card types alone. Basic-land conversions remain in
  their separate rules-bearing owner; this patch does not reinterpret them as
  ordinary creature animation.
- `becomes ..., gets ..., and gains ...` has three complete shared-subject
  children with one duration. The become clause stops before the get verb. The
  first targeted child declares the target; pump and grants use its exact alias.
  A malformed pump cannot disappear while the transformation and grant survive.
- `lose all other abilities` on a quoted template clears old abilities before
  installing the stated grants. Quoted internal tails cannot be misclassified
  as outer retention/removal. Ordinary grants remain layer6 changes, not layer1
  copiable exceptions.
- A trailing `and only once` activation restriction is a conjunction with the
  full preceding condition and is an object-lifetime count, not once each turn.

## Eight complete source candidates

Defiling Tears; Dragonsoul Knight; Paragon of the Amesha; Kellan, Planar
Trailblazer; Origin of Spider-Man; Possessed Goat; Surge Engine; Kitesail
Larcenist. Oracle IDs and complete metadata are in the exact fixture.

Authored scenarios cover real colored-mana/discard payments, one target across
all modifiers, actual regeneration, cleanup of every temporary characteristic,
subtype-only retention of Artifact, Kellan's retained combat trigger and expiring
exile-play permission, all Saga chapters, lifetime limits across turns and blink,
Surge's Defender/color gate and actual draw, and Kitesail's per-player target set,
real quoted mana ability and exact-source duration. Full strict tools payloads
and JSON artifact round trips remain unrun.

The Irencrag is partial: its named Equipment descriptor is represented, but the
following optional result/grant/lose-other-abilities composition still requires
complete-body review. Thermal Flux is partial: Snow addition is represented, but
its negative-supertype sibling and deferred draw body are not claimed here.
Other land/color-choice and negative-card-type surfaces remain separate work.

The silent token cap and manager checkpoint recovery dependencies used by Origin
and Defiling are source-addressed in the cumulative campaign; their runtime gates
remain unexecuted. Supported-card regression replay must check new stricter
complete-descriptor rejection and subtype-only card-type preservation.
