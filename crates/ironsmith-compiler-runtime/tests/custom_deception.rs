//! MSEM 702.216, composed from existing return/entry operations without official keyword identity.
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const BODY: &str = "Mana cost: {3}{B}\nType: Creature — Human Rogue\nPower/Toughness: 2/2\n{2}{B}, Return an unblocked attacker you control to its owner's hand: Put this card onto the battlefield from your hand tapped and attacking the same player, planeswalker, or battle as the returned creature. Activate only if this card is in your hand.";
fn definitions() -> [CardDefinition; 2] {
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), "Deception witness").with_custom_keyword("Deception");
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_builder_to_artifact(builder, BODY, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
struct ChooseAttacker(ObjectId);
impl DecisionMaker for ChooseAttacker {
    fn decide_objects(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        if ctx.candidates.iter().any(|o| o.id == self.0 && o.legal) { vec![self.0] }
        else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
    }
}
fn action(game: &GameState, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source))
}
#[test]
fn deception_preserves_hand_cost_return_owner_reveal_and_defender_without_ninjutsu_identity() {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    for definition in definitions() {
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(source).unwrap().stable_id;
        let activated = game.current_abilities(source).unwrap().iter().find_map(|ability| match &ability.kind { ironsmith::ability::AbilityKind::Activated(a) => Some(a.clone()), _ => None }).unwrap();
        assert_eq!(activated.keyword, None);
        assert_ne!(activated.effects.keyword.as_deref(), Some("ninjutsu"));
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Borrowed attacker", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let attacker = game.create_object_from_definition(&body, B, Zone::Battlefield);
        game.set_current_controller(attacker, A).unwrap();
        let returned_stable = game.object(attacker).unwrap().stable_id;
        game.combat = Some(CombatState { block_declaration_complete: false, attackers: vec![AttackerInfo { creature: attacker, target: AttackTarget::Player(C) }], ..Default::default() });
        game.mark_creature_attacked_this_turn(attacker);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
        assert!(action(&game, source).is_none(), "colored mana cost is required");
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
        assert!(action(&game, source).is_none(), "cannot activate before blockers are declared");
        game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
        game.combat.as_mut().unwrap().block_declaration_complete = true;
        let selected = action(&game, source).expect("hand-only ability works after blockers");
        let mut dm = ChooseAttacker(attacker);
        let mut state = PriorityLoopState::new(3);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(selected), &mut dm).unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
        }
        assert!(!state.has_pending_action());
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        let returned = game.find_object_by_stable_id(returned_stable).unwrap();
        assert!(game.player(B).unwrap().hand.contains(&returned));
        assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
        assert!(game.is_revealed_by_pending_ninjutsu(source), "operation receipt keeps card revealed through stack lifetime");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let entered = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        assert!(game.is_tapped(entered));
        let context = ironsmith::effects::EffectContext::new_default(entered, A);
        for (marker, expected) in [("deception", 1), ("ninjutsu", 0)] {
            let mut filter = ironsmith::target::ObjectFilter::creature().with_ability_marker(marker);
            filter.zone = Some(Zone::Battlefield);
            assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &ironsmith::effect::Value::Count(filter), &context).unwrap(), expected,
                "custom Deception identity must not imply official Ninjutsu identity");
        }
        assert!(game.combat.as_ref().unwrap().attackers.iter().any(|a| a.creature == entered && a.target == AttackTarget::Player(C)));
        assert!(!game.is_revealed_by_pending_ninjutsu(source));
        assert!(action(&game, entered).is_none(), "ability cannot activate from battlefield");
    }
}
