use super::*;
use crate::TextSpan;
use crate::cards::builders::{
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst,
};
use crate::runtime_backend::lexer::{
    TokenWordView, token_slice_at_is, word_slice_contains_all_words, word_slice_contains_any_word,
    word_slice_contains_phrase, word_slice_eq, word_slice_eq_any, word_slice_starts_with,
    word_slice_starts_with_any,
};
use crate::runtime_backend::parse_counter_type_from_tokens;

const MONARCH_WORDS: &[&[&str]] = &[&["the", "monarch"], &["monarch"]];
const INSTEAD_WORDS: &[&str] = &["instead"];
const POWER_WORD: &str = "power";
const IT_WORD: &str = "it";
const NEXT_COMBAT_PHASE_THIS_TURN_WORDS: &[&str] = &["combat", "phase", "next", "this", "turn"];
const COMBAT_PHASE_TURN_WORDS: &[&str] = &["combat", "turn"];
const COMBAT_PHASE_WORDS: &[&str] = &["phase", "phases"];
const DRAW_STEP_WORDS: &[&str] = &["draw", "step"];
const TURN_WORDS: &[&str] = &["turn"];
const END_TURN_WORDS: &[&[&str]] = &[&["the", "turn"], &["turn"]];
const END_STEP_YOU_LOSE_WORDS: &[&str] = &["step", "you", "lose", "the", "game"];
const COIN_WORDS: &[&[&str]] = &[&["a", "coin"], &["coin"]];
const SELF_FLIP_TARGET_WORDS: &[&[&str]] = &[
    &["it"],
    &["this"],
    &["this", "creature"],
    &["this", "permanent"],
];
const ARTICLE_WORDS: &[&str] = &["a", "an"];
const DIE_SIDED_SUFFIX: &str = "-sided";
const SIDED_WORD: &str = "sided";
const DIE_WORDS: &[&str] = &["die", "dice"];
const CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"], &["each"]];
const ON_WORD: &str = "on";
const FOR_EACH_EXPLICIT_PREFIX: &[&str] = &["for", "each"];
const THIS_REFERENCE_PREFIXES: &[&[&str]] = &[&["this"], &["it"]];
const ENERGY_WORD: &str = "e";
const TICKET_WORD: &str = "tk";
const BEGINNING_OF_PREFIX: &[&str] = &["at", "the", "beginning", "of"];
const THIS_WORD: &str = "this";
const ALL_OR_EACH_WORDS: &[&str] = &["all", "each"];
const COUNTER_OR_COUNTERS_WORDS: &[&str] = &["counter", "counters"];
const THEM_WORDS: &[&str] = &["them"];
const FOR_EACH_PHRASE: &[&str] = &["for", "each"];
const CHOSEN_THIS_WAY_PHRASE: &[&str] = &["chosen", "this", "way"];
const THOSE_OR_THEM_WORDS: &[&str] = &["those", "them"];
const ENERGY_COUNTER_PAY_IGNORED_WORDS: &[&str] = &["and", "or", "energy", "counter", "counters"];
const ENERGY_TEXT_WORD: &str = "energy";

fn mana_group_token_matches_symbol(token: &OwnedLexToken, expected: &str) -> bool {
    if token.kind != TokenKind::ManaGroup {
        return false;
    }
    let Some(symbol) = token.mana_group_inner() else {
        return false;
    };
    symbol == expected
}

fn token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word().is_some_and(|word| word == expected)
}

fn energy_symbol_token(token: &OwnedLexToken) -> bool {
    token_is_word(token, ENERGY_WORD) || mana_group_token_matches_symbol(token, ENERGY_WORD)
}

fn exact_mana_pip_groups(tokens: &[OwnedLexToken]) -> Option<Vec<Vec<ManaSymbol>>> {
    let tokens = trim_commas(tokens);
    if tokens.is_empty() {
        return None;
    }

    let mut pips = Vec::new();
    for token in tokens {
        match token.kind {
            TokenKind::ManaGroup => {
                let slice = token.slice.as_str();
                pips.push(parse_mana_symbol_group(slice).ok()?);
            }
            TokenKind::Word | TokenKind::Number => {
                let word = token.as_word()?;
                pips.push(parse_mana_symbol_group(word).ok()?);
            }
            _ => return None,
        }
    }

    (!pips.is_empty()).then_some(pips)
}

fn exact_pay_component(tokens: &[OwnedLexToken], player: PlayerAst) -> Option<EffectAst> {
    let tokens = trim_commas(tokens);
    if tokens.is_empty() {
        return None;
    }

    if let Some((amount, used)) = parse_value(&tokens)
        && token_slice_at_is(&tokens, used, "life")
        && trim_commas(&tokens[used + 1..]).is_empty()
    {
        return Some(subject_verb_player_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        ));
    }

    if let Some((amount, used)) = parse_value(&tokens)
        && tokens
            .get(used)
            .is_some_and(|token| token.as_word().is_some_and(|word| word == ENERGY_TEXT_WORD))
        && trim_commas(&tokens[used + 1..]).is_empty()
    {
        return Some(EffectAst::subject_verb_pay_energy(player, amount));
    }

    let pips = exact_mana_pip_groups(&tokens)?;
    Some(EffectAst::subject_verb_pay_mana(
        player,
        ManaCost::from_pips(pips),
    ))
}

fn parse_compound_pay(tokens: &[OwnedLexToken], player: PlayerAst) -> Option<EffectAst> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_some_and(|word| word == "and") {
            parts.push(trim_commas(&tokens[start..idx]));
            start = idx + 1;
        }
    }
    if parts.is_empty() {
        return None;
    }
    parts.push(trim_commas(&tokens[start..]));
    if parts.iter().any(|part| part.is_empty()) {
        return None;
    }

    let mut effects = Vec::new();
    for part in parts {
        effects.push(exact_pay_component(&part, player)?);
    }
    (effects.len() > 1).then_some(EffectAst::Sequence { effects })
}

fn ticket_symbol_token(token: &OwnedLexToken) -> bool {
    token_is_word(token, TICKET_WORD) || mana_group_token_matches_symbol(token, TICKET_WORD)
}

fn subject_verb_player_effect(
    role: SubjectVerbRoleAst,
    player: PlayerAst,
    action: SubjectVerbActionAst,
) -> EffectAst {
    EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { role, player },
        action,
    })
}

fn player_filter_for_library_count(player: PlayerAst) -> Option<PlayerFilter> {
    let filter = match player {
        PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
        PlayerAst::Opponent => PlayerFilter::Opponent,
        PlayerAst::NotYou => PlayerFilter::NotYou,
        PlayerAst::Any => PlayerFilter::Any,
        PlayerAst::Target => PlayerFilter::target_player(),
        PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
        PlayerAst::That => PlayerFilter::IteratedPlayer,
        PlayerAst::ThatPlayerOrTargetController => PlayerFilter::target_player(),
        PlayerAst::Defending => PlayerFilter::Defending,
        PlayerAst::Attacking => PlayerFilter::Attacking,
        PlayerAst::Chosen => PlayerFilter::ChosenPlayer,
        PlayerAst::MostCardsInHand => PlayerFilter::MostCardsInHand,
        PlayerAst::MostLifeTied => PlayerFilter::MostLifeTied,
        PlayerAst::LowestLifeTied => PlayerFilter::LowestLifeTied,
        PlayerAst::ItsController | PlayerAst::ItsOwner => return None,
    };
    Some(filter)
}

fn parse_half_library_count_value(
    words: &[&str],
    subject_player: PlayerAst,
) -> Option<(Value, usize)> {
    if words.first().copied() != Some("half") {
        return None;
    }

    let (player, library_idx) = match words.get(1..4) {
        Some(["their", "library", _]) => (player_filter_for_library_count(subject_player)?, 2),
        Some(["your", "library", _]) => (PlayerFilter::You, 2),
        Some(["target", "player", "library"]) | Some(["target", "players", "library"]) => {
            (PlayerFilter::target_player(), 3)
        }
        Some(["target", "opponent", "library"]) | Some(["target", "opponents", "library"]) => {
            (PlayerFilter::target_opponent(), 3)
        }
        Some(["that", "player", "library"]) | Some(["that", "players", "library"]) => {
            (player_filter_for_library_count(subject_player)?, 3)
        }
        _ => return None,
    };

    let round_words = words.get(library_idx + 1..library_idx + 3)?;
    let base = Value::CardsInLibrary(player);
    match round_words {
        ["rounded", "down"] => Some((Value::HalfRoundedDown(Box::new(base)), library_idx + 3)),
        ["rounded", "up"] => Some((
            Value::HalfRoundedDown(Box::new(Value::Add(
                Box::new(base),
                Box::new(Value::Fixed(1)),
            ))),
            library_idx + 3,
        )),
        _ => None,
    }
}

pub(crate) fn parse_become(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let Some(SubjectAst::Player(player)) = subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported become clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_eq_any(&clause_words, MONARCH_WORDS) {
        return Ok(EffectAst::subject_verb_become_monarch(player));
    }

    let amount = parse_value(tokens)
        .map(|(value, _)| value)
        .or_else(|| parse_half_starting_life_total_value(tokens, player))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing life total amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    Ok(EffectAst::subject_verb_set_life_total(player, amount))
}

pub(crate) fn parse_switch(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    use crate::effect::Until;

    let clause_words = crate::runtime_backend::token_word_refs(tokens);

    // Split off trailing duration, if present.
    let (duration, remainder) =
        if let Some((duration, remainder)) = parse_restriction_duration(tokens)? {
            (duration, remainder)
        } else {
            (Until::EndOfTurn, trim_commas(tokens).to_vec())
        };

    let Some(power_idx) = find_index(&remainder, |token| {
        token.as_word().is_some_and(|word| word == POWER_WORD)
    }) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported switch clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    // Target phrase is everything up to "power".
    let target_tokens = &remainder[..power_idx];
    let target_words = crate::runtime_backend::token_word_refs(target_tokens);
    let target = if target_words.is_empty()
        || matches!(
            target_words.as_slice(),
            ["this"]
                | ["this", "creature"]
                | ["this", "creatures"]
                | ["this", "permanent"]
                | ["it"]
        ) {
        if word_slice_eq(&target_words, &[IT_WORD]) {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens))
        } else {
            TargetAst::Source(span_from_tokens(target_tokens))
        }
    } else {
        parse_target_phrase(target_tokens)?
    };

    // Require "... power and toughness ..." somewhere in remainder.
    if !grammar::contains_word(&remainder, "power")
        || !grammar::contains_word(&remainder, "toughness")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported switch clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_switch_power_toughness(
        target, duration,
    ))
}

pub(crate) fn parse_skip(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let (player, words) = match subject {
        Some(SubjectAst::Player(player)) => (player, clause_words),
        _ => {
            if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, YOUR_PREFIXES) {
                (PlayerAst::You, clause_words[prefix.len()..].to_vec())
            } else if let Some((prefix, _)) =
                grammar::words_match_any_prefix(tokens, THEIR_PREFIXES)
            {
                (PlayerAst::That, clause_words[prefix.len()..].to_vec())
            } else if let Some((prefix, _)) =
                grammar::words_match_any_prefix(tokens, THAT_PLAYER_PREFIXES)
            {
                (PlayerAst::That, clause_words[prefix.len()..].to_vec())
            } else if let Some((prefix, _)) =
                grammar::words_match_any_prefix(tokens, TARGET_PLAYER_PREFIXES)
            {
                (PlayerAst::Target, clause_words[prefix.len()..].to_vec())
            } else if let Some((prefix, _)) =
                grammar::words_match_any_prefix(tokens, TARGET_OPPONENT_PREFIXES)
            {
                (
                    PlayerAst::TargetOpponent,
                    clause_words[prefix.len()..].to_vec(),
                )
            } else if grammar::words_match_any_prefix(tokens, TURN_PREFIXES).is_some() {
                (PlayerAst::Implicit, clause_words)
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported skip clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    };

    let skips_next_combat_phase_this_turn =
        word_slice_contains_all_words(&words, NEXT_COMBAT_PHASE_THIS_TURN_WORDS);
    if skips_next_combat_phase_this_turn {
        return Ok(EffectAst::subject_verb_skip_next_combat_phase_this_turn(
            player,
        ));
    }
    if word_slice_contains_all_words(&words, COMBAT_PHASE_TURN_WORDS)
        && word_slice_contains_any_word(&words, COMBAT_PHASE_WORDS)
    {
        return Ok(EffectAst::subject_verb_skip_combat_phases(player));
    }
    if word_slice_contains_all_words(&words, DRAW_STEP_WORDS) {
        return Ok(EffectAst::subject_verb_skip_draw_step(player));
    }
    if word_slice_contains_all_words(&words, TURN_WORDS) {
        return Ok(EffectAst::subject_verb_skip_turn(player));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported skip clause (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn parse_end(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let player = match subject.unwrap_or(SubjectAst::This) {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
    };

    if word_slice_eq_any(&clause_words, END_TURN_WORDS) {
        return Ok(EffectAst::subject_verb_end_turn(player));
    }
    if word_slice_eq(&clause_words, END_STEP_YOU_LOSE_WORDS) {
        return Ok(EffectAst::subject_verb_lose_game(PlayerAst::You));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported end clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_flip(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject.unwrap_or(SubjectAst::This) {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
    };
    if tokens.is_empty() {
        return Ok(EffectAst::subject_verb_flip(TargetAst::Source(None)));
    }

    if let Some(timed_tokens) = split_trailing_next_end_step_timing(tokens) {
        let timed_effect = parse_flip(timed_tokens, subject)?;
        return Ok(EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![timed_effect],
        });
    }

    let token_words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_eq_any(&token_words, COIN_WORDS) {
        return Ok(EffectAst::subject_verb_flip_coin(player));
    }
    if word_slice_eq_any(&token_words, SELF_FLIP_TARGET_WORDS) {
        return Ok(EffectAst::subject_verb_flip(TargetAst::Source(
            span_from_tokens(tokens),
        )));
    }

    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::subject_verb_flip(target))
}

fn split_trailing_next_end_step_timing(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let words = TokenWordView::new(tokens);
    let timing_phrases: &[&[&str]] = &[
        &["at", "the", "beginning", "of", "the", "next", "end", "step"],
        &["at", "the", "beginning", "of", "next", "end", "step"],
        &["at", "beginning", "of", "the", "next", "end", "step"],
        &["at", "beginning", "of", "next", "end", "step"],
    ];

    for phrase in timing_phrases {
        if words.len() < phrase.len() {
            continue;
        }
        let phrase_start = words.len() - phrase.len();
        if !words.slice_eq(phrase_start, phrase) {
            continue;
        }
        let token_start = words.token_index_for_word_index(phrase_start)?;
        let action_tokens = &tokens[..token_start];
        if !trim_commas(action_tokens).is_empty() {
            return Some(action_tokens);
        }
    }

    None
}

pub(crate) fn parse_roll(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_sided_die_word(word: &str) -> Option<u32> {
        let prefix = word.strip_suffix(DIE_SIDED_SUFFIX)?;
        crate::runtime_backend::util::parse_number_word_u32(prefix)
    }

    let player = match subject.unwrap_or(SubjectAst::This) {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
    };
    let mut die_tokens = tokens;
    if die_tokens.first().is_some_and(|token| {
        token
            .as_word()
            .is_some_and(|word| ARTICLE_WORDS.contains(&word))
    }) {
        die_tokens = &die_tokens[1..];
    }
    let Some(die_word) = die_tokens.first().and_then(OwnedLexToken::as_word) else {
        return Err(CardTextError::ParseError(
            "roll clause missing die size".to_string(),
        ));
    };
    let die_word = die_word.to_ascii_lowercase();
    let die_noun = die_tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .map(str::to_ascii_lowercase)
        .take(3)
        .collect::<Vec<_>>();
    let die_text = parse_roll_die_text(&die_noun);
    let Some(sides) = die_word
        .strip_prefix('d')
        .and_then(|sides| sides.parse::<u32>().ok())
        .or_else(|| {
            let has_die_noun = die_tokens
                .get(1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| DIE_WORDS.contains(&word));
            has_die_noun
                .then(|| parse_sided_die_word(&die_word))
                .flatten()
        })
        .or_else(|| {
            let has_sided_die_noun = die_tokens
                .get(1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| word == SIDED_WORD)
                && die_tokens
                    .get(2)
                    .and_then(OwnedLexToken::as_word)
                    .is_some_and(|word| DIE_WORDS.contains(&word));
            has_sided_die_noun
                .then(|| crate::runtime_backend::util::parse_number_word_u32(&die_word))
                .flatten()
        })
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported roll clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };
    Ok(EffectAst::subject_verb_roll_die_with_die_text(
        player, sides, die_text,
    ))
}

fn is_sided_die_size_word(word: &str) -> bool {
    word.ends_with(DIE_SIDED_SUFFIX)
}

fn parse_roll_die_text(die_noun: &[String]) -> Option<String> {
    let first = die_noun.first()?.as_str();
    let second = die_noun.get(1)?.as_str();
    if is_sided_die_size_word(first) && DIE_WORDS.contains(&second) {
        return Some(format!("{first} {second}"));
    }

    let third = die_noun.get(2)?.as_str();
    if second == SIDED_WORD && DIE_WORDS.contains(&third) {
        return Some(format!("{first}-sided {third}"));
    }
    None
}

pub(crate) fn parse_regenerate(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words
        .first()
        .is_some_and(|word| ALL_OR_EACH_WORDS.contains(word))
    {
        if tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "regenerate clause missing filter after each/all".to_string(),
            ));
        }
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::subject_verb_regenerate_all(filter));
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::subject_verb_regenerate(target))
}

pub(crate) fn parse_mill(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_trailing_for_each_count(tokens: &[OwnedLexToken]) -> Option<Value> {
        let word_view = TokenWordView::new(tokens);
        let words = word_view.word_refs();
        let mut start_idx = usize::from(
            words
                .first()
                .is_some_and(|word| CARD_OR_CARDS_WORDS.contains(word)),
        );
        if !word_slice_starts_with_any(&words[start_idx..], FOR_EACH_PREFIXES) {
            return None;
        }

        let after_each_start =
            if word_slice_starts_with(&words[start_idx..], FOR_EACH_EXPLICIT_PREFIX) {
                start_idx += 2;
                start_idx
            } else {
                start_idx += 1;
                start_idx
            };
        let after_each = &words[after_each_start..];
        if let Some(on_idx) = after_each
            .iter()
            .position(|word| *word == ON_WORD)
            .filter(|on_idx| *on_idx > 0)
        {
            let counter_words = &after_each[..on_idx];
            let reference = &after_each[on_idx + 1..];
            if counter_words
                .last()
                .is_some_and(|word| COUNTER_OR_COUNTERS_WORDS.contains(word))
                && word_slice_starts_with_any(reference, THIS_REFERENCE_PREFIXES)
            {
                let Some(counter_start) = word_view.token_index_for_word_index(after_each_start)
                else {
                    return None;
                };
                let Some(counter_end) =
                    word_view.token_index_for_word_index(after_each_start + on_idx)
                else {
                    return None;
                };
                if let Some(counter_type) =
                    parse_counter_type_from_tokens(&tokens[counter_start..counter_end])
                {
                    return Some(Value::CountersOnSource(counter_type));
                }
            }
        }

        let mut number_of_words = vec!["the", "number", "of"];
        number_of_words.extend_from_slice(after_each);
        if let Some((value, used)) =
            crate::runtime_backend::util::parse_value_expr_words(&number_of_words)
            && used == number_of_words.len()
        {
            return Some(value);
        }

        parse_get_for_each_count_value(tokens).ok().flatten()
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let starts_with_card_keyword = tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| CARD_OR_CARDS_WORDS.contains(&word));

    let subject_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let (mut count, used, count_includes_library_noun) =
        if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, THAT_MANY_PREFIXES) {
            (
                Value::EventValue(EventValueSpec::Amount),
                prefix.len(),
                false,
            )
        } else if starts_with_card_keyword {
            if let Some((count, used_after_cards)) = parse_value(&tokens[1..]) {
                (count, 1 + used_after_cards, false)
            } else if let Some((count, used_after_cards)) =
                parse_half_library_count_value(&clause_words[1..], subject_player)
            {
                let used_words = 1 + used_after_cards;
                let used = crate::runtime_backend::token_index_for_word_index(tokens, used_words)
                    .unwrap_or(tokens.len());
                (count, used, true)
            } else if let Some(count) = parse_add_mana_equal_amount_value(&tokens[1..]) {
                // Mill clauses like "cards equal to its toughness" place the amount after "cards".
                (count, tokens.len(), false)
            } else {
                return Err(CardTextError::ParseError(format!(
                    "missing mill count (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        } else if let Some((count, used)) =
            parse_half_library_count_value(&clause_words, subject_player)
        {
            let used = crate::runtime_backend::token_index_for_word_index(tokens, used)
                .unwrap_or(tokens.len());
            (count, used, true)
        } else {
            let (count, used) = parse_value(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing mill count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            (count, used, false)
        };

    let rest = &tokens[used..];
    if starts_with_card_keyword {
        let trailing_count_tokens = if rest
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| CARD_OR_CARDS_WORDS.contains(&word))
        {
            &rest[1..]
        } else {
            rest
        };
        let trailing_words: Vec<&str> = trailing_count_tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect();
        if !trailing_words.is_empty() {
            if word_slice_eq(&trailing_words, INSTEAD_WORDS) {
                // Conditional self-replacement lowering consumes the semantic
                // role of "instead" after the mill effect parses.
            } else if matches!(count, Value::Fixed(1))
                && let Some(for_each_count) = parse_trailing_for_each_count(trailing_count_tokens)
            {
                count = for_each_count;
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing mill clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    } else {
        if rest.is_empty() && !count_includes_library_noun {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        }
        if rest
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| !CARD_OR_CARDS_WORDS.contains(&word))
        {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        }
        let trailing_count_tokens = if rest.is_empty() { rest } else { &rest[1..] };
        let trailing_words: Vec<&str> = trailing_count_tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect();
        if !trailing_words.is_empty() {
            if word_slice_eq(&trailing_words, INSTEAD_WORDS) {
                // Conditional self-replacement lowering consumes the semantic
                // role of "instead" after the mill effect parses.
            } else if matches!(count, Value::Fixed(1))
                && let Some(for_each_count) = parse_trailing_for_each_count(trailing_count_tokens)
            {
                count = for_each_count;
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing mill clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    }

    Ok(subject_verb_player_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        subject_player,
        SubjectVerbActionAst::Mill { count },
    ))
}

pub(crate) fn parse_get(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let tokens = if tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| matches!(word, "get" | "gets"))
    {
        &tokens[1..]
    } else {
        tokens
    };

    fn parse_pump_for_each_tail(
        tail_tokens: &[OwnedLexToken],
        subject: Option<SubjectAst>,
        power_per: i32,
        toughness_per: i32,
        clause_words: &[&str],
    ) -> Result<Option<EffectAst>, CardTextError> {
        if grammar::words_match_prefix(tail_tokens, &["until", "end", "of", "turn", "for", "each"])
            .is_none()
        {
            return Ok(None);
        }

        let count = parse_get_for_each_count_value(&tail_tokens[4..])?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported get-for-each filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        Ok(Some(EffectAst::subject_verb_pump_for_each(
            power_per,
            toughness_per,
            target,
            count,
            Until::EndOfTurn,
        )))
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::contains_word(tokens, "poison")
        && (grammar::contains_word(tokens, "counter") || grammar::contains_word(tokens, "counters"))
    {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let count = if matches!(
            clause_words.first().copied(),
            Some("a" | "an" | "another" | "one")
        ) {
            Value::Fixed(1)
        } else {
            parse_value(tokens)
                .map(|(value, _)| value)
                .unwrap_or(Value::Fixed(1))
        };
        return Ok(EffectAst::subject_verb_poison_counters(player, count));
    }

    if grammar::contains_word(tokens, "experience")
        && (grammar::contains_word(tokens, "counter") || grammar::contains_word(tokens, "counters"))
    {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let count = if matches!(
            clause_words.first().copied(),
            Some("a" | "an" | "another" | "one")
        ) {
            Value::Fixed(1)
        } else {
            parse_value(tokens)
                .map(|(value, _)| value)
                .unwrap_or(Value::Fixed(1))
        };
        return Ok(EffectAst::subject_verb_experience_counters(player, count));
    }

    let energy_count = tokens
        .iter()
        .filter(|token| energy_symbol_token(token))
        .count();
    if energy_count > 0 {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let count = parse_add_mana_equal_amount_value(tokens)
            .or(parse_equal_to_number_of_filter_value(tokens))
            .or(parse_dynamic_cost_modifier_value(tokens)?)
            .or_else(|| parse_value(tokens).map(|(value, _)| value))
            .unwrap_or(Value::Fixed(energy_count as i32));
        return Ok(EffectAst::subject_verb_energy_counters(player, count));
    }

    let ticket_count = tokens
        .iter()
        .filter(|token| ticket_symbol_token(token))
        .count();
    if ticket_count > 0 {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        return Ok(EffectAst::subject_verb_ticket_counters(
            player,
            Value::Fixed(ticket_count as i32),
        ));
    }

    if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, EMBLEM_WITH_PREFIXES) {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let text_words = &clause_words[prefix.len()..];
        if text_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing emblem text (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let text = crate::runtime_backend::token_index_for_word_index(tokens, prefix.len())
            .and_then(|start| {
                let rendered = crate::runtime_backend::lexer::render_token_slice(&tokens[start..]);
                let rendered = rendered
                    .trim()
                    .trim_matches('"')
                    .trim_matches('“')
                    .trim_matches('”')
                    .trim()
                    .to_string();
                (!rendered.is_empty()).then_some(rendered)
            })
            .unwrap_or_else(|| {
                if word_slice_starts_with(text_words, BEGINNING_OF_PREFIX)
                    && let Some(this_idx) = find_index(&text_words, |word| *word == THIS_WORD)
                {
                    let head = text_words[..this_idx].join(" ");
                    let tail = text_words[this_idx..].join(" ");
                    format!(
                        "{}{}, {}.",
                        head[..1].to_ascii_uppercase(),
                        &head[1..],
                        tail
                    )
                } else {
                    let joined = text_words.join(" ");
                    format!("{}{}.", joined[..1].to_ascii_uppercase(), &joined[1..])
                }
            });
        let text = if text.ends_with(['.', '!', '?']) {
            text
        } else {
            format!("{text}.")
        };
        return Ok(EffectAst::subject_verb_create_emblem(player, text));
    }

    let modifier_start =
        if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, ADDITIONAL_PREFIXES) {
            prefix.len()
        } else {
            0usize
        };
    if modifier_start > 0
        && let Some(mod_token) = tokens.get(modifier_start).map(OwnedLexToken::parser_text)
        && let Ok((power_per, toughness_per)) = parse_pt_modifier(mod_token)
    {
        let tail_tokens = tokens.get(modifier_start + 1..).unwrap_or_default();
        if let Some(effect) = parse_pump_for_each_tail(
            tail_tokens,
            subject,
            power_per,
            toughness_per,
            &clause_words,
        )? {
            return Ok(effect);
        }
    }

    if let Some(mod_token) = tokens.first().map(OwnedLexToken::parser_text)
        && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
    {
        if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) = (&power, &toughness)
            && let Some(effect) = parse_pump_for_each_tail(
                tokens.get(1..).unwrap_or_default(),
                subject,
                *power_per,
                *toughness_per,
                &clause_words,
            )?
        {
            return Ok(effect);
        }
        let (power, toughness, duration, condition) =
            parse_get_modifier_values_with_tail(tokens, power, toughness)?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        return Ok(EffectAst::subject_verb_pump(
            power, toughness, target, duration, condition,
        ));
    }

    if let Some(collapsed_tokens) = collapse_leading_signed_pt_modifier_tokens(tokens)
        && let Some(mod_token) = collapsed_tokens.first().map(OwnedLexToken::parser_text)
        && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
    {
        if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) = (&power, &toughness)
            && let Some(effect) = parse_pump_for_each_tail(
                collapsed_tokens.get(1..).unwrap_or_default(),
                subject,
                *power_per,
                *toughness_per,
                &clause_words,
            )?
        {
            return Ok(effect);
        }
        let (power, toughness, duration, condition) =
            parse_get_modifier_values_with_tail(&collapsed_tokens, power, toughness)?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        return Ok(EffectAst::subject_verb_pump(
            power, toughness, target, duration, condition,
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported get clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_untap(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "untap clause missing target".to_string(),
        ));
    }
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words
        .first()
        .is_some_and(|word| ALL_OR_EACH_WORDS.contains(word))
    {
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::subject_verb_untap_all(filter));
    }
    if word_slice_eq(&words, THEM_WORDS) {
        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        return Ok(EffectAst::subject_verb_untap_all(filter));
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::subject_verb_untap(target))
}

pub(crate) fn parse_scry(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing scry count (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(subject_verb_player_effect(
        SubjectVerbRoleAst::Chooser,
        player,
        SubjectVerbActionAst::Scry { count },
    ))
}

pub(crate) fn parse_surveil(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing surveil count (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(subject_verb_player_effect(
        SubjectVerbRoleAst::Chooser,
        player,
        SubjectVerbActionAst::Surveil { count },
    ))
}

pub(crate) fn parse_pay(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let energy_symbol_count = tokens
        .iter()
        .filter(|token| energy_symbol_token(token))
        .count();

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::words_match_any_prefix(tokens, ANY_AMOUNT_OF_PREFIXES).is_some()
        && (grammar::contains_word(tokens, "e") || energy_symbol_count > 0)
    {
        return Ok(EffectAst::subject_verb_pay_any_energy(player, 0));
    }
    if grammar::words_match_any_prefix(tokens, ANY_AMOUNT_OF_PREFIXES).is_some()
        && grammar::contains_word(tokens, "life")
    {
        return Ok(EffectAst::subject_verb_pay_any_life(player, 0));
    }
    if grammar::words_match_any_prefix(tokens, &[&["one", "or", "more"]]).is_some()
        && (grammar::contains_word(tokens, "e") || energy_symbol_count > 0)
    {
        return Ok(EffectAst::subject_verb_pay_any_energy(player, 1));
    }
    if grammar::words_match_any_prefix(tokens, &[&["one", "or", "more"]]).is_some()
        && grammar::contains_word(tokens, "life")
    {
        return Ok(EffectAst::subject_verb_pay_any_life(player, 1));
    }
    if let Some(compound) = parse_compound_pay(tokens, player) {
        return Ok(compound);
    }
    let has_for_each = word_slice_contains_phrase(&clause_words, FOR_EACH_PHRASE);
    let references_tagged_choice = clause_words
        .iter()
        .any(|word| THOSE_OR_THEM_WORDS.contains(word))
        || word_slice_contains_phrase(&clause_words, CHOSEN_THIS_WAY_PHRASE);
    let repeats_for_tagged_choice = has_for_each && references_tagged_choice;

    if repeats_for_tagged_choice {
        let repeated_pips = {
            use winnow::prelude::*;
            let mut stream = LexStream::new(tokens);
            grammar::collect_mana_pip_groups
                .parse_next(&mut stream)
                .ok()
                .unwrap_or_default()
        };
        if !repeated_pips.is_empty() {
            return Ok(EffectAst::ForEachTagged {
                tag: TagKey::from(IT_TAG),
                effects: vec![EffectAst::subject_verb_pay_mana(
                    player,
                    ManaCost::from_pips(repeated_pips),
                )],
            });
        }
    }

    if clause_words.len() >= 4
        && grammar::contains_word(tokens, "for")
        && grammar::contains_word(tokens, "each")
        && let Ok(symbols) = parse_mana_symbol_group(clause_words[0])
    {
        return Ok(EffectAst::subject_verb_pay_mana(
            player,
            ManaCost::from_pips(vec![symbols]),
        ));
    }

    if let Some((amount, used)) = parse_value(tokens)
        && token_slice_at_is(tokens, used, "life")
    {
        return Ok(subject_verb_player_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        ));
    }
    if let Some((amount, used)) = parse_value(tokens)
        && tokens
            .get(used)
            .is_some_and(|token| token.as_word().is_some_and(|word| word == ENERGY_TEXT_WORD))
    {
        return Ok(EffectAst::subject_verb_pay_energy(player, amount));
    }
    if energy_symbol_count > 0 {
        let mut energy_count = 0u32;
        for token in tokens {
            if energy_symbol_token(token) {
                energy_count += 1;
                continue;
            }
            let Some(word) = token.as_word() else {
                continue;
            };
            if is_article(word) || ENERGY_COUNTER_PAY_IGNORED_WORDS.contains(&word) {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported pay clause token '{word}' (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        if energy_count > 0 {
            return Ok(EffectAst::subject_verb_pay_energy(
                player,
                Value::Fixed(energy_count as i32),
            ));
        }
    }

    let pips = {
        use winnow::prelude::*;
        let mut stream = LexStream::new(tokens);
        grammar::collect_mana_pip_groups
            .parse_next(&mut stream)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "missing payment cost (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?
    };

    Ok(EffectAst::subject_verb_pay_mana(
        player,
        ManaCost::from_pips(pips),
    ))
}
