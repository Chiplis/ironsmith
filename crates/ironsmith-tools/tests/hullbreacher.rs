//! Hullbreacher: "If an opponent would draw a card except the first one they
//! draw in each of their draw steps, instead you create a Treasure token."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::AutoPassDecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_state::{Phase, Step};
use ironsmith::ids::CardId;
use ironsmith::{CardType, Effect, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Hullbreacher",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn bob() -> PlayerId {
    PlayerId::from_index(1)
}

fn filler(name: &str) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .build()
}

/// Alice controls Hullbreacher; both players have ten-card libraries.
fn setup() -> (GameState, ObjectId, ObjectId) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let hullbreacher = game.create_object_from_definition(&def, alice(), Zone::Battlefield);
    for player in [alice(), bob()] {
        for i in 0..10 {
            game.create_object_from_definition(
                &filler(&format!("Card {i}")),
                player,
                Zone::Library,
            );
        }
    }
    let bob_source =
        game.create_object_from_definition(&filler("Bob's Spell"), bob(), Zone::Battlefield);
    // Past the starting player's skipped first draw.
    game.turn.turn_number = 3;
    (game, hullbreacher, bob_source)
}

fn hand_size(game: &GameState, player: PlayerId) -> usize {
    game.player(player).unwrap().hand.len()
}

fn treasures(game: &GameState, player: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|id| {
            let obj = game.object(**id).unwrap();
            obj.name == "Treasure" && game.current_controller(**id) == Some(player)
        })
        .count()
}

fn draw(game: &mut GameState, player: PlayerId, source: ObjectId, count: u32) {
    let mut dm = AutoPassDecisionMaker;
    let mut ctx = EffectContext::new(source, player, &mut dm);
    execute_effect(game, &Effect::draw(count), &mut ctx).unwrap();
}

#[test]
fn opponents_first_draw_step_draw_is_kept_and_later_draws_become_treasures() {
    let (mut game, _hullbreacher, bob_source) = setup();
    game.turn.active_player = bob();
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Draw);

    ironsmith::turn::execute_draw_step(&mut game);
    assert_eq!(
        hand_size(&game, bob()),
        1,
        "the draw-step draw is not replaced"
    );
    assert_eq!(treasures(&game, alice()), 0);

    // An additional draw during that same draw step is replaced.
    game.turn.step = Some(Step::Draw);
    game.turn_store.tracked_draw_step_player = Some(bob());
    game.turn_store.cards_drawn_this_draw_step = 1;
    draw(&mut game, bob(), bob_source, 2);
    assert_eq!(
        hand_size(&game, bob()),
        1,
        "extra draw-step draws are replaced"
    );
    assert_eq!(
        treasures(&game, alice()),
        2,
        "each replaced draw makes a Treasure for Hullbreacher's controller"
    );
    assert_eq!(treasures(&game, bob()), 0);

    // Draws in Bob's main phase are replaced too.
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    draw(&mut game, bob(), bob_source, 1);
    assert_eq!(hand_size(&game, bob()), 1);
    assert_eq!(treasures(&game, alice()), 3);
}

#[test]
fn opponent_draws_on_controllers_turn_are_replaced_but_controller_draws_normally() {
    let (mut game, hullbreacher, bob_source) = setup();
    game.turn.active_player = alice();
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Draw);

    ironsmith::turn::execute_draw_step(&mut game);
    assert_eq!(
        hand_size(&game, alice()),
        1,
        "Hullbreacher never affects its controller"
    );

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    draw(&mut game, alice(), hullbreacher, 2);
    assert_eq!(hand_size(&game, alice()), 3);
    assert_eq!(treasures(&game, alice()), 0);

    // Bob's very first draw of the turn is replaced: it is not in his draw step.
    draw(&mut game, bob(), bob_source, 1);
    assert_eq!(hand_size(&game, bob()), 0);
    assert_eq!(treasures(&game, alice()), 1);
}

#[test]
fn replacement_stops_when_hullbreacher_leaves() {
    let (mut game, hullbreacher, bob_source) = setup();
    game.turn.active_player = alice();
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.move_object(
        hullbreacher,
        Zone::Graveyard,
        ironsmith::events::cause::EventCause::effect(),
    );
    draw(&mut game, bob(), bob_source, 1);
    assert_eq!(hand_size(&game, bob()), 1);
    assert_eq!(treasures(&game, alice()), 0);
}

#[test]
fn replaced_draw_creates_an_artifact_treasure_token_owned_by_controller() {
    let (mut game, _hullbreacher, bob_source) = setup();
    game.turn.active_player = alice();
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    draw(&mut game, bob(), bob_source, 1);
    let treasure = *game
        .battlefield
        .iter()
        .find(|id| game.object(**id).unwrap().name == "Treasure")
        .unwrap();
    let obj = game.object(treasure).unwrap();
    assert_eq!(obj.kind, ironsmith::object::ObjectKind::Token);
    assert!(obj.card_types.contains(&CardType::Artifact));
    assert_eq!(obj.owner, alice());
}
