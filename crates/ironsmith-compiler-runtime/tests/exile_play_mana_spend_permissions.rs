//! cf8 p05: play-from-exile permission extensions on the tagged grant.
//! "you may spend colorless mana as though it were mana of any color to cast
//! that spell" carries `ManaSpendMode::ColorlessAsAnyColor` through the
//! exile permission (CR 609.4b: only colorless mana converts), and a
//! permission may name another player as its grantee ("that creature's
//! controller may play that card and they may spend mana as though it were
//! mana of any color"). Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

use ironsmith::effects::GrantPlayTaggedEffect;
use ironsmith::target::{ObjectRef, PlayerFilter};
use ironsmith_core::GrantPlayTaggedDuration;
use ironsmith_core::value_model::ManaSpendMode;

const CLUSTER: &str = "exile_play_mana_spend_permissions";

fn grants(definition: &ironsmith::cards::CardDefinition) -> Vec<GrantPlayTaggedEffect> {
    support::effects_of::<GrantPlayTaggedEffect>(definition)
}

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 2);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn abstruse_appropriation_converts_only_colorless_mana_for_as_long_as_exiled() {
    for definition in support::definitions(&support::row(CLUSTER, "Abstruse Appropriation")) {
        let grants = grants(&definition);
        assert_eq!(grants.len(), 1, "{grants:?}");
        let grant = &grants[0];
        assert_eq!(grant.duration, GrantPlayTaggedDuration::ForAsLongAsExiled);
        assert_eq!(grant.player, PlayerFilter::You);
        assert_eq!(grant.mana_spend_mode, ManaSpendMode::ColorlessAsAnyColor);
        assert!(!grant.allow_any_color_for_cast, "colored mana is not converted");
        assert!(!grant.permission_bound_mana);
    }
}

#[test]
fn curse_of_hospitality_grants_the_damaging_creatures_controller_any_color_play() {
    for definition in support::definitions(&support::row(CLUSTER, "Curse of Hospitality")) {
        let grants = grants(&definition);
        assert_eq!(grants.len(), 1, "{grants:?}");
        let grant = &grants[0];
        assert_eq!(grant.duration, GrantPlayTaggedDuration::UntilEndOfTurn);
        assert!(grant.allow_land, "\"play that card\" includes a land");
        assert_eq!(grant.mana_spend_mode, ManaSpendMode::AnyColor);
        // The grantee is the controller of the creature that dealt the
        // damage (the trigger's event source), not the curse's controller.
        assert!(
            matches!(&grant.player, PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
                if tag.as_str() == "triggering_source"),
            "{:?}",
            grant.player
        );
    }
}
