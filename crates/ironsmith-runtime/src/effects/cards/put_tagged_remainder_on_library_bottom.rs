use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, consult_helpers::*};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::PlayerFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct PutTaggedRemainderOnLibraryBottomEffect {
    pub tag: TagKey,
    pub keep_tagged: Option<TagKey>,
    pub order: LibraryBottomOrder,
    pub player: PlayerFilter,
}

impl PutTaggedRemainderOnLibraryBottomEffect {
    pub fn new(
        tag: impl Into<TagKey>,
        keep_tagged: Option<TagKey>,
        order: LibraryBottomOrder,
        player: PlayerFilter,
    ) -> Self {
        Self {
            tag: tag.into(),
            keep_tagged,
            order,
            player,
        }
    }
}

impl EffectExecutor for PutTaggedRemainderOnLibraryBottomEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.player, ctx)?;
        move_tagged_remainder_to_library_bottom(
            game,
            ctx,
            &self.tag,
            self.keep_tagged.as_ref(),
            self.order,
            chooser,
        )
    }
}
