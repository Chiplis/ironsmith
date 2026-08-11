//! Temporary repeatable priority special actions created by resolving effects.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{GameState, RepeatableManaPaymentAction};
use crate::target::PlayerFilter;

/// Grants a player a repeatable mana-payment special action through end of turn.
///
/// This models instructions of the form "Until end of turn, you may pay [cost]
/// any time you could cast an instant. If you do, [effects]." The original
/// resolution targets and typed reference context are captured when the grant
/// resolves so each later action applies to the same objects or players.
pub type GrantRepeatableManaPaymentActionUntilEndOfTurnEffect =
    ironsmith_core::GrantRepeatableManaPaymentActionUntilEndOfTurnEffect<Effect>;

impl EffectExecutor for GrantRepeatableManaPaymentActionUntilEndOfTurnEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        game.effect_store
            .repeatable_mana_payment_actions
            .push(RepeatableManaPaymentAction {
                player,
                source: ctx.source,
                controller: ctx.controller,
                cost: self.cost.clone(),
                effects: self.effects.clone(),
                targets: ctx.targets.clone(),
                tagged_objects: ctx.tagged_objects.clone(),
                tagged_players: ctx.tagged_players.clone(),
                expires_end_of_turn: game.turn.turn_number,
            });
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::{ExecutionContext, PreventDamageEffect, ResolvedTarget};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::mana::ManaSymbol;
    use crate::special_actions::{SpecialAction, can_perform_check, perform};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn granted_action_is_repeatable_reuses_target_and_expires_at_end_of_turn() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.turn.priority_player = Some(alice);
        let target_card = CardBuilder::new(CardId::new(), "Protected creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let target = game.create_object_from_card(&target_card, alice, Zone::Battlefield);
        let source = game.new_object_id();
        let prevention = Effect::new(PreventDamageEffect::any_target(
            1,
            crate::effect::Until::EndOfTurn,
        ));
        let grant = GrantRepeatableManaPaymentActionUntilEndOfTurnEffect::new(
            PlayerFilter::You,
            ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]),
            vec![prevention],
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);
        grant
            .execute(&mut game, &mut ctx)
            .expect("grant should register the priority action");
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        let action = SpecialAction::PerformRepeatableManaPaymentAction { action_index: 0 };
        let bob = PlayerId::from_index(1);
        game.turn.priority_player = Some(bob);
        assert!(
            can_perform_check(&action, &game, alice).is_err(),
            "the granted action follows instant-timing priority"
        );
        game.turn.priority_player = Some(alice);
        let mut decisions = SelectFirstDecisionMaker;
        for _ in 0..2 {
            assert!(can_perform_check(&action, &game, alice).is_ok());
            perform(action.clone(), &mut game, alice, &mut decisions)
                .expect("each paid action should register another shield");
        }
        assert!(
            can_perform_check(&action, &game, alice).is_err(),
            "the action remains registered but is illegal without enough mana"
        );
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 2);

        let damage_source = game.new_object_id();
        let (remaining, _) = crate::events::processing::process_damage_with_event(
            &mut game,
            damage_source,
            crate::events::DamageTarget::Object(target),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        );
        assert_eq!(remaining, 1);
        assert_eq!(
            game.damage_on(target),
            0,
            "the event processor reports the final assignment but does not apply it"
        );

        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        game.turn.turn_number += 1;
        assert!(
            can_perform_check(&action, &game, alice).is_err(),
            "the priority permission expires at the turn boundary"
        );
        game.cleanup_repeatable_mana_payment_actions_end_of_turn();
        assert!(game.effect_store.repeatable_mana_payment_actions.is_empty());
    }
}
