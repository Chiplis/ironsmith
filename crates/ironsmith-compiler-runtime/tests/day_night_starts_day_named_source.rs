//! UNVALIDATED implementation-first coverage: "If it's neither day nor night,
//! it becomes day as <named source> enters" (CR 731.2a) with the source named
//! by a proper name, which normalizes to a bare self reference.
use ironsmith::ability::AbilityKind;
use ironsmith::static_abilities::StaticAbilityId;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn vadrik_starts_day_as_it_enters() {
    let rows = common::rows(include_str!("../../../fixtures/day_night_starts_day_named_source.json.fixture"));
    let row = common::row(&rows, "Vadrik, Astral Archmage");
    for definition in common::definitions(row) {
        let ids = common::static_ids(&definition);
        assert!(ids.contains(&StaticAbilityId::DayNightStartsDayAsEnters), "{ids:?}");
        assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("night"))));
    }
}
