//! "Each opponent exiles cards from the top of their library until ..."
//! followed by a statement over every opponent's exiled cards:
//! - "You may cast any number of spells from among those nonland cards
//!   without paying their mana costs." (Fevered Suspicion): a free-cast
//!   choice among the cards that stopped each opponent's traversal;
//! - "Until end of turn, you may cast cards exiled this way without paying
//!   their mana costs." (Dream Harvest): a free-cast permission over every
//!   card exiled this way.
//! The traversal runs once per opponent (CR 101.4); its exposed and matched
//! collections aggregate across the loop, so the statement sees all of them.
use super::*;
use crate::cards::builders::{ForEachEffectAst, ObjectChoiceEffectAst};
use crate::zone::Zone;

pub(super) fn read(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1)) else {
        return Ok(None);
    };
    let first_tokens = crate::lexer::trim_lexed_commas(first.lowered());
    if !super::super::consult_family::consult_subject_is_each_opponent(first_tokens) {
        return Ok(None);
    }
    let Some(parts) = super::super::consult_family::parse_consult_traversal_sentence(first_tokens)?
    else {
        return Ok(None);
    };
    let words = crate::lexer::parser_token_word_refs(second.lowered());
    let statement = match words.as_slice() {
        [
            "you", "may", "cast", "any", "number", "of", "spells", "from", "among", "those",
            "nonland", "cards", "without", "paying", "their", "mana", "costs",
        ] => {
            let chosen = crate::util::helper_tag_for_tokens(second.lowered(), "cast_from_consulted");
            vec![
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                    filter: ObjectFilter::tagged(parts.match_tag.clone()).in_zone(Zone::Exile),
                    count: ChoiceCount::any_number(),
                    player: PlayerAst::You,
                    tag: crate::tag::TagRef::of(chosen.clone()),
                    zone: Zone::Exile,
                }),
                EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                    tag: crate::tag::TagRef::of(chosen),
                    effects: vec![EffectAst::subject_verb_cast_tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        PlayerAst::You,
                        false,
                        false,
                        true,
                        None,
                    )],
                }),
            ]
        }
        [
            "until", "end", "of", "turn", "you", "may", "cast", "cards", "exiled", "this", "way",
            "without", "paying", "their", "mana", "costs",
        ] => vec![EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
            crate::tag::TagRef::of(parts.all_tag.clone()),
            PlayerAst::You,
            false,
            true,
            ironsmith_core::value_model::ManaSpendMode::Normal,
        )],
        _ => return Ok(None),
    };
    let mut effects = super::super::consult_family::wrap_each_opponent_consult(parts.effects);
    effects.extend(statement);
    Ok(Some(effects))
}

/// "Each player exiles cards from the top of their library until they exile a
/// nonland card. An opponent chooses a nonland card exiled this way. You may
/// cast up to two spells from among the other cards exiled this way without
/// paying their mana costs." (Plargg and Nassari): one traversal per player,
/// an opponent's exclusion choice among the stopping cards, then free casts
/// among the rest of the exiled cards.
pub(super) fn read_each_player_opponent_excludes(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second), Some(third)) = (
        sentences.get(index),
        sentences.get(index + 1),
        sentences.get(index + 2),
    ) else {
        return Ok(None);
    };
    let first_tokens = crate::lexer::trim_lexed_commas(first.lowered());
    if !super::super::consult_family::consult_subject_is(first_tokens, &["each", "player"]) {
        return Ok(None);
    }
    let Some(parts) = super::super::consult_family::parse_consult_traversal_sentence_with_player(first_tokens, Some(PlayerAst::That))?
    else {
        return Ok(None);
    };
    if !matches!(
        crate::lexer::parser_token_word_refs(second.lowered()).as_slice(),
        ["an", "opponent", "chooses", "a", "nonland", "card", "exiled", "this", "way"]
    ) {
        return Ok(None);
    }
    let count = match crate::lexer::parser_token_word_refs(third.lowered()).as_slice() {
        [
            "you", "may", "cast", "up", "to", count, "spells", "from", "among", "the", "other",
            "cards", "exiled", "this", "way", "without", "paying", "their", "mana", "costs",
        ] => match *count {
            "one" => 1,
            "two" => 2,
            "three" => 3,
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    let excluded = crate::util::helper_tag_for_tokens(second.lowered(), "opponent_excluded");
    let chosen = crate::util::helper_tag_for_tokens(third.lowered(), "cast_from_others");
    let mut castable = ObjectFilter::tagged(parts.all_tag.clone()).in_zone(Zone::Exile);
    castable.excluded_card_types.push(crate::types::CardType::Land);
    castable
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: excluded.key.clone(),
            relation: crate::target::TaggedOpbjectRelation::IsNotTaggedObject,
        });
    let mut effects = super::super::consult_family::wrap_each_player_consult(parts.effects);
    effects.extend([
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: ObjectFilter::tagged(parts.match_tag.clone()).in_zone(Zone::Exile),
            count: ChoiceCount::exactly(1),
            player: PlayerAst::Opponent,
            tag: crate::tag::TagRef::of(excluded),
            zone: Zone::Exile,
        }),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: castable,
            count: ChoiceCount::up_to(count),
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(chosen.clone()),
            zone: Zone::Exile,
        }),
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::TagRef::of(chosen),
            effects: vec![EffectAst::subject_verb_cast_tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                PlayerAst::You,
                false,
                false,
                true,
                None,
            )],
        }),
    ]);
    Ok(Some(effects))
}

/// "Exile cards from the top of your library until you exile two nonland
/// cards with mana value 4 or less. You may cast one of those two cards
/// without paying its mana cost. Put one of them into your hand. Then put the
/// other cards exiled this way on the bottom of your library in a random
/// order." (Invasion of Alara): the cast chooses up to one of the matched
/// cards; the hand pick then chooses among the matched cards still in exile
/// (the other one when a card was cast, CR 608.2c); every remaining exiled
/// card goes to the bottom.
pub(super) fn read_cast_one_then_hand_one(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(cast), Some(hand), Some(rest)) = (
        sentences.get(index),
        sentences.get(index + 1),
        sentences.get(index + 2),
        sentences.get(index + 3),
    ) else {
        return Ok(None);
    };
    let words = |sentence: &SentenceInput| {
        crate::lexer::parser_token_word_refs(sentence.lowered())
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let cast_words = words(cast);
    let cast_words = cast_words.iter().map(String::as_str).collect::<Vec<_>>();
    if !matches!(
        cast_words.as_slice(),
        ["you", "may", "cast", "one", "of", "those", "two", "cards" | "spells", "without", "paying", "its", "mana", "cost"]
            | ["you", "may", "cast", "one", "of", "those", "cards", "without", "paying", "its", "mana", "cost"]
            | ["you", "may", "cast", "one", "of", "them", "without", "paying", "its", "mana", "cost"]
    ) {
        return Ok(None);
    }
    let hand_words = words(hand);
    let hand_words = hand_words.iter().map(String::as_str).collect::<Vec<_>>();
    if !matches!(
        hand_words.as_slice(),
        ["put", "one", "of", "them", "into", "your", "hand"]
            | ["put", "one", "of", "those", "cards", "into", "your", "hand"]
    ) {
        return Ok(None);
    }
    let rest_words = words(rest);
    let rest_words = rest_words.iter().map(String::as_str).collect::<Vec<_>>();
    let rest_words = rest_words.strip_prefix(&["then"]).unwrap_or(&rest_words);
    let order = match rest_words {
        [
            "put", "the", "other" | "rest" | "remaining", "cards", "exiled", "this", "way", "on",
            "the", "bottom", "of", "your", "library", "in", "a", "random", "order",
        ] => crate::cards::builders::LibraryBottomOrderAst::Random,
        [
            "put", "the", "other" | "rest" | "remaining", "cards", "exiled", "this", "way", "on",
            "the", "bottom", "of", "your", "library", "in", "any", "order",
        ] => crate::cards::builders::LibraryBottomOrderAst::ChooserChooses,
        _ => return Ok(None),
    };
    let first_tokens = crate::lexer::trim_lexed_commas(first.lowered());
    let Some(parts) = super::super::consult_family::parse_consult_traversal_sentence(first_tokens)?
    else {
        return Ok(None);
    };
    if parts.player != PlayerAst::You {
        return Ok(None);
    }
    let chosen = crate::util::helper_tag_for_tokens(cast.lowered(), "cast_from_consulted");
    let kept = crate::util::helper_tag_for_tokens(hand.lowered(), "kept_from_consulted");
    let mut effects = parts.effects;
    effects.extend([
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: ObjectFilter::tagged(parts.match_tag.clone()).in_zone(Zone::Exile),
            count: ChoiceCount::up_to(1),
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(chosen.clone()),
            zone: Zone::Exile,
        }),
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::TagRef::of(chosen),
            effects: vec![EffectAst::subject_verb_cast_tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                PlayerAst::You,
                false,
                false,
                true,
                None,
            )],
        }),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: ObjectFilter::tagged(parts.match_tag.clone()).in_zone(Zone::Exile),
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(kept.clone()),
            zone: Zone::Exile,
        }),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::TagRef::of(kept), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            crate::tag::TagRef::of(parts.all_tag.clone()),
            None,
            order,
            PlayerAst::You,
        ),
    ]);
    Ok(Some(effects))
}
