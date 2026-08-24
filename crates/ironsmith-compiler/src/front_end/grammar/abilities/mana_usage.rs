use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

use crate::ability::{
    ManaPaymentPredicate, ManaPaymentPurpose, ManaSpendAbilityGrantDuration,
    ManaSpendBonusCondition, ManaSpendGrantedKeyword, ManaUsageSubtypeRequirement,
};
use crate::cards::builders::{
    EffectAst, PlayerAst, SubjectVerbActionAst, SubjectVerbRoleAst, TargetAst,
};
use crate::effect::Value;
use crate::model::CompilerManaUsageRestriction as ManaUsageRestriction;
use crate::object::CounterType;
use crate::static_abilities::StaticAbilityId;
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
use crate::util::{parse_counter_type_from_tokens, parse_number};

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

pub fn is_spend_mana_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_prefix_tokens(tokens, SPEND_MANA_RESTRICTION_PREFIXES)
}

pub fn parse_mana_usage_restriction_sentence_lexed(
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
    let (_, spell_words) = crate::slice_primitives::strip_any_suffix(&words, SEPARATORS)?;
    let separator_start = spell_words.len();
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

pub fn parse_mana_spend_bonus_sentence_lexed(
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
    let words = TokenWordView::new(tokens).word_refs();
    if crate::word_primitives::parse_any_sequence_complete(
        &words,
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
    if crate::word_primitives::parse_any_sequence_complete(
        &words,
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

    let cast_prefix = crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[
            &["when", "that", "mana", "is", "spent", "to", "cast"],
            &["when", "you", "spend", "this", "mana", "to", "cast"],
        ],
    );
    if !cast_prefix {
        return None;
    }

    let (filter, additional_predicate) = if crate::word_primitives::sequence_occurs(
        &words,
        &[
            "creature", "spell", "that", "shares", "a", "creature", "type", "with",
        ],
    ) && crate::word_primitives::sequence_occurs(
        &words,
        &["your", "commander"],
    ) {
        (
            ObjectFilter::default().with_type(CardType::Creature),
            Some(ManaPaymentPredicate::SharesCreatureTypeWithPayersCommander),
        )
    } else if crate::word_primitives::sequence_occurs(&words, &["your", "commander", "scry", "x"]) {
        (
            ObjectFilter::default()
                .commander()
                .owned_by(PlayerFilter::You),
            None,
        )
    } else if crate::word_primitives::sequence_occurs(
        &words,
        &["red", "instant", "or", "sorcery", "spell"],
    ) {
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
    let effect = if crate::word_primitives::sequence_occurs(&words, &["scry", "1"]) {
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::Scry {
                count: Value::Fixed(1),
            },
        )
    } else if crate::word_primitives::sequence_occurs(&words, &["scry", "x"]) {
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::Scry {
                count: Value::CommanderCastCount(PlayerFilter::You),
            },
        )
    } else if crate::word_primitives::sequence_occurs(&words, &["copy", "that", "spell"]) {
        EffectAst::subject_verb_copy_spell(
            TargetAst::Tagged(
                crate::tag::TagKey::from(ironsmith_core::MANA_PAID_OBJECT_TAG),
                None,
            ),
            Value::Fixed(1),
            PlayerAst::You,
            false,
            false,
            Vec::new(),
        )
    } else {
        return None;
    };

    Some(ManaUsageRestriction::PaymentTransaction {
        restriction: None,
        on_spend: vec![ironsmith_core::ManaSpendPayload {
            predicate: ManaPaymentPredicate::All(predicates),
            effects: ironsmith_core::ResolutionProgram::from_effects(vec![effect]),
            choices: Vec::new(),
        }],
    })
}

pub fn is_mana_spend_bonus_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
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

#[cfg(test)]
#[path = "mana_usage_inline_tests.rs"]
mod tests;

#[path = "mana_usage/mana_usage_object_action_programs.rs"]
mod mana_usage_object_action_programs;
use mana_usage_object_action_programs::token_slice_for_words;
#[path = "mana_usage/mana_usage_core_programs.rs"]
mod mana_usage_core_programs;
use mana_usage_core_programs::{
    last_exact_suffix_offset, matches_exact_word_slice, parse_any_prefix_word_count,
    parse_any_prefix_words, strip_article,
};
#[path = "mana_usage/mana_usage_choice_programs.rs"]
mod mana_usage_choice_programs;
use mana_usage_choice_programs::{first_phrase_choice, first_word_choice};
#[path = "mana_usage/mana_usage_counter_programs.rs"]
mod mana_usage_counter_programs;
use mana_usage_counter_programs::{
    mana_spend_counter_subject_matches, parse_mana_spend_counter_shape,
    parse_mana_spend_counter_tail,
};
#[path = "mana_usage/mana_usage_reference_programs.rs"]
mod mana_usage_reference_programs;
use mana_usage_reference_programs::{
    parse_ability_source_filter, parse_filter_cast_shape, parse_mana_usage_spell_filter,
    parse_nondefault_spell_filter, parse_simple_subtype_spell_filter, parse_special_spell_filter,
};
#[path = "mana_usage/mana_usage_library_programs.rs"]
mod mana_usage_library_programs;
use mana_usage_library_programs::parse_simple_bonus_card_type;
#[path = "mana_usage/mana_usage_resource_programs.rs"]
mod mana_usage_resource_programs;
use mana_usage_resource_programs::{
    parse_mana_spend_bonus_condition_prefix, parse_mana_spend_bonus_shape,
    strip_mana_spend_duration_grant_suffix,
};
#[path = "mana_usage/mana_usage_permission_programs.rs"]
mod mana_usage_permission_programs;
use mana_usage_permission_programs::parse_alternative_cast_spell_with_origin;
#[path = "mana_usage/mana_usage_condition_programs.rs"]
mod mana_usage_condition_programs;
use mana_usage_condition_programs::classify_spec;
