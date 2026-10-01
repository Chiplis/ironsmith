//! Isolated, opt-in canonical Rune/Equipment execution probes.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{DecisionContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::Write;

fn emit(stage: &str, data: Value) {
    println!("RUNE_STAGE {}", json!({"stage":stage,"data":data}));
    std::io::stdout().flush().unwrap();
}
struct Choices { target: ObjectId }
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let target = Target::Object(self.target);
        let legal = ctx.requirements.len() == 1 && ctx.requirements[0].legal_targets.contains(&target);
        emit("target_choice", json!({"requested":format!("{target:?}"),"legal":legal,"context":format!("{ctx:?}")}));
        assert!(legal, "fixture target must be explicitly offered");
        vec![target]
    }
}
fn perform(g: &mut GameState, source: ObjectId, ability: Option<usize>, dm: &mut Choices) -> Result<(), String> {
    g.turn.priority_player = Some(PlayerId(0));
    emit("before_legal_actions", json!({"source":source.0,"ability":ability}));
    let action = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a| match a {
        LegalAction::CastSpell{spell_id,..} => ability.is_none() && *spell_id == source,
        LegalAction::ActivateAbility{source:id,ability_index} => *id == source && ability == Some(*ability_index),
        _ => false,
    }).ok_or("intended legal action absent")?;
    let mana = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    emit("before_announcement", json!({"action":format!("{action:?}")}));
    let mut progress = apply_priority_response_with_dm(g, &mut q, &mut state, &PriorityResponse::PriorityAction(action), dm).map_err(|e|e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() && !g.stack.is_empty() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { return Err(format!("announcement stalled: {progress:?}")); };
        if matches!(ctx, DecisionContext::Priority(_)) { return Err("announcement returned priority without stack entry".into()); }
        progress = apply_decision_context_with_dm(g, &mut q, &mut state, &ctx, dm).map_err(|e|e.to_string())?;
    }
    if g.stack.is_empty() { return Err("announcement decision budget".into()); }
    emit("announced", json!({"mana_paid":mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"stack":g.stack.len(),"targets":format!("{:?}",g.stack.last().unwrap().targets)}));
    for step in 0..16 {
        emit("before_priority", json!({"step":step,"stack":g.stack.len()}));
        advance_priority_with_dm(g, &mut q, dm).map_err(|e|e.to_string())?;
        if g.stack.is_empty() { return Ok(()); }
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            apply_priority_response_with_dm(g, &mut q, &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority), dm).map_err(|e|e.to_string())?;
        }
    }
    Err("priority resolution budget".into())
}
fn run(defs: &HashMap<String,CardDefinition>, name: &str, fixture: &str) -> Result<Value,String> {
    let mut g = GameState::new(vec!["Alice".into(),"Bob".into()],20);
    g.turn.turn_number=3;
    g.turn.active_player=PlayerId(0);
    g.turn.priority_player=Some(PlayerId(0));
    g.turn.phase=ironsmith::Phase::FirstMain;
    g.turn.step=None;
    for color in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green,ManaSymbol::Colorless] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color,10);
    }
    let creature = CardDefinitionBuilder::new(CardId::new(),"Rune witness creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,6)).build();
    let target = g.create_object_from_definition(&creature,PlayerId(0),Zone::Battlefield);
    let inert = CardDefinitionBuilder::new(CardId::new(),"Rune witness land").card_types(vec![CardType::Land]).build();
    let land = g.create_object_from_definition(&inert,PlayerId(0),Zone::Battlefield);
    for _ in 0..8 {
        let card = CardDefinitionBuilder::new(CardId::new(),"Rune library card").card_types(vec![CardType::Instant]).build();
        g.create_object_from_definition(&card,PlayerId(0),Zone::Library);
    }
    let equip = g.create_object_from_definition(&defs["Bonesplitter"],PlayerId(0),Zone::Battlefield);
    let equip_index = defs["Bonesplitter"].abilities.iter().position(|a|matches!(a.kind,AbilityKind::Activated(_))).ok_or("missing equip")?;
    if fixture == "equipment_before" || fixture == "equipment_control" {
        emit("equip_before_rune", json!({"equipment":equip.0,"creature":target.0}));
        perform(&mut g,equip,Some(equip_index),&mut Choices{target})?;
    }
    if fixture != "equipment_control" {
        let aura = g.create_object_from_definition(&defs[name],PlayerId(0),Zone::Hand);
        let enchant = match fixture { "creature" => target, "land" => land, _ => equip };
        emit("cast_rune",json!({"enchant":enchant.0,"fixture":fixture}));
        perform(&mut g,aura,None,&mut Choices{target:enchant})?;
    }
    if fixture == "equipment_after" {
        emit("equip_after_rune",json!({"equipment":equip.0,"creature":target.0}));
        perform(&mut g,equip,Some(equip_index),&mut Choices{target})?;
    }
    emit("before_characteristic_query",json!({}));
    let keyword = match name {
        "Rune of Flight" => StaticAbilityId::Flying,
        "Rune of Might" => StaticAbilityId::Trample,
        "Rune of Mortality" => StaticAbilityId::Deathtouch,
        "Rune of Speed" => StaticAbilityId::Haste,
        "Rune of Sustenance" => StaticAbilityId::Lifelink,
        _ => return Err("unknown Rune".into()),
    };
    Ok(json!({"keyword":g.current_has_static_ability_id(target,keyword),"power":g.calculated_power(target),"toughness":g.calculated_toughness(target),"hand":g.player(PlayerId(0)).unwrap().hand.len(),"stack":g.stack.len()}))
}
#[test]
#[ignore = "isolated audit reporter; passing does not certify card behavior"]
fn report_rune_case() {
    let name=std::env::var("AUDIT_RUNE_NAME").unwrap();
    let fixture=std::env::var("AUDIT_RUNE_FIXTURE").unwrap();
    let input:Value=serde_json::from_slice(&std::fs::read(std::env::var("AUDIT_RUNE_INPUT").unwrap()).unwrap()).unwrap();
    let mut defs=HashMap::new();
    for p in input["cards"].as_array().unwrap() {
        let n=p["name"].as_str().unwrap();
        let (artifact,def)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false).unwrap();
        emit("compiled",json!({"name":n,"artifact_checksum":artifact.payload_checksum}));
        defs.insert(n.to_string(),def);
    }
    emit("fixture_start",json!({"card":name,"fixture":fixture}));
    match run(&defs,&name,&fixture) {
        Ok(actual) => emit("finished",json!({"actual":actual})),
        Err(error) => emit("finished",json!({"execution_or_fixture_error":error})),
    }
}
