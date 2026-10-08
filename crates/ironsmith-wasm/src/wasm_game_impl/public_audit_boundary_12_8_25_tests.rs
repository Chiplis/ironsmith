// Source-authored, UNRUN. Exercise the real public restricted-mana projection
// through fresh native effects, not handwritten substitutes for its wire output.
use super::{PUBLIC_AUDIT_VERSION, SyncRestrictedManaUnit, sync_restricted_mana};
use ironsmith::ability::{Ability, RestrictedManaUnit};
use ironsmith::effects::{PreventAllDamageToTargetEffect, PreventDamageEffect};
use ironsmith::effects::player::GrantNextSpellAbilityEffect;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::{Effect, ObjectId, PlayerId};
use ironsmith_compiled_artifact::{WireAbility, WireEffect};
use ironsmith_core::{
    ChooseSpec, Color, Condition, DamageFilter, ManaPaymentPredicate, ManaSpendPayload,
    ManaSymbol, ManaUsageRestriction, ObjectFilter, PlayerFilter, ResolutionProgram,
    Subtype, Until, Zone,
};
use serde_json::Value;

fn project(effects: Vec<Effect>) -> (Vec<WireEffect>, Value) {
    let unit = RestrictedManaUnit {
        symbol: ManaSymbol::Green,
        source: ObjectId::from_raw(17),
        source_controller: Some(PlayerId::from_index(0)),
        source_chosen_creature_type: None,
        restrictions: vec![ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::Any),
            on_spend: vec![ManaSpendPayload {
                predicate: ManaPaymentPredicate::Any,
                effects: ResolutionProgram::from_effects(effects),
                choices: vec![],
            }],
        }],
    };
    assert_eq!(PUBLIC_AUDIT_VERSION, 11);
    let projected = sync_restricted_mana(&[unit.clone()]).unwrap();
    let bytes = serde_json::to_vec(&projected).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&sync_restricted_mana(&[unit]).unwrap()).unwrap());
    let restored: Vec<SyncRestrictedManaUnit> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&restored).unwrap());
    let ManaUsageRestriction::PaymentTransaction { on_spend, .. } = &restored[0].restrictions[0]
    else { panic!("typed payment carrier must survive public projection") };
    (on_spend[0].effects.all_effects().into_iter().cloned().collect(),
        serde_json::from_slice(&bytes).unwrap())
}

fn filter() -> DamageFilter {
    let mut filter = DamageFilter::combat();
    filter.from_source = Some(ObjectFilter::creature());
    filter.from_colors = Some(vec![Color::Red]);
    filter
}

#[test]
fn native_finite_filters_and_followups_remain_distinct_in_public_typed_programs() {
    let mut previous = Vec::new();
    for damage_filter in [DamageFilter::all(), DamageFilter::combat(),
        DamageFilter::noncombat(), filter()] {
        let effect = Effect::new(PreventDamageEffect::to_you(4, Until::EndOfTurn)
            .with_filter(damage_filter.clone()).with_follow_up_effects(vec![Effect::gain_life(2)]));
        assert!(effect.serialized_model().is_none(), "exercise the native encoder");
        let (effects, json) = project(vec![effect]);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].kind(), "PreventDamageEffect");
        let model: ironsmith_core::PreventDamageEffect<WireEffect> =
            serde_json::from_value(effects[0].payload().clone()).unwrap();
        assert_eq!(model.damage_filter, damage_filter);
        assert_eq!(model.amount, ironsmith_core::Value::Fixed(4));
        assert_eq!(model.follow_up_effects.len(), 1);
        assert_eq!(model.follow_up_effects[0].kind(), "GainLifeEffect");
        for earlier in &previous { assert_ne!(earlier, &json); }
        previous.push(json);
    }
}

#[test]
fn native_target_filters_color_choice_and_complete_children_reach_public_wire() {
    let mut previous = Vec::new();
    for damage_filter in [DamageFilter::all(), filter()] {
        for choose_color in [false, true] {
            let mut native = PreventAllDamageToTargetEffect::new(
                ChooseSpec::SpecificPlayer(PlayerId::from_index(0)), Until::EndOfTurn,
            ).with_filter(damage_filter.clone()).with_follow_up_effects(vec![Effect::gain_life(3)]);
            native.source_color_of_your_choice = choose_color;
            let effect = Effect::new(native);
            assert!(effect.serialized_model().is_none());
            let (effects, json) = project(vec![effect]);
            assert_eq!(effects[0].kind(), "PreventAllDamageToTargetEffect");
            let model: ironsmith_core::PreventAllDamageToTargetEffect<WireEffect> =
                serde_json::from_value(effects[0].payload().clone()).unwrap();
            assert_eq!(model.damage_filter, damage_filter);
            assert_eq!(model.combat_only, damage_filter.combat_only);
            assert_eq!(model.source_color_of_your_choice, choose_color);
            assert_eq!(model.follow_up_effects.len(), 1);
            assert_eq!(model.follow_up_effects[0].kind(), "GainLifeEffect");
            for earlier in &previous { assert_ne!(earlier, &json); }
            previous.push(json);
        }
    }
}

#[test]
fn granted_subtype_construction_zones_survive_the_public_typed_ability_carrier() {
    let source = ironsmith_core::StaticAbility::add_subtypes(
        ObjectFilter::source(), vec![Subtype::Wizard],
    );
    for conditional in [false, true] {
        let model = if conditional { source.clone().with_condition(Condition::YourTurn) }
            else { source.clone() };
        let ability = Ability::static_ability(StaticAbility::from_model(model));
        let expected = ability.functional_zones.clone();
        if conditional {
            assert_eq!(expected, vec![Zone::Battlefield]);
        } else {
            assert!(expected.contains(&Zone::Hand));
            assert!(expected.contains(&Zone::Graveyard));
            assert!(expected.contains(&Zone::Stack));
        }
        let grant = Effect::new(GrantNextSpellAbilityEffect::new(
            PlayerFilter::You, ObjectFilter::creature(), ability,
        ));
        let (effects, _) = project(vec![grant]);
        assert_eq!(effects[0].kind(), "GrantNextSpellAbilityEffect");
        let model: ironsmith_core::GrantNextSpellAbilityEffect<WireAbility> =
            serde_json::from_value(effects[0].payload().clone()).unwrap();
        assert_eq!(model.ability.functional_zones, expected);
        // JSON loading preserves an explicit historical zone list. It must not
        // silently rerun construction defaults to make old artifacts current.
        let mut legacy_zones = serde_json::to_value(&model.ability).unwrap();
        legacy_zones["functional_zones"] = serde_json::json!(["Battlefield"]);
        let restored: WireAbility = serde_json::from_value(legacy_zones).unwrap();
        assert_eq!(restored.functional_zones, vec![Zone::Battlefield]);
    }
}

#[test]
fn source_counter_native_bridge_exposes_only_the_three_existing_public_payload_fields() {
    use ironsmith_core::{CounterType, RemoveAnyCountersFromSourceEffect};
    let mut previous = Vec::new();
    for counter_type in [None, Some(CounterType::PlusOnePlusOne), Some(CounterType::Named("hour".into()))] {
        for display_x in [false, true] {
            for remove_all in [false, true] {
                let model = RemoveAnyCountersFromSourceEffect { counter_type, display_x, remove_all };
                let effect = Effect::new(model.clone());
                assert!(effect.serialized_model().is_none());
                let (effects, json) = project(vec![effect]);
                assert_eq!(effects.len(), 1);
                assert_eq!(effects[0].kind(), "RemoveAnyCountersFromSourceEffect");
                assert_eq!(effects[0].payload(), &serde_json::to_value(&model).unwrap());
                assert_eq!(effects[0].payload().as_object().unwrap().len(), 3);
                let restored: RemoveAnyCountersFromSourceEffect = serde_json::from_value(effects[0].payload().clone()).unwrap();
                assert_eq!(serde_json::to_vec(&restored).unwrap(), serde_json::to_vec(&model).unwrap());
                for earlier in &previous { assert_ne!(earlier, &json); }
                previous.push(json);
            }
        }
    }
}

#[test]
fn static_prevention_source_controller_and_lifetime_survive_public_ability_projection() {
    use ironsmith_core::{AbilityKind, PreventMatchingDamageSpec, StaticDamagePreventionAmount};
    use ironsmith_compiled_artifact::WireStaticAbility;
    let aura = |combat_only| PreventMatchingDamageSpec {
        source_filter: ObjectFilter::tagged("enchanted"),
        target_player_filter: Some(PlayerFilter::Any),
        target_object_filter: Some(ObjectFilter::permanent()),
        combat_only, noncombat_only: false, maximum_damage: None,
        amount: StaticDamagePreventionAmount::All,
        display: "Prevent damage dealt by enchanted creature".into(),
    };
    let fixed_filter = ObjectFilter::default().controlled_by(PlayerFilter::Opponent);
    let mut previous = Vec::new();
    // Static owners retain their canonical model; the enclosing native grant
    // starts without a serialized effect model and must use the real encoder.
    for (native, expected) in [
        (StaticAbility::from_model(ironsmith_core::StaticAbility::prevent_matching_damage(aura(false))),
            WireStaticAbility::prevent_matching_damage(aura(false))),
        (StaticAbility::from_model(ironsmith_core::StaticAbility::prevent_matching_damage(aura(true))),
            WireStaticAbility::prevent_matching_damage(aura(true))),
        (StaticAbility::from_model(ironsmith_core::StaticAbility::prevent_damage_to_you_from_source_filter(
            1, fixed_filter.clone(), "Prevent 1 to you")),
            WireStaticAbility::prevent_damage_to_you_from_source_filter(1, fixed_filter, "Prevent 1 to you")),
    ] {
        let ability = Ability::static_ability(native);
        assert_eq!(ability.functional_zones, vec![Zone::Battlefield]);
        let grant = Effect::new(GrantNextSpellAbilityEffect::new(
            PlayerFilter::You, ObjectFilter::default(), ability,
        ));
        assert!(grant.serialized_model().is_none());
        let (effects, json) = project(vec![grant]);
        let restored: ironsmith_core::GrantNextSpellAbilityEffect<WireAbility> =
            serde_json::from_value(effects[0].payload().clone()).unwrap();
        assert_eq!(restored.ability.functional_zones, vec![Zone::Battlefield]);
        let AbilityKind::Static(model) = restored.ability.kind else { panic!("retain static owner") };
        assert_eq!(serde_json::to_value(model).unwrap(), serde_json::to_value(expected).unwrap());
        for earlier in &previous { assert_ne!(earlier, &json); }
        previous.push(json);
    }
}
