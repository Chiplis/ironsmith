use super::*;

#[test]
fn test_can_cast_spell_respects_cant_cast_creature_spells_restriction() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;

    let creature = CardBuilder::new(CardId::from_raw(77), "Restriction Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);
    let creature_obj = game
        .object(creature_id)
        .expect("creature in hand must exist")
        .clone();

    game.effect_store.cant_effects.add_cant_cast_filter(
        alice,
        crate::target::ObjectFilter::default().with_type(CardType::Creature),
    );
    assert!(
        !can_cast_spell(&game, alice, &creature_obj, &CastingMethod::Normal),
        "creature spell should be uncastable when player can't cast creature spells"
    );
}

#[test]
fn test_can_cast_spell_respects_cast_limit_one_per_turn_restriction() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let instant = CardBuilder::new(CardId::from_raw(78), "Restriction Spark")
        .card_types(vec![CardType::Instant])
        .build();
    let instant_id = game.create_object_from_card(&instant, alice, Zone::Hand);
    let instant_obj = game
        .object(instant_id)
        .expect("instant in hand must exist")
        .clone();

    game.effect_store
        .cant_effects
        .add_cast_limit_filter(alice, crate::target::ObjectFilter::default());
    stage_spell_cast_for_test(&mut game, ObjectId::from_raw(7801), alice, Zone::Hand);

    assert!(
        !can_cast_spell(&game, alice, &instant_obj, &CastingMethod::Normal),
        "second spell in same turn should be blocked by one-spell limit"
    );
}

#[test]
fn test_can_cast_spell_respects_noncreature_cast_limit() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let instant = CardBuilder::new(CardId::from_raw(79), "Restriction Snuff")
        .card_types(vec![CardType::Instant])
        .build();
    let instant_id = game.create_object_from_card(&instant, alice, Zone::Hand);
    let instant_obj = game
        .object(instant_id)
        .expect("instant in hand must exist")
        .clone();

    let prior_noncreature = CardBuilder::new(CardId::from_raw(80), "Prior Noncreature")
        .card_types(vec![CardType::Sorcery])
        .build();
    let prior_noncreature_id =
        game.create_object_from_card(&prior_noncreature, alice, Zone::Graveyard);
    let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(prior_noncreature_id)
            .expect("prior noncreature must exist"),
        &game,
    );
    stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
    game.effect_store.cant_effects.add_cast_limit_filter(
        alice,
        crate::target::ObjectFilter::default().without_type(CardType::Creature),
    );

    assert!(
        !can_cast_spell(&game, alice, &instant_obj, &CastingMethod::Normal),
        "second noncreature spell in same turn should be blocked by noncreature cast limit"
    );
}

#[test]
fn test_can_cast_spell_noncreature_limit_still_allows_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;

    let creature = CardBuilder::new(CardId::from_raw(81), "Restriction Beast")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::new())
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);
    let creature_obj = game
        .object(creature_id)
        .expect("creature in hand must exist")
        .clone();

    let prior_noncreature = CardBuilder::new(CardId::from_raw(82), "Prior Noncreature")
        .card_types(vec![CardType::Instant])
        .build();
    let prior_noncreature_id =
        game.create_object_from_card(&prior_noncreature, alice, Zone::Graveyard);
    let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(prior_noncreature_id)
            .expect("prior noncreature must exist"),
        &game,
    );
    stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
    game.effect_store.cant_effects.add_cast_limit_filter(
        alice,
        crate::target::ObjectFilter::default().without_type(CardType::Creature),
    );

    assert!(
        can_cast_spell(&game, alice, &creature_obj, &CastingMethod::Normal),
        "noncreature cast limit should still allow creature spell"
    );
}

#[test]
fn test_can_cast_spell_respects_nonartifact_cast_limit() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let nonartifact_spell = CardBuilder::new(CardId::from_raw(83), "Restriction Chant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::new())
        .build();
    let nonartifact_spell_id = game.create_object_from_card(&nonartifact_spell, alice, Zone::Hand);
    let nonartifact_spell_obj = game
        .object(nonartifact_spell_id)
        .expect("nonartifact spell in hand must exist")
        .clone();

    let prior_nonartifact = CardBuilder::new(CardId::from_raw(84), "Prior Nonartifact")
        .card_types(vec![CardType::Sorcery])
        .build();
    let prior_nonartifact_id =
        game.create_object_from_card(&prior_nonartifact, alice, Zone::Graveyard);
    let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(prior_nonartifact_id)
            .expect("prior nonartifact must exist"),
        &game,
    );
    stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
    game.effect_store.cant_effects.add_cast_limit_filter(
        alice,
        crate::target::ObjectFilter::default().without_type(CardType::Artifact),
    );

    assert!(
        !can_cast_spell(&game, alice, &nonartifact_spell_obj, &CastingMethod::Normal),
        "second nonartifact spell in same turn should be blocked by nonartifact cast limit"
    );
}

#[test]
fn test_can_cast_spell_nonartifact_limit_allows_artifact() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;

    let artifact_spell = CardBuilder::new(CardId::from_raw(85), "Restriction Relic")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::new())
        .build();
    let artifact_spell_id = game.create_object_from_card(&artifact_spell, alice, Zone::Hand);
    let artifact_spell_obj = game
        .object(artifact_spell_id)
        .expect("artifact spell in hand must exist")
        .clone();

    let prior_nonartifact = CardBuilder::new(CardId::from_raw(86), "Prior Nonartifact")
        .card_types(vec![CardType::Instant])
        .build();
    let prior_nonartifact_id =
        game.create_object_from_card(&prior_nonartifact, alice, Zone::Graveyard);
    let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(prior_nonartifact_id)
            .expect("prior nonartifact must exist"),
        &game,
    );
    stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
    game.effect_store.cant_effects.add_cast_limit_filter(
        alice,
        crate::target::ObjectFilter::default().without_type(CardType::Artifact),
    );

    assert!(
        can_cast_spell(&game, alice, &artifact_spell_obj, &CastingMethod::Normal),
        "nonartifact cast limit should still allow artifact spell"
    );
}

#[test]
fn test_can_cast_spell_respects_nonphyrexian_cast_limit() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let nonphyrexian_spell = CardBuilder::new(CardId::from_raw(87), "Restriction Spell")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::new())
        .subtypes(vec![Subtype::Elf])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let nonphyrexian_spell_id =
        game.create_object_from_card(&nonphyrexian_spell, alice, Zone::Hand);
    let nonphyrexian_spell_obj = game
        .object(nonphyrexian_spell_id)
        .expect("non-Phyrexian spell in hand must exist")
        .clone();

    let prior_nonphyrexian = CardBuilder::new(CardId::from_raw(88), "Prior Nonphyrexian")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let prior_nonphyrexian_id =
        game.create_object_from_card(&prior_nonphyrexian, alice, Zone::Graveyard);
    let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(prior_nonphyrexian_id)
            .expect("prior non-Phyrexian must exist"),
        &game,
    );
    stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
    game.effect_store.cant_effects.add_cast_limit_filter(
        alice,
        crate::target::ObjectFilter::default().without_subtype(Subtype::Phyrexian),
    );

    assert!(
        !can_cast_spell(
            &game,
            alice,
            &nonphyrexian_spell_obj,
            &CastingMethod::Normal
        ),
        "second non-Phyrexian spell in same turn should be blocked by non-Phyrexian cast limit"
    );
}

#[test]
fn test_can_cast_spell_nonphyrexian_limit_allows_phyrexian() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;

    let phyrexian_spell = CardBuilder::new(CardId::from_raw(89), "Restriction Horror")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::new())
        .subtypes(vec![Subtype::Phyrexian])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let phyrexian_spell_id = game.create_object_from_card(&phyrexian_spell, alice, Zone::Hand);
    let phyrexian_spell_obj = game
        .object(phyrexian_spell_id)
        .expect("Phyrexian spell in hand must exist")
        .clone();

    let prior_nonphyrexian = CardBuilder::new(CardId::from_raw(90), "Prior Nonphyrexian")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let prior_nonphyrexian_id =
        game.create_object_from_card(&prior_nonphyrexian, alice, Zone::Graveyard);
    let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(prior_nonphyrexian_id)
            .expect("prior non-Phyrexian must exist"),
        &game,
    );
    stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
    game.effect_store.cant_effects.add_cast_limit_filter(
        alice,
        crate::target::ObjectFilter::default().without_subtype(Subtype::Phyrexian),
    );

    assert!(
        can_cast_spell(&game, alice, &phyrexian_spell_obj, &CastingMethod::Normal),
        "non-Phyrexian cast limit should still allow a Phyrexian spell"
    );
}

#[test]
fn test_can_cast_spell_uses_conditional_spell_flash_threshold() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.active_player = bob;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::BeginCombat);

    let sorcery = CardBuilder::new(CardId::from_raw(1200), "Threshold Flash Sorcery")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::new())
        .build();
    let spell_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);
    let spec = crate::static_abilities::ConditionalSpellKeywordSpec {
        keyword: crate::static_abilities::ConditionalSpellKeywordKind::Flash,
        metric: crate::static_abilities::GraveyardCountMetric::ManaValues,
        threshold: 5,
    };
    game.object_mut(spell_id)
        .expect("spell should exist")
        .abilities_mut()
        .push(
            Ability::static_ability(StaticAbility::conditional_spell_keyword(spec))
                .in_zones(vec![Zone::Hand, Zone::Stack]),
        );

    for (idx, mv) in [1u8, 2, 3, 4].into_iter().enumerate() {
        let card = CardBuilder::new(
            CardId::from_raw(1300 + idx as u32),
            format!("MV{mv} Graveyard Card"),
        )
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(mv)]]))
        .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }

    let spell_obj = game.object(spell_id).expect("spell should exist").clone();
    assert!(
        !can_cast_spell(&game, alice, &spell_obj, &CastingMethod::Normal),
        "sorcery should remain sorcery-speed before mana-value threshold is met"
    );

    let fifth = CardBuilder::new(CardId::from_raw(1399), "MV5 Graveyard Card")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
        .build();
    game.create_object_from_card(&fifth, alice, Zone::Graveyard);

    let spell_obj = game.object(spell_id).expect("spell should exist").clone();
    assert!(
        can_cast_spell(&game, alice, &spell_obj, &CastingMethod::Normal),
        "conditional flash should allow casting once the mana-value threshold is met"
    );
}

#[test]
fn conditional_flash_model_is_inactive_until_its_typed_condition_is_true() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.active_player = bob;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::BeginCombat);

    let sorcery = CardBuilder::new(CardId::from_raw(1201), "Conditional Flash Sorcery")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::new())
        .build();
    let spell_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);
    let condition = ironsmith_core::Condition::YouControl(
        crate::target::ObjectFilter::default()
            .with_type(CardType::Artifact)
            .you_control(),
    );
    let modeled = crate::static_abilities::CompiledStaticAbility::flash().with_labeled_condition(
        condition,
        "As long as you control an artifact, you may cast this spell as though it had flash",
    );
    game.object_mut(spell_id)
        .expect("spell should exist")
        .abilities_mut()
        .push(
            Ability::static_ability(StaticAbility::from_model(modeled))
                .in_zones(vec![Zone::Hand, Zone::Stack]),
        );

    let spell = game.object(spell_id).expect("spell should exist").clone();
    assert!(
        !can_cast_spell(&game, alice, &spell, &CastingMethod::Normal),
        "the Flash id on a false conditional wrapper must not grant unconditional timing"
    );

    let artifact = CardBuilder::new(CardId::from_raw(1202), "Condition Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&artifact, alice, Zone::Battlefield);

    let spell = game.object(spell_id).expect("spell should exist").clone();
    assert!(
        can_cast_spell(&game, alice, &spell, &CastingMethod::Normal),
        "the same typed condition should grant flash timing once satisfied"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_compute_legal_actions_includes_kentaro_mana_value_cast_for_samurai() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let kentaro = CardDefinitionBuilder::new(CardId::from_raw(1400), "Kentaro Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Bushido 1\nYou may pay {X} rather than pay the mana cost for Samurai spells you cast, where X is that spell's mana value.",
            )
            .expect("Kentaro text should parse");
    let _kentaro_id = game.create_object_from_definition(&kentaro, alice, Zone::Battlefield);

    let samurai = CardBuilder::new(CardId::from_raw(1401), "Samurai Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Samurai])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::White],
        ]))
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let samurai_id = game.create_object_from_card(&samurai, alice, Zone::Hand);

    game.player_mut(alice)
        .expect("alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 5);

    let granted = game
        .effect_store
        .grant_registry
        .granted_alternative_casts_for_card(&game, samurai_id, Zone::Hand, alice);
    assert_eq!(
        granted.len(),
        1,
        "Kentaro should grant one hand alternative cost"
    );
    assert_eq!(granted[0].method.name(), "Pay mana value");
    assert_eq!(
        granted[0]
            .method
            .mana_cost()
            .expect("Kentaro grant should have a mana cost")
            .generic_mana_total(),
        5,
        "Kentaro should turn the spell's mana value into a generic hand-cast cost"
    );

    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == samurai_id
        )),
        "without white mana, the Samurai should not be normally castable"
    );
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::PlayFrom {
                    zone: Zone::Hand,
                    use_alternative: Some(_),
                    ..
                },
            } if *spell_id == samurai_id
        )),
        "Kentaro should surface a hand cast action that uses the mana-value alternative cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_compute_legal_actions_includes_rooftop_storm_free_cast_only_for_zombies() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let rooftop_storm = CardDefinitionBuilder::new(CardId::from_raw(1450), "Rooftop Storm")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.",
        )
        .expect("Rooftop Storm text should parse");
    game.create_object_from_definition(&rooftop_storm, alice, Zone::Battlefield);

    let zombie = CardBuilder::new(CardId::from_raw(1451), "Zombie Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let zombie_id = game.create_object_from_card(&zombie, alice, Zone::Hand);

    let non_zombie = CardBuilder::new(CardId::from_raw(1452), "Human Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let non_zombie_id = game.create_object_from_card(&non_zombie, alice, Zone::Hand);

    let granted = game
        .effect_store
        .grant_registry
        .granted_alternative_casts_for_card(&game, zombie_id, Zone::Hand, alice);
    assert_eq!(
        granted.len(),
        1,
        "Rooftop Storm should grant one hand alternative cost to Zombies"
    );
    assert_eq!(
        granted[0]
            .method
            .mana_cost()
            .expect("Rooftop Storm grant should have a mana cost")
            .generic_mana_total(),
        0,
        "Rooftop Storm should turn Zombie creature spells into zero-mana alternative casts"
    );

    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == zombie_id || *spell_id == non_zombie_id
        )),
        "without mana, neither creature should be normally castable"
    );
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::PlayFrom {
                    zone: Zone::Hand,
                    use_alternative: Some(_),
                    ..
                },
            } if *spell_id == zombie_id
        )),
        "Rooftop Storm should surface a free hand cast action for Zombie creature spells"
    );
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::PlayFrom {
                    zone: Zone::Hand,
                    use_alternative: Some(_),
                    ..
                },
            } if *spell_id == non_zombie_id
        )),
        "Rooftop Storm should not grant a free cast to non-Zombie creature spells"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_can_cast_spell_with_non_targeted_prevent_all_damage_without_creatures() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;

    let definition =
        crate::cards::CardDefinitionBuilder::new(CardId::from_raw(13000), "Sivvi Cast Probe")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .parse_text(
                "Prevent all damage that would be dealt this turn to creatures you control.",
            )
            .expect("prevent-all damage line should parse as a non-targeted effect");

    let spell_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("player should exist")
        .mana_pool
        .add(ManaSymbol::White, 1);

    let spell_obj = game.object(spell_id).expect("spell should exist").clone();
    assert!(
        can_cast_spell(&game, alice, &spell_obj, &CastingMethod::Normal),
        "spell should be castable without creatures because effect is non-targeted"
    );
}

#[test]
fn test_compute_legal_targets_respects_cant_target_player_restriction() {
    use crate::target::{ChooseSpec, PlayerFilter};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.effect_store
        .cant_effects
        .cant_target_players
        .insert(bob);
    let targets = crate::game_loop::compute_legal_targets(
        &game,
        &ChooseSpec::Player(PlayerFilter::Any),
        alice,
        None,
    );

    assert!(
        !targets.contains(&crate::Target::Player(bob)),
        "untargetable player should not appear in legal target set: {targets:?}"
    );
    assert!(
        targets.contains(&crate::Target::Player(alice)),
        "other legal players should remain targetable: {targets:?}"
    );
}

#[test]
fn test_auto_pass_decision_maker() {
    use crate::decisions::context::PriorityContext;

    let game = setup_game();
    let mut dm = AutoPassDecisionMaker;

    let ctx = PriorityContext::new(PlayerId::from_index(0), vec![LegalAction::PassPriority]);

    let response = dm.decide_priority(&game, &ctx);
    assert!(matches!(response, LegalAction::PassPriority));
}

#[test]
fn test_numeric_input_decision_maker() {
    use crate::decisions::context::PriorityContext;

    let game = setup_game();

    // Test priority decisions with numeric input
    let mut dm = NumericInputDecisionMaker::from_strs(&["0", "1", ""]);

    let legal_actions = vec![
        LegalAction::PassPriority,
        LegalAction::PlayLand {
            land_id: ObjectId::from_raw(1),
        },
    ];

    let ctx = PriorityContext::new(PlayerId::from_index(0), legal_actions.clone());

    // "0" should select PassPriority
    assert!(matches!(
        dm.decide_priority(&game, &ctx),
        LegalAction::PassPriority
    ));

    // "1" should select PlayLand
    let ctx2 = PriorityContext::new(PlayerId::from_index(0), legal_actions.clone());
    assert!(matches!(
        dm.decide_priority(&game, &ctx2),
        LegalAction::PlayLand { .. }
    ));

    // "" (empty) should default to PassPriority
    let ctx3 = PriorityContext::new(PlayerId::from_index(0), legal_actions);
    assert!(matches!(
        dm.decide_priority(&game, &ctx3),
        LegalAction::PassPriority
    ));
}

#[test]
fn test_numeric_input_priority_commander_shortcut_single() {
    use crate::alternative_cast::CastingMethod;
    use crate::decisions::context::PriorityContext;
    use crate::zone::Zone;

    let game = setup_game();
    let mut dm = NumericInputDecisionMaker::from_strs(&["c"]);

    let actions = vec![
        LegalAction::PassPriority,
        LegalAction::CastSpell {
            spell_id: ObjectId::from_raw(100),
            from_zone: Zone::Command,
            casting_method: CastingMethod::Normal,
        },
    ];

    let ctx = PriorityContext::new(PlayerId::from_index(0), actions);
    assert!(matches!(
        dm.decide_priority(&game, &ctx),
        LegalAction::CastSpell {
            from_zone: Zone::Command,
            ..
        }
    ));
}

#[test]
fn test_numeric_input_priority_commander_shortcut_indexed() {
    use crate::alternative_cast::CastingMethod;
    use crate::decisions::context::PriorityContext;
    use crate::zone::Zone;

    let game = setup_game();
    let mut dm = NumericInputDecisionMaker::from_strs(&["c1"]);

    let actions = vec![
        LegalAction::PassPriority,
        LegalAction::CastSpell {
            spell_id: ObjectId::from_raw(101),
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        },
        LegalAction::CastSpell {
            spell_id: ObjectId::from_raw(102),
            from_zone: Zone::Command,
            casting_method: CastingMethod::Normal,
        },
        LegalAction::CastSpell {
            spell_id: ObjectId::from_raw(103),
            from_zone: Zone::Command,
            casting_method: CastingMethod::Normal,
        },
    ];

    let ctx = PriorityContext::new(PlayerId::from_index(0), actions);
    assert!(matches!(
        dm.decide_priority(&game, &ctx),
        LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Command,
            ..
        } if spell_id == ObjectId::from_raw(103)
    ));
}

#[test]
fn test_commander_tax_applies_to_recasts_from_command_zone() {
    use crate::ability::Ability;
    use crate::cost::TotalCost;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let commander = CardBuilder::new(CardId::from_raw(2000), "Test Commander")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let commander_id = game.create_object_from_card(&commander, alice, Zone::Command);
    game.set_as_commander(commander_id, alice);

    for idx in 0..2 {
        let land = CardBuilder::new(
            CardId::from_raw(2100 + idx),
            format!("Green Source {}", idx),
        )
        .card_types(vec![CardType::Land])
        .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        game.object_mut(land_id)
            .expect("green source should exist")
            .abilities_mut()
            .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
    }

    let commander_obj = game
        .object(commander_id)
        .expect("commander should remain in command zone")
        .clone();
    assert!(
        can_cast_spell(&game, alice, &commander_obj, &CastingMethod::Normal),
        "initial cast should be affordable with two mana"
    );

    game.record_commander_cast_from_command_zone(commander_id);
    let commander_obj = game
        .object(commander_id)
        .expect("commander should remain in command zone")
        .clone();
    assert!(
        !can_cast_spell(&game, alice, &commander_obj, &CastingMethod::Normal),
        "recast should require commander tax"
    );

    for idx in 0..2 {
        let land = CardBuilder::new(
            CardId::from_raw(2200 + idx),
            format!("Extra Green Source {}", idx),
        )
        .card_types(vec![CardType::Land])
        .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        game.object_mut(land_id)
            .expect("extra green source should exist")
            .abilities_mut()
            .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
    }

    let commander_obj = game
        .object(commander_id)
        .expect("commander should remain in command zone")
        .clone();
    assert!(
        can_cast_spell(&game, alice, &commander_obj, &CastingMethod::Normal),
        "four mana should pay the taxed commander cost"
    );
}

#[test]
fn test_numeric_input_may_choice() {
    use crate::decisions::context::BooleanContext;

    let game = setup_game();
    let mut dm = NumericInputDecisionMaker::from_strs(&["y", "n", "", "1"]);

    let ctx = BooleanContext {
        player: PlayerId::from_index(0),
        source: Some(ObjectId::from_raw(1)),
        description: "Test?".to_string(),
        source_name: None,
        ui_hints: crate::decisions::context::DecisionUiHints::default(),
    };

    // "y" = true
    assert!(dm.decide_boolean(&game, &ctx));

    // "n" = false
    assert!(!dm.decide_boolean(&game, &ctx));

    // "" = false
    assert!(!dm.decide_boolean(&game, &ctx));

    // "1" = true
    assert!(dm.decide_boolean(&game, &ctx));
}

/// Tests that tapped creatures cannot activate mana abilities with tap costs.
///
/// Scenario: Alice controls an untapped Llanowar Elves (which has "{T}: Add {G}").
/// When untapped, she should be able to activate the mana ability. After tapping it,
/// she should no longer be able to activate the ability.
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_activated_ability_tap_cost_validation() {
    use crate::cards::definitions::llanowar_elves;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase (for priority)
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    // Create Llanowar Elves on battlefield (has {T}: Add {G} - a mana ability)
    let elves_def = llanowar_elves();
    let creature_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);

    // Remove summoning sickness so it can tap
    game.remove_summoning_sickness(creature_id);

    // Check legal actions - should include the mana ability
    let actions = compute_legal_actions(&game, alice);
    assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Should be able to activate untapped creature's tap mana ability"
        );

    // Now tap the creature (simulating it was already tapped for mana earlier)
    game.tap(creature_id);

    // Check legal actions again - should NOT include the mana ability
    let actions = compute_legal_actions(&game, alice);
    assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Should NOT be able to activate already-tapped creature's tap mana ability"
        );
}

#[test]
fn test_activated_ability_mana_cost_validation() {
    use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::effect::Effect;
    use crate::mana::{ManaCost, ManaSymbol};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    // Create a creature with an activated ability that costs {1}{G}
    let creature = CardBuilder::new(CardId::from_raw(1), "Pump Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

    // Add an activated ability: {1}{G}: +2/+2 until EOT
    let mana_cost =
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)], vec![ManaSymbol::Green]]);
    let activated_ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::mana(mana_cost),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::pump(
                2,
                2,
                crate::target::ChooseSpec::Source,
                crate::effect::Until::EndOfTurn,
            )]),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
    };
    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(activated_ability);
    game.remove_summoning_sickness(creature_id);

    // Cost payment is validated during the activation flow, so the action
    // should still surface even before the player floats mana.
    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(
            |a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)
        ),
        "Should surface the activation even before mana is available"
    );

    // Add mana to pool
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Green, 1);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    // Now should be able to activate
    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(
            |a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)
        ),
        "Should be able to activate with sufficient mana"
    );
}

#[test]
fn test_compute_legal_actions_includes_at_least_graveyard_exile_material_cost() {
    use crate::ability::{AbilityKind, ActivatedAbility};
    use crate::card::LinkedFaceLayout;
    use crate::color::{Color, ColorSet};
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::events::KeywordActionKind;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::NextMain;
    game.turn.step = None;

    let ore = CardBuilder::new(CardId::from_raw(40_001), "Ore-Rich Stalactite")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Artifact])
        .other_face(CardId::from_raw(40_002))
        .other_face_name("Cosmium Catalyst")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let ore_id = game.create_object_from_card(&ore, alice, Zone::Battlefield);

    let material_filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
        .with_colors(ColorSet::from_color(Color::Red))
        .with_type(CardType::Instant)
        .with_type(CardType::Sorcery);
    let craft_cost = TotalCost::from_costs(vec![
        Cost::mana(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ])),
        Cost::validated_effect(Effect::exile(
            ChooseSpec::Object(material_filter).with_count(crate::effect::ChoiceCount::at_least(4)),
        )),
        Cost::validated_effect(Effect::emit_keyword_action(KeywordActionKind::Craft, 1)),
        Cost::exile_self(),
    ]);
    let craft_ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: craft_cost,
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::new(
                    crate::effects::MoveToZoneEffect::new(
                        ChooseSpec::Source,
                        Zone::Battlefield,
                        false,
                    )
                    .under_owner_control()
                    .transfer_exiled_with_source_links(),
                ),
                Effect::transform(ChooseSpec::Source),
            ]),
            choices: vec![],
            timing: crate::ability::ActivationTiming::SorcerySpeed,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    };
    game.object_mut(ore_id)
        .expect("Ore-Rich Stalactite should exist")
        .abilities_mut()
        .push(craft_ability);

    for _ in 0..6 {
        let mountain = CardBuilder::new(CardId::new(), "Mountain")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&mountain, alice, Zone::Battlefield);
    }
    // The bare test lands have no mana abilities, so float the craft mana
    // directly — ability affordability now prechecks potential mana.
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 5);

    let material_specs = [
        (
            "Opt",
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
            CardType::Instant,
        ),
        (
            "Arc Lightning",
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Red]]),
            CardType::Sorcery,
        ),
        (
            "Lightning Helix",
            ManaCost::from_pips(vec![vec![ManaSymbol::Red], vec![ManaSymbol::White]]),
            CardType::Instant,
        ),
        (
            "Lightning Strike",
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)], vec![ManaSymbol::Red]]),
            CardType::Instant,
        ),
    ];
    for (name, mana_cost, card_type) in material_specs {
        let card = CardBuilder::new(CardId::new(), name)
            .mana_cost(mana_cost)
            .card_types(vec![card_type])
            .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }

    let actions_before = compute_legal_actions(&game, alice);
    assert!(
            !actions_before
                .iter()
                .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == ore_id)),
            "craft should not be available before the fourth red instant/sorcery card"
        );

    let bolt = CardBuilder::new(CardId::new(), "Lightning Bolt")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_card(&bolt, alice, Zone::Graveyard);

    let actions_after = compute_legal_actions(&game, alice);
    assert!(
            actions_after
                .iter()
                .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == ore_id)),
            "craft should be available after the fourth red instant/sorcery card"
        );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_tayam_wall_of_roots_activation_uses_mana_sequence_solver() {
    use crate::ability::AbilityKind;
    use crate::cards::definitions::{tayam_luminous_enigma, wall_of_roots};
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let tayam_id =
        game.create_object_from_definition(&tayam_luminous_enigma(), alice, Zone::Battlefield);
    let wall_id = game.create_object_from_definition(&wall_of_roots(), alice, Zone::Battlefield);

    if let Some(wall) = game.object_mut(wall_id) {
        wall.counters.insert(CounterType::MinusOneMinusOne, 2);
    }

    // Start with only 2 mana and 2 counters; activation should still be legal
    // because Wall of Roots can be activated during cost payment.
    if let Some(player) = game.player_mut(alice) {
        player.mana_pool.add(ManaSymbol::Colorless, 2);
    }

    let tayam_ability_index = game
        .object(tayam_id)
        .expect("Tayam should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Tayam should have an activated ability");

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source, ability_index }
                if *source == tayam_id && *ability_index == tayam_ability_index
        )),
        "Tayam activation should be legal when Wall of Roots can provide the 3rd mana and 3rd counter during payment"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_tayam_wall_of_roots_activation_blocked_when_wall_already_used() {
    use crate::ability::AbilityKind;
    use crate::cards::definitions::{tayam_luminous_enigma, wall_of_roots};
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let tayam_id =
        game.create_object_from_definition(&tayam_luminous_enigma(), alice, Zone::Battlefield);
    let wall_id = game.create_object_from_definition(&wall_of_roots(), alice, Zone::Battlefield);

    if let Some(wall) = game.object_mut(wall_id) {
        wall.counters.insert(CounterType::MinusOneMinusOne, 2);
    }

    if let Some(player) = game.player_mut(alice) {
        player.mana_pool.add(ManaSymbol::Colorless, 2);
    }

    let wall_mana_ability_index = game
        .object(wall_id)
        .expect("Wall of Roots should exist")
        .abilities
        .iter()
        .position(|ability| ability.is_mana_ability())
        .expect("Wall of Roots should have a mana ability");
    game.record_ability_activation(wall_id, wall_mana_ability_index);

    let tayam_ability_index = game
        .object(tayam_id)
        .expect("Tayam should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Tayam should have an activated ability");

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source, ability_index }
                if *source == tayam_id && *ability_index == tayam_ability_index
        )),
        "Tayam activation should still surface even when the payment flow will reject it"
    );
}

#[test]
fn test_activated_ability_cost_reduction_respects_minimum_one_mana() {
    use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::effect::Effect;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    // Creature with two activated abilities: one costs {2}, one costs {1}.
    let creature = CardBuilder::new(CardId::from_raw(11), "Reducer Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    game.remove_summoning_sickness(creature_id);

    let cost_two = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]);
    let cost_one = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]);
    let activated = |cost: ManaCost| Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::mana(cost),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
    };
    game.object_mut(creature_id)
        .expect("creature exists")
        .abilities_mut()
        .extend([activated(cost_two), activated(cost_one)]);

    // Training Grounds-style static ability.
    let reducer = CardBuilder::new(CardId::from_raw(12), "Training Grounds Effect")
        .card_types(vec![CardType::Enchantment])
        .build();
    let reducer_id = game.create_object_from_card(&reducer, alice, Zone::Battlefield);
    game.object_mut(reducer_id)
        .expect("reducer exists")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::reduce_activated_ability_costs(
                ObjectFilter::creature().you_control(),
                2,
                Some(1),
            ),
        ));

    let actions_without_mana = compute_legal_actions(&game, alice);
    assert!(
        actions_without_mana.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility {
                source,
                ability_index: 0
            } if *source == creature_id
        )),
        "reduced activated abilities should still surface before mana is floated"
    );

    game.player_mut(alice)
        .expect("player exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let actions_with_one = compute_legal_actions(&game, alice);
    assert!(
        actions_with_one.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility {
                source,
                ability_index: 0
            } if *source == creature_id
        )),
        "with one mana, {{2}} ability should be reduced to {{1}}"
    );
    assert!(
        actions_with_one.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility {
                source,
                ability_index: 1
            } if *source == creature_id
        )),
        "minimum-one-mana floor should keep {{1}} ability at {{1}}"
    );

    let reduced_one = calculate_effective_activation_total_cost(
        &game,
        alice,
        creature_id,
        &TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]])),
    );
    assert_eq!(
        reduced_one
            .mana_cost()
            .expect("reduced cost keeps mana component")
            .generic_mana_total(),
        1,
        "minimum-one-mana floor should not reduce a {{1}} activation cost to zero"
    );
}

#[test]
fn test_self_hand_activated_ability_cost_reduction_counts_matching_battlefield_objects() {
    use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::effect::Effect;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let legend = CardBuilder::new(CardId::from_raw(21), "Legendary Scout")
        .supertypes(vec![crate::types::Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&legend, alice, Zone::Battlefield);
    game.create_object_from_card(&legend, alice, Zone::Battlefield);

    let card = CardBuilder::new(CardId::from_raw(22), "Hand Reducer")
        .card_types(vec![CardType::Creature])
        .build();
    let source_id = game.create_object_from_card(&card, alice, Zone::Hand);
    game.object_mut(source_id)
        .expect("source exists")
        .abilities_mut()
        .extend([
            Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: TotalCost::mana(ManaCost::from_pips(vec![vec![
                        ManaSymbol::Generic(3),
                    ]])),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::draw(1),
                    ]),
                    choices: vec![],
                    timing: ActivationTiming::AnyTime,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                    is_loyalty_ability: false,
                }),
                functional_zones: vec![Zone::Hand],
            },
            Ability::static_ability(StaticAbility::reduce_activated_ability_costs_for_each(
                ObjectFilter::source(),
                1,
                ObjectFilter::creature()
                    .you_control()
                    .with_supertype(crate::types::Supertype::Legendary),
                Some(1),
            ))
            .in_zones(vec![Zone::Battlefield, Zone::Hand]),
        ]);

    let reduced = calculate_effective_activation_mana_cost(
        &game,
        alice,
        source_id,
        &ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]),
    );
    assert_eq!(
        reduced.generic_mana_total(),
        1,
        "two matching legendary creatures should reduce a hand-zone activation from {{3}} to {{1}}"
    );
}

#[test]
fn test_activated_ability_cost_reduction_counts_distinct_basic_land_types() {
    use crate::ability::Ability;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(31), "Domain Codex")
        .card_types(vec![CardType::Artifact])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    game.object_mut(source_id)
        .expect("source exists")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::reduce_activated_ability_costs_for_each_basic_land_type(
                ObjectFilter::source(),
                1,
                ObjectFilter::land().you_control(),
                Some(1),
            ),
        ));

    for (id, name, subtype) in [
        (32, "Plains", Subtype::Plains),
        (33, "Snowfield", Subtype::Plains),
        (34, "Island", Subtype::Island),
    ] {
        let land = CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Land])
            .subtypes(vec![subtype])
            .build();
        game.create_object_from_card(&land, alice, Zone::Battlefield);
    }

    let reduced = calculate_effective_activation_mana_cost(
        &game,
        alice,
        source_id,
        &ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]),
    );
    assert_eq!(
        reduced.generic_mana_total(),
        3,
        "three lands with two basic land types should reduce {{5}} by two"
    );
}

/// Tests that summoning sick creatures cannot activate mana abilities with tap costs.
///
/// Scenario: Alice casts Llanowar Elves. On the same turn, the creature has
/// summoning sickness, so she should not be able to activate its "{T}: Add {G}"
/// mana ability.
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_activated_ability_summoning_sickness_blocks_tap() {
    use crate::cards::definitions::llanowar_elves;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    // Create Llanowar Elves on battlefield with summoning sickness
    let elves_def = llanowar_elves();
    let creature_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);

    // Creature just entered battlefield, so it has summoning sickness
    game.set_summoning_sick(creature_id);

    // Should NOT be able to activate tap mana ability due to summoning sickness
    let actions = compute_legal_actions(&game, alice);
    assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Summoning sick creature should not be able to use tap mana abilities"
        );
}

/// Tests that creatures with haste can use tap mana abilities despite summoning sickness.
///
/// Scenario: Alice has given her Llanowar Elves haste (e.g., via an effect like
/// Swiftfoot Boots). Even though the creature just entered the battlefield and
/// has summoning sickness, haste allows it to activate its "{T}: Add {G}" mana ability.
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_activated_ability_haste_bypasses_summoning_sickness() {
    use crate::ability::Ability;
    use crate::cards::definitions::llanowar_elves;
    use crate::static_abilities::StaticAbility;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    // Create Llanowar Elves with summoning sickness but also with haste
    let elves_def = llanowar_elves();
    let creature_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);

    // Add haste (e.g., from equipment or an enchantment)
    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::haste()));

    // Creature just entered battlefield, so it has summoning sickness
    game.set_summoning_sick(creature_id);

    // Should be able to activate tap mana ability despite summoning sickness (has haste)
    let actions = compute_legal_actions(&game, alice);
    assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Creature with haste should be able to use tap mana abilities despite summoning sickness"
        );
}

#[test]
fn test_compute_legal_actions_includes_turn_face_up_for_morph() {
    use crate::ability::Ability;
    use crate::static_abilities::StaticAbility;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let creature = CardBuilder::new(CardId::from_raw(101), "Morph Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::morph(
            crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Green,
            ]])),
        )));
    game.set_face_down(creature_id);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(crate::mana::ManaSymbol::Green, 1);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(
            |a| matches!(a, LegalAction::TurnFaceUp { creature_id: id, .. } if *id == creature_id)
        ),
        "face-down creature with payable morph cost should have TurnFaceUp legal action"
    );
}

#[test]
fn test_compute_legal_actions_includes_face_down_cast_for_morph_when_normal_cast_is_too_expensive()
{
    use crate::ability::Ability;
    use crate::static_abilities::StaticAbility;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    let creature = CardBuilder::new(CardId::from_raw(102), "Costly Morph Bear")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![
            vec![crate::mana::ManaSymbol::Generic(5)],
            vec![crate::mana::ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);
    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::morph(
            crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Green,
            ]])),
        )));
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(crate::mana::ManaSymbol::Colorless, 3);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::FaceDown,
            } if *spell_id == creature_id
        )),
        "morph card should be castable face down when {{3}} is payable"
    );
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == creature_id
        )),
        "normal cast should stay unavailable when the printed mana cost is too expensive"
    );
}

#[test]
fn test_activated_ability_sorcery_speed_timing() {
    use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::effect::Effect;
    use crate::game_state::Step;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create a creature with sorcery-speed activated ability
    let creature = CardBuilder::new(CardId::from_raw(1), "Sorcery Speed Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

    // Add sorcery-speed activated ability (no cost, just free)
    let activated_ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::free(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life(1)]),
            choices: vec![],
            timing: ActivationTiming::SorcerySpeed,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
    };
    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(activated_ability);
    game.remove_summoning_sickness(creature_id);

    // Main phase, empty stack - should be able to activate
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(
            |a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)
        ),
        "Should be able to activate sorcery-speed ability during main phase with empty stack"
    );

    // Combat phase - should NOT be able to activate
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(
            |a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)
        ),
        "Should NOT be able to activate sorcery-speed ability during combat"
    );
}

#[test]
fn test_compute_legal_actions_includes_hand_activated_ability() {
    use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::effect::Effect;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let card = CardBuilder::new(CardId::from_raw(777), "Hand Ability Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source_id = game.create_object_from_card(&card, alice, Zone::Hand);
    game.object_mut(source_id)
        .expect("source card should exist")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::gain_life(1),
                ]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Hand],
        });

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(
            |a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == source_id)
        ),
        "hand-zone activated ability should be discoverable as a legal action"
    );
}

#[test]
fn forecast_activation_requires_the_source_owners_upkeep_and_is_once_per_turn() {
    use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::effect::Effect;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = CardBuilder::new(CardId::from_raw(57_200), "Forecast Timing Probe").build();
    let source_id = game.create_object_from_card(&card, alice, Zone::Hand);
    game.object_mut(source_id)
        .expect("Forecast source")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::gain_life(1),
                ]),
                choices: vec![],
                timing: ActivationTiming::DuringSourceOwnersUpkeep,
                additional_restrictions: vec![],
                activation_restrictions: vec![crate::ConditionExpr::MaxActivationsPerTurn(1)],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Hand],
        });

    let has_forecast_action = |game: &GameState| {
        compute_legal_actions(game, alice).iter().any(|action| {
            matches!(action, LegalAction::ActivateAbility { source, .. } if *source == source_id)
        })
    };

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Draw);
    assert!(!has_forecast_action(&game));

    game.turn.step = Some(Step::Upkeep);
    assert!(has_forecast_action(&game));

    game.record_ability_activation(source_id, 0);
    assert!(!has_forecast_action(&game));

    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    assert!(
        !has_forecast_action(&game),
        "Forecast follows the card owner's upkeep, not another player's upkeep"
    );
}

#[test]
fn any_player_mana_activation_uses_the_activators_turn_before_end_step() {
    use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = CardBuilder::new(CardId::from_raw(57_201), "Shared Mana Source")
        .card_types(vec![CardType::Enchantment])
        .build();
    let source_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
    game.object_mut(source_id)
        .expect("shared mana source")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::default(),
                choices: vec![],
                timing: ActivationTiming::AnyPlayerDuringTheirTurnBeforeEndStep,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: Some(vec![ManaSymbol::Colorless]),
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    let bob_can_activate = |game: &GameState| {
        compute_legal_actions(game, bob).iter().any(|action| {
            matches!(
                action,
                LegalAction::ActivateManaAbility { source, .. } if *source == source_id
            )
        })
    };

    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    game.turn.phase = Phase::NextMain;
    game.turn.step = None;
    assert!(
        bob_can_activate(&game),
        "an opponent may activate the source during that opponent's own turn"
    );

    game.turn.active_player = alice;
    game.turn.priority_player = Some(bob);
    assert!(
        !bob_can_activate(&game),
        "the permission does not extend into another player's turn"
    );

    game.turn.active_player = bob;
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(Step::End);
    assert!(
        !bob_can_activate(&game),
        "the activator-relative permission ends before the end step"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_compute_legal_actions_excludes_hand_only_ability_from_battlefield() {
    use crate::cards::CardDefinitionBuilder;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let def = CardDefinitionBuilder::new(CardId::from_raw(779), "Boseiju Regression Probe")
            .card_types(vec![CardType::Land])
            .mana_cost(ManaCost::new())
            .parse_text(
                "{T}: Add {G}.\nChannel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls.\nThis ability costs {1} less to activate for each legendary creature you control.",
            )
            .expect("channel land probe should parse");

    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(source_id);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::ActivateManaAbility { source, .. } if *source == source_id
        )),
        "battlefield Boseiju should still expose its tap-for-mana ability"
    );
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source, .. } if *source == source_id
        )),
        "battlefield Boseiju should not expose its hand-only channel ability"
    );
}

#[test]
fn test_tap_only_activation_skips_empty_next_cost_prompt() {
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm};
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(780), "Tap Probe")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.object_mut(source_id)
        .expect("tap probe should exist")
        .abilities_mut()
        .push(Ability::activated_with_costs(
            TotalCost::free(),
            vec![Cost::tap()],
            vec![Effect::gain_life(1)],
        ));

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
            source: source_id,
            ability_index: 0,
        }),
        &mut dm,
    )
    .expect("tap-only activation should resolve its cost flow");

    assert!(
        state.pending_activation.is_none(),
        "tap-only activation should not get stuck in a pending cost prompt"
    );
    assert!(
        game.is_tapped(source_id),
        "tap-only activation should pay the tap cost immediately"
    );
    assert_eq!(
        game.stack.len(),
        1,
        "tap-only activation should place the ability on the stack"
    );
    assert!(
        matches!(
            progress,
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_)
            )
        ),
        "after a tap-only activation resolves its cost, priority should continue normally"
    );
}

#[test]
fn test_compute_legal_actions_excludes_tapped_non_mana_tap_ability() {
    use crate::cost::TotalCost;
    use crate::costs::Cost;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(781), "Tapped Ability Probe")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.object_mut(source_id)
        .expect("probe should exist")
        .abilities_mut()
        .push(Ability::activated_with_costs(
            TotalCost::free(),
            vec![Cost::tap()],
            vec![Effect::gain_life(1)],
        ));
    game.tap(source_id);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source, .. } if *source == source_id
        )),
        "tapped permanents should not expose non-mana tap abilities as legal actions"
    );
}

#[test]
fn test_compute_legal_actions_excludes_summoning_sick_non_mana_untap_ability() {
    use crate::cost::TotalCost;
    use crate::costs::Cost;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(782), "Untap Ability Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.object_mut(source_id)
        .expect("probe should exist")
        .abilities_mut()
        .push(Ability::activated_with_costs(
            TotalCost::free(),
            vec![Cost::untap()],
            vec![Effect::gain_life(1)],
        ));
    game.tap(source_id);
    game.set_summoning_sick(source_id);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source, .. } if *source == source_id
        )),
        "summoning-sick creatures should not expose non-mana untap abilities as legal actions"
    );

    game.remove_summoning_sickness(source_id);
    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source, .. } if *source == source_id
        )),
        "once summoning sickness is removed, the untap ability should become legal"
    );
}

#[test]
fn test_compute_legal_actions_includes_foretell_special_action() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 2);

    let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(778), "Foretell Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .foretell(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::SpecialAction(crate::special_actions::SpecialAction::Foretell {
                card_id: found
            }) if *found == card_id
        )),
        "expected foretell special action in legal actions, got {actions:?}"
    );
}

#[test]
fn test_compute_legal_actions_includes_suspend_special_action() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 1);

    let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(779), "Suspend Probe")
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .suspend(2, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .build();
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::SpecialAction(crate::special_actions::SpecialAction::Suspend {
                card_id: found
            }) if *found == card_id
        )),
        "expected suspend special action in legal actions, got {actions:?}"
    );
}

#[test]
fn test_suspend_special_action_respects_cant_cast_restrictions() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 1);

    let def = crate::cards::CardDefinitionBuilder::new(
        CardId::from_raw(7791),
        "Suspend Restriction Probe",
    )
    .card_types(vec![CardType::Creature])
    .power_toughness(PowerToughness::fixed(2, 2))
    .suspend(2, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
    .build();
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    game.effect_store.cant_effects.add_cant_cast_filter(
        alice,
        crate::target::ObjectFilter::default().with_type(CardType::Creature),
    );

    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::SpecialAction(crate::special_actions::SpecialAction::Suspend {
                card_id: found
            }) if *found == card_id
        )),
        "suspend should not be offered when a cast prohibition would stop starting the cast, got {actions:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_suspend_only_card_does_not_offer_normal_cast_from_hand() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(782), "Lotus Bloom")
            .parse_text(
                "Type: Artifact\n\
                 Suspend 3—{0} (Rather than cast this card from your hand, pay {0} and exile it with three time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)\n\
                 {T}, Sacrifice this artifact: Add three mana of any one color.",
            )
            .expect("Lotus Bloom text should parse");
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == card_id
        )),
        "suspend-only card should not offer a normal cast action, got {actions:?}"
    );
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::SpecialAction(crate::special_actions::SpecialAction::Suspend {
                card_id: found
            }) if *found == card_id
        )),
        "suspend-only card should still offer suspend, got {actions:?}"
    );
}

#[test]
fn test_suspend_special_action_exiles_with_time_counters() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 1);

    let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(781), "Suspend Runtime")
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .suspend(2, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .build();
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::Suspend { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("suspend special action should resolve");

    let exiled_id = *game.exile.first().expect("card should be exiled");
    let exiled = game.object(exiled_id).expect("exiled card should exist");
    assert_eq!(exiled.zone, Zone::Exile);
    assert_eq!(
        game.counter_count(exiled_id, crate::object::CounterType::Time),
        2
    );
}

#[test]
fn test_plot_special_action_enables_cast_on_later_turn_only() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 3);

    let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(780), "Plot Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .plot(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .build();
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::Plot { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("plot special action should resolve");

    let exiled_id = *game.exile.first().expect("card should be in exile");
    let same_turn_actions = compute_legal_actions(&game, alice);
    assert!(
        !same_turn_actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == exiled_id
        )),
        "plotted card should not be castable the same turn it was plotted"
    );

    game.next_turn();
    game.next_turn();
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let later_actions = compute_legal_actions(&game, alice);
    assert!(
        later_actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == exiled_id
        )),
        "plotted card should be castable from exile on a later turn"
    );
}

#[test]
fn test_aloe_alchemist_plot_records_plot_keyword_action_event() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 2);

    let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(783), "Aloe Alchemist")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Trample\nWhen this card becomes plotted, target creature gets +3/+2 and gains trample until end of turn.\nPlot {1}{G} (You may pay {1}{G} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
            )
            .expect("Aloe Alchemist oracle text should parse");
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::Plot { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("Aloe Alchemist plot special action should resolve");

    let exiled_id = *game
        .exile
        .first()
        .expect("Aloe Alchemist should be in exile");
    assert!(
        game.object_performed_keyword_action_this_turn(
            exiled_id,
            crate::events::KeywordActionKind::Plot,
        ),
        "plotting Aloe Alchemist should record a plot keyword action event"
    );
}

#[test]
fn test_spectacle_condition_controls_alternative_cast_legality() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(782), "Spectacle Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .spectacle(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .build();
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let card = game.object(card_id).expect("spectacle card should exist");
    assert!(
        !can_cast_with_alternative_from_hand(
            &game,
            alice,
            card,
            card_id,
            &card.alternative_casts[0]
        ),
        "spectacle alternative should not be available before an opponent loses life"
    );

    stage_life_loss_for_test(&mut game, bob, 1);
    let card = game
        .object(card_id)
        .expect("spectacle card should still exist");
    assert!(
        can_cast_with_alternative_from_hand(
            &game,
            alice,
            card,
            card_id,
            &card.alternative_casts[0]
        ),
        "spectacle alternative should become available once an opponent has lost life"
    );
}

#[test]
fn test_bestow_alternative_uses_aura_cost_reductions() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    for _ in 0..4 {
        game.create_object_from_definition(&basic_forest(), alice, Zone::Battlefield);
    }
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(881), "Cost Reducer")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .parse_text("Aura spells you cast cost {1} less to cast.")
            .expect("cost reducer should parse"),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(882), "Second Cost Reducer")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .parse_text("Aura spells you cast cost {1} less to cast.")
            .expect("cost reducer should parse"),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(883), "Bestow Host")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );

    let bestow_def = CardDefinitionBuilder::new(CardId::from_raw(884), "Bestow Cost Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Bestow {5}{G}\nEnchanted creature gets +3/+3.")
        .expect("bestow card should parse");
    let card_id = game.create_object_from_definition(&bestow_def, alice, Zone::Hand);
    let card = game.object(card_id).expect("bestow card should exist");
    let view = DerivedGameView::new(&game);
    let effective = calculate_effective_mana_cost_with_view_for_casting_method(
        &game,
        alice,
        card,
        &ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)], vec![ManaSymbol::Green]]),
        &CastingMethod::Alternative(0),
        &view,
    );
    assert_eq!(
        effective,
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)], vec![ManaSymbol::Green]])
    );

    assert!(
        can_cast_with_alternative_from_hand(
            &game,
            alice,
            card,
            card_id,
            &card.alternative_casts[0]
        ),
        "two Aura cost reducers should make bestow {{5}}{{G}} payable with four Forests"
    );
}

#[test]
fn semblance_anvil_imprint_trigger_exiles_card_and_records_source_link() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let semblance = CardDefinitionBuilder::new(CardId::from_raw(9010), "Semblance Anvil")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "Imprint — When this artifact enters, you may exile a nonland card from your hand.\n\
                 Spells you cast that share a card type with the exiled card cost {2} less to cast.",
            )
            .expect("Semblance Anvil should parse");
    let anvil_id = game.create_object_from_definition(&semblance, alice, Zone::Battlefield);
    let spell = CardBuilder::new(CardId::from_raw(9011), "Semblance Anvil Imprint Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .build();
    let original_card_id = game.create_object_from_card(&spell, alice, Zone::Hand);

    let triggered = semblance
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Semblance Anvil should have an imprint trigger");
    let mut ctx = ExecutionContext::new_default(anvil_id, alice);
    for effect in &triggered.effects {
        effect.0.execute(&mut game, &mut ctx).unwrap();
    }

    assert!(
        !game
            .object(original_card_id)
            .is_some_and(|obj| obj.zone == Zone::Hand)
    );
    let imprinted = game.get_imprinted_cards(anvil_id);
    assert_eq!(
        imprinted.len(),
        1,
        "Semblance Anvil should imprint one card"
    );
    assert_eq!(game.get_exiled_with_source_links(anvil_id), imprinted);
    assert_eq!(
        game.object(imprinted[0]).map(|obj| obj.zone),
        Some(Zone::Exile),
        "Semblance Anvil should move the imprinted card to exile"
    );
}

#[test]
fn semblance_anvil_declined_imprint_leaves_card_in_hand_and_costs_unchanged() {
    struct DeclineImprint;

    impl DecisionMaker for DeclineImprint {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            Vec::new()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let semblance = CardDefinitionBuilder::new(CardId::from_raw(9012), "Semblance Anvil")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "Imprint — When this artifact enters, you may exile a nonland card from your hand.\n\
                 Spells you cast that share a card type with the exiled card cost {2} less to cast.",
            )
            .expect("Semblance Anvil should parse");
    let anvil_id = game.create_object_from_definition(&semblance, alice, Zone::Battlefield);
    let imprinted_candidate = CardBuilder::new(
        CardId::from_raw(9013),
        "Semblance Anvil Declined Imprint Probe",
    )
    .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
    .card_types(vec![CardType::Artifact])
    .build();
    let original_card_id = game.create_object_from_card(&imprinted_candidate, alice, Zone::Hand);

    let triggered = semblance
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Semblance Anvil should have an imprint trigger");
    let mut dm = DeclineImprint;
    let mut ctx = ExecutionContext::new_default(anvil_id, alice).with_decision_maker(&mut dm);
    for effect in &triggered.effects {
        effect.0.execute(&mut game, &mut ctx).unwrap();
    }

    assert_eq!(
        game.object(original_card_id).map(|obj| obj.zone),
        Some(Zone::Hand)
    );
    assert!(game.get_imprinted_cards(anvil_id).is_empty());
    assert!(game.get_exiled_with_source_links(anvil_id).is_empty());

    let artifact_spell = CardBuilder::new(
        CardId::from_raw(9014),
        "Semblance Anvil Declined Cost Probe",
    )
    .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
    .card_types(vec![CardType::Artifact])
    .build();
    let artifact_spell_id = game.create_object_from_card(&artifact_spell, alice, Zone::Hand);
    let artifact_spell = game
        .object(artifact_spell_id)
        .expect("artifact spell exists");
    let unreduced = calculate_effective_mana_cost(
        &game,
        alice,
        artifact_spell,
        artifact_spell.mana_cost.as_ref().unwrap(),
    );

    assert_eq!(unreduced.to_oracle(), "{4}");
}

#[test]
fn semblance_anvil_reduces_only_your_spells_sharing_imprinted_card_type() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let semblance = CardDefinitionBuilder::new(CardId::from_raw(9020), "Semblance Anvil")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "Imprint — When this artifact enters, you may exile a nonland card from your hand.\n\
                 Spells you cast that share a card type with the exiled card cost {2} less to cast.",
            )
            .expect("Semblance Anvil should parse");
    let anvil_id = game.create_object_from_definition(&semblance, alice, Zone::Battlefield);
    let imprinted = CardBuilder::new(CardId::from_raw(9021), "Semblance Anvil Exiled Artifact")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let imprinted_id = game.create_object_from_card(&imprinted, alice, Zone::Exile);
    game.imprint_card(anvil_id, imprinted_id);
    game.add_exiled_with_source_link(anvil_id, imprinted_id);

    let artifact_spell = CardBuilder::new(CardId::from_raw(9022), "Semblance Anvil Artifact Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_spell_id = game.create_object_from_card(&artifact_spell, alice, Zone::Hand);
    let artifact_spell = game
        .object(artifact_spell_id)
        .expect("artifact spell exists");
    let reduced = calculate_effective_mana_cost(
        &game,
        alice,
        artifact_spell,
        artifact_spell.mana_cost.as_ref().unwrap(),
    );
    assert_eq!(reduced.to_oracle(), "{2}");

    let sorcery = CardBuilder::new(CardId::from_raw(9023), "Semblance Anvil Sorcery Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);
    let sorcery = game.object(sorcery_id).expect("sorcery exists");
    let unreduced_sorcery =
        calculate_effective_mana_cost(&game, alice, sorcery, sorcery.mana_cost.as_ref().unwrap());
    assert_eq!(unreduced_sorcery.to_oracle(), "{4}");

    let bob_artifact = CardBuilder::new(CardId::from_raw(9024), "Semblance Anvil Opponent Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let bob_artifact_id = game.create_object_from_card(&bob_artifact, bob, Zone::Hand);
    let bob_artifact = game
        .object(bob_artifact_id)
        .expect("opponent artifact exists");
    let unreduced_opponent = calculate_effective_mana_cost(
        &game,
        bob,
        bob_artifact,
        bob_artifact.mana_cost.as_ref().unwrap(),
    );
    assert_eq!(unreduced_opponent.to_oracle(), "{4}");
}

#[test]
fn semblance_anvil_does_not_reduce_without_an_exiled_card() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let semblance = CardDefinitionBuilder::new(CardId::from_raw(9030), "Semblance Anvil")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "Imprint — When this artifact enters, you may exile a nonland card from your hand.\n\
                 Spells you cast that share a card type with the exiled card cost {2} less to cast.",
            )
            .expect("Semblance Anvil should parse");
    game.create_object_from_definition(&semblance, alice, Zone::Battlefield);

    let artifact_spell =
        CardBuilder::new(CardId::from_raw(9031), "Semblance Anvil No Imprint Probe")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Artifact])
            .build();
    let artifact_spell_id = game.create_object_from_card(&artifact_spell, alice, Zone::Hand);
    let artifact_spell = game
        .object(artifact_spell_id)
        .expect("artifact spell exists");
    let unreduced = calculate_effective_mana_cost(
        &game,
        alice,
        artifact_spell,
        artifact_spell.mana_cost.as_ref().unwrap(),
    );

    assert_eq!(unreduced.to_oracle(), "{4}");
}

#[test]
fn test_foretell_special_action_enables_cast_from_exile() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 4);

    let def =
        crate::cards::CardDefinitionBuilder::new(CardId::from_raw(779), "Foretell Runtime Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Instant])
            .with_spell_effect(vec![Effect::gain_life(1)])
            .foretell(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
    let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    let mut dm = SelectFirstDecisionMaker;
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::Foretell { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("foretell special action should succeed");

    let foretold_id = *game.exile.last().expect("card should be in exile");
    assert!(game.is_face_down(foretold_id));
    assert!(game.is_foretold(foretold_id));

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == foretold_id
        )),
        "expected foretold card to be castable from exile, got {actions:?}"
    );
}

/// Tests that compute_potential_mana correctly calculates mana from untapped sources.
///
/// Scenario: Player has empty mana pool but 4 untapped Mountains on battlefield.
/// compute_potential_mana should return a pool with 4 red mana.
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_compute_potential_mana_with_untapped_lands() {
    use crate::cards::definitions::basic_mountain;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    // Verify mana pool is empty
    assert_eq!(
        game.player(alice).unwrap().mana_pool.total(),
        0,
        "Mana pool should start empty"
    );

    // Create 4 Mountains on battlefield
    let mountain_def = basic_mountain();
    for _ in 0..4 {
        game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);
    }

    // compute_potential_mana should include mana from untapped lands
    let potential = compute_potential_mana(&game, alice);
    assert_eq!(
        potential.red, 4,
        "Should have 4 potential red mana from Mountains"
    );
    assert_eq!(potential.total(), 4, "Total potential mana should be 4");
}

/// Tests that max_x_for_cost works correctly with potential mana.
///
/// Scenario: Player has empty mana pool but 4 untapped Mountains.
/// For a Fireball ({X}{R}), max X should be 3 (4 total mana - 1 for {R} = 3 for X).
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_max_x_with_potential_mana() {
    use crate::cards::definitions::basic_mountain;
    use crate::mana::{ManaCost, ManaSymbol};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    // Verify mana pool is empty
    assert_eq!(
        game.player(alice).unwrap().mana_pool.total(),
        0,
        "Mana pool should start empty"
    );

    // Create 4 Mountains on battlefield
    let mountain_def = basic_mountain();
    for _ in 0..4 {
        game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);
    }

    // Fireball cost: {X}{R}
    let fireball_cost = ManaCost::from_pips(vec![vec![ManaSymbol::X], vec![ManaSymbol::Red]]);

    // Using just the mana pool (which is empty), max_x would be 0
    let max_x_from_pool = game
        .player(alice)
        .unwrap()
        .mana_pool
        .max_x_for_cost(&fireball_cost);
    assert_eq!(max_x_from_pool, 0, "max_x from empty pool should be 0");

    // Using potential mana (including untapped lands), max_x should be 3
    let potential = compute_potential_mana(&game, alice);
    let max_x_from_potential = potential.max_x_for_cost(&fireball_cost);
    assert_eq!(
        max_x_from_potential, 3,
        "max_x from potential mana should be 3 (4 mana - 1 for R = 3 for X)"
    );
}

/// Tests that potential mana includes mana dorks (creatures with mana abilities).
///
/// Scenario: Player has 1 Mountain and 1 Llanowar Elves (untapped, no summoning sickness).
/// For Fireball ({X}{R}), max X should be 1 (2 total mana - 1 for {R} = 1 for X).
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_max_x_with_mana_dork() {
    use crate::cards::definitions::{basic_mountain, llanowar_elves};
    use crate::mana::{ManaCost, ManaSymbol};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    // Create Mountain and Llanowar Elves
    let mountain_def = basic_mountain();
    game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);

    let elves_def = llanowar_elves();
    let elves_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(elves_id);

    // Fireball cost: {X}{R}
    let fireball_cost = ManaCost::from_pips(vec![vec![ManaSymbol::X], vec![ManaSymbol::Red]]);

    // Potential mana: 1R from Mountain + 1G from Elves = 2 total
    let potential = compute_potential_mana(&game, alice);
    assert_eq!(potential.red, 1, "Should have 1 potential red mana");
    assert_eq!(potential.green, 1, "Should have 1 potential green mana");
    assert_eq!(potential.total(), 2, "Total potential mana should be 2");

    // max_x should be 1: pay {R} with Mountain, {X}=1 with Elves' green mana
    let max_x = potential.max_x_for_cost(&fireball_cost);
    assert_eq!(max_x, 1, "max_x should be 1 (2 total - 1 for R = 1 for X)");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_graveyard_play_from_actions_include_variable_mana_sources() {
    use crate::cards::definitions::lightning_bolt;
    use crate::cards::tokens::treasure_token_definition;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    // Treasure's mana ability is effect-backed ("any color"), so this specifically
    // verifies variable mana producers are considered in castability checks.
    let treasure = treasure_token_definition();
    game.create_object_from_definition(&treasure, alice, Zone::Battlefield);

    let bolt = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Graveyard);

    let source_id = game.new_object_id();
    game.effect_store
        .grant_registry
        .grant_to_filter_until_end_of_turn(
            ObjectFilter::nonland(),
            Zone::Graveyard,
            alice,
            Grantable::play_from(),
            source_id,
            game.turn.turn_number,
        );

    let actions = compute_legal_actions(&game, alice);
    let can_cast_from_graveyard = actions.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::PlayFrom {
                    zone: Zone::Graveyard,
                    ..
                },
                ..
            } if *spell_id == bolt_id
        )
    });

    assert!(
        can_cast_from_graveyard,
        "variable mana sources should allow castability inference for play-from-graveyard actions"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_counter_unless_pays_spell_not_castable_without_stack_target() {
    use crate::cards::definitions::{basic_island, mana_tithe};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Give Alice the mana to cast Mana Tithe.
    let island = basic_island();
    game.create_object_from_definition(&island, alice, Zone::Battlefield);

    // Put Mana Tithe in hand and leave stack empty.
    let mana_tithe_def = mana_tithe();
    let mana_tithe_id = game.create_object_from_definition(&mana_tithe_def, alice, Zone::Hand);

    let actions = compute_legal_actions(&game, alice);
    let can_cast = actions.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == mana_tithe_id
        )
    });

    assert!(
        !can_cast,
        "counter-unless-pays spells must not be castable without a legal spell target on stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_conditional_counter_spell_not_castable_without_stack_target() {
    use crate::cards::definitions::basic_island;
    use crate::effect::Condition;
    use crate::game_state::StackEntry;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Give Alice the mana to cast the conditional counterspell.
    let island = basic_island();
    game.create_object_from_definition(&island, alice, Zone::Battlefield);

    // Corrupted Resolve-shaped payload:
    // "Counter target spell if its controller is poisoned."
    let card = CardBuilder::new(CardId::from_raw(91), "Corrupted Resolve Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .build();
    let spell_id = game.create_object_from_card(&card, alice, Zone::Hand);
    game.object_mut(spell_id)
        .expect("spell exists")
        .spell_effect = Some(
        crate::resolution::ResolutionProgram::from_effects(vec![Effect::conditional(
            Condition::TargetSpellControllerIsPoisoned,
            vec![Effect::counter(ChooseSpec::target_spell())],
            vec![],
        )])
        .into(),
    );

    // With no spell on stack, the counterspell must not be castable.
    let actions_without_stack = compute_legal_actions(&game, alice);
    let can_cast_without_stack = actions_without_stack.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )
    });
    assert!(
        !can_cast_without_stack,
        "conditional counterspell should not be castable without a legal spell target on stack"
    );

    // Add a dummy spell to the stack and verify the cast action appears.
    let dummy_spell = CardBuilder::new(CardId::from_raw(92), "Stack Dummy")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .build();
    let dummy_id = game.create_object_from_card(&dummy_spell, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(dummy_id, bob));

    let actions_with_stack = compute_legal_actions(&game, alice);
    let can_cast_with_stack = actions_with_stack.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )
    });
    assert!(
        can_cast_with_stack,
        "conditional counterspell should be castable once a legal spell target exists on stack"
    );
}

#[test]
fn optional_cost_can_make_cast_time_targets_legal() {
    use crate::effect::Condition;
    use crate::game_state::StackEntry;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let island = basic_island();
    game.create_object_from_definition(&island, alice, Zone::Battlefield);
    game.create_object_from_definition(&island, alice, Zone::Battlefield);

    let card = CardBuilder::new(CardId::from_raw(95), "Long River's Pull Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Blue,
            ManaSymbol::Blue,
        ]))
        .build();
    let spell_id = game.create_object_from_card(&card, alice, Zone::Hand);

    let gift_player_cost = crate::costs::Cost::effect(
        crate::effects::ChoosePlayerEffect::new(
            PlayerFilter::You,
            PlayerFilter::Opponent,
            "gifted_player",
        )
        .remember_as_chosen_player(),
    );
    let mut creature_spell_filter = ObjectFilter::spell();
    creature_spell_filter.card_types.push(CardType::Creature);
    let program =
        crate::resolution::ResolutionProgram::new(vec![crate::resolution::ResolutionSegment {
            default_effects: vec![Effect::counter(ChooseSpec::target(ChooseSpec::Object(
                creature_spell_filter,
            )))],
            self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                Condition::ThisSpellPaidLabel("Gift".into()),
                vec![Effect::counter(ChooseSpec::target_spell())],
            )],
            starts_new_source_line: false,
        }]);
    if let Some(spell) = game.object_mut(spell_id) {
        spell.optional_costs = vec![crate::cost::OptionalCost::custom(
            "Gift a card",
            crate::cost::TotalCost::from_cost(gift_player_cost),
        )]
        .into();
        spell.spell_effect = Some(program.into());
    }

    let dummy_spell = CardBuilder::new(CardId::from_raw(96), "Noncreature Stack Spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .build();
    let dummy_id = game.create_object_from_card(&dummy_spell, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(dummy_id, bob));

    let actions = compute_legal_actions(&game, alice);
    let can_cast = actions.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )
    });
    assert!(
        can_cast,
        "a payable Gift cost should expose the promised branch's noncreature spell target"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_if_effect_counter_spell_not_castable_without_stack_target() {
    use crate::cards::definitions::basic_island;
    use crate::game_state::StackEntry;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Give Alice the mana to cast the spell.
    let island = basic_island();
    game.create_object_from_definition(&island, alice, Zone::Battlefield);

    // "If you do, counter target spell." shape:
    // represented as IfEffect branching on a prior effect result.
    let card = CardBuilder::new(CardId::from_raw(93), "If Counter Variant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .build();
    let spell_id = game.create_object_from_card(&card, alice, Zone::Hand);
    game.object_mut(spell_id)
        .expect("spell exists")
        .spell_effect = Some(
        crate::resolution::ResolutionProgram::from_effects(vec![Effect::if_then(
            crate::effect::EffectId(0),
            crate::effect::EffectPredicate::Happened,
            vec![Effect::counter(ChooseSpec::target_spell())],
        )])
        .into(),
    );

    // With no spell on stack, the spell must not be castable.
    let actions_without_stack = compute_legal_actions(&game, alice);
    let can_cast_without_stack = actions_without_stack.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )
    });
    assert!(
        !can_cast_without_stack,
        "if-effect counterspell should not be castable without a legal spell target on stack"
    );

    // Add a legal stack spell; cast action should appear.
    let dummy_spell = CardBuilder::new(CardId::from_raw(94), "If Stack Dummy")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .build();
    let dummy_id = game.create_object_from_card(&dummy_spell, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(dummy_id, bob));

    let actions_with_stack = compute_legal_actions(&game, alice);
    let can_cast_with_stack = actions_with_stack.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )
    });
    assert!(
        can_cast_with_stack,
        "if-effect counterspell should be castable once a legal spell target exists on stack"
    );
}
