use crate::ability::ActivationTiming;
use crate::activation_and_restrictions::activated_line_core::parse_activation_cost;
use crate::cards::builders::{CardTextError, ParsedAbility, ReferenceImports};
use crate::costs::Cost;
use crate::effect::Value;
use crate::filter::Comparison;
use crate::grammar::{leaf, permission_shapes};
use crate::lexer::{OwnedLexToken, TokenKind, TokenWordView, render_token_slice};
use crate::model::CompilerAlternativeCastingMethod as AlternativeCastingMethod;
use crate::model::CompilerCost;
use crate::model::CompilerOptionalCost as OptionalCost;
use crate::model::CompilerStaticAbilityCore as StaticAbility;
use crate::model::compiler_semantic::{
    CompilerAbilityCore as Ability, CompilerAbilityKindCore as AbilityKind,
};
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

pub fn parse_buyback(tokens: &[OwnedLexToken]) -> Result<Option<OptionalCost>, CardTextError> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if !permission_shapes::prefix_words(&words, &["buyback"]) {
        return Ok(None);
    }
    if permission_shapes::prefix_words(&words, &["buyback", "costs"]) {
        return Ok(None);
    }
    let cost_tokens = keyword_cost_clause(tokens, 1, ReminderBoundary::MayPay);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "buyback keyword missing cost".to_string(),
        ));
    }
    Ok(Some(OptionalCost::buyback(parse_activation_cost(
        cost_tokens,
    )?)))
}

pub fn parse_optional_cost(
    tokens: &[OwnedLexToken],
    keyword: &str,
    constructor: fn(ironsmith_core::TotalCost<CompilerCost>) -> OptionalCost,
) -> Result<Option<OptionalCost>, CardTextError> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if words
        .first()
        .is_none_or(|word| !permission_shapes::exact_words(&[*word], &[keyword]))
    {
        return Ok(None);
    }
    let cost_tokens = keyword_cost_clause(tokens, 1, ReminderBoundary::MayPayOrPeriod);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{keyword} keyword missing cost"
        )));
    }
    Ok(Some(constructor(parse_activation_cost(cost_tokens)?)))
}

#[derive(Clone, Copy)]
enum MorphKind {
    Morph,
    Megamorph,
    Disguise,
}

impl MorphKind {
    fn parser_name(self) -> &'static str {
        match self {
            Self::Morph => "morph",
            Self::Megamorph => "megamorph",
            Self::Disguise => "disguise",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Morph => "Morph",
            Self::Megamorph => "Megamorph",
            Self::Disguise => "Disguise",
        }
    }

    fn static_ability(self, cost: ironsmith_core::TotalCost<CompilerCost>) -> StaticAbility {
        match self {
            Self::Morph => StaticAbility::morph(cost),
            Self::Megamorph => StaticAbility::megamorph(cost),
            Self::Disguise => StaticAbility::disguise(cost),
        }
    }
}

fn parse_compiler_total_cost(
    tokens: &[OwnedLexToken],
) -> Result<ironsmith_core::TotalCost<CompilerCost>, CardTextError> {
    let cst = crate::grammar::activation_costs::parse_activation_cost_tokens(tokens)?;
    Ok(crate::semantic_assembly::assemble_activation_cost(&cst)?.to_core_total_cost())
}

pub fn parse_morph(tokens: &[OwnedLexToken]) -> Result<Option<ParsedAbility>, CardTextError> {
    let words = TokenWordView::new(tokens).word_refs();
    let Some(kind) = words.first().and_then(|word| match *word {
        "morph" => Some(MorphKind::Morph),
        "megamorph" => Some(MorphKind::Megamorph),
        "disguise" => Some(MorphKind::Disguise),
        _ => None,
    }) else {
        return Ok(None);
    };

    let cost_tokens = morph_cost_clause(tokens);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{} keyword missing cost",
            kind.parser_name()
        )));
    }
    let unsupported = || {
        CardTextError::ParseError(format!(
            "unsupported {} cost clause (line: '{}')",
            kind.parser_name(),
            render_token_slice(cost_tokens).trim()
        ))
    };
    let cost = match parse_compiler_total_cost(cost_tokens) {
        Ok(cost) if !cost.is_free() => cost,
        _ if leaf::parse_leaf_mana_cost_prefix_tokens(cost_tokens).is_some() => {
            return Err(unsupported());
        }
        _ => return Err(unsupported()),
    };
    if cost.is_free() {
        return Err(CardTextError::ParseError(format!(
            "{} keyword missing cost",
            kind.parser_name()
        )));
    }
    Ok(Some(ParsedAbility {
        ability: Ability::static_ability(kind.static_ability(cost)).into(),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub fn parse_escape(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if !permission_shapes::prefix_words(&words, &["escape"]) {
        return Ok(None);
    }
    if tokens.len() <= 1 {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }
    let cost_start = if tokens
        .get(1)
        .is_some_and(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
    {
        2
    } else {
        1
    };
    let comma = first_kind_after(tokens, cost_start, TokenKind::Comma).ok_or_else(|| {
        CardTextError::ParseError("escape keyword missing exile clause separator".to_string())
    })?;
    if comma <= cost_start {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }
    let total_cost = parse_activation_cost(&tokens[cost_start..comma])?;
    let mana_cost = total_cost.mana_cost().cloned().ok_or_else(|| {
        CardTextError::ParseError("escape keyword missing mana symbols".to_string())
    })?;
    let tail = trim_edge_commas(tokens.get(comma + 1..).unwrap_or_default());
    if tail.is_empty() {
        return Err(CardTextError::ParseError(
            "escape keyword missing exile clause".to_string(),
        ));
    }
    let tail_view = TokenWordView::new(tail);
    let tail_words = tail_view.word_refs();
    if !permission_shapes::prefix_words(&tail_words, &["exile"]) {
        return Err(unsupported_escape(&tail_words));
    }
    let count_start = tail_view
        .token_start_indices()
        .get(1)
        .copied()
        .unwrap_or(tail.len());
    let parsed = leaf::parse_leaf_number_prefix_tokens(&tail[count_start..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "escape keyword missing exile count (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;
    let (count, consumed) = parsed.into_fixed().ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported escape exile count (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;
    let after_count = count_start + consumed;
    let remaining_words =
        TokenWordView::new(tail.get(after_count..).unwrap_or_default()).word_refs();
    let noun_word = usize::from(remaining_words.first().is_some_and(|word| *word == "other"));
    if remaining_words.get(noun_word).is_none_or(|word| {
        !permission_shapes::exact_words(&[*word], &["card"])
            && !permission_shapes::exact_words(&[*word], &["cards"])
    }) {
        return Err(CardTextError::ParseError(format!(
            "escape keyword missing exiled card noun (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    if !permission_shapes::exact_words(
        remaining_words.get(noun_word + 1..).unwrap_or_default(),
        &["from", "your", "graveyard"],
    ) {
        return Err(unsupported_escape(&tail_words));
    }
    Ok(Some(AlternativeCastingMethod::Escape {
        cost: Some(mana_cost),
        exile_count: count,
        additional_cost: ironsmith_core::TotalCost::from_cost(CompilerCost::ExileChosen {
            count: crate::effect::ChoiceCount::exactly(count as usize),
            filter: ObjectFilter::default()
                .owned_by(crate::target::PlayerFilter::You)
                .in_zone(Zone::Graveyard),
            top_only: false,
            turn_face_up: false,
            binding: None,
        }),
    }))
}

pub fn parse_jump_start(tokens: &[OwnedLexToken]) -> Option<AlternativeCastingMethod> {
    let words = TokenWordView::new(tokens).word_refs();
    (permission_shapes::prefix_words(&words, &["jumpstart"])
        || permission_shapes::prefix_words(&words, &["jump", "start"]))
    .then(|| AlternativeCastingMethod::JumpStart {
        additional_cost: ironsmith_core::TotalCost::from_cost(CompilerCost::Discard {
            count: 1,
            card_types: Vec::new(),
            supertypes: Vec::new(),
            filter: None,
            random: false,
            name: None,
            other: false,
            binding: None,
        }),
    })
}

pub fn parse_bestow(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !starts_with_keyword(tokens, "bestow") {
        return Ok(None);
    }
    let mana_prefix = leaf::parse_leaf_mana_cost_prefix_tokens(&tokens[1..])
        .ok_or_else(|| CardTextError::ParseError("bestow keyword missing mana cost".to_string()))?;
    let mana_cost = mana_prefix.cost;
    let mut total_cost = ironsmith_core::TotalCost::<CompilerCost>::mana(mana_cost.clone());
    let mut cost_tokens = tokens[1..1 + mana_prefix.consumed].to_vec();
    let tail = tokens.get(1 + mana_prefix.consumed..).unwrap_or_default();
    if tail.first().is_some_and(OwnedLexToken::is_comma) {
        let clause_end = first_kind_after(tail, 0, TokenKind::Period).unwrap_or(tail.len());
        let clause = trim_edge_commas(&tail[..clause_end]);
        if !permission_shapes::prefix_words(&TokenWordView::new(clause).word_refs(), &["if"]) {
            cost_tokens.extend_from_slice(clause);
        }
    }
    if let Ok(parsed) = parse_activation_cost(&cost_tokens) {
        total_cost = ensure_mana_component(parsed, mana_cost);
    }
    Ok(Some(AlternativeCastingMethod::Bestow { total_cost }))
}

pub fn parse_blitz(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !starts_with_keyword(tokens, "blitz") {
        return Ok(None);
    }
    let mana_prefix = leaf::parse_leaf_mana_cost_prefix_tokens(&tokens[1..])
        .ok_or_else(|| CardTextError::ParseError("blitz keyword missing mana cost".to_string()))?;
    let mana_cost = mana_prefix.cost;
    let mut total_cost = ironsmith_core::TotalCost::<CompilerCost>::mana(mana_cost.clone());
    let mut cost_tokens = tokens[1..1 + mana_prefix.consumed].to_vec();
    let tail = tokens.get(1 + mana_prefix.consumed..).unwrap_or_default();
    if tail.first().is_some_and(OwnedLexToken::is_comma) {
        let clause_end = first_kind_after(tail, 0, TokenKind::Period).unwrap_or(tail.len());
        cost_tokens.extend_from_slice(&tail[..clause_end]);
    }
    if let Ok(parsed) = parse_activation_cost(&cost_tokens) {
        total_cost = ensure_mana_component(parsed, mana_cost);
    }
    let words = TokenWordView::new(tail).word_refs();
    if let Some(pay) = permission_shapes::find_words(&words, &["pay"])
        && words.get(pay + 2).is_some_and(|word| *word == "life")
        && !total_cost
            .costs()
            .iter()
            .any(|cost| matches!(cost, CompilerCost::Life(_)))
        && let Some(amount) = words.get(pay + 1).and_then(|word| parse_fixed_word(word))
    {
        let mut components = total_cost.costs().to_vec();
        components.push(CompilerCost::Life(Value::Fixed(amount as i32)));
        total_cost = ironsmith_core::TotalCost::from_costs(components);
    }
    Ok(Some(AlternativeCastingMethod::Blitz { total_cost }))
}

pub fn parse_transmute(tokens: &[OwnedLexToken]) -> Result<Option<ParsedAbility>, CardTextError> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if !permission_shapes::prefix_words(&words, &["transmute"])
        || words.iter().any(|word| matches!(*word, "has" | "have"))
    {
        return Ok(None);
    }
    let mana_prefix = leaf::parse_leaf_mana_cost_prefix_tokens(&tokens[1..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "transmute keyword missing mana cost (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let base_mana_cost = mana_prefix.cost;
    let mana_cost = ironsmith_core::TotalCost::from_costs(vec![
        CompilerCost::Mana(base_mana_cost.clone()),
        CompilerCost::DiscardSource,
    ]);
    let parsed_mana_value = permission_shapes::find_words(&words, &["mana", "value"])
        .and_then(|word| view.token_start_indices().get(word + 2).copied())
        .and_then(|token| leaf::parse_leaf_number_prefix_tokens(&tokens[token..]))
        .and_then(|parsed| parsed.into_fixed().map(|(value, _)| value));
    let filter = if let Some(mana_value) = parsed_mana_value {
        ObjectFilter::default().with_mana_value(Comparison::Equal(mana_value as i32))
    } else {
        ObjectFilter::default().with_mana_value(Comparison::EqualExpr(Box::new(
            Value::ManaValueOf(Box::new(ChooseSpec::Source)),
        )))
    };
    let effects_ast = vec![
        crate::cards::builders::EffectAst::subject_verb_search_library(
            filter,
            Zone::Hand,
            crate::cards::builders::PlayerAst::You,
            crate::cards::builders::PlayerAst::You,
            crate::effect::SearchSelectionMode::Exact,
            true,
            None,
            true,
            crate::effect::ChoiceCount::exactly(1),
            None,
            None,
            crate::effect::SearchResultReferenceSurface::ThatCard,
            false,
            false,
            false,
        ),
    ];
    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(
                crate::model::compiler_semantic::CompilerActivatedAbilityCore {
                    mana_cost,
                    effects: ironsmith_core::ResolutionProgram::default(),
                    choices: Vec::new(),
                    timing: ActivationTiming::SorcerySpeed,
                    additional_restrictions: Vec::new(),
                    activation_restrictions: Vec::new(),
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                    is_loyalty_ability: false,
                },
            ),
            functional_zones: vec![Zone::Hand],
        }
        .into(),
        effects_ast: Some(effects_ast),
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

#[derive(Clone, Copy)]
enum ReminderBoundary {
    MayPay,
    MayPayOrPeriod,
}

#[path = "keyword_cost_lines/core.rs"]
mod core_programs;
pub use core_programs::parse_transfigure;
use core_programs::{first_kind_after, parse_fixed_word, trim_edge_commas, unsupported_escape};
#[path = "keyword_cost_lines/ability.rs"]
mod ability_programs;
use ability_programs::starts_with_keyword;
#[path = "keyword_cost_lines/resource.rs"]
mod resource_programs;
use resource_programs::{ensure_mana_component, keyword_cost_clause, morph_cost_clause};
