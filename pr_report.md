# Open PR Merge & Card-Compile Report

All 42 open `codex/aws-card-fix-*` pull requests (PR #661 – #728) were merged into `main`
one at a time, resolving every merge conflict by hand. After the full merge the workspace
(`ironsmith-core`, `ironsmith-runtime`, `ironsmith-compiler`, including `--tests`) compiles
cleanly with **zero errors**.

Each target card was then re-compiled with:

```
compile_oracle_text --name "<card>" --compare-text
```

`sim` = text similarity between original oracle text and the compiled-back text
(1.0 = byte-identical). `mismatch=false` means the compiled definition is **semantically**
equivalent to the oracle text even when wording differs.

## Headline

| | Before merges (baseline `main` 93ee31241) | After merging all 42 PRs |
|---|---|---|
| Cards compiling | **6 / 42** | **35 / 42** |
| Cards failing to parse | 36 / 42 | 7 / 42 |

**29 previously-failing cards now compile.** No card that compiled at baseline regressed.

## The big picture: why conflicts were heavy

Every PR was authored against an older `main` whose parser has since been **heavily
refactored** (the winnow/logos rewrite, plus the migration of ad-hoc `&str`/`&[&str]`
phrase constants into `ClauseShape`/`clause_shape!` patterns, and a token-vs-word API
flip in several modules). So almost no PR merged textually clean *and* compiled — even
the "clean" git merges (#694, #697, #721, #728) left stale constant names or
single-vs-`Vec` return-type mismatches that only surfaced at build time. Resolution
therefore meant re-expressing each PR's intent in current `main`'s idiom rather than
just picking a side.

Three resolution strategies were used:
- **Graft** – keep `main`'s refactored code, port only the PR's net new logic into the
  current API (most PRs).
- **`-s ours`** – record the merge but keep `main`'s tree, used when the card already
  compiled on `main` (PR superseded) **or** the fix was too entangled with the refactor
  to port safely without risking the whole build.
- **Fixup commits** – for clean auto-merges that compiled-broken, a follow-up commit
  realigned the stale references (called out below).

## Per-card results

Legend: ✅ compiles · ⚠️ compiles, wording differs (semantically OK) · ❌ still fails to parse

| PR | Card | Baseline | After merge | How it was merged |
|----|------|----------|-------------|-------------------|
| 661 | The Space Family Goblinson | ❌ | ✅ 1.0000 | union: added `PlayerRollsDie` trigger + dice-rolled static condition alongside main's `PlayerRollsHighestNaturalResult` |
| 662 | Rakdos, the Muscle | ❌ | ✅ 1.0000 | ported PR's exile-top-of-library fn to main's `ClauseShape`/token API |
| 663 | Staff of the Storyteller | ❌ | ✅ 1.0000 | clean merge |
| 665 | Last Rites | ✅ 1.0 | ✅ 1.0000 | `-s ours` — superseded; main already fixed it (dup tests on PR branch) |
| 666 | Kodama of the Center Tree | ✅ 1.0 | ✅ 1.0000 | `-s ours` — superseded; main has more-refined soulshift handling |
| 668 | Feast of the Victorious Dead | ❌ | ✅ 1.0000 | source auto-merged; resolved test union |
| 669 | Maddening Cacophony | ⚠️ 0.95 | ⚠️ 0.9487 | `-s ours` — already parses (mismatch=false); PR was a stale full `clause_shape` rewrite |
| 670 | Hundred-Battle Veteran | ❌ | ✅ 1.0000 | clean merge |
| 671 | Aberrant Return | ❌ | ✅ 1.0000 | clean merge |
| 672 | Mindstorm Crown | ❌ | ❌ | grafted at-turn-start cards-in-hand predicate; **advances past baseline error** but now hits unsupported predicate `you had card in hand` at lowering (see below) |
| 673 | Gloomwidow's Feast | ❌ | ✅ 1.0000 | ported `tagged_historical_identity_shape` to main's `_tokens`/`_word_refs` helpers |
| 674 | Vassal's Duty | ❌ | ✅ 1.0000 | union match arm: added `RedirectNextDamageFromSourceToTarget` |
| 675 | Nightscape Battlemage | ❌ | ✅ 1.0000 | union imports + both line-family dispatch checks |
| 680 | Disciple of Perdition | ❌ | ✅ 1.0000 | took main's exile_actions (already covers PR's variants); real fix auto-merged |
| 683 | Unbound Flourishing | ❌ | ⚠️ 0.8788 | added `ScaleXValue` arm + "double the value of x"; also completed its cfg(test) match arm |
| 684 | Realmwright | ❌ | ✅ 1.0000 | broadened as-enters subjects + Land→`add_chosen_basic_land_type` branch in main's idiom |
| 685 | Thassa's Intervention | ❌ | ✅ 1.0000 | clean merge |
| 686 | Nicol Bolas, Dragon-God | ❌ | ✅ 1.0000 | core loyalty-ability fix auto-merged; grafted hand-or-permanent exile pattern |
| 687 | Aspect of Wolf | ❌ | ✅ 1.0000 | clean merge |
| 688 | Increasing Confusion | ❌ | ✅ 1.0000 | took PR's `misc_actions.rs` (base+11 lines) — body had auto-merged PR code |
| 690 | Aligned Heart | ❌ | ✅ 1.0000 | took PR's `creation_handlers.rs` (base+63 lines, rally-counter fix) |
| 691 | All of History, All at Once | ❌ | ✅ 1.0000 | grafted Time Travel hook in main's `&str` idiom (`--theirs` broke on renamed AST variants) |
| 694 | Dust Animus | ❌ | ✅ 1.0000 | clean merge **+ fixup**: stale `ETB_*` const names → `AND_/ARTICLE_/COUNTER_OR_COUNTERS_WORD_PATTERN`; counter fn → `Vec` return |
| 695 | Niambi, Esteemed Speaker | ❌ | ✅ 1.0000 | leaf.rs import union (legendary-card discard cost) |
| 697 | Tormented Thoughts | ❌ | ✅ 1.0000 | clean merge **+ fixup**: `DISCARD_AT_RANDOM_PATTERN` → `word_slice_eq(.., DISCARD_AT_RANDOM_WORDS)` |
| 698 | Aetheric Amplifier | ❌ | ✅ 1.0000 | clean merge |
| 700 | Turtle-Duck | ❌ | ❌ | `-s ours` — base-power-only fix threads through 10+ hunks of main's refactored `gain_ability.rs`; not safely portable |
| 701 | Fall from Favor | ❌ | ✅ 1.0000 | 4-file graft (monarch unless-clause, `tail_tokens` AST field auto-merged) |
| 706 | Katara, Seeking Revenge | ❌ | ❌ | grafted possessive paid-cost label + discard-unless predicate; **advances past baseline error** but card also uses unsupported `waterbend 2` (see below) |
| 708 | Hargilde, Kindly Runechanter | ❌ | ❌ | `-s ours` — main already has a competing, more-refined 2-filter mana-restriction model; PR's 1-filter approach would regress it. Card's `spend-this-mana-only` marker still dropped by main's renderer |
| 711 | Creeping Peeper | ❌ | ✅ 1.0000 | clean merge |
| 713 | Captain Howler, Sea Scourge | ❌ | ❌ | removed main's "one or more" trigger-subject guard (PR fix); **advances past baseline error** but now hits unsupported trigger subject filter `that creature` (see below) |
| 715 | Ratonhnhaké꞉ton | ❌ | ❌ | `-s ours` — 6-file feature entangled with token-vs-word refactor + a competing struct field; too high-risk to graft |
| 716 | Tangle Wire | ❌ | ✅ 1.0000 | fix auto-merged (for-each fade counter → `CountersOnSource`); resolved test union |
| 720 | Summoner's Grimoire | ❌ | ❌ | `-s ours` — named-equip fix entangled across 3 files incl. a diverged `KeywordAction` enum |
| 721 | Flycatcher Giraffid | ❌ | ✅ 1.0000 | clean merge **+ fixup**: stale `ETB_*` names; `enters_with_counter_choice` wrapped in `vec!` |
| 722 | Glissa Sunseeker | ❌ | ✅ 1.0000 | added "unspent mana you have" → `Value::UnspentMana(You)` in main's matcher form |
| 723 | Will Kenrith | ❌ | ⚠️ 0.9810 | threaded `&duration` (def auto-merged to 4-arg); took PR's until-your-next-turn handling |
| 725 | Eternal Scourge | ✅ 1.0 | ✅ 1.0000 | `-s ours` — superseded; main already compiles it |
| 726 | Decode Transmissions | ⚠️ 0.54 | ⚠️ 0.5353 | `-s ours` — multi-file "warp" mechanic entangled with refactor; card already parses |
| 727 | Defensive Formation | ❌ | ✅ 1.0000 | clean merge |
| 728 | Stolen Strategy | ⚠️ 0.78 | ✅ 1.0000 | took main's `generic_subject_verb_programs` (PR fns duplicated main's); fixed stale `EXILE_CARD_OR_CARDS_WORD_PATTERN` |

## The 7 cards that still fail

**Skipped by `-s ours` (PR fix not portable to refactored `main`; card unchanged from baseline):**

- **Turtle-Duck** (#700) — `unsupported trailing base power clause` ("…has base power 4 and gains trample"). Fix needs base-power-only threading through main's rewritten `parse_gain_ability_sentence`.
- **Hargilde, Kindly Runechanter** (#708) — `compiled text dropped required semantic marker: spend-this-mana-only`. `main` has a competing, more-refined mana-restriction model; adopting the PR's would regress it, and `main`'s own renderer still drops the marker for this card.
- **Ratonhnhaké꞉ton** (#715) — `unsupported static condition clause` ("this hasn't dealt damage yet"). 6-file feature collided with main's token-vs-word refactor and a competing reference-env field.
- **Summoner's Grimoire** (#720) — `parser does not yet support line family: 'Abraxas — Equip {3}'`. Named-equip fix collided with a diverged `KeywordAction` enum.

**Grafted fix landed, card advanced past its original error, but still fails on a *different / deeper* unsupported construct the PR didn't cover (or only partially covered):**

- **Mindstorm Crown** (#672) — baseline error (`missing condition after trailing if clause`) is gone; now fails at `unsupported predicate: you had card in hand` during lowering. The at-turn-start predicate now parses but the no-amount ("had a card in hand") variant isn't supported downstream.
- **Katara, Seeking Revenge** (#706) — baseline error (`unsupported trailing discard clause`) is gone; the "unless her additional cost was paid" path now parses, but the card also has `Waterbend 2`, a keyword action the parser still doesn't recognize.
- **Captain Howler, Sea Scourge** (#713) — baseline error (`unsupported triggered line … discard one or more cards`) is gone; now fails at `unsupported trigger subject filter: that creature` in the "+2/+0 for each card discarded this way" clause.

These three are genuine partial wins: the specific defect each PR targeted is fixed, but the cards carry additional unimplemented mechanics.

## Notes / caveats

- Merges were made **locally** on `main`; nothing was pushed to `origin` and no PRs were closed on GitHub (say the word and I'll push / close them).
- Two pre-existing generated artifacts (`web/wasm_demo/pkg/ironsmith_bg.wasm`, the frontend cards checksum) were stashed before merging and are untouched.
- `Maddening Cacophony`, `Will Kenrith`, `Unbound Flourishing`, `Decode Transmissions` compile with `mismatch=false` but sub-1.0 similarity — semantically correct, wording differs from the oracle text.

---

# `cargo nextest run --workspace --release --all-targets --no-fail-fast`

**Result: `8657 tests run: 8600 passed, 57 failed, 3 skipped`.**

(One extra fix was required just to get the full workspace to compile: the non-default-member
`ironsmith-wasm` crate had exhaustive `match`es over `SpecialAction` and Creeping Peeper (#711)
added a new `UnlockRoomDoor` variant — handled in a follow-up commit.)

The 57 failures were classified by checking whether each failing test exists in the baseline
(`93ee31241`) source:

## A. Regressions — 40 tests that PASS on baseline `main` but FAIL after the merge

These are the real concern: the merge changed shared parser behavior and tripped architectural
ratchet lints.

**9 architectural ratchet lints** (`ironsmith-tools::workspace_boundaries`) — the merged PRs were
written in `main`'s *pre-refactor* word-based idiom (`word_slice_eq`, `.contains`, `token_word_refs`
cursor walks), and several of my conflict resolutions kept that idiom. The ratchets require
clause-shape / token-backed matching and an allowlist of cursor walks:
`runtime_backend_matches_words_is_clause_shape_primitive_only`,
`runtime_backend_word_cursor_walks_are_allowlisted`,
`shared_util_production_shape_gates_use_token_backed_matching`,
`whole_clause_shape_gates_use_lexed_clause_matching`,
`creation_handlers_route_shape_gates_through_lexed_clauses`,
`etb_clause_shape_guards_match_captured_clauses`,
`keyword_static_as_enters_simple_choice_parsers_use_token_tail_wrapper`,
`keyword_static_trigger_duplication_and_untap_if_tails_use_token_shapes`,
`player_cards_in_hand_conditions_use_shared_capture_parser`.

**31 existing card / effect-parse regressions** — unrelated cards whose parsing/rendering changed.
The clearest example: **Inferno Titan** now errors `unsupported divided-damage target count
('divided as you choose among one two or three targets')`. Full list:
`captain_america_first_avenger_…`, `captain_america_throw_…`, `commander_liara_portyr_…`,
`gnarled_sage_…`, `mighty_servant_of_leuk_o_…`, `mighty_servant_two_creature_crew_…`,
`minds_dilation_…`, `octavia_living_thesis_…`, `oracle_render_regression_named_cards_compile_cleanly`,
`parse_conditional_anthem_and_haste_…`, `parse_conditional_anthem_and_keyword_…`,
`parse_day_of_black_sun_…`, `parse_descend_condition_…`, `parse_exile_top_card_of_target_library_…`,
`parse_metalcraft_self_buff_…`, `parse_oracle_gwen_stacy_ghost_spider_…`, `parse_orzhov_advokist_…`,
`parse_rejects_divided_damage_distribution_clause`, `parse_the_sixth_doctor_…`,
`parse_the_stasis_coffin_…`, `parse_trigger_with_and_or_subtype_list_…`,
`parse_until_end_of_turn_you_may_cast_that_card`, `parse_where_x_this_ability_resolved_…`,
`player_subject_role_boundary_regressions_…`, `rampaging_cyclops_…`,
`render_source_surface_for_hard_triggered_and_static_clauses`, `saving_grace_…`,
`scryfall_inferno_cards_parse_without_unsupported_markers`, `stoic_sphinx_…`,
`test_parse_labeled_leading_condition_with_gets_and_has`, `urabrask_heretic_praetor_…`.

## B. New PR-added tests that don't pass — 17 tests (not baseline regressions)

These are tests the PRs themselves added; they fail because the graft is incomplete or produces
output that differs from what the PR's test asserts (consistent with §"The 7 cards that still fail"):
Mindstorm Crown (3), Katara (5, incl. the unsupported `waterbend` cost), Captain Howler (3),
Will Kenrith (2, strict text differs at sim 0.98), Unbound Flourishing (1, sim 0.88),
Tangle Wire / fade-counter (3 — produces `CountersOnSource(Fade)` where the test expects a
distinct "source fade counter count" value).

## Root cause & recommendation

A targeted check (reverting the two whole-file `--theirs` resolutions, #688/#690) recovered only
2 of the 48 sampled failures, so the regressions are **spread across many of the word-idiom
grafts**, not one file. The underlying issue is structural: these 42 PRs were authored against a
`main` that has since been heavily refactored (winnow/logos rewrite + `&str`→`ClauseShape`
migration + token-vs-word API flip). Resolving the conflicts in a way that compiles and adds the
target cards still leaves behavior/lint regressions because the PRs' parsing approach predates the
refactor.

**This merge is therefore not production-ready as-is.** Recommended next steps (not done here, as
the request was to surface regressions):
1. Re-express the surviving grafts in the current clause-shape / token idiom to clear the 9 ratchet
   lints.
2. `git bisect` the 31 card regressions across the merge commits and either fix or downgrade the
   offending resolutions to `-s ours` (dropping that card rather than regressing others).
3. Decide per-card whether the 7 still-failing / partially-grafted cards are worth completing.

Nothing was pushed to `origin`; all work is on the local `main`. The full per-merge resolution
log is preserved in the commit history (`git log 93ee31241..HEAD`).
