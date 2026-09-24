//! Deep Analysis: "Target player draws two cards. Flashback—{1}{U}, Pay 3 life."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Deep Analysis",
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

struct TargetPlayer(PlayerId);

impl DecisionMaker for TargetPlayer {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert!(ctx.requirements[0].legal_targets.contains(&Target::Player(self.0)));
        vec![Target::Player(self.0)]
    }
}

/// Alice holds or has Deep Analysis in `zone`, 4 blue mana, and `life` life;
/// both players have ten-card libraries.
fn setup(zone: Zone, life: i32) -> (GameState, ironsmith::ObjectId) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().life = life;
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Blue, 4);
    for player in [alice, PlayerId::from_index(1)] {
        for i in 0..10 {
            let card = CardDefinitionBuilder::new(CardId::new(), &format!("Card {i}"))
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_definition(&card, player, Zone::Library);
        }
    }
    let spell = game.create_object_from_definition(&def, alice, zone);
    (game, spell)
}

fn cast(game: &mut GameState, spell: ironsmith::ObjectId, target: PlayerId) -> Result<(), String> {
    let alice = PlayerId::from_index(0);
    let Some(action) = compute_legal_actions(game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
    else {
        return Err("not castable".into());
    };
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = TargetPlayer(target);
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..32 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    if game.stack.len() != 1 {
        return Err(format!("{result:?}"));
    }
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    Ok(())
}

#[test]
fn hand_cast_makes_the_target_player_draw_two() {
    let (mut game, spell) = setup(Zone::Hand, 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let stable = game.object(spell).unwrap().stable_id;
    cast(&mut game, spell, bob).unwrap();
    assert_eq!(game.player(bob).unwrap().hand.len(), 2);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0, "paid {{3}}{{U}}");
    assert_eq!(game.player(alice).unwrap().life, 20);
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Graveyard);
}

#[test]
fn flashback_pays_mana_and_three_life_and_exiles_the_card() {
    let (mut game, spell) = setup(Zone::Graveyard, 20);
    let alice = PlayerId::from_index(0);
    let stable = game.object(spell).unwrap().stable_id;
    cast(&mut game, spell, alice).unwrap();
    assert_eq!(game.player(alice).unwrap().hand.len(), 2);
    assert_eq!(game.player(alice).unwrap().life, 17, "paid 3 life");
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 2, "paid {{1}}{{U}}");
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Exile);
}

#[test]
fn flashback_cannot_pay_more_life_than_you_have() {
    let (mut game, spell) = setup(Zone::Graveyard, 2);
    let alice = PlayerId::from_index(0);
    let stable = game.object(spell).unwrap().stable_id;
    assert!(cast(&mut game, spell, alice).is_err(), "CR 119.4: can't pay 3 life at 2 life");
    assert_eq!(game.player(alice).unwrap().life, 2);
    let now = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(now).unwrap().zone, Zone::Graveyard);
}
