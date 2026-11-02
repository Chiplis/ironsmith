//! A donor filter and an ability-group selector are independent readers.
use super::*;
pub fn parse_designated_ability_inheritance_line(tokens: &[OwnedLexToken]) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let view = crate::lexer::TokenWordView::new(&tokens);
    let words = view.word_refs();
    let Some(has) = words.iter().position(|word| matches!(*word, "has" | "have")) else { return Ok(None); };
    let subject = &words[..has];
    if !matches!(subject, ["this"] | ["it"] | ["they"] | ["he"] | ["she"] | ["this", "creature" | "permanent" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle"]) { return Ok(None); }
    let tail = &words[has + 1..];
    let (donor_word, excluded) = match tail {
        ["all", "abilities", "of", ..] => (has + 4, Vec::new()),
        ["all", selector, "abilities", "of", ..] => {
            let Some(excluded) = selector.strip_prefix("non-").or_else(|| selector.strip_prefix("non")) else { return Ok(None); };
            if excluded.is_empty() || !excluded.chars().all(|character| character.is_ascii_alphabetic() || character == '-') { return Ok(None); }
            (has + 5, vec![excluded.to_string()])
        }
        _ => return Ok(None),
    };
    if donor_word >= words.len() { return Ok(None); }
    let filter = parse_object_filter_lexed(&tokens[view.token_start_indices()[donor_word]..], false)?;
    let mut activated = crate::static_abilities::CopyActivatedAbilities::new(filter.clone());
    activated.excluded_ability_designations = excluded.clone();
    let mut triggered = crate::static_abilities::CopyTriggeredAbilities::new(filter.clone());
    triggered.excluded_ability_designations = excluded.clone();
    let mut statics = ironsmith_core::CopyStaticAbilityVariants::new(filter, Vec::new(), crate::lexer::render_token_slice(&tokens));
    statics.copy_all = true;
    statics.excluded_ability_designations = excluded;
    crate::word_claims::claim_tokens(&tokens[..view.token_start_indices()[donor_word]]);
    Ok(Some(vec![StaticAbility::copy_activated_abilities(activated).into(), StaticAbility::copy_triggered_abilities(triggered).into(), StaticAbility::copy_static_ability_variants(statics).into()]))
}
