use super::*;
use crate::lexer::lex_line;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_resource_look_shapes() {
    assert!(matches!(
        parse_resource_look_shape(&lex("at the top two cards of your library"), None),
        Some(ResourceLookShape::TopCards {
            player: PlayerAst::You,
            count: Value::Fixed(2)
        })
    ));
    let hand_tokens = lex("at target player's hand.");
    assert!(matches!(
        parse_resource_look_shape(&hand_tokens, None),
        Some(ResourceLookShape::Hand {
            player: PlayerAst::Target,
            ..
        })
    ));

    let dynamic_tokens =
        lex("at the top X cards of your library, where X is that creature's power");
    let Some(ResourceLookShape::TopCards { count, .. }) =
        parse_resource_look_shape(&dynamic_tokens, None)
    else {
        panic!("expected dynamic top-card look shape");
    };
    assert!(count.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs));
    assert!(matches!(count.unhinted(), Value::PowerOf(_)));
}

#[path = "tests/reference_programs.rs"]
mod reference_programs;
use reference_programs::parses_all_unspent_mana_resource_shape;
#[path = "tests/choice_programs.rs"]
mod choice_programs;
use choice_programs::parses_resource_chosen_name_target_shape;
#[path = "tests/library_programs.rs"]
mod library_programs;
use library_programs::{
    parses_hyphenated_face_down_look_shapes, parses_resource_shuffle_shapes,
    parses_spy_network_compound_look_shape,
};
