# Residual next-cohort audit, exact main 1dd81cd

Source: 1dd81cd84c62f272479f26e16d74719fff24b97b. Read-only source/retained-metadata audit. No compiler, probe, build, test, corpus, or code generation was run. No source changed. No repository AGENTS.md or .agents/skills directory was present; the only discovered AGENTS.md was inside a downloaded dependency and irrelevant to this audit. Report files are under git-ignored analysis/.

## Recommendation

Prefer three small, bounded parser changes over claiming the 246-ID family. Eight mutually exclusive primary candidate IDs are enumerated below, with zero measured recoveries. The first two proposals have specific source-level grammar/routing mismatches. The third is a bounded investigation/implementation candidate with existing typed permission machinery, not a proven one-line fix. Titania is a separate, higher-severity one-ID crash candidate; fixing the panic alone must not count as recovering the card.

The 246 IDs are the exact first overlapping-cause group in overlapping-diagnostic-causes.json. Searching the whole parse_error field finds 325 entries containing this phrase, including differently decorated/fallback causes. These scopes are not interchangeable. The sibling JSON preserves all 246 full entries and exact IDs, recommended cohorts, held cohorts, and full authored bodies/diagnostics. It is a source-only eligibility inventory, not a pass result. Of these 246, only Summer Bloom belongs to the recommended eight-ID union; the other 245 remain uncredited.

## P1. Restore complete source-subject must-be-blocked dispatch (3 IDs)

Highest confidence. clause_primitives.rs:529-532 admits only it/that/they/target to parse_must_be_blocked_if_able_clause, but the existing parser at :885-950 parses complete subjects and lowers to Restriction::MustBeBlocked, with EndOfTurn/EndOfCombat duration. All three retained errors start with `this creature must be blocked`. The registered head list omits `this`. Preserve full consumption and typed source binding; do not special-case card names or relax general object filters.

Full-body obligations:
- Anzrag: preserve becomes-blocked trigger, untap all your creatures, additional combat, activation mana price, and each-combat-through-end-of-turn requirement.
- Glorfindel: preserve scry trigger, choose-one plus shared +1/+1, both modes, and one-blocker maximum. Its two restrictions interact; a fragment-only parser test is insufficient.
- Loathsome Catoblepas: preserve activated price and separate death trigger targeting an opponent creature with -3/-3.

Runtime obligations after authorization: legal-blocker versus no-legal-blocker cases, tapped blockers, another combat in the same turn, end-of-turn cleanup, source identity after zone changes, and Glorfindel combined minimum/maximum-blocker semantics. Do not convert a game-rule requirement into an ability grant. Existing source tests: engine/src/cards/builders/tests/shard_23.rs:295 and frozen_aboleth_glorfindel.rs:97. Runtime pointers: engine/src/game_state.rs:1851,2866,7947; engine/src/effects/restrictions.rs:256.

## P2. Consume optional up-to in temporary additional-land permission (2 IDs)

permission_facts/tagged_surface.rs:864 takes all words between `play` and `additional lands this turn` as the typed value. Thus the authored `up to two/three` reaches complete-value parsing instead of a permission-cap grammar. The static parser already explicitly consumes optional `up to` in grammar/static_keyword_facts/late.rs:470, but its static controller scope must not be reused blindly. permission_helpers.rs:1807-1840 already lowers a temporary numeric cap to additional-land plays until end of turn; leading-may handling explicitly avoids an optional resolution decision (clause_readings/part_2.rs:41-78).

Full-body obligations: Summer Bloom is exactly the temporary cap; Journey of Discovery also has a complete search/reveal/hand/shuffle mode and Entwine {2}{G}. Choosing either mode or entwining must retain order and costs. Runtime obligations: baseline land play plus exactly N extra, zero through N optional actual land plays without a new resolution prompt, stack with other grants, preserve already-used land count, no off-turn land plays, expiration at cleanup. Runtime pointer: engine/src/effects/player/additional_land_plays.rs:8-93.

Do not claim Ghirapur Orrery, Rites of Flourishing, or Storm Cauldron: `each player` recurring static permission needs an explicit recipient-scoped model/consumer. core/src/static_ability_model.rs:1106,6529 exposes AdditionalLandPlays(u32), without a recipient field. Do not claim Nahiri's Lithoforming: its failure is an enclosing turn-scoped enters-tapped replacement swallowing earlier statements, not merely this optional-cap grammar.

## P3. Preserve countered-spell identity through durable free-cast permission (3 IDs)

A bounded three-card permission route repair, lower confidence than P1/P2. Existing machinery explicitly accepts implicit `cast`/`play` leads (grammar/permission_facts/tagged_surface.rs:446-473), has free-cost and ForAsLongAsExiled tails, and runtime durable tagged grants. But clause_primitives.rs:482-486 routes the specialized parser only on you/that/its, while player-may strips its lead in chain_carry/chain_readings.rs:990-1030. There are other unconditional tagged-permission readings (clause_readings/part_1.rs:638-674); inspect route precedence before concluding a head addition suffices. Do not just add cast to generic known verbs.

Full-body obligations: all three counter a target spell, replace its destination only if actually countered, and grant only that resulting exiled object free casting for as long as it remains exiled. Thranduil additionally limits the replacement/grant to a permanent spell; a countered instant must go to the ordinary destination. Spelljack uses play rather than cast and must retain that distinction without granting unrelated land permissions. Kheru has morph and a turned-face-up trigger, which both must remain functional. Source/reference identity must survive spell-to-card zone movement and source departure.

Runtime obligations: counterable/uncounterable targets; permanent/nonpermanent branches; opponent-owned exiled card; source leaves; target leaves/reenters exile; one identified object, not all exile cards; ordinary timing constraints for durable permission; free cost still retains mandatory additional costs; cannot combine two alternative costs; no runtime unimplemented path. Pointers: permission_helpers.rs:2280 onward; grammar/permission_facts/tagged_surface.rs:690-720,767 onward; engine/src/effects/player/grant_play_tagged.rs:234,325,489-509; engine/src/alternative_cast/play_permission.rs.

## Separate high-severity candidate: Titania alternative-cost panic

Titania, Rugged Rumbler is the sole retained compiler-panic ID. Exact failure: `panic: TotalCost::costs called for an alternative cost`. Both complete lines use a disjunction: `As an additional cost to cast this spell, discard a card or pay {2}.` and `Ward—Discard a card or pay {2}.`

core/src/cost_model.rs:590-626 represents All versus OneOf, and costs() intentionally panics on OneOf. This is a representation-contract violation, not unsupported English. Inspect all production callers on these two paths; do not flatten OneOf into conjunction or choose one branch. Candidates include compiler-grammar/src/semantic_line_parsing/lines/lines_resource.rs:79 (unguarded optional-keyword inspection); compiler-lowering/src/lowering_impl/lower/line_lowering.rs:1378,1537 (append to additional costs); lowering_support.rs:6018 (single component materialization). Some apparent callers are already safe: keyword_static/alternative_prices.rs:204 guards as_all before :211, and cost_materialization.rs:1222 is test code. Retained error has no stack trace, so no exact caller is asserted here.

Implementation obligation: preserve disjunction recursively across parsing, materialization, rendering, artifact/verification, and runtime choice. Verify cast additional cost and Ward independently and together; payer is caster for casting cost and targeting opponent for Ward, and each independently chooses discard or {2}. Unaffordable branches must not appear payable, inability/refusal to pay Ward counters only the targeting spell/ability, and ordinary mana cost still applies. No blank cost or empty-vector fallback. A graceful rejection removes a crash but earns zero support credit.

## Why larger groups are held

- No-verb families (63/60/29/25 etc.) are diagnostic-route groups, not mechanics. The widest apparent free-cast group mixes immediate versus persistent permissions, linked source exile, piles/copies, mana-value budgets, collection limits, player ownership, and post-cast facts. Eight named held witnesses are in JSON. Hellcarver has mass sacrifice/discard before casting; Kefka requires per-owner life loss for spells actually cast; Forger has spent-mana provenance and a resolution replacement; Jeleva needs spent-mana X and per-player exile; Kaho binds activated X; Izzet and Shell need source-linked exile after source sacrifice; Planeswalker's Mischief needs delayed return only if not cast. Do not count any as a collateral P3 recovery.
- Attack-if-able also has a head gap (creatures/enchanted omitted at clause_primitives.rs:520), but existing :790-855 lowers to GrantedAbilityAst::MustAttack. Compare temporary_attack_requirement.rs:1-68, which explicitly treats temporary rules as restrictions rather than ability grants. Hold Incite War/Instigator/Nettling Curse until this semantic distinction and target-player binding are resolved. The random-opponent quartet additionally needs selected-player attack constraints; static if-creature-attacks text is simultaneous declaration logic, not a triggered effect.
- Predicate groups contain replacement events (`would`), historical/LKI queries, intervening-if double checks, costs, and true conditions. One safe-looking next investigation is the exact same `a graveyard has twenty or more cards in it` predicate on Visions of Beyond, Jace, and Nightmares and Daydreams (IDs/full bodies in JSON). It must quantify one graveyard, not sum all graveyards, and the latter two require all loyalty/saga bodies and self-replacement semantics. No generic predicate acceptance or broad threshold credit.
- The broad 246-ID cause contains genuine missing mechanics/scopes (drafting, stickers, Attractions, per-player secrets, granted casting methods/costs, prevention/replacement rules, global static recipients, unusual combat and face-up rules). No complete-family parser fallback, textual marker relaxation, or generic success sentinel is acceptable.
- Already active chosen-type, conditional-untap, numeric, speculative-loss diagnostic scopes and held Sinister Concierge are excluded. Linked/supplemental faces are not executed by this retained run; no supplemental face is credited.

## Gate for each implementation

Freeze the exact IDs and full bodies below. First require strict non-lossy no-unimplemented full-card acceptance, then inspect lowered/compiled semantics and execute focused runtime obligations. Rerun baseline controls for rejected near-matches and neighboring forms. Only then count exact-ID recoveries; retain separate failures exposed after the first obstruction. The current baseline remains 1,925 unique unresolved IDs, 899 signatures, and 911 overlapping causes. These three counts are not additive.

### P1_source_must_be_blocked
- Anzrag, the Quake-Mole: `4adcd967-9ff7-4940-b8b9-0c4215bbcb75`
- Glorfindel, Dauntless Rescuer: `93842030-2017-4233-a7ac-7112361c019f`
- Loathsome Catoblepas: `c6f68e4b-af2b-43d5-8052-104defe7f3ec`

### P2_temporary_additional_land_cap
- Summer Bloom: `e5df4597-1647-4ac2-bdb3-a517598d1431`
- Journey of Discovery: `1c586d8a-9d1a-48a7-bb3e-9b2c0c329f8d`

### P3_counter_exile_durable_free_cast
- Spelljack: `7687b2a7-816d-4416-979b-675e35e235fc`
- Thranduil's Decree: `d7cba934-02ad-4677-bb4d-50808b01b4f9`
- Kheru Spellsnatcher: `c01411e0-77b2-4e65-a369-5dbe13745769`

### held_global_land_scope
- Ghirapur Orrery: `c6bb3a58-131a-4d7b-8c07-173a7de23b06`
- Rites of Flourishing: `7080fb43-5d93-41f5-87d3-bc1801805ea0`
- Storm Cauldron: `5a51b168-02d1-4eb4-8ccc-614ab6f6cffa`

### held_attack_rule_semantics
- Incite War: `8a640369-59ab-4506-b473-1f804721414c`
- Instigator: `0fe1a1f7-41c9-415e-a5e3-b5ba0c0271e5`
- Nettling Curse: `93d575b3-0fff-4fc6-85c6-3a8a9d672d6f`

### held_graveyard_count_predicate
- Visions of Beyond: `c68964df-51ae-43b0-abea-74735016c13f`
- Jace, the Perfected Mind: `03dafab4-841f-41cb-8f2e-8188f5177837`
- Nightmares and Daydreams: `0403bfb0-2174-4360-994d-68d8ca96fc55`

### held_freecast_fullbody_dependencies
- Forger's Foundry: `118da256-d1ea-44e8-9026-317e49694d29`
- Hellcarver Demon: `90514aa5-84b6-4be7-b6d0-b05529c97140`
- Izzet Chemister: `e4a3d6f3-36ba-4e6d-a254-351bc8886f20`
- Jeleva, Nephalia's Scourge: `a014f283-c531-415c-ac00-e6773ea5d64d`
- Kaho, Minamo Historian: `395a7bd6-7c2d-448e-842c-ca53256d7008`
- Kefka, Dancing Mad: `fe5690d3-547b-4ce9-8e94-77fdc0e9c5c6`
- Planeswalker's Mischief: `78e87805-19e3-415b-ad0b-3275183d7297`
- Shell of the Last Kappa: `a8b24b38-019c-45e7-a5f4-a1bc3014f7a2`

### separate_high_severity_panic
- Titania, Rugged Rumbler: `e380e37d-926b-4a4b-a275-7844bf4956d5`
