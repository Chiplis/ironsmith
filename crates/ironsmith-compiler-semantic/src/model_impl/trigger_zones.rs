//! Zone ownership for trigger alternatives, shared by grammar and lowering.
use super::ast::{PredicateAst, SourcePredicateAst, TriggerSpec};
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
        _ => vec![Zone::Battlefield],
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
}
