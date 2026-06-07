use super::*;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use crate::runtime_backend::effect_sentences::find_verb_words;
use crate::runtime_backend::lex_patterns::{LexCaptureKind, LexPattern, LexPatternAtom};

const TARGET_WORD: &str = "target";
const THAT_WORD: &str = "that";
const THE_WORD: &str = "the";
const YOU_WORD: &str = "you";
const VOTER_WORD: &str = "voter";
const PLAYER_OR_PLAYERS_WORDS: &[&str] = &["player", "players"];
const PLAYER_WORD: &str = "player";
const OPPONENT_OR_OPPONENTS_WORDS: &[&str] = &["opponent", "opponents"];
const PLAYER_OR_OPPONENT_WORDS: &[&str] = &["player", "opponent"];
const PUT_OR_PUTS_WORDS: &[&str] = &["put", "puts"];
const LEADING_ARTICLE_WORDS: &[&str] = &["a", "an", "the"];
const SECOND_WORD: &str = "second";
const THIRD_WORD: &str = "third";
const CHOOSE_WORDS: &[&str] = &["choose", "chooses"];
const FOR_WORD: &str = "for";
const EACH_WORD: &str = "each";
const CHOICE_CONNECTOR_WORDS: &[&str] = &["or", "and"];
const CREATURE_TYPE_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["creature", "type"]);
const OTHER_THAN_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["other", "than"]);
const COLOR_WORD: &str = "color";
const PERMANENT_TYPE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["permanent", "type"], &["permanent", "types"]]);
const BASIC_LAND_TYPE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["basic", "land", "type"]);
const LAND_TYPE_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["land", "type"]);
const BECOME_OR_BECOMES_WORDS: &[&str] = &["become", "becomes"];
const THAT_TYPE_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that", "type"]);
const EACH_OR_ALL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["each"], &["all"]]);
const AND_WORD: &str = "and";
const ON_WORD: &str = "on";
const TOP_OF_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["top", "of"]);
const LIBRARY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["library"]);
const TAGGED_CHOICE_LIBRARY_MOVED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"], &["those"], &["those", "cards"],]);
const THEN_WORD: &str = "then";
const YOU_PUT_OR_PUTS_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you", "put"], &["you", "puts"]]);
const ONTO_WORD: &str = "onto";
const BATTLEFIELD_WORD: &str = "battlefield";
const TAPPED_WORD: &str = "tapped";
const GRAVEYARD_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["graveyard", "graveyards"]]);
const HAND_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["hand", "hands"]]);
const FROM_OR_IN_IT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["from", "it"],
            &["from", "them"],
            &["in", "it"],
            &["in", "them"],
        ]
);
const FROM_THERE_IN_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["from", "there", "in"]);
const FROM_OR_IN_TAGGED_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["from", "it"],
            &["from", "them"],
            &["in", "it"],
            &["in", "them"],
        ]
);
const OF_TAGGED_CHOICE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["of", "them"], &["of", "those"]]);
const OF_TAGGED_REFERENCE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["of", "them"], &["of", "those"]]);
const OF_TAGGED_CARDS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["of", "those", "card"], &["of", "those", "cards"]]);
const FOR_EACH_CARD_DISCARDED_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["for", "each", "card", "discarded", "this", "way"],
            &["for", "each", "cards", "discarded", "this", "way"]
        ]
);
const CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const OPTIONAL_THE_ATOMS: &[LexPatternAtom<'static>] = &[LexPattern::word("the")];
const ONE_OR_MORE_PHRASES: &[&[&str]] = &[&["one", "or", "more"]];
const CHOSEN_PLAYER_MOST_LIFE_TIED_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("with"),
    LexPattern::optional(OPTIONAL_THE_ATOMS),
    LexPattern::phrase(&["most", "life", "or", "tied", "for", "most", "life"]),
]);
const CHOSEN_PLAYER_CAST_TYPE_THIS_TURN_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::any_word(&["who", "that"]),
    LexPattern::word("cast"),
    LexPattern::amount("amount", LexCaptureKind::OneOfPhrase(ONE_OR_MORE_PHRASES)),
    LexPattern::object("card_type", LexCaptureKind::WordCount(1)),
    LexPattern::any_word(&["spell", "spells"]),
    LexPattern::phrase(&["this", "turn"]),
]);
const CHOSEN_OBJECT_CANT_BLOCK_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["block", "this", "turn"], &["block"]]);
const AT_RANDOM_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["at", "random"]);
const TAGGED_CHOICE_MOVED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["that", "card"], &["that", "permanent"]]);
const BATTLEFIELD_UNDER_YOUR_CONTROL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["under", "your", "control"]);
const BATTLEFIELD_UNDER_OWNER_CONTROL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["under", "its", "owners", "control"],
            &["under", "their", "owners", "control"],
            &["under", "that", "players", "control"],
        ]
);
const CARD_TYPE_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["card", "type"]);
const THEN_REVEAL_TOP_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["then", "reveal", "the", "top"]);
const OF_YOUR_LIBRARY_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["of", "your", "library"]);
const PUT_OR_PUTS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["put"], &["puts"]]);
const CHOSEN_TYPE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["chosen", "type"]]);
const REVEALED_THIS_WAY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["revealed", "this", "way"]]);
const INTO_YOUR_HAND_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["into", "your", "hand"]]);
const BOTTOM_OF_YOUR_LIBRARY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["bottom", "of", "your", "library"]]);
const OTHER_OR_ANOTHER_WORDS: &[&str] = &["other", "another"];

fn choice_object_shape_matches_words<'a>(words: &[&str], shape: ClauseShape<'a>) -> bool {
    shape.matches_word_slice(words)
}

fn choice_word_is_any(word: &str, expected: &[&str]) -> bool {
    expected.contains(&word)
}

fn choice_word_is(word: &str, expected: &str) -> bool {
    choice_word_is_any(word, &[expected])
}

fn choice_word_at_is_any(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    words
        .get(idx)
        .is_some_and(|word| choice_word_is_any(word, expected))
}

fn choice_word_at_is(words: &[&str], idx: usize, expected: &str) -> bool {
    choice_word_at_is_any(words, idx, &[expected])
}

fn choice_token_word_is_any(token: &OwnedLexToken, expected: &[&str]) -> bool {
    token
        .as_word()
        .is_some_and(|_| choice_word_is_any(token.parser_text(), expected))
}

fn choice_token_word_is(token: &OwnedLexToken, expected: &str) -> bool {
    choice_token_word_is_any(token, &[expected])
}

fn find_choice_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    find_index(tokens, |token| choice_token_word_is(token, expected))
}

fn find_choice_token_word_any(tokens: &[OwnedLexToken], expected: &[&str]) -> Option<usize> {
    find_index(tokens, |token| choice_token_word_is_any(token, expected))
}

fn strip_chosen_player_prefix(words: &mut Vec<&str>) -> usize {
    let mut exclude_previous_choices = 0usize;
    while let Some(word) = words.first().copied() {
        if choice_word_is_any(word, LEADING_ARTICLE_WORDS) {
            *words = words[1..].to_vec();
        } else if choice_word_is_any(word, OTHER_OR_ANOTHER_WORDS) {
            exclude_previous_choices = exclude_previous_choices.max(1);
            *words = words[1..].to_vec();
        } else if choice_word_is(word, SECOND_WORD) {
            exclude_previous_choices = exclude_previous_choices.max(1);
            *words = words[1..].to_vec();
        } else if choice_word_is(word, THIRD_WORD) {
            exclude_previous_choices = exclude_previous_choices.max(2);
            *words = words[1..].to_vec();
        } else {
            break;
        }
    }
    exclude_previous_choices
}

fn parse_chosen_player_base_filter(words: &mut Vec<&str>) -> Option<Option<PlayerFilter>> {
    let first = words.first().copied()?;
    if choice_word_is(first, PLAYER_WORD) {
        *words = words[1..].to_vec();
        Some(None)
    } else if choice_word_is_any(first, OPPONENT_OR_OPPONENTS_WORDS) {
        *words = words[1..].to_vec();
        Some(Some(PlayerFilter::Opponent))
    } else {
        None
    }
}

fn parse_chosen_player_filter_tail(words: &[&str]) -> Option<PlayerFilter> {
    if words.is_empty() {
        Some(PlayerFilter::Any)
    } else if CHOSEN_PLAYER_MOST_LIFE_TIED_PATTERN
        .match_word_refs(words)
        .is_some()
    {
        Some(PlayerFilter::MostLifeTied)
    } else if let Some(matched) = CHOSEN_PLAYER_CAST_TYPE_THIS_TURN_PATTERN.match_word_refs(words) {
        let card_type_range = matched.capture_word_range("card_type")?;
        let [card_type_word] = words.get(card_type_range)? else {
            return None;
        };
        parse_card_type(card_type_word).map(PlayerFilter::CastCardTypeThisTurn)
    } else {
        None
    }
}

fn expand_graveyard_or_hand_disjunction_filter(
    mut filter: ObjectFilter,
    words: &[&str],
) -> ObjectFilter {
    let has_graveyard = choice_object_shape_matches_words(words, GRAVEYARD_WORD_PATTERN);
    let has_hand = choice_object_shape_matches_words(words, HAND_WORD_PATTERN);
    if !(has_graveyard && has_hand) {
        return filter;
    }

    filter.zone = None;
    filter.controller = None;
    filter.any_of = vec![
        ObjectFilter {
            zone: Some(Zone::Graveyard),
            ..ObjectFilter::default()
        },
        ObjectFilter {
            zone: Some(Zone::Hand),
            ..ObjectFilter::default()
        },
    ];
    filter
}

fn parse_choose_objects_for_each_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if choice_object_shape_matches_words(&words, FOR_EACH_CARD_DISCARDED_THIS_WAY_PATTERN) {
        Some(Value::Count(ObjectFilter::tagged(TagKey::from(IT_TAG))))
    } else {
        None
    }
}

pub(crate) fn parse_target_player_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let (mut chooser, choose_start_idx) =
        if choice_word_at_is(&clause_words, 0, TARGET_WORD) && clause_words.len() >= 4 {
            let chooser = match clause_words.get(1).copied() {
                Some("player") => PlayerAst::Target,
                Some("opponent") | Some("opponents") => PlayerAst::TargetOpponent,
                _ => return Ok(None),
            };
            if !choice_word_at_is_any(&clause_words, 2, CHOOSE_WORDS) {
                return Ok(None);
            }
            (chooser, 3usize)
        } else if clause_words.len() >= 4
            && choice_word_at_is(&clause_words, 0, THAT_WORD)
            && choice_word_at_is_any(&clause_words, 1, PLAYER_OR_PLAYERS_WORDS)
            && choice_word_at_is_any(&clause_words, 2, CHOOSE_WORDS)
        {
            (PlayerAst::That, 3usize)
        } else if clause_words.len() >= 4
            && choice_word_at_is(&clause_words, 0, THE_WORD)
            && choice_word_at_is(&clause_words, 1, VOTER_WORD)
            && choice_word_at_is_any(&clause_words, 2, CHOOSE_WORDS)
        {
            (PlayerAst::That, 3usize)
        } else {
            return Ok(None);
        };

    let mut choose_object_tokens = trim_commas(&tokens[choose_start_idx..]);
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object after target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (count, stripped_choose_object_tokens) =
        parse_choice_count_token_prefix(&choose_object_tokens);
    choose_object_tokens = stripped_choose_object_tokens;
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object filter after count in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let choose_object_words = crate::runtime_backend::token_word_refs(&choose_object_tokens);
    if choice_word_at_is(&choose_object_words, 0, TARGET_WORD)
        && choice_word_at_is_any(&choose_object_words, 1, PLAYER_OR_OPPONENT_WORDS)
    {
        return Ok(None);
    }
    if find_verb(&choose_object_tokens).is_some() {
        return Ok(None);
    }

    let mut choose_filter = parse_object_filter(&choose_object_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported chosen object filter in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    choose_filter = expand_graveyard_or_hand_disjunction_filter(choose_filter, &clause_words);
    if chooser == PlayerAst::That
        && choose_filter.controller.is_none()
        && choose_filter.owner.is_none()
        && choose_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
    {
        chooser = PlayerAst::ItsController;
    }
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            PlayerAst::ItsController => {
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG))
            }
            _ => PlayerFilter::target_player(),
        });
    }

    Ok(Some((chooser, choose_filter, count)))
}

pub(crate) fn parse_you_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    Ok(parse_you_choose_objects_clause_with_count_value(tokens)?
        .map(|(chooser, filter, count, _count_value)| (chooser, filter, count)))
}

pub(crate) fn parse_you_choose_objects_clause_with_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount, Option<Value>)>, CardTextError> {
    let trimmed_tokens = trim_edge_punctuation(tokens);
    let tokens = trimmed_tokens.as_slice();
    let clause_words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let choose_word_idx = if choice_word_at_is(&clause_words, 0, YOU_WORD) {
        1usize
    } else {
        0usize
    };
    if !clause_words
        .get(choose_word_idx)
        .is_some_and(|word| choice_word_is_any(word, CHOOSE_WORDS))
    {
        return Ok(None);
    }

    let choose_word_token_idx =
        token_index_for_word_index(tokens, choose_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing choose keyword in choose clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let mut choose_object_tokens = trim_commas(&tokens[choose_word_token_idx + 1..]).to_vec();
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object after choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut count_value = None;
    let for_each_idx = (0..choose_object_tokens.len().saturating_sub(1)).find(|idx| {
        choice_token_word_is(&choose_object_tokens[*idx], FOR_WORD)
            && choice_token_word_is(&choose_object_tokens[*idx + 1], EACH_WORD)
    });
    if let Some(for_each_idx) = for_each_idx {
        let count_tokens = trim_commas(&choose_object_tokens[for_each_idx..]);
        if let Some(value) = parse_choose_objects_for_each_count_value(&count_tokens) {
            count_value = Some(value);
            choose_object_tokens.truncate(for_each_idx);
        }
    }

    let mut references_it = false;
    let mut references_container_it = false;
    let mut explicit_container_reference = false;
    loop {
        let choose_object_words = crate::runtime_backend::token_word_refs(&choose_object_tokens);
        if choice_object_shape_matches_words(&choose_object_words, FROM_OR_IN_IT_TAIL_PATTERN) {
            references_it = true;
            references_container_it = true;
            explicit_container_reference = true;
            choose_object_tokens.truncate(choose_object_tokens.len().saturating_sub(2));
            continue;
        }
        if choice_object_shape_matches_words(&choose_object_words, FROM_THERE_IN_TAIL_PATTERN) {
            references_it = true;
            references_container_it = true;
            explicit_container_reference = true;
            choose_object_tokens.truncate(choose_object_tokens.len().saturating_sub(3));
            continue;
        }
        break;
    }
    let mut choose_words =
        crate::runtime_backend::lexer::parser_token_word_refs(&choose_object_tokens);
    loop {
        if choice_object_shape_matches_words(&choose_words, FROM_OR_IN_IT_TAIL_PATTERN) {
            references_it = true;
            references_container_it = true;
            explicit_container_reference = true;
            choose_words.truncate(choose_words.len().saturating_sub(2));
            continue;
        }
        if choice_object_shape_matches_words(&choose_words, FROM_THERE_IN_TAIL_PATTERN) {
            references_it = true;
            references_container_it = true;
            explicit_container_reference = true;
            choose_words.truncate(choose_words.len().saturating_sub(3));
            continue;
        }
        break;
    }
    let mut count = ChoiceCount::exactly(1);
    if let Some((parsed_count, used)) = parse_choice_count_word_prefix(&choose_words) {
        count = parsed_count;
        choose_words = choose_words[used..].to_vec();
    } else if choose_words
        .first()
        .is_some_and(|word| choice_word_is_any(word, LEADING_ARTICLE_WORDS))
    {
        choose_words = choose_words[1..].to_vec();
    }
    let mut idx = 0usize;
    while idx + 1 < choose_words.len() {
        if choose_words[idx] == "at" && choose_words[idx + 1] == "random" {
            count = count.at_random();
            choose_words.drain(idx..idx + 2);
            continue;
        }
        idx += 1;
    }
    if count_value.is_some() {
        count = ChoiceCount::dynamic_x();
    }

    if choose_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object filter in choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if choose_words.ends_with(&["card", "name"]) {
        return Ok(None);
    }
    if choice_object_shape_matches_words(&choose_words, OF_TAGGED_CHOICE_PATTERN) {
        references_it = true;
        references_container_it = true;
        choose_words = vec!["card"];
    } else if choose_words.len() > 2
        && choice_object_shape_matches_words(&choose_words, OF_TAGGED_REFERENCE_PREFIX_PATTERN)
    {
        references_it = true;
        if choice_object_shape_matches_words(&choose_words, OF_TAGGED_CARDS_PATTERN) {
            references_container_it = true;
        }
        choose_words = choose_words[2..].to_vec();
    }
    let mut idx = 0usize;
    while idx + 1 < choose_words.len() {
        if choice_object_shape_matches_words(
            &choose_words[idx..idx + 2],
            FROM_OR_IN_TAGGED_REFERENCE_PATTERN,
        ) {
            references_it = true;
            references_container_it = true;
            explicit_container_reference = true;
            choose_words.drain(idx..idx + 2);
            continue;
        }
        idx += 1;
    }

    let controller_tail = crate::runtime_backend::object_filters::parse_simple_object_filter_words(
        &choose_words,
        false,
    )
    .is_some_and(|filter| filter.controller.is_some());
    if find_verb_words(&choose_words).is_some() && !controller_tail {
        return Ok(None);
    }

    let mut choose_filter = if references_it
        && choose_words.len() == 1
        && choice_word_is_any(choose_words[0], CARD_OR_CARDS_WORDS)
    {
        ObjectFilter::default()
    } else {
        crate::runtime_backend::object_filters::parse_object_filter_words(&choose_words, false)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported chosen object filter in choose clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
    };
    choose_filter = expand_graveyard_or_hand_disjunction_filter(choose_filter, &choose_words);
    if references_it {
        if explicit_container_reference
            && matches!(choose_filter.zone, None | Some(Zone::Battlefield))
        {
            choose_filter.zone = Some(Zone::Hand);
        } else if references_container_it && choose_filter.zone.is_none() {
            choose_filter.zone = Some(Zone::Hand);
        }
        if !choose_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
        {
            choose_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
        }
    }
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    let chooser = if choose_word_idx == 0 {
        PlayerAst::Implicit
    } else {
        PlayerAst::You
    };

    if references_it {
        choose_filter.controller = None;
        choose_filter.owner = None;
    } else if chooser == PlayerAst::You
        && choose_filter.controller.is_none()
        && choose_filter.owner.is_none()
        && choose_filter.could_be_targeted_by.is_none()
    {
        choose_filter.controller = Some(PlayerFilter::You);
    }

    Ok(Some((chooser, choose_filter, count, count_value)))
}

pub(crate) fn parse_you_choose_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, PlayerFilter, bool, usize)>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let choose_word_idx = if choice_word_at_is(&clause_words, 0, YOU_WORD) {
        1usize
    } else {
        0usize
    };
    if !clause_words
        .get(choose_word_idx)
        .is_some_and(|word| choice_word_is_any(word, CHOOSE_WORDS))
    {
        return Ok(None);
    }

    let choose_word_token_idx =
        token_index_for_word_index(tokens, choose_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing choose keyword in choose-player clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let player_tokens = trim_commas(&tokens[choose_word_token_idx + 1..]);
    let mut player_words = crate::runtime_backend::token_word_refs(&player_tokens);
    if player_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen player in choose-player clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let exclude_previous_choices = strip_chosen_player_prefix(&mut player_words);

    let mut filter = match parse_chosen_player_base_filter(&mut player_words) {
        Some(filter) => filter,
        None => return Ok(None),
    };

    let mut random = false;
    if choice_object_shape_matches_words(&player_words, AT_RANDOM_PREFIX_PATTERN) {
        random = true;
        player_words = player_words[2..].to_vec();
    }

    let filter = if let Some(filter) = filter.take() {
        if player_words.is_empty() {
            filter
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported chosen player filter in choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    } else {
        if let Some(filter) = parse_chosen_player_filter_tail(&player_words) {
            filter
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported chosen player filter in choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };

    Ok(Some((
        PlayerAst::You,
        filter,
        random,
        exclude_previous_choices,
    )))
}

pub(crate) fn parse_target_player_chooses_then_other_cant_block(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, mut choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(first)?
    else {
        return Ok(None);
    };
    if choose_filter.card_types.is_empty() {
        choose_filter.card_types.push(CardType::Creature);
    }

    let second_words = crate::runtime_backend::token_word_refs(second);
    let Some((neg_start, neg_end)) = find_negation_span(second) else {
        return Ok(None);
    };
    let tail_words_storage = normalize_cant_words(&second[neg_end..]);
    let tail_words = tail_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !choice_object_shape_matches_words(&tail_words, CHOSEN_OBJECT_CANT_BLOCK_TAIL_PATTERN) {
        return Ok(None);
    }

    let mut subject_tokens = trim_commas(&second[..neg_start]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing subject in cant-block clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let mut exclude_tagged_choice = false;
    if subject_tokens
        .first()
        .is_some_and(|token| choice_token_word_is_any(token, OTHER_OR_ANOTHER_WORDS))
    {
        exclude_tagged_choice = true;
        subject_tokens = trim_commas(&subject_tokens[1..]);
    }
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in cant-block clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let mut restriction_filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported cant-block subject filter (clause: '{}')",
            second_words.join(" ")
        ))
    })?;
    if restriction_filter.card_types.is_empty() {
        restriction_filter.card_types.push(CardType::Creature);
    }
    if restriction_filter.controller.is_none() {
        restriction_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            _ => PlayerFilter::target_player(),
        });
    }
    if exclude_tagged_choice
        && !restriction_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            })
    {
        restriction_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_cant(
            crate::effect::Restriction::block(restriction_filter),
            Until::EndOfTurn,
            None,
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::super::super::util::tokenize_line;
    use super::*;
    use crate::effect::Restriction;
    use crate::zone::Zone;

    #[test]
    fn parse_negated_object_restriction_clause_supports_attack_or_block_alone() {
        let tokens = tokenize_line("This creature can't attack or block alone.", 0);

        let parsed = parse_negated_object_restriction_clause(&tokens)
            .expect("parse attack-or-block-alone restriction")
            .expect("expected restriction");

        assert!(matches!(
            parsed.restriction,
            Restriction::AttackOrBlockAlone(_)
        ));
    }

    #[test]
    fn parse_negated_object_restriction_clause_supports_activated_abilities_of_that_permanent() {
        let tokens = tokenize_line(
            "Activated abilities of that permanent can't be activated.",
            0,
        );

        let parsed = parse_negated_object_restriction_clause(&tokens)
            .expect("parse activated-abilities restriction")
            .expect("expected restriction");

        assert!(matches!(
            parsed.restriction,
            Restriction::ActivateAbilitiesOf(_)
        ));
    }

    #[test]
    fn parse_you_choose_objects_clause_supports_bare_card_from_it() {
        let tokens = tokenize_line("You choose a card from it.", 0);

        let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
            .expect("parse choose-a-card-from-it clause")
            .expect("expected choose clause");

        assert_eq!(chooser, PlayerAst::You);
        assert_eq!(count, ChoiceCount::exactly(1));
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG),
            "expected hand choice to stay tied to the prior revealed hand, got {filter:?}"
        );
        assert!(
            filter.controller.is_none(),
            "expected no controller pin, got {filter:?}"
        );
        assert!(
            filter.owner.is_none(),
            "expected no owner pin, got {filter:?}"
        );
    }

    #[test]
    fn parse_you_choose_objects_clause_supports_card_from_it_with_filter_tail() {
        let tokens = tokenize_line("You choose a card from it with mana value 4 or greater.", 0);

        let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
            .expect("parse choose-a-card-from-it-with-filter-tail clause")
            .expect("expected choose clause");

        assert_eq!(chooser, PlayerAst::You);
        assert_eq!(count, ChoiceCount::exactly(1));
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG),
            "expected hand choice to stay tied to the prior revealed hand, got {filter:?}"
        );
        assert!(
            filter.controller.is_none(),
            "expected no controller pin, got {filter:?}"
        );
        assert!(
            filter.owner.is_none(),
            "expected no owner pin, got {filter:?}"
        );
    }

    #[test]
    fn parse_you_choose_objects_clause_supports_opponent_graveyard_or_hand() {
        let tokens = tokenize_line(
            "You choose a nonland card from that player's graveyard or hand.",
            0,
        );

        let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
            .expect("parse choose from graveyard-or-hand clause")
            .expect("expected choose clause");

        assert_eq!(chooser, PlayerAst::You);
        assert_eq!(count, ChoiceCount::exactly(1));
        assert_eq!(filter.zone, None);
        assert_eq!(filter.controller, None);
        assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
        assert!(filter.excluded_card_types.contains(&CardType::Land));
        assert_eq!(filter.any_of.len(), 2);
        assert!(
            filter
                .any_of
                .iter()
                .any(|arm| arm.zone == Some(Zone::Graveyard))
        );
        assert!(filter.any_of.iter().any(|arm| arm.zone == Some(Zone::Hand)));
    }

    #[test]
    fn parse_you_choose_objects_clause_container_reference_overrides_permanent_default() {
        let tokens = tokenize_line("You choose an artifact or creature card from it.", 0);

        let (_chooser, filter, _count) = parse_you_choose_objects_clause(&tokens)
            .expect("parse choose-artifact-or-creature-card-from-it clause")
            .expect("expected choose clause");

        assert_eq!(filter.zone, Some(Zone::Hand));
        assert!(
            filter.controller.is_none(),
            "expected no battlefield controller default, got {filter:?}"
        );
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG),
            "expected hand choice to stay tied to the prior revealed hand, got {filter:?}"
        );
    }

    #[test]
    fn parse_you_choose_objects_clause_supports_one_of_them() {
        let tokens = tokenize_line("You choose one of them.", 0);

        let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
            .expect("parse choose-one-of-them clause")
            .expect("expected choose clause");

        assert_eq!(chooser, PlayerAst::You);
        assert_eq!(count, ChoiceCount::exactly(1));
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG),
            "expected one-of-them choice to reference the previous object set, got {filter:?}"
        );
        assert!(
            filter.controller.is_none() && filter.owner.is_none(),
            "expected referenced choice not to default to your permanent, got {filter:?}"
        );
    }

    #[test]
    fn parse_bare_choose_objects_clause_keeps_implicit_chooser() {
        let tokens = tokenize_line("Choose an artifact.", 0);

        let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
            .expect("parse bare choose-artifact clause")
            .expect("expected choose clause");

        assert_eq!(chooser, PlayerAst::Implicit);
        assert_eq!(count, ChoiceCount::exactly(1));
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
        assert!(
            filter.controller.is_none(),
            "implicit choose should let lowering bind the controller to the chooser, got {filter:?}"
        );
    }

    #[test]
    fn parse_that_player_chooses_one_of_those_uses_last_object_controller() {
        let tokens = tokenize_line("That player chooses one of those creatures.", 0);

        let (chooser, filter, count) = parse_target_player_choose_objects_clause(&tokens)
            .expect("parse that-player chooses one-of-those clause")
            .expect("expected target-player choose clause");

        assert_eq!(chooser, PlayerAst::ItsController);
        assert_eq!(count, ChoiceCount::exactly(1));
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject),
            "expected one-of-those choice to stay tied to tagged objects, got {filter:?}"
        );
    }

    #[test]
    fn parse_you_choose_player_clause_supports_choose_an_opponent() {
        let tokens = tokenize_line("Choose an opponent.", 0);

        let (chooser, filter, random, exclude_previous_choices) =
            parse_you_choose_player_clause(&tokens)
                .expect("parse choose-an-opponent clause")
                .expect("expected choose-player clause");

        assert_eq!(chooser, PlayerAst::You);
        assert_eq!(filter, PlayerFilter::Opponent);
        assert!(!random);
        assert_eq!(exclude_previous_choices, 0);
    }

    #[test]
    fn parse_chosen_player_filter_tail_uses_captured_cast_type_shape() {
        assert_eq!(
            parse_chosen_player_filter_tail(&[
                "who", "cast", "one", "or", "more", "sorcery", "spells", "this", "turn"
            ]),
            Some(PlayerFilter::CastCardTypeThisTurn(CardType::Sorcery))
        );
        assert_eq!(
            parse_chosen_player_filter_tail(&[
                "that", "cast", "one", "or", "more", "creature", "spells", "this", "turn"
            ]),
            Some(PlayerFilter::CastCardTypeThisTurn(CardType::Creature))
        );
    }

    #[test]
    fn parse_choose_card_type_phrase_words_supports_limited_type_lists() {
        let parsed =
            parse_choose_card_type_phrase_words(&["choose", "artifact", "creature", "or", "land"])
                .expect("limited choose-card-type phrase should parse")
                .expect("expected choose-card-type phrase");

        assert_eq!(
            parsed,
            (
                5,
                vec![CardType::Artifact, CardType::Creature, CardType::Land]
            )
        );
    }

    #[test]
    fn parse_choose_card_type_phrase_words_supports_permanent_types() {
        let parsed = parse_choose_card_type_phrase_words(&["choose", "a", "permanent", "type"])
            .expect("permanent-type choice phrase should parse")
            .expect("expected choose-card-type phrase");

        assert_eq!(
            parsed,
            (
                4,
                vec![
                    CardType::Artifact,
                    CardType::Creature,
                    CardType::Enchantment,
                    CardType::Land,
                    CardType::Planeswalker,
                    CardType::Battle,
                ]
            )
        );
    }

    #[test]
    fn parse_cant_restriction_clause_supports_that_player_cant_cast_spells() {
        let tokens = tokenize_line("That player can't cast spells.", 0);

        let parsed = parse_cant_restriction_clause(&tokens)
            .expect("parse that-player cant-cast clause")
            .expect("expected cant restriction");

        assert_eq!(
            parsed.restriction,
            Restriction::cast_spells(PlayerFilter::IteratedPlayer)
        );
    }
}

pub(crate) fn parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_words = crate::runtime_backend::token_word_refs(first);
    let Some(mut idx) = find_index(&first_words, |word| choice_word_is_any(word, CHOOSE_WORDS))
    else {
        return Ok(None);
    };
    idx += 1;
    if word_refs_at_is_article(&first_words, idx) {
        idx += 1;
    }
    if !choice_object_shape_matches_words(&first_words[idx..], CARD_TYPE_PATTERN) {
        return Ok(None);
    }
    idx += 2;

    let reveal_words = &first_words[idx..];
    if !choice_object_shape_matches_words(reveal_words, THEN_REVEAL_TOP_PREFIX_PATTERN) {
        return Ok(None);
    }
    let reveal_count_words = &reveal_words[4..];
    let (count, used) = crate::runtime_backend::util::parse_number_word_refs(reveal_count_words)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing reveal count in choose-card-type reveal clause (clause: '{}')",
                first_words.join(" ")
            ))
        })?;
    if reveal_count_words
        .get(used)
        .is_none_or(|word| !choice_word_is_any(word, CARD_OR_CARDS_WORDS))
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in choose-card-type reveal clause (clause: '{}')",
            first_words.join(" ")
        )));
    }
    let reveal_tail = &reveal_count_words[used + 1..];
    if !choice_object_shape_matches_words(&reveal_tail, OF_YOUR_LIBRARY_TAIL_PATTERN) {
        return Ok(None);
    }

    let second_words = crate::runtime_backend::token_word_refs(second);
    if !choice_object_shape_matches_words(&second_words, PUT_OR_PUTS_PREFIX_PATTERN) {
        return Ok(None);
    }
    let has_chosen_type =
        choice_object_shape_matches_words(&second_words, CHOSEN_TYPE_WORD_PATTERN);
    let has_revealed_this_way =
        choice_object_shape_matches_words(&second_words, REVEALED_THIS_WAY_WORD_PATTERN);
    let has_into_your_hand =
        choice_object_shape_matches_words(&second_words, INTO_YOUR_HAND_WORD_PATTERN);
    let has_bottom_of_library =
        choice_object_shape_matches_words(&second_words, BOTTOM_OF_YOUR_LIBRARY_WORD_PATTERN);
    if !has_chosen_type || !has_revealed_this_way || !has_into_your_hand || !has_bottom_of_library {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_reveal_top_choose_card_type_put_to_hand_rest_bottom(
            PlayerAst::You,
            count,
        ),
    ]))
}

pub(crate) fn parse_choose_creature_type_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Vec<Subtype>)>, CardTextError> {
    let Some(mut idx) = parse_choose_phrase_prefix_words(words) else {
        return Ok(None);
    };
    if !choice_object_shape_matches_words(&words[idx..], CREATURE_TYPE_PATTERN) {
        return Ok(None);
    }
    idx += 2;

    let mut excluded_subtypes = Vec::new();
    if choice_object_shape_matches_words(&words[idx..], OTHER_THAN_PATTERN) {
        let subtype_word = words.get(idx + 2).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing creature subtype exclusion in creature-type choice clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        let subtype = parse_subtype_flexible(subtype_word).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported creature subtype exclusion in creature-type choice clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        excluded_subtypes.push(subtype);
        idx += 3;
    }

    Ok(Some((idx, excluded_subtypes)))
}

pub(crate) fn parse_choose_phrase_prefix_words(words: &[&str]) -> Option<usize> {
    if words
        .first()
        .is_none_or(|word| !choice_word_is_any(word, CHOOSE_WORDS))
    {
        return None;
    }

    let mut idx = 1usize;
    if word_refs_at_is_article(words, idx) {
        idx += 1;
    }
    Some(idx)
}

pub(crate) fn parse_choose_color_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Option<ColorSet>)>, CardTextError> {
    let Some(mut idx) = parse_choose_phrase_prefix_words(words) else {
        return Ok(None);
    };
    if words
        .get(idx)
        .is_none_or(|word| !choice_word_is(word, COLOR_WORD))
    {
        return Ok(None);
    }
    idx += 1;

    let mut excluded = None;
    if choice_object_shape_matches_words(&words[idx..], OTHER_THAN_PATTERN) {
        let color_word = words.get(idx + 2).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing color exclusion in choose-color clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        excluded = Some(parse_color(color_word).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported color exclusion in choose-color clause (clause: '{}')",
                words.join(" ")
            ))
        })?);
        idx += 3;
    }

    Ok(Some((idx, excluded)))
}

pub(crate) fn parse_choose_card_type_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Vec<CardType>)>, CardTextError> {
    let Some(mut idx) = parse_choose_phrase_prefix_words(words) else {
        return Ok(None);
    };
    if choice_object_shape_matches_words(&words[idx..], CARD_TYPE_PATTERN) {
        return Ok(Some((idx + 2, Vec::new())));
    }
    if choice_object_shape_matches_words(&words[idx..], PERMANENT_TYPE_PATTERN) {
        return Ok(Some((
            idx + 2,
            vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ],
        )));
    }

    let mut options = Vec::new();
    let mut consumed_any = false;
    while let Some(word) = words.get(idx).copied() {
        if choice_word_is_any(word, CHOICE_CONNECTOR_WORDS) {
            idx += 1;
            continue;
        }
        let Some(card_type) = parse_card_type(word) else {
            break;
        };
        crate::slice_primitives::push_unique(&mut options, card_type);
        consumed_any = true;
        idx += 1;
    }

    if !consumed_any {
        return Ok(None);
    }

    Ok(Some((idx, options)))
}

pub(crate) fn parse_choose_player_phrase_words(words: &[&str]) -> Option<usize> {
    let mut idx = parse_choose_phrase_prefix_words(words)?;
    if words
        .get(idx)
        .is_none_or(|word| !choice_word_is(word, PLAYER_WORD))
    {
        return None;
    }
    idx += 1;
    Some(idx)
}

pub(crate) fn parse_choose_basic_land_type_phrase_words(words: &[&str]) -> Option<usize> {
    let mut idx = parse_choose_phrase_prefix_words(words)?;
    if !choice_object_shape_matches_words(&words[idx..], BASIC_LAND_TYPE_PATTERN) {
        return None;
    }
    idx += 3;
    Some(idx)
}

pub(crate) fn parse_choose_land_type_phrase_words(words: &[&str]) -> Option<usize> {
    let mut idx = parse_choose_phrase_prefix_words(words)?;
    if !choice_object_shape_matches_words(&words[idx..], LAND_TYPE_PATTERN) {
        return None;
    }
    idx += 2;
    Some(idx)
}

pub(crate) fn parse_choose_creature_type_then_become_type(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = crate::runtime_backend::token_word_refs(&first_tokens);
    enum ChoiceKind {
        CreatureType { excluded_subtypes: Vec<Subtype> },
        BasicLandType,
    }

    let choice_kind = if let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(&first_words)?
    {
        if consumed != first_words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported creature-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
        Some(ChoiceKind::CreatureType { excluded_subtypes })
    } else if let Some(consumed) = parse_choose_basic_land_type_phrase_words(&first_words) {
        if consumed != first_words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported basic-land-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
        Some(ChoiceKind::BasicLandType)
    } else {
        None
    };
    let Some(choice_kind) = choice_kind else {
        return Ok(None);
    };

    let second_words = crate::runtime_backend::token_word_refs(second);
    let Some(become_idx) = find_choice_token_word_any(second, BECOME_OR_BECOMES_WORDS) else {
        return Ok(None);
    };
    if become_idx == 0 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&second[..become_idx]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in creature-type become clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let become_tail_tokens = trim_commas(&second[become_idx + 1..]);
    let (duration, become_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(&become_tail_tokens)? {
            (duration, remainder)
        } else {
            (Until::Forever, become_tail_tokens.to_vec())
        };
    let become_words = crate::runtime_backend::token_word_refs(&become_tokens);
    if !choice_object_shape_matches_words(&become_words, THAT_TYPE_PATTERN) {
        return Ok(None);
    }

    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    let target = if choice_object_shape_matches_words(&subject_words, EACH_OR_ALL_PREFIX_PATTERN) {
        let filter_tokens = trim_commas(&subject_tokens[1..]);
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
        let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            ))
        })?;
        TargetAst::Object(filter, None, None)
    } else {
        parse_target_phrase(&subject_tokens)?
    };

    let effect = match choice_kind {
        ChoiceKind::CreatureType { excluded_subtypes } => {
            EffectAst::subject_verb_become_creature_type_choice(target, duration, excluded_subtypes)
        }
        ChoiceKind::BasicLandType => {
            EffectAst::subject_verb_become_basic_land_type_choice(target, duration)
        }
    };

    Ok(Some(vec![effect]))
}

pub(crate) fn parse_sentence_target_player_chooses_then_puts_on_top_of_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(and_idx) = find_choice_token_word(tokens, AND_WORD) else {
        return Ok(None);
    };
    let first_clause = trim_commas(&tokens[..and_idx]);
    let second_clause = trim_commas(&tokens[and_idx + 1..]);
    if second_clause.is_empty() {
        return Ok(None);
    }

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(&first_clause)?
    else {
        return Ok(None);
    };

    let second_words = crate::runtime_backend::token_word_refs(&second_clause);
    if !choice_word_at_is_any(&second_words, 0, PUT_OR_PUTS_WORDS) {
        return Ok(None);
    }
    let Some(on_idx) = find_choice_token_word(&second_clause, ON_WORD) else {
        return Ok(None);
    };
    let top_of_words = crate::runtime_backend::token_word_refs(
        second_clause
            .get(on_idx + 1..on_idx + 3)
            .unwrap_or_default(),
    );
    if !choice_object_shape_matches_words(&top_of_words, TOP_OF_PATTERN) {
        return Ok(None);
    }
    let destination_words = crate::runtime_backend::token_word_refs(&second_clause[on_idx + 3..]);
    if !choice_object_shape_matches_words(&destination_words, LIBRARY_WORD_PATTERN) {
        return Ok(None);
    }

    let moved_tokens = trim_commas(&second_clause[1..on_idx]);
    let moved_words = crate::runtime_backend::token_word_refs(&moved_tokens);
    let target = if moved_tokens.is_empty()
        || choice_object_shape_matches_words(&moved_words, TAGGED_CHOICE_LIBRARY_MOVED_PATTERN)
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&second_clause))
    } else {
        parse_target_phrase(&moved_tokens)?
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Library,
            true,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ]))
}

pub(crate) fn parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = find_window_by(tokens, 2, |window| {
        token_slice_first_kind(window, TokenKind::Comma) && token_slice_at_is(window, 1, "then")
    })
    .map(|idx| (idx, idx + 2))
    .or_else(|| {
        find_choice_token_word(tokens, THEN_WORD)
            .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
    });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let first_clause = trim_commas(&tokens[..head_end]);
    let second_clause = trim_commas(&tokens[tail_start..]);
    if second_clause.is_empty() {
        return Ok(None);
    }

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(&first_clause)?
    else {
        return Ok(None);
    };

    let second_words = crate::runtime_backend::token_word_refs(&second_clause);
    if second_words.len() < 4
        || !choice_object_shape_matches_words(&second_words, YOU_PUT_OR_PUTS_PATTERN)
    {
        return Ok(None);
    }

    let Some(onto_idx) = find_choice_token_word(&second_clause, ONTO_WORD) else {
        return Ok(None);
    };
    if onto_idx < 2 {
        return Ok(None);
    }

    let moved_words = crate::runtime_backend::token_word_refs(&second_clause[2..onto_idx]);
    if !choice_object_shape_matches_words(&moved_words, TAGGED_CHOICE_MOVED_PATTERN) {
        return Ok(None);
    }

    let destination_words =
        crate::runtime_backend::util::non_article_token_word_refs(&second_clause[onto_idx + 1..]);
    if destination_words
        .first()
        .is_none_or(|word| !choice_word_is(word, BATTLEFIELD_WORD))
    {
        return Ok(None);
    }
    let mut destination_tail: Vec<&str> = destination_words[1..].to_vec();
    let battlefield_tapped = destination_tail
        .iter()
        .any(|word| choice_word_is(word, TAPPED_WORD));
    destination_tail.retain(|word| !choice_word_is(word, TAPPED_WORD));
    let battlefield_controller = if choice_object_shape_matches_words(
        &destination_tail,
        BATTLEFIELD_UNDER_YOUR_CONTROL_PATTERN,
    ) {
        ReturnControllerAst::You
    } else if destination_tail.is_empty() {
        ReturnControllerAst::Preserve
    } else if choice_object_shape_matches_words(
        &destination_tail,
        BATTLEFIELD_UNDER_OWNER_CONTROL_PATTERN,
    ) {
        ReturnControllerAst::Owner
    } else {
        return Ok(None);
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&second_clause)),
            Zone::Battlefield,
            false,
            battlefield_controller,
            battlefield_tapped,
            None,
        ),
    ]))
}
