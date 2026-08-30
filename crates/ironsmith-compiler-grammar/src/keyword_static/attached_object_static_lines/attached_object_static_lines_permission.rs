use super::*;

pub fn parse_enchanted_has_activated_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = attached_grammar::parse_attached_has_tokens(tokens) else {
        return Ok(None);
    };
    if shape.subject.is_equipped() {
        return Ok(None);
    }
    let ability_tokens_raw = shape.ability_tokens;
    let ability_tokens = trim_edge_punctuation(ability_tokens_raw);

    // A mixed `has vigilance and "{W}, {T}: ..."` clause is not one
    // activated ability.  The permissive activated-line parser can recover the
    // quoted colon tail from that larger slice, so prove that no leading
    // keyword grant would be discarded before letting this early rule claim
    // the line.  The later attached-object rule lowers both halves.
    for split in attached_grammar::parse_attached_ability_splits_tokens(&ability_tokens) {
        if parse_ability_line(split.keyword_tokens).is_some()
            && parse_attached_granted_activated_line(split.granted_tokens)?.is_some()
        {
            return Ok(None);
        }
    }

    let Some(parsed) = parse_attached_granted_activated_line(ability_tokens_raw)? else {
        return Ok(None);
    };

    Ok(Some(StaticAbilityAst::AttachedObjectAbilityGrant {
        ability: parsed,
        display: format!(
            "{} has {}",
            shape.subject.display(),
            display_text_for_tokens(&ability_tokens, true)
        ),
        condition: None,
    }))
}
