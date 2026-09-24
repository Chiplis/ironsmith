//! Demonic Counsel: "Search your library for a Demon card, reveal it, put it
//! into your hand, then shuffle. Delirium — If there are four or more card
//! types among cards in your graveyard, instead search your library for any
//! card, put it into your hand, then shuffle."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Subtype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Demonic Counsel",
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

/// Records the searchable candidates and takes the named one when legal.
struct Search {
    want: &'static str,
    offered: Vec<String>,
}

impl DecisionMaker for Search {
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.offered = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.name.to_string())
            .collect();
        ctx.candidates
            .iter()
            .filter(|c| c.legal && c.name.as_str() == self.want)
            .map(|c| c.id)
            .take(1)
            .collect()
    }
}

fn card(name: &str, card_type: CardType, subtypes: Vec<Subtype>) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .subtypes(subtypes);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(5, 5));
    }
    builder.build()
}

/// Casts Demonic Counsel with `graveyard_types` distinct card types in Alice's
/// graveyard; returns the legal search candidates and Alice's hand.
fn cast(graveyard_types: usize, want: &'static str) -> (Vec<String>, Vec<String>) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    for (name, card_type) in [
        ("Grave Land", CardType::Land),
        ("Grave Artifact", CardType::Artifact),
        ("Grave Instant", CardType::Instant),
        ("Grave Enchantment", CardType::Enchantment),
    ]
    .into_iter()
    .take(graveyard_types)
    {
        game.create_object_from_definition(&card(name, card_type, vec![]), alice, Zone::Graveyard);
    }
    game.create_object_from_definition(&card("Demon", CardType::Creature, vec![Subtype::Demon]), alice, Zone::Library);
    game.create_object_from_definition(&card("Tutor Target", CardType::Sorcery, vec![]), alice, Zone::Library);
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let spell = game.create_object_from_definition(&def, alice, Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Black, 1);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Search { want, offered: Vec::new() };
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
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
        result = ironsmith::game_loop::apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm);
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let hand = game
        .player(alice)
        .unwrap()
        .hand
        .iter()
        .map(|id| game.object(*id).unwrap().name.to_string())
        .collect();
    let mut offered = dm.offered;
    offered.sort();
    (offered, hand)
}

#[test]
fn without_delirium_only_a_demon_can_be_found() {
    let (offered, hand) = cast(3, "Demon");
    assert_eq!(offered, vec!["Demon".to_string()]);
    assert_eq!(hand, vec!["Demon".to_string()]);
}

#[test]
fn with_delirium_any_card_can_be_found_instead() {
    let (offered, hand) = cast(4, "Tutor Target");
    assert_eq!(offered, vec!["Demon".to_string(), "Tutor Target".to_string()]);
    assert_eq!(hand, vec!["Tutor Target".to_string()], "one search, not two");
}
