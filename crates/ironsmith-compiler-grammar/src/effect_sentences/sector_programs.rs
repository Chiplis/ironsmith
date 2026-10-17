//! Space sculptor sector instructions (CR 702.158). A space sculptor divides
//! the battlefield into the alpha, beta and gamma sectors; instructions then
//! refer to "the sector of your choice" (a named-option choice recorded on
//! the source, read back by the `SectorPredicate::ChosenBySource` filter
//! quality) or relate an attacker's sector to its blockers'.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst};
use crate::effect::Until;
use crate::lexer::OwnedLexToken;
use ironsmith_core::filter_model::{SectorName, SectorPredicate};

const SECTOR_CHOICE: [&str; 5] = ["the", "sector", "of", "your", "choice"];

fn words_at(tokens: &[OwnedLexToken], start: usize, words: &[&str]) -> bool {
    tokens.len() >= start + words.len()
        && tokens[start..start + words.len()]
            .iter()
            .zip(words)
            .all(|(token, word)| token.is_word(word))
}

fn sector_choice_position(tokens: &[OwnedLexToken]) -> Option<usize> {
    (0..tokens.len()).find(|&start| words_at(tokens, start, &SECTOR_CHOICE))
}

/// Read the sector instructions this sentence run opens with, if any.
pub(super) fn parse_sector_sentences(
    sentences: &[SentenceInput],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let [sentence] = sentences
        && let Some(effects) = parse_same_sector_block_restriction(sentence.lexed())?
    {
        return Ok(Some(effects));
    }
    parse_sector_of_your_choice(sentences)
}

/// "<instruction> ... in the sector of your choice": choose a sector, then
/// follow the instruction with "the sector" read as the chosen one. The
/// choice is made once, before the instruction, as it resolves.
fn parse_sector_of_your_choice(
    sentences: &[SentenceInput],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !sentences
        .iter()
        .any(|sentence| sector_choice_position(sentence.lexed()).is_some())
    {
        return Ok(None);
    }
    let rewritten = sentences
        .iter()
        .map(|sentence| {
            let mut tokens = sentence.lexed().to_vec();
            while let Some(start) = sector_choice_position(&tokens) {
                // Keep "the sector"; drop "of your choice" so the object
                // filter reads the definite chosen sector.
                tokens.drain(start + 2..start + SECTOR_CHOICE.len());
            }
            SentenceInput::from_lexed(&tokens)
        })
        .collect::<Vec<_>>();
    let mut effects = vec![EffectAst::subject_verb_choose_named_option(
        PlayerAst::You,
        SectorName::ALL
            .iter()
            .map(|sector| sector.word().to_string())
            .collect(),
    )];
    effects.extend(super::dispatch_entry::parse_effect_sentences_from_sentence_inputs(
        rewritten,
    )?);
    Ok(Some(effects))
}

/// "Creatures in each sector can be blocked this turn only by creatures in
/// the same sector": for each sector, its creatures can't be blocked by the
/// blocker noun's objects outside that sector until end of turn.
fn parse_same_sector_block_restriction(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut end = tokens.len();
    while end > 0 && tokens[end - 1].as_word().is_none() {
        end -= 1;
    }
    let tokens = &tokens[..end];
    let Some(each_sector) = (1..tokens.len())
        .find(|&start| words_at(tokens, start, &["in", "each", "sector"]))
    else {
        return Ok(None);
    };
    let restriction_head = [
        "can", "be", "blocked", "this", "turn", "only", "by",
    ];
    let after_subject = each_sector + 3;
    if !words_at(tokens, after_subject, &restriction_head) {
        return Ok(None);
    }
    let blocker_start = after_subject + restriction_head.len();
    let same_sector = ["in", "the", "same", "sector"];
    if tokens.len() < blocker_start + same_sector.len() + 1
        || !words_at(tokens, tokens.len() - same_sector.len(), &same_sector)
    {
        return Ok(None);
    }
    let attacker_noun =
        crate::object_filters::parse_object_filter(&tokens[..each_sector], false)?;
    let blocker_noun = crate::object_filters::parse_object_filter(
        &tokens[blocker_start..tokens.len() - same_sector.len()],
        false,
    )?;
    let effects = SectorName::ALL
        .into_iter()
        .map(|sector| {
            let mut attacker = attacker_noun.clone();
            attacker.sector = Some(SectorPredicate::In(sector));
            let mut blockers = blocker_noun.clone();
            blockers.sector = Some(SectorPredicate::NotIn(sector));
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::block_specific_attacker(blockers, attacker),
                Until::EndOfTurn,
                None,
            )
        })
        .collect();
    Ok(Some(effects))
}
