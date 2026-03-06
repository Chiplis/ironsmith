use super::*;

const PRE_TOKEN_DIAGNOSTIC_RULES: [UnsupportedRuleDef; 10] = [
    UnsupportedRuleDef {
        id: "commander-cast-count",
        priority: 100,
        heads: &["for"],
        shape_mask: 0,
        message: "unsupported commander-cast-count clause",
        predicate: line_has_commander_cast_count_clause,
    },
    UnsupportedRuleDef {
        id: "verb-leading-spell",
        priority: 110,
        heads: &["sacrifice"],
        shape_mask: 0,
        message: "unsupported verb-leading spell clause",
        predicate: line_has_verb_leading_spell_clause,
    },
    UnsupportedRuleDef {
        id: "choose-leading-spell",
        priority: 120,
        heads: &["choose"],
        shape_mask: 0,
        message: "unsupported choose-leading spell clause",
        predicate: line_has_choose_leading_spell_clause,
    },
    UnsupportedRuleDef {
        id: "put-from-among",
        priority: 130,
        heads: &[],
        shape_mask: 0,
        message: "unsupported put-from-among clause",
        predicate: line_has_put_from_among_clause,
    },
    UnsupportedRuleDef {
        id: "standalone-token-reminder",
        priority: 140,
        heads: &["it"],
        shape_mask: 0,
        message: "unsupported standalone token reminder clause",
        predicate: line_has_standalone_token_reminder_clause,
    },
    UnsupportedRuleDef {
        id: "multi-destination-put",
        priority: 150,
        heads: &["put"],
        shape_mask: 0,
        message: "unsupported multi-destination put clause",
        predicate: line_has_multi_destination_put_clause,
    },
    UnsupportedRuleDef {
        id: "marker-keyword-tail",
        priority: 160,
        heads: &["ninjutsu"],
        shape_mask: 0,
        message: "unsupported marker keyword tail clause",
        predicate: line_has_marker_keyword_tail_clause,
    },
    UnsupportedRuleDef {
        id: "aura-copy-attachment-fanout",
        priority: 170,
        heads: &[],
        shape_mask: 0,
        message: "unsupported aura-copy attachment fanout clause",
        predicate: line_has_aura_copy_attachment_fanout_clause,
    },
    UnsupportedRuleDef {
        id: "defending-players-choice",
        priority: 180,
        heads: &["of", "target"],
        shape_mask: 0,
        message: "unsupported defending-players-choice clause",
        predicate: line_has_defending_players_choice_clause,
    },
    UnsupportedRuleDef {
        id: "first-spell-cost-modifier",
        priority: 190,
        heads: &["the"],
        shape_mask: 0,
        message: "unsupported first-spell cost modifier mechanic",
        predicate: line_has_first_spell_cost_modifier_clause,
    },
];

const STATIC_LINE_DIAGNOSTIC_RULES: [UnsupportedRuleDef; 7] = [
    UnsupportedRuleDef {
        id: "activate-only-standalone",
        priority: 200,
        heads: &["activate"],
        shape_mask: 0,
        message: "unsupported standalone activate-only restriction line",
        predicate: line_has_activate_only_standalone_clause,
    },
    UnsupportedRuleDef {
        id: "graveyard-cast-permission",
        priority: 210,
        heads: &["you"],
        shape_mask: 0,
        message: "unsupported graveyard cast-permission static clause",
        predicate: line_has_graveyard_cast_permission_clause,
    },
    UnsupportedRuleDef {
        id: "pregame-replacement",
        priority: 220,
        heads: &["if", "you"],
        shape_mask: 0,
        message: "unsupported pregame/replacement static clause",
        predicate: line_has_pregame_or_replacement_clause,
    },
    UnsupportedRuleDef {
        id: "dynamic-gets-from-counters",
        priority: 230,
        heads: &[],
        shape_mask: 0,
        message: "unsupported dynamic gets-from-counters static clause",
        predicate: line_has_dynamic_gets_from_counters_clause,
    },
    UnsupportedRuleDef {
        id: "foretell-cost-modifier",
        priority: 240,
        heads: &["foretelling"],
        shape_mask: 0,
        message: "unsupported foretell-cost modifier static clause",
        predicate: line_has_foretell_cost_modifier_clause,
    },
    UnsupportedRuleDef {
        id: "trigger-frequency-standalone",
        priority: 250,
        heads: &["this"],
        shape_mask: 0,
        message: "unsupported standalone trigger-frequency restriction line",
        predicate: line_has_trigger_frequency_restriction_clause,
    },
    UnsupportedRuleDef {
        id: "level-marker-static",
        priority: 260,
        heads: &[],
        shape_mask: 0,
        message: "unsupported level marker static clause",
        predicate: line_has_level_marker_clause,
    },
];

const PRE_TOKEN_DIAGNOSER: UnsupportedDiagnoser = UnsupportedDiagnoser::new(&PRE_TOKEN_DIAGNOSTIC_RULES);
const STATIC_LINE_DIAGNOSER: UnsupportedDiagnoser =
    UnsupportedDiagnoser::new(&STATIC_LINE_DIAGNOSTIC_RULES);

fn normalized_line<'a>(view: &'a ClauseView<'a>) -> &'a str {
    view.normalized
        .or(view.raw)
        .unwrap_or_default()
        .trim()
}

fn normalized_line_without_braces<'a>(view: &'a ClauseView<'a>) -> &'a str {
    view.normalized_without_braces
        .unwrap_or_else(|| normalized_line(view))
}

fn diagnose_line_unsupported(
    view: &ClauseView<'_>,
    is_collective_restraint_domain_attack_tax: bool,
    is_fixed_attack_tax_per_attacker: bool,
) -> Option<CardTextError> {
    PRE_TOKEN_DIAGNOSER
        .diagnose(view, "line")
        .or_else(|| STATIC_LINE_DIAGNOSER.diagnose(view, "line"))
        .or_else(|| {
            if line_is_known_unsupported_static_clause(
                normalized_line_without_braces(view),
                is_collective_restraint_domain_attack_tax,
                is_fixed_attack_tax_per_attacker,
            ) {
                let text = view.display_text();
                Some(CardTextError::ParseError(format!(
                    "unsupported static clause (line: '{}') [rule=known-static-clause]",
                    text
                )))
            } else {
                None
            }
        })
}

fn line_has_commander_cast_count_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("for each time")
        && normalized.contains("cast")
        && normalized.contains("commander")
        && normalized.contains("from the command zone")
}

fn line_has_verb_leading_spell_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("sacrifice x lands")
        && normalized.contains("you may play x additional lands this turn")
}

fn line_has_choose_leading_spell_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("choose target land")
        && normalized.contains("create three tokens that are copies of it")
}

fn line_has_put_from_among_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("put a land card from among them into your hand")
        || normalized.contains("put a card from among them into your hand")
}

fn line_has_standalone_token_reminder_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("it has \"this token gets +1/+1 for each card named")
        && normalized.contains("in each graveyard")
}

fn line_has_multi_destination_put_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("put one of them into your hand and the rest into your graveyard")
}

fn line_has_marker_keyword_tail_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("ninjutsu abilities you activate cost")
}

fn line_has_aura_copy_attachment_fanout_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("copy of that aura attached to that creature")
}

fn line_has_defending_players_choice_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("of defending players choice")
}

fn line_has_first_spell_cost_modifier_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("the first creature spell you cast each turn costs")
        && normalized.contains("less to cast")
}

fn line_has_activate_only_standalone_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("activate only")
}

fn line_has_graveyard_cast_permission_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("you may cast this card from your graveyard as long as you control")
        || normalized.starts_with("you may cast this from your graveyard as long as you control")
}

fn line_has_pregame_or_replacement_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("if this card is in your opening hand")
        || normalized.contains("you may begin the game with")
        || (normalized.starts_with("if this land would enter")
            && normalized.contains("if you do")
            && normalized.contains("put this"))
}

fn line_has_dynamic_gets_from_counters_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains("gets +x/+x")
        && normalized.contains("where x is the number of counters on this")
}

fn line_has_foretell_cost_modifier_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("foretelling cards from your hand costs")
}

fn line_has_trigger_frequency_restriction_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.starts_with("this ability triggers only")
}

fn line_has_level_marker_clause(view: &ClauseView<'_>) -> bool {
    let normalized = normalized_line(view);
    normalized.contains(": level ")
}

fn line_is_known_unsupported_static_clause(
    normalized: &str,
    is_collective_restraint_domain_attack_tax: bool,
    is_fixed_attack_tax_per_attacker: bool,
) -> bool {
    normalized == "play with the top card of your library revealed"
        || normalized.starts_with("gain the next level as a sorcery to add its ability")
        || normalized.starts_with("when this class becomes level")
        || normalized.starts_with("whenever you play a card")
        || normalized.starts_with("when there are no creatures on the battlefield")
        || normalized.starts_with("when there are no creatures on battlefield")
        || normalized == "you may play lands and cast spells from the top of your library"
        || normalized == "play lands and cast spells from the top of your library"
        || normalized == "all mountains are plains"
        || normalized.starts_with("you may look at top card of your library any time")
        || normalized.starts_with("you may look at the top card of your library any time")
        || normalized.starts_with("once each turn, you may play a card from exile")
        || normalized.starts_with("once each turn you may play a card from exile")
        || (normalized.starts_with("creatures cant attack you unless")
            && !is_collective_restraint_domain_attack_tax
            && !is_fixed_attack_tax_per_attacker)
        || normalized.starts_with("this creature cant attack unless")
        || normalized.starts_with("this creature cant attack if")
        || normalized.starts_with("this creature cant block unless")
        || normalized.starts_with("this creature cant block if")
        || normalized == "this creature attacks or blocks each combat if able"
        || normalized
            .starts_with("players cant untap more than one artifact during their untap steps")
        || normalized.starts_with("as long as equipped creature is a human")
        || normalized
            .starts_with("while an opponent is choosing targets as part of casting a spell")
        || normalized.starts_with("it enters with") && normalized.contains("+1/+1 counter")
        || normalized.starts_with("enchanted creature gets -x/-x")
        || normalized.starts_with("if one or more +1/+1 counters would be put on")
        || normalized.starts_with("if an effect would create one or more tokens under your control")
}

fn parse_first_parsed_ability_rule(
    tokens: &[Token],
) -> Result<Option<(&'static str, ParsedAbility)>, CardTextError> {
    fn parse_equip_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_equip_line(view.tokens)
    }

    fn parse_level_up_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_level_up_line(view.tokens)
    }

    fn parse_reinforce_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_reinforce_line(view.tokens)
    }

    fn parse_cycling_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_cycling_line(view.tokens)
    }

    fn parse_morph_rule(view: &ClauseView<'_>) -> Result<Option<ParsedAbility>, CardTextError> {
        parse_morph_keyword_line(view.tokens)
    }

    const RULES: [RuleDef<ParsedAbility>; 5] = [
        RuleDef {
            id: "equip",
            priority: 100,
            heads: &["equip"],
            shape_mask: RULE_SHAPE_HAS_COLON,
            run: parse_equip_rule,
        },
        RuleDef {
            id: "level-up",
            priority: 110,
            heads: &["level", "levelup"],
            shape_mask: 0,
            run: parse_level_up_rule,
        },
        RuleDef {
            id: "reinforce",
            priority: 120,
            heads: &["reinforce"],
            shape_mask: 0,
            run: parse_reinforce_rule,
        },
        RuleDef {
            id: "cycling",
            priority: 130,
            heads: &["cycling", "basic", "forestcycling", "mountaincycling", "swampcycling"],
            shape_mask: 0,
            run: parse_cycling_rule,
        },
        RuleDef {
            id: "morph",
            priority: 140,
            heads: &["morph", "megamorph"],
            shape_mask: 0,
            run: parse_morph_rule,
        },
    ];
    let view = ClauseView::from_tokens(tokens);
    let index = RuleIndex::new(&RULES);
    index.run_first(&view)
}

fn parse_first_optional_cost_rule(
    tokens: &[Token],
) -> Result<Option<(&'static str, OptionalCost)>, CardTextError> {
    fn parse_buyback_rule(view: &ClauseView<'_>) -> Result<Option<OptionalCost>, CardTextError> {
        parse_buyback_line(view.tokens)
    }

    fn parse_kicker_rule(view: &ClauseView<'_>) -> Result<Option<OptionalCost>, CardTextError> {
        parse_kicker_line(view.tokens)
    }

    fn parse_multikicker_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<OptionalCost>, CardTextError> {
        parse_multikicker_line(view.tokens)
    }

    fn parse_entwine_rule(view: &ClauseView<'_>) -> Result<Option<OptionalCost>, CardTextError> {
        parse_entwine_line(view.tokens)
    }

    const RULES: [RuleDef<OptionalCost>; 4] = [
        RuleDef {
            id: "buyback",
            priority: 100,
            heads: &["buyback"],
            shape_mask: 0,
            run: parse_buyback_rule,
        },
        RuleDef {
            id: "kicker",
            priority: 110,
            heads: &["kicker"],
            shape_mask: 0,
            run: parse_kicker_rule,
        },
        RuleDef {
            id: "multikicker",
            priority: 120,
            heads: &["multikicker"],
            shape_mask: 0,
            run: parse_multikicker_rule,
        },
        RuleDef {
            id: "entwine",
            priority: 130,
            heads: &["entwine"],
            shape_mask: 0,
            run: parse_entwine_rule,
        },
    ];
    let view = ClauseView::from_tokens(tokens);
    let index = RuleIndex::new(&RULES);
    index.run_first(&view)
}

fn parse_first_alternative_cast_rule(
    tokens: &[Token],
    line: &str,
) -> Result<Option<(&'static str, AlternativeCastingMethod)>, CardTextError> {
    fn parse_if_conditional_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_if_conditional_alternative_cost_line(view.tokens, view.raw.unwrap_or_default())
    }

    fn parse_self_free_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        Ok(parse_self_free_cast_alternative_cost_line(view.tokens))
    }

    fn parse_you_may_rather_than_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_you_may_rather_than_spell_cost_line(view.tokens, view.raw.unwrap_or_default())
    }

    fn parse_escape_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_escape_line(view.tokens)
    }

    fn parse_bestow_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_bestow_line(view.tokens)
    }

    fn parse_flashback_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_flashback_line(view.tokens)
    }

    fn parse_madness_rule(
        view: &ClauseView<'_>,
    ) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
        parse_madness_line(view.tokens)
    }

    const RULES: [RuleDef<AlternativeCastingMethod>; 7] = [
        RuleDef {
            id: "if-conditional-alternative-cost",
            priority: 100,
            heads: &["if"],
            shape_mask: 0,
            run: parse_if_conditional_rule,
        },
        RuleDef {
            id: "self-free-cast-alternative-cost",
            priority: 110,
            heads: &["if", "you"],
            shape_mask: 0,
            run: parse_self_free_rule,
        },
        RuleDef {
            id: "alternative-cost",
            priority: 120,
            heads: &["you"],
            shape_mask: 0,
            run: parse_you_may_rather_than_rule,
        },
        RuleDef {
            id: "escape",
            priority: 130,
            heads: &["escape"],
            shape_mask: 0,
            run: parse_escape_rule,
        },
        RuleDef {
            id: "bestow",
            priority: 140,
            heads: &["bestow"],
            shape_mask: 0,
            run: parse_bestow_rule,
        },
        RuleDef {
            id: "flashback",
            priority: 150,
            heads: &["flashback"],
            shape_mask: 0,
            run: parse_flashback_rule,
        },
        RuleDef {
            id: "madness",
            priority: 160,
            heads: &["madness"],
            shape_mask: 0,
            run: parse_madness_rule,
        },
    ];
    let view = ClauseView::from_line(line, line, line, tokens);
    let index = RuleIndex::new(&RULES);
    index.run_first(&view)
}

fn line_ast_from_static_abilities(abilities: Vec<StaticAbility>) -> LineAst {
    match abilities.as_slice() {
        [ability] => LineAst::StaticAbility(ability.clone()),
        _ => LineAst::StaticAbilities(abilities),
    }
}

pub(crate) fn parse_line(line: &str, line_index: usize) -> Result<LineAst, CardTextError> {
    parser_trace_line("parse_line:entry", line);
    let normalized = line
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    let normalized = normalized.replace('\'', "").replace('’', "");
    let normalized_without_braces = normalized.replace('{', "").replace('}', "");
    let normalized_without_braces = normalized_without_braces.trim_end_matches('.');
    let tokens = tokenize_line(line, line_index);
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty line".to_string()));
    }
    let line_view = ClauseView::from_line(line, &normalized, normalized_without_braces, &tokens);

    if normalized.starts_with("this effect cant reduce the mana in that cost to less than")
        || normalized.starts_with("this effect cant reduce the mana in those costs to less than")
    {
        // Reminder text for already-modeled cost-modifier clauses.
        return Ok(LineAst::StaticAbilities(Vec::new()));
    }
    if (normalized.starts_with("this creature enters with")
        || normalized.starts_with("this creature enters the battlefield with")
        || normalized.starts_with("it enters with")
        || normalized.starts_with("it enters the battlefield with"))
        && normalized.contains("+1/+1 counter")
    {
        let tokens = tokenize_line(line, line_index);
        if let Ok(Some(ability)) = parse_enters_with_counters_line(&tokens) {
            parser_trace("parse_line:branch=self-etb-counters", &tokens);
            return Ok(LineAst::StaticAbility(ability));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported self-enters-with-counters static clause (line: '{}')",
            line
        )));
    }
    let is_collective_restraint_domain_attack_tax = normalized_without_braces.starts_with(
        "creatures cant attack you unless their controller pays x for each creature they control thats attacking you",
    ) && normalized_without_braces.contains("where x is the number of basic land type");
    let is_fixed_attack_tax_per_attacker = normalized_without_braces
        .strip_prefix("creatures cant attack you unless their controller pays ")
        .and_then(|rest| rest.strip_suffix(" for each creature they control thats attacking you"))
        .is_some_and(|amount| !amount.is_empty() && amount.chars().all(|ch| ch.is_ascii_digit()));
    let unsupported_diagnostic = diagnose_line_unsupported(
        &line_view,
        is_collective_restraint_domain_attack_tax,
        is_fixed_attack_tax_per_attacker,
    );

    let is_this_cant_attack_unless_clause = normalized
        .starts_with("this creature cant attack unless")
        || normalized.starts_with("this cant attack unless");
    if is_this_cant_attack_unless_clause {
        let tokens = tokenize_line(line, line_index);
        if let Ok(Some(abilities)) = parse_static_ability_line(&tokens) {
            parser_trace("parse_line:branch=this-cant-attack-unless-static", &tokens);
            return Ok(line_ast_from_static_abilities(abilities));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported this-cant-attack-unless static clause (line: '{}')",
            line
        )));
    }
    if normalized.starts_with("cast this spell only") {
        let tokens = tokenize_line(line, line_index);
        if let Ok(Some(ability)) = parse_cast_this_spell_only_line(&tokens) {
            parser_trace("parse_line:branch=this-spell-cast-only-static", &tokens);
            return Ok(LineAst::StaticAbility(ability));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported cast-this-spell-only static clause (line: '{}')",
            line
        )));
    }
    if normalized.starts_with("creatures with power less than this creatures power cant block it") {
        let tokens = tokenize_line(line, line_index);
        if let Ok(Some(abilities)) = parse_static_ability_line(&tokens) {
            parser_trace("parse_line:branch=skulk-rules-text-static", &tokens);
            return Ok(line_ast_from_static_abilities(abilities));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported skulk-rules-text static clause (line: '{}')",
            line
        )));
    }

    if let Some((chapters, rest)) = parse_saga_chapter_prefix(&normalized) {
        let tokens = tokenize_line(rest, line_index);
        parser_trace("parse_line:branch=saga", &tokens);
        let effects = parse_effect_sentences(&tokens)?;
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::SagaChapter(chapters),
            effects,
            max_triggers_per_turn: None,
        });
    }

    if tokens
        .first()
        .is_some_and(|token| token.is_word("replicate"))
        && line.contains('—')
    {
        let cost_tokens = tokens.get(1..).unwrap_or_default();
        if cost_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "replicate line missing cost (line: '{line}')"
            )));
        }
        parser_trace("parse_line:branch=replicate", &tokens);
        let (cost, _) = parse_activation_cost(cost_tokens)?;
        return Ok(LineAst::OptionalCost(
            OptionalCost::custom("Replicate", cost).repeatable(),
        ));
    }

    if normalized.starts_with("as an additional cost to cast this spell") {
        let comma_idx = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)));
        let effect_start = if let Some(idx) = comma_idx {
            idx + 1
        } else if let Some(idx) = tokens.iter().position(|token| token.is_word("spell")) {
            idx + 1
        } else {
            tokens.len()
        };
        let effect_tokens = tokens.get(effect_start..).unwrap_or_default();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "additional cost line missing effect clause".to_string(),
            ));
        }
        parser_trace("parse_line:branch=additional-cost", effect_tokens);
        if let Some(options) = parse_additional_cost_choice_options(effect_tokens)? {
            return Ok(LineAst::AdditionalCostChoice { options });
        }
        let effects = parse_effect_sentences(effect_tokens)?;
        return Ok(LineAst::AdditionalCost { effects });
    }

    if is_non_mana_additional_cost_modifier_line(&normalized) {
        return Err(CardTextError::ParseError(format!(
            "unsupported non-mana additional-cost modifier line (line: '{}')",
            line
        )));
    }

    if let Some((branch, method)) = parse_first_alternative_cast_rule(&tokens, line)? {
        let stage = format!("parse_line:branch={branch}");
        parser_trace(stage.as_str(), &tokens);
        return Ok(LineAst::AlternativeCastingMethod(method));
    }

    if let Some((branch, ability)) = parse_first_parsed_ability_rule(&tokens)? {
        let stage = format!("parse_line:branch={branch}");
        parser_trace(stage.as_str(), &tokens);
        return Ok(LineAst::Ability(ability));
    }

    if let Some((branch, cost)) = parse_first_optional_cost_rule(&tokens)? {
        let stage = format!("parse_line:branch={branch}");
        parser_trace(stage.as_str(), &tokens);
        return Ok(LineAst::OptionalCost(cost));
    }

    if let Some((trigger_idx, _)) = tokens.iter().enumerate().find(|(idx, token)| {
        token.is_word("whenever") || token.is_word("when") || is_at_trigger_intro(&tokens, *idx)
    }) && (trigger_idx <= 2
        || (trigger_idx > 2 && dash_labeled_remainder_starts_with_trigger(line)))
    {
        parser_trace("parse_line:branch=triggered", &tokens[trigger_idx..]);
        return parse_triggered_line(&tokens[trigger_idx..]);
    }

    if tokens
        .first()
        .is_some_and(|token| token.is_word("waterbend"))
        && let Some(ability) = parse_activated_line(&tokens[1..])?
    {
        parser_trace("parse_line:branch=waterbend-activated", &tokens[1..]);
        return Ok(LineAst::Ability(ability));
    }

    if let Some(colon_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Colon(_)))
    {
        let cost_tokens = &tokens[..colon_idx];
        if starts_with_activation_cost(cost_tokens) {
            if let Some(ability) = parse_activated_line(&tokens)? {
                parser_trace("parse_line:branch=activated", &tokens);
                return Ok(LineAst::Ability(ability));
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported activated ability line (line: '{line}')"
            )));
        } else if (line.contains('—') || line.contains(" - "))
            && find_activation_cost_start(cost_tokens).is_some()
            && let Some(ability) = parse_activated_line(&tokens)?
        {
            parser_trace("parse_line:branch=activated-labeled", &tokens);
            return Ok(LineAst::Ability(ability));
        }
    }

    let line_words = words(&tokens);
    let has_token_mana_reminder_tail = line_words.contains(&"create")
        && line_words.contains(&"sacrifice")
        && line_words.contains(&"add")
        && line_words
            .windows(2)
            .any(|window| window == ["it", "has"] || window == ["they", "have"]);
    if has_token_mana_reminder_tail
        && let Ok(effects) = parse_effect_sentences(&tokens)
        && !effects.is_empty()
    {
        parser_trace("parse_line:branch=statement-token-mana-reminder", &tokens);
        return Ok(LineAst::Statement { effects });
    }

    let is_each_other_player_untap_static =
        is_untap_during_each_other_players_untap_step_words(&line_words);

    if tokens.first().is_some_and(|token| token.is_word("if"))
        && let Some(ability) = parse_if_this_spell_costs_less_to_cast_line(&tokens)?
    {
        parser_trace("parse_line:branch=if-this-spell-costs-less", &tokens);
        return Ok(LineAst::StaticAbility(ability));
    }

    let starts_with_statement_effect_head = find_verb(&tokens).is_some_and(|(_, idx)| idx == 0)
        || find_verb(&tokens).is_some_and(|(_, idx)| {
            idx == 1
                && tokens.first().is_some_and(|token| {
                    token.is_word("this") || token.is_word("it") || token.is_word("that")
                })
        })
        || tokens
            .first()
            .is_some_and(|token| token.is_word("choose") || token.is_word("if"))
        || starts_with_until_end_of_turn(&line_words);
    let is_damage_prevent_with_remove_static = line_words
        .starts_with(&["if", "damage", "would", "be", "dealt", "to", "this"])
        && line_words
            .windows(3)
            .any(|window| window == ["prevent", "that", "damage"])
        && line_words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
        && line_words.iter().any(|word| *word == "remove");
    let is_prevent_all_damage_to_source_by_creatures_static = line_words.starts_with(&[
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to", "this",
    ]) && line_words
        .ends_with(&["by", "creatures"]);
    let is_prevent_all_combat_damage_to_source_static = line_words
        == [
            "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to", "this",
            "creature",
        ]
        || line_words
            == [
                "prevent",
                "all",
                "combat",
                "damage",
                "that",
                "would",
                "be",
                "dealt",
                "to",
                "this",
                "permanent",
            ]
        || line_words
            == [
                "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to", "it",
            ];
    if starts_with_statement_effect_head
        && !is_each_other_player_untap_static
        && !is_damage_prevent_with_remove_static
        && !is_prevent_all_damage_to_source_by_creatures_static
        && !is_prevent_all_combat_damage_to_source_static
    {
        match parse_effect_sentences(&tokens) {
            Ok(effects) if !effects.is_empty() => {
                parser_trace("parse_line:branch=statement-verb-leading", &tokens);
                return Ok(LineAst::Statement { effects });
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }

    if let Some(abilities) = parse_static_ability_line(&tokens)? {
        if let Some(diag) = unsupported_diagnostic.clone() {
            return Err(diag);
        }
        parser_trace("parse_line:branch=static", &tokens);
        return Ok(line_ast_from_static_abilities(abilities));
    }

    if let Some(actions) = parse_ability_line(&tokens) {
        parser_trace("parse_line:branch=keyword-ability-line", &tokens);
        return Ok(LineAst::Abilities(actions));
    }

    parser_trace("parse_line:branch=statement", &tokens);
    let effects = match parse_effect_sentences(&tokens) {
        Ok(effects) => effects,
        Err(parse_err) => {
            if let Some(diag) = unsupported_diagnostic.clone() {
                return Err(diag);
            }
            return Err(parse_err);
        }
    };
    if effects.is_empty() {
        parser_trace("parse_line:branch=statement-empty", &tokens);
        if let Some(diag) = unsupported_diagnostic {
            return Err(diag);
        }
        let head = tokens
            .first()
            .and_then(Token::as_word)
            .unwrap_or("unknown-head");
        return Err(CardTextError::ParseError(format!(
            "unsupported line (no-line-rule-match, head='{head}'): {line}"
        )));
    }

    Ok(LineAst::Statement { effects })
}

pub(crate) fn parse_additional_cost_choice_options(
    tokens: &[Token],
) -> Result<Option<Vec<AdditionalCostChoiceOptionAst>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"or") {
        return Ok(None);
    }

    let option_tokens = split_on_or(tokens);
    if option_tokens.len() < 2 {
        return Ok(None);
    }

    let mut normalized_options = Vec::new();
    for mut option in option_tokens {
        while option
            .first()
            .is_some_and(|token| token.is_word("and") || token.is_word("or"))
        {
            option.remove(0);
        }
        let option = trim_commas(&option).to_vec();
        if option.is_empty() {
            continue;
        }
        normalized_options.push(option);
    }

    if normalized_options.len() < 2 {
        return Ok(None);
    }

    // If any branch lacks a verb, this "or" belongs to a noun phrase
    // (for example, "discard a red or green card"), not a cost choice.
    if normalized_options
        .iter()
        .any(|option| find_verb(option).is_none())
    {
        return Ok(None);
    }

    let mut options = Vec::new();
    for option in normalized_options {
        let effects = parse_effect_sentences(&option)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "additional cost option parsed to no effects (clause: '{}')",
                words(&option).join(" ")
            )));
        }
        options.push(AdditionalCostChoiceOptionAst {
            description: words(&option).join(" "),
            effects,
        });
    }

    if options.len() < 2 {
        return Ok(None);
    }

    Ok(Some(options))
}

pub(crate) fn is_at_trigger_intro(tokens: &[Token], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return false;
    }

    let second = tokens.get(idx + 1).and_then(Token::as_word);
    let third = tokens.get(idx + 2).and_then(Token::as_word);
    matches!(
        (second, third),
        (Some("beginning"), _)
            | (Some("end"), _)
            | (Some("the"), Some("beginning"))
            | (Some("the"), Some("end"))
    )
}

pub(crate) fn starts_with_activation_cost(tokens: &[Token]) -> bool {
    let Some(word) = tokens.first().and_then(Token::as_word) else {
        return false;
    };
    if matches!(
        word,
        "tap"
            | "t"
            | "pay"
            | "discard"
            | "mill"
            | "sacrifice"
            | "put"
            | "remove"
            | "exile"
            | "return"
            | "e"
    ) {
        return true;
    }
    if word.contains('/') {
        return parse_mana_symbol_group(word).is_ok();
    }
    parse_mana_symbol(word).is_ok()
}

pub(crate) fn find_activation_cost_start(tokens: &[Token]) -> Option<usize> {
    (0..tokens.len()).find(|idx| starts_with_activation_cost(&tokens[*idx..]))
}

pub(crate) fn parse_flashback_keyword_line(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let words_all = words(tokens);
    if words_all.first().copied() != Some("flashback") {
        return None;
    }
    let (cost, consumed) = leading_mana_symbols_to_oracle(&words_all[1..])?;
    let mut text = format!("Flashback {cost}");
    let tail = &words_all[1 + consumed..];
    if !tail.is_empty() {
        let mut tail_text = tail.join(" ");
        if let Some(first) = tail_text.chars().next() {
            let upper = first.to_ascii_uppercase().to_string();
            let rest = &tail_text[first.len_utf8()..];
            tail_text = format!("{upper}{rest}");
        }
        text.push_str(", ");
        text.push_str(&tail_text);
    }
    Some(vec![KeywordAction::MarkerText(text)])
}

pub(crate) fn parse_flashback_line(
    tokens: &[Token],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("flashback"))
    {
        return Ok(None);
    }

    let cost_start = 1usize;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(
            "flashback keyword missing mana cost".to_string(),
        ));
    }

    let (parsed_cost, cost_effects) = parse_activation_cost(&tokens[cost_start..])?;
    let total_cost = crate::ability::merge_cost_effects(parsed_cost, cost_effects);
    if total_cost.mana_cost().is_none() {
        return Err(CardTextError::ParseError(
            "flashback keyword missing mana symbols".to_string(),
        ));
    }

    Ok(Some(AlternativeCastingMethod::Flashback { total_cost }))
}

pub(crate) fn parse_bestow_line(
    tokens: &[Token],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("bestow")) {
        return Ok(None);
    }

    let words_all = words(tokens);
    let (mana_cost_text, mana_word_count) = leading_mana_symbols_to_oracle(&words_all[1..])
        .ok_or_else(|| CardTextError::ParseError("bestow keyword missing mana cost".to_string()))?;
    let mana_cost = parse_scryfall_mana_cost(&mana_cost_text).map_err(|err| {
        CardTextError::ParseError(format!(
            "invalid bestow mana cost '{mana_cost_text}': {err:?}"
        ))
    })?;
    let mut total_cost = TotalCost::mana(mana_cost.clone());

    let mut consumed_mana_tokens = 0usize;
    for token in tokens.iter().skip(1) {
        let Some(word) = token.as_word() else {
            break;
        };
        if parse_mana_symbol(word).is_ok() {
            consumed_mana_tokens += 1;
            continue;
        }
        break;
    }
    if consumed_mana_tokens == 0 {
        consumed_mana_tokens = mana_word_count;
    }
    consumed_mana_tokens = consumed_mana_tokens.min(tokens.len().saturating_sub(1));

    let mut cost_tokens = tokens
        .get(1..1 + consumed_mana_tokens)
        .unwrap_or_default()
        .to_vec();
    let tail_tokens = tokens.get(1 + consumed_mana_tokens..).unwrap_or_default();
    if tail_tokens
        .first()
        .is_some_and(|token| matches!(token, Token::Comma(_)))
    {
        let clause_end = tail_tokens
            .iter()
            .position(|token| matches!(token, Token::Period(_)))
            .unwrap_or(tail_tokens.len());
        let clause_tokens = trim_commas(&tail_tokens[..clause_end]).to_vec();
        let clause_words = words(&clause_tokens);
        if !clause_words.is_empty() && clause_words[0] != "if" {
            cost_tokens.extend(clause_tokens);
        }
    }

    if let Ok((parsed_total_cost, parsed_cost_effects)) = parse_activation_cost(&cost_tokens) {
        total_cost = crate::ability::merge_cost_effects(parsed_total_cost, parsed_cost_effects);
        if total_cost.mana_cost().is_none() {
            let mut components = total_cost.costs().to_vec();
            components.insert(0, crate::costs::Cost::mana(mana_cost));
            total_cost = TotalCost::from_costs(components);
        }
    }

    Ok(Some(AlternativeCastingMethod::Bestow { total_cost }))
}

fn is_self_free_cast_clause(words: &[&str]) -> bool {
    words
        == [
            "you", "may", "cast", "this", "spell", "without", "paying", "its", "mana", "cost",
        ]
        || words
            == [
                "you", "may", "cast", "this", "spell", "without", "paying", "this", "spells",
                "mana", "cost",
            ]
}

pub(crate) fn parse_self_free_cast_alternative_cost_line(
    tokens: &[Token],
) -> Option<AlternativeCastingMethod> {
    let clause_words = words(tokens);
    if !is_self_free_cast_clause(&clause_words) {
        return None;
    }
    Some(AlternativeCastingMethod::alternative_cost(
        "Parsed alternative cost",
        None,
        Vec::new(),
    ))
}

pub(crate) fn parse_you_may_rather_than_spell_cost_line(
    tokens: &[Token],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !(tokens.first().is_some_and(|token| token.is_word("you"))
        && tokens.get(1).is_some_and(|token| token.is_word("may")))
    {
        return Ok(None);
    }
    let Some(rather_idx) = tokens.iter().position(|token| token.is_word("rather")) else {
        return Ok(None);
    };
    let rather_tail = words(tokens.get(rather_idx + 1..).unwrap_or_default());
    let is_spell_cost_clause = rather_tail.starts_with(&["than", "pay", "this"])
        && rather_tail.contains(&"mana")
        && rather_tail.contains(&"cost")
        && (rather_tail.contains(&"spell") || rather_tail.contains(&"spells"));
    if !is_spell_cost_clause {
        return Ok(None);
    }
    let cost_clause_end = (rather_idx + 1..tokens.len())
        .rfind(|idx| tokens[*idx].is_word("cost") || tokens[*idx].is_word("costs"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "alternative cost line missing terminal cost word (line: '{}')",
                line
            ))
        })?;
    let trailing_words = words(&tokens[cost_clause_end + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing clause after alternative cost (line: '{}', trailing: '{}')",
            line,
            trailing_words.join(" ")
        )));
    }
    let cost_tokens = tokens.get(2..rather_idx).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "alternative cost line missing cost clause".to_string(),
        ));
    }
    let (parsed_cost, cost_effects) = parse_activation_cost(cost_tokens)?;
    let total_cost = crate::ability::merge_cost_effects(parsed_cost, cost_effects);
    Ok(Some(AlternativeCastingMethod::Composed {
        name: "Parsed alternative cost",
        total_cost,
        condition: None,
    }))
}

pub(crate) fn parse_if_conditional_alternative_cost_line(
    tokens: &[Token],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["if"]) {
        return Ok(None);
    }

    let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };
    let condition_tokens = trim_commas(&tokens[1..comma_idx]);
    let tail_tokens = trim_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    let tail_words = words(&tail_tokens);
    if !is_self_free_cast_clause(&tail_words)
        && parse_you_may_rather_than_spell_cost_line(&tail_tokens, line)?.is_none()
    {
        return Ok(None);
    }
    let Some(condition) = parse_this_spell_cost_condition(&condition_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    if is_self_free_cast_clause(&tail_words) {
        return Ok(Some(
            AlternativeCastingMethod::alternative_cost_with_condition(
                "Parsed alternative cost",
                None,
                Vec::new(),
                condition,
            ),
        ));
    }

    let Some(method) = parse_you_may_rather_than_spell_cost_line(&tail_tokens, line)? else {
        return Ok(None);
    };
    Ok(Some(method.with_cast_condition(condition)))
}
