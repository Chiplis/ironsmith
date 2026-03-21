use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, TagKey, TargetAst, TextSpan, Token,
    Until, parse_target_phrase, parse_value, words,
};

pub(crate) fn parse_prevent_next_damage_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("prevent") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if clause_words.get(idx) == Some(&"the") {
        idx += 1;
    }
    if clause_words.get(idx) != Some(&"next") {
        return Ok(None);
    }
    idx += 1;

    let amount_token = Token::Word(
        clause_words
            .get(idx)
            .copied()
            .unwrap_or_default()
            .to_string(),
        TextSpan::synthetic(),
    );
    let Some((amount, amount_used)) = parse_value(&[amount_token]) else {
        return Err(CardTextError::ParseError(format!(
            "missing prevent damage amount (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    idx += amount_used;

    if clause_words.get(idx) != Some(&"damage") {
        return Ok(None);
    }
    idx += 1;

    if clause_words.get(idx..idx + 4) != Some(["that", "would", "be", "dealt"].as_slice()) {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage clause tail (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 4;

    if clause_words.get(idx) != Some(&"to") {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 1;

    let this_turn_rel = clause_words[idx..]
        .windows(2)
        .position(|window| window == ["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported prevent-next damage duration (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let this_turn_idx = idx + this_turn_rel;
    if this_turn_idx + 2 != clause_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing prevent-next damage clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = &clause_words[idx..this_turn_idx];
    if target_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-next damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target_tokens = target_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(EffectAst::PreventDamage {
        amount,
        target,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_prevent_all_damage_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let prefix_target_then_duration = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
    ];
    let prefix_duration_then_target = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
    ];
    if !clause_words.starts_with(&prefix_target_then_duration)
        && !clause_words.starts_with(&prefix_duration_then_target)
    {
        return Ok(None);
    }
    let target_words = if clause_words.starts_with(&prefix_duration_then_target) {
        &clause_words[prefix_duration_then_target.len()..]
    } else {
        if clause_words.len() <= prefix_target_then_duration.len() + 1 {
            return Err(CardTextError::ParseError(format!(
                "missing prevent-all damage target (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if clause_words[clause_words.len().saturating_sub(2)..] != ["this", "turn"] {
            return Err(CardTextError::ParseError(format!(
                "unsupported prevent-all damage duration (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        &clause_words[prefix_target_then_duration.len()..clause_words.len() - 2]
    };
    if target_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_tokens = target_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(EffectAst::PreventAllDamageToTarget {
        target,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_can_attack_as_though_no_defender_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(can_idx) = clause_words.iter().position(|word| *word == "can") else {
        return Ok(None);
    };
    let subject_words = &clause_words[..can_idx];
    let tail = &clause_words[can_idx..];
    let has_core = tail.starts_with(&["can", "attack"])
        && tail.windows(2).any(|window| window == ["as", "though"])
        && tail.contains(&"turn")
        && tail.contains(&"have")
        && tail.last().copied() == Some("defender");
    if !has_core {
        return Ok(None);
    }

    let target = if subject_words.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        let subject_tokens = subject_words
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        parse_target_phrase(&subject_tokens)?
    };

    Ok(Some(EffectAst::GrantAbilitiesToTarget {
        target,
        abilities: vec![GrantedAbilityAst::CanAttackAsThoughNoDefender],
        duration: Until::EndOfTurn,
    }))
}
