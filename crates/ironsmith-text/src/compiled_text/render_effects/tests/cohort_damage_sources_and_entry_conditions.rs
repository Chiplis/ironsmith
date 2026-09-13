use super::*;
use crate::card::PowerToughness;
use crate::game_state::{GameState, StackEntry};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::mana::{ManaCost, ManaSymbol};

fn creature(name: &str, power: i32, toughness: i32, text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .parse_text(text)
        .unwrap()
}

fn entered(source: ObjectId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            source, Zone::Stack, Zone::Battlefield,
            crate::events::cause::EventCause::effect(), None,
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

fn queue(game: &mut GameState, event: crate::triggers::TriggerEvent) -> usize {
    let triggers = crate::triggers::check_triggers(game, &event);
    let count = triggers.len();
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers { queue.add(trigger); }
    crate::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
    count
}

#[test]
fn cohort_entering_aura_is_damage_source_and_gained_loss_follows_attachment() {
    let aura = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Grounding")
        .card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura])
        .parse_text("Enchant creature\nLifelink\nWhen this Aura enters, if enchanted creature has flying, this Aura deals 2 damage to that creature and this Aura gains \"Enchanted creature loses flying.\"").unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&aura).join("\n");
    assert!(rendered.contains("if enchanted creature has flying"), "{rendered}");
    assert!(rendered.contains("gains \"Enchanted creature loses flying.\""), "{rendered}");
    for initially_flying in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let host = game.create_object_from_definition(
            &creature("Old host", 2, 10, if initially_flying { "Flying" } else { "" }),
            bob, Zone::Battlefield,
        );
        let other = game.create_object_from_definition(&creature("New host", 2, 10, "Flying"), bob, Zone::Battlefield);
        let source = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
        game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(host));
        assert_eq!(queue(&mut game, entered(source)), usize::from(initially_flying));
        if initially_flying { crate::game_loop::resolve_stack_entry(&mut game).unwrap(); }
        assert_eq!(game.damage_on(host), if initially_flying { 2 } else { 0 });
        assert_eq!(game.player(alice).unwrap().life, 20 + if initially_flying { 2 } else { 0 }, "the Aura's lifelink applies to its damage");
        assert_eq!(game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::RemoveAbilityForFilter), initially_flying);
        assert!(!game.current_has_static_ability_id(host, crate::static_abilities::StaticAbilityId::Flying));
        game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(other));
        assert_eq!(game.current_has_static_ability_id(host, crate::static_abilities::StaticAbilityId::Flying), initially_flying);
        assert_eq!(game.current_has_static_ability_id(other, crate::static_abilities::StaticAbilityId::Flying), !initially_flying, "only an Aura whose entry condition held gains the persistent ability");
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert!(game.current_has_static_ability_id(other, crate::static_abilities::StaticAbilityId::Flying));
    }
}

#[test]
fn cohort_amassed_army_deals_its_power_with_its_own_lifelink_to_non_armies() {
    let spell = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Formation")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Amass Zombies 2, then the Army you amassed deals damage equal to its power to each non-Army creature.").unwrap();
    for existing in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let army = crate::CardDefinitionBuilder::new(CardId::new(), "Existing army")
            .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Army])
            .power_toughness(PowerToughness::fixed(3, 10)).parse_text("Lifelink").unwrap();
        let our_army = existing.then(|| game.create_object_from_definition(&army, alice, Zone::Battlefield));
        let enemy_army = game.create_object_from_definition(&army, bob, Zone::Battlefield);
        let ours = game.create_object_from_definition(&creature("Our non-Army", 1, 10, ""), alice, Zone::Battlefield);
        let theirs = game.create_object_from_definition(&creature("Their non-Army", 1, 10, ""), bob, Zone::Battlefield);
        let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
        game.stack.push(StackEntry::new(source, alice));
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        let damage = if existing { 5 } else { 2 };
        assert_eq!(game.damage_on(ours), damage);
        assert_eq!(game.damage_on(theirs), damage);
        assert_eq!(game.damage_on(enemy_army), 0);
        assert_eq!(game.player(alice).unwrap().life, 20 + if existing { 10 } else { 0 }, "damage must come from the amassed Army, including its lifelink");
        assert_eq!(game.player(bob).unwrap().life, 20);
        let armies = game.battlefield.iter().copied().filter(|id| game.current_controller(*id) == Some(alice) && game.calculated_subtypes(*id).contains(&Subtype::Army)).collect::<Vec<_>>();
        assert_eq!(armies.len(), 1);
        assert_eq!(game.current_power(armies[0]), Some(damage as i32));
        assert_eq!(game.damage_on(armies[0]), 0);
        assert!(game.calculated_subtypes(armies[0]).contains(&Subtype::Zombie));
        if let Some(id) = our_army { assert_eq!(armies[0], id); }
    }
}

fn record_cast(game: &mut GameState, id: ObjectId, caster: PlayerId) {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(), game);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::SpellCastEvent::new_with_snapshot(id, caster, Zone::Hand, snapshot),
        crate::provenance::ProvNodeId::default(),
    );
    game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event);
}

#[test]
fn cohort_entry_unless_another_red_spell_excludes_self_other_players_and_old_turns() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Impetuous")
        .card_types(vec![CardType::Creature]).mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]))
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("Haste\nThis creature enters with two -1/-1 counters on it unless you've cast another red spell this turn.").unwrap();
    for case in 0..6 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        if case > 0 && case != 5 {
            let other = crate::CardDefinitionBuilder::new(CardId::new(), "Recorded spell")
                .card_types(vec![CardType::Sorcery])
                .mana_cost(ManaCost::from_symbols(vec![if case == 3 { ManaSymbol::Blue } else { ManaSymbol::Red }])).build();
            let caster = if case == 2 { bob } else { alice };
            let id = game.create_object_from_definition(&other, caster, Zone::Stack);
            record_cast(&mut game, id, caster);
            game.move_object_by_effect(id, Zone::Graveyard).unwrap();
            if case == 4 { game.next_turn(); game.next_turn(); }
        }
        let mut stack = game.create_object_from_definition(&card, alice, Zone::Stack);
        if case == 5 {
            // An earlier cast of this same physical card is another spell;
            // only the currently entering incarnation is excluded.
            record_cast(&mut game, stack, alice);
            let hand = game.move_object_by_effect(stack, Zone::Hand).unwrap();
            stack = game.move_object_by_effect(hand, Zone::Stack).unwrap();
        }
        record_cast(&mut game, stack, alice);
        let entered = game.move_object_with_etb_processing(stack, Zone::Battlefield).unwrap().new_id;
        let expected = if case == 1 || case == 5 { 0 } else { 2 };
        assert_eq!(game.counter_count(entered, crate::CounterType::MinusOneMinusOne), expected, "case={case}");
        assert_eq!(game.current_power(entered), Some(4 - expected as i32));
    }
}

#[test]
fn cohort_each_player_sacrifices_their_own_nontoken_creature_or_loses_life() {
    struct Payments { accept: Vec<bool>, selected: Vec<ObjectId>, players: Vec<PlayerId> }
    impl crate::decision::DecisionMaker for Payments {
        fn decide_boolean(&mut self, _: &GameState, ctx: &crate::decisions::context::BooleanContext) -> bool {
            self.accept[self.players.iter().position(|id| *id == ctx.player).unwrap()]
        }
        fn decide_objects(&mut self, game: &GameState, ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            let chosen = self.selected[self.players.iter().position(|id| *id == ctx.player).unwrap()];
            assert!(ctx.candidates.iter().any(|candidate| candidate.id == chosen && candidate.legal));
            for candidate in ctx.candidates.iter().filter(|candidate| candidate.legal) {
                assert_eq!(game.current_controller(candidate.id), Some(ctx.player));
                assert!(game.object_has_card_type(candidate.id, CardType::Creature));
                assert!(!matches!(game.object(candidate.id).unwrap().kind, crate::object::ObjectKind::Token));
            }
            vec![chosen]
        }
    }
    let card = creature("Ravelon Tollkeeper", 4, 6, "At the beginning of your end step, each player loses 4 life unless they sacrifice a nontoken creature of their choice.");
    for mask in 0..16 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let players = game.players.iter().map(|p| p.id).collect::<Vec<_>>();
        let source = game.create_object_from_definition(&card, players[0], Zone::Battlefield);
        let selected = players.iter().map(|player| game.create_object_from_definition(&creature("Sacrifice candidate", 2, 2, ""), *player, Zone::Battlefield)).collect::<Vec<_>>();
        let bob_has_no_candidate = mask & 8 != 0;
        if bob_has_no_candidate { game.move_object_by_effect(selected[1], Zone::Hand).unwrap(); }
        let token = crate::CardDefinitionBuilder::new(CardId::new(), "Excluded token").token().card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2)).build();
        let artifact = crate::CardDefinitionBuilder::new(CardId::new(), "Excluded artifact").card_types(vec![CardType::Artifact]).build();
        let mut excluded = vec![];
        for player in &players {
            excluded.push(game.create_object_from_definition(&token, *player, Zone::Battlefield));
            excluded.push(game.create_object_from_definition(&artifact, *player, Zone::Battlefield));
        }
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfEndStepEvent::new(players[0]), crate::provenance::ProvNodeId::default(),
        );
        assert_eq!(queue(&mut game, event), 1);
        let accept = (0..3).map(|index| mask & (1 << index) != 0).collect::<Vec<_>>();
        let mut payments = Payments { accept: accept.clone(), selected: selected.clone(), players: players.clone() };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut payments).unwrap();
        for index in 0..3 {
            let available = index != 1 || !bob_has_no_candidate;
            let paid = accept[index] && available;
            assert_eq!(game.player(players[index]).unwrap().life, if paid { 20 } else { 16 }, "mask={mask}, player={index}");
            assert_eq!(game.battlefield.contains(&selected[index]), available && !paid);
        }
        assert!(game.battlefield.contains(&source));
        assert!(excluded.iter().all(|id| game.battlefield.contains(id)));
    }
}

#[test]
fn cohort_counter_condition_freezes_chosen_enemy_only_at_resolution_for_one_untap() {
    let card = creature("Ravelon Regulator", 2, 3, "Flying\nWhen this creature enters, choose target creature you don't control and tap it. If you control a creature with a counter on it, the chosen creature doesn't untap during its controller's next untap step.");
    for case in 0..4 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let own = game.create_object_from_definition(&creature("Own creature", 1, 4, ""), alice, Zone::Battlefield);
        let target = game.create_object_from_definition(&creature("Enemy creature", 1, 4, ""), bob, Zone::Battlefield);
        let artifact = crate::CardDefinitionBuilder::new(CardId::new(), "Counter artifact").card_types(vec![CardType::Artifact]).build();
        let artifact = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
        if case > 0 {
            game.add_counters(match case { 1 => own, 2 => target, _ => artifact }, crate::CounterType::Shield, 1);
        }
        assert_eq!(queue(&mut game, entered(source)), 1);
        assert_eq!(game.stack.last().unwrap().targets, vec![crate::game_state::Target::Object(target)]);
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.is_tapped(target));
        assert!(!game.is_tapped(own));
        if case == 1 { game.remove_counters(own, crate::CounterType::Shield, 1, None, None).unwrap(); }
        else { game.add_counters(own, crate::CounterType::Shield, 1); }
        game.next_turn();
        assert_eq!(game.turn.active_player, bob);
        crate::turn::execute_untap_step(&mut game);
        assert_eq!(game.is_tapped(target), case == 1, "counter condition is checked at resolution, case={case}");
        game.next_turn(); game.next_turn();
        crate::turn::execute_untap_step(&mut game);
        assert!(!game.is_tapped(target), "the second untap is unaffected");
    }
}
