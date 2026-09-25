//! World War Hulk: "I — The next red or green creature spell you cast this
//! turn can be cast without paying its mana cost. II — Put three +1/+1
//! counters on target creature you control. III — Choose target creature you
//! control. Until end of turn, double its power and toughness and it gains
//! trample."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::effects::ResolvedTarget;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{AbilityKind, CardDefinition, CardType, GameState, ObjectId, PlayerId, Zone};

fn definition() -> CardDefinition {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "World War Hulk",
    )
    .unwrap()
    .remove(0);
    ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "World War Hulk",
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

/// Resolves chapter `index` (0-based) of the Saga with `targets`.
fn resolve_chapter(
    game: &mut GameState,
    saga: ObjectId,
    index: usize,
    targets: Vec<ResolvedTarget>,
) {
    let definition = definition();
    let AbilityKind::Triggered(triggered) = &definition.abilities[index].kind else {
        panic!("chapter ability");
    };
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(saga, PlayerId::from_index(0), &mut dm)
        .with_targets(targets);
    for effect in triggered.effects.all_effects() {
        ironsmith::effects::execute_effect(game, effect, &mut ctx).unwrap();
    }
    game.refresh_continuous_state();
}

fn creature(name: &str, color: ManaSymbol) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![color],
        ]))
        .power_toughness(PowerToughness::fixed(4, 4))
        .build()
}

/// Alice's first main phase with no mana; chapter I has resolved.
fn after_chapter_one() -> (GameState, [ObjectId; 3]) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let saga = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
    let red = game.create_object_from_definition(
        &creature("Red Brute", ManaSymbol::Red),
        alice,
        Zone::Hand,
    );
    let green = game.create_object_from_definition(
        &creature("Green Brute", ManaSymbol::Green),
        alice,
        Zone::Hand,
    );
    let blue = game.create_object_from_definition(
        &creature("Blue Brute", ManaSymbol::Blue),
        alice,
        Zone::Hand,
    );
    resolve_chapter(&mut game, saga, 0, Vec::new());
    (game, [red, green, blue])
}

fn cast_action(game: &GameState, spell: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, PlayerId::from_index(0))
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
}

fn cast(game: &mut GameState, action: LegalAction) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
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
}

#[test]
fn chapter_one_lets_the_next_red_or_green_creature_be_cast_free_once() {
    let (mut game, [red, green, blue]) = after_chapter_one();
    assert!(
        cast_action(&game, blue).is_none(),
        "blue creatures are not covered"
    );
    let action = cast_action(&game, red).expect("red creature castable without mana");
    assert!(cast_action(&game, green).is_some());
    cast(&mut game, action);
    assert!(
        cast_action(&game, green).is_none(),
        "only the next such spell"
    );
}

#[test]
fn casting_a_matching_spell_normally_uses_up_the_permission() {
    let (mut game, [red, green, _]) = after_chapter_one();
    game.player_mut(PlayerId::from_index(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 4);
    // The mana pays for the red creature; the grant is still available too,
    // so pick the paid method explicitly.
    let paid = compute_legal_actions(&game, PlayerId::from_index(0))
        .into_iter()
        .find(|a| {
            matches!(a, LegalAction::CastSpell { spell_id, casting_method, .. }
                if *spell_id == red && matches!(casting_method, ironsmith::alternative_cast::CastingMethod::Normal))
        })
        .expect("normal cast");
    cast(&mut game, paid);
    assert!(
        cast_action(&game, green).is_none(),
        "the next matching spell was already cast"
    );
}

#[test]
fn the_permission_ends_with_the_turn() {
    let (mut game, [red, _, _]) = after_chapter_one();
    game.turn.turn_number = 4;
    assert!(cast_action(&game, red).is_none());
}

#[test]
fn chapter_three_doubles_power_and_toughness_and_grants_trample() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    let saga = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
    let bear = game.create_object_from_definition(
        &creature("Brute", ManaSymbol::Green),
        alice,
        Zone::Battlefield,
    );
    resolve_chapter(&mut game, saga, 2, vec![ResolvedTarget::Object(bear)]);
    assert_eq!(game.calculated_power(bear), Some(8));
    assert_eq!(game.calculated_toughness(bear), Some(8));
    assert!(game.object_has_static_ability_id(bear, StaticAbilityId::Trample));
}
