# Relative hand quantities and instruction-wide rounding

Status: **UNVALIDATED source proposal**. No builds, compiler execution, or tests were run. The requested source-first workflow is still in force.

## Exact membership and before/after contract

`fixtures/relative_hand_quantities.json.fixture` preserves the full frozen metadata and oracle text for ten cards. Nine are independent of the discard-cost worker; the tenth is a joint closure with discard-cost commit `6e29de42`.

| Subgroup | Exact cards | Stack07 diagnostic | Proposed outcome |
| --- | --- | --- | --- |
| Half-hand discard, including secondary sacrifice rounding | Fraying Omnipotence; Lord Xander, the Collector; Pox Plague; Rush of Dread | missing discard count | Full metadata-bearing strict compilation, no loss, executable counts |
| Compared quantities | Balance of Power; Sandstone Oracle; Slithermuse; Skull Raid | missing draw count | Full strict compilation and correctly bound draw quantity |
| Inherited prior-object primitive | Heed the Mists | missing draw count | Full closure from the earlier typed milled-card mana-value leaf; now explicitly covered |
| Joint cast trigger and activation cost | Kozilek, the Great Distortion | unsupported discard activation cost; draw body also needed this family | Compiler/body closure after both commits; whole-card proposal remains partial pending payment-disclosure Undo remediation |

Do not count the remaining ten discard-count or ten draw-count diagnostics from this work. Balance-style minimums, choose-and-discard-the-rest, equal-to discard references, draw-step misdispatch, bottom-library replacement, base-power copying, toxic totals, and source-specific damage history remain distinct work.

## Implementation

- Half-hand discard is recognized as a complete noun phrase, with explicit `your hand` distinct from the recipient-relative `their hand`. It produces existing `CardsInHand`, `HalfRoundedDown`, and `Add` values. Each player's hand is sampled by that player's existing discard effect at resolution, before its selections.
- Fractional sacrifice can inherit the ordinary downward default. A following `Round up each time` now reaches the actual `ChooseObjects.count_value`, as well as the life/discard counts it already adjusted. The existing chooser and sacrifice executors enforce the computed number of controlled matching permanents.
- Bare `the difference` uses the existing pending comparison primitive. The draw equal-to reader already delegates through the shared value grammar; no duplicate parser fallback is necessary.
- A conditional's operands are available inside its own body. Early recursive passes defer the body’s pending comparison references until the local lexical environment exists, rather than accidentally binding an earlier comparison. Existing target/chosen-player resolution and object-tag aliases supply the actual compared references.
- Strict hand comparisons preserve the authored boundary. `Fewer than seven` is a typed `< 7`, not a Boolean-only `<= 6` subsequently misused as the quantity boundary. Promoting a leading condition to an intervening-if retains its comparison operands for the body.
- `Fewer than N cards were discarded this way` is a typed, negated prior-discard threshold. Its difference reads the exact producer's affected-object count, including zero; it does not infer success from hand size or from the requested discard count. No new serialized variants or executor markers were introduced.

## Authored regressions

- Normal tools integration target: aggregate strict metadata-bearing checks for all ten full payloads; Kozilek requires the joint cost change.
- Normal compiler-runtime integration target: direct compiler definition versus JSON-round-tripped artifact materialization, and eight gameplay scenarios.
- Multiplayer half-loss/discard/sacrifice with odd and even resources, public draws and life loss after casting, and a controller change before resolution.
- Xander's separate enter/attack/death resource instructions; Spree modes with different announced player targets; balanced and negative hand differences; chosen-opponent ETB/LTB/Evoke behavior; Skull Raid with zero, one, two, or five available cards; exact milled-card mana values; Kozilek's cast trigger with responses changing hand size before resolution.
- Grammar/reference tests cover complete-shape recognition and rejection, default/explicit fraction rounding, preservation of threshold seven, the typed discard predicate, prior-effect identity, local comparison scope, and lexical versus iterated player references.

Kozilek is currently withheld from whole-card source coverage because its publicly disclosed discard cost can be undone before the ability resolves. See [payment disclosure boundary](card-failure-payment-disclosure-boundary.md). The other nine remain proposed.

No source-proposed outcome is a measured success. Any independent full-card blocker exposed by the first permitted validation must keep that identity open until repaired.
