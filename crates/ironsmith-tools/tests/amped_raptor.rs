//! Amped Raptor: "First strike. When this creature enters, you get {E}{E}.
//! Then if you cast it from your hand, exile cards from the top of your
//! library until you exile a nonland card. You may cast that card by paying an
//! amount of {E} equal to its mana value rather than paying its mana cost."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::BooleanContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, PlayerId, Supertype, Zone};

fn load(name: &str) -> ironsmith::cards::CardDefinition {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        name,
    )
    .unwrap()
    .remove(0);
    ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Amped Raptor",
    )
    .unwrap()
    .remove(0);
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload);
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

struct Accept;

impl DecisionMaker for Accept {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        true
    }
}

struct Outcome {
    energy: u32,
    /// Zone of the nonland card revealed from the library.
    spell_zone: Zone,
    /// Zone of the land exiled on the way to it.
    land_zone: Zone,
    hand_size: usize,
}

/// Alice's library (top first): a land, then `spell`, then a filler card.
/// With `from_hand`, Alice casts Amped Raptor; otherwise it is put onto the
/// battlefield directly. The enter trigger and any spell it casts resolve.
fn run(spell: &str, from_hand: bool, starting_energy: u32) -> Outcome {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().energy_counters = starting_energy;
    let filler = CardDefinitionBuilder::new(CardId::new(), "Filler")
        .card_types(vec![CardType::Sorcery])
        .build();
    let land = CardDefinitionBuilder::new(CardId::new(), "Wastes")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .build();
    // Library is created bottom-first.
    game.create_object_from_definition(&filler, alice, Zone::Library);
    game.create_object_from_definition(&filler, alice, Zone::Library);
    let spell_id = game.create_object_from_definition(&load(spell), alice, Zone::Library);
    let spell_stable = game.object(spell_id).unwrap().stable_id;
    let land_id = game.create_object_from_definition(&land, alice, Zone::Library);
    let land_stable = game.object(land_id).unwrap().stable_id;
    let raptor = game.create_object_from_definition(&load("Amped Raptor"), alice, Zone::Hand);

    let mut queue = TriggerQueue::new();
    let mut dm = Accept;
    if from_hand {
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 2);
        let action = compute_legal_actions(&game, alice)
            .into_iter()
            .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == raptor))
            .expect("Amped Raptor is castable");
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        );
        for _ in 0..12 {
            if !game.stack.is_empty() || progress.is_err() {
                break;
            }
            let Ok(GameProgress::NeedsDecisionCtx(ctx)) = progress else {
                break;
            };
            progress = ironsmith::game_loop::apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &ctx, &mut dm,
            );
        }
        assert_eq!(game.stack.len(), 1, "{progress:?}");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    } else {
        game.move_object_by_effect(raptor, Zone::Battlefield).unwrap();
    }
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1, "enter trigger");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    // Resolve a spell cast during the trigger, if any.
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    }
    let zone_of = |stable| {
        game.object(game.find_object_by_stable_id(stable).unwrap())
            .unwrap()
            .zone
    };
    Outcome {
        energy: game.player(alice).unwrap().energy_counters,
        spell_zone: zone_of(spell_stable),
        land_zone: zone_of(land_stable),
        hand_size: game.player(alice).unwrap().hand.len(),
    }
}

#[test]
fn cast_from_hand_exiles_to_a_nonland_card_and_casts_it_for_energy() {
    // Opt ({U}): scry 1, then draw a card. Mana value 1.
    let outcome = run("Opt", true, 0);
    assert_eq!(outcome.land_zone, Zone::Exile, "the land is exiled on the way");
    assert_eq!(outcome.spell_zone, Zone::Graveyard, "Opt was cast and resolved");
    assert_eq!(outcome.energy, 1, "two energy gained, one paid for Opt");
    assert_eq!(outcome.hand_size, 1, "Opt drew a card");
}

#[test]
fn not_enough_energy_leaves_the_card_in_exile() {
    // Divination ({2}{U}) has mana value 3 but Alice has only {E}{E}.
    let outcome = run("Divination", true, 0);
    assert_eq!(outcome.spell_zone, Zone::Exile);
    assert_eq!(outcome.energy, 2, "no energy is paid");
    assert_eq!(outcome.hand_size, 0);
}

#[test]
fn stored_energy_pays_for_a_bigger_spell() {
    let outcome = run("Divination", true, 1);
    assert_eq!(outcome.spell_zone, Zone::Graveyard, "Divination was cast");
    assert_eq!(outcome.energy, 0, "three energy paid");
    assert_eq!(outcome.hand_size, 2, "Divination drew two cards");
}

#[test]
fn not_cast_from_hand_only_gives_energy() {
    let outcome = run("Opt", false, 0);
    assert_eq!(outcome.energy, 2);
    assert_eq!(outcome.land_zone, Zone::Library, "nothing is exiled");
    assert_eq!(outcome.spell_zone, Zone::Library);
}
