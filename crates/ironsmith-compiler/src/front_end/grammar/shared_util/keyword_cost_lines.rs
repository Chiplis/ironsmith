use crate::ability::{Ability, AbilityKind, ActivationTiming};
use crate::activation_and_restrictions::activated_line_core::parse_activation_cost;
use crate::activation_and_restrictions::parse_payment_clause_as_total_cost;
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::builders::{CardTextError, ParsedAbility, ReferenceImports};
use crate::cost::{OptionalCost, TotalCost};
use crate::costs::Cost;
use crate::effect::{Effect, Value};
use crate::filter::Comparison;
use crate::grammar::{leaf, permission_shapes};
use crate::lexer::{OwnedLexToken, TokenKind, TokenWordView, render_token_slice};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

pub(crate) fn parse_buyback(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
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

pub(crate) fn parse_optional_cost(
    tokens: &[OwnedLexToken],
    keyword: &str,
    constructor: fn(TotalCost) -> OptionalCost,
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

    fn static_ability(self, cost: TotalCost) -> StaticAbility {
        match self {
            Self::Morph => StaticAbility::morph(cost),
            Self::Megamorph => StaticAbility::megamorph(cost),
            Self::Disguise => StaticAbility::disguise(cost),
        }
    }
}

pub(crate) fn parse_morph(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
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
    let cost = match parse_activation_cost(cost_tokens) {
        Ok(cost) if !cost.is_free() => cost,
        _ if leaf::parse_leaf_mana_cost_prefix_tokens(cost_tokens).is_some() => {
            return Err(unsupported());
        }
        _ => parse_payment_clause_as_total_cost(cost_tokens)?.ok_or_else(unsupported)?,
    };
    if cost.is_free() {
        return Err(CardTextError::ParseError(format!(
            "{} keyword missing cost",
            kind.parser_name()
        )));
    }
    let text = format!("{}—{}", kind.display_name(), cost.display());
    Ok(Some(ParsedAbility {
        ability: Ability::static_ability(kind.static_ability(cost)).into(),
        text: Some(text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_escape(
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
    let comma = first_kind_after(tokens, 1, TokenKind::Comma).ok_or_else(|| {
        CardTextError::ParseError("escape keyword missing exile clause separator".to_string())
    })?;
    if comma <= 1 {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }
    let total_cost = parse_activation_cost(&tokens[1..comma])?;
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
        additional_cost: TotalCost::from_cost(Cost::exile_from_graveyard(count, Vec::new())),
    }))
}

pub(crate) fn parse_jump_start(tokens: &[OwnedLexToken]) -> Option<AlternativeCastingMethod> {
    let words = TokenWordView::new(tokens).word_refs();
    (permission_shapes::prefix_words(&words, &["jumpstart"])
        || permission_shapes::prefix_words(&words, &["jump", "start"]))
    .then(|| AlternativeCastingMethod::JumpStart {
        additional_cost: TotalCost::from_cost(Cost::discard(1, None)),
    })
}

pub(crate) fn parse_bestow(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !starts_with_keyword(tokens, "bestow") {
        return Ok(None);
    }
    let mana_prefix = leaf::parse_leaf_mana_cost_prefix_tokens(&tokens[1..])
        .ok_or_else(|| CardTextError::ParseError("bestow keyword missing mana cost".to_string()))?;
    let mana_cost = mana_prefix.cost;
    let mut total_cost = TotalCost::mana(mana_cost.clone());
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

pub(crate) fn parse_blitz(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !starts_with_keyword(tokens, "blitz") {
        return Ok(None);
    }
    let mana_prefix = leaf::parse_leaf_mana_cost_prefix_tokens(&tokens[1..])
        .ok_or_else(|| CardTextError::ParseError("blitz keyword missing mana cost".to_string()))?;
    let mana_cost = mana_prefix.cost;
    let mut total_cost = TotalCost::mana(mana_cost.clone());
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
            .any(|cost| matches!(cost, Cost::Life(_)))
        && let Some(amount) = words.get(pay + 1).and_then(|word| parse_fixed_word(word))
    {
        let mut components = total_cost.costs().to_vec();
        components.push(Cost::life(Value::Fixed(amount as i32)));
        total_cost = TotalCost::from_costs(components);
    }
    Ok(Some(AlternativeCastingMethod::Blitz { total_cost }))
}

pub(crate) fn parse_transmute(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
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
    let mut merged_costs = TotalCost::mana(base_mana_cost.clone()).costs().to_vec();
    merged_costs.push(Cost::discard_source());
    let mana_cost = TotalCost::from_costs(merged_costs);
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
    let text = format!("Transmute {}", base_mana_cost.to_oracle());
    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::search_library_to_hand(filter, true),
                ]),
                choices: Vec::new(),
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: Vec::new(),
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Hand],
        }
        .into(),
        text: Some(text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_transfigure(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if !permission_shapes::prefix_words(&words, &["transfigure"])
        || words.iter().any(|word| matches!(*word, "has" | "have"))
    {
        return Ok(None);
    }
    let mana_prefix = leaf::parse_leaf_mana_cost_prefix_tokens(&tokens[1..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "transfigure keyword missing mana cost (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let base_mana_cost = mana_prefix.cost;
    let mut merged_costs = TotalCost::mana(base_mana_cost.clone()).costs().to_vec();
    merged_costs.push(Cost::sacrifice_self());
    let mana_cost = TotalCost::from_costs(merged_costs);
    let filter = ObjectFilter::default()
        .with_type(crate::types::CardType::Creature)
        .with_mana_value(Comparison::EqualExpr(Box::new(Value::ManaValueOf(
            Box::new(ChooseSpec::Source),
        ))));
    let text = format!("Transfigure {}", base_mana_cost.to_oracle());
    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                    crate::effects::SearchLibraryEffect::to_battlefield(
                        filter,
                        crate::target::PlayerFilter::You,
                        false,
                    ),
                )]),
                choices: Vec::new(),
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: Vec::new(),
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        }
        .into(),
        text: Some(text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

#[derive(Clone, Copy)]
enum ReminderBoundary {
    MayPay,
    MayPayOrPeriod,
}

fn keyword_cost_clause(
    tokens: &[OwnedLexToken],
    first_cost_token: usize,
    boundary: ReminderBoundary,
) -> &[OwnedLexToken] {
    let mut start = first_cost_token.min(tokens.len());
    if tokens
        .get(start)
        .is_some_and(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
    {
        start += 1;
    }
    let tail = tokens.get(start..).unwrap_or_default();
    let view = TokenWordView::new(tail);
    let words = view.word_refs();
    let reminder_word = permission_shapes::find_words(&words, &["you", "may", "pay"])
        .or_else(|| permission_shapes::find_words(&words, &["you", "may"]));
    let reminder_token = reminder_word
        .and_then(|word| view.token_start_indices().get(word).copied())
        .unwrap_or(tail.len());
    let period = match boundary {
        ReminderBoundary::MayPay => tail.len(),
        ReminderBoundary::MayPayOrPeriod => {
            first_kind_after(tail, 0, TokenKind::Period).unwrap_or(tail.len())
        }
    };
    trim_edge_commas(&tail[..reminder_token.min(period)])
}

fn morph_cost_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let tail = tokens.get(1..).unwrap_or_default();
    let view = TokenWordView::new(tail);
    let words = view.word_refs();
    let reminder_word = permission_shapes::find_words(&words, &["you", "may", "cast"])
        .or_else(|| permission_shapes::find_words(&words, &["turn", "it", "face", "up"]));
    let reminder = reminder_word
        .and_then(|word| view.token_start_indices().get(word).copied())
        .unwrap_or(tail.len());
    let period = first_kind_after(tail, 0, TokenKind::Period).unwrap_or(tail.len());
    trim_edge_commas(&tail[..reminder.min(period)])
}

fn ensure_mana_component(parsed: TotalCost, mana_cost: crate::mana::ManaCost) -> TotalCost {
    if parsed.mana_cost().is_some() {
        return parsed;
    }
    let mut components = parsed.costs().to_vec();
    components.insert(0, Cost::mana(mana_cost));
    TotalCost::from_costs(components)
}

fn parse_fixed_word(word: &str) -> Option<u32> {
    leaf::parse_leaf_number_prefix_words(&[word])?
        .into_fixed()
        .map(|(value, _)| value)
}

fn starts_with_keyword(tokens: &[OwnedLexToken], keyword: &str) -> bool {
    TokenWordView::new(tokens)
        .word_refs()
        .first()
        .is_some_and(|word| permission_shapes::exact_words(&[*word], &[keyword]))
}

fn first_kind_after(tokens: &[OwnedLexToken], start: usize, kind: TokenKind) -> Option<usize> {
    tokens
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, token)| (token.kind == kind).then_some(index))
}

fn trim_edge_commas(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[1..];
    }
    while tokens.last().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

fn unsupported_escape(words: &[&str]) -> CardTextError {
    CardTextError::ParseError(format!(
        "unsupported escape clause tail (clause: '{}')",
        words.join(" ")
    ))
}
