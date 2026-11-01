//! Conditional prices use life-loss history, never damage or current life.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, Effect, GameProgress, GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const PRICE: &str = "Mana cost: {3}{U}\nType: Instant\nYou may pay {U} rather than pay this spell's mana cost if an opponent lost life this turn.\nYou gain 1 life.";
fn definition() -> CardDefinition {
    let mut definition = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || compile_to_runtime_definition("Life-history price", PRICE, false)).unwrap();
    assert_eq!(definition.alternative_casts.len(), 1);
    assert!(definition.alternative_casts[0].keyword().is_none());
    assert_ne!(definition.alternative_casts[0].name(), "Spectacle");
    assert!(definition.attribute_custom_keyword_payment("bleed"));
    assert_eq!(definition.alternative_casts[0].name(), "bleed");
    definition
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}
fn loss(game: &mut GameState, player: PlayerId, payment: bool, regain: bool) {
    let helper = compile_to_runtime_definition("Life operation source", "Type: Sorcery", false).unwrap();
    let source = game.create_object_from_definition(&helper, player, Zone::Stack);
    let mut context = EffectContext::new_default(source, player);
    let effect = if payment { Effect::new(ironsmith::effects::PayLifeEffect::you(2)) } else { Effect::new(ironsmith::effects::LoseLifeEffect::you(2)) };
    execute_effect(game, &effect, &mut context).unwrap();
    if regain { execute_effect(game, &Effect::gain_life(2), &mut context).unwrap(); }
}
fn cast(game: &mut GameState, definition: &CardDefinition, alternative: bool) {
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == spell && matches!(casting_method, CastingMethod::Alternative(_)) == alternative)).expect("requested price is available");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
            return;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    panic!("cast did not finish");
}
#[test]
fn opponent_life_loss_includes_payment_and_survives_life_regain() {
    let definition = definition();
    for payment in [false, true] {
        for regain in [false, true] {
            let mut game = game();
            loss(&mut game, B, payment, regain);
            assert_eq!(game.player(B).unwrap().life, if regain { 20 } else { 18 });
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
            cast(&mut game, &definition, true);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert!(game.stack[0].optional_costs_paid.paid_custom_keyword("bleed"));
            assert!(!game.stack[0].optional_costs_paid.was_paid_label("Spectacle"));
            resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.player(A).unwrap().life, 21);
        }
    }
}
#[test]
fn own_life_loss_does_not_enable_price_and_normal_price_remains_available() {
    let definition = definition();
    for own_loss in [false, true] {
        let mut game = game();
        if own_loss { loss(&mut game, A, true, false); }
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)), "only an opponent's history enables the alternative; full price is not funded");
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
        cast(&mut game, &definition, false);
        assert!(!game.stack[0].optional_costs_paid.paid_custom_keyword("bleed"));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}
