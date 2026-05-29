//! Unified grant system for granting abilities and alternative casting methods.
//!
//! This module provides a unified way to grant:
//! - Static abilities (flash, flying, hexproof, etc.)
//! - Alternative casting methods (flashback, escape, etc.)
//!
//! Grants can be applied through:
//! - Static abilities on permanents (while the source is on the battlefield)
//! - One-shot effects from resolving spells/abilities (with a duration like "until end of turn")
//!
//! # Example
//!
//! ```ignore
//! // Grant flash to noncreature spells in hand (Valley Floodcaller)
//! StaticAbility::grants(GrantSpec {
//!     grantable: Grantable::Ability(StaticAbility::flash()),
//!     filter: ObjectFilter::noncreature_spell(),
//!     zone: Zone::Hand,
//! })
//!
//! // Grant escape to nonland cards in graveyard (Underworld Breach)
//! StaticAbility::grants(GrantSpec {
//!     grantable: Grantable::escape(3),
//!     filter: ObjectFilter::nonland(),
//!     zone: Zone::Graveyard,
//! })
//!
//! // Grant flashback until end of turn (Snapcaster Mage)
//! Effect::grant(
//!     Grantable::flashback_from_cards_mana_cost(),
//!     target,
//!     GrantDuration::UntilEndOfTurn,
//! )
//! ```

use crate::alternative_cast::AlternativeCastingMethod;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::object::Object;
use crate::static_abilities::StaticAbility;
use crate::types::CardType;
use crate::zone::Zone;

pub type DerivedAlternativeCast = ironsmith_core::DerivedAlternativeCast<Cost>;
pub type Grantable = ironsmith_core::Grantable<
    StaticAbility,
    crate::effect::Effect,
    Cost,
    crate::static_abilities::ThisSpellCostCondition,
>;
pub type GrantSpec = ironsmith_core::GrantSpec<
    StaticAbility,
    crate::effect::Effect,
    Cost,
    crate::static_abilities::ThisSpellCostCondition,
>;
pub use ironsmith_core::{GrantDuration, GrantUsageLimit};

impl ironsmith_core::GrantStaticAbility for StaticAbility {
    fn grant_flash() -> Self {
        Self::flash()
    }

    fn grant_display(&self) -> String {
        self.display()
    }

    fn grant_has_flash(&self) -> bool {
        self.has_flash()
    }
}

pub trait DerivedAlternativeCastRuntimeExt {
    fn materialize_for(&self, card: &Object) -> Option<AlternativeCastingMethod>;
}

impl DerivedAlternativeCastRuntimeExt for DerivedAlternativeCast {
    fn materialize_for(&self, card: &Object) -> Option<AlternativeCastingMethod> {
        match self {
            Self::FlashbackFromCardManaCost { additional_costs } => {
                let mana_cost = card.mana_cost.clone()?;
                if !card.has_card_type(CardType::Instant) && !card.has_card_type(CardType::Sorcery)
                {
                    return None;
                }
                if card.zone != Zone::Graveyard {
                    return None;
                }

                let mut costs = vec![Cost::mana(mana_cost)];
                costs.extend(additional_costs.iter().cloned());
                Some(AlternativeCastingMethod::Flashback {
                    total_cost: TotalCost::from_costs(costs),
                })
            }
            Self::BlitzFromCardManaCost => {
                let mana_cost = card.mana_cost.clone()?;
                Some(AlternativeCastingMethod::Blitz {
                    total_cost: TotalCost::mana(mana_cost),
                })
            }
            Self::RetraceFromCardManaCost => {
                let mana_cost = card.mana_cost.clone()?;
                if card.zone != Zone::Graveyard {
                    return None;
                }
                Some(AlternativeCastingMethod::Retrace {
                    total_cost: TotalCost::from_costs(vec![
                        Cost::mana(mana_cost),
                        Cost::discard(1, Some(CardType::Land)),
                    ]),
                })
            }
            Self::EmergeFromCardManaCost => {
                let mana_cost = card.mana_cost.clone()?;
                if card.zone != Zone::Hand || !card.has_card_type(CardType::Creature) {
                    return None;
                }
                Some(AlternativeCastingMethod::alternative_cost(
                    "Emerge",
                    Some(mana_cost),
                    vec![Cost::sacrifice(
                        crate::target::ObjectFilter::creature().you_control(),
                    )],
                ))
            }
            Self::MiracleFromCardManaCostReducedBy { reduction } => {
                let mana_cost = card.mana_cost.clone()?;
                if card.zone != Zone::Hand {
                    return None;
                }
                Some(AlternativeCastingMethod::Miracle {
                    cost: mana_cost.reduce_generic(*reduction),
                })
            }
            Self::EscapeFromCardManaCost { exile_count } => {
                let mana_cost = card.mana_cost.clone()?;
                if card.zone != Zone::Graveyard {
                    return None;
                }
                Some(AlternativeCastingMethod::Escape {
                    cost: Some(mana_cost),
                    exile_count: *exile_count,
                })
            }
            Self::ManaValueAsGenericFromHand => {
                if card.zone != Zone::Hand {
                    return None;
                }
                let mana_value =
                    u8::try_from(card.mana_cost.as_ref().map_or(0, |c| c.mana_value())).ok()?;
                Some(AlternativeCastingMethod::alternative_cost(
                    "Pay mana value",
                    Some(crate::mana::ManaCost::from_symbols(vec![
                        crate::mana::ManaSymbol::Generic(mana_value),
                    ])),
                    Vec::new(),
                ))
            }
            Self::LifeEqualManaValueFromHand { .. } => {
                if card.zone != Zone::Hand {
                    return None;
                }
                Some(AlternativeCastingMethod::alternative_cost(
                    "Pay life equal to mana value",
                    None,
                    vec![
                        Cost::try_from_runtime_effect(crate::effect::Effect::new(
                            crate::effects::LoseLifeEffect::you(crate::effect::Value::ManaValueOf(
                                Box::new(crate::target::ChooseSpec::Source),
                            )),
                        ))
                        .expect("mana-value life payment should be cost-capable"),
                    ],
                ))
            }
            Self::GraveyardCastFromCardManaCost {
                additional_costs,
                usage_limit,
                condition,
                exiles_after_resolution,
            } => {
                let mana_cost = card.mana_cost.clone()?;
                if card.zone != Zone::Graveyard {
                    return None;
                }

                let mut costs = vec![Cost::mana(mana_cost)];
                costs.extend(additional_costs.iter().cloned());
                Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
                    "Cast from graveyard",
                    Zone::Graveyard,
                    TotalCost::from_costs(costs),
                    condition.clone().or_else(|| {
                        matches!(
                            usage_limit,
                            Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns)
                        )
                        .then_some(crate::static_abilities::ThisSpellCostCondition::YourTurn)
                    }),
                    *exiles_after_resolution,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::ThisSpellCostCondition;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;

    fn graveyard_card_object() -> Object {
        let card = CardBuilder::new(CardId::from_raw(91_500), "Graveyard Probe")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Instant])
            .build();
        Object::from_card(
            ObjectId::from_raw(91_500),
            &card,
            PlayerId::from_index(0),
            Zone::Graveyard,
        )
    }

    #[test]
    fn test_grantable_display() {
        let flash = Grantable::Ability(StaticAbility::flash());
        assert_eq!(flash.display(), "Flash");

        let flashback = Grantable::flashback_from_cards_mana_cost();
        assert_eq!(flashback.display(), "flashback");

        let escape = Grantable::escape(3);
        assert_eq!(escape.display(), "Escape");

        let mana_value = Grantable::mana_value_as_generic_from_hand();
        assert_eq!(mana_value.display(), "Pay mana value");

        let emerge = Grantable::emerge_from_cards_mana_cost();
        assert_eq!(emerge.display(), "Emerge");
    }

    #[test]
    fn test_grant_spec_flash_to_noncreature_spells() {
        let spec = GrantSpec::flash_to_noncreature_spells();
        assert_eq!(spec.zone, Zone::Hand);
        assert!(matches!(spec.grantable, Grantable::Ability(_)));
        assert!(
            spec.filter
                .excluded_card_types
                .contains(&CardType::Creature)
        );
        assert_eq!(
            spec.display(),
            "You may cast noncreature spells as though they had flash"
        );
    }

    #[test]
    fn test_grant_spec_flash_to_spells() {
        let spec = GrantSpec::flash_to_spells();
        assert_eq!(spec.zone, Zone::Hand);
        assert!(matches!(spec.grantable, Grantable::Ability(_)));
        assert!(spec.filter.excluded_card_types.contains(&CardType::Land));
        assert_eq!(
            spec.display(),
            "You may cast spells as though they had flash"
        );
    }

    #[test]
    fn test_grant_spec_flash_to_spells_any_player_display() {
        let spec = GrantSpec::flash_to_spells().with_beneficiary(PlayerFilter::Any);
        assert_eq!(
            spec.display(),
            "Any player may cast spells as though they had flash"
        );
    }

    #[test]
    fn test_grant_spec_escape_to_nonland() {
        let spec = GrantSpec::escape_to_nonland(3);
        assert_eq!(spec.zone, Zone::Graveyard);
        assert!(matches!(
            spec.grantable,
            Grantable::DerivedAlternativeCast(DerivedAlternativeCast::EscapeFromCardManaCost {
                exile_count: 3
            })
        ));
        assert!(spec.filter.excluded_card_types.contains(&CardType::Land));
    }

    #[test]
    fn test_grant_spec_play_from_graveyard() {
        let spec = GrantSpec::play_from_graveyard();
        assert_eq!(spec.zone, Zone::Graveyard);
        assert_eq!(
            spec.display(),
            "You may play lands and cast spells from your graveyard"
        );
    }

    #[test]
    fn test_grant_spec_play_lands_from_graveyard_uses_nonbattlefield_land_filter() {
        let spec = GrantSpec::play_lands_from_graveyard();
        assert_eq!(spec.zone, Zone::Graveyard);
        assert_eq!(spec.filter.card_types, vec![CardType::Land]);
        assert_eq!(
            spec.filter.zone, None,
            "grant zone already scopes graveyard land permissions"
        );
        assert_eq!(spec.display(), "You may play lands from your graveyard");
    }

    #[test]
    fn test_once_during_your_turn_graveyard_cast_keeps_turn_condition() {
        let method = DerivedAlternativeCast::GraveyardCastFromCardManaCost {
            additional_costs: Vec::new(),
            usage_limit: Some(GrantUsageLimit::OnceDuringEachOfYourTurns),
            condition: None,
            exiles_after_resolution: false,
        }
        .materialize_for(&graveyard_card_object())
        .expect("graveyard cast grant should produce an alternative method");

        assert!(matches!(
            method.cast_condition(),
            Some(ThisSpellCostCondition::YourTurn)
        ));
        assert!(!method.exiles_after_resolution());
    }

    #[test]
    fn test_explicit_graveyard_cast_condition_and_exile_are_preserved() {
        let method = DerivedAlternativeCast::graveyard_cast_from_cards_mana_cost_with_condition(
            ThisSpellCostCondition::NotYourTurn,
            true,
        )
        .materialize_for(&graveyard_card_object())
        .expect("conditional graveyard cast grant should produce an alternative method");

        assert!(matches!(
            method.cast_condition(),
            Some(ThisSpellCostCondition::NotYourTurn)
        ));
        assert!(method.exiles_after_resolution());
    }

    #[test]
    fn test_grant_spec_cast_from_hand_without_paying_mana_cost_matching() {
        let spec =
            GrantSpec::cast_from_hand_without_paying_mana_cost_matching(ObjectFilter::nonland());
        assert_eq!(spec.zone, Zone::Hand);
        assert!(matches!(
            &spec.grantable,
            Grantable::AlternativeCast(method)
                if method.cast_from_zone() == Zone::Hand
                    && method.mana_cost().is_none()
                    && method.non_mana_costs().is_empty()
        ));
        assert_eq!(
            spec.display(),
            "You may cast spells from your hand without paying their mana costs"
        );
    }
}
