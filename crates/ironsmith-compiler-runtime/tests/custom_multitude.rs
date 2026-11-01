//! Unregistered candidate: generic alternative total price + selected-price cast trigger.
//! Parent runs validation. No official keyword alias or additional payment is used.
use ironsmith::{CardDefinition, Effect, GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith_compiled_artifact::{ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload, wire_definition_from_serializable};
const A: PlayerId = PlayerId(0);
const RULES: &str = "You may pay {2}{U} rather than pay this spell's mana cost.\nWhen you cast this spell, if its Multitude cost was paid, copy this spell. You may choose new targets for the copy.";
fn definitions(permanent: bool) -> Vec<CardDefinition> {
    let body = if permanent {"Type: Creature — Wizard\nPower/Toughness: 2/3"} else {"Type: Instant\nYou gain 1 life."};
    definitions_for_body(body)
}
fn definitions_for_body(body: &str) -> Vec<CardDefinition> {
    let mut definitions = Vec::new();
    // Match the actual materializer's one-paragraph output as well as two
    // separately authored ability lines; never repair one into the other.
    for rules in [RULES.to_string(), RULES.replace('\n', " ")] {
        let text = format!("Mana cost: {{U}}\n{body}\n{rules}");
        definitions.extend(compile_named("Selected price copier", &text));
    }
    definitions
}
fn compile_named(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler::CompilerFacade::new().compile_definition(ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), name), text.to_string(), ironsmith_compiler::CompilePolicy {allow_unsupported: false})));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let mut compiled = result.unwrap();
    assert!(compiled.definition.attribute_custom_keyword_payment("Multitude"));
    let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
    let direct = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone()).unwrap();
    assert!(direct.optional_costs.is_empty(), "the total alternative price must not become an additional cost");
    assert_eq!(direct.alternative_casts.len(), 1);
    assert!(direct.alternative_casts[0].keyword().is_none());
    let artifact = CompiledCardArtifact::new(ArtifactCardIdentity {local_id: ArtifactCardId(direct.card.id.0), name: direct.card.name.clone(), face_name: None, other_face: None, linked_face_layout: None}, CompiledCardPayload {definition: wire, canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"), ability_labels: ironsmith_text::ability_surface_texts(&direct)}, "selected-price-copy-test", text.as_bytes());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None; game.turn.active_player = A; game.turn.priority_player = Some(A);
    game
}
fn cast(game: &mut GameState, definition: &CardDefinition, alternative: bool) -> ObjectId {
    cast_with_dm(game, definition, alternative, &mut SelectFirstDecisionMaker)
}
fn cast_with_dm(game: &mut GameState, definition: &CardDefinition, alternative: bool, dm: &mut impl ironsmith::decision::DecisionMaker) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell {spell_id, casting_method, ..} if *spell_id == source && matches!(casting_method, CastingMethod::Alternative(_)) == alternative)).expect("requested price available");
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, dm).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
            return game.stack.first().unwrap().object_id;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("cast unfinished");
}
#[test]
fn selected_total_price_only_creates_a_cast_copy_trigger() {
    for definition in definitions(false) {for alternative in [false, true] {
        let mut game = setup();
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        cast(&mut game, &definition, alternative);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), if alternative {0} else {2}, "alternative price is total3, not printed1 plus3 or printed1 subtracted from3");
        assert_eq!(game.stack.len(), if alternative {2} else {1});
        if alternative {
            assert!(game.stack[0].optional_costs_paid.paid_custom_keyword("Multitude"));
            resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.stack.len(), 2, "copy is a spell, not another cast trigger");
        }
        while !game.stack.is_empty() {resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();}
        assert_eq!(game.player(A).unwrap().life, if alternative {22} else {21});
    }}
}
#[test]
fn permanent_copy_resolves_as_token_and_cast_trigger_survives_original_counter() {
    for definition in definitions(true) {for counter_original in [false, true] {
        let mut game = setup();
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        let original = cast(&mut game, &definition, true);
        assert_eq!(game.stack.len(), 2);
        if counter_original {
            execute_effect(&mut game, &Effect::new(ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(original))), &mut EffectContext::new_default(original, A)).unwrap();
            assert_eq!(game.stack.len(), 1, "cast trigger remains after original is countered");
        }
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.stack.len(), if counter_original {1} else {2});
        let copy = game.stack.last().unwrap().object_id;
        let stable = game.object(copy).unwrap().stable_id;
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        let token = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(token).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(token).unwrap().kind, ironsmith::object::ObjectKind::Token);
        assert_eq!(game.object(token).unwrap().power(), Some(2));
        assert_eq!(game.object(token).unwrap().toughness(), Some(3));
    }}
}


struct TargetChoices {
    original: ObjectId,
    replacement: ObjectId,
    illegal: ObjectId,
    change: bool,
    target_prompts: usize,
    retarget_prompts: usize,
}
impl ironsmith::decision::DecisionMaker for TargetChoices {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
        // Verify the single optional decision by its resulting targets below;
        // decision-label decoration is independent of runtime behavior.
        self.retarget_prompts += 1;
        self.change
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
        use ironsmith::game_state::Target;
        self.target_prompts += 1;
        let chosen = if self.target_prompts == 1 {self.original} else {self.replacement};
        for requirement in &ctx.requirements {
            assert!(requirement.legal_targets.contains(&Target::Object(chosen)));
            assert!(!requirement.legal_targets.contains(&Target::Object(self.illegal)), "noncreature is never a legal new creature target");
        }
        vec![Target::Object(chosen)]
    }
}
#[test]
fn optional_new_targets_are_legal_and_copy_and_original_can_be_countered_independently() {
    use ironsmith::game_state::Target;
    use ironsmith::object::CounterType;
    for definition in definitions_for_body("Type: Instant\nPut a +1/+1 counter on target creature.") {
        for alternative in [false, true] {for change in [false, true] {for counter_copy in [false, true] {
            let mut game = setup();
            let creature = ironsmith_compiler_runtime::compile_to_runtime_definition("Recipient", "Type: Creature — Bear\nPower/Toughness: 2/2", false).unwrap();
            let land = ironsmith_compiler_runtime::compile_to_runtime_definition("Not a creature", "Type: Land", false).unwrap();
            let first = game.create_object_from_definition(&creature, A, Zone::Battlefield);
            let second = game.create_object_from_definition(&creature, PlayerId(1), Zone::Battlefield);
            let illegal = game.create_object_from_definition(&land, A, Zone::Battlefield);
            let mut choices = TargetChoices {original: first, replacement: second, illegal, change, target_prompts: 0, retarget_prompts: 0};
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
            let original = cast_with_dm(&mut game, &definition, alternative, &mut choices);
            assert_eq!(game.stack[0].targets, vec![Target::Object(first)]);
            assert_eq!(choices.target_prompts, 1);
            if alternative {
                resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                assert_eq!(game.stack.len(), 2, "the copy is available for responses before either spell resolves");
                assert_eq!(choices.retarget_prompts, 1);
                assert_eq!(choices.target_prompts, if change {2} else {1});
                let copy = game.stack.last().unwrap().object_id;
                assert_ne!(copy, original);
                assert_eq!(game.stack.last().unwrap().targets, vec![Target::Object(if change {second} else {first})]);
                assert_eq!(game.stack[0].targets, vec![Target::Object(first)], "retargeting a copy never changes original targets");
                let interrupted = if counter_copy {copy} else {original};
                execute_effect(&mut game, &Effect::new(ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(interrupted))), &mut EffectContext::new_default(original, A)).unwrap();
                assert_eq!(game.stack.len(), 1, "countering either spell leaves the other");
            } else {
                assert_eq!(game.stack.len(), 1, "ordinary-price cast never gets the copy trigger");
                assert_eq!(choices.retarget_prompts, 0);
            }
            while !game.stack.is_empty() {resolve_stack_entry_with(&mut game, &mut choices).unwrap();}
            let second_gets_counter = alternative && !counter_copy && change;
            assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), u32::from(!second_gets_counter));
            assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), u32::from(second_gets_counter));
        }}}
    }
}


#[test]
fn complete_materialized_source_card_uses_total_price_and_resolves_original_and_token() {
    // Consume evidence produced by compileDraftInput, preserving the original
    // same-paragraph boundary and every other source-card ability verbatim.
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/multitude.json")).unwrap();
    let row = fixture["mechanics"].as_array().unwrap().iter().find(|row| row["id"] == "msem-929").unwrap();
    let card = &row["representative_cards"][0];
    let name = card["name"].as_str().unwrap();
    let text = card["executable_text"].as_str().unwrap();
    assert!(text.contains("mana cost. When you cast this spell"), "source case must keep frontend's actual same-paragraph dispatch");
    assert!(text.contains("Whenever Lichbane Forerunner attacks"), "no unrelated operative source text may be removed");
    for definition in compile_named(name, text) {for alternative in [false, true] {
        let mut game = setup();
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White, 2);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        cast(&mut game, &definition, alternative);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), if alternative {0} else {2});
        assert_eq!(game.stack.len(), if alternative {2} else {1});
        while !game.stack.is_empty() {resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();}
        let resolved: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object| object.name == name).collect();
        assert_eq!(resolved.len(), if alternative {2} else {1});
        assert_eq!(resolved.iter().filter(|object| object.kind == ironsmith::object::ObjectKind::Token).count(), usize::from(alternative));
        for object in resolved {assert_eq!(object.power(), Some(3)); assert_eq!(object.toughness(), Some(1));}
    }}
}
