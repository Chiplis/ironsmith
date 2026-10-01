//! Imprint effect implementation.
//!
//! Imprint exiles a card from a zone (typically hand) and associates it with
//! the source permanent. Used by Chrome Mox, Isochron Scepter, etc.

use crate::effects::zones::apply_zone_change_with_context_and_additional_effects;
use crate::decisions::context::SelectionRevealPolicy;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::decisions::{MayChooseCardSpec, make_decision};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::ObjectId;
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
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
        let controller = ctx.controller;
        let source_id = ctx.source;

        // Find valid cards in hand that match the filter
        // Imprint filters can depend on resolution context, most notably the
        // announced X in Panoptic Mirror's "mana value X" restriction.  A
        // bare game filter context silently evaluates those dynamic values
        // without X and makes every positive-X card ineligible.
        let filter_ctx = ctx.filter_context(game);
        let hand = game
            .player(controller)
            .map(|p| p.hand.clone())
            .unwrap_or_default();

        // In peer matches a hidden hand card's filter result is known only
        // to its owner: keep this peer's placeholders choosable and always ask,
        // so every peer replays the owner's answer (see
        // `game_state::hidden_hand_choices`).
        let hidden_hand_choice =
            game.hand_choice_depends_on_hidden_identity(&self.filter, hand.iter().copied());
        let placeholders = if hidden_hand_choice {
            game.hidden_hand_placeholder_candidates(&self.filter, &filter_ctx, hand.iter().copied())
        } else {
            Vec::new()
        };
        let valid_cards: Vec<_> = hand
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| {
                placeholders.contains(&obj.id) || self.filter.matches(obj, &filter_ctx, game)
            })
            .map(|obj| obj.id)
            .collect();

        if valid_cards.is_empty() && !hidden_hand_choice {
            // No valid cards to imprint
            return Ok(EffectOutcome::count(0));
        }

        // Ask the player if they want to imprint (this is optional - "you may")
        let chosen_card = if hidden_hand_choice
            || valid_cards
                .iter()
                .any(|id| game.hidden_identity_is_private(*id))
        {
            // The exiled card becomes public: the owner opens it on every
            // peer before the answer is replayed.
            let spec = ChooseObjectsSpec::new(
                source_id,
                "choose a card to exile and imprint",
                valid_cards.clone(),
                0,
                Some(1),
            )
            .require_explicit_choice()
            .with_selection_reveal_policy(SelectionRevealPolicy::Public);
            let chosen: Vec<ObjectId> = make_decision(
                game,
                &mut ctx.decision_maker,
                controller,
                Some(source_id),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let chosen = chosen.into_iter().find(|id| valid_cards.contains(id));
            game.record_hidden_identity_obligations(
                chosen.as_slice(),
                &self.filter,
                &filter_ctx,
                "imprint a card matching the filter",
            );
            if let Some(id) = chosen {
                game.mark_hidden_cards_publicly_revealed(&[id]);
            }
            chosen
        } else {
            let spec = MayChooseCardSpec::new(
                source_id,
                "choose a card to exile and imprint",
                valid_cards.clone(),
            );
            make_decision(
                game,
                &mut ctx.decision_maker,
                controller,
                Some(source_id),
                spec,
            )
        };

        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }

        // Verify the card is still valid
        let chosen_card = chosen_card.filter(|card_id| valid_cards.contains(card_id));

        if let Some(card_id) = chosen_card {
            // Exile the card (move_object returns the new ID in exile)
            // CR 614.1: the exile is an ordinary zone change, so replacement
            // effects apply to it.
            let from_zone = game.object(card_id).map(|object| object.zone);
            let additional_effects = ctx.additional_replacement_effects_snapshot();
            let mut receipts = Vec::new();
            let exiled_id = if let Some(from_zone) = from_zone {
                let receipt = crate::effects::zones::apply_zone_change_with_context_and_additional_effects(
    game,
    card_id,
    from_zone,
    Zone::Exile,
    ctx.cause.clone(),
    ctx,
    &additional_effects
)?;
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                let exiled_id = match &receipt.original {
                    crate::events::processing::EventOutcome::Proceed(change)
                        if change.final_zone == Zone::Exile =>
                    {
                        change.new_object_id
                    }
                    _ => None,
                };
                receipts.push((card_id, receipt));
                exiled_id
            } else { None };

            let original_outcome = if let Some(exiled_id) = exiled_id {
                game.imprint_card(source_id, exiled_id);
                game.add_exiled_with_source_link(source_id, exiled_id);
                EffectOutcome::with_objects(vec![exiled_id])
            } else {
                EffectOutcome::count(0)
            };
            crate::effects::zones::finish_zone_change_receipts(game, ctx, original_outcome, receipts)
        } else {
            // Player chose not to imprint
            Ok(EffectOutcome::count(0))
        }
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if ctx.decision_maker.awaiting_choice() { return result.map(|_| EffectOutcome::count(0)); }
        result
    }
}
