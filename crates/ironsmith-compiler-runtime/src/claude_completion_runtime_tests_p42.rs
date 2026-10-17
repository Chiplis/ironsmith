//! Claude package 42 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

#[test]
fn froghemoth_exiles_up_to_the_combat_damage_amount_and_counts_exiled_types() {
    let definition = compile_to_runtime_definition(
        "Froghemoth",
        "Mana cost: {3}{G}{G}\nType: Creature — Frog Horror\nPower/Toughness: 4/4\nTrample, haste\nWhenever this creature deals combat damage to a player, exile up to that many target cards from their graveyard. Put a +1/+1 counter on this creature for each creature card exiled this way. You gain 1 life for each noncreature card exiled this way.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    // "up to that many" counts the combat damage dealt by the trigger event.
    assert!(debug.contains("WithCountValue"), "{debug}");
    assert!(debug.contains("EventValue(Amount)"), "{debug}");
    assert!(debug.contains("ExileEffect") || debug.contains("Exile"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    assert!(debug.contains("GainLifeEffect"), "{debug}");
}

fn p42_find_effects<T: Clone + 'static>(effect: &Effect, found: &mut Vec<T>) {
    if let Some(hit) = effect.downcast_ref::<T>() {
        found.push(hit.clone());
    }
    effect.visit_child_effects(&mut |child| p42_find_effects(child, found));
}

fn p42_spell_effects<T: Clone + 'static>(definition: &ironsmith::cards::CardDefinition) -> Vec<T> {
    let mut found = Vec::new();
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        p42_find_effects(effect, &mut found);
    }
    found
}

#[test]
fn galvanic_relay_permission_is_usable_only_during_your_next_turn() {
    use ironsmith::card::CardBuilder;
    use ironsmith::grant_registry::GrantSource;
    use ironsmith::ids::CardId;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition(
        "Galvanic Relay",
        "Mana cost: {2}{R}\nType: Sorcery\nExile the top card of your library. During your next turn, you may play that card.\nStorm (When you cast this spell, copy it for each spell cast before it this turn.)",
        false,
    )
    .unwrap();
    let grants = p42_spell_effects::<ironsmith::effects::GrantPlayTaggedEffect>(&definition);
    assert_eq!(grants.len(), 1, "{definition:#?}");
    let grant = grants[0].clone();
    assert_eq!(
        grant.duration,
        ironsmith::effects::GrantPlayTaggedDuration::DuringYourNextTurn
    );
    assert!(grant.allow_land, "play covers lands and spells");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Exiled card")
        .card_types(vec![CardType::Creature])
        .build();
    let exiled = game.create_object_from_card(&card, alice, Zone::Exile);
    let source = game.create_object_from_card(&card, alice, Zone::Stack);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ctx.set_tagged_objects(
        grant.tag.clone(),
        vec![ObjectSnapshot::from_object_id(&game, exiled).unwrap()],
    );
    ironsmith::effects::execute_effect(&mut game, &Effect::new(grant.clone()), &mut ctx).unwrap();
    let registered = game
        .effect_store
        .grant_registry
        .grants
        .iter()
        .find(|grant| {
            matches!(
                grant.source,
                GrantSource::EffectUntilPlayerNextTurnEnd { during_only: true, .. }
            )
        })
        .cloned()
        .expect("during-next-turn permission");
    assert!(!registered.source.is_valid(&game), "not usable for the rest of this turn");
    game.next_turn();
    assert!(!registered.source.is_valid(&game), "not usable on the opponent's turn");
    game.next_turn();
    assert!(game.is_active_player(alice));
    assert!(registered.source.is_valid(&game), "usable during your next turn");
    game.next_turn();
    assert!(!registered.source.is_valid(&game), "expired after that turn");
}

#[test]
fn flying_drone_colored_this_ability_reduction_folds_into_its_printed_cost() {
    use ironsmith::ability::AbilityKind;
    let definition = compile_to_runtime_definition(
        "Flying Drone",
        "Mana cost: {2}\nType: Artifact Creature — Robot Scout\nPower/Toughness: 1/1\nFlying, vigilance\n{1}{U}, {T}: Draw a card, then discard a card. This ability costs {1}{U} less to activate if another creature with flying entered the battlefield under your control this turn.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(
        definition.abilities.iter().any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
        "{debug}"
    );
    // {1}{U} minus {1}{U} is a free activation, gated on the ETB condition.
    assert!(debug.contains("ActivatedAbilityCostReduction"), "{debug}");
    assert!(debug.contains("replacement_mana_cost: Some("), "{debug}");
    assert!(debug.contains("reduction_mana: None"), "{debug}");
    assert!(debug.contains("ThisAbility"), "{debug}");
    assert!(debug.contains("entered_battlefield"), "{debug}");
}
