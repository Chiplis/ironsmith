//! Connive effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::DrawCardsEffect;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    normalize_object_selection, resolve_objects_for_effect, resolve_value,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::cause::EventCause;
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::types::CardType;

/// Effect that makes target creature(s) connive.
///
/// Connive: Draw a card, then discard a card.
/// If a nonland card was discarded this way, put a +1/+1 counter on that creature.
#[derive(Debug, Clone, PartialEq)]
pub struct ConniveEffect {
    pub target: ChooseSpec,
    pub count: Value,
}

#[derive(Debug, Clone)]
struct ConniveInstruction {
    object_id: ObjectId,
    controller: PlayerId,
    snapshot: ObjectSnapshot,
}

impl ConniveEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self::new_with_count(target, Value::Fixed(1))
    }

    pub fn new_with_count(target: ChooseSpec, count: impl Into<Value>) -> Self {
        Self {
            target,
            count: count.into(),
        }
    }
}

fn players_in_apnap_order(game: &GameState) -> Vec<PlayerId> {
    game.team_apnap_player_order()
}

fn connive_snapshot_for_object(
    game: &GameState,
    ctx: &ExecutionContext,
    object_id: ObjectId,
) -> Option<ObjectSnapshot> {
    if let Some(object) = game.object(object_id) {
        return Some(ObjectSnapshot::from_object_with_calculated_characteristics(
            object, game,
        ));
    }
    if let Some(snapshot) = ctx.target_snapshots.get(&object_id) {
        return Some(snapshot.clone());
    }
    if let Some(snapshot) = ctx.source_snapshot.as_ref()
        && snapshot.object_id == object_id
    {
        return Some(snapshot.clone());
    }
    if let Some(snapshot) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.snapshot().cloned())
        && snapshot.object_id == object_id
    {
        return Some(snapshot);
    }
    ctx.tagged_objects
        .values()
        .flat_map(|snapshots| snapshots.iter())
        .find(|snapshot| snapshot.object_id == object_id)
        .cloned()
}

fn connive_tagged_object_ids(ctx: &ExecutionContext, spec: &ChooseSpec) -> Option<Vec<ObjectId>> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } | ChooseSpec::Target(spec) => {
            connive_tagged_object_ids(ctx, spec)
        }
        ChooseSpec::WithCount(spec, _) | ChooseSpec::WithCountValue(spec, _, _) => {
            connive_tagged_object_ids(ctx, spec)
        }
        ChooseSpec::Tagged(tag) => {
            if let Some(snapshots) = ctx.get_tagged_all(tag)
                && !snapshots.is_empty()
            {
                return Some(
                    snapshots
                        .iter()
                        .map(|snapshot| snapshot.object_id)
                        .collect::<Vec<_>>(),
                );
            }
            matches!(tag.as_str(), "triggering" | "__it__" | "it").then_some(vec![ctx.source])
        }
        _ => None,
    }
}

impl EffectExecutor for ConniveEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let (target_ids, from_tagged_lki) = match connive_tagged_object_ids(ctx, &self.target) {
            Some(ids) => (ids, true),
            None => (resolve_objects_for_effect(game, ctx, &self.target)?, false),
        };
        if target_ids.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        let mut remaining = target_ids
            .into_iter()
            .filter_map(|object_id| {
                let snapshot = connive_snapshot_for_object(game, ctx, object_id)?;
                if !snapshot.has_card_type(CardType::Creature)
                    && !(from_tagged_lki && game.object(object_id).is_none())
                {
                    return None;
                }
                Some(ConniveInstruction {
                    object_id,
                    controller: snapshot.controller,
                    snapshot,
                })
            })
            .collect::<Vec<_>>();
        if remaining.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        let mut events = Vec::new();
        let mut connived_objects = Vec::new();
        let player_order = players_in_apnap_order(game);

        for player in player_order {
            while let Some(candidate_indices) = (!remaining.is_empty()).then(|| {
                remaining
                    .iter()
                    .enumerate()
                    .filter_map(|(index, instruction)| {
                        (instruction.controller == player).then_some(index)
                    })
                    .collect::<Vec<_>>()
            }) {
                if candidate_indices.is_empty() {
                    break;
                }

                let chosen_index = if candidate_indices.len() == 1 {
                    candidate_indices[0]
                } else {
                    use crate::decisions::make_decision;
                    use crate::decisions::specs::ChooseObjectsSpec;

                    let choices = candidate_indices
                        .iter()
                        .map(|&index| remaining[index].object_id)
                        .collect::<Vec<_>>();
                    let spec = ChooseObjectsSpec::new(
                        ctx.source,
                        "Choose a permanent to connive next",
                        choices.clone(),
                        1,
                        Some(1),
                    );
                    let selection: Vec<ObjectId> =
                        make_decision(game, ctx.decision_maker, player, Some(ctx.source), spec);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(
                            EffectOutcome::with_objects(connived_objects).with_events(events)
                        );
                    }
                    let normalized = normalize_object_selection(selection, &choices, 1);
                    let chosen_object = normalized.first().copied().unwrap_or(choices[0]);
                    candidate_indices
                        .into_iter()
                        .find(|index| remaining[*index].object_id == chosen_object)
                        .unwrap_or(0)
                };

                let instruction = remaining.remove(chosen_index);
                let controller = instruction.controller;
                let target_id = instruction.object_id;

                // Rule 701.50e: connive N draws N, discards N, then counts nonlands discarded this way.
                let draw_outcome =
                    DrawCardsEffect::new(count as i32, PlayerFilter::Specific(controller))
                        .execute(game, ctx)?;
                events.extend(draw_outcome.events);

                // Then discard that many cards if possible.
                let hand_cards: Vec<ObjectId> = game
                    .player(controller)
                    .map(|p| p.hand.iter().copied().collect())
                    .unwrap_or_default();

                let required = count.min(hand_cards.len());
                if required > 0 {
                    use crate::decisions::make_decision;
                    use crate::decisions::specs::ChooseObjectsSpec;
                    use crate::events::processing::execute_discard;

                    let spec = ChooseObjectsSpec::new(
                        ctx.source,
                        format!(
                            "Choose {} card{} to discard for connive",
                            required,
                            if required == 1 { "" } else { "s" }
                        ),
                        hand_cards.clone(),
                        required,
                        Some(required),
                    );
                    let chosen: Vec<_> =
                        make_decision(game, ctx.decision_maker, controller, Some(ctx.source), spec);
                    let selected = normalize_object_selection(chosen, &hand_cards, required);
                    let mut discarded_nonlands = 0;
                    for card_to_discard in selected {
                        let discarded_nonland = game
                            .object(card_to_discard)
                            .map(|obj| !obj.has_card_type(CardType::Land))
                            .unwrap_or(false);
                        let discard_result = execute_discard(
                            game,
                            card_to_discard,
                            controller,
                            EventCause::from_effect(ctx.source, ctx.controller),
                            false,
                            ctx.provenance,
                            &mut *ctx.decision_maker,
                        );

                        if !discard_result.prevented && discarded_nonland {
                            discarded_nonlands += 1;
                        }
                    }

                    if discarded_nonlands > 0
                        && let Some(event) = game.add_counters_with_source(
                            target_id,
                            crate::object::CounterType::PlusOnePlusOne,
                            discarded_nonlands,
                            Some(ctx.source),
                            Some(ctx.controller),
                        )
                    {
                        events.push(event);
                    }
                }

                events.push(TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(
                        KeywordActionKind::Connive,
                        controller,
                        target_id,
                        count as u32,
                    )
                    .with_snapshot(Some(instruction.snapshot)),
                    ctx.provenance,
                ));
                connived_objects.push(target_id);
            }
        }

        Ok(EffectOutcome::with_objects(connived_objects).with_events(events))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to connive"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectObjectsContext;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::zone::Zone;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct ConniveDecisionMaker {
        object_choices: VecDeque<Vec<ObjectId>>,
    }

    impl DecisionMaker for ConniveDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.object_choices
                .pop_front()
                .unwrap_or_default()
                .into_iter()
                .filter(|id| {
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.legal && candidate.id == *id)
                })
                .collect()
        }
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_card_to_hand(
        game: &mut GameState,
        owner: PlayerId,
        card_types: Vec<CardType>,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Hand Card")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(card_types)
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    fn create_creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Conniver")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn connive_puts_counter_when_nonland_discarded() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_creature(&mut game, alice);
        add_card_to_hand(&mut game, alice, vec![CardType::Instant]);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ConniveEffect::new(ChooseSpec::SpecificObject(creature));
        let result = effect.execute(&mut game, &mut ctx).unwrap();
        assert!(result.status.is_success());
        assert_eq!(
            game.object(creature)
                .and_then(|obj| obj
                    .counters
                    .get(&crate::object::CounterType::PlusOnePlusOne))
                .copied()
                .unwrap_or(0),
            1
        );
    }

    #[test]
    fn connive_does_not_put_counter_when_land_discarded() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_creature(&mut game, alice);
        add_card_to_hand(&mut game, alice, vec![CardType::Land]);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ConniveEffect::new(ChooseSpec::SpecificObject(creature));
        let result = effect.execute(&mut game, &mut ctx).unwrap();
        assert!(result.status.is_success());
        assert_eq!(
            game.object(creature)
                .and_then(|obj| obj
                    .counters
                    .get(&crate::object::CounterType::PlusOnePlusOne))
                .copied()
                .unwrap_or(0),
            0
        );
    }

    #[test]
    fn connive_n_discards_n_and_counts_only_nonlands_for_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_creature(&mut game, alice);
        add_card_to_hand(&mut game, alice, vec![CardType::Instant]);
        add_card_to_hand(&mut game, alice, vec![CardType::Sorcery]);
        add_card_to_hand(&mut game, alice, vec![CardType::Land]);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ConniveEffect::new_with_count(ChooseSpec::SpecificObject(creature), 2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert!(result.status.is_success());
        assert_eq!(
            game.object(creature)
                .and_then(|obj| obj
                    .counters
                    .get(&crate::object::CounterType::PlusOnePlusOne))
                .copied()
                .unwrap_or(0),
            2
        );

        let event = result
            .events
            .iter()
            .find_map(|event| event.downcast::<KeywordActionEvent>())
            .expect("expected connive keyword action");
        assert_eq!(event.action, KeywordActionKind::Connive);
        assert_eq!(event.player, alice);
        assert_eq!(event.source, creature);
        assert_eq!(event.amount, 2);
    }

    #[test]
    fn connive_uses_last_known_information_for_permanent_that_left_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_creature(&mut game, alice);
        let snapshot = ObjectSnapshot::from_object(game.object(creature).expect("creature"), &game);
        let moved = game.move_object_by_effect(creature, Zone::Graveyard);
        assert!(moved.is_some());
        assert!(game.object(creature).is_none());

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(creature)]);
        ctx.target_snapshots.insert(creature, snapshot);

        let result = ConniveEffect::new(ChooseSpec::SpecificObject(creature))
            .execute(&mut game, &mut ctx)
            .unwrap();

        assert!(result.status.is_success());
        let event = result
            .events
            .iter()
            .find_map(|event| event.downcast::<KeywordActionEvent>())
            .expect("expected connive keyword action");
        assert_eq!(event.action, KeywordActionKind::Connive);
        assert_eq!(event.player, alice);
        assert_eq!(event.source, creature);
        assert_eq!(
            event.snapshot.as_ref().map(|snapshot| snapshot.object_id),
            Some(creature)
        );
    }

    #[test]
    fn multiple_connive_instructions_use_apnap_and_controller_choice_order() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;
        let source = game.new_object_id();
        let alice_creature = create_creature(&mut game, alice);
        let bob_first = create_creature(&mut game, bob);
        let bob_second = create_creature(&mut game, bob);
        let mut dm = ConniveDecisionMaker {
            object_choices: VecDeque::from([vec![bob_second]]),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let result = ConniveEffect::new(ChooseSpec::all(crate::target::ObjectFilter::creature()))
            .execute(&mut game, &mut ctx)
            .unwrap();

        let connive_events = result
            .events
            .iter()
            .filter_map(|event| event.downcast::<KeywordActionEvent>())
            .filter(|event| event.action == KeywordActionKind::Connive)
            .map(|event| (event.player, event.source))
            .collect::<Vec<_>>();
        assert_eq!(
            connive_events,
            vec![(bob, bob_second), (bob, bob_first), (alice, alice_creature)]
        );
    }
}
