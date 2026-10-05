//! UNVALIDATED source-authored Flash timing scenarios.
#![allow(dead_code)]
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
        "../../../fixtures/flash_timing_permissions.json.fixture"
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
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
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
    targets: Vec<Target>,
    forbidden_target: Option<Target>,
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
        if let Some(forbidden) = self.forbidden_target {
            assert!(
                context
                    .requirements
                    .iter()
                    .all(|requirement| !requirement.legal_targets.contains(&forbidden))
            );
        }
        if !self.targets.is_empty() {
            return context
                .requirements
                .iter()
                .flat_map(|requirement| {
                    self.targets
                        .iter()
                        .copied()
                        .filter(move |target| requirement.legal_targets.contains(target))
                        .take(requirement.min_targets)
                })
                .collect();
        }
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
fn instant_window(game: &mut GameState, caster: PlayerId) {
    game.turn.active_player = B;
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::Step::BeginCombat);
    game.turn.priority_player = Some(caster);
}
fn casts(game: &GameState, caster: PlayerId, card: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, caster)
        .unwrap()
        .into_iter()
        .filter(
            |action| matches!(action, LegalAction::CastSpell {spell_id, ..} if *spell_id == card),
        )
        .collect()
}
fn ordinary_origin(game: &mut GameState, caster: PlayerId, zone: Zone) {
    let grant = ironsmith::grant::GrantSpec::new(
        ironsmith::grant::Grantable::play_from(),
        ironsmith::ObjectFilter::default().owned_by(ironsmith::PlayerFilter::You),
        zone,
    );
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::CardId::new(),
        "Independent origin",
    )
    .card_types(vec![CardType::Enchantment])
    .with_ability(ironsmith::ability::Ability::static_ability(
        ironsmith::StaticAbility::grants(grant),
    ))
    .build();
    game.create_object_from_definition(&definition, caster, Zone::Battlefield);
}
#[test]
fn eight_complete_frozen_bodies_are_strict_artifact_round_trip_gates() {
    assert_eq!(rows().len(), 8);
    for row in rows() {
        assert_eq!(definitions(row["name"].as_str().unwrap()).len(), 2);
    }
}
#[test]
fn global_flash_is_caster_relative_cross_origin_timing_without_new_origin_permission() {
    for (name, types) in [
        ("Quick Sliver", "Creature — Sliver"),
        ("Tidal Barracuda", "Creature — Bear"),
        ("Vernal Equinox", "Enchantment"),
    ] {
        for definition in definitions(name) {
            for caster in [A, B] {
                for zone in [Zone::Hand, Zone::Graveyard, Zone::Exile, Zone::Library] {
                    let mut game = game();
                    let host =
                        game.create_object_from_definition(&definition, A, Zone::Battlefield);
                    fund(&mut game, caster, 10);
                    instant_window(&mut game, caster);
                    let text = format!(
                        "Mana cost: {{2}}\nType: {types}{}",
                        if types.starts_with("Creature") {
                            "\nPower/Toughness: 2/2"
                        } else {
                            ""
                        }
                    );
                    let candidate = card(&mut game, caster, "Flash candidate", &text, zone);
                    assert_eq!(
                        !casts(&game, caster, candidate).is_empty(),
                        zone == Zone::Hand
                    );
                    if zone != Zone::Hand {
                        ordinary_origin(&mut game, caster, zone);
                    }
                    assert!(!casts(&game, caster, candidate).is_empty());
                    game.phase_out(host);
                    assert!(casts(&game, caster, candidate).is_empty());
                    game.phase_in(host);
                    assert!(!casts(&game, caster, candidate).is_empty());
                    let action = casts(&game, caster, candidate).into_iter().next().unwrap();
                    announce(&mut game, caster, action, &mut Choices::default());
                    assert!(game.battlefield.iter().any(|id| {
                        game.object(*id)
                            .is_some_and(|object| object.name.as_ref() == "Flash candidate")
                    }));
                    let made = *game
                        .battlefield
                        .iter()
                        .find(|id| {
                            game.object(**id)
                                .is_some_and(|object| object.name.as_ref() == "Flash candidate")
                        })
                        .unwrap();
                    assert!(
                        !game.current_has_static_ability_id(
                            made,
                            ironsmith::static_abilities::StaticAbilityId::Flash
                        ),
                        "as-though timing is not a battlefield keyword grant"
                    );
                }
            }
        }
    }
}
#[test]
fn global_type_filters_and_barracuda_cast_prohibition_remain_independent() {
    for name in ["Quick Sliver", "Vernal Equinox", "Tidal Barracuda"] {
        for definition in definitions(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            fund(&mut game, A, 8);
            fund(&mut game, B, 8);
            instant_window(&mut game, A);
            let wrong = card(
                &mut game,
                A,
                "Wrong type",
                "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.",
                Zone::Hand,
            );
            assert_eq!(
                !casts(&game, A, wrong).is_empty(),
                name == "Tidal Barracuda"
            );
            if name == "Tidal Barracuda" {
                game.turn.active_player = A;
                game.turn.priority_player = Some(B);
                let instant = card(
                    &mut game,
                    B,
                    "Prohibited instant",
                    "Mana cost: {0}\nType: Instant\nYou gain 1 life.",
                    Zone::Hand,
                );
                assert!(casts(&game, B, instant).is_empty());
                game.phase_out(host);
                assert!(!casts(&game, B, instant).is_empty());
            }
        }
    }
}
#[test]
fn self_state_flash_uses_the_caster_and_actual_live_controlled_permanent() {
    for (name, helper_type) in [("Bard's Company", "Human"), ("Illusion Spinners", "Faerie")] {
        for definition in definitions(name) {
            let mut game = game();
            fund(&mut game, A, 10);
            instant_window(&mut game, A);
            let spell = game.create_object_from_definition(&definition, A, Zone::Graveyard);
            ordinary_origin(&mut game, A, Zone::Graveyard);
            assert!(casts(&game, A, spell).is_empty());
            let helper = card(
                &mut game,
                B,
                "Foreign helper",
                &format!("Type: Creature — {helper_type}\nPower/Toughness: 1/1"),
                Zone::Battlefield,
            );
            assert!(casts(&game, A, spell).is_empty());
            let helper = card(
                &mut game,
                A,
                "Own helper",
                &format!("Type: Creature — {helper_type}\nPower/Toughness: 1/1"),
                Zone::Battlefield,
            );
            assert!(!casts(&game, A, spell).is_empty());
            game.phase_out(helper);
            assert!(casts(&game, A, spell).is_empty());
            game.phase_in(helper);
            assert!(!casts(&game, A, spell).is_empty());
        }
    }
}
#[test]
fn timely_ward_requires_a_legal_commander_target_and_retains_real_attachment() {
    for definition in definitions("Timely Ward") {
        let mut game = game();
        fund(&mut game, A, 10);
        instant_window(&mut game, A);
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let ordinary = card(
            &mut game,
            A,
            "Ordinary creature",
            "Type: Creature\nPower/Toughness: 2/2",
            Zone::Battlefield,
        );
        assert!(casts(&game, A, spell).is_empty());
        let commander = card(
            &mut game,
            A,
            "Commander creature",
            "Type: Legendary Creature\nPower/Toughness: 3/3",
            Zone::Battlefield,
        );
        game.set_as_commander(commander, game.object(commander).unwrap().owner);
        assert!(!casts(&game, A, spell).is_empty());
        game.phase_out(commander);
        assert!(casts(&game, A, spell).is_empty());
        game.phase_in(commander);
        let action = casts(&game, A, spell).into_iter().next().unwrap();
        announce(
            &mut game,
            A,
            action,
            &mut Choices {
                target: Some(Target::Object(commander)),
                forbidden_target: Some(Target::Object(ordinary)),
                ..Default::default()
            },
        );
        let aura = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name.as_ref() == "Timely Ward")
            })
            .unwrap();
        assert_eq!(
            game.object(aura).unwrap().attached_to.unwrap().object_id(),
            Some(commander)
        );
        assert!(game.current_has_static_ability_id(
            commander,
            ironsmith::static_abilities::StaticAbilityId::Indestructible
        ));
        assert!(!game.current_has_static_ability_id(
            ordinary,
            ironsmith::static_abilities::StaticAbilityId::Indestructible
        ));
    }
}
#[test]
fn bard_recruit_keeps_real_draw_discard_result_and_anthem_on_both_entry_and_attack() {
    for definition in definitions("Bard's Company") {
        let mut game = game();
        card(
            &mut game,
            A,
            "Nonland recruit card",
            "Mana cost: {0}\nType: Artifact",
            Zone::Library,
        );
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        let soldiers: Vec<_> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id).is_some_and(|object| {
                    matches!(object.kind, ironsmith::object::ObjectKind::Token)
                        && object.subtypes.contains(&ironsmith::Subtype::Human)
                        && object.subtypes.contains(&ironsmith::Subtype::Soldier)
                })
            })
            .collect();
        assert_eq!(soldiers.len(), 1);
        assert_eq!(
            (
                game.current_power(soldiers[0]),
                game.current_toughness(soldiers[0])
            ),
            (Some(2), Some(2))
        );
        assert_eq!(
            (game.current_power(source), game.current_toughness(source)),
            (Some(2), Some(3))
        );
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        card(
            &mut game,
            A,
            "Land recruit card",
            "Type: Land",
            Zone::Library,
        );
        attack(&mut game, source, &mut Choices::default());
        assert_eq!(
            game.battlefield
                .iter()
                .filter(|id| game.object(**id).is_some_and(|object| matches!(
                    object.kind,
                    ironsmith::object::ObjectKind::Token
                )))
                .count(),
            1
        );
        assert_eq!(game.player(A).unwrap().graveyard.len(), 2);
    }
}
#[test]
fn illusion_spinners_keeps_flying_and_only_untapped_hexproof() {
    for definition in definitions("Illusion Spinners") {
        let mut game = game();
        let source = enter(&mut game, &definition, A, &mut Choices::default());
        assert!(game.current_has_static_ability_id(
            source,
            ironsmith::static_abilities::StaticAbilityId::Flying
        ));
        assert!(game.current_has_static_ability_id(
            source,
            ironsmith::static_abilities::StaticAbilityId::Hexproof
        ));
        execute_effect(
            &mut game,
            &Effect::tap(ChooseSpec::SpecificObject(source)),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        assert!(!game.current_has_static_ability_id(
            source,
            ironsmith::static_abilities::StaticAbilityId::Hexproof
        ));
        game.untap(source);
        assert!(game.current_has_static_ability_id(
            source,
            ironsmith::static_abilities::StaticAbilityId::Hexproof
        ));
    }
}
#[test]
fn scarring_memories_checks_an_attacking_legend_and_applies_all_three_actions_to_the_opponent() {
    for definition in definitions("Scarring Memories") {
        let mut game = game();
        fund(&mut game, A, 10);
        instant_window(&mut game, A);
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let attacker = card(
            &mut game,
            A,
            "Legend attacker",
            "Type: Legendary Creature\nPower/Toughness: 3/3",
            Zone::Battlefield,
        );
        let sacrificed = card(
            &mut game,
            B,
            "Sacrificed creature",
            "Type: Creature\nPower/Toughness: 2/2",
            Zone::Battlefield,
        );
        card(&mut game, B, "Discarded card", "Type: Artifact", Zone::Hand);
        assert!(casts(&game, A, spell).is_empty());
        game.turn.active_player = A;
        attack(&mut game, attacker, &mut Choices::default());
        game.turn.priority_player = Some(A);
        let action = casts(&game, A, spell).into_iter().next().unwrap();
        announce(
            &mut game,
            A,
            action,
            &mut Choices {
                target: Some(Target::Player(B)),
                ..Default::default()
            },
        );
        assert!(game.object(sacrificed).is_none());
        assert!(game.player(B).unwrap().hand.is_empty());
        assert_eq!(game.player(B).unwrap().graveyard.len(), 2);
        assert_eq!(game.player(B).unwrap().life, 17);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}
#[test]
fn fated_clash_checks_both_combat_states_and_protects_exact_shared_targets_before_wrath() {
    for definition in definitions("Fated Clash") {
        let mut game = game();
        fund(&mut game, A, 10);
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let attacker = card(
            &mut game,
            A,
            "Chosen attacker",
            "Type: Creature\nPower/Toughness: 3/3",
            Zone::Battlefield,
        );
        let blocker = card(
            &mut game,
            B,
            "Chosen blocker",
            "Type: Creature\nPower/Toughness: 3/3",
            Zone::Battlefield,
        );
        let other_a = card(
            &mut game,
            A,
            "Other A",
            "Type: Creature\nPower/Toughness: 3/3",
            Zone::Battlefield,
        );
        let other_b = card(
            &mut game,
            B,
            "Other B",
            "Type: Creature\nPower/Toughness: 3/3",
            Zone::Battlefield,
        );
        attack(&mut game, attacker, &mut Choices::default());
        assert!(
            casts(&game, A, spell).is_empty(),
            "an attack alone does not satisfy the conjunction"
        );
        game.turn.step = Some(ironsmith::Step::DeclareBlockers);
        let mut combat = game.combat.take().unwrap();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[ironsmith::BlockerDeclaration {
                blocker,
                blocking: attacker,
            }],
            B,
        )
        .unwrap();
        game.combat = Some(combat);
        game.turn.priority_player = Some(A);
        let action = casts(&game, A, spell).into_iter().next().unwrap();
        announce(
            &mut game,
            A,
            action,
            &mut Choices {
                targets: vec![Target::Object(attacker), Target::Object(blocker)],
                ..Default::default()
            },
        );
        assert!(game.battlefield.contains(&attacker));
        assert!(game.battlefield.contains(&blocker));
        assert!(!game.battlefield.contains(&other_a));
        assert!(!game.battlefield.contains(&other_b));
        for id in [attacker, blocker] {
            assert!(game.current_has_static_ability_id(
                id,
                ironsmith::static_abilities::StaticAbilityId::Indestructible
            ));
        }
        cleanup(&mut game);
        for id in [attacker, blocker] {
            assert!(!game.current_has_static_ability_id(
                id,
                ironsmith::static_abilities::StaticAbilityId::Indestructible
            ));
        }
    }
}
#[test]
fn global_flash_uses_the_chosen_face_instead_of_borrowing_the_other_faces_subtype() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::card::{LinkedFaceLayout, PowerToughness};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    for definition in definitions("Quick Sliver") {
        for sliver_front in [false, true] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            instant_window(&mut game, A);
            let front_id = ironsmith::CardId::new();
            let back_id = ironsmith::CardId::new();
            let front = CardDefinitionBuilder::new(front_id, "Front face")
                .mana_cost(ManaCost::new())
                .card_types(vec![CardType::Creature])
                .subtypes(vec![if sliver_front {
                    ironsmith::Subtype::Sliver
                } else {
                    ironsmith::Subtype::Bear
                }])
                .power_toughness(PowerToughness::fixed(2, 2))
                .other_face(back_id)
                .other_face_name("Back face")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            let back = CardDefinitionBuilder::new(back_id, "Back face")
                .mana_cost(ManaCost::new())
                .card_types(vec![CardType::Creature])
                .subtypes(vec![if sliver_front {
                    ironsmith::Subtype::Bear
                } else {
                    ironsmith::Subtype::Sliver
                }])
                .power_toughness(PowerToughness::fixed(2, 2))
                .other_face(front_id)
                .other_face_name("Front face")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let candidate = game.create_object_from_definition(&front, A, Zone::Hand);
            let available = casts(&game, A, candidate);
            assert_eq!(
                available.iter().any(|action| matches!(
                    action,
                    LegalAction::CastSpell {
                        casting_method: CastingMethod::Normal,
                        ..
                    }
                )),
                sliver_front
            );
            assert_eq!(
                available.iter().any(|action| matches!(
                    action,
                    LegalAction::CastSpell {
                        casting_method: CastingMethod::SplitOtherHalf,
                        ..
                    }
                )),
                !sliver_front
            );
        }
    }
}
#[test]
fn genuinely_hand_limited_flash_does_not_lend_timing_to_a_graveyard_route() {
    let mut game = game();
    instant_window(&mut game, A);
    ordinary_origin(&mut game, A, Zone::Graveyard);
    let limited = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::CardId::new(),
        "Hand-only flash",
    )
    .card_types(vec![CardType::Enchantment])
    .with_ability(ironsmith::ability::Ability::static_ability(
        ironsmith::StaticAbility::grants(ironsmith::grant::GrantSpec::flash_to_spells_matching(
            ironsmith::ObjectFilter::nonland(),
        )),
    ))
    .build();
    game.create_object_from_definition(&limited, A, Zone::Battlefield);
    let hand = card(
        &mut game,
        A,
        "Hand artifact",
        "Mana cost: {0}\nType: Artifact",
        Zone::Hand,
    );
    let grave = card(
        &mut game,
        A,
        "Grave artifact",
        "Mana cost: {0}\nType: Artifact",
        Zone::Graveyard,
    );
    assert!(!casts(&game, A, hand).is_empty());
    assert!(casts(&game, A, grave).is_empty());
    let action = casts(&game, A, hand).into_iter().next().unwrap();
    announce(&mut game, A, action, &mut Choices::default());
}
#[test]
fn checked_cast_time_board_count_does_not_erase_incomplete_characteristics() {
    let mut game = game();
    let source = card(
        &mut game,
        A,
        "Cast-time source",
        "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.",
        Zone::Hand,
    );
    let human = card(
        &mut game,
        A,
        "Human witness",
        "Type: Creature — Human\nPower/Toughness: 1/1",
        Zone::Battlefield,
    );
    let condition = ironsmith::ConditionExpr::CountComparison {
        count: ironsmith::static_abilities::AnthemCountExpression::MatchingFilter(
            ironsmith::ObjectFilter::creature()
                .with_subtype(ironsmith::Subtype::Human)
                .you_control(),
        ),
        comparison: ironsmith::effect::Comparison::GreaterThanOrEqual(1),
        display: None,
    };
    assert!(
        ironsmith::condition_eval::evaluate_condition_cast_time_checked(
            &game, &condition, A, source
        )
        .unwrap()
    );
    game.phase_out(human);
    assert!(
        !ironsmith::condition_eval::evaluate_condition_cast_time_checked(
            &game, &condition, A, source
        )
        .unwrap()
    );
    game.phase_in(human);
    game.player_mut(A).unwrap().mana_pool.colorless = u32::MAX;
    assert!(matches!(
        ironsmith::condition_eval::evaluate_condition_cast_time_checked(
            &game, &condition, A, source
        ),
        Err(ironsmith::ExecutionError::ContinuousDiscovery(_))
    ));
    assert!(compute_legal_actions(&game, A).is_err());
}
#[test]
fn conditional_flash_counts_current_subtypes_and_control_instead_of_printed_objects() {
    use ironsmith::continuous::Modification;
    use ironsmith::effect::Until;
    for (name, subtype) in [
        ("Bard's Company", ironsmith::Subtype::Human),
        ("Illusion Spinners", ironsmith::Subtype::Faerie),
    ] {
        for definition in definitions(name) {
            let mut game = game();
            fund(&mut game, A, 10);
            instant_window(&mut game, A);
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let helper = card(
                &mut game,
                A,
                "Mutable helper",
                "Type: Creature — Bird\nPower/Toughness: 1/1",
                Zone::Battlefield,
            );
            assert!(casts(&game, A, spell).is_empty());
            for (modification, expected) in [
                (Modification::AddSubtypes(vec![subtype]), true),
                (Modification::RemoveSubtypes(vec![subtype]), false),
                (Modification::AddSubtypes(vec![subtype]), true),
            ] {
                let change = Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(
                    ChooseSpec::SpecificObject(helper),
                    modification,
                    Until::Forever,
                ));
                execute_effect(
                    &mut game,
                    &change,
                    &mut EffectContext::new_default(spell, A),
                )
                .unwrap();
                assert_eq!(!casts(&game, A, spell).is_empty(), expected);
                assert!(
                    !game.object(helper).unwrap().subtypes.contains(&subtype),
                    "printed object intentionally remains unchanged"
                );
            }
            for (controller, expected) in [(B, false), (A, true)] {
                execute_effect(
                    &mut game,
                    &Effect::gain_control_with_duration(
                        ChooseSpec::SpecificObject(helper),
                        Until::Forever,
                    ),
                    &mut EffectContext::new_default(spell, controller),
                )
                .unwrap();
                assert_eq!(game.current_controller(helper), Some(controller));
                assert_eq!(!casts(&game, A, spell).is_empty(), expected);
            }
        }
    }
}
