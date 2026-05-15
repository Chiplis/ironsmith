//! DrawCards effect implementation.

use crate::decision::DecisionMaker;
use crate::decisions::context::{BooleanContext, ViewCardsContext};
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{
    TraitEventResult, process_trait_event_with_dm_and_applied_effects,
};
use crate::events::{CardRevealedEvent, CardsDrawnEvent, Event};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;
pub use ironsmith_core::DrawCardsEffect;

fn execute_draw_replacement_effects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: Vec<Effect>,
    effect_id: crate::replacement::ReplacementEffectId,
) -> Result<EffectOutcome, ExecutionError> {
    let replacement_effect = game
        .effect_store
        .replacement_effects
        .get_effect(effect_id)
        .cloned();
    let (replacement_source, replacement_controller) = replacement_effect
        .as_ref()
        .map(|effect| (effect.source, effect.controller))
        .unwrap_or((ctx.source, ctx.controller));
    let replacement_key = replacement_effect
        .as_ref()
        .map(|effect| effect.application_key());

    let original_source = ctx.source;
    let original_controller = ctx.controller;
    let original_cause = ctx.cause.clone();
    let was_suppressed = !ctx
        .replacement
        .suppressed_replacement_effects
        .insert(effect_id);
    let key_was_suppressed = if let Some(key) = replacement_key.as_ref() {
        !ctx.replacement
            .suppressed_replacement_effect_keys
            .insert(key.clone())
    } else {
        true
    };

    ctx.source = replacement_source;
    ctx.controller = replacement_controller;
    ctx.cause =
        crate::events::cause::EventCause::from_effect(replacement_source, replacement_controller);

    let execution_result = (|| -> Result<EffectOutcome, ExecutionError> {
        let mut outcomes = Vec::new();
        for effect in effects {
            outcomes.push(crate::effects::execute_effect(game, &effect, ctx)?);
        }
        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    })();

    ctx.source = original_source;
    ctx.controller = original_controller;
    ctx.cause = original_cause;
    if !was_suppressed {
        ctx.replacement
            .suppressed_replacement_effects
            .remove(&effect_id);
    }
    if !key_was_suppressed && let Some(key) = replacement_key {
        ctx.replacement
            .suppressed_replacement_effect_keys
            .remove(&key);
    }

    execution_result
}

#[derive(Debug, Clone)]
pub(crate) struct AutomaticDrawRevealCandidate {
    pub source_id: ObjectId,
    pub source_name: String,
    pub player_id: PlayerId,
    pub card_id: ObjectId,
    pub zone: Zone,
    pub optional: bool,
    pub snapshot: Option<ObjectSnapshot>,
}

pub(crate) fn automatic_draw_reveal_boolean_context(
    candidate: &AutomaticDrawRevealCandidate,
) -> BooleanContext {
    BooleanContext::new(
        candidate.player_id,
        Some(candidate.source_id),
        "reveal the first card you draw",
    )
    .with_source_name(candidate.source_name.clone())
}

pub(crate) fn collect_automatic_draw_reveal_candidates(
    game: &GameState,
    player_id: PlayerId,
    drawn: &[ObjectId],
    draws_before: u32,
) -> Vec<AutomaticDrawRevealCandidate> {
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    let mut candidates = Vec::new();
    let draws_after = draws_before + drawn.len() as u32;

    for &source_id in &game.battlefield {
        let Some(source_obj) = game.object(source_id) else {
            continue;
        };
        if game.controller_of(source_obj) != player_id {
            continue;
        }
        let Some(static_abilities) = view.static_abilities_rc(source_id) else {
            continue;
        };
        for static_ability in static_abilities.iter() {
            let Some(spec) = static_ability.reveal_drawn_card_spec() else {
                continue;
            };
            if spec.your_turns_only && game.turn.active_player != player_id {
                continue;
            }
            let draw_number = spec.card_number;
            if draw_number == 0 || draws_before >= draw_number || draw_number > draws_after {
                continue;
            }

            let drawn_index = (draw_number - draws_before - 1) as usize;
            let Some(&card_id) = drawn.get(drawn_index) else {
                continue;
            };

            let snapshot = game
                .object(card_id)
                .map(|obj| ObjectSnapshot::from_object(obj, game));
            candidates.push(AutomaticDrawRevealCandidate {
                source_id,
                source_name: source_obj.name.clone(),
                player_id,
                card_id,
                zone: Zone::Hand,
                optional: spec.optional,
                snapshot,
            });
        }
    }

    candidates
}

pub(crate) fn emit_automatic_draw_reveal_event(
    game: &GameState,
    decision_maker: &mut (impl DecisionMaker + ?Sized),
    candidate: &AutomaticDrawRevealCandidate,
    provenance: ProvNodeId,
) -> TriggerEvent {
    for viewer_idx in 0..game.players.len() {
        let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
        let view_ctx = ViewCardsContext::new(
            viewer,
            candidate.player_id,
            Some(candidate.source_id),
            candidate.zone,
            "Reveal drawn card",
        )
        .with_public(true);
        decision_maker.view_cards(game, viewer, &[candidate.card_id], &view_ctx);
    }

    TriggerEvent::new_with_provenance(
        CardRevealedEvent::new(
            candidate.player_id,
            candidate.card_id,
            candidate.zone,
            Some(candidate.source_id),
            candidate.snapshot.clone(),
        ),
        provenance,
    )
}

pub(crate) fn automatic_reveal_events_for_draw(
    game: &GameState,
    player_id: PlayerId,
    drawn: &[ObjectId],
    draws_before: u32,
    decision_maker: &mut (impl DecisionMaker + ?Sized),
    provenance: ProvNodeId,
) -> Vec<TriggerEvent> {
    let mut reveal_events = Vec::new();

    for candidate in collect_automatic_draw_reveal_candidates(game, player_id, drawn, draws_before)
    {
        if candidate.optional
            && !decision_maker
                .decide_boolean(game, &automatic_draw_reveal_boolean_context(&candidate))
        {
            continue;
        }

        reveal_events.push(emit_automatic_draw_reveal_event(
            game,
            decision_maker,
            &candidate,
            provenance,
        ));
    }

    reveal_events
}

/// Effect that causes a player to draw cards.
///
/// Handles replacement effects, "can't draw extra cards" restrictions,
/// and tracks cards drawn this turn for triggered abilities.
///
/// # Fields
///
/// * `count` - Number of cards to draw
/// * `player` - Which player draws (defaults to controller)
///
/// # Example
///
/// ```ignore
/// // Draw 2 cards (you draw)
/// let effect = DrawCardsEffect::you(2);
///
/// // Opponent draws 3 cards
/// let effect = DrawCardsEffect::new(3, PlayerFilter::Opponent);
///
/// // Specific player draws 2 cards
/// let effect = DrawCardsEffect::new(2, PlayerFilter::Specific(player_id));
/// ```
impl EffectExecutor for DrawCardsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let requested_count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        // Check for "can't draw extra cards" restriction (e.g., Narset)
        let count = if !game.can_draw_extra_cards(player_id) {
            let current_draws = game
                .turn_store
                .turn_history
                .cards_drawn_by_player(player_id);
            // Player can only draw their first card of the turn
            if current_draws >= 1 {
                // Already drew this turn, can't draw any more
                return Ok(EffectOutcome::prevented());
            }
            // First draw - can only draw 1, not more
            requested_count.min(1)
        } else {
            requested_count
        };

        let mut total_drawn = 0;
        let mut replacement_count = 0;
        let mut events = Vec::new();
        let mut direct_drawn = Vec::new();
        let mut direct_draw_is_first = false;
        let mut direct_draw_step_context = (false, 0);
        let mut direct_draws_before = 0;

        for _ in 0..count {
            if !game.can_draw(player_id) {
                continue;
            }

            let current_draws = game
                .turn_store
                .turn_history
                .cards_drawn_by_player(player_id)
                .saturating_add(direct_drawn.len() as u32);
            let is_first = current_draws == 0;
            let (is_during_players_draw_step, cards_previously_drawn_this_draw_step) =
                game.draw_step_context_for_player(player_id);
            let applied_effects = ctx.replacement.suppressed_replacement_effects.clone();
            let applied_effect_keys = ctx.replacement.suppressed_replacement_effect_keys.clone();
            if applied_effects.is_empty() && applied_effect_keys.is_empty() {
                game.update_replacement_effects();
            }

            let draw_event = Event::draw(player_id, 1, is_first);
            match process_trait_event_with_dm_and_applied_effects(
                game,
                draw_event,
                ctx.decision_maker,
                &applied_effects,
                &applied_effect_keys,
            ) {
                TraitEventResult::Prevented => continue,
                TraitEventResult::Replaced { effects, effect_id } => {
                    let replacement_outcome =
                        execute_draw_replacement_effects(game, ctx, effects, effect_id)?;
                    replacement_count += replacement_outcome.count_or_zero();
                    events.extend(replacement_outcome.events);
                    continue;
                }
                TraitEventResult::NeedsChoice { .. }
                | TraitEventResult::NeedsInteraction { .. } => {
                    return Ok(
                        EffectOutcome::count(total_drawn + replacement_count).with_events(events)
                    );
                }
                TraitEventResult::Proceed(e) | TraitEventResult::Modified(e) => {
                    let final_count =
                        crate::events::downcast_event::<crate::events::DrawEvent>(e.inner())
                            .map(|draw| draw.count)
                            .unwrap_or(1);

                    let drawn = game.draw_cards_with_dm(
                        player_id,
                        final_count as usize,
                        &mut *ctx.decision_maker,
                    );

                    // Only emit event if cards were actually drawn
                    if drawn.is_empty() {
                        continue;
                    }
                    let drawn_len = drawn.len() as i32;
                    if direct_drawn.is_empty() {
                        direct_draw_is_first = is_first;
                        direct_draw_step_context = (
                            is_during_players_draw_step,
                            cards_previously_drawn_this_draw_step,
                        );
                        direct_draws_before = current_draws;
                    }
                    total_drawn += drawn_len;
                    direct_drawn.extend(drawn);
                }
            }
        }

        if !direct_drawn.is_empty() {
            let event = TriggerEvent::new_with_provenance(
                CardsDrawnEvent::new_with_step_context(
                    player_id,
                    direct_drawn,
                    direct_draw_is_first,
                    direct_draw_step_context.0,
                    direct_draw_step_context.1,
                ),
                ctx.provenance,
            );
            let drawn_count = event
                .downcast::<CardsDrawnEvent>()
                .map(CardsDrawnEvent::amount)
                .unwrap_or(0);
            game.record_cards_drawn_in_current_draw_step(player_id, drawn_count);
            let reveal_events = automatic_reveal_events_for_draw(
                game,
                player_id,
                event
                    .downcast::<CardsDrawnEvent>()
                    .map(|drawn_event| drawn_event.cards.as_slice())
                    .unwrap_or(&[]),
                direct_draws_before,
                &mut *ctx.decision_maker,
                ctx.provenance,
            );

            events.push(event);
            events.extend(reveal_events);
        }

        Ok(EffectOutcome::count(total_drawn + replacement_count).with_events(events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_cards_to_library(game: &mut GameState, owner: PlayerId, count: usize) {
        for i in 1..=count {
            let card = CardBuilder::new(CardId::new(), &format!("Library Card {}", i))
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_card(&card, owner, Zone::Library);
        }
    }

    fn add_static_source(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        ability: crate::static_abilities::StaticAbility,
    ) -> ObjectId {
        let source_card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&source_card, owner, Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .abilities
            .push(crate::ability::Ability::static_ability(ability));
        source
    }

    struct ChooseReplacementNamed(&'static str);

    impl crate::decision::DecisionMaker for ChooseReplacementNamed {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx
                .description
                .eq_ignore_ascii_case("Choose which replacement effect to apply")
                && let Some(option) = ctx
                    .options
                    .iter()
                    .find(|option| option.description.contains(self.0))
            {
                return vec![option.index];
            }

            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    #[test]
    fn test_draw_cards_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        assert_eq!(game.player(alice).unwrap().library.len(), 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 3);
    }

    #[test]
    fn test_draw_cards_tracks_drawn_this_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // First draw
        let effect = DrawCardsEffect::you(2);
        crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .unwrap();

        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 2);

        // Second draw
        let effect = DrawCardsEffect::you(1);
        crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .unwrap();

        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 3);
    }

    #[test]
    fn test_draw_cards_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // No cards in library
        assert_eq!(game.player(alice).unwrap().library.len(), 0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Can't draw from empty library
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
    }

    #[test]
    fn test_draw_cards_partial_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 2);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only draw what's available
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 0);
    }

    #[test]
    fn test_draw_cards_respects_double_draw_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        let source = game.new_object_id();
        game.effect_store.replacement_effects.add_resolution_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Modify(
                    crate::replacement::EventModification::Multiply(2),
                ),
            ),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 3);
    }

    #[test]
    fn test_draw_cards_executes_instead_draw_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 3);
        let source = add_static_source(
            &mut game,
            alice,
            "Draw Replacer",
            crate::static_abilities::StaticAbility::draw_replacement_exile_top_face_down(),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
        assert_eq!(game.player(alice).unwrap().library.len(), 1);
        assert_eq!(game.exile.len(), 2);
    }

    #[test]
    fn test_draw_replacement_double_executes_nested_draws() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        let source = add_static_source(
            &mut game,
            alice,
            "Thought Reflection",
            crate::static_abilities::StaticAbility::draw_replacement_double(),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 3);
    }

    #[test]
    fn test_draw_replacement_double_allows_other_replacements_on_nested_draws() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        let asmodeus = add_static_source(
            &mut game,
            alice,
            "Asmodeus the Archfiend",
            crate::static_abilities::StaticAbility::draw_replacement_exile_top_face_down(),
        );
        add_static_source(
            &mut game,
            alice,
            "Thought Reflection",
            crate::static_abilities::StaticAbility::draw_replacement_double(),
        );
        let mut dm = ChooseReplacementNamed("Thought Reflection");
        let mut ctx = ExecutionContext::new(asmodeus, alice, &mut dm);

        let effect = DrawCardsEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
        assert_eq!(game.player(alice).unwrap().library.len(), 3);
        assert_eq!(game.exile.len(), 2);
    }

    #[test]
    fn test_draw_replacement_instead_can_preempt_double_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);
        let asmodeus = add_static_source(
            &mut game,
            alice,
            "Asmodeus the Archfiend",
            crate::static_abilities::StaticAbility::draw_replacement_exile_top_face_down(),
        );
        add_static_source(
            &mut game,
            alice,
            "Thought Reflection",
            crate::static_abilities::StaticAbility::draw_replacement_double(),
        );
        let mut dm = ChooseReplacementNamed("Asmodeus the Archfiend");
        let mut ctx = ExecutionContext::new(asmodeus, alice, &mut dm);

        let effect = DrawCardsEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
        assert_eq!(game.player(alice).unwrap().library.len(), 4);
        assert_eq!(game.exile.len(), 1);
    }

    #[test]
    fn test_draw_cards_executes_static_instead_replacement_for_each_requested_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 3);
        let source = add_static_source(
            &mut game,
            alice,
            "Asmodeus the Archfiend",
            crate::static_abilities::StaticAbility::draw_replacement_exile_top_face_down(),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
        assert_eq!(game.player(alice).unwrap().library.len(), 1);
        assert_eq!(game.exile.len(), 2);
    }

    #[test]
    fn test_draw_cards_for_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        add_cards_to_library(&mut game, bob, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Alice makes Bob draw
        let effect = DrawCardsEffect::new(2, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(bob).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
    }

    #[test]
    fn test_draw_cards_variable_count() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 10);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_x(3);

        let effect = DrawCardsEffect::new(Value::X, PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
        assert_eq!(game.player(alice).unwrap().hand.len(), 3);
    }

    #[test]
    fn test_draw_cards_clone_box() {
        let effect = DrawCardsEffect::you(2);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("DrawCardsEffect"));
    }

    #[test]
    fn test_draw_cards_returns_events() {
        use crate::events::EventKind;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DrawCardsEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should have 1 CardsDrawnEvent containing all 3 cards
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind(), EventKind::CardsDrawn);

        let event = result.events[0].downcast::<CardsDrawnEvent>().unwrap();
        assert_eq!(event.cards.len(), 3);
        assert!(event.is_first_this_turn);
    }

    #[test]
    fn test_draw_cards_first_draw_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        add_cards_to_library(&mut game, alice, 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // First draw of turn
        let effect = DrawCardsEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let event = result.events[0].downcast::<CardsDrawnEvent>().unwrap();
        assert!(event.is_first_this_turn);
        assert_eq!(event.cards.len(), 2);
        game.stage_turn_history_event(&result.events[0]);

        // Second draw of turn
        let effect2 = DrawCardsEffect::you(1);
        let result2 = effect2.execute(&mut game, &mut ctx).unwrap();

        let event2 = result2.events[0].downcast::<CardsDrawnEvent>().unwrap();
        assert!(!event2.is_first_this_turn); // Not first draw anymore
    }
}
