//! Campfire: "{1}, {T}: You gain 2 life. {2}, {T}, Exile this artifact: Put
//! all commanders you own from the command zone and from your graveyard into
//! your hand. Then shuffle your graveyard into your library."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Campfire",
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

fn creature(name: &str) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build()
}

/// Declines the optional "put it into the command zone instead" replacement
/// (CR 903.9b) so the commanders reach the hand.
struct KeepInHand;

impl DecisionMaker for KeepInHand {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        false
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let keep: Vec<usize> = ctx
            .options
            .iter()
            .filter(|o| o.legal && !o.description.to_ascii_lowercase().contains("command zone"))
            .map(|o| o.index)
            .take(ctx.min.max(1))
            .collect();
        if keep.is_empty() {
            ctx.options
                .iter()
                .filter(|o| o.legal)
                .take(ctx.min.max(1))
                .map(|o| o.index)
                .collect()
        } else {
            keep
        }
    }
}

fn names(game: &GameState, ids: impl Iterator<Item = ObjectId>) -> Vec<String> {
    let mut names: Vec<String> = ids
        .map(|id| game.object(id).unwrap().name.to_string())
        .collect();
    names.sort();
    names
}

fn activate(game: &mut GameState, source: ObjectId, ability_index: usize) {
    let alice = PlayerId::from_index(0);
    let action = compute_legal_actions(game, alice)
        .into_iter()
        .find(|a| {
            matches!(a, LegalAction::ActivateAbility { source: s, ability_index: i, .. }
                if *s == source && *i == ability_index)
        })
        .expect("activatable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = KeepInHand;
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
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
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
}

#[test]
fn gains_two_life() {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let campfire = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    activate(&mut game, campfire, 0);
    assert_eq!(game.player(alice).unwrap().life, 22);
    assert!(game.is_tapped(campfire));
}

#[test]
fn returns_owned_commanders_from_command_zone_and_graveyard_then_shuffles_graveyard() {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let campfire = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let in_command =
        game.create_object_from_definition(&creature("Commander A"), alice, Zone::Command);
    let in_graveyard =
        game.create_object_from_definition(&creature("Commander B"), alice, Zone::Graveyard);
    let on_battlefield =
        game.create_object_from_definition(&creature("Commander C"), alice, Zone::Battlefield);
    let bobs = game.create_object_from_definition(&creature("Bob Commander"), bob, Zone::Command);
    for id in [in_command, in_graveyard, on_battlefield] {
        game.player_mut(alice).unwrap().add_commander(id);
    }
    game.player_mut(bob).unwrap().add_commander(bobs);
    game.create_object_from_definition(&creature("Graveyard Bear"), alice, Zone::Graveyard);
    game.create_object_from_definition(&creature("Library Bear"), alice, Zone::Library);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    assert!(game.is_commander(in_graveyard));
    activate(&mut game, campfire, 1);

    let player = game.player(alice).unwrap();
    assert_eq!(
        names(&game, player.hand.iter().copied()),
        vec!["Commander A".to_string(), "Commander B".to_string()],
        "commanders from the command zone and the graveyard"
    );
    assert!(player.graveyard.is_empty(), "graveyard shuffled away");
    assert_eq!(
        names(&game, player.library.iter().copied()),
        vec!["Graveyard Bear".to_string(), "Library Bear".to_string()]
    );
    assert!(
        game.battlefield
            .iter()
            .any(|id| game.object(*id).is_some_and(|o| o.name == "Commander C"))
    );
    assert!(
        game.player(bob).unwrap().hand.is_empty(),
        "only commanders you own"
    );
    assert!(
        game.exile
            .iter()
            .any(|id| game.object(*id).is_some_and(|o| o.name == "Campfire")),
        "exiled as a cost"
    );
}
