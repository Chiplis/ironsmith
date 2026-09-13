use super::*;
use crate::card::{CardBuilder, PowerToughness, PtValue};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};

fn fixture() -> (GameState, ObjectId, ObjectId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Subject fixture")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .power_toughness(PowerToughness::fixed(2, 3))
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let candidate = game.create_object_from_card(&card, alice, Zone::Battlefield);
    (game, source, candidate, alice)
}

#[test]
fn shared_subject_preserves_captured_characteristics_and_nested_current_state() {
    let (mut game, source, candidate, alice) = fixture();
    let snapshot = ObjectSnapshot::from_object(game.object(candidate).unwrap(), &game);
    {
        let object = game.object_mut(candidate).unwrap();
        object.card_types = vec![CardType::Artifact].into();
        object.subtypes = vec![Subtype::Equipment].into();
        object.color_override = Some(ColorSet::RED);
        object.base_power = Some(PtValue::Fixed(5));
    }
    game.tap(candidate);
    let ctx = FilterContext::new(alice).with_source(source);
    let retained = ObjectFilter {
        card_types: vec![CardType::Creature],
        subtypes: vec![Subtype::Elf],
        colors: Some(ColorSet::GREEN),
        power: Some(Comparison::Equal(2)),
        untapped: true,
        ..Default::default()
    };
    assert!(retained.matches_snapshot(&snapshot, &ctx, &game));
    assert!(!retained.matches(game.object(candidate).unwrap(), &ctx, &game));
    let current = ObjectFilter {
        card_types: vec![CardType::Artifact],
        colors: Some(ColorSet::RED),
        tapped: true,
        match_current_state: true,
        ..Default::default()
    };
    // Redirection must work inside a union, not just at the outer entry point.
    let union = ObjectFilter {
        any_of: vec![current],
        ..Default::default()
    };
    assert!(union.matches_snapshot(&snapshot, &ctx, &game));
    game.move_object_by_effect(candidate, Zone::Graveyard);
    assert!(!union.matches_snapshot(&snapshot, &ctx, &game));
    assert!(retained.matches_snapshot(&snapshot, &ctx, &game));
}

#[test]
fn shared_subject_keeps_live_only_predicate_policy() {
    let (game, source, candidate, alice) = fixture();
    let snapshot = ObjectSnapshot::from_object(game.object(candidate).unwrap(), &game);
    let ctx = FilterContext::new(alice).with_source(source);
    // These are existing differences, not changes to rule support in this refactor.
    let filters = [
        ObjectFilter {
            modified: true,
            ..Default::default()
        },
        ObjectFilter {
            blocking: true,
            ..Default::default()
        },
        ObjectFilter {
            drawn_this_turn: true,
            ..Default::default()
        },
        ObjectFilter {
            surveilled_this_turn: true,
            ..Default::default()
        },
        ObjectFilter {
            dealt_damage_this_turn: true,
            ..Default::default()
        },
        ObjectFilter {
            entered_graveyard_this_turn: true,
            ..Default::default()
        },
    ];
    for filter in filters {
        assert!(
            !filter.matches(game.object(candidate).unwrap(), &ctx, &game),
            "{filter:?}"
        );
        assert!(
            filter.matches_snapshot(&snapshot, &ctx, &game),
            "{filter:?}"
        );
    }
    let union = ObjectFilter {
        type_or_subtype_union: true,
        card_types: vec![CardType::Artifact],
        subtypes: vec![Subtype::Elf],
        ..Default::default()
    };
    assert!(union.matches(game.object(candidate).unwrap(), &ctx, &game));
    assert!(!union.matches_snapshot(&snapshot, &ctx, &game));
}

#[test]
fn shared_subject_retains_snapshot_cast_order_without_a_live_stack_entry() {
    let (mut game, source, candidate, alice) = fixture();
    let spell = game.move_object_by_effect(candidate, Zone::Stack).unwrap();
    let mut snapshot = ObjectSnapshot::from_object(game.object(spell).unwrap(), &game);
    snapshot.cast_order_this_turn = Some(1);
    let mut ctx = FilterContext::new(alice).with_source(source);
    ctx.caster = Some(alice);
    let filter = ObjectFilter {
        zone: Some(Zone::Stack),
        stack_kind: Some(StackObjectKind::Spell),
        cast_this_turn: true,
        ..Default::default()
    };
    assert!(filter.matches_snapshot(&snapshot, &ctx, &game));
    assert!(!filter.matches(game.object(spell).unwrap(), &ctx, &game));
    snapshot.cast_order_this_turn = None;
    assert!(!filter.matches_snapshot(&snapshot, &ctx, &game));
}

#[test]
fn shared_subject_other_retains_stable_identity_after_zone_change() {
    let (mut game, _, candidate, alice) = fixture();
    let snapshot = ObjectSnapshot::from_object(game.object(candidate).unwrap(), &game);
    let new_id = game
        .move_object_by_effect(candidate, Zone::Graveyard)
        .unwrap();
    let ctx = FilterContext::new(alice).with_source(new_id);
    let other = ObjectFilter {
        other: true,
        ..Default::default()
    };
    assert!(!other.matches_snapshot(&snapshot, &ctx, &game));
    assert!(!other.matches(game.object(new_id).unwrap(), &ctx, &game));
}
