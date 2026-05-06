use crate::{
    AlternativeCastingMethod, CardType, CostComponent, ManaCost, ObjectFilter, PlayerFilter, Zone,
};

pub trait GrantStaticAbility: Clone + PartialEq {
    fn grant_flash() -> Self;
    fn grant_display(&self) -> String;
    fn grant_has_flash(&self) -> bool;
}

/// A granted alternative cast whose exact cost is derived from the granted card.
#[derive(Debug, Clone, PartialEq)]
pub enum DerivedAlternativeCast<C> {
    /// Flashback using the card's mana cost plus optional extra cost components.
    FlashbackFromCardManaCost { additional_costs: Vec<C> },
    /// Escape using the card's mana cost and exiling N other graveyard cards.
    EscapeFromCardManaCost { exile_count: u32 },
    /// Cast from hand by paying generic mana equal to the card's mana value.
    ManaValueAsGenericFromHand,
}

impl<C> DerivedAlternativeCast<C> {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FlashbackFromCardManaCost { .. } => "flashback",
            Self::EscapeFromCardManaCost { .. } => "Escape",
            Self::ManaValueAsGenericFromHand => "Pay mana value",
        }
    }
}

impl<C: CostComponent> DerivedAlternativeCast<C> {
    pub fn flashback_from_cards_mana_cost() -> Self {
        Self::FlashbackFromCardManaCost {
            additional_costs: Vec::new(),
        }
    }

    pub fn escape_from_cards_mana_cost(exile_count: u32) -> Self {
        Self::EscapeFromCardManaCost { exile_count }
    }
}

/// Duration for one-shot grant effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantDuration {
    /// Permanent (for effects that say "gains X" without duration).
    Forever,
    /// Until end of turn.
    UntilEndOfTurn,
    /// Until the end of the controller's next turn.
    UntilYourNextTurnEnd,
}

/// What can be granted to a card.
#[derive(Debug, Clone, PartialEq)]
pub enum Grantable<SA, E, C, Cond> {
    /// Grant a static ability (flash, flying, hexproof, etc.)
    Ability(SA),
    /// Grant an alternative casting method (flashback, escape, etc.)
    AlternativeCast(AlternativeCastingMethod<E, C, Cond>),
    /// Grant an alternative casting method whose exact cost is derived from the card.
    DerivedAlternativeCast(DerivedAlternativeCast<C>),
    /// Grant the ability to play a card from a non-hand zone as if it were in hand.
    PlayFrom,
}

impl<SA, E, C, Cond> Grantable<SA, E, C, Cond> {
    /// Create a grantable for a static ability.
    pub fn ability(ability: SA) -> Self {
        Self::Ability(ability)
    }

    /// Create a grantable for playing cards from a non-hand zone as if from hand.
    pub fn play_from() -> Self {
        Self::PlayFrom
    }
}

impl<SA, E, C, Cond> Grantable<SA, E, C, Cond>
where
    C: CostComponent,
{
    /// Create a grantable for flashback that uses the granted card's mana cost.
    pub fn flashback_from_cards_mana_cost() -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::flashback_from_cards_mana_cost())
    }

    /// Create a grantable for escape with the given exile count and the granted card's mana cost.
    pub fn escape(exile_count: u32) -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::escape_from_cards_mana_cost(
            exile_count,
        ))
    }

    /// Create a grantable for casting from hand by paying generic mana equal
    /// to the granted card's mana value.
    pub fn mana_value_as_generic_from_hand() -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::ManaValueAsGenericFromHand)
    }
}

impl<SA, E, C, Cond> Grantable<SA, E, C, Cond>
where
    SA: GrantStaticAbility,
    E: Clone,
    C: CostComponent,
    Cond: Clone,
{
    /// Get a display string for this grantable.
    pub fn display(&self) -> String {
        match self {
            Self::Ability(a) => a.grant_display(),
            Self::AlternativeCast(m) => m.name().to_string(),
            Self::DerivedAlternativeCast(spec) => spec.display_name().to_string(),
            Self::PlayFrom => "play from zone".to_string(),
        }
    }
}

/// A grant specification describing what to grant and to whom.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantSpec<SA, E, C, Cond> {
    /// What to grant (ability or alternative casting method).
    pub grantable: Grantable<SA, E, C, Cond>,
    /// Filter for cards that receive this grant.
    pub filter: ObjectFilter,
    /// The zone where this grant applies.
    pub zone: Zone,
    /// Which player may use the grant when rendered or applied statically.
    pub beneficiary: PlayerFilter,
}

impl<SA, E, C, Cond> GrantSpec<SA, E, C, Cond> {
    /// Create a new grant specification.
    pub fn new(grantable: Grantable<SA, E, C, Cond>, filter: ObjectFilter, zone: Zone) -> Self {
        Self {
            grantable,
            filter,
            zone,
            beneficiary: PlayerFilter::You,
        }
    }

    /// Return a copy of this grant specification with an explicit beneficiary.
    pub fn with_beneficiary(mut self, beneficiary: PlayerFilter) -> Self {
        self.beneficiary = beneficiary;
        self
    }

    /// Create a grant spec for playing cards from your graveyard.
    pub fn play_from_graveyard() -> Self {
        Self::new(
            Grantable::play_from(),
            ObjectFilter::default(),
            Zone::Graveyard,
        )
    }

    /// Create a grant spec for playing lands from your graveyard.
    pub fn play_lands_from_graveyard() -> Self {
        Self::new(
            Grantable::play_from(),
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
        )
    }
}

impl<SA, E, C, Cond> GrantSpec<SA, E, C, Cond>
where
    SA: GrantStaticAbility,
{
    /// Create a grant spec for flash to spells in hand.
    pub fn flash_to_spells() -> Self {
        Self::flash_to_spells_matching(ObjectFilter::nonland())
    }

    /// Create a grant spec for flash to matching spells in hand.
    pub fn flash_to_spells_matching(filter: ObjectFilter) -> Self {
        Self {
            grantable: Grantable::Ability(SA::grant_flash()),
            filter,
            zone: Zone::Hand,
            beneficiary: PlayerFilter::You,
        }
    }

    /// Create a grant spec for flash to noncreature spells in hand.
    pub fn flash_to_noncreature_spells() -> Self {
        Self::flash_to_spells_matching(ObjectFilter::noncreature_spell())
    }
}

impl<SA, E, C, Cond> GrantSpec<SA, E, C, Cond>
where
    E: Clone,
    C: CostComponent,
    Cond: Clone,
{
    /// Create a grant spec for casting matching spells from hand without paying mana cost.
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

    /// Create a grant spec for casting matching spells from hand for a fixed alternative mana cost.
    pub fn cast_from_hand_for_alternative_mana_cost_matching(
        filter: ObjectFilter,
        mana_cost: ManaCost,
    ) -> Self {
        Self::new(
            Grantable::AlternativeCast(AlternativeCastingMethod::alternative_cost(
                "alternative mana cost",
                Some(mana_cost),
                Vec::new(),
            )),
            filter,
            Zone::Hand,
        )
    }
}

impl<SA, E, C, Cond> GrantSpec<SA, E, C, Cond>
where
    C: CostComponent,
{
    /// Create a grant spec for escape to nonland cards in graveyard.
    pub fn escape_to_nonland(exile_count: u32) -> Self {
        Self {
            grantable: Grantable::escape(exile_count),
            filter: ObjectFilter::nonland(),
            zone: Zone::Graveyard,
            beneficiary: PlayerFilter::You,
        }
    }
}

impl<SA, E, C, Cond> GrantSpec<SA, E, C, Cond>
where
    SA: GrantStaticAbility,
    E: Clone,
    C: CostComponent,
    Cond: Clone,
{
    /// Get a display string for this grant specification.
    pub fn display(&self) -> String {
        fn small_number_word(n: u32) -> Option<&'static str> {
            match n {
                0 => Some("zero"),
                1 => Some("one"),
                2 => Some("two"),
                3 => Some("three"),
                4 => Some("four"),
                5 => Some("five"),
                6 => Some("six"),
                7 => Some("seven"),
                8 => Some("eight"),
                9 => Some("nine"),
                10 => Some("ten"),
                _ => None,
            }
        }

        fn zone_name(zone: Zone) -> &'static str {
            match zone {
                Zone::Battlefield => "battlefield",
                Zone::Hand => "hand",
                Zone::Library => "library",
                Zone::Graveyard => "graveyard",
                Zone::Exile => "exile",
                Zone::Stack => "stack",
                Zone::Command => "command zone",
                Zone::OutsideGame => "outside the game",
            }
        }

        fn castable_filter_description(filter: &ObjectFilter) -> String {
            if !filter.any_of.is_empty() {
                return filter
                    .any_of
                    .iter()
                    .map(castable_filter_description)
                    .collect::<Vec<_>>()
                    .join(" or ");
            }
            if *filter == ObjectFilter::noncreature_spell() {
                return "noncreature spells".to_string();
            }
            let description = filter.description();
            if description.contains("permanent") {
                description.replace("permanent", "spell")
            } else if description.contains("spell") || description.contains("card") {
                description
            } else {
                format!("{description} spells")
            }
        }

        fn flashback_filter_description(filter: &ObjectFilter) -> String {
            if filter.any_of.len() == 2 {
                let first = &filter.any_of[0];
                let second = &filter.any_of[1];
                if first.zone == second.zone
                    && first.owner == second.owner
                    && first.controller == second.controller
                    && first.card_types.len() == 1
                    && second.card_types.len() == 1
                    && first.subtypes.is_empty()
                    && second.subtypes.is_empty()
                    && first.supertypes.is_empty()
                    && second.supertypes.is_empty()
                {
                    let mut merged = first.clone();
                    merged.card_types = vec![first.card_types[0], second.card_types[0]];
                    return merged.description();
                }
            }
            filter.description()
        }

        fn beneficiary_may_prefix(beneficiary: &PlayerFilter) -> String {
            match beneficiary {
                PlayerFilter::Any => "Any player may".to_string(),
                PlayerFilter::You => "You may".to_string(),
                PlayerFilter::NotYou => "Any player other than you may".to_string(),
                PlayerFilter::Opponent => "Opponent may".to_string(),
                PlayerFilter::Teammate => "A teammate may".to_string(),
                PlayerFilter::Active => "The active player may".to_string(),
                PlayerFilter::Defending => "The defending player may".to_string(),
                PlayerFilter::Attacking => "The attacking player may".to_string(),
                PlayerFilter::DamagedPlayer => "That player may".to_string(),
                PlayerFilter::EffectController => "The player who cast this spell may".to_string(),
                PlayerFilter::Specific(_) => "That player may".to_string(),
                PlayerFilter::MostLifeTied => {
                    "The player with the most life or tied for most life may".to_string()
                }
                PlayerFilter::LowestLifeTied => {
                    "The player with the lowest life or tied for lowest life may".to_string()
                }
                PlayerFilter::MostCardsInHand => {
                    "The player who has the most cards in hand may".to_string()
                }
                PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
                    "Any player who cast one or more {} spells this turn may",
                    card_type.to_string().to_ascii_lowercase()
                ),
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => "That player may".to_string(),
                PlayerFilter::MaxSpeed { .. } => "That player may".to_string(),
                PlayerFilter::ChosenPlayer => "The chosen player may".to_string(),
                PlayerFilter::TaggedPlayer(_)
                | PlayerFilter::IteratedPlayer
                | PlayerFilter::AliasedOwnerOf(_)
                | PlayerFilter::AliasedControllerOf(_) => "That player may".to_string(),
                PlayerFilter::TargetPlayerOrControllerOfTarget => {
                    "That player or that object's controller may".to_string()
                }
                PlayerFilter::Excluding { .. } => "A player may".to_string(),
                PlayerFilter::Target(_) => "Target player may".to_string(),
                PlayerFilter::ControllerOf(_) => "That object's controller may".to_string(),
                PlayerFilter::OwnerOf(_) => "That object's owner may".to_string(),
            }
        }

        let mut filter = self.filter.clone();
        filter.zone.get_or_insert(self.zone);
        let filter_desc = filter.description();
        let may_prefix = beneficiary_may_prefix(&self.beneficiary);

        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Graveyard
            && self.filter.card_types.as_slice() == [CardType::Land]
        {
            return format!("{may_prefix} play lands from your graveyard");
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Graveyard
            && self.filter == ObjectFilter::default()
        {
            return format!("{may_prefix} play lands and cast spells from your graveyard");
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Library
            && self.filter == ObjectFilter::default()
        {
            return format!("{may_prefix} play lands and cast spells from the top of your library");
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Library
            && self.filter.card_types.as_slice() == [CardType::Land]
        {
            return format!("{may_prefix} play lands from the top of your library");
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Library
            && self.filter.any_of.len() == 2
        {
            let land_branch = self
                .filter
                .any_of
                .iter()
                .find(|branch| branch.card_types.as_slice() == [CardType::Land]);
            let other_branch = self
                .filter
                .any_of
                .iter()
                .find(|branch| branch.card_types.as_slice() != [CardType::Land]);
            if land_branch.is_some()
                && let Some(other) = other_branch
            {
                return format!(
                    "{may_prefix} play lands and cast {} from the top of your library",
                    castable_filter_description(other)
                );
            }
        }
        if matches!(self.grantable, Grantable::PlayFrom) && self.zone == Zone::Library {
            return format!(
                "{may_prefix} play {} from the top of your library",
                filter_desc
            );
        }
        if let Grantable::AlternativeCast(method) = &self.grantable
            && self.zone == Zone::Hand
            && self.filter == ObjectFilter::nonland()
            && method.cast_from_zone() == Zone::Hand
            && method.mana_cost().is_none()
            && method.non_mana_costs().is_empty()
        {
            return format!(
                "{may_prefix} cast spells from your hand without paying their mana costs"
            );
        }
        if let Grantable::AlternativeCast(method @ AlternativeCastingMethod::Composed { .. }) =
            &self.grantable
            && self.zone == Zone::Hand
            && method.cast_from_zone() == Zone::Hand
            && method.non_mana_costs().is_empty()
            && let Some(mana_cost) = method.mana_cost()
        {
            return format!(
                "{may_prefix} pay {} rather than pay the mana cost for {} you cast",
                mana_cost.to_oracle(),
                castable_filter_description(&self.filter)
            );
        }
        if let Grantable::DerivedAlternativeCast(DerivedAlternativeCast::EscapeFromCardManaCost {
            exile_count,
        }) = &self.grantable
            && self.zone == Zone::Graveyard
        {
            let count_text = small_number_word(*exile_count)
                .map(str::to_string)
                .unwrap_or_else(|| exile_count.to_string());
            let graveyard = if matches!(filter.owner, Some(PlayerFilter::You)) {
                "your graveyard"
            } else {
                "that graveyard"
            };
            return format!(
                "Each {filter_desc} has escape. The escape cost is equal to the card's mana cost plus exile {count_text} other cards from {graveyard}"
            );
        }
        if let Grantable::DerivedAlternativeCast(
            DerivedAlternativeCast::FlashbackFromCardManaCost { additional_costs },
        ) = &self.grantable
        {
            let filter_desc = flashback_filter_description(&filter);
            let cost_text = if additional_costs.is_empty() {
                "its mana cost".to_string()
            } else {
                format!(
                    "its mana cost plus {}",
                    additional_costs
                        .iter()
                        .map(CostComponent::display)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            return format!(
                "Each {filter_desc} has flashback. Its flashback cost is equal to {cost_text}"
            );
        }
        if let Grantable::DerivedAlternativeCast(DerivedAlternativeCast::ManaValueAsGenericFromHand) =
            &self.grantable
            && self.zone == Zone::Hand
        {
            return format!(
                "{may_prefix} pay {{X}} rather than pay the mana cost for {} you cast, where X is that spell's mana value",
                castable_filter_description(&self.filter)
            );
        }
        if let Grantable::Ability(ability) = &self.grantable
            && ability.grant_has_flash()
            && self.zone == Zone::Hand
        {
            if self.filter == ObjectFilter::nonland() {
                return format!("{may_prefix} cast spells as though they had flash");
            }
            return format!(
                "{may_prefix} cast {} as though they had flash",
                castable_filter_description(&self.filter)
            );
        }
        format!(
            "Cards in {} have {}",
            zone_name(self.zone),
            self.grantable.display()
        )
    }
}
