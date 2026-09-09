use super::*;
const TEXT: &str = "Whenever one or more Halflings you control attack a player, create a Food token.";
#[test]
fn subtype_attack_group_preserves_plural_noun() {
    for (noun, subtype) in [("Halflings", Subtype::Halfling), ("Dragons", Subtype::Dragon)] {
        let text = TEXT.replace("Halflings", noun);
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Attack group")
            .card_types(vec![CardType::Creature]).subtypes(vec![subtype]).parse_text(&text).unwrap();
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), text);
    }
}
#[test]
fn subtype_attack_group_creates_one_food_per_attacked_player() {
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Meriadoc Brandybuck")
        .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Halfling])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2)).parse_text(TEXT).unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()],20);
    let alice=game.players[0].id; let bob=game.players[1].id; let charlie=game.players[2].id;
    let source=game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    let walker=crate::card::CardBuilder::new(crate::ids::CardId::new(),"Walker").card_types(vec![CardType::Planeswalker]).build();
    let walker=game.create_object_from_card(&walker,bob,Zone::Battlefield);
    let mut combat=CombatState::default();
    // Opponent's Halfling, wrong subtype and planeswalker attack come first;
    // none may consume a later qualifying player's trigger group.
    for (owner,subtype,target) in [
        (bob,Subtype::Halfling,AttackTarget::Player(charlie)),
        (alice,Subtype::Human,AttackTarget::Player(bob)),
        (alice,Subtype::Halfling,AttackTarget::Planeswalker(walker)),
        (alice,Subtype::Halfling,AttackTarget::Player(bob)),
        (alice,Subtype::Halfling,AttackTarget::Player(bob)),
        (alice,Subtype::Halfling,AttackTarget::Player(charlie)),
    ] {
        let card=crate::card::CardBuilder::new(crate::ids::CardId::new(),"Attacker")
            .card_types(vec![CardType::Creature]).subtypes(vec![subtype])
            .power_toughness(crate::card::PowerToughness::fixed(2,2)).build();
        let creature=game.create_object_from_card(&card,owner,Zone::Battlefield);
        combat.attackers.push(AttackerInfo{creature,target});
    }
    game.combat=Some(combat.clone());
    let mut count=0;
    for (index,info) in combat.attackers.iter().enumerate() {
        let target=match info.target { AttackTarget::Player(p)=>crate::triggers::AttackEventTarget::Player(p), AttackTarget::Planeswalker(o)=>crate::triggers::AttackEventTarget::Planeswalker(o), _=>unreachable!() };
        let event=crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::combat::CreatureAttackedEvent::with_total_attackers(info.creature,target,6),crate::provenance::ProvNodeId::default());
        let triggers=crate::triggers::check_triggers(&game,&event);
        assert_eq!(triggers.len(),usize::from(index==3 || index==5),"attacker {index}");
        for trigger in triggers {
            count+=1;
            let mut ctx=crate::effects::EffectContext::new_default(source,alice);
            for effect in &trigger.ability.effects { crate::effects::execute_effect(&mut game,effect,&mut ctx).unwrap(); }
        }
    }
    assert_eq!(count,2);
    assert_eq!(game.objects_in_zone(Zone::Battlefield).into_iter().filter(|id|game.object(*id).unwrap().subtypes.contains(&Subtype::Food)).count(),2);
}
