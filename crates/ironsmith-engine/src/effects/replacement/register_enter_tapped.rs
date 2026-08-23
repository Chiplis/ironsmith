use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::zones::matchers::WouldEnterBattlefieldMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterEnterTappedReplacementEffect =
    ironsmith_core::RegisterEnterTappedReplacementEffect;

impl EffectExecutor for RegisterEnterTappedReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            WouldEnterBattlefieldMatcher::new(self.filter.clone()),
            ReplacementAction::EnterTapped,
        );

        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute(game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn artifact(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        zone: Zone,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn turn_scoped_entry_tapping_is_multi_use_and_expires() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = artifact(&mut game, alice, "Entry Rule Source", Zone::Battlefield);
        let first = artifact(&mut game, alice, "First Incoming", Zone::Hand);
        let second = artifact(&mut game, alice, "Second Incoming", Zone::Hand);
        let after = artifact(&mut game, alice, "Post-Cleanup Incoming", Zone::Hand);

        let effect = RegisterEnterTappedReplacementEffect::new(
            ObjectFilter::permanent(),
            crate::effects::ReplacementApplyMode::UntilEndOfTurn,
        );
        let mut decisions = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
        effect
            .execute(&mut game, &mut ctx)
            .expect("entry replacement should register");

        let first = game
            .move_object_with_etb_processing(first, Zone::Battlefield)
            .unwrap();
        let second = game
            .move_object_with_etb_processing(second, Zone::Battlefield)
            .unwrap();
        assert!(first.enters_tapped && game.is_tapped(first.new_id));
        assert!(second.enters_tapped && game.is_tapped(second.new_id));

        game.effect_store
            .replacement_effects
            .clear_until_end_of_turn_effects();
        let after = game
            .move_object_with_etb_processing(after, Zone::Battlefield)
            .unwrap();
        assert!(!after.enters_tapped && !game.is_tapped(after.new_id));
    }
}
