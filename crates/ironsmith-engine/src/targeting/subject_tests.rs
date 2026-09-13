use super::*;
use crate::ability::{Ability, ProtectionFrom};
use crate::card::{CardBuilder, PowerToughness};
use crate::color::ColorSet;
use crate::game_state::{ObjectCantBeTargetedFrom, TargetingAsThoughOverride};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::{StaticAbility, StaticAbilityId};

fn fixture(ability: StaticAbility, source_owner: PlayerId) -> (GameState, ObjectId, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Protection subject")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .build();
    let source = game.create_object_from_card(&card, source_owner, Zone::Battlefield);
    let target = game.create_object_from_card(&card, PlayerId::from_index(1), Zone::Battlefield);
    std::sync::Arc::make_mut(&mut game.object_mut(target).unwrap().abilities)
        .push(Ability::static_ability(ability));
    (game, source, target)
}

fn target_result(
    game: &GameState,
    source: ObjectId,
    target: ObjectId,
    snapshot: Option<&ObjectSnapshot>,
) -> TargetingResult {
    let view = crate::derived_view::DerivedGameView::new(game);
    can_target_object_with_view_and_source_snapshot(
        game,
        target,
        source,
        snapshot,
        PlayerId::from_index(0),
        &view,
    )
}

#[test]
fn shared_subject_targeting_prefers_live_source_then_retained_color() {
    let alice = PlayerId::from_index(0);
    for (ability, expected) in [
        (
            StaticAbility::protection(ProtectionFrom::Color(ColorSet::RED)),
            TargetingInvalidReason::HasProtection,
        ),
        (
            StaticAbility::hexproof_from(ObjectFilter {
                colors: Some(ColorSet::RED),
                ..Default::default()
            }),
            TargetingInvalidReason::HasHexproofFrom,
        ),
    ] {
        let (mut game, source, target) = fixture(ability, alice);
        let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        game.object_mut(source).unwrap().color_override = Some(ColorSet::GREEN);
        assert!(target_result(&game, source, target, Some(&snapshot)).is_legal());
        game.move_object_by_effect(source, Zone::Graveyard);
        assert_eq!(
            target_result(&game, source, target, Some(&snapshot)),
            TargetingResult::Invalid(expected)
        );
        assert!(target_result(&game, source, target, None).is_legal());
    }
}

#[test]
fn shared_subject_targeting_preserves_ignore_permission_controller_policy() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for allowed in [alice, bob] {
        let (mut game, source, target) = fixture(StaticAbility::shroud(), bob);
        game.effect_store
            .cant_effects
            .targeting_as_though_overrides
            .push(TargetingAsThoughOverride {
                objects: Some(ObjectFilter::creature()),
                players: None,
                allowed_source_controller: Some(allowed),
                ignored_ability: StaticAbilityId::Shroud,
                controller: alice,
                source,
            });
        let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        assert_eq!(
            target_result(&game, source, target, Some(&snapshot)).is_legal(),
            allowed == alice
        );
        game.move_object_by_effect(source, Zone::Graveyard);
        assert_eq!(
            target_result(&game, source, target, Some(&snapshot)).is_legal(),
            allowed == bob
        );
    }
}

#[test]
fn shared_subject_targeting_restrictions_use_retained_source_filter_facts() {
    let alice = PlayerId::from_index(0);
    let (mut game, source, target) = fixture(StaticAbility::hexproof(), alice);
    // Target belongs to Bob; permit hexproof so the source restriction decides.
    game.effect_store
        .cant_effects
        .targeting_as_though_overrides
        .push(TargetingAsThoughOverride {
            objects: Some(ObjectFilter::creature()),
            players: None,
            allowed_source_controller: None,
            ignored_ability: StaticAbilityId::Hexproof,
            controller: alice,
            source,
        });
    game.effect_store
        .cant_effects
        .cant_be_targeted_from
        .push(ObjectCantBeTargetedFrom {
            object: target,
            source_filter: ObjectFilter {
                colors: Some(ColorSet::RED),
                ..Default::default()
            },
            controller: alice,
        });
    let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    assert_eq!(
        target_result(&game, source, target, None),
        TargetingResult::Invalid(TargetingInvalidReason::CantBeTargeted)
    );
    game.object_mut(source).unwrap().color_override = Some(ColorSet::GREEN);
    assert!(target_result(&game, source, target, Some(&snapshot)).is_legal());
    game.move_object_by_effect(source, Zone::Graveyard);
    assert_eq!(
        target_result(&game, source, target, Some(&snapshot)),
        TargetingResult::Invalid(TargetingInvalidReason::CantBeTargeted)
    );
}
