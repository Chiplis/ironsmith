//! Metamorphosis Fanatic: "Lifelink. When this creature enters, return up to
//! one target creature card from your graveyard to the battlefield with a
//! lifelink counter on it. Miracle {1}{B}"
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{CardType, CounterType, Effect, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Metamorphosis Fanatic",
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

struct Choices {
    cast_for_miracle: bool,
    miracle_prompts: usize,
}

impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.miracle_prompts += 1;
        self.cast_for_miracle
    }

    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        ctx.requirements
            .iter()
            .filter_map(|requirement| requirement.legal_targets.first().cloned())
            .collect()
    }
}

struct Setup {
    game: GameState,
    fanatic_stable: ironsmith::ids::StableId,
    old_bear_stable: ironsmith::ids::StableId,
}

/// Alice has Metamorphosis Fanatic on top of her library, a creature card in
/// her graveyard, and exactly {1}{B} available.
fn setup() -> Setup {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.phase = ironsmith::game_state::Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Draw);
    let bear = CardDefinitionBuilder::new(CardId::new(), "Old Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let old_bear = game.create_object_from_definition(&bear, alice, Zone::Graveyard);
    let fanatic = game.create_object_from_definition(&def, alice, Zone::Library);
    let player = game.player_mut(alice).unwrap();
    player.mana_pool.add(ManaSymbol::Black, 1);
    player.mana_pool.add(ManaSymbol::Colorless, 1);
    Setup {
        fanatic_stable: game.object(fanatic).unwrap().stable_id,
        old_bear_stable: game.object(old_bear).unwrap().stable_id,
        game,
    }
}

fn zone_of(game: &GameState, stable: ironsmith::ids::StableId) -> Zone {
    game.object(game.find_object_by_stable_id(stable).unwrap())
        .unwrap()
        .zone
}

fn queue_and_stack(game: &mut GameState, events: Vec<ironsmith::triggers::TriggerEvent>, dm: &mut Choices) -> usize {
    let mut queue = TriggerQueue::new();
    for event in events {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    let count = queue.entries.len();
    if count > 0 {
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    }
    count
}

#[test]
fn first_draw_of_the_turn_can_be_cast_for_its_miracle_cost() {
    let Setup {
        mut game,
        fanatic_stable,
        old_bear_stable,
    } = setup();
    let alice = PlayerId::from_index(0);
    let mut dm = Choices {
        cast_for_miracle: true,
        miracle_prompts: 0,
    };
    let events = ironsmith::turn::execute_draw_step(&mut game);
    assert_eq!(zone_of(&game, fanatic_stable), Zone::Hand);
    assert_eq!(queue_and_stack(&mut game, events, &mut dm), 1, "miracle triggers on the first draw");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(dm.miracle_prompts, 1);
    assert_eq!(zone_of(&game, fanatic_stable), Zone::Stack, "cast for its miracle cost");
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0, "paid {{1}}{{B}}, not its mana cost");

    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(zone_of(&game, fanatic_stable), Zone::Battlefield);
    let pending = game.take_pending_trigger_events();
    assert_eq!(queue_and_stack(&mut game, pending, &mut dm), 1, "the enters trigger");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let bear = game.find_object_by_stable_id(old_bear_stable).unwrap();
    assert_eq!(game.object(bear).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.object(bear).unwrap().counters.get(&CounterType::Lifelink).copied(),
        Some(1),
        "returned with a lifelink counter"
    );
}

#[test]
fn declining_the_miracle_keeps_the_card_in_hand() {
    let Setup {
        mut game,
        fanatic_stable,
        ..
    } = setup();
    let alice = PlayerId::from_index(0);
    let mut dm = Choices {
        cast_for_miracle: false,
        miracle_prompts: 0,
    };
    let events = ironsmith::turn::execute_draw_step(&mut game);
    assert_eq!(queue_and_stack(&mut game, events, &mut dm), 1);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(zone_of(&game, fanatic_stable), Zone::Hand);
    assert!(game.stack.is_empty());
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 2);
}

#[test]
fn a_second_draw_in_the_turn_does_not_trigger_miracle() {
    let Setup {
        mut game,
        fanatic_stable,
        ..
    } = setup();
    let alice = PlayerId::from_index(0);
    // Put a different card on top so the draw step draws it first.
    let filler = CardDefinitionBuilder::new(CardId::new(), "Filler")
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_definition(&filler, alice, Zone::Library);
    let top = *game.player(alice).unwrap().library.last().unwrap();
    assert_eq!(game.object(top).unwrap().name, "Filler", "filler is on top");
    let mut dm = Choices {
        cast_for_miracle: true,
        miracle_prompts: 0,
    };
    let events = ironsmith::turn::execute_draw_step(&mut game);
    assert_eq!(queue_and_stack(&mut game, events, &mut dm), 0);

    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    let source: ObjectId = top;
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
    ironsmith::effects::execute_effect(&mut game, &Effect::draw(1), &mut ctx).unwrap();
    assert_eq!(zone_of(&game, fanatic_stable), Zone::Hand);
    let pending = game.take_pending_trigger_events();
    assert_eq!(queue_and_stack(&mut game, pending, &mut dm), 0, "not the first card drawn this turn");
    assert_eq!(dm.miracle_prompts, 0);
}
