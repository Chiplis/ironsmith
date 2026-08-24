use super::*;

pub fn parse_get_then_ability_shape(tokens: &[OwnedLexToken]) -> Option<GetThenAbilityShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (get_token, (), after_get) = primitives::find_prefix(tokens, || get_verb)?;
    let raw_subject_tokens = tokens.get(..get_token)?;
    if independent_player_action_precedes_shared_subject(raw_subject_tokens) {
        return None;
    }
    let subject_tokens = semantic_subject_tokens(raw_subject_tokens)?;
    let (separator_token, ability_verb, ability_tokens) =
        primitives::find_prefix(after_get, || {
            (primitives::kw("and"), shared_ability_verb).map(|(_, verb)| verb)
        })?;
    let pump_tokens = nonempty_trimmed(after_get.get(..separator_token)?)?;
    let ability_tokens = nonempty_trimmed(ability_tokens)?;
    Some(GetThenAbilityShape {
        subject_tokens,
        pump_tokens,
        ability_tokens,
        ability_verb,
    })
}
