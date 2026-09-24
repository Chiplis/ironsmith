//! Once Upon a Time: "If this spell is the first spell you've cast this game,
//! you may cast it without paying its mana cost. Look at the top five cards of
//! your library. You may reveal a creature or land card from among them and put
//! it into your hand. Put the rest on the bottom of your library in a random
//! order."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Once Upon a Time",
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

/// Takes the named card from among the looked-at cards.
struct Take(&'static str);

impl DecisionMaker for Take {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        true
    }

    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .filter(|c| c.legal && c.name.as_str() == self.0)
            .map(|c| c.id)
            .collect()
    }
}

fn card(name: &str, card_type: CardType) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(1, 1));
    }
    builder.build()
}

fn setup() -> (GameState, ObjectId) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    // Library bottom-first: the top five are Sorcery A, Bear, Sorcery B,
    // Sorcery C, Sorcery D; Bottom Card sits below them.
    for (name, card_type) in [
        ("Bottom Card", CardType::Sorcery),
        ("Sorcery D", CardType::Sorcery),
        ("Sorcery C", CardType::Sorcery),
        ("Sorcery B", CardType::Sorcery),
        ("Bear", CardType::Creature),
        ("Sorcery A", CardType::Sorcery),
    ] {
        game.create_object_from_definition(&card(name, card_type), alice, Zone::Library);
    }
    let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    (game, hand)
}

fn castable(game: &GameState, spell: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, PlayerId::from_index(0))
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
}

fn cast(game: &mut GameState, action: LegalAction, dm: &mut impl DecisionMaker) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm);
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
}

#[test]
fn first_spell_of_the_game_is_free_and_finds_a_creature() {
    let (mut game, spell) = setup();
    let alice = PlayerId::from_index(0);
    let action = castable(&game, spell).expect("free as the first spell of the game");
    cast(&mut game, action, &mut Take("Bear"));
    let player = game.player(alice).unwrap();
    let hand: Vec<String> = player.hand.iter().map(|id| game.object(*id).unwrap().name.to_string()).collect();
    assert_eq!(hand, vec!["Bear".to_string()]);
    let library: Vec<String> = player.library.iter().map(|id| game.object(*id).unwrap().name.to_string()).collect();
    assert_eq!(library.len(), 5);
    assert_eq!(library.last().map(String::as_str), Some("Bottom Card"), "the rest go under it: {library:?}");
}

#[test]
fn not_free_after_you_have_cast_another_spell_this_game() {
    let (mut game, spell) = setup();
    let alice = PlayerId::from_index(0);
    let cheap = CardDefinitionBuilder::new(CardId::new(), "Opening Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let opening = game.create_object_from_definition(&cheap, alice, Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
    let action = castable(&game, opening).expect("opening spell castable");
    cast(&mut game, action, &mut Take("none"));
    // A later turn: the count is per game, not per turn.
    game.turn_store.turn_history.clear_for_new_turn();
    game.turn.turn_number += 2;
    assert!(castable(&game, spell).is_none(), "no longer the first spell, and no mana");
}
