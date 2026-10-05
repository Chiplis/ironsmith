//! UNVALIDATED: source-scoped activation modifiers, real payment and resolution.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
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
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::AttachmentTarget;
use ironsmith::object::CounterType;
use ironsmith::object::ObjectKind;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::types::Subtype;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/typed_activation_modifiers.json.fixture"
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
#[derive(Default)]
struct Choices {
    object: Option<ObjectId>,
    target: Option<Target>,
    x: u32,
    mode: Option<usize>,
    expected_x_bounds: Option<(u32, u32)>,
    saw_x: bool,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose mode") {
            if let Some(mode) = self.mode {
                assert!(
                    context
                        .options
                        .iter()
                        .any(|option| option.index == mode && option.legal)
                );
                return vec![mode];
            }
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
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
            return vec![
                target;
                context
                    .requirements
                    .iter()
                    .map(|requirement| requirement.min_targets)
                    .sum()
            ];
        }
        SelectFirstDecisionMaker.decide_targets(game, context)
    }
    fn decide_number(&mut self, _game: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value {
            self.saw_x = true;
            if let Some(bounds) = self.expected_x_bounds {
                assert_eq!((context.min, context.max), bounds);
            }
        }
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

fn printed(game: &mut GameState, name: &str, text: &str, owner: PlayerId, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn price(game: &GameState, source: ObjectId, ordinal: usize, targets: &[Target]) -> ManaCost {
    let abilities = game.current_abilities(source).unwrap();
    let (index, activated) = abilities
        .iter()
        .enumerate()
        .filter_map(|(index, ability)| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                Some((index, activated))
            } else {
                None
            }
        })
        .nth(ordinal)
        .unwrap();
    let payer = game.current_controller(source).unwrap();
    let cost = ironsmith::decision::calculate_effective_activation_total_cost_for_ability(
        game,
        payer,
        source,
        &activated.mana_cost,
        targets,
        ironsmith::decision::ActivationCostAbility::at(game, payer, source, index),
    );
    let mut pips = Vec::new();
    for component in cost.costs() {
        if let Some(mana) = component.mana_cost_ref() {
            pips.extend_from_slice(mana.pips());
        }
        assert!(
            component.dynamic_mana_cost_ref().is_none(),
            "price must be locked before payment: {cost:?}"
        );
    }
    ManaCost::from_pips(pips)
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for _ in 0..count {
        creature(game, player, Zone::Library, "Draw witness", None);
    }
}
fn pay(
    game: &mut GameState,
    source: ObjectId,
    ordinal: usize,
    target: Option<Target>,
    symbols: &[ManaSymbol],
) {
    let player = game.current_controller(source).unwrap();
    main_phase(game, player);
    game.remove_summoning_sickness(source);
    for symbol in symbols {
        mana(game, player, *symbol, 1);
    }
    let action = activation(game, player, source, ordinal).unwrap_or_else(|| {
        panic!(
            "no activation for {} ordinal {ordinal}, abilities={:?}",
            game.object(source).unwrap().name,
            game.current_abilities(source)
        )
    });
    announce(
        game,
        action,
        &mut Choices {
            target,
            ..Default::default()
        },
    );
    assert_eq!(
        game.player(player).unwrap().mana_pool.total(),
        0,
        "the actual total was paid"
    );
    resolve_stack_entry_with(game, &mut Choices::default()).unwrap();
}
fn colored(game: &mut GameState, owner: PlayerId, color: ColorSet, power: i32) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Price target")
        .card_types(vec![CardType::Creature])
        .color_indicator(color)
        .power_toughness(PowerToughness::fixed(power, 12))
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}
#[test]
fn fourteen_frozen_complete_cards_strict_compile_and_round_trip_typed_modifiers() {
    fn retains_pricing(model: &ironsmith::static_abilities::CompiledStaticAbility) -> bool {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostReduction { .. }
            | ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostIncrease { .. } => true,
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                retains_pricing(ability)
            }
            _ => false,
        }
    }
    let fixture: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/typed_activation_modifiers.json.fixture"
    ))
    .unwrap();
    assert_eq!(fixture.len(), 14);
    for row in fixture {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert!(definition.abilities.iter().any(|ability| {
                matches!(&ability.kind, AbilityKind::Static(modifier) if modifier.activated_ability_cost_reduction().is_some() || modifier.activated_ability_cost_increase().is_some() || modifier.compiled_model().is_some_and(retains_pricing))
            }), "{} retains executable pricing", row["name"]);
        }
    }
}
#[test]
fn agatha_uses_modifier_power_and_controller_keeps_colored_pips_and_reprices_each_activation() {
    for definition in definitions("Agatha of the Vile Cauldron") {
        let mut game = game();
        let agatha = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = printed(
            &mut game,
            "Large customer",
            "Type: Creature\nPower/Toughness: 9/9\n{4}{R}: You gain 2 life.",
            A,
            Zone::Battlefield,
        );
        assert_eq!(price(&game, source, 0, &[]).to_oracle(), "{3}{R}");
        game.add_counters(agatha, CounterType::PlusOnePlusOne, 4);
        assert_eq!(price(&game, source, 0, &[]).to_oracle(), "{R}");
        let foreign = printed(
            &mut game,
            "Foreign customer",
            "Type: Creature\nPower/Toughness: 9/9\n{4}{R}: You gain 2 life.",
            B,
            Zone::Battlefield,
        );
        assert_eq!(price(&game, foreign, 0, &[]).to_oracle(), "{4}{R}");
        // Wrong-color mana cannot satisfy the colored pip even with excess reduction.
        mana(&mut game, A, ManaSymbol::Colorless, 1);
        assert!(!game.can_pay_mana_cost(A, Some(source), &price(&game, source, 0, &[]), 0));
        game.player_mut(A).unwrap().mana_pool.empty();
        pay(&mut game, source, 0, None, &[ManaSymbol::Red]);
        assert_eq!(game.player(A).unwrap().life, 22);
        // Its own activated body is still complete and uses its current power.
        pay(
            &mut game,
            agatha,
            0,
            None,
            &[ManaSymbol::Red, ManaSymbol::Green],
        );
        assert_eq!(game.current_power(source), Some(10));
        assert_eq!(game.current_power(agatha), Some(5));
    }
}
#[test]
fn equipment_reductions_price_the_actual_target_and_only_the_equip_ability() {
    for name in ["Belt of Giant Strength", "Ghostfire Blade"] {
        for definition in definitions(name) {
            let mut game = game();
            let equipment = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let eligible = colored(&mut game, A, ColorSet::COLORLESS, 9);
            let other = colored(&mut game, A, ColorSet::RED, 1);
            assert_eq!(
                price(&game, equipment, 0, &[Target::Object(eligible)]).mana_value(),
                1
            );
            let expensive = if name == "Belt of Giant Strength" {
                9
            } else {
                3
            };
            assert_eq!(
                price(&game, equipment, 0, &[Target::Object(other)]).mana_value(),
                expensive
            );
            pay(
                &mut game,
                equipment,
                0,
                Some(Target::Object(eligible)),
                &[ManaSymbol::Colorless],
            );
            assert_eq!(
                game.object(equipment).unwrap().attached_to,
                Some(ironsmith::object::AttachmentTarget::Object(eligible))
            );
            assert_eq!(
                game.current_power(eligible),
                Some(if name == "Belt of Giant Strength" {
                    10
                } else {
                    11
                })
            );
            // Next operation re-prices its new target, not the prior attached host.
            assert_eq!(
                price(&game, equipment, 0, &[Target::Object(other)]).mana_value(),
                expensive
            );
        }
    }
}
#[test]
fn power_artifact_tracks_its_exact_host_and_combines_generic_reducers_with_a_one_mana_floor() {
    for definition in definitions("Power Artifact") {
        let mut game = game();
        let source = printed(
            &mut game,
            "Host",
            "Type: Artifact\n{4}: You gain 2 life.\n{1}{U}: You gain 3 life.",
            B,
            Zone::Battlefield,
        );
        let unrelated = printed(
            &mut game,
            "Unrelated",
            "Type: Artifact\n{4}: You gain 2 life.",
            B,
            Zone::Battlefield,
        );
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(source)));
        assert_eq!(price(&game, source, 0, &[]).mana_value(), 2);
        assert_eq!(price(&game, unrelated, 0, &[]).mana_value(), 4);
        printed(
            &mut game,
            "Second reducer",
            "Type: Enchantment\nActivated abilities of artifacts cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana.",
            A,
            Zone::Battlefield,
        );
        assert_eq!(price(&game, source, 0, &[]).mana_value(), 1);
        assert_eq!(price(&game, source, 1, &[]).to_oracle(), "{U}");
        pay(&mut game, source, 1, None, &[ManaSymbol::Blue]);
        assert_eq!(game.player(B).unwrap().life, 23);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}
#[test]
fn source_bound_conditional_reducers_keep_ability_identity_and_graveyard_zone() {
    for definition in definitions("Hylda's Crown of Winter") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = creature(&mut game, B, Zone::Battlefield, "Tap target", None);
        assert_eq!(
            price(&game, source, 0, &[Target::Object(target)]).mana_value(),
            0
        );
        assert_eq!(
            price(&game, source, 1, &[]).mana_value(),
            3,
            "the sibling sacrifice ability is not discounted"
        );
        main_phase(&mut game, B);
        assert_eq!(
            price(&game, source, 0, &[Target::Object(target)]).mana_value(),
            1
        );
        pay(&mut game, source, 0, Some(Target::Object(target)), &[]);
        assert!(game.is_tapped(target));
    }
    for definition in definitions("Razorlash Transmogrant") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        assert_eq!(price(&game, source, 0, &[]).to_oracle(), "{4}{B}{B}");
        for n in 0..4 {
            printed(
                &mut game,
                &format!("Nonbasic {n}"),
                "Type: Land",
                B,
                Zone::Battlefield,
            );
        }
        assert_eq!(price(&game, source, 0, &[]).to_oracle(), "{B}{B}");
        let stable = game.object(source).unwrap().stable_id;
        pay(
            &mut game,
            source,
            0,
            None,
            &[ManaSymbol::Black, ManaSymbol::Black],
        );
        let returned = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().stable_id == stable)
            .unwrap();
        assert_ne!(returned, source);
        assert_eq!(game.counter_count(returned, CounterType::PlusOnePlusOne), 1);
    }
}
#[test]
fn source_conditional_counts_and_monarch_gate_change_prices_before_real_payment() {
    for name in [
        "Crown of Gondor",
        "Esquire of the King",
        "Starport Security",
        "Sewer Crocodile",
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let target = colored(&mut game, A, ColorSet::GREEN, 2);
            let before = price(&game, source, 0, &[Target::Object(target)]).mana_value();
            let payment = match name {
                "Crown of Gondor" => {
                    game.set_monarch(Some(A))
                        .expect("checked designation/departure fixture");
                    vec![ManaSymbol::Colorless]
                }
                "Esquire of the King" => {
                    printed(
                        &mut game,
                        "Legend",
                        "Type: Legendary Creature — Human\nPower/Toughness: 2/2",
                        A,
                        Zone::Battlefield,
                    );
                    vec![
                        ManaSymbol::Colorless,
                        ManaSymbol::Colorless,
                        ManaSymbol::White,
                    ]
                }
                "Starport Security" => {
                    game.add_counters(target, CounterType::PlusOnePlusOne, 1);
                    vec![ManaSymbol::Colorless, ManaSymbol::White]
                }
                _ => {
                    for mv in 1..=5 {
                        printed(
                            &mut game,
                            &format!("Grave {mv}"),
                            &format!("Mana cost: {{{mv}}}\nType: Sorcery\nYou gain 1 life."),
                            A,
                            Zone::Graveyard,
                        );
                    }
                    vec![ManaSymbol::Blue]
                }
            };
            let after = price(&game, source, 0, &[Target::Object(target)]).mana_value();
            assert!(after < before, "{name}");
            pay(
                &mut game,
                source,
                0,
                matches!(name, "Crown of Gondor" | "Starport Security")
                    .then_some(Target::Object(target)),
                &payment,
            );
            match name {
                "Crown of Gondor" => assert_eq!(
                    game.object(source).unwrap().attached_to,
                    Some(ironsmith::object::AttachmentTarget::Object(target))
                ),
                "Esquire of the King" => assert_eq!(game.current_power(target), Some(3)),
                "Starport Security" => assert!(game.is_tapped(target)),
                _ => {
                    game.update_cant_effects();
                    assert!(!game.can_be_blocked(source));
                }
            }
        }
    }
}
#[test]
fn loreseeker_surcharge_counts_hand_at_each_announcement_and_draws_only_after_payment() {
    for definition in definitions("Loreseeker's Stone") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..2 {
            creature(&mut game, A, Zone::Hand, "Hand card", None);
        }
        creature(&mut game, B, Zone::Hand, "Opponent hand", None);
        library(&mut game, A, 6);
        assert_eq!(price(&game, source, 0, &[]).mana_value(), 5);
        pay(&mut game, source, 0, None, &vec![ManaSymbol::Colorless; 5]);
        assert_eq!(game.player(A).unwrap().hand.len(), 5);
        game.untap(source);
        assert_eq!(price(&game, source, 0, &[]).mana_value(), 8);
        pay(&mut game, source, 0, None, &vec![ManaSymbol::Colorless; 8]);
        assert_eq!(game.player(A).unwrap().hand.len(), 8);
    }
}
#[test]
fn dynamic_matching_object_values_count_the_right_sources_and_keep_full_bodies() {
    for definition in definitions("Baru, Wurmspeaker") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(price(&game, source, 0, &[]).mana_value(), 8);
        printed(
            &mut game,
            "Foreign Wurm",
            "Type: Creature — Wurm\nPower/Toughness: 20/20",
            B,
            Zone::Battlefield,
        );
        let wurm = printed(
            &mut game,
            "Own Wurm",
            "Type: Creature — Wurm\nPower/Toughness: 5/5",
            A,
            Zone::Battlefield,
        );
        assert_eq!(game.current_power(wurm), Some(7));
        assert_eq!(price(&game, source, 0, &[]).to_oracle(), "{G}");
        pay(&mut game, source, 0, None, &[ManaSymbol::Green]);
        let token = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().kind == ObjectKind::Token)
            .unwrap();
        assert!(
            game.object(token)
                .unwrap()
                .subtypes
                .contains(&Subtype::Wurm)
        );
        assert_eq!(game.current_power(token), Some(6));
    }
    for definition in definitions("Survey Mechan") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for name in ["Duplicate", "Duplicate", "Distinct"] {
            printed(&mut game, name, "Type: Land", A, Zone::Battlefield);
        }
        printed(
            &mut game,
            "Foreign land",
            "Type: Land",
            B,
            Zone::Battlefield,
        );
        assert_eq!(
            price(&game, source, 0, &[Target::Player(B)]).mana_value(),
            8
        );
        library(&mut game, B, 3);
        pay(
            &mut game,
            source,
            0,
            Some(Target::Player(B)),
            &vec![ManaSymbol::Colorless; 8],
        );
        assert!(!game.battlefield.contains(&source));
        assert_eq!(game.player(B).unwrap().hand.len(), 3);
        assert_eq!(
            game.player(B).unwrap().life,
            20,
            "three damage and three life are both resolved"
        );
    }
}

#[test]
fn attack_history_reduction_uses_the_actor_and_historical_spacecraft_even_after_it_leaves() {
    for definition in definitions("Thaumaton Torpedo") {
        for actor in [A, B] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let target = colored(&mut game, B, ColorSet::RED, 4);
            let ship = printed(
                &mut game,
                "Attacking Spacecraft",
                "Type: Artifact Creature — Spacecraft\nPower/Toughness: 3/3",
                actor,
                Zone::Battlefield,
            );
            game.remove_summoning_sickness(ship);
            game.turn.active_player = actor;
            game.turn.priority_player = Some(actor);
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
            let mut combat = ironsmith::combat_state::CombatState::default();
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::apply_attacker_declarations_with_dm(
                &mut game,
                &mut combat,
                &mut queue,
                &[ironsmith::decision::AttackerDeclaration {
                    creature: ship,
                    target: ironsmith::combat_state::AttackTarget::Player(if actor == A {
                        B
                    } else {
                        A
                    }),
                }],
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            game.combat = Some(combat);
            let mut dm = SelectFirstDecisionMaker;
            let mut context = EffectContext::new(source, A, &mut dm);
            execute_effect(
                &mut game,
                &Effect::destroy(ChooseSpec::SpecificObject(ship)),
                &mut context,
            )
            .unwrap();
            let amount = if actor == A { 3 } else { 6 };
            assert_eq!(
                price(&game, source, 0, &[Target::Object(target)]).mana_value(),
                amount
            );
            pay(
                &mut game,
                source,
                0,
                Some(Target::Object(target)),
                &vec![ManaSymbol::Colorless; amount as usize],
            );
            assert!(!game.battlefield.contains(&source));
            assert!(!game.battlefield.contains(&target));
        }
    }
}
#[test]
fn conditional_target_cost_gate_never_discounts_a_sibling_ability_and_queries_are_read_only() {
    let mut game = game();
    let source = printed(
        &mut game,
        "Scoped equip",
        "Type: Artifact — Equipment\nEquip {3}. This ability costs {2} less to activate if it targets a colorless creature.\n{4}: Target creature gets +1/+1 until end of turn.",
        A,
        Zone::Battlefield,
    );
    let target = colored(&mut game, A, ColorSet::COLORLESS, 1);
    assert_eq!(
        price(&game, source, 0, &[Target::Object(target)]).mana_value(),
        1
    );
    assert_eq!(
        price(&game, source, 1, &[Target::Object(target)]).mana_value(),
        4
    );
    let before = format!("{:?}", game.player(A).unwrap().mana_pool);
    let ids = game.next_object_id_counter();
    for _ in 0..3 {
        compute_legal_actions(&game, A).unwrap();
    }
    assert_eq!(format!("{:?}", game.player(A).unwrap().mana_pool), before);
    assert_eq!(game.next_object_id_counter(), ids);
    assert!(!game.is_tapped(source));
}

#[test]
fn cancellation_keeps_the_hand_and_tap_state_when_a_dynamic_total_was_announced() {
    struct Cancel;
    impl DecisionMaker for Cancel {
        fn decide_mana_payment(
            &mut self,
            _game: &GameState,
            _context: &ironsmith::decisions::context::ManaPaymentContext,
        ) -> ironsmith::mana_payment::ManaPaymentResponse {
            ironsmith::mana_payment::ManaPaymentResponse::Cancel
        }
    }
    for definition in definitions("Loreseeker's Stone") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        creature(&mut game, A, Zone::Hand, "Hand witness", None);
        library(&mut game, A, 3);
        mana(&mut game, A, ManaSymbol::Colorless, 4);
        let action = activation(&game, A, source, 0).unwrap();
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        let mut dm = Cancel;
        let mut result = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        );
        for _ in 0..30 {
            if state.pending_activation.is_none() || result.is_err() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(context) = result.unwrap() else {
                panic!("pending payment needs a decision");
            };
            result = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            );
        }
        assert!(state.pending_activation.is_none());
        assert!(game.stack.is_empty());
        assert!(!game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 4);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(game.player(A).unwrap().library.len(), 3);
    }
}
#[test]
fn old_activation_increase_artifacts_default_new_scope_and_surface_fields() {
    let (artifact, _) = compile_to_artifact(
        "Existing surcharge",
        "Type: Enchantment\nActivated abilities of creatures cost {1} more to activate.",
        false,
    )
    .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&artifact.to_json().unwrap()).unwrap();
    fn remove_optional_fields(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if let Some(serde_json::Value::Object(increase)) =
                    object.get_mut("ActivatedAbilityCostIncrease")
                {
                    assert_eq!(
                        increase.remove("ability_condition"),
                        Some(serde_json::Value::Null)
                    );
                    assert_eq!(increase.remove("display"), Some(serde_json::Value::Null));
                }
                for value in object.values_mut() {
                    remove_optional_fields(value);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    remove_optional_fields(value);
                }
            }
            _ => {}
        }
    }
    remove_optional_fields(&mut json);
    let old = CompiledCardArtifact::from_json(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(old, artifact);
    materialize_artifact(&old).unwrap();
}

#[test]
fn conditional_surcharge_checks_its_predicate_and_does_not_tax_sibling_abilities() {
    for condition_true in [false, true] {
        let mut game = game();
        let source = printed(
            &mut game,
            "Conditional surcharge",
            "Type: Artifact\n{2}: You gain 1 life. This ability costs {1} more to activate if you control a legendary creature.\n{2}: You gain 2 life.",
            A,
            Zone::Battlefield,
        );
        let legend = if condition_true {
            printed(
                &mut game,
                "Legend witness",
                "Type: Legendary Creature\nPower/Toughness: 2/2",
                A,
                Zone::Battlefield,
            )
        } else {
            printed(
                &mut game,
                "Foreign legend",
                "Type: Legendary Creature\nPower/Toughness: 2/2",
                B,
                Zone::Battlefield,
            )
        };
        let expected = if condition_true { 3 } else { 2 };
        assert_eq!(price(&game, source, 0, &[]).mana_value(), expected);
        assert_eq!(price(&game, source, 1, &[]).mana_value(), 2);
        pay(
            &mut game,
            source,
            0,
            None,
            &vec![ManaSymbol::Colorless; expected as usize],
        );
        assert_eq!(game.player(A).unwrap().life, 21);
        if condition_true {
            let mut dm = SelectFirstDecisionMaker;
            let mut context = EffectContext::new(source, A, &mut dm);
            execute_effect(
                &mut game,
                &Effect::destroy(ChooseSpec::SpecificObject(legend)),
                &mut context,
            )
            .unwrap();
            assert_eq!(
                price(&game, source, 0, &[]).mana_value(),
                2,
                "the condition turning false removes the surcharge on a new operation"
            );
            pay(
                &mut game,
                source,
                0,
                None,
                &[ManaSymbol::Colorless, ManaSymbol::Colorless],
            );
            assert_eq!(game.player(A).unwrap().life, 22);
        }
    }
}
