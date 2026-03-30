//! DrawCards effect implementation.

use crate::decision::DecisionMaker;
use crate::decisions::context::{BooleanContext, ViewCardsContext};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::events::{CardRevealedEvent, CardsDrawnEvent};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::snapshot::ObjectSnapshot;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

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
        if source_obj.controller != player_id {
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

    for candidate in collect_automatic_draw_reveal_candidates(game, player_id, drawn, draws_before) {
        if candidate.optional
            && !decision_maker.decide_boolean(game, &automatic_draw_reveal_boolean_context(&candidate))
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
#[derive(Debug, Clone, PartialEq)]
pub struct DrawCardsEffect {
    /// Number of cards to draw.
    pub count: Value,
    /// Which player draws.
    pub player: PlayerFilter,
}

impl DrawCardsEffect {
    /// Create a new DrawCards effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create a "draw N cards" effect for the controller.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for DrawCardsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::event_processor::{EventOutcome, process_draw};

        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        // Check if this is the first draw this turn
        let current_draws = game.turn_history.cards_drawn_by_player(player_id);
        let is_first = current_draws == 0;

        // Check for "can't draw extra cards" restriction (e.g., Narset)
        let count = if !game.can_draw_extra_cards(player_id) {
            // Player can only draw their first card of the turn
            if current_draws >= 1 {
                // Already drew this turn, can't draw any more
                return Ok(EffectOutcome::prevented());
            }
            // First draw - can only draw 1, not more
            count.min(1)
        } else {
            count
        };

        // Process through replacement effects with decision maker
        match process_draw(game, player_id, count, is_first, &mut *ctx.decision_maker) {
            EventOutcome::Prevented => Ok(EffectOutcome::prevented()),
            EventOutcome::Proceed(final_count) => {
                let drawn = game.draw_cards_with_dm(
                    player_id,
                    final_count as usize,
                    &mut *ctx.decision_maker,
                );
                let count = drawn.len() as i32;

                // Only emit event if cards were actually drawn
                if drawn.is_empty() {
                    return Ok(EffectOutcome::count(0));
                }

                let event = TriggerEvent::new_with_provenance(
                    CardsDrawnEvent::new(player_id, drawn, is_first),
                    ctx.provenance,
                );
                let reveal_events = automatic_reveal_events_for_draw(
                    game,
                    player_id,
                    event
                        .downcast::<CardsDrawnEvent>()
                        .map(|drawn_event| drawn_event.cards.as_slice())
                        .unwrap_or(&[]),
                    current_draws,
                    &mut *ctx.decision_maker,
                    ctx.provenance,
                );

                let mut outcome = EffectOutcome::count(count).with_event(event);
                for reveal_event in reveal_events {
                    outcome = outcome.with_event(reveal_event);
                }
                Ok(outcome)
            }
            EventOutcome::Replaced => {
                // Replacement effects already executed by process_draw
                Ok(EffectOutcome::replaced())
            }
            EventOutcome::NotApplicable => {
                // Player can't draw (no library, etc.)
                Ok(EffectOutcome::prevented())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
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
        crate::executor::execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .unwrap();

        assert_eq!(game.turn_history.cards_drawn_by_player(alice), 2);

        // Second draw
        let effect = DrawCardsEffect::you(1);
        crate::executor::execute_effect(&mut game, &crate::effect::Effect::new(effect), &mut ctx)
            .unwrap();

        assert_eq!(game.turn_history.cards_drawn_by_player(alice), 3);
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
