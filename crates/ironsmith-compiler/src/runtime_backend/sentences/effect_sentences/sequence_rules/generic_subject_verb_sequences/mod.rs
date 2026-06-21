use super::SentenceInput;

pub(crate) mod pairs;
pub(crate) mod quads;
pub(crate) mod triples;
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, ObjectFilter, OwnedLexToken, PlayerAst,
    PredicateAst, ReturnControllerAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey, TargetAst,
};
use crate::effect::{EventValueSpec, Value};
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::front_end::lexer::{
    LexedClause, word_slice_contains_all_words, word_slice_contains_any_word,
    word_slice_contains_phrase, word_slice_eq, word_slice_eq_any, word_slice_starts_with,
};
use crate::runtime_backend::object_filters::parse_object_filter_lexed;
use crate::runtime_backend::util::{mana_pips_from_token, non_article_token_word_refs};
use crate::target::PlayerFilter;
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GenericSequenceVerb {
    GainParameterizedAbility,
    SearchLibraryProcedure,
    IterateLibraryProcedure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GenericSubjectVerbSequence {
    pub(super) verb: GenericSequenceVerb,
    pub(super) consumed_sentences: usize,
}

impl GenericSubjectVerbSequence {
    pub(super) fn parameterized_flashback_grant() -> Self {
        Self {
            verb: GenericSequenceVerb::GainParameterizedAbility,
            consumed_sentences: 2,
        }
    }

    pub(super) fn prefixed_library_consult() -> Self {
        Self {
            verb: GenericSequenceVerb::SearchLibraryProcedure,
            consumed_sentences: 3,
        }
    }

    pub(super) fn iterative_library_procedure() -> Self {
        Self {
            verb: GenericSequenceVerb::IterateLibraryProcedure,
            consumed_sentences: 3,
        }
    }
}

const FLASHBACK_UNTIL_END_TAIL: &[&str] = &["flashback", "until", "end", "of", "turn"];
const FLASHBACK_COST_FOLLOWUPS: &[&[&str]] = &[
    &[
        "the",
        "flashback",
        "cost",
        "is",
        "equal",
        "to",
        "its",
        "mana",
        "cost",
    ],
    &[
        "that",
        "cards",
        "flashback",
        "cost",
        "is",
        "equal",
        "to",
        "its",
        "mana",
        "cost",
    ],
];

const EXILE_TOP_CARD_OF_LIBRARY: &[&str] = &["exile", "top", "card", "of", "your", "library"];
const ITERATIVE_LIBRARY_MAY_KEEP_UNLESS_DUPLICATE_NAME_CLAUSES: &[&[&str]] = &[
    &[
        "you", "may", "put", "that", "card", "into", "your", "hand", "unless", "it", "has", "same",
        "name", "as", "another", "card", "exiled", "this", "way",
    ],
    &[
        "you", "may", "put", "it", "into", "your", "hand", "unless", "it", "has", "same", "name",
        "as", "another", "card", "exiled", "this", "way",
    ],
];
const ITERATIVE_LIBRARY_REPEAT_UNTIL_KEEP_OR_DUPLICATE: &[&str] = &[
    "repeat",
    "this",
    "process",
    "until",
    "you",
    "put",
    "card",
    "into",
    "your",
    "hand",
    "or",
    "you",
    "exile",
    "two",
    "cards",
    "with",
    "same",
    "name",
    "whichever",
    "comes",
    "first",
];
const STARTING_EACH_PLAYER_MAY_PAY_ANY_LIFE: &[&str] = &[
    "starting", "with", "you", "each", "player", "may", "pay", "any", "amount", "of", "life",
];
const REPEAT_UNTIL_NO_ONE_PAYS_LIFE: &[&str] = &[
    "repeat", "this", "process", "until", "no", "one", "pays", "life",
];
const EACH_PLAYER_CREATES_RATS_FOR_LIFE_PAID: &[&str] = &[
    "each", "player", "creates", "1/1", "black", "rat", "creature", "token", "for", "each", "1",
    "life", "they", "paid", "this", "way",
];

const EACH_PLAYER_SHUFFLE_REVEAL_PREFIX: &[&str] = &["each", "player", "shuffles", "all"];
const EACH_PLAYER_SHUFFLE_REVEAL_REQUIRED_PHRASES: &[&[&str]] = &[
    &["they", "own", "into", "their", "library"],
    &[
        "then", "reveals", "that", "many", "cards", "from", "the", "top", "of", "their", "library",
    ],
];
const EACH_PLAYER_PUT_REVEALED_TYPES_PREFIX: &[&str] = &["each", "player", "puts", "all"];
const EACH_PLAYER_PUT_REVEALED_TYPES_REQUIRED_PHRASES: &[&[&str]] = &[
    &["revealed", "this", "way", "onto", "the", "battlefield"],
    &["then", "does", "the", "same", "for"],
    &["on", "the", "bottom", "of", "their", "library"],
];

const PREVENTED_DAMAGE_COUNTER_FOLLOWUP_PREFIX: &[&str] =
    &["for", "each", "1", "damage", "prevented", "this", "way"];
const PREVENTED_DAMAGE_COUNTER_FOLLOWUP_WORDS: &[&str] = &["put", "+1/+1", "counter", "on"];
const SOURCE_TAPPED_DURATION_PHRASE: &[&str] = &["for", "as", "long", "as"];
const SOURCE_TAPPED_DURATION_WORDS: &[&str] = &["remains", "tapped"];
const SOURCE_TAPPED_DURATION_SOURCE_WORDS: &[&str] = &[
    "this",
    "thiss",
    "source",
    "artifact",
    "creature",
    "permanent",
];

fn non_article_tokens_eq(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    word_slice_eq(&non_article_token_word_refs(tokens), expected)
}

fn non_article_tokens_eq_any(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    word_slice_eq_any(&non_article_token_word_refs(tokens), expected)
}

fn words_contain_all_phrases(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .all(|phrase| word_slice_contains_phrase(words, phrase))
}

pub(crate) fn parse_parameterized_flashback_grant_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let _shape = GenericSubjectVerbSequence::parameterized_flashback_grant();
    let first_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();
    let Some(gain_idx) = first_clause.find_word_any(&["gain", "gains"]) else {
        return Ok(None);
    };
    let Some(flashback_tail) = first_clause.from_word(gain_idx + 1) else {
        return Ok(None);
    };
    if !word_slice_eq(&flashback_tail.word_refs(), FLASHBACK_UNTIL_END_TAIL) {
        return Ok(None);
    }

    let target_clause = first_clause
        .before_word(gain_idx)
        .unwrap_or_else(|| first_clause.before(0))
        .trimmed();
    if target_clause.is_empty() {
        return Ok(None);
    }
    let target = effect_sentences::parse_target_phrase(target_clause.tokens())?;

    let second_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    if !word_slice_eq_any(&second_clause.word_refs(), FLASHBACK_COST_FOLLOWUPS) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::subject_verb_grant_to_target(
        target,
        crate::grant::Grantable::flashback_from_cards_mana_cost(),
        crate::grant::GrantDuration::UntilEndOfTurn,
    )]))
}

pub(crate) fn parse_prefixed_library_consult_hand_exile_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let _shape = GenericSubjectVerbSequence::prefixed_library_consult();
    let Ok(prefix_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
            .or_else(|_| effect_sentences::parse_effect_chain(sentences[sentence_idx].lowered()))
    else {
        return Ok(None);
    };
    if prefix_effects.is_empty() {
        return Ok(None);
    }
    let Some(mut combined) =
        pairs::parse_consult_match_into_hand_exile_others(sentences, sentence_idx + 1)?
    else {
        return Ok(None);
    };
    let mut effects = prefix_effects;
    effects.append(&mut combined);
    Ok(Some(effects))
}

pub(crate) fn parse_iterative_library_procedure_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let _shape = GenericSubjectVerbSequence::iterative_library_procedure();
    let first_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();
    if !non_article_tokens_eq(first_clause.tokens(), EXILE_TOP_CARD_OF_LIBRARY) {
        return Ok(None);
    }

    let second_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    if !non_article_tokens_eq_any(
        second_clause.tokens(),
        ITERATIVE_LIBRARY_MAY_KEEP_UNLESS_DUPLICATE_NAME_CLAUSES,
    ) {
        return Ok(None);
    }

    let third_clause = LexedClause::new(sentences[sentence_idx + 2].lowered()).trimmed();
    if !non_article_tokens_eq(
        third_clause.tokens(),
        ITERATIVE_LIBRARY_REPEAT_UNTIL_KEEP_OR_DUPLICATE,
    ) {
        return Ok(None);
    }

    let current_tag = TagKey::from("iterative_library_current");
    let exiled_tag = TagKey::from("iterative_library_exiled");
    let all_exiled_filter = ObjectFilter::tagged(exiled_tag.clone()).in_zone(Zone::Exile);
    Ok(Some(vec![EffectAst::RepeatProcess {
        effects: vec![
            EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::You,
                Value::Fixed(1),
                vec![current_tag.clone()],
                vec![exiled_tag.clone()],
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::And(
                    Box::new(PredicateAst::TaggedMatches(
                        current_tag.clone(),
                        ObjectFilter::default().in_zone(Zone::Exile),
                    )),
                    Box::new(PredicateAst::ValueComparison {
                        left: Value::Count(all_exiled_filter.clone()),
                        operator: crate::effect::ValueComparisonOperator::Equal,
                        right: Value::DistinctNames(all_exiled_filter),
                    }),
                ),
                if_true: vec![EffectAst::subject_verb_may_move_to_zone(
                    PlayerAst::You,
                    TargetAst::Tagged(current_tag.clone(), None),
                    Zone::Hand,
                )],
                if_false: Vec::new(),
            },
        ],
        continue_effect_index: 1,
        continue_predicate: IfResultPredicate::WasDeclined,
    }]))
}

pub(crate) fn parse_each_player_repeat_pay_life_tokens_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();
    if !non_article_tokens_eq(first_clause.tokens(), STARTING_EACH_PLAYER_MAY_PAY_ANY_LIFE) {
        return Ok(None);
    }

    let second_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    if !non_article_tokens_eq(second_clause.tokens(), REPEAT_UNTIL_NO_ONE_PAYS_LIFE) {
        return Ok(None);
    }

    let third_clause = LexedClause::new(sentences[sentence_idx + 2].lowered()).trimmed();
    if !non_article_tokens_eq(
        third_clause.tokens(),
        EACH_PLAYER_CREATES_RATS_FOR_LIFE_PAID,
    ) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::RepeatProcess {
            effects: vec![EffectAst::ForEachPlayer {
                effects: vec![EffectAst::subject_verb_pay_any_life(PlayerAst::That, 0)],
            }],
            continue_effect_index: 0,
            continue_predicate: IfResultPredicate::Did,
        },
        EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::That,
                SubjectVerbActionAst::CreateTokenWithMods {
                    name: "1/1 black Rat creature".to_string(),
                    count: Value::PendingEffectMetric {
                        source: ironsmith_core::EffectMetricSource::Outcome,
                        metric: ironsmith_core::EffectMetric::Count,
                    },
                    dynamic_power_toughness: None,
                    player: PlayerAst::That,
                    attached_to: None,
                    tapped: false,
                    attacking: false,
                    exile_at_end_of_combat: false,
                    sacrifice_at_end_of_combat: false,
                    sacrifice_at_next_end_step: false,
                    exile_at_next_end_step: false,
                    granted_abilities: Vec::new(),
                },
            )],
        },
    ]))
}

pub(crate) fn parse_each_player_shuffle_reveal_then_put_revealed_types_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();
    let first_words = first_clause.word_refs();
    if !word_slice_starts_with(&first_words, EACH_PLAYER_SHUFFLE_REVEAL_PREFIX)
        || !words_contain_all_phrases(&first_words, EACH_PLAYER_SHUFFLE_REVEAL_REQUIRED_PHRASES)
    {
        return Ok(None);
    }

    let second_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    let second_words = second_clause.word_refs();
    if !word_slice_starts_with(&second_words, EACH_PLAYER_PUT_REVEALED_TYPES_PREFIX)
        || !words_contain_all_phrases(
            &second_words,
            EACH_PLAYER_PUT_REVEALED_TYPES_REQUIRED_PHRASES,
        )
    {
        return Ok(None);
    }

    let Some(revealed_idx) = second_clause.find_phrase_start(&["revealed", "this", "way"]) else {
        return Ok(None);
    };
    let second_tokens = second_clause.tokens();
    let Some(filter_start) = second_clause.token_index_for_word_index(3) else {
        return Ok(None);
    };
    let Some(filter_end) = second_clause.token_index_for_word_index(revealed_idx) else {
        return Ok(None);
    };
    let mut battlefield_filter =
        parse_object_filter_lexed(&second_tokens[filter_start..filter_end], false)?;
    battlefield_filter.zone = None;

    if let Some(same_for_idx) = second_clause.find_phrase_start(&["same", "for"]) {
        let extra_start_word = same_for_idx + 2;
        let extra_end_word = second_clause
            .after_words(extra_start_word)
            .and_then(|tail| tail.find_phrase_start(&["then"]))
            .map(|offset| extra_start_word + offset)
            .unwrap_or(second_clause.word_len());
        if extra_end_word > extra_start_word
            && let Some(extra_start) = second_clause.token_index_for_word_index(extra_start_word)
            && let Some(extra_end) = second_clause.token_index_for_word_index(extra_end_word)
        {
            let extra_filter =
                parse_object_filter_lexed(&second_tokens[extra_start..extra_end], false)?;
            for card_type in extra_filter.card_types {
                crate::slice_primitives::push_unique(&mut battlefield_filter.card_types, card_type);
            }
            for subtype in extra_filter.subtypes {
                crate::slice_primitives::push_unique(&mut battlefield_filter.subtypes, subtype);
            }
        }
    }

    if battlefield_filter.card_types.is_empty() && battlefield_filter.subtypes.is_empty() {
        return Ok(None);
    }

    let revealed_tag = TagKey::from("__each_player_revealed_this_way");
    let mut shuffled_filter = ObjectFilter::permanent_card();
    shuffled_filter.zone = Some(Zone::Battlefield);
    shuffled_filter.owner = Some(PlayerFilter::IteratedPlayer);
    let iterated = TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None);

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_shuffle_objects_into_library(
                PlayerAst::That,
                TargetAst::Object(shuffled_filter, None, None),
            ),
            EffectAst::subject_verb_reveal_top_cards(
                PlayerAst::That,
                Value::PendingEffectMetric {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::Count,
                },
                revealed_tag.clone(),
            ),
            EffectAst::ForEachTagged {
                tag: revealed_tag,
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::ItMatches(battlefield_filter),
                    if_true: vec![EffectAst::subject_verb_move_to_zone(
                        iterated.clone(),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Owner,
                        false,
                        None,
                    )],
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        iterated,
                        Zone::Library,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            },
        ],
    }]))
}

pub(crate) fn parse_damage_prevention_counter_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(first_effect) = first_effects.first() else {
        return Ok(None);
    };
    if first_effects.len() != 1 {
        return Ok(None);
    }

    let (amount, target, duration) = match first_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PreventDamage {
                    amount,
                    target,
                    duration,
                    ..
                },
            ..
        }) => (Some(amount.clone()), target.clone(), duration.clone()),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PreventAllDamageToTarget {
                    target, duration, ..
                },
            ..
        }) => (None, target.clone(), duration.clone()),
        _ => return Ok(None),
    };

    let second_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    let second_words = non_article_token_word_refs(second_clause.tokens());
    if !word_slice_starts_with(&second_words, PREVENTED_DAMAGE_COUNTER_FOLLOWUP_PREFIX)
        || !word_slice_contains_all_words(&second_words, PREVENTED_DAMAGE_COUNTER_FOLLOWUP_WORDS)
    {
        return Ok(None);
    }

    let Some(on_idx) = second_words.iter().position(|word| *word == "on") else {
        return Ok(None);
    };
    let target_words = &second_words[on_idx + 1..];
    let valid_target_tail = matches!(
        target_words,
        ["that", "creature"] | ["it"] | ["that", "permanent"] | ["that", "object"]
    );
    if !valid_target_tail {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_prevent_damage_to_target_put_counters(
            amount,
            target,
            duration,
            CounterType::PlusOnePlusOne,
        ),
    ]))
}

pub(crate) fn parse_damage_prevention_reflect_to_any_target_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(first_effect) = first_effects.first() else {
        return Ok(None);
    };
    if first_effects.len() != 1 {
        return Ok(None);
    }

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PreventDamage {
                amount,
                target,
                duration,
                source_of_your_choice,
                protect_you_and_permanents_you_control,
                ..
            },
        ..
    }) = first_effect
    else {
        return Ok(None);
    };

    let second_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    let second_words = second_clause.word_refs();
    let prefix = ["if", "damage", "is", "prevented", "this", "way"];
    if !second_words.starts_with(&prefix) {
        return Ok(None);
    }
    let Some(deals_idx) = second_words
        .iter()
        .position(|word| matches!(*word, "deal" | "deals"))
    else {
        return Ok(None);
    };
    if deals_idx <= prefix.len() {
        return Ok(None);
    }
    if second_words.get(deals_idx + 1..)
        != Some(&["that", "much", "damage", "to", "any", "target"][..])
    {
        return Ok(None);
    }

    let follow_up = EffectAst::subject_verb_damage(
        Value::EventValue(EventValueSpec::Amount),
        TargetAst::AnyTarget(None),
    );
    Ok(Some(vec![
        EffectAst::subject_verb_prevent_damage_with_options(
            amount.clone(),
            target.clone(),
            duration.clone(),
            *source_of_your_choice,
            *protect_you_and_permanents_you_control,
            vec![follow_up],
        ),
    ]))
}

pub(crate) fn parse_next_damage_prevention_gain_life_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first_effect] = first_effects.as_mut_slice() else {
        return Ok(None);
    };

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PreventNextTimeDamage {
                follow_up_effects, ..
            },
        ..
    }) = first_effect
    else {
        return Ok(None);
    };
    if !follow_up_effects.is_empty() {
        return Ok(None);
    }

    let Ok(second_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let [second_effect] = second_effects.as_slice() else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject:
            SubjectVerbSubjectAst {
                player: PlayerAst::You,
                ..
            },
        action: SubjectVerbActionAst::GainLife { amount },
    }) = second_effect
    else {
        return Ok(None);
    };
    if !matches!(amount, Value::EventValue(EventValueSpec::Amount)) {
        return Ok(None);
    }

    follow_up_effects.push(second_effect.clone());
    Ok(Some(first_effects))
}

const THEY_DONT_UNTAP_DURING_PREFIXES: &[&[&str]] = &[
    &["they", "dont", "untap", "during"],
    &["they", "don't", "untap", "during"],
    &["those", "permanents", "dont", "untap", "during"],
    &["those", "permanents", "don't", "untap", "during"],
];

pub(crate) fn parse_tap_lock_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: crate::cards::builders::SubjectVerbActionAst::TapAll { filter },
            ..
        }),
    ] = first_effects.as_slice()
    else {
        return Ok(None);
    };

    let second_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    let second_tokens = second_clause.tokens();
    let starts_with_supported_pronoun_clause =
        second_clause.starts_with_any(THEY_DONT_UNTAP_DURING_PREFIXES);
    let second_words = second_clause.word_refs();
    let has_source_tapped_duration =
        word_slice_contains_phrase(&second_words, SOURCE_TAPPED_DURATION_PHRASE)
            && word_slice_contains_all_words(&second_words, SOURCE_TAPPED_DURATION_WORDS)
            && word_slice_contains_any_word(&second_words, SOURCE_TAPPED_DURATION_SOURCE_WORDS);
    if !starts_with_supported_pronoun_clause || !has_source_tapped_duration {
        return Ok(None);
    }

    let Some((duration, clause_tokens)) =
        effect_sentences::parse_restriction_duration(&second_tokens)?
    else {
        return Ok(None);
    };
    let valid_untap_clause =
        LexedClause::new(&clause_tokens).starts_with_any(THEY_DONT_UNTAP_DURING_PREFIXES);
    if !valid_untap_clause {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_tap_all(filter.clone()),
        EffectAst::subject_verb_cant(
            crate::effect::Restriction::untap(filter.clone()),
            duration,
            Some(crate::ConditionExpr::SourceIsTapped),
        ),
    ]))
}

pub(crate) fn parse_search_delayed_upkeep_unless_pays_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = effect_sentences::parse_effect_chain(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if first_effects.is_empty() {
        return Ok(None);
    }

    let upkeep_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    let pay_idx = if upkeep_clause.starts_with_any(&[
        &[
            "at",
            "the",
            "beginning",
            "of",
            "your",
            "next",
            "upkeep",
            "pay",
        ],
        &[
            "at",
            "the",
            "beginning",
            "of",
            "the",
            "next",
            "upkeep",
            "pay",
        ],
    ]) {
        7usize
    } else {
        return Ok(None);
    };
    let Some(pay_token_idx) = upkeep_clause.token_index_for_word_index(pay_idx) else {
        return Ok(None);
    };
    let mana_clause = upkeep_clause.from(pay_token_idx + 1).trimmed();
    let mana_tokens = mana_clause.tokens();
    if mana_tokens.is_empty() {
        return Ok(None);
    }

    let mut mana = Vec::<ManaSymbol>::new();
    for token in mana_tokens {
        if let Some(pips) = mana_pips_from_token(&token) {
            mana.extend(pips);
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if let Ok(generic) = word.parse::<u8>() {
            mana.push(ManaSymbol::Generic(generic));
            continue;
        }
        return Ok(None);
    }
    if mana.is_empty() {
        return Ok(None);
    }

    let lose_clause = LexedClause::new(sentences[sentence_idx + 2].lowered()).trimmed();
    let valid_lose_clause = lose_clause.matches_any_words(&[
        &["if", "you", "dont", "you", "lose", "the", "game"],
        &["if", "you", "don't", "you", "lose", "the", "game"],
        &["if", "you", "do", "not", "you", "lose", "the", "game"],
    ]);
    if !valid_lose_clause {
        return Ok(None);
    }

    let mut effects = first_effects;
    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::subject_verb_lose_game(PlayerAst::You)],
            player: PlayerAst::You,
            cost: crate::cost::TotalCost::mana(crate::mana::ManaCost::from_symbols(mana)),
        }],
    });
    Ok(Some(effects))
}
