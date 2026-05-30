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
    LexedClause, word_slice_contains_all_words, word_slice_eq, word_slice_eq_any,
    word_slice_find_word, word_slice_starts_with,
};
use crate::runtime_backend::object_filters::parse_object_filter_lexed;
use crate::runtime_backend::util::{
    mana_pips_from_token, non_article_token_word_refs, trim_commas,
};
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

pub(crate) fn parse_parameterized_flashback_grant_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let _shape = GenericSubjectVerbSequence::parameterized_flashback_grant();
    let first_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();
    let Some(gain_idx) = first_clause.find_word_any(&["gain", "gains"]) else {
        return Ok(None);
    };
    let first_words = first_clause.word_refs();
    if !word_slice_eq(
        &first_words[gain_idx + 1..],
        &["flashback", "until", "end", "of", "turn"],
    ) {
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
    let valid_followup = second_clause.matches_any_words(&[
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
    ]);
    if !valid_followup {
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
    let first_words = non_article_token_word_refs(first_clause.tokens());
    if !word_slice_eq(
        &first_words,
        &["exile", "top", "card", "of", "your", "library"],
    ) {
        return Ok(None);
    }

    let second_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    let second_words = non_article_token_word_refs(second_clause.tokens());
    let second_matches = word_slice_eq_any(
        &second_words,
        &[
            &[
                "you", "may", "put", "that", "card", "into", "your", "hand", "unless", "it", "has",
                "same", "name", "as", "another", "card", "exiled", "this", "way",
            ],
            &[
                "you", "may", "put", "it", "into", "your", "hand", "unless", "it", "has", "same",
                "name", "as", "another", "card", "exiled", "this", "way",
            ],
        ],
    );
    if !second_matches {
        return Ok(None);
    }

    let third_clause = LexedClause::new(sentences[sentence_idx + 2].lowered()).trimmed();
    let third_words = non_article_token_word_refs(third_clause.tokens());
    let third_matches = word_slice_eq(
        &third_words,
        &[
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
        ],
    );
    if !third_matches {
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

pub(crate) fn parse_each_player_shuffle_reveal_then_put_revealed_types_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();
    if !first_clause.starts_with(&["each", "player", "shuffles", "all"])
        || !first_clause.contains_phrase(&["they", "own", "into", "their", "library"])
        || !first_clause.contains_phrase(&[
            "then", "reveals", "that", "many", "cards", "from", "the", "top", "of", "their",
            "library",
        ])
    {
        return Ok(None);
    }

    let second_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    if !second_clause.starts_with(&["each", "player", "puts", "all"])
        || !second_clause.contains_phrase(&[
            "revealed",
            "this",
            "way",
            "onto",
            "the",
            "battlefield",
        ])
        || !second_clause.contains_phrase(&["then", "does", "the", "same", "for"])
        || !second_clause.contains_phrase(&["on", "the", "bottom", "of", "their", "library"])
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
            action: SubjectVerbActionAst::PreventAllDamageToTarget { target, duration },
            ..
        }) => (None, target.clone(), duration.clone()),
        _ => return Ok(None),
    };

    let second_clause = LexedClause::new(sentences[sentence_idx + 1].lowered()).trimmed();
    let second_words = non_article_token_word_refs(second_clause.tokens());
    if !word_slice_starts_with(
        &second_words,
        &["for", "each", "1", "damage", "prevented", "this", "way"],
    ) || !word_slice_contains_all_words(&second_words, &["put", "+1/+1", "counter", "on"])
    {
        return Ok(None);
    }

    let Some(on_idx) = word_slice_find_word(&second_words, "on") else {
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
                follow_up_effects,
                ..
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
        subject: SubjectVerbSubjectAst {
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
    let has_source_tapped_duration = second_clause.contains_phrase(&["for", "as", "long", "as"])
        && second_clause.contains_word("remains")
        && second_clause.contains_word("tapped")
        && second_clause.contains_any_word(&[
            "this",
            "thiss",
            "source",
            "artifact",
            "creature",
            "permanent",
        ]);
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
    let search_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();
    if first_effects.is_empty() || !search_clause.starts_with(&["search", "your", "library"]) {
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
