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
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            // CR 701.50e: zero is not a connive event, including for replacements.
            let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
            if count == 0 {
                return Ok(EffectOutcome::count(0));
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
                return Ok(EffectOutcome::target_invalid());
            }

            let mut remaining = target_ids
                .into_iter()
                .filter_map(|object_id| {
                    let snapshot = connive_snapshot_for_object(game, ctx, object_id)?;
                    // CR 701.50a-b: the instructed permanent connives even if it
                    // stopped being a creature. Target legality, where relevant,
                    // is handled by the target specification/resolution pipeline.
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

            let mut events = Vec::new();
            let mut counter_facts = Vec::new();
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

                    // CR 614: replacement effects such as Leader, Super-Genius's
                    // ("instead you draw a card, then that creature connives") see
                    // the would-connive event first. The replacement suppresses
                    // itself, so the connive it performs is not replaced again.
                    let would_event = crate::events::Event::new_with_provenance(
                        KeywordActionEvent::new(
                            KeywordActionKind::Connive,
                            controller,
                            target_id,
                            count as u32,
                        )
                        .with_snapshot(Some(instruction.snapshot.clone())),
                        ctx.provenance,
                    );
                    let replacement_result = crate::events::processing::process_trait_event_with_execution_context(game, would_event, ctx)?;
                    let iteration_outcome = crate::effects::replacement::execute_event_expansion_with_bindings(game, ctx, replacement_result, |game, ctx, original| {
                        let mut events = Vec::new();
                        let mut connived_objects = Vec::new();
                        let mut counter_facts = Vec::new();
                        match original {
                            crate::events::processing::TraitEventResult::Replaced { effects, source, controller, context, .. } => {
                                let snapshot = context.event.inner().snapshot().cloned();
                                let mut outcome = crate::effects::composition::mechanic_actions::execute_keyword_action_replacement_effects(game, ctx, effects, source, controller, &context, snapshot)?;
                                let objects = outcome.events.iter().filter_map(|event| event.downcast::<KeywordActionEvent>())
                                    .filter(|action| action.action == KeywordActionKind::Connive).map(|action| action.source).collect();
                                outcome.value = crate::effect::OutcomeValue::Objects(objects);
                                return Ok(outcome);
                            }
                            crate::events::processing::TraitEventResult::Prevented => return Ok(EffectOutcome::prevented()),
                            crate::events::processing::TraitEventResult::NeedsChoice { .. } | crate::events::processing::TraitEventResult::NeedsInteraction { .. } => {
                                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                                return Err(ExecutionError::InternalError("connive suspended without a captured decision".into()));
                            }
                            crate::events::processing::TraitEventResult::Proceed(_) | crate::events::processing::TraitEventResult::Modified(_) => {}
                            crate::events::processing::TraitEventResult::Expanded { .. } => return Err(ExecutionError::InternalError("connive commit received an unflattened result".into())),
                        }

                    // Rule 701.50e: connive N draws N, discards N, then counts nonlands discarded this way.
                    let draw_outcome =
                        DrawCardsEffect::new(count as i32, PlayerFilter::Specific(controller))
                            .execute(game, ctx)?;
                    events.extend(draw_outcome.events);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(
                            EffectOutcome::with_objects(connived_objects).with_events(events)
                        );
                    }

                    // Then discard that many cards if possible.
                    let hand_cards: Vec<ObjectId> = game
                        .player(controller)
                        .map(|p| p.hand.to_vec())
                        .unwrap_or_default();

                    let required = count.min(hand_cards.len());
                    if required > 0 {
                        use crate::decisions::make_decision;
                        use crate::decisions::specs::ChooseObjectsSpec;
                        use crate::events::processing::execute_discard_with_scope;

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
                        )
                        // Discarded hidden cards are opened publicly before the
                        // answer is replayed (Madness, discard triggers); see
                        // `game_state::hidden_hand_choices`.
                        .with_selection_reveal_policy(
                            crate::decisions::context::SelectionRevealPolicy::Public,
                        );
                        let chosen: Vec<_> = make_decision(
                            game,
                            ctx.decision_maker,
                            controller,
                            Some(ctx.source),
                            spec,
                        );
                        // The UI surfaces this prompt on a probe pass whose state it
                        // keeps on screen; committing a fallback discard here would
                        // show a card discarded (and a counter placed) before the
                        // player has chosen.
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(
                                EffectOutcome::with_objects(connived_objects).with_events(events)
                            );
                        }
                        let selected = normalize_object_selection(chosen, &hand_cards, required);
                        let mut discarded_nonlands: u32 = 0;
                        let mut successful_discards = Vec::new();
                        let mut receipts = Vec::new();
                        let cause = EventCause::from_effect(ctx.source, ctx.controller);
                        for card_to_discard in selected {
                            let snapshot = game.object(card_to_discard)
                                .map(|object| ObjectSnapshot::from_object(object, game));
                            let discarded_nonland = snapshot.as_ref()
                                .is_some_and(|snapshot| !snapshot.card_types.contains(&CardType::Land));
                            let receipt = execute_discard_with_scope(
                                game,
                                card_to_discard,
                                controller,
                                cause.clone(),
                                false,
                                ctx.provenance,
                                &mut *ctx.decision_maker,
                                &ctx.replacement,
                                ctx.source_snapshot.as_ref(),
                            )?;

                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(EffectOutcome::count(0));
                            }
                            let discard_result = &receipt.result;
                            if !discard_result.prevented && discard_result.new_id.is_some() {
                                if discarded_nonland {
                                    discarded_nonlands += 1;
                                }
                                let event = receipt.resolved_event.as_ref()
                                    .ok_or_else(|| ExecutionError::InternalError("connive discard lost its resolved event".into()))?;
                                if event.player != controller || event.card != card_to_discard || event.cause != cause {
                                    return Err(ExecutionError::InternalError("connive discard changed an unsupported batch identity".into()));
                                }
                                successful_discards.push((event.card, snapshot, discard_result.final_zone, discard_result.new_id));
                            }
                            receipts.push(receipt);
                        }
                        let discard_events = super::discard::completed_discard_events(
                            game, controller, cause, ctx.provenance, successful_discards,
                        );
                        let discard_outcome = super::discard::finish_discard_receipts(game, ctx,
                            EffectOutcome::count(i32::try_from(discarded_nonlands).map_err(|_| ExecutionError::InternalError("connive discard count overflow".into()))?).with_events(discard_events), receipts)?;
                        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                        events.extend(discard_outcome.events);
                        counter_facts.extend(discard_outcome.execution_facts);

                        if discarded_nonlands > 0 {
                            let event = crate::events::Event::put_counters(
                                target_id,
                                crate::object::CounterType::PlusOnePlusOne,
                                discarded_nonlands,
                                ctx.cause.clone(),
                            )
                            .with_provenance(ctx.provenance);
                            let placement =
                                crate::effects::counters::execute_object_counter_placement(
                                    game, ctx, event,
                                )?;
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(EffectOutcome::count(0));
                            }
                            events.extend(placement.events);
                            counter_facts.extend(placement.execution_facts);
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
                        Ok(EffectOutcome::with_objects(connived_objects).with_events(events)
                            .with_execution_facts(EffectOutcome::merge_execution_facts(counter_facts)))
                    }, |_, context, receipt| {
                        let captured = crate::events::downcast_event::<KeywordActionEvent>(context.event.inner())
                            .filter(|action| action.action == KeywordActionKind::Connive)
                            .ok_or_else(|| ExecutionError::InternalError("connive addition captured an incompatible event".into()))?;
                        let action = receipt.events.iter().rev().filter_map(|event| event.downcast::<KeywordActionEvent>())
                            .find(|action| action.action == KeywordActionKind::Connive && action.source == captured.source).unwrap_or(captured);
                        let mut tags = action.object_tags.clone();
                        if let Some(snapshot) = action.snapshot.as_ref().or(captured.snapshot.as_ref()) {
                            tags.insert(crate::tag::TagKey::from("it"), vec![snapshot.clone()]);
                            tags.insert(crate::tag::TagKey::from("__it__"), vec![snapshot.clone()]);
                        }
                        Ok(crate::effects::replacement::ReplacementProgramBindings { targets: None,
                            object_tags: tags.into_iter().map(|(name,snapshots)| (name.as_str().to_owned(),snapshots)).collect(),
                        })
                    })?;
                    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                    if let Some(objects) = iteration_outcome.objects() { connived_objects.extend_from_slice(objects); }
                    events.extend(iteration_outcome.events);
                    counter_facts.extend(iteration_outcome.execution_facts);

                }
            }

            Ok(EffectOutcome::with_objects(connived_objects)
                .with_events(events)
                .with_execution_facts(EffectOutcome::merge_execution_facts(counter_facts)))
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
        }
        result
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
}
