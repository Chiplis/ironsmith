use super::*;
use crate::effect_sentences::parse_effect_sentence_lexed;
use crate::lexer::lex_line;
use crate::model::ast::{SubjectVerbActionAst, SubjectVerbEffectAst};
use crate::types::CardType;

#[test]
fn plural_return_back_reference_preserves_its_authored_pronoun() {
    let tokens = lex_line("them to the battlefield under their owners' control", 0)
        .expect("lex plural return back-reference");
    let effect = parse_return(&tokens).expect("parse plural return back-reference");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ReturnToBattlefield { target, .. },
        ..
    }) = effect
    else {
        panic!("expected a battlefield return");
    };
    let TargetAst::Object(filter, None, _) = target else {
        panic!("expected a typed plural back-reference");
    };

    assert!(filter.has_plural_pronoun_reference_surface());
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == IT_TAG
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
}

#[test]
fn return_to_hand_can_be_declined_by_target_opponents_life_payment() {
    let tokens = lex_line("it to your hand unless target opponent pays 3 life", 0)
        .expect("lex return-unless clause");
    let effect = parse_return(&tokens).expect("parse return-unless clause");
    let EffectAst::UnlessPays {
        effects,
        player,
        cost,
        before_delayed_step,
    } = effect
    else {
        panic!("expected a return wrapped in UnlessPays: {effect:#?}");
    };

    assert_eq!(player, PlayerAst::TargetOpponent);
    assert!(!before_delayed_step);
    assert!(
        matches!(
            effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ReturnToHand { .. },
                ..
            })]
        ),
        "{effects:#?}"
    );
    assert!(
        matches!(
            cost.costs(),
            [crate::model::CompilerCost::Life(Value::Fixed(3))]
        ),
        "{cost:#?}"
    );
}

#[test]
fn parses_top_graveyard_card_as_a_top_only_return_choice() {
    let tokens = lex_line(
        "the top creature card of your graveyard to the battlefield",
        0,
    )
    .expect("lex return clause");
    let effect = parse_return(&tokens).expect("parse return clause");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ReturnToBattlefield {
                target, top_only, ..
            },
        ..
    }) = effect
    else {
        panic!("expected a singular battlefield return");
    };
    let TargetAst::Object(filter, None, _) = target else {
        panic!("expected an untargeted graveyard object filter");
    };

    assert!(top_only);
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(filter.card_types, [CardType::Creature]);
}

#[test]
fn preserves_explicit_controller_and_source_link_for_exiled_card_returns() {
    let tokens = lex_line("the exiled cards to the battlefield under your control", 0)
        .expect("lex return clause");
    let effect = parse_return(&tokens).expect("parse return clause");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ReturnAllToBattlefield {
                filter, controller, ..
            },
        ..
    }) = effect
    else {
        panic!("expected a bulk battlefield return");
    };

    assert_eq!(controller, ReturnControllerAst::You);
    assert!(
        filter
            .tagged_constraints
            .iter()
            .any(|constraint| { constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG })
    );
}

#[test]
fn source_linked_return_tail_excludes_only_the_current_exile_result() {
    let tokens = lex_line(
        "each other card exiled with this Vehicle to the battlefield under its owner's control",
        0,
    )
    .expect("lex source-linked return clause");
    let effect = parse_return(&tokens).expect("parse source-linked return clause");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::MoveToZone {
                target: TargetAst::Object(filter, None, _),
                zone: Zone::Battlefield,
                all: true,
                exiled_with_source_surface: Some(surface),
                ..
            },
        ..
    }) = effect
    else {
        panic!("expected a source-linked bulk move: {effect:#?}");
    };

    assert_eq!(filter.zone, Some(Zone::Exile));
    assert!(
        !filter.other,
        "`other` is result-relative, not source-relative"
    );
    assert!(filter.card_types.is_empty(), "{filter:#?}");
    assert!(filter.subtypes.is_empty(), "{filter:#?}");
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::cards::builders::IT_TAG
            && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
    }));
    assert_eq!(
        surface.subject,
        ironsmith_core::ExiledWithSourceSubjectSurface::Custom("each other card".to_string())
    );
}

#[test]
fn exchange_target_preserves_joint_negative_owner_and_controller_predicates() {
    let tokens = lex_line(
        "control of this enchantment and target permanent you neither own nor control",
        0,
    )
    .expect("lex heterogeneous exchange clause");
    let effect = parse_exchange(&tokens, None).expect("parse heterogeneous exchange clause");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ExchangeControlHeterogeneous {
                permanent2: TargetAst::Object(filter, Some(_), _),
                ..
            },
        ..
    }) = effect
    else {
        panic!("expected heterogeneous source/target exchange: {effect:#?}");
    };
    assert_eq!(filter.owner, Some(PlayerFilter::NotYou));
    assert_eq!(filter.controller, Some(PlayerFilter::NotYou));
    assert_eq!(
        filter.description(),
        "a permanent you neither own nor control"
    );
}

#[test]
fn exchange_target_preserves_different_controller_set_constraint() {
    let tokens = lex_line(
        "control of two target creatures controlled by different players",
        0,
    )
    .expect("lex homogeneous exchange clause");
    let effect = parse_exchange(&tokens, None).expect("parse homogeneous exchange clause");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ExchangeControl {
            filter, count: 2, ..
        },
        ..
    }) = effect
    else {
        panic!("expected one counted exchange target set: {effect:#?}");
    };
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert!(filter.target_set_different_controllers, "{filter:#?}");
    assert!(!filter.target_set_same_controller, "{filter:#?}");
}

#[test]
fn preserves_destination_first_surface_on_a_singular_graveyard_target() {
    let tokens = lex_line(
        "to your hand target artifact card in your graveyard with lesser mana value",
        0,
    )
    .expect("lex destination-first return clause");
    let effect = parse_return(&tokens).expect("parse destination-first return clause");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ReturnToHand { target, .. },
        ..
    }) = effect
    else {
        panic!("expected a singular hand return");
    };
    let TargetAst::Object(filter, Some(_), _) = target else {
        panic!("expected a targeted graveyard object filter");
    };

    assert!(filter.has_return_destination_first_surface());
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(filter.card_types, [CardType::Artifact]);
}

#[test]
fn destination_first_return_preserves_branch_scoped_collection() {
    let tokens = lex_line(
            "to your hand all enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control",
            0,
        )
        .expect("lex destination-first branch-scoped return clause");
    let effect = parse_return(&tokens).expect("parse branch-scoped return clause");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ReturnAllToHand { filter, .. },
        ..
    }) = effect
    else {
        panic!("expected a bulk hand return");
    };

    assert_eq!(filter.owner, Some(PlayerFilter::You), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
    assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
    assert!(filter.has_return_destination_first_surface(), "{filter:#?}");
}

#[test]
fn full_return_sentence_preserves_branch_scoped_collection() {
    let tokens = lex_line(
            "Return to your hand all enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control.",
            0,
        )
        .expect("lex full branch-scoped return sentence");
    let effects =
        parse_effect_sentence_lexed(&tokens).expect("parse full branch-scoped return sentence");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnAllToHand { filter, .. },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one bulk hand return, got {effects:#?}");
    };

    assert_eq!(filter.owner, Some(PlayerFilter::You), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
    assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
}

#[test]
fn each_player_destination_first_return_keeps_graveyard_history() {
    let tokens = lex_line(
            "Each player returns to the battlefield all artifact, creature, enchantment, and land cards in their graveyard that were put there from the battlefield this turn.",
            0,
        )
        .expect("lex each-player historical return sentence");
    let effects =
        parse_effect_sentence_lexed(&tokens).expect("parse each-player historical return");
    let [EffectAst::ForEachPlayer { effects }] = effects.as_slice() else {
        panic!("expected an each-player return, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnAllToBattlefield { filter, .. },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one return-all action, got {effects:#?}");
    };

    assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::IteratedPlayer),
        "{filter:#?}"
    );
    assert_eq!(
        filter.card_types,
        [
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Land,
        ],
        "{filter:#?}"
    );
    assert!(filter.entered_graveyard_this_turn, "{filter:#?}");
    assert!(
        filter.entered_graveyard_from_battlefield_this_turn,
        "{filter:#?}"
    );
    assert!(filter.has_return_destination_first_surface(), "{filter:#?}");
}

#[test]
fn return_for_each_discarded_card_repeats_from_exact_prior_effect() {
    let tokens = lex_line(
        "a card from your graveyard to your hand for each card discarded this way",
        0,
    )
    .expect("lex return-for-each clause");
    let effect = parse_return(&tokens).expect("parse return-for-each clause");
    let EffectAst::RepeatEffects { count, effects } = effect else {
        panic!("expected repeated return effect");
    };
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        count.unhinted(),
        Value::PendingPriorEffectMetric(query)
            if query.action == Some(ironsmith_core::PriorEffectAction::Discarded)
    ));
}
