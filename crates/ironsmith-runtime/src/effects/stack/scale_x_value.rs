//! Effect that scales the chosen X value of a stack object.

use crate::effect::{EffectOutcome, OutcomeStatus};
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Scales the X value carried by a spell or ability on the stack.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleXValueEffect {
    pub target: ChooseSpec,
    pub multiplier: u32,
}

impl ScaleXValueEffect {
    pub fn new(target: ChooseSpec, multiplier: u32) -> Self {
        Self { target, multiplier }
    }
}

impl EffectExecutor for ScaleXValueEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_id = match resolve_single_object_for_effect(game, ctx, &self.target) {
            Ok(object_id) => object_id,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };

        let Some(entry) = game
            .stack
            .iter_mut()
            .find(|entry| entry.object_id == object_id)
        else {
            return Ok(EffectOutcome::target_invalid());
        };
        let Some(x_value) = entry.x_value else {
            return Ok(EffectOutcome::from_status(OutcomeStatus::Impossible));
        };

        let scaled = x_value.saturating_mul(self.multiplier);
        entry.x_value = Some(scaled);

        if let Some(object) = game.object_mut(object_id)
            && object.zone == Zone::Stack
        {
            object.x_value = Some(scaled);
        }

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "stack object with X value"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::types::CardType;
    use std::collections::HashMap;

    struct NoopDecisionMaker;

    impl crate::decision::DecisionMaker for NoopDecisionMaker {}

    #[test]
    fn scales_spell_stack_entry_and_object_x_value() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(1), "X Spell")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::X]]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let spell = game.create_object_from_card(&card, alice, Zone::Stack);
        game.object_mut(spell).unwrap().x_value = Some(3);
        game.push_to_stack(crate::game_state::StackEntry::new(spell, alice).with_x(3));

        let mut decision_maker = NoopDecisionMaker;
        let mut ctx = ExecutionContext::new(spell, alice, &mut decision_maker);
        let effect = ScaleXValueEffect::new(ChooseSpec::SpecificObject(spell), 2);

        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            game.stack
                .iter()
                .find(|entry| entry.object_id == spell)
                .unwrap()
                .x_value,
            Some(6)
        );
        assert_eq!(game.object(spell).unwrap().x_value, Some(6));
    }

    #[test]
    fn scales_triggering_tagged_stack_object_x_value() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(1), "X Creature Spell")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::X]]))
            .card_types(vec![CardType::Creature])
            .build();
        let spell = game.create_object_from_card(&card, alice, Zone::Stack);
        game.object_mut(spell).unwrap().x_value = Some(2);
        game.push_to_stack(crate::game_state::StackEntry::new(spell, alice).with_x(2));
        let snapshot = ObjectSnapshot::from_object(game.object(spell).unwrap(), &game);
        let mut tags = HashMap::new();
        tags.insert(TagKey::from("triggering"), vec![snapshot]);

        let mut decision_maker = NoopDecisionMaker;
        let mut ctx =
            ExecutionContext::new(spell, alice, &mut decision_maker).with_tagged_objects(tags);
        let effect = ScaleXValueEffect::new(ChooseSpec::Tagged(TagKey::from("triggering")), 2);

        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            game.stack
                .iter()
                .find(|entry| entry.object_id == spell)
                .unwrap()
                .x_value,
            Some(4)
        );
        assert_eq!(game.object(spell).unwrap().x_value, Some(4));
    }

    #[test]
    fn reports_impossible_when_target_has_no_x_value() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(1), "Plain Spell")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let spell = game.create_object_from_card(&card, alice, Zone::Stack);
        game.push_to_stack(crate::game_state::StackEntry::new(spell, alice));

        let mut decision_maker = NoopDecisionMaker;
        let mut ctx = ExecutionContext::new(spell, alice, &mut decision_maker);
        let effect = ScaleXValueEffect::new(ChooseSpec::SpecificObject(spell), 2);

        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(outcome.status, OutcomeStatus::Impossible);
        assert_eq!(
            game.stack
                .iter()
                .find(|entry| entry.object_id == spell)
                .unwrap()
                .x_value,
            None
        );
    }
}
