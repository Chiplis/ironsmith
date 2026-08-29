use super::*;

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
