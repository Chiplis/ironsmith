//! Regressions at the boundary between recognized Oracle text and runtime lowering.
use crate::cards::builders::{
    ConditionalEffectAst, EffectAst, PermanentStateActionAst, PredicateAst, ReferenceImports,
    SubjectVerbActionAst, TargetAst, TriggerSpec,
};
use crate::compile_support::{
    materialize_prepared_effects_with_trigger_context, materialize_prepared_statement_effects,
    materialize_prepared_triggered_effects,
};
use crate::effect::{Condition, Effect, Until, Value};
use crate::effect_ast_traversal::for_each_nested_effects;
use crate::effect_pipeline::{
    EffectPreludeTag, NormalizedParsedAbility, NormalizedPreparedAbility,
};
use crate::filter::{ObjectFilter, ObjectRef};
use crate::lowering_support::{
    lower_prepared_ability, stage_effects_for_lowering,
    stage_effects_with_trigger_context_for_lowering, stage_statement_effects_for_lowering,
    stage_triggered_effects_for_lowering,
};
use crate::target::{ChooseSpec, PlayerFilter, TaggedOpbjectRelation};
use crate::{CardDefinitionBuilder, CardId, CardType, ParseCardText, TagKey};

fn parse_effects(text: &str) -> Vec<EffectAst> {
    let tokens = crate::lexer::lex_line(text, 0).expect("lex effect text");
    crate::effect_sentences::parse_effect_sentences_lexed(&tokens).expect("parse effect text")
}

fn lower_control_loss_trigger(
    sentences: Vec<EffectAst>,
    expected_segments: usize,
) -> crate::ability::TriggeredAbility {
    let trigger = TriggerSpec::DiesCreatureDealtDamageByThisTurn {
        victim: ObjectFilter::creature(),
        damager: crate::cards::builders::DamageBySpec::ThisCreature,
    };
    let parsed = ironsmith_compiler_semantic::keyword_abilities::assemble_parsed_triggered_ability(
        trigger.clone(),
        sentences.clone(),
        vec![crate::Zone::Battlefield],
        None,
        None,
        ReferenceImports::default(),
    );
    let (trigger, prepared) =
        stage_triggered_effects_for_lowering(trigger, &sentences, ReferenceImports::default())
            .expect("prepare control-loss trigger");
    let (unfused, intervening_if) = materialize_prepared_triggered_effects(&prepared).unwrap();
    assert!(intervening_if.is_none());
    assert_eq!(
        unfused.effects.segments.len(),
        expected_segments,
        "check the input to the final ability lowering pass"
    );
    let ability = lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered { trigger, prepared }),
    })
    .expect("lower the complete triggered ability");
    let crate::ability::AbilityKind::Triggered(triggered) = ability.kind else {
        panic!("expected the control-loss trigger");
    };
    triggered
}

fn control_loss_triggers(text: &str) -> [crate::ability::TriggeredAbility; 2] {
    let effects = parse_effects(text);
    let inline = lower_control_loss_trigger(effects.clone(), 1);
    let sentences = effects
        .into_iter()
        .map(|effect| EffectAst::SourceSentence {
            effects: vec![effect],
            leading_then: false,
            starting_with_controller: false,
        })
        .collect::<Vec<_>>();
    [inline, lower_control_loss_trigger(sentences, 2)]
}

#[test]
fn explicit_damage_source_keeps_target_legality_after_reference_capture() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Targeted damage source probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target creature an opponent controls deals damage equal to its power to another target creature that player controls. The Ring tempts you.")
        .expect("dependent targeted damage should compile");
    fn visit(effect: &Effect, sources: &mut Vec<ChooseSpec>) {
        if let Some(bound) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
            sources.push(bound.source.clone());
        }
        effect.visit_child_effects(&mut |child| visit(child, sources));
    }
    let mut sources = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        visit(effect, &mut sources);
    }
    assert_eq!(sources.len(), 1, "{sources:#?}");
    assert!(
        sources[0].is_target(),
        "an explicit damage source must retain CR 608.2b legality: {sources:#?}"
    );
    assert!(
        matches!(sources[0].base(), ChooseSpec::Object(_)),
        "{sources:#?}"
    );
}

#[test]
fn typed_trigger_program_replaces_unbound_player_import_at_shared_staging_boundary() {
    let effects = parse_effects("Exile the top card of that player's library.");
    let mut imports = ReferenceImports::default();
    imports.last_player_filter = Some(PlayerFilter::IteratedPlayer);
    let prepared = stage_effects_with_trigger_context_for_lowering(
        Some(&TriggerSpec::ThisAttacks),
        &effects,
        imports,
    )
    .unwrap();
    let lowered = materialize_prepared_effects_with_trigger_context(&prepared).unwrap();
    let effects = lowered.effects.flattened_default_effects();
    let exile = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>())
        .unwrap();
    assert_eq!(exile.player, PlayerFilter::Defending);
}

#[test]
fn concrete_attack_player_role_replaces_unbound_import_placeholder() {
    let effects = parse_effects("Exile the top card of that player's library.");
    let mut imports = ReferenceImports::default();
    imports.last_player_filter = Some(PlayerFilter::IteratedPlayer);
    let (_, prepared) =
        stage_triggered_effects_for_lowering(TriggerSpec::ThisAttacks, &effects, imports).unwrap();
    let (lowered, _) = materialize_prepared_triggered_effects(&prepared).unwrap();
    let effects = lowered.effects.flattened_default_effects();
    let exile = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>())
        .unwrap();
    assert_eq!(exile.player, PlayerFilter::Defending);
}

#[test]
fn phase_step_attachment_survives_incompatible_discard_antecedent() {
    let effects = parse_effects(
        "That player may discard a card at random. If the player does, untap that creature.",
    );
    let trigger = TriggerSpec::BeginningOfUpkeep(PlayerFilter::ControllerOf(ObjectRef::tagged(
        ironsmith_compiler_semantic::tag::declared_key("enchanted"),
    )));
    let (_, prepared) =
        stage_triggered_effects_for_lowering(trigger, &effects, ReferenceImports::default())
            .expect("attachment trigger should prepare");
    fn untap_reference_tag(effect: &EffectAst) -> Option<TagKey> {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap {
                target: TargetAst::Object(filter, _, _),
            }) = &subject_verb.action
        {
            return filter
                .tagged_constraints
                .iter()
                .find(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
                .map(|constraint| constraint.tag.clone());
        }
        let mut found = None;
        for_each_nested_effects(effect, true, |nested| {
            if found.is_none() {
                found = nested.iter().find_map(untap_reference_tag);
            }
        });
        found
    }
    let tag = prepared
        .prepared
        .annotated
        .effects
        .iter()
        .find_map(|annotated| untap_reference_tag(&annotated.effect))
        .expect("prepared body should retain the typed untap target");
    assert_eq!(tag.as_str(), "enchanted");
}

#[test]
fn statement_preparation_exports_terminal_per_player_memory_for_next_line() {
    let effects = parse_effects(
        "Each opponent exiles a creature with the greatest power among creatures that player controls.",
    );
    let prepared = stage_statement_effects_for_lowering(&effects, ReferenceImports::default())
        .expect("statement preparation should assign its terminal result producer");

    assert!(
        prepared.exports.to_imports().last_effect_id.is_some(),
        "the next source statement must be able to import the per-player exile result: {prepared:#?}"
    );
}

#[test]
fn self_contained_coordinated_statement_does_not_export_unused_result_id() {
    let effects = parse_effects("Target player draws two cards and loses 2 life.");
    let prepared = stage_statement_effects_for_lowering(&effects, ReferenceImports::default())
        .expect("coordinated statement should prepare");

    assert!(
        prepared.exports.to_imports().last_effect_id.is_none(),
        "a self-contained statement must not manufacture a terminal result dependency: {prepared:#?}"
    );
    let lowered = materialize_prepared_statement_effects(&prepared)
        .expect("coordinated statement should lower");
    assert!(
        lowered
            .effects
            .flattened_default_effects()
            .iter()
            .all(|effect| effect
                .downcast_ref::<crate::effects::WithIdEffect>()
                .is_none()),
        "unused statement export must not obscure the coordinated runtime shell: {lowered:#?}"
    );
}

#[test]
fn blocks_or_becomes_blocked_union_binds_that_creature_to_the_other_participant() {
    let filter = ObjectFilter::creature();
    let trigger = TriggerSpec::Either(
        Box::new(TriggerSpec::ThisBlocksObject {
            filter: filter.clone(),
            min_blocked_objects: None,
        }),
        Box::new(TriggerSpec::ThisBecomesBlockedByObject(filter.clone())),
    );
    let effects = parse_effects("That creature gains first strike until end of turn.");
    let (_, prepared) =
        stage_triggered_effects_for_lowering(trigger, &effects, ReferenceImports::default())
            .expect("combat union should prepare a shared event-participant reference");
    assert!(matches!(
        prepared.prepared.prelude.as_slice(),
        [EffectPreludeTag::OtherBlockParticipant(tag, candidate)]
            if tag.as_str() == "blocking" && candidate == &filter
    ));

    let (lowered, intervening_if) =
        materialize_prepared_triggered_effects(&prepared).expect("combat union body should lower");
    assert!(intervening_if.is_none());
    let flattened = lowered.effects.flattened_default_effects();
    assert!(
        flattened
            .first()
            .and_then(|effect| {
                effect.downcast_ref::<crate::effects::TagOtherBlockParticipantEffect>()
            })
            .is_some_and(|tag| tag.tag.as_str() == "blocking"),
        "the executable prelude must select the participant opposite the source: {flattened:#?}"
    );
    let grant = flattened
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ApplyContinuousEffect>())
        .expect("the combat participant should receive a typed temporary grant");
    assert!(matches!(
        grant.target_spec.as_ref().map(ChooseSpec::unhinted),
        Some(ChooseSpec::Tagged(tag)) if tag.as_str() == "blocking"
    ));
    assert_eq!(grant.until, Until::EndOfTurn);
    assert!(
        format!("{flattened:#?}").contains("FirstStrike"),
        "the typed grant must survive lowering: {flattened:#?}"
    );
}

#[test]
fn delayed_return_and_control_loss_sacrifice_rejoin_across_sentence_segments() {
    for lowered in control_loss_triggers(
        "Put that card onto the battlefield under your control at the beginning of the next end step. Sacrifice the creature when you lose control of this creature.",
    ) {
        assert_eq!(lowered.effects.segments.len(), 1, "{:#?}", lowered.effects);

        let schedule = lowered.effects.segments[0].default_effects[1]
            .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            .expect("the triggering card should still return at the next end step");
        assert_eq!(schedule.effects.len(), 2, "{schedule:#?}");
        let returned = schedule.effects[0]
            .downcast_ref::<crate::effects::TaggedEffect>()
            .expect("the returned identity must be tagged");
        assert_eq!(returned.tag.as_str(), "returned_control_loss");
        let control_loss = schedule.effects[1]
            .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            .expect("the returned identity must receive a control-loss watcher");
        assert!(matches!(
            control_loss.trigger,
            ironsmith_core::DelayedTriggerSpec::SourceControllerLosesControl { .. }
        ));
        assert_eq!(
            control_loss
                .target_tag
                .as_ref()
                .map(crate::tag::TagKey::as_str),
            Some("returned_control_loss")
        );
        assert!(control_loss.watch_ability_source);
    }
}

#[test]
fn immediate_return_and_control_loss_sacrifice_rejoin_across_sentence_segments() {
    for lowered in control_loss_triggers(
        "Put that card onto the battlefield under your control. Sacrifice it when you lose control of this creature.",
    ) {
        assert_eq!(lowered.effects.segments.len(), 1, "{:#?}", lowered.effects);
        let semantic_effects = lowered.effects.segments[0]
            .default_effects
            .iter()
            .filter(|effect| {
                effect
                    .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                    .is_none()
            })
            .collect::<Vec<_>>();
        let [returned, control_loss] = semantic_effects.as_slice() else {
            panic!(
                "expected returned object plus one control-loss watcher: {:#?}",
                lowered.effects
            );
        };
        let returned = returned
            .downcast_ref::<crate::effects::TaggedEffect>()
            .expect("the returned identity must stay tagged");
        let control_loss = control_loss
            .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            .expect("the returned identity must receive a control-loss watcher");
        assert!(matches!(
            control_loss.trigger,
            ironsmith_core::DelayedTriggerSpec::SourceControllerLosesControl { .. }
        ));
        assert_eq!(control_loss.target_tag.as_ref(), Some(&returned.tag));
        assert!(control_loss.watch_ability_source);
    }
}

#[test]
fn discard_trigger_intervening_it_predicate_binds_the_discarded_card() {
    let trigger = TriggerSpec::PlayerDiscardsCard {
        player: PlayerFilter::You,
        filter: None,
        cause_controller: None,
        effect_like_only: false,
        one_or_more: false,
    };
    let body = parse_effects("Untap this creature.");
    let madness_filter =
        ObjectFilter::default().with_alternative_cast(ironsmith_core::AlternativeCastKind::Madness);
    let effects = vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::TaggedMatches(
            crate::tag::CompilerReferenceTag::It.bind(),
            madness_filter.clone(),
        ),
        if_true: body,
        if_false: Vec::new(),
    })];

    let (_, prepared) =
        stage_triggered_effects_for_lowering(trigger, &effects, ReferenceImports::default())
            .expect("discarded-card intervening predicate should prepare");
    assert!(prepared.prepared.prelude.iter().any(|prelude| {
        matches!(
            prelude,
            EffectPreludeTag::TriggeringObject(tag) if tag.as_str() == "triggering"
        )
    }));

    let (_, intervening_if) = materialize_prepared_triggered_effects(&prepared)
        .expect("discarded-card intervening predicate should lower");
    assert!(matches!(
        intervening_if,
        Some(Condition::TaggedObjectMatches(tag, filter))
            if tag.as_str() == "triggering" && filter == madness_filter
    ));
}

#[test]
fn unblocked_attacker_offer_keeps_offset_target_and_followup_identity() {
    let effects = parse_effects(
        "Its controller may have it deal damage equal to its power plus 2 to another target creature. If that player does, the attacking creature assigns no combat damage this turn.",
    );
    let trigger = TriggerSpec::AttacksAndIsntBlocked(ObjectFilter::creature().match_tagged(
        ironsmith_compiler_semantic::tag::declared_key("enchanted"),
        TaggedOpbjectRelation::IsTaggedObject,
    ));
    let (_, prepared) =
        stage_triggered_effects_for_lowering(trigger, &effects, ReferenceImports::default())
            .expect("unblocked-attacker body should prepare");
    let (lowered, intervening_if) = materialize_prepared_triggered_effects(&prepared)
        .expect("unblocked-attacker body should lower");
    assert!(intervening_if.is_none());

    let flattened = lowered.effects.flattened_default_effects();
    let triggering = flattened
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>())
        .expect("unblocked attacker should receive an event identity tag");
    let offer = flattened
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::WithIdEffect>())
        .expect("optional offer should export its result");
    let may = offer
        .effect
        .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
        .expect("controller should make a may choice");
    assert!(matches!(
        may.decider.as_ref(),
        Some(PlayerFilter::ControllerOf(ObjectRef::Tagged(tag)))
            if tag.as_str() == triggering.tag.as_str()
    ));
    let with_source = may
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>())
        .expect("the triggering attacker should be the damage source");
    assert!(matches!(
        with_source.source.unhinted(),
        ChooseSpec::Tagged(tag) if tag.as_str() == triggering.tag.as_str()
    ));
    let damage = with_source
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .expect("optional effect should deal damage");
    assert!(
        matches!(
            damage.amount.unhinted(),
            Value::Add(power, offset)
                if matches!(
                    power.unhinted(),
                    Value::PowerOf(spec)
                        if matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag.as_str() == triggering.tag.as_str())
                ) && offset.unhinted() == &Value::Fixed(2)
        ),
        "damage amount lost triggering-source identity or its authored offset: {damage:#?}"
    );
    let ChooseSpec::Target(target) = damage.target.unhinted() else {
        panic!("damage recipient should remain targeted: {damage:#?}");
    };
    let ChooseSpec::Object(filter) = target.unhinted() else {
        panic!("damage recipient should be an object target: {damage:#?}");
    };
    assert!(
        filter.other,
        "recipient must be another creature: {filter:#?}"
    );
    assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == triggering.tag.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }),
        "the damage source must be excluded during target legality: {filter:#?}"
    );

    let result = flattened
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::IfEffect>())
        .expect("followup should test the optional offer result");
    assert_eq!(result.condition, offer.id);
    assert_eq!(result.predicate, crate::effect::EffectPredicate::Happened);
    let [assignment_effect] = result.then.as_slice() else {
        panic!("expected one no-combat-damage assignment: {result:#?}");
    };
    let assignment = assignment_effect
        .downcast_ref::<crate::effects::AssignNoCombatDamageEffect>()
        .expect("followup should assign no combat damage");
    assert_eq!(assignment.until, Until::EndOfTurn);
    assert!(matches!(
        assignment.source.unhinted(),
        ChooseSpec::Tagged(tag) if tag.as_str() == triggering.tag.as_str()
    ));
}

#[test]
fn coordinated_tap_does_not_replace_countered_spell_controller_actor() {
    let effects = parse_effects(
        "Counter target spell unless its controller pays {X}. If that player doesn't, they tap all lands with mana abilities they control and lose all unspent mana.",
    );
    let prepared = stage_effects_for_lowering(&effects, ReferenceImports::default())
        .expect("power-sink probe should prepare");
    let lowered =
        materialize_prepared_statement_effects(&prepared).expect("power-sink probe should lower");
    let debug = format!("{:#?}", lowered.effects);

    assert!(
        debug.contains("AliasedControllerOf")
            && debug.contains("countered_0")
            && !debug.contains("AliasedControllerOf(Tagged(TagKey(\"tapped_"),
        "nonpayment actor must remain the countered spell's controller: {debug}"
    );
}

#[test]
fn repeated_optional_exiles_prepare_plural_return_with_aggregate_tag() {
    let effects = parse_effects(
        "Exile up to one target artifact you control and up to one target creature you control. Then return them to the battlefield under their owners' control.",
    );
    let prepared =
        stage_effects_for_lowering(&effects, ReferenceImports::default()).expect("prepare");
    let debug = format!("{:#?}", prepared.annotated.effects);

    assert!(
        debug.contains("ReturnAllToBattlefield")
            && !debug.contains(crate::tag::CompilerReferenceTag::SourceExiled.as_str()),
        "expected aggregate helper tag to replace the plural placeholder, got {debug}"
    );
}

#[test]
fn trailing_unless_stays_a_resolution_time_conditional() {
    let cases = [
        (
            "This creature deals 4 damage to that player unless they control a commander.",
            TriggerSpec::BeginningOfEndStep(PlayerFilter::Any),
        ),
        (
            "This creature deals 2 damage to that player unless they control two or more basic lands.",
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any),
        ),
        (
            "This artifact deals 2 damage to that player unless they have exactly three or exactly four cards in hand.",
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::Opponent),
        ),
        (
            "This Aura deals 2 damage to that player unless that creature attacked this turn.",
            TriggerSpec::BeginningOfEndStep(PlayerFilter::ControllerOf(ObjectRef::tagged(
                ironsmith_compiler_semantic::tag::declared_key("enchanted"),
            ))),
        ),
    ];

    for (text, trigger) in cases {
        let effects = parse_effects(text);
        assert!(
            matches!(
                effects.as_slice(),
                [EffectAst::Conditionals(
                    ConditionalEffectAst::TrailingUnless { .. }
                )]
            ) || matches!(
                effects.as_slice(),
                [EffectAst::ControlFlow(control)]
                    if matches!(
                        &control.node,
                        crate::model::control_flow::ControlFlowNodeAst::Condition {
                            condition,
                            ..
                        } if condition.position
                            == crate::model::control_flow::ConditionPositionAst::Postcondition
                            && condition.negated_surface
                    )
            ),
            "expected canonical trailing-unless control flow for {text}: {effects:#?}"
        );

        let (_, prepared) =
            stage_triggered_effects_for_lowering(trigger, &effects, ReferenceImports::default())
                .expect("prepare trailing-unless trigger");
        assert!(
            prepared.intervening_if.is_none(),
            "trailing unless must not be promoted to intervening-if: {text}"
        );

        let (lowered, intervening_if) = materialize_prepared_triggered_effects(&prepared)
            .expect("lower trailing-unless trigger");
        assert!(
            intervening_if.is_none(),
            "unexpected intervening-if: {text}"
        );
        let conditional = lowered
            .effects
            .flattened_default_effects()
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
            .expect("lowered trailing-unless conditional");
        assert_eq!(
            conditional.surface,
            ironsmith_core::ConditionalSurface::TrailingUnless,
            "missing trailing-unless runtime surface: {text}"
        );
        assert!(
            matches!(&conditional.condition, Condition::Not(_)),
            "runtime true branch must retain the negated executable gate: {text}"
        );
    }
}
