# Animation characteristic templates

Status: UNVALIDATED source implementation; all tests are authored and unrun.

Twelve frozen candidates are listed as proposed-complete in
`fixtures/animation_characteristic_templates.json.fixture`. Behind the Mask is
included only as a partial requiring its conditional P/T self-replacement.
Dynamic numeric P/T references, quoted no-P/T transformations, custom cast-
bounded lifetimes and combat blocked/unblocked changes are separate families.

## Shared roots

- Leading legendary/snow/basic supertypes are read independently from the base
  P/T pair. Names after keyword grants remain names; names inside a quoted
  ability are not treated as the animation's name.
- Explicit base P/T can precede a complete static/granted ability list and an
  outer retention rider. The same granted-ability reader handles executable
  keywords and static protection rather than losing one kind of grant.
- Other colors and other types are independent semantic facts. A typed AST
  color-retention flag lowers to a real `AddColors`, while replacing color
  uses `SetColors`. Rendering inspects those modifications, not surface labels.
- Color-only additions, all five colors, and explicit colorless characteristics
  are retained. Color plus card-type changes use the existing typed operations
  without inventing a base P/T setting. Explicit retained creature subtypes
  survive; ordinary replacement forms remove the previous creature subtypes.
- Unknown leading-P/T descriptor or ability tails now reject the complete
  clause instead of silently becoming a bare creature. This closes a lossy
  fallback; supported-card replay must investigate any newly exposed cases.

The tests cover direct/restored full programs; one target reused across its
characteristic changes; counter versus base-size layering; legendary/name/type
identity; color/type retention and replacement; keyword/static grants;
expiration; enchanted-object scope; triggered-object scope; and unrecognized
suffix rejection. These source proposals are not measured compile/gameplay
recoveries.

Deferred commands, after the campaign gate permits validation:
- `cargo test -p ironsmith-compiler-runtime --test animation_characteristic_templates`
- `cargo test -p ironsmith-compiler-grammar animation_templates`

Primary rules reference: CR205.1 and613,
https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf

Current CR205.1b distinguishes a bare artifact-creature conversion from one
that names a new creature subtype: implicit retention keeps other card types,
but the latter replaces old creature subtypes unless retention is explicit.
The color/card-type helper preserves that distinction.
