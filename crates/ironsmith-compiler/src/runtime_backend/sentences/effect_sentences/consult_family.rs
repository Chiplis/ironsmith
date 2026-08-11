use winnow::Parser as _;
use winnow::combinator::{alt, cut_err, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::token::take_till;

use super::super::grammar::primitives as grammar;
use super::super::lexer::{LexStream, OwnedLexToken};
use super::super::util::{helper_tag_for_tokens, parse_subject, trim_commas};
use super::dispatch_entry::{
    ConsultCastClause, ConsultCastCost, ConsultCastManaValueCondition, ConsultCastTiming,
    ConsultSentenceParts, parse_looked_card_reveal_filter,
};
use super::search_library::normalize_search_library_filter;
use super::{find_verb, parse_effect_chain, parse_effect_sentence_lexed};
use crate::cards::builders::{
    CardTextError, EffectAst, LibraryBottomOrderAst, LibraryConsultModeAst, ObjectFilter,
    PlayerAst, PredicateAst, SubjectAst, TagKey, TargetAst,
};
use crate::effect::Value;
use crate::runtime_backend::front_end::grammar::effects as effect_grammar;
use crate::target::TaggedOpbjectRelation;
use crate::zone::Zone;

pub(crate) fn parse_exile_top_library_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let count = super::dispatch_entry::parse_top_of_your_library_count(
        tokens,
        effect_grammar::dispatch_entry_shapes::TopLibraryAction::Exile,
    )?;

    Some(vec![EffectAst::subject_verb_exile_top_of_library(
        PlayerAst::You,
        Value::Fixed(count as i32),
        Vec::new(),
        Vec::new(),
    )])
}

pub(crate) fn parse_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<ConsultSentenceParts>, CardTextError> {
    let Some(shape) = effect_grammar::parse_consult_traversal_shape(tokens) else {
        return Ok(None);
    };
    let prefix_tokens = shape.prefix.unwrap_or_default();
    let mut prefix_effects = if prefix_tokens.is_empty() {
        Vec::new()
    } else {
        let effects = parse_exile_top_library_prefix(&prefix_tokens)
            .or_else(|| parse_effect_sentence_lexed(&prefix_tokens).ok())
            .or_else(|| parse_effect_chain(&prefix_tokens).ok())
            .unwrap_or_default();
        if effects.is_empty() {
            return Ok(None);
        }
        effects
    };
    let inferred_prefix_player = infer_consult_player_from_prefix(&prefix_tokens);
    let player = match shape.player {
        effect_grammar::ConsultTraversalPlayerShape::ImpliedByPrefixOrYou => {
            inferred_prefix_player.unwrap_or(PlayerAst::You)
        }
        effect_grammar::ConsultTraversalPlayerShape::ThatPlayer => PlayerAst::That,
        effect_grammar::ConsultTraversalPlayerShape::Subject(subject) => {
            match parse_subject(&subject) {
                SubjectAst::Player(player) => player,
                _ => return Ok(None),
            }
        }
    };
    if let Some(prefix_player) = inferred_prefix_player {
        apply_consult_prefix_player_surface(&mut prefix_effects, prefix_player);
    }
    let mode = shape.mode;
    let where_x = shape.where_x;
    let effect_grammar::ConsultTraversalStopShape {
        mut stop_rule,
        max_exposed,
        filter: filter_tokens,
        kind: stop_kind,
    } = shape.stop;
    if matches!(
        stop_rule,
        crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(Value::X)
    ) && let Some(value) = where_x
    {
        stop_rule = crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs),
        );
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else if matches!(stop_kind, effect_grammar::ConsultTraversalStopKind::Passive)
        && filter_tokens.is_empty()
    {
        ObjectFilter::default()
    } else {
        match super::super::object_filters::parse_object_filter(&filter_tokens, false) {
            Ok(filter) => filter,
            Err(_) => return Ok(None),
        }
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;
    bind_consult_it_relation_to_prefix_affected_object(tokens, &mut prefix_effects, &mut filter);

    let all_tag = helper_tag_for_tokens(
        tokens,
        match mode {
            LibraryConsultModeAst::Reveal => "revealed",
            LibraryConsultModeAst::Exile => "exiled",
        },
    );
    let match_tag = helper_tag_for_tokens(tokens, "consult_match");
    let mut effects = prefix_effects;
    effects.push(if let Some(max_exposed) = max_exposed {
        EffectAst::subject_verb_consult_top_of_library_with_max_exposed(
            player,
            mode,
            filter,
            stop_rule,
            max_exposed,
            all_tag.clone(),
            match_tag.clone(),
        )
    } else {
        EffectAst::subject_verb_consult_top_of_library(
            player,
            mode,
            filter,
            stop_rule,
            all_tag.clone(),
            match_tag.clone(),
        )
    });
    Ok(Some(ConsultSentenceParts {
        effects,
        player,
        all_tag,
        match_tag,
    }))
}

/// Parse a consult traversal whose comma-delimited tail continues with one or
/// more actions in the same sentence.
///
/// The traversal grammar deliberately separates the stop condition from that
/// tail. Keep the narrow `parse_consult_traversal_sentence` API focused on the
/// traversal itself for callers that compose specialized dispositions, while
/// this entry point preserves a generic authored continuation such as damage
/// followed by moving the revealed collection.
pub(crate) fn parse_consult_traversal_with_inline_followup(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_consult_traversal_shape(tokens) else {
        return Ok(None);
    };
    if shape.trailing_effect.is_empty() {
        return Ok(None);
    }
    let each_opponent = matches!(
        &shape.player,
        effect_grammar::ConsultTraversalPlayerShape::Subject(subject)
            if crate::runtime_backend::front_end::lexer::parser_token_word_refs(subject).as_slice()
                == ["each", "opponent"]
    );
    let Some(parts) = parse_consult_traversal_sentence(tokens)? else {
        return Ok(None);
    };
    let mut trailing = parse_effect_chain(&shape.trailing_effect)?;
    if trailing.is_empty() {
        return Ok(None);
    }
    let mut effects = parts.effects;
    effects.append(&mut trailing);
    if each_opponent {
        for effect in &mut effects {
            super::chain_carry::bind_implicit_player_context(effect, PlayerAst::That);
            if let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                subject,
                action:
                    crate::cards::builders::SubjectVerbActionAst::ConsultTopOfLibrary { player, .. },
            }) = effect
            {
                // The consult action carries its library owner separately
                // from the subject surface. Both must point at the active
                // loop participant; changing only the subject leaves the
                // executable consult bound to the broad Opponent filter.
                subject.player = PlayerAst::That;
                *player = PlayerAst::That;
            }
        }
        effects = vec![EffectAst::ForEachOpponent { effects }];
    }
    Ok(Some(effects))
}

/// In a sentence such as “that player exiles it, then exiles cards … until
/// they exile a card that shares a card type with it,” both occurrences of
/// “it” name the object affected by the prefix. Keep that antecedent stable
/// across the prefix's zone change instead of letting the consult's generic
/// prior-result resolution rebind it to a broader source-exiled collection.
fn bind_consult_it_relation_to_prefix_affected_object(
    tokens: &[OwnedLexToken],
    prefix_effects: &mut [EffectAst],
    filter: &mut ObjectFilter,
) {
    let relation_uses_it = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::cards::builders::IT_TAG
            && matches!(
                constraint.relation,
                TaggedOpbjectRelation::SharesCardType
                    | TaggedOpbjectRelation::SharesPermanentType
                    | TaggedOpbjectRelation::SharesSubtypeWithTagged
                    | TaggedOpbjectRelation::SharesColorWithTagged
                    | TaggedOpbjectRelation::SameManaValueAsTagged
                    | TaggedOpbjectRelation::SameNameAsTagged
            )
    });
    if !relation_uses_it {
        return;
    }

    let Some(prefix_effect) = prefix_effects.last_mut() else {
        return;
    };
    if !matches!(
        prefix_effect,
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: crate::cards::builders::SubjectVerbActionAst::Exile { .. }
                | crate::cards::builders::SubjectVerbActionAst::MoveToZone { .. },
            ..
        })
    ) {
        return;
    }

    let antecedent_tag = helper_tag_for_tokens(tokens, "consult_antecedent");
    let affected = prefix_effect.clone();
    *prefix_effect = EffectAst::TagAffected {
        effect: Box::new(affected),
        tag: antecedent_tag.clone(),
    };
    for constraint in &mut filter.tagged_constraints {
        if constraint.tag.as_str() == crate::cards::builders::IT_TAG {
            constraint.tag = antecedent_tag.clone();
        }
    }
}

fn infer_consult_player_from_prefix(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let prefix_tokens = trim_commas(tokens);
    let (_, verb_idx) = find_verb(&prefix_tokens)?;
    match parse_subject(&prefix_tokens[..verb_idx]) {
        SubjectAst::Player(player) => Some(player),
        _ => None,
    }
}

/// Preserve the authored player subject on a zone-change prefix.
///
/// Standalone tagged-object exile can leave its actor implicit because that
/// does not change the zone transition. A consult sentence immediately
/// reuses the same player for its traversal, so retaining the prefix actor is
/// useful provenance and renders the authored “that player exiles it”.
fn apply_consult_prefix_player_surface(effects: &mut [EffectAst], player: PlayerAst) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(subject_verb)
                if subject_verb.subject.player == PlayerAst::Implicit
                    && matches!(
                        &subject_verb.action,
                        crate::cards::builders::SubjectVerbActionAst::Exile { .. }
                            | crate::cards::builders::SubjectVerbActionAst::MoveToZone { .. }
                    ) =>
            {
                subject_verb.subject.player = player;
            }
            EffectAst::Sequence { effects } => {
                apply_consult_prefix_player_surface(effects, player);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
pub(crate) fn parse_consult_condition_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    effect_grammar::parse_consult_condition_value_shape(tokens)
}

fn take_remaining_clause_tokens<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    take_till(0.., |_token: &OwnedLexToken| false).parse_next(input)
}

fn parse_face_down_search_cast_mana_value_gate_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(crate::effect::ValueComparisonOperator, Value), ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "you" => (
            alt((
                grammar::phrase(&["you", "may", "cast", "the", "exiled", "card"]),
                grammar::phrase(&["you", "may", "cast", "that", "card"]),
                grammar::phrase(&["you", "may", "cast", "it"]),
            )),
            cut_err(grammar::phrase(&["without", "paying", "its", "mana", "cost"])),
            cut_err(|input: &mut LexStream<'a>| {
                let condition_tokens = take_remaining_clause_tokens(input)?;
                let condition = parse_consult_mana_value_condition_tokens(condition_tokens)
                    .ok_or_else(|| {
                        grammar::cut_err_ctx(
                            "mana value condition",
                            "supported mana value condition",
                        )
                    })?;
                Ok((condition.operator, condition.right))
            }),
        )
            .map(|(_, _, parsed)| parsed),
        _ => fail::<_, (crate::effect::ValueComparisonOperator, Value), _>,
    }
    .parse_next(input)
}

fn parse_bargained_face_down_cast_mana_value_gate_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(crate::effect::ValueComparisonOperator, Value), ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "if" => (
            grammar::phrase(&["if", "this", "spell", "was", "bargained"]),
            opt(grammar::comma()),
            cut_err(parse_face_down_search_cast_mana_value_gate_inner),
        )
            .map(|(_, _, parsed)| parsed),
        _ => fail::<_, (crate::effect::ValueComparisonOperator, Value), _>,
    }
    .parse_next(input)
}

pub(crate) fn parse_bargained_face_down_cast_mana_value_gate(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::ValueComparisonOperator, Value)>, CardTextError> {
    grammar::parse_all_or_none(
        tokens,
        parse_bargained_face_down_cast_mana_value_gate_inner,
        "bargained face-down cast clause",
    )
}

fn parse_if_you_dont_remainder_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "if" => (
            alt((
                grammar::phrase(&["if", "you", "dont"]),
                grammar::phrase(&["if", "you", "don't"]),
                grammar::phrase(&["if", "you", "do", "not"]),
            ))
            .context(StrContext::Label("if-you-don't prefix"))
            .context(StrContext::Expected(StrContextValue::Description(
                "if you don't",
            ))),
            cut_err(grammar::comma())
                .context(StrContext::Label("if-you-don't separator"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "comma after if-you-don't clause",
                ))),
            cut_err(take_remaining_clause_tokens),
        )
            .map(|(_, _, remainder)| remainder),
        _ => fail::<_, &'a [OwnedLexToken], _>,
    }
    .parse_next(input)
}

fn parse_if_you_cant_remainder_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "if" => (
            alt((
                grammar::phrase(&["if", "you", "cant"]),
                grammar::phrase(&["if", "you", "can't"]),
                grammar::phrase(&["if", "you", "cannot"]),
            ))
            .context(StrContext::Label("if-you-can't prefix"))
            .context(StrContext::Expected(StrContextValue::Description(
                "if you can't",
            ))),
            cut_err(grammar::comma())
                .context(StrContext::Label("if-you-can't separator"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "comma after if-you-can't clause",
                ))),
            cut_err(take_remaining_clause_tokens),
        )
            .map(|(_, _, remainder)| remainder),
        _ => fail::<_, &'a [OwnedLexToken], _>,
    }
    .parse_next(input)
}

pub(crate) fn parse_consult_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConsultCastManaValueCondition> {
    let shape = effect_grammar::parse_consult_mana_value_condition_shape(tokens)?;
    Some(ConsultCastManaValueCondition {
        operator: shape.operator,
        right: shape.right,
    })
}

pub(crate) fn parse_consult_cast_clause(tokens: &[OwnedLexToken]) -> Option<ConsultCastClause> {
    let shape = effect_grammar::parse_consult_cast_shape(tokens)?;
    let caster = match parse_subject(&shape.caster) {
        SubjectAst::Player(player) => player,
        _ => return None,
    };
    let timing = match shape.timing {
        effect_grammar::ConsultCastTimingShape::Immediate => ConsultCastTiming::Immediate,
        effect_grammar::ConsultCastTimingShape::UntilEndOfTurn => ConsultCastTiming::UntilEndOfTurn,
        effect_grammar::ConsultCastTimingShape::UntilYourNextTurnEnd => {
            ConsultCastTiming::UntilYourNextTurnEnd
        }
    };
    let cost = match shape.cost {
        effect_grammar::ConsultCastCostShape::Normal => ConsultCastCost::Normal,
        effect_grammar::ConsultCastCostShape::WithoutPayingManaCost => {
            ConsultCastCost::WithoutPayingManaCost
        }
        effect_grammar::ConsultCastCostShape::PayLifeEqualToManaValue => {
            ConsultCastCost::PayLifeEqualToManaValue
        }
    };
    Some(ConsultCastClause {
        caster,
        allow_land: shape.allow_land,
        timing,
        cost,
        surface: shape.surface,
        mana_value_condition: shape.mana_value_condition.map(|condition| {
            ConsultCastManaValueCondition {
                operator: condition.operator,
                right: condition.right,
            }
        }),
    })
}

pub(crate) fn parse_consult_bottom_remainder_clause(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<LibraryBottomOrderAst> {
    effect_grammar::parse_consult_bottom_remainder_shape(tokens, mode)
}

pub(crate) fn parse_if_declined_put_match_into_hand(
    tokens: &[OwnedLexToken],
    match_tag: TagKey,
) -> Option<Vec<EffectAst>> {
    if !effect_grammar::is_if_declined_put_match_into_hand_shape(tokens) {
        return None;
    }

    Some(vec![EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(match_tag, None),
        Zone::Hand,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    )])
}

pub(crate) fn consult_cast_effects(
    clause: &ConsultCastClause,
    match_tag: TagKey,
) -> Result<Vec<EffectAst>, CardTextError> {
    if clause.allow_land && !matches!(clause.cost, ConsultCastCost::Normal) {
        return Err(CardTextError::ParseError(
            "playing a land without paying its mana cost is unsupported".to_string(),
        ));
    }

    let mut cast_effects = match clause.cost {
        ConsultCastCost::Normal | ConsultCastCost::WithoutPayingManaCost => {
            let without_paying_mana_cost =
                matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost);
            if clause.allow_land
                || matches!(
                    clause.timing,
                    ConsultCastTiming::UntilEndOfTurn | ConsultCastTiming::UntilYourNextTurnEnd
                )
            {
                let grant = if matches!(clause.timing, ConsultCastTiming::UntilYourNextTurnEnd) {
                    EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                        match_tag.clone(),
                        clause.caster,
                        clause.allow_land,
                        false,
                    )
                } else {
                    EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
                        match_tag.clone(),
                        clause.caster,
                        clause.allow_land,
                        without_paying_mana_cost,
                        false,
                        Some(clause.surface.clone()),
                    )
                };
                vec![grant]
            } else {
                vec![EffectAst::MayByPlayer {
                    player: clause.caster,
                    effects: vec![EffectAst::subject_verb_cast_tagged(
                        match_tag.clone(),
                        clause.caster,
                        false,
                        false,
                        without_paying_mana_cost,
                        None,
                    )],
                }]
            }
        }
        ConsultCastCost::PayLifeEqualToManaValue => {
            if clause.allow_land {
                return Err(CardTextError::ParseError(
                    "pay-life consult cast clauses cannot allow lands".to_string(),
                ));
            }
            vec![
                EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
                    match_tag.clone(),
                    clause.caster,
                    false,
                    false,
                    false,
                    Some(clause.surface.clone()),
                ),
                EffectAst::subject_verb_grant_tagged_spell_alternative_cost_pay_life_by_mana_value_until_end_of_turn(match_tag.clone(), clause.caster),
            ]
        }
    };

    if let Some(condition) = &clause.mana_value_condition {
        cast_effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::ValueComparison {
                left: Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(match_tag))),
                operator: condition.operator,
                right: condition.right.clone(),
            },
            if_true: cast_effects,
            if_false: Vec::new(),
        }]
    }

    Ok(cast_effects)
}

pub(crate) fn parse_if_you_dont_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Some card text spells out the action whose failure is being tested:
    // "If you don't put the card into your hand, ...".  The compact form
    // above has a comma immediately after "don't", but both forms are the
    // same result-gated follow-up.  Consume the explicit action only as the
    // predicate surface; the clause after its comma is the effect to run.
    if let Some(after_explicit_action) = explicit_if_you_dont_action_remainder(tokens) {
        let effects = parse_effect_chain(after_explicit_action)?;
        if !effects.is_empty() {
            return Ok(Some(effects));
        }
    }
    if let Some(prefix_len) = if_you_dont_prefix_len(tokens)
        && tokens
            .get(prefix_len)
            .is_some_and(|token| !token.is_comma())
        && !is_explicit_failed_action_verb(tokens.get(prefix_len))
    {
        // `parse_if_you_dont_remainder_inner` deliberately cuts after its
        // prefix so malformed result-followups report a useful missing-comma
        // error. Do not enter that cut for an ordinary state predicate such
        // as "If you don't control a Faerie, ..."; the general conditional
        // parser owns that sentence.
        return Ok(None);
    }
    let Some(after) = grammar::parse_all_or_none(
        tokens,
        parse_if_you_dont_remainder_inner,
        "if-you-don't clause",
    )?
    else {
        return Ok(None);
    };
    let effects = parse_effect_chain(after)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

fn explicit_if_you_dont_action_remainder(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let prefix_len = if_you_dont_prefix_len(tokens)?;

    // The ordinary "If you don't, ..." form is intentionally left to the
    // cut-error parser above, including its missing-comma diagnostic.
    if tokens.get(prefix_len).is_some_and(OwnedLexToken::is_comma) {
        return None;
    }

    // This result-followup form names an action from the preceding sentence
    // ("don't cast/draw/put ... this way").  A state predicate beginning
    // with the same three words is an ordinary condition and must remain in
    // the conditional grammar ("don't control a Human"), not become failure
    // of the preceding effect. Keep this list to executable result-producing
    // verbs rather than accepting every token before the comma.
    if !is_explicit_failed_action_verb(tokens.get(prefix_len)) {
        return None;
    }

    let comma = tokens
        .iter()
        .enumerate()
        .skip(prefix_len)
        .find_map(|(idx, token)| token.is_comma().then_some(idx))?;
    (comma > prefix_len).then(|| &tokens[comma + 1..])
}

fn if_you_dont_prefix_len(tokens: &[OwnedLexToken]) -> Option<usize> {
    if tokens.first().is_some_and(|token| token.is_word("if"))
        && tokens.get(1).is_some_and(|token| token.is_word("you"))
        && tokens
            .get(2)
            .is_some_and(|token| token.is_word("don't") || token.is_word("dont"))
    {
        Some(3)
    } else if tokens.first().is_some_and(|token| token.is_word("if"))
        && tokens.get(1).is_some_and(|token| token.is_word("you"))
        && tokens.get(2).is_some_and(|token| token.is_word("do"))
        && tokens.get(3).is_some_and(|token| token.is_word("not"))
    {
        Some(4)
    } else {
        None
    }
}

fn is_explicit_failed_action_verb(token: Option<&OwnedLexToken>) -> bool {
    token.is_some_and(|token| {
        token.as_word().is_some_and(|word| {
            matches!(
                word,
                "cast" | "copy" | "draw" | "put" | "reveal" | "sacrifice"
            )
        })
    })
}

pub(crate) fn parse_if_you_cant_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(after) = grammar::parse_all_or_none(
        tokens,
        parse_if_you_cant_remainder_inner,
        "if-you-can't clause",
    )?
    else {
        return Ok(None);
    };

    let effects = parse_effect_chain(after)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Subtype;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn ordinary_negated_control_condition_is_not_a_failed_action_followup() {
        let tokens = lex_line(
            "If you don't control a Human, you lose life equal to that creature's toughness.",
            0,
        )
        .expect("ordinary condition should lex");

        assert_eq!(
            parse_if_you_dont_sentence(&tokens).expect("result-followup probe should not error"),
            None
        );
    }

    #[test]
    fn active_consult_preserves_explicit_repeated_card_union() {
        let tokens = lex_line(
            "Reveal cards from the top of your library until you reveal a Doctor card, a card with doctor's companion, or a Vehicle card",
            0,
        )
        .expect("consult sentence should lex");
        let parsed = parse_consult_traversal_sentence(&tokens)
            .expect("consult sentence should parse")
            .expect("consult traversal shape");
        let EffectAst::SubjectVerb(subject_verb) = &parsed.effects[0] else {
            panic!(
                "expected consult subject-verb effect: {:#?}",
                parsed.effects
            );
        };
        let crate::cards::builders::SubjectVerbActionAst::ConsultTopOfLibrary { filter, .. } =
            &subject_verb.action
        else {
            panic!("expected consult action: {subject_verb:#?}");
        };

        assert!(filter.has_explicit_union_branch_articles(), "{filter:#?}");
        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        assert_eq!(filter.any_of[0].subtypes, [Subtype::Doctor]);
        assert!(filter.any_of[1].subtypes.is_empty(), "{filter:#?}");
        assert_eq!(
            filter.any_of[1].ability_markers,
            ["doctor's companion".to_string()]
        );
        assert_eq!(filter.any_of[2].subtypes, [Subtype::Vehicle]);
        assert_eq!(
            filter.description(),
            "a Doctor card, a card with doctor's companion, or a Vehicle card"
        );
    }

    #[test]
    fn active_consult_matches_sacrificed_card_type_stop() {
        let tokens = lex_line(
            "they reveal cards from the top of their library until they reveal a permanent card that shares a card type with the sacrificed permanent, put that card onto the battlefield, then shuffle",
            0,
        )
        .expect("consult sentence should lex");
        let parsed = parse_consult_traversal_sentence(&tokens)
            .expect("consult sentence should parse")
            .expect("consult traversal shape");
        let EffectAst::SubjectVerb(subject_verb) = &parsed.effects[0] else {
            panic!(
                "expected consult subject-verb effect: {:#?}",
                parsed.effects
            );
        };
        let crate::cards::builders::SubjectVerbActionAst::ConsultTopOfLibrary { filter, .. } =
            &subject_verb.action
        else {
            panic!("expected consult action: {subject_verb:#?}");
        };
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "sacrificed_0"
                && constraint.relation == crate::target::TaggedOpbjectRelation::SharesCardType
        }));
    }

    #[test]
    fn prefixed_zone_change_keeps_shared_type_it_bound_to_affected_object() {
        let tokens = lex_line(
            "that player exiles it, then exiles cards from the top of their library until they exile a card that shares a card type with it",
            0,
        )
        .expect("consult sentence should lex");
        let parsed = parse_consult_traversal_sentence(&tokens)
            .expect("consult sentence should parse")
            .expect("consult traversal shape");
        let [
            EffectAst::TagAffected {
                tag: antecedent_tag,
                effect: prefix,
            },
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::ConsultTopOfLibrary { filter, .. },
                ..
            }),
        ] = parsed.effects.as_slice()
        else {
            panic!(
                "expected tagged prefix followed by consult: {:#?}",
                parsed.effects
            );
        };
        assert!(matches!(
            prefix.as_ref(),
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                subject: crate::cards::builders::SubjectVerbSubjectAst {
                    player: PlayerAst::That,
                    ..
                },
                ..
            })
        ));

        assert!(
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *antecedent_tag
                    && constraint.relation == TaggedOpbjectRelation::SharesCardType
            }),
            "expected consult relation to use the prefix's stable affected-object tag: {filter:#?}"
        );
    }

    #[test]
    fn immediate_consult_cast_keeps_the_authored_player_as_decider() {
        let tokens = lex_line(
            "That player may cast that card without paying its mana cost.",
            0,
        )
        .expect("consult cast clause should lex");
        let clause = parse_consult_cast_clause(&tokens).expect("consult cast clause should parse");
        let effects = consult_cast_effects(&clause, TagKey::from("consult_match"))
            .expect("consult cast clause should lower");

        assert!(matches!(
            effects.as_slice(),
            [EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects,
            }] if matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: crate::cards::builders::SubjectVerbActionAst::CastTagged { .. },
                    ..
                })]
            )
        ));
    }

    #[test]
    fn each_opponent_inline_consult_binds_the_executable_library_owner() {
        let tokens = lex_line(
            "Each opponent reveals cards from the top of their library until they reveal a land card, then puts those cards into their graveyard.",
            0,
        )
        .expect("each-opponent consult should lex");
        let effects = parse_consult_traversal_with_inline_followup(&tokens)
            .expect("each-opponent consult should parse")
            .expect("each-opponent consult traversal");

        let [EffectAst::ForEachOpponent { effects }] = effects.as_slice() else {
            panic!("expected one each-opponent loop: {effects:#?}");
        };
        let Some(EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            subject,
            action: crate::cards::builders::SubjectVerbActionAst::ConsultTopOfLibrary { player, .. },
        })) = effects.first()
        else {
            panic!("expected consult as the first loop action: {effects:#?}");
        };
        assert_eq!(subject.player, PlayerAst::That);
        assert_eq!(*player, PlayerAst::That);
    }
}
