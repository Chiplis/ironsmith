//! Companion designation and starting-deck validation (CR 103.2b, 702.139).

use crate::ability::AbilityKind;
use crate::cards::CardDefinition;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::Object;
use crate::static_abilities::{CompanionDeckCardFacts, CompanionDeckCondition, StaticAbilityId};
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

/// Whether a supplied static ability can define this card before the game.
/// Earlier compiled definitions omit OutsideGame from an explicit zone
/// exclusion. CR 113.6c also applies that exclusion before the game begins.
fn pregame_static(ability: &crate::Ability) -> bool {
    let AbilityKind::Static(rule) = &ability.kind else {
        return false;
    };
    rule.may_generate_continuous_effects() && (ability.functions_in(&Zone::OutsideGame)
        || rule.compiled_model().is_some_and(|model| matches!(&model.payload,
            ironsmith_core::StaticAbilityPayload::Conditional { condition: crate::ConditionExpr::Not(inner), .. }
                if matches!(inner.as_ref(), crate::ConditionExpr::SourceIsInZone(zone) if *zone != Zone::OutsideGame))))
}

pub fn companion_deck_facts_from_object(object: &Object) -> CompanionDeckCardFacts {
    let mut facts = CompanionDeckCardFacts {
        name: object.name.to_owned_string(),
        mana_cost: object
            .split_combined_mana_cost()
            .or_else(|| object.mana_cost.as_ref().map(|cost| cost.to_owned_value())),
        card_types: object.zone_card_types().to_vec(),
        subtypes: object.zone_subtypes().to_vec(),
        has_all_creature_types: false,
        has_activated_ability: false,
    };
    let abilities = object.abilities.iter().chain(
        object
            .split_combined_active()
            .into_iter()
            .flat_map(|c| c.other_half_abilities.iter()),
    );
    for ability in abilities {
        match &ability.kind {
            AbilityKind::Activated(_) => facts.has_activated_ability = true,
            AbilityKind::Static(rule) if rule.id() == StaticAbilityId::Changeling => {
                facts.has_all_creature_types = true
            }
            _ => {}
        }
    }
    // No battlefield effects apply during deck construction. Evaluate only
    // the card's own pregame abilities, in a world isolated from the match.
    if object.abilities.iter().any(pregame_static) {
        let mut card = object.clone();
        let owner = PlayerId(0);
        card.owner = owner;
        card.initial_controller = owner;
        card.zone = Zone::OutsideGame;
        for ability in card
            .abilities_mut()
            .iter_mut()
            .filter(|a| pregame_static(a))
        {
            if !ability.functions_in(&Zone::OutsideGame) {
                ability.functional_zones.push(Zone::OutsideGame);
            }
        }
        let mut pregame = GameState::new(vec![String::from("Owner")], 20);
        pregame.add_object(card);
        if let Some(chars) = pregame.current_characteristics(object.id) {
            facts.card_types = chars.card_types.to_vec();
            facts.subtypes = chars.subtypes.to_vec();
        }
    }
    facts.has_activated_ability |= facts.subtypes.iter().any(|s| s.is_basic_land_type());
    facts
}

fn definition_facts(
    definition: &CardDefinition,
    other: Option<&CardDefinition>,
) -> CompanionDeckCardFacts {
    let make_object = |def: &CardDefinition| {
        Object::from_card_definition(ObjectId(0), def, PlayerId(0), Zone::OutsideGame)
    };
    let mut object = make_object(definition);
    if let Some(other) = other {
        object.linked_face_mana_cost = other.card.mana_cost.clone().map(Into::into);
        object.split_combined = Some(
            crate::object::SplitCombinedCharacteristics::from_halves(&object, &make_object(other))
                .into(),
        );
    }
    companion_deck_facts_from_object(&object)
}

pub fn companion_deck_facts_from_definition(definition: &CardDefinition) -> CompanionDeckCardFacts {
    let other = (definition.card.linked_face_layout == crate::card::LinkedFaceLayout::Split)
        .then(|| {
            crate::cards::linked_face_definition_by_name_or_id(
                definition.card.other_face_name.as_deref(),
                definition.card.other_face,
            )
        })
        .flatten();
    definition_facts(definition, other.as_ref())
}

pub fn validate_companion_definition(
    companion: &CardDefinition,
    starting_deck: &[CardDefinition],
    minimum_deck_size: usize,
) -> Result<(), CompanionDesignationError> {
    GameState::new(vec![String::from("Owner")], 20).validate_companion_definition(
        companion,
        starting_deck,
        minimum_deck_size,
    )
}

impl GameState {
    /// CR 709.4: deck construction requires the complete registered split
    /// family, including when the supplied definitions came from a local catalog.
    pub fn validate_companion_definition(
        &self,
        companion: &CardDefinition,
        starting_deck: &[CardDefinition],
        minimum_deck_size: usize,
    ) -> Result<(), CompanionDesignationError> {
        let condition = companion_definition_condition(companion)
            .ok_or(CompanionDesignationError::NoCompanionAbility)?;
        let facts = starting_deck
            .iter()
            .map(|def| {
                let other = if def.card.linked_face_layout == crate::card::LinkedFaceLayout::Split {
                    Some(
                        self.linked_face_definition_by_name_or_id(
                            def.card.other_face_name.as_deref(),
                            def.card.other_face,
                        )
                        .ok_or(CompanionDesignationError::StartingDeckCardMissing)?,
                    )
                } else {
                    None
                };
                Ok(definition_facts(def, other.as_ref()))
            })
            .collect::<Result<Vec<_>, CompanionDesignationError>>()?;
        condition
            .is_fulfilled_by(&facts, minimum_deck_size)
            .then_some(())
            .ok_or(CompanionDesignationError::ConditionNotFulfilled)
    }

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
