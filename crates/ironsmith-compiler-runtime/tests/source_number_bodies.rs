//! UNVALIDATED: independent direct/artifact whole-card scenarios. Do not infer
//! runtime coverage until the campaign's separately authorized execution gate.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, NumberContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::{EffectContext, EffectExecutor, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/source_number_bodies.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) { text.push_str(&format!("Power/Toughness: {p}/{t}\n")); }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap(); assert_eq!(artifact, restored);
    let decoded = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}
fn simple(name: &str, types: &str, cost: &str, pt: Option<(i32, i32)>, body: &str) -> CardDefinition {
    let pt = pt.map(|(p,t)| format!("\nPower/Toughness: {p}/{t}")).unwrap_or_default();
    compile_to_runtime_definition(name, format!("Mana cost: {cost}\nType: {types}{pt}\n{body}"), false).unwrap()
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain; game.turn.active_player = A; game.turn.priority_player = Some(A);
    for player in [A,B,C] {
        for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(color, 30);
        }
        for _ in 0..8 { game.create_object_from_definition(&simple("Draw card", "Artifact", "{1}", None, ""), player, Zone::Library); }
    }
    game
}
#[derive(Default)]
struct Choices { number: u32, target: Option<PlayerId>, decline: bool, defer_number: bool, waiting: bool, prompts: Vec<(PlayerId,u32,Option<u32>)> }
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        self.prompts.push((ctx.player, ctx.min, ctx.authored_max)); self.waiting=self.defer_number; self.number
    }
    fn awaiting_choice(&self) -> bool { self.waiting }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { !self.decline }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if let Some(option) = ctx.options.iter().find(|option|
            option.legal && option.description.starts_with("Enter as a copy of")) {
            return vec![option.index];
        }
        if ctx.description == "Choose a color" {
            return vec![ctx.options.iter().find(|option| option.description == "Blue").unwrap().index];
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if let Some(player) = self.target {
            assert_eq!(ctx.requirements.len(), 1);
            assert!(ctx.requirements[0].legal_targets.contains(&Target::Player(player)));
            vec![Target::Player(player)]
        } else { SelectFirstDecisionMaker.decide_targets(game, ctx) }
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition, controller: PlayerId, dm: &mut Choices) -> ObjectId {
    let hand = game.create_object_from_definition(definition, controller, Zone::Hand);
    let receipt = game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, dm).unwrap();
    assert!(!receipt.pending);
    let entered = receipt.original.into_result().unwrap().new_id;
    game.refresh_continuous_state().unwrap(); entered
}
fn announce(game: &mut GameState, player: PlayerId, action: LegalAction, dm: &mut Choices) {
    game.turn.priority_player = Some(player);
    assert!(compute_legal_actions(game, player).unwrap().contains(&action));
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..80 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() && state.pending_activation.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn cast(game: &mut GameState, definition: &CardDefinition, player: PlayerId, dm: &mut Choices) {
    let spell_id = game.create_object_from_definition(definition, player, Zone::Hand);
    announce(game, player, LegalAction::CastSpell { spell_id, from_zone: Zone::Hand, casting_method: CastingMethod::Normal }, dm);
}
fn activate(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let index = game.current_abilities(source).unwrap().iter().position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))).unwrap();
    announce(game, A, LegalAction::ActivateAbility { source, ability_index: index }, dm);
}
fn resolve(game: &mut GameState, dm: &mut Choices) { resolve_stack_entry_with(game, dm).unwrap(); }
fn event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) -> usize {
    let entries = check_triggers(game, &event); let count = entries.len();
    let mut queue = TriggerQueue::new(); for entry in entries { queue.add(entry); }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap(); count
}
fn pt(game: &GameState, id: ObjectId) -> (Option<i32>,Option<i32>) { (game.current_power(id), game.current_toughness(id)) }
fn chosen(game:&GameState,source:ObjectId)->Option<u32>{
    let memory=game.numeric_choice_memory(source);assert!(memory.len()<=1,"test helper requires a singular acquisition");
    memory.values().next().map(|record|record.number)
}
fn can_cast(game: &GameState, player: PlayerId, id: ObjectId) -> bool {
    compute_legal_actions(game, player).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
}

#[test]
fn every_frozen_body_has_both_strict_materializations() {
    for name in ["Sanctum Prelate", "Scrying Glass", "Void", "Talion, the Kindly Lord", "Shapeshifter"] { definitions(name); }
}

#[test]
fn prelate_zero_large_choices_live_prohibition_control_phasing_and_source_departure() {
    for definition in definitions("Sanctum Prelate") {
        for number in [0, 3, u32::MAX] {
            let mut g = game(); let mut dm = Choices { number, ..Default::default() };
            let source = enter(&mut g, &definition, A, &mut dm);
            assert_eq!(dm.prompts, vec![(A,0,None)]);
            assert_eq!(chosen(&g,source), Some(number));
            let zero = g.create_object_from_definition(&simple("Zero", "Artifact", "{0}", None, ""), A, Zone::Hand);
            let three = g.create_object_from_definition(&simple("Three", "Artifact", "{3}", None, ""), A, Zone::Hand);
            let creature = g.create_object_from_definition(&simple("Creature", "Artifact Creature", "{3}", Some((2,2)), ""), A, Zone::Hand);
            assert_eq!(can_cast(&g,A,zero), number != 0);
            assert_eq!(can_cast(&g,A,three), number != 3); assert!(can_cast(&g,A,creature));
            g.set_current_controller(source, B).unwrap(); g.refresh_continuous_state().unwrap();
            assert_eq!(can_cast(&g,A,three), number != 3, "the prohibition covers every caster");
            g.phase_out(source); g.refresh_continuous_state().unwrap(); assert!(can_cast(&g,A,three));
            g.phase_in(source); g.refresh_continuous_state().unwrap(); assert_eq!(can_cast(&g,A,three), number != 3);
            g.move_object_by_effect(source, Zone::Graveyard).unwrap(); g.refresh_continuous_state().unwrap(); assert!(can_cast(&g,A,three));
        }
        let mut g = game();
        let copy_without_entry = g.create_object_from_definition(&definition,A,Zone::Battlefield);
        g.refresh_continuous_state().unwrap();
        let zero = g.create_object_from_definition(&simple("Unchosen zero", "Artifact", "{0}",None,""),A,Zone::Hand);
        assert!(can_cast(&g,A,zero)); assert_eq!(chosen(&g,copy_without_entry),None);
    }
}

#[test]
fn shapeshifter_optional_upkeep_preserves_last_choice_under_new_controller_and_new_incarnation() {
    for definition in definitions("Shapeshifter") {
        let mut g = game(); let mut dm = Choices { number: 2, ..Default::default() };
        let source = enter(&mut g,&definition,A,&mut dm); assert_eq!(pt(&g,source),(Some(2),Some(5)));
        dm.decline = true; dm.number = 7;
        assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()),&mut dm),1);
        resolve(&mut g,&mut dm); assert_eq!(dm.prompts.len(),1); assert_eq!(pt(&g,source),(Some(2),Some(5)));
        g.set_current_controller(source,C).unwrap(); g.refresh_continuous_state().unwrap();
        assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()),&mut dm),0);
        dm.decline = false; dm.number = 0;
        assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(C),Default::default()),&mut dm),1);
        resolve(&mut g,&mut dm); g.refresh_continuous_state().unwrap(); assert_eq!(pt(&g,source),(Some(0),Some(7)));
        assert_eq!(dm.prompts.last(),Some(&(C,0,Some(7))));
        let hand = g.move_object_by_effect(source,Zone::Hand).unwrap();
        assert_eq!(chosen(&g,hand),None);
        assert_eq!(pt(&g,hand),(Some(0),Some(7)),"a known never-made choice has its printed initial CDA value");
        dm.number = 6;
        let returned = g.move_object_with_etb_processing_with_dm(hand,Zone::Battlefield,&mut dm).unwrap().original.into_result().unwrap().new_id;
        g.refresh_continuous_state().unwrap(); assert_eq!(pt(&g,returned),(Some(6),Some(1)));
        assert_ne!(returned,source);
    }
}

#[test]
fn shapeshifter_token_copy_makes_its_own_choice_and_old_upkeep_cannot_write_new_source() {
    for definition in definitions("Shapeshifter") {
        let mut g = game(); let mut dm = Choices { number: 2, ..Default::default() };
        let source = enter(&mut g,&definition,A,&mut dm);
        dm.number = 4;
        let copied = ironsmith::effects::CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(source))
            .execute(&mut g,&mut EffectContext::new(source,A,&mut dm)).unwrap().explicit_objects().unwrap()[0];
        g.refresh_continuous_state().unwrap(); assert_eq!(pt(&g,source),(Some(2),Some(5))); assert_eq!(pt(&g,copied),(Some(4),Some(3)));
        event(&mut g,TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()),&mut dm);
        let hand = g.move_object_by_effect(source,Zone::Hand).unwrap(); dm.number = 1;
        let returned = g.move_object_with_etb_processing_with_dm(hand,Zone::Battlefield,&mut dm).unwrap().original.into_result().unwrap().new_id;
        dm.number = 6;
        while !g.stack_is_empty() { resolve(&mut g,&mut dm); }
        assert_eq!(chosen(&g,returned),Some(1));
    }
}

#[test]
fn talion_one_trigger_for_any_matching_axis_uses_completed_cast_evidence_and_controller() {
    for definition in definitions("Talion, the Kindly Lord") {
        for (cost,p,t,matches) in [("{3}",3,3,true),("{1}",3,4,true),("{1}",4,3,true),("{1}",4,4,false)] {
            let mut g = game(); let mut dm = Choices { number: 3, ..Default::default() };
            let source = enter(&mut g,&definition,A,&mut dm);
            assert!(g.current_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Flying));
            assert_eq!(dm.prompts,vec![(A,1,Some(10))]);
            let spell = g.create_object_from_definition(&simple("Cast witness","Creature",cost,Some((p,t)),""),B,Zone::Stack);
            let captured = ironsmith::events::spells::SpellCastEvent::from_completed_cast(spell,B,Zone::Hand,&g);
            g.object_mut(spell).unwrap().base_power=Some(ironsmith::card::PtValue::Fixed(9));
            g.object_mut(spell).unwrap().base_toughness=Some(ironsmith::card::PtValue::Fixed(9));
            g.move_object_by_effect(spell,Zone::Graveyard).unwrap();
            assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(captured,Default::default()),&mut dm),usize::from(matches));
            if matches {
                g.move_object_by_effect(source,Zone::Graveyard).unwrap(); resolve(&mut g,&mut dm);
                assert_eq!(g.player(B).unwrap().life,18); assert_eq!(g.player(A).unwrap().hand.len(),1); assert_eq!(g.player(C).unwrap().life,20);
            }
        }
        let mut g=game();let mut dm=Choices{number:10,..Default::default()};let source=enter(&mut g,&definition,A,&mut dm);
        g.set_current_controller(source,B).unwrap();g.refresh_continuous_state().unwrap();
        let spell=g.create_object_from_definition(&simple("Ten","Sorcery","{10}",None,""),B,Zone::Stack);
        let own=ironsmith::events::spells::SpellCastEvent::from_completed_cast(spell,B,Zone::Hand,&g);
        assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(own,Default::default()),&mut dm),0);
        let spell=g.create_object_from_definition(&simple("Ten opponent","Sorcery","{10}",None,""),C,Zone::Stack);
        let opponent=ironsmith::events::spells::SpellCastEvent::from_completed_cast(spell,C,Zone::Hand,&g);
        assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(opponent,Default::default()),&mut dm),1);
        resolve(&mut g,&mut dm);assert_eq!(g.player(C).unwrap().life,18);assert_eq!(g.player(B).unwrap().hand.len(),1);
    }
}

#[test]
fn void_announces_player_and_retains_number_through_destruction_and_reveal() {
    for definition in definitions("Void") {
        for chosen in [0,2] {
            let mut g=game();let mut dm=Choices{number:chosen,target:Some(B),..Default::default()};
            let cost=if chosen==0 {"{0}"} else {"{2}"};
            let creature=g.create_object_from_definition(&simple("Chosen creature","Creature",cost,Some((2,2)),""),C,Zone::Battlefield);
            let artifact=g.create_object_from_definition(&simple("Chosen artifact","Artifact",cost,None,""),A,Zone::Battlefield);
            let safe=g.create_object_from_definition(&simple("Other type","Enchantment",cost,None,""),B,Zone::Battlefield);
            let indestructible=g.create_object_from_definition(&simple("Protected","Artifact Creature",cost,Some((2,2)),"Indestructible"),B,Zone::Battlefield);
            let discard=g.create_object_from_definition(&simple("Discard","Sorcery",cost,None,""),B,Zone::Hand);
            let land=g.create_object_from_definition(&simple("Land","Land","",None,""),B,Zone::Hand);
            let other=g.create_object_from_definition(&simple("Other hand","Sorcery",cost,None,""),C,Zone::Hand);
            cast(&mut g,&definition,A,&mut dm);assert!(dm.prompts.is_empty());
            assert_eq!(g.stack.last().unwrap().targets,vec![Target::Player(B)]);
            resolve(&mut g,&mut dm);assert_eq!(dm.prompts,vec![(A,0,None)]);
            assert!(g.object(creature).is_none());assert!(g.object(artifact).is_none());assert!(g.object(discard).is_none());
            assert!(g.object(safe).is_some());assert!(g.object(indestructible).is_some());assert!(g.object(land).is_some());assert!(g.object(other).is_some());
        }
    }
}

#[test]
fn void_and_glass_all_illegal_targets_prevent_every_resolution_choice() {
    for name in ["Void","Scrying Glass"] { for definition in definitions(name) {
        let mut g=game();let mut dm=Choices{number:2,target:Some(B),..Default::default()};
        let witness=g.create_object_from_definition(&simple("Survives fizzle","Artifact","{2}",None,""),A,Zone::Battlefield);
        if name=="Void" {cast(&mut g,&definition,A,&mut dm);} else {let source=enter(&mut g,&definition,A,&mut dm);activate(&mut g,source,&mut dm);}
        assert!(dm.prompts.is_empty());
        enter(&mut g,&simple("Player protection","Enchantment","{1}",None,"You have hexproof."),B,&mut dm);
        resolve(&mut g,&mut dm);assert!(dm.prompts.is_empty());assert!(g.object(witness).is_some());
        assert!(g.player(A).unwrap().hand.is_empty());
    }}
}

#[test]
fn scrying_glass_positive_domain_exact_color_card_count_and_departed_source() {
    for definition in definitions("Scrying Glass") {
        for chosen in [1,2,3,u32::MAX] {
            let mut g=game();let mut dm=Choices{number:chosen,target:Some(B),..Default::default()};let source=enter(&mut g,&definition,A,&mut dm);
            for (name,cost) in [("Blue","{U}"),("Blue multicolor","{U}{R}"),("Red","{R}")] {
                g.create_object_from_definition(&simple(name,"Instant",cost,None,""),B,Zone::Hand);
            }
            let before=g.player(A).unwrap().mana_pool.total();activate(&mut g,source,&mut dm);
            assert!(g.is_tapped(source));assert_eq!(g.player(A).unwrap().mana_pool.total(),before-3);
            assert!(dm.prompts.is_empty());assert_eq!(g.stack.last().unwrap().targets,vec![Target::Player(B)]);
            g.move_object_by_effect(source,Zone::Graveyard).unwrap();resolve(&mut g,&mut dm);
            assert_eq!(dm.prompts,vec![(A,1,None)]);assert_eq!(g.player(A).unwrap().hand.len(),usize::from(chosen==2));
            assert_eq!(g.player(B).unwrap().hand.len(),3);
        }
    }
}

#[test]
fn persistent_choice_value_uses_exact_departure_receipt_and_missing_evidence_rolls_back() {
    let mut g=game();let source=g.create_object_from_definition(&simple("Source","Artifact","{1}",None,""),A,Zone::Battlefield);
    let pair=ironsmith_core::LinkedExilePair{definition:ironsmith_core::LinkedExileDefinition([44;32]),pair:0};
    let owner=ironsmith::linked_exile::LinkedExileOwner{host:source,pair,acquisition:ironsmith::linked_exile::LinkedExileAcquisition::Printed};
    let mut dm=Choices{number:u32::MAX,..Default::default()};
    execute_effect(&mut g,&Effect::new(ironsmith::effects::ChooseNumberEffect::unbounded(PlayerFilter::You).with_source_retention()),&mut EffectContext::new(source,A,&mut dm).with_source_number_owner(Some(owner.clone()))).unwrap();
    let snapshot=ObjectSnapshot::from_object_with_calculated_characteristics(g.object(source).unwrap(),&g);
    let new=g.move_object_by_effect(source,Zone::Hand).unwrap();g.set_number_for_acquisition(ironsmith::linked_exile::LinkedExileOwner{host:new,..owner.clone()},7).unwrap();
    g.turn_store.turn_history.clear_for_new_turn();
    let value=Value::SourceChosenNumber{if_unset:Some(0),pair:Some(pair)};
    assert_eq!(ironsmith::effects::helpers::resolve_value_wide(&g,&value,&EffectContext::new(source,A,&mut dm).with_source_snapshot(snapshot.clone()).with_source_number_owner(Some(owner.clone()))).unwrap(),i64::from(u32::MAX));
    let mismatch=ObjectSnapshot::from_object_with_calculated_characteristics(g.object(new).unwrap(),&g);
    assert!(ironsmith::effects::helpers::resolve_value_wide(&g,&value,&EffectContext::new(source,A,&mut dm).with_source_snapshot(mismatch).with_source_number_owner(Some(owner.clone()))).is_err());
    let before=g.player(A).unwrap().life;
    let transaction=Effect::new(ironsmith::effects::SequenceEffect::new(vec![Effect::gain_life(4),Effect::gain_life(value)]));
    assert!(execute_effect(&mut g,&transaction,&mut EffectContext::new(source,A,&mut dm)).is_err());
    assert_eq!(g.player(A).unwrap().life,before);
    assert_eq!(chosen(&g,new),Some(7));
}

#[test]
fn acquired_copy_numeric_choice_cannot_borrow_original_choice_and_expiry_restores_original() {
    // The original and copied text box have separate linked acquisitions.
    for original_definition in definitions("Shapeshifter") {
        let [donor_definition, _]=definitions("Sanctum Prelate");
        let mut g=game();let mut dm=Choices{number:2,..Default::default()};
        let original=enter(&mut g,&original_definition,A,&mut dm);dm.number=3;
        let donor=enter(&mut g,&donor_definition,B,&mut dm);
        let values=ironsmith::snapshot::CopiableValues::from_object(g.object(donor).unwrap());
        let copy=g.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::from_resolution(
            donor,B,vec![original],ironsmith::continuous::Modification::CopyOf{
                target_id:donor,copiable_values:Box::new(values),preserve_source_abilities:false,
                name_override:None,name_override_surface:None,add_supertypes:vec![],
            }));
        // Remove the donor so its independently chosen three cannot mask the
        // copy's genuinely never-made choice.
        g.move_object_by_effect(donor,Zone::Graveyard).unwrap();g.refresh_continuous_state().unwrap();
        let two=g.create_object_from_definition(&simple("Two mana","Artifact","{2}",None,""),A,Zone::Hand);
        let three=g.create_object_from_definition(&simple("Three mana","Artifact","{3}",None,""),A,Zone::Hand);
        assert!(can_cast(&g,A,two),"a new copied Prelate ability cannot use original Shapeshifter's two");
        assert!(can_cast(&g,A,three),"a copy cannot import the donor's chosen number");
        g.effect_store.continuous_effects.remove_effect(copy);g.refresh_continuous_state().unwrap();
        assert_eq!(pt(&g,original),(Some(2),Some(5)),"original linked choice returns with original abilities");
    }
}

#[test]
fn copied_shapeshifter_reselection_has_distinct_group_and_public_proof_ignores_internal_allocations() {
    for definition in definitions("Shapeshifter") {
        let mut proofs=Vec::new();let mut copy_ids=Vec::new();
        for perturb in [false,true] {
            let mut g=game();let mut dm=Choices{number:2,..Default::default()};
            let source=enter(&mut g,&definition,A,&mut dm);
            let donor=g.create_object_from_definition(&definition,B,Zone::Battlefield);
            if perturb {
                for _ in 0..5 {
                    let _unused=ironsmith::card::CardBuilder::new(ironsmith::CardId::new(),"Unused native allocation").build();
                    let _unused=ironsmith::static_abilities::StaticAbility::flying();
                    let temporary=g.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::from_resolution(
                        source,A,vec![],ironsmith::continuous::Modification::AddColors(ironsmith::color::ColorSet::BLUE)));
                    g.effect_store.continuous_effects.remove_effect(temporary);
                }
            }
            let values=ironsmith::snapshot::CopiableValues::from_object(g.object(donor).unwrap());
            let copy=g.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::from_resolution(
                source,A,vec![source],ironsmith::continuous::Modification::CopyOf{
                    target_id:donor,copiable_values:Box::new(values),preserve_source_abilities:false,
                    name_override:None,name_override_surface:None,add_supertypes:vec![],
                }));
            copy_ids.push(copy);
            g.move_object_by_effect(donor,Zone::Graveyard).unwrap();g.refresh_continuous_state().unwrap();
            assert_eq!(pt(&g,source),(Some(0),Some(7)),"a new copied acquisition has not chosen a number");
            let fresh=ironsmith::source_numbers::public_proof(&g,source,true).unwrap();
            assert_eq!(fresh.records.len(),1);assert!(fresh.bindings.iter().all(|binding|binding.group.is_none()));
            dm.number=5;
            assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()),&mut dm),1);
            resolve(&mut g,&mut dm);g.refresh_continuous_state().unwrap();assert_eq!(pt(&g,source),(Some(5),Some(2)));
            let copied=ironsmith::source_numbers::public_proof(&g,source,true).unwrap();
            assert_eq!(copied.records.iter().map(|record|(record.group,record.number)).collect::<Vec<_>>(),vec![(0,2),(1,5)]);
            assert!(copied.bindings.iter().all(|binding|binding.group==Some(1)));
            assert_eq!(ironsmith::source_numbers::public_proof(&g,source,false).unwrap(),Default::default(),"hidden identity suppresses records and definition hashes");
            g.effect_store.continuous_effects.remove_effect(copy);g.refresh_continuous_state().unwrap();
            assert_eq!(pt(&g,source),(Some(2),Some(5)));
            let expired=ironsmith::source_numbers::public_proof(&g,source,true).unwrap();
            assert_eq!(expired.records,copied.records);assert!(expired.bindings.iter().all(|binding|binding.group==Some(0)));
            proofs.push((fresh,copied,expired));
        }
        assert_ne!(copy_ids[0],copy_ids[1],"the games actually used different native copy registration IDs");
        assert_eq!(proofs[0],proofs[1],"canonical proof is independent of native card/static/effect allocation order");
    }
}

#[test]
fn prelate_uses_selected_split_face_and_the_announced_x_proposal() {
    for definition in definitions("Sanctum Prelate") {
        for chosen in [1,4] {
            let mut g=game();let mut dm=Choices{number:chosen,..Default::default()};enter(&mut g,&definition,A,&mut dm);
            let mut front=simple("One half","Sorcery","{1}",None,"You gain 1 life.");
            let mut back=simple("Four half","Sorcery","{4}",None,"You gain 4 life.");
            front.card.other_face=Some(back.card.id);front.card.other_face_name=Some(back.card.name.clone());front.card.linked_face_layout=ironsmith::card::LinkedFaceLayout::Split;
            back.card.other_face=Some(front.card.id);back.card.other_face_name=Some(front.card.name.clone());back.card.linked_face_layout=ironsmith::card::LinkedFaceLayout::Split;
            g.register_linked_face_definition(&back);
            let spell_id=g.create_object_from_definition(&front,A,Zone::Hand);
            let first=LegalAction::CastSpell{spell_id,from_zone:Zone::Hand,casting_method:CastingMethod::Normal};
            let second=LegalAction::CastSpell{spell_id,from_zone:Zone::Hand,casting_method:CastingMethod::SplitOtherHalf};
            let actions=compute_legal_actions(&g,A).unwrap();assert_eq!(actions.contains(&first),chosen!=1);assert_eq!(actions.contains(&second),chosen!=4);
            announce(&mut g,A,if chosen==1{second}else{first},&mut dm);
            let spell=g.stack.last().unwrap().object_id;
            assert_eq!(g.object(spell).unwrap().name,if chosen==1{"Four half"}else{"One half"});
        }
        let mut g=game();let mut dm=Choices{number:0,..Default::default()};enter(&mut g,&definition,A,&mut dm);
        let x_spell=simple("Announced X","Sorcery","{X}",None,"You gain X life.");
        let spell_id=g.create_object_from_definition(&x_spell,A,Zone::Hand);
        g.player_mut(A).unwrap().mana_pool=Default::default();assert!(!can_cast(&g,A,spell_id),"only payable X=0 is prohibited");
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);assert!(can_cast(&g,A,spell_id),"a payable nonzero proposal permits casting to begin");
        dm.number=1;
        announce(&mut g,A,LegalAction::CastSpell{spell_id,from_zone:Zone::Hand,casting_method:CastingMethod::Normal},&mut dm);
        let spell=g.stack.last().unwrap().object_id;assert_eq!(g.object(spell).unwrap().x_value,Some(1));
        resolve(&mut g,&mut dm);assert_eq!(g.player(A).unwrap().life,21);
    }
}

#[test]
fn pending_upkeep_keeps_its_controller_and_numeric_acquisition_when_control_changes() {
    for definition in definitions("Shapeshifter") {
        let mut g=game();let mut dm=Choices{number:2,..Default::default()};let source=enter(&mut g,&definition,A,&mut dm);
        assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()),&mut dm),1);
        g.set_current_controller(source,B).unwrap();g.refresh_continuous_state().unwrap();dm.number=4;
        resolve(&mut g,&mut dm);assert_eq!(dm.prompts.last(),Some(&(A,0,Some(7))));assert_eq!(pt(&g,source),(Some(4),Some(3)));
        assert_eq!(g.current_controller(source),Some(B));
    }
}


fn duration_copy_definition() -> CardDefinition {
    // Related fixture exercises the existing entry-copy owner; this family
    // makes no full-card coverage claim for Cursed Mirror.
    simple("Cursed Mirror", "Artifact", "{2}{R}", None,
        "{T}: Add {R}.\nAs this artifact enters, you may have it become a copy of any creature on the battlefield until end of turn, except it has haste.")
}

#[test]
fn duration_entry_copies_choose_for_their_actual_acquisitions_and_expire_independently() {
    for definition in definitions("Sanctum Prelate") {
        let mut g=game();let mut dm=Choices{number:1,..Default::default()};
        let donor=enter(&mut g,&definition,B,&mut dm);let mirror=duration_copy_definition();
        dm.number=3;let first=enter(&mut g,&mirror,A,&mut dm);
        dm.number=4;let second=enter(&mut g,&mirror,C,&mut dm);
        g.move_object_by_effect(donor,Zone::Graveyard).unwrap();g.refresh_continuous_state().unwrap();
        g.turn.active_player=B;g.turn.priority_player=Some(B);
        for mana in [1,2,3,4] {
            let spell=g.create_object_from_definition(&simple("Copy prohibition witness","Artifact",
                &format!("{{{mana}}}"),None,""),B,Zone::Hand);
            assert_eq!(can_cast(&g,B,spell),!matches!(mana,3|4));
        }
        for (source,number) in [(first,3),(second,4)] {
            let memory=g.numeric_choice_memory(source);
            assert_eq!(memory.len(),1);assert_eq!(memory.values().next().unwrap().number,number);
            assert!(memory.keys().all(|owner|matches!(&owner.acquisition,ironsmith::linked_exile::LinkedExileAcquisition::Effect(_))));
            let proof=ironsmith::source_numbers::public_proof(&g,source,true).unwrap();
            assert!(proof.bindings.iter().all(|binding|binding.group==Some(0)));
        }
        g.effect_store.continuous_effects.cleanup_end_of_turn();g.refresh_continuous_state().unwrap();
        for source in [first,second] {
            assert_eq!(g.current_name(source).as_deref(),Some("Cursed Mirror"));
            let proof=ironsmith::source_numbers::public_proof(&g,source,true).unwrap();
            assert_eq!(proof.records.len(),1);assert!(proof.bindings.is_empty());
        }
        let spell=g.create_object_from_definition(&simple("Released prohibition","Artifact","{3}",None,""),B,Zone::Hand);
        assert!(can_cast(&g,B,spell));
    }
    for definition in definitions("Shapeshifter") {
        let mut g=game();let mut dm=Choices{number:2,..Default::default()};
        enter(&mut g,&definition,B,&mut dm);let mirror=duration_copy_definition();
        dm.number=4;let first=enter(&mut g,&mirror,A,&mut dm);
        dm.number=6;let second=enter(&mut g,&mirror,C,&mut dm);
        assert_eq!(pt(&g,first),(Some(4),Some(3)));assert_eq!(pt(&g,second),(Some(6),Some(1)));
        dm.number=5;
        assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()),&mut dm),1);
        resolve(&mut g,&mut dm);assert_eq!(pt(&g,first),(Some(5),Some(2)));assert_eq!(pt(&g,second),(Some(6),Some(1)));
        assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()),&mut dm),1);
        g.effect_store.continuous_effects.cleanup_end_of_turn();g.refresh_continuous_state().unwrap();
        dm.number=3;resolve(&mut g,&mut dm);
        assert_eq!(g.current_name(first).as_deref(),Some("Cursed Mirror"));
        assert_eq!(g.numeric_choice_memory(first).values().next().unwrap().number,3,"captured upkeep can update its dormant acquisition");
        assert_eq!(g.numeric_choice_memory(second).values().next().unwrap().number,6);
        assert_eq!(g.numeric_choice_memory(first).len(),1,"reselection keeps the entry acquisition and ordinal");
    }
}

#[test]
fn delayed_numeric_match_keeps_the_admitted_host_acquisition_and_latest_departure_choice() {
    for definition in definitions("Talion, the Kindly Lord") {
        let mut g=game();let mut dm=Choices{number:3,..Default::default()};
        let source=enter(&mut g,&definition,A,&mut dm);
        let chars=g.current_characteristics(source).unwrap();
        let (slot,ability)=chars.abilities.iter().enumerate().find_map(|(slot,ability)|
            if let ironsmith::ability::AbilityKind::Triggered(ability)=&ability.kind {Some((slot,ability.clone()))}else{None}).unwrap();
        let owner=ironsmith::linked_exile::LinkedExileOwner::capture(source,ability.effects.source_number_pair,chars.abilities.origin(slot)).unwrap();
        let snapshot=ObjectSnapshot::from_object_with_calculated_characteristics(g.object(source).unwrap(),&g);
        let watched=g.create_object_from_definition(&simple("Watched object","Artifact","{1}",None,""),C,Zone::Battlefield);
        g.effect_store.delayed_triggers.push(ironsmith::triggers::DelayedTrigger{
            linked_exile_owner:None,source_number_owner:Some(owner.clone()),trigger:ability.trigger.clone(),effects:ability.effects.clone(),
            one_shot:false,x_value:None,not_before_turn:None,expires_at_turn:None,expires_before_controller_turn_after:None,
            expires_at_end_of_combat:false,bound_extra_turn_index:None,while_any_tagged_object_in_zone:None,
            target_objects:vec![watched],ability_source:Some(source),ability_source_stable_id:Some(snapshot.stable_id),
            ability_source_name:Some(snapshot.name.clone()),ability_source_snapshot:Some(snapshot),controller:A,
            choices:ability.choices.clone(),tagged_objects:Default::default(),tagged_players:Default::default(),
            defending_player_reference:None,prepayment:None,prevention_shield:None,
        });
        let values=ironsmith::snapshot::CopiableValues::from_object(g.object(source).unwrap());
        g.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::from_resolution(
            source,A,vec![source],ironsmith::continuous::Modification::CopyOf{target_id:source,copiable_values:Box::new(values),
                preserve_source_abilities:false,name_override:None,name_override_surface:None,add_supertypes:vec![]}));
        g.refresh_continuous_state().unwrap();let copied=g.current_characteristics(source).unwrap();
        let copied_owner=ironsmith::linked_exile::LinkedExileOwner::capture(source,ability.effects.source_number_pair,copied.abilities.origin(slot)).unwrap();
        assert_ne!(owner,copied_owner);g.set_number_for_acquisition(copied_owner,7).unwrap();
        let matches=|g:&mut GameState,mana:u32| {
            let spell=g.create_object_from_definition(&simple("Delayed cast witness","Artifact",&format!("{{{mana}}}"),None,""),B,Zone::Stack);
            let event=TriggerEvent::new_with_provenance(ironsmith::events::SpellCastEvent::from_completed_cast(spell,B,Zone::Hand,g),Default::default());
            ironsmith::triggers::check_delayed_triggers(g,&event).len()
        };
        assert_eq!(matches(&mut g,3),1);assert_eq!(matches(&mut g,7),0);
        g.set_number_for_acquisition(owner.clone(),5).unwrap();
        let later=g.move_object_by_effect(source,Zone::Graveyard).unwrap();
        g.set_number_for_acquisition(ironsmith::linked_exile::LinkedExileOwner{host:later,..owner},7).unwrap();
        assert_eq!(matches(&mut g,5),1,"true departure supersedes the old admitted snapshot");
        assert_eq!(matches(&mut g,3),0);assert_eq!(matches(&mut g,7),0,"neither copied acquisition nor later incarnation supplies the chosen number");
    }
}


#[test]
fn simultaneous_same_definition_acquisitions_on_one_host_keep_each_live_prohibition() {
    // Synthetic preserving-copy effects exercise multiple concurrent text
    // acquisitions on one host, beyond the five printed card programs.
    for definition in definitions("Sanctum Prelate") {
        let mut g=game();let mut dm=Choices{number:2,..Default::default()};
        let source=enter(&mut g,&definition,A,&mut dm);
        let pair=g.numeric_choice_memory(source).keys().next().unwrap().pair;
        let values=ironsmith::snapshot::CopiableValues::from_object(g.object(source).unwrap());
        let mut copies=Vec::new();
        for number in [3,4] {
            let id=g.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::from_resolution(
                source,A,vec![source],ironsmith::continuous::Modification::CopyOf{target_id:source,copiable_values:Box::new(values.clone()),
                    preserve_source_abilities:true,name_override:None,name_override_surface:None,add_supertypes:vec![]}));
            let effect=g.effect_store.continuous_effects.effects().iter().find(|effect|effect.id==id).unwrap();
            let owner=ironsmith::linked_exile::LinkedExileOwner{host:source,pair,
                acquisition:ironsmith::linked_exile::LinkedExileAcquisition::Effect(effect.into())};
            dm.number=number;
            execute_effect(&mut g,&Effect::new(ironsmith::effects::ChooseNumberEffect::unbounded(PlayerFilter::You).with_source_retention()),
                &mut EffectContext::new(source,A,&mut dm).with_source_number_owner(Some(owner))).unwrap();
            copies.push(id);
        }
        g.refresh_continuous_state().unwrap();
        let proof=ironsmith::source_numbers::public_proof(&g,source,true).unwrap();
        assert_eq!(proof.records.iter().map(|record|(record.group,record.number)).collect::<Vec<_>>(),vec![(0,2),(1,3),(2,4)]);
        assert_eq!(proof.bindings.iter().filter_map(|binding|binding.group).collect::<std::collections::HashSet<_>>(),[0,1,2].into_iter().collect());
        let mut spells=Vec::new();
        for mana in [2,3,4,5] {
            let spell=g.create_object_from_definition(&simple("Concurrent prohibition witness","Artifact",&format!("{{{mana}}}"),None,""),A,Zone::Hand);
            assert_eq!(can_cast(&g,A,spell),mana==5);spells.push(spell);
        }
        g.effect_store.continuous_effects.remove_effect(copies[0]);g.refresh_continuous_state().unwrap();
        assert!(!can_cast(&g,A,spells[0]));assert!(can_cast(&g,A,spells[1]));assert!(!can_cast(&g,A,spells[2]));
        g.effect_store.continuous_effects.remove_effect(copies[1]);g.refresh_continuous_state().unwrap();
        assert!(!can_cast(&g,A,spells[0]));assert!(can_cast(&g,A,spells[1]));assert!(can_cast(&g,A,spells[2]));
        assert_eq!(g.numeric_choice_memory(source).len(),3,"expired choices remain dormant, not reassigned");
    }
}

#[test]
fn two_preserved_talion_acquisitions_each_trigger_once_when_all_three_axes_match() {
    for definition in definitions("Talion, the Kindly Lord") {
        let mut g=game();let mut dm=Choices{number:3,..Default::default()};
        let source=enter(&mut g,&definition,A,&mut dm);
        let pair=g.numeric_choice_memory(source).keys().next().unwrap().pair;
        let values=ironsmith::snapshot::CopiableValues::from_object(g.object(source).unwrap());
        let copy=g.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::from_resolution(
            source,A,vec![source],ironsmith::continuous::Modification::CopyOf{target_id:source,copiable_values:Box::new(values),
                preserve_source_abilities:true,name_override:None,name_override_surface:None,add_supertypes:vec![]}));
        let effect=g.effect_store.continuous_effects.effects().iter().find(|effect|effect.id==copy).unwrap();
        let owner=ironsmith::linked_exile::LinkedExileOwner{host:source,pair,
            acquisition:ironsmith::linked_exile::LinkedExileAcquisition::Effect(effect.into())};
        g.set_number_for_acquisition(owner,3).unwrap();g.refresh_continuous_state().unwrap();
        let spell=g.create_object_from_definition(&simple("Triple match","Creature","{3}",Some((3,3)),""),B,Zone::Stack);
        let cast=ironsmith::events::SpellCastEvent::from_completed_cast(spell,B,Zone::Hand,&g);
        let life=g.player(B).unwrap().life;let hand=g.player(A).unwrap().hand.len();
        assert_eq!(event(&mut g,TriggerEvent::new_with_provenance(cast,Default::default()),&mut dm),2,
            "two independent acquisitions each trigger once, even though each matches three axes");
        resolve(&mut g,&mut dm);resolve(&mut g,&mut dm);
        assert_eq!(g.player(B).unwrap().life,life-4);assert_eq!(g.player(A).unwrap().hand.len(),hand+2);
    }
}


#[test]
fn pending_and_failed_entry_number_choices_restore_the_reserved_copy_acquisition() {
    for definition in definitions("Shapeshifter") {
        for pending in [false,true] {
            let mut g=game();let mut dm=Choices{number:2,..Default::default()};
            let donor=enter(&mut g,&definition,B,&mut dm);
            let entrant=g.create_object_from_definition(&duration_copy_definition(),A,Zone::Hand);
            let before=g.clone();dm.number=if pending{4}else{8};dm.defer_number=pending;dm.prompts.clear();
            let result=g.move_object_with_etb_processing_with_dm(entrant,Zone::Battlefield,&mut dm);
            assert_eq!(dm.prompts,vec![(A,0,Some(7))],"the copy reservation precedes this real number prompt");
            if pending {assert!(result.unwrap().pending);}else{assert!(result.is_err());}
            assert_eq!(g.object(entrant).unwrap().zone,Zone::Hand);assert!(g.numeric_choice_memory(entrant).is_empty());
            assert_eq!(g.numeric_choice_memory(donor),before.numeric_choice_memory(donor));
            let probe=ironsmith::continuous::ContinuousEffect::new(entrant,A,
                ironsmith::continuous::EffectTarget::Specific(entrant),ironsmith::continuous::Modification::AddColors(ironsmith::color::ColorSet::BLUE));
            let mut expected=before.effect_store.continuous_effects.clone();let mut actual=g.effect_store.continuous_effects.clone();
            assert_eq!(actual.add_effect(probe.clone()),expected.add_effect(probe),
                "the entry root restores the allocation sequence after allocating its reservation");
        }
    }
}

#[test]
fn omitted_program_numeric_pair_in_a_full_body_codec_cannot_produce_a_complete_public_proof() {
    let rows:Vec<serde_json::Value>=serde_json::from_str(include_str!("../../../fixtures/source_number_bodies.json.fixture")).unwrap();
    let row=rows.iter().find(|row|row["name"]=="Shapeshifter").unwrap();
    let text=format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(),row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(),row["toughness"].as_str().unwrap(),row["oracle_text"].as_str().unwrap());
    let (artifact,_)=compile_to_artifact("Shapeshifter",text,false).unwrap();
    let mut wire:serde_json::Value=serde_json::from_slice(&artifact.to_json().unwrap()).unwrap();
    fn omit(value:&mut serde_json::Value)->usize {
        match value {
            serde_json::Value::Object(fields)=>{
                let own=usize::from(fields.remove("source_number_pair").is_some());
                own+fields.values_mut().map(omit).sum::<usize>()
            }
            serde_json::Value::Array(values)=>values.iter_mut().map(omit).sum(),
            _=>0,
        }
    }
    assert!(omit(&mut wire)>=2,"entry and upkeep program ownership was present before omission");
    let legacy=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(wire.to_string().as_bytes()).unwrap();
    let definition=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&legacy).unwrap();
    let mut g=game();let source=g.create_object_from_definition(&definition,A,Zone::Battlefield);
    assert!(matches!(ironsmith::source_numbers::public_proof(&g,source,true),Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
}
