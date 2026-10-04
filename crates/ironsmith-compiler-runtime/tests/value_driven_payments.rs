//! UNVALIDATED implementation-first regressions for value-driven activation payments.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::costs::{Cost, CostContext, PaymentReason};
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker,
    calculate_effective_activation_total_cost, compute_legal_actions,
};
use ironsmith::decisions::context::{
    ManaPaymentContext, NumberContext, SelectObjectsContext, TargetsContext,
};
use ironsmith::effect::{Effect, Until, Value};
use ironsmith::effects::{EffectContext, PayEnergyEffect, PayLifeEffect, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::object::CounterType;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/value_driven_payments.json.fixture"
    ))
    .unwrap()
}
fn round_trip(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let card = fixtures()
        .into_iter()
        .find(|card| card["name"] == name)
        .unwrap();
    round_trip(name, card["text"].as_str().unwrap())
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let filler = CardBuilder::new(CardId::new(), "Library filler")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..10 {
        game.create_object_from_card(&filler, A, Zone::Library);
    }
    game
}
fn permanent(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
    mv: u8,
    kind: CardType,
    zone: Zone,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(mv)]]))
        .card_types(vec![kind])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_card(&card, owner, zone)
}
fn payment_ability(def: &CardDefinition) -> usize {
    def.abilities.iter().position(|ability| matches!(&ability.kind,
        AbilityKind::Activated(a) if a.mana_cost.costs().iter().any(|cost|
            cost.dynamic_mana_cost_ref().is_some() || cost.effect_ref().is_some_and(|e|
                e.downcast_ref::<PayEnergyEffect>().is_some() || e.downcast_ref::<PayLifeEffect>().is_some()))))
        .expect("printed value-driven activation")
}
fn action(def: &CardDefinition, source: ObjectId) -> LegalAction {
    LegalAction::ActivateAbility {
        source,
        ability_index: payment_ability(def),
    }
}
#[derive(Default)]
struct Choices {
    x: u32,
    max_x: Option<u32>,
    saw_x: bool,
    target: Option<Target>,
    forbidden: Vec<Target>,
    sacrifice: Option<ObjectId>,
    cancel: bool,
    cancelled: bool,
}
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        assert!(ctx.is_x_value);
        assert_eq!(ctx.min, 0);
        if let Some(expected) = self.max_x {
            assert_eq!(ctx.max, expected);
        }
        assert!(self.x <= ctx.max);
        self.saw_x = true;
        self.x
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        for forbidden in &self.forbidden {
            assert!(
                !ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(forbidden))
            );
        }
        if let Some(target) = self.target {
            assert!(
                ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&target))
            );
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, ctx)
        }
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.sacrifice {
            assert!(
                ctx.candidates
                    .iter()
                    .any(|candidate| candidate.legal && candidate.id == id)
            );
            vec![id]
        } else {
            SelectFirstDecisionMaker.decide_objects(game, ctx)
        }
    }
    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        ctx: &ManaPaymentContext,
    ) -> ManaPaymentResponse {
        if self.cancel {
            self.cancelled = true;
            ManaPaymentResponse::Cancel
        } else {
            SelectFirstDecisionMaker.decide_mana_payment(game, ctx)
        }
    }
}
fn activate(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("unfinished activation: {progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
}
fn mana(game: &mut GameState, symbol: ManaSymbol, count: u32) {
    game.player_mut(A).unwrap().mana_pool.add(symbol, count);
}

#[test]
fn frozen_value_payment_cards_keep_typed_costs_through_artifact_transport() {
    let cards = fixtures();
    assert_eq!(cards.len(), 8);
    assert_eq!(
        cards
            .iter()
            .filter(|c| c["proposed_coverage"] == "complete")
            .count(),
        7
    );
    for card in cards
        .into_iter()
        .filter(|c| c["proposed_coverage"] == "complete")
    {
        for definition in definitions(card["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            payment_ability(&definition);
        }
    }
}

#[test]
fn sphinx_energy_x_is_announced_paid_and_retained_after_source_departure() {
    for definition in definitions("Sphinx of the Revelation") {
        for x in [0, 3] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            game.player_mut(A).unwrap().energy_counters = 4;
            mana(&mut game, ManaSymbol::White, 1);
            mana(&mut game, ManaSymbol::Blue, 2);
            let mut dm = Choices {
                x,
                max_x: Some(4),
                ..Default::default()
            };
            activate(&mut game, action(&definition, source), &mut dm);
            assert!(dm.saw_x);
            assert_eq!(game.player(A).unwrap().energy_counters, 4 - x);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert!(game.is_tapped(source));
            assert!(game.player(A).unwrap().hand.is_empty());
            game.move_object_by_effect(source, Zone::Exile).unwrap();
            game.player_mut(A).unwrap().energy_counters = 9;
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                x as usize,
                "resolution uses announced X, not current energy or paid-count drift"
            );
        }
    }
}

#[test]
fn cancel_x_energy_activation_restores_all_cost_resources() {
    for definition in definitions("Sphinx of the Revelation") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        game.player_mut(A).unwrap().energy_counters = 4;
        mana(&mut game, ManaSymbol::White, 1);
        mana(&mut game, ManaSymbol::Blue, 2);
        let pool = game.player(A).unwrap().mana_pool.clone();
        let mut dm = Choices {
            x: 3,
            max_x: Some(4),
            cancel: true,
            ..Default::default()
        };
        activate(&mut game, action(&definition, source), &mut dm);
        assert!(dm.saw_x && dm.cancelled);
        assert!(game.stack_is_empty());
        assert!(!game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().energy_counters, 4);
        assert_eq!(game.player(A).unwrap().mana_pool, pool);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}

#[test]
fn helios_exact_x_target_survives_announcement_then_dies_after_source_is_sacrificed() {
    for definition in definitions("HELIOS One") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = permanent(
            &mut game,
            B,
            "Three-mana artifact",
            3,
            CardType::Artifact,
            Zone::Battlefield,
        );
        let wrong = permanent(
            &mut game,
            B,
            "Four-mana artifact",
            4,
            CardType::Artifact,
            Zone::Battlefield,
        );
        let land = permanent(
            &mut game,
            B,
            "Three-mana land",
            3,
            CardType::Land,
            Zone::Battlefield,
        );
        game.player_mut(A).unwrap().energy_counters = 4;
        mana(&mut game, ManaSymbol::Colorless, 3);
        let mut dm = Choices {
            x: 3,
            max_x: Some(4),
            target: Some(Target::Object(target)),
            forbidden: vec![Target::Object(wrong), Target::Object(land)],
            ..Default::default()
        };
        activate(&mut game, action(&definition, source), &mut dm);
        assert!(dm.saw_x);
        assert!(game.object(source).is_none());
        assert_eq!(game.player(A).unwrap().energy_counters, 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.object(target).unwrap().zone, Zone::Battlefield);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.object(target).is_none());
        assert_eq!(game.object(wrong).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(land).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn chthonian_energy_is_payers_resource_and_bounce_uses_sources_owner() {
    for definition in definitions("Chthonian Nightmare") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        game.set_current_controller(source, A).unwrap();
        let fodder = permanent(
            &mut game,
            A,
            "Sacrifice fodder",
            3,
            CardType::Creature,
            Zone::Battlefield,
        );
        let target = permanent(
            &mut game,
            A,
            "Return creature",
            3,
            CardType::Creature,
            Zone::Graveyard,
        );
        let foreign = permanent(
            &mut game,
            B,
            "Foreign graveyard",
            3,
            CardType::Creature,
            Zone::Graveyard,
        );
        game.player_mut(A).unwrap().energy_counters = 4;
        game.player_mut(B).unwrap().energy_counters = 7;
        let mut dm = Choices {
            x: 3,
            max_x: Some(4),
            target: Some(Target::Object(target)),
            sacrifice: Some(fodder),
            forbidden: vec![Target::Object(fodder), Target::Object(foreign)],
            ..Default::default()
        };
        activate(&mut game, action(&definition, source), &mut dm);
        assert!(dm.saw_x);
        assert_eq!(game.player(A).unwrap().energy_counters, 1);
        assert_eq!(game.player(B).unwrap().energy_counters, 7);
        assert!(game.object(source).is_none() && game.object(fodder).is_none());
        assert!(
            game.player(B)
                .unwrap()
                .hand
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Chthonian Nightmare")
        );
        assert_eq!(game.object(target).unwrap().zone, Zone::Graveyard);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|o| o.name == "Return creature" && game.current_controller(o.id) == Some(A))
        }));
        assert_eq!(game.object(foreign).unwrap().zone, Zone::Graveyard);
    }
}

#[test]
fn krumar_x_is_bounded_by_both_life_and_mana_and_endure_keeps_announced_amount() {
    for definition in definitions("Krumar Initiate") {
        for (life, available_mana, max, x) in [(20, 6, 5, 3), (2, 6, 2, 1)] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            game.player_mut(A).unwrap().life = life;
            mana(&mut game, ManaSymbol::Black, available_mana);
            let mut dm = Choices {
                x,
                max_x: Some(max),
                ..Default::default()
            };
            activate(&mut game, action(&definition, source), &mut dm);
            assert!(dm.saw_x && game.is_tapped(source));
            assert_eq!(game.player(A).unwrap().life, life - x as i32);
            assert_eq!(
                game.player(A).unwrap().mana_pool.total(),
                available_mana - x - 1
            );
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&CounterType::PlusOnePlusOne)
                    .copied()
                    .unwrap_or(0),
                x
            );
        }
    }
}

#[test]
fn half_life_rounds_up_and_the_determined_cost_does_not_drift_during_payment() {
    for definition in definitions("Lurking Evil") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.player_mut(A).unwrap().life = 15;
        let AbilityKind::Activated(ability) =
            &definition.abilities[payment_ability(&definition)].kind
        else {
            unreachable!()
        };
        let determined =
            calculate_effective_activation_total_cost(&game, A, source, &ability.mana_cost);
        let life_cost = determined
            .costs()
            .iter()
            .find(|c| {
                c.effect_ref()
                    .is_some_and(|e| e.downcast_ref::<PayLifeEffect>().is_some())
            })
            .unwrap();
        let amount = &life_cost
            .effect_ref()
            .unwrap()
            .downcast_ref::<PayLifeEffect>()
            .unwrap()
            .amount;
        assert_eq!(*amount, Value::Fixed(8));
        game.player_mut(A).unwrap().life = 11;
        let mut dm = Choices::default();
        let mut ctx =
            CostContext::new(source, A, &mut dm).with_reason(PaymentReason::ActivateAbility);
        life_cost.pay(&mut game, &mut ctx).unwrap();
        assert_eq!(
            game.player(A).unwrap().life,
            3,
            "locked amount was eight, not half the changed total"
        );

        game.player_mut(A).unwrap().life = 15;
        activate(&mut game, action(&definition, source), &mut dm);
        assert_eq!(game.player(A).unwrap().life, 7);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.current_power(source), Some(4));
        assert_eq!(game.current_toughness(source), Some(4));
        assert!(game.current_has_static_ability_id(
            source,
            ironsmith::static_abilities::StaticAbilityId::Flying
        ));
    }
}

#[test]
fn murderous_betrayal_pays_half_life_and_does_not_allow_a_regeneration_shield() {
    for definition in definitions("Murderous Betrayal") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = permanent(
            &mut game,
            B,
            "Nonblack victim",
            3,
            CardType::Creature,
            Zone::Battlefield,
        );
        let mut dm = Choices {
            target: Some(Target::Object(target)),
            ..Default::default()
        };
        let mut ctx = EffectContext::new_default(target, B);
        execute_effect(
            &mut game,
            &Effect::regenerate(ChooseSpec::SpecificObject(target), Until::EndOfTurn),
            &mut ctx,
        )
        .unwrap();
        game.player_mut(A).unwrap().life = 15;
        mana(&mut game, ManaSymbol::Black, 2);
        activate(&mut game, action(&definition, source), &mut dm);
        assert_eq!(game.player(A).unwrap().life, 7);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(
            game.object(target).is_none(),
            "no-regeneration clause bypasses an existing shield"
        );
    }
}

#[test]
fn tornado_uses_velocity_not_age_counters_and_remains_once_per_turn() {
    for definition in definitions("Tornado") {
        for velocity in [0, 1, 2] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.object_mut(source)
                .unwrap()
                .add_counters(CounterType::Velocity, velocity);
            game.object_mut(source)
                .unwrap()
                .add_counters(CounterType::Age, 4);
            let target = permanent(
                &mut game,
                B,
                "Victim",
                3,
                CardType::Artifact,
                Zone::Battlefield,
            );
            mana(&mut game, ManaSymbol::Green, 6);
            let mut dm = Choices {
                target: Some(Target::Object(target)),
                ..Default::default()
            };
            let act = action(&definition, source);
            activate(&mut game, act.clone(), &mut dm);
            assert_eq!(game.player(A).unwrap().life, 20 - 3 * velocity as i32);
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(game.object(target).is_none());
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&CounterType::Velocity),
                Some(&(velocity + 1))
            );
            assert!(!compute_legal_actions(&game, A).unwrap().contains(&act));
        }
    }
}

#[test]
fn partial_scavengers_cost_uses_existing_dynamic_mana_and_modifies_the_total_after_scaling() {
    // The exact Scavengers program remains partial because its regeneration rider
    // creates a delayed trigger. Isolate only the repaired printed payment syntax.
    let text = "Type: Creature — Skeleton\nPower/Toughness: 1/1\n{2}, Pay {1} for each +1/+1 counter on this creature: Draw a card.";
    for definition in round_trip("Per-counter cost control", text) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .add_counters(CounterType::PlusOnePlusOne, 4);
        let training = compile_to_runtime_definition("Training Grounds", "Mana cost: {U}\nType: Enchantment\nActivated abilities of creatures you control cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana.", false).unwrap();
        game.create_object_from_definition(&training, A, Zone::Battlefield);
        let AbilityKind::Activated(ability) =
            &definition.abilities[payment_ability(&definition)].kind
        else {
            unreachable!()
        };
        let determined =
            calculate_effective_activation_total_cost(&game, A, source, &ability.mana_cost);
        assert_eq!(
            determined
                .costs()
                .iter()
                .filter(|c| c.is_mana_cost())
                .count(),
            1
        );
        assert_eq!(
            determined.mana_cost().unwrap().generic_mana_total(),
            4,
            "(2 + 4) - 2, not 2 + (4 - 2) - 2"
        );
        game.object_mut(source)
            .unwrap()
            .add_counters(CounterType::PlusOnePlusOne, 1);
        assert_eq!(
            determined.mana_cost().unwrap().generic_mana_total(),
            4,
            "already determined total is fixed"
        );
        mana(&mut game, ManaSymbol::Colorless, 5);
        let mut dm = Choices::default();
        activate(&mut game, action(&definition, source), &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
    }
}

#[test]
fn x_resource_preflight_is_nonmutating_and_announced_amounts_fail_closed() {
    let mut game = game();
    let source = permanent(
        &mut game,
        A,
        "Payer",
        0,
        CardType::Artifact,
        Zone::Battlefield,
    );
    game.player_mut(A).unwrap().energy_counters = 2;
    game.player_mut(A).unwrap().life = 3;
    mana(&mut game, ManaSymbol::Blue, 4);
    let pool = game.player(A).unwrap().mana_pool.clone();
    for (effect, available) in [
        (
            Effect::new(PayEnergyEffect::new(
                Value::X,
                ChooseSpec::Player(PlayerFilter::You),
            )),
            2,
        ),
        (Effect::new(PayLifeEffect::you(Value::X)), 3),
    ] {
        let cost = Cost::try_effect(effect).unwrap();
        let mut dm = Choices::default();
        let mut ctx =
            CostContext::new(source, A, &mut dm).with_reason(PaymentReason::ActivateAbility);
        assert!(
            cost.can_pay(&game, &ctx).is_ok(),
            "unannounced X admits the legal minimum zero"
        );
        assert!(
            cost.pay(&mut game, &mut ctx).is_err(),
            "execution never substitutes zero for missing announced X"
        );
        ctx.x_value = Some(available + 1);
        assert!(cost.can_pay(&game, &ctx).is_err());
        assert!(cost.pay(&mut game, &mut ctx).is_err());
        ctx.x_value = Some(available);
        assert!(cost.can_pay(&game, &ctx).is_ok());
        assert_eq!(game.player(A).unwrap().energy_counters, 2);
        assert_eq!(game.player(A).unwrap().life, 3);
        assert_eq!(game.player(A).unwrap().mana_pool, pool);
    }
}
