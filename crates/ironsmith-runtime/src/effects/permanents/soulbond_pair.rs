//! Soulbond pairing effect implementation.

use crate::decisions::{ChoiceSpec, DisplayOption, make_decision};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SoulbondPairEffect;

impl SoulbondPairEffect {
    pub const fn new() -> Self {
        Self
    }

    fn source_is_valid(game: &GameState, source: ObjectId, controller: PlayerId) -> bool {
        game.object(source).is_some_and(|object| {
            object.zone == Zone::Battlefield
                && game.current_is_creature(source)
                && object.controller == controller
        })
    }

    fn candidate_creatures(
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Vec<ObjectId> {
        game.battlefield
            .iter()
            .copied()
            .filter(|id| *id != source)
            .filter(|id| {
                game.object(*id).is_some_and(|object| {
                    object.zone == Zone::Battlefield
                        && game.current_is_creature(*id)
                        && object.controller == controller
                })
            })
            .filter(|id| !game.is_soulbond_paired(*id))
            .collect()
    }
}

impl EffectExecutor for SoulbondPairEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source = ctx.source;
        if !Self::source_is_valid(game, source, ctx.controller) || game.is_soulbond_paired(source) {
            return Ok(EffectOutcome::count(0));
        }

        let candidates = Self::candidate_creatures(game, source, ctx.controller);
        if candidates.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let options: Vec<DisplayOption> = candidates
            .iter()
            .enumerate()
            .map(|(index, object_id)| {
                let name = game
                    .object(*object_id)
                    .map(|object| object.name.clone())
                    .unwrap_or_else(|| format!("Object {}", object_id.0));
                DisplayOption::new(index, name)
            })
            .collect();

        let spec = ChoiceSpec::new(source, options, 0, 1);
        let chosen = make_decision(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(source),
            spec,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(choice_index) = chosen.first().copied() else {
            return Ok(EffectOutcome::count(0));
        };
        let Some(partner) = candidates.get(choice_index).copied() else {
            return Ok(EffectOutcome::count(0));
        };

        if !Self::source_is_valid(game, source, ctx.controller)
            || game.is_soulbond_paired(source)
            || game.is_soulbond_paired(partner)
        {
            return Ok(EffectOutcome::count(0));
        }
        if !game.object(partner).is_some_and(|object| {
            object.zone == Zone::Battlefield
                && game.current_is_creature(partner)
                && object.controller == ctx.controller
        }) {
            return Ok(EffectOutcome::count(0));
        }

        game.set_soulbond_pair(source, partner);
        Ok(EffectOutcome::count(
            if game.soulbond_partner(source) == Some(partner) {
                1
            } else {
                0
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    fn creature(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    struct ChooseFirstOption;

    impl DecisionMaker for ChooseFirstOption {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.legal)
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }

    struct DeclinePairing;

    impl DecisionMaker for DeclinePairing {
        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            Vec::new()
        }
    }

    #[test]
    fn pairs_source_with_chosen_unpaired_creature() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(&creature(1, "Source"), alice, Zone::Battlefield);
        let partner =
            game.create_object_from_card(&creature(2, "Partner"), alice, Zone::Battlefield);

        let mut decision_maker = ChooseFirstOption;
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
        let outcome = SoulbondPairEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute soulbond pairing");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.soulbond_partner(source), Some(partner));
        assert_eq!(game.soulbond_partner(partner), Some(source));
    }

    #[test]
    fn can_decline_pairing() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(&creature(3, "Source"), alice, Zone::Battlefield);
        game.create_object_from_card(&creature(4, "Partner"), alice, Zone::Battlefield);

        let mut decision_maker = DeclinePairing;
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
        let outcome = SoulbondPairEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute soulbond pairing");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.soulbond_partner(source), None);
    }
}
