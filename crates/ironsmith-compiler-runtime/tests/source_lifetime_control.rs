//! UNVALIDATED source-lifetime control; scenarios are authored, unrun.
#![allow(dead_code)]
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/source_lifetime_control.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|r| r["name"] == name).unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().into());
    definitions_text(name, &lines.join("\n"))
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    game
}
fn vanilla(name: &str, cost: &str, subtype: &str, p: i32, t: i32) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!("Mana cost: {cost}\nType: Creature — {subtype}\nPower/Toughness: {p}/{t}"),
        false,
    )
    .unwrap()
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    objects_explicit: bool,
    x: u32,
    decline: bool,
    land_choice: Option<&'static str>,
    land_prompts: usize,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description == "Choose a basic land type" {
            self.land_prompts += 1;
            return vec![ctx.options.iter().find(|option|option.description==self.land_choice.unwrap_or("Island")).unwrap().index];
        }
        if ctx.description.starts_with("Choose optional costs") {
            if self.decline {
                return vec![];
            }
            return ctx
                .options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            assert!(self.x <= ctx.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, ctx)
        }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if !self.targets.is_empty() {
            assert_eq!(context.requirements.len(), self.targets.len());
            for (requirement, target) in context.requirements.iter().zip(&self.targets) {
                assert!(requirement.legal_targets.contains(target));
            }
            self.targets.clone()
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if self.objects_explicit || !self.objects.is_empty() {
            for id in &self.objects {
                assert!(
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                );
            }
            self.objects.clone()
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) -> EffectOutcome {
    let mut dm = SelectFirstDecisionMaker;
    let controller = game.current_controller(source).unwrap_or(A);
    execute_effect(
        game,
        &effect,
        &mut EffectContext::new(source, controller, &mut dm),
    )
    .unwrap()
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn resolve_all(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..30 {
        if game.stack_is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("unexpected continuing trigger chain");
}
fn resource(name: &str, types: &str) -> CardDefinition {
    let pt = if types.contains("Creature") {
        "\nPower/Toughness: 1/1"
    } else {
        ""
    };
    compile_to_runtime_definition(name, format!("Mana cost: {{1}}\nType: {types}{pt}"), false)
        .unwrap()
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for event in outcome.events {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn pt(game: &GameState, id: ObjectId) -> (i32, i32) {
    (
        game.current_power(id).unwrap(),
        game.current_toughness(id).unwrap(),
    )
}
fn pump(game: &mut GameState, source: ObjectId, id: ObjectId, p: i32, t: i32) {
    apply(
        game,
        source,
        Effect::pump(p, t, ChooseSpec::SpecificObject(id), Until::EndOfTurn),
    );
}
fn counter(game: &mut GameState, source: ObjectId, id: ObjectId, count: i32) {
    apply(
        game,
        source,
        Effect::put_counters(
            ironsmith::object::CounterType::PlusOnePlusOne,
            count,
            ChooseSpec::SpecificObject(id),
        ),
    );
}
fn cast(
    game: &mut GameState,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none());
    let spell = game
        .stack
        .iter()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}


fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    let action = LegalAction::ActivateAbility { source, ability_index };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..60 {
        if state.pending_activation.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
}
fn activated_at(definition: &CardDefinition, position: usize) -> usize {
    definition.abilities.iter().enumerate().filter_map(|(i,a)|
        matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)).nth(position).unwrap()
}
fn current_activated_at(game: &GameState, source: ObjectId, position: usize) -> usize {
    game.calculated_characteristics(source).unwrap().abilities.iter().enumerate().filter_map(|(i,a)|
        matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)).nth(position).unwrap()
}
fn has(game: &GameState, id: ObjectId, ability: ironsmith::static_abilities::StaticAbilityId) -> bool {
    game.current_has_static_ability_id(id, ability)
}
fn event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) { queue.add(trigger); }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn lore(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let outcome = apply(game, source, Effect::put_counters(ironsmith::object::CounterType::Lore, 1, ChooseSpec::SpecificObject(source)));
    queue_outcome(game, outcome, dm); resolve_all(game, &mut Choices::default());
}

fn attach(game: &mut GameState, aura: ObjectId, host: ObjectId) {
    assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
}

fn flush(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
        if game.stack_is_empty() { return; }
        resolve(game, dm);
    }
    panic!("control trigger did not settle");
}
fn named(game: &GameState, name: &str) -> ObjectId {
    *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == name).unwrap()
}
fn enter(game: &mut GameState, definition: &CardDefinition, owner: PlayerId, dm: &mut Choices) -> ObjectId {
    let old=game.create_object_from_definition(definition,owner,Zone::Hand);
    let receipt=game.move_object_with_etb_processing_with_dm(old,Zone::Battlefield,dm).unwrap();
    assert!(!receipt.pending);assert!(receipt.programs.is_empty(),"do not discard entry additions");
    receipt.original.into_result().unwrap().new_id
}
#[test]
fn five_frozen_complete_bodies_are_strict_and_round_trip() {
    let rows=fixtures().into_iter().filter(|row|row["proposed_complete"]==true).collect::<Vec<_>>();
    assert_eq!(rows.len(),5);
    for row in rows { for definition in definitions(row["name"].as_str().unwrap()) {
        assert_eq!(definition.card.name,row["name"]);
        assert!(format!("{definition:?}").contains("ObjectOnBattlefield"));
    }}
}
#[test]
fn sower_source_control_changes_do_not_change_the_beneficiary_and_phasing_latches_expiry() {
    for definition in definitions("Sower of Temptation") {
        let mut game=game();
        let victim=game.create_object_from_definition(&vanilla("Victim","{2}","Bear",2,2),B,Zone::Battlefield);
        let mut dm=Choices{targets:vec![Target::Object(victim)],..Default::default()};
        cast(&mut game,&definition,CastingMethod::Normal,&mut dm);flush(&mut game,&mut dm);
        let sower=named(&game,"Sower of Temptation");
        assert_eq!(game.current_controller(victim),Some(A));
        assert!(has(&game,sower,ironsmith::static_abilities::StaticAbilityId::Flying));
        let mut first=SelectFirstDecisionMaker;
        execute_effect(&mut game,&Effect::new(ironsmith::effects::GainControlEffect::permanent(ChooseSpec::SpecificObject(sower))),
            &mut EffectContext::new(sower,C,&mut first)).unwrap();
        assert_eq!(game.current_controller(sower),Some(C));
        assert_eq!(game.current_controller(victim),Some(A));
        game.phase_out(sower);assert_eq!(game.current_controller(victim),Some(B));
        game.phase_in(sower);assert_eq!(game.current_controller(victim),Some(B),"expired duration never restarts");
    }
}
#[test]
fn departed_or_blinked_sower_before_resolution_never_starts_the_duration() {
    for definition in definitions("Sower of Temptation") { for blink in [false,true] {
        let mut game=game();
        let victim=game.create_object_from_definition(&vanilla("Victim","{2}","Bear",2,2),B,Zone::Battlefield);
        let mut dm=Choices{targets:vec![Target::Object(victim)],..Default::default()};
        cast(&mut game,&definition,CastingMethod::Normal,&mut dm);resolve(&mut game,&mut dm);
        put_triggers_on_stack_with_dm(&mut game,&mut TriggerQueue::new(),&mut dm).unwrap();
        assert_eq!(game.stack.len(),1);
        let sower=named(&game,"Sower of Temptation");
        let departed=game.move_object_by_effect(sower,Zone::Exile).unwrap();
        if blink { let replacement=game.move_object(departed,Zone::Battlefield).unwrap();assert_ne!(replacement,sower); }
        resolve(&mut game,&mut dm);
        assert_eq!(game.current_controller(victim),Some(B));
    }}
}
#[test]
fn giants_grasp_enchant_restriction_and_lifetime_are_owned_by_the_aura() {
    for definition in definitions("Giant's Grasp") {
        let mut game=game();
        let host=game.create_object_from_definition(&vanilla("Giant host","{3}","Giant",3,3),A,Zone::Battlefield);
        let victim=game.create_object_from_definition(&resource("Enemy relic","Artifact"),B,Zone::Battlefield);
        let mut dm=Choices{targets:vec![Target::Object(host)],..Default::default()};
        cast(&mut game,&definition,CastingMethod::Normal,&mut dm);resolve(&mut game,&mut dm);
        let aura=named(&game,"Giant's Grasp");
        assert_eq!(game.object(aura).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(host)));
        dm.targets=vec![Target::Object(victim)];flush(&mut game,&mut dm);
        assert_eq!(game.current_controller(victim),Some(A));
        game.phase_out(host);assert!(game.is_phased_out(aura));
        assert_eq!(game.current_controller(victim),Some(B));
        game.phase_in(host);assert_eq!(game.current_controller(victim),Some(B));
    }
}
#[test]
fn charisma_uses_the_damaged_creature_and_original_aura_even_after_reattachment() {
    for definition in definitions("Charisma") {
        let mut game=game();
        let host=game.create_object_from_definition(&vanilla("Enchanted dealer","{2}","Wizard",2,4),A,Zone::Battlefield);
        let next=game.create_object_from_definition(&vanilla("New host","{2}","Wizard",2,4),A,Zone::Battlefield);
        let victim=game.create_object_from_definition(&vanilla("Damaged victim","{2}","Bear",2,4),B,Zone::Battlefield);
        let mut dm=Choices{targets:vec![Target::Object(host)],..Default::default()};
        cast(&mut game,&definition,CastingMethod::Normal,&mut dm);flush(&mut game,&mut dm);
        let aura=named(&game,"Charisma");
        let outcome=apply(&mut game,host,Effect::deal_damage(1,ChooseSpec::SpecificObject(victim)));
        dm.targets.clear();queue_outcome(&mut game,outcome,&mut dm);flush(&mut game,&mut dm);
        assert_eq!(game.current_controller(victim),Some(A));
        attach(&mut game,aura,next);assert_eq!(game.current_controller(victim),Some(A));
        game.move_object_by_effect(aura,Zone::Graveyard).unwrap();
        assert_eq!(game.current_controller(victim),Some(B));
    }
}
#[test]
fn cytoplast_graft_entry_and_paid_activation_keep_the_counter_target_requirement() {
    use ironsmith::object::CounterType;
    for definition in definitions("Cytoplast Manipulator") {
        let mut game=game();let mut dm=Choices::default();
        cast(&mut game,&definition,CastingMethod::Normal,&mut dm);flush(&mut game,&mut dm);
        let source=named(&game,"Cytoplast Manipulator");
        assert_eq!(game.object(source).unwrap().counters.get(&CounterType::PlusOnePlusOne),Some(&2));
        let victim=enter(&mut game,&vanilla("Grafted victim","{2}","Bear",2,2),B,&mut dm);flush(&mut game,&mut dm);
        assert_eq!(game.object(victim).unwrap().counters.get(&CounterType::PlusOnePlusOne),Some(&1));
        assert_eq!(game.object(source).unwrap().counters.get(&CounterType::PlusOnePlusOne),Some(&1));
        game.remove_summoning_sickness(source);dm.targets=vec![Target::Object(victim)];
        let index=activated_at(&definition,0);let mana=game.player(A).unwrap().mana_pool.total();
        activate(&mut game,source,index,&mut dm);assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.total(),mana-1);resolve(&mut game,&mut dm);
        assert_eq!(game.current_controller(victim),Some(A));
        game.remove_counters(victim,CounterType::PlusOnePlusOne,1,Some(source),Some(A));
        assert_eq!(game.current_controller(victim),Some(A),"the counter is a target restriction, not the duration");
        game.move_object_by_effect(source,Zone::Graveyard).unwrap();assert_eq!(game.current_controller(victim),Some(B));
    }
}
#[test]
fn scarwood_opponent_payment_prevents_control_and_nonpayment_preserves_exact_source_duration() {
    for definition in definitions("Scarwood Bandits") { for pay in [false,true] {
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let victim=game.create_object_from_definition(&resource("Contested artifact","Artifact"),B,Zone::Battlefield);
        if pay { game.player_mut(B).unwrap().mana_pool.add(ManaSymbol::Colorless,2); }
        let mut dm=Choices{targets:vec![Target::Object(victim)],..Default::default()};
        let mana=game.player(A).unwrap().mana_pool.total();
        activate(&mut game,source,activated_at(&definition,0),&mut dm);
        assert!(game.is_tapped(source));assert_eq!(game.player(A).unwrap().mana_pool.total(),mana-3);
        resolve(&mut game,&mut dm);
        assert_eq!(game.current_controller(victim),Some(if pay { B } else { A }));
        if pay { assert_eq!(game.player(B).unwrap().mana_pool.total(),0); }
        game.phase_out(source);assert_eq!(game.current_controller(victim),Some(B));
        game.phase_in(source);assert_eq!(game.current_controller(victim),Some(B));
    }}
}

#[test]
fn common_grant_and_leading_duration_paths_expire_on_phase_out_but_literal_until_leaves_does_not() {
    for text in [
        "Mana cost: {1}\nType: Artifact\n{T}: Target creature gains flying for as long as this artifact remains on the battlefield.",
        "Mana cost: {1}\nType: Artifact\n{T}: For as long as this artifact remains on the battlefield, target creature gains flying.",
    ] { for definition in definitions_text("Duration source",text) {
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let victim=game.create_object_from_definition(&vanilla("Grant recipient","{2}","Bear",2,2),B,Zone::Battlefield);
        let mut dm=Choices{targets:vec![Target::Object(victim)],..Default::default()};
        activate(&mut game,source,0,&mut dm);resolve(&mut game,&mut dm);
        assert!(has(&game,victim,ironsmith::static_abilities::StaticAbilityId::Flying));
        game.phase_out(source);assert!(!has(&game,victim,ironsmith::static_abilities::StaticAbilityId::Flying));
        game.phase_in(source);assert!(!has(&game,victim,ironsmith::static_abilities::StaticAbilityId::Flying));
        apply(&mut game,source,Effect::new(ironsmith::effects::GainControlEffect::new(
            ChooseSpec::SpecificObject(victim),Until::ThisLeavesTheBattlefield)));
        assert_eq!(game.current_controller(victim),Some(A));
        game.phase_out(source);assert_eq!(game.current_controller(victim),Some(A),"literal until-leaves is not a visible-state predicate");
        game.phase_in(source);game.move_object_by_effect(source,Zone::Graveyard).unwrap();
        assert_eq!(game.current_controller(victim),Some(B));
    }}
}
