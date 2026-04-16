use crate::alternative_cast::AlternativeCastingMethod;
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub enum Grantable {
    Ability(StaticAbility),
    AlternativeCast(AlternativeCastingMethod),
    PlayFrom,
    FlashbackFromCardManaCost,
    EscapeFromCardManaCost { exile_count: u32 },
    ManaValueAsGenericFromHand,
}

impl Grantable {
    pub fn ability(ability: StaticAbility) -> Self {
        Self::Ability(ability)
    }

    pub fn play_from() -> Self {
        Self::PlayFrom
    }

    pub fn flashback_from_cards_mana_cost() -> Self {
        Self::FlashbackFromCardManaCost
    }

    pub fn escape(exile_count: u32) -> Self {
        Self::EscapeFromCardManaCost { exile_count }
    }

    pub fn mana_value_as_generic_from_hand() -> Self {
        Self::ManaValueAsGenericFromHand
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantDuration {
    Forever,
    UntilEndOfTurn,
    UntilYourNextTurnEnd,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantSpec {
    pub grantable: Grantable,
    pub filter: ObjectFilter,
    pub zone: Zone,
    pub beneficiary: PlayerFilter,
}

impl GrantSpec {
    pub fn new(grantable: Grantable, filter: ObjectFilter, zone: Zone) -> Self {
        Self {
            grantable,
            filter,
            zone,
            beneficiary: PlayerFilter::You,
        }
    }

    pub fn with_beneficiary(mut self, beneficiary: PlayerFilter) -> Self {
        self.beneficiary = beneficiary;
        self
    }

    pub fn flash_to_spells() -> Self {
        Self::flash_to_spells_matching(ObjectFilter::nonland())
    }

    pub fn flash_to_noncreature_spells() -> Self {
        Self::flash_to_spells_matching(ObjectFilter::noncreature_spell())
    }

    pub fn flash_to_spells_matching(filter: ObjectFilter) -> Self {
        Self {
            grantable: Grantable::Ability(StaticAbility::flash()),
            filter,
            zone: Zone::Hand,
            beneficiary: PlayerFilter::You,
        }
    }

    pub fn play_lands_from_graveyard() -> Self {
        Self::new(
            Grantable::play_from(),
            ObjectFilter::default().with_type(crate::types::CardType::Land),
            Zone::Graveyard,
        )
    }

    pub fn play_from_graveyard() -> Self {
        Self::new(
            Grantable::play_from(),
            ObjectFilter::default(),
            Zone::Graveyard,
        )
    }

    pub fn cast_from_hand_without_paying_mana_cost_matching(filter: ObjectFilter) -> Self {
        Self::new(
            Grantable::AlternativeCast(AlternativeCastingMethod::alternative_cost(
                "Cast without paying mana cost",
                None,
                Vec::new(),
            )),
            filter,
            Zone::Hand,
        )
    }

    pub fn cast_from_hand_for_alternative_mana_cost_matching(
        filter: ObjectFilter,
        mana_cost: crate::mana::ManaCost,
    ) -> Self {
        Self::new(
            Grantable::AlternativeCast(AlternativeCastingMethod::Composed {
                name: "alternative mana cost",
                total_cost: crate::cost::TotalCost::mana(mana_cost),
                condition: None,
            }),
            filter,
            Zone::Hand,
        )
    }
}
