//! Exact-count optional mixed permanent payments retain custom receipts.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;


const A: PlayerId = PlayerId::from_index(0);

use ironsmith_compiled_artifact::{ArtifactCardId,ArtifactCardIdentity,CompiledCardArtifact,CompiledCardPayload,wire_definition_from_serializable};
#[derive(Default)]
struct Choices {
    pay: Vec<&'static str>,
    seen_optional_costs: Vec<String>,
    pay_index: Option<usize>,
}

impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        true
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            self.seen_optional_costs
                .extend(ctx.options.iter().map(|option| option.description.clone()));
            if let Some(index) = self.pay_index {
                return ctx.options.iter().filter(|option| option.legal && option.index == index).map(|option| option.index).collect();
            }
            return ctx
                .options
                .iter()
                .filter(|option| {
                    option.legal
                        && self.pay.iter().any(|needle| {
                            option.description.to_ascii_lowercase().contains(needle)
                        })
                })
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}

fn mana(game: &mut GameState, symbol: ManaSymbol, amount: u32) {
    game.player_mut(A).unwrap().mana_pool.add(symbol, amount);
}

fn fixture(name: &str, text: &str) -> CardDefinition {
    ironsmith_compiler::word_claims::with_enforcement_options(true, true, || compile_to_runtime_definition(name, text, false)).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("the spell is castable");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
            return game.stack.first().unwrap().object_id;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("cast did not finish");
}
fn definitions(name:&str,text:&str,identity:bool)->[CardDefinition;2] {
 let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),name);
 let builder=if identity{builder.with_custom_keyword("tithe")}else{builder};
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler::CompilerFacade::new().compile_definition(builder,text.into(),ironsmith_compiler::CompilePolicy{allow_unsupported:false})));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let mut compiled=result.unwrap();
 assert!(compiled.definition.attribute_custom_keyword_payment("tithe"));
 let wire=wire_definition_from_serializable(&compiled.definition).unwrap();let direct=ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone()).unwrap();
 let artifact=CompiledCardArtifact::new(ArtifactCardIdentity{local_id:ArtifactCardId(direct.card.id.0),name:direct.card.name.clone(),face_name:None,other_face:None,linked_face_layout:None},CompiledCardPayload{definition:wire,canonical_text:ironsmith_text::compiled_text_lines(&direct).join("\n"),ability_labels:ironsmith_text::ability_surface_texts(&direct)},"universal-keyword-condition",text.as_bytes());
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
#[test]
fn optional_exact_three_cost_uses_only_distinct_untapped_controlled_artifacts_or_creatures() {
 let text="Mana cost: {U}\nType: Instant\nAs an additional cost to cast this spell, you may tap three untapped artifacts and/or creatures you control.\nYou gain 1 life. If its tithe was paid, you gain 2 life.";
 for definition in definitions("Independent mixed payment",text,true) {for accept in [false,true] {for enough in [false,true] {
  let mut game=game();mana(&mut game,ManaSymbol::Blue,1);
  let artifact=fixture("Artifact payment","Type: Artifact");let creature=fixture("Creature payment","Type: Creature\nPower/Toughness: 2/2");let both=fixture("Mixed payment","Type: Artifact Creature\nPower/Toughness: 2/2");
  let first=game.create_object_from_definition(&artifact,A,Zone::Battlefield);let second=game.create_object_from_definition(&creature,A,Zone::Battlefield);
  let third=if enough {Some(game.create_object_from_definition(&both,A,Zone::Battlefield))}else{None};
  let excluded=game.create_object_from_definition(&artifact,A,Zone::Battlefield);game.tap(excluded);
  let opponent=game.create_object_from_definition(&creature,PlayerId(1),Zone::Battlefield);
  let mut dm=Choices{pay:if accept{vec!["tap"]}else{vec![]},..Default::default()};cast(&mut game,&definition,&mut dm);
  let paid=accept && enough;assert_eq!(game.stack[0].optional_costs_paid.paid_custom_keyword("tithe"),paid);
  assert_eq!(game.is_tapped(first),paid);assert_eq!(game.is_tapped(second),paid);if let Some(third)=third{assert_eq!(game.is_tapped(third),paid);}
  assert!(game.is_tapped(excluded));assert!(!game.is_tapped(opponent));
  resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,if paid{23}else{21});
 }}}
}
#[test]
fn named_additional_receipt_does_not_match_unrelated_payments_or_official_keywords() {
 use ironsmith_core::{OptionalCostRef,OptionalCostKind,OptionalCostsPaid};
 let mut paid=OptionalCostsPaid::default();let mut chosen=OptionalCostRef::with_discriminator(OptionalCostKind::Additional,"first independent payment");chosen.custom_keyword=Some("tithe".into());paid.mark_label_paid(chosen);
 assert!(paid.was_paid_label("Tithe"));assert!(!paid.was_paid_label("Harness"));assert!(!paid.was_paid_label("Kicker"));
 let mut unrelated=OptionalCostsPaid::default();unrelated.mark_label_paid(OptionalCostRef::with_discriminator(OptionalCostKind::Additional,"tithe"));assert!(!unrelated.was_paid_label("Tithe"),"opaque discriminators are not inferred as custom identities");
 let mut official=OptionalCostsPaid::default();official.mark_label_paid("Kicker");assert!(!official.was_paid_label("Tithe"));
}
