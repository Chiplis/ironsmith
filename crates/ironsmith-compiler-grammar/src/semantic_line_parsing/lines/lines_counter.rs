use super::*;

pub(super) fn lower_spell_cast_snow_mana_enter_counter_static_chunk(
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
    intervening_if: Option<&PredicateAst>,
) -> Result<Option<LineAst>, CardTextError> {
    let Some(spec) = semantic_grammar::parse_snow_mana_counter_entry_tokens(
        effect_parse_tokens,
        matches!(
            intervening_if,
            Some(PredicateAst::SnowManaOfAnySpellColorSpentToCastThisSpell)
        ),
    ) else {
        return Ok(None);
    };

    let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
    let Some((mut filter, caster)) = spell_cast_trigger_filter(&trigger) else {
        return Ok(None);
    };
    if !matches!(filter.zone, Some(Zone::Stack))
        || filter.card_types.len() != 1
        || filter.card_types.first().copied() != Some(CardType::Creature)
    {
        return Ok(None);
    }

    filter.zone = Some(Zone::Battlefield);
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    filter.controller = Some(caster);

    let ability = StaticAbility::enters_with_counters_and_subtypes_for_filter(
        filter,
        spec.counter_type,
        spec.count,
        Vec::new(),
    )
    .with_condition(spec.condition);

    Ok(Some(LineAst::StaticAbilities(vec![
        crate::cards::builders::StaticAbilityAst::Static(ability),
    ])))
}

pub(super) fn parse_exiled_last_counter_triggered_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(split) = semantic_grammar::parse_comma_split_tokens(tokens) else {
        return Ok(None);
    };
    if !split
        .before
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        return Ok(None);
    }
    let Some(while_idx) =
        crate::slice_primitives::select_position(split.before, |token| token.is_word("while"))
    else {
        return Ok(None);
    };
    let qualifier_words = crate::lexer::parser_token_word_refs(&split.before[while_idx..]);
    let is_exiled_qualifier = crate::word_primitives::parse_any_sequence_complete(
        &qualifier_words,
        &[
            &["while", "it", "s", "exiled"],
            &["while", "it", "is", "exiled"],
            &["while", "its", "exiled"],
            &["while", "it's", "exiled"],
        ],
    );
    if !is_exiled_qualifier || while_idx <= 1 {
        return Ok(None);
    }

    let trigger = parse_trigger_clause_lexed(&split.before[1..while_idx])?;
    if !matches!(
        &trigger,
        TriggerSpec::CounterRemovedFrom {
            filter,
            last: true,
            ..
        } if filter.source
    ) {
        return Ok(None);
    }
    let effects = parse_effect_sentences_preserving_source_boundaries(split.after)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(LineAst::Triggered {
        trigger,
        effects,
        max_triggers_per_turn: None,
    }))
}

#[cfg(test)]
#[test]
pub(super) fn exiled_last_counter_qualifier_stays_on_the_trigger_side_of_the_comma() {
    let exact = lex_line(
        "When the last time counter is removed from this card while it's exiled, creatures can't be blocked this turn.",
        0,
    )
    .expect("exiled last-counter trigger should lex");
    let parsed = parse_exiled_last_counter_triggered_line(&exact)
        .expect("exiled last-counter trigger should parse")
        .expect("typed exiled qualifier should be recognized");
    let LineAst::Triggered {
        trigger, effects, ..
    } = parsed
    else {
        panic!("expected one triggered line: {parsed:#?}");
    };
    assert!(
        matches!(
            trigger,
            TriggerSpec::CounterRemovedFrom {
                ref filter,
                counter_type: Some(crate::CounterType::Time),
                last: true,
                ..
            } if filter.source
        ),
        "{trigger:#?}"
    );
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::BeBlocked(filter),
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one creature blocking restriction: {effects:#?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature], "{filter:#?}");
    assert!(filter.any_of.is_empty(), "{filter:#?}");

    let near_miss = lex_line(
        "When the last time counter is removed from this card while it's on the battlefield, creatures can't be blocked this turn.",
        0,
    )
    .expect("last-counter near miss should lex");
    assert!(
        parse_exiled_last_counter_triggered_line(&near_miss)
            .expect("near miss should not error")
            .is_none()
    );
}

#[test]
pub(super) fn ability_word_marker_detection_uses_token_kinds() {
    let marker_tokens = lex_line("Landfall", 0).expect("marker should lex");
    assert!(semantic_grammar::parse_ability_word_marker_tokens(&marker_tokens).is_some());

    let sentence_tokens = lex_line(
        "Landfall — Whenever a land enters under your control, draw a card.",
        0,
    )
    .expect("sentence should lex");
    assert!(semantic_grammar::parse_ability_word_marker_tokens(&sentence_tokens).is_none());
}
