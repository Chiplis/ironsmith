use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, OwnedLexToken, PlayerAst,
    PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst, SubjectAst, TagKey, TargetAst,
    TextSpan, Verb,
};
use crate::effect::{Until, Value};
use crate::static_abilities::StaticAbilityId;
use crate::target::ObjectFilter;
use crate::zone::Zone;
use crate::{ChoiceCount, Supertype};

use super::super::activation_and_restrictions::{
    starts_with_target_indicator, title_case_token_word,
};
use super::super::native_tokens::LowercaseWordView;
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    parse_card_type, parse_color, parse_counter_type_from_tokens, parse_counter_type_word,
    parse_number, parse_subject, parse_target_count_range_prefix, parse_target_phrase, parse_value,
    span_from_tokens, token_index_for_word_index, trim_commas, words, wrap_target_count,
};
use super::chain_carry::find_verb;
use super::parse_subtype_word;
use super::sentence_primitives::parse_distribute_counters_sentence;
use super::verb_dispatch::parse_effect_with_verb;

pub(crate) fn extract_subject_player(subject: Option<SubjectAst>) -> Option<PlayerAst> {
    match subject {
        Some(SubjectAst::Player(player)) => Some(player),
        _ => None,
    }
}

pub(crate) fn parse_prevent_next_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("prevent") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if clause_words.get(idx) == Some(&"the") {
        idx += 1;
    }
    if clause_words.get(idx) != Some(&"next") {
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
            clause_words.join(" ")
        )));
    };
    idx += amount_used;

    if clause_words.get(idx) != Some(&"damage") {
        return Ok(None);
    }
    idx += 1;

    if clause_words.get(idx..idx + 4) != Some(["that", "would", "be", "dealt"].as_slice()) {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage clause tail (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 4;

    if clause_words.get(idx) != Some(&"to") {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 1;

    let this_turn_rel = clause_words[idx..]
        .windows(2)
        .position(|window| window == ["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported prevent-next damage duration (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let this_turn_idx = idx + this_turn_rel;
    if this_turn_idx + 2 != clause_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing prevent-next damage clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = &clause_words[idx..this_turn_idx];
    if target_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-next damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target_tokens = target_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(EffectAst::PreventDamage {
        amount,
        target,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_double_counters_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["double", "the", "number", "of"]) {
        return Ok(None);
    }

    let counters_idx = tokens
        .iter()
        .position(|token| token.is_word("counter") || token.is_word("counters"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counters keyword (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    if counters_idx <= 4 {
        return Err(CardTextError::ParseError(format!(
            "missing counter type (clause: '{}')",
            clause_words.join(" ")
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
                clause_words.join(" ")
            ))
        })?;

    let on_idx = tokens[counters_idx + 1..]
        .iter()
        .position(|token| token.is_word("on"))
        .map(|offset| counters_idx + 1 + offset)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing 'on' in double-counters clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let mut filter_tokens = &tokens[on_idx + 1..];
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("each") || token.is_word("all"))
    {
        filter_tokens = &filter_tokens[1..];
    }
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing filter in double-counters clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(Some(EffectAst::DoubleCountersOnEach {
        counter_type,
        filter,
    }))
}

pub(crate) fn parse_distribute_counters_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_distribute_counters_sentence(tokens)
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
        "regenerate" => Verb::Regenerate,
        "mill" => Verb::Mill,
        "get" => Verb::Get,
        "remove" => Verb::Remove,
        "return" => Verb::Return,
        "exchange" => Verb::Exchange,
        "become" => Verb::Become,
        "skip" => Verb::Skip,
        "surveil" => Verb::Surveil,
        "shuffle" => Verb::Shuffle,
        "pay" => Verb::Pay,
        "goad" => Verb::Goad,
        "look" => Verb::Look,
        _ => return Ok(None),
    };

    let effect = parse_effect_with_verb(verb, None, &tokens[1..])?;
    Ok(Some(effect))
}

pub(crate) fn is_simple_chosen_object_reference(tokens: &[OwnedLexToken]) -> bool {
    let words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !super::super::util::is_article(word) && *word != "then")
        .collect();
    if words.is_empty() {
        return false;
    }
    if words == ["it"] || words == ["them"] {
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
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["choose", "target"]) {
        return Ok(None);
    }

    let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if and_idx <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..and_idx]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target after choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if find_verb(&target_tokens).is_some() {
        return Ok(None);
    }

    let mut tail_tokens = trim_commas(&tokens[and_idx + 1..]);
    if tail_tokens
        .first()
        .is_some_and(|token| token.is_word("then"))
    {
        tail_tokens = tail_tokens[1..].to_vec();
    }
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let Some((verb, verb_idx)) = find_verb(&tail_tokens) else {
        return Ok(None);
    };
    if verb_idx != 0 {
        return Ok(None);
    }

    let rest_tokens = trim_commas(&tail_tokens[1..]);
    if !is_simple_chosen_object_reference(&rest_tokens) {
        return Ok(None);
    }

    let effect = parse_effect_with_verb(verb, None, &target_tokens)?;
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
            let mut after = words(&tail[idx + 1..]);
            if after.first().copied() == Some("may") {
                after.remove(0);
            }
            if after.first().copied() == Some("choose")
                && after
                    .iter()
                    .any(|word| *word == "target" || *word == "targets")
                && after.iter().any(|word| *word == "copy")
            {
                return Some(idx);
            }
        }
        None
    }

    let clause_words = words(tokens);
    let Some(copy_idx) = tokens
        .iter()
        .position(|token| token.is_word("copy") || token.is_word("copies"))
    else {
        return Ok(None);
    };
    let tail = &tokens[copy_idx + 1..];
    let split_idx = find_choose_new_targets_split_idx(tail);
    let simple_copy_reference = copy_idx == 0
        && matches!(
            clause_words.get(1).copied(),
            Some("it") | Some("this") | Some("that")
        );
    if simple_copy_reference {
        if let Some(then_idx) = tokens.iter().position(|token| token.is_word("then")) {
            let tail_tokens = trim_commas(&tokens[then_idx + 1..]);
            if let Some(spec) =
                super::super::activation_and_restrictions::parse_may_cast_it_sentence(&tail_tokens)
                && spec.as_copy
            {
                return Ok(Some(
                    super::super::activation_and_restrictions::build_may_cast_tagged_effect(&spec),
                ));
            }
        }
        let mut count = Value::Fixed(1);
        let copy_target_tail = if let Some(idx) = split_idx {
            &tail[..idx]
        } else {
            tail
        };
        if let Some(for_each_idx) = copy_target_tail
            .windows(2)
            .position(|window| window[0].is_word("for") && window[1].is_word("each"))
        {
            let count_filter_tokens = trim_commas(&copy_target_tail[for_each_idx + 2..]);
            if count_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing count filter after 'for each' in copy clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let count_filter = parse_object_filter(&count_filter_tokens, false)?;
            count = Value::Count(count_filter);
        }
        let base = EffectAst::CopySpell {
            target: TargetAst::Source(None),
            count,
            player: PlayerAst::Implicit,
            may_choose_new_targets: split_idx.is_some(),
        };
        if let Some(if_idx) = tokens.iter().position(|token| token.is_word("if")) {
            let predicate_tokens = trim_commas(&tokens[if_idx + 1..]);
            if predicate_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing predicate after copy clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let predicate = super::conditionals::parse_predicate(&predicate_tokens)?;
            return Ok(Some(EffectAst::Conditional {
                predicate,
                if_true: vec![base],
                if_false: Vec::new(),
            }));
        }
        return Ok(Some(base));
    }
    if !clause_words.contains(&"spell")
        && !clause_words.contains(&"spells")
        && !clause_words.contains(&"ability")
        && !clause_words.contains(&"abilities")
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
            clause_words.join(" ")
        )));
    }

    let mut count = Value::Fixed(1);
    let mut copy_target_tail = if let Some(idx) = split_idx {
        &tail[..idx]
    } else {
        tail
    };
    if let Some(for_each_idx) = copy_target_tail
        .windows(2)
        .position(|window| window[0].is_word("for") && window[1].is_word("each"))
    {
        let count_filter_tokens = trim_commas(&copy_target_tail[for_each_idx + 2..]);
        if count_filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing count filter after 'for each' in copy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let count_filter = parse_object_filter(&count_filter_tokens, false)?;
        count = Value::Count(count_filter);
        copy_target_tail = &copy_target_tail[..for_each_idx];
    }

    let copy_target_tokens = trim_commas(copy_target_tail);
    if copy_target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = words(&copy_target_tokens);
    let target = if target_words.as_slice() == ["this", "spell"]
        || target_words.as_slice() == ["that", "spell"]
    {
        TargetAst::Source(None)
    } else {
        parse_counter_target_phrase(&copy_target_tokens)?
    };

    let mut may_choose_new_targets = false;
    if let Some(idx) = split_idx {
        let mut choose_words = words(&tail[idx + 1..]);
        if choose_words.first().copied() == Some("may") {
            may_choose_new_targets = true;
            choose_words.remove(0);
        }
        let has_choose = choose_words.first().copied() == Some("choose");
        let has_new = choose_words.contains(&"new");
        let has_target = choose_words
            .iter()
            .any(|word| *word == "target" || *word == "targets");
        let has_copy = choose_words.contains(&"copy");
        if !has_choose || !has_target || !has_copy {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing copy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if !has_new {
            return Err(CardTextError::ParseError(format!(
                "missing 'new' in copy retarget clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    Ok(Some(EffectAst::CopySpell {
        target,
        count,
        player,
        may_choose_new_targets,
    }))
}

pub(crate) fn parse_counter_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if let Some(target) = parse_counter_ability_target_phrase(tokens)? {
        return Ok(target);
    }

    let clause_words = words(tokens);
    if clause_words.contains(&"ability")
        && (clause_words.contains(&"activated") || clause_words.contains(&"triggered"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-ability target clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    parse_target_phrase(tokens)
}

fn parse_counter_ability_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let clause_tokens = trim_commas(tokens);
    let clause_words = words(&clause_tokens);
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
    if !clause_words.contains(&"ability")
        || (!clause_words.contains(&"activated") && !clause_words.contains(&"triggered"))
    {
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

    let mut term_filters: Vec<(ObjectFilter, CounterTargetTerm)> = Vec::new();
    let mut list_end = clause_tokens.len();
    let mut scan = idx;
    while scan < clause_tokens.len() {
        if clause_tokens
            .get(scan)
            .is_some_and(|token| token.is_word("from"))
        {
            list_end = scan;
            break;
        }
        if is_you_control_tail(scan) {
            list_end = scan;
            break;
        }
        scan += 1;
    }

    while idx < list_end {
        let Some(word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word) else {
            idx += 1;
            continue;
        };
        if matches!(word, "or" | "and") {
            idx += 1;
            continue;
        }

        if word == "activated"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("or"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("triggered"))
            && clause_tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("ability"))
        {
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            idx += 4;
            continue;
        }

        if word == "triggered"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("or"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("activated"))
            && clause_tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("ability"))
        {
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            idx += 4;
            continue;
        }

        if word == "activated"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("ability"))
        {
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            idx += 2;
            continue;
        }

        if word == "triggered"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("ability"))
        {
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            idx += 2;
            continue;
        }

        if word == "spell" {
            term_filters.push((ObjectFilter::spell(), CounterTargetTerm::Spell));
            idx += 1;
            continue;
        }

        if word == "instant"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            term_filters.push((
                ObjectFilter::spell().with_type(crate::types::CardType::Instant),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if word == "sorcery"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            term_filters.push((
                ObjectFilter::spell().with_type(crate::types::CardType::Sorcery),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if word == "legendary"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            term_filters.push((
                ObjectFilter::spell().with_supertype(Supertype::Legendary),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if word == "noncreature"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            let mut filter = ObjectFilter::noncreature_spell().in_zone(Zone::Stack);
            filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
            term_filters.push((filter, CounterTargetTerm::Spell));
            idx += 2;
            continue;
        }

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
                    .or_else(|| type_word.strip_suffix('s').and_then(parse_card_type));
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
    let clause_words = words(tokens);
    let prefix_target_then_duration = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
    ];
    let prefix_duration_then_target = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
    ];
    if !clause_words.starts_with(&prefix_target_then_duration)
        && !clause_words.starts_with(&prefix_duration_then_target)
    {
        return Ok(None);
    }
    let target_words = if clause_words.starts_with(&prefix_duration_then_target) {
        &clause_words[prefix_duration_then_target.len()..]
    } else {
        if clause_words.len() <= prefix_target_then_duration.len() + 1 {
            return Err(CardTextError::ParseError(format!(
                "missing prevent-all damage target (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if clause_words[clause_words.len().saturating_sub(2)..] != ["this", "turn"] {
            return Err(CardTextError::ParseError(format!(
                "unsupported prevent-all damage duration (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        &clause_words[prefix_target_then_duration.len()..clause_words.len() - 2]
    };
    if target_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_tokens = target_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(EffectAst::PreventAllDamageToTarget {
        target,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_can_attack_as_though_no_defender_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(can_idx) = clause_words.iter().position(|word| *word == "can") else {
        return Ok(None);
    };
    let subject_words = &clause_words[..can_idx];
    let tail = &clause_words[can_idx..];
    let has_core = tail.starts_with(&["can", "attack"])
        && tail.windows(2).any(|window| window == ["as", "though"])
        && tail.contains(&"turn")
        && tail.contains(&"have")
        && tail.last().copied() == Some("defender");
    if !has_core {
        return Ok(None);
    }

    let target = if subject_words.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        let subject_tokens = subject_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        parse_target_phrase(&subject_tokens)?
    };

    Ok(Some(EffectAst::GrantAbilitiesToTarget {
        target,
        abilities: vec![GrantedAbilityAst::CanAttackAsThoughNoDefender],
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_prevent_next_time_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if !clause_words.starts_with(&["the", "next", "time"]) {
        return Ok(None);
    }

    let Some(would_idx) = clause_words.iter().position(|w| *w == "would") else {
        return Ok(None);
    };
    if clause_words.get(would_idx + 1..would_idx + 4) != Some(["deal", "damage", "to"].as_slice()) {
        return Ok(None);
    }

    let this_turn_rel = clause_words[would_idx + 4..]
        .windows(2)
        .position(|window| window == ["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported prevent-next-time damage duration (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let this_turn_idx = (would_idx + 4) + this_turn_rel;

    let tail = &clause_words[this_turn_idx + 2..];
    if tail != ["prevent", "that", "damage"] {
        return Ok(None);
    }

    let source_words = &clause_words[3..would_idx];
    if source_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-next-time damage source (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let source = if source_words
        .windows(3)
        .any(|w| w == ["of", "your", "choice"])
    {
        PreventNextTimeDamageSourceAst::Choice
    } else {
        let mut words = source_words.to_vec();
        if words.first().is_some_and(|w| matches!(*w, "a" | "an")) {
            words.remove(0);
        }
        if words.last().copied() == Some("source") {
            words.pop();
        }
        if words.is_empty() {
            return Ok(Some(vec![EffectAst::PreventNextTimeDamage {
                source: PreventNextTimeDamageSourceAst::Filter(ObjectFilter::default()),
                target: PreventNextTimeDamageTargetAst::AnyTarget,
            }]));
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
                if !filter.card_types.contains(&card_type) {
                    filter.card_types.push(card_type);
                }
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

    let target_words = &clause_words[would_idx + 4..this_turn_idx];
    let target = if target_words == ["you"] {
        PreventNextTimeDamageTargetAst::You
    } else if target_words == ["any", "target"] {
        PreventNextTimeDamageTargetAst::AnyTarget
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next-time damage target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    Ok(Some(vec![EffectAst::PreventNextTimeDamage {
        source,
        target,
    }]))
}

pub(crate) fn parse_redirect_next_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if clause_words.starts_with(&["the", "next", "time"]) {
        let Some(would_idx) = clause_words.iter().position(|word| *word == "would") else {
            return Ok(None);
        };
        if clause_words.get(would_idx + 1..would_idx + 4)
            != Some(["deal", "damage", "to"].as_slice())
        {
            return Ok(None);
        }

        let this_turn_rel = clause_words[would_idx + 4..]
            .windows(2)
            .position(|window| window == ["this", "turn"])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported redirected-next-time damage duration (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let this_turn_idx = (would_idx + 4) + this_turn_rel;

        let source_words = &clause_words[3..would_idx];
        if source_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-next-time damage source (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let source = if source_words
            .windows(3)
            .any(|window| window == ["of", "your", "choice"])
        {
            PreventNextTimeDamageSourceAst::Choice
        } else {
            let mut words = source_words.to_vec();
            if words
                .first()
                .is_some_and(|word| matches!(*word, "a" | "an"))
            {
                words.remove(0);
            }
            if words.last().copied() == Some("source") {
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
                    if !filter.card_types.contains(&card_type) {
                        filter.card_types.push(card_type);
                    }
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

        let target_words = &clause_words[would_idx + 4..this_turn_idx];
        if target_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-next-time damage target (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target_tokens = target_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let target = parse_target_phrase(&target_tokens)?;

        let tail = &clause_words[this_turn_idx + 2..];
        if tail.len() < 7
            || !tail.starts_with(&["that", "damage", "is", "dealt", "to"])
            || tail.last().copied() != Some("instead")
        {
            return Ok(None);
        }
        let redirect_words = &tail[5..tail.len() - 1];
        let redirects_to_source = matches!(
            redirect_words,
            ["this"] | ["it"] | ["this", "creature"] | ["this", "permanent"]
        );
        if !redirects_to_source {
            return Err(CardTextError::ParseError(format!(
                "unsupported redirected-next-time damage destination (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(Some(vec![EffectAst::RedirectNextTimeDamageToSource {
            source,
            target,
        }]));
    }

    if !clause_words.starts_with(&["the", "next"]) {
        return Ok(None);
    }

    let amount_token = OwnedLexToken::word(
        clause_words.get(2).copied().unwrap_or_default().to_string(),
        TextSpan::synthetic(),
    );
    let Some((amount, amount_used)) = parse_value(&[amount_token]) else {
        return Ok(None);
    };
    if amount_used != 1 {
        return Err(CardTextError::ParseError(format!(
            "unsupported redirected-next-damage amount (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut idx = 3usize;
    if clause_words.get(idx..idx + 6)
        != Some(["damage", "that", "would", "be", "dealt", "to"].as_slice())
    {
        return Ok(None);
    }
    idx += 6;

    let this_turn_rel = clause_words[idx..]
        .windows(2)
        .position(|window| window == ["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported redirected-next-damage duration (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let this_turn_idx = idx + this_turn_rel;
    let protected_words = &clause_words[idx..this_turn_idx];
    let protects_source = matches!(
        protected_words,
        ["this"] | ["it"] | ["this", "creature"] | ["this", "permanent"]
    );
    if !protects_source {
        return Err(CardTextError::ParseError(format!(
            "unsupported redirected-next-damage protected target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let tail = &clause_words[this_turn_idx + 2..];
    if tail.len() < 5
        || !tail.starts_with(&["is", "dealt", "to"])
        || tail.last().copied() != Some("instead")
    {
        return Ok(None);
    }

    let target_words = &tail[3..tail.len() - 1];
    if target_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing redirected-next-damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target_tokens = target_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(vec![
        EffectAst::RedirectNextDamageFromSourceToTarget { amount, target },
    ]))
}

pub(crate) fn parse_can_block_additional_creature_this_turn_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(can_idx) = clause_words.iter().position(|word| *word == "can") else {
        return Ok(None);
    };
    let subject_words = &clause_words[..can_idx];
    let tail = &clause_words[can_idx..];
    if !tail.starts_with(&["can", "block"]) || !tail.ends_with(&["this", "turn"]) {
        return Ok(None);
    }

    let Some(additional_offset) = tail.iter().position(|word| *word == "additional") else {
        return Ok(None);
    };
    if tail.get(additional_offset + 1).copied() != Some("creature")
        && tail.get(additional_offset + 1).copied() != Some("creatures")
    {
        return Ok(None);
    }

    let mut additional = 1usize;
    if additional_offset > 0 {
        let number_word_idx = can_idx + additional_offset - 1;
        if clause_words[number_word_idx] != "a"
            && clause_words[number_word_idx] != "an"
            && let Some(number_token_idx) = token_index_for_word_index(tokens, number_word_idx)
            && let Some((parsed, used)) = parse_number(&tokens[number_token_idx..])
            && used > 0
        {
            additional = parsed as usize;
        }
    }

    let target = if subject_words.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        let subject_tokens = subject_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        parse_target_phrase(&subject_tokens)?
    };

    Ok(Some(EffectAst::GrantAbilitiesToTarget {
        target,
        abilities: vec![GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat { additional }],
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_win_the_game_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 4 || !clause_words.starts_with(&["you", "win", "the", "game"]) {
        return Ok(None);
    }

    if clause_words.len() == 4 {
        return Ok(Some(EffectAst::WinGame {
            player: PlayerAst::You,
        }));
    }

    if clause_words.get(4).copied() != Some("if") {
        return Ok(None);
    }

    let if_tail = clause_words.get(5..).unwrap_or_default();
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
    let Some(in_idx) = after_named.iter().position(|word| *word == "in") else {
        return Ok(None);
    };
    if in_idx == 0 {
        return Ok(None);
    }

    let name_words = &after_named[..in_idx];
    let remainder = &after_named[in_idx..];

    let has_exile = remainder.contains(&"exile");
    let has_hand = remainder.contains(&"hand");
    let has_graveyard = remainder.contains(&"graveyard");
    let has_battlefield = remainder.contains(&"battlefield");
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
        if_true: vec![EffectAst::WinGame {
            player: PlayerAst::You,
        }],
        if_false: Vec::new(),
    }))
}

pub(crate) fn parse_choose_target_prelude_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("choose") {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..]);
    if target_tokens.is_empty() || !starts_with_target_indicator(&target_tokens) {
        return Ok(None);
    }
    if find_verb(&target_tokens).is_some() {
        return Ok(None);
    }

    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::TargetOnly { target }))
}

pub(crate) fn parse_keyword_mechanic_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut start = 0usize;
    if tokens.get(start).is_some_and(|token| token.is_word("then")) {
        start += 1;
    }
    if tokens.get(start).is_some_and(|token| token.is_word("you")) {
        start += 1;
    }
    if start >= tokens.len() {
        return Ok(None);
    }

    let clause_tokens = &tokens[start..];
    let clause_words = words(clause_tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    if clause_words.first() == Some(&"amass") {
        let mut amount_start = 1usize;
        let mut subtype = None;

        if let Some(candidate) = clause_words.get(amount_start).copied()
            && let Some(parsed_subtype) = parse_subtype_word(candidate)
                .or_else(|| candidate.strip_suffix('s').and_then(parse_subtype_word))
            && parsed_subtype.is_creature_type()
        {
            subtype = Some(parsed_subtype);
            amount_start += 1;
        }

        let (amount, used) = parse_number(&clause_tokens[amount_start..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing numeric amount for amass clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if amount_start + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing amass clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(Some(EffectAst::Amass { subtype, amount }));
    }

    if clause_words.first() == Some(&"roll") && clause_words.contains(&"dice") {
        return Err(CardTextError::ParseError(format!(
            "unsupported roll-dice clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if clause_words.starts_with(&["for", "each", "odd", "result"])
        || clause_words.starts_with(&["for", "each", "even", "result"])
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported odd/even-result clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if clause_words.first() == Some(&"dredge")
        || clause_words.first() == Some(&"warp")
        || clause_words.first() == Some(&"harness")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported keyword effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if (clause_words.ends_with(&["phase", "out"]) || clause_words.ends_with(&["phases", "out"]))
        && clause_tokens.len() >= 2
    {
        let target_tokens = trim_commas(&clause_tokens[..clause_tokens.len() - 2]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target in phase-out clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target = parse_target_phrase(&target_tokens)?;
        return Ok(Some(EffectAst::PhaseOut { target }));
    }

    if clause_words.starts_with(&["open", "an", "attraction"])
        || clause_words.starts_with(&["opens", "an", "attraction"])
    {
        return Ok(Some(EffectAst::OpenAttraction));
    }

    if clause_words == ["manifest", "dread"] {
        return Ok(Some(EffectAst::ManifestDread));
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
            "bolster" => EffectAst::Bolster { amount },
            "support" => EffectAst::Support { amount },
            "adapt" => EffectAst::Adapt { amount },
            _ => unreachable!(),
        };
        return Ok(Some(effect));
    }

    if clause_words.first() == Some(&"fateseal") {
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
        return Ok(Some(EffectAst::Scry {
            count,
            player: PlayerAst::Opponent,
        }));
    }

    if matches!(
        clause_words.first().copied(),
        Some("discover" | "discovers")
    ) {
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
        return Ok(Some(EffectAst::Discover {
            count,
            player: PlayerAst::You,
        }));
    }

    if matches!(clause_words.last().copied(), Some("explore" | "explores")) {
        let subject_tokens = &clause_tokens[..clause_tokens.len().saturating_sub(1)];
        let subject_words = words(subject_tokens);
        let target = if subject_words.is_empty()
            || subject_words == ["it"]
            || subject_words == ["this"]
            || subject_words == ["this", "creature"]
            || subject_words == ["this", "permanent"]
        {
            TargetAst::Source(span_from_tokens(subject_tokens))
        } else {
            parse_target_phrase(subject_tokens)?
        };
        return Ok(Some(EffectAst::Explore { target }));
    }

    Ok(None)
}

pub(crate) fn parse_connive_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(connive_idx) = tokens
        .iter()
        .rposition(|token| token.is_word("connive") || token.is_word("connives"))
    else {
        return Ok(None);
    };

    if tokens[connive_idx + 1..]
        .iter()
        .any(|token| token.as_word().is_some())
    {
        return Ok(None);
    }

    let subject_tokens = &tokens[..connive_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = words(subject_tokens);
    if subject_words == ["each", "creature", "that", "convoked", "this", "spell"] {
        return Ok(Some(EffectAst::ForEachTagged {
            tag: TagKey::from("convoked_this_spell"),
            effects: vec![EffectAst::ConniveIterated],
        }));
    }

    let target = parse_target_phrase(subject_tokens)?;
    Ok(Some(EffectAst::Connive { target }))
}
