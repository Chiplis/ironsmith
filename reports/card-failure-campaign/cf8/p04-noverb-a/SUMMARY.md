# cf8 / p04-noverb-a — summary

Package: 151 cards failing with "could not find verb in effect clause". Branch cf8/p04-noverb-a, based on origin/main 84ea8b41c. Nothing was built or run (campaign policy). The prebuilt hint binary vanished mid-run; the fallback `.agents/target-score/release` binary is from June, so its results are weak evidence.

Status (after round 6): 74 blocked, 4 covered-by-other-package, 73 source-proposed, 0 untriaged.

## Fixed clusters (source-proposed)
- **elided-damage-recipients** (Tropical Storm, Hail Storm, Neonate's Rush, The Fall of Kroog, Wildfire Howl)
  - The paired damage fanout now reads the second part when it is "N additional damage", a recipient plus an each-set, "its controller", or "that player". It also strips a leading self-replacement "instead".
  - Files: fanout_family.rs, fanout_shapes.rs, back_references.rs, damage.rs.
- **forced-attack-requirements** (Incite War, Instigator, Nettling Curse, Rowan Kenrith)
  - The attack-if-able primitive now accepts `creatures`/`enchanted`/`equipped` as heads.
  - Rowan's "during target player's next turn ... attacks if able" becomes TargetOnly + CantEffect MustAttack starting NextTurn.
  - The engine binds a targeted MustAttack controller at resolution (CR 508.1d).
- **collection-casts** (Hellcarver Demon, Izzet Chemister, Kylox, Doom Reigns Supreme, Kaho, Krang & Shredder, Shell of the Last Kappa, Boiling Rock Rioter, Jeleva, Summon: Esper Valigarmanda, Chandra Ablaze, Forger's Foundry)
  - New collection-cast clause primitive in permission_helpers/collection_casts.rs → ChooseObjects + ForEachTagged CastTagged (CR 608.2g, 607.2a).
- **life-bid-procedure** (Illicit Auction): the existing bundle grammar is now reachable through a five-sentence pair procedure.
- **attacked-turn-permissions** (Boros Strike-Captain, Goblin Researcher, Neriv, Neyali, Robber of the Rich)
  - New AttackedWithTurnCondition on GrantPlayTaggedEffect → GrantSource::EffectDuringTurnsAttackedWith, evaluated from turn-history CreaturesAttackedWith.
  - Changes span core, engine, interpreter, AST, lowering, grammar and renderer.
- **qualified-lure-requirements** (You Look Upon the Tarrasque): "all creatures <filter> able to block X do so" with a qualified blocker filter (CR 509.1c).
- **tapped-attacking-entry-riders** (Grim Reaper): the chain split now keeps "tapped and attacking". Speculative — the root cause is unconfirmed on current main.
- **controller-life-doubling** (Celestial Mantle): "double its controller's life total".
- **source-and-each-exile** (Ajani, Strength of the Pride; Fraying Line): "exile <source> and each <set>" is now split into two exiles.

## Blocked, by missing mechanic
The ledger's gameplay_gap field has the exact gap for each card.
- **Owned elsewhere:**
  - p05 permission variants (14 cards) and attack-toward-player (4)
  - p06 would-X-instead (8)
  - p11 repeat loops (2)
  - p09 predicates (2)
  - p10 library look/rest procedures (6)
  - p12 copy-then-cast and token/characteristic shapes (13)
- **Engine gaps:**
  - loyalty-activation overrides (4)
  - conditional attack requirements (3)
  - two-pile separation (2)
  - defending-player choices (5)
  - divided prevention (2)
  - Blight and Behold costs
  - cloak from hand
  - outside-the-game casting
  - Attractions and stickers
  - targetless life auction
  - per-player targets
  - per-type casting
  - control of a player's next turn
  - chosen attackers
  - block assignment and reassignment
  - labeled piles
  - dynamic activation limit
  - extra-turn restriction
- **Mine and tractable next:**
  - Blech, Tawnos's Tinkering, Brigid (counter/damage target-list grammar)
  - Chaos Moon, Rumbling Ruin (count-then-reference)
  - Liege of the Tangle, Minas Morgul, Ultima (for-as-long-as-counter animation)
  - Storm of Souls
  - Oskar, Sproutback Trudge, Syrix (graveyard self/trigger casts)
  - Crabomination
  - Meddle, Quicksilver Dragon
  - remaining singletons

## Risks
- **attacked-turn-permissions overlaps p05's permission area:**
  - it adds a field to GrantPlayTaggedEffect (serde-default)
  - it adds a GrantSource/GrantLifetime variant
  - it adds an AST field on GrantPlayTaggedForAsLongAsExiled
  - expect conflicts in grant_play_tagged.rs, grant_registry.rs, subject_verb_middle.rs and the text guard lines
- **The collection-cast primitive** defers to cast-or-play-tagged-clause whenever that reader returns Some. Another package adding cast readings with heads you/cast could still create registry ambiguity.
- **Unverified reference resolution:**
  - the "exiled this way" alias
  - untyped "them" pools in ChooseObjects
  - Celestial Mantle's "its"
- **Grim Reaper and Tidebinder Mage** need re-measurement on a current build.

## Round 2 (after coordinator review)
- **collection-cast ownership.** No two registry readers can claim the same clause any more:
  - the primitive head is narrowed to the imperative `cast`;
  - "you may cast ..." is reached only through cast-or-play-tagged-clause's final fallback;
  - the reader no longer calls back into cast-or-play.
- **color-disjunction-targets** (Tidebinder Mage). Root cause, read from source: the coordination recognizer treated the `or` in "red or green creature" as an effect boundary. Fix: Or between two color words is never coordination.
- **serial-object-qualifiers** (Blech, Tawnos's Tinkering, Brigid). The coordination recognizer kept these together as one operand:
  - a serial creature-type list after each/all;
  - a card-type list of a put-counters operand;
  - attacking-or-blocking.
- **behold-creature-noun** (Hulk's Thunderclap). The behold cost segment now accepts the trailing "creature" after a creature type; the existing optional behold reader then takes the line.
- **cloak-from-hand** (Vannifar). New keyword shape: ChooseObjects in your hand, then the existing cloak put-onto-battlefield (CR 701.58a).
- **outside-game-casts** (Spawnsire of Ulamog). The collection-cast reader now accepts "from among cards you own outside the game", using Zone::OutsideGame — the same sideboard zone p11 uses.
- **loyalty-activation-allowances** (Jace's Machinations, Kaito, The Chain Veil, Urza Assembles the Titans).
  - New core/engine effect GrantLoyaltyActivationAllowanceEffect:
    - scope: Source, EachControlledPlaneswalkerNow, or ControlledPlaneswalkers{subtype};
    - allowance: ExtraActivation or InstantSpeed;
    - stored in named turn counters.
  - The legal-action loyalty rule (CR 606.3) now allows 1 + allowances, plus instant-speed timing.
  - Plumbing: decoder registry, interpreter, materializer, renderer, AST variant and lowering, plus a chain-entry grammar module.
- **defending-player-choices** (Crashing Boars, Drana). New object-choice actor: "defending player" → PlayerAst::Defending.
- **for-as-long-as-counter** (carry prefix only). A leading "For as long as that <noun> has a <kind> counter on it, ..." is carried as Until::ForAsLongAs(affected_object_has_counter), the same predicate as p05's suffix. Liege, Minas Morgul and Ultima remain blocked on p05's copula/suffix readers (and Ultima on land-type loss).
- **Ownership updates.**
  - Divided prevention: Angel of Salvation and Serra's Hymn → p03.
  - Two-pile separation: Brilliant Ultimatum, Jace AoT and Celestial Toymaker → p01.
- **Blocked with precise design notes (mine):**
  - conditional attack requirements: needs a joint requirement-maximization search over the declared set;
  - count-then-reference;
  - returned-set copula with keywords;
  - graveyard self-casts;
  - Crabomination's three-part exile tag;
  - retarget with a single-target condition;
  - Pain's Reward;
  - Blight X;
  - Stromgald Spy;
  - The Seventh Doctor.
- **Risk:** the binary is gone, so nothing in round 2 was even hint-checked; every proposal is from source reading only.

## Round 3 (on cf8/integration)
Everything below is source-only (no build, no tests run).
- **counter-linked animations** (Liege of the Tangle, Minas Morgul, Ultima).
  - The copular animation shape accepts a trailing counter-linked duration (p05's suffix reader) without "in addition to"; p05's become branch owns the duration. "Those lands"/"those artifacts" are tagged become subjects.
  - Minas Morgul: my carried prefix plus p05's contracted copula.
  - Ultima: new "loses all <family> types and abilities" reader = the ordinary remove-all-abilities instruction plus RemoveAllSubtypesOfFamily on the same target (CR 205.3, 613.1d/f); the carried ForAsLongAs duration applies to both and to the "has {T}: Add {C}" grant.
- **attacked-turn permission reconciled with p05** (one implementation). The engine was already one shared GrantPlayTaggedEffect (p05 added no competing grant). The grammar now reads the permission body through the shared `parse_permission_clause_spec`, and the retag sites that rebuild GrantPlayTaggedForAsLongAsExiled (exile-top bundle, ordered control flow, tax rebuild, looked partitions) keep or refuse the turn scope instead of silently dropping it.
- **conditional attack requirements** (Viashino Bey, War's Toll, Magnetic Web; CR 508.1d).
  - New static payload/id `ConditionalAttackRequirement{trigger, required}` (appended for wire stability) and a static line reader "If <creature> attacks, all <creatures> attack if able".
  - Design: the proposed declaration fixes which conditional requirements exist (a matching creature attacks in it). Every alternative declaration is measured against that same fixed set, so the conditional terms are simply added to the existing independent per-attacker scores; the search and the unconditional scoring are unchanged. Bey never has to attack, but once it does the rest must; declaring nobody stays legal.
- **implied-creature keyword animation** (Storm of Souls): "a 1/1 Spirit with flying in addition to its other types".
- **single-target retarget** (Meddle, Quicksilver Dragon): TargetOnly(any spell) + Conditional(target has exactly one target matching X) + retarget with require_change and a new-target restriction. The AST RetargetStackObject gained `new_target_restriction`, lowered to the engine's existing NewTargetRestriction::Object.
- **graveyard resolution casts** (Sproutback Trudge, Syrix, Oskar): "you may cast this card/creature/it from your graveyard"; the SourceObject tag lowers to CastSourceEffect, and "cast this card from your graveyard" makes the trigger function from the graveyard (CR 113.6k).
- **Blight X** (Soul Immolation): "blight X" in both cost grammars, PutCounters(Value::X) with the Blight completion action reporting `references_cost_x` so the cast flow announces X, plus a generic "X can't be greater than <aggregate>" reader → ThisSpellXMaximum.
- **defending-player guess** (The Seventh Doctor): a four-sentence procedure → ChooseObjects(hand card) + ChooseOneOf by the defending player, each answer checking ManaValueOf(card) > N; wrong guess → may cast free, else investigate.
- **Still blocked (re-triaged with exact sites):**
  - Stromgald Spy: a player-scoped public hand is a privacy/mental-poker protocol change (the reveal statics are UI-layer id checks), not a parser gap.
  - Chaos Moon, Rumbling Ruin: need a value frozen at resolution and substituted into a stored restriction filter (CR 608.2h).
  - Crabomination: the ExiledThisWay alias keeps only the last exile producer's tag.

### Round 3 risks
- New enum variants: StaticAbilityId/StaticAbilityPayload::ConditionalAttackRequirement (appended), the ActivationCostSegmentCst/CompilerCost/MaterializationCost `Blight{count, x}` field, the AST RetargetStackObject `new_target_restriction` field, and KeywordMechanicShape::Blight `amount: Option<u32>`. Sibling arms were added where exhaustive (core map, interpreter, text-change statics, combat text renderer, Debug).
- Combat: the conditional set is computed only when requirements are enforced, so AI proposals that attack with Bey alone are now rejected; check the AI declaration fallback.
- Unverified bindings: Oskar's "it" relies on reference resolution binding It to the discarded (triggering) card; Liege's "each of those lands" → It; whether Soul Immolation's trailing X-bound sentence reaches the early static reader; IfResult(DidNot) after May inside a ChooseOneOf mode.
- Magnetic Web's block line and War's Toll's mana line were not re-verified.

## Round 4
- **Attack-declaration safety.** Every automatic producer was audited:
  - trait default and AutoPass/SelectFirst declare nothing;
  - the Minimum fallback, the wasm replay fallback and the UI auto-declare (`defaultOpponentAttackerDeclarations`) declare exactly the `must_attack` creatures;
  - the Maximum fallback declares every legal attacker;
  - the UI and network paths submit a human/peer proposal validated by the same `prepare_attacker_declarations`.

  A declaration of nobody is always legal for conditional requirements, so only one path could get stuck: a creature that must attack and also meets a conditional trigger (a goaded or "attacks each combat" Viashino Bey). Its forced-only declaration would be rejected. Fix: `compute_legal_attackers_with_view` now propagates conditional requirements into `must_attack` (fixpoint), so every forced-only default is legal. Gameplay test: the forced-only and attack-with-everything declarations both complete declare attackers with Bey, including a forced Bey.
- **Counted numbers (Rumbling Ruin).** "Count the number of X. ... that number ..." substitutes the counted quantity into the later sentences (procedure shapes for 2/3 sentences plus a bundle reading). Resolving restrictions now freeze game-wide comparison quantities into fixed numbers (CR 608.2h); the affected creatures' power stays live.
- **Chaos Moon** stays blocked. The count/parity half now works through the existing CountParity predicate. Still missing: the coordinated "until end of turn, <anthem> and <temporary mana trigger / mana rewrite>" bodies (CR 605.1b), detailed in the ledger.
- **Crabomination.** "Exiled this way" is now the union of consecutive exile producers in one effect list; with more than one producer, the pool filter becomes `any_of` over the producers' tags.
- **Stromgald Spy** stays blocked, with the exact requirement. The only hand-reveal state the mental-poker view honours is controller-relative battlefield static ids, read by `hand_revealed_by_static_ability` in the wasm crate. The Opponents id would over-reveal in multiplayer. It needs a resolved player-scoped designation read by both wasm copies plus crypto public-opening scenarios.
- **Bindings verified or fixed:**
  - Oskar's "it": verified. The discard trigger seeds Triggering.
  - Liege's "each of those lands": verified as a tagged subject.
  - Soul Immolation: fixed. The X bound was swallowed by the additional-cost effect parse; the line now yields Multiple[AdditionalCost, ThisSpellXMaximum].
  - The Seventh Doctor's "if you don't": fixed. It is now IfEffectDidNotHappen, bound to the exact cast.
- **Risks:**
  - The `exiled this way` union changes the alias for any card with consecutive exile producers before a "this way" reference. The change is intended for one instruction, but it also applies across adjacent sentences.
  - Freezing restriction comparisons affects every resolving restriction whose power/toughness/mana-value comparison uses a game-wide count. This is rules-correct (CR 608.2h) but newly enforced.

## Round 5
- **"Exiled this way" union narrowed to one instruction.** A single-sentence chain with one exile verb that lowered into several exile actions (Crabomination's comma list) is grouped as one `EffectAst::Coordinated` clause. The resolver unions the alias only among a Coordinated clause's members. Separate sentences keep binding to the most recent exile (negative test).
- **Chaos Moon** (now source-proposed).
  - Reader for "until end of turn, <change> and (whenever a player taps <land> for mana, ... | if a player taps <land> for mana, that <land> produces ... instead)". The trigger half is a DelayedTriggerThisTurn; the rewrite half is a RegisterManaRewrite until end of turn. `mana_rewrite_filtered_source` accepts "that <noun> produces".
  - **Engine fix (CR 605.1b):** a manually activated mana ability queued its ManaAdded events without delayed triggers (`priority_mana.rs` → `queue_triggers_for_events`). Temporary "whenever a player taps ... for mana" triggers therefore never fired outside the auto-payment planner, which drains pending events with delayed triggers. Those events are now queued including delayed triggers. Test: a Bubbling-Muck-style trigger adds the extra {B} immediately, without the stack.
- **Stromgald Spy** left blocked; the requirement write-up stands.
- **Blocked re-check, biggest shared gaps first:**
  - **copy-card-then-cast** (Mnemonic Deluge, Arcane Bombardment, Chandra, Pyromaster): new pair reader "copy <cards> [N times] / choose <card> exiled this way and copy it N times" + "you may cast [any number of] the copies [free]" → per card, N optional copy casts (CR 707.12). The other 7 copy cards are refined in the ledger (each needs a distinct extra piece).
  - **would-x-instead-on-cast** (Sorcerous Squall): the existing graveyard-cast/exile-replacement procedure now reads a leading "<action>, then" clause. Kylox needs a library-bottom destination in RegisterFutureZoneReplacement; Gale, Mavinda and Bösium Strip are refined.
  - **covered-by-other-package** (new status; regression tests only, no package code):
    - divided prevention via p03: Angel of Salvation, Serra's Hymn;
    - main's exact counter/exile permission: Thranduil's Decree, Kheru Spellsnatcher.
  - Repeat loops (p11's RepeatProcess exists) and attack-toward-player (p05's MustAttackPlayer): the remaining cards need more than the shared piece; the ledger says what.
- **Risks:**
  - Grouping one exile verb's actions into a Coordinated clause changes AST shape for those sentences. It applies only when at least two of the produced effects are exiles, so bundle readers matching flat "exile + other" lists are unaffected.
  - Delayed triggers now observe manual mana activations; any existing delayed trigger keyed on ManaAdded will fire there too, as the rules require.

## Round 6
- **Verifications (by reading):**
  - **Crabomination: was wrong, now fixed.** The comma list goes through the coordination recognizer with object omission, so it reaches the resolver as ONE conjunctive Coordination, not a flat list, and the round-5 grouping never fired. The resolver now unions "exiled this way" across a conjunctive Coordination's members (beside the existing created-token union). Separate sentences stay last-wins.
  - **Arcane Bombardment: half wrong, now fixed.** ExileEffect does link every exiled card to the source. But inside the same resolution, the source-exiled TAG is replaced by this resolution's exile only; the full linked set is rebuilt only in `filter_context`. "Each card exiled with <source>" now iterates a ForEachObject filter over the linked set.
  - **Count procedure and Chaos Moon reader in trigger bodies: confirmed.** Trigger bodies go through `parse_effect_sentences_lexed`, whose document-program probe and bundle readings both include the new shapes.
- **Kylox's Voltstrider:** p01's follow-the-card stack replacement with a library-bottom destination (b204e9960 / 5af14a45a) is NOT in this tree. Marked; the ledger has the wiring once merged.
- **Copy cards:**
  - Reversal of Fortune: an optional copy of a revealed hand card, then an optional cast of the copy.
  - Bloodthirsty Adversary: the copy closes a longer reflexive instruction, so the casts go inside the "When you pay this cost one or more times" result.
  - Zethi: "copy each exiled card <qualifier>" iterates the described exiled cards.
  - Still blocked, each with its exact piece: Zemo (cast cap + Boast mana-symbol cost), Tamiyo I–III (pairwise shares-a-card-type over the milled cards), Spellweaver Volute (Aura on a graveyard card), Myra (Attractions).
- **Other shared/singleton mechanisms:**
  - Withering Wisps: appended `Condition::MaxActivationsPerTurnCount(AnthemCountExpression)`, with a reader for "activate no more times each turn than the number of <objects>".
  - Dominaria's Judgment: per-quality conditional protection grants.
  - Shay Cormac: bare "protection"/"ward" in a lost-ability list removes the whole static family.
  - Exponential Growth: "<doubling> X times" via RepeatEffects.
- **Risks:**
  - New appended Condition variant (core, PredicateAst, resolve, engine eval, dependency, text-change, renderers).
  - The exiled-this-way union now also applies to any conjunctive coordination with two or more exile members.
  - The protection-list reader is a chain hook that requires two or more "from <quality> if <condition>" items.
