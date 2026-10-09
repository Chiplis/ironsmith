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
    let Some(parts) = super::super::consult_family::parse_consult_traversal_sentence(first_tokens)?
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
