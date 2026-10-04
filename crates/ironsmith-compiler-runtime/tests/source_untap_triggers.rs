use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::CardBuilder;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::BooleanContext;
use ironsmith::effects::{EffectContext, EffectExecutor, UntapEffect};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Step;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{Trigger, TriggerQueue};
use ironsmith::{CardId, CardType, Effect, GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ObjectFilter, TriggerKind};
use ironsmith_runtime_catalog::CardRegistryArtifactExt;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&restored).unwrap();
    [direct, registry.get(name).unwrap().clone()]
}

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/source_untap_triggers.json.fixture"
    ))
    .unwrap()
}

fn fixture(name: &str) -> [CardDefinition; 2] {
    let fixture = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    definitions(name, fixture["text"].as_str().unwrap())
}

fn settle(game: &mut GameState, dm: &mut impl DecisionMaker) -> usize {
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    let initial = game.stack.len();
    for _ in 0..8 {
        if game.stack_is_empty() {
            return initial;
        }
        resolve_stack_entry_with(game, dm).unwrap();
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    }
    panic!("untap and its resulting room/choice work did not settle");
}

fn untap(game: &mut GameState, source: ObjectId, target: ObjectId) -> usize {
    untap_spec(game, source, ChooseSpec::SpecificObject(target))
}

fn untap_spec(game: &mut GameState, source: ObjectId, target: ChooseSpec) -> usize {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    let outcome = UntapEffect::with_spec(target)
        .execute(game, &mut ctx)
        .unwrap();
    for event in outcome.events {
        game.queue_trigger_event(Default::default(), event);
    }
    settle(game, &mut dm)
}

#[test]
fn source_untap_triggers_both_complete_artifacts_round_trip_as_source_events() {
    assert_eq!(fixtures().len(), 2);
    for fixture in fixtures() {
        for definition in definitions(
            fixture["name"].as_str().unwrap(),
            fixture["text"].as_str().unwrap(),
        ) {
            assert!(definition.abilities.iter().any(|ability| {
                matches!(&ability.kind, AbilityKind::Triggered(ability)
                    if ability.trigger.compiled_model().is_some_and(|model| model.kind == TriggerKind::BecomesUntapped))
            }));
        }
    }
}

#[test]
fn source_untap_triggers_generic_nouns_keep_source_identity_controller_and_phasing() {
    for noun in [
        "artifact",
        "creature",
        "enchantment",
        "land",
        "planeswalker",
        "permanent",
        "Equipment",
    ] {
        for definition in definitions(
            "Untap source probe",
            &format!("Type: Artifact\nWhenever this {noun} becomes untapped, you gain 1 life."),
        ) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
            let other = game.create_object_from_definition(
                &compile_to_runtime_definition("Other", "Type: Artifact", false).unwrap(),
                B,
                Zone::Battlefield,
            );
            game.tap(other);
            assert_eq!(untap(&mut game, source, other), 0);
            assert_eq!(
                untap(&mut game, other, source),
                0,
                "already untapped is not an event"
            );
            game.tap(source);
            assert_eq!(untap(&mut game, other, source), 1);
            assert_eq!(game.player(B).unwrap().life, 21);
            assert_eq!(game.player(A).unwrap().life, 20);
            game.tap(source);
            game.phase_out(source);
            // Use real recipient discovery while the source is unavailable.
            // A forced SpecificObject ID bypasses that legal-choice boundary.
            assert_eq!(
                untap_spec(&mut game, other, ChooseSpec::All(ObjectFilter::permanent())),
                0
            );
            assert!(game.is_tapped(source));
            game.phase_in(source);
            assert_eq!(untap(&mut game, other, source), 1);
            assert_eq!(game.player(B).unwrap().life, 22);
        }
    }
}

struct UntapChoice(bool);
impl DecisionMaker for UntapChoice {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.0
    }
}

#[test]
fn source_untap_triggers_immovable_rod_ventures_only_after_actual_controller_untap() {
    let mut dungeon = CardDefinition::new(
        CardBuilder::new(CardId::new(), "Untap test dungeon")
            .card_types(vec![CardType::Dungeon])
            .build(),
    );
    dungeon.abilities.push(Ability::triggered(
        Trigger::dungeon_room("Entry", vec!["Exit".into()]),
        vec![Effect::gain_life(1)],
    ));
    dungeon.abilities.push(Ability::triggered(
        Trigger::dungeon_room("Exit", vec![]),
        vec![Effect::gain_life(2)],
    ));
    ironsmith::dungeon::register_dungeon_definition(&dungeon).unwrap();
    for definition in fixture("Immovable Rod") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let rod = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        game.tap(rod);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Untap);
        game.turn.active_player = A;
        ironsmith::turn::execute_untap_step_with(&mut game, &mut UntapChoice(true)).unwrap();
        assert!(game.is_tapped(rod));
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
        game.turn.active_player = B;
        ironsmith::turn::execute_untap_step_with(&mut game, &mut UntapChoice(false)).unwrap();
        assert!(
            game.is_tapped(rod),
            "the controller can choose to keep the Rod tapped"
        );
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
        ironsmith::turn::execute_untap_step_with(&mut game, &mut UntapChoice(true)).unwrap();
        assert!(!game.is_tapped(rod));
        assert!(
            game.active_dungeon(B).is_none(),
            "venturing waits for trigger resolution"
        );
        game.turn.step = Some(Step::Upkeep);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert_eq!(game.active_dungeon(B).unwrap().room_name, "Entry");
        assert_eq!(game.player(B).unwrap().life, 21);
        assert!(game.active_dungeon(A).is_none());
    }
}

#[test]
fn source_untap_triggers_key_to_the_city_can_pay_two_to_draw() {
    for definition in fixture("Key to the City") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let key = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let draw = compile_to_runtime_definition("Key draw card", "Type: Sorcery", false).unwrap();
        game.create_object_from_definition(&draw, B, Zone::Library);
        game.player_mut(B)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        game.tap(key);
        assert_eq!(untap(&mut game, key, key), 1);
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        let hand = &game.player(B).unwrap().hand;
        assert_eq!(hand.len(), 1);
        assert_eq!(game.object(hand[0]).unwrap().name, "Key draw card");
        assert!(game.player(A).unwrap().hand.is_empty());
    }
}
