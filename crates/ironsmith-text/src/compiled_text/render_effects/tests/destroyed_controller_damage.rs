use super::*;
use crate::CardDefinitionBuilder;
use crate::ids::{CardId, PlayerId};
const TEXT: &str = "Destroy X target artifacts. Builder's Bane deals damage to each player equal to the number of artifacts they controlled that were put into a graveyard this way.";

#[test]
fn destroyed_artifact_count_uses_last_controller_and_only_this_effect() {
    for protection in 0..5 {
        check_destroyed_count(protection);
    }
}

fn check_destroyed_count(protection: u8) {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Builder's Bane")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let first = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
    let second = game.create_object_from_definition(&artifact, bob, Zone::Battlefield);
    let protected_artifact = CardDefinitionBuilder::new(CardId::new(), "Protected Artifact")
        .card_types(vec![CardType::Artifact])
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::indestructible(),
        ))
        .build();
    let regenerating_artifact = CardDefinitionBuilder::new(CardId::new(), "Artifact Creature")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let borrowed = game.create_object_from_definition(
        if protection == 1 {
            &protected_artifact
        } else if protection == 2 {
            &regenerating_artifact
        } else {
            &artifact
        },
        alice,
        Zone::Battlefield,
    );
    game.set_current_controller(borrowed, bob);
    for _ in 0..4 {
        game.create_object_from_definition(&artifact, alice, Zone::Graveyard);
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_targets(
        vec![first, second, borrowed]
            .into_iter()
            .map(crate::effects::ResolvedTarget::Object)
            .collect(),
    );
    ctx.x_value = Some(if protection == 4 { 0 } else { 3 });
    if protection == 4 {
        ctx.targets.clear();
    }
    if protection == 2 {
        let regenerate = Effect::regenerate(
            ChooseSpec::Object(ObjectFilter::specific(borrowed)),
            Until::EndOfTurn,
        );
        crate::effects::execute_effect(&mut game, &regenerate, &mut ctx).unwrap();
    }
    if protection == 3 {
        let replacement = crate::effects::RegisterFutureZoneReplacementEffect::new(
            ObjectFilter::specific(borrowed),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            crate::effects::ReplacementApplyMode::OneShot,
        );
        crate::effects::execute_effect(&mut game, &Effect::new(replacement), &mut ctx).unwrap();
    }
    ctx.snapshot_targets(&game);
    for effect in definition.spell_effect.as_ref().unwrap() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(
        game.player(alice).unwrap().life,
        if protection == 4 { 20 } else { 19 }
    );
    assert_eq!(
        game.player(bob).unwrap().life,
        if protection == 0 {
            18
        } else if protection == 4 {
            20
        } else {
            19
        },
        "protection={protection}"
    );
    if protection == 1 || protection == 2 {
        assert_eq!(game.object(borrowed).unwrap().zone, Zone::Battlefield);
    }
    if protection == 3 {
        assert!(game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Artifact")
        }));
    }
}

#[test]
fn destroyed_controller_damage_text() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Builder's Bane")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        TEXT
    );
}
