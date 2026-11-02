//! cf8 p05: "<creature> attacks that player this combat if able" and
//! "attacks a player each combat if able" as a rule-effect requirement to
//! attack a specific player (CR 508.1d), Restriction::MustAttackPlayer.
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

const CLUSTER: &str = "attack_specific_player_requirement";

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 4);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn requirements_name_the_player_and_their_duration() {
    for name in ["Ruhan of the Fomori", "Raving Dead", "Ursine Monstrosity"] {
        support::assert_markers(CLUSTER, name, &["MustAttackPlayer", "EndOfCombat"]);
    }
    // The official text says `a player`, not an opponent of Nahiri's
    // controller. An affected opposing creature may be required to attack us.
    for definition in support::definitions(&support::row(CLUSTER,"Nahiri, the Unforgiving")) {
        let rules=program_support::find_all::<ironsmith::effects::CantEffect>(&definition);
        let requirements:Vec<_>=rules.iter().filter_map(|rule| match &rule.restriction {
            ironsmith::effect::Restriction::MustAttackPlayer { player, .. }=>Some((player,&rule.duration)),
            _=>None,
        }).collect();
        assert_eq!(requirements.len(),1,"exactly one player-attack requirement");
        assert_eq!(*requirements[0].0,ironsmith::target::PlayerFilter::Any);
        assert_eq!(*requirements[0].1,ironsmith::Until::YourNextTurn);
    }
}

#[path="cf8_p08/support.rs"] mod program_support;
#[path="cf8_p08/play.rs"] mod play;
#[test]
fn nahiri_requirement_can_include_her_controller_and_applies_only_to_the_announced_creature(){
 for definition in support::definitions(&support::row(CLUSTER,"Nahiri, the Unforgiving")) {
  let mut game=play::game();let nahiri=game.create_object_from_definition(&definition,play::A,ironsmith::Zone::Battlefield);
  let body=play::vanilla("Attack requirement witness","{0}","Soldier",2,2);let chosen=game.create_object_from_definition(&body,play::B,ironsmith::Zone::Battlefield);let unrelated=game.create_object_from_definition(&body,play::B,ironsmith::Zone::Battlefield);
  let mut dm=play::Script{targets:vec![ironsmith::Target::Object(chosen)],..Default::default()};play::activate(&mut game,nahiri,play::activated_at(&definition,0),&mut dm);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();game.refresh_continuous_state().unwrap();
  let required:std::collections::BTreeSet<_>=game.required_attack_players_this_turn(chosen).collect();
  assert!(required.contains(&play::A),"a creature controlled by B can be required to attack Nahiri's controller A");assert!(required.contains(&play::C),"the requirement also permits another legal player");
  assert!(game.can_attack_defending_player(chosen,play::A));assert!(game.can_attack_defending_player(chosen,play::C));assert!(!game.can_attack_defending_player(chosen,play::B),"the creature still cannot attack its own controller");
  assert!(game.required_attack_players_this_turn(unrelated).next().is_none(),"target binding cannot become a population-wide requirement");
 }
}
