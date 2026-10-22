//! Pending source-pass regressions: deliberately not executed before the batch
//! is ready, per the user's no-build instruction.
use super::compile_to_runtime_definition;
use ironsmith::{CardId, GameState, PlayerId, Zone};
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};

/// Frozen batch gate: check every face and report every failure in one run.
/// Authored only; execution remains deferred until builds are authorized.
#[test]
fn all_78_remaining_cards_compile_strictly() {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../reports/compilation-completion-20261016/sol-78-source-pass/all-78.fixture.json"
    )).unwrap();
    assert_eq!(cards.len(), 78);
    let mut failures = Vec::new();
    for card in cards {
        for face in card["faces"].as_array().unwrap() {
            let name = face["name"].as_str().unwrap();
            if let Err(error) = compile_to_runtime_definition(name, face["text"].as_str().unwrap(), false) {
                failures.push(format!("{} / {name}: {error}", card["name"].as_str().unwrap()));
            }
        }
    }
    assert!(failures.is_empty(), "{} face failures:\n{}", failures.len(), failures.join("\n"));
}

fn definition(name: &str) -> ironsmith::cards::CardDefinition {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../reports/compilation-completion-20261016/sol-78-source-pass/root-source-ready.fixture.json"
    )).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    compile_to_runtime_definition(name, row["text"].as_str().unwrap(), false)
        .unwrap_or_else(|error| panic!("{name}: {error:?}"))
}

#[test]
fn root_cards_compile_from_full_oracle_text() {
    for name in ["Arcum's Whistle", "Hold for Ransom", "Ashiok, Wicked Manipulator", "Ichor Elixir", "The Ruinous Powers"] {
        definition(name);
    }
}

#[test]
fn life_payment_replacement_uses_library_only_for_accepted_payments() {
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    for library_size in [2, 3, 5] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition("Ashiok, Wicked Manipulator"), a, Zone::Battlefield);
        let card = CardBuilder::new(CardId::new(), "Library card")
            .card_types(vec![ironsmith::CardType::Land]).build();
        for _ in 0..library_size { game.create_object_from_card(&card, a, Zone::Library); }
        game.refresh_continuous_state().unwrap();
        assert!(game.pay_life(a, 3).unwrap());
        let replaced = library_size >= 3;
        assert_eq!(game.player(a).unwrap().life, if replaced {20} else {17});
        assert_eq!(game.player(a).unwrap().library.len(), if replaced {library_size - 3} else {library_size});
        let mut ctx = EffectContext::new_default(source, a);
        execute_effect(&mut game, &Effect::lose_life(1), &mut ctx).unwrap();
        assert_eq!(game.player(a).unwrap().life, if replaced {19} else {16}, "ordinary life loss is never replaced");
        assert!(game.pay_life(b, 2).unwrap());
        assert_eq!(game.player(b).unwrap().life, 18, "the other player's payment is not replaced");
        assert!(!game.pay_life(a, 30).unwrap(), "a replacement does not make an unaffordable life cost legal");
    }
}

#[test]
fn named_grant_controller_sacrifices_attachment_and_draws_after_it_leaves() {
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Enchanted creature")
        .card_types(vec![ironsmith::CardType::Creature])
        .power_toughness(PowerToughness::fixed(2,2)).build();
    let host = game.create_object_from_card(&creature, b, Zone::Battlefield);
    let aura = game.create_object_from_definition(&definition("Hold for Ransom"), a, Zone::Battlefield);
    game.object_mut(aura).unwrap().attached_to = Some(ironsmith::object::AttachmentTarget::Object(host));
    game.create_object_from_card(&creature, a, Zone::Library);
    game.refresh_continuous_state().unwrap();
    let granted = game.current_abilities(host).unwrap().into_iter().find_map(|ability| {
        if let ironsmith::ability::AbilityKind::Activated(ability) = ability.kind { Some(ability) } else { None }
    }).unwrap();
    let mut ctx = EffectContext::new_default(host, b);
    ctx.set_tagged_objects(ironsmith::tag::GRANTING_SOURCE_TAG,
        vec![ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, aura).unwrap()]);
    for effect in granted.effects.all_effects() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    assert!(game.object(host).is_some_and(|object| object.zone == Zone::Battlefield));
    assert!(!game.battlefield.contains(&aura));
    assert_eq!(game.player(a).unwrap().hand.len(), 1);
    assert_eq!(game.player(b).unwrap().hand.len(), 0);
}

#[test]
fn planar_extra_die_ignores_unselected_symbols_and_counts_one_roll() {
    use ironsmith::decision::DecisionMaker;
    struct IgnoreFirst;
    impl DecisionMaker for IgnoreFirst {
        fn decide_options(&mut self, _: &GameState, _: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> { vec![0] }
    }
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    let deck = || (0..10).map(|i| (
        ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), format!("Plane {i}"))
            .card_types(vec![ironsmith::CardType::Plane]).build(), ironsmith::PlanarCardKind::Plane,
    )).collect();
    for effect_driven in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.enable_planechase(vec![(a, deck()), (b, deck())]).unwrap();
        game.reveal_starting_plane().unwrap();
        let source = game.create_object_from_definition(&definition("Ichor Elixir"), a, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        game.force_next_die_roll(1); // ignored planeswalker symbol
        game.force_next_die_roll(6); // retained blank
        game.force_next_die_roll(2); // must remain unconsumed
        let mut dm = IgnoreFirst;
        let before = game.face_up_planar_objects().to_vec();
        if effect_driven {
            let mut ctx = EffectContext::new(source, a, &mut dm);
            execute_effect(&mut game, &Effect::new(ironsmith::effects::RollPlanarDieEffect { player: ironsmith::PlayerFilter::You }), &mut ctx).unwrap();
        } else {
            assert_eq!(game.roll_planar_die_with_decision_maker(a, true, &mut dm).unwrap(), Some(ironsmith::PlanarDieFace::Blank));
        }
        assert_eq!(game.face_up_planar_objects(), before);
        assert_eq!(game.take_forced_die_roll(), Some(2));
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(a), 1);
    }
}

#[test]
fn delayed_cast_watcher_binds_only_the_immediately_preceding_grants() {
    use ironsmith::effects::{GrantPlayTaggedEffect, GrantPlayTaggedDuration, ScheduleDelayedTriggerEffect};
    use ironsmith::triggers::{SpellCastThroughPermissionTrigger, Trigger};
    let a = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Granted card").card_types(vec![ironsmith::CardType::Sorcery]).build();
    let source = game.create_object_from_definition(&definition("The Ruinous Powers"), a, Zone::Battlefield);
    let exiled = game.create_object_from_card(&card, a, Zone::Exile);
    let mut ctx = EffectContext::new_default(source, a);
    ctx.set_tagged_objects("grant_pool", vec![ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, exiled).unwrap()]);
    let grant = Effect::new(GrantPlayTaggedEffect::new("grant_pool", ironsmith::PlayerFilter::You,
        GrantPlayTaggedDuration::UntilEndOfTurn, true, ironsmith_core::value_model::ManaSpendMode::Normal));
    execute_effect(&mut game, &grant, &mut ctx).unwrap();
    let earlier = ctx.last_play_permissions.clone();
    execute_effect(&mut game, &grant, &mut ctx).unwrap();
    let current = ctx.last_play_permissions.clone();
    assert_ne!(earlier, current);
    let delayed = ScheduleDelayedTriggerEffect::new(
        Trigger::new(SpellCastThroughPermissionTrigger { permissions: Vec::new(), include_land_plays: false }),
        vec![Effect::lose_life(1)], true, Vec::new(), ironsmith::PlayerFilter::You,
    ).until_end_of_turn();
    execute_effect(&mut game, &Effect::new(delayed), &mut ctx).unwrap();
    let watcher = game.effect_store.delayed_triggers.last().unwrap().trigger
        .downcast_ref::<SpellCastThroughPermissionTrigger>().unwrap();
    assert_eq!(watcher.permissions, current);
    for grant in &game.effect_store.grant_registry.grants {
        let linked = grant.permission_identity.as_ref().is_some_and(|id| current.contains(id));
        assert_eq!(grant.play_from_constraints.requires_cast_receipt, linked);
    }
}
