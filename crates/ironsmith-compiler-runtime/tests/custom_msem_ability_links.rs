//! Source-equivalent MSEM ability links; direct/artifact negative cases.
use ironsmith::{CardDefinition, GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith_compiled_artifact::{ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload, wire_definition_from_serializable};
const A: PlayerId = PlayerId(0);
fn compile_named(name: &str, text: &str, keyword: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler::CompilerFacade::new().compile_definition(ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), name).with_custom_keyword(keyword), text.to_string(), ironsmith_compiler::CompilePolicy {allow_unsupported: false})));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let mut compiled = result.unwrap();
    assert!(compiled.definition.attribute_custom_keyword_payment(keyword));
    let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
    let direct = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone()).unwrap();
    assert!(direct.optional_costs.is_empty(), "the total alternative price must not become an additional cost");
    assert_eq!(direct.alternative_casts.len(), 1);
    assert!(direct.alternative_casts[0].keyword().is_none());
    let artifact = CompiledCardArtifact::new(ArtifactCardIdentity {local_id: ArtifactCardId(direct.card.id.0), name: direct.card.name.clone(), face_name: None, other_face: None, linked_face_layout: None}, CompiledCardPayload {definition: wire, canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"), ability_labels: ironsmith_text::ability_surface_texts(&direct)}, "ability-link-payment-test", text.as_bytes());
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

fn fixture_card(id: &str) -> (String,String) {
 let data:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-ability-links.json")).unwrap();
 let row=data["mechanics"].as_array().unwrap().iter().find(|row|row["id"]==id).unwrap();
 let card=&row["representative_cards"][0];
 (card["name"].as_str().unwrap().into(),card["executable_text"].as_str().expect("frontend materialized source").into())
}
fn flush(game:&mut GameState) {
 let mut queue=TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut dm).unwrap();
 ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut dm).unwrap();
}
#[test]
fn embellish_selected_price_creates_thopter_only_after_entry_trigger() {
 let (name,text)=fixture_card("msem-684");
 for definition in compile_named(&name,&text,"embellish") { for alternative in [false,true] {
  let mut game=setup();game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,3);
  let spell=cast(&mut game,&definition,alternative);let stable=game.object(spell).unwrap().stable_id;
  assert_eq!(game.stack[0].optional_costs_paid.paid_custom_keyword("embellish"),alternative);
  assert_eq!(game.player(A).unwrap().mana_pool.total(),if alternative{0}else{2});
  resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  let permanent=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(permanent).unwrap().zone,Zone::Battlefield);
  let thopters=|g:&GameState|g.battlefield.iter().filter(|id|g.object(**id).unwrap().has_subtype(ironsmith::types::Subtype::Thopter)).count();
  assert_eq!(thopters(&game),0,"Embellish is an entry trigger, not an entry replacement");
  flush(&mut game);
  assert_eq!(game.stack.len(),usize::from(alternative),"normal payment must not pass intervening-if");
  if alternative {resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  assert_eq!(thopters(&game),usize::from(alternative));
  if alternative {
   let token=game.battlefield.iter().filter_map(|id|game.object(*id)).find(|o|o.has_subtype(ironsmith::types::Subtype::Thopter)).unwrap();
   assert!(token.card_types.contains(&ironsmith::types::CardType::Artifact));assert!(token.is_creature());assert_eq!(token.power(),Some(1));assert_eq!(token.toughness(),Some(1));
   assert!(token.has_static_ability_id(ironsmith::static_abilities::StaticAbilityId::Flying));
  }
 }}
}
#[test]
fn pressure_threshold_counts_only_your_graveyard_and_keeps_normal_price() {
 let text="Mana cost: {4}{B}\nType: Sorcery\nYou may pay {B} rather than pay this spell's mana cost if there are seven or more cards in your graveyard.\nYou gain 1 life.";
 for definition in compile_named("Independent graveyard price",text,"pressure") { for own in [false,true] {for count in [6,7,8] {
  let mut game=setup();let filler=ironsmith_compiler_runtime::compile_to_runtime_definition("Graveyard filler","Type: Sorcery",false).unwrap();
  for _ in 0..count {game.create_object_from_definition(&filler,if own{A}else{PlayerId(1)},Zone::Graveyard);}
  let spell=game.create_object_from_definition(&definition,A,Zone::Hand);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black,1);
  let has_alt=compute_legal_actions(&game,A).unwrap().iter().any(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method:CastingMethod::Alternative(_),..} if *spell_id==spell));
  assert_eq!(has_alt,own && count>=7);
  game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,4);
  assert!(compute_legal_actions(&game,A).unwrap().iter().any(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..} if *spell_id==spell && !matches!(casting_method,CastingMethod::Alternative(_)))));
 }}}
}

#[test]
fn beckon_source_animation_preserves_enchantment_and_does_nothing_to_existing_creatures() {
 use ironsmith::ability::AbilityKind;
 use ironsmith::effects::{EffectContext,execute_effect};
 use ironsmith::types::{CardType,Subtype};
 let (name,text)=fixture_card("msem-699");
 let vocabulary=[ironsmith_core::custom_subtypes::CustomSubtypeDefinition{singular:"Deity".into(),plural:"Deities".into(),family:ironsmith_core::SubtypeFamily::Enchantment}];
 let (artifact,direct)=ironsmith_compiler::with_custom_subtypes(&vocabulary,||ironsmith_compiler_runtime::compile_to_artifact(&name,&text,false)).unwrap().unwrap();
 let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 assert_eq!(artifact,decoded);
 for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()] {
  let effects=definition.abilities.iter().find_map(|a|match &a.kind{AbilityKind::Activated(a)=>Some(a.effects.all_effects_owned()),_=>None}).unwrap();
  for already_creature in [false,true] {
   let mut game=setup();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
   if already_creature {let object=game.object_mut(source).unwrap();object.card_types.push(CardType::Creature);object.subtypes.push(Subtype::Drake);}
   let mut context=EffectContext::new_default(source,A);
   for effect in &effects {execute_effect(&mut game,effect,&mut context).unwrap();}
   let chars=game.calculated_characteristics(source).unwrap();
   assert!(chars.card_types.contains(&CardType::Enchantment));
   assert!(chars.card_types.contains(&CardType::Creature));
   assert_eq!(game.current_power(source),Some(3));assert_eq!(game.current_toughness(source),Some(3));
   assert_eq!(chars.subtypes.contains(&Subtype::Ox),!already_creature,"a creature's existing types must not change even if they differ from Beckon's type");
   if already_creature {assert!(chars.subtypes.contains(&Subtype::Drake));}
   game.effect_store.continuous_effects.cleanup_end_of_turn();
   let after=game.calculated_characteristics(source).unwrap();
   assert!(after.card_types.contains(&CardType::Creature),"Beckon lasts indefinitely after turn cleanup");
   assert_eq!(after.subtypes.contains(&Subtype::Ox),!already_creature);
  }
 }
}
