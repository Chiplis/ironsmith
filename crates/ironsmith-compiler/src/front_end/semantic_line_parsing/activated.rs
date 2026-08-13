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
use crate::effect::Effect;
use crate::object::CounterType;
use crate::model::compiler_semantic::ParsedManaRestriction;
use crate::util::activation_cost_reference_imports;
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
    let choose_words = token_word_refs(choose_sentence);
    let add_words = token_word_refs(add_sentence);
    if !choose_words.starts_with(&["choose", "a", "color", "of"])
        || choose_words.len() <= 4
        || add_words != ["add", "one", "mana", "of", "that", "color"]
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
    let mut filter =
        crate::object_filters::parse_object_filter(&filter_tokens, false)?;
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

fn activation_cost_defines_x_for_mana_ability(cost: &TotalCost) -> bool {
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
        Cost::Mana(cost) => cost.has_x(),
        Cost::Energy(amount) | Cost::Life(amount) | Cost::Mill(amount) => value_uses_x(amount),
        Cost::RemoveAnyCountersFromSource { .. } => true,
        Cost::Effect(effect) => {
            effect
                .downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
                .is_some_and(|effect| effect.display_x)
                || effect
                    .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                    .is_some_and(|effect| effect.count.is_dynamic_x())
                || effect
                    .downcast_ref::<crate::effects::SacrificeEffect>()
                    .is_some_and(|_| false)
                || effect
                    .downcast_ref::<crate::effects::DiscardEffect>()
                    .is_some_and(|_| false)
                || effect
                    .downcast_ref::<crate::effects::MillEffect>()
                    .is_some_and(|_| false)
                || effect
                    .downcast_ref::<crate::effects::PayEnergyEffect>()
                    .is_some_and(|effect| value_uses_x(&effect.amount))
                || effect
                    .downcast_ref::<crate::effects::RemoveCountersEffect>()
                    .is_some_and(|effect| value_uses_x(&effect.count))
        }
        _ => false,
    })
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
        &sentences,
        0,
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
    let Some(get_token_index) = tokens.iter().position(|token| {
        token.as_word().is_some_and(|word| {
            word.eq_ignore_ascii_case("get") || word.eq_ignore_ascii_case("gets")
        })
    }) else {
        return Ok(None);
    };
    let Some(get_word_index) = words
        .iter()
        .position(|word| word.eq_ignore_ascii_case("get") || word.eq_ignore_ascii_case("gets"))
    else {
        return Ok(None);
    };
    if !words[get_word_index + 1..]
        .iter()
        .any(|word| word.eq_ignore_ascii_case("gain") || word.eq_ignore_ascii_case("gains"))
    {
        return Ok(None);
    }
    let subject_start = tokens[..get_token_index]
        .iter()
        .rposition(|token| token.kind == TokenKind::Comma)
        .map_or(0, |index| index + 1);
    let subject_tokens = trim_lexed_commas(&tokens[subject_start..get_token_index]);
    let subject_words = token_word_refs(subject_tokens);
    let authored_subject = subject_words.join(" ");
    let contextual_surface =
        crate::util::current_source_reference_name().and_then(
            |source_name| {
                if authored_subject.eq_ignore_ascii_case(&source_name) {
                    return Some(crate::target::SourceReferenceSurface::FullName(source_name));
                }
                let short_name = source_name.split(',').next()?.trim().to_string();
                authored_subject
                    .eq_ignore_ascii_case(&short_name)
                    .then_some(crate::target::SourceReferenceSurface::ShortName(short_name))
            },
        );
    let Some(surface) = contextual_surface
        .or_else(|| {
            crate::util::source_reference_surface_for_words(
                &subject_words,
            )
        })
        .or_else(|| {
            crate::util::this_source_surface_for_words(
                &subject_words,
            )
        })
    else {
        return Ok(None);
    };

    let Some(gain_token_index) = tokens[get_token_index + 1..]
        .iter()
        .position(|token| {
            token.as_word().is_some_and(|word| {
                word.eq_ignore_ascii_case("gain") || word.eq_ignore_ascii_case("gains")
            })
        })
        .map(|index| get_token_index + 1 + index)
    else {
        return Ok(None);
    };
    let Some(and_token_index) = tokens[get_token_index + 1..gain_token_index]
        .iter()
        .rposition(|token| token.is_word("and"))
        .map(|index| get_token_index + 1 + index)
    else {
        return Ok(None);
    };
    let modifier_tokens = trim_lexed_commas(&tokens[get_token_index + 1..and_token_index]);
    let Some(pump_head) = crate::grammar::effects::gain_ability_shapes::parse_gain_pump_head_shape(modifier_tokens) else {
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

    let self_tokens = crate::front_end::lexer::lex_line("this creature", 0)?;
    let mut ability_tokens = Vec::with_capacity(tokens.len());
    ability_tokens.extend_from_slice(&tokens[..subject_start]);
    ability_tokens.extend(self_tokens);
    ability_tokens.extend_from_slice(&tokens[gain_token_index..]);
    let Some(grant) =
        crate::effect_sentences::parse_simple_gain_ability_clause_lexed(
            &ability_tokens,
        )?
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

fn parse_activated_effects_lexed(
    _effect_text: &str,
    tokens: &[OwnedLexToken],
    _line_index: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effect) = parse_choose_color_of_matching_object_mana_effect(tokens)? {
        return Ok(vec![effect]);
    }
    if activated_effect_is_for_each_color_among_add_mana_lexed(tokens) {
        return Ok(vec![
            crate::activation_helpers::parse_add_mana(tokens, None)?,
        ]);
    }
    if let Some(effects) = parse_each_player_and_their_creatures_damage_sentence(tokens) {
        return Ok(effects);
    }
    if let Some(effects) = parse_hidden_look_partition_activated(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_named_source_leading_gain_activated(tokens)? {
        return Ok(effects);
    }
    // Keep the P/T modification and evasion restriction as one activated
    // program. The broad restriction-oriented source-boundary parser can
    // otherwise claim only the trailing `can't be blocked` arm.
    if let Some(effects) =
        crate::effect_sentences::parse_source_gets_unblockable_subject_verb(
            tokens,
        )?
    {
        return Ok(effects);
    }
    let words = token_word_refs(tokens);
    if crate::grammar::effects::gain_ability_shapes::parse_leading_gain_duration_shape(
        &words,
    )
    .is_some()
    {
        if let Some(effects) =
            crate::effect_sentences::parse_gain_ability_sentence(tokens)?
        {
            fn contains_compound_members(effects: &[EffectAst]) -> bool {
                let mut pump = false;
                let mut grant = false;
                fn inspect(effects: &[EffectAst], pump: &mut bool, grant: &mut bool) {
                    for effect in effects {
                        if let EffectAst::SubjectVerb(subject_verb) = effect {
                            *pump |= matches!(subject_verb.action, SubjectVerbActionAst::Pump { .. });
                            *grant |= matches!(
                                subject_verb.action,
                                SubjectVerbActionAst::GrantAbilitiesToTarget { .. }
                            );
                        }
                        crate::model::effect_ast_traversal::for_each_nested_effects(
                            effect,
                            true,
                            |nested| inspect(nested, pump, grant),
                        );
                    }
                }
                inspect(effects, &mut pump, &mut grant);
                pump && grant
            }
            // Activated bodies such as "Until end of turn, this creature gets
            // +1/+1 ... and gains menace" are one coordinated modifier.  The
            // generic source-boundary path can otherwise claim the leading
            // `gets` as an unrelated counter-gain action.
            if contains_compound_members(&effects) {
                return Ok(effects);
            }
        }
    }
    if let Ok(effects) = parse_effect_sentences_preserving_source_boundaries(tokens) {
        return Ok(effects);
    }

    let sentence_chunks = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentence_chunks.is_empty() {
        return Err(CardTextError::ParseError(
            "rewrite activated effect parser found no sentences".to_string(),
        ));
    }

    let mut effects = Vec::new();
    for sentence_lexed in sentence_chunks {
        if let Some(effect) = parse_next_spell_cost_reduction_sentence(sentence_lexed) {
            effects.push(effect);
            continue;
        }
        effects.extend(parse_effect_sentences_lexed(sentence_lexed)?);
    }
    Ok(effects)
}

pub(crate) struct ParsedActivatedLine {
    pub(crate) chunk: LineAst,
    pub(crate) restrictions: ParsedRestrictions,
}

pub(crate) fn parse_activated_line(
    info: LineInfo,
    cost: TotalCost,
    compiler_cost: crate::model::CompilerTotalCost,
    cost_parse_tokens: Vec<OwnedLexToken>,
    effect_parse_tokens: Vec<OwnedLexToken>,
    timing_hint: ActivationTiming,
    is_loyalty_ability: bool,
    presentation: Option<PresentationLabel>,
    chosen_option: Option<ChosenOptionContext>,
) -> Result<ParsedActivatedLine, CardTextError> {
    // Labeled/public activation parsing first produces the generic cost CST.
    // Reconcile the exact zone-movement payment from its retained cost tokens
    // before that CST's broad `put ... cards` interpretation can survive as a
    // counter-placement cost. The grammar is strict about count, source zone,
    // ownership scope, and library destination.
    let cost = crate::families::activation_and_restrictions::parse_single_graveyard_bottom_library_payment(
        &cost_parse_tokens,
    )?
    .unwrap_or(cost);
    parse_activated_line_impl(
        &RewriteActivatedLine {
            functional_zones: activated_grammar::parse_activated_functional_zones_tokens(
                &cost_parse_tokens,
                &effect_parse_tokens,
            ),
            presentation_kind: activated_grammar::parse_activated_presentation_kind_tokens(
                &info.source_tokens,
            ),
            presentation,
            info,
            cost,
            compiler_cost,
            cost_parse_tokens: cost_parse_tokens.clone(),
            effect_parse_tokens: effect_parse_tokens.clone(),
            timing_hint,
            is_loyalty_ability,
            chosen_option,
        },
        &effect_parse_tokens,
    )
}

fn parse_activated_line_impl(
    line: &RewriteActivatedLine,
    original_effect_parse_tokens: &[OwnedLexToken],
) -> Result<ParsedActivatedLine, CardTextError> {
    let has_x_definition_value =
        activated_x_definition_value(original_effect_parse_tokens).is_some();
    let SplitRewriteActivatedEffectText {
        effect_text,
        effect_parse_tokens,
        restrictions,
        mana_restrictions,
        x_cant_be_zero,
    } = split_rewrite_activated_effect_text(original_effect_parse_tokens);
    if effect_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite activated lowering produced no parsed effect text for '{}'",
            line.info.raw_line
        )));
    }

    let normalized_cost = line.cost.clone();
    let original_effect_mentions_where_x =
        activated_grammar::contains_where_x_definition(original_effect_parse_tokens);
    let ability_text = rewrite_activated_display_text(line);
    let presentation_display = activated_presentation_display(line);
    let is_forecast = presentation_display
        .as_deref()
        .is_some_and(|display| display.eq_ignore_ascii_case("Forecast"));
    let normalized_cost = if is_forecast {
        mark_forecast_reveal_duration(normalized_cost)
    } else {
        normalized_cost
    };
    let activation_timing = if is_forecast {
        ActivationTiming::DuringSourceOwnersUpkeep
    } else {
        line.timing_hint
    };
    let activation_restrictions = is_forecast
        .then_some(crate::ConditionExpr::MaxActivationsPerTurn(1))
        .into_iter()
        .collect::<Vec<_>>();
    let mut additional_activation_restrictions = if line.presentation_kind
        == Some(crate::ir::ActivatedPresentationKind::Exhaust)
    {
        vec!["Activate each exhaust ability only once.".to_string()]
    } else {
        Vec::new()
    };
    if let Some(display) = presentation_display.as_deref() {
        additional_activation_restrictions.push(format!("__ironsmith_activation_label:{display}"));
    }
    if x_cant_be_zero {
        additional_activation_restrictions.push("X can't be 0.".to_string());
    }
    if activated_grammar::contains_add_x_mana(&effect_parse_tokens)
        && !has_x_definition_value
        && !original_effect_mentions_where_x
        && !activation_cost_defines_x_for_mana_ability(&normalized_cost)
    {
        return Err(CardTextError::ParseError(
            "unresolved X in mana ability".to_string(),
        ));
    }

    if let Some(level) = activated_grammar::parse_level_number_tokens(&effect_parse_tokens) {
        let parsed = ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: normalized_cost,
                    effects: ResolutionProgram::from_effects(vec![Effect::put_counters_on_source(
                        CounterType::Level,
                        1,
                    )]),
                    choices: vec![],
                    timing: ActivationTiming::SorcerySpeed,
                    is_loyalty_ability: line.is_loyalty_ability,
                    additional_restrictions: vec![format!("__ironsmith_class_level:{level}")],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones: vec![Zone::Battlefield],
            }
            .into(),
            text: Some(line.info.raw_line.trim().to_string()),
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        };
        return Ok(ParsedActivatedLine {
            chunk: LineAst::Ability(parsed),
            restrictions,
        });
    }

    if let Some(spec) = parse_fixed_mana_output_clause_spec_lexed(&effect_parse_tokens) {
        let functional_zones = infer_rewrite_activated_functional_zones(line)?;
        let mut parsed = ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: normalized_cost.clone(),
                    effects: ResolutionProgram::default(),
                    choices: vec![],
                    timing: activation_timing,
                    is_loyalty_ability: line.is_loyalty_ability,
                    additional_restrictions: additional_activation_restrictions.clone(),
                    activation_restrictions: activation_restrictions.clone(),
                    mana_output: Some(spec.mana),
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones: if functional_zones.is_empty() {
                    vec![Zone::Battlefield]
                } else {
                    functional_zones
                },
            }
            .into(),
            text: ability_text.clone(),
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        };
        apply_pending_mana_restrictions(&mut parsed, &mana_restrictions)?;
        apply_chosen_option_condition_to_activated(&mut parsed, line.chosen_option.as_ref());
        return Ok(ParsedActivatedLine {
            chunk: LineAst::Ability(parsed),
            restrictions,
        });
    }

    if activated_effect_may_be_mana_ability_lexed(&effect_parse_tokens) {
        let effects_ast = parse_activated_effects_lexed(
            effect_text.as_str(),
            &effect_parse_tokens,
            line.info.line_index,
        )?;
        if effects_ast_can_lower_as_mana_ability(&effects_ast)
            || effects_ast
                .first()
                .is_some_and(effect_ast_starts_with_mana_effect)
        {
            let functional_zones = infer_rewrite_activated_functional_zones(line)?;
            let reference_imports = activation_cost_reference_imports(&normalized_cost);
            let mut parsed = ParsedAbility {
                ability: Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost: normalized_cost.clone(),
                        effects: ResolutionProgram::default(),
                        choices: vec![],
                        timing: activation_timing,
                        is_loyalty_ability: line.is_loyalty_ability,
                        additional_restrictions: additional_activation_restrictions.clone(),
                        activation_restrictions: activation_restrictions.clone(),
                        mana_output: Some(vec![]),
                        activation_condition: None,
                        mana_usage_restrictions: vec![],
                    }),
                    functional_zones: if functional_zones.is_empty() {
                        vec![Zone::Battlefield]
                    } else {
                        functional_zones
                    },
                }
                .into(),
                text: ability_text.clone(),
                effects_ast: Some(effects_ast),
                reference_imports,
                trigger_spec: None,
            };
            apply_pending_mana_restrictions(&mut parsed, &mana_restrictions)?;
            apply_chosen_option_condition_to_activated(&mut parsed, line.chosen_option.as_ref());

            return Ok(ParsedActivatedLine {
                chunk: LineAst::Ability(parsed),
                restrictions,
            });
        }
        return Err(CardTextError::ParseError(format!(
            "rewrite activated lowering does not yet support mana-style activated effect '{}'",
            line.info.raw_line
        )));
    }

    let mut effects_ast = parse_activated_effects_lexed(
        effect_text.as_str(),
        &effect_parse_tokens,
        line.info.line_index,
    )?;
    let functional_zones = infer_rewrite_activated_functional_zones(line)?;
    let reference_imports = activation_cost_reference_imports(&normalized_cost);
    let mut parsed = ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: normalized_cost,
                effects: ResolutionProgram::default(),
                choices: vec![],
                timing: activation_timing,
                is_loyalty_ability: line.is_loyalty_ability,
                additional_restrictions: additional_activation_restrictions,
                activation_restrictions,
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: if functional_zones.is_empty() {
                vec![Zone::Battlefield]
            } else {
                functional_zones
            },
        }
        .into(),
        text: ability_text,
        effects_ast: Some(effects_ast),
        reference_imports,
        trigger_spec: None,
    };
    apply_pending_mana_restrictions(&mut parsed, &mana_restrictions)?;
    apply_chosen_option_condition_to_activated(&mut parsed, line.chosen_option.as_ref());

    Ok(ParsedActivatedLine {
        chunk: LineAst::Ability(parsed),
        restrictions,
    })
}

fn mark_forecast_reveal_duration(cost: crate::cost::TotalCost) -> crate::cost::TotalCost {
    cost.try_map(|component| {
        component.try_map_effect(
            |effect| -> Result<crate::effect::Effect, std::convert::Infallible> {
                if effect
                    .downcast_ref::<crate::effects::RevealSourceFromHandEffect>()
                    .is_some()
                {
                    Ok(crate::effect::Effect::reveal_source_from_hand_until_upkeep_ends())
                } else {
                    Ok(effect)
                }
            },
        )
    })
    .expect("mapping a Forecast reveal cost is infallible")
}

fn apply_chosen_option_condition_to_activated(
    parsed: &mut ParsedAbility,
    chosen_option: Option<&ChosenOptionContext>,
) {
    let Some(context) = chosen_option else {
        return;
    };
    let condition = condition_for_chosen_option(context);
    let AbilityKind::Activated(activated) = parsed.kind_mut() else {
        return;
    };
    activated.activation_condition = Some(match activated.activation_condition.take() {
        Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
        None => condition,
    });
    if let Some(threshold) = context.station_threshold() {
        // Renderer-only surface metadata derived from the typed station fact;
        // no later stage parses Oracle text to recover this threshold.
        activated
            .additional_restrictions
            .push(format!("__ironsmith_station_threshold:{threshold}"));
    }
}

fn rewrite_activated_display_text(line: &RewriteActivatedLine) -> Option<String> {
    let display = activated_presentation_display(line)?;
    Some(format!(
        "{display} — {}: {}",
        render_token_slice(&line.cost_parse_tokens).trim(),
        render_token_slice(&line.effect_parse_tokens).trim()
    ))
}

fn activated_presentation_display(line: &RewriteActivatedLine) -> Option<String> {
    line.presentation
        .as_ref()
        .and_then(PresentationLabel::display_prefix)
        .or_else(|| {
            line.presentation_kind
                .map(|kind| kind.display().to_string())
        })
}

fn infer_rewrite_activated_functional_zones(
    line: &RewriteActivatedLine,
) -> Result<Vec<Zone>, CardTextError> {
    Ok(line.functional_zones.clone())
}

#[cfg(test)]
mod choose_color_of_object_tests {
    use super::*;

    #[test]
    fn chooses_a_color_from_the_filtered_objects_instead_of_an_object() {
        let tokens = crate::runtime_backend::lex_line(
            "Choose a color of a permanent you control. Add one mana of that color.",
            0,
        )
        .expect("dynamic color-choice sentence should lex");
        let effects = parse_activated_effects_lexed("", &tokens, 0)
            .expect("dynamic color-choice sentence should parse");
        let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
            panic!("expected one typed mana effect, got {effects:#?}");
        };
        let SubjectVerbActionAst::AddOneManaAnyColorAmong {
            filter,
            choose_color_of_object_surface,
        } = &subject_verb.action
        else {
            panic!("expected a restricted color-choice effect, got {effects:#?}");
        };
        assert!(*choose_color_of_object_surface);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert!(filter.card_types.is_empty(), "{filter:#?}");
    }

    #[test]
    fn chooses_a_color_of_a_typed_permanent_without_erasing_that_type() {
        let tokens = crate::runtime_backend::lex_line(
            "Choose a color of an artifact you control. Add one mana of that color.",
            0,
        )
        .expect("typed color-choice sentence should lex");
        let effects = parse_activated_effects_lexed("", &tokens, 0)
            .expect("typed color-choice sentence should parse");
        let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
            panic!("expected one typed mana effect, got {effects:#?}");
        };
        let SubjectVerbActionAst::AddOneManaAnyColorAmong { filter, .. } = &subject_verb.action
        else {
            panic!("expected a restricted color-choice effect, got {effects:#?}");
        };
        assert_eq!(filter.card_types, [CardType::Artifact]);
    }

    #[test]
    fn unrelated_choose_object_then_chosen_color_is_not_reinterpreted() {
        let tokens = crate::runtime_backend::lex_line(
            "Choose a permanent you control. Add one mana of the chosen color.",
            0,
        )
        .expect("near-miss sentence should lex");
        assert!(!is_choose_color_of_matching_object_mana_shape(&tokens));
    }
}

#[cfg(test)]
mod hidden_look_partition_activated_tests {
    use super::*;

    fn parse(text: &str) -> Option<Vec<EffectAst>> {
        let tokens = crate::runtime_backend::lex_line(text, 0).expect("activated body should lex");
        parse_hidden_look_partition_activated(&tokens).expect("typed activated partition parser")
    }

    #[test]
    fn activated_body_keeps_one_hidden_exiled_card_and_its_permission_linked() {
        let tokens = crate::runtime_backend::lex_line(
            "Look at the top three cards of your library. Exile one face down and put the rest on the bottom of your library in any order. For as long as it remains exiled, you may cast it if it's a creature spell.",
            0,
        )
        .expect("activated body should lex");
        let effects = parse_activated_effects_lexed("", &tokens, 0)
            .expect("activated route should keep the exact hidden looked-card partition");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
        assert!(
            debug.contains("GrantPlayTaggedForAsLongAsExiled"),
            "{debug}"
        );
        assert!(debug.contains("Creature"), "{debug}");
    }

    #[test]
    fn unrelated_exile_one_sentence_is_not_claimed() {
        assert!(
            parse(
                "Look at the top three cards of your library. Exile one face up and put the rest on the bottom of your library in any order. Draw a card."
            )
            .is_none()
        );
    }
}

#[cfg(test)]
mod leading_duration_gain_activated_tests {
    use super::*;

    #[test]
    fn activated_pump_and_keyword_share_the_leading_duration() {
        crate::runtime_backend::front_end::shared::util::with_source_reference_context(
            "Azula, Ruthless Firebender",
            || {
                for text in [
                    "Until end of turn, this creature gets +1/+1 for each experience counter you have and gains menace.",
                    "Until end of turn, Azula gets +1/+1 for each experience counter you have and gains menace.",
                ] {
                    let tokens = crate::runtime_backend::lex_line(text, 0)
                        .expect("activated body should lex");
                    let effects = parse_activated_effects_lexed("", &tokens, 0)
                        .expect("activated pump-and-keyword body should parse");
                    let debug = format!("{effects:#?}");
                    assert!(debug.contains("Pump"), "{debug}");
                    assert!(debug.contains("PlayerCounters"), "{debug}");
                    assert!(debug.contains("Menace"), "{debug}");
                    assert!(!debug.contains("ExperienceCounters"), "{debug}");
                }
            },
        );
    }
}
