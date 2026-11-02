//! Graveyard activation composes conditional token creation and attached transformed Aura entry.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith_compiled_artifact::{CompiledCardArtifact, ArtifactCardId, ArtifactCardIdentity, CompiledCardPayload, wire_definition_from_serializable};
const A: PlayerId = PlayerId(0);
const RULES: &str = "{1}{G}: If you don't control a Totem creature, create a 0/0 colorless Totem creature token. Return this card from your graveyard to the battlefield transformed. Activate only as a sorcery.";
fn pairs() -> [(CardDefinition, CardDefinition); 2] {
    let vocabulary = [ironsmith_core::custom_subtypes::CustomSubtypeDefinition { singular: "Totem".into(), plural: "Totems".into(), family: ironsmith_core::SubtypeFamily::Creature }];
    ironsmith_compiler::with_custom_subtypes(&vocabulary, || pairs_in_scope()).unwrap()
}
fn pairs_in_scope() -> [(CardDefinition, CardDefinition); 2] {
    let front_id = ironsmith_compiler::ids::CardId::new();
    let back_id = ironsmith_compiler::ids::CardId::new();
    let front = ironsmith_compiler::CardDefinitionBuilder::new(front_id, "Graveyard price front").other_face(back_id).other_face_name("Graveyard price back").linked_face_layout(ironsmith_compiler::card::LinkedFaceLayout::TransformLike).transforming_dfc(true).with_custom_keyword("Invoke");
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
        "selected-cast-payment-test", text.as_bytes());
        (artifact, direct)
    }
    let (fa, f) = compile(front, format!("Mana cost: {{G}}\nType: Instant\nYou gain 1 life.\n{RULES}"), None);
    // Custom identity intentionally renders the named keyword. Verify the
    // underlying destination rules independently of that display shorthand.
    let (ba, b) = compile(back, "Type: Enchantment — Aura\nEnchant Totem\nEnchanted creature gets +1/+2.\nIf this permanent would leave the battlefield, exile it instead.".into(), None);
    let restored = |a: CompiledCardArtifact| {
        let decoded = CompiledCardArtifact::from_json(&a.to_json().unwrap()).unwrap();
        assert_eq!(decoded, a);
        let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
        if !restored.alternative_casts.is_empty() {assert_eq!(restored.alternative_casts[0].name(), "Invoke");}
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
fn graveyard_activation_creates_totem_before_transformed_aura_entry() {
    for (front, back) in pairs() {
        let (mut game, source) = setup(&front, &back, Zone::Graveyard);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 1);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
        let ability = front.abilities.iter().find(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))).unwrap();
        assert_eq!(ability.functional_zones, vec![Zone::Graveyard]);
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action, LegalAction::ActivateAbility {source: id, ..} if *id == source)));
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility {source: id, ..} if *id == source)).expect("graveyard-only sorcery activation is legal");
        let stable = game.object(source).unwrap().stable_id;
        announce(&mut game, action);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        let aura = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(aura).unwrap().zone, Zone::Battlefield);
        let Some(ironsmith::object::AttachmentTarget::Object(token)) = game.object(aura).unwrap().attached_to else { panic!("new Totem exists before Aura attachment choice"); };
        assert_eq!(game.calculated_power(token), Some(1));
        assert_eq!(game.calculated_toughness(token), Some(2));
        let departed = move_effect(&mut game, aura, Zone::Hand);
        assert_eq!(game.object(departed).unwrap().zone, Zone::Exile);
    }
}
