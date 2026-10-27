# Compiled static ability integration audit

## Scope

The October 2026 K'rrik investigation reproduced a missing life-payment option
in the existing browser WASM build. The compiled artifact already contained
`BlackManaMayBePaidWithLife`; the model interpreter inherited the trait's
`false` default instead of consulting its executable leaf ability.

This audit compared all 160 `StaticAbilityKind` hooks with the compiled model
interpreter and checked the remaining defaults, native implementations, and
callers. It checks the integration boundary, not full rules compliance of every
card or every implementation already behind that boundary.

## Missing connections repaired

Twelve hooks now forward to the existing executable leaf:

| Hook family | Effect of the missing connection |
| --- | --- |
| Black mana paid with life | K'rrik did not offer life payment in compiled-card games. |
| Cannot be countered | The capability query returned false; counter execution already had an ability-ID fallback. Forwarding restores interface consistency. |
| Bands with other | The banding eligibility filter was absent. |
| Attacking and blocking groups | Group-dependent legality queries returned no restriction. |
| Attack-cost affordability and generic amount | Attack declaration queries lost the existing cost information. |
| Defending-player card types (single and multiple) | Conditional unblockability queries lost the required types. |
| Maximum attackers, attackers against controller, and blockers | Combat declaration queries lost their limits. |

Payment-specific hooks for convoke, improvise, delve, affinity, cost reductions
and increases, minimum spell mana, and prohibitions on life payment or nonland
sacrifice already have explicit interpreter implementations. K'rrik was the
missing permission in that payment group; it was not the only missing hook in
the wider static-ability interface.

The shipped artifact inventory confirms examples beyond K'rrik: Crawlspace,
Silent Arbiter, Dueling Grounds, and Caverns of Despair use the combat-limit
hooks; Neurok Spy, Tanglewalker, Scrapdiver Serpent, and the Beebles use the
defending-player card-type restrictions.

## Defaults retained after review

The interpreter explicitly implements 148 hooks after the fix. The remaining
twelve do not need this forwarding change:

| Hooks | Existing path or reason |
| --- | --- |
| `canonical_model`, `rewrite_text_words`, `source_line_static_group_member_count` | Trait defaults read the interpreter's compiled model. |
| `clone_box` | The trait's clone implementation applies to the interpreter. |
| `characteristic_defining_colors`, `characteristic_defining_subtypes`, `is_source_only_graveyard_replacement` | Public `StaticAbility` accessors read the compiled model directly. |
| `equipment_grant`, `pay_non_mana_attack_cost` | No concrete native overrides; current behavior uses other hooks. |
| `goaded_by_player`, `required_attack_player` | Source-bound runtime grants create native abilities directly; there is no compiled leaf payload carrying their object IDs. |
| `structural_effect_filter` | No engine runtime consumer; compiled-text rendering already falls back to matching the compiled model payload. |

## Regression coverage

- Compiled K'rrik permission reaches payment options, planning, explicit pip
  selection, and life deduction; preview does not mutate life.
- Permission belongs to the controller, preserves the black cost, rejects
  insufficient life, and respects compiled life-payment prohibitions.
- Existing K'rrik/Trinisphere/Yasharn cost tests now use compiled model abilities.
- Compiled combat restrictions preserve limits, filters, group legality, and
  attack payment requirements.
- Native web-session coverage exercises the payment editor's life option,
  replan command, confirmation, and spell placement on the stack.
- Browser coverage selects “Pay 2 life” and verifies that the exact pip choice
  reaches replanning and requires a separate confirmation.

## Validation results

- All four new engine regressions passed. The final corrected test source was
  compiled as an isolated Rust test executable against the freshly built engine
  library, avoiding an unrelated shared Cargo build lock.
- All 72 selected existing cost and combat tests passed.
- The focused browser life-selection regression passed.
- The native web-session test could not run: the existing
  `public_audit_boundary_17_11_31_tests.rs` imports missing `CounterExileGate` and
  `CounterExilePermission` types and accesses the removed `exile_permission`
  field. That file was not changed by this fix.
- The broader payment-editor browser fixture failed its earlier assertion that
  Pay is disabled while a battlefield-click replan is held, before life
  selection. The focused life-selection test passes independently.
- The browser WASM bundle was not rebuilt; the source fix requires the normal
  WASM rebuild to appear in the running app.
