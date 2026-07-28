#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn find_backref_damage(
    effect: &crate::effect::Effect,
) -> Option<&crate::effects::DealDamageEffect> {
    if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
        return find_backref_damage(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        return find_backref_damage(&tagged.effect);
    }
    if let Some(if_effect) = effect.downcast_ref::<IfEffect>() {
        return if_effect
            .then
            .iter()
            .chain(&if_effect.else_)
            .find_map(find_backref_damage);
    }
    let damage = effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    let crate::effect::Value::Count(filter) = damage.amount.unhinted() else {
        return None;
    };
    (filter.controller == Some(PlayerFilter::TargetPlayerOrControllerOfTarget)).then_some(damage)
}

fn find_backref_count_filter(effect: &crate::effect::Effect) -> Option<&ObjectFilter> {
    let damage = find_backref_damage(effect)?;
    let crate::effect::Value::Count(filter) = damage.amount.unhinted() else {
        unreachable!("back-reference damage was already checked as a count")
    };
    Some(filter)
}

fn goblin_lyre_loss_damage(definition: &crate::cards::CardDefinition) -> crate::effect::Effect {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .flat_map(|activated| activated.effects.flattened_default_effects())
        .find_map(find_backref_damage)
        .cloned()
        .map(crate::effect::Effect::new)
        .expect("Goblin Lyre should have controller-relative loss damage")
}

#[test]
fn goblin_lyre_keeps_the_target_controller_backref_out_of_the_counted_types() {
    let definition = parse_oracle_card_definition("Goblin Lyre");
    let count_filter = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .flat_map(|activated| activated.effects.flattened_default_effects())
        .find_map(find_backref_count_filter)
        .expect("Goblin Lyre should retain its target-controller count relation");

    assert_eq!(
        count_filter.card_types,
        [CardType::Creature],
        "{count_filter:#?}"
    );
    assert_eq!(
        count_filter.zone,
        Some(Zone::Battlefield),
        "{count_filter:#?}"
    );
    assert!(
        !count_filter.card_types.contains(&CardType::Planeswalker),
        "{count_filter:#?}"
    );

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Sacrifice this artifact: Flip a coin. If you win the flip, this artifact deals damage to target opponent or planeswalker equal to the number of creatures you control. If you lose the flip, this artifact deals damage to you equal to the number of creatures that opponent or that planeswalker's controller controls."
    );
}

#[test]
fn goblin_lyre_loss_count_tracks_the_selected_player_for_both_target_kinds() {
    let definition = parse_oracle_card_definition("Goblin Lyre");
    let loss_damage = goblin_lyre_loss_damage(&definition);

    for (case, target_planeswalker, expected_damage) in
        [("opponent", false, 3), ("planeswalker", true, 2)]
    {
        let mut game = crate::game_state::GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

        let creature = CardDefinitionBuilder::new(CardId::new(), "Counted Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        for _ in 0..2 {
            game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        }
        for _ in 0..3 {
            game.create_object_from_definition(&creature, charlie, Zone::Battlefield);
        }

        let artifact = CardDefinitionBuilder::new(CardId::new(), "Uncounted Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_definition(&artifact, bob, Zone::Battlefield);
        let walker = CardDefinitionBuilder::new(CardId::new(), "Target Planeswalker")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(5)
            .build();
        let walker_id = game.create_object_from_definition(&walker, bob, Zone::Battlefield);

        let selected = if target_planeswalker {
            crate::effects::ResolvedTarget::Object(walker_id)
        } else {
            crate::effects::ResolvedTarget::Player(charlie)
        };
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
            .with_targets(vec![selected]);
        ctx.snapshot_targets(&game);
        crate::effects::execute_effect(&mut game, &loss_damage, &mut ctx)
            .unwrap_or_else(|error| panic!("{case} loss damage should resolve: {error}"));

        assert_eq!(
            game.player(alice).expect("Alice should exist").life,
            20 - expected_damage,
            "{case} target must count only creatures controlled by the referenced player; other players' creatures and the planeswalker itself must not count"
        );
    }
}
