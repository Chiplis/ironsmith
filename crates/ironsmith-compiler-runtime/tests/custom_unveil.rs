//! Complete linked source faces for the existing transform operation in Unveil.
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::{CardDefinition, CardId, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn action(game: &GameState, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source))
}
fn linked(game: &mut GameState, artifact_route: bool) -> CardDefinition {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-late-abilities.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "msem-772")
        .unwrap();
    let faces = row["representative_cards"][0]["executable_faces"]
        .as_array()
        .unwrap();
    assert_eq!(faces.len(), 2);
    let ids = [CardId::new(), CardId::new()];
    let mut definitions = Vec::new();
    for i in 0..2 {
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            ids[i],
            faces[i]["name"].as_str().unwrap(),
        )
        .other_face(ids[1 - i])
        .other_face_name(faces[1 - i]["name"].as_str().unwrap())
        .linked_face_layout(ironsmith::card::LinkedFaceLayout::TransformLike)
        .transforming_dfc(true)
        .with_custom_keyword("unveil");
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
            ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
                ironsmith_compiler_runtime::compile_builder_to_artifact(
                    builder,
                    faces[i]["executable_text"].as_str().unwrap(),
                    false,
                )
            })
        });
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let (artifact, direct) = result.unwrap();
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        assert_eq!(artifact, restored);
        let definition = if artifact_route {
            ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored)
                .unwrap()
        } else {
            direct
        };
        game.register_linked_face_definition(&definition);
        definitions.push(definition);
    }
    definitions.remove(0)
}
#[test]
fn unveil_source_faces_transform_after_full_payment_with_sorcery_and_zone_restrictions() {
    for route in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let definition = linked(&mut game, route);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let watcher=ironsmith_compiler_runtime::compile_to_runtime_definition("Unveil observer", "Type: Enchantment\nWhenever a permanent you control transforms caused by a unveil activated ability, you gain 5 life.", false).unwrap();
        game.create_object_from_definition(&watcher, A, Zone::Battlefield);
        let ordinary = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        ironsmith::effects::execute_effect(
            &mut game,
            &ironsmith::Effect::new(ironsmith::effects::TransformEffect::new(
                ironsmith::target::ChooseSpec::SpecificObject(ordinary),
            )),
            &mut ironsmith::effects::EffectContext::new_default(ordinary, A),
        )
        .unwrap();
        let mut unrelated_queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(
            &mut game,
            &mut unrelated_queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut game,
            &mut unrelated_queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert!(
            game.stack.is_empty(),
            "an ordinary transform is not an unveil action"
        );

        assert!(action(&game, source).is_none(), "mana required");
        for (symbol, amount) in [
            (ironsmith::mana::ManaSymbol::Colorless, 3),
            (ironsmith::mana::ManaSymbol::Green, 1),
            (ironsmith::mana::ManaSymbol::White, 1),
        ] {
            game.player_mut(A).unwrap().mana_pool.add(symbol, amount);
        }
        let graveyard = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        assert!(action(&game, graveyard).is_none());
        game.turn.phase = ironsmith::Phase::Combat;
        assert!(action(&game, source).is_none());
        game.turn.phase = ironsmith::Phase::FirstMain;
        let selected = action(&game, source).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut state = PriorityLoopState::new(2);
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(selected),
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
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(
            game.current_power(source),
            Some(3),
            "transform waits for resolution"
        );
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            game.object(source).unwrap().name.as_ref(),
            "Venerated Beast"
        );
        assert_eq!(game.current_power(source), Some(6));
        assert_eq!(game.current_toughness(source), Some(6));
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm)
            .unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
            .unwrap();
        assert_eq!(
            game.stack.len(),
            1,
            "named transform completion reaches an independent watcher"
        );
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().life, 25);

        assert!(
            game.calculated_subtypes(source)
                .contains(&ironsmith::types::Subtype::Angel)
        );
        assert!(
            action(&game, source).is_none(),
            "back face does not inherit activation"
        );
    }
}
