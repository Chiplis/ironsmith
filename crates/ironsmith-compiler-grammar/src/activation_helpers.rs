use crate::effect::Value;
use crate::host::{
    CardTextError, ConditionalEffectAst, EffectAst, OwnedLexToken, PlayerAst, SubjectAst, TagKey,
};
use crate::mana::ManaSymbol;
use crate::target::ObjectFilter;

use super::activation_and_restrictions::activated_line_core::parse_devotion_value_from_add_clause;
use super::effect_sentences::clause_pattern_helpers::extract_subject_player;
use super::grammar::activation_helpers as activation_grammar;
use super::grammar::structure::parse_trailing_instead_if_predicate_lexed;
use super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_add_mana_that_much_value,
    parse_dynamic_cost_modifier_value, parse_value_binding_clause_lexed,
    parse_where_x_is_number_of_filter_value,
};
use super::lexer::TokenWordView;
use super::object_filters::parse_object_filter;
pub use super::util::{
    find_activation_cost_start, join_sentences_with_period, non_article_word_refs,
    parse_subtype_flexible, parse_value, strip_leading_article_tokens,
    trim_edge_punctuation_tokens, value_contains_unbound_x,
};
pub use crate::grammar::shared_util::value_semantics::{
    parse_equal_to_aggregate_filter_value, parse_filter_comparison_tokens,
};
fn bind_revealed_this_way_count_to_last_object(value: Value) -> Value {
    match value {
        Value::Count(mut filter) => {
            for constraint in &mut filter.tagged_constraints {
                if constraint.tag.as_str()
                    == crate::tag::CompilerReferenceTag::PublicRevealed.as_str()
                {
                    constraint.tag = (crate::tag::CompilerReferenceTag::It.bind()).into();
                }
            }
            Value::Count(filter)
        }
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_revealed_this_way_count_to_last_object(*value)),
            hints,
        },
        other => other,
    }
}

fn first_non_comma_token_index(tokens: &[OwnedLexToken]) -> usize {
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_comma() {
            return idx;
        }
    }
    tokens.len()
}

fn parse_add_mana_amount(tokens: &[OwnedLexToken]) -> Option<Value> {
    let amount_tokens = match tokens {
        [article, additional, rest @ ..]
            if article.is_word("an") && additional.is_word("additional") =>
        {
            rest
        }
        [additional, rest @ ..] if additional.is_word("additional") => rest,
        _ => tokens,
    };
    parse_value(amount_tokens).map(|(value, _)| value)
}

/// "Add {U}{U}, {U}{R}, or {R}{R}" (filter lands) lists every combination of
/// two mana from its colors, which is exactly "two mana in any combination of
/// {U} and/or {R}". Any other list ("{B}{B} or {G}{G}", Cadaverous Bloom) is
/// a choice between the listed fixed outputs.
fn add_mana_alternatives_effect(player: PlayerAst, groups: Vec<Vec<ManaSymbol>>) -> EffectAst {
    fn symbol_color(symbol: ManaSymbol) -> Option<crate::color::Color> {
        match symbol {
            ManaSymbol::White => Some(crate::color::Color::White),
            ManaSymbol::Blue => Some(crate::color::Color::Blue),
            ManaSymbol::Black => Some(crate::color::Color::Black),
            ManaSymbol::Red => Some(crate::color::Color::Red),
            ManaSymbol::Green => Some(crate::color::Color::Green),
            _ => None,
        }
    }
    let size = groups[0].len();
    let colors = {
        let mut colors: Vec<crate::color::Color> = Vec::new();
        for symbol in groups.iter().flatten() {
            if let Some(color) = symbol_color(*symbol)
                && !colors.contains(&color)
            {
                colors.push(color);
            }
        }
        colors
    };
    let all_colored = groups
        .iter()
        .flatten()
        .all(|symbol| symbol_color(*symbol).is_some());
    if all_colored && groups.iter().all(|group| group.len() == size) {
        let normalize = |group: &Vec<ManaSymbol>| {
            let mut counts = colors
                .iter()
                .map(|color| {
                    group
                        .iter()
                        .filter(|symbol| {
                            symbol_color(**symbol) == Some(*color)
                        })
                        .count()
                })
                .collect::<Vec<_>>();
            counts.shrink_to_fit();
            counts
        };
        let mut listed = groups.iter().map(normalize).collect::<Vec<_>>();
        listed.sort();
        listed.dedup();
        // Number of multisets of `size` mana over the colors.
        let expected = (1..=size).fold(1usize, |acc, k| acc * (colors.len() + k - 1) / k);
        if listed.len() == expected {
            return EffectAst::subject_verb_add_mana_any_color(
                player,
                Value::Fixed(size as i32),
                Some(colors),
            );
        }
    }
    EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseOneOf {
        chooser: crate::target::PlayerFilter::You,
        modes: groups
            .into_iter()
            .map(|group| crate::cards::builders::ChooseOneModeAst {
                description: format!(
                    "Add {}",
                    crate::mana::ManaCost::from_symbols(group.clone()).to_oracle()
                ),
                effects: vec![EffectAst::subject_verb_add_mana(player, group)],
            })
            .collect(),
    })
}

/// "for each {S} spent to cast this spell" (Tundra Fumarole): {S} is mana
/// from a snow source (CR 107.4h), so the count is the mana spent to cast the
/// spell whose producing source was snow.
fn parse_for_each_snow_mana_spent_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = match tokens.last() {
        Some(last) if last.kind == crate::lexer::TokenKind::Period => &tokens[..tokens.len() - 1],
        _ => tokens,
    };
    let [for_word, each, snow, rest @ ..] = tokens else {
        return None;
    };
    if !for_word.is_word("for") || !each.is_word("each") {
        return None;
    }
    let pip = crate::grammar::leaf::parse_leaf_surface_mana_pip_token(snow)?;
    if pip.into_pip().as_slice() != [ManaSymbol::Snow] {
        return None;
    }
    let words = crate::lexer::token_word_refs(rest);
    let reference = match words.as_slice() {
        ["spent", "to", "cast", "this", "spell"] => ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell,
        ["spent", "to", "cast", "it"] => ironsmith_core::ManaSpentCastReferenceSurface::It,
        _ => return None,
    };
    if rest.iter().any(|token| token.as_word().is_none()) {
        return None;
    }
    let mut source_filter = ObjectFilter::default();
    source_filter.supertypes.push(crate::types::Supertype::Snow);
    Some(Value::ManaFromSourceSpentToCastThisSpell {
        source_filter,
        include_source_noun: false,
        reference,
    })
}

/// "chosen as you drafted cards named <name>" after "mana of any color".
fn parse_drafted_chosen_color_mana_tail(tail_tokens: &[OwnedLexToken]) -> Option<String> {
    const PREFIX: &[&str] = &["chosen", "as", "you", "drafted", "cards", "named"];
    let view = TokenWordView::new(tail_tokens);
    let words = view.to_word_refs();
    if words.len() <= PREFIX.len() || !words.starts_with(PREFIX) {
        return None;
    }
    let name_start = view.map_word_or_end_to_token_boundary(PREFIX.len())?;
    let name = super::lexer::render_literal_token_slice(tail_tokens.get(name_start..)?)
        .trim()
        .trim_end_matches('.')
        .trim()
        .to_string();
    (!name.is_empty()).then_some(name)
}

pub fn parse_add_mana(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let clause_word_view = TokenWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let facts = activation_grammar::parse_add_mana_clause_facts(tokens);
    let wrap_instead_if_tail = |base_effect: EffectAst,
                                tail_tokens: &[OwnedLexToken]|
     -> Result<Option<EffectAst>, CardTextError> {
        if !activation_grammar::is_instead_if_tail(tail_tokens) {
            return Ok(None);
        }
        let predicate =
            parse_trailing_instead_if_predicate_lexed(tail_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported trailing mana clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        Ok(Some(EffectAst::Conditionals(
            ConditionalEffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            },
        )))
    };

    if let Some(any_color_among) = parse_add_one_mana_any_color_among_filter(tokens)? {
        return Ok(EffectAst::subject_verb_add_one_mana_any_color_among(
            player,
            any_color_among,
        ));
    }
    if let Some(colors_among) = parse_add_mana_colors_among_filter(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_colors_among(
            player,
            colors_among,
        ));
    }
    if facts.imprinted_colors {
        return Ok(EffectAst::subject_verb_add_mana_imprinted_colors());
    }
    // "you add the mana lost this way" (Drain Power): the types of the mana
    // the preceding instruction made a player lose.
    if matches!(
        clause_words.as_slice(),
        ["the", "mana", "lost", "this", "way"] | ["all", "mana", "lost", "this", "way"]
    ) {
        return Ok(EffectAst::subject_verb_add_mana_lost_this_way(
            player,
            Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::Outcome,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_action(ironsmith_core::PriorEffectAction::ManaLost),
            ),
        ));
    }
    if let Some(filter) = parse_add_mana_equal_to_mana_cost_filter(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_equal_to_mana_cost(
            player, filter,
        ));
    }
    // "Add one mana of either of the circled colors." (Cryptic Spires): the
    // colors circled for this card as its owner built their deck.
    if matches!(
        clause_words.as_slice(),
        [.., "mana", "of", "either" | "any", "of", "the", "circled", "colors"]
    ) {
        let amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_deck_construction_chosen_colors(
            player, amount,
        ));
    }
    // "Add one mana of this artifact's last noted type."
    if clause_words.ends_with(&["last", "noted", "type"]) && clause_words.contains(&"mana") {
        let amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_noted_type(player, amount));
    }

    if facts.commander_identity {
        let amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_commander_identity(
            player, amount,
        ));
    }

    if facts.different_colors {
        let amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_any_color_with_distinct(
            player, amount, None, true,
        ));
    }

    // "Add three mana in any combination of its colors" (Omnath, Locus of
    // All): the colors are the referenced object's, read on resolution.
    if matches!(
        clause_words.as_slice(),
        [.., "any", "combination", "of", "its" | "that", "colors"]
            | [.., "any", "combination", "of", "that", "card's" | "cards" | "card’s", "colors"]
    ) {
        let amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_any_color_among(
            player,
            amount,
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
        ));
    }
    if let Some(available_colors) = parse_any_combination_mana_colors(tokens)? {
        let mut amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        if matches!(amount, Value::X)
            && let Some(tail) = activation_grammar::parse_any_combination_mana_tail(tokens)
            && let Some(dynamic_amount) = parse_value_binding_clause_lexed(tail)
        {
            amount = dynamic_amount;
        }
        return Ok(EffectAst::subject_verb_add_mana_any_color(
            player,
            amount,
            Some(available_colors),
        ));
    }

    let fixed_output = activation_grammar::parse_fixed_mana_output(tokens);
    if let Some(for_each_idx) = fixed_output.first_for_each_token
        && let Some(available_colors) = parse_or_mana_color_choices(&tokens[..for_each_idx])?
    {
        let amount =
            parse_dynamic_cost_modifier_value(&tokens[for_each_idx..])?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported dynamic color-choice mana amount (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        return Ok(EffectAst::subject_verb_add_mana_any_color(
            player,
            bind_revealed_this_way_count_to_last_object(amount),
            Some(available_colors),
        ));
    }

    if let Some(groups) = activation_grammar::parse_or_mana_symbol_groups(tokens) {
        return Ok(add_mana_alternatives_effect(player, groups));
    }
    if let Some(available_colors) = parse_or_mana_color_choices(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_any_color(
            player,
            Value::Fixed(1),
            Some(available_colors),
        ));
    }

    // "adds three mana of the chosen color for each artifact sacrificed this
    // way" (Radiant Lotus): a chosen-color amount scaled by a for-each count.
    if !fixed_output.has_explicit_symbol
        && let Some(for_each_idx) = tokens
            .windows(2)
            .position(|pair| pair[0].is_word("for") && pair[1].is_word("each"))
        && crate::lexer::token_word_refs(&tokens[..for_each_idx])
            .ends_with(&["mana", "of", "the", "chosen", "color"])
        && let Some(per) = parse_dynamic_cost_modifier_value(&tokens[for_each_idx..])?
    {
        let per = bind_revealed_this_way_count_to_last_object(per);
        let amount = match parse_add_mana_amount(&tokens[..for_each_idx]) {
            None | Some(Value::Fixed(1)) => per,
            Some(Value::Fixed(multiplier)) => Value::Scaled(Box::new(per), multiplier),
            Some(_) => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported dynamic chosen-color mana multiplier (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        };
        return Ok(EffectAst::subject_verb_add_mana_chosen_color(
            player, amount, None,
        ));
    }
    if !fixed_output.has_explicit_symbol && facts.chosen_color_reference {
        let amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_chosen_color(
            player, amount, None,
        ));
    }
    if let Some(tail_tokens) = facts.one_that_color_tail {
        if tail_tokens.is_empty() || is_mana_pool_tail_tokens(tail_tokens) {
            return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                player,
                Value::Fixed(1),
                None,
            ));
        }
        if let Some(amount) = parse_dynamic_cost_modifier_value(tail_tokens)? {
            let amount = bind_revealed_this_way_count_to_last_object(amount);
            return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                player, amount, None,
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic chosen-color mana amount (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if facts.amount_that_color {
        let amount = parse_devotion_value_from_add_clause(tokens)?
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_chosen_color(
            player, amount, None,
        ));
    }

    if facts.amount_that_type {
        // "add three mana of that type instead" (Incubation Druid) restates
        // the default's type choice for the replacement amount.
        let amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_from_land_could_produce(
            player,
            amount,
            ObjectFilter::land().controlled_by(crate::target::PlayerFilter::You),
            true,
            true,
            crate::effects::ManaTypeSource::MatchingLandsCouldProduce,
        ));
    }

    if let Some(mana_choice) = facts.choice {
        let mut amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        let any_one = mana_choice.kind.any_one();
        let any_type = mana_choice.kind.allow_colorless();
        let tail_tokens = mana_choice.tail_tokens;

        if tail_tokens.is_empty() || is_mana_pool_tail_tokens(tail_tokens) {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        // "one mana of any color chosen as you drafted cards named <name>"
        // (Paliano, the High City): the colors come from the draft record.
        if !any_type
            && !any_one
            && let Some(card_name) = parse_drafted_chosen_color_mana_tail(tail_tokens)
        {
            return Ok(EffectAst::subject_verb_add_mana_draft_chosen_colors(
                player, amount, card_name,
            ));
        }

        // "add an additional one mana of any type that land produced for each
        // other land you control with the same name as it" (Sasaya's
        // Essence): the produced-type mana is repeated once per matching
        // permanent. "Other" excludes the land that was tapped, which "it"
        // names, rather than this ability's source.
        if matches!(amount, Value::Fixed(1))
            && let Some(for_idx) = tail_tokens.windows(2).position(|pair| {
                pair[0].is_word("for") && pair[1].is_word("each")
            })
            && for_idx > 0
            && for_idx + 2 < tail_tokens.len()
            && let Some((filter, mana_type_source)) =
                parse_land_could_produce_filter(&tail_tokens[..for_idx])?
        {
            let per_tokens = &tail_tokens[for_idx + 2..];
            let per_words = TokenWordView::new(per_tokens).to_word_refs();
            let triggering_relative_other = per_words.first() == Some(&"other")
                && per_words.ends_with(&["as", "it"])
                && mana_type_source == crate::effects::ManaTypeSource::TriggeringEventProduced;
            let mut per_filter = parse_object_filter(per_tokens, false)?;
            if triggering_relative_other {
                per_filter.other = false;
                per_filter = per_filter.not_tagged(crate::tag::CompilerReferenceTag::Triggering.key());
            }
            return Ok(EffectAst::subject_verb_add_mana_from_land_could_produce(
                player,
                Value::Count(per_filter),
                filter,
                any_type,
                any_one,
                mana_type_source,
            ));
        }
        if let Some((filter, mana_type_source)) = parse_land_could_produce_filter(tail_tokens)? {
            return Ok(EffectAst::subject_verb_add_mana_from_land_could_produce(
                player,
                amount,
                filter,
                any_type,
                any_one,
                mana_type_source,
            ));
        }

        if matches!(amount, Value::X)
            && let Some(dynamic_amount) = parse_where_x_is_number_of_filter_value(tail_tokens)
        {
            amount = dynamic_amount;
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        if activation_grammar::is_player_choice_tail(tail_tokens) {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }
        if activation_grammar::is_removed_this_way_tail(tail_tokens)
            && let Some(dynamic_amount) = parse_dynamic_cost_modifier_value(tail_tokens)?
        {
            amount = bind_revealed_this_way_count_to_last_object(dynamic_amount);
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        if activation_grammar::is_among_tail(tail_tokens) {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        // "Add two mana of any one color and two mana of any other color"
        // (Open the Omenpaths): a choice of two different colors, each
        // repeated the same fixed number of times.
        if any_one
            && !any_type
            && let Value::Fixed(per_color) = amount.clone()
            && per_color > 0
            && let Some(second_amount) = parse_any_other_color_mana_tail(tail_tokens)
            && second_amount == per_color
        {
            const COLORS: [ManaSymbol; 5] = [
                ManaSymbol::White,
                ManaSymbol::Blue,
                ManaSymbol::Black,
                ManaSymbol::Red,
                ManaSymbol::Green,
            ];
            let mut groups = Vec::new();
            for (first_index, first) in COLORS.iter().enumerate() {
                for second in &COLORS[first_index + 1..] {
                    let mut group = vec![*first; per_color as usize];
                    group.extend(std::iter::repeat_n(*second, per_color as usize));
                    groups.push(group);
                }
            }
            return Ok(add_mana_alternatives_effect(player, groups));
        }

        let base_effect = if any_one {
            EffectAst::subject_verb_add_mana_any_one_color(player, amount)
        } else {
            EffectAst::subject_verb_add_mana_any_color(player, amount, None)
        };
        if let Some(conditional) = wrap_instead_if_tail(base_effect, tail_tokens)? {
            return Ok(conditional);
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mana = fixed_output.mana;
    let last_mana_idx = fixed_output.last_mana_token;
    let for_each_idx = fixed_output.first_for_each_token;

    if !mana.is_empty() {
        if let Some(amount) = parse_add_mana_that_much_value(tokens) {
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(amount) = parse_devotion_value_from_add_clause(tokens)? {
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(for_each_idx) = for_each_idx {
            let amount_tokens = &tokens[for_each_idx..];
            if let Some(amount) = parse_for_each_snow_mana_spent_value(amount_tokens) {
                return Ok(EffectAst::subject_verb_add_mana_scaled(
                    player, mana, amount,
                ));
            }
            let amount = parse_dynamic_cost_modifier_value(amount_tokens)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported dynamic mana amount (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            let amount = bind_revealed_this_way_count_to_last_object(amount);
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(amount) = parse_equal_to_aggregate_filter_value(tokens)
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
        {
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        let trailing_tokens = last_mana_idx
            .map(|last_idx| &tokens[last_idx + 1..])
            .unwrap_or_default();
        let tail_kind = activation_grammar::classify_fixed_mana_tail(trailing_tokens);
        if !trailing_tokens.is_empty()
            && tail_kind == activation_grammar::FixedManaTailKind::ChosenColor
        {
            if mana.len() != 1 {
                return Err(CardTextError::ParseError(format!(
                    "unsupported chosen-color mana clause with multiple symbols (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let Some(color) = mana_symbol_to_color(mana[0]) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported chosen-color mana clause with non-colored symbol (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                player,
                Value::Fixed(1),
                Some(color),
            ));
        }
        let supported_tail = matches!(
            tail_kind,
            activation_grammar::FixedManaTailKind::Pool
                | activation_grammar::FixedManaTailKind::Instead
        );
        if !trailing_tokens.is_empty() && !supported_tail {
            if let Some(last_idx) = last_mana_idx
                && let Some(conditional) = wrap_instead_if_tail(
                    EffectAst::subject_verb_add_mana(player, mana.clone()),
                    trim_leading_commas(&tokens[last_idx + 1..]),
                )?
            {
                return Ok(conditional);
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if mana.len() == 1
            && let Some(Value::Fixed(amount)) = parse_add_mana_amount(tokens)
            && amount > 1
        {
            return Ok(EffectAst::subject_verb_add_mana(
                player,
                std::iter::repeat_n(mana[0], amount as usize).collect(),
            ));
        }
        return Ok(EffectAst::subject_verb_add_mana(player, mana));
    }

    Err(CardTextError::ParseError(format!(
        "missing mana symbols (clause: '{}')",
        clause_words.join(" ")
    )))
}

/// "add mana equal to enchanted permanent's mana cost" (Elemental
/// Resonance): the object whose printed mana symbols are produced.
fn parse_add_mana_equal_to_mana_cost_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let word_tokens: Vec<OwnedLexToken> = tokens
        .iter()
        .filter(|token| token.as_word().is_some())
        .cloned()
        .collect();
    let words = crate::lexer::token_word_refs(&word_tokens);
    let Some(equal) = words
        .windows(3)
        .position(|window| window == ["mana", "equal", "to"])
    else {
        return Ok(None);
    };
    if equal != 0 || words.len() < 6 || !words.ends_with(&["mana", "cost"]) {
        return Ok(None);
    }
    let mut object = word_tokens[3..word_tokens.len() - 2].to_vec();
    let Some(last) = object.last_mut() else {
        return Ok(None);
    };
    let text = last.as_word().unwrap_or_default().to_string();
    let stripped = text
        .strip_suffix("'s")
        .or_else(|| text.strip_suffix("’s"))
        .or_else(|| text.strip_suffix('s'))
        .map(str::to_string);
    let Some(stripped) = stripped else {
        return Ok(None);
    };
    if stripped.is_empty() {
        return Ok(None);
    }
    *last = OwnedLexToken::word(stripped, last.span());
    let mut filter = parse_object_filter(&object, false)?;
    if filter.zone.is_none() {
        filter.zone = Some(crate::zone::Zone::Battlefield);
    }
    Ok(Some(filter))
}

/// "and two mana of any other color" after "<N> mana of any one color":
/// returns the second fixed amount.
fn parse_any_other_color_mana_tail(tokens: &[OwnedLexToken]) -> Option<i32> {
    let tokens = match tokens.last() {
        Some(last) if last.kind == crate::lexer::TokenKind::Period => &tokens[..tokens.len() - 1],
        _ => tokens,
    };
    let [and, amount_and_rest @ ..] = tokens else {
        return None;
    };
    if !and.is_word("and") {
        return None;
    }
    let (amount, used) = parse_value(amount_and_rest)?;
    let Value::Fixed(amount) = amount else {
        return None;
    };
    let rest = amount_and_rest.get(used..)?;
    if rest.iter().any(|token| token.as_word().is_none()) {
        return None;
    }
    (crate::lexer::token_word_refs(rest).as_slice() == ["mana", "of", "any", "other", "color"])
        .then_some(amount)
}

fn parse_add_mana_colors_among_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(span) = activation_grammar::parse_colors_among_span(tokens) else {
        return Ok(None);
    };
    let filter_tokens = span.filter_tokens;
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(Some(filter))
}

#[path = "activation_helpers/reference.rs"]
mod reference_programs;
use reference_programs::parse_add_one_mana_any_color_among_filter;
pub use reference_programs::parse_land_could_produce_filter;
#[path = "activation_helpers/resource.rs"]
mod resource_programs;
pub use resource_programs::{
    is_mana_pool_tail_tokens, mana_symbol_to_color, parse_any_combination_mana_colors,
};
#[path = "activation_helpers/core.rs"]
mod core_programs;
pub use core_programs::trim_leading_commas;
#[path = "activation_helpers/choice.rs"]
mod choice_programs;
pub use choice_programs::parse_or_mana_color_choices;
