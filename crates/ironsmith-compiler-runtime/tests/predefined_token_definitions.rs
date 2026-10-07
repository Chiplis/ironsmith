//! Source-authored, UNRUN complete predefined-token scenarios.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::color::Color;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, ColorsContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::{
    ManaPaymentRequest, check_mana_payment, execute_mana_payment_plan, plan_first_mana_payment,
};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, CounterType, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/predefined_token_definitions.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> Vec<CardDefinition> {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("independent direct route {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "{name}: {}", direct_loss.reasons_text());
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    vec![direct, materialize_artifact(&restored).unwrap()]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn card(game: &mut GameState, owner: PlayerId, name: &str, text: &str, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn fund(game: &mut GameState, player: PlayerId, amount: u32) {
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(player)
            .unwrap()
            .mana_pool
            .add(symbol, amount);
    }
}
#[derive(Default)]
struct Choices {
    target: Option<Target>,
    objects: Vec<ObjectId>,
    option: Option<String>,
    decline: bool,
    color: Option<Color>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, context: &BooleanContext) -> bool {
        !self.decline && context.can_accept
    }
    fn decide_colors(&mut self, game: &GameState, context: &ColorsContext) -> Vec<Color> {
        if let Some(color) = self.color {
            assert!(
                context
                    .available_colors
                    .as_ref()
                    .is_none_or(|colors| colors.contains(&color))
            );
            vec![color; context.count as usize]
        } else {
            SelectFirstDecisionMaker.decide_colors(game, context)
        }
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if let Some(needle) = &self.option
            && let Some(option) = context.options.iter().find(|option| {
                option.legal && option.description.to_ascii_lowercase().contains(needle)
            })
        {
            vec![option.index]
        } else {
            SelectFirstDecisionMaker.decide_options(game, context)
        }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target
            && context
                .requirements
                .iter()
                .all(|requirement| requirement.legal_targets.contains(&target))
        {
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if !self.objects.is_empty() {
            assert!(
                self.objects.iter().all(|id| context
                    .candidates
                    .iter()
                    .any(|candidate| candidate.id == *id && candidate.legal)),
                "authored selection must be legal"
            );
            self.objects.clone()
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::ManaPaymentContext,
    ) -> ironsmith::mana_payment::ManaPaymentResponse {
        SelectFirstDecisionMaker.decide_mana_payment(game, context)
    }
}
fn tokens(game: &GameState, name: &str) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                matches!(object.kind, ironsmith::object::ObjectKind::Token)
                    && object.name.as_ref() == name
            })
        })
        .collect()
}
fn settle(game: &mut GameState, queue: &mut TriggerQueue, choices: &mut Choices) {
    for _ in 0..40 {
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, queue, choices).unwrap();
        if game.stack_is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, choices).unwrap();
        ironsmith::rules::state_based::apply_state_based_actions_with(game, choices).unwrap();
    }
    panic!("bounded token fixture did not settle");
}
fn enter(
    game: &mut GameState,
    definition: &CardDefinition,
    owner: PlayerId,
    choices: &mut Choices,
) -> ObjectId {
    let source = game.create_object_from_definition(definition, owner, Zone::Hand);
    let receipt = game
        .move_object_with_etb_processing_with_dm(source, Zone::Battlefield, choices)
        .unwrap();
    assert!(!receipt.pending);
    assert!(receipt.programs.is_empty());
    let source = receipt
        .original
        .into_result()
        .expect("entry commits")
        .new_id;
    settle(game, &mut TriggerQueue::new(), choices);
    source
}
fn announce(game: &mut GameState, player: PlayerId, action: LegalAction, choices: &mut Choices) {
    game.turn.priority_player = Some(player);
    assert!(
        compute_legal_actions(game, player)
            .unwrap()
            .contains(&action)
    );
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        choices,
    )
    .unwrap();
    for _ in 0..50 {
        if !state.has_pending_action() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("missing pending decision");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices)
            .unwrap();
    }
    assert!(!state.has_pending_action());
    settle(game, &mut queue, choices);
}
fn activate(game: &mut GameState, source: ObjectId, nth: usize, choices: &mut Choices) {
    let player = game.current_controller(source).unwrap();
    game.turn.priority_player = Some(player);
    let ability_index = game
        .current_abilities(source)
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, ability)| matches!(ability.kind, AbilityKind::Activated(_)))
        .nth(nth)
        .unwrap()
        .0;
    let action = compute_legal_actions(game, player).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility {source: id, ability_index: index} | LegalAction::ActivateManaAbility {source: id, ability_index: index} if *id == source && *index == ability_index)).unwrap();
    announce(game, player, action, choices);
}
fn cast(game: &mut GameState, source: ObjectId, choices: &mut Choices) -> Option<ObjectId> {
    let stable = game.object(source).unwrap().stable_id;
    let player = game.object(source).unwrap().owner;
    game.turn.priority_player = Some(player);
    let action = compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .find(
            |action| matches!(action, LegalAction::CastSpell {spell_id, ..} if *spell_id == source),
        )
        .unwrap();
    announce(game, player, action, choices);
    game.find_object_by_stable_id(stable)
}
fn attack(game: &mut GameState, source: ObjectId, choices: &mut Choices) {
    game.remove_summoning_sickness(source);
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    game.mark_combat_phase_started();
    let mut combat = ironsmith::combat_state::CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(
        game,
        &mut combat,
        &mut queue,
        &[ironsmith::decision::AttackerDeclaration {
            creature: source,
            target: ironsmith::combat_state::AttackTarget::Player(B),
        }],
    )
    .unwrap();
    game.combat = Some(combat);
    settle(game, &mut queue, choices);
}
fn cleanup(game: &mut GameState) {
    ironsmith::turn::execute_cleanup_step(game);
    game.refresh_continuous_state().unwrap();
}
#[test]
fn twelve_full_frozen_programs_compile_strictly_and_round_trip_complete_artifacts() {
    let rows = rows();
    assert_eq!(rows.len(), 12);
    for row in rows {
        assert_eq!(definitions(row["name"].as_str().unwrap()).len(), 2);
    }
}
fn create_named(game: &mut GameState, name: &str, expected_name: &str, restored: bool) -> ObjectId {
    let (artifact, direct) = compile_to_artifact(
        "Canonical token fixture",
        format!("Type: Sorcery\nCreate a {name} token."),
        false,
    )
    .unwrap();
    let definition = if restored {
        materialize_artifact(
            &CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap(),
        )
        .unwrap()
    } else {
        direct
    };
    let source = game.create_object_from_definition(&definition, A, Zone::Stack);
    let program = definition.spell_effect.as_ref().unwrap();
    assert_eq!(program.segments.len(), 1);
    assert_eq!(program.segments[0].default_effects.len(), 1);
    execute_effect(
        game,
        &program.segments[0].default_effects[0],
        &mut EffectContext::new_default(source, A),
    )
    .unwrap();
    *tokens(game, expected_name).last().unwrap()
}
fn mana_request(
    source: ObjectId,
    reason: ironsmith::costs::PaymentReason,
    symbol: ManaSymbol,
) -> ManaPaymentRequest {
    ManaPaymentRequest::new(A, source, reason, ManaCost::from_pips(vec![vec![symbol]]))
}
#[test]
fn eight_predefined_artifact_creators_execute_real_entry_and_optional_sacrifice_bodies() {
    for (name, token_name, count) in [
        ("Aerid Konstrari", "Heartwood Token", 1),
        ("Hungering Puppetbeast", "Heartwood Token", 1),
        ("Tenured Tethermage", "Heartwood Token", 2),
        ("Dora Milaje Elite", "Vibranium Token", 1),
        ("Shuri's Fabricator", "Vibranium Token", 2),
        ("T'Challa, the Black Panther", "Vibranium Token", 1),
        ("Vibranium Mining Mech", "Vibranium Token", 1),
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let mut choices = Choices::default();
            if name == "Tenured Tethermage" {
                let land = card(
                    &mut game,
                    A,
                    "Offered land",
                    "Type: Land",
                    Zone::Battlefield,
                );
                choices.objects = vec![land];
            }
            if name == "Dora Milaje Elite" {
                card(&mut game, B, "More land", "Type: Land", Zone::Battlefield);
            }
            enter(&mut game, &definition, A, &mut choices);
            let made = tokens(&game, token_name);
            assert_eq!(made.len(), count, "{name}");
            for token in made {
                let chars = game.current_characteristics(token).unwrap();
                assert_eq!(chars.card_types.as_slice(), &[CardType::Artifact]);
                assert!(chars.mana_cost.is_none());
                if token_name == "Heartwood Token" {
                    assert!(chars.subtypes.contains(&ironsmith::Subtype::Heartwood));
                    assert_eq!(
                        chars.colors,
                        ironsmith::ColorSet::RED.union(ironsmith::ColorSet::GREEN)
                    );
                    assert_eq!(game.is_tapped(token), name == "Tenured Tethermage");
                } else {
                    assert!(chars.subtypes.contains(&ironsmith::Subtype::Vibranium));
                    assert!(chars.colors.is_empty());
                    assert!(game.current_has_static_ability_id(
                        token,
                        ironsmith::static_abilities::StaticAbilityId::Indestructible
                    ));
                    assert!(game.is_tapped(token));
                }
            }
        }
    }
    for definition in definitions("The Great Mound") {
        let mut game = game();
        let land = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        fund(&mut game, A, 3);
        activate(&mut game, land, 1, &mut Choices::default());
        let made = tokens(&game, "Vibranium Token");
        assert_eq!(made.len(), 1);
        assert!(game.is_tapped(made[0]));
    }
}
#[test]
fn optional_or_intervening_entry_qualifiers_do_not_create_artifacts_when_false() {
    for name in ["Tenured Tethermage", "Dora Milaje Elite"] {
        for definition in definitions(name) {
            let mut game = game();
            let mut choices = Choices {
                decline: true,
                ..Default::default()
            };
            if name == "Tenured Tethermage" {
                card(&mut game, A, "Kept land", "Type: Land", Zone::Battlefield);
            }
            enter(&mut game, &definition, A, &mut choices);
            assert!(
                tokens(
                    &game,
                    if name == "Tenured Tethermage" {
                        "Heartwood Token"
                    } else {
                        "Vibranium Token"
                    }
                )
                .is_empty()
            );
        }
    }
}
#[test]
fn heartwood_taps_repeatedly_for_exact_red_or_green_and_has_no_sacrifice_cost() {
    for restored in [false, true] {
        for color in [Color::Red, Color::Green] {
            let mut game = game();
            let token = create_named(&mut game, "Heartwood", "Heartwood Token", restored);
            let green = mana_request(
                token,
                ironsmith::costs::PaymentReason::Other,
                ManaSymbol::Green,
            );
            let blue = mana_request(
                token,
                ironsmith::costs::PaymentReason::Other,
                ManaSymbol::Blue,
            );
            assert!(check_mana_payment(&game, &green).is_ok());
            assert!(check_mana_payment(&game, &blue).is_err());
            activate(
                &mut game,
                token,
                0,
                &mut Choices {
                    color: Some(color),
                    ..Default::default()
                },
            );
            assert_eq!(
                game.player(A)
                    .unwrap()
                    .mana_pool
                    .amount(ManaSymbol::from_color(color)),
                1
            );
            assert!(game.battlefield.contains(&token));
            assert!(game.is_tapped(token));
            game.untap(token);
            activate(
                &mut game,
                token,
                0,
                &mut Choices {
                    color: Some(color),
                    ..Default::default()
                },
            );
            assert_eq!(
                game.player(A)
                    .unwrap()
                    .mana_pool
                    .amount(ManaSymbol::from_color(color)),
                2
            );
        }
    }
}
#[test]
fn vibranium_indestructibility_and_floated_payment_restrictions_execute_from_artifact_payload() {
    for restored in [false, true] {
        for purpose in 0..3 {
            let mut game = game();
            let token = create_named(&mut game, "Vibranium", "Vibranium Token", restored);
            activate(&mut game, token, 0, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().mana_pool.colorless, 1);
            assert!(game.current_has_static_ability_id(
                token,
                ironsmith::static_abilities::StaticAbilityId::Indestructible
            ));
            let destroy = Effect::destroy(ChooseSpec::SpecificObject(token));
            execute_effect(
                &mut game,
                &destroy,
                &mut EffectContext::new_default(token, A),
            )
            .unwrap();
            assert!(game.battlefield.contains(&token));
            game.move_object_by_effect(token, Zone::Exile);
            let (kind, zone, reason, allowed) = match purpose {
                0 => (
                    "Artifact",
                    Zone::Stack,
                    ironsmith::costs::PaymentReason::CastSpell,
                    true,
                ),
                1 => (
                    "Sorcery",
                    Zone::Stack,
                    ironsmith::costs::PaymentReason::CastSpell,
                    false,
                ),
                _ => (
                    "Creature",
                    Zone::Battlefield,
                    ironsmith::costs::PaymentReason::ActivateAbility,
                    true,
                ),
            };
            let source = card(
                &mut game,
                A,
                "Payment recipient",
                &format!(
                    "Mana cost: {{1}}\nType: {kind}{}",
                    if kind == "Creature" {
                        "\nPower/Toughness: 2/2"
                    } else {
                        ""
                    }
                ),
                zone,
            );
            let request = mana_request(source, reason, ManaSymbol::Generic(1));
            assert_eq!(check_mana_payment(&game, &request).is_ok(), allowed);
            if allowed {
                let plan = plan_first_mana_payment(&game, &request).unwrap();
                execute_mana_payment_plan(&mut game, &request, &plan, &mut Choices::default())
                    .unwrap();
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            }
        }
    }
}
#[test]
fn canonical_card_name_tokens_retain_printed_cost_color_type_and_complete_abilities() {
    for restored in [false, true] {
        let mut game = game();
        for (name, value, colors, types, abilities) in [
            (
                "Gingerbrute",
                1,
                ironsmith::ColorSet::COLORLESS,
                vec![CardType::Artifact, CardType::Creature],
                3,
            ),
            (
                "Mutavault",
                0,
                ironsmith::ColorSet::COLORLESS,
                vec![CardType::Land],
                2,
            ),
            (
                "Spellgorger Weird",
                3,
                ironsmith::ColorSet::RED,
                vec![CardType::Creature],
                1,
            ),
            (
                "Tarmogoyf",
                2,
                ironsmith::ColorSet::GREEN,
                vec![CardType::Creature],
                1,
            ),
        ] {
            let token = create_named(&mut game, name, name, restored);
            let object = game.object(token).unwrap();
            assert!(matches!(object.kind, ironsmith::object::ObjectKind::Token));
            assert_eq!(
                object
                    .mana_cost
                    .as_ref()
                    .map_or(0, |cost| cost.mana_value()),
                value
            );
            let chars = game.current_characteristics(token).unwrap();
            assert_eq!(chars.colors, colors);
            assert_eq!(chars.card_types.as_slice(), types.as_slice());
            assert_eq!(chars.abilities.len(), abilities);
        }
    }
}
#[test]
fn aerid_hungering_and_tethermage_pay_real_costs_and_retain_their_non_token_effects() {
    for definition in definitions("Aerid Konstrari") {
        let mut game = game();
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        let power = game.current_power(source).unwrap();
        fund(&mut game, A, 2);
        activate(&mut game, source, 0, &mut Choices::default());
        assert_eq!(tokens(&game, "Heartwood Token").len(), 2);
        assert_eq!(game.current_power(source), Some(power + 2));
        cleanup(&mut game);
        assert_eq!(game.current_power(source), Some(power));
        execute_effect(
            &mut game,
            &Effect::destroy(ChooseSpec::SpecificObject(source)),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        settle(&mut game, &mut TriggerQueue::new(), &mut Choices::default());
        assert_eq!(tokens(&game, "Heartwood Token").len(), 3);
    }
    for definition in definitions("Hungering Puppetbeast") {
        let mut game = game();
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        let food = tokens(&game, "Heartwood Token")[0];
        fund(&mut game, A, 1);
        let mut choices = Choices {
            objects: vec![food],
            option: Some("hexproof".into()),
            ..Default::default()
        };
        activate(&mut game, source, 0, &mut choices);
        assert!(tokens(&game, "Heartwood Token").is_empty());
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1);
        assert!(game.current_has_static_ability_id(
            source,
            ironsmith::static_abilities::StaticAbilityId::Hexproof
        ));
        cleanup(&mut game);
        assert!(!game.current_has_static_ability_id(
            source,
            ironsmith::static_abilities::StaticAbilityId::Hexproof
        ));
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1);
    }
    for definition in definitions("Tenured Tethermage") {
        let mut game = game();
        let land = card(&mut game, A, "Cost land", "Type: Land", Zone::Battlefield);
        let mut choices = Choices {
            objects: vec![land],
            ..Default::default()
        };
        let source = enter(&mut game, &definition, A, &mut choices);
        let made = tokens(&game, "Heartwood Token");
        assert_eq!(made.len(), 2);
        assert!(made.iter().all(|id| game.is_tapped(*id)));
        for id in &made {
            game.untap(*id);
        }
        choices.objects = made.clone();
        activate(&mut game, source, 0, &mut choices);
        assert!(made.iter().all(|id| game.is_tapped(*id)));
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 2);
    }
}
#[test]
fn dora_sacrifice_grants_legendary_indestructibility_and_shuri_returns_the_actual_artifact_with_finality()
 {
    for definition in definitions("Dora Milaje Elite") {
        let mut game = game();
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        let own = card(
            &mut game,
            A,
            "Own legend",
            "Type: Legendary Land",
            Zone::Battlefield,
        );
        let foreign = card(
            &mut game,
            B,
            "Foreign legend",
            "Type: Legendary Land",
            Zone::Battlefield,
        );
        activate(&mut game, source, 0, &mut Choices::default());
        assert!(!game.battlefield.contains(&source));
        assert!(game.current_has_static_ability_id(
            own,
            ironsmith::static_abilities::StaticAbilityId::Indestructible
        ));
        assert!(!game.current_has_static_ability_id(
            foreign,
            ironsmith::static_abilities::StaticAbilityId::Indestructible
        ));
        cleanup(&mut game);
        assert!(!game.current_has_static_ability_id(
            own,
            ironsmith::static_abilities::StaticAbilityId::Indestructible
        ));
    }
    for definition in definitions("Shuri's Fabricator") {
        let mut game = game();
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        let artifact = card(
            &mut game,
            A,
            "Returned artifact",
            "Type: Artifact Creature\nPower/Toughness: 2/2",
            Zone::Graveyard,
        );
        let stable = game.object(artifact).unwrap().stable_id;
        fund(&mut game, A, 2);
        game.turn.active_player = B;
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action|
            matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)), "Shuri's return is sorcery-only");
        game.turn.active_player = A;
        activate(
            &mut game,
            source,
            0,
            &mut Choices {
                target: Some(Target::Object(artifact)),
                ..Default::default()
            },
        );
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.counter_count(returned, CounterType::Finality), 1);
        assert!(game.is_tapped(source));
        execute_effect(
            &mut game,
            &Effect::destroy(ChooseSpec::SpecificObject(returned)),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
    }
}
#[test]
fn tchalla_cast_filter_vehicle_crew_attack_and_great_mound_draw_keep_full_bodies() {
    for definition in definitions("T'Challa, the Black Panther") {
        let mut game = game();
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        fund(&mut game, A, 4);
        for (mana, expected) in [(3, 0), (4, 2)] {
            let spell = card(
                &mut game,
                A,
                "Artifact cast",
                &format!("Mana cost: {{{mana}}}\nType: Artifact"),
                Zone::Hand,
            );
            cast(&mut game, spell, &mut Choices::default());
            assert_eq!(
                game.counter_count(source, CounterType::PlusOnePlusOne),
                expected
            );
        }
        attack(&mut game, source, &mut Choices::default());
        assert_eq!(tokens(&game, "Vibranium Token").len(), 2);
    }
    for definition in definitions("Vibranium Mining Mech") {
        let mut game = game();
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        let pilot = card(
            &mut game,
            A,
            "Crew pilot",
            "Type: Creature\nPower/Toughness: 2/2",
            Zone::Battlefield,
        );
        activate(
            &mut game,
            source,
            1,
            &mut Choices {
                objects: vec![pilot],
                ..Default::default()
            },
        );
        assert!(game.is_tapped(pilot));
        assert!(game.current_is_creature(source));
        let power = game.current_power(source).unwrap();
        fund(&mut game, A, 1);
        activate(&mut game, source, 0, &mut Choices::default());
        assert_eq!(game.current_power(source), Some(power + 1));
        attack(&mut game, source, &mut Choices::default());
        assert_eq!(tokens(&game, "Vibranium Token").len(), 2);
        cleanup(&mut game);
        assert!(!game.current_is_creature(source));
    }
    for definition in definitions("The Great Mound") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let drawn = card(
            &mut game,
            A,
            "Drawn card",
            "Type: Sorcery\nDraw a card.",
            Zone::Library,
        );
        fund(&mut game, A, 2);
        activate(&mut game, source, 2, &mut Choices::default());
        assert!(game.is_tapped(source));
        assert!(game.player(A).unwrap().hand.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name.as_ref() == "Drawn card")
        }));
        assert!(!game.player(A).unwrap().library.contains(&drawn));
    }
}
#[test]
fn ginger_monarchy_makes_exact_food_creature_and_both_token_activations_work() {
    for definition in definitions("Ginger, Queen of Sweets") {
        let mut game = game();
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        assert!(game.is_monarch(A));
        game.turn.active_player = B;
        game.turn.phase = ironsmith::Phase::Beginning;
        let mut queue = TriggerQueue::new();
        ironsmith::turn_runner::TurnRunner::from_state_for_sync(
            ironsmith::turn_runner::TurnState::Upkeep,
        )
        .advance(&mut game, &mut queue)
        .unwrap();
        settle(&mut game, &mut queue, &mut Choices::default());
        let token = tokens(&game, "Gingerbrute")[0];
        assert!(game.current_has_static_ability_id(
            token,
            ironsmith::static_abilities::StaticAbilityId::Haste
        ));
        let ordinary = card(
            &mut game,
            B,
            "Slow blocker",
            "Type: Creature\nPower/Toughness: 2/2",
            Zone::Battlefield,
        );
        let hasty = card(
            &mut game,
            B,
            "Fast blocker",
            "Type: Creature\nPower/Toughness: 2/2\nHaste",
            Zone::Battlefield,
        );
        fund(&mut game, A, 2);
        activate(&mut game, token, 0, &mut Choices::default());
        assert!(!game.can_block_attacker(ordinary, token));
        assert!(game.can_block_attacker(hasty, token));
        let life = game.player(A).unwrap().life;
        activate(&mut game, token, 1, &mut Choices::default());
        assert!(tokens(&game, "Gingerbrute").is_empty());
        assert_eq!(game.player(A).unwrap().life, life + 3);
        game.remove_summoning_sickness(source);
        activate(&mut game, source, 0, &mut Choices::default());
        assert!(!game.battlefield.contains(&source));
        assert_eq!(game.player(A).unwrap().life, life + 9);
    }
}
#[test]
fn mutable_explorer_creates_a_tapped_land_with_real_mana_and_expiring_all_type_animation() {
    for definition in definitions("Mutable Explorer") {
        let mut game = game();
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        assert!(
            game.current_subtypes(source)
                .unwrap()
                .contains(&ironsmith::Subtype::Human)
        );
        let token = tokens(&game, "Mutavault")[0];
        assert!(game.is_tapped(token));
        game.untap(token);
        activate(&mut game, token, 0, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().mana_pool.colorless, 1);
        activate(&mut game, token, 1, &mut Choices::default());
        assert!(game.current_is_creature(token));
        assert!(
            game.current_card_types(token)
                .unwrap()
                .contains(&CardType::Land)
        );
        assert_eq!(
            (game.current_power(token), game.current_toughness(token)),
            (Some(2), Some(2))
        );
        for subtype in ironsmith::Subtype::all_creature_types() {
            assert!(game.current_subtypes(token).unwrap().contains(subtype));
        }
        cleanup(&mut game);
        assert!(!game.current_is_creature(token));
        assert!(
            game.current_card_types(token)
                .unwrap()
                .contains(&CardType::Land)
        );
    }
}
#[test]
fn ral_saga_keeps_all_chapters_and_the_weird_counts_only_its_controllers_noncreature_casts() {
    for definition in definitions("Ral and the Implicit Maze") {
        let mut game = game();
        let own = card(
            &mut game,
            A,
            "Own creature",
            "Type: Creature\nPower/Toughness: 4/4",
            Zone::Battlefield,
        );
        let enemy = card(
            &mut game,
            B,
            "Enemy creature",
            "Type: Creature\nPower/Toughness: 4/4",
            Zone::Battlefield,
        );
        let planeswalker = card(
            &mut game,
            B,
            "Enemy planeswalker",
            "Type: Planeswalker — Jace\nLoyalty: 5",
            Zone::Battlefield,
        );
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        assert_eq!(game.counter_count(source, CounterType::Lore), 1);
        assert_eq!(game.damage_on(own), 0);
        assert_eq!(game.damage_on(enemy), 2);
        assert_eq!(game.counter_count(planeswalker, CounterType::Loyalty), 3);
        let discard = card(
            &mut game,
            A,
            "Discarded chapter card",
            "Type: Artifact",
            Zone::Hand,
        );
        for name in ["Exiled first", "Exiled second"] {
            card(
                &mut game,
                A,
                name,
                "Mana cost: {0}\nType: Artifact",
                Zone::Library,
            );
        }
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::add_lore_counter_and_check_chapters(&mut game, source, &mut queue)
            .unwrap();
        settle(
            &mut game,
            &mut queue,
            &mut Choices {
                objects: vec![discard],
                ..Default::default()
            },
        );
        let exiled: Vec<_> = game
            .exile
            .iter()
            .copied()
            .filter(|id| game.object(*id).is_some_and(|object| object.owner == A))
            .collect();
        assert_eq!(exiled.len(), 2);
        cast(&mut game, exiled[0], &mut Choices::default());
        ironsmith::game_loop::add_lore_counter_and_check_chapters(&mut game, source, &mut queue)
            .unwrap();
        settle(&mut game, &mut queue, &mut Choices::default());
        let weird = tokens(&game, "Spellgorger Weird")[0];
        assert!(!game.battlefield.contains(&source));
        assert_eq!(game.counter_count(weird, CounterType::PlusOnePlusOne), 0);
        cast(&mut game, exiled[1], &mut Choices::default());
        assert_eq!(
            game.counter_count(weird, CounterType::PlusOnePlusOne),
            1,
            "exile permission survives the Saga's sacrifice"
        );
        let creature = card(
            &mut game,
            A,
            "Creature cast",
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 1/1",
            Zone::Hand,
        );
        cast(&mut game, creature, &mut Choices::default());
        assert_eq!(game.counter_count(weird, CounterType::PlusOnePlusOne), 1);
        let foreign = card(
            &mut game,
            B,
            "Foreign instant",
            "Mana cost: {0}\nType: Instant\nYou gain 1 life.",
            Zone::Hand,
        );
        game.turn.priority_player = Some(B);
        cast(&mut game, foreign, &mut Choices::default());
        assert_eq!(game.counter_count(weird, CounterType::PlusOnePlusOne), 1);
    }
}
#[test]
fn tarmogoyf_nest_grants_to_the_lands_controller_and_token_cda_counts_card_types_across_graveyards()
{
    for definition in definitions("Tarmogoyf Nest") {
        let mut game = game();
        let land = card(
            &mut game,
            B,
            "Enchanted opposing land",
            "Type: Land",
            Zone::Battlefield,
        );
        let aura = game.create_object_from_definition(&definition, A, Zone::Hand);
        fund(&mut game, A, 4);
        let aura = cast(
            &mut game,
            aura,
            &mut Choices {
                target: Some(Target::Object(land)),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            game.object(aura)
                .unwrap()
                .attached_to
                .as_ref()
                .and_then(|target| target.object_id()),
            Some(land)
        );
        fund(&mut game, B, 1);
        activate(&mut game, land, 0, &mut Choices::default());
        let token = tokens(&game, "Tarmogoyf")[0];
        assert_eq!(game.current_controller(token), Some(B));
        assert!(game.is_tapped(land));
        assert_eq!(
            (game.current_power(token), game.current_toughness(token)),
            (Some(0), Some(1))
        );
        card(
            &mut game,
            A,
            "Grave artifact",
            "Type: Artifact",
            Zone::Graveyard,
        );
        let instant = card(
            &mut game,
            B,
            "Grave instant",
            "Type: Instant\nYou gain 1 life.",
            Zone::Graveyard,
        );
        card(
            &mut game,
            B,
            "Grave artifact creature",
            "Type: Artifact Creature\nPower/Toughness: 2/2",
            Zone::Graveyard,
        );
        assert_eq!(
            (game.current_power(token), game.current_toughness(token)),
            (Some(3), Some(4))
        );
        game.move_object_by_effect(instant, Zone::Exile);
        assert_eq!(
            (game.current_power(token), game.current_toughness(token)),
            (Some(2), Some(3))
        );
        let noncard = ironsmith::cards::builders::CardDefinitionBuilder::new(
            ironsmith::CardId::new(),
            "Grave token",
        )
        .token()
        .card_types(vec![CardType::Enchantment])
        .build();
        game.create_object_from_definition(&noncard, A, Zone::Graveyard);
        assert_eq!(
            (game.current_power(token), game.current_toughness(token)),
            (Some(2), Some(3)),
            "tokens are not cards in graveyards"
        );
        let departed = game.move_object_by_effect(token, Zone::Exile).unwrap();
        assert_eq!(
            (
                game.current_power(departed),
                game.current_toughness(departed)
            ),
            (Some(2), Some(3)),
            "the CDA functions outside the battlefield before token cessation"
        );
    }
}
