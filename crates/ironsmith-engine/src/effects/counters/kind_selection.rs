//! Counter-kind discovery and lexical choice, independent of the counter action.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::target::ChooseSpec;
use crate::{game_state::GameState, ids::ObjectId, object::CounterType};

pub(super) fn kinds_on(game: &GameState, ids: &[ObjectId]) -> Vec<CounterType> {
    let mut kinds = ids
        .iter()
        .filter_map(|id| game.object(*id))
        .flat_map(|object| {
            object
                .counters
                .iter()
                .filter_map(|(kind, count)| (*count > 0).then_some(*kind))
        })
        .collect::<Vec<_>>();
    kinds.sort_by_key(|kind| format!("{kind:?}"));
    kinds.dedup();
    kinds
}
pub(super) fn counter_label(kind: CounterType) -> String {
    format!("{kind:?}").to_ascii_lowercase()
}
pub(super) fn choose_kind(
    game: &GameState,
    ctx: &mut ExecutionContext,
    kinds: &[CounterType],
) -> Option<CounterType> {
    choose_binding(
        game,
        ctx,
        "Choose a counter kind",
        &kinds
            .iter()
            .map(|kind| (*kind, format!("Choose {} counter", counter_label(*kind))))
            .collect::<Vec<_>>(),
        false,
    )
}
pub(super) fn choose_binding<T: Copy>(
    game: &GameState,
    ctx: &mut ExecutionContext,
    prompt: &str,
    options: &[(T, String)],
    fallback: bool,
) -> Option<T> {
    let choice = crate::decisions::context::SelectOptionsContext::new(
        ctx.controller,
        Some(ctx.source),
        prompt,
        options
            .iter()
            .enumerate()
            .map(|(idx, (_, label))| {
                crate::decisions::context::SelectableOption::new(idx, label.clone())
            })
            .collect(),
        1,
        1,
    );
    let selected = ctx
        .decision_maker
        .decide_options(game, &choice)
        .first()
        .copied();
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    selected
        .and_then(|idx| options.get(idx))
        .or_else(|| fallback.then(|| options.first()).flatten())
        .map(|(value, _)| *value)
}

/// Bind one kind to the fixed recipients. Absence is checked immediately before
/// each child, because an earlier replacement addition may change later objects.
pub(super) fn put_kind_on_recipients(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    kind: CounterType,
    recipients: &[ObjectId],
    exclude: Option<ObjectId>,
    only_if_absent: bool,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let mut children = Vec::new();
    for &id in recipients {
        if Some(id) == exclude
            || (only_if_absent
                && game
                    .object(id)
                    .is_some_and(|object| object.counters.get(&kind).copied().unwrap_or(0) > 0))
        {
            continue;
        }
        let effect = Effect::new(super::PutCountersEffect::new(
            kind,
            1,
            ChooseSpec::SpecificObject(id),
        ));
        children.push(effect.0.execute_child_with_outputs(game, ctx)?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        if ctx.resolution_stopped() {
            break;
        }
    }
    Ok(CompletedEffectOutputs::from_children(
        children,
        EffectOutcome::aggregate_summing_counts,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectExecutor;
    use crate::{
        card::CardBuilder,
        ids::{CardId, PlayerId},
        zone::Zone,
    };
    struct PauseLater {
        asks: usize,
        pending: bool,
    }
    impl crate::decision::DecisionMaker for PauseLater {
        fn decide_options(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.asks += 1;
            self.pending = self.asks == 2;
            vec![0]
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    #[test]
    fn later_kind_choice_rolls_back_earlier_counter_actions() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId(0);
        let id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Counter recipient").build(),
            player,
            Zone::Battlefield,
        );
        game.add_counters(id, CounterType::Charge, 1).unwrap();
        game.add_counters(id, CounterType::Flying, 1).unwrap();
        let mut dm = PauseLater {
            asks: 0,
            pending: false,
        };
        let mut ctx = ExecutionContext::new(id, player, &mut dm);
        let output =
            super::super::ForEachCounterKindPutOrRemoveEffect::new(ChooseSpec::SpecificObject(id))
                .execute_with_outputs(&mut game, &mut ctx)
                .unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert_eq!(output.outcome.count_or_zero(), 0);
        assert_eq!(game.counter_count(id, CounterType::Charge), 1);
        assert_eq!(game.counter_count(id, CounterType::Flying), 1);
    }
    #[test]
    fn chosen_donor_and_existing_kind_are_excluded_from_recipient_program() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId(0);
        let card = CardBuilder::new(CardId::new(), "Recipient").build();
        let ids = (0..3)
            .map(|_| game.create_object_from_card(&card, player, Zone::Battlefield))
            .collect::<Vec<_>>();
        game.add_counters(ids[1], CounterType::Charge, 2).unwrap();
        let mut ctx = ExecutionContext::new_default(ids[0], player);
        let output = put_kind_on_recipients(
            &mut game,
            &mut ctx,
            CounterType::Charge,
            &ids,
            Some(ids[0]),
            true,
        )
        .unwrap();
        assert_eq!(output.outcome.count_or_zero(), 1);
        assert_eq!(game.counter_count(ids[0], CounterType::Charge), 0);
        assert_eq!(game.counter_count(ids[1], CounterType::Charge), 2);
        assert_eq!(game.counter_count(ids[2], CounterType::Charge), 1);
    }
}
