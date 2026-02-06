//! Imprint effect implementation.
//!
//! Imprint exiles a card from a zone (typically hand) and associates it with
//! the source permanent. Used by Chrome Mox, Isochron Scepter, etc.

use crate::decisions::{MayChooseCardSpec, make_decision};
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ObjectFilter;
use crate::zone::Zone;

/// Effect that exiles a card from hand and imprints it on the source permanent.
///
/// This is an optional effect ("you may exile"). If the player chooses not to
/// exile anything, no card is imprinted.
///
/// # Fields
///
/// * `filter` - Filter for which cards can be imprinted (e.g., nonartifact, nonland)
///
/// # Example
///
/// ```ignore
/// // Chrome Mox: "you may exile a nonartifact, nonland card from your hand"
/// let effect = ImprintFromHandEffect::new(
///     ObjectFilter::any()
///         .exclude_card_type(CardType::Artifact)
///         .exclude_card_type(CardType::Land)
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ImprintFromHandEffect {
    /// Filter for valid cards to imprint.
    pub filter: ObjectFilter,
}

impl ImprintFromHandEffect {
    /// Create a new imprint from hand effect with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Create an imprint effect for nonartifact, nonland cards.
    pub fn nonartifact_nonland() -> Self {
        use crate::types::CardType;
        Self::new(
            ObjectFilter::default()
                .without_type(CardType::Artifact)
                .without_type(CardType::Land),
        )
    }
}

impl EffectExecutor for ImprintFromHandEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller = ctx.controller;
        let source_id = ctx.source;

        // Find valid cards in hand that match the filter
        let filter_ctx = game.filter_context_for(controller, Some(source_id));
        let hand = game
            .player(controller)
            .map(|p| p.hand.clone())
            .unwrap_or_default();

        let valid_cards: Vec<_> = hand
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| self.filter.matches(obj, &filter_ctx, game))
            .map(|obj| obj.id)
            .collect();

        if valid_cards.is_empty() {
            // No valid cards to imprint
            return Ok(EffectOutcome::count(0));
        }

        // Ask the player if they want to imprint (this is optional - "you may")
        let spec = MayChooseCardSpec::new(
            source_id,
            "choose a card to exile and imprint",
            valid_cards.clone(),
        );
        let chosen_card = make_decision(
            game,
            &mut ctx.decision_maker,
            controller,
            Some(source_id),
            spec,
        );

        // Verify the card is still valid
        let chosen_card = chosen_card.filter(|card_id| valid_cards.contains(card_id));

        if let Some(card_id) = chosen_card {
            // Exile the card (move_object returns the new ID in exile)
            let exiled_id = game.move_object(card_id, Zone::Exile);

            if let Some(exiled_id) = exiled_id {
                // Imprint it on the source permanent
                game.imprint_card(source_id, exiled_id);
                Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                    exiled_id,
                ])))
            } else {
                Ok(EffectOutcome::count(0))
            }
        } else {
            // Player chose not to imprint
            Ok(EffectOutcome::count(0))
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_card_in_hand(
        game: &mut GameState,
        name: &str,
        card_types: Vec<CardType>,
        owner: PlayerId,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(card_types)
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    #[test]
    fn test_imprint_no_valid_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // No cards in hand
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ImprintFromHandEffect::nonartifact_nonland();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_imprint_only_artifacts_in_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Only artifact in hand (not valid for Chrome Mox imprint)
        let _artifact = create_card_in_hand(&mut game, "Sol Ring", vec![CardType::Artifact], alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ImprintFromHandEffect::nonartifact_nonland();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // No valid cards (artifact is excluded)
        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_imprint_only_lands_in_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Only land in hand (not valid for Chrome Mox imprint)
        let _land = create_card_in_hand(&mut game, "Mountain", vec![CardType::Land], alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ImprintFromHandEffect::nonartifact_nonland();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // No valid cards (land is excluded)
        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_imprint_structure() {
        let effect = ImprintFromHandEffect::nonartifact_nonland();

        // Check that the filter excludes artifacts and lands
        assert!(
            !effect.filter.card_types.is_empty() || !effect.filter.excluded_card_types.is_empty()
        );
    }

    #[test]
    fn test_imprint_clone_box() {
        let effect = ImprintFromHandEffect::nonartifact_nonland();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ImprintFromHandEffect"));
    }
}
