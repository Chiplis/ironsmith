//! Register a one-shot spell-ability grant for the next matching spell this turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter};

#[derive(Debug, Clone, PartialEq)]
pub struct GrantNextSpellAbilityEffect {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub ability: StaticAbility,
}

impl GrantNextSpellAbilityEffect {
    pub fn new(player: PlayerFilter, filter: ObjectFilter, ability: StaticAbility) -> Self {
        Self {
            player,
            filter,
            ability,
        }
    }
}

impl EffectExecutor for GrantNextSpellAbilityEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        game.add_temporary_spell_ability_grant(
            player,
            ctx.source,
            self.filter.clone(),
            self.ability.clone(),
            1,
        );
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    #[test]
    fn execute_registers_next_spell_ability_grant() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = GrantNextSpellAbilityEffect::new(
            PlayerFilter::You,
            ObjectFilter::noncreature_spell().cast_by(PlayerFilter::You),
            StaticAbility::cascade(),
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("grant effect should resolve");

        assert_eq!(game.temporary_spell_ability_grants.len(), 1);
        let grant = &game.temporary_spell_ability_grants[0];
        assert_eq!(grant.player, alice);
        assert_eq!(
            grant.ability.id(),
            crate::static_abilities::StaticAbilityId::Cascade
        );
    }
}
