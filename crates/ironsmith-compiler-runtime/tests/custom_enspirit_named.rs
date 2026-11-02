//! Named Enspirit cast-cost completion: one exile instruction, characteristic reduction,
//! source-linked lifetime and the selected optional-cost receipt.
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const RULES: &str = "As an additional cost to cast this spell, you may exile a nonenchantment creature you control until this spell leaves the stack without entering the battlefield or the permanent it becomes leaves the battlefield. Perform the enspirit action with creatures exiled this way.\nThis spell costs {X} less to cast, where X is the total mana value of creatures exiled this way.";
fn compile(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler::CompilerFacade::new().compile_definition(
                ironsmith_compiler::CardDefinitionBuilder::new(
                    ironsmith_compiler::ids::CardId::new(),
                    "Exile cost source",
                )
                .with_custom_keyword("enspirit"),
                text,
                ironsmith_compiler::CompilePolicy {
                    allow_unsupported: false,
                },
            )
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let mut compiled = result.unwrap();
    assert!(
        compiled
            .definition
            .attribute_custom_keyword_payment("enspirit")
    );
    // Both routes start from the same attributed compiler definition and IDs,
    // but the native route does not pass through wire materialization.
    let direct = ironsmith_compiler_runtime::into_runtime_definition(compiled.definition.clone())
        .unwrap();
    let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
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
        "enspirit-cost-composition",
        text.as_bytes(),
    );
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn routes() -> [CardDefinition; 2] {
    compile(&format!(
        "Mana cost: {{4}}{{U}}{{B}}\nType: Artifact\n{RULES}"
    ))
}
fn body(game: &mut GameState, owner: PlayerId, mana: u32, enchantment: bool) -> ObjectId {
    let card = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Cost resource",
        &format!(
            "Mana cost: {{{mana}}}\nType: {}Creature — Fox\nPower/Toughness: 1/1",
            if enchantment { "Enchantment " } else { "" }
        ),
        false,
    )
    .unwrap();
    game.create_object_from_definition(&card, owner, Zone::Battlefield)
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}
fn fund(game: &mut GameState, generic: u32) {
    let pool = &mut game.player_mut(A).unwrap().mana_pool;
    pool.add(ManaSymbol::Blue, 1);
    pool.add(ManaSymbol::Black, 1);
    pool.add(ManaSymbol::Colorless, generic);
}
struct Choices {
    pay: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.pay
    }
    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            return ctx
                .options
                .iter()
                .filter(|o| self.pay && o.legal)
                .map(|o| o.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
}
fn announce(game: &mut GameState, source: ObjectId, pay: bool) -> (ObjectId, ironsmith::triggers::TriggerQueue) {
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source))
        .expect("cast affordable with selected resource");
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = Choices { pay };
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
            return (game.stack[0].object_id, queue);
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    panic!("cast did not finish");
}
fn drain(game: &mut GameState) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(
        game,
        &mut queue,
        &mut Choices { pay: false },
    )
    .unwrap();
    assert!(queue.entries.is_empty());
}
fn resource_with_watcher(game: &mut GameState) -> ObjectId {
    let card=ironsmith_compiler_runtime::compile_to_runtime_definition("Enspirited participant","Mana cost: {4}\nType: Creature — Fox\nPower/Toughness: 1/1\nWhenever this creature participates in the enspirit action, you gain 2 life.",false).unwrap();
    game.create_object_from_definition(&card, A, Zone::Battlefield)
}
#[test]
fn becoming_enspirited_is_a_ltb_participant_event_during_actual_cast_payment() {
    for definition in routes() {
        let mut game = setup();
        fund(&mut game, 0);
        let resource = resource_with_watcher(&mut game);
        let stable = game.object(resource).unwrap().stable_id;
        let card = game.create_object_from_definition(&definition, A, Zone::Hand);
        let (spell, mut queue) = announce(&mut game, card, true);
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        // Finalizing the cast advances priority, whose ordinary SBA/APNAP
        // boundary has already put this captured cost trigger above the spell.
        assert!(queue.entries.is_empty());
        assert_eq!(game.stack.len(), 2, "exactly one participant trigger is already above the spell");
        let trigger = game.stack.last().unwrap();
        assert!(trigger.is_ability);
        assert_eq!(trigger.object_id, resource, "the old participant incarnation owns the trigger");
        let snapshot = trigger.source_snapshot.as_ref().expect("LTB observer snapshot");
        assert_eq!(snapshot.object_id, resource);
        assert_eq!(snapshot.zone, Zone::Battlefield);
        let action = trigger.triggering_event.as_ref().unwrap()
            .downcast::<ironsmith::events::NamedActionEvent>().expect("actual named-action receipt");
        assert_eq!(action.label, "enspirit");
        assert_eq!(action.objects.len(), 1);
        assert_eq!(action.objects[0].object_id, resource);
        // An ordinary later drain acknowledges the completed occurrence;
        // it must not recreate an already stacked trigger.
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(
            &mut game, &mut queue, &mut SelectFirstDecisionMaker,
        ).unwrap();
        assert!(queue.entries.is_empty());
        assert_eq!(game.stack.len(), 2);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(game.player(A).unwrap().life, 22);
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(
            spell, A, vec![Effect::counter(ChooseSpec::SpecificObject(spell))]));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Choices { pay: false }).unwrap();
        drain(&mut game);
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.player(A).unwrap().life, 22);
    }
}
#[test]
fn replacement_prevented_exile_does_not_publish_an_enspirit_participant_receipt() {
    use ironsmith::events::zones::matchers::WouldBeExiledMatcher;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in routes() {
        let mut game = setup();
        fund(&mut game, 4);
        let resource = resource_with_watcher(&mut game);
        let mut filter = ironsmith::target::ObjectFilter::default();
        filter.specific = Some(resource);
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::with_matcher(
                resource,
                A,
                WouldBeExiledMatcher::new(filter),
                ReplacementAction::Prevent,
            ));
        let card = game.create_object_from_definition(&definition, A, Zone::Hand);
        let (spell, queue) = announce(&mut game, card, true);
        assert!(queue.entries.is_empty(), "prevented exile has no actual participant");
        assert_eq!(game.stack.len(), 1, "prevented exile stacks only the spell, no participant trigger");
        drain(&mut game);
        assert!(game.object(resource).is_some());
        assert!(
            game.object(spell)
                .unwrap()
                .optional_costs_paid
                .paid_custom_keyword("enspirit"),
            "nominal cost receipt remains distinct from the replaced physical action"
        );
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(
            game.player(A).unwrap().mana_pool.total(),
            0,
            "no exiled card means no mana-value reduction"
        );
    }
}
