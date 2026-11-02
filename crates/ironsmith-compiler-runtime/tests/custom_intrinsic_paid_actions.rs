use ironsmith::{CardType, GameState, PlayerId, Zone};
use ironsmith::ability::AbilityKind;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::mana::ManaSymbol;
use ironsmith::special_actions::{SpecialAction, can_perform_check, perform};
#[path = "custom_final_next_wave_support.rs"] mod source_support;
const A: PlayerId = PlayerId::from_index(0);
fn definitions(name: &str, text: &str) -> [ironsmith::cards::CardDefinition; 2] {
    source_support::definitions(&serde_json::json!({"name":name,"executable_text":text}))
        .unwrap_or_else(|error|panic!("{name}: {error}"))
}
fn source_definitions(source: &serde_json::Value) -> [ironsmith::cards::CardDefinition; 2] {
    source_support::definitions(source).unwrap_or_else(|error|panic!("{}: {error}",source["name"]))
}

fn action(game: &GameState, source: ironsmith::ObjectId) -> SpecialAction {
    let index = game.current_characteristics(source).unwrap().abilities.iter().position(|ability| matches!(&ability.kind,
        AbilityKind::Static(rule) if matches!(rule.compiled_model().map(|model| &model.payload),
            Some(ironsmith_core::StaticAbilityPayload::IntrinsicImmediateAction { .. })))).unwrap();
    SpecialAction::PerformIntrinsicAction { source_id: source, ability_index: index }
}
#[test]
fn ordinary_description_compiles_to_static_permission_and_immediate_base_setting_on_both_routes() {
    let text="Mana cost: {G}\nType: Creature\nPower/Toughness: 1/3\nAny time you could cast a sorcery, you may pay {1} to take a special action. If you do, this permanent has base power and toughness 4/4.";
    for definition in definitions("Immediate action probe", text) {
        assert!(!definition.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Activated(_))));
        let mut game = GameState::new(vec!["Alice".into(),"Bob".into()],20);
        game.turn.priority_player=Some(A); game.turn.phase=ironsmith::game_state::Phase::FirstMain;
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue,1);
        let action=action(&game,source);
        assert!(can_perform_check(&action,&game,A).is_ok());
        perform(action,&mut game,A,&mut SelectFirstDecisionMaker).unwrap();
        assert!(game.stack.is_empty());
        assert_eq!((game.calculated_power(source),game.calculated_toughness(source)),(Some(4),Some(4)));
        assert_eq!(game.current_card_types(source),Some(vec![CardType::Creature]));
    }
}
#[test]
fn every_source_rhapsody_and_stardom_face_compiles_through_direct_and_artifact_routes() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-intrinsic-characteristics-next.json")).unwrap();
    let rows=fixture["mechanics"].as_array().unwrap(); assert_eq!(rows.len(),31);
    let mut failures=Vec::new();
    for row in rows {
        for card in row["representative_cards"].as_array().unwrap() {
            for face in card["executable_faces"].as_array().unwrap() {
                match source_support::definitions(face) {
                    Ok(definitions) => for definition in definitions {
                        if ironsmith::cards::generated_definition_has_unimplemented_content(&definition) { failures.push(format!("{}: unimplemented compiled content",face["name"])); }
                    },
                    Err(error) => failures.push(format!("{}: {error}",face["name"])),
                }
            }
        }
    }
    assert!(failures.is_empty(),"All source failures:\n{}",failures.join("\n"));
}

#[test]
fn animus_source_exception_is_priority_timing_and_stardom_use_limit_is_structural() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-intrinsic-characteristics-next.json")).unwrap();
    for id in ["msem-809","msem-953"] {
        let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]==id).unwrap();
        let card=&row["representative_cards"][0];
        for definition in source_definitions(card) {
            let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
            game.turn.priority_player=Some(A); game.turn.phase=ironsmith::game_state::Phase::Combat;
            let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue,30);
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green,30);
            let action=action(&game,source);
            if id=="msem-809" {
                assert!(can_perform_check(&action,&game,A).is_ok(),"explicit instant window overrides default sorcery restriction");
            } else {
                assert!(can_perform_check(&action,&game,A).is_err(),"not attacking");
                let mut combat=ironsmith::combat_state::CombatState::default();
                combat.attackers.push(ironsmith::combat_state::AttackerInfo {creature:source,target:ironsmith::combat_state::AttackTarget::Player(PlayerId::from_index(1))});game.combat=Some(combat);
                perform(action.clone(),&mut game,A,&mut SelectFirstDecisionMaker).unwrap();
                assert_eq!(game.calculated_power(source),Some(4),"printed 8/8 becomes base 4/4");
                game.turn.turn_number+=1;
                assert!(can_perform_check(&action,&game,A).is_err());
            }
        }
    }
}
#[test]
fn serin_compound_grant_supplies_hand_price_and_attacking_static_action_without_battlefield_cast_permission() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-intrinsic-characteristics-next.json")).unwrap();
    let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]=="msem-964").unwrap();
    let card=&row["representative_cards"][0];
    for definition in source_definitions(card) {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
        game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;
        let source=game.create_object_from_definition(&definition,A,Zone::Hand);
        let stable=game.object(source).unwrap().stable_id;
        let mut dm=SelectFirstDecisionMaker;
        let entry={
            let mut ctx=ironsmith::effects::EffectContext::new(source,A,&mut dm);
            ironsmith::effects::execute_effect(&mut game,&ironsmith::effect::Effect::put_onto_battlefield(
                ironsmith::target::ChooseSpec::SpecificObject(source),false,ironsmith::target::PlayerFilter::You),&mut ctx).unwrap()
        };
        for event in entry.events { game.queue_trigger_event(Default::default(),event); }
        let mut queue=ironsmith::triggers::TriggerQueue::new();
        for step in 0..40 {
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
            if game.stack_is_empty() { break; }
            assert!(step<39,"entry work did not settle");
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        }
        let entered=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(entered).unwrap().zone,Zone::Battlefield);
        assert_eq!(game.counter_count(entered,ironsmith::object::CounterType::PlusOnePlusOne),3,"Serin's entry replacement must finish");
        let receiver=ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Grant receiver")
            .card_types(vec![CardType::Creature]).mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Red]))
            .power_toughness(ironsmith::card::PowerToughness::fixed(1,3)).build();
        let hand=game.create_object_from_definition(&receiver,A,Zone::Hand);
        let battlefield=game.create_object_from_definition(&receiver,A,Zone::Battlefield);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue,10);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green,10);
        let prices=game.effect_store.grant_registry.granted_alternative_casts_for_card(&game,hand,Zone::Hand,A);
        assert!(prices.iter().any(|price|price.method.battlefield_instructions().casting_power_toughness==Some(ironsmith::card::PowerToughness::fixed(4,4))));
        let legal=ironsmith::decision::compute_legal_actions(&game,A).unwrap();
        assert!(legal.iter().any(|legal|matches!(legal,ironsmith::decision::LegalAction::CastSpell {spell_id,casting_method,..} if *spell_id==hand&&!matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Normal))));
        assert!(!legal.iter().any(|legal|matches!(legal,ironsmith::decision::LegalAction::SpecialAction(SpecialAction::PerformIntrinsicAction {source_id,..}) if *source_id==battlefield)));
        game.turn.phase=ironsmith::game_state::Phase::Combat;
        let mut combat=ironsmith::combat_state::CombatState::default();combat.attackers.push(ironsmith::combat_state::AttackerInfo {creature:battlefield,target:ironsmith::combat_state::AttackTarget::Player(PlayerId::from_index(1))});game.combat=Some(combat);game.tap(battlefield);let _=game.object_mut(battlefield);
        let legal=ironsmith::decision::compute_legal_actions(&game,A).unwrap();
        assert!(!legal.iter().any(|legal|matches!(legal,ironsmith::decision::LegalAction::CastSpell {spell_id,..} if *spell_id==battlefield)),"granting a battlefield action does not authorize casting the permanent");
        let action=action(&game,battlefield);
        perform(action,&mut game,A,&mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.calculated_power(battlefield),Some(4));
        assert!(game.current_supertypes(battlefield).unwrap().contains(&ironsmith::types::Supertype::Legendary));
        assert!(game.stack.is_empty());
    }
}

#[test]
fn supplied_source_vocabulary_is_transported_on_both_routes_and_stays_local() {
    let source=serde_json::json!({"name":"Local source vocabulary probe",
        "executable_text":"Type: Creature — PrivatePerformer\nPower/Toughness: 1/1",
        "custom_subtypes":[{"singular":"PrivatePerformer","plural":"PrivatePerformers","family":"Creature"}]});
    for definition in source_definitions(&source) { assert_eq!(definition.card.subtypes.len(),1); }
    let mut undeclared=source;undeclared.as_object_mut().unwrap().remove("custom_subtypes");
    assert!(source_support::definitions(&undeclared).is_err(),"a previous source's vocabulary must not become process-global admission");
}

#[test]
fn imperative_player_loop_keeps_optional_caster_separate_from_graveyard_owner() {
    fn find_cast<'a>(effects:&'a [ironsmith::Effect])->Option<&'a ironsmith::effects::MayCastMatchingSpellWithoutPayingManaCostEffect> {
        for effect in effects {
            if let Some(cast)=effect.downcast_ref::<ironsmith::effects::MayCastMatchingSpellWithoutPayingManaCostEffect>() { return Some(cast); }
            if let Some(optional)=effect.downcast_ref::<ironsmith::effects::MayEffect>() {
                if let Some(cast)=find_cast(&optional.effects) { return Some(cast); }
            }
        }
        None
    }
    let body="Type: Sorcery\nFor each player, you may cast an instant or sorcery card from that player's graveyard without paying its mana cost.";
    for definition in definitions("Participant-key casting probe",body) {
        let program=definition.spell_effect.as_ref().unwrap();
        let effects=program.all_effects();
        let loop_effect=effects.iter().find_map(|effect|effect.downcast_ref::<ironsmith::effects::ForPlayersEffect>()).unwrap();
        assert_eq!(loop_effect.filter,ironsmith::target::PlayerFilter::Any);
        assert!(loop_effect.sequential);
        let cast=find_cast(&loop_effect.effects).unwrap();
        assert_eq!(cast.player,ironsmith::target::PlayerFilter::You);
        assert_eq!(cast.zone_owner,ironsmith::target::PlayerFilter::IteratedPlayer);
        assert_eq!(cast.zone,Zone::Graveyard);
    }
}
