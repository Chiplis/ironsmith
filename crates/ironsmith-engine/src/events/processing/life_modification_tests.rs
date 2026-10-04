use super::*;
use crate::effects::{EffectExecutor, ExecutionContext};
use crate::ids::{CardId, PlayerId};
use crate::target::{ChooseSpec, PlayerFilter};

fn setup() -> (GameState, ObjectId, PlayerId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = crate::card::CardBuilder::new(CardId::new(), "Life modifier source")
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    game.take_pending_trigger_events();
    (game, source, alice, bob)
}

#[test]
fn numeric_life_loss_modifications_commit_the_resolved_amount() {
    for (modification, expected) in [
        (crate::replacement::EventModification::Multiply(2), 6),
        (crate::replacement::EventModification::Add(2), 5),
        (crate::replacement::EventModification::Add(-5), 0),
        (crate::replacement::EventModification::Subtract(5), 0),
        (crate::replacement::EventModification::SetTo(7), 7),
        (
            crate::replacement::EventModification::SetToAtLeast(crate::effect::Value::Fixed(8)),
            8,
        ),
        (crate::replacement::EventModification::ReduceToZero, 0),
    ] {
        let (mut game, source, alice, bob) = setup();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.replacement
            .additional_replacement_effects
            .push(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldLoseLifeMatcher::new(PlayerFilter::Specific(bob)),
                ReplacementAction::Modify(modification),
            ));
        let outcome = crate::effects::LoseLifeEffect::new(3, ChooseSpec::SpecificPlayer(bob))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(outcome.count_or_zero(), expected);
        assert_eq!(i64::from(game.player(bob).unwrap().life), 20 - expected);
        assert_eq!(game.player(alice).unwrap().life, 20);
        let mut events = outcome.events;
        events.extend(game.take_pending_trigger_events());
        let loss = events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::LifeLossEvent>())
            .collect::<Vec<_>>();
        assert_eq!(loss.len(), usize::from(expected > 0));
        assert!(
            loss.iter()
                .all(|event| event.player == bob && event.amount == expected as u32)
        );
    }
}

#[test]
fn temporary_life_effect_ids_do_not_alias_registered_suppression() {
    for mask in 0..4 {
        let (mut game, source, alice, _) = setup();
        let registered = game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageFromSourceMatcher::new(
                    crate::target::ObjectFilter::specific(source),
                ),
                ReplacementAction::Double,
            ),
        );
        let temporary = ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::WouldGainLifeMatcher::new(PlayerFilter::Specific(alice)),
            ReplacementAction::Double,
        );
        assert_eq!(
            temporary.id, registered,
            "exercise the placeholder-ID collision"
        );
        let key = temporary.application_key();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.replacement
            .additional_replacement_effects
            .push(temporary);
        if mask & 1 != 0 {
            ctx.replacement
                .suppressed_replacement_effects
                .insert(registered);
        }
        if mask & 2 != 0 {
            ctx.replacement
                .suppressed_replacement_effect_keys
                .insert(key.clone());
        }
        let outcome = crate::effects::GainLifeEffect::new(2, ChooseSpec::SpecificPlayer(alice))
            .execute(&mut game, &mut ctx)
            .unwrap();
        let expected = if mask & 2 == 0 { 4 } else { 2 };
        assert_eq!(outcome.count_or_zero(), expected);
        assert_eq!(i64::from(game.player(alice).unwrap().life), 20 + expected);
        assert_eq!(
            ctx.replacement.additional_replacement_effects[0].id,
            registered
        );
        assert_eq!(
            ctx.replacement
                .suppressed_replacement_effects
                .contains(&registered),
            mask & 1 != 0
        );
        assert_eq!(
            ctx.replacement
                .suppressed_replacement_effect_keys
                .contains(&key),
            mask & 2 != 0
        );
    }
}
