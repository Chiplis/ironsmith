use super::super::super::lexer::{
    parser_token_word_refs, word_slice_contains_any_word, word_slice_contains_phrase,
};
use super::*;

const TARGET_OR_TARGETS_WORDS: &[&str] = &["target", "targets"];
const THAT_WORD: &str = "that";
const ONLY_WORD: &str = "only";
const SINGLE_WORD: &str = "single";
const YOU_TARGET_PREFIX: &[&str] = &["you"];
const OPPONENT_TARGET_PREFIXES: &[&[&str]] = &[&["opponent"], &["opponents"]];
const PLAYER_TARGET_PREFIXES: &[&[&str]] = &[&["player"], &["players"]];
const OR_WORD: &str = "or";
const UNTIL_WORD: &str = "until";
const OTHER_OR_ANOTHER_WORDS: &[&str] = &["other", "another"];
const OTHER_THAN_PREFIX: &[&str] = &["other", "than"];
const SELF_REFERENCE_WORDS: &[&str] = &["this", "it", "them"];
const OBJECT_REFERENCE_NOUN_WORDS: &[&str] = &[
    "artifact",
    "artifacts",
    "battle",
    "battles",
    "card",
    "cards",
    "creature",
    "creatures",
    "enchantment",
    "enchantments",
    "land",
    "lands",
    "permanent",
    "permanents",
    "planeswalker",
    "planeswalkers",
    "spell",
    "spells",
    "token",
    "tokens",
];
const EXCLUSION_RELATION_IGNORED_PREFIXES: &[&[&str]] =
    &[&["enchanted"], &["equipped"], &["basic", "land"]];
const REST_REVEALED_OBJECT_PHRASES: &[&[&str]] = &[
    &["rest"],
    &["rest", "of", "revealed", "cards"],
    &["remaining", "revealed", "cards"],
];
const TAGGED_COUNTER_STATE_DISJUNCTION_PHRASES: &[&[&str]] = &[
    &["counter", "on", "it", "or"],
    &["counter", "on", "them", "or"],
];
const SUSPENDED_CARD_DISJUNCTION_PHRASES: &[&[&str]] =
    &[&["or", "suspended", "card"], &["or", "suspended", "cards"]];
const ENTERED_THIS_TURN_UNSUPPORTED_PHRASE: &[&str] = &["entered", "this", "turn"];
const BLOCKED_BY_TAGGED_OBJECT_PHRASES: &[&[&str]] = &[
    &["blocked", "by", "one", "of", "those"],
    &["blocked", "by", "those"],
    &["blocked", "by", "that"],
];
const POWER_OR_TOUGHNESS_PHRASES: &[&[&str]] =
    &[&["power", "or", "toughness"], &["toughness", "or", "power"]];
const TARGET_PLAYER_REFERENCE_PHRASES: &[&[&str]] =
    &[&["target", "player"], &["target", "players"]];
const TARGET_OPPONENT_REFERENCE_PHRASES: &[&[&str]] =
    &[&["target", "opponent"], &["target", "opponents"]];
const WITH_WORD: &str = "with";
const WITHOUT_WORD: &str = "without";
const BASE_POWER_TOUGHNESS_PREFIX: &[&str] = &["base", "power", "and", "toughness"];
const POWER_TOUGHNESS_PREFIX: &[&str] = &["power", "and", "toughness"];
const BASE_WORD: &str = "base";
const POWER_WORD: &str = "power";
const TOUGHNESS_WORD: &str = "toughness";
const AND_WORD: &str = "and";
const AND_OR_WORDS: &[&str] = &["and", "or"];
const BE_VERB_WORDS: &[&str] = &["are", "is", "was", "were"];
const HAS_HAVE_WORDS: &[&str] = &["has", "have"];
const TAGGED_SPELL_REFERENCE_WORDS: &[&str] = &["that", "this", "its", "their"];
const ABILITY_OR_ABILITIES_WORDS: &[&str] = &["ability", "abilities"];
const ACTIVATED_ABILITY_WORDS: &[&str] = &["activated", "ability"];
const TRIGGERED_ABILITY_WORDS: &[&str] = &["triggered", "ability"];
const ACTIVATED_OR_TRIGGERED_ABILITY_PHRASES: &[&[&str]] = &[
    &["activated", "or", "triggered", "ability"],
    &["triggered", "or", "activated", "ability"],
];
const TEXT_NEGATION_WORDS: &[&str] = &["not", "isnt", "isn't", "arent", "aren't"];
const LEGENDARY_OR_PREFIX: &[&str] = &["legendary", "or"];
const PUT_ON_PREFIX: &[&str] = &["put", "on"];
const PUT_ON_REFERENCE_WORDS: &[&str] = &["it", "them"];
const MANA_VALUE_PREFIX: &[&str] = &["mana", "value"];
const NOT_HISTORIC_PHRASE: &[&str] = &["not", "historic"];
const ATTACKING_WORD: &str = "attacking";
const BLOCKING_WORD: &str = "blocking";
const BLOCKED_WORD: &str = "blocked";
const HISTORIC_WORD: &str = "historic";
const COMMANDER_OR_COMMANDERS_WORDS: &[&str] = &["commander", "commanders"];
const CHOSEN_WORD: &str = "chosen";
const NONCHOSEN_WORD: &str = "nonchosen";
const COLOR_WORD: &str = "color";
const TYPE_WORD: &str = "type";
const PERMANENT_OR_PERMANENTS_WORDS: &[&str] = &["permanent", "permanents"];
const SPELL_OR_SPELLS_WORDS: &[&str] = &["spell", "spells"];
const POWER_GREATER_THAN_BASE_POWER_PHRASE: &[&str] =
    &["power", "greater", "than", "its", "base", "power"];
const NON_WORD: &str = "non";
const ATTACKED_THIS_TURN_PHRASE: &[&str] = &["attacked", "this", "turn"];
const TYPE_LIST_CONJUNCTION_WORDS: &[&str] = &["and", "or", "and/or"];
const STRICT_COMPOUND_COUNT_PREFIXES: &[&[&str]] = &[&["and", "each"], &["and", "every"]];
const STRICT_FOR_EACH_TAIL_PREFIX: &[&str] = &["for", "each"];
const OTHER_THAN_BASIC_LAND_PREFIX: &[&str] = &["other", "than", "basic", "land"];
const CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const AGGREGATE_SCOPE_WORDS: &[&str] = &["greatest", "least", "total"];
const AGGREGATE_SCOPE_MARKER_WORDS: &[&str] = &["among", "of"];

fn find_phrase_start(words: &[&str], phrase: &[&str]) -> Option<usize> {
    words
        .windows(phrase.len())
        .position(|window| window == phrase)
}

fn word_is_any(word: &str, expected: &[&str]) -> bool {
    expected.contains(&word)
}

fn non_article_parser_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str> {
    parser_token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect()
}

fn non_article_token_words_eq(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    word_slice_eq(&non_article_parser_word_refs(tokens), expected)
}

fn non_article_token_words_eq_any(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    word_slice_eq_any(&non_article_parser_word_refs(tokens), expected)
}

fn non_article_token_words_starts_with(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    word_slice_starts_with(&non_article_parser_word_refs(tokens), expected)
}

fn non_article_token_words_starts_with_any(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    word_slice_starts_with_any(&non_article_parser_word_refs(tokens), expected)
}

fn non_article_token_words_contains_phrase(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    word_slice_contains_phrase(&non_article_parser_word_refs(tokens), phrase)
}

fn non_article_token_words_contains_any_phrase(
    tokens: &[OwnedLexToken],
    phrases: &[&[&str]],
) -> bool {
    word_slice_contains_any_phrase(&non_article_parser_word_refs(tokens), phrases)
}

fn non_article_token_words_contains_any_word(tokens: &[OwnedLexToken], words: &[&str]) -> bool {
    let token_words = non_article_parser_word_refs(tokens);
    words
        .iter()
        .any(|word| word_slice_contains_word(&token_words, word))
}

fn has_tap_activated_ability_phrase(words: &[&str]) -> bool {
    const TAP_ACTIVATED_ABILITY_PHRASES: &[&[&str]] = &[
        &[
            "has",
            "activated",
            "ability",
            "with",
            "t",
            "in",
            "its",
            "cost",
        ],
        &[
            "has",
            "activated",
            "ability",
            "with",
            "tap",
            "in",
            "its",
            "cost",
        ],
        &[
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
        ],
        &[
            "activated",
            "abilities",
            "with",
            "tap",
            "in",
            "their",
            "costs",
        ],
    ];
    TAP_ACTIVATED_ABILITY_PHRASES
        .iter()
        .any(|phrase| find_phrase_start(words, phrase).is_some())
}

fn strip_be_put_on_reference_prefix(all_words: &mut Vec<&str>, segment_tokens: &[OwnedLexToken]) {
    if all_words.len() < 4 || segment_tokens.len() < 4 {
        return;
    }

    let be_words = non_article_parser_word_refs(&segment_tokens[..1]);
    let put_on_words = non_article_parser_word_refs(&segment_tokens[1..4]);
    if !be_words
        .first()
        .is_some_and(|word| word_is_any(word, BE_VERB_WORDS))
        || !word_slice_starts_with(&put_on_words, PUT_ON_PREFIX)
        || !word_slice_contains_any_word(&put_on_words, PUT_ON_REFERENCE_WORDS)
    {
        return;
    }

    all_words.drain(0..3);
}

pub(crate) fn parse_object_filter_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_lexed(tokens, other)
}

pub(super) fn parse_object_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_inner(tokens, other, true)
}

pub(super) fn parse_object_filter_permissive(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_inner(tokens, other, false)
}

pub(super) fn parse_object_filter_inner(
    tokens: &[OwnedLexToken],
    other: bool,
    strict: bool,
) -> Result<ObjectFilter, CardTextError> {
    let (tokens, vote_winners_only) = trim_vote_winner_suffix(tokens);
    let mut filter = ObjectFilter::default();
    if other {
        filter.other = true;
    }

    let mut target_player: Option<PlayerFilter> = None;
    let mut target_object: Option<ObjectFilter> = None;
    let mut targets_only = false;
    let mut target_count: Option<crate::effect::ChoiceCount> = None;
    let mut base_tokens: Vec<OwnedLexToken> = tokens.to_vec();
    let mut targets_idx: Option<usize> = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token
            .as_word()
            .is_some_and(|word| TARGET_OR_TARGETS_WORDS.contains(&word))
        {
            if idx > 0
                && tokens[idx - 1]
                    .as_word()
                    .is_some_and(|word| word == THAT_WORD)
            {
                targets_idx = Some(idx);
                break;
            }
        }
    }
    if let Some(targets_idx) = targets_idx {
        let that_idx = targets_idx - 1;
        base_tokens = tokens[..that_idx].to_vec();
        let target_tokens = &tokens[targets_idx + 1..];
        let parse_target_fragment = |fragment_tokens: &[OwnedLexToken]| -> Result<
            (
                Option<PlayerFilter>,
                Option<ObjectFilter>,
                bool,
                Option<crate::effect::ChoiceCount>,
            ),
            CardTextError,
        > {
            let mut fragment_tokens = trim_commas(fragment_tokens);
            let mut only = false;
            let mut count = None;
            // The outer scan splits target fragments after the demonstrative
            // "that target(s)" marker, so a fragment never re-introduces a
            // leading "that"; strip one defensively to keep the fragment shape
            // stable if upstream splitting changes.
            if fragment_tokens
                .first()
                .is_some_and(|token| token.as_word().is_some_and(|word| word == THAT_WORD))
            {
                fragment_tokens.drain(..1);
            }
            if fragment_tokens
                .first()
                .is_some_and(|token| token.as_word().is_some_and(|word| word == ONLY_WORD))
            {
                only = true;
                fragment_tokens.drain(..1);
            }
            if fragment_tokens.len() >= 2
                && fragment_tokens[0].is_word("a")
                && fragment_tokens[1]
                    .as_word()
                    .is_some_and(|word| word == SINGLE_WORD)
            {
                count = Some(crate::effect::ChoiceCount::exactly(1));
                fragment_tokens.drain(..2);
            } else if fragment_tokens
                .first()
                .is_some_and(|token| token.as_word().is_some_and(|word| word == SINGLE_WORD))
            {
                count = Some(crate::effect::ChoiceCount::exactly(1));
                fragment_tokens.drain(..1);
            }

            if non_article_token_words_starts_with(&fragment_tokens, YOU_TARGET_PREFIX) {
                return Ok((Some(PlayerFilter::You), None, only, count));
            }
            if non_article_token_words_starts_with_any(&fragment_tokens, OPPONENT_TARGET_PREFIXES) {
                return Ok((Some(PlayerFilter::Opponent), None, only, count));
            }
            if non_article_token_words_starts_with_any(&fragment_tokens, PLAYER_TARGET_PREFIXES) {
                return Ok((Some(PlayerFilter::Any), None, only, count));
            }

            let mut target_filter_tokens = fragment_tokens.as_slice();
            if target_filter_tokens.first().is_some_and(|token| {
                token
                    .as_word()
                    .is_some_and(|word| TARGET_OR_TARGETS_WORDS.contains(&word))
            }) {
                target_filter_tokens = &target_filter_tokens[1..];
            }
            if target_filter_tokens.is_empty() {
                return Ok((None, None, only, count));
            }
            Ok((
                None,
                Some(parse_object_filter_permissive(target_filter_tokens, false)?),
                only,
                count,
            ))
        };

        if let Some(or_token_idx) = target_tokens
            .iter()
            .position(|token| token.as_word().is_some_and(|word| word == OR_WORD))
        {
            let left_tokens = trim_commas(&target_tokens[..or_token_idx]);
            let right_tokens = trim_commas(&target_tokens[or_token_idx + 1..]);
            let (left_player, left_object, left_only, left_count) =
                parse_target_fragment(&left_tokens)?;
            let (right_player, right_object, right_only, right_count) =
                parse_target_fragment(&right_tokens)?;
            target_player = left_player.or(right_player);
            target_object = left_object.or(right_object);
            targets_only = left_only || right_only;
            target_count = left_count.or(right_count);
            if target_player.is_some() && target_object.is_some() {
                filter.targets_any_of = true;
            }
        } else {
            let (parsed_player, parsed_object, parsed_only, parsed_count) =
                parse_target_fragment(target_tokens)?;
            target_player = parsed_player;
            target_object = parsed_object;
            targets_only = parsed_only;
            target_count = parsed_count;
        }
    }

    // Object filters should not absorb trailing duration clauses such as
    // "... until this enchantment leaves the battlefield".
    if let Some(until_token_idx) = token_find_index(&base_tokens, |token| {
        token.as_word().is_some_and(|word| word == UNTIL_WORD)
    }) && until_token_idx > 0
    {
        base_tokens.truncate(until_token_idx);
    }

    let not_on_battlefield = strip_not_on_battlefield_phrase(&mut base_tokens);

    // "other than this/it/them ..." marks an exclusion, not an additional
    // type selector. Keep "other" but drop the self-reference tail.
    let mut idx = 0usize;
    while idx + 2 < base_tokens.len() {
        if !non_article_token_words_eq(&base_tokens[idx..idx + 2], OTHER_THAN_PREFIX) {
            idx += 1;
            continue;
        }

        let mut end = idx + 2;
        let starts_with_self_reference = base_tokens[end]
            .as_word()
            .is_some_and(|word| SELF_REFERENCE_WORDS.contains(&word));
        if !starts_with_self_reference {
            idx += 1;
            continue;
        }
        end += 1;

        if end < base_tokens.len()
            && base_tokens[end]
                .as_word()
                .is_some_and(|word| OBJECT_REFERENCE_NOUN_WORDS.contains(&word))
        {
            end += 1;
        }

        base_tokens.drain(idx + 1..end);
    }

    // "other than Werewolves and Wolves" is an exclusion on the described
    // object class, not the source-relative "other" predicate.
    let mut idx = 0usize;
    while idx + 2 < base_tokens.len() {
        if !non_article_token_words_eq(&base_tokens[idx..idx + 2], OTHER_THAN_PREFIX) {
            idx += 1;
            continue;
        }

        let mut base_card_types = Vec::new();
        for token in &base_tokens[..idx] {
            for piece in token.parser_word_pieces() {
                if let Some(card_type) = parse_card_type(piece.text.as_str()) {
                    push_unique(&mut base_card_types, card_type);
                }
            }
        }

        let tail_tokens = &base_tokens[idx + 2..];
        if non_article_token_words_starts_with_any(tail_tokens, EXCLUSION_RELATION_IGNORED_PREFIXES)
        {
            idx += 1;
            continue;
        }
        let mut excluded_card_types = Vec::new();
        let mut excluded_subtypes = Vec::new();
        let mut excluded_supertypes = Vec::new();
        let mut excluded_colors = ColorSet::new();
        for token in tail_tokens {
            for piece in token.parser_word_pieces() {
                let word = piece.text.as_str();
                if is_article(word) || AND_OR_WORDS.contains(&word) {
                    continue;
                }
                if let Some(card_type) = parse_card_type(word) {
                    push_unique(&mut excluded_card_types, card_type);
                }
                if let Some(subtype) = parse_subtype_flexible(word) {
                    push_unique(&mut excluded_subtypes, subtype);
                }
                if let Some(supertype) = parse_supertype_word(word) {
                    push_unique(&mut excluded_supertypes, supertype);
                }
                if let Some(color) = parse_color(word) {
                    excluded_colors = excluded_colors.union(color);
                }
            }
        }

        let has_specific_exclusion = !excluded_subtypes.is_empty()
            || !excluded_supertypes.is_empty()
            || !excluded_colors.is_empty();
        let saw_exclusion = !excluded_card_types.is_empty() || has_specific_exclusion;
        if !saw_exclusion {
            idx += 1;
            continue;
        }

        for card_type in excluded_card_types {
            if has_specific_exclusion && base_card_types.contains(&card_type) {
                continue;
            }
            push_unique(&mut filter.excluded_card_types, card_type);
        }
        for subtype in excluded_subtypes {
            push_unique(&mut filter.excluded_subtypes, subtype);
        }
        for supertype in excluded_supertypes {
            push_unique(&mut filter.excluded_supertypes, supertype);
        }
        filter.excluded_colors = filter.excluded_colors.union(excluded_colors);
        base_tokens.truncate(idx);
        break;
    }

    if let Some(mut disjunction) = parse_attached_reference_or_another_disjunction(&base_tokens)? {
        if target_player.is_some() || target_object.is_some() {
            disjunction = if targets_only {
                disjunction.targeting_only(target_player.take(), target_object.take())
            } else {
                disjunction.targeting(target_player.take(), target_object.take())
            };
            if let Some(count) = target_count {
                disjunction = disjunction.with_target_count(count);
            } else if targets_only {
                disjunction = disjunction.target_count_exact(1);
            }
        }
        return Ok(disjunction);
    }
    let mut segment_tokens = base_tokens.clone();

    let raw_words_with_articles = parser_token_word_refs(&base_tokens);
    let all_words_with_articles = word_refs_except(&raw_words_with_articles, &["instead"]);

    let map_non_article_index = |non_article_idx: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_idx {
                return Some(idx);
            }
            seen += 1;
        }
        None
    };

    let map_non_article_end = |non_article_end: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_end {
                return Some(idx);
            }
            seen += 1;
        }
        if seen == non_article_end {
            return Some(all_words_with_articles.len());
        }
        None
    };

    let mut all_words = non_article_word_refs(&all_words_with_articles);
    let has_tap_activated_ability = has_tap_activated_ability_phrase(&all_words);
    if non_article_token_words_eq(&base_tokens, ACTIVATED_ABILITY_WORDS) {
        return Ok(ObjectFilter::activated_ability());
    }
    if non_article_token_words_eq(&base_tokens, TRIGGERED_ABILITY_WORDS) {
        let mut filter = ObjectFilter::ability();
        filter.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
        return Ok(filter);
    }
    if non_article_token_words_eq_any(&base_tokens, ACTIVATED_OR_TRIGGERED_ABILITY_PHRASES) {
        let mut triggered = ObjectFilter::ability();
        triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
        let mut filter = ObjectFilter::default();
        filter.any_of = vec![ObjectFilter::activated_ability(), triggered];
        return Ok(filter);
    }
    if non_article_token_words_eq_any(&base_tokens, REST_REVEALED_OBJECT_PHRASES) {
        return Ok(ObjectFilter::tagged("rest"));
    }

    try_apply_distinct_powers_clause(&mut filter, &mut all_words);
    try_apply_distinct_creature_types_clause(&mut filter, &mut all_words);

    try_apply_could_be_targeted_by_that_spell_clause(&mut filter, &mut all_words);

    // "that were put there from the battlefield this turn" means the card entered
    // a graveyard from the battlefield this turn.
    try_apply_put_there_from_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "legendary or Rat card" (Nashi, Moon's Legacy) is a supertype/subtype disjunction.
    // We parse it by collecting both selectors and then expanding into an `any_of` filter
    // after the normal pass so other shared qualifiers (zone/owner/etc.) are preserved.
    let legendary_or_subtype = find_phrase_start(&all_words, LEGENDARY_OR_PREFIX)
        .and_then(|idx| all_words.get(idx + 2).copied())
        .and_then(parse_subtype_word);

    // "in a graveyard that was put there from anywhere this turn" (Reenact the Crime)
    // means the card entered a graveyard this turn.
    try_apply_put_there_from_anywhere_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "... graveyard from the battlefield this turn" means the card entered a graveyard
    // from the battlefield this turn.
    try_apply_graveyard_from_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "... entered the battlefield ... this turn" marks a battlefield entry this turn.
    try_apply_entered_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    try_apply_drawn_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);

    if non_article_token_words_contains_any_phrase(
        &segment_tokens,
        BLOCKED_BY_TAGGED_OBJECT_PHRASES,
    ) {
        filter.blocked = true;
        filter.blocked_by = Some(crate::filter::ObjectRef::Tagged(TagKey::from(IT_TAG)));
    }

    // Avoid treating reference phrases like "... with mana value equal to the number of charge
    // counters on this artifact" as additional type selectors on the filtered object.
    // (Aether Vial: "put a creature card with mana value equal to the number of charge counters
    // on this artifact from your hand onto the battlefield.")
    let _ = try_apply_mana_value_eq_counters_on_source_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    try_apply_attached_exclusion_phrases(&mut filter, &mut all_words);
    let exclude_basic_land_cards =
        strip_other_than_basic_land_cards_clause(&mut all_words, &mut segment_tokens);

    let _ = try_apply_pt_literal_prefix(&mut filter, &mut all_words);

    strip_object_filter_leading_prefixes(&mut all_words);

    let _ = try_apply_not_all_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_not_exactly_two_colors_clause(&mut filter, &mut all_words);

    strip_be_put_on_reference_prefix(&mut all_words, &segment_tokens);

    let _ = try_apply_leading_tagged_reference_prefix(&mut filter, &mut all_words);

    let _ = try_apply_entered_since_your_last_turn_ended_clause(&mut filter, &mut all_words);

    strip_object_filter_face_state_words(&mut filter, &mut all_words);

    if non_article_token_words_contains_phrase(
        &segment_tokens,
        ENTERED_THIS_TURN_UNSUPPORTED_PHRASE,
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported entered-this-turn object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    let has_counter_state_or_clause = non_article_token_words_contains_any_phrase(
        &segment_tokens,
        TAGGED_COUNTER_STATE_DISJUNCTION_PHRASES,
    );
    let has_supported_suspended_disjunction = non_article_token_words_contains_any_phrase(
        &segment_tokens,
        SUSPENDED_CARD_DISJUNCTION_PHRASES,
    );
    if has_counter_state_or_clause && !has_supported_suspended_disjunction {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-state object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    strip_single_graveyard_phrase(&mut filter, &mut all_words);

    let _ = try_apply_not_named_clause(
        &mut filter,
        &mut all_words,
        &all_words_with_articles,
        &map_non_article_index,
        &map_non_article_end,
    )?;

    let _ = try_apply_named_clause(
        &mut filter,
        &mut all_words,
        &all_words_with_articles,
        &map_non_article_index,
        &map_non_article_end,
    )?;

    let _ = try_apply_color_count_phrase(&mut filter, &mut all_words)?;
    let has_power_or_toughness_clause =
        non_article_token_words_contains_any_phrase(&segment_tokens, POWER_OR_TOUGHNESS_PHRASES);
    if has_power_or_toughness_clause
        && !all_words
            .iter()
            .any(|word| word_is_any(word, SPELL_OR_SPELLS_WORDS))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    let reference_stage =
        apply_reference_and_tag_stage(&mut filter, &mut all_words, &mut segment_tokens);
    if reference_stage.early_return {
        return Ok(filter);
    }
    let source_linked_exile_reference = reference_stage.source_linked_exile_reference;

    let references_target_player = non_article_token_words_contains_any_phrase(
        &segment_tokens,
        TARGET_PLAYER_REFERENCE_PHRASES,
    );
    let references_target_opponent = non_article_token_words_contains_any_phrase(
        &segment_tokens,
        TARGET_OPPONENT_REFERENCE_PHRASES,
    );
    let pronoun_player_filter = if references_target_opponent {
        PlayerFilter::target_opponent()
    } else if references_target_player {
        PlayerFilter::target_player()
    } else {
        PlayerFilter::IteratedPlayer
    };

    if let Some(attacking_filter) =
        attacking_player_filter_from_words(&all_words, &pronoun_player_filter)
    {
        filter.attacking_player_or_planeswalker_controlled_by = Some(attacking_filter);
    }

    let is_tagged_spell_reference_at = |idx: usize| {
        all_words
            .get(idx.wrapping_sub(1))
            .is_some_and(|prev| word_is_any(prev, TAGGED_SPELL_REFERENCE_WORDS))
    };
    let contains_unqualified_spell_word = all_words.iter().enumerate().any(|(idx, word)| {
        word_is_any(word, SPELL_OR_SPELLS_WORDS) && !is_tagged_spell_reference_at(idx)
    });
    let mentions_ability_word = all_words
        .iter()
        .any(|word| word_is_any(word, ABILITY_OR_ABILITIES_WORDS));
    if contains_unqualified_spell_word && !mentions_ability_word {
        filter.has_mana_cost = true;
    }

    if !all_words.is_empty() {
        let mut idx = 0usize;
        while idx < all_words.len() {
            let slice = &all_words[idx..];
            if relation_clause_is_inside_aggregate_scope(&all_words, idx) {
                idx += 1;
                continue;
            }
            if let Some(consumed) =
                try_apply_joint_owner_controller_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) = try_apply_chosen_player_graveyard_clause(&mut filter, slice) {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) = try_apply_negated_you_relation_clause(&mut filter, slice) {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) =
                try_apply_player_relation_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            idx += 1;
        }
    }

    let mut with_idx = 0usize;
    while with_idx + 1 < all_words.len() {
        if all_words[with_idx] != WITH_WORD {
            with_idx += 1;
            continue;
        }

        if let Some(consumed) = try_apply_with_clause_tail(&mut filter, &all_words[with_idx + 1..])
        {
            with_idx += 1 + consumed;
            continue;
        }

        with_idx += 1;
    }

    let mut has_idx = 0usize;
    while has_idx + 1 < all_words.len() {
        if !word_is_any(all_words[has_idx], HAS_HAVE_WORDS) {
            has_idx += 1;
            continue;
        }
        if filter.with_counter.is_none()
            && let Some((counter_constraint, consumed)) =
                parse_filter_counter_constraint_words(&all_words[has_idx + 1..])
        {
            filter.with_counter = Some(counter_constraint);
            has_idx += 1 + consumed;
            continue;
        }
        has_idx += 1;
    }

    let mut without_idx = 0usize;
    while without_idx + 1 < all_words.len() {
        if all_words[without_idx] != WITHOUT_WORD {
            without_idx += 1;
            continue;
        }

        if let Some(consumed) =
            try_apply_without_clause_tail(&mut filter, &all_words[without_idx + 1..])
        {
            without_idx += 1 + consumed;
            continue;
        }

        without_idx += 1;
    }

    if has_tap_activated_ability {
        filter.has_tap_activated_ability = true;
    }

    let mut referenced_zones = Vec::new();
    for idx in 0..all_words.len() {
        if let Some(zone) = parse_zone_word(all_words[idx]) {
            if !slice_contains(&referenced_zones, &zone) {
                referenced_zones.push(zone);
            }
            let is_reference_zone_for_spell = if contains_unqualified_spell_word {
                idx > 0
                    && matches!(
                        all_words[idx - 1],
                        "controller"
                            | "controllers"
                            | "owner"
                            | "owners"
                            | "its"
                            | "their"
                            | "that"
                            | "this"
                    )
            } else {
                false
            };
            if is_reference_zone_for_spell {
                continue;
            }
            if filter.zone.is_none() {
                filter.zone = Some(zone);
            }
            if idx > 0 {
                match all_words[idx - 1] {
                    "your" => {
                        filter.owner = Some(PlayerFilter::You);
                    }
                    "opponent" | "opponents" => {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    "their" => {
                        filter.owner = Some(pronoun_player_filter.clone());
                    }
                    _ => {}
                }
            }
            if idx > 1 {
                let owner_pair = (all_words[idx - 2], all_words[idx - 1]);
                match owner_pair {
                    ("target", "player") | ("target", "players") => {
                        filter.owner = Some(PlayerFilter::target_player());
                    }
                    ("target", "opponent") | ("target", "opponents") => {
                        filter.owner = Some(PlayerFilter::target_opponent());
                    }
                    ("that", "player") | ("that", "players") => {
                        filter.owner = Some(PlayerFilter::IteratedPlayer);
                    }
                    _ => {}
                }
            }
        }
    }
    if referenced_zones.len() > 1 && filter.any_of.is_empty() {
        filter.zone = None;
        filter.any_of = referenced_zones
            .into_iter()
            .map(|zone| ObjectFilter::default().in_zone(zone))
            .collect();
    }

    let clause_words = all_words.clone();
    for idx in 0..all_words.len() {
        let value_tokens = match all_words.get(idx..) {
            Some(["total", "power", "and", "toughness", rest @ ..])
            | Some(["power", "and", "toughness", "totaling", rest @ ..]) => rest,
            _ => continue,
        };
        let Some((cmp, _consumed)) =
            parse_filter_comparison_tokens("power", value_tokens, &clause_words)?
        else {
            continue;
        };
        filter.total_power_toughness = Some(cmp);
        break;
    }

    for idx in 0..all_words.len() {
        let (is_base_reference, pt_word_idx) = if idx + 4 < all_words.len()
            && word_slice_starts_with(&all_words[idx..], BASE_POWER_TOUGHNESS_PREFIX)
        {
            (true, idx + 4)
        } else if idx + 3 < all_words.len()
            && word_slice_starts_with(&all_words[idx..], POWER_TOUGHNESS_PREFIX)
            && (idx == 0 || all_words[idx - 1] != BASE_WORD)
        {
            (false, idx + 3)
        } else {
            continue;
        };

        if let Ok((power, toughness)) = parse_pt_modifier(all_words[pt_word_idx]) {
            filter.power = Some(crate::filter::Comparison::Equal(power));
            filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
            filter.power_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
            filter.toughness_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
        }
    }

    let mut idx = 0usize;
    while idx < all_words.len() {
        let axis = if all_words[idx] == POWER_WORD {
            Some("power")
        } else if all_words[idx] == TOUGHNESS_WORD {
            Some("toughness")
        } else if idx + 1 < all_words.len()
            && word_slice_starts_with(&all_words[idx..], MANA_VALUE_PREFIX)
        {
            Some("mana value")
        } else {
            None
        };
        let Some(axis) = axis else {
            idx += 1;
            continue;
        };
        let is_base_reference = idx > 0 && all_words[idx - 1] == BASE_WORD;

        let axis_word_count =
            usize::from(word_slice_starts_with(&all_words[idx..], MANA_VALUE_PREFIX)) + 1;
        let value_tokens = if idx + axis_word_count < all_words.len() {
            &all_words[idx + axis_word_count..]
        } else {
            &[]
        };
        if axis == POWER_WORD && value_tokens.first().is_some_and(|word| *word == AND_WORD) {
            idx += 1;
            continue;
        }
        if axis == TOUGHNESS_WORD
            && idx >= 3
            && matches!(
                &all_words[idx - 3..idx],
                ["total", "power", "and"] | ["base", "power", "and"] | ["power", "and", "base"]
            )
        {
            idx += 1;
            continue;
        }
        let Some((cmp, consumed)) =
            parse_filter_comparison_tokens(axis, value_tokens, &clause_words)?
        else {
            idx += 1;
            continue;
        };

        match axis {
            "power" => {
                filter.power = Some(cmp);
                filter.power_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "toughness" => {
                filter.toughness = Some(cmp);
                filter.toughness_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "mana value" => filter.mana_value = Some(cmp),
            _ => {}
        }
        idx += axis_word_count + consumed;
    }

    apply_parity_filter_phrases(&clause_words, &mut filter);

    if word_slice_contains_phrase(&clause_words, POWER_GREATER_THAN_BASE_POWER_PHRASE) {
        filter.power_greater_than_base_power = true;
    }

    let mut saw_permanent = false;
    let mut saw_spell = false;
    let mut saw_permanent_type = false;

    let mut saw_subtype = false;
    let mut negated_word_indices = std::collections::HashSet::new();
    let mut negated_historic_indices = std::collections::HashSet::new();
    let is_text_negation_word = |word: &str| word_is_any(word, TEXT_NEGATION_WORDS);
    for idx in 0..all_words.len().saturating_sub(1) {
        if all_words[idx] != NON_WORD {
            continue;
        }
        let next = all_words[idx + 1];
        if is_outlaw_word(next) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(card_type) = parse_card_type(next)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(idx + 1);
        }
        if next == ATTACKING_WORD {
            filter.nonattacking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == BLOCKING_WORD {
            filter.nonblocking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == BLOCKED_WORD {
            filter.unblocked = true;
            negated_word_indices.insert(idx + 1);
        }
        if word_is_any(next, COMMANDER_OR_COMMANDERS_WORDS) {
            filter.noncommander = true;
            negated_word_indices.insert(idx + 1);
        }
        if let Some(color) = parse_color(next) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(subtype) = parse_subtype_flexible(next)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(idx + 1);
        }
    }
    for idx in 0..all_words.len() {
        if !is_text_negation_word(all_words[idx]) {
            continue;
        }
        let mut target_idx = idx + 1;
        if target_idx >= all_words.len() {
            continue;
        }
        if is_article(all_words[target_idx]) {
            target_idx += 1;
            if target_idx >= all_words.len() {
                continue;
            }
        }

        let negated_word = all_words[target_idx];
        if negated_word == ATTACKING_WORD {
            filter.nonattacking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == BLOCKING_WORD {
            filter.nonblocking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == BLOCKED_WORD {
            filter.unblocked = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == HISTORIC_WORD {
            filter.nonhistoric = true;
            negated_historic_indices.insert(target_idx);
        }
        if word_is_any(negated_word, COMMANDER_OR_COMMANDERS_WORDS) {
            filter.noncommander = true;
            negated_word_indices.insert(target_idx);
        }
        if let Some(card_type) = parse_card_type(negated_word)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(target_idx);
        }
        if let Some(supertype) = parse_supertype_word(negated_word)
            && !slice_has(&filter.excluded_supertypes, &supertype)
        {
            filter.excluded_supertypes.push(supertype);
            negated_word_indices.insert(target_idx);
        }
        if let Some(color) = parse_color(negated_word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(target_idx);
        }
        if let Some(subtype) = parse_subtype_flexible(negated_word)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(target_idx);
        }
    }
    for idx in 0..all_words.len().saturating_sub(1) {
        if word_slice_eq(&all_words[idx..idx + 2], NOT_HISTORIC_PHRASE) {
            filter.nonhistoric = true;
            negated_historic_indices.insert(idx + 1);
        }
    }

    if non_article_token_words_contains_phrase(&segment_tokens, ATTACKED_THIS_TURN_PHRASE) {
        filter.attacked_this_turn = true;
    }

    for (idx, word) in all_words.iter().enumerate() {
        let idx: usize = idx;
        let is_negated_word = set_has(&negated_word_indices, &idx);
        match *word {
            "permanent" | "permanents" => saw_permanent = true,
            "spell" | "spells" => {
                if !is_tagged_spell_reference_at(idx) {
                    saw_spell = true;
                }
            }
            word if word == CHOSEN_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == COLOR_WORD) =>
            {
                filter.chosen_color = true;
            }
            word if word == CHOSEN_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == TYPE_WORD) =>
            {
                filter.chosen_creature_type = true;
            }
            word if word == NONCHOSEN_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == TYPE_WORD) =>
            {
                filter.excluded_chosen_creature_type = true;
            }
            "token" | "tokens" => filter.token = true,
            "nontoken" => filter.nontoken = true,
            "other" => filter.other = true,
            "tapped" => filter.tapped = true,
            "untapped" => filter.untapped = true,
            "attacking" if !is_negated_word => filter.attacking = true,
            "nonattacking" => filter.nonattacking = true,
            "blocking" if !is_negated_word => filter.blocking = true,
            "nonblocking" => filter.nonblocking = true,
            "blocked" if !is_negated_word => filter.blocked = true,
            "unblocked" if !is_negated_word => filter.unblocked = true,
            "commander" | "commanders" => {
                let prev = idx.checked_sub(1).and_then(|i| all_words.get(i)).copied();
                let prev2 = idx.checked_sub(2).and_then(|i| all_words.get(i)).copied();
                let negated_by_phrase = prev.is_some_and(is_text_negation_word)
                    || (prev.is_some_and(is_article) && prev2.is_some_and(is_text_negation_word));
                if is_negated_word || negated_by_phrase {
                    filter.noncommander = true;
                } else {
                    filter.is_commander = true;
                    match prev {
                        Some("your") => filter.owner = Some(PlayerFilter::You),
                        Some("opponent") | Some("opponents") => {
                            filter.owner = Some(PlayerFilter::Opponent);
                        }
                        Some("their") => filter.owner = Some(pronoun_player_filter.clone()),
                        _ => {}
                    }
                }
            }
            "noncommander" | "noncommanders" => filter.noncommander = true,
            "nonbasic" => {
                filter = filter.without_supertype(Supertype::Basic);
            }
            "colorless" => filter.colorless = true,
            "multicolored" => filter.multicolored = true,
            "monocolored" => filter.monocolored = true,
            "nonhistoric" => filter.nonhistoric = true,
            "historic" if !set_has(&negated_historic_indices, &idx) => filter.historic = true,
            "modified" if !is_negated_word => filter.modified = true,
            _ => {}
        }

        if is_non_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            continue;
        }

        if set_has(&negated_word_indices, &idx) {
            continue;
        }

        if is_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.subtypes);
            saw_subtype = true;
            continue;
        }

        if let Some(card_type) = parse_non_type(word) {
            filter.excluded_card_types.push(card_type);
        }

        if let Some(supertype) = parse_non_supertype(word)
            && !slice_has(&filter.excluded_supertypes, &supertype)
        {
            filter.excluded_supertypes.push(supertype);
        }

        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
        }
        if let Some(subtype) = parse_non_subtype(word)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }

        if let Some(supertype) = parse_supertype_word(word)
            && !slice_has(&filter.supertypes, &supertype)
        {
            filter.supertypes.push(supertype);
        }

        if let Some(card_type) = parse_card_type(word) {
            push_unique(&mut filter.card_types, card_type);
            if is_permanent_type(card_type) {
                saw_permanent_type = true;
            }
        }

        if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique(&mut filter.subtypes, subtype);
            saw_subtype = true;
        }
    }
    if saw_spell && source_linked_exile_reference {
        // "spell ... exiled with this" describes a stack spell with a relation
        // to source-linked exiled cards, not a spell object in exile.
        filter.zone = Some(Zone::Stack);
    }

    let segments = split_lexed_slices_on_or(&segment_tokens);
    let mut segment_types = Vec::new();
    let mut segment_subtypes = Vec::new();
    let mut segment_marker_counts = Vec::new();
    let mut segment_words_lists: Vec<Vec<String>> = Vec::new();

    for segment in &segments {
        let segment_words: Vec<String> = non_article_parser_word_refs(segment)
            .into_iter()
            .map(ToString::to_string)
            .collect();
        segment_words_lists.push(segment_words.clone());
        let mut types = Vec::new();
        let mut subtypes = Vec::new();
        for word in &segment_words {
            if let Some(card_type) = parse_card_type(word) {
                push_unique(&mut types, card_type);
            }
            if let Some(subtype) = parse_subtype_flexible(word) {
                push_unique(&mut subtypes, subtype);
            }
        }
        segment_marker_counts.push(types.len() + subtypes.len());
        if !types.is_empty() {
            segment_types.push(types);
        }
        if !subtypes.is_empty() {
            segment_subtypes.push(subtypes);
        }
    }

    if segments.len() > 1 {
        let qualifier_in_all_segments = |qualifier: &str| {
            segment_words_lists.iter().all(|segment| {
                let segment_refs = segment.iter().map(String::as_str).collect::<Vec<_>>();
                segment_refs.contains(&qualifier)
            })
        };
        let shared_leading_qualifier = |qualifier: &str, opposite: &str| {
            if qualifier_in_all_segments(qualifier) {
                return true;
            }
            if all_words.contains(&opposite) {
                return false;
            }
            let Some(first_segment) = segment_words_lists.first() else {
                return false;
            };
            let first_segment_refs = first_segment.iter().map(String::as_str).collect::<Vec<_>>();
            if !first_segment_refs.contains(&qualifier) {
                return false;
            }
            segment_words_lists.iter().skip(1).all(|segment| {
                let segment_refs = segment.iter().map(String::as_str).collect::<Vec<_>>();
                !segment_refs.contains(&opposite)
            })
        };

        if filter.tapped && !shared_leading_qualifier("tapped", "untapped") {
            filter.tapped = false;
        }
        if filter.untapped && !shared_leading_qualifier("untapped", "tapped") {
            filter.untapped = false;
        }
    }

    if segments.len() > 1 {
        let type_list_candidate = !segment_marker_counts.is_empty()
            && segment_marker_counts.iter().all(|count| *count == 1);

        if type_list_candidate {
            let mut any_types = Vec::new();
            let mut any_subtypes = Vec::new();
            for types in segment_types {
                let Some(card_type) = types.first().copied() else {
                    continue;
                };
                push_unique(&mut any_types, card_type);
            }
            for subtypes in segment_subtypes {
                let Some(subtype) = subtypes.first().copied() else {
                    continue;
                };
                push_unique(&mut any_subtypes, subtype);
            }
            if !any_types.is_empty() {
                filter.card_types = any_types;
            }
            if !any_subtypes.is_empty() {
                filter.subtypes = any_subtypes;
            }
            if !filter.card_types.is_empty() && !filter.subtypes.is_empty() {
                filter.type_or_subtype_union = true;
            }
        }
    } else if let Some(types) = segment_types.into_iter().next() {
        let has_conjunction =
            non_article_token_words_contains_any_word(&segment_tokens, TYPE_LIST_CONJUNCTION_WORDS);
        let has_and = non_article_token_words_contains_any_word(&segment_tokens, &["and"]);
        let has_or = non_article_token_words_contains_any_word(&segment_tokens, &["or"]);
        let has_and_or = non_article_token_words_contains_any_word(&segment_tokens, &["and/or"]);
        if types.len() > 1 {
            if has_conjunction {
                filter.card_types = types;
            } else {
                filter.all_card_types = types;
            }
        } else if types.len() == 1 {
            filter.card_types = types;
        }
        if (has_and_or || (has_and && has_or))
            && !filter.card_types.is_empty()
            && !filter.subtypes.is_empty()
        {
            filter.type_or_subtype_union = true;
        }
    }

    let permanent_type_defaults = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    let and_segments = split_lexed_slices_on_and(&segment_tokens);
    let and_segment_words_lists: Vec<Vec<String>> = and_segments
        .iter()
        .map(|segment| {
            non_article_parser_word_refs(segment)
                .into_iter()
                .map(ToString::to_string)
                .collect()
        })
        .collect();

    let segment_has_standalone_spell = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| word_is_any(word, SPELL_OR_SPELLS_WORDS));
        if !contains_spell {
            return false;
        }

        !segment.iter().any(|word| {
            OBJECT_REFERENCE_NOUN_WORDS.contains(&word.as_str())
                || parse_card_type(word).is_some()
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_nonspell_permanent_head = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| word_is_any(word, SPELL_OR_SPELLS_WORDS));
        if contains_spell {
            return false;
        }

        segment.iter().any(|word| {
            word_is_any(word, PERMANENT_OR_PERMANENTS_WORDS)
                || parse_card_type(word).is_some_and(is_permanent_type)
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_permanent_spell_head = |segment: &[String]| {
        if segment.len() < 2 {
            return false;
        }
        let mut idx = 0usize;
        while idx + 1 < segment.len() {
            let permanent = &segment[idx];
            let spell = &segment[idx + 1];
            if word_is_any(permanent, PERMANENT_OR_PERMANENTS_WORDS)
                && word_is_any(spell, SPELL_OR_SPELLS_WORDS)
            {
                return true;
            }
            idx += 1;
        }
        false
    };
    let has_standalone_spell_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_standalone_spell(segment));
    let has_nonspell_permanent_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_nonspell_permanent_head(segment));
    let has_split_permanent_spell_segments = and_segment_words_lists.len() > 1
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_permanent_spell_head(segment))
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_nonspell_permanent_head(segment));

    if saw_spell && has_standalone_spell_segment && has_nonspell_permanent_segment {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.card_types.clear();
        spell_filter.all_card_types.clear();
        spell_filter.subtypes.clear();
        spell_filter.type_or_subtype_union = false;

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent && has_split_permanent_spell_segments {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.has_mana_cost = false;
        if spell_filter.card_types.is_empty()
            && spell_filter.all_card_types.is_empty()
            && spell_filter.subtypes.is_empty()
        {
            spell_filter.card_types = permanent_type_defaults.clone();
        }

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent {
        if filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
        filter.zone = Some(Zone::Stack);
    } else {
        if saw_permanent && filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
    }

    if filter.any_of.is_empty() {
        if let Some(zone) = filter.zone {
            if saw_spell && zone != Zone::Stack {
                let is_spell_origin_zone = matches!(
                    zone,
                    Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command
                );
                if !is_spell_origin_zone {
                    return Err(CardTextError::ParseError(
                        "spell targets must be on the stack".to_string(),
                    ));
                }
            }
        } else if saw_spell {
            filter.zone = Some(Zone::Stack);
        } else if saw_permanent || saw_permanent_type || saw_subtype {
            filter.zone = Some(Zone::Battlefield);
        }
    }

    if contains_unqualified_spell_word
        && filter.cast_by.is_some()
        && matches!(
            filter.zone,
            Some(Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command)
        )
    {
        filter.owner = None;
    }

    if target_player.is_some() || target_object.is_some() {
        filter = if targets_only {
            filter.targeting_only(target_player.take(), target_object.take())
        } else {
            filter.targeting(target_player.take(), target_object.take())
        };
        if let Some(count) = target_count {
            filter = filter.with_target_count(count);
        } else if targets_only {
            filter = filter.target_count_exact(1);
        }
    }

    if let Some(or_subtype) = legendary_or_subtype
        && filter.any_of.is_empty()
        && slice_has(&filter.supertypes, &Supertype::Legendary)
        && slice_has(&filter.subtypes, &or_subtype)
    {
        let mut legendary_branch = filter.clone();
        legendary_branch.any_of.clear();
        legendary_branch
            .subtypes
            .retain(|subtype| *subtype != or_subtype);

        let mut subtype_branch = filter.clone();
        subtype_branch.any_of.clear();
        subtype_branch
            .supertypes
            .retain(|supertype| *supertype != Supertype::Legendary);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![legendary_branch, subtype_branch];
        filter = disjunction;
    }

    let owner_or_controller_player = all_words.iter().enumerate().find_map(|(idx, _)| {
        parse_owner_or_controller_disjunction_player(&all_words[idx..], &pronoun_player_filter)
            .map(|(player_filter, _)| player_filter)
    });
    if let Some(player_filter) = owner_or_controller_player
        && filter.any_of.is_empty()
    {
        let mut base = filter.clone();
        base.any_of.clear();
        base.owner = None;
        base.controller = None;

        let mut owner_branch = base.clone();
        owner_branch.owner = Some(player_filter.clone());

        let mut controller_branch = base;
        controller_branch.controller = Some(player_filter);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![owner_branch, controller_branch];
        filter = disjunction;
    }

    if has_power_or_toughness_clause && saw_spell {
        let mut power_or_toughness_cmp = None;
        for idx in 0..all_words.len() {
            let (_, value_tokens) = match all_words.get(idx..) {
                Some(["power", "or", "toughness", rest @ ..])
                | Some(["toughness", "or", "power", rest @ ..]) => {
                    (crate::filter::PtReference::Effective, rest)
                }
                _ => continue,
            };
            let Some((cmp, _)) =
                parse_filter_comparison_tokens("power", value_tokens, &clause_words)?
            else {
                continue;
            };
            power_or_toughness_cmp = Some(cmp);
            break;
        }
        if let Some(cmp) = power_or_toughness_cmp {
            let mut base = filter.clone();
            base.any_of.clear();
            base.power = None;
            base.toughness = None;

            let mut power_branch = base.clone();
            power_branch.power = Some(cmp.clone());

            let mut toughness_branch = base;
            toughness_branch.toughness = Some(cmp);

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![power_branch, toughness_branch];
            filter = disjunction;
        }
    }

    if exclude_basic_land_cards {
        apply_basic_land_exception(&mut filter);
    }

    if non_article_token_words_contains_any_word(&segment_tokens, TYPE_LIST_CONJUNCTION_WORDS)
        && !filter.card_types.is_empty()
    {
        filter.all_card_types.clear();
    }

    let has_constraints = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.controller.is_some()
        || filter.owner.is_some()
        || filter.other
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.color_count.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.total_power_toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.any_of.is_empty();

    if !has_constraints {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase (clause: '{}')",
            all_words.join(" ")
        )));
    }

    let has_object_identity = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.color_count.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.total_power_toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || filter.chosen_color
        || filter.chosen_creature_type
        || filter.excluded_chosen_creature_type
        || filter.colors.is_some()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.any_of.is_empty();
    if !has_object_identity {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase lacking object selector (clause: '{}')",
            all_words.join(" ")
        )));
    }

    if vote_winners_only {
        filter = filter.match_tagged(
            TagKey::from(VOTE_WINNERS_TAG),
            TaggedOpbjectRelation::IsTaggedObject,
        );
    }

    if not_on_battlefield && filter.any_of.is_empty() && !matches!(filter.zone, Some(Zone::Stack)) {
        let mut base = filter.clone();
        base.any_of.clear();
        base.zone = None;

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
        ]
        .into_iter()
        .map(|zone| {
            let mut branch = base.clone();
            branch.zone = Some(zone);
            branch
        })
        .collect();
        filter = disjunction;
    }

    // Strict mode: detect structural patterns in the input that indicate
    // unconsumed compound content (e.g. "for each card in your hand AND EACH
    // foretold card you own in exile" where the second clause was silently
    // absorbed into the first filter).
    if strict {
        let tokens = tokens.as_slice();
        let input_words = non_article_parser_word_refs(tokens);
        let all_words = input_words.as_slice();

        // "and each" / "and every" signals a compound count source when
        // the word after "each"/"every" introduces a new filter (type word,
        // zone word, etc.) rather than qualifying the current subject
        // (e.g. "and each other creature" is a subject qualifier, but
        // "and each foretold card you own in exile" is a new clause).
        for (idx, _) in input_words.iter().enumerate() {
            if !word_slice_starts_with_any(&input_words[idx..], STRICT_COMPOUND_COUNT_PREFIXES) {
                continue;
            }
            // A "other than basic land card(s)" exception is stripped before
            // this point, so it never reaches the compound-clause check; guard
            // for it defensively to keep the strict scan stable.
            if word_slice_starts_with(&all_words[idx..], OTHER_THAN_BASIC_LAND_PREFIX) {
                continue;
            }
            // "and each other" is typically a subject qualifier, allow it.
            let after_each = input_words.get(idx + 2).copied();
            if after_each.is_some_and(|w| word_is_any(w, OTHER_OR_ANOTHER_WORDS)) {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "object filter has unconsumed compound clause '{}' (full input: '{}')",
                input_words[idx..].join(" "),
                input_words.join(" "),
            )));
        }

        // "for each" signals a trailing iteration clause that should have
        // been split out by the caller before passing to the filter parser.
        for (idx, _) in input_words.iter().enumerate() {
            if idx > 0 && word_slice_starts_with(&input_words[idx..], STRICT_FOR_EACH_TAIL_PREFIX) {
                return Err(CardTextError::ParseError(format!(
                    "object filter has unconsumed 'for each' clause '{}' (full input: '{}')",
                    input_words[idx..].join(" "),
                    input_words.join(" "),
                )));
            }
        }
    }

    Ok(filter)
}

fn relation_clause_is_inside_aggregate_scope(words: &[&str], relation_start: usize) -> bool {
    let Some(with_idx) = words[..relation_start]
        .iter()
        .rposition(|word| *word == WITH_WORD)
    else {
        return false;
    };
    let prefix = &words[with_idx + 1..relation_start];
    let has_aggregate = prefix
        .iter()
        .any(|word| word_is_any(word, AGGREGATE_SCOPE_WORDS));
    let has_scope_marker = word_slice_contains_any_word(prefix, AGGREGATE_SCOPE_MARKER_WORDS);
    has_aggregate && has_scope_marker
}

fn strip_other_than_basic_land_cards_clause(
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let mut idx = 0usize;
    while idx + 3 < all_words.len() {
        if !word_slice_starts_with(&all_words[idx..], OTHER_THAN_BASIC_LAND_PREFIX) {
            idx += 1;
            continue;
        }

        let mut end = idx + 4;
        if all_words
            .get(end)
            .is_some_and(|word| CARD_OR_CARDS_WORDS.contains(word))
        {
            end += 1;
        }
        all_words.drain(idx..end);
        strip_other_than_basic_land_cards_tokens(segment_tokens);
        return true;
    }

    false
}

fn strip_other_than_basic_land_cards_tokens(segment_tokens: &mut Vec<OwnedLexToken>) {
    let mut idx = 0usize;
    while idx + 3 < segment_tokens.len() {
        if !non_article_token_words_starts_with(
            &segment_tokens[idx..],
            OTHER_THAN_BASIC_LAND_PREFIX,
        ) {
            idx += 1;
            continue;
        }

        let mut end = idx + 4;
        if segment_tokens.get(end).is_some_and(|token| {
            token
                .as_word()
                .is_some_and(|word| CARD_OR_CARDS_WORDS.contains(&word))
        }) {
            end += 1;
        }
        segment_tokens.drain(idx..end);
        return;
    }
}

fn apply_basic_land_exception(filter: &mut ObjectFilter) {
    let mut nonland_branch = filter.clone();
    nonland_branch.any_of.clear();
    push_unique(&mut nonland_branch.excluded_card_types, CardType::Land);

    let mut nonbasic_branch = filter.clone();
    nonbasic_branch.any_of.clear();
    push_unique(&mut nonbasic_branch.excluded_supertypes, Supertype::Basic);

    *filter = ObjectFilter {
        any_of: vec![nonland_branch, nonbasic_branch],
        ..Default::default()
    };
}

fn try_apply_could_be_targeted_by_that_spell_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        ["that", "spell", "could", "target"].as_slice(),
        ["this", "spell", "could", "target"].as_slice(),
        ["it", "could", "target"].as_slice(),
    ] {
        let Some(idx) = find_phrase_start(all_words, phrase) else {
            continue;
        };
        filter.could_be_targeted_by = Some(TargetabilityConstraint::by_stack_object(
            ObjectRef::tagged(TagKey::from(IT_TAG)),
        ));
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}

fn try_apply_distinct_powers_clause(filter: &mut ObjectFilter, all_words: &mut Vec<&str>) -> bool {
    for phrase in [
        ["with", "different", "powers"].as_slice(),
        ["that", "have", "different", "powers"].as_slice(),
        ["that", "has", "different", "powers"].as_slice(),
    ] {
        let Some(idx) = find_phrase_start(all_words, phrase) else {
            continue;
        };
        filter.distinct_powers = true;
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}

fn try_apply_distinct_creature_types_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        ["that", "share", "no", "creature", "types"].as_slice(),
        ["that", "shares", "no", "creature", "types"].as_slice(),
        ["with", "no", "creature", "types", "in", "common"].as_slice(),
    ] {
        let Some(idx) = find_phrase_start(all_words, phrase) else {
            continue;
        };
        filter.distinct_creature_types = true;
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}
