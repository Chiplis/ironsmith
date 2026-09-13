//! Frozen atlas observation 24871566: costed Mayhem permission and identity.
use ironsmith::{GameState, PlayerId, ObjectId, Zone};
use ironsmith::decision::{LegalAction, compute_legal_actions, SelectFirstDecisionMaker};
use ironsmith::alternative_cast::CastingMethod;
fn setup() -> (GameState, PlayerId, ObjectId) {
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(), "Abomination, World Ravager").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(),"Bob".into()],20);
    game.turn.active_player=alice;
    game.turn.priority_player=Some(alice);
    game.turn.phase=ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,7);
    game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red,1);
    let card=game.create_object_from_definition(&definition,alice,Zone::Hand);
    (game,alice,card)
}
fn mayhem(game: &GameState, player: PlayerId, card: ObjectId) -> bool {
    compute_legal_actions(game,player).iter().any(|a| matches!(a,LegalAction::CastSpell {spell_id,from_zone:Zone::Graveyard,casting_method:CastingMethod::Alternative(0)} if *spell_id==card))
}
fn discard(game: &mut GameState, player: PlayerId, card: ObjectId) -> ObjectId {
    let stable=game.object(card).unwrap().stable_id;
    let fixture=ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(),"Discard fixture").card_types(vec![ironsmith::CardType::Artifact]).build();
    let source=game.create_object_from_definition(&fixture,player,Zone::Battlefield);
    game.push_to_stack(ironsmith::game_state::StackEntry::ability(source,player,vec![ironsmith::Effect::discard(1)]));
    ironsmith::game_loop::resolve_stack_entry_with(game,&mut SelectFirstDecisionMaker).unwrap();
    let card=game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(card).unwrap().zone,Zone::Graveyard);
    card
}
#[test]
fn mayhem_requires_discard_current_turn_and_sorcery_timing() {
    let (mut game,alice,hand)=setup();
    assert!(!mayhem(&game,alice,hand));
    let mut milled=game.clone();
    let grave=milled.move_object_by_effect(hand,Zone::Graveyard).unwrap();
    assert!(!mayhem(&milled,alice,grave),"a non-discard move does not enable Mayhem");
    let card=discard(&mut game,alice,hand);
    assert!(mayhem(&game,alice,card),"actual discard enables Mayhem");
    let mut later=game.clone(); later.next_turn(); later.turn.active_player=alice; later.turn.priority_player=Some(alice); later.turn.phase=ironsmith::game_state::Phase::FirstMain;
    assert!(!mayhem(&later,alice,card),"discard permission expires with the turn");
    let mut combat=game.clone(); combat.turn.phase=ironsmith::game_state::Phase::Combat;
    assert!(!mayhem(&combat,alice,card));
    let mut opponent=game.clone(); opponent.turn.active_player=PlayerId::from_index(1);
    assert!(!mayhem(&opponent,alice,card));
    let mut occupied=game.clone(); occupied.push_to_stack(ironsmith::game_state::StackEntry::ability(card,alice,vec![]));
    assert!(!mayhem(&occupied,alice,card),"Mayhem does not grant flash");
}
#[test]
fn mayhem_does_not_follow_a_card_out_of_the_graveyard() {
    let (mut game,alice,hand)=setup();
    let card=discard(&mut game,alice,hand);
    assert!(mayhem(&game,alice,card));
    let exile=game.move_object_by_effect(card,Zone::Exile).unwrap();
    let returned=game.move_object_by_effect(exile,Zone::Graveyard).unwrap();
    assert!(!mayhem(&game,alice,returned),"a new graveyard object was not discarded");
}
#[test]
fn mayhem_pays_alternative_cost_and_resolves_as_a_creature() {
    use ironsmith::decision::GameProgress;
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse};
    let (mut game,alice,hand)=setup();
    let card=discard(&mut game,alice,hand);
    let stable=game.object(card).unwrap().stable_id;
    let action=compute_legal_actions(&game,alice).into_iter().find(|a| matches!(a,LegalAction::CastSpell{spell_id,casting_method:CastingMethod::Alternative(0),..} if *spell_id==card)).unwrap();
    let mut queue=ironsmith::triggers::TriggerQueue::new();
    let mut state=PriorityLoopState::new(game.players_in_game());
    let mut dm=SelectFirstDecisionMaker;
    let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty(){break;}
        let GameProgress::NeedsDecisionCtx(ctx)=progress else {panic!("{progress:?}")};
        progress=ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
    }
    assert_eq!(game.stack.len(),1);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(),3,"pays 4R instead of 7R");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
    let permanent=game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(permanent).unwrap().zone,Zone::Battlefield);
    assert_eq!(game.calculated_power(permanent),Some(10));
    assert_eq!(game.calculated_toughness(permanent),Some(10));
    let died=game.move_object_by_effect(permanent,Zone::Graveyard).unwrap();
    game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red,5);
    assert!(!mayhem(&game,alice,died),"the resolved creature cannot reuse its old discard event after dying");
}
#[test]
fn canonical_menace_and_trample_combat() {
    use ironsmith::combat_state::{AttackTarget,CombatState,declare_attackers,declare_blockers};
    let (mut game,alice,hand)=setup();
    let source=game.move_object_with_etb_processing(hand,Zone::Battlefield).unwrap().new_id;
    let bob=PlayerId::from_index(1);
    game.remove_summoning_sickness(source);
    for toughness in [2,5,7] {
        let mut game=game.clone();
        let definition=ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(),"Blocker fixture")
            .card_types(vec![ironsmith::CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(1,toughness)).build();
        let first=game.create_object_from_definition(&definition,bob,Zone::Battlefield);
        let second=game.create_object_from_definition(&definition,bob,Zone::Battlefield);
        let mut combat=CombatState::default();
        declare_attackers(&mut game,&mut combat,vec![(source,AttackTarget::Player(bob))]).unwrap();
        assert!(declare_blockers(&game,&mut combat.clone(),vec![]).is_ok());
        assert!(declare_blockers(&game,&mut combat.clone(),vec![(first,source)]).is_err(),"menace disallows exactly one blocker");
        declare_blockers(&game,&mut combat,vec![(first,source),(second,source)]).unwrap();
        ironsmith::game_loop::execute_combat_damage_step(&mut game,&combat,false);
        assert_eq!(game.damage_on(first)+game.damage_on(second),(2*toughness).min(10) as u32);
        assert_eq!(game.damage_on(source),2);
        assert_eq!(game.player(bob).unwrap().life,20-(10-2*toughness).max(0));
        assert_eq!(game.player(alice).unwrap().life,20);
    }
}
#[test]
fn mayhem_permission_is_card_specific_and_requires_red_payment() {
    let (mut game,alice,hand)=setup();
    assert!(compute_legal_actions(&game,alice).iter().any(|a| matches!(a,LegalAction::CastSpell{spell_id,casting_method:CastingMethod::Normal,..} if *spell_id==hand)));
    let card=discard(&mut game,alice,hand);
    let payloads=ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(),"Abomination, World Ravager").unwrap();
    let definition=ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let other=game.create_object_from_definition(&definition,alice,Zone::Graveyard);
    assert!(mayhem(&game,alice,card));
    assert!(!mayhem(&game,alice,other),"another copy was not discarded");
    assert!(!mayhem(&game,PlayerId::from_index(1),card));
    game.player_mut(alice).unwrap().mana_pool = Default::default();
    game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,8);
    assert!(!mayhem(&game,alice,card),"generic mana cannot pay the required red");
    game.player_mut(alice).unwrap().mana_pool = Default::default();
    game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red,4);
    assert!(!mayhem(&game,alice,card),"four mana is insufficient");
    game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red,1);
    assert!(mayhem(&game,alice,card),"exactly five red can pay 4R");
}
