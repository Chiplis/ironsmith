//! Recovered full-body scenarios, authored but UNRUN during the campaign pause.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{NumberContext, SelectObjectsContext, TargetsContext, OrderContext};
use ironsmith::effect::{Effect, EffectId};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, direct_loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = result.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "{}", direct_loss.reasons_text());
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&direct));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/activation_counter_receipts.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) { text += &format!("Power/Toughness: {p}/{t}\n"); }
    text += row["oracle_text"].as_str().unwrap();
    definitions_text(name, &text)
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    game.turn.phase = Phase::FirstMain; game.turn.step = None;
    game.turn.active_player = A; game.turn.priority_player = Some(A); game
}
fn mana(game: &mut GameState, color: ManaSymbol, amount: u32) { game.player_mut(A).unwrap().mana_pool.add(color, amount); }
fn creature(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, power: i32, toughness: i32) -> ObjectId {
    let definition = compile_to_runtime_definition(name, &format!("Type: Creature — Soldier\nPower/Toughness: {power}/{toughness}"), false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn library(game: &mut GameState, count: usize) -> Vec<ObjectId> {
    (0..count).map(|n| creature(game, A, Zone::Library, &format!("Library card {n}"), 2, 3)).collect()
}
#[derive(Default)]
struct Choices { quantity: u32, announced_x: u32, target: Option<Target>, pick: Option<ObjectId>, numbers: Vec<u32>, ordered: Vec<ObjectId> }
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        let number = if ctx.is_x_value { self.announced_x } else { self.numbers.push(ctx.min); self.quantity };
        assert!(number >= ctx.min && number <= ctx.max); number
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target { assert!(ctx.requirements.iter().all(|r| r.legal_targets.contains(&target))); vec![target] }
        else { SelectFirstDecisionMaker.decide_targets(game, ctx) }
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.pick.filter(|id| ctx.candidates.iter().any(|c| c.id == *id && c.legal)) { vec![id] }
        else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
    }
    fn decide_order(&mut self, _: &GameState, ctx: &OrderContext) -> Vec<ObjectId> {
        self.ordered = ctx.items.iter().rev().map(|(id, _)| *id).collect(); self.ordered.clone()
    }
}
fn drain(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new(); drain_pending_trigger_events(game, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    while !game.stack.is_empty() {
        resolve_stack_entry_with(game, dm).unwrap(); drain_pending_trigger_events(game, &mut queue);
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    }
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect, dm: &mut Choices) {
    let controller = game.current_controller(source).unwrap_or(A);
    let outcome = execute_effect(game, &effect, &mut EffectContext::new(source, controller, dm)).unwrap();
    let mut queue = TriggerQueue::new();
    for event in outcome.events { for entry in check_triggers(game, &event) { queue.add(entry); } }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap(); drain(game, dm);
}
fn begin(game: &mut GameState, source: ObjectId, ordinal: usize, dm: &mut Choices) -> (TriggerQueue, PriorityLoopState, GameProgress) {
    game.turn.priority_player = Some(A);
    let index = game.current_abilities(source).unwrap().iter().enumerate().filter(|(_, a)| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_))).nth(ordinal).unwrap().0;
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::ActivateAbility { source: id, ability_index } | LegalAction::ActivateManaAbility { source: id, ability_index } if *id == source && *ability_index == index)).expect("real activation is legal");
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(3);
    let progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    (queue, state, progress)
}
fn finish(game: &mut GameState, queue: &mut TriggerQueue, state: &mut PriorityLoopState, mut progress: GameProgress, dm: &mut Choices) {
    for _ in 0..64 {
        if state.pending_activation.is_none() && state.pending_mana_ability.is_none() { return; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        *game = game.clone(); *state = state.clone();
        progress = apply_decision_context_with_dm(game, queue, state, &ctx, dm).unwrap();
    }
    panic!("activation did not finish");
}
fn activate(game: &mut GameState, source: ObjectId, ordinal: usize, dm: &mut Choices) {
    let (mut queue, mut state, progress) = begin(game, source, ordinal, dm); finish(game, &mut queue, &mut state, progress, dm);
}
fn tokens(game: &GameState, player: PlayerId, subtype: Subtype) -> Vec<ObjectId> {
    game.battlefield.iter().copied().filter(|id| game.object(*id).is_some_and(|o| o.kind == ObjectKind::Token)
        && game.current_controller(*id) == Some(player) && game.calculated_subtypes(*id).contains(&subtype)).collect()
}
fn receipt(game: &GameState, amount: u32, x: Option<u32>) {
    let entry = game.stack.last().unwrap(); assert_eq!(entry.x_value, x);
    assert_eq!(entry.effect_outcomes[&EffectId::ACTIVATION_COUNTER_COST].instruction_result().count_or_zero(), i64::from(amount));
}
#[test]
fn ant_man_full_draw_trigger_and_paid_tokens_survive_native_recovery_and_source_departure() {
    for definition in definitions("The Astonishing Ant-Man") { for quantity in [0, 2] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source); library(&mut game, 4);
        let mut dm = Choices { quantity, ..Default::default() }; apply(&mut game, source, Effect::draw(3), &mut dm);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 3);
        mana(&mut game, ManaSymbol::Green, 1); mana(&mut game, ManaSymbol::Colorless, 2);
        activate(&mut game, source, 0, &mut dm); receipt(&game, quantity, None);
        assert!(game.is_tapped(source)); assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 3 - quantity);
        let mut recovered = game.clone(); game.add_counters(source, CounterType::PlusOnePlusOne, 9);
        execute_effect(&mut recovered, &Effect::move_to_zone(ChooseSpec::SpecificObject(source), Zone::Graveyard, false), &mut EffectContext::new(source, A, &mut dm)).unwrap();
        for state in [&mut game, &mut recovered] { resolve_stack_entry_with(state, &mut dm).unwrap();
            let insects = tokens(state, A, Subtype::Insect); assert_eq!(insects.len(), quantity as usize);
            for id in insects { let c = state.current_characteristics(id).unwrap(); assert_eq!((c.power, c.toughness), (Some(1), Some(1))); assert_eq!(c.colors, ironsmith::color::ColorSet::GREEN); }
        }
    }}
}
#[test]
fn rasputin_full_entry_mana_and_knight_bodies_use_the_actual_dream_cost() {
    for definition in definitions("Rasputin, the Oneiromancer") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(source).unwrap().stable_id; let mut dm = Choices { quantity: 1, ..Default::default() };
        apply(&mut game, source, Effect::move_to_zone(ChooseSpec::SpecificObject(source), Zone::Battlefield, false), &mut dm);
        let source = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(game.counter_count(source, CounterType::Dream), 2);
        for player in [B, C] { assert_eq!(tokens(&game, player, Subtype::Goblin).len(), 1); }
        game.remove_summoning_sickness(source); activate(&mut game, source, 0, &mut dm);
        assert!(game.stack.is_empty()); assert_eq!(game.player(A).unwrap().mana_pool.total(), 1); assert!(dm.numbers.contains(&1));
        game.untap(source); activate(&mut game, source, 1, &mut dm); receipt(&game, 1, None); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let knight = tokens(&game, A, Subtype::Knight)[0]; let c = game.current_characteristics(knight).unwrap();
        assert_eq!((c.power,c.toughness),(Some(2),Some(2))); assert_eq!(c.colors, ironsmith::color::ColorSet::WHITE);
        let red = compile_to_runtime_definition("Red witness", "Type: Creature — Goblin\nColor indicator: Red\nPower/Toughness: 1/1", false).unwrap();
        let red = game.create_object_from_definition(&red, B, Zone::Battlefield); apply(&mut game, red, Effect::deal_damage(1, ChooseSpec::SpecificObject(knight)), &mut dm); assert_eq!(game.damage_on(knight), 0);
        game.untap(source); let other = creature(&mut game,A,Zone::Battlefield,"Other dream holder",2,3); game.add_counters(other,CounterType::Dream,9);
        assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|a| matches!(a,LegalAction::ActivateAbility {source:id,..}|LegalAction::ActivateManaAbility {source:id,..} if *id==source)));
    }
}
#[test]
fn jar_full_death_trigger_paid_look_pool_and_bottom_order_keep_unlooked_cards() {
    for definition in definitions("Jar of Eyeballs") { for amount in [0,4] {
        let mut game=game(); let source=game.create_object_from_definition(&definition,A,Zone::Battlefield); let mut dm=Choices::default();
        let foreign=creature(&mut game,B,Zone::Battlefield,"Foreign creature",2,3); apply(&mut game,source,Effect::destroy(ChooseSpec::SpecificObject(foreign)),&mut dm); assert_eq!(game.counter_count(source,CounterType::Eyeball),0);
        if amount==4 { for name in ["First death","Second death"] { let own=creature(&mut game,A,Zone::Battlefield,name,2,3);apply(&mut game,source,Effect::destroy(ChooseSpec::SpecificObject(own)),&mut dm); }}
        assert_eq!(game.counter_count(source,CounterType::Eyeball),amount); let original=library(&mut game,6);let chosen_stable=game.object(original[4]).unwrap().stable_id;
        let before=game.player(A).unwrap().library.clone(); dm.pick=Some(original[4]); mana(&mut game,ManaSymbol::Colorless,3);
        activate(&mut game,source,0,&mut dm);receipt(&game,amount,None);assert_eq!(game.counter_count(source,CounterType::Eyeball),0);
        game=game.clone();resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
        if amount==0 { assert_eq!(game.player(A).unwrap().library,before);assert!(game.player(A).unwrap().hand.is_empty()); }
        else { assert_eq!(game.player(A).unwrap().hand.len(),1);assert_eq!(game.object(game.player(A).unwrap().hand[0]).unwrap().stable_id,chosen_stable); assert_eq!(dm.ordered.len(),3);
            let after=&game.player(A).unwrap().library; assert_eq!(&after[3..],&before[..2]);assert_eq!(&after[..3],dm.ordered.as_slice());
            assert!(dm.ordered.iter().all(|id| before[2..].contains(id) && *id!=original[4]));
        }
    }}
}
#[test]
fn independent_mana_x_does_not_replace_the_paid_counter_quantity() {
    let text="Type: Artifact\n{X}, {T}, Remove any number of charge counters from this artifact: Create that many 1/1 green Insect creature tokens. Draw X cards.";
    for definition in definitions_text("Independent quantities",text) { let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.add_counters(source,CounterType::Charge,5);library(&mut game,6);mana(&mut game,ManaSymbol::Colorless,3);
        let mut dm=Choices {quantity:2,announced_x:3,..Default::default()};activate(&mut game,source,0,&mut dm);receipt(&game,2,Some(3));assert_eq!(game.counter_count(source,CounterType::Charge),3);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(tokens(&game,A,Subtype::Insect).len(),2);assert_eq!(game.player(A).unwrap().hand.len(),3);
    }
}

#[test]
fn hankyu_both_grants_keep_exact_equipment_counters_recipient_tap_and_lifelink_source() {
    for definition in definitions("Hankyu") { for lifetime in ["present", "grantor left", "reattached", "host left", "target left"] {
        let mut game = game();
        let host_definition = compile_to_runtime_definition("Lifelink host", "Type: Creature — Soldier\nPower/Toughness: 2/3\nLifelink", false).unwrap();
        let host = game.create_object_from_definition(&host_definition,A,Zone::Battlefield); game.remove_summoning_sickness(host);
        let other_host = creature(&mut game,A,Zone::Battlefield,"Second host",2,3);
        let foreign = creature(&mut game,B,Zone::Battlefield,"Foreign controller",2,3);
        let first = game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let second = game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let mut dm = Choices {target:Some(Target::Object(host)),..Default::default()};
        mana(&mut game,ManaSymbol::Colorless,8);
        for equipment in [first,second] {activate(&mut game,equipment,0,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.object(equipment).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(host)));}
        game.add_counters(host,CounterType::Aim,9);game.add_counters(first,CounterType::Arrow,5);game.add_counters(second,CounterType::Aim,7);
        dm.target=None;
        for expected in [1,2] {activate(&mut game,host,0,&mut dm);assert!(game.is_tapped(host));resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.counter_count(first,CounterType::Aim),expected);assert_eq!(game.counter_count(second,CounterType::Aim),7);game.untap(host);}
        apply(&mut game,foreign,Effect::gain_control_with_duration(ChooseSpec::SpecificObject(first),ironsmith::effect::Until::Forever),&mut dm);
        assert_eq!(game.current_controller(first),Some(B));
        let target=if lifetime=="target left" {Target::Object(foreign)} else {Target::Player(B)};
        dm.target=Some(target);activate(&mut game,host,1,&mut dm);receipt(&game,2,None);
        assert!(game.is_tapped(host));assert_eq!(game.counter_count(first,CounterType::Aim),0);assert_eq!(game.counter_count(first,CounterType::Arrow),5);assert_eq!(game.counter_count(second,CounterType::Aim),7);assert_eq!(game.counter_count(host,CounterType::Aim),9);
        match lifetime {
            "grantor left" => {execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(first),Zone::Graveyard,false),&mut EffectContext::new(host,A,&mut dm)).unwrap();}
            "host left" => {execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(host),Zone::Graveyard,false),&mut EffectContext::new(host,A,&mut dm)).unwrap();}
            "reattached" => {assert!(game.attach_object_to_target(first,ironsmith::object::AttachmentTarget::Object(other_host)));}
            "target left" => {execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(foreign),Zone::Graveyard,false),&mut EffectContext::new(host,A,&mut dm)).unwrap();}
            _ => {}
        }
        game=game.clone();resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().life,if lifetime=="target left" {20} else {22});
        assert_eq!(game.player(B).unwrap().life,if lifetime=="target left" {20} else {18});
    }}
}
#[test]
fn hankyu_zero_removal_is_a_completed_cost_receipt() {
    for definition in definitions("Hankyu") {
        let mut game=game();let host=creature(&mut game,A,Zone::Battlefield,"Empty bow host",2,3);game.remove_summoning_sickness(host);
        let equipment=game.create_object_from_definition(&definition,A,Zone::Battlefield);mana(&mut game,ManaSymbol::Colorless,4);
        let mut dm=Choices {target:Some(Target::Object(host)),..Default::default()};activate(&mut game,equipment,0,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        dm.target=Some(Target::Player(B));activate(&mut game,host,1,&mut dm);receipt(&game,0,None);assert!(game.is_tapped(host));resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(B).unwrap().life,20);
    }
}

#[test]
fn simic_full_evolve_body_checks_either_characteristic_your_entries_and_resolution() {
    for definition in definitions("Simic Manipulator") {
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let mut dm=Choices::default();
        for (owner,power,toughness,expected) in [(B,8,8,0),(A,1,1,1),(A,1,3,2),(A,1,1,2)] {
            let entering=creature(&mut game,owner,Zone::Hand,"Evolve entrant",power,toughness);
            apply(&mut game,source,Effect::move_to_zone(ChooseSpec::SpecificObject(entering),Zone::Battlefield,false),&mut dm);
            assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),expected);
        }
        let entrant=creature(&mut game,A,Zone::Hand,"Intervening evolve entrant",3,4);
        let outcome=execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(entrant),Zone::Battlefield,false),&mut EffectContext::new(source,A,&mut dm)).unwrap();
        let mut queue=TriggerQueue::new();for event in outcome.events {for entry in check_triggers(&game,&event) {queue.add(entry);}}
        put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert!(!game.stack.is_empty());
        game.add_counters(source,CounterType::PlusOnePlusOne,5);resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),7,"the intervening comparison must be rechecked");
    }
}
#[test]
fn simic_declares_before_targets_locks_one_payment_and_keeps_actual_result_after_departure() {
    use ironsmith::decisions::context::DecisionContext;
    for definition in definitions("Simic Manipulator") {
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.remove_summoning_sickness(source);game.add_counters(source,CounterType::PlusOnePlusOne,4);
        let target=creature(&mut game,B,Zone::Battlefield,"Power two",2,3);let too_large=creature(&mut game,B,Zone::Battlefield,"Power three",3,3);
        let mut dm=Choices {quantity:2,target:Some(Target::Object(target)),..Default::default()};let (mut queue,mut state,first)=begin(&mut game,source,0,&mut dm);
        let GameProgress::NeedsDecisionCtx(DecisionContext::Number(number))=first else {panic!("declaration must precede targets");};
        assert_eq!((number.min,number.max,number.is_x_value),(1,4,false));assert!(!game.is_tapped(source));assert!(state.pending_activation.as_ref().unwrap().effect_outcomes.is_empty());
        state=state.clone();let progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&DecisionContext::Number(number),&mut dm).unwrap();
        let GameProgress::NeedsDecisionCtx(DecisionContext::Targets(ref choices))=progress else {panic!("targets must follow declaration");};
        assert!(choices.requirements[0].legal_targets.contains(&Target::Object(target)));assert!(!choices.requirements[0].legal_targets.contains(&Target::Object(too_large)));
        let pending=state.pending_activation.as_ref().unwrap();assert_eq!(pending.counter_removal_declaration.unwrap().amount,2);assert!(pending.effect_outcomes.is_empty());assert_eq!(pending.x_value,None);
        game.add_counters(source,CounterType::PlusOnePlusOne,3);finish(&mut game,&mut queue,&mut state,progress,&mut dm);
        assert_eq!(dm.numbers,vec![1],"payment cannot ask a second quantity");receipt(&game,2,None);assert!(game.is_tapped(source));assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),5);
        let mut fizzled=game.clone();fizzled.add_counters(target,CounterType::PlusOnePlusOne,1);resolve_stack_entry_with(&mut fizzled,&mut dm).unwrap();assert_eq!(fizzled.current_controller(target),Some(B));
        execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(source),Zone::Graveyard,false),&mut EffectContext::new(source,A,&mut dm)).unwrap();game=game.clone();resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.current_controller(target),Some(A));
    }
}
#[test]
fn simic_replacement_modified_costs_stay_paid_and_actual_counts_control_resolution() {
    use ironsmith::replacement::{EventModification,ReplacementAction,ReplacementEffect};
    for definition in definitions("Simic Manipulator") {for scenario in 0..5 {
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.remove_summoning_sickness(source);game.add_counters(source,CounterType::PlusOnePlusOne,4);
        let power=if scenario==4 {0} else {2};let target=creature(&mut game,B,Zone::Battlefield,"Replacement target",power,3);
        let (action,actual,life)=match scenario {0=>(ReplacementAction::Modify(EventModification::Subtract(1)),1,20),1=>(ReplacementAction::Modify(EventModification::Add(1)),3,20),2=>(ReplacementAction::Prevent,0,20),_=>(ReplacementAction::Instead(vec![Effect::gain_life(2)]),0,22)};
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source,A,ironsmith::events::counters::matchers::WouldRemoveCountersMatcher::any(),action));
        let mut dm=Choices {quantity:2,target:Some(Target::Object(target)),..Default::default()};let (mut queue,mut state,first)=begin(&mut game,source,0,&mut dm);
        let GameProgress::NeedsDecisionCtx(context)=first else {panic!("declaration prompt");};let progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&context,&mut dm).unwrap();
        assert_eq!(state.pending_activation.as_ref().unwrap().counter_removal_declaration.unwrap().amount,2);assert!(state.pending_activation.as_ref().unwrap().effect_outcomes.is_empty());
        finish(&mut game,&mut queue,&mut state,progress,&mut dm);assert!(!state.has_pending_action());assert_eq!(dm.numbers,vec![1]);assert!(game.is_tapped(source));assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),4-actual);assert_eq!(game.player(A).unwrap().life,life);assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());receipt(&game,actual,None);
        game=game.clone();resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.current_controller(target),Some(if power as u32<=actual {A} else {B}),"CR 118.11 completes the modified payment; resolution reads its actual removal count");
    }}
}
#[test]
fn simic_stale_targets_current_shortages_and_cancellation_restore_the_announcement() {
    for definition in definitions("Simic Manipulator") {for failure in ["shortage","target grew","cancel"] {
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.remove_summoning_sickness(source);game.add_counters(source,CounterType::PlusOnePlusOne,3);let target=creature(&mut game,B,Zone::Battlefield,"Chosen target",2,3);
        let mut dm=Choices {quantity:2,target:Some(Target::Object(target)),..Default::default()};let (mut queue,mut state,first)=begin(&mut game,source,0,&mut dm);let GameProgress::NeedsDecisionCtx(context)=first else {panic!("declaration prompt");};
        let progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&context,&mut dm).unwrap();assert!(matches!(progress,GameProgress::NeedsDecisionCtx(ironsmith::decisions::context::DecisionContext::Targets(_))));
        match failure {"shortage"=>{game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne,1);},"target grew"=>{game.add_counters(target,CounterType::PlusOnePlusOne,1);},_=>{}}
        let selected=if failure=="cancel" {vec![]} else {vec![Target::Object(target)]};assert!(apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::Targets(selected),&mut dm).is_err());
        assert!(!state.has_pending_action());assert!(state.checkpoint.is_none());assert!(game.stack.is_empty());assert!(!game.is_tapped(source));assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),3);assert_eq!(game.object(source).unwrap().x_value,None);assert_eq!(game.current_controller(target),Some(B));
    }}
}
#[test]
fn simic_pending_invalid_and_duplicate_numeric_answers_cannot_publish_or_replace_a_declaration() {
    struct Suspended;impl DecisionMaker for Suspended {fn awaiting_choice(&self)->bool {true}}
    for definition in definitions("Simic Manipulator") {
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.remove_summoning_sickness(source);game.add_counters(source,CounterType::PlusOnePlusOne,3);let target=creature(&mut game,B,Zone::Battlefield,"Pending target",2,3);
        let mut dm=Choices {quantity:2,target:Some(Target::Object(target)),..Default::default()};let (mut queue,mut state,_)=begin(&mut game,source,0,&mut dm);let mut pending=Suspended;
        assert!(matches!(apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::NumberChoice(0),&mut pending).unwrap(),GameProgress::Continue));assert!(state.pending_activation.as_ref().unwrap().counter_removal_declaration.is_none());
        assert!(apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::NumberChoice(4),&mut dm).is_err());assert!(state.pending_activation.as_ref().unwrap().counter_removal_declaration.is_none());
        let progress=apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::NumberChoice(2),&mut dm).unwrap();assert!(matches!(apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::Targets(vec![]),&mut pending).unwrap(),GameProgress::Continue));
        assert!(apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::NumberChoice(3),&mut dm).is_err());let saved=state.pending_activation.as_ref().unwrap();assert_eq!(saved.counter_removal_declaration.unwrap().amount,2);assert_eq!(saved.x_value,None);assert!(saved.effect_outcomes.is_empty());assert!(saved.chosen_targets.is_empty());
        finish(&mut game,&mut queue,&mut state,progress,&mut dm);receipt(&game,2,None);assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),1);
    }
}
#[test]
fn prospective_counter_feasibility_uses_real_resources_and_has_no_side_effects() {
    for definition in definitions("Simic Manipulator") {
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.remove_summoning_sickness(source);let other=creature(&mut game,B,Zone::Battlefield,"Foreign rich source",2,3);game.add_counters(other,CounterType::PlusOnePlusOne,20);
        let offered=|game:&GameState|compute_legal_actions(game,A).unwrap().iter().any(|a|matches!(a,LegalAction::ActivateAbility {source:id,..} if *id==source));assert!(!offered(&game));game.add_counters(source,CounterType::PlusOnePlusOne,1);assert!(offered(&game));assert!(offered(&game));assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),1);assert_eq!(game.object(source).unwrap().x_value,None);assert!(!game.is_tapped(source));assert!(game.stack.is_empty());game.tap(source);assert!(!offered(&game));
    }
    for definition in definitions_text("Prospective feasibility","Type: Artifact\n{2}, {T}, Remove one or more charge counters from this artifact: Gain control of target creature with power less than or equal to the number of charge counters removed this way.") {
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let _target=creature(&mut game,B,Zone::Battlefield,"Power two",2,3);game.add_counters(source,CounterType::Charge,1);mana(&mut game,ManaSymbol::Colorless,2);
        let offered=|game:&GameState|compute_legal_actions(game,A).unwrap().iter().any(|a|matches!(a,LegalAction::ActivateAbility {source:id,..} if *id==source));assert!(!offered(&game));game.add_counters(source,CounterType::Charge,1);assert!(offered(&game));game.player_mut(A).unwrap().mana_pool.empty();assert!(!offered(&game));assert_eq!(game.counter_count(source,CounterType::Charge),2);
    }
}
#[test]
fn prospective_counter_cost_preserves_independent_mana_x_and_target_repricing() {
    for (name,text,x,available) in [
        ("Independent prospective quantities","Type: Artifact\n{X}, {T}, Remove one or more charge counters from this artifact: Gain control of target creature with power less than or equal to the number of charge counters removed this way. Draw X cards.",Some(3),3),
        ("Target-priced prospective quantity","Type: Artifact\n{3}, {T}, Remove one or more charge counters from this artifact: Gain control of target creature with power less than or equal to the number of charge counters removed this way. This ability costs {2} less to activate if it targets a colorless creature.",None,1)] {
        for definition in definitions_text(name,text) {
            let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.add_counters(source,CounterType::Charge,5);let target=creature(&mut game,B,Zone::Battlefield,"Prospective target",2,3);library(&mut game,5);mana(&mut game,ManaSymbol::Colorless,available);
            let mut dm=Choices {quantity:2,announced_x:x.unwrap_or(0),target:Some(Target::Object(target)),..Default::default()};activate(&mut game,source,0,&mut dm);receipt(&game,2,x);assert_eq!(dm.numbers,vec![1]);assert_eq!(game.counter_count(source,CounterType::Charge),3);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.current_controller(target),Some(A));assert_eq!(game.player(A).unwrap().hand.len(),x.unwrap_or(0) as usize);
        }
    }
}
#[test]
fn declaration_without_targets_rolls_back_and_unsupported_producer_contracts_fail_closed() {
    for definition in definitions_text("No declared target","Type: Artifact\n{T}, Remove one or more charge counters from this artifact: Gain control of target creature with power less than or equal to the number of charge counters removed this way.") {
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.add_counters(source,CounterType::Charge,3);let target=creature(&mut game,B,Zone::Battlefield,"Power two",2,3);let mut dm=Choices {quantity:2,target:Some(Target::Object(target)),..Default::default()};let (mut queue,mut state,_)=begin(&mut game,source,0,&mut dm);
        assert!(apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::NumberChoice(1),&mut dm).is_err());assert!(!state.has_pending_action());assert!(!game.is_tapped(source));assert_eq!(game.counter_count(source,CounterType::Charge),3);activate(&mut game,source,0,&mut dm);receipt(&game,2,None);
    }
    for text in [
        "Remove all charge counters from this artifact: Gain control of target creature with power less than or equal to the number of charge counters removed this way.",
        "Remove one or more charge counters from this artifact: Gain control of target creature with power equal to the number of charge counters removed this way.",
        "Remove one or more charge counters from this artifact: Gain control of target creature with power less than or equal to the number of dream counters removed this way.",
        "Remove one or more charge counters from among permanents you control: Gain control of target creature with power less than or equal to the number of charge counters removed this way.",
        "Remove all charge counters from this artifact, Remove all dream counters from this artifact: Create that many 1/1 green Insect creature tokens."] {
        assert!(compile_to_artifact("Unsupported payment contract",&format!("Type: Artifact\n{text}"),false).is_err());
    }
}
