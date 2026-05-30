use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, IfResultPredicate, OwnedLexToken,
    PlayerAst, PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst, SubjectAst, TagKey,
    TargetAst, TextSpan, Verb,
};
use crate::effect::{EventValueSpec, Until, Value};
use crate::static_abilities::StaticAbilityId;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;
use crate::{ChoiceCount, Supertype};

use super::super::activation_and_restrictions::activation_restriction_clauses::starts_with_target_indicator;
use super::super::activation_and_restrictions::trigger_subject_filters::title_case_token_word;
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::split_trailing_if_clause_lexed;
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::{
    LexedClause, find_token_word_sequence, token_slice_at_is, token_slice_last_is,
    word_slice_at_is, word_slice_contains_word, word_slice_ends_with, word_slice_eq,
    word_slice_eq_any, word_slice_find_phrase_start, word_slice_find_word,
    word_slice_find_word_where, word_slice_first_is, word_slice_first_is_any, word_slice_last_is,
    word_slice_last_is_any, word_slice_starts_with, word_slice_starts_with_at,
};
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::{
    find_index as find_token_index, rfind_index as find_token_index_rev,
};
use super::super::util::{
    parse_card_type, parse_color, parse_counter_type_from_tokens, parse_counter_type_word,
    parse_number, parse_subject, parse_target_count_range_prefix, parse_target_phrase, parse_value,
    span_from_tokens, strip_leading_article_word_refs, token_index_for_word_index, trim_commas,
    wrap_target_count,
};
use super::chain_carry::find_verb;
use super::parse_subtype_word;
use super::subject_verb_primitives::{
    SubjectVerbPrimitiveClause, parse_distribute_counters_sentence,
};
use super::verb_dispatch::parse_effect_with_verb;

type ClausePatternCompatWords<'a> = TokenWordView<'a>;

const ODD_EVEN_RESULT_PREFIXES: &[&[&str]] = &[
    &["for", "each", "odd", "result"],
    &["for", "each", "even", "result"],
];

const ODD_RESULT_VALUES_D6: &[i32] = &[1, 3, 5];
const EVEN_RESULT_VALUES_D6: &[i32] = &[2, 4, 6];
const OPEN_ATTRACTION_PREFIXES: &[&[&str]] = &[
    &["open", "an", "attraction"],
    &["opens", "an", "attraction"],
];

fn strip_suffix_char<'a>(word: &'a str, suffix: char) -> Option<&'a str> {
    crate::string_primitives::strip_suffix_char(word, suffix)
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    crate::slice_primitives::push_unique(items, item);
}

pub(crate) fn extract_subject_player(subject: Option<SubjectAst>) -> Option<PlayerAst> {
    match subject {
        Some(SubjectAst::Player(player)) => Some(player),
        _ => None,
    }
}

pub(crate) fn parse_prevent_next_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let clause_text = clause.text();
    if !word_slice_first_is(&clause_words, "prevent") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if word_slice_at_is(&clause_words, idx, "the") {
        idx += 1;
    }
    if !word_slice_at_is(&clause_words, idx, "next") {
        return Ok(None);
    }
    idx += 1;

    let amount_token = OwnedLexToken::word(
        clause_words
            .get(idx)
            .copied()
            .unwrap_or_default()
            .to_string(),
        TextSpan::synthetic(),
    );
    let Some((amount, amount_used)) = parse_value(&[amount_token]) else {
        return Err(CardTextError::ParseError(format!(
            "missing prevent damage amount (clause: '{}')",
            clause_text
        )));
    };
    idx += amount_used;

    if !word_slice_at_is(&clause_words, idx, "damage") {
        return Ok(None);
    }
    idx += 1;

    if !word_slice_starts_with_at(&clause_words, idx, &["that", "would", "be", "dealt"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage clause tail (clause: '{}')",
            clause_text
        )));
    }
    idx += 4;

    if !word_slice_at_is(&clause_words, idx, "to") {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage target scope (clause: '{}')",
            clause_text
        )));
    }
    idx += 1;

    let this_turn_rel = word_slice_find_phrase_start(&clause_words[idx..], &["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported prevent-next damage duration (clause: '{}')",
                clause_text
            ))
        })?;
    let this_turn_idx = idx + this_turn_rel;
    let source_of_your_choice = if this_turn_idx + 2 == clause_words.len() {
        false
    } else if word_slice_eq(
        &clause_words[this_turn_idx + 2..],
        &["by", "a", "source", "of", "your", "choice"],
    ) {
        true
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing prevent-next damage clause (clause: '{}')",
            clause_text
        )));
    };

    let target_clause = clause.between_words_trimmed(idx, this_turn_idx);
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-next damage target (clause: '{}')",
            clause_text
        )));
    }
    let target = parse_target_phrase(target_clause.tokens())?;

    Ok(Some(
        EffectAst::subject_verb_prevent_damage_with_source_choice(
            amount,
            target,
            Until::EndOfTurn,
            source_of_your_choice,
        ),
    ))
}

pub(crate) fn parse_double_counters_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    if !clause.starts_with(&["double", "the", "number", "of"]) {
        return Ok(None);
    }

    let counters_idx = find_token_index(tokens, |token| {
        token.is_word("counter") || token.is_word("counters")
    })
    .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counters keyword (clause: '{}')",
            clause_text
        ))
    })?;
    if counters_idx <= 4 {
        return Err(CardTextError::ParseError(format!(
            "missing counter type (clause: '{}')",
            clause_text
        )));
    }

    let counter_tokens = &tokens[4..counters_idx];
    let counter_type = parse_counter_type_from_tokens(counter_tokens)
        .or_else(|| {
            if counter_tokens.len() == 1 {
                counter_tokens[0]
                    .as_word()
                    .and_then(parse_counter_type_word)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported counter type in double-counters clause (clause: '{}')",
                clause_text
            ))
        })?;

    let on_idx = find_token_index(&tokens[counters_idx + 1..], |token| token.is_word("on"))
        .map(|offset| counters_idx + 1 + offset)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing 'on' in double-counters clause (clause: '{}')",
                clause_text
            ))
        })?;

    let mut filter_clause = clause.from(on_idx + 1).trimmed();
    if filter_clause.first_is_any_word(&["each", "all"]) {
        filter_clause = filter_clause.from(1).trimmed();
    }
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing filter in double-counters clause (clause: '{}')",
            clause_text
        )));
    }

    let filter = parse_object_filter(filter_clause.tokens(), false)?;
    Ok(Some(EffectAst::subject_verb_double_counters_on_each(
        counter_type,
        filter,
    )))
}

pub(crate) fn parse_distribute_counters_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_distribute_counters_sentence(SubjectVerbPrimitiveClause::new(tokens))
}

pub(crate) fn parse_verb_first_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(word) = tokens.first().and_then(OwnedLexToken::as_word) else {
        return Ok(None);
    };

    let verb = match word {
        "add" => Verb::Add,
        "move" => Verb::Move,
        "counter" => Verb::Counter,
        "destroy" => Verb::Destroy,
        "exile" => Verb::Exile,
        "draw" => Verb::Draw,
        "deal" => Verb::Deal,
        "sacrifice" => Verb::Sacrifice,
        "create" => Verb::Create,
        "investigate" => Verb::Investigate,
        "proliferate" => Verb::Proliferate,
        "tap" => Verb::Tap,
        "attach" => Verb::Attach,
        "untap" => Verb::Untap,
        "scry" => Verb::Scry,
        "discard" => Verb::Discard,
        "transform" => Verb::Transform,
        "convert" => Verb::Convert,
        "regenerate" => Verb::Regenerate,
        "mill" => Verb::Mill,
        "get" => Verb::Get,
        "remove" => Verb::Remove,
        "return" => Verb::Return,
        "exchange" => Verb::Exchange,
        "become" => Verb::Become,
        "skip" => Verb::Skip,
        "surveil" => Verb::Surveil,
        "incubate" => Verb::Incubate,
        "shuffle" => Verb::Shuffle,
        "pay" => Verb::Pay,
        "detain" => Verb::Detain,
        "goad" => Verb::Goad,
        "suspect" => Verb::Suspect,
        "look" => Verb::Look,
        "end" => Verb::End,
        _ => return Ok(None),
    };

    let effect = parse_effect_with_verb(verb, None, &tokens[1..])?;
    Ok(Some(effect))
}

pub(crate) fn is_simple_chosen_object_reference(tokens: &[OwnedLexToken]) -> bool {
    let raw_words = LexedClause::new(tokens).word_refs();
    let words = super::super::util::non_article_word_refs_except(&raw_words, &["then"]);
    if words.is_empty() {
        return false;
    }
    if word_slice_eq_any(&words, &[&["it"], &["them"]]) {
        return true;
    }
    if super::for_each_helpers::has_demonstrative_object_reference(&words) {
        return true;
    }
    false
}

pub(crate) fn parse_choose_target_and_verb_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    if !clause.starts_with(&["choose", "target"]) {
        return Ok(None);
    }

    let Some((before_and, after_and)) = grammar::split_lexed_once_on_separator(tokens, || {
        use winnow::Parser as _;
        grammar::kw("and").void()
    }) else {
        return Ok(None);
    };

    let target_clause = LexedClause::new(&before_and[1..]).trimmed();
    let target_tokens = target_clause.tokens();
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target after choose clause (clause: '{}')",
            clause_text
        )));
    }
    if find_verb(target_tokens).is_some() {
        return Ok(None);
    }

    let mut tail_clause = LexedClause::new(after_and).trimmed();
    if tail_clause.first_is_word("then") {
        tail_clause = tail_clause.from(1).trimmed();
    }
    if tail_clause.is_empty() {
        return Ok(None);
    }
    let tail_tokens = tail_clause.tokens();

    let Some((verb, verb_idx)) = find_verb(tail_tokens) else {
        return Ok(None);
    };
    if verb_idx != 0 {
        return Ok(None);
    }

    let rest_clause = tail_clause.from(1).trimmed();
    if !is_simple_chosen_object_reference(rest_clause.tokens()) {
        return Ok(None);
    }

    let effect = parse_effect_with_verb(verb, None, target_tokens)?;
    Ok(Some(effect))
}

pub(crate) fn parse_copy_spell_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    fn find_choose_new_targets_split_idx(tail: &[OwnedLexToken]) -> Option<usize> {
        for idx in 0..tail.len() {
            if !tail[idx].is_word("and") {
                continue;
            }
            let after = normalized_copy_retarget_tail_clause(&tail[idx + 1..], false);
            if after.first_word() == Some("choose")
                && after.contains_any_word(&["target", "targets"])
                && after.contains_word("copy")
            {
                return Some(idx);
            }
        }
        None
    }

    fn normalized_copy_retarget_tail_clause(
        tokens: &[OwnedLexToken],
        keep_may: bool,
    ) -> LexedClause<'_> {
        let mut clause = LexedClause::new(tokens).trimmed();
        if let Some(rest) = clause.strip_prefix_clause(&["you"]) {
            clause = rest.trimmed();
        } else if let Some(rest) = clause.strip_prefix_clause(&["that", "player"]) {
            clause = rest.trimmed();
        }
        if !keep_may && let Some(rest) = clause.strip_prefix_clause(&["may"]) {
            clause = rest.trimmed();
        }
        clause
    }

    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let clause_text = clause.text();
    let Some(copy_idx) = find_token_index(tokens, |token| {
        token.is_word("copy") || token.is_word("copies")
    }) else {
        return Ok(None);
    };
    let tail = &tokens[copy_idx + 1..];
    let split_idx = find_choose_new_targets_split_idx(tail);
    let exception_idx = word_slice_find_word(&clause_words, "except");
    let clause_words_before_exception = exception_idx
        .map(|idx| &clause_words[..idx])
        .unwrap_or(&clause_words);
    let simple_copy_reference = copy_idx == 0
        && (matches!(
            clause_words.get(1).copied(),
            Some("it") | Some("this") | Some("that")
        ) || word_slice_eq_any(
            &clause_words,
            &[
                &["copy", "that", "card"],
                &["copy", "the", "exiled", "card"],
            ],
        ));
    if simple_copy_reference {
        let trailing_if = split_trailing_if_clause_lexed(tokens);
        let copy_clause_tokens = trailing_if
            .as_ref()
            .map_or(tokens, |spec| spec.leading_tokens);
        let Some(copy_clause_copy_idx) = find_token_index(copy_clause_tokens, |token| {
            token.is_word("copy") || token.is_word("copies")
        }) else {
            return Ok(None);
        };
        let copy_clause_tail = &copy_clause_tokens[copy_clause_copy_idx + 1..];
        let copy_clause_split_idx = find_choose_new_targets_split_idx(copy_clause_tail);

        if let Some(then_idx) = find_token_index(copy_clause_tokens, |token| token.is_word("then"))
        {
            let tail_clause = LexedClause::new(&copy_clause_tokens[then_idx + 1..]).trimmed();
            if let Some(spec) =
                super::super::activation_and_restrictions::parse_may_cast_it_sentence(
                    tail_clause.tokens(),
                )
                && spec.as_copy
            {
                return Ok(Some(
                    super::super::activation_and_restrictions::build_may_cast_tagged_effect(&spec),
                ));
            }
        }
        let mut count = Value::Fixed(1);
        let copy_clause_exception_idx =
            find_token_index(copy_clause_tail, |token| token.is_word("except"));
        let copy_target_tail = if let Some(idx) = copy_clause_split_idx {
            &copy_clause_tail[..idx]
        } else if let Some(idx) = copy_clause_exception_idx {
            &copy_clause_tail[..idx]
        } else {
            copy_clause_tail
        };
        let (copy_target_tail, explicit_count) = strip_copy_count_suffix(copy_target_tail);
        if let Some(count_value) = explicit_count {
            count = count_value;
        }
        if let Some(for_each_idx) = find_token_word_sequence(copy_target_tail, &["for", "each"]) {
            let copy_target_clause = LexedClause::new(copy_target_tail);
            let count_filter_clause = copy_target_clause
                .after_words(for_each_idx + 2)
                .unwrap_or_else(|| copy_target_clause.from(copy_target_clause.len()))
                .trimmed();
            if count_filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing count filter after 'for each' in copy clause (clause: '{}')",
                    clause_text
                )));
            }
            let count_filter = parse_object_filter(count_filter_clause.tokens(), false)?;
            count = Value::Count(count_filter);
        }
        let target_words = LexedClause::new(copy_target_tail).word_refs();
        let target = if word_slice_eq(&target_words, &["this", "spell"]) {
            TargetAst::Source(None)
        } else if word_slice_eq(&target_words, &["that", "spell"]) {
            TargetAst::Tagged(TagKey::from("triggering"), None)
        } else if word_slice_eq(&target_words, &["that", "ability"]) {
            TargetAst::Tagged(TagKey::from("triggering_source"), None)
        } else if word_slice_eq_any(
            &target_words,
            &[
                &["it"],
                &["that"],
                &["that", "card"],
                &["the", "exiled", "card"],
            ],
        ) {
            TargetAst::Tagged(TagKey::from(IT_TAG), None)
        } else {
            TargetAst::Source(None)
        };
        let base = EffectAst::subject_verb_copy_spell(
            target,
            count,
            PlayerAst::Implicit,
            copy_clause_split_idx.is_some(),
            parse_copy_spell_removed_supertypes(copy_clause_tail),
        );
        if let Some(trailing_if) = trailing_if {
            return Ok(Some(EffectAst::Conditional {
                predicate: trailing_if.predicate,
                if_true: vec![base],
                if_false: Vec::new(),
            }));
        }
        return Ok(Some(base));
    }
    if !word_slice_contains_word(clause_words_before_exception, "spell")
        && !word_slice_contains_word(clause_words_before_exception, "spells")
        && !word_slice_contains_word(clause_words_before_exception, "ability")
        && !word_slice_contains_word(clause_words_before_exception, "abilities")
    {
        return Ok(None);
    }

    let subject = parse_subject(&tokens[..copy_idx]);
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
    };

    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_text
        )));
    }

    let mut count = Value::Fixed(1);
    let exception_split_idx = find_token_index(tail, |token| token.is_word("except"));
    let mut copy_target_tail = if let Some(idx) = split_idx {
        &tail[..idx]
    } else if let Some(idx) = exception_split_idx {
        &tail[..idx]
    } else {
        tail
    };
    let (stripped_copy_target_tail, explicit_count) = strip_copy_count_suffix(copy_target_tail);
    copy_target_tail = stripped_copy_target_tail;
    if let Some(count_value) = explicit_count {
        count = count_value;
    }
    if let Some(for_each_idx) = find_token_word_sequence(copy_target_tail, &["for", "each"]) {
        let copy_target_clause = LexedClause::new(copy_target_tail);
        let count_filter_clause = copy_target_clause
            .after_words(for_each_idx + 2)
            .unwrap_or_else(|| copy_target_clause.from(copy_target_clause.len()))
            .trimmed();
        if count_filter_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing count filter after 'for each' in copy clause (clause: '{}')",
                clause_text
            )));
        }
        let count_filter = parse_object_filter(count_filter_clause.tokens(), false)?;
        count = Value::Count(count_filter);
        copy_target_tail = &copy_target_tail[..for_each_idx];
    }

    let copy_target_clause = LexedClause::new(copy_target_tail).trimmed();
    if copy_target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_text
        )));
    }

    let target_words = copy_target_clause.word_refs();
    let target = if word_slice_eq(&target_words, &["this", "spell"]) {
        TargetAst::Source(None)
    } else {
        parse_counter_target_phrase(copy_target_clause.tokens())?
    };

    let mut may_choose_new_targets = false;
    if let Some(idx) = split_idx {
        let raw_choose_clause = normalized_copy_retarget_tail_clause(&tail[idx + 1..], true);
        let choose_clause = if let Some(rest) = raw_choose_clause.strip_prefix_clause(&["may"]) {
            may_choose_new_targets = true;
            rest.trimmed()
        } else {
            raw_choose_clause
        };
        let has_choose = choose_clause.first_word() == Some("choose");
        let has_new = choose_clause.contains_word("new");
        let has_target = choose_clause.contains_any_word(&["target", "targets"]);
        let has_copy = choose_clause.contains_word("copy");
        if !has_choose || !has_target || !has_copy {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing copy clause (clause: '{}')",
                clause_text
            )));
        }
        if !has_new {
            return Err(CardTextError::ParseError(format!(
                "missing 'new' in copy retarget clause (clause: '{}')",
                clause_text
            )));
        }
    }

    Ok(Some(EffectAst::subject_verb_copy_spell(
        target,
        count,
        player,
        may_choose_new_targets,
        parse_copy_spell_removed_supertypes(tail),
    )))
}

fn parse_copy_spell_removed_supertypes(tokens: &[OwnedLexToken]) -> Vec<crate::types::Supertype> {
    let clause = LexedClause::new(tokens);
    if clause.contains_word("legendary") && clause.contains_any_word(&["except", "isnt"]) {
        vec![crate::types::Supertype::Legendary]
    } else {
        Vec::new()
    }
}

fn strip_copy_count_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], Option<Value>) {
    if token_slice_last_is(tokens, "twice") {
        return (
            &tokens[..tokens.len().saturating_sub(1)],
            Some(Value::Fixed(2)),
        );
    }
    (tokens, None)
}

pub(crate) fn parse_counter_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if let Some(target) = parse_counter_ability_target_phrase(tokens)? {
        return Ok(target);
    }

    let clause = LexedClause::new(tokens);
    if clause.contains_word("ability") && clause.contains_any_word(&["activated", "triggered"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-ability target clause (clause: '{}')",
            clause.text()
        )));
    }

    parse_target_phrase(tokens)
}

fn parse_counter_ability_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let clause_tokens = LexedClause::new(tokens).trim();
    let clause = LexedClause::new(&clause_tokens);
    let is_you_control_tail = |idx: usize| {
        clause_tokens
            .get(idx)
            .is_some_and(|token| token.is_word("you"))
            && ((clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("control") || token.is_word("controls")))
                || (clause_tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("dont") || token.is_word("don't"))
                    && clause_tokens
                        .get(idx + 2)
                        .is_some_and(|token| token.is_word("control")))
                || (clause_tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("do"))
                    && clause_tokens
                        .get(idx + 2)
                        .is_some_and(|token| token.is_word("not"))
                    && clause_tokens
                        .get(idx + 3)
                        .is_some_and(|token| token.is_word("control"))))
    };
    if !clause.contains_word("ability") || clause.contains_no_words(&["activated", "triggered"]) {
        return Ok(None);
    }

    let mut idx = 0usize;
    let mut target_count: Option<ChoiceCount> = None;
    if clause_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("up"))
        && clause_tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("to"))
        && let Some((count, used)) = parse_number(&clause_tokens[idx + 2..])
    {
        target_count = Some(ChoiceCount::up_to(count as usize));
        idx += 2 + used;
    } else if let Some((count, used)) = parse_number(&clause_tokens[idx..])
        && clause_tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"))
    {
        target_count = Some(ChoiceCount::exactly(count as usize));
        idx += used;
    } else if let Some((count, used)) = parse_target_count_range_prefix(&clause_tokens[idx..])
        && clause_tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"))
    {
        target_count = Some(count);
        idx += used;
    }

    if !clause_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("target"))
    {
        return Ok(None);
    }
    idx += 1;

    #[derive(Clone, Copy)]
    enum CounterTargetTerm {
        Ability,
        Spell,
    }

    let mut list_end = clause_tokens.len();
    let mut scan = idx;
    while scan < clause_tokens.len() {
        if clause_tokens[scan].is_word("from") || is_you_control_tail(scan) {
            list_end = scan;
            break;
        }
        scan += 1;
    }

    // Parse counter target terms using winnow phrase matching on a sub-stream.
    use super::super::lexer::LexStream;
    use winnow::combinator::{alt, opt, repeat};
    use winnow::prelude::*;

    fn parse_counter_term<'a>(
        input: &mut LexStream<'a>,
    ) -> Result<
        Vec<(ObjectFilter, CounterTargetTerm)>,
        winnow::error::ErrMode<winnow::error::ContextError>,
    > {
        let make_triggered = || {
            let mut f = ObjectFilter::ability();
            f.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            f
        };

        alt((
            // "activated or triggered ability" / "triggered or activated ability"
            alt((
                grammar::phrase(&["activated", "or", "triggered", "ability"]),
                grammar::phrase(&["triggered", "or", "activated", "ability"]),
            ))
            .map(move |_| {
                vec![
                    (
                        ObjectFilter::activated_ability(),
                        CounterTargetTerm::Ability,
                    ),
                    (make_triggered(), CounterTargetTerm::Ability),
                ]
            }),
            grammar::phrase(&["activated", "ability"]).map(|_| {
                vec![(
                    ObjectFilter::activated_ability(),
                    CounterTargetTerm::Ability,
                )]
            }),
            grammar::phrase(&["triggered", "ability"]).map(move |_| {
                let mut f = ObjectFilter::ability();
                f.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
                vec![(f, CounterTargetTerm::Ability)]
            }),
            grammar::phrase(&["instant", "spell"]).map(|_| {
                vec![(
                    ObjectFilter::spell().with_type(crate::types::CardType::Instant),
                    CounterTargetTerm::Spell,
                )]
            }),
            grammar::phrase(&["sorcery", "spell"]).map(|_| {
                vec![(
                    ObjectFilter::spell().with_type(crate::types::CardType::Sorcery),
                    CounterTargetTerm::Spell,
                )]
            }),
            grammar::phrase(&["legendary", "spell"]).map(|_| {
                vec![(
                    ObjectFilter::spell().with_supertype(Supertype::Legendary),
                    CounterTargetTerm::Spell,
                )]
            }),
            grammar::phrase(&["noncreature", "spell"]).map(|_| {
                let mut f = ObjectFilter::noncreature_spell().in_zone(Zone::Stack);
                f.stack_kind = Some(crate::filter::StackObjectKind::Spell);
                vec![(f, CounterTargetTerm::Spell)]
            }),
            grammar::phrase(&["colorless", "spell"])
                .map(|_| vec![(ObjectFilter::spell().colorless(), CounterTargetTerm::Spell)]),
            grammar::kw("spell").map(|_| vec![(ObjectFilter::spell(), CounterTargetTerm::Spell)]),
        ))
        .parse_next(input)
    }

    let term_slice = &clause_tokens[idx..list_end];
    let mut stream = LexStream::new(term_slice);
    let mut term_filters: Vec<(ObjectFilter, CounterTargetTerm)> = Vec::new();

    type TermGroup = Vec<(ObjectFilter, CounterTargetTerm)>;
    let parsed_terms: Option<Vec<TermGroup>> = opt(|input: &mut LexStream<'_>| -> Result<Vec<TermGroup>, winnow::error::ErrMode<winnow::error::ContextError>> {
        let first = parse_counter_term.parse_next(input)?;
        let rest: Vec<TermGroup> = repeat(
            0..,
            (grammar::list_separator, parse_counter_term).map(|(_, t)| t),
        )
        .parse_next(input)?;
        let mut all = vec![first];
        all.extend(rest);
        Ok(all)
    })
    .parse_next(&mut stream)
    .unwrap_or(None);

    if let Some(groups) = parsed_terms {
        for group in groups {
            term_filters.extend(group);
        }
        idx += term_slice.len() - stream.len();
    } else {
        return Ok(None);
    }

    if term_filters.is_empty() {
        return Ok(None);
    }

    let mut source_types: Vec<crate::types::CardType> = Vec::new();
    let mut controller_filter: Option<crate::target::PlayerFilter> = None;
    while idx < clause_tokens.len() {
        let Some(word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word) else {
            idx += 1;
            continue;
        };
        if matches!(word, "and" | "or") {
            idx += 1;
            continue;
        }
        if word == "you"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("control") || token.is_word("controls"))
        {
            controller_filter = Some(crate::target::PlayerFilter::You);
            idx += 2;
            continue;
        }
        if word == "you"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("dont") || token.is_word("don't"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("control"))
        {
            controller_filter = Some(crate::target::PlayerFilter::NotYou);
            idx += 3;
            continue;
        }
        if word == "you"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("do"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("not"))
            && clause_tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("control"))
        {
            controller_filter = Some(crate::target::PlayerFilter::NotYou);
            idx += 4;
            continue;
        }
        if word == "from" {
            idx += 1;
            if clause_tokens
                .get(idx)
                .is_some_and(|token| matches!(token.as_word(), Some("a" | "an" | "the")))
            {
                idx += 1;
            }

            let mut parsed_type = false;
            while idx < clause_tokens.len() {
                let Some(type_word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word)
                else {
                    idx += 1;
                    continue;
                };
                if matches!(type_word, "source" | "sources") {
                    idx += 1;
                    break;
                }
                if matches!(type_word, "and" | "or") {
                    idx += 1;
                    continue;
                }
                let parsed = parse_card_type(type_word)
                    .or_else(|| strip_suffix_char(type_word, 's').and_then(parse_card_type));
                let Some(card_type) = parsed else {
                    return Ok(None);
                };
                source_types.push(card_type);
                parsed_type = true;
                idx += 1;
            }
            if !parsed_type {
                return Ok(None);
            }
            continue;
        }

        return Ok(None);
    }

    for (filter, term) in &mut term_filters {
        if let Some(controller) = controller_filter.clone() {
            let mut updated = filter.clone();
            updated.controller = Some(controller);
            *filter = updated;
        }
        if !source_types.is_empty() && matches!(term, CounterTargetTerm::Ability) {
            for card_type in &source_types {
                *filter = filter.clone().with_type(*card_type);
            }
        }
    }

    let target_filter = if term_filters.len() == 1 {
        term_filters
            .pop()
            .map(|(filter, _)| filter)
            .expect("single term filter should be present")
    } else {
        let mut any = ObjectFilter::default();
        any.any_of = term_filters.into_iter().map(|(filter, _)| filter).collect();
        any
    };

    let target = wrap_target_count(
        TargetAst::Object(target_filter, span_from_tokens(&clause_tokens), None),
        target_count,
    );
    Ok(Some(target))
}

pub(crate) fn parse_prevent_all_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let clause_text = clause.text();
    let prefix_target_then_duration = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
    ];
    let prefix_duration_then_target = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
    ];
    let prefix_duration_then_source = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "by",
    ];
    if !clause.starts_with(&prefix_target_then_duration)
        && !clause.starts_with(&prefix_duration_then_target)
        && !clause.starts_with(&prefix_duration_then_source)
    {
        return Ok(None);
    }
    if clause.starts_with(&prefix_duration_then_source) {
        let source_clause = clause
            .after_words(prefix_duration_then_source.len())
            .unwrap_or_else(|| clause.from(tokens.len()))
            .trimmed();
        if source_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing prevent-all damage source filter (clause: '{}')",
                clause_text
            )));
        }
        let source_filter_clause = if source_clause.ends_with(&["sources"]) {
            source_clause
                .strip_suffix_clause(&["sources"])
                .unwrap_or(source_clause)
                .trimmed()
        } else {
            source_clause
        };
        if source_filter_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported prevent-all damage source phrase (clause: '{}')",
                clause_text
            )));
        }
        let source_filter_target = parse_target_phrase(source_filter_clause.tokens())?;
        let TargetAst::Object(source_filter, _, _) = source_filter_target else {
            return Err(CardTextError::ParseError(format!(
                "unsupported prevent-all damage source filter target (clause: '{}')",
                clause_text
            )));
        };
        return Ok(Some(
            EffectAst::subject_verb_prevent_all_damage_from_source_filter(
                source_filter,
                Until::EndOfTurn,
            ),
        ));
    }
    let target_clause = if clause.starts_with(&prefix_duration_then_target) {
        clause
            .after_words(prefix_duration_then_target.len())
            .unwrap_or_else(|| clause.from(tokens.len()))
            .trimmed()
    } else {
        if clause_words.len() <= prefix_target_then_duration.len() + 1 {
            return Err(CardTextError::ParseError(format!(
                "missing prevent-all damage target (clause: '{}')",
                clause_text
            )));
        }
        if !word_slice_eq(
            &clause_words[clause_words.len().saturating_sub(2)..],
            &["this", "turn"],
        ) {
            return Err(CardTextError::ParseError(format!(
                "unsupported prevent-all damage duration (clause: '{}')",
                clause_text
            )));
        }
        clause.between_words_trimmed(prefix_target_then_duration.len(), clause_words.len() - 2)
    };
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all damage target (clause: '{}')",
            clause_text
        )));
    }

    let target = parse_target_phrase(target_clause.tokens())?;

    Ok(Some(EffectAst::subject_verb_prevent_all_damage_to_target(
        target,
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_can_attack_as_though_no_defender_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let Some(can_idx) = clause.find_word("can") else {
        return Ok(None);
    };
    let tail_clause = clause.after_words(can_idx).unwrap_or(clause).trimmed();
    let has_full_core = tail_clause.starts_with(&["can", "attack"])
        && tail_clause.contains_phrase(&["as", "though"])
        && tail_clause.contains_word("turn")
        && tail_clause.contains_word("have")
        && tail_clause.ends_with(&["defender"]);
    let has_split_core = tail_clause.starts_with(&["can", "attack"])
        && tail_clause.contains_phrase(&["as", "though"])
        && tail_clause.contains_word("turn")
        && tail_clause.ends_with_any(&[&["didnt"], &["didn't"]]);
    if !has_full_core && !has_split_core {
        return Ok(None);
    }

    let subject_clause = clause
        .before_word(can_idx)
        .unwrap_or(clause.before(0))
        .trimmed();
    let target = if subject_clause.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        parse_target_phrase(subject_clause.tokens())?
    };

    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        target,
        vec![GrantedAbilityAst::CanAttackAsThoughNoDefender],
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_prevent_next_time_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    if !clause.starts_with(&["the", "next", "time"]) {
        return Ok(None);
    }

    let Some(would_idx) = clause.find_word("would") else {
        return Ok(None);
    };
    let clause_words = clause.word_refs();
    if !word_slice_starts_with_at(&clause_words, would_idx + 1, &["deal", "damage", "to"]) {
        return Ok(None);
    }

    let this_turn_rel =
        word_slice_find_phrase_start(&clause_words[would_idx + 4..], &["this", "turn"])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported prevent-next-time damage duration (clause: '{}')",
                    clause_text
                ))
            })?;
    let this_turn_idx = (would_idx + 4) + this_turn_rel;

    let tail_clause = clause
        .after_words(this_turn_idx + 2)
        .unwrap_or_else(|| clause.from(clause.len()))
        .trimmed();
    let reflect_damage_to_source_controller =
        if tail_clause.matches_words(&["prevent", "that", "damage"]) {
            false
        } else if tail_clause.starts_with(&[
            "prevent",
            "that",
            "damage",
            "if",
            "damage",
            "is",
            "prevented",
            "this",
            "way",
        ]) && tail_clause.contains_phrase(&[
            "deals",
            "that",
            "much",
            "damage",
            "to",
            "that",
            "source's",
            "controller",
        ]) {
            true
        } else {
            return Ok(None);
        };

    let source_clause = clause.between_words_trimmed(3, would_idx);
    let source_words = source_clause.word_refs();
    if source_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-next-time damage source (clause: '{}')",
            clause_text
        )));
    }

    let source = if source_clause.contains_phrase(&["of", "your", "choice"]) {
        PreventNextTimeDamageSourceAst::Choice
    } else {
        let mut words = strip_leading_article_word_refs(&source_words).to_vec();
        if word_slice_last_is(&words, "source") {
            words.pop();
        }
        if words.is_empty() {
            let effect = if reflect_damage_to_source_controller {
                EffectAst::subject_verb_prevent_next_time_damage_with_reflection(
                    PreventNextTimeDamageSourceAst::Filter(ObjectFilter::default()),
                    PreventNextTimeDamageTargetAst::AnyTarget,
                    true,
                )
            } else {
                EffectAst::subject_verb_prevent_next_time_damage(
                    PreventNextTimeDamageSourceAst::Filter(ObjectFilter::default()),
                    PreventNextTimeDamageTargetAst::AnyTarget,
                )
            };
            return Ok(Some(vec![effect]));
        }

        let mut filter = ObjectFilter::default();
        let mut colors: Option<crate::color::ColorSet> = None;
        for w in words {
            if matches!(w, "or" | "and") {
                continue;
            }
            if let Some(color) = parse_color(w) {
                colors = Some(
                    colors
                        .unwrap_or_else(crate::color::ColorSet::new)
                        .union(color),
                );
                continue;
            }
            if let Some(card_type) = parse_card_type(w) {
                push_unique(&mut filter.card_types, card_type);
                continue;
            }
            if w == "shadow" {
                filter = filter.with_static_ability(StaticAbilityId::Shadow);
                continue;
            }
        }
        if let Some(colors) = colors {
            filter.colors = Some(colors);
        }

        PreventNextTimeDamageSourceAst::Filter(filter)
    };

    let target_clause = clause.between_words_trimmed(would_idx + 4, this_turn_idx);
    let target = if target_clause.matches_words(&["you"]) {
        PreventNextTimeDamageTargetAst::You
    } else if target_clause.matches_words(&["any", "target"]) {
        PreventNextTimeDamageTargetAst::AnyTarget
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next-time damage target scope (clause: '{}')",
            clause_text
        )));
    };

    let effect = if reflect_damage_to_source_controller {
        EffectAst::subject_verb_prevent_next_time_damage_with_reflection(source, target, true)
    } else {
        EffectAst::subject_verb_prevent_next_time_damage(source, target)
    };
    Ok(Some(vec![effect]))
}

pub(crate) fn parse_redirect_next_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let clause_text = clause.text();
    if word_slice_starts_with(
        &clause_words,
        &[
            "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
        ],
    ) {
        let target_start = 9usize;
        let is_dealt_rel =
            word_slice_find_phrase_start(&clause_words[target_start..], &["is", "dealt", "to"])
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported redirected-all-damage destination (clause: '{}')",
                        clause_text
                    ))
                })?;
        let is_dealt_idx = target_start + is_dealt_rel;
        let protected_words = &clause_words[target_start..is_dealt_idx];
        let object_filter = match protected_words {
            ["you", "and", "permanents", "you", "control"]
            | ["you", "and", "permanent", "you", "control"] => {
                ObjectFilter::permanent().you_control()
            }
            ["you", "and", "other", "permanents", "you", "control"]
            | ["you", "and", "other", "permanent", "you", "control"] => {
                ObjectFilter::permanent().you_control().other()
            }
            _ => return Ok(None),
        };

        let redirect_words = &clause_words[is_dealt_idx + 3..];
        if !word_slice_last_is(redirect_words, "instead") || redirect_words.len() < 2 {
            return Ok(None);
        }
        let target_tokens = redirect_words[..redirect_words.len() - 1]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let target = parse_target_phrase(&target_tokens)?;

        return Ok(Some(vec![
            EffectAst::subject_verb_redirect_all_damage_this_turn_to_target(
                PlayerFilter::You,
                object_filter,
                target,
            ),
        ]));
    }

    if clause.starts_with(&["all", "damage", "that", "would", "be", "dealt", "to"]) {
        let idx = 7usize;
        let this_turn_rel = LexedClause::new(
            clause
                .from_word(idx)
                .unwrap_or_else(|| clause.from(clause.len()))
                .tokens(),
        )
        .find_phrase_start(&["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported redirected-all-damage duration (clause: '{}')",
                clause_text
            ))
        })?;
        let this_turn_idx = idx + this_turn_rel;
        let target_clause = clause.between_words_trimmed(idx, this_turn_idx);
        if target_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-all-damage target (clause: '{}')",
                clause_text
            )));
        }

        let by_idx = this_turn_idx + 2;
        if !word_slice_at_is(&clause_words, by_idx, "by") {
            return Ok(None);
        }
        let is_dealt_rel = clause
            .from_word(by_idx + 1)
            .unwrap_or_else(|| clause.from(clause.len()))
            .find_phrase_start(&["is", "dealt", "to"])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported redirected-all-damage destination (clause: '{}')",
                    clause_text
                ))
            })?;
        let is_dealt_idx = by_idx + 1 + is_dealt_rel;

        let source_clause = clause.between_words_trimmed(by_idx + 1, is_dealt_idx);
        if source_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-all-damage source (clause: '{}')",
                clause_text
            )));
        }

        let source = if source_clause.contains_phrase(&["of", "your", "choice"]) {
            PreventNextTimeDamageSourceAst::Choice
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported redirected-all-damage source scope (clause: '{}')",
                clause_text
            )));
        };

        let redirect_clause = clause
            .after_words(is_dealt_idx + 3)
            .unwrap_or_else(|| clause.from(clause.len()))
            .trimmed();
        let redirects_to_source = redirect_clause.matches_any_words(&[
            &["this", "creature", "instead"],
            &["this", "permanent", "instead"],
            &["this", "instead"],
            &["it", "instead"],
        ]);
        if !redirects_to_source {
            return Err(CardTextError::ParseError(format!(
                "unsupported redirected-all-damage protected destination (clause: '{}')",
                clause_text
            )));
        }

        let target = parse_target_phrase(target_clause.tokens())?;

        return Ok(Some(vec![
            EffectAst::subject_verb_redirect_all_damage_this_turn_to_source(source, target),
        ]));
    }

    if clause.starts_with(&["the", "next", "time"]) {
        let Some(would_idx) = clause.find_word("would") else {
            return Ok(None);
        };
        if clause_words.get(would_idx + 1..would_idx + 4)
            != Some(["deal", "damage", "to"].as_slice())
        {
            return Ok(None);
        }

        let this_turn_rel = clause
            .from_word(would_idx + 4)
            .unwrap_or_else(|| clause.from(clause.len()))
            .find_phrase_start(&["this", "turn"])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported redirected-next-time damage duration (clause: '{}')",
                    clause_text
                ))
            })?;
        let this_turn_idx = (would_idx + 4) + this_turn_rel;

        let source_clause = clause.between_words_trimmed(3, would_idx);
        let source_words = source_clause.word_refs();
        if source_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-next-time damage source (clause: '{}')",
                clause_text
            )));
        }

        let source = if source_clause.contains_phrase(&["of", "your", "choice"]) {
            PreventNextTimeDamageSourceAst::Choice
        } else {
            let mut words = strip_leading_article_word_refs(&source_words).to_vec();
            if word_slice_last_is(&words, "source") {
                words.pop();
            }
            let mut filter = ObjectFilter::default();
            let mut colors: Option<crate::color::ColorSet> = None;
            for word in words {
                if matches!(word, "or" | "and") {
                    continue;
                }
                if let Some(color) = parse_color(word) {
                    colors = Some(
                        colors
                            .unwrap_or_else(crate::color::ColorSet::new)
                            .union(color),
                    );
                    continue;
                }
                if let Some(card_type) = parse_card_type(word) {
                    push_unique(&mut filter.card_types, card_type);
                    continue;
                }
                if word == "shadow" {
                    filter = filter.with_static_ability(StaticAbilityId::Shadow);
                    continue;
                }
            }
            if let Some(colors) = colors {
                filter.colors = Some(colors);
            }
            PreventNextTimeDamageSourceAst::Filter(filter)
        };

        let target_clause = clause.between_words_trimmed(would_idx + 4, this_turn_idx);
        if target_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-next-time damage target (clause: '{}')",
                clause_text
            )));
        }
        let target = parse_target_phrase(target_clause.tokens())?;

        let tail_clause = clause
            .after_words(this_turn_idx + 2)
            .unwrap_or_else(|| clause.from(clause.len()))
            .trimmed();
        if tail_clause.word_len() < 7
            || !tail_clause.starts_with(&["that", "damage", "is", "dealt", "to"])
            || !tail_clause.ends_with(&["instead"])
        {
            return Ok(None);
        }
        let redirect_clause = tail_clause.between_words_trimmed(5, tail_clause.word_len() - 1);
        let redirects_to_source = redirect_clause.matches_any_words(&[
            &["this"],
            &["it"],
            &["this", "creature"],
            &["this", "permanent"],
        ]);
        if !redirects_to_source {
            return Err(CardTextError::ParseError(format!(
                "unsupported redirected-next-time damage destination (clause: '{}')",
                clause_text
            )));
        }

        return Ok(Some(vec![
            EffectAst::subject_verb_redirect_next_time_damage_to_source(source, target),
        ]));
    }

    if !clause.starts_with(&["the", "next"]) {
        return Ok(None);
    }

    let Some(amount_token_idx) = clause.token_index_for_word_index(2) else {
        return Ok(None);
    };
    let amount_token = tokens[amount_token_idx].clone();
    let Some((amount, amount_used)) = parse_value(&[amount_token]) else {
        return Ok(None);
    };
    if amount_used != 1 {
        return Err(CardTextError::ParseError(format!(
            "unsupported redirected-next-damage amount (clause: '{}')",
            clause_text
        )));
    }

    let mut idx = 3usize;
    if !clause
        .words()
        .slice_eq(idx, &["damage", "that", "would", "be", "dealt", "to"])
    {
        return Ok(None);
    }
    idx += 6;

    let this_turn_rel = clause
        .from_word(idx)
        .unwrap_or_else(|| clause.from(clause.len()))
        .find_phrase_start(&["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported redirected-next-damage duration (clause: '{}')",
                clause_text
            ))
        })?;
    let this_turn_idx = idx + this_turn_rel;
    let protected_clause = clause.between_words_trimmed(idx, this_turn_idx);
    let protects_source = protected_clause.matches_any_words(&[
        &["this"],
        &["it"],
        &["this", "creature"],
        &["this", "permanent"],
    ]);
    if !protects_source {
        return Err(CardTextError::ParseError(format!(
            "unsupported redirected-next-damage protected target (clause: '{}')",
            clause_text
        )));
    }

    let tail_clause = clause
        .after_words(this_turn_idx + 2)
        .unwrap_or_else(|| clause.from(clause.len()))
        .trimmed();
    if tail_clause.word_len() < 5
        || !tail_clause.starts_with(&["is", "dealt", "to"])
        || !tail_clause.ends_with(&["instead"])
    {
        return Ok(None);
    }

    let target_clause = tail_clause.between_words_trimmed(3, tail_clause.word_len() - 1);
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing redirected-next-damage target (clause: '{}')",
            clause_text
        )));
    }
    let target = parse_target_phrase(target_clause.tokens())?;

    Ok(Some(vec![
        EffectAst::subject_verb_redirect_next_damage_from_source_to_target(amount, target),
    ]))
}

pub(crate) fn parse_can_block_additional_creature_this_turn_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let Some(can_idx) = clause.find_word("can") else {
        return Ok(None);
    };
    let tail_clause = clause.after_words(can_idx).unwrap_or(clause).trimmed();
    if !tail_clause.starts_with(&["can", "block"]) || !tail_clause.ends_with(&["this", "turn"]) {
        return Ok(None);
    }

    let Some(additional_offset) = tail_clause.find_word("additional") else {
        return Ok(None);
    };
    let tail_words = tail_clause.word_refs();
    if tail_words.get(additional_offset + 1).copied() != Some("creature")
        && tail_words.get(additional_offset + 1).copied() != Some("creatures")
    {
        return Ok(None);
    }

    let mut additional = 1usize;
    if additional_offset > 0 {
        let number_word_idx = can_idx + additional_offset - 1;
        if clause_words[number_word_idx] != "a"
            && clause_words[number_word_idx] != "an"
            && let Some(number_token_idx) = clause.token_index_for_word_index(number_word_idx)
            && let Some((parsed, used)) = parse_number(&tokens[number_token_idx..])
            && used > 0
        {
            additional = parsed as usize;
        }
    }

    let subject_clause = clause
        .before_word(can_idx)
        .unwrap_or(clause.before(0))
        .trimmed();
    let target = if subject_clause.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        parse_target_phrase(subject_clause.tokens())?
    };

    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        target,
        vec![GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat { additional }],
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_win_the_game_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if clause.word_len() < 4 || !clause.starts_with(&["you", "win", "the", "game"]) {
        return Ok(None);
    }

    if clause.word_len() == 4 {
        return Ok(Some(EffectAst::subject_verb_win_game(PlayerAst::You)));
    }

    if let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) {
        let leading_clause = LexedClause::new(trailing_if.leading_tokens);
        if word_slice_eq(&leading_clause.word_refs(), &["you", "win", "the", "game"]) {
            return Ok(Some(EffectAst::Conditional {
                predicate: trailing_if.predicate,
                if_true: vec![EffectAst::subject_verb_win_game(PlayerAst::You)],
                if_false: Vec::new(),
            }));
        }
    }

    let Some(if_tail_clause) = clause.after_words(4) else {
        return Ok(None);
    };
    if !if_tail_clause.first_is_word("if") {
        return Ok(None);
    }

    let if_tail_clause = if_tail_clause
        .after_words(1)
        .unwrap_or(if_tail_clause)
        .trimmed();
    let if_tail = if_tail_clause.word_refs();
    if if_tail.len() < 6
        || if_tail[0] != "you"
        || if_tail[1] != "own"
        || !matches!(if_tail[2], "a" | "an" | "the")
        || if_tail[3] != "card"
        || if_tail[4] != "named"
    {
        return Ok(None);
    }

    let after_named = &if_tail[5..];
    let Some(in_idx) = word_slice_find_word_where(after_named, |word| word == "in") else {
        return Ok(None);
    };
    if in_idx == 0 {
        return Ok(None);
    }

    let name_words = &after_named[..in_idx];
    let remainder_clause = if_tail_clause
        .after_words(5 + in_idx)
        .unwrap_or_else(|| if_tail_clause.from(if_tail_clause.len()))
        .trimmed();

    let has_exile = remainder_clause.contains_word("exile");
    let has_hand = remainder_clause.contains_word("hand");
    let has_graveyard = remainder_clause.contains_word("graveyard");
    let has_battlefield = remainder_clause.contains_word("battlefield");
    if !(has_exile && has_hand && has_graveyard && has_battlefield) {
        return Ok(None);
    }

    let name = name_words
        .iter()
        .map(|word| title_case_token_word(word))
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() {
        return Ok(None);
    }

    Ok(Some(EffectAst::Conditional {
        predicate: crate::cards::builders::PredicateAst::PlayerOwnsCardNamedInZones {
            player: PlayerAst::You,
            name,
            zones: vec![Zone::Exile, Zone::Hand, Zone::Graveyard, Zone::Battlefield],
        },
        if_true: vec![EffectAst::subject_verb_win_game(PlayerAst::You)],
        if_false: Vec::new(),
    }))
}

fn parse_choose_target_prelude_targets(
    target_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<TargetAst>>, CardTextError> {
    let Some((first, second)) = grammar::split_lexed_once_on_separator(target_tokens, || {
        use winnow::Parser as _;
        grammar::kw("and").void()
    }) else {
        return Ok(None);
    };
    let first = trim_commas(first);
    let second = trim_commas(second);
    if first.is_empty() || second.is_empty() || !starts_with_target_indicator(&second) {
        return Ok(None);
    }

    Ok(Some(vec![
        parse_target_phrase(&first)?,
        parse_target_phrase(&second)?,
    ]))
}

pub(crate) fn parse_choose_target_prelude_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if !clause.first_is_word("choose") {
        return Ok(None);
    }

    let target_clause = clause.from(1).trimmed();
    let target_tokens = target_clause.tokens();
    if target_clause.is_empty() || !starts_with_target_indicator(target_tokens) {
        return Ok(None);
    }
    if find_verb(target_tokens).is_some() {
        return Ok(None);
    }

    if let Some(targets) = parse_choose_target_prelude_targets(target_tokens)? {
        return Ok(Some(
            targets
                .into_iter()
                .map(EffectAst::subject_verb_target_only)
                .collect(),
        ));
    }

    let target = parse_target_phrase(target_tokens)?;
    Ok(Some(vec![EffectAst::subject_verb_target_only(target)]))
}

pub(crate) fn parse_keyword_mechanic_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut start = 0usize;
    if token_slice_at_is(tokens, start, "then") {
        start += 1;
    }
    if token_slice_at_is(tokens, start, "you") {
        start += 1;
    }
    if start >= tokens.len() {
        return Ok(None);
    }

    let clause = LexedClause::new(&tokens[start..]);
    let clause_tokens = clause.tokens();
    let clause_words = clause.word_refs();
    let clause_text = clause.text();
    if clause.is_empty() {
        return Ok(None);
    }

    if word_slice_first_is(&clause_words, "amass") {
        let mut amount_start = 1usize;
        let mut subtype = None;

        if let Some(candidate) = clause_words.get(amount_start).copied()
            && let Some(parsed_subtype) = parse_subtype_word(candidate)
                .or_else(|| strip_suffix_char(candidate, 's').and_then(parse_subtype_word))
            && parsed_subtype.is_creature_type()
        {
            subtype = Some(parsed_subtype);
            amount_start += 1;
        }

        let (mut amount, used) = parse_value(&clause_tokens[amount_start..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing numeric amount for amass clause (clause: '{}')",
                clause_text
            ))
        })?;
        let trailing_tokens = LexedClause::new(&clause_tokens[amount_start + used..]).trim();
        if !trailing_tokens.is_empty() {
            let Some(where_value) = parse_value_binding_clause(&trailing_tokens) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing amass clause (clause: '{}')",
                    clause_text
                )));
            };
            amount = super::super::util::replace_unbound_x_with_value(
                amount,
                &where_value,
                &crate::runtime_backend::token_word_refs(&trailing_tokens).join(" "),
            )?;
        }

        return Ok(Some(EffectAst::subject_verb_amass(subtype, amount)));
    }

    if word_slice_eq_any(&clause_words, &[&["forage"], &["forages"]]) {
        return Ok(Some(EffectAst::subject_verb_emit_keyword_action(
            crate::events::KeywordActionKind::Forage,
            1,
        )));
    }

    if clause.first_is_word("roll") && clause.contains_word("dice") {
        if word_slice_last_is(&clause_words, "dice")
            && clause_words.len() >= 5
            && word_slice_eq(
                &clause_words[clause_words.len() - 3..clause_words.len() - 1],
                &["six", "sided"],
            )
        {
            let value_clause = clause.between_words_trimmed(1, clause_words.len() - 3);
            let value_tokens = value_clause.tokens();
            let (count, used) = parse_value(value_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing roll-dice count (clause: '{}')",
                    clause_text
                ))
            })?;
            if used != value_tokens.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported roll-dice count tail (clause: '{}')",
                    clause_text
                )));
            }
            return Ok(Some(EffectAst::RepeatEffects {
                count,
                effects: vec![EffectAst::subject_verb_roll_die(PlayerAst::Implicit, 6)],
            }));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported roll-dice clause (clause: '{}')",
            clause_text
        )));
    }
    if let Some((prefix, tail_clause)) = clause.strip_any_prefix_clause(ODD_EVEN_RESULT_PREFIXES) {
        let predicate = if prefix == ODD_EVEN_RESULT_PREFIXES[0] {
            crate::effect::Comparison::OneOf(ODD_RESULT_VALUES_D6)
        } else {
            crate::effect::Comparison::OneOf(EVEN_RESULT_VALUES_D6)
        };
        let mut tail_clause = tail_clause.trimmed();
        while tail_clause.first_is_any_word(&["then", "you"]) {
            tail_clause = tail_clause.from(1).trimmed();
        }
        let tail_tokens = tail_clause.tokens();
        let Some((verb, verb_idx)) = find_verb(tail_tokens) else {
            return Err(CardTextError::ParseError(format!(
                "missing action after odd/even-result clause (clause: '{}')",
                clause_text
            )));
        };
        if verb_idx != 0 {
            return Err(CardTextError::ParseError(format!(
                "unsupported odd/even-result action prefix (clause: '{}')",
                clause_text
            )));
        }
        let effect = parse_effect_with_verb(verb, None, &tail_tokens[1..])?;
        return Ok(Some(EffectAst::IfResult {
            predicate: IfResultPredicate::Value(predicate),
            effects: vec![effect],
        }));
    }

    if word_slice_first_is_any(&clause_words, &["dredge", "warp", "harness"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported keyword effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some((_, mut target_clause)) =
        clause.strip_any_suffix_clause(&[&["phase", "out"], &["phases", "out"]])
        && !target_clause.is_empty()
    {
        target_clause = target_clause.trimmed();
        if target_clause.first_is_word("simultaneously") {
            target_clause = target_clause.from(1).trimmed();
        }
        if target_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target in phase-out clause (clause: '{}')",
                clause_text
            )));
        }
        if target_clause.first_is_word("all") {
            let filter_clause = target_clause.from(1).trimmed();
            if filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing filter in all phase-out clause (clause: '{}')",
                    clause_text
                )));
            }
            let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
            filter.zone.get_or_insert(Zone::Battlefield);
            return Ok(Some(EffectAst::subject_verb_phase_out_all(filter)));
        }
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(EffectAst::subject_verb_phase_out(target)));
    }

    if let Some((_, mut target_clause)) =
        clause.strip_any_suffix_clause(&[&["phase", "in"], &["phases", "in"]])
        && clause_tokens.len() >= 2
    {
        target_clause = target_clause.trimmed();
        if target_clause.first_is_word("simultaneously") {
            target_clause = target_clause.from(1).trimmed();
        }
        if target_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target in phase-in clause (clause: '{}')",
                clause_text
            )));
        }
        if target_clause.first_is_word("all")
            && target_clause
                .token(1)
                .is_some_and(|token| token.is_word("phased-out") || token.is_word("phased"))
        {
            let filter_clause = target_clause.from(2).trimmed();
            if filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing filter in all phase-in clause (clause: '{}')",
                    clause_text
                )));
            }
            let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
            filter.zone.get_or_insert(Zone::Battlefield);
            return Ok(Some(EffectAst::subject_verb_phase_in_all(filter)));
        }
        if target_clause.first_is_word("all") {
            let filter_clause = target_clause.from(1).trimmed();
            if filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing filter in all phase-in clause (clause: '{}')",
                    clause_text
                )));
            }
            let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
            filter.zone.get_or_insert(Zone::Battlefield);
            return Ok(Some(EffectAst::subject_verb_phase_in_all(filter)));
        }
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(EffectAst::subject_verb_phase_in(target)));
    }

    if clause.starts_with_any(OPEN_ATTRACTION_PREFIXES) {
        return Ok(Some(EffectAst::subject_verb_open_attraction(
            PlayerAst::Implicit,
        )));
    }

    if word_slice_first_is(&clause_words, "behold") {
        let mut idx = 1usize;
        let mut count = 1u32;
        if let Some((value, used)) = parse_number(&clause_tokens[idx..]) {
            count = value;
            idx += used;
        } else if clause_words
            .get(idx)
            .is_some_and(|word| matches!(*word, "a" | "an"))
        {
            idx += 1;
        }

        let subtype_word = clause_words.get(idx).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing subtype in behold clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let subtype = parse_subtype_word(subtype_word)
            .or_else(|| strip_suffix_char(subtype_word, 's').and_then(parse_subtype_word))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported subtype in behold clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;

        if idx + 1 != clause_words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing behold clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(Some(EffectAst::subject_verb_behold(subtype, count)));
    }

    if word_slice_first_is(&clause_words, "blight") {
        let (amount, used) = parse_number(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing numeric amount for blight clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing blight clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(EffectAst::subject_verb_put_counters(
            crate::object::CounterType::MinusOneMinusOne,
            Value::Fixed(amount as i32),
            TargetAst::Object(ObjectFilter::creature().you_control(), None, None),
            None,
            false,
        )));
    }

    if word_slice_starts_with(&clause_words, &["manifest", "dread"]) {
        let manifest_dread = EffectAst::subject_verb_manifest_dread(PlayerAst::Implicit);
        let trailing_words = &clause_words[2..];
        if trailing_words.is_empty() {
            return Ok(Some(manifest_dread));
        }

        if word_slice_eq(trailing_words, &["twice"]) {
            return Ok(Some(EffectAst::RepeatEffects {
                count: Value::Fixed(2),
                effects: vec![manifest_dread],
            }));
        }

        if word_slice_last_is_any(trailing_words, &["time", "times"]) {
            let value_tokens = &clause_tokens[2..clause_tokens.len() - 1];
            let (count, used) = parse_value(value_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing manifest dread count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            if used != value_tokens.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported manifest dread count tail (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(Some(EffectAst::RepeatEffects {
                count,
                effects: vec![manifest_dread],
            }));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing manifest dread clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if word_slice_eq(
        &clause_words,
        &["manifest", "the", "top", "card", "of", "your", "library"],
    ) {
        return Ok(Some(EffectAst::subject_verb_manifest_top_card(
            PlayerAst::You,
        )));
    }

    if word_slice_eq(
        &clause_words,
        &["manifest", "a", "card", "from", "your", "hand"],
    ) {
        return Ok(Some(EffectAst::subject_verb_manifest_from_hand(
            PlayerAst::You,
        )));
    }

    if word_slice_eq_any(
        &clause_words,
        &[
            &[
                "manifest", "the", "top", "card", "of", "that", "player's", "library",
            ],
            &[
                "manifest", "the", "top", "card", "of", "that", "players", "library",
            ],
        ],
    ) {
        return Ok(Some(EffectAst::subject_verb_manifest_top_card(
            PlayerAst::ThatPlayerOrTargetController,
        )));
    }

    if word_slice_first_is(&clause_words, "populate") {
        if clause_words.len() == 1 {
            return Ok(Some(EffectAst::subject_verb_populate(Value::Fixed(1))));
        }

        if word_slice_at_is(&clause_words, 1, "twice") && clause_words.len() == 2 {
            return Ok(Some(EffectAst::subject_verb_populate(Value::Fixed(2))));
        }

        let (count, used) = parse_value(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing amount for populate clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let trailing = &clause_words[1 + used..];
        if !word_slice_eq_any(trailing, &[&["time"], &["times"]]) {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing populate clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(Some(EffectAst::subject_verb_populate(count)));
    }

    if word_slice_first_is(&clause_words, "meld")
        && let Some(into_idx) = word_slice_find_word_where(&clause_words, |word| word == "into")
    {
        let subject_words = &clause_words[1..into_idx];
        if !word_slice_eq_any(subject_words, &[&["them"], &["those", "cards"]]) {
            return Err(CardTextError::ParseError(format!(
                "unsupported meld subject (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if into_idx + 1 >= clause_words.len() {
            return Err(CardTextError::ParseError(format!(
                "missing meld result name (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let result_name = clause_words[into_idx + 1..].join(" ");
        return Ok(Some(EffectAst::subject_verb_meld(
            result_name,
            false,
            false,
        )));
    }

    if matches!(
        clause_words.first().copied(),
        Some("bolster" | "support" | "adapt")
    ) {
        let keyword = clause_words[0];
        let (amount, used) = parse_number(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing numeric amount for {keyword} clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing {keyword} clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effect = match keyword {
            "bolster" => EffectAst::subject_verb_bolster(amount),
            "support" => EffectAst::subject_verb_support(amount),
            "adapt" => EffectAst::subject_verb_adapt(amount),
            _ => unreachable!(),
        };
        return Ok(Some(effect));
    }

    if word_slice_first_is(&clause_words, "fateseal") {
        let (count, used) = parse_value(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing amount for fateseal clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing fateseal clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(EffectAst::subject_verb_fateseal(
            PlayerAst::You,
            count,
        )));
    }

    if matches!(
        clause_words.first().copied(),
        Some("discover" | "discovers")
    ) {
        if clause_words
            .get(1..)
            .is_some_and(|tail| word_slice_eq(tail, &["again", "for", "the", "same", "value"]))
        {
            return Ok(Some(EffectAst::subject_verb_discover(
                PlayerAst::You,
                Value::EventValue(EventValueSpec::Amount),
            )));
        }
        let (count, used) = parse_value(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing amount for discover clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing discover clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(EffectAst::subject_verb_discover(
            PlayerAst::You,
            count,
        )));
    }

    if let Some(explore_idx) = clause_words
        .iter()
        .position(|word| matches!(*word, "explore" | "explores"))
    {
        let tail_words = &clause_words[explore_idx + 1..];
        if !tail_words.is_empty()
            && !word_slice_eq(tail_words, &["again"])
            && !word_slice_last_is(tail_words, "times")
        {
            return Ok(None);
        }

        let subject_tokens = &clause_tokens[..explore_idx];
        let subject_word_view = ClausePatternCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        let target = if subject_words.is_empty()
            || word_slice_eq_any(
                &subject_words,
                &[
                    &["it"],
                    &["this"],
                    &["this", "creature"],
                    &["this", "permanent"],
                ],
            ) {
            TargetAst::Source(span_from_tokens(subject_tokens))
        } else {
            parse_target_phrase(subject_tokens)?
        };
        let explore = EffectAst::subject_verb_explore(target);
        if word_slice_last_is(tail_words, "times") {
            let value_tokens = &clause_tokens[explore_idx + 1..clause_tokens.len() - 1];
            let (count, used) = parse_value(value_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing explore count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            if used != value_tokens.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported explore count tail (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(Some(EffectAst::RepeatEffects {
                count,
                effects: vec![explore],
            }));
        }
        return Ok(Some(explore));
    }

    Ok(None)
}

pub(crate) fn parse_connive_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(connive_idx) = find_token_index_rev(tokens, |token| {
        token.is_word("connive") || token.is_word("connives")
    }) else {
        return Ok(None);
    };

    let mut count = Value::Fixed(1);
    let mut trailing_tokens = trim_commas(&tokens[connive_idx + 1..]);
    if !trailing_tokens.is_empty() {
        let Some((parsed_count, used)) = parse_value(&trailing_tokens) else {
            return Ok(None);
        };
        count = parsed_count;
        trailing_tokens = trim_commas(&trailing_tokens[used..]);
        if !trailing_tokens.is_empty() {
            let Some(where_value) = parse_value_binding_clause(&trailing_tokens) else {
                return Ok(None);
            };
            count = super::super::util::replace_unbound_x_with_value(
                count,
                &where_value,
                &crate::runtime_backend::token_word_refs(&trailing_tokens).join(" "),
            )?;
        }
    }

    if trailing_tokens
        .iter()
        .any(|token| token.as_word().is_some())
    {
        return Ok(None);
    }

    let subject_tokens = &tokens[..connive_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_word_view = ClausePatternCompatWords::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    if word_slice_eq(
        &subject_words,
        &["each", "creature", "that", "convoked", "this", "spell"],
    ) {
        return Ok(Some(EffectAst::ForEachTagged {
            tag: TagKey::from("convoked_this_spell"),
            effects: vec![EffectAst::subject_verb_connive_iterated()],
        }));
    }

    let target_tokens = if subject_words.len() >= 4
        && subject_words[0] == "each"
        && subject_words[1] == "of"
        && (subject_words[2] == "x" || subject_words[2] == "X")
        && subject_words[3] == "target"
    {
        &subject_tokens[2..]
    } else {
        subject_tokens
    };
    let target = parse_target_phrase(target_tokens)?;
    Ok(Some(EffectAst::subject_verb_connive(target, count)))
}
