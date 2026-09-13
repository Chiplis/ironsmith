use super::*;
use crate::ability::AbilityKind;
use crate::card::PowerToughness;
use crate::game_state::{GameState,StackEntry,Target};
use crate::ids::{CardId,ObjectId};

fn body(name:&str,subtype:Subtype)->crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(CardId::new(),name).card_types(vec![CardType::Creature]).subtypes(vec![subtype]).power_toughness(PowerToughness::fixed(2,8)).build()
}
fn event(source:ObjectId)->crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(crate::events::zones::ZoneChangeEvent::with_cause(source,Zone::Stack,Zone::Battlefield,crate::events::cause::EventCause::effect(),None),crate::provenance::ProvNodeId::default())
}
struct Accept;
impl crate::decision::DecisionMaker for Accept {
    fn decide_boolean(&mut self,_:&GameState,_:&crate::decisions::context::BooleanContext)->bool {true}
}
fn queue(game:&mut GameState,event:crate::triggers::TriggerEvent,dm:&mut dyn crate::decision::DecisionMaker)->usize {
    let ts=crate::triggers::check_triggers(game,&event);let n=ts.len();let mut q=crate::triggers::TriggerQueue::new();for t in ts {q.add(t);}
    crate::game_loop::put_triggers_on_stack_with_dm(game,&mut q,dm).unwrap();n
}
fn mana_abilities(game:&GameState,id:ObjectId)->usize {
    game.current_abilities(id).unwrap().iter().filter(|a|matches!(&a.kind,AbilityKind::Activated(a) if a.is_mana_ability())).count()
}

#[test]
fn cohort_entry_counters_are_optional_distinct_targets_and_mana_grant_is_live() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Ravelon Renegade").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2))
        .parse_text("When this creature enters, put a +1/+1 counter on each of up to two target creatures.\nEach creature you control with a counter on it has \"{T}: Add {G}.\"").unwrap();
    struct Choice(Vec<Target>);
    impl crate::decision::DecisionMaker for Choice {fn decide_targets(&mut self,_:&GameState,ctx:&crate::decisions::context::TargetsContext)->Vec<Target>{assert_eq!(ctx.requirements[0].min_targets,0);assert_eq!(ctx.requirements[0].max_targets,Some(2));self.0.clone()}}
    for count in 0..=2 {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let (alice,bob)=(game.players[0].id,game.players[1].id);
        let source=game.create_object_from_definition(&card,alice,Zone::Battlefield);
        let own=game.create_object_from_definition(&body("Own",Subtype::Human),alice,Zone::Battlefield);
        let enemy=game.create_object_from_definition(&body("Enemy",Subtype::Elf),bob,Zone::Battlefield);
        let artifact=crate::CardDefinitionBuilder::new(CardId::new(),"Artifact").card_types(vec![CardType::Artifact]).build();let artifact=game.create_object_from_definition(&artifact,alice,Zone::Battlefield);
        let targets=[Target::Object(source),Target::Object(enemy)][..count].to_vec();
        assert_eq!(queue(&mut game,event(source),&mut Choice(targets)),1);crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.counter_count(source,crate::CounterType::PlusOnePlusOne),u32::from(count>0));assert_eq!(game.counter_count(enemy,crate::CounterType::PlusOnePlusOne),u32::from(count>1));
        assert_eq!(mana_abilities(&game,source),usize::from(count>0));assert_eq!(mana_abilities(&game,enemy),0);assert_eq!(mana_abilities(&game,own),0);
        game.add_counters(own,crate::CounterType::Shield,1);game.add_counters(artifact,crate::CounterType::Shield,1);
        assert_eq!(mana_abilities(&game,own),1);assert_eq!(mana_abilities(&game,artifact),0);
        game.object_mut(own).unwrap().counters.clear();assert_eq!(mana_abilities(&game,own),0);
        game.add_counters(own,crate::CounterType::Shield,1);game.move_object_by_effect(source,Zone::Graveyard).unwrap();assert_eq!(mana_abilities(&game,own),0);
    }
}

#[test]
fn cohort_level_bands_gate_mana_and_grant_only_to_current_controllers_elves() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Ravelon Speaker").card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Elf]).power_toughness(PowerToughness::fixed(1,1))
        .parse_text("Level up {1}{G}\nLEVEL 1-4\n1/2\n{T}: Add {G}{G}.\nLEVEL 5+\n1/4\nElves you control have \"{T}: Add {G}{G}.\"").unwrap();
    let text=crate::compiled_text::compiled_text_lines(&card).join("\n");
    assert!(!text.contains("Activate only if"),"{text}");
    for level in [0,1,4,5,6] {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let (alice,bob)=(game.players[0].id,game.players[1].id);
        let source=game.create_object_from_definition(&card,alice,Zone::Battlefield);
        let elf=game.create_object_from_definition(&body("Own elf",Subtype::Elf),alice,Zone::Battlefield);
        let enemy=game.create_object_from_definition(&body("Enemy elf",Subtype::Elf),bob,Zone::Battlefield);
        let human=game.create_object_from_definition(&body("Own human",Subtype::Human),alice,Zone::Battlefield);
        game.add_counters(source,crate::CounterType::Level,level);
        for id in [source,elf,enemy,human] {game.remove_summoning_sickness(id);}
        assert_eq!(game.current_toughness(source),Some(if level==0{1}else if level<5{2}else{4}));
        let actions=crate::decision::compute_legal_actions(&game,alice);
        for (id,expected) in [(source,level>=1),(elf,level>=5),(human,false)] {
            assert_eq!(actions.iter().any(|a|matches!(a,crate::decision::LegalAction::ActivateManaAbility{source,..} if *source==id)),expected,"level={level} id={id:?}");
        }
        assert_eq!(mana_abilities(&game,enemy),0);
        if level>=5 {assert_eq!(mana_abilities(&game,elf),1);game.set_current_controller(source,bob);assert_eq!(mana_abilities(&game,elf),0);assert_eq!(mana_abilities(&game,enemy),1);}
    }
}

#[test]
fn cohort_saga_chapters_damage_exile_grant_temporary_play_and_add_six_mana() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Ravelon Flux").card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Saga])
        .parse_text("I — This Saga deals 4 damage to target creature an opponent controls.\nII, III, IV, V — Exile the top card of your library. You may play that card this turn.\nVI — Add six {R}.").unwrap();
    assert!(crate::compiled_text::compiled_text_lines(&card).join("\n").contains("VI — Add six {R}"));
    for chapter in 1..=6 {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let (alice,bob)=(game.players[0].id,game.players[1].id);
        let source=game.create_object_from_definition(&card,alice,Zone::Battlefield);
        let enemy=game.create_object_from_definition(&body("Enemy",Subtype::Human),bob,Zone::Battlefield);
        let top=game.create_object_from_definition(&body("Exiled card",Subtype::Human),alice,Zone::Library);let stable=game.object(top).unwrap().stable_id;
        game.object_mut(source).unwrap().counters.insert(crate::CounterType::Lore,chapter);
        let ev=crate::triggers::TriggerEvent::new_with_provenance(crate::events::other::CounterPlacedEvent::new(source,crate::CounterType::Lore,1),crate::provenance::ProvNodeId::default());
        assert_eq!(queue(&mut game,ev,&mut Accept),1);crate::game_loop::resolve_stack_entry_with(&mut game,&mut Accept).unwrap();
        assert_eq!(game.damage_on(enemy),if chapter==1{4}else{0});
        assert_eq!(game.player(alice).unwrap().mana_pool.total(),if chapter==6{6}else{0});
        if (2..=5).contains(&chapter) {
            let id=*game.exile.iter().find(|id|game.object(**id).unwrap().stable_id==stable).unwrap();
            assert!(game.effect_store.grant_registry.card_can_play_from_zone(&game,id,Zone::Exile,alice));assert!(!game.effect_store.grant_registry.card_can_play_from_zone(&game,id,Zone::Exile,bob));
            game.next_turn();assert!(!game.effect_store.grant_registry.card_can_play_from_zone(&game,id,Zone::Exile,alice));
        }else {assert!(game.exile.is_empty());}
    }
}

#[test]
fn cohort_spell_name_trigger_matches_own_graveyard_at_cast_time_and_copy_threshold_is_event_time() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Ravelon Ascension").card_types(vec![CardType::Enchantment])
        .parse_text("Whenever you cast an instant or sorcery spell that has the same name as a card in your graveyard, you may put a quest counter on this enchantment.\nWhenever you cast an instant or sorcery spell while this enchantment has two or more quest counters on it, you may copy that spell. You may choose new targets for the copy.").unwrap();
    for case in 0..7 {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let (alice,bob)=(game.players[0].id,game.players[1].id);
        let source=game.create_object_from_definition(&card,alice,Zone::Battlefield);
        let spell=crate::CardDefinitionBuilder::new(CardId::new(),"Named spell").card_types(vec![if case==2{CardType::Creature}else if case==1{CardType::Sorcery}else{CardType::Instant}]).parse_text("Draw a card.").unwrap();
        let grave=game.create_object_from_definition(&spell,if case==3{bob}else{alice},if case==4{Zone::Exile}else{Zone::Graveyard});
        let caster=if case==5{bob}else{alice};let id=game.create_object_from_definition(&spell,caster,Zone::Stack);game.stack.push(StackEntry::new(id,caster));
        if case==6 {game.add_counters(source,crate::CounterType::Quest,2);}
        game.take_pending_trigger_events();
        let snapshot=crate::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(),&game);
        let ev=crate::triggers::TriggerEvent::new_with_provenance(crate::events::SpellCastEvent::new_with_snapshot(id,caster,Zone::Hand,snapshot),crate::provenance::ProvNodeId::default());
        let expected=if case==6{2}else if case<=1{1}else{0};assert_eq!(queue(&mut game,ev,&mut Accept),expected,"case={case}");
        game.move_object_by_effect(grave,Zone::Hand).unwrap();
        if case==6 {game.object_mut(source).unwrap().counters.clear();}
        // Resolve triggers while leaving both the original spell and any copy on the stack.
        for _ in 0..expected {
            if !game.stack.last().unwrap().is_ability {break;}
            crate::game_loop::resolve_stack_entry_with(&mut game,&mut Accept).unwrap();
        }
        if case<=1 {assert_eq!(game.counter_count(source,crate::CounterType::Quest),1,"name relation must not be rechecked at resolution");}
        if case==6 {assert!(game.stack.iter().filter(|entry|!entry.is_ability).count()>=2,"cast-time threshold remains satisfied for the queued trigger");}
    }
}

#[test]
fn cohort_curse_search_compares_names_only_on_enchanted_player_without_targeting() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Ravelon Misfortune").card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura,Subtype::Curse])
        .parse_text("Enchant player\nAt the beginning of your upkeep, you may search your library for a Curse card that doesn't have the same name as a Curse attached to enchanted player, put it onto the battlefield attached to that player, then shuffle.").unwrap();
    struct Search {chosen:ObjectId,excluded:Vec<ObjectId>}
    impl crate::decision::DecisionMaker for Search {
        fn decide_boolean(&mut self,_:&GameState,_:&crate::decisions::context::BooleanContext)->bool{true}
        fn decide_objects(&mut self,_:&GameState,ctx:&crate::decisions::context::SelectObjectsContext)->Vec<ObjectId>{for id in &self.excluded{assert!(!ctx.candidates.iter().any(|c|c.id==*id&&c.legal));}assert!(ctx.candidates.iter().any(|c|c.id==self.chosen&&c.legal));vec![self.chosen]}
        fn decide_targets(&mut self,_:&GameState,_:&crate::decisions::context::TargetsContext)->Vec<Target>{panic!("the upkeep search does not target the enchanted player")}
    }
    let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let (alice,bob)=(game.players[0].id,game.players[1].id);
    let source=game.create_object_from_definition(&card,alice,Zone::Battlefield);game.attach_object_to_target(source,crate::object::AttachmentTarget::Player(bob));
    let shield=crate::CardDefinitionBuilder::new(CardId::new(),"Player shield").card_types(vec![CardType::Enchantment]).parse_text("You have hexproof.").unwrap();game.create_object_from_definition(&shield,bob,Zone::Battlefield);
    let curse=|name:&str|crate::CardDefinitionBuilder::new(CardId::new(),name).card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura,Subtype::Curse]).parse_text("Enchant player").unwrap();
    let existing=game.create_object_from_definition(&curse("Existing curse"),alice,Zone::Battlefield);game.attach_object_to_target(existing,crate::object::AttachmentTarget::Player(bob));
    let other=game.create_object_from_definition(&curse("Other player's curse"),alice,Zone::Battlefield);game.attach_object_to_target(other,crate::object::AttachmentTarget::Player(alice));
    let duplicate=game.create_object_from_definition(&curse("Existing curse"),alice,Zone::Library);let source_duplicate=game.create_object_from_definition(&card,alice,Zone::Library);
    let chosen=game.create_object_from_definition(&curse("Other player's curse"),alice,Zone::Library);let stable=game.object(chosen).unwrap().stable_id;
    let noncurse=game.create_object_from_definition(&body("Not a Curse",Subtype::Human),alice,Zone::Library);
    let mut dm=Search{chosen,excluded:vec![duplicate,source_duplicate,noncurse]};
    let ev=crate::triggers::TriggerEvent::new_with_provenance(crate::events::phase::BeginningOfUpkeepEvent::new(alice),crate::provenance::ProvNodeId::default());
    assert_eq!(queue(&mut game,ev,&mut dm),1);assert_eq!(game.stack.len(),1);crate::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
    let entered=*game.battlefield.iter().find(|id|game.object(**id).unwrap().stable_id==stable).unwrap();assert_eq!(game.object(entered).unwrap().attached_to,Some(crate::object::AttachmentTarget::Player(bob)));
    assert_eq!(game.player(alice).unwrap().library.len(),3);
}

#[test]
fn cohort_blocked_creature_bonus_locks_recipients_and_survives_departed_blockers() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Ravelon Witch").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,1))
        .parse_text("{R}{G}: Each blocked creature gets +1/+0 and gains trample until end of turn.").unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&card).join("\n"), "{R}{G}: Each blocked creature gets +1/+0 and gains trample until end of turn.");
    let AbilityKind::Activated(ability)=&card.abilities[0].kind else{panic!("activation")};
    for enemy_attacks in [false,true] {for blocked in 0..=2 {for blocker_phases in [false,true] {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let (alice,bob)=(game.players[0].id,game.players[1].id);
        let (attacker_controller,defender)=if enemy_attacks{(bob,alice)}else{(alice,bob)};
        let source=game.create_object_from_definition(&card,alice,Zone::Battlefield);
        let attackers=(0..3).map(|_|game.create_object_from_definition(&body("Attacker",Subtype::Human),attacker_controller,Zone::Battlefield)).collect::<Vec<_>>();
        let blockers=(0..2).map(|_|game.create_object_from_definition(&body("Blocker",Subtype::Human),defender,Zone::Battlefield)).collect::<Vec<_>>();
        let mut combat=crate::combat_state::CombatState::default();
        combat.attackers=attackers.iter().map(|id|crate::combat_state::AttackerInfo{creature:*id,target:crate::combat_state::AttackTarget::Player(defender)}).collect();
        crate::combat_state::declare_blockers(&game,&mut combat,(0..blocked).map(|i|(blockers[i],attackers[i])).collect()).unwrap();
        game.combat=Some(combat);
        if blocker_phases&&blocked>0 {game.phase_out(blockers[0]);}
        game.stack.push(StackEntry::ability(source,alice,ability.effects.clone()));crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        for (i,id) in attackers.iter().enumerate() {
            assert_eq!(game.current_power(*id),Some(if i<blocked{3}else{2}),"blocked={blocked} phased={blocker_phases} i={i}");
            assert_eq!(game.current_has_static_ability_id(*id,crate::static_abilities::StaticAbilityId::Trample),i<blocked);
        }
        assert_eq!(game.current_power(source),Some(2));
        if blocker_phases&&blocked>0 {game.phase_in(blockers[0]);}
        for id in blockers {assert_eq!(game.current_power(id),Some(2));}
        game.combat=None;
        for (i,id) in attackers.iter().enumerate(){assert_eq!(game.current_power(*id),Some(if i<blocked{3}else{2}));}
        crate::turn::execute_cleanup_step(&mut game);
        game.next_turn();
        for id in attackers {assert_eq!(game.current_power(id),Some(2));assert!(!game.current_has_static_ability_id(id,crate::static_abilities::StaticAbilityId::Trample));}
    }}}
}
