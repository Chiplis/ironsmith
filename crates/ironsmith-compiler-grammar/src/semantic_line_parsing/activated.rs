use super::super::effect_ast_traversal::for_each_nested_effects_mut;
use super::super::grammar::activated_lowering as activated_grammar;
use super::super::grammar::activated_lowering::{
    ActivatedManaEffectKind, ActivatedRestrictionSentenceKind, ActivatedXDefinitionIntro,
};
use super::super::grammar::effects::parse_fixed_mana_output_clause_spec_lexed;
use super::super::grammar::restriction_facts::{
    parse_activation_restriction_surface_tokens, parse_mana_restriction_surface_tokens,
    parse_mana_restriction_tokens,
};
use super::super::ir::RewriteActivatedLine;
use super::*;
use crate::effect::{Effect, Value};
use crate::model::compiler_semantic::ParsedManaRestriction;
use crate::object::CounterType;
use crate::util::compiler_activation_cost_reference_imports;
use ironsmith_core::TotalCostKind;

fn activated_effect_may_be_mana_ability_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_grammar::parse_activated_mana_effect_kind(tokens).is_some()
        || is_choose_color_of_matching_object_mana_shape(tokens)
}

fn choose_color_of_matching_object_sentences(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    let [choose_sentence, add_sentence] = sentences.as_slice() else {
        return None;
    };
    let choose_words = crate::lexer::parser_token_word_refs(choose_sentence);
    let add_words = crate::lexer::parser_token_word_refs(add_sentence);
    if !crate::word_primitives::parse_sequence_prefix(
        &choose_words,
        &["choose", "a", "color", "of"],
    ) || choose_words.len() <= 4
        || !crate::word_primitives::parse_sequence_complete(
            &add_words,
            &["add", "one", "mana", "of", "that", "color"],
        )
    {
        return None;
    }
    let filter_tokens = choose_sentence
        .iter()
        .filter(|token| token.as_word().is_some())
        .skip(4)
        .cloned()
        .collect::<Vec<_>>();
    Some((filter_tokens, add_sentence.to_vec()))
}

fn is_choose_color_of_matching_object_mana_shape(tokens: &[OwnedLexToken]) -> bool {
    choose_color_of_matching_object_sentences(tokens).is_some()
}

fn parse_choose_color_of_matching_object_mana_effect(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some((filter_tokens, _)) = choose_color_of_matching_object_sentences(tokens) else {
        return Ok(None);
    };
    let mut filter = crate::object_filters::parse_object_filter(&filter_tokens, false)?;
    // The generic object-filter grammar expands the bare type word
    // "permanent" into every permanent card type. This exact sentence family
    // is selecting a color of an object already constrained to the
    // battlefield, so retain the canonical all-permanent domain instead of a
    // presentation-sensitive six-type expansion.
    if filter.zone == Some(Zone::Battlefield) && filter.has_all_permanent_card_types() {
        filter.card_types.clear();
    }
    Ok(Some(
        EffectAst::subject_verb_choose_color_of_object_add_mana(PlayerAst::You, filter),
    ))
}

fn activated_effect_is_for_each_color_among_add_mana_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_grammar::parse_activated_mana_effect_kind(tokens)
        == Some(ActivatedManaEffectKind::ColorsAmong)
}

fn activation_cost_defines_x_for_mana_ability(
    cost: &ironsmith_core::TotalCost<crate::model::CompilerCost>,
) -> bool {
    if cost.mana_cost().is_some_and(crate::mana::ManaCost::has_x) {
        return true;
    }

    fn value_uses_x(value: &crate::effect::Value) -> bool {
        use crate::effect::Value;

        match value {
            Value::X | Value::XTimes(_) => true,
            Value::Scaled(inner, _)
            | Value::DividedRoundedDown(inner, _)
            | Value::HalfRoundedDown(inner) => value_uses_x(inner),
            Value::Add(left, right) => value_uses_x(left) || value_uses_x(right),
            _ => false,
        }
    }

    cost.costs().iter().any(|component| match component {
        crate::model::CompilerCost::Mana(cost)
        | crate::model::CompilerCost::DynamicMana(ironsmith_core::DynamicManaCost {
            base: cost,
            ..
        }) => cost.has_x(),
        crate::model::CompilerCost::Life(amount) => value_uses_x(amount),
        crate::model::CompilerCost::Sacrifice { count, .. }
        | crate::model::CompilerCost::ExileChosen { count, .. } => count.dynamic_x,
        crate::model::CompilerCost::RemoveCounters { dynamic, .. } => *dynamic,
        _ => false,
    })
}

fn activation_cost_sets_x_from_counter_removal(
    cost: &ironsmith_core::TotalCost<crate::model::CompilerCost>,
) -> bool {
    fn component_sets_x(component: &crate::model::CompilerCost) -> bool {
        matches!(component, crate::model::CompilerCost::RemoveCounters { .. })
    }

    match cost.kind() {
        TotalCostKind::All(components) => components.iter().any(component_sets_x),
        TotalCostKind::OneOf(branches) => branches
            .iter()
            .any(activation_cost_sets_x_from_counter_removal),
    }
}

fn bind_event_amount_to_cost_x(value: &mut crate::effect::Value) {
    use crate::effect::{EventValueSpec, Value};

    match value {
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) => {
            *value = Value::X;
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) => {
            *value = Value::Add(Box::new(Value::X), Box::new(Value::Fixed(*offset)));
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            bind_event_amount_to_cost_x(left);
            bind_event_amount_to_cost_x(right);
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner) => {
            bind_event_amount_to_cost_x(inner);
        }
        Value::SurfaceHinted { value, .. } => bind_event_amount_to_cost_x(value),
        _ => {}
    }
}

fn bind_event_amounts_to_cost_x_in_effect(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::PumpByLastEffect {
            power,
            toughness,
            target,
            duration,
            includes_this_way,
        } = &subject_verb.action
    {
        let basis = crate::effect::Value::X.with_surface_hint(if *includes_this_way {
            ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay
        } else {
            ironsmith_core::ValueSurfaceHint::CountersRemoved
        });
        let scale = |multiplier: i32| match multiplier {
            0 => crate::effect::Value::Fixed(0),
            1 => basis.clone(),
            _ => crate::effect::Value::Scaled(Box::new(basis.clone()), multiplier),
        };
        *effect = EffectAst::subject_verb_pump(
            scale(*power),
            scale(*toughness),
            target.clone(),
            duration.clone(),
            None,
        );
        return;
    }

    if let EffectAst::SubjectVerb(subject_verb) = effect {
        match &mut subject_verb.action {
            SubjectVerbActionAst::DealDamage { amount, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { amount, .. }
            | SubjectVerbActionAst::DealDistributedDamage { amount, .. }
            | SubjectVerbActionAst::DealDamageEach { amount, .. }
            | SubjectVerbActionAst::Mill { count: amount }
            | SubjectVerbActionAst::Draw { count: amount }
            | SubjectVerbActionAst::AddManaScaled { amount, .. }
            | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
            | SubjectVerbActionAst::AddManaAnyOneColor { amount }
            | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
            | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount, .. }
            | SubjectVerbActionAst::AddManaCommanderIdentity { amount } => {
                bind_event_amount_to_cost_x(amount);
            }
            _ => {}
        }
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for inner in nested {
            bind_event_amounts_to_cost_x_in_effect(inner);
        }
    });
}

fn bind_event_amounts_to_cost_x(effects: &mut [EffectAst]) {
    for effect in effects {
        bind_event_amounts_to_cost_x_in_effect(effect);
    }
}

fn effect_ast_is_mana_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => matches!(
            &subject_verb.action,
            SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaColorsAmong { .. }
                | SubjectVerbActionAst::AddOneManaAnyColorAmong { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::AddManaImprintedColors
        ),
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ManaRestricted { effects, .. } => {
            !effects.is_empty() && effects.iter().all(effect_ast_is_mana_effect)
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            (!if_true.is_empty() && if_true.iter().all(effect_ast_is_mana_effect))
                || (!if_false.is_empty() && if_false.iter().all(effect_ast_is_mana_effect))
        }
        _ => false,
    }
}

fn effect_ast_starts_with_mana_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ManaRestricted { effects, .. } => effects
            .first()
            .is_some_and(effect_ast_starts_with_mana_effect),
        other => effect_ast_is_mana_effect(other),
    }
}

fn effects_ast_can_lower_as_mana_ability(effects: &[EffectAst]) -> bool {
    !effects.is_empty() && effects.iter().all(effect_ast_is_mana_effect)
}

struct SplitRewriteActivatedEffectText {
    effect_text: String,
    effect_parse_tokens: Vec<OwnedLexToken>,
    restrictions: ParsedRestrictions,
    mana_restrictions: Vec<ParsedManaRestriction>,
    x_cant_be_zero: bool,
}

fn parse_standalone_x_definition_value(tokens: &[OwnedLexToken]) -> Option<crate::effect::Value> {
    let shape = activated_grammar::parse_activated_x_definition_tokens(tokens)?;
    let value_tokens = match shape.intro {
        ActivatedXDefinitionIntro::WhereXIs => tokens.to_vec(),
        ActivatedXDefinitionIntro::XIs => {
            let mut synthetic = vec![
                OwnedLexToken::word("where".to_string(), TextSpan::synthetic()),
                OwnedLexToken::word("x".to_string(), TextSpan::synthetic()),
                OwnedLexToken::word("is".to_string(), TextSpan::synthetic()),
            ];
            synthetic.extend_from_slice(shape.value_tokens);
            synthetic
        }
    };

    if shape.exiled_card_mana_value {
        // A standalone "X is ... that card's mana value" clause on an
        // activated ability refers to the source-linked exiled card (the
        // persistent imprint), not the transient `__it__` binding from a
        // different resolution context. Check this typed shape before the
        // generic value parser can erase that distinction.
        return Some(crate::effect::Value::ManaValueOf(Box::new(
            ChooseSpec::Tagged(TagKey::from(crate::tag::SOURCE_EXILED_TAG)),
        )));
    }

    parse_value_binding_clause(&value_tokens)
}

fn is_standalone_x_definition_sentence(tokens: &[OwnedLexToken]) -> bool {
    parse_standalone_x_definition_value(tokens).is_some()
}

fn activated_x_definition_value(tokens: &[OwnedLexToken]) -> Option<crate::effect::Value> {
    split_lexed_sentences(tokens)
        .into_iter()
        .find_map(parse_standalone_x_definition_value)
        .or_else(|| {
            let shape = activated_grammar::find_activated_x_definition_tokens(tokens)?;
            let offset = tokens.len().checked_sub(shape.value_tokens.len() + 3)?;
            parse_standalone_x_definition_value(&tokens[offset..])
        })
}

fn bind_activated_x_definition_to_mana_cost(
    cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    x_value: Option<crate::effect::Value>,
) -> ironsmith_core::TotalCost<crate::model::CompilerCost> {
    let Some(x_value) = x_value else {
        return cost;
    };

    cost.try_map(|component| {
        if let Some(mana_cost) = component.mana_cost_ref()
            && mana_cost.has_x()
        {
            Ok(crate::model::CompilerCost::DynamicMana(
                ironsmith_core::DynamicManaCost::from_x(mana_cost.clone(), x_value.clone()),
            ))
        } else {
            Ok(component)
        }
    })
    .unwrap_or_else(|_: std::convert::Infallible| unreachable!())
}

fn finalize_rewrite_activated_effect_sentences(
    mut restrictions: ParsedRestrictions,
    sentence_tokens: Vec<Vec<OwnedLexToken>>,
) -> SplitRewriteActivatedEffectText {
    let mut effect_sentences = Vec::new();
    let mut effect_sentence_tokens = Vec::new();
    let mut mana_restrictions = Vec::new();
    let mut x_cant_be_zero = false;

    for tokens in sentence_tokens {
        let sentence = render_token_slice(&tokens).trim().to_string();
        let restriction_kind = activated_grammar::classify_activated_restriction_sentence(&tokens);
        if restriction_kind == Some(ActivatedRestrictionSentenceKind::ManaSource) {
            restrictions
                .activation
                .push(parse_activation_restriction_surface_tokens(&tokens));
        } else if let Some(parsed) = parse_mana_restriction_tokens(&tokens) {
            mana_restrictions.push(parsed);
        } else if matches!(
            restriction_kind,
            Some(ActivatedRestrictionSentenceKind::SpendThisManaOnly)
                | Some(ActivatedRestrictionSentenceKind::WhenSpendThisManaToCast)
        ) {
            mana_restrictions.push(parse_mana_restriction_surface_tokens(&tokens));
        } else if super::super::grammar::effects::dispatch_entry_shapes::is_x_cant_be_zero_tokens(
            &tokens,
        ) {
            x_cant_be_zero = true;
        } else if is_standalone_x_definition_sentence(&tokens) {
            continue;
        } else if is_any_player_may_activate_sentence_lexed(&tokens) {
            restrictions
                .activation
                .push(parse_activation_restriction_surface_tokens(&tokens));
        } else {
            effect_sentences.push(sentence);
            effect_sentence_tokens.push(tokens);
        }
    }

    SplitRewriteActivatedEffectText {
        effect_text: effect_sentences.join(". "),
        effect_parse_tokens: join_sentences_with_period(&effect_sentence_tokens),
        restrictions,
        mana_restrictions,
        x_cant_be_zero,
    }
}

fn split_rewrite_activated_effect_text(
    effect_parse_tokens: &[OwnedLexToken],
) -> SplitRewriteActivatedEffectText {
    let (sentence_tokens, restrictions) = split_tokens_for_parse(effect_parse_tokens);
    finalize_rewrite_activated_effect_sentences(restrictions, sentence_tokens)
}

/// Recognize an authored trailing sorcery-speed sentence only when it is
/// outside quoted granted/token rules text.
///
/// Full-card preprocessing intentionally keeps periods inside quoted ability
/// text from splitting the outer ability. For a quoted effect followed by an
/// ordinary activation restriction, that can leave the closing quote and the
/// trailing sentence in one token group. The raw source line still preserves
/// the exact quote boundary, so use it solely to recover this already-typed
/// timing fact. A restriction that belongs inside the quote ends with the
/// quote and therefore cannot satisfy this predicate.
fn authored_trailing_sorcery_speed_restriction(raw_line: &str) -> bool {
    const SUFFIX: &str = "Activate only as a sorcery.";
    let trimmed = raw_line.trim();
    let Some(prefix) = trimmed.strip_suffix(SUFFIX) else {
        return false;
    };

    let mut in_quote = false;
    let mut escaped = false;
    for character in prefix.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
        } else if character == '"' {
            in_quote = !in_quote;
        }
    }
    !in_quote
}

/// Keep a hidden looked-card partition and its linked exile permission
/// together while lowering an activated ability.  The ordinary effect-body
/// dispatcher supports this shape, but activated parsing has a sentence-wise
/// fallback whose second sentence begins with the nontargeted instruction
/// "Exile one".  Once split, that sentence can be claimed by the broad exile
/// target parser and the selected-card tag is no longer available to the
/// permission.  Give the already-typed three-sentence grammar first refusal
/// at the activated boundary.
fn parse_hidden_look_partition_activated(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .map(crate::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() != 3 {
        return Ok(None);
    }

    crate::effect_sentences::parse_look_at_top_partition_face_down_then_filtered_permission(
        &sentences, 0,
    )
}

/// Reparse a grammar-proven leading-duration compound whose subject is an
/// authored source name. The generic gain parser deliberately understands
/// typed self references, while full-card preprocessing retains the authored
/// name for presentation. Parse through a typed self subject, then restore
/// that exact source-reference surface on the two coordinated consumers.
fn parse_named_source_leading_gain_activated(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = token_word_refs(tokens);
    const UNTIL_END_OF_TURN: [&str; 4] = ["until", "end", "of", "turn"];
    if words.len() < UNTIL_END_OF_TURN.len()
        || !words
            .iter()
            .zip(UNTIL_END_OF_TURN)
            .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
    {
        return Ok(None);
    }
    let Some(get_token_index) = crate::slice_primitives::select_position(tokens, |token| {
        token.as_word().is_some_and(|word| {
            word.eq_ignore_ascii_case("get") || word.eq_ignore_ascii_case("gets")
        })
    }) else {
        return Ok(None);
    };
    let Some(get_word_index) = crate::slice_primitives::select_position(&words, |word| {
        word.eq_ignore_ascii_case("get") || word.eq_ignore_ascii_case("gets")
    }) else {
        return Ok(None);
    };
    if crate::slice_primitives::select_position(&words[get_word_index + 1..], |word| {
        word.eq_ignore_ascii_case("gain") || word.eq_ignore_ascii_case("gains")
    })
    .is_none()
    {
        return Ok(None);
    }
    let subject_start =
        crate::slice_primitives::select_last_position(&tokens[..get_token_index], |token| {
            token.kind == TokenKind::Comma
        })
        .map_or(0, |index| index + 1);
    let subject_tokens = trim_lexed_commas(&tokens[subject_start..get_token_index]);
    let subject_words = token_word_refs(subject_tokens);
    let Some(surface) = crate::util::source_reference_surface_for_words(&subject_words)
        .or_else(|| crate::util::this_source_surface_for_words(&subject_words))
        .or_else(|| {
            subject_words
                .first()
                .and_then(|word| word.chars().next())
                .is_some_and(char::is_uppercase)
                .then(|| {
                    crate::target::SourceReferenceSurface::ShortName(
                        render_token_slice(subject_tokens).trim().to_string(),
                    )
                })
        })
    else {
        return Ok(None);
    };

    let Some(gain_token_index) =
        crate::slice_primitives::select_position(&tokens[get_token_index + 1..], |token| {
            token.as_word().is_some_and(|word| {
                word.eq_ignore_ascii_case("gain") || word.eq_ignore_ascii_case("gains")
            })
        })
        .map(|index| get_token_index + 1 + index)
    else {
        return Ok(None);
    };
    let Some(and_token_index) = crate::slice_primitives::select_last_position(
        &tokens[get_token_index + 1..gain_token_index],
        |token| token.is_word("and"),
    )
    .map(|index| get_token_index + 1 + index) else {
        return Ok(None);
    };
    let modifier_tokens = trim_lexed_commas(&tokens[get_token_index + 1..and_token_index]);
    let Some(pump_head) =
        crate::grammar::effects::gain_ability_shapes::parse_gain_pump_head_shape(modifier_tokens)
    else {
        return Ok(None);
    };
    let (crate::effect::Value::Fixed(power_per), crate::effect::Value::Fixed(toughness_per)) =
        (&pump_head.power, &pump_head.toughness)
    else {
        return Ok(None);
    };
    let Some(count) = crate::effect_sentences::parse_get_for_each_count_value(
        modifier_tokens.get(1..).unwrap_or_default(),
    )?
    else {
        return Ok(None);
    };

    let self_tokens = crate::lexer::lex_line("this creature", 0)?;
    let mut ability_tokens = Vec::with_capacity(tokens.len());
    ability_tokens.extend_from_slice(&tokens[..subject_start]);
    ability_tokens.extend(self_tokens);
    ability_tokens.extend_from_slice(&tokens[gain_token_index..]);
    let Some(grant) =
        crate::effect_sentences::parse_simple_gain_ability_clause_lexed(&ability_tokens)?
    else {
        return Ok(None);
    };

    let target = TargetAst::Object(
        ObjectFilter::source_with_surface(surface.clone()),
        None,
        None,
    );
    let pump = EffectAst::subject_verb_pump_for_each(
        *power_per,
        *toughness_per,
        target,
        count,
        Until::EndOfTurn,
    );
    let mut effects = vec![EffectAst::Coordinated {
        effects: vec![pump, grant],
        leading_duration: true,
        result_conjunction: false,
    }];

    fn apply_surface(target: &mut TargetAst, surface: &crate::target::SourceReferenceSurface) {
        match target {
            TargetAst::Source(span) => {
                *target = TargetAst::Object(
                    ObjectFilter::source_with_surface(surface.clone()),
                    None,
                    *span,
                );
            }
            TargetAst::Object(filter, _, _) if filter.source => {
                filter.source_surface = Some(surface.clone());
            }
            _ => {}
        }
    }
    fn apply(effects: &mut [EffectAst], surface: &crate::target::SourceReferenceSurface) {
        for effect in effects {
            if let EffectAst::SubjectVerb(subject_verb) = effect {
                match &mut subject_verb.action {
                    SubjectVerbActionAst::Pump { target, .. }
                    | SubjectVerbActionAst::PumpForEach { target, .. }
                    | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. } => {
                        apply_surface(target, surface);
                    }
                    _ => {}
                }
            }
            for_each_nested_effects_mut(effect, true, |nested| apply(nested, surface));
        }
    }
    apply(&mut effects, &surface);
    Ok(Some(effects))
}

pub struct ParsedActivatedLine {
    pub chunk: LineAst,
    pub restrictions: ParsedRestrictions,
}

#[cfg(test)]
#[path = "activated_inline_choose_color_of_object_tests.rs"]
mod choose_color_of_object_tests;

#[cfg(test)]
#[path = "activated_inline_hidden_look_partition_activated_tests_2.rs"]
mod hidden_look_partition_activated_tests;

#[cfg(test)]
#[path = "activated_inline_leading_duration_gain_activated_tests_3.rs"]
mod leading_duration_gain_activated_tests;

#[cfg(test)]
#[path = "activated_inline_trailing_sorcery_speed_surface_tests_4.rs"]
mod trailing_sorcery_speed_surface_tests;

#[path = "activated/activated_permission_programs.rs"]
mod activated_permission_programs;
pub use activated_permission_programs::parse_activated_line;
use activated_permission_programs::{
    activated_presentation_display, infer_rewrite_activated_functional_zones,
    parse_activated_effects_lexed, parse_activated_line_impl,
};
#[path = "activated/activated_choice_programs.rs"]
mod activated_choice_programs;
use activated_choice_programs::apply_chosen_option_condition_to_activated;
#[path = "activated/activated_library_programs.rs"]
mod activated_library_programs;
use activated_library_programs::mark_forecast_reveal_duration;
#[path = "activated/activated_reference_programs.rs"]
mod activated_reference_programs;
use activated_reference_programs::reconcile_named_source_action_surfaces;
#[path = "activated/activated_resource_programs.rs"]
mod activated_resource_programs;
use activated_resource_programs::normalize_mana_replacement_effects;
#[path = "activated/activated_condition_programs.rs"]
mod activated_condition_programs;
use activated_condition_programs::rewrite_self_replacements_as_conditionals;
