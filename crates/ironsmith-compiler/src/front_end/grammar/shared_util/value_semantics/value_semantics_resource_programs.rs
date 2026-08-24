use super::*;

pub fn parse_where_x_greatest_commander_mana_value(
    tokens: &[OwnedLexToken],
    commander_start_word_idx: usize,
) -> Option<Value> {
    let words = TokenWordView::new(tokens);
    let commander_range = words.token_span_for_words(commander_start_word_idx, words.len())?;
    let commander_words = crate::lexer::token_word_refs(&tokens[commander_range]);
    let normalized = commander_words
        .iter()
        .copied()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect::<Vec<_>>();
    let owner = commander_owner_from_battlefield_or_command_zone_words(&normalized)?;

    let mut battlefield_commander = ObjectFilter::default();
    battlefield_commander.zone = Some(Zone::Battlefield);
    battlefield_commander.is_commander = true;
    battlefield_commander.owner = Some(owner);

    let mut command_zone_commander = battlefield_commander.clone();
    command_zone_commander.zone = Some(Zone::Command);

    let mut combined = ObjectFilter::default();
    combined.any_of = vec![battlefield_commander, command_zone_commander];

    Some(Value::GreatestManaValue(combined))
}
