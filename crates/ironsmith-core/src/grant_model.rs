use crate::{
    AlternativeCastingMethod, CardType, CostComponent, ManaCost, ObjectFilter, PlayerFilter,
    ThisSpellCostCondition, Zone,
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
    /// Retrace using the card's mana cost plus discarding a land card.
    RetraceFromCardManaCost,
    /// Blitz using the card's mana cost.
    BlitzFromCardManaCost,
    /// Emerge using the card's mana cost and sacrificing a creature.
    EmergeFromCardManaCost,
    /// Miracle using the card's mana cost reduced by a fixed generic amount.
    MiracleFromCardManaCostReducedBy { reduction: u32 },
    /// Escape using the card's mana cost and exiling N other graveyard cards.
    EscapeFromCardManaCost { exile_count: u32 },
    /// Cast from hand by paying generic mana equal to the card's mana value.
    ManaValueAsGenericFromHand,
    /// Cast from hand by paying life equal to the card's mana value.
    LifeEqualManaValueFromHand {
        usage_limit: Option<GrantUsageLimit>,
    },
    /// Cast from the graveyard using the card's mana cost plus optional extra cost components.
    GraveyardCastFromCardManaCost {
        additional_costs: Vec<C>,
        usage_limit: Option<GrantUsageLimit>,
        condition: Option<ThisSpellCostCondition>,
        exiles_after_resolution: bool,
    },
}

impl<C> DerivedAlternativeCast<C> {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FlashbackFromCardManaCost { .. } => "flashback",
            Self::RetraceFromCardManaCost => "Retrace",
            Self::BlitzFromCardManaCost => "Blitz",
            Self::EmergeFromCardManaCost => "Emerge",
            Self::MiracleFromCardManaCostReducedBy { .. } => "Miracle",
            Self::EscapeFromCardManaCost { .. } => "Escape",
            Self::ManaValueAsGenericFromHand => "Pay mana value",
            Self::LifeEqualManaValueFromHand { .. } => "Pay life equal to mana value",
            Self::GraveyardCastFromCardManaCost { .. } => "Cast from graveyard",
        }
    }

    pub fn usage_limit(&self) -> Option<GrantUsageLimit> {
        match self {
            Self::LifeEqualManaValueFromHand { usage_limit } => *usage_limit,
            Self::GraveyardCastFromCardManaCost { usage_limit, .. } => *usage_limit,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantUsageLimit {
    OnceDuringEachOfYourTurns,
}

impl<C: CostComponent> DerivedAlternativeCast<C> {
    pub fn flashback_from_cards_mana_cost() -> Self {
        Self::FlashbackFromCardManaCost {
            additional_costs: Vec::new(),
        }
    }

    pub fn blitz_from_cards_mana_cost() -> Self {
        Self::BlitzFromCardManaCost
    }

    pub fn retrace_from_cards_mana_cost() -> Self {
        Self::RetraceFromCardManaCost
    }

    pub fn emerge_from_cards_mana_cost() -> Self {
        Self::EmergeFromCardManaCost
    }

    pub fn escape_from_cards_mana_cost(exile_count: u32) -> Self {
        Self::EscapeFromCardManaCost { exile_count }
    }

    pub fn miracle_from_cards_mana_cost_reduced_by(reduction: u32) -> Self {
        Self::MiracleFromCardManaCostReducedBy { reduction }
    }

    pub fn once_each_turn_graveyard_cast_from_cards_mana_cost(additional_costs: Vec<C>) -> Self {
        Self::once_each_turn_graveyard_cast_from_cards_mana_cost_exiles_after_resolution(
            additional_costs,
            false,
        )
    }

    pub fn once_each_turn_graveyard_cast_from_cards_mana_cost_exiles_after_resolution(
        additional_costs: Vec<C>,
        exiles_after_resolution: bool,
    ) -> Self {
        Self::GraveyardCastFromCardManaCost {
            additional_costs,
            usage_limit: Some(GrantUsageLimit::OnceDuringEachOfYourTurns),
            condition: None,
            exiles_after_resolution,
        }
    }

    pub fn life_equal_mana_value_from_hand(usage_limit: Option<GrantUsageLimit>) -> Self {
        Self::LifeEqualManaValueFromHand { usage_limit }
    }

    pub fn graveyard_cast_from_cards_mana_cost_with_condition(
        condition: ThisSpellCostCondition,
        exiles_after_resolution: bool,
    ) -> Self {
        Self::GraveyardCastFromCardManaCost {
            additional_costs: Vec::new(),
            usage_limit: None,
            condition: Some(condition),
            exiles_after_resolution,
        }
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

    /// Create a grantable for blitz that uses the granted card's mana cost.
    pub fn blitz_from_cards_mana_cost() -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::blitz_from_cards_mana_cost())
    }

    /// Create a grantable for retrace that uses the granted card's mana cost.
    pub fn retrace_from_cards_mana_cost() -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::retrace_from_cards_mana_cost())
    }

    /// Create a grantable for emerge that uses the granted card's mana cost.
    pub fn emerge_from_cards_mana_cost() -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::emerge_from_cards_mana_cost())
    }

    /// Create a grantable for escape with the given exile count and the granted card's mana cost.
    pub fn escape(exile_count: u32) -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::escape_from_cards_mana_cost(
            exile_count,
        ))
    }

    /// Create a grantable for miracle whose cost is the granted card's mana cost reduced by a fixed amount.
    pub fn miracle_from_cards_mana_cost_reduced_by(reduction: u32) -> Self {
        Self::DerivedAlternativeCast(
            DerivedAlternativeCast::miracle_from_cards_mana_cost_reduced_by(reduction),
        )
    }

    /// Create a grantable for casting from hand by paying generic mana equal
    /// to the granted card's mana value.
    pub fn mana_value_as_generic_from_hand() -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::ManaValueAsGenericFromHand)
    }

    /// Create a grantable for casting from hand by paying life equal to
    /// the granted card's mana value.
    pub fn life_equal_mana_value_from_hand(usage_limit: Option<GrantUsageLimit>) -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::life_equal_mana_value_from_hand(
            usage_limit,
        ))
    }

    pub fn once_each_turn_graveyard_cast_from_cards_mana_cost(additional_costs: Vec<C>) -> Self {
        Self::DerivedAlternativeCast(
            DerivedAlternativeCast::once_each_turn_graveyard_cast_from_cards_mana_cost(
                additional_costs,
            ),
        )
    }

    pub fn once_each_turn_graveyard_cast_from_cards_mana_cost_exiles_after_resolution(
        additional_costs: Vec<C>,
        exiles_after_resolution: bool,
    ) -> Self {
        Self::DerivedAlternativeCast(
            DerivedAlternativeCast::once_each_turn_graveyard_cast_from_cards_mana_cost_exiles_after_resolution(
                additional_costs,
                exiles_after_resolution,
            ),
        )
    }

    pub fn graveyard_cast_from_cards_mana_cost_with_condition(
        condition: ThisSpellCostCondition,
        exiles_after_resolution: bool,
    ) -> Self {
        Self::DerivedAlternativeCast(
            DerivedAlternativeCast::graveyard_cast_from_cards_mana_cost_with_condition(
                condition,
                exiles_after_resolution,
            ),
        )
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
    /// Static abilities granted to a spell as it is cast using this permission.
    pub cast_this_way_grants: Vec<SA>,
}

impl<SA, E, C, Cond> GrantSpec<SA, E, C, Cond> {
    /// Create a new grant specification.
    pub fn new(grantable: Grantable<SA, E, C, Cond>, filter: ObjectFilter, zone: Zone) -> Self {
        Self {
            grantable,
            filter,
            zone,
            beneficiary: PlayerFilter::You,
            cast_this_way_grants: Vec::new(),
        }
    }

    /// Return a copy of this grant specification with an explicit beneficiary.
    pub fn with_beneficiary(mut self, beneficiary: PlayerFilter) -> Self {
        self.beneficiary = beneficiary;
        self
    }

    pub fn with_cast_this_way_grant(mut self, ability: SA) -> Self {
        self.cast_this_way_grants.push(ability);
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
            cast_this_way_grants: Vec::new(),
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
            cast_this_way_grants: Vec::new(),
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

        fn list_card_types(types: &[CardType]) -> String {
            let names = types
                .iter()
                .map(|card_type| card_type.to_string().to_ascii_lowercase())
                .collect::<Vec<_>>();
            match names.as_slice() {
                [] => String::new(),
                [one] => one.clone(),
                [left, right] => format!("{left} or {right}"),
                _ => {
                    let Some((last, rest)) = names.split_last() else {
                        return String::new();
                    };
                    format!("{}, or {last}", rest.join(", "))
                }
            }
        }

        fn is_simple_card_type_filter(filter: &ObjectFilter) -> bool {
            if filter.card_types.is_empty() {
                return false;
            }
            let mut normalized = filter.clone();
            normalized.card_types.clear();
            normalized.zone = None;
            normalized == ObjectFilter::default()
        }

        fn article_for(phrase: &str) -> &'static str {
            match phrase.chars().next().map(|c| c.to_ascii_lowercase()) {
                Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
                _ => "a",
            }
        }

        fn merged_simple_any_of_card_type_filter(filter: &ObjectFilter) -> Option<ObjectFilter> {
            if filter.any_of.is_empty() {
                return None;
            }
            let mut merged = ObjectFilter::default();
            for branch in &filter.any_of {
                if !is_simple_card_type_filter(branch) || branch.card_types.len() != 1 {
                    return None;
                }
                merged.card_types.push(branch.card_types[0]);
            }
            Some(merged)
        }

        fn castable_filter_description(filter: &ObjectFilter) -> String {
            if filter.card_types.as_slice() == [CardType::Creature]
                && let Some(crate::filter_model::Comparison::GreaterThanOrEqual(power)) = filter.power
            {
                let mut normalized = filter.clone();
                normalized.card_types.clear();
                normalized.power = None;
                if normalized == ObjectFilter::default() {
                    return format!("creature spells with power {power} or greater");
                }
            }
            if let Some(merged) = merged_simple_any_of_card_type_filter(filter) {
                return castable_filter_description(&merged);
            }
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
            if is_simple_card_type_filter(filter) {
                let permanent_types = [
                    CardType::Artifact,
                    CardType::Creature,
                    CardType::Enchantment,
                    CardType::Planeswalker,
                    CardType::Battle,
                ];
                if filter.card_types == permanent_types {
                    return "a permanent spell".to_string();
                }
                let type_text = list_card_types(&filter.card_types);
                return format!("{} {type_text} spell", article_for(type_text.as_str()));
            }
            let description = filter.description();
            if description.contains("permanent") {
                description.replace("permanent", "spell")
            } else if description.contains("spell") {
                description
            } else if description.contains(" card") {
                description.replace(" card", " spell")
            } else {
                format!("{description} spells")
            }
        }

        fn sacrifice_cost_filter_description(filter: &ObjectFilter) -> Option<String> {
            let mut normalized = filter.clone();
            normalized.controller = None;
            normalized.zone = None;
            if !matches!(filter.controller, None | Some(PlayerFilter::You))
                || !is_simple_card_type_filter(&normalized)
            {
                return None;
            }
            let type_text = list_card_types(&normalized.card_types);
            Some(format!("{} {type_text}", article_for(type_text.as_str())))
        }

        fn graveyard_cast_cost_text<C: CostComponent>(additional_costs: &[C]) -> String {
            if let [cost] = additional_costs
                && let Some(filter) = cost.sacrifice_filter()
                && let Some(filter_text) = sacrifice_cost_filter_description(filter)
            {
                return format!(
                    "sacrificing {filter_text} in addition to paying its other costs"
                );
            }

            if additional_costs.is_empty() {
                "paying its mana cost".to_string()
            } else {
                format!(
                    "paying its mana cost plus {}",
                    additional_costs
                        .iter()
                        .map(CostComponent::display)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
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
                PlayerFilter::HasMoreLifeThanYou { .. } => "That player may".to_string(),
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
        let cast_this_way_suffix = || {
            if self.cast_this_way_grants.is_empty() {
                return String::new();
            }
            let grants = self
                .cast_this_way_grants
                .iter()
                .map(GrantStaticAbility::grant_display)
                .collect::<Vec<_>>();
            if grants.len() == 1 && grants[0].eq_ignore_ascii_case("haste") {
                let cast_desc = castable_filter_description(&self.filter);
                let spell_text = if let Some(base) = cast_desc.strip_suffix(" spells") {
                    format!("a {base} spell")
                } else if let Some((base, tail)) = cast_desc.split_once(" spells ") {
                    format!("a {base} spell {tail}")
                } else if cast_desc.starts_with("a ") {
                    cast_desc
                } else {
                    "a spell".to_string()
                };
                return format!(
                    ". If you cast {spell_text} this way, it gains haste until end of turn"
                );
            }
            format!(". Spells cast this way gain {}", grants.join(" and "))
        };

        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Graveyard
            && self.filter == ObjectFilter::source()
        {
            return format!("{may_prefix} cast this card from your graveyard");
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Exile
            && self.filter == ObjectFilter::source()
        {
            return format!("{may_prefix} cast this card from exile");
        }
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
                "{may_prefix} cast {} from the top of your library",
                castable_filter_description(&self.filter)
            ) + &cast_this_way_suffix();
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
            let count_text =
                crate::cardinal_word(*exile_count).unwrap_or_else(|| exile_count.to_string());
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
        if let Grantable::DerivedAlternativeCast(DerivedAlternativeCast::RetraceFromCardManaCost) =
            &self.grantable
            && self.zone == Zone::Graveyard
        {
            let filter_desc = castable_filter_description(&filter);
            return format!("Each {filter_desc} has retrace");
        }
        if let Grantable::DerivedAlternativeCast(DerivedAlternativeCast::BlitzFromCardManaCost) =
            &self.grantable
        {
            let filter_desc = castable_filter_description(&filter);
            return format!(
                "Each {filter_desc} has blitz. The blitz cost is equal to its mana cost"
            );
        }
        if let Grantable::DerivedAlternativeCast(DerivedAlternativeCast::EmergeFromCardManaCost) =
            &self.grantable
            && self.zone == Zone::Hand
        {
            let cast_desc = castable_filter_description(&self.filter);
            let filter_desc = cast_desc
                .strip_suffix(" spells")
                .map(|base| format!("{base} spell you cast"))
                .unwrap_or(cast_desc);
            return format!(
                "Each {filter_desc} has emerge. The emerge cost is equal to its mana cost"
            );
        }
        if let Grantable::DerivedAlternativeCast(
            DerivedAlternativeCast::MiracleFromCardManaCostReducedBy { reduction },
        ) = &self.grantable
            && self.zone == Zone::Hand
        {
            let mut base_filter = self.filter.clone();
            base_filter.zone = None;
            if matches!(base_filter.owner, Some(PlayerFilter::You)) {
                base_filter.owner = None;
            }
            let base = castable_filter_description(&base_filter)
                .strip_suffix(" spells")
                .unwrap_or("cards")
                .to_string();
            let owner_phrase = if matches!(self.filter.owner, Some(PlayerFilter::You)) {
                "your"
            } else {
                "that"
            };
            return format!(
                "Each {base} card in {owner_phrase} hand has miracle. Its miracle cost is equal to its mana cost reduced by {}",
                format!("{{{reduction}}}")
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
        if let Grantable::DerivedAlternativeCast(
            DerivedAlternativeCast::LifeEqualManaValueFromHand { usage_limit },
        ) = &self.grantable
            && self.zone == Zone::Hand
        {
            let prefix = if matches!(
                usage_limit,
                Some(GrantUsageLimit::OnceDuringEachOfYourTurns)
            ) {
                "Once during each of your turns, "
            } else {
                ""
            };
            let filter_desc = castable_filter_description(&self.filter);
            let singular_filter_desc = filter_desc
                .strip_suffix(" spells")
                .map(|base| format!("a {base} spell"))
                .unwrap_or(filter_desc);
            return format!(
                "{prefix}{} cast {singular_filter_desc} by paying life equal to its mana value rather than paying its mana cost",
                may_prefix.to_ascii_lowercase()
            );
        }
        if let Grantable::DerivedAlternativeCast(
            DerivedAlternativeCast::GraveyardCastFromCardManaCost {
                additional_costs,
                usage_limit,
                condition,
                exiles_after_resolution,
            },
        ) = &self.grantable
            && self.zone == Zone::Graveyard
        {
            let mut cast_filter = self.filter.clone();
            cast_filter.zone = None;
            let filter_desc = castable_filter_description(&cast_filter);
            let cost_text = graveyard_cast_cost_text(additional_costs);
            if self.filter == ObjectFilter::source() {
                let mut line = format!("{may_prefix} cast this card from your graveyard");
                if let Some(condition) = condition
                    && let Some(condition_text) = describe_cast_condition(condition)
                {
                    line.push_str(" as long as ");
                    line.push_str(&condition_text);
                }
                if *exiles_after_resolution {
                    line.push_str(". If you cast it this way and it would be put into your graveyard, exile it instead");
                }
                return line;
            }
            let prefix = if matches!(
                usage_limit,
                Some(GrantUsageLimit::OnceDuringEachOfYourTurns)
            ) {
                "Once during each of your turns, "
            } else {
                ""
            };
            let mut line = format!(
                "{prefix}{} cast {} from your graveyard by {}",
                may_prefix.to_ascii_lowercase(),
                filter_desc,
                cost_text
            );
            if *exiles_after_resolution {
                line.push_str(
                    ". If a spell cast this way would be put into your graveyard, exile it instead",
                );
            }
            return line;
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

fn describe_cast_condition(condition: &ThisSpellCostCondition) -> Option<String> {
    match condition {
        ThisSpellCostCondition::Always => None,
        ThisSpellCostCondition::ConditionExpr { display, .. } => Some(display.clone()),
        ThisSpellCostCondition::YourTurn => Some("it's your turn".to_string()),
        ThisSpellCostCondition::NotYourTurn => Some("it isn't your turn".to_string()),
        _ => Some("the condition is true".to_string()),
    }
}
