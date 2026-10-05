//! UNVALIDATED public announcement identities, referenced prices and true payment.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/prospective_cost_references.json.fixture"
    ))
    .unwrap();
    let card = fixtures.iter().find(|card| card["name"] == name).unwrap();
    let text = card["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    main_phase(&mut game, A);
    game
}
fn main_phase(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}
fn mana(game: &mut GameState, player: PlayerId, symbol: ManaSymbol, count: u32) {
    game.player_mut(player)
        .unwrap()
        .mana_pool
        .add(symbol, count);
}
fn creature(
    game: &mut GameState,
    owner: PlayerId,
    zone: Zone,
    name: &str,
    cost: Option<ManaCost>,
) -> ObjectId {
    let mut builder = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3));
    if let Some(cost) = cost {
        builder = builder.mana_cost(cost);
    }
    game.create_object_from_card(&builder.build(), owner, zone)
}
fn cost(symbols: Vec<ManaSymbol>) -> ManaCost {
    ManaCost::from_symbols(symbols)
}
#[derive(Default)]
struct Choices {
    object: Option<ObjectId>,
    target: Option<Target>,
    x: u32,
    saw_reference: bool,
    branch: Option<usize>,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.contains("Choose an activation cost") {
            if let Some(branch) = self.branch {
                return vec![branch];
            }
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if context.description.contains("determine this activation") {
            self.saw_reference = true;
        }
        if let Some(id) = self.object.filter(|id| {
            context
                .candidates
                .iter()
                .any(|candidate| candidate.id == *id && candidate.legal)
        }) {
            return vec![id];
        }
        SelectFirstDecisionMaker.decide_objects(game, context)
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert!(
                context
                    .requirements
                    .iter()
                    .all(|requirement| requirement.legal_targets.contains(&target))
            );
            return vec![target];
        }
        SelectFirstDecisionMaker.decide_targets(game, context)
    }
    fn decide_number(&mut self, _game: &GameState, context: &NumberContext) -> u32 {
        assert!(self.x >= context.min && self.x <= context.max);
        self.x
    }
}
fn announce(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        choices,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() && state.pending_cast.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("pending announcement: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices)
            .unwrap();
    }
    assert!(state.pending_activation.is_none() && state.pending_cast.is_none());
    assert_eq!(game.stack.len(), 1);
}
fn activation(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    index: usize,
) -> Option<LegalAction> {
    let index = game
        .current_abilities(source)?
        .iter()
        .enumerate()
        .filter(|(_, ability)| {
            matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))
        })
        .nth(index)?
        .0;
    compute_legal_actions(game, player).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, ability_index } if *id == source && *ability_index == index))
}
fn cast(game: &mut GameState, spell: ObjectId, choices: &mut Choices) {
    let player = game.turn.priority_player.unwrap();
    let action = compute_legal_actions(game, player).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).unwrap();
    announce(game, action, choices);
    resolve_stack_entry_with(game, choices).unwrap();
}
#[test]
fn brink_prices_the_announced_graveyard_card_and_copies_that_exact_exile() {
    for definition in definitions("Back from the Brink") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let expensive = creature(
            &mut game,
            A,
            Zone::Graveyard,
            "Expensive",
            Some(cost(vec![ManaSymbol::Blue, ManaSymbol::Blue])),
        );
        let chosen = creature(
            &mut game,
            A,
            Zone::Graveyard,
            "Chosen",
            Some(cost(vec![ManaSymbol::Generic(1), ManaSymbol::Green])),
        );
        let no_cost = creature(&mut game, A, Zone::Graveyard, "No mana cost", None);
        let stable = game.object(chosen).unwrap().stable_id;
        mana(&mut game, A, ManaSymbol::Green, 1);
        mana(&mut game, A, ManaSymbol::Colorless, 1);
        let action = activation(&game, A, source, 0).unwrap();
        let mut choices = Choices {
            object: Some(chosen),
            ..Default::default()
        };
        announce(&mut game, action, &mut choices);
        assert!(
            choices.saw_reference,
            "a real public choice precedes referenced mana pricing"
        );
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.object(expensive).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(no_cost).unwrap().zone, Zone::Graveyard);
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        let tokens: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|object| object.kind == ironsmith::ObjectKind::Token)
            .collect();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name.to_string(), "Chosen");
        assert_eq!(game.current_power(tokens[0].id), Some(2));
    }
}
#[test]
fn brink_uses_zero_for_referenced_x_and_keeps_no_mana_cost_unpayable() {
    for definition in definitions("Back from the Brink") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        creature(&mut game, A, Zone::Graveyard, "Unpayable", None);
        assert!(activation(&game, A, source, 0).is_none());
        let chosen = creature(
            &mut game,
            A,
            Zone::Graveyard,
            "X green",
            Some(cost(vec![ManaSymbol::X, ManaSymbol::Green])),
        );
        mana(&mut game, A, ManaSymbol::Green, 1);
        let action = activation(&game, A, source, 0).unwrap();
        announce(
            &mut game,
            action,
            &mut Choices {
                object: Some(chosen),
                ..Default::default()
            },
        );
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn merseine_uses_the_host_cost_and_live_host_controller_permission() {
    for definition in definitions("Merseine") {
        let mut game = game();
        let host = creature(
            &mut game,
            B,
            Zone::Battlefield,
            "Host",
            Some(cost(vec![ManaSymbol::Green])),
        );
        let aura = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(aura).unwrap().stable_id;
        mana(&mut game, A, ManaSymbol::Blue, 2);
        mana(&mut game, A, ManaSymbol::Colorless, 2);
        cast(
            &mut game,
            aura,
            &mut Choices {
                target: Some(Target::Object(host)),
                ..Default::default()
            },
        );
        let aura = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.counter_count(aura, CounterType::Net), 3);
        assert_eq!(game.current_controller(aura).unwrap(), A);
        mana(&mut game, A, ManaSymbol::Green, 3);
        mana(&mut game, B, ManaSymbol::Green, 3);
        assert!(activation(&game, A, aura, 0).is_none());
        game.turn.active_player = B;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Untap);
        game.tap(host);
        ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(
            game.is_tapped(host),
            "net counters still prevent the host's normal untap"
        );
        main_phase(&mut game, B);
        for left in (0..3).rev() {
            let action = activation(&game, B, aura, 0).unwrap();
            announce(&mut game, action, &mut Choices::default());
            resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.counter_count(aura, CounterType::Net), left);
        }
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
        game.set_current_controller(host, A).unwrap();
        main_phase(&mut game, A);
        assert!(activation(&game, A, aura, 0).is_some());
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Untap);
        game.tap(host);
        ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!game.is_tapped(host));
    }
}
#[test]
fn veteran_voice_excludes_the_exact_enchanted_cost_object_before_payment() {
    for definition in definitions("Veteran's Voice") {
        let mut game = game();
        let host = creature(
            &mut game,
            A,
            Zone::Battlefield,
            "Host",
            Some(ManaCost::new()),
        );
        let aura = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(aura).unwrap().stable_id;
        mana(&mut game, A, ManaSymbol::Red, 1);
        cast(
            &mut game,
            aura,
            &mut Choices {
                target: Some(Target::Object(host)),
                ..Default::default()
            },
        );
        let aura = game.find_object_by_stable_id(stable).unwrap();
        assert!(
            activation(&game, A, aura, 0).is_none(),
            "the host cannot be its own other target"
        );
        let target = creature(
            &mut game,
            B,
            Zone::Battlefield,
            "Other target",
            Some(ManaCost::new()),
        );
        let action = activation(&game, A, aura, 0).unwrap();
        announce(
            &mut game,
            action,
            &mut Choices {
                target: Some(Target::Object(target)),
                ..Default::default()
            },
        );
        assert!(game.is_tapped(host));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.current_power(target), Some(4));
        assert_eq!(game.current_toughness(target), Some(4));
    }
}
#[test]
fn shelob_pays_with_only_its_current_linked_creature_card_and_returns_it_to_its_owner() {
    for definition in definitions("Shelob, Dread Weaver") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let linked = creature(
            &mut game,
            B,
            Zone::Graveyard,
            "Linked creature",
            Some(cost(vec![ManaSymbol::Generic(2)])),
        );
        let stable = game.object(linked).unwrap().stable_id;
        let mut context = EffectContext::new_default(source, A);
        execute_effect(
            &mut game,
            &Effect::exile(ChooseSpec::SpecificObject(linked)),
            &mut context,
        )
        .unwrap();
        let linked = game.find_object_by_stable_id(stable).unwrap();
        let unrelated = creature(
            &mut game,
            A,
            Zone::Exile,
            "Unrelated",
            Some(ManaCost::new()),
        );
        creature(&mut game, A, Zone::Library, "Drawn", Some(ManaCost::new()));
        mana(&mut game, A, ManaSymbol::Colorless, 2);
        mana(&mut game, A, ManaSymbol::Black, 1);
        let action = activation(&game, A, source, 0).unwrap();
        announce(
            &mut game,
            action,
            &mut Choices {
                object: Some(linked),
                ..Default::default()
            },
        );
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Graveyard
        );
        assert!(
            game.player(B)
                .unwrap()
                .graveyard
                .iter()
                .any(|id| game.object(*id).unwrap().stable_id == stable)
        );
        assert_eq!(game.object(unrelated).unwrap().zone, Zone::Exile);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 2);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        let graveyard_card = game.find_object_by_stable_id(stable).unwrap();
        let later_exile = game
            .move_object_by_effect(graveyard_card, Zone::Exile)
            .unwrap();
        assert_ne!(later_exile, linked);
        mana(&mut game, A, ManaSymbol::Colorless, 2);
        mana(&mut game, A, ManaSymbol::Black, 1);
        assert!(
            activation(&game, A, source, 0).is_none(),
            "an unrelated later exile cannot restore the source link"
        );
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
    }
}
#[test]
fn old_dynamic_mana_payloads_default_the_reference_off() {
    let original = ironsmith_core::DynamicManaCost::from_source_mana_cost();
    let mut json = serde_json::to_value(&original).unwrap();
    json.as_object_mut().unwrap().remove("mana_cost_of");
    let restored: ironsmith_core::DynamicManaCost = serde_json::from_value(json).unwrap();
    assert_eq!(original, restored);
}

fn queue_outcome(game: &mut GameState, outcome: ironsmith::effect::EffectOutcome) {
    let mut queue = TriggerQueue::new();
    // Publish reported receipts through the native queue, which deduplicates
    // aliases of observations that the instruction already queued.
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
}
#[test]
fn fishing_pole_pays_with_its_specific_grantor_then_untapping_the_host_creates_the_fish() {
    for definition in definitions("Fishing Pole") {
        let mut game = game();
        let pole = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let spare = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let host = creature(
            &mut game,
            A,
            Zone::Battlefield,
            "Angler",
            Some(ManaCost::new()),
        );
        game.remove_summoning_sickness(host);
        mana(&mut game, A, ManaSymbol::Colorless, 3);
        let equip = activation(&game, A, pole, 0).unwrap();
        announce(
            &mut game,
            equip,
            &mut Choices {
                target: Some(Target::Object(host)),
                ..Default::default()
            },
        );
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        game.tap(pole);
        assert!(
            activation(&game, A, host, 0).is_none(),
            "a different untapped Fishing Pole cannot pay this granted ability"
        );
        game.untap(pole);
        let action = activation(&game, A, host, 0).unwrap();
        announce(
            &mut game,
            action,
            &mut Choices {
                object: Some(pole),
                ..Default::default()
            },
        );
        assert!(game.is_tapped(host) && game.is_tapped(pole));
        assert!(!game.is_tapped(spare));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(
            game.counter_count(pole, CounterType::Named("bait".into())),
            1
        );
        assert_eq!(
            game.counter_count(spare, CounterType::Named("bait".into())),
            0
        );
        let mut context = EffectContext::new_default(pole, A);
        let outcome = execute_effect(
            &mut game,
            &Effect::untap(ChooseSpec::SpecificObject(host)),
            &mut context,
        )
        .unwrap();
        queue_outcome(&mut game, outcome);
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(
            game.counter_count(pole, CounterType::Named("bait".into())),
            0
        );
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == ironsmith::ObjectKind::Token
                    && object.subtypes.contains(&ironsmith::Subtype::Fish)
            })
        }));
    }
}
#[test]
fn public_reference_prompt_can_cancel_without_moving_cards_or_spending_resources() {
    for definition in definitions("Back from the Brink") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = creature(
            &mut game,
            A,
            Zone::Graveyard,
            "First",
            Some(cost(vec![ManaSymbol::Green])),
        );
        let second = creature(
            &mut game,
            A,
            Zone::Graveyard,
            "Second",
            Some(cost(vec![ManaSymbol::Green])),
        );
        mana(&mut game, A, ManaSymbol::Green, 1);
        let action = activation(&game, A, source, 0).unwrap();
        let mut state = PriorityLoopState::new(2);
        let progress = apply_priority_response_with_dm(
            &mut game,
            &mut TriggerQueue::new(),
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        let GameProgress::NeedsDecisionCtx(
            ironsmith::decisions::context::DecisionContext::SelectObjects(context),
        ) = progress
        else {
            panic!("reference choice expected: {progress:?}");
        };
        assert_eq!(
            context.selection_identity,
            ironsmith::decisions::context::SelectionIdentity::ObjectId
        );
        assert_eq!(
            context.reveal_policy,
            ironsmith::decisions::context::SelectionRevealPolicy::None
        );
        assert!(state.rollback_action(&mut game));
        assert_eq!(game.object(first).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(second).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
        assert!(game.stack_is_empty());
    }
}
#[test]
fn shelob_death_trigger_and_x_return_keep_current_exile_identity_and_controller() {
    for definition in definitions("Shelob, Dread Weaver") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let victim = creature(
            &mut game,
            B,
            Zone::Battlefield,
            "Victim",
            Some(cost(vec![ManaSymbol::Generic(2)])),
        );
        let stable = game.object(victim).unwrap().stable_id;
        let mut context = EffectContext::new_default(source, A);
        let outcome = execute_effect(
            &mut game,
            &Effect::destroy(ChooseSpec::SpecificObject(victim)),
            &mut context,
        )
        .unwrap();
        queue_outcome(&mut game, outcome);
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        mana(&mut game, A, ManaSymbol::Colorless, 3);
        mana(&mut game, A, ManaSymbol::Black, 1);
        let action = activation(&game, A, source, 1).unwrap();
        announce(
            &mut game,
            action,
            &mut Choices {
                target: Some(Target::Object(exiled)),
                x: 2,
                ..Default::default()
            },
        );
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(returned).unwrap().owner, B);
        assert_eq!(game.current_controller(returned).unwrap(), A);
        assert!(game.is_tapped(returned));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn an_alternative_branch_does_not_inherit_an_unselected_object_reference_or_price() {
    let brink = definitions("Back from the Brink")[0].clone();
    let referenced_cost = brink
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(ability) => Some(ability.mana_cost.clone()),
            _ => None,
        })
        .unwrap();
    let mut definition = compile_to_runtime_definition(
        "Alternative reference control",
        "Type: Artifact\n{3}: You gain 1 life.",
        false,
    )
    .unwrap();
    let ability = definition
        .abilities
        .iter_mut()
        .find_map(|ability| match &mut ability.kind {
            ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
            _ => None,
        })
        .unwrap();
    ability.mana_cost = ironsmith::cost::TotalCost::one_of(vec![
        referenced_cost,
        ironsmith::cost::TotalCost::mana(cost(vec![ManaSymbol::Generic(3)])),
    ]);
    let mut game = game();
    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    let card = creature(
        &mut game,
        A,
        Zone::Graveyard,
        "Unpaid alternative",
        Some(cost(vec![ManaSymbol::Green])),
    );
    mana(&mut game, A, ManaSymbol::Colorless, 3);
    let action = activation(&game, A, source, 0).unwrap();
    announce(
        &mut game,
        action,
        &mut Choices {
            branch: Some(1),
            ..Default::default()
        },
    );
    assert_eq!(game.object(card).unwrap().zone, Zone::Graveyard);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.player(A).unwrap().life, 21);
}

fn with_reference_cost(
    mut definition: CardDefinition,
    ordinary_mana: Option<ManaCost>,
) -> CardDefinition {
    let reference = definitions("Back from the Brink")[0].clone();
    let reference_cost = reference
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(ability) => Some(ability.mana_cost.clone()),
            _ => None,
        })
        .unwrap();
    let mut components = reference_cost.costs().to_vec();
    if let Some(mana) = ordinary_mana {
        components.push(ironsmith::costs::Cost::mana(mana));
    }
    let ability = definition
        .abilities
        .iter_mut()
        .find_map(|ability| match &mut ability.kind {
            ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
            _ => None,
        })
        .unwrap();
    ability.mana_cost = ironsmith::cost::TotalCost::from_costs(components);
    definition
}
#[test]
fn referenced_and_ordinary_mana_share_one_affordability_budget_and_one_actual_payment() {
    let definition = compile_to_runtime_definition(
        "Combined reference price",
        "Type: Artifact\n{0}: You gain 1 life.",
        false,
    )
    .unwrap();
    let definition = with_reference_cost(definition, Some(cost(vec![ManaSymbol::Generic(1)])));
    let mut game = game();
    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    let chosen = creature(
        &mut game,
        A,
        Zone::Graveyard,
        "One more mana",
        Some(cost(vec![ManaSymbol::Generic(1)])),
    );
    mana(&mut game, A, ManaSymbol::Colorless, 1);
    assert!(
        activation(&game, A, source, 0).is_none(),
        "one mana cannot independently pay both one-mana components"
    );
    assert_eq!(game.object(chosen).unwrap().zone, Zone::Graveyard);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
    mana(&mut game, A, ManaSymbol::Colorless, 1);
    let action = activation(&game, A, source, 0).unwrap();
    announce(
        &mut game,
        action,
        &mut Choices {
            object: Some(chosen),
            ..Default::default()
        },
    );
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.player(A).unwrap().life, 21);
}
#[test]
fn reference_cost_modal_preflight_requires_a_legal_selection_without_requiring_unchosen_modes() {
    let definition = compile_to_runtime_definition(
        "Modal reference price",
        "Type: Artifact\n{0}: Choose one —\n• Destroy target creature.\n• You gain 2 life.",
        false,
    )
    .unwrap();
    let definition = with_reference_cost(definition, None);
    let mut game = game();
    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    let chosen = creature(
        &mut game,
        A,
        Zone::Graveyard,
        "Cost creature",
        Some(cost(vec![ManaSymbol::Generic(1)])),
    );
    mana(&mut game, A, ManaSymbol::Colorless, 1);
    let action = activation(&game, A, source, 0)
        .expect("gain-life mode remains legal without a battlefield creature target");
    announce(
        &mut game,
        action,
        &mut Choices {
            object: Some(chosen),
            ..Default::default()
        },
    );
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.player(A).unwrap().life, 22);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
}

#[test]
fn alternative_reference_branches_apply_activation_taxes_or_reductions_once() {
    for reduce in [false, true] {
        let definition = compile_to_runtime_definition(
            "Modified reference branch",
            "Type: Artifact\n{0}: You gain 1 life.",
            false,
        )
        .unwrap();
        let mut definition = with_reference_cost(definition, None);
        let activated = definition
            .abilities
            .iter_mut()
            .find_map(|ability| match &mut ability.kind {
                ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .unwrap();
        activated.mana_cost = ironsmith::cost::TotalCost::one_of(vec![
            activated.mana_cost.clone(),
            ironsmith::cost::TotalCost::mana(cost(vec![ManaSymbol::Generic(9)])),
        ]);
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let modifier = if reduce {
            ironsmith::static_abilities::StaticAbility::reduce_activated_ability_costs(
                ironsmith::ObjectFilter::specific(source),
                2,
                None,
            )
        } else {
            ironsmith::static_abilities::StaticAbility::increase_activated_ability_costs(
                ironsmith::ObjectFilter::specific(source),
                ironsmith::cost::TotalCost::mana(cost(vec![ManaSymbol::Generic(2)])),
            )
        };
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(ironsmith::ability::Ability::static_ability(modifier));
        let chosen = creature(
            &mut game,
            A,
            Zone::Graveyard,
            "Referenced price",
            Some(cost(vec![ManaSymbol::Generic(if reduce { 5 } else { 1 })])),
        );
        mana(&mut game, A, ManaSymbol::Colorless, 2);
        assert!(activation(&game, A, source, 0).is_none());
        assert_eq!(game.object(chosen).unwrap().zone, Zone::Graveyard);
        mana(&mut game, A, ManaSymbol::Colorless, 1);
        let action = activation(&game, A, source, 0).unwrap();
        announce(
            &mut game,
            action,
            &mut Choices {
                object: Some(chosen),
                branch: Some(0),
                ..Default::default()
            },
        );
        assert_eq!(
            game.player(A).unwrap().mana_pool.total(),
            0,
            "(1 + 2) or (5 - 2), each priced once"
        );
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().life, 21);
    }
}

#[test]
fn target_time_repricing_does_not_resolve_an_unchosen_reference_branch() {
    let definition = compile_to_runtime_definition(
        "Targeted alternative reference",
        "Type: Artifact\n{0}: Target creature gets +1/+1 until end of turn.",
        false,
    )
    .unwrap();
    let mut definition = with_reference_cost(definition, None);
    let activated = definition
        .abilities
        .iter_mut()
        .find_map(|ability| match &mut ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .unwrap();
    activated.mana_cost = ironsmith::cost::TotalCost::one_of(vec![
        activated.mana_cost.clone(),
        ironsmith::cost::TotalCost::mana(cost(vec![ManaSymbol::Generic(2)])),
    ]);
    let mut game = game();
    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    let unselected = creature(
        &mut game,
        A,
        Zone::Graveyard,
        "Unselected reference",
        Some(cost(vec![ManaSymbol::Green])),
    );
    let target = creature(
        &mut game,
        B,
        Zone::Battlefield,
        "Actual target",
        Some(ManaCost::new()),
    );
    mana(&mut game, A, ManaSymbol::Colorless, 2);
    let action = activation(&game, A, source, 0).unwrap();
    announce(
        &mut game,
        action,
        &mut Choices {
            branch: Some(1),
            target: Some(Target::Object(target)),
            ..Default::default()
        },
    );
    assert_eq!(game.object(unselected).unwrap().zone, Zone::Graveyard);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.current_power(target), Some(3));
    assert_eq!(game.current_toughness(target), Some(4));
}
