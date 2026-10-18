//! Claude package 16 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::target::ChooseSpec;
use ironsmith::types::CardType;
use ironsmith::{PlayerId, Zone};

fn p16_visit(effect: &Effect, f: &mut dyn FnMut(&Effect)) {
    f(effect);
    effect.visit_child_effects(&mut |child| p16_visit(child, f));
}

fn p16_triggered(definition: &ironsmith::cards::CardDefinition) -> Vec<&ironsmith::ability::TriggeredAbility> {
    definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).collect()
}

fn p16_find_in_trigger<T: Clone + 'static>(definition: &ironsmith::cards::CardDefinition) -> Option<T> {
    let mut found = None;
    for trigger in p16_triggered(definition) {
        for effect in trigger.effects.all_effects() {
            p16_visit(effect, &mut |effect| {
                if found.is_none() && let Some(value) = effect.downcast_ref::<T>() {
                    found = Some(value.clone());
                }
            });
        }
    }
    found
}

fn p16_creature(game: &mut ironsmith::GameState, name: &str, controller: PlayerId) -> ironsmith::ids::ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[test]
fn bogbonder_moves_any_number_of_counters_of_every_kind_from_among_donors() {
    use ironsmith::object::CounterType;
    let definition = compile_to_runtime_definition("Slippery Bogbonder",
        "Type: Creature — Human Druid\nFlash\nHexproof\nWhen this creature enters, put a hexproof counter on target creature. Then move any number of counters from among creatures you control onto that creature.", false)
        .expect("Slippery Bogbonder should compile");
    let moved = p16_find_in_trigger::<ironsmith::effects::MoveCountersEffect>(&definition)
        .expect("counter move");
    assert_eq!(moved.count, ironsmith_core::effect::CounterMoveAmount::AnyNumberOfAnyKind, "{moved:?}");
    assert!(matches!(moved.from.base(), ChooseSpec::All(_)), "{moved:?}");

    struct Numbers { answers: std::collections::VecDeque<u32>, bounds: Vec<(u32, u32)> }
    impl ironsmith::decision::DecisionMaker for Numbers {
        fn decide_number(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::NumberContext) -> u32 {
            self.bounds.push((ctx.min, ctx.max));
            self.answers.pop_front().unwrap_or(ctx.min)
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let recipient = p16_creature(&mut game, "Recipient", alice);
    let first = p16_creature(&mut game, "First donor", alice);
    let second = p16_creature(&mut game, "Second donor", alice);
    game.object_mut(first).unwrap().counters.insert(CounterType::PlusOnePlusOne, 2);
    game.object_mut(first).unwrap().counters.insert(CounterType::Flying, 1);
    game.object_mut(second).unwrap().counters.insert(CounterType::Charge, 3);
    let mut effect = moved.clone();
    effect.to = ChooseSpec::SpecificObject(recipient);
    let mut dm = Numbers { answers: [1, 1, 3].into(), bounds: Vec::new() };
    let mut ctx = ironsmith::effects::EffectContext::new(recipient, alice, &mut dm);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap();
    drop(ctx);
    // One choice per (donor, kind) pair, bounded by that kind's count.
    assert_eq!(dm.bounds.len(), 3, "{:?}", dm.bounds);
    assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 1);
    assert_eq!(game.counter_count(first, CounterType::Flying), 0);
    assert_eq!(game.counter_count(second, CounterType::Charge), 0);
    assert_eq!(game.counter_count(recipient, CounterType::PlusOnePlusOne), 1);
    assert_eq!(game.counter_count(recipient, CounterType::Flying), 1);
    assert_eq!(game.counter_count(recipient, CounterType::Charge), 3);
}

#[test]
fn starving_revenant_draws_and_drains_per_card_surveil_kept_on_top() {
    let definition = compile_to_runtime_definition("Starving Revenant",
        "Type: Creature — Spirit Horror\nWhen this creature enters, surveil 2. Then for each card you put on top of your library, you draw a card and you lose 3 life.\nDescend 8 — Whenever you draw a card, if there are eight or more permanent cards in your graveyard, target opponent loses 1 life and you gain 1 life.", false)
        .expect("Starving Revenant should compile");
    let loop_effect = p16_find_in_trigger::<ironsmith::effects::ForEachTaggedEffect>(&definition)
        .expect("kept-on-top iteration");
    assert_eq!(loop_effect.tag.as_str(), ironsmith::tag::SURVEIL_KEPT_ON_TOP_TAG);

    struct Partition { to_graveyard: Vec<ironsmith::ids::ObjectId> }
    impl ironsmith::decision::DecisionMaker for Partition {
        fn decide_partition(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::PartitionContext) -> Vec<ironsmith::ids::ObjectId> {
            self.to_graveyard.clone()
        }
    }
    let trigger = p16_triggered(&definition).into_iter()
        .find(|trigger| format!("{:?}", trigger.effects).contains("Surveil"))
        .expect("ETB surveil trigger");
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let library_card = CardBuilder::new(CardId::new(), "Library card").card_types(vec![CardType::Instant]).build();
    for _ in 0..5 {
        game.create_object_from_card(&library_card, alice, Zone::Library);
    }
    let source = p16_creature(&mut game, "Revenant", alice);
    let top = *game.player(alice).unwrap().library.last().unwrap();
    let mut dm = Partition { to_graveyard: vec![top] };
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
    for effect in trigger.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let player = game.player(alice).unwrap();
    assert_eq!(player.graveyard.len(), 1);
    assert_eq!(player.hand.len(), 1, "one card kept on top means one draw");
    assert_eq!(player.life, 17, "one card kept on top means 3 life lost");
}

#[test]
fn silver_surfer_returns_only_lands_tapped_in_one_move() {
    use ironsmith::snapshot::ObjectSnapshot;
    let definition = compile_to_runtime_definition("Silver Surfer, Cosmic Voyager",
        "Type: Legendary Creature — Alien Hero\nFlash\nFlying\nWhen Silver Surfer enters, exile any number of other target permanents you control. Return those cards to the battlefield under their owner's control at the beginning of the next end step. If a land enters this way, it enters tapped.", false)
        .expect("Silver Surfer should compile");
    let mut found = None;
    for trigger in p16_triggered(&definition) {
        for effect in trigger.effects.all_effects() {
            p16_visit(effect, &mut |effect| {
                if let Some(move_back) = effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>()
                    && move_back.enters_tapped_if.is_some()
                {
                    found = Some(move_back.clone());
                }
            });
        }
    }
    let move_back = found.expect("delayed return with a per-object tapped entry");
    assert_eq!(move_back.zone, Zone::Battlefield);
    assert!(!move_back.enters_tapped, "only lands enter tapped");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let land = CardBuilder::new(CardId::new(), "Exiled land").card_types(vec![CardType::Land]).build();
    let creature = CardBuilder::new(CardId::new(), "Exiled creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1)).build();
    let land_id = game.create_object_from_card(&land, alice, Zone::Exile);
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Exile);
    let source = p16_creature(&mut game, "Surfer", alice);
    let mut effect = move_back.clone();
    effect.target = ChooseSpec::Tagged("p16_returned".into());
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ctx.set_tagged_objects("p16_returned", vec![
        ObjectSnapshot::from_object_id(&game, land_id).unwrap(),
        ObjectSnapshot::from_object_id(&game, creature_id).unwrap(),
    ]);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap();
    let on_battlefield: Vec<_> = game.battlefield.iter().copied()
        .filter(|id| *id != source)
        .collect();
    assert_eq!(on_battlefield.len(), 2, "both cards return together");
    for id in on_battlefield {
        let is_land = game.current_card_types(id).unwrap().contains(&CardType::Land);
        assert_eq!(game.is_tapped(id), is_land, "land tapped, creature untapped");
    }
}

#[test]
fn tahngarth_joins_the_attack_on_something_that_opponent_is_attacking() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo};
    let definition = compile_to_runtime_definition("Tahngarth, First Mate",
        "Type: Legendary Creature — Minotaur Warrior\nTahngarth can't be blocked by more than one creature.\nWhenever an opponent attacks with one or more creatures, if Tahngarth is tapped, you may have that opponent gain control of Tahngarth until end of combat. If you do, choose a player or planeswalker that opponent is attacking. Tahngarth is attacking that player or planeswalker.", false)
        .expect("Tahngarth should compile");
    let reselect = p16_find_in_trigger::<ironsmith::effects::ReselectAttackTargetEffect>(&definition)
        .expect("attack insertion");
    assert!(reselect.enters_combat, "{reselect:?}");
    assert!(reselect.among_attacked_by.is_some(), "{reselect:?}");
    assert!(reselect.attacked_player.is_none(), "{reselect:?}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = bob;
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let attacker = p16_creature(&mut game, "Bob's attacker", bob);
    let joiner = p16_creature(&mut game, "Lent minotaur", bob);
    let mut combat = ironsmith::combat_state::CombatState::default();
    combat.attackers.push(AttackerInfo { creature: attacker, target: AttackTarget::Player(alice) });
    game.combat = Some(combat);
    let mut effect = reselect.clone();
    effect.target = ChooseSpec::SpecificObject(joiner);
    effect.among_attacked_by = Some(ironsmith::target::PlayerFilter::Specific(bob));
    let mut ctx = ironsmith::effects::EffectContext::new_default(joiner, alice);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap();
    let target = game.combat.as_ref().unwrap().attackers.iter()
        .find(|info| info.creature == joiner).map(|info| info.target.clone());
    assert_eq!(target, Some(AttackTarget::Player(alice)));
}

#[test]
fn takeno_pumps_other_samurai_by_their_total_bushido() {
    let definition = compile_to_runtime_definition("Takeno, Samurai General",
        "Type: Legendary Creature — Human Samurai\nPower/Toughness: 3/3\nBushido 2 (Whenever this creature blocks or becomes blocked, it gets +2/+2 until end of turn.)\nEach other Samurai creature you control gets +1/+1 for each point of bushido it has.", false)
        .expect("Takeno should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("BushidoOfAffected"), "{debug}");
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let first = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let samurai_without_bushido = CardBuilder::new(CardId::new(), "Plain samurai")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![ironsmith::types::Subtype::Samurai])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let plain = game.create_object_from_card(&samurai_without_bushido, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(first), Some(3), "Takeno doesn't pump itself");
    assert_eq!(game.calculated_power(plain), Some(2), "no bushido, no bonus");
    let second = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(first), Some(5), "the other Takeno has bushido 2");
    assert_eq!(game.calculated_power(second), Some(5));
}

#[test]
fn sword_of_hearth_and_home_puts_the_exiled_and_searched_cards_onto_the_battlefield_together() {
    use ironsmith::snapshot::ObjectSnapshot;
    let definition = compile_to_runtime_definition("Sword of Hearth and Home",
        "Type: Artifact — Equipment\nEquipped creature gets +2/+2 and has protection from green and from white.\nWhenever equipped creature deals combat damage to a player, exile up to one target creature you own, then search your library for a basic land card. Put both cards onto the battlefield under your control, then shuffle.\nEquip {2}", false)
        .expect("Sword of Hearth and Home should compile");
    let mut found = None;
    for trigger in p16_triggered(&definition) {
        for effect in trigger.effects.all_effects() {
            p16_visit(effect, &mut |effect| {
                if let Some(moved) = effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>()
                    && moved.zone == Zone::Battlefield
                    && let ChooseSpec::All(filter) = moved.target.base()
                    && filter.any_of.len() == 2
                {
                    found = Some(moved.clone());
                }
            });
        }
    }
    let moved = found.expect("one union move onto the battlefield");
    let ChooseSpec::All(filter) = moved.target.base() else { unreachable!() };
    let tags: Vec<_> = filter.any_of.iter()
        .map(|member| (member.zone, member.tagged_constraints[0].tag.clone()))
        .collect();

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Exiled creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let land = CardBuilder::new(CardId::new(), "Searched land")
        .supertypes(vec![ironsmith::types::Supertype::Basic])
        .card_types(vec![CardType::Land]).build();
    let exiled = game.create_object_from_card(&creature, alice, Zone::Exile);
    let searched = game.create_object_from_card(&land, alice, Zone::Library);
    let source = p16_creature(&mut game, "Bearer", alice);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    for (zone, tag) in tags {
        let id = if zone == Some(Zone::Exile) { exiled } else { searched };
        ctx.set_tagged_objects(tag, vec![ObjectSnapshot::from_object_id(&game, id).unwrap()]);
    }
    ironsmith::effects::execute_effect(&mut game, &Effect::new(moved), &mut ctx).unwrap();
    assert_eq!(game.battlefield.len(), 3, "both cards enter alongside the bearer");
    assert!(game.player(alice).unwrap().library.is_empty());
}

#[test]
fn soul_sculptor_lasts_until_a_player_casts_a_creature_spell() {
    let definition = compile_to_runtime_definition("Soul Sculptor",
        "Type: Creature — Human\n{1}{W}, {T}: Target creature becomes an enchantment and loses all abilities until a player casts a creature spell.", false)
        .expect("Soul Sculptor should compile");
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None,
    }).expect("activated ability");
    let debug = format!("{:?}", ability.effects);
    assert!(debug.contains("AnyPlayerCastsCreatureSpell"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let target = p16_creature(&mut game, "Sculpted", bob);
    game.player_mut(bob).unwrap().creature_spells_cast_this_game = 2;
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice)
        .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(target)]);
    for effect in ability.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    game.refresh_continuous_state().unwrap();
    let types = game.current_card_types(target).unwrap();
    assert!(types.contains(&CardType::Enchantment) && !types.contains(&CardType::Creature), "{types:?}");
    // Earlier creature spells don't end it; the next one does.
    game.player_mut(alice).unwrap().creature_spells_cast_this_game = 1;
    game.refresh_continuous_state().unwrap();
    assert!(game.current_card_types(target).unwrap().contains(&CardType::Creature));
}

#[test]
fn tam_gives_hexproof_from_each_of_the_protected_creatures_own_colors() {
    let definition = compile_to_runtime_definition("Tam, Mindful First-Year",
        "Type: Legendary Creature — Gorgon Wizard\nEach other creature you control has hexproof from each of its colors.\n{T}: Target creature you control becomes all colors until end of turn.", false)
        .expect("Tam should compile");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let colored = |name: &str, symbol: ironsmith::mana::ManaSymbol| CardBuilder::new(CardId::new(), name)
        .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![symbol]]))
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1)).build();
    let protected = game.create_object_from_card(&colored("Green ward", ironsmith::mana::ManaSymbol::Green), alice, Zone::Battlefield);
    let green_source = game.create_object_from_card(&colored("Green attacker", ironsmith::mana::ManaSymbol::Green), bob, Zone::Battlefield);
    let blue_source = game.create_object_from_card(&colored("Blue attacker", ironsmith::mana::ManaSymbol::Blue), bob, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(!ironsmith::targeting::can_target_object(&game, protected, green_source, bob).is_legal(),
        "a green source shares the protected creature's color");
    assert!(ironsmith::targeting::can_target_object(&game, protected, blue_source, bob).is_legal(),
        "a blue source doesn't");
}

#[test]
fn sinister_concierge_grants_suspend_to_each_card_either_exile_produced() {
    let definition = compile_to_runtime_definition("Sinister Concierge",
        "Type: Creature — Human Wizard\nWhen this creature dies, you may exile it and put three time counters on it. If you do, exile up to one target creature and put three time counters on it. Each card exiled this way that doesn't have suspend gains suspend. (For each card with suspend, its owner removes a time counter from it at the beginning of their upkeep. When the last is removed, they may cast it without paying its mana cost. Those creature spells have haste.)", false)
        .expect("Sinister Concierge should compile");
    let trigger = p16_triggered(&definition).into_iter().next().expect("dies trigger");
    let mut loops = Vec::new();
    for effect in trigger.effects.all_effects() {
        p16_visit(effect, &mut |effect| {
            if let Some(each) = effect.downcast_ref::<ironsmith::effects::ForEachTaggedEffect>() {
                loops.push(each.clone());
            }
        });
    }
    let granting: Vec<_> = loops.iter()
        .filter(|each| format!("{:?}", each.effects).contains("Time"))
        .collect();
    assert_eq!(granting.len(), 2, "one suspend grant per exile instruction: {loops:#?}");
    assert_ne!(granting[0].tag, granting[1].tag);
}
