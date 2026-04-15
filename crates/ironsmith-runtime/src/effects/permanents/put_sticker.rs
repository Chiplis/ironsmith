use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_single_object_from_spec;
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct PutStickerEffect {
    pub target: ChooseSpec,
    pub action: KeywordActionKind,
}

impl PutStickerEffect {
    pub fn new(target: ChooseSpec, action: KeywordActionKind) -> Self {
        Self { target, action }
    }
}

impl EffectExecutor for PutStickerEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_id = match resolve_single_object_from_spec(game, &self.target, ctx) {
            Ok(id) => id,
            Err(ExecutionError::InvalidTarget) if self.target.is_target() => {
                return Ok(EffectOutcome::target_invalid());
            }
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::count(0)),
            Err(err) => return Err(err),
        };

        let snapshot = game
            .object(object_id)
            .map(|object| ObjectSnapshot::from_object(object, game));
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(self.action, ctx.controller, ctx.source, 1)
                .with_snapshot(snapshot),
            ctx.provenance,
        );

        Ok(EffectOutcome::with_objects(vec![object_id]).with_event(event))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectExecutor;
    use crate::executor::ResolvedTarget;
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;

    #[test]
    fn put_sticker_effect_emits_sticker_action_for_chosen_object() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = game.create_object_from_card(
            &crate::card::CardBuilder::new(CardId::from_raw(1), "Sticker Source")
                .card_types(vec![crate::types::CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let target_id = game.create_object_from_card(
            &crate::card::CardBuilder::new(CardId::from_raw(2), "Sticker Target")
                .card_types(vec![crate::types::CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );

        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.targets.push(ResolvedTarget::Object(target_id));

        let effect = PutStickerEffect::new(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact())),
            KeywordActionKind::Sticker,
        );
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("put sticker should resolve");

        assert_eq!(
            outcome.value.objects().expect("sticker target outcome"),
            &[target_id]
        );
        let events_debug = format!("{:?}", outcome.events);
        assert!(events_debug.contains("put a sticker"), "{events_debug}");
        assert!(events_debug.contains("KeywordAction"), "{events_debug}");
    }
}
