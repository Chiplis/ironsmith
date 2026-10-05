//! Source-authored only: no compilation or execution in this campaign stage.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::game_state::{Phase, Step};
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::types::Supertype;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str) -> Vec<CardDefinition> {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/enter_copy_exceptions.json.fixture"
    ))
    .unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let faces = row["card_faces"]
        .as_array()
        .map(|faces| faces.iter().collect::<Vec<_>>())
        .unwrap_or_else(|| vec![row]);
    let mut definitions = Vec::new();
    for face in faces {
        let face_name = face["name"].as_str().unwrap();
        let text = format!(
            "Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
            face["mana_cost"].as_str().unwrap(),
            face["type_line"].as_str().unwrap(),
            face["power"].as_str().unwrap(),
            face["toughness"].as_str().unwrap(),
            face["oracle_text"].as_str().unwrap()
        );
        let (result, loss) =
            ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(face_name, text, false));
        let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
        artifact.validate().unwrap();
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        assert_eq!(artifact, restored);
        definitions.extend([direct, materialize_artifact(&restored).unwrap()]);
    }
    definitions
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn original(game: &mut GameState, owner: PlayerId, legendary: bool) -> ObjectId {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Borrowed original")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .with_ability(Ability::static_ability(StaticAbility::vigilance()));
    let definition = if legendary {
        builder.supertypes(vec![Supertype::Legendary]).build()
    } else {
        builder.build()
    };
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}
#[derive(Default)]
struct CopyChoice {
    accept: bool,
    offered_copies: usize,
}
impl DecisionMaker for CopyChoice {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        context.can_accept
    }
    fn decide_options(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if context
            .options
            .iter()
            .any(|option| option.description.starts_with("Enter as a copy of"))
        {
            self.offered_copies += 1;
            return vec![
                context
                    .options
                    .iter()
                    .find(|option| {
                        option.legal
                            && option.description.starts_with("Enter as a copy of") == self.accept
                    })
                    .unwrap()
                    .index,
            ];
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
}
fn enter(game: &mut GameState, source: ObjectId, choice: &mut CopyChoice) -> ObjectId {
    let receipt = game
        .move_object_with_etb_processing_with_dm(source, Zone::Battlefield, choice)
        .unwrap();
    assert!(!receipt.pending);
    assert!(receipt.programs.is_empty());
    receipt.original.into_result().unwrap().new_id
}
fn settle(game: &mut GameState, choice: &mut CopyChoice) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for _ in 0..20 {
        ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
        if game.stack_is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, choice).unwrap();
    }
    panic!("bounded copy-trigger scenario did not settle");
}
fn attack(game: &mut GameState, attacker: ObjectId, defender: PlayerId) {
    game.remove_summoning_sickness(attacker);
    game.mark_combat_phase_started();
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut combat = ironsmith::combat_state::CombatState::default();
    ironsmith::game_loop::apply_attacker_declarations(
        game,
        &mut combat,
        &mut queue,
        &[ironsmith::decision::AttackerDeclaration {
            creature: attacker,
            target: ironsmith::combat_state::AttackTarget::Player(defender),
        }],
    )
    .unwrap();
    game.combat = Some(combat);
    ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
}
#[test]
fn six_root_bodies_and_the_identical_sakashima_face_alias_have_strict_direct_and_artifact_gates() {
    for name in [
        "Altered Ego",
        "Auton Soldier",
        "Dack's Duplicate",
        "Protean Raider",
        "Sakashima of a Thousand Faces",
        "Sakashima of a Thousand Faces // Sakashima of a Thousand Faces",
        "Undercover Operative",
    ] {
        assert!(!definitions(name).is_empty());
    }
}
#[test]
fn copied_entry_x_counters_use_the_entrants_own_x_and_declining_copy_keeps_the_printed_body() {
    for definition in definitions("Altered Ego") {
        for (x, accept, expected) in [(Some(4), true, 4), (None, true, 0), (Some(4), false, 0)] {
            let mut game = game();
            original(&mut game, B, false);
            let entrant = game.create_object_from_definition(
                &definition,
                A,
                if x.is_some() { Zone::Stack } else { Zone::Hand },
            );
            game.object_mut(entrant).unwrap().x_value = x;
            let entered = enter(
                &mut game,
                entrant,
                &mut CopyChoice {
                    accept,
                    offered_copies: 0,
                },
            );
            assert_eq!(
                game.object(entered)
                    .unwrap()
                    .counters
                    .get(&ironsmith::CounterType::PlusOnePlusOne)
                    .copied()
                    .unwrap_or(0),
                expected
            );
            assert_eq!(
                game.object(entered).unwrap().name.as_ref(),
                if accept {
                    "Borrowed original"
                } else {
                    "Altered Ego"
                }
            );
        }
    }
}
#[test]
fn shield_exception_uses_the_chosen_creatures_controller_and_is_not_a_copy_of_existing_counters() {
    for definition in definitions("Undercover Operative") {
        for owner in [A, B] {
            let mut game = game();
            let template = original(&mut game, owner, false);
            game.object_mut(template)
                .unwrap()
                .counters
                .insert(ironsmith::CounterType::Shield, 4);
            let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
            let entered = enter(
                &mut game,
                entrant,
                &mut CopyChoice {
                    accept: true,
                    offered_copies: 0,
                },
            );
            assert_eq!(
                game.object(entered)
                    .unwrap()
                    .counters
                    .get(&ironsmith::CounterType::Shield)
                    .copied()
                    .unwrap_or(0),
                u32::from(owner == A)
            );
        }
    }
}
#[test]
fn sakashima_preserves_other_copiable_abilities_and_the_legend_rule_but_not_layer_six_grants() {
    for definition in definitions("Sakashima of a Thousand Faces") {
        let mut game = game();
        let template = original(&mut game, A, true);
        let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
        game.effect_store.continuous_effects.add_effect(
            ironsmith::continuous::ContinuousEffect::new(
                template,
                A,
                ironsmith::continuous::EffectTarget::Specific(entrant),
                ironsmith::continuous::Modification::AddAbility(StaticAbility::flying()),
            ),
        );
        let entered = enter(
            &mut game,
            entrant,
            &mut CopyChoice {
                accept: true,
                offered_copies: 0,
            },
        );
        assert!(game.current_has_static_ability_id(entered, StaticAbilityId::Vigilance));
        assert!(game.current_has_static_ability_id(entered, StaticAbilityId::Partner));
        assert!(!game.current_has_static_ability_id(entered, StaticAbilityId::Flying));
        assert!(!game.object(entered).unwrap().abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.enter_as_copy_as_enters().is_some())));
        ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
        assert!(game.battlefield.contains(&template) && game.battlefield.contains(&entered));
    }
}
#[test]
fn raid_copy_condition_uses_actual_attack_history_and_has_one_replacement_occurrence() {
    for definition in definitions("Protean Raider") {
        for attacked in [false, true] {
            let mut game = game();
            let template = original(&mut game, A, false);
            if attacked {
                attack(&mut game, template, B);
            }
            let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
            let mut choice = CopyChoice {
                accept: true,
                offered_copies: 0,
            };
            let entered = enter(&mut game, entrant, &mut choice);
            assert_eq!(
                game.object(entered).unwrap().name.as_ref(),
                if attacked {
                    "Borrowed original"
                } else {
                    "Protean Raider"
                }
            );
            assert_eq!(choice.offered_copies, usize::from(attacked));
        }
    }
}
#[test]
fn added_dethrone_is_a_real_attack_trigger_and_its_life_qualification_is_not_rechecked_on_resolution()
 {
    for definition in definitions("Dack's Duplicate") {
        let mut game = game();
        original(&mut game, B, false);
        game.player_mut(B).unwrap().life = 30;
        let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut choice = CopyChoice {
            accept: true,
            offered_copies: 0,
        };
        let entered = enter(&mut game, entrant, &mut choice);
        assert!(game.current_has_static_ability_id(entered, StaticAbilityId::Haste));
        attack(&mut game, entered, B);
        assert!(!game.stack_is_empty());
        game.player_mut(B).unwrap().life = 1;
        settle(&mut game, &mut choice);
        assert_eq!(
            game.object(entered)
                .unwrap()
                .counters
                .get(&ironsmith::CounterType::PlusOnePlusOne),
            Some(&1)
        );
    }
}
#[test]
fn added_myriad_creates_nonlegendary_artifact_copies_for_other_opponents_and_exiles_them_at_combat_end()
 {
    for definition in definitions("Auton Soldier") {
        let mut game = game();
        original(&mut game, B, true);
        let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut choice = CopyChoice {
            accept: true,
            offered_copies: 0,
        };
        let entered = enter(&mut game, entrant, &mut choice);
        assert!(
            game.object(entered)
                .unwrap()
                .card_types
                .contains(&CardType::Artifact)
        );
        assert!(
            !game
                .object(entered)
                .unwrap()
                .supertypes
                .contains(&Supertype::Legendary)
        );
        attack(&mut game, entered, B);
        settle(&mut game, &mut choice);
        let copies: Vec<_> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id).is_some_and(|object| {
                    matches!(object.kind, ironsmith::object::ObjectKind::Token)
                        && object.name.as_ref() == "Borrowed original"
                })
            })
            .collect();
        assert_eq!(copies.len(), 2);
        for copy in &copies {
            assert!(game.is_tapped(*copy));
            assert!(
                !game
                    .object(*copy)
                    .unwrap()
                    .supertypes
                    .contains(&Supertype::Legendary)
            );
        }
        let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
            ironsmith::turn_runner::TurnState::EndCombat,
        );
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        runner.advance(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choice)
            .unwrap();
        settle(&mut game, &mut choice);
        assert!(copies.iter().all(|id| !game.battlefield.contains(id)));
        assert!(game.battlefield.contains(&entered));
    }
}

struct MyriadControllerChoices {
    answers: Vec<bool>,
    asked: Vec<PlayerId>,
}
impl DecisionMaker for MyriadControllerChoices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        assert_eq!(
            context.player, A,
            "the source controller chooses for every opponent"
        );
        let index = self.asked.len();
        self.asked.push(context.player);
        context.can_accept && self.answers[index]
    }
    fn decide_options(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(game, context)
    }
}
#[test]
fn embedded_and_printed_myriad_ask_only_the_controller_and_honor_each_accept_or_decline() {
    let printed = CardDefinitionBuilder::new(CardId::new(), "Printed Myriad")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .myriad()
        .build();
    for definition in definitions("Auton Soldier").into_iter().chain([printed]) {
        for answers in [
            vec![false, false],
            vec![true, false],
            vec![false, true],
            vec![true, true],
        ] {
            let mut game = game();
            original(&mut game, B, false);
            let printed = definition.card.name.as_str() == "Printed Myriad";
            let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
            let entered = if printed {
                game.move_object_by_effect(entrant, Zone::Battlefield)
                    .unwrap()
            } else {
                enter(
                    &mut game,
                    entrant,
                    &mut CopyChoice {
                        accept: true,
                        offered_copies: 0,
                    },
                )
            };
            attack(&mut game, entered, B);
            let mut choices = MyriadControllerChoices {
                answers: answers.clone(),
                asked: Vec::new(),
            };
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(choices.asked, vec![A, A]);
            assert_eq!(
                game.battlefield
                    .iter()
                    .filter(|id| game.object(**id).is_some_and(|object| matches!(
                        object.kind,
                        ironsmith::object::ObjectKind::Token
                    )))
                    .count(),
                answers.iter().filter(|answer| **answer).count()
            );
        }
    }
}

#[test]
fn myriad_entry_and_creation_additions_observe_all_original_copies_before_removing_the_source() {
    use ironsmith::effect::{Effect, Value};
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    for definition in definitions("Auton Soldier") {
        for entry_addition in [false, true] {
            let mut game = game();
            original(&mut game, B, false);
            let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
            let mut choice = CopyChoice {
                accept: true,
                offered_copies: 0,
            };
            let entered = enter(&mut game, entrant, &mut choice);
            let additions = vec![
                Effect::gain_life(Value::Count(ObjectFilter::creature().token())),
                Effect::destroy(ChooseSpec::SpecificObject(entered)),
            ];
            let replacement = if entry_addition {
                ReplacementEffect::with_matcher(
                    entered,
                    A,
                    ironsmith::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                        ObjectFilter::creature().token().you_control(),
                    ),
                    ReplacementAction::Additionally(additions),
                )
            } else {
                ReplacementEffect::with_matcher(
                    entered,
                    A,
                    ironsmith::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
                        PlayerFilter::You,
                    ),
                    ReplacementAction::Additionally(additions),
                )
            };
            game.effect_store
                .replacement_effects
                .add_one_shot_effect(replacement);
            attack(&mut game, entered, B);
            settle(&mut game, &mut choice);
            let copies: Vec<_> = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| {
                    game.object(*id).is_some_and(|object| {
                        matches!(object.kind, ironsmith::object::ObjectKind::Token)
                    })
                })
                .collect();
            assert_eq!(
                copies.len(),
                2,
                "the second proposal retains the source's copiable values"
            );
            assert_eq!(
                game.player(A).unwrap().life,
                22,
                "the first addition sees the whole original batch"
            );
            assert!(!game.battlefield.contains(&entered));
            for id in &copies {
                assert_eq!(game.object(*id).unwrap().name.as_ref(), "Borrowed original");
                assert!(game.is_tapped(*id));
                assert!(
                    game.object(*id)
                        .unwrap()
                        .card_types
                        .contains(&CardType::Artifact)
                );
                assert_eq!(game.current_power(*id), Some(2));
            }
            let targets: Vec<_> = game
                .combat
                .as_ref()
                .unwrap()
                .attackers
                .iter()
                .filter(|attacker| copies.contains(&attacker.creature))
                .map(|attacker| attacker.target.clone())
                .collect();
            assert_eq!(
                targets,
                vec![
                    ironsmith::combat_state::AttackTarget::Player(PlayerId(2)),
                    ironsmith::combat_state::AttackTarget::Player(PlayerId(3))
                ]
            );
            let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
                ironsmith::turn_runner::TurnState::EndCombat,
            );
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            runner.advance(&mut game, &mut queue).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choice)
                .unwrap();
            settle(&mut game, &mut choice);
            assert!(
                copies.iter().all(|id| !game.battlefield.contains(id)),
                "both cleanups survive source departure"
            );
        }
    }
}

struct PreparedMyriadDestinations {
    asked: usize,
}
impl DecisionMaker for PreparedMyriadDestinations {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        assert_eq!(context.player, A);
        context.can_accept
    }
    fn decide_options(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if context
            .options
            .iter()
            .any(|option| option.description.starts_with("Attack "))
        {
            assert_eq!(context.player, A);
            assert!(
                !context
                    .options
                    .iter()
                    .any(|option| option.description.contains("Protected battle")),
                "Myriad permits a player or their planeswalker, never their protected battle"
            );
            assert!(
                !context
                    .options
                    .iter()
                    .any(|option| option.description.contains("Phased walker")),
                "phased-out permanents are not available attack destinations"
            );
            assert!(
                game.battlefield.iter().all(|id| !matches!(
                    game.object(*id).unwrap().kind,
                    ironsmith::object::ObjectKind::Token
                )),
                "every participant's destination is selected before the first original copy"
            );
            self.asked += 1;
            return vec![
                context
                    .options
                    .iter()
                    .find(|option| option.description.contains("Defending walker"))
                    .unwrap()
                    .index,
            ];
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
}
#[test]
fn myriad_attack_destinations_are_locked_once_before_original_token_creation() {
    for definition in definitions("Auton Soldier") {
        let mut game = game();
        original(&mut game, B, false);
        let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
        let entered = enter(
            &mut game,
            entrant,
            &mut CopyChoice {
                accept: true,
                offered_copies: 0,
            },
        );
        let walker = CardDefinitionBuilder::new(CardId::new(), "Defending walker")
            .card_types(vec![CardType::Planeswalker])
            .build();
        let walkers: Vec<_> = [PlayerId(2), PlayerId(3)]
            .into_iter()
            .map(|owner| game.create_object_from_definition(&walker, owner, Zone::Battlefield))
            .collect();
        let battle = CardDefinitionBuilder::new(CardId::new(), "Protected battle")
            .card_types(vec![CardType::Battle])
            .subtypes(vec![ironsmith::types::Subtype::Siege])
            .defense(5)
            .build();
        let battle = game.create_object_from_definition(&battle, A, Zone::Battlefield);
        assert!(game.set_battle_protector(battle, PlayerId(2)));
        let phased = CardDefinitionBuilder::new(CardId::new(), "Phased walker")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(5)
            .build();
        let phased = game.create_object_from_definition(&phased, PlayerId(2), Zone::Battlefield);
        game.phase_out(phased);
        // Starting loyalty prevents unrelated state-based removal while the
        // test inspects the exact preselected permanent destinations.
        for &id in &walkers {
            game.object_mut(id)
                .unwrap()
                .counters
                .insert(ironsmith::CounterType::Loyalty, 5);
        }
        attack(&mut game, entered, B);
        let mut choices = PreparedMyriadDestinations { asked: 0 };
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(choices.asked, 2);
        let targets: Vec<_> = game
            .combat
            .as_ref()
            .unwrap()
            .attackers
            .iter()
            .filter(|attacker| {
                matches!(
                    game.object(attacker.creature).unwrap().kind,
                    ironsmith::object::ObjectKind::Token
                )
            })
            .map(|attacker| attacker.target.clone())
            .collect();
        assert_eq!(
            targets,
            walkers
                .into_iter()
                .map(ironsmith::combat_state::AttackTarget::Planeswalker)
                .collect::<Vec<_>>()
        );
    }
}

struct AnnouncedCopyX {
    x: u32,
    saw_x: bool,
}
impl DecisionMaker for AnnouncedCopyX {
    fn decide_number(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        assert!(context.is_x_value);
        assert!(self.x >= context.min && self.x <= context.max);
        self.saw_x = true;
        self.x
    }
    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::ManaPaymentContext,
    ) -> ironsmith::mana_payment::ManaPaymentResponse {
        SelectFirstDecisionMaker.decide_mana_payment(game, context)
    }
}
fn activate_copy_x(game: &mut GameState, source: ObjectId, x: u32) {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    game.turn.priority_player = Some(A);
    let ability_index = game
        .object(source)
        .unwrap()
        .abilities
        .iter()
        .position(|ability| matches!(&ability.kind, AbilityKind::Activated(_)))
        .unwrap();
    game.player_mut(A).unwrap().mana_pool.empty();
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Colorless, x);
    let action = LegalAction::ActivateAbility {
        source,
        ability_index,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players.len());
    let mut choices = AnnouncedCopyX { x, saw_x: false };
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut choices,
    )
    .unwrap();
    for _ in 0..40 {
        if !state.has_pending_action() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("pending X activation without decision");
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut choices)
                .unwrap();
    }
    assert!(!state.has_pending_action());
    assert!(choices.saw_x);
    assert_eq!(
        game.player(A).unwrap().mana_pool.total(),
        0,
        "the announced X was paid"
    );
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut choices).unwrap();
}
#[test]
fn gigantoplasm_pays_announced_x_and_keeps_its_lasting_stat_assignment_noncopiable() {
    use ironsmith::effects::EffectExecutor;
    use ironsmith::target::ChooseSpec;
    for definition in definitions("Gigantoplasm") {
        let mut game = game();
        original(&mut game, B, false);
        let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
        let entered = enter(
            &mut game,
            entrant,
            &mut CopyChoice {
                accept: true,
                offered_copies: 0,
            },
        );
        game.add_counters(entered, ironsmith::CounterType::PlusOnePlusOne, 1)
            .unwrap();
        for x in [5, 7] {
            activate_copy_x(&mut game, entered, x);
            assert_eq!(game.current_power(entered), Some(x as i32 + 1));
            assert_eq!(game.current_toughness(entered), Some(x as i32 + 1));
            assert_eq!(
                game.object(entered).unwrap().base_power,
                Some(ironsmith::card::PtValue::Fixed(2))
            );
            assert_eq!(
                game.object(entered).unwrap().base_toughness,
                Some(ironsmith::card::PtValue::Fixed(3))
            );
        }
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            game.current_power(entered),
            Some(8),
            "undated assignment survives cleanup"
        );
        let outcome =
            ironsmith::effects::CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(entered))
                .execute(
                    &mut game,
                    &mut ironsmith::effects::EffectContext::new_default(entered, A),
                )
                .unwrap();
        let token = outcome.explicit_objects().unwrap()[0];
        assert_eq!(game.current_power(token), Some(2));
        assert_eq!(game.current_toughness(token), Some(3));
        assert!(
            game.object(token)
                .unwrap()
                .abilities
                .iter()
                .any(|ability| matches!(&ability.kind, AbilityKind::Activated(_))),
            "the quoted activation is a copiable exception; its resolved 7b assignment is not"
        );
        let exiled = game.move_object_by_effect(entered, Zone::Exile).unwrap();
        let returned = enter(
            &mut game,
            exiled,
            &mut CopyChoice {
                accept: true,
                offered_copies: 0,
            },
        );
        assert_ne!(entered, returned);
        assert_eq!(game.current_power(returned), Some(2));
        assert_eq!(
            game.current_toughness(returned),
            Some(3),
            "old-incarnation assignment cannot follow the physical card"
        );
    }
}
fn assert_live_goad_declarations(game: &GameState, attacker: ObjectId) {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    for target in [None, Some(A), Some(PlayerId(2))] {
        let mut branch = game.clone();
        branch.turn.active_player = B;
        branch.turn.priority_player = Some(B);
        branch.turn.phase = Phase::Combat;
        branch.turn.step = Some(Step::DeclareAttackers);
        branch.remove_summoning_sickness(attacker);
        branch.untap(attacker);
        let mut combat = CombatState::default();
        let legal = ironsmith::decision::compute_legal_attackers(&branch, &combat);
        assert!(
            legal
                .iter()
                .find(|option| option.creature == attacker)
                .unwrap()
                .must_attack
        );
        let declaration = target
            .into_iter()
            .map(|player| AttackerDeclaration {
                creature: attacker,
                target: AttackTarget::Player(player),
            })
            .collect::<Vec<_>>();
        let result = ironsmith::game_loop::apply_attacker_declarations(
            &mut branch,
            &mut combat,
            &mut ironsmith::triggers::TriggerQueue::new(),
            &declaration,
        );
        assert_eq!(
            result.is_ok(),
            target == Some(PlayerId(2)),
            "live goad must affect declaration legality: {result:?}"
        );
    }
}
#[test]
fn mocking_same_name_goad_tracks_current_names_source_scope_and_actual_attack_requirements() {
    use ironsmith::continuous::Modification;
    use ironsmith::effect::Until;
    use ironsmith::effects::EffectContext;
    use ironsmith::effects::{ApplyContinuousEffect, EffectExecutor};
    use ironsmith::target::ChooseSpec;
    for definition in definitions("Mocking Doppelganger") {
        let mut game = game();
        let opponent = original(&mut game, B, false);
        let entrant = game.create_object_from_definition(&definition, A, Zone::Hand);
        assert!(game.current_has_static_ability_id(entrant, StaticAbilityId::Flash));
        let source = enter(
            &mut game,
            entrant,
            &mut CopyChoice {
                accept: true,
                offered_copies: 0,
            },
        );
        let own_other = original(&mut game, A, false);
        for id in [opponent, own_other] {
            assert_eq!(game.active_goaders_for(id), [A].into_iter().collect());
        }
        assert!(
            game.active_goaders_for(source).is_empty(),
            "other excludes the supplying copy itself"
        );
        assert_live_goad_declarations(&game, opponent);
        ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(opponent),
            Modification::RemoveAllAbilities,
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, A))
        .unwrap();
        assert_eq!(game.active_goaders_for(opponent), [A].into_iter().collect());
        game.set_current_controller(source, PlayerId(2)).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            game.active_goaders_for(opponent),
            [PlayerId(2)].into_iter().collect()
        );
        game.set_current_controller(source, A).unwrap();
        game.phase_out(source);
        game.refresh_continuous_state().unwrap();
        assert!(game.active_goaders_for(opponent).is_empty());
        game.phase_in(source);
        game.refresh_continuous_state().unwrap();
        assert_live_goad_declarations(&game, opponent);
        ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(source),
            Modification::SetName("Changed name".into()),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, A))
        .unwrap();
        assert!(
            game.active_goaders_for(opponent).is_empty(),
            "predicate reads the current source name"
        );
        ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(opponent),
            Modification::SetName("Changed name".into()),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, A))
        .unwrap();
        assert_eq!(
            game.active_goaders_for(opponent),
            [A].into_iter().collect(),
            "predicate reads the current recipient name"
        );
        ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(source),
            Modification::RemoveAllAbilities,
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, A))
        .unwrap();
        assert!(
            game.active_goaders_for(opponent).is_empty(),
            "removing the supplying ability stops its designation"
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.active_goaders_for(opponent), [A].into_iter().collect());
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(game.active_goaders_for(opponent).is_empty());
    }
}
