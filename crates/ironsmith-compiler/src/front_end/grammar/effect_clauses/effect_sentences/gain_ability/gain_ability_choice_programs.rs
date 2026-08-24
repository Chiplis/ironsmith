use super::*;

pub fn parse_choice_of_abilities(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let shape = gain_shapes::parse_ability_choice_shape(tokens)?;
    let mut actions = Vec::new();
    for segment in shape.options {
        let action = parse_ability_phrase(segment)?;
        push_unique_keyword_action(&mut actions, action);
    }

    if actions.len() < 2 {
        return None;
    }
    Some(actions)
}
