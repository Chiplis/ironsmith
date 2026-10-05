# Paired-characteristic animation and pre-P/T grants

**UNVALIDATED: source and authored regressions only; no builds or tests run.**

One proposed additional full frozen identity: **Woodcaller Automaton**.
Prerequisites: dynamic base-characteristic assignments (`d3428ba2`) and complete
animation descriptors (`789d46bf`, locally `0a560480`). The fixture contains the
full frozen Oracle text, Prototype reminder/ability and metadata.

The shared value reader now represents `this/that creature's power and toughness`
as two independently evaluated characteristics of the same exact object. The
animation base-P/T tail accepts plain `equal to` as well as the existing scalar
`each equal to` form. The earlier base-assignment parser uses the same pair reader.

A bounded complete animation branch composes `with <granted abilities> and base
power and toughness <quantity>` without dropping the pre-P/T abilities. It uses
the existing complete granted-ability parser, typed creature descriptor and one
BecomeBasePtCreature action. Unknown grants or incomplete quantities reject.
Outer type/color-retention facts from the descriptor branch are retained; the
ordinary `It's still a land` follow-up retains Woodcaller's target's land types.
The existing one-shot animation executor freezes its two values at resolution.

Authored coverage:
- Exact tools strict/non-lossy fixture and direct/JSON-artifact runtime checks.
- Real normal and Prototype casts; trigger only if cast; target land untapping,
  Treefolk creature + haste + preserved Forest/Land; independent power/toughness
  responses; source departure LKI versus a new returned incarnation; existing
  counters above base P/T and indefinite duration after cleanup.
- Noncast entry does not animate or untap the land.
- Grammar pair identity, full sentence type-retention/grant composition, and
  rejected unknown grant/quantity tails.

Halfdane and Behind the Mask remain separate partials. No new serialized variants
or custom execution/test commands are introduced.
