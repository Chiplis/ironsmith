//! The readings of one alternative-cast keyword line: aftermath, encore,
//! surge, the prefixed keyword shapes, blitz from the graveyard, the
//! self-free and flash-with-cost alternatives, jump-start, "you may ... rather
//! than", conditional alternative costs, prowl, and the cost-less static.
//! Formerly a first-match ladder in `keyword_payloads`; every reading runs,
//! resolved by rank while the overlaps are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

/// The input the readings read.
pub(super) struct AlternativeCastLine<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) line: &'a PreprocessedLine,
    pub(super) full_tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl AlternativeCastLine<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<KeywordLinePayload>, CardTextError>,
    ) -> ParseOutcome<KeywordLinePayload> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("alternative-cast-registry-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&AlternativeCastLine<'_>) -> bool,
    read: fn(&AlternativeCastLine<'_>) -> ParseOutcome<KeywordLinePayload>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("alternative-cast-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("reveal-from-hand-to-add-spell-text"),
        head: HeadDiscriminator::Words(&["as"]),
        admits: |_| true,
        read: |input| input.outcome(read_reveal_to_add_spell_text(input)),
    },

    Reading {
        id: RuleId::new("payment-for-each-extra-mode"),
        head: HeadDiscriminator::Words(&["pay", "sacrifice", "discard", "exile", "return", "tap", "remove"]),
        admits: |_| true,
        read: |input| input.outcome(read_payment_per_extra_mode(input)),
    },

    Reading {
        id: RuleId::new("optional-payment-chooses-all-modes"),
        head: HeadDiscriminator::Words(&["choose"]),
        admits: |_| true,
        read: |input| input.outcome(read_optional_all_modes(input)),
    },

    Reading {
        id: RuleId::new("alternative-price-entry-counter-countdown"),
        head: HeadDiscriminator::Words(&["if"]),
        admits: |_| true,
        read: |input| input.outcome(read_entry_counter_countdown(input)),
    },

    Reading {
        id: RuleId::new("bound-reminder-alternative-price"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(
            crate::grammar::shared_util::alternative_cost_lines::parse_self_bound_alternative_price(input.tokens)
                .map(|method| method.and_then(|method| ast(LineAst::AlternativeCastingMethod(method))))
        ),
    },
    Reading {
        id: RuleId::new("intrinsic-zone-alternative-cost"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(
            crate::grammar::shared_util::alternative_cost_lines::parse_self_zone_alternative_cost(input.tokens)
                .map(|method| method.and_then(|method| ast(LineAst::AlternativeCastingMethod(method))))
        ),
    },
    Reading {
        id: RuleId::new("emerge-from"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_emerge_from(input)),
    },
    Reading {
        id: RuleId::new("aftermath"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_aftermath(input)),
    },
    Reading {
        id: RuleId::new("encore"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_encore(input)),
    },
    Reading {
        id: RuleId::new("surge"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_surge(input)),
    },
    Reading {
        id: RuleId::new("freerunning"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_freerunning(input)),
    },
    Reading {
        id: RuleId::new("more-than-meets-the-eye"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_more_than_meets_the_eye(input)),
    },
    Reading {
        id: RuleId::new("offering"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_offering(input)),
    },
    Reading {
        id: RuleId::new("mayhem"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_mayhem(input)),
    },
    Reading {
        id: RuleId::new("web-slinging"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_web_slinging(input)),
    },
    Reading {
        id: RuleId::new("sneak"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_sneak(input)),
    },
    Reading {
        id: RuleId::new("blitz-from-graveyard"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_blitz_from_graveyard(input)),
    },
    Reading {
        id: RuleId::new("self-free-cast-alternative-cost-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_self_free_cast_alternative_cost_line(input)),
    },
    Reading {
        id: RuleId::new("flash-with-additional-cost-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_flash_with_additional_cost_line(input)),
    },
    Reading {
        id: RuleId::new("flash-timing-optional-payment-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_flash_timing_optional_payment_line(input)),
    },
    Reading {
        id: RuleId::new("jump-start-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_jump_start_line(input)),
    },
    Reading {
        id: RuleId::new("you-may-rather-than-spell-cost-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_you_may_rather_than_spell_cost_line(input)),
    },
    Reading {
        id: RuleId::new("if-conditional-alternative-cost-line"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("freerunning") && !input.read_by("surge")
        },
        read: |input| input.outcome(read_if_conditional_alternative_cost_line(input)),
    },
    Reading {
        id: RuleId::new("prowl-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_prowl_line(input)),
    },
    Reading {
        id: RuleId::new("if-this-spell-costs-less-to-cast-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_if_this_spell_costs_less_to_cast_line(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &AlternativeCastLine<'_>) -> ParseOutcome<RuleMatch<KeywordLinePayload>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        match (reading.read)(input).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<KeywordLinePayload>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_ranked_candidates(REGISTRY, distinct, diagnostics, || {
        crate::lexer::parser_token_word_refs(input.tokens).join(" ")
    });
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_aftermath(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(input.tokens);
    let native = words == ["aftermath"];
    let described = matches!(words.as_slice(), ["cast", "this", "spell" | "card", "only", "from", "your", "graveyard", "then", "exile", "it"]);
    if !native && !described { return Ok(None); }
    let restriction = crate::model::CompilerStaticAbilityCore::this_spell_cast_restriction(
        crate::static_abilities::ThisSpellCastRestrictionKind::only_if(
            ironsmith_core::Condition::SourceIsInZone(crate::zone::Zone::Graveyard),
        ), "Cast this spell only from your graveyard",
    );
    let permission = crate::model::CompilerAlternativeCastingMethod::cast_from_zone_with_total_cost(
        if native { "Aftermath" } else { "Parsed exclusive graveyard permission" },
        crate::zone::Zone::Graveyard, ironsmith_core::TotalCost::free(), None, false,
    ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
        add_printed_mana_cost: true, exile_on_stack_exit: true, ..Default::default()
    });
    Ok(ast(LineAst::Multiple(vec![
        LineAst::StaticAbility(restriction.into()),
        LineAst::StaticAbility(crate::model::CompilerStaticAbilityCore::keyword_marker("If this spell was cast from your graveyard, exile it instead of putting it anywhere else any time it would leave the stack.").into()),
        LineAst::AlternativeCastingMethod(permission),
    ])))
}

fn read_encore(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    let line = input.line;
    if token_slice_first_is(tokens, "encore") {
        let (cost, _) = leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "encore keyword missing mana cost '{}'",
                    line.info.raw_line
                ))
            })?;
        return Ok(ast(LineAst::Abilities(vec![
            crate::cards::builders::KeywordAction::Encore(cost),
        ])));
    }
    Ok(None)
}
fn read_surge(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    let line = input.line;
    let full_tokens = input.full_tokens;
    // A spell-cost condition is evaluated against the game, not recognized
    // structure, so it holds the bound form.
    if let Some(raw_tokens) =
        keyword_tokens_for_shape(tokens, full_tokens, KeywordPrefixShape::Surge)
    {
        let (cost, _) = leading_mana_cost_from_tokens(raw_tokens.get(1..).unwrap_or_default())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "surge keyword missing cost '{}'",
                    line.info.raw_line
                ))
            })?;
        let condition = crate::static_abilities::ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::Or(
                Box::new(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: crate::cards::builders::PlayerFilter::You,
                    count: 1,
                }),
                Box::new(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: crate::cards::builders::PlayerFilter::Teammate,
                    count: 1,
                }),
            ),
            display: "you or a teammate has cast another spell this turn".to_string(),
        };
        return Ok(ast(LineAst::AlternativeCastingMethod(
            crate::model::CompilerAlternativeCastingMethod::alternative_cost_with_condition(
                "Surge",
                Some(cost),
                Vec::new(),
                condition,
            ),
        )));
    }
    Ok(None)
}
fn read_freerunning(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    let line = input.line;
    let full_tokens = input.full_tokens;
    if let Some(keyword_tokens) =
        keyword_tokens_for_shape(tokens, full_tokens, KeywordPrefixShape::Freerunning)
    {
        let condition = crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
            crate::types::Subtype::Assassin,
        );
        let cost_tokens = keyword_tokens.get(1..).unwrap_or_default();
        // "Freerunning—Return a blue creature you control to its owner's
        // hand" (Escape Detection): a non-mana freerunning cost.
        if leading_mana_cost_from_tokens(cost_tokens).is_none() {
            let cost_tokens = cost_tokens
                .iter()
                .skip_while(|token| matches!(token.kind, crate::lexer::TokenKind::Dash | crate::lexer::TokenKind::EmDash))
                .cloned()
                .collect::<Vec<_>>();
            let Some(total_cost) =
                crate::activation_and_restrictions::parse_payment_clause_as_total_cost(
                    &cost_tokens,
                )?
            else {
                return Ok(None);
            };
            let method = crate::model::CompilerAlternativeCastingMethod::Composed {
                name: "Freerunning".into(),
                total_cost,
                condition: None,
                prototype_power_toughness: None,
            };
            return Ok(ast(LineAst::AlternativeCastingMethod(
                crate::model::CompilerAlternativeCastingMethod::alternative_cost_with_condition(
                    "Freerunning",
                    method.mana_cost().cloned(),
                    method.non_mana_costs(),
                    condition,
                ),
            )));
        }
        let (cost, _) = leading_mana_cost_from_tokens(cost_tokens)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "freerunning keyword missing cost '{}'",
                    line.info.raw_line
                ))
            })?;
        return Ok(ast(LineAst::AlternativeCastingMethod(
                crate::model::CompilerAlternativeCastingMethod::alternative_cost_with_condition(
                    "Freerunning",
                    Some(cost),
                    Vec::new(),
                    crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
                        crate::types::Subtype::Assassin,
                    ),
                ),
            )));
    }
    Ok(None)
}
fn read_more_than_meets_the_eye(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let Some(tokens) = keyword_tokens_for_shape(
        input.tokens,
        input.full_tokens,
        KeywordPrefixShape::MoreThanMeetsTheEye,
    ) else {
        return Ok(None);
    };
    let (cost, _) =
        leading_mana_cost_from_tokens(tokens.get(5..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("more than meets the eye keyword missing mana cost".into())
        })?;
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::model::CompilerAlternativeCastingMethod::alternative_cost(
            "More than meets the eye",
            Some(cost),
            vec![],
        ),
    )))
}

fn read_emerge_from(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let words = crate::grammar::primitives::TokenWordView::new(input.tokens).word_refs();
    if !words.starts_with(&["emerge", "from"]) {
        return Ok(None);
    }
    let quality = words
        .get(2)
        .ok_or_else(|| CardTextError::ParseError("emerge from missing quality".into()))?;
    let mut filter = crate::target::ObjectFilter::default().you_control();
    if let Some(card_type) = crate::util::parse_card_type(quality) {
        filter.card_types.push(card_type);
    } else if let Some(subtype) = crate::util::parse_subtype_word(quality) {
        filter.subtypes.push(subtype);
    } else {
        return Err(CardTextError::ParseError(
            "unsupported emerge permanent quality".into(),
        ));
    }
    let (cost, _) = leading_mana_cost_from_tokens(input.tokens.get(3..).unwrap_or_default())
        .ok_or_else(|| CardTextError::ParseError("emerge from missing mana cost".into()))?;
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::model::CompilerAlternativeCastingMethod::alternative_cost(
            "Emerge",
            Some(cost),
            vec![crate::model::CompilerCost::Sacrifice {
                count: ironsmith_core::ChoiceCount::exactly(1),
                filter,
                all: false,
                binding: None,
            }],
        ),
    )))
}

fn read_offering(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let words = crate::grammar::primitives::TokenWordView::new(input.tokens).word_refs();
    if words.len() != 2 || words[1] != "offering" {
        return Ok(None);
    }
    let mut filter = crate::target::ObjectFilter::default().you_control();
    if let Some(card_type) = crate::util::parse_card_type(words[0]) {
        filter.card_types.push(card_type);
    } else if let Some(subtype) = crate::util::parse_subtype_word(words[0]) {
        filter.subtypes.push(subtype);
    } else {
        return Err(CardTextError::ParseError(
            "unknown offering permanent quality".into(),
        ));
    }
    let cost = crate::model::CompilerCost::Sacrifice {
        count: ironsmith_core::ChoiceCount::exactly(1),
        filter,
        all: false,
        binding: None,
    };
    Ok(ast(LineAst::OptionalCost(
        crate::model::CompilerOptionalCost::custom(
            "Offering",
            ironsmith_core::TotalCost::from_cost(cost),
        ),
    )))
}

fn read_mayhem(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let Some(tokens) =
        keyword_tokens_for_shape(input.tokens, input.full_tokens, KeywordPrefixShape::Mayhem)
    else {
        return Ok(None);
    };
    if crate::grammar::primitives::TokenWordView::new(tokens)
        .word_refs()
        .len()
        == 1
        && leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default()).is_none()
    {
        let filter = crate::target::ObjectFilter::source()
            .discarded_or_cycled_this_turn_by(crate::target::PlayerFilter::You);
        let mut ability = crate::model::CompilerStaticAbilityCore::grants(
            crate::model::CompilerGrantSpecCore::new(
                crate::model::CompilerGrantableCore::play_from(),
                filter,
                crate::zone::Zone::Graveyard,
            ),
        );
        ability.label = "Mayhem".into();
        return Ok(ast(LineAst::StaticAbility(ability.into())));
    }
    let (cost, _) = leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default())
        .ok_or_else(|| CardTextError::ParseError("mayhem keyword missing mana cost".into()))?;
    let condition = crate::static_abilities::ThisSpellCostCondition::ConditionExpr {
        condition: crate::effect::Condition::SourceMatches(
            crate::target::ObjectFilter::default()
                .discarded_or_cycled_this_turn_by(crate::target::PlayerFilter::You),
        ),
        display: "you discarded this card this turn".into(),
    };
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::model::CompilerAlternativeCastingMethod::cast_from_zone_with_total_cost(
            "Mayhem",
            crate::zone::Zone::Graveyard,
            ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::Mana(cost)),
            Some(condition),
            false,
        ),
    )))
}

fn read_web_slinging(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let Some(tokens) = keyword_tokens_for_shape(
        input.tokens,
        input.full_tokens,
        KeywordPrefixShape::WebSlinging,
    ) else {
        return Ok(None);
    };
    let (cost, _) =
        leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("web-slinging keyword missing mana cost".into())
        })?;
    let mut filter = crate::target::ObjectFilter::creature().you_control();
    filter.tapped = true;
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::model::CompilerAlternativeCastingMethod::alternative_cost(
            "Web-slinging",
            Some(cost),
            vec![crate::model::CompilerCost::ReturnChosenToHand { count: 1, filter }],
        ),
    )))
}

fn read_sneak(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    let full_tokens = input.full_tokens;
    if let Some(keyword_tokens) =
        keyword_tokens_for_shape(tokens, full_tokens, KeywordPrefixShape::Sneak)
    {
        let price = keyword_tokens.get(1..).unwrap_or_default();
        let start = price.iter().position(|token| !matches!(token.kind, TokenKind::Dash | TokenKind::EmDash)).unwrap_or(price.len());
        let total = crate::activation_and_restrictions::parse_compiler_activation_cost(&price[start..])?;
        let cost = total.mana_cost().cloned().unwrap_or_default();
        let mut non_mana = total.non_mana_costs().cloned().collect::<Vec<_>>();
        non_mana.push(crate::model::CompilerCost::Sneak);
        return Ok(ast(LineAst::AlternativeCastingMethod(
            crate::model::CompilerAlternativeCastingMethod::alternative_cost(
                "Sneak",
                Some(cost),
                non_mana,
            ),
        )));
    }
    Ok(None)
}
fn read_blitz_from_graveyard(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    let words = crate::lexer::parser_token_word_refs(tokens);
    if let [
        "you",
        "may",
        "cast",
        "this",
        "card",
        "from",
        "your",
        "graveyard",
        "using",
        "its",
        keyword,
        "ability",
    ] = words.as_slice()
    {
        use ironsmith_core::alternative_cast_model::AlternativeCastKeyword;
        let method = match *keyword {
            "bestow" => AlternativeCastKeyword::Bestow,
            "blitz" => AlternativeCastKeyword::Blitz,
            "warp" => AlternativeCastKeyword::Warp,
            "mutate" => AlternativeCastKeyword::Mutate,
            _ => return Ok(None),
        };
        let ability = crate::model::CompilerStaticAbilityCore::native_alternative_cast_from_zone(
            crate::zone::Zone::Graveyard,
            method,
        );
        return Ok(ast(LineAst::StaticAbility(ability.into())));
    }
    Ok(None)
}
fn read_self_free_cast_alternative_cost_line(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    if let Some(method) = parse_self_free_cast_alternative_cost_line_lexed(tokens) {
        return Ok(ast(LineAst::AlternativeCastingMethod(method)));
    }
    Ok(None)
}
fn read_flash_with_additional_cost_line(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    if let Some(method) = parse_flash_with_additional_cost_line_lexed(tokens) {
        return Ok(ast(LineAst::AlternativeCastingMethod(method)));
    }
    Ok(None)
}
fn read_flash_timing_optional_payment_line(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    if let Some(optional) =
        crate::grammar::shared_util::alternative_cost_lines::parse_flash_timing_optional_payment(input.tokens)?
    {
        return Ok(ast(LineAst::OptionalCost(optional)));
    }
    Ok(None)
}
fn read_jump_start_line(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    if let Some(method) = parse_jump_start_line_lexed(tokens)? {
        return Ok(ast(LineAst::AlternativeCastingMethod(method)));
    }
    Ok(None)
}
fn read_you_may_rather_than_spell_cost_line(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    let line = input.line;
    let surface = line.info.normalized.normalized.as_str();
    if let Some(method) = parse_you_may_rather_than_spell_cost_line_lexed(tokens, surface)? {
        return Ok(ast(LineAst::AlternativeCastingMethod(method)));
    }
    Ok(None)
}
fn read_if_conditional_alternative_cost_line(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    let line = input.line;
    if let Some(method) = parse_if_conditional_alternative_cost_line_lexed(tokens, &line.tokens)? {
        return Ok(ast(LineAst::AlternativeCastingMethod(method)));
    }
    Ok(None)
}
fn read_prowl_line(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    if let Some(method) = parse_prowl_line_lexed(tokens)? {
        return Ok(ast(LineAst::AlternativeCastingMethod(method)));
    }
    Ok(None)
}
fn read_if_this_spell_costs_less_to_cast_line(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(tokens)? {
        return Ok(ast(LineAst::StaticAbility(ability.into())));
    }
    Ok(None)
}

fn read_entry_counter_countdown(input: &AlternativeCastLine<'_>) -> Result<Option<KeywordLinePayload>, CardTextError> {
    use crate::cards::builders::{ConditionalEffectAst, SourcePredicateAst};
    let sentences = split_lexed_sentences(input.tokens);
    let [first, second] = sentences.as_slice() else { return Ok(None); };
    let view = TokenWordView::new(first);
    let words = view.word_refs();
    if words.get(..6) != Some(&["if", "you", "cast", "this", "spell", "for"]) { return Ok(None); }
    let Some(enters) = words.windows(3).position(|w| w == ["it", "enters", "with"]) else { return Ok(None); };
    let Some(counters) = words.iter().position(|w| *w == "counters") else { return Ok(None); };
    if counters <= enters + 4 || !matches!(&words[counters + 1..], ["and", "isn't" | "isnt", "a", "creature", "until", "the", "last", "is", "removed"]) { return Ok(None); }
    let Some(amount) = crate::util::parse_number_word_u32(words[enters + 3]) else { return Ok(None); };
    let Some(counter_type) = crate::grammar::filters::parse_counter_type_words(&words[enters + 4..=counters]) else { return Ok(None); };
    let second_words = TokenWordView::new(second).word_refs();
    if second_words.get(..9) != Some(&["at", "the", "beginning", "of", "your", "end", "step", "remove", "a"])
        || !second_words.ends_with(&["counter", "from", "it"])
        || crate::grammar::filters::parse_counter_type_words(&second_words[9..second_words.len() - 2]) != Some(counter_type) { return Ok(None); }
    let cost_start = view.token_start_indices()[5] + 1;
    let cost_end = view.token_start_indices()[enters];
    let total_cost = crate::activation_and_restrictions::parse_compiler_activation_cost(crate::util::trim_edge_punctuation_tokens(&first[cost_start..cost_end]))?;
    let method = crate::model::CompilerAlternativeCastingMethod::Composed {
        name: "Parsed entry countdown price".into(), total_cost, condition: None, prototype_power_toughness: None,
    };
    let paid_ref = ironsmith_core::OptionalCostRef::new(ironsmith_core::OptionalCostKind::AlternativeCast(
        ironsmith_core::AlternativeCostReference::as_that_cost(method.name(), method.mana_cost()),
    ));
    let paid = PredicateAst::ThisSpellPaidLabel(paid_ref.clone());
    let predicate = PredicateAst::And(Box::new(PredicateAst::ThisSpellPaidLabel(paid_ref)), Box::new(PredicateAst::Source(SourcePredicateAst::SourceHasCounterAtLeast { counter_type, count: 1, surface: Default::default() })));
    Ok(ast(LineAst::Multiple(vec![
        LineAst::AlternativeCastingMethod(method),
        LineAst::StaticAbility(crate::model::CompilerStaticAbilityCore::enters_with_counters_value(counter_type, amount.into()).with_condition(paid).into()),
        LineAst::StaticAbility(crate::model::CompilerStaticAbilityCore::remove_card_types(ObjectFilter::source(), vec![crate::types::CardType::Creature], Some(predicate.clone())).into()),
        LineAst::Triggered {
            trigger: TriggerSpec::BeginningOfEndStep(PlayerFilter::You),
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate, if_true: vec![EffectAst::subject_verb_remove_counters_all(Value::Fixed(1), ObjectFilter::source(), Some(counter_type), false)], if_false: vec![],
            })], max_triggers_per_turn: None,
        },
    ])))
}

fn read_optional_all_modes(input: &AlternativeCastLine<'_>) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(input.tokens);
    let prefix = if tokens.iter().take(5).map(|t| t.as_word().unwrap_or("")).eq(["choose", "both", "if", "you", "pay"]) { 5 }
        else if tokens.iter().take(6).map(|t| t.as_word().unwrap_or("")).eq(["choose", "all", "modes", "if", "you", "pay"]) { 6 }
        else if tokens.iter().take(4).map(|t| t.as_word().unwrap_or("")).eq(["choose", "both", "if", "you"]) { 4 }
        else { return Ok(None); };
    let cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&tokens[prefix..])?;
    Ok(ast(LineAst::OptionalCost(crate::model::CompilerOptionalCost::choose_all_modes(cost))))
}

fn read_payment_per_extra_mode(input: &AlternativeCastLine<'_>) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(input.tokens);
    if tokens.len() < 9 || !crate::lexer::parser_token_word_refs(tokens).ends_with(&["for", "each", "mode", "chosen", "beyond", "the", "first"]) { return Ok(None); }
    let mut price = &tokens[..tokens.len() - 7];
    if price.first().is_some_and(|t| t.is_word("pay")) && price.get(1).is_some_and(|t| t.slice.starts_with('{')) { price = &price[1..]; }
    let cost = crate::activation_and_restrictions::parse_compiler_activation_cost(price)?;
    Ok(ast(LineAst::StaticAbility(crate::model::CompilerStaticAbilityCore::payment_per_extra_mode(
        cost, crate::lexer::render_token_slice(price),
    ).into())))
}

fn read_reveal_to_add_spell_text(input: &AlternativeCastLine<'_>) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let sentences = crate::lexer::split_lexed_sentences(input.tokens);
    if sentences.len() != 2 { return Ok(None); }
    let first = crate::util::trim_edge_punctuation_tokens(&sentences[0]);
    let words = crate::lexer::parser_token_word_refs(first);
    if !words.starts_with(&["as", "you", "cast", "an"]) && !words.starts_with(&["as", "you", "cast", "a"]) { return Ok(None); }
    let Some(spell) = first.iter().position(|t| t.is_word("spell")) else { return Ok(None); };
    let quality = match crate::lexer::parser_token_word_refs(&first[4..spell]).as_slice() {
        ["instant", "or", "sorcery"] => ironsmith_core::SpliceQuality::InstantOrSorcery,
        ["arcane"] => ironsmith_core::SpliceQuality::Arcane,
        _ => return Ok(None),
    };
    let tail = crate::util::trim_edge_punctuation_tokens(&first[spell + 1..]);
    let intro = ["you", "may", "reveal", "this", "card", "from", "your", "hand", "and"];
    if !crate::lexer::parser_token_word_refs(tail).starts_with(&intro) { return Ok(None); }
    let last = crate::lexer::parser_token_word_refs(&sentences[1]);
    if !matches!(last.as_slice(), ["if", "you", "do", "add", "this", "card's" | "cards", "effects", "to", "that", "spell"]) { return Ok(None); }
    let mut price = &tail[intro.len()..];
    if price.first().is_some_and(|t| t.is_word("pay")) && price.get(1).is_some_and(|t| t.slice.starts_with('{')) { price = &price[1..]; }
    let cost = crate::activation_and_restrictions::parse_compiler_activation_cost(price)?;
    Ok(ast(LineAst::StaticAbility(crate::model::CompilerStaticAbilityCore::reveal_to_add_spell_text(quality, cost, crate::lexer::render_token_slice(price)).into())))
}
