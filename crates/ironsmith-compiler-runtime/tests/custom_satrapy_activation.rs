//! MSEM 205.3m vocabulary composed with existing tap-cost and token actions.
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, LegalAction, compute_legal_actions};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions() -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler_runtime::compile_to_artifact("Satrapy General",
                "Mana cost: {2}{W}\nType: Creature — Angel Warrior\nPower/Toughness: 3/3\nFlying\nTap three untapped Satrapies you control: Create a 0/0 white Soldier Army creature token. Then put a +1/+1 counter on each Army you control. Activate only as a sorcery.", false)
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
struct ChooseLegal;
impl DecisionMaker for ChooseLegal {
    fn decide_objects(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        ctx.candidates.iter().filter(|o| o.legal).map(|o| o.id).take(ctx.min).collect()
    }
}
fn legal(game: &GameState, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source))
}
#[test]
fn satrapy_tap_cost_selects_exactly_three_eligible_lands_and_respects_timing() {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    for definition in definitions() {
        let mut casting = GameState::new(vec!["A".into(), "B".into()], 20);
        casting.turn.active_player = A;
        casting.turn.priority_player = Some(A);
        casting.turn.phase = ironsmith::game_state::Phase::FirstMain;
        casting.turn.step = None;
        let card = casting.create_object_from_definition(&definition, A, Zone::Hand);
        let castable = |game: &GameState| compute_legal_actions(game, A).unwrap().iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card));
        assert!(!castable(&casting));
        casting.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::White, 1);
        casting.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
        assert!(castable(&casting), "the original card's casting cost is preserved separately from its activation cost");
        let land = ironsmith_compiler_runtime::compile_to_runtime_definition("Satrapy resource", "Type: Land — Satrapy", false).unwrap();
        let other = ironsmith_compiler_runtime::compile_to_runtime_definition("Other resource", "Type: Land — Gate", false).unwrap();
        for eligible in [2, 3, 4] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let general = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let candidates: Vec<_> = (0..eligible).map(|_| game.create_object_from_definition(&land, A, Zone::Battlefield)).collect();
            let opponent = game.create_object_from_definition(&land, B, Zone::Battlefield);
            let tapped = game.create_object_from_definition(&land, A, Zone::Battlefield);
            game.tap(tapped);
            let wrong_type = game.create_object_from_definition(&other, A, Zone::Battlefield);
            if eligible < 3 {
                assert!(legal(&game, general).is_none(), "opponent, tapped and non-Satrapy lands cannot supply the third resource");
                continue;
            }
            game.turn.phase = ironsmith::game_state::Phase::Combat;
            assert!(legal(&game, general).is_none(), "activation is sorcery restricted");
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.active_player = B;
            assert!(legal(&game, general).is_none(), "own main phase required");
            game.turn.active_player = A;
            let action = legal(&game, general).expect("three eligible Satrapies pay the cost");
            let mut dm = ChooseLegal;
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut state = PriorityLoopState::new(2);
            let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
            for _ in 0..64 {
                if !state.has_pending_action() { break; }
                let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
                progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
            }
            assert!(!state.has_pending_action());
            assert_eq!(game.stack.len(), 1);
            assert_eq!(candidates.iter().filter(|id| game.is_tapped(**id)).count(), 3, "all tap costs paid before effects resolve");
            assert!(!game.is_tapped(opponent));
            assert!(!game.is_tapped(wrong_type));
            assert!(game.is_tapped(tapped));
            assert!(!game.is_tapped(general), "the cost taps resources, not the General");
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let armies: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|o| o.subtypes.contains(&ironsmith::types::Subtype::Army)).collect();
            assert_eq!(armies.len(), 1);
            assert_eq!(game.counter_count(armies[0].id, ironsmith::object::CounterType::PlusOnePlusOne), 1);
            assert_eq!(game.calculated_toughness(armies[0].id), Some(1));
        }
    }
}
