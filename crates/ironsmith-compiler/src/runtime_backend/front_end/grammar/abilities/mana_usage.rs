use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

use crate::ability::{
    ManaPaymentPredicate, ManaPaymentPurpose, ManaSpendAbilityGrantDuration,
    ManaSpendBonusCondition, ManaSpendGrantedKeyword, ManaSpendPayload, ManaUsageRestriction,
    ManaUsageSubtypeRequirement,
};
use crate::effect::{Effect, Value};
use crate::object::CounterType;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbilityId;
use crate::target::ChooseSpec;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::filters::parse_spell_filter_with_grammar_entrypoint;
use super::super::{leaf, primitives};
use super::surface::{
    matches_any_exact_tokens, matches_any_prefix_tokens, parse_phrase_words, phrase_offset_words,
    take_word,
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
const WHEN_MANA_SPENT_SPELL_PREFIXES: &[&[&str]] =
    &[&["when", "you", "spend", "this", "mana", "to", "cast"]];
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
    condition: ManaSpendBonusCondition,
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
    parse_generic_mana_transaction(tokens)
        .or_else(|| parse_cast_unlock_turn_face_up(tokens))
        .or_else(|| parse_cast_or_activate_source(tokens))
        .or_else(|| parse_cast_or_activate_any_ability(tokens))
        .or_else(|| parse_activate_any_ability_or_cast(tokens))
        .or_else(|| parse_legacy_restriction(tokens))
        .or_else(|| parse_activate_ability_restriction(tokens))
        .or_else(|| parse_cant_be_spent_restriction(tokens))
        .or_else(|| parse_filter_restriction(tokens))
}

fn cast_spell_payment_predicate(filter: ObjectFilter) -> ManaPaymentPredicate {
    ManaPaymentPredicate::All(vec![
        ManaPaymentPredicate::Purpose(ManaPaymentPurpose::CastSpell),
        ManaPaymentPredicate::SourceMatches(filter),
    ])
}

fn any_ability_payment_predicates() -> [ManaPaymentPredicate; 2] {
    [
        ManaPaymentPredicate::Purpose(ManaPaymentPurpose::ActivateAbility),
        ManaPaymentPredicate::Purpose(ManaPaymentPurpose::ActivateManaAbility),
    ]
}

fn cast_or_any_ability_restriction(spell_filter: ObjectFilter) -> ManaUsageRestriction {
    let [activate, activate_mana] = any_ability_payment_predicates();
    ManaUsageRestriction::PaymentTransaction {
        restriction: Some(ManaPaymentPredicate::AnyOf(vec![
            cast_spell_payment_predicate(spell_filter),
            activate,
            activate_mana,
        ])),
        on_spend: Vec::new(),
    }
}

fn parse_cast_or_activate_any_ability(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    const SEPARATORS: &[&[&str]] = &[
        &["or", "activate", "an", "ability"],
        &["or", "activate", "abilities"],
        &["or", "to", "activate", "an", "ability"],
        &["or", "to", "activate", "abilities"],
    ];
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let prefix_end = parse_any_prefix_word_count(&words, SPEND_MANA_CAST_PREFIXES)?;
    let separator = SEPARATORS.iter().find(|separator| {
        words.get(words.len().saturating_sub(separator.len())..) == Some(**separator)
    })?;
    let separator_start = words.len().checked_sub(separator.len())?;
    if separator_start <= prefix_end {
        return None;
    }
    let spell_tokens = token_slice_for_words(tokens, &view, prefix_end, separator_start)?;
    Some(cast_or_any_ability_restriction(
        parse_mana_usage_spell_filter(spell_tokens)?,
    ))
}

fn parse_activate_any_ability_or_cast(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    const PREFIXES: &[&[&str]] = &[
        &[
            "spend", "this", "mana", "only", "to", "activate", "an", "ability", "or", "cast",
        ],
        &[
            "spend",
            "this",
            "mana",
            "only",
            "to",
            "activate",
            "abilities",
            "or",
            "cast",
        ],
        &[
            "spend", "that", "mana", "only", "to", "activate", "an", "ability", "or", "cast",
        ],
        &[
            "spend",
            "that",
            "mana",
            "only",
            "to",
            "activate",
            "abilities",
            "or",
            "cast",
        ],
    ];
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let spell_start = parse_any_prefix_word_count(&words, PREFIXES)?;
    let spell_tokens = token_slice_for_words(tokens, &view, spell_start, words.len())?;
    Some(cast_or_any_ability_restriction(
        parse_mana_usage_spell_filter(spell_tokens)?,
    ))
}

pub(crate) fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    if let Some(transaction) = parse_generic_mana_transaction(tokens)
        && matches!(
            transaction,
            ManaUsageRestriction::PaymentTransaction { ref on_spend, .. } if !on_spend.is_empty()
        )
    {
        return Some(transaction);
    }
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
    let granted_keywords = if matches_any_exact_tokens(
        bonus_tokens,
        &[
            &["it", "gains", "riot"],
            &["that", "creature", "gains", "riot"],
            &["that", "spell", "gains", "riot"],
        ],
    ) {
        vec![ManaSpendGrantedKeyword::Riot]
    } else {
        Vec::new()
    };

    let (counter_bonus_tokens, mut duration_grants) =
        strip_mana_spend_duration_grant_suffix(bonus_tokens);
    duration_grants.extend(
        granted_abilities
            .iter()
            .copied()
            .map(|ability| (ability, ManaSpendAbilityGrantDuration::UntilEndOfTurn)),
    );
    let enters_with_counters = parse_mana_spend_counter_shape(counter_bonus_tokens)
        .and_then(parse_mana_spend_counter_tail)
        .into_iter()
        .collect::<Vec<_>>();

    if simple_card_type.is_none()
        || matches!(
            parsed.condition,
            ManaSpendBonusCondition::WhenYouSpendThisManaToCast
        )
        || duration_grants
            .iter()
            .any(|(_, duration)| *duration == ManaSpendAbilityGrantDuration::UntilYourNextTurn)
        || !granted_keywords.is_empty()
    {
        if !grant_uncounterable
            && enters_with_counters.is_empty()
            && duration_grants.is_empty()
            && granted_keywords.is_empty()
        {
            return None;
        }
        let filter = simple_card_type
            .map(|card_type| ObjectFilter::default().with_type(card_type))
            .or_else(|| parse_nondefault_spell_filter(spec_tokens))?;
        return Some(ManaUsageRestriction::CastSpellWithManaBonus {
            filter,
            condition: parsed.condition,
            grant_uncounterable,
            enters_with_counters,
            granted_abilities: duration_grants,
            granted_keywords,
        });
    }

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
    let (counter_type, count) = enters_with_counters.into_iter().next()?;
    Some(ManaUsageRestriction::CastSpell {
        card_types: vec![card_type],
        subtype_requirement: None,
        restrict_to_matching_spell: false,
        grant_uncounterable: false,
        enters_with_counters: vec![(counter_type, count)],
        granted_abilities: vec![],
    })
}

fn parse_generic_mana_transaction(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    if matches_any_exact_tokens(
        tokens,
        &[
            &[
                "spend",
                "this",
                "mana",
                "only",
                "to",
                "pay",
                "cumulative",
                "upkeep",
                "costs",
            ],
            &[
                "spend",
                "that",
                "mana",
                "only",
                "to",
                "pay",
                "cumulative",
                "upkeep",
                "costs",
            ],
        ],
    ) {
        return Some(ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::Purpose(
                ManaPaymentPurpose::CumulativeUpkeep,
            )),
            on_spend: Vec::new(),
        });
    }
    if matches_any_exact_tokens(
        tokens,
        &[
            &[
                "spend", "this", "mana", "only", "on", "costs", "that", "contain", "x",
            ],
            &[
                "spend", "that", "mana", "only", "on", "costs", "that", "contain", "x",
            ],
        ],
    ) {
        return Some(ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::CostContainsX),
            on_spend: Vec::new(),
        });
    }

    let words = TokenWordView::new(tokens).word_refs();
    let cast_prefix = words.starts_with(&["when", "that", "mana", "is", "spent", "to", "cast"])
        || words.starts_with(&["when", "you", "spend", "this", "mana", "to", "cast"]);
    if !cast_prefix {
        return None;
    }

    let (filter, additional_predicate) = if words.windows(8).any(|window| {
        window
            == [
                "creature", "spell", "that", "shares", "a", "creature", "type", "with",
            ]
    }) && words
        .windows(2)
        .any(|window| window == ["your", "commander"])
    {
        (
            ObjectFilter::default().with_type(CardType::Creature),
            Some(ManaPaymentPredicate::SharesCreatureTypeWithPayersCommander),
        )
    } else if words
        .windows(4)
        .any(|window| window == ["your", "commander", "scry", "x"])
    {
        (
            ObjectFilter::default()
                .commander()
                .owned_by(PlayerFilter::You),
            None,
        )
    } else if words
        .windows(5)
        .any(|window| window == ["red", "instant", "or", "sorcery", "spell"])
    {
        let mut filter = ObjectFilter::default().with_colors(crate::color::ColorSet::RED);
        filter.card_types = vec![CardType::Instant, CardType::Sorcery];
        (filter, None)
    } else {
        return None;
    };

    let mut predicates = vec![
        ManaPaymentPredicate::Purpose(ManaPaymentPurpose::CastSpell),
        ManaPaymentPredicate::SourceMatches(filter),
    ];
    predicates.extend(additional_predicate);
    let effect = if words.windows(2).any(|window| window == ["scry", "1"]) {
        Effect::scry(1)
    } else if words.windows(2).any(|window| window == ["scry", "x"]) {
        Effect::scry(Value::CommanderCastCount(PlayerFilter::You))
    } else if words
        .windows(3)
        .any(|window| window == ["copy", "that", "spell"])
    {
        Effect::new(crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(
            crate::tag::TagKey::from(ironsmith_core::MANA_PAID_OBJECT_TAG),
        )))
    } else {
        return None;
    };

    Some(ManaUsageRestriction::PaymentTransaction {
        restriction: None,
        on_spend: vec![ManaSpendPayload {
            predicate: ManaPaymentPredicate::All(predicates),
            effects: ResolutionProgram::from_effects(vec![effect]),
            choices: Vec::new(),
        }],
    })
}

pub(crate) fn is_mana_spend_bonus_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    parse_mana_spend_bonus_sentence_lexed(tokens).is_some()
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
        &["or", "to", "activate", "an", "ability", "of"],
        &["or", "to", "activate", "abilities", "of"],
        &["and", "activate", "an", "ability", "of"],
        &["and", "activate", "abilities", "of"],
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
    let forbidden_filter = if matches_any_exact_tokens(
        spec,
        &[
            &["a", "nonartifact", "spell"],
            &["nonartifact", "spell"],
            &["nonartifact", "spells"],
        ],
    ) {
        ObjectFilter::default().without_type(CardType::Artifact)
    } else if matches_any_exact_tokens(
        spec,
        &[
            &["a", "spell", "from", "your", "hand"],
            &["spells", "from", "your", "hand"],
        ],
    ) {
        ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You)
    } else {
        return None;
    };
    Some(ManaUsageRestriction::PaymentTransaction {
        restriction: Some(ManaPaymentPredicate::Not(Box::new(
            ManaPaymentPredicate::All(vec![
                ManaPaymentPredicate::Purpose(ManaPaymentPurpose::CastSpell),
                ManaPaymentPredicate::SourceMatches(forbidden_filter),
            ]),
        ))),
        on_spend: Vec::new(),
    })
}

fn parse_activate_ability_restriction(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    if matches_any_exact_tokens(
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
                "of",
                "artifact",
                "sources",
            ],
            &[
                "spend",
                "that",
                "mana",
                "only",
                "to",
                "activate",
                "abilities",
                "of",
                "artifact",
                "sources",
            ],
        ],
    ) {
        return Some(ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::All(vec![
                ManaPaymentPredicate::AnyOf(vec![
                    ManaPaymentPredicate::Purpose(ManaPaymentPurpose::ActivateAbility),
                    ManaPaymentPredicate::Purpose(ManaPaymentPurpose::ActivateManaAbility),
                ]),
                ManaPaymentPredicate::SourceMatches(
                    ObjectFilter::default().with_type(CardType::Artifact),
                ),
            ])),
            on_spend: Vec::new(),
        });
    }

    const SOURCE_PREFIXES: &[&[&str]] = &[
        &[
            "spend",
            "this",
            "mana",
            "only",
            "to",
            "activate",
            "abilities",
            "of",
        ],
        &[
            "spend", "this", "mana", "only", "to", "activate", "an", "ability", "of",
        ],
        &[
            "spend",
            "that",
            "mana",
            "only",
            "to",
            "activate",
            "abilities",
            "of",
        ],
        &[
            "spend", "that", "mana", "only", "to", "activate", "an", "ability", "of",
        ],
    ];
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if let Some(source_start) = parse_any_prefix_word_count(&words, SOURCE_PREFIXES)
        && source_start < words.len()
    {
        let source_tokens = token_slice_for_words(tokens, &view, source_start, words.len())?;
        let source_filter = parse_ability_source_filter(source_tokens)?;
        let [activate, activate_mana] = any_ability_payment_predicates();
        return Some(ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::All(vec![
                ManaPaymentPredicate::AnyOf(vec![activate, activate_mana]),
                ManaPaymentPredicate::SourceMatches(source_filter),
            ])),
            on_spend: Vec::new(),
        });
    }

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
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let semantic_end = words
        .last()
        .is_some_and(|word| matches!(*word, "source" | "sources"))
        .then(|| words.len().saturating_sub(1))
        .unwrap_or(words.len());
    if semantic_end == 0 {
        return None;
    }
    let semantic = token_slice_for_words(tokens, &view, 0, semantic_end)?;
    let parsed = parse_spell_filter_with_grammar_entrypoint(semantic);
    if parsed != ObjectFilter::default() {
        return Some(parsed);
    }

    let semantic_words = TokenWordView::new(semantic).word_refs();
    let [kind] = semantic_words.as_slice() else {
        return None;
    };
    match *kind {
        "artifact" | "artifacts" => Some(ObjectFilter::default().with_type(CardType::Artifact)),
        "creature" | "creatures" => Some(ObjectFilter::default().with_type(CardType::Creature)),
        "land" | "lands" => Some(ObjectFilter::default().with_type(CardType::Land)),
        _ => Some(
            ObjectFilter::default()
                .with_subtype(leaf::parse_leaf_subtype_flexible_complete(kind).ok()?),
        ),
    }
}

fn parse_alternative_cast_spell_with_origin(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let tokens = strip_article(trim_lexed_commas(tokens));
    let words = TokenWordView::new(tokens).word_refs();
    if !matches!(words.first().copied(), Some("spell" | "spells"))
        || words.get(1).copied() != Some("with")
    {
        return None;
    }

    let alternative = leaf::parse_leaf_alternative_cast_prefix_words(words.get(2..)?)?;
    let origin = words.get(2 + alternative.consumed..)?;
    let (zone, owner) = match origin {
        ["from", "a", "graveyard"] | ["from", "graveyard"] => (Zone::Graveyard, None),
        ["from", "your", "graveyard"] => (Zone::Graveyard, Some(PlayerFilter::You)),
        ["from", "a", "hand"] | ["from", "hand"] => (Zone::Hand, None),
        ["from", "your", "hand"] => (Zone::Hand, Some(PlayerFilter::You)),
        ["from", "a", "library"] | ["from", "library"] => (Zone::Library, None),
        ["from", "your", "library"] => (Zone::Library, Some(PlayerFilter::You)),
        ["from", "exile"] => (Zone::Exile, None),
        ["from", "the", "command", "zone"] => (Zone::Command, None),
        ["from", "outside", "the", "game"] => (Zone::OutsideGame, None),
        _ => return None,
    };

    let mut filter = ObjectFilter::default().in_zone(zone);
    filter.owner = owner;
    filter.alternative_cast = Some(alternative.kind);
    Some(filter)
}

fn parse_special_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    if let Some(filter) = parse_alternative_cast_spell_with_origin(tokens) {
        return Some(filter);
    }
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
    let (spec_start, condition) = parse_mana_spend_bonus_condition_prefix(&words)?;
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
        condition,
    })
}

fn parse_mana_spend_bonus_condition_prefix(
    words: &[&str],
) -> Option<(usize, ManaSpendBonusCondition)> {
    let candidates = [
        (
            &["if", "this", "mana", "is", "spent", "to", "cast"] as &[&str],
            ManaSpendBonusCondition::IfThisManaIsSpentToCast,
        ),
        (
            &["if", "that", "mana", "is", "spent", "to", "cast"] as &[&str],
            ManaSpendBonusCondition::IfThatManaIsSpentToCast,
        ),
        (
            &["if", "this", "mana", "is", "spent", "on"] as &[&str],
            ManaSpendBonusCondition::IfThisManaIsSpentOn,
        ),
        (
            &["if", "that", "mana", "is", "spent", "on"] as &[&str],
            ManaSpendBonusCondition::IfThatManaIsSpentOn,
        ),
        (
            WHEN_MANA_SPENT_SPELL_PREFIXES[0],
            ManaSpendBonusCondition::WhenYouSpendThisManaToCast,
        ),
    ];
    candidates.into_iter().find_map(|(prefix, condition)| {
        let mut input: primitives::WordSliceInput<'_> = words;
        parse_phrase_words(&mut input, prefix).ok()?;
        Some((words.len().saturating_sub(input.len()), condition))
    })
}

fn strip_mana_spend_duration_grant_suffix(
    tokens: &[OwnedLexToken],
) -> (
    &[OwnedLexToken],
    Vec<(StaticAbilityId, ManaSpendAbilityGrantDuration)>,
) {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let suffixes: &[&[&str]] = &[
        &["and", "gains", "hexproof", "until", "your", "next", "turn"],
        &[
            "and", "it", "gains", "hexproof", "until", "your", "next", "turn",
        ],
    ];
    let Some(start) = last_exact_suffix_offset(&words, suffixes) else {
        return (tokens, Vec::new());
    };
    let Some(primary) = token_slice_for_words(tokens, &view, 0, start) else {
        return (tokens, Vec::new());
    };
    (
        primary,
        vec![(
            StaticAbilityId::Hexproof,
            ManaSpendAbilityGrantDuration::UntilYourNextTurn,
        )],
    )
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
    if words.as_slice() == ["it"] {
        return true;
    }
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

    #[test]
    fn mana_spend_bonus_preserves_riot_as_a_runtime_keyword_grant() {
        let riot = parse_mana_spend_bonus_sentence_lexed(&lex(
            "If that mana is spent on a creature spell, it gains riot.",
        ));
        assert!(matches!(
            riot,
            Some(ManaUsageRestriction::CastSpellWithManaBonus {
                condition: ManaSpendBonusCondition::IfThatManaIsSpentOn,
                granted_keywords,
                ..
            }) if granted_keywords == [ManaSpendGrantedKeyword::Riot]
        ));
    }

    #[test]
    fn u078_parses_arbitrary_payment_purpose_and_cost_predicates() {
        let cumulative = parse_mana_usage_restriction_sentence_lexed(&lex(
            "Spend this mana only to pay cumulative upkeep costs.",
        ));
        assert!(matches!(
            cumulative,
            Some(ManaUsageRestriction::PaymentTransaction {
                restriction: Some(ManaPaymentPredicate::Purpose(
                    ManaPaymentPurpose::CumulativeUpkeep
                )),
                ref on_spend,
            }) if on_spend.is_empty()
        ));

        let contains_x = parse_mana_usage_restriction_sentence_lexed(&lex(
            "Spend this mana only on costs that contain {X}.",
        ));
        assert!(matches!(
            contains_x,
            Some(ManaUsageRestriction::PaymentTransaction {
                restriction: Some(ManaPaymentPredicate::CostContainsX),
                ref on_spend,
            }) if on_spend.is_empty()
        ));
    }

    #[test]
    fn u078_parses_generic_scry_and_copy_on_spend_payloads() {
        for text in [
            "When that mana is spent to cast a creature spell that shares a creature type with your commander, scry 1.",
            "When you spend this mana to cast your commander, scry X, where X is the number of times it's been cast from the command zone this game.",
            "When that mana is spent to cast a red instant or sorcery spell, copy that spell and you may choose new targets for the copy.",
        ] {
            let parsed = parse_mana_spend_bonus_sentence_lexed(&lex(text));
            assert!(
                matches!(
                    parsed,
                    Some(ManaUsageRestriction::PaymentTransaction {
                        restriction: None,
                        ref on_spend,
                    }) if on_spend.len() == 1 && !on_spend[0].effects.is_empty()
                ),
                "failed to parse {text}: {parsed:?}"
            );
        }
    }
}
