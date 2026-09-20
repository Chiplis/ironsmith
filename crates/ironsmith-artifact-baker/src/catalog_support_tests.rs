use super::*;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    text: String,
}

#[test]
fn catalog_strict_parser_regressions() {
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("catalog_regressions.json.fixture")).unwrap();
    let mut failures = Vec::new();
    for fixture in &fixtures {
        match compile_artifact(CompileInput {
            name: &fixture.name,
            text: &fixture.text,
            score: None,
            local_id: 1,
            other_face_id: None,
            other_face_name: None,
            layout: LinkedFaceLayout::None,
        }) {
            Ok(_) => {}
            Err(error) => failures.push(format!("{}: {error}", fixture.name)),
        }
    }
    assert!(
        failures.is_empty(),
        "{} cards still fail:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

use engine::ability::AbilityKind;
use engine::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith_runtime_catalog as engine;

#[test]
fn catalog_vizier_reduces_only_matching_counter_placements() {
    use engine::events::processing::process_put_counters_with_event;
    use engine::object::CounterType;
    use engine::types::CardType;
    let definition = fixture("Vizier of Remedies");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature =
        engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), "Creature")
            .card_types(vec![CardType::Creature])
            .build();
    let artifact =
        engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), "Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
    let own = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let opposing = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let noncreature = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
    game.update_replacement_effects();
    for (target, counter, count, expected) in [
        (own, CounterType::MinusOneMinusOne, 0, 0),
        (own, CounterType::MinusOneMinusOne, 1, 0),
        (own, CounterType::MinusOneMinusOne, 4, 3),
        (own, CounterType::PlusOnePlusOne, 4, 4),
        (opposing, CounterType::MinusOneMinusOne, 4, 4),
        (noncreature, CounterType::MinusOneMinusOne, 4, 4),
    ] {
        assert_eq!(
            process_put_counters_with_event(
                &mut game,
                target,
                counter,
                count,
                engine::events::cause::EventCause::effect()
            ),
            expected
        );
    }
    let entering_definition = engine::cards::builders::CardDefinitionBuilder::new(
        engine::ids::CardId::new(),
        "Enters with counters",
    )
    .card_types(vec![CardType::Creature])
    .with_ability(engine::ability::Ability::static_ability(
        engine::static_abilities::StaticAbility::enters_with_counters(
            CounterType::MinusOneMinusOne,
            3,
        ),
    ))
    .build();
    let entering = game.create_object_from_definition(&entering_definition, alice, Zone::Hand);
    game.update_replacement_effects();
    let mut dm = engine::decision::AutoPassDecisionMaker;
    let id = game
        .move_object_with_etb_processing_with_dm(entering, Zone::Battlefield, &mut dm)
        .unwrap();
    assert_eq!(
        game.object(id.new_id)
            .unwrap()
            .counters
            .get(&CounterType::MinusOneMinusOne)
            .copied(),
        Some(2)
    );
}

fn fixture(name: &str) -> CardDefinition {
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("catalog_regressions.json.fixture")).unwrap();
    let f = fixtures.iter().find(|f| f.name == name).unwrap();
    let artifact = compile_artifact(CompileInput {
        name: &f.name,
        text: &f.text,
        score: None,
        local_id: 1,
        other_face_id: None,
        other_face_name: None,
        layout: LinkedFaceLayout::None,
    });
    let artifact = artifact.unwrap();
    {
        let mut definition =
            engine::artifact_materializer::materialize_artifact(&artifact).unwrap();
        definition.card.id = engine::ids::CardId::new();
        definition
    }
}

#[test]
fn catalog_graveyard_permissions_allow_only_the_named_alternative() {
    use engine::alternative_cast::CastingMethod;
    use engine::decision::LegalAction;
    use engine::mana::ManaSymbol;
    for name in ["Detective's Phoenix", "Timeline Culler"] {
        let definition = fixture(name);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        game.turn.phase = engine::Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        let fodder = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Evidence",
        )
        .card_types(vec![engine::types::CardType::Creature])
        .mana_cost(engine::mana::ManaCost::from_symbols(vec![
            ManaSymbol::Generic(6),
        ]))
        .power_toughness(engine::card::PowerToughness::fixed(1, 1))
        .build();
        let evidence = game.create_object_from_definition(&fodder, alice, Zone::Graveyard);
        let host = game.create_object_from_definition(&fodder, alice, Zone::Battlefield);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 5);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 5);
        let actions = engine::decision::compute_legal_actions(&game, alice);
        let casts: Vec<_> = actions
            .iter()
            .filter_map(|action| match action {
                LegalAction::CastSpell {
                    spell_id,
                    from_zone,
                    casting_method,
                } if *spell_id == source => Some((*from_zone, casting_method)),
                _ => None,
            })
            .collect();
        assert_eq!(casts.len(), 1, "{name}: {actions:#?}");
        assert_eq!(casts[0].0, Zone::Graveyard);
        assert!(
            matches!(
                casts[0].1,
                CastingMethod::PlayFrom {
                    zone: Zone::Graveyard,
                    use_alternative: Some(0),
                    ..
                }
            ),
            "{casts:?}"
        );
        assert!(engine::decision::compute_legal_actions(&game, PlayerId::from_index(1)).iter().all(|action| !matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)));
        let mut insufficient = game.clone();
        insufficient.move_object_by_effect(evidence, Zone::Exile);
        insufficient.player_mut(alice).unwrap().life = 1;
        assert!(engine::decision::compute_legal_actions(&insufficient, alice).iter().all(|action| !matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)), "{name} cannot ignore the nonmana cost");
        let cast = actions.into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
        let mut dm = ObjectChoices::default();
        let mut state = engine::game_loop::PriorityLoopState::new(2);
        let mut triggers = engine::triggers::TriggerQueue::new();
        let mut progress = engine::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut triggers,
            &mut state,
            &engine::game_loop::PriorityResponse::PriorityAction(cast),
            &mut dm,
        )
        .unwrap();
        if name == "Detective's Phoenix" {
            assert!(
                matches!(
                    progress,
                    engine::decision::GameProgress::NeedsDecisionCtx(
                        engine::decisions::context::DecisionContext::Targets(_)
                    )
                ),
                "{progress:?}"
            );
            progress = engine::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut triggers,
                &mut state,
                &engine::game_loop::PriorityResponse::Targets(vec![
                    engine::game_state::Target::Object(host),
                ]),
                &mut dm,
            )
            .unwrap();
        }
        while let engine::decision::GameProgress::NeedsDecisionCtx(
            engine::decisions::context::DecisionContext::ManaPayment(ref payment),
        ) = progress
        {
            let response = engine::game_loop::PriorityResponse::ManaPaymentPlan(
                engine::mana_payment::ManaPaymentResponse::Confirm {
                    plan_id: payment.plan.id,
                    request_hash: payment.plan.request_hash,
                },
            );
            progress = engine::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut triggers,
                &mut state,
                &response,
                &mut dm,
            )
            .unwrap();
        }
        if name == "Detective's Phoenix" {
            assert_eq!(
                game.exile.len(),
                1,
                "collect evidence must exile the selected card"
            );
        } else {
            assert_eq!(
                game.player(alice).unwrap().life,
                18,
                "warp must pay two life"
            );
        }
        assert_eq!(game.stack.len(), 1);
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let permanent = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().name == name)
            .unwrap();
        if name == "Detective's Phoenix" {
            assert_eq!(
                game.object(permanent).unwrap().attached_to,
                Some(engine::object::AttachmentTarget::Object(host))
            );
            assert!(!game.object(permanent).unwrap().is_creature());
        } else {
            let event = engine::triggers::TriggerEvent::new_with_provenance(
                engine::events::phase::BeginningOfEndStepEvent::new(alice),
                engine::provenance::ProvNodeId::default(),
            );
            for trigger in engine::triggers::check_delayed_triggers(&mut game, &event) {
                triggers.add(trigger);
            }
            engine::game_loop::put_triggers_on_stack(&mut game, &mut triggers).unwrap();
            while !game.stack_is_empty() {
                engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            }
            assert!(!game.battlefield.contains(&permanent));
            assert!(
                game.exile
                    .iter()
                    .any(|id| game.object(*id).unwrap().name == name)
            );
        }
    }
}

#[test]
fn catalog_enduring_cards_return_as_enchantments() {
    for name in [
        "Enduring Curiosity",
        "Enduring Innocence",
        "Enduring Vitality",
    ] {
        let definition = fixture(name);
        let ability = definition
            .abilities
            .iter()
            .filter_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .last()
            .unwrap();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let snapshot =
            engine::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(),
                &game,
            );
        let stable = snapshot.stable_id;
        let graveyard = game.move_object_by_sba(source, Zone::Graveyard).unwrap();
        let event = engine::triggers::TriggerEvent::new_with_provenance(
            engine::events::zones::ZoneChangeEvent::with_results(
                source,
                vec![graveyard],
                Zone::Battlefield,
                Zone::Graveyard,
                engine::events::cause::EventCause::from_sba(),
                Some(snapshot.clone()),
            ),
            engine::provenance::ProvNodeId::default(),
        );
        let mut dm = engine::decision::AutoPassDecisionMaker;
        game.push_to_stack(
            engine::game_state::StackEntry::ability(graveyard, alice, ability.effects.clone())
                .with_source_snapshot(snapshot)
                .with_triggering_event(event),
        );
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(returned).unwrap().zone,
            Zone::Battlefield,
            "{name}"
        );
        assert_ne!(returned, graveyard);
        assert_eq!(
            game.current_card_types(returned),
            Some(vec![engine::types::CardType::Enchantment]),
            "{name}"
        );
    }
}

#[test]
fn catalog_bankbuster_pilot_crews_for_three_but_does_not_saddle_for_three() {
    use engine::effects::CostExecutableEffect;
    let definition = fixture("Reckoner Bankbuster");
    let ability = definition
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    // Resolve after the last charge counter has been paid.
    game.object_mut(source).unwrap().counters.clear();
    let filler = engine::cards::builders::CardDefinitionBuilder::new(
        engine::ids::CardId::new(),
        "Draw card",
    )
    .build();
    game.create_object_from_definition(&filler, alice, Zone::Library);
    let mut dm = engine::decision::AutoPassDecisionMaker;
    game.push_to_stack(engine::game_state::StackEntry::ability(
        source,
        alice,
        ability.effects.clone(),
    ));
    engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|id| game.object(**id).unwrap().name == "Treasure")
            .count(),
        1
    );
    let pilot = *game
        .battlefield
        .iter()
        .find(|id| game.object(**id).unwrap().name == "Pilot")
        .unwrap();
    assert_eq!(
        game.calculated_characteristics(pilot).unwrap().power,
        Some(1)
    );
    engine::effects::CrewCostEffect { required_power: 3 }
        .can_execute_as_cost(&game, source, alice)
        .unwrap();
    assert!(
        engine::effects::CrewCostEffect { required_power: 4 }
            .can_execute_as_cost(&game, source, alice)
            .is_err()
    );
    assert!(
        engine::effects::SaddleCostEffect::new(3)
            .can_execute_as_cost(&game, source, alice)
            .is_err()
    );
}

#[test]
fn catalog_proft_counts_only_cards_drawn_beyond_the_first() {
    let definition = fixture("Proft's Eidetic Memory");
    let ability = definition
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .last()
        .unwrap();
    for draws in [2, 4] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Target",
        )
        .card_types(vec![engine::types::CardType::Creature])
        .power_toughness(engine::card::PowerToughness::fixed(1, 1))
        .build();
        let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        for _ in 0..draws {
            game.create_object_from_definition(&creature, alice, Zone::Library);
        }
        let mut dm = engine::decision::AutoPassDecisionMaker;
        game.push_to_stack(engine::game_state::StackEntry::ability(
            source,
            alice,
            vec![engine::effect::Effect::draw(draws)],
        ));
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        game.push_to_stack(
            engine::game_state::StackEntry::ability(source, alice, ability.effects.clone())
                .with_targets(vec![engine::game_state::Target::Object(target)]),
        );
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            game.object(target)
                .unwrap()
                .counters
                .get(&engine::object::CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            (draws - 1) as u32
        );
    }
}

#[test]
fn catalog_past_in_flames_grants_only_the_current_graveyard_cards() {
    use engine::types::CardType;
    let definition = fixture("Past in Flames");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let card = |name: &str, kind| {
        engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), name)
            .card_types(vec![kind])
            .mana_cost(engine::mana::ManaCost::from_symbols(vec![
                engine::mana::ManaSymbol::Generic(2),
            ]))
            .build()
    };
    let instant = card("Instant", CardType::Instant);
    let sorcery = card("Sorcery", CardType::Sorcery);
    let creature = card("Creature", CardType::Creature);
    let eligible = [
        game.create_object_from_definition(&instant, alice, Zone::Graveyard),
        game.create_object_from_definition(&sorcery, alice, Zone::Graveyard),
    ];
    let ineligible = [
        game.create_object_from_definition(&instant, bob, Zone::Graveyard),
        game.create_object_from_definition(&creature, alice, Zone::Graveyard),
    ];
    let mut dm = engine::decision::AutoPassDecisionMaker;
    game.push_to_stack(engine::game_state::StackEntry::ability(
        source,
        alice,
        definition.spell_effect.clone().unwrap(),
    ));
    engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let late = game.create_object_from_definition(&instant, alice, Zone::Graveyard);
    for id in eligible {
        let grants = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, id, Zone::Graveyard, alice);
        assert_eq!(grants.len(), 1, "{id:?}: {grants:#?}");
        assert!(
            matches!(&grants[0].method, engine::alternative_cast::AlternativeCastingMethod::Flashback { total_cost } if total_cost.mana_cost() == instant.card.mana_cost.as_ref())
        );
    }
    for id in ineligible.into_iter().chain([late]) {
        for player in [alice, bob] {
            assert!(
                game.effect_store
                    .grant_registry
                    .granted_alternative_casts_for_card(&game, id, Zone::Graveyard, player)
                    .is_empty()
            );
        }
    }
}

#[test]
fn catalog_inside_information_requires_life_for_spells_and_preserves_land_permission() {
    let definition = fixture("Inside Information");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let make = |name: &str, kind| {
        engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), name)
            .card_types(vec![kind])
            .mana_cost(engine::mana::ManaCost::from_symbols(vec![
                engine::mana::ManaSymbol::Generic(3),
            ]))
            .build()
    };
    game.create_object_from_definition(
        &make("Stolen spell", engine::types::CardType::Sorcery),
        bob,
        Zone::Library,
    );
    game.create_object_from_definition(
        &make("Stolen land", engine::types::CardType::Land),
        bob,
        Zone::Library,
    );
    let mut dm = engine::decision::AutoPassDecisionMaker;
    game.push_to_stack(
        engine::game_state::StackEntry::ability(
            source,
            alice,
            definition.spell_effect.clone().unwrap(),
        )
        .with_x(2)
        .with_targets(vec![engine::game_state::Target::Player(bob)]),
    );
    engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(game.exile.len(), 2);
    for &id in &game.exile {
        let land = game.object(id).unwrap().is_land();
        let ordinary = game.effect_store.grant_registry.granted_play_from_for_card(
            &game,
            id,
            Zone::Exile,
            alice,
        );
        let alternative = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, id, Zone::Exile, alice);
        assert_eq!(
            ordinary.len(),
            usize::from(land),
            "ordinary permission for {:?}",
            game.object(id).unwrap().name
        );
        assert_eq!(alternative.len(), usize::from(!land));
        assert!(
            game.effect_store
                .grant_registry
                .granted_play_from_for_card(&game, id, Zone::Exile, bob)
                .is_empty()
        );
    }
}

#[derive(Default)]
struct ObjectChoices {
    players: Vec<PlayerId>,
}
impl engine::decision::DecisionMaker for ObjectChoices {
    fn decide_objects(
        &mut self,
        _: &GameState,
        context: &engine::decisions::context::SelectObjectsContext,
    ) -> Vec<engine::ObjectId> {
        self.players.push(context.player);
        context
            .candidates
            .iter()
            .take(context.min.max(1))
            .map(|candidate| candidate.id)
            .collect()
    }
}

#[test]
fn catalog_deadly_cover_up_requires_optional_evidence_for_the_search() {
    struct ChooseMaximum;
    impl engine::decision::DecisionMaker for ChooseMaximum {
        fn decide_objects(
            &mut self,
            _: &GameState,
            ctx: &engine::decisions::context::SelectObjectsContext,
        ) -> Vec<engine::ObjectId> {
            ctx.candidates
                .iter()
                .take(ctx.max.unwrap_or(ctx.candidates.len()))
                .map(|candidate| candidate.id)
                .collect()
        }
    }
    let definition = fixture("Deadly Cover-Up");
    assert_eq!(definition.optional_costs.len(), 1);
    for paid in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let victim = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Creature",
        )
        .card_types(vec![engine::types::CardType::Creature])
        .power_toughness(engine::card::PowerToughness::fixed(2, 2))
        .build();
        let named = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Chosen name",
        )
        .card_types(vec![engine::types::CardType::Artifact])
        .build();
        let filler = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Filler",
        )
        .build();
        game.create_object_from_definition(&named, bob, Zone::Graveyard);
        game.create_object_from_definition(&named, bob, Zone::Hand);
        game.create_object_from_definition(&named, bob, Zone::Library);
        for _ in 0..4 {
            game.create_object_from_definition(&filler, bob, Zone::Library);
        }
        game.create_object_from_definition(&victim, alice, Zone::Battlefield);
        game.create_object_from_definition(&victim, bob, Zone::Battlefield);
        let mut entry = engine::game_state::StackEntry::ability(
            source,
            alice,
            definition.spell_effect.clone().unwrap(),
        );
        if paid {
            entry.optional_costs_paid.mark_label_paid("Evidence");
        }
        game.push_to_stack(entry);
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut ChooseMaximum).unwrap();
        assert!(game.battlefield.is_empty());
        assert_eq!(game.exile.len(), if paid { 3 } else { 0 }, "paid={paid}");
        assert_eq!(
            game.player(bob).unwrap().hand.len(),
            1,
            "one replacement card for one exiled hand card, paid={paid}"
        );
    }
}

#[test]
fn catalog_thought_stalker_uses_the_target_players_life_history_and_correct_chooser() {
    let definition = fixture("Thought-Stalker Warlock");
    let ability = definition
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .unwrap();
    for lost_life in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for (name, kind) in [
            ("Land", engine::types::CardType::Land),
            ("Spell", engine::types::CardType::Instant),
            ("Second spell", engine::types::CardType::Instant),
        ] {
            let card = engine::cards::builders::CardDefinitionBuilder::new(
                engine::ids::CardId::new(),
                name,
            )
            .card_types(vec![kind])
            .build();
            game.create_object_from_definition(&card, bob, Zone::Hand);
        }
        let mut dm = ObjectChoices::default();
        // Losing life by the ability's controller must not satisfy the condition.
        game.push_to_stack(engine::game_state::StackEntry::ability(
            source,
            alice,
            vec![engine::effect::Effect::lose_life_player(
                1,
                engine::target::PlayerFilter::Specific(if lost_life { bob } else { alice }),
            )],
        ));
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let snapshot =
            engine::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let event = engine::triggers::TriggerEvent::new_with_provenance(
            engine::events::zones::ZoneChangeEvent::with_results(
                source,
                vec![source],
                Zone::Hand,
                Zone::Battlefield,
                engine::events::cause::EventCause::from_sba(),
                Some(snapshot),
            ),
            engine::provenance::ProvNodeId::default(),
        );
        game.push_to_stack(
            engine::game_state::StackEntry::ability(source, alice, ability.effects.clone())
                .with_targets(vec![engine::game_state::Target::Player(bob)])
                .with_triggering_event(event),
        );
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.players[1].hand.len(), 2);
        assert_eq!(
            game.object(game.players[1].hand[0]).unwrap().name,
            if lost_life { "Land" } else { "Spell" }
        );
        assert_eq!(dm.players, vec![if lost_life { alice } else { bob }]);
    }
}

#[test]
fn catalog_dragonfire_blade_counts_target_colors_and_blocks_only_monocolored_opponents() {
    use engine::mana::{ManaCost, ManaSymbol};
    use engine::types::CardType;
    let definition = fixture("Dragonfire Blade");
    let ability = definition
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(t) => Some(t),
            _ => None,
        })
        .unwrap();
    for symbols in [
        vec![],
        vec![ManaSymbol::White],
        vec![ManaSymbol::White, ManaSymbol::Blue],
        vec![
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
        ],
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Equipped",
        )
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(symbols.clone()))
        .power_toughness(engine::card::PowerToughness::fixed(1, 1))
        .build();
        let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let cost = engine::decision::calculate_effective_activation_total_cost_with_chosen_targets(
            &game,
            alice,
            source,
            &ability.mana_cost,
            &[engine::game_state::Target::Object(target)],
        );
        assert_eq!(
            cost.mana_cost().map_or(0, |m| m.mana_value()),
            4u32.saturating_sub(symbols.len() as u32)
        );
        assert!(
            game.attach_object_to_target(source, engine::object::AttachmentTarget::Object(target))
        );
        assert_eq!(
            game.calculated_characteristics(target).unwrap().power,
            Some(3)
        );
        for source_symbols in [
            vec![],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red, ManaSymbol::Blue],
        ] {
            let spell = engine::cards::builders::CardDefinitionBuilder::new(
                engine::ids::CardId::new(),
                "Targeting spell",
            )
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(source_symbols.clone()))
            .build();
            let their_spell = game.create_object_from_definition(&spell, bob, Zone::Stack);
            let own_spell = game.create_object_from_definition(&spell, alice, Zone::Stack);
            assert_eq!(
                engine::targeting::can_target_object(&game, target, their_spell, bob).is_legal(),
                source_symbols.len() != 1
            );
            assert!(
                engine::targeting::can_target_object(&game, target, own_spell, alice).is_legal()
            );
        }
    }
}

#[test]
fn catalog_invoke_despair_repeats_each_sacrifice_with_independent_failure_results() {
    use engine::types::CardType;
    let definition = fixture("Invoke Despair");
    for mask in 0u32..8 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        for (index, kind) in [
            CardType::Creature,
            CardType::Enchantment,
            CardType::Planeswalker,
        ]
        .into_iter()
        .enumerate()
        {
            let card = engine::cards::builders::CardDefinitionBuilder::new(
                engine::ids::CardId::new(),
                &format!("Permanent {index}"),
            )
            .card_types(vec![kind])
            .power_toughness(engine::card::PowerToughness::fixed(1, 1))
            .loyalty(3)
            .build();
            if mask & (1 << index) != 0 {
                game.create_object_from_definition(&card, bob, Zone::Battlefield);
            }
            game.create_object_from_definition(&card, alice, Zone::Library);
        }
        let mut dm = engine::decision::AutoPassDecisionMaker;
        game.push_to_stack(
            engine::game_state::StackEntry::ability(
                source,
                alice,
                definition.spell_effect.clone().unwrap(),
            )
            .with_targets(vec![engine::game_state::Target::Player(bob)]),
        );
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let failures = 3 - mask.count_ones();
        assert_eq!(
            game.players[1].life,
            20 - failures as i32 * 2,
            "mask {mask}"
        );
        assert_eq!(game.players[0].hand.len(), failures as usize, "mask {mask}");
        assert_eq!(
            game.players[1].graveyard.len(),
            mask.count_ones() as usize,
            "mask {mask}"
        );
    }
}

#[test]
fn catalog_accumulate_wisdom_takes_all_three_only_with_three_lessons() {
    let definition = fixture("Accumulate Wisdom");
    for lessons in [0, 2, 3, 4] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let lesson = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Lesson",
        )
        .card_types(vec![engine::types::CardType::Sorcery])
        .subtypes(vec![engine::types::Subtype::Lesson])
        .build();
        for _ in 0..lessons {
            game.create_object_from_definition(&lesson, alice, Zone::Graveyard);
        }
        for _ in 0..5 {
            game.create_object_from_definition(&lesson, alice, Zone::Library);
        }
        let mut dm = ObjectChoices::default();
        game.push_to_stack(engine::game_state::StackEntry::ability(
            source,
            alice,
            definition.spell_effect.clone().unwrap(),
        ));
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let drawn = if lessons >= 3 { 3 } else { 1 };
        assert_eq!(game.players[0].hand.len(), drawn, "{lessons} lessons");
        assert_eq!(game.players[0].library.len(), 5 - drawn);
    }
}

#[test]
fn catalog_urgent_necropsy_locks_evidence_to_announced_targets() {
    use engine::game_loop::PriorityResponse;
    use engine::mana::{ManaCost, ManaSymbol};
    use engine::types::CardType;
    struct EvidenceChoices {
        minimum: Option<i32>,
    }
    impl engine::decision::DecisionMaker for EvidenceChoices {
        fn decide_objects(
            &mut self,
            _: &GameState,
            ctx: &engine::decisions::context::SelectObjectsContext,
        ) -> Vec<engine::ObjectId> {
            if let Some(constraint) = &ctx.aggregate_constraint {
                let Some(engine::effect::Value::Fixed(amount)) = &constraint.minimum else {
                    panic!("unfrozen cost: {constraint:?}")
                };
                self.minimum = Some(*amount);
                return ctx
                    .candidates
                    .iter()
                    .take(usize::from(*amount > 0))
                    .map(|c| c.id)
                    .collect();
            }
            ctx.candidates.iter().take(ctx.min).map(|c| c.id).collect()
        }
    }
    for target_count in [0, 2] {
        let definition = fixture("Urgent Necropsy");
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.phase = engine::Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let mut victims = Vec::new();
        for (kind, mana_value) in [(CardType::Artifact, 2), (CardType::Creature, 3)] {
            let definition = engine::cards::builders::CardDefinitionBuilder::new(
                engine::ids::CardId::new(),
                "Target",
            )
            .card_types(vec![kind])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
                mana_value,
            )]))
            .power_toughness(engine::card::PowerToughness::fixed(2, 2))
            .build();
            victims.push(game.create_object_from_definition(&definition, bob, Zone::Battlefield));
        }
        let fodder = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Evidence",
        )
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(5)]))
        .build();
        game.create_object_from_definition(&fodder, alice, Zone::Graveyard);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 3);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        let mut dm = EvidenceChoices { minimum: None };
        let mut state = engine::game_loop::PriorityLoopState::new(2);
        let mut triggers = engine::triggers::TriggerQueue::new();
        let mut progress = engine::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut triggers,
            &mut state,
            &PriorityResponse::PriorityAction(engine::decision::LegalAction::CastSpell {
                spell_id: source,
                from_zone: Zone::Hand,
                casting_method: engine::alternative_cast::CastingMethod::Normal,
            }),
            &mut dm,
        )
        .unwrap();
        assert!(
            matches!(
                progress,
                engine::decision::GameProgress::NeedsDecisionCtx(
                    engine::decisions::context::DecisionContext::Targets(_)
                )
            ),
            "{progress:?}"
        );
        progress = engine::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut triggers,
            &mut state,
            &PriorityResponse::Targets(
                victims
                    .iter()
                    .take(target_count)
                    .map(|id| engine::game_state::Target::Object(*id))
                    .collect(),
            ),
            &mut dm,
        )
        .unwrap();
        // Changing a target after costs have been locked must not change evidence.
        game.object_mut(victims[0]).unwrap().mana_cost =
            Some(ManaCost::from_symbols(vec![ManaSymbol::Generic(10)]).into());
        while let engine::decision::GameProgress::NeedsDecisionCtx(
            engine::decisions::context::DecisionContext::ManaPayment(ref payment),
        ) = progress
        {
            let response = PriorityResponse::ManaPaymentPlan(
                engine::mana_payment::ManaPaymentResponse::Confirm {
                    plan_id: payment.plan.id,
                    request_hash: payment.plan.request_hash,
                },
            );
            progress = engine::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut triggers,
                &mut state,
                &response,
                &mut dm,
            )
            .unwrap();
        }
        assert_eq!(game.stack.len(), 1, "{progress:?}");
        if target_count > 0 {
            assert_eq!(dm.minimum, Some(5));
        }
        assert_eq!(game.exile.len(), usize::from(target_count > 0));
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.battlefield.len(), 2 - target_count);
    }
}

#[test]
fn catalog_braided_net_lock_ends_permanently_when_the_target_untaps() {
    let definition = fixture("Braided Net");
    let ability = definition
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let victim =
        engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), "Artifact")
            .card_types(vec![engine::types::CardType::Artifact])
            .build();
    let target = game.create_object_from_definition(&victim, bob, Zone::Battlefield);
    game.tap(source);
    game.push_to_stack(
        engine::game_state::StackEntry::ability(source, alice, ability.effects.clone())
            .with_targets(vec![engine::game_state::Target::Object(target)]),
    );
    engine::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut engine::decision::AutoPassDecisionMaker,
    )
    .unwrap();
    assert!(game.is_tapped(target));
    assert!(
        !game
            .effect_store
            .cant_effects
            .can_activate_abilities_of(target)
    );
    game.untap(source);
    assert!(
        !game
            .effect_store
            .cant_effects
            .can_activate_abilities_of(target)
    );
    game.move_object_by_effect(source, Zone::Graveyard);
    game.update_cant_effects();
    assert!(
        !game
            .effect_store
            .cant_effects
            .can_activate_abilities_of(target)
    );
    game.untap(target);
    assert!(
        game.effect_store
            .cant_effects
            .can_activate_abilities_of(target)
    );
    game.tap(target);
    game.update_cant_effects();
    assert!(
        game.effect_store
            .cant_effects
            .can_activate_abilities_of(target)
    );
}

#[test]
fn catalog_ashiok_only_blocks_opponents_searching_their_own_library_from_their_own_effect() {
    use engine::effects::EffectExecutor;
    use engine::target::{ObjectFilter, PlayerFilter};
    for (controller, searcher, owner, blocked) in [
        (1, 1, 1, true),
        (0, 1, 1, false),
        (1, 1, 0, false),
        (0, 0, 0, false),
        (1, 0, 0, false),
    ] {
        for generic_choice in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let controller = PlayerId::from_index(controller);
            let searcher = PlayerId::from_index(searcher);
            let owner = PlayerId::from_index(owner);
            game.create_object_from_definition(
                &fixture("Ashiok, Dream Render"),
                alice,
                Zone::Battlefield,
            );
            let card = engine::cards::builders::CardDefinitionBuilder::new(
                engine::ids::CardId::new(),
                "Find me",
            )
            .build();
            game.create_object_from_definition(&card, owner, Zone::Library);
            let source = game.create_object_from_definition(&card, controller, Zone::Stack);
            game.update_cant_effects();
            let mut dm = ObjectChoices::default();
            let mut ctx = engine::effects::EffectContext::new(source, controller, &mut dm);
            let outcome = if generic_choice {
                engine::effects::ChooseObjectsEffect::new(
                    ObjectFilter::default()
                        .owned_by(PlayerFilter::Specific(owner))
                        .in_zone(Zone::Library),
                    1,
                    PlayerFilter::Specific(searcher),
                    "found",
                )
                .as_search()
                .execute(&mut game, &mut ctx)
                .unwrap()
            } else {
                engine::effects::SearchLibraryEffect::new(
                    ObjectFilter::default(),
                    Zone::Hand,
                    PlayerFilter::Specific(searcher),
                    PlayerFilter::Specific(owner),
                    false,
                )
                .execute(&mut game, &mut ctx)
                .unwrap()
            };
            assert_eq!(
                matches!(outcome.value, engine::effect::OutcomeValue::Objects(ref ids) if !ids.is_empty()),
                !blocked,
                "controller={controller:?} searcher={searcher:?} owner={owner:?} generic={generic_choice}, {outcome:?}"
            );
            if !generic_choice {
                assert_eq!(
                    game.player(owner).unwrap().hand.len(),
                    usize::from(!blocked)
                );
            }
        }
    }
}

#[test]
fn catalog_wish_allows_one_normal_play_and_independent_resolutions_stack() {
    use engine::decision::LegalAction;
    use engine::game_loop::PriorityResponse;
    for (copies, play_land) in [(1, false), (1, true), (2, false)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.phase = engine::Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        let artifact = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Free Artifact",
        )
        .card_types(vec![engine::types::CardType::Artifact])
        .mana_cost(engine::mana::ManaCost::from_symbols(vec![
            engine::mana::ManaSymbol::Generic(0),
        ]))
        .build();
        let land =
            engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), "Land")
                .card_types(vec![engine::types::CardType::Land])
                .build();
        let first = game.create_object_from_definition(
            if play_land { &land } else { &artifact },
            alice,
            Zone::OutsideGame,
        );
        let second = game.create_object_from_definition(&artifact, alice, Zone::OutsideGame);
        let opposing = game.create_object_from_definition(&artifact, bob, Zone::OutsideGame);
        let definition = fixture("Wish");
        let mut dm = engine::decision::AutoPassDecisionMaker;
        for _ in 0..copies {
            let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
            game.push_to_stack(engine::game_state::StackEntry::ability(
                source,
                alice,
                definition.spell_effect.clone().unwrap(),
            ));
            engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        }
        let identifies = |action: &LegalAction, id| match action {
            LegalAction::CastSpell { spell_id, .. } => *spell_id == id,
            LegalAction::PlayLand { land_id } => *land_id == id,
            _ => false,
        };
        let actions = engine::decision::compute_legal_actions(&game, alice);
        assert!(actions.iter().any(|a| identifies(a, first)), "{actions:?}");
        assert!(actions.iter().any(|a| identifies(a, second)));
        assert!(!actions.iter().any(|a| identifies(a, opposing)));
        let mut expired = game.clone();
        expired.turn.turn_number += 1;
        assert!(
            !engine::decision::compute_legal_actions(&expired, alice)
                .iter()
                .any(|a| identifies(a, first))
        );
        let mut wrong_timing = game.clone();
        wrong_timing.turn.active_player = bob;
        assert!(
            !engine::decision::compute_legal_actions(&wrong_timing, alice)
                .iter()
                .any(|a| identifies(a, first))
        );
        let action = actions.into_iter().find(|a| identifies(a, first)).unwrap();
        let mut state = engine::game_loop::PriorityLoopState::new(2);
        let mut triggers = engine::triggers::TriggerQueue::new();
        engine::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut triggers,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        while !game.stack_is_empty() {
            engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        }
        assert_eq!(
            engine::decision::compute_legal_actions(&game, alice)
                .iter()
                .any(|a| identifies(a, second)),
            copies == 2
        );
    }
}

#[test]
fn catalog_bookworm_discards_only_after_accepting_draw_without_either_exception() {
    struct DrawChoice(bool);
    impl engine::decision::DecisionMaker for DrawChoice {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &engine::decisions::context::BooleanContext,
        ) -> bool {
            self.0
        }
    }
    for accept in [false, true] {
        for history in [
            "none",
            "face-down entry",
            "face-up entry",
            "opponent entry",
            "turned up",
            "opponent turned up",
            "previous turn",
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let definition = fixture("Oblivious Bookworm");
            let ability = definition
                .abilities
                .iter()
                .find_map(|a| match &a.kind {
                    AbilityKind::Triggered(a) => Some(a),
                    _ => None,
                })
                .unwrap();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let card = engine::cards::builders::CardDefinitionBuilder::new(
                engine::ids::CardId::new(),
                "Card",
            )
            .card_types(vec![engine::types::CardType::Creature])
            .build();
            for _ in 0..3 {
                game.create_object_from_definition(&card, alice, Zone::Library);
            }
            game.create_object_from_definition(&card, alice, Zone::Hand);
            if history != "none" {
                let owner = if history.starts_with("opponent") {
                    bob
                } else {
                    alice
                };
                let permanent = game.create_object_from_definition(&card, owner, Zone::Battlefield);
                game.set_face_down(permanent);
                let mut snapshot = engine::snapshot::ObjectSnapshot::from_object(
                    game.object(permanent).unwrap(),
                    &game,
                );
                snapshot.face_down = history != "face-up entry";
                let event = if history.ends_with("entry") {
                    engine::triggers::TriggerEvent::new_with_provenance(
                        engine::events::zones::ZoneChangeEvent::with_results(
                            permanent,
                            vec![permanent],
                            Zone::Hand,
                            Zone::Battlefield,
                            engine::events::cause::EventCause::effect(),
                            Some(snapshot.clone()),
                        ),
                        engine::provenance::ProvNodeId::default(),
                    )
                } else {
                    engine::triggers::TriggerEvent::new_with_provenance(
                        engine::events::TurnedFaceUpEvent::new(permanent, owner),
                        engine::provenance::ProvNodeId::default(),
                    )
                };
                game.turn_store
                    .turn_history
                    .record_event(&event, Some(snapshot), None);
                game.move_object_by_effect(permanent, Zone::Graveyard);
                if history == "previous turn" {
                    game.turn_store.turn_history.clear_for_new_turn();
                }
            }
            game.push_to_stack(engine::game_state::StackEntry::ability(
                source,
                alice,
                ability.effects.clone(),
            ));
            engine::game_loop::resolve_stack_entry_with(&mut game, &mut DrawChoice(accept))
                .unwrap();
            let exception = matches!(history, "face-down entry" | "turned up");
            assert_eq!(
                game.player(alice).unwrap().hand.len(),
                1 + usize::from(accept && exception),
                "{history}, accept={accept}"
            );
            assert_eq!(
                game.player(alice).unwrap().library.len(),
                3 - usize::from(accept),
                "{history}, accept={accept}"
            );
        }
    }
}

#[test]
fn catalog_infinity_abilities_exist_only_while_harnessed_and_reset_on_zone_change() {
    for name in ["The Mind Stone", "The Soul Stone"] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let definition = fixture(name);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let other = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let card = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Creature",
        )
        .card_types(vec![engine::types::CardType::Creature])
        .power_toughness(engine::card::PowerToughness::fixed(2, 2))
        .build();
        let target = game.create_object_from_definition(
            &card,
            alice,
            if name == "The Mind Stone" {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            },
        );
        let target_stable = game.object(target).unwrap().stable_id;
        let event_for = |player| {
            if name == "The Mind Stone" {
                engine::triggers::TriggerEvent::new_with_provenance(
                    engine::events::phase::BeginningOfEndStepEvent::new(player),
                    engine::provenance::ProvNodeId::default(),
                )
            } else {
                engine::triggers::TriggerEvent::new_with_provenance(
                    engine::events::phase::BeginningOfUpkeepEvent::new(player),
                    engine::provenance::ProvNodeId::default(),
                )
            }
        };
        assert!(engine::triggers::check_triggers(&game, &event_for(alice)).is_empty());
        let harness = definition
            .abilities
            .iter()
            .filter_map(|a| match &a.kind {
                AbilityKind::Activated(a) => Some(a),
                _ => None,
            })
            .last()
            .unwrap();
        game.push_to_stack(engine::game_state::StackEntry::ability(
            source,
            alice,
            harness.effects.clone(),
        ));
        let mut dm = engine::decision::AutoPassDecisionMaker;
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.is_harnessed(source));
        assert!(!game.is_harnessed(other));
        assert!(!game.harness(source));
        assert!(
            engine::triggers::check_triggers(&game, &event_for(PlayerId::from_index(1))).is_empty()
        );
        let triggers = engine::triggers::check_triggers(&game, &event_for(alice));
        assert_eq!(triggers.len(), 1, "{name}: {triggers:?}");
        let triggered = game
            .current_abilities(source)
            .unwrap()
            .into_iter()
            .find_map(|a| match a.kind {
                AbilityKind::Triggered(a) => Some(a),
                _ => None,
            })
            .unwrap();
        let source_snapshot =
            engine::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(),
                &game,
            );
        game.push_to_stack(
            engine::game_state::StackEntry::ability(source, alice, triggered.effects)
                .with_target_assignments(vec![engine::game_state::TargetAssignment {
                    spec: triggered.choices[0].clone(),
                    range: 0..1,
                }])
                .with_targets(vec![engine::game_state::Target::Object(target)])
                .with_source_snapshot(source_snapshot),
        );
        let exiled = game.move_object_by_effect(source, Zone::Exile).unwrap();
        assert!(!game.is_harnessed(source));
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let returned = game.find_object_by_stable_id(target_stable).unwrap();
        assert_eq!(
            game.object(returned).unwrap().zone,
            Zone::Battlefield,
            "{name}"
        );
        assert_ne!(
            returned,
            target,
            "{name}: exile={:?} current={:?}",
            game.exile,
            game.current_abilities(other)
        );
        let stone = game
            .move_object_by_effect(exiled, Zone::Battlefield)
            .unwrap();
        assert!(!game.is_harnessed(stone));
        assert!(engine::triggers::check_triggers(&game, &event_for(alice)).is_empty());
    }
}

#[test]
fn catalog_replacement_reflexive_trigger_waits_for_the_stack() {
    for opponent in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(
            &fixture("Head of the Hunt"),
            alice,
            Zone::Battlefield,
        );
        let creature = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Victim",
        )
        .card_types(vec![engine::types::CardType::Creature])
        .power_toughness(engine::card::PowerToughness::fixed(2, 2))
        .build();
        let victim = game.create_object_from_definition(
            &creature,
            if opponent { bob } else { alice },
            Zone::Battlefield,
        );
        let stable = game.object(victim).unwrap().stable_id;
        game.update_replacement_effects();
        let mut dm = engine::decision::AutoPassDecisionMaker;
        engine::effects::execute_effect(
            &mut game,
            &engine::effect::Effect::destroy(engine::target::ChooseSpec::SpecificObject(victim)),
            &mut engine::effects::EffectContext::new(source, alice, &mut dm),
        )
        .unwrap();
        let moved = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(moved).unwrap().zone,
            if opponent {
                Zone::Exile
            } else {
                Zone::Graveyard
            }
        );
        assert_eq!(
            game.battlefield.len(),
            1,
            "token creation must wait for the reflexive trigger"
        );
        assert_eq!(game.stack.len(), usize::from(opponent));
        if opponent {
            game.move_object_by_effect(source, Zone::Graveyard);
            engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.battlefield.len(), 1);
            let wolf = game.object(game.battlefield[0]).unwrap();
            assert!(matches!(wolf.kind, engine::object::ObjectKind::Token));
            assert_eq!(game.controller_of(wolf), alice);
            assert_eq!(game.calculated_power(wolf.id), Some(2));
        }
    }
}

#[test]
fn catalog_escape_counts_distinct_card_types_across_the_selected_set() {
    use engine::decision::LegalAction;
    use engine::types::CardType::*;
    for types in [
        vec![vec![Artifact, Creature], vec![Artifact, Enchantment]],
        vec![
            vec![Artifact, Creature],
            vec![Artifact, Enchantment],
            vec![Kindred],
        ],
        vec![vec![Artifact], vec![Enchantment], vec![Instant]],
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        game.turn.phase = engine::Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        let source =
            game.create_object_from_definition(&fixture("Nethergoyf"), alice, Zone::Graveyard);
        for card_types in &types {
            let card = engine::cards::builders::CardDefinitionBuilder::new(
                engine::ids::CardId::new(),
                "Fodder",
            )
            .card_types(card_types.clone())
            .build();
            game.create_object_from_definition(&card, alice, Zone::Graveyard);
        }
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(engine::mana::ManaSymbol::Black, 3);
        let legal = types
            .iter()
            .flatten()
            .collect::<std::collections::HashSet<_>>()
            .len()
            >= 4;
        let cast = engine::decision::compute_legal_actions(&game, alice).into_iter()
            .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source));
        assert_eq!(cast.is_some(), legal, "types: {types:?}");
        if let Some(cast) = cast {
            let mut dm = ObjectChoices::default();
            let mut state = engine::game_loop::PriorityLoopState::new(2);
            let mut triggers = engine::triggers::TriggerQueue::new();
            let mut progress = engine::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut triggers,
                &mut state,
                &engine::game_loop::PriorityResponse::PriorityAction(cast),
                &mut dm,
            )
            .unwrap();
            while let engine::decision::GameProgress::NeedsDecisionCtx(
                engine::decisions::context::DecisionContext::ManaPayment(ref payment),
            ) = progress
            {
                let response = engine::game_loop::PriorityResponse::ManaPaymentPlan(
                    engine::mana_payment::ManaPaymentResponse::Confirm {
                        plan_id: payment.plan.id,
                        request_hash: payment.plan.request_hash,
                    },
                );
                progress = engine::game_loop::apply_priority_response_with_dm(
                    &mut game,
                    &mut triggers,
                    &mut state,
                    &response,
                    &mut dm,
                )
                .unwrap();
            }
            assert_eq!(game.exile.len(), 3);
            assert_eq!(game.stack.len(), 1);
            engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(
                game.battlefield
                    .iter()
                    .any(|id| game.object(*id).unwrap().name == "Nethergoyf")
            );
        }
    }
}

#[test]
fn catalog_forage_permission_pays_its_cost_and_adds_finality_only_when_used() {
    use engine::decision::LegalAction;
    use engine::types::{CardType, Subtype};
    for food in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        game.turn.phase = engine::Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        let adept = fixture("Osteomancer Adept");
        let source = game.create_object_from_definition(&adept, alice, Zone::Battlefield);
        let activation = adept
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Activated(a) => Some(a),
                _ => None,
            })
            .unwrap();
        game.push_to_stack(engine::game_state::StackEntry::ability(
            source,
            alice,
            activation.effects.clone(),
        ));
        let mut dm = ObjectChoices::default();
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let creature = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Creature",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(engine::card::PowerToughness::fixed(2, 2))
        .mana_cost(engine::mana::ManaCost::from_symbols(vec![
            engine::mana::ManaSymbol::Black,
        ]))
        .build();
        let spell = game.create_object_from_definition(&creature, alice, Zone::Graveyard);
        let stable = game.object(spell).unwrap().stable_id;
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(engine::mana::ManaSymbol::Black, 3);
        let cast_for = |game: &GameState| {
            engine::decision::compute_legal_actions(game, alice).into_iter()
            .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        };
        assert!(cast_for(&game).is_none(), "forage cannot be free");
        if food {
            let snack = engine::cards::builders::CardDefinitionBuilder::new(
                engine::ids::CardId::new(),
                "Food",
            )
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Food])
            .build();
            game.create_object_from_definition(&snack, alice, Zone::Battlefield);
        } else {
            for _ in 0..2 {
                game.create_object_from_definition(&creature, alice, Zone::Graveyard);
            }
            assert!(
                cast_for(&game).is_none(),
                "the spell itself cannot pay for its own forage cost"
            );
            game.create_object_from_definition(&creature, alice, Zone::Graveyard);
        }
        assert!(
            cast_for(&game).is_some(),
            "before leaving: food={food} grants={:#?}",
            game.effect_store
                .grant_registry
                .granted_alternative_casts_for_card(&game, spell, Zone::Graveyard, alice)
        );
        game.move_object_by_effect(source, Zone::Exile);
        let cast = cast_for(&game).expect("permission survives the source leaving");
        let mut state = engine::game_loop::PriorityLoopState::new(2);
        let mut triggers = engine::triggers::TriggerQueue::new();
        let mut progress = engine::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut triggers,
            &mut state,
            &engine::game_loop::PriorityResponse::PriorityAction(cast),
            &mut dm,
        )
        .unwrap();
        while let engine::decision::GameProgress::NeedsDecisionCtx(
            engine::decisions::context::DecisionContext::ManaPayment(ref payment),
        ) = progress
        {
            let response = engine::game_loop::PriorityResponse::ManaPaymentPlan(
                engine::mana_payment::ManaPaymentResponse::Confirm {
                    plan_id: payment.plan.id,
                    request_hash: payment.plan.request_hash,
                },
            );
            progress = engine::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut triggers,
                &mut state,
                &response,
                &mut dm,
            )
            .unwrap();
        }
        assert_eq!(game.stack.len(), 1, "{progress:?}");
        assert_eq!(game.exile.len(), if food { 1 } else { 4 });
        assert_eq!(game.battlefield.len(), 0, "Food must have been sacrificed");
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.object(returned)
                .unwrap()
                .counters
                .get(&engine::object::CounterType::Finality),
            Some(&1)
        );
        engine::effects::execute_effect(
            &mut game,
            &engine::effect::Effect::destroy(engine::target::ChooseSpec::SpecificObject(returned)),
            &mut engine::effects::EffectContext::new(returned, alice, &mut dm),
        )
        .unwrap();
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn catalog_coin_batch_skips_one_turn_per_head() {
    use engine::effects::EffectExecutor;
    let definition = fixture("Ral Zarek, Guest Lecturer");
    let ultimate = definition
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .last()
        .unwrap();
    for seed in 1..=20 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.set_random_seed(seed);
        let mut prediction = game.clone();
        let mut flips =
            engine::effects::FlipCoinEffect::face_only(engine::target::PlayerFilter::You);
        flips.count = 5;
        let expected = flips
            .execute(
                &mut prediction,
                &mut engine::effects::EffectContext::new_default(source, alice),
            )
            .unwrap()
            .as_count()
            .unwrap() as u32;
        game.push_to_stack(
            engine::game_state::StackEntry::ability(source, alice, ultimate.effects.clone())
                .with_targets(vec![engine::game_state::Target::Player(bob)]),
        );
        let mut dm = engine::decision::AutoPassDecisionMaker;
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let count = game.turn_store.skip_next_turn.pending(bob);
        assert_eq!(count, expected, "seed={seed}");
        assert_eq!(game.turn_store.skip_next_turn.pending(alice), 0);
        for _ in 0..expected {
            game.next_turn();
            assert_eq!(game.turn.active_player, alice);
        }
        game.next_turn();
        assert_eq!(game.turn.active_player, bob);
    }
    for (face, expected) in [(engine::CoinFace::Heads, 5), (engine::CoinFace::Tails, 0)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut flip =
            engine::effects::FlipCoinEffect::face_only(engine::target::PlayerFilter::You)
                .with_forced_face(face);
        flip.count = 5;
        let outcome = flip
            .execute(
                &mut game,
                &mut engine::effects::EffectContext::new_default(
                    engine::ObjectId::from_raw(999),
                    alice,
                ),
            )
            .unwrap();
        assert_eq!(outcome.as_count(), Some(expected));
        assert_eq!(outcome.events.len(), 5);
        for _ in 0..expected {
            engine::effects::SkipTurnEffect::opponent()
                .execute(
                    &mut game,
                    &mut engine::effects::EffectContext::new_default(
                        engine::ObjectId::from_raw(999),
                        alice,
                    ),
                )
                .unwrap();
        }
        for remaining in (0..expected).rev() {
            assert!(game.turn_store.skip_next_turn.remove(&bob));
            assert_eq!(
                game.turn_store.skip_next_turn.pending(bob),
                remaining as u32
            );
        }
        assert!(!game.turn_store.skip_next_turn.remove(&bob));
    }
}

#[test]
fn catalog_paradigm_recurs_once_per_name_and_survives_the_card_leaving_exile() {
    struct CopyChoice(bool);
    impl engine::decision::DecisionMaker for CopyChoice {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &engine::decisions::context::BooleanContext,
        ) -> bool {
            self.0
        }
    }
    for name in ["Decorum Dissertation", "Improvisation Capstone"] {
        let definition = fixture(name);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.phase = engine::Phase::FirstMain;
        let filler = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Uncastable land",
        )
        .card_types(vec![engine::types::CardType::Land])
        .mana_cost(engine::mana::ManaCost::from_symbols(vec![
            engine::mana::ManaSymbol::Generic(4),
        ]))
        .build();
        for player in [alice, bob] {
            for _ in 0..20 {
                game.create_object_from_definition(&filler, player, Zone::Library);
            }
        }
        for resolution in 0..2 {
            let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let stable = game.object(spell).unwrap().stable_id;
            let mut entry = engine::game_state::StackEntry::new(spell, alice);
            if name == "Decorum Dissertation" {
                entry = entry.with_targets(vec![engine::game_state::Target::Player(bob)]);
            }
            game.push_to_stack(entry);
            engine::game_loop::resolve_stack_entry_with(&mut game, &mut CopyChoice(false)).unwrap();
            let exiled = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile, "{name}");
            assert_eq!(
                game.effect_store.delayed_triggers.len(),
                1,
                "resolution {resolution}"
            );
            game.move_object_by_effect(exiled, Zone::Hand);
        }
        for accept in [false, true] {
            let other_event = engine::triggers::TriggerEvent::new_with_provenance(
                engine::events::phase::BeginningOfPrecombatMainPhaseEvent::new(bob),
                engine::provenance::ProvNodeId::default(),
            );
            assert!(engine::triggers::check_delayed_triggers(&mut game, &other_event).is_empty());
            let event = engine::triggers::TriggerEvent::new_with_provenance(
                engine::events::phase::BeginningOfPrecombatMainPhaseEvent::new(alice),
                engine::provenance::ProvNodeId::default(),
            );
            let delayed = engine::triggers::check_delayed_triggers(&mut game, &event);
            assert_eq!(delayed.len(), 1, "{name}");
            let trigger = &delayed[0];
            game.push_to_stack(engine::game_state::StackEntry::ability(
                trigger.source,
                trigger.controller,
                trigger.ability.effects.clone(),
            ));
            engine::game_loop::resolve_stack_entry_with(&mut game, &mut CopyChoice(accept))
                .unwrap();
            assert_eq!(
                game.stack.len(),
                usize::from(accept),
                "{name}: accept={accept}"
            );
            if accept {
                let copy = &game.stack[0];
                assert_eq!(game.object(copy.object_id).unwrap().name, name);
                engine::game_loop::resolve_stack_entry_with(&mut game, &mut CopyChoice(false))
                    .unwrap();
            }
            assert_eq!(game.effect_store.delayed_triggers.len(), 1);
            game.turn.turn_number += 2;
        }
    }
}

#[test]
#[ignore = "requires CATALOG_AUDIT_INPUT containing the current catalog card source blocks"]
fn catalog_full_current_sources_strict_audit() {
    let input = std::env::var("CATALOG_AUDIT_INPUT").expect("catalog audit input path");
    let input: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(input).unwrap()).unwrap();
    let cards = input["cards"].as_array().unwrap();
    let mut results = Vec::new();
    let mut failures = Vec::new();
    for card in cards {
        let name = card["name"].as_str().unwrap();
        let mut errors = Vec::new();
        for face in card["faces"].as_array().unwrap() {
            let face_name = face["name"].as_str().unwrap();
            if let Err(error) = compile_artifact(CompileInput {
                name: face_name,
                text: face["text"].as_str().unwrap(),
                score: None,
                local_id: 1,
                other_face_id: None,
                other_face_name: None,
                layout: LinkedFaceLayout::None,
            }) {
                errors.push(format!("{face_name}: {error}"));
            }
        }
        if !errors.is_empty() {
            failures.extend(errors.clone());
        }
        results.push(serde_json::json!({"name": name, "errors": errors}));
    }
    if let Ok(output) = std::env::var("CATALOG_AUDIT_OUTPUT") {
        std::fs::write(output, serde_json::to_string_pretty(&results).unwrap()).unwrap();
    }
    eprintln!("Audited {} catalog cards", cards.len());
    assert!(
        failures.is_empty(),
        "{} failing faces:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[derive(Default)]
struct AcceptChoices;
impl engine::decision::DecisionMaker for AcceptChoices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &engine::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
    fn decide_objects(
        &mut self,
        _: &GameState,
        ctx: &engine::decisions::context::SelectObjectsContext,
    ) -> Vec<engine::ObjectId> {
        ctx.candidates
            .iter()
            .take(ctx.min.max(1))
            .map(|c| c.id)
            .collect()
    }
}

#[test]
fn catalog_town_greeter_rewards_only_a_town_returned_from_its_milled_cards() {
    use engine::types::{CardType, Subtype};
    for town in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let definition = fixture("Town Greeter");
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let trigger = definition
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(a) => Some(a),
                _ => None,
            })
            .unwrap();
        let land =
            engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), "Land")
                .card_types(vec![CardType::Land])
                .subtypes(if town {
                    vec![Subtype::Town]
                } else {
                    Vec::new()
                })
                .build();
        game.create_object_from_definition(&land, alice, Zone::Library);
        for _ in 0..3 {
            game.create_object_from_definition(&definition, alice, Zone::Library);
        }
        game.push_to_stack(engine::game_state::StackEntry::ability(
            source,
            alice,
            trigger.effects.clone(),
        ));
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 3);
        assert_eq!(game.player(alice).unwrap().life, if town { 22 } else { 20 });
    }
}

#[test]
fn catalog_teachings_uses_the_original_hand_size_for_both_thresholds() {
    for (opponent_hand, drawn) in [(1, 0), (2, 0), (3, 2), (5, 2), (6, 3)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = fixture("Teachings of the Archaics");
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        for _ in 0..2 {
            game.create_object_from_definition(&definition, alice, Zone::Hand);
        }
        for _ in 0..opponent_hand {
            game.create_object_from_definition(&definition, bob, Zone::Hand);
        }
        for _ in 0..5 {
            game.create_object_from_definition(&definition, alice, Zone::Library);
        }
        game.push_to_stack(engine::game_state::StackEntry::new(spell, alice));
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            2 + drawn,
            "opponent hand={opponent_hand}"
        );
    }
}

#[test]
fn catalog_unravel_reads_the_countered_spells_mana_payment() {
    for (spent, x, draw) in [
        (0, None, true),
        (4, None, true),
        (5, None, false),
        (6, None, false),
        (5, Some(5), true),
        (6, Some(5), false),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = fixture("Unravel");
        let target_card = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Spell",
        )
        .card_types(vec![engine::types::CardType::Sorcery])
        .mana_cost(engine::mana::ManaCost::from_symbols(if x.is_some() {
            vec![
                engine::mana::ManaSymbol::Generic(1),
                engine::mana::ManaSymbol::X,
            ]
        } else {
            vec![engine::mana::ManaSymbol::Generic(5)]
        }))
        .build();
        let target = game.create_object_from_definition(&target_card, bob, Zone::Stack);
        game.object_mut(target).unwrap().x_value = x;
        game.object_mut(target)
            .unwrap()
            .mana_spent_to_cast
            .add(engine::mana::ManaSymbol::Blue, spent);
        let mut target_entry = engine::game_state::StackEntry::new(target, bob);
        target_entry.x_value = x;
        game.push_to_stack(target_entry);
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        game.create_object_from_definition(&definition, alice, Zone::Library);
        game.push_to_stack(
            engine::game_state::StackEntry::new(spell, alice)
                .with_targets(vec![engine::game_state::Target::Object(target)]),
        );
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
        assert_eq!(game.stack.len(), 0);
        assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            usize::from(draw),
            "spent={spent}"
        );
    }
}

#[test]
fn catalog_redcap_melee_sacrifices_a_land_only_after_nonred_damage() {
    for red in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = fixture("Redcap Melee");
        let target_card = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Target",
        )
        .card_types(vec![engine::types::CardType::Creature])
        .power_toughness(engine::card::PowerToughness::fixed(5, 5))
        .mana_cost(engine::mana::ManaCost::from_symbols(vec![if red {
            engine::mana::ManaSymbol::Red
        } else {
            engine::mana::ManaSymbol::Blue
        }]))
        .build();
        let target = game.create_object_from_definition(&target_card, bob, Zone::Battlefield);
        let land_card =
            engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), "Land")
                .card_types(vec![engine::types::CardType::Land])
                .build();
        let land = game.create_object_from_definition(&land_card, alice, Zone::Battlefield);
        let stable = game.object(land).unwrap().stable_id;
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        game.push_to_stack(
            engine::game_state::StackEntry::new(spell, alice)
                .with_targets(vec![engine::game_state::Target::Object(target)]),
        );
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
        assert_eq!(game.damage_on(target), 4);
        let land = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(land).unwrap().zone,
            if red {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
    }
}

#[test]
fn catalog_hoverships_exiled_owner_manifests_dread_after_the_vehicle_leaves() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = fixture("Unidentified Hovership");
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let triggers: Vec<_> = definition
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            AbilityKind::Triggered(a) => Some(a),
            _ => None,
        })
        .collect();
    let creature =
        engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), "Creature")
            .card_types(vec![engine::types::CardType::Creature])
            .power_toughness(engine::card::PowerToughness::fixed(2, 2))
            .build();
    let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let target_stable = game.object(target).unwrap().stable_id;
    for player in [alice, bob] {
        for _ in 0..3 {
            game.create_object_from_definition(&creature, player, Zone::Library);
        }
    }
    game.push_to_stack(
        engine::game_state::StackEntry::ability(source, alice, triggers[0].effects.clone())
            .with_targets(vec![engine::game_state::Target::Object(target)]),
    );
    engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
    assert_eq!(
        game.object(game.find_object_by_stable_id(target_stable).unwrap())
            .unwrap()
            .zone,
        Zone::Exile
    );
    let snapshot = engine::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(source).unwrap(),
        &game,
    );
    game.move_object_by_effect(source, Zone::Graveyard);
    let event = engine::triggers::TriggerEvent::new_with_provenance(
        engine::events::zones::ZoneChangeEvent::batch_with_snapshots(
            vec![source],
            Zone::Battlefield,
            Zone::Graveyard,
            engine::events::cause::EventCause::effect(),
            vec![snapshot.clone()],
        ),
        engine::provenance::ProvNodeId::default(),
    );
    game.push_to_stack(
        engine::game_state::StackEntry::ability(source, alice, triggers[1].effects.clone())
            .with_source_snapshot(snapshot)
            .with_triggering_event(event),
    );
    engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
    assert_eq!(game.player(alice).unwrap().library.len(), 3);
    assert_eq!(game.player(bob).unwrap().library.len(), 1);
    assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
    assert_eq!(game.battlefield.len(), 1);
    assert!(game.is_face_down(game.battlefield[0]));
    assert_eq!(
        game.controller_of(game.object(game.battlefield[0]).unwrap()),
        bob
    );
}

#[test]
fn catalog_case_solves_only_with_an_empty_hand_and_enables_the_upkeep_trigger() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let definition = fixture("Case of the Crimson Pulse");
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
    for _ in 0..4 {
        game.create_object_from_definition(&definition, alice, Zone::Library);
    }
    let end = engine::triggers::TriggerEvent::new_with_provenance(
        engine::events::phase::BeginningOfEndStepEvent::new(alice),
        engine::provenance::ProvNodeId::default(),
    );
    let upkeep = engine::triggers::TriggerEvent::new_with_provenance(
        engine::events::phase::BeginningOfUpkeepEvent::new(alice),
        engine::provenance::ProvNodeId::default(),
    );
    assert!(engine::triggers::check_triggers(&game, &upkeep).is_empty());
    assert!(engine::triggers::check_triggers(&game, &end).is_empty());
    game.move_object_by_effect(hand, Zone::Graveyard);
    let solve = engine::triggers::check_triggers(&game, &end);
    assert_eq!(solve.len(), 1);
    let mut queue = engine::triggers::TriggerQueue::new();
    for trigger in solve {
        queue.add(trigger);
    }
    engine::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut AcceptChoices)
        .unwrap();
    engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
    assert!(game.is_case_solved(source));
    let upkeep = engine::triggers::check_triggers(&game, &upkeep);
    assert_eq!(upkeep.len(), 1);
    game.create_object_from_definition(&definition, alice, Zone::Hand);
    game.push_to_stack(engine::game_state::StackEntry::ability(
        source,
        alice,
        upkeep[0].ability.effects.clone(),
    ));
    engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
    assert_eq!(game.player(alice).unwrap().hand.len(), 2);
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 2);
}

#[test]
fn catalog_malcolm_casts_the_discarded_card_only_after_the_fourth_counter() {
    for initial in [2, 3] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let definition = fixture("Malcolm, Alluring Scoundrel");
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .counters
            .insert(engine::object::CounterType::Named("chorus".into()), initial);
        let trigger = definition
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(a) => Some(a),
                _ => None,
            })
            .unwrap();
        let spell = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Free spell",
        )
        .card_types(vec![engine::types::CardType::Sorcery])
        .mana_cost(engine::mana::ManaCost::from_symbols(vec![
            engine::mana::ManaSymbol::Blue,
        ]))
        .with_spell_effect(vec![engine::effect::Effect::gain_life(5)])
        .build();
        game.create_object_from_definition(&spell, alice, Zone::Library);
        let event = engine::triggers::TriggerEvent::new_with_provenance(
            engine::events::DamageEvent::with_cause(
                source,
                engine::events::DamageTarget::Player(PlayerId::from_index(1)),
                2,
                true,
                engine::events::cause::EventCause::effect(),
            ),
            engine::provenance::ProvNodeId::default(),
        );
        game.push_to_stack(
            engine::game_state::StackEntry::ability(source, alice, trigger.effects.clone())
                .with_triggering_event(event),
        );
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&engine::object::CounterType::Named("chorus".into())),
            Some(&(initial + 1))
        );
        assert_eq!(game.stack.len(), usize::from(initial == 3));
        if initial == 3 {
            assert_eq!(
                game.object(game.stack[0].object_id).unwrap().name,
                "Free spell"
            );
            engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
            assert_eq!(game.player(alice).unwrap().life, 25);
        }
    }
}

#[test]
fn catalog_robbery_permission_tracks_only_that_exile_incarnation() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = fixture("Outrageous Robbery");
    let creature =
        engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), "Creature")
            .card_types(vec![engine::types::CardType::Creature])
            .power_toughness(engine::card::PowerToughness::fixed(2, 2))
            .mana_cost(engine::mana::ManaCost::from_symbols(vec![
                engine::mana::ManaSymbol::Green,
            ]))
            .build();
    let card = game.create_object_from_definition(&creature, bob, Zone::Library);
    let stable = game.object(card).unwrap().stable_id;
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut entry = engine::game_state::StackEntry::new(spell, alice)
        .with_targets(vec![engine::game_state::Target::Player(bob)]);
    entry.x_value = Some(1);
    game.push_to_stack(entry);
    engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert!(game.is_face_down(exiled));
    assert!(game.can_player_look_at_face_down_exiled_card(exiled, alice));
    assert!(!game.can_player_look_at_face_down_exiled_card(exiled, bob));
    game.turn.turn_number += 4;
    assert!(game.effect_store.grant_registry.card_can_play_from_zone(
        &game,
        exiled,
        Zone::Exile,
        alice
    ));
    let hand = game.move_object_by_effect(exiled, Zone::Hand).unwrap();
    let reexiled = game.move_object_by_effect(hand, Zone::Exile).unwrap();
    assert!(!game.effect_store.grant_registry.card_can_play_from_zone(
        &game,
        reexiled,
        Zone::Exile,
        alice
    ));
}

#[test]
fn catalog_quipu_draws_for_artifacts_then_returns_third_from_top() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let definition = fixture("Braided Quipu");
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let stable = game.object(source).unwrap().stable_id;
    let ability = definition
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    for _ in 0..2 {
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    }
    for _ in 0..8 {
        game.create_object_from_definition(&definition, alice, Zone::Library);
    }
    game.push_to_stack(engine::game_state::StackEntry::ability(
        source,
        alice,
        ability.effects.clone(),
    ));
    engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
    assert_eq!(game.player(alice).unwrap().hand.len(), 3);
    assert_eq!(game.battlefield.len(), 2);
    let library = &game.player(alice).unwrap().library;
    assert_eq!(library.len(), 6);
    assert_eq!(
        game.find_object_by_stable_id(stable),
        Some(library[library.len() - 3])
    );
}

#[test]
fn catalog_oven_uses_the_sacrificed_creatures_last_known_toughness() {
    for toughness in [3, 4, 5] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        game.turn.phase = engine::Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        let definition = fixture("Witch's Oven");
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            "Sacrifice",
        )
        .card_types(vec![engine::types::CardType::Creature])
        .power_toughness(engine::card::PowerToughness::fixed(1, toughness))
        .build();
        game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let action = engine::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .find(|a| {
                matches!(a,
            engine::decision::LegalAction::ActivateAbility { source: id, .. } if *id == source)
            })
            .expect("oven activation");
        let mut state = engine::game_loop::PriorityLoopState::new(2);
        let mut queue = engine::triggers::TriggerQueue::new();
        let mut progress = engine::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &engine::game_loop::PriorityResponse::PriorityAction(action),
            &mut AcceptChoices,
        )
        .unwrap();
        loop {
            let response = match &progress {
                engine::decision::GameProgress::NeedsDecisionCtx(
                    engine::decisions::context::DecisionContext::ManaPayment(payment),
                ) => engine::game_loop::PriorityResponse::ManaPaymentPlan(
                    engine::mana_payment::ManaPaymentResponse::Confirm {
                        plan_id: payment.plan.id,
                        request_hash: payment.plan.request_hash,
                    },
                ),
                engine::decision::GameProgress::NeedsDecisionCtx(
                    engine::decisions::context::DecisionContext::SelectObjects(selection),
                ) => {
                    engine::game_loop::PriorityResponse::SacrificeTarget(selection.candidates[0].id)
                }
                _ => break,
            };
            progress = engine::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &response,
                &mut AcceptChoices,
            )
            .unwrap();
        }
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            1,
            "{progress:?}"
        );
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptChoices).unwrap();
        assert_eq!(
            game.battlefield
                .iter()
                .filter(|id| game.object(**id).unwrap().name == "Food")
                .count(),
            if toughness >= 4 { 2 } else { 1 }
        );
    }
}

#[test]
fn catalog_capstone_stops_at_total_value_and_offers_each_exiled_spell() {
    struct CastAll;
    impl engine::decision::DecisionMaker for CastAll {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &engine::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
        fn decide_objects(
            &mut self,
            _: &GameState,
            ctx: &engine::decisions::context::SelectObjectsContext,
        ) -> Vec<engine::ObjectId> {
            ctx.candidates
                .iter()
                .take(ctx.max.unwrap_or(ctx.candidates.len()))
                .map(|c| c.id)
                .collect()
        }
    }
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let definition = fixture("Improvisation Capstone");
    for mv in [9, 2, 0, 2] {
        let card = engine::cards::builders::CardDefinitionBuilder::new(
            engine::ids::CardId::new(),
            format!("Spell {mv}"),
        )
        .card_types(vec![engine::types::CardType::Sorcery])
        .mana_cost(engine::mana::ManaCost::from_symbols(vec![
            engine::mana::ManaSymbol::Generic(mv),
        ]))
        .with_spell_effect(vec![engine::effect::Effect::gain_life(1)])
        .build();
        game.create_object_from_definition(&card, alice, Zone::Library);
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(engine::game_state::StackEntry::new(source, alice));
    engine::game_loop::resolve_stack_entry_with(&mut game, &mut CastAll).unwrap();
    assert_eq!(game.player(alice).unwrap().library.len(), 1);
    assert_eq!(game.stack.len(), 3);
    while !game.stack.is_empty() {
        engine::game_loop::resolve_stack_entry_with(&mut game, &mut CastAll).unwrap();
    }
    assert_eq!(game.player(alice).unwrap().life, 23);
}

#[test]
fn catalog_manifest_dread_preserves_old_artifacts_and_the_named_player() {
    use engine::target::PlayerFilter;
    use ironsmith_compiler::effects::ManifestDreadEffect;
    let legacy: ManifestDreadEffect = serde_json::from_str("null").unwrap();
    assert_eq!(legacy.player, PlayerFilter::You);
    let effect = ManifestDreadEffect::for_player(PlayerFilter::Specific(PlayerId::from_index(1)));
    let json = serde_json::to_string(&effect).unwrap();
    let decoded: ManifestDreadEffect = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.player, effect.player);
}
