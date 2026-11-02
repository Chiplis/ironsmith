//! Full immutable Catalyze source, paid casting continuation, and action replacement.
//! Parent runs validation; this module does not introduce gameplay rules.
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

#[derive(Default)]
struct Choices {
    pay: Vec<&'static str>,
    seen_optional_costs: Vec<String>,
    pay_index: Option<usize>,
    decline_optional: bool,
    library_top: bool,
}

impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        !self.decline_optional
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
        if self.library_top {
            if let Some(option) = ctx.options.iter().find(|option| option.legal && option.description.to_ascii_lowercase().contains("top of")) {
                return vec![option.index];
            }
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


#[path = "custom_final_next_wave_support.rs"]
mod source_support;
fn source_routes(name: &str) -> [CardDefinition; 2] {
    let value: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-catalyze-fullsource.json")).unwrap();
    let card = value["mechanics"].as_array().unwrap().iter().flat_map(|row| row["representative_cards"].as_array().unwrap()).find(|card| card["name"] == name).unwrap();
    source_support::definitions(card).unwrap_or_else(|error| panic!("full source {name}: {error}"))
}
#[test]
fn all_sixteen_actual_source_faces_compile_strictly_on_both_routes() {
    let value: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-catalyze-fullsource.json")).unwrap();
    let cards: Vec<_> = value["mechanics"].as_array().unwrap().iter().flat_map(|row| row["representative_cards"].as_array().unwrap()).collect();
    assert_eq!(cards.len(), 16);
    for card in cards { source_routes(card["name"].as_str().unwrap()); }
}
#[test]
fn bazaar_actual_full_source_doubles_each_action_with_independent_providers() {
    use ironsmith::effects::{EffectContext, execute_effect};
    for definition in source_routes("Bazaar Trickster") {
        for count in [1,2] {
            let mut game=game();
            for _ in 0..count { game.create_object_from_definition(&definition,A,Zone::Battlefield); }
            let operation=fixture("Witness", "Type: Sorcery\nPerform the catalyze action as follows: The next spell you cast this turn costs {1} less to cast.");
            let source=game.create_object_from_definition(&operation,A,Zone::Stack);
            for effect in operation.spell_effect.as_ref().unwrap().flattened_default_effects() {
                execute_effect(&mut game,effect,&mut EffectContext::new_default(source,A)).unwrap();
            }
            assert_eq!(game.effect_store.temporary_spell_cost_reductions.len(),1usize<<count);
            assert!(game.effect_store.temporary_spell_cost_reductions.iter().all(|reduction| reduction.remaining_uses==1));
        }
    }
}
#[test]
fn bazaar_payment_is_optional_once_and_its_instruction_finishes_before_priority() {
    for definition in source_routes("Bazaar Trickster") {
        for paid in [false,true] {
            let mut game=game();
            game.create_object_from_definition(&definition,A,Zone::Battlefield);
            let bottom=game.create_object_from_definition(&fixture("Bottom witness","Type: Sorcery\nYou gain 1 life."),A,Zone::Library);
            let top=game.create_object_from_definition(&fixture("Top witness","Type: Sorcery\nYou gain 1 life."),A,Zone::Library);
            assert_eq!(game.player(A).unwrap().library.first(),Some(&bottom));
            assert_eq!(game.player(A).unwrap().library.last(),Some(&top));
            mana(&mut game,ManaSymbol::Colorless,3);
            let spell=fixture("Actual cast", "Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");
            let mut choices=Choices {pay: if paid {vec!["additional"]}else{vec![]},..Default::default()};
            cast(&mut game,&spell,&mut choices);
            assert_eq!(game.stack.len(),1,"paid instruction never becomes a trigger or separate stack entry");
            assert_eq!(game.player(A).unwrap().mana_pool.total(),if paid {0}else{2});
            if paid {
                assert!(game.player(A).unwrap().graveyard.iter().any(|id| game.object(*id).is_some_and(|object| object.name == "Bottom witness")));
            } else { assert_eq!(game.object(bottom).unwrap().zone,Zone::Library); }
            assert_eq!(game.object(top).unwrap().zone,Zone::Library);
            assert_eq!(choices.seen_optional_costs.len(),1);
        }
    }
}
#[test]
fn paid_library_instruction_can_decline_and_empty_library_does_not_reuse_a_reference() {
    for definition in source_routes("Bazaar Trickster") {
        for library in [false,true] {
            let mut game=game();game.create_object_from_definition(&definition,A,Zone::Battlefield);
            let bottom=library.then(||game.create_object_from_definition(&fixture("Only witness","Type: Sorcery\nYou gain 1 life."),A,Zone::Library));
            mana(&mut game,ManaSymbol::Colorless,3);
            let mut choices=Choices{pay:vec!["additional"],decline_optional:true,..Default::default()};
            cast(&mut game,&fixture("Actual cast","Mana cost: {1}\nType: Sorcery\nYou gain 1 life."),&mut choices);
            assert_eq!(game.stack.len(),1);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
            if let Some(bottom)=bottom {assert_eq!(game.object(bottom).unwrap().zone,Zone::Library);}
        }
    }
}

#[test]
fn bazaar_can_put_the_actual_bottom_card_on_top_without_moving_the_old_top() {
    for definition in source_routes("Bazaar Trickster") {
        let mut game=game();game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let bottom=game.create_object_from_definition(&fixture("Bottom witness","Type: Sorcery\nYou gain 1 life."),A,Zone::Library);
        let top=game.create_object_from_definition(&fixture("Top witness","Type: Sorcery\nYou gain 1 life."),A,Zone::Library);
        mana(&mut game,ManaSymbol::Colorless,3);
        let mut choices=Choices{pay:vec!["additional"],library_top:true,..Default::default()};
        cast(&mut game,&fixture("Actual cast","Mana cost: {1}\nType: Sorcery\nYou gain 1 life."),&mut choices);
        assert_eq!(game.player(A).unwrap().library.last(),Some(&bottom));
        assert_eq!(game.object(top).unwrap().zone,Zone::Library);
        assert_eq!(game.stack.len(),1);
    }
}
#[test]
fn acquired_payment_and_consequence_survive_provider_used_to_fund_the_cast() {
    let text="Type: Artifact\nYou may pay an additional {2} as you cast a spell. If you do, you gain 4 life.\n{T}, Sacrifice this artifact: Add {C}{C}{C}.";
    let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact("Funding witness",text,false).unwrap();
    let roundtrip=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&roundtrip).unwrap()] {
        let mut game=game();let provider=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let provider_card=game.object(provider).unwrap().card;
        let mut choices=Choices{pay:vec!["additional"],..Default::default()};
        cast(&mut game,&fixture("Actual cast","Mana cost: {1}\nType: Sorcery\nYou gain 1 life."),&mut choices);
        assert!(!game.battlefield.iter().any(|id|game.object(*id).is_some_and(|object|object.card==provider_card)));
        assert_eq!(game.player(A).unwrap().life,24,"paid casting consequence survives granting provider's departure");
        assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
        assert_eq!(game.stack.len(),1);
    }
}

#[test]
fn paid_instruction_survives_recipient_ability_loss_during_payment() {
    let text="Type: Artifact\nYou may pay an additional {2} as you cast a spell. If you do, you gain 4 life.\n{T}, Sacrifice this artifact: Add {C}{C}{C}. All spells lose all abilities until end of turn.";
    let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact("Funding witness",text,false).unwrap();
    let roundtrip=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&roundtrip).unwrap()] {
        let mut game=game();let provider=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let provider_card=game.object(provider).unwrap().card;
        let mut choices=Choices{pay:vec!["additional"],..Default::default()};
        cast(&mut game,&fixture("Actual cast","Mana cost: {1}\nType: Sorcery\nYou gain 1 life."),&mut choices);
        assert!(!game.battlefield.iter().any(|id|game.object(*id).is_some_and(|object|object.card==provider_card)));
        assert_eq!(game.player(A).unwrap().life,24,"paid casting consequence survives granting provider's departure");
        assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
        assert_eq!(game.stack.len(),1);
    }
}
