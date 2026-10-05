//! Source-authored only. No build, compilation, test or replay has run.
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{DrawCardsEffect, EffectContext, EffectExecutor};
use ironsmith::events::CardsDrawnEvent;
use ironsmith::game_loop::{put_triggers_on_stack, resolve_stack_entry};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, ChooseSpec, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
fn a() -> PlayerId {
    PlayerId::from_index(0)
}
fn b() -> PlayerId {
    PlayerId::from_index(1)
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/draw_replacement_programs.json.fixture"
    ))
    .unwrap();
    let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        card["mana_cost"].as_str().unwrap(),
        card["type_line"].as_str().unwrap()
    );
    if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(card["oracle_text"].as_str().unwrap());
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn card(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    kind: CardType,
    zone: Zone,
) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&definition, player, zone)
}
fn fill_library(game: &mut GameState, player: PlayerId, count: usize) {
    for index in 0..count {
        card(
            game,
            player,
            &format!("Library card {index}"),
            CardType::Artifact,
            Zone::Library,
        );
    }
}
fn draw(game: &mut GameState, source: ObjectId, player: PlayerId, count: i32) -> usize {
    let mut dm = SelectFirstDecisionMaker;
    let outcome = DrawCardsEffect::you(count)
        .execute(game, &mut EffectContext::new(source, player, &mut dm))
        .unwrap();
    outcome
        .events
        .iter()
        .filter_map(|event| event.downcast::<CardsDrawnEvent>())
        .map(|event| event.amount() as usize)
        .sum()
}
fn settle(game: &mut GameState, queue: &mut TriggerQueue) {
    for _ in 0..16 {
        put_triggers_on_stack(game, queue).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry(game).unwrap();
    }
    panic!("bounded replacement follow-ups did not settle");
}
#[test]
fn seven_full_frozen_programs_keep_strict_serialized_artifacts() {
    for name in [
        "Tomorrow, Azami's Familiar",
        "Obstinate Familiar",
        "Possessed Portal",
        "Forbidden Crypt",
        "Out of the Tombs",
        "Chains of Mephistopheles",
        "Enduring Renewal",
    ] {
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
        }
    }
}
#[test]
fn tomorrow_uses_one_owned_look_choose_remainder_program_without_a_draw_event() {
    for definition in definitions("Tomorrow, Azami's Familiar") {
        let mut game = game();
        fill_library(&mut game, a(), 5);
        fill_library(&mut game, b(), 2);
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        assert_eq!(draw(&mut game, host, a(), 1), 0);
        assert_eq!(game.player(a()).unwrap().hand.len(), 1);
        assert_eq!(game.player(a()).unwrap().library.len(), 4);
        assert_eq!(
            draw(&mut game, host, b(), 1),
            1,
            "your replacement does not affect the opponent"
        );
        game.phase_out(host);
        assert_eq!(draw(&mut game, host, a(), 1), 1);
    }
}
struct OptionalDraw {
    decline: bool,
    pause: bool,
    pending: bool,
    calls: usize,
}
impl DecisionMaker for OptionalDraw {
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
    fn decide_options(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.calls += 1;
        self.pending = self.pause;
        let option = ctx
            .options
            .iter()
            .find(|option| {
                option.legal && option.description.starts_with("Do not apply") == self.decline
            })
            .unwrap();
        vec![option.index]
    }
}
#[test]
fn obstinate_decline_draws_normally_acceptance_cancels_only_this_draw_and_replay_is_atomic() {
    for definition in definitions("Obstinate Familiar") {
        for decline in [false, true] {
            let mut game = game();
            fill_library(&mut game, a(), 3);
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let mut dm = OptionalDraw {
                decline,
                pause: true,
                pending: false,
                calls: 0,
            };
            DrawCardsEffect::you(1)
                .execute(&mut game, &mut EffectContext::new(host, a(), &mut dm))
                .unwrap();
            assert!(game.player(a()).unwrap().hand.is_empty());
            assert_eq!(game.player(a()).unwrap().library.len(), 3);
            dm.pause = false;
            dm.pending = false;
            DrawCardsEffect::you(1)
                .execute(&mut game, &mut EffectContext::new(host, a(), &mut dm))
                .unwrap();
            assert_eq!(game.player(a()).unwrap().hand.len(), usize::from(decline));
            assert_eq!(dm.calls, 2);
            game.move_object_by_effect(host, Zone::Exile).unwrap();
            assert_eq!(
                draw(&mut game, host, a(), 1),
                1,
                "acceptance must not register a future skip"
            );
        }
    }
}
#[test]
fn forbidden_crypt_returns_a_card_then_loses_only_when_no_return_is_possible() {
    for definition in definitions("Forbidden Crypt") {
        let mut game = game();
        card(
            &mut game,
            a(),
            "Crypt memory",
            CardType::Artifact,
            Zone::Graveyard,
        );
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        assert_eq!(draw(&mut game, host, a(), 1), 0);
        assert_eq!(game.player(a()).unwrap().hand.len(), 1);
        assert!(!game.player(a()).unwrap().has_lost);
        let returned = game.player(a()).unwrap().hand[0];
        ironsmith::effects::execute_effect(
            &mut game,
            &Effect::move_to_zone(ChooseSpec::SpecificObject(returned), Zone::Graveyard, false),
            &mut EffectContext::new(host, a(), &mut SelectFirstDecisionMaker),
        )
        .unwrap();
        assert!(
            game.player(a()).unwrap().graveyard.is_empty(),
            "the second static replaces later graveyard entries with exile"
        );
        draw(&mut game, host, a(), 1);
        assert!(game.player(a()).unwrap().has_lost);
        assert!(!game.player(a()).unwrap().attempted_draw_from_empty_library);
    }
}
#[test]
fn out_of_tombs_gates_each_draw_and_shares_the_return_result_with_failure_branch() {
    for definition in definitions("Out of the Tombs") {
        let mut game = game();
        fill_library(&mut game, a(), 1);
        card(
            &mut game,
            a(),
            "Tomb creature",
            CardType::Creature,
            Zone::Graveyard,
        );
        card(
            &mut game,
            a(),
            "Tomb noncreature",
            CardType::Artifact,
            Zone::Graveyard,
        );
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        assert_eq!(draw(&mut game, host, a(), 1), 1);
        assert_eq!(draw(&mut game, host, a(), 1), 0);
        assert!(!game.player(a()).unwrap().has_lost);
        assert!(
            game.battlefield
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Tomb creature")
        );
        assert!(
            game.player(a())
                .unwrap()
                .graveyard
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Tomb noncreature")
        );
        draw(&mut game, host, a(), 1);
        assert!(game.player(a()).unwrap().has_lost);
        assert!(!game.player(a()).unwrap().attempted_draw_from_empty_library);
    }
}
#[test]
fn chains_uses_affected_drawer_and_preserves_the_discard_result_across_both_continuations() {
    for definition in definitions("Chains of Mephistopheles") {
        for initial_hand in [0, 1] {
            let mut game = game();
            fill_library(&mut game, a(), 4);
            fill_library(&mut game, b(), 4);
            if initial_hand == 1 {
                card(
                    &mut game,
                    b(),
                    "Discarded hand card",
                    CardType::Artifact,
                    Zone::Hand,
                );
            }
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            assert_eq!(
                draw(&mut game, host, b(), 2),
                if initial_hand == 1 { 2 } else { 0 }
            );
            assert_eq!(game.player(b()).unwrap().hand.len(), initial_hand);
            assert_eq!(game.player(b()).unwrap().library.len(), 2);
            assert_eq!(game.player(b()).unwrap().graveyard.len(), 2);
            assert_eq!(game.player(a()).unwrap().library.len(), 4);
            assert!(game.player(a()).unwrap().hand.is_empty());
        }
    }
}
#[test]
fn enduring_reveal_branch_does_not_redraw_creatures_but_still_returns_dead_creatures() {
    for definition in definitions("Enduring Renewal") {
        for creature in [false, true] {
            let mut game = game();
            card(
                &mut game,
                a(),
                "Renewal top",
                if creature {
                    CardType::Creature
                } else {
                    CardType::Artifact
                },
                Zone::Library,
            );
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            assert_eq!(draw(&mut game, host, a(), 1), usize::from(!creature));
            assert_eq!(game.player(a()).unwrap().hand.len(), usize::from(!creature));
            assert_eq!(
                game.player(a()).unwrap().graveyard.len(),
                usize::from(creature)
            );
            let dying = card(
                &mut game,
                a(),
                "Renewal battlefield creature",
                CardType::Creature,
                Zone::Battlefield,
            );
            game.move_object_by_effect(dying, Zone::Graveyard).unwrap();
            settle(&mut game, &mut TriggerQueue::new());
            assert!(
                game.player(a())
                    .unwrap()
                    .hand
                    .iter()
                    .any(|id| game.object(*id).unwrap().name == "Renewal battlefield creature")
            );
        }
    }
}
#[test]
fn possessed_portal_skips_each_players_draw_without_future_skip_or_empty_library_failure() {
    for definition in definitions("Possessed Portal") {
        let mut game = game();
        fill_library(&mut game, b(), 3);
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        for player in [a(), b()] {
            assert_eq!(draw(&mut game, host, player, 2), 0);
            assert!(game.player(player).unwrap().hand.is_empty());
            assert!(
                !game
                    .player(player)
                    .unwrap()
                    .attempted_draw_from_empty_library
            );
        }
        game.move_object_by_effect(host, Zone::Exile).unwrap();
        assert_eq!(draw(&mut game, host, b(), 1), 1);
    }
}

#[test]
fn chains_exempts_first_actual_draw_of_each_draw_step_not_first_draw_of_turn() {
    for definition in definitions("Chains of Mephistopheles") {
        let mut game = game();
        fill_library(&mut game, a(), 6);
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let mut queue = TriggerQueue::new();
        game.turn.phase = ironsmith::game_state::Phase::Beginning;
        game.turn.turn_number = 2;
        let mut first = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
            ironsmith::turn_runner::TurnState::Draw,
        );
        first.advance(&mut game, &mut queue).unwrap();
        assert_eq!(game.player(a()).unwrap().hand.len(), 1);
        assert_eq!(game.draw_step_context_for_player(a()), (true, 1));
        assert_eq!(draw(&mut game, host, a(), 1), 1);
        assert_eq!(game.player(a()).unwrap().hand.len(), 1);
        assert_eq!(game.player(a()).unwrap().graveyard.len(), 1);
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        game.sync_draw_step_tracking();
        let mut extra = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
            ironsmith::turn_runner::TurnState::Draw,
        );
        extra.advance(&mut game, &mut queue).unwrap();
        assert_eq!(
            game.player(a()).unwrap().hand.len(),
            2,
            "a distinct draw step has its own first draw"
        );
    }
}
#[test]
fn tombs_upkeep_retains_counter_then_dynamic_mill_before_replacement_draws() {
    for definition in definitions("Out of the Tombs") {
        let mut game = game();
        fill_library(&mut game, a(), 5);
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let mut queue = TriggerQueue::new();
        for (expected_counters, remaining) in [(2, 3), (4, 0)] {
            game.turn.phase = ironsmith::game_state::Phase::Beginning;
            let mut upkeep = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
                ironsmith::turn_runner::TurnState::Upkeep,
            );
            upkeep.advance(&mut game, &mut queue).unwrap();
            settle(&mut game, &mut queue);
            assert_eq!(
                game.object(host)
                    .unwrap()
                    .counters
                    .get(&ironsmith::object::CounterType::Eon),
                Some(&expected_counters)
            );
            assert_eq!(game.player(a()).unwrap().library.len(), remaining);
            assert!(!game.player(a()).unwrap().attempted_draw_from_empty_library);
        }
    }
}
#[test]
fn portal_end_step_body_still_processes_each_players_mandatory_alternative() {
    for definition in definitions("Possessed Portal") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        card(
            &mut game,
            a(),
            "Alice permanent",
            CardType::Artifact,
            Zone::Battlefield,
        );
        card(
            &mut game,
            b(),
            "Bob permanent",
            CardType::Artifact,
            Zone::Battlefield,
        );
        let before = game.battlefield.len();
        let mut queue = TriggerQueue::new();
        let mut end = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
            ironsmith::turn_runner::TurnState::EndStep,
        );
        end.advance(&mut game, &mut queue).unwrap();
        let mut dm = SelectFirstDecisionMaker;
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
            .unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            game.battlefield.len(),
            before - 2,
            "no hand cards means both players sacrifice, even if the Portal itself is sacrificed first"
        );
        assert!(!game.player(a()).unwrap().has_lost && !game.player(b()).unwrap().has_lost);
        let _ = host;
    }
}
