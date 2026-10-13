//! "For each player, choose friend or foe." The controller designates each
//! in-game player (themself included) a friend or a foe as the spell
//! resolves (CR 608.2d); the groups are tagged for later "Each friend/foe"
//! instructions of the same resolution.
use crate::decisions::context::BooleanContext;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type ChooseFriendsOrFoesEffect = ironsmith_core::ChooseFriendsOrFoesEffect;

impl EffectExecutor for ChooseFriendsOrFoesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let chooser = ctx.controller;
        let players = game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| (player.id, player.name.to_string()))
            .collect::<Vec<_>>();
        let Some(choices) =
            crate::effects::composition::scoped_choice::collect_choices_before_commit(
                game,
                ctx,
                players,
                |game, ctx, (player, name)| {
                    let prompt = BooleanContext::new(
                        chooser,
                        Some(ctx.source),
                        format!("Choose {name} as a friend? (Otherwise {name} is a foe.)"),
                    );
                    Ok((player, ctx.decision_maker.decide_boolean(game, &prompt)))
                },
            )?
        else {
            return Ok(EffectOutcome::count(0));
        };
        let mut friends = Vec::new();
        let mut foes = Vec::new();
        for (player, friend) in choices {
            if friend {
                friends.push(player);
            } else {
                foes.push(player);
            }
        }
        ctx.set_tagged_players(self.friends_tag.clone(), friends);
        ctx.set_tagged_players(self.foes_tag.clone(), foes);
        Ok(EffectOutcome::resolved())
    }
}
