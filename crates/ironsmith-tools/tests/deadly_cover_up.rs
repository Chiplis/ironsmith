//! Deadly Cover-Up: "As an additional cost to cast this spell, you may collect
//! evidence 6. Destroy all creatures. If evidence was collected, exile a card
//! from an opponent's graveyard. Then search its owner's graveyard, hand, and
//! library for any number of cards with that name and exile them. That player
//! shuffles, then draws a card for each card exiled from their hand this way."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{SelectObjectsContext, SelectOptionsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Deadly Cover-Up",
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

const SECRET: &str = "Secret Plan";

#[derive(Default)]
struct Choices {
    collect: bool,
    offered_evidence: bool,
    /// Candidate names offered for the "exile a card from an opponent's graveyard" choice.
    graveyard_choice: Vec<String>,
    /// Zones of the candidates offered by the name search.
    searched_zones: Vec<Zone>,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            self.offered_evidence = true;
            return if self.collect { vec![0] } else { Vec::new() };
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .take(ctx.min)
            .map(|option| option.index)
            .collect()
    }

    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal: Vec<_> = ctx.candidates.iter().filter(|c| c.legal).collect();
        if ctx.aggregate_constraint.is_some() {
            return legal
                .iter()
                .filter(|c| c.name.starts_with("Evidence"))
                .map(|c| c.id)
                .collect();
        }
        if ctx.max == Some(1) {
            self.graveyard_choice = legal.iter().map(|c| c.name.clone()).collect();
            return legal
                .iter()
                .filter(|c| c.name == SECRET)
                .map(|c| c.id)
                .take(1)
                .collect();
        }
        // The name search: take every copy found.
        self.searched_zones = legal
            .iter()
            .map(|c| game.object(c.id).unwrap().zone)
            .collect();
        legal.iter().map(|c| c.id).collect()
    }
}

fn card(name: &str, types: Vec<CardType>, mana_value: u8) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(types.clone())
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(mana_value)]));
    if types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

fn count_named(game: &GameState, zone_ids: &[ObjectId], name: &str) -> usize {
    zone_ids
        .iter()
        .filter(|id| game.object(**id).unwrap().name == name)
        .count()
}

struct Outcome {
    game: GameState,
    dm: Choices,
}

/// Bob's graveyard holds one Secret Plan and one Other Card; his hand holds two
/// Secret Plans and a Filler; his library holds one Secret Plan and four Fillers.
fn cast(collect: bool) -> Outcome {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Black, 5);
    let spell = game.create_object_from_definition(&def, alice, Zone::Hand);
    for name in ["Evidence A", "Evidence B"] {
        game.create_object_from_definition(&card(name, vec![CardType::Instant], 3), alice, Zone::Graveyard);
    }
    for (owner, name) in [(alice, "Alice Bear"), (bob, "Bob Bear")] {
        game.create_object_from_definition(&card(name, vec![CardType::Creature], 2), owner, Zone::Battlefield);
    }
    let secret = card(SECRET, vec![CardType::Instant], 1);
    let filler = card("Filler", vec![CardType::Instant], 1);
    game.create_object_from_definition(&secret, bob, Zone::Graveyard);
    game.create_object_from_definition(&card("Other Card", vec![CardType::Instant], 1), bob, Zone::Graveyard);
    for _ in 0..2 {
        game.create_object_from_definition(&secret, bob, Zone::Hand);
    }
    game.create_object_from_definition(&filler, bob, Zone::Hand);
    game.create_object_from_definition(&secret, bob, Zone::Library);
    for _ in 0..4 {
        game.create_object_from_definition(&filler, bob, Zone::Library);
    }

    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Choices {
        collect,
        ..Default::default()
    };
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
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
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    assert!(dm.offered_evidence);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    Outcome { game, dm }
}

#[test]
fn without_evidence_only_destroys_all_creatures() {
    let Outcome { game, dm } = cast(false);
    let bob = PlayerId::from_index(1);
    assert!(game.battlefield.is_empty(), "every creature is destroyed");
    assert!(dm.graveyard_choice.is_empty(), "no graveyard exile without evidence");
    let bob_state = game.player(bob).unwrap();
    assert_eq!(bob_state.hand.len(), 3);
    assert_eq!(bob_state.library.len(), 5);
    assert_eq!(count_named(&game, &bob_state.graveyard, SECRET), 1);
    assert_eq!(count_named(&game, &game.exile, SECRET), 0);
}

#[test]
fn collected_evidence_exiles_every_copy_and_replaces_only_hand_cards() {
    let Outcome { game, dm } = cast(true);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    assert!(game.battlefield.is_empty(), "every creature is destroyed");
    assert!(
        dm.graveyard_choice.iter().any(|name| name == SECRET)
            && dm.graveyard_choice.iter().all(|name| name != "Evidence A"),
        "the chosen card comes from an opponent's graveyard: {:?}",
        dm.graveyard_choice
    );
    let mut searched = dm.searched_zones.clone();
    searched.sort_by_key(|zone| format!("{zone:?}"));
    assert_eq!(searched, vec![Zone::Hand, Zone::Hand, Zone::Library], "search spans hand and library copies");

    let bob_state = game.player(bob).unwrap();
    assert_eq!(count_named(&game, &game.exile, SECRET), 4, "the chosen card plus all three copies");
    assert_eq!(count_named(&game, &bob_state.graveyard, SECRET), 0);
    assert_eq!(count_named(&game, &bob_state.hand, SECRET), 0);
    assert_eq!(count_named(&game, &bob_state.library, SECRET), 0);
    assert_eq!(bob_state.hand.len(), 3, "one Filler kept plus two draws for two hand exiles");
    assert_eq!(bob_state.library.len(), 2, "five minus one exiled minus two drawn");
    assert_eq!(count_named(&game, &bob_state.graveyard, "Other Card"), 1);
    assert_eq!(
        count_named(&game, &game.exile, "Evidence A") + count_named(&game, &game.exile, "Evidence B"),
        2,
        "evidence exiled as the cost"
    );
    assert_eq!(game.player(alice).unwrap().hand.len(), 0, "the caster draws nothing");
}
