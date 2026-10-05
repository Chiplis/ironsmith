//! UNVALIDATED full-card zone shuffle reference and participant regressions.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
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
        "../../../fixtures/zone_shuffle_references.json.fixture"
    ))
    .unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text += &format!("Power/Toughness: {p}/{t}\n");
    }
    text += row["oracle_text"].as_str().unwrap();
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
    objects: Vec<ObjectId>,
    modes: Vec<usize>,
    expected_mode_max: Option<usize>,
    accept: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.accept
    }
    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        for id in &self.objects {
            assert!(
                context
                    .candidates
                    .iter()
                    .any(|candidate| candidate.id == *id && candidate.legal)
            );
        }
        self.objects.clone()
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose optional costs for") {
            return vec![];
        }
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
        self.targets
            .iter()
            .copied()
            .filter(|target| {
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            })
            .collect()
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
fn resolve(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..64 {
        if game.stack.is_empty() {
            return;
        }
        resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("resolution did not settle");
}
fn library(game: &mut GameState, owner: PlayerId, count: usize) {
    for index in 0..count {
        vanilla(
            game,
            owner,
            Zone::Library,
            &format!("Library {index}"),
            false,
        );
    }
}
fn shuffles(game: &GameState) -> Vec<PlayerId> {
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .filter_map(|record| {
            record
                .event
                .downcast::<ironsmith::events::ShuffleLibraryEvent>()
        })
        .map(|event| event.player)
        .collect()
}
#[test]
fn every_full_frozen_payload_round_trips_without_lost_body() {
    for name in [
        "Archangel's Light",
        "Mnemonic Nexus",
        "Molten Psyche",
        "Rite of Renewal",
        "Stream of Thought",
        "Whirlpool Warrior",
        "Winds of Change",
        "Visions",
    ] {
        definitions(name);
    }
}
#[test]
fn archangels_light_counts_graveyard_before_its_shuffle() {
    for definition in definitions("Archangel's Light") {
        let mut game = game();
        for _ in 0..3 {
            vanilla(&mut game, A, Zone::Graveyard, "Own grave", false);
        }
        vanilla(&mut game, B, Zone::Graveyard, "Foreign grave", false);
        library(&mut game, A, 4);
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 26);
        assert_eq!(game.player(A).unwrap().library.len(), 7);
        assert_eq!(
            game.player(A).unwrap().graveyard.len(),
            1,
            "only the resolving spell remains"
        );
        assert_eq!(game.player(B).unwrap().graveyard.len(), 1);
        assert_eq!(shuffles(&game), vec![A]);
    }
}
#[test]
fn mnemonic_nexus_keeps_each_graveyard_and_empty_participant_separate() {
    for definition in definitions("Mnemonic Nexus") {
        let mut game = game();
        for (player, count) in [(A, 2), (B, 3), (C, 0)] {
            library(&mut game, player, 5);
            for _ in 0..count {
                vanilla(&mut game, player, Zone::Graveyard, "Grave", false);
            }
        }
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(
            [
                game.player(A).unwrap().library.len(),
                game.player(B).unwrap().library.len(),
                game.player(C).unwrap().library.len()
            ],
            [7, 8, 5]
        );
        assert_eq!(shuffles(&game), vec![A, B, C]);
    }
}
#[test]
fn winds_and_molten_keep_actual_per_player_hand_and_draw_quantities() {
    for name in ["Winds of Change", "Molten Psyche"] {
        for definition in definitions(name) {
            let mut game = game();
            for (player, count) in [(A, 2), (B, 3), (C, 0)] {
                library(&mut game, player, 12);
                for _ in 0..count {
                    vanilla(&mut game, player, Zone::Hand, "Old hand", false);
                }
            }
            for _ in 0..3 {
                let artifact = ironsmith::CardBuilder::new(ironsmith::CardId::new(), "Metalcraft")
                    .card_types(vec![ironsmith::CardType::Artifact])
                    .build();
                game.create_object_from_card(&artifact, A, Zone::Battlefield);
            }
            let mut dm = Choices::default();
            cast(&mut game, &definition, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(
                [
                    game.player(A).unwrap().hand.len(),
                    game.player(B).unwrap().hand.len(),
                    game.player(C).unwrap().hand.len()
                ],
                [2, 3, 0]
            );
            assert_eq!(shuffles(&game), vec![A, B, C]);
            assert_eq!(
                game.player(B).unwrap().life,
                if name == "Molten Psyche" { 17 } else { 20 }
            );
            assert_eq!(game.player(C).unwrap().life, 20);
        }
    }
}
#[test]
fn stream_mills_the_target_but_selects_only_its_controllers_graveyard() {
    for definition in definitions("Stream of Thought") {
        let mut game = game();
        library(&mut game, A, 5);
        library(&mut game, B, 8);
        let selected: Vec<_> = (0..3)
            .map(|_| vanilla(&mut game, A, Zone::Graveyard, "Own grave", false))
            .collect();
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            objects: selected,
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().library.len(), 4);
        assert_eq!(game.player(B).unwrap().graveyard.len(), 4);
        assert_eq!(game.player(A).unwrap().library.len(), 8);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        assert_eq!(shuffles(&game), vec![A]);
    }
}
#[test]
fn rite_keeps_target_player_distinct_from_both_sets_of_target_cards() {
    for definition in definitions("Rite of Renewal") {
        let mut game = game();
        library(&mut game, A, 5);
        library(&mut game, B, 5);
        let own = vanilla(&mut game, A, Zone::Graveyard, "Own permanent", true);
        let theirs = vanilla(&mut game, B, Zone::Graveyard, "Their card", false);
        let mut dm = Choices {
            targets: vec![
                Target::Object(own),
                Target::Player(B),
                Target::Object(theirs),
            ],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(game.player(B).unwrap().library.len(), 6);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 0);
        assert_eq!(shuffles(&game), vec![B]);
        assert!(game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name.as_ref() == "Rite of Renewal")
        }));
    }
}
#[test]
fn visions_optional_shuffle_uses_the_looked_at_library_owner() {
    for definition in definitions("Visions") {
        for accept in [false, true] {
            let mut game = game();
            library(&mut game, A, 8);
            library(&mut game, B, 8);
            let before = game.player(A).unwrap().library.clone();
            let mut dm = Choices {
                targets: vec![Target::Player(B)],
                accept,
                ..Default::default()
            };
            cast(&mut game, &definition, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().library, before);
            assert_eq!(shuffles(&game), if accept { vec![B] } else { vec![] });
            assert_eq!(game.player(B).unwrap().library.len(), 8);
        }
    }
}
