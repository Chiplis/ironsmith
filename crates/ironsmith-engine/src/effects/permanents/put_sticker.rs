use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_single_object_from_spec;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
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
            Err(ExecutionError::InvalidTarget) if self.action == KeywordActionKind::NameSticker => {
                return Ok(EffectOutcome::impossible());
            }
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::count(0)),
            Err(err) => return Err(err),
        };

        let snapshot = game
            .object(object_id)
            .map(|object| ObjectSnapshot::from_object(object, game));
        let name_fact = if self.action == KeywordActionKind::NameSticker {
            use crate::decisions::context::{SelectOptionsContext, SelectableOption};
            let Some(object) = game.object(object_id) else {
                return Ok(EffectOutcome::impossible());
            };
            if object.owner != ctx.controller || !object.zone.is_public() {
                return Ok(EffectOutcome::impossible());
            }
            let controller = game.current_controller(object_id).unwrap_or(object.owner);
            let available = game.available_name_stickers(ctx.controller);
            if available.is_empty() {
                return Ok(EffectOutcome::impossible());
            }
            let options = available
                .iter()
                .enumerate()
                .map(|(i, (_, name))| SelectableOption::new(i, name.clone()))
                .collect();
            let choice = SelectOptionsContext::new(
                ctx.controller,
                Some(ctx.source),
                "Choose a name sticker",
                options,
                1,
                1,
            );
            let selected = ctx.decision_maker.decide_options(game, &choice);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let Some((sticker_id, _)) = selected.first().and_then(|index| available.get(*index))
            else {
                return Ok(EffectOutcome::impossible());
            };
            let name = game.current_name(object_id).unwrap_or_default();
            let word_count = name
                .split_whitespace()
                .filter(|word| !word.chars().all(|ch| ch == '_'))
                .count();
            let positions = (0..=word_count)
                .map(|i| {
                    SelectableOption::new(
                        i,
                        if i == 0 {
                            "At the beginning".to_string()
                        } else {
                            format!("After word {i}")
                        },
                    )
                })
                .collect();
            let choice = SelectOptionsContext::new(
                controller,
                Some(ctx.source),
                "Choose the name sticker's position",
                positions,
                1,
                1,
            );
            let selected = ctx.decision_maker.decide_options(game, &choice);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let Some(position) = selected
                .first()
                .copied()
                .filter(|position| *position <= word_count)
            else {
                return Ok(EffectOutcome::impossible());
            };
            let Some(name) =
                game.apply_available_name_sticker(ctx.controller, object_id, *sticker_id, position)
            else {
                return Ok(EffectOutcome::impossible());
            };
            Some(crate::effect::ExecutionFact::AppliedNameSticker {
                sticker_id: *sticker_id,
                name,
            })
        } else {
            game.put_sticker_on_object(object_id, self.action);
            None
        };
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(self.action, ctx.controller, ctx.source, 1)
                .with_snapshot(snapshot),
            ctx.provenance,
        );

        let mut outcome = EffectOutcome::with_objects(vec![object_id]).with_event(event);
        if let Some(fact) = name_fact {
            outcome = outcome.with_execution_fact(fact);
        }
        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectExecutor;
    use crate::effects::ResolvedTarget;
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
