use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, StaticAbilityAst, TargetAst,
};
use crate::effect::Until;
use crate::grammar::effects::{become_shapes, toughness_assignment as shape};
use crate::lexer::OwnedLexToken;
use crate::object_filters::parse_object_filter;
use crate::static_abilities::StaticAbility;
use crate::util::{parse_target_phrase, span_from_tokens};

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    // Complete actions, not a later predicate of an unsplit shared-subject
    // chain. Ordinary coordination carries the original subject and duration.
    let Some(original_verb) = shape::assignment_verb(tokens) else {
        return Ok(None);
    };
    if tokens[..original_verb].iter().any(|token| {
        [
            "gain", "gains", "get", "gets", "lose", "loses", "can", "may",
        ]
        .iter()
        .any(|word| token.is_word(word))
    }) {
        return Ok(None);
    }
    let (duration, tokens) = super::parse_restriction_duration(tokens)?
        .unwrap_or_else(|| (Until::Forever, tokens.to_vec()));
    let Some(verb) = shape::assignment_verb(&tokens) else {
        return Ok(None);
    };
    let Some(no_defender) = shape::assignment_body(&tokens[verb + 1..]) else {
        return Ok(None);
    };
    let subject = &tokens[..verb];
    let mut abilities = vec![GrantedAbilityAst::StaticAbility(Box::new(
        StaticAbilityAst::Static(
            crate::model::CompilerStaticAbilityCore::this_creature_assigns_combat_damage_using_toughness(),
        ),
    ))];
    if no_defender {
        abilities.push(GrantedAbilityAst::CanAttackAsThoughNoDefender);
    }
    let target =
        match become_shapes::parse_become_target_subject_shape(subject, &tokens[verb + 1..]) {
            become_shapes::BecomeTargetSubjectShape::FilteredMany(filter) => {
                // A resolving continuous grant locks this recipient set now. It
                // must not start affecting later arrivals or drop creatures whose
                // own P/T relation subsequently changes during the same turn.
                return Ok(Some(EffectAst::subject_verb_grant_abilities_all(
                    parse_object_filter(filter, false)?,
                    abilities,
                    duration,
                )));
            }
            become_shapes::BecomeTargetSubjectShape::Source(_) => {
                TargetAst::Source(span_from_tokens(subject))
            }
            become_shapes::BecomeTargetSubjectShape::Tagged => TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                span_from_tokens(subject),
            ),
            become_shapes::BecomeTargetSubjectShape::Parsed(subject) => {
                parse_target_phrase(subject)?
            }
            become_shapes::BecomeTargetSubjectShape::Mass(_) => return Ok(None),
        };
    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        target, abilities, duration,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{GrantActionAst, SubjectVerbActionAst};
    #[test]
    fn temporary_mass_assignment_locks_its_set_but_keeps_live_damage_stat() {
        let tokens = crate::lexer::lex_line("Until end of turn, creatures you control with toughness greater than their power assign combat damage equal to their toughness rather than their power.", 0).unwrap();
        let effect = parse(&tokens).unwrap().unwrap();
        let EffectAst::SubjectVerb(subject) = effect else {
            panic!("expected grant");
        };
        let SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
            filter,
            duration,
            lock_filter_at_resolution,
            ..
        }) = subject.action
        else {
            panic!("expected mass grant");
        };
        assert!(lock_filter_at_resolution);
        assert_eq!(duration, Until::EndOfTurn);
        assert_eq!(
            filter.power_toughness_relation,
            Some(ironsmith_core::PowerToughnessRelation::ToughnessGreaterThanPower)
        );
    }
    #[test]
    fn targeted_toughness_assignment_keeps_its_target_and_does_not_make_the_choice_optional() {
        let tokens = crate::lexer::lex_line("Target creature you control assigns combat damage equal to its toughness rather than its power this turn.", 0).unwrap();
        let effect = parse(&tokens).unwrap().unwrap();
        let EffectAst::SubjectVerb(subject) = effect else {
            panic!("expected grant");
        };
        let SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
            target,
            duration,
            abilities,
            ..
        }) = subject.action
        else {
            panic!("expected target grant");
        };
        assert_eq!(duration, Until::EndOfTurn);
        assert!(matches!(target, TargetAst::Object(_, Some(_), _)));
        assert_eq!(abilities.len(), 1);
        assert!(parse(&crate::lexer::lex_line("Target creature you control may assign combat damage equal to its toughness rather than its power this turn.",0).unwrap()).unwrap().is_none());
    }
}
