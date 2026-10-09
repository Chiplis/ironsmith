//! UNVALIDATED implementation-first coverage (cf8 p09): "It doesn't untap
//! during its controller's next two untap steps." keeps the restriction for
//! that many of the controller's untap steps.
use ironsmith::effect::RestrictionDurationSurface;
use ironsmith::effects::CantEffect;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn telekinesis_covers_two_untap_steps() {
    let rows = common::rows(include_str!("../../../fixtures/counted_untap_steps.json.fixture"));
    for definition in common::definitions(common::row(&rows, "Telekinesis")) {
        let effects = common::all_effects(&definition);
        let cant = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<CantEffect>())
            .expect("untap restriction");
        assert_eq!(cant.duration_surface, RestrictionDurationSurface::NextUntapSteps(2));
        assert_eq!(cant.duration_surface.additional_untap_steps(), 1);
    }
}
