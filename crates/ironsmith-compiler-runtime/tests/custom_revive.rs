//! MSEM702.207 expressed using independent exile-cost and source-copy operations.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn definitions() -> [CardDefinition; 2] {
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), "Revive witness").with_custom_keyword("Revive");
    let text = "Mana cost: {3}{U}\nType: Creature — Bird Wizard\nPower/Toughness: 3/2\nFlying\nWhen this creature enters, you gain 1 life.\n{1}{B}, Discard a card, Exile this card from your graveyard: Create a token that's a copy of this card, except it has no mana cost. Activate only as a sorcery.";
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_builder_to_artifact(builder, text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn action(game: &GameState, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source))
}
struct First;
impl DecisionMaker for First {
    fn decide_objects(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {SelectFirstDecisionMaker.decide_objects(game, ctx)}
}
#[test]
fn revive_pays_complete_cost_and_copies_original_graveyard_source_through_stack() {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    for definition in definitions() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A; game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain; game.turn.step = None;
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        let stable = game.object(source).unwrap().stable_id;
        let hand_source = game.create_object_from_definition(&definition, A, Zone::Hand);
        assert!(action(&game, hand_source).is_none(), "graveyard-only activation");
        assert!(action(&game, source).is_none(), "cannot skip mana payment");
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
        // Use a dedicated discard card; the hand copy remains a separate
        // negative-zone witness until it is moved out of the payment pool.
        game.move_object_by_effect(hand_source, Zone::Exile).unwrap();
        assert!(action(&game, source).is_none(), "discard is required in addition to mana and self-exile");
        let discard_definition = ironsmith_compiler_runtime::compile_to_runtime_definition("Discard payment", "Type: Instant\nYou gain 1 life.", false).unwrap();
        let discard = game.create_object_from_definition(&discard_definition, A, Zone::Hand);
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        assert!(action(&game, source).is_none(), "sorcery timing remains required");
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let selected = action(&game, source).unwrap();
        let mut dm = First;
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(selected), &mut dm).unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() {break;}
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
        }
        assert!(!state.has_pending_action());
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(game.object(discard).is_none());
        assert!(game.player(A).unwrap().graveyard.iter().any(|id| game.object(*id).unwrap().name == "Discard payment"));
        let snapshot = game.stack[0].source_snapshot.as_ref().expect("pre-cost source evidence");
        assert_eq!(snapshot.object_id, source);
        assert_eq!(snapshot.zone, Zone::Graveyard);
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile, "self-exile is a paid activation cost");
        // The physical card can move again before the ability resolves. The
        // created copy remains bound to its original graveyard incarnation.
        execute_effect(&mut game, &Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(exiled), Zone::Hand, false)), &mut EffectContext::new_default(exiled, A)).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.battlefield.len(), 1);
        let token = game.battlefield[0];
        let object = game.object(token).unwrap();
        assert!(object.mana_cost.is_none());
        assert_eq!(game.calculated_power(token), Some(3));
        assert_eq!(game.calculated_toughness(token), Some(2));
        assert!(game.current_has_static_ability_id(token, ironsmith_core::StaticAbilityId::Flying));
        assert!(object.subtypes.contains(&ironsmith::types::Subtype::Bird));
        assert!(!object.subtypes.contains(&ironsmith::types::Subtype::Zombie), "no Embalm substitution");
        let context = EffectContext::new_default(token, A);
        for marker in ["embalm", "eternalize"] {
            let mut filter = ironsmith::target::ObjectFilter::creature().with_ability_marker(marker); filter.zone = Some(Zone::Battlefield);
            assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &ironsmith::effect::Value::Count(filter), &context).unwrap(), 0);
        }
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1, "copied entry trigger remains functional");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().life, 21);
    }
}
