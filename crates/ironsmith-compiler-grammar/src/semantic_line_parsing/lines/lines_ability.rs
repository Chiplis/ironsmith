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
                    effects: vec![EffectAst::Conditional {
                        predicate: PredicateAst::And(
                            Box::new(PredicateAst::SourceWasCast),
                            Box::new(PredicateAst::Not(Box::new(
                                PredicateAst::ThisSpellWasCastAtSorceryTiming,
                            ))),
                        ),
                        if_true: vec![EffectAst::DelayedUntilNextCleanupStep {
                            player: PlayerFilter::Any,
                            effects: vec![sacrifice_source],
                        }],
                        if_false: Vec::new(),
                    }],
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
    let source_partner_label = crate::util::lex_fragment(&line.info.raw_line, line.info.line_index)
        .and_then(|tokens| keyword_special_grammar::parse_partner_visible_label_tokens(&tokens));
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
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if let Some(ability) = parse_spell_additional_life_cost_per_target_line(lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if let Some(ability) = parse_spell_cost_increase_per_target_beyond_first_line(lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    // A quoted cost modifier is the ability granted by the subject before
    // the quote, not a cost modifier whose spell filter includes that outer
    // subject. The static AST router binds the quoted ability to its grant
    // before the broad cost parser scans the whole line for "spells ... cost".
    // Keep that same precedence at the CST-to-semantic boundary: this is the
    // document path used by ordinary card compilation.
    if lexed.iter().any(|token| token.kind == TokenKind::Quote)
        && let Some(abilities) = parse_static_ability_ast_line_lexed(lexed)?
    {
        return wrap_chosen_option_static_chunk(LineAst::StaticAbilities(abilities), chosen_option);
    }
    if let Some(abilities) = parse_spell_and_player_activated_ability_cost_modifier_line(lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities.into_iter().map(Into::into).collect()),
            chosen_option,
        );
    }
    // Keep a compound spell-cost line intact before the broad single cost
    // modifier parser accepts its left clause and discards the terminal
    // countering restriction. The specialized parser reuses one typed spell
    // filter for both executable static abilities.
    if let Some(abilities) =
        crate::keyword_static::parse_spells_cost_reduction_and_cant_be_countered_line(lexed)?
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities.into_iter().map(Into::into).collect()),
            chosen_option,
        );
    }
    // Preserve a shared first-spell filter across the coordinated reduction
    // and flash permission before the ordinary cost parser consumes only the
    // left side of the sentence.
    if let Some(abilities) =
        crate::keyword_static::parse_first_spell_cost_reduction_and_flash_line(lexed)?
    {
        return wrap_chosen_option_static_chunk(LineAst::StaticAbilities(abilities), chosen_option);
    }
    if let Some(ability) = parse_spells_cost_modifier_line(lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if let Some(chunk) = parse_compound_buff_and_unblockable_static_chunk(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if semantic_grammar::parse_combined_spell_and_activation_tax_tokens(lexed).is_some()
        && let Some(abilities) = parse_static_ability_ast_line_lexed(lexed)?
    {
        return wrap_chosen_option_static_chunk(LineAst::StaticAbilities(abilities), chosen_option);
    }
    if let Some(ability) = crate::keyword_static::parse_double_counters_replacement_line(lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if has_standard_menace_reminder(&line.info.source_tokens)
        && matches!(
            parse_ability_line_lexed(lexed).as_deref(),
            Some([KeywordAction::Menace])
        )
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::menace()
                    .with_text(STANDARD_MENACE_REMINDER)
                    .into(),
            ),
            chosen_option,
        );
    }
    if has_standard_flanking_reminder(&line.info.raw_line)
        && matches!(
            parse_ability_line_lexed(lexed).as_deref(),
            Some([KeywordAction::Flanking])
        )
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::flanking()
                    .with_text(STANDARD_FLANKING_REMINDER)
                    .into(),
            ),
            chosen_option,
        );
    }
    if let Some(actions) = semantic_grammar::parse_source_keyword_tail_tokens(lexed)
        .and_then(|tail| parse_ability_line_lexed(tail.ability_tokens))
    {
        return wrap_chosen_option_static_chunk(LineAst::Abilities(actions), chosen_option);
    }
    if let Some(abilities) = crate::keyword_static::parse_additional_land_play_line(lexed)? {
        let abilities = abilities
            .into_iter()
            .map(crate::cards::builders::StaticAbilityAst::Static)
            .collect();
        return wrap_chosen_option_static_chunk(LineAst::StaticAbilities(abilities), chosen_option);
    }
    // A complete comma-separated keyword line is one authored ability line,
    // even when an individual keyword (for example cascade) also has a
    // specialized static-ability representation. Keep the group provenance
    // before the broad static parser claims each member independently.
    if let Some(actions) = parse_ability_line_lexed(lexed)
        && actions.len() > 1
    {
        return wrap_chosen_option_static_chunk(LineAst::Abilities(actions), chosen_option);
    }
    match parse_static_ability_ast_line_lexed(lexed) {
        Ok(Some(mut abilities)) => {
            restore_copy_static_variant_source_display(&mut abilities, &line.info.raw_line);
            restore_named_characteristic_subject_surface(&mut abilities, &line.info.source_tokens);
            return wrap_chosen_option_static_chunk(
                LineAst::StaticAbilities(abilities),
                chosen_option,
            );
        }
        Ok(None) => {}
        Err(_)
            if parse_tokens
                .iter()
                .any(|token| token.kind == TokenKind::Period) => {}
        Err(err) => return Err(err),
    }
    if semantic_grammar::parse_skip_keyword_action_probe_tokens(parse_tokens).is_none()
        && let Some(actions) = parse_ability_line_lexed(lexed)
    {
        return wrap_chosen_option_static_chunk(LineAst::Abilities(actions), chosen_option);
    }
    if let Some(chunk) = parse_split_static_chunk(line, parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if semantic_grammar::parse_ability_word_marker_tokens(parse_tokens).is_some() {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::keyword_marker(render_token_slice(parse_tokens).trim().to_string())
                    .into(),
            ),
            chosen_option,
        );
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

pub fn parse_keyword_special_cases(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(chunk) = try_lower_hideaway_keyword(parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_partner_variant_keyword(line, parse_tokens) {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_partner_with_tokens(parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_optional_cost_with_cast_trigger(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_chosen_type_behold_two_additional_cost(line, parse_tokens) {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_optional_behold_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_optional_waterbend_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
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
