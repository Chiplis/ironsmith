use super::*;

#[test]
fn officer_klingon_and_tribble_filters_accept_singular_and_plural() {
    for (word, subtype) in [("officer", Subtype::Officer), ("klingon", Subtype::Klingon), ("tribble", Subtype::Tribble)] {
        for surface in [word.to_string(), format!("{word}s")] {
            assert_eq!(parse_leaf_subtype_complete(&surface).unwrap(), subtype);
            assert_eq!(parse_leaf_subtype_flexible_complete(&surface).unwrap(), subtype);
        }
    }
}

#[test]
fn card_types_and_zones_preserve_singular_and_plural_surfaces() {
    assert_eq!(
        parse_leaf_card_type_complete("sorceries").unwrap(),
        CardType::Sorcery
    );
    assert_eq!(
        parse_leaf_zone_complete("libraries").unwrap(),
        Zone::Library
    );
}

#[test]
fn subtype_parser_preserves_irregular_and_flexible_plurals() {
    assert_eq!(parse_leaf_subtype_complete("mice").unwrap(), Subtype::Mouse);
    assert_eq!(
        parse_leaf_subtype_flexible_complete("fungi").unwrap(),
        Subtype::Fungus
    );
    assert_eq!(
        parse_leaf_subtype_flexible_complete("foxes").unwrap(),
        Subtype::Fox
    );
    assert_eq!(
        parse_leaf_subtype_flexible_complete("wolves").unwrap(),
        Subtype::Wolf
    );
}

#[test]
fn subtype_surface_index_matches_exhaustive_lookup() {
    for family in [
        SubtypeFamily::Land,
        SubtypeFamily::Creature,
        SubtypeFamily::Artifact,
        SubtypeFamily::Enchantment,
        SubtypeFamily::Spell,
        SubtypeFamily::Planeswalker,
        SubtypeFamily::Battle,
    ] {
        for subtype in family.all_subtypes() {
            for surface in subtype_surfaces(*subtype) {
                assert_eq!(
                    classify_token_definition_subtype(surface.as_str()),
                    classify_token_definition_subtype_slow(surface.as_str()),
                    "surface {surface:?} for {subtype:?}"
                );
            }
        }
    }
}

#[test]
fn negated_descriptors_return_the_positive_typed_atom() {
    assert_eq!(
        parse_leaf_non_card_type_complete("noncreature").unwrap(),
        CardType::Creature
    );
    assert_eq!(
        parse_leaf_non_subtype_complete("nonwolves").unwrap(),
        Subtype::Wolf
    );
    assert!(parse_leaf_non_card_type_complete("creature").is_err());
}

#[test]
fn demonstrative_heads_are_typed() {
    assert_eq!(
        parse_leaf_demonstrative_object_head_complete("sources").unwrap(),
        LeafDemonstrativeObjectHead::Source
    );
    assert_eq!(
        parse_leaf_demonstrative_object_head_complete("creatures").unwrap(),
        LeafDemonstrativeObjectHead::CardType(CardType::Creature)
    );
    assert_eq!(
        parse_leaf_object_reference_head_complete("goblins").unwrap(),
        LeafObjectReferenceHead::Subtype(Subtype::Goblin)
    );
}

#[test]
fn printed_subtype_collision_keeps_official_default_and_explicit_family_identity() {
    for spelling in ["Omen", "Omens"] {
        assert_eq!(classify_token_definition_subtype(spelling), Some(Subtype::Omen));
        assert_eq!(classify_token_definition_subtype_slow(spelling), Some(Subtype::Omen));
        assert_eq!(classify_token_definition_subtype_in_family(spelling, SubtypeFamily::Spell), Some(Subtype::Omen));
        assert_eq!(classify_token_definition_subtype_in_family(spelling, SubtypeFamily::Artifact), Some(Subtype::ArtifactOmen));
        let alternatives = ambiguous_subtype_spellings(spelling);
        assert_eq!(alternatives.len(), 2);
        assert!(alternatives.contains(&Subtype::Omen));
        assert!(alternatives.contains(&Subtype::ArtifactOmen));
    }
    assert!(ambiguous_subtype_spellings("Omen of the Sea").is_empty());
    assert_eq!(classify_token_definition_subtype_in_family("Mantra", SubtypeFamily::Spell), Some(Subtype::Mantra));
    assert_eq!(classify_token_definition_subtype_in_family("Mantra", SubtypeFamily::Artifact), None);
}

#[test]
fn primary_official_collision_keeps_existing_family_precedence() {
    // Urza's is a land type; it must not be replaced by the plural of the
    // planeswalker type Urza when the cached vocabulary encounters that later.
    assert_eq!(classify_token_definition_subtype("Urza's"), Some(Subtype::Urzas));
    assert_eq!(classify_token_definition_subtype("Urzas"), Some(Subtype::Urzas));
    assert_eq!(classify_token_definition_subtype("Urza"), Some(Subtype::Urza));
    assert_eq!(classify_token_definition_subtype_in_family("Urzas", SubtypeFamily::Planeswalker), Some(Subtype::Urza));
    assert_eq!(classify_token_definition_subtype_in_family("Urzas", SubtypeFamily::Creature), None);
}

#[test]
fn inscription_vocabulary_keeps_source_families_out_of_creature_choices() {
    for spelling in ["Inscription", "Inscriptions"] {
        assert_eq!(classify_token_definition_subtype_in_family(spelling, SubtypeFamily::Spell), Some(Subtype::Inscription));
        assert_eq!(classify_token_definition_subtype_in_family(spelling, SubtypeFamily::Artifact), Some(Subtype::ArtifactInscription));
        assert_eq!(classify_token_definition_subtype_in_family(spelling, SubtypeFamily::Creature), None);
        let meanings=ambiguous_subtype_spellings(spelling);
        assert_eq!(meanings.len(),2);
        assert!(meanings.contains(&Subtype::Inscription));
        assert!(meanings.contains(&Subtype::ArtifactInscription));
    }
    assert!(!Subtype::all_creature_types().contains(&Subtype::Inscription));
    assert!(!Subtype::all_creature_types().contains(&Subtype::ArtifactInscription));
}
