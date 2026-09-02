use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::{CardDefinition, CardDefinitionBuilder};
use ironsmith_compiler::continuous::{EffectTarget, Modification};
use ironsmith_compiler::costs::Cost;
use ironsmith_compiler::effects::ChooseObjectsEffect;
use ironsmith_compiler::filter::Comparison;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::mana::ManaSymbol;
use ironsmith_compiler::static_abilities::{StaticAbility, StaticAbilityPayload};
use ironsmith_compiler::types::CardType;

fn compile_creature(name: &str, text: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"))
}

fn block_cost_payload(ability: &StaticAbility) -> &StaticAbilityPayload {
    match &ability.payload {
        payload @ StaticAbilityPayload::BlockCost { .. } => payload,
        StaticAbilityPayload::Conditional { ability, .. } => block_cost_payload(ability),
        payload => panic!("expected typed BlockCost payload, got {payload:#?}"),
    }
}

fn only_static_ability(definition: &CardDefinition) -> &StaticAbility {
    assert_eq!(definition.abilities.len(), 1, "{:#?}", definition.abilities);
    let AbilityKind::Static(ability) = &definition.abilities[0].kind else {
        panic!("expected one static ability: {:#?}", definition.abilities);
    };
    ability
}

#[test]
fn fixed_source_block_cost_lowers_to_typed_total_cost() {
    let definition = compile_creature(
        "Qal Sisma Behemoth Probe",
        "This creature can't attack or block unless you pay {2}.",
    );
    let StaticAbilityPayload::BlockCost { blockers, cost, .. } =
        block_cost_payload(only_static_ability(&definition))
    else {
        unreachable!();
    };
    assert!(blockers.source, "{blockers:#?}");
    assert_eq!(
        cost.mana_cost().expect("fixed mana cost").pips(),
        &[vec![ManaSymbol::Generic(2)]]
    );
}

#[test]
fn attacker_filter_is_preserved_on_block_cost() {
    let definition = compile_creature(
        "Hipparion Probe",
        "This creature can't block creatures with power 3 or greater unless you pay {1}.",
    );
    let StaticAbilityPayload::BlockCost { attackers, .. } =
        block_cost_payload(only_static_ability(&definition))
    else {
        unreachable!();
    };
    assert_eq!(attackers.power, Some(Comparison::GreaterThanOrEqual(3)));
}

#[test]
fn per_blocking_creature_global_tax_is_one_charge_per_blocker() {
    let definition = compile_creature(
        "Archangel Blocking Probe",
        "As long as this creature is attacking, creatures can't block unless their controller pays {1} for each of those creatures.",
    );
    let StaticAbilityPayload::BlockCost { blockers, cost, .. } =
        block_cost_payload(only_static_ability(&definition))
    else {
        unreachable!();
    };
    assert_eq!(blockers.card_types, vec![CardType::Creature]);
    assert!(cost.dynamic_mana_cost().is_none(), "{cost:#?}");
    assert_eq!(
        cost.mana_cost()
            .expect("one fixed charge per blocker")
            .pips(),
        &[vec![ManaSymbol::Generic(1)]]
    );
}

#[test]
fn attached_creature_cost_keeps_attachment_source_identity() {
    let definition = compile_creature(
        "Oppressive Aura Probe",
        "Enchanted creature can't attack or block unless its controller pays {3}.",
    );
    let StaticAbilityPayload::BlockCost {
        blockers,
        blocker_is_attached_to_source,
        cost,
        ..
    } = block_cost_payload(only_static_ability(&definition))
    else {
        unreachable!();
    };
    assert!(*blocker_is_attached_to_source);
    assert_eq!(blockers.card_types, vec![CardType::Creature]);
    assert_eq!(
        cost.mana_cost()
            .expect("fixed attached-creature cost")
            .pips(),
        &[vec![ManaSymbol::Generic(3)]]
    );
}

#[test]
fn attached_dynamic_cost_remains_source_relative_until_declaration_lock() {
    let definition = compile_creature(
        "Cowed by Wisdom Probe",
        "Enchanted creature can't attack or block unless its controller pays {1} for each card in your hand.",
    );
    let StaticAbilityPayload::BlockCost {
        blocker_is_attached_to_source,
        cost,
        ..
    } = block_cost_payload(only_static_ability(&definition))
    else {
        unreachable!();
    };
    assert!(*blocker_is_attached_to_source);
    assert!(
        cost.dynamic_mana_cost().is_some(),
        "the hand-count multiplier must be resolved only at CR 509.1d: {cost:#?}"
    );
}

#[test]
fn direct_tap_block_cost_excludes_declared_combatants() {
    let definition = compile_creature(
        "Hollow Warrior Probe",
        "This creature can't attack or block unless you tap an untapped creature you control not declared as an attacking or blocking creature this combat.",
    );
    let StaticAbilityPayload::BlockCost { cost, .. } =
        block_cost_payload(only_static_ability(&definition))
    else {
        unreachable!();
    };
    fn choose_in_cost_effect(
        effect: &ironsmith_compiler::effect::Effect,
    ) -> Option<&ChooseObjectsEffect> {
        if let Some(choose) = effect.downcast_ref::<ChooseObjectsEffect>() {
            return Some(choose);
        }
        if let Some(sequence) = effect.downcast_ref::<ironsmith_compiler::effects::SequenceEffect>()
        {
            return sequence.effects.iter().find_map(choose_in_cost_effect);
        }
        None
    }

    let chosen = cost
        .costs()
        .iter()
        .find_map(|component| match component {
            Cost::Effect(effect) => choose_in_cost_effect(effect),
            _ => None,
        })
        .expect("tap cost should choose an eligible creature");
    assert!(chosen.filter.untapped, "{:#?}", chosen.filter);
    assert!(chosen.filter.nonattacking, "{:#?}", chosen.filter);
    assert!(chosen.filter.nonblocking, "{:#?}", chosen.filter);
}

#[test]
fn temporary_x_block_tax_keeps_filter_live_and_x_dynamic_until_resolution() {
    let definition = compile_creature(
        "War Cadence Probe",
        "{X}{R}: This turn, creatures can't block unless their controller pays {X} for each blocking creature they control.",
    );
    assert_eq!(definition.abilities.len(), 1, "{:#?}", definition.abilities);
    let AbilityKind::Activated(activated) = &definition.abilities[0].kind else {
        panic!("expected activated ability: {:#?}", definition.abilities);
    };
    let apply = activated
        .effects
        .all_effects()
        .into_iter()
        .find_map(|effect| effect.as_apply_continuous())
        .expect("block tax should lower through ApplyContinuousEffect");
    assert!(
        !apply.lock_filter_at_resolution,
        "later-entering creatures must also receive the turn-long blocking rule"
    );
    assert!(
        matches!(&apply.target, EffectTarget::Filter(filter) if filter.card_types == vec![CardType::Creature])
    );
    let Some(Modification::AddAbility(ability)) = &apply.modification else {
        panic!("expected granted block-cost ability: {apply:#?}");
    };
    let StaticAbilityPayload::BlockCost { blockers, cost, .. } = block_cost_payload(ability) else {
        unreachable!();
    };
    assert!(blockers.source, "{blockers:#?}");
    let dynamic = cost
        .dynamic_mana_cost()
        .expect("activation X must remain dynamic until the effect resolves");
    assert!(dynamic.base.has_x(), "{dynamic:#?}");
}
