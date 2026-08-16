#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn assert_exact_oracle(name: &str, definition: &CardDefinition) {
    assert_eq!(
        canonical_compiled_lines(definition).join("\n"),
        oracle_text_by_name()[name]
    );
}

fn find_nested<T: Clone + 'static>(effect: &crate::effect::Effect) -> Option<T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested::<T>(child);
        }
    });
    found
}

#[test]
fn vivien_hidden_choice_and_creature_permission_share_one_exiled_tag() {
    let definition = parse_oracle_card_definition("Vivien, Champion of the Wilds");
    assert_exact_oracle("Vivien, Champion of the Wilds", &definition);

    let activated = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .find(|activated| {
            activated
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| {
                    find_nested::<crate::effects::GrantPlayTaggedEffect>(effect).is_some()
                })
        })
        .expect("Vivien's hidden-card ability");
    let effects = activated.effects.flattened_default_effects();
    let choose = effects
        .iter()
        .find_map(find_nested::<crate::effects::ChooseObjectsEffect>)
        .expect("one looked-card choice");
    let exile = effects
        .iter()
        .find_map(find_nested::<crate::effects::ExileEffect>)
        .expect("face-down exile");
    let grant = effects
        .iter()
        .find_map(find_nested::<crate::effects::GrantPlayTaggedEffect>)
        .expect("while-exiled cast permission");
    assert!(choose.count.is_single());
    assert!(exile.face_down);
    assert!(matches!(
        exile.spec.base(),
        ChooseSpec::Tagged(tag) if tag == &choose.tag
    ));
    assert_eq!(grant.tag.as_str(), crate::tag::SOURCE_EXILED_TAG);
    assert_eq!(
        grant.duration,
        crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
    );
    let filter = grant.filter.expect("creature-spell gate");
    assert_eq!(filter.card_types, [CardType::Creature]);
}

#[test]
fn agonizing_remorse_keeps_the_revealed_hand_and_target_opponents_graveyard_disjoint() {
    let definition = parse_oracle_card_definition("Agonizing Remorse");
    assert_exact_oracle("Agonizing Remorse", &definition);

    let program = definition.spell_effect.as_ref().expect("spell program");
    let effects = program.flattened_default_effects();
    let choose = effects
        .iter()
        .find_map(find_nested::<crate::effects::ChooseObjectsEffect>)
        .expect("cross-zone choice");
    assert!(choose.count.is_single());
    assert!(matches!(
        (choose.zone, choose.additional_zones.as_slice()),
        (Some(Zone::Hand), [Zone::Graveyard]) | (Some(Zone::Graveyard), [Zone::Hand])
    ));
    let hand = choose
        .filter
        .any_of
        .iter()
        .find(|arm| arm.zone == Some(Zone::Hand))
        .expect("revealed hand branch");
    assert_eq!(hand.excluded_card_types, [CardType::Land]);
    assert!(hand.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }));
    let graveyard = choose
        .filter
        .any_of
        .iter()
        .find(|arm| arm.zone == Some(Zone::Graveyard))
        .expect("opponent graveyard branch");
    assert!(matches!(
        graveyard.owner.as_ref(),
        Some(PlayerFilter::Target(player) | PlayerFilter::AliasedTarget(player))
            if player.as_ref() == &PlayerFilter::Opponent
    ));
    assert!(graveyard.excluded_card_types.is_empty());

    let exile = effects
        .iter()
        .find_map(find_nested::<crate::effects::MoveToZoneEffect>)
        .expect("chosen card exile");
    assert_eq!(exile.zone, Zone::Exile);
    assert!(matches!(
        exile.target.base(),
        ChooseSpec::Tagged(tag) if tag == &choose.tag
    ));
    let lose = effects
        .iter()
        .find_map(find_nested::<crate::effects::LoseLifeEffect>)
        .expect("one-life instruction");
    assert_eq!(lose.player, ChooseSpec::Player(PlayerFilter::You));
    assert_eq!(lose.amount.unhinted(), &crate::effect::Value::Fixed(1));
}
