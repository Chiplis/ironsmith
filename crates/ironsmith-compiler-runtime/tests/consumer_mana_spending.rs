//! UNVALIDATED: consumer-side source restrictions, carried by the priced cost.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::{
    ManaPaymentFailure, ManaPaymentRequest, execute_mana_payment_plan, mana_payment_transaction_id,
    plan_first_mana_payment,
};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/consumer_mana_spending.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [ironsmith::cards::CardDefinition; 2] {
    let rows = rows();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn source(game: &mut GameState, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    let id = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}
fn cast_action(game: &GameState, spell: ObjectId, alternative: bool) -> Option<LegalAction> {
    compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(action,
        LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == spell &&
            if alternative { matches!(casting_method, CastingMethod::Alternative(_)) }
            else { matches!(casting_method, CastingMethod::Normal) })
        })
}
fn announce(game: &mut GameState, action: LegalAction) {
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            return;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .unwrap();
    }
    panic!("announcement did not finish");
}
fn cast(game: &mut GameState, spell: ObjectId, alternative: bool) {
    let action = cast_action(game, spell, alternative).unwrap();
    announce(game, action);
    resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
}
fn activate(game: &mut GameState, source: ObjectId) {
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } | LegalAction::ActivateManaAbility { source: id, .. } if *id == source)).unwrap();
    announce(game, action);
    assert!(game.stack.is_empty());
}
#[test]
fn exact_source_spending_cards_compile_and_round_trip_with_visible_rules() {
    let rows = rows();
    assert_eq!(rows.len(), 10);
    assert_eq!(
        rows.iter()
            .filter(|row| row["proposed_coverage"] == "source_complete_unvalidated")
            .count(),
        10
    );
    for name in ["Imperiosaur", "Myr Superion", "Security Rhox"] {
        for definition in definitions(name) {
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            assert!(
                rendered
                    .to_ascii_lowercase()
                    .contains("spend only mana produced by"),
                "{name}: {rendered}"
            );
        }
    }
}
#[test]
fn imperiosaur_requires_basic_production_in_discovery_planning_and_paid_casting() {
    for definition in definitions("Imperiosaur") {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let wrong = (0..4)
            .map(|_| {
                source(
                    &mut game,
                    "Nonbasic green source",
                    "Type: Land\n{T}: Add {G}.",
                )
            })
            .collect::<Vec<_>>();
        assert!(cast_action(&game, spell, false).is_none());
        let right = (0..4)
            .map(|_| {
                source(
                    &mut game,
                    "Basic green source",
                    "Type: Basic Land\n{T}: Add {G}.",
                )
            })
            .collect::<Vec<_>>();
        cast(&mut game, spell, false);
        assert!(right.iter().all(|id| game.is_tapped(*id)));
        assert!(
            wrong.iter().all(|id| !game.is_tapped(*id)),
            "equivalent colors cannot merge away source eligibility"
        );
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(
            game.battlefield
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Imperiosaur")
        );
    }
}
#[test]
fn myr_superion_requires_creature_production_even_when_a_land_makes_identical_mana() {
    for definition in definitions("Myr Superion") {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let land = source(
            &mut game,
            "Noncreature producer",
            "Type: Land\n{T}: Add {C}{C}.",
        );
        assert!(cast_action(&game, spell, false).is_none());
        let creature = source(
            &mut game,
            "Creature producer",
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 1/1\n{T}: Add {C}{C}.",
        );
        cast(&mut game, spell, false);
        assert!(game.is_tapped(creature));
        assert!(!game.is_tapped(land));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn sacrificed_producer_remains_qualified_by_its_production_snapshot() {
    for definition in definitions("Myr Superion") {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let producer = source(
            &mut game,
            "Departing creature producer",
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 1/1\n{T}, Sacrifice this creature: Add {C}{C}.",
        );
        activate(&mut game, producer);
        assert!(!game.battlefield.contains(&producer));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
        cast(&mut game, spell, false);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn rhox_treasure_rule_belongs_only_to_its_chosen_alternative_and_includes_taxes() {
    for definition in definitions("Security Rhox") {
        for (alternative, taxed) in [(false, false), (true, false), (true, true)] {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let lands = ["{R}", "{G}", "{C}", "{C}"]
                .into_iter()
                .map(|symbol| {
                    source(
                        &mut game,
                        "Ordinary producer",
                        &format!("Type: Land\n{{T}}: Add {symbol}."),
                    )
                })
                .collect::<Vec<_>>();
            assert!(cast_action(&game, spell, true).is_none());
            if taxed {
                source(
                    &mut game,
                    "Spell tax",
                    "Type: Artifact\nSpells cost {1} more to cast.",
                );
            }
            let treasures = if alternative {
                (0..if taxed { 3 } else { 2 })
                    .map(|_| {
                        game.create_object_from_definition(
                            &ironsmith::cards::tokens::treasure_token_definition(),
                            A,
                            Zone::Battlefield,
                        )
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            cast(&mut game, spell, alternative);
            assert!(treasures.iter().all(|id| !game.battlefield.contains(id)));
            let tapped_lands = lands.iter().filter(|id| game.is_tapped(**id)).count() as u32;
            if !alternative {
                assert_eq!(tapped_lands, 4);
            }
            // The first legal plan need not minimize extra activations. Every
            // ordinary land mana remains unspent under the Treasure-only price.
            assert_eq!(
                game.player(A).unwrap().mana_pool.total(),
                if alternative { tapped_lands } else { 0 }
            );
            let paid = game
                .battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .find(|object| object.name == "Security Rhox")
                .unwrap();
            assert_eq!(
                paid.mana_spent_to_cast.total(),
                if alternative {
                    if taxed { 3 } else { 2 }
                } else {
                    4
                }
            );
        }
    }
}
#[test]
fn request_hash_and_commit_reject_a_plan_with_the_same_pips_but_a_different_source_rule() {
    use ironsmith_core::mana::{ManaProducerFilter, ManaSpendingRestriction};
    let mut game = game();
    let producer = source(
        &mut game,
        "Creature resource",
        "Type: Creature\nPower/Toughness: 1/1\n{T}: Add {C}{C}.",
    );
    let spell = source(&mut game, "Payment source", "Type: Artifact");
    let raw = ManaCost::new().add_generic(2);
    let restricted = raw
        .clone()
        .with_spending_restriction(ManaSpendingRestriction::ProducedBy(
            ManaProducerFilter::CardType(ironsmith::CardType::Creature),
        ));
    let reason = ironsmith::costs::PaymentReason::CastSpell;
    let plain = ManaPaymentRequest::new(A, spell, reason, raw);
    let constrained = ManaPaymentRequest::new(A, spell, reason, restricted);
    assert_ne!(
        mana_payment_transaction_id(&plain),
        mana_payment_transaction_id(&constrained)
    );
    let plain_plan = plan_first_mana_payment(&game, &plain).unwrap();
    assert!(matches!(
        execute_mana_payment_plan(
            &mut game,
            &constrained,
            &plain_plan,
            &mut SelectFirstDecisionMaker
        ),
        Err(ManaPaymentFailure::StalePlan)
    ));
    assert!(!game.is_tapped(producer));
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    let plan = plan_first_mana_payment(&game, &constrained).unwrap();
    assert_eq!(
        plan.mana_cost_after_alternatives.spending_restrictions(),
        constrained.cost.spending_restrictions()
    );
    execute_mana_payment_plan(
        &mut game,
        &constrained,
        &plan,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert!(game.is_tapped(producer));
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
}

#[test]
fn old_mana_json_remains_exact_and_new_source_rule_round_trips() {
    use ironsmith_core::mana::{ManaProducerFilter, ManaSpendingRestriction};
    let legacy = r#"{"pips":[[{"Generic":2}]]}"#;
    let plain: ManaCost = serde_json::from_str(legacy).unwrap();
    assert!(plain.spending_restrictions().is_empty());
    assert_eq!(serde_json::to_string(&plain).unwrap(), legacy);
    let constrained = plain.with_spending_restriction(ManaSpendingRestriction::ProducedBy(
        ManaProducerFilter::Subtype(ironsmith::Subtype::Treasure),
    ));
    let restored: ManaCost =
        serde_json::from_str(&serde_json::to_string(&constrained).unwrap()).unwrap();
    assert_eq!(restored, constrained);
}

#[test]
fn actual_production_characteristics_are_frozen_before_later_animation_changes() {
    for definition in definitions("Myr Superion") {
        for animated_at_production in [false, true] {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let producer = source(
                &mut game,
                "Animated mana source",
                "Type: Land\nPower/Toughness: 2/2\n{T}: Add {C}{C}.",
            );
            let animation = || {
                ironsmith::continuous::ContinuousEffect::new(
                    producer,
                    A,
                    ironsmith::continuous::EffectTarget::Specific(producer),
                    ironsmith::continuous::Modification::AddCardTypes(vec![
                        ironsmith::CardType::Creature,
                    ]),
                )
            };
            let active = animated_at_production
                .then(|| game.effect_store.continuous_effects.add_effect(animation()));
            game.refresh_continuous_state().unwrap();
            activate(&mut game, producer);
            if let Some(active) = active {
                game.effect_store.continuous_effects.remove_effect(active);
            } else {
                game.effect_store.continuous_effects.add_effect(animation());
            }
            game.refresh_continuous_state().unwrap();
            assert_eq!(
                cast_action(&game, spell, false).is_some(),
                animated_at_production
            );
            if animated_at_production {
                cast(&mut game, spell, false);
            } else {
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
            }
        }
    }
}
#[test]
fn untracked_mana_and_as_though_color_permission_cannot_supply_producer_evidence() {
    use ironsmith_core::mana::{ManaProducerFilter, ManaSpendingRestriction};
    let mut game = game();
    let spell = source(&mut game, "Payment source", "Type: Artifact");
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    let constrained = ManaCost::from_symbols(vec![ManaSymbol::Green]).with_spending_restriction(
        ManaSpendingRestriction::ProducedBy(ManaProducerFilter::CardType(
            ironsmith::CardType::Creature,
        )),
    );
    let mut request = ManaPaymentRequest::new(
        A,
        spell,
        ironsmith::costs::PaymentReason::CastSpell,
        constrained,
    )
    .with_spend_policy(ironsmith::player::ManaSpendPolicy::from_any_color(true));
    assert!(plan_first_mana_payment(&game, &request).is_err());
    assert_eq!(game.player(A).unwrap().mana_pool.blue, 1);
    request.cost = ManaCost::from_symbols(vec![ManaSymbol::Green]);
    assert!(plan_first_mana_payment(&game, &request).is_ok());
}

#[test]
fn a_creature_card_in_hand_is_not_a_creature_mana_producer() {
    for definition in definitions("Myr Superion") {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let spirit = compile_to_runtime_definition("Hand mana producer",
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2\nExile this card from your hand: Add {C}{C}.", false).unwrap();
        let producer = game.create_object_from_definition(&spirit, A, Zone::Hand);
        activate(&mut game, producer);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
        assert!(cast_action(&game, spell, false).is_none());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
    }
}
#[test]
fn triggered_bonus_mana_is_produced_by_its_own_source_not_the_tapped_basic_land() {
    for definition in definitions("Imperiosaur") {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        source(
            &mut game,
            "Triggered mana producer",
            "Type: Enchantment\nWhenever you tap a land for mana, add {G}.",
        );
        for _ in 0..2 {
            let land = source(
                &mut game,
                "Basic producer",
                "Type: Basic Land\n{T}: Add {G}.",
            );
            activate(&mut game, land);
        }
        assert_eq!(game.player(A).unwrap().mana_pool.green, 4);
        assert!(
            cast_action(&game, spell, false).is_none(),
            "only two of the four green mana were produced by basic lands"
        );
        assert_eq!(game.player(A).unwrap().mana_pool.green, 4);
    }
}

#[test]
fn assist_retains_the_spells_source_rule_for_the_helpers_actual_payment() {
    for mut definition in definitions("Imperiosaur") {
        definition
            .abilities
            .push(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::assist(),
            ));
        for basic_helper in [false, true] {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let caster_sources = (0..2)
                .map(|_| {
                    source(
                        &mut game,
                        "Caster's basic source",
                        "Type: Basic Land\n{T}: Add {G}.",
                    )
                })
                .collect::<Vec<_>>();
            let helper_definition = compile_to_runtime_definition(
                "Helper's source",
                if basic_helper {
                    "Type: Basic Land\n{T}: Add {C}."
                } else {
                    "Type: Land\n{T}: Add {C}."
                },
                false,
            )
            .unwrap();
            let helpers = (0..2)
                .map(|_| {
                    game.create_object_from_definition(
                        &helper_definition,
                        PlayerId(1),
                        Zone::Battlefield,
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(cast_action(&game, spell, false).is_some(), basic_helper);
            if basic_helper {
                cast(&mut game, spell, false);
                assert!(
                    caster_sources
                        .iter()
                        .chain(helpers.iter())
                        .all(|id| game.is_tapped(*id))
                );
                assert!(
                    game.players
                        .iter()
                        .all(|player| player.mana_pool.total() == 0)
                );
            } else {
                assert!(
                    caster_sources
                        .iter()
                        .chain(helpers.iter())
                        .all(|id| !game.is_tapped(*id))
                );
            }
        }
    }
}

#[test]
fn assist_menu_keeps_a_helper_production_discovery_failure_typed_and_pending() {
    std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(|| {
            for mut definition in definitions("Imperiosaur") {
                definition
                    .abilities
                    .push(ironsmith::ability::Ability::static_ability(
                        ironsmith::static_abilities::StaticAbility::assist(),
                    ));
                let mut game = game();
                let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                let payer_sources = (0..4)
                    .map(|_| source(&mut game, "Payer basic", "Type: Basic Land\n{T}: Add {G}."))
                    .collect::<Vec<_>>();
                let helper = compile_to_runtime_definition(
                    "Tap-sensitive helper",
                    "Type: Basic Land\n{T}: Add {G}.",
                    false,
                )
                .unwrap();
                let helper =
                    game.create_object_from_definition(&helper, PlayerId(1), Zone::Battlefield);
                let mut model: ironsmith::static_abilities::CompiledStaticAbility =
                    ironsmith_core::StaticAbility::haste();
                for _ in 0..140 {
                    model = ironsmith_core::StaticAbility::grant_object_ability_for_filter(
                        ironsmith::target::ObjectFilter::source(),
                        ironsmith_core::Ability::static_ability(model),
                        "Source gains a finite child",
                    );
                }
                model = model.with_condition(ironsmith::ConditionExpr::SourceIsTapped);
                game.object_mut(helper).unwrap().abilities_mut().push(
                    ironsmith::ability::Ability::static_ability(
                        ironsmith::static_abilities::StaticAbility::from_model(model),
                    ),
                );
                game.refresh_continuous_state().unwrap();
                // The caster can pay unaided, so legal enumeration can establish a
                // real cast before the later Assist menu analyzes the helper.
                let action = cast_action(&game, spell, false).unwrap();
                let mut state = PriorityLoopState::new(2);
                let mut queue = TriggerQueue::new();
                let mut dm = SelectFirstDecisionMaker;
                let mut result = apply_priority_response_with_dm(
                    &mut game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::PriorityAction(action),
                    &mut dm,
                );
                for _ in 0..40 {
                    let Ok(GameProgress::NeedsDecisionCtx(context)) = &result else {
                        break;
                    };
                    let context = context.clone();
                    result = apply_decision_context_with_dm(
                        &mut game, &mut queue, &mut state, &context, &mut dm,
                    );
                }
                assert!(
                    matches!(
                        result,
                        Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(
                            ironsmith::effects::ExecutionError::ContinuousDiscovery(_)
                        ))
                    ),
                    "{result:?}"
                );
                let pending = state
                    .pending_cast
                    .as_ref()
                    .expect("incomplete query retains the announced spell");
                assert_eq!(pending.caster, A);
                assert!(
                    !pending
                        .mana_cost_to_pay
                        .as_ref()
                        .unwrap()
                        .spending_restrictions()
                        .is_empty()
                );
                assert!(!game.is_tapped(helper));
                assert!(payer_sources.iter().all(|id| !game.is_tapped(*id)));
                assert!(
                    game.players
                        .iter()
                        .all(|player| player.mana_pool.total() == 0 && player.life == 20)
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
