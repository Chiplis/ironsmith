use std::collections::HashMap;

use crate::cards::CardDefinition;
use crate::ids::PlayerId;
use crate::types::{CardType, Supertype};

use super::{ConspiracyDraftState, DraftCardView, DraftSelection};

/// Booster product identity needed by the CR 903.13e-f construction exceptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommanderDraftProduct {
    CommanderLegends,
    CommanderMasters,
    BattleForBaldursGate,
    Other,
}

/// One unopened Commander Draft booster and its rules-relevant product identity.
#[derive(Debug, Clone)]
pub struct CommanderDraftBooster {
    pub product: CommanderDraftProduct,
    pub cards: Vec<CardDefinition>,
}

/// CR 903.13 draft state layered on the reusable private-pack draft engine.
#[derive(Debug, Clone)]
pub struct CommanderDraftState {
    draft: ConspiracyDraftState,
    products: Vec<CommanderDraftProduct>,
}

impl CommanderDraftState {
    /// Create the usual three-round, two-card-pick Commander Draft.
    pub fn new(
        players: Vec<PlayerId>,
        packs: Vec<(PlayerId, Vec<CommanderDraftBooster>)>,
    ) -> Result<Self, String> {
        if players.len() < 3 {
            return Err(
                "Commander Draft requires a multiplayer game of at least three players".to_string(),
            );
        }
        let mut products = packs
            .iter()
            .flat_map(|(_, boosters)| boosters.iter().map(|booster| booster.product))
            .collect::<Vec<_>>();
        products.sort_unstable();
        products.dedup();
        let packs = packs
            .into_iter()
            .map(|(player, boosters)| {
                (
                    player,
                    boosters.into_iter().map(|booster| booster.cards).collect(),
                )
            })
            .collect();
        Ok(Self {
            draft: ConspiracyDraftState::new_with_pick_count(players, packs, 2)?,
            products,
        })
    }

    pub fn products(&self) -> &[CommanderDraftProduct] {
        &self.products
    }

    pub fn round(&self) -> u8 {
        self.draft.round()
    }

    pub fn is_complete(&self) -> bool {
        self.draft.is_complete()
    }

    pub fn draft_step(&mut self, selections: Vec<DraftSelection>) -> Result<(), String> {
        self.draft.draft_step(selections)
    }

    pub fn current_pack_view(&self, viewer: PlayerId, holder: PlayerId) -> Vec<DraftCardView> {
        self.draft.current_pack_view(viewer, holder)
    }

    pub fn drafted_view(&self, viewer: PlayerId, owner: PlayerId) -> Vec<DraftCardView> {
        self.draft.drafted_view(viewer, owner)
    }

    pub fn card_pool(&self, player: PlayerId) -> Result<Vec<CardDefinition>, String> {
        self.draft.card_pool(player)
    }

    pub fn contains_product(&self, product: CommanderDraftProduct) -> bool {
        self.products.contains(&product)
    }

    /// Validate the limited-pool and size exceptions before the ordinary
    /// Commander eligibility, partner, and color-identity checks are applied.
    /// `main_deck` excludes the commander slots, matching match setup inputs.
    pub fn validate_pool_and_size(
        &self,
        player: PlayerId,
        main_deck: &[CardDefinition],
        commanders: &[CardDefinition],
    ) -> Result<(), String> {
        Self::validate_completed_pool_and_size(
            &self.products,
            &self.card_pool(player)?,
            main_deck,
            commanders,
        )
    }

    /// Apply CR 903.13e-f to an explicit completed card pool, as used by the
    /// frontend handoff after a draft conducted on another trusted peer.
    pub fn validate_completed_pool_and_size(
        products: &[CommanderDraftProduct],
        card_pool: &[CardDefinition],
        main_deck: &[CardDefinition],
        commanders: &[CardDefinition],
    ) -> Result<(), String> {
        if !(commanders.len() == 1 || commanders.len() == 2) {
            return Err(
                "Commander Draft requires exactly one or two commanders per player".to_string(),
            );
        }
        if main_deck.len() + commanders.len() < 60 {
            return Err(
                "a Commander Draft deck must contain at least 60 cards including commanders"
                    .to_string(),
            );
        }

        let mut available = HashMap::<String, usize>::new();
        for definition in card_pool {
            *available
                .entry(definition.name().trim().to_ascii_lowercase())
                .or_default() += 1;
        }
        for definition in main_deck {
            let card = &definition.card;
            if card.has_supertype(Supertype::Basic) && card.has_card_type(CardType::Land) {
                continue;
            }
            let name = definition.name().trim().to_ascii_lowercase();
            let Some(remaining) = available.get_mut(&name) else {
                return Err(format!(
                    "{} is not in that player's Commander Draft card pool",
                    definition.name()
                ));
            };
            if *remaining == 0 {
                return Err(format!(
                    "the deck contains more copies of {} than that player drafted",
                    definition.name()
                ));
            }
            *remaining -= 1;
        }

        let mut special_additions = 0usize;
        for definition in commanders {
            let name = definition.name().trim().to_ascii_lowercase();
            if let Some(remaining) = available.get_mut(&name)
                && *remaining > 0
            {
                *remaining -= 1;
                continue;
            }
            let special_allowed = match name.as_str() {
                "the prismatic piper" => {
                    products.contains(&CommanderDraftProduct::CommanderLegends)
                        || products.contains(&CommanderDraftProduct::CommanderMasters)
                }
                "faceless one" => products.contains(&CommanderDraftProduct::BattleForBaldursGate),
                _ => false,
            };
            if special_allowed && special_additions < 2 {
                special_additions += 1;
                continue;
            }
            return Err(format!(
                "{} is not available as a Commander Draft commander",
                definition.name()
            ));
        }
        Ok(())
    }
}
