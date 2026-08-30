use super::*;

pub fn is_exile_hand_or_permanent_choice_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        hand_or_permanent_choice,
        "exile-hand-or-permanent-choice",
    )
    .is_ok()
}

pub fn parse_each_opponent_exile_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<EachOpponentExileChoiceShape> {
    let ((), choice) = primitives::parse_prefix(tokens, each_opponent_exiles)?;
    is_exile_hand_or_permanent_choice_shape(choice).then(|| EachOpponentExileChoiceShape {
        choice: choice.to_vec(),
    })
}
