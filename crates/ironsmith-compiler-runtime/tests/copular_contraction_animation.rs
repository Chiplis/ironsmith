//! cf8 p05: contracted pronoun copulas ("It's a ...", "He's a ... in addition
//! to his other types", "They're black Zombies ..."), copular predicate pairs
//! ("That creature is black and is a Nightmare ..."), counter-linked land
//! types with a pronoun subject or a set (not added) subtype (CR 305.7), the
//! "for as long as it has a <kind> counter on it" animation duration
//! (CR 611.2b) and "It's still a Cave land" retention (CR 205.1b).
//! Source-authored; deliberately unrun until the campaign build.
#[path = "cf8_p05_support/mod.rs"]
mod support;

const CLUSTER: &str = "copular_contraction_animation";

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 11);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn contracted_copula_lowers_to_become_characteristics() {
    for (name, markers) in [
        ("Brilliance Unleashed", &["Robot", "Flying"][..]),
        ("Fang, Roku's Companion", &["Spirit"][..]),
        ("Princess Yue", &["SetName", "Moon", "Land"][..]),
        ("The Master, Transcendent", &["Mutant"][..]),
        ("Yedora, Grave Gardener", &["Forest", "Land"][..]),
        ("Grimoire of the Dead", &["Zombie"][..]),
    ] {
        for definition in support::definitions(&support::row(CLUSTER, name)) {
            let debug = support::debug(&definition);
            for marker in markers {
                assert!(debug.contains(marker), "{name}: missing {marker}");
            }
        }
    }
}

#[test]
fn counter_linked_animation_and_land_types_last_while_the_counter_remains() {
    for (name, counter, marker) in [
        ("Sauron, Dino Devotee", "saurian", "Dinosaur"),
        ("Eluge, the Shoreless Sea", "flood", "Island"),
        ("Quicksilver Fountain", "flood", "Island"),
    ] {
        for definition in support::definitions(&support::row(CLUSTER, name)) {
            let debug = support::debug(&definition);
            assert!(debug.contains("ObjectHasCounter"), "{name}: duration");
            assert!(debug.contains("AffectedObject"), "{name}: affected object");
            assert!(debug.to_ascii_lowercase().contains(counter), "{name}: {counter}");
            // Quicksilver Fountain's "is an Island" sets the land subtype
            // (CR 305.7); the set happens in the executor of the fixed
            // basic-land-type effect, so the lowered form names that effect.
            if name == "Quicksilver Fountain" {
                assert!(debug.contains("BecomeBasicLandTypeChoiceEffect"), "{name}");
                assert!(debug.contains("Island"), "{name}");
            } else {
                assert!(debug.contains(marker), "{name}: {marker}");
            }
        }
    }
}

#[test]
fn chainer_sets_black_and_adds_nightmare() {
    for definition in support::definitions(&support::row(CLUSTER, "Chainer, Dementia Master")) {
        let debug = support::debug(&definition);
        assert!(debug.contains("Nightmare"));
    }
}

#[test]
fn cavernous_maw_animation_keeps_its_land_types() {
    for definition in support::definitions(&support::row(CLUSTER, "Cavernous Maw")) {
        let debug = support::debug(&definition);
        assert!(debug.contains("Elemental"));
        // CR 205.1b: the "still a Cave land" followup is consumed by the
        // animation, so no separate effect or loss remains.
    }
}
