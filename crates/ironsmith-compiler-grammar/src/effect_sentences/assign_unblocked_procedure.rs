//! "You may have creatures you control assign their combat damage this turn as
//! though they weren't blocked." (Predatory Focus)
//!
//! A resolving grant of the unblocked-assignment permission (CR 510.1c) to the
//! creatures you control until end of turn; the affected set is locked in as
//! the spell resolves (CR 611.2c). The choice is still made per creature as
//! combat damage is assigned, by that creature's controller.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst, GrantedAbilityAst, StaticAbilityAst};
use crate::effect::Until;
use crate::model::CompilerStaticAbilityCore as StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

pub(super) struct AssignUnblockedGroup {
    effects: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<AssignUnblockedGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    if !crate::grammar::abilities::is_controlled_creatures_may_assign_this_turn_as_unblocked_lexed(
        sentence.lexed(),
    ) {
        return Ok(None);
    }
    let filter = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    let grant = EffectAst::subject_verb_grant_abilities_all(
        filter,
        vec![GrantedAbilityAst::StaticAbility(Box::new(StaticAbilityAst::Static(
            StaticAbility::may_assign_damage_as_unblocked(),
        )))],
        Until::EndOfTurn,
    );
    Ok(Some(AssignUnblockedGroup {
        effects: vec![grant],
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

pub(super) fn continue_with(
    _group: &mut AssignUnblockedGroup,
    _sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    Ok(false)
}

pub(super) fn finish(group: AssignUnblockedGroup) -> Vec<EffectAst> {
    group.effects
}

/// "X target blocked creatures assign their combat damage this turn as though
/// they weren't blocked." (Outmaneuver): a resolving grant of the mandatory
/// unblocked assignment (CR 510.1c) to the targets until end of turn. The
/// optional "may have ... assign" form stays with [`open`].
pub(crate) fn parse_targeted_mandatory_unblocked_assignment(
    tokens: &[crate::lexer::OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut tokens = tokens;
    while tokens.last().is_some_and(|token| token.is_period()) {
        tokens = &tokens[..tokens.len() - 1];
    }
    let Some(verb) = tokens
        .iter()
        .position(|token| token.is_word("assign") || token.is_word("assigns"))
    else {
        return Ok(None);
    };
    let subject = &tokens[..verb];
    if subject.is_empty()
        || !subject.iter().any(|token| token.is_word("target"))
        || subject
            .iter()
            .any(|token| ["may", "can", "have", "has"].iter().any(|word| token.is_word(word)))
    {
        return Ok(None);
    }
    let mut body = crate::lexer::token_word_refs(&tokens[verb + 1..]);
    let mut duration = Until::Forever;
    if let Some(index) = body
        .windows(2)
        .position(|pair| pair == ["this", "turn"])
    {
        body.drain(index..index + 2);
        duration = Until::EndOfTurn;
    }
    if duration != Until::EndOfTurn
        || !matches!(
            body.as_slice(),
            [
                "their" | "its",
                "combat",
                "damage",
                "as",
                "though",
                "they" | "it",
                "weren't" | "werent" | "wasn't" | "wasnt",
                "blocked",
            ]
        )
    {
        return Ok(None);
    }
    let target = crate::util::parse_target_phrase(subject)?;
    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        target,
        vec![GrantedAbilityAst::StaticAbility(Box::new(StaticAbilityAst::Static(
            StaticAbility::assigns_damage_as_unblocked(),
        )))],
        duration,
    )))
}
