//! Source-independent structural normalization for compiler effects.
//!
//! Recognition, reference binding, target selection, and control-flow
//! construction are complete before this pass. Normalization may therefore
//! canonicalize representation only; it must never invent or redirect
//! semantics.

use crate::cards::builders::EffectAst;

/// Return the canonical structural representation of an effect program.
///
/// The transformation is deliberately small and idempotent: nested programs
/// are normalized recursively, associative `Sequence` nodes are flattened,
/// and empty `Sequence` wrappers disappear. Authored coordination and source
/// provenance wrappers remain intact.
pub(crate) fn normalize_effects_ast(effects: &[EffectAst]) -> Vec<EffectAst> {
    let mut normalized = effects.to_vec();
    normalize_effects_vec(&mut normalized);
    normalized
}

fn normalize_effects_vec(effects: &mut Vec<EffectAst>) {
    for effect in effects.iter_mut() {
        crate::model::visit::for_each_nested_effect_vec_mut(effect, true, normalize_effects_vec);
    }

    let mut flattened = Vec::with_capacity(effects.len());
    for effect in std::mem::take(effects) {
        match effect {
            EffectAst::Sequence { effects } => flattened.extend(effects),
            effect => flattened.push(effect),
        }
    }
    *effects = flattened;
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::{CHOSEN_OBJECTS_TAG, IT_TAG, IfResultPredicate};
    use crate::cards::builders::{
        EffectAst, PlayerAst, PredicateAst, SubjectVerbActionAst, TagKey, TargetAst,
    };
    use crate::effect::{ChoiceCount, Until, Value};
    use crate::filter::{
        ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
    };
    use crate::zone::Zone;
    use ironsmith_core::ValueSurfaceHint;

    use super::normalize_effects_ast;

    #[test]
    fn normalize_removes_empty_global_grant_effect() {
        let effects = vec![EffectAst::subject_verb_grant_abilities_all(
            ObjectFilter::default(),
            Vec::new(),
            Until::EndOfTurn,
        )];

        let normalized = normalize_effects_ast(&effects);
        assert!(normalized.is_empty());
    }

    #[test]
    fn normalize_removes_empty_global_grant_effect_inside_wrappers() {
        let effects = vec![EffectAst::May {
            effects: vec![
                EffectAst::subject_verb_grant_abilities_all(
                    ObjectFilter::default(),
                    Vec::new(),
                    Until::EndOfTurn,
                ),
                EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    crate::cards::builders::SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                ),
            ],
        }];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::May { effects } = &normalized[0] else {
            panic!("expected wrapped may effect");
        };
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Draw { .. },
                ..
            })
        ));
    }

    #[test]
    fn normalize_treats_all_players_chosen_subtypes_as_characteristics_not_objects() {
        let choose_type = EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_choose_creature_type(
                PlayerAst::That,
                Vec::new(),
            )],
        };
        let misbound = ObjectFilter::creature()
            .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);

        let normalized =
            normalize_effects_ast(&[choose_type, EffectAst::subject_verb_destroy_all(misbound)]);
        let filter = super::direct_destroy_filter(&normalized[1]).expect("destroy filter");
        assert!(filter.tagged_constraints.is_empty(), "{filter:#?}");
        assert!(filter.excluded_any_chosen_creature_type, "{filter:#?}");
        assert!(filter.has_chosen_type_this_way_surface(), "{filter:#?}");
    }

    #[test]
    fn normalize_keeps_all_players_chosen_object_destroy_procedures_tagged() {
        let choose_object = EffectAst::ForEachPlayer {
            effects: vec![EffectAst::ChooseObjects {
                filter: ObjectFilter::creature(),
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::That,
                tag: TagKey::from(IT_TAG),
            }],
        };
        let chosen = ObjectFilter::creature()
            .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);

        let normalized =
            normalize_effects_ast(&[choose_object, EffectAst::subject_verb_destroy_all(chosen)]);
        let filter = super::direct_destroy_filter(&normalized[1]).expect("destroy filter");
        assert!(!filter.excluded_any_chosen_creature_type, "{filter:#?}");
        assert_eq!(filter.tagged_constraints.len(), 1, "{filter:#?}");
    }

    #[test]
    fn normalize_binds_direct_choice_to_explicit_chosen_set_value() {
        let choose = EffectAst::ChooseObjects {
            filter: ObjectFilter::creature().controlled_by(PlayerFilter::You),
            count: ChoiceCount::exactly(2),
            count_value: None,
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        };
        let chosen_filter = ObjectFilter::creature().match_tagged(
            TagKey::from(CHOSEN_OBJECTS_TAG),
            TaggedOpbjectRelation::IsTaggedObject,
        );
        let difference = Value::absolute_difference(
            Value::GreatestPower(chosen_filter.clone()),
            Value::LeastPower(chosen_filter),
        )
        .with_surface_hint(ValueSurfaceHint::Difference);
        let draw = EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw { count: difference },
        );

        let normalized = normalize_effects_ast(&[choose, draw]);
        let [EffectAst::ChooseObjects { tag, .. }, _] = normalized.as_slice() else {
            panic!("expected choice followed by draw: {normalized:#?}");
        };
        assert_eq!(tag.as_str(), CHOSEN_OBJECTS_TAG);
    }

    #[test]
    fn normalize_correlates_conditional_quantified_choice_with_chosen_set_destroy() {
        let choose = EffectAst::ChooseObjects {
            filter: ObjectFilter::permanent().controlled_by(PlayerFilter::IteratedPlayer),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        };
        let mut chosen_permanents = ObjectFilter::permanent();
        chosen_permanents
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(CHOSEN_OBJECTS_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        let effects = vec![
            EffectAst::Conditional {
                predicate: PredicateAst::ThisSpellWasCastFromZone(Zone::Exile),
                if_true: vec![EffectAst::ForEachOpponent {
                    effects: vec![choose],
                }],
                if_false: Vec::new(),
            },
            EffectAst::subject_verb_destroy(TargetAst::Object(chosen_permanents, None, None)),
        ];

        let normalized = normalize_effects_ast(&effects);
        let [
            EffectAst::Conditional {
                if_true, if_false, ..
            },
        ] = normalized.as_slice()
        else {
            panic!("expected one correlated conditional: {normalized:#?}");
        };
        assert!(if_false.is_empty());
        let [EffectAst::ForEachOpponent { effects }, destroy] = if_true.as_slice() else {
            panic!("expected choice and destroy in the true branch: {if_true:#?}");
        };
        let [EffectAst::ChooseObjects { tag, .. }] = effects.as_slice() else {
            panic!("expected one quantified object choice: {effects:#?}");
        };
        assert_eq!(tag.as_str(), CHOSEN_OBJECTS_TAG);
        assert!(super::direct_destroy_references_chosen_collection(destroy));
    }

    #[test]
    fn normalize_binds_repeated_choices_to_destroy_complement() {
        let choose = EffectAst::ChooseObjects {
            filter: ObjectFilter::creature(),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        };
        let mut complement = ObjectFilter::creature();
        complement.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
        let normalized = normalize_effects_ast(&[
            EffectAst::RepeatEffects {
                count: Value::DistinctPowers(ObjectFilter::creature()),
                effects: vec![choose],
            },
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [EffectAst::RepeatEffects { effects, .. }, destroy] = normalized.as_slice() else {
            panic!("expected repeated choice followed by destroy: {normalized:#?}");
        };
        let [EffectAst::ChooseObjects { tag, .. }] = effects.as_slice() else {
            panic!("expected repeated object choice: {effects:#?}");
        };
        assert_eq!(tag.as_str(), CHOSEN_OBJECTS_TAG);
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == CHOSEN_OBJECTS_TAG
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_unions_direct_and_per_player_choices_before_destroying_others() {
        let choice = || EffectAst::ChooseObjects {
            filter: ObjectFilter::permanent(),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        };
        let mut complement = ObjectFilter::permanent();
        complement.other = true;
        let normalized = normalize_effects_ast(&[
            choice(),
            EffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::NotYou,
                effects: vec![choice()],
            },
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [
            EffectAst::ChooseObjects { tag: direct, .. },
            quantified,
            destroy,
        ] = normalized.as_slice()
        else {
            panic!("expected two choice producers and a destroy: {normalized:#?}");
        };
        let EffectAst::ForEachPlayersFiltered { effects, .. } = quantified else {
            panic!("expected quantified choice: {quantified:#?}");
        };
        let [EffectAst::ChooseObjects { tag: repeated, .. }] = effects.as_slice() else {
            panic!("expected quantified object choice: {effects:#?}");
        };
        assert_eq!(direct.as_str(), CHOSEN_OBJECTS_TAG);
        assert_eq!(repeated.as_str(), CHOSEN_OBJECTS_TAG);
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(!filter.other);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == CHOSEN_OBJECTS_TAG
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_does_not_bind_an_unrelated_destroy_after_a_repeated_choice() {
        let mut unrelated_complement = ObjectFilter::artifact();
        unrelated_complement.other = true;
        let normalized = normalize_effects_ast(&[
            EffectAst::RepeatEffects {
                count: Value::Fixed(2),
                effects: vec![EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature(),
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                }],
            },
            EffectAst::subject_verb_destroy_all(unrelated_complement),
        ]);

        let [EffectAst::RepeatEffects { effects, .. }, destroy] = normalized.as_slice() else {
            panic!("expected unchanged repeated choice: {normalized:#?}");
        };
        let [EffectAst::ChooseObjects { tag, .. }] = effects.as_slice() else {
            panic!("expected object choice: {effects:#?}");
        };
        assert_eq!(tag.as_str(), IT_TAG);
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.other);
        assert!(filter.tagged_constraints.is_empty());
    }

    #[test]
    fn normalize_preserves_custom_choice_collection_tags() {
        let custom_tag = TagKey::from("custom_choice_collection");
        let mut complement = ObjectFilter::creature();
        complement.other = true;
        let normalized = normalize_effects_ast(&[
            EffectAst::RepeatEffects {
                count: Value::Fixed(2),
                effects: vec![EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature(),
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: custom_tag.clone(),
                }],
            },
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [EffectAst::RepeatEffects { effects, .. }, destroy] = normalized.as_slice() else {
            panic!("expected unchanged repeated choice: {normalized:#?}");
        };
        let [EffectAst::ChooseObjects { tag, .. }] = effects.as_slice() else {
            panic!("expected object choice: {effects:#?}");
        };
        assert_eq!(tag, &custom_tag);
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.other);
        assert!(filter.tagged_constraints.is_empty());
    }

    #[test]
    fn normalize_binds_demonstrative_grant_to_draw_counted_set() {
        let counted = ObjectFilter::creature().you_control();
        let draw = EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: Value::Count(counted.clone()),
            },
        );
        let mut grant = EffectAst::subject_verb_grant_abilities_to_target(
            TargetAst::Source(None),
            Vec::new(),
            Until::EndOfTurn,
        );
        let EffectAst::SubjectVerb(grant_subject) = &mut grant else {
            panic!("expected targeted grant");
        };
        let SubjectVerbActionAst::GrantAbilitiesToTarget {
            set_quantifier_surface,
            ..
        } = &mut grant_subject.action
        else {
            panic!("expected targeted grant action");
        };
        *set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Each);

        let normalized = normalize_effects_ast(&[draw, grant]);
        assert!(matches!(
            &normalized[1],
            EffectAst::SubjectVerb(subject)
                if matches!(
                    &subject.action,
                    SubjectVerbActionAst::GrantAbilitiesToTarget {
                        target: TargetAst::Object(filter, _, _),
                        ..
                    } if filter == &counted
                )
        ));
    }

    #[test]
    fn normalize_binds_later_predicate_x_to_typed_where_x_value() {
        let where_x =
            Value::CardsInHand(PlayerFilter::You).with_surface_hint(ValueSurfaceHint::WhereXIs);
        let effects = vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                where_x,
                TagKey::from("looked"),
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::ValueComparison {
                    left: Value::X,
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                },
                if_true: Vec::new(),
                if_false: Vec::new(),
            },
        ];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::Conditional {
            predicate: PredicateAst::ValueComparison { left, .. },
            ..
        } = &normalized[1]
        else {
            panic!("expected typed value comparison");
        };
        assert_eq!(*left, Value::CardsInHand(PlayerFilter::You));
    }

    #[test]
    fn normalize_rewrites_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::May {
                effects: vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    crate::cards::builders::SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                )],
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![
                    EffectAst::subject_verb(
                        crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        crate::cards::builders::SubjectVerbActionAst::GainLife {
                            amount: Value::Fixed(1),
                        },
                    ),
                    EffectAst::RepeatThisProcess,
                ],
            },
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::RepeatProcess {
                continue_effect_index: 0,
                continue_predicate: IfResultPredicate::Did,
                ..
            }]
        ));
    }

    #[test]
    fn normalize_unless_payment_as_the_repeat_continuation_gate() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                SubjectVerbActionAst::FlipCoin,
            ),
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                )],
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                effects: vec![EffectAst::Coordinated {
                    effects: vec![
                        EffectAst::UnlessPays {
                            effects: vec![EffectAst::subject_verb(
                                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                                PlayerAst::You,
                                SubjectVerbActionAst::LoseLife {
                                    amount: Value::Fixed(1),
                                },
                            )],
                            player: PlayerAst::You,
                            cost: crate::cost::TotalCost::mana(
                                crate::mana::ManaCost::from_symbols(vec![
                                    crate::mana::ManaSymbol::Generic(3),
                                ]),
                            ),
                            before_delayed_step: false,
                        },
                        EffectAst::RepeatThisProcess,
                    ],
                    leading_duration: false,
                    result_conjunction: false,
                }],
            },
        ];

        let normalized = normalize_effects_ast(&effects);
        let [
            EffectAst::RepeatProcess {
                effects,
                continue_effect_index,
                continue_predicate,
            },
        ] = normalized.as_slice()
        else {
            panic!("expected one typed repeat process: {normalized:#?}");
        };
        assert_eq!(*continue_effect_index, 2);
        assert_eq!(*continue_predicate, IfResultPredicate::WasDeclined);
        assert!(matches!(
            effects.as_slice(),
            [
                EffectAst::SubjectVerb(_),
                EffectAst::IfResult { .. },
                EffectAst::IfResult {
                    effects: loss_effects,
                    ..
                }
            ] if matches!(
                loss_effects.as_slice(),
                [EffectAst::Coordinated {
                    effects,
                    ..
                }] if matches!(effects.as_slice(), [EffectAst::UnlessPays { .. }])
            )
        ));
    }

    #[test]
    fn normalize_removes_empty_clash_result_marker_from_repeat_body() {
        let effects = vec![
            EffectAst::subject_verb_clash(crate::cards::builders::ClashOpponentAst::Opponent),
            EffectAst::IfResult {
                predicate: IfResultPredicate::WonClash,
                effects: vec![EffectAst::RepeatThisProcess],
            },
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::RepeatProcess {
                effects,
                continue_effect_index: 0,
                continue_predicate: IfResultPredicate::WonClash,
            }] if effects.len() == 1
        ));
    }

    #[test]
    fn normalize_rewrites_optional_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::Draw {
                    count: Value::Fixed(1),
                },
            ),
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::LoseLife {
                    amount: Value::Fixed(1),
                },
            ),
            EffectAst::RepeatThisProcessMay,
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::RepeatProcess {
                continue_effect_index: 2,
                continue_predicate: IfResultPredicate::Did,
                ..
            }]
        ));
    }

    #[test]
    fn normalize_preserves_repeat_this_process_once_as_a_typed_repeat() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::Draw {
                    count: Value::Fixed(1),
                },
            ),
            EffectAst::RepeatThisProcessOnce,
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::RepeatEffects { count, effects }]
                if count.unhinted() == &Value::Fixed(2)
                    && count.has_surface_hint(ValueSurfaceHint::RepeatThisProcessOnce)
                    && effects.len() == 1
        ));
    }

    fn tagged_target_pool(tag: &str) -> EffectAst {
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::Actor,
                PlayerAst::You,
                SubjectVerbActionAst::TargetOnly {
                    target: TargetAst::WithCount(
                        Box::new(TargetAst::Object(
                            ObjectFilter::creature().in_zone(Zone::Graveyard),
                            Some(crate::cards::TextSpan::synthetic()),
                            None,
                        )),
                        ChoiceCount::up_to(2),
                    ),
                    explicit_declaration: true,
                },
            )),
            tag: TagKey::from(tag),
        }
    }

    fn opponent_subset_choice() -> EffectAst {
        EffectAst::ChooseObjects {
            filter: ObjectFilter::tagged(IT_TAG),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: TagKey::from(IT_TAG),
        }
    }

    #[test]
    fn normalize_binds_other_to_prior_pool_minus_delegated_subset() {
        let normalized = normalize_effects_ast(&[
            tagged_target_pool("target_pool"),
            opponent_subset_choice(),
            EffectAst::subject_verb_return_to_hand(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                false,
            ),
            EffectAst::subject_verb_return_to_battlefield(
                TargetAst::AnyOtherTarget(None),
                false,
                false,
                false,
                crate::cards::builders::ReturnControllerAst::You,
                None,
            ),
        ]);

        let EffectAst::ChooseObjects { filter, tag, .. } = &normalized[1] else {
            panic!("expected delegated subset choice");
        };
        assert_eq!(tag.as_str(), "target_pool__delegated_subset");
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "target_pool"
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        let EffectAst::SubjectVerb(subject_verb) = &normalized[3] else {
            panic!("expected complement return");
        };
        let SubjectVerbActionAst::ReturnToBattlefield { target, .. } = &subject_verb.action else {
            panic!("expected return-to-battlefield action");
        };
        let TargetAst::Object(filter, ..) = target else {
            panic!("the other must lower as an exact set difference: {target:#?}");
        };
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "target_pool"
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "target_pool__delegated_subset"
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_keeps_conditional_remainder_inside_subset_branch() {
        let conditional = EffectAst::Conditional {
            predicate: PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter: ObjectFilter::creature(),
            },
            if_true: Vec::new(),
            if_false: vec![opponent_subset_choice()],
        };
        let rest_to_hand = EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::MoveToZone {
                target: TargetAst::Tagged(TagKey::from("rest"), None),
                source_top_only: false,
                zone: Zone::Hand,
                to_top: false,
                library_order: None,
                library_order_chooser: PlayerAst::Implicit,
                verb_surface: ironsmith_core::MoveToZoneVerbSurface::Canonical,
                target_plural_surface: true,
                target_reference_surface: None,
                destination_player_surface: None,
                destination_player_reference_surface: None,
                exiled_with_source_surface: None,
                battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                battlefield_attacking: false,
                battlefield_attack_target_player_or_planeswalker_controlled_by: None,
                battlefield_face_down: false,
                battlefield_transformed: false,
                attached_to: None,
                all: false,
            },
        );

        let normalized = normalize_effects_ast(&[
            tagged_target_pool("conditional_pool"),
            conditional,
            rest_to_hand,
        ]);
        assert_eq!(
            normalized.len(),
            2,
            "remainder must not run after true branch"
        );
        let EffectAst::Conditional { if_false, .. } = &normalized[1] else {
            panic!("expected conditional");
        };
        assert!(matches!(
            if_false.as_slice(),
            [
                EffectAst::ChooseObjects { tag, .. },
                EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::PutTaggedRemainderInZone {
                        tag: pool,
                        keep_tagged,
                        zone: Zone::Hand,
                        ..
                    },
                    ..
                })
            ] if tag.as_str() == "conditional_pool__delegated_subset"
                && pool.as_str() == "conditional_pool"
                && keep_tagged == tag
        ));
    }

    #[test]
    fn normalize_binds_next_turn_permission_to_exile_inside_delayed_trigger() {
        let exiled_tag = TagKey::from("delayed_exiled_cards");
        let delayed = EffectAst::DelayedTriggerForDuration {
            trigger: crate::cards::builders::TriggerSpec::Dies(ObjectFilter::creature()),
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::You,
                Value::Fixed(1),
                vec![exiled_tag.clone()],
                Vec::new(),
            )],
            one_shot: false,
            duration: Until::EndOfTurn,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: None,
        };
        let grant = EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
            TagKey::from(IT_TAG),
            PlayerAst::You,
            true,
            false,
        );

        let normalized = normalize_effects_ast(&[delayed, grant]);
        assert!(matches!(
            normalized.get(1),
            Some(EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. },
                ..
            })) if tag == &exiled_tag
        ));
    }

    #[test]
    fn normalize_keeps_explicit_next_turn_permission_tag() {
        let delayed = EffectAst::DelayedTriggerForDuration {
            trigger: crate::cards::builders::TriggerSpec::Dies(ObjectFilter::creature()),
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::You,
                Value::Fixed(1),
                vec![TagKey::from("delayed_exiled_cards")],
                Vec::new(),
            )],
            one_shot: false,
            duration: Until::EndOfTurn,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: None,
        };
        let explicit_tag = TagKey::from("explicit_permission_pool");
        let grant = EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
            explicit_tag.clone(),
            PlayerAst::You,
            true,
            false,
        );

        let normalized = normalize_effects_ast(&[delayed, grant]);
        assert!(matches!(
            normalized.get(1),
            Some(EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. },
                ..
            })) if tag == &explicit_tag
        ));
    }
}
