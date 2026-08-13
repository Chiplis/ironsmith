use super::*;

fn parse_builtin_token_kind<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> WResult<BuiltinTokenShape> {
    alt((
        alt((
            primitives::kw("treasure").value(BuiltinTokenShape::Treasure),
            primitives::kw("clue").value(BuiltinTokenShape::Clue),
            primitives::kw("map").value(BuiltinTokenShape::Map),
            primitives::kw("lander").value(BuiltinTokenShape::Lander),
            primitives::kw("junk").value(BuiltinTokenShape::Junk),
        )),
        alt((
            primitives::kw("mutagen").value(BuiltinTokenShape::Mutagen),
            primitives::kw("gold").value(BuiltinTokenShape::Gold),
            primitives::kw("shard").value(BuiltinTokenShape::Shard),
            primitives::kw("walker").value(BuiltinTokenShape::Walker),
            primitives::phrase(&["eldrazi", "spawn"]).value(BuiltinTokenShape::EldraziSpawn),
        )),
        alt((
            primitives::phrase(&["eldrazi", "scion"]).value(BuiltinTokenShape::EldraziScion),
            primitives::kw("food").value(BuiltinTokenShape::Food),
            primitives::phrase(&["wicked", "role"]).value(BuiltinTokenShape::WickedRole),
            primitives::phrase(&["young", "hero", "role"]).value(BuiltinTokenShape::YoungHeroRole),
            primitives::phrase(&["monster", "role"]).value(BuiltinTokenShape::MonsterRole),
        )),
        alt((
            primitives::phrase(&["sorcerer", "role"]).value(BuiltinTokenShape::SorcererRole),
            primitives::phrase(&["royal", "role"]).value(BuiltinTokenShape::RoyalRole),
            primitives::phrase(&["cursed", "role"]).value(BuiltinTokenShape::CursedRole),
            primitives::kw("blood").value(BuiltinTokenShape::Blood),
            primitives::kw("powerstone").value(BuiltinTokenShape::Powerstone),
        )),
    ))
    .parse_next(input)
}

fn parse_dies_create_builtin_token_rule<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> WResult<TokenEmbeddedRuleShape> {
    primitives::phrase(&["when", "this", "token", "dies"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("create").parse_next(input)?;
    let count = alt((
        primitives::kw("a").value(1),
        primitives::kw("an").value(1),
        leaf::parse_leaf_number_prefix_lexed,
    ))
    .parse_next(input)?;
    let token = parse_builtin_token_kind.parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens"))).parse_next(input)?;
    Ok(TokenEmbeddedRuleShape::DiesCreateBuiltinToken { token, count })
}

fn parse_embedded_rule_subject<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> WResult<()> {
    alt((
        primitives::phrase(&["this", "token"]),
        primitives::phrase(&["this", "creature"]),
    ))
    .void()
    .parse_next(input)
}

fn parse_reciprocal_non_subtype_blocking_rule<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> WResult<TokenEmbeddedRuleShape> {
    parse_embedded_rule_subject.parse_next(input)?;
    alt((primitives::kw("cant"), primitives::kw("can't"))).parse_next(input)?;
    primitives::phrase(&["block", "or", "be", "blocked", "by"]).parse_next(input)?;
    let subtype = any
        .verify_map(|token: &OwnedLexToken| {
            leaf::parse_leaf_non_subtype_complete(token.parser_text()).ok()
        })
        .parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures"))).parse_next(input)?;
    Ok(TokenEmbeddedRuleShape::CantBlockOrBeBlockedByNonSubtypeCreatures { subtype })
}

fn parse_damage_triggered_rule<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> WResult<TokenEmbeddedRuleShape> {
    primitives::kw("whenever").parse_next(input)?;
    parse_embedded_rule_subject.parse_next(input)?;
    primitives::kw("deals").parse_next(input)?;
    let combat_only = opt(primitives::kw("combat")).parse_next(input)?.is_some();
    primitives::phrase(&["damage", "to"]).parse_next(input)?;
    let recipient = alt((
        (
            opt(alt((primitives::kw("a"), primitives::kw("the")))),
            primitives::kw("player"),
        )
            .value(false),
        (
            opt(alt((primitives::kw("a"), primitives::kw("the")))),
            primitives::kw("planeswalker"),
        )
            .value(true),
    ))
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;

    if recipient {
        primitives::phrase(&["destroy", "that", "planeswalker"]).parse_next(input)?;
        return Ok(TokenEmbeddedRuleShape::DealsDamageToPlaneswalkerDestroy { combat_only });
    }

    alt((
        (primitives::phrase(&[
            "that", "player", "loses", "the", "game",
        ]),)
            .value(TokenEmbeddedRuleShape::DealsDamageToPlayerLoseGame { combat_only }),
        (
            primitives::phrase(&["that", "player", "gets"]),
            alt((
                alt((primitives::kw("a"), primitives::kw("an"))).value(1),
                leaf::parse_leaf_number_prefix_lexed,
            )),
            primitives::kw("poison"),
            alt((primitives::kw("counter"), primitives::kw("counters"))),
        )
            .map(|(_, count, _, _)| {
                TokenEmbeddedRuleShape::DealsDamageToPlayerPutCounters {
                    combat_only,
                    counter_type: CounterType::Poison,
                    count,
                }
            }),
    ))
    .parse_next(input)
}

fn parse_upkeep_sacrifice_else_damage_rule<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> WResult<TokenEmbeddedRuleShape> {
    primitives::phrase(&["at", "the", "beginning", "of", "your", "upkeep"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["sacrifice", "another", "creature"]).parse_next(input)?;
    primitives::end_of_sentence().parse_next(input)?;
    (
        primitives::kw("if"),
        primitives::kw("you"),
        alt((primitives::kw("cant"), primitives::kw("can't"))),
    )
        .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    parse_embedded_rule_subject.parse_next(input)?;
    primitives::kw("deals").parse_next(input)?;
    let damage = leaf::parse_leaf_number_prefix_lexed.parse_next(input)? as i32;
    primitives::phrase(&["damage", "to", "you"]).parse_next(input)?;
    Ok(
        TokenEmbeddedRuleShape::BeginningOfYourUpkeepSacrificeAnotherCreatureOrSourceDamagesYou {
            damage,
        },
    )
}

fn parse_single_colored_mana_option<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> WResult<ManaSymbol> {
    leaf::parse_leaf_mana_group_token
        .verify(|symbols: &Vec<ManaSymbol>| {
            symbols.len() == 1
                && matches!(
                    symbols[0],
                    ManaSymbol::White
                        | ManaSymbol::Blue
                        | ManaSymbol::Black
                        | ManaSymbol::Red
                        | ManaSymbol::Green
                )
        })
        .map(|symbols| symbols[0])
        .parse_next(input)
}

fn parse_tap_sacrifice_mana_life_rule<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> WResult<TokenEmbeddedRuleShape> {
    parse_tap_symbol.parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["sacrifice", "this", "token"]).parse_next(input)?;
    primitives::colon().parse_next(input)?;
    primitives::kw("add").parse_next(input)?;
    let mana_options =
        separated(2.., parse_single_colored_mana_option, primitives::kw("or")).parse_next(input)?;
    primitives::end_of_sentence().parse_next(input)?;
    primitives::phrase(&["you", "gain"]).parse_next(input)?;
    let life = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::kw("life").parse_next(input)?;
    Ok(TokenEmbeddedRuleShape::TapSacrificeAddManaOrGainLife(
        TokenTapSacrificeManaLifeShape { mana_options, life },
    ))
}

fn parse_tap_sacrifice_any_color_rule<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> WResult<TokenEmbeddedRuleShape> {
    parse_tap_symbol.parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["sacrifice", "this", "token"]).parse_next(input)?;
    primitives::colon().parse_next(input)?;
    primitives::phrase(&["add", "one", "mana", "of", "any", "color"]).parse_next(input)?;
    Ok(TokenEmbeddedRuleShape::TapSacrificeAddManaOfAnyColor)
}

fn parse_land_enters_counter_rule<'a>(
    input: &mut crate::front_end::lexer::LexStream<'a>,
) -> WResult<LandEntersCounterRuleShape<'a>> {
    alt((
        primitives::phrase(&["whenever", "a", "land", "you", "control", "enters"]),
        primitives::phrase(&[
            "whenever", "a", "land", "enters", "under", "your", "control",
        ]),
    ))
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("put").parse_next(input)?;
    let count = alt((
        alt((primitives::kw("a"), primitives::kw("an"))).value(1),
        leaf::parse_leaf_number_prefix_lexed,
    ))
    .parse_next(input)?;
    let descriptor_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))).void(),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    let counter_type =
        filters::parse_counter_type_from_tokens(descriptor_tokens).ok_or_else(|| {
            primitives::backtrack_err("embedded land-entry counter rule", "known counter type")
        })?;
    Ok(LandEntersCounterRuleShape {
        counter_type,
        count,
        target_tokens,
    })
}

fn token_rule_target_is_self(target_tokens: &[OwnedLexToken], named_token: Option<&str>) -> bool {
    let target_words = parser_token_word_refs(target_tokens);
    [
        &["it"][..],
        &["this", "token"][..],
        &["this", "creature"][..],
    ]
    .iter()
    .any(|expected| primitives::parse_word_sequence_complete(&target_words, expected).is_some())
        || named_token.is_some_and(|name| trimmed_render(target_tokens).eq_ignore_ascii_case(name))
}

pub(crate) fn parse_embedded_token_rule_tokens(
    tokens: &[OwnedLexToken],
    named_token: Option<&str>,
) -> Option<TokenEmbeddedRuleShape> {
    let body_tokens = effects::labeled_dispatch::parse_leading_effect_label_tokens(tokens)
        .map(|shape| shape.body_tokens)
        .unwrap_or(tokens);
    for parser in [
        parse_reciprocal_non_subtype_blocking_rule,
        parse_dies_create_builtin_token_rule,
        parse_damage_triggered_rule,
        parse_upkeep_sacrifice_else_damage_rule,
        parse_tap_sacrifice_any_color_rule,
        parse_tap_sacrifice_mana_life_rule,
    ] {
        if let Ok(rule) = primitives::parse_all(
            body_tokens,
            (parser, primitives::sentence_end()).map(|(rule, ())| rule),
            "embedded token rule",
        ) {
            return Some(rule);
        }
    }
    let shape = primitives::parse_all(
        body_tokens,
        (parse_land_enters_counter_rule, primitives::sentence_end()).map(|(shape, ())| shape),
        "embedded land-entry counter rule",
    )
    .ok()?;
    token_rule_target_is_self(shape.target_tokens, named_token).then_some(
        TokenEmbeddedRuleShape::LandEntersPutCountersOnSelf {
            counter_type: shape.counter_type,
            count: shape.count,
        },
    )
}

pub(crate) fn parse_inline_noncreature_spell_damage_tokens(
    tokens: &[OwnedLexToken],
) -> Option<InlineNoncreatureSpellDamageShape> {
    let words = parser_token_word_refs(&tokens);
    let has_cast_trigger =
        common::phrase_present(
            &words,
            &["whenever", "you", "cast", "a", "noncreature", "spell"],
        ) || common::phrase_present(&words, &["whenever", "you", "cast", "noncreature", "spell"]);
    let has_damage_subject = [
        &["this", "token", "deals"][..],
        &["this", "creature", "deals"],
        &["this", "token", "deal"],
        &["this", "creature", "deal"],
        &["it", "deals"],
        &["it", "deal"],
    ]
    .iter()
    .any(|phrase| common::phrase_present(&words, phrase));
    if !has_cast_trigger
        || !has_damage_subject
        || !common::phrase_present(&words, &["to", "each", "opponent"])
    {
        return None;
    }
    Some(InlineNoncreatureSpellDamageShape {
        amount: damage_amount(&words)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;
    use crate::types::Subtype;

    #[test]
    fn reciprocal_non_subtype_blocking_rule_is_typed() {
        let tokens = lex_line(
            "This token can't block or be blocked by non-Spirit creatures.",
            0,
        )
        .expect("reciprocal blocking rule should lex");

        assert_eq!(
            parse_embedded_token_rule_tokens(&tokens, None),
            Some(
                TokenEmbeddedRuleShape::CantBlockOrBeBlockedByNonSubtypeCreatures {
                    subtype: Subtype::Spirit,
                }
            )
        );
    }

    #[test]
    fn reciprocal_rule_parser_does_not_capture_unconditional_blocking_rules() {
        for text in ["This token can't block.", "This token can't be blocked."] {
            let tokens = lex_line(text, 0).expect("unconditional blocking rule should lex");
            assert_eq!(parse_embedded_token_rule_tokens(&tokens, None), None);
        }
    }
}
