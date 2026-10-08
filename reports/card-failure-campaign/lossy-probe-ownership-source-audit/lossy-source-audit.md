# Strict-lossy source-only audit, 2026-10-08

## Result and limits

Baseline exactly `1dd81cd84c62f272479f26e16d74719fff24b97b`. 94 strict-lossy entries: 93 carry suffix-object-filter recovery; Mathemagics carries input fallback. This is not evidence of 88 new gameplay regressions. No builds, tests, parser probes, corpus reruns, or code generation were executed for this audit. Source inspection and processing existing evidence only. No AGENTS.md or relevant .agents/skills files were available in this worktree or its accessible task parents; unrelated vendored dependency AGENTS.md is outside edited scope.

`lossy-source-audit-families.md` partitions all 94 entries into 18 nonoverlapping grammar families with exact IDs and Oct7 status. `lossy-source-audit-exact-ids.json` retains each full Oracle body, compiled prose, diagnostics, hashes, and category. Labels are triage, not independently measured defect counts.

Four preexisting losses: Brenard, Ginger Sculptor; Hofri Ghostforge; The Prydwen, Steel Flagship; Mathemagics. Two formerly failed: Akoum Hellkite (parser failure), Shard of the Void Dragon (semantic-output failure). Other 88 were supported-strict. The misleading existing key `lossy_gate_regressions` includes all 95 gate regressions, including seven parse failures; this audit filters actual category.

## Concrete root cause: speculative diagnostic leakage

1. `keyword_static/mod.rs:2383` directly invokes the new `parse_conditional_copular_creature_line` in the unstacked dispatcher, before the later single-line trigger guard (`:2990`) and outside the registry candidate captures (`:1885`). This invocation is newly present relative to Oct7 `5cc46c1`.
2. `keyword_static/costs_replacements_and_permissions.rs:2701` starts that new reader with an ownership check calling `parse_filter_is_pt_creature_in_addition_line(tokens)?`. Both Some and None are only speculative here: Some makes this reader return None to defer to the established owner.
3. The established reader (`:2968`) finds any animation copula, parses prefix and subject, and calls `parse_anthem_subject` BEFORE it verifies that the post-copula descriptor even starts with a P/T value.
4. `anthem_grant_lines.rs:2875` can call `parse_best_object_filter_suffix`; its `:2807` record is immediate. That record function already existed on Oct7 unchanged: the detector was not newly introduced. The new unisolated call route exposes abandoned candidates to the top-level loss capture.
5. Example from raw evidence: all 15 Leyline/Leyline Axe bodies plus Gemstone Caverns and Quicksilver yield a suffix such as 'this card' from 'if this card'. The legacy animation candidate then rejects 'in your opening hand' as non-P/T. The complete pregame reader can still succeed, yet the unrelated candidate's loss remains.
6. `ironsmith-compiler-api/src/parse_loss.rs` documents capture as isolating and restoring the previous collector; observe deliberately propagates, and replay commits recorded diagnostics. Registry candidates already capture and replay only the selected report. The proposed fix uses that same ownership principle, not a blanket diagnostic suppression.

The 17 opening-hand cards are the strongest coherent initial admission family. The same new probe can account for many unrelated trigger/quantity bodies because it reads to an embedded 'is' or 'are' before validating the descriptor. Attribution of every individual remaining diagnostic to this path is not proven without deferred execution. Genuine selected-path losses may coexist, especially the four already lossy cards.

## Smallest correction, authored but unexecuted

Isolated worktree: `/workspace/scratch/bc560e8d90ff/ironsmith-lossy-probe-ownership`, branch `card-repair/lossy-probe-ownership`, based exactly on the baseline. Only the ownership probe in `costs_replacements_and_permissions.rs` is wrapped in `parse_loss::capture`; its Result still propagates identically via `?`, and its report is discarded because this call never commits the AST. A later established owner recomputes and records any real selected loss. No dispatcher file, shared loss API, strict gate, parser acceptance policy, runtime logic, or renderer is changed.

Authored tests in separate `copular_probe_loss_tests.rs`: expose legacy None-with-loss behavior; require the new deferring reader to be clean; preserve outer-before/outer-after losses around nested capture; require an actually accepted suffix recovery to remain lossy; check typed complete pregame ownership including Gemstone's not-starting, luck-counter, and hand-exile obligations; retain the established complete sized-animation owner. Tests are source proposals, not passed results. No measured recovery count is claimed.

## Runtime and complete-body admission

The pregame compiler owner is `keyword_static/mod.rs:3365`, which constructs `PregameActionKind::BeginOnBattlefield` with not-starting-player, counters, and exile-card fields. Runtime `ironsmith-wasm/src/wasm_game_impl/pregame.rs:2220–2258` enumerates current hand objects, rejects previously used actions, enforces not-starting and sufficient other hand cards; `:2800–2860` rechecks hand membership and restrictions, moves the card, adds counters, and begins required exile selection. This is structurally different from a battlefield-only conditional static. The proposed change leaves that path unchanged; prose equality alone cannot verify its actual compiled use.

Admission must use full frozen bodies, not just the common opening line: Leylines retain their independent continuous, replacement, trigger, or activation rules; Gemstone retains conditional mana and luck/exile requirements; Quicksilver retains haste and counter activation. Leyline of Transformation is a separate parse-failing chosen-type body and is deliberately NOT in this 17-card family.

Further family-specific obligations, all carried in the exact-ID inventory:
- Pregame choose-color/mulligan: commander condition, choice lifetime, all-hand exile and same-count redraw; cannot become generic source static.
- Graveyard name equivalence: function in graveyard and affect only specified named spell effects.
- Mana replacement: distinguish replacing type from replacing amount; Damping Sphere's separate escalating spell tax remains mandatory.
- Ordinal trigger conditions: reset/track resolution count for the same ability, not entry or trigger count; preserve optional payment/reflexive trigger and all ordinal branches.
- Land-subtype branches: bind the entering land, preserve instead-vs-additive effects and optional target moves; Guardian's untap restriction must remain linked to the chosen creature.
- Cost reductions: bind the correct activation/equip, compute X at activation, preserve graveyard diversity threshold and mana components.
- Entry replacements: additional counters, other-object scope, conditional count, subtype/type retention, power lookup timing.
- Copy/quoted abilities: copy actual subject, exception base size/types, exile link ownership and recipient-owned activation/trigger. Preexisting losses must stay excluded unless independently fixed.
- Triggered animations: only target land and duration; retain land types, haste and separate vigilance/trample grants.
- Conditional cast/move: comparison reference, optional choice, no-cast/no-move fallback and destination are distinct. Rashmi prose currently says 'a nonland permanent with lesser mana value than it was revealed'; Oracle says spell with lesser value. This is a semantic warning requiring typed/runtime investigation, not a proven new regression or safe admission on detector cleanup.
- Dynamic quantities and attack taxes: preserve quantity source, controller, affected recipients, minimum/tie semantics, and per-attacker payment. Mixed player/creature damage must not drop one recipient class.
- Trigger joins/conditions: exact event disjunction, intervening-if check at trigger and resolution, command-zone functioning, target binding, and copied spell behavior.
- Draw replacement: reveal/order, condition and discard/payment choice; no second unintended draw.
- Participant/zone/delayed/granted bodies: ownership/controller, per-player target sets, delayed source identity, quoted ability ownership, and optional-failure branching.
- Postposed conditions: gain life only for the milled Lesson event, counter override for poisoned controller, live Desert condition for lifelink.
- Life and Limb: source evidence recovers 'all saprolings' from 'all forests and all saprolings'; verify the complete union remains Forest OR Saproling, both types and green 1/1, preserving other types. Do not declare the Forest branch lost from diagnostics alone because this may also be an abandoned probe.
- Mathemagics: unchanged unsupported superscript-zero lexer fallback, not covered by this patch.

## Deferred validation and negatives

When execution is authorized: run focused source tests, full frozen 17-card bodies through direct runtime and artifact round-trip paths, inspect typed pregame payloads and all additional abilities, then compare the exact 94-card admission set and unrelated strict gate boundary against retained baseline. Do not assume 90 or 93 clean recoveries. Cold/warm memo paths must agree because capture/replay participates in memoization.

Negative cases must retain selected suffix loss, unknown token/tail rejection, no unsupported source-zone copular admission, no broadened attachment antecedent, no granted-quote duration absorption, and no loss erasure from a surrounding or subsequently selected candidate. Check the existing sized-animation Some case still delegates, its true committed report survives, and any existing error remains an error. Include out-of-hand/starting-player pregame runtime negatives, insufficient exile payment, and wrong ordinal/subtype/destination branches before treating whole cards as supported.
