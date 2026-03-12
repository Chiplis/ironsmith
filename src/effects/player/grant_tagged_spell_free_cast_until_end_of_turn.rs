//! Grant temporary "cast this tagged exiled spell without paying its mana cost"
//! permissions.

use crate::alternative_cast::AlternativeCastingMethod;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant_registry::GrantSource;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Grants a temporary zero-mana alternative casting method to tagged exiled spells.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantTaggedSpellFreeCastUntilEndOfTurnEffect {
    pub tag: TagKey,
    pub player: PlayerFilter,
}

impl GrantTaggedSpellFreeCastUntilEndOfTurnEffect {
    pub fn new(tag: impl Into<TagKey>, player: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            player,
        }
    }
}

impl EffectExecutor for GrantTaggedSpellFreeCastUntilEndOfTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let Some(snapshots) = ctx.get_tagged_all(self.tag.as_str()).cloned() else {
            return Ok(EffectOutcome::count(0));
        };

        let expires_end_of_turn = game.turn.turn_number;
        let mut granted = 0usize;
        let mut seen = std::collections::HashSet::new();

        for snapshot in snapshots {
            let mut object_id = snapshot.object_id;
            if game.object(object_id).is_none() {
                if let Some(found) = game.find_object_by_stable_id(snapshot.stable_id) {
                    object_id = found;
                } else {
                    continue;
                }
            }

            let Some(object) = game.object(object_id) else {
                continue;
            };
            if object.zone != Zone::Exile || object.is_land() || !seen.insert(object_id) {
                continue;
            }

            let method = AlternativeCastingMethod::alternative_cost(
                "Without paying its mana cost",
                None,
                vec![],
            );
            game.grant_registry.grant_alternative_cast_to_card(
                object_id,
                Zone::Exile,
                player_id,
                method,
                GrantSource::Effect {
                    source_id: ctx.source,
                    expires_end_of_turn,
                },
            );
            granted += 1;
        }

        Ok(EffectOutcome::count(granted as i32))
    }
}
