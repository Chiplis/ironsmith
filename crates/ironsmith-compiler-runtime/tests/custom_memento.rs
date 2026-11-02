//! Selected front-face spell resolution followed by transformed battlefield entry.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith_compiled_artifact::{CompiledCardArtifact, ArtifactCardId, ArtifactCardIdentity, CompiledCardPayload, wire_definition_from_serializable};
const A: PlayerId = PlayerId(0);
const RULES: &str = "You may cast this spell from your graveyard for {1}{G} rather than pay its mana cost. If you cast this spell this way, put it onto the battlefield transformed as it resolves. If this spell would leave the stack other than being put onto the battlefield, exile it instead. If the permanent this spell becomes would leave the battlefield, exile it instead.";
fn pairs() -> [(CardDefinition, CardDefinition); 2] {
    let front_id = ironsmith_compiler::ids::CardId::new();
    let back_id = ironsmith_compiler::ids::CardId::new();
    let front = ironsmith_compiler::CardDefinitionBuilder::new(front_id, "Graveyard price front").other_face(back_id).other_face_name("Graveyard price back").linked_face_layout(ironsmith_compiler::card::LinkedFaceLayout::TransformLike).transforming_dfc(true).with_custom_keyword("Memento");
    let back = ironsmith_compiler::CardDefinitionBuilder::new(back_id, "Graveyard price back").other_face(front_id).other_face_name("Graveyard price front").linked_face_layout(ironsmith_compiler::card::LinkedFaceLayout::TransformLike).transforming_dfc(true);
    fn compile(builder: ironsmith_compiler::CardDefinitionBuilder, text: String, payment: Option<&str>) -> (CompiledCardArtifact, CardDefinition) {
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler::CompilerFacade::new().compile_definition(builder, text.clone(), ironsmith_compiler::CompilePolicy {allow_unsupported: false})
        }));
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let mut compiled = result.unwrap();
        // Match browser/audit ordering: attribute on the compiler definition,
        // before wire serialization or runtime materialization.
        if let Some(keyword) = payment {
            assert!(compiled.definition.attribute_custom_keyword_payment(keyword), "one uniquely attributable generic price is required");
        }
        let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
        let wire = ironsmith_text::with_choice_decision_labels(wire, |effect| ironsmith_runtime_catalog::artifact_materializer::materialize_effect(effect).map_err(|error| error.to_string())).unwrap();
        let direct = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone()).unwrap();
        if let Some(keyword) = payment {assert_eq!(direct.alternative_casts[0].name(), keyword);}
        let artifact = CompiledCardArtifact::new(ArtifactCardIdentity {
            local_id: ArtifactCardId(compiled.definition.card.id.0), name: compiled.definition.card.name.clone(), face_name: None,
            other_face: compiled.definition.card.other_face.map(|id| ArtifactCardId(id.0)),
            linked_face_layout: Some("transform_like".into()),
        }, CompiledCardPayload {definition: wire, canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"), ability_labels: ironsmith_text::ability_surface_texts(&direct)},
        "front-spell-entry-test", text.as_bytes());
        (artifact, direct)
    }
    let (fa, f) = compile(front, format!("Mana cost: {{G}}\nType: Instant\nYou gain 1 life.\n{RULES}"), Some("Memento"));
    // Custom identity intentionally renders the named keyword. Verify the
    // underlying destination rules independently of that display shorthand.
    let instructions = f.alternative_casts[0].battlefield_instructions();
    assert!(instructions.enters_transformed);
    assert!(!instructions.transformed, "entry does not select the back face while casting");
    assert_eq!(instructions.departure_replacements.len(), 2);
    assert!(instructions.departure_replacements.iter().any(|rule|
        rule.from_zone == Zone::Stack && rule.to_zone.is_none()
        && rule.excluded_to_zones == vec![Zone::Battlefield]
        && rule.destination == Zone::Exile));
    assert!(instructions.departure_replacements.iter().any(|rule|
        rule.from_zone == Zone::Battlefield && rule.to_zone.is_none()
        && rule.excluded_to_zones.is_empty() && rule.destination == Zone::Exile));
    let (ba, b) = compile(back, "Type: Artifact Creature — Construct\nPower/Toughness: 2/2\nTrample".into(), None);
    let restored = |a: CompiledCardArtifact| {
        let decoded = CompiledCardArtifact::from_json(&a.to_json().unwrap()).unwrap();
        assert_eq!(decoded, a);
        let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
        if !restored.alternative_casts.is_empty() {assert_eq!(restored.alternative_casts[0].name(), "Memento");}
        restored
    };
    [(f, b), (restored(fa), restored(ba))]
}
fn announce(game: &mut GameState, action: LegalAction) {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
}
fn setup(front: &CardDefinition, back: &CardDefinition, zone: Zone) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.register_linked_face_definition(front); game.register_linked_face_definition(back);
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain; game.turn.step = None;
    let source = game.create_object_from_definition(front, A, zone);
    (game, source)
}
fn move_effect(game: &mut GameState, source: ObjectId, destination: Zone) -> ObjectId {
    let stable = game.object(source).unwrap().stable_id;
    execute_effect(game, &Effect::new(ironsmith::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(source), destination, false)), &mut EffectContext::new_default(source, A)).unwrap();
    game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn original_front_spell_resolves_then_enters_transformed_with_scoped_departure_redirects() {
    for (front, back) in pairs() {
        // A normally created back-face object has no selected-method rider.
        let (mut ordinary, _) = setup(&front, &back, Zone::Hand);
        let normal_back = ordinary.create_object_from_definition(&back, A, Zone::Battlefield);
        let hand = move_effect(&mut ordinary, normal_back, Zone::Hand);
        assert_eq!(ordinary.object(hand).unwrap().zone, Zone::Hand);
        // The front's ordinary cast and destination remain untouched.
        let (mut normal, source) = setup(&front, &back, Zone::Hand);
        normal.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 1);
        let action = compute_legal_actions(&normal, A).unwrap().into_iter().find(|a| matches!(a, LegalAction::CastSpell {spell_id, ..} if *spell_id == source)).unwrap();
        announce(&mut normal, action);
        let stack = normal.stack[0].object_id;
        let stable = normal.object(stack).unwrap().stable_id;
        ironsmith::game_loop::resolve_stack_entry_with(&mut normal, &mut SelectFirstDecisionMaker).unwrap();
        let finished = normal.find_object_by_stable_id(stable).unwrap();
        assert_eq!(normal.object(finished).unwrap().zone, Zone::Graveyard);
        assert_eq!(normal.player(A).unwrap().life, 21);
        let (mut game, source) = setup(&front, &back, Zone::Graveyard);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 1);
        let offered = |game: &GameState| compute_legal_actions(game, A).unwrap().into_iter().find(|a| matches!(a, LegalAction::CastSpell {spell_id, ..} if *spell_id == source));
        assert!(offered(&game).is_none(), "alternative price includes its generic mana");
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
        let action = offered(&game).expect("selected graveyard price permits casting the original front face");
        announce(&mut game, action);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        let stack = game.stack[0].object_id;
        assert_eq!(game.object(stack).unwrap().cast_alternative_method.as_deref().map(|method| method.name()), Some("Memento"), "selected-method receipt retains custom name");
        assert!(game.object(stack).unwrap().card_types.contains(&ironsmith::types::CardType::Instant));
        assert!(!game.object(stack).unwrap().card_types.contains(&ironsmith::types::CardType::Artifact));
        for destination in [Zone::Hand, Zone::Graveyard, Zone::Library, Zone::Exile] {
            let mut interrupted = game.clone();
            let arrival = move_effect(&mut interrupted, stack, destination);
            assert_eq!(interrupted.object(arrival).unwrap().zone, Zone::Exile, "stack departure {destination:?}");
        }
        let stable = game.object(stack).unwrap().stable_id;
        let mut countered = game.clone();
        execute_effect(&mut countered, &Effect::new(ironsmith::effects::CounterEffect::new(ChooseSpec::SpecificObject(stack))), &mut EffectContext::new_default(stack, A)).unwrap();
        let exiled = countered.find_object_by_stable_id(stable).unwrap();
        assert_eq!(countered.object(exiled).unwrap().zone, Zone::Exile, "counter replaces the stack departure");
        assert_eq!(countered.player(A).unwrap().life, 20, "countered front instructions do not run");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield, "successful resolution must not be exiled");
        assert_eq!(game.player(A).unwrap().life, 21, "the original front spell instructions resolved before back-face entry");
        assert!(game.object(permanent).unwrap().card_types.contains(&ironsmith::types::CardType::Artifact));
        assert!(!game.object(permanent).unwrap().card_types.contains(&ironsmith::types::CardType::Instant));
        let mut destroyed = game.clone();
        execute_effect(&mut destroyed, &Effect::new(ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(permanent))), &mut EffectContext::new_default(permanent, A)).unwrap();
        let exiled = destroyed.find_object_by_stable_id(stable).unwrap();
        assert_eq!(destroyed.object(exiled).unwrap().zone, Zone::Exile, "destroy uses the same battlefield replacement");
        for destination in [Zone::Hand, Zone::Graveyard, Zone::Library, Zone::Exile] {
            let mut departed = game.clone();
            let arrival = move_effect(&mut departed, permanent, destination);
            assert_eq!(departed.object(arrival).unwrap().zone, Zone::Exile, "permanent departure {destination:?}");
            let hand = move_effect(&mut departed, arrival, Zone::Hand);
            let grave = move_effect(&mut departed, hand, Zone::Graveyard);
            assert_eq!(departed.object(grave).unwrap().zone, Zone::Graveyard, "new incarnation must not inherit replacement");
        }
    }
}
#[test]
fn unknown_selected_replacement_tail_is_not_silently_discarded() {
    for rules in [
        RULES.replace("exile it instead.", "exile it instead {2}."),
        format!("{RULES} Also remember seven impossible things."),
        "You may cast this card from your graveyard transformed for a banana. If this spell would leave the stack other than being put onto the battlefield, exile it instead.".into(),
        format!("{RULES} If the permanent this spell becomes would leave the battlefield, draw seven cards instead."),
    ] {
        let text = format!("Type: Instant\nYou gain 1 life.\n{rules}");
        let result = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_runtime_definition("Unknown rider", &text, false));
        assert!(result.is_err(), "unsupported combined program compiled: {rules}");
    }
}
