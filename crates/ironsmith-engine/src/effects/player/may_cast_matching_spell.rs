//! One-shot free-cast effect for matching spells.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::zone::Zone;
pub use ironsmith_core::MayCastMatchingSpellWithoutPayingManaCostEffect;

use super::runtime_helpers::{
    EffectDrivenCastOption, EffectDrivenCastPayment, cast_effect_driven_spell_with_payment,
    effect_driven_cast_options_for_card_with_payment, with_spell_cast_event,
};

fn runtime_payment(
    payment: &ironsmith_core::MayCastMatchingSpellPayment,
) -> EffectDrivenCastPayment {
    match payment {
        ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost => {
            EffectDrivenCastPayment::WithoutPayingManaCost
        }
        ironsmith_core::MayCastMatchingSpellPayment::AlternativeCost(kind) => {
            EffectDrivenCastPayment::AlternativeCost(*kind)
        }
    }
}

fn object_ids_in_zone(game: &GameState, player: PlayerId, zone: Zone) -> Vec<ObjectId> {
    match zone {
        Zone::Hand => game
            .player(player)
            .map(|player| player.hand.clone())
            .unwrap_or_default(),
        Zone::Graveyard => game
            .player(player)
            .map(|player| player.graveyard.clone())
            .unwrap_or_default(),
        Zone::Library => game
            .player(player)
            .map(|player| player.library.clone())
            .unwrap_or_default(),
        Zone::Exile => game.exile.clone(),
        Zone::Battlefield => game.battlefield.clone(),
        Zone::Stack => game.stack.iter().map(|entry| entry.object_id).collect(),
        Zone::Command => game.command_zone.clone(),
        Zone::Ante => game.ante.clone(),
        Zone::OutsideGame => Vec::new(),
    }
}

impl EffectExecutor for MayCastMatchingSpellWithoutPayingManaCostEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let zone_owner_id = resolve_player_filter(game, &self.zone_owner, ctx)?;
        let object_ids = object_ids_in_zone(game, zone_owner_id, self.zone);
        let mut options = Vec::<EffectDrivenCastOption>::new();
        let payment = runtime_payment(&self.payment);
        for object_id in object_ids {
            options.extend(effect_driven_cast_options_for_card_with_payment(
                game,
                player_id,
                ctx.source,
                object_id,
                self.zone,
                &self.filter,
                payment,
            ));
        }
        if options.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let should_cast = {
            let choice_ctx = crate::decisions::context::BooleanContext::new(
                player_id,
                Some(ctx.source),
                "Cast a spell without paying its mana cost?".to_string(),
            );
            ctx.decision_maker.decide_boolean(game, &choice_ctx)
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if !should_cast {
            return Ok(EffectOutcome::count(0));
        }

        let option = if options.len() == 1 {
            options[0].clone()
        } else {
            let choices = options
                .iter()
                .cloned()
                .map(|option| (option.label.clone(), option))
                .collect::<Vec<_>>();
            let Some(choice) = crate::decisions::ask_choose_one(
                game,
                ctx.decision_maker,
                player_id,
                ctx.source,
                &choices,
            ) else {
                return Ok(EffectOutcome::count(0));
            };
            choice
        };

        let Some(result) =
            cast_effect_driven_spell_with_payment(game, ctx, player_id, &option, payment)?
        else {
            return Ok(EffectOutcome::impossible());
        };

        Ok(with_spell_cast_event(
            EffectOutcome::with_objects(vec![result.new_id]),
            game,
            result.new_id,
            player_id,
            result.from_zone,
            ctx.provenance,
        ))
    }
}
