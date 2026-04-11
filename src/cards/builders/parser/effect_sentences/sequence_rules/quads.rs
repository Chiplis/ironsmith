use crate::cards::builders::parser::effect_sentences;
use crate::cards::builders::parser::effect_sentences::SentenceInput;
use super::super::dispatch_entry::{
    is_put_rest_on_bottom_of_library_sentence,
    parse_counted_looked_cards_into_your_hand_tokens,
    parse_if_no_card_into_hand_this_way_sentence,
    parse_if_this_spell_was_kicked_counted_looked_cards_into_hand,
    parse_if_you_dont_put_card_from_among_them_into_your_hand,
};
use crate::cards::builders::{CardTextError, EffectAst};

pub(super) fn parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { player, .. }] = first_effects.as_slice() else {
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
            if_true: vec![EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: *player,
                count: kicked_count,
            }],
            if_false: vec![EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: *player,
                count: base_count,
            }],
        },
    ]))
}

pub(super) fn parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

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
        EffectAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            player: chooser,
            battlefield_filter,
            tapped,
        },
    ]))
}

pub(super) fn parse_look_at_top_reveal_match_put_rest_bottom_then_if_not_into_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) = super::triples::parse_look_at_top_reveal_match_put_rest_bottom(
        sentences,
        sentence_idx,
    )?
    else {
        return Ok(None);
    };
    let Some(if_not_chosen) =
        parse_if_no_card_into_hand_this_way_sentence(sentences[sentence_idx + 3].lowered())?
    else {
        return Ok(None);
    };

    let Some(EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
        if_not_chosen: existing,
        ..
    }) = effects.get_mut(1)
    else {
        return Ok(None);
    };
    *existing = if_not_chosen;
    Ok(Some(effects))
}
