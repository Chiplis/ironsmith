//! Connive effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::DrawCardsEffect;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_value};
use crate::effects::{CompletedEffectOutputs, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::CardType;

/// Effect that makes target creature(s) connive.
///
/// Connive: Draw a card, then discard a card.
/// If a nonland card was discarded this way, put a +1/+1 counter on that creature.
#[derive(Debug, Clone, PartialEq)]
pub struct ConniveEffect {
    pub target: ChooseSpec,
    pub count: Value,
    pub descriptive: bool,
}

impl ConniveEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self::new_with_count(target, Value::Fixed(1))
    }

    pub fn new_with_count(target: ChooseSpec, count: impl Into<Value>) -> Self {
        Self {
            target,
            count: count.into(),
            descriptive: false,
        }
    }

    fn connive_once(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        action: &KeywordActionEvent,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let count = i32::try_from(action.amount)
            .map_err(|_| ExecutionError::InternalError("connive magnitude overflow".into()))?;
        if count == 0 {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::with_objects(Vec::new()),
            ));
        }
        let mut outputs = connive_body(
            ChooseSpec::SpecificObject(action.source),
            PlayerFilter::Specific(action.player),
            count.into(),
        )
        .0
        .execute_child_with_outputs(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(outputs);
        }
        let mut children = vec![outputs.outcome.clone()];
        if !self.descriptive {
            let notification = crate::effects::composition::complete_keyword_action(
                game,
                ctx,
                KeywordActionEvent::new(
                    KeywordActionKind::Connive,
                    action.player,
                    action.source,
                    action.amount,
                )
                .with_snapshot(action.snapshot.clone()),
            )?;
            children.push(notification.clone());
            outputs.retain_batch_children([CompletedEffectOutputs::aggregate_only(notification)]);
        }
        Ok(
            outputs.project_aggregate(EffectOutcome::aggregate_with_primary_result(
                EffectOutcome::with_objects(vec![action.source]),
                children,
            )),
        )
    }
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
    // A trigger can be queued before control changes and the permanent
    // departs. The departure receipt is newer than the entry/stack snapshot.
    // Match the exact incarnation; a later return must not replace its LKI.
    if let Some(snapshot) = game.source_last_known_snapshot(object_id) {
        return Some(snapshot.clone());
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

/// One connive instruction's body uses an exact, locally scoped discard
/// receipt. Values are read from the original discard, excluding replacement
/// additions and cards merely selected for a prevented discard.
fn connive_body(target: ChooseSpec, player: PlayerFilter, count: Value) -> crate::effect::Effect {
    use crate::effect::{Effect, EffectId};
    use ironsmith_core::{
        EffectMetric, EffectMetricSource, PriorEffectAction, PriorEffectMetricQuery,
    };
    const DISCARD_RESULT: EffectId = EffectId(0xFFFF_FD00);
    let mut nonland = crate::target::ObjectFilter::default();
    nonland.excluded_card_types.push(CardType::Land);
    let mut query =
        PriorEffectMetricQuery::new(EffectMetricSource::AffectedObjects, EffectMetric::Count)
            .with_filter(nonland);
    query.action = Some(PriorEffectAction::Discarded);
    query.player = Some(player.clone());
    let quantity = Value::PriorEffectMetric {
        effect_id: DISCARD_RESULT,
        query,
    };
    let counters = Effect::conditional_only(
        crate::effect::Condition::ValueComparison {
            left: quantity.clone(),
            operator: ironsmith_core::ValueComparisonOperator::GreaterThan,
            right: 0.into(),
        },
        vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            quantity,
            target,
        ))],
    );
    Effect::new(crate::effects::SequenceEffect::new(vec![
        Effect::new(DrawCardsEffect::new(count.clone(), player.clone())),
        Effect::new(crate::effects::composition::ScopedOutcomeProgram {
            id: DISCARD_RESULT,
            preserve_producer_result: false,
            producer: Effect::new(crate::effects::DiscardEffect::new(count, player, false)),
            continuation: vec![counters],
        }),
    ]))
}

impl EffectExecutor for ConniveEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        visitor(&connive_body(
            self.target.clone(),
            PlayerFilter::You,
            self.count.clone(),
        ));
    }

    #[crate::effects::project_effect_outcome]
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                // CR 701.50e: zero is not a connive event, including for replacements.
                let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
                if count == 0 {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let target_ids = match connive_tagged_object_ids(ctx, &self.target) {
                    Some(ids) => ids,
                    // CR 701.50c: a source that changed zones still connives, using
                    // its last known information.
                    None if matches!(self.target.base(), ChooseSpec::Source)
                        && ctx.source_snapshot.is_some()
                        && resolve_objects_for_effect(game, ctx, &self.target).is_err() =>
                    {
                        vec![ctx.source]
                    }
                    None => resolve_objects_for_effect(game, ctx, &self.target)?,
                };
                if target_ids.is_empty() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }

                let actions = target_ids
                    .into_iter()
                    .filter_map(|object_id| {
                        let snapshot = connive_snapshot_for_object(game, ctx, object_id)?;
                        // CR 701.50a-b: the instructed permanent connives even if it
                        // stopped being a creature. Target legality, where relevant,
                        // is handled by the target specification/resolution pipeline.
                        Some(
                            KeywordActionEvent::new(
                                KeywordActionKind::Connive, snapshot.controller, object_id, count as u32,
                            )
                            .with_snapshot(Some(snapshot)),
                        )
                    })
                    .collect::<Vec<_>>();
                if actions.is_empty() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }

                crate::effects::composition::execute_ordered_permanent_actions(
                    game, ctx, actions, "Choose a permanent to connive next", self.descriptive,
                    crate::effects::composition::KeywordActionAmount::BodyMagnitude,
                    |game, ctx, action| self.connive_once(game, ctx, action),
                )
            },
        )
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

    /// Surfaces the first object prompt without answering it, like the UI's
    /// replay decision maker on its first pass.
    #[derive(Default)]
    struct AwaitingDecisionMaker {
        prompted: bool,
    }

    impl DecisionMaker for AwaitingDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.prompted
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.prompted = true;
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.min)
                .collect()
        }
    }

    #[test]
    fn connive_does_not_commit_a_fallback_discard_while_awaiting_the_choice() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_creature(&mut game, alice);
        let instant = add_card_to_hand(&mut game, alice, vec![CardType::Instant]);
        let sorcery = add_card_to_hand(&mut game, alice, vec![CardType::Sorcery]);
        let graveyard_before = game.player(alice).expect("Alice").graveyard.len();

        let mut dm = AwaitingDecisionMaker::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let result = ConniveEffect::new(ChooseSpec::SpecificObject(creature))
            .execute(&mut game, &mut ctx)
            .unwrap();

        let alice_state = game.player(alice).expect("Alice");
        assert!(
            alice_state.hand.contains(&instant) && alice_state.hand.contains(&sorcery),
            "no card may be discarded before the player chooses"
        );
        assert_eq!(alice_state.graveyard.len(), graveyard_before);
        assert_eq!(
            game.object(creature)
                .and_then(|obj| obj
                    .counters
                    .get(&crate::object::CounterType::PlusOnePlusOne))
                .copied()
                .unwrap_or(0),
            0,
            "no counter may be placed before the discard is chosen"
        );
        assert!(
            !result.events.iter().any(|event| event
                .downcast::<KeywordActionEvent>()
                .is_some_and(|event| event.action == KeywordActionKind::Connive)),
            "the creature has not connived yet"
        );
    }

    fn add_card_to_library(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Library Card")
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, owner, Zone::Library)
    }

    #[test]
    fn leader_replacement_draws_first_then_connives_exactly_once() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice);
        let leader = create_creature(&mut game, alice);
        game.object_mut(leader)
            .expect("leader exists")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::keyword_action_replacement(
                    KeywordActionKind::Connive,
                    crate::target::ObjectFilter::creature().you_control(),
                    vec![
                        crate::effect::Effect::draw(1),
                        crate::effect::Effect::new(ConniveEffect::new(ChooseSpec::Tagged(
                            crate::tag::TagKey::from("it"),
                        ))),
                    ],
                    "If a creature you control would connive, instead you draw a card, then that creature connives.",
                ),
            ));
        for _ in 0..3 {
            add_card_to_library(&mut game, alice);
        }
        let hand_before = game.player(alice).expect("Alice").hand.len();
        let library_before = game.player(alice).expect("Alice").library.len();

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = ConniveEffect::new(ChooseSpec::SpecificObject(creature))
            .execute(&mut game, &mut ctx)
            .unwrap();

        let alice_state = game.player(alice).expect("Alice");
        assert_eq!(
            library_before - alice_state.library.len(),
            2,
            "one draw from the replacement plus the connive's own draw"
        );
        assert_eq!(
            alice_state.hand.len(),
            hand_before + 1,
            "two draws, one discard"
        );
        let connives = result
            .events
            .iter()
            .filter_map(|event| event.downcast::<KeywordActionEvent>())
            .filter(|event| event.action == KeywordActionKind::Connive)
            .map(|event| event.source)
            .collect::<Vec<_>>();
        assert_eq!(
            connives,
            vec![creature],
            "the creature connives exactly once"
        );
        assert_eq!(
            game.object(creature)
                .and_then(|obj| obj
                    .counters
                    .get(&crate::object::CounterType::PlusOnePlusOne))
                .copied()
                .unwrap_or(0),
            1,
            "a nonland was discarded, so the conniving creature gets the counter"
        );
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

    #[test]
    fn connive_and_explore_follow_team_member_order_and_repeat_instructions() {
        let [a, b, c, d] = [0, 1, 2, 3].map(PlayerId::from_index);
        for kind in [KeywordActionKind::Connive, KeywordActionKind::Explore] {
            let mut game = GameState::new(["A", "B", "C", "D"].map(String::from).into(), 20);
            game.turn.active_player = c;
            game.set_teams(vec![vec![a, b], vec![c, d]]).unwrap();
            game.enable_shared_team_turns().unwrap();
            game.set_shared_team_member_order(0, vec![b, a]).unwrap();
            game.set_shared_team_member_order(1, vec![d, c]).unwrap();
            let [ca, cb, cc, cd, second_c] = [a, b, c, d, c].map(|p| create_creature(&mut game, p));
            let source = game.new_object_id();
            let mut dm = ConniveDecisionMaker {
                object_choices: VecDeque::from([vec![second_c]]),
            };
            let mut ctx = ExecutionContext::new(source, a, &mut dm);
            ctx.set_tagged_objects("batch", [ca, cc, cb, second_c, cd, ca].map(|id| {
                ObjectSnapshot::from_object(game.object(id).unwrap(), &game)
            }).into());
            let target = ChooseSpec::tagged("batch");
            let result = match kind {
                KeywordActionKind::Connive => ConniveEffect::new(target).execute(&mut game, &mut ctx),
                _ => crate::effects::ExploreEffect::new(target).execute(&mut game, &mut ctx),
            }.unwrap();
            let events: Vec<_> = result.events.iter()
                .filter_map(|event| event.downcast::<KeywordActionEvent>())
                .filter(|event| event.action == kind)
                .map(|event| (event.player, event.source))
                .collect();
            assert_eq!(events, [(d, cd), (c, second_c), (c, cc), (b, cb), (a, ca), (a, ca)]);
        }
    }

}
