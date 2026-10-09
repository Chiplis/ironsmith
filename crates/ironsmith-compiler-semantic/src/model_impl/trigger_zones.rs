//! Zone ownership for trigger alternatives, shared by grammar and lowering.
use super::ast::{PredicateAst, SourcePredicateAst, TriggerSpec};
use ironsmith_core::Zone;

fn qualified_zone(condition: &PredicateAst) -> Option<Zone> {
    match condition {
        PredicateAst::Source(SourcePredicateAst::SourceIsInZone(zone)) => Some(*zone),
        PredicateAst::And(left, right) => match (qualified_zone(left), qualified_zone(right)) {
            (Some(a), Some(b)) if a != b => None,
            (a, b) => a.or(b),
        },
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
            qualified_zone(condition).map(|zone| vec![zone])
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
            TriggerSpec::ConditionQualified { condition, .. } => qualified_zone(condition) == Some(zone),
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
