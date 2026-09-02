//! Antecedent-binding tests.
//!
//! The binding itself is AST vocabulary and lives in the semantic crate; these
//! tests stay here because their fixtures are built with the grammar's own
//! token-definition parser.

use ironsmith_compiler_semantic::condition_antecedent::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{
        EffectAst, GrantedAbilityAst, ObjectFilter, PlayerAst, PredicateAst, SubjectVerbActionAst,
        SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TargetAst,
    };
    use crate::effect::{Until, Value};
    use crate::filter::TaggedOpbjectRelation;
    use crate::object::CounterType;

    fn effect(action: SubjectVerbActionAst) -> EffectAst {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst {
                role: SubjectVerbRoleAst::Actor,
                player: PlayerAst::You,
            },
            action,
        })
    }

    fn it_target() -> TargetAst {
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.as_str().into(), None)
    }

    #[test]
    fn direct_tagged_condition_establishes_object_antecedent() {
        let tag: crate::tag::TagKey = "enchanted".into();
        let predicate = PredicateAst::TaggedMatches(tag.clone(), ObjectFilter::creature());

        assert_eq!(
            predicate_object_filter_antecedent(&predicate),
            Some(ObjectFilter::tagged(tag))
        );
    }

    #[test]
    fn existential_condition_does_not_establish_object_antecedent() {
        let predicate = PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: ObjectFilter::creature(),
        };

        assert_eq!(predicate_object_filter_antecedent(&predicate), None);
    }

    #[test]
    fn existential_collection_choice_materializes_one_of_those_objects() {
        let mut contested_lands =
            ObjectFilter::land().with_counter_type(CounterType::Named("contested".into()));
        contested_lands.union_surface = contested_lands
            .union_surface
            .with_counter_requirement_surface(false, true, true);
        let predicate = PredicateAst::PlayerControls {
            player: PlayerAst::That,
            filter: contested_lands,
        };
        let one_of_those = TargetAst::WithCount(
            Box::new(TargetAst::Object(
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key()),
                None,
                None,
            )),
            crate::effect::ChoiceCount::exactly(1),
        );
        let mut effects = vec![
            effect(SubjectVerbActionAst::GainControl {
                target: one_of_those,
                duration: Until::Forever,
                condition: None,
                controller_reference: None,
                source_reference_surface: None,
            }),
            effect(SubjectVerbActionAst::Untap {
                target: it_target(),
            }),
        ];

        bind_condition_collection_antecedent_in_effects(&mut effects, &predicate);

        let [
            EffectAst::Sequence {
                effects: collection_effects,
            },
            EffectAst::SubjectVerb(untap),
        ] = effects.as_slice()
        else {
            panic!("expected an explicit collection choice followed by untap: {effects:#?}");
        };
        let [
            EffectAst::ChooseObjects {
                filter,
                count,
                player,
                tag,
                ..
            },
            EffectAst::SubjectVerb(gain),
        ] = collection_effects.as_slice()
        else {
            panic!("expected choose-then-gain sequence: {collection_effects:#?}");
        };
        assert!(count.is_single());
        assert_eq!(*player, PlayerAst::That);
        assert_eq!(
            tag.as_str(),
            crate::tag::CompilerReferenceTag::ConditionCollectionChoice.as_str()
        );
        assert_eq!(filter.card_types, [crate::types::CardType::Land]);
        assert_eq!(
            filter.controller,
            Some(crate::target::PlayerFilter::IteratedPlayer)
        );
        assert!(filter.with_counter.is_some());
        assert!(matches!(
            &gain.action,
            SubjectVerbActionAst::GainControl {
                target: TargetAst::Tagged(gain_tag, _),
                ..
            } if gain_tag == tag
        ));
        assert!(matches!(
            &untap.action,
            SubjectVerbActionAst::Untap {
                target: TargetAst::Tagged(untap_tag, _),
            } if untap_tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        ));
    }

    #[test]
    fn negated_tagged_condition_does_not_establish_object_antecedent() {
        let predicate = PredicateAst::Not(Box::new(PredicateAst::TaggedMatches(
            "enchanted".into(),
            ObjectFilter::creature(),
        )));

        assert_eq!(predicate_object_filter_antecedent(&predicate), None);
    }

    #[test]
    fn ambiguous_or_condition_does_not_establish_object_antecedent() {
        let predicate = PredicateAst::Or(
            Box::new(PredicateAst::TaggedMatches(
                "that_creature".into(),
                ObjectFilter::creature(),
            )),
            Box::new(PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter: ObjectFilter::creature().with_subtype(crate::types::Subtype::Lizard),
            }),
        );

        assert_eq!(predicate_object_filter_antecedent(&predicate), None);
    }

    #[test]
    fn or_condition_with_same_tagged_subject_establishes_unique_antecedent() {
        let tag: crate::tag::TagKey = "that_creature".into();
        let predicate = PredicateAst::Or(
            Box::new(PredicateAst::TaggedMatches(
                tag.clone(),
                ObjectFilter::creature(),
            )),
            Box::new(PredicateAst::TaggedMatches(
                tag.clone(),
                ObjectFilter::creature().you_control(),
            )),
        );

        assert_eq!(
            predicate_object_filter_antecedent(&predicate),
            Some(ObjectFilter::tagged(tag))
        );
    }

    #[test]
    fn counted_condition_binds_only_explicit_random_those_target() {
        let mut counted = ObjectFilter::creature().with_counter_type(CounterType::Aim);
        counted.controller = Some(crate::target::PlayerFilter::NotYou);
        let predicate = PredicateAst::ValueComparison {
            left: Value::Count(counted.clone()),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(2),
        };
        let random_those = TargetAst::WithCount(
            Box::new(TargetAst::Object(
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key()),
                None,
                None,
            )),
            crate::effect::ChoiceCount::exactly(1).at_random(),
        );
        let mut effects = vec![EffectAst::subject_verb_destroy(random_those)];

        bind_random_count_condition_antecedent_in_effects(&mut effects, &predicate);

        let EffectAst::SubjectVerb(destroy) = &effects[0] else {
            panic!("expected destroy effect");
        };
        assert!(matches!(
            &destroy.action,
            SubjectVerbActionAst::Destroy {
                target: TargetAst::WithCount(inner, count),
                ..
            } if count.random
                && matches!(inner.as_ref(), TargetAst::Object(filter, _, _) if filter == &counted)
        ));
    }

    #[test]
    fn counted_condition_leaves_nonrandom_it_target_unbound() {
        let predicate = PredicateAst::ValueComparison {
            left: Value::Count(ObjectFilter::creature()),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(2),
        };
        let mut effects = vec![effect(SubjectVerbActionAst::GrantAbilitiesToTarget {
            target: it_target(),
            abilities: Vec::new(),
            duration: Until::EndOfTurn,
            condition: None,
            set_quantifier_surface: None,
        })];

        bind_random_count_condition_antecedent_in_effects(&mut effects, &predicate);

        let EffectAst::SubjectVerb(grant) = &effects[0] else {
            panic!("expected grant effect");
        };
        assert!(matches!(
            &grant.action,
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: TargetAst::Tagged(tag, _),
                ..
            } if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        ));
    }

    #[test]
    fn source_exiled_count_condition_binds_plural_move_to_the_whole_collection() {
        let mut source_exiled =
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.key())
                .in_zone(crate::zone::Zone::Exile);
        source_exiled.source_surface =
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string(),
            ));
        let predicate = PredicateAst::ValueComparison {
            left: Value::Count(source_exiled.clone()),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(1),
        };
        let mut effects = vec![
            EffectAst::subject_verb_move_to_zone(
                it_target(),
                crate::zone::Zone::Graveyard,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )
            .with_move_to_zone_plural_surface(),
        ];

        bind_condition_collection_antecedent_in_effects(&mut effects, &predicate);

        let EffectAst::SubjectVerb(move_effect) = &effects[0] else {
            panic!("expected move effect");
        };
        assert!(matches!(
            &move_effect.action,
            SubjectVerbActionAst::MoveToZone {
                target: TargetAst::Object(filter, _, _),
                target_plural_surface: true,
                all: true,
                ..
            } if filter == &source_exiled
        ));
    }

    #[test]
    fn source_damage_to_player_retargets_implicit_must_attack_subject() {
        let mut effects = vec![
            EffectAst::subject_verb_damage(
                Value::Fixed(3),
                TargetAst::Player(crate::target::PlayerFilter::You, None),
            ),
            EffectAst::subject_verb_grant_abilities_to_target(
                it_target(),
                vec![GrantedAbilityAst::MustAttack],
                Until::EndOfTurn,
            ),
        ];

        resolve_source_damage_attack_followups_to_source(&mut effects);

        let EffectAst::SubjectVerb(grant) = &effects[1] else {
            panic!("expected grant effect");
        };
        assert!(matches!(
            &grant.action,
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: TargetAst::Source(_),
                ..
            }
        ));
    }

    #[test]
    fn body_local_target_supersedes_condition_antecedent() {
        let controlled = TargetAst::Object(ObjectFilter::creature(), None, None);
        let mut effects = vec![
            effect(SubjectVerbActionAst::GainControl {
                target: controlled,
                duration: Until::EndOfTurn,
                condition: None,
                controller_reference: None,
                source_reference_surface: None,
            }),
            effect(SubjectVerbActionAst::Untap {
                target: it_target(),
            }),
        ];

        bind_condition_antecedent_in_effects(
            &mut effects,
            &ObjectFilter::creature().you_control(),
            ConditionAntecedentBinding::TaggedItOnly,
        );

        let EffectAst::SubjectVerb(untap) = &effects[1] else {
            panic!("expected untap effect");
        };
        assert!(matches!(
            &untap.action,
            SubjectVerbActionAst::Untap {
                target: TargetAst::Tagged(tag, _)
            } if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        ));
    }

    #[test]
    fn created_tokens_supersede_condition_antecedent() {
        let create = effect(SubjectVerbActionAst::CreateTokenWithMods {
            name: "Goblin Rogue".to_string(),
            definition: crate::grammar::token_definitions::parse_token_definition_shape_text(
                "1/1 black Goblin Rogue creature token",
            )
            .expect("test token definition should parse"),
            count: Value::Fixed(2),
            dynamic_power_toughness: None,
            player: PlayerAst::That,
            actor_surface_explicit: false,
            attached_to: None,
            tapped: false,
            attacking: false,
            attack_target_player: None,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: crate::target::PlayerFilter::Any,
            granted_abilities: Vec::new(),
            ability_presentation: None,
        });
        let mut effects = vec![
            create,
            effect(SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: it_target(),
                abilities: vec![crate::cards::builders::KeywordAction::Haste.into()],
                duration: Until::EndOfTurn,
                condition: None,
                set_quantifier_surface: None,
            }),
        ];

        bind_condition_antecedent_in_effects(
            &mut effects,
            &ObjectFilter::tagged("sacrificed"),
            ConditionAntecedentBinding::TaggedItOnly,
        );

        let EffectAst::SubjectVerb(grant) = &effects[1] else {
            panic!("expected token ability grant");
        };
        assert!(matches!(
            &grant.action,
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: TargetAst::Tagged(tag, _),
                ..
            } if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        ));
    }

    #[test]
    fn condition_antecedent_binds_coordinated_object_actions() {
        let mut effects = vec![
            effect(SubjectVerbActionAst::GainLife {
                amount: Value::Fixed(1),
            }),
            effect(SubjectVerbActionAst::Tap {
                target: it_target(),
            }),
            effect(SubjectVerbActionAst::Untap {
                target: it_target(),
            }),
        ];
        let antecedent = ObjectFilter::creature().you_control();

        bind_condition_antecedent_in_effects(
            &mut effects,
            &antecedent,
            ConditionAntecedentBinding::TaggedItOnly,
        );

        let EffectAst::SubjectVerb(tap) = &effects[1] else {
            panic!("expected tap effect");
        };
        assert!(matches!(
            &tap.action,
            SubjectVerbActionAst::Tap {
                target: TargetAst::Object(filter, _, _)
            } if filter == &antecedent
        ));
        let EffectAst::SubjectVerb(untap) = &effects[2] else {
            panic!("expected untap effect");
        };
        assert!(matches!(
            &untap.action,
            SubjectVerbActionAst::Untap {
                target: TargetAst::Object(filter, _, _)
            } if filter == &antecedent
        ));
    }

    #[test]
    fn source_condition_animation_retarget_yields_to_body_local_target() {
        let mut effects = vec![
            effect(SubjectVerbActionAst::GainControl {
                target: TargetAst::Object(ObjectFilter::creature(), None, None),
                duration: Until::EndOfTurn,
                condition: None,
                controller_reference: None,
                source_reference_surface: None,
            }),
            effect(SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: it_target(),
                abilities: Vec::new(),
                duration: Until::EndOfTurn,
                condition: None,
                set_quantifier_surface: None,
            }),
        ];

        resolve_it_animations_to_source(&mut effects);

        let EffectAst::SubjectVerb(grant) = &effects[1] else {
            panic!("expected grant effect");
        };
        assert!(matches!(
            &grant.action,
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: TargetAst::Tagged(tag, _),
                ..
            } if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        ));
    }

    #[test]
    fn source_condition_animation_retargets_coordinated_unshadowed_it() {
        let grant = || {
            effect(SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: it_target(),
                abilities: Vec::new(),
                duration: Until::EndOfTurn,
                condition: None,
                set_quantifier_surface: None,
            })
        };
        let mut effects = vec![grant(), grant()];

        resolve_it_animations_to_source(&mut effects);

        for grant in &effects {
            let EffectAst::SubjectVerb(grant) = grant else {
                panic!("expected grant effect");
            };
            assert!(matches!(
                &grant.action,
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    target: TargetAst::Source(_),
                    ..
                }
            ));
        }
    }

    #[test]
    fn top_library_observation_keeps_persistent_trigger_subjects_distinct() {
        let mut effects = vec![
            EffectAst::subject_verb_reveal_top(PlayerAst::You),
            EffectAst::Conditional {
                predicate: PredicateAst::ItIsLandCard,
                if_true: vec![EffectAst::subject_verb_destroy(it_target())],
                if_false: vec![EffectAst::subject_verb_pump(
                    Value::Fixed(3),
                    Value::Fixed(3),
                    it_target(),
                    Until::EndOfTurn,
                    None,
                )],
            },
            EffectAst::subject_verb_remove_from_combat(it_target()),
            EffectAst::subject_verb_move_to_zone(
                it_target(),
                crate::zone::Zone::Library,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            ),
        ];

        bind_trigger_antecedent_after_top_library_observation(
            &mut effects,
            &crate::tag::CompilerReferenceTag::Triggering.key(),
        );

        let EffectAst::Conditional {
            if_true, if_false, ..
        } = &effects[1]
        else {
            panic!("expected conditional");
        };
        assert!(matches!(
            &if_true[0],
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::Destroy {
                        target: TargetAst::Tagged(tag, _),
                        ..
                    } if tag.as_str() == "triggering"
                )
        ));
        assert!(matches!(
            &if_false[0],
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::Pump {
                        target: TargetAst::Tagged(tag, _),
                        ..
                    } if tag.as_str() == "triggering"
                )
        ));
        assert!(matches!(
            &effects[2],
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::RemoveFromCombat {
                        target: TargetAst::Tagged(tag, _),
                    } if tag.as_str() == "triggering"
                )
        ));
        assert!(matches!(
            &effects[3],
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::MoveToZone {
                        target: TargetAst::Tagged(tag, _),
                        ..
                    } if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                )
        ));
    }

    #[test]
    fn moved_observed_card_supersedes_trigger_antecedent_within_its_branch() {
        let mut effects = vec![
            EffectAst::subject_verb_reveal_top(PlayerAst::You),
            EffectAst::Conditional {
                predicate: PredicateAst::ItIsLandCard,
                if_true: vec![
                    EffectAst::subject_verb_move_to_zone(
                        it_target(),
                        crate::zone::Zone::Battlefield,
                        false,
                        crate::cards::builders::ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb_pump(
                        Value::Fixed(1),
                        Value::Fixed(1),
                        it_target(),
                        Until::Forever,
                        None,
                    ),
                ],
                if_false: Vec::new(),
            },
        ];

        bind_trigger_antecedent_after_top_library_observation(
            &mut effects,
            &crate::tag::CompilerReferenceTag::Triggering.key(),
        );

        let EffectAst::Conditional { if_true, .. } = &effects[1] else {
            panic!("expected conditional");
        };
        for effect in if_true {
            let EffectAst::SubjectVerb(subject_verb) = effect else {
                panic!("expected subject-verb effect");
            };
            let target = match &subject_verb.action {
                SubjectVerbActionAst::MoveToZone { target, .. }
                | SubjectVerbActionAst::Pump { target, .. } => target,
                other => panic!("unexpected action {other:?}"),
            };
            assert!(matches!(
                target,
                TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            ));
        }
    }
}

pub use crate::condition_antecedents::{bind_condition_filter_antecedent, merge_filter_overlay};
