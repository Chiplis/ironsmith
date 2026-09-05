use super::*;

pub(super) fn parse_controller_sacrifice_consult_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_controller_sacrifice_consult_tokens(tokens)?;
    let revealed_tag = crate::tag::CompilerReferenceTag::ControllerConsultRevealed.bind();
    let matched_tag = crate::tag::CompilerReferenceTag::ControllerConsultMatched.bind();
    let target = TargetAst::Object(shape.target_filter, Some(TextSpan::synthetic()), None);
    let sacrifice = EffectAst::subject_verb_sacrifice(
        PlayerAst::ItsController,
        ObjectFilter::default(),
        1,
        Some(target),
    );
    let mut match_filter = shape.match_filter;

    if shape.conditional_on_sacrifice {
        let sacrificed_tag = helper_tag_for_tokens(tokens, "sacrificed");
        for constraint in &mut match_filter.tagged_constraints {
            if constraint.relation == TaggedOpbjectRelation::SharesCardType {
                constraint.tag = sacrificed_tag.clone();
            }
        }
        match_filter.tagged_constraints.dedup();
        let followups = vec![
            EffectAst::subject_verb_consult_top_of_library(
                PlayerAst::That,
                LibraryConsultModeAst::Reveal,
                match_filter,
                LibraryConsultStopRuleAst::FirstMatch,
                revealed_tag,
                matched_tag.clone(),
            ),
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(matched_tag, None),
                shape.destination,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::ItsController,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
        ];
        return Some(vec![
            EffectAst::TagAffected {
                effect: Box::new(sacrifice),
                tag: sacrificed_tag,
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: followups,
            },
        ]);
    }

    Some(vec![
        sacrifice,
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::That,
            LibraryConsultModeAst::Reveal,
            match_filter,
            LibraryConsultStopRuleAst::FirstMatch,
            revealed_tag,
            matched_tag.clone(),
        ),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(matched_tag, None),
            shape.destination,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::ItsController,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ])
}

pub(super) fn parse_energy_pay_any_destroy_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_energy_pay_any_destroy_tokens(tokens)?;
    Some(vec![
        EffectAst::subject_verb_energy_counters(PlayerAst::You, shape.energy),
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects: vec![EffectAst::subject_verb_pay_any_energy(
                PlayerAst::You,
                shape.minimum_payment,
            )],
        },
        EffectAst::subject_verb_destroy_all(shape.filter),
    ])
}

pub(super) fn parse_bid_life_for_control_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_life_bid_shape(tokens)?;
    let target = crate::grammar::primitives::probe_shape(parse_target_phrase(shape.target))?;

    Some(vec![EffectAst::BidLife {
        target: target.clone(),
        starting_bid: 0,
        winner_effects: vec![EffectAst::subject_verb_gain_control(
            PlayerAst::Implicit,
            target,
            crate::effect::Until::Forever,
        )],
    }])
}
