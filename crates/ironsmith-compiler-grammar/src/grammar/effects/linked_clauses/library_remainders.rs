use winnow::Parser;

use crate::grammar::primitives;
use crate::lexer::OwnedLexToken;

pub fn parse_remainder_to_hand_presence(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        primitives::phrase(&["put", "the", "rest", "into", "your", "hand"]).void()
    })
    .is_some()
}
