//! "Artifact landcycling {2}" (Sojourner's Companion): CR 702.29e
//! typecycling whose quality is qualified by a leading card type; the
//! searched card must be both an artifact and a land. Source-authored, unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::{CardType, Zone};

#[path = "p02_line_families/compile.rs"]
mod compile;

const SOJOURNERS_COMPANION: &str = "Mana cost: {7}\nType: Artifact Creature — Salamander\nPower/Toughness: 4/4\nAffinity for artifacts\nArtifact landcycling {2} ({2}, Discard this card: Search your library for an artifact land card, reveal it, put it into your hand, then shuffle.)";

#[test]
fn artifact_landcycling_searches_for_artifact_lands_from_hand() {
    for definition in compile::compile_both("Sojourner's Companion", SOJOURNERS_COMPANION) {
        let cycling = definition
            .abilities
            .iter()
            .find(|ability| {
                matches!(ability.kind, AbilityKind::Activated(_))
                    && ability.functional_zones.contains(&Zone::Hand)
            })
            .expect("landcycling is an activated ability from hand");
        let debug = format!("{cycling:?}");
        assert!(debug.contains("Discard"), "{debug}");
        let all_types = format!("all_card_types: {:?}", vec![CardType::Artifact, CardType::Land]);
        assert!(debug.contains(&all_types), "artifact AND land: {debug}");
        assert!(debug.contains("ShuffleLibraryEffect"), "{debug}");
    }
}
