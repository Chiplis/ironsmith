#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Exile all permanents. For as long as any of those cards remain exiled, at the beginning of each player's upkeep, that player returns one of the exiled cards they own to the battlefield.";

#[test]
fn dimensional_breach_compiles_to_exact_oracle_text() {
    let definition = parse_oracle_card_definition("Dimensional Breach");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);
}

#[test]
fn dimensional_breach_preserves_collection_lifetime_and_active_player_choice() {
    let definition = parse_oracle_card_definition("Dimensional Breach");
    let spell = definition
        .spell_effect
        .as_ref()
        .expect("Dimensional Breach should have a spell effect");
    let effects = spell.flattened_default_effects();

    let schedule = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
        .expect("spell should register one repeating collection-scoped trigger");
    assert!(!schedule.one_shot);
    assert!(
        schedule
            .trigger
            .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
            .is_some_and(|upkeep| upkeep.player == PlayerFilter::Any)
    );
    assert!(
        matches!(
            &schedule.while_any_tagged_object_in_zone,
            Some((tag, Zone::Exile)) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        ),
        "the trigger should expire when its captured exile collection empties: {schedule:#?}"
    );

    let delayed = schedule.effects.flattened_default_effects();
    let choose = delayed
        .iter()
        .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        .expect("the active player should choose one owned captured card");
    assert_eq!(choose.chooser, PlayerFilter::Active);
    assert_eq!(choose.count, crate::effect::ChoiceCount::exactly(1));
    assert_eq!(choose.zone, Some(Zone::Exile));
    assert_eq!(choose.filter.owner, Some(PlayerFilter::Active));
    assert!(choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }));

    let return_effect = delayed
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<WithIdEffect>()
                .map_or(effect, |with_id| &with_id.effect)
                .downcast_ref::<MoveToZoneEffect>()
        })
        .expect("the chosen captured card should return to the battlefield");
    assert_eq!(return_effect.zone, Zone::Battlefield);
    assert_eq!(
        return_effect.battlefield_controller,
        crate::effects::BattlefieldController::Owner
    );
    assert!(
        matches!(
            return_effect.target.base(),
            ChooseSpec::Tagged(tag) if tag == &choose.tag
        ),
        "the return should consume exactly the active player's choice: {return_effect:#?}"
    );
}
