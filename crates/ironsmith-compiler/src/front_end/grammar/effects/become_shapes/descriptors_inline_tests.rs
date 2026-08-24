use crate::lexer::lex_line;

use super::*;

#[test]
fn parses_typed_simple_descriptors_and_animation_suffixes() {
    assert!(matches!(
        parse_become_simple_descriptor_words(&["blue", "zombie"]),
        BecomeSimpleDescriptorShape::ColorsAndSubtypes { .. }
    ));
    assert!(matches!(
        parse_become_simple_descriptor_words(&["bird"]),
        BecomeSimpleDescriptorShape::Subtypes {
            replace_creature_subtypes: true,
            ..
        }
    ));

    let tokens = lex_line("with flying and all creature types", 0).expect("lex fixture");
    assert!(matches!(
        parse_become_animation_suffix_shape(&tokens),
        BecomeAnimationSuffixShape::With {
            grants_all_creature_types: true,
            ..
        }
    ));

    let tokens =
        lex_line("with flying in addition to its other types", 0).expect("lex additive animation");
    assert!(matches!(
        parse_become_animation_suffix_shape(&tokens),
        BecomeAnimationSuffixShape::With {
            preserve_other_types: true,
            ..
        }
    ));

    let tokens = lex_line("it's still a land", 0).expect("lex retained land");
    assert!(matches!(
        parse_become_animation_suffix_shape(&tokens),
        BecomeAnimationSuffixShape::Ignored {
            preserve_other_types: true,
            type_retention_surface: Some(ironsmith_core::TypeRetentionSurface::StillALand),
        }
    ));

    let tokens = lex_line("with vigilance and haste that's still a land", 0)
        .expect("lex retained land ability suffix");
    assert!(matches!(
        parse_become_animation_suffix_shape(&tokens),
        BecomeAnimationSuffixShape::With {
            preserve_other_types: true,
            type_retention_surface: Some(ironsmith_core::TypeRetentionSurface::StillALand),
            ..
        }
    ));

    let tokens = lex_line("that's still a planeswalker", 0).expect("lex retained planeswalker");
    let retained_planeswalker = parse_become_animation_suffix_shape(&tokens);
    assert!(
        matches!(
            retained_planeswalker,
            BecomeAnimationSuffixShape::Ignored {
                preserve_other_types: true,
                type_retention_surface: Some(ironsmith_core::TypeRetentionSurface::StillACardType(
                    CardType::Planeswalker
                )),
            }
        ),
        "{retained_planeswalker:#?}; words={:?}",
        parser_token_word_refs(&tokens)
    );

    assert!(matches!(
        parse_become_simple_descriptor_words(&[
            "enchantment",
            "in",
            "addition",
            "to",
            "its",
            "other",
            "types",
        ]),
        BecomeSimpleDescriptorShape::CardTypes {
            preserve_other_types: true,
            ..
        }
    ));
}

#[test]
fn reconstructed_hyphenated_subtypes_remain_typed_in_animations() {
    let prefix = parse_become_leading_creature_prefix(&["assembly", "worker", "artifact"]);
    assert!(prefix.supported, "{prefix:#?}");
    assert_eq!(prefix.subtypes, [Subtype::AssemblyWorker], "{prefix:#?}");
    assert!(
        prefix.card_types.contains(&CardType::Artifact),
        "{prefix:#?}"
    );

    let descriptor =
        parse_become_creature_descriptor_words(&["assembly", "worker", "artifact", "creature"])
            .expect("synthetic split subtype should remain a typed descriptor");
    assert_eq!(
        descriptor.subtypes,
        [Subtype::AssemblyWorker],
        "{descriptor:#?}"
    );
}
