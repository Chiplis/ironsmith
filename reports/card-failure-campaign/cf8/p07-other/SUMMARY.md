# cf8 / p07-other — fixer summary

Branch `cf8/p07-other` (worktree `ironsmith-cf8-p07`), off origin/main 84ea8b41c.
All work is **source-only and unbuilt/unrun** (campaign policy). Diagnostics came
from the read-only prebuilt `compile_oracle_text` (the Oct 8 `b4/base13` simdev
binary reproduces the baseline errors; the June `target-score` binary does not and
was only used for early probes, later re-checked on base13).

## Counts
- 156 cards: 52 `source-proposed`, 1 `already-on-main`, 1 `semantic-fix-collateral`, 102 `blocked` (plus Balance, a non-package `semantic-fix-collateral`).

## Clusters fixed (general mechanisms)

| cluster | cards | root cause | general fix |
|---|---|---|---|
| modal-trigger-intervening-if | Tetsuo, Vision, Wardens of the Cycle | `<trigger>, if <pred>, choose one —` fed the `if` clause to the prefix-effect parser ("missing comma in if clause") | `ParsedModalHeader.intervening_if`; the modal header lifts a fully-modeled pre-choice `if` predicate into the triggered ability's intervening-if (CR 603.4); lowering passes it to `assemble_parsed_triggered_ability` |
| earthbend-where-x | Beifong's Bounty Hunters, Bumi's Feast Lecture, The Boulder, The Legend of Kyoshi, Toph | earthbend count was a `u32` | `EarthbendEffect.counters: Value` (resolved once at resolution, CR 107.3a); AST `Earthbend { counters: Value }`; grammar reads `earthbend X`; where-X binding (AnyEffect) and modal header X substitute it; lowering binds trigger references via `resolve_value_it_tag`; renderer prints `Earthbend X, where X is …` |
| conditional-counter | Corrupted Resolve | spell-context poisoned predicate required "its controller poisoned" | strip the copula `is` |
| counter-removal-references | Heirloom Mirror, Smoldering Egg, Strixhaven Stadium, Lightning Coils | "remove them / them all / all of them from it" unparsed; "if it has N or more X counters" (pronoun holder) not an antecedent | remove-shape accepts those surfaces as the unresolved all-of-them removal; `predicate_source_counter_antecedent` also reads source `CountersOn` threshold comparisons; new `bind_condition_it_counter_antecedent_in_effects` binds pronoun-holder thresholds in conditional and intervening-if bodies (unbound uses still fail closed) |
| counter-removal-any-number | Galloping Lizrog, Rhys | "remove any number of [kind] counters" had no amount | shape `any_number`; amount = CountersOn(holder, kind) (rebound to the target / from-among set); renderer prints "any number of" |
| saga-modal-chapter | Life of Toshiro Umezawa, Summon: Magus Sisters | `I, II — Choose one —` + bullets read as a plain chapter with effect "choose one" | `RecognizedModalBlock/RewriteModalBlock.saga_chapters`; saga chapter + bullets → modal block whose header trigger is `TriggerSpec::SagaChapter` |
| derived-escape-grant | Confession Dial | no escape grant reading | new pair shape (`gains escape until end of turn` + `The escape cost is equal to its mana cost plus exile N other cards from your graveyard`) → `DerivedAlternativeCast::EscapeFromCardManaCost{N}` |
| dynamic-count-quantities | Lifeblood Hydra, Blim, Jagged Poppet | bare "gain/lose life and <verb> … equal to X"; discard "cards equal to X"; "equal to the damage"; **silent miscompile**: "permanents they control but don't own" → owner NotYou | shared terminal equal-to amount restated on the life clause before chain splitting; discard reuses the draw equal-to count grammar; draw/discard equal-to reads "the/that damage" as the event amount; elided "they" subject keeps owner ≠ that player |
| trailing-if-predicates | Ludevic (Leaves from the Vine already handled by merged ae3fc7ff3) | life-change subject "a player other than you" | `PlayerFilter::NotYou` subject → `LifeLostThisTurn(NotYou) >= 1` |
| for-each-quantities | Might of the Nephilim, Whispering Specter | for-each vocabulary lacked "of its colors" / "poison counter(s) they have" | `ColorsOf(it)`; `PlayerCounters(IteratedPlayer, kind)` |
| negated-participant-control | Thornbow Archer | "each opponent who doesn't control X <effect>" routed to the "for each opponent who doesn't" follow-up | new participant reading negates the positive relative-control predicate |
| gendered-counter-references | Red Hulk | "counters on him/her" unparsed | source counter references accept him/her |
| discard-the-rest | Monomania, Breakthrough, Flay | "and discard(s) the rest"; "discards another card" | `RestActionShape::Discard` (both rest grammars) → discard the complement of the chosen set; discard count reads "another" as 1 |
| explicit-ability-choice-lists | Atraxa's Skitterfang, Hunter's Axe | inner-chain coordination split "your choice of A, B, or C" into clauses ("missing life gain amount") | skip coordination for an explicit choice list with no other conjunction |

Additional correctness fix (no package card depends on it): the quantified
graveyard predicate read "if a <card> is in your graveyard" as `Count == 1`; an
indefinite article now means `>= 1` (only Bazaar of Wonders uses the wording, and
it is blocked on p09's same-name predicate).

## Round 3 (on cf8/integration)
| cluster | cards | fix |
|---|---|---|
| lose-all-player-counters | Final Act | '<player> loses all [kind] counters' → existing counter removal with a player holder: exact `PlayerCounters(kind)` or all-kinds `CountersOn(EachPlayer)`; the iterated participant is `IteratedPlayer` (fails closed outside a loop). Leeches/Survivor's Med Kit parse the removal but stay blocked on other clauses. |
| bloodthirst-x | Petrified Wood-Kin | new `KeywordAction::BloodthirstX` (all KeywordAction match sites updated) lowering to the existing enters-with-counters static with `DamageDealtToPlayersThisTurn(Opponent)` (CR 702.54c). |

Queue items re-checked against the merged tree and left blocked with an owner:
derived jump-start/harmonize/encore grants (no shared casting-keyword grant path exists yet; needs p02),
partial prevention shields, protection from a player, chosen-type landwalk grants, 'the same way'
Balance repetition, dynamic bushido and self-'has soulshift X' — all built in Round 4 below.

## Round 4 (mechanisms built on cf8/integration)
| cluster | cards | mechanism |
|---|---|---|
| protection-from-player | Eon Frolicker, Noble Heritage | `protection from that player` → `ProtectionFromFilter(controlled by that player)`; player grants compose the existing can't-be-targeted-from restriction + damage prevention (as `GrantAbilitiesTarget` already does for players); `you and <permanents you control> gain ...` subject (also guards the coordinated-leading-duration split that dropped "you"); engine binds Target/AliasedTarget/IteratedPlayer controller references to the concrete player as the shield/restriction/granted protection is created (`effects/player_reference_binding.rs`, CR 702.16k, 611.2c) |
| chosen-landwalk-grant | Illusionary Presence, Barbarian Guides, Giant Slug | `LandwalkKind::ChosenType { snow }` ("[snow] landwalk of the chosen type"), materialized at grant resolution from the resolving source's chosen land type (CR 702.14a); `ChooseLandTypeEffect.basic_only` ("choose a basic land type"); "until the end of that turn" duration |
| same-way-repetition | Balancing Act, Magus of the Balance, Restore Balance (+ Balance collateral) | pair procedure `same_way_balance`: the Balance head plus "<players> <discard cards / sacrifice X> [and ...] the same way" → one choose-then-rest step per domain in written order; new `Value::LeastCount(filter)` (fewest per player, partitioned by controller or owner) |
| dynamic-keyword-amount | Fumiko the Lowblood, Kodama of the Center Tree | defined-X keyword reading for `[<self> has] bushido/soulshift X, where X is ...` in the keyword-line family (after self-name normalization); new `KeywordAction::BushidoValue(Value)` (CR 702.45a) |
| partial-prevention-shield | Dark Sphere, Forcefield | `NextTimeDamagePreventionPortion` (All / HalfRoundedDown / AllBut(n)) + `combat_only` on the one-shot next-time prevention effect, applied with the existing `PreventHalfDamage` / `PreventDamageByRule(AllBut)` actions (CR 615.1, 615.7) |
| counter-removal-followups | Leeches, Survivor's Med Kit | reference resolution tracks a counter removal's player holder as "that player"; an imperative "Sacrifice this <permanent>" is the controller's (CR 608.2c) instead of inheriting the previous sentence's target player |

Re-checked "needs X (pNN)" entries (same-name, random opponent, granted casting keywords, damage
multipliers, draw-from-bottom): none of those owned mechanisms exists in the merged tree yet.

Round 4 risk notes:
- `Value::LeastCount` is a new `Value` variant; every grouped `GreatestCount` arm got a sibling arm
  (engine, resolve, lowering, text, core); any branch adding a new exhaustive `Value` match will need it.
- `LandwalkKind::ChosenType` (core and engine enums), `KeywordAction::BushidoValue`,
  `ChooseLandTypeEffect.basic_only`, `PreventNextTimeDamageEffect.{portion,combat_only}` (core with
  serde defaults, engine, AST variant) change shared types.
- `register_prevention_shield` now binds player references in `from_source` for every shield; this
  only replaces Target/AliasedTarget/IteratedPlayer controller/owner references that resolve.
- Temporary player protection (composed restriction + prevention) does not detach an opposing Curse
  already attached (CR 702.16e / 704.5m), matching the existing player-grant composition.
- The imperative source-sacrifice rebinding changes the subject of every "Sacrifice this <permanent>."
  sentence from Implicit to You; base13 rejected all such sentences after a target-player sentence.

## Round 5
| item | cards | change |
|---|---|---|
| player protection removes Auras | Eon Frolicker, Noble Heritage (and every existing composed player-protection grant) | `player_has_protection_from_object` recognizes the composed pair (can't-be-targeted-from restriction + all-damage shield from the same source and quality) as protection, so an Aura/Curse with the quality can't enchant the player and falls off as a state-based action (CR 702.16e, 704.5m). Gameplay test with two Curses. |
| landwalk per sacrificed land type | Excavator | `LandwalkKind::SacrificedLandTypes`, expanded by the resolving grant (`ApplyContinuousEffect`, `GrantAbilitiesTarget`) into one landwalk per land type of the sacrifice-cost snapshot (CR 702.14a, LKI) |
| scoped damage multipliers | Jeska, Lightning, Impulsive Maneuvers | extends the existing `RegisterDamageMultiplierEffect`: leading "until end of turn / until your next turn," scope, "the next time ... it deals double that damage instead" (one-shot), referenced source ("that creature"), recipients "one of your opponents" and "that player or a permanent that player controls"; lowering resolves the references and the engine locks the named object/player at registration |
| same-mana-value conditional counter | Hisoka, Counterbalance | "counter target spell / that spell if it has the same mana value as the discarded / revealed card" → resolution-time conditional (`TargetMatches` / triggering `TaggedMatches` with `SameManaValueAsTagged`) |

Assumption checks (by reading):
- Leeches' "that much": confirmed. The removal lowers through `Effect::with_id` and sets `last_effect_id`; `resolve_value_it_tag` turns the event amount into `EffectValue(last id)`.
- Keyword-line self name: the line family runs on unnormalized line tokens (the firebending reader takes the card name separately); the defined-X reading normalizes the self name first, so both forms are handled.
- Forcefield "unblocked creature of your choice": WRONG — the damage-source descriptor silently skipped "unblocked" (any creature could be chosen). Fixed: unblocked/attacking/blocking/tapped/untapped are part of the descriptor. Only Forcefield uses such a source choice.

Re-check of owned mechanisms in the tree: same-name, random opponent, granted casting keywords,
draw-from-bottom, copy exceptions and static half-damage replacement are still absent; Ghosts of the
Innocent and Goblin Charbelcher stay blocked (no halving damage modification / conditional self-doubling).

## Files touched (main)
grammar: `grammar/modal_support.rs`, `grammar/structure.rs`, `grammar/conditions.rs`,
`grammar/effects/remove_destroy_shapes.rs`(+tests), `grammar/effects/generic_sequence_shapes.rs`,
`grammar/effects/sacrifice_discard_shapes/discard.rs`, `grammar/effects/chain_carry/carry_facts.rs`,
`grammar/effects/linked_clauses/residual.rs`, `grammar/filters/predicate_phrases/advanced.rs`,
`grammar/filters/reference_tag_stage/reference_tag_stage_reference.rs`,
`grammar/shared_util/count_shapes/{count_shapes_core,count_shapes_counter}.rs`,
`grammar/shared_util/value_expr/value_expr_counter.rs`, `document_parser/{mod,block_parsing}.rs`,
`recognized_document.rs`, `ir.rs`, `semantic_assembly.rs`, `semantic_line_parsing/lines/lines_choice.rs`,
`effect_sentences/{chain_carry.rs, remove_destroy.rs, search_library/core.rs, pair_procedure/kinds.rs,
sacrifice_discard/library.rs, verb_handlers/zone_move_verbs.rs, dispatch_entry.rs,
dispatch_inner/labeled_prefixes/reference.rs, sequence_rules/.../reference_linked.rs,
for_each_helpers/opponent_iteration/for_each_opponent_readings.rs}`;
semantic: `model_impl/compiler_semantic.rs`, `model_impl/ast/{effects.rs, actions/keyword_actions.rs}`,
`condition_antecedent.rs`; lowering: `lower/modal_and_level_lowering.rs`,
`compile_support/effect_dispatch/subject_verb_early.rs`, `compile_support/effect_handlers.rs`,
`lowering_support.rs`; core: `effect.rs` (EarthbendEffect only); engine:
`effects/permanents/earthbend.rs`; text: `render_effects/effect_impl/{late,early}.rs`.
Tests: `crates/ironsmith-compiler-runtime/tests/{modal_trigger_intervening_if, earthbend_where_x,
spell_controller_poisoned_counter, counter_removal_references, saga_modal_chapters,
derived_escape_grants, dynamic_count_quantities, other_player_life_loss_predicate,
for_each_quantities, negated_participant_control, gendered_counter_references, discard_the_rest,
discard_another_card, explicit_ability_choice_lists}.rs` + `p07_support/mod.rs` and matching
`fixtures/*.json.fixture`.

## Risk notes
- **EarthbendEffect.counters u32 → Value** changes the serialized artifact payload shape of
  `EarthbendEffect` (a number becomes a Value object); every compiled artifact containing
  earthbend/awaken must be rebaked, and any artifact schema/version gate may need a bump.
  All `EarthbendEffect::new/awaken` callers keep compiling via `impl Into<Value>`.
- `ParsedModalHeader`, `RecognizedModalBlock`, `RewriteModalBlock`, `RemoveClauseShape::Counters`
  and both `RestActionShape` enums gained fields/variants; all constructors/exhaustive matches in
  the tree were updated, but sibling branches adding constructors will conflict.
- `parse_modeled_predicate` became `pub(crate)`.
- The coordination bypass for explicit "your choice of" lists and the shared life equal-to
  rewrite are token-level pre-passes in `chain_carry.rs` (a hot shared file) — expect merge
  friction with other packages editing it.
- Several proposals rely on base13 probes of paraphrased text (noted per card in the ledger);
  none are build-verified.

## Blocked, grouped by missing mechanic
- **cast-time-snapshot** (2): Flame Discharge — 'If you controlled a modified creature as you cast this spell' — as-you-cast snapshot (owned by p01); Dragon's Fire — 'If you revealed a Dragon card or chose a Dragon as you cast this spell, ... damage equal to the power of that card or creature instead' — optional reveal/choose cost snapshot (owned by p01)
- **difference-quantity** (2): Spiteful Repossession — 'deals damage to each opponent who controls more lands than you equal to the difference' — per-participant difference value; Tales of the Ancestors — 'Each player with fewer cards in hand than the player with the most ... draws cards equal to the difference' — per-player difference value
- **dynamic-count-quantities** (2): Vanish into Memory — 'discard cards equal to that creature's toughness' inside the delayed return: referent (exiled card LKI vs returned permanent) must be pinned by the delayed-trigger linkage; discard equal-to grammar itself now supported; Felothar the Steadfast — ', then <verb> cards equal to its <stat>' after a 'the sacrificed creature's toughness' amount fails the comma-then chain reader (base13: even 'then draw cards equal to its power' fails); discard 'equal to' count itself now supported
- **event-amount-that-much** (2): Imminent Doom — 'deals that much damage' where 'that much' = the cast spell's mana value matched against doom counters; event-derived amount has no compatible trigger value; Magnanimous Magistrate — 'if its mana value was 1 or greater, you may remove that many reprieve counters' — 'that many' = dying creature's mana value
- **kicked-entry-granted-trigger** (2): Necravolver — 'it enters with a +1/+1 counter on it and with "Whenever this creature deals damage, you gain that much life."' — ETB counters plus a granted quoted triggered ability in a kicker-conditional entry static (the quoted trigger alone compiles); Rakavolver — 'it enters with two +1/+1 counters on it and with "Whenever this creature deals damage, you gain that much life."' — same as Necravolver
- **mass-attach** (2): Ardenn, Intrepid Archaeologist — 'attach any number of Auras and Equipment you control to target permanent or player' — multi-object attach effect with legality per attachment (CR 701.3) not modeled; Heavenly Blademaster — 'attach any number of Auras and Equipment you control to it' — multi-object attach effect not modeled
- **player-or-their-planeswalker** (2): Curse of the Pierced Heart — 'deals 1 damage to that player or a planeswalker that player controls' — recipient choice between a player and one of their planeswalkers; Vial Smasher the Fierce — 'choose an opponent at random ... deals damage ... to that player or a planeswalker that player controls' — random opponent (owned by p01) plus player-or-planeswalker recipient choice
- **same-name-return** (2): Bloodbond March — 'returns all cards with the same name as that spell from their graveyard' — same-name predicate (owned by p09); Rat King, Verminister — 'Return target creature card and all other cards with the same name as that card' — same-name predicate (owned by p09)
- **stickers** (2): Roxi, Publicist to the Stars — art sticker mechanics (Unfinity stickers) are not modeled; _____ _____ _____ Trespasser — name sticker mechanics (Unfinity stickers) are not modeled
- **attack-history-predicate** (1): Firemane Commando — 'they draw a card if none of those creatures attacked you' — predicate over the triggering attackers' defenders
- **attacked-recipient-damage** (1): Fathom Fleet Swordjack — 'deals damage to the player or planeswalker it's attacking equal to ...' — the attacked recipient as a damage target (the 'defending player' form compiles)
- **bloodthirst-x** (1): Indoraptor, the Perfect Hybrid — Bloodthirst X now supported; remaining: 'choose an opponent at random. Indoraptor deals damage equal to its power to that player unless they sacrifice a nontoken creature of their choice' (random opponent, owned by p01; unless-sacrifice by the damaged player)
- **casualty-copy-exception** (1): Ob Nixilis, the Adversary — Casualty X with copy exceptions 'isn't legendary and has starting loyalty X' (copy of a permanent spell with modified loyalty/supertype; copy mechanics owned by p12)
- **choice-of-two-counters** (1): Grimdancer — 'enters with your choice of two different counters on it from among menace, deathtouch, and lifelink' — keyword counters choice
- **choose-one-exiled** (1): Chandra, Flameshaper — 'Exile the top three cards ... Choose one. You may play that card this turn.' — choose one among exiled cards + play permission (owned by p05)
- **chosen-number-cost** (1): Liquid Fire — 'As an additional cost ..., choose a number between 0 and 5' then 'X damage ... and 5 minus X damage' — chosen-number additional cost bound to X
- **colors-of-mana-spent-on-spell** (1): Magmablood Archaic — 'for each color of mana spent to cast that spell' — converge-style count for the triggering spell
- **conditional-counter** (1): Bazaar of Wonders — needs 'a card with the same name [as the cast spell] is in a graveyard or a nontoken permanent with the same name is on the battlefield' same-name predicate (owned by p09)
- **conditional-self-flashback** (1): Viral Spawning — 'As long as an opponent has three or more poison counters and this card is in your graveyard, it has flashback {2}{G}' — conditional self-granted flashback
- **copy-with-ability-exception** (1): Aurora Shifter — 'becomes a copy of another target creature you control, except it has this ability and "..."' — copy exception (owned by p12); 'you get that many {E}' itself compiles
- **counter-kind-choice** (1): Dismantle — 'put that many +1/+1 counters or charge counters on an artifact you control' — counter kind choice with prior-count amount
- **counter-removal-any-number** (1): Tetravus — token grant 'They each have flying and "This token can't be enchanted."' (unsupported negated restriction tail for granted quoted ability) and 'exile any number of tokens created with this creature' provenance tracking
- **counter-replacement** (1): Desertion — 'If an artifact or creature spell is countered this way, put that card onto the battlefield under your control instead of into its owner's graveyard' — resolving-spell destination replacement (owned by p01)
- **cumulative-upkeep-paid-mana** (1): Balduvian Fallen — 'gets +1/+0 for each {B} or {R} spent this way' — cumulative-upkeep payment mana-color count
- **damage-dealt-to-target-this-turn** (1): Knollspine Dragon — 'draw cards equal to the damage dealt to target opponent this turn' — per-target damage-history value with its own target declaration
- **delayed-copy-token** (1): Esoteric Duplicator — 'If you do, at the beginning of the next end step, create a token that's a copy of that artifact' — delayed copy of a sacrificed artifact (token copies owned by p12)
- **delayed-dies-trigger** (1): Reckless Blaze — 'Whenever a creature you control dealt damage this way dies this turn, add {R}' — delayed trigger over the damaged-creature set needs prior-damage tagging for a dies-this-turn watcher
- **delayed-player-attack-history** (1): Faramir, Prince of Ithilien — 'At the beginning of that player's next end step, you draw a card if they didn't attack you that turn. Otherwise ...' — delayed trigger with player attack-history predicate
- **derived-escape-grant** (1): Desdemona, Freedom's Edge — target filter 'creature card ... that's an artifact or that has mana value 3 or less' is misread as '(creature or artifact) card with mana value 3 or less' (needs a disjunction of two relative clauses on one noun)
- **distribute-among-it-and-commanders** (1): Stumpsquall Hydra — 'distribute X +1/+1 counters among it and any number of commanders' — distribution over source plus untargeted commander set
- **distribute-up-to-that-many** (1): Lathiel, the Bounteous Dawn — 'distribute up to that many +1/+1 counters among any number of other target creatures' where that many = life gained this turn — dynamic distributed counter amount with variable target count
- **double-that-damage-instead** (1): Goblin Charbelcher — 'If the revealed land card was a Mountain, this artifact deals double that damage instead' — conditional self-replacement doubling a prior amount
- **draft-matters** (1): Garbage Fire — 'note how many cards you've drafted this draft round' — draft-matters (Conspiracy) mechanics are out of scope
- **draw-from-bottom** (1): River Song — 'You draw cards from the bottom of your library rather than the top' — draw-source replacement (owned by p10)
- **earthbend-where-x** (1): Avatar Aang // Aang, Master of Elements — trigger 'Whenever you waterbend, earthbend, firebend, or airbend' needs a multi-kind keyword-action trigger list, and 'if you've done all four this turn' needs a per-turn keyword-action history condition
- **energy-paid-this-way** (1): Die Young — 'you may pay any amount of {E}. The creature gets -1/-1 for each {E} paid this way' — variable energy payment amount
- **escape-extra-costs** (1): Lunar Hatchling — 'Escape—{4}{G}{U}, Exile a land you control, Exile five other cards from your graveyard' — escape keyword with an extra non-graveyard exile cost component
- **exile-copy-cast** (1): Mizzix's Mastery — 'For each card exiled this way, copy it, and you may cast the copy without paying its mana cost' with Overload — copy of exiled cards (owned by p12)
- **exiled-hit-counter-loss** (1): Etrata, the Silencer — 'That player loses the game if they own three or more exiled cards with hit counters on them' + 'shuffles Etrata into their library' — exile-zone counter count predicate
- **explicit-ability-choice-lists** (1): Éowyn, Lady of Rohan — the conditional-instead sentence ('If that creature is equipped, it gains first strike and vigilance ... instead') must replace the choice grant; coordination of the explicit choice list is now suppressed, but the instead rewrite over a ChooseMode grant is unverified
- **for-as-long-as-tapped** (1): Giant Oyster — 'For as long as this creature remains tapped, ... and at the beginning of each of your draw steps, put a -1/-1 counter on that creature. When this creature leaves ... or becomes untapped, remove all -1/-1 counters from the creature.' — tapped-duration delayed triggers
- **for-each-quantities** (1): Phyresis Outbreak — 'for each poison counter its controller has' inside 'each creature your opponents control gets ...' needs a per-object player reference (PlayerCounters of ControllerOf(iterated object)) in the per-object pump
- **gains-other-abilities-of-card** (1): Symbiote Spider-Man — 'It gains this card's other abilities' — granting an exiled source card's abilities
- **grant-if-lacks-ability** (1): Musician — 'If it doesn't have "...", it gains that ability' — has-ability predicate for a quoted ability (owned by p09)
- **granted-encore** (1): Araumi of the Dead Tide — needs derived casting-keyword grant path (p02): encore granted to a graveyard card with cost = its mana cost (encore is an activated ability from the graveyard; p02 also lists granted-encore as blocked)
- **granted-exile-cast-permission** (1): Lukka, Coppercoat Outcast — 'Creature cards exiled this way gain "You may cast this card from exile as long as you control a Lukka planeswalker."' — play-from-exile permission grant (owned by p05)
- **granted-flashback-fixed-cost** (1): The Fugitive Doctor — 'target instant or sorcery card in your graveyard gains flashback {2}{R}{G} until end of turn' — reflexive grant of flashback with a fixed printed cost (KeywordFallbackText)
- **granted-foretell-cost** (1): Bohn, Beguiling Balladeer — 'Each nonland card in your hand without foretell has foretell. Its foretell cost is equal to its mana cost reduced by {2}.' — derived foretell grant
- **granted-harmonize** (1): Songcrafter Mage — needs derived casting-keyword grant path (p02): harmonize with cost = its mana cost (no DerivedAlternativeCast::Harmonize)
- **granted-jump-start** (1): Filigree Racer — needs derived casting-keyword grant path (p02): jump-start granted to a graveyard instant/sorcery (no DerivedAlternativeCast::JumpStart; p02 lists granted-jump-start as blocked)
- **granted-plot** (1): Fblthp, Lost on the Range — 'The top card of your library has plot. The plot cost is equal to its mana cost.' + plot from library top (play-from-zone variants owned by p05)
- **granted-prevention-draw** (1): Sokrates, Athenian Teacher — granted '"If this creature would deal combat damage to a player, prevent that damage. This creature's controller and that player each draw half that many cards"' — prevention with prevented-amount follow-up (owned by p06)
- **greatest-mv-discarded** (1): Scythe Specter — 'Each player who discarded a card with the greatest mana value among cards discarded this way loses life equal to that mana value' — per-player greatest-of-discards comparison
- **halve-damage-replacement** (1): Ghosts of the Innocent — 'If a source would deal damage to a permanent or player, it deals half that damage, rounded down, instead' — damage replacement (owned by p06)
- **hideaway-duplicate** (1): Evercoat Ursine — 'Hideaway 3, hideaway 3' (two instances, CR 702.75) plus 'cards exiled with it, you may play one of them' — duplicate keyword list falls to KeywordFallbackText
- **intel-counter-return** (1): Flamewar, Brash Veteran // Flamewar, Streetwise Operative — 'Put all exiled cards you own with intel counters on them into your hand' — exile-zone counter selection
- **last-counter-draw-game** (1): Divine Intervention — 'When you remove the last intervention counter from this enchantment, the game is a draw.' — last-counter-removed trigger and game-draw effect
- **linked-life-memory** (1): Soulgorger Orgg — 'you lose all but 1 life' + 'you gain life equal to the life you lost when it entered' — linked memory of the earlier loss
- **mana-loss-event** (1): Yurlok of Scorch Thrash — 'A player losing unspent mana causes that player to lose that much life' — static mana-burn replacement over mana-empty events
- **milled-collection-return** (1): Avatar Destiny — 'Return this card ... and up to one creature card milled this way to the battlefield' after a dies trigger — card selection over the mill-this-way collection from a leaves-battlefield trigger
- **modal-trigger-intervening-if** (1): Jin Sakai, Ghost of Tsushima — intervening-if predicate 'no other creatures are attacking that player' (per-defending-player attacker census bound to the trigger's attacked player) is not modeled by parse_modeled_predicate
- **modular-replacement** (1): Zabaz, the Glimmerwasp — 'If a modular triggered ability would put ... counters on a creature you control, that many plus one ... instead' — replacement keyed on modular ability source (owned by p06)
- **multi-recipient-new-target** (1): Ian the Reckless — 'you may have it deal damage equal to its power to you and any target' — one damage packet to 'you' and a freshly declared target (shared-amount recipient set only admits non-target references)
- **named-card-copy** (1): Garth One-Eye — 'Create a copy of the card with the chosen name' (copy of a card not in any zone) — owned by p12
- **next-damage-redirect-owner** (1): Personal Incarnation — 'The next 1 damage that would be dealt to this creature this turn is dealt to its owner instead' (redirect owned by p06) + 'its owner loses half their life, rounded up' (ItsOwner life reference)
- **next-x-spell-delayed** (1): Brass Infiniscope — 'When you next cast a spell with {X} in its mana cost this turn, you draw a card and gain half X life, rounded down' — delayed cast trigger with that spell's X
- **non-mana-unearth** (1): Salvation Colossus — 'Unearth—Pay eight {E}' — unearth with a non-mana (energy) cost is not representable in the Unearth keyword payload
- **per-kind-counter-choice** (1): Bribe Taker — 'for each kind of counter on permanents you control, you may put your choice of a +1/+1 counter or a counter of that kind'
- **per-opponent-returned-this-way** (1): Faerie Slumber Party — 'For each opponent who controlled a creature returned this way' — participant predicate over the prior bounce's affected set
- **player-mode-targets** (1): Shadrix Silverquill — 'you may choose two. Each mode must target a different player.' — empty may-by-player modal branch with distinct player targets per mode
- **plural-become-with-quoted** (1): Sparkshaper Visionary — 'they become 3/3 blue Bird creatures with flying, hexproof, and "..."' — plural animate with a serial keyword list ending in a quoted ability is split by the coordinated-and reader (the singular form compiles)
- **prevent-then-create-that-many** (1): Ria Ivor, Bane of Bladehold — 'the next time target creature would deal combat damage to one or more players this combat, prevent that damage. If damage is prevented this way, create that many ...' — prevention shield with prevented-amount follow-up
- **put-or-remove-reflexive** (1): Immard, the Stormcleaver — 'put a charge counter on it or remove one from it. When you remove a counter this way, choose one —' — reflexive modal gated on the chosen branch
- **put-rest-into-graveyard-count** (1): Dihada, Binder of Wills — 'Create a Treasure token for each card put into your graveyard this way' after a reveal/put split (library look/put owned by p10)
- **radiation-life-replacement** (1): Strong, the Brutish Thespian — 'You gain life rather than lose life from radiation' — radiation life-loss replacement
- **random-reveal-pay-life** (1): Wand of Ith — 'reveals a card at random from their hand. If it's a land card, that player discards it unless they pay 1 life. If it isn't, ... unless they pay life equal to its mana value' — revealed-card branches with dynamic life payment
- **redirect-to-chosen-creature** (1): Flaming Gambit — 'That player or that planeswalker's controller may choose a creature they control and have Flaming Gambit deal that damage to it instead' — damage redirection choice (owned by p06)
- **relative-target-player** (1): Keeper of the Dead — 'target opponent who has at least two fewer creature cards in their graveyard than you do as you activate this ability' + 'that player' — as-you-activate snapshot target restriction (owned by p01)
- **sacrifice-serial-list** (1): Malevolent Witchkite — 'sacrifice any number of artifacts, enchantments, and/or tokens' — the serial and/or object list is split into clauses ('and or tokens'); single-type version compiles
- **sacrificed-colors-discard** (1): Mind Extraction — 'discards all cards of each of the sacrificed creature's colors' — discard filter keyed on the sacrificed cost object's colors
- **sacrificed-mv-search** (1): Vivien on the Hunt — 'search for a creature card with mana value equal to 1 plus the sacrificed creature's mana value' + mill-this-way selection — card selection has no resolved source zone
- **saddled-copy-repeat** (1): Calamity, Galloping Inferno — copy of creature that saddled it + 'Repeat this process once' (repeat loop owned by p11, token copies by p12)
- **saga-gains-ability-word-trigger** (1): Down in the Valley — 'This Saga gains "Landfall — Whenever a land you control enters, create ..."' — permanent self-grant of a labeled quoted trigger
- **same-is-true-for** (1): Concerted Effort — 'The same is true for fear, first strike, ...' — keyword-by-keyword replication clause
- **spliced-name-discard** (1): Minamo's Meddling — 'discards each card with the same name as a card spliced onto that spell' — splice history + same-name (owned by p09)
- **supertype-change** (1): Arcum's Weathervane — Runtime marker: 'Target snow land is no longer snow' / 'Target nonsnow basic land becomes snow' need permanent add/remove-supertype (Snow) continuous effects; no lowering exists
- **target-set-choice** (1): Retribution — 'Choose two target creatures controlled by the same opponent. That player chooses and sacrifices one of those creatures. Put a -1/-1 counter on the other.' — opponent choice among the targets with 'the other' remainder
- **tempting-offer** (1): Tempt with Discovery — 'For each opponent who searches a library this way ... Then each player who searched a library this way shuffles' — tempting-offer participant history
- **total-toxic-value** (1): Goliath Hatchery — 'draw cards equal to its total toxic value' — sum of toxic keyword values
- **type-count-of-it** (1): Embiggen — '+1/+1 for each supertype, card type, and subtype it has' — count of the object's own types
- **villainous-choice** (1): Genesis of the Daleks — 'faces a villainous choice — Destroy all Dalek creatures and each of your opponents loses life equal to the total power of Daleks that died this turn, or ...' (multi-choice designations owned by p09)
- **vote-protection** (1): Council Guardian — 'protection from each color with the most votes or tied for most votes' — votes (owned by p09)
- **your-choice-keyword-loss** (1): Walking Sponge — 'Target creature loses your choice of flying, first strike, or trample' — ability-choice is only admitted for gains (allow_choice = !losing)
