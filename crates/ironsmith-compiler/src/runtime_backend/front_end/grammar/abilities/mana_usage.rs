use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

use crate::ability::{ManaUsageRestriction, ManaUsageSubtypeRequirement};
use crate::object::CounterType;
use crate::static_abilities::StaticAbilityId;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::filters::parse_spell_filter_with_grammar_entrypoint;
use super::super::{leaf, primitives};
use super::surface::{
    matches_any_exact_tokens, matches_any_prefix_tokens, matches_exact_tokens, parse_phrase_words,
    phrase_offset_words, take_word,
};
use crate::runtime_backend::util::{parse_counter_type_from_tokens, parse_number};

const SPEND_MANA_RESTRICTION_PREFIXES: &[&[&str]] = &[
    &["spend", "only", "mana"],
    &["spend", "this", "mana", "only"],
    &["spend", "that", "mana", "only"],
    &["this", "mana", "cant", "be", "spent", "to", "cast"],
    &["this", "mana", "can't", "be", "spent", "to", "cast"],
    &["that", "mana", "cant", "be", "spent", "to", "cast"],
    &["that", "mana", "can't", "be", "spent", "to", "cast"],
];
const SPEND_MANA_CAST_PREFIXES: &[&[&str]] = &[
    &["spend", "this", "mana", "only", "to", "cast"],
    &["spend", "that", "mana", "only", "to", "cast"],
];
const IF_MANA_SPENT_SPELL_PREFIXES: &[&[&str]] = &[
    &["if", "this", "mana", "is", "spent", "to", "cast"],
    &["if", "that", "mana", "is", "spent", "to", "cast"],
    &["if", "this", "mana", "is", "spent", "on"],
    &["if", "that", "mana", "is", "spent", "on"],
];
const UNCOUNTERABLE_TAILS: &[&[&str]] = &[
    &["and", "that", "spell", "can't", "be", "countered"],
    &["and", "that", "spell", "cant", "be", "countered"],
];
const UNSUPPORTED_SPEC_WORDS: &[&str] = &[
    "activate",
    "activates",
    "activated",
    "activation",
    "ability",
    "abilities",
    "pay",
    "foretell",
    "unlock",
    "turn",
    "cost",
    "costs",
];
const PLAIN_SPELL_WORDS: &[&str] = &["a", "an", "the", "spell", "spells"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LegacyCastShape<'a> {
    card_type: CardType,
    tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FilterCastShape<'a> {
    spec_tokens: &'a [OwnedLexToken],
    grant_uncounterable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManaSpendBonusShape<'a> {
    spec_tokens: &'a [OwnedLexToken],
    bonus_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManaSpendCounterShape<'a> {
    counter_tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManaUsageSpecShape {
    Unsupported,
    PlainSpell,
    Other,
}

pub(crate) fn is_spend_mana_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_prefix_tokens(tokens, SPEND_MANA_RESTRICTION_PREFIXES)
}

pub(crate) fn parse_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    parse_cast_unlock_turn_face_up(tokens)
        .or_else(|| parse_cast_or_activate_source(tokens))
        .or_else(|| parse_legacy_restriction(tokens))
        .or_else(|| parse_activate_ability_restriction(tokens))
        .or_else(|| parse_cant_be_spent_restriction(tokens))
        .or_else(|| parse_filter_restriction(tokens))
}

pub(crate) fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let parsed = parse_mana_spend_bonus_shape(tokens)?;
    let spec_tokens = trim_lexed_commas(parsed.spec_tokens);
    if TokenWordView::new(spec_tokens).is_empty() {
        return None;
    }
    let simple_card_type = parse_simple_bonus_card_type(spec_tokens);
    let bonus_tokens = trim_lexed_commas(parsed.bonus_tokens);
    if TokenWordView::new(bonus_tokens).is_empty() {
        return None;
    }

    let grant_uncounterable = matches_any_exact_tokens(
        bonus_tokens,
        &[
            &["that", "spell", "can't", "be", "countered"],
            &["that", "spell", "cant", "be", "countered"],
        ],
    );
    let granted_abilities = if matches_any_exact_tokens(
        bonus_tokens,
        &[
            &["it", "gains", "haste"],
            &["that", "spell", "gains", "haste"],
            &["that", "creature", "gains", "haste"],
            &["it", "gains", "haste", "until", "end", "of", "turn"],
            &[
                "that", "spell", "gains", "haste", "until", "end", "of", "turn",
            ],
            &[
                "that", "creature", "gains", "haste", "until", "end", "of", "turn",
            ],
        ],
    ) {
        vec![StaticAbilityId::Haste]
    } else {
        Vec::new()
    };

    if grant_uncounterable || !granted_abilities.is_empty() {
        if let Some(card_type) = simple_card_type {
            return Some(ManaUsageRestriction::CastSpell {
                card_types: vec![card_type],
                subtype_requirement: None,
                restrict_to_matching_spell: false,
                grant_uncounterable,
                enters_with_counters: vec![],
                granted_abilities,
            });
        }
        let filter = parse_nondefault_spell_filter(spec_tokens)?;
        return Some(ManaUsageRestriction::CastSpellMatching {
            filter,
            restrict_to_matching_spell: false,
            grant_uncounterable,
            enters_with_counters: vec![],
            granted_abilities,
        });
    }

    let card_type = simple_card_type?;
    let counter = parse_mana_spend_counter_shape(bonus_tokens)?;
    let (counter_type, count) = parse_mana_spend_counter_tail(counter)?;
    Some(ManaUsageRestriction::CastSpell {
        card_types: vec![card_type],
        subtype_requirement: None,
        restrict_to_matching_spell: false,
        grant_uncounterable: false,
        enters_with_counters: vec![(counter_type, count)],
        granted_abilities: vec![],
    })
}

pub(crate) fn is_mana_spend_bonus_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_prefix_tokens(tokens, IF_MANA_SPENT_SPELL_PREFIXES)
}

fn parse_cast_unlock_turn_face_up(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let prefix_end = parse_any_prefix_word_count(&words, SPEND_MANA_CAST_PREFIXES)?;
    let unlock =
        prefix_end + phrase_offset_words(words.get(prefix_end..)?, &["unlock", "a", "door"])?;
    let mut tail: primitives::WordSliceInput<'_> = words.get(unlock..)?;
    parse_phrase_words(&mut tail, &["unlock", "a", "door"]).ok()?;
    alt((
        |input: &mut primitives::WordSliceInput<'_>| {
            parse_phrase_words(input, &["or", "turn", "a", "permanent", "face", "up"])
        },
        |input: &mut primitives::WordSliceInput<'_>| {
            parse_phrase_words(input, &["or", "turn", "permanents", "face", "up"])
        },
    ))
    .parse_next(&mut tail)
    .ok()?;
    primitives::word_slice_eof(&mut tail).ok()?;
    if unlock == prefix_end {
        return None;
    }
    let spell_tokens = token_slice_for_words(tokens, &view, prefix_end, unlock)?;
    let spell_filter = parse_mana_usage_spell_filter(spell_tokens)?;
    Some(ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp { spell_filter })
}

fn parse_cast_or_activate_source(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    const SEPARATORS: &[&[&str]] = &[
        &["or", "activate", "an", "ability", "of"],
        &["or", "activate", "abilities", "of"],
    ];
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let prefix_end = parse_any_prefix_word_count(&words, SPEND_MANA_CAST_PREFIXES)?;
    let (separator, separator_start) = first_phrase_choice(words.get(prefix_end..)?, SEPARATORS)?;
    let separator_start = prefix_end + separator_start;
    let source_start = separator_start + separator.len();
    if separator_start == prefix_end || source_start >= words.len() {
        return None;
    }
    let spell_tokens = token_slice_for_words(tokens, &view, prefix_end, separator_start)?;
    let source_tokens = token_slice_for_words(tokens, &view, source_start, words.len())?;
    Some(
        ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
            spell_filter: parse_mana_usage_spell_filter(spell_tokens)?,
            ability_source_filter: parse_ability_source_filter(source_tokens)?,
        },
    )
}

fn parse_cant_be_spent_restriction(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    const PREFIXES: &[&[&str]] = &[
        &["this", "mana", "cant", "be", "spent", "to", "cast"],
        &["this", "mana", "can't", "be", "spent", "to", "cast"],
        &["that", "mana", "cant", "be", "spent", "to", "cast"],
        &["that", "mana", "can't", "be", "spent", "to", "cast"],
    ];
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let start = parse_any_prefix_word_count(&words, PREFIXES)?;
    (start < words.len()).then_some(())?;
    let spec = token_slice_for_words(tokens, &view, start, words.len())?;
    let filter = matches_any_exact_tokens(
        spec,
        &[&["nonartifact", "spell"], &["nonartifact", "spells"]],
    )
    .then(|| ObjectFilter::default().with_type(CardType::Artifact))?;
    Some(ManaUsageRestriction::CastSpellMatching {
        filter,
        restrict_to_matching_spell: true,
        grant_uncounterable: false,
        enters_with_counters: vec![],
        granted_abilities: vec![],
    })
}

fn parse_activate_ability_restriction(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    matches_any_exact_tokens(
        tokens,
        &[
            &[
                "spend",
                "this",
                "mana",
                "only",
                "to",
                "activate",
                "abilities",
            ],
            &[
                "spend", "this", "mana", "only", "to", "activate", "an", "ability",
            ],
        ],
    )
    .then_some(ManaUsageRestriction::ActivateAbility)
}

fn parse_legacy_restriction(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    let parsed = parse_legacy_cast_shape(tokens)?;
    let (tail, subtype_requirement) = strip_chosen_type_tail(parsed.tail_tokens);
    let tail = trim_lexed_commas(tail);
    let grant_uncounterable = matches_any_exact_tokens(tail, UNCOUNTERABLE_TAILS);
    if !grant_uncounterable && !TokenWordView::new(tail).is_empty() {
        return None;
    }
    Some(ManaUsageRestriction::CastSpell {
        card_types: vec![parsed.card_type],
        subtype_requirement,
        restrict_to_matching_spell: true,
        grant_uncounterable,
        enters_with_counters: vec![],
        granted_abilities: vec![],
    })
}

fn parse_legacy_cast_shape(tokens: &[OwnedLexToken]) -> Option<LegacyCastShape<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    parse_any_prefix_words(&mut input, SPEND_MANA_CAST_PREFIXES)?;
    opt(alt((
        primitives::word_slice_exact("a"),
        primitives::word_slice_exact("an"),
    )))
    .parse_next(&mut input)
    .ok()?;
    let card_type = leaf::parse_leaf_card_type_complete(take_word(&mut input).ok()?).ok()?;
    alt((
        primitives::word_slice_exact("spell"),
        primitives::word_slice_exact("spells"),
    ))
    .parse_next(&mut input)
    .ok()?;
    let tail_start = words.len().checked_sub(input.len())?;
    Some(LegacyCastShape {
        card_type,
        tail_tokens: token_slice_for_words(tokens, &view, tail_start, words.len())?,
    })
}

fn strip_chosen_type_tail(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<ManaUsageSubtypeRequirement>) {
    let tokens = trim_lexed_commas(tokens);
    let mut input = LexStream::new(tokens);
    if primitives::phrase(&["of", "the", "chosen", "type"])
        .parse_next(&mut input)
        .is_err()
    {
        return (tokens, None);
    }
    let consumed = tokens.len().saturating_sub(input.len());
    (
        &tokens[consumed..],
        Some(ManaUsageSubtypeRequirement::ChosenTypeOfSource),
    )
}

fn parse_filter_restriction(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    let parsed = parse_filter_cast_shape(tokens)?;
    let spec_tokens = trim_lexed_commas(parsed.spec_tokens);
    let spec_shape = classify_spec(spec_tokens);
    let special_filter = parse_special_spell_filter(spec_tokens);
    if special_filter.is_none() && spec_shape == ManaUsageSpecShape::Unsupported {
        return None;
    }
    let filter = special_filter.or_else(|| {
        let filter = parse_spell_filter_with_grammar_entrypoint(spec_tokens);
        (filter != ObjectFilter::default() || spec_shape == ManaUsageSpecShape::PlainSpell)
            .then_some(filter)
    })?;
    Some(ManaUsageRestriction::CastSpellMatching {
        filter,
        restrict_to_matching_spell: true,
        grant_uncounterable: parsed.grant_uncounterable,
        enters_with_counters: vec![],
        granted_abilities: vec![],
    })
}

fn parse_filter_cast_shape(tokens: &[OwnedLexToken]) -> Option<FilterCastShape<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let spec_start = parse_any_prefix_word_count(&words, SPEND_MANA_CAST_PREFIXES)?;
    if spec_start >= words.len() {
        return None;
    }
    if let Some(suffix) = last_exact_suffix_offset(words.get(spec_start..)?, UNCOUNTERABLE_TAILS) {
        if suffix == 0 {
            return None;
        }
        return Some(FilterCastShape {
            spec_tokens: token_slice_for_words(tokens, &view, spec_start, spec_start + suffix)?,
            grant_uncounterable: true,
        });
    }
    Some(FilterCastShape {
        spec_tokens: token_slice_for_words(tokens, &view, spec_start, words.len())?,
        grant_uncounterable: false,
    })
}

fn classify_spec(tokens: &[OwnedLexToken]) -> ManaUsageSpecShape {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    while let Ok(word) = take_word(&mut input) {
        if UNSUPPORTED_SPEC_WORDS
            .iter()
            .any(|candidate| *candidate == word)
        {
            return ManaUsageSpecShape::Unsupported;
        }
    }
    let mut input: primitives::WordSliceInput<'_> = &words;
    while let Ok(word) = take_word(&mut input) {
        if !PLAIN_SPELL_WORDS.iter().any(|candidate| *candidate == word) {
            return ManaUsageSpecShape::Other;
        }
    }
    ManaUsageSpecShape::PlainSpell
}

fn parse_mana_usage_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    parse_special_spell_filter(tokens)
        .or_else(|| parse_simple_subtype_spell_filter(tokens))
        .or_else(|| {
            let filter = parse_spell_filter_with_grammar_entrypoint(tokens);
            (filter != ObjectFilter::default()).then_some(filter)
        })
}

fn parse_simple_subtype_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let tokens = strip_article(trim_lexed_commas(tokens));
    let words = TokenWordView::new(tokens).word_refs();
    let [subtype_word, spell_word] = words.as_slice() else {
        return None;
    };
    matches!(*spell_word, "spell" | "spells").then_some(())?;
    Some(
        ObjectFilter::default()
            .with_subtype(leaf::parse_leaf_subtype_flexible_complete(subtype_word).ok()?),
    )
}

fn parse_ability_source_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let tokens = strip_article(trim_lexed_commas(tokens));
    let words = TokenWordView::new(tokens).word_refs();
    let [subtype_word, source_word] = words.as_slice() else {
        return None;
    };
    matches!(*source_word, "source" | "sources").then_some(())?;
    Some(
        ObjectFilter::default()
            .with_subtype(leaf::parse_leaf_subtype_flexible_complete(subtype_word).ok()?),
    )
}

fn parse_special_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let tokens = strip_article(tokens);
    if matches_any_exact_tokens(
        tokens,
        &[
            &["monocolored", "spell", "of", "that", "color"],
            &["monocolored", "spells", "of", "that", "color"],
            &["monocolored", "spell", "of", "the", "chosen", "color"],
            &["monocolored", "spells", "of", "the", "chosen", "color"],
        ],
    ) {
        return Some(ObjectFilter::default().monocolored().of_chosen_color());
    }
    if matches_any_exact_tokens(
        tokens,
        &[
            &["your", "commander"],
            &["your", "commander", "spell"],
            &["your", "commander", "spells"],
        ],
    ) {
        return Some(
            ObjectFilter::default()
                .commander()
                .owned_by(PlayerFilter::You),
        );
    }
    if matches_any_exact_tokens(
        tokens,
        &[
            &["spell", "from", "your", "graveyard"],
            &["spells", "from", "your", "graveyard"],
        ],
    ) {
        return Some(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
    }
    if matches_any_exact_tokens(
        tokens,
        &[&["spell", "from", "exile"], &["spells", "from", "exile"]],
    ) {
        return Some(ObjectFilter::default().in_zone(Zone::Exile));
    }
    if matches_any_exact_tokens(
        tokens,
        &[&["spell", "with", "devoid"], &["spells", "with", "devoid"]],
    ) {
        return Some(ObjectFilter::default().with_static_ability(StaticAbilityId::MakeColorless));
    }
    if matches_any_exact_tokens(
        tokens,
        &[
            &["creature", "spell", "with", "no", "abilities"],
            &["creature", "spells", "with", "no", "abilities"],
        ],
    ) {
        let mut filter = ObjectFilter::default().with_type(CardType::Creature);
        filter.no_abilities = true;
        return Some(filter);
    }
    if matches_any_exact_tokens(
        tokens,
        &[
            &["spell", "you", "don't", "own"],
            &["spell", "you", "dont", "own"],
            &["spells", "you", "don't", "own"],
            &["spells", "you", "dont", "own"],
        ],
    ) {
        return Some(ObjectFilter::default().owned_by(PlayerFilter::NotYou));
    }
    None
}

fn strip_article(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut input = LexStream::new(tokens);
    if alt((primitives::kw("a"), primitives::kw("an")))
        .parse_next(&mut input)
        .is_err()
    {
        return tokens;
    }
    &tokens[tokens.len().saturating_sub(input.len())..]
}

fn parse_mana_spend_bonus_shape(tokens: &[OwnedLexToken]) -> Option<ManaSpendBonusShape<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let spec_start = parse_any_prefix_word_count(&words, IF_MANA_SPENT_SPELL_PREFIXES)?;
    let spell_offset = first_word_choice(words.get(spec_start..)?, &["spell", "spells"])?;
    if spell_offset == 0 {
        return None;
    }
    let spell_word_end = spec_start + spell_offset + 1;
    let head_end = view.token_index_after_words(spell_word_end)?;
    let after_head = tokens.get(head_end..)?;
    let mut input = LexStream::new(after_head);
    let skipped: &[OwnedLexToken] = repeat_till(0.., any.void(), peek(primitives::comma()).void())
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input)
        .ok()?;
    primitives::comma().parse_next(&mut input).ok()?;
    let bonus_start = head_end + skipped.len() + 1;
    Some(ManaSpendBonusShape {
        spec_tokens: token_slice_for_words(tokens, &view, spec_start, spec_start + spell_offset)?,
        bonus_tokens: tokens.get(bonus_start..)?,
    })
}

fn parse_simple_bonus_card_type(tokens: &[OwnedLexToken]) -> Option<CardType> {
    let tokens = strip_article(tokens);
    let words = TokenWordView::new(tokens).word_refs();
    let [word] = words.as_slice() else {
        return None;
    };
    leaf::parse_leaf_card_type_complete(word).ok()
}

fn parse_nondefault_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let filter = parse_spell_filter_with_grammar_entrypoint(tokens);
    (filter != ObjectFilter::default()).then_some(filter)
}

fn parse_mana_spend_counter_shape(tokens: &[OwnedLexToken]) -> Option<ManaSpendCounterShape<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let enter = first_word_choice(&words, &["enter", "enters"])?;
    if enter == 0 || enter + 2 > words.len() {
        return None;
    }
    let subject = token_slice_for_words(tokens, &view, 0, enter)?;
    if !mana_spend_counter_subject_matches(subject) {
        return None;
    }
    let mut tail: primitives::WordSliceInput<'_> = words.get(enter + 1..)?;
    primitives::word_slice_exact("with")
        .parse_next(&mut tail)
        .ok()?;
    if tail.is_empty() {
        return None;
    }
    let start = words.len().checked_sub(tail.len())?;
    Some(ManaSpendCounterShape {
        counter_tail_tokens: token_slice_for_words(tokens, &view, start, words.len())?,
    })
}

fn mana_spend_counter_subject_matches(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    if primitives::word_slice_exact("that")
        .parse_next(&mut input)
        .is_err()
    {
        return false;
    }
    let Ok(noun) = take_word(&mut input) else {
        return false;
    };
    matches!(noun, "creature" | "spell" | "permanent" | "card")
        || leaf::parse_leaf_card_type_complete(noun).is_ok()
}

fn parse_mana_spend_counter_tail(bonus: ManaSpendCounterShape<'_>) -> Option<(CounterType, u32)> {
    let tokens = bonus.counter_tail_tokens;
    let (count, used) = if tokens
        .first()
        .is_some_and(|token| token.is_any_word(&["a", "an"]))
        && tokens
            .get(1)
            .is_some_and(|token| token.is_word("additional"))
    {
        (1, 2)
    } else if tokens
        .first()
        .is_some_and(|token| token.is_word("additional"))
    {
        (1, 1)
    } else if let Some((parsed, number_used)) = parse_number(tokens) {
        let used = if tokens
            .get(number_used)
            .is_some_and(|token| token.is_word("additional"))
        {
            number_used + 1
        } else {
            number_used
        };
        (parsed, used)
    } else {
        return None;
    };
    let counter_type = parse_counter_type_from_tokens(tokens.get(used..)?)?;
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let counter = first_word_choice(&words, &["counter", "counters"])?;
    let tail = token_slice_for_words(tokens, &view, counter + 1, words.len())?;
    matches_any_exact_tokens(
        tail,
        &[
            &[],
            &["on", "it"],
            &["on", "that", "creature"],
            &["on", "that", "spell"],
            &["on", "that", "permanent"],
            &["on", "that", "card"],
        ],
    )
    .then_some((counter_type, count))
}

fn parse_any_prefix_word_count(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_any_prefix_words(&mut input, phrases)?;
    words.len().checked_sub(input.len())
}

fn parse_any_prefix_words<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    phrases: &[&[&str]],
) -> Option<()> {
    for phrase in phrases {
        let mut probe = *input;
        if parse_phrase_words(&mut probe, phrase).is_ok() {
            *input = probe;
            return Some(());
        }
    }
    None
}

fn first_phrase_choice<'a>(
    words: &[&str],
    phrases: &'a [&'a [&'a str]],
) -> Option<(&'a [&'a str], usize)> {
    let mut best = None;
    for phrase in phrases {
        let Some(offset) = phrase_offset_words(words, phrase) else {
            continue;
        };
        if best.is_none_or(|(_, current)| offset < current) {
            best = Some((*phrase, offset));
        }
    }
    best
}

fn first_word_choice(words: &[&str], expected: &[&str]) -> Option<usize> {
    let mut best = None;
    for word in expected {
        let Some(offset) = phrase_offset_words(words, &[*word]) else {
            continue;
        };
        best = Some(best.map_or(offset, |current: usize| current.min(offset)));
    }
    best
}

fn last_exact_suffix_offset(words: &[&str], tails: &[&[&str]]) -> Option<usize> {
    for start in (0..words.len()).rev() {
        for tail in tails {
            if matches_exact_word_slice(words.get(start..)?, tail) {
                return Some(start);
            }
        }
    }
    None
}

fn matches_exact_word_slice(words: &[&str], phrase: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (
        |input: &mut primitives::WordSliceInput<'_>| parse_phrase_words(input, phrase),
        eof,
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}

fn token_slice_for_words<'a>(
    tokens: &'a [OwnedLexToken],
    view: &TokenWordView<'a>,
    start: usize,
    end: usize,
) -> Option<&'a [OwnedLexToken]> {
    Some(trim_lexed_commas(
        tokens.get(view.token_span_for_words(start, end)?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn mana_usage_shapes_return_existing_typed_restrictions() {
        assert_eq!(
            parse_mana_usage_restriction_sentence_lexed(&lex(
                "Spend this mana only to cast a creature spell."
            )),
            Some(ManaUsageRestriction::CastSpell {
                card_types: vec![CardType::Creature],
                subtype_requirement: None,
                restrict_to_matching_spell: true,
                grant_uncounterable: false,
                enters_with_counters: vec![],
                granted_abilities: vec![],
            })
        );
        assert_eq!(
            parse_mana_usage_restriction_sentence_lexed(&lex(
                "Spend this mana only to activate an ability."
            )),
            Some(ManaUsageRestriction::ActivateAbility)
        );
    }

    #[test]
    fn mana_spend_bonus_shapes_preserve_haste_and_counter_facts() {
        let haste = parse_mana_spend_bonus_sentence_lexed(&lex(
            "If this mana is spent to cast a creature spell, it gains haste.",
        ))
        .unwrap();
        assert!(matches!(
            haste,
            ManaUsageRestriction::CastSpell { granted_abilities, .. }
                if granted_abilities == [StaticAbilityId::Haste]
        ));

        let counter = parse_mana_spend_bonus_sentence_lexed(&lex(
            "If this mana is spent to cast a creature spell, that creature enters with an additional +1/+1 counter on it.",
        ));
        assert!(counter.is_some());
    }
}
