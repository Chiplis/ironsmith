use super::*;

pub fn parse_pay(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let energy_symbol_count = tokens
        .iter()
        .filter(|token| energy_symbol_token(token))
        .count();

    let clause_words = crate::lexer::token_word_refs(tokens);
    if grammar::match_any_word_prefix(tokens, ANY_AMOUNT_OF_PREFIXES).is_some()
        && (grammar::contains_word(tokens, "e") || energy_symbol_count > 0)
    {
        return Ok(EffectAst::subject_verb_pay_any_energy(player, 0));
    }
    if grammar::match_any_word_prefix(tokens, ANY_AMOUNT_OF_PREFIXES).is_some()
        && grammar::contains_word(tokens, "life")
    {
        return Ok(EffectAst::subject_verb_pay_any_life(player, 0));
    }
    if grammar::match_any_word_prefix(tokens, &[&["one", "or", "more"]]).is_some()
        && (grammar::contains_word(tokens, "e") || energy_symbol_count > 0)
    {
        return Ok(EffectAst::subject_verb_pay_any_energy(player, 1));
    }
    if grammar::match_any_word_prefix(tokens, &[&["one", "or", "more"]]).is_some()
        && grammar::contains_word(tokens, "life")
    {
        return Ok(EffectAst::subject_verb_pay_any_life(player, 1));
    }
    if let Some(compound) = parse_compound_pay(tokens, player) {
        return Ok(compound);
    }
    if let Some(repeated) = misc_action_shapes::parse_repeated_tagged_mana_payment_tokens(tokens) {
        // In a clause such as "that player may choose ... and pay {2} for
        // each creature chosen this way", the omitted subject of the payment
        // is the iterated player, not the resolving ability's controller.
        let payer = if player == PlayerAst::Implicit {
            PlayerAst::That
        } else {
            player
        };
        return Ok(EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: vec![EffectAst::subject_verb_pay_mana(
                payer,
                ManaCost::from_pips(repeated.pip_groups),
            )],
        });
    }

    if let Some((for_each_idx, (), _)) =
        grammar::find_prefix(tokens, || grammar::phrase(&["for", "each"]))
        && let Some(parsed_cost) = parse_leaf_mana_cost_prefix_tokens(&tokens[..for_each_idx])
        && parsed_cost.consumed == for_each_idx
        && let [pip] = parsed_cost.cost.pips()
        && let [crate::mana::ManaSymbol::Generic(multiplier)] = pip.as_slice()
    {
        let count_words = crate::lexer::token_word_refs(&tokens[for_each_idx..]);
        if let Some((count, used)) = crate::util::parse_for_each_count_value_words(&count_words)
            && used == count_words.len()
        {
            let count = match *multiplier {
                1 => count,
                multiplier => Value::Scaled(Box::new(count), i32::from(multiplier)),
            }
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
            return Ok(subject_verb_player_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::PayMana {
                    cost: ManaCost::from_symbols(vec![crate::mana::ManaSymbol::X]),
                    x_value: Some(count),
                    x_maximum: None,
                },
            ));
        }
    }

    if clause_words.len() >= 4
        && grammar::contains_word(tokens, "for")
        && grammar::contains_word(tokens, "each")
        && let Ok(symbols) = parse_mana_symbol_group(clause_words[0])
    {
        return Ok(EffectAst::subject_verb_pay_mana(
            player,
            ManaCost::from_pips(vec![symbols]),
        ));
    }

    if let Some(amount) =
        crate::effect_sentences::verb_handlers::parse_half_life_value(tokens, player)
    {
        return Ok(EffectAst::subject_verb_pay_life(player, amount));
    }

    if let Some((amount, used)) = parse_value(tokens)
        && token_slice_at_is(tokens, used, "life")
    {
        return Ok(EffectAst::subject_verb_pay_life(player, amount));
    }
    if let Some((amount, used)) = parse_value(tokens)
        && tokens
            .get(used)
            .is_some_and(|token| token.as_word().is_some_and(|word| word == ENERGY_TEXT_WORD))
    {
        return Ok(EffectAst::subject_verb_pay_energy(player, amount));
    }
    if energy_symbol_count > 0 {
        if let Some((equal_idx, _, _)) =
            grammar::find_prefix(tokens, || grammar::phrase(&["equal", "to"]))
        {
            let amount_tokens = &tokens[equal_idx + 2..];
            if let Some((amount, used)) = parse_value(amount_tokens)
                && used == amount_tokens.len()
            {
                return Ok(EffectAst::subject_verb_pay_energy(player, amount));
            }
            if let Some(amount) = parse_dynamic_cost_modifier_value(amount_tokens)? {
                return Ok(EffectAst::subject_verb_pay_energy(player, amount));
            }
        }
        let mut energy_count = 0u32;
        for token in tokens {
            if energy_symbol_token(token) {
                energy_count += 1;
                continue;
            }
            let Some(word) = token.as_word() else {
                continue;
            };
            if is_article(word) || misc_word_is_any(word, ENERGY_COUNTER_PAY_IGNORED_WORDS) {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported pay clause token '{word}' (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            )));
        }
        if energy_count > 0 {
            return Ok(EffectAst::subject_verb_pay_energy(
                player,
                Value::Fixed(energy_count as i32),
            ));
        }
    }

    let pips = {
        use winnow::prelude::*;
        let mut stream = LexStream::new(tokens);
        grammar::collect_mana_pip_groups
            .parse_next(&mut stream)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "missing payment cost (clause: '{}')",
                    crate::lexer::token_word_refs(tokens).join(" ")
                ))
            })?
    };

    Ok(EffectAst::subject_verb_pay_mana(
        player,
        ManaCost::from_pips(pips),
    ))
}
