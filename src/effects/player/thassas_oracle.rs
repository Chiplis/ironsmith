use crate::color::Color;
use crate::decisions::context::{SelectObjectsContext, SelectableObject, ViewCardsContext};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ThassasOracleEffect;

impl EffectExecutor for ThassasOracleEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let devotion = game.devotion_to_color(ctx.controller, Color::Blue);
        let library_size = game
            .player(ctx.controller)
            .map(|player| player.library.len())
            .unwrap_or(0);

        if devotion > 0 {
            let top_cards: Vec<_> = game
                .player(ctx.controller)
                .map(|player| player.library.iter().rev().take(devotion).copied().collect())
                .unwrap_or_default();

            if !top_cards.is_empty() {
                let view_ctx = ViewCardsContext::new(
                    ctx.controller,
                    ctx.controller,
                    Some(ctx.source),
                    Zone::Library,
                    "Look at cards from the top of your library",
                );
                ctx.decision_maker
                    .view_cards(game, ctx.controller, &top_cards, &view_ctx);

                let candidates: Vec<_> = top_cards
                    .iter()
                    .filter_map(|&id| {
                        game.object(id)
                            .map(|obj| SelectableObject::new(id, obj.name.clone()))
                    })
                    .collect();
                let choose_ctx = SelectObjectsContext::new(
                    ctx.controller,
                    Some(ctx.source),
                    "Choose up to one card to leave on top of your library",
                    candidates,
                    0,
                    Some(1),
                );
                let selected = ctx.decision_maker.decide_objects(game, &choose_ctx);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }

                let chosen_top = selected.into_iter().find(|id| top_cards.contains(id));
                let Some(current_library) = game
                    .player(ctx.controller)
                    .map(|player| player.library.clone())
                else {
                    return Ok(EffectOutcome::resolved());
                };

                let split_at = current_library.len().saturating_sub(top_cards.len());
                let untouched = current_library[..split_at].to_vec();
                let mut bottom_cards: Vec<_> = top_cards
                    .iter()
                    .copied()
                    .filter(|id| Some(*id) != chosen_top)
                    .collect();
                game.shuffle_slice(&mut bottom_cards);

                let mut reordered = bottom_cards;
                reordered.extend(untouched);
                if let Some(chosen_top) = chosen_top {
                    reordered.push(chosen_top);
                }

                if let Some(player) = game.player_mut(ctx.controller) {
                    player.library = reordered;
                }
            }
        }

        if devotion >= library_size {
            return crate::effects::WinTheGameEffect::you().execute(game, ctx);
        }

        Ok(EffectOutcome::resolved())
    }
}
