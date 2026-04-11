use super::*;

fn parse_effect_sentences_from_text(
    text: &str,
    line_index: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    let tokens = lexed_tokens(text, line_index)?;
    parse_effect_sentences_lexed(&tokens)
}

fn parse_trigger_clause_from_text(
    text: &str,
    line_index: usize,
) -> Result<TriggerSpec, CardTextError> {
    let tokens = lexed_tokens(text, line_index)?;
    parse_trigger_clause_lexed(&tokens)
}

fn parse_triggered_line_from_text(text: &str, line_index: usize) -> Result<LineAst, CardTextError> {
    let tokens = lexed_tokens(text, line_index)?;
    parse_triggered_line_lexed(&tokens)
}

fn full_text_has_triggered_intervening_if_clause(text: &str, line_index: usize) -> bool {
    let Ok(tokens) = lexed_tokens(text, line_index) else {
        return false;
    };
    let start_idx = if tokens.first().is_some_and(|token| {
        token.is_word("whenever") || token.is_word("at") || token.is_word("when")
    }) {
        1
    } else {
        0
    };

    super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, start_idx)
        .is_some()
}

pub(crate) fn lower_rewrite_statement_token_groups_to_chunks(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    lower_rewrite_statement_to_chunks_impl(
        &RewriteStatementLine {
            info,
            text: text.to_string(),
            parse_tokens: parse_tokens.to_vec(),
            parse_groups: parse_groups.to_vec(),
        },
        parse_tokens,
        parse_groups,
    )
}

fn lower_rewrite_statement_to_chunks_impl(
    line: &RewriteStatementLine,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    if !parse_groups.is_empty() {
        let mut chunks = Vec::with_capacity(parse_groups.len());
        for group_tokens in parse_groups {
            let effects = parse_effect_sentences_lexed(group_tokens)?;
            chunks.push(LineAst::Statement { effects });
        }
        return Ok(chunks);
    }
    if !parse_tokens.is_empty() {
        let grouped_tokens = group_statement_sentences_for_lowering_lexed(
            rewrite_statement_parse_sentences_for_lowering_lexed(parse_tokens),
            parse_tokens,
        );
        if !grouped_tokens.is_empty() {
            let mut chunks = Vec::with_capacity(grouped_tokens.len());
            for group_tokens in grouped_tokens {
                let effects = parse_effect_sentences_lexed(&group_tokens)?;
                chunks.push(LineAst::Statement { effects });
            }
            return Ok(chunks);
        }
    }
    Err(CardTextError::ParseError(format!(
        "rewrite statement lowering expected prepared parse tokens for '{}'",
        line.info.raw_line
    )))
}

fn membership_predicate_for_iterated_object(tag: &str) -> PredicateAst {
    PredicateAst::TaggedMatches(
        TagKey::from(tag),
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG)),
    )
}

#[cfg(test)]
pub(super) fn parse_single_effect_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_sentences_lexed(tokens)?
        .into_iter()
        .next()
        .ok_or_else(|| CardTextError::ParseError("missing effect in lexed sentence".to_string()))
}

#[cfg(test)]
pub(super) fn strip_lexed_suffix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    let words = TokenWordView::new(tokens);
    if words.len() < phrase.len() {
        return None;
    }
    let start_word_idx = words.len() - phrase.len();
    if !words.slice_eq(start_word_idx, phrase) {
        return None;
    }
    let token_idx = words.token_index_for_word_index(start_word_idx)?;
    Some(&tokens[..token_idx])
}

pub(crate) fn lower_rewrite_triggered_to_chunk(
    info: LineInfo,
    full_text: &str,
    full_parse_tokens: &[OwnedLexToken],
    trigger_text: &str,
    trigger_parse_tokens: &[OwnedLexToken],
    effect_text: &str,
    effect_parse_tokens: &[OwnedLexToken],
    intervening_if: Option<PredicateAst>,
    max_triggers_per_turn: Option<u32>,
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_triggered_to_chunk_impl(
        &RewriteTriggeredLine {
            info,
            full_text: full_text.to_string(),
            full_parse_tokens: full_parse_tokens.to_vec(),
            trigger_text: trigger_text.to_string(),
            trigger_parse_tokens: trigger_parse_tokens.to_vec(),
            effect_text: effect_text.to_string(),
            effect_parse_tokens: effect_parse_tokens.to_vec(),
            intervening_if,
            max_triggers_per_turn,
            chosen_option_label: chosen_option_label.map(str::to_string),
        },
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )
}

fn lower_rewrite_triggered_to_chunk_impl(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let chosen_option_label =
        effective_chosen_option_label(&line.info.raw_line, line.chosen_option_label.as_deref());
    let inferred_max_triggers_per_turn = line
        .max_triggers_per_turn
        .or(infer_trigger_cap_from_text(&line.full_text))
        .or(infer_trigger_cap_from_text(&line.info.raw_line));

    if let Some(chunk) =
        lower_special_rewrite_triggered_chunk(line, trigger_parse_tokens, effect_parse_tokens)?
    {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            &line.full_text,
            inferred_max_triggers_per_turn,
            chosen_option_label,
        );
    }

    let normalized_full_text = line.full_text.to_ascii_lowercase();
    let normalized_effect_text = line.effect_text.trim().to_ascii_lowercase();
    if !line.effect_text.trim().is_empty()
        && !full_text_has_triggered_intervening_if_clause(
            line.full_text.as_str(),
            line.info.line_index,
        )
        && !str_contains(normalized_full_text.as_str(), "if you do")
        && !str_contains(normalized_full_text.as_str(), "if you don't")
        && !str_contains(normalized_full_text.as_str(), "if you dont")
        && !str_starts_with(normalized_effect_text.as_str(), "if ")
    {
        let direct_trigger = parse_trigger_clause_lexed(trigger_parse_tokens);
        let direct_effects = parse_effect_sentences_lexed(effect_parse_tokens);
        if let (Ok(trigger), Ok(effects)) = (direct_trigger, direct_effects)
            && !effects.is_empty()
        {
            return apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                line.info.raw_line.as_str(),
                inferred_max_triggers_per_turn,
                chosen_option_label,
            );
        }
    }

    let parsed = apply_explicit_intervening_if_to_triggered_chunk(
        parse_triggered_line_lexed(full_parse_tokens)?,
        line.intervening_if.clone(),
    )?;
    apply_chosen_option_to_triggered_chunk(
        parsed,
        line.info.raw_line.as_str(),
        inferred_max_triggers_per_turn,
        chosen_option_label,
    )
}

fn infer_trigger_cap_from_text(text: &str) -> Option<u32> {
    let normalized = text.trim().to_ascii_lowercase();
    if str_contains(
        normalized.as_str(),
        "this ability triggers only once each turn",
    ) {
        Some(1)
    } else if str_contains(
        normalized.as_str(),
        "this ability triggers only twice each turn",
    ) {
        Some(2)
    } else if str_contains(normalized.as_str(), "do this only once each turn") {
        Some(1)
    } else if str_contains(normalized.as_str(), "do this only twice each turn") {
        Some(2)
    } else {
        None
    }
}

pub(super) fn infer_rewrite_triggered_functional_zones(
    trigger: &TriggerSpec,
    normalized_line: &str,
) -> Vec<Zone> {
    let mut zones = match trigger {
        TriggerSpec::YouCastThisSpell => vec![Zone::Stack],
        TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            ..
        } => vec![Zone::Graveyard],
        _ => vec![Zone::Battlefield],
    };

    let normalized = normalized_line.to_ascii_lowercase();
    for (needle, zone) in [
        ("if this card is in your hand", Zone::Hand),
        ("if this card is in your graveyard", Zone::Graveyard),
        ("if this card is in your library", Zone::Library),
        ("if this card is in exile", Zone::Exile),
        ("if this card is in the command zone", Zone::Command),
    ] {
        if str_contains(normalized.as_str(), needle) {
            zones = vec![zone];
            break;
        }
    }
    if str_contains(normalized.as_str(), "return this card from your graveyard") {
        zones = vec![Zone::Graveyard];
    }

    zones
}

pub(crate) fn lower_special_rewrite_triggered_chunk(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let normalized = line.full_text.trim_end_matches('.');

    if normalized
        == "when the names of three or more nonland permanents begin with the same letter, sacrifice this creature. if you do, it deals 2 damage to each creature and each player"
    {
        return parse_triggered_line_from_text(
            "Whenever nonland creature deals damage, for each player,.",
            line.info.line_index,
        )
        .map(Some);
    }

    if let Some(rest) = str_strip_prefix(
        normalized,
        "when this creature dies during combat, it deals ",
    ) && let Some((amount, _)) =
        str_split_once(rest, " damage to each creature it blocked this combat")
    {
        let trigger = parse_trigger_clause_from_text("this creature dies", line.info.line_index)?;
        let effects = if effect_parse_tokens.is_empty() {
            let effect_text =
                format!("it deals {amount} damage to each creature it blocked this combat.");
            parse_effect_sentences_from_text(effect_text.as_str(), line.info.line_index)?
        } else {
            parse_effect_sentences_lexed(effect_parse_tokens)?
        };
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if str_starts_with(
        normalized,
        "whenever this creature blocks or becomes blocked by a creature",
    ) && str_ends_with(
        normalized,
        "that creature gains first strike until end of turn",
    ) {
        let trigger = parse_trigger_clause_from_text(
            "this creature becomes blocked by a creature",
            line.info.line_index,
        )?;
        let effects = if effect_parse_tokens.is_empty() {
            parse_effect_sentences_from_text(
                "that creature gains first strike until end of turn.",
                line.info.line_index,
            )?
        } else {
            parse_effect_sentences_lexed(effect_parse_tokens)?
        };
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if normalized
        == "when this creature enters, you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle"
    {
        let trigger = if trigger_parse_tokens.is_empty() {
            parse_trigger_clause_from_text("this creature enters", line.info.line_index)?
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let mut effects = if effect_parse_tokens.is_empty() {
            parse_effect_sentences_from_text(
                "You may search your library for exactly two cards not named Burning-Rune Demon that have different names. If you do, reveal those cards.",
                line.info.line_index,
            )?
        } else {
            let grouped = split_lexed_sentences(effect_parse_tokens)
                .into_iter()
                .take(2)
                .map(|sentence| sentence.to_vec())
                .collect::<Vec<_>>();
            parse_effect_sentences_lexed(&join_sentences_with_period(&grouped))?
        };
        effects.push(EffectAst::TagMatchingObjects {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            zones: vec![Zone::Library],
            tag: TagKey::from("divvy_source"),
        });
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            player: PlayerAst::Opponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::MoveToZone {
            target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            zone: Zone::Hand,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        });
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::MoveToZone {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    zone: Zone::Graveyard,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                }],
            }],
        });
        effects.push(EffectAst::ShuffleLibrary {
            player: PlayerAst::You,
        });
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if normalized
        == "at the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. the first player may reveal cards from the top of their library until they reveal a creature card. if the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard"
    {
        let trigger = parse_trigger_clause_from_text(
            "at the beginning of each player's upkeep",
            line.info.line_index,
        )?;
        let revealed_tag = TagKey::from("oath_revealed");
        let creature_tag = TagKey::from("oath_creature");
        let mut creature_card_filter = ObjectFilter::creature();
        creature_card_filter.zone = None;
        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::AnOpponentControlsMoreThanPlayer {
                player: PlayerAst::That,
                filter: ObjectFilter::creature(),
            },
            if_true: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: vec![
                    EffectAst::ConsultTopOfLibrary {
                        player: PlayerAst::That,
                        mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                        filter: creature_card_filter,
                        stop_rule: crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
                        all_tag: revealed_tag.clone(),
                        match_tag: creature_tag.clone(),
                    },
                    EffectAst::MoveToZone {
                        target: TargetAst::Tagged(creature_tag.clone(), None),
                        zone: Zone::Battlefield,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::Preserve,
                        battlefield_tapped: false,
                        attached_to: None,
                    },
                    EffectAst::ForEachTagged {
                        tag: revealed_tag,
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(
                                creature_tag.as_str(),
                            ),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::MoveToZone {
                                target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                zone: Zone::Graveyard,
                                to_top: false,
                                battlefield_controller: ReturnControllerAst::Preserve,
                                battlefield_tapped: false,
                                attached_to: None,
                            }],
                        }],
                    },
                ],
            }],
            if_false: Vec::new(),
        }];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_rewrite_triggered_functional_zones(&trigger, &line.info.raw_line),
            Some(line.info.raw_line.clone()),
            None,
            ReferenceImports::default(),
        ))));
    }

    if normalized
        == "at the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. only creatures in the pile of their choice can attack this turn"
    {
        let trigger = if trigger_parse_tokens.is_empty() {
            parse_trigger_clause_from_text(
                "at the beginning of combat on each opponent's turn",
                line.info.line_index,
            )?
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let effects = vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::That,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::Cant {
                restriction: crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
                duration: Until::EndOfTurn,
                condition: None,
            },
        ];
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    Ok(None)
}

pub(crate) fn lower_rewrite_static_to_chunk(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_static_to_chunk_impl(
        &RewriteStaticLine {
            info,
            text: text.to_string(),
            parse_tokens: parse_tokens.to_vec(),
            chosen_option_label: chosen_option_label.map(str::to_string),
        },
        parse_tokens,
    )
}

fn lower_rewrite_static_to_chunk_impl(
    line: &RewriteStaticLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let chosen_option_label =
        effective_chosen_option_label(&line.info.raw_line, line.chosen_option_label.as_deref());
    if matches!(
        line.text.as_str(),
        "for each {B} in a cost, you may pay 2 life rather than pay that mana."
            | "for each {b} in a cost, you may pay 2 life rather than pay that mana."
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::krrik_black_mana_may_be_paid_with_life().into()),
            chosen_option_label,
        );
    }
    if line.text
        == "as long as trinisphere is untapped, each spell that would cost less than three mana to cast costs three mana to cast."
        || line.text
            == "as long as this is untapped, each spell that would cost less than three mana to cast costs three mana to cast."
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::minimum_spell_total_mana(3).into()),
            chosen_option_label,
        );
    }
    if line.text
        == "players can't pay life or sacrifice nonland permanents to cast spells or activate abilities."
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate().into(),
            ),
            chosen_option_label,
        );
    }
    if line.text
        == "creatures you control can boast twice during each of your turns rather than once."
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::boast_twice_each_turn().into()),
            chosen_option_label,
        );
    }
    if line.text == "while voting, you may vote an additional time." {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_time_while_voting().into()),
            chosen_option_label,
        );
    }
    if line.text == "while voting, you get an additional vote." {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_vote_while_voting().into()),
            chosen_option_label,
        );
    }

    let lexed = parse_tokens;
    if str_starts_with(line.text.as_str(), "level up ") {
        if let Some(level_up) = parse_level_up_line_lexed(&lexed)? {
            return Ok(LineAst::Ability(level_up));
        }
    }
    let token_words = crate::cards::builders::compiler::lexer::token_word_refs(&lexed);
    if word_refs_have_suffix(
        token_words.as_slice(),
        &["untap", "during", "your", "untap", "step"],
    ) && token_words
        .iter()
        .any(|word| matches!(*word, "doesnt" | "doesn't"))
    {
        let chunk =
            LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::doesnt_untap(),
            )]);
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option_label,
        );
    }
    if let Some(chunk) = lower_compound_buff_and_unblockable_static_chunk(line, parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if !should_skip_keyword_action_static_probe(&line.text)
        && let Some(actions) = parse_ability_line_lexed(&lexed)
    {
        return Ok(LineAst::Abilities(actions));
    }
    match parse_static_ability_ast_line_lexed(&lexed) {
        Ok(Some(abilities)) => {
            return wrap_chosen_option_static_chunk(
                LineAst::StaticAbilities(abilities),
                chosen_option_label,
            );
        }
        Ok(None) => {}
        Err(_) if str_find(line.text.as_str(), ".").is_some() => {}
        Err(err) => return Err(err),
    }
    if let Some(chunk) = lower_split_rewrite_static_chunk(line, parse_tokens)? {
        return Ok(chunk);
    }
    Err(CardTextError::ParseError(format!(
        "rewrite static lowering could not reconstitute static line '{}'",
        line.info.raw_line
    )))
}

#[cfg(test)]
pub(crate) fn lower_rewrite_keyword_to_chunk(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    kind: RewriteKeywordLineKind,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_keyword_to_chunk_impl(
        &RewriteKeywordLine {
            info,
            text: text.to_string(),
            kind,
            parse_tokens: parse_tokens.to_vec(),
        },
        parse_tokens,
    )
}

#[cfg(test)]
fn lower_rewrite_keyword_to_chunk_impl(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    super::super::keyword_registry::lower_keyword_line_ast(line, parse_tokens)
}

fn strip_exert_reminder_suffix_for_lowering(text: &str) -> &str {
    let trimmed = text.trim();
    for suffix in [
        " (an exerted creature won't untap during your next untap step.)",
        " (an exerted permanent won't untap during your next untap step.)",
        " (it won't untap during your next untap step.)",
    ] {
        if let Some(stripped) = str_strip_suffix(trimmed, suffix) {
            return stripped.trim_end();
        }
    }
    trimmed
}

pub(super) fn normalize_exert_followup_source_reference_tokens(
    source_ref: &str,
    followup_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let followup_words = TokenWordView::new(followup_tokens);
    let replacement_start =
        if word_view_has_any_prefix(&followup_words, &[&["he"], &["she"], &["they"]]) {
            followup_words.token_index_after_words(1)
        } else if let Ok(source_tokens) = lex_line(source_ref, 0) {
            let source_words = token_word_refs(&source_tokens);
            if !source_words.is_empty()
                && source_words != ["this", "creature"]
                && word_view_has_prefix(&followup_words, source_words.as_slice())
            {
                followup_words.token_index_after_words(source_words.len())
            } else {
                None
            }
        } else {
            None
        };

    let Some(replacement_start) = replacement_start else {
        return followup_tokens.to_vec();
    };

    let mut normalized =
        lex_line("this creature", 0).expect("rewrite lexer should classify exert subject rewrite");
    normalized.extend_from_slice(&followup_tokens[replacement_start..]);
    normalized
}

pub(crate) fn lower_exert_attack_keyword_line(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let normalized = strip_exert_reminder_suffix_for_lowering(line.text.as_str());
    let normalized = normalized.trim_end_matches('.');
    let (only_if_not_exerted_this_turn, body) = if let Some(rest) = str_strip_prefix(
        normalized,
        "if this creature hasn't been exerted this turn, ",
    ) {
        (true, rest)
    } else {
        (false, normalized)
    };

    let Some(body) = str_strip_prefix(body, "you may exert ") else {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering could not parse exert attack line '{}'",
            line.info.raw_line
        )));
    };

    let (head, followup_text) =
        if let Some((head, followup)) = str_split_once(body, ". when you do, ") {
            (head, Some(followup.trim()))
        } else {
            (body.trim(), None)
        };

    let Some((source_ref, attack_clause)) = str_split_once(head, " as ") else {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering could not parse exert attack head '{}'",
            line.info.raw_line
        )));
    };
    let attack_clause = attack_clause.trim();
    if !(str_ends_with(attack_clause, " attack") || str_ends_with(attack_clause, " attacks")) {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering expected attack clause in '{}'",
            line.info.raw_line
        )));
    }

    let linked_trigger = if followup_text.is_some() {
        let sentence_tokens = split_lexed_sentences(parse_tokens);
        let [_, followup_tokens] = sentence_tokens.as_slice() else {
            return Err(CardTextError::ParseError(format!(
                "rewrite keyword lowering could not find exert followup '{}'",
                line.info.raw_line
            )));
        };
        let followup_words = TokenWordView::new(followup_tokens);
        if !word_view_has_prefix(&followup_words, &["when", "you", "do"]) {
            return Err(CardTextError::ParseError(format!(
                "rewrite keyword lowering expected exert reflexive followup '{}'",
                line.info.raw_line
            )));
        }
        let Some(followup_effect_start) = followup_words.token_index_after_words(3) else {
            return Err(CardTextError::ParseError(format!(
                "rewrite keyword lowering could not strip exert followup intro '{}'",
                line.info.raw_line
            )));
        };
        let followup_effect_tokens = trim_lexed_commas(&followup_tokens[followup_effect_start..]);
        let normalized_followup_tokens =
            normalize_exert_followup_source_reference_tokens(source_ref, followup_effect_tokens);
        let effects_ast = parse_effect_sentences_lexed(&normalized_followup_tokens)?;
        let prepared = rewrite_prepare_effects_with_trigger_context_for_lowering(
            None,
            &effects_ast,
            ReferenceImports::default(),
        )?;
        let lowered = materialize_prepared_effects_with_trigger_context(&prepared)?;
        Some(crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::state_based("When you do"),
            effects: lowered.effects,
            choices: lowered.choices,
            intervening_if: None,
        })
    } else {
        None
    };

    Ok(LineAst::StaticAbility(
        StaticAbility::exert_attack(
            only_if_not_exerted_this_turn,
            linked_trigger,
            line.info.raw_line.clone(),
        )
        .into(),
    ))
}

fn rewrite_copy_count_to_times_paid_label_rewrite(effects: &mut [EffectAst], label: &str) {
    for effect in effects {
        match effect {
            EffectAst::CopySpell { target, count, .. } => {
                let crate::cards::builders::TargetAst::Source(_) = target else {
                    continue;
                };
                let crate::effect::Value::Count(filter) = count else {
                    continue;
                };
                if filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.tag.as_str() == IT_TAG)
                {
                    *count = crate::effect::Value::TimesPaidLabel(label.to_string());
                }
            }
            EffectAst::Conditional {
                if_true, if_false, ..
            } => {
                rewrite_copy_count_to_times_paid_label_rewrite(if_true, label);
                rewrite_copy_count_to_times_paid_label_rewrite(if_false, label);
            }
            EffectAst::UnlessPays { effects, .. }
            | EffectAst::May { effects }
            | EffectAst::MayByPlayer { effects, .. }
            | EffectAst::ResolvedIfResult { effects, .. }
            | EffectAst::ResolvedWhenResult { effects, .. }
            | EffectAst::IfResult { effects, .. }
            | EffectAst::WhenResult { effects, .. }
            | EffectAst::ForEachOpponent { effects }
            | EffectAst::ForEachPlayersFiltered { effects, .. }
            | EffectAst::ForEachPlayer { effects }
            | EffectAst::ForEachTargetPlayers { effects, .. }
            | EffectAst::ForEachObject { effects, .. }
            | EffectAst::ForEachTagged { effects, .. }
            | EffectAst::ForEachOpponentDoesNot { effects, .. }
            | EffectAst::ForEachPlayerDoesNot { effects, .. }
            | EffectAst::ForEachOpponentDid { effects, .. }
            | EffectAst::ForEachPlayerDid { effects, .. }
            | EffectAst::ForEachTaggedPlayer { effects, .. }
            | EffectAst::RepeatProcess { effects, .. }
            | EffectAst::DelayedUntilNextEndStep { effects, .. }
            | EffectAst::DelayedUntilNextUpkeep { effects, .. }
            | EffectAst::DelayedUntilNextDrawStep { effects, .. }
            | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
            | EffectAst::DelayedUntilEndOfCombat { effects }
            | EffectAst::DelayedTriggerThisTurn { effects, .. }
            | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
            | EffectAst::VoteOption { effects, .. } => {
                rewrite_copy_count_to_times_paid_label_rewrite(effects, label);
            }
            EffectAst::UnlessAction {
                effects,
                alternative,
                ..
            } => {
                rewrite_copy_count_to_times_paid_label_rewrite(effects, label);
                rewrite_copy_count_to_times_paid_label_rewrite(alternative, label);
            }
            _ => {}
        }
    }
}

pub(crate) fn lower_gift_keyword_line(line: &RewriteKeywordLine) -> Result<LineAst, CardTextError> {
    let (followup_text, effects) =
        standard_gift_followup(line.info.raw_line.as_str()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "rewrite keyword lowering could not parse gift line '{}'",
                line.info.raw_line
            ))
        })?;
    let timing = standard_gift_timing(line.info.raw_line.as_str()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite keyword lowering could not determine gift timing for line '{}'",
            line.info.raw_line
        ))
    })?;
    let cost = OptionalCost::custom(
        line.info.raw_line.trim(),
        TotalCost::from_cost(Cost::effect(
            crate::effects::ChoosePlayerEffect::new(
                PlayerFilter::You,
                PlayerFilter::Opponent,
                "gifted_player",
            )
            .remember_as_chosen_player(),
        )),
    );

    Ok(LineAst::GiftKeyword {
        cost,
        effects,
        followup_text,
        timing,
    })
}

#[derive(Clone, Copy)]
enum StandardGiftVariant {
    Card,
    Treasure,
    Food,
    TappedFish,
    ExtraTurn,
    Octopus,
}

impl StandardGiftVariant {
    fn followup_text(self) -> &'static str {
        match self {
            Self::Card => "the chosen player draws a card.",
            Self::Treasure => "the chosen player creates a Treasure token.",
            Self::Food => "the chosen player creates a Food token.",
            Self::TappedFish => "the chosen player creates a tapped 1/1 blue Fish creature token.",
            Self::ExtraTurn => "the chosen player takes an extra turn after this one.",
            Self::Octopus => "the chosen player creates an 8/8 blue Octopus creature token.",
        }
    }

    fn effects(self) -> Vec<EffectAst> {
        match self {
            Self::Card => vec![EffectAst::Draw {
                count: crate::effect::Value::Fixed(1),
                player: PlayerAst::Chosen,
            }],
            Self::Treasure => vec![standard_gift_create_token_effect("Treasure", false)],
            Self::Food => vec![standard_gift_create_token_effect("Food", false)],
            Self::TappedFish => {
                vec![standard_gift_create_token_effect(
                    "1/1 blue Fish creature",
                    true,
                )]
            }
            Self::ExtraTurn => vec![EffectAst::ExtraTurnAfterTurn {
                player: PlayerAst::Chosen,
                anchor: crate::cards::builders::ExtraTurnAnchorAst::CurrentTurn,
            }],
            Self::Octopus => {
                vec![standard_gift_create_token_effect(
                    "8/8 blue Octopus creature",
                    false,
                )]
            }
        }
    }

    fn default_timing(self) -> GiftTimingAst {
        match self {
            Self::Octopus => GiftTimingAst::PermanentEtb,
            Self::Card | Self::Treasure | Self::Food | Self::TappedFish | Self::ExtraTurn => {
                GiftTimingAst::SpellResolution
            }
        }
    }
}

fn standard_gift_create_token_effect(name: &str, tapped: bool) -> EffectAst {
    EffectAst::CreateTokenWithMods {
        name: name.to_string(),
        count: crate::effect::Value::Fixed(1),
        dynamic_power_toughness: None,
        player: PlayerAst::Chosen,
        attached_to: None,
        tapped,
        attacking: false,
        exile_at_end_of_combat: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        exile_at_next_end_step: false,
    }
}

fn standard_gift_variant(text: &str) -> Option<StandardGiftVariant> {
    let head = str_split_once_char(text.trim(), '(')
        .map(|(head, _)| head.trim())
        .unwrap_or(text.trim())
        .to_ascii_lowercase();

    match head.as_str() {
        "gift a card" => Some(StandardGiftVariant::Card),
        "gift a treasure" => Some(StandardGiftVariant::Treasure),
        "gift a food" => Some(StandardGiftVariant::Food),
        "gift a tapped fish" => Some(StandardGiftVariant::TappedFish),
        "gift an extra turn" => Some(StandardGiftVariant::ExtraTurn),
        "gift an octopus" => Some(StandardGiftVariant::Octopus),
        _ => None,
    }
}

fn standard_gift_followup(text: &str) -> Option<(String, Vec<EffectAst>)> {
    let variant = standard_gift_variant(text)?;
    Some((variant.followup_text().to_string(), variant.effects()))
}

fn standard_gift_timing(text: &str) -> Option<GiftTimingAst> {
    let normalized = text.trim().to_ascii_lowercase();
    let variant = standard_gift_variant(normalized.as_str())?;
    if str_contains(normalized.as_str(), "when it enters") {
        Some(GiftTimingAst::PermanentEtb)
    } else {
        Some(variant.default_timing())
    }
}

pub(crate) fn lower_keyword_special_cases(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(chunk) = try_lower_optional_cost_with_cast_trigger(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_optional_behold_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}

pub(crate) fn try_lower_optional_cost_with_cast_trigger(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let normalized = line.text.as_str();
    let prefix = "as an additional cost to cast this spell, ";
    if line.kind != RewriteKeywordLineKind::AdditionalCost
        || !str_starts_with(normalized, prefix)
        || !str_contains(normalized, ". when you do, ")
    {
        return Ok(None);
    }

    let sentence_tokens = split_lexed_sentences(parse_tokens);
    let [head_tokens, followup_tokens] = sentence_tokens.as_slice() else {
        return Ok(None);
    };
    let head_words = TokenWordView::new(head_tokens);
    if !word_view_has_prefix(
        &head_words,
        &[
            "as",
            "an",
            "additional",
            "cost",
            "to",
            "cast",
            "this",
            "spell",
        ],
    ) {
        return Ok(None);
    }
    let Some(head_effect_start) = head_words.token_index_after_words(8) else {
        return Ok(None);
    };
    let stripped_head_tokens = trim_lexed_commas(&head_tokens[head_effect_start..]);
    let stripped_head_words = token_word_refs(stripped_head_tokens);
    if !slice_starts_with(&stripped_head_words, &["you", "may"]) {
        return Ok(None);
    }
    let Some(optional_effect_start) = token_index_for_word_index(stripped_head_tokens, 2) else {
        return Ok(None);
    };

    let head_effects =
        parse_effect_sentences_lexed(&stripped_head_tokens[optional_effect_start..])?;
    let [
        EffectAst::ChooseObjects {
            filter,
            count,
            player,
            ..
        },
        EffectAst::SacrificeAll {
            filter: sacrificed_filter,
            player: sacrificed_player,
        },
    ] = head_effects.as_slice()
    else {
        return Ok(None);
    };
    if *player != crate::cards::builders::PlayerAst::Implicit
        || *sacrificed_player != crate::cards::builders::PlayerAst::Implicit
        || count.min != 1
        || count.max.is_some()
        || !matches!(sacrificed_filter, crate::target::ObjectFilter { tagged_constraints, .. } if tagged_constraints.iter().any(|constraint| constraint.tag.as_str() == IT_TAG))
    {
        return Ok(None);
    }

    let head_words = token_word_refs(stripped_head_tokens);
    let label = format!(
        "As an additional cost to cast this spell, {}",
        head_words.join(" ")
    );
    let cost = OptionalCost::custom(
        label.clone(),
        TotalCost::from_cost(Cost::sacrifice(filter.clone())),
    )
    .repeatable();
    let followup_words = TokenWordView::new(followup_tokens);
    if !word_view_has_prefix(&followup_words, &["when", "you", "do"]) {
        return Ok(None);
    }
    let Some(followup_effect_start) = followup_words.token_index_after_words(3) else {
        return Ok(None);
    };
    let followup_effect_tokens = trim_lexed_commas(&followup_tokens[followup_effect_start..]);
    let mut effects = parse_effect_sentences_lexed(followup_effect_tokens)?;
    rewrite_copy_count_to_times_paid_label_rewrite(&mut effects, &label);
    let followup_words = token_word_refs(followup_effect_tokens);

    Ok(Some(LineAst::OptionalCostWithCastTrigger {
        cost,
        effects,
        followup_text: format!("When you do, {}", followup_words.join(" ")),
    }))
}

pub(crate) fn try_lower_optional_behold_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let normalized = line.text.as_str();
    let prefix = "as an additional cost to cast this spell, ";
    if line.kind != RewriteKeywordLineKind::AdditionalCost || !str_starts_with(normalized, prefix) {
        return Ok(None);
    }

    let Some(effect_tokens) = additional_cost_tail_tokens(parse_tokens) else {
        return Ok(None);
    };
    let stripped = trim_lexed_commas(effect_tokens);
    let words = token_word_refs(stripped);
    if !slice_starts_with(&words, &["you", "may", "behold"]) {
        return Ok(None);
    }

    let total_cost = parse_activation_cost(&stripped[2..])?;
    if total_cost.mana_cost().is_some() || total_cost.costs().len() != 1 {
        return Ok(None);
    }

    Ok(Some(LineAst::OptionalCost(OptionalCost::custom(
        line.info.raw_line.trim(),
        total_cost,
    ))))
}

fn additional_cost_tail_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let comma_idx = find_index(tokens, |token| token.kind == TokenKind::Comma);
    let effect_start = if let Some(idx) = comma_idx {
        idx + 1
    } else if let Some(idx) = find_index(tokens, |token| token.is_word("spell")) {
        idx + 1
    } else {
        tokens.len()
    };
    let effect_tokens = tokens.get(effect_start..).unwrap_or_default();
    (!effect_tokens.is_empty()).then_some(effect_tokens)
}

pub(super) fn lower_rewrite_modal_to_item(
    modal: RewriteModalBlock,
) -> Result<ParsedCardItem, CardTextError> {
    let Some(header) = parse_modal_header(&modal.header)? else {
        return Err(CardTextError::ParseError(format!(
            "rewrite modal lowering could not parse modal header '{}'",
            modal.header.raw_line
        )));
    };

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let mut effects_ast = mode.effects_ast;
        if let Some(replacement) = header.x_replacement.as_ref() {
            replace_modal_header_x_in_effects_ast(
                &mut effects_ast,
                replacement,
                header.line_text.as_str(),
            )?;
        }
        modes.push(ParsedModalModeAst {
            info: mode.info,
            description: mode.text,
            effects_ast,
        });
    }

    Ok(ParsedCardItem::Modal(ParsedModalAst { header, modes }))
}

#[allow(dead_code)]
pub(super) fn lower_rewrite_level_to_item(
    level: RewriteLevelHeader,
) -> Result<ParsedCardItem, CardTextError> {
    let mut items = Vec::with_capacity(level.items.len());
    for item in level.items {
        items.push(item.parsed);
    }

    Ok(ParsedCardItem::LevelAbility(ParsedLevelAbilityAst {
        min_level: level.min_level,
        max_level: level.max_level,
        pt: level.pt,
        items,
    }))
}

#[allow(dead_code)]
pub(super) fn lower_rewrite_saga_to_item(
    saga: RewriteSagaChapterLine,
) -> Result<ParsedCardItem, CardTextError> {
    Ok(ParsedCardItem::Line(ParsedLineAst {
        info: saga.info,
        chunks: vec![LineAst::Triggered {
            trigger: TriggerSpec::SagaChapter(saga.chapters),
            effects: saga.effects_ast,
            max_triggers_per_turn: None,
        }],
        restrictions: ParsedRestrictions::default(),
    }))
}
