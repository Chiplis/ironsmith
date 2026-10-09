//! Canonical native encoders required by the typed program-word owner.
//! Included as a child of artifact_materializer; retained executable models
//! and existing direct native payload encoders remain with that parent.

use super::{RuntimePayloadEncodingError, encode_runtime_cost, encode_runtime_effect, wire};
use crate::effect::Effect;
use crate::effects::*;

fn encoded<T: serde::Serialize>(kind: &'static str, model: T)
    -> Result<Option<wire::WireEffect>, RuntimePayloadEncodingError>
{
    serde_json::to_value(model).map(|payload| Some(wire::WireEffect::new(kind, payload)))
        .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() })
}

fn children(effects: &[Effect]) -> Result<Vec<wire::WireEffect>, RuntimePayloadEncodingError> {
    effects.iter().cloned().map(encode_runtime_effect).collect()
}

fn unsupported(detail: &'static str) -> RuntimePayloadEncodingError {
    RuntimePayloadEncodingError::InvalidEffectModel { detail: detail.into() }
}

pub(super) fn encode_text_changed_native_effect(effect: &Effect)
    -> Result<Option<wire::WireEffect>, RuntimePayloadEncodingError>
{
    if let Some(model) = effect.downcast_ref::<ChangeTextEffect>() {
        model.selection.validate().map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() })?;
        return encoded("ChangeTextEffect", model);
    }
    if let Some(model) = effect.downcast_ref::<DestroyEffect>() {
        return encoded("DestroyEffect", ironsmith_core::DestroyEffect::with_spec(model.spec.clone()));
    }
    if let Some(model) = effect.downcast_ref::<DestroyNoRegenerationEffect>() {
        return encoded("DestroyNoRegenerationEffect", ironsmith_core::DestroyNoRegenerationEffect {
            filter: None, target: Some(model.spec.clone()),
            creature_destroyed_this_way_surface: model.creature_destroyed_this_way_surface,
        });
    }
    if let Some(model) = effect.downcast_ref::<LoseLifeEffect>() {
        let crate::target::ChooseSpec::Player(player) = &model.player else {
            return Err(unsupported("native life-loss choice needs a complete choice transport model"));
        };
        return encoded("LoseLifeEffect", ironsmith_core::LoseLifeEffect {
            amount: model.amount.clone(), player: player.clone(),
        });
    }
    if let Some(model) = effect.downcast_ref::<SacrificeEffect>() {
        let crate::effect::Value::Fixed(count) = &model.count else {
            return Err(unsupported("native tagged sacrifice needs a dynamic-count transport model"));
        };
        if model.player != crate::target::PlayerFilter::You {
            return Err(unsupported("native tagged sacrifice needs a player-scoped transport model"));
        }
        return encoded("SacrificeEffect", ironsmith_core::SacrificeEffect {
            filter: model.filter.clone(), count: *count,
            event_object_tags: model.event_object_tags.clone(), event_source_tags: model.event_source_tags.clone(),
        });
    }
    if let Some(model) = effect.downcast_ref::<DiscardEffect>() {
        return encoded("DiscardEffect", ironsmith_core::DiscardEffect {
            count: model.count.clone(), player: model.player.clone(), random: model.random,
            any_number: model.any_number, card_filter: model.card_filter.clone(), tag: model.tag.clone(),
        });
    }
    if let Some(model) = effect.downcast_ref::<DoubleCountersEffect>() {
        return encoded("DoubleCountersEffect", model);
    }
    if let Some(model) = effect.downcast_ref::<NoteLifeTotalEffect>() {
        return encoded("NoteLifeTotalEffect", model);
    }
    if let Some(model) = effect.downcast_ref::<AddColorlessManaEffect>() {
        // Both native implementations use the same Repeated mana-production
        // owner and preserve the exact ManaAdded result, including zero mana.
        return encoded("AddScaledManaEffect", ironsmith_core::AddScaledManaEffect {
            mana: vec![crate::mana::ManaSymbol::Colorless], amount: model.amount.clone(), player: model.player.clone(),
        });
    }
    if let Some(model) = effect.downcast_ref::<ChoosePlayerEffect>() {
        return encoded("ChoosePlayerEffect", ironsmith_core::ChoosePlayerEffect {
            chooser: model.chooser.clone(), filter: model.filter.clone(), tag: model.tag.clone(),
            excluded_tags: model.excluded_tags.clone(), random: model.random,
            remember_as_chosen_player: model.remember_as_chosen_player,
        });
    }
    if let Some(model) = effect.downcast_ref::<WithIdEffect>() {
        return encoded("WithIdEffect", ironsmith_core::WithIdEffect {
            id: model.id, effect: Box::new(encode_runtime_effect((*model.effect).clone())?),
        });
    }
    if let Some(model) = effect.downcast_ref::<TaggedEffect>() {
        return encoded("TaggedEffect", model.with_effect(encode_runtime_effect((*model.effect).clone())?));
    }
    if let Some(model) = effect.downcast_ref::<ExecuteWithSourceEffect>() {
        return encoded("ExecuteWithSourceEffect", ironsmith_core::ExecuteWithSourceEffect {
            source: model.source.clone(), effect: Box::new(encode_runtime_effect((*model.effect).clone())?),
        });
    }
    if let Some(model) = effect.downcast_ref::<RepeatProcessPromptEffect>() {
        if model.fallback != crate::decision::FallbackStrategy::Decline {
            return Err(unsupported("native repeat prompt fallback needs an explicit transport model"));
        }
        return encoded("RepeatProcessPromptEffect", ironsmith_core::RepeatProcessPromptEffect::new(model.kind)
            .with_decider(model.decider.clone()));
    }
    if let Some(model) = effect.downcast_ref::<MayEffect>() {
        if model.fallback != crate::decision::FallbackStrategy::Decline {
            return Err(unsupported("native optional-action fallback needs an explicit transport model"));
        }
        return encoded("MayEffect", ironsmith_core::MayEffect {
            effects: children(&model.effects)?, decider: model.decider.clone(), pay_as_cost: model.pay_as_cost,
        });
    }
    if let Some(model) = effect.downcast_ref::<UnlessPaysEffect>() {
        return encoded("UnlessPaysEffect", ironsmith_core::UnlessPaysEffect {
            effects: children(&model.effects)?, player: model.player.clone(),
            cost: model.cost.clone().try_map(encode_runtime_cost)?,
            leading_surface: model.leading_surface, before_delayed_step: model.before_delayed_step,
        });
    }
    if let Some(model) = effect.downcast_ref::<UnlessActionEffect>() {
        return encoded("UnlessActionEffect", ironsmith_core::UnlessActionEffect {
            effects: children(&model.effects)?, alternative: children(&model.alternative)?, player: model.player.clone(),
        });
    }
    if let Some(model) = effect.downcast_ref::<ConditionalEffect>() {
        return encoded("ConditionalEffect", ironsmith_core::ConditionalEffect {
            condition: model.condition.clone(), if_true: children(&model.if_true)?,
            if_false: children(&model.if_false)?, surface: model.surface,
            capture_condition_result: model.capture_condition_result,
        });
    }
    if let Some(model) = effect.downcast_ref::<IfEffect>() {
        return encoded("IfEffect", ironsmith_core::IfEffect {
            condition: model.condition, predicate: model.predicate.clone(),
            then: children(&model.then)?, else_: children(&model.else_)?,
            per_player_result: model.per_player_result,
            prior_result_replacement_surface: model.prior_result_replacement_surface,
        });
    }
    if let Some(model) = effect.downcast_ref::<ReflexiveTriggerEffect>() {
        return encoded("ReflexiveTriggerEffect", ironsmith_core::ReflexiveTriggerEffect {
            condition: model.condition, predicate: model.predicate.clone(), effects: children(&model.effects)?,
            choices: model.choices.clone(), intervening_if: model.intervening_if.clone(),
        });
    }
    if let Some(model) = effect.downcast_ref::<ChooseModeEffect>() {
        // Explicit construction preserves every modal flag and cost along
        // with each definition's source text; only children change vocabulary.
        return encoded("ChooseModeEffect", ironsmith_core::ChooseModeEffect {
            modes: model.modes.iter().map(|mode| Ok(ironsmith_core::EffectMode {
                source_text: mode.source_text.clone(), effects: children(&mode.effects)?,
            })).collect::<Result<_, RuntimePayloadEncodingError>>()?,
            common_prefix_effects: children(&model.common_prefix_effects)?, chooser: model.chooser.clone(),
            min: model.min.clone(), max: model.max.clone(), allow_repeat: model.allow_repeat, random: model.random,
            choose_count: model.choose_count.clone(), min_choose_count: model.min_choose_count.clone(),
            allow_repeated_modes: model.allow_repeated_modes, mode_point_costs: model.mode_point_costs.clone(),
            spree: model.spree, tiered: model.tiered, mode_additional_mana_costs: model.mode_additional_mana_costs.clone(),
            common_suffix_effect_count: model.common_suffix_effect_count,
            disallow_previously_chosen_modes: model.disallow_previously_chosen_modes,
            disallow_previously_chosen_modes_this_turn: model.disallow_previously_chosen_modes_this_turn,
            distinct_player_targets_per_mode: model.distinct_player_targets_per_mode,
            conditional_mode_range: model.conditional_mode_range.clone(), presentation_label: model.presentation_label.clone(),
            endure: model.endure, cast_chooser: model.cast_chooser.clone(),
        });
    }
    if let Some(model) = effect.downcast_ref::<ForEachObject>() {
        return encoded("ForEachObject", ironsmith_core::ForEachObject {
            filter: model.filter.clone(), effects: children(&model.effects)?,
        });
    }
    if let Some(model) = effect.downcast_ref::<ForEachObjectCorrelatedResultEffect>() {
        return encoded("ForEachObjectCorrelatedResultEffect", ironsmith_core::ForEachObjectCorrelatedResultEffect {
            filter: model.filter.clone(), producer_effects: children(&model.producer_effects)?,
            result_tag: model.result_tag.clone(), source_binding_tag: model.source_binding_tag.clone(),
            result_binding_tag: model.result_binding_tag.clone(), consumer_effects: children(&model.consumer_effects)?,
        });
    }
    if let Some(model) = effect.downcast_ref::<ForEachControllerOfTaggedEffect>() {
        return encoded("ForEachControllerOfTaggedEffect", ironsmith_core::ForEachControllerOfTaggedEffect {
            tag: model.tag.clone(), effects: children(&model.effects)?,
        });
    }
    if let Some(model) = effect.downcast_ref::<ForEachTaggedPlayerEffect>() {
        return encoded("ForEachTaggedPlayerEffect", ironsmith_core::ForEachTaggedPlayerEffect {
            tag: model.tag.clone(), effects: children(&model.effects)?, require_evidence: model.require_evidence,
        });
    }
    if let Some(model) = effect.downcast_ref::<RepeatProcessEffect>() {
        return encoded("RepeatProcessEffect", ironsmith_core::RepeatProcessEffect {
            effects: children(&model.effects)?, condition: model.condition, predicate: model.predicate.clone(),
            choice_history: model.choice_history.clone(),
        });
    }
    if let Some(model) = effect.downcast_ref::<RepeatEffectsEffect>() {
        return encoded("RepeatEffectsEffect", ironsmith_core::RepeatEffectsEffect {
            count: model.count.clone(), effects: children(&model.effects)?,
        });
    }
    if let Some(model) = effect.downcast_ref::<ManaRestrictedEffect>() {
        return encoded("ManaRestrictedEffect", ironsmith_core::ManaRestrictedEffect {
            effects: children(&model.effects)?, restrictions: model.restrictions.iter().cloned()
                .map(|restriction| restriction.try_map_effects(&mut encode_runtime_effect)).collect::<Result<_, _>>()?,
        });
    }
    if let Some(model) = effect.downcast_ref::<ManaRetainedEffect>() {
        return encoded("ManaRetainedEffect", ironsmith_core::ManaRetainedEffect {
            effects: children(&model.effects)?, duration: model.duration,
        });
    }
    if let Some(model) = effect.downcast_ref::<CumulativeUpkeepEffect>() {
        return encoded("CumulativeUpkeepEffect", ironsmith_core::CumulativeUpkeepEffect {
            player: model.player.clone(), payment: children(&model.payment)?, failure: children(&model.failure)?, kind: model.kind,
        });
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    // These native/wire scenarios are authored but intentionally unrun.
    use super::*;
    use crate::effect::{Condition, EffectId, EffectPredicate, Value};
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    use ironsmith_core::{Color, ColorSet, TextChange};

    fn black() -> ObjectFilter { ObjectFilter { colors: Some(ColorSet::BLACK), ..ObjectFilter::default() } }
    fn quantity() -> Value { Value::Count(black()) }
    fn child() -> Effect { Effect::new(DrawCardsEffect::you(quantity())) }

    fn complete_native_examples() -> Vec<Effect> {
        let mut modal = ChooseModeEffect::new(vec![ironsmith_core::EffectMode::new(
            "black-mode-label", vec![child()],
        )], Value::Fixed(1), quantity(), true);
        modal.common_prefix_effects = vec![child()];
        modal.chooser = Some(PlayerFilter::ControlsMost { filter: Box::new(black()) });
        modal.min_choose_count = Value::Fixed(1);
        modal.choose_count = quantity();
        modal.random = true;
        modal.spree = true;
        modal.tiered = true;
        modal.mode_point_costs = vec![2];
        modal.mode_additional_mana_costs = vec![crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Black]])];
        modal.common_suffix_effect_count = 1;
        modal.disallow_previously_chosen_modes = true;
        modal.disallow_previously_chosen_modes_this_turn = true;
        modal.distinct_player_targets_per_mode = true;
        modal.conditional_mode_range = Some(ironsmith_core::ConditionalModeRange::new("black-cost", Value::Fixed(1), quantity()));
        modal.endure = true;
        let restriction = crate::ability::ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ironsmith_core::ManaPaymentPredicate::SourceMatches(black())),
            on_spend: vec![ironsmith_core::ManaSpendPayload {
                predicate: ironsmith_core::ManaPaymentPredicate::SourceMatches(black()),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![child()])
                    .with_linked_exile_pair(ironsmith_core::LinkedExilePair {
                        definition: ironsmith_core::LinkedExileDefinition([23; 32]), pair: 3,
                    }),
                choices: vec![ChooseSpec::target(ChooseSpec::Object(black()))],
            }],
        };
        let mut tagged_player = ForEachTaggedPlayerEffect::new("black-players", vec![child()]);
        tagged_player.require_evidence = true;
        let mut unless = UnlessPaysEffect::new_total_cost(vec![child()], PlayerFilter::You,
            crate::cost::TotalCost::one_of(vec![
                crate::cost::TotalCost::from_cost(crate::costs::Cost::sacrifice(black())),
                crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            ]));
        unless.leading_surface = true;
        unless.before_delayed_step = true;
        vec![
            Effect::new(DestroyEffect::with_spec(ChooseSpec::target(ChooseSpec::Object(black())))),
            Effect::new(DestroyNoRegenerationEffect::all(black()).with_creature_destroyed_this_way_surface(true)),
            Effect::new(LoseLifeEffect::with_filter(quantity(), PlayerFilter::You)),
            Effect::new(SacrificeEffect::you(black(), 2).with_event_object_tag("black-paid").with_event_source_tag("black-source")),
            Effect::new(DiscardEffect::new_with_filter(quantity(), PlayerFilter::You, true, Some(black()))
                .with_any_number(true).with_tag("black-discard")),
            Effect::new(DoubleCountersEffect::new(None, ChooseSpec::target(ChooseSpec::Object(black())))),
            Effect::new(NoteLifeTotalEffect),
            Effect::new(AddColorlessManaEffect::new(quantity(), PlayerFilter::You)),
            Effect::new(ChoosePlayerEffect::new(PlayerFilter::You,
                PlayerFilter::ControlsMost { filter: Box::new(black()) }, "black-player")
                .at_random().excluding_tags(vec!["black-excluded".into()]).remember_as_chosen_player()),
            Effect::new(WithIdEffect::new(EffectId(43), child())),
            Effect::new(TaggedEffect { tag: "black-tag".into(), effect: Box::new(child()), outcome_only: true }),
            Effect::new(ExecuteWithSourceEffect::new(ChooseSpec::All(black()), child())),
            Effect::new(MayEffect::new_for_player(vec![child()], PlayerFilter::You).with_pay_as_cost(true)),
            Effect::new(unless),
            Effect::new(UnlessActionEffect::new(vec![child()], vec![child()], PlayerFilter::You)),
            Effect::new(ConditionalEffect::new(Condition::YouControl(black()), vec![child()], vec![child()])
                .with_surface(ironsmith_core::ConditionalSurface::TrailingIf)),
            Effect::new(IfEffect::new(EffectId(43), EffectPredicate::Happened, vec![child()], vec![child()])
                .with_per_player_result(true).with_prior_result_replacement_surface(true)),
            Effect::new(ReflexiveTriggerEffect::new(EffectId(43), EffectPredicate::Happened,
                vec![child()], vec![ChooseSpec::target(ChooseSpec::Object(black()))])
                .with_intervening_if(Some(Condition::YouControl(black())))),
            Effect::new(modal),
            Effect::new(ForEachObject::new(black(), vec![child()])),
            Effect::new(ForEachObjectCorrelatedResultEffect::new(black(), vec![child()],
                "black-result", "black-source-binding", "black-result-binding", vec![child()])),
            Effect::new(ForEachControllerOfTaggedEffect::new("black-controller", vec![child()])),
            Effect::new(tagged_player),
            Effect::new(RepeatEffectsEffect::new(quantity(), vec![child()])),
            Effect::new(ManaRestrictedEffect::new(vec![child()], vec![restriction])),
            Effect::new(ManaRetainedEffect::new(vec![child()], ironsmith_core::ManaRetentionDuration::EndOfCombat)),
            Effect::new(CumulativeUpkeepEffect::echo(PlayerFilter::You, vec![child()], vec![child()])),
            Effect::new(AttachToEffect::new(ChooseSpec::target(ChooseSpec::Object(black())))),
            Effect::new(AttachObjectsEffect::new(ChooseSpec::All(black()),
                ChooseSpec::target(ChooseSpec::Player(PlayerFilter::ControlsMost { filter: Box::new(black()) })))
                .with_individual_targets()),
        ]
    }

    #[test]
    fn native_rewrites_roundtrip_every_new_codec_owner_then_rewrite_loaded_definitions() {
        let first = TextChange::color(Color::Black, Color::Blue).unwrap();
        let second = TextChange::color(Color::Blue, Color::Red).unwrap();
        for original in complete_native_examples() {
            let rewritten = original.with_text_change(first).unwrap();
            let wire = encode_runtime_effect(rewritten.clone()).unwrap();
            let restored = crate::artifact_materializer::materialize_effect(wire.clone()).unwrap();
            if let Some(reencoded) = encode_text_changed_native_effect(&restored).unwrap() {
                assert_eq!(reencoded, wire, "native/core payload fields must roundtrip");
            }
            // Force loaded native payloads through a second actual rewrite.
            // Merely comparing retained JSON would not detect stale lowering.
            let second_native = rewritten.with_text_change(second).unwrap();
            let second_restored = restored.with_text_change(second).unwrap();
            assert_eq!(encode_runtime_effect(second_restored).unwrap(), encode_runtime_effect(second_native).unwrap());
            let captured_wire = encode_runtime_effect(original.clone()).unwrap();
            let captured_again = crate::artifact_materializer::materialize_effect(captured_wire.clone()).unwrap();
            assert_eq!(encode_runtime_effect(captured_again).unwrap(), captured_wire);
        }
    }

    #[test]
    fn native_destroy_and_cost_program_roundtrip_preserve_binding_and_current_predicates() {
        let ability = crate::ability::Ability::activated(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::sacrifice(black())),
            vec![Effect::new(TaggedEffect::new("black-link", Effect::new(WithIdEffect::new(
                EffectId(11), Effect::new(DestroyEffect::with_spec(ChooseSpec::target(ChooseSpec::Object(black())))),
            ))))],
        );
        let rewritten = crate::continuous::text_change_programs::rewrite_ability_words(&ability,
            TextChange::color(Color::Black, Color::Blue).unwrap()).unwrap();
        let encoded = super::super::encode_runtime_ability(rewritten).unwrap();
        let restored = super::super::restore_runtime_ability(encoded.clone()).unwrap();
        let crate::ability::AbilityKind::Activated(model) = restored.kind else { panic!("activation lost"); };
        let blue = ObjectFilter { colors: Some(ColorSet::BLUE), ..ObjectFilter::default() };
        assert_eq!(model.mana_cost.costs()[0].compiled_model(), Some(&ironsmith_core::Cost::Sacrifice(blue.clone())));
        let tagged = model.effects.flattened_default_effects()[0].downcast_ref::<TaggedEffect>().unwrap();
        assert_eq!(tagged.tag.as_str(), "black-link");
        let observed = tagged.effect.downcast_ref::<WithIdEffect>().unwrap();
        assert_eq!(observed.id, EffectId(11));
        assert_eq!(observed.effect.downcast_ref::<DestroyEffect>().unwrap().spec,
            ChooseSpec::target(ChooseSpec::Object(blue)));
        assert_eq!(super::super::encode_runtime_ability(super::super::restore_runtime_ability(encoded.clone()).unwrap()).unwrap(), encoded);
    }

    #[test]
    fn codec_refuses_native_fields_that_the_core_schema_cannot_retain() {
        let life = Effect::new(LoseLifeEffect::new(quantity(), ChooseSpec::target_player()));
        assert!(encode_text_changed_native_effect(&life).is_err());
        let optional = Effect::new(MayEffect::new(vec![child()]).with_fallback(crate::decision::FallbackStrategy::Accept));
        assert!(encode_text_changed_native_effect(&optional).is_err());
        let sacrifice = Effect::new(SacrificeEffect::you(black(), quantity()));
        assert!(encode_text_changed_native_effect(&sacrifice).is_err());
    }

    #[test]
    fn attachment_wire_preserves_internal_tags_and_individual_player_destinations() {
        let original = Effect::new(AttachObjectsEffect::new(ChooseSpec::Tagged("black-entered-auras".into()),
            ChooseSpec::target(ChooseSpec::Player(PlayerFilter::ControlsMost { filter: Box::new(black()) })))
            .with_individual_targets());
        let changed = original.with_text_change(TextChange::color(Color::Black, Color::Blue).unwrap()).unwrap();
        let wire = encode_runtime_effect(changed).unwrap();
        let restored = crate::artifact_materializer::materialize_effect(wire).unwrap();
        let model = restored.downcast_ref::<AttachObjectsEffect>().unwrap();
        let blue = ObjectFilter { colors: Some(ColorSet::BLUE), ..ObjectFilter::default() };
        assert_eq!(model.objects, ChooseSpec::Tagged("black-entered-auras".into()));
        assert_eq!(model.target, ChooseSpec::target(ChooseSpec::Player(
            PlayerFilter::ControlsMost { filter: Box::new(blue) })));
        assert!(model.individual_targets);
    }

    fn elf_token_template() -> crate::effects::CreateTokenEffect {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::static_abilities::StaticAbility;
        use ironsmith_core::{CardType, Subtype, TokenNameTextRole, TokenTextRoles};
        let card = CardBuilder::new(crate::ids::CardId::new(), "Elf Token").token().card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf]).color_indicator(ColorSet::RED)
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Black]]))
            .power_toughness(PowerToughness::fixed(2, 2)).build();
        let ability = crate::ability::Ability::static_ability(StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(ColorSet::GREEN)));
        crate::effects::CreateTokenEffect::one(crate::cards::CardDefinition::with_abilities(card, vec![ability]))
            .with_text_roles(TokenTextRoles::authored(TokenNameTextRole::SubtypeDerived, 1))
    }

    #[test]
    fn token_word_edits_and_fresh_native_codecs_preserve_captured_numeric_pair_evidence() {
        let pair = ironsmith_core::LinkedExilePair {
            definition: ironsmith_core::LinkedExileDefinition([67; 32]), pair: 5,
        };
        let count = crate::effect::Value::SourceChosenNumber { if_unset: Some(0), pair: Some(pair) };
        let mut token = elf_token_template();
        token.count = count.clone();
        token.link_source_exiled_this_resolution = true;
        token.enters_tapped = true;
        let id = token.token.card.id;
        let source = Effect::new(token);
        let changed = source.with_text_change(TextChange::creature_type(ironsmith_core::Subtype::Elf, ironsmith_core::Subtype::Human).unwrap()).unwrap();
        let changed = changed.downcast_ref::<crate::effects::CreateTokenEffect>().unwrap();
        assert_eq!(changed.count, count);
        assert_eq!(changed.token.card.id, id);
        let wire = crate::artifact_materializer::encode_runtime_effect(Effect::new(changed.clone())).unwrap();
        let restored = crate::artifact_materializer::materialize_effect(wire).unwrap();
        let restored = restored.with_text_change(TextChange::creature_type(ironsmith_core::Subtype::Human, ironsmith_core::Subtype::Zombie).unwrap()).unwrap();
        let actual = restored.downcast_ref::<crate::effects::CreateTokenEffect>().unwrap();
        assert_eq!(actual.count, count);
        assert_eq!(actual.token.card.id, id);
        assert_eq!(actual.token.card.name, "Zombie Token");
        assert_eq!(actual.text_roles, changed.text_roles);
        assert!(actual.link_source_exiled_this_resolution && actual.enters_tapped);
        assert_eq!(source.downcast_ref::<crate::effects::CreateTokenEffect>().unwrap().token.card.name, "Elf Token");
    }
}
