//! Actual MSEM source918, using generic aggregate selection and validated tap.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions() -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-final-abilities.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "msem-918")
        .unwrap();
    let card = &row["representative_cards"][0];
    source_support::definitions(card).unwrap()
}
struct Choices {
    tappers: Vec<ObjectId>,
    target: ObjectId,
    seen: bool,
}
impl DecisionMaker for Choices {
    fn decide_objects(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.seen = true;
        let constraint = ctx
            .aggregate_constraint
            .as_ref()
            .expect("aggregate bound retained");
        assert_eq!(
            constraint.metric,
            ironsmith_core::ChoiceAggregateMetric::Power
        );
        assert!(
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .all(|c| self.tappers.contains(&c.id))
        );
        self.tappers.clone()
    }
    fn decide_targets(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::Target> {
        vec![ironsmith::Target::Object(self.target)]
    }
}
fn body(game: &mut GameState, owner: PlayerId, power: i32) -> ObjectId {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Tap contributor",
        &format!("Type: Creature — Human\nPower/Toughness: {power}/{power}"),
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}
fn cast(game: &GameState, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source))
}
#[test]
fn resurface_requires_printed_mana_and_untapped_controlled_total_power_then_exiles() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        let stable = game.object(source).unwrap().stable_id;
        let two = body(&mut game, A, 2);
        let tapped = body(&mut game, A, 8);
        game.tap(tapped);
        let enemy = body(&mut game, B, 8);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Blue, 2);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 1);
        assert!(
            cast(&game, source).is_none(),
            "tapped and opponents' creatures cannot satisfy aggregate price"
        );
        let one = body(&mut game, A, 1);
        assert!(cast(&game, source).is_some());
        game.player_mut(A).unwrap().mana_pool = Default::default();
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Blue, 2);
        assert!(
            cast(&game, source).is_none(),
            "tap price does not waive printed generic mana"
        );
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 1);
        let action = cast(&game, source).expect("printed price and3totalpower available");
        let mut dm = Choices {
            tappers: vec![two, one],
            target: two,
            seen: false,
        };
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut state = ironsmith::game_loop::PriorityLoopState::new(2);
        use ironsmith::game_loop::{
            PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm,
        };
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{progress:?}")
            };
            progress =
                apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm)
                    .unwrap();
        }
        assert!(!state.has_pending_action());
        assert!(dm.seen);
        assert!(game.is_tapped(two) && game.is_tapped(one));
        assert!(!game.is_tapped(enemy));
        assert_eq!(
            game.player(A).unwrap().mana_pool.total(),
            0,
            "printed mana cost remains payable in addition to taps"
        );
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
            .unwrap();
        let id = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(id).unwrap().zone, Zone::Exile);
        assert_eq!(
            game.battlefield.len(),
            5,
            "the exact source also creates its copy token"
        );
    }
}
