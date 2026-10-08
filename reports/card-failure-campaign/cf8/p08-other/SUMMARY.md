# cf8 / p08-other — summary

175 cards. Source-only: nothing built or run (campaign policy). The prebuilt probe binary was used read-only for first-pass triage, then disappeared from the main checkout. Second-pass triage of the 59 single-card failures was done by source reading only, using four read-only exploration passes. Fixes marked "unprobed" in the ledger have reading-only confidence.

| status | count |
|---|---|
| source-proposed | 60 |
| already-on-main | 2 (Loathsome Troll, cba78f342; Titania, c000cb0a1 — both unvalidated) |
| blocked | 113, each with its precise missing mechanism |
| untriaged | 0 |

## Clusters fixed (general fixes)

### Pass 1

| cluster | cards | fix |
|---|---|---|
| bare-singular-object-noun-recipient | Settle the Score, Liliana's Scrounger, Cosmium Confluence, Wick, Astarion's Thirst | Bare card-type, supertype, subtype or `commander` head becomes a non-targeted exactly-one choice. (`target_semantics/reference.rs`) |
| departed-object-controller-that-player | Pain Distributor, Sardian Avenger, Shriek | Zone-change and dies triggers on a non-you controller filter infer `AliasedControllerOf(triggering)` (CR 603.10a). (`trigger_players.rs`) |
| join-forces-validator | Minds Aglow, Collective Voyage, Alliance of Arms, Mana-Charged Dragon | The validator traverses `CollectManaPaymentsEffect`; both head orders are accepted. |
| counter-kind-put-another | Maulfist Revolutionary, Skyship Plunderer, Powerful Broker | Put-only, all-kinds counter-kind form; the engine handles player targets (CR 122.1). |
| granted-banding | Cooperation, Dire Wolves, Fortified Area | Banding arm in `runtime_static_ability_for_keyword_action` (CR 702.22). |
| separate-line-instead-restatement | Slaying Fire, Summary Judgment, Fiery Impulse | "If <cond>, <action> instead." joins the preceding spell statement. **Narrowed in pass 2:** only when the condition has no "would", the restated action verb equals the verb of the statement's last sentence, and that sentence is not itself an instead-replacement. A negative test covers a draw statement followed by a damage restatement, and a different-sentence restatement. |
| twice-that-much-damage | Chocobo Kick, Surtland Flinger | "twice that much" = `Scaled(prior amount, 2)`; a definite description names the targeted source; the replacement now also reaches into a preceding "When you do" body. |
| reflexive-fight | Back for More, Curse of the Werefox | The fight primitive yields when/if-led subjects to the reflexive rule (CR 603.12). |
| static-reading-ownership | Thran Lens, Mycosynth Lattice, Vnwxt, Wings of Velis Vel | Generic readings defer to the dedicated colorless, draw-doubling, and base-P/T readings. |
| named-untap-step-tagged-set | Sleep | A tagged set's "that player's next untap step" binds the declared target player. |
| library-viewer-player-antecedent | Fortune's Favor, Atris | The reference pass tracks "Target opponent looks at ..." as "that player". |
| bare-sneak-keyword | Leonardo, Shark Shredder, Dark Leo & Shredder | A bare "Sneak {cost}" line is accepted without reminder text (other lines unprobed). |

### Pass 2

| cluster | cards | fix |
|---|---|---|
| return-to-command-zone | Leadership Vacuum, Hellkite Courser | `ReturnZoneShape::Command` → move to `Zone::Command` (CR 408). |
| return-all-card-type-exceptions | Cyclone Summoner | "except for Giants, Wizards, and lands" adds `excluded_card_types`. |
| implied-battlefield-return | Meathook Massacre II | A zoneless "return ... under <player>'s control" names the battlefield (CR 108.4). |
| that-player-in-amount | Anathemancer | The source-bound damage arm binds "that player" in the amount to the explicit player target. |
| revealed-this-way-producer | Stomping Slabs | A revealing top-of-library look is a Revealed result producer. |
| villainous-choice-that-player | Damocles Base | "that player faces a villainous choice" in a trigger: one choice, no loop. |
| as-long-as-dont-untap | Winter's Rest | The narrow don't-untap rule yields "As long as" lines to the conditional wrapper. |
| turn-scoped-enter-tapped-subject | Nahiri's Lithoforming | The greedy whole-document reading never takes a multi-sentence subject. |
| clause-head-verbs | Fatal Fissure | `earthbend(s)` and `rolls` added to the clause-head actions. |
| tap-or-untap-coordination | Tolarian Kraken | "tap or untap <object>" is not split. |
| payment-get-counters | The Serpent Society | "Ward—Get N poison counters" is read as the payer's own sentence. |
| duration-scoped-trigger | Season of the Bold | "until (the) end of your next turn, whenever ..." is accepted. |
| player-combat-damage-count | Vivien's Stampede | "for each player who was dealt combat damage this turn" counts all players via turn history. |
| kicker-ability-marker | Coralhelm Chronicler | "a card with a kicker ability" → `Marker(kicker)`; the engine matches Kicker/Multikicker (CR 702.33). |
| damage-amount-ownership | Call Forth the Tempest | The relative aggregate owns its phrase over the cost-modifier fragment. |
| each-object-set-union | Corpse Explosion | "each creature and each planeswalker" becomes one union recipient set, in both damage readings. |
| replacement-reflexive-followup | Valentin (front face) | The nontoken would-die exile replacement accepts "When you do, ...". |

## Tests (authored, unrun)

All files are in `crates/ironsmith-compiler-runtime/tests/` and share the helper `cf8_p08/support.rs`.

- **Pass 1:**
  - `bare_object_noun_counter_recipients.rs`
  - `departed_object_controller_player.rs`
  - `join_forces_collective_scope.rs`
  - `counter_kind_put_another.rs`
  - `granted_banding.rs`
  - `separate_line_instead_restatement.rs`, including the negative join tests
  - `twice_that_much_damage.rs`
  - `reflexive_fight.rs`
  - `static_reading_ownership.rs`
  - `named_untap_step_tagged_set.rs`
- **Pass 2:**
  - `return_destination_extensions.rs`
  - `reference_and_result_bindings.rs`
  - `line_ownership_and_heads.rs`
  - `damage_amount_ownership.rs`
  - `replacement_reflexive_followup.rs`

## Round 3 (on cf8/integration)
| cluster | cards | fix |
|---|---|---|
| villainous-choice-routing | The Dalek Emperor, Damocles Base, Ensnared by the Mara | The em dash after "faces a villainous choice" was split as an ability-word label. The trigger became a label and the modes became a statement with an unbound "that player". That dash is no longer treated as a label (`document_shapes/labels.rs`). Exiling from the top of a library is now a memory producer, so "those exiled cards" binds (Ensnared). |
| player-and-each-object-recipients | Cerebral Eruption | "to that player and each creature that player controls" deals damage to the player and to the object set, in both damage readings. |
| optional-search-producer | Unlucky Cabbage Merchant | A "you may" library search produces the result for "If you search your library this way". |
| die-arithmetic-coordination | Gale's Redirection | Coordination no longer splits "roll a dN and add/subtract X"; the die-arithmetic reading owns it. |
| die-row-owner | The Deck of Many Things, Druid of the Emerald Grove | The row-owner check skips a condition that only reads the roll's own result. The search reading leaves a trailing ", then roll/flip" to the comma-then chain. |

Synthetic token insertions replaced with grammar:
- **Controller-only return destination:** `last_destination_split` now returns (target end, destination start). A destination that names only "under <player>'s control" (no to/onto, no zone) parses as the battlefield.
- **Ward payment:** "get N poison/energy counters" is read by a dedicated payment grammar instead of a prepended "you".

Test: `villainous_and_recipient_routing.rs`.

## Coordinator's requested groups
- **Opponent chooses the mode at cast:** still blocked.
  - Rules: CR 700.2e (cited from memory; the repo has no CR text) has the other player choose the mode while the spell is being cast, with the caster picking which opponent. The caster still chooses targets (601.2c). In Fatal Lore, "that player" also narrows the target filter, so the chooser must be bound before targeting.
  - Design needed:
    - a cast-time chooser field on ChooseModeEffect/ModalSpec;
    - a cast stage for choosing the opponent;
    - routing ModesContext to that player;
    - a stack-entry player binding usable by both target legality and mode effects.
  - Not attempted: it spans the cast pipeline, decision routing, targeting and lowering, and could not be compile-checked.
- **Die tables not last:** Deck of Many Things and Druid are done.
  - Wand of Wonder needs an X-table: rows "X is N" binding X in the preceding sentences, plus "exile until, cast up to X from among" grammar.
  - Wizard's Spellbook needs a probe of its exile-then-roll owner, and its row 3 (copy each card exiled with this artifact) is unsupported.
- **Player-plus-object recipients:** damage is done (Cerebral Eruption).
  - Kitsune Palliator (prevention) and Faith's Shield (protection for you and your permanents) need the same union in those effect families.
  - Sewers of Estark needs "it and each creature it's blocking" as a prevention source set.
- **Pay any amount of mana:** not generalized.
  - Karn's "that many" is the only case the join-forces collective payment would cover.
  - Errant Minion and Power Leak need partial prevention of one damage event.
  - Leyline Tyrant needs a colour-restricted variable payment plus a reflexive "that much".
  - Liege of the Hollows needs per-player paid amounts.
- **Villainous choice:** Dalek and Ensnared are done (see Round 3).
- **Claim Jumper:** needs the repeat-this-process owner (p11). Unlucky Cabbage Merchant is done.

## Risk notes
- **Shared hot spots touched:**
  - `target_semantics/reference.rs` (bare-noun fallback)
  - `trigger_players.rs` (zone-change player inference changes the compiled output of already-passing cards)
  - `statement_recognition.rs` (instead join, now verb-matched)
  - `coordination.rs` (tap/untap guard)
  - `return_shapes.rs` / `return_exchange_zone.rs` (new zone variant; three match sites updated)
  - `typed_clause_heads.rs`
  - `combat_verbs.rs`
  - `keyword_static/mod.rs`
  - `costs_replacements_and_permissions.rs` (two small hunks)
- **Engine changes:**
  - Counter-kind player branch: goes through the counter-batch gateway, no new checkpoint wrapper.
  - Kicker marker: in `filter/descriptions.rs`.
- **Token rewrites:**
  - None remain (the two synthetic-token rewrites were replaced by grammar in round 3).

## Blocked, grouped by missing mechanism
- **unresolved-it-reference** (10): Desolation, Elite Arcanist, Instill Furor, Ixhel, Scion of Atraxa, Nascent Metamorph, Replicating Ring, Snort, Talion's Messenger, Tavern Brawler, Teo, Spirited Glider. 'it' without prior reference: 'each player who tapped a land for mana this turn' / 'who sacrificed a Plains this way' player-history predicates.
- **selection-phrases** (8): Crashing Wave, Dwarven Catapult, Legion's End, Sorrow's Path, Split the Party, Sword of Hearth and Home, Ulamog, the Defiler, Ultimate Nullification. Unsupported selection phrase ('both cards', 'both of them' with block rewiring, 'graveyard(s)' as exile recipient, 'half the creatures/library rounded up', 'divided evenly', 'tapped creatures ... distribute'); each needs its own typed selection.
- **dynamic-mana-or-roll-amounts** (5): Danse Macabre, Drain Power, Elemental Resonance, Radiant Lotus, Quag Feast. roll plus toughness of sacrificed creature, rows not represented.
- **pay-any-amount-of-mana** (5): Errant Minion, Karn, Living Legacy, Leyline Tyrant, Liege of the Hollows, Power Leak. Single-payer 'pay any amount of mana' is not represented. Generalizing CollectManaPaymentsEffect with a payer would cover only Karn's X binding; Errant Minion/Power Leak need partial prevention of one damage event ('Prevent X of that damage'), Leyline Tyrant a red-only payment plus reflexive 'that much', and Liege of the Hollows per-player amounts. Each needs its own mechanism, so the generalization was not done.
- **damage-modification-replacement** (3): Neriv, Heart of the Storm, Benevolent Unicorn, Lashknife Barrier. 'deals that much damage minus 1 / twice that much damage ... instead' as a static damage replacement needs a typed amount-modifying replacement reading.
- **compound-recipient-sets** (3): Faith's Shield, Kitsune Palliator, Sewers of Estark. Compound recipient 'X and each Y' (player+objects, two object sets) in damage/prevention/protection needs a union recipient spec.
- **up-to-that-many-targets** (3): Cephalid Constable, Coveted Falcon, Froghemoth. 'up to that many target ...' from combat damage amount / 'for each one they gained control of this way' needs an event-amount target count and per-object result metrics.
- **opponent-chooses-mode** (3): Fatal Lore, Library of Lat-Nam, Misfortune. 'An opponent chooses one' needs the opponent to choose the mode when the controller normally would, i.e. while casting (CR 700.2e, 601.2b), with the controller picking which opponent and 'that player' bound to that chooser. The cast pipeline announces modes only through the caster's ModesContext; ChooseModeEffect.chooser is a resolution-time chooser, so lowering to it would move the choice to resolution (observably wrong for Fatal Lore's targets). Design needed: a cast-time mode chooser on ChooseModeEffect/ModalSpec, a cast stage where the caster picks which opponent chooses (multiplayer), routing the ModesContext to that player while the caster still picks targets (601.2c), and a stack-entry player binding so 'that player' in modes and target filters ('target creatures that player controls') name the chooser.
- **conditional-has-and-is-type** (3): Ezio, Brash Novice, Hero of Bretagard, Skyknight Squire. 'As long as ... counters on it, it has K and is a T in addition to its other types' needs a conditional keyword+type-addition static pair reading.
- **mana-trigger-additional-chosen-color** (2): Caged Sun, Gauntlet of Power. 'adds an additional one mana of that color' for a chosen-color mana-production trigger is not represented.
- **pay-life-equal-to-dynamic** (2): Lorcan, Warlock Collector, Madame Null, Power Broker. 'you may pay life equal to its mana value/power' needs a dynamic life cost bound to the trigger object.
- **die-result-owner** (2): Wand of Wonder, Wizard's Spellbook. The die-row owner requires the roll to be the last unconditional action. Here the roll is modified ('subtract the number of cards in your hand' then an If-result), is preceded by a search-and-reveal whose probe fails, or the rows bind X for a later sentence (Wand of Wonder). Needs a non-terminal roll owner with a modified-result value and row-bound X; not attempted without a probe.
- **return-with-attached-auras** (2): Essence Reliquary, Orzhov Charm. 'target permanent and all Auras you control attached to it' needs a return-to-hand counterpart of the exile attached-bundle (simultaneous move).
- **past-tense-that-creature** (2): Taborax, Hope's Demise, Venom, Eddie Brock. 'If that creature was a X' after an intervening source action: conditional lowering loses the trigger object antecedent (saved last tag is None); needs the dies-trigger object kept as the 'that creature' antecedent across source-targeted actions.
- **filtered-result-metric** (2): Convert to Slime, The Hunger Tide Rises. Aggregate over objects destroyed/sacrificed this way requires a memory-producing effect id binding for the filtered metric.
- **repeated-payment-count-modes** (2): Tranquil Frillback, Hawkeye, Master Marksman. 'pay {G} up to three times' repeated resolution payment with count, and 'choose up to that many' modal range.
- **per-attacker-sacrifice-attack-cost** (2): Flooded Woodlands, Reclamation. 'can't attack unless their controller sacrifices a land for each ... attacking' needs a cost-executable per-attacker attack tax (CR 508.1h).
- **ordered-exile-pile** (2): Mangara's Tome, Parallel Thoughts. Ordered face-down exile pile linked to the source plus 'put the top card of the exiled pile' draw replacement.
- **either-of-them** (2): Call of the Death-Dweller, Wicked Slumber. Second 'either of them' must re-bind to the original plural set, but the reference env rebinds 'them' to the first counter recipient.
- **pay-object-mana-cost** (1): Ice Cave. No resolution-time 'pay <object>'s mana cost' (PayManaEffect holds a fixed cost) and no 'any other player may pay; if a player does' chooser.
- **next-spell-life-alternative-cost** (1): Marshland Bloodcaster. GrantNextSpellCostReduction supports only 'without paying its mana cost'; needs a 'pay life equal to that spell's mana value' variant plus grammar.
- **repeat-process** (1): Claim Jumper. needs repeat-this-process loop (owned by p11)
- **coin-flip-reflexive-gates** (1): Breeches, the Blastmaker. 'When you win/lose the flip' gates sit outside the 'If you do' body holding the flip, so they find no producer id; needs the flip's result id exported to the sibling sentences.
- **fight-excess-damage-recipient** (1): Rhino's Rampage. Fight is not a DealtDamage producer; adding it naively would fire on excess damage to your own creature too. Needs a recipient-filtered excess-damage result.
- **color-union-return-all** (1): Balthor the Defiled. 'all black and all red creature cards' is split by coordination; needs a no-split guard plus an 'all A and all B <noun>' union filter for return-all.
- **repeat-process-for-types** (1): Linessa, Zephyr Mage. 'then repeats this process for an artifact, an enchantment, and a land' needs expansion into sequential per-type returns by the same player.
- **spell-or-permanent-target-union** (1): Press the Enemy. 'target spell or nonland permanent an opponent controls' is split by coordination and spell_filters only knows 'spell or permanent'; also needs a free cast bound to the returned object's mana value.
- **random-retarget-multiple** (1): Chef's Kiss. Random target reselection for a spell and its copy with a 'can't be you or a permanent you control' restriction is not represented.
- **cross-zone-additional-cost-choice** (1): Close Encounter. Non-targeted additional-cost choice spanning battlefield and exile, a 'warped' card filter, and a value bound to that choice.
- **create-another-of-those-tokens** (1): Wurmquake. 'for each opponent with N or more poison counters' (only 'who has') and 'create another one of those tokens' (re-emit the prior token definition) are unsupported.
- **per-attached-damage** (1): Baki's Curse. 'deals 2 damage to each creature for each Aura attached to that creature' needs a per-object damage amount over attached Auras.
- **still-exiled-count** (1): Dragonhawk, Fate's Tempest. 'for each of those cards that are still exiled' needs a tagged still-in-exile count carried into a delayed trigger and a damage multiplier split.
- **life-total-zero-replacement** (1): Enduring Angel // Angelic Enforcer. 'If your life total would be reduced to 0 or less, instead transform ... life total becomes 3' is not represented.
- **do-the-same-procedure** (1): Guild Feud. 'You do the same with ...' replay of a reveal/choose/put procedure and fighting the two creatures put onto the battlefield this way.
- **life-bid-restricted** (1): Mages' Contest. Life bidding exists only for all players; needs a two-player bidder set and an 'if you win the bidding' condition.
- **attacked-players-binding** (1): Zurzoth, Chaos Rider. 'you and those players each draw, then discard at random' needs the attacked players of the triggering Devils as a binding.
- **triggers-only-once-ever** (1): Acrobatic Cheerleader. 'This ability triggers only once' needs a lifetime trigger cap (only per-turn caps exist).
- **optional-enter-tapped-followup** (1): Mariposa Military Base. 'You may have this land enter tapped. If you do, ...' needs an optional entry replacement with a follow-up program.
- **sectors** (1): Space Beleren. No sector filter, 'sector of your choice', or same-sector block restriction.
- **highest-roll-attack-restriction** (1): Chaos Dragon. Per-player roll comparison and a this-combat attack restriction against the highest-rolling opponents ('rolls' head fixed only).
- **conditional-enters-tapped-on-return** (1): Silver Surfer, Cosmic Voyager. 'If a land enters this way, it enters tapped' needs a per-object entry modifier on the delayed batch return.
- **conditional-tapped-attacking-entry** (1): Summoner's Grimoire. Granted quoted ability needs 'If that card is an enchantment card, it enters tapped and attacking' as a conditional entry modifier.
- **villainous-choice-additional-time** (1): The Valeyard. Replacement making an opponent face a villainous choice an additional time is not represented.
- **per-player-exile-play-permission** (1): Rocco, Street Chef. Each player exiles their top card and may play the card they exiled this way until your next end step: needs per-player tagged play grants.
- **lexer-and-damage-history** (1): Ratonhnhaké꞉ton. Lexer rejects U+A789 in the name; 'hasn't dealt damage yet' needs per-object damage history.
- **per-die-result-grants** (1): Celebr-8000. Two-dice roll with per-result bullet grants and a doubles condition.
- **type-change-with-granted-cost-ability** (1): Supper for Spiders. 'They are Food artifacts with "{2}, {T}, Sacrifice this artifact: ..."' on the returned creatures (lowered in the spell's context).
- **player-subject-destroy-choice** (1): Burning of Xinye. 'target opponent destroys four lands they control' needs a subject-chosen counted destroy (parse_destroy drops the subject).
- **special-action-ignore-effect** (1): Lost in Thought. Static-granted special action letting a player ignore the effect until end of turn.
- **triggering-group-choice** (1): Frantic Scapegoat. Choosing one of the triggering batch of entering creatures with an 'if you do' on suspecting.
- **hand-or-battlefield-exile-until** (1): Cloak and Dagger, Entwined. Exile a card from a revealed hand or the chosen creature until the source leaves: tag-affected prelude restrictions.
- **delayed-trigger-player-capture** (1): Horn of Plenty. 'they draw a card at the beginning of the next end step' needs the paying player captured as a tagged player for the delayed body.
- **sacrificed-cost-reference-in-mode** (1): Desperate Plea. Mode-level reference to the additional-cost sacrificed creature's power is unbound.
- **per-object-coin-flips** (1): Rakdos, the Showstopper. 'flip a coin for each creature ... destroy each creature whose coin comes up tails' needs per-object coin results.
- **artifact-offering** (1): Blast-Furnace Hellkite. 'Artifact offering' quality unsupported by the offering reader.
- **stickers** (1): Ambassador Blorpityblorpboop. Sticker sheet/ticket counters and sticker power totals not modeled.
- **choose-counter-give-another** (1): Animation Module. Two-sentence 'Choose a counter on target permanent or player. Give that permanent or player another counter of that kind.' needs a chosen-kind form that also covers players.
- **cast-opponent-owned-exile** (1): Ashiok, Nightmare Muse. Cast up to three face-up exiled cards opponents own without paying needs a counted free-cast grant over opponent-owned exile.
- **random-counter-kind** (1): Crystalline Giant. Random choice among counter kinds the creature doesn't have needs a random-kind choice effect.
- **excess-count-phase-out-repeat** (1): Equipoise. Per-type excess counting with 'repeat this process for artifacts and creatures' needs a parametrized repeat.
- **prevention-plural-recipient** (1): Ethersworn Shieldmage. 'prevent all damage ... to artifact creatures this turn' plural object recipient not recognized in the prevention reading.
- **owner-shuffles-source** (1): Fblthp, Impossibly Lost. 'Fblthp's owner shuffles him into their library' source-by-name/pronoun 'him' not recognized.
- **counter-removal-prohibition** (1): Fear of Sleep Paralysis. 'Stun counters can't be removed from permanents your opponents control' needs a counter-removal prohibition static.
- **resolution-count-trigger** (1): Gimli, Mournful Avenger. 'When this ability resolves for the third time this turn' needs per-ability resolution counting.
- **play-from-exile-with-counter** (1): Lara Croft, Tomb Raider. 'play a card from exile with a discovery counter on it this turn' needs a counter-filtered play permission.
- **tap-others-lands-for-mana** (1): Piracy. Tapping lands you don't control for mana needs a cross-controller mana-ability permission.
- **pronoun-his** (1): Sarkhan, Dragon Ascendant. 'becomes a Dragon in addition to his other types' with 'his' pronoun unsupported.
- **cast-from-counter-marked-exile** (1): Tasha, the Witch Queen. Cast from exile among cards with page counters without paying mana cost needs a counter-filtered exile cast grant.
- **enters-with-multiple-counters-and-haste** (1): Voidpouncer. 'enters with two +1/+1 counters and a trample counter on it and with haste' compound entry rider unsupported.
