//! Claude package 4 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p4_debug(name: &str, text: &str) -> String {
    let definition = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
    format!("{definition:#?}")
}

#[test]
fn player_mandatory_free_cast_if_able_is_a_plain_tagged_cast() {
    let debug = p4_debug("Omen Machine", "Type: Artifact\nPlayers can't draw cards.\nAt the beginning of each player's draw step, that player exiles the top card of their library. If it's a land card, the player puts it onto the battlefield. Otherwise, the player casts it without paying its mana cost if able.");
    assert!(debug.contains("CastTaggedEffect"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    assert!(!debug.contains("MayEffect"), "an if-able cast is mandatory: {debug}");
}

#[test]
fn base_power_toughness_and_cant_be_blocked_share_the_subject() {
    let debug = p4_debug("Riverfall Mimic", "Type: Creature — Shapeshifter\nWhenever you cast a spell that's both blue and red, this creature has base power and toughness 3/3 until end of turn and can't be blocked this turn.");
    assert!(debug.contains("SetPowerToughness"), "{debug}");
    assert!(debug.contains("CantEffect") && debug.contains("BeBlocked("), "restriction half must survive: {debug}");
}

#[test]
fn look_at_that_library_reads_the_triggering_players_library() {
    let definition = compile_to_runtime_definition("Psychic Surgery", "Mana cost: {1}{U}\nType: Enchantment\nWhenever an opponent shuffles their library, you may look at the top two cards of that library. You may exile one of those cards. Then put the rest on top of that library in any order.", false).unwrap();
    fn inspect(effect: &Effect, looks: &mut usize) {
        if let Some(look) = effect.downcast_ref::<ironsmith::effects::LookAtTopCardsEffect>() {
            *looks += 1;
            assert_eq!(look.player, ironsmith::target::PlayerFilter::IteratedPlayer);
            assert_eq!(look.viewer, ironsmith::target::PlayerFilter::You);
            assert_eq!(look.count, ironsmith::effect::Value::Fixed(2));
        }
        effect.visit_child_effects(&mut |child| inspect(child, looks));
    }
    let mut looks = 0;
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(trigger) = &ability.kind {
            for effect in trigger.effects.all_effects() { inspect(effect, &mut looks); }
        }
    }
    assert_eq!(looks, 1);
}

#[test]
fn owner_only_activation_and_owner_half_life_loss() {
    let definition = compile_to_runtime_definition("Personal Incarnation", "Type: Creature — Avatar Incarnation\n{0}: The next 1 damage that would be dealt to this creature this turn is dealt to its owner instead. Only this creature's owner may activate this ability.\nWhen this creature dies, its owner loses half their life, rounded up.", false)
        .expect("Personal Incarnation should compile");
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
        _ => None,
    }).expect("owner-only activation");
    assert_eq!(activated.timing, ironsmith::ability::ActivationTiming::AnyTimeBySourceOwner);
    assert!(activated.allows_any_player_to_activate());
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("HalfLifeTotalRoundedUp(OwnerOf("), "{debug}");
    assert!(!compact.contains("HalfLifeTotalRoundedUp(OwnerOf(Tagged(TagKey(\"__it__\""), "the owner reference must be resolved: {debug}");
}

#[test]
fn combat_damage_or_dies_is_a_shared_subject_trigger_union() {
    let debug = p4_debug("Park Heights Maverick", "Type: Creature — Human Soldier\nDethrone (Whenever this creature attacks the player with the most life or tied for most life, put a +1/+1 counter on it.)\nThis creature can't be blocked by creatures with power 2 or less.\nWhenever this creature deals combat damage to a player or dies, proliferate.");
    assert!(debug.contains("ProliferateEffect"), "{debug}");
    let lower = debug.to_ascii_lowercase();
    assert!(lower.contains("dies") || lower.contains("zonechange"), "death arm must survive: {debug}");
    assert!(lower.contains("combatdamage") || lower.contains("combat_damage"), "damage arm must survive: {debug}");
}

#[test]
fn life_for_each_snow_mana_spent_reads_the_payment_count() {
    let debug = p4_debug("Search for Glory", "Type: Snow Sorcery\nSearch your library for a snow permanent card, a legendary card, or a Saga card, reveal it, put it into your hand, then shuffle. You gain 1 life for each {S} spent to cast this spell. ({S} is mana from a snow source.)");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("GainLifeEffect"), "{debug}");
    assert!(compact.contains("ManaSymbolSpentToCastThisSpell{symbol:Snow"), "{debug}");
}

#[test]
fn chroma_counts_symbols_in_the_revealed_cards_mana_costs() {
    use ironsmith::card::CardBuilder;
    use ironsmith::decision::DecisionMaker;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::game_state::GameState;
    use ironsmith::ids::{CardId, ObjectId};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::{PlayerId, Zone};
    struct RevealSelection(Vec<ObjectId>);
    impl DecisionMaker for RevealSelection {
        fn decide_objects(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            ctx.candidates.iter().filter(|candidate| candidate.legal && self.0.contains(&candidate.id)).map(|candidate| candidate.id).collect()
        }
    }
    let definition = compile_to_runtime_definition("Phosphorescent Feast", "Type: Sorcery\nChroma — Reveal any number of cards in your hand. You gain 2 life for each green mana symbol in those cards' mana costs.", false).unwrap();
    let debug = format!("{definition:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("RevealTaggedEffect"), "{debug}");
    assert!(compact.contains("ManaSymbolsInManaCostOf{spec:All("), "count must aggregate the revealed cards: {debug}");
    for mask in 0..8 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let costs = [
            ManaCost::from_symbols(vec![ManaSymbol::Green]),
            ManaCost::from_pips(vec![vec![ManaSymbol::Green, ManaSymbol::Blue], vec![ManaSymbol::Green]]),
            ManaCost::from_symbols(vec![ManaSymbol::Generic(2), ManaSymbol::Red]),
        ];
        let mut selected = Vec::new();
        let mut expected = 20;
        for (index, cost) in costs.into_iter().enumerate() {
            let card = CardBuilder::new(CardId::new(), "Reveal candidate").mana_cost(cost).build();
            let id = game.create_object_from_card(&card, alice, Zone::Hand);
            if mask & (1 << index) != 0 {
                selected.push(id);
                expected += 2 * [1, 2, 0][index];
            }
        }
        let other = CardBuilder::new(CardId::new(), "Other player's green card")
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green; 4])).build();
        game.create_object_from_card(&other, bob, Zone::Hand);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut chooser = RevealSelection(selected);
        let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut chooser);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, expected, "selected mask {mask}");
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().hand.len(), 3, "reveal doesn't move cards");
    }
}

#[test]
fn dies_or_put_into_command_zone_shares_the_subject() {
    let debug = p4_debug("Reyhan, Last of the Abzan", "Type: Legendary Creature — Human Warrior\nReyhan enters with three +1/+1 counters on it.\nWhenever a creature you control dies or is put into the command zone, if it had one or more +1/+1 counters on it, you may put that many +1/+1 counters on target creature.\nPartner (You can have two commanders if both have partner.)");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("to:Some(Command)"), "command-zone arm must survive: {debug}");
    assert!(compact.contains("PutCountersEffect"), "{debug}");
}

#[test]
fn if_you_sacrifice_an_island_this_way_reads_the_sacrificed_set() {
    let debug = p4_debug("Serendib Djinn", "Type: Creature — Djinn\nFlying\nAt the beginning of your upkeep, sacrifice a land. If you sacrifice an Island this way, this creature deals 3 damage to you.\nWhen you control no lands, sacrifice this creature.");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("Island"), "{debug}");
    assert!(compact.contains("DealDamageEffect"), "{debug}");
    assert!(compact.contains("Conditional") || compact.contains("If"), "the damage must be gated: {debug}");
}

#[test]
fn created_token_entry_counters_are_part_of_the_creation_instruction() {
    let definition = compile_to_runtime_definition("Printlifter Ooze", "Type: Creature — Ooze\nDeathtouch\nWhenever this creature or another creature you control is turned face up, create a 0/0 green Ooze creature token with trample. The token enters with X +1/+1 counters on it, where X is the number of other creatures you control.\nDisguise {3}{G}", false)
        .expect("Printlifter Ooze should compile");
    let triggered = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(ability) => Some(ability),
        _ => None,
    }).expect("face-up trigger");
    let effects = triggered.effects.all_effects();
    let create = effects.iter().find_map(|effect| {
        let mut effect: &Effect = effect;
        while let Some(tagged) = effect.downcast_ref::<ironsmith::effects::TaggedEffect>() { effect = &tagged.effect; }
        effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>()
    }).expect("token creation");
    assert_eq!(create.entry_counters.len(), 1, "{create:#?}");
    assert_eq!(create.entry_counters[0].counter_type, ironsmith::object::CounterType::PlusOnePlusOne);
    assert!(!matches!(create.entry_counters[0].amount.unhinted(), ironsmith::effect::Value::X), "X must be bound: {create:#?}");
    assert!(effects.iter().all(|effect| effect.downcast_ref::<ironsmith::effects::PutCountersEffect>().is_none()),
        "the entry counters must not also be a later put");
}

#[test]
fn comma_then_whole_object_action_on_a_referent_splits_from_the_imperative_head() {
    let debug = p4_debug("Quag Feast", "Type: Sorcery\nChoose target creature, planeswalker, or Vehicle. Mill two cards, then destroy the chosen permanent if its mana value is less than or equal to the number of cards in your graveyard.");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("MillEffect"), "{debug}");
    assert!(compact.contains("DestroyEffect"), "{debug}");
    assert!(compact.contains("ManaValueOf"), "the destroy must be gated on the chosen permanent's mana value: {debug}");
}

#[test]
fn unless_that_opponent_pays_binds_the_triggering_opponent() {
    let debug = p4_debug("Prince of Thralls", "Type: Creature — Demon\nWhenever a permanent an opponent controls is put into a graveyard, put that card onto the battlefield under your control unless that opponent pays 3 life.");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("UnlessPaysEffect"), "{debug}");
    assert!(compact.contains("PayLifeEffect"), "{debug}");
    assert!(compact.contains("MoveToZoneEffect"), "{debug}");
}

#[test]
fn choose_a_color_then_become_the_chosen_color_is_one_choice() {
    let debug = p4_debug("Puca's Eye", "Type: Artifact\nWhen this artifact enters, draw a card, then choose a color. This artifact becomes the chosen color.\n{3}, {T}: Draw a card. Activate only if there are five colors among permanents you control.");
    assert!(debug.contains("BecomeColorChoiceEffect"), "{debug}");
    assert!(!debug.contains("ChooseColorEffect"), "the color must be chosen once: {debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn enchanted_player_cant_activate_non_mana_non_loyalty_abilities() {
    let debug = p4_debug("Overwhelming Splendor", "Type: Enchantment — Aura Curse\nEnchant player\nCreatures enchanted player controls lose all abilities and have base power and toughness 1/1.\nEnchanted player can't activate abilities that aren't mana abilities or loyalty abilities.");
    assert!(debug.contains("ActivateNonManaNonLoyaltyAbilities"), "{debug}");
    assert!(!debug.contains("ActivateNonManaAbilities("), "loyalty abilities must remain legal: {debug}");
}

#[test]
fn hexproof_until_first_combat_damage_reads_a_combat_only_history() {
    let debug = p4_debug("Ruric Thar, Magecrusher", "Type: Legendary Creature — Ogre Warrior\nThis spell can't be countered.\nReach, vigilance, trample\nRuric Thar has hexproof as long as they haven't dealt combat damage yet. (They can't be the target of spells or abilities your opponents control.)");
    assert!(debug.contains("SourceHasDealtCombatDamageSinceEntered"), "{debug}");
    assert!(!debug.contains("SourceHasDealtDamageSinceEntered"), "noncombat damage must not end the hexproof: {debug}");
}

#[test]
fn modifier_colon_name_lexes_and_reflexive_chain_keeps_the_attach() {
    let debug = p4_debug("Ratonhnhaké꞉ton", "Type: Legendary Creature — Human Assassin\nAs long as Ratonhnhaké꞉ton hasn't dealt damage yet, it has hexproof and can't be blocked.\nWhenever Ratonhnhaké꞉ton deals combat damage to a player, create a 1/1 black Assassin creature token with menace. When you do, return target Equipment card from your graveyard to the battlefield, then attach it to that token.");
    assert!(debug.contains("SourceHasDealtDamageSinceEntered"), "{debug}");
    assert!(debug.contains("ReflexiveTriggerEffect"), "{debug}");
    assert!(debug.contains("ReturnFromGraveyardToBattlefieldEffect"), "{debug}");
    assert!(debug.contains("AttachObjectsEffect"), "the attach to the token must survive: {debug}");
}

#[test]
fn untargeted_life_auction_rewards_the_high_bidder() {
    let definition = compile_to_runtime_definition("Pain's Reward", "Type: Sorcery\nEach player may bid life. You start the bidding with a bid of any number. In turn order, each player may top the high bid. The bidding ends if the high bid stands. The high bidder loses life equal to the high bid and draws four cards.", false)
        .expect("Pain's Reward should compile");
    let program = definition.spell_effect.as_ref().expect("spell effect");
    let effects = program.all_effects();
    let bid = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::BidLifeEffect>())
        .expect("one life auction");
    assert!(bid.target.is_none(), "{bid:#?}");
    assert_eq!(bid.starting_bid, ironsmith::effects::LifeBidStart::ControllerChooses);
    let reward = format!("{:#?}", bid.winner_effects);
    assert!(reward.contains("DrawCardsEffect"), "{reward}");
}

#[test]
fn redistribute_life_totals_is_one_typed_instruction() {
    let definition = compile_to_runtime_definition("Reverse the Sands", "Type: Sorcery\nRedistribute any number of players' life totals. (Each of those players gets one life total back.)", false)
        .expect("Reverse the Sands should compile");
    let program = definition.spell_effect.as_ref().expect("spell effect");
    let effects = program.all_effects();
    assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::RedistributeLifeTotalsEffect>().is_some()),
        "{effects:#?}");
}

#[test]
fn opponent_protects_it_reads_the_battle_protector() {
    let debug = p4_debug("Portent Tracker", "Type: Creature — Satyr Scout\n{T}: Untap target land.\n{T}: Choose target battle. If an opponent protects it, remove a defense counter from it. Otherwise, put a defense counter on it. Activate only as a sorcery.");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("protected_by:Some(Opponent)"), "{debug}");
    assert!(compact.contains("RemoveCountersEffect") && compact.contains("PutCountersEffect"), "{debug}");
}

#[test]
fn granted_ability_damage_to_itself_is_a_two_recipient_fanout() {
    let debug = p4_debug("Psionic Sliver", "Type: Creature — Sliver\nAll Sliver creatures have \"{T}: This creature deals 2 damage to any target and 3 damage to itself.\"");
    assert!(debug.matches("DealDamageEffect").count() >= 2, "both recipients must be damaged: {debug}");
}

#[test]
fn chosen_color_mana_scales_with_sacrificed_artifacts() {
    let debug = p4_debug("Radiant Lotus", "Type: Artifact\n{T}, Sacrifice one or more artifacts: Choose a color. Target player adds three mana of the chosen color for each artifact sacrificed this way. (Activate only as an instant.)");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("AddManaOfChosenColorEffect"), "{debug}");
    assert!(compact.contains("Scaled("), "three per artifact: {debug}");
}

#[test]
fn discard_a_card_of_another_players_choice_is_a_chosen_discard() {
    let debug = p4_debug("Rakdos Augermage", "Type: Creature — Human Wizard\nFirst strike\n{T}: Reveal your hand and discard a card of target opponent's choice. Then that player reveals their hand and discards a card of your choice. Activate only as a sorcery.");
    assert!(debug.matches("ChooseObjectsEffect").count() >= 2, "{debug}");
    assert!(debug.matches("DiscardEffect").count() >= 2, "{debug}");
}

#[test]
fn unearth_with_an_energy_payment_is_a_graveyard_activation() {
    let definition = compile_to_runtime_definition("Salvation Colossus", "Type: Artifact Creature — Construct\nFlying, vigilance, trample\nWhenever you attack, other creatures you control get +2/+2 and gain indestructible until end of turn.\nUnearth—Pay eight {E}. (Pay eight energy counters: Return this card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of the next end step or if it would leave the battlefield. Unearth only as a sorcery.)", false)
        .expect("Salvation Colossus should compile");
    let unearth = definition.abilities.iter().find(|ability| {
        matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_))
            && ability.functional_zones.contains(&ironsmith::Zone::Graveyard)
    }).expect("graveyard unearth activation");
    let debug = format!("{unearth:#?}");
    assert!(debug.contains("Energy") || debug.contains("energy"), "the cost must be eight energy: {debug}");
    assert!(debug.contains("UnearthEffect"), "{debug}");
}

#[test]
fn equipped_creature_attacking_the_life_leader_triggers() {
    let debug = p4_debug("Seraphic Greatsword", "Type: Artifact — Equipment\nEquipped creature gets +2/+2.\nWhenever equipped creature attacks the player with the most life or tied for most life, create a 4/4 white Angel creature token with flying that's tapped and attacking that player.\nEquip {4}");
    assert!(debug.contains("MostLifeTied"), "{debug}");
    assert!(debug.contains("CreateTokenEffect"), "{debug}");
}

#[test]
fn bargained_look_keeps_up_to_two_on_top_and_mills_the_rest() {
    let debug = p4_debug("Rowan's Grim Search", "Type: Instant\nBargain (You may sacrifice an artifact, enchantment, or token as you cast this spell.)\nIf this spell was bargained, look at the top four cards of your library, then put up to two of them back on top of your library in any order and the rest into your graveyard.\nYou draw two cards and you lose 2 life.");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("Bargain") || compact.contains("bargain"), "{debug}");
    assert!(compact.contains("zone:Graveyard"), "the rest must be milled: {debug}");
    assert!(compact.contains("DrawCardsEffect") && compact.contains("LoseLifeEffect"), "{debug}");
}

#[test]
fn random_graveyard_reorder_then_random_opponent_pick() {
    let debug = p4_debug("Search for Survivors", "Type: Sorcery\nReorder your graveyard at random. An opponent chooses a card at random in your graveyard. If it's a creature card, put it onto the battlefield. Otherwise, exile it.");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("ReorderGraveyardEffect{player:You,at_random:true"), "{debug}");
    assert!(compact.contains("ChooseObjectsEffect"), "{debug}");
}

#[test]
fn scroll_rack_swaps_hand_cards_through_face_down_exile() {
    let debug = p4_debug("Scroll Rack", "Type: Artifact\n{1}, {T}: Exile any number of cards from your hand face down. Put that many cards from the top of your library into your hand. Then look at the exiled cards and put them on top of your library in any order.");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("face_down:true") || compact.contains("FaceDown"), "{debug}");
    assert!(compact.matches("MoveToZoneEffect").count() >= 2, "{debug}");
}

#[test]
fn damage_to_you_as_though_infect_while_at_zero_or_less_life() {
    let debug = p4_debug("Phyrexian Unlife", "Type: Enchantment\nYou don't lose the game for having 0 or less life.\nAs long as you have 0 or less life, all damage is dealt to you as though its source had infect. (Damage is dealt to you in the form of poison counters.)");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("DamageAsThoughInfect(You)"), "{debug}");
    assert!(compact.contains("condition:Some("), "the infect rule is conditional: {debug}");
}

#[test]
fn per_creature_coin_flip_destroys_on_tails() {
    let debug = p4_debug("Rakdos, the Showstopper", "Type: Legendary Creature — Demon\nFlying, trample\nWhen Rakdos enters, flip a coin for each creature that isn't a Demon, Devil, or Imp. Destroy each creature whose coin comes up tails.");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("FlipCoinEffect"), "{debug}");
    assert!(compact.contains("DestroyEffect"), "{debug}");
    assert!(compact.contains("ForEachObject") || compact.contains("ForEachTagged"), "one flip per creature: {debug}");
}

#[test]
fn bounce_creature_and_its_attached_auras_together() {
    let debug = p4_debug("Orzhov Charm", "Type: Instant\nChoose one —\n• Return target creature you control and all Auras you control attached to it to their owner's hand.\n• Destroy target creature and you lose life equal to its toughness.\n• Return target creature card with mana value 1 or less from your graveyard to the battlefield.");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(compact.contains("Aura"), "{debug}");
    assert!(compact.contains("ChooseModeEffect"), "{debug}");
}
