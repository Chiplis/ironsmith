use super::*;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = crate::grammar::keyword_static_lines::parse_additive_damage_amount_tokens(tokens)
    else { return Ok(None); };
    if !shape.this_turn { return Ok(None); }
    let Some(spec) = crate::keyword_static::damage_addition_parts_from_shape(shape)?
    else { return Ok(None); };
    Ok(Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
        SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamageAddition { spec }),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolving_addition_captures_x_or_fixed_and_complete_source_recipient_domains() {
        for (text, expected, opponent, noncombat) in [
            ("If a source you control would deal noncombat damage to a permanent or player this turn, it deals that much damage plus X instead.", Value::X, false, true),
            ("If a red source you control would deal damage to a permanent or player this turn, it deals that much damage plus 2 instead.", Value::Fixed(2), false, false),
            ("If a source would deal damage to a player or battle this turn, it deals that much damage plus 2 instead.", Value::Fixed(2), false, false),
            ("If a source you control would deal damage to an opponent this turn, it deals that much damage plus 3 to that player instead.", Value::Fixed(3), true, false),
        ] {
            let tokens = crate::lexer::lex_line(text,0).unwrap();
            let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamageAddition { spec }), .. }) = parse(&tokens).unwrap().unwrap() else { panic!("wrong registration"); };
            assert_eq!(spec.delta.unhinted(), &expected);
            assert_eq!(spec.noncombat_only, noncombat);
            assert_eq!(spec.mode, ironsmith_core::ReplacementApplyMode::UntilEndOfTurn);
            if opponent { assert_eq!(spec.target_player_filter,Some(PlayerFilter::Opponent)); }
        }
    }
    #[test]
    fn unrelated_recipient_or_tail_is_not_silently_dropped() {
        for text in [
            "If a source would deal damage to an opponent this turn, it deals that much damage plus 2 to that creature instead.",
            "If a source would deal damage to a player this turn, it deals that much damage plus 2 instead and draw a card.",
        ] {
            let tokens = crate::lexer::lex_line(text,0).unwrap();
            assert!(parse(&tokens).unwrap().is_none());
        }
    }
    #[test]
    fn static_bonus_expressions_are_complete_and_live_not_announced_x() {
        for text in [
            "If a source you control would deal damage to an opponent or a permanent an opponent controls, it deals that much damage plus an amount of damage equal to the number of fire counters on this enchantment instead.",
            "If a source you control would deal noncombat damage to an opponent or a permanent an opponent controls, instead it deals that much damage plus X, where X is this creature's power.",
        ] {
            let tokens=crate::lexer::lex_line(text,0).unwrap();
            assert!(parse(&tokens).unwrap().is_none(), "not a resolving duration");
            let ability=crate::keyword_static::parse_damage_amount_replacement_line(&tokens).unwrap().unwrap();
            let ironsmith_core::StaticAbilityPayload::ModifyDamageAmountReplacement { dynamic_delta: Some(value), .. } = ability.payload else { panic!("not a live typed bonus"); };
            assert!(!matches!(value.unhinted(), Value::X));
        }
    }

}
