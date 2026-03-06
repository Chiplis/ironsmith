//! Discard effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{normalize_object_selection, resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilter;
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::types::CardType;

/// Effect that causes a player to discard cards.
///
/// Can optionally discard at random.
///
/// # Fields
///
/// * `count` - Number of cards to discard
/// * `player` - The player who discards
/// * `random` - Whether to discard at random
///
/// # Example
///
/// ```ignore
/// // Discard a card
/// let effect = DiscardEffect::you(1);
///
/// // Discard two cards at random
/// let effect = DiscardEffect::random(2, PlayerFilter::You);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DiscardEffect {
    /// Number of cards to discard.
    pub count: Value,
    /// The player who discards.
    pub player: PlayerFilter,
    /// Whether to discard at random.
    pub random: bool,
    /// Optional hand-card restriction for cards that can be discarded.
    pub card_filter: Option<ObjectFilter>,
}

impl DiscardEffect {
    /// Create a new discard effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter, random: bool) -> Self {
        Self::new_with_filter(count, player, random, None)
    }

    /// Create a new discard effect with an optional card filter.
    pub fn new_with_filter(
        count: impl Into<Value>,
        player: PlayerFilter,
        random: bool,
        card_filter: Option<ObjectFilter>,
    ) -> Self {
        Self {
            count: count.into(),
            player,
            random,
            card_filter,
        }
    }

    /// The controller discards N cards (player chooses).
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You, false)
    }

    /// The controller discards N cards at random.
    pub fn you_random(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You, true)
    }

    /// Target player discards N cards at random.
    pub fn random(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self::new(count, player, true)
    }

    /// Target opponent discards N cards.
    pub fn opponent(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::Opponent, false)
    }
}

fn card_type_name(card_type: CardType) -> &'static str {
    card_type.name()
}

fn format_discard_card_type_phrase(card_types: &[CardType]) -> String {
    if card_types.is_empty() {
        return "card".to_string();
    }
    if card_types.len() == 1 {
        return format!("{} card", card_type_name(card_types[0]));
    }

    let mut parts: Vec<&str> = card_types.iter().map(|ct| card_type_name(*ct)).collect();
    let last = parts.pop().expect("len checked");
    format!("{} or {} card", parts.join(", "), last)
}

impl EffectExecutor for DiscardEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::decisions::make_decision;
        use crate::decisions::specs::ChooseObjectsSpec;
        use crate::event_processor::execute_discard;
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        let mut discarded = 0;

        let mut hand_cards: Vec<_> = game
            .player(player_id)
            .map(|p| p.hand.iter().copied().collect())
            .unwrap_or_default();
        if let Some(filter) = &self.card_filter {
            let filter_ctx = ctx.filter_context(game);
            hand_cards.retain(|card_id| {
                game.object(*card_id)
                    .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
            });
        }

        let required = count.min(hand_cards.len());
        if required == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let cards_to_discard = if self.random {
            game.shuffle_slice(&mut hand_cards);
            hand_cards.into_iter().take(required).collect::<Vec<_>>()
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!(
                    "Choose {} card{} to discard",
                    required,
                    if required == 1 { "" } else { "s" }
                ),
                hand_cards.clone(),
                required,
                Some(required),
            );
            let chosen: Vec<_> =
                make_decision(game, ctx.decision_maker, player_id, Some(ctx.source), spec);
            normalize_object_selection(chosen, &hand_cards, required)
        };

        // Discard each card using the event system. The cause is inherited from
        // the execution context so discard-as-cost stays cost-caused.
        let cause = ctx.cause.clone();
        for card_id in cards_to_discard {
            let result = execute_discard(
                game,
                card_id,
                player_id,
                cause.clone(),
                false,
                ctx.provenance,
                &mut *ctx.decision_maker,
            );
            if !result.prevented {
                discarded += 1;
            }
        }

        Ok(EffectOutcome::count(discarded))
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;

        let required = match self.count {
            Value::Fixed(n) => n.max(0) as usize,
            _ => {
                return Err(CostValidationError::Other(
                    "dynamic discard cost amount is unsupported".to_string(),
                ));
            }
        };
        if required == 0 {
            return Ok(());
        }

        let player_id = match self.player {
            PlayerFilter::You => controller,
            PlayerFilter::Specific(id) => id,
            _ => {
                return Err(CostValidationError::Other(
                    "discard cost supports only 'you' or a specific player".to_string(),
                ));
            }
        };

        let mut hand_cards: Vec<_> = game
            .player(player_id)
            .map(|p| p.hand.iter().copied().collect())
            .unwrap_or_default();
        hand_cards.retain(|card_id| *card_id != source);

        if let Some(filter) = &self.card_filter {
            let filter_ctx = crate::filter::FilterContext::new(controller).with_source(source);
            hand_cards.retain(|card_id| {
                game.object(*card_id)
                    .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
            });
        }

        if hand_cards.len() < required {
            return Err(CostValidationError::NotEnoughCards);
        }

        Ok(())
    }

    fn cost_description(&self) -> Option<String> {
        let count = match self.count {
            Value::Fixed(n) if n > 0 => n as usize,
            _ => return None,
        };
        let card_types = self
            .card_filter
            .as_ref()
            .map(|f| f.card_types.clone())
            .unwrap_or_default();
        let type_phrase = format_discard_card_type_phrase(&card_types);
        let random_suffix = if self.random { " at random" } else { "" };
        Some(if count == 1 {
            format!("Discard a {type_phrase}{random_suffix}")
        } else {
            format!("Discard {count} {type_phrase}s{random_suffix}")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardBuilder};
    use crate::effect::EffectResult;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_spell_card(card_id: u32, name: &str) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Instant])
            .build()
    }

    fn add_card_to_hand(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_spell_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Hand);
        game.add_object(obj); // add_object automatically updates player.hand for Zone::Hand
        id
    }

    #[test]
    fn test_discard_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_card_to_hand(&mut game, "Card 1", alice);
        add_card_to_hand(&mut game, "Card 2", alice);
        add_card_to_hand(&mut game, "Card 3", alice);

        assert_eq!(game.player(alice).unwrap().hand.len(), 3);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    }

    #[test]
    fn test_discard_more_than_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_card_to_hand(&mut game, "Card 1", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only discarded 1 card (all that was in hand)
        assert_eq!(result.result, EffectResult::Count(1));
        assert!(game.player(alice).unwrap().hand.is_empty());
    }

    #[test]
    fn test_discard_empty_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_discard_variable_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_card_to_hand(&mut game, "Card 1", alice);
        add_card_to_hand(&mut game, "Card 2", alice);
        add_card_to_hand(&mut game, "Card 3", alice);
        add_card_to_hand(&mut game, "Card 4", alice);

        let mut ctx = ExecutionContext::new_default(source, alice).with_x(2);
        let effect = DiscardEffect::new(Value::X, PlayerFilter::You, false);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
    }

    #[test]
    fn test_discard_clone_box() {
        let effect = DiscardEffect::you(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("DiscardEffect"));
    }

    #[test]
    fn test_discard_can_execute_as_cost_requires_enough_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let effect = DiscardEffect::you_random(1);
        let can_pay = effect.can_execute_as_cost(&game, source, alice);
        assert_eq!(
            can_pay,
            Err(crate::effects::CostValidationError::NotEnoughCards)
        );

        add_card_to_hand(&mut game, "Card 1", alice);
        let can_pay = effect.can_execute_as_cost(&game, source, alice);
        assert!(can_pay.is_ok(), "expected discard cost to be payable");
    }

    #[test]
    fn test_discard_cost_description_random() {
        let effect = DiscardEffect::you_random(1);
        assert_eq!(
            effect.cost_description().as_deref(),
            Some("Discard a card at random")
        );
    }
}
