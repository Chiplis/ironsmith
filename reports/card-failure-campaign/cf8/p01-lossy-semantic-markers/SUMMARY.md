# cf8 p01 — lossy compilations and dropped semantic markers

Branch `cf8/p01-lossy-semantic-markers` (worktree `ironsmith-cf8-p01`), off origin/main 84ea8b41c.
Source-only: nothing was built or run. The prebuilt `compile_oracle_text` was used for
diagnostics until it disappeared mid-session (main rebuild); later work is reviewed by reading.

Status: 90 already-on-main, 27 source-proposed, 53 blocked, 0 untriaged (170 total).
Ledger: `ledger.jsonl`. Fixture with full typed bodies: `fixtures/p01_lossy_semantic_markers.json.fixture`.

## Clusters, root causes and fixes

### speculative-probe-loss (93: 90 already-on-main, 3 source-proposed)
Root cause: `parse_best_object_filter_suffix` records `suffix_object_filter_recovery`
immediately, and the new conditional copular reader ran the legacy sized-animation reader as an
uncaptured ownership probe; that probe read a subject up to any is/are/it's/its copula and then
declined, leaking the loss into an unrelated committed reading (every recovered span ends before a
copula). Fixed on main by cba78f342 (PR #873, captured probe). This branch adds a general backstop:
`parse_static_ability_ast_line_lexed` now captures its whole run and replays losses only when it
returns static abilities (same ownership rule the static registry already used for candidates), so
an abandoned whole-line static probe can no longer taint the trigger/activated/effect reading that
owns the line. That is what the three pre-Oct7 losses (Prydwen, Brenard, Hofri — spans start at
"whenever ...") need. Mathemagics is separate (blocked).
Files: `keyword_static/mod.rs`. Test: `speculative_probe_loss_ownership.rs` (all 93 bodies, both routes, loss-free).

### waterbend-additional-cost-render (4, source-proposed)
Mandatory "As an additional cost ..., waterbend {N}/{X}" is a waterbend-scoped `ManaPaymentCost`
in `additional_cost`; the renderer printed only non-mana components, dropping the line. Now renders
`ManaCost::payment_surface()`. Files: `text/.../ast_render.rs` (small additive hunk).

### fixed-mana-alternative-price-ambiguity (5, source-proposed)
"You may pay <mana> rather than pay the mana cost for <spells> you cast" matched both the fixed
alternative-mana-cost grant and `parse_independent_alternative_price_line` (its "pay rather"
deferral never fired because mana groups become word pieces). Different ASTs ⇒ registry ambiguity ⇒
the effect parser silently kept "You may pay {..}". The independent reader now defers pure-mana
prices. Files: `keyword_static/alternative_prices.rs`. Gap noted: the grant is hand(+command)-scoped.

### draw-replacement-ambiguity (2, source-proposed)
`parse_if_you_would_draw_instead_effects_line` did not defer to `DrawReplacementDouble` or
`DrawReplacementExileTopFaceDown`; ambiguity sent Thought Reflection / Asmodeus to a resolving
"If custom condition you_would_draw_card" effect. Added both exclusions and a flavor-label strip in
the face-down reader. Files: `keyword_static/costs_replacements_and_permissions.rs` (2 small hunks).

### tagged-reference-render (9: 6 source-proposed, 3 blocked)
Renderer-only: `chosen_exiled` helper tag ⇒ "that card" (play-until-next-turn); hand-zone
`PlayerTaggedObjectMatches` ⇒ "you put <card> into your hand this way"; ForEachTaggedPlayer over
`coin_opponents_lost` ⇒ "For each flip you lose", and over any this-way action tag ⇒ "For each
player <action> this way". Blocked: Aurelia's Fury (tap filter lacks Creature), Hog-Monkey Rampage
and Stolen Uniform (conditions bound to the shared `__chosen_objects__` set / wrong object).
Files: `effect_impl/late.rs`, `normalize_common/condition_rendering.rs`.

### stale-fail-loud-guards (5: 3 source-proposed, 2 blocked)
Exact-text "unsupported" rules in `grammar/document_shapes/unsupported.rs` blocked lines whose owners
now exist: Tetsuko (power-or-toughness disjunction filter; also removed the static-line guard),
Hellraiser Goblin (granted keyword + must-attack split already tested), Leviathan (new
`keyword_static/enters_tapped_untap_conjunction.rs` splits the source "enters tapped and doesn't
untap" line into its two owned statics). Death Cloud / Rebuild the City rules kept (not verifiable
without a build).

### each-basic-land-type (4, source-proposed)
Coalition Victory was misread as "a land with a basic land type and a creature". New predicate
reading `predicate_phrases/each_quality_control.rs` lowers "you control a <noun> of each basic land
type / color" to five `PlayerControls` leaves per quality (dual lands satisfy several types), and
the condition renderer folds them back. Planar Overlay / Sundering Titan already lowered five
per-type choices; new `effect_list/basic_land_type_choices.rs` renders the run. Tromp the Domains:
`ModifyPowerToughnessAll` reuses `describe_basic_land_type_pt_for_each`.

## Blocked, grouped by missing mechanic
- Random target / random reveal: Goblin Test Pilot, Witch Hunt, Singe-Mind Ogre.
- Activation-time value snapshot surface ("as you activate this ability"): Agility/Endurance Bobblehead, Lukka; Keeper of the Beasts (target-player filter missing entirely).
- Self-replacement / "instead": Epicenter (cross-paragraph), Orim's Touch (kicked), Archmage's Newt (saddled flashback {0}), Crackling/Harmonious Emergence (attached would-be-destroyed replacement static).
- Resolution-destination replacement for a cast spell: Goliath Daydreamer, Lilah, Quintorius.
- Counter replacement: Guile.
- Command zone: Liesa (commander tax alternative), Next of Kin, Stinging Study, The Ur-Dragon.
- Shares-a-card-type relation: Creeping Dread, Holistic Wisdom, Reality Scramble, Wild Magic Surge.
- Same-name relations: Locket of Yesterdays, The Apprentice's Folly, Yenna.
- Card types among (value / aggregate choice): April O'Neil, Winter.
- Keyword-cost alternatives (first-each-turn cycling/power-up, echo): Gavi, Thick-Skinned Goblin, Advancing the Spirit.
- Piles/divvy: Death or Glory, Ecological Appreciation, Abstract Performance.
- Level-up triggered abilities: Lighthouse Chronologist, Lord of Shatterskull Pass.
- Draft-matters / noted info: Paliano Vanguard, Smuggler Captain, Volo.
- Misc: Mathemagics (2^X value + lexer), Gideon's Triumph (attacked-or-blocked set + "of those"), Atomic Microsizer (conjoined can't-be-blocked), Rekindling Phoenix (token quoted ability unverified), Vesuvan Doppelganger (enter-as-copy exception), Trial of Agony (same-opponent targets), Aluren (any-player free+flash grant), The Ruinous Wrecking Crew (self-name rendering vs "crew" marker), Death Cloud / Rebuild the City (fail-loud rules kept), Aurelia's Fury, Hog-Monkey Rampage, Stolen Uniform.

## Risk notes
- `parse_static_ability_ast_line_lexed` loss ownership: losses from a static probe that returns
  None/Err are now dropped. A caller that relied on a declined static reading to report loss would
  lose that report — but such a reading committed nothing, so the loss was never the committed
  body's. The memoized `broad_static` cache was already non-replaying.
- Removed fail-loud rules: dead `cfg(ironsmith_runtime_parser_tests)` engine tests (shard_07,
  shard_09, shard_10) still assert the old errors.
- Cross-package: `unsupported.rs` RULES table, `alternative_prices.rs`, `condition_rendering.rs`,
  `effect_impl/late.rs` and `early.rs` are shared; hunks are small and additive. `keyword_static/mod.rs`
  gained a wrapper around the static line entry plus one reader call — likely merge touch point.
- History note: an early commit accidentally included `.cargo/config.toml`; the branch was rewritten
  locally to drop it before any push.
