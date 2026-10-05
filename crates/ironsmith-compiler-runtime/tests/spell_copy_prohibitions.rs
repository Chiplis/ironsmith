//! UNVALIDATED full-card regressions. Authored during the implementation-first campaign.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, OutcomeStatus};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, StackEntry};
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/spell_copy_prohibitions.json.fixture"
    ))
    .unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = format!(
        "Mana cost: {}\nType: {}\n{}",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap()
    );
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 30);
    }
    game
}
fn vanilla(
    game: &mut GameState,
    owner: PlayerId,
    zone: Zone,
    name: &str,
    creature: bool,
) -> ObjectId {
    let text = if creature {
        "Mana cost: {1}\nType: Creature — Bear\nPower/Toughness: 2/2"
    } else {
        "Mana cost: {1}\nType: Instant\nYou gain 1 life."
    };
    let (_, definition) = compile_to_artifact(name, text, false).unwrap();
    let id = game.create_object_from_definition(&definition, owner, zone);
    if zone == Zone::Stack {
        game.stack.push(StackEntry::new(id, owner));
    }
    id
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    modes: Vec<usize>,
    expected_mode_max: Option<usize>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        false
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose mode for") {
            if let Some(max) = self.expected_mode_max {
                assert_eq!(context.max, max);
            }
            assert!(self.modes.len() >= context.min && self.modes.len() <= context.max);
            return self.modes.clone();
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        for target in &self.targets {
            assert!(
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            );
        }
        self.targets.clone()
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition, choices: &mut Choices) -> ObjectId {
    game.turn.priority_player = Some(A);
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
        choices,
    )
    .unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices)
            .unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    game.stack.last().unwrap().object_id
}
#[test]
fn all_three_complete_artifacts_prohibit_spell_copying() {
    for name in ["Choreographed Sparks", "Display of Power", "See Double"] {
        for definition in definitions(name) {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Stack);
            game.stack.push(StackEntry::new(spell, A));
            let mut dm = SelectFirstDecisionMaker;
            let result = execute_effect(
                &mut game,
                &Effect::new(ironsmith::effects::CopySpellEffect::single(
                    ChooseSpec::Source,
                )),
                &mut EffectContext::new(spell, A, &mut dm),
            )
            .unwrap();
            assert_eq!(result.status, OutcomeStatus::Protected, "{name}");
            assert_eq!(game.stack.len(), 1);
        }
    }
}
#[test]
fn display_copies_every_selected_legal_spell_but_no_protected_spell() {
    for definition in definitions("Display of Power") {
        let mut game = game();
        let left = vanilla(&mut game, A, Zone::Stack, "Left", false);
        let right = vanilla(&mut game, B, Zone::Stack, "Right", false);
        let protected = game.create_object_from_definition(&definition, B, Zone::Stack);
        game.stack.push(StackEntry::new(protected, B));
        let mut dm = Choices {
            targets: vec![
                Target::Object(left),
                Target::Object(right),
                Target::Object(protected),
            ],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            game.stack.len(),
            5,
            "two originals plus protected original and exactly two copies"
        );
        assert_eq!(
            game.stack
                .iter()
                .filter(|entry| game.object(entry.object_id).unwrap().kind
                    == ironsmith::object::ObjectKind::SpellCopy)
                .count(),
            2
        );
    }
}
#[test]
fn display_can_choose_zero_targets() {
    for definition in definitions("Display of Power") {
        let mut game = game();
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.stack.is_empty());
    }
}
#[test]
fn sparks_creature_copy_keeps_haste_and_its_own_end_step_sacrifice() {
    for definition in definitions("Choreographed Sparks") {
        let mut game = game();
        let creature = vanilla(&mut game, A, Zone::Stack, "Bear", true);
        let mut dm = Choices {
            targets: vec![Target::Object(creature)],
            modes: vec![1],
            expected_mode_max: Some(2),
        };
        cast(&mut game, &definition, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let copy = game.stack.last().unwrap().object_id;
        assert_ne!(copy, creature);
        let identity = game.object(copy).unwrap().stable_id;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let token = game
            .objects_in_deterministic_order()
            .into_iter()
            .find(|object| object.stable_id == identity && object.zone == Zone::Battlefield)
            .unwrap()
            .id;
        assert!(game.current_abilities(token).unwrap().iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(rule) if rule.id() == StaticAbilityId::Haste)));
        let mut queue = TriggerQueue::new();
        for entry in check_triggers(
            &game,
            &TriggerEvent::new(
                ironsmith::events::BeginningOfEndStepEvent::new(A),
                Default::default(),
            ),
        ) {
            queue.add(entry);
        }
        assert_eq!(queue.entries.len(), 1);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(
            game.object(token)
                .is_none_or(|object| object.zone != Zone::Battlefield)
        );
        assert!(game.stack.iter().any(|entry| entry.object_id == creature));
    }
}
#[test]
fn see_double_modal_threshold_is_per_opponent_and_frozen_at_announcement() {
    for definition in definitions("See Double") {
        for counts in [(4, 4, 1), (0, 8, 2)] {
            let mut game = game();
            for (owner, count) in [(B, counts.0), (C, counts.1)] {
                for _ in 0..count {
                    vanilla(&mut game, owner, Zone::Graveyard, "Grave", false);
                }
            }
            let spell = vanilla(&mut game, A, Zone::Stack, "Spell", false);
            let creature = vanilla(&mut game, B, Zone::Battlefield, "Creature", true);
            let mut dm = Choices {
                targets: if counts.2 == 2 {
                    vec![Target::Object(spell), Target::Object(creature)]
                } else {
                    vec![Target::Object(spell)]
                },
                modes: if counts.2 == 2 { vec![0, 1] } else { vec![0] },
                expected_mode_max: Some(counts.2),
            };
            cast(&mut game, &definition, &mut dm);
            for owner in [B, C] {
                for id in game.player(owner).unwrap().graveyard.clone() {
                    game.move_object_by_effect(id, Zone::Exile);
                }
            }
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.stack.len(), 2, "one original and one copy");
            let tokens = game
                .objects_in_deterministic_order()
                .into_iter()
                .filter(|object| {
                    object.zone == Zone::Battlefield
                        && object.kind == ironsmith::object::ObjectKind::Token
                })
                .count();
            assert_eq!(tokens, usize::from(counts.2 == 2));
        }
    }
}
