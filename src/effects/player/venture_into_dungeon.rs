//! Venture into a dungeon by starting or advancing dungeon progress.

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::dungeon::{
    ActiveDungeonProgress, first_room_name, next_room_names, normal_venture_dungeon_names,
    undercity_name,
};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct VentureIntoDungeonEffect {
    pub player: PlayerFilter,
    pub undercity_if_no_active: bool,
}

impl VentureIntoDungeonEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            undercity_if_no_active: false,
        }
    }

    pub fn via_initiative(player: PlayerFilter) -> Self {
        Self {
            player,
            undercity_if_no_active: true,
        }
    }
}

fn choose_named_option(
    ctx: &mut ExecutionContext,
    game: &GameState,
    chooser: PlayerId,
    prompt: &str,
    options: &[String],
) -> Result<Option<String>, ExecutionError> {
    let selectable = options
        .iter()
        .enumerate()
        .map(|(idx, option)| SelectableOption::new(idx, option.clone()))
        .collect::<Vec<_>>();
    let choice_ctx = SelectOptionsContext::new(chooser, Some(ctx.source), prompt, selectable, 1, 1);
    let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    Ok(selected
        .into_iter()
        .next()
        .filter(|idx| *idx < options.len())
        .map(|idx| options[idx].clone()))
}

pub(crate) fn advance_player_dungeon(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    undercity_if_no_active: bool,
) -> Result<EffectOutcome, ExecutionError> {
    let (dungeon_name, room_name) = if let Some(progress) = game.active_dungeon(player_id).cloned()
    {
        let next_rooms =
            next_room_names(&progress.dungeon_name, &progress.room_name).ok_or_else(|| {
                ExecutionError::Impossible(format!(
                    "missing next room data for {} -> {}",
                    progress.dungeon_name, progress.room_name
                ))
            })?;
        if next_rooms.is_empty() {
            return Err(ExecutionError::Impossible(format!(
                "{player_id:?} is already in the final room of {}",
                progress.dungeon_name
            )));
        }
        let next_room = if next_rooms.len() == 1 {
            next_rooms[0].clone()
        } else {
            let Some(next_room) = choose_named_option(
                ctx,
                game,
                player_id,
                "Choose the next dungeon room",
                &next_rooms,
            )?
            else {
                return Ok(EffectOutcome::count(0));
            };
            next_room
        };
        (progress.dungeon_name, next_room)
    } else {
        let dungeon_options = if undercity_if_no_active {
            vec![undercity_name().to_string()]
        } else {
            normal_venture_dungeon_names()
        };
        let dungeon_name = if dungeon_options.len() == 1 {
            dungeon_options[0].clone()
        } else {
            let Some(dungeon_name) =
                choose_named_option(ctx, game, player_id, "Choose a dungeon", &dungeon_options)?
            else {
                return Ok(EffectOutcome::count(0));
            };
            dungeon_name
        };
        let room_name = first_room_name(&dungeon_name)
            .ok_or_else(|| ExecutionError::Impossible(format!("unknown dungeon {dungeon_name}")))?
            .to_string();
        (dungeon_name, room_name)
    };

    game.set_active_dungeon(
        player_id,
        ActiveDungeonProgress::new(dungeon_name.clone(), room_name.clone()),
    );

    let mut outcome = EffectOutcome::resolved();
    let next_rooms = next_room_names(&dungeon_name, &room_name)
        .ok_or_else(|| ExecutionError::Impossible(format!("unknown room {room_name}")))?;
    if next_rooms.is_empty() {
        game.clear_active_dungeon(player_id);
        game.record_completed_dungeon(player_id, dungeon_name);
        outcome = outcome.with_event(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::CompleteDungeon, player_id, ctx.source, 1),
            ctx.provenance,
        ));
    }

    Ok(outcome)
}

impl EffectExecutor for VentureIntoDungeonEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        advance_player_dungeon(game, ctx, player_id, self.undercity_if_no_active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectOptionsContext;
    use crate::ids::{ObjectId, PlayerId};

    struct ChooseFirstOptionDecisionMaker;

    impl DecisionMaker for ChooseFirstOptionDecisionMaker {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            ctx.options
                .first()
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }

    #[test]
    fn venture_starts_lost_mine_by_default() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(700);
        let mut dm = ChooseFirstOptionDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        VentureIntoDungeonEffect::new(PlayerFilter::Specific(alice))
            .execute(&mut game, &mut ctx)
            .expect("venture should resolve");

        let progress = game
            .active_dungeon(alice)
            .expect("venture should start a dungeon");
        assert_eq!(progress.dungeon_name, "Lost Mine of Phandelver");
        assert_eq!(progress.room_name, "Cave Entrance");
        assert!(!game.has_completed_dungeon(alice));
    }

    #[test]
    fn venture_can_complete_a_dungeon_and_emit_completion_event() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(701);
        let mut dm = ChooseFirstOptionDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = VentureIntoDungeonEffect::new(PlayerFilter::Specific(alice));

        for _ in 0..3 {
            effect
                .execute(&mut game, &mut ctx)
                .expect("venture progress should resolve");
        }
        let final_outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("final venture should resolve");

        assert!(game.active_dungeon(alice).is_none());
        assert!(game.has_completed_named_dungeon(alice, "Lost Mine of Phandelver"));
        let completion = final_outcome.events[0]
            .downcast::<KeywordActionEvent>()
            .expect("expected dungeon completion event");
        assert_eq!(completion.action, KeywordActionKind::CompleteDungeon);
        assert_eq!(completion.player, alice);
    }
}
