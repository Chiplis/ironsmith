//! Create a card copy from captured copiable characteristics and offer an
//! immediate free cast. The original card need not remain in its old zone.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::Object;
use crate::target::PlayerFilter;
use crate::zone::Zone;

#[derive(Debug, Clone)]
pub(crate) struct CastStoredCardCopyEffect {
    card: Object,
}
impl CastStoredCardCopyEffect {
    pub(crate) fn new(card: &Object) -> Self { Self { card: card.clone() } }
}
impl EffectExecutor for CastStoredCardCopyEffect {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        let id = game.new_object_id();
        let mut copy = Object::token_copy_of(&self.card, id, ctx.controller);
        copy.zone = Zone::Exile;
        game.add_object(copy);
        let tag = crate::tag::TagKey::from("__stored_card_copy__");
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(id).expect("created copy"), game);
        ctx.set_tagged_objects(tag.clone(), vec![snapshot]);
        let effect = Effect::may(vec![Effect::cast_tagged(tag, PlayerFilter::You, false, false, true, None)]);
        crate::effects::execute_effect(game, &effect, ctx)
    }
}
