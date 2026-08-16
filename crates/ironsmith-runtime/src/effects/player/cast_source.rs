//! Cast-source effect implementation.
//!
//! Casts the source card of the resolving effect/ability.

use crate::alternative_cast::CastingMethod;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::zone::Zone;
pub use ironsmith_core::CastSourceEffect;

use super::runtime_helpers::with_spell_cast_event;

fn restore_other_face_after_failed_cast(
    game: &mut GameState,
    source_id: crate::ids::ObjectId,
    original_source: &crate::object::Object,
    trigger_face: Option<&crate::cards::CardDefinition>,
) {
    let Some(source) = game.object_mut(source_id) else {
        return;
    };
    if let Some(trigger_face) = trigger_face {
        source.apply_definition_face(trigger_face);
    } else {
        *source = original_source.clone();
    }
}

/// Effect that casts the source card immediately.
impl EffectExecutor for CastSourceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_id = ctx.source;
        let Some(source_obj) = game.object(source_id) else {
            return Ok(EffectOutcome::target_invalid());
        };

        let original_source = source_obj.clone();
        let trigger_face = if self.cast_other_face {
            ctx.source_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.name != source_obj.name)
                .and_then(|snapshot| {
                    game.linked_face_definition_by_name_or_id(
                        Some(snapshot.name.as_str()),
                        snapshot.card,
                    )
                })
        } else {
            None
        };
        let source_matches_trigger_snapshot = ctx
            .source_snapshot
            .as_ref()
            .is_none_or(|snapshot| snapshot.name == source_obj.name);
        if self.cast_other_face && source_matches_trigger_snapshot {
            if source_obj.linked_face_layout != crate::card::LinkedFaceLayout::TransformLike {
                return Ok(EffectOutcome::target_invalid());
            }
            let Some(other_def) = game.linked_face_definition_by_name_or_id(
                source_obj.other_face_name.as_deref(),
                source_obj.other_face,
            ) else {
                return Ok(EffectOutcome::target_invalid());
            };
            if let Some(source_obj) = game.object_mut(source_id) {
                source_obj.apply_definition_face(&other_def);
            }
        }

        let Some(source_obj) = game.object(source_id) else {
            return Ok(EffectOutcome::target_invalid());
        };

        if source_obj.is_land() {
            if self.cast_other_face {
                restore_other_face_after_failed_cast(
                    game,
                    source_id,
                    &original_source,
                    trigger_face.as_ref(),
                );
            }
            return Ok(EffectOutcome::target_invalid());
        }
        if self.require_exile && source_obj.zone != Zone::Exile {
            if self.cast_other_face {
                restore_other_face_after_failed_cast(
                    game,
                    source_id,
                    &original_source,
                    trigger_face.as_ref(),
                );
            }
            return Ok(EffectOutcome::target_invalid());
        }

        let from_zone = source_obj.zone;
        let mut suspend_alternative_index = if from_zone == Zone::Exile {
            source_obj
                .alternative_casts
                .iter()
                .position(|method| method.suspend_spec().is_some())
        } else {
            None
        };
        if self.cast_as_suspend
            && suspend_alternative_index.is_none()
            && let Some(obj) = game.object_mut(source_id)
        {
            suspend_alternative_index = Some(obj.alternative_casts.len());
            obj.alternative_casts.push(
                crate::alternative_cast::AlternativeCastingMethod::Suspend {
                    cost: crate::mana::ManaCost::new(),
                    time: 0,
                },
            );
        }
        let casting_method = CastingMethod::PlayFrom {
            source: source_id,
            zone: from_zone,
            use_alternative: suspend_alternative_index,
        };
        let result = match crate::game_loop::cast_spell_from_resolving_effect(
            game,
            source_id,
            from_zone,
            ctx.controller,
            &casting_method,
            self.without_paying_mana_cost,
            None,
            ctx.provenance,
            &mut ctx.decision_maker,
        ) {
            Ok(result) => result,
            Err(error) => {
                if self.cast_other_face {
                    restore_other_face_after_failed_cast(
                        game,
                        source_id,
                        &original_source,
                        trigger_face.as_ref(),
                    );
                }
                return Err(ExecutionError::Impossible(error.to_string()));
            }
        };
        let Some(new_id) = result else {
            if self.cast_other_face && !ctx.decision_maker.awaiting_choice() {
                restore_other_face_after_failed_cast(
                    game,
                    source_id,
                    &original_source,
                    trigger_face.as_ref(),
                );
            }
            return if ctx.decision_maker.awaiting_choice() {
                Ok(EffectOutcome::count(0))
            } else {
                Ok(EffectOutcome::impossible())
            };
        };
        Ok(with_spell_cast_event(
            EffectOutcome::with_objects(vec![new_id]),
            game,
            new_id,
            ctx.controller,
            from_zone,
            ctx.provenance,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, LinkedFaceLayout};
    use crate::cards::CardDefinitionBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::{OutcomeStatus, OutcomeValue};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn cast_source_requires_exile_when_requested() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Suspend Probe")
                .card_types(vec![CardType::Sorcery])
                .build(),
            alice,
            Zone::Hand,
        );

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        let outcome = CastSourceEffect::new()
            .without_paying_mana_cost()
            .require_exile()
            .execute(&mut game, &mut ctx)
            .expect("cast source should execute");

        assert_eq!(outcome.status, OutcomeStatus::TargetInvalid);
        assert!(game.stack.is_empty());
    }

    #[test]
    fn cast_source_free_cast_sets_x_to_zero_and_emits_spell_cast_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "X Fireball")
                .card_types(vec![CardType::Sorcery])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X, ManaSymbol::Red]))
                .build(),
            alice,
            Zone::Exile,
        );

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        let outcome = CastSourceEffect::new()
            .without_paying_mana_cost()
            .require_exile()
            .execute(&mut game, &mut ctx)
            .expect("free cast from exile should resolve");

        let OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected the source card to move to the stack");
        };
        let cast_id = ids[0];

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == crate::events::EventKind::SpellCast),
            "cast-source should emit a SpellCast event"
        );

        let stack_entry = game
            .stack
            .iter()
            .find(|entry| entry.object_id == cast_id)
            .expect("cast object should be on the stack");
        assert_eq!(stack_entry.x_value, Some(0));

        let spell = game.object(cast_id).expect("stack spell should exist");
        assert_eq!(spell.zone, Zone::Stack);
        assert_eq!(spell.x_value, Some(0));
    }

    #[test]
    fn cast_source_can_cast_the_linked_transform_face() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let front_id = CardId::from_raw(990_001);
        let back_id = CardId::from_raw(990_002);
        let front = CardDefinitionBuilder::new(front_id, "Test Siege")
            .card_types(vec![CardType::Battle])
            .subtypes(vec![crate::types::Subtype::Siege])
            .defense(3)
            .other_face(back_id)
            .other_face_name("Test Victory")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        let back = CardDefinitionBuilder::new(back_id, "Test Victory")
            .card_types(vec![CardType::Sorcery])
            .other_face(front_id)
            .other_face_name("Test Siege")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        game.register_linked_face_definition(&back);
        let source_id = game.create_object_from_definition(&front, alice, Zone::Exile);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source_id).expect("front face"),
            &game,
        );

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx =
            ExecutionContext::new(source_id, alice, &mut dm).with_source_snapshot(snapshot);
        let outcome = CastSourceEffect::new()
            .without_paying_mana_cost()
            .require_exile()
            .other_face()
            .execute(&mut game, &mut ctx)
            .expect("linked face should be cast");

        let OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected the linked face on the stack");
        };
        assert_eq!(game.object(ids[0]).expect("spell").name, "Test Victory");
        assert!(game.stack.iter().any(|entry| entry.object_id == ids[0]));
    }

    #[test]
    fn cancelled_resumed_other_face_cast_restores_the_trigger_face() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let front_id = CardId::from_raw(990_011);
        let back_id = CardId::from_raw(990_012);
        let front = CardDefinitionBuilder::new(front_id, "Rollback Siege")
            .card_types(vec![CardType::Battle])
            .subtypes(vec![crate::types::Subtype::Siege])
            .defense(3)
            .other_face(back_id)
            .other_face_name("Uncastable Victory")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        let back = CardDefinitionBuilder::new(back_id, "Uncastable Victory")
            .card_types(vec![CardType::Land])
            .other_face(front_id)
            .other_face_name("Rollback Siege")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
        let source_id = game.create_object_from_definition(&front, alice, Zone::Exile);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source_id).expect("front face"),
            &game,
        );
        game.object_mut(source_id)
            .expect("source")
            .apply_definition_face(&back);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx =
            ExecutionContext::new(source_id, alice, &mut dm).with_source_snapshot(snapshot);
        let _ = CastSourceEffect::new()
            .without_paying_mana_cost()
            .require_exile()
            .other_face()
            .execute(&mut game, &mut ctx)
            .expect("the failed cast should roll back cleanly");

        let restored = game.object(source_id).expect("restored front face");
        assert_eq!(restored.name, "Rollback Siege");
        assert!(restored.card_types.contains(&CardType::Battle));
    }
}
