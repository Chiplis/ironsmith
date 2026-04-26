use super::SentenceInput;

pub(crate) mod pairs;
pub(crate) mod quads;
pub(crate) mod triples;
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, ObjectFilter, PlayerAst, PredicateAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey,
    TargetAst,
};
use crate::effect::Value;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::grammar::primitives as grammar;
use crate::runtime_backend::token_index_for_word_index;
use crate::runtime_backend::token_primitives::{find_index, slice_contains, slice_starts_with};
use crate::runtime_backend::util::{is_article, mana_pips_from_token, trim_commas};
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

    pub(super) fn tainted_pact_style_iteration() -> Self {
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
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_words = crate::runtime_backend::token_word_refs(&first_tokens);
    let Some(gain_idx) = first_words
        .iter()
        .position(|word| matches!(*word, "gain" | "gains"))
    else {
        return Ok(None);
    };
    if first_words[gain_idx + 1..] != ["flashback", "until", "end", "of", "turn"] {
        return Ok(None);
    }

    let Some(gain_token_idx) = token_index_for_word_index(&first_tokens, gain_idx) else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&first_tokens[..gain_token_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = effect_sentences::parse_target_phrase(&target_tokens)?;

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words = crate::runtime_backend::token_word_refs(&second_tokens);
    let valid_followup = second_words.as_slice()
        == [
            "the",
            "flashback",
            "cost",
            "is",
            "equal",
            "to",
            "its",
            "mana",
            "cost",
        ]
        || second_words.as_slice()
            == [
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
            ];
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
    let _shape = GenericSubjectVerbSequence::tainted_pact_style_iteration();
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_words: Vec<&str> = crate::runtime_backend::token_word_refs(&first_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if first_words.as_slice() != ["exile", "top", "card", "of", "your", "library"] {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words: Vec<&str> = crate::runtime_backend::token_word_refs(&second_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let second_matches = second_words.as_slice()
        == [
            "you", "may", "put", "that", "card", "into", "your", "hand", "unless", "it", "has",
            "same", "name", "as", "another", "card", "exiled", "this", "way",
        ]
        || second_words.as_slice()
            == [
                "you", "may", "put", "it", "into", "your", "hand", "unless", "it", "has", "same",
                "name", "as", "another", "card", "exiled", "this", "way",
            ];
    if !second_matches {
        return Ok(None);
    }

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let third_words: Vec<&str> = crate::runtime_backend::token_word_refs(&third_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let third_matches = third_words.as_slice()
        == [
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
    if !third_matches {
        return Ok(None);
    }

    let current_tag = TagKey::from("tainted_pact_current");
    let exiled_tag = TagKey::from("tainted_pact_exiled");
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
                },
            ..
        }) => (Some(amount.clone()), target.clone(), duration.clone()),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PreventAllDamageToTarget { target, duration },
            ..
        }) => (None, target.clone(), duration.clone()),
        _ => return Ok(None),
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words: Vec<&str> = crate::runtime_backend::token_word_refs(&second_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !slice_starts_with(
        &second_words,
        &["for", "each", "1", "damage", "prevented", "this", "way"],
    ) || !slice_contains(&second_words, &"put")
        || !slice_contains(&second_words, &"+1/+1")
        || !slice_contains(&second_words, &"counter")
        || !slice_contains(&second_words, &"on")
    {
        return Ok(None);
    }

    let Some(on_idx) = find_index(&second_words, |word| *word == "on") else {
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

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let starts_with_supported_pronoun_clause =
        grammar::words_match_any_prefix(&second_tokens, THEY_DONT_UNTAP_DURING_PREFIXES).is_some();
    let has_source_tapped_duration =
        grammar::words_find_phrase(&second_tokens, &["for", "as", "long", "as"]).is_some()
            && grammar::contains_word(&second_tokens, "remains")
            && grammar::contains_word(&second_tokens, "tapped")
            && (grammar::contains_word(&second_tokens, "this")
                || grammar::contains_word(&second_tokens, "thiss")
                || grammar::contains_word(&second_tokens, "source")
                || grammar::contains_word(&second_tokens, "artifact")
                || grammar::contains_word(&second_tokens, "creature")
                || grammar::contains_word(&second_tokens, "permanent"));
    if !starts_with_supported_pronoun_clause || !has_source_tapped_duration {
        return Ok(None);
    }

    let Some((duration, clause_tokens)) =
        effect_sentences::parse_restriction_duration(&second_tokens)?
    else {
        return Ok(None);
    };
    let valid_untap_clause =
        grammar::words_match_any_prefix(&clause_tokens, THEY_DONT_UNTAP_DURING_PREFIXES).is_some();
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
    if first_effects.is_empty()
        || grammar::words_match_prefix(
            sentences[sentence_idx].lowered(),
            &["search", "your", "library"],
        )
        .is_none()
    {
        return Ok(None);
    }

    let upkeep_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let pay_idx = if grammar::words_match_prefix(
        &upkeep_tokens,
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
    )
    .is_some()
    {
        7usize
    } else if grammar::words_match_prefix(
        &upkeep_tokens,
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
    )
    .is_some()
    {
        7usize
    } else {
        return Ok(None);
    };
    let Some(pay_token_idx) = token_index_for_word_index(&upkeep_tokens, pay_idx) else {
        return Ok(None);
    };
    let mana_tokens = trim_commas(&upkeep_tokens[pay_token_idx + 1..]);
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

    let lose_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let lose_words = crate::runtime_backend::token_word_refs(&lose_tokens);
    let valid_lose_clause = lose_words == ["if", "you", "dont", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "don't", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "do", "not", "you", "lose", "the", "game"];
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
