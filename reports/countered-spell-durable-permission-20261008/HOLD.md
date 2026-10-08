# Countered-spell durable free-cast cohort: HOLD

Source-review base: `d65bd6564569a38132ae107cbe209af81c87f4f6`.
Retained diagnostic baseline: `1dd81cd84c62f272479f26e16d74719fff24b97b`.
Scope: exactly three primary oracle IDs. Measured recoveries: **0**.
All parser, lowering, direct-artifact, runtime, negative, build, and corpus execution: **UNRUN**. No production code or inherited artifact15 was changed. Only source reading and retained JSON extraction were performed. No repository AGENTS.md or repository skills were found; the dependency AGENTS.md outside this worktree is irrelevant. The software-engineering skill was read.

## Frozen inputs

`frozen-inputs.json` contains the complete actual metadata records from retained `data/cards-current.json`, retained failure entries with raw and normalized bodies, content hashes, and source metadata provenance. The source metadata declares SHA-256 `bae465b9d536fffa24c656daff5577a87f5a963dcb2160be0c1dc9ba8e225750`; this is retained provenance, not a newly executed hash validation.

- Spelljack, `7687b2a7-816d-4416-979b-675e35e235fc`: Instant, {3}{U}{U}{U}. Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard. You may play it without paying its mana cost for as long as it remains exiled. Raw text also has the X-is-zero reminder.
- Thranduil's Decree, `d7cba934-02ad-4677-bb4d-50808b01b4f9`: Instant, {4}{U}{U}. Counter target spell. If a permanent spell is countered this way, exile it instead of putting it into its owner's graveyard. You may cast that card without paying its mana cost for as long as it remains exiled.
- Kheru Spellsnatcher, `c01411e0-77b2-4e65-a369-5dbe13745769`: Creature — Snake Wizard, {3}{U}, 3/3. Morph {4}{U}{U}. When this creature is turned face up, counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard. You may cast that card without paying its mana cost for as long as it remains exiled. Frozen metadata retains the complete morph reminder and body.

## Actual route and first obstruction

Paths below are relative to `crates/`.

1. `ironsmith-compiler-grammar/src/effect_sentences/chain_carry/chain_readings.rs:990-1040` strips leading player-may, parses the remainder, binds the player, and distinguishes permission from an optional resolution action.
2. The primitive table at `effect_sentences/clause_primitives.rs:482` indeed omits cast/play from its head list. However, `effect_sentences/clause_dispatch/clause_dispatch_core/clause_readings.rs:188-196` has an unconditional-head `cast-or-play-tagged` reading (unless already read by may-cast-it), calling the same specialized parser via `clause_readings/part_1.rs:637-673`. `clause_dispatch_core.rs:139-145` executes that registry. Adding primitive heads is not demonstrated to resolve this failure.
3. `grammar/permission_facts/tagged_surface.rs:446-473` accepts implicit cast/play. `permission_helpers.rs:1275-1298` delegates the tail and returns no match for an unsupported unprefixed tail.
4. The decisive missing composition is `grammar/permission_facts/tagged_surface.rs:780-811`: its tail alternation accepts turn duration optionally followed by free cost, free cost followed by turn duration, long lifetime alone, free cost alone, or an empty tail. It has no branch combining free cost with `for as long as ... remains exiled`. The lifetime itself exists at lines 691-718. Both complete third sentences therefore lack this compositional owner, irrespective of primitive heads. This is source inference, not an executed parse trace.

Do not add cast/play to generic known verbs. A future narrow tail repair must require complete consumption and preserve actor, verb, demonstrative surface, free price, and exact lifetime, but it must wait for the semantic issues below.

## Independent semantic blockers

### Permanent-spell replacement gate is lost by a broad owner

`effect_sentences/dispatch_entry.rs:1009-1034` recognizes countered-this-way, instead-of, graveyard, and exile by marker presence and returns a Stack→Graveyard replacement into Exile targeting `It`. It does not inspect or preserve `a permanent spell`. The sentence loop invokes this directly at lines 3078-3093 and appends its result before ordinary sentence parsing. Thus this route can consume Thranduil's full second sentence while erasing its permanent restriction. `maybe_rewrite_future_zone_replacement_sentence` at lines 1339-1378 is another user of the same lossy recognizer. A repaired permission tail must not make that semantic loss look like recovered support.

Required boundary: a complete counter-destination production with a typed target-characteristic gate evaluated against the stack spell before movement. Any spell remains a legal counter target. Only permanent spells get the exile replacement and resulting grant. Instant/sorcery targets are still countered to the ordinary destination. Gate the replacement, not the initial target domain. Register the replacement only around the relevant counter event, never a later unrelated move.

### Durable free-cost consumer does not share the exact-incarnation contract

`ironsmith-compiler-lowering/src/lowering_impl/compile_support/effect_dispatch/subject_verb_middle.rs:1757-1837` lowers the durable free permission to a `GrantPlayTaggedEffect` plus a separate `GrantTaggedSpellFreeCastUntilEndOfTurnEffect.for_as_long_as_exiled()`.

The play consumer `ironsmith-engine/src/effects/player/grant_play_tagged.rs:322-337,489-518` requires an extant exile snapshot/object and uses exact object-ID grants for this duration. The free-cost consumer `effects/player/grant_tagged_spell_free_cast_until_end_of_turn.rs:55-72,117-128` instead follows a stale snapshot by stable ID and installs a stable-card grant. Its default zone is Exile (`ironsmith-core/src/effect.rs:6236-6263`), but it does not require the snapshot itself to name the current exile incarnation.

Important qualification: ordinary departure DOES revoke stable grants. `game_state/zones_and_characteristics.rs:1226-1238` and `grant_registry.rs:1762-1766` implement that cleanup; the existing Release to the Wind unit test explicitly covers ordinary exile→graveyard→exile. It would be incorrect to claim ordinary reentry revival merely from stable-ID storage.

The remaining concrete mismatch is (a) stale-snapshot rebinding at grant creation, and (b) the explicit Adventure exception: exile→stack casts of Adventure cards preserve exile stable grants; stack→exile also does not clear them. This duration's permission must end on leaving exile even if an Adventure subsequently creates a new exile permission. A stale free-cost grant must not discount that later, independently authorized cast. These are source-established mechanisms requiring focused executable witnesses, not measured failing tests.

A narrow repair should bind the free-cost alternative to the same exact committed exile object and lifetime as PlayFrom, or represent both as one priced permission. Existing exact-ID `GrantPlayTaggedEffect.with_alternative_cost(...)` machinery is a candidate, not an automatically safe replacement: preserve play versus cast, multi-face legality, mandatory additional costs, alternative-cost exclusivity, renderer, verification, and serializable artifact meaning. Do not broadly remove Adventure's existing zone-lifetime behavior without its own controls.

### Counter receipt-to-permission ownership still needs proof

There is real infrastructure; this is not a claim that receipts are absent. `effects/stack/counter.rs:153-199,379-431` commits zone movement, records successful counter events, retains published outputs, and finalizes zone-change receipts. `effects/composition/local_rewrite.rs` scopes destination replacements around the antecedent effect. Counter effects also add exiled-with-source links when the final destination is exile.

What is not established is the complete compiled three-card path from that successful counter receipt to both durable grants. The permission must consume only this resolution's exact resulting exile incarnation, not a pre-move stack snapshot, stable-card fallback, broad source-linked collection, or another effect's last tag. An uncounterable/invalid target, prevented move, or a different final destination must produce no permission. Source departure must not remove a valid duration-while-exiled grant. This is a mandatory unresolved proof obligation, not a reason to force parser success.

## Bounded next implementation and authored evidence plan

All cases below are **planned / UNRUN**, not executable tests authored or results obtained. HOLD requires no speculative production patch.

1. Introduce a fully consumed counter→replacement→durable permission route or equally precise compositional facts. Preserve the permanent gate, counter-success provenance, final Exile destination, exact post-move reference, actor, play/cast distinction, free cost, and duration. Preserve Kheru's morph {4}{U}{U} and the turned-face-up trigger as complete card behavior.
2. Repair free-cost lifetime ownership before enabling the missing lifetime-tail composition. Choose one permission family or two demonstrably co-owned exact-ID grants, with no stable-ID recovery of a stale input.
3. Parser evidence: compile all three complete frozen metadata/body inputs strictly and non-lossily; verify exact AST fields, actor, counter target, permanent gate, replacement rather than post-graveyard exile, play/cast, lifetime, free price, full morph and trigger. Assert no optional resolution prompt for durable permission. Reject unsupported extra tail text, copy/pile/global-exile variants, wrong gate, incomplete destination rider, and missing antecedent. Controls: existing immediate free casts, duration-only tagged grants, turn-only free grants, Release to the Wind.
4. Lowering evidence: assert the local counter replacement and output receipt binding, exact Exile reference, gate placement, recipient, both play/cast domains, and common lifetime/price identity. Assert no global source pool or stable fallback, no unimplemented effect, no hard-coded card-name dispatch.
5. Direct-artifact evidence: construct/materialize the same typed runtime representation without grammar, serialize/deserialize if supported, inspect price/identity/lifetime and receipt declarations, and run matching runtime witnesses. Ensure artifact/semantic verification rejects a dropped permanent gate or dangling consumer. Do not rewrite inherited artifact15. Any new typed producer/reference/permission relation requires explicit later semantic-boundary admission and direct-artifact controls on the repaired stack.
6. Runtime positive evidence: opponent-owned counterable creature spell, exact resulting exile object only, source leaves, turn passes, controller casts free using ordinary timing; mandatory additional cost still paid, X=0, no combining alternative costs. Spelljack permits play for a legitimate land face without an unrelated land-play grant; the cast-only cards do not. Kheru is cast face down and turned face up by paying the full morph cost, then resolves the complete trigger.
7. Runtime negative evidence: uncounterable or absent target; Thranduil instant/sorcery targets go to ordinary destination; counter replacement redirected/prevented; source had older linked exiles; unrelated current exile; object leaves before permission creation; stale snapshot after reentry; ordinary leave/reentry; Adventure exile→stack→exile followed by a distinct new casting permission; other controller tries to use grant; no off-turn sorcery/land permission; insufficient mandatory additional cost; alternative-cost combination. Verify both availability and actual casting/payment, not merely registry entries.
8. Run authorized focused parser/lowering/direct-artifact/runtime negatives first. Only after those pass, request/perform the later narrow exact-ID corpus measurement and semantic/release boundary on the final integrated SHA. Retain newly exposed blockers separately. No recovery credit before those gates; no collateral credit to unrelated free-cast families.

## Outcome

HOLD all three IDs. The first grammar obstruction is understood more precisely, but complete ownership is not established. Fixing that obstruction alone would expose a lost permanent gate and mismatched free-cost identity/lifetime. This report is a bounded next-step artifact, not source clearance or release admission.
