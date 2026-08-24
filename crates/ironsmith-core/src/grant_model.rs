use crate::{
    AlternativeCastingMethod, CardType, CostComponent, ManaCost, ObjectFilter, PlayerFilter,
    SourceReferenceSurface, ThisSpellCostCondition, Zone,
};

pub trait GrantStaticAbility: Clone + PartialEq {
    fn grant_flash() -> Self;
    fn grant_display(&self) -> String;
    fn grant_has_flash(&self) -> bool;
}

/// A granted alternative cast whose exact cost is derived from the granted card.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "derived casting grants preserve typed conditions and costs inline"
)]
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
    /// Cast from a specified zone by paying life equal to the card's mana value.
    LifeEqualManaValueFromZone {
        zone: Zone,
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
            Self::LifeEqualManaValueFromZone { .. } => "Pay life equal to mana value",
            Self::GraveyardCastFromCardManaCost { .. } => "Cast from graveyard",
        }
    }

    pub fn usage_limit(&self) -> Option<GrantUsageLimit> {
        match self {
            Self::LifeEqualManaValueFromHand { usage_limit } => *usage_limit,
            Self::LifeEqualManaValueFromZone { usage_limit, .. } => *usage_limit,
            Self::GraveyardCastFromCardManaCost { usage_limit, .. } => *usage_limit,
            _ => None,
        }
    }

    pub fn try_map<C2, Error>(
        self,
        mut map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<DerivedAlternativeCast<C2>, Error> {
        Ok(match self {
            Self::FlashbackFromCardManaCost { additional_costs } => {
                DerivedAlternativeCast::FlashbackFromCardManaCost {
                    additional_costs: additional_costs
                        .into_iter()
                        .map(&mut map_cost)
                        .collect::<Result<Vec<_>, _>>()?,
                }
            }
            Self::RetraceFromCardManaCost => DerivedAlternativeCast::RetraceFromCardManaCost,
            Self::BlitzFromCardManaCost => DerivedAlternativeCast::BlitzFromCardManaCost,
            Self::EmergeFromCardManaCost => DerivedAlternativeCast::EmergeFromCardManaCost,
            Self::MiracleFromCardManaCostReducedBy { reduction } => {
                DerivedAlternativeCast::MiracleFromCardManaCostReducedBy { reduction }
            }
            Self::EscapeFromCardManaCost { exile_count } => {
                DerivedAlternativeCast::EscapeFromCardManaCost { exile_count }
            }
            Self::ManaValueAsGenericFromHand => DerivedAlternativeCast::ManaValueAsGenericFromHand,
            Self::LifeEqualManaValueFromHand { usage_limit } => {
                DerivedAlternativeCast::LifeEqualManaValueFromHand { usage_limit }
            }
            Self::LifeEqualManaValueFromZone { zone, usage_limit } => {
                DerivedAlternativeCast::LifeEqualManaValueFromZone { zone, usage_limit }
            }
            Self::GraveyardCastFromCardManaCost {
                additional_costs,
                usage_limit,
                condition,
                exiles_after_resolution,
            } => DerivedAlternativeCast::GraveyardCastFromCardManaCost {
                additional_costs: additional_costs
                    .into_iter()
                    .map(&mut map_cost)
                    .collect::<Result<Vec<_>, _>>()?,
                usage_limit,
                condition,
                exiles_after_resolution,
            },
        })
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantUsageLimit {
    OnceEachTurn,
    OnceDuringEachOfYourTurns,
}

/// Oracle-facing surface for a persistent permission tied to cards exiled by
/// the granting source. Runtime identity is carried by `SOURCE_EXILED_TAG`;
/// this value only preserves the authored source noun and plural spell/pool
/// wording.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceExiledGrantSurface {
    pub source: SourceReferenceSurface,
    pub plural_spell_subject: bool,
    pub generic_card_pool: bool,
    pub generic_cast_this_way_subject: bool,
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

    pub fn graveyard_cast_from_cards_mana_cost(
        additional_costs: Vec<C>,
        exiles_after_resolution: bool,
    ) -> Self {
        Self::GraveyardCastFromCardManaCost {
            additional_costs,
            usage_limit: None,
            condition: None,
            exiles_after_resolution,
        }
    }

    pub fn life_equal_mana_value_from_hand(usage_limit: Option<GrantUsageLimit>) -> Self {
        Self::LifeEqualManaValueFromHand { usage_limit }
    }

    pub fn life_equal_mana_value_from_zone(
        zone: Zone,
        usage_limit: Option<GrantUsageLimit>,
    ) -> Self {
        Self::LifeEqualManaValueFromZone { zone, usage_limit }
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "grant payloads preserve shared static-ability values inline"
)]
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

    pub fn try_map<SA2, E2, C2, Error>(
        self,
        mut map_static: impl FnMut(SA) -> Result<SA2, Error>,
        mut map_effect: impl FnMut(E) -> Result<E2, Error>,
        mut map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<Grantable<SA2, E2, C2, Cond>, Error>
    where
        E2: Clone,
        C: Clone,
        C2: CostComponent,
    {
        Ok(match self {
            Self::Ability(ability) => Grantable::Ability(map_static(ability)?),
            Self::AlternativeCast(method) => {
                Grantable::AlternativeCast(method.try_map(&mut map_effect, &mut map_cost)?)
            }
            Self::DerivedAlternativeCast(spec) => {
                Grantable::DerivedAlternativeCast(spec.try_map(&mut map_cost)?)
            }
            Self::PlayFrom => Grantable::PlayFrom,
        })
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

    pub fn life_equal_mana_value_from_zone(
        zone: Zone,
        usage_limit: Option<GrantUsageLimit>,
    ) -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::life_equal_mana_value_from_zone(
            zone,
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

    pub fn graveyard_cast_from_cards_mana_cost(
        additional_costs: Vec<C>,
        exiles_after_resolution: bool,
    ) -> Self {
        Self::DerivedAlternativeCast(DerivedAlternativeCast::graveyard_cast_from_cards_mana_cost(
            additional_costs,
            exiles_after_resolution,
        ))
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// How often this permission may be used from the same source.
    pub usage_limit: Option<GrantUsageLimit>,
    /// Static abilities granted to a spell as it is cast using this permission.
    pub cast_this_way_grants: Vec<SA>,
    /// An optional narrower filter for the spell that receives
    /// `cast_this_way_grants`. The permission itself continues to use
    /// `filter`, which matters for permissions that include lands or
    /// noncreature spells but only modify creature spells cast this way.
    pub cast_this_way_filter: Option<ObjectFilter>,
    /// Presentation metadata for a persistent source-linked exile grant.
    pub source_exiled_surface: Option<SourceExiledGrantSurface>,
}

impl<SA, E, C, Cond> GrantSpec<SA, E, C, Cond> {
    /// Create a new grant specification.
    pub fn new(grantable: Grantable<SA, E, C, Cond>, filter: ObjectFilter, zone: Zone) -> Self {
        Self {
            grantable,
            filter,
            zone,
            beneficiary: PlayerFilter::You,
            usage_limit: None,
            cast_this_way_grants: Vec::new(),
            cast_this_way_filter: None,
            source_exiled_surface: None,
        }
    }

    pub fn try_map<SA2, E2, C2, Error>(
        self,
        mut map_static: impl FnMut(SA) -> Result<SA2, Error>,
        mut map_effect: impl FnMut(E) -> Result<E2, Error>,
        mut map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<GrantSpec<SA2, E2, C2, Cond>, Error>
    where
        E2: Clone,
        C: Clone,
        C2: CostComponent,
    {
        Ok(GrantSpec {
            grantable: self
                .grantable
                .try_map(&mut map_static, &mut map_effect, &mut map_cost)?,
            filter: self.filter,
            zone: self.zone,
            beneficiary: self.beneficiary,
            usage_limit: self.usage_limit,
            cast_this_way_grants: self
                .cast_this_way_grants
                .into_iter()
                .map(&mut map_static)
                .collect::<Result<Vec<_>, _>>()?,
            cast_this_way_filter: self.cast_this_way_filter,
            source_exiled_surface: self.source_exiled_surface,
        })
    }

    /// Return a copy of this grant specification with an explicit beneficiary.
    pub fn with_beneficiary(mut self, beneficiary: PlayerFilter) -> Self {
        self.beneficiary = beneficiary;
        self
    }

    pub fn with_usage_limit(mut self, usage_limit: GrantUsageLimit) -> Self {
        self.usage_limit = Some(usage_limit);
        self
    }

    pub fn with_cast_this_way_grant(mut self, ability: SA) -> Self {
        self.cast_this_way_grants.push(ability);
        self
    }

    pub fn with_cast_this_way_filter(mut self, filter: ObjectFilter) -> Self {
        self.cast_this_way_filter = Some(filter);
        self
    }

    pub fn with_source_exiled_surface(mut self, surface: SourceExiledGrantSurface) -> Self {
        self.source_exiled_surface = Some(surface);
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
            usage_limit: None,
            cast_this_way_grants: Vec::new(),
            cast_this_way_filter: None,
            source_exiled_surface: None,
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
            usage_limit: None,
            cast_this_way_grants: Vec::new(),
            cast_this_way_filter: None,
            source_exiled_surface: None,
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
                Zone::Ante => "ante",
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

        fn list_card_types_and_or(types: &[CardType]) -> String {
            let names = types
                .iter()
                .map(|card_type| card_type.to_string().to_ascii_lowercase())
                .collect::<Vec<_>>();
            match names.as_slice() {
                [] => String::new(),
                [one] => one.clone(),
                [left, right] => format!("{left} and/or {right}"),
                _ => {
                    let Some((last, rest)) = names.split_last() else {
                        return String::new();
                    };
                    format!("{}, and/or {last}", rest.join(", "))
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
            if *filter == ObjectFilter::noncreature_spell() {
                return "noncreature spells".to_string();
            }
            if filter.subtypes.as_slice() == [crate::Subtype::Aura]
                && filter.ability_markers.len() == 1
                && filter.ability_markers[0].eq_ignore_ascii_case("enchant creature")
            {
                let mut normalized = filter.clone();
                normalized.subtypes.clear();
                normalized.ability_markers.clear();
                if normalized == ObjectFilter::default() {
                    return "Aura spells with enchant creature".to_string();
                }
            }
            if filter.excluded_card_types.contains(&CardType::Land) {
                let mut spell_filter = filter.clone();
                spell_filter
                    .excluded_card_types
                    .retain(|card_type| *card_type != CardType::Land);
                return castable_filter_description(&spell_filter);
            }
            if filter.card_types.as_slice() == [CardType::Creature]
                && let Some(crate::filter_model::Comparison::GreaterThanOrEqual(power)) =
                    filter.power
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
            if description.contains("card exiled with this permanent")
                && description.contains(" card")
            {
                description.replacen(" card", " spell", 1)
            } else if description.contains("permanent") {
                description.replace("permanent", "spell")
            } else if description.contains("spell") {
                description
            } else if description.contains(" card") {
                description.replacen(" card", " spell", 1)
            } else {
                format!("{description} spells")
            }
        }

        fn pluralize_castable_spell_subject(subject: String) -> String {
            if subject.ends_with(" spells") || subject.contains(" spells ") {
                return subject;
            }
            let subject = subject
                .strip_prefix("an ")
                .or_else(|| subject.strip_prefix("a "))
                .unwrap_or(&subject)
                .to_string();
            if let Some(rest) = subject.strip_prefix("spell") {
                return format!("spells{rest}");
            }
            if let Some((head, tail)) = subject.split_once(" spell") {
                return format!("{head} spells{tail}");
            }
            subject
        }

        fn unlimited_zone_castable_filter_description(filter: &ObjectFilter) -> String {
            if !filter.any_of.is_empty() {
                return filter
                    .any_of
                    .iter()
                    .map(unlimited_zone_castable_filter_description)
                    .collect::<Vec<_>>()
                    .join(" or ");
            }
            pluralize_castable_spell_subject(castable_filter_description(filter))
        }

        fn filter_can_include_lands(filter: &ObjectFilter) -> bool {
            if filter.excluded_card_types.contains(&CardType::Land) {
                return false;
            }
            if !filter.card_types.is_empty() && !filter.card_types.contains(&CardType::Land) {
                return false;
            }
            if !filter.all_card_types.is_empty() && !filter.all_card_types.contains(&CardType::Land)
            {
                return false;
            }
            if !filter.any_of.is_empty() {
                return filter.any_of.iter().any(filter_can_include_lands);
            }
            true
        }

        fn cast_this_way_spell_subject(filter: &ObjectFilter) -> String {
            let cast_desc = castable_filter_description(filter);
            if let Some(base) = cast_desc.strip_suffix(" spells") {
                return format!("a {base} spell");
            }
            for pt_clause in [" spells with power ", " spells with toughness "] {
                if let Some((base, _)) = cast_desc.split_once(pt_clause) {
                    return format!("a {base} spell");
                }
            }
            if let Some((base, tail)) = cast_desc.split_once(" spells ") {
                return format!("a {base} spell {tail}");
            }
            if cast_desc.starts_with("a ") || cast_desc.starts_with("an ") {
                return cast_desc;
            }
            "a spell".to_string()
        }

        fn cast_this_way_entered_object_subject(filter: &ObjectFilter) -> Option<String> {
            if filter.card_types.len() != 1 {
                return None;
            }
            match filter.card_types[0] {
                CardType::Artifact
                | CardType::Creature
                | CardType::Enchantment
                | CardType::Planeswalker
                | CardType::Battle => Some(filter.card_types[0].to_string().to_ascii_lowercase()),
                _ => None,
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
                return format!("sacrificing {filter_text} in addition to paying its other costs");
            }

            if let [cost] = additional_costs
                && let Some((count, card_types)) = cost.exile_from_graveyard_details()
            {
                let count_text = if count == 1 {
                    "a".to_string()
                } else {
                    crate::cardinal_word(count).unwrap_or_else(|| count.to_string())
                };
                let type_text = list_card_types_and_or(card_types);
                let type_prefix = if type_text.is_empty() {
                    String::new()
                } else {
                    format!("{type_text} ")
                };
                let card_word = if count == 1 { "card" } else { "cards" };
                return format!(
                    "exiling {count_text} {type_prefix}{card_word} from your graveyard in addition to paying its other costs"
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
                PlayerFilter::PlayerToYourLeft => "The player to your left may".to_string(),
                PlayerFilter::PlayerToYourRight => "The player to your right may".to_string(),
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
                PlayerFilter::AttackedBySourceThisTurn => {
                    "A player this creature attacked this turn may".to_string()
                }
                PlayerFilter::WasDealtDamageBySourceThisGame { .. } => {
                    "A player this source has dealt damage to this game may".to_string()
                }
                PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. } => {
                    "A player dealt combat damage this game by a matching source may".to_string()
                }
                PlayerFilter::LostLifeThisTurn { .. } => {
                    "A player who lost life this turn may".to_string()
                }
                PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. } => {
                    "That player may".to_string()
                }
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => "That player may".to_string(),
                PlayerFilter::HasMoreLifeThanYou { .. } => "That player may".to_string(),
                PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => {
                    "That player may".to_string()
                }
                PlayerFilter::ControlsMost { .. } => "That player may".to_string(),
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
                PlayerFilter::AliasedTarget(_) => "That player may".to_string(),
                PlayerFilter::ControllerOf(_) => "That object's controller may".to_string(),
                PlayerFilter::OwnerOf(_) => "That object's owner may".to_string(),
            }
        }

        fn source_exiled_cast_subject(
            filter: &ObjectFilter,
            beneficiary: &PlayerFilter,
            surface: Option<&SourceExiledGrantSurface>,
        ) -> Option<String> {
            if filter.zone != Some(Zone::Exile) {
                return None;
            }

            let mut normalized = filter.clone();
            normalized.zone = None;
            let before_constraints = normalized.tagged_constraints.len();
            normalized.tagged_constraints.retain(|constraint| {
                !(constraint.tag.as_str() == crate::SOURCE_EXILED_TAG
                    && constraint.relation
                        == crate::filter_model::TaggedOpbjectRelation::IsTaggedObject)
            });
            if normalized.tagged_constraints.len() == before_constraints
                || !normalized.tagged_constraints.is_empty()
            {
                return None;
            }

            let owner_words = match normalized.owner.take() {
                None => String::new(),
                Some(PlayerFilter::NotYou) if matches!(beneficiary, PlayerFilter::Any) => {
                    "they don't own ".to_string()
                }
                Some(PlayerFilter::NotYou) => "you don't own ".to_string(),
                Some(PlayerFilter::You) => "you own ".to_string(),
                Some(_) => return None,
            };

            let (spell_subject, mut card_subject) = if surface
                .is_some_and(|surface| surface.plural_spell_subject)
            {
                (
                    unlimited_zone_castable_filter_description(&normalized),
                    normalized.description().replace("permanent", "card"),
                )
            } else if normalized == ObjectFilter::default() {
                ("a spell".to_string(), "cards".to_string())
            } else if is_simple_card_type_filter(&normalized) && normalized.card_types.len() == 1 {
                let type_text = normalized.card_types[0].to_string().to_ascii_lowercase();
                (
                    format!("{} {type_text} spell", article_for(type_text.as_str())),
                    format!("{type_text} cards"),
                )
            } else if normalized.excluded_card_types.as_slice() == [CardType::Land] && {
                let mut without_land = normalized.clone();
                without_land.excluded_card_types.clear();
                without_land == ObjectFilter::default()
            } {
                ("a nonland spell".to_string(), "nonland cards".to_string())
            } else {
                return None;
            };
            if surface.is_some_and(|surface| surface.generic_card_pool) {
                card_subject = "cards".to_string();
            }
            let article = if owner_words.is_empty()
                || surface.is_some_and(|surface| surface.generic_card_pool)
            {
                ""
            } else {
                "the "
            };
            let source = surface
                .map(|surface| surface.source.display_text())
                .unwrap_or_else(|| "this permanent".to_string());
            Some(format!(
                "{spell_subject} from among {article}{card_subject} {owner_words}exiled with {source}"
            ))
        }

        fn simple_ability_marker_spell_subject(filter: &ObjectFilter) -> Option<String> {
            let [marker] = filter.ability_markers.as_slice() else {
                return None;
            };
            let mut normalized = filter.clone();
            normalized.ability_markers.clear();
            normalized
                .excluded_card_types
                .retain(|card_type| *card_type != CardType::Land);
            if normalized != ObjectFilter::default() {
                return None;
            }
            let marker = marker.to_ascii_lowercase();
            Some(format!(
                "spells that have {} {marker} ability",
                article_for(&marker)
            ))
        }

        fn countered_exile_filter_facts(
            filter: &ObjectFilter,
        ) -> Option<(crate::CounterType, Option<PlayerFilter>, bool, ObjectFilter)> {
            let crate::filter_model::CounterConstraint::Typed(counter_type) = filter.with_counter?
            else {
                return None;
            };
            if filter.zone != Some(Zone::Exile) {
                return None;
            }
            let source_linked = filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == crate::SOURCE_EXILED_TAG
                    && constraint.relation
                        == crate::filter_model::TaggedOpbjectRelation::IsTaggedObject
            });
            let mut normalized = filter.clone();
            let owner = normalized.owner.take();
            normalized.zone = None;
            normalized.with_counter = None;
            normalized.tagged_constraints.retain(|constraint| {
                !(constraint.tag.as_str() == crate::SOURCE_EXILED_TAG
                    && constraint.relation
                        == crate::filter_model::TaggedOpbjectRelation::IsTaggedObject)
            });
            if !normalized.tagged_constraints.is_empty() {
                return None;
            }
            Some((counter_type, owner, source_linked, normalized))
        }

        fn countered_exile_play_permission(
            filter: &ObjectFilter,
            beneficiary: &PlayerFilter,
            may_prefix: &str,
        ) -> Option<String> {
            let owner_clause = |owner: Option<&PlayerFilter>| match owner {
                Some(PlayerFilter::Opponent) => Some("your opponents own"),
                Some(PlayerFilter::You) => Some("you own"),
                Some(PlayerFilter::NotYou) => Some("you don't own"),
                None => Some(""),
                _ => None,
            };
            let counter_clause = |counter_type: crate::CounterType| {
                format!("{} counters on them", counter_type.description())
            };

            if filter.any_of.is_empty() {
                let (counter_type, owner, source_linked, mut normalized) =
                    countered_exile_filter_facts(filter)?;
                if source_linked {
                    return None;
                }
                let excluded_lands = normalized.excluded_card_types.contains(&CardType::Land);
                normalized
                    .excluded_card_types
                    .retain(|card_type| *card_type != CardType::Land);
                if normalized != ObjectFilter::default() {
                    return None;
                }
                let owner_clause = owner_clause(owner.as_ref())?;
                if !excluded_lands {
                    // Lands stay playable, so the authored surface says
                    // "play cards": "you may play cards you don't own with
                    // stash counters on them from exile" (Tinybones).
                    if owner_clause.is_empty() {
                        return None;
                    }
                    return Some(format!(
                        "{may_prefix} play cards {owner_clause} with {} from exile",
                        counter_clause(counter_type)
                    ));
                }
                let owner_clause = if owner_clause.is_empty() {
                    String::new()
                } else {
                    format!(" {owner_clause}")
                };
                return Some(format!(
                    "{may_prefix} cast spells from among cards in exile{owner_clause} with {}",
                    counter_clause(counter_type)
                ));
            }

            if filter.any_of.len() != 2 {
                return None;
            }
            let mut outer = filter.clone();
            outer.any_of.clear();
            if outer != ObjectFilter::default() {
                return None;
            }
            let first = countered_exile_filter_facts(&filter.any_of[0])?;
            let second = countered_exile_filter_facts(&filter.any_of[1])?;
            if first.0 != second.0 || first.1 != second.1 || first.2 != second.2 {
                return None;
            }
            // Unlinked lands-plus-spells over an owned countered exile pool:
            // "play lands and cast spells from among cards you own in exile
            // with <type> counters on them" (Grolnok, the Omnivore).
            if !first.2 {
                let (land_branch, spell_branch) =
                    if first.3.card_types.as_slice() == [CardType::Land] {
                        (&first.3, &second.3)
                    } else if second.3.card_types.as_slice() == [CardType::Land] {
                        (&second.3, &first.3)
                    } else {
                        return None;
                    };
                let mut normalized_spells = spell_branch.clone();
                normalized_spells
                    .excluded_card_types
                    .retain(|card_type| *card_type != CardType::Land);
                if land_branch != &ObjectFilter::default().with_type(CardType::Land)
                    || normalized_spells != ObjectFilter::default()
                {
                    return None;
                }
                let owner_clause = owner_clause(first.1.as_ref())?;
                let owner_clause = if owner_clause.is_empty() {
                    String::new()
                } else {
                    format!(" {owner_clause}")
                };
                return Some(format!(
                    "{may_prefix} play lands and cast spells from among cards{owner_clause} in exile with {}",
                    counter_clause(first.0)
                ));
            }
            let (land_branch, spell_branch) = if first.3.card_types.as_slice() == [CardType::Land] {
                (&first.3, &second.3)
            } else if second.3.card_types.as_slice() == [CardType::Land] {
                (&second.3, &first.3)
            } else {
                return None;
            };
            if land_branch != &ObjectFilter::default().with_type(CardType::Land)
                || filter_can_include_lands(spell_branch)
            {
                return None;
            }
            let card_pool = if matches!(beneficiary, PlayerFilter::You) {
                "cards you exiled"
            } else {
                "cards exiled with this permanent"
            };
            Some(format!(
                "{may_prefix} play lands and cast {} from among {card_pool} that have {}",
                unlimited_zone_castable_filter_description(spell_branch),
                counter_clause(first.0)
            ))
        }

        let mut filter = self.filter.clone();
        filter.zone.get_or_insert(self.zone);
        let filter_desc = filter.description();
        let mut may_prefix = beneficiary_may_prefix(&self.beneficiary);
        if matches!(self.usage_limit, Some(GrantUsageLimit::OnceEachTurn))
            && let Some(rest) = may_prefix.strip_prefix("You may")
        {
            may_prefix = format!("Once each turn, you may{rest}");
        } else if matches!(
            self.usage_limit,
            Some(GrantUsageLimit::OnceDuringEachOfYourTurns)
        ) && let Some(rest) = may_prefix.strip_prefix("You may")
        {
            may_prefix = format!("Once during each of your turns, you may{rest}");
        }
        let cast_this_way_suffix = || {
            if self.cast_this_way_grants.is_empty() {
                return String::new();
            }
            let grants = self
                .cast_this_way_grants
                .iter()
                .map(GrantStaticAbility::grant_display)
                .collect::<Vec<_>>();
            let cast_filter = self.cast_this_way_filter.as_ref().unwrap_or(&self.filter);
            let entered_object_filter = if self
                .source_exiled_surface
                .as_ref()
                .is_some_and(|surface| surface.generic_cast_this_way_subject)
            {
                // A generic authored rider such as "If you cast a spell this
                // way, that creature ..." inherits its permanent kind from
                // the permission's subject, not from the word `spell`.
                &self.filter
            } else {
                cast_filter
            };
            let cast_spell_text = || {
                if self
                    .source_exiled_surface
                    .as_ref()
                    .is_some_and(|surface| surface.generic_cast_this_way_subject)
                {
                    "a spell".to_string()
                } else {
                    cast_this_way_spell_subject(cast_filter)
                }
            };
            if grants.len() == 1 && grants[0].eq_ignore_ascii_case("haste") {
                let spell_text = cast_spell_text();
                return format!(
                    ". If you cast {spell_text} this way, it gains haste until end of turn"
                );
            }
            if grants.len() == 1
                && matches!(
                    grants[0].to_ascii_lowercase().as_str(),
                    "enters tapped" | "this enters tapped"
                )
            {
                if self.filter == ObjectFilter::source() {
                    return ". If you do, it enters tapped".to_string();
                }
                let spell_text = cast_spell_text();
                if let Some(subject) = cast_this_way_entered_object_subject(entered_object_filter) {
                    return format!(
                        ". If you cast {spell_text} this way, that {subject} enters tapped"
                    );
                }
                return format!(". If you cast {spell_text} this way, it enters tapped");
            }
            if grants.len() == 1
                && let Some(rest) = grants[0]
                    .strip_prefix("Enters the battlefield with ")
                    .or_else(|| grants[0].strip_prefix("enters the battlefield with "))
            {
                let spell_text = cast_spell_text();
                if self.filter == ObjectFilter::source() {
                    return format!(". If you do, it enters with {rest}");
                }
                if let Some(subject) = cast_this_way_entered_object_subject(entered_object_filter) {
                    return format!(
                        ". If you cast {spell_text} this way, that {subject} enters with {rest}"
                    );
                }
                return format!(". If you cast {spell_text} this way, it enters with {rest}");
            }
            format!(". Spells cast this way gain {}", grants.join(" and "))
        };

        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Graveyard
            && self.filter == ObjectFilter::source()
        {
            return format!(
                "{may_prefix} cast this card from your graveyard{}",
                cast_this_way_suffix()
            );
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Exile
            && self.filter == ObjectFilter::source()
        {
            return format!(
                "{may_prefix} cast this card from exile{}",
                cast_this_way_suffix()
            );
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Exile
            && let Some(cast_subject) = source_exiled_cast_subject(
                &self.filter,
                &self.beneficiary,
                self.source_exiled_surface.as_ref(),
            )
        {
            let prefix = if matches!(self.beneficiary, PlayerFilter::Any)
                && matches!(self.filter.owner, Some(PlayerFilter::NotYou))
            {
                "During each player's turn, that player may".to_string()
            } else {
                may_prefix.clone()
            };
            return format!("{prefix} cast {cast_subject}{}", cast_this_way_suffix());
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Graveyard
            && self.filter.card_types.as_slice() == [CardType::Land]
        {
            return format!(
                "{may_prefix} play lands from your graveyard{}",
                cast_this_way_suffix()
            );
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Graveyard
            && self.filter == ObjectFilter::default()
        {
            return format!(
                "{may_prefix} play lands and cast spells from your graveyard{}",
                cast_this_way_suffix()
            );
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Graveyard
            && self.filter.surveilled_this_turn
        {
            return format!(
                "{may_prefix} play lands and cast spells from among cards in your graveyard you've surveilled this turn"
            );
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Graveyard
            && !filter_can_include_lands(&self.filter)
        {
            let spell_subject = simple_ability_marker_spell_subject(&self.filter)
                .unwrap_or_else(|| unlimited_zone_castable_filter_description(&self.filter));
            return format!(
                "{may_prefix} cast {spell_subject} from your graveyard{}",
                cast_this_way_suffix()
            );
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Exile
            && let Some(permission) =
                countered_exile_play_permission(&self.filter, &self.beneficiary, &may_prefix)
        {
            return format!("{permission}{}", cast_this_way_suffix());
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Library
            && self.filter == ObjectFilter::default()
        {
            return format!(
                "{may_prefix} play lands and cast spells from the top of your library{}",
                cast_this_way_suffix()
            );
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Library
            && self.filter.card_types.as_slice() == [CardType::Land]
        {
            return format!(
                "{may_prefix} play lands from the top of your library{}",
                cast_this_way_suffix()
            );
        }
        if matches!(self.grantable, Grantable::PlayFrom)
            && self.zone == Zone::Library
            && matches!(self.usage_limit, Some(GrantUsageLimit::OnceEachTurn))
            && self.filter.excluded_card_types.as_slice() == [CardType::Land]
            && self.filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == crate::SOURCE_EXILED_TAG
                    && constraint.relation
                        == crate::filter_model::TaggedOpbjectRelation::SharesCardType
            })
        {
            return format!(
                "{may_prefix} cast a spell from the top of your library if it shares a card type with a card exiled with this permanent{}",
                cast_this_way_suffix()
            );
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
                    "{may_prefix} play lands and cast {} from the top of your library{}",
                    if self.usage_limit.is_none() {
                        unlimited_zone_castable_filter_description(other)
                    } else {
                        castable_filter_description(other)
                    },
                    cast_this_way_suffix()
                );
            }
        }
        if matches!(self.grantable, Grantable::PlayFrom) && self.zone == Zone::Library {
            if filter_can_include_lands(&self.filter) {
                return format!(
                    "{may_prefix} play {} from the top of your library",
                    filter_desc
                ) + &cast_this_way_suffix();
            }
            return format!(
                "{may_prefix} cast {} from the top of your library",
                if self.usage_limit.is_none() {
                    unlimited_zone_castable_filter_description(&self.filter)
                } else {
                    castable_filter_description(&self.filter)
                }
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
                "Each {base} card in {owner_phrase} hand has miracle. Its miracle cost is equal to its mana cost reduced by {{{reduction}}}"
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
            DerivedAlternativeCast::LifeEqualManaValueFromZone { zone, usage_limit },
        ) = &self.grantable
            && self.zone == *zone
        {
            let prefix = if matches!(
                usage_limit,
                Some(GrantUsageLimit::OnceDuringEachOfYourTurns)
            ) {
                "Once during each of your turns, "
            } else {
                ""
            };
            if *zone == Zone::Graveyard && self.filter.surveilled_this_turn {
                return format!(
                    "{prefix}If you cast a spell this way, you pay life equal to its mana value rather than paying its mana cost"
                );
            }
            let filter_desc = castable_filter_description(&self.filter);
            return format!(
                "{prefix}{} cast {filter_desc} by paying life equal to its mana value rather than paying its mana cost",
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
                if !additional_costs.is_empty() {
                    line.push_str(" by ");
                    line.push_str(&cost_text);
                }
                line.push_str(&cast_this_way_suffix());
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
            let mut line = if additional_costs.is_empty() {
                format!(
                    "{prefix}{} cast {} from your graveyard",
                    may_prefix.to_ascii_lowercase(),
                    filter_desc
                )
            } else {
                format!(
                    "{prefix}{} cast {} from your graveyard by {}",
                    may_prefix.to_ascii_lowercase(),
                    filter_desc,
                    cost_text
                )
            };
            line.push_str(&cast_this_way_suffix());
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
        ThisSpellCostCondition::ConditionExpr { display, .. }
        | ThisSpellCostCondition::AsLongAsConditionExpr { display, .. } => Some(display.clone()),
        ThisSpellCostCondition::YourTurn => Some("it's your turn".to_string()),
        ThisSpellCostCondition::NotYourTurn => Some("it isn't your turn".to_string()),
        _ => Some("the condition is true".to_string()),
    }
}
