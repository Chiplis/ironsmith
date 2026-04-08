#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, ControlDurationAst, EffectAst, EventValueSpec, IT_TAG, ObjectRefAst,
    OwnedLexToken, PlayerAst, PredicateAst, ReturnControllerAst, SubjectAst, TagKey, TargetAst,
    TextSpan, Verb,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::mana::ManaSymbol;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

use super::super::activation_and_restrictions::{
    find_word_sequence_start, parse_devotion_value_from_add_clause,
};
use super::super::activation_helpers::parse_add_mana;
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::{
    parse_trailing_if_predicate_lexed, parse_trailing_instead_if_predicate_lexed,
    parse_who_player_predicate_lexed, split_trailing_if_clause_lexed,
    split_trailing_unless_clause_lexed,
};
use super::super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_dynamic_cost_modifier_value,
    parse_where_x_value_clause,
};
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::{
    find_index, find_window_by, rfind_index, slice_contains, slice_starts_with, str_strip_suffix,
};
use super::super::util::{
    is_article, is_source_reference_words, mana_pips_from_token, parse_card_type,
    parse_mana_symbol, parse_number, parse_number_word_u32, parse_target_count_range_prefix,
    parse_target_phrase, parse_value, parse_value_expr_words, replace_unbound_x_with_value,
    span_from_tokens, token_index_for_word_index, trim_commas, value_contains_unbound_x, words,
    wrap_target_count,
};
use super::clause_pattern_helpers::extract_subject_player;
use super::creation_handlers::{parse_create, parse_investigate};
use super::for_each_helpers::parse_who_did_this_way_predicate;
use super::sentence_primitives::try_build_unless;
use super::zone_counter_helpers::{parse_convert, parse_put_counters, parse_transform};
use super::zone_handlers::{
    DelayedReturnTimingAst, parse_become, parse_delayed_return_timing_words, parse_destroy,
    parse_discard, parse_equal_to_aggregate_filter_value,
    parse_equal_to_number_of_counters_on_reference_value,
    parse_equal_to_number_of_filter_plus_or_minus_fixed_value,
    parse_equal_to_number_of_filter_value, parse_equal_to_number_of_opponents_you_have_value,
    parse_exchange, parse_exile, parse_flip, parse_get, parse_graveyard_owner_prefix, parse_mill,
    parse_pay, parse_regenerate, parse_remove, parse_return, parse_sacrifice, parse_scry,
    parse_skip, parse_surveil, parse_switch, parse_tap, parse_untap,
    wrap_return_with_delayed_timing,
};

const SOURCE_ATTACHMENT_PREFIXES: &[&[&str]] = &[
    &["this", "equipment"],
    &["this", "aura"],
    &["this", "enchantment"],
    &["this", "artifact"],
];
const ADDITIONAL_PREFIXES: &[&[&str]] = &[&["an", "additional"], &["additional"]];
const FOR_EACH_OPPONENT_WHO_PREFIXES: &[&[&str]] = &[
    &["for", "each", "opponent", "who"],
    &["for", "each", "opponents", "who"],
];
const FOR_EACH_PLAYER_WHO_PREFIXES: &[&[&str]] = &[
    &["for", "each", "player", "who"],
    &["for", "each", "players", "who"],
];
const EACH_OPPONENT_WHO_PREFIXES: &[&[&str]] =
    &[&["each", "opponent", "who"], &["each", "opponents", "who"]];
const EACH_PLAYER_WHO_PREFIXES: &[&[&str]] =
    &[&["each", "player", "who"], &["each", "players", "who"]];
const THAT_PLAYER_PREFIXES: &[&[&str]] = &[&["that", "player"], &["that", "players"]];
const EVENT_AMOUNT_PREFIXES: &[&[&str]] = &[&["that", "much"], &["that", "many"]];
const DAMAGE_TO_EACH_OPPONENT_PREFIXES: &[&[&str]] = &[&["damage", "to", "each", "opponent"]];
const EACH_OF_PREFIXES: &[&[&str]] = &[&["each", "of"]];
const ANY_NUMBER_OF_PREFIXES: &[&[&str]] = &[&["any", "number", "of"]];
const YOU_CONTROL_PREFIXES: &[&[&str]] = &[&["you", "control"], &["you", "controlled"]];
const FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"]];
const EACH_OPPONENT_AND_EACH_PREFIXES: &[&[&str]] = &[&["each", "opponent", "and", "each"]];
const FIRST_CARD_YOU_DRAW_PREFIXES: &[&[&str]] = &[&["the", "first", "card", "you", "draw"]];

pub(crate) fn parse_effect_with_verb(
    verb: Verb,
    subject: Option<SubjectAst>,
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    match verb {
        Verb::Add => parse_add_mana(tokens, subject),
        Verb::Move => parse_move(tokens),
        Verb::Deal => parse_deal_damage(tokens),
        Verb::Draw => parse_draw(tokens, subject),
        Verb::Counter => parse_counter(tokens),
        Verb::Destroy => parse_destroy(tokens),
        Verb::Exile => parse_exile(tokens, subject),
        Verb::Reveal => parse_reveal(tokens, subject),
        Verb::Look => parse_look(tokens, subject),
        Verb::Lose => parse_lose_life(tokens, subject),
        Verb::Gain => {
            if tokens.first().is_some_and(|token| token.is_word("control")) {
                parse_gain_control(tokens, subject)
            } else {
                parse_gain_life(tokens, subject)
            }
        }
        Verb::Put => {
            let has_onto = tokens.iter().any(|token| token.is_word("onto"));
            let has_counter_words = tokens
                .iter()
                .any(|token| token.is_word("counter") || token.is_word("counters"));

            // Prefer zone moves like "... onto the battlefield" over counter placement because
            // "counter(s)" may appear in subordinate clauses (e.g. "mana value equal to the number
            // of charge counters on this artifact").
            if has_onto {
                if let Ok(effect) = parse_put_into_hand(tokens, subject) {
                    Ok(effect)
                } else if has_counter_words {
                    parse_put_counters(tokens)
                } else {
                    parse_put_into_hand(tokens, subject)
                }
            } else if has_counter_words {
                parse_put_counters(tokens)
            } else {
                parse_put_into_hand(tokens, subject)
            }
        }
        Verb::Sacrifice => parse_sacrifice(tokens, subject),
        Verb::Create => parse_create(tokens, subject),
        Verb::Investigate => parse_investigate(tokens),
        Verb::Proliferate => parse_proliferate(tokens),
        Verb::Tap => parse_tap(tokens),
        Verb::Attach => parse_attach(tokens),
        Verb::Untap => parse_untap(tokens),
        Verb::Scry => parse_scry(tokens, subject),
        Verb::Discard => parse_discard(tokens, subject),
        Verb::Transform => parse_transform(tokens),
        Verb::Convert => parse_convert(tokens),
        Verb::Flip => parse_flip(tokens),
        Verb::Regenerate => parse_regenerate(tokens),
        Verb::Mill => parse_mill(tokens, subject),
        Verb::Get => parse_get(tokens, subject),
        Verb::Remove => parse_remove(tokens),
        Verb::Return => parse_return(tokens),
        Verb::Exchange => parse_exchange(tokens, subject),
        Verb::Become => parse_become(tokens, subject),
        Verb::Switch => parse_switch(tokens),
        Verb::Skip => parse_skip(tokens, subject),
        Verb::Surveil => parse_surveil(tokens, subject),
        Verb::Shuffle => parse_shuffle(tokens, subject),
        Verb::Reorder => parse_reorder(tokens, subject),
        Verb::Pay => parse_pay(tokens, subject),
        Verb::Detain => parse_detain(tokens),
        Verb::Goad => parse_goad(tokens),
    }
}

fn parse_proliferate(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::Proliferate {
            count: Value::Fixed(1),
        });
    }

    let (count, used) = if let Some(first) = tokens.first().and_then(OwnedLexToken::as_word) {
        match first {
            "once" => (Value::Fixed(1), 1),
            "twice" => (Value::Fixed(2), 1),
            _ => parse_value(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing proliferate count (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                ))
            })?,
        }
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing proliferate count (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    };

    let trailing = trim_commas(&tokens[used..]);
    let trailing_words = crate::cards::builders::parser::token_word_refs(&trailing);
    let trailing_ok = trailing_words.is_empty()
        || trailing_words.as_slice() == ["time"]
        || trailing_words.as_slice() == ["times"];
    if !trailing_ok {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing proliferate clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::Proliferate { count })
}

fn parse_library_nth_from_top_destination(tokens: &[OwnedLexToken]) -> Option<Value> {
    let library_idx = find_index(tokens, |token: &OwnedLexToken| {
        token.is_word("library") || token.is_word("libraries")
    })?;
    let tail_tokens = trim_commas(&tokens[library_idx + 1..]);
    if tail_tokens.is_empty() {
        return None;
    }

    let filtered_tail: Vec<&str> = crate::cards::builders::parser::token_word_refs(&tail_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let fixed_position = match filtered_tail.as_slice() {
        ["second", "from", "top"] => Some(2),
        ["third", "from", "top"] => Some(3),
        ["fourth", "from", "top"] => Some(4),
        ["fifth", "from", "top"] => Some(5),
        _ => None,
    };
    if let Some(position) = fixed_position {
        return Some(Value::Fixed(position));
    }

    let amount_start = match filtered_tail.as_slice() {
        ["just", "beneath", "top", ..] => Some(3usize),
        ["beneath", "top", ..] => Some(2usize),
        _ => None,
    }?;
    let amount_tokens = filtered_tail[amount_start..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (amount, used) = parse_value(&amount_tokens)?;
    let amount_words = crate::cards::builders::parser::token_word_refs(&amount_tokens);
    if !matches!(amount_words.get(used).copied(), Some("card" | "cards")) {
        return None;
    }
    if used + 1 > amount_words.len() {
        return None;
    }
    if amount_words[used + 1..] != ["of", "that", "library"] {
        return None;
    }

    Some(Value::Add(Box::new(amount), Box::new(Value::Fixed(1))))
}

pub(crate) fn parse_look(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_hand_owner(words: &[&str]) -> Option<(PlayerAst, usize)> {
        if slice_starts_with(&words, &["your", "hand"]) {
            return Some((PlayerAst::You, 2));
        }
        if slice_starts_with(&words, &["each", "player", "hand"])
            || slice_starts_with(&words, &["each", "players", "hand"])
        {
            return Some((PlayerAst::Any, 3));
        }
        if slice_starts_with(&words, &["their", "hand"]) {
            return Some((PlayerAst::That, 2));
        }
        if slice_starts_with(&words, &["that", "player", "hand"])
            || slice_starts_with(&words, &["that", "players", "hand"])
        {
            return Some((PlayerAst::That, 3));
        }
        if slice_starts_with(&words, &["target", "player", "hand"])
            || slice_starts_with(&words, &["target", "players", "hand"])
        {
            return Some((PlayerAst::Target, 3));
        }
        if slice_starts_with(&words, &["target", "opponent", "hand"])
            || slice_starts_with(&words, &["target", "opponents", "hand"])
        {
            return Some((PlayerAst::TargetOpponent, 3));
        }
        if slice_starts_with(&words, &["opponent", "hand"])
            || slice_starts_with(&words, &["opponents", "hand"])
        {
            return Some((PlayerAst::Opponent, 2));
        }
        if slice_starts_with(&words, &["his", "or", "her", "hand"]) {
            return Some((PlayerAst::That, 4));
        }
        None
    }

    fn parse_library_owner(words: &[&str]) -> Option<(PlayerAst, usize)> {
        if slice_starts_with(&words, &["your", "library"]) {
            return Some((PlayerAst::You, 2));
        }
        if slice_starts_with(&words, &["each", "player", "library"])
            || slice_starts_with(&words, &["each", "players", "library"])
        {
            return Some((PlayerAst::Any, 3));
        }
        if slice_starts_with(&words, &["their", "library"]) {
            return Some((PlayerAst::That, 2));
        }
        if slice_starts_with(&words, &["that", "player", "library"])
            || slice_starts_with(&words, &["that", "players", "library"])
        {
            return Some((PlayerAst::That, 3));
        }
        if slice_starts_with(&words, &["target", "player", "library"])
            || slice_starts_with(&words, &["target", "players", "library"])
        {
            return Some((PlayerAst::Target, 3));
        }
        if slice_starts_with(&words, &["target", "opponent", "library"])
            || slice_starts_with(&words, &["target", "opponents", "library"])
        {
            return Some((PlayerAst::TargetOpponent, 3));
        }
        if slice_starts_with(&words, &["its", "owner", "library"])
            || slice_starts_with(&words, &["its", "owners", "library"])
        {
            return Some((PlayerAst::ItsOwner, 3));
        }
        if slice_starts_with(&words, &["his", "or", "her", "library"]) {
            return Some((PlayerAst::That, 4));
        }
        None
    }

    // "Look at the top N cards of your library."
    let mut clause_tokens = trim_commas(tokens);
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("at"))
    {
        clause_tokens = trim_commas(&clause_tokens[1..]);
    }
    let clause_word_storage = TokenWordView::new(&clause_tokens).owned_words();
    let clause_words = clause_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let mut hand_tokens = clause_tokens.clone();
    while hand_tokens
        .first()
        .is_some_and(|token| token.is_word("the") || token.is_word("a") || token.is_word("an"))
    {
        hand_tokens = hand_tokens[1..].to_vec();
    }
    let hand_word_storage = TokenWordView::new(&hand_tokens).owned_words();
    let hand_words = hand_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if let Some((player, used_words)) = parse_hand_owner(&hand_words) {
        if used_words < hand_words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing look clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let target = match player {
            PlayerAst::You => TargetAst::Player(PlayerFilter::You, None),
            PlayerAst::Opponent => TargetAst::Player(PlayerFilter::Opponent, None),
            PlayerAst::Target => TargetAst::Player(
                PlayerFilter::target_player(),
                span_from_tokens(&hand_tokens),
            ),
            PlayerAst::TargetOpponent => TargetAst::Player(
                PlayerFilter::target_opponent(),
                span_from_tokens(&hand_tokens),
            ),
            PlayerAst::That => TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            PlayerAst::Any => {
                return Ok(EffectAst::ForEachPlayer {
                    effects: vec![EffectAst::LookAtHand {
                        target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                    }],
                });
            }
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported look clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        };

        return Ok(EffectAst::LookAtHand { target });
    }

    let Some(top_idx) = find_index(&clause_tokens, |t| t.is_word("top")) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if top_idx + 1 >= clause_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing look top noun (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut idx = top_idx + 1;
    let count = if clause_tokens
        .get(idx)
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|w| w == "card" || w == "cards")
    {
        Value::Fixed(1)
    } else {
        let (value, used) = parse_value(&clause_tokens[idx..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing look count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        idx += used;
        value
    };

    // Consume "card(s)"
    if clause_tokens
        .get(idx)
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|w| w == "card" || w == "cards")
    {
        idx += 1;
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing look card noun (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    // Consume "of <player> library"
    if !clause_tokens.get(idx).is_some_and(|t| t.is_word("of")) {
        return Err(CardTextError::ParseError(format!(
            "missing 'of' in look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 1;
    let mut owner_tokens = &clause_tokens[idx..];
    while owner_tokens
        .first()
        .is_some_and(|t| t.is_word("the") || t.is_word("a") || t.is_word("an"))
    {
        owner_tokens = &owner_tokens[1..];
    }
    let owner_word_storage = TokenWordView::new(owner_tokens).owned_words();
    let owner_words = owner_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let (player, used_words) = parse_library_owner(&owner_words)
        .or_else(|| {
            // If the clause uses a subject ("target player looks ..."), treat that as the default.
            subject.and_then(|s| match s {
                SubjectAst::Player(p) => Some((p, 0)),
                _ => None,
            })
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported look library owner (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    // No trailing words supported for now (based on word tokens).
    if used_words < owner_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if matches!(player, PlayerAst::Any) {
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::LookAtTopCards {
                player: PlayerAst::That,
                count,
                tag: TagKey::from(IT_TAG),
            }],
        });
    }

    Ok(EffectAst::LookAtTopCards {
        player,
        count,
        tag: TagKey::from(IT_TAG),
    })
}

pub(crate) fn parse_reorder(
    tokens: &[OwnedLexToken],
    _subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause = crate::cards::builders::parser::token_word_refs(tokens).join(" ");
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing reorder target".to_string(),
        ));
    }

    let Some((player, consumed)) = parse_graveyard_owner_prefix(&clause_words) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause (clause: '{clause}')"
        )));
    };
    if !matches!(
        player,
        PlayerAst::You | PlayerAst::That | PlayerAst::ItsController | PlayerAst::ItsOwner
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause (clause: '{clause}')"
        )));
    }
    let rest = &clause_words[consumed..];

    if !rest.is_empty() && rest != ["as", "you", "choose"] {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause tail (clause: '{clause}')"
        )));
    }

    Ok(EffectAst::ReorderGraveyard { player })
}

pub(crate) fn parse_shuffle(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_library_destination_player(
        words: &[&str],
        default_player: PlayerAst,
    ) -> Option<(PlayerAst, usize)> {
        match words {
            ["library", ..] => Some((default_player, 1)),
            ["your", "library", ..] => Some((PlayerAst::You, 2)),
            ["their", "library", ..] => Some((
                if matches!(default_player, PlayerAst::Implicit) {
                    PlayerAst::ItsController
                } else {
                    default_player
                },
                2,
            )),
            ["that", "player", "library", ..] => Some((PlayerAst::That, 3)),
            ["that", "players", "library", ..] => Some((PlayerAst::That, 3)),
            ["its", "owner", "library", ..] => Some((PlayerAst::ItsOwner, 3)),
            ["its", "owners", "library", ..] => Some((PlayerAst::ItsOwner, 3)),
            ["his", "or", "her", "library", ..] => Some((
                if matches!(default_player, PlayerAst::Implicit) {
                    PlayerAst::ItsController
                } else {
                    default_player
                },
                4,
            )),
            _ => None,
        }
    }

    fn is_supported_shuffle_source_tail(words: &[&str]) -> bool {
        matches!(
            words,
            [] | ["from", "graveyard"]
                | ["from", "your", "graveyard"]
                | ["from", "their", "graveyard"]
                | ["from", "that", "player", "graveyard"]
                | ["from", "that", "players", "graveyard"]
                | ["from", "its", "owner", "graveyard"]
                | ["from", "its", "owners", "graveyard"]
                | ["from", "his", "or", "her", "graveyard"]
        )
    }

    fn is_simple_library_phrase(words: &[&str]) -> bool {
        matches!(
            words,
            ["library"]
                | ["your", "library"]
                | ["their", "library"]
                | ["that", "player", "library"]
                | ["that", "players", "library"]
                | ["its", "owner", "library"]
                | ["its", "owners", "library"]
                | ["his", "or", "her", "library"]
        )
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if tokens.is_empty() {
        // Support standalone "Shuffle." clauses. If the sentence includes an explicit player
        // subject, use it; otherwise return an implicit player that can be filled in by the
        // carry-context logic (and compiles to "you" by default).
        return Ok(EffectAst::ShuffleLibrary { player });
    }

    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if let Some(into_idx) = find_index(&clause_words, |word| *word == "into") {
        let target_words = &clause_words[..into_idx];
        let destination_words: Vec<&str> = clause_words[into_idx + 1..]
            .iter()
            .copied()
            .filter(|word| !is_article(word))
            .collect();
        if matches!(
            target_words,
            ["it"] | ["them"] | ["that", "card"] | ["those", "cards"]
        ) && let Some((destination_player, consumed)) =
            parse_library_destination_player(&destination_words, player)
        {
            let trailing_words = &destination_words[consumed..];
            if is_supported_shuffle_source_tail(trailing_words) {
                return Ok(EffectAst::ForEachTagged {
                    tag: TagKey::from(IT_TAG),
                    effects: vec![
                        EffectAst::MoveToZone {
                            target: TargetAst::Tagged(
                                TagKey::from(IT_TAG),
                                span_from_tokens(tokens),
                            ),
                            zone: Zone::Library,
                            to_top: false,
                            battlefield_controller: ReturnControllerAst::Preserve,
                            battlefield_tapped: false,
                            attached_to: None,
                        },
                        EffectAst::ShuffleLibrary {
                            player: destination_player,
                        },
                    ],
                });
            }
        }

        let consult_style_remainder_shuffle = slice_starts_with(&target_words, &["the", "rest"])
            || (slice_starts_with(&target_words, &["all", "other"])
                && slice_contains(&target_words, &"cards")
                && (slice_contains(&target_words, &"revealed")
                    || slice_contains(&target_words, &"exiled")));
        if consult_style_remainder_shuffle
            && let Some((destination_player, consumed)) =
                parse_library_destination_player(&destination_words, player)
            && is_supported_shuffle_source_tail(&destination_words[consumed..])
        {
            return Ok(EffectAst::ShuffleLibrary {
                player: destination_player,
            });
        }
    }

    if matches!(player, PlayerAst::ItsOwner)
        && matches!(
            clause_words.as_slice(),
            ["them", "into", "their", "libraries"]
                | ["them", "into", "their", "library"]
                | ["those", "cards", "into", "their", "libraries"]
                | ["those", "cards", "into", "their", "library"]
        )
    {
        return Ok(EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: vec![
                EffectAst::MoveToZone {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                    zone: Zone::Library,
                    to_top: true,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                },
                EffectAst::ShuffleLibrary {
                    player: PlayerAst::ItsOwner,
                },
            ],
        });
    }
    if grammar::contains_word(tokens, "graveyard")
        || grammar::contains_word(tokens, "cards")
        || grammar::contains_word(tokens, "card")
        || grammar::contains_word(tokens, "into")
        || grammar::contains_word(tokens, "from")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported shuffle clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_simple_library_phrase(&clause_words) {
        return Ok(EffectAst::ShuffleLibrary { player });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported shuffle clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_goad(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError("missing goad target".to_string()));
    }

    let target_words = crate::cards::builders::parser::token_word_refs(&target_tokens);
    if target_words.as_slice() == ["it"] || target_words.as_slice() == ["them"] {
        return Ok(EffectAst::Goad {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens)),
        });
    }

    let target = parse_target_phrase(&target_tokens)?;
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::Goad { target })
}

pub(crate) fn parse_detain(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing detain target".to_string(),
        ));
    }

    let target_words = crate::cards::builders::parser::token_word_refs(&target_tokens);
    if matches!(target_words.as_slice(), ["it"] | ["them"]) {
        return Ok(EffectAst::Detain {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens)),
        });
    }

    Ok(EffectAst::Detain {
        target: parse_target_phrase(&target_tokens)?,
    })
}

pub(crate) fn parse_attach_object_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let object_words = crate::cards::builders::parser::token_word_refs(tokens);
    let object_span = span_from_tokens(tokens);
    if object_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object to attach".to_string(),
        ));
    }

    let is_source_attachment = is_source_reference_words(&object_words)
        || grammar::words_match_any_prefix(tokens, SOURCE_ATTACHMENT_PREFIXES).is_some();
    if is_source_attachment {
        return Ok(TargetAst::Source(object_span));
    }

    if matches!(object_words.as_slice(), ["it"] | ["them"]) {
        return Ok(TargetAst::Tagged(TagKey::from(IT_TAG), object_span));
    }

    let mut tagged_filter = ObjectFilter::default();
    if matches!(
        object_words.as_slice(),
        ["that", "equipment"] | ["those", "equipment"]
    ) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Artifact);
        tagged_filter.subtypes.push(Subtype::Equipment);
    } else if matches!(
        object_words.as_slice(),
        ["that", "aura"] | ["those", "auras"]
    ) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Enchantment);
        tagged_filter.subtypes.push(Subtype::Aura);
    } else if matches!(
        object_words.as_slice(),
        ["that", "artifact"] | ["those", "artifacts"]
    ) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Artifact);
    } else if object_words.as_slice() == ["that", "enchantment"] {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Enchantment);
    }

    if tagged_filter.zone.is_some() {
        tagged_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(TargetAst::Object(tagged_filter, object_span, None));
    }

    if tokens.first().is_some_and(|token| token.is_word("target"))
        && let Some((head_slice, _after_attached_to)) =
            super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
                use winnow::Parser as _;
                super::super::grammar::primitives::phrase(&["attached", "to"]).void()
            })
    {
        let head_tokens = trim_commas(head_slice);
        if !head_tokens.is_empty() {
            return parse_target_phrase(&head_tokens);
        }
    }

    parse_target_phrase(tokens)
}

pub(crate) fn parse_attach(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "attach clause missing object and destination".to_string(),
        ));
    }

    if tokens.first().is_some_and(|token| token.is_word("to")) {
        let rest = trim_commas(&tokens[1..]);
        let Some(first) = rest.first() else {
            return Err(CardTextError::ParseError(format!(
                "attach clause missing object or destination (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        if first.is_word("it") || first.is_word("them") {
            let target_tokens = vec![first.clone()];
            let object_tokens = trim_commas(&rest[1..]);
            if object_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "attach clause missing object or destination (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens));
            let object = parse_attach_object_phrase(&object_tokens)?;
            return Ok(EffectAst::Attach { object, target });
        }
    }

    let Some(to_idx) = rfind_index(tokens, |token| token.is_word("to")) else {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing destination (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if to_idx == 0 || to_idx + 1 >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing object or destination (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let object_tokens = trim_commas(&tokens[..to_idx]);
    let target_tokens = trim_commas(&tokens[to_idx + 1..]);
    if object_tokens.is_empty() || target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing object or destination (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let object = parse_attach_object_phrase(&object_tokens)?;
    let target_words = crate::cards::builders::parser::token_word_refs(&target_tokens);
    let target = if matches!(target_words.as_slice(), ["it"] | ["them"]) {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
    } else {
        parse_target_phrase(&target_tokens)?
    };

    Ok(EffectAst::Attach { object, target })
}

pub(crate) fn parse_deal_damage(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let tokens =
        if let Some((_, rest)) = grammar::words_match_any_prefix(tokens, ADDITIONAL_PREFIXES) {
            rest
        } else {
            tokens
        };
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["damage", "to", "each", "opponent", "equal", "to"])
        .is_some()
        && grammar::contains_word(tokens, "number")
        && grammar::contains_word(tokens, "cards")
        && grammar::contains_word(tokens, "hand")
    {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: Value::CardsInHand(PlayerFilter::IteratedPlayer),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }
    let is_divided_as_you_choose_clause = grammar::contains_word(tokens, "divided")
        && grammar::contains_word(tokens, "choose")
        && grammar::contains_word(tokens, "among");
    if is_divided_as_you_choose_clause {
        if let Some((value, used)) = parse_value(tokens) {
            return parse_divided_damage_with_amount(tokens, value, used);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported divided-damage distribution clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_deal_damage_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_deal_damage_to_target_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, EVENT_AMOUNT_PREFIXES) {
        return parse_deal_damage_with_amount(
            tokens,
            Value::EventValue(EventValueSpec::Amount),
            prefix.len(),
        );
    }

    if let Some((value, used)) = parse_value(tokens) {
        return parse_deal_damage_with_amount(tokens, value, used);
    }

    if grammar::words_match_any_prefix(tokens, DAMAGE_TO_EACH_OPPONENT_PREFIXES).is_some()
        && grammar::contains_word(tokens, "number")
        && grammar::contains_word(tokens, "cards")
        && grammar::contains_word(tokens, "hand")
    {
        let value = Value::CardsInHand(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: value,
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }

    Err(CardTextError::ParseError(format!(
        "missing damage amount (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_deal_damage_to_target_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["damage", "to"]).is_none() {
        return Ok(None);
    }

    let Some(equal_word_idx) = grammar::words_find_phrase(tokens, &["equal", "to"]) else {
        return Ok(None);
    };
    let Some(equal_token_idx) = token_index_for_word_index(tokens, equal_word_idx) else {
        return Ok(None);
    };

    let mut target_tokens = trim_commas(&tokens[1..equal_token_idx]);
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens.remove(0);
    }
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let amount = parse_add_mana_equal_amount_value(tokens)
        .or(parse_equal_to_aggregate_filter_value(tokens))
        .or(parse_equal_to_number_of_filter_value(tokens))
        .or(parse_dynamic_cost_modifier_value(tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_words = crate::cards::builders::parser::token_word_refs(&target_tokens);
    if target_words.as_slice() == ["each", "player"]
        || target_words.as_slice() == ["each", "players"]
    {
        return Ok(Some(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    if target_words.as_slice() == ["each", "opponent"]
        || target_words.as_slice() == ["each", "opponents"]
        || target_words.as_slice() == ["each", "other", "player"]
        || target_words.as_slice() == ["each", "other", "players"]
    {
        return Ok(Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::DealDamage { amount, target }))
}

pub(crate) fn parse_deal_damage_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["damage", "equal", "to"]).is_none() {
        return Ok(None);
    }

    let mut target_to_idx = None;
    for idx in 3..tokens.len() {
        if !tokens[idx].is_word("to") {
            continue;
        }
        let tail_words = crate::cards::builders::parser::token_word_refs(&tokens[idx + 1..]);
        if tail_words.is_empty() {
            continue;
        }
        let looks_like_target = grammar::contains_word(&tokens[idx + 1..], "target")
            || matches!(
                tail_words.first().copied(),
                Some(
                    "any"
                        | "each"
                        | "all"
                        | "it"
                        | "itself"
                        | "them"
                        | "him"
                        | "her"
                        | "that"
                        | "this"
                        | "you"
                        | "player"
                        | "opponent"
                        | "creature"
                        | "planeswalker"
                )
            );
        if looks_like_target {
            target_to_idx = Some(idx);
            break;
        }
    }

    let Some(target_to_idx) = target_to_idx else {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let amount_tokens = &tokens[..target_to_idx];
    let amount = parse_add_mana_equal_amount_value(amount_tokens)
        .or(parse_equal_to_aggregate_filter_value(amount_tokens))
        .or(parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
            amount_tokens,
        ))
        .or(parse_equal_to_number_of_filter_value(amount_tokens))
        .or(parse_equal_to_number_of_opponents_you_have_value(
            amount_tokens,
        ))
        .or(parse_equal_to_number_of_counters_on_reference_value(
            amount_tokens,
        ))
        .or(parse_dynamic_cost_modifier_value(amount_tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let target_tokens = &tokens[target_to_idx + 1..];
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut normalized_target_tokens = target_tokens;
    if grammar::words_match_any_prefix(target_tokens, EACH_OF_PREFIXES).is_some() {
        let each_of_tokens = &target_tokens[2..];
        if grammar::contains_word(each_of_tokens, "target") {
            normalized_target_tokens = each_of_tokens;
        }
    }
    if grammar::words_match_any_prefix(
        normalized_target_tokens,
        &[&["each", "player"], &["each", "players"]],
    )
    .is_some()
    {
        return Ok(Some(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    if grammar::words_match_any_prefix(
        normalized_target_tokens,
        &[
            &["each", "opponent"],
            &["each", "opponents"],
            &["each", "other", "player"],
            &["each", "other", "players"],
        ],
    )
    .is_some()
    {
        return Ok(Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    let target = parse_target_phrase(normalized_target_tokens)?;
    Ok(Some(EffectAst::DealDamage { amount, target }))
}

fn parse_divided_damage_target(
    target_tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let Some(among_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        token.is_word("among")
    }) else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage targets after 'among' (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(target_tokens).join(" ")
        )));
    };
    let among_tail = trim_commas(&target_tokens[among_idx + 1..]);
    let among_words = crate::cards::builders::parser::token_word_refs(&among_tail);
    let Some(target_idx) = find_index(&among_words, |word| matches!(*word, "target" | "targets"))
    else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage target phrase (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(target_tokens).join(" ")
        )));
    };

    let max_targets = among_words[..target_idx]
        .iter()
        .filter_map(|word| parse_number_word_u32(word))
        .max()
        .unwrap_or(0);
    if max_targets == 0
        && grammar::words_match_any_prefix(&among_tail, ANY_NUMBER_OF_PREFIXES).is_none()
    {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage target count (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(target_tokens).join(" ")
        )));
    }

    let target_phrase_tokens = &among_tail[target_idx..];
    let base_target =
        if among_words[target_idx..] == ["target"] || among_words[target_idx..] == ["targets"] {
            TargetAst::AnyTarget(span_from_tokens(target_phrase_tokens))
        } else {
            parse_target_phrase(target_phrase_tokens)?
        };
    let count = if grammar::words_match_any_prefix(&among_tail, ANY_NUMBER_OF_PREFIXES).is_some() {
        ChoiceCount::any_number()
    } else {
        ChoiceCount::up_to(max_targets as usize)
    };
    Ok(TargetAst::WithCount(Box::new(base_target), count))
}

fn parse_divided_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let rest = &tokens[used..];
    if !rest.first().is_some_and(|token| token.is_word("damage")) {
        return Err(CardTextError::ParseError(format!(
            "missing damage keyword in divided-damage clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }
    let mut target_tokens = &rest[1..];
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens = &target_tokens[1..];
    }
    let target = parse_divided_damage_target(target_tokens)?;
    Ok(EffectAst::DealDistributedDamage { amount, target })
}

pub(crate) fn parse_deal_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let rest = &tokens[used..];
    let Some(word) = rest.first().and_then(OwnedLexToken::as_word) else {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    };
    if word != "damage" {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    }

    let mut target_tokens = &rest[1..];
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens = &target_tokens[1..];
    }
    if let Some(among_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        token.is_word("among")
    }) {
        let among_tail = &target_tokens[among_idx + 1..];
        if among_tail.iter().any(|token| token.is_word("target"))
            && among_tail.iter().any(|token| {
                token.is_word("player")
                    || token.is_word("players")
                    || token.is_word("creature")
                    || token.is_word("creatures")
            })
        {
            target_tokens = among_tail;
        }
    }

    if target_tokens.iter().any(|token| token.is_word("where")) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing where damage clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    if let Some(instead_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        token.is_word("instead")
    }) && target_tokens
        .get(instead_idx + 1)
        .is_some_and(|token| token.is_word("if"))
    {
        let pre_target_tokens = trim_commas(&target_tokens[..instead_idx]);
        let predicate = if let Some(predicate) =
            parse_instead_if_control_predicate(&trim_commas(&target_tokens[instead_idx + 2..]))?
        {
            predicate
        } else {
            parse_trailing_instead_if_predicate_lexed(&target_tokens[instead_idx..]).ok_or_else(
                || {
                    CardTextError::ParseError(format!(
                        "unsupported trailing instead-if clause in damage effect (clause: '{}')",
                        crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                    ))
                },
            )?
        };
        let target = if pre_target_tokens.is_empty() {
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
        } else {
            parse_target_phrase(&pre_target_tokens)?
        };
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target,
            }],
            if_false: Vec::new(),
        });
    }

    if let Some(spec) = split_trailing_if_clause_lexed(target_tokens) {
        let target = parse_target_phrase(spec.leading_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate: spec.predicate,
            if_true: vec![EffectAst::DealDamage { amount, target }],
            if_false: Vec::new(),
        });
    }

    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
    {
        let predicate = parse_trailing_if_predicate_lexed(target_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported trailing if clause in damage effect (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            ))
        })?;
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::DealDamage {
                amount,
                // Follow-up "deals N damage if ..." clauses can omit the target and rely
                // on parser-level merge with a prior damage sentence.
                target: TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None),
            }],
            if_false: Vec::new(),
        });
    }

    if find_index(&target_tokens, |token| token.is_word("if")).is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing if clause in damage effect (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let target_words = crate::cards::builders::parser::token_word_refs(target_tokens);
    if target_words.as_slice() == ["instead"] {
        return Ok(EffectAst::DealDamage {
            amount,
            target: TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None),
        });
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_OF_PREFIXES).is_some() {
        let each_of_tokens = &target_tokens[2..];
        let each_of_words = crate::cards::builders::parser::token_word_refs(each_of_tokens);
        if matches!(
            each_of_words.as_slice(),
            ["up", "to", _, "target"] | ["up", "to", _, "targets"]
        ) && let Some(count) = parse_number_word_u32(each_of_words[2])
        {
            let target = TargetAst::WithCount(
                Box::new(TargetAst::AnyTarget(span_from_tokens(each_of_tokens))),
                ChoiceCount::up_to(count as usize),
            );
            return Ok(EffectAst::DealDamage { amount, target });
        }
        if grammar::contains_word(each_of_tokens, "target") {
            let target = parse_target_phrase(each_of_tokens)?;
            return Ok(EffectAst::DealDamage { amount, target });
        }
    }
    if target_words.as_slice() == ["each", "player"]
        || target_words.as_slice() == ["each", "players"]
    {
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }
    if target_words.as_slice() == ["each", "opponent"]
        || target_words.as_slice() == ["each", "opponents"]
    {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_OPPONENT_WHO_PREFIXES).is_some()
        && grammar::words_find_phrase(target_tokens, &["this", "way"]).is_some()
    {
        let predicate = parse_who_did_this_way_predicate(&target_tokens[2..])?;
        return Ok(EffectAst::ForEachOpponentDid {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
            predicate,
        });
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_PLAYER_WHO_PREFIXES).is_some()
        && grammar::words_find_phrase(target_tokens, &["this", "way"]).is_some()
    {
        let predicate = parse_who_did_this_way_predicate(&target_tokens[2..])?;
        return Ok(EffectAst::ForEachPlayerDid {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
            predicate,
        });
    }

    if matches!(target_words.first(), Some(&"each") | Some(&"all"))
        && let Some(and_each_idx) = find_window_by(&target_words, 3, |window| {
            window == ["and", "each", "player"] || window == ["and", "each", "players"]
        })
        && and_each_idx >= 1
        && and_each_idx + 3 == target_words.len()
    {
        let filter_tokens = &target_tokens[1..and_each_idx];
        let mut filter = parse_object_filter(filter_tokens, false)?;
        if filter.controller.is_none() {
            filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![
                EffectAst::DealDamage {
                    amount: amount.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                },
                EffectAst::DealDamageEach {
                    amount: amount.clone(),
                    filter,
                },
            ],
        });
    }

    if grammar::words_match_any_prefix(target_tokens, EACH_OPPONENT_AND_EACH_PREFIXES).is_some()
        && grammar::contains_word(target_tokens, "creature")
        && grammar::contains_word(target_tokens, "planeswalker")
        && (grammar::words_find_phrase(target_tokens, &["they", "control"]).is_some()
            || grammar::words_find_phrase(target_tokens, &["that", "player", "controls"]).is_some())
    {
        let mut filter = ObjectFilter::default();
        filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::DealDamage {
                    amount: amount.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                },
                EffectAst::DealDamageEach {
                    amount: amount.clone(),
                    filter,
                },
            ],
        });
    }

    if matches!(target_words.first(), Some(&"each") | Some(&"all")) {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter_tokens = &target_tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::DealDamageEach {
            amount: amount.clone(),
            filter,
        });
    }

    if let Some(at_idx) = find_index(&target_tokens, |token| token.is_word("at")) {
        let timing_words =
            crate::cards::builders::parser::token_word_refs(&target_tokens[at_idx..]);
        let matches_end_of_combat = timing_words.as_slice() == ["at", "end", "of", "combat"]
            || timing_words.as_slice() == ["at", "the", "end", "of", "combat"];
        if matches_end_of_combat && at_idx >= 1 {
            let pre_target_tokens = trim_commas(&target_tokens[..at_idx]);
            if !pre_target_tokens.is_empty() {
                let target = parse_target_phrase(&pre_target_tokens)?;
                return Ok(EffectAst::DelayedUntilEndOfCombat {
                    effects: vec![EffectAst::DealDamage { amount, target }],
                });
            }
        }
    }

    let target = parse_target_phrase(&target_tokens)?;
    Ok(EffectAst::DealDamage { amount, target })
}

pub(crate) fn parse_instead_if_control_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let starts_with_you_control =
        grammar::words_match_any_prefix(tokens, YOU_CONTROL_PREFIXES).is_some();
    if !starts_with_you_control {
        return Ok(None);
    }

    let mut filter_tokens = &tokens[2..];
    let mut min_count: Option<u32> = None;
    if let Some((count, used)) = parse_number(filter_tokens)
        && count > 1
    {
        let tail = &filter_tokens[used..];
        if tail.first().is_some_and(|token| token.is_word("or"))
            && tail.get(1).is_some_and(|token| token.is_word("more"))
        {
            min_count = Some(count);
            filter_tokens = &tail[2..];
        } else if tail.first().is_some_and(|token| token.is_word("or"))
            && tail.get(1).is_some_and(|token| token.is_word("fewer"))
        {
            // Keep unsupported "or fewer" variants as plain control checks for now.
            filter_tokens = &tail[2..];
        }
    }
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("at"))
        && filter_tokens
            .get(1)
            .is_some_and(|token| token.is_word("least"))
        && let Some((count, used)) = parse_number(&filter_tokens[2..])
        && count > 1
    {
        min_count = Some(count);
        filter_tokens = &filter_tokens[2 + used..];
    }
    let cut_markers: &[&[&str]] = &[&["as", "you", "cast", "this", "spell"], &["this", "turn"]];
    for marker in cut_markers {
        let filter_words = crate::cards::builders::parser::token_word_refs(filter_tokens);
        if let Some(idx) = find_window_by(&filter_words, marker.len(), |window| window == *marker) {
            let cut_idx =
                token_index_for_word_index(filter_tokens, idx).unwrap_or(filter_tokens.len());
            filter_tokens = &filter_tokens[..cut_idx];
            break;
        }
    }
    let mut filter_tokens = trim_commas(filter_tokens);
    let filter_words = crate::cards::builders::parser::token_word_refs(&filter_tokens);
    let mut requires_different_powers = false;
    if grammar::words_match_suffix(&filter_tokens, &["with", "different", "powers"]).is_some()
        || grammar::words_match_suffix(&filter_tokens, &["with", "different", "power"]).is_some()
    {
        requires_different_powers = true;
        let cut_word_idx = filter_words.len().saturating_sub(3);
        let cut_token_idx =
            token_index_for_word_index(&filter_tokens, cut_word_idx).unwrap_or(filter_tokens.len());
        filter_tokens = trim_commas(&filter_tokens[..cut_token_idx]);
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let other = filter_tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"));
    let filter = parse_object_filter(&filter_tokens, other)?;
    if let Some(count) = min_count {
        if requires_different_powers {
            return Ok(Some(
                PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                    player: PlayerAst::You,
                    filter,
                    count,
                },
            ));
        }
        Ok(Some(PredicateAst::PlayerControlsAtLeast {
            player: PlayerAst::You,
            filter,
            count,
        }))
    } else {
        Ok(Some(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        }))
    }
}

pub(crate) fn parse_move(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    use super::super::grammar::primitives as grammar;
    use winnow::Parser as _;

    // "all counters from <source> onto/to <destination>"
    let Some(after_prefix) =
        grammar::strip_lexed_prefix_phrase(tokens, &["all", "counters", "from"])
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported move clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    };

    let split = grammar::split_lexed_once_on_separator(after_prefix, || grammar::kw("onto").void())
        .or_else(|| {
            grammar::split_lexed_once_on_separator(after_prefix, || grammar::kw("to").void())
        });
    let Some((from_tokens, to_tokens)) = split else {
        return Err(CardTextError::ParseError(format!(
            "missing move destination (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    };

    let from = parse_target_phrase(from_tokens)?;
    let to = parse_target_phrase(to_tokens)?;

    Ok(EffectAst::MoveAllCounters { from, to })
}

pub(crate) fn parse_draw(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let mut parsed_that_many_minus_one = false;
    let mut parsed_that_many_plus_one = false;
    let mut consumed_embedded_card_keyword = false;
    let (mut count, used) =
        if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, EVENT_AMOUNT_PREFIXES) {
            let mut value = Value::EventValue(EventValueSpec::Amount);
            let consumed = prefix.len();
            let rest = &tokens[consumed..];
            if rest
                .first()
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
            {
                let trailing = trim_commas(&rest[1..]);
                let trailing_words = crate::cards::builders::parser::token_word_refs(&trailing);
                if trailing_words.as_slice() == ["minus", "one"] {
                    value = Value::EventValueOffset(EventValueSpec::Amount, -1);
                    parsed_that_many_minus_one = true;
                } else if trailing_words.as_slice() == ["plus", "one"] {
                    value = Value::EventValueOffset(EventValueSpec::Amount, 1);
                    parsed_that_many_plus_one = true;
                } else if !trailing_words.is_empty()
                    && find_window_by(&trailing_words, 2, |window| {
                        window[0] == "for" && window[1] == "each"
                    })
                    .is_none()
                {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing draw clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
            }
            (value, consumed)
        } else if let Some((value, used_words)) =
            parse_half_rounded_down_draw_count_words(&clause_words)
        {
            consumed_embedded_card_keyword = true;
            (
                value,
                token_index_for_word_index(tokens, used_words).unwrap_or(tokens.len()),
            )
        } else if let Some(value) = parse_draw_as_many_cards_value(tokens) {
            consumed_embedded_card_keyword = true;
            (value, tokens.len())
        } else if tokens.first().is_some_and(|token| token.is_word("another"))
            && tokens
                .get(1)
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            (Value::Fixed(1), 1)
        } else if tokens
            .first()
            .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            let tail = trim_commas(&tokens[1..]);
            let value = parse_draw_card_prefixed_count_value(&tail)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing draw count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            consumed_embedded_card_keyword = true;
            (value, tokens.len())
        } else if tokens.first().is_some_and(|token| token.is_word("up"))
            && tokens.get(1).is_some_and(|token| token.is_word("to"))
        {
            let Some((amount, used_amount)) = parse_number(&tokens[2..]) else {
                return Err(CardTextError::ParseError(format!(
                    "missing draw count (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            (Value::Fixed(amount as i32), 2 + used_amount)
        } else {
            parse_value(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing draw count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
        };

    let rest = &tokens[used..];
    let tail = if consumed_embedded_card_keyword {
        trim_commas(rest)
    } else {
        let mut card_word_idx = 0usize;
        if rest
            .first()
            .is_some_and(|token| token.is_word("additional"))
        {
            card_word_idx = 1;
        }
        let Some(card_word) = rest.get(card_word_idx).and_then(OwnedLexToken::as_word) else {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        };
        if card_word != "card" && card_word != "cards" {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        }
        trim_commas(&rest[card_word_idx + 1..])
    };
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let mut effect = EffectAst::Draw {
        count: count.clone(),
        player,
    };

    if !tail.is_empty() {
        let tail_words = crate::cards::builders::parser::token_word_refs(&tail);
        if !((parsed_that_many_minus_one && tail_words.as_slice() == ["minus", "one"])
            || (parsed_that_many_plus_one && tail_words.as_slice() == ["plus", "one"]))
        {
            if let Some(parsed) = parse_draw_for_each_player_condition(&tail, effect.clone())? {
                effect = parsed;
            } else {
                let has_for_each = find_window_by(&tail, 2, |window: &[OwnedLexToken]| {
                    window[0].is_word("for") && window[1].is_word("each")
                })
                .is_some();
                if has_for_each {
                    let dynamic = parse_dynamic_cost_modifier_value(&tail)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported draw for-each clause (clause: '{}')",
                            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                        ))
                    })?;
                    match count {
                        Value::Fixed(1) => count = dynamic,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported multiplied draw count (clause: '{}')",
                                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                            )));
                        }
                    }
                    effect = EffectAst::Draw {
                        count: count.clone(),
                        player,
                    };
                } else if let Some(parsed) = parse_draw_trailing_clause(&tail, effect.clone())? {
                    effect = parsed;
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing draw clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
            }
        }
    }
    Ok(effect)
}

fn parse_draw_for_each_player_condition(
    tokens: &[OwnedLexToken],
    draw_effect: EffectAst,
) -> Result<Option<EffectAst>, CardTextError> {
    fn bind_loop_player_predicate(predicate: PredicateAst) -> PredicateAst {
        match predicate {
            PredicateAst::And(left, right) => PredicateAst::And(
                Box::new(bind_loop_player_predicate(*left)),
                Box::new(bind_loop_player_predicate(*right)),
            ),
            PredicateAst::Not(inner) => {
                PredicateAst::Not(Box::new(bind_loop_player_predicate(*inner)))
            }
            PredicateAst::PlayerControls { player, filter } if player == PlayerAst::That => {
                PredicateAst::PlayerControls {
                    player: PlayerAst::Implicit,
                    filter,
                }
            }
            PredicateAst::PlayerControlsAtLeast {
                player,
                filter,
                count,
            } if player == PlayerAst::That => PredicateAst::PlayerControlsAtLeast {
                player: PlayerAst::Implicit,
                filter,
                count,
            },
            PredicateAst::PlayerControlsExactly {
                player,
                filter,
                count,
            } if player == PlayerAst::That => PredicateAst::PlayerControlsExactly {
                player: PlayerAst::Implicit,
                filter,
                count,
            },
            PredicateAst::PlayerControlsMost { player, filter } if player == PlayerAst::That => {
                PredicateAst::PlayerControlsMost {
                    player: PlayerAst::Implicit,
                    filter,
                }
            }
            PredicateAst::PlayerControlsMoreThanYou { player, filter }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerControlsMoreThanYou {
                    player: PlayerAst::Implicit,
                    filter,
                }
            }
            PredicateAst::PlayerHasLessLifeThanYou { player } if player == PlayerAst::That => {
                PredicateAst::PlayerHasLessLifeThanYou {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasMoreLifeThanYou { player } if player == PlayerAst::That => {
                PredicateAst::PlayerHasMoreLifeThanYou {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasMoreCardsInHandThanYou { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHasMoreCardsInHandThanYou {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerTappedLandForManaThisTurn { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerTappedLandForManaThisTurn {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
                    player: PlayerAst::Implicit,
                }
            }
            other => other,
        }
    }

    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let (start, opponents_only) = if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, FOR_EACH_OPPONENT_WHO_PREFIXES)
    {
        (prefix.len() - 1, true)
    } else if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, FOR_EACH_PLAYER_WHO_PREFIXES)
    {
        (prefix.len() - 1, false)
    } else if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, EACH_OPPONENT_WHO_PREFIXES)
    {
        (prefix.len() - 1, true)
    } else if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, EACH_PLAYER_WHO_PREFIXES)
    {
        (prefix.len() - 1, false)
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&tokens[start..]);
    let inner_words = crate::cards::builders::parser::token_word_refs(&inner_tokens);
    if inner_words.first().copied() != Some("who") {
        return Ok(None);
    }

    let predicate_tail = trim_commas(&inner_tokens[1..]);
    if predicate_tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing predicate in draw for-each clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let predicate = bind_loop_player_predicate(
        parse_who_player_predicate_lexed(&inner_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing predicate in draw for-each clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?,
    );

    let effects = vec![EffectAst::Conditional {
        predicate,
        if_true: vec![draw_effect],
        if_false: Vec::new(),
    }];
    Ok(Some(if opponents_only {
        EffectAst::ForEachOpponent { effects }
    } else {
        EffectAst::ForEachPlayer { effects }
    }))
}

pub(crate) fn parse_half_rounded_down_draw_count_words(words: &[&str]) -> Option<(Value, usize)> {
    if words.first().copied() != Some("half") {
        return None;
    }

    let mut card_idx = None;
    for idx in 1..words.len() {
        if matches!(words.get(idx).copied(), Some("card" | "cards"))
            && words.get(idx + 1..idx + 3) == Some(&["rounded", "down"][..])
        {
            card_idx = Some(idx);
            break;
        }
    }
    let card_idx = card_idx?;

    let inner_words = &words[1..card_idx];
    let (inner, used_inner) = parse_value_expr_words(inner_words)?;
    if used_inner != inner_words.len() {
        return None;
    }

    Some((Value::HalfRoundedDown(Box::new(inner)), card_idx + 3))
}

pub(crate) fn parse_draw_trailing_clause(
    tokens: &[OwnedLexToken],
    draw_effect: EffectAst,
) -> Result<Option<EffectAst>, CardTextError> {
    let tail_words = crate::cards::builders::parser::token_word_refs(tokens);
    if tail_words.as_slice() == ["instead"] {
        return Ok(Some(draw_effect));
    }

    if let Some(timing) = parse_draw_delayed_timing_words(&tail_words) {
        return Ok(Some(wrap_return_with_delayed_timing(
            draw_effect,
            Some(timing),
        )));
    }

    if tail_words.first().copied() == Some("if") {
        let predicate = parse_trailing_if_predicate_lexed(tokens).ok_or_else(|| {
            CardTextError::ParseError("missing condition after trailing if clause".to_string())
        })?;
        return Ok(Some(EffectAst::Conditional {
            predicate,
            if_true: vec![draw_effect],
            if_false: Vec::new(),
        }));
    }

    if tail_words.first().copied() == Some("unless") {
        return try_build_unless(vec![draw_effect], tokens, 0);
    }

    Ok(None)
}

pub(crate) fn parse_draw_delayed_timing_words(words: &[&str]) -> Option<DelayedReturnTimingAst> {
    if let Some(timing) = parse_delayed_return_timing_words(words) {
        return Some(timing);
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "turns", "upkeep"]
            | ["at", "beginning", "of", "next", "turn's", "upkeep"]
            | ["at", "beginning", "of", "next", "turn’s", "upkeep"]
            | ["at", "beginning", "of", "the", "next", "turns", "upkeep"]
            | ["at", "beginning", "of", "the", "next", "turn's", "upkeep"]
            | ["at", "beginning", "of", "the", "next", "turn’s", "upkeep"]
            | ["at", "the", "beginning", "of", "next", "turns", "upkeep"]
            | ["at", "the", "beginning", "of", "next", "turn's", "upkeep"]
            | ["at", "the", "beginning", "of", "next", "turn’s", "upkeep"]
            | [
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "turns",
                "upkeep"
            ]
            | [
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "turn's",
                "upkeep"
            ]
            | [
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "turn’s",
                "upkeep"
            ]
    ) {
        return Some(DelayedReturnTimingAst::NextUpkeep(PlayerAst::Any));
    }

    None
}

pub(crate) fn parse_draw_as_many_cards_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let starts_as_many = clause_words.len() >= 4
        && clause_words[0] == "as"
        && clause_words[1] == "many"
        && matches!(clause_words[2], "card" | "cards")
        && clause_words[3] == "as";
    if !starts_as_many {
        return None;
    }

    let references_previous_event = grammar::words_find_phrase(tokens, &["this", "way"]).is_some();
    if references_previous_event {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }

    None
}

pub(crate) fn parse_draw_card_prefixed_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    if let Some(value) = parse_draw_equal_to_value(tokens)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
        return Ok(Some(value));
    }

    Ok(None)
}

pub(crate) fn parse_draw_equal_to_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if grammar::words_match_prefix(tokens, &["equal", "to"]).is_none() {
        return Ok(None);
    }

    if let Some(value) = parse_devotion_value_from_add_clause(tokens)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_add_mana_equal_amount_value(tokens)
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(tokens))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(tokens))
        .or_else(|| parse_equal_to_aggregate_filter_value(tokens))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(tokens))
        .or_else(|| parse_equal_to_number_of_filter_value(tokens))
    {
        return Ok(Some(value));
    }
    if grammar::words_find_phrase(tokens, &["this", "way"]).is_some() {
        return Ok(Some(Value::EventValue(EventValueSpec::Amount)));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
        return Ok(Some(value));
    }

    Ok(None)
}

pub(crate) fn parse_counter(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if let Some(spec) = split_trailing_if_clause_lexed(tokens) {
        let target = parse_counter_target_phrase(spec.leading_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate: spec.predicate,
            if_true: vec![EffectAst::Counter { target }],
            if_false: Vec::new(),
        });
    }

    if super::super::grammar::primitives::contains_word(tokens, "if") {
        return Err(CardTextError::ParseError(format!(
            "missing conditional counter target or predicate (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    if let Some((target_tokens, unless_tokens)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("unless").void()
        })
    {
        let target = parse_counter_target_phrase(target_tokens)?;
        let pays_idx = find_index(unless_tokens, |token: &OwnedLexToken| token.is_word("pays"))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing pays keyword (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                ))
            })?;

        // Parse the contiguous mana payment immediately following "pays".
        // Stop at the first non-mana word so trailing dynamic qualifiers
        // ("for each ...", "where X is ...", "plus an additional ...") do not
        // accidentally duplicate symbols.
        let mut mana = Vec::new();
        let mut trailing_start: Option<usize> = None;
        for (offset, token) in unless_tokens[pays_idx + 1..].iter().enumerate() {
            if let Some(group) = mana_pips_from_token(token) {
                mana.extend(group);
                continue;
            }
            if token.is_comma() || token.is_period() {
                continue;
            }
            let Some(word) = token.as_word() else {
                if !mana.is_empty() {
                    trailing_start = Some(pays_idx + 1 + offset);
                    break;
                }
                continue;
            };
            match parse_mana_symbol(word) {
                Ok(symbol) => mana.push(symbol),
                Err(_) => {
                    trailing_start = Some(pays_idx + 1 + offset);
                    break;
                }
            }
        }

        let mut life = None;
        let mut additional_generic = None;
        if mana.is_empty() {
            let payment_tokens = trim_commas(&unless_tokens[pays_idx + 1..]);
            let payment_words = crate::cards::builders::parser::token_word_refs(&payment_tokens);
            // "unless its controller pays mana equal to ..." uses a dynamic generic payment.
            if payment_words.first().copied() == Some("mana")
                && let Some(value) = parse_equal_to_aggregate_filter_value(&payment_tokens)
                    .or_else(|| parse_equal_to_number_of_filter_value(&payment_tokens))
            {
                additional_generic = Some(value);
                trailing_start = None;
            } else {
                return Err(CardTextError::ParseError(format!(
                    "missing mana cost (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                )));
            }
        }

        if let Some(trailing_idx) = trailing_start {
            let trailing_tokens = trim_commas(&unless_tokens[trailing_idx..]);
            let trailing_words = crate::cards::builders::parser::token_word_refs(&trailing_tokens);
            if trailing_tokens
                .first()
                .is_some_and(|token| token.is_word("and"))
            {
                let life_tokens = trim_commas(&trailing_tokens[1..]);
                if let Some((amount, used)) = parse_value(&life_tokens)
                    && life_tokens
                        .get(used)
                        .is_some_and(|token| token.is_word("life"))
                    && trim_commas(&life_tokens[used + 1..]).is_empty()
                {
                    life = Some(amount);
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        crate::cards::builders::parser::token_word_refs(tokens).join(" "),
                        trailing_words.join(" ")
                    )));
                }
            } else if let Some(value) =
                parse_counter_unless_additional_generic_value(&trailing_tokens)?
            {
                additional_generic = Some(value);
            } else if grammar::words_match_any_prefix(&trailing_tokens, FOR_EACH_PREFIXES).is_some()
            {
                if let Some(dynamic) = parse_dynamic_cost_modifier_value(&trailing_tokens)? {
                    if let [ManaSymbol::Generic(multiplier)] = mana.as_slice() {
                        additional_generic =
                            Some(scale_value_multiplier(dynamic, *multiplier as i32));
                        mana.clear();
                    } else {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                            crate::cards::builders::parser::token_word_refs(tokens).join(" "),
                            trailing_words.join(" ")
                        )));
                    }
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        crate::cards::builders::parser::token_word_refs(tokens).join(" "),
                        trailing_words.join(" ")
                    )));
                }
            } else if !trailing_words.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" "),
                    trailing_words.join(" ")
                )));
            }
        }

        if mana.is_empty() && life.is_none() && additional_generic.is_none() {
            return Err(CardTextError::ParseError(format!(
                "missing mana cost (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            )));
        }

        return Ok(EffectAst::CounterUnlessPays {
            target,
            mana,
            life,
            additional_generic,
        });
    }

    let target = parse_counter_target_phrase(tokens)?;
    Ok(EffectAst::Counter { target })
}

pub(crate) fn parse_counter_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if let Some(target) = parse_counter_ability_target_phrase(tokens)? {
        return Ok(target);
    }

    if grammar::contains_word(tokens, "ability")
        && (grammar::contains_word(tokens, "activated")
            || grammar::contains_word(tokens, "triggered"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-ability target clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    parse_target_phrase(tokens)
}

fn parse_counter_ability_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let clause_tokens = trim_commas(tokens);
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
    if !grammar::contains_word(&clause_tokens, "ability")
        || (!grammar::contains_word(&clause_tokens, "activated")
            && !grammar::contains_word(&clause_tokens, "triggered"))
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
                ObjectFilter::spell().with_type(CardType::Instant),
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
                ObjectFilter::spell().with_type(CardType::Sorcery),
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

    let mut source_types: Vec<CardType> = Vec::new();
    let mut controller_filter: Option<PlayerFilter> = None;
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
            controller_filter = Some(PlayerFilter::You);
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
            controller_filter = Some(PlayerFilter::NotYou);
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
            controller_filter = Some(PlayerFilter::NotYou);
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
                    .or_else(|| str_strip_suffix(type_word, "s").and_then(parse_card_type));
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

pub(crate) fn scale_value_multiplier(value: Value, multiplier: i32) -> Value {
    if multiplier <= 0 {
        return Value::Fixed(0);
    }
    if multiplier == 1 {
        return value;
    }
    match value {
        Value::Fixed(amount) => Value::Fixed(amount * multiplier),
        Value::Count(filter) => Value::CountScaled(filter, multiplier),
        Value::CountScaled(filter, factor) => Value::CountScaled(filter, factor * multiplier),
        other => {
            let mut result = Value::Fixed(0);
            for _ in 0..multiplier {
                result = match result {
                    Value::Fixed(0) => other.clone(),
                    _ => Value::Add(Box::new(result), Box::new(other.clone())),
                };
            }
            result
        }
    }
}

pub(crate) fn parse_counter_unless_additional_generic_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if tokens.is_empty() || !tokens[0].is_word("plus") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if tokens.get(idx).is_some_and(|token| token.is_word("an")) {
        idx += 1;
    }
    if !tokens
        .get(idx)
        .is_some_and(|token| token.is_word("additional"))
    {
        return Ok(None);
    }
    idx += 1;

    let multiplier = if let Some(token) = tokens.get(idx) {
        if let Some(group) = mana_pips_from_token(token) {
            match group.as_slice() {
                [ManaSymbol::Generic(amount)] => *amount as i32,
                _ => {
                    return Err(CardTextError::ParseError(
                        "unsupported nongeneric additional counter payment".to_string(),
                    ));
                }
            }
        } else {
            let symbol_word = token.as_word().ok_or_else(|| {
                CardTextError::ParseError("missing additional mana symbol".to_string())
            })?;
            let symbol = parse_mana_symbol(symbol_word).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported additional payment symbol '{}' in counter clause",
                    symbol_word
                ))
            })?;
            match symbol {
                ManaSymbol::Generic(amount) => amount as i32,
                _ => {
                    return Err(CardTextError::ParseError(
                        "unsupported nongeneric additional counter payment".to_string(),
                    ));
                }
            }
        }
    } else {
        return Err(CardTextError::ParseError(
            "missing additional mana symbol".to_string(),
        ));
    };

    let filter_tokens = trim_commas(&tokens[idx + 1..]);
    if grammar::words_match_prefix(&filter_tokens, &["for", "each"]).is_none() {
        return Err(CardTextError::ParseError(format!(
            "unsupported additional counter payment tail (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let dynamic = parse_dynamic_cost_modifier_value(&filter_tokens)?.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported additional counter payment filter (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        ))
    })?;
    Ok(Some(scale_value_multiplier(dynamic, multiplier)))
}

pub(crate) fn parse_reveal(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let words = crate::cards::builders::parser::token_word_refs(tokens);
    // Many effects split "reveal it/that card/those cards" into a standalone clause.
    // The engine does not model hidden information, so this compiles to a semantic no-op
    // that still allows parsing and auditing to proceed.
    if matches!(
        words.as_slice(),
        ["it"]
            | ["them"]
            | ["that"]
            | ["that", "card"]
            | ["those", "cards"]
            | ["those"]
            | ["this", "card"]
            | ["this"]
    ) {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_from_among = grammar::contains_word(tokens, "from")
        && grammar::contains_word(tokens, "among")
        && (grammar::contains_word(tokens, "them") || grammar::contains_word(tokens, "those"));
    if reveals_from_among {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_outside_game =
        grammar::contains_word(tokens, "outside") && grammar::contains_word(tokens, "game");
    if reveals_outside_game {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_first_draw =
        grammar::words_match_any_prefix(tokens, FIRST_CARD_YOU_DRAW_PREFIXES).is_some();
    if reveals_first_draw {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_card_this_way = (grammar::contains_word(tokens, "card")
        || grammar::contains_word(tokens, "cards"))
        && grammar::words_match_suffix(tokens, &["this", "way"]).is_some();
    if reveals_card_this_way {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_conditional_it =
        words.first() == Some(&"it") && grammar::contains_word(tokens, "if");
    if reveals_conditional_it {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    if grammar::contains_word(tokens, "hand") {
        let is_full_hand_reveal = matches!(words.as_slice(), ["your", "hand"] | ["their", "hand"])
            || words.as_slice() == ["his", "or", "her", "hand"];
        if !is_full_hand_reveal {
            if grammar::contains_word(tokens, "from") {
                return Ok(EffectAst::RevealTagged {
                    tag: TagKey::from(IT_TAG),
                });
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported reveal-hand clause (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(EffectAst::RevealHand { player });
    }

    let has_card =
        grammar::contains_word(tokens, "card") || grammar::contains_word(tokens, "cards");
    let has_library =
        grammar::contains_word(tokens, "library") || grammar::contains_word(tokens, "libraries");
    let explicit_top_card =
        words.as_slice() == ["top", "card"] || words.as_slice() == ["the", "top", "card"];

    if !has_card || (!has_library && !explicit_top_card) {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(EffectAst::RevealTop { player })
}

pub(crate) fn parse_life_amount(
    tokens: &[OwnedLexToken],
    amount_kind: &str,
) -> Result<(Value, usize), CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words == ["that", "much", "life"] {
        // "that much life" binds to the triggering event amount.
        return Ok((Value::EventValue(EventValueSpec::Amount), 2));
    }

    parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing {amount_kind} amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })
}

pub(crate) fn parse_life_equal_to_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["life", "equal", "to"]).is_none() {
        return Ok(None);
    }

    let amount_tokens = &tokens[1..];
    let amount_words = crate::cards::builders::parser::token_word_refs(amount_tokens);

    if let Some(value) = parse_add_mana_equal_amount_value(amount_tokens) {
        return Ok(Some(value));
    }
    if let Some(value) = parse_devotion_value_from_add_clause(amount_tokens)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_equal_to_number_of_filter_value(amount_tokens) {
        return Ok(Some(value));
    }
    if let Some(value) = parse_equal_to_aggregate_filter_value(amount_tokens) {
        return Ok(Some(value));
    }
    if matches!(
        amount_words.as_slice(),
        ["equal", "to", "the", "life", "lost", "this", "way"]
            | ["equal", "to", "life", "lost", "this", "way"]
            | [
                "equal", "to", "the", "amount", "of", "life", "lost", "this", "way"
            ]
            | ["equal", "to", "amount", "of", "life", "lost", "this", "way"]
    ) {
        return Ok(Some(Value::EventValue(EventValueSpec::LifeAmount)));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(amount_tokens)? {
        return Ok(Some(value));
    }

    Err(CardTextError::ParseError(format!(
        "missing life amount in equal-to clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_life_amount_from_trailing(
    base_amount: &Value,
    trailing: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if trailing.is_empty() {
        return Ok(None);
    }

    if let Some(dynamic) = parse_dynamic_cost_modifier_value(trailing)? {
        if let Some(multiplier) = match base_amount {
            Value::Fixed(value) => Some(*value),
            Value::X => Some(1),
            _ => None,
        } {
            return Ok(Some(scale_value_multiplier(dynamic, multiplier)));
        }
    }

    if let Some(where_value) = parse_where_x_value_clause(trailing) {
        if value_contains_unbound_x(base_amount) {
            let clause = crate::cards::builders::parser::token_word_refs(trailing).join(" ");
            return Ok(Some(replace_unbound_x_with_value(
                base_amount.clone(),
                &where_value,
                &clause,
            )?));
        }
        if matches!(base_amount, Value::Fixed(1)) {
            return Ok(Some(where_value));
        }
    }

    Ok(None)
}

pub(crate) fn validate_life_keyword(rest: &[OwnedLexToken]) -> Result<(), CardTextError> {
    if rest
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word != "life")
    {
        return Err(CardTextError::ParseError(
            "missing life keyword".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn remap_source_stat_value_to_it(value: Value) -> Value {
    match value {
        Value::PowerOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::ToughnessOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::ManaValueOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::Add(left, right) => Value::Add(
            Box::new(remap_source_stat_value_to_it(*left)),
            Box::new(remap_source_stat_value_to_it(*right)),
        ),
        other => other,
    }
}

fn player_filter_for_life_reference(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::ThatPlayerOrTargetController => None,
        PlayerAst::ItsController | PlayerAst::ItsOwner => None,
    }
}

fn parse_half_life_value(tokens: &[OwnedLexToken], player: PlayerAst) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.first().copied() != Some("half")
        || !grammar::contains_word(tokens, "life")
        || grammar::contains_word(tokens, "lost")
    {
        return None;
    }

    let player_filter = player_filter_for_life_reference(player)?;
    let rounded_down = grammar::words_find_phrase(tokens, &["rounded", "down"]).is_some();
    if rounded_down {
        Some(Value::HalfLifeTotalRoundedDown(player_filter))
    } else {
        Some(Value::HalfLifeTotalRoundedUp(player_filter))
    }
}

pub(crate) fn parse_lose_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.len() == 2
        && clause_words[1] == "life"
        && let Some((amount, _)) = parse_number(tokens)
    {
        return Ok(EffectAst::LoseLife {
            amount: Value::Fixed(amount as i32),
            player,
        });
    }
    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
            && (grammar::words_find_phrase(tokens, &["its", "power"]).is_some()
                || grammar::words_find_phrase(tokens, &["its", "toughness"]).is_some()
                || grammar::words_find_phrase(tokens, &["its", "mana", "value"]).is_some())
        {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(EffectAst::LoseLife { amount, player });
    }
    if clause_words.as_slice() == ["the", "game"] {
        return Ok(EffectAst::LoseGame { player });
    }

    if let Some(amount) = parse_half_life_value(tokens, player) {
        return Ok(EffectAst::LoseLife { amount, player });
    }

    let (mut amount, used) = parse_life_amount(tokens, "life loss")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(EffectAst::LoseLife { amount, player });
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-loss clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::LoseLife { amount, player })
}

pub(crate) fn parse_gain_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if let Some(amount) = parse_life_equal_to_value(tokens)? {
        return Ok(EffectAst::GainLife { amount, player });
    }

    let (mut amount, used) = parse_life_amount(tokens, "life gain")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if grammar::words_find_phrase(
            &trailing,
            &["then", "shuffle", "your", "graveyard", "into", "your"],
        )
        .is_some()
            && grammar::contains_word(&trailing, "library")
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing life-gain shuffle-graveyard clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            )));
        }
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(EffectAst::GainLife { amount, player });
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-gain clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::GainLife { amount, player })
}

pub(crate) fn parse_gain_control(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let has_dynamic_power_bound = grammar::contains_word(tokens, "power")
        && grammar::contains_word(tokens, "number")
        && grammar::words_find_phrase(tokens, &["you", "control"]).is_some();
    if has_dynamic_power_bound {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic power-bound control clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut idx = 0;
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("control"))
    {
        idx += 1;
    } else {
        return Err(CardTextError::ParseError(
            "missing control keyword".to_string(),
        ));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }

    let duration_idx = find_index(&tokens[idx..], |token: &OwnedLexToken| {
        token.is_word("during") || token.is_word("until")
    })
    .map(|offset| idx + offset)
    .or_else(|| {
        find_window_by(&tokens[idx..], 4, |window: &[OwnedLexToken]| {
            window[0].is_word("for")
                && window[1].is_word("as")
                && window[2].is_word("long")
                && window[3].is_word("as")
        })
        .map(|offset| idx + offset)
    });

    let target_tokens = if let Some(dur_idx) = duration_idx {
        &tokens[idx..dur_idx]
    } else {
        &tokens[idx..]
    };
    let invalid_conditional_error = || {
        CardTextError::ParseError(format!(
            "unsupported conditional gain-control clause (clause: '{}')",
            clause_words.join(" ")
        ))
    };
    let (target_ast, trailing_predicate, is_unless) =
        if let Some(spec) = split_trailing_if_clause_lexed(target_tokens) {
            (
                parse_target_phrase(spec.leading_tokens)?,
                Some(spec.predicate),
                false,
            )
        } else if target_tokens.iter().any(|token| token.is_word("if")) {
            return Err(invalid_conditional_error());
        } else if let Some(spec) = split_trailing_unless_clause_lexed(target_tokens) {
            (
                parse_target_phrase(spec.leading_tokens)?,
                Some(spec.predicate),
                true,
            )
        } else if target_tokens.iter().any(|token| token.is_word("unless")) {
            return Err(invalid_conditional_error());
        } else {
            (parse_target_phrase(target_tokens)?, None, false)
        };
    let duration_tokens = duration_idx
        .map(|dur_idx| &tokens[dur_idx..])
        .unwrap_or(&[]);
    let duration = parse_control_duration(duration_tokens)?;
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let base_effect = match target_ast {
        TargetAst::Player(filter, _) => EffectAst::ControlPlayer {
            player: PlayerFilter::Target(Box::new(filter)),
            duration,
        },
        _ => {
            let until = match duration {
                ControlDurationAst::UntilEndOfTurn => Until::EndOfTurn,
                ControlDurationAst::Forever => Until::Forever,
                ControlDurationAst::AsLongAsYouControlSource => Until::YouStopControllingThis,
                ControlDurationAst::DuringNextTurn => {
                    return Err(CardTextError::ParseError(
                        "unsupported control duration for permanents".to_string(),
                    ));
                }
            };
            EffectAst::GainControl {
                target: target_ast,
                player,
                duration: until,
            }
        }
    };

    if let Some(predicate) = trailing_predicate {
        return Ok(if is_unless {
            EffectAst::Conditional {
                predicate,
                if_true: Vec::new(),
                if_false: vec![base_effect],
            }
        } else {
            EffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            }
        });
    }

    Ok(base_effect)
}

pub(crate) fn parse_control_duration(
    tokens: &[OwnedLexToken],
) -> Result<ControlDurationAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(ControlDurationAst::Forever);
    }

    let has_for_as_long_as =
        grammar::words_find_phrase(tokens, &["for", "as", "long", "as"]).is_some();
    if has_for_as_long_as
        && grammar::contains_word(tokens, "you")
        && grammar::contains_word(tokens, "control")
        && (grammar::contains_word(tokens, "this")
            || grammar::contains_word(tokens, "thiss")
            || grammar::contains_word(tokens, "source")
            || grammar::contains_word(tokens, "creature")
            || grammar::contains_word(tokens, "permanent"))
    {
        return Ok(ControlDurationAst::AsLongAsYouControlSource);
    }

    let has_during = grammar::contains_word(tokens, "during");
    let has_next = grammar::contains_word(tokens, "next");
    let has_turn = grammar::contains_word(tokens, "turn");
    if has_during && has_next && has_turn {
        return Ok(ControlDurationAst::DuringNextTurn);
    }

    let has_until = grammar::contains_word(tokens, "until");
    let has_end = grammar::contains_word(tokens, "end");
    if has_until && has_end && has_turn {
        return Ok(ControlDurationAst::UntilEndOfTurn);
    }

    Err(CardTextError::ParseError(
        "unsupported control duration".to_string(),
    ))
}

pub(crate) fn parse_put_into_hand(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_put_into_hand_delayed_timing(
        tokens: &[OwnedLexToken],
    ) -> Option<DelayedReturnTimingAst> {
        let hand_idx = rfind_index(tokens, |token: &OwnedLexToken| {
            token.is_word("hand") || token.is_word("hands")
        })?;
        let tail_tokens = trim_commas(&tokens[hand_idx + 1..]);
        let tail_words = crate::cards::builders::parser::token_word_refs(&tail_tokens);
        parse_delayed_return_timing_words(&tail_words)
    }

    fn force_object_targeting(target: TargetAst, span: TextSpan) -> TargetAst {
        match target {
            TargetAst::Object(filter, explicit_span, fixed_span) => {
                TargetAst::Object(filter, explicit_span.or(Some(span)), fixed_span)
            }
            TargetAst::WithCount(inner, count) => {
                TargetAst::WithCount(Box::new(force_object_targeting(*inner, span)), count)
            }
            other => other,
        }
    }

    fn expand_graveyard_or_hand_disjunction(
        mut target: TargetAst,
        target_tokens: &[OwnedLexToken],
    ) -> TargetAst {
        let target_words = crate::cards::builders::parser::token_word_refs(target_tokens);
        let has_graveyard = target_words
            .iter()
            .any(|word| matches!(*word, "graveyard" | "graveyards"));
        let has_hand = target_words
            .iter()
            .any(|word| matches!(*word, "hand" | "hands"));
        if !(has_graveyard && has_hand) {
            return target;
        }

        fn apply(filter: &ObjectFilter) -> ObjectFilter {
            let mut graveyard = filter.clone();
            graveyard.any_of.clear();
            graveyard.zone = Some(Zone::Graveyard);

            let mut hand = filter.clone();
            hand.any_of.clear();
            hand.zone = Some(Zone::Hand);

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![graveyard, hand];
            disjunction
        }

        match &mut target {
            TargetAst::Object(filter, _, _) => {
                *filter = apply(filter);
            }
            TargetAst::WithCount(inner, _) => {
                if let TargetAst::Object(filter, _, _) = inner.as_mut() {
                    *filter = apply(filter);
                }
            }
            _ => {}
        }

        target
    }

    fn apply_source_zone_constraint(target: &mut TargetAst, zone: Zone) {
        match target {
            TargetAst::Source(span) => {
                *target = TargetAst::Object(ObjectFilter::source().in_zone(zone), *span, None);
            }
            TargetAst::Object(filter, _, _) => {
                filter.zone = Some(zone);
            }
            TargetAst::WithCount(inner, _) => apply_source_zone_constraint(inner, zone),
            _ => {}
        }
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);

    // "Put them/it back in any order." (typically after looking at the top cards of a library).
    if grammar::contains_word(tokens, "back")
        && grammar::contains_word(tokens, "any")
        && grammar::contains_word(tokens, "order")
        && matches!(clause_words.first().copied(), Some("it" | "them"))
    {
        return Ok(EffectAst::ReorderTopOfLibrary {
            tag: TagKey::from(IT_TAG),
        });
    }

    if grammar::contains_word(tokens, "from")
        && grammar::contains_word(tokens, "among")
        && grammar::contains_word(tokens, "hand")
    {
        return Ok(EffectAst::PutSomeIntoHandRestIntoGraveyard { player, count: 1 });
    }
    let has_it = grammar::contains_word(tokens, "it");
    let has_them = grammar::contains_word(tokens, "them");
    let has_hand = grammar::contains_word(tokens, "hand");
    let has_into = grammar::contains_word(tokens, "into");

    if has_hand && has_into && (has_it || has_them) {
        // "Put N of them into your hand and the rest on the bottom of your library in any order."
        if has_them
            && grammar::contains_word(tokens, "rest")
            && grammar::contains_word(tokens, "bottom")
            && grammar::contains_word(tokens, "library")
            && clause_words.iter().any(|w| *w == "and" || *w == "then")
        {
            let (count, used) = parse_number(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing put count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            let mut idx = used;
            if tokens.get(idx).is_some_and(|t| t.is_word("of")) {
                idx += 1;
            }
            if !tokens.get(idx).is_some_and(|t| t.is_word("them")) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported multi-destination put clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let dest_player = if grammar::contains_word(tokens, "your") {
                PlayerAst::You
            } else if grammar::contains_word(tokens, "their")
                || grammar::words_match_any_prefix(tokens, THAT_PLAYER_PREFIXES).is_some()
            {
                PlayerAst::That
            } else {
                player
            };

            return Ok(EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: dest_player,
                count: count as u32,
            });
        }

        // "Put N of them into your hand and the rest into your graveyard."
        if has_them
            && grammar::contains_word(tokens, "rest")
            && grammar::contains_word(tokens, "graveyard")
            && clause_words.iter().any(|w| *w == "and" || *w == "then")
        {
            let (count, used) = parse_number(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing put count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            // Accept optional "of" before "them".
            let mut idx = used;
            if tokens.get(idx).is_some_and(|t| t.is_word("of")) {
                idx += 1;
            }
            if !tokens.get(idx).is_some_and(|t| t.is_word("them")) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported multi-destination put clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            // The chooser is typically the player whose hand is referenced.
            let dest_player = if grammar::contains_word(tokens, "your") {
                PlayerAst::You
            } else if grammar::contains_word(tokens, "their")
                || grammar::words_match_any_prefix(tokens, THAT_PLAYER_PREFIXES).is_some()
            {
                PlayerAst::That
            } else {
                player
            };

            return Ok(EffectAst::PutSomeIntoHandRestIntoGraveyard {
                player: dest_player,
                count: count as u32,
            });
        }

        let effect = EffectAst::PutIntoHand {
            player,
            object: ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
        };
        return Ok(wrap_return_with_delayed_timing(
            effect,
            parse_put_into_hand_delayed_timing(tokens),
        ));
    }

    // Support destination-first wording:
    // "Put onto the battlefield under your control all creature cards ..."
    if tokens.first().is_some_and(|token| token.is_word("onto")) {
        let mut idx = 1usize;
        while tokens
            .get(idx)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(is_article)
        {
            idx += 1;
        }
        if !tokens
            .get(idx)
            .is_some_and(|token| token.is_word("battlefield"))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        idx += 1;

        let mut battlefield_tapped = false;
        if tokens.get(idx).is_some_and(|token| token.is_word("tapped")) {
            battlefield_tapped = true;
            idx += 1;
        }

        let mut battlefield_controller = ReturnControllerAst::Preserve;
        if tokens.get(idx).is_some_and(|token| token.is_word("under")) {
            let consumed =
                if grammar::words_match_prefix(&tokens[idx..], &["under", "your", "control"])
                    .is_some()
                {
                    battlefield_controller = ReturnControllerAst::You;
                    Some(3usize)
                } else if grammar::words_match_prefix(
                    &tokens[idx..],
                    &["under", "its", "owners", "control"],
                )
                .is_some()
                    || grammar::words_match_prefix(
                        &tokens[idx..],
                        &["under", "their", "owners", "control"],
                    )
                    .is_some()
                    || grammar::words_match_prefix(
                        &tokens[idx..],
                        &["under", "that", "players", "control"],
                    )
                    .is_some()
                {
                    battlefield_controller = ReturnControllerAst::Owner;
                    Some(4usize)
                } else {
                    None
                };
            if let Some(consumed) = consumed {
                idx += consumed;
            }
        }

        let target_tokens = trim_commas(&tokens[idx..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("attached"))
            && target_tokens
                .get(1)
                .is_some_and(|token| token.is_word("to"))
        {
            let after_to = &target_tokens[2..];
            if after_to.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attachment target after 'attached to' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let attachment_target_len = if after_to.first().is_some_and(|token| token.is_word("it"))
            {
                1usize
            } else if after_to.len() >= 2
                && after_to[0].is_word("that")
                && after_to[1].as_word().is_some_and(|word| {
                    matches!(
                        word,
                        "creature" | "permanent" | "object" | "aura" | "equipment"
                    )
                })
            {
                2usize
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported attachment target after 'attached to' (clause: '{}')",
                    clause_words.join(" ")
                )));
            };

            let attachment_target = parse_target_phrase(&after_to[..attachment_target_len])?;
            let object_tokens = trim_commas(&after_to[attachment_target_len..]);
            if object_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing object after attachment target (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let mut object_target = parse_target_phrase(&object_tokens)?;
            object_target = expand_graveyard_or_hand_disjunction(object_target, &object_tokens);
            object_target = force_object_targeting(object_target, tokens[0].span());

            return Ok(EffectAst::MoveToZone {
                target: object_target,
                zone: Zone::Battlefield,
                to_top: false,
                battlefield_controller,
                battlefield_tapped,
                attached_to: Some(attachment_target),
            });
        }

        if !target_tokens
            .first()
            .is_some_and(|token| token.is_word("attached"))
        {
            let mut rewritten = target_tokens;
            rewritten.push(OwnedLexToken::word("onto".to_string(), tokens[0].span()));
            rewritten.extend_from_slice(&tokens[1..idx]);
            return parse_put_into_hand(&rewritten, subject);
        }
    }

    if let Some((target_slice, after_on_top_of)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::phrase(&["on", "top", "of"]).void()
        })
    {
        let target_tokens = trim_commas(target_slice);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'on top of' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if !super::super::grammar::primitives::contains_word(after_on_top_of, "library") {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'on top of' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target = if let Some((count, used)) = parse_number(&target_tokens)
            && target_tokens
                .get(used)
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            let inner = parse_target_phrase(&target_tokens[used..])?;
            TargetAst::WithCount(Box::new(inner), ChoiceCount::exactly(count as usize))
        } else {
            parse_target_phrase(&target_tokens)?
        };
        return Ok(EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: true,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        });
    }

    if let Some(on_idx) = find_index(tokens, |token| token.is_word("on")) {
        let mut bottom_idx = on_idx + 1;
        if tokens
            .get(bottom_idx)
            .is_some_and(|token| token.is_word("the"))
        {
            bottom_idx += 1;
        }
        if tokens
            .get(bottom_idx)
            .is_some_and(|token| token.is_word("bottom"))
            && tokens
                .get(bottom_idx + 1)
                .is_some_and(|token| token.is_word("of"))
        {
            let target_tokens = trim_commas(&tokens[..on_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing target before 'on bottom of' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if !grammar::contains_word(&tokens[bottom_idx + 2..], "library") {
                return Err(CardTextError::ParseError(format!(
                    "unsupported put destination after 'on bottom of' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let target_words = crate::cards::builders::parser::token_word_refs(&target_tokens);
            let is_rest_target =
                target_words.as_slice() == ["the", "rest"] || target_words.as_slice() == ["rest"];
            if is_rest_target {
                return Ok(EffectAst::PutRestOnBottomOfLibrary);
            }

            let target = if let Some((count, used)) = parse_number(&target_tokens)
                && target_tokens
                    .get(used)
                    .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
            {
                let inner = parse_target_phrase(&target_tokens[used..])?;
                TargetAst::WithCount(Box::new(inner), ChoiceCount::exactly(count as usize))
            } else {
                parse_target_phrase(&target_tokens)?
            };

            return Ok(EffectAst::MoveToZone {
                target,
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            });
        }
    }

    if let Some((target_slice, destination_tokens)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("into").void()
        })
    {
        let target_tokens = trim_commas(target_slice);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'into' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let zone = if super::super::grammar::primitives::contains_word(destination_tokens, "hand")
            || super::super::grammar::primitives::contains_word(destination_tokens, "hands")
        {
            Some(Zone::Hand)
        } else if super::super::grammar::primitives::contains_word(destination_tokens, "graveyard")
            || super::super::grammar::primitives::contains_word(destination_tokens, "graveyards")
        {
            Some(Zone::Graveyard)
        } else if let Some(position) = parse_library_nth_from_top_destination(destination_tokens) {
            let target = parse_target_phrase(&target_tokens)?;
            return Ok(EffectAst::MoveToLibraryNthFromTop { target, position });
        } else {
            None
        };

        if let Some(zone) = zone {
            let delayed_hand_timing = if zone == Zone::Hand {
                parse_put_into_hand_delayed_timing(tokens)
            } else {
                None
            };
            let target_words = crate::cards::builders::parser::token_word_refs(&target_tokens);
            if zone == Zone::Graveyard
                && matches!(target_words.as_slice(), ["the", "rest"] | ["rest"])
            {
                return Ok(EffectAst::MoveToZone {
                    target: TargetAst::Object(
                        ObjectFilter::tagged(TagKey::from(IT_TAG)),
                        None,
                        None,
                    ),
                    zone,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                });
            }

            if zone == Zone::Hand {
                if matches!(
                    target_words.as_slice(),
                    ["it"] | ["them"] | ["that", "card"] | ["those", "card"] | ["those", "cards"]
                ) {
                    let effect = EffectAst::PutIntoHand {
                        player,
                        object: ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
                    };
                    return Ok(wrap_return_with_delayed_timing(effect, delayed_hand_timing));
                }
            }

            let effect = EffectAst::MoveToZone {
                target: parse_target_phrase(&target_tokens)?,
                zone,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            };
            return Ok(if zone == Zone::Hand {
                wrap_return_with_delayed_timing(effect, delayed_hand_timing)
            } else {
                effect
            });
        }
    }

    if let Some((target_slice, dest_slice)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("onto").void()
        })
    {
        let target_tokens = trim_commas(target_slice);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let destination_words: Vec<&str> =
            crate::cards::builders::parser::token_word_refs(dest_slice)
                .into_iter()
                .filter(|word| !is_article(word))
                .collect();
        if destination_words.first() != Some(&"battlefield") {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut destination_tail: Vec<&str> = destination_words[1..].to_vec();
        let battlefield_attacking = slice_contains(&destination_tail, &"attacking");
        let battlefield_tapped = slice_contains(&destination_tail, &"tapped");
        if let Some(from_idx) =
            find_word_sequence_start(&destination_tail, &["from", "command", "zone"])
        {
            destination_tail.drain(from_idx..from_idx + 3);
        }
        destination_tail.retain(|word| *word != "and");
        destination_tail.retain(|word| *word != "tapped");
        destination_tail.retain(|word| *word != "attacking");
        if battlefield_attacking {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let supported_control_tail = destination_tail.is_empty()
            || destination_tail.as_slice() == ["under", "your", "control"]
            || destination_tail.as_slice() == ["under", "its", "owners", "control"]
            || destination_tail.as_slice() == ["under", "their", "owners", "control"]
            || destination_tail.as_slice() == ["under", "that", "players", "control"];
        if !supported_control_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let battlefield_controller = if destination_tail.as_slice() == ["under", "your", "control"]
        {
            ReturnControllerAst::You
        } else if destination_tail.as_slice() == ["under", "its", "owners", "control"]
            || destination_tail.as_slice() == ["under", "their", "owners", "control"]
            || destination_tail.as_slice() == ["under", "that", "players", "control"]
        {
            ReturnControllerAst::Owner
        } else {
            ReturnControllerAst::Preserve
        };

        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("all") || token.is_word("each"))
        {
            let mut filter = parse_object_filter(&target_tokens[1..], false)?;
            if grammar::words_find_phrase(&target_tokens[1..], &["from", "it"]).is_some() {
                filter.zone = Some(Zone::Hand);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::You);
                }
                filter
                    .tagged_constraints
                    .retain(|constraint| constraint.tag.as_str() != IT_TAG);
            }
            if grammar::contains_word(tokens, "among") && grammar::contains_word(tokens, "them") {
                filter.zone = Some(Zone::Exile);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                if grammar::contains_word(tokens, "permanent") {
                    filter.card_types = vec![
                        CardType::Artifact,
                        CardType::Creature,
                        CardType::Enchantment,
                        CardType::Land,
                        CardType::Planeswalker,
                        CardType::Battle,
                    ];
                }
            }
            return Ok(EffectAst::ReturnAllToBattlefield {
                filter,
                tapped: battlefield_tapped,
            });
        }

        let mut target = parse_target_phrase(&target_tokens)?;
        if super::super::grammar::primitives::contains_phrase(
            dest_slice,
            &["from", "the", "command", "zone"],
        ) || super::super::grammar::primitives::contains_phrase(
            dest_slice,
            &["from", "command", "zone"],
        ) {
            apply_source_zone_constraint(&mut target, Zone::Command);
        }

        return Ok(EffectAst::MoveToZone {
            target,
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller,
            battlefield_tapped,
            attached_to: None,
        });
    }

    if grammar::contains_word(tokens, "sticker") {
        return Err(CardTextError::ParseError(format!(
            "unsupported sticker clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported put clause (clause: '{}')",
        clause_words.join(" ")
    )))
}
