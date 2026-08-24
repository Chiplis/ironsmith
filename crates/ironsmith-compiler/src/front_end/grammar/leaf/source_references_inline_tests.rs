use super::*;

fn exact(aliases: &[LeafSourceReferenceAlias], words: &[&str]) -> Option<SourceReferenceSurface> {
    parse_leaf_source_reference_alias_words(aliases, words)
}

#[test]
fn name_aliases_preserve_full_short_face_article_and_internal_article_surfaces() {
    let aliases = parse_leaf_source_reference_aliases_for_name("Kraven the Hunter");
    assert_eq!(
        exact(&aliases, &["kraven", "the", "hunter"]),
        Some(SourceReferenceSurface::FullName(
            "Kraven the Hunter".to_string()
        ))
    );
    assert_eq!(
        exact(&aliases, &["kraven", "hunter"]),
        Some(SourceReferenceSurface::FullName(
            "Kraven the Hunter".to_string()
        ))
    );
    assert_eq!(
        exact(&aliases, &["kraven"]),
        Some(SourceReferenceSurface::ShortName("Kraven".to_string()))
    );

    let aliases =
        parse_leaf_source_reference_aliases_for_name("Delver of Secrets // Insectile Aberration");
    assert_eq!(
        exact(&aliases, &["delver", "of", "secrets"]),
        Some(SourceReferenceSurface::FullName(
            "Delver of Secrets".to_string()
        ))
    );

    let aliases = parse_leaf_source_reference_aliases_for_name("The Gitrog Monster");
    assert_eq!(
        exact(&aliases, &["gitrog", "monster"]),
        Some(SourceReferenceSurface::FullName(
            "The Gitrog Monster".to_string()
        ))
    );
}

#[test]
fn planeswalker_first_names_remain_valid_source_aliases() {
    for (name, first_name) in [
        ("Sorin of House Markov // Sorin, Ravenous Neonate", "sorin"),
        ("Jace, Vryn's Prodigy // Jace, Telepath Unbound", "jace"),
    ] {
        let aliases = parse_leaf_source_reference_aliases_for_name(name);
        assert_eq!(
            exact(&aliases, &[first_name]),
            Some(SourceReferenceSurface::ShortName(
                first_name[..1].to_ascii_uppercase() + &first_name[1..]
            ))
        );
    }
}

#[test]
fn source_alias_matching_is_case_insensitive_after_lexical_name_restoration() {
    let aliases = parse_leaf_source_reference_aliases_for_name("Ghyrson Starn, Kelermorph");
    assert_eq!(
        exact(&aliases, &["ghyrson", "Starn"]),
        Some(SourceReferenceSurface::ShortName(
            "Ghyrson Starn".to_string()
        ))
    );
}

#[test]
fn color_adjectives_do_not_become_short_source_aliases() {
    for (name, color) in [
        ("Black Scarab", "black"),
        ("Blue Scarab", "blue"),
        ("Green Scarab", "green"),
        ("Red Scarab", "red"),
        ("White Scarab", "white"),
    ] {
        let aliases = parse_leaf_source_reference_aliases_for_name(name);
        assert_eq!(
            exact(&aliases, &[color]),
            None,
            "{color} must remain available to object-filter parsing: {aliases:#?}"
        );
        assert_eq!(
            exact(&aliases, &[color, "scarab"]),
            Some(SourceReferenceSurface::FullName(name.to_string()))
        );
    }
}

#[test]
fn name_aliases_preserve_comma_digital_and_roman_variants() {
    let aliases = parse_leaf_source_reference_aliases_for_name("Sarulf, Realm Eater");
    assert_eq!(
        exact(&aliases, &["sarulf"]),
        Some(SourceReferenceSurface::ShortName("Sarulf".to_string()))
    );

    let aliases = parse_leaf_source_reference_aliases_for_name("A-Satoru Umezawa");
    assert_eq!(
        exact(&aliases, &["satoru", "umezawa"]),
        Some(SourceReferenceSurface::FullName(
            "A-Satoru Umezawa".to_string()
        ))
    );
    assert_eq!(
        exact(&aliases, &["satoru"]),
        Some(SourceReferenceSurface::ShortName("Satoru".to_string()))
    );

    let aliases = parse_leaf_source_reference_aliases_for_name("Ajani Vengeant II");
    assert_eq!(
        exact(&aliases, &["ajani", "vengeant"]),
        Some(SourceReferenceSurface::FullName(
            "Ajani Vengeant".to_string()
        ))
    );
}

#[test]
fn alias_word_variants_preserve_parser_lexer_and_surface_tokenizations() {
    let variants = parse_source_reference_word_variants("Kraven’s the-Hunter");
    assert!(variants.contains(&vec![
        "kravens".to_string(),
        "the".to_string(),
        "hunter".to_string()
    ]));
    assert!(variants.contains(&vec!["kraven's".to_string(), "the-hunter".to_string()]));
    assert!(variants.contains(&vec!["kravens".to_string(), "hunter".to_string()]));
}

#[test]
fn exact_and_possessive_alias_parsers_return_the_original_surface() {
    let aliases = parse_leaf_source_reference_aliases_for_name("Sarulf, Realm Eater");
    let short = SourceReferenceSurface::ShortName("Sarulf".to_string());
    assert_eq!(exact(&aliases, &["sarulf"]), Some(short.clone()));
    assert_eq!(exact(&aliases, &["sarulfs"]), None);
    assert_eq!(
        parse_leaf_source_reference_possessive_alias_words(&aliases, &["sarulfs"]),
        Some(short)
    );
    assert_eq!(
        parse_leaf_source_reference_possessive_alias_words(
            &aliases,
            &["sarulf", "realm", "eaters"]
        ),
        Some(SourceReferenceSurface::FullName(
            "Sarulf, Realm Eater".to_string()
        ))
    );
}

#[test]
fn this_source_parser_preserves_canonical_surface_rules() {
    for (words, expected) in [
        (&["this"][..], "this"),
        (&["thiss"][..], "this"),
        (&["this", "creatures"][..], "this creature"),
        (&["this", "goblins"][..], "this goblin"),
        (&["this", "of", "those", "cards"][..], "this of those cards"),
    ] {
        assert_eq!(
            parse_leaf_this_source_reference_words(words),
            Some(SourceReferenceSurface::ThisPermanentType(
                expected.to_string()
            ))
        );
    }
    assert_eq!(parse_leaf_this_source_reference_words(&[]), None);
    assert_eq!(
        parse_leaf_this_source_reference_surface("Creature"),
        Some(SourceReferenceSurface::ThisPermanentType(
            "this creature".to_string()
        ))
    );
    assert_eq!(
        parse_leaf_this_source_reference_surface("Goblin"),
        Some(SourceReferenceSurface::ThisPermanentType(
            "this Goblin".to_string()
        ))
    );
}
