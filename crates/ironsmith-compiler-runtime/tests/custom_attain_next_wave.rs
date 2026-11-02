//! Actual Attain source, generic minimum actual color payment.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
fn definitions(id: &str) -> Vec<[CardDefinition; 2]> {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-final-abilities.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    let card = &row["representative_cards"][0];
    source_support::linked_definitions(card).unwrap()
}
#[test]
fn attain_rejects_enough_mana_of_two_colors_and_pays_three_actual_colors_at_sorcery_speed() {
    use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    let faces = definitions("msem-862");
    assert_eq!(faces.len(), 2);
    for route in 0..2 {
        let front = faces[0][route].clone();
        let back = faces[1][route].clone();
        assert_eq!(front.card.other_face, Some(back.card.id));
        assert_eq!(back.card.other_face, Some(front.card.id));
        assert!(front.card.transforming_dfc && back.card.transforming_dfc);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
        let source = game.create_object_from_definition(&front, A, Zone::Battlefield);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);
        let action = |g: &GameState| {
            compute_legal_actions(g, A)
                .unwrap()
                .into_iter()
                .find(|a| matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source))
        };
        assert!(
            action(&game).is_none(),
            "five mana with only two actual colors is insufficient"
        );
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        game.turn.phase = ironsmith::Phase::Combat;
        assert!(action(&game).is_none());
        game.turn.phase = ironsmith::Phase::FirstMain;
        use ironsmith::game_loop::{
            PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
            apply_priority_response_with_dm,
        };
        let mut q = ironsmith::triggers::TriggerQueue::new();
        let mut state = PriorityLoopState::new(2);
        let mut dm = SelectFirstDecisionMaker;
        let chosen = action(&game).unwrap();
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut q,
            &mut state,
            &PriorityResponse::PriorityAction(chosen),
            &mut dm,
        )
        .unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{progress:?}")
            };
            progress = apply_decision_context_with_dm(&mut game, &mut q, &mut state, &ctx, &mut dm)
                .unwrap();
        }
        assert!(!state.has_pending_action());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.object(source).unwrap().name, back.card.name);
    }
}
#[test]
fn all_sixteen_attain_source_variants_compile_without_loss() {
    for n in 862..=877 {
        let _ = definitions(&format!("msem-{n}"));
    }
}

fn independent_definitions(name:&str,text:&str)->[CardDefinition;2] {
    source_support::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap()
}
struct LastChoice;
impl ironsmith::decision::DecisionMaker for LastChoice {
    fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
        ctx.options.iter().rev().find(|option|option.legal).map(|option|vec![option.index]).unwrap_or_default()
    }
}
#[test]
fn mixed_fixed_and_source_color_mana_keeps_granted_recipient_colors_and_exactly_one_output(){
    use ironsmith::ability::{AbilityKind,ActivatedAbilityRuntimeExt};
    use ironsmith::mana::ManaSymbol;
    let grant=independent_definitions("Independent color teacher","Mana cost: {G}\nType: Creature\nPower/Toughness: 1/2\nCreatures you control have \"{T}: Add {C} or one mana of any of this creature's colors.\"");
    let hosts=independent_definitions("Independent red recipient","Mana cost: {R}\nType: Creature\nPower/Toughness: 2/2");
    for route in 0..2{for fixed in [false,true]{
        let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.create_object_from_definition(&grant[route],A,Zone::Battlefield);let host=g.create_object_from_definition(&hosts[route],A,Zone::Battlefield);
        let ability=g.current_abilities(host).unwrap().into_iter().find_map(|ability|match ability.kind{AbilityKind::Activated(a)=>Some(a),_=>None}).expect("recipient mana ability");
        let outputs=ability.inferred_mana_symbols(&g,host,A);assert!(outputs.contains(&ManaSymbol::Red));assert!(outputs.contains(&ManaSymbol::Colorless));assert!(!outputs.contains(&ManaSymbol::Green),"grantor is not the mana producer");
        let mut first=ironsmith::decision::SelectFirstDecisionMaker;let mut last=LastChoice;let dm:&mut dyn ironsmith::decision::DecisionMaker=if fixed{&mut first}else{&mut last};
        let mut ctx=ironsmith::effects::EffectContext::new(host,A,dm);for effect in ability.effects.all_effects(){ironsmith::effects::execute_effect(&mut g,effect,&mut ctx).unwrap();}
        assert_eq!(g.player(A).unwrap().mana_pool.total(),1);assert_eq!(g.player(A).unwrap().mana_pool.green,0);
        assert_eq!(g.player(A).unwrap().mana_pool.red,u32::from(!fixed));assert_eq!(g.player(A).unwrap().mana_pool.colorless,u32::from(fixed));
    }}
}
#[test]
fn activator_scoped_reducer_retains_one_total_mana_floor_and_colored_pips(){
    use ironsmith::decision::{compute_legal_actions,LegalAction};use ironsmith::mana::ManaSymbol;
    let reducers=independent_definitions("Independent activation teacher","Type: Creature\nPower/Toughness: 3/5\nActivated abilities you activate cost {3} less to activate. This effect can't reduce the mana in that cost to less than one mana.");
    for (price,colored) in [("{2}",false),("{1}{U}",true)]{let bodies=independent_definitions("Independent priced recipient",&format!("Type: Creature\nPower/Toughness: 2/2\n{price}: You gain 1 life."));
    for route in 0..2{let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;
        g.create_object_from_definition(&reducers[route],A,Zone::Battlefield);let host=g.create_object_from_definition(&bodies[route],A,Zone::Battlefield);
        let legal=|game:&GameState|compute_legal_actions(game,A).unwrap().iter().any(|action|matches!(action,LegalAction::ActivateAbility{source,..}if *source==host));
        assert!(!legal(&g),"minimum floor does not create a free activation");g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);assert_eq!(legal(&g),!colored,"generic reduction cannot erase a colored pip");
        if colored{g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue,1);assert!(legal(&g));}
        let opponent=g.create_object_from_definition(&bodies[route],PlayerId(1),Zone::Battlefield);g.turn.priority_player=Some(PlayerId(1));
        g.player_mut(PlayerId(1)).unwrap().mana_pool.add(if colored{ManaSymbol::Blue}else{ManaSymbol::Colorless},1);
        assert!(!compute_legal_actions(&g,PlayerId(1)).unwrap().iter().any(|action|matches!(action,LegalAction::ActivateAbility{source,..}if *source==opponent)),"opponent's activation receives no reduction");
    }}
}

#[test]
fn unrestricted_mixed_mana_keeps_fixed_output_and_one_color_output_through_both_routes() {
    use ironsmith::ability::{AbilityKind,ActivatedAbilityRuntimeExt};
    use ironsmith::ManaSymbol;
    let grants=independent_definitions("Independent unrestricted teacher","Type: Creature\nPower/Toughness: 1/2\nCreatures you control have \"{T}: Add {C} or one mana of any color.\"");
    let hosts=independent_definitions("Independent recipient","Type: Creature\nPower/Toughness: 2/2");
    for route in 0..2 {for fixed in [true,false] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.create_object_from_definition(&grants[route],A,Zone::Battlefield);
        let host=game.create_object_from_definition(&hosts[route],A,Zone::Battlefield);
        let ability=game.current_abilities(host).unwrap().into_iter().find_map(|ability|match ability.kind {
            AbilityKind::Activated(ability)=>Some(ability),_=>None,
        }).unwrap();
        let outputs=ability.inferred_mana_symbols(&game,host,A);
        for symbol in [ManaSymbol::Colorless,ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green] {
            assert!(outputs.contains(&symbol),"{outputs:?}");
        }
        let mut first=ironsmith::decision::SelectFirstDecisionMaker;let mut last=LastChoice;
        let dm:&mut dyn ironsmith::decision::DecisionMaker=if fixed{&mut first}else{&mut last};
        let mut ctx=ironsmith::effects::EffectContext::new(host,A,dm);
        for effect in ability.effects.all_effects(){ironsmith::effects::execute_effect(&mut game,effect,&mut ctx).unwrap();}
        assert_eq!(game.player(A).unwrap().mana_pool.total(),1);
        assert_eq!(game.player(A).unwrap().mana_pool.colorless,u32::from(fixed));
    }}
}

#[test]
fn actual_attain_back_hooks_require_their_own_attain_activation_not_an_unrelated_transform() {
    use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
    for row in ["msem-868","msem-870"] {
        let faces=definitions(row);
        for route in 0..2 { for own in [false,true] {
            let front=&faces[0][route];let back=&faces[1][route];
            let mut game=GameState::new(vec!["A".into(),"B".into()],20);
            game.register_linked_face_definition(front);game.register_linked_face_definition(back);
            game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
            let source=game.create_object_from_definition(front,A,Zone::Battlefield);
            let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Discard witness","Type: Artifact",false).unwrap();
            game.create_object_from_definition(&witness,PlayerId(1),Zone::Hand);
            for color in [ironsmith::mana::ManaSymbol::White,ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Black,ironsmith::mana::ManaSymbol::Green] {
                game.player_mut(A).unwrap().mana_pool.add(color,5);
            }
            let mut q=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
            if own {
                let chosen=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source)).unwrap();
                let mut state=PriorityLoopState::new(2);
                let mut progress=apply_priority_response_with_dm(&mut game,&mut q,&mut state,&PriorityResponse::PriorityAction(chosen),&mut dm).unwrap();
                for _ in 0..64 {
                    if !state.has_pending_action(){break;}
                    let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};
                    progress=apply_decision_context_with_dm(&mut game,&mut q,&mut state,&ctx,&mut dm).unwrap();
                }
                assert!(!state.has_pending_action());
                ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            } else {
                let effect=ironsmith::Effect::transform(ironsmith::ChooseSpec::SpecificObject(source));
                let output=ironsmith::effects::execute_effect(&mut game,&effect,&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();
                for event in output.events {game.queue_trigger_event(Default::default(),event);}
            }
            for _ in 0..32 {
                ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut q,&mut dm).unwrap();
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut q,&mut dm).unwrap();
                if game.stack.is_empty(){break;}
                ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            }
            assert_eq!(game.object(source).unwrap().name,back.card.name);
            if row=="msem-868" {assert_eq!(game.battlefield.len(),if own{3}else{1});}
            else {assert_eq!(game.player(PlayerId(1)).unwrap().hand.len(),usize::from(!own));}
        }}
    }
}
