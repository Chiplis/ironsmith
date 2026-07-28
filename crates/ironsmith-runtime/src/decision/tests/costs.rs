use super::*;

#[test]
fn krrik_keeps_black_spell_costs_as_black_pips() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source = CardBuilder::new(CardId::from_raw(7000), "Krrik Cost Helper")
        .card_types(vec![CardType::Creature])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
    game.object_mut(source_id)
        .expect("helper permanent should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::krrik_black_mana_may_be_paid_with_life(),
        ));

    let spell = CardBuilder::new(CardId::from_raw(7001), "Black Cost Probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let spell_obj = game.object(spell_id).expect("spell should exist");
    let base_cost = spell_obj
        .mana_cost
        .as_ref()
        .expect("spell should have a cost");

    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{1}{B}{B}");
}

#[test]
fn trinisphere_raises_single_black_spell_to_three_total_mana() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source = CardBuilder::new(CardId::from_raw(7002), "Trinisphere Helper")
        .card_types(vec![CardType::Artifact])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
    game.object_mut(source_id)
        .expect("helper permanent should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::minimum_spell_total_mana(3),
        ));

    let spell = CardBuilder::new(CardId::from_raw(7003), "Cheap Black Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Black]))
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let spell_obj = game.object(spell_id).expect("spell should exist");
    let base_cost = spell_obj
        .mana_cost
        .as_ref()
        .expect("spell should have a cost");

    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{B}{2}");
    assert_eq!(effective.mana_value(), 3);
}

#[test]
fn trinisphere_counts_krrik_life_paid_black_pips_toward_floor() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let krrik = CardBuilder::new(CardId::from_raw(7004), "Krrik Cost Helper")
        .card_types(vec![CardType::Creature])
        .build();
    let krrik_id = game.create_object_from_card(&krrik, alice, Zone::Battlefield);
    game.object_mut(krrik_id)
        .expect("krrik helper should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::krrik_black_mana_may_be_paid_with_life(),
        ));

    let trini = CardBuilder::new(CardId::from_raw(7005), "Trinisphere Helper")
        .card_types(vec![CardType::Artifact])
        .build();
    let trini_id = game.create_object_from_card(&trini, alice, Zone::Battlefield);
    game.object_mut(trini_id)
        .expect("trinisphere helper should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::minimum_spell_total_mana(3),
        ));

    let spell = CardBuilder::new(CardId::from_raw(7006), "Necro Probe")
        .card_types(vec![CardType::Enchantment])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Black,
            ManaSymbol::Black,
            ManaSymbol::Black,
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let effective = {
        let spell_obj = game.object(spell_id).expect("spell should exist");
        let base_cost = spell_obj
            .mana_cost
            .as_ref()
            .expect("spell should have a cost");
        calculate_effective_mana_cost(&game, alice, spell_obj, base_cost)
    };

    assert_eq!(effective.to_oracle(), "{B}{B}{B}");
    assert_eq!(effective.mana_value(), 3);
    assert!(
        game.try_pay_mana_cost_with_reason(
            alice,
            Some(spell_id),
            &effective,
            0,
            PaymentReason::CastSpell
        ),
        "three black pips should already satisfy Trinisphere even when Krrik pays them with life"
    );
    assert_eq!(game.player(alice).expect("alice exists").life, 14);
}

#[test]
fn yasharn_blocks_krrik_life_payment_without_rewriting_spell_costs() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let krrik = CardBuilder::new(CardId::from_raw(7007), "Krrik Cost Helper")
        .card_types(vec![CardType::Creature])
        .build();
    let krrik_id = game.create_object_from_card(&krrik, alice, Zone::Battlefield);
    game.object_mut(krrik_id)
        .expect("krrik helper should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::krrik_black_mana_may_be_paid_with_life(),
        ));

    let yasharn = CardBuilder::new(CardId::from_raw(7008), "Yasharn Cost Helper")
        .card_types(vec![CardType::Creature])
        .build();
    let yasharn_id = game.create_object_from_card(&yasharn, alice, Zone::Battlefield);
    game.object_mut(yasharn_id)
        .expect("yasharn helper should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
        ));

    let spell = CardBuilder::new(CardId::from_raw(7009), "Yasharn Probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Black,
            ManaSymbol::Black,
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let spell_obj = game.object(spell_id).expect("spell should exist");
    let base_cost = spell_obj
        .mana_cost
        .as_ref()
        .expect("spell should have a cost");

    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{B}{B}");
    assert!(
        !game.can_pay_mana_cost_with_reason(
            alice,
            Some(spell_id),
            &effective,
            0,
            PaymentReason::CastSpell
        ),
        "without black mana in the pool, Yasharn should remove Krrik's life-payment option"
    );
}

#[test]
fn emerge_alternative_cost_reduces_generic_by_sacrificed_creature_mana_value() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let sacrifice = CardBuilder::new(CardId::from_raw(7010), "Silvercoat Lion")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(1),
            ManaSymbol::White,
        ]))
        .build();
    game.create_object_from_card(&sacrifice, alice, Zone::Battlefield);

    let spell = CardBuilder::new(CardId::from_raw(7011), "Wretched Gryff")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(7),
            ManaSymbol::Blue,
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let emerge = crate::alternative_cast::AlternativeCastingMethod::alternative_cost(
        "Emerge",
        Some(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(5),
            ManaSymbol::Blue,
        ])),
        vec![crate::costs::Cost::sacrifice(
            ObjectFilter::creature().you_control(),
        )],
    );
    game.object_mut(spell_id)
        .expect("emerge spell should exist")
        .alternative_casts
        .push(emerge);

    let spell_obj = game.object(spell_id).expect("emerge spell should exist");
    let reduced = spell_mana_cost_for_cast(
        &game,
        alice,
        spell_obj,
        &CastingMethod::Alternative(0),
        Zone::Hand,
    )
    .expect("emerge cost should resolve");

    assert_eq!(reduced.to_oracle(), "{3}{U}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn yasharn_blocks_force_of_will_alternative_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let bolt_id = game.create_object_from_definition(&lightning_bolt(), bob, Zone::Stack);
    game.stack.push(crate::StackEntry::new(bolt_id, bob));

    let yasharn = CardBuilder::new(CardId::from_raw(7010), "Yasharn Cost Helper")
        .card_types(vec![CardType::Creature])
        .build();
    let yasharn_id = game.create_object_from_card(&yasharn, alice, Zone::Battlefield);
    game.object_mut(yasharn_id)
        .expect("yasharn helper should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
        ));

    let fow_id = game.create_object_from_definition(&force_of_will(), alice, Zone::Hand);
    game.create_object_from_definition(&counterspell(), alice, Zone::Hand);

    let fow_obj = game.object(fow_id).expect("force of will should exist");
    let method = &fow_obj.alternative_casts[0];
    assert!(
        !can_cast_with_alternative_from_hand(&game, alice, fow_obj, fow_id, method),
        "Yasharn should stop Force of Will's alternative cost because it includes paying life"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn trinisphere_requires_three_mana_for_force_of_will_alternative_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let bolt_id = game.create_object_from_definition(&lightning_bolt(), bob, Zone::Stack);
    game.stack.push(crate::StackEntry::new(bolt_id, bob));

    let trini = CardBuilder::new(CardId::from_raw(7011), "Trinisphere Helper")
        .card_types(vec![CardType::Artifact])
        .build();
    let trini_id = game.create_object_from_card(&trini, alice, Zone::Battlefield);
    game.object_mut(trini_id)
        .expect("trinisphere helper should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::minimum_spell_total_mana(3),
        ));

    let fow_id = game.create_object_from_definition(&force_of_will(), alice, Zone::Hand);
    game.create_object_from_definition(&counterspell(), alice, Zone::Hand);

    for _ in 0..2 {
        game.create_object_from_definition(&basic_island(), alice, Zone::Battlefield);
    }
    let fow_obj = game.object(fow_id).expect("force of will should exist");
    let method = &fow_obj.alternative_casts[0];
    assert!(
        !can_cast_with_alternative_from_hand(&game, alice, fow_obj, fow_id, method),
        "Trinisphere should make Force of Will's free alternative cost require three mana"
    );

    game.create_object_from_definition(&basic_island(), alice, Zone::Battlefield);
    let fow_obj = game.object(fow_id).expect("force of will should exist");
    let method = &fow_obj.alternative_casts[0];
    assert!(
        can_cast_with_alternative_from_hand(&game, alice, fow_obj, fow_id, method),
        "with three Islands available, the alternative cost should become legal again"
    );
}

fn stage_noncombat_damage_to_player_for_test(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: u32,
) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Player(player),
            amount,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);
}

fn stage_combat_damage_to_player_for_test(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: u32,
) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Player(player),
            amount,
            true,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);
}

fn stage_artifact_sacrifice_for_test(game: &mut GameState, player: PlayerId) {
    let artifact = CardBuilder::new(CardId::new(), "Sacrificed Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_id = game.create_object_from_card(&artifact, player, Zone::Battlefield);
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(artifact_id).expect("artifact exists"),
        game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::permanents::SacrificeEvent::new(artifact_id, None)
            .with_snapshot(Some(snapshot), Some(player)),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);
}

#[test]
fn test_compute_legal_actions_basic() {
    let game = setup_game();
    let alice = PlayerId::from_index(0);

    let actions = compute_legal_actions(&game, alice);

    // Should at least have pass priority
    assert!(actions.contains(&LegalAction::PassPriority));
}

#[test]
fn test_compute_legal_actions_surfaces_activated_ability_before_mana_payment() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let sink = CardDefinitionBuilder::new(CardId::from_raw(700_950), "Mana Sink Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("{B}{B}: Draw a card.")
        .expect("activated ability text should parse");
    let sink_id = game.create_object_from_definition(&sink, alice, Zone::Battlefield);

    let activations_for_sink = |game: &GameState| {
        compute_legal_actions(game, alice)
            .into_iter()
            .filter(|action| {
                matches!(
                    action,
                    LegalAction::ActivateAbility { source, .. } if *source == sink_id
                )
            })
            .count()
    };

    assert_eq!(
        activations_for_sink(&game),
        1,
        "an ability costing {{B}}{{B}} should still surface before mana is floated"
    );

    let swamp = CardDefinitionBuilder::new(CardId::from_raw(700_951), "Swamp")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {B}.")
        .expect("swamp mana text should parse");
    game.create_object_from_definition(&swamp, alice, Zone::Battlefield);
    game.create_object_from_definition(&swamp, alice, Zone::Battlefield);

    assert_eq!(
        activations_for_sink(&game),
        1,
        "two untapped Swamps make {{B}}{{B}} potentially payable"
    );
}

#[test]
fn test_compute_legal_actions_counts_floating_mana_for_activated_ability() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let sink = CardDefinitionBuilder::new(CardId::from_raw(700_952), "Mana Sink Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("{B}{B}: Draw a card.")
        .expect("activated ability text should parse");
    let sink_id = game.create_object_from_definition(&sink, alice, Zone::Battlefield);

    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Black, 2);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == sink_id
            )
        }),
        "floating {{B}}{{B}} should keep the ability visible"
    );
}

#[test]
fn test_compute_legal_actions_with_land() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    // Add a land to hand
    let land = CardBuilder::new(CardId::from_raw(1), "Forest")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, alice, Zone::Hand);

    let actions = compute_legal_actions(&game, alice);

    // Should have play land action
    assert!(actions.contains(&LegalAction::PlayLand { land_id }));
}

#[test]
fn test_compute_legal_actions_includes_graveyard_land_with_play_from_grant() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let land = CardBuilder::new(CardId::from_raw(71_018), "Ash Barrens")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, alice, Zone::Graveyard);

    let source_id = game.new_object_id();
    game.effect_store
        .grant_registry
        .grant_to_filter_until_end_of_turn(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
            alice,
            Grantable::play_from(),
            source_id,
            game.turn.turn_number,
        );

    let actions = compute_legal_actions(&game, alice);

    assert!(
        actions.contains(&LegalAction::PlayLand { land_id }),
        "play-from-graveyard grants should surface playable lands as land actions"
    );
}

#[test]
fn test_compute_legal_actions_excludes_graveyard_land_after_land_play_used() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let land = CardBuilder::new(CardId::from_raw(71_019), "Haunted Mire")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, alice, Zone::Graveyard);

    let source_id = game.new_object_id();
    game.effect_store
        .grant_registry
        .grant_to_filter_until_end_of_turn(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
            alice,
            Grantable::play_from(),
            source_id,
            game.turn.turn_number,
        );

    game.player_mut(alice)
        .expect("alice should exist")
        .record_land_play();

    let actions = compute_legal_actions(&game, alice);

    assert!(
        !actions.contains(&LegalAction::PlayLand { land_id }),
        "granted graveyard land plays must still respect the per-turn land limit"
    );
}

#[test]
fn test_compute_legal_actions_includes_exile_land_with_play_from_grant() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let land = CardBuilder::new(CardId::from_raw(71_020), "Forgotten Cave")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, alice, Zone::Exile);

    let source_id = game.new_object_id();
    game.effect_store
        .grant_registry
        .grant_to_filter_until_end_of_turn(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Exile,
            alice,
            Grantable::play_from(),
            source_id,
            game.turn.turn_number,
        );

    let actions = compute_legal_actions(&game, alice);

    assert!(
        actions.contains(&LegalAction::PlayLand { land_id }),
        "public-zone play-from grants should continue to surface exile lands"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn simple_battlefield_mana_ability_output_recognizes_basic_land_tap() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let forest = CardDefinitionBuilder::new(CardId::from_raw(700_901), "Forest")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {G}.")
        .expect("forest mana text should parse");
    let forest_id = game.create_object_from_definition(&forest, alice, Zone::Battlefield);
    let ability = game
        .current_ability(forest_id, 0)
        .expect("forest should expose a mana ability");
    let view = DerivedGameView::new(&game);

    assert_eq!(
        simple_battlefield_mana_ability_output(&game, alice, forest_id, 0, &ability, &view),
        Some(vec![ManaSymbol::Green]),
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn simple_battlefield_mana_ability_output_ignores_non_mana_activated_abilities() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let yawgmoth = crate::cards::definitions::yawgmoth_thran_physician();
    let yawgmoth_id = game.create_object_from_definition(&yawgmoth, alice, Zone::Battlefield);
    let ability = game
        .current_ability(yawgmoth_id, 0)
        .expect("Yawgmoth should expose its first activated ability");
    let view = DerivedGameView::new(&game);

    assert_eq!(
        simple_battlefield_mana_ability_output(&game, alice, yawgmoth_id, 0, &ability, &view),
        None,
    );
}

#[test]
fn test_select_first_decision_maker_supports_multi_target_requirement() {
    let first = Target::Object(ObjectId::from_raw(1));
    let second = Target::Object(ObjectId::from_raw(2));
    let ctx = TargetsContext::new(
        PlayerId::from_index(0),
        ObjectId::from_raw(99),
        "test spell",
        vec![TargetRequirementContext {
            description: "two targets".to_string(),
            legal_targets: vec![first, second],
            legal_target_sets: Vec::new(),
            min_targets: 2,
            max_targets: Some(2),
            distinct_player_group: None,
        }],
    );

    let mut dm = SelectFirstDecisionMaker;
    let chosen = dm.decide_targets(&setup_game(), &ctx);

    assert_eq!(chosen, vec![first, second]);
}

/// Tests computation of legal attackers during declare attackers step.
///
/// Scenario: Alice controls a Grizzly Bears that has been on the battlefield
/// since the beginning of her turn (no summoning sickness). When computing
/// legal attackers, it should be available to attack Bob (player 1).
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_compute_legal_attackers() {
    use crate::cards::definitions::grizzly_bears;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Create Grizzly Bears on battlefield
    let bears_def = grizzly_bears();
    let creature_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);

    // Remove summoning sickness (creature has been on battlefield since turn start)
    game.remove_summoning_sickness(creature_id);

    let combat = CombatState::default();
    let options = compute_legal_attackers(&game, &combat);

    assert_eq!(options.len(), 1, "Should have one legal attacker");
    assert_eq!(options[0].creature, creature_id);
    assert!(
        !options[0].must_attack,
        "Grizzly Bears doesn't have 'must attack'"
    );
    // Should be able to attack Bob (player 1)
    assert!(
        options[0]
            .valid_targets
            .contains(&AttackTarget::Player(bob)),
        "Should be able to attack the opponent"
    );
}

#[test]
fn compute_legal_attackers_reuses_one_derived_view_for_many_candidates() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    for index in 0..32 {
        let creature = CardBuilder::new(
            CardId::from_raw(701_100 + index),
            &format!("Attack Candidate {index}"),
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.remove_summoning_sickness(creature_id);
    }
    game.refresh_continuous_state();

    let before = game.work_counters();
    let options = compute_legal_attackers(&game, &CombatState::default());
    let after = game.work_counters();

    assert_eq!(options.len(), 32);
    assert_eq!(
        after.derived_view_rebuilds - before.derived_view_rebuilds,
        1,
        "legal-attacker generation should share one derived view across all candidates"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_compute_legal_attackers_respects_cant_attack_restriction_tracker() {
    use crate::cards::definitions::grizzly_bears;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let bears_def = grizzly_bears();
    let creature_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(creature_id);
    game.effect_store
        .cant_effects
        .cant_attack
        .insert(creature_id);

    let options = compute_legal_attackers(&game, &CombatState::default());
    assert!(
        options.is_empty(),
        "cant-attack tracker should prevent declaring attackers, got {options:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_compute_legal_attackers_respects_cant_attack_alone_with_single_attacker() {
    use crate::cards::definitions::grizzly_bears;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let bears_def = grizzly_bears();
    let creature_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(creature_id);
    game.effect_store
        .cant_effects
        .cant_attack_alone
        .insert(creature_id);

    let options = compute_legal_attackers(&game, &CombatState::default());
    assert!(
        options.is_empty(),
        "single creature with can't-attack-alone should not be legal attacker, got {options:?}"
    );
}

#[test]
fn test_compute_legal_attackers_respects_cast_creature_spell_attack_restriction() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let cohort_card = CardBuilder::new(CardId::from_raw(901), "Goblin Cohort Variant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::new(
            crate::card::PtValue::Fixed(2),
            crate::card::PtValue::Fixed(2),
        ))
        .build();
    let cohort_id = game.create_object_from_card(&cohort_card, alice, Zone::Battlefield);
    game.object_mut(cohort_id)
        .expect("cohort exists")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn(),
        ));
    game.remove_summoning_sickness(cohort_id);

    game.refresh_continuous_state();
    let options = compute_legal_attackers(&game, &CombatState::default());
    assert!(
        options.iter().all(|option| option.creature != cohort_id),
        "cohort should not be legal attacker before controller casts a creature spell this turn"
    );

    let prior_creature = CardBuilder::new(CardId::from_raw(902), "Prior Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let prior_id = game.create_object_from_card(&prior_creature, alice, Zone::Graveyard);
    let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(prior_id).expect("prior creature exists"),
        &game,
    );
    stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);

    game.refresh_continuous_state();
    let options = compute_legal_attackers(&game, &CombatState::default());
    assert!(
        options.iter().any(|option| option.creature == cohort_id),
        "cohort should become a legal attacker after controller casts a creature spell this turn"
    );
}

#[test]
fn test_compute_legal_attackers_respects_graveyard_threshold_attack_restriction() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let threshold_card = CardBuilder::new(CardId::from_raw(903), "Threshold Raider Variant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::new(
            crate::card::PtValue::Fixed(3),
            crate::card::PtValue::Fixed(2),
        ))
        .build();
    let attacker_id = game.create_object_from_card(&threshold_card, alice, Zone::Battlefield);
    game.object_mut(attacker_id)
            .expect("threshold attacker exists")
                .abilities_mut().push(Ability::static_ability(
                    StaticAbility::cant_attack_unless_condition(
                    crate::static_abilities::CantAttackUnlessConditionSpec::ControllerGraveyardHasCardsAtLeast(5),
                    "Can't attack unless there are five or more cards in your graveyard",
                ),
            ));
    game.remove_summoning_sickness(attacker_id);

    game.refresh_continuous_state();
    let options = compute_legal_attackers(&game, &CombatState::default());
    assert!(
        options.iter().all(|option| option.creature != attacker_id),
        "attacker should not be legal before threshold is met"
    );

    for idx in 0..5 {
        let filler = CardBuilder::new(CardId::from_raw(1000 + idx), &format!("Filler {}", idx + 1))
            .card_types(vec![CardType::Creature])
            .build();
        let _ = game.create_object_from_card(&filler, alice, Zone::Graveyard);
    }

    game.refresh_continuous_state();
    let options = compute_legal_attackers(&game, &CombatState::default());
    assert!(
        options.iter().any(|option| option.creature == attacker_id),
        "attacker should become legal after graveyard threshold is met"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_compute_legal_blockers_respects_cant_block_alone_with_single_blocker() {
    use crate::cards::definitions::grizzly_bears;
    use crate::combat_state::AttackerInfo;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let blocker_def = grizzly_bears();
    let blocker_id = game.create_object_from_definition(&blocker_def, alice, Zone::Battlefield);
    game.effect_store
        .cant_effects
        .cant_block_alone
        .insert(blocker_id);

    let attacker_def = grizzly_bears();
    let attacker_id = game.create_object_from_definition(&attacker_def, bob, Zone::Battlefield);
    game.remove_summoning_sickness(attacker_id);

    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(alice),
    });

    let options = compute_legal_blockers(&game, &combat, alice);
    assert_eq!(options.len(), 1, "expected one attacker option");
    assert!(
        options[0].valid_blockers.is_empty(),
        "single creature with can't-block-alone should not be a legal blocker, got {options:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_compute_legal_blockers_excludes_tapped_creatures() {
    use crate::cards::definitions::grizzly_bears;
    use crate::combat_state::AttackerInfo;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let blocker_def = grizzly_bears();
    let blocker_id = game.create_object_from_definition(&blocker_def, alice, Zone::Battlefield);
    game.tap(blocker_id);

    let attacker_def = grizzly_bears();
    let attacker_id = game.create_object_from_definition(&attacker_def, bob, Zone::Battlefield);
    game.remove_summoning_sickness(attacker_id);

    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(alice),
    });

    let options = compute_legal_blockers(&game, &combat, alice);
    assert_eq!(options.len(), 1, "expected one attacker option");
    assert!(
        options[0].valid_blockers.is_empty(),
        "tapped creature should not be a legal blocker, got {options:?}"
    );
}

#[test]
fn global_colored_spell_cost_increase_adds_pips_to_effective_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Battlefield permanent that taxes black spells you cast by {B}.
    let tax_card = CardBuilder::new(CardId::from_raw(10), "Derelor Variant")
        .card_types(vec![CardType::Creature])
        .build();
    let tax_id = game.create_object_from_card(&tax_card, alice, Zone::Battlefield);
    let mut filter = ObjectFilter::default();
    filter.colors = Some(ColorSet::BLACK);
    filter.cast_by = Some(PlayerFilter::You);
    let tax = StaticAbility::new(crate::static_abilities::CostIncreaseManaCost::new(
        filter,
        ManaCost::from_symbols(vec![ManaSymbol::Black]),
    ));
    game.object_mut(tax_id)
        .expect("tax permanent exists")
        .abilities_mut()
        .push(Ability::static_ability(tax));

    // A black spell with base cost {1}{B}.
    let black_spell_card = CardBuilder::new(CardId::from_raw(11), "Black Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&black_spell_card, alice, Zone::Hand);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{1}{B}{B}");
}

#[test]
fn global_spell_cost_increase_matches_spell_filter_power() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let tax_card = CardBuilder::new(CardId::from_raw(12), "Power Tax")
        .card_types(vec![CardType::Creature])
        .build();
    let tax_id = game.create_object_from_card(&tax_card, alice, Zone::Battlefield);
    let mut filter = ObjectFilter::default();
    filter.power = Some(Comparison::GreaterThanOrEqual(4));
    let tax = StaticAbility::new(crate::static_abilities::CostIncrease::new(
        filter,
        Value::Fixed(1),
    ));
    game.object_mut(tax_id)
        .expect("tax permanent exists")
        .abilities_mut()
        .push(Ability::static_ability(tax));

    let creature_spell = CardBuilder::new(CardId::from_raw(13), "Large Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&creature_spell, alice, Zone::Hand);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{3}{G}{1}");
}

#[test]
fn global_spell_cost_increase_uses_caster_for_spell_filter_controller() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let tax_card = CardBuilder::new(CardId::from_raw(14), "Caster Tax")
        .card_types(vec![CardType::Creature])
        .build();
    let tax_id = game.create_object_from_card(&tax_card, alice, Zone::Battlefield);
    let mut filter = ObjectFilter::default();
    filter.cast_by = Some(PlayerFilter::You);
    let tax = StaticAbility::new(crate::static_abilities::CostIncrease::new(
        filter,
        Value::Fixed(1),
    ));
    game.object_mut(tax_id)
        .expect("tax permanent exists")
        .abilities_mut()
        .push(Ability::static_ability(tax));

    let spell_card = CardBuilder::new(CardId::from_raw(15), "Borrowed Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    // Bob owns/controls the card object, but we evaluate castability for Alice.
    let spell_id = game.create_object_from_card(&spell_card, bob, Zone::Exile);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

    let effective_for_alice = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective_for_alice.to_oracle(), "{2}{U}{1}");

    let effective_for_bob = calculate_effective_mana_cost(&game, bob, spell_obj, base_cost);
    assert_eq!(effective_for_bob.to_oracle(), "{2}{U}");
}

#[test]
fn granted_target_tax_uses_each_affected_flying_creature_as_its_source() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let grant_source = CardBuilder::new(CardId::from_raw(160), "Flying Tax Grant")
        .card_types(vec![CardType::Enchantment])
        .build();
    let grant_source_id = game.create_object_from_card(&grant_source, alice, Zone::Battlefield);

    let mut taxed_spell_filter = ObjectFilter::default();
    taxed_spell_filter.cast_by = Some(PlayerFilter::Opponent);
    taxed_spell_filter.targets_object = Some(Box::new(ObjectFilter::source()));
    let granted_tax = StaticAbility::new(crate::static_abilities::CostIncrease::new(
        taxed_spell_filter,
        Value::Fixed(2),
    ));
    let affected = ObjectFilter::creature()
        .you_control()
        .with_static_ability(crate::static_abilities::StaticAbilityId::Flying);
    game.object_mut(grant_source_id)
        .expect("grant source exists")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::new(
            crate::static_abilities::GrantAbility::new(affected, granted_tax),
        )));

    let creature_card = CardBuilder::new(CardId::from_raw(161), "Tax Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let alice_flying = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    game.object_mut(alice_flying)
        .expect("Alice's flying creature exists")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::flying()));
    let alice_grounded = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    let bob_flying = game.create_object_from_card(&creature_card, bob, Zone::Battlefield);
    game.object_mut(bob_flying)
        .expect("Bob's flying creature exists")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::flying()));

    let base_cost = ManaCost::from_symbols(vec![ManaSymbol::Blue]);
    let spell_card = CardBuilder::new(CardId::from_raw(162), "Targeted Spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(base_cost.clone())
        .build();
    let spell_id = game.create_object_from_card(&spell_card, bob, Zone::Hand);
    let spell = game.object(spell_id).expect("targeted spell exists");

    let cost_targeting = |caster, target| {
        calculate_effective_mana_cost_with_chosen_targets(
            &game,
            caster,
            spell,
            &base_cost,
            &[Target::Object(target)],
        )
        .to_oracle()
    };

    assert_eq!(cost_targeting(bob, alice_flying), "{U}{2}");
    assert_eq!(
        cost_targeting(bob, alice_grounded),
        "{U}",
        "the grant filter must exclude creatures without flying"
    );
    assert_eq!(
        cost_targeting(bob, bob_flying),
        "{U}",
        "the grant filter must exclude flying creatures the grant source's controller doesn't control"
    );
    assert_eq!(
        cost_targeting(alice, alice_flying),
        "{U}",
        "the quoted tax applies only to spells cast by an opponent of the affected creature's controller"
    );
}

#[test]
fn spell_attached_global_cost_reduction_requires_functional_zone() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(16), "Zone Scoped Reducer")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .build();
    let mut filter = ObjectFilter::default();
    filter.cast_by = Some(PlayerFilter::You);

    // A battlefield-only static modifier on a spell card in hand should not apply.
    let battlefield_only_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
        filter.clone(),
        Value::Fixed(1),
    ));
    game.object_mut(battlefield_only_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(reduction));
    let battlefield_only_obj = game.object(battlefield_only_id).expect("spell exists");
    let battlefield_only_base = battlefield_only_obj
        .mana_cost
        .as_ref()
        .expect("spell has mana cost");
    let battlefield_only_effective =
        calculate_effective_mana_cost(&game, alice, battlefield_only_obj, battlefield_only_base);
    assert_eq!(
        battlefield_only_effective.to_oracle(),
        "{2}",
        "battlefield-only modifiers must not apply while the spell is in hand"
    );

    // A hand/stack-scoped modifier still applies (e.g. Undaunted-style implementations).
    let hand_scoped_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let hand_scoped_reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
        filter,
        Value::Fixed(1),
    ));
    game.object_mut(hand_scoped_id)
        .expect("spell exists")
        .abilities_mut()
        .push(
            Ability::static_ability(hand_scoped_reduction).in_zones(vec![Zone::Hand, Zone::Stack]),
        );
    let hand_scoped_obj = game.object(hand_scoped_id).expect("spell exists");
    let hand_scoped_base = hand_scoped_obj
        .mana_cost
        .as_ref()
        .expect("spell has mana cost");
    let hand_scoped_effective =
        calculate_effective_mana_cost(&game, alice, hand_scoped_obj, hand_scoped_base);
    assert_eq!(
        hand_scoped_effective.to_oracle(),
        "{1}",
        "zone-scoped spell modifiers should still apply"
    );
}

#[test]
fn spell_attached_global_cost_reduction_respects_color_filter() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let mut red_filter = ObjectFilter::default();
    red_filter.cast_by = Some(PlayerFilter::You);
    red_filter.colors = Some(ColorSet::RED);

    let colorless_spell = CardBuilder::new(CardId::from_raw(17), "Colorless Probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .build();
    let colorless_id = game.create_object_from_card(&colorless_spell, alice, Zone::Hand);
    let reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
        red_filter.clone(),
        Value::Fixed(1),
    ));
    game.object_mut(colorless_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(reduction).in_zones(vec![Zone::Hand, Zone::Stack]));
    let colorless_obj = game.object(colorless_id).expect("spell exists");
    let colorless_base = colorless_obj
        .mana_cost
        .as_ref()
        .expect("spell has mana cost");
    let colorless_effective =
        calculate_effective_mana_cost(&game, alice, colorless_obj, colorless_base);
    assert_eq!(
        colorless_effective.to_oracle(),
        "{2}",
        "red-only filter must not reduce non-red spell costs"
    );

    let red_spell = CardBuilder::new(CardId::from_raw(18), "Red Probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .build();
    let red_id = game.create_object_from_card(&red_spell, alice, Zone::Hand);
    let red_reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
        red_filter,
        Value::Fixed(1),
    ));
    game.object_mut(red_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(red_reduction).in_zones(vec![Zone::Hand, Zone::Stack]));
    let red_obj = game.object(red_id).expect("spell exists");
    let red_base = red_obj.mana_cost.as_ref().expect("spell has mana cost");
    let red_effective = calculate_effective_mana_cost(&game, alice, red_obj, red_base);
    assert_eq!(
        red_effective.to_oracle(),
        "{R}",
        "red-only filter should reduce matching red spell costs"
    );
}

#[test]
fn battlefield_cost_reduction_applies_only_to_the_chosen_creature_type() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let reducer = CardBuilder::new(CardId::from_raw(71_100), "Chosen Type Reducer")
        .card_types(vec![CardType::Artifact])
        .build();
    let reducer_id = game.create_object_from_card(&reducer, alice, Zone::Battlefield);
    let mut filter = ObjectFilter::default();
    filter.cast_by = Some(PlayerFilter::You);
    filter.chosen_creature_type = true;
    game.object_mut(reducer_id)
        .expect("reducer exists")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, Value::Fixed(1)),
        )));
    game.set_chosen_creature_type(reducer_id, Subtype::Giant);

    let matching = CardBuilder::new(CardId::from_raw(71_101), "Giant Cost Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Giant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]))
        .build();
    let matching_id = game.create_object_from_card(&matching, alice, Zone::Hand);
    let nonmatching = CardBuilder::new(CardId::from_raw(71_102), "Elf Cost Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]))
        .build();
    let nonmatching_id = game.create_object_from_card(&nonmatching, alice, Zone::Hand);

    for (spell_id, expected) in [(matching_id, "{2}"), (nonmatching_id, "{3}")] {
        let spell = game.object(spell_id).expect("cost probe exists");
        let effective = calculate_effective_mana_cost(
            &game,
            alice,
            spell,
            spell.mana_cost.as_ref().expect("cost probe has a cost"),
        );
        assert_eq!(effective.to_oracle(), expected);
    }

    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let actions = compute_legal_actions(&game, alice);
    assert!(actions.iter().any(|action| matches!(
        action,
        LegalAction::CastSpell { spell_id, .. } if *spell_id == matching_id
    )));
    assert!(!actions.iter().any(|action| matches!(
        action,
        LegalAction::CastSpell { spell_id, .. } if *spell_id == nonmatching_id
    )));
}

#[test]
fn generic_chosen_type_cost_filter_falls_back_to_the_sources_chosen_card_type() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let reducer = CardBuilder::new(CardId::from_raw(71_103), "Chosen Card Type Reducer")
        .card_types(vec![CardType::Artifact])
        .build();
    let reducer_id = game.create_object_from_card(&reducer, alice, Zone::Battlefield);
    let mut filter = ObjectFilter::default();
    filter.cast_by = Some(PlayerFilter::You);
    filter.chosen_creature_type = true;
    game.object_mut(reducer_id)
        .expect("reducer exists")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, Value::Fixed(1)),
        )));
    game.set_chosen_card_type(reducer_id, CardType::Instant);

    let matching = CardBuilder::new(CardId::from_raw(71_104), "Instant Cost Probe")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]))
        .build();
    let matching_id = game.create_object_from_card(&matching, alice, Zone::Hand);
    let nonmatching = CardBuilder::new(CardId::from_raw(71_105), "Sorcery Cost Probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]))
        .build();
    let nonmatching_id = game.create_object_from_card(&nonmatching, alice, Zone::Hand);

    for (spell_id, expected) in [(matching_id, "{2}"), (nonmatching_id, "{3}")] {
        let spell = game.object(spell_id).expect("cost probe exists");
        let effective = calculate_effective_mana_cost(
            &game,
            alice,
            spell,
            spell.mana_cost.as_ref().expect("cost probe has a cost"),
        );
        assert_eq!(effective.to_oracle(), expected);
    }

    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let actions = compute_legal_actions(&game, alice);
    assert!(actions.iter().any(|action| matches!(
        action,
        LegalAction::CastSpell { spell_id, .. } if *spell_id == matching_id
    )));
    assert!(!actions.iter().any(|action| matches!(
        action,
        LegalAction::CastSpell { spell_id, .. } if *spell_id == nonmatching_id
    )));
}

#[test]
fn chosen_type_colored_reduction_removes_only_matching_colored_pips() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let reducer = CardBuilder::new(CardId::from_raw(71_106), "Colored Chosen Type Reducer")
        .card_types(vec![CardType::Creature])
        .build();
    let reducer_id = game.create_object_from_card(&reducer, alice, Zone::Battlefield);
    let mut filter = ObjectFilter::default();
    filter.cast_by = Some(PlayerFilter::You);
    filter.chosen_creature_type = true;
    let colored_reduction = ManaCost::from_symbols(vec![
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ]);
    game.object_mut(reducer_id)
        .expect("reducer exists")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::new(
            crate::static_abilities::CostReductionManaCost::new(filter, colored_reduction),
        )));
    game.set_chosen_creature_type(reducer_id, Subtype::Giant);

    let five_color_cost = ManaCost::from_symbols(vec![
        ManaSymbol::Generic(2),
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ]);
    let matching = CardBuilder::new(CardId::from_raw(71_107), "Five Color Giant Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Giant])
        .mana_cost(five_color_cost.clone())
        .build();
    let matching_id = game.create_object_from_card(&matching, alice, Zone::Hand);
    let nonmatching = CardBuilder::new(CardId::from_raw(71_108), "Five Color Elf Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .mana_cost(five_color_cost)
        .build();
    let nonmatching_id = game.create_object_from_card(&nonmatching, alice, Zone::Hand);

    for (spell_id, expected) in [(matching_id, "{2}"), (nonmatching_id, "{2}{W}{U}{B}{R}{G}")] {
        let spell = game.object(spell_id).expect("cost probe exists");
        let effective = calculate_effective_mana_cost(
            &game,
            alice,
            spell,
            spell.mana_cost.as_ref().expect("cost probe has a cost"),
        );
        assert_eq!(effective.to_oracle(), expected);
    }
}

#[test]
fn prototype_cost_reduction_filters_see_prototyped_color() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let mut red_filter = ObjectFilter::default();
    red_filter.cast_by = Some(PlayerFilter::You);
    red_filter.colors = Some(ColorSet::RED);

    let reducer_card = CardBuilder::new(CardId::from_raw(19), "Red Cost Reducer")
        .card_types(vec![CardType::Artifact])
        .build();
    let reducer_id = game.create_object_from_card(&reducer_card, alice, Zone::Battlefield);
    let reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
        red_filter,
        Value::Fixed(1),
    ));
    game.object_mut(reducer_id)
        .expect("reducer exists")
        .abilities_mut()
        .push(Ability::static_ability(reduction));

    let prototype_cost =
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Red]]);
    let prototype_def = CardDefinitionBuilder::new(CardId::from_raw(20), "Prototype Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(6, 4))
        .parse_text("Prototype {2}{R} — 3/2\nHaste")
        .expect("prototype probe should parse");
    let spell_id = game.create_object_from_definition(&prototype_def, alice, Zone::Hand);
    let spell = game.object(spell_id).expect("prototype spell exists");
    assert_eq!(
        spell
            .alternative_casts
            .first()
            .and_then(|method| method.prototype_power_toughness()),
        Some(crate::PowerToughness::fixed(3, 2)),
        "typed Prototype P/T should survive compiler-to-runtime conversion"
    );
    let view = DerivedGameView::new(&game);

    let normal_effective = calculate_effective_mana_cost_with_view_for_casting_method(
        &game,
        alice,
        spell,
        spell
            .mana_cost
            .as_ref()
            .expect("spell has printed mana cost"),
        &CastingMethod::Normal,
        &view,
    );
    assert_eq!(
        normal_effective.to_oracle(),
        "{7}",
        "normally cast artifact creature should remain colorless for red reductions"
    );

    let prototype_effective = calculate_effective_mana_cost_with_view_for_casting_method(
        &game,
        alice,
        spell,
        &prototype_cost,
        &CastingMethod::Alternative(0),
        &view,
    );
    assert_eq!(
        prototype_effective.to_oracle(),
        "{1}{R}",
        "prototyped spell should be red for red spell cost reductions"
    );
}

#[test]
fn dynamic_spell_cost_reduction_distinct_names_reduces_generic_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Two differently named lands you control.
    let forest = CardBuilder::new(CardId::from_raw(20), "Forest Variant")
        .card_types(vec![CardType::Land])
        .build();
    game.create_object_from_card(&forest, alice, Zone::Battlefield);
    let island = CardBuilder::new(CardId::from_raw(21), "Island Variant")
        .card_types(vec![CardType::Land])
        .build();
    game.create_object_from_card(&island, alice, Zone::Battlefield);

    // A spell with base cost {6}{G} that costs {X} less where X is distinct land names.
    let spell_card = CardBuilder::new(CardId::from_raw(22), "Fungal Colossus Variant")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::Green],
        ]))
        .build();
    let mut filter = ObjectFilter::land().you_control();
    filter.zone = Some(Zone::Battlefield);
    let reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
        ObjectFilter::default(),
        Value::DistinctNames(filter),
    ));

    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(reduction).in_zones(vec![Zone::Hand, Zone::Stack]));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{4}{G}");
}

#[test]
fn conditional_this_spell_cost_reduction_only_applies_when_active() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(30), "Avatar Cost Variant")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(6),
        crate::static_abilities::ThisSpellCostCondition::YouLifeTotalOrLess(3),
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    // Condition not met.
    game.player_mut(alice).expect("alice exists").life = 4;
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{6}{B}{B}");

    // Condition met.
    game.player_mut(alice).expect("alice exists").life = 3;
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{B}{B}");
}

#[test]
fn this_spell_cost_reduction_counts_distinct_creature_types_with_cap() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    for (idx, subtypes) in [
        vec![Subtype::Elf, Subtype::Druid],
        vec![Subtype::Goblin, Subtype::Warrior],
        vec![Subtype::Human, Subtype::Soldier],
    ]
    .into_iter()
    .enumerate()
    {
        let creature = CardBuilder::new(
            CardId::from_raw(100 + idx as u32),
            format!("Type Bearer {idx}"),
        )
        .card_types(vec![CardType::Creature])
        .subtypes(subtypes)
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
        game.create_object_from_card(&creature, alice, Zone::Battlefield);
    }

    let spell_card = CardBuilder::new(CardId::from_raw(140), "Capped Type Discount")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(7)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let amount = Value::Min(
        Box::new(Value::CreatureTypesAmong(
            ObjectFilter::creature().you_control(),
        )),
        Box::new(Value::Fixed(5)),
    );
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        amount,
        crate::static_abilities::ThisSpellCostCondition::Always,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}{W}{W}");
}

#[test]
fn conditional_this_spell_mana_cost_reduction_checks_opponent_drawn_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell_card = CardBuilder::new(CardId::from_raw(31), "Even the Score Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let reduction = ManaCost::from_pips(vec![
        vec![ManaSymbol::Blue],
        vec![ManaSymbol::Blue],
        vec![ManaSymbol::Blue],
    ]);
    let ability = StaticAbility::new(
        crate::static_abilities::ThisSpellCostReductionManaCost::new(
            reduction,
            crate::static_abilities::ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(4),
        ),
    );
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    // Condition not met.
    stage_cards_drawn_for_test(&mut game, bob, 3);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{3}{U}{U}{U}");

    // Condition met.
    game.turn_store.turn_history.clear_for_new_turn();
    stage_cards_drawn_for_test(&mut game, bob, 4);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{3}");
}

#[test]
fn conditional_this_spell_mana_cost_reduction_checks_opponent_cast_spells() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell_card = CardBuilder::new(CardId::from_raw(32), "Ertai's Scorn Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let reduction = ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]);
    let ability = StaticAbility::new(
        crate::static_abilities::ThisSpellCostReductionManaCost::new(
            reduction,
            crate::static_abilities::ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(2),
        ),
    );
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    // Condition not met.
    stage_spell_cast_for_test(&mut game, ObjectId::from_raw(3201), bob, Zone::Hand);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}{U}");

    // Condition met.
    game.turn_store.turn_history.clear_for_new_turn();
    stage_spell_cast_for_test(&mut game, ObjectId::from_raw(3201), bob, Zone::Hand);
    stage_spell_cast_for_test(&mut game, ObjectId::from_raw(3202), bob, Zone::Hand);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}");
}

#[test]
fn conditional_this_spell_mana_cost_reduction_with_generic_and_colored_pips() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell_card = CardBuilder::new(CardId::from_raw(33), "Discontinuity Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let reduction = ManaCost::from_pips(vec![
        vec![ManaSymbol::Generic(2)],
        vec![ManaSymbol::Blue],
        vec![ManaSymbol::Blue],
    ]);
    let ability = StaticAbility::new(
        crate::static_abilities::ThisSpellCostReductionManaCost::new(
            reduction,
            crate::static_abilities::ThisSpellCostCondition::YourTurn,
        ),
    );
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    // Condition met (it's your turn).
    game.turn.active_player = alice;
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{4}");

    // Condition not met.
    game.turn.active_player = bob;
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{6}{U}{U}");
}

#[test]
fn this_spell_cost_reduction_with_target_condition_uses_chosen_targets() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(133), "Target Discount Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let condition = crate::static_abilities::ThisSpellCostCondition::TargetsObject(
        ObjectFilter::creature().tapped(),
    );
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(2),
        condition,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let creature_card = CardBuilder::new(CardId::from_raw(134), "Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

    // Untapped target does not satisfy condition.
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost_for_payment_with_chosen_targets(
        &game,
        alice,
        spell_obj,
        base_cost,
        &[Target::Object(creature_id)],
    );
    assert_eq!(effective.to_oracle(), "{3}{R}");

    // Tapped target satisfies condition.
    game.tap(creature_id);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost_for_payment_with_chosen_targets(
        &game,
        alice,
        spell_obj,
        base_cost,
        &[Target::Object(creature_id)],
    );
    assert_eq!(effective.to_oracle(), "{1}{R}");
}

#[test]
fn this_spell_cost_reduction_cast_another_instant_or_sorcery_condition() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(135), "Spell History Discount Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let condition = crate::static_abilities::ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
        count: 1,
        card_types: vec![CardType::Instant, CardType::Sorcery],
    };
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(2),
        condition,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    // No prior instant/sorcery this turn.
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{4}{U}");

    // One instant cast this turn enables reduction.
    let prior_card = CardBuilder::new(CardId::from_raw(136), "Prior Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let prior_id = game.create_object_from_card(&prior_card, alice, Zone::Graveyard);
    let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(prior_id).expect("prior instant exists"),
        &game,
    );
    stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}{U}");
}

#[test]
fn this_spell_cost_reduction_graveyard_card_count_condition() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(137), "Graveyard Cards Discount Variant")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(8)],
            vec![ManaSymbol::Black],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let condition =
        crate::static_abilities::ThisSpellCostCondition::YouHaveCardsInYourGraveyardOrMore(9);
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(3),
        condition,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    // Not enough cards.
    for idx in 0..8 {
        let filler = CardBuilder::new(CardId::from_raw(200 + idx), format!("GY Card {idx}"))
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&filler, alice, Zone::Graveyard);
    }
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{8}{B}");

    // Ninth card enables reduction.
    let extra = CardBuilder::new(CardId::from_raw(300), "GY Extra")
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_card(&extra, alice, Zone::Graveyard);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{5}{B}");
}

#[test]
fn this_spell_cost_reduction_creature_attacking_you_condition() {
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell_card = CardBuilder::new(CardId::from_raw(138), "Attack Trap Discount Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let condition = crate::static_abilities::ThisSpellCostCondition::CreatureIsAttackingYou;
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(2),
        condition,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    // No attackers: no reduction.
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{4}{B}");

    // One attacker attacking Alice enables reduction.
    let attacker_card = CardBuilder::new(CardId::from_raw(139), "Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let attacker_id = game.create_object_from_card(&attacker_card, bob, Zone::Battlefield);
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(alice),
    });
    game.combat = Some(combat);

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}{B}");
}

#[test]
fn this_spell_cost_reduction_is_night_condition() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(140), "Night Discount Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Red],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let condition = crate::static_abilities::ThisSpellCostCondition::IsNight;
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(2),
        condition,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{4}{R}");

    game.is_night = true;
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}{R}");
}

#[test]
fn this_spell_cost_reduction_sacrificed_artifact_condition() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(141), "Artifact Sac Discount Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Red],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let condition = crate::static_abilities::ThisSpellCostCondition::YouSacrificedArtifactThisTurn;
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(3),
        condition,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{5}{R}");

    stage_artifact_sacrifice_for_test(&mut game, alice);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}{R}");
}

#[test]
fn this_spell_cost_reduction_creature_left_battlefield_condition() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(142), "Creature Left Discount Variant")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Green],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let condition = crate::static_abilities::ThisSpellCostCondition::
            CreatureLeftBattlefieldUnderYourControlThisTurn;
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(2),
        condition,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{4}{G}");

    let departed_creature = CardBuilder::new(CardId::from_raw(5000), "Fallen Helper")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let departed_id = game.create_object_from_card(&departed_creature, alice, Zone::Battlefield);
    game.move_object_by_effect(departed_id, Zone::Graveyard);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}{G}");
}

#[test]
fn this_spell_cost_reduction_committed_crime_condition() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(143), "Crime Discount Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let condition = crate::static_abilities::ThisSpellCostCondition::YouCommittedCrimeThisTurn;
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(1),
        condition,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{3}{U}");

    stage_commit_crime_for_test(&mut game, alice);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}{U}");
}

#[test]
fn this_spell_cost_reduction_only_named_creatures_in_hand_condition() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(144), "Mothrider Cavalry")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::White],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let condition = crate::static_abilities::ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(
        "mothrider cavalry".to_string(),
    );
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(2),
        condition,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    // Only this card in hand (named Mothrider Cavalry): reduction applies.
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{4}{W}");

    // Another creature with a different name disables the reduction.
    let other_creature = CardBuilder::new(CardId::from_raw(145), "Not Mothrider")
        .card_types(vec![CardType::Creature])
        .build();
    game.create_object_from_card(&other_creature, alice, Zone::Hand);
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{6}{W}");
}

#[test]
fn this_spell_cost_reduction_x_uses_life_difference_from_starting() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(146), "Starting Life X Discount Variant")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(13)],
            vec![ManaSymbol::Black],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::X,
        crate::static_abilities::ThisSpellCostCondition::LifeTotalLessThanStarting,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    // At starting life, no reduction.
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{13}{B}");

    // Reduced by life lost from starting life total.
    game.player_mut(alice).expect("player exists").life = 12;
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{5}{B}");
}

#[test]
fn shadow_of_mortality_cost_reduction_applies_only_below_starting_life() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_card = CardBuilder::new(CardId::from_raw(9_146), "Shadow of Mortality")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(13)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::X,
        crate::static_abilities::ThisSpellCostCondition::LifeTotalLessThanStarting,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let at_starting_life = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(
        at_starting_life.to_oracle(),
        "{13}{B}{B}",
        "cost reduction must not apply at starting life total"
    );

    game.player_mut(alice).expect("player exists").life = 12;
    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let reduced = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(
        reduced.to_oracle(),
        "{5}{B}{B}",
        "cost reduction must equal life lost from starting life total"
    );
}

#[test]
fn knowledge_exploitation_prowl_alternative_cost_requires_rogue_combat_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = CardBuilder::new(CardId::from_raw(9501), "Knowledge Exploitation")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(5),
            ManaSymbol::Blue,
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let prowl_condition =
            crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeThisTurn(
                Subtype::Rogue,
            );
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(2),
        prowl_condition,
    ));
    game.object_mut(spell_id)
        .expect("knowledge exploitation should exist")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game
        .object(spell_id)
        .expect("knowledge exploitation should exist");
    let base_cost = spell_obj
        .mana_cost
        .as_ref()
        .expect("knowledge exploitation should have mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(
        effective.to_oracle(),
        "{5}{U}",
        "prowl condition should be false before Rogue combat damage"
    );

    let rogue = CardBuilder::new(CardId::from_raw(9502), "Rogue Test Creature")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Rogue])
        .power_toughness(PowerToughness::fixed(2, 1))
        .build();
    let rogue_id = game.create_object_from_card(&rogue, alice, Zone::Battlefield);
    stage_combat_damage_to_player_for_test(&mut game, rogue_id, bob, 2);

    let spell_obj = game
        .object(spell_id)
        .expect("knowledge exploitation should exist");
    let base_cost = spell_obj
        .mana_cost
        .as_ref()
        .expect("knowledge exploitation should have mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(
        effective.to_oracle(),
        "{3}{U}",
        "prowl condition should become true after your Rogue deals combat damage to a player"
    );
}

#[test]
fn knowledge_exploitation_prowl_alternative_cost_rejects_noncombat_or_nonrogue_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = CardBuilder::new(CardId::from_raw(9503), "Knowledge Exploitation")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(5),
            ManaSymbol::Blue,
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let prowl_condition =
            crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeThisTurn(
                Subtype::Rogue,
            );
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(2),
        prowl_condition,
    ));
    game.object_mut(spell_id)
        .expect("knowledge exploitation should exist")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let wizard = CardBuilder::new(CardId::from_raw(9504), "Wizard Test Creature")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let wizard_id = game.create_object_from_card(&wizard, alice, Zone::Battlefield);
    stage_combat_damage_to_player_for_test(&mut game, wizard_id, bob, 2);

    let rogue = CardBuilder::new(CardId::from_raw(9505), "Rogue Test Creature")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Rogue])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let rogue_id = game.create_object_from_card(&rogue, alice, Zone::Battlefield);
    stage_noncombat_damage_to_player_for_test(&mut game, rogue_id, bob, 1);

    let spell_obj = game
        .object(spell_id)
        .expect("knowledge exploitation should exist");
    let base_cost = spell_obj
        .mana_cost
        .as_ref()
        .expect("knowledge exploitation should have mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(
        effective.to_oracle(),
        "{5}{U}",
        "prowl condition should stay false when damage is noncombat or from non-Rogue creatures"
    );
}

#[test]
fn overpowering_attack_freerunning_condition_accepts_assassin_or_commander_combat_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = CardBuilder::new(CardId::from_raw(9510), "Overpowering Attack")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(3),
            ManaSymbol::Red,
            ManaSymbol::Red,
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let freerunning_condition = crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(Subtype::Assassin);
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Fixed(2),
        freerunning_condition,
    ));
    game.object_mut(spell_id)
        .expect("Overpowering Attack should exist")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game
        .object(spell_id)
        .expect("Overpowering Attack should exist");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let initial_cost = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(
        initial_cost.to_oracle(),
        "{3}{R}{R}",
        "freerunning condition should be false before combat damage"
    );

    let wizard = CardBuilder::new(CardId::from_raw(9511), "Wizard Test Creature")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let wizard_id = game.create_object_from_card(&wizard, alice, Zone::Battlefield);
    stage_combat_damage_to_player_for_test(&mut game, wizard_id, bob, 2);

    let spell_obj = game
        .object(spell_id)
        .expect("Overpowering Attack should exist");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let wizard_only_cost = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(
        wizard_only_cost.to_oracle(),
        "{3}{R}{R}",
        "freerunning should remain unavailable after non-Assassin non-commander combat damage"
    );

    let assassin = CardBuilder::new(CardId::from_raw(9512), "Assassin Test Creature")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Assassin])
        .power_toughness(PowerToughness::fixed(2, 1))
        .build();
    let assassin_id = game.create_object_from_card(&assassin, alice, Zone::Battlefield);
    stage_combat_damage_to_player_for_test(&mut game, assassin_id, bob, 2);

    let spell_obj = game
        .object(spell_id)
        .expect("Overpowering Attack should exist");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let assassin_cost = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(
        assassin_cost.to_oracle(),
        "{1}{R}{R}",
        "freerunning should apply after Assassin combat damage"
    );
}

#[test]
fn overpowering_attack_freerunning_condition_accepts_commander_combat_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = CardBuilder::new(CardId::from_raw(9513), "Overpowering Attack")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(3),
            ManaSymbol::Red,
            ManaSymbol::Red,
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(Subtype::Assassin),
        ));
    game.object_mut(spell_id)
        .expect("Overpowering Attack should exist")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let soldier = CardBuilder::new(CardId::from_raw(9514), "Commander Test Creature")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let commander_id = game.create_object_from_card(&soldier, alice, Zone::Battlefield);
    game.set_commander(commander_id);
    game.player_mut(alice)
        .expect("player exists")
        .set_commanders(vec![commander_id]);
    stage_combat_damage_to_player_for_test(&mut game, commander_id, bob, 3);

    let spell_obj = game
        .object(spell_id)
        .expect("Overpowering Attack should exist");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let commander_cost = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(
        commander_cost.to_oracle(),
        "{1}{R}{R}",
        "freerunning should apply after commander combat damage"
    );
}

#[test]
fn this_spell_cost_reduction_supports_devotion_where_x_is_clause() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Devotion to black = 3.
    let perm1 = CardBuilder::new(CardId::from_raw(40), "BB Permanent")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .build();
    game.create_object_from_card(&perm1, alice, Zone::Battlefield);
    let perm2 = CardBuilder::new(CardId::from_raw(41), "1B Permanent")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
        ]))
        .build();
    game.create_object_from_card(&perm2, alice, Zone::Battlefield);

    let spell_card = CardBuilder::new(CardId::from_raw(42), "Devotion Cost Variant")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::Black],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::Devotion {
            player: PlayerFilter::You,
            color: crate::color::Color::Black,
        },
        crate::static_abilities::ThisSpellCostCondition::Always,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{3}{B}");
}

#[test]
fn this_spell_cost_reduction_supports_total_power_where_x_is_clause() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let bear = CardBuilder::new(CardId::from_raw(43), "Cost Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_card(&bear, alice, Zone::Battlefield);
    let giant = CardBuilder::new(CardId::from_raw(44), "Cost Giant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    game.create_object_from_card(&giant, alice, Zone::Battlefield);

    let spell_card = CardBuilder::new(CardId::from_raw(45), "Power Discount Variant")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(10)],
            vec![ManaSymbol::Green],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::TotalPower(ObjectFilter::creature().you_control()),
        crate::static_abilities::ThisSpellCostCondition::Always,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{3}{G}");
}

#[test]
fn this_spell_cost_reduction_supports_life_gained_this_turn_where_x_is_clause() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    stage_life_gain_for_test(&mut game, alice, 5);

    let spell_card = CardBuilder::new(CardId::from_raw(46), "Life Discount Variant")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(7)],
            vec![ManaSymbol::Green],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::LifeGainedThisTurn(PlayerFilter::You),
        crate::static_abilities::ThisSpellCostCondition::Always,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}{G}");
}

#[test]
fn this_spell_cost_reduction_supports_noncombat_damage_to_opponents_where_x_is_clause() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    stage_noncombat_damage_to_player_for_test(&mut game, ObjectId::from_raw(4701), bob, 6);

    let spell_card = CardBuilder::new(CardId::from_raw(47), "Damage Discount Variant")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(8)],
            vec![ManaSymbol::Red],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::NoncombatDamageDealtToPlayersThisTurn(PlayerFilter::Opponent),
        crate::static_abilities::ThisSpellCostCondition::Always,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{2}{R}");
}

#[test]
fn this_spell_cost_reduction_supports_greatest_commander_mana_value_where_x_is_clause() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let commander_battlefield = CardBuilder::new(CardId::from_raw(48), "Battlefield Commander")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
        ]))
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let battlefield_id =
        game.create_object_from_card(&commander_battlefield, alice, Zone::Battlefield);
    game.set_as_commander(battlefield_id, alice);

    let commander_command_zone = CardBuilder::new(CardId::from_raw(49), "Command Zone Commander")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Blue],
        ]))
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let command_id = game.create_object_from_card(&commander_command_zone, alice, Zone::Command);
    game.set_as_commander(command_id, alice);

    let spell_card = CardBuilder::new(CardId::from_raw(50), "Commander Discount Variant")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(9)],
            vec![ManaSymbol::White],
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);

    let mut battlefield_filter = ObjectFilter::default();
    battlefield_filter.zone = Some(Zone::Battlefield);
    battlefield_filter.owner = Some(PlayerFilter::You);
    battlefield_filter.is_commander = true;
    let mut command_filter = battlefield_filter.clone();
    command_filter.zone = Some(Zone::Command);
    let mut commander_filter = ObjectFilter::default();
    commander_filter.any_of = vec![battlefield_filter, command_filter];

    let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
        Value::GreatestManaValue(commander_filter),
        crate::static_abilities::ThisSpellCostCondition::Always,
    ));
    game.object_mut(spell_id)
        .expect("spell exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));

    let spell_obj = game.object(spell_id).expect("spell exists");
    let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
    let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
    assert_eq!(effective.to_oracle(), "{3}{W}");
}

#[test]
fn this_way_commander_reduction_applies_only_to_flashback_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let own_commander = CardBuilder::new(CardId::from_raw(50_100), "Own Commander")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Green],
        ]))
        .power_toughness(PowerToughness::fixed(6, 6))
        .build();
    let own_commander_id = game.create_object_from_card(&own_commander, alice, Zone::Battlefield);
    game.set_as_commander(own_commander_id, alice);

    let own_command_zone_commander =
        CardBuilder::new(CardId::from_raw(50_101), "Own Command Zone Commander")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Blue],
            ]))
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
    let own_command_zone_id =
        game.create_object_from_card(&own_command_zone_commander, alice, Zone::Command);
    game.set_as_commander(own_command_zone_id, alice);

    let opposing_commander = CardBuilder::new(CardId::from_raw(50_102), "Opposing Commander")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(9)],
            vec![ManaSymbol::Red],
        ]))
        .power_toughness(PowerToughness::fixed(10, 10))
        .build();
    let opposing_commander_id =
        game.create_object_from_card(&opposing_commander, bob, Zone::Battlefield);
    game.set_as_commander(opposing_commander_id, bob);

    let normal_cost =
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Black]]);
    let flashback_cost = ManaCost::from_pips(vec![
        vec![ManaSymbol::Generic(8)],
        vec![ManaSymbol::Black],
        vec![ManaSymbol::Black],
    ]);
    let spell_card = CardBuilder::new(CardId::from_raw(50_103), "Visions Cost Probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(normal_cost.clone())
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Graveyard);

    let mut battlefield_filter = ObjectFilter::default();
    battlefield_filter.zone = Some(Zone::Battlefield);
    battlefield_filter.owner = Some(PlayerFilter::You);
    battlefield_filter.is_commander = true;
    let mut command_filter = battlefield_filter.clone();
    command_filter.zone = Some(Zone::Command);
    let mut commander_filter = ObjectFilter::default();
    commander_filter.any_of = vec![battlefield_filter, command_filter];

    let reduction = StaticAbility::new(
        crate::static_abilities::ThisSpellCostReduction::new(
            Value::GreatestManaValue(commander_filter),
            crate::static_abilities::ThisSpellCostCondition::Always,
        )
        .with_alternative_cast(crate::filter::AlternativeCastKind::Flashback),
    );
    let spell = game.object_mut(spell_id).expect("spell exists");
    spell
        .abilities_mut()
        .push(Ability::static_ability(reduction));
    spell.alternative_casts.push(
        crate::alternative_cast::AlternativeCastingMethod::Flashback {
            total_cost: crate::cost::TotalCost::mana(flashback_cost.clone()),
        },
    );

    let spell = game.object(spell_id).expect("spell exists");
    let view = DerivedGameView::new(&game);
    let normal = calculate_effective_mana_cost_with_view_for_casting_method(
        &game,
        alice,
        spell,
        &normal_cost,
        &CastingMethod::Normal,
        &view,
    );
    let flashback = calculate_effective_mana_cost_with_view_for_casting_method(
        &game,
        alice,
        spell,
        &flashback_cost,
        &CastingMethod::Alternative(0),
        &view,
    );

    assert_eq!(normal.to_oracle(), "{2}{B}");
    assert_eq!(flashback.to_oracle(), "{2}{B}{B}");
}
