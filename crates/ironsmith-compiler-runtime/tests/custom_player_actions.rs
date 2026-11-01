//! Explicit custom action player binding uses existing target and unless operations.
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, GameProgress, GameState, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
const B:PlayerId=PlayerId(1);
const BODY:&str="that player loses 3 life unless they discard a card or sacrifice a nonland permanent";
fn definitions(text:&str)->[CardDefinition;2] {
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Repeated player action",text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());
    let (artifact,definition)=result.unwrap();
    let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact,decoded);
    [definition,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[derive(Default)]
struct Choices { target_prompts:usize, requirements:usize }
impl DecisionMaker for Choices {
    fn decide_targets(&mut self,_game:&GameState,ctx:&TargetsContext)->Vec<Target> {
        self.target_prompts+=1;
        self.requirements+=ctx.requirements.len();
        assert!(ctx.requirements.iter().all(|r|r.legal_targets.contains(&Target::Player(B))));
        vec![Target::Player(B)]
    }
}
fn game()->GameState {
    let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);
    game.turn.phase=ironsmith::game_state::Phase::FirstMain;
    game.turn.step=None;game.turn.active_player=A;game.turn.priority_player=Some(A);game
}
fn cast(game:&mut GameState,definition:&CardDefinition,dm:&mut Choices) {
    let spell=game.create_object_from_definition(definition,A,Zone::Hand);
    let action=compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell)).unwrap();
    let mut queue=TriggerQueue::new();let mut state=PriorityLoopState::new(3);
    let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action(){return;}
        let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};
        progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).unwrap();
    }
    panic!("cast did not complete");
}
#[test]
fn player_target_prelude_binds_a_following_life_instruction() {
    for definition in definitions("Mana cost: {0}\nType: Sorcery\nChoose target player. That player loses 3 life.") {
        let mut game=game(); let mut dm=Choices::default();
        cast(&mut game,&definition,&mut dm);
        assert_eq!(game.stack[0].targets, vec![Target::Player(B)]);
        resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(B).unwrap().life,17);
        assert_eq!(game.player(A).unwrap().life,20);
    }
}

#[test]
fn repeated_target_player_action_announces_one_target_and_keeps_all_iterations_on_it() {
    // Exact generic frontend form: the target prelude is outside repeated body.
    let text=format!("Mana cost: {{0}}\nType: Sorcery\nChoose target player. {BODY}. {BODY}. {BODY}.");
    for definition in definitions(&text) { for resource in ["none", "card", "permanent", "land"] {
        let mut game=game();let mut dm=Choices::default();
        let reserve = ironsmith_compiler_runtime::compile_to_runtime_definition("Reserve", "Type: Creature\nPower/Toughness: 1/1", false).unwrap();
        let caster_card = game.create_object_from_definition(&reserve, A, Zone::Hand);
        let payment = match resource {
            "card" => Some(game.create_object_from_definition(&reserve, B, Zone::Hand)),
            "permanent" => Some(game.create_object_from_definition(&reserve, B, Zone::Battlefield)),
            "land" => {
                let land = ironsmith_compiler_runtime::compile_to_runtime_definition("Field", "Type: Land", false).unwrap();
                Some(game.create_object_from_definition(&land, B, Zone::Battlefield))
            }
            _ => None,
        };
        cast(&mut game,&definition,&mut dm);
        assert_eq!(dm.target_prompts,1);assert_eq!(dm.requirements,1);
        resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker)
            .unwrap_or_else(|error| panic!("{error:?}\nCompiled player program: {:#?}", definition.spell_effect));
        assert_eq!(game.player(A).unwrap().life,20);
        let expected_life = if matches!(resource, "card" | "permanent") {14} else {11};
        if game.player(B).unwrap().life != expected_life {
            eprintln!("PLAYER_ACTION_PROGRAM: {:#?}\nPLAYER_ACTION_LIFE: {:?}", definition.spell_effect, [game.player(A).unwrap().life, game.player(B).unwrap().life, game.player(PlayerId(2)).unwrap().life]);
        }
        assert_eq!(game.player(B).unwrap().life, expected_life, "resource={resource}");
        assert_eq!(game.player(PlayerId(2)).unwrap().life,20);
        assert!(game.player(A).unwrap().hand.contains(&caster_card), "the caster never pays the selected player's cost");
        if resource == "land" { assert!(game.battlefield.contains(&payment.unwrap())); }
    } }
}

#[test]
fn repeated_token_then_group_counters_keeps_each_complete_action_in_order() {
    use ironsmith::object::CounterType;
    use ironsmith::types::Subtype;
    const RECRUIT: &str = "Create a 0/0 white Soldier Army creature token. Then put a +1/+1 counter on each Army you control.";
    for count in [1, 2] {
        let text = format!("Mana cost: {{0}}\nType: Sorcery\n{}", vec![RECRUIT; count].join(" "));
        for definition in definitions(&text) {
            let mut game = game();
            let army = ironsmith_compiler_runtime::compile_to_runtime_definition("Veterans", "Type: Creature — Army\nPower/Toughness: 2/2", false).unwrap();
            let bystander = ironsmith_compiler_runtime::compile_to_runtime_definition("Bystander", "Type: Creature — Soldier\nPower/Toughness: 2/2", false).unwrap();
            let own = game.create_object_from_definition(&army, A, Zone::Battlefield);
            let opponent = game.create_object_from_definition(&army, B, Zone::Battlefield);
            let other = game.create_object_from_definition(&bystander, A, Zone::Battlefield);
            game.object_mut(own).unwrap().counters.insert(CounterType::PlusOnePlusOne, 2);
            let mut dm = Choices::default();
            cast(&mut game, &definition, &mut dm);
            assert_eq!(dm.target_prompts, 0, "group counter placement is not targeted");
            resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.counter_count(own, CounterType::PlusOnePlusOne), 2 + count as u32);
            assert_eq!(game.counter_count(opponent, CounterType::PlusOnePlusOne), 0);
            assert_eq!(game.counter_count(other, CounterType::PlusOnePlusOne), 0);
            let mut created = game.battlefield.iter().copied()
                .filter(|id| *id != own && *id != opponent)
                .filter(|id| game.object(*id).unwrap().subtypes.contains(&Subtype::Army))
                .map(|id| {
                    assert_eq!(game.controller_of(game.object(id).unwrap()), A);
                    game.counter_count(id, CounterType::PlusOnePlusOne)
                }).collect::<Vec<_>>();
            created.sort();
            assert_eq!(created, (1..=count as u32).collect::<Vec<_>>(), "each creation is followed by its own whole-group counter instruction");
        }
    }
}
