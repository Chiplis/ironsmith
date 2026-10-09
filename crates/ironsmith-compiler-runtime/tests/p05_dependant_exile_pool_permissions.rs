//! cf8 p05 shapes for other packages' rows: permissions over an exiled
//! pool. "Until end of turn, you may cast spells from among cards exiled
//! with this Saga, and you may spend mana as though it were mana of any color
//! to cast those spells." reads the source-linked pool; "Exile the top three
//! cards of your library. Choose one. You may play that card this turn."
//! selects one exiled card for the permission; "You may cast Equipment spells
//! this way without paying their mana costs." gives the permission's free
//! price to the matching spells only. Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

use ironsmith::effects::{ChooseObjectsEffect, GrantPlayTaggedEffect};
use ironsmith_core::GrantPlayTaggedDuration;
use ironsmith_core::value_model::ManaSpendMode;

const CLUSTER: &str = "exile_pool_permissions";

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::dependant_rows(CLUSTER);
    assert_eq!(rows.len(), 3);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn king_narfis_betrayal_casts_from_the_saga_pool_with_any_color_mana() {
    for definition in
        support::definitions(&support::dependant_row(CLUSTER, "King Narfi's Betrayal"))
    {
        let grants = support::effects_of::<GrantPlayTaggedEffect>(&definition);
        assert!(!grants.is_empty());
        for grant in &grants {
            assert_eq!(grant.duration, GrantPlayTaggedDuration::UntilEndOfTurn);
            assert_eq!(grant.mana_spend_mode, ManaSpendMode::AnyColor);
            assert!(!grant.allow_land, "\"cast spells\" excludes lands");
        }
    }
}

#[test]
fn chandra_chooses_one_exiled_card_to_play_this_turn() {
    for definition in support::definitions(&support::dependant_row(CLUSTER, "Chandra, Flameshaper")) {
        let choices = support::effects_of::<ChooseObjectsEffect>(&definition);
        assert!(
            choices
                .iter()
                .any(|choice| choice.count.min == 1 && choice.count.max == Some(1)),
            "{choices:#?}"
        );
        let grants = support::effects_of::<GrantPlayTaggedEffect>(&definition);
        assert_eq!(grants.len(), 1, "{grants:#?}");
        assert!(grants[0].allow_land);
        assert_eq!(grants[0].duration, GrantPlayTaggedDuration::UntilEndOfTurn);
    }
}

#[test]
fn nahiri_frees_only_equipment_cast_through_the_permission() {
    for definition in support::definitions(&support::dependant_row(CLUSTER, "Nahiri, Forged in Fury")) {
        let debug = support::debug(&definition);
        assert!(debug.contains("GrantTaggedSpellFreeCastUntilEndOfTurnEffect"), "{debug}");
        assert!(debug.contains("Equipment"), "{debug}");
        assert!(debug.contains("TagMatchingObjectsEffect"), "{debug}");
    }
}
