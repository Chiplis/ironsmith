//! Companion designation and starting-deck validation (CR 103.2b, 702.139).

use crate::ability::AbilityKind;
use crate::cards::CardDefinition;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::Object;
use crate::static_abilities::{
    CompanionDeckCardFacts, CompanionDeckCondition, StaticAbilityId,
};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionDesignationError {
    PlayerNotFound,
    ObjectNotFound,
    AlreadyChosen,
    NotOwned,
    WrongZone,
    NoCompanionAbility,
    StartingDeckCardMissing,
    ConditionNotFulfilled,
}

impl std::fmt::Display for CompanionDesignationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlayerNotFound => f.write_str("companion player was not found"),
            Self::ObjectNotFound => f.write_str("companion object was not found"),
            Self::AlreadyChosen => f.write_str("a player may reveal no more than one companion"),
            Self::NotOwned => f.write_str("the player does not own that outside-game card"),
            Self::WrongZone => f.write_str("a companion must remain outside the game"),
            Self::NoCompanionAbility => f.write_str("the selected card has no companion ability"),
            Self::StartingDeckCardMissing => {
                f.write_str("the complete starting deck is not available for validation")
            }
            Self::ConditionNotFulfilled => {
                f.write_str("the starting deck does not fulfill the companion condition")
            }
        }
    }
}

impl std::error::Error for CompanionDesignationError {}

pub fn companion_definition_condition(
    definition: &CardDefinition,
) -> Option<&CompanionDeckCondition> {
    definition.abilities.iter().find_map(|ability| {
        let AbilityKind::Static(ability) = &ability.kind else {
            return None;
        };
        ability.companion_deck_condition()
    })
}

fn companion_object_condition(object: &Object) -> Option<&CompanionDeckCondition> {
    object.abilities.iter().find_map(|ability| {
        let AbilityKind::Static(ability) = &ability.kind else {
            return None;
        };
        ability.companion_deck_condition()
    })
}

pub fn companion_deck_facts_from_definition(
    definition: &CardDefinition,
) -> CompanionDeckCardFacts {
    CompanionDeckCardFacts {
        name: definition.card.name.clone(),
        mana_cost: definition.card.mana_cost.clone(),
        card_types: definition.card.card_types.clone(),
        subtypes: definition.card.subtypes.clone(),
        has_all_creature_types: definition.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Changeling
            )
        }),
        has_activated_ability: definition
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
    }
}

pub fn companion_deck_facts_from_object(object: &Object) -> CompanionDeckCardFacts {
    CompanionDeckCardFacts {
        name: object.name.to_owned_string(),
        mana_cost: object.mana_cost.as_ref().map(|cost| cost.to_owned_value()),
        card_types: object.card_types.to_vec(),
        subtypes: object.subtypes.to_vec(),
        has_all_creature_types: object.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Changeling
            )
        }),
        has_activated_ability: object
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
    }
}

pub fn validate_companion_definition(
    companion: &CardDefinition,
    starting_deck: &[CardDefinition],
    minimum_deck_size: usize,
) -> Result<(), CompanionDesignationError> {
    let condition = companion_definition_condition(companion)
        .ok_or(CompanionDesignationError::NoCompanionAbility)?;
    let facts = starting_deck
        .iter()
        .map(companion_deck_facts_from_definition)
        .collect::<Vec<_>>();
    condition
        .is_fulfilled_by(&facts, minimum_deck_size)
        .then_some(())
        .ok_or(CompanionDesignationError::ConditionNotFulfilled)
}

impl GameState {
    /// Reveal and record one owned outside-game companion after validating the
    /// complete CR 103.2 starting deck. This operation mutates only on success.
    pub fn designate_companion(
        &mut self,
        player: PlayerId,
        companion: ObjectId,
        starting_deck: &[ObjectId],
        minimum_deck_size: usize,
    ) -> Result<(), CompanionDesignationError> {
        let player_state = self
            .player(player)
            .ok_or(CompanionDesignationError::PlayerNotFound)?;
        if player_state.companion.is_some() {
            return Err(CompanionDesignationError::AlreadyChosen);
        }

        let companion_object = self
            .object(companion)
            .ok_or(CompanionDesignationError::ObjectNotFound)?;
        if companion_object.owner != player {
            return Err(CompanionDesignationError::NotOwned);
        }
        if companion_object.zone != Zone::OutsideGame {
            return Err(CompanionDesignationError::WrongZone);
        }
        let condition = companion_object_condition(companion_object)
            .cloned()
            .ok_or(CompanionDesignationError::NoCompanionAbility)?;

        let facts = starting_deck
            .iter()
            .map(|object_id| {
                self.object(*object_id)
                    .map(companion_deck_facts_from_object)
                    .ok_or(CompanionDesignationError::StartingDeckCardMissing)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !condition.is_fulfilled_by(&facts, minimum_deck_size) {
            return Err(CompanionDesignationError::ConditionNotFulfilled);
        }

        self.player_mut(player)
            .ok_or(CompanionDesignationError::PlayerNotFound)?
            .companion = Some(companion);
        Ok(())
    }
}
