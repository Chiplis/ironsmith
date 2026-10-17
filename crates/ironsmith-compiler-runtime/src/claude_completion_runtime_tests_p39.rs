//! Claude package 39 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::types::{CardType, Subtype};
use ironsmith::{PlayerId, Zone};

fn p39_static_ids(definition: &ironsmith::cards::CardDefinition) -> Vec<StaticAbilityId> {
    definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Static(ability) => Some(ability.id()),
        _ => None,
    }).collect()
}

fn p39_creature(game: &mut ironsmith::GameState, name: &str, owner: PlayerId, power: i32, toughness: i32) -> ironsmith::ids::ObjectId {
    let card = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness)).build();
    let id = game.create_object_from_card(&card, owner, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

const P39_CUNNING_GIANT: &str = "Mana cost: {5}{R}\nType: Creature — Giant\nPower/Toughness: 4/4\nIf this creature is unblocked, you may have it assign its combat damage to a creature defending player controls.";

#[test]
fn cunning_giant_unblocked_assigns_all_damage_to_one_defending_creature_or_the_player() {
    let definition = compile_to_runtime_definition("Cunning Giant", P39_CUNNING_GIANT, false)
        .expect("Cunning Giant should compile");
    assert!(p39_static_ids(&definition).contains(&StaticAbilityId::MayAssignUnblockedDamageToDefendingCreature));
    for redirect in [false, true] {
        let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let giant = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.remove_summoning_sickness(giant);
        let wall = p39_creature(&mut game, "Defending wall", bob, 0, 5);
        let own = p39_creature(&mut game, "Alice bystander", alice, 1, 1);
        game.refresh_continuous_state().unwrap();
        let combat = ironsmith::combat_state::CombatState {
            attackers: vec![ironsmith::combat_state::AttackerInfo {
                creature: giant, target: ironsmith::combat_state::AttackTarget::Player(bob),
            }],
            ..Default::default()
        };
        let prompt = ironsmith::game_loop::next_combat_damage_assignment_prompt(&game, &combat, false, None)
            .expect("the unblocked giant has a real choice");
        assert_eq!(prompt.source, giant);
        assert_eq!(prompt.player, alice);
        let targets = prompt.division.targets();
        assert!(targets.contains(&ironsmith::game_state::Target::Player(bob)));
        assert!(targets.contains(&ironsmith::game_state::Target::Object(wall)));
        assert!(!targets.contains(&ironsmith::game_state::Target::Object(own)));
        // Splitting between the player and the creature is illegal.
        assert!(prompt.validate(&game, &[
            (ironsmith::game_state::Target::Player(bob), 2),
            (ironsmith::game_state::Target::Object(wall), 2),
        ]).is_err());
        let choice = if redirect { ironsmith::game_state::Target::Object(wall) } else { ironsmith::game_state::Target::Player(bob) };
        prompt.record(&mut game, &[(choice, 4)]);
        ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(game.damage_on(wall), if redirect { 4 } else { 0 });
        assert_eq!(game.player(bob).unwrap().life, if redirect { 20 } else { 16 });
    }
    // A blocked giant has no such option.
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let giant = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let blocker = p39_creature(&mut game, "Blocker", bob, 1, 1);
    let _other = p39_creature(&mut game, "Other defender", bob, 1, 1);
    let mut combat = ironsmith::combat_state::CombatState {
        attackers: vec![ironsmith::combat_state::AttackerInfo {
            creature: giant, target: ironsmith::combat_state::AttackTarget::Player(bob),
        }],
        ..Default::default()
    };
    combat.blockers.insert(giant, vec![blocker]);
    assert!(ironsmith::game_loop::next_combat_damage_assignment_prompt(&game, &combat, false, None).is_none());
}

const P39_DAWNHAND_DISSIDENT: &str = "Mana cost: {B}\nType: Creature — Elf Warlock\nPower/Toughness: 1/2\n{T}, Blight 1: Surveil 1.\n{T}, Blight 2: Exile target card from a graveyard.\nDuring your turn, you may cast creature spells from among cards you own exiled with this creature by removing three counters from among creatures you control in addition to paying their other costs.";

#[test]
fn dawnhand_dissident_casts_its_exiled_creatures_only_by_paying_the_extra_counter_cost() {
    let definition = compile_to_runtime_definition("Dawnhand Dissident", P39_DAWNHAND_DISSIDENT, false)
        .expect("Dawnhand Dissident should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ZoneCastFromCardManaCost"), "{debug}");
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let dissident = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature_card = CardBuilder::new(CardId::new(), "Exiled creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)]))
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let mine = game.create_object_from_card(&creature_card, alice, Zone::Exile);
    let theirs = game.create_object_from_card(&creature_card, bob, Zone::Exile);
    let unlinked = game.create_object_from_card(&creature_card, alice, Zone::Exile);
    game.add_exiled_with_source_link(dissident, mine);
    game.add_exiled_with_source_link(dissident, theirs);
    for (turn_player, expected) in [(alice, true), (bob, false)] {
        game.turn.active_player = turn_player;
        game.refresh_continuous_state().unwrap();
        let casts = game.effect_store.grant_registry
            .granted_alternative_casts_for_card(&game, mine, Zone::Exile, alice);
        assert_eq!(!casts.is_empty(), expected, "during {turn_player:?}'s turn: {casts:?}");
        if expected {
            assert_eq!(casts[0].method.cast_from_zone(), Zone::Exile);
            assert_eq!(casts[0].method.non_mana_costs().len(), 1, "the counter removal is the extra cost");
        }
        // The plain (cost-free) permission no longer exists.
        assert!(game.effect_store.grant_registry.granted_play_from_for_card(&game, mine, Zone::Exile, alice).is_empty());
        assert!(game.effect_store.grant_registry
            .granted_alternative_casts_for_card(&game, theirs, Zone::Exile, alice).is_empty(), "only cards you own");
        assert!(game.effect_store.grant_registry
            .granted_alternative_casts_for_card(&game, unlinked, Zone::Exile, alice).is_empty(), "only cards exiled with it");
    }
}

const P39_DEIFICATION: &str = "Mana cost: {1}{W}\nType: Enchantment\nAs this enchantment enters, choose a planeswalker type.\nPlaneswalkers you control of the chosen type have hexproof.\nAs long as you control a creature, if damage dealt to a planeswalker you control of the chosen type would result in all loyalty counters on it being removed, instead all but one of those counters are removed.";

#[test]
fn deification_leaves_one_loyalty_counter_only_on_chosen_type_planeswalkers_while_you_control_a_creature() {
    use ironsmith::object::CounterType;
    let definition = compile_to_runtime_definition("Deification", P39_DEIFICATION, false)
        .expect("Deification should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("DamageRemovesAllCounters") && debug.contains("AllButOne"), "{debug}");
    for with_creature in [false, true] {
        let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let deification = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.set_chosen_creature_type(deification, Subtype::Jace);
        let walker = |game: &mut ironsmith::GameState, subtype: Subtype, owner: PlayerId| {
            let card = CardBuilder::new(CardId::new(), "Planeswalker witness")
                .card_types(vec![CardType::Planeswalker]).subtypes(vec![subtype]).build();
            let id = game.create_object_from_card(&card, owner, Zone::Battlefield);
            game.add_counters(id, CounterType::Loyalty, 3);
            id
        };
        let jace = walker(&mut game, Subtype::Jace, alice);
        let small = walker(&mut game, Subtype::Jace, alice);
        let chandra = walker(&mut game, Subtype::Chandra, alice);
        let bobs_jace = walker(&mut game, Subtype::Jace, bob);
        if with_creature { p39_creature(&mut game, "Alice creature", alice, 1, 1); }
        let source = p39_creature(&mut game, "Bob damage source", bob, 5, 5);
        game.refresh_continuous_state().unwrap();
        for (target, amount) in [(jace, 5), (small, 2), (chandra, 5), (bobs_jace, 5)] {
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, bob);
            ironsmith::effects::execute_effect(&mut game,
                &Effect::deal_damage(amount, ironsmith::target::ChooseSpec::SpecificObject(target)), &mut ctx).unwrap();
        }
        assert_eq!(game.counter_count(jace, CounterType::Loyalty), if with_creature { 1 } else { 0 });
        assert_eq!(game.counter_count(small, CounterType::Loyalty), 1, "damage that leaves counters is unchanged");
        assert_eq!(game.counter_count(chandra, CounterType::Loyalty), 0, "other planeswalker types are unaffected");
        assert_eq!(game.counter_count(bobs_jace, CounterType::Loyalty), 0, "only planeswalkers you control");
    }
}

const P39_DANCE_WITH_CALAMITY: &str = "Mana cost: {7}{R}\nType: Sorcery\nShuffle your library. As many times as you choose, you may exile the top card of your library. If the total mana value of the cards exiled this way is 13 or less, you may cast any number of spells from among those cards without paying their mana costs.";

#[test]
fn dance_with_calamity_exiles_as_many_times_as_chosen_and_sums_every_exiled_card() {
    struct Rounds { remaining: usize, prompts: usize }
    impl ironsmith::decision::DecisionMaker for Rounds {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.prompts += 1;
            if self.remaining > 0 { self.remaining -= 1; true } else { false }
        }
    }
    let definition = compile_to_runtime_definition("Dance with Calamity", P39_DANCE_WITH_CALAMITY, false)
        .expect("Dance with Calamity should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("RepeatProcessEffect") && debug.contains("exiled_as_many_times_as_chosen"), "{debug}");
    assert!(debug.contains("TotalManaValue"), "{debug}");
    for rounds in [0usize, 3, 4] {
        let alice = PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = CardBuilder::new(CardId::new(), "Four-drop sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(4)]))
            .build();
        for _ in 0..6 { game.create_object_from_card(&card, alice, Zone::Library); }
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut dm = Rounds { remaining: rounds, prompts: 0 };
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert_eq!(game.player(alice).unwrap().library.len(), 6 - rounds, "{rounds} rounds exile {rounds} cards");
        if rounds == 4 {
            // 16 total mana value: no casting is offered after the last round.
            assert_eq!(dm.prompts, rounds + 1);
            assert_eq!(game.exile.len(), 4);
        } else {
            assert!(dm.prompts >= rounds + 1);
        }
    }
}

const P39_CURSECLOTH_WRAPPINGS: &str = "Mana cost: {2}{B}{B}\nType: Artifact\nZombies you control get +1/+1.\n{T}: Target creature card in your graveyard gains embalm until end of turn. The embalm cost is equal to its mana cost. (Exile that card and pay its embalm cost: Create a token that's a copy of it, except it's a white Zombie in addition to its other types and has no mana cost. Embalm only as a sorcery.)";

#[test]
fn cursecloth_wrappings_grants_a_graveyard_embalm_paying_the_cards_own_mana_cost() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    let definition = compile_to_runtime_definition("Cursecloth Wrappings", P39_CURSECLOTH_WRAPPINGS, false)
        .expect("Cursecloth Wrappings should compile");
    let tap = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => Some(activated.clone()),
        _ => None,
    }).expect("the tap ability");
    let debug = format!("{:?}", tap.effects);
    assert!(debug.contains("source_mana_cost: true") && debug.contains("Embalm"), "{debug}");
    assert!(debug.contains("EndOfTurn"), "the grant lasts until end of turn: {debug}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let wrappings = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = CardBuilder::new(CardId::new(), "Buried creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
            ironsmith::mana::ManaSymbol::Generic(1), ironsmith::mana::ManaSymbol::Black]))
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let buried = game.create_object_from_card(&creature, alice, Zone::Graveyard);
    let before = game.current_abilities(buried).map_or(0, |abilities| abilities.len());
    let effects = tap.effects.all_effects();
    let target_spec = effects.iter().find_map(|effect| effect.0.get_target_spec()).unwrap().clone();
    let mut ctx = EffectContext::new_default(wrappings, alice)
        .with_targets(vec![ResolvedTarget::Object(buried)])
        .with_target_assignments(vec![ironsmith::game_state::TargetAssignment { spec: target_spec, range: 0..1 }]);
    for effect in effects { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    drop(ctx);
    game.refresh_continuous_state().unwrap();
    let abilities = game.current_abilities(buried).expect("the card is still in the graveyard");
    assert_eq!(abilities.len(), before + 1, "one granted embalm ability");
    let granted = abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) if ability.functional_zones.contains(&Zone::Graveyard) => Some(activated.clone()),
        _ => None,
    }).expect("embalm works from the graveyard");
    let cost = format!("{:?}", granted.mana_cost);
    assert!(cost.contains("source_mana_cost: true"), "pays the card's own mana cost: {cost}");
    assert_eq!(granted.timing, ironsmith::ability::ActivationTiming::SorcerySpeed);
}

const P39_DAMPING_ENGINE: &str = "Mana cost: {4}\nType: Artifact\nA player who controls more permanents than each other player can't play lands or cast artifact, creature, or enchantment spells. That player may sacrifice a permanent of their choice for that player to ignore this effect until end of turn.";

#[test]
fn damping_engine_restricts_the_permanent_leader_until_they_sacrifice_to_ignore_it() {
    use ironsmith::special_actions::SpecialAction;
    let definition = compile_to_runtime_definition("Damping Engine", P39_DAMPING_ENGINE, false)
        .expect("Damping Engine should compile");
    let ids = p39_static_ids(&definition);
    assert!(ids.contains(&StaticAbilityId::PlayerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn), "{ids:?}");
    let ignore_index = definition.abilities.iter().position(|ability| matches!(&ability.kind,
        AbilityKind::Static(ability) if ability.id() == StaticAbilityId::PlayerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn)).unwrap();
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    for _ in 0..3 { p39_creature(&mut game, "Alice fodder", alice, 1, 1); }
    let engine = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    p39_creature(&mut game, "Bob creature", bob, 1, 1);
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
    let restricted = |game: &ironsmith::GameState, player: PlayerId| {
        game.effect_store.cant_effects.cant_play_land_filters.contains_key(&player)
            && game.effect_store.cant_effects.cant_cast_filters.contains_key(&player)
    };
    assert!(restricted(&game, alice), "Alice controls the most permanents (4 to 1)");
    assert!(!restricted(&game, bob));
    let action = SpecialAction::IgnoreSourceEffect { source_id: engine, ability_index: ignore_index };
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    game.turn.priority_player = Some(bob);
    assert!(ironsmith::special_actions::can_perform(&action, &game, bob, &mut dm).is_err(), "only the restricted player");
    game.turn.priority_player = Some(alice);
    assert!(ironsmith::special_actions::can_perform(&action, &game, alice, &mut dm).is_ok());
    ironsmith::special_actions::perform(action, &mut game, alice, &mut dm).expect("sacrifice to ignore");
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1, "a permanent was sacrificed");
    assert!(game.battlefield.contains(&engine), "the fodder, not the engine, was sacrificed");
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
    assert!(!restricted(&game, alice), "Alice ignores the effect this turn");
}

const P39_DACK_FAYDEN: &str = "Mana cost: {4}{W}{W}\nType: Legendary Creature — Human Advisor\nPower/Toughness: 4/4\nWhen Dack Fayden enters, reveal cards from the top of your library until you reveal X creature cards, where X is the number of opponents you have. Put those creature cards onto the battlefield, then shuffle. They're goaded for the rest of the game. For each of those permanents, choose a different opponent. Each opponent gains control of the permanent for which they were chosen.";

#[test]
fn dack_fayden_gives_each_opponent_a_different_revealed_creature_goaded_forever() {
    let definition = compile_to_runtime_definition("Dack Fayden, Helping Hand", P39_DACK_FAYDEN, false)
        .expect("Dack Fayden should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Triggered(triggered) => Some(triggered.clone()),
        _ => None,
    }).expect("the enters trigger");
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("accumulate_into: Some"), "a different opponent per permanent: {debug}");
    let (alice, bob, carol) = (PlayerId::from_index(0), PlayerId::from_index(1), PlayerId::from_index(2));
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Revealed creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let land = CardBuilder::new(CardId::new(), "Revealed land").card_types(vec![CardType::Land]).build();
    for card in [&land, &creature, &land, &creature, &land] {
        game.create_object_from_card(card, alice, Zone::Library);
    }
    let dack = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(dack, alice);
    for effect in trigger.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    game.refresh_continuous_state().unwrap();
    let revealed = game.battlefield.iter().copied()
        .filter(|id| game.object(*id).is_some_and(|object| object.name.as_ref() == "Revealed creature"))
        .collect::<Vec<_>>();
    assert_eq!(revealed.len(), 2, "two opponents: two creatures enter");
    let mut controllers = revealed.iter().map(|id| game.controller_of_id(*id).unwrap()).collect::<Vec<_>>();
    controllers.sort_by_key(|player| player.0);
    assert_eq!(controllers, vec![bob, carol], "each opponent gets exactly one");
    for id in &revealed {
        assert!(game.effect_store.goad_effects.iter().any(|goad| goad.creature == *id
            && goad.duration == ironsmith::effect::Until::Forever), "goaded for the rest of the game");
    }
}

const P39_DANSE_MACABRE: &str = "Mana cost: {3}{B}{B}\nType: Sorcery\nEach player sacrifices a nontoken creature of their choice. Roll a d20 and add the toughness of the creature you sacrificed this way.\n1—14 | Return a creature card put into a graveyard this way to the battlefield under your control.\n15+ | Return up to two creature cards put into graveyards this way to the battlefield under your control.";

#[test]
fn danse_macabre_rolls_once_adding_your_sacrifice_and_returns_from_every_sacrifice() {
    struct TakeAll;
    impl ironsmith::decision::DecisionMaker for TakeAll {
        fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ids::ObjectId> {
            let legal = ctx.candidates.iter().filter(|c| c.legal).map(|c| c.id);
            match ctx.max { Some(max) => legal.take(max).collect(), None => legal.collect() }
        }
    }
    let definition = compile_to_runtime_definition("Danse Macabre", P39_DANSE_MACABRE, false)
        .expect("Danse Macabre should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("RollDieEffect"), "{debug}");
    assert_eq!(debug.matches("RollDieEffect").count(), 1, "one roll, by you: {debug}");
    assert!(debug.contains("TotalToughness"), "the roll adds your sacrificed creature's toughness: {debug}");
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    // Toughness 20 makes every roll 21 or more: the "15+" row.
    p39_creature(&mut game, "Alice's sturdy creature", alice, 1, 20);
    p39_creature(&mut game, "Bob's creature", bob, 2, 2);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut dm = TakeAll;
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let returned = game.battlefield.iter().copied()
        .filter(|id| game.object(*id).is_some_and(|object| object.name.as_ref() != "Danse Macabre"))
        .collect::<Vec<_>>();
    assert_eq!(returned.len(), 2, "both sacrificed creatures come back");
    assert!(returned.iter().all(|id| game.controller_of_id(*id) == Some(alice)), "under your control");
}
