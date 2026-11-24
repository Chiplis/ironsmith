use crate::cards::builders::{
    ActivationTiming, CardTextError, ConditionalModeSelection, EffectAst, EffectPredicate,
    IfResultPredicate, LineInfo, ParsedConditionalModeChange, ParsedModalActivatedHeader,
    ParsedModalGate, ParsedModalHeader,
};
use crate::effect::Value;
use crate::target::PlayerFilter;
use ironsmith_core::ValueSurfaceHint;

use super::activation_and_restrictions::activated_line_core::{
    infer_activated_functional_zones_lexed, parse_activate_only_timing_lexed,
};
use super::clause_support::{parse_effect_sentences_lexed, parse_trigger_clause_lexed};
use super::grammar::abilities::parse_activation_condition_lexed;
use super::grammar::activation_costs::parse_activation_cost_tokens;
use super::grammar::primitives as grammar;
use super::grammar::structure::{
    ModalHeaderChooseSpec, parse_modal_header_choose_spec, parse_opponent_modal_choose_spec,
    scan_modal_header_flags,
    split_lexed_sentences, split_trailing_modal_gate_clause,
};
use super::keyword_static::parse_value_binding_clause_lexed;
use super::lexer::{
    OwnedLexToken, TokenKind, contains_token_word, locate_token_word_choice, render_token_slice,
    trim_lexed_commas,
};
use super::semantic_assembly::assemble_activation_cost;

type ModalHeader = ParsedModalHeader;
type ModalActivatedHeader = ParsedModalActivatedHeader;
type ModalGate = ParsedModalGate;

/// The comma that ends a modal trigger's event. A serial comma inside the
/// event's characteristic list ("a spell with power, toughness, or mana value
/// 4", The Fantastic Four) continues the trigger, so a comma followed by
/// another characteristic of that list is skipped.
pub(crate) fn first_trigger_boundary_comma(tokens: &[OwnedLexToken], choose_idx: usize) -> Option<usize> {
    tokens
        .iter()
        .enumerate()
        .take(choose_idx)
        .filter(|(_, token)| token.kind == TokenKind::Comma)
        .map(|(idx, _)| idx)
        .find(|idx| !crate::grammar::trigger_subjects::comma_continues_characteristic_list(&tokens[idx + 1..]))
        .or_else(|| locate_token_index(tokens, |token| token.kind == TokenKind::Comma))
}

fn locate_token_index(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if predicate(&tokens[idx]) {
            return Some(idx);
        }
        idx += 1;
    }

    None
}

fn strip_leading_sign(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    if bytes
        .first()
        .is_some_and(|byte| matches!(*byte, b'+' | b'-'))
    {
        return text.get(1..);
    }

    None
}

/// Parse a return action authored once between a modal choice instruction and
/// its bullet list. Restrict this to demonstrative object follow-ups so
/// ordinary modal metadata sentences (for example, repeated-mode permission)
/// and not-yet-specialized common actions remain header text.
fn parse_modal_common_suffix_effects(
    tokens: &[OwnedLexToken],
    choose_idx: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    let Some(sentence_end) = tokens
        .iter()
        .enumerate()
        .skip(choose_idx)
        .find_map(|(index, token)| token.is_period().then_some(index))
    else {
        return Ok(Vec::new());
    };

    let mut effects = Vec::new();
    for sentence in split_lexed_sentences(&tokens[sentence_end + 1..]) {
        // Detection only: the parse below reads the words it uses.
        let (return_action, demonstrative_object) = crate::word_claims::probe_only(|| {
            (
                sentence.first().is_some_and(|token| token.is_word("return")),
                sentence.iter().any(|token| {
                    ["it", "them", "that", "those"].iter().any(|word| token.is_word(word))
                }),
            )
        });
        if return_action && demonstrative_object {
            effects.extend(parse_effect_sentences_lexed(sentence)?);
        }
    }
    Ok(effects)
}

/// Parse effects authored once after the modal instruction itself, as in
/// "choose one and this creature gets +1/+1 until end of turn." These effects
/// are neither pre-choice setup nor part of any individual bullet.
fn parse_modal_common_prefix_effects(
    tokens: &[OwnedLexToken],
    choose_idx: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    let Some(sentence_end) =
        tokens
            .iter()
            .enumerate()
            .skip(choose_idx)
            .find_map(|(index, token)| {
                (token.is_period() || matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
                    .then_some(index)
            })
    else {
        return Ok(Vec::new());
    };
    let Some(and_idx) = crate::slice_primitives::select_last_position(
        &tokens[choose_idx + 1..sentence_end],
        |token| token.is_word("and"),
    )
    .map(|offset| choose_idx + 1 + offset) else {
        return Ok(Vec::new());
    };
    let effect_tokens = trim_lexed_commas(&tokens[and_idx + 1..sentence_end]);
    if effect_tokens.is_empty()
        || effect_tokens
            .first()
            .is_some_and(|token| token.is_word("or"))
    {
        return Ok(Vec::new());
    }
    parse_effect_sentences_lexed(effect_tokens)
}

/// The bounded operator before a modal choose verb; quoted programs do not
/// lend payment semantics to their enclosing header.
pub(crate) fn paid_modal_choice_prefix(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut quoted = false;
    for start in 0..tokens.len() {
        if tokens[start].kind == TokenKind::Quote { quoted = !quoted; continue; }
        if !quoted && tokens.get(start..start + 4).is_some_and(|part|
            part[0].is_word("pay") && part[1].is_word("for")
                && part[2].is_word("and") && part[3].is_word("choose")) {
            return Some(start);
        }
    }
    None
}

pub fn parse_modal_header(
    info: &LineInfo,
    tokens: &[OwnedLexToken],
) -> Result<Option<ModalHeader>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    let prose_spree = words == ["choose", "one", "or", "more", "additional", "costs"];
    let prose_tiered = words == ["choose", "one", "additional", "cost"];
    let mut descriptive_mode_costs = prose_spree || prose_tiered;
    let spree = prose_spree || tokens.first().is_some_and(|token| token.is_word("spree"));
    let tiered = prose_tiered || tokens.first().is_some_and(|token| token.is_word("tiered"));
    // CR 700.2: "An opponent chooses one —" has an opponent choose the modes
    // while the spell is cast.
    let opponent_choose_spec = parse_opponent_modal_choose_spec(tokens);
    let cast_chooser = opponent_choose_spec.as_ref().map(|_| PlayerFilter::Opponent);
    let choose_spec = if let Some(spec) = opponent_choose_spec {
        spec
    } else if spree || tiered {
        ModalHeaderChooseSpec {
            choose_idx: 0,
            min: Value::Fixed(1),
            max: tiered.then_some(Value::Fixed(1)),
            random: false,
            x_clause_start: None,
        }
    } else {
        let Some(choose_spec) = grammar::parse_all_with_display_line(
            tokens,
            parse_modal_header_choose_spec,
            "modal-header",
            info.display_line_index,
        )?
        else {
            return Ok(None);
        };
        choose_spec
    };
    let modal_flags = scan_modal_header_flags(tokens);
    let conditional_mode_change = parse_conditional_mode_change(tokens, choose_spec.choose_idx)?;
    let presentation_label = parse_modal_presentation_label(&info.source_tokens);
    let presentation_prefix_end = presentation_label.as_ref().and_then(|_| {
        tokens
            .iter()
            .enumerate()
            .take(choose_spec.choose_idx)
            .find_map(|(idx, token)| {
                matches!(token.kind, TokenKind::Dash | TokenKind::EmDash).then_some(idx + 1)
            })
    });
    let choose_idx = choose_spec.choose_idx;
    let paid_mode_prefix = paid_modal_choice_prefix(tokens)
        .filter(|start| start + 3 == choose_idx);
    descriptive_mode_costs |= paid_mode_prefix.is_some();
    let min = choose_spec.min;
    let max = choose_spec.max;
    let random = choose_spec.random;
    // "Choose two or four —": only those exact counts, not the range
    // between them.
    let allowed_mode_counts = {
        let tail = crate::lexer::token_word_refs(&tokens[choose_idx + 1..]);
        match tail.as_slice() {
            [low, "or", high, ..] => match (ironsmith_core::parse_cardinal_word(low), ironsmith_core::parse_cardinal_word(high)) {
                (Some(low), Some(high)) if high > low + 1 => vec![low, high],
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    };

    let mut trigger = None;
    let mut activated = None;
    let x_replacement = choose_spec.x_clause_start.and_then(|x_clause_start| {
        let value = parse_x_is_value_clause(trim_lexed_commas(&tokens[x_clause_start..]))?;
        // "choose up to X, where X is ...": the binding re-reads the clause
        // behind a synthetic "where"; the authored one is read here.
        let before = trim_lexed_commas(&tokens[..x_clause_start]);
        if let Some(where_token) = before.last()
            && crate::word_claims::probe_only(|| where_token.is_word("where"))
        {
            crate::word_claims::claim_tokens(std::slice::from_ref(where_token));
        }
        Some(value)
    });
    let mut effect_start_idx = presentation_prefix_end.unwrap_or(0);
    if let Some(colon_idx) = locate_token_index(tokens, |token| token.kind == TokenKind::Colon)
        .filter(|idx| *idx < choose_idx)
    {
        let cost_tokens = &tokens[..colon_idx];
        let cost_raw = render_token_slice(cost_tokens);
        let cost_raw = cost_raw.trim();
        if !cost_raw.is_empty() {
            let cost_cst = parse_activation_cost_tokens(cost_tokens)?;
            let mana_cost = assemble_activation_cost(&cost_cst)?.to_core_total_cost();
            let prechoose_tokens = trim_lexed_commas(&tokens[colon_idx + 1..choose_idx]);
            let effect_sentences = if prechoose_tokens.is_empty() {
                Vec::new()
            } else {
                split_lexed_sentences(prechoose_tokens)
            };
            let loyalty_shorthand = is_loyalty_shorthand_cost_text(cost_raw);
            let functional_zones =
                infer_activated_functional_zones_lexed(cost_tokens, &effect_sentences);

            activated = Some(ModalActivatedHeader {
                keyword: crate::grammar::activated_lowering::parse_activated_presentation_kind_tokens(&info.source_tokens)
                    .and_then(crate::ir::ActivatedPresentationKind::keyword),
                mana_cost,
                functional_zones,
                timing: if loyalty_shorthand {
                    ActivationTiming::SorcerySpeed
                } else {
                    ActivationTiming::AnyTime
                },
                is_loyalty_ability: loyalty_shorthand,
                once_per_turn: loyalty_shorthand,
                x_cant_be_zero: false,
                activation_restrictions: Vec::new(),
            });
            effect_start_idx = colon_idx + 1;
        }
    }

    if let Some(activated) = activated.as_mut() {
        for sentence in split_lexed_sentences(&tokens[choose_idx + 1..]) {
            if crate::grammar::effects::dispatch_entry_shapes::is_x_cant_be_zero_tokens(sentence) {
                activated.x_cant_be_zero = true;
            } else if let Some(timing) = parse_activate_only_timing_lexed(sentence) {
                activated.timing = timing;
            } else if let Some(condition) = parse_activation_condition_lexed(sentence) {
                activated.activation_restrictions.push(condition);
            }
        }
    }

    if activated.is_none()
        // Only an actual leading trigger owns this comma. An imperative
        // producer followed by `When you do` is one resolving prefix plus
        // a reflexive gate, not a trigger with the producer in its subject.
        && tokens.get(effect_start_idx).is_some_and(|token|
            token.is_any_word(&["when", "whenever", "at"]))
        && let Some(comma_idx) = first_trigger_boundary_comma(tokens, choose_idx)
        && choose_idx > comma_idx
    {
        // An ability-word label ("Trick Arrows — Whenever ...") moves the
        // start past its dash; the trigger keyword still has to be skipped
        // there, or the trigger subject absorbs it.
        let start_idx = if tokens.get(effect_start_idx).is_some_and(|token| {
            token.is_word("whenever") || token.is_word("when") || token.is_word("at")
        }) {
            effect_start_idx + 1
        } else {
            effect_start_idx
        };
        if comma_idx > start_idx {
            let trigger_tokens = &tokens[start_idx..comma_idx];
            if !trigger_tokens.is_empty() {
                trigger = Some(parse_trigger_clause_lexed(trigger_tokens)?);
            }
        }
        effect_start_idx = comma_idx + 1;
    }

    // "An opponent" names the chooser, not an effect before the choice.
    let prechoose_tokens = if spree || tiered || cast_chooser.is_some() {
        &[]
    } else {
        trim_lexed_commas(&tokens[effect_start_idx..choose_idx])
    };
    // "you may choose two" (CR 700.2): the choice itself is optional, so
    // choosing no modes is also legal. It is not a "may" effect before the
    // choice.
    let optional_choice = prechoose_tokens.len() >= 2
        && prechoose_tokens[prechoose_tokens.len() - 2].is_word("you")
        && prechoose_tokens[prechoose_tokens.len() - 1].is_word("may");
    let prechoose_tokens = if optional_choice {
        trim_lexed_commas(&prechoose_tokens[..prechoose_tokens.len() - 2])
    } else {
        prechoose_tokens
    };
    let prechoose_tokens = if paid_mode_prefix.is_some() {
        let keep = prechoose_tokens.len().checked_sub(3).ok_or_else(||
            CardTextError::ParseError("paid modal header lacks its payment operator".into()))?;
        crate::word_claims::claim_tokens(&prechoose_tokens[keep..]);
        trim_lexed_commas(&prechoose_tokens[..keep])
    } else { prechoose_tokens };
    let (intervening_if, prechoose_tokens) =
        split_modal_trigger_intervening_if(trigger.is_some(), prechoose_tokens);
    let (prefix_effects_ast, modal_gate) = parse_modal_header_prefix_effects(prechoose_tokens)?;
    let common_prefix_effects_ast = parse_modal_common_prefix_effects(tokens, choose_idx)?;
    let common_suffix_effects_ast = parse_modal_common_suffix_effects(tokens, choose_idx)?;
    // The "choose" verb the choose spec was located at.
    crate::word_claims::claim_tokens(&tokens[choose_idx..=choose_idx]);

    Ok(Some(ModalHeader {
        info: info.semantic_info(),
        min,
        max,
        spree,
        tiered,
        descriptive_mode_costs,
        weighted_mode_points: super::grammar::modal::parse_modal_point_header_tokens(tokens)
            .is_some(),
        random,
        same_mode_more_than_once: modal_flags.same_mode_more_than_once,
        mode_must_be_unchosen: modal_flags.mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn: modal_flags.mode_must_be_unchosen_this_turn,
        distinct_player_targets_per_mode: modal_flags.distinct_player_targets_per_mode,
        if_kicked_choose_any_number: modal_flags.if_kicked_choose_any_number,
        conditional_mode_change,
        presentation_label,
        commander_allows_both: modal_flags.commander_allows_both,
        choose_both_control_card_types: modal_flags.choose_both_control_card_types,
        choose_both_exact_life_total: modal_flags.choose_both_exact_life_total,
        trigger,
        intervening_if,
        activated,
        x_replacement,
        prefix_effects_ast,
        common_prefix_effects_ast,
        common_suffix_effects_ast,
        modal_gate,
        cast_chooser,
        optional_choice,
        cost_choice_keywords: Vec::new(),
        allowed_mode_counts,
    }))
}

/// CR 603.4: in `<trigger>, if <condition>, choose one —` the whole
/// pre-choice clause is an intervening-if condition, not a resolution-time
/// conditional effect. Claim it only when the complete clause is a modeled
/// predicate; anything else keeps the ordinary prefix-effect reading.
fn split_modal_trigger_intervening_if(
    has_trigger: bool,
    prechoose_tokens: &[OwnedLexToken],
) -> (
    Option<crate::cards::builders::PredicateAst>,
    &[OwnedLexToken],
) {
    if !has_trigger
        || !prechoose_tokens
            .first()
            .is_some_and(|token| token.is_word("if"))
    {
        return (None, prechoose_tokens);
    }
    let predicate_tokens = trim_lexed_commas(&prechoose_tokens[1..]);
    if predicate_tokens.is_empty() {
        return (None, prechoose_tokens);
    }
    match super::grammar::structure::parse_modeled_predicate(predicate_tokens) {
        Some(predicate) => (Some(predicate), &[]),
        None => (None, prechoose_tokens),
    }
}

fn parse_modal_presentation_label(
    source_tokens: &[OwnedLexToken],
) -> Option<crate::ability::PresentationLabel> {
    let choose_idx =
        crate::slice_primitives::select_position(source_tokens, |token| token.is_word("choose"))?;
    let dash_idx = crate::slice_primitives::select_position(source_tokens, |token| {
        matches!(token.kind, TokenKind::Dash | TokenKind::EmDash)
    })?;
    if dash_idx >= choose_idx {
        return None;
    }
    let label_tokens = trim_lexed_commas(&source_tokens[..dash_idx]);
    let word_count = label_tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Word || token.kind == TokenKind::Number)
        .count();
    if word_count == 0 || word_count > 4 || label_tokens.iter().any(|token| token.is_period()) {
        return None;
    }
    let label = render_token_slice(label_tokens).trim().to_string();
    (!label.is_empty()).then(|| crate::ability::PresentationLabel::from_ability_word(label))
}

/// Parse the typed condition and alternate selection range from the common
/// modal sentence "Choose one. If ..., choose ... instead." Ordinary prose
/// containing either word is rejected unless the complete grammar is present.
fn parse_conditional_mode_change(
    tokens: &[OwnedLexToken],
    base_choose_idx: usize,
) -> Result<Option<ParsedConditionalModeChange>, CardTextError> {
    let Some(if_idx) = tokens
        .iter()
        .enumerate()
        .skip(base_choose_idx + 1)
        .find_map(|(idx, token)| token.matches_word("if").then_some(idx))
    else {
        return Ok(None);
    };
    let Some(comma_idx) = tokens
        .iter()
        .enumerate()
        .skip(if_idx + 1)
        .find_map(|(idx, token)| (token.kind == TokenKind::Comma).then_some(idx))
    else {
        return Ok(None);
    };
    let Some(change_choose_idx) = tokens
        .iter()
        .enumerate()
        .skip(comma_idx + 1)
        .find_map(|(idx, token)| token.matches_word("choose").then_some(idx))
    else {
        return Ok(None);
    };
    let Some(instead_idx) = tokens[change_choose_idx + 1..]
        .iter()
        .position(|token| token.matches_word("instead"))
        .map(|idx| change_choose_idx + 1 + idx)
    else {
        return Ok(None);
    };

    let selection = if tokens.get(change_choose_idx + 1).is_some_and(|token| token.is_word("up"))
        && tokens.get(change_choose_idx + 2).is_some_and(|token| token.is_word("to"))
    {
        let Some(maximum) = tokens.get(change_choose_idx + 3).and_then(|token| token.as_word()).and_then(crate::util::parse_number_word_u32) else { return Ok(None); };
        if !tokens.get(change_choose_idx + 4).is_some_and(|token|token.is_word("instead")) { return Ok(None); }
        ConditionalModeSelection::UpTo(maximum)
    } else if tokens
        .get(change_choose_idx + 1)
        .is_some_and(|token| token.is_word("both") || token.is_word("two"))
    {
        ConditionalModeSelection::BothOrTwo
    } else if tokens
        .get(change_choose_idx + 1)
        .is_some_and(|token| token.is_word("any"))
        && tokens
            .get(change_choose_idx + 2)
            .is_some_and(|token| token.is_word("number"))
    {
        ConditionalModeSelection::AnyNumber
    } else if tokens
        .get(change_choose_idx + 1)
        .is_some_and(|token| token.is_word("one"))
        && tokens
            .get(change_choose_idx + 2)
            .is_some_and(|token| token.is_word("or"))
        && tokens
            .get(change_choose_idx + 3)
            .is_some_and(|token| token.is_word("more"))
    {
        ConditionalModeSelection::OneOrMore
    } else if tokens
        .get(change_choose_idx + 1)
        .is_some_and(|token| token.is_word("one"))
    {
        ConditionalModeSelection::One
    } else {
        return Ok(None);
    };

    let mut condition_end = comma_idx;
    if condition_end >= if_idx + 5
        && tokens[condition_end - 5..condition_end]
            .iter()
            .zip(["as", "you", "cast", "this", "spell"])
            .all(|(token, word)| token.is_word(word))
    {
        condition_end -= 5;
    }
    let condition_tokens = trim_lexed_commas(&tokens[if_idx + 1..condition_end]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let condition = if condition_tokens.len() == 3
        && condition_tokens[0].is_word("it")
        && condition_tokens[1].is_word("was")
        && condition_tokens[2].is_word(crate::keyword_forms::KICKED)
    {
        crate::cards::builders::PredicateAst::ThisSpellWasKicked
    } else {
        super::grammar::filters::parse_condition_predicate_lexed(condition_tokens)?
    };
    let optional = change_choose_idx >= 2
        && tokens[change_choose_idx - 2].is_word("you")
        && tokens[change_choose_idx - 1].is_word("may");
    // "..., you choose one instead": the chooser of the changed selection.
    if !optional
        && change_choose_idx > comma_idx + 1
        && crate::lexer::parser_token_word_refs(&tokens[comma_idx + 1..change_choose_idx]) == ["you"]
    {
        crate::word_claims::claim_tokens(&tokens[comma_idx + 1..change_choose_idx]);
    }
    // The "if" that opens the condition, the changed "choose" and its
    // "instead": the words this reading was located by.
    for idx in [if_idx, change_choose_idx, instead_idx] {
        crate::word_claims::claim_tokens(&tokens[idx..=idx]);
    }
    Ok(Some(ParsedConditionalModeChange {
        condition,
        selection,
        optional,
    }))
}

fn parse_x_is_value_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    if tokens.len() < 2 || !tokens[0].is_word("x") || !tokens[1].is_word("is") {
        return None;
    }

    // "X is the number of spells you've cast this turn" (Gnostro): read
    // exactly, every word claimed.
    {
        let words = crate::lexer::token_word_refs(tokens);
        let player = match words.as_slice() {
            ["x", "is", "the", "number", "of", "spells", "you've" | "youve", "cast", "this", "turn"]
            | ["x", "is", "the", "number", "of", "spells", "you", "have", "cast", "this", "turn"] => {
                Some(PlayerFilter::You)
            }
            ["x", "is", "the", "number", "of", "spells", "cast", "this", "turn"] => {
                Some(PlayerFilter::Any)
            }
            _ => None,
        };
        if let Some(player) = player {
            crate::word_claims::claim_tokens(tokens);
            return Some(Value::SpellsCastThisTurn(player));
        }
    }
    if locate_token_word_choice(tokens, &["spell", "spells"]).is_some()
        && locate_token_word_choice(tokens, &["cast", "casts"]).is_some()
        && contains_token_word(tokens, "turn")
    {
        let player =
            if locate_token_word_choice(tokens, &["you", "your", "youve", "you've"]).is_some() {
                PlayerFilter::You
            } else if locate_token_word_choice(tokens, &["opponent", "opponents"]).is_some() {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
        return Some(Value::SpellsCastThisTurn(player));
    }

    let mut where_prefixed = Vec::with_capacity(tokens.len() + 3);
    where_prefixed.push(OwnedLexToken::word(
        "where",
        tokens
            .first()
            .map(|token| token.span)
            .unwrap_or_else(crate::cards::builders::TextSpan::synthetic),
    ));
    where_prefixed.extend_from_slice(tokens);
    parse_value_binding_clause_lexed(&where_prefixed)
        .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
}

pub use crate::effect_sentences::replace_modal_header_x_in_effects_ast;

fn parse_modal_header_prefix_effects(
    tokens: &[OwnedLexToken],
) -> Result<(Vec<EffectAst>, Option<ModalGate>), CardTextError> {
    if tokens.is_empty() {
        return Ok((Vec::new(), None));
    }

    let (prefix_tokens, modal_gate) =
        if let Some(gate_spec) = split_trailing_modal_gate_clause(tokens) {
            let effect_predicate = match gate_spec.predicate {
                IfResultPredicate::Did | IfResultPredicate::SourcePowerBecomes(_) => {
                    EffectPredicate::Happened
                }
                IfResultPredicate::WonClash => {
                    EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0))
                }
                IfResultPredicate::AcceptedChoice => EffectPredicate::Chosen,
                IfResultPredicate::DidNot
                | IfResultPredicate::ExplicitDidNot
                | IfResultPredicate::Otherwise => EffectPredicate::DidNotHappen,
                IfResultPredicate::SearchedLibrary => EffectPredicate::SearchedLibrary,
                IfResultPredicate::DiesThisWay => EffectPredicate::PriorEffectResult(
                    ironsmith_core::PriorEffectResultSurface::new(
                        ironsmith_core::PriorEffectAction::Died,
                        crate::filter::ObjectFilter::from(ironsmith_core::filter_model::ObjectFilterFields {
                            zone: None,
                            ..ironsmith_core::filter_model::ObjectFilterFields::from(crate::filter::ObjectFilter::creature())
                        }),
                        ironsmith_core::PriorEffectResultActor::Passive,
                        ironsmith_core::PriorEffectResultQuantifier::One,
                    ),
                ),
                IfResultPredicate::ExcessDamageDealt => EffectPredicate::ExcessDamageDealt,
                IfResultPredicate::ExcessDamageDealtToCreature => {
                    EffectPredicate::ExcessDamageDealtToCreature
                }
                IfResultPredicate::ExcessDamageDealtToObjectMatching(filter) => {
                    EffectPredicate::ExcessDamageDealtToObjectMatching(filter)
                }
                IfResultPredicate::DealtDamageToPlayer => EffectPredicate::DealtDamageToPlayer,
                IfResultPredicate::ExcessDamageRedirected => EffectPredicate::ExcessDamageRedirected,
                IfResultPredicate::AffectedObjectMatchesCardType { card_type, negated } => {
                    EffectPredicate::AffectedObjectMatchesCardType { card_type, negated }
                }
                IfResultPredicate::PriorEffectResult(surface) => {
                    EffectPredicate::PriorEffectResult(surface)
                }
                IfResultPredicate::NamedActionCompleted { label } => EffectPredicate::NamedActionCompleted {label},
        IfResultPredicate::ResultObjectsMatch { filter, negated } => {
                    EffectPredicate::ResultObjectsMatch { filter, negated }
                }
                IfResultPredicate::AffectedObjectsShare { required_count, characteristic } => {
                    EffectPredicate::AffectedObjectsShare { required_count, characteristic }
                }
                IfResultPredicate::ConditionMatched => {
                    EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0))
                }
                IfResultPredicate::WasDeclined => EffectPredicate::WasDeclined,
        IfResultPredicate::ChoiceUnavailable => EffectPredicate::ChoiceUnavailable,
                IfResultPredicate::Value(cmp) | IfResultPredicate::DieValue(cmp) => EffectPredicate::Value(cmp),
            };
            (
                gate_spec.prefix_tokens,
                Some(ModalGate {
                    predicate: effect_predicate,
                    remove_mode_only: gate_spec.remove_mode_only,
                    reflexive: gate_spec.reflexive,
                }),
            )
        } else {
            (tokens, None)
        };
    if prefix_tokens.is_empty() {
        return Ok((Vec::new(), modal_gate));
    }

    let effects = parse_effect_sentences_lexed(prefix_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(
            "modal header prefix produced no effects".to_string(),
        ));
    }

    Ok((effects, modal_gate))
}

fn is_loyalty_shorthand_cost_text(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == "0"
        || strip_leading_sign(trimmed)
            .is_some_and(|tail| tail.eq_ignore_ascii_case("x") || tail.parse::<u32>().is_ok())
}

#[test]
fn numeric_keyword_actions_preserve_modal_header_x_binding() {
    use crate::cards::builders::{KeywordActionAst, SubjectVerbActionAst};

    for mut effect in [
        EffectAst::subject_verb_collect_evidence(Value::X),
        EffectAst::subject_verb_empower_jace(Value::X),
    ] {
        replace_modal_header_x_in_effects_ast(
            std::slice::from_mut(&mut effect),
            &Value::Fixed(3),
            "typed keyword action",
        )
        .unwrap();
        let EffectAst::SubjectVerb(subject) = effect else {
            panic!("subject/verb action");
        };
        let amount = match subject.action {
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::CollectEvidence { amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::EmpowerJace { amount }) => {
                amount
            }
            _ => panic!("typed numeric keyword action"),
        };
        assert_eq!(amount, Value::Fixed(3));
    }
}

#[cfg(test)]
mod x_scope_tests {
    use super::*;
    use crate::cards::builders::{
        GrantActionAst, GrantedAbilityAst, LifeResourceActionAst, StaticAbilityAst,
        SubjectVerbActionAst, TargetAst,
    };
    use crate::effect::Until;
    use crate::effect_sentences::replace_unbound_x_in_effects_anywhere;
    use crate::model::CompilerStaticAbilityCore;
    use crate::object::CounterType;
    use crate::tag::CompilerReferenceTag;
    use crate::target::{ChooseSpec, SourceReferenceSurface};

    #[test]
    fn outer_x_does_not_capture_a_granted_activation() {
        let text = r#"Target creature gains "{X}: Draw X cards." until end of turn."#;
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let grant = parse_effect_sentences_lexed(&tokens).unwrap();
        let [EffectAst::SubjectVerb(subject)] = grant.as_slice() else {
            panic!("expected a grant: {grant:?}");
        };
        let SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
            abilities, ..
        }) = &subject.action
        else {
            panic!("expected targeted grant: {subject:?}")
        };
        let [GrantedAbilityAst::ParsedObjectAbility { ability, .. }] = abilities.as_slice() else {
            panic!("expected a parsed ability: {abilities:?}");
        };
        let Some([EffectAst::SubjectVerb(draw)]) = ability.effects_ast.as_deref() else {
            panic!("expected a draw: {ability:?}");
        };
        assert!(matches!(&draw.action, SubjectVerbActionAst::LifeResources(
            LifeResourceActionAst::Draw { count }
        ) if super::super::modal_helpers::value_contains_unbound_x(count)));

        for rewrite in [
            replace_modal_header_x_in_effects_ast,
            replace_unbound_x_in_effects_anywhere,
        ] {
            let mut effects = grant.clone();
            effects.push(EffectAst::subject_verb_collect_evidence(Value::XTimes(2)));
            rewrite(&mut effects, &Value::Fixed(3), text).unwrap();
            assert_eq!(effects[0], grant[0]);
            assert_eq!(
                effects[1],
                EffectAst::subject_verb_collect_evidence(Value::Fixed(6))
            );
        }
    }

    #[test]
    fn only_where_x_binds_granted_entry_counters_and_rebases_it() {
        let grant = |count| {
            EffectAst::subject_verb_grant_abilities_to_target(
                TargetAst::Tagged(CompilerReferenceTag::It.bind(), None),
                vec![GrantedAbilityAst::StaticAbility(Box::new(
                    StaticAbilityAst::Static(
                        CompilerStaticAbilityCore::enters_with_counters_value(
                            CounterType::PlusOnePlusOne,
                            count,
                        ),
                    ),
                ))],
                Until::EndOfTurn,
            )
        };
        let replacement =
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(CompilerReferenceTag::It.key())));
        let mut effects = [grant(Value::X)];
        replace_modal_header_x_in_effects_ast(&mut effects, &replacement, "modal header").unwrap();
        assert_eq!(effects, [grant(Value::X)]);
        replace_unbound_x_in_effects_anywhere(
            &mut effects,
            &replacement,
            "where X is its mana value",
        )
        .unwrap();
        let recipient = ChooseSpec::Source.with_surface_hint(
            ironsmith_core::ChooseSpecSurfaceHint::SourceReference(
                SourceReferenceSurface::ThisPermanentType("it".into()),
            ),
        );
        assert_eq!(effects, [grant(Value::ManaValueOf(Box::new(recipient)))]);
    }
}
