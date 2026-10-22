//! "Each player looks at the top five cards of their library and may reveal
//! a land card and/or an instant or sorcery card from among them. Each player
//! puts the cards they revealed this way into their hand and the rest on the
//! bottom of their library in a random order." (Explore the Vastlands)
//!
//! One per-player loop: each player looks at their own top cards, chooses up
//! to one card for each "and/or" branch among them, reveals the chosen cards,
//! puts them into their hand, and puts the rest of the looked-at cards on the
//! bottom of their library. The second sentence only reads the collections
//! the first bound, so both must share the loop iteration.
use super::*;
use crate::cards::builders::{ForEachEffectAst, LibraryBottomOrderAst, ObjectChoiceEffectAst};
use crate::lexer::OwnedLexToken;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};

fn word_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    crate::util::trim_edge_punctuation_tokens(tokens)
        .iter()
        .filter(|token| token.as_word().is_some())
        .cloned()
        .collect()
}

/// "each player looks at the top N cards of their library and may reveal
/// <branch> [and/or <branch>] from among them": the count and the branch
/// filters.
fn read_look_and_reveal(
    tokens: &[OwnedLexToken],
) -> Result<Option<(u32, Vec<ObjectFilter>)>, CardTextError> {
    let tokens = word_tokens(tokens);
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    let [
        "each",
        "player",
        "looks",
        "at",
        "the",
        "top",
        count,
        "cards",
        "of",
        "their",
        "library",
        "and",
        "may",
        "reveal",
        tail @ ..,
    ] = words.as_slice()
    else {
        return Ok(None);
    };
    let Some(count) = crate::util::parse_number_word_u32(count) else {
        return Ok(None);
    };
    let Some(filter_words) = tail.strip_suffix(&["from", "among", "them"][..]) else {
        return Ok(None);
    };
    let filter_len = filter_words.len();
    let filter_start = 14;
    let filter_tokens = &tokens[filter_start..filter_start + filter_len];
    let mut filters = Vec::new();
    for branch in filter_tokens.split(|token| token.is_word("and/or")) {
        let branch_words = branch
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>();
        if !matches!(branch_words.first(), Some(&("a" | "an")))
            || !branch_words.last().is_some_and(|word| *word == "card")
        {
            return Ok(None);
        }
        let mut filter = crate::object_filters::parse_object_filter(branch, false)?;
        filter.zone = None;
        filters.push(filter);
    }
    if filters.is_empty() {
        return Ok(None);
    }
    Ok(Some((count, filters)))
}

/// "each player puts the cards they revealed this way into their hand and
/// the rest on the bottom of their library in a random order".
fn read_revealed_to_hand_rest_bottom(tokens: &[OwnedLexToken]) -> Option<LibraryBottomOrderAst> {
    let tokens = word_tokens(tokens);
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    let [
        "each",
        "player",
        "puts",
        "the",
        "cards",
        "they",
        "revealed",
        "this",
        "way",
        "into",
        "their",
        "hand",
        "and",
        "the",
        "rest",
        "on",
        "the",
        "bottom",
        "of",
        "their",
        "library",
        order @ ..,
    ] = words.as_slice()
    else {
        return None;
    };
    match order {
        ["in", "a", "random", "order"] | ["in", "random", "order"] => {
            Some(LibraryBottomOrderAst::Random)
        }
        ["in", "any", "order"] => Some(LibraryBottomOrderAst::ChooserChooses),
        _ => None,
    }
}

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(look), Some(dispose)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let Some(order) = read_revealed_to_hand_rest_bottom(dispose.lowered()) else {
        return Ok(None);
    };
    let Some((count, filters)) = read_look_and_reveal(look.lowered())? else {
        return Ok(None);
    };
    let looked = helper_tag_for_tokens(look.lowered(), "each_player_looked");
    let chosen = helper_tag_for_tokens(dispose.lowered(), "each_player_revealed");
    let mut effects = vec![EffectAst::subject_verb_player_looks_at_own_top_cards(
        PlayerAst::That,
        Value::Fixed(count as i32),
        looked.clone(),
    )];
    for mut filter in filters {
        filter.zone = Some(Zone::Library);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: looked.key.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: chosen.key.clone(),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
        effects.push(EffectAst::ObjectChoices(
            ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: ChoiceCount::up_to(1),
                player: PlayerAst::That,
                tag: chosen.clone(),
                zone: Zone::Library,
            },
        ));
    }
    effects.push(EffectAst::subject_verb_reveal_tagged(chosen.clone()));
    effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: chosen.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    }));
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked,
            Some(chosen),
            order,
            PlayerAst::That,
        ),
    );
    Ok(Some(vec![EffectAst::ForEach(
        ForEachEffectAst::ForEachPlayer { effects },
    )]))
}
