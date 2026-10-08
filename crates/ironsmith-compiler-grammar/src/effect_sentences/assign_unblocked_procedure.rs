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
