use crate::cards::builders::SourcePredicateAst;
use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::DelayedEffectAst;
use super::*;

pub(super) fn parse_day_night_starts_day_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    let rendered = render_token_slice(tokens);
    semantic_grammar::parse_day_night_starts_day_tokens(tokens).map(|_| {
        LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
            StaticAbility::rule_fallback_text(rendered.trim().trim_end_matches('.').to_string()),
        )])
    })
}

pub fn parse_static_line(
    info: LineInfo,
    parse_tokens: &[OwnedLexToken],
    chosen_option: Option<&ChosenOptionContext>,
) -> Result<LineAst, CardTextError> {
    parse_static_line_impl(
        &RewriteStaticLine {
            info,
            parse_tokens: parse_tokens.to_vec(),
            chosen_option: chosen_option.cloned(),
        },
        parse_tokens,
    )
}

use crate::recognition::ParseOutcome;
#[path = "lines_ability/static_line_readings.rs"]
mod static_line_readings;

pub(super) fn parse_static_line_impl(
    line: &RewriteStaticLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let chosen_option = line.chosen_option.as_ref();
    if crate::grammar::abilities::is_cast_as_though_flash_with_next_cleanup_sacrifice_line_lexed(
        parse_tokens,
    ) {
        let sacrifice_source =
            EffectAst::subject_verb_sacrifice(PlayerAst::You, ObjectFilter::source(), 1, None);
        return wrap_chosen_option_static_chunk(
            LineAst::Multiple(vec![
                LineAst::StaticAbility(
                    StaticAbility::flash()
                        .with_text("You may cast this spell as though it had flash")
                        .into(),
                ),
                LineAst::Statement {
                    effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                        predicate: PredicateAst::And(
                            Box::new(PredicateAst::Source(SourcePredicateAst::SourceWasCast)),
                            Box::new(PredicateAst::Not(Box::new(
                                PredicateAst::ThisSpellWasCastAtSorceryTiming,
                            ))),
                        ),
                        if_true: vec![EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextCleanupStep {
                            player: PlayerFilter::Any,
                            effects: vec![sacrifice_source],
                        })],
                        if_false: Vec::new(),
                    })],
                },
            ]),
            chosen_option,
        );
    }
    if let Some(prototype) = crate::grammar::abilities::parse_prototype_keyword_tokens(parse_tokens)
    {
        return wrap_chosen_option_static_chunk(
            LineAst::Abilities(vec![KeywordAction::Prototype {
                cost: prototype.cost,
                power_toughness: prototype.power_toughness,
            }]),
            chosen_option,
        );
    }
    let source_partner_label =
        keyword_special_grammar::parse_partner_visible_label_tokens(&line.info.source_tokens);
    if let Some(visible_label) = source_partner_label
        .or_else(|| keyword_special_grammar::parse_partner_visible_label_tokens(&line.parse_tokens))
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::partner().with_text(visible_label).into()),
            chosen_option,
        );
    }
    if let Some(variant) = semantic_grammar::parse_partner_variant_label_tokens(&line.parse_tokens)
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::partner_variant(variant.display).into()),
            chosen_option,
        );
    }
    let special_shape = semantic_grammar::parse_static_special_line_tokens(parse_tokens);
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::BlackManaMayBePaidWithLife)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::krrik_black_mana_may_be_paid_with_life().into()),
            chosen_option,
        );
    }
    if is_minimum_spell_total_mana_three_line_lexed(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::minimum_spell_total_mana(3).into()),
            chosen_option,
        );
    }
    if is_players_cant_pay_life_or_sacrifice_line_lexed(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate().into(),
            ),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::BoastTwice)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::boast_twice_each_turn().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::DraftRule)
    ) {
        let display = render_token_slice(parse_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::draft_rule_text(display).into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::HiddenAgenda)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::hidden_agenda().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::DoubleAgenda)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::double_agenda().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::AnyNumberNamedDeckConstruction)
    ) {
        let display = render_token_slice(parse_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::deck_construction_rule_text(display).into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::FirstEquipCostAlternative)
    ) {
        let display = capitalize_first_equip_cost_alternative_display(parse_tokens);
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::first_equip_cost_alternative(display).into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::EquipAtInstantSpeed)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::equip_abilities_any_time().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::AdditionalVoteTime)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_time_while_voting().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::AdditionalVote)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_vote_while_voting().into()),
            chosen_option,
        );
    }
    if let Some(count) = semantic_grammar::parse_additional_land_play_count_tokens(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::additional_land_plays(count).into()),
            chosen_option,
        );
    }
    if let Some(chunk) = try_lower_hideaway_tokens(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if let Some(chunk) = try_lower_partner_with_tokens(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }

    let lexed = parse_tokens;
    if let Some(abilities) =
        crate::keyword_static::parse_attached_anthem_reach_shadow_permission_line(lexed)
    {
        return wrap_chosen_option_static_chunk(LineAst::StaticAbilities(abilities), chosen_option);
    }
    if semantic_grammar::parse_level_up_intro_tokens(lexed).is_some()
        && let Some(level_up) = parse_level_up_line_lexed(lexed)?
    {
        return Ok(LineAst::Ability(level_up));
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::DoesntUntap)
    ) {
        let chunk =
            LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::doesnt_untap(),
            )]);
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    let input = static_line_readings::StaticLine {
        tokens: parse_tokens,
        line,
        broad_static: Default::default(),
        read_by_cache: Default::default(),
    };
    // A quoted granted ability's static parse error is authoritative: the
    // line is a grant whose quote the static grammar could not read, not a
    // line for the split or keyword fallbacks to lower piecemeal.
    if lexed.iter().any(|token| token.kind == TokenKind::Quote)
        && let Err(error) = input.broad_static()
    {
        return Err(error);
    }
    match static_line_readings::read(&input) {
        ParseOutcome::Match(matched) => {
            return wrap_chosen_option_static_chunk(matched.value.value, chosen_option);
        }
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    // The ability-word marker keeps a keyword-shaped line no grammar reads.
    if let Some(chunk) = static_line_readings::read_ability_word_marker_line(&input)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }

    Err(CardTextError::ParseError(format!(
        "rewrite static lowering could not reconstitute static line '{}'",
        line.info.raw_line
    )))
}

#[cfg(any(test, feature = "test-support"))]
pub fn parse_keyword_line_for_test(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    kind: RewriteKeywordLineKind,
) -> Result<LineAst, CardTextError> {
    parse_keyword_line_with_full_tokens_for_test(info, text, parse_tokens, parse_tokens, kind)
}

#[test]
pub(super) fn standard_menace_reminder_is_typed_without_broad_keyword_expansion() {
    let standard = lex_line(STANDARD_MENACE_REMINDER, 0).expect("standard reminder should lex");
    let bare = lex_line("Menace", 0).expect("bare menace should lex");
    let nonstandard = lex_line(
        "Menace (This creature can't be blocked by only one creature.)",
        0,
    )
    .expect("nonstandard reminder should lex");

    assert!(has_standard_menace_reminder(&standard));
    assert!(!has_standard_menace_reminder(&bare));
    assert!(!has_standard_menace_reminder(&nonstandard));
}

#[test]
pub(super) fn standard_flanking_reminder_is_typed_without_broad_keyword_expansion() {
    assert!(has_standard_flanking_reminder(STANDARD_FLANKING_REMINDER));
    assert!(!has_standard_flanking_reminder("Flanking"));
    assert!(!has_standard_flanking_reminder(
        "Flanking (Whenever a creature without flanking blocks this creature, it gets -1/-1 until end of turn.)"
    ));
}

#[path = "lines_ability/keyword_special_case_readings.rs"]
mod keyword_special_case_readings;

pub fn parse_keyword_special_cases(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let input = keyword_special_case_readings::KeywordSpecialCase {
        tokens: parse_tokens,
        line,
    };
    match keyword_special_case_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }

    Ok(None)
}

pub(super) fn try_lower_partner_variant_keyword(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Option<LineAst> {
    let visible_tokens = if line.full_parse_tokens.is_empty() {
        parse_tokens
    } else {
        line.full_parse_tokens.as_slice()
    };
    let variant = semantic_grammar::parse_partner_variant_label_tokens(visible_tokens)?;
    Some(LineAst::StaticAbility(
        StaticAbility::partner_variant(variant.display).into(),
    ))
}

pub(super) fn try_lower_hideaway_keyword(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    try_lower_hideaway_tokens(parse_tokens)
}
