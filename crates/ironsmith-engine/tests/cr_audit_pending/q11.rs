//! CR audit (in-review adjudication, package q11): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::EffectExecutor;
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{GameProgress, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::events::KeywordActionKind;
use ironsmith::game_state::Target;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    Ability, CardId, CardType, ChooseSpec, CounterType, DecisionMaker, LegalAction, ObjectId,
    Phase, PowerToughness, PriorityLoopState, PriorityResponse, StaticAbility, Zone,
};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn compile(name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture compile failed for {name}: {e:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "fixture: {name} must be fully implemented"
    );
    def
}

fn new_game(active: PlayerId) -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = active;
    game.turn.priority_player = Some(active);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn vanilla(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .build()
}

fn goblin() -> CardDefinition {
    compile(
        "_____ Goblin",
        "Mana cost: {2}{R}\nType: Creature — Goblin Guest\nPower/Toughness: 2/2\nWhen this creature enters, you may put a name sticker on it. Add {R} for each unique vowel on that sticker. (The vowels are A, E, I, O, U, and Y.)",
    )
}

fn chicken_troupe() -> CardDefinition {
    compile(
        "Chicken Troupe",
        "Mana cost: {1}{G}\nType: Creature — Bird Performer\nPower/Toughness: 2/2\nWard {2}\nWhen Chicken Troupe enters, you get {TK}, then you may put a sticker on a nonland permanent you own.",
    )
}

/// Accepts every "you may", picks the first option and first legal target.
struct Accept;
impl DecisionMaker for Accept {
    fn decide_boolean(&mut self, _g: &GameState, _c: &BooleanContext) -> bool {
        true
    }
    fn decide_number(&mut self, _g: &GameState, c: &NumberContext) -> u32 {
        c.max
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(g, c)
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_targets(&mut self, _g: &GameState, c: &TargetsContext) -> Vec<Target> {
        c.requirements
            .iter()
            .filter_map(|req| req.legal_targets.first().copied())
            .collect()
    }
}

fn complete_action<D: DecisionMaker>(
    game: &mut GameState,
    action: LegalAction,
    dm: &mut D,
    queue: &mut TriggerQueue,
) -> Result<(), String> {
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..24 {
        if result.is_err()
            || (state.pending_cast.is_none()
                && state.pending_method_selection.is_none()
                && state.pending_activation.is_none())
        {
            break;
        }
        if state.pending_method_selection.is_some() {
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(0),
                dm,
            );
        } else {
            let context = match &result {
                Ok(GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending action has no decision: {other:?}")),
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(
                game, queue, &mut state, &context, dm,
            );
        }
    }
    result.map_err(|error| format!("{error:?}"))?;
    Ok(())
}

fn cast<D: DecisionMaker>(game: &mut GameState, player: PlayerId, spell: ObjectId, dm: &mut D) {
    let action = ironsmith::compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("fixture spell must be castable");
    let mut queue = TriggerQueue::new();
    complete_action(game, action, dm, &mut queue).expect("fixture cast");
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm)
        .expect("stack triggers");
}

fn resolve_all<D: DecisionMaker>(game: &mut GameState, dm: &mut D) {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    for _ in 0..80 {
        if game.stack.is_empty() {
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm)
                .expect("stack triggers");
            if game.stack.is_empty() {
                break;
            }
        }
        match ironsmith::game_loop::apply_priority_response_with_dm(
            game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        )
        .expect("pass priority")
        {
            GameProgress::NeedsDecisionCtx(ctx) => {
                ironsmith::game_loop::apply_decision_context_with_dm(
                    game, &mut queue, &mut state, &ctx, dm,
                )
                .expect("answer decision");
            }
            GameProgress::GameOver(r) => panic!("unexpected game over {r:?}"),
            _ => {}
        }
    }
    assert!(game.stack.is_empty(), "stack did not empty");
}

fn battlefield_named(game: &GameState, name: &str) -> ObjectId {
    game.battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).is_some_and(|o| o.name.as_ref() as &str == name))
        .unwrap_or_else(|| panic!("fixture: {name} on the battlefield"))
}

/// Cast the real `_____ Goblin` and accept its "you may put a name sticker on it".
/// Returns the number of stickers on it afterwards and its current name.
fn cast_goblin(game: &mut GameState, seat: PlayerId) -> (u32, Option<String>) {
    let spell = game.create_object_from_definition(&goblin(), seat, Zone::Hand);
    game.player_mut(seat).unwrap().mana_pool.red = 3;
    let mut dm = Accept;
    cast(game, seat, spell, &mut dm);
    resolve_all(game, &mut dm);
    let goblin = battlefield_named(game, "_____ Goblin");
    (
        game.sticker_count_on_object(goblin, KeywordActionKind::Sticker, None),
        game.current_name(goblin).map(|n| n.to_string()),
    )
}

// ---------------------------------------------------------------------------
// 103.2d / 123.2a / 123.2c: playing sticker cards does not supply sticker
// sheets. The player must declare at least ten unique sheets before pregame
// selects three; only those sheets provide stickers. These engine fixtures
// deliberately omit that setup. Positive random-selection coverage lives in
// wasm_game_impl::sticker_sheet_tests, which exercises the actual setup API.
// ---------------------------------------------------------------------------

#[test]
fn cr_103_2d_123_2a_sticker_card_does_not_create_sheets_without_pregame_selection() {
    for seat in [A, B] {
        // Fixture control: once a name sticker is accessible, the same real
        // card and the same decision flow place it. This proves the wiring;
        // the only difference below is that nobody registered a sticker.
        let mut control = new_game(seat);
        control.add_accessible_name_sticker(seat, "Shiny");
        let (count, name) = cast_goblin(&mut control, seat);
        assert_eq!(count, 1, "fixture: registered name sticker is placed by the real card");
        assert_ne!(name.as_deref(), Some("_____ Goblin"), "fixture: name changed");

        // No sticker-sheet pool was declared or selected. A sticker card in
        // the library cannot manufacture that missing pregame input.
        let mut game = new_game(seat);
        for _ in 0..3 {
            game.create_object_from_definition(&goblin(), seat, Zone::Library);
        }
        let (count, name) = cast_goblin(&mut game, seat);
        assert_eq!(count, 0, "CR 123.2c: without selected sheets there is no name sticker to place");
        assert_eq!(name.as_deref(), Some("_____ Goblin"));
    }
}

#[test]
fn cr_103_2d_123_2a_tickets_do_not_supply_stickers_without_selected_sheets() {
    // Fixture control: with one accessible art sticker the real card places it.
    let mut control = new_game(A);
    control.add_accessible_art_sticker(A);
    let spell = control.create_object_from_definition(&chicken_troupe(), A, Zone::Hand);
    control.player_mut(A).unwrap().mana_pool.green = 2;
    let mut dm = Accept;
    cast(&mut control, A, spell, &mut dm);
    resolve_all(&mut control, &mut dm);
    let troupe = battlefield_named(&control, "Chicken Troupe");
    assert_eq!(
        control.sticker_count_on_object(troupe, KeywordActionKind::Sticker, None),
        1,
        "fixture: registered art sticker is placed by the real card"
    );

    let mut game = new_game(A);
    for _ in 0..3 {
        game.create_object_from_definition(&chicken_troupe(), A, Zone::Library);
    }
    let spell = game.create_object_from_definition(&chicken_troupe(), A, Zone::Hand);
    game.player_mut(A).unwrap().mana_pool.green = 2;
    let mut dm = Accept;
    cast(&mut game, A, spell, &mut dm);
    resolve_all(&mut game, &mut dm);
    let troupe = battlefield_named(&game, "Chicken Troupe");
    let ticket = CounterType::Named("ticket".into());
    let tickets = game.player(A).unwrap().counter_count(ticket);
    let stickers = game.sticker_count_on_object(troupe, KeywordActionKind::Sticker, None);
    assert!(
        tickets <= 1,
        "fixture: the {{TK}} gain resolved at most once (tickets={tickets})"
    );
    assert_eq!(stickers, 0,
        "CR 123.2c/123.3c: tickets pay for an accessible sticker; they do not create a sticker-sheet pool");
}

// ---------------------------------------------------------------------------
// 123.2: a sticker sheet is a predetermined Unfinity combination; every sheet
// carries name, ability, power/toughness and art stickers.
// ---------------------------------------------------------------------------

#[test]
fn cr_123_2_sticker_cards_do_not_fabricate_any_sheet_combination() {
    let mut game = new_game(A);
    for seat in [A, B] {
        for _ in 0..3 {
            game.create_object_from_definition(&goblin(), seat, Zone::Library);
            game.create_object_from_definition(&chicken_troupe(), seat, Zone::Library);
        }
    }
    for seat in [A, B] {
        let kinds = (
            game.available_name_stickers(seat).len(),
            game.available_ability_stickers(seat).len(),
            game.available_pt_stickers(seat).len(),
            game.available_art_stickers(seat).len(),
        );
        assert_eq!(kinds, (0, 0, 0, 0),
            "CR 123.2/123.2a/123.2c: seat {seat:?} declared no sheets; sticker cards cannot invent name/ability/PT/art stickers");
    }
}

// ---------------------------------------------------------------------------
// 123.3d: a sticker already on an object that comes to be on another object
// (the only engine path is 123.5 relocation to a new public-zone object)
// does not have its ticket cost paid again.
// ---------------------------------------------------------------------------

#[test]
fn cr_123_3d_relocated_ticket_sticker_is_not_paid_for_again() {
    use ironsmith::effects::PutStickerEffect;
    let ticket = CounterType::Named("ticket".into());
    for (destinations, balance) in [
        (vec![Zone::Graveyard], 2u32),
        (vec![Zone::Exile], 2),
        (vec![Zone::Graveyard, Zone::Exile], 2),
        (vec![Zone::Graveyard, Zone::Battlefield], 2),
        (vec![Zone::Graveyard], 3),
    ] {
        let mut game = new_game(A);
        let host = game.create_object_from_definition(&vanilla("Sticker host"), A, Zone::Battlefield);
        game.add_player_counters_with_source(A, ticket.clone(), balance, Some(host), Some(A))
            .unwrap();
        let sticker = game.add_accessible_ability_sticker(
            A,
            2,
            vec![Ability::static_ability(StaticAbility::flying())],
            "Flying",
        );
        PutStickerEffect::new(ChooseSpec::SpecificObject(host), KeywordActionKind::AbilitySticker)
            .execute(&mut game, &mut EffectContext::new_default(host, A))
            .unwrap();
        assert_eq!(
            game.sticker_count_on_object(host, KeywordActionKind::Sticker, None),
            1,
            "fixture: ticket sticker placed"
        );
        assert_eq!(
            game.player(A).unwrap().counter_count(ticket.clone()),
            balance - 2,
            "fixture: initial placement paid 2 tickets"
        );
        let mut current = host;
        for destination in &destinations {
            current = game.move_object_by_effect(current, *destination).unwrap();
            assert_ne!(current, host, "fixture: the card became a new object");
            assert_eq!(
                game.sticker_count_on_object(current, KeywordActionKind::Sticker, None),
                1,
                "CR 123.3d/123.5: the sticker is now on the new object ({destinations:?}, balance {balance})"
            );
            assert!(
                game.object_has_ability(current, &StaticAbility::flying()),
                "CR 123.3d/123.5: the relocated sticker keeps applying without a new payment ({destinations:?})"
            );
            assert_eq!(
                game.player(A).unwrap().counter_count(ticket.clone()),
                balance - 2,
                "CR 123.3d: moving an already-placed sticker to another object must not charge its ticket cost again ({destinations:?}, balance {balance})"
            );
            assert!(
                !game
                    .available_ability_stickers(A)
                    .iter()
                    .any(|entry| entry.id == sticker),
                "CR 123.3d: the moved sticker stays placed rather than returning to the sheet"
            );
        }
    }
}
