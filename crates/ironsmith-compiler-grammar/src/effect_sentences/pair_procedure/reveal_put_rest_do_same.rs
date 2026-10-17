//! "<player> reveals the top N cards of their library, may put <a card> from
//! among them onto the battlefield, then puts the rest into their graveyard."
//! optionally followed by "You do the same with the top N cards of your
//! library." and "If two creatures are put onto the battlefield this way,
//! those creatures fight each other." (Guild Feud).
//!
//! The subject player reveals, chooses, and puts its own card (it enters
//! under its owner's control); "the rest" are the revealed cards still in
//! that library. "Do the same" repeats the procedure for the named
//! participant with its own revealed and chosen collections, so the closing
//! fight names each participant's arrival separately.
use super::*;
use crate::cards::builders::{ConditionalEffectAst, ObjectChoiceEffectAst};
use crate::lexer::OwnedLexToken;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};

/// One participant's reading of the opening sentence.
pub(super) struct RevealPutRest {
    player: PlayerAst,
    count: u32,
    filter: ObjectFilter,
    destination: Zone,
}

fn word_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    crate::util::trim_edge_punctuation_tokens(tokens)
        .iter()
        .filter(|token| token.as_word().is_some())
        .cloned()
        .collect()
}

fn subject(words: &[&str]) -> Option<(PlayerAst, bool, usize)> {
    match words {
        ["target", "opponent", ..] => Some((PlayerAst::TargetOpponent, true, 2)),
        ["target", "player", ..] => Some((PlayerAst::Target, true, 2)),
        ["you", ..] => Some((PlayerAst::You, false, 1)),
        _ => None,
    }
}

/// Read "<player> reveals the top N cards of their library, may put <a card>
/// from among them <destination>, then puts the rest into their graveyard."
pub(super) fn read_opening(
    tokens: &[OwnedLexToken],
) -> Result<Option<RevealPutRest>, CardTextError> {
    let tokens = word_tokens(tokens);
    let words = tokens.iter().filter_map(OwnedLexToken::as_word).collect::<Vec<_>>();
    let Some((player, third_person, start)) = subject(&words) else {
        return Ok(None);
    };
    let (reveal, puts, possessive) = if third_person {
        ("reveals", "puts", "their")
    } else {
        ("reveal", "put", "your")
    };
    let rest = &words[start..];
    let [verb, "the", "top", count, "cards", "of", owner, "library", "may", "put", tail @ ..] =
        rest
    else {
        return Ok(None);
    };
    if *verb != reveal || *owner != possessive {
        return Ok(None);
    }
    let Some(count) = crate::util::parse_number_word_u32(count) else {
        return Ok(None);
    };
    let Some(among) = tail
        .windows(3)
        .position(|window| window == ["from", "among", "them"])
    else {
        return Ok(None);
    };
    let filter_words = &tail[..among];
    if !matches!(filter_words.first(), Some(&("a" | "an"))) || filter_words.len() < 2 {
        return Ok(None);
    }
    let after = &tail[among + 3..];
    let (destination, after) = match after {
        ["onto", "the", "battlefield", after @ ..] => (Zone::Battlefield, after),
        ["into", hand_owner, "hand", after @ ..] if *hand_owner == possessive => (Zone::Hand, after),
        _ => return Ok(None),
    };
    let ["then", rest_verb, "the", "rest", "into", graveyard_owner, "graveyard"] = after else {
        return Ok(None);
    };
    if *rest_verb != puts || *graveyard_owner != possessive {
        return Ok(None);
    }
    let filter_start = start + 10;
    let filter_tokens = &tokens[filter_start..filter_start + filter_words.len()];
    let filter = crate::object_filters::parse_object_filter(filter_tokens, false)?;
    Ok(Some(RevealPutRest {
        player,
        count,
        filter,
        destination,
    }))
}

/// "You do the same with the top N cards of your library."
fn read_do_the_same(tokens: &[OwnedLexToken]) -> Option<u32> {
    let tokens = word_tokens(tokens);
    let words = tokens.iter().filter_map(OwnedLexToken::as_word).collect::<Vec<_>>();
    let [
        "you", "do", "the", "same", "with", "the", "top", count, "cards", "of", "your", "library",
    ] = words.as_slice()
    else {
        return None;
    };
    crate::util::parse_number_word_u32(count)
}

fn is_two_arrivals_fight(tokens: &[OwnedLexToken]) -> bool {
    let tokens = word_tokens(tokens);
    let words = tokens.iter().filter_map(OwnedLexToken::as_word).collect::<Vec<_>>();
    matches!(
        words.as_slice(),
        [
            "if", "two", "creatures", "are", "put", "onto", "the", "battlefield", "this", "way",
            "those", "creatures", "fight", "each", "other",
        ]
    )
}

/// One participant's reveal, choice, put, and graveyard remainder; returns
/// the tag naming the card it put.
fn participant_effects(
    reading: &RevealPutRest,
    anchor: &[OwnedLexToken],
    effects: &mut Vec<EffectAst>,
) -> crate::tag::TagRef {
    let revealed = helper_tag_for_tokens(anchor, "reveal_put_rest_revealed");
    let chosen = helper_tag_for_tokens(anchor, "reveal_put_rest_chosen");
    effects.push(EffectAst::subject_verb_reveal_top_cards(
        reading.player,
        Value::Fixed(reading.count as i32),
        revealed.clone(),
    ));
    let mut choice = reading.filter.clone();
    choice.zone = Some(Zone::Library);
    choice.tagged_constraints.push(TaggedObjectConstraint {
        tag: revealed.key.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    effects.push(EffectAst::ObjectChoices(
        ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: choice,
            count: ChoiceCount::up_to(1),
            player: reading.player,
            tag: chosen.clone(),
            zone: Zone::Library,
        },
    ));
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(chosen.clone(), None),
        reading.destination,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::subject_verb_move_all_to_zone(
        TargetAst::Object(
            ObjectFilter::default()
                .in_zone(Zone::Library)
                .match_tagged(revealed.key.clone(), TaggedOpbjectRelation::SameObjectId),
            None,
            None,
        ),
        Zone::Graveyard,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    chosen
}

/// The opening sentence alone ("..., then puts the rest into their
/// graveyard."), read as one participant's procedure.
pub(crate) fn read_single_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(reading) = read_opening(tokens)? else {
        return Ok(None);
    };
    let mut effects = Vec::new();
    participant_effects(&reading, tokens, &mut effects);
    Ok(Some(effects))
}

/// The opening sentence followed by "You do the same with ...", and, when
/// `with_fight`, by "If two creatures are put onto the battlefield this way,
/// those creatures fight each other."
pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    with_fight: bool,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let fight_sentence = if with_fight {
        let Some(third) = sentences.get(sentence_idx + 2) else {
            return Ok(None);
        };
        if !is_two_arrivals_fight(third.lowered()) {
            return Ok(None);
        }
        true
    } else {
        false
    };
    let Some(count) = read_do_the_same(second.lowered()) else {
        return Ok(None);
    };
    let Some(opening) = read_opening(first.lowered())? else {
        return Ok(None);
    };
    if opening.player == PlayerAst::You
        || (fight_sentence && opening.destination != Zone::Battlefield)
    {
        return Ok(None);
    }
    let yours = RevealPutRest {
        player: PlayerAst::You,
        count,
        filter: opening.filter.clone(),
        destination: opening.destination,
    };
    let mut effects = Vec::new();
    let theirs_chosen = participant_effects(&opening, first.lowered(), &mut effects);
    let yours_chosen = participant_effects(&yours, second.lowered(), &mut effects);
    if fight_sentence {
        let arrived = ObjectFilter::creature().in_zone(Zone::Battlefield);
        effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::And(
                Box::new(PredicateAst::TaggedMatches(theirs_chosen.clone(), arrived.clone())),
                Box::new(PredicateAst::TaggedMatches(yours_chosen.clone(), arrived)),
            ),
            if_true: vec![
                EffectAst::subject_verb_fight(
                    TargetAst::Tagged(theirs_chosen, None),
                    TargetAst::Tagged(yours_chosen, None),
                )
                .with_mutual_fight_surface(),
            ],
            if_false: Vec::new(),
        }));
    }
    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_put_rest_do_the_same_then_fight_reads_two_participants() {
        let sentences = [
            "target opponent reveals the top three cards of their library, may put a creature card from among them onto the battlefield, then puts the rest into their graveyard",
            "You do the same with the top three cards of your library",
            "If two creatures are put onto the battlefield this way, those creatures fight each other",
        ]
        .map(|text| crate::lexer::lex_line(text, 0).unwrap());
        let inputs = sentences
            .iter()
            .map(|tokens| SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();
        let effects = read(&inputs, 0, true).unwrap().expect("guild feud procedure");
        assert_eq!(effects.len(), 9, "{effects:?}");
        let pair = read(&inputs, 0, false).unwrap().expect("do-the-same pair");
        assert_eq!(pair.len(), 8, "{pair:?}");
    }
}
