#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn assert_exact(name: &str, definition: &CardDefinition) {
    assert_eq!(
        canonical_compiled_lines(definition).join("\n"),
        oracle_text_by_name()[name]
    );
}

fn schedule_in_program(
    program: &crate::resolution::ResolutionProgram,
) -> &crate::effects::ScheduleDelayedTriggerEffect {
    program
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
        .unwrap_or_else(|| panic!("expected delayed trigger registration: {program:#?}"))
}

#[test]
fn swooping_pteranodon_keeps_the_delayed_land_as_the_damage_source() {
    let definition = parse_oracle_card_definition("Swooping Pteranodon");
    assert_exact("Swooping Pteranodon", &definition);
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Swooping Pteranodon should have its enters trigger");
    let schedule = schedule_in_program(&triggered.effects);
    assert!(schedule.one_shot);
    assert!(
        schedule
            .trigger
            .downcast_ref::<crate::triggers::BeginningOfEndStepTrigger>()
            .is_some_and(|trigger| trigger.player == PlayerFilter::Any),
        "the damage should wait for the next end step: {schedule:#?}"
    );

    let [land_root, damage_root] = schedule.effects.flattened_default_effects() else {
        panic!("expected tagged land choice followed by its damage: {schedule:#?}");
    };
    let tagged_land = land_root
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the target land must be tagged for use as the damage source");
    let target_land = tagged_land
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .expect("the delayed instruction should target a land");
    assert_eq!(
        target_land.target.base(),
        &ChooseSpec::Object(ObjectFilter::land())
    );

    let execute = damage_root
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        .expect("the land tag must become the damage source");
    assert_eq!(execute.source, ChooseSpec::Tagged(tagged_land.tag.clone()));
    let damage = execute
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .expect("the land should deal damage");
    assert_eq!(damage.amount, Value::Fixed(3));
    let ChooseSpec::Object(damaged_creature) = damage.target.base() else {
        panic!("damage should stay bound to the stolen creature: {damage:#?}");
    };
    assert_eq!(damaged_creature.card_types, [CardType::Creature]);
    assert!(
        damaged_creature
            .tagged_constraints
            .iter()
            .all(|constraint| {
                constraint.tag != tagged_land.tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
    );
}

#[test]
fn zevlor_keeps_each_copy_bound_to_that_opponents_chosen_recipient() {
    let definition = parse_oracle_card_definition("Zevlor, Elturel Exile");
    assert_exact("Zevlor, Elturel Exile", &definition);
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Zevlor should have its activated ability");
    let schedule = schedule_in_program(&activated.effects);
    let spell_cast = schedule
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .expect("Zevlor should watch the next qualifying spell cast");
    let spell_filter = spell_cast.filter.as_ref().expect("qualified spell filter");
    assert_eq!(spell_filter.target_count, Some(ChoiceCount::exactly(1)));
    assert_eq!(
        spell_filter.targets_only_player,
        Some(PlayerFilter::Opponent)
    );
    assert!(spell_filter.targets_only_any_of);

    let [_, for_players_root] = schedule.effects.flattened_default_effects() else {
        panic!("expected triggering-spell tag plus opponent loop: {schedule:#?}");
    };
    let for_players = for_players_root
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("each other opponent should receive one copy");
    assert_eq!(
        for_players.filter,
        PlayerFilter::excluding(
            PlayerFilter::Opponent,
            PlayerFilter::TargetPlayerOrControllerOfTarget,
        )
    );
    let [choice_root, _, retarget_root] = for_players.effects.as_slice() else {
        panic!("expected recipient choice, copy, and fixed retarget: {for_players:#?}");
    };
    let tagged_choice = choice_root
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the per-opponent recipient choice must be tagged");
    let choice = tagged_choice
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .expect("the per-opponent recipient should be chosen during resolution");
    assert!(matches!(
        choice.target.base(),
        ChooseSpec::ObjectOrPlayer(object, PlayerFilter::IteratedPlayer)
            if object.controller == Some(PlayerFilter::IteratedPlayer)
    ));

    let retarget = retarget_root
        .downcast_ref::<crate::effects::RetargetStackObjectEffect>()
        .expect("the copy must receive a fixed legal target");
    let crate::effects::RetargetMode::OneToFixed(fixed) = &retarget.mode else {
        panic!("the copy should target the chosen recipient: {retarget:#?}");
    };
    let ChooseSpec::ObjectOrPlayer(object, PlayerFilter::IteratedPlayer) = fixed.base() else {
        panic!("the fixed recipient should be that player or their permanent: {fixed:#?}");
    };
    assert!(object.tagged_constraints.iter().any(|constraint| {
        constraint.tag == tagged_choice.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(
        object
            .tagged_constraints
            .iter()
            .all(|constraint| constraint.tag.as_str() != "triggering")
    );
}
