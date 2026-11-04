//! Generic selected sacrifice prices compose origin, payment and discount riders.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
const A: PlayerId = PlayerId(0);
fn definitions(name: &str, text: &str, payment: Option<&str>) -> [CardDefinition; 2] {
    let mut builder =
        ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), name);
    if let Some(label) = payment {
        builder = builder.with_custom_keyword(label);
    }
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler::CompilerFacade::new().compile_definition(
                builder,
                text.to_string(),
                ironsmith_compiler::CompilePolicy {
                    allow_unsupported: false,
                },
            )
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let mut compiled = result.unwrap();
    if let Some(label) = payment {
        assert!(compiled.definition.attribute_custom_keyword_payment(label));
    }
    let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
    let direct =
        ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone())
            .unwrap();
    let artifact = CompiledCardArtifact::new(
        ArtifactCardIdentity {
            local_id: ArtifactCardId(direct.card.id.0),
            name: direct.card.name.clone(),
            face_name: None,
            other_face: None,
            linked_face_layout: None,
        },
        CompiledCardPayload {
            definition: wire,
            canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"),
            ability_labels: ironsmith_text::ability_surface_texts(&direct),
        },
        "named-action-state",
        text.as_bytes(),
    );
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn act(game: &mut GameState, action: LegalAction) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            return;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    panic!("unfinished action");
}
fn settle(game: &mut GameState) {
    let mut dm = SelectFirstDecisionMaker;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for _ in 0..32 {
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut dm)
            .unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    panic!("unsettled");
}
#[test]
fn artifact_price_discount_uses_its_own_selected_sacrifice_not_an_unrelated_additional_cost() {
    let text = "Mana cost: {W}{B}\nType: Artifact Creature — Drone\nPower/Toughness: 3/2\nAs an additional cost to cast this spell, sacrifice a creature.\nYou may cast this spell from your hand for {3}{W}{B} and sacrifice an artifact rather than pay its mana cost. This cost is reduced by an amount of generic mana equal to the sacrificed permanent's mana value.";
    for definition in definitions("Separate sacrifice prices", text, Some("refine")) {
        let mut game = game();
        let artifact_definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Selected artifact",
            "Mana cost: {2}\nType: Artifact",
            false,
        )
        .unwrap();
        let artifact =
            game.create_object_from_definition(&artifact_definition, A, Zone::Battlefield);
        let creature_definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Other mandatory sacrifice",
            "Mana cost: {8}\nType: Creature — Bear\nPower/Toughness: 1/1",
            false,
        )
        .unwrap();
        let creature =
            game.create_object_from_definition(&creature_definition, A, Zone::Battlefield);
        for symbol in [ManaSymbol::Colorless, ManaSymbol::White, ManaSymbol::Black] {
            game.player_mut(A).unwrap().mana_pool.add(symbol, 1);
        }
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(source).unwrap().stable_id;
        let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method:CastingMethod::Alternative(_),..}if *spell_id==source)).unwrap();
        act(&mut game, action);
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().mana_pool.total(),
            0,
            "exactly one generic plus both colored symbols remains after selected MV2 discount"
        );
        assert!(game.object(artifact).is_none());
        assert!(game.object(creature).is_none());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 2);
        let source = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
    }
}
#[test]
fn graveyard_discounted_price_exiles_all_stack_departures_but_normal_cast_does_not() {
    let text = "Mana cost: {G}\nType: Sorcery\nYou gain 2 life.\nYou may cast this spell from your graveyard for {4}{G} and sacrifice a permanent rather than pay its mana cost. This cost is reduced by an amount of generic mana equal to the sacrificed permanent's mana value. If this spell would leave the stack, exile it instead.";
    for definition in definitions("Graveyard sacrifice price", text, Some("reconstruct")) {
        for countered in [false, true] {
            let mut game = game();
            let resource_definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Selected permanent",
                "Mana cost: {3}\nType: Artifact",
                false,
            )
            .unwrap();
            game.create_object_from_definition(&resource_definition, A, Zone::Battlefield);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 1);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Green, 1);
            let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
            let stable = game.object(source).unwrap().stable_id;
            let action = compute_legal_actions(&game, A)
                .unwrap()
                .into_iter()
                .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source))
                .unwrap();
            act(&mut game, action);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            let stack = game.stack[0].object_id;
            if countered {
                execute_effect(
                    &mut game,
                    &Effect::new(ironsmith::effects::CounterEffect::new(
                        ChooseSpec::SpecificObject(stack),
                    )),
                    &mut EffectContext::new_default(stack, A),
                )
                .unwrap();
            }
            settle(&mut game);
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(current).unwrap().zone, Zone::Exile);
            assert_eq!(
                game.player(A).unwrap().life,
                if countered { 20 } else { 22 }
            );
            let ordinary = game.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = game.object(ordinary).unwrap().stable_id;
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Green, 1);
            let action = compute_legal_actions(&game, A)
                .unwrap()
                .into_iter()
                .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==ordinary))
                .unwrap();
            act(&mut game, action);
            settle(&mut game);
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(current).unwrap().zone, Zone::Graveyard);
        }
    }
}
#[test]
fn sacrifice_discount_rider_rejects_missing_or_ambiguous_sacrifice_payment() {
    for cost in [
        "{4}",
        "{4} and sacrifice two creatures",
        "{4} and sacrifice a creature and sacrifice an artifact",
    ] {
        let text = format!(
            "Type: Artifact\nYou may cast this spell from your hand for {cost} rather than pay its mana cost. This cost is reduced by an amount of generic mana equal to the sacrificed permanent's mana value."
        );
        assert!(
            ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Invalid payment link",
                &text,
                false
            )
            .is_err()
        );
    }
}
