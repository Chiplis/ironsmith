//! Sephiroth's death trigger targets an opponent. The UI auto-places a target
//! only when the requirement leaves no choice, so what the engine reports as
//! legal is exactly what decides whether that happens.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_state::Target;
use ironsmith::card::PowerToughness;
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, PlayerId, Zone};

#[derive(Default)]
struct CaptureLegal {
    seen: Vec<Vec<Target>>,
    mins: Vec<usize>,
}

impl DecisionMaker for CaptureLegal {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        for requirement in &ctx.requirements {
            self.seen.push(requirement.legal_targets.clone());
            self.mins.push(requirement.min_targets);
        }
        ctx.requirements
            .first()
            .and_then(|r| r.legal_targets.first().copied())
            .into_iter()
            .collect()
    }
}

fn legal_targets_for_players(player_count: usize) -> (Vec<Vec<Target>>, Vec<usize>) {
    let payloads = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Sephiroth, Fabled SOLDIER",
    )
    .unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();

    let names: Vec<String> = (0..player_count).map(|i| format!("P{i}")).collect();
    let mut game = GameState::new(names, 20);
    let alice = PlayerId::from_index(0);

    let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
    game.move_object_with_etb_processing(hand, Zone::Battlefield)
        .unwrap();
    let _ = game.take_pending_trigger_events();

    // Another creature dies: the trigger's event.
    let bear = CardDefinitionBuilder::new(CardId::new(), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let doomed = game.create_object_from_definition(&bear, alice, Zone::Battlefield);
    game.move_object_by_effect(doomed, Zone::Graveyard);

    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        for entry in ironsmith::triggers::check_triggers(&game, &event) {
            queue.add(entry);
        }
    }

    let mut dm = CaptureLegal::default();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    (dm.seen, dm.mins)
}

#[test]
fn sephiroth_trigger_reports_every_opponent_as_legal() {
    let (two_player, mins) = legal_targets_for_players(2);
    println!("2 players -> {two_player:#?} mins {mins:?}");
    assert_eq!(two_player.len(), 1, "one target requirement");
    assert_eq!(mins, vec![1], "mandatory single target");
    assert_eq!(
        two_player[0],
        vec![Target::Player(PlayerId::from_index(1))],
        "the lone opponent is the only legal target"
    );

    let (four_player, mins) = legal_targets_for_players(4);
    println!("4 players -> {four_player:#?} mins {mins:?}");
    assert_eq!(mins, vec![1]);
    assert_eq!(
        four_player[0].len(),
        3,
        "every opponent is legal, so the player really is choosing"
    );
}
