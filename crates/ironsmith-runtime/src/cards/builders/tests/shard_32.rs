#![allow(unused_imports)]
use super::shard_16::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
fn unwrapped_effect(effect: &crate::effect::Effect) -> &crate::effect::Effect {
    let mut current = effect;
    while let Some(tagged) = current.downcast_ref::<TaggedEffect>() {
        current = tagged.effect.as_ref();
    }
    current
}

#[cfg(ironsmith_runtime_parser_tests)]
fn continuous_effect(
    effect: &crate::effect::Effect,
) -> Option<&crate::effects::ApplyContinuousEffect> {
    unwrapped_effect(effect).downcast_ref::<crate::effects::ApplyContinuousEffect>()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pugnacious_hammerskull_negative_condition_does_not_target_a_dinosaur() {
    let def = parse_oracle_card_definition("Pugnacious Hammerskull");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Pugnacious Hammerskull should have an attack trigger");

    assert!(
        triggered.choices.is_empty(),
        "the negative Dinosaur condition must not create a target choice: {:?}",
        triggered.choices
    );
    let put_counters = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| {
            unwrapped_effect(effect).downcast_ref::<crate::effects::PutCountersEffect>()
        })
        .expect("the attack trigger should put a stun counter on its antecedent");
    assert!(
        matches!(put_counters.target.unhinted(), ChooseSpec::Tagged(_)),
        "the stun counter should stay on the triggering creature, got {:?}",
        put_counters.target
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn reptilian_recruiter_or_condition_keeps_every_action_on_chosen_creature() {
    let def = parse_oracle_card_definition("Reptilian Recruiter");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Reptilian Recruiter should have an enters trigger");

    let [choice] = triggered.choices.as_slice() else {
        panic!(
            "Recruiter should choose only its initial target creature, got {:?}",
            triggered.choices
        );
    };
    let ChooseSpec::Object(choice_filter) = choice.base() else {
        panic!("Recruiter's single choice should be a creature, got {choice:?}");
    };
    assert!(choice_filter.card_types.contains(&CardType::Creature));
    assert!(
        !choice_filter.subtypes.contains(&Subtype::Lizard),
        "the existential Lizard branch must not become a new target: {choice_filter:?}"
    );

    let initial_target_tag = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| {
            let tagged = effect.downcast_ref::<TaggedEffect>()?;
            unwrapped_effect(effect)
                .downcast_ref::<TargetOnlyEffect>()
                .map(|_| &tagged.tag)
        })
        .expect("Recruiter's initial creature target should have a target slot");

    let conditional = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| unwrapped_effect(effect).downcast_ref::<ConditionalEffect>())
        .expect("Recruiter's gain-control chain should remain conditional");
    let branch_effects = match conditional.if_true.as_slice() {
        [effect]
            if unwrapped_effect(effect)
                .downcast_ref::<crate::effects::SequenceEffect>()
                .is_some() =>
        {
            &unwrapped_effect(effect)
                .downcast_ref::<crate::effects::SequenceEffect>()
                .expect("checked above")
                .effects
        }
        effects => effects,
    };
    let (controlled_tag, control) = branch_effects
        .iter()
        .find_map(|effect| {
            let tagged = effect.downcast_ref::<TaggedEffect>()?;
            let continuous = continuous_effect(effect)?;
            continuous
                .runtime_modifications
                .contains(&crate::effects::RuntimeModification::ChangeControllerToEffectController)
                .then_some((&tagged.tag, continuous))
        })
        .expect("Recruiter should gain control of the chosen creature");
    let untap = branch_effects
        .iter()
        .find_map(|effect| unwrapped_effect(effect).downcast_ref::<UntapEffect>())
        .expect("Recruiter should untap the chosen creature");
    let haste = branch_effects
        .iter()
        .filter_map(|effect| continuous_effect(effect))
        .find(|effect| {
            matches!(
                effect.modification.as_ref(),
                Some(crate::continuous::Modification::AddAbility(ability))
                    if ability.id() == StaticAbilityId::Haste
            )
        })
        .expect("Recruiter should grant haste to the chosen creature");

    let control_references_initial_target = match control
        .target_spec
        .as_ref()
        .map(ChooseSpec::unhinted)
    {
        Some(ChooseSpec::Tagged(tag)) => tag == initial_target_tag,
        Some(ChooseSpec::Object(filter)) => filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *initial_target_tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        }),
        _ => false,
    };
    assert!(
        control_references_initial_target,
        "Recruiter's control effect should reference the initial target slot: {:?}",
        control.target_spec
    );
    assert!(
        matches!(untap.target.unhinted(), ChooseSpec::Tagged(tag) if tag == controlled_tag),
        "Recruiter's untap should use the controlled creature tag: {:?}",
        untap.target
    );
    assert!(
        matches!(
            haste.target_spec.as_ref().map(ChooseSpec::unhinted),
            Some(ChooseSpec::Tagged(tag)) if tag == controlled_tag
        ),
        "Recruiter's haste grant should use the controlled creature tag: {:?}",
        haste.target_spec
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kytheons_tactics_count_condition_preserves_prior_creature_set() {
    let def = parse_oracle_card_definition("Kytheon's Tactics");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Kytheon's Tactics should have a spell effect");
    let effects = spell.flattened_default_effects();
    let (pumped_tag, pump) = effects
        .iter()
        .find_map(|effect| {
            let tagged = effect.downcast_ref::<TaggedEffect>()?;
            let pump = tagged
                .effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
            Some((&tagged.tag, pump))
        })
        .expect("Kytheon's Tactics should pump creatures you control");
    let conditional = effects
        .iter()
        .find_map(|effect| unwrapped_effect(effect).downcast_ref::<ConditionalEffect>())
        .expect("Kytheon's Tactics should retain its spell-mastery condition");
    let vigilance = conditional
        .if_true
        .iter()
        .filter_map(continuous_effect)
        .find(|effect| {
            matches!(
                effect.modification.as_ref(),
                Some(crate::continuous::Modification::AddAbility(ability))
                    if ability.id() == StaticAbilityId::Vigilance
            )
        })
        .expect("the spell-mastery branch should grant vigilance");

    assert!(
        matches!(
            &pump.target,
            crate::continuous::EffectTarget::Filter(filter)
                if filter.card_types.contains(&CardType::Creature)
        ),
        "the pump should establish the controlled-creature set, got {:?}",
        pump.target
    );
    assert!(
        matches!(
            vigilance.target_spec.as_ref().map(ChooseSpec::unhinted),
            Some(ChooseSpec::Tagged(tag)) if tag == pumped_tag
        ),
        "the graveyard count predicate must not retarget vigilance to an instant or sorcery card: {:?}",
        vigilance.target_spec
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn jets_brainwashing_keeps_control_untap_haste_as_one_clause() {
    let def = parse_oracle_card_definition("Jet's Brainwashing");
    let rendered = compiled_text_lines(&def).join(" ");

    assert!(
        rendered.contains(", untap it, and it gains haste until end of turn")
            || rendered.contains(", untap that creature, and it gains haste until end of turn"),
        "the kicked control bundle should render as one coordinated clause: {rendered}"
    );
    assert!(
        !rendered.contains(". Untap it.") && !rendered.contains(". Untap that creature."),
        "the two-effect prefix must not split the haste follow-up: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn haphazard_bombardment_random_target_uses_intervening_counted_set() {
    let def = parse_oracle_card_definition("Haphazard Bombardment");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if matches!(
                    &triggered.intervening_if,
                    Some(crate::effect::Condition::ValueComparison { .. })
                ) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Haphazard Bombardment should have a counted end-step trigger");
    let destroy = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| unwrapped_effect(effect).downcast_ref::<DestroyEffect>())
        .expect("the end-step trigger should destroy one permanent");

    let ChooseSpec::WithCount(inner, count) = destroy.spec.unhinted() else {
        panic!(
            "Haphazard Bombardment should select one random permanent, got {:?}",
            destroy.spec
        );
    };
    assert!(count.random && count.min == 1 && count.max == Some(1));
    let ChooseSpec::Object(filter) = inner.unhinted() else {
        panic!("the random selection should use the counted permanent set: {inner:?}");
    };
    assert_eq!(filter.controller, Some(PlayerFilter::NotYou));
    assert_eq!(
        filter.with_counter,
        Some(crate::filter::CounterConstraint::Typed(CounterType::Aim))
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kookus_coordinated_attack_grant_targets_source() {
    let def = parse_oracle_card_definition("Kookus");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Kookus should have an upkeep trigger");
    let must_attack = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .flat_map(|effect| {
            effect
                .downcast_ref::<crate::effects::SequenceEffect>()
                .map_or_else(
                    || std::slice::from_ref(effect),
                    |sequence| sequence.effects.as_slice(),
                )
        })
        .filter_map(continuous_effect)
        .find(|effect| {
            matches!(
                effect.modification.as_ref(),
                Some(crate::continuous::Modification::AddAbility(ability))
                    if ability.id() == StaticAbilityId::MustAttack
            )
        })
        .expect("Kookus should have to attack this turn");

    assert!(
        matches!(&must_attack.target, crate::continuous::EffectTarget::Source),
        "Kookus's coordinated attack subject should be the source, got {:?}",
        must_attack.target
    );
    assert!(
        matches!(
            must_attack.target_spec.as_ref().map(ChooseSpec::unhinted),
            Some(ChooseSpec::Source)
        ),
        "Kookus's attack grant should retain its source target, got {:?}",
        must_attack.target_spec
    );
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        !rendered.contains("hand attacks"),
        "Kookus's source reference must not fall back to a hand-card set: {rendered}"
    );
}
