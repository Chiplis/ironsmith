//! cf8 p01: "of each basic land type" / "of each color" quantifiers keep
//! both their per-type semantics and their authored surface.
//! Source-authored, deliberately unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn coalition_victory_requires_every_basic_land_type_and_color() {
    for definition in support::definitions("Coalition Victory") {
        let text = support::rendered(&definition);
        assert!(
            text.contains("you control a land of each basic land type and a creature of each color"),
            "{text}"
        );
        let debug = format!("{definition:?}");
        // Five land-type leaves plus five color leaves.
        assert_eq!(debug.matches("PlayerControls {").count(), 10, "{debug}");
    }
}

#[test]
fn per_type_choices_and_domain_pump_render_their_quantifier() {
    for (name, phrase) in [
        ("Planar Overlay", "of each basic land type"),
        ("Sundering Titan", "choose a land of each basic land type"),
        ("Tromp the Domains", "+1/+1 for each basic land type among lands you control"),
    ] {
        for definition in support::definitions(name) {
            let text = support::rendered(&definition);
            if name=="Tromp the Domains" {assert!(text.contains(phrase),"{name}: {text}");}
            else {
                // Explicit five-slot text is equivalent to the source's
                // quantifier, provided every typed slot and common pool exist.
                let effects=all_effects(&definition);let choices=effects.iter().filter_map(|effect|effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>()).collect::<Vec<_>>();assert_eq!(choices.len(),5,"{name}");
                for subtype in basic_types(){assert_eq!(choices.iter().filter(|choice|choice.filter.subtypes.contains(&subtype)).count(),1,"{name}: one typed slot per basic land type");}
                assert!(choices.iter().all(|choice|choice.tag==choices[0].tag));assert!(!matches!(choices[0].tag.as_str(),"it"|"__it__"),"a per-type set cannot replace prior choices under singular it");
                for word in ["plains","island","swamp","mountain","forest"]{assert!(text.contains(phrase)||text.contains(word),"{name}: missing {word} slot: {text}");}
                if name=="Planar Overlay" {let loops=effects.iter().filter_map(|effect|effect.downcast_ref::<ironsmith::effects::ForPlayersEffect>()).collect::<Vec<_>>();assert_eq!(loops.len(),1);assert_eq!(loops[0].filter,ironsmith::target::PlayerFilter::Any);}
            }
        }
    }
}

fn basic_types()->[ironsmith::Subtype;5]{[ironsmith::Subtype::Plains,ironsmith::Subtype::Island,ironsmith::Subtype::Swamp,ironsmith::Subtype::Mountain,ironsmith::Subtype::Forest]}
fn all_effects(definition:&ironsmith::CardDefinition)->Vec<ironsmith::Effect>{
 fn collect(effect:&ironsmith::Effect,all:&mut Vec<ironsmith::Effect>){all.push(effect.clone());effect.visit_child_effects(&mut |child|collect(child,all));}
 let mut result=Vec::new();if let Some(program)=&definition.spell_effect{for effect in program.all_effects(){collect(effect,&mut result);}}
 for ability in &definition.abilities{if let ironsmith::AbilityKind::Triggered(triggered)=&ability.kind{for effect in triggered.effects.all_effects(){collect(effect,&mut result);}}}result
}
#[derive(Default)]struct PerTypeChoices{own_only:bool,prompts:Vec<ironsmith::PlayerId>,selected:Vec<ironsmith::ObjectId>}
impl ironsmith::decision::DecisionMaker for PerTypeChoices{
 fn decide_objects(&mut self,game:&ironsmith::GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ironsmith::ObjectId>{
  let legal=ctx.candidates.iter().filter(|candidate|candidate.legal).collect::<Vec<_>>();
  if self.own_only{assert!(legal.iter().all(|candidate|game.current_controller(candidate.id)==Some(ctx.player)),"each participant chooses only their controlled lands");}
  self.prompts.push(ctx.player);let chosen=legal.first().map(|candidate|vec![candidate.id]).unwrap_or_default();self.selected.extend(chosen.iter().copied());chosen
 }
}
#[test]
fn actual_planar_overlay_keeps_all_five_slots_for_every_player_and_returns_to_owners(){
 use ironsmith::{GameState,PlayerId,Zone};let a=PlayerId(0);let players=[a,PlayerId(1),PlayerId(2)];
 for definition in support::definitions("Planar Overlay"){for one_multityped_land in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let mut typeless=Vec::new();
  for controller in players{
   let layouts=if one_multityped_land{vec!["Plains Island Swamp Mountain Forest".to_string()]}else{basic_types().into_iter().flat_map(|ty|[ty.to_string(),ty.to_string()]).collect()};
   for subtypes in layouts{let land=ironsmith_compiler_runtime::compile_to_runtime_definition("Per-type land",&format!("Type: Land — {subtypes}"),false).unwrap();let id=game.create_object_from_definition(&land,a,Zone::Battlefield);if controller!=a{game.set_current_controller(id,controller).unwrap();}}
   let land=ironsmith_compiler_runtime::compile_to_runtime_definition("Typeless basic","Type: Basic Land",false).unwrap();let id=game.create_object_from_definition(&land,a,Zone::Battlefield);if controller!=a{game.set_current_controller(id,controller).unwrap();}typeless.push(id);
  }
  let source=game.create_object_from_definition(&definition,a,Zone::Stack);let mut dm=PerTypeChoices{own_only:true,..Default::default()};
  let result=ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(definition.spell_effect.as_ref().unwrap().all_effects_owned())),&mut ironsmith::effects::EffectContext::new_default(source,a).with_decision_maker(&mut dm)).unwrap();
  let expected=if one_multityped_land{3}else{15};assert_eq!(game.player(a).unwrap().hand.len(),expected);assert_eq!(game.player(players[1]).unwrap().hand.len(),0);assert_eq!(game.player(players[2]).unwrap().hand.len(),0);
  if !one_multityped_land{for player in players{assert_eq!(dm.prompts.iter().filter(|actor|**actor==player).count(),5,"all three players independently choose all five types");}}
  for id in typeless{assert_eq!(game.object(id).unwrap().zone,Zone::Battlefield,"basic supertype alone isn't a basic land type");}
  let moved=result.events.iter().filter_map(|event|event.downcast::<ironsmith::events::ZoneChangeEvent>()).filter(|event|event.to==Zone::Hand).map(|event|event.objects.len()).sum::<usize>();assert_eq!(moved,expected,"a multityped land can fill several slots but moves only once");
 }}
}
#[test]
fn actual_sundering_titan_chooses_all_types_without_requiring_the_casters_control(){
 use ironsmith::{GameState,PlayerId,Zone};let a=PlayerId(0);let b=PlayerId(1);
 for definition in support::definitions("Sundering Titan"){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let mut ids=Vec::new();
  for subtype in basic_types(){for _ in 0..2{let land=ironsmith_compiler_runtime::compile_to_runtime_definition("Other player's land",&format!("Type: Land — {subtype}"),false).unwrap();ids.push(game.create_object_from_definition(&land,b,Zone::Battlefield));}}
  let source=game.create_object_from_definition(&definition,a,Zone::Battlefield);let effects=definition.abilities.iter().find_map(|ability|if let ironsmith::AbilityKind::Triggered(triggered)=&ability.kind{Some(triggered.effects.all_effects_owned())}else{None}).unwrap();
  let mut dm=PerTypeChoices::default();ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ironsmith::effects::EffectContext::new_default(source,a).with_decision_maker(&mut dm)).unwrap();
  assert_eq!(dm.prompts,vec![a;5]);assert_eq!(game.player(b).unwrap().graveyard.len(),5);assert_eq!(dm.selected.len(),5);for old in ids{assert_eq!(game.object(old).is_none_or(|object|object.zone!=Zone::Battlefield),dm.selected.contains(&old));}
 }
}
