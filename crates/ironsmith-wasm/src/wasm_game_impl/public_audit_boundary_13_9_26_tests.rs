// Source-authored, UNRUN. Exercise the actual restricted-mana public carrier.
use super::{PUBLIC_AUDIT_VERSION, SyncRestrictedManaUnit, sync_restricted_mana};
use ironsmith::ability::{Ability, RestrictedManaUnit};
use ironsmith::effects::{CantEffect, ConditionalEffect, RepeatProcessEffect, RepeatProcessPromptEffect,
    ScheduleDelayedTriggerEffect};
use ironsmith::effects::player::GrantNextSpellAbilityEffect;
use ironsmith::{Effect, ObjectId, PlayerId};
use ironsmith_compiled_artifact::{WireAbility, WireEffect};
use ironsmith_core::{Anthem, AnthemCountExpression, AnthemValue, Condition,
    ContinuousDurationObject, CounterType, DelayedTriggerSpec, EffectId, EffectPredicate,
    ManaPaymentPredicate, ManaSpendPayload, ManaSymbol, ManaUsageRestriction, ObjectCharacteristic,
    ObjectFilter, PlayerFilter, RepeatProcessPromptKind, ResolutionProgram, Restriction, Until};
use ironsmith_core::trigger_model::PlayerAttackGrouping;

fn carrier(effects: Vec<Effect>) -> RestrictedManaUnit {
    RestrictedManaUnit {
        symbol: ManaSymbol::Green, source: ObjectId::from_raw(17),
        source_controller: Some(PlayerId::from_index(0)), source_chosen_creature_type: None,
        restrictions: vec![ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::Any),
            on_spend: vec![ManaSpendPayload { predicate: ManaPaymentPredicate::Any,
                effects: ResolutionProgram::from_effects(effects), choices: vec![] }],
        }],
    }
}

fn project(effects: Vec<Effect>) -> Vec<WireEffect> {
    assert_eq!(PUBLIC_AUDIT_VERSION, 11);
    let unit = carrier(effects);
    let bytes = serde_json::to_vec(&sync_restricted_mana(&[unit.clone()]).unwrap()).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&sync_restricted_mana(&[unit]).unwrap()).unwrap());
    let restored: Vec<SyncRestrictedManaUnit> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    let ManaUsageRestriction::PaymentTransaction { on_spend, .. } = &restored[0].restrictions[0]
    else { panic!("retain typed public payment carrier") };
    on_spend[0].effects.all_effects().into_iter().cloned().collect()
}

#[test]
fn fresh_native_repeat_gate_actor_and_exact_result_predicate_survive_public_projection() {
    for capture in [false, true] {
        for decider in [None, Some(PlayerFilter::Opponent)] {
            for characteristic in [ObjectCharacteristic::Color, ObjectCharacteristic::Name] {
                let gate = Effect::new(ConditionalEffect::new(Condition::YourTurn,
                    vec![Effect::gain_life(2)], vec![Effect::gain_life(3)])
                    .with_condition_result(capture));
                let prompt = Effect::new(RepeatProcessPromptEffect::new(
                    RepeatProcessPromptKind::MayRepeatAnyNumberOfTimes).with_decider(decider.clone()));
                let predicate = EffectPredicate::AffectedObjectsShare { required_count: 2, characteristic };
                let repeat = Effect::new(RepeatProcessEffect::new(vec![gate, prompt], EffectId(73), predicate.clone()));
                assert!(repeat.serialized_model().is_none());
                let effects = project(vec![repeat]);
                assert_eq!(effects[0].kind(), "RepeatProcessEffect");
                let repeat: ironsmith_core::RepeatProcessEffect<WireEffect> =
                    serde_json::from_value(effects[0].payload().clone()).unwrap();
                assert_eq!(repeat.condition, EffectId(73));
                assert_eq!(repeat.predicate, predicate);
                assert_eq!(repeat.effects.len(), 2);
                let gate: ironsmith_core::ConditionalEffect<WireEffect> =
                    serde_json::from_value(repeat.effects[0].payload().clone()).unwrap();
                assert_eq!(gate.capture_condition_result, capture);
                assert_eq!(gate.if_true[0].kind(), "GainLifeEffect");
                assert_eq!(gate.if_false[0].kind(), "GainLifeEffect");
                assert_ne!(gate.if_true[0].payload(), gate.if_false[0].payload());
                let prompt: ironsmith_core::RepeatProcessPromptEffect =
                    serde_json::from_value(repeat.effects[1].payload().clone()).unwrap();
                assert_eq!(prompt.decider, decider);
            }
        }
    }
}

#[test]
fn delayed_declaration_projects_retained_canonical_model_but_never_invents_a_native_encoder() {
    for grouping in [PlayerAttackGrouping::Attacker, PlayerAttackGrouping::Defender,
        PlayerAttackGrouping::Pair, PlayerAttackGrouping::AttackerAnyTarget] {
        let spec = DelayedTriggerSpec::PlayerAttackDeclaration {
            attacker: PlayerFilter::Opponent, defender: PlayerFilter::You, grouping,
        };
        let child = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(Effect::gain_life(2)).unwrap();
        let model = ironsmith_core::ScheduleDelayedTriggerEffect::new(
            spec.clone(), vec![child], false, vec![], PlayerFilter::You);
        let wire = WireEffect::new("ScheduleDelayedTriggerEffect", serde_json::to_value(&model).unwrap());
        let native = ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire.clone()).unwrap();
        assert!(native.serialized_model().is_some(), "this route owns a retained compiled model");
        assert_eq!(project(vec![native]), vec![wire]);

        // A genuinely fresh native schedule has no complete reverse codec in
        // this boundary. The real public projection must fail closed, not omit it.
        let fresh = Effect::new(ScheduleDelayedTriggerEffect::new(
            ironsmith::triggers::Trigger::from_delayed_trigger_spec(spec),
            vec![Effect::gain_life(2)], false, vec![], PlayerFilter::You));
        assert!(fresh.serialized_model().is_none());
        assert!(sync_restricted_mana(&[carrier(vec![fresh])]).is_err());
    }
}

#[test]
fn fresh_native_duration_owners_keep_the_named_player_distinct_from_the_exact_object() {
    let player = PlayerFilter::Specific(PlayerId::from_index(1));
    let duration = Until::PlayersNextUntapStep { player: player.clone() };
    let cant = Effect::new(CantEffect::new(Restriction::untap(
        ObjectFilter::creature().controlled_by(player)), duration.clone()));
    assert!(cant.serialized_model().is_none());
    let effects = project(vec![cant]);
    let model: ironsmith_core::CantEffect = serde_json::from_value(effects[0].payload().clone()).unwrap();
    assert_eq!(model.duration, duration);

    let duration = Until::UntilControllersNextUntapStep {
        object: ContinuousDurationObject::Specific(ObjectId::from_raw(81)),
    };
    let pump = Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
        ironsmith::continuous::EffectTarget::Source,
        ironsmith::continuous::Modification::AddSubtypes(vec![ironsmith_core::Subtype::Wizard]),
        duration.clone()));
    assert!(pump.serialized_model().is_none());
    let effects = project(vec![pump]);
    assert_eq!(effects[0].kind(), "ApplyContinuousEffect");
    assert_eq!(effects[0].payload()["until"], serde_json::to_value(duration).unwrap());
}

#[test]
fn player_counter_domains_survive_a_fresh_native_granted_ability_carrier() {
    for player in [PlayerFilter::You, PlayerFilter::Opponent] {
        for counter_type in [CounterType::Experience, CounterType::Poison] {
            let count = AnthemCountExpression::PlayerCounters(player.clone(), counter_type);
            let anthem: Anthem<Condition> = Anthem::for_source(0, 0).with_values(
                AnthemValue::scaled(-1, count.clone()), AnthemValue::scaled_capped(2, count.clone(), 10));
            let model = ironsmith_core::StaticAbility::new(anthem);
            let ability = Ability::static_ability(ironsmith::static_abilities::StaticAbility::from_model(model));
            let grant = Effect::new(GrantNextSpellAbilityEffect::new(PlayerFilter::You,
                ObjectFilter::creature(), ability));
            assert!(grant.serialized_model().is_none());
            let effects = project(vec![grant]);
            let model: ironsmith_core::GrantNextSpellAbilityEffect<WireAbility> =
                serde_json::from_value(effects[0].payload().clone()).unwrap();
            let ironsmith_core::AbilityKind::Static(ability) = model.ability.kind else { panic!("static grant") };
            let ironsmith_core::StaticAbilityPayload::Anthem(anthem) = ability.payload else { panic!("anthem") };
            assert_eq!(anthem.power, AnthemValue::scaled(-1, count.clone()));
            assert_eq!(anthem.toughness, AnthemValue::scaled_capped(2, count, 10));
        }
    }
}
