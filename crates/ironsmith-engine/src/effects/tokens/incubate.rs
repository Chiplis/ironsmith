//! Incubate keyword action implementation.

use crate::cards::tokens::incubator_token_definitions;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::KeywordActionKind;
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

pub type IncubateEffect = ironsmith_core::IncubateEffect;

fn execute_token_instruction_with_outputs(
    effect: &IncubateEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let controller = resolve_player_filter(game, &effect.controller, ctx)?;
    let amount = resolve_value(game, &effect.amount, ctx)?.max(0) as u32;
    let count = resolve_value(game, &effect.count, ctx)?.max(0) as usize;
    if count == 0 {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::with_objects(Vec::new()),
        ));
    }
    game.reserve_token_repetition_work(count)?;
    let (front, back) = incubator_token_definitions();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    let counters = if amount == 0 {
        Vec::new()
    } else {
        vec![(CounterType::PlusOnePlusOne, amount)]
    };
    let request = crate::effects::CreateTokenEffect::new(
        front,
        1,
        crate::target::PlayerFilter::Specific(controller),
    );
    let mut outcomes = super::resources::buffer(count)?;
    let mut created = Vec::new();
    for _ in 0..count {
        let outcome = super::create_tokens_with_entry_counters_with_outputs(
            &request,
            game,
            ctx,
            counters.clone(),
            Some((KeywordActionKind::Incubate, amount)),
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        created.extend_from_slice(outcome.outcome.instruction_result().output_objects());
        outcomes.push(outcome);
    }
    Ok(crate::effects::CompletedEffectOutputs::from_children(
        outcomes,
        |children| {
            EffectOutcome::aggregate_with_primary_result(
                EffectOutcome::with_objects(created),
                children,
            )
        },
    ))
}

impl EffectExecutor for IncubateEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::lifecycle::execute_token_instruction_with_pending_value(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| execute_token_instruction_with_outputs(self, game, ctx),
        )
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
        assert_eq!(token.name, "Incubator Token");
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

        let mut transform_back_ctx = ExecutionContext::new_default(token_id, alice);
        TransformEffect::source().execute(&mut game, &mut transform_back_ctx)
            .expect("the same linked token can transform back");
        let front = game.object(token_id).unwrap();
        assert_eq!(front.name, "Incubator Token");
        assert!(front.subtypes.contains(&Subtype::Incubator));
        assert!(!game.current_is_creature(token_id));
        assert_eq!(game.counter_count(token_id, CounterType::PlusOnePlusOne), 3);
        assert_eq!(front.abilities.len(), 1);
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
            assert_eq!(token.name, "Incubator Token");
            assert_eq!(game.controller_of(token), alice);
            assert_eq!(game.counter_count(id, CounterType::PlusOnePlusOne), 2);
        }
    }
}
