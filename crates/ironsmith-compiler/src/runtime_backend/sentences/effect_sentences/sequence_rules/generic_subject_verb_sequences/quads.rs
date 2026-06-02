use super::super::super::dispatch_entry::{
    is_put_rest_on_bottom_of_library_sentence, parse_counted_looked_cards_into_your_hand_tokens,
    parse_if_this_spell_was_kicked_counted_looked_cards_into_hand,
    parse_if_you_dont_put_card_from_among_them_into_your_hand,
};
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, ObjectFilter, PlayerAst, PredicateAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::effect::ChoiceCount;
use crate::filter::TaggedObjectConstraint;
use crate::mana::{ManaCost, ManaSymbol};
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use crate::runtime_backend::front_end::lexer::{LexedClause, OwnedLexToken};
use crate::runtime_backend::util::{
    helper_tag_for_tokens, mana_pips_from_token, parse_choice_count_token_prefix_consumed,
    parse_target_phrase,
};
use crate::target::TaggedOpbjectRelation;
use crate::zone::Zone;

fn look_at_top_cards_player(effect: &EffectAst) -> Option<PlayerAst> {
    let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
        subject: crate::cards::builders::SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { .. },
    }) = effect
    else {
        return None;
    };
    Some(*player)
}

fn title_case_card_name(words: &[&str]) -> String {
    const LOWERCASE_WORDS: &[&str] = &[
        "a", "an", "the", "and", "or", "but", "nor", "for", "so", "yet", "of", "in", "on", "at",
        "to", "from", "with", "without", "by", "as", "into", "onto", "over", "under",
    ];
    words
        .iter()
        .filter(|word| !word.is_empty())
        .enumerate()
        .map(|(idx, word)| {
            if idx > 0 && LOWERCASE_WORDS.iter().any(|candidate| candidate == word) {
                return (*word).to_string();
            }
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut out = first.to_uppercase().to_string();
            out.push_str(chars.as_str());
            out
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn search_reveal_tag(effects: &[EffectAst]) -> Option<TagKey> {
    let searched_tag = effects.iter().find_map(|effect| match effect {
        EffectAst::ChooseObjects { filter, tag, .. }
        | EffectAst::ChooseObjectsAcrossZones { filter, tag, .. }
            if filter.zone == Some(Zone::Library) =>
        {
            Some(tag.clone())
        }
        _ => None,
    })?;
    effects
        .iter()
        .any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(subject_verb)
                    if matches!(
                        &subject_verb.action,
                        SubjectVerbActionAst::RevealTagged { tag } if tag == &searched_tag
                    )
            )
        })
        .then_some(searched_tag)
}

const NAMED_REVEALED_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix &["if", "you", "reveal"];
    contains_phrases &[&["this", "way"]]
);

const PUTS_LOOKED_CARD_ONTO_BATTLEFIELD_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["put", "it", "onto", "the", "battlefield"],
            &["put", "that", "card", "onto", "the", "battlefield"],
        ]]
);

const OTHERWISE_PUTS_LOOKED_CARD_INTO_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["put", "that", "card", "into", "your", "hand"],
            &["put", "it", "into", "your", "hand"],
        ]
);

const OTHERWISE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["otherwise"]);

const THEN_SHUFFLE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["then", "shuffle"], &["shuffle"]]);

const FOR_EACH_THOSE_CONTROLLER_MAY_PAY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["for", "each", "of", "those"];
    contains_phrases & [&["its", "controller", "may", "pay"], &["or"]]
);

const IF_THAT_PLAYER_DOESNT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any & [
        &["if", "that", "player", "doesnt"],
        &["if", "that", "player", "doesn't"],
        &["if", "that", "player", "does", "not"],
    ]
);

const IF_THAT_PLAYER_PAYS_ONLY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "that", "player", "pays", "only"]);

fn token_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    crate::runtime_backend::token_word_refs(tokens)
}

fn strip_after_comma(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let comma_idx = tokens.iter().position(|token| token.slice == ",")?;
    let tail = &tokens[comma_idx + 1..];
    (!tail.is_empty()).then_some(tail)
}

fn generic_mana_amount(cost: &ManaCost) -> Option<u8> {
    match cost.pips() {
        [pip] => match pip.as_slice() {
            [ManaSymbol::Generic(amount)] => Some(*amount),
            _ => None,
        },
        _ => None,
    }
}

fn parse_two_generic_payment_costs(tokens: &[OwnedLexToken]) -> Option<(ManaCost, ManaCost)> {
    let words = token_words(tokens);
    if !FOR_EACH_THOSE_CONTROLLER_MAY_PAY_PATTERN.matches_words(&words) {
        return None;
    }

    let costs = tokens
        .iter()
        .filter_map(|token| mana_pips_from_token(token).map(ManaCost::from_symbols))
        .collect::<Vec<_>>();
    if costs.len() != 2 {
        return None;
    }

    let low = generic_mana_amount(&costs[0])?;
    let high = generic_mana_amount(&costs[1])?;
    (high > low).then(|| (costs[0].clone(), costs[1].clone()))
}

fn parse_choose_target_objects_effect(tokens: &[OwnedLexToken]) -> Option<(EffectAst, TagKey)> {
    let words = token_words(tokens);
    if words.first().copied() != Some("choose") {
        return None;
    }
    let target = parse_target_phrase(&tokens[1..]).ok()?;
    let tag = TagKey::from("targeted_0");
    Some((EffectAst::subject_verb_target_only(target), tag))
}

pub(crate) fn parse_choose_for_each_controller_may_pay_two_amounts_else_branch(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if sentence_idx + 3 >= sentences.len() {
        return Ok(None);
    }

    let Some((choose_effect, chosen_tag)) =
        parse_choose_target_objects_effect(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };

    let Some((low_cost, high_cost)) =
        parse_two_generic_payment_costs(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };

    let no_pay_words = token_words(sentences[sentence_idx + 2].lowered());
    if !IF_THAT_PLAYER_DOESNT_PREFIX_PATTERN.matches_words(&no_pay_words) {
        return Ok(None);
    }
    let Some(no_pay_tail) = strip_after_comma(sentences[sentence_idx + 2].lowered()) else {
        return Ok(None);
    };
    let no_pay_effects = effect_sentences::parse_effect_sentence_lexed(no_pay_tail)?;

    let low_only_words = token_words(sentences[sentence_idx + 3].lowered());
    if !IF_THAT_PLAYER_PAYS_ONLY_PREFIX_PATTERN.matches_words(&low_only_words) {
        return Ok(None);
    }
    let Some(low_only_tail) = strip_after_comma(sentences[sentence_idx + 3].lowered()) else {
        return Ok(None);
    };
    let low_only_effects = effect_sentences::parse_effect_sentence_lexed(low_only_tail)?;

    let low = generic_mana_amount(&low_cost).expect("already checked generic low payment");
    let high = generic_mana_amount(&high_cost).expect("already checked generic high payment");
    let extra_cost = ManaCost::from_symbols(vec![ManaSymbol::Generic(high - low)]);

    let pay_low = EffectAst::subject_verb_pay_mana(PlayerAst::ItsController, low_cost);
    let pay_extra = EffectAst::subject_verb_pay_mana(PlayerAst::ItsController, extra_cost);
    let pay_only_low_branch = EffectAst::UnlessAction {
        effects: low_only_effects,
        alternative: vec![pay_extra],
        player: PlayerAst::ItsController,
    };
    let payment_branch = EffectAst::UnlessAction {
        effects: no_pay_effects,
        alternative: vec![pay_low, pay_only_low_branch],
        player: PlayerAst::ItsController,
    };

    let mut effects = vec![choose_effect];
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag,
        effects: vec![payment_branch],
    });
    Ok(Some(effects))
}

const MAY_REVEAL_FROM_LOOKED_CARDS_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "may", "reveal"]);

const BARGAINED_PUT_REVEALED_CARDS_ONTO_BATTLEFIELD_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["if", "this", "spell", "was", "bargained"];
    contains_phrases & [&["put", "the", "revealed", "cards", "onto", "the", "battlefield"]]
);

const OTHERWISE_PUT_REVEALED_CARDS_INTO_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "otherwise",
            "put",
            "the",
            "revealed",
            "cards",
            "into",
            "your",
            "hand",
        ]
);

fn named_revealed_card_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let clause = LexedClause::new(tokens);
    if !NAMED_REVEALED_THIS_WAY_PATTERN.matches(clause) {
        return None;
    }
    let words = clause.word_refs();
    let named_idx = clause.find_word("named")?;
    let this_way_idx = clause.find_phrase_start(&["this", "way"])?;
    if named_idx + 1 >= this_way_idx {
        return None;
    }
    let mut filter = ObjectFilter::default();
    filter.name = Some(title_case_card_name(&words[named_idx + 1..this_way_idx]));
    Some(filter)
}

fn puts_it_onto_battlefield(tokens: &[OwnedLexToken]) -> bool {
    let clause = LexedClause::new(tokens);
    PUTS_LOOKED_CARD_ONTO_BATTLEFIELD_PATTERN.matches(clause)
}

fn otherwise_puts_that_card_into_hand(tokens: &[OwnedLexToken]) -> bool {
    let mut clause = LexedClause::new(tokens).trimmed();
    if clause
        .word_refs()
        .first()
        .is_some_and(|word| OTHERWISE_WORD_PATTERN.matches_word(word))
    {
        clause = clause.from(1).trimmed();
    }
    OTHERWISE_PUTS_LOOKED_CARD_INTO_HAND_PATTERN.matches(clause)
}

fn then_shuffle(tokens: &[OwnedLexToken]) -> bool {
    let clause = LexedClause::new(tokens).trimmed();
    THEN_SHUFFLE_PATTERN.matches(clause)
}

fn parse_may_reveal_up_to_from_looked_cards(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    if !MAY_REVEAL_FROM_LOOKED_CARDS_PATTERN.matches(clause) {
        return Ok(None);
    }

    let Some(count_start) = clause.token_index_for_word_index(3) else {
        return Ok(None);
    };
    let tokens = clause.tokens();
    let (count, count_used) = parse_choice_count_token_prefix_consumed(&tokens[count_start..])
        .ok_or_else(|| {
            CardTextError::ParseError("unable to parse reveal count from looked cards".to_string())
        })?;
    let filter_start = count_start + count_used;
    let Some((filter_clause, _)) = clause.split_once_on_phrase(&["from", "among", "them"]) else {
        return Ok(None);
    };
    let filter_end = filter_clause.len();
    let mut filter =
        effect_sentences::parse_looked_card_choice_filter(&tokens[filter_start..filter_end])
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to parse reveal filter from looked cards".to_string(),
                )
            })?;
    filter.zone = Some(Zone::Library);

    Ok(Some((filter, count)))
}

pub(crate) fn parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    let Some(player) = look_at_top_cards_player(first_effect) else {
        return Ok(None);
    };

    let Some(base_count) =
        parse_counted_looked_cards_into_your_hand_tokens(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let Some(kicked_count) = parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    if !is_put_rest_on_bottom_of_library_sentence(sentences[sentence_idx + 3].lowered()) {
        return Ok(None);
    }

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::Conditional {
            predicate: crate::cards::builders::PredicateAst::ThisSpellWasKicked,
            if_true: vec![
                EffectAst::subject_verb_put_some_into_hand_rest_on_bottom_of_library(
                    player,
                    kicked_count,
                ),
            ],
            if_false: vec![
                EffectAst::subject_verb_put_some_into_hand_rest_on_bottom_of_library(
                    player, base_count,
                ),
            ],
        },
    ]))
}

pub(crate) fn parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    if look_at_top_cards_player(first_effect).is_none() {
        return Ok(None);
    }

    let Some((chooser, battlefield_filter, tapped)) =
        effect_sentences::parse_may_put_filtered_looked_card_onto_battlefield(
            sentences[sentence_idx + 1].lowered(),
        )?
    else {
        return Ok(None);
    };
    if !parse_if_you_dont_put_card_from_among_them_into_your_hand(
        sentences[sentence_idx + 2].lowered(),
    ) {
        return Ok(None);
    }
    if !is_put_rest_on_bottom_of_library_sentence(sentences[sentence_idx + 3].lowered()) {
        return Ok(None);
    }

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::subject_verb_choose_from_looked_cards_onto_battlefield_or_into_hand_rest_on_bottom_of_library(
            chooser,
            battlefield_filter,
            tapped,
        ),
    ]))
}

pub(crate) fn parse_look_at_top_may_reveal_match_bargain_battlefield_else_hand_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }
    let Some((mut filter, reveal_count)) =
        parse_may_reveal_up_to_from_looked_cards(sentences[sentence_idx + 1].lowered())?
    else {
        return Ok(None);
    };

    let third_clause = LexedClause::new(sentences[sentence_idx + 2].lowered());
    let fourth_clause = LexedClause::new(sentences[sentence_idx + 3].lowered());
    if !BARGAINED_PUT_REVEALED_CARDS_ONTO_BATTLEFIELD_PATTERN.matches(third_clause)
        || !OTHERWISE_PUT_REVEALED_CARDS_INTO_HAND_PATTERN.matches(fourth_clause)
        || !then_shuffle(sentences[sentence_idx + 4].lowered())
    {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let revealed_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "revealed");
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag),
        EffectAst::ChooseObjects {
            filter,
            count: reveal_count,
            count_value: None,
            player,
            tag: revealed_tag.clone(),
        },
        EffectAst::subject_verb_reveal_tagged(revealed_tag.clone()),
        EffectAst::Conditional {
            predicate: PredicateAst::ThisSpellPaidLabel("Bargain".to_string()),
            if_true: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(revealed_tag.clone(), None),
                Zone::Battlefield,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(revealed_tag, None),
                Zone::Hand,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ]))
}

pub(crate) fn parse_search_reveal_named_match_battlefield_else_hand_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(searched_tag) = search_reveal_tag(&effects) else {
        return Ok(None);
    };
    let Some(named_filter) = named_revealed_card_filter(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    if !puts_it_onto_battlefield(sentences[sentence_idx + 1].lowered())
        || !otherwise_puts_that_card_into_hand(sentences[sentence_idx + 2].lowered())
        || !then_shuffle(sentences[sentence_idx + 3].lowered())
    {
        return Ok(None);
    }

    effects.push(EffectAst::Conditional {
        predicate: PredicateAst::TaggedMatches(searched_tag.clone(), named_filter),
        if_true: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(searched_tag.clone(), None),
            Zone::Battlefield,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
        if_false: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(searched_tag, None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::You,
        SubjectVerbActionAst::ShuffleLibrary,
    ));
    Ok(Some(effects))
}
