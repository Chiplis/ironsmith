//! Generic selected sacrifice prices compose origin, payment and discount riders.
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
fn completed_copy_program_attributes_the_action_to_actual_tokens() {
    let text="Mana cost: {2}{B}\nType: Creature — Zombie\nPower/Toughness: 2/2\nWhen this permanent performs the arise action, you gain 3 life.\n{0}, Exile this card from your graveyard: Create a token that's a copy of this card, except it's black, it's a Zombie in addition to its other types, and it has \"This creature can't block.\" Have those tokens perform the arise action. Activate only as a sorcery.";
    for definition in definitions("Creation action witness",text,None) {
        let mut game=game();
        let source=game.create_object_from_definition(&definition,A,Zone::Graveyard);
        let stable=game.object(source).unwrap().stable_id;
        let selected=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility {source:id,..}if *id==source)).unwrap();
        act(&mut game,selected);
        settle(&mut game);
        let old=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(old).unwrap().zone,Zone::Exile);
        let tokens:Vec<_>=game.battlefield.iter().copied().collect();
        assert_eq!(tokens.len(),1);
        assert_eq!(game.named_action_count(tokens[0],"arise",false),1);
        assert_eq!(game.named_action_count(old,"arise",false),0);
        assert_eq!(game.player(A).unwrap().life,23,"the copied token's own observer sees its completed action");
    }
}
#[test]
fn doubled_and_prevented_creation_receipts_follow_only_completed_token_objects() {
    let text="Mana cost: {2}{B}\nType: Creature — Zombie\nPower/Toughness: 2/2\nWhen this permanent performs the arise action, you gain 3 life.\n{0}, Exile this card from your graveyard: Create a token that's a copy of this card, except it's black, it's a Zombie in addition to its other types, and it has \"This creature can't block.\" Have those tokens perform the arise action. Activate only as a sorcery.";
    for definition in definitions("Modified creation witness",text,None) {
        for expected in [0usize,2] {
            let mut game=game();
            let source=game.create_object_from_definition(&definition,A,Zone::Graveyard);
            let multiplier=ironsmith_compiler_runtime::compile_to_runtime_definition("Creation modifier","Type: Enchantment\nIf an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.",false).unwrap();
            let modifier=game.create_object_from_definition(&multiplier,A,Zone::Battlefield);
            if expected==0 {
                game.effect_store.replacement_effects.add_one_shot_effect(
                    ironsmith::replacement::ReplacementEffect::with_matcher(modifier,A,
                        ironsmith::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(ironsmith::PlayerFilter::You),
                        ironsmith::replacement::ReplacementAction::Prevent));
            }
            let selected=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility {source:id,..}if *id==source)).unwrap();
            act(&mut game,selected);
            settle(&mut game);
            let tokens:Vec<_>=game.battlefield.iter().copied().filter(|id|*id!=modifier).collect();
            assert_eq!(tokens.len(),expected);
            for token in tokens {
                assert_eq!(game.named_action_count(token,"arise",false),1,"one completion per actual created token");
            }
            assert_eq!(game.player(A).unwrap().life,20+3*expected as i32,"only actual tokens can observe their own completed action");
        }
    }
}
#[test]
fn countered_arise_program_never_emits_a_completion() {
    let text="Type: Creature — Zombie\nPower/Toughness: 2/2\nWhen this permanent performs the arise action, you gain 3 life.\n{0}, Exile this card from your graveyard: Create a token that's a copy of this card, except it's black, it's a Zombie in addition to its other types, and it has \"This creature can't block.\" Have those tokens perform the arise action. Activate only as a sorcery.";
    for definition in definitions("Countered creation witness",text,None) {
        let mut game=game();
        let source=game.create_object_from_definition(&definition,A,Zone::Graveyard);
        let stable=game.object(source).unwrap().stable_id;
        let selected=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility {source:id,..}if *id==source)).unwrap();
        act(&mut game,selected);
        let ability=game.stack.last().unwrap().ability_id.unwrap();
        execute_effect(&mut game,&Effect::new(ironsmith::effects::CounterEffect::new(ChooseSpec::SpecificObject(ability))),&mut EffectContext::new_default(ability,A)).unwrap();
        settle(&mut game);
        assert!(game.battlefield.is_empty());
        assert_eq!(game.player(A).unwrap().life,20);
        let old=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(old).unwrap().zone,Zone::Exile,"activation cost remains paid");
        assert_eq!(game.named_action_count(old,"arise",false),0);
    }
}
#[test]
fn full_bugrund_source_arise_observer_attaches_equipment_to_the_new_token() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-arise.json")).unwrap();
    let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]=="msem-846").unwrap();
    let card=&row["representative_cards"][0];
    assert!(card["original_text"].as_str().unwrap().contains("When Bugrund arises"));
    assert!(card["executable_text"].as_str().unwrap().contains("Have those tokens perform the arise action"));
    for definition in definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap(),None) {
        for origin in [Zone::Hand,Zone::Graveyard] {
            let mut game=game();
            let equipment_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Equipment witness","Type: Artifact — Equipment\nEquipped creature gets +1/+1.\nEquip {1}",false).unwrap();
            let equipment=game.create_object_from_definition(&equipment_definition,A,origin);
            let equipment_stable=game.object(equipment).unwrap().stable_id;
            let source=game.create_object_from_definition(&definition,A,Zone::Graveyard);
            let source_stable=game.object(source).unwrap().stable_id;
            for (symbol,amount) in [(ManaSymbol::Colorless,3),(ManaSymbol::Red,1),(ManaSymbol::White,1)] {
                game.player_mut(A).unwrap().mana_pool.add(symbol,amount);
            }
            let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility {source:id,..}if *id==source)).unwrap();
            act(&mut game,action);
            settle(&mut game);
            let equipment=game.find_object_by_stable_id(equipment_stable).unwrap();
            assert_eq!(game.object(equipment).unwrap().zone,Zone::Battlefield);
            let token=*game.battlefield.iter().find(|id|**id!=equipment).unwrap();
            assert_eq!(game.object(equipment).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(token)));
            assert_eq!(game.named_action_count(token,"arise",false),1);
            let source=game.find_object_by_stable_id(source_stable).unwrap();
            assert_eq!(game.object(source).unwrap().zone,Zone::Exile);
            assert_ne!(source,token);
        }
    }
}
