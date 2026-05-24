# Naked Singularity - reusable runtime/parser gap

## Card
- Name: `Naked Singularity`
- Oracle line that fails strict parsing:
  - `If tapped for mana, Plains produce {R}, Islands produce {G}, Swamps produce {W}, Mountains produce {U}, and Forests produce {B} instead of any other type.`

## Current failure
- `cargo run -p ironsmith-tools --bin compile_oracle_text -- --name "Naked Singularity" --compare-text` fails with:
  - `could not find verb in effect clause (clause: 'plains produce r islands produce g swamps produce w mountains produce u and forests produce b instead of any other type', ...)`

## Investigation summary
- The compiler currently detects this shape as a mana-replacement pattern but does not lower it to a structured effect; nearby coverage intentionally asserts strict failure for similar clauses.
- Existing runtime replacement infrastructure models prevention, destination changes, redirects, ETB modifications, and related event rewrites, but there is no reusable event/action model for "land tapped for mana" output substitution by land subtype.
- This card needs a global substitution map over basic land subtypes (Plains->R, Island->G, Swamp->W, Mountain->U, Forest->B) applied to mana production events, not a one-card text normalization.

## Confirmed reusable gap
- Missing reusable parser/lowering/runtime support for mana-production replacement clauses of the form:
  - `If [land filter] is tapped for mana, it produces [mapped output] instead of any other type[/and amount].`

## Needed follow-up capability
- Add a reusable effect/replacement model for mana-production event replacement that can:
  - match land-tapped-for-mana events with land filters/subtype predicates,
  - substitute produced mana symbols/types via structured mappings,
  - preserve composability with other mana replacement/additional mana effects.
- Wire parser/lowering for these clauses into that generic model and add regression tests for both parsing and runtime behavior.
