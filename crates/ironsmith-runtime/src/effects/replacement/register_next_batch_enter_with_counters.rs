use crate::effect::{EffectOutcome, OutcomeStatus};
use crate::effects::{EffectExecutionCategory, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::zones::matchers::WouldEnterBattlefieldMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterNextBatchEnterWithCountersEffect =
    ironsmith_core::RegisterNextBatchEnterWithCountersEffect;

impl EffectExecutor for RegisterNextBatchEnterWithCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut matcher = WouldEnterBattlefieldMatcher::new(self.filter.clone());
        if let Some(tag) = &self.same_stable_id_tag {
            let stable_id = ctx
                .tagged_objects
                .get(tag)
                .and_then(|snapshots| snapshots.first())
                .map(|snapshot| snapshot.stable_id)
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(format!(
                        "missing tagged object for future entry replacement: {tag}"
                    ))
                })?;
            matcher = matcher.with_stable_id(stable_id);
        }
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            matcher,
            ReplacementAction::EnterWithCounters {
                counter_type: self.counter_type,
                count: self.count.clone(),
                count_condition: None,
                otherwise_count: None,
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
            },
        );
        game.effect_store
            .replacement_effects
            .add_batch_one_shot_effect(replacement);
        Ok(EffectOutcome::from_status(OutcomeStatus::Succeeded))
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::zones::{
        BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_batch_with_options,
    };
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::CounterType;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn permanent(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        card_types: Vec<CardType>,
        zone: Zone,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    fn moved_id(outcome: BattlefieldEntryOutcome) -> ObjectId {
        let BattlefieldEntryOutcome::Moved(id) = outcome else {
            panic!("expected permanent to enter the battlefield");
        };
        id
    }

    #[test]
    fn next_matching_entry_batch_counters_every_matching_entrant_then_expires() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = permanent(
            &mut game,
            alice,
            "Batch Counter Source",
            vec![CardType::Enchantment],
            Zone::Battlefield,
        );
        let first = permanent(
            &mut game,
            alice,
            "First Enchantment Creature",
            vec![CardType::Enchantment, CardType::Creature],
            Zone::Hand,
        );
        let second = permanent(
            &mut game,
            alice,
            "Second Enchantment Creature",
            vec![CardType::Enchantment, CardType::Creature],
            Zone::Hand,
        );
        let nonmatching = permanent(
            &mut game,
            alice,
            "Ordinary Creature",
            vec![CardType::Creature],
            Zone::Hand,
        );
        let later = permanent(
            &mut game,
            alice,
            "Later Enchantment Creature",
            vec![CardType::Enchantment, CardType::Creature],
            Zone::Hand,
        );

        let filter = ObjectFilter::default()
            .with_all_type(CardType::Enchantment)
            .with_all_type(CardType::Creature)
            .you_control()
            .in_zone(Zone::Battlefield);
        let effect = RegisterNextBatchEnterWithCountersEffect::new(
            filter,
            CounterType::PlusOnePlusOne,
            Value::Fixed(2),
        );
        let mut decisions = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
        effect
            .execute(&mut game, &mut ctx)
            .expect("next-batch entry replacement should register");

        let outcomes = move_to_battlefield_batch_with_options(
            &mut game,
            &mut ctx,
            vec![
                (first, BattlefieldEntryOptions::preserve(false)),
                (nonmatching, BattlefieldEntryOptions::preserve(false)),
                (second, BattlefieldEntryOptions::preserve(false)),
            ],
        );
        let first = moved_id(outcomes[0]);
        let nonmatching = moved_id(outcomes[1]);
        let second = moved_id(outcomes[2]);

        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 2);
        assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 2);
        assert_eq!(
            game.counter_count(nonmatching, CounterType::PlusOnePlusOne),
            0,
            "a nonmatching sibling in the simultaneous event must not gain counters"
        );
        assert!(
            game.effect_store
                .replacement_effects
                .one_shot_effects_snapshot()
                .is_empty(),
            "the batch replacement must be consumed after the first matching event"
        );

        let later = moved_id(
            move_to_battlefield_batch_with_options(
                &mut game,
                &mut ctx,
                vec![(later, BattlefieldEntryOptions::preserve(false))],
            )[0],
        );
        assert_eq!(
            game.counter_count(later, CounterType::PlusOnePlusOne),
            0,
            "an independent later ETB event must not reuse the consumed replacement"
        );
    }
}
