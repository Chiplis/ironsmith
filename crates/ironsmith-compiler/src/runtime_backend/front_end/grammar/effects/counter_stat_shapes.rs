use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{literal, rest, take_till};

use super::super::super::lexer::{OwnedLexToken, TokenWordView};
use super::super::leaf;
use super::super::permission_shapes;
use crate::cards::builders::PlayerAst;
use crate::effect::Value;
use crate::target::PlayerFilter;
use crate::types::CardType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CounterWordSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevealReferenceShape {
    Tagged,
    FromAmongTagged,
    OutsideGame,
    FirstCardDrawn,
    CardThisWay,
    ConditionalIt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CounterReferenceShape<'a> {
    Source {
        counter_type_tokens: &'a [OwnedLexToken],
    },
    Tagged {
        counter_type_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LifeEqualSurface {
    LifeLostThisWay,
    DamagePreventedThisWay,
    AllPlayersLifeLostThisTurn,
    IteratedPlayerLifeLostThisTurn,
    TargetPlayerDamageThisTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetStatKind {
    Power,
    Toughness,
    ManaValue,
}

fn life_total_turn_subject<'a>(
    input: &mut super::super::primitives::WordSliceInput<'a>,
) -> WResult<PlayerFilter> {
    alt((
        alt((
            super::super::primitives::word_slice_exact("your"),
            super::super::primitives::word_slice_exact("you"),
        ))
        .value(PlayerFilter::You),
        alt((
            (
                super::super::primitives::word_slice_exact("that"),
                super::super::primitives::word_slice_exact("player"),
            )
                .void(),
            (
                super::super::primitives::word_slice_exact("that"),
                super::super::primitives::word_slice_exact("player's"),
            )
                .void(),
        ))
        .value(PlayerFilter::IteratedPlayer),
        alt((
            (
                super::super::primitives::word_slice_exact("target"),
                super::super::primitives::word_slice_exact("player"),
            )
                .void(),
            (
                super::super::primitives::word_slice_exact("target"),
                super::super::primitives::word_slice_exact("player's"),
            )
                .void(),
        ))
        .value(PlayerFilter::target_player()),
        alt((
            (
                super::super::primitives::word_slice_exact("target"),
                super::super::primitives::word_slice_exact("opponent"),
            )
                .void(),
            (
                super::super::primitives::word_slice_exact("target"),
                super::super::primitives::word_slice_exact("opponent's"),
            )
                .void(),
        ))
        .value(PlayerFilter::target_opponent()),
        alt((
            (
                super::super::primitives::word_slice_exact("opponent"),
                opt(super::super::primitives::word_slice_exact("'s")),
            )
                .void(),
            (super::super::primitives::word_slice_exact("opponent's"),).void(),
            (
                super::super::primitives::word_slice_exact("an"),
                alt((
                    super::super::primitives::word_slice_exact("opponent"),
                    super::super::primitives::word_slice_exact("opponent's"),
                )),
            )
                .void(),
            (
                super::super::primitives::word_slice_exact("each"),
                alt((
                    super::super::primitives::word_slice_exact("opponent"),
                    super::super::primitives::word_slice_exact("opponent's"),
                )),
            )
                .void(),
        ))
        .value(PlayerFilter::Opponent),
    ))
    .parse_next(input)
}

fn life_total_as_turn_began<'a>(
    input: &mut super::super::primitives::WordSliceInput<'a>,
) -> WResult<Value> {
    let player = life_total_turn_subject.parse_next(input)?;
    (
        super::super::primitives::word_slice_exact("life"),
        super::super::primitives::word_slice_exact("total"),
        super::super::primitives::word_slice_exact("as"),
        super::super::primitives::word_slice_exact("the"),
        super::super::primitives::word_slice_exact("turn"),
        super::super::primitives::word_slice_exact("began"),
    )
        .parse_next(input)?;
    Ok(Value::LifeTotalAsTurnBegan(player))
}

pub(crate) fn parse_life_total_as_turn_began_words(words: &[&str]) -> Option<Value> {
    super::super::primitives::parse_full_word_slice(words, life_total_as_turn_began)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PossessiveTargetStatShape {
    pub(crate) target_tokens: Vec<OwnedLexToken>,
    pub(crate) stat: TargetStatKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CounterAdditionalPaymentShape<'a> {
    pub(crate) multiplier_token: &'a OwnedLexToken,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HalfLifeShape {
    pub(crate) rounded_down: bool,
}

pub(crate) fn parse_prefix_at(
    words: &[&str],
    start: usize,
    alternatives: &[&[&str]],
) -> Option<CounterWordSpan> {
    let tail = words.get(start..)?;
    alternatives.iter().find_map(|expected| {
        permission_shapes::prefix_words(tail, expected).then_some(CounterWordSpan {
            start,
            end: start + expected.len(),
        })
    })
}

pub(crate) fn parse_prefix(words: &[&str], alternatives: &[&[&str]]) -> Option<CounterWordSpan> {
    parse_prefix_at(words, 0, alternatives)
}

pub(crate) fn parse_token_prefix_at(
    tokens: &[OwnedLexToken],
    start: usize,
    expected: &[&str],
) -> Option<CounterWordSpan> {
    permission_shapes::prefix_tokens(tokens.get(start..)?, expected).then_some(CounterWordSpan {
        start,
        end: start + expected.len(),
    })
}

pub(crate) fn parse_word(word: &str, choices: &[&str]) -> Option<CounterWordSpan> {
    choices
        .contains(&word)
        .then_some(CounterWordSpan { start: 0, end: 1 })
}

pub(crate) fn parse_exact(words: &[&str], alternatives: &[&[&str]]) -> Option<CounterWordSpan> {
    alternatives.iter().find_map(|expected| {
        permission_shapes::exact_words(words, expected).then_some(CounterWordSpan {
            start: 0,
            end: words.len(),
        })
    })
}

pub(crate) fn find_phrase(words: &[&str], alternatives: &[&[&str]]) -> Option<CounterWordSpan> {
    alternatives.iter().find_map(|expected| {
        permission_shapes::find_words(words, expected).map(|start| CounterWordSpan {
            start,
            end: start + expected.len(),
        })
    })
}

pub(crate) fn find_word(words: &[&str], choices: &[&str]) -> Option<usize> {
    choices
        .iter()
        .filter_map(|expected| permission_shapes::find_words(words, &[*expected]))
        .min()
}

pub(crate) fn find_all_words(words: &[&str], required: &[&str]) -> Option<Vec<usize>> {
    required
        .iter()
        .map(|expected| permission_shapes::find_words(words, &[*expected]))
        .collect()
}

pub(crate) fn word_token_boundary(tokens: &[OwnedLexToken], word: usize) -> Option<usize> {
    TokenWordView::new(tokens)
        .token_start_indices()
        .get(word)
        .copied()
}

pub(crate) fn parse_source_card_type(word: &str) -> Option<CardType> {
    if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
        return Some(card_type);
    }
    parse_plural_card_type.parse(word).ok()
}

fn parse_plural_card_type(input: &mut &str) -> WResult<CardType> {
    alt((
        literal("lands").value(CardType::Land),
        literal("creatures").value(CardType::Creature),
        literal("artifacts").value(CardType::Artifact),
        literal("enchantments").value(CardType::Enchantment),
        literal("planeswalkers").value(CardType::Planeswalker),
        literal("instants").value(CardType::Instant),
        literal("sorceries").value(CardType::Sorcery),
        literal("battles").value(CardType::Battle),
        literal("kindreds").value(CardType::Kindred),
    ))
    .parse_next(input)
}

pub(crate) fn parse_reveal_reference(tokens: &[OwnedLexToken]) -> Option<RevealReferenceShape> {
    let words = TokenWordView::new(tokens).word_refs();
    const TAGGED: &[&[&str]] = &[
        &["it"],
        &["them"],
        &["that"],
        &["that", "card"],
        &["those", "cards"],
        &["those"],
        &["this", "card"],
        &["this"],
    ];
    if parse_exact(&words, TAGGED).is_some() {
        return Some(RevealReferenceShape::Tagged);
    }
    if find_all_words(&words, &["from", "among"]).is_some()
        && find_word(&words, &["them", "those"]).is_some()
    {
        return Some(RevealReferenceShape::FromAmongTagged);
    }
    if find_phrase(&words, &[&["outside", "game"]]).is_some() {
        return Some(RevealReferenceShape::OutsideGame);
    }
    if parse_prefix(&words, &[&["the", "first", "card", "you", "draw"]]).is_some() {
        return Some(RevealReferenceShape::FirstCardDrawn);
    }
    if let Some(card) = find_word(&words, &["card", "cards"])
        && let Some(this_way) = find_phrase(&words[card + 1..], &[&["this", "way"]])
        && this_way.start <= words.len()
    {
        return Some(RevealReferenceShape::CardThisWay);
    }
    if parse_prefix(&words, &[&["it"]]).is_some()
        && find_word(words_after_first(&words), &["if"]).is_some()
    {
        return Some(RevealReferenceShape::ConditionalIt);
    }
    None
}

pub(crate) fn parse_reveal_full_hand(tokens: &[OwnedLexToken]) -> Option<CounterWordSpan> {
    let words = TokenWordView::new(tokens).word_refs();
    parse_exact(
        &words,
        &[
            &["your", "hand"],
            &["their", "hand"],
            &["his", "or", "her", "hand"],
        ],
    )
}

pub(crate) fn find_reveal_hand_source(tokens: &[OwnedLexToken]) -> Option<CounterWordSpan> {
    let words = TokenWordView::new(tokens).word_refs();
    find_phrase(
        &words,
        &[
            &["from", "your", "hand"],
            &["from", "their", "hand"],
            &["from", "his", "or", "her", "hand"],
        ],
    )
}

pub(crate) fn find_from_preposition(tokens: &[OwnedLexToken]) -> Option<CounterWordSpan> {
    let words = TokenWordView::new(tokens).word_refs();
    find_word(&words, &["from"]).map(|start| CounterWordSpan {
        start,
        end: start + 1,
    })
}

pub(crate) fn parse_explicit_top_card(tokens: &[OwnedLexToken]) -> Option<CounterWordSpan> {
    let words = TokenWordView::new(tokens).word_refs();
    parse_exact(&words, &[&["top", "card"], &["the", "top", "card"]])
}

pub(crate) fn parse_top_library_source(tokens: &[OwnedLexToken]) -> Option<CounterWordSpan> {
    let words = TokenWordView::new(tokens).word_refs();
    const PREFIXES: &[&[&str]] = &[
        &["the", "top", "card", "of", "your", "library"],
        &["the", "top", "card", "of", "your", "libraries"],
        &["the", "top", "card", "of", "their", "library"],
        &["the", "top", "card", "of", "their", "libraries"],
        &["top", "card", "of", "your", "library"],
        &["top", "card", "of", "your", "libraries"],
        &["top", "card", "of", "their", "library"],
        &["top", "card", "of", "their", "libraries"],
        &["the", "top", "cards", "of", "your", "library"],
        &["the", "top", "cards", "of", "your", "libraries"],
        &["the", "top", "cards", "of", "their", "library"],
        &["the", "top", "cards", "of", "their", "libraries"],
        &["top", "cards", "of", "your", "library"],
        &["top", "cards", "of", "your", "libraries"],
        &["top", "cards", "of", "their", "library"],
        &["top", "cards", "of", "their", "libraries"],
    ];
    parse_prefix(&words, PREFIXES)
}

pub(crate) fn parse_top_library_owner(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let words = TokenWordView::new(tokens).word_refs();
    if find_phrase(
        &words,
        &[
            &["of", "target", "opponent's", "library"],
            &["of", "target", "opponents'", "library"],
            &["of", "target", "opponent", "library"],
            &["of", "target", "opponents", "library"],
        ],
    )
    .is_some()
    {
        return Some(PlayerAst::TargetOpponent);
    }
    if find_phrase(
        &words,
        &[
            &["of", "target", "player's", "library"],
            &["of", "target", "players'", "library"],
            &["of", "target", "player", "library"],
            &["of", "target", "players", "library"],
        ],
    )
    .is_some()
    {
        return Some(PlayerAst::Target);
    }
    if find_phrase(
        &words,
        &[
            &["of", "that", "player's", "library"],
            &["of", "that", "players", "library"],
            &["of", "their", "library"],
        ],
    )
    .is_some()
    {
        return Some(PlayerAst::That);
    }
    find_phrase(&words, &[&["of", "your", "library"]]).map(|_| PlayerAst::You)
}

pub(crate) fn parse_library_tail(tokens: &[OwnedLexToken]) -> Option<CounterWordSpan> {
    let words = TokenWordView::new(tokens).word_refs();
    parse_prefix(
        &words,
        &[
            &["card", "of", "your", "library"],
            &["card", "of", "your", "libraries"],
            &["card", "of", "their", "library"],
            &["card", "of", "their", "libraries"],
            &["cards", "of", "your", "library"],
            &["cards", "of", "your", "libraries"],
            &["cards", "of", "their", "library"],
            &["cards", "of", "their", "libraries"],
        ],
    )
}

pub(crate) fn parse_additional_payment_head(
    tokens: &[OwnedLexToken],
) -> Option<CounterAdditionalPaymentShape<'_>> {
    let words = TokenWordView::new(tokens).word_refs();
    let shape = if parse_prefix(&words, &[&["plus", "an", "additional"]]).is_some() {
        3
    } else if parse_prefix(&words, &[&["plus", "additional"]]).is_some() {
        2
    } else {
        return None;
    };
    let view = TokenWordView::new(tokens);
    let multiplier_index = *view.token_start_indices().get(shape)?;
    let filter_start = view.token_index_after_words(shape + 1)?;
    Some(CounterAdditionalPaymentShape {
        multiplier_token: tokens.get(multiplier_index)?,
        filter_tokens: tokens.get(filter_start..)?,
    })
}

pub(crate) fn parse_prior_effect_count_source(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::EffectMetricSource> {
    let words = TokenWordView::new(tokens).word_refs();
    let prefix = parse_prefix(&words, &[&["where", "x", "is"]])?;
    let mut index = prefix.end;
    if words.get(index) == Some(&"the") {
        index += 1;
    }
    let number = parse_prefix_at(&words, index, &[&["number", "of"]])?;
    let objects = words.get(number.end..)?;
    if find_phrase(objects, &[&["this", "way"]]).is_none()
        && find_word(
            objects,
            &[
                "chosen",
                "destroyed",
                "discarded",
                "exiled",
                "milled",
                "revealed",
                "sacrificed",
                "searched",
            ],
        )
        .is_none()
    {
        return None;
    }
    Some(if find_word(objects, &["chosen"]).is_some() {
        ironsmith_core::EffectMetricSource::ChosenObjects
    } else {
        ironsmith_core::EffectMetricSource::AffectedObjects
    })
}

pub(crate) fn parse_life_equal_surface(words: &[&str]) -> Option<LifeEqualSurface> {
    const LIFE_LOST_THIS_WAY: &[&[&str]] = &[
        &["equal", "to", "the", "life", "lost", "this", "way"],
        &["equal", "to", "life", "lost", "this", "way"],
        &[
            "equal", "to", "the", "amount", "of", "life", "lost", "this", "way",
        ],
        &["equal", "to", "amount", "of", "life", "lost", "this", "way"],
    ];
    const DAMAGE_PREVENTED: &[&[&str]] = &[
        &["equal", "to", "the", "damage", "prevented", "this", "way"],
        &["equal", "to", "damage", "prevented", "this", "way"],
        &[
            "equal",
            "to",
            "the",
            "amount",
            "of",
            "damage",
            "prevented",
            "this",
            "way",
        ],
        &[
            "equal",
            "to",
            "amount",
            "of",
            "damage",
            "prevented",
            "this",
            "way",
        ],
    ];
    const ALL_PLAYERS: &[&[&str]] = &[
        &[
            "equal", "to", "the", "total", "life", "lost", "by", "all", "players", "this", "turn",
        ],
        &[
            "equal", "to", "total", "life", "lost", "by", "all", "players", "this", "turn",
        ],
        &[
            "equal", "to", "the", "total", "amount", "of", "life", "lost", "by", "all", "players",
            "this", "turn",
        ],
        &[
            "equal", "to", "total", "amount", "of", "life", "lost", "by", "all", "players", "this",
            "turn",
        ],
    ];
    const ITERATED: &[&[&str]] = &[
        &[
            "equal", "to", "the", "life", "that", "player", "lost", "this", "turn",
        ],
        &[
            "equal", "to", "life", "that", "player", "lost", "this", "turn",
        ],
        &[
            "equal", "to", "the", "amount", "of", "life", "that", "player", "lost", "this", "turn",
        ],
        &[
            "equal", "to", "amount", "of", "life", "that", "player", "lost", "this", "turn",
        ],
    ];
    const DAMAGE_TO_PLAYER: &[&[&str]] = &[
        &[
            "equal", "to", "the", "damage", "already", "dealt", "to", "that", "player", "this",
            "turn",
        ],
        &[
            "equal", "to", "damage", "already", "dealt", "to", "that", "player", "this", "turn",
        ],
        &[
            "equal", "to", "the", "amount", "of", "damage", "already", "dealt", "to", "that",
            "player", "this", "turn",
        ],
        &[
            "equal", "to", "amount", "of", "damage", "already", "dealt", "to", "that", "player",
            "this", "turn",
        ],
    ];
    if parse_exact(words, LIFE_LOST_THIS_WAY).is_some() {
        Some(LifeEqualSurface::LifeLostThisWay)
    } else if parse_exact(words, DAMAGE_PREVENTED).is_some() {
        Some(LifeEqualSurface::DamagePreventedThisWay)
    } else if parse_exact(words, ALL_PLAYERS).is_some() {
        Some(LifeEqualSurface::AllPlayersLifeLostThisTurn)
    } else if parse_exact(words, ITERATED).is_some() {
        Some(LifeEqualSurface::IteratedPlayerLifeLostThisTurn)
    } else if parse_exact(words, DAMAGE_TO_PLAYER).is_some() {
        Some(LifeEqualSurface::TargetPlayerDamageThisTurn)
    } else {
        None
    }
}

pub(crate) fn parse_possessive_target_stat(
    tokens: &[OwnedLexToken],
) -> Option<PossessiveTargetStatShape> {
    let words = TokenWordView::new(tokens).word_refs();
    let (stat_words, stat) = if permission_shapes::suffix_words(&words, &["mana", "value"]) {
        (2, TargetStatKind::ManaValue)
    } else if permission_shapes::suffix_words(&words, &["toughness"]) {
        (1, TargetStatKind::Toughness)
    } else if permission_shapes::suffix_words(&words, &["power"]) {
        (1, TargetStatKind::Power)
    } else {
        return None;
    };
    let target_word_count = words.len().checked_sub(stat_words)?;
    let target_end = TokenWordView::new(tokens).token_index_after_words(target_word_count)?;
    let mut target_tokens = tokens.get(..target_end)?.to_vec();
    let possessive = target_tokens.last_mut()?;
    let stem = parse_possessive_stem.parse(possessive.as_word()?).ok()?;
    if !possessive.replace_word(stem) {
        return None;
    }
    Some(PossessiveTargetStatShape {
        target_tokens,
        stat,
    })
}

pub(crate) fn parse_counter_reference(
    tokens: &[OwnedLexToken],
) -> Option<CounterReferenceShape<'_>> {
    let words = TokenWordView::new(tokens).word_refs();
    let prefix = parse_prefix(&words, &[&["for", "each"]])?;
    let counter = find_word(&words[prefix.end..], &["counter", "counters"])? + prefix.end;
    if counter <= prefix.end || words.get(counter + 1) != Some(&"on") {
        return None;
    }
    let reference = words.get(counter + 2..)?;
    const SOURCE: &[&[&str]] = &[
        &["it"],
        &["this"],
        &["this", "artifact"],
        &["this", "aura"],
        &["this", "battle"],
        &["this", "card"],
        &["this", "creature"],
        &["this", "enchantment"],
        &["this", "land"],
        &["this", "permanent"],
        &["this", "planeswalker"],
        &["this", "source"],
    ];
    const TAGGED: &[&[&str]] = &[
        &["that"],
        &["that", "creature"],
        &["that", "permanent"],
        &["that", "object"],
        &["those"],
        &["those", "creatures"],
        &["those", "permanents"],
    ];
    let type_start = TokenWordView::new(tokens).token_index_after_words(prefix.end)?;
    let type_end = TokenWordView::new(tokens).token_index_after_words(counter + 1)?;
    let counter_type_tokens = tokens.get(type_start..type_end)?;
    if parse_exact(reference, SOURCE).is_some() {
        Some(CounterReferenceShape::Source {
            counter_type_tokens,
        })
    } else if parse_exact(reference, TAGGED).is_some() {
        Some(CounterReferenceShape::Tagged {
            counter_type_tokens,
        })
    } else {
        None
    }
}

pub(crate) fn parse_half_life(words: &[&str]) -> Option<HalfLifeShape> {
    if parse_prefix(words, &[&["half"]]).is_none()
        || find_word(words, &["life"]).is_none()
        || find_word(words, &["lost"]).is_some()
    {
        return None;
    }
    Some(HalfLifeShape {
        rounded_down: find_phrase(words, &[&["rounded", "down"]]).is_some(),
    })
}

fn words_after_first<'a>(words: &'a [&'a str]) -> &'a [&'a str] {
    words.get(1..).unwrap_or_default()
}

fn parse_possessive_stem(input: &mut &str) -> WResult<String> {
    let stem: &str = take_till(1.., |character: char| matches!(character, '\'' | '’' | '‘'))
        .parse_next(input)?;
    alt((literal("'s"), literal("’s"), literal("‘s"))).parse_next(input)?;
    eof.parse_next(input)?;
    let mut output = String::new();
    output.push_str(stem);
    let _: &str = rest.parse_next(input)?;
    Ok(output)
}

#[cfg(test)]
#[path = "counter_stat_shapes/tests.rs"]
mod tests;
