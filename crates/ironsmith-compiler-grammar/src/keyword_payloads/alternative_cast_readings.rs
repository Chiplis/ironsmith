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
        id: RuleId::new("emerge-from"), head: HeadDiscriminator::Any, admits: |_| true,
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
    let tokens = input.tokens;
    if token_slice_first_is(tokens, "aftermath") {
        let mut ability = crate::model::CompilerStaticAbilityCore::grants(
            crate::model::CompilerGrantSpecCore::new(
                crate::model::CompilerGrantableCore::graveyard_cast_from_cards_mana_cost(
                    Vec::<crate::model::CompilerCost>::new(),
                    true,
                ),
                crate::target::ObjectFilter::source(),
                crate::zone::Zone::Graveyard,
            ),
        );
        ability.label = "Aftermath".to_string();
        return Ok(ast(LineAst::StaticAbility(ability.into())));
    }
    Ok(None)
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
        let (cost, _) = leading_mana_cost_from_tokens(keyword_tokens.get(1..).unwrap_or_default())
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
fn read_more_than_meets_the_eye(input: &AlternativeCastLine<'_>) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let Some(tokens) = keyword_tokens_for_shape(input.tokens, input.full_tokens, KeywordPrefixShape::MoreThanMeetsTheEye) else { return Ok(None); };
    let (cost, _) = leading_mana_cost_from_tokens(tokens.get(5..).unwrap_or_default())
        .ok_or_else(|| CardTextError::ParseError("more than meets the eye keyword missing mana cost".into()))?;
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::model::CompilerAlternativeCastingMethod::alternative_cost("More than meets the eye", Some(cost), vec![])
    )))
}

fn read_emerge_from(input: &AlternativeCastLine<'_>) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let words = crate::grammar::primitives::TokenWordView::new(input.tokens).word_refs();
    if !words.starts_with(&["emerge", "from"]) { return Ok(None); }
    let quality = words.get(2).ok_or_else(|| CardTextError::ParseError("emerge from missing quality".into()))?;
    let mut filter = crate::target::ObjectFilter::default().you_control();
    if let Some(card_type) = crate::util::parse_card_type(quality) { filter.card_types.push(card_type); }
    else if let Some(subtype) = crate::util::parse_subtype_word(quality) { filter.subtypes.push(subtype); }
    else { return Err(CardTextError::ParseError("unsupported emerge permanent quality".into())); }
    let (cost, _) = leading_mana_cost_from_tokens(input.tokens.get(3..).unwrap_or_default())
        .ok_or_else(|| CardTextError::ParseError("emerge from missing mana cost".into()))?;
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::model::CompilerAlternativeCastingMethod::alternative_cost("Emerge", Some(cost),
            vec![crate::model::CompilerCost::Sacrifice { count: ironsmith_core::ChoiceCount::exactly(1), filter, all: false, binding: None }])
    )))
}

fn read_offering(input: &AlternativeCastLine<'_>) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let words = crate::grammar::primitives::TokenWordView::new(input.tokens).word_refs();
    if words.len() != 2 || words[1] != "offering" { return Ok(None); }
    let subtype = crate::util::parse_subtype_word(words[0])
        .ok_or_else(|| CardTextError::ParseError("unknown offering permanent quality".into()))?;
    let mut filter = crate::target::ObjectFilter::default().you_control();
    filter.subtypes.push(subtype);
    let cost = crate::model::CompilerCost::Sacrifice { count: ironsmith_core::ChoiceCount::exactly(1),
        filter, all: false, binding: None };
    Ok(ast(LineAst::OptionalCost(crate::model::CompilerOptionalCost::custom("Offering", ironsmith_core::TotalCost::from_cost(cost)))))
}

fn read_mayhem(input: &AlternativeCastLine<'_>) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let Some(tokens) = keyword_tokens_for_shape(input.tokens, input.full_tokens, KeywordPrefixShape::Mayhem) else { return Ok(None); };
    if crate::grammar::primitives::TokenWordView::new(tokens).word_refs().len() == 1
        && leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default()).is_none() {
        let filter = crate::target::ObjectFilter::source()
            .discarded_or_cycled_this_turn_by(crate::target::PlayerFilter::You);
        let mut ability = crate::model::CompilerStaticAbilityCore::grants(
            crate::model::CompilerGrantSpecCore::new(crate::model::CompilerGrantableCore::play_from(), filter,
                crate::zone::Zone::Graveyard));
        ability.label = "Mayhem".into();
        return Ok(ast(LineAst::StaticAbility(ability.into())));
    }
    let (cost, _) = leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default())
        .ok_or_else(|| CardTextError::ParseError("mayhem keyword missing mana cost".into()))?;
    let condition = crate::static_abilities::ThisSpellCostCondition::ConditionExpr {
        condition: crate::effect::Condition::SourceMatches(crate::target::ObjectFilter::default()
            .discarded_or_cycled_this_turn_by(crate::target::PlayerFilter::You)),
        display: "you discarded this card this turn".into(),
    };
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::model::CompilerAlternativeCastingMethod::cast_from_zone_with_total_cost("Mayhem",
            crate::zone::Zone::Graveyard, ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::Mana(cost)), Some(condition), false)
    )))
}

fn read_web_slinging(input: &AlternativeCastLine<'_>) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let Some(tokens) = keyword_tokens_for_shape(input.tokens, input.full_tokens, KeywordPrefixShape::WebSlinging) else { return Ok(None); };
    let (cost, _) = leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default())
        .ok_or_else(|| CardTextError::ParseError("web-slinging keyword missing mana cost".into()))?;
    let mut filter = crate::target::ObjectFilter::creature().you_control();
    filter.tapped = true;
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::model::CompilerAlternativeCastingMethod::alternative_cost("Web-slinging", Some(cost),
            vec![crate::model::CompilerCost::ReturnChosenToHand { count: 1, filter }])
    )))
}

fn read_sneak(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    let line = input.line;
    let full_tokens = input.full_tokens;
    if let Some(keyword_tokens) =
        keyword_tokens_for_shape(tokens, full_tokens, KeywordPrefixShape::Sneak)
    {
        let support_tokens = if full_tokens.is_empty() {
            keyword_tokens
        } else {
            full_tokens
        };
        if !line.info.semantic_facts.supported_sneak_form && !is_supported_sneak_line(support_tokens) {
            return Err(CardTextError::ParseError(format!(
                "sneak keyword form is not yet supported: '{}'",
                line.info.raw_line
            )));
        }
        let (cost, _) = leading_mana_cost_from_tokens(keyword_tokens.get(1..).unwrap_or_default())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "sneak keyword missing cost '{}'",
                    line.info.raw_line
                ))
            })?;
        return Ok(ast(LineAst::AlternativeCastingMethod(
            crate::model::CompilerAlternativeCastingMethod::alternative_cost(
                "Sneak",
                Some(cost),
                vec![crate::model::CompilerCost::Sneak],
            ),
        )));
    }
    Ok(None)
}
fn read_blitz_from_graveyard(
    input: &AlternativeCastLine<'_>,
) -> Result<Option<KeywordLinePayload>, CardTextError> {
    let tokens = input.tokens;
    if parse_keyword_special_form_shape_tokens(tokens)
        == Some(KeywordSpecialFormShape::BlitzFromGraveyard)
    {
        return Ok(ast(LineAst::Abilities(vec![
            crate::cards::builders::KeywordAction::BlitzFromGraveyard,
        ])));
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
