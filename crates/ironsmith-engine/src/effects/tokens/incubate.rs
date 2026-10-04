//! Incubate keyword action implementation.

use crate::cards::tokens::incubator_token_definitions;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::lifecycle::{
    TokenEntryOptions, apply_token_battlefield_entry, create_replacement_additional_tokens,
};

pub type IncubateEffect = ironsmith_core::IncubateEffect;

fn execute_token_instruction(
    effect: &IncubateEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    let controller_id = resolve_player_filter(game, &effect.controller, ctx)?;
    let amount = resolve_value(game, &effect.amount, ctx)?.max(0) as u32;
    let count = resolve_value(game, &effect.count, ctx)?.max(0) as usize;

    game.reserve_token_repetition_work(count)?;
    let mut created_ids = super::resources::buffer(count)?;
    let mut events = super::resources::buffer(count * 2)?;
    let mut replacement_outcomes = Vec::new();
    let mut committed_outcomes = Vec::new();
    let entry_options = TokenEntryOptions::default();

    for _ in 0..count {
        let (front, back) = incubator_token_definitions();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);

        // CR 701.53a: incubating creates an Incubator token, so token
        // creation replacements (Doubling Season, Parallel Lives, ...)
        // apply (CR 111.1, 614.1). Host exhaustion is an error, not a rule.
        let token_preview = game.object_from_token_definition(
            crate::ids::ObjectId::from_raw(0),
            &front,
            controller_id,
        );
        let mut committed_original = false;
        let completed = crate::events::processing::execute_token_creation_with_event(
            game,
            controller_id,
            1,
            Some(token_preview.clone()),
            ctx.cause.clone(),
            ctx,
            |game, ctx, replacement, provenance| {
                ctx.provenance = provenance;
                committed_original = true;
        // Keep this receipt's observations separate from earlier iterations
        // whose original creation was replaced completely.
        let mut events = Vec::new();
    let mut entry_receipts = Vec::new();
        let controller_id = replacement.controller;
        let token_preview = replacement.token.clone().unwrap_or(token_preview);
        game.reserve_token_creation(replacement.total_count())?;
        let token_count = replacement.count as usize;

        let mut incubated_ids = super::resources::buffer(token_count)?;
        for _ in 0..token_count {
            let id = game.new_object_id();
            let mut token_obj = game.object_from_token_definition(id, &front, controller_id);
            token_obj.zone = Zone::Command;
            let token_is_creature = token_obj.is_creature();

            game.commit_token_resource_slot()?;
        game.add_object(token_obj);

            let initial_counters = if amount > 0 {
                vec![(CounterType::PlusOnePlusOne, amount)]
            } else {
                Vec::new()
            };
            let entry_result = game.move_object_with_etb_processing_with_initial_counters_with_dm(
                id,
                Zone::Battlefield,
                initial_counters,
                &mut ctx.decision_maker,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::with_objects(Vec::new()));
            }
            let Some(entry_result) = super::lifecycle::retain_token_entry_receipt(game, id, entry_result, &mut entry_receipts)? else {
                game.remove_object(id);
                continue;
            };

            let entered_id = entry_result.new_id;
            incubated_ids.push(entered_id);

            let entered_battlefield = game
                .object(entered_id)
                .is_some_and(|obj| obj.zone == Zone::Battlefield);
            if entered_battlefield {
                let entered_is_creature = game.current_is_creature(entered_id);
                let tracks_creature_etb = entered_is_creature || token_is_creature;
                apply_token_battlefield_entry(
                    game,
                    ctx,
                    entered_id,
                    controller_id,
                    tracks_creature_etb,
                    entry_options,
                    Zone::Command,
                    entry_result.enters_tapped,
                    &mut events,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::with_objects(Vec::new()));
                }
            }
        }

        let mut actual_creation = replacement.clone();
        actual_creation.count = incubated_ids.len() as u32;
        actual_creation.token = Some(token_preview);
        let mut iteration_ids = incubated_ids;
        let additional_ids = create_replacement_additional_tokens(
            game,
            ctx,
            controller_id,
            &mut actual_creation,
            &super::lifecycle::AdditionalTokenInstructions {
                entry: entry_options,
                initial_counters: if amount > 0 { vec![(CounterType::PlusOnePlusOne, amount)] } else { Vec::new() },
                ..Default::default()
            },
            &mut events,
        &mut entry_receipts,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::with_objects(Vec::new()));
        }
        iteration_ids.extend(additional_ids);
        super::lifecycle::publish_created_token_groups(game, ctx, actual_creation, &mut events);

        events.push(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(
                KeywordActionKind::Incubate,
                controller_id,
                ctx.source,
                amount,
            ),
            ctx.provenance,
        ));

        // Complete this creation and its appended programs before the next
        // incubate iteration. Retain actual successor IDs in the receipt.
        let original = EffectOutcome::with_objects(iteration_ids.clone())
            .with_result_objects(iteration_ids.clone())
            .with_events(std::mem::take(&mut events))
            .with_affected_objects_from_game(game, iteration_ids.clone());
                created_ids.extend(iteration_ids);
                crate::effects::zones::finish_zone_change_receipts(game, ctx, original, entry_receipts)
            },
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::with_objects(Vec::new()));
        }
        if committed_original {
            committed_outcomes.push(completed);
        } else {
            replacement_outcomes.push(completed);
            events.push(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::Incubate,
                    controller_id,
                    ctx.source,
                    amount,
                ),
                ctx.provenance,
            ));
        }
    }

    // Each committed receipt already owns its events and execution facts;
    // aggregate those once and retain the whole instruction's token summary.
    // Keyword observations from finished replacements remain in `events`.
    let mut original = EffectOutcome::aggregate(committed_outcomes);
    original.value = crate::effect::OutcomeValue::Objects(created_ids);
    original.events.extend(events);
    if replacement_outcomes.is_empty() {
        Ok(original)
    } else {
        replacement_outcomes.push(original);
        Ok(EffectOutcome::aggregate(replacement_outcomes))
    }
}

impl EffectExecutor for IncubateEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::lifecycle::execute_token_instruction_atomically(game, ctx, |game, ctx| {
            execute_token_instruction(self, game, ctx)
        })
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.controller_target.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player to incubate"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::LinkedFaceLayout;
    use crate::effect::Value;
    use crate::effects::TransformEffect;
    use crate::ids::PlayerId;
    use crate::types::{CardType, Subtype};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn incubate_creates_incubator_with_counters_and_transform_face() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = IncubateEffect::you(Value::Fixed(3), Value::Fixed(1))
            .execute(&mut game, &mut ctx)
            .expect("incubate should resolve");

        let ids = outcome.objects().expect("incubate should create a token");
        assert_eq!(ids.len(), 1);
        let token_id = ids[0];
        let token = game.object(token_id).expect("incubator should exist");
        assert_eq!(token.name, "Incubator");
        assert!(token.card_types.contains(&CardType::Artifact));
        assert!(token.subtypes.contains(&Subtype::Incubator));
        assert!(!game.current_is_creature(token_id));
        assert_eq!(game.counter_count(token_id, CounterType::PlusOnePlusOne), 3);
        assert_eq!(token.linked_face_layout, LinkedFaceLayout::TransformLike);
        assert_eq!(token.abilities.len(), 1);

        let mut transform_ctx = ExecutionContext::new_default(token_id, alice);
        TransformEffect::source()
            .execute(&mut game, &mut transform_ctx)
            .expect("incubator should transform");

        let transformed = game
            .object(token_id)
            .expect("transformed token should exist");
        assert_eq!(transformed.name, "Phyrexian Token");
        assert!(transformed.card_types.contains(&CardType::Artifact));
        assert!(transformed.card_types.contains(&CardType::Creature));
        assert!(transformed.subtypes.contains(&Subtype::Phyrexian));
        assert_eq!(game.counter_count(token_id, CounterType::PlusOnePlusOne), 3);
        assert_eq!(game.calculated_power(token_id), Some(3));
        assert_eq!(game.calculated_toughness(token_id), Some(3));
    }

    #[test]
    fn incubate_count_creates_multiple_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = IncubateEffect::you(Value::Fixed(2), Value::Fixed(3))
            .execute(&mut game, &mut ctx)
            .expect("incubate should resolve");

        let ids = outcome.objects().expect("incubate should create tokens");
        assert_eq!(ids.len(), 3);
        for &id in ids {
            let token = game.object(id).expect("incubator should exist");
            assert_eq!(token.name, "Incubator");
            assert_eq!(game.controller_of(token), alice);
            assert_eq!(game.counter_count(id, CounterType::PlusOnePlusOne), 2);
        }
    }
}
