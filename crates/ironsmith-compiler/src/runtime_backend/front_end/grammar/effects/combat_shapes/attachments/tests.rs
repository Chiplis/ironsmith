use crate::runtime_backend::lexer::{lex_line, parser_token_word_refs};

use super::*;

#[test]
fn parses_attachment_clause_shapes() {
    let tokens = lex_line("that equipment to target creature", 0).unwrap();
    let CombatAttachClauseShape::Standard {
        object_tokens,
        target_tokens,
        ..
    } = parse_combat_attach_clause_shape_lexed(&tokens).unwrap()
    else {
        panic!("expected standard attachment")
    };
    assert_eq!(parser_token_word_refs(object_tokens), ["that", "equipment"]);
    assert_eq!(
        parser_token_word_refs(target_tokens),
        ["target", "creature"]
    );

    let shape = parse_combat_attach_object_shape_lexed(object_tokens).unwrap();
    assert!(matches!(
        shape,
        CombatAttachObjectShape::Tagged(CombatAttachTaggedObjectShape::Equipment)
    ));

    let triggering_to_token = lex_line("it to the token.", 0).unwrap();
    assert!(matches!(
        parse_combat_attach_clause_shape_lexed(&triggering_to_token).unwrap(),
        CombatAttachClauseShape::Standard {
            triggering_object_to_token: true,
            target_is_tagged: false,
            ..
        }
    ));

    let attached = lex_line("enchanted Equipment", 0).unwrap();
    assert_eq!(
        parse_attached_object_reference_tokens(&attached),
        Some(AttachedObjectReferenceShape {
            tag: AttachedObjectReferenceTag::Enchanted,
            kind: AttachedObjectReferenceKind::Equipment,
        })
    );
}
