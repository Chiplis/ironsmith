//! Zone ownership for trigger alternatives, shared by grammar and lowering.
use super::ast::{PredicateAst, SourcePredicateAst, TriggerSpec};
use super::facts::TriggerFunctionalZoneFacts;
use ironsmith_core::tag::TagKeyWalk;
use ironsmith_core::Zone;

fn qualified_zones(condition: &PredicateAst) -> Option<Vec<Zone>> {
    match condition {
        PredicateAst::Source(SourcePredicateAst::SourceIsInZone(zone)) => Some(vec![*zone]),
        PredicateAst::And(left, right) => match (qualified_zones(left), qualified_zones(right)) {
            (Some(a), Some(b)) => Some(a.into_iter().filter(|zone| b.contains(zone)).collect()),
            (a, b) => a.or(b),
        },
        PredicateAst::Or(left, right) => {
            let mut zones = qualified_zones(left)?;
            for zone in qualified_zones(right)? {
                if !zones.contains(&zone) { zones.push(zone); }
            }
            Some(zones)
        }
        _ => None,
    }
}

pub fn base_trigger_functional_zones(trigger: &TriggerSpec) -> Vec<Zone> {
    fn union<'a>(branches: impl Iterator<Item = &'a TriggerSpec>) -> Vec<Zone> {
        let mut zones = Vec::new();
        for branch in branches {
            for zone in base_trigger_functional_zones(branch) {
                if !zones.contains(&zone) { zones.push(zone); }
            }
        }
        zones
    }
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => base_trigger_functional_zones(trigger),
        TriggerSpec::Either(left, right) => union([left.as_ref(), right.as_ref()].into_iter()),
        TriggerSpec::AnyOf(branches) => union(branches.iter()),
        TriggerSpec::ConditionQualified { condition, trigger, .. } => {
            qualified_zones(condition)
                .unwrap_or_else(|| base_trigger_functional_zones(trigger))
        }
        // CR 603.6c: a card's "put into a graveyard from anywhere" trigger
        // is acquired on arrival, including when it came from the battlefield.
        TriggerSpec::PutIntoGraveyard(filter) if filter.source => vec![Zone::Graveyard],
        TriggerSpec::ZoneChange(change)
            if change.to == Some(Zone::Graveyard)
                && change.from.is_none()
                && change.from_zones.is_none()
                && change.from_excluded.is_none()
                && (change.this || change.filter.as_ref().is_some_and(|filter| filter.source)) =>
        {
            vec![Zone::Graveyard]
        }
        // CR 603.10a: a move out of a hidden library doesn't look back, so
        // "When this card is put into your graveyard from your library"
        // (Narcomoeba, Gaea's Blessing) is checked where the card arrived.
        TriggerSpec::ZoneChange(change)
            if change.to == Some(Zone::Graveyard)
                && change.from == Some(Zone::Library)
                && (change.this || change.filter.as_ref().is_some_and(|filter| filter.source)) =>
        {
            vec![Zone::Graveyard]
        }
        // The parser's typed form of the same self move ("When this card is
        // put into your graveyard from your library"): only a departure from
        // the battlefield looks back (CR 603.10a); otherwise the ability
        // functions where the card arrived (CR 603.6d).
        TriggerSpec::PutIntoGraveyardFromZone { filter, from, .. }
            if filter.source && *from != Zone::Battlefield =>
        {
            vec![Zone::Graveyard]
        }
        TriggerSpec::ZoneChange(ironsmith_core::trigger_model::ZoneChangeTrigger {
            this: true, from: Some(origin), ..
        }) => vec![*origin],
        TriggerSpec::YouCastThisSpell => vec![Zone::Stack],
        TriggerSpec::CounterRemovedFrom { filter, .. } if filter.source && filter.zone.is_some() => {
            vec![filter.zone.unwrap()]
        }
        TriggerSpec::KeywordActionFromSource {
            action: ironsmith_core::event_model::KeywordActionKind::Cycle, ..
        } => vec![Zone::Hand],
        // CR 702.170a/c: a card becomes plotted as it is exiled, so "When this
        // card becomes plotted" is observed from exile.
        TriggerSpec::KeywordActionFromSource {
            action: ironsmith_core::event_model::KeywordActionKind::Plot, ..
        } => vec![Zone::Exile],
        _ => vec![Zone::Battlefield],
    }
}

/// Apply whole-ability zone facts after identifying the trigger's event zones.
pub fn derive_triggered_ability_functional_zones_from_facts(
    trigger: &TriggerSpec,
    facts: &TriggerFunctionalZoneFacts,
) -> Vec<Zone> {
    if let TriggerSpec::WithIntro { trigger, .. } = trigger {
        return derive_triggered_ability_functional_zones_from_facts(trigger, facts);
    }
    if !facts.excluded_zones.is_empty() {
        return [
            Zone::Library,
            Zone::Hand,
            Zone::Battlefield,
            Zone::Graveyard,
            Zone::Stack,
            Zone::Exile,
            Zone::Command,
            Zone::Ante,
            Zone::OutsideGame,
        ]
        .into_iter()
        .filter(|zone| !facts.excluded_zones.contains(zone))
        .collect();
    }
    // A self-discard is observed in hand even if the ability returns the card.
    // Other alternatives keep their own zones ("When this creature dies and
    // when you discard this card" -- Bartered Cow -- also looks back from the
    // battlefield).
    if facts.discards_this_card {
        fn self_discard(trigger: &TriggerSpec) -> bool {
            match trigger {
                TriggerSpec::WithIntro { trigger, .. } => self_discard(trigger),
                TriggerSpec::PlayerDiscardsCard { filter, .. } => {
                    filter.as_ref().is_some_and(|filter| filter.source)
                }
                _ => false,
            }
        }
        let branches: Vec<&TriggerSpec> = match trigger {
            TriggerSpec::Either(left, right) => vec![left.as_ref(), right.as_ref()],
            TriggerSpec::AnyOf(branches) => branches.iter().collect(),
            _ => Vec::new(),
        };
        let mut zones = vec![Zone::Hand];
        if branches.iter().any(|branch| self_discard(branch)) {
            for branch in branches.into_iter().filter(|branch| !self_discard(branch)) {
                for zone in base_trigger_functional_zones(branch) {
                    if !zones.contains(&zone) { zones.push(zone); }
                }
            }
        }
        return zones;
    }
    if facts.returns_self_from_graveyard && !trigger_references_attached_object(trigger) {
        return vec![Zone::Graveyard];
    }
    if let Some(zone) = facts.explicit_zone
        && !explicit_zone_belongs_to_branch(trigger, zone)
    {
        let mut zones = vec![zone];
        if facts.explicit_zone_or_battlefield && zone != Zone::Battlefield {
            zones.push(Zone::Battlefield);
        }
        return zones;
    }
    base_trigger_functional_zones(trigger)
}

fn trigger_references_attached_object(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. }
        | TriggerSpec::ConditionQualified { trigger, .. } => {
            trigger_references_attached_object(trigger)
        }
        TriggerSpec::PutIntoGraveyard(filter)
        | TriggerSpec::PutIntoGraveyardOneOrMore(filter)
        | TriggerSpec::PutIntoGraveyardFromZone { filter, .. }
        | TriggerSpec::PutIntoGraveyardFromAnyExcept { filter, .. } => {
            filter.contains_tag_key("enchanted") || filter.contains_tag_key("equipped")
        }
        TriggerSpec::Either(left, right) => {
            trigger_references_attached_object(left) || trigger_references_attached_object(right)
        }
        TriggerSpec::AnyOf(branches) => branches.iter().any(trigger_references_attached_object),
        _ => false,
    }
}

/// A textual zone fact already owned by an alternative's typed qualification
/// must not narrow the whole ability and disable its other alternatives.
pub fn explicit_zone_belongs_to_branch(trigger: &TriggerSpec, zone: Zone) -> bool {
    fn owns(trigger: &TriggerSpec, zone: Zone) -> bool {
        match trigger {
            TriggerSpec::WithIntro { trigger, .. } => owns(trigger, zone),
            TriggerSpec::ConditionQualified { condition, .. } => qualified_zones(condition).is_some_and(|zones| zones.contains(&zone)),
            TriggerSpec::Either(left, right) => owns(left, zone) || owns(right, zone),
            TriggerSpec::AnyOf(branches) => branches.iter().any(|branch| owns(branch, zone)),
            _ => false,
        }
    }
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => explicit_zone_belongs_to_branch(trigger, zone),
        TriggerSpec::Either(left, right) => owns(left, zone) || owns(right, zone),
        TriggerSpec::AnyOf(branches) => branches.iter().any(|branch| owns(branch, zone)),
        _ => false,
    }
}

#[cfg(test)]
mod source_zone_alternative_tests {
    use super::*;

    fn in_zone(zone: Zone) -> PredicateAst {
        PredicateAst::Source(SourcePredicateAst::SourceIsInZone(zone))
    }

    #[test]
    fn source_zone_unions_and_intersections_keep_each_authored_location() {
        let alternatives = PredicateAst::Or(Box::new(in_zone(Zone::Graveyard)), Box::new(in_zone(Zone::Battlefield)));
        assert_eq!(qualified_zones(&alternatives), Some(vec![Zone::Graveyard, Zone::Battlefield]));
        assert_eq!(qualified_zones(&PredicateAst::And(Box::new(alternatives.clone()), Box::new(in_zone(Zone::Battlefield)))), Some(vec![Zone::Battlefield]));
        assert_eq!(qualified_zones(&PredicateAst::And(Box::new(in_zone(Zone::Graveyard)), Box::new(in_zone(Zone::Battlefield)))), Some(vec![]));
        let qualified = TriggerSpec::ConditionQualified {
            trigger: Box::new(TriggerSpec::BeginningOfUpkeep(ironsmith_core::PlayerFilter::You)),
            condition: alternatives,
            surface: String::new(),
        };
        let either = TriggerSpec::Either(Box::new(qualified), Box::new(TriggerSpec::YouCastThisSpell));
        assert_eq!(base_trigger_functional_zones(&either), vec![Zone::Graveyard, Zone::Battlefield, Zone::Stack]);
        assert!(explicit_zone_belongs_to_branch(&either, Zone::Graveyard));
        assert!(explicit_zone_belongs_to_branch(&either, Zone::Battlefield));
    }

    #[test]
    fn excluded_zones_and_self_discard_take_precedence_over_a_graveyard_return() {
        let trigger = TriggerSpec::BeginningOfUpkeep(ironsmith_core::PlayerFilter::You);
        let mut facts = TriggerFunctionalZoneFacts {
            returns_self_from_graveyard: true,
            discards_this_card: true,
            excluded_zones: vec![Zone::Hand, Zone::Battlefield],
            ..Default::default()
        };
        assert_eq!(
            derive_triggered_ability_functional_zones_from_facts(&trigger, &facts),
            [
                Zone::Library,
                Zone::Graveyard,
                Zone::Stack,
                Zone::Exile,
                Zone::Command,
                Zone::Ante,
                Zone::OutsideGame
            ],
        );
        facts.excluded_zones.clear();
        assert_eq!(
            derive_triggered_ability_functional_zones_from_facts(&trigger, &facts),
            [Zone::Hand]
        );
        facts.discards_this_card = false;
        assert_eq!(
            derive_triggered_ability_functional_zones_from_facts(&trigger, &facts),
            [Zone::Graveyard]
        );
    }

    #[test]
    fn a_qualified_attachment_death_in_a_trigger_alternative_keeps_its_event_zone() {
        use ironsmith_core::filter_model::{ObjectFilter, ObjectRef, PlayerFilter};
        let death = TriggerSpec::PutIntoGraveyard(
            ObjectFilter::creature()
                .controlled_by(PlayerFilter::ControllerOf(ObjectRef::tagged("equipped"))),
        );
        let trigger = TriggerSpec::AnyOf(vec![
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::You),
            TriggerSpec::ConditionQualified {
                trigger: Box::new(death),
                condition: PredicateAst::YourTurn,
                surface: "during your turn".into(),
            },
        ]);
        assert_eq!(
            derive_triggered_ability_functional_zones_from_facts(
                &trigger,
                &TriggerFunctionalZoneFacts {
                    returns_self_from_graveyard: true,
                    ..Default::default()
                }
            ),
            [Zone::Battlefield],
        );
    }
}
