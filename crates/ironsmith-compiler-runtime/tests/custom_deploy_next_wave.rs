//! Source-faithful Deploy. Requires generic entry-designation staging,
//! aggregate tap payment, and status-adjective frontend scratch patches.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::decision::DecisionMaker;
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(id: &str) -> [CardDefinition; 2] {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-deploy-next-wave.json")).unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap();
    let card = &row["representative_cards"][0];
    source_support::definitions(card).unwrap()
}
struct Choices {
    accept: bool,
    contributors: Vec<ObjectId>,
    selections: usize,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.accept
    }
    fn decide_objects(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.selections += 1;
        let constraint = ctx
            .aggregate_constraint
            .as_ref()
            .expect("aggregate lower bound retained");
        assert_eq!(
            constraint.metric,
            ironsmith_core::ChoiceAggregateMetric::Power
        );
        assert!(
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .all(|c| self.contributors.contains(&c.id))
        );
        self.contributors.clone()
    }
}
fn body(game: &mut GameState, owner: PlayerId, power: i32) -> ObjectId {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Deploy contributor",
        &format!("Type: Creature — Human\nPower/Toughness: {power}/{power}"),
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}
#[test]
fn deploy_pays_aggregate_cost_before_entry_and_the_marker_enables_only_its_entry_trigger() {
    for definition in definitions("msem-880") {
        for mode in ["pay", "decline", "insufficient"] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let two = body(&mut game, A, 2);
            let enemy = body(&mut game, B, 8);
            let tapped = body(&mut game, A, 8);
            game.tap(tapped);
            let mut contributors = vec![two];
            if mode != "insufficient" {
                contributors.push(body(&mut game, A, 3));
            }
            let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = game.object(hand).unwrap().stable_id;
            let mut dm = Choices {
                accept: mode != "decline",
                contributors: contributors.clone(),
                selections: 0,
            };
            let receipt = game
                .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
                .unwrap();
            let entered = receipt.original.into_result().unwrap().new_id;
            let paid = mode == "pay";
            assert_eq!(game.has_custom_status(entered, "deployed"), paid, "{mode}");
            assert_eq!(game.is_tapped(two), paid);
            assert!(!game.is_tapped(enemy));
            if !paid {
                assert_eq!(
                    dm.selections, 0,
                    "unpayable or declined costs choose no partial contributors"
                );
            }
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(
                &mut game, &mut queue, &mut dm,
            )
            .unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
                .unwrap();
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            }
            assert_eq!(
                game.player(A).unwrap().life,
                if paid { 25 } else { 20 },
                "status exists before entry trigger matching"
            );
            let reset = game.move_object_by_effect(entered, Zone::Hand).unwrap();
            assert!(
                !game.has_custom_status(reset, "deployed"),
                "designation belongs to the battlefield incarnation"
            );
            assert_eq!(game.find_object_by_stable_id(stable), Some(reset));
        }
    }
}
#[test]
fn all_five_exact_deploy_source_variants_compile_without_loss() {
    for id in ["msem-878", "msem-879", "msem-880", "msem-881", "msem-882"] {
        let _ = definitions(id);
    }
}

#[test]
fn scoped_starting_life_arithmetic_is_claimed_and_evaluated_at_resolution() {
    for (reference, expected) in [("your starting life total plus 7", 37),
        ("your starting life total minus 4",26)] {
        let text=format!("Type: Creature\nPower/Toughness: 1/1\nWhen this creature enters, your life total becomes equal to {reference}.");
        let (artifact,direct)=ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Independent life adjustment",&text,false)).unwrap();
        let wire=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&wire).unwrap()] {
            let mut game=GameState::new(vec!["A".into(),"B".into()],30);
            game.player_mut(A).unwrap().life=11;
            let source=game.create_object_from_definition(&definition,A,Zone::Hand);
            let outcome=ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Battlefield,false),&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();
            for event in outcome.events{game.queue_trigger_event(Default::default(),event);}
            let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();
            ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();assert_eq!(game.stack.len(),1);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,expected);
        }
    }
}
