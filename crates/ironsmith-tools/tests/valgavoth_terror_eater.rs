//! Valgavoth, Terror Eater: "If a card you didn't control would be put into
//! an opponent's graveyard from anywhere, exile it instead. During your turn,
//! you may play cards exiled with Valgavoth. If you cast a spell this way,
//! pay life equal to its mana value rather than pay its mana cost."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::target::ChooseSpec;
use ironsmith::{CardType, GameState, ObjectId, PlayerFilter, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Valgavoth, Terror Eater",
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

fn sorcery(name: &str) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Red]]))
        .build()
}

fn creature(name: &str) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

/// Alice controls Valgavoth. Bob mills his top card (a sorcery) and loses
/// a creature he controls; a creature Bob owns but Alice controls dies too.
/// Returns the game plus (milled sorcery, Bob's creature, stolen creature).
fn setup() -> (GameState, [ObjectId; 3]) {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let valgavoth = game.create_object_from_definition(
        &ironsmith_tools::compile_definition_from_payload(&payload()).unwrap(),
        alice,
        Zone::Battlefield,
    );
    let sorcery_stable = {
        let id = game.create_object_from_definition(&sorcery("Bob's Sorcery"), bob, Zone::Library);
        game.object(id).unwrap().stable_id
    };
    let bobs = game.create_object_from_definition(&creature("Bob's Bear"), bob, Zone::Battlefield);
    let bobs_stable = game.object(bobs).unwrap().stable_id;
    let stolen = game.create_object_from_definition(&creature("Stolen Bear"), bob, Zone::Battlefield);
    let stolen_stable = game.object(stolen).unwrap().stable_id;
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(valgavoth, alice, &mut dm);
    ironsmith::effects::execute_effect(
        &mut game,
        &ironsmith::Effect::gain_control_with_duration(
            ChooseSpec::SpecificObject(stolen),
            ironsmith::effect::Until::Forever,
        ),
        &mut ctx,
    )
    .unwrap();
    game.refresh_continuous_state();
    assert_eq!(game.controller_of_id(stolen), Some(alice));
    ironsmith::effects::execute_effect(
        &mut game,
        &ironsmith::Effect::mill_player(1, PlayerFilter::Specific(bob)),
        &mut ctx,
    )
    .unwrap();
    for victim in [bobs, stolen] {
        ironsmith::effects::execute_effect(
            &mut game,
            &ironsmith::Effect::destroy(ChooseSpec::SpecificObject(victim)),
            &mut ctx,
        )
        .unwrap();
    }
    let ids = [sorcery_stable, bobs_stable, stolen_stable]
        .map(|stable| game.find_object_by_stable_id(stable).unwrap());
    (game, ids)
}

#[test]
fn cards_an_opponent_would_lose_to_the_graveyard_are_exiled_unless_you_controlled_them() {
    let (game, [sorcery, bobs, stolen]) = setup();
    assert_eq!(game.object(sorcery).unwrap().zone, Zone::Exile, "milled");
    assert_eq!(game.object(bobs).unwrap().zone, Zone::Exile, "destroyed under Bob's control");
    assert_eq!(game.object(stolen).unwrap().zone, Zone::Graveyard, "you controlled it");
}

#[test]
fn exiled_cards_are_playable_on_your_turn_for_life() {
    let alice = PlayerId::from_index(0);
    let (mut game, [sorcery, bobs, _]) = setup();
    let cast = |game: &GameState, card: ObjectId| {
        compute_legal_actions(game, PlayerId::from_index(0))
            .into_iter()
            .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
    };
    let action = cast(&game, sorcery).expect("castable from exile without mana");
    assert!(cast(&game, bobs).is_some(), "any card exiled with Valgavoth");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm);
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    assert_eq!(game.player(alice).unwrap().life, 17, "paid life equal to mana value 3");
}

#[test]
fn not_during_an_opponents_turn() {
    let (mut game, [sorcery, _, _]) = setup();
    game.turn.active_player = PlayerId::from_index(1);
    let actions = compute_legal_actions(&game, PlayerId::from_index(0));
    assert!(!actions
        .iter()
        .any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == sorcery)));
}
