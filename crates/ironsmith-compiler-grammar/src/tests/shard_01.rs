#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::*;

pub(super) fn conditional_effect_parts(
    effect: &crate::cards::builders::EffectAst,
) -> (
    &crate::cards::builders::PredicateAst,
    &[crate::cards::builders::EffectAst],
    &[crate::cards::builders::EffectAst],
) {
    match effect {
        crate::cards::builders::EffectAst::ControlFlow(control) => {
            let crate::model::ControlFlowNodeAst::Condition {
                condition,
                consequence_program,
                alternative_program,
                ..
            } = &control.node
            else {
                panic!("expected a conditional control-flow node, got {control:?}");
            };
            let crate::model::ControlPredicateAst::State(predicate) = &condition.predicate else {
                panic!("expected a state predicate, got {condition:?}");
            };
            let if_true = &control
                .program(*consequence_program)
                .expect("conditional consequence program")
                .effects;
            let if_false = alternative_program
                .map(|program| {
                    control
                        .program(program)
                        .expect("conditional alternative program")
                        .effects
                        .as_slice()
                })
                .unwrap_or(&[]);
            if condition.negated_surface {
                (predicate, if_false, if_true)
            } else {
                (predicate, if_true, if_false)
            }
        }
        crate::cards::builders::EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => (predicate, if_true, if_false),
        crate::cards::builders::EffectAst::TrailingIf {
            predicate,
            effects: if_true,
        } => (predicate, if_true, &[]),
        crate::cards::builders::EffectAst::TrailingUnless { predicate, effects } => {
            (predicate, &[], effects)
        }
        other => panic!("expected a conditional effect, got {other:?}"),
    }
}

#[test]
pub(super) fn aetherflux_conduit_uses_triggering_spell_mana_spent_value() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Aetherflux Conduit")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever you cast a spell, you get an amount of {E} (energy counters) equal to the amount of mana spent to cast that spell.\n{T}, Pay fifty {E}: Draw seven cards. You may cast any number of spells from your hand without paying their mana costs.",
        )
        .expect("Aetherflux Conduit should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("ManaSpentToCastTriggeringObject"), "{debug}");
    assert!(debug.contains("ForEachObject"), "{debug}");
    assert!(format!("{:?}", def.abilities).contains("zone: Some(Hand)"));
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        !debug.contains("MayCastMatchingSpellWithoutPayingManaCost"),
        "{debug}"
    );
}

#[test]
pub(super) fn archaics_agony_binds_excess_damage_to_the_damage_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Archaic's Agony")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Converge — Archaic's Agony deals X damage to target creature, where X is the number of colors of mana spent to cast this spell. Exile cards from the top of your library equal to the excess damage dealt to that creature this way. You may play those cards until the end of your next turn.",
        )
        .expect("Archaic's Agony should parse");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(debug.contains("WithIdEffect"), "{debug}");
    assert!(debug.contains("ExcessDamage"), "{debug}");
    assert!(!debug.contains("PendingEffectMetric"), "{debug}");
}

#[test]
pub(super) fn attach_up_to_one_target_equipment_to_it_parses_target_object() {
    let tokens = lex_line("Attach up to one target Equipment to it.", 0)
        .expect("rewrite lexer should classify attach clause");
    let parsed = parse_effect_sentence_lexed(&tokens).expect("attach clause should parse");

    let [
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Attach { object, target },
                ..
            },
        ),
    ] = parsed.as_slice()
    else {
        panic!("expected attach effect, got {parsed:?}");
    };

    assert!(
        !matches!(object, crate::cards::builders::TargetAst::Source(_)),
        "expected object side to remain a targetable attachment object"
    );
    assert!(matches!(
        target,
        crate::cards::builders::TargetAst::Tagged(tag, _)
            if tag.as_str() == crate::cards::builders::IT_TAG
    ));
}

#[test]
pub(super) fn attach_source_to_up_to_one_target_preserves_optional_destination_count() {
    let tokens = lex_line(
        "Attach this Equipment to up to one target creature you control.",
        0,
    )
    .expect("rewrite lexer should classify attach clause");
    let parsed = parse_effect_sentence_lexed(&tokens).expect("attach clause should parse");

    let [
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Attach { target, .. },
                ..
            },
        ),
    ] = parsed.as_slice()
    else {
        panic!("expected attach effect, got {parsed:?}");
    };

    assert!(
        matches!(target,
            crate::cards::builders::TargetAst::WithCount(inner, count)
                if *count == ChoiceCount::up_to(1)
                    && matches!(inner.as_ref(),
                        crate::cards::builders::TargetAst::Object(filter, Some(_), _)
                            if filter.card_types.contains(&CardType::Creature)
                                && filter.controller
                                    == Some(crate::cards::builders::PlayerFilter::You))),
        "optional attachment destination should retain its authored target count: {target:#?}"
    );

    let definition = CardDefinitionBuilder::new(CardId::new(), "Optional Attach Variant")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "When this Equipment enters, attach it to up to one target creature you control.",
        )
        .expect("optional attachment destination should lower");
    let attach = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<crate::effects::AttachObjectsEffect>())
                .cloned(),
            _ => None,
        })
        .expect("trigger should contain an attachment effect");

    assert_eq!(
        attach.target.count(),
        ChoiceCount::up_to(1),
        "lowering must preserve the optional target cardinality: {attach:#?}"
    );
}

#[test]
pub(super) fn attach_to_that_creature_reuses_trigger_identity_without_a_fresh_choice()
-> Result<(), CardTextError> {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Kemba, Kha Enduring")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Cat])
        .parse_text(
            "Whenever Kemba or another Cat you control enters, attach up to one target Equipment you control to that creature.",
        )?;
    let effects = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                Some(triggered.effects.flattened_default_effects())
            }
            _ => None,
        })
        .expect("Kemba should have a triggered ability");
    let attach = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::AttachObjectsEffect>())
        .expect("Kemba should attach the targeted Equipment");

    assert!(
        matches!(attach.target.base(), crate::target::ChooseSpec::Tagged(_)),
        "the attachment destination should remain the exact triggering creature: {attach:#?}"
    );
    assert!(
        !effects.iter().any(|effect| effect
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .is_some_and(|choice| choice.tag.as_str().starts_with("attachment_target"))),
        "the exact triggering creature must not become a fresh attachment choice: {effects:#?}"
    );
    Ok(())
}

#[test]
pub(super) fn storm_herald_keeps_one_destination_choice_per_returned_aura()
-> Result<(), CardTextError> {
    fn spec_references_tag(
        spec: &crate::target::ChooseSpec,
        expected: &crate::tag::TagKey,
    ) -> bool {
        match spec.unhinted() {
            crate::target::ChooseSpec::Object(filter) | crate::target::ChooseSpec::All(filter) => {
                filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag == *expected
                        && constraint.relation
                            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                })
            }
            crate::target::ChooseSpec::Target(inner)
            | crate::target::ChooseSpec::WithCount(inner, _)
            | crate::target::ChooseSpec::WithCountValue(inner, _, _) => {
                spec_references_tag(inner, expected)
            }
            crate::target::ChooseSpec::Tagged(tag) => tag == expected,
            _ => false,
        }
    }

    let definition = CardDefinitionBuilder::new(CardId::new(), "Storm Herald")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Haste\nWhen this creature enters, return any number of Aura cards from your graveyard to the battlefield attached to creatures you control. Exile those Auras at the beginning of your next end step. If those Auras would leave the battlefield, exile them instead of putting them anywhere else.",
        )?;
    let effects = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                Some(triggered.effects.flattened_default_effects())
            }
            _ => None,
        })
        .expect("Storm Herald should have an enters trigger");
    let attach = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::AttachObjectsEffect>(effect))
        .expect("the returned Auras should retain their attachment instruction");
    assert!(
        attach.individual_targets,
        "plural destinations require one legal choice per Aura: {attach:#?}"
    );
    let crate::target::ChooseSpec::All(returned_filter) = attach.objects.base() else {
        panic!("the complete returned Aura collection should be attached: {attach:#?}");
    };
    let returned_tag = returned_filter
        .tagged_constraints
        .iter()
        .find(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        .map(|constraint| constraint.tag.clone())
        .expect("the returned Aura collection should retain its move-result tag");

    let delayed_exile = effects
        .iter()
        .find_map(|effect| {
            let schedule =
                super::find_nested_effect::<crate::effects::ScheduleDelayedTriggerEffect>(effect)?;
            schedule
                .effects
                .iter()
                .find_map(|nested| {
                    super::find_nested_effect::<crate::effects::MoveToZoneEffect>(nested)
                })
                .filter(|moved| moved.zone == Zone::Exile)
        })
        .expect("the returned Auras should be exiled at the next end step");
    assert!(
        spec_references_tag(&delayed_exile.target, &returned_tag),
        "the delayed exile must consume the exact returned collection: {delayed_exile:#?}"
    );

    let replacement = effects
        .iter()
        .find_map(|effect| {
            super::find_nested_effect::<crate::effects::RegisterZoneReplacementEffect>(effect)
        })
        .expect("the returned Auras should retain their leave-battlefield replacement");
    assert!(
        spec_references_tag(&replacement.target, &returned_tag),
        "the replacement must watch the exact returned collection: {replacement:#?}"
    );
    Ok(())
}

#[test]
pub(super) fn library_search_put_attach_shuffle_preserves_the_attachment_step() {
    for (name, text, card_types) in [
        (
            "Stonehewer Giant",
            "Vigilance\n{1}{W}, {T}: Search your library for an Equipment card, put it onto the battlefield, attach it to a creature you control, then shuffle.",
            vec![CardType::Creature],
        ),
        (
            "Quest for the Holy Relic",
            "Whenever you cast a creature spell, you may put a quest counter on this enchantment.\nRemove five quest counters from this enchantment and sacrifice it: Search your library for an Equipment card, put it onto the battlefield, attach it to a creature you control, then shuffle.",
            vec![CardType::Enchantment],
        ),
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .parse_text(text)
            .unwrap_or_else(|error| panic!("{name} should parse: {error}"));
        let debug = format!("{definition:#?}");

        assert!(
            debug.contains("ChooseObjectsEffect")
                && debug.contains("MoveToZoneEffect")
                && debug.contains("AttachObjectsEffect")
                && debug.contains("ShuffleLibraryEffect"),
            "{name} should retain the complete search/put/attach/shuffle pipeline: {debug}"
        );
    }
}

#[test]
pub(super) fn return_all_attached_to_a_prior_object_keeps_the_attachment_step() {
    fn contains_tagged_move_all_to_battlefield(
        effect: &crate::effect::Effect,
        expected_tag: &crate::tag::TagKey,
    ) -> bool {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
            && &tagged.tag == expected_tag
            && tagged
                .effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                .is_some_and(|moved| {
                    moved.zone == Zone::Battlefield
                        && matches!(moved.target.base(), crate::target::ChooseSpec::All(_))
                })
        {
            return true;
        }

        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            if !found && contains_tagged_move_all_to_battlefield(child, expected_tag) {
                found = true;
            }
        });
        found
    }

    let definition = CardDefinitionBuilder::new(CardId::new(), "Flickerform")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant creature\n{2}{W}{W}: Exile enchanted creature and all Auras attached to it. At the beginning of the next end step, return that card to the battlefield under its owner's control. If you do, return the other cards exiled this way to the battlefield under their owners' control attached to that creature.",
        )
        .expect("return-all attached to a prior object should compile");
    let effects = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => {
                Some(activated.effects.flattened_default_effects())
            }
            _ => None,
        })
        .expect("Flickerform should have an activated ability");
    let attach = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::AttachObjectsEffect>(effect))
        .expect("the returned Aura collection should be attached");
    let crate::target::ChooseSpec::All(attached_filter) = attach.objects.base() else {
        panic!("the attachment step should retain the complete returned collection: {attach:#?}");
    };
    let moved_tag = attached_filter
        .tagged_constraints
        .iter()
        .find(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        .map(|constraint| &constraint.tag)
        .expect("the attachment collection should reference the return move");

    assert!(
        effects
            .iter()
            .any(|effect| contains_tagged_move_all_to_battlefield(effect, moved_tag)),
        "the return-all move and attachment collection should share one tag: {definition:#?}"
    );
}

#[test]
pub(super) fn attach_any_number_equipment_to_it_parses_counted_object_set() {
    let tokens = lex_line("Attach any number of Equipment you control to it.", 0)
        .expect("rewrite lexer should classify attach clause");
    let parsed = parse_effect_sentence_lexed(&tokens).expect("attach clause should parse");

    let [
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Attach { object, target },
                ..
            },
        ),
    ] = parsed.as_slice()
    else {
        panic!("expected attach effect, got {parsed:?}");
    };

    let crate::cards::builders::TargetAst::WithCount(inner, count) = object else {
        panic!("expected counted attachment object, got {object:?}");
    };
    assert_eq!(*count, crate::cards::builders::ChoiceCount::any_number());
    assert!(
        matches!(inner.as_ref(), crate::cards::builders::TargetAst::Object(filter, None, _)
            if filter.subtypes.contains(&crate::Subtype::Equipment)
                && filter.controller == Some(crate::cards::builders::PlayerFilter::You)
                && filter.zone == Some(crate::Zone::Battlefield)),
        "expected counted Equipment-you-control object filter, got {inner:?}"
    );
    assert!(matches!(
        target,
        crate::cards::builders::TargetAst::Tagged(tag, _)
            if tag.as_str() == crate::cards::builders::IT_TAG
    ));
}

#[test]
pub(super) fn attach_any_number_equipment_to_it_lowers_without_targeting_equipment()
-> Result<(), CardTextError> {
    let def = CardDefinitionBuilder::new(CardId::new(), "Armed and Armored Variant")
        .mana_cost(super::super::util::parse_scryfall_mana_cost("{1}{W}").unwrap())
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Vehicles you control become artifact creatures until end of turn. Choose a Dwarf you control. Attach any number of Equipment you control to it.",
        )?;

    let effects = def
        .spell_effect
        .as_ref()
        .expect("spell should lower")
        .flattened_default_effects();
    let attach = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::AttachObjectsEffect>())
        .expect("attach effect should be present");

    assert!(
        matches!(
            &attach.objects,
            crate::target::ChooseSpec::WithCount(inner, count)
                if *count == ChoiceCount::any_number()
                    && matches!(inner.as_ref(), crate::target::ChooseSpec::Object(filter)
                        if filter.subtypes.contains(&Subtype::Equipment)
                            && filter.controller == Some(crate::target::PlayerFilter::You)
                            && filter.zone == Some(Zone::Battlefield))
        ),
        "{attach:#?}"
    );
    assert!(
        !effects.iter().any(|effect| effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_some_and(|target_only| target_only.target == attach.objects)),
        "non-target Equipment attachments should not get a target-only prelude: {effects:#?}"
    );
    Ok(())
}

#[test]
pub(super) fn attach_all_card_controls_lower_to_all_object_selection() {
    for (name, text, card_types) in [
        (
            "Balan, Wandering Knight",
            "First strike\nBalan has double strike as long as two or more Equipment are attached to it.\n{1}{W}: Attach all Equipment you control to Balan.",
            vec![CardType::Creature],
        ),
        (
            "Glamer Spinners",
            "Flash\nFlying\nWhen this creature enters, attach all Auras enchanting target permanent to another permanent with the same controller.",
            vec![CardType::Creature],
        ),
        (
            "Rhuk, Hexgold Nabber",
            "Trample, haste\nWhenever an equipped creature you control other than Rhuk attacks or dies, you may attach all Equipment attached to that creature to Rhuk.",
            vec![CardType::Creature],
        ),
        (
            "Vulshok Battlemaster",
            "Haste\nWhen this creature enters, attach all Equipment on the battlefield to it. (Control of the Equipment doesn't change.)",
            vec![CardType::Creature],
        ),
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .parse_text(text)
            .unwrap_or_else(|error| panic!("{name} should parse: {error}"));
        let debug = format!("{definition:#?}");
        assert!(debug.contains("AttachObjectsEffect"), "{name}: {debug}");
        assert!(debug.contains("All("), "{name}: {debug}");
        if name == "Glamer Spinners" {
            assert!(debug.contains("AttachedToTaggedObject"), "{name}: {debug}");
            assert!(debug.contains("SameControllerAsTagged"), "{name}: {debug}");
            assert!(debug.contains("IsNotTaggedObject"), "{name}: {debug}");
        }
    }
}

#[test]
pub(super) fn amass_where_x_clause_replaces_unbound_x() {
    let tokens = lex_line(
        "Amass Orcs X, where X is the number of Equipment attached to this creature.",
        0,
    )
    .expect("rewrite lexer should classify amass where-x clause");
    let parsed = parse_effect_sentence_lexed(&tokens).expect("amass clause should parse");

    let [
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Amass { amount, .. },
                ..
            },
        ),
    ] = parsed.as_slice()
    else {
        panic!("expected amass effect, got {parsed:?}");
    };

    assert!(
        !matches!(amount, crate::effect::Value::X),
        "expected where-X clause to bind amass amount"
    );
}

#[test]
pub(super) fn rewrite_triggered_attach_it_to_target_seeds_triggering_object_reference() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Attach Source Variant")
        .parse_text("When this Equipment enters, attach it to target creature you control.")
        .expect("triggered attach-it clause should lower");

    let debug = format!("{def:?}");
    assert!(
        debug.contains("Attach"),
        "expected lowered definition to contain attach effect, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_if_you_do_attach_it_uses_prior_returned_object_reference() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Attach Returned Variant")
        .parse_text(
            "When this creature enters, you may return target Equipment card from your graveyard to the battlefield. If you do, you may attach it to this creature.",
        )
        .expect("conditional attach-it clause should use returned object");

    let debug = format!("{def:?}");
    assert!(
        debug.contains("Attach"),
        "expected lowered definition to contain attach effect, got {debug}"
    );
}

#[test]
pub(super) fn return_it_as_aura_enchantment_lowers_to_atomic_aura_return() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Returned Aura Variant")
        .parse_text(
            "When this creature dies, return it to the battlefield. It's an Aura enchantment with enchant creature you control and it loses all other abilities.",
        )
        .expect("return-as-aura clause should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ReturnFromGraveyardToBattlefieldEffect")
            && debug.contains("AddCardTypes")
            && debug.contains("Enchantment")
            && debug.contains("AddSubtypes")
            && debug.contains("Aura")
            && debug.contains("SetAuraAttachmentFilter")
            && debug.contains("RemoveAllAbilities")
            && !debug.contains("BecomeAuraEnchantment"),
        "expected return-as-aura wording to lower to atomic aura return, got {debug}"
    );
}

#[test]
pub(super) fn graveyard_static_enters_with_additional_counter_for_filter_lowers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Graveyard Counter Variant")
        .parse_text(
            "As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.",
        )
        .expect("graveyard ETB counter replacement should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("EnterWithCountersForFilter")
            && debug.contains("functional_zones")
            && debug.contains("Graveyard")
            && debug.contains("Human")
            && debug.contains("PlusOnePlusOne"),
        "expected graveyard functional ETB counter replacement, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_if_clause_supports_passive_this_way_tagged_object_predicate() {
    let tokens = lex_line(
        "If a red card is discarded this way, this deals 4 damage to any target.",
        0,
    )
    .expect("rewrite lexer should classify passive this-way predicate");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("passive this-way predicate should parse");

    let [
        crate::cards::builders::EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::PriorEffectResult(surface),
            effects,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected conditional damage clause, got {parsed:?}");
    };
    let filter = &surface.filter;
    assert_eq!(surface.action, ironsmith_core::PriorEffectAction::Discarded);
    assert!(
        filter.colors.is_some(),
        "expected red-card predicate to retain color filter, got {filter:?}"
    );
    assert!(matches!(
        effects.as_slice(),
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::DealDamage { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_if_clause_keeps_passive_this_way_card_filters_zone_neutral() {
    let tokens = lex_line(
        "If a land card is discarded this way, this deals 4 damage to any target.",
        0,
    )
    .expect("rewrite lexer should classify passive this-way card predicate");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("passive this-way card predicate should parse");

    let [
        crate::cards::builders::EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::PriorEffectResult(surface),
            ..
        },
    ] = parsed.as_slice()
    else {
        panic!("expected conditional damage clause, got {parsed:?}");
    };
    let filter = &surface.filter;
    assert_eq!(surface.action, ironsmith_core::PriorEffectAction::Discarded);
    assert_eq!(filter.card_types, vec![CardType::Land]);
    assert!(
        filter.zone.is_none(),
        "card predicates should not inherit a battlefield-only zone, got {filter:?}"
    );
}

#[test]
pub(super) fn rewrite_exile_from_hand_or_graveyard_preserves_both_choice_zones() {
    let tokens = lex_line(
        "You may exile an artifact or creature card from your hand or graveyard and put a cage counter on it.",
        0,
    )
    .expect("rewrite lexer should classify hand-or-graveyard exile clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("hand-or-graveyard exile clause should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("Hand"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("cage"), "{debug}");
    assert!(debug.contains("Coordination"), "{debug}");
    assert!(
        !debug.contains("Disjunction"),
        "the zone union must not become alternate executable effects: {debug}"
    );
}

#[test]
pub(super) fn rewrite_copy_activated_abilities_static_preserves_counter_display_and_once_limit() {
    let tokens = lex_line(
        "This has all activated abilities of all cards you own in exile with cage counters on them. You may activate each of those abilities only once each turn.",
        0,
    )
    .expect("rewrite lexer should classify copied-activated-ability static line");

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("copied-activated-ability static line should parse")
        .expect("copied-activated-ability static line should produce an ability");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("CopyActivatedAbilities"), "{debug}");
    assert!(debug.contains("cage"), "{debug}");
    assert!(debug.contains("force_once_each_turn: true"), "{debug}");
    assert!(
        debug.contains("You may activate each of those abilities only once each turn"),
        "{debug}"
    );

    let loyalty_tokens = lex_line(
        "This planeswalker has all loyalty abilities of all other planeswalkers on the battlefield.",
        0,
    )
    .expect("rewrite lexer should classify copied loyalty abilities");
    let loyalty =
        super::super::keyword_static::parse_static_ability_ast_line_lexed(&loyalty_tokens)
            .expect("copied loyalty abilities should parse")
            .expect("copied loyalty abilities should produce an ability");
    let loyalty_debug = format!("{loyalty:#?}");
    assert!(
        loyalty_debug.contains("only_loyalty: true"),
        "{loyalty_debug}"
    );
    assert!(loyalty_debug.contains("counter: None"), "{loyalty_debug}");
    assert!(
        !loyalty_debug.contains("Named(\"battlefield\")"),
        "{loyalty_debug}"
    );
}

#[test]
pub(super) fn rewrite_if_clause_binds_it_was_cast_to_tagged_object() {
    let tokens = lex_line("If it was cast, exile it.", 0)
        .expect("rewrite lexer should classify tagged cast-history predicate");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("tagged cast-history conditional should parse");

    let [crate::cards::builders::EffectAst::Conditional { predicate, .. }] = parsed.as_slice()
    else {
        panic!("expected conditional exile clause, got {parsed:?}");
    };
    assert!(matches!(
        predicate,
        crate::cards::builders::PredicateAst::TaggedWasCast(tag)
            if tag.as_str() == crate::cards::builders::IT_TAG
    ));
}

#[test]
pub(super) fn rewrite_verb_handlers_keep_trailing_instead_if_damage_clause_after_structure_cutover()
{
    let tokens = lex_line(
        "This creature deals 5 damage to target creature instead if it's white.",
        0,
    )
    .expect("rewrite lexer should classify instead-if damage clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("instead-if damage clause should parse");

    let [effect] = parsed.as_slice() else {
        panic!("expected one instead-if damage clause, got {parsed:?}");
    };
    let (predicate, if_true, if_false) = conditional_effect_parts(effect);
    assert!(if_false.is_empty());
    assert!(matches!(
        predicate,
        crate::cards::builders::PredicateAst::ItMatches(_)
    ));
    assert!(matches!(
        if_true,
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::DealDamage { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_verb_handlers_keep_trailing_if_draw_clause_after_structure_cutover() {
    let tokens = lex_line("Draw a card if you control an artifact.", 0)
        .expect("rewrite lexer should classify conditional draw clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("draw clause should parse");

    let [effect] = parsed.as_slice() else {
        panic!("expected one conditional draw clause, got {parsed:?}");
    };
    let (_, if_true, if_false) = conditional_effect_parts(effect);
    assert!(if_false.is_empty());
    assert!(matches!(
        if_true,
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Draw { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_verb_handlers_keep_draw_for_each_player_condition_after_structure_cutover() {
    let tokens = lex_line("Draw a card for each player who controls an artifact.", 0)
        .expect("rewrite lexer should classify draw-for-each-player clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("draw-for-each clause should parse");

    match parsed.as_slice() {
        [crate::cards::builders::EffectAst::ForEachPlayer { effects }] => {
            match effects.as_slice() {
                [
                    crate::cards::builders::EffectAst::Conditional {
                        predicate: _,
                        if_true,
                        if_false,
                    },
                ] => {
                    assert!(if_false.is_empty());
                    assert!(matches!(
                        if_true.as_slice(),
                        [crate::cards::builders::EffectAst::SubjectVerb(
                            crate::cards::builders::SubjectVerbEffectAst {
                                action: crate::cards::builders::SubjectVerbActionAst::Draw { .. },
                                ..
                            }
                        )]
                    ));
                }
                other => panic!("expected conditional draw effect, got {other:?}"),
            }
        }
        other => panic!("expected for-each-player draw clause, got {other:?}"),
    }
}

#[test]
pub(super) fn each_player_exiles_hand_and_draws_keeps_draw_on_iterated_player() {
    let tokens = lex_line(
        "Each player exiles all cards from their hand face down and draws seven cards.",
        0,
    )
    .expect("rewrite lexer should classify each-player hand exchange clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("hand exchange clause should parse");
    let debug = format!("{parsed:#?}");

    fn has_iterated_draw(effect: &crate::cards::builders::EffectAst) -> bool {
        match effect {
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    subject:
                        crate::cards::builders::SubjectVerbSubjectAst {
                            player: crate::cards::builders::PlayerAst::That,
                            ..
                        },
                    action: crate::cards::builders::SubjectVerbActionAst::Draw { .. },
                },
            ) => true,
            crate::cards::builders::EffectAst::ForEachPlayer { effects }
            | crate::cards::builders::EffectAst::Coordinated { effects, .. } => {
                effects.iter().any(has_iterated_draw)
            }
            crate::cards::builders::EffectAst::Coordination(coordination) => {
                coordination.effects().any(has_iterated_draw)
            }
            _ => false,
        }
    }
    let has_iterated_draw = parsed.iter().any(has_iterated_draw);
    assert!(has_iterated_draw, "{debug}");
    assert!(!debug.contains("ItsOwner"), "{debug}");
    assert!(!debug.contains("LibraryOwner"), "{debug}");
}

#[test]
pub(super) fn each_player_exiles_hand_and_draws_keeps_draw_on_iterated_player_in_sequence() {
    let tokens = lex_line(
        "Each player exiles all cards from their hand face down and draws seven cards. At the beginning of the next end step, each player discards their hand.",
        0,
    )
    .expect("rewrite lexer should classify each-player hand exchange sequence");

    let parsed = super::super::effect_sentences::parse_effect_sentences_lexed(&tokens)
        .expect("hand exchange sequence should parse");
    let debug = format!("{parsed:#?}");

    fn has_iterated_draw(effect: &crate::cards::builders::EffectAst) -> bool {
        match effect {
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    subject:
                        crate::cards::builders::SubjectVerbSubjectAst {
                            player: crate::cards::builders::PlayerAst::That,
                            ..
                        },
                    action: crate::cards::builders::SubjectVerbActionAst::Draw { .. },
                },
            ) => true,
            crate::cards::builders::EffectAst::ForEachPlayer { effects }
            | crate::cards::builders::EffectAst::Coordinated { effects, .. } => {
                effects.iter().any(has_iterated_draw)
            }
            crate::cards::builders::EffectAst::Coordination(coordination) => {
                coordination.effects().any(has_iterated_draw)
            }
            _ => false,
        }
    }
    let has_iterated_draw = parsed.iter().any(has_iterated_draw);
    assert!(has_iterated_draw, "{debug}");
    assert!(!debug.contains("ItsOwner"), "{debug}");
    assert!(!debug.contains("LibraryOwner"), "{debug}");
}

#[test]
pub(super) fn each_player_return_with_additional_counter_clause_keeps_counter_followup() {
    let tokens = lex_line(
        "Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it.",
        0,
    )
    .expect("rewrite lexer should classify each-player return-with-counter clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("return-with-counter clause should parse");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("ForEachPlayer")
            && debug.contains("ReturnAllToBattlefield")
            && debug.contains("PutCounters")
            && debug.contains("MinusOneMinusOne"),
        "expected each-player return plus counter follow-up, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_verb_handlers_keep_conditional_gain_control_clause_after_structure_cutover() {
    let tokens = lex_line(
        "Gain control of target creature if you control an artifact until end of turn.",
        0,
    )
    .expect("rewrite lexer should classify conditional gain-control clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("conditional gain-control clause should parse");

    let [effect] = parsed.as_slice() else {
        panic!("expected one conditional gain-control clause, got {parsed:?}");
    };
    let (_, if_true, if_false) = conditional_effect_parts(effect);
    assert!(if_false.is_empty());
    assert!(matches!(
        if_true,
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::GainControl { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_verb_handlers_keep_unless_gain_control_clause_after_structure_cutover() {
    let tokens = lex_line(
        "Gain control of target creature unless you control an artifact until end of turn.",
        0,
    )
    .expect("rewrite lexer should classify unless gain-control clause");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("unless gain-control clause should parse");

    let [effect] = parsed.as_slice() else {
        panic!("expected one unless gain-control clause, got {parsed:?}");
    };
    let (_, if_true, if_false) = conditional_effect_parts(effect);
    assert!(if_true.is_empty());
    assert!(matches!(
        if_false,
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::GainControl { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_etb_where_x_source_stat_normalizes_apostrophe_shapes() {
    let tokens = lex_line("Where X is this creature's power", 0)
        .expect("rewrite lexer should classify where-x source-stat clause");

    let parsed = super::super::keyword_static::parse_where_x_source_stat_value(&tokens);

    assert!(
        matches!(parsed.as_ref(), Some(crate::effect::Value::SourcePower))
            || matches!(
                parsed.as_ref(),
                Some(crate::effect::Value::PowerOf(source))
                    if matches!(source.unhinted(), crate::target::ChooseSpec::Source)
            )
    );
}

#[test]
pub(super) fn rewrite_etb_where_x_named_source_stat_preserves_surface_hint() {
    let tokens = lex_line("Where X is Amy Rose's power", 0)
        .expect("rewrite lexer should classify where-x named source-stat clause");

    let parsed =
        super::super::keyword_static::parse_where_x_named_source_stat_value(&tokens, "Amy Rose");

    assert!(
        format!("{parsed:?}").contains("ShortName(\"Amy Rose\")")
            || format!("{parsed:?}").contains("FullName(\"Amy Rose\")"),
        "expected named-source stat value to preserve a source-reference surface hint, got {parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_etb_enters_tapped_filter_preserves_played_by_opponents_suffix() {
    let tokens = lex_line("Artifacts played by your opponents enter tapped.", 0)
        .expect("rewrite lexer should classify enters-tapped filter clause");

    let ability = super::super::keyword_static::parse_enters_tapped_for_filter_line(&tokens)
        .expect("enters-tapped filter clause should parse")
        .expect("enters-tapped filter clause should build a static ability");
    let debug = format!("{ability:?}");

    assert!(
        debug.contains("played_by_opponent: Some(YourOpponents)"),
        "expected typed played-by-opponents surface provenance, got {debug}"
    );
    assert!(
        debug.contains("controller: Some(Opponent)") && debug.contains("Artifact"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_etb_static_parser_does_not_swallow_triggered_mold_earth_text() {
    let tokens = lex_line(
        "Mold Earth — Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify mold earth text");

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("static parser should accept the line shape");
    assert!(
        parsed.is_none(),
        "expected triggered Mold Earth text to bypass static ETB parsing, got {parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_etb_where_x_aggregate_filter_routes_and_split_through_grammar_separator_helper()
 {
    let tokens = lex_line(
        "where x is the total power of creatures you control and creature cards in your graveyard",
        0,
    )
    .expect("rewrite lexer should classify aggregate where-x clause");

    let parsed = super::super::keyword_static::parse_where_x_is_aggregate_filter_value(&tokens)
        .expect("aggregate where-x clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("TotalPower"), "{debug}");
    assert!(debug.contains("any_of"), "{debug}");
    assert!(debug.contains("controller: Some(You)"), "{debug}");
    assert!(debug.contains("zone: Some(Graveyard)"), "{debug}");
}

#[test]
pub(super) fn rewrite_etb_where_x_total_power_of_sacrificed_creatures_uses_the_sacrifice_reference()
{
    let tokens = lex_line("where x is the total power of the sacrificed creatures", 0)
        .expect("rewrite lexer should classify sacrificed aggregate clause");

    let parsed = super::super::keyword_static::parse_where_x_is_aggregate_filter_value(&tokens)
        .expect("sacrificed aggregate clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("TotalPower"), "{debug}");
    assert!(
        debug.contains("tag: TagKey(\"__it__\")") || debug.contains("tag: TagKey(\"sacrificed"),
        "expected sacrificed creatures to stay tied to a tag, got {debug}"
    );
    assert!(
        !debug.contains("zone: Some(Battlefield)"),
        "sacrificed creatures should not be collapsed to a battlefield-only filter, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_where_x_number_of_creatures_of_chosen_type_is_board_count() {
    let tokens = lex_line(
        "where X is the number of creatures they control of the chosen type",
        0,
    )
    .expect("rewrite lexer should classify chosen-type count clause");

    let parsed = super::super::keyword_static::parse_where_x_is_number_of_filter_value(&tokens)
        .expect("chosen-type count where-x clause should parse");

    match parsed.unhinted() {
        crate::effect::Value::Count(filter) => {
            assert_eq!(filter.card_types, vec![CardType::Creature]);
            assert_eq!(
                filter.controller,
                Some(crate::target::PlayerFilter::IteratedPlayer)
            );
            assert!(filter.chosen_creature_type);
        }
        other => panic!("expected chosen-type creature count, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_where_x_number_of_static_abilities_among_creatures() {
    let tokens = lex_line(
        "where X is the number of abilities from among flying, first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, reach, trample, and vigilance found among creatures you control",
        0,
    )
    .expect("rewrite lexer should classify static ability count clause");

    let parsed = super::super::keyword_static::parse_where_x_is_number_of_filter_value(&tokens)
        .expect("static ability count where-x clause should parse");

    match parsed.unhinted() {
        Value::StaticAbilitiesAmong { filter, abilities } => {
            assert_eq!(filter.card_types, vec![CardType::Creature]);
            assert_eq!(filter.controller, Some(crate::target::PlayerFilter::You));
            assert_eq!(
                abilities,
                &vec![
                    StaticAbilityId::Flying,
                    StaticAbilityId::FirstStrike,
                    StaticAbilityId::DoubleStrike,
                    StaticAbilityId::Deathtouch,
                    StaticAbilityId::Haste,
                    StaticAbilityId::Hexproof,
                    StaticAbilityId::Indestructible,
                    StaticAbilityId::Lifelink,
                    StaticAbilityId::Menace,
                    StaticAbilityId::Reach,
                    StaticAbilityId::Trample,
                    StaticAbilityId::Vigilance,
                ]
            );
        }
        other => panic!("expected static ability count, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_etb_where_x_one_plus_exiled_creatures_mana_value() {
    let tokens = lex_line("where x is 1 plus the exiled creature's mana value", 0)
        .expect("rewrite lexer should classify fixed-plus exiled reference clause");

    let parsed = super::super::keyword_static::parse_value_binding_clause(&tokens)
        .expect("fixed-plus exiled mana-value clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Add"), "{debug}");
    assert!(debug.contains("Fixed(1)"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
}

#[test]
pub(super) fn rewrite_zone_handlers_keep_conditional_destroy_clause_after_structure_cutover() {
    let tokens = lex_line("Destroy target creature if it's white.", 0)
        .expect("rewrite lexer should classify conditional destroy clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("destroy clause should parse");

    let [effect] = parsed.as_slice() else {
        panic!("expected one conditional destroy clause, got {parsed:?}");
    };
    let (predicate, if_true, if_false) = conditional_effect_parts(effect);
    assert!(if_false.is_empty());
    assert!(matches!(
        predicate,
        crate::cards::builders::PredicateAst::ItMatches(_)
    ));
    assert!(matches!(
        if_true,
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Destroy { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_zone_handlers_parse_destroy_unless_target_color_sets_differ() {
    let tokens = lex_line(
        "Destroy two target nonblack creatures unless either one is a color the other isn't.",
        0,
    )
    .expect("rewrite lexer should classify target-set conditional destroy clause");

    let parsed = parse_effect_sentence_lexed(&tokens)
        .expect("target-set conditional destroy clause should parse");

    let [
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected conditional destroy clause, got {parsed:#?}");
    };
    assert!(if_false.is_empty());
    assert!(matches!(
        predicate,
        crate::cards::builders::PredicateAst::Not(inner)
            if matches!(
                inner.as_ref(),
                crate::cards::builders::PredicateAst::TargetObjectsHaveDifferentColorSets
            )
    ));

    let [
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                crate::cards::builders::SubjectVerbActionAst::Destroy {
                    target,
                    no_regeneration: false,
                    ..
                },
            ..
        }),
    ] = if_true.as_slice()
    else {
        panic!("expected one typed destroy branch, got {if_true:#?}");
    };
    let crate::cards::builders::TargetAst::WithCount(inner, count) = target else {
        panic!("expected a counted target set, got {target:#?}");
    };
    assert_eq!(*count, ChoiceCount::exactly(2));
    assert!(matches!(
        inner.as_ref(),
        crate::cards::builders::TargetAst::Object(filter, Some(_), _)
            if filter.card_types == [CardType::Creature]
                && filter.excluded_colors.contains(crate::color::Color::Black)
    ));
}

#[test]
pub(super) fn rewrite_destroy_target_unless_controller_chooses_source_power_damage() {
    let tokens = lex_line(
        "Destroy target permanent unless its controller has this creature deal damage to them equal to his power.",
        0,
    )
    .expect("rewrite lexer should classify targeted-action unless-damage clause");

    let parsed = parse_effect_sentence_lexed(&tokens)
        .expect("targeted-action unless-damage clause should parse");

    let [
        EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one typed unless-action clause, got {parsed:#?}");
    };
    assert_eq!(*player, crate::cards::builders::PlayerAst::ItsController);
    assert!(matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Destroy {
                    target: crate::cards::builders::TargetAst::Object(_, _, _),
                    no_regeneration: false,
                    ..
                },
                ..
            }
        )]
    ));
    let [
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                crate::cards::builders::SubjectVerbActionAst::DealDamage {
                    amount,
                    target,
                    unpreventable: false,
                },
            ..
        }),
    ] = alternative.as_slice()
    else {
        panic!("expected one typed source-power damage alternative, got {alternative:#?}");
    };
    assert_eq!(amount.unhinted(), &crate::effect::Value::SourcePower);
    assert!(
        amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
            && amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::MasculineSourcePossessive),
        "{amount:#?}"
    );
    assert!(matches!(
        target,
        crate::cards::builders::TargetAst::Player(
            crate::filter::PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            None,
        )
    ));
}

#[test]
pub(super) fn rewrite_dead_ringers_lowers_target_color_condition_and_regeneration_prohibition()
-> Result<(), CardTextError> {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Dead Ringers")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy two target nonblack creatures unless either one is a color the other isn't. They can't be regenerated.",
        )?;
    let effects = &definition
        .spell_effect
        .as_ref()
        .expect("Dead Ringers should lower to a spell program")
        .segments[0]
        .default_effects;
    let conditional = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
        .expect("Dead Ringers should lower to a conditional effect");

    assert!(matches!(
        &conditional.condition,
        crate::effect::Condition::Not(inner)
            if matches!(
                inner.as_ref(),
                crate::effect::Condition::TargetObjectsHaveDifferentColorSets
            )
    ));
    let destroy = conditional
        .if_true
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()
                .or_else(|| {
                    effect
                        .downcast_ref::<crate::effects::TaggedEffect>()
                        .and_then(|tagged| {
                            tagged
                                .effect
                                .downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()
                        })
                })
        })
        .expect("regeneration follow-up should remain inside the conditional destroy branch");
    let Some(crate::target::ChooseSpec::WithCount(target, count)) = destroy.target.as_ref() else {
        panic!("destroy branch should preserve its counted target set: {destroy:#?}");
    };
    assert_eq!(*count, ChoiceCount::exactly(2));
    let crate::target::ChooseSpec::Target(target) = target.as_ref() else {
        panic!("destroy branch should preserve targeted selection: {target:#?}");
    };
    let crate::target::ChooseSpec::Object(filter) = target.as_ref() else {
        panic!("destroy branch should preserve its object filter: {target:#?}");
    };
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert!(filter.excluded_colors.contains(crate::color::Color::Black));
    Ok(())
}

#[test]
pub(super) fn rewrite_zone_handlers_keep_nested_instead_if_destroy_clause_after_structure_cutover()
{
    let tokens = lex_line(
        "Destroy target creature if it's white instead if you control an artifact.",
        0,
    )
    .expect("rewrite lexer should classify nested instead-if destroy clause");

    let parsed = parse_effect_sentence_lexed(&tokens)
        .expect("nested instead-if destroy clause should parse");

    let [effect] = parsed.as_slice() else {
        panic!("expected one nested instead-if destroy clause, got {parsed:?}");
    };
    let (_, if_true, if_false) = conditional_effect_parts(effect);
    assert!(if_false.is_empty());
    let [nested] = if_true else {
        panic!("expected nested conditional destroy branch, got {if_true:?}");
    };
    let (base_predicate, nested_if_true, nested_if_false) = conditional_effect_parts(nested);
    assert!(nested_if_false.is_empty());
    assert!(matches!(
        base_predicate,
        crate::cards::builders::PredicateAst::ItMatches(_)
    ));
    assert!(matches!(
        nested_if_true,
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Destroy { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn rewrite_zone_handlers_keep_conditional_exile_clause_after_structure_cutover() {
    let tokens = lex_line("Exile target creature if it's white.", 0)
        .expect("rewrite lexer should classify conditional exile clause");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("exile clause should parse");

    let [effect] = parsed.as_slice() else {
        panic!("expected one conditional exile clause, got {parsed:?}");
    };
    let (predicate, if_true, if_false) = conditional_effect_parts(effect);
    assert!(if_false.is_empty());
    assert!(matches!(
        predicate,
        crate::cards::builders::PredicateAst::ItMatches(_)
    ));
    assert!(matches!(
        if_true,
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Exile { .. },
                ..
            }
        )]
    ));
}

#[test]
pub(super) fn optional_exile_pair_keeps_repeated_article_filters_independent() {
    let tokens = lex_line(
        "You may exile a Human you control and an artifact you control.",
        0,
    )
    .expect("independent optional exile pair should lex");
    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("independent optional exile pair should parse");

    let optional_effects = match parsed.as_slice() {
        [crate::cards::builders::EffectAst::May { effects }]
        | [crate::cards::builders::EffectAst::MayByPlayer { effects, .. }] => effects,
        _ => panic!("expected one optional effect, got {parsed:#?}"),
    };
    let [crate::cards::builders::EffectAst::Coordination(coordination)] =
        optional_effects.as_slice()
    else {
        panic!("expected one coordinated pair, got {optional_effects:#?}");
    };
    fn leaf_effects<'a>(
        effect: &'a crate::cards::builders::EffectAst,
        output: &mut Vec<&'a crate::cards::builders::EffectAst>,
    ) {
        match effect {
            crate::cards::builders::EffectAst::Coordination(nested) => {
                for effect in nested.effects() {
                    leaf_effects(effect, output);
                }
            }
            crate::cards::builders::EffectAst::Sequence { effects }
            | crate::cards::builders::EffectAst::CommaThen { effects }
            | crate::cards::builders::EffectAst::Coordinated { effects, .. } => {
                for effect in effects {
                    leaf_effects(effect, output);
                }
            }
            effect => output.push(effect),
        }
    }
    let mut effects = Vec::new();
    for effect in coordination.effects() {
        leaf_effects(effect, &mut effects);
    }
    let [first, second] = effects.as_slice() else {
        panic!("expected two independent exile selections, got {coordination:#?}");
    };
    fn exile_filter(effect: &crate::cards::builders::EffectAst) -> &crate::target::ObjectFilter {
        let crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::Exile {
                        target,
                        face_down: false,
                        ..
                    },
                ..
            },
        ) = effect
        else {
            panic!("expected a typed exile action, got {effect:#?}");
        };
        match target {
            crate::cards::builders::TargetAst::Object(filter, None, _) => filter,
            crate::cards::builders::TargetAst::WithCount(inner, count) if count.is_single() => {
                let crate::cards::builders::TargetAst::Object(filter, None, _) = inner.as_ref()
                else {
                    panic!("expected a counted object filter, got {target:#?}");
                };
                filter
            }
            _ => panic!("expected a non-target object filter, got {target:#?}"),
        }
    }

    let human = exile_filter(first);
    let artifact = exile_filter(second);
    assert_eq!(human.subtypes, vec![Subtype::Human]);
    assert!(human.card_types.is_empty(), "{human:#?}");
    assert_eq!(human.controller, Some(crate::target::PlayerFilter::You));
    assert_eq!(artifact.card_types, vec![CardType::Artifact]);
    assert!(artifact.subtypes.is_empty(), "{artifact:#?}");
    assert_eq!(artifact.controller, Some(crate::target::PlayerFilter::You));
}

#[test]
pub(super) fn rewrite_zone_handlers_parse_mixed_target_and_all_exile_list() {
    fn mana_value_lte_void_counters(filter: &crate::target::ObjectFilter) -> bool {
        matches!(
            filter.mana_value.as_ref(),
            Some(crate::filter::Comparison::LessThanOrEqualExpr(value))
                if value.as_ref() == &Value::CountersOnSource(CounterType::Void)
        )
    }

    let tokens = lex_line(
        "Exile this artifact, all creatures and planeswalkers with mana value less than or equal to the number of void counters on it, and all creature and planeswalker cards in graveyards with mana value less than or equal to the number of void counters on it.",
        0,
    )
    .expect("rewrite lexer should classify mixed exile list");

    let parsed = parse_effect_sentence_lexed(&tokens).expect("mixed exile list should parse");

    let [crate::cards::builders::EffectAst::Coordination(coordination)] = parsed.as_slice() else {
        panic!("expected mixed exile list to parse as typed coordination, got {parsed:#?}");
    };
    fn leaf_effects<'a>(
        effect: &'a crate::cards::builders::EffectAst,
        output: &mut Vec<&'a crate::cards::builders::EffectAst>,
    ) {
        match effect {
            crate::cards::builders::EffectAst::Coordination(nested) => {
                for effect in nested.effects() {
                    leaf_effects(effect, output);
                }
            }
            crate::cards::builders::EffectAst::Sequence { effects }
            | crate::cards::builders::EffectAst::CommaThen { effects }
            | crate::cards::builders::EffectAst::Coordinated { effects, .. } => {
                for effect in effects {
                    leaf_effects(effect, output);
                }
            }
            effect => output.push(effect),
        }
    }
    let mut effects = Vec::new();
    for effect in coordination.effects() {
        leaf_effects(effect, &mut effects);
    }
    let exile_effects = effects
        .into_iter()
        .filter(|effect| {
            matches!(
                effect,
                crate::cards::builders::EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action: crate::cards::builders::SubjectVerbActionAst::Exile { .. }
                            | crate::cards::builders::SubjectVerbActionAst::ExileAll { .. },
                        ..
                    }
                )
            )
        })
        .collect::<Vec<_>>();
    let [source_exile, battlefield_exile, graveyard_exile] = exile_effects.as_slice() else {
        panic!("expected source plus two exile-all effects, got {coordination:#?}");
    };
    assert!(matches!(
        source_exile,
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Exile { .. },
                ..
            }
        )
    ));
    let battlefield_filter = match battlefield_exile {
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::ExileAll {
                        filter,
                        face_down: false,
                    },
                ..
            },
        ) => filter,
        other => panic!("expected battlefield exile-all effect, got {other:#?}"),
    };
    assert!(battlefield_filter.card_types.contains(&CardType::Creature));
    assert!(
        battlefield_filter
            .card_types
            .contains(&CardType::Planeswalker)
    );
    assert!(mana_value_lte_void_counters(battlefield_filter));

    let graveyard_filter = match graveyard_exile {
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::ExileAll {
                        filter,
                        face_down: false,
                    },
                ..
            },
        ) => filter,
        other => panic!("expected graveyard exile-all effect, got {other:#?}"),
    };
    assert_eq!(graveyard_filter.zone, Some(Zone::Graveyard));
    assert!(graveyard_filter.card_types.contains(&CardType::Creature));
    assert!(
        graveyard_filter
            .card_types
            .contains(&CardType::Planeswalker)
    );
    assert!(mana_value_lte_void_counters(graveyard_filter));
}

#[test]
pub(super) fn rewrite_zone_counter_helpers_parse_half_starting_life_total_variants() {
    let your_tokens = lex_line("half your starting life total", 0)
        .expect("rewrite lexer should classify half-life value");
    let target_tokens = lex_line("half target player's starting life total rounded down", 0)
        .expect("rewrite lexer should classify rounded-down half-life value");

    assert_eq!(
        crate::effect_sentences::parse_half_starting_life_total_value(
            &your_tokens,
            crate::cards::builders::PlayerAst::Implicit,
        ),
        Some(crate::effect::Value::HalfStartingLifeTotalRoundedUp(
            crate::target::PlayerFilter::You,
        ))
    );
    assert_eq!(
        crate::effect_sentences::parse_half_starting_life_total_value(
            &target_tokens,
            crate::cards::builders::PlayerAst::Target,
        ),
        Some(crate::effect::Value::HalfStartingLifeTotalRoundedDown(
            crate::target::PlayerFilter::target_player(),
        ))
    );
}

#[test]
pub(super) fn rewrite_value_expr_parses_half_rounded_forest_count_and_y_pt_modifier() {
    let pt_tokens = lex_line("+X/+Y", 0).expect("rewrite lexer should classify +X/+Y");
    assert_eq!(token_word_refs(&pt_tokens), vec!["+X/+Y"]);

    let down_words = [
        "half", "the", "number", "of", "forests", "you", "control", "rounded", "down",
    ];
    let up_words = [
        "half", "the", "number", "of", "forests", "you", "control", "rounded", "up",
    ];
    let (down, down_used) =
        parse_value_expr_words(&down_words).expect("half rounded-down Forest count should parse");
    let (up, up_used) =
        parse_value_expr_words(&up_words).expect("half rounded-up Forest count should parse");

    assert_eq!(down_used, down_words.len());
    assert_eq!(up_used, up_words.len());
    assert!(matches!(down, Value::HalfRoundedDown(_)), "{down:?}");
    assert!(
        matches!(
            up,
            Value::HalfRoundedDown(ref inner) if matches!(inner.as_ref(), Value::Add(_, _))
        ),
        "{up:?}"
    );

    let anthem_tokens = lex_line(
        "enchanted creature gets +x/+y, where x is half the number of forests you control, rounded down, and y is half the number of forests you control, rounded up.",
        0,
    )
    .expect("Aspect-style anthem line should lex");
    let anthem = super::super::keyword_static::parse_anthem_line(&anthem_tokens)
        .expect("Aspect-style anthem line should parse without an error");
    assert!(
        anthem.is_some(),
        "Aspect-style anthem line should match anthem parser"
    );
}

#[test]
pub(super) fn rewrite_activation_helpers_cover_color_choice_mana_helpers() {
    let or_tokens =
        lex_line("{W}, {U}, or {B}", 0).expect("rewrite lexer should classify color choices");
    let combination_tokens = lex_line("any combination of {W}, {U}, or {R}", 0)
        .expect("rewrite lexer should classify any-combination mana");
    let land_filter_tokens = lex_line("that a land an opponent controls could produce", 0)
        .expect("rewrite lexer should classify land filter tail");

    assert_eq!(
        super::super::activation_helpers::parse_or_mana_color_choices(&or_tokens)
            .expect("or-choice mana colors should parse"),
        Some(vec![
            crate::color::Color::White,
            crate::color::Color::Blue,
            crate::color::Color::Black,
        ])
    );
    assert_eq!(
        super::super::activation_helpers::parse_any_combination_mana_colors(&combination_tokens)
            .expect("any-combination mana colors should parse"),
        Some(vec![
            crate::color::Color::White,
            crate::color::Color::Blue,
            crate::color::Color::Red,
        ])
    );
    assert!(matches!(
        super::super::activation_helpers::parse_land_could_produce_filter(&land_filter_tokens)
            .expect("land could produce tail should parse"),
        Some((filter, crate::effects::ManaTypeSource::MatchingLandsCouldProduce))
            if filter.card_types == vec![CardType::Land]
                && filter.controller == Some(crate::target::PlayerFilter::Opponent)
    ));
}

#[test]
pub(super) fn rewrite_activation_helpers_parse_add_mana_preserves_chosen_color_tail() {
    let tokens = lex_line("{R} or one mana of the chosen color", 0)
        .expect("rewrite lexer should classify chosen-color mana clause");

    assert!(matches!(
        super::super::activation_helpers::parse_add_mana(&tokens, None)
            .expect("chosen-color mana clause should parse"),
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                subject: crate::cards::builders::SubjectVerbSubjectAst {
                    player: crate::cards::builders::PlayerAst::Implicit,
                    ..
                },
                action: crate::cards::builders::SubjectVerbActionAst::AddManaChosenColor {
                    amount: crate::effect::Value::Fixed(1),
                    fixed_option: Some(crate::color::Color::Red),
                },
            }
        )
    ));
}

#[test]
pub(super) fn rewrite_activation_helpers_parse_add_mana_chooses_one_color_per_prior_object() {
    let tokens = lex_line("{B} or {G} for each permanent destroyed this way", 0)
        .expect("rewrite lexer should classify dynamic color-choice mana");

    match super::super::activation_helpers::parse_add_mana(&tokens, None)
        .expect("dynamic color-choice mana should parse")
    {
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::AddManaAnyColor {
                        amount,
                        available_colors: Some(colors),
                        ..
                    },
                ..
            },
        ) => {
            assert_eq!(
                colors,
                vec![crate::color::Color::Black, crate::color::Color::Green,]
            );
            assert!(matches!(
                amount.unhinted(),
                crate::effect::Value::PendingPriorEffectMetric(query)
                    if query.action
                        == Some(ironsmith_core::PriorEffectAction::Destroyed)
            ));
        }
        other => panic!("expected per-object color-choice mana, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_activation_helpers_parse_add_mana_scales_by_greatest_power_entered_this_turn()
{
    let tokens = lex_line(
        "{R} equal to the greatest power among creatures you control that entered this turn",
        0,
    )
    .expect("rewrite lexer should classify aggregate red mana clause");

    match super::super::activation_helpers::parse_add_mana(&tokens, None)
        .expect("aggregate red mana clause should parse")
    {
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::AddManaScaled { mana, amount },
                ..
            },
        ) => {
            assert_eq!(mana, vec![crate::mana::ManaSymbol::Red]);
            assert!(matches!(
                amount.unhinted(),
                crate::effect::Value::GreatestPower(filter)
                    if filter.card_types == vec![CardType::Creature]
                        && filter.controller == Some(crate::target::PlayerFilter::You)
                        && filter.entered_battlefield_this_turn
            ));
        }
        other => panic!("expected scaled red mana from greatest power, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_activation_helpers_parse_add_mana_wraps_instead_if_tail() {
    let tokens = lex_line(
        "{B}{B}{B}{B}{B} instead if there are seven or more cards in your graveyard",
        0,
    )
    .expect("rewrite lexer should classify conditional mana clause");

    let effect = super::super::activation_helpers::parse_add_mana(&tokens, None)
        .expect("conditional mana clause should parse");

    match effect {
        crate::cards::builders::EffectAst::Conditional {
            predicate: _,
            if_true,
            if_false,
        } => {
            assert!(if_false.is_empty());
            match if_true.as_slice() {
                [
                    crate::cards::builders::EffectAst::SubjectVerb(
                        crate::cards::builders::SubjectVerbEffectAst {
                            subject,
                            action: crate::cards::builders::SubjectVerbActionAst::AddMana { mana },
                        },
                    ),
                ] => {
                    assert_eq!(subject.player, crate::cards::builders::PlayerAst::Implicit);
                    assert_eq!(
                        mana.as_slice(),
                        &[
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                            crate::mana::ManaSymbol::Black,
                        ]
                    );
                }
                other => panic!("expected add-mana branch, got {other:?}"),
            }
        }
        other => panic!("expected conditional add-mana effect, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_activation_helpers_parse_add_mana_accepts_player_choice_tail_without_word_view()
 {
    let tokens = lex_line("one mana of any color that player chooses", 0)
        .expect("rewrite lexer should classify player-choice mana clause");

    assert!(matches!(
        super::super::activation_helpers::parse_add_mana(&tokens, None)
            .expect("player-choice mana clause should parse"),
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                subject: crate::cards::builders::SubjectVerbSubjectAst {
                    player: crate::cards::builders::PlayerAst::Implicit,
                    ..
                },
                action: crate::cards::builders::SubjectVerbActionAst::AddManaAnyColor {
                    amount: crate::effect::Value::Fixed(1),
                    available_colors: None,
                    distinct_colors: false,
                },
            }
        )
    ));
}

#[test]
pub(super) fn rewrite_activation_helpers_preserve_additional_any_combination_mana_amount() {
    let tokens = lex_line("an additional two mana in any combination of colors", 0)
        .expect("additional any-combination mana clause should lex");

    assert!(matches!(
        super::super::activation_helpers::parse_add_mana(&tokens, None)
            .expect("additional any-combination mana clause should parse"),
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::AddManaAnyColor {
                    amount: crate::effect::Value::Fixed(2),
                    available_colors: Some(colors),
                    distinct_colors: false,
                },
                ..
            }
        ) if colors == crate::color::Color::ALL.to_vec()
    ));
}

#[test]
pub(super) fn rewrite_activation_helpers_normalize_player_apostrophe_in_mana_pool_tail() {
    let tokens = lex_line("to that player's mana pool", 0)
        .expect("rewrite lexer should classify mana-pool tail");

    assert!(super::super::activation_helpers::is_mana_pool_tail_tokens(
        &tokens
    ));
}

#[test]
pub(super) fn rewrite_effect_sentence_parse_add_mana_wraps_instead_if_tail() {
    let tokens = lex_line(
        "Add {B}{B}{B}{B}{B} instead if there are seven or more cards in your graveyard",
        0,
    )
    .expect("rewrite lexer should classify mana sentence");

    let effects = parse_effect_sentence_lexed(&tokens).expect("mana sentence should parse");

    let [effect] = effects.as_slice() else {
        panic!("expected one conditional add-mana effect, got {effects:?}");
    };
    let (_, if_true, if_false) = conditional_effect_parts(effect);
    assert!(if_false.is_empty());
    let [
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                subject,
                action: crate::cards::builders::SubjectVerbActionAst::AddMana { mana },
            },
        ),
    ] = if_true
    else {
        panic!("expected add-mana branch, got {if_true:?}");
    };
    assert_eq!(subject.player, crate::cards::builders::PlayerAst::Implicit);
    assert_eq!(
        mana.as_slice(),
        &[
            crate::mana::ManaSymbol::Black,
            crate::mana::ManaSymbol::Black,
            crate::mana::ManaSymbol::Black,
            crate::mana::ManaSymbol::Black,
            crate::mana::ManaSymbol::Black,
        ]
    );
}

#[test]
pub(super) fn rewrite_effect_sentence_parse_add_mana_scales_by_greatest_power_entered_this_turn() {
    let tokens = lex_line(
        "Add {R} equal to the greatest power among creatures you control that entered this turn",
        0,
    )
    .expect("rewrite lexer should classify aggregate red mana sentence");

    let effects = parse_effect_sentence_lexed(&tokens).expect("mana sentence should parse");

    match effects.as_slice() {
        [
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    action:
                        crate::cards::builders::SubjectVerbActionAst::AddManaScaled { mana, amount },
                    ..
                },
            ),
        ] => {
            assert_eq!(mana.as_slice(), &[crate::mana::ManaSymbol::Red]);
            assert!(matches!(
                amount.unhinted(),
                crate::effect::Value::GreatestPower(filter)
                    if filter.card_types == vec![CardType::Creature]
                        && filter.controller == Some(crate::target::PlayerFilter::You)
                        && filter.entered_battlefield_this_turn
            ));
        }
        other => panic!("expected scaled red mana sentence, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_lexed_activation_condition_parser_handles_control_and_graveyard_conditions() {
    let graveyard = lex_line(
        "Activate only if there is an artifact card in your graveyard.",
        0,
    )
    .expect("rewrite lexer should classify graveyard condition");
    let control = lex_line("Activate only if you control three or more artifacts.", 0)
        .expect("rewrite lexer should classify control condition");
    let dynamic_control = lex_line(
        "Activate only if you control two or more artifact creatures.",
        0,
    )
    .expect("rewrite lexer should classify dynamic control condition");

    assert!(matches!(
        parse_activation_condition_lexed(&graveyard),
        Some(crate::ConditionExpr::CardInYourGraveyard { card_types, subtypes })
            if card_types == vec![CardType::Artifact] && subtypes.is_empty()
    ));
    assert!(matches!(
        parse_activation_condition_lexed(&control),
        Some(crate::ConditionExpr::PlayerHasAtLeast {
            player: crate::target::PlayerFilter::You,
            count: 3,
            ..
        })
    ));
    assert!(matches!(
        parse_activation_condition_lexed(&dynamic_control),
        Some(crate::ConditionExpr::PlayerHasAtLeast {
            player: crate::target::PlayerFilter::You,
            count: 2,
            filter,
        }) if filter.card_types == vec![CardType::Artifact, CardType::Creature]
    ));
}

#[test]
pub(super) fn rewrite_lexed_spell_filter_parser_preserves_native_shape() {
    let tokens = lex_line("face-down noncreature spells", 0)
        .expect("rewrite lexer should classify spell filter text");
    let filter = crate::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);

    assert_eq!(filter.face_down, Some(true));
    assert_eq!(filter.excluded_card_types, vec![CardType::Creature]);
}

#[test]
pub(super) fn rewrite_lexed_object_filter_tracks_spell_caster_and_origin_zone() {
    let tokens = lex_line("enchantment spells you cast from your hand", 0)
        .expect("rewrite lexer should classify spell grant filter text");
    let filter = crate::object_filters::parse_object_filter_lexed(&tokens, false)
        .expect("spell grant filter should parse");

    assert_eq!(filter.zone, Some(crate::zone::Zone::Hand));
    assert_eq!(filter.cast_by, Some(crate::target::PlayerFilter::You));
    assert_eq!(filter.owner, None);
    assert_eq!(filter.card_types, vec![CardType::Enchantment]);
}

#[test]
pub(super) fn rewrite_lexed_value_and_permission_helpers_match_existing_semantics() {
    let count_tokens = lex_line("equal to the number of creatures", 0)
        .expect("rewrite lexer should classify count-value clause");
    let permission_tokens = lex_line("You may cast it this turn", 0)
        .expect("rewrite lexer should classify permission clause");

    assert!(matches!(
        super::super::grammar::shared_util::value_semantics::parse_equal_to_number_of_filter_value(
            &count_tokens,
        ),
        Some(value) if matches!(value.unhinted(), crate::effect::Value::Count(filter)
            if filter.card_types == vec![CardType::Creature])
    ));
    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&permission_tokens),
        Ok(Some(
            crate::permission_helpers::PermissionClauseSpec::Tagged {
                player: crate::cards::builders::PlayerAst::You,
                allow_land: false,
                as_copy: false,
                without_paying_mana_cost: false,
                lifetime: crate::permission_helpers::PermissionLifetime::ThisTurn,
                ..
            }
        ))
    ));
}

pub(super) fn assert_source_counter_surface(value: &Value, expected_surface: &str) {
    let Value::CountersOn(spec, Some(CounterType::Charge)) = value.unhinted() else {
        panic!("expected charge counters on hinted source, got {value:?}");
    };
    let Some(crate::target::SourceReferenceSurface::ThisPermanentType(surface)) =
        spec.source_reference_surface()
    else {
        panic!("expected this-permanent source surface hint, got {value:?}");
    };
    assert_eq!(surface, expected_surface);
}

#[test]
pub(super) fn card_source_reference_context_registers_this_type_and_subtype_surfaces() {
    let context = crate::parse_context::ParseContext::for_fragment(
        "Opaline Bracers",
        vec![CardType::Artifact],
        vec![Subtype::Equipment],
        "",
    );
    assert_eq!(
        super::super::util::source_reference_surface_for_words_with_context(
            context.view(),
            &["this", "artifact"],
        ),
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            "this artifact".to_string()
        ))
    );
    assert_eq!(
        super::super::util::source_reference_surface_for_words_with_context(
            context.view(),
            &["this", "equipment"],
        ),
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            "this Equipment".to_string()
        ))
    );
    assert_eq!(
        super::super::util::source_reference_surface_for_words_with_context(
            context.view(),
            &["this", "adventure"],
        ),
        None
    );
}

#[test]
pub(super) fn object_filter_source_reference_preserves_this_subtype_surface_hint() {
    let tokens = lex_line("this Equipment", 0).expect("source-reference fixture should lex");
    let filter = crate::object_filters::parse_object_filter_lexed(&tokens, false)
        .expect("source-reference object filter should parse");
    assert!(filter.source, "expected source object filter: {filter:?}");
    assert_eq!(
        filter.source_surface,
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            "this Equipment".to_string()
        ))
    );
    assert_eq!(filter.description(), "this Equipment");
}

#[test]
pub(super) fn raw_this_source_surfaces_singularize_possessive_normalized_nouns() {
    assert_eq!(
        super::super::util::this_source_surface_for_words(&["this", "creatures"]),
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            "this creature".to_string()
        ))
    );
    assert_eq!(
        super::super::util::this_source_surface_for_words(&["thiss", "creatures"]),
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            "this creature".to_string()
        ))
    );
}

pub(super) fn assert_source_counter_surface_in_card_context(
    text: &str,
    _card_types: &[CardType],
    _subtypes: &[Subtype],
    expected_surface: &str,
) {
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify source counter reference value");
    let parsed =
        super::super::grammar::shared_util::value_semantics::parse_equal_to_number_of_counters_on_reference_value(&tokens)
            .expect("source counter reference value should parse");
    assert_source_counter_surface(&parsed, expected_surface);
}

#[test]
pub(super) fn source_counter_reference_values_preserve_this_type_surface_hints() {
    assert_source_counter_surface_in_card_context(
        "equal to the number of charge counters on this artifact",
        &[CardType::Artifact],
        &[],
        "this artifact",
    );
    assert_source_counter_surface_in_card_context(
        "equal to the number of charge counters on this Equipment",
        &[CardType::Artifact],
        &[Subtype::Equipment],
        "this Equipment",
    );
    assert_source_counter_surface_in_card_context(
        "equal to the number of charge counters on this Aura",
        &[CardType::Enchantment],
        &[Subtype::Aura],
        "this Aura",
    );
}

#[test]
pub(super) fn source_counter_reference_values_allow_bare_it_without_surface_hint() {
    let tokens = lex_line("equal to the number of charge counters on it", 0)
        .expect("rewrite lexer should classify bare source counter reference value");

    let parsed =
        super::super::grammar::shared_util::value_semantics::parse_equal_to_number_of_counters_on_reference_value(&tokens)
            .expect("bare source counter reference value should parse");
    assert!(matches!(
        parsed.unhinted(),
        Value::CountersOnSource(CounterType::Charge)
    ));
}

#[test]
pub(super) fn rewrite_grammar_add_mana_equal_amount_value_entrypoint_matches_parser_root_output() {
    let tokens = lex_line("equal to its toughness plus 2", 0)
        .expect("rewrite lexer should classify equal-amount value text");

    let parsed = crate::keyword_static::parse_add_mana_equal_amount_value(&tokens);
    let grammar_parsed =
        super::super::grammar::values::parse_add_mana_equal_amount_value_lexed(&tokens);

    assert_eq!(grammar_parsed, parsed);
    assert_eq!(
        grammar_parsed,
        Some(crate::effect::Value::Add(
            Box::new(crate::effect::Value::ToughnessOf(Box::new(
                crate::target::ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.key())
                    .with_surface_hint(crate::target::ChooseSpecSurfaceHint::SourceReference(
                        crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
                    )),
            ))),
            Box::new(crate::effect::Value::Fixed(2)),
        ))
    );
}

#[test]
pub(super) fn rewrite_grammar_object_filter_entrypoint_matches_parser_root_lexed_output() {
    let text = "creature card with mana value equal to 3";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comparison filter");

    let grammar =
        super::super::grammar::filters::reference_tag_stage::parse_object_filter_with_grammar_entrypoint_lexed(&lexed, false)
            .expect("grammar-owned object filter entrypoint should parse");
    let parser_root = crate::object_filters::parse_object_filter_lexed(&lexed, false)
        .expect("parser-root object filter entrypoint should parse");

    assert_eq!(format!("{grammar:?}"), format!("{parser_root:?}"));
}

#[test]
pub(super) fn rewrite_parser_root_nonlexed_object_filter_entrypoint_matches_grammar_lexed_output() {
    let tokens = lex_line("artifact card in your graveyard", 0)
        .expect("rewrite lexer should classify non-lexed object filter text");

    let parser_root =
        crate::grammar::filters::parse_object_filter_with_grammar_entrypoint(&tokens, false)
            .expect("parser-root non-lexed object filter entrypoint should parse");
    let grammar_lexed =
        super::super::grammar::filters::reference_tag_stage::parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false)
            .expect("grammar-owned lexed object filter entrypoint should parse");

    assert_eq!(format!("{parser_root:?}"), format!("{grammar_lexed:?}"));
}

#[test]
pub(super) fn rewrite_grammar_spell_filter_entrypoint_matches_parser_root_output() {
    let text = "creature spells with power or toughness 2 or less";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comparison spell filter");

    let grammar =
        super::super::grammar::filters::spell_filters::parse_spell_filter_with_grammar_entrypoint_lexed(
            &lexed,
        );
    let parser_root =
        crate::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed(&lexed);

    assert_eq!(format!("{grammar:?}"), format!("{parser_root:?}"));
}

#[test]
pub(super) fn rewrite_parser_root_nonlexed_spell_filter_entrypoint_matches_lexed_output() {
    let tokens = lex_line("face-down noncreature spells", 0)
        .expect("rewrite lexer should classify non-lexed spell filter text");

    let parser_root = crate::grammar::filters::parse_spell_filter_with_grammar_entrypoint(&tokens);
    let lexed = crate::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);

    assert_eq!(format!("{parser_root:?}"), format!("{lexed:?}"));
}

#[test]
pub(super) fn rewrite_lexed_cant_sentence_supports_next_turn_silence() {
    let text = "Each opponent can't cast instant or sorcery spells during that player's next turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify next-turn silence");

    let parsed =
        parse_cant_effect_sentence_lexed(&lexed).expect("lexed next-turn silence should parse");
    let sentence = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("sentence parser");

    assert!(
        parsed.is_some(),
        "expected next-turn silence helper to match"
    );
    assert!(
        !sentence.is_empty(),
        "expected sentence parser to produce next-turn silence effects"
    );
}

#[test]
pub(super) fn rewrite_effect_sentence_routes_cant_family_through_grammar_entrypoint() {
    let text = "Each opponent can't cast instant or sorcery spells during that player's next turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify next-turn silence");

    let grammar =
        super::super::grammar::effects::parse_cant_effect_sentence_with_grammar_entrypoint_lexed(
            &lexed,
        )
        .expect("grammar-owned cant sentence entrypoint should parse");
    let sentence = parse_effect_sentence_lexed(&lexed).expect("effect sentence parser");
    let grammar = grammar.unwrap_or_default();

    assert_eq!(format!("{sentence:?}"), format!("{grammar:?}"));
}

#[test]
pub(super) fn leading_if_result_restriction_yields_to_the_result_parser() {
    let text = "If the player doesn't, creatures they control can't attack you this turn.";
    let lexed = lex_line(text, 0).expect("result-dependent restriction should lex");

    let bare_restriction =
        parse_cant_effect_sentence_lexed(&lexed).expect("the cant-family probe should not fail");
    assert!(
        bare_restriction.is_none(),
        "the restriction parser must leave the leading result predicate intact: {bare_restriction:#?}"
    );

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("the generic result parser should own the complete sentence");
    let debug = format!("{parsed:#?}");
    assert!(
        debug.contains("IfResult")
            && debug.contains("DidNot")
            && debug.contains("AttackPlayer")
            && debug.contains("IteratedPlayer")
            && debug.contains("EndOfTurn"),
        "the result predicate, affected player, defender, and duration must remain typed: {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_cant_sentence_preserves_hyphenated_spell_filter_for_next_turn_silence()
{
    let text = "Each opponent can't cast non-Creature spells during that player's next turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify hyphenated next-turn silence");
    let parsed =
        parse_cant_effect_sentence_lexed(&lexed).expect("lexed next-turn silence should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("excluded_card_types: [Creature]"),
        "expected non-Creature spell filter to survive parsing, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_cant_sentence_supports_phase_out_until_next_upkeep() {
    let text = "Until your next upkeep, target permanent can't phase out.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify phase-out restriction");

    let parsed =
        parse_cant_effect_sentence_lexed(&lexed).expect("phase-out cant sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("YourNextUpkeep") && debug.contains("PhaseOut"),
        "expected phase-out restriction with next-upkeep duration, got {debug}"
    );
}

#[test]
pub(super) fn semantic_document_supports_proliferate_then_choose_permanents_phase_out() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Ripples of Potential")
        .card_types(vec![CardType::Instant]);
    let text = "Proliferate, then choose any number of permanents you control that had a counter put on them this way. Those permanents phase out.";
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)
        .expect("expected proliferate/phase-out line to parse and lower");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(
        debug.contains("TaggedEffect")
            && debug.contains("proliferated_this_way")
            && debug.contains("IsTaggedObject")
            && debug.contains("PhaseOutEffect"),
        "the later choice must consume the proliferate action's stable affected-object set: {debug}"
    );
}

#[test]
pub(super) fn semantic_document_supports_flash_cast_timing_cleanup_sacrifice() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Lightning Reflexes Timing Probe")
        .card_types(vec![CardType::Enchantment]);
    let text = "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.";
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)
        .expect("expected flash/next-cleanup line to parse and lower");

    let abilities = format!("{:#?}", definition.abilities);
    let spell = format!("{:#?}", definition.spell_effect);
    assert!(abilities.contains("Flash"), "{abilities}");
    assert!(
        spell.contains("SourceWasCast")
            && spell.contains("ThisSpellWasCastAtSorceryTiming")
            && spell.contains("BeginningOfNextCleanupStep")
            && spell.contains("SacrificeTargetEffect")
            && spell.contains("Source"),
        "the timing consequence must remain a conditional next-cleanup sacrifice of the resulting permanent: {spell}"
    );
}

#[test]
pub(super) fn semantic_document_supports_scavenge_granted_at_each_cards_mana_cost() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Dynamic Scavenge Grant Probe")
        .card_types(vec![CardType::Creature]);
    let text = "Each creature card in your graveyard has scavenge. The scavenge cost is equal to its mana cost.";
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)
        .expect("expected recipient-derived scavenge grant to parse and lower");
    let debug = format!("{:#?}", definition.abilities);
    assert!(
        debug.contains("GrantObjectAbilityForFilter")
            && debug.contains("source_mana_cost: true")
            && debug.contains("DynamicMana")
            && debug.contains("ExileSelf")
            && debug.contains("SorcerySpeed")
            && debug.contains("SourcePower")
            && debug.contains("Graveyard"),
        "expected a graveyard activated-ability grant with a recipient-derived mana cost: {debug}"
    );
}

#[test]
pub(super) fn rewrite_parse_target_phrase_preserves_hyphenated_filter_before_random_suffix() {
    let text = "target non-Vampire creature chosen at random";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify hyphenated random target phrase");
    let target = super::super::util::parse_target_phrase(&tokens)
        .expect("hyphenated random target should parse");
    let debug = format!("{target:?}");

    assert!(
        debug.contains("random: true"),
        "expected target to remain random, got {debug}"
    );
    assert!(
        debug.contains("card_types: [Creature]"),
        "expected creature filter in parsed target, got {debug}"
    );
    assert!(
        debug.contains("excluded_subtypes: [Vampire]"),
        "expected excluded Vampire subtype in parsed target, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_parse_target_phrase_supports_enchanted_player() {
    let tokens = lex_line("enchanted player", 0)
        .expect("rewrite lexer should classify enchanted player target phrase");
    let target = super::super::util::parse_target_phrase(&tokens)
        .expect("enchanted player target should parse");

    assert!(matches!(
        target,
        crate::cards::builders::TargetAst::Player(
            crate::target::PlayerFilter::TaggedPlayer(tag),
            _
        ) if tag.as_str() == "enchanted"
    ));
}

#[test]
pub(super) fn semantic_document_supports_next_turn_silence() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Sphinx's Decree")
        .card_types(vec![CardType::Sorcery]);

    let parsed = parse_text_to_semantic_document(
        builder,
        "Each opponent can't cast instant or sorcery spells during that player's next turn."
            .to_string(),
        false,
    );

    parsed.expect("expected semantic document parse to succeed");

    let tokens = lex_line(
        "Each opponent can't cast instant or sorcery spells during that player's next turn.",
        0,
    )
    .expect("rewrite lexer");
    let effects = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("next-turn restriction AST");
    let [crate::cards::builders::EffectAst::ForEachOpponent { effects }] = effects.as_slice()
    else {
        panic!("expected per-opponent restriction, got {effects:#?}");
    };
    let [crate::cards::builders::EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
        panic!("expected direct scheduled restriction, got {effects:#?}");
    };
    let crate::cards::builders::SubjectVerbActionAst::Cant {
        restriction:
            crate::effect::Restriction::CastSpellsMatching(
                crate::target::PlayerFilter::IteratedPlayer,
                _,
            ),
        start:
            crate::effect::RestrictionStart::NextTurn(crate::target::PlayerFilter::IteratedPlayer),
        duration: crate::effect::Until::EndOfTurn,
        ..
    } = &subject_verb.action
    else {
        panic!("expected next-turn Cant AST, got {subject_verb:#?}");
    };
}

#[test]
pub(super) fn rewrite_lexed_restriction_duration_handles_for_as_long_as_token_shapes() {
    let prefix = lex_line(
        "For as long as you control this, target creature can't attack.",
        0,
    )
    .expect("rewrite lexer should classify for-as-long-as prefix duration");
    let parsed = parse_restriction_duration_lexed(&prefix)
        .expect("prefix duration should parse")
        .expect("prefix duration should be present");
    assert_eq!(parsed.0, crate::effect::Until::YouStopControllingThis);
    assert_eq!(
        TokenWordView::new(&parsed.1).to_word_refs(),
        vec!["target", "creature", "cant", "attack"]
    );

    let suffix = lex_line(
        "Target creature can't attack for as long as this remains tapped.",
        0,
    )
    .expect("rewrite lexer should classify for-as-long-as suffix duration");
    let parsed = parse_restriction_duration_lexed(&suffix)
        .expect("suffix duration should parse")
        .expect("suffix duration should be present");
    assert_eq!(parsed.0, crate::effect::Until::SourceUntaps);
    assert_eq!(
        TokenWordView::new(&parsed.1).to_word_refs(),
        vec!["target", "creature", "cant", "attack"]
    );
}

pub(super) fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("rewrite audit should read source directory");
    for entry in entries {
        let entry = entry.expect("rewrite audit should read directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

#[test]
pub(super) fn rewrite_runtime_sources_do_not_reintroduce_token_bridge_helpers() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = {
        let compiler_local = manifest_dir.join("src/runtime_backend");
        if compiler_local.is_dir() {
            compiler_local
        } else {
            manifest_dir.join("../ironsmith-compiler/src/runtime_backend")
        }
    };
    let removed_helper_names = [
        format!("{}_{}", "compat_tokens_from", "lexed"),
        format!("{}_{}", "lexed_tokens_from", "compat"),
    ];
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);

    let mut offenders = Vec::new();
    for path in files {
        if path.ends_with("tests.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("rewrite audit should read source file");
        let relative = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .expect("rewrite audit should relativize source path")
            .display()
            .to_string();

        if removed_helper_names
            .iter()
            .any(|helper_name| source.contains(helper_name))
        {
            offenders.push(relative);
        }
    }

    assert!(
        offenders.is_empty(),
        "token bridge helpers should stay removed: {}",
        offenders.join(", ")
    );
}

#[test]
pub(super) fn rewrite_lexed_value_helpers_cover_offset_and_aggregate_counts() {
    let offset_tokens = lex_line("equal to the number of creatures plus 2", 0)
        .expect("rewrite lexer should classify offset count-value clause");
    let aggregate_tokens = lex_line("equal to the greatest power among creatures you control", 0)
        .expect("rewrite lexer should classify aggregate-value clause");
    let spells_cast_tokens = lex_line(
        "equal to the number of instant spells you cast this turn",
        0,
    )
    .expect("rewrite lexer should classify spells-cast count-value clause");

    let offset_value = super::super::grammar::shared_util::value_semantics::parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
        &offset_tokens,
    )
    .expect("offset count value should parse");
    assert!(matches!(
        offset_value.unhinted(),
        crate::effect::Value::Add(base, offset)
            if matches!(base.unhinted(), crate::effect::Value::Count(_))
                && matches!(**offset, crate::effect::Value::Fixed(2))
    ));
    assert!(matches!(
        super::super::grammar::shared_util::value_semantics::parse_equal_to_aggregate_filter_value(
            &aggregate_tokens,
        ),
        Some(value) if matches!(value.unhinted(), crate::effect::Value::GreatestPower(filter)
            if filter.card_types == vec![CardType::Creature]
                && filter.controller == Some(crate::target::PlayerFilter::You))
    ));
    let spells_value =
        super::super::grammar::shared_util::value_semantics::parse_equal_to_number_of_filter_value(
            &spells_cast_tokens,
        );
    assert!(matches!(
        spells_value,
        Some(value) if match value.unhinted() {
            crate::effect::Value::SpellsCastThisTurnMatching { player, filter, .. }
                if *player == crate::target::PlayerFilter::You
                    && filter.card_types == vec![CardType::Instant] => true,
            crate::effect::Value::TurnHistoryCount(
                ironsmith_core::TurnHistoryCount::SpellsCast { player, filter, .. },
            ) if *player == crate::target::PlayerFilter::You
                && filter.card_types == vec![CardType::Instant] => true,
            _ => false,
        }
    ));
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_cover_flash_and_free_cast_grants() {
    let flash_tokens = lex_line("You may cast creature spells as though they had flash", 0)
        .expect("rewrite lexer should classify flash permission clause");
    let free_cast_tokens = lex_line(
        "You may cast creature spells from your hand without paying their mana costs",
        0,
    )
    .expect("rewrite lexer should classify free-cast permission clause");
    let duration_free_cast_tokens = lex_line(
        "Until end of turn, you may cast spells from your hand without paying their mana costs",
        0,
    )
    .expect("rewrite lexer should classify duration-scoped free-cast permission clause");

    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&flash_tokens),
        Ok(Some(crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::permission_helpers::PermissionLifetime::Static,
        })) if spec == crate::model::CompilerGrantSpecCore::flash_to_spells_matching(
            crate::target::ObjectFilter {
                card_types: vec![CardType::Creature],
                ..crate::target::ObjectFilter::default()
            }
        )
    ));
    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&free_cast_tokens),
        Ok(Some(crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::permission_helpers::PermissionLifetime::Static,
        })) if !spec.filter.has_mana_cost
            && spec.filter.card_types == vec![CardType::Creature]
            && spec.zone == crate::zone::Zone::Hand
    ));
    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(
            &duration_free_cast_tokens
        ),
        Ok(Some(crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::permission_helpers::PermissionLifetime::UntilEndOfTurn,
        })) if !spec.filter.has_mana_cost
            && spec.zone == crate::zone::Zone::Hand
    ));
    assert!(matches!(
        super::super::permission_helpers::parse_cast_or_play_tagged_clause(
            &duration_free_cast_tokens
        ),
        Ok(Some(crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::GrantBySpec {
                    duration: crate::grant::GrantDuration::UntilEndOfTurn,
                    ..
                },
                ..
            }
        )))
    ));
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_parse_once_each_turn_top_library_source_exiled_type_grant()
 {
    let tokens = lex_line(
        "Once each turn, you may cast a spell from the top of your library if it shares a card type with a card exiled with this creature.",
        0,
    )
    .expect("rewrite lexer should classify once-per-turn top-library cast permission");

    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::permission_helpers::PermissionLifetime::Static,
        })) if spec.zone == crate::zone::Zone::Library
            && matches!(spec.grantable, crate::model::CompilerGrantableCore::PlayFrom)
            && spec.usage_limit == Some(crate::grant::GrantUsageLimit::OnceEachTurn)
            && spec.filter.tagged_constraints.iter().any(|constraint|
                constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                    && constraint.relation == crate::target::TaggedOpbjectRelation::SharesCardType
            )
    ));
}

#[test]
pub(super) fn rewrite_lexed_top_library_permissions_preserve_cast_and_land_domains() {
    fn parse_grant(line: &str) -> crate::model::CompilerGrantSpecCore {
        let tokens = lex_line(line, 0).expect("top-library permission should lex");
        match super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens) {
            Ok(Some(crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
                player: crate::cards::builders::PlayerAst::You,
                spec,
                lifetime: crate::permission_helpers::PermissionLifetime::Static,
            })) => spec,
            parsed => panic!("expected a static top-library grant for {line:?}, got {parsed:?}"),
        }
    }

    let dragon = parse_grant("You may cast Dragon spells from the top of your library.");
    assert_eq!(dragon.zone, crate::zone::Zone::Library);
    assert!(
        dragon
            .filter
            .subtypes
            .contains(&crate::types::Subtype::Dragon)
    );
    assert!(dragon.filter.excluded_card_types.contains(&CardType::Land));

    let artifact_or_colorless = parse_grant(
        "You may cast artifact spells and colorless spells from the top of your library.",
    );
    assert!(
        artifact_or_colorless
            .filter
            .excluded_card_types
            .contains(&CardType::Land),
        "cast-only union must exclude lands at its root: {:#?}",
        artifact_or_colorless.filter
    );
    assert!(
        artifact_or_colorless
            .filter
            .any_of
            .iter()
            .any(|branch| branch.card_types == [CardType::Artifact])
    );
    assert!(
        artifact_or_colorless
            .filter
            .any_of
            .iter()
            .any(|branch| branch.colorless)
    );

    let creature = parse_grant("You may cast creature spells from the top of your library.");
    assert_eq!(creature.filter.card_types, [CardType::Creature]);
    assert!(
        creature
            .filter
            .excluded_card_types
            .contains(&CardType::Land),
        "cast-only creature permission must retain its spell-domain constraint"
    );

    let mixed =
        parse_grant("You may play lands and cast Bird spells from the top of your library.");
    let land_branch = mixed
        .filter
        .any_of
        .iter()
        .find(|branch| branch.card_types == [CardType::Land])
        .expect("mixed permission should retain a land branch");
    assert_eq!(land_branch.zone, None);
    let bird_branch = mixed
        .filter
        .any_of
        .iter()
        .find(|branch| branch.subtypes.contains(&crate::types::Subtype::Bird))
        .expect("mixed permission should retain its Bird-spell branch");
    assert!(
        bird_branch.excluded_card_types.contains(&CardType::Land),
        "the cast branch of a mixed permission must remain nonland"
    );

    let unrestricted =
        parse_grant("You may play lands and cast spells from the top of your library.");
    assert_eq!(unrestricted.filter, crate::target::ObjectFilter::default());
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_preserve_until_next_turn_flash_grants() {
    let tokens = lex_line(
        "Until your next turn, you may cast sorcery spells as though they had flash",
        0,
    )
    .expect("rewrite lexer should classify until-next-turn flash permission clause");

    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::permission_helpers::PermissionLifetime::UntilYourNextTurn,
        })) if spec.filter.card_types == vec![CardType::Sorcery]
            && spec.zone == crate::zone::Zone::Hand
    ));

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("until-next-turn flash permission should parse as an effect");

    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    crate::cards::builders::SubjectVerbActionAst::GrantBySpec {
                        spec,
                        player: crate::cards::builders::PlayerAst::You,
                        duration: crate::grant::GrantDuration::UntilYourNextTurnEnd,
                    } if spec.filter.card_types == vec![CardType::Sorcery]
                        && spec.zone == crate::zone::Zone::Hand
                )
        )),
        "expected until-next-turn sorcery flash grant, got {effects:#?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_parse_temporary_graveyard_cast_grants() {
    let tokens = lex_line(
        "You may cast a creature spell from your graveyard this turn",
        0,
    )
    .expect("rewrite lexer should classify temporary graveyard-cast permission");

    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::permission_helpers::PermissionLifetime::ThisTurn,
        })) if spec.filter.card_types == vec![CardType::Creature]
            && spec.zone == crate::zone::Zone::Graveyard
    ));

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("temporary graveyard-cast permission should parse as an effect");
    fn has_temporary_creature_graveyard_grant(
        effects: &[crate::cards::builders::EffectAst],
    ) -> bool {
        effects.iter().any(|effect| match effect {
            crate::cards::builders::EffectAst::SubjectVerb(subject_verb) => matches!(
                &subject_verb.action,
                crate::cards::builders::SubjectVerbActionAst::GrantBySpec {
                    spec,
                    player:
                        crate::cards::builders::PlayerAst::You
                        | crate::cards::builders::PlayerAst::Implicit,
                    duration: crate::grant::GrantDuration::UntilEndOfTurn,
                } if spec.filter.card_types == vec![CardType::Creature]
                    && spec.zone == crate::zone::Zone::Graveyard
            ),
            crate::cards::builders::EffectAst::May { effects }
            | crate::cards::builders::EffectAst::MayByPlayer { effects, .. } => {
                has_temporary_creature_graveyard_grant(effects)
            }
            _ => false,
        })
    }

    assert!(
        has_temporary_creature_graveyard_grant(&effects),
        "expected temporary graveyard creature cast grant, got {effects:#?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_route_subject_filters_through_grammar_entrypoint() {
    let tokens = lex_line("You may cast creature spells as though they had flash", 0)
        .expect("rewrite lexer should classify flash permission clause");

    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::permission_helpers::PermissionLifetime::Static,
        })) if spec == crate::model::CompilerGrantSpecCore::flash_to_spells_matching(
            crate::target::ObjectFilter {
                card_types: vec![CardType::Creature],
                ..crate::target::ObjectFilter::default()
            }
        )
    ));
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_preserve_disjunctive_subject_filters_without_local_word_view()
 {
    let tokens = lex_line(
        "You may cast instant and sorcery spells as though they had flash",
        0,
    )
    .expect("rewrite lexer should classify disjunctive flash permission clause");

    let parsed = super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens)
        .expect("permission clause should parse")
        .expect("permission clause should build a grant spec");

    match parsed {
        crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime,
        } => {
            assert_eq!(player, crate::cards::builders::PlayerAst::You);
            assert_eq!(
                lifetime,
                crate::permission_helpers::PermissionLifetime::Static
            );
            assert_eq!(spec.filter.any_of.len(), 2);
            assert!(
                spec.filter
                    .any_of
                    .iter()
                    .any(|filter| filter.card_types == vec![CardType::Instant])
            );
            assert!(
                spec.filter
                    .any_of
                    .iter()
                    .any(|filter| filter.card_types == vec![CardType::Sorcery])
            );
        }
        other => panic!("expected disjunctive flash grant, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_preserve_conjunctive_artifact_creature_subject_filters()
 {
    let tokens = lex_line(
        "You may cast artifact creature spells from your graveyard",
        0,
    )
    .expect("rewrite lexer should classify artifact creature permission clause");

    let parsed = super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens)
        .expect("permission clause should parse")
        .expect("permission clause should build a grant spec");

    match parsed {
        crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime,
        } => {
            assert_eq!(player, crate::cards::builders::PlayerAst::You);
            assert_eq!(
                lifetime,
                crate::permission_helpers::PermissionLifetime::Static
            );
            assert_eq!(spec.zone, crate::zone::Zone::Graveyard);
            assert_eq!(spec.filter.card_types, Vec::<CardType>::new());
            assert_eq!(
                spec.filter.all_card_types,
                vec![CardType::Artifact, CardType::Creature]
            );
        }
        other => panic!("expected graveyard artifact creature cast grant, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_route_free_cast_spell_filters_through_grammar_entrypoint()
 {
    let tokens = lex_line(
        "You may cast creature spells from your hand without paying their mana costs",
        0,
    )
    .expect("rewrite lexer should classify free-cast permission clause");

    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(crate::permission_helpers::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::permission_helpers::PermissionLifetime::Static,
        })) if !spec.filter.has_mana_cost
            && spec.filter.card_types == vec![CardType::Creature]
            && spec.zone == crate::zone::Zone::Hand
    ));
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_route_singular_hand_free_casts_to_one_shot_effect() {
    let tokens = lex_line(
        "You may cast a spell with mana value 3 or less from your hand without paying its mana cost",
        0,
    )
    .expect("rewrite lexer should classify singular free-cast permission clause");

    assert!(
        matches!(
            super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
            Ok(None)
        ),
        "singular hand free-cast permissions should not become static grants"
    );

    let effects =
        parse_effect_sentence_lexed(&tokens).expect("singular free-cast effect should parse");

    let (player, filter, zone) = match effects.as_slice() {
        [
            crate::cards::builders::EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                player,
                filter,
                zone,
                ..
            },
        ] => (player, filter, zone),
        [
            crate::cards::builders::EffectAst::MayByPlayer {
                player: crate::cards::builders::PlayerAst::You,
                effects,
            },
        ] => match effects.as_slice() {
            [
                crate::cards::builders::EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                    player,
                    filter,
                    zone,
                    ..
                },
            ] => (player, filter, zone),
            _ => panic!("expected nested singular hand free-cast effect, got {effects:#?}"),
        },
        _ => panic!("expected singular hand free-cast effect, got {effects:#?}"),
    };
    assert!(matches!(
        player,
        crate::cards::builders::PlayerAst::Implicit | crate::cards::builders::PlayerAst::You
    ));
    assert_eq!(*zone, crate::zone::Zone::Hand);
    assert_eq!(filter.excluded_card_types, vec![CardType::Land]);
    assert_eq!(
        filter.mana_value,
        Some(crate::filter::Comparison::LessThanOrEqual(3))
    );
}

#[test]
pub(super) fn rewrite_lexed_parse_commander_command_zone_free_cast_clause() {
    let tokens = lex_line(
        "You may cast your commander from the command zone without paying its mana cost",
        0,
    )
    .expect("rewrite lexer should classify commander command-zone free-cast clause");

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("commander command-zone free-cast clause should parse");

    let (player, filter, zone) = match effects.as_slice() {
        [
            crate::cards::builders::EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                player,
                filter,
                zone,
                ..
            },
        ] => (player, filter, zone),
        [
            crate::cards::builders::EffectAst::MayByPlayer {
                player: crate::cards::builders::PlayerAst::You,
                effects,
            },
        ] => match effects.as_slice() {
            [
                crate::cards::builders::EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                    player,
                    filter,
                    zone,
                    ..
                },
            ] => (player, filter, zone),
            _ => {
                panic!("expected nested commander command-zone free-cast effect, got {effects:#?}")
            }
        },
        _ => panic!("expected commander command-zone free-cast effect, got {effects:#?}"),
    };
    assert!(matches!(
        player,
        crate::cards::builders::PlayerAst::Implicit | crate::cards::builders::PlayerAst::You
    ));
    assert_eq!(*zone, crate::zone::Zone::Command);
    assert!(
        filter.is_commander,
        "expected commander filter, got {filter:#?}"
    );
    assert_eq!(filter.owner, Some(crate::target::PlayerFilter::You));
}

#[test]
pub(super) fn rewrite_lexed_parse_cast_target_graveyard_without_paying_mana_cost() {
    let tokens = lex_line(
        "Cast target instant, sorcery, or artifact card from your graveyard without paying its mana cost",
        0,
    )
    .expect("rewrite lexer should classify targeted graveyard free-cast clause");

    let effects =
        parse_effect_sentence_lexed(&tokens).expect("targeted graveyard free-cast should parse");

    assert!(
        matches!(
            effects.as_slice(),
            [crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    subject: crate::cards::builders::SubjectVerbSubjectAst {
                        role: crate::cards::builders::SubjectVerbRoleAst::Actor,
                        player: crate::cards::builders::PlayerAst::Implicit,
                    },
                    action: crate::cards::builders::SubjectVerbActionAst::CastTagged {
                        player: crate::cards::builders::PlayerAst::Implicit,
                        allow_land: false,
                        as_copy: false,
                        without_paying_mana_cost: true,
                        ..
                    },
                },
            )]
        ),
        "expected targeted graveyard free-cast CastTagged effect, got {effects:#?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_parse_counterpoint_followup_clause_with_tagged_mana_value_gate() {
    let tokens = lex_line(
        "You may cast a creature, instant, sorcery, or planeswalker spell from your graveyard with mana value less than or equal to that spell's mana value without paying its mana cost",
        0,
    )
    .expect("rewrite lexer should classify Counterpoint follow-up clause");

    let token_words = crate::lexer::token_word_refs(&tokens);
    assert!(
        super::super::permission_helpers::parse_cast_or_play_tagged_clause(&tokens)
            .expect("Counterpoint follow-up clause should not throw parser errors")
            .is_some(),
        "expected cast/play helper to parse Counterpoint follow-up clause; words={token_words:?}"
    );

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("Counterpoint follow-up clause should parse as a supported effect");

    let (player, filter, zone) = match effects.as_slice() {
        [
            crate::cards::builders::EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                player,
                filter,
                zone,
                ..
            },
        ] => (player, filter, zone),
        [
            crate::cards::builders::EffectAst::MayByPlayer {
                player: crate::cards::builders::PlayerAst::You,
                effects,
            },
        ] => match effects.as_slice() {
            [
                crate::cards::builders::EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                    player,
                    filter,
                    zone,
                    ..
                },
            ] => (player, filter, zone),
            _ => panic!("expected nested free-cast effect, got {effects:#?}"),
        },
        _ => panic!("expected free-cast effect, got {effects:#?}"),
    };

    assert!(matches!(
        player,
        crate::cards::builders::PlayerAst::Implicit | crate::cards::builders::PlayerAst::You
    ));
    assert_eq!(*zone, crate::zone::Zone::Graveyard);
    let has_type = |card_type: crate::types::CardType| {
        filter.card_types.contains(&card_type)
            || filter
                .any_of
                .iter()
                .any(|branch| branch.card_types.contains(&card_type))
    };
    assert!(has_type(crate::types::CardType::Creature), "{filter:#?}");
    assert!(has_type(crate::types::CardType::Instant), "{filter:#?}");
    assert!(has_type(crate::types::CardType::Sorcery), "{filter:#?}");
    assert!(
        !has_type(crate::types::CardType::Artifact),
        "counter reference should not add artifact to the castable spell filter: {filter:#?}"
    );
    assert!(
        has_type(crate::types::CardType::Planeswalker),
        "{filter:#?}"
    );
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG)
                && constraint.relation == crate::filter::TaggedOpbjectRelation::ManaValueLteTagged
        }),
        "expected mana-value-to-tagged constraint, got {filter:#?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_parse_glamdring_trigger_clause_with_damage_value_gate() {
    let tokens = lex_line(
        "Cast an instant or sorcery spell from your hand with mana value less than or equal to that damage without paying its mana cost",
        0,
    )
    .expect("rewrite lexer should classify Glamdring cast clause");

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("Glamdring cast clause should parse as a supported effect");

    let (player, filter, zone) = match effects.as_slice() {
        [
            crate::cards::builders::EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                player,
                filter,
                zone,
                ..
            },
        ] => (player, filter, zone),
        _ => panic!("expected one-shot hand free-cast effect, got {effects:#?}"),
    };

    assert!(matches!(
        player,
        crate::cards::builders::PlayerAst::Implicit | crate::cards::builders::PlayerAst::You
    ));
    assert_eq!(*zone, crate::zone::Zone::Hand);
    assert!(
        filter.card_types.contains(&crate::types::CardType::Instant),
        "{filter:#?}"
    );
    assert!(
        filter.card_types.contains(&crate::types::CardType::Sorcery),
        "{filter:#?}"
    );
    assert!(matches!(
        filter.mana_value.as_ref(),
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value))
            if *value.as_ref().unhinted()
                == crate::effect::Value::EventValue(crate::effect::EventValueSpec::Amount)
    ));
}

#[test]
pub(super) fn rewrite_lexed_parse_surtland_elementalist_trigger_clause_without_mana_value_gate() {
    let tokens = lex_line(
        "Cast an instant or sorcery spell from your hand without paying its mana cost",
        0,
    )
    .expect("rewrite lexer should classify Surtland Elementalist cast clause");

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("Surtland Elementalist cast clause should parse as a supported effect");

    let (player, filter, zone) = match effects.as_slice() {
        [
            crate::cards::builders::EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                player,
                filter,
                zone,
                ..
            },
        ] => (player, filter, zone),
        _ => panic!("expected one-shot hand free-cast effect, got {effects:#?}"),
    };

    assert!(matches!(
        player,
        crate::cards::builders::PlayerAst::Implicit | crate::cards::builders::PlayerAst::You
    ));
    assert_eq!(*zone, crate::zone::Zone::Hand);
    assert!(filter.card_types.contains(&crate::types::CardType::Instant));
    assert!(filter.card_types.contains(&crate::types::CardType::Sorcery));
    assert_eq!(filter.mana_value, None);
}

#[test]
pub(super) fn rewrite_lexed_parse_brain_in_a_jar_free_cast_clause_with_counter_value_gate() {
    let tokens = lex_line(
        "Cast an instant or sorcery spell with mana value equal to the number of charge counters on this artifact from your hand without paying its mana cost",
        0,
    )
    .expect("rewrite lexer should classify Brain in a Jar cast clause");

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("Brain in a Jar cast clause should parse as a supported effect");

    let (filter, zone) = match effects.as_slice() {
        [
            crate::cards::builders::EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                filter,
                zone,
                ..
            },
        ] => (filter, zone),
        _ => panic!("expected one-shot counter-gated hand free-cast effect, got {effects:#?}"),
    };

    assert_eq!(*zone, crate::zone::Zone::Hand);
    let has_type = |card_type: crate::types::CardType| {
        filter.card_types.contains(&card_type)
            || filter
                .any_of
                .iter()
                .any(|branch| branch.card_types.contains(&card_type))
    };
    let has_counter_gate = |filter: &crate::cards::builders::ObjectFilter| {
        filter.mana_value_eq_counters_on_source == Some(crate::object::CounterType::Charge)
            || matches!(
                filter.mana_value.as_ref(),
                Some(crate::filter::Comparison::EqualExpr(value))
                    if matches!(
                        value.unhinted(),
                        crate::effect::Value::CountersOn(
                            _,
                            Some(crate::object::CounterType::Charge)
                        )
                    )
            )
            || filter.any_of.iter().any(|branch| {
                branch.mana_value_eq_counters_on_source == Some(crate::object::CounterType::Charge)
            })
    };
    assert!(has_type(crate::types::CardType::Instant), "{filter:#?}");
    assert!(has_type(crate::types::CardType::Sorcery), "{filter:#?}");
    assert!(
        !has_type(crate::types::CardType::Artifact),
        "counter reference should not add artifact to the castable spell filter: {filter:#?}"
    );
    assert!(
        has_counter_gate(filter),
        "expected charge-counter mana-value gate, got {filter:#?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_parse_glamdring_static_clause_keeps_first_strike_and_anthem() {
    let tokens = lex_line(
        "Equipped creature has first strike and gets +1/+0 for each instant and sorcery card in your graveyard",
        0,
    )
    .expect("rewrite lexer should classify Glamdring static clause");

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("Glamdring static clause should parse as static ability");

    let debug = format!("{parsed:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("firststrike") && debug.contains("anthem"),
        "expected static clause to keep both first strike and anthem, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_cover_until_next_turn_tagged_play() {
    let tokens = lex_line("Until the end of your next turn, you may play that card", 0)
        .expect("rewrite lexer should classify until-next-turn permission clause");

    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(
            crate::permission_helpers::PermissionClauseSpec::Tagged {
                player: crate::cards::builders::PlayerAst::You,
                allow_land: true,
                as_copy: false,
                without_paying_mana_cost: false,
                lifetime: crate::permission_helpers::PermissionLifetime::UntilYourNextTurn,
                ..
            }
        ))
    ));
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_distinguish_next_end_step_from_next_turn() {
    let tokens = lex_line("Until your next end step, you may play that card", 0)
        .expect("rewrite lexer should classify next-end-step permission clause");

    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(
            crate::permission_helpers::PermissionClauseSpec::Tagged {
                player: crate::cards::builders::PlayerAst::You,
                allow_land: true,
                as_copy: false,
                without_paying_mana_cost: false,
                lifetime: crate::permission_helpers::PermissionLifetime::UntilYourNextEndStep,
                ..
            }
        ))
    ));

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("next-end-step tagged play permission should parse as an effect");
    assert!(effects.iter().any(|effect| matches!(
        effect,
        crate::cards::builders::EffectAst::SubjectVerb(subject_verb)
            if matches!(
                &subject_verb.action,
                crate::cards::builders::SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                    allow_land: true,
                    until_next_end_step: true,
                    ..
                }
            )
    )));
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_keep_one_shared_play_for_tagged_pool() {
    let tokens = lex_line(
        "Until your next end step, you may play one of those cards",
        0,
    )
    .expect("one-of tagged permission should lex");

    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(
            crate::permission_helpers::PermissionClauseSpec::Tagged {
                lifetime: crate::permission_helpers::PermissionLifetime::UntilYourNextEndStep,
                max_plays: Some(1),
                ..
            }
        ))
    ));

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("one-of tagged permission should parse as an effect");
    assert!(effects.iter().any(|effect| matches!(
        effect,
        crate::cards::builders::EffectAst::SubjectVerb(subject_verb)
            if matches!(
                &subject_verb.action,
                crate::cards::builders::SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                    until_next_end_step: true,
                    max_plays: Some(1),
                    ..
                }
            )
    )));
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_cover_until_next_turn_tagged_cast() {
    let tokens = lex_line("Until the end of your next turn, you may cast that card", 0)
        .expect("rewrite lexer should classify until-next-turn cast permission clause");

    assert!(matches!(
        super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens),
        Ok(Some(
            crate::permission_helpers::PermissionClauseSpec::Tagged {
                player: crate::cards::builders::PlayerAst::You,
                allow_land: false,
                as_copy: false,
                without_paying_mana_cost: false,
                lifetime: crate::permission_helpers::PermissionLifetime::UntilYourNextTurn,
                ..
            }
        ))
    ));

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("until-next-turn tagged cast permission should parse as an effect");
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    crate::cards::builders::SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                        player:
                            crate::cards::builders::PlayerAst::You
                            | crate::cards::builders::PlayerAst::Implicit,
                        allow_land: false,
                        ..
                    }
                )
        )),
        "expected until-next-turn tagged cast permission effect, got {effects:#?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_cover_until_next_turn_tagged_cast_with_any_color_mana()
 {
    let tokens = lex_line(
        "Until the end of your next turn, you may cast that card and you may spend mana as though it were mana of any color to cast that spell",
        0,
    )
    .expect("rewrite lexer should classify until-next-turn cast permission with any-color mana");

    let effects = parse_effect_sentence_lexed(&tokens).expect(
        "until-next-turn tagged cast permission with any-color mana should parse as an effect",
    );
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            crate::cards::builders::EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    crate::cards::builders::SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                        allow_land: false,
                        allow_any_color_for_cast:
                            ironsmith_core::value_model::ManaSpendMode::AnyColor,
                        ..
                    }
                )
        )),
        "expected until-next-turn tagged cast permission effect with any-color mana, got {effects:#?}"
    );
}

#[test]
pub(super) fn rewrite_token_primitives_cover_turn_duration_prefix_and_suffix_phrases() {
    let prefixed = lex_line("Until the end of your next turn, you may play that card", 0)
        .expect("rewrite lexer should classify prefixed duration phrase");
    let suffixed = lex_line("Target creature can't attack this turn", 0)
        .expect("rewrite lexer should classify suffixed duration phrase");

    let (prefix_duration, prefix_remainder) =
        super::super::token_primitives::parse_turn_duration_prefix(&prefixed)
            .expect("prefixed duration should parse");
    let (suffix_remainder, suffix_duration) =
        super::super::token_primitives::parse_turn_duration_suffix(&suffixed)
            .expect("suffixed duration should parse");

    assert_eq!(
        prefix_duration,
        super::super::token_primitives::TurnDurationPhrase::UntilYourNextTurnEnd
    );
    assert_eq!(
        TokenWordView::new(prefix_remainder).to_word_refs(),
        vec!["you", "may", "play", "that", "card"]
    );
    assert_eq!(
        suffix_duration,
        super::super::token_primitives::TurnDurationPhrase::ThisTurn
    );
    assert_eq!(
        TokenWordView::new(suffix_remainder).to_word_refs(),
        vec!["target", "creature", "cant", "attack"]
    );
}

#[test]
pub(super) fn rewrite_token_primitives_split_comma_then_with_bounded_parser() {
    let tokens = lex_line("Draw a card, then discard a card.", 0)
        .expect("rewrite lexer should classify comma-then sentence");

    let (head, tail) = super::super::lexer::LexedClause::new(&tokens)
        .split_comma_then()
        .expect("comma-then splitter should find boundary");

    assert_eq!(token_word_refs(head.tokens()), vec!["Draw", "a", "card"]);
    assert_eq!(token_word_refs(tail.tokens()), vec!["discard", "a", "card"]);
}

#[test]
pub(super) fn rewrite_token_primitives_cover_simple_restriction_duration_boundaries() {
    let prefixed = lex_line("Until end of combat, target creature gains menace", 0)
        .expect("rewrite lexer should classify combat duration prefix");
    let suffixed = lex_line(
        "Target creature can't attack during its controller's next untap step",
        0,
    )
    .expect("rewrite lexer should classify untap-step duration suffix");
    let forever = lex_line("That player can't gain life for the rest of the game", 0)
        .expect("rewrite lexer should classify forever duration suffix");

    let (prefix_duration, prefix_remainder) =
        super::super::token_primitives::parse_simple_restriction_duration_prefix(&prefixed)
            .expect("combat duration prefix should parse");
    let (suffix_remainder, suffix_duration) =
        super::super::token_primitives::parse_simple_restriction_duration_suffix(&suffixed)
            .expect("untap-step duration suffix should parse");
    let (forever_remainder, forever_duration) =
        super::super::token_primitives::parse_simple_restriction_duration_suffix(&forever)
            .expect("forever duration suffix should parse");

    assert_eq!(prefix_duration, crate::effect::Until::EndOfCombat);
    assert_eq!(
        TokenWordView::new(prefix_remainder).to_word_refs(),
        vec!["target", "creature", "gains", "menace"]
    );
    assert_eq!(
        suffix_duration,
        crate::effect::Until::ControllersNextUntapStep
    );
    assert_eq!(
        TokenWordView::new(suffix_remainder).to_word_refs(),
        vec!["target", "creature", "cant", "attack"]
    );
    assert_eq!(forever_duration, crate::effect::Until::Forever);
    assert_eq!(
        TokenWordView::new(forever_remainder).to_word_refs(),
        vec!["that", "player", "cant", "gain", "life"]
    );
}

#[test]
pub(super) fn rewrite_token_primitives_cover_bare_value_comparison_phrases() {
    let equal = lex_line("equal to 3", 0).expect("rewrite lexer should classify bare equality");
    let not_equal =
        lex_line("not equal to 3", 0).expect("rewrite lexer should classify bare inequality");
    let less_than =
        lex_line("less than 3", 0).expect("rewrite lexer should classify bare less-than");
    let greater_equal = lex_line("greater than or equal to 3", 0)
        .expect("rewrite lexer should classify bare greater-or-equal");

    let (equal_op, equal_remainder) =
        super::super::token_primitives::parse_value_comparison_tokens(&equal)
            .expect("bare equality should parse");
    let (not_equal_op, not_equal_remainder) =
        super::super::token_primitives::parse_value_comparison_tokens(&not_equal)
            .expect("bare inequality should parse");
    let (less_than_op, less_than_remainder) =
        super::super::token_primitives::parse_value_comparison_tokens(&less_than)
            .expect("bare less-than should parse");
    let (greater_equal_op, greater_equal_remainder) =
        super::super::token_primitives::parse_value_comparison_tokens(&greater_equal)
            .expect("bare greater-or-equal should parse");

    assert_eq!(equal_op, crate::effect::ValueComparisonOperator::Equal);
    assert_eq!(
        TokenWordView::new(equal_remainder).to_word_refs(),
        vec!["3"]
    );
    assert_eq!(
        not_equal_op,
        crate::effect::ValueComparisonOperator::NotEqual
    );
    assert_eq!(
        TokenWordView::new(not_equal_remainder).to_word_refs(),
        vec!["3"]
    );
    assert_eq!(
        less_than_op,
        crate::effect::ValueComparisonOperator::LessThan
    );
    assert_eq!(
        TokenWordView::new(less_than_remainder).to_word_refs(),
        vec!["3"]
    );
    assert_eq!(
        greater_equal_op,
        crate::effect::ValueComparisonOperator::GreaterThanOrEqual
    );
    assert_eq!(
        TokenWordView::new(greater_equal_remainder).to_word_refs(),
        vec!["3"]
    );
}

#[test]
pub(super) fn rewrite_values_comparison_parser_handles_suffix_forms_directly() {
    let suffix = lex_line("3 or less", 0).expect("rewrite lexer should classify suffix comparison");
    let prefixed_suffix = lex_line("is 4 or more", 0)
        .expect("rewrite lexer should classify prefixed suffix comparison");

    let (suffix_op, suffix_operand) =
        super::super::grammar::values::parse_value_comparison_tokens(&suffix)
            .expect("direct values comparison parser should accept suffix form");
    let (prefixed_op, prefixed_operand) =
        super::super::grammar::values::parse_value_comparison_tokens(&prefixed_suffix)
            .expect("direct values comparison parser should accept prefixed suffix form");

    assert_eq!(
        suffix_op,
        crate::effect::ValueComparisonOperator::LessThanOrEqual
    );
    assert_eq!(TokenWordView::new(suffix_operand).to_word_refs(), vec!["3"]);
    assert_eq!(
        prefixed_op,
        crate::effect::ValueComparisonOperator::GreaterThanOrEqual
    );
    assert_eq!(
        TokenWordView::new(prefixed_operand).to_word_refs(),
        vec!["4"]
    );
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_cover_or_less_conditional_free_casts() {
    let tokens = lex_line(
        "Cast that card without paying its mana cost if its mana value is 3 or less",
        0,
    )
    .expect("rewrite lexer should classify conditional free-cast permission");

    let parsed = super::super::permission_helpers::parse_cast_or_play_tagged_clause(&tokens)
        .expect("conditional free-cast clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("LessThanOrEqual"), "{debug}");
    assert!(debug.contains("Fixed(3)"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_preserve_any_color_cast_suffix() {
    let tokens = lex_line(
        "You may play that card this turn and mana of any type can be spent to cast it",
        0,
    )
    .expect("rewrite lexer should classify tagged permission with mana-spend suffix");

    let parsed = super::super::permission_helpers::parse_cast_or_play_tagged_clause(&tokens)
        .expect("tagged permission clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("GrantPlayTaggedUntilEndOfTurn"), "{debug}");
    assert!(
        debug.contains("allow_any_color_for_cast: AnyType"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_preserve_until_next_end_step_any_color_cast_suffix()
{
    let tokens = lex_line(
        "Until your next end step, you may play those cards, and mana of any type can be spent to cast those spells",
        0,
    )
    .expect("rewrite lexer should classify plural tagged permission with mana-spend suffix");

    let parsed = super::super::permission_helpers::parse_cast_or_play_tagged_clause(&tokens)
        .expect("tagged permission clause should parse")
        .expect("tagged permission clause should produce an effect");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("GrantPlayTaggedUntilYourNextTurn"),
        "{debug}"
    );
    assert!(debug.contains("allow_land: true"), "{debug}");
    assert!(
        debug.contains("allow_any_color_for_cast: AnyType"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_parse_while_exiled_tail_lifetime() {
    let tokens = lex_line(
        "cast that card for as long as it remains exiled and mana of any type can be spent to cast that spell",
        0,
    )
    .expect("rewrite lexer should classify while-exiled tagged cast permission");

    let parsed = super::super::permission_helpers::parse_cast_or_play_tagged_clause(&tokens)
        .expect("while-exiled tagged permission should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("GrantPlayTaggedForAsLongAsExiled"),
        "{debug}"
    );
    assert!(
        debug.contains("allow_any_color_for_cast: AnyType"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_parse_while_exiled_you_may_spend_mana_suffix() {
    let tokens = lex_line(
        "You may cast that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell",
        0,
    )
    .expect("rewrite lexer should classify while-exiled tagged cast permission");

    let parsed = super::super::permission_helpers::parse_cast_or_play_tagged_clause(&tokens)
        .expect("while-exiled tagged permission should parse")
        .expect("while-exiled tagged permission should produce a typed effect");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("GrantPlayTaggedForAsLongAsExiled"),
        "{debug}"
    );
    assert!(
        debug.contains("allow_any_color_for_cast: AnyColor")
            || debug.contains("allow_any_color_for_cast: AnyType"),
        "{debug}"
    );
    let sentence = parse_effect_sentence_lexed(&tokens)
        .expect("while-exiled tagged permission should parse through the public sentence route");
    assert!(!sentence.is_empty(), "{sentence:#?}");
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_parse_while_exiled_look_then_permanent_spells() {
    let tokens = lex_line(
        "For as long as those cards remain exiled, you may look at them, you may cast permanent spells from among them, and you may spend mana as though it were mana of any color to cast those spells",
        0,
    )
    .expect("rewrite lexer should classify plural while-exiled tagged permission");
    let inner_tokens = lex_line(
        "For as long as those cards remain exiled, you may cast permanent spells from among them, and you may spend mana as though it were mana of any color to cast those spells",
        0,
    )
    .expect("rewrite lexer should classify inner plural while-exiled permission");
    let inner = super::super::permission_helpers::parse_permission_clause_spec(&inner_tokens)
        .expect("inner plural while-exiled permission spec should parse");
    assert!(inner.is_some(), "inner permission spec returned None");
    let inner_debug = format!("{:?}", inner.as_ref().unwrap());
    let inner_effect =
        super::super::permission_helpers::parse_cast_or_play_tagged_clause(&inner_tokens)
            .expect("inner plural while-exiled permission effect should parse");
    let inner_no_suffix_tokens = lex_line(
        "For as long as those cards remain exiled, you may cast permanent spells from among them",
        0,
    )
    .expect("rewrite lexer should classify inner no-suffix permission");
    let inner_no_suffix_effect =
        super::super::permission_helpers::parse_cast_or_play_tagged_clause(&inner_no_suffix_tokens)
            .expect("inner no-suffix permission effect should parse");
    let inner_no_suffix_spec =
        super::super::permission_helpers::parse_permission_clause_spec(&inner_no_suffix_tokens)
            .expect("inner no-suffix spec should parse");
    let inner_no_suffix_debug = format!("{:?}", inner_no_suffix_spec.as_ref());
    assert!(
        inner_no_suffix_effect.is_some(),
        "inner no-suffix permission effect returned None for {inner_no_suffix_debug}"
    );
    assert!(
        inner_effect.is_some(),
        "inner permission effect returned None for {inner_debug}"
    );

    let parsed = super::super::permission_helpers::parse_cast_or_play_tagged_clause(&tokens)
        .expect("plural while-exiled tagged permission should parse")
        .expect("plural while-exiled tagged permission should produce an effect");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("LookAtObjects"), "{debug}");
    assert!(
        debug.contains("GrantPlayTaggedForAsLongAsExiled"),
        "{debug}"
    );
    assert!(
        debug.contains("allow_any_color_for_cast: AnyColor")
            || debug.contains("allow_any_color_for_cast: AnyType"),
        "{debug}"
    );
    assert!(debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("Planeswalker"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_permission_helpers_parse_while_exiled_owner_prefix() {
    let tokens = lex_line(
        "For as long as that card remains exiled, its owner may cast it without paying its mana cost",
        0,
    )
    .expect("rewrite lexer should classify while-exiled owner cast permission");

    let parsed = super::super::permission_helpers::parse_cast_or_play_tagged_clause(&tokens)
        .expect("while-exiled owner permission should parse")
        .expect("while-exiled owner permission should produce an effect");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("GrantPlayTaggedForAsLongAsExiled"),
        "{debug}"
    );
    assert!(debug.contains("player: ItsOwner"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
}

#[test]
pub(super) fn rewrite_lowering_choose_from_opponent_graveyard_or_hand_keeps_choice_zones()
-> Result<(), CardTextError> {
    let def = CardDefinitionBuilder::new(CardId::new(), "Psychic Intrusion Variant")
        .mana_cost(super::super::util::parse_scryfall_mana_cost("{3}{U}{B}").unwrap())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or hand and exile it. You may cast that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
        )?;

    let effects = def
        .spell_effect
        .as_ref()
        .expect("spell should lower")
        .flattened_default_effects();
    let choose = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::ChooseObjectsEffect>(effect))
        .expect("choice effect should be present");

    assert_eq!(choose.filter.zone, None);
    assert_eq!(choose.filter.controller, None);
    assert!(matches!(
        choose.filter.owner.as_ref(),
        Some(crate::PlayerFilter::Target(inner) | crate::PlayerFilter::AliasedTarget(inner))
            if inner.as_ref() == &crate::PlayerFilter::Opponent
    ));
    assert!(choose.filter.excluded_card_types.contains(&CardType::Land));
    assert!(matches!(
        (choose.zone, choose.additional_zones.as_slice()),
        (
            Some(crate::zone::Zone::Graveyard),
            [crate::zone::Zone::Hand]
        ) | (
            Some(crate::zone::Zone::Hand),
            [crate::zone::Zone::Graveyard]
        )
    ));
    let exile = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::MoveToZoneEffect>(effect))
        .expect("chosen card should be exiled");
    assert!(matches!(
        &exile.target,
        crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == crate::cards::builders::IT_TAG
    ));

    Ok(())
}

#[test]
pub(super) fn lonis_keeps_the_target_opponent_as_the_revealed_library_owner()
-> Result<(), CardTextError> {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lonis, Cryptozoologist")
        .mana_cost(super::super::util::parse_scryfall_mana_cost("{G}{U}").unwrap())
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever another nontoken creature you control enters, investigate.\n{T}, Sacrifice X Clues: Target opponent reveals the top X cards of their library. You may put a nonland permanent card with mana value X or less from among them onto the battlefield under your control. That player puts the rest on the bottom of their library in a random order.",
        )?;

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Lonis should have an activated ability");
    assert_eq!(
        activated.choices,
        vec![crate::target::ChooseSpec::target_opponent()],
        "the leading target opponent must remain a real target choice: {activated:#?}"
    );

    let effects = &activated.effects.segments[0].default_effects;
    let look = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>())
        .expect("the activated ability should inspect the targeted opponent's library");
    assert!(
        matches!(&look.player, crate::PlayerFilter::Target(inner)
            if inner.as_ref() == &crate::PlayerFilter::Opponent),
        "the first library action must use the explicit target: {look:#?}"
    );

    let remainder = effects
        .iter()
        .find_map(|effect| {
            effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
        })
        .expect("the unchosen cards should return to their library");
    assert!(
        matches!(&remainder.player, crate::PlayerFilter::AliasedTarget(inner)
            if inner.as_ref() == &crate::PlayerFilter::Opponent),
        "the remainder must use the already-selected opponent, not create another target: {remainder:#?}"
    );

    let battlefield_move = effects
        .iter()
        .find_map(|effect| {
            effect.downcast_ref::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>()
        })
        .and_then(|for_each| {
            for_each
                .effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<crate::effects::MoveToZoneEffect>())
        })
        .expect("the chosen permanent should move to the battlefield");
    assert_eq!(
        battlefield_move.battlefield_controller,
        crate::effects::BattlefieldController::You,
        "the explicit controller clause must survive looked-card lowering: {battlefield_move:#?}"
    );

    Ok(())
}

#[test]
pub(super) fn blue_dragon_keeps_three_independent_target_slots() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Blue Dragon")
        .mana_cost(super::super::util::parse_scryfall_mana_cost("{5}{U}{U}").unwrap())
        .card_types(vec![CardType::Creature]);
    let text = "Flying\nLightning Breath — When this creature enters, until your next turn, target creature an opponent controls gets -3/-0, up to one other target creature gets -2/-0, and up to one other target creature gets -1/-0.";
    let (semantic, _) = parse_text_to_semantic_document(builder.clone(), text.to_string(), false)?;
    let semantic_ability = semantic
        .items
        .iter()
        .filter_map(|item| match item {
            crate::ir::RewriteSemanticItem::ParsedLine(line) => Some(&line.chunks),
            _ => None,
        })
        .flatten()
        .find_map(|chunk| match chunk {
            crate::cards::builders::LineAst::Ability(ability) if ability.effects_ast.is_some() => {
                Some(ability)
            }
            _ => None,
        })
        .expect("Blue Dragon trigger should retain its semantic effect AST");
    let effects_ast = semantic_ability.effects_ast.as_deref().unwrap();
    assert!(
        !matches!(
            effects_ast,
            [crate::cards::builders::EffectAst::Coordinated { effects, .. }]
                if matches!(effects.as_slice(), [crate::cards::builders::EffectAst::Coordinated { .. }])
        ),
        "semantic normalization must not retain redundant coordination: {effects_ast:#?}"
    );
    if let crate::model::CompilerAbilityKindCore::Triggered(triggered) = semantic_ability.kind()
        && !triggered.effects.is_empty()
    {
        let [coordinated] = triggered.effects.flattened_default_effects() else {
            panic!("semantic trigger should contain one coordinated effect: {triggered:#?}");
        };
        let crate::cards::builders::EffectAst::Coordinated { effects, .. } = coordinated else {
            panic!("semantic trigger should retain coordinated execution: {triggered:#?}");
        };
        assert_eq!(
            effects.len(),
            3,
            "front-end runtime compatibility payload must not nest coordination: {triggered:#?}"
        );
    }
    let parsed_card = crate::compiler_pipeline::parse_semantic_document(semantic.clone())?;
    let parsed_effects = parsed_card
        .items
        .iter()
        .filter_map(|item| match item {
            crate::model::compiler_semantic::ParsedCardItem::Line(line) => Some(&line.chunks),
            _ => None,
        })
        .flatten()
        .find_map(|chunk| match chunk {
            crate::cards::builders::LineAst::Ability(ability) if ability.effects_ast.is_some() => {
                ability.effects_ast.as_deref()
            }
            _ => None,
        })
        .expect("parsed card should retain the Blue Dragon effect AST");
    assert!(
        !matches!(
            parsed_effects,
            [crate::cards::builders::EffectAst::Coordinated { effects, .. }]
                if matches!(effects.as_slice(), [crate::cards::builders::EffectAst::Coordinated { .. }])
        ),
        "reference resolution must not introduce redundant coordination: {parsed_effects:#?}"
    );
    let normalized_card = crate::compiler_pipeline::prepare_parsed_document(parsed_card)?;
    let normalized_prepared = normalized_card
        .items
        .iter()
        .filter_map(|item| match item {
            crate::effect_pipeline::NormalizedCardItem::Line(line) => Some(&line.chunks),
            _ => None,
        })
        .flatten()
        .find_map(|chunk| match chunk {
            crate::effect_pipeline::NormalizedLineChunk::Ability(ability) => {
                match ability.prepared.as_ref() {
                    Some(crate::effect_pipeline::NormalizedPreparedAbility::Triggered {
                        prepared,
                        ..
                    }) => Some(&prepared.prepared),
                    _ => None,
                }
            }
            _ => None,
        })
        .expect("Blue Dragon trigger should have typed prepared effects");
    assert!(
        !matches!(
            normalized_prepared.annotated.effects.as_slice(),
            [annotated]
                if matches!(
                    &annotated.effect,
                    crate::cards::builders::EffectAst::Coordinated { effects, .. }
                        if matches!(effects.as_slice(), [crate::cards::builders::EffectAst::Coordinated { .. }])
                )
        ),
        "reference annotation must not introduce redundant coordination: {normalized_prepared:#?}"
    );
    let (normalized_lowered, _) = crate::compile_support::materialize_prepared_triggered_effects(
        &crate::effect_pipeline::PreparedTriggeredEffectsForLowering {
            prepared: normalized_prepared.clone(),
            intervening_if: None,
        },
    )?;
    let [coordinated] = normalized_lowered.effects.flattened_default_effects() else {
        panic!("prepared Blue Dragon trigger should contain one effect: {normalized_lowered:#?}");
    };
    let coordinated = coordinated
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("prepared Blue Dragon trigger should lower as coordination");
    assert_eq!(
        coordinated.effects.len(),
        3,
        "prepared effect materialization must retain all coordination members: {normalized_lowered:#?}"
    );
    let def = builder.parse_text(text)?;

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Blue Dragon should have a triggered ability");
    assert_eq!(
        triggered.choices.len(),
        3,
        "the target-choice registry must keep all three independent target specs: {triggered:#?}"
    );

    let sequence = triggered.effects.flattened_default_effects()[0]
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the coordinated P/T clauses should remain a sequence");
    assert_eq!(
        sequence.effects.len(),
        3,
        "the required leading target and both optional targets must remain executable children: {triggered:#?}"
    );
    assert!(
        sequence
            .effects
            .iter()
            .all(|effect| effect.as_tagged().is_some()),
        "each independently targeted child must carry its own tag: {sequence:#?}"
    );
    assert!(sequence.effects.iter().all(|effect| {
        effect
            .as_tagged()
            .and_then(|tagged| {
                tagged
                    .effect
                    .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            })
            .is_some_and(|apply| apply.until == crate::effect::Until::YourNextTurn)
    }));
    Ok(())
}

#[test]
pub(super) fn rewrite_lexed_trigger_keeps_look_exile_and_while_exiled_play_permission() {
    let text = "Whenever equipped creature deals combat damage to a player, look at the top card of their library, then exile it face down. For as long as it remains exiled, you may play it, and mana of any type can be spent to cast that spell.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify while-exiled play trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("while-exiled play trigger should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("DealsCombatDamageToPlayer"), "{debug}");
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("player: That"), "{debug}");
    assert!(debug.contains("Exile"), "{debug}");
    assert!(debug.contains("face_down: true"), "{debug}");
    assert!(
        debug.contains("GrantPlayTaggedForAsLongAsExiled"),
        "{debug}"
    );
    assert!(
        debug.contains("allow_any_color_for_cast: AnyColor")
            || debug.contains("allow_any_color_for_cast: AnyType"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lowering_exile_bottom_card_of_each_opponent_library_face_down()
-> Result<(), CardTextError> {
    let def = CardDefinitionBuilder::new(CardId::new(), "Bottom Library Exile")
        .mana_cost(super::super::util::parse_scryfall_mana_cost("{3}{B}").unwrap())
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile the bottom card of each opponent's library face down.")?;

    let debug = format!("{def:#?}");
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("filter: Opponent"), "{debug}");
    assert!(
        debug.contains("zone: Some(") && debug.contains("Library"),
        "{debug}"
    );
    assert!(debug.contains("chooser: IteratedPlayer"), "{debug}");
    assert!(debug.contains("top_only: false"), "{debug}");
    assert!(debug.contains("bottom_only: true"), "{debug}");
    assert!(debug.contains("face_down: true"), "{debug}");

    Ok(())
}

#[test]
pub(super) fn rewrite_lexed_keyword_line_and_static_cost_probe_work_natively() {
    let flashback_tokens = lex_line("Flashback {2}{R}", 0)
        .expect("rewrite lexer should classify flashback keyword line");
    let cost_probe_tokens = lex_line("If it is night, this spell costs {2} less to cast.", 0)
        .expect("rewrite lexer should classify this-spell cost probe");

    assert!(matches!(
        super::super::clause_support::parse_ability_line_lexed(&flashback_tokens),
        Some(actions) if matches!(
            actions.as_slice(),
            [crate::cards::builders::KeywordAction::MarkerText(text)]
                if text == "Flashback {2}{R}"
        )
    ));
    let split =
        super::super::grammar::abilities::split_if_this_spell_costs_line_lexed(&cost_probe_tokens)
            .expect("grammar-owned this-spell cost splitter should match");
    assert_eq!(
        crate::lexer::token_word_refs(split.condition_tokens),
        vec!["it", "is", "night"],
    );
    assert_eq!(
        crate::lexer::token_word_refs(split.tail_tokens),
        vec!["this", "spell", "costs", "less", "to", "cast"],
    );
    assert!(matches!(
        super::super::keyword_static::parse_if_this_spell_costs_less_to_cast_line_lexed(
            &cost_probe_tokens
        ),
        Ok(Some(_))
    ));

    let trailing_cost_tokens = lex_line(
        "This spell costs {8} less to cast if you have eight or more instant and/or sorcery cards in your graveyard.",
        0,
    )
    .expect("rewrite lexer should classify trailing conditional this-spell cost line");
    match super::super::keyword_static::parse_spells_cost_modifier_line(&trailing_cost_tokens) {
        Ok(Some(_)) => {}
        other => panic!("expected trailing conditional cost modifier to parse, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_cost_reductions_count_controlled_creatures_with_counters() {
    let text = "This spell costs {1} less to cast for each creature you control with a +1/+1 counter on it.\nCreature spells you cast cost {1} less to cast for each creature you control with a +1/+1 counter on it.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Countered Creature Reduction")
                .card_types(vec![CardType::Creature]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("countered-creature reductions should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());

    let amounts = compiled
        .definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => match &static_ability.payload {
                StaticAbilityPayload::ThisSpellCostReduction(reduction) => Some(&reduction.amount),
                StaticAbilityPayload::CostReduction(reduction) => Some(&reduction.amount),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(amounts.len(), 2, "{:#?}", compiled.definition.abilities);
    for amount in amounts {
        let Value::Count(filter) = amount.unhinted() else {
            panic!("expected a creature count reduction, got {amount:?}");
        };
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.controller, Some(crate::target::PlayerFilter::You));
        assert_eq!(
            filter.with_counter,
            Some(crate::filter::CounterConstraint::Typed(
                CounterType::PlusOnePlusOne
            ))
        );
    }
}

#[test]
pub(super) fn rewrite_simultaneous_phase_pair_keeps_both_all_subjects() {
    let text = "Simultaneously, all phased-out creatures phase in and all creatures with phasing phase out.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Simultaneous Phasing")
                .card_types(vec![CardType::Instant]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("simultaneous phase pair should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let effects = compiled
        .definition
        .spell_effect
        .as_ref()
        .expect("simultaneous phasing should lower as a spell program")
        .flattened_default_effects();
    let phase_in = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::PhaseInEffect>(effect))
        .expect("simultaneous phasing should retain the phase-in action");
    let phase_out = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::PhaseOutEffect>(effect))
        .expect("simultaneous phasing should retain the phase-out action");
    assert!(
        matches!(&phase_in.target, crate::target::ChooseSpec::All(_))
            && matches!(&phase_out.target, crate::target::ChooseSpec::All(_)),
        "both plural phase subjects should lower to all-object specs: {phase_in:#?}, {phase_out:#?}"
    );
}

#[test]
pub(super) fn rewrite_endure_source_surface_keeps_typed_source_target() {
    let text = "Whenever this creature attacks, you lose 1 life and this creature endures 1.";
    let effect_tokens = lex_line("you lose 1 life and this creature endures 1.", 0)
        .expect("endure trigger payload should lex");
    let parsed_effects = super::super::clause_support::parse_effect_sentences_lexed(&effect_tokens)
        .expect("endure trigger payload should parse");
    let parsed_debug = format!("{parsed_effects:#?}");
    assert!(
        parsed_debug.contains("Endure") && parsed_debug.contains("target: Source"),
        "{parsed_debug}"
    );
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Endure Source Surface")
                .card_types(vec![CardType::Creature]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("source endure trigger should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let choose = compiled
        .definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| {
                    super::find_nested_effect::<crate::effects::ChooseModeEffect>(effect)
                }),
            _ => None,
        })
        .expect("endure should lower to a typed two-mode choice");
    let put = choose
        .modes
        .iter()
        .flat_map(|mode| &mode.effects)
        .find_map(|effect| effect.downcast_ref::<crate::effects::PutCountersEffect>())
        .expect("endure should keep its counter mode");
    assert!(
        matches!(put.target.base(), crate::target::ChooseSpec::Source),
        "endure's source-reference surface should still be semantically Source: {put:?}"
    );
}

#[test]
pub(super) fn flashback_keyword_accepts_non_mana_total_cost() {
    let flashback_tokens = lex_line("Flashback--Sacrifice three creatures", 0)
        .expect("rewrite lexer should classify non-mana flashback keyword line");

    let parsed = crate::util::parse_flashback_line_lexed(&flashback_tokens)
        .expect("non-mana flashback should parse")
        .expect("flashback line should be recognized");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("Flashback"), "{debug}");
    let crate::model::CompilerAlternativeCastingMethod::Flashback { total_cost } = parsed else {
        panic!("expected compiler-owned flashback cost: {debug}");
    };
    let costs = total_cost
        .as_all()
        .expect("non-mana flashback should have one ordered cost branch");
    assert!(matches!(
        costs,
        [crate::model::CompilerCost::Sacrifice { count, .. }]
            if count.min == 3 && count.max == Some(3)
    ));
    assert!(!debug.contains("Mana("), "{debug}");
}

#[test]
pub(super) fn rewrite_spell_cost_increase_per_target_beyond_first_hits_specific_parser() {
    let tokens = lex_line(
        "This spell costs {1} more to cast for each target beyond the first.",
        0,
    )
    .expect("rewrite lexer should classify additional-target spell tax");

    let parsed =
        super::super::keyword_static::parse_spell_cost_increase_per_target_beyond_first_line(
            &tokens,
        )
        .expect("additional-target spell tax parser should not error");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("CostIncreaseManaCostPerAdditionalTarget"),
        "{debug}"
    );
}

#[test]
pub(super) fn spell_life_cost_per_target_uses_typed_nonmana_cost_parser() {
    let tokens = lex_line("This spell costs 3 life more to cast for each target.", 0)
        .expect("rewrite lexer should classify per-target life cost");

    let parsed =
        super::super::keyword_static::parse_spell_additional_life_cost_per_target_line(&tokens)
            .expect("per-target life cost parser should not error")
            .expect("per-target life cost should be recognized");
    let debug = format!("{parsed:#?}");

    assert!(
        matches!(
            parsed.payload,
            ironsmith_core::StaticAbilityPayload::AdditionalLifeCostPerTarget(3)
        ),
        "{debug}"
    );
    assert!(!debug.contains("ManaCost"), "{debug}");
}

#[test]
pub(super) fn spell_tax_controller_turn_exception_becomes_active_player_exclusion() {
    let tokens = lex_line(
        "Each spell costs {3} more to cast except during its controller's turn.",
        0,
    )
    .expect("rewrite lexer should classify the controller-turn spell tax");

    let parsed = super::super::keyword_static::parse_spells_cost_modifier_line(&tokens)
        .expect("controller-turn spell tax parser should not error")
        .expect("controller-turn spell tax should be recognized");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("Excluding")
            && debug.contains("excluded: Active")
            && debug.contains("except_during_controller_turn: true"),
        "expected a typed active-player caster exclusion, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_combined_spell_and_activation_tax_hits_multi_parser() {
    let tokens = lex_line(
        "During your turn, spells your opponents cast cost {1} more to cast and abilities your opponents activate cost {1} more to activate unless they're mana abilities.",
        0,
    )
    .expect("rewrite lexer should classify combined tax line");

    let parsed =
        super::super::keyword_static::parse_spell_and_player_activated_ability_cost_modifier_line(
            &tokens,
        )
        .expect("combined tax parser should not error");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("CostIncrease"), "{debug}");
    assert!(debug.contains("ActivatedAbilityCostIncrease"), "{debug}");
}

#[test]
pub(super) fn jump_start_keyword_line_is_classified_as_alternative_cast() {
    let tokens = lex_line(
        "Jump-start (You may cast this card from your graveyard by discarding a card in addition to paying its other costs. Then exile this card.)",
        0,
    )
    .expect("rewrite lexer should classify jump-start keyword line");

    assert!(matches!(
        crate::keyword_families::parse_keyword_dispatch_hint(&tokens),
        Some(crate::keyword_families::KeywordDispatchHint::AlternativeOrExertFamily)
    ));

    let parsed = crate::util::parse_jump_start_line_lexed(&tokens)
        .expect("jump-start parse should not error")
        .expect("jump-start keyword line should parse");
    assert!(format!("{parsed:?}").contains("JumpStart"));
}

#[test]
pub(super) fn demilich_graveyard_cast_additional_exile_cost_permission_parses() {
    let tokens = lex_line(
        "You may cast this card from your graveyard by exiling four instant and/or sorcery cards from your graveyard in addition to paying its other costs.",
        0,
    )
    .expect("rewrite lexer should classify Demilich graveyard-cast line");

    let words = super::super::token_word_refs(&tokens).join(" ");
    let parsed = super::super::permission_helpers::parse_permission_clause_spec_lexed(&tokens)
        .expect("Demilich graveyard-cast permission should not error")
        .unwrap_or_else(|| panic!("Demilich graveyard-cast permission should parse: {words}"));
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("GraveyardCastFromCardManaCost"), "{debug}");
    assert!(debug.contains("ExileChosen"), "{debug}");
    assert!(debug.contains("Instant"), "{debug}");
    assert!(debug.contains("Sorcery"), "{debug}");
}

#[test]
pub(super) fn equal_to_number_of_cards_you_ve_discarded_this_turn_parses() {
    let tokens = lex_line("equal to the number of cards you've discarded this turn", 0)
        .expect("rewrite lexer should classify value clause");

    let parsed =
        super::super::grammar::shared_util::value_semantics::parse_equal_to_number_of_filter_value(
            &tokens,
        )
        .expect("discarded this turn count should parse");

    assert!(matches!(
        parsed.unhinted(),
        crate::Value::CardsDiscardedThisTurn(crate::PlayerFilter::You)
    ));
}

#[test]
pub(super) fn draw_for_target_opponents_discard_history_declares_and_reuses_one_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Targeted Discard History")
        .card_types(vec![CardType::Instant])
        .parse_text("Draw cards equal to the number of cards target opponent discarded this turn.")
        .expect("targeted discard-history draw should parse and lower");
    let effects = def
        .spell_effect
        .as_ref()
        .expect("spell should lower")
        .flattened_default_effects();
    let [target_effect, draw_effect] = effects else {
        panic!("expected one target declaration and one draw, got {effects:#?}");
    };
    let target = target_effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .expect("targeted value should synthesize a target declaration");
    assert_eq!(target.target, crate::target::ChooseSpec::target_opponent());
    assert!(!target.explicit_declaration);

    let draw = draw_effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .expect("second effect should draw cards");
    assert!(matches!(
        draw.count.unhinted(),
        crate::Value::CardsDiscardedThisTurn(crate::PlayerFilter::Target(inner))
            if inner.as_ref() == &crate::PlayerFilter::Opponent
    ));
}

#[test]
pub(super) fn rewrite_lower_routes_next_spell_cost_reduction_filters_through_grammar_entrypoint() {
    let text = "{T}: The next noncreature spell you cast this turn costs {2} less to cast.";
    let builder = CardDefinitionBuilder::new(CardId::new(), "Cost Reducer")
        .card_types(vec![CardType::Artifact]);

    let (doc, _) = parse_text_to_semantic_document(builder, text.to_string(), false).expect(
        "next-spell cost reduction should lower through the grammar-owned spell filter entrypoint",
    );
    let parsed = crate::compiler_pipeline::parse_semantic_document(doc)
        .expect("next-spell cost reduction should parse semantic items before preparation");
    let debug = format!("{parsed:?}");

    // The semantic document carries the typed next-spell reduction action;
    // the retired runtime-only CostReduction marker is no longer produced.
    assert!(debug.contains("ReduceNextSpellCostThisTurn"), "{debug}");
    assert!(debug.contains("excluded_card_types: [Creature]"), "{debug}");
}

#[test]
pub(super) fn rewrite_anthem_grant_static_parses_flashback_tail_without_word_view() {
    let tokens = lex_line(
        "During your turn, each instant and sorcery card in your graveyard has flashback. Its flashback cost is equal to its mana cost.",
        0,
    )
    .expect("rewrite lexer should classify granted flashback static line");

    let parsed = super::super::keyword_static::parse_granted_keyword_static_line(&tokens)
        .expect("granted flashback static line should parse")
        .expect("granted flashback static line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("Grants") || debug.contains("grants"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_anthem_grant_static_parses_escape_tail_without_word_view() {
    let tokens = lex_line(
        "Each nonland card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard.",
        0,
    )
    .expect("rewrite lexer should classify granted escape static line");

    let parsed = super::super::keyword_static::parse_granted_keyword_static_line(&tokens)
        .expect("granted escape static line should parse")
        .expect("granted escape static line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("Grants") || debug.contains("grants"),
        "{debug}"
    );
}

#[test]
pub(super) fn triggering_object_controller_target_choice_keeps_typed_chooser_and_filter()
-> Result<(), CardTextError> {
    let def = CardDefinitionBuilder::new(CardId::new(), "Controller Choice Exchange")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever an artifact, creature, or enchantment enters, its controller chooses target permanent another player controls that shares a card type with it. Exchange control of those permanents.",
        )?;
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let target_only = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .or_else(|| {
                    effect
                        .downcast_ref::<crate::effects::TaggedEffect>()?
                        .effect
                        .downcast_ref::<crate::effects::TargetOnlyEffect>()
                })
        })
        .expect("expected target-only declaration");

    assert!(target_only.explicit_declaration);
    let chooser = target_only.chooser.as_ref().expect("typed chooser");
    assert!(matches!(
        chooser,
        crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
            if tag.as_str() == "triggering"
    ));
    let crate::target::ChooseSpec::Object(filter) = target_only.target.base() else {
        panic!(
            "expected permanent target filter, got {:?}",
            target_only.target
        );
    };
    let Some(crate::target::PlayerFilter::Excluding { base, excluded }) = &filter.controller else {
        panic!("expected another-player controller exclusion, got {filter:?}");
    };
    assert_eq!(base.as_ref(), &crate::target::PlayerFilter::Any);
    assert_eq!(excluded.as_ref(), chooser);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "triggering"
            && constraint.relation == crate::target::TaggedOpbjectRelation::SharesCardType
    }));

    Ok(())
}
