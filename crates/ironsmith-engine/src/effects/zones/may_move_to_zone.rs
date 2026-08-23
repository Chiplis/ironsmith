//! Prompt a player to optionally move an object to a zone.

use crate::decision::FallbackStrategy;
use crate::decisions::context::DecisionHiddenCardVisibility;
use crate::decisions::{make_decision_with_fallback, specs::MaySpec};
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::ChooseSpec;
use crate::zone::Zone;
pub use ironsmith_core::MayMoveToZoneEffect;

fn describe_move(zone: Zone, game: &GameState, object_id: crate::ids::ObjectId) -> String {
    let object_name = game
        .object(object_id)
        .map(|obj| obj.name.to_string())
        .unwrap_or_else(|| "that card".to_string());
    match zone {
        Zone::Hand => format!("Put {object_name} into your hand?"),
        Zone::Exile => format!("Exile {object_name}?"),
        Zone::Graveyard => format!("Put {object_name} into its owner's graveyard?"),
        Zone::Library => format!("Put {object_name} into its owner's library?"),
        Zone::Battlefield => format!("Put {object_name} onto the battlefield?"),
        Zone::Command => format!("Put {object_name} into the command zone?"),
        Zone::Ante => format!("Ante {object_name}?"),
        Zone::Stack => format!("Move {object_name} to the stack?"),
        Zone::OutsideGame => format!("Move {object_name} outside the game?"),
    }
}

fn hidden_decision_visibility(
    game: &GameState,
    object_id: ObjectId,
    viewer: PlayerId,
) -> Option<DecisionHiddenCardVisibility> {
    let object = game.object(object_id)?;
    game.hidden_card_info(object_id)?;

    if game.is_face_down(object_id) {
        return (object.zone == Zone::Exile
            && game.can_player_look_at_face_down_exiled_card(object_id, viewer))
        .then_some(DecisionHiddenCardVisibility::PrivateToDecisionPlayer);
    }

    if object.zone.is_hidden() {
        Some(DecisionHiddenCardVisibility::PrivateToDecisionPlayer)
    } else {
        Some(DecisionHiddenCardVisibility::Public)
    }
}

impl EffectExecutor for MayMoveToZoneEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        let Some(object_id) = object_ids.first().copied() else {
            return Ok(EffectOutcome::count(0));
        };
        let decider =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.decider, ctx)?;
        let description = describe_move(self.zone, game, object_id);
        let mut spec = MaySpec::new(ctx.source, description);
        let decision_player = game.controlling_player_for(decider);
        if let Some(visibility) = hidden_decision_visibility(game, object_id, decision_player) {
            spec = spec.with_hidden_card_view(
                vec![object_id],
                visibility,
                "Inspect hidden card for decision",
            );
        }
        let should_move = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            decider,
            Some(ctx.source),
            spec,
            FallbackStrategy::Decline,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if !should_move {
            return Ok(EffectOutcome::count(0).with_execution_fact(ExecutionFact::Declined));
        }

        let move_effect =
            crate::effects::MoveToZoneEffect::new(self.target.clone(), self.zone, false);
        move_effect.execute(game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "object to optionally move"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::{BooleanContext, ViewCardsContext};
    use crate::target::PlayerFilter;

    #[derive(Default)]
    struct CapturePromptDecisionMaker {
        boolean_contexts: Vec<BooleanContext>,
        views: Vec<(PlayerId, Vec<ObjectId>, ViewCardsContext)>,
    }

    impl DecisionMaker for CapturePromptDecisionMaker {
        fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
            self.boolean_contexts.push(ctx.clone());
            true
        }

        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &ViewCardsContext,
        ) {
            self.views.push((viewer, cards.to_vec(), ctx.clone()));
        }
    }

    #[test]
    fn may_move_hidden_face_up_exiled_card_opens_public_decision_view() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let hidden = game.create_hidden_card_placeholder(
            alice,
            Zone::Exile,
            0,
            "alice-exile-slot-0".to_string(),
        );
        let source = ObjectId::from_raw(7001);
        let mut dm = CapturePromptDecisionMaker::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = MayMoveToZoneEffect::new(
            ChooseSpec::SpecificObject(hidden),
            Zone::Hand,
            PlayerFilter::You,
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("may-move prompt should resolve");
        drop(ctx);

        assert!(
            dm.boolean_contexts.iter().any(|decision| decision
                .ui_hints
                .hidden_card_views
                .iter()
                .any(|view| {
                    view.object_ids == vec![hidden]
                        && view.visibility == DecisionHiddenCardVisibility::Public
                })),
            "the boolean prompt should carry a public hidden-card visibility policy"
        );
        assert!(
            dm.views.iter().any(|(viewer, cards, view_ctx)| {
                *viewer == alice
                    && cards == &vec![hidden]
                    && view_ctx.public
                    && view_ctx.zone == Zone::Exile
            }),
            "the hidden card should be opened publicly before the decision is rendered"
        );
    }
}
