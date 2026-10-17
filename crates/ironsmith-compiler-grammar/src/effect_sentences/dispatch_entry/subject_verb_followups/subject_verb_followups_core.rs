use super::*;

const SINGLE_WORD_KEYWORDS: &[&str] = &[
    "flying", "haste", "vigilance", "trample", "lifelink", "deathtouch", "menace", "reach",
    "indestructible", "hexproof", "defender", "shroud", "flash", "fear", "intimidate",
];

/// "If that creature has haste and Super-Adaptoid doesn't, put a haste counter
/// on Super-Adaptoid. Do the same for flying, first strike, ... and
/// vigilance." (Super-Adaptoid): the previous sentence repeated once per
/// listed keyword, with its keyword replaced.
pub(super) fn pre_rule_do_the_same_for_keywords_followup(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = crate::lexer::token_word_refs(sentence_tokens);
    let Some(list) = words.strip_prefix(&["do", "the", "same", "for"][..]) else {
        return Ok(None);
    };
    let Some(previous) = sentence_idx
        .checked_sub(1)
        .and_then(|index| sentences.get(index))
        .map(SentenceInput::lexed)
    else {
        return Ok(None);
    };
    let Some(original) = previous
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .find(|word| SINGLE_WORD_KEYWORDS.contains(word))
        .map(str::to_string)
    else {
        return Ok(None);
    };
    // Split the authored list into keyword phrases ("first strike").
    let mut keywords: Vec<Vec<&str>> = Vec::new();
    let mut current = Vec::new();
    for word in list {
        if matches!(*word, "and" | "or") {
            if !current.is_empty() {
                keywords.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(*word);
        if SINGLE_WORD_KEYWORDS.contains(word)
            || matches!(current.as_slice(), ["first", "strike"] | ["double", "strike"])
        {
            keywords.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() || keywords.is_empty() {
        return Ok(None);
    }
    let mut repeated = Vec::new();
    for keyword in keywords {
        let mut rewritten = Vec::new();
        for token in previous {
            if token.as_word() == Some(original.as_str()) {
                rewritten.extend(keyword.iter().map(|word| {
                    OwnedLexToken::word(*word, crate::cards::builders::TextSpan::synthetic())
                }));
            } else {
                rewritten.push(token.clone());
            }
        }
        repeated.extend(crate::effect_sentences::parse_effect_sentence_lexed(&rewritten)?);
    }
    state.effects.extend(repeated);
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

/// "This effect can't reduce their speed below 1." (Spikeshell Harrier):
/// the floor of the preceding speed reduction.
pub(crate) fn speed_floor_followup_minimum(tokens: &[OwnedLexToken]) -> Option<u8> {
    let words = crate::lexer::token_word_refs(tokens);
    let [
        "this",
        "effect",
        "can't" | "cant",
        "reduce",
        "their" | "your" | "that" | "its",
        rest @ ..,
    ] = words.as_slice()
    else {
        return None;
    };
    let rest = match rest {
        ["player's" | "players" | "opponent's" | "opponents", tail @ ..] => tail,
        tail => tail,
    };
    let ["speed", "below", amount] = rest else {
        return None;
    };
    crate::util::parse_number_word_u32(amount)
        .or_else(|| crate::util::decimal_count(amount))
        .and_then(|value| u8::try_from(value).ok())
}

fn set_reduce_speed_minimum(effect: &mut EffectAst, floor: u8) -> bool {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Characteristics(
                crate::cards::builders::CharacteristicActionAst::ReduceSpeed { minimum, .. },
            ),
        ..
    }) = effect
    {
        *minimum = floor;
        return true;
    }
    let mut applied = false;
    crate::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |effects| {
        for nested in effects.iter_mut() {
            applied |= set_reduce_speed_minimum(nested, floor);
        }
    });
    applied
}

pub(super) fn pre_rule_speed_floor_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(floor) = speed_floor_followup_minimum(sentence_tokens) else {
        return Ok(None);
    };
    let Some(last) = state.effects.last_mut() else {
        return Ok(None);
    };
    if !set_reduce_speed_minimum(last, floor) {
        return Ok(None);
    }
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

fn battlefield_move_mut(effect: &mut EffectAst) -> Option<&mut EffectAst> {
    let is_move = matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                zone: Zone::Battlefield,
                ..
            }),
            ..
        })
    );
    if is_move {
        return Some(effect);
    }
    match effect {
        EffectAst::Permissions(
            PermissionEffectAst::May { effects } | PermissionEffectAst::MayByPlayer { effects, .. },
        ) if effects.len() == 1 => battlefield_move_mut(&mut effects[0]),
        _ => None,
    }
}

/// "You may put a creature card from your hand onto the battlefield. If that
/// card is an enchantment card, it enters tapped and attacking." (Summoner's
/// Grimoire): the condition selects how the preceding put enters.
pub(super) fn pre_rule_conditional_entry_modifier_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some((intro, body)) = crate::split_lexed_once_on_comma(sentence_tokens) else {
        return Ok(None);
    };
    let body_words = crate::lexer::parser_token_word_refs(body);
    let attacking = match body_words.as_slice() {
        ["it", "enters", "tapped"] => false,
        ["it", "enters", "tapped", "and", "attacking"] => true,
        _ => return Ok(None),
    };
    if !intro.first().is_some_and(|token| token.is_word("if")) || intro.len() < 2 {
        return Ok(None);
    }
    let predicate_tokens = &intro[1..];
    let predicate = match crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(
        predicate_tokens,
    ) {
        Ok(predicate) => predicate,
        Err(_) => {
            let words = crate::lexer::parser_token_word_refs(predicate_tokens);
            let Some(is_idx) = predicate_tokens.iter().position(|token| token.is_word("is")) else {
                return Ok(None);
            };
            if !matches!(words.as_slice(), ["that", "card", "is", ..]) {
                return Ok(None);
            }
            let Some(filter) = crate::grammar::primitives::probe_shape(
                crate::object_filters::parse_object_filter_lexed(
                    &predicate_tokens[is_idx + 1..],
                    false,
                ),
            ) else {
                return Ok(None);
            };
            PredicateAst::ItMatches(filter)
        }
    };
    let Some(last) = state.effects.last_mut() else {
        return Ok(None);
    };
    let Some(movement) = battlefield_move_mut(last) else {
        return Ok(None);
    };
    let plain = movement.clone();
    let mut modified = plain.clone();
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                battlefield_tapped,
                battlefield_attacking,
                ..
            }),
        ..
    }) = &mut modified
    {
        *battlefield_tapped = true;
        *battlefield_attacking |= attacking;
    }
    *movement = EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate,
        if_true: vec![modified],
        if_false: vec![plain],
    });
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

pub(super) fn pre_rule_elliptical_condition_fallback(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some((intro, body)) = crate::split_lexed_once_on_comma(sentence_tokens) else {
        return Ok(None);
    };
    let words = crate::lexer::parser_token_word_refs(intro);
    if !matches!(words.as_slice(), ["if", "it", "doesn't" | "doesnt"] | ["if", "it", "does", "not"]) {
        return Ok(None);
    }
    // The omitted predicate must have a preceding state condition. It does
    // not mean that a preceding draw or counter-placement action failed.
    if !state.effects.last().is_some_and(|effect| match effect {
        EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { .. }
            | ConditionalEffectAst::Conditional { .. }) => true,
        EffectAst::ControlFlow(flow) => matches!(flow.node,
            crate::model::ControlFlowNodeAst::Condition { .. }),
        _ => false,
    }) {
        return Ok(None);
    }
    let mut plan = SentenceParsePlan::new(body.to_vec());
    plan.wrap_if_result = Some(IfResultPredicate::Otherwise);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

pub(super) fn pre_rule_otherwise_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(without_otherwise) = strip_otherwise_sentence_prefix(sentence_tokens) else {
        return Ok(None);
    };
    let mut plan = SentenceParsePlan::new(rewrite_otherwise_referential_subject(without_otherwise));
    plan.wrap_if_result = Some(IfResultPredicate::Otherwise);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

pub(super) fn is_destroy_those_creatures_sentence(tokens: &[OwnedLexToken]) -> bool {
    followup_shapes::is_destroy_those_creatures_followup(tokens)
}

pub(super) fn pre_rule_destroy_those_creatures_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if !is_destroy_those_creatures_sentence(sentence_tokens) {
        return Ok(None);
    }
    let Some(filter) = last_remove_abilities_all_filter(state.effects) else {
        return Ok(None);
    };
    state
        .effects
        .push(EffectAst::subject_verb_destroy_all(filter));
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

/// Retain an authored label inside an exact numeric-result row.
///
/// The document grammar keeps `N | ...` rows attached to the roll instruction,
/// while the ordinary statement-label parser intentionally strips a label such
/// as `Trapped! —` before parsing its executable body. Reattach that label only
/// when both pieces are still proven here: the outer typed numeric predicate and
/// the inner label/body split from the same source sentence.
pub(super) fn post_rule_numeric_result_branch_label(
    _state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let Some(prefix) =
        crate::grammar::structure::split_leading_result_prefix_lexed(sentence_tokens)
    else {
        return Ok(None);
    };
    let IfResultPredicate::DieValue(_) = &prefix.predicate else {
        return Ok(None);
    };
    let authored_tokens = sentences
        .get(sentence_idx)
        .map(SentenceInput::lexed)
        .unwrap_or(sentence_tokens);
    let Some(authored_prefix) =
        crate::grammar::structure::split_leading_result_prefix_lexed(authored_tokens)
    else {
        return Ok(None);
    };
    let Some(label_split) = crate::grammar::document_shapes::parse_statement_label_split_tokens(
        authored_prefix.trailing_tokens,
    ) else {
        return Ok(None);
    };
    let label = crate::lexer::render_token_slice(label_split.label_tokens)
        .trim()
        .to_string();
    if label.is_empty() {
        return Ok(None);
    }
    let [EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate, effects })] =
        sentence_effects.as_mut_slice()
    else {
        return Ok(None);
    };
    if predicate != &prefix.predicate
        || effects.is_empty()
        || matches!(effects.as_slice(), [EffectAst::ResultBranchLabel { .. }])
    {
        return Ok(None);
    }
    let nested = std::mem::take(effects);
    effects.push(EffectAst::ResultBranchLabel {
        label,
        effects: nested,
    });
    // This is a local annotation of the current sentence, not a follow-up
    // consumed into an earlier effect. Let ordinary dispatch append it.
    Ok(Some(PostParseFollowupResult::Annotated))
}

/// "You may pay {X}. If you do, draw X cards. X can't be greater than the
/// amount of life you gained this turn." (Shanna, Purifying Blade): the
/// trailing sentence bounds the X the paying player announces for the
/// preceding `{X}` payment (CR 107.3a: the player chooses X). It is the
/// typed inclusive maximum of that payment, not a separate instruction.
pub(super) fn pre_rule_x_maximum_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(sentence_tokens);
    let [x, negation, be, greater, than, value_tokens @ ..] = tokens else {
        return Ok(None);
    };
    if !x.is_word("x")
        || !(negation.is_word("can't") || negation.is_word("cant") || negation.is_word("cannot"))
        || !be.is_word("be")
        || !greater.is_word("greater")
        || !than.is_word("than")
        || value_tokens.is_empty()
    {
        return Ok(None);
    }
    let Some((maximum, used)) = crate::util::parse_value(value_tokens) else {
        return Ok(None);
    };
    if used != value_tokens.len() {
        return Ok(None);
    }
    let Some(slot) = last_unbounded_x_payment_maximum_mut(state.effects) else {
        return Ok(None);
    };
    *slot = Some(maximum);
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

fn is_unbounded_x_payment(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Mana(ManaActionAst::PayMana {
                cost,
                x_value: None,
                x_maximum: None,
                independent_x_choice: false,
                mana_cost_of: None,
            }),
            ..
        }) if cost.has_x()
    )
}

fn last_unbounded_x_payment_maximum_mut(
    effects: &mut [EffectAst],
) -> Option<&mut Option<crate::effect::Value>> {
    for effect in effects.iter_mut().rev() {
        if is_unbounded_x_payment(effect) {
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Mana(ManaActionAst::PayMana { x_maximum, .. }),
                ..
            }) = effect
            else {
                return None;
            };
            return Some(x_maximum);
        }
        if let Some(children) = token_copy_followup_container_effects_mut(effect)
            && let Some(found) = last_unbounded_x_payment_maximum_mut(children)
        {
            return Some(found);
        }
    }
    None
}
