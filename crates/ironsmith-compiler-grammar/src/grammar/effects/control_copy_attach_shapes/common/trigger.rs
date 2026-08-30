use super::*;

pub fn parse_delayed_hand_tail(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let mut offset = 0usize;
    let mut last = None;
    while offset < tokens.len() {
        let Some((relative, _, _)) = primitives::find_prefix(&tokens[offset..], || {
            alt((primitives::kw("hand"), primitives::kw("hands"))).void()
        }) else {
            break;
        };
        let index = offset + relative;
        last = tokens.get(index + 1..);
        offset = index + 1;
    }
    last.map(trim_lexed_commas)
}
